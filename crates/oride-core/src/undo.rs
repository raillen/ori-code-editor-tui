//! Pilha de undo/redo com edits inversos.

use crate::buffer::Buffer;
use crate::position::ByteOffset;

/// Uma operação atômica aplicada (ou invertida) no buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Texto inserido em `at` (para desfazer: apagar esse trecho).
    Insert { at: ByteOffset, text: String },
    /// Texto removido de `at` (para desfazer: reinserir).
    Delete { at: ByteOffset, text: String },
}

impl Edit {
    /// Aplica o edit ao buffer.
    pub fn apply(&self, buffer: &mut Buffer) {
        match self {
            Edit::Insert { at, text } => {
                buffer
                    .insert(*at, text)
                    .expect("undo insert offset must be valid");
            }
            Edit::Delete { at, text } => {
                let end = ByteOffset::new(at.as_usize() + text.len());
                buffer
                    .delete_range(*at, end)
                    .expect("undo delete range must be valid");
            }
        }
    }

    /// Produz o edit inverso (undo de `self`).
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Edit::Insert { at, text } => Edit::Delete {
                at: *at,
                text: text.clone(),
            },
            Edit::Delete { at, text } => Edit::Insert {
                at: *at,
                text: text.clone(),
            },
        }
    }
}

/// Grupo de edits (ex.: uma digitação coalescida ou um paste).
#[derive(Debug, Clone, Default)]
pub struct EditGroup {
    edits: Vec<Edit>,
}

impl EditGroup {
    #[must_use]
    pub fn new() -> Self {
        Self { edits: Vec::new() }
    }

    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    fn apply_forward(&self, buffer: &mut Buffer) {
        for edit in &self.edits {
            edit.apply(buffer);
        }
    }

    fn apply_inverse(&self, buffer: &mut Buffer) {
        for edit in self.edits.iter().rev() {
            edit.inverse().apply(buffer);
        }
    }
}

/// Pilha clássica undo / redo. Um `push` limpa o ramo de redo.
#[derive(Debug, Default)]
pub struct UndoStack {
    undo: Vec<EditGroup>,
    redo: Vec<EditGroup>,
    /// Grupo aberto para coalescer inserts contíguos (digitação).
    open: Option<EditGroup>,
}

impl UndoStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um edit já aplicado no buffer e limpa redo.
    pub fn push_applied(&mut self, edit: Edit) {
        self.redo.clear();
        match &mut self.open {
            Some(group) => group.push(edit),
            None => {
                let mut group = EditGroup::new();
                group.push(edit);
                self.open = Some(group);
            }
        }
    }

    /// Fecha o grupo aberto (boundary: seta, save, blur, etc.).
    pub fn commit_group(&mut self) {
        if let Some(group) = self.open.take() {
            if !group.is_empty() {
                self.undo.push(group);
            }
        }
    }

    /// Desfaz o último grupo. Retorna `true` se houve o que desfazer.
    pub fn undo(&mut self, buffer: &mut Buffer) -> bool {
        self.commit_group();
        let Some(group) = self.undo.pop() else {
            return false;
        };
        group.apply_inverse(buffer);
        self.redo.push(group);
        true
    }

    /// Refaz o último grupo desfeito.
    pub fn redo(&mut self, buffer: &mut Buffer) -> bool {
        self.commit_group();
        let Some(group) = self.redo.pop() else {
            return false;
        };
        group.apply_forward(buffer);
        self.undo.push(group);
        true
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.open.as_ref().is_some_and(|g| !g.is_empty()) || !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_insert() {
        let mut buf = Buffer::from_text("");
        let mut stack = UndoStack::new();
        buf.insert(ByteOffset(0), "hi").unwrap();
        stack.push_applied(Edit::Insert {
            at: ByteOffset(0),
            text: "hi".into(),
        });
        stack.commit_group();
        assert!(stack.undo(&mut buf));
        assert_eq!(buf.as_string(), "");
        assert!(stack.redo(&mut buf));
        assert_eq!(buf.as_string(), "hi");
    }

    #[test]
    fn undo_delete() {
        let mut buf = Buffer::from_text("abcd");
        let mut stack = UndoStack::new();
        let removed = buf.delete_range(ByteOffset(1), ByteOffset(3)).unwrap();
        stack.push_applied(Edit::Delete {
            at: ByteOffset(1),
            text: removed,
        });
        stack.commit_group();
        assert!(stack.undo(&mut buf));
        assert_eq!(buf.as_string(), "abcd");
    }
}
