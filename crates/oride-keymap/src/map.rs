//! Tabela chord → Action.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use thiserror::Error;

use crate::action::{parse_action, Action};
use crate::chord::{parse_chord, KeyChord};

#[derive(Debug, Error)]
pub enum KeymapError {
    #[error("chord `{chord}`: {source}")]
    Chord {
        chord: String,
        #[source]
        source: crate::chord::ParseChordError,
    },
    #[error("action for `{chord}`: {source}")]
    Action {
        chord: String,
        #[source]
        source: crate::action::ActionParseError,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Keymap {
    bindings: HashMap<KeyChord, Action>,
}

impl Keymap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Constrói a partir de pares chord_string → action_id (ex.: config TOML).
    pub fn from_string_map<'a, I>(pairs: I) -> Result<Self, KeymapError>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut map = Self::new();
        for (chord_s, action_s) in pairs {
            let chord = parse_chord(chord_s).map_err(|source| KeymapError::Chord {
                chord: chord_s.to_string(),
                source,
            })?;
            let action = parse_action(action_s).map_err(|source| KeymapError::Action {
                chord: chord_s.to_string(),
                source,
            })?;
            map.bindings.insert(chord, action);
        }
        Ok(map)
    }

    #[must_use]
    pub fn resolve_chord(&self, chord: KeyChord) -> Option<Action> {
        self.bindings.get(&chord).copied()
    }

    /// Resolve tecla: binding → senão insert de caractere imprimível.
    #[must_use]
    pub fn resolve_event(&self, key: KeyEvent) -> Option<ResolvedKey> {
        let chord = KeyChord::from_event(key);
        if let Some(action) = self.resolve_chord(chord) {
            return Some(ResolvedKey::Action(action));
        }

        // Fallback: alguns terminais mandam Ctrl+Shift+letra sem bit SHIFT,
        // mas com Char maiúsculo — from_event já cobre. Se ainda falhar e
        // for ctrl+letra, tenta a variante com shift forçado (ex.: save_as).
        if chord.ctrl && !chord.shift {
            if let KeyCode::Char(c) = chord.code {
                if c.is_ascii_lowercase() {
                    let with_shift = KeyChord {
                        ctrl: true,
                        alt: chord.alt,
                        shift: true,
                        code: KeyCode::Char(c),
                    };
                    // Só usa se existir binding *específico* com shift
                    // (não reescreve ctrl+s → save_as a menos que exista
                    // ctrl+shift+s E o evento original tenha indícios de shift
                    // — ver `event_suggests_shift`).
                    if event_suggests_shift(key) {
                        if let Some(action) = self.resolve_chord(with_shift) {
                            return Some(ResolvedKey::Action(action));
                        }
                    }
                }
            }
        }

        // Digitação: sem ctrl/alt
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL)
            || matches!(key.code, KeyCode::Char(c) if (c as u32) <= 26 && c as u32 >= 1);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if ctrl || alt {
            return None;
        }

        match key.code {
            KeyCode::Char(c) if !c.is_control() => Some(ResolvedKey::InsertChar(c)),
            _ => None,
        }
    }
}

/// Heurística: o evento parece incluir Shift além do CONTROL.
fn event_suggests_shift(key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        return true;
    }
    matches!(key.code, KeyCode::Char(c) if c.is_ascii_uppercase())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedKey {
    Action(Action),
    InsertChar(char),
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
    fn resolves_ctrl_s_save() {
        let map = Keymap::from_string_map([("ctrl+s", "save")]).unwrap();
        let r = map
            .resolve_event(key(KeyCode::Char('s'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(r, ResolvedKey::Action(Action::Save));
    }

    #[test]
    fn resolves_ctrl_shift_s_save_as() {
        let map = Keymap::from_string_map([
            ("ctrl+s", "save"),
            ("ctrl+shift+s", "save_as"),
            ("f12", "save_as"),
        ])
        .unwrap();

        let r = map
            .resolve_event(key(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ))
            .unwrap();
        assert_eq!(r, ResolvedKey::Action(Action::SaveAs));

        // Maiúsculo + Ctrl (sem bit SHIFT) — comum em TTY
        let r = map
            .resolve_event(key(KeyCode::Char('S'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(r, ResolvedKey::Action(Action::SaveAs));

        let r = map
            .resolve_event(key(KeyCode::F(12), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(r, ResolvedKey::Action(Action::SaveAs));
    }

    #[test]
    fn typing_falls_through() {
        let map = Keymap::from_string_map([("ctrl+s", "save")]).unwrap();
        let r = map
            .resolve_event(key(KeyCode::Char('x'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(r, ResolvedKey::InsertChar('x'));
    }
}
