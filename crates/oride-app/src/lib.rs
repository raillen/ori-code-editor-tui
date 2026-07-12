//! Composição do app TUI: estado, teclas e loop principal.

mod app;
mod browser;
mod clipboard;
mod disk_watch;
mod find;
mod run;
mod session;
mod split;
mod terminal_guard;

pub use app::{App, KeyCommand};
pub use run::run;
