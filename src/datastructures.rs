use crate::sieve::{SIEVE_ACTIONS, SIEVE_EXTENSIONS, SIEVE_TAGS, SIEVE_TESTS};
use dashmap::DashMap;
use lazy_static::lazy_static;
use regex::Regex;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use tracing::{trace, error, info, warn};
use url::Url;

// ================================================================================================
// DATA STRUCTURES
// ================================================================================================

/// Configuration settings for the Sieve Language Server
/// These can be customized by the client (editor) to change server behavior
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SieveSettings {
    /// Enable Proton Mail specific extensions like 'expire' and 'currentdate'
    /// When false, these extensions will be flagged as warnings
    #[serde(default = "default_true")]
    proton_extensions: bool,

    /// Strict RFC 5228 compliance mode
    /// When true, only RFC-standardized features are allowed
    #[serde(default = "default_false")]
    strict_mode: bool,

    /// Maximum number of diagnostics to report per document
    /// Prevents overwhelming the editor with too many errors
    #[serde(default = "default_max_errors")]
    max_errors: usize,

    /// Enable advanced semantic analysis
    /// Includes checking for undefined extensions, unreachable code, etc.
    #[serde(default = "default_true")]
    semantic_analysis: bool,
}

// Helper functions for default values in serde
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_max_errors() -> usize {
    100
}

impl Default for SieveSettings {
    fn default() -> Self {
        Self {
            proton_extensions: true,
            strict_mode: false,
            max_errors: 100,
            semantic_analysis: true,
        }
    }
}

/// Represents a Sieve document in memory with efficient text operations
/// Uses Rope for O(log n) insertions/deletions and UTF-8 safety
#[derive(Debug, Clone)]
pub struct SieveDocument {
    /// URI of the document (file path or other identifier)
    pub uri: Url,
    /// Rope data structure for efficient text editing operations
    /// Rope allows for fast insertions/deletions without copying entire text
    text: Rope,
    /// Document version number for synchronization with client
    /// Incremented each time the document is modified
    pub version: i32,
}

impl SieveDocument {
    /// Create a new Sieve document from URI and initial text content
    pub fn new(uri: Url, text: String, version: i32) -> Self {
        Self {
            uri,
            text: Rope::from_str(&text),
            version,
        }
    }

    /// Apply a text change to the document
    /// LSP sends incremental changes as ranges to avoid sending entire document
    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        match change.range {
            Some(range) => {
                // Incremental change - replace text in specific range
                let start_idx = self.text.line_to_char(range.start.line as usize)
                    + range.start.character as usize;
                let end_idx =
                    self.text.line_to_char(range.end.line as usize) + range.end.character as usize;

                // Remove old text and insert new text atomically
                self.text.remove(start_idx..end_idx);
                self.text.insert(start_idx, &change.text);
            }
            None => {
                // Full document replacement
                self.text = Rope::from_str(&change.text);
            }
        }
    }

    /// Get the full text content of the document as a String
    pub fn get_text(&self) -> String {
        self.text.to_string()
    }

    /// Get a specific line of text (0-indexed)
    /// Returns None if line number is out of bounds
    pub fn get_line(&self, line: usize) -> Option<String> {
        if line < self.text.len_lines() {
            Some(self.text.line(line).to_string())
        } else {
            None
        }
    }
}

/// The main Language Server structure
/// Handles all LSP protocol interactions and maintains server state
#[derive(Debug)]
pub struct SieveLanguageServer {
    /// LSP client handle for sending notifications and requests back to editor
    pub client: Client,

    /// Thread-safe cache of open documents
    /// DashMap provides concurrent access without locks for reading
    pub document_map: Arc<DashMap<Url, SieveDocument>>,

    /// Global settings that apply to all documents
    /// RwLock allows multiple readers or single writer access
    pub settings: Arc<RwLock<SieveSettings>>,
}

impl SieveLanguageServer {
    /// Create a new language server instance
    pub fn new(client: Client) -> Self {
        trace!("Creating new language server instance");
        Self {
            client,
            document_map: Arc::new(DashMap::new()),
            settings: Arc::new(RwLock::new(SieveSettings::default())),
        }
    }

