//! Chords de teclado e mapa chord → action.

mod action;
mod chord;
mod map;

pub use action::{parse_action, Action, ActionParseError};
pub use chord::{parse_chord, KeyChord, ParseChordError};
pub use map::{Keymap, KeymapError, ResolvedKey};
