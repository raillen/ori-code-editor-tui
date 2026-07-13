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

    // Mouse off por default; só captura se config/menu ligar.
    let mut guard =
        TerminalGuard::enter(app.mouse_is_enabled()).context("enter terminal raw mode")?;

    loop {
        // Sincroniza capture se o usuário toggou pelo menu/palette
        guard.set_mouse_capture(app.mouse_is_enabled());

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
                    if let Some(d) = last_drag.take() {
                        app.handle_mouse(d);
                    }
                    app.handle_key(key);
                }
                Event::Mouse(me) => {
                    // Sem mouse enabled, ignora (capture deve estar off; defesa em profundidade)
                    if !app.mouse_is_enabled() {
                        continue;
                    }
                    if matches!(me.kind, MouseEventKind::Drag(_)) {
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
