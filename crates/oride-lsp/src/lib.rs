//! Cliente LSP mínimo (stdio, Content-Length framing).
//!
//! Focado em `oriscript lsp`: initialize, sync de docs, diagnostics,
//! hover, completion, definition e formatting.

mod client;
mod protocol;
mod types;

pub use client::{LspClient, LspError, LspEvent};
pub use types::{
    CompletionItem, Diagnostic, DiagnosticSeverity, HoverInfo, Location, Position, Range,
};
