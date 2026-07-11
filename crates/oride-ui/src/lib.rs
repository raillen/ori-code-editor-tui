//! Widgets de renderização do Oride (sem event loop).

mod color;
mod editor;
mod status;
mod theme;

pub use color::{parse_color, ColorParseError};
pub use editor::{render_editor, EditorView};
pub use status::{render_status, StatusModel};
pub use theme::{ThemeBuildError, UiTheme};
