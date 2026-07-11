//! Ações nomeadas (ids estáveis em TOML).

use thiserror::Error;

/// Ação de editor resolvida a partir do keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Save,
    Undo,
    Redo,
    InsertNewline,
    InsertTab,
    Backspace,
    Delete,
    MoveLeft {
        extend: bool,
    },
    MoveRight {
        extend: bool,
    },
    MoveUp {
        extend: bool,
    },
    MoveDown {
        extend: bool,
    },
    MoveLineStart {
        extend: bool,
    },
    MoveLineEnd {
        extend: bool,
    },
    PageUp,
    PageDown,
    // IDE shell (P1)
    ToggleTree,
    ToggleTerminal,
    FocusTree,
    FocusEditor,
    FocusTerminal,
    NextTab,
    PrevTab,
    CloseTab,
    NewTab,
    CommandPalette,
    OpenFileFuzzy,
    TreeNewFile,
    TreeNewDir,
    TreeRefresh,
    /// Abre uma pasta como workspace do editor.
    OpenFolder,
    /// Alterna foco entre editor e árvore (sem esconder o painel).
    FocusToggleTreeEditor,
    /// Soft wrap (quebra visual de linha).
    ToggleSoftWrap,
    /// Comenta/descomenta linha(s) conforme a linguagem.
    ToggleComment,
}

impl Action {
    /// Rótulo para a command palette.
    #[must_use]
    pub fn palette_label(self) -> &'static str {
        match self {
            Self::Quit => "Quit",
            Self::Save => "Save",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::InsertNewline => "Insert newline",
            Self::InsertTab => "Insert tab",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::MoveLeft { .. } => "Move left",
            Self::MoveRight { .. } => "Move right",
            Self::MoveUp { .. } => "Move up",
            Self::MoveDown { .. } => "Move down",
            Self::MoveLineStart { .. } => "Line start",
            Self::MoveLineEnd { .. } => "Line end",
            Self::PageUp => "Page up",
            Self::PageDown => "Page down",
            Self::ToggleTree => "Toggle project tree",
            Self::ToggleTerminal => "Toggle terminal",
            Self::FocusTree => "Focus tree",
            Self::FocusEditor => "Focus editor",
            Self::FocusTerminal => "Focus terminal",
            Self::NextTab => "Next tab",
            Self::PrevTab => "Previous tab",
            Self::CloseTab => "Close tab",
            Self::NewTab => "New tab",
            Self::CommandPalette => "Command palette",
            Self::OpenFileFuzzy => "Open file (fuzzy)",
            Self::TreeNewFile => "New file (tree)",
            Self::TreeNewDir => "New folder (tree)",
            Self::TreeRefresh => "Refresh tree",
            Self::OpenFolder => "Open folder…",
            Self::FocusToggleTreeEditor => "Focus: toggle tree / editor",
            Self::ToggleSoftWrap => "Toggle soft wrap",
            Self::ToggleComment => "Toggle comment",
        }
    }

    /// Actions listadas na palette de comandos.
    pub fn palette_actions() -> &'static [Action] {
        &[
            Action::Save,
            Action::Undo,
            Action::Redo,
            Action::NewTab,
            Action::CloseTab,
            Action::NextTab,
            Action::PrevTab,
            Action::OpenFolder,
            Action::ToggleSoftWrap,
            Action::ToggleComment,
            Action::ToggleTree,
            Action::ToggleTerminal,
            Action::FocusTree,
            Action::FocusEditor,
            Action::FocusToggleTreeEditor,
            Action::FocusTerminal,
            Action::OpenFileFuzzy,
            Action::CommandPalette,
            Action::TreeNewFile,
            Action::TreeNewDir,
            Action::TreeRefresh,
            Action::Quit,
        ]
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown action id: {0}")]
pub struct ActionParseError(pub String);

pub fn parse_action(id: &str) -> Result<Action, ActionParseError> {
    let action = match id {
        "quit" => Action::Quit,
        "save" => Action::Save,
        "undo" => Action::Undo,
        "redo" => Action::Redo,
        "insert_newline" => Action::InsertNewline,
        "insert_tab" => Action::InsertTab,
        "backspace" => Action::Backspace,
        "delete" => Action::Delete,
        "move_left" => Action::MoveLeft { extend: false },
        "move_right" => Action::MoveRight { extend: false },
        "move_up" => Action::MoveUp { extend: false },
        "move_down" => Action::MoveDown { extend: false },
        "move_line_start" => Action::MoveLineStart { extend: false },
        "move_line_end" => Action::MoveLineEnd { extend: false },
        "move_left_extend" => Action::MoveLeft { extend: true },
        "move_right_extend" => Action::MoveRight { extend: true },
        "move_up_extend" => Action::MoveUp { extend: true },
        "move_down_extend" => Action::MoveDown { extend: true },
        "move_line_start_extend" => Action::MoveLineStart { extend: true },
        "move_line_end_extend" => Action::MoveLineEnd { extend: true },
        "page_up" => Action::PageUp,
        "page_down" => Action::PageDown,
        "toggle_tree" => Action::ToggleTree,
        "toggle_terminal" => Action::ToggleTerminal,
        "focus_tree" => Action::FocusTree,
        "focus_editor" => Action::FocusEditor,
        "focus_terminal" => Action::FocusTerminal,
        "next_tab" => Action::NextTab,
        "prev_tab" => Action::PrevTab,
        "close_tab" => Action::CloseTab,
        "new_tab" => Action::NewTab,
        "command_palette" => Action::CommandPalette,
        "open_file_fuzzy" => Action::OpenFileFuzzy,
        "tree_new_file" => Action::TreeNewFile,
        "tree_new_dir" => Action::TreeNewDir,
        "tree_refresh" => Action::TreeRefresh,
        "open_folder" => Action::OpenFolder,
        "focus_toggle_tree_editor" => Action::FocusToggleTreeEditor,
        "toggle_soft_wrap" => Action::ToggleSoftWrap,
        "toggle_comment" => Action::ToggleComment,
        other => return Err(ActionParseError(other.to_string())),
    };
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_actions() {
        assert_eq!(parse_action("save").unwrap(), Action::Save);
        assert_eq!(parse_action("toggle_tree").unwrap(), Action::ToggleTree);
        assert_eq!(
            parse_action("move_left_extend").unwrap(),
            Action::MoveLeft { extend: true }
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!(parse_action("fly_to_moon").is_err());
    }
}
