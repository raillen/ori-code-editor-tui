//! Highlight de sintaxe baseado em tree-sitter.

mod highlight;
mod kind;
mod language;
mod markdown;

pub use highlight::{line_spans, HighlightEngine, HighlightSpan};
pub use kind::HighlightKind;
pub use language::{detect_language, LanguageId};
pub use markdown::{continue_list_on_enter, list_prefix};
