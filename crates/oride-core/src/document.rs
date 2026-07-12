//! Documento (buffer + path + seleção + undo) e store multi-tab.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::buffer::{Buffer, BufferError};
use crate::position::{ByteOffset, Caret};
use crate::selection::Selection;
use crate::undo::{Edit, UndoStack};

/// Identificador opaco de documento aberto (tab).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DocumentId(u64);

impl DocumentId {
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Resumo de uma tab para a UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabSummary {
    pub id: DocumentId,
    pub title: String,
    pub dirty: bool,
    pub active: bool,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error(transparent)]
    Buffer(#[from] BufferError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("document {0:?} not found")]
    NotFound(DocumentId),
    #[error("no active document")]
    NoActiveDocument,
}

/// Um arquivo (ou buffer sem path) aberto no editor.
#[derive(Debug)]
pub struct Document {
    id: DocumentId,
    path: Option<PathBuf>,
    buffer: Buffer,
    selection: Selection,
    undo: UndoStack,
    dirty: bool,
    /// Coluna preferida ao mover ↑/↓ (estilo editores clássicos).
    preferred_column: Option<usize>,
}

impl Document {
    #[must_use]
    pub fn id(&self) -> DocumentId {
        self.id
    }

    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    #[must_use]
    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = selection;
        self.preferred_column = None;
        self.undo.commit_group();
    }

    /// Caret atual (linha/coluna).
    pub fn caret(&self) -> Result<Caret, DocumentError> {
        Ok(self.buffer.byte_to_caret(self.selection.head)?)
    }

    fn move_head_to(&mut self, head: ByteOffset, extend: bool) {
        self.selection = self.selection.move_head(head, extend);
        self.undo.commit_group();
    }

