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
    MoveLeft { extend: bool },
    MoveRight { extend: bool },
    MoveUp { extend: bool },
    MoveDown { extend: bool },
    MoveLineStart { extend: bool },
    MoveLineEnd { extend: bool },
    PageUp,
    PageDown,
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
