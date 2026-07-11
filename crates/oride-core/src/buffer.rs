//! Buffer de texto baseado em rope (`ropey`).

use ropey::Rope;
use thiserror::Error;

use crate::position::{ByteOffset, Caret};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BufferError {
    #[error("byte offset {offset} out of bounds (len {len})")]
    OffsetOutOfBounds { offset: usize, len: usize },
    #[error("byte offset {offset} is not on a char boundary")]
    NotCharBoundary { offset: usize },
    #[error("line {line} out of bounds ({line_count} lines)")]
    LineOutOfBounds { line: usize, line_count: usize },
}

/// Texto editável com acesso eficiente por linha e por offset de byte.
#[derive(Debug, Clone)]
pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    #[must_use]
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        // ropey: arquivo vazio tem 1 linha "vazia" se modelarmos como editores
        // comuns (uma linha). ropey com rope vazio tem 1 linha.
        self.rope.len_lines()
    }

    #[must_use]
    pub fn as_string(&self) -> String {
        self.rope.to_string()
    }

    /// Texto da linha `line` **sem** o `\n` final (se houver).
    pub fn line_text(&self, line: usize) -> Result<String, BufferError> {
        self.ensure_line(line)?;
        let owned = self.rope.line(line).to_string();
        Ok(line_content_without_eol(&owned).to_string())
    }

    /// Byte offset do início da linha (0-based).
    pub fn line_to_byte(&self, line: usize) -> Result<ByteOffset, BufferError> {
        self.ensure_line(line)?;
        Ok(ByteOffset::new(self.rope.line_to_byte(line)))
    }

    pub fn byte_to_caret(&self, offset: ByteOffset) -> Result<Caret, BufferError> {
        self.ensure_offset_boundary(offset)?;
        let line = self.rope.byte_to_line(offset.as_usize());
        let line_start = self.rope.line_to_byte(line);
        let column_bytes = offset.as_usize().saturating_sub(line_start);
        let line_owned = self.rope.line(line).to_string();
        let content = line_content_without_eol(&line_owned);
        let column_bytes = column_bytes.min(content.len());
        // Garante boundary dentro do conteúdo da linha
        let mut col_end = column_bytes;
        while col_end > 0 && !content.is_char_boundary(col_end) {
            col_end -= 1;
        }
        let column = content[..col_end].chars().count();
        Ok(Caret::new(line, column))
    }

    pub fn caret_to_byte(&self, caret: Caret) -> Result<ByteOffset, BufferError> {
        self.ensure_line(caret.line)?;
        let line_start = self.rope.line_to_byte(caret.line);
        let line_owned = self.rope.line(caret.line).to_string();
        let content = line_content_without_eol(&line_owned);
        let col = caret.column.min(content.chars().count());
        let byte_rel: usize = content.chars().take(col).map(|c| c.len_utf8()).sum();
        Ok(ByteOffset::new(line_start + byte_rel))
    }

    /// Offset do início do caractere UTF-8 anterior a `offset` (ou 0).
    pub fn prev_char_offset(&self, offset: ByteOffset) -> Result<ByteOffset, BufferError> {
        self.ensure_offset_boundary(offset)?;
        if offset.as_usize() == 0 {
            return Ok(offset);
        }
        let char_idx = self.rope.byte_to_char(offset.as_usize());
        Ok(ByteOffset::new(self.rope.char_to_byte(char_idx - 1)))
    }

    /// Offset do início do próximo caractere UTF-8 (ou fim do buffer).
    pub fn next_char_offset(&self, offset: ByteOffset) -> Result<ByteOffset, BufferError> {
        self.ensure_offset_boundary(offset)?;
        if offset.as_usize() >= self.len_bytes() {
            return Ok(offset);
        }
        let char_idx = self.rope.byte_to_char(offset.as_usize());
        let next = (char_idx + 1).min(self.rope.len_chars());
        Ok(ByteOffset::new(self.rope.char_to_byte(next)))
    }

    /// Insere `text` em `at`.
    pub fn insert(&mut self, at: ByteOffset, text: &str) -> Result<(), BufferError> {
        self.ensure_offset_boundary(at)?;
        let char_idx = self.rope.byte_to_char(at.as_usize());
        self.rope.insert(char_idx, text);
        Ok(())
    }

    /// Texto no intervalo semi-aberto `[start, end)` em bytes.
    pub fn text_range(&self, start: ByteOffset, end: ByteOffset) -> Result<String, BufferError> {
        if start > end {
            return Err(BufferError::OffsetOutOfBounds {
                offset: start.as_usize(),
                len: self.len_bytes(),
            });
        }
        self.ensure_offset_boundary(start)?;
        self.ensure_offset_boundary(end)?;
        let start_char = self.rope.byte_to_char(start.as_usize());
        let end_char = self.rope.byte_to_char(end.as_usize());
        Ok(self.rope.slice(start_char..end_char).to_string())
    }

    /// Remove o intervalo semi-aberto `[start, end)` em bytes.
    /// Retorna o texto removido.
    pub fn delete_range(
        &mut self,
        start: ByteOffset,
        end: ByteOffset,
    ) -> Result<String, BufferError> {
        if start > end {
            return Err(BufferError::OffsetOutOfBounds {
                offset: start.as_usize(),
                len: self.len_bytes(),
            });
        }
        self.ensure_offset_boundary(start)?;
        self.ensure_offset_boundary(end)?;
        let start_char = self.rope.byte_to_char(start.as_usize());
        let end_char = self.rope.byte_to_char(end.as_usize());
        let removed = self.rope.slice(start_char..end_char).to_string();
        self.rope.remove(start_char..end_char);
        Ok(removed)
    }

    fn ensure_offset_boundary(&self, offset: ByteOffset) -> Result<(), BufferError> {
        let o = offset.as_usize();
        let len = self.len_bytes();
        if o > len {
            return Err(BufferError::OffsetOutOfBounds { offset: o, len });
        }
        // ropey: byte_to_char exige boundary; offset == len é válido (EOF).
        if o < len && self.rope.try_byte_to_char(o).is_err() {
            return Err(BufferError::NotCharBoundary { offset: o });
        }
        Ok(())
    }

    fn ensure_line(&self, line: usize) -> Result<(), BufferError> {
        let line_count = self.line_count();
        if line >= line_count {
            return Err(BufferError::LineOutOfBounds { line, line_count });
        }
        Ok(())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

fn line_content_without_eol(line_with_eol: &str) -> &str {
    line_with_eol
        .strip_suffix("\r\n")
        .or_else(|| line_with_eol.strip_suffix('\n'))
        .or_else(|| line_with_eol.strip_suffix('\r'))
        .unwrap_or(line_with_eol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_has_one_line() {
        let buf = Buffer::new();
        assert_eq!(buf.line_count(), 1);
        assert!(buf.is_empty());
    }

    #[test]
    fn insert_and_read_lines() {
        let mut buf = Buffer::new();
        buf.insert(ByteOffset(0), "hello\nworld").unwrap();
        assert_eq!(buf.line_text(0).unwrap(), "hello");
        assert_eq!(buf.line_text(1).unwrap(), "world");
        assert_eq!(buf.as_string(), "hello\nworld");
    }

    #[test]
    fn delete_range_middle() {
        let mut buf = Buffer::from_text("abcdef");
        let removed = buf.delete_range(ByteOffset(2), ByteOffset(4)).unwrap();
        assert_eq!(removed, "cd");
        assert_eq!(buf.as_string(), "abef");
    }

    #[test]
    fn caret_roundtrip_ascii() {
        let buf = Buffer::from_text("ab\ncd");
        let off = buf.caret_to_byte(Caret::new(1, 1)).unwrap();
        assert_eq!(off, ByteOffset(4)); // "ab\n" + "c"
        let caret = buf.byte_to_caret(off).unwrap();
        assert_eq!(caret, Caret::new(1, 1));
    }

    #[test]
    fn rejects_offset_past_end() {
        let buf = Buffer::from_text("hi");
        let err = buf.byte_to_caret(ByteOffset(99)).unwrap_err();
        assert!(matches!(err, BufferError::OffsetOutOfBounds { .. }));
    }
}
