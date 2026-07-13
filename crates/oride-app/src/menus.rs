//! Definição da menu bar (File / Edit / View / Go / Git / Help).

use oride_ui::{MenuColumn, MenuItem};

/// Menus default do Oride (labels + action_id + shortcut visual).
#[must_use]
pub fn default_menus() -> Vec<MenuColumn> {
    vec![
        MenuColumn {
            title: "File".into(),
            hotkey: 'f',
            items: vec![
                item("New tab", "ctrl+n", "new_tab"),
                item("Open file…", "ctrl+p", "open_file_fuzzy"),
                item("Open folder…", "ctrl+o", "open_folder"),
                item("Save", "ctrl+s", "save"),
                item("Save as…", "f12", "save_as"),
                item("Save all", "ctrl+alt+s", "save_all"),
                item("Reload file", "ctrl+r", "reload_file"),
                item("Quit", "ctrl+q", "quit"),
            ],
        },
        MenuColumn {
            title: "Edit".into(),
            hotkey: 'e',
            items: vec![
                item("Undo", "ctrl+z", "undo"),
                item("Redo", "ctrl+y", "redo"),
                item("Cut", "ctrl+x", "cut"),
                item("Copy", "ctrl+c", "copy"),
                item("Paste", "ctrl+v", "paste"),
                item("Select all", "ctrl+a", "select_all"),
                item("Find…", "ctrl+f", "find"),
                item("Find in project…", "ctrl+shift+f", "project_find"),
                item("Replace…", "ctrl+h", "replace"),
                item("Toggle comment", "ctrl+/", "toggle_comment"),
            ],
        },
        MenuColumn {
            title: "View".into(),
            hotkey: 'v',
            items: vec![
                item("Command palette…", "ctrl+shift+p", "command_palette"),
                item("Toggle tree", "ctrl+shift+b", "toggle_tree"),
                item("Toggle terminal", "ctrl+\"", "toggle_terminal"),
                item("Toggle SCM panel", "ctrl+shift+g", "toggle_scm"),
                item("MD preview", "alt+p", "toggle_md_preview"),
                item("Soft wrap", "alt+z", "toggle_soft_wrap"),
                item("Split vertical", "ctrl+alt+v", "split_vertical"),
                item("Split horizontal", "ctrl+alt+h", "split_horizontal"),
                item("Which-key", "alt+/", "which_key"),
                item("Enable / disable mouse", "palette", "toggle_mouse"),
            ],
        },
        MenuColumn {
            title: "Go".into(),
            hotkey: 'g',
            items: vec![
                item("Buffer picker…", "ctrl+shift+o", "buffer_picker"),
                item("Jump back", "ctrl+alt+o", "jump_back"),
                item("Jump forward", "ctrl+alt+i", "jump_forward"),
                item("LSP: definition", "f4", "lsp_goto_definition"),
                item("LSP: hover", "ctrl+k", "lsp_hover"),
                item("Diagnostics", "ctrl+shift+m", "toggle_diagnostics"),
                item("Next tab", "ctrl+pagedown", "next_tab"),
                item("Prev tab", "ctrl+pageup", "prev_tab"),
            ],
        },
        MenuColumn {
            title: "Git".into(),
            hotkey: 'i',
            items: vec![
                item("SCM panel", "ctrl+shift+g", "toggle_scm"),
                item("Diff active file", "f2", "show_diff"),
                item("Refresh git", "f5", "tree_refresh"),
                item("Show path", "", "plugin:show_path"),
            ],
        },
        MenuColumn {
            title: "Help".into(),
            hotkey: 'h',
            items: vec![
                item("All keybindings…", "f1", "help"),
                item("Essential shortcuts", "alt+shift+/", "welcome"),
                item("Which-key", "alt+/", "which_key"),
                item("Command palette…", "ctrl+shift+p", "command_palette"),
                item("Multi picker…", "ctrl+shift+t", "multi_picker"),
                item("Surround selection…", "f8", "surround"),
                item("Macro record/stop", "f9", "macro_toggle_record"),
                item("Macro play", "f10", "macro_play"),
                item("Undo history…", "ctrl+shift+u", "undo_tree"),
                item("Word count", "", "plugin:word_count"),
            ],
        },
    ]
}

fn item(label: &str, shortcut: &str, action_id: &str) -> MenuItem {
    MenuItem {
        label: label.into(),
        shortcut: shortcut.into(),
        action_id: action_id.into(),
    }
}
