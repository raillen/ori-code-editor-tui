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

        // Digitação: sem ctrl/alt
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
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
    fn typing_falls_through() {
        let map = Keymap::from_string_map([("ctrl+s", "save")]).unwrap();
        let r = map
            .resolve_event(key(KeyCode::Char('x'), KeyModifiers::NONE))
            .unwrap();
        assert_eq!(r, ResolvedKey::InsertChar('x'));
    }
}
