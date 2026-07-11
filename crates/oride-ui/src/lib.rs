//! Widgets de renderização do Oride (sem event loop).

mod color;
mod editor;
mod palette;
mod status;
mod tabs;
mod terminal_panel;
mod theme;
mod tree;

pub use color::{parse_color, ColorParseError};
pub use editor::{render_editor, EditorView};
pub use palette::{render_palette, PaletteView};
pub use status::{render_status, StatusModel};
pub use tabs::render_tabs;
pub use terminal_panel::render_terminal_panel;
pub use theme::{ThemeBuildError, UiTheme};
pub use tree::{render_tree, TreeView};
