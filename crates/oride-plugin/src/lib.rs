//! Superfície de extensão **built-in** (P8).
//!
//! Sem carregamento dinâmico (Lua/WASM). Plugins e language providers
//! são registrados no host em tempo de compilação.

mod host;
mod language;
mod plugin;

pub use host::{builtin_host, PluginHost};
pub use language::{
    BuiltinLang, LanguageProvider, LANG_CSS, LANG_HTML, LANG_JS, LANG_MD, LANG_MDX, LANG_ORIS,
    LANG_PLAIN,
};
pub use plugin::{
    CommandMeta, Plugin, PluginCtx, PluginError, PluginHook, PluginResult, ShowPathPlugin,
    WordCountPlugin,
};
