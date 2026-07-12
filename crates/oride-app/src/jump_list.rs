//! Jump list (últimas posições de navegação).

use std::path::PathBuf;

use oride_core::ByteOffset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jump {
    pub path: Option<PathBuf>,
    pub byte: ByteOffset,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct JumpList {
    stack: Vec<Jump>,
    /// Índice no stack (para forward).
    index: usize,
}

impl JumpList {
    pub fn push(&mut self, jump: Jump) {
        if let Some(last) = self.stack.get(self.index.saturating_sub(1)) {
            if last.path == jump.path && last.byte == jump.byte {
                return;
            }
        }
        if self.index < self.stack.len() {
            self.stack.truncate(self.index);
        }
        self.stack.push(jump);
        if self.stack.len() > 64 {
            self.stack.remove(0);
        }
        self.index = self.stack.len();
    }

    pub fn back(&mut self) -> Option<Jump> {
        if self.index == 0 || self.stack.is_empty() {
            return None;
        }
        // primeiro back: se index == len, recua para len-2 (pula posição atual se já pushada)
        if self.index >= self.stack.len() {
            self.index = self.stack.len().saturating_sub(1);
        }
        if self.index == 0 {
            return self.stack.first().cloned();
        }
        self.index -= 1;
        self.stack.get(self.index).cloned()
    }

    pub fn forward(&mut self) -> Option<Jump> {
        if self.index + 1 >= self.stack.len() {
            return None;
        }
        self.index += 1;
        self.stack.get(self.index).cloned()
    }
}
