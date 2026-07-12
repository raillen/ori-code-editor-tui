//! Estado da aplicação e despacho de teclas (testável sem TTY).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use oride_config::{resolve_indent_for_file, Config, EditorIndent};
use oride_core::{DocumentError, DocumentId, DocumentStore};
use oride_fs::{list_files_recursive, CreateKind, ProjectTree};
use oride_git::{blame_line, current_branch, diff_file, scm_entries, status_map, GitFileStatus};
use oride_keymap::{parse_action, Action, Keymap, ResolvedKey};
use oride_lsp::{Diagnostic, LspClient, LspEvent, Position as LspPos};
use oride_plugin::{builtin_host, PluginCtx, PluginHook, PluginHost};
use oride_search::{format_hit_label, search_project, SearchHit, SearchQuery};
use oride_syntax::{
    continue_list_on_enter, detect_language, render_preview_lines, HighlightEngine, LanguageId,
};
use oride_terminal::EmbeddedTerminal;
use oride_ui::{
    render_context_banner, render_editor, render_find_modal, render_md_preview, render_menu_bar,
    render_menu_dropdown, render_mini_modal, render_palette, render_scm_panel, render_status,
    render_tabs, render_terminal_panel, render_tree, render_which_key, EditorView, FindModalView,
    MdPreviewView, MiniModalView, PaletteView, ScmItem, StatusModel, TreeView, UiTheme,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use crate::browser::{BrowseAction, BrowseMode, PathBrowser};
use crate::clipboard;
use crate::disk_watch::DiskWatch;
use crate::find::FindState;
use crate::jump_list::{Jump, JumpList};
use crate::menus::default_menus;
use crate::session::Session;
use crate::split::{SplitOrientation, SplitState};

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
    Scm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Overlay {
    None,
    CommandPalette {
        query: String,
        selected: usize,
    },
    /// Navegador de pastas (workspace) ou arquivos.
    Browse(PathBrowser),
    Prompt {
        kind: PromptKind,
        buffer: String,
    },
    /// Lista completa de keybinds (filtro + scroll).
    Help {
        query: String,
        selected: usize,
    },
    /// Find/replace compacto (barra no rodapé; estado em `App.find`).
    Find,
    /// Busca no projeto (Ctrl+Shift+F).
    ProjectFind {
        query: String,
        selected: usize,
        case_sensitive: bool,
        use_regex: bool,
        hits: Vec<SearchHit>,
        status: String,
    },
    Diagnostics {
        selected: usize,
    },
    Completion {
        items: Vec<String>,
        selected: usize,
    },
    Hover {
        text: String,
    },
    /// Arquivo mudou no disco — Enter recarrega, Esc ignora.
    ReloadConfirm {
        path: PathBuf,
    },
    /// Lista de buffers abertos (fuzzy).
    BufferPicker {
        query: String,
        selected: usize,
    },
    /// Diff git read-only.
    Diff {
        path: PathBuf,
        lines: Vec<String>,
        scroll: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    NewFile,
    NewDir,
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
    /// Altura útil do texto no editor (linhas visuais, sem bordas/tabs).
    last_editor_height: usize,
    /// Largura útil do texto (sem gutter) — para soft-wrap no scroll.
    last_editor_text_width: usize,
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
    show_md_preview: bool,
    preview_scroll: usize,
    find: FindState,
    disk_watch: DiskWatch,
    lsp: Option<LspClient>,
    diagnostics: Vec<(PathBuf, Diagnostic)>,
    show_diagnostics: bool,
    lsp_doc_version: i32,
    pending_reload: Option<PathBuf>,
    plugin_host: PluginHost,
    split: SplitState,
    /// Painel SCM à direita.
    show_scm: bool,
    scm_selected: usize,
    scm_width: u16,
    scm_cache: Vec<(GitFileStatus, PathBuf)>,
    /// Menu bar: Some((menu_idx, item_idx)).
    menu_open: Option<(usize, usize)>,
    show_which_key: bool,
    show_welcome: bool,
    jump_list: JumpList,
    menus: Vec<oride_ui::MenuColumn>,
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
        let theme = UiTheme::from_config_parts(&config.theme_ui, &config.syntax)
            .unwrap_or_else(|_| UiTheme::default());
        let keymap =
            Keymap::from_string_map(config.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .unwrap_or_else(|_| build_default_keymap());

        let show_hidden = config.tree.show_hidden;
        let tree = ProjectTree::open(&workspace, show_hidden).ok();
        let git_status = if config.tree.git_status {
            status_map(&workspace)
        } else {
            HashMap::new()
        };
        let git_branch = current_branch(&workspace);
        let file_index = list_files_recursive(&workspace, show_hidden).unwrap_or_default();

        let term_h = config.terminal.default_height.max(3);
        let terminal = EmbeddedTerminal::spawn(&workspace, 80, term_h)
            .ok()
            .map(|mut t| {
                t.height_lines = term_h;
                t
            });

        let disk_watch = DiskWatch::start(&workspace);
        let lsp = if config.lsp.enabled {
            match LspClient::spawn(
                &config.lsp.oriscript_command,
                &workspace,
                config.lsp.timeout_ms,
            ) {
                Ok(c) => Some(c),
                Err(e) => {
                    // silencioso no boot; status depois se usuário pedir LSP
                    let _ = e;
                    None
                }
            }
        } else {
            None
        };

        let active_id = store
            .active_id()
            .unwrap_or_else(|| oride_core::DocumentId::from_raw(0));
        let scm_cache = if config.tree.git_status {
            scm_entries(&workspace)
        } else {
            Vec::new()
        };
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
            config: config.clone(),
            last_editor_height: 20,
            last_editor_text_width: 80,
            focus: Focus::Editor,
            show_tree: true,
            tree,
            tree_scroll: 0,
            workspace,
            git_status,
            git_branch,
            use_nerd_icons: true,
            tree_width: config.tree.width.max(8),
            terminal,
            overlay: Overlay::None,
            file_index,
            highlight: HighlightEngine::new(),
            soft_wrap: config.soft_wrap,
            show_md_preview: false,
            preview_scroll: 0,
            find: FindState::default(),
            disk_watch,
            lsp,
            diagnostics: Vec::new(),
            show_diagnostics: false,
            lsp_doc_version: 1,
            pending_reload: None,
            plugin_host: builtin_host(),
            split: SplitState::single(active_id),
            show_scm: false,
            scm_selected: 0,
            scm_width: 28,
            scm_cache,
            menu_open: None,
            show_which_key: false,
            // Welcome só no boot interativo (`new_empty_or_session`), não em testes/unitários.
            show_welcome: false,
            jump_list: JumpList::default(),
            menus: default_menus(),
        }
    }

    /// Restaura sessão salva se existir; senão buffer vazio no CWD.
    pub fn new_empty_or_session() -> Self {
        if let Some(session) = Session::load() {
            if session.workspace.is_dir() {
                let mut app = match Self::open_workspace(session.workspace.clone()) {
                    Ok(a) => a,
                    Err(_) => return Self::new_empty(),
                };
                for f in &session.files {
                    if f.is_file() {
                        let _ = app.store.open_path(f);
                        let lang = detect_language(Some(f.as_path()));
                        app.apply_language_defaults(lang);
                    }
                }
                let paths = app.store.open_paths();
                if !paths.is_empty() {
                    let idx = session.active_index.min(paths.len() - 1);
                    let _ = app.store.open_path(&paths[idx]);
                }
                app.show_welcome = false;
                app.set_status("sessão restaurada");
                return app;
            }
        }
        let mut app = Self::new_empty();
        app.show_welcome = true;
        app
    }

    pub fn persist_session(&self) {
        let files = self.store.open_paths();
        let active_index = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(|p| files.iter().position(|f| f == p)))
            .flatten()
            .unwrap_or(0);
        let session = Session::from_workspace(&self.workspace, files, active_index);
        let _ = session.save();
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
        self.poll_disk_changes();
        self.poll_lsp_events();
    }

    fn poll_disk_changes(&mut self) {
        let changed = self.disk_watch.poll();
        let open: Vec<PathBuf> = self.store.open_paths();
        for path in changed {
            if open
                .iter()
                .any(|p| p == &path || p.ends_with(&path) || path.ends_with(p))
            {
                // se dirty, pede confirmação; senão recarrega
                let dirty = self
                    .store
                    .active()
                    .ok()
                    .filter(|d| d.path() == Some(path.as_path()))
                    .map(|d| d.is_dirty())
                    .unwrap_or(false);
                if dirty {
                    self.pending_reload = Some(path.clone());
                    self.overlay = Overlay::ReloadConfirm { path };
                    self.set_status("arquivo mudou no disco — Enter recarrega · Esc ignora");
                } else if let Ok(doc) = self.store.active_mut() {
                    if doc.path() == Some(path.as_path()) {
                        let _ = doc.reload_from_disk();
                        self.lsp_sync_active();
                        self.set_status(format!("recarregado: {}", path.display()));
                    }
                }
                break;
            }
        }
    }

    fn poll_lsp_events(&mut self) {
        let Some(lsp) = self.lsp.as_mut() else {
            return;
        };
        for ev in lsp.poll_events() {
            match ev {
                LspEvent::Diagnostics { uri, diagnostics } => {
                    let path = uri_to_path(&uri);
                    self.diagnostics.retain(|(p, _)| Some(p) != path.as_ref());
                    if let Some(p) = path {
                        for d in diagnostics {
                            self.diagnostics.push((p.clone(), d));
                        }
                    }
                }
                LspEvent::Exited => {
                    self.lsp = None;
                    self.set_status("LSP saiu");
                    break;
                }
                LspEvent::ServerMessage(m) => self.set_status(m),
            }
        }
    }

    fn lsp_sync_active(&mut self) {
        let Some(lsp) = self.lsp.as_mut() else {
            return;
        };
        let Ok(doc) = self.store.active() else {
            return;
        };
        let Some(path) = doc.path().map(Path::to_path_buf) else {
            return;
        };
        let text = doc.buffer().as_string();
        self.lsp_doc_version += 1;
        let ver = self.lsp_doc_version;
        let _ = lsp.did_change(&path, ver, &text);
    }

    fn lsp_open_active(&mut self) {
        let Some(lsp) = self.lsp.as_mut() else {
            return;
        };
        let Ok(doc) = self.store.active() else {
            return;
        };
        let Some(path) = doc.path().map(Path::to_path_buf) else {
            return;
        };
        if detect_language(Some(path.as_path())) != LanguageId::OriScript {
            return;
        }
        let lang = "oriscript";
        let text = doc.buffer().as_string();
        let _ = lsp.did_open(&path, lang, &text);
    }

    fn apply_editorconfig_for_active(&mut self) {
        if !self.config.editor.use_editorconfig {
            return;
        }
        let Ok(doc) = self.store.active() else {
            return;
        };
        let Some(path) = doc.path() else {
            return;
        };
        let ind = resolve_indent_for_file(
            path,
            EditorIndent {
                tab_size: self.config.editor.tab_size,
                insert_spaces: self.config.editor.insert_spaces,
            },
        );
        self.config.editor.tab_size = ind.tab_size;
        self.config.editor.insert_spaces = ind.insert_spaces;
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
        // Welcome / which-key / menu têm prioridade sobre o resto
        if self.show_welcome {
            self.handle_welcome_key(key);
            return;
        }
        if self.show_which_key {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.show_which_key = false;
            }
            return;
        }
        if self.menu_open.is_some() {
            self.handle_menu_key(key);
            return;
        }
        // Alt+letra abre menu (File/Edit/…)
        if key.modifiers.contains(KeyModifiers::ALT)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::SHIFT)
        {
            if let KeyCode::Char(c) = key.code {
                let c = c.to_ascii_lowercase();
                if let Some(idx) = self.menus.iter().position(|m| m.hotkey == c) {
                    self.menu_open = Some((idx, 0));
                    return;
                }
            }
        }
        if self.handle_overlay_key(key) {
            return;
        }
        // Preview MD: Alt+↑/↓ ajusta offset fino em cima do scroll do editor
        if self.show_md_preview
            && key.modifiers.contains(KeyModifiers::ALT)
            && matches!(
                key.code,
                KeyCode::PageUp | KeyCode::PageDown | KeyCode::Up | KeyCode::Down
            )
        {
            match key.code {
                KeyCode::PageUp | KeyCode::Up => {
                    // offset pode ser "negativo" via saturating: usamos i32 interno?
                    // mantém usize: Alt+Up reduz offset; se 0, não sobe além do editor
                    self.preview_scroll = self.preview_scroll.saturating_sub(3);
                }
                KeyCode::PageDown | KeyCode::Down => {
                    self.preview_scroll = self.preview_scroll.saturating_add(3);
                }
                _ => {}
            }
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
            Focus::Scm => {
                if self.handle_scm_key(key) {
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
            Overlay::CommandPalette { .. } => {
                self.handle_palette_key(key);
                true
            }
            Overlay::Browse(_) => {
                self.handle_browse_key(key);
                true
            }
            Overlay::Prompt { .. } => {
                self.handle_prompt_key(key);
                true
            }
            Overlay::Help { .. } => {
                self.handle_help_key(key);
                true
            }
            Overlay::Find => {
                self.handle_find_key(key);
                true
            }
            Overlay::ProjectFind { .. } => {
                self.handle_project_find_key(key);
                true
            }
            Overlay::Diagnostics { .. } => {
                self.handle_diagnostics_key(key);
                true
            }
            Overlay::Completion { .. } => {
                self.handle_completion_key(key);
                true
            }
            Overlay::Hover { .. } => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                    self.overlay = Overlay::None;
                }
                true
            }
            Overlay::BufferPicker { .. } => {
                self.handle_buffer_picker_key(key);
                true
            }
            Overlay::Diff { .. } => {
                self.handle_diff_key(key);
                true
            }
            Overlay::ReloadConfirm { path } => {
                match key.code {
                    KeyCode::Enter => {
                        let path = path.clone();
                        self.overlay = Overlay::None;
                        if let Ok(doc) = self.store.active_mut() {
                            if doc.path() == Some(path.as_path()) {
                                match doc.reload_from_disk() {
                                    Ok(()) => {
                                        self.lsp_sync_active();
                                        self.set_status(format!("recarregado: {}", path.display()));
                                    }
                                    Err(e) => self.set_status(format!("reload: {e}")),
                                }
                            }
                        }
                        self.pending_reload = None;
                    }
                    KeyCode::Esc => {
                        self.overlay = Overlay::None;
                        self.pending_reload = None;
                        self.set_status("reload ignorado");
                    }
                    _ => {}
                }
                true
            }
        }
    }

    fn handle_browse_key(&mut self, key: KeyEvent) {
        let Overlay::Browse(browser) = &mut self.overlay else {
            return;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let mode = browser.mode;

        // Confirmar pasta/save: Ctrl+Enter, Alt+Enter, F2, Ctrl+O, Ctrl+S (save as).
        // Terminais variam: Ctrl+Enter pode vir como Enter+Ctrl, ou Char('\n'/j/m).
        let confirm = matches!(key.code, KeyCode::F(2))
            || (matches!(key.code, KeyCode::Enter) && (ctrl || alt))
            || (ctrl && matches!(key.code, KeyCode::Char('o')))
            || (ctrl && matches!(key.code, KeyCode::Char('s')) && mode == BrowseMode::SaveAs)
            || (ctrl
                && matches!(
                    key.code,
                    KeyCode::Char('\n')
                        | KeyCode::Char('\r')
                        | KeyCode::Char('j')
                        | KeyCode::Char('m')
                ));

        let action = match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                None
            }
            _ if confirm => Some(browser.confirm_folder()),
            KeyCode::Up | KeyCode::Char('k') if !ctrl => {
                browser.move_selection(-1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => {
                browser.move_selection(1);
                None
            }
            KeyCode::PageUp => {
                browser.move_selection(-10);
                None
            }
            KeyCode::PageDown => {
                browser.move_selection(10);
                None
            }
            KeyCode::Backspace if browser.filter.is_empty() => {
                browser.go_parent();
                None
            }
            KeyCode::Backspace => {
                browser.filter.pop();
                browser.selected = 0;
                None
            }
            // Save as: Enter com nome → salva; Right/l → entra na pasta
            KeyCode::Enter if mode == BrowseMode::SaveAs => {
                if browser.filter.trim().is_empty() {
                    self.set_status("save as: digite o nome do arquivo");
                    None
                } else {
                    Some(browser.confirm_folder())
                }
            }
            KeyCode::Right | KeyCode::Char('l') if !ctrl => Some(browser.activate()),
            KeyCode::Enter => Some(browser.activate()),
            KeyCode::Char(c) if !ctrl && !alt && !c.is_control() => {
                browser.filter.push(c);
                browser.selected = 0;
                None
            }
            _ => None,
        };

        if let Some(action) = action {
            self.apply_browse_action(action);
        }
    }

    /// Abre o browser “salvar como” (path + nome do arquivo).
    fn open_save_as_browser(&mut self) {
        let (start_dir, name) = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(|p| p.to_path_buf()))
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "untitled.txt".into());
                let dir = p
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| self.workspace.clone());
                (dir, name)
            })
            .unwrap_or_else(|| (self.workspace.clone(), "untitled.txt".into()));
        // canonicalize pode falhar se o dir ainda não existe — PathBrowser já trata
        let mut browser = PathBrowser::new(&start_dir, BrowseMode::SaveAs);
        browser.filter = name;
        self.set_status(format!(
            "{} · (atalhos: Ctrl+Shift+S · F12 · Alt+Shift+S)",
            browser.hint()
        ));
        self.overlay = Overlay::Browse(browser);
    }

    fn apply_browse_action(&mut self, action: BrowseAction) {
        match action {
            BrowseAction::Stay => {}
            BrowseAction::OpenFile(path) => {
                self.overlay = Overlay::None;
                if let Err(e) = self.store.open_path(&path) {
                    self.set_status(format!("open: {e}"));
                } else {
                    let lang = detect_language(Some(path.as_path()));
                    self.apply_language_defaults(lang);
                    self.apply_editorconfig_for_active();
                    self.lsp_open_active();
                    self.focus = Focus::Editor;
                    self.scroll_y = 0;
                    self.set_status(format!("aberto · {}", path.display()));
                }
            }
            BrowseAction::OpenFolder(path) => {
                self.overlay = Overlay::None;
                self.open_workspace_folder(path);
            }
            BrowseAction::SaveAsPath(path) => {
                self.overlay = Overlay::None;
                match self.store.active_mut() {
                    Ok(doc) => match doc.save_to(Some(&path)) {
                        Ok(()) => {
                            self.set_status(format!("salvo: {}", path.display()));
                            self.refresh_git_and_index();
                        }
                        Err(e) => self.set_status(format!("save as: {e}")),
                    },
                    Err(e) => self.set_status(format!("save as: {e}")),
                }
            }
        }
    }

    fn handle_find_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            KeyCode::Tab => {
                if !self.find.show_replace {
                    self.find.show_replace = true;
                }
                self.find.focus_replace = !self.find.focus_replace;
            }
            KeyCode::F(3) if shift => {
                self.jump_find(false);
            }
            KeyCode::F(3) | KeyCode::Enter if !alt && !ctrl => {
                self.jump_find(true);
            }
            // Alt+Enter: replace one · Ctrl+Alt+Enter: replace all
            KeyCode::Enter if alt && ctrl => {
                self.replace_all_matches();
            }
            KeyCode::Enter if alt => {
                self.replace_current_match();
            }
            KeyCode::Char('c') if alt && !ctrl => {
                self.find.toggle_case();
                self.recompute_find_and_jump();
            }
            KeyCode::Char('a') if alt && !ctrl => {
                self.find.toggle_accents();
                self.recompute_find_and_jump();
            }
            KeyCode::Char('w') if alt && !ctrl => {
                self.find.toggle_whole_word();
                self.recompute_find_and_jump();
            }
            KeyCode::Char('r') if alt && !ctrl => {
                self.find.toggle_regex();
                self.recompute_find_and_jump();
            }
            KeyCode::Char('h') if ctrl => {
                self.find.show_replace = !self.find.show_replace;
                if self.find.show_replace {
                    self.find.focus_replace = true;
                }
            }
            KeyCode::Backspace => {
                if self.find.focus_replace && self.find.show_replace {
                    self.find.replace.pop();
                } else {
                    self.find.query.pop();
                    self.recompute_find_and_jump();
                }
            }
            KeyCode::Char(c) if !ctrl && !alt && !c.is_control() => {
                if self.find.focus_replace && self.find.show_replace {
                    self.find.replace.push(c);
                } else {
                    self.find.query.push(c);
                    self.recompute_find_and_jump();
                }
            }
            _ => {}
        }
        self.set_status(self.find.status());
    }

    fn recompute_find(&mut self) {
        let text = self
            .store
            .active()
            .map(|d| d.buffer().as_string())
            .unwrap_or_default();
        self.find.recompute(&text);
    }

    fn recompute_find_and_jump(&mut self) {
        self.recompute_find();
        if let Some(m) = self.find.current_match() {
            if let Ok(doc) = self.store.active_mut() {
                doc.select_byte_range(
                    oride_core::ByteOffset::new(m.start),
                    oride_core::ByteOffset::new(m.end),
                );
            }
            self.ensure_cursor_visible();
        }
    }

    fn jump_find(&mut self, forward: bool) {
        self.recompute_find();
        let m = if forward {
            self.find.next()
        } else {
            self.find.prev()
        };
        if let Some(m) = m {
            if let Ok(doc) = self.store.active_mut() {
                doc.select_byte_range(
                    oride_core::ByteOffset::new(m.start),
                    oride_core::ByteOffset::new(m.end),
                );
            }
            self.ensure_cursor_visible();
        }
        self.set_status(self.find.status());
    }

    fn replace_current_match(&mut self) {
        self.recompute_find();
        let Some(m) = self.find.current_match() else {
            self.set_status("replace: nenhuma ocorrência");
            return;
        };
        let repl = self.find.replace.clone();
        if let Ok(doc) = self.store.active_mut() {
            doc.select_byte_range(
                oride_core::ByteOffset::new(m.start),
                oride_core::ByteOffset::new(m.end),
            );
            let _ = doc.delete_selection();
            let _ = doc.insert_text(&repl);
        }
        self.recompute_find();
        if let Some(m) = self.find.current_match() {
            if let Ok(doc) = self.store.active_mut() {
                doc.select_byte_range(
                    oride_core::ByteOffset::new(m.start),
                    oride_core::ByteOffset::new(m.end),
                );
            }
        }
        self.set_status(format!("replaced 1 · {}", self.find.status()));
    }

    fn replace_all_matches(&mut self) {
        self.recompute_find();
        if self.find.matches.is_empty() {
            self.set_status("replace all: 0 ocorrências");
            return;
        }
        let repl = self.find.replace.clone();
        let matches: Vec<_> = self.find.matches.clone();
        let n = matches.len();
        // De trás para frente para não invalidar offsets
        if let Ok(doc) = self.store.active_mut() {
            for m in matches.into_iter().rev() {
                doc.select_byte_range(
                    oride_core::ByteOffset::new(m.start),
                    oride_core::ByteOffset::new(m.end),
                );
                let _ = doc.delete_selection();
                let _ = doc.insert_text(&repl);
            }
        }
        self.recompute_find();
        self.set_status(format!("replace all: {n} ocorrências"));
    }

    fn handle_palette_key(&mut self, key: KeyEvent) {
        let Overlay::CommandPalette { query, selected } = &self.overlay else {
            return;
        };
        let query = query.clone();
        let selected = *selected;

        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            KeyCode::Enter => {
                let items = self.command_palette_items(&query);
                if let Some(item) = items.get(selected).cloned() {
                    self.overlay = Overlay::None;
                    if let Some(action) = Action::palette_actions()
                        .iter()
                        .find(|a| a.palette_label() == item)
                        .copied()
                    {
                        let _ = self.apply_action(action);
                    } else if let Some(cmd_id) = self.plugin_host.command_id_for_label(&item) {
                        self.run_plugin_command(cmd_id);
                    } else {
                        // tenta id direto
                        self.run_plugin_command(&item);
                    }
                }
            }
            KeyCode::Up => {
                let len = self.command_palette_items(&query).len();
                let selected = if len == 0 {
                    0
                } else {
                    selected.saturating_sub(1)
                };
                self.overlay = Overlay::CommandPalette { query, selected };
            }
            KeyCode::Down => {
                let len = self.command_palette_items(&query).len();
                let selected = if len == 0 {
                    0
                } else {
                    (selected + 1).min(len - 1)
                };
                self.overlay = Overlay::CommandPalette { query, selected };
            }
            KeyCode::Backspace => {
                let mut q = query;
                q.pop();
                self.overlay = Overlay::CommandPalette {
                    query: q,
                    selected: 0,
                };
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut q = query;
                q.push(c);
                self.overlay = Overlay::CommandPalette {
                    query: q,
                    selected: 0,
                };
            }
            _ => {}
        }
    }

    fn command_palette_items(&self, query: &str) -> Vec<String> {
        let mut items: Vec<String> = Action::palette_actions()
            .iter()
            .map(|a| a.palette_label().to_string())
            .collect();
        for cmd in self.plugin_host.palette_commands() {
            items.push(cmd.label.to_string());
        }
        items.retain(|l| fuzzy_match(query, l));
        items.sort();
        items
    }

    /// Linhas `chord → ação` a partir do keymap efetivo (defaults + config).
    fn keybind_list_items(&self, query: &str) -> Vec<String> {
        self.keymap
            .list_bindings()
            .into_iter()
            .map(|(chord, action)| format!("{:<24}  {}", chord, action.palette_label()))
            .filter(|line| fuzzy_match(query, line))
            .collect()
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        let Overlay::Help { query, selected } = &self.overlay else {
            return;
        };
        let mut query = query.clone();
        let mut selected = *selected;
        let items_len = self.keybind_list_items(&query).len();

        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                // 'q' só fecha se a query estiver vazia (senão digita q no filtro)
                if matches!(key.code, KeyCode::Char('q')) && !query.is_empty() {
                    query.push('q');
                    selected = 0;
                    self.overlay = Overlay::Help { query, selected };
                    return;
                }
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Up => {
                selected = selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if items_len > 0 {
                    selected = (selected + 1).min(items_len - 1);
                }
            }
            KeyCode::PageUp => {
                selected = selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if items_len > 0 {
                    selected = (selected + 10).min(items_len - 1);
                }
            }
            KeyCode::Home => selected = 0,
            KeyCode::End if items_len > 0 => selected = items_len - 1,
            KeyCode::Backspace => {
                query.pop();
                selected = 0;
            }
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                    && !c.is_control() =>
            {
                query.push(c);
                selected = 0;
            }
            _ => {}
        }

        let len = self.keybind_list_items(&query).len();
        if len == 0 {
            selected = 0;
        } else {
            selected = selected.min(len - 1);
        }
        self.overlay = Overlay::Help { query, selected };
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
                if let Some(tree) = self.tree.as_mut() {
                    let create = match kind {
                        PromptKind::NewFile => CreateKind::File,
                        PromptKind::NewDir => CreateKind::Directory,
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
        // Esc (sem modificadores) volta ao editor
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            self.focus = Focus::Editor;
            self.set_status("foco: editor");
            return true;
        }

        // Atalhos IDE permitidos mesmo com foco no terminal
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            if let Some(cmd) = self.map_key(key) {
                if let KeyCommand::Action(action) = cmd {
                    if matches!(
                        action,
                        Action::ToggleTerminal
                            | Action::FocusEditor
                            | Action::FocusTree
                            | Action::FocusToggleTreeEditor
                            | Action::ToggleTree
                            | Action::ToggleScm
                            | Action::FocusScm
                            | Action::CommandPalette
                            | Action::Help
                            | Action::Quit
                            | Action::WhichKey
                            | Action::Welcome
                            | Action::TerminalGrow
                            | Action::TerminalShrink
                            | Action::NextTab
                            | Action::PrevTab
                    ) {
                        self.apply(cmd);
                        return true;
                    }
                }
            }
        }

        let Some(term) = self.terminal.as_mut() else {
            return false;
        };

        // Ctrl+A–Z → byte de controle (shell real)
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
        {
            if let KeyCode::Char(c) = key.code {
                let b = (c.to_ascii_lowercase() as u8) & 0x1f;
                if b != 0 {
                    let _ = term.write_bytes(&[b]);
                    return true;
                }
            }
        }

        // Alt+char → ESC + char (meta no shell)
        if key.modifiers.contains(KeyModifiers::ALT)
            && !key.modifiers.contains(KeyModifiers::CONTROL)
        {
            if let KeyCode::Char(c) = key.code {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                let mut data = Vec::with_capacity(1 + s.len());
                data.push(0x1b);
                data.extend_from_slice(s.as_bytes());
                let _ = term.write_bytes(&data);
                return true;
            }
        }

        match key.code {
            KeyCode::Enter => {
                let _ = term.write_str("\r");
            }
            KeyCode::Backspace => {
                let _ = term.write_bytes(&[0x7f]);
            }
            KeyCode::Delete => {
                let _ = term.write_bytes(&[0x1b, b'[', b'3', b'~']);
            }
            KeyCode::Tab => {
                let _ = term.write_str("\t");
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
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
            KeyCode::Home => {
                let _ = term.write_bytes(&[0x1b, b'[', b'H']);
            }
            KeyCode::End => {
                let _ = term.write_bytes(&[0x1b, b'[', b'F']);
            }
            KeyCode::PageUp => {
                let _ = term.write_bytes(&[0x1b, b'[', b'5', b'~']);
            }
            KeyCode::PageDown => {
                let _ = term.write_bytes(&[0x1b, b'[', b'6', b'~']);
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
                // Sem path → Save As (evita “no path” silencioso / modal que não abre)
                let needs_path = self
                    .store
                    .active()
                    .map(|d| d.path().is_none())
                    .unwrap_or(true);
                if needs_path {
                    self.open_save_as_browser();
                    return Ok(());
                }
                let doc = self.store.active_mut()?;
                match doc.save_to(None) {
                    Ok(()) => {
                        self.quit_confirm_pending = false;
                        if let Ok(doc) = self.store.active() {
                            if let Some(path) = doc.path() {
                                self.disk_watch.mark_saved(path);
                                if let Some(lsp) = self.lsp.as_mut() {
                                    let text = doc.buffer().as_string();
                                    let _ = lsp.did_save(path, &text);
                                }
                            }
                        }
                        if self.config.editor.format_on_save {
                            let _ = self.lsp_format();
                            // re-save after format
                            if let Ok(doc) = self.store.active_mut() {
                                let _ = doc.save_to(None);
                            }
                        }
                        self.set_status("saved");
                        self.refresh_git_and_index();
                        self.fire_plugin_hook(PluginHook::OnSave);
                    }
                    Err(DocumentError::Io(e)) if e.kind() == std::io::ErrorKind::InvalidInput => {
                        self.open_save_as_browser();
                    }
                    Err(e) => return Err(e),
                }
            }
            Action::SaveAs => {
                self.open_save_as_browser();
            }
            Action::SaveAll => {
                let (n, skip) = self.store.save_all();
                self.refresh_git_and_index();
                self.set_status(format!("save all: {n} ok · {skip} sem path"));
            }
            Action::Help => {
                self.overlay = Overlay::Help {
                    query: String::new(),
                    selected: 0,
                };
                self.set_status(format!(
                    "atalhos: {} binds · digite filtra · ↑↓ · Esc",
                    self.keymap.len()
                ));
            }
            Action::Find => {
                self.find.show_replace = false;
                self.find.focus_replace = false;
                self.overlay = Overlay::Find;
                self.set_status(self.find.status());
            }
            Action::ProjectFind => {
                self.overlay = Overlay::ProjectFind {
                    query: String::new(),
                    selected: 0,
                    case_sensitive: false,
                    use_regex: false,
                    hits: Vec::new(),
                    status: "project find · digite · Alt+C case · Alt+R regex · Enter abre".into(),
                };
            }
            Action::FindNext => {
                self.jump_find(true);
            }
            Action::FindPrev => {
                self.jump_find(false);
            }
            Action::Replace => {
                self.find.show_replace = true;
                self.find.focus_replace = true;
                self.overlay = Overlay::Find;
                self.set_status(self.find.status());
            }
            Action::SelectAll => {
                self.store.active_mut()?.select_all();
                self.set_status("select all");
            }
            Action::Copy => {
                let text = self
                    .store
                    .active()
                    .map(|d| {
                        let s = d.selected_text();
                        if s.is_empty() {
                            d.caret()
                                .ok()
                                .and_then(|c| d.buffer().line_text(c.line).ok())
                                .unwrap_or_default()
                        } else {
                            s
                        }
                    })
                    .unwrap_or_default();
                // Sempre grava no buffer interno; arboard é best-effort.
                let _ = clipboard::copy_text(&text);
                self.set_status(format!("copied {} bytes", text.len()));
            }
            Action::Paste => {
                let text = clipboard::paste_text();
                if text.is_empty() {
                    self.set_status("clipboard vazio (Ctrl+C copia · buffer interno se sem X11)");
                } else {
                    self.store.active_mut()?.insert_text(&text)?;
                    self.set_status(format!("pasted {} bytes", text.len()));
                }
            }
            Action::Cut => {
                let text = {
                    let doc = self.store.active()?;
                    let s = doc.selected_text();
                    if s.is_empty() {
                        // Como VS Code: corta a linha atual se não há seleção
                        if let Ok(c) = doc.caret() {
                            doc.buffer().line_text(c.line).unwrap_or_default()
                        } else {
                            String::new()
                        }
                    } else {
                        s
                    }
                };
                if text.is_empty() {
                    self.set_status("nada para cortar");
                    return Ok(());
                }
                let had_sel = !self.store.active()?.selection().is_empty();
                let _ = clipboard::copy_text(&text);
                if had_sel {
                    self.store.active_mut()?.delete_selection()?;
                } else {
                    // corta linha inteira + \n
                    let doc = self.store.active_mut()?;
                    let caret = doc.caret()?;
                    let start = doc.buffer().line_to_byte(caret.line)?;
                    let line = doc.buffer().line_text(caret.line)?;
                    let mut end = oride_core::ByteOffset::new(start.as_usize() + line.len());
                    // inclui newline se existir
                    if end.as_usize() < doc.buffer().len_bytes() {
                        end = oride_core::ByteOffset::new(end.as_usize() + 1);
                    }
                    doc.select_byte_range(start, end);
                    doc.delete_selection()?;
                }
                self.set_status(format!("cut {} bytes", text.len()));
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
            Action::MoveDocStart { extend } => {
                self.store.active_mut()?.move_buffer_start(extend)?;
            }
            Action::MoveDocEnd { extend } => {
                self.store.active_mut()?.move_buffer_end(extend)?;
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
                self.ensure_terminal();
                if let Some(term) = self.terminal.as_mut() {
                    term.toggle_visible();
                    if term.visible {
                        self.focus = Focus::Terminal;
                        self.set_status(
                            "terminal: digite · Esc=editor · Ctrl+C no shell · Ctrl+` fecha",
                        );
                    } else if self.focus == Focus::Terminal {
                        self.focus = Focus::Editor;
                        self.set_status("terminal oculto");
                    }
                } else {
                    self.set_status("terminal unavailable (PTY)");
                }
            }
            Action::TerminalGrow => {
                let h = if let Some(term) = self.terminal.as_mut() {
                    term.grow(2);
                    Some(term.height_lines)
                } else {
                    None
                };
                if let Some(h) = h {
                    self.set_status(format!("terminal height {h}"));
                }
            }
            Action::TerminalShrink => {
                let h = if let Some(term) = self.terminal.as_mut() {
                    term.shrink(2);
                    Some(term.height_lines)
                } else {
                    None
                };
                if let Some(h) = h {
                    self.set_status(format!("terminal height {h}"));
                }
            }
            Action::ReloadFile => match self.store.active_mut() {
                Ok(doc) => match doc.reload_from_disk() {
                    Ok(()) => {
                        self.apply_editorconfig_for_active();
                        self.lsp_sync_active();
                        self.set_status("file reloaded");
                    }
                    Err(e) => self.set_status(format!("reload: {e}")),
                },
                Err(e) => self.set_status(format!("reload: {e}")),
            },
            Action::ToggleDiagnostics => {
                self.show_diagnostics = !self.show_diagnostics;
                if self.show_diagnostics {
                    self.overlay = Overlay::Diagnostics { selected: 0 };
                    self.set_status(format!("{} diagnostics", self.diagnostics.len()));
                } else {
                    if matches!(self.overlay, Overlay::Diagnostics { .. }) {
                        self.overlay = Overlay::None;
                    }
                }
            }
            Action::LspComplete => self.lsp_complete()?,
            Action::LspHover => self.lsp_hover()?,
            Action::LspGotoDefinition => self.lsp_goto()?,
            Action::LspFormat => self.lsp_format()?,
            Action::SplitVertical => self.split_editor(SplitOrientation::Vertical),
            Action::SplitHorizontal => self.split_editor(SplitOrientation::Horizontal),
            Action::FocusNextPane => self.focus_next_pane(),
            Action::ClosePane => {
                if self.split.close_focused() {
                    let id = self.split.focused_pane().doc_id;
                    let _ = self.store.set_active(id);
                    self.scroll_y = self.split.focused_pane().scroll_y;
                    self.set_status("pane closed");
                } else {
                    self.set_status("já em pane único");
                }
            }
            Action::AddCursorAbove => {
                self.store.active_mut()?.add_cursor_above()?;
                self.set_status(format!(
                    "cursors: {}",
                    1 + self
                        .store
                        .active()
                        .map(|d| d.extra_carets().len())
                        .unwrap_or(0)
                ));
            }
            Action::AddCursorBelow => {
                self.store.active_mut()?.add_cursor_below()?;
                self.set_status(format!(
                    "cursors: {}",
                    1 + self
                        .store
                        .active()
                        .map(|d| d.extra_carets().len())
                        .unwrap_or(0)
                ));
            }
            Action::ClearExtraCursors => {
                if let Ok(d) = self.store.active_mut() {
                    d.clear_extra_carets();
                }
                self.set_status("multi-cursor limpo");
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
                self.ensure_terminal();
                if let Some(term) = self.terminal.as_mut() {
                    term.visible = true;
                    self.focus = Focus::Terminal;
                    self.set_status("foco: terminal · digite · Esc=editor");
                } else {
                    self.set_status("terminal unavailable");
                }
            }
            Action::ToggleScm => {
                self.show_scm = !self.show_scm;
                if self.show_scm {
                    self.refresh_scm_cache();
                    self.focus = Focus::Scm;
                    self.set_status(
                        "SCM · ↑↓ · Enter abre · d diff · r refresh · Esc editor · Ctrl+Shift+G",
                    );
                } else {
                    if self.focus == Focus::Scm {
                        self.focus = Focus::Editor;
                    }
                    self.set_status("SCM oculto");
                }
            }
            Action::FocusScm => {
                self.show_scm = true;
                self.refresh_scm_cache();
                self.focus = Focus::Scm;
                self.set_status("foco: SCM");
            }
            Action::BufferPicker => {
                self.overlay = Overlay::BufferPicker {
                    query: String::new(),
                    selected: 0,
                };
            }
            Action::JumpBack => {
                self.record_jump();
                if let Some(j) = self.jump_list.back() {
                    self.goto_jump(&j);
                    self.set_status("jump back");
                } else {
                    self.set_status("jump list vazio");
                }
            }
            Action::JumpForward => {
                if let Some(j) = self.jump_list.forward() {
                    self.goto_jump(&j);
                    self.set_status("jump forward");
                } else {
                    self.set_status("sem jump forward");
                }
            }
            Action::WhichKey => {
                self.show_which_key = true;
            }
            Action::Welcome => {
                self.show_welcome = true;
            }
            Action::ShowDiff => {
                self.open_diff_for_active();
            }
            Action::OpenFolder => {
                let browser = PathBrowser::new(&self.workspace, BrowseMode::Folder);
                self.set_status(browser.hint());
                self.overlay = Overlay::Browse(browser);
            }
            Action::OpenFileFuzzy => {
                let browser = PathBrowser::new(&self.workspace, BrowseMode::File);
                self.set_status(browser.hint());
                self.overlay = Overlay::Browse(browser);
            }
            Action::ToggleSoftWrap => {
                self.soft_wrap = !self.soft_wrap;
                self.set_status(if self.soft_wrap {
                    "soft wrap: on"
                } else {
                    "soft wrap: off"
                });
            }
            Action::ToggleMdPreview => {
                let lang = self
                    .store
                    .active()
                    .ok()
                    .map(|d| detect_language(d.path()))
                    .unwrap_or_default();
                if !lang.is_markdown_family() {
                    self.set_status("preview só para Markdown/MDX");
                } else {
                    self.show_md_preview = !self.show_md_preview;
                    self.preview_scroll = 0;
                    self.set_status(if self.show_md_preview {
                        "md preview: on · segue scroll do editor · Alt+↑/↓ fine · Alt+P fecha"
                    } else {
                        "md preview: off"
                    });
                }
            }
            Action::ToggleComment => {
                self.toggle_line_comment()?;
            }
            Action::NextTab => {
                self.store.activate_next_tab();
                self.scroll_y = 0;
                if let Some(id) = self.store.active_id() {
                    self.split.set_focused_doc(id);
                }
            }
            Action::PrevTab => {
                self.store.activate_prev_tab();
                self.scroll_y = 0;
                if let Some(id) = self.store.active_id() {
                    self.split.set_focused_doc(id);
                }
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
                self.refresh_scm_cache();
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
        let provider = self.plugin_host.language(lang);
        let open = match provider.comment_open() {
            Some(o) => o,
            None => {
                self.set_status("comentário não definido para esta linguagem");
                return Ok(());
            }
        };
        let close = provider.comment_close().unwrap_or("");
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
        let provider = self.plugin_host.language(lang);
        if provider.default_soft_wrap() {
            self.soft_wrap = true;
        }
        self.fire_plugin_hook(PluginHook::OnOpen);
    }

    fn run_plugin_command(&mut self, cmd_id: &str) {
        let text = self
            .store
            .active()
            .map(|d| d.buffer().as_string())
            .unwrap_or_default();
        let path = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(Path::to_path_buf));
        let dirty = self.store.active().map(|d| d.is_dirty()).unwrap_or(false);
        let workspace = self.workspace.clone();
        let mut adapter = HostCtx {
            status: None,
            workspace,
            path,
            text,
            dirty,
        };
        match self.plugin_host.run_command(cmd_id, &mut adapter) {
            Ok(()) => {
                if let Some(s) = adapter.status {
                    self.set_status(s);
                }
            }
            Err(e) => self.set_status(format!("plugin: {e}")),
        }
    }

    fn split_editor(&mut self, orientation: SplitOrientation) {
        let id = match self.store.active_id() {
            Some(id) => id,
            None => {
                self.set_status("sem documento");
                return;
            }
        };
        // sync scroll into focused pane first
        self.split.sync_scroll(self.scroll_y);
        self.split.split(orientation, id);
        let _ = self.store.set_active(id);
        self.set_status(match orientation {
            SplitOrientation::Vertical => "split vertical · F6 / Ctrl+Alt+←→ troca pane",
            SplitOrientation::Horizontal => "split horizontal · F6 troca pane",
        });
    }

    fn focus_next_pane(&mut self) {
        if !self.split.is_split() {
            self.set_status("sem split");
            return;
        }
        self.split.sync_scroll(self.scroll_y);
        self.split.focus_next();
        let pane = self.split.focused_pane().clone();
        let _ = self.store.set_active(pane.doc_id);
        self.scroll_y = pane.scroll_y;
        self.set_status(format!(
            "pane {}/{}",
            self.split.focused + 1,
            self.split.panes.len()
        ));
    }

    fn fire_plugin_hook(&mut self, hook: PluginHook) {
        let text = self
            .store
            .active()
            .map(|d| d.buffer().as_string())
            .unwrap_or_default();
        let path = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(Path::to_path_buf));
        let dirty = self.store.active().map(|d| d.is_dirty()).unwrap_or(false);
        let workspace = self.workspace.clone();
        let mut adapter = HostCtx {
            status: None,
            workspace,
            path,
            text,
            dirty,
        };
        self.plugin_host.dispatch_hook(hook, &mut adapter);
        if let Some(s) = adapter.status {
            self.set_status(s);
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

    fn refresh_scm_cache(&mut self) {
        self.scm_cache = scm_entries(&self.workspace);
        if self.scm_selected >= self.scm_cache.len() && !self.scm_cache.is_empty() {
            self.scm_selected = self.scm_cache.len() - 1;
        }
        if self.scm_cache.is_empty() {
            self.scm_selected = 0;
        }
    }

    fn ensure_terminal(&mut self) {
        let dead = self
            .terminal
            .as_ref()
            .and_then(|t| t.last_error.as_deref())
            .map(|e| e.contains("encerrado"))
            .unwrap_or(false);
        if self.terminal.is_none() || dead {
            let h = self
                .terminal
                .as_ref()
                .map(|t| t.height_lines)
                .unwrap_or(self.config.terminal.default_height)
                .max(3);
            match EmbeddedTerminal::spawn(&self.workspace, 80, h) {
                Ok(mut t) => {
                    t.height_lines = h;
                    t.visible = true;
                    self.terminal = Some(t);
                }
                Err(e) => self.set_status(format!("pty: {e}")),
            }
        }
    }

    fn record_jump(&mut self) {
        let Ok(doc) = self.store.active() else {
            return;
        };
        let path = doc.path().map(Path::to_path_buf);
        let byte = doc.selection().head;
        let line = doc.caret().map(|c| c.line).unwrap_or(0);
        self.jump_list.push(Jump { path, byte, line });
    }

    fn goto_jump(&mut self, jump: &Jump) {
        if let Some(path) = &jump.path {
            let _ = self.store.open_path(path);
            self.apply_editorconfig_for_active();
            self.lsp_open_active();
        }
        if let Ok(doc) = self.store.active_mut() {
            doc.jump_to_byte(jump.byte);
        }
        if let Some(id) = self.store.active_id() {
            self.split.set_focused_doc(id);
        }
        self.scroll_y = jump.line.saturating_sub(2);
        self.focus = Focus::Editor;
        self.ensure_cursor_visible();
    }

    fn open_diff_for_active(&mut self) {
        let path = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(Path::to_path_buf));
        let Some(path) = path else {
            self.set_status("diff: sem path");
            return;
        };
        self.open_diff_for_path(&path);
    }

    fn open_diff_for_path(&mut self, path: &Path) {
        match diff_file(&self.workspace, path) {
            Some(text) => {
                let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
                self.overlay = Overlay::Diff {
                    path: path.to_path_buf(),
                    lines,
                    scroll: 0,
                };
            }
            None => self.set_status(format!("diff vazio: {}", path.display())),
        }
    }

    fn apply_action_id(&mut self, id: &str) {
        if let Some(cmd) = id.strip_prefix("plugin:") {
            self.run_plugin_command(cmd);
            return;
        }
        match parse_action(id) {
            Ok(action) => {
                let _ = self.apply_action(action);
            }
            Err(_) => self.set_status(format!("ação desconhecida: {id}")),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        let Some((mi, ii)) = self.menu_open else {
            return;
        };
        let n_menus = self.menus.len();
        let n_items = self.menus.get(mi).map(|m| m.items.len()).unwrap_or(0);
        match key.code {
            KeyCode::Esc => self.menu_open = None,
            KeyCode::Left => {
                let mi = if mi == 0 {
                    n_menus.saturating_sub(1)
                } else {
                    mi - 1
                };
                self.menu_open = Some((mi, 0));
            }
            KeyCode::Right => {
                let mi = if n_menus == 0 { 0 } else { (mi + 1) % n_menus };
                self.menu_open = Some((mi, 0));
            }
            KeyCode::Up => {
                let ii = ii.saturating_sub(1);
                self.menu_open = Some((mi, ii));
            }
            KeyCode::Down => {
                let ii = if n_items == 0 {
                    0
                } else {
                    (ii + 1).min(n_items - 1)
                };
                self.menu_open = Some((mi, ii));
            }
            KeyCode::Enter => {
                let action_id = self
                    .menus
                    .get(mi)
                    .and_then(|m| m.items.get(ii))
                    .map(|it| it.action_id.clone());
                self.menu_open = None;
                if let Some(id) = action_id {
                    self.apply_action_id(&id);
                }
            }
            KeyCode::Char(c) => {
                // first letter of item
                if let Some(menu) = self.menus.get(mi) {
                    let c = c.to_ascii_lowercase();
                    if let Some(idx) = menu
                        .items
                        .iter()
                        .position(|it| it.label.to_ascii_lowercase().starts_with(c))
                    {
                        self.menu_open = Some((mi, idx));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_welcome_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char(' ') => {
                self.show_welcome = false;
            }
            _ => {
                // qualquer tecla fecha e reprocessa? só fecha.
                self.show_welcome = false;
            }
        }
    }

    fn handle_scm_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.focus = Focus::Editor;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scm_selected = self.scm_selected.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.scm_cache.is_empty() {
                    self.scm_selected = (self.scm_selected + 1).min(self.scm_cache.len() - 1);
                }
                true
            }
            KeyCode::Char('r') => {
                self.refresh_scm_cache();
                self.refresh_git_and_index();
                self.set_status("SCM refreshed");
                true
            }
            KeyCode::Char('d') => {
                if let Some((_, p)) = self.scm_cache.get(self.scm_selected) {
                    let path = self.workspace.join(p);
                    self.open_diff_for_path(&path);
                }
                true
            }
            KeyCode::Enter => {
                if let Some((_, p)) = self.scm_cache.get(self.scm_selected).cloned() {
                    let path = self.workspace.join(p);
                    self.record_jump();
                    match self.store.open_path(&path) {
                        Ok(_) => {
                            self.apply_editorconfig_for_active();
                            self.lsp_open_active();
                            self.focus = Focus::Editor;
                            self.scroll_y = 0;
                            if let Some(id) = self.store.active_id() {
                                self.split.set_focused_doc(id);
                            }
                            self.set_status(format!("opened {}", path.display()));
                        }
                        Err(e) => self.set_status(format!("open: {e}")),
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_buffer_picker_key(&mut self, key: KeyEvent) {
        let Overlay::BufferPicker { query, selected } = &self.overlay else {
            return;
        };
        let mut query = query.clone();
        let mut selected = *selected;
        let items = self.buffer_picker_items(&query);
        match key.code {
            KeyCode::Esc => self.overlay = Overlay::None,
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down if !items.is_empty() => {
                selected = (selected + 1).min(items.len() - 1);
            }
            KeyCode::Backspace => {
                query.pop();
                selected = 0;
            }
            KeyCode::Char(c) if !c.is_control() => {
                query.push(c);
                selected = 0;
            }
            KeyCode::Enter => {
                if let Some((id, _)) = items.get(selected).cloned() {
                    self.record_jump();
                    let _ = self.store.set_active(id);
                    self.split.set_focused_doc(id);
                    self.focus = Focus::Editor;
                    self.scroll_y = 0;
                    self.overlay = Overlay::None;
                    return;
                }
            }
            _ => {}
        }
        if !matches!(self.overlay, Overlay::None) {
            self.overlay = Overlay::BufferPicker { query, selected };
        }
    }

    fn buffer_picker_items(&self, query: &str) -> Vec<(DocumentId, String)> {
        let q = query.to_lowercase();
        let mut out = Vec::new();
        for tab in self.store.tab_summaries() {
            let path = self
                .store
                .get(tab.id)
                .and_then(|d| d.path().map(Path::to_path_buf));
            let label = if let Some(p) = path {
                let rel = p.strip_prefix(&self.workspace).unwrap_or(p.as_path());
                format!("{}{}", if tab.dirty { "● " } else { "  " }, rel.display())
            } else {
                format!("{}{}", if tab.dirty { "● " } else { "  " }, tab.title)
            };
            if q.is_empty() || label.to_lowercase().contains(&q) {
                out.push((tab.id, label));
            }
        }
        out
    }

    fn handle_diff_key(&mut self, key: KeyEvent) {
        let Overlay::Diff {
            path,
            lines,
            scroll,
        } = &self.overlay
        else {
            return;
        };
        let path = path.clone();
        let lines = lines.clone();
        let mut scroll = *scroll;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                scroll = (scroll + 1).min(lines.len().saturating_sub(1));
            }
            KeyCode::PageUp => scroll = scroll.saturating_sub(10),
            KeyCode::PageDown => {
                scroll = (scroll + 10).min(lines.len().saturating_sub(1));
            }
            KeyCode::Home => scroll = 0,
            KeyCode::End => scroll = lines.len().saturating_sub(1),
            _ => {}
        }
        self.overlay = Overlay::Diff {
            path,
            lines,
            scroll,
        };
    }

    fn focus_label(&self) -> &'static str {
        match self.focus {
            Focus::Editor => "FOCUS: EDITOR",
            Focus::Tree => "FOCUS: TREE",
            Focus::Terminal => "FOCUS: TERMINAL",
            Focus::Scm => "FOCUS: SCM",
        }
    }

    fn which_key_rows(&self) -> Vec<(String, String)> {
        vec![
            ("F1".into(), "todos os atalhos".into()),
            ("Ctrl+Shift+P".into(), "command palette".into()),
            ("Ctrl+P".into(), "abrir arquivo".into()),
            ("Ctrl+S".into(), "salvar".into()),
            ("Ctrl+F".into(), "find".into()),
            ("Ctrl+Shift+F".into(), "find no projeto".into()),
            ("Ctrl+`".into(), "terminal".into()),
            ("Ctrl+Shift+G".into(), "SCM panel".into()),
            ("Ctrl+Shift+O".into(), "buffer picker".into()),
            ("Ctrl+B".into(), "foco árvore".into()),
            ("Ctrl+E".into(), "foco editor".into()),
            ("Alt+F/E/V/G/I/H".into(), "menus".into()),
            ("Alt+/".into(), "este which-key".into()),
        ]
    }

    fn welcome_lines(&self) -> Vec<String> {
        vec![
            "Bem-vindo ao Oride".into(),
            "".into(),
            "  F1              todos os atalhos".into(),
            "  Ctrl+Shift+P    command palette".into(),
            "  Ctrl+P          abrir arquivo".into(),
            "  Ctrl+S          salvar".into(),
            "  Ctrl+\"          terminal interativo".into(),
            "  Ctrl+Shift+G    painel Git (SCM)".into(),
            "  Alt+F           menu File".into(),
            "".into(),
            "  Enter / Esc / Space — começar".into(),
        ]
    }

    fn current_blame(&self) -> Option<String> {
        let doc = self.store.active().ok()?;
        let path = doc.path()?;
        let line = doc.caret().ok()?.line + 1;
        blame_line(&self.workspace, path, line)
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
        let text_width = self.last_editor_text_width.max(1);
        let soft = self.soft_wrap;

        // Acima do viewport → sobe
        if caret.line < self.scroll_y {
            self.scroll_y = caret.line;
            self.split.sync_scroll(self.scroll_y);
            return;
        }

        // Linha visual do caret relativa ao topo (scroll_y)
        let caret_visual = visual_row_of_caret(
            doc.buffer(),
            self.scroll_y,
            caret.line,
            caret.column,
            text_width,
            soft,
        );

        if caret_visual < height {
            return; // ainda visível
        }

        // Coloca o caret na última linha visual do viewport
        self.scroll_y = scroll_origin_for_caret(
            doc.buffer(),
            caret.line,
            caret.column,
            height,
            text_width,
            soft,
        );
        self.split.sync_scroll(self.scroll_y);
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        self.tick();
        let area = frame.area();

        let term_h = self
            .terminal
            .as_ref()
            .filter(|t| t.visible)
            .map(|t| t.height_lines.min(area.height.saturating_sub(5)).max(3))
            .unwrap_or(0);

        // menu(1) + banner(1) + body + term + status(1)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // menu bar
                Constraint::Length(1), // context banner
                Constraint::Min(1),
                Constraint::Length(term_h),
                Constraint::Length(1), // status
            ])
            .split(area);

        let menu_area = main_chunks[0];
        let banner_area = main_chunks[1];
        let body = main_chunks[2];
        let term_area = main_chunks[3];
        let status_area = main_chunks[4];

        let open_menu = self.menu_open.map(|(i, _)| i);
        render_menu_bar(frame, menu_area, &self.menus, open_menu);

        let banner_hint = match self.focus {
            Focus::Editor => "F1 help · Ctrl+Shift+P cmds · Alt+F menu",
            Focus::Tree => "↑↓ Enter · Ctrl+E editor",
            Focus::Terminal => "digite · Esc=editor · Ctrl+C shell",
            Focus::Scm => "↑↓ Enter · d=diff · r=refresh · Esc",
        };
        render_context_banner(frame, banner_area, self.focus_label(), banner_hint);

        let tree_w = if self.show_tree {
            self.tree_width.min(body.width / 2).max(12)
        } else {
            0
        };
        let scm_w = if self.show_scm {
            self.scm_width.min(body.width / 3).max(16)
        } else {
            0
        };

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(tree_w),
                Constraint::Min(1),
                Constraint::Length(scm_w),
            ])
            .split(body);

        if tree_w > 0 {
            self.draw_tree(frame, body_chunks[0]);
        }
        self.draw_editor_column(frame, body_chunks[1]);
        if scm_w > 0 {
            self.draw_scm(frame, body_chunks[2]);
        }

        if term_h > 0 {
            if let Some(term) = self.terminal.as_mut() {
                term.poll_output();
                let cols = term_area.width.max(20);
                let rows = term_area.height.max(2);
                term.resize(cols, rows);
                let lines = term.visible_lines(rows as usize);
                let err = term.last_error.clone();
                render_terminal_panel(
                    frame,
                    term_area,
                    &lines,
                    self.focus == Focus::Terminal,
                    &self.theme,
                    err.as_deref(),
                );
            }
        }

        self.draw_status(frame, status_area);

        // dropdown menu por cima
        if let Some((mi, ii)) = self.menu_open {
            render_menu_dropdown(frame, area, &self.menus, mi, ii);
        }

        // overlays
        match &self.overlay {
            Overlay::None => {}
            Overlay::CommandPalette { query, selected } => {
                let items = self.command_palette_items(query);
                let view = PaletteView {
                    title: "commands",
                    query,
                    items: &items,
                    selected: *selected,
                    hint: "↑↓ · Enter executa · Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Browse(browser) => {
                let items = browser.list_labels();
                let q = browser.query_display();
                let view = PaletteView {
                    title: &browser.title(),
                    query: &q,
                    items: &items,
                    selected: browser.selected_index_for_display(),
                    hint: browser.hint(),
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Prompt { kind, buffer } => {
                let title = match kind {
                    PromptKind::NewFile => "novo arquivo",
                    PromptKind::NewDir => "nova pasta",
                };
                let items: &[String] = &[];
                let view = PaletteView {
                    title,
                    query: buffer,
                    items,
                    selected: 0,
                    hint: "Enter confirma · Esc cancela",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Help { query, selected } => {
                let items = self.keybind_list_items(query);
                let title = format!(
                    "atalhos ({}/{}) — F1 · Ctrl+G · Ctrl+Shift+/",
                    items.len(),
                    self.keymap.len()
                );
                let view = PaletteView {
                    title: &title,
                    query,
                    items: &items,
                    selected: *selected,
                    hint: "↑↓ navegar · digite filtra · Enter/Esc/q fecha",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Find => {
                let match_label = self.find.match_label();
                let view = FindModalView {
                    query: &self.find.query,
                    replace: &self.find.replace,
                    show_replace: self.find.show_replace,
                    focus_replace: self.find.focus_replace,
                    match_label: &match_label,
                    case_sensitive: self.find.case_sensitive,
                    ignore_accents: self.find.ignore_accents,
                    whole_word: self.find.whole_word,
                    use_regex: self.find.use_regex,
                    error: self.find.regex_error.as_deref(),
                };
                render_find_modal(frame, area, &view);
            }
            Overlay::ProjectFind {
                query,
                selected,
                hits,
                status,
                ..
            } => {
                let items: Vec<String> = hits
                    .iter()
                    .map(|h| format_hit_label(h, &self.workspace))
                    .collect();
                let title = format!("find in project ({})", status);
                let view = PaletteView {
                    title: &title,
                    query,
                    items: &items,
                    selected: *selected,
                    hint: "↑↓ · Enter abre · Alt+C case · Alt+R regex · Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Diagnostics { selected } => {
                let items: Vec<String> = self
                    .diagnostics
                    .iter()
                    .map(|(path, d)| {
                        format!(
                            "L{}:{}  {}  {}",
                            d.range.start.line + 1,
                            d.range.start.character + 1,
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                            d.message
                        )
                    })
                    .collect();
                let view = PaletteView {
                    title: "diagnostics (Enter jump · Esc)",
                    query: "",
                    items: &items,
                    selected: *selected,
                    hint: "↑↓ · Enter salta · Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Completion { items, selected } => {
                let view = PaletteView {
                    title: "completions",
                    query: "",
                    items,
                    selected: *selected,
                    hint: "↑↓ · Enter insere · Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Hover { text } => {
                let items: Vec<String> = text.lines().map(|l| l.to_string()).collect();
                let view = PaletteView {
                    title: "hover",
                    query: "",
                    items: &items,
                    selected: 0,
                    hint: "Esc fecha",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::ReloadConfirm { path } => {
                let items = [
                    format!("arquivo: {}", path.display()),
                    "Enter = recarregar do disco (descarta edições)".into(),
                    "Esc = manter buffer".into(),
                ];
                let view = PaletteView {
                    title: "arquivo mudou no disco",
                    query: "",
                    items: &items,
                    selected: 0,
                    hint: "Enter / Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::BufferPicker { query, selected } => {
                let items: Vec<String> = self
                    .buffer_picker_items(query)
                    .into_iter()
                    .map(|(_, l)| l)
                    .collect();
                let view = PaletteView {
                    title: "buffers",
                    query,
                    items: &items,
                    selected: *selected,
                    hint: "↑↓ · Enter abre · digite filtra · Esc",
                };
                render_palette(frame, area, &view, &self.theme);
            }
            Overlay::Diff {
                path,
                lines,
                scroll,
            } => {
                let start = *scroll;
                let end = (start + 40).min(lines.len());
                let slice: Vec<String> = lines[start..end].to_vec();
                let title = format!("diff · {} · j/k scroll · Esc", path.display());
                let view = PaletteView {
                    title: &title,
                    query: "",
                    items: &slice,
                    selected: 0,
                    hint: "↑↓/jk · PgUp/PgDn · Esc fecha",
                };
                render_palette(frame, area, &view, &self.theme);
            }
        }

        if self.show_which_key {
            let rows = self.which_key_rows();
            render_which_key(frame, area, "which-key", &rows);
        }
        if self.show_welcome {
            let lines = self.welcome_lines();
            let view = MiniModalView {
                title: "Oride · essentials",
                lines: &lines,
                selected: 0,
            };
            render_mini_modal(frame, area, &view, &self.theme);
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

        let (lang, source) = match self.store.active() {
            Ok(d) => (detect_language(d.path()), d.buffer().as_string()),
            Err(_) => return,
        };
        self.highlight.update(lang, &source);

        let show_preview = self.show_md_preview && lang.is_markdown_family();
        let body = chunks[1];

        // Área do editor (pode ser split em 2 panes) + preview MD opcional
        let (editor_zone, preview_area) = if show_preview {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(body);
            (split[0], Some(split[1]))
        } else {
            (body, None)
        };

        self.split.sync_scroll(self.scroll_y);
        let pane_areas: Vec<Rect> = if self.split.is_split() {
            let orient = match self.split.orientation {
                SplitOrientation::Vertical => Direction::Horizontal,
                SplitOrientation::Horizontal => Direction::Vertical,
            };
            Layout::default()
                .direction(orient)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(editor_zone)
                .to_vec()
        } else {
            vec![editor_zone]
        };

        // Viewport real do texto (desconta borda do split + gutter)
        if let Some(a) = pane_areas.get(self.split.focused) {
            let border: u16 = if self.split.is_split() { 2 } else { 0 };
            let gutter = if self.show_line_numbers {
                self.theme.gutter_width.min(a.width.saturating_sub(border))
            } else {
                0
            };
            self.last_editor_height = a.height.saturating_sub(border).max(1) as usize;
            self.last_editor_text_width =
                a.width.saturating_sub(border).saturating_sub(gutter).max(1) as usize;
        }
        self.ensure_cursor_visible();
        self.split.sync_scroll(self.scroll_y);

        for (i, pane_area) in pane_areas.iter().enumerate() {
            let pane = &self.split.panes[i.min(self.split.panes.len() - 1)];
            let Some(doc) = self.store.get(pane.doc_id) else {
                continue;
            };
            let caret = doc.caret().unwrap_or_default();
            let selection = doc.selection();
            let extra: Vec<_> = doc
                .extra_carets()
                .iter()
                .filter_map(|b| doc.buffer().byte_to_caret(*b).ok())
                .collect();
            // highlight por documento: recompute se não for o ativo
            let lang_p = detect_language(doc.path());
            let source_p = doc.buffer().as_string();
            // use engine only for focused to avoid thrash; others re-highlight cheaply
            let highlights = if i == self.split.focused {
                self.highlight.update(lang_p, &source_p);
                self.highlight.spans()
            } else {
                // empty highlights for secondary to keep simple
                &[]
            };
            let view = EditorView {
                buffer: doc.buffer(),
                caret,
                selection,
                extra_carets: &extra,
                scroll_y: pane.scroll_y,
                show_line_numbers: self.show_line_numbers,
                highlights,
                show_cursor: self.focus == Focus::Editor
                    && matches!(self.overlay, Overlay::None)
                    && i == self.split.focused,
                soft_wrap: self.soft_wrap,
                focused_pane: self.split.is_split() && i == self.split.focused,
            };
            render_editor(frame, *pane_area, &view, &self.theme);
        }

        if let Some(prev_area) = preview_area {
            let lines = render_preview_lines(&source);
            // Preview segue o scroll do editor (1 linha fonte ≈ 1 linha preview).
            // preview_scroll = offset fino (Alt+↑/↓) em cima do scroll_y.
            let max_scroll = lines.len().saturating_sub(1);
            let base = self.scroll_y.min(max_scroll);
            let scroll = base.saturating_add(self.preview_scroll).min(max_scroll);
            let caret_line = self
                .store
                .active()
                .ok()
                .and_then(|d| d.caret().ok())
                .map(|c| c.line + 1)
                .unwrap_or(0);
            let title = format!("preview · editor L{caret_line} · Alt+↑/↓ fine · Alt+P fecha");
            let view = MdPreviewView {
                title: &title,
                lines: &lines,
                scroll,
            };
            render_md_preview(frame, prev_area, &view, &self.theme);
        }
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let doc = self.store.active().ok();
        let caret = doc.and_then(|d| d.caret().ok()).unwrap_or_default();
        let title = doc.map(|d| d.tab_title()).unwrap_or_else(|| "oride".into());
        let dirty = doc.map(|d| d.is_dirty()).unwrap_or(false);
        let blame = self.current_blame();

        let status = StatusModel {
            title,
            dirty,
            line: caret.line,
            column: caret.column,
            git_branch: self.git_branch.clone(),
            blame,
            message: self.status_message.clone(),
            help_hint: "F1 help · Ctrl+Shift+P cmds".into(),
        };
        render_status(frame, area, &status, &self.theme);
    }

    fn draw_scm(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ScmItem> = self
            .scm_cache
            .iter()
            .map(|(st, p)| ScmItem {
                badge: st.badge(),
                path: p.display().to_string(),
            })
            .collect();
        render_scm_panel(
            frame,
            area,
            "SCM",
            &items,
            self.scm_selected,
            self.focus == Focus::Scm,
        );
    }

    fn handle_diagnostics_key(&mut self, key: KeyEvent) {
        let Overlay::Diagnostics { selected } = &self.overlay else {
            return;
        };
        let mut selected = *selected;
        let n = self.diagnostics.len();
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.overlay = Overlay::None,
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down if n > 0 => selected = (selected + 1).min(n - 1),
            KeyCode::Enter if n > 0 => {
                if let Some((path, diag)) = self.diagnostics.get(selected).cloned() {
                    let _ = self.store.open_path(&path);
                    if let Ok(doc) = self.store.active_mut() {
                        let line = diag.range.start.line as usize;
                        let col = diag.range.start.character as usize;
                        if let Ok(off) = doc
                            .buffer()
                            .caret_to_byte(oride_core::Caret::new(line, col))
                        {
                            doc.jump_to_byte(off);
                        }
                    }
                    self.scroll_y = 0;
                    self.ensure_cursor_visible();
                    self.overlay = Overlay::None;
                    self.focus = Focus::Editor;
                }
            }
            _ => {}
        }
        if matches!(self.overlay, Overlay::Diagnostics { .. }) {
            self.overlay = Overlay::Diagnostics { selected };
        }
    }

    fn handle_completion_key(&mut self, key: KeyEvent) {
        let Overlay::Completion { items, selected } = &self.overlay else {
            return;
        };
        let mut selected = *selected;
        let items = items.clone();
        let n = items.len();
        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down if n > 0 => selected = (selected + 1).min(n - 1),
            KeyCode::Enter if n > 0 => {
                if let Some(item) = items.get(selected) {
                    let insert = item.split(" — ").next().unwrap_or(item).to_string();
                    let _ = self.store.active_mut().map(|d| d.insert_text(&insert));
                    self.lsp_sync_active();
                }
                self.overlay = Overlay::None;
                return;
            }
            _ => {}
        }
        self.overlay = Overlay::Completion { items, selected };
    }

    fn lsp_complete(&mut self) -> Result<(), DocumentError> {
        let (path, pos) = self.active_lsp_pos()?;
        let Some(lsp) = self.lsp.as_mut() else {
            self.set_status("LSP offline (oriscript lsp?)");
            return Ok(());
        };
        match lsp.completion(&path, pos) {
            Ok(items) if items.is_empty() => self.set_status("no completions"),
            Ok(items) => {
                let labels: Vec<String> = items
                    .into_iter()
                    .map(|i| match i.detail {
                        Some(d) => format!("{} — {}", i.label, d),
                        None => i.label,
                    })
                    .collect();
                self.overlay = Overlay::Completion {
                    items: labels,
                    selected: 0,
                };
            }
            Err(e) => self.set_status(format!("complete: {e}")),
        }
        Ok(())
    }

    fn lsp_hover(&mut self) -> Result<(), DocumentError> {
        let (path, pos) = self.active_lsp_pos()?;
        let Some(lsp) = self.lsp.as_mut() else {
            self.set_status("LSP offline");
            return Ok(());
        };
        match lsp.hover(&path, pos) {
            Ok(Some(h)) => {
                self.overlay = Overlay::Hover { text: h.contents };
            }
            Ok(None) => self.set_status("no hover"),
            Err(e) => self.set_status(format!("hover: {e}")),
        }
        Ok(())
    }

    fn lsp_goto(&mut self) -> Result<(), DocumentError> {
        let (path, pos) = self.active_lsp_pos()?;
        let Some(lsp) = self.lsp.as_mut() else {
            self.set_status("LSP offline");
            return Ok(());
        };
        match lsp.definition(&path, pos) {
            Ok(Some(loc)) => {
                if let Some(p) = uri_to_path(&loc.uri) {
                    self.record_jump();
                    let _ = self.store.open_path(&p);
                    if let Ok(doc) = self.store.active_mut() {
                        let line = loc.range.start.line as usize;
                        let col = loc.range.start.character as usize;
                        if let Ok(off) = doc
                            .buffer()
                            .caret_to_byte(oride_core::Caret::new(line, col))
                        {
                            doc.jump_to_byte(off);
                        }
                    }
                    self.apply_editorconfig_for_active();
                    self.lsp_open_active();
                    self.focus = Focus::Editor;
                    self.ensure_cursor_visible();
                    self.set_status(format!("goto {}", p.display()));
                }
            }
            Ok(None) => self.set_status("no definition"),
            Err(e) => self.set_status(format!("goto: {e}")),
        }
        Ok(())
    }

    fn lsp_format(&mut self) -> Result<(), DocumentError> {
        let path = self
            .store
            .active()
            .ok()
            .and_then(|d| d.path().map(Path::to_path_buf));
        let Some(path) = path else {
            self.set_status("format: sem path");
            return Ok(());
        };
        let Some(lsp) = self.lsp.as_mut() else {
            // fallback: oriscript fmt via std process se existir
            self.set_status("LSP offline — format indisponível");
            return Ok(());
        };
        match lsp.formatting(&path) {
            Ok(Some(text)) => {
                self.store.active_mut()?.replace_full_text(&text)?;
                self.lsp_sync_active();
                self.set_status("formatted");
            }
            Ok(None) => self.set_status("format: sem edits"),
            Err(e) => self.set_status(format!("format: {e}")),
        }
        Ok(())
    }

    fn active_lsp_pos(&self) -> Result<(PathBuf, LspPos), DocumentError> {
        let doc = self.store.active()?;
        let path = doc.path().map(Path::to_path_buf).ok_or_else(|| {
            DocumentError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no path",
            ))
        })?;
        let caret = doc.caret()?;
        Ok((
            path,
            LspPos {
                line: caret.line as u32,
                character: caret.column as u32,
            },
        ))
    }

    fn handle_project_find_key(&mut self, key: KeyEvent) {
        let Overlay::ProjectFind {
            query,
            selected,
            case_sensitive,
            use_regex,
            hits,
            status,
        } = &self.overlay
        else {
            return;
        };
        let mut query = query.clone();
        let mut selected = *selected;
        let mut case_sensitive = *case_sensitive;
        let mut use_regex = *use_regex;
        let mut hits = hits.clone();
        let mut status = status.clone();
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Enter => {
                if let Some(hit) = hits.get(selected).cloned() {
                    self.jump_to_project_hit(hit);
                }
                return;
            }
            KeyCode::Up => selected = selected.saturating_sub(1),
            KeyCode::Down if !hits.is_empty() => {
                selected = (selected + 1).min(hits.len() - 1);
            }
            KeyCode::Char('c') if alt && !ctrl => {
                case_sensitive = !case_sensitive;
                self.recompute_project_find(
                    &query,
                    &mut hits,
                    &mut status,
                    case_sensitive,
                    use_regex,
                );
                selected = 0;
            }
            KeyCode::Char('r') if alt && !ctrl => {
                use_regex = !use_regex;
                self.recompute_project_find(
                    &query,
                    &mut hits,
                    &mut status,
                    case_sensitive,
                    use_regex,
                );
                selected = 0;
            }
            KeyCode::Backspace => {
                query.pop();
                self.recompute_project_find(
                    &query,
                    &mut hits,
                    &mut status,
                    case_sensitive,
                    use_regex,
                );
                selected = 0;
            }
            KeyCode::Char(c) if !ctrl && !alt && !c.is_control() => {
                query.push(c);
                self.recompute_project_find(
                    &query,
                    &mut hits,
                    &mut status,
                    case_sensitive,
                    use_regex,
                );
                selected = 0;
            }
            _ => {}
        }

        if !hits.is_empty() {
            selected = selected.min(hits.len() - 1);
        } else {
            selected = 0;
        }
        self.overlay = Overlay::ProjectFind {
            query,
            selected,
            case_sensitive,
            use_regex,
            hits,
            status,
        };
    }

    fn recompute_project_find(
        &self,
        query: &str,
        hits: &mut Vec<SearchHit>,
        status: &mut String,
        case_sensitive: bool,
        use_regex: bool,
    ) {
        if query.trim().is_empty() {
            hits.clear();
            *status = "project find · digite · Alt+C case · Alt+R regex".into();
            return;
        }
        let q = SearchQuery {
            pattern: query.to_string(),
            case_sensitive,
            use_regex,
            max_hits: 500,
        };
        match search_project(&self.workspace, &q) {
            Ok(r) => {
                *hits = r.hits;
                let backend = match r.backend {
                    oride_search::SearchBackend::Ripgrep => "rg",
                    oride_search::SearchBackend::RustWalk => "rust",
                };
                let trunc = if r.truncated { " · truncated" } else { "" };
                *status = format!("{} hits · {backend}{trunc}", hits.len());
            }
            Err(e) => {
                hits.clear();
                *status = format!("erro: {e}");
            }
        }
    }

    fn jump_to_project_hit(&mut self, hit: SearchHit) {
        let path = if hit.path.is_absolute() {
            hit.path.clone()
        } else {
            self.workspace.join(&hit.path)
        };
        if let Err(e) = self.store.open_path(&path) {
            self.set_status(format!("open: {e}"));
            return;
        }
        let lang = detect_language(Some(path.as_path()));
        self.apply_language_defaults(lang);
        self.apply_editorconfig_for_active();
        self.lsp_open_active();
        if let Ok(doc) = self.store.active_mut() {
            let line = hit.line.saturating_sub(1);
            let col = hit.column.saturating_sub(1);
            if let Ok(off) = doc
                .buffer()
                .caret_to_byte(oride_core::Caret::new(line, col))
            {
                doc.jump_to_byte(off);
            }
        }
        self.scroll_y = hit.line.saturating_sub(1);
        self.ensure_cursor_visible();
        self.focus = Focus::Editor;
        self.overlay = Overlay::None;
        self.set_status(format!(
            "{}:{}  {}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            hit.line,
            hit.line_text.chars().take(40).collect::<String>()
        ));
    }
}

struct HostCtx {
    status: Option<String>,
    workspace: PathBuf,
    path: Option<PathBuf>,
    text: String,
    dirty: bool,
}

impl PluginCtx for HostCtx {
    fn set_status(&mut self, msg: &str) {
        self.status = Some(msg.to_string());
    }
    fn workspace_root(&self) -> &Path {
        &self.workspace
    }
    fn active_path(&self) -> Option<PathBuf> {
        self.path.clone()
    }
    fn active_buffer_text(&self) -> String {
        self.text.clone()
    }
    fn active_is_dirty(&self) -> bool {
        self.dirty
    }
}

/// Quantas linhas **visuais** uma linha lógica ocupa.
fn line_visual_height(content: &str, text_width: usize) -> usize {
    let w = text_width.max(1);
    let n = content.chars().count();
    if n == 0 {
        return 1;
    }
    n.div_ceil(w)
}

/// Índice da linha visual do caret, contado a partir de `scroll_y` (= 0 no topo).
fn visual_row_of_caret(
    buffer: &oride_core::Buffer,
    scroll_y: usize,
    caret_line: usize,
    caret_col: usize,
    text_width: usize,
    soft_wrap: bool,
) -> usize {
    if caret_line < scroll_y {
        return 0;
    }
    if !soft_wrap {
        return caret_line - scroll_y;
    }
    let w = text_width.max(1);
    let mut rows = 0usize;
    for ln in scroll_y..caret_line {
        let t = buffer.line_text(ln).unwrap_or_default();
        // line_text pode incluir `\n` no fim — remove para medir wrap
        let t = t.trim_end_matches(['\n', '\r']);
        rows = rows.saturating_add(line_visual_height(t, w));
    }
    rows.saturating_add(caret_col / w)
}

/// Calcula `scroll_y` (primeira linha lógica) para manter o caret na última
/// linha visual do viewport.
fn scroll_origin_for_caret(
    buffer: &oride_core::Buffer,
    caret_line: usize,
    caret_col: usize,
    viewport_rows: usize,
    text_width: usize,
    soft_wrap: bool,
) -> usize {
    let height = viewport_rows.max(1);
    if !soft_wrap {
        return caret_line.saturating_add(1).saturating_sub(height);
    }
    let w = text_width.max(1);
    // Queremos que a linha visual do caret fique no índice `height - 1`.
    // Acumulamos linhas lógicas de trás para frente.
    let mut remaining = height.saturating_sub(1);
    // linhas visuais acima do caret na mesma linha lógica
    let on_line_above = caret_col / w;
    if on_line_above >= remaining {
        return caret_line;
    }
    remaining -= on_line_above;
    let mut start = caret_line;
    while start > 0 && remaining > 0 {
        start -= 1;
        let t = buffer.line_text(start).unwrap_or_default();
        let t = t.trim_end_matches(['\n', '\r']);
        let h = line_visual_height(t, w);
        if h > remaining {
            // linha anterior não cabe inteira → começa depois dela
            start += 1;
            break;
        }
        remaining -= h;
    }
    start
}

fn build_default_keymap() -> Keymap {
    let defaults = Config::default();
    Keymap::from_string_map(defaults.keys.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .expect("default key bindings must parse")
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    let path = path.replace("%20", " ");
    Some(PathBuf::from(path))
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
        // untitled → Save As browser (em vez de erro “no path”)
        assert!(matches!(app.overlay, Overlay::Browse(_)));
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
        let items = app.command_palette_items("tab");
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
    fn open_folder_opens_browser() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::OpenFolder));
        assert!(matches!(app.overlay, Overlay::Browse(_)));
        if let Overlay::Browse(b) = &app.overlay {
            assert_eq!(b.mode, BrowseMode::Folder);
        }
    }

    #[test]
    fn open_file_opens_browser() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::OpenFileFuzzy));
        assert!(matches!(app.overlay, Overlay::Browse(_)));
        if let Overlay::Browse(b) = &app.overlay {
            assert_eq!(b.mode, BrowseMode::File);
        }
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
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('h'))),
            Some(KeyCommand::Action(Action::Replace))
        );
        assert_eq!(
            app.map_key(KeyEvent {
                code: KeyCode::F(1),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            }),
            Some(KeyCommand::Action(Action::Help))
        );
        let list_keys = KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(list_keys),
            Some(KeyCommand::Action(Action::Help))
        );
        assert_eq!(
            app.map_key(key_ctrl(KeyCode::Char('"'))),
            Some(KeyCommand::Action(Action::ToggleTerminal))
        );
    }

    #[test]
    fn project_find_opens_and_lists_hits() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "needle here\n").unwrap();
        fs::write(dir.path().join("b.txt"), "other\n").unwrap();
        let mut store = DocumentStore::new();
        store.open_empty();
        let mut app =
            App::from_store_with_config(store, Config::default(), dir.path().to_path_buf());
        app.apply(KeyCommand::Action(Action::ProjectFind));
        assert!(matches!(app.overlay, Overlay::ProjectFind { .. }));
        // simula digitar "needle"
        for c in "needle".chars() {
            app.handle_key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            });
        }
        if let Overlay::ProjectFind { hits, .. } = &app.overlay {
            assert!(!hits.is_empty(), "expected hits for needle");
            assert!(hits.iter().any(|h| h.line_text.contains("needle")));
        } else {
            panic!("expected ProjectFind overlay");
        }
        let pf = KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let app2 = App::new_empty();
        assert_eq!(
            app2.map_key(pf),
            Some(KeyCommand::Action(Action::ProjectFind))
        );
    }

    #[test]
    fn help_lists_all_keybinds() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::Help));
        assert!(matches!(app.overlay, Overlay::Help { .. }));
        let items = app.keybind_list_items("");
        assert!(
            items.len() >= 10,
            "expected full keybind list, got {}",
            items.len()
        );
        assert!(
            items
                .iter()
                .any(|l| l.contains("ctrl+s") && l.contains("Save")),
            "missing ctrl+s save: {items:?}"
        );
        assert!(
            items
                .iter()
                .any(|l| l.contains("f1") && l.contains("keybind")),
            "missing f1 help: {items:?}"
        );
        // filtro
        let filtered = app.keybind_list_items("save");
        assert!(!filtered.is_empty());
        assert!(filtered
            .iter()
            .all(|l| l.to_ascii_lowercase().contains("save") || fuzzy_match("save", l)));
    }

    #[test]
    fn map_save_as_and_save_all() {
        use crossterm::event::KeyModifiers;
        let app = App::new_empty();
        let save_as = KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let save_as_upper = KeyEvent {
            code: KeyCode::Char('S'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let save_as_f12 = KeyEvent {
            code: KeyCode::F(12),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let save_all = KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(save_as),
            Some(KeyCommand::Action(Action::SaveAs))
        );
        assert_eq!(
            app.map_key(save_as_upper),
            Some(KeyCommand::Action(Action::SaveAs))
        );
        assert_eq!(
            app.map_key(save_as_f12),
            Some(KeyCommand::Action(Action::SaveAs))
        );
        assert_eq!(
            app.map_key(save_all),
            Some(KeyCommand::Action(Action::SaveAll))
        );
    }

    #[test]
    fn save_as_opens_path_browser() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::SaveAs));
        assert!(matches!(app.overlay, Overlay::Browse(_)));
        if let Overlay::Browse(b) = &app.overlay {
            assert_eq!(b.mode, BrowseMode::SaveAs);
            assert!(!b.filter.is_empty());
        }
    }

    #[test]
    fn save_without_path_opens_save_as() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('x'));
        app.apply(KeyCommand::Action(Action::Save));
        assert!(
            matches!(app.overlay, Overlay::Browse(_)),
            "Ctrl+S em untitled deve abrir Save As"
        );
        if let Overlay::Browse(b) = &app.overlay {
            assert_eq!(b.mode, BrowseMode::SaveAs);
        }
    }

    #[test]
    fn handle_key_ctrl_shift_s_opens_browser() {
        let mut app = App::new_empty();
        app.handle_key(KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert!(
            matches!(app.overlay, Overlay::Browse(_)),
            "handle_key Ctrl+Shift+S deve abrir browser"
        );
    }

    #[test]
    fn map_tab_navigation_keys() {
        use crossterm::event::KeyModifiers;
        let app = App::new_empty();
        let page_up = KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let page_down = KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let alt_left = KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let alt_right = KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(page_up),
            Some(KeyCommand::Action(Action::PrevTab))
        );
        assert_eq!(
            app.map_key(page_down),
            Some(KeyCommand::Action(Action::NextTab))
        );
        assert_eq!(
            app.map_key(alt_left),
            Some(KeyCommand::Action(Action::PrevTab))
        );
        assert_eq!(
            app.map_key(alt_right),
            Some(KeyCommand::Action(Action::NextTab))
        );
    }

    #[test]
    fn selection_extend_and_select_all() {
        use crossterm::event::KeyModifiers;
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('a'));
        app.apply(KeyCommand::InsertChar('b'));
        app.apply(KeyCommand::InsertChar('c'));
        app.apply(KeyCommand::Action(Action::MoveLineStart { extend: false }));
        app.apply(KeyCommand::Action(Action::MoveRight { extend: true }));
        app.apply(KeyCommand::Action(Action::MoveRight { extend: true }));
        let doc = app.store.active().unwrap();
        assert_eq!(doc.selected_text(), "ab");

        app.apply(KeyCommand::Action(Action::SelectAll));
        let doc = app.store.active().unwrap();
        assert_eq!(doc.selected_text(), "abc");

        let shift_end = KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(shift_end),
            Some(KeyCommand::Action(Action::MoveLineEnd { extend: true }))
        );
        let ctrl_a = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(ctrl_a),
            Some(KeyCommand::Action(Action::SelectAll))
        );
        let ctrl_shift_end = KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(ctrl_shift_end),
            Some(KeyCommand::Action(Action::MoveDocEnd { extend: true }))
        );
    }

    #[test]
    fn copy_paste_roundtrip_internal() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('x'));
        app.apply(KeyCommand::InsertChar('y'));
        app.apply(KeyCommand::Action(Action::SelectAll));
        app.apply(KeyCommand::Action(Action::Copy));
        app.apply(KeyCommand::Action(Action::MoveDocEnd { extend: false }));
        app.apply(KeyCommand::Action(Action::Paste));
        let text = app.store.active().unwrap().buffer().as_string();
        assert_eq!(text, "xyxy");
    }

    #[test]
    fn save_as_enter_confirms_with_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = App::new_empty();
        app.apply(KeyCommand::InsertChar('z'));
        let mut browser = crate::browser::PathBrowser::new(dir.path(), BrowseMode::SaveAs);
        browser.filter = "out.txt".into();
        app.overlay = Overlay::Browse(browser);
        // Enter no SaveAs com nome → salva
        app.handle_key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert!(matches!(app.overlay, Overlay::None));
        assert!(dir.path().join("out.txt").exists());
    }

    #[test]
    fn toggle_scm_and_buffer_picker() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::ToggleScm));
        assert!(app.show_scm);
        assert_eq!(app.focus, Focus::Scm);
        app.apply(KeyCommand::Action(Action::ToggleScm));
        assert!(!app.show_scm);
        app.apply(KeyCommand::Action(Action::BufferPicker));
        assert!(matches!(app.overlay, Overlay::BufferPicker { .. }));
    }

    #[test]
    fn which_key_and_menu_hotkey() {
        let mut app = App::new_empty();
        app.apply(KeyCommand::Action(Action::WhichKey));
        assert!(app.show_which_key);
        app.handle_key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert!(!app.show_which_key);

        // Alt+F abre menu File
        app.handle_key(KeyEvent {
            code: KeyCode::Char('f'),
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert_eq!(app.menu_open, Some((0, 0)));
        app.handle_key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert!(app.menu_open.is_none());
    }

    #[test]
    fn terminal_ctrl_c_not_stolen_as_copy() {
        let mut app = App::new_empty();
        app.ensure_terminal();
        if app.terminal.is_none() {
            return; // ambiente sem PTY
        }
        if let Some(t) = app.terminal.as_mut() {
            t.visible = true;
        }
        app.focus = Focus::Terminal;
        // Ctrl+C com foco terminal deve ser consumido pelo handler (não Copy)
        app.handle_key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert_eq!(app.focus, Focus::Terminal);
        // Esc volta
        app.handle_key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        });
        assert_eq!(app.focus, Focus::Editor);
    }

    #[test]
    fn jump_list_records_and_back() {
        use oride_core::ByteOffset;
        let mut jl = crate::jump_list::JumpList::default();
        jl.push(Jump {
            path: None,
            byte: ByteOffset::new(0),
            line: 0,
        });
        jl.push(Jump {
            path: None,
            byte: ByteOffset::new(10),
            line: 1,
        });
        let b = jl.back().expect("back");
        assert_eq!(b.byte, ByteOffset::new(0));
    }

    #[test]
    fn map_ux_actions() {
        let app = App::new_empty();
        let scm = KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(scm),
            Some(KeyCommand::Action(Action::ToggleScm))
        );
        let pick = KeyEvent {
            code: KeyCode::Char('o'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        assert_eq!(
            app.map_key(pick),
            Some(KeyCommand::Action(Action::BufferPicker))
        );
    }

    #[test]
    fn scroll_follows_cursor_down() {
        let mut app = App::new_empty();
        app.last_editor_height = 5;
        app.last_editor_text_width = 40;
        app.soft_wrap = false;
        // 20 linhas
        for i in 0..20 {
            if i > 0 {
                app.apply(KeyCommand::Action(Action::InsertNewline));
            }
            app.apply(KeyCommand::InsertChar('x'));
        }
        // caret no fim; com height 5 deve scrollar
        app.ensure_cursor_visible();
        let caret = app.store.active().unwrap().caret().unwrap();
        assert!(
            caret.line >= app.scroll_y,
            "caret.line={} scroll={}",
            caret.line,
            app.scroll_y
        );
        assert!(
            caret.line < app.scroll_y + app.last_editor_height,
            "caret saiu da tela: line={} scroll={} height={}",
            caret.line,
            app.scroll_y,
            app.last_editor_height
        );
        // sobe para o topo
        app.apply(KeyCommand::Action(Action::MoveDocStart { extend: false }));
        app.ensure_cursor_visible();
        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn scroll_follows_cursor_with_soft_wrap() {
        let mut app = App::new_empty();
        app.last_editor_height = 4;
        app.last_editor_text_width = 10;
        app.soft_wrap = true;
        // uma linha longa (3 rows visuais) + várias curtas
        let long = "abcdefghij".repeat(3); // 30 chars → 3 visual rows
        for c in long.chars() {
            app.apply(KeyCommand::InsertChar(c));
        }
        app.apply(KeyCommand::Action(Action::InsertNewline));
        for _ in 0..10 {
            app.apply(KeyCommand::InsertChar('z'));
            app.apply(KeyCommand::Action(Action::InsertNewline));
        }
        app.ensure_cursor_visible();
        let caret = app.store.active().unwrap().caret().unwrap();
        let visual = visual_row_of_caret(
            app.store.active().unwrap().buffer(),
            app.scroll_y,
            caret.line,
            caret.column,
            app.last_editor_text_width,
            true,
        );
        assert!(
            visual < app.last_editor_height,
            "soft-wrap: caret visual={visual} height={} scroll={}",
            app.last_editor_height,
            app.scroll_y
        );
    }

    #[test]
    fn scroll_origin_helpers() {
        let buf = oride_core::Buffer::from_text("aa\nbb\ncc\ndd\nee\n");
        assert_eq!(line_visual_height("hello", 10), 1);
        assert_eq!(line_visual_height("hello world!!", 5), 3);
        let origin = scroll_origin_for_caret(&buf, 4, 0, 3, 40, false);
        assert_eq!(origin, 2); // lines 2,3,4 visible
    }
}
