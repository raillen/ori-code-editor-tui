//! Loop principal do TUI.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;
use crate::terminal_guard::TerminalGuard;

/// Abre o editor em TUI. `path` opcional (buffer vazio se `None`).
pub fn run(path: Option<PathBuf>) -> anyhow::Result<()> {
    let mut app = match path {
        Some(p) => App::open_path(p).context("open file")?,
        None => App::new_empty(),
    };

    let mut guard = TerminalGuard::enter().context("enter terminal raw mode")?;
    // Esconde cursor do terminal host; desenhamos o caret nós mesmos.
    guard.terminal().hide_cursor()?;

    loop {
        guard.terminal().draw(|frame| app.draw(frame))?;

        if app.should_quit {
            break;
        }

        // Poll com timeout para expirar mensagens de status
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(cmd) = app.map_key(key) {
                        app.apply(cmd);
                    }
                }
                Event::Resize(_, _) => {
                    app.ensure_cursor_visible();
                }
                _ => {}
            }
        } else {
            app.tick_messages();
        }
    }

    Ok(())
}
