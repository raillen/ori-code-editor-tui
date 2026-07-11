//! Loop principal do TUI.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, Event, KeyEventKind};

use crate::app::App;
use crate::terminal_guard::TerminalGuard;

/// Abre o editor em TUI. `path` opcional (CWD / buffer vazio se `None`).
pub fn run(path: Option<PathBuf>) -> anyhow::Result<()> {
    let mut app = match path {
        Some(p) => App::open_path(p).context("open path")?,
        None => App::new_empty(),
    };

    let mut guard = TerminalGuard::enter().context("enter terminal raw mode")?;
    // Cursor do terminal é posicionado pelo render do editor (set_cursor_position).

    loop {
        guard.terminal().draw(|frame| app.draw(frame))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_key(key);
                }
                Event::Resize(_, _) => {
                    app.ensure_cursor_visible();
                }
                _ => {}
            }
        } else {
            app.tick();
        }
    }

    Ok(())
}
