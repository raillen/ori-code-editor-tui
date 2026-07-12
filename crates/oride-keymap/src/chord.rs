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
    /// Normaliza um evento de teclado para lookup estável.
    ///
    /// Terminais reportam Ctrl/Shift de formas diferentes:
    /// - `Char('s')` + CONTROL|SHIFT
    /// - `Char('S')` + CONTROL (sem bit SHIFT)
    /// - `Char('\u{13}')` (ASCII DC3 = Ctrl+S legado) ± modifiers
    #[must_use]
    pub fn from_event(key: KeyEvent) -> Self {
        let mut ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let mut shift = key.modifiers.contains(KeyModifiers::SHIFT);

        let code = match key.code {
            KeyCode::Char(c) => normalize_char_key(c, &mut ctrl, &mut shift),
            other => other,
        };

        Self {
            ctrl,
            alt,
            shift,
            code,
        }
    }

    /// Forma estável minúscula: `ctrl+shift+s`, `esc`, `ctrl+s`.
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

/// Converte Char do evento em letra canônica + atualiza ctrl/shift.
fn normalize_char_key(c: char, ctrl: &mut bool, shift: &mut bool) -> KeyCode {
    // Ctrl+A..Ctrl+Z legados: bytes 1..=26 (sem bit CONTROL em alguns TTYs)
    if let Some(letter) = control_char_to_letter(c) {
        *ctrl = true;
        // Não inventa shift — se o terminal não enviou SHIFT, não há como saber.
        return KeyCode::Char(letter);
    }

    if c.is_ascii_alphabetic() {
        // Ctrl+Shift+S frequentemente chega como 'S' maiúsculo + CONTROL
        // (sem KeyModifiers::SHIFT). Tratar maiúscula como Shift.
        if c.is_ascii_uppercase() {
            *shift = true;
        }
        return KeyCode::Char(c.to_ascii_lowercase());
    }

    // Não-alfabético: se Shift está no modifier e for símbolo, mantém o char.
    // Para lookup usamos o char como veio (já lower se aplicável).
    if c.is_ascii() {
        KeyCode::Char(c.to_ascii_lowercase())
    } else {
        KeyCode::Char(c)
    }
}

/// `Ctrl+A` = 1 … `Ctrl+Z` = 26 → letra minúscula.
fn control_char_to_letter(c: char) -> Option<char> {
    let b = c as u32;
    if (1..=26).contains(&b) {
        Some((b'a' + (b as u8 - 1)) as char)
    } else {
        None
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
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "`" | "grave" | "backtick" => KeyCode::Char('`'),
        "\\" | "backslash" => KeyCode::Char('\\'),
        "/" | "slash" => KeyCode::Char('/'),
        "\"" | "quote" | "doublequote" | "dquote" => KeyCode::Char('"'),
        "'" | "apostrophe" | "squote" => KeyCode::Char('\''),
        "?" | "question" | "questionmark" => KeyCode::Char('?'),
        "=" | "equals" | "equal" => KeyCode::Char('='),
        "-" | "minus" | "dash" | "hyphen" => KeyCode::Char('-'),
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
        KeyCode::F(n) => format!("f{n}"),
        KeyCode::Char(' ') => "space".into(),
        KeyCode::Char('`') => "`".into(),
        KeyCode::Char('\\') => "\\".into(),
        KeyCode::Char('"') => "\"".into(),
        KeyCode::Char('\'') => "'".into(),
        KeyCode::Char('?') => "?".into(),
        KeyCode::Char(c) => c.to_ascii_lowercase().to_string(),
        _ => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

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
        for s in [
            "esc",
            "ctrl+z",
            "pageup",
            "shift+home",
            "ctrl+shift+s",
            "f12",
        ] {
            let c = parse_chord(s).unwrap();
            assert_eq!(c.canonical_string(), s);
        }
    }

    #[test]
    fn ctrl_shift_s_with_shift_modifier() {
        let c = KeyChord::from_event(key(
            KeyCode::Char('s'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        assert_eq!(c.canonical_string(), "ctrl+shift+s");
    }

    #[test]
    fn ctrl_shift_s_as_uppercase_s() {
        // Terminal envia 'S' + CONTROL sem bit SHIFT
        let c = KeyChord::from_event(key(KeyCode::Char('S'), KeyModifiers::CONTROL));
        assert_eq!(c.canonical_string(), "ctrl+shift+s");
    }

    #[test]
    fn ctrl_s_legacy_control_char() {
        // ASCII 0x13 = Ctrl+S
        let c = KeyChord::from_event(key(KeyCode::Char('\u{13}'), KeyModifiers::empty()));
        assert!(c.ctrl);
        assert!(!c.shift);
        assert_eq!(c.code, KeyCode::Char('s'));
        assert_eq!(c.canonical_string(), "ctrl+s");
    }

    #[test]
    fn ctrl_s_plain_lowercase() {
        let c = KeyChord::from_event(key(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(c.canonical_string(), "ctrl+s");
    }
}
