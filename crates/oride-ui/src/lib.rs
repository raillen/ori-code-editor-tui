//! Widgets de renderização do Oride (sem event loop).

mod chrome;
mod color;
mod editor;
mod md_preview;
mod palette;
mod status;
mod tabs;
mod terminal_panel;
mod theme;
mod tree;

pub use chrome::{
    render_context_banner, render_find_modal, render_menu_bar, render_menu_dropdown,
    render_mini_modal, render_scm_panel, render_which_key, FindModalView, MenuColumn, MenuItem,
    MiniModalView, ScmItem,
};
pub use color::{parse_color, ColorParseError};
pub use editor::{render_editor, EditorView};
pub use md_preview::{render_md_preview, MdPreviewView};
pub use palette::{render_find_bar, render_palette, FindBarView, PaletteView};
pub use status::{render_status, StatusModel};
pub use tabs::render_tabs;
pub use terminal_panel::render_terminal_panel;
pub use theme::{ThemeBuildError, UiTheme};
pub use tree::{render_tree, TreeView};
