//! Estado da aplicação e despacho de teclas (testável sem TTY).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::KeyEvent;
use oride_config::Config;
use oride_core::{DocumentError, DocumentStore};
use oride_keymap::{Action, Keymap, ResolvedKey};
use oride_ui::{render_editor, render_status, EditorView, StatusModel, UiTheme};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

/// Comando aplicado ao documento (inclui insert de char).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Action(Action),
    InsertChar(char),
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
    pub keymap: Keymap,
    pub config: Config,
    /// Altura do viewport do editor na última pintura (para PageUp/Down).
    last_editor_height: usize,
}

impl App {
    pub fn new_empty() -> Self {
        let mut store = DocumentStore::new();
        store.open_empty();
        Self::from_store(store, None)
    }

    pub fn open_path(path: PathBuf) -> Result<Self, DocumentError> {
        let hint = path.parent().map(Path::to_path_buf);
        let mut store = DocumentStore::new();
        store.open_path(&path)?;
        Ok(Self::from_store(store, hint.as_deref()))
    }

    /// Constrói app com config já carregada (testes / override).
    pub fn from_store_with_config(store: DocumentStore, config: Config) -> Self {
        let theme = UiTheme::from_config(&config.theme_ui).unwrap_or_else(|_| UiTheme::default());
        let keymap =
            Keymap::from_string_map(config.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .unwrap_or_else(|_| build_default_keymap());

        Self {
            store,
            scroll_y: 0,
            should_quit: false,
            status_message: None,
            message_expires: None,
            quit_confirm_pending: false,
            theme,
            show_line_numbers: config.show_line_numbers,
            keymap,
            config,
            last_editor_height: 20,
        }
    }

    fn from_store(store: DocumentStore, workspace_hint: Option<&Path>) -> Self {
        let config = oride_config::load_merged(workspace_hint).unwrap_or_default();
        Self::from_store_with_config(store, config)
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

    /// Resolve tecla via keymap + digitar.
    #[must_use]
    pub fn map_key(&self, key: KeyEvent) -> Option<KeyCommand> {
        match self.keymap.resolve_event(key)? {
            ResolvedKey::Action(action) => Some(KeyCommand::Action(action)),
            ResolvedKey::InsertChar(c) => Some(KeyCommand::InsertChar(c)),
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
            KeyCommand::InsertChar(c) => {
                self.quit_confirm_pending = false;
                self.insert_char_or_tab(c)?;
            }
            KeyCommand::Action(action) => self.apply_action(action)?,
        }
        self.ensure_cursor_visible();
        Ok(())
    }

    fn insert_char_or_tab(&mut self, c: char) -> Result<(), DocumentError> {
        if c == '\t' {
            return self.insert_tab();
        }
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.store.active_mut()?.insert_text(s)?;
        Ok(())
    }

    fn insert_tab(&mut self) -> Result<(), DocumentError> {
        let text = if self.config.editor.insert_spaces {
            " ".repeat(self.config.editor.tab_size as usize)
        } else {
            "\t".into()
        };
        self.store.active_mut()?.insert_text(&text)?;
        Ok(())
    }

    fn apply_action(&mut self, action: Action) -> Result<(), DocumentError> {
        match action {
            Action::Quit => {
                let dirty = self.store.active().map(|d| d.is_dirty()).unwrap_or(false);
                if dirty && !self.quit_confirm_pending {
                    self.quit_confirm_pending = true;
                    self.set_status(
                        "unsaved changes — save, then quit again (or quit twice to discard)",
                    );
                } else {
                    self.should_quit = true;
                }
            }
            Action::Save => {
                let doc = self.store.active_mut()?;
                match doc.save_to(None) {
                    Ok(()) => {
                        self.quit_confirm_pending = false;
                        self.set_status("saved");
                    }
                    Err(DocumentError::Io(e)) if e.kind() == std::io::ErrorKind::InvalidInput => {
                        self.set_status("no path — open a file to save");
                    }
                    Err(e) => return Err(e),
                }
            }
            Action::Undo => {
                let doc = self.store.active_mut()?;
                if !doc.undo() {
                    self.set_status("nothing to undo");
                }
            }
            Action::Redo => {
                let doc = self.store.active_mut()?;
                if !doc.redo() {
                    self.set_status("nothing to redo");
                }
            }
            Action::InsertNewline => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.insert_text("\n")?;
            }
            Action::InsertTab => {
                self.quit_confirm_pending = false;
                self.insert_tab()?;
            }
            Action::Backspace => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.backspace()?;
            }
            Action::Delete => {
                self.quit_confirm_pending = false;
                self.store.active_mut()?.delete_forward()?;
            }
            Action::MoveLeft { extend } => {
                self.store.active_mut()?.move_left(extend)?;
            }
            Action::MoveRight { extend } => {
                self.store.active_mut()?.move_right(extend)?;
            }
            Action::MoveUp { extend } => {
                self.store.active_mut()?.move_up(extend)?;
            }
            Action::MoveDown { extend } => {
                self.store.active_mut()?.move_down(extend)?;
            }
            Action::MoveLineStart { extend } => {
                self.store.active_mut()?.move_line_start(extend)?;
            }
            Action::MoveLineEnd { extend } => {
                self.store.active_mut()?.move_line_end(extend)?;
            }
            Action::PageUp => {
                let steps = self.last_editor_height.saturating_sub(1).max(1);
                let doc = self.store.active_mut()?;
                for _ in 0..steps {
                    doc.move_up(false)?;
                }
            }
            Action::PageDown => {
                let steps = self.last_editor_height.saturating_sub(1).max(1);
                let doc = self.store.active_mut()?;
                for _ in 0..steps {
                    doc.move_down(false)?;
                }
            }
        }
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
            help_hint: "Ctrl+S save · Ctrl+Z undo · Esc quit · config: ~/.config/oride/".into(),
        };
        render_status(frame, status_area, &status, &self.theme);
    }
}

