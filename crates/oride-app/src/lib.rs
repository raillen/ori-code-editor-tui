//! Composição do app TUI: estado, teclas e loop principal.

mod app;
mod run;
mod terminal_guard;

pub use app::{App, KeyCommand};
pub use run::run;
