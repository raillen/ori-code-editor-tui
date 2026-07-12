//! Modelo serializável da config (TOML).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Config efetiva após merge de camadas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Nome lógico do tema (reserva futura; cores vêm de `[ui]` / `[syntax]`).
    pub theme: String,
    pub show_line_numbers: bool,
    pub soft_wrap: bool,
    pub editor: EditorConfig,
    pub theme_ui: ThemeUiConfig,
    pub syntax: SyntaxColorsConfig,
    pub tree: TreeConfig,
    pub terminal: TerminalConfig,
    pub lsp: LspConfig,
    /// Chord string → action id (`"ctrl+s" = "save"`).
    pub keys: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "default".into(),
            show_line_numbers: true,
            soft_wrap: false,
            editor: EditorConfig::default(),
            theme_ui: ThemeUiConfig::default(),
            syntax: SyntaxColorsConfig::default(),
            tree: TreeConfig::default(),
            terminal: TerminalConfig::default(),
            lsp: LspConfig::default(),
            keys: default_key_bindings(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub format_on_save: bool,
    /// Aplicar `.editorconfig` ao abrir arquivo (indent).
    pub use_editorconfig: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            format_on_save: false,
            use_editorconfig: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TreeConfig {
    pub width: u16,
    pub show_hidden: bool,
    pub git_status: bool,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            width: 28,
            show_hidden: false,
            git_status: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Shell; vazio = `$SHELL` ou `/bin/sh`.
    pub shell: String,
    pub default_height: u16,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: String::new(),
            default_height: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LspConfig {
    pub enabled: bool,
    /// Ex.: `["oriscript", "lsp"]`
    pub oriscript_command: Vec<String>,
    pub timeout_ms: u64,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            oriscript_command: vec!["oriscript".into(), "lsp".into()],
            timeout_ms: 10_000,
        }
    }
}

/// Cores da UI como strings (`"white"`, `"#1a1b26"`, `"reset"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeUiConfig {
    pub background: String,
    pub foreground: String,
    pub line_number: String,
    pub status_bg: String,
    pub status_fg: String,
    pub status_dirty: String,
    pub cursor_bg: String,
    pub cursor_fg: String,
    pub gutter_width: u16,
}

impl Default for ThemeUiConfig {
    fn default() -> Self {
        Self {
            background: "reset".into(),
            foreground: "reset".into(),
            line_number: "darkgray".into(),
            status_bg: "darkgray".into(),
            status_fg: "white".into(),
            status_dirty: "yellow".into(),
            cursor_bg: "white".into(),
            cursor_fg: "black".into(),
            gutter_width: 5,
        }
    }
}

/// Cores de syntax highlight (opcional no TOML `[syntax]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SyntaxColorsConfig {
    pub comment: String,
    pub keyword: String,
    pub string: String,
    pub number: String,
    pub type_name: String,
    pub function: String,
    pub operator: String,
    pub punctuation: String,
    pub variable: String,
    pub constant: String,
    pub property: String,
    pub tag: String,
    pub attribute: String,
    pub heading: String,
    pub emphasis: String,
    pub strong: String,
    pub link: String,
    pub code: String,
    pub list_marker: String,
    pub quote: String,
}

