//! Posições no buffer: offset em bytes UTF-8 e caret linha/coluna (0-based).

/// Offset em bytes UTF-8 a partir do início do buffer.
///
/// Newtype evita confusão com índices de char ou de linha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ByteOffset(pub usize);

impl ByteOffset {
    #[must_use]
    pub const fn new(bytes: usize) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

impl From<usize> for ByteOffset {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Cursor 0-based: linha e coluna em **caracteres** Unicode (não bytes, não colunas visuais).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Caret {
    pub line: usize,
    pub column: usize,
}

impl Caret {
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}
