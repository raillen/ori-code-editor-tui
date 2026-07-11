//! Widgets de renderização do Oride (sem event loop).

mod editor;
mod status;
mod theme;

pub use editor::{render_editor, EditorView};
pub use status::{render_status, StatusModel};
pub use theme::UiTheme;