impl Default for SyntaxColorsConfig {
    fn default() -> Self {
        Self {
            comment: "darkgray".into(),
            keyword: "magenta".into(),
            string: "green".into(),
            number: "yellow".into(),
            type_name: "cyan".into(),
            function: "blue".into(),
            operator: "reset".into(),
            punctuation: "darkgray".into(),
            variable: "reset".into(),
            constant: "yellow".into(),
            property: "cyan".into(),
            tag: "red".into(),
            attribute: "yellow".into(),
            heading: "magenta".into(),
            emphasis: "cyan".into(),
            strong: "yellow".into(),
            link: "blue".into(),
            code: "green".into(),
            list_marker: "yellow".into(),
            quote: "darkgray".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct EditorConfigPartial {
    tab_size: Option<u8>,
    insert_spaces: Option<bool>,
    format_on_save: Option<bool>,
    use_editorconfig: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ThemeUiConfigPartial {
    background: Option<String>,
    foreground: Option<String>,
    line_number: Option<String>,
    status_bg: Option<String>,
    status_fg: Option<String>,
    status_dirty: Option<String>,
    cursor_bg: Option<String>,
    cursor_fg: Option<String>,
    gutter_width: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct SyntaxPartial {
    comment: Option<String>,
    keyword: Option<String>,
    string: Option<String>,
    number: Option<String>,
    type_name: Option<String>,
    function: Option<String>,
    operator: Option<String>,
    punctuation: Option<String>,
    variable: Option<String>,
    constant: Option<String>,
    property: Option<String>,
    tag: Option<String>,
    attribute: Option<String>,
    heading: Option<String>,
    emphasis: Option<String>,
    strong: Option<String>,
    link: Option<String>,
    code: Option<String>,
    list_marker: Option<String>,
    quote: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct TreePartial {
    width: Option<u16>,
    show_hidden: Option<bool>,
    git_status: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct TerminalPartial {
    shell: Option<String>,
    default_height: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct LspPartial {
    enabled: Option<bool>,
    oriscript_command: Option<Vec<String>>,
    timeout_ms: Option<u64>,
}

/// Arquivo TOML real (campos opcionais).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawConfigFile {
    theme: Option<String>,
    show_line_numbers: Option<bool>,
    soft_wrap: Option<bool>,
    editor: Option<EditorConfigPartial>,
    ui: Option<ThemeUiConfigPartial>,
    syntax: Option<SyntaxPartial>,
    tree: Option<TreePartial>,
    terminal: Option<TerminalPartial>,
    lsp: Option<LspPartial>,
    keys: Option<BTreeMap<String, String>>,
}

impl Config {
    pub(crate) fn apply_raw(&mut self, raw: RawConfigFile) {
        if let Some(v) = raw.theme {
            self.theme = v;
        }
        if let Some(v) = raw.show_line_numbers {
            self.show_line_numbers = v;
        }
        if let Some(v) = raw.soft_wrap {
            self.soft_wrap = v;
        }
        if let Some(ed) = raw.editor {
            if let Some(v) = ed.tab_size {
                self.editor.tab_size = v.max(1);
            }
            if let Some(v) = ed.insert_spaces {
                self.editor.insert_spaces = v;
            }
            if let Some(v) = ed.format_on_save {
                self.editor.format_on_save = v;
            }
            if let Some(v) = ed.use_editorconfig {
                self.editor.use_editorconfig = v;
            }
        }
        if let Some(ui) = raw.ui {
            merge_theme_ui(&mut self.theme_ui, ui);
        }
        if let Some(sx) = raw.syntax {
            merge_syntax(&mut self.syntax, sx);
        }
        if let Some(t) = raw.tree {
            if let Some(v) = t.width {
                self.tree.width = v.max(8);
            }
            if let Some(v) = t.show_hidden {
                self.tree.show_hidden = v;
            }
            if let Some(v) = t.git_status {
                self.tree.git_status = v;
            }
        }
        if let Some(t) = raw.terminal {
            if let Some(v) = t.shell {
                self.terminal.shell = v;
            }
            if let Some(v) = t.default_height {
                self.terminal.default_height = v.max(3);
            }
        }
        if let Some(l) = raw.lsp {
            if let Some(v) = l.enabled {
                self.lsp.enabled = v;
            }
            if let Some(v) = l.oriscript_command {
                if !v.is_empty() {
                    self.lsp.oriscript_command = v;
                }
            }
            if let Some(v) = l.timeout_ms {
                self.lsp.timeout_ms = v.max(500);
            }
        }
        if let Some(keys) = raw.keys {
            for (k, v) in keys {
                self.keys.insert(k, v);
            }
        }
    }
}

fn merge_theme_ui(dst: &mut ThemeUiConfig, src: ThemeUiConfigPartial) {
    if let Some(v) = src.background {
        dst.background = v;
    }
    if let Some(v) = src.foreground {
        dst.foreground = v;
    }
    if let Some(v) = src.line_number {
        dst.line_number = v;
    }
    if let Some(v) = src.status_bg {
        dst.status_bg = v;
    }
    if let Some(v) = src.status_fg {
        dst.status_fg = v;
    }
    if let Some(v) = src.status_dirty {
        dst.status_dirty = v;
    }
    if let Some(v) = src.cursor_bg {
        dst.cursor_bg = v;
    }
    if let Some(v) = src.cursor_fg {
        dst.cursor_fg = v;
    }
    if let Some(v) = src.gutter_width {
        dst.gutter_width = v.max(1);
    }
}

fn merge_syntax(dst: &mut SyntaxColorsConfig, src: SyntaxPartial) {
    macro_rules! set {
        ($field:ident) => {
            if let Some(v) = src.$field {
                dst.$field = v;
            }
        };
    }
    set!(comment);
    set!(keyword);
    set!(string);
    set!(number);
    set!(type_name);
    set!(function);
    set!(operator);
    set!(punctuation);
    set!(variable);
    set!(constant);
    set!(property);
    set!(tag);
    set!(attribute);
    set!(heading);
    set!(emphasis);
    set!(strong);
    set!(link);
    set!(code);
    set!(list_marker);
    set!(quote);
}

/// Bindings padrão (P0–P4 + P3 LSP).
pub fn default_key_bindings() -> BTreeMap<String, String> {
    let pairs = [
        ("ctrl+s", "save"),
        ("ctrl+shift+s", "save_as"),
        ("f12", "save_as"),
        ("alt+shift+s", "save_as"),
        ("ctrl+alt+s", "save_all"),
        ("ctrl+q", "quit"),
        ("esc", "quit"),
        ("ctrl+z", "undo"),
        ("ctrl+y", "redo"),
        ("ctrl+c", "copy"),
        ("ctrl+v", "paste"),
        ("ctrl+x", "cut"),
        ("ctrl+a", "select_all"),
        ("ctrl+f", "find"),
        ("f3", "find_next"),
        ("shift+f3", "find_prev"),
        ("ctrl+shift+f", "project_find"),
        ("ctrl+h", "replace"),
        ("ctrl+shift+h", "replace"),
        ("f1", "help"),
        ("ctrl+g", "help"),
        ("ctrl+shift+/", "help"),
        ("ctrl+?", "help"),
        ("enter", "insert_newline"),
        ("backspace", "backspace"),
        ("delete", "delete"),
        ("left", "move_left"),
        ("right", "move_right"),
        ("up", "move_up"),
        ("down", "move_down"),
        ("home", "move_line_start"),
        ("end", "move_line_end"),
        ("pageup", "page_up"),
        ("pagedown", "page_down"),
        ("shift+left", "move_left_extend"),
        ("shift+right", "move_right_extend"),
        ("shift+up", "move_up_extend"),
        ("shift+down", "move_down_extend"),
        ("shift+home", "move_line_start_extend"),
        ("shift+end", "move_line_end_extend"),
        ("ctrl+home", "move_doc_start"),
        ("ctrl+end", "move_doc_end"),
        ("ctrl+shift+home", "move_doc_start_extend"),
        ("ctrl+shift+end", "move_doc_end_extend"),
        ("tab", "insert_tab"),
        ("ctrl+b", "focus_tree"),
        ("ctrl+e", "focus_editor"),
        ("ctrl+shift+b", "toggle_tree"),
        ("ctrl+shift+e", "focus_toggle_tree_editor"),
        ("ctrl+\"", "toggle_terminal"),
        ("ctrl+'", "toggle_terminal"),
        ("ctrl+`", "toggle_terminal"),
        ("alt+=", "terminal_grow"),
        ("alt+-", "terminal_shrink"),
        ("ctrl+\\", "focus_tree"),
        ("ctrl+o", "open_folder"),
        ("alt+z", "toggle_soft_wrap"),
        ("ctrl+/", "toggle_comment"),
        ("ctrl+shift+v", "toggle_md_preview"),
        ("alt+p", "toggle_md_preview"),
        ("ctrl+pageup", "prev_tab"),
        ("ctrl+pagedown", "next_tab"),
        ("alt+left", "prev_tab"),
        ("alt+right", "next_tab"),
        ("ctrl+shift+[", "prev_tab"),
        ("ctrl+shift+]", "next_tab"),
        ("ctrl+w", "close_tab"),
        ("ctrl+n", "new_tab"),
        ("ctrl+shift+p", "command_palette"),
        ("ctrl+p", "open_file_fuzzy"),
        ("ctrl+shift+n", "tree_new_file"),
        ("ctrl+shift+d", "tree_new_dir"),
        ("f5", "tree_refresh"),
        // P3 LSP
        ("ctrl+space", "lsp_complete"),
        ("ctrl+k", "lsp_hover"),
        ("f4", "lsp_goto_definition"),
        ("ctrl+shift+i", "lsp_format"),
        ("ctrl+shift+m", "toggle_diagnostics"),
        ("ctrl+r", "reload_file"),
    ];
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
