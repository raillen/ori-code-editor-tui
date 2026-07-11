//! Núcleo do Oride sem dependência de UI.
//!
//! Buffer (rope), seleção, undo/redo e documentos com tabs.

mod buffer;
mod document;
mod position;
mod selection;
mod undo;

pub use buffer::{Buffer, BufferError};
pub use document::{Document, DocumentError, DocumentId, DocumentStore, TabSummary};
pub use position::{ByteOffset, Caret};
pub use selection::Selection;
pub use undo::{Edit, UndoStack};
