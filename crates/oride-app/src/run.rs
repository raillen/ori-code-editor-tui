//! Loop principal do TUI.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use crossterm::event::{self, Event, KeyEventKind, MouseEvent, MouseEventKind};

use crate::app::App;
use crate::terminal_guard::TerminalGuard;

/// Abre o editor em TUI. `path` opcional (CWD / buffer vazio se `None`).
pub fn run(path: Option<PathBuf>) -> anyhow::Result<()> {
    let mut app = match path {
        Some(p) => App::open_path(p).context("open path")?,
        None => App::new_empty_or_session(),
    };

    let mut guard = TerminalGuard::enter().context("enter terminal raw mode")?;

    loop {
        guard.terminal().draw(|frame| app.draw(frame))?;

        if app.should_quit {
            app.persist_session();
            break;
        }

        // Timeout curto durante drag → seleção acompanha o mouse sem “degrau” de 100ms.
        let timeout = if app.is_mouse_dragging() {
            Duration::from_millis(8)
        } else {
            Duration::from_millis(33)
        };

        if !event::poll(timeout)? {
            app.tick();
            continue;
        }

        // Drena a fila inteira antes do próximo draw (vários Drag → um frame).
        let mut last_drag: Option<MouseEvent> = None;
        loop {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // flush drag pendente antes de tecla
                    if let Some(d) = last_drag.take() {
                        app.handle_mouse(d);
                    }
                    app.handle_key(key);
                }
                Event::Mouse(me) => {
                    if matches!(me.kind, MouseEventKind::Drag(_)) {
                        // mantém só o último drag da rajada
                        last_drag = Some(me);
                    } else {
                        if let Some(d) = last_drag.take() {
                            app.handle_mouse(d);
                        }
                        app.handle_mouse(me);
                    }
                }
                Event::Resize(_, _) => {
                    if let Some(d) = last_drag.take() {
                        app.handle_mouse(d);
                    }
                    app.ensure_cursor_visible();
                }
                _ => {}
            }
            if !event::poll(Duration::ZERO)? {
                break;
            }
        }
        if let Some(d) = last_drag.take() {
            app.handle_mouse(d);
        }
        app.tick();
    }

    Ok(())
}