fn build_default_keymap() -> Keymap {
    let defaults = Config::default();
    Keymap::from_string_map(defaults.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .expect("default key bindings must parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use oride_keymap::Action;

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
        app.apply(KeyCommand::Action(Action::Save));
        assert!(app
            .status_message
            .as_deref()
            .is_some_and(|m| m.contains("no path")));
    }

    #[test]
    fn map_quit_and_save_from_config_defaults() {
        let app = App::new_empty();
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('q'))),
            Some(KeyCommand::Action(Action::Quit))
        );
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('s'))),
            Some(KeyCommand::Action(Action::Save))
        );
        assert_eq!(
            app.map_key(key(KeyCode::Enter)),
            Some(KeyCommand::Action(Action::InsertNewline))
        );
    }

    #[test]
    fn dirty_quit_requires_confirm() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('x'));
        app.apply(KeyCommand::Action(Action::Quit));
        assert!(!app.should_quit);
        app.apply(KeyCommand::Action(Action::Quit));
        assert!(app.should_quit);
    }

    #[test]
    fn rebind_ctrl_s_to_quit_via_config() {
        let mut cfg = Config::default();
        cfg.keys.insert("ctrl+s".into(), "quit".into());
        let store = {
            let mut s = DocumentStore::new();
            s.open_empty();
            s
        };
        let app = App::from_store_with_config(store, cfg);
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('s'))),
            Some(KeyCommand::Action(Action::Quit))
        );
    }

    #[test]
    fn tab_inserts_spaces_from_config() {
        let mut cfg = Config::default();
        cfg.editor.tab_size = 2;
        cfg.editor.insert_spaces = true;
        let store = {
            let mut s = DocumentStore::new();
            s.open_empty();
            s
        };
        let mut app = App::from_store_with_config(store, cfg);
        app.apply(KeyCommand::Action(Action::InsertTab));
        assert_eq!(app.store.active().unwrap().buffer().as_string(), "  ");
    }
}
