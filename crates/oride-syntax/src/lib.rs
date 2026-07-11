//! Highlight de sintaxe baseado em tree-sitter.

mod highlight;
mod kind;
mod language;

pub use highlight::{line_spans, HighlightEngine, HighlightSpan};
pub use kind::HighlightKind;
pub use language::{detect_language, LanguageId};
