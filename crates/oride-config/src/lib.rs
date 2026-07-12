//! Configuração TOML do Oride: defaults → user → projeto.

mod editorconfig;
mod load;
mod model;

pub use editorconfig::{resolve_indent_for_file, EditorIndent};
pub use load::{config_search_roots, load_merged, user_config_path};
pub use model::{
    default_key_bindings, Config, EditorConfig, LspConfig, SyntaxColorsConfig, TerminalConfig,
    ThemeUiConfig, TreeConfig,
};
