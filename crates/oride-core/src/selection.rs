//! Seleção de texto: âncora + head (caret ativo).

use crate::position::ByteOffset;

/// Intervalo orientado: `anchor` é onde a seleção começou; `head` é o cursor.
///
/// Quando `anchor == head`, a seleção é um caret (sem intervalo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: ByteOffset,
    pub head: ByteOffset,
}

impl Selection {
    #[must_use]
    pub const fn caret(at: ByteOffset) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    #[must_use]
    pub const fn new(anchor: ByteOffset, head: ByteOffset) -> Self {
        Self { anchor, head }
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }

    /// Início do intervalo normalizado (min).
    #[must_use]
    pub fn start(self) -> ByteOffset {
        if self.anchor.as_usize() <= self.head.as_usize() {
            self.anchor
        } else {
            self.head
        }
    }

    /// Fim do intervalo normalizado (max).
    #[must_use]
    pub fn end(self) -> ByteOffset {
        if self.anchor.as_usize() <= self.head.as_usize() {
            self.head
        } else {
            self.anchor
        }
    }

    /// Move o head (e opcionalmente a âncora se não estiver estendendo).
    #[must_use]
    pub fn move_head(self, head: ByteOffset, extend: bool) -> Self {
        if extend {
            Self {
                anchor: self.anchor,
                head,
            }
        } else {
            Self::caret(head)
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::caret(ByteOffset(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_range_when_reversed() {
        let sel = Selection::new(ByteOffset(10), ByteOffset(3));
        assert_eq!(sel.start(), ByteOffset(3));
        assert_eq!(sel.end(), ByteOffset(10));
        assert!(!sel.is_empty());
    }
}
