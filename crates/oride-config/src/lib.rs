//! Configuração TOML do Oride: defaults → user → projeto.

mod load;
mod model;

pub use load::{config_search_roots, load_merged, user_config_path};
pub use model::{default_key_bindings, Config, EditorConfig, ThemeUiConfig};
