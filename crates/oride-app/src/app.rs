//! Estado da aplicação e despacho de teclas (testável sem TTY).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use oride_config::Config;
use oride_core::{DocumentError, DocumentId, DocumentStore};
use oride_fs::{list_files_recursive, CreateKind, ProjectTree};
use oride_git::{current_branch, status_map, GitFileStatus};
use oride_keymap::{Action, Keymap, ResolvedKey};
use oride_syntax::{continue_list_on_enter, detect_language, HighlightEngine, LanguageId};
use oride_terminal::EmbeddedTerminal;
use oride_ui::{
    render_editor, render_palette, render_status, render_tabs, render_terminal_panel, render_tree,
    EditorView, PaletteView, StatusModel, TreeView, UiTheme,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

/// Comando aplicado ao documento (inclui insert de char).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCommand {
    Action(Action),
    InsertChar(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    Tree,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Overlay {
    None,
    CommandPalette { query: String, selected: usize },
    OpenFile { query: String, selected: usize },
    Prompt { kind: PromptKind, buffer: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    NewFile,
    NewDir,
    /// Caminho de pasta a abrir como workspace.
    OpenFolder,
}

pub struct App {
    pub store: DocumentStore,
    pub scroll_y: usize,
    pub should_quit: bool,
    pub status_message: Option<String>,
    message_expires: Option<Instant>,
    quit_confirm_pending: bool,
    close_tab_confirm: Option<DocumentId>,
    pub theme: UiTheme,
    pub show_line_numbers: bool,
    pub keymap: Keymap,
    pub config: Config,
    last_editor_height: usize,
    pub focus: Focus,
    pub show_tree: bool,
    pub tree: Option<ProjectTree>,
    tree_scroll: usize,
    workspace: PathBuf,
    git_status: HashMap<PathBuf, GitFileStatus>,
    git_branch: Option<String>,
    use_nerd_icons: bool,
    tree_width: u16,
    terminal: Option<EmbeddedTerminal>,
    overlay: Overlay,
    file_index: Vec<PathBuf>,
    highlight: HighlightEngine,
    /// Soft wrap (default true em Markdown).
    soft_wrap: bool,
}

impl App {
    pub fn new_empty() -> Self {
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut store = DocumentStore::new();
        store.open_empty();
        Self::from_store(store, workspace)
    }

    pub fn open_path(path: PathBuf) -> Result<Self, DocumentError> {
        if path.is_dir() {
            return Self::open_workspace(path);
        }
        let workspace = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let workspace = std::fs::canonicalize(&workspace).unwrap_or(workspace);
        let mut store = DocumentStore::new();
        store.open_path(&path)?;
        let mut app = Self::from_store(store, workspace);
        app.apply_language_defaults(detect_language(Some(path.as_path())));
        Ok(app)
    }

    pub fn open_workspace(dir: PathBuf) -> Result<Self, DocumentError> {
        let workspace = std::fs::canonicalize(&dir).unwrap_or(dir);
        let mut store = DocumentStore::new();
        store.open_empty();
        Ok(Self::from_store(store, workspace))
    }

    pub fn from_store_with_config(
        store: DocumentStore,
        config: Config,
        workspace: PathBuf,
    ) -> Self {
        let theme = UiTheme::from_config(&config.theme_ui).unwrap_or_else(|_| UiTheme::default());
        let keymap =
            Keymap::from_string_map(config.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .unwrap_or_else(|_| build_default_keymap());

        let show_hidden = false;
        let tree = ProjectTree::open(&workspace, show_hidden).ok();
        let git_status = status_map(&workspace);
        let git_branch = current_branch(&workspace);
        let file_index = list_files_recursive(&workspace, show_hidden).unwrap_or_default();

        let terminal = EmbeddedTerminal::spawn(&workspace, 80, 12).ok();

        Self {
            store,
            scroll_y: 0,
            should_quit: false,
            status_message: None,
            message_expires: None,
            quit_confirm_pending: false,
            close_tab_confirm: None,
            theme,
            show_line_numbers: config.show_line_numbers,
            keymap,
            config,
            last_editor_height: 20,
            focus: Focus::Editor,
            show_tree: true,
            tree,
            tree_scroll: 0,
            workspace,
            git_status,
            git_branch,
            use_nerd_icons: true,
            tree_width: 28,
            terminal,
            overlay: Overlay::None,
            file_index,
            highlight: HighlightEngine::new(),
            soft_wrap: false,
        }
    }

    fn from_store(store: DocumentStore, workspace: PathBuf) -> Self {
        let config = oride_config::load_merged(Some(workspace.as_path())).unwrap_or_default();
        Self::from_store_with_config(store, config, workspace)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.message_expires = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn tick(&mut self) {
        self.tick_messages();
        if let Some(term) = self.terminal.as_mut() {
            term.poll_output();
        }
    }

    pub fn tick_messages(&mut self) {
        if let Some(exp) = self.message_expires {
            if Instant::now() >= exp {
                self.status_message = None;
                self.message_expires = None;
            }
        }
    }

    /// Entrada principal de teclado (overlays / focus / keymap).
    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_overlay_key(key) {
            return;
        }
        match self.focus {
            Focus::Tree => {
                if self.handle_tree_key(key) {
                    return;
                }
            }
            Focus::Terminal => {
                if self.handle_terminal_key(key) {
                    return;
                }
            }
            Focus::Editor => {}
        }
        if let Some(cmd) = self.map_key(key) {
            self.apply(cmd);
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) -> bool {
        match &self.overlay {
            Overlay::None => false,
            Overlay::CommandPalette { .. } | Overlay::OpenFile { .. } => {
                self.handle_palette_key(key);
                true
            }
            Overlay::Prompt { .. } => {
                self.handle_prompt_key(key);
                true
            }
        }
    }

    fn handle_palette_key(&mut self, key: KeyEvent) {
        let (is_cmd, query, selected) = match &self.overlay {
            Overlay::CommandPalette { query, selected } => (true, query.clone(), *selected),
            Overlay::OpenFile { query, selected } => (false, query.clone(), *selected),
            _ => return,
        };

        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            KeyCode::Enter => {
                let items = self.palette_items(is_cmd, &query);
                if let Some(item) = items.get(selected).cloned() {
                    self.overlay = Overlay::None;
                    if is_cmd {
                        if let Some(action) = Action::palette_actions()
                            .iter()
                            .find(|a| a.palette_label() == item)
                            .copied()
                        {
                            let _ = self.apply_action(action);
                        }
                    } else {
                        let path = self.workspace.join(&item);
                        if let Err(e) = self.store.open_path(&path) {
                            self.set_status(format!("open: {e}"));
                        } else {
                            let lang = detect_language(Some(path.as_path()));
                            self.apply_language_defaults(lang);
                            self.focus = Focus::Editor;
                            self.scroll_y = 0;
                        }
                    }
                }
            }
            KeyCode::Up => {
                let len = self.palette_items(is_cmd, &query).len();
                let selected = if len == 0 {
                    0
                } else {
                    selected.saturating_sub(1)
                };
                self.set_palette(is_cmd, query, selected);
            }
            KeyCode::Down => {
                let len = self.palette_items(is_cmd, &query).len();
                let selected = if len == 0 {
                    0
                } else {
                    (selected + 1).min(len - 1)
                };
                self.set_palette(is_cmd, query, selected);
            }
            KeyCode::Backspace => {
                let mut q = query;
                q.pop();
                self.set_palette(is_cmd, q, 0);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut q = query;
                q.push(c);
                self.set_palette(is_cmd, q, 0);
            }
            _ => {}
        }
    }

    fn set_palette(&mut self, is_cmd: bool, query: String, selected: usize) {
        self.overlay = if is_cmd {
            Overlay::CommandPalette { query, selected }
        } else {
            Overlay::OpenFile { query, selected }
        };
    }

    fn palette_items(&self, is_cmd: bool, query: &str) -> Vec<String> {
        if is_cmd {
            let mut items: Vec<String> = Action::palette_actions()
                .iter()
                .map(|a| a.palette_label().to_string())
                .filter(|l| fuzzy_match(query, l))
                .collect();
            items.sort();
            items
        } else {
            let mut items: Vec<String> = self
                .file_index
                .iter()
                .map(|p| p.display().to_string())
                .filter(|p| fuzzy_match(query, p))
                .collect();
            items.truncate(200);
            items
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        let Overlay::Prompt { kind, buffer } = &self.overlay else {
            return;
        };
        let kind = *kind;
        let mut buffer = buffer.clone();
        match key.code {
            KeyCode::Esc => self.overlay = Overlay::None,
            KeyCode::Enter => {
                self.overlay = Overlay::None;
                match kind {
                    PromptKind::OpenFolder => {
                        self.open_workspace_folder(PathBuf::from(buffer.trim()));
                    }
                    PromptKind::NewFile | PromptKind::NewDir => {
                        if let Some(tree) = self.tree.as_mut() {
                            let create = match kind {
                                PromptKind::NewFile => CreateKind::File,
                                PromptKind::NewDir => CreateKind::Directory,
                                PromptKind::OpenFolder => unreachable!(),
                            };
                            match tree.create_under_selection(create, &buffer) {
                                Ok(path) => {
                                    self.refresh_git_and_index();
                                    if kind == PromptKind::NewFile {
                                        if let Err(e) = self.store.open_path(path) {
                                            self.set_status(format!("open: {e}"));
                                        } else {
                                            self.focus = Focus::Editor;
                                        }
                                    } else {
                                        self.set_status("folder created");
                                    }
                                }
                                Err(e) => self.set_status(format!("create: {e}")),
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                buffer.pop();
                self.overlay = Overlay::Prompt { kind, buffer };
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                buffer.push(c);
                self.overlay = Overlay::Prompt { kind, buffer };
            }
            _ => {}
        }
    }

    fn handle_tree_key(&mut self, key: KeyEvent) -> bool {
        // Atalhos com Ctrl ainda passam pelo keymap (foco, save, etc.)
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return false;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(t) = self.tree.as_mut() {
                    t.move_selection(-1);
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(t) = self.tree.as_mut() {
                    t.move_selection(1);
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Enter => {
                if let Some(t) = self.tree.as_mut() {
                    match t.activate_selected() {
                        Ok(Some(path)) => {
                            if let Err(e) = self.store.open_path(&path) {
                                self.set_status(format!("open: {e}"));
                            } else {
                                let lang = detect_language(Some(path.as_path()));
                                self.apply_language_defaults(lang);
                                self.focus = Focus::Editor;
                                self.scroll_y = 0;
                                self.set_status(format!("aberto · {}", lang.as_str()));
                            }
                        }
                        Ok(None) => {
                            self.ensure_tree_visible();
                            self.set_status("↑↓ navegar · Enter abrir/expandir · ←→ colapsar/expandir · Esc editor");
                        }
                        Err(e) => self.set_status(format!("tree: {e}")),
                    }
                }
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if let Some(t) = self.tree.as_mut() {
                    if let Err(e) = t.expand_selected() {
                        self.set_status(format!("tree: {e}"));
                    }
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(t) = self.tree.as_mut() {
                    if let Err(e) = t.collapse_or_parent() {
                        self.set_status(format!("tree: {e}"));
                    }
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Char(' ') => {
                if let Some(t) = self.tree.as_mut() {
                    if let Err(e) = t.toggle_selected() {
                        self.set_status(format!("tree: {e}"));
                    }
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Tab | KeyCode::Esc => {
                self.focus = Focus::Editor;
                self.set_status("foco: editor (Ctrl+B árvore · Ctrl+E editor)");
                true
            }
            KeyCode::PageUp => {
                if let Some(t) = self.tree.as_mut() {
                    t.move_selection(-10);
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::PageDown => {
                if let Some(t) = self.tree.as_mut() {
                    t.move_selection(10);
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::Home => {
                if let Some(t) = self.tree.as_mut() {
                    t.set_selected(0);
                    self.ensure_tree_visible();
                }
                true
            }
            KeyCode::End => {
                if let Some(t) = self.tree.as_mut() {
                    let n = t.flat_rows().len().saturating_sub(1);
                    t.set_selected(n);
                    self.ensure_tree_visible();
                }
                true
            }
            _ => false,
        }
    }

    fn handle_terminal_key(&mut self, key: KeyEvent) -> bool {
        // Ctrl+` e outros ctrl* deixam para o keymap
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return false;
        }
        if key.code == KeyCode::Esc {
            self.focus = Focus::Editor;
            return true;
        }
        let Some(term) = self.terminal.as_mut() else {
            return false;
        };
        match key.code {
            KeyCode::Enter => {
                let _ = term.write_str("\r");
            }
            KeyCode::Backspace => {
                let _ = term.write_bytes(&[0x7f]);
            }
            KeyCode::Tab => {
                let _ = term.write_str("\t");
            }
            KeyCode::Char(c) => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                let _ = term.write_str(s);
            }
            KeyCode::Up => {
                let _ = term.write_bytes(&[0x1b, b'[', b'A']);
            }
            KeyCode::Down => {
                let _ = term.write_bytes(&[0x1b, b'[', b'B']);
            }
            KeyCode::Right => {
                let _ = term.write_bytes(&[0x1b, b'[', b'C']);
            }
            KeyCode::Left => {
                let _ = term.write_bytes(&[0x1b, b'[', b'D']);
            }
            _ => return false,
        }
        true
    }

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
                if self.focus != Focus::Editor {
                    return Ok(());
                }
                self.quit_confirm_pending = false;
                self.close_tab_confirm = None;
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
                if self.overlay != Overlay::None {
                    self.overlay = Overlay::None;
                    return Ok(());
                }
                if self.focus != Focus::Editor {
                    self.focus = Focus::Editor;
                    return Ok(());
                }
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
                        self.refresh_git_and_index();
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
                self.insert_newline_smart()?;
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
            Action::ToggleTree => {
                self.show_tree = !self.show_tree;
                if self.show_tree {
                    self.focus = Focus::Tree;
                    self.set_status("árvore visível · foco na árvore (Ctrl+E volta ao editor)");
                } else {
                    if self.focus == Focus::Tree {
                        self.focus = Focus::Editor;
                    }
                    self.set_status("árvore oculta");
                }
            }
            Action::ToggleTerminal => {
                if let Some(term) = self.terminal.as_mut() {
                    term.toggle_visible();
                    if term.visible {
                        self.focus = Focus::Terminal;
                    } else if self.focus == Focus::Terminal {
                        self.focus = Focus::Editor;
                    }
                } else {
                    self.set_status("terminal unavailable");
                }
            }
            Action::FocusTree => {
                self.show_tree = true;
                if self.tree.is_none() {
                    self.tree = ProjectTree::open(&self.workspace, false).ok();
                }
                self.focus = Focus::Tree;
                self.set_status(
                    "foco: árvore · ↑↓/jk · Enter abrir · ←→ expandir/colapsar · Ctrl+E editor",
                );
            }
            Action::FocusEditor => {
                self.focus = Focus::Editor;
                self.set_status("foco: editor (Ctrl+B árvore · Ctrl+O abrir pasta)");
            }
            Action::FocusToggleTreeEditor => match self.focus {
                Focus::Tree => {
                    self.focus = Focus::Editor;
                    self.set_status("foco: editor");
                }
                _ => {
                    self.show_tree = true;
                    if self.tree.is_none() {
                        self.tree = ProjectTree::open(&self.workspace, false).ok();
                    }
                    self.focus = Focus::Tree;
                    self.set_status("foco: árvore");
                }
            },
            Action::FocusTerminal => {
                if let Some(term) = self.terminal.as_mut() {
                    term.visible = true;
                    self.focus = Focus::Terminal;
                }
            }
            Action::OpenFolder => {
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::OpenFolder,
                    buffer: self.workspace.display().to_string(),
                };
            }
            Action::ToggleSoftWrap => {
                self.soft_wrap = !self.soft_wrap;
                self.set_status(if self.soft_wrap {
                    "soft wrap: on"
                } else {
                    "soft wrap: off"
                });
            }
            Action::ToggleComment => {
                self.toggle_line_comment()?;
            }
            Action::NextTab => {
                self.store.activate_next_tab();
                self.scroll_y = 0;
            }
            Action::PrevTab => {
                self.store.activate_prev_tab();
                self.scroll_y = 0;
            }
            Action::NewTab => {
                self.store.open_empty();
                self.focus = Focus::Editor;
                self.scroll_y = 0;
            }
            Action::CloseTab => self.close_active_tab()?,
            Action::CommandPalette => {
                self.overlay = Overlay::CommandPalette {
                    query: String::new(),
                    selected: 0,
                };
            }
            Action::OpenFileFuzzy => {
                self.refresh_file_index();
                self.overlay = Overlay::OpenFile {
                    query: String::new(),
                    selected: 0,
                };
            }
            Action::TreeNewFile => {
                self.show_tree = true;
                self.focus = Focus::Tree;
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::NewFile,
                    buffer: String::new(),
                };
            }
            Action::TreeNewDir => {
                self.show_tree = true;
                self.focus = Focus::Tree;
                self.overlay = Overlay::Prompt {
                    kind: PromptKind::NewDir,
                    buffer: String::new(),
                };
            }
            Action::TreeRefresh => {
                if let Some(t) = self.tree.as_mut() {
                    if let Err(e) = t.refresh() {
                        self.set_status(format!("refresh: {e}"));
                    } else {
                        self.set_status("tree refreshed");
                    }
                }
                self.refresh_git_and_index();
            }
        }
        Ok(())
    }

    /// Troca o workspace para `path` (pasta no sistema).
    fn active_language(&self) -> LanguageId {
        self.store
            .active()
            .ok()
            .map(|d| detect_language(d.path()))
            .unwrap_or(LanguageId::Plain)
    }

    /// Enter inteligente: continua listas Markdown.
    fn insert_newline_smart(&mut self) -> Result<(), DocumentError> {
        let lang = self.active_language();
        if lang.is_markdown_family() {
            let (line, caret_line) = {
                let doc = self.store.active()?;
                let caret = doc.caret()?;
                (
                    doc.buffer().line_text(caret.line).unwrap_or_default(),
                    caret.line,
                )
            };
            if let Some(cont) = continue_list_on_enter(&line) {
                self.store.active_mut()?.insert_text(&format!("\n{cont}"))?;
                return Ok(());
            }
            // Linha só com marcador → sai da lista (apaga o marcador)
            if let Some(prefix) = oride_syntax::list_prefix(&line) {
                if line[prefix.len()..].trim().is_empty() {
                    let doc = self.store.active_mut()?;
                    let start = doc.buffer().line_to_byte(caret_line)?;
                    let end = oride_core::ByteOffset::new(start.as_usize() + line.len());
                    doc.set_selection(oride_core::Selection::new(start, end));
                    doc.delete_selection()?;
                    return Ok(());
                }
            }
        }
        self.store.active_mut()?.insert_text("\n")?;
        Ok(())
    }

    fn toggle_line_comment(&mut self) -> Result<(), DocumentError> {
        let lang = self.active_language();
        let open = match lang.line_comment() {
            Some(o) => o,
            None => {
                self.set_status("comentário não definido para esta linguagem");
                return Ok(());
            }
        };
        let close = lang.block_comment_close().unwrap_or("");
        let doc = self.store.active_mut()?;
        let caret = doc.caret()?;
        let line = doc.buffer().line_text(caret.line).unwrap_or_default();
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let body = line.trim_start();

        let new_line = if close.is_empty() {
            let open_t = open.trim_end();
            if let Some(rest) = body.strip_prefix(open_t) {
                let rest = rest.strip_prefix(' ').unwrap_or(rest);
                format!("{indent}{rest}")
            } else {
                format!("{indent}{open}{body}")
            }
        } else {
            let open_t = open.trim();
            let close_t = close.trim();
            if body.starts_with(open_t) && body.ends_with(close_t) {
                let inner = body
                    .strip_prefix(open_t)
                    .and_then(|s| s.strip_suffix(close_t))
                    .unwrap_or(body)
                    .trim();
                format!("{indent}{inner}")
            } else {
                format!("{indent}{open_t} {body} {close_t}")
            }
        };

        let start = doc.buffer().line_to_byte(caret.line)?;
        let end = oride_core::ByteOffset::new(start.as_usize() + line.len());
        doc.set_selection(oride_core::Selection::new(start, end));
        doc.delete_selection()?;
        doc.insert_text(&new_line)?;
        let head = doc.buffer().line_to_byte(caret.line)?;
        doc.set_selection(oride_core::Selection::caret(head));
        Ok(())
    }

    fn apply_language_defaults(&mut self, lang: LanguageId) {
        if lang.default_soft_wrap() {
            self.soft_wrap = true;
        }
    }

    fn open_workspace_folder(&mut self, path: PathBuf) {
        let path = if path.as_os_str().is_empty() {
            self.set_status("caminho vazio");
            return;
        } else {
            path
        };
        let path = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(e) => {
                self.set_status(format!("pasta inválida: {e}"));
                return;
            }
        };
        if !path.is_dir() {
            self.set_status(format!("não é pasta: {}", path.display()));
            return;
        }
        self.workspace = path;
        match ProjectTree::open(&self.workspace, false) {
            Ok(t) => self.tree = Some(t),
            Err(e) => {
                self.tree = None;
                self.set_status(format!("árvore: {e}"));
                return;
            }
        }
        self.show_tree = true;
        self.focus = Focus::Tree;
        self.tree_scroll = 0;
        self.refresh_git_and_index();
        // reinicia terminal no novo cwd se possível
        if let Some(old) = self.terminal.take() {
            drop(old);
        }
        self.terminal = EmbeddedTerminal::spawn(&self.workspace, 80, 12).ok();
        self.set_status(format!("projeto: {}", self.workspace.display()));
    }

    fn close_active_tab(&mut self) -> Result<(), DocumentError> {
        let id = self
            .store
            .active_id()
            .ok_or(DocumentError::NoActiveDocument)?;
        let dirty = self.store.get(id).map(|d| d.is_dirty()).unwrap_or(false);
        if dirty && self.close_tab_confirm != Some(id) {
            self.close_tab_confirm = Some(id);
            self.set_status("tab dirty — Ctrl+W again to close without save");
            return Ok(());
        }
        self.close_tab_confirm = None;
        let _ = self.store.close(id)?;
        if self.store.tab_ids().is_empty() {
            self.store.open_empty();
        }
        self.scroll_y = 0;
        Ok(())
    }

    fn refresh_git_and_index(&mut self) {
        self.git_status = status_map(&self.workspace);
        self.git_branch = current_branch(&self.workspace);
        self.refresh_file_index();
    }

    fn refresh_file_index(&mut self) {
        self.file_index = list_files_recursive(&self.workspace, false).unwrap_or_default();
    }

    fn ensure_tree_visible(&mut self) {
        let Some(tree) = &self.tree else { return };
        let sel = tree.selected_index();
        // height unknown; clamp scroll
        if sel < self.tree_scroll {
            self.tree_scroll = sel;
        } else if sel >= self.tree_scroll + 20 {
            self.tree_scroll = sel.saturating_sub(19);
        }
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
        self.tick();
        let area = frame.area();

        let term_h = self
            .terminal
            .as_ref()
            .filter(|t| t.visible)
            .map(|t| t.height_lines.min(area.height.saturating_sub(3)).max(3))
            .unwrap_or(0);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(term_h),
                Constraint::Length(1),
            ])
            .split(area);

        let body = main_chunks[0];
        let term_area = main_chunks[1];
        let status_area = main_chunks[2];

        let tree_w = if self.show_tree {
            self.tree_width.min(body.width / 2).max(12)
        } else {
            0
        };

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(tree_w), Constraint::Min(1)])
            .split(body);

        if tree_w > 0 {
            self.draw_tree(frame, body_chunks[0]);
        }
        self.draw_editor_column(frame, body_chunks[1]);

        if term_h > 0 {
            if let Some(term) = self.terminal.as_mut() {
                term.poll_output();
                let cols = term_area.width.max(20);
                let rows = term_area.height.max(2);
                term.resize(cols, rows);
                let lines = term.visible_lines(rows as usize);
                render_terminal_panel(
                    frame,
                    term_area,
                    &lines,
                    self.focus == Focus::Terminal,
                    &self.theme,
                );
            }
        }

        self.draw_status(frame, status_area);

        // overlays
        match &self.overlay {
            Overlay::None => {}
            Overlay::CommandPalette { query, selected } => {
                let items = self.palette_items(true, query);
                let view = PaletteView {
                    title: "commands",
                    query,
                    items: &items,
                    selected: *selected,
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::OpenFile { query, selected } => {
                let items = self.palette_items(false, query);
                let view = PaletteView {
                    title: "open file",
                    query,
                    items: &items,
                    selected: *selected,
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Prompt { kind, buffer } => {
                let title = match kind {
                    PromptKind::NewFile => "novo arquivo",
                    PromptKind::NewDir => "nova pasta",
                    PromptKind::OpenFolder => "abrir pasta de projeto (caminho)",
                };
                let items: &[String] = &[];
                let view = PaletteView {
                    title,
                    query: buffer,
                    items,
                    selected: 0,
                };
                render_palette(frame, area, &view, &self.theme);
            }
        }
    }

    fn draw_tree(&mut self, frame: &mut Frame, area: Rect) {
        let Some(tree) = &self.tree else {
            return;
        };
        let rows = tree.flat_rows();
        let visible = area.height.saturating_sub(2) as usize;
        let sel = tree.selected_index();
        if sel < self.tree_scroll {
            self.tree_scroll = sel;
        } else if visible > 0 && sel >= self.tree_scroll + visible {
            self.tree_scroll = sel + 1 - visible;
        }
        let view = TreeView {
            title: tree.root_name(),
            rows: &rows,
            selected: tree.selected_index(),
            scroll: self.tree_scroll,
            use_nerd_icons: self.use_nerd_icons,
            git: &self.git_status,
            workspace_root: &self.workspace,
            focused: self.focus == Focus::Tree,
        };
        render_tree(frame, area, &view, &self.theme);
    }

    fn draw_editor_column(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let tabs = self.store.tab_summaries();
        render_tabs(frame, chunks[0], &tabs, &self.theme);

        let editor_area = chunks[1];
        self.last_editor_height = editor_area.height as usize;
        self.ensure_cursor_visible();

        // Atualiza syntax highlight a partir do buffer ativo
        let (lang, source) = match self.store.active() {
            Ok(d) => (detect_language(d.path()), d.buffer().as_string()),
            Err(_) => return,
        };
        self.highlight.update(lang, &source);

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
            highlights: self.highlight.spans(),
            show_cursor: self.focus == Focus::Editor && self.overlay == Overlay::None,
            soft_wrap: self.soft_wrap,
        };
        render_editor(frame, editor_area, &view, &self.theme);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let doc = self.store.active().ok();
        let caret = doc.and_then(|d| d.caret().ok()).unwrap_or_default();
        let title = doc.map(|d| d.tab_title()).unwrap_or_else(|| "oride".into());
        let dirty = doc.map(|d| d.is_dirty()).unwrap_or(false);
        let branch = self
            .git_branch
            .as_deref()
            .map(|b| format!("  git:{b}"))
            .unwrap_or_default();
        let focus = match self.focus {
            Focus::Editor => "editor",
            Focus::Tree => "árvore",
            Focus::Terminal => "term",
        };
        let lang = self.highlight.language().as_str();

        // Caminho do item selecionado na árvore (localização clara)
        let tree_sel = self.tree.as_ref().and_then(|t| {
            t.selected_row().map(|r| {
                let rel = r.path.strip_prefix(&self.workspace).unwrap_or(&r.path);
                let kind = if r.is_dir { "dir" } else { "file" };
                format!("▶ {kind}:{}", rel.display())
            })
        });

        let mut message = self.status_message.clone();
        if message.is_none() {
            message = Some(match (self.focus, tree_sel) {
                (Focus::Tree, Some(sel)) => {
                    format!("{focus} · {sel} · ↑↓ navegar · Enter abrir · Ctrl+E editor")
                }
                _ => format!(
                    "{focus} · {lang}{branch} · Ctrl+B árvore · Ctrl+E editor · Ctrl+O pasta"
                ),
            });
        }
        let status = StatusModel {
            title,
            dirty,
            line: caret.line,
            column: caret.column,
            message,
            help_hint: String::new(),
        };
        render_status(frame, area, &status, &self.theme);
    }
}

fn build_default_keymap() -> Keymap {
    let defaults = Config::default();
    Keymap::from_string_map(defaults.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .expect("default key bindings must parse")
}

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_ascii_lowercase();
    let c = candidate.to_ascii_lowercase();
    if c.contains(&q) {
        return true;
    }
    // subsequence
    let mut it = c.chars();
    for qc in q.chars() {
        loop {
            match it.next() {
                Some(cc) if cc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
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
    fn multi_tab_next() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::NewTab));
        assert_eq!(app.store.tab_ids().len(), 2);
        let a = app.store.active_id();
        app.apply(KeyCommand::Action(Action::NextTab));
        assert_ne!(app.store.active_id(), a);
    }

    #[test]
    fn close_dirty_tab_needs_confirm() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('x'));
        app.apply(KeyCommand::Action(Action::CloseTab));
        assert_eq!(app.store.tab_ids().len(), 1);
        assert!(app.store.active().unwrap().is_dirty());
        app.apply(KeyCommand::Action(Action::CloseTab));
        // still one tab (empty recreated) or empty opened
        assert!(!app.store.tab_ids().is_empty());
    }

    #[test]
    fn command_palette_filters() {
        let app = App::new_empty();
        let items = app.palette_items(true, "tab");
        assert!(items.iter().any(|i| i.to_lowercase().contains("tab")));
    }

    #[test]
    fn fuzzy_subsequence() {
        assert!(fuzzy_match("ore", "oride-core"));
        assert!(!fuzzy_match("zzz", "oride"));
    }

    #[test]
    fn focus_tree_and_editor_actions() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::FocusTree));
        assert_eq!(app.focus, Focus::Tree);
        assert!(app.show_tree);
        app.apply(KeyCommand::Action(Action::FocusEditor));
        assert_eq!(app.focus, Focus::Editor);
        app.apply(KeyCommand::Action(Action::FocusToggleTreeEditor));
        assert_eq!(app.focus, Focus::Tree);
        app.apply(KeyCommand::Action(Action::FocusToggleTreeEditor));
        assert_eq!(app.focus, Focus::Editor);
    }

    #[test]
    fn open_folder_prompt() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::OpenFolder));
        assert!(matches!(
            app.overlay,
            Overlay::Prompt {
                kind: PromptKind::OpenFolder,
                ..
            }
        ));
    }

    #[test]
    fn map_focus_and_open_folder_keys() {
        let app = App::new_empty();
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('b'))),
            Some(KeyCommand::Action(Action::FocusTree))
        );
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('e'))),
            Some(KeyCommand::Action(Action::FocusEditor))
        );
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('o'))),
            Some(KeyCommand::Action(Action::OpenFolder))
        );
    }
}
