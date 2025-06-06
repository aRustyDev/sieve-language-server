/*
 * Sieve Language Server Protocol Implementation
 *
 * This is a full-featured Language Server Protocol (LSP) implementation for the Sieve
 * email filtering language (RFC 5228) with support for extensions and Proton Mail features.
 *
 * Built with Rust using the tower-lsp crate for maximum performance and reliability.
 */

// ================================================================================================
// IMPORTS AND DEPENDENCIES
// ================================================================================================
use sieve_language_server::datastructures::*;
use tower_lsp::{LspService, Server};
use tracing::info;

// ================================================================================================
// MAIN FUNCTION - ENTRY POINT
// ================================================================================================

#[tokio::main]
async fn main() {
    // Initialize logging for debugging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Sieve Language Server");

    // Create stdin/stdout for LSP communication
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Create the language server service
    let (service, socket) = LspService::new(|client| {
        info!("Creating new language server instance");
        SieveLanguageServer::new(client)
    });

    // Start the server
    info!("Sieve Language Server listening on stdin/stdout");
    Server::new(stdin, stdout, socket).serve(service).await;

    info!("Sieve Language Server shutting down");
}
