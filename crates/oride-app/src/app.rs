//! Estado da aplicação e despacho de teclas (testável sem TTY).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use oride_core::{DocumentError, DocumentStore};
use oride_ui::{render_editor, render_status, EditorView, StatusModel, UiTheme};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

/// Comando de alto nível (pré-keymap TOML; hardcode P0.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Quit,
    Save,
    Undo,
    Redo,
    InsertChar(char),
    InsertNewline,
    Backspace,
    Delete,
    MoveLeft { extend: bool },
    MoveRight { extend: bool },
    MoveUp { extend: bool },
    MoveDown { extend: bool },
    MoveLineStart { extend: bool },
    MoveLineEnd { extend: bool },
    PageUp,
    PageDown,
}

#[derive(Debug)]
pub struct App {
    pub store: DocumentStore,
    pub scroll_y: usize,
    pub should_quit: bool,
    pub status_message: Option<String>,
    message_expires: Option<Instant>,
    /// Segundo quit com buffer dirty.
    quit_confirm_pending: bool,
    pub theme: UiTheme,
    pub show_line_numbers: bool,
    /// Altura do viewport do editor na última pintura (para PageUp/Down).
    last_editor_height: usize,
}

impl App {
    pub fn new_empty() -> Self {
        let mut store = DocumentStore::new();
        store.open_empty();
        Self::from_store(store)
    }

    pub fn open_path(path: PathBuf) -> Result<Self, DocumentError> {
        let mut store = DocumentStore::new();
        store.open_path(path)?;
        Ok(Self::from_store(store))
    }

    fn from_store(store: DocumentStore) -> Self {
        Self {
            store,
            scroll_y: 0,
            should_quit: false,
            status_message: None,
            message_expires: None,
            quit_confirm_pending: false,
            theme: UiTheme::default(),
            show_line_numbers: true,
            last_editor_height: 20,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.message_expires = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn tick_messages(&mut self) {
        if let Some(exp) = self.message_expires {
            if Instant::now() >= exp {
                self.status_message = None;
                self.message_expires = None;
            }
        }
    }

    /// Mapeia tecla física → comando (camada keymap virá em P0.3).
    #[must_use]
    pub fn map_key(key: KeyEvent) -> Option<KeyCommand> {
        let extend = key.modifiers.contains(KeyModifiers::SHIFT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if ctrl {
            return match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => Some(KeyCommand::Quit),
                KeyCode::Char('s') | KeyCode::Char('S') => Some(KeyCommand::Save),
                KeyCode::Char('z') | KeyCode::Char('Z') => Some(KeyCommand::Undo),
                KeyCode::Char('y') | KeyCode::Char('Y') => Some(KeyCommand::Redo),
                KeyCode::Home => Some(KeyCommand::MoveLineStart { extend }),
                KeyCode::End => Some(KeyCommand::MoveLineEnd { extend }),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Esc => Some(KeyCommand::Quit),
            KeyCode::Enter => Some(KeyCommand::InsertNewline),
            KeyCode::Backspace => Some(KeyCommand::Backspace),
            KeyCode::Delete => Some(KeyCommand::Delete),
            KeyCode::Left => Some(KeyCommand::MoveLeft { extend }),
            KeyCode::Right => Some(KeyCommand::MoveRight { extend }),
            KeyCode::Up => Some(KeyCommand::MoveUp { extend }),
            KeyCode::Down => Some(KeyCommand::MoveDown { extend }),
            KeyCode::Home => Some(KeyCommand::MoveLineStart { extend }),
            KeyCode::End => Some(KeyCommand::MoveLineEnd { extend }),
            KeyCode::PageUp => Some(KeyCommand::PageUp),
            KeyCode::PageDown => Some(KeyCommand::PageDown),
            KeyCode::Tab => Some(KeyCommand::InsertChar('\t')),
            KeyCode::Char(c) if !c.is_control() => Some(KeyCommand::InsertChar(c)),
            _ => None,
        }
    }

    pub fn apply(&mut self, cmd: KeyCommand) {
        let result = self.apply_inner(cmd);
        if let Err(err) = result {
            self.set_status(format!("error: {err}"));
        }
    }

    fn apply_inner(&mut self, cmd: KeyCommand) -> Result<(), DocumentError> {
        match cmd {
            KeyCommand::Quit => {
                let dirty = self.store.active().map(|d| d.is_dirty()).unwrap_or(false);
                if dirty && !self.quit_confirm_pending {
                    self.quit_confirm_pending = true;
                    self.set_status("unsaved changes — Ctrl+S save, Esc/Ctrl+Q again to quit");
                } else {
                    self.should_quit = true;
                }
            }
            KeyCommand::Save => {
                let doc = self.store.active_mut()?;
                match doc.save_to(None) {
                    Ok(()) => {
                        self.quit_confirm_pending = false;
                        self.set_status("saved");
                    }
                    Err(DocumentError::Io(e)) if e.kind() == std::io::ErrorKind::InvalidInput => {
                        self.set_status("no path — open a file to save (P0.2)");
                    }
                    Err(e) => return Err(e),
                }
            }
            KeyCommand::Undo => {
                let doc = self.store.active_mut()?;
                if !doc.undo() {
                    self.set_status("nothing to undo");
                }
            }
            KeyCommand::Redo => {
                let doc = self.store.active_mut()?;
                if !doc.redo() {
                    self.set_status("nothing to redo");
                }
            }
            KeyCommand::InsertChar(c) => {
                self.quit_confirm_pending = false;
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                self.store.active_mut()?.insert_text(s)?;
            }
            KeyCommand::InsertNewline => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.insert_text("\n")?;
            }
            KeyCommand::Backspace => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.backspace()?;
            }
            KeyCommand::Delete => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.delete_forward()?;
            }
            KeyCommand::MoveLeft { extend } => {
                self.store.active_mut()?.move_left(extend)?;
            }
            KeyCommand::MoveRight { extend } => {
                self.store.active_mut()?.move_right(extend)?;
            }
            KeyCommand::MoveUp { extend } => {
                self.store.active_mut()?.move_up(extend)?;
            }
            KeyCommand::MoveDown { extend } => {
                self.store.active_mut()?.move_down(extend)?;
            }
            KeyCommand::MoveLineStart { extend } => {
                self.store.active_mut()?.move_line_start(extend)?;
            }
            KeyCommand::MoveLineEnd { extend } => {
                self.store.active_mut()?.move_line_end(extend)?;
            }
            KeyCommand::PageUp => {
                let steps = self.last_editor_height.saturating_sub(1).max(1);
                let doc = self.store.active_mut()?;
                for _ in 0..steps {
                    doc.move_up(false)?;
                }
            }
            KeyCommand::PageDown => {
                let steps = self.last_editor_height.saturating_sub(1).max(1);
                let doc = self.store.active_mut()?;
                for _ in 0..steps {
                    doc.move_down(false)?;
                }
            }
        }
        self.ensure_cursor_visible();
        Ok(())
    }

