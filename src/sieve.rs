use lazy_static::lazy_static;
use std::collections::HashMap;

// ================================================================================================
// SIEVE LANGUAGE DEFINITIONS
// ================================================================================================

lazy_static! {
    /// All standard Sieve test commands defined in RFC 5228 and common extensions
    /// These are the conditional tests that can be used in if/elsif statements
    pub static ref SIEVE_TESTS: Vec<&'static str> = vec![
        // RFC 5228 base tests - core functionality that should always be available
        "address",     // Test email addresses in headers (From, To, Cc, etc.)
        "allof",       // Logical AND - all sub-tests must be true
        "anyof",       // Logical OR - any sub-test can be true
        "envelope",    // Test SMTP envelope information
        "exists",      // Test if a header field exists
        "false",       // Always evaluates to false (useful for debugging)
        "header",      // Test header field values
        "not",         // Logical NOT - inverts the test result
        "size",        // Test message size
        "true",        // Always evaluates to true (useful for catch-all rules)

        // Common Sieve extensions - widely supported additional functionality
        "body",        // Test message body content (RFC 5173)
        "currentdate", // Test current date/time (useful for time-based rules)
        "date",        // Test date values from headers (RFC 5260)
        "environment", // Access server environment info (RFC 5183)
        "mailbox",     // Test mailbox properties
        "mailboxexists", // Test if mailbox exists before filing
        "regex",       // Regular expression matching (draft standard)
        "spamtest",    // Interface with spam detection systems (RFC 5235)
        "virustest",   // Interface with virus detection systems (RFC 5235)
    ];
}

lazy_static! {
    /// All standard Sieve action commands - these perform operations on messages
    pub static ref SIEVE_ACTIONS: Vec<&'static str> = vec![
        // RFC 5228 base actions - core message handling
        "discard",     // Silently delete the message
        "fileinto",    // Move message to specified folder/mailbox
        "keep",        // Keep message in default location (usually INBOX)
        "redirect",    // Forward message to another email address
        "reject",      // Reject message with error response to sender
        "stop",        // Stop processing this script (but continue with others)

        // IMAP flags extension (RFC 5232) - for IMAP flag manipulation
        "addflag",     // Add IMAP flags to message (e.g., \Seen, \Flagged)
        "removeflag",  // Remove IMAP flags from message
        "setflag",     // Set IMAP flags (replaces existing flags)

        // Additional common actions
        "vacation",    // Send auto-reply message (RFC 5230)
        "notify",      // Send notification to external system
        "denotify",    // Cancel previous notification

        // Proton Mail specific extensions
        "expire",      // Set message expiration time (Proton-specific)
    ];
}

lazy_static! {
    /// Sieve tagged arguments (parameters that start with :)
    /// These modify the behavior of tests and actions
    pub static ref SIEVE_TAGS: Vec<&'static str> = vec![
        // Match type tags - control how string matching is performed
        ":is",         // Exact string match (case-insensitive by default)
        ":contains",   // Substring match
        ":matches",    // Wildcard pattern match (* and ? supported)
        ":regex",      // Regular expression match (if regex extension enabled)

        // Numeric comparison tags (RFC 5231)
        ":count",      // Compare count of header occurrences
        ":value",      // Compare numeric values

        // String comparison tags
        ":comparator", // Specify comparison method (e.g., "i;ascii-casemap")

        // Address part tags - specify which part of email address to test
        ":localpart",  // Local part of address (before @)
        ":domain",     // Domain part of address (after @)
        ":all",        // Entire address

        // Size comparison tags
        ":over",       // Size greater than specified value
        ":under",      // Size less than specified value

        // Action modifier tags
        ":copy",       // Copy message instead of moving it
        ":create",     // Create mailbox if it doesn't exist

        // Date/time tags (RFC 5260)
        ":zone",       // Specify timezone for date operations
        ":originalzone", // Use original message timezone

        // Advanced tags for various extensions
        ":flags",      // Specify IMAP flags
        ":importance", // Message importance level
        ":mime",       // MIME-related operations
        ":anychild",   // Match any child MIME part
        ":type",       // MIME content type
        ":subtype",    // MIME content subtype
        ":contenttype", // Full MIME content type
        ":param",      // MIME parameter
    ];
}

lazy_static! {
    /// Sieve extensions that can be loaded with 'require' statements
    /// Each extension adds new functionality to the base Sieve language
    pub static ref SIEVE_EXTENSIONS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();

        // RFC standardized extensions
        map.insert("body", "Message body testing (RFC 5173)");
        map.insert("copy", "Copy messages instead of moving (RFC 3894)");
        map.insert("date", "Date/time operations (RFC 5260)");
        map.insert("editheader", "Modify message headers (RFC 5293)");
        map.insert("encoded-character", "Encoded character support (RFC 5228)");
        map.insert("envelope", "SMTP envelope testing (RFC 5228)");
        map.insert("environment", "Access to server environment (RFC 5183)");
        map.insert("ereject", "Enhanced reject with reason (RFC 5429)");
        map.insert("fileinto", "File messages into folders (RFC 5228)");
        map.insert("foreverypart", "Iterate over MIME parts (RFC 5703)");
        map.insert("imap4flags", "IMAP flag manipulation (RFC 5232)");
        map.insert("include", "Include other scripts (RFC 6609)");
        map.insert("index", "Positional testing of headers (RFC 5260)");
        map.insert("mailbox", "Mailbox metadata access (RFC 5490)");
        map.insert("mboxmetadata", "Mailbox metadata operations (RFC 5490)");
        map.insert("mime", "MIME structure operations (RFC 5703)");
        map.insert("regex", "Regular expression support (draft)");
        map.insert("reject", "Reject messages with errors (RFC 5228)");
        map.insert("relational", "Numeric comparisons (RFC 5231)");
        map.insert("servermetadata", "Server metadata access (RFC 5490)");
        map.insert("spamtest", "Spam testing interface (RFC 5235)");
        map.insert("subaddress", "Sub-addressing support (RFC 5233)");
        map.insert("vacation", "Auto-reply functionality (RFC 5230)");
        map.insert("variables", "Variable support (RFC 5229)");
        map.insert("virustest", "Virus testing interface (RFC 5235)");

        map
    };
}
