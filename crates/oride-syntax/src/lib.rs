//! Highlight de sintaxe baseado em tree-sitter.

mod highlight;
mod kind;
mod language;
mod markdown;
mod md_preview;

pub use highlight::{line_spans, HighlightEngine, HighlightSpan};
pub use kind::HighlightKind;
pub use language::{detect_language, LanguageId};
pub use markdown::{continue_list_on_enter, list_prefix};
pub use md_preview::{render_preview_lines, PreviewLine, PreviewStyle};
