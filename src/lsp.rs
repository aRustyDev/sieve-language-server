// ================================================================================================
// IMPORTS AND DEPENDENCIES
// ================================================================================================

use crate::datastructures::*;
use crate::sieve::*;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer};
use tracing::{debug, info, warn};

// ================================================================================================
// LSP PROTOCOL IMPLEMENTATION
// ================================================================================================

/// Implementation of the Language Server Protocol for Sieve
/// This implements all the LSP methods that editors can call
#[tower_lsp::async_trait]
impl LanguageServer for SieveLanguageServer {
    /// Initialize the language server
    /// Called once when the editor starts the server
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initializing Sieve Language Server");
        info!("Client: {:?}", params.client_info);
        info!("Root URI: {:?}", params.root_uri);

        // Return server capabilities - tells the editor what features we support
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // We support incremental text synchronization
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),

                // We provide completion suggestions
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string()]), // Trigger on colon for tags
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),

                // We provide hover information
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                // We provide diagnostics (error checking)
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("sieve-lsp".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),

                // Future capabilities we might add:
                // definition_provider: Some(OneOf::Left(true)), // Go to definition
                // document_formatting_provider: Some(OneOf::Left(true)), // Code formatting
                // document_symbol_provider: Some(OneOf::Left(true)), // Document outline
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Sieve Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    /// Called after initialize completes
    /// Server is now ready to handle requests
    async fn initialized(&self, _: InitializedParams) {
        info!("Sieve Language Server initialized successfully");

        // Log server capabilities for debugging
        self.client
            .log_message(MessageType::INFO, "Sieve Language Server is ready!")
            .await;
    }

    /// Handle shutdown request from client
    /// Cleanup and prepare for exit
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Sieve Language Server");
        Ok(())
    }

    /// Called when a document is opened in the editor
    /// We need to start tracking this document
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        info!("Document opened: {}", params.text_document.uri);

        // Create and store document in our cache
        let document = SieveDocument::new(
            params.text_document.uri.clone(),
            params.text_document.text,
            params.text_document.version,
        );

        self.document_map
            .insert(params.text_document.uri.clone(), document);

        // Validate the document and send diagnostics
        let diagnostics = self.validate_document(&params.text_document.uri).await;

        self.client
            .publish_diagnostics(params.text_document.uri, diagnostics, None)
            .await;
    }

    /// Called when a document is modified in the editor
    /// We need to update our cached version and re-validate
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        debug!("Document changed: {}", params.text_document.uri);

        // Update our cached document with the changes
        if let Some(mut document) = self.document_map.get_mut(&params.text_document.uri) {
            document.version = params.text_document.version;

            // Apply all content changes
            for change in params.content_changes {
                document.apply_change(&change);
            }
        } else {
            warn!(
                "Received change for unknown document: {}",
                params.text_document.uri
            );
            return;
        }

        // Re-validate the document and send updated diagnostics
        let diagnostics = self.validate_document(&params.text_document.uri).await;

        self.client
            .publish_diagnostics(params.text_document.uri, diagnostics, None)
            .await;
    }

    /// Called when a document is closed in the editor
    /// We can remove it from our cache to save memory
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        info!("Document closed: {}", params.text_document.uri);

        // Remove from cache
        self.document_map.remove(&params.text_document.uri);

        // Clear diagnostics for this document
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    /// Handle completion requests
    /// Called when user triggers auto-completion (e.g., Ctrl+Space)
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        debug!(
            "Completion requested at {:?}",
            params.text_document_position
        );

        let completions = self
            .get_completions(
                &params.text_document_position.text_document.uri,
                params.text_document_position.position,
            )
            .await;

        Ok(Some(CompletionResponse::Array(completions)))
    }

    /// Handle hover requests
    /// Called when user hovers over text to get information
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        debug!(
            "Hover requested at {:?}",
            params.text_document_position_params
        );

        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get the document
        let document = match self.document_map.get(uri) {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Get the line at cursor position
        let line = match document.get_line(position.line as usize) {
            Some(line) => line,
            None => return Ok(None),
        };

        // Find the word at cursor position
        let word = self.get_word_at_position(&line, position.character as usize);

        if let Some(word) = word {
            // Generate hover information based on the word
            let documentation = if SIEVE_TESTS.contains(&word.as_str()) {
                Some(self.get_test_documentation(&word))
            } else if SIEVE_ACTIONS.contains(&word.as_str()) {
                Some(self.get_action_documentation(&word))
            } else if SIEVE_TAGS.contains(&word.as_str()) {
                Some(self.get_tag_documentation(&word))
            } else if SIEVE_EXTENSIONS.contains_key(word.as_str()) {
                SIEVE_EXTENSIONS.get(word.as_str()).map(|s| s.to_string())
            } else {
                None
            };

            if let Some(doc) = documentation {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(doc)),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    /// Handle configuration changes from the editor
    /// Called when user updates settings
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        info!("Configuration changed: {:?}", params.settings);

        // Parse new settings (this is simplified - real implementation would be more robust)
        if let Ok(new_settings) = serde_json::from_value::<SieveSettings>(params.settings) {
            let mut settings = self.settings.write().await;
            *settings = new_settings;
            info!("Updated settings: {:?}", *settings);

            // Re-validate all open documents with new settings
            for item in self.document_map.iter() {
                let uri = item.key().clone();
                let diagnostics = self.validate_document(&uri).await;
                self.client
                    .publish_diagnostics(uri, diagnostics, None)
                    .await;
            }
        }
    }
}