    /// Move o caret um caractere à esquerda.
    pub fn move_left(&mut self, extend: bool) -> Result<(), DocumentError> {
        let head = self.buffer.prev_char_offset(self.selection.head)?;
        self.preferred_column = None;
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Move o caret um caractere à direita.
    pub fn move_right(&mut self, extend: bool) -> Result<(), DocumentError> {
        let head = self.buffer.next_char_offset(self.selection.head)?;
        self.preferred_column = None;
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Move o caret uma linha acima, preservando coluna preferida.
    pub fn move_up(&mut self, extend: bool) -> Result<(), DocumentError> {
        let caret = self.buffer.byte_to_caret(self.selection.head)?;
        if caret.line == 0 {
            return Ok(());
        }
        let col = self.preferred_column.unwrap_or(caret.column);
        self.preferred_column = Some(col);
        let target = Caret::new(caret.line - 1, col);
        let head = self.buffer.caret_to_byte(target)?;
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Move o caret uma linha abaixo, preservando coluna preferida.
    pub fn move_down(&mut self, extend: bool) -> Result<(), DocumentError> {
        let caret = self.buffer.byte_to_caret(self.selection.head)?;
        let last_line = self.buffer.line_count().saturating_sub(1);
        if caret.line >= last_line {
            return Ok(());
        }
        let col = self.preferred_column.unwrap_or(caret.column);
        self.preferred_column = Some(col);
        let target = Caret::new(caret.line + 1, col);
        let head = self.buffer.caret_to_byte(target)?;
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Home da linha atual.
    pub fn move_line_start(&mut self, extend: bool) -> Result<(), DocumentError> {
        let caret = self.buffer.byte_to_caret(self.selection.head)?;
        let head = self.buffer.caret_to_byte(Caret::new(caret.line, 0))?;
        self.preferred_column = Some(0);
        self.move_head_to(head, extend);
        Ok(())
    }

    /// End da linha atual.
    pub fn move_line_end(&mut self, extend: bool) -> Result<(), DocumentError> {
        let caret = self.buffer.byte_to_caret(self.selection.head)?;
        let line = self.buffer.line_text(caret.line)?;
        let col = line.chars().count();
        let head = self.buffer.caret_to_byte(Caret::new(caret.line, col))?;
        self.preferred_column = Some(col);
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Início do buffer (Ctrl+Home).
    pub fn move_buffer_start(&mut self, extend: bool) -> Result<(), DocumentError> {
        self.preferred_column = Some(0);
        self.move_head_to(ByteOffset::new(0), extend);
        Ok(())
    }

    /// Fim do buffer (Ctrl+End).
    pub fn move_buffer_end(&mut self, extend: bool) -> Result<(), DocumentError> {
        let head = ByteOffset::new(self.buffer.len_bytes());
        self.preferred_column = None;
        self.move_head_to(head, extend);
        Ok(())
    }

    /// Seleciona todo o documento (Ctrl+A).
    pub fn select_all(&mut self) {
        let end = ByteOffset::new(self.buffer.len_bytes());
        self.selection = Selection::new(ByteOffset::new(0), end);
        self.preferred_column = None;
        self.undo.commit_group();
    }

    /// Título para tab: nome do arquivo ou "untitled".
    #[must_use]
    pub fn tab_title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => "untitled".into(),
        }
    }

    /// Insere texto na posição do caret (substitui seleção se houver).
    pub fn insert_text(&mut self, text: &str) -> Result<(), DocumentError> {
        if !self.selection.is_empty() {
            self.delete_selection()?;
        }
        let at = self.selection.head;
        self.buffer.insert(at, text)?;
        self.undo.push_applied(Edit::Insert {
            at,
            text: text.to_string(),
        });
        let new_head = ByteOffset::new(at.as_usize() + text.len());
        self.selection = Selection::caret(new_head);
        self.dirty = true;
        Ok(())
    }

    /// Apaga a seleção atual, ou o caractere anterior se vazia (backspace).
    pub fn backspace(&mut self) -> Result<(), DocumentError> {
        if !self.selection.is_empty() {
            return self.delete_selection();
        }
        let end = self.selection.head;
        if end.as_usize() == 0 {
            return Ok(());
        }
        let start = self.buffer.prev_char_offset(end)?;
        let removed = self.buffer.delete_range(start, end)?;
        self.undo.push_applied(Edit::Delete {
            at: start,
            text: removed,
        });
        self.selection = Selection::caret(start);
        self.preferred_column = None;
        self.dirty = true;
        Ok(())
    }

    /// Delete à frente do caret (ou a seleção).
    pub fn delete_forward(&mut self) -> Result<(), DocumentError> {
        if !self.selection.is_empty() {
            return self.delete_selection();
        }
        let start = self.selection.head;
        if start.as_usize() >= self.buffer.len_bytes() {
            return Ok(());
        }
        let end = self.buffer.next_char_offset(start)?;
        let removed = self.buffer.delete_range(start, end)?;
        self.undo.push_applied(Edit::Delete {
            at: start,
            text: removed,
        });
        self.preferred_column = None;
        self.dirty = true;
        Ok(())
    }

    pub fn delete_selection(&mut self) -> Result<(), DocumentError> {
        if self.selection.is_empty() {
            return Ok(());
        }
        let start = self.selection.start();
        let end = self.selection.end();
        let removed = self.buffer.delete_range(start, end)?;
        self.undo.push_applied(Edit::Delete {
            at: start,
            text: removed,
        });
        self.selection = Selection::caret(start);
        self.dirty = true;
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let changed = self.undo.undo(&mut self.buffer);
        if changed {
            self.dirty = true;
            let len = self.buffer.len_bytes();
            let head = self.selection.head.as_usize().min(len);
            self.selection = Selection::caret(ByteOffset::new(head));
        }
        changed
    }

    pub fn redo(&mut self) -> bool {
        let changed = self.redo_inner();
        if changed {
            self.dirty = true;
            let len = self.buffer.len_bytes();
            let head = self.selection.head.as_usize().min(len);
            self.selection = Selection::caret(ByteOffset::new(head));
        }
        changed
    }

    fn redo_inner(&mut self) -> bool {
        self.undo.redo(&mut self.buffer)
    }

    pub fn commit_edit_group(&mut self) {
        self.undo.commit_group();
    }

    /// Marca limpo após save bem-sucedido.
    pub fn mark_saved(&mut self) {
        self.undo.commit_group();
        self.dirty = false;
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    /// Serializa o buffer para disco no path atual ou `path` fornecido.
    pub fn save_to(&mut self, path: Option<&Path>) -> Result<(), DocumentError> {
        let target = match path {
            Some(p) => p.to_path_buf(),
            None => self.path.clone().ok_or_else(|| {
                DocumentError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "document has no path; provide one to save",
                ))
            })?,
        };
        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(&target, self.buffer.as_string())?;
        self.path = Some(target);
        self.mark_saved();
        Ok(())
    }

    /// Texto da seleção (vazio se caret).
    #[must_use]
    pub fn selected_text(&self) -> String {
        if self.selection.is_empty() {
            return String::new();
        }
        self.buffer
            .text_range(self.selection.start(), self.selection.end())
            .unwrap_or_default()
    }

    /// Move o caret para `byte` (seleção colapsada).
    pub fn jump_to_byte(&mut self, byte: ByteOffset) {
        let len = self.buffer.len_bytes();
        let b = ByteOffset::new(byte.as_usize().min(len));
        self.selection = Selection::caret(b);
        self.preferred_column = None;
        self.undo.commit_group();
    }

    /// Seleciona o intervalo `[start, end)` em bytes e coloca o head no fim.
    pub fn select_byte_range(&mut self, start: ByteOffset, end: ByteOffset) {
        let len = self.buffer.len_bytes();
        let s = ByteOffset::new(start.as_usize().min(len));
        let e = ByteOffset::new(end.as_usize().min(len));
        self.selection = Selection::new(s, e);
        self.preferred_column = None;
        self.undo.commit_group();
    }
}

/// Coleção de documentos abertos + tab ativa.
#[derive(Debug, Default)]
pub struct DocumentStore {
    next_id: u64,
    docs: Vec<Document>,
    active: Option<DocumentId>,
}

impl DocumentStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_id(&mut self) -> DocumentId {
        let id = DocumentId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Buffer vazio sem path.
    pub fn open_empty(&mut self) -> DocumentId {
        let id = self.alloc_id();
        self.docs.push(Document {
            id,
            path: None,
            buffer: Buffer::new(),
            selection: Selection::default(),
            undo: UndoStack::new(),
            dirty: false,
            preferred_column: None,
        });
        self.active = Some(id);
        id
    }

    /// Abre arquivo do disco (UTF-8). Se já estiver aberto, só ativa a tab.
    pub fn open_path(&mut self, path: impl AsRef<Path>) -> Result<DocumentId, DocumentError> {
        let path = path.as_ref();
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(existing) = self.docs.iter().find(|d| {
            d.path
                .as_ref()
                .is_some_and(|p| p == &canonical || p == path)
        }) {
            let id = existing.id;
            self.active = Some(id);
            return Ok(id);
        }
        let text = std::fs::read_to_string(path)?;
        let id = self.alloc_id();
        self.docs.push(Document {
            id,
            path: Some(canonical),
            buffer: Buffer::from_text(&text),
            selection: Selection::default(),
            undo: UndoStack::new(),
            dirty: false,
            preferred_column: None,
        });
        self.active = Some(id);
        Ok(id)
    }

    /// Tab seguinte (cíclico).
    pub fn activate_next_tab(&mut self) -> Option<DocumentId> {
        let ids = self.tab_ids();
        if ids.is_empty() {
            return None;
        }
        let cur = self.active?;
        let idx = ids.iter().position(|id| *id == cur).unwrap_or(0);
        let next = ids[(idx + 1) % ids.len()];
        self.active = Some(next);
        Some(next)
    }

    /// Tab anterior (cíclico).
    pub fn activate_prev_tab(&mut self) -> Option<DocumentId> {
        let ids = self.tab_ids();
        if ids.is_empty() {
            return None;
        }
        let cur = self.active?;
        let idx = ids.iter().position(|id| *id == cur).unwrap_or(0);
        let prev = ids[(idx + ids.len() - 1) % ids.len()];
        self.active = Some(prev);
        Some(prev)
    }

    /// Metadados leves para a barra de tabs.
    #[must_use]
    pub fn tab_summaries(&self) -> Vec<TabSummary> {
        self.docs
            .iter()
            .map(|d| TabSummary {
                id: d.id,
                title: d.tab_title(),
                dirty: d.dirty,
                active: Some(d.id) == self.active,
            })
            .collect()
    }

    #[must_use]
    pub fn active_id(&self) -> Option<DocumentId> {
        self.active
    }

    pub fn set_active(&mut self, id: DocumentId) -> Result<(), DocumentError> {
        if !self.docs.iter().any(|d| d.id == id) {
            return Err(DocumentError::NotFound(id));
        }
        self.active = Some(id);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: DocumentId) -> Option<&Document> {
        self.docs.iter().find(|d| d.id == id)
    }

    pub fn get_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.docs.iter_mut().find(|d| d.id == id)
    }

    pub fn active(&self) -> Result<&Document, DocumentError> {
        let id = self.active.ok_or(DocumentError::NoActiveDocument)?;
        self.get(id).ok_or(DocumentError::NotFound(id))
    }

    pub fn active_mut(&mut self) -> Result<&mut Document, DocumentError> {
        let id = self.active.ok_or(DocumentError::NoActiveDocument)?;
        // borrow checker: find index
        let idx = self
            .docs
            .iter()
            .position(|d| d.id == id)
            .ok_or(DocumentError::NotFound(id))?;
        Ok(&mut self.docs[idx])
    }

    /// Ordem das tabs.
    #[must_use]
    pub fn tab_ids(&self) -> Vec<DocumentId> {
        self.docs.iter().map(|d| d.id).collect()
    }

    /// Fecha tab. Retorna se o doc estava dirty (chamador deve confirmar).
    pub fn close(&mut self, id: DocumentId) -> Result<bool, DocumentError> {
        let idx = self
            .docs
            .iter()
            .position(|d| d.id == id)
            .ok_or(DocumentError::NotFound(id))?;
        let dirty = self.docs[idx].dirty;
        self.docs.remove(idx);
        if self.active == Some(id) {
            self.active = self.docs.last().map(|d| d.id);
        }
        Ok(dirty)
    }

    /// Salva todos os documentos com path. Retorna (salvos, sem_path).
    pub fn save_all(&mut self) -> (usize, usize) {
        let mut saved = 0usize;
        let mut skipped = 0usize;
        let ids: Vec<_> = self.docs.iter().map(|d| d.id).collect();
        for id in ids {
            if let Some(doc) = self.get_mut(id) {
                if doc.path().is_none() {
                    skipped += 1;
                    continue;
                }
                if doc.save_to(None).is_ok() {
                    saved += 1;
                }
            }
        }
        (saved, skipped)
    }

    /// Paths de abas abertas (ordem das tabs).
    #[must_use]
    pub fn open_paths(&self) -> Vec<PathBuf> {
        self.docs
            .iter()
            .filter_map(|d| d.path().map(Path::to_path_buf))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_undo_through_document() {
        let mut store = DocumentStore::new();
        let id = store.open_empty();
        {
            let doc = store.get_mut(id).unwrap();
            doc.insert_text("abc").unwrap();
            assert_eq!(doc.buffer().as_string(), "abc");
            assert!(doc.is_dirty());
            doc.commit_edit_group();
            assert!(doc.undo());
            assert_eq!(doc.buffer().as_string(), "");
        }
    }

    #[test]
    fn backspace_utf8() {
        let mut store = DocumentStore::new();
        let id = store.open_empty();
        let doc = store.get_mut(id).unwrap();
        doc.insert_text("a✨b").unwrap();
        doc.backspace().unwrap();
        assert_eq!(doc.buffer().as_string(), "a✨");
        doc.backspace().unwrap();
        assert_eq!(doc.buffer().as_string(), "a");
    }

    #[test]
    fn multi_tab_active() {
        let mut store = DocumentStore::new();
        let a = store.open_empty();
        let b = store.open_empty();
        assert_eq!(store.active_id(), Some(b));
        store.set_active(a).unwrap();
        assert_eq!(store.active_id(), Some(a));
        assert_eq!(store.tab_ids().len(), 2);
    }

    #[test]
    fn move_up_down_preserves_preferred_column() {
        let mut store = DocumentStore::new();
        let id = store.open_empty();
        let doc = store.get_mut(id).unwrap();
        doc.insert_text("hello\nxy\nworld").unwrap();
        doc.move_line_end(false).unwrap();
        assert_eq!(doc.caret().unwrap().column, 5);
        doc.move_up(false).unwrap();
        assert_eq!(doc.caret().unwrap(), Caret::new(1, 2)); // "xy" só tem 2 cols
        doc.move_up(false).unwrap();
        assert_eq!(doc.caret().unwrap(), Caret::new(0, 5));
    }
}
