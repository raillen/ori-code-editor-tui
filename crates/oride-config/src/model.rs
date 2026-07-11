//! Modelo serializável da config (TOML).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Config efetiva após merge de camadas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Nome lógico do tema (reserva futura; cores vêm de `[ui]`).
    pub theme: String,
    pub show_line_numbers: bool,
    pub editor: EditorConfig,
    pub theme_ui: ThemeUiConfig,
    /// Chord string → action id (`"ctrl+s" = "save"`).
    pub keys: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "default".into(),
            show_line_numbers: true,
            editor: EditorConfig::default(),
            theme_ui: ThemeUiConfig::default(),
            keys: default_key_bindings(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub tab_size: u8,
    pub insert_spaces: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct EditorConfigPartial {
    tab_size: Option<u8>,
    insert_spaces: Option<bool>,
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

/// Arquivo TOML real (campos opcionais).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct RawConfigFile {
    theme: Option<String>,
    show_line_numbers: Option<bool>,
    editor: Option<EditorConfigPartial>,
    /// Cores em `[ui]` (evita conflito com `theme = "name"`).
    ui: Option<ThemeUiConfigPartial>,
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
        if let Some(ed) = raw.editor {
            if let Some(v) = ed.tab_size {
                self.editor.tab_size = v.max(1);
            }
            if let Some(v) = ed.insert_spaces {
                self.editor.insert_spaces = v;
            }
        }
        if let Some(ui) = raw.ui {
            merge_theme_ui(&mut self.theme_ui, ui);
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

/// Bindings padrão (P0 + P1 shell).
pub fn default_key_bindings() -> BTreeMap<String, String> {
    let pairs = [
        ("ctrl+s", "save"),
        ("ctrl+shift+s", "save_as"),
        ("ctrl+alt+s", "save_all"),
        ("ctrl+q", "quit"),
        ("esc", "quit"),
        ("ctrl+z", "undo"),
        ("ctrl+y", "redo"),
        ("ctrl+c", "copy"),
        ("ctrl+v", "paste"),
        ("ctrl+x", "cut"),
        ("ctrl+f", "find"),
        ("f3", "find_next"),
        ("shift+f3", "find_prev"),
        ("ctrl+shift+h", "replace"),
        ("ctrl+h", "help"),
        ("ctrl+g", "help"),
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
        ("tab", "insert_tab"),
        // P1 / P2 shell
        // Foco explícito editor ↔ árvore (não esconde o painel)
        ("ctrl+b", "focus_tree"),
        ("ctrl+e", "focus_editor"),
        ("ctrl+shift+b", "toggle_tree"),
        ("ctrl+shift+e", "focus_toggle_tree_editor"),
        ("ctrl+\"", "toggle_terminal"),
        ("ctrl+'", "toggle_terminal"),
        ("ctrl+`", "toggle_terminal"),
        ("ctrl+\\", "focus_tree"),
        ("ctrl+o", "open_folder"),
        ("alt+z", "toggle_soft_wrap"),
        ("ctrl+/", "toggle_comment"),
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
        ("ctrl+shift+f", "tree_new_dir"),
        ("f5", "tree_refresh"),
    ];
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