    pub fn ensure_cursor_visible(&mut self) {
        let Ok(doc) = self.store.active() else {
            return;
        };
        let Ok(caret) = doc.caret() else {
            return;
        };
        let height = self.last_editor_height.max(1);
        if caret.line < self.scroll_y {
            self.scroll_y = caret.line;
        } else if caret.line >= self.scroll_y + height {
            self.scroll_y = caret.line + 1 - height;
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        self.tick_messages();
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        let editor_area = chunks[0];
        let status_area = chunks[1];
        self.last_editor_height = editor_area.height as usize;
        self.ensure_cursor_visible();

        let doc = match self.store.active() {
            Ok(d) => d,
            Err(_) => return,
        };
        let caret = doc.caret().unwrap_or_default();

        let view = EditorView {
            buffer: doc.buffer(),
            caret,
            scroll_y: self.scroll_y,
            show_line_numbers: self.show_line_numbers,
        };
        render_editor(frame, editor_area, &view, &self.theme);

        let status = StatusModel {
            title: doc.tab_title(),
            dirty: doc.is_dirty(),
            line: caret.line,
            column: caret.column,
            message: self.status_message.clone(),
            help_hint: "Ctrl+S save · Ctrl+Z undo · Esc quit".into(),
        };
        render_status(frame, status_area, &status, &self.theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn typing_and_save_status() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('h'));
        app.apply(KeyCommand::InsertChar('i'));
        let doc = app.store.active().unwrap();
        assert_eq!(doc.buffer().as_string(), "hi");
        assert!(doc.is_dirty());
        app.apply(KeyCommand::Save);
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|m| m.contains("no path")));
    }

    #[test]
    fn map_quit_and_save() {
        assert_eq!(
            App::map_key(key_ctrl(KeyCode::Char('q'))),
            Some(KeyCommand::Quit)
        );
        assert_eq!(
            App::map_key(key_ctrl(KeyCode::Char('s'))),
            Some(KeyCommand::Save)
        );
        assert_eq!(
            App::map_key(key(KeyCode::Enter)),
            Some(KeyCommand::InsertNewline)
        );
    }

    #[test]
    fn dirty_quit_requires_confirm() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('x'));
        app.apply(KeyCommand::Quit);
        assert!(!app.should_quit);
        app.apply(KeyCommand::Quit);
        assert!(app.should_quit);
    }
}
