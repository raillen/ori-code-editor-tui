//! Parse e normalização de chords (`ctrl+s`, `shift+left`, …).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use thiserror::Error;

/// Chord canônico para lookup no mapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub code: KeyCode,
}

impl KeyChord {
    #[must_use]
    pub fn from_event(key: KeyEvent) -> Self {
        let mut ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let mut shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let code = match key.code {
            KeyCode::Char(c) => {
                let lower = c.to_ascii_lowercase();
                if c.is_ascii_uppercase() {
                    shift = true;
                }
                KeyCode::Char(lower)
            }
            other => other,
        };
        if matches!(code, KeyCode::Char(_)) && !ctrl && !alt {
            shift = key.modifiers.contains(KeyModifiers::SHIFT);
        }
        if matches!(
            code,
            KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::F(_)
        ) {
            shift = key.modifiers.contains(KeyModifiers::SHIFT);
            ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        }
        // Alguns terminais enviam Ctrl+/ como Char('?') ou Char('_') — normaliza common case
        let code = match (ctrl, code) {
            (true, KeyCode::Char('?')) => KeyCode::Char('/'),
            _ => code,
        };
        Self {
            ctrl,
            alt,
            shift,
            code,
        }
    }

    /// Forma estável minúscula: `ctrl+shift+left`, `esc`, `ctrl+s`.
    #[must_use]
    pub fn canonical_string(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("ctrl".into());
        }
        if self.alt {
            parts.push("alt".into());
        }
        if self.shift {
            parts.push("shift".into());
        }
        parts.push(code_token(self.code));
        parts.join("+")
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseChordError {
    #[error("empty chord")]
    Empty,
    #[error("unknown key token: {0}")]
    UnknownToken(String),
}

pub fn parse_chord(s: &str) -> Result<KeyChord, ParseChordError> {
    let s = s.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Err(ParseChordError::Empty);
    }
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut code: Option<KeyCode> = None;

    for part in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part {
            "ctrl" | "control" | "cmd" | "super" => ctrl = true,
            "alt" | "option" | "opt" => alt = true,
            "shift" => shift = true,
            token => {
                if code.is_some() {
                    return Err(ParseChordError::UnknownToken(token.into()));
                }
                code = Some(parse_code_token(token)?);
            }
        }
    }

    let code = code.ok_or_else(|| ParseChordError::UnknownToken(s.clone()))?;
    Ok(KeyChord {
        ctrl,
        alt,
        shift,
        code,
    })
}

fn parse_code_token(token: &str) -> Result<KeyCode, ParseChordError> {
    let code = match token {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "backspace" | "bs" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "tab" => KeyCode::Tab,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" | "pgdown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f5" => KeyCode::F(5),
        "`" | "grave" | "backtick" => KeyCode::Char('`'),
        "\\" | "backslash" => KeyCode::Char('\\'),
        "/" | "slash" => KeyCode::Char('/'),
        "\"" | "quote" | "doublequote" | "dquote" => KeyCode::Char('"'),
        "'" | "apostrophe" | "squote" => KeyCode::Char('\''),
        s if s.chars().count() == 1 => {
            let c = s.chars().next().unwrap();
            KeyCode::Char(c)
        }
        other => return Err(ParseChordError::UnknownToken(other.into())),
    };
    Ok(code)
}

fn code_token(code: KeyCode) -> String {
    match code {
        KeyCode::Esc => "esc".into(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Delete => "delete".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pageup".into(),
        KeyCode::PageDown => "pagedown".into(),
        KeyCode::F(1) => "f1".into(),
        KeyCode::F(2) => "f2".into(),
        KeyCode::F(3) => "f3".into(),
        KeyCode::F(5) => "f5".into(),
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char('`') => "`".into(),
        KeyCode::Char('\\') => "\\".into(),
        KeyCode::Char('"') => "\"".into(),
        KeyCode::Char('\'') => "'".into(),
        KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
        _ => "?".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ctrl_s() {
        let c = parse_chord("ctrl+s").unwrap();
        assert!(c.ctrl);
        assert_eq!(c.code, KeyCode::Char('s'));
        assert_eq!(c.canonical_string(), "ctrl+s");
    }

    #[test]
    fn parse_shift_left() {
        let c = parse_chord("shift+left").unwrap();
        assert!(c.shift);
        assert_eq!(c.code, KeyCode::Left);
    }

    #[test]
    fn roundtrip_defaults() {
        for s in ["esc", "ctrl+z", "pageup", "shift+home"] {
            let c = parse_chord(s).unwrap();
            assert_eq!(c.canonical_string(), s);
        }
    }
}