    /// Extract word at specific character position in a line
    /// This is a utility method for the hover functionality
    pub fn get_word_at_position(&self, line: &str, character: usize) -> Option<String> {
        trace!("get_word_at_position: line={}, character={}", line, character);
        if character > line.len() {
            return None;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut start = character;
        let mut end = character;

        // Find start of word (go backwards until we hit non-word character)
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == ':')
        {
            start -= 1;
        }

        // Find end of word (go forwards until we hit non-word character)
        while end < chars.len()
            && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == ':')
        {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    /// Generate diagnostics (errors, warnings) for a Sieve document
    /// This is the core validation logic that checks syntax and semantics
    pub async fn validate_document(&self, uri: &Url) -> Vec<Diagnostic> {
        trace!("Validating document {}", uri);
        let mut diagnostics = Vec::new();

        // Get document from cache
        let document = match self.document_map.get(uri) {
            Some(doc) => doc.clone(),
            None => {
                error!("Document not found in cache: {}", uri);
                return diagnostics;
            }
        };

        // Get current settings
        let settings = self.settings.read().await.clone();

        let text = document.get_text();
        let lines: Vec<&str> = text.lines().collect();

        info!("Validating document with {} lines", lines.len());

        // Track required extensions to validate 'require' statements
        let mut required_extensions = Vec::new();
        let mut used_extensions = Vec::new();

        // Analyze each line for syntax and semantic errors
        for (line_idx, line) in lines.iter().enumerate() {
            trace!("Analyzing line {}", line_idx);
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for basic syntax errors
            self.check_line_syntax(&mut diagnostics, line_idx, line, &settings)
                .await;

            // Track extension usage for semantic analysis
            if settings.semantic_analysis {
                self.analyze_extensions(
                    &mut diagnostics,
                    line_idx,
                    line,
                    &mut required_extensions,
                    &mut used_extensions,
                    &settings,
                )
                .await;
            }

            // Stop if we've hit the error limit to avoid overwhelming the editor
            if diagnostics.len() >= settings.max_errors {
                warn!("Reached maximum error limit of {}", settings.max_errors);
                break;
            }
        }

        // Perform global semantic analysis
        if settings.semantic_analysis {
            self.check_extension_consistency(
                &mut diagnostics,
                &required_extensions,
                &used_extensions,
            )
            .await;
        }

        info!("Generated {} diagnostics for {}", diagnostics.len(), uri);
        diagnostics
    }

    /// Check syntax errors for a single line
    async fn check_line_syntax(
        &self,
        diagnostics: &mut Vec<Diagnostic>,
        line_idx: usize,
        line: &str,
        settings: &SieveSettings,
    ) {
        trace!("Checking syntax for line {}", line_idx);
        let trimmed = line.trim();

        // Check for missing semicolons on action statements
        if self.is_action_line(trimmed) && !trimmed.ends_with(';') {
            error!("Missing semicolon after action statement");
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: (line.len() - 1) as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("missing-semicolon".to_string())),
                code_description: Some(CodeDescription {
                    href: Url::parse("https://datatracker.ietf.org/doc/html/rfc5228#section-2.1")
                        .unwrap(),
                }),
                source: Some("sieve-lsp".to_string()),
                message: "Missing semicolon after action statement".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // Check for invalid Sieve statements
        if trimmed.ends_with(';') && !self.is_valid_sieve_statement(trimmed, settings) {
            error!("Invalid Sieve statement syntax");
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("invalid-syntax".to_string())),
                code_description: Some(CodeDescription {
                    href: Url::parse("https://datatracker.ietf.org/doc/html/rfc5228#section-8")
                        .unwrap(),
                }),
                source: Some("sieve-lsp".to_string()),
                message: "Invalid Sieve statement syntax".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // Check for Proton extensions when disabled
        if !settings.proton_extensions {
            let proton_commands = ["expire", "currentdate"];
            for cmd in &proton_commands {
                if trimmed.contains(cmd) {
                    if let Some(pos) = trimmed.find(cmd) {
                        warn!("Invalid Proton extension syntax");
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: pos as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (pos + cmd.len()) as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String(
                                "proton-extension-disabled".to_string(),
                            )),
                            code_description: Some(CodeDescription {
                                href: Url::parse(
                                    "https://proton.me/support/sieve-advanced-custom-filters",
                                )
                                .unwrap(),
                            }),
                            source: Some("sieve-lsp".to_string()),
                            message: format!("Proton extension '{}' is disabled in settings", cmd),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
            }
        }
    }

    /// Analyze extension usage and requirements
    async fn analyze_extensions(
        &self,
        _diagnostics: &mut Vec<Diagnostic>,
        _line_idx: usize,
        line: &str,
        required_extensions: &mut Vec<String>,
        used_extensions: &mut Vec<String>,
        _settings: &SieveSettings,
    ) {
        trace!("Analyzing extension");
        let trimmed = line.trim();

        // Parse 'require' statements to track required extensions
        if trimmed.starts_with("require") {
            trace!("Parsing require statement");
            // Extract extensions from require statement
            // Examples: require "fileinto"; or require ["body", "regex"];
            if let Some(extensions) = self.parse_require_statement(trimmed) {
                required_extensions.extend(extensions);
            }
        }

        // Check if line uses extensions that should be required
        for (ext_name, _) in SIEVE_EXTENSIONS.iter() {
            trace!("Checking extension usage : {}", ext_name);
            if self.line_uses_extension(trimmed, ext_name) {
                if !used_extensions.contains(&ext_name.to_string()) {
                    used_extensions.push(ext_name.to_string());
                }
            }
        }
    }

    /// Check that all used extensions are properly required
    async fn check_extension_consistency(
        &self,
        diagnostics: &mut Vec<Diagnostic>,
        required_extensions: &[String],
        used_extensions: &[String],
    ) {
        trace!("Checking extension consistency");
        // Find extensions that are used but not required
        for used_ext in used_extensions {
            trace!("Checking extension usage : {}", used_ext);
            if !required_extensions.contains(used_ext) {
                // This would need line-specific information for proper positioning
                // For now, we'll add a general diagnostic
                warn!("Extension {} is used but not required", used_ext);
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("missing-require".to_string())),
                    code_description: Some(CodeDescription {
                        href: Url::parse(
                            "https://datatracker.ietf.org/doc/html/rfc5228#section-3.2",
                        )
                        .unwrap(),
                    }),
                    source: Some("sieve-lsp".to_string()),
                    message: format!("Extension '{}' is used but not required", used_ext),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }
    }

    /// Check if a line contains an action statement
    fn is_action_line(&self, line: &str) -> bool {
        SIEVE_ACTIONS
            .iter()
            .any(|action| line.trim_start().starts_with(action))
    }

    /// Validate if a statement follows Sieve syntax rules
    fn is_valid_sieve_statement(&self, line: &str, settings: &SieveSettings) -> bool {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return true;
        }

        // Check for known Sieve constructs
        let valid_starts = ["require", "if", "elsif", "else", "stop", "{", "}"];

        // Check if line starts with valid keyword
        if valid_starts.iter().any(|start| trimmed.starts_with(start)) {
            return true;
        }

        // Check if line starts with known test or action
        let available_tests = if settings.proton_extensions {
            SIEVE_TESTS.clone()
        } else {
            SIEVE_TESTS
                .iter()
                .filter(|test| !["currentdate"].contains(test))
                .cloned()
                .collect()
        };

        let available_actions = if settings.proton_extensions {
            SIEVE_ACTIONS.clone()
        } else {
            SIEVE_ACTIONS
                .iter()
                .filter(|action| !["expire"].contains(action))
                .cloned()
                .collect()
        };

        available_tests.iter().any(|test| trimmed.contains(test))
            || available_actions
                .iter()
                .any(|action| trimmed.starts_with(action))
    }

    /// Parse a require statement to extract extension names
    fn parse_require_statement(&self, line: &str) -> Option<Vec<String>> {
        // This is a simplified parser - a full implementation would use the tree-sitter grammar
        lazy_static! {
            static ref REQUIRE_REGEX: Regex =
                Regex::new(r#"require\s+(?:\[([^\]]+)\]|"([^"]+)")"#).unwrap();
        }

        if let Some(captures) = REQUIRE_REGEX.captures(line) {
            if let Some(list_match) = captures.get(1) {
                // Handle array format: require ["ext1", "ext2"];
                let list_content = list_match.as_str();
                let extensions: Vec<String> = list_content
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();
                Some(extensions)
            } else if let Some(single_match) = captures.get(2) {
                // Handle single string format: require "ext";
                Some(vec![single_match.as_str().to_string()])
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if a line uses a specific extension
    fn line_uses_extension(&self, line: &str, extension: &str) -> bool {
        match extension {
            "body" => line.contains("body "),
            "regex" => line.contains(":regex"),
            "fileinto" => line.contains("fileinto"),
            "vacation" => line.contains("vacation"),
            "copy" => line.contains(":copy"),
            "date" => line.contains("date ") || line.contains("currentdate"),
            "relational" => line.contains(":value") || line.contains(":count"),
            "imap4flags" => {
                line.contains("addflag") || line.contains("setflag") || line.contains("removeflag")
            }
            _ => false,
        }
    }

    /// Generate completion items for the current cursor position
    pub async fn get_completions(&self, _uri: &Url, _position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        let settings = self.settings.read().await;

        // Add test command completions
        for test in SIEVE_TESTS.iter() {
            // Skip Proton extensions if disabled
            if !settings.proton_extensions && ["currentdate"].contains(test) {
                continue;
            }

            completions.push(CompletionItem {
                label: test.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Sieve test: {}", test)),
                documentation: Some(Documentation::String(self.get_test_documentation(test))),
                insert_text: Some(test.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        // Add action command completions
        for action in SIEVE_ACTIONS.iter() {
            // Skip Proton extensions if disabled
            if !settings.proton_extensions && ["expire"].contains(action) {
                continue;
            }

            completions.push(CompletionItem {
                label: action.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!("Sieve action: {}", action)),
                documentation: Some(Documentation::String(self.get_action_documentation(action))),
                insert_text: Some(format!("{};", action)), // Auto-add semicolon for actions
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        // Add tag completions
        for tag in SIEVE_TAGS.iter() {
            completions.push(CompletionItem {
                label: tag.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("Sieve tag: {}", tag)),
                documentation: Some(Documentation::String(self.get_tag_documentation(tag))),
                insert_text: Some(tag.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        // Add extension completions for require statements
        for (ext_name, ext_desc) in SIEVE_EXTENSIONS.iter() {
            completions.push(CompletionItem {
                label: format!("\"{}\"", ext_name),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some(format!("Sieve extension: {}", ext_name)),
                documentation: Some(Documentation::String(ext_desc.to_string())),
                insert_text: Some(format!("\"{}\"", ext_name)),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        info!("Generated {} completion items", completions.len());
        completions
    }

    /// Get documentation for a test command
    pub fn get_test_documentation(&self, test: &str) -> String {
        match test {
            "address" => "Tests email addresses in headers like From, To, Cc, Bcc".to_string(),
            "allof" => "Logical AND operator - all contained tests must be true".to_string(),
            "anyof" => "Logical OR operator - any contained test can be true".to_string(),
            "envelope" => "Tests SMTP envelope information (MAIL FROM, RCPT TO)".to_string(),
            "exists" => "Tests whether specified header fields exist in the message".to_string(),
            "header" => "Tests the contents of specified header fields".to_string(),
            "size" => "Tests the size of the message in bytes".to_string(),
            "body" => {
                "Tests the body content of the message (requires 'body' extension)".to_string()
            }
            "currentdate" => {
                "Tests the current date/time on the server (Proton extension)".to_string()
            }
            "regex" => {
                "Provides regular expression matching (requires 'regex' extension)".to_string()
            }
            _ => format!("Sieve test command: {}", test),
        }
    }

    /// Get documentation for an action command
    pub fn get_action_documentation(&self, action: &str) -> String {
        match action {
            "fileinto" => "Files the message into the specified mailbox/folder".to_string(),
            "redirect" => "Redirects the message to the specified email address".to_string(),
            "reject" => "Rejects the message with an error sent back to sender".to_string(),
            "discard" => "Silently discards the message (no error sent)".to_string(),
            "keep" => "Keeps the message in the default location (usually INBOX)".to_string(),
            "stop" => "Stops processing the current script".to_string(),
            "vacation" => "Sends an auto-reply message (requires 'vacation' extension)".to_string(),
            "expire" => "Sets message expiration time (Proton extension)".to_string(),
            _ => format!("Sieve action command: {}", action),
        }
    }

    /// Get documentation for a tag parameter
    pub fn get_tag_documentation(&self, tag: &str) -> String {
        match tag {
            ":is" => "Exact string match (case-insensitive by default)".to_string(),
            ":contains" => {
                "Substring match - tests if the string contains the specified text".to_string()
            }
            ":matches" => "Wildcard pattern match using * and ? characters".to_string(),
            ":regex" => "Regular expression match (requires 'regex' extension)".to_string(),
            ":over" => {
                "Size comparison - tests if size is greater than specified value".to_string()
            }
            ":under" => "Size comparison - tests if size is less than specified value".to_string(),
            ":copy" => "Copy the message instead of moving it (preserves original)".to_string(),
            ":zone" => "Specifies timezone for date operations".to_string(),
            _ => format!("Sieve tag parameter: {}", tag),
        }
    }
}
