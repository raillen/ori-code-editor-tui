//! Busca no buffer ativo (case-insensitive).

use oride_core::ByteOffset;

#[derive(Debug, Clone, Default)]
pub struct FindState {
    pub query: String,
    /// Inícios das ocorrências em bytes.
    pub matches: Vec<usize>,
    pub current: usize,
}

impl FindState {
    pub fn recompute(&mut self, haystack: &str) {
        self.matches.clear();
        self.current = 0;
        if self.query.is_empty() {
            return;
        }
        let h = haystack.to_ascii_lowercase();
        let q = self.query.to_ascii_lowercase();
        let mut start = 0usize;
        while let Some(rel) = h[start..].find(&q) {
            let abs = start + rel;
            self.matches.push(abs);
            start = abs + q.len().max(1);
            if start >= h.len() {
                break;
            }
        }
    }

    #[must_use]
    pub fn current_byte(&self) -> Option<ByteOffset> {
        self.matches.get(self.current).map(|b| ByteOffset::new(*b))
    }

    pub fn next(&mut self) -> Option<ByteOffset> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = (self.current + 1) % self.matches.len();
        self.current_byte()
    }

    pub fn prev(&mut self) -> Option<ByteOffset> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current == 0 {
            self.current = self.matches.len() - 1;
        } else {
            self.current -= 1;
        }
        self.current_byte()
    }

    #[must_use]
    pub fn status(&self) -> String {
        if self.query.is_empty() {
            return "find: (digite)".into();
        }
        if self.matches.is_empty() {
            return format!("find: \"{}\" — 0 ocorrências", self.query);
        }
        format!(
            "find: \"{}\" — {}/{}",
            self.query,
            self.current + 1,
            self.matches.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_all_case_insensitive() {
        let mut f = FindState {
            query: "Ab".into(),
            ..Default::default()
        };
        f.recompute("ab x AB y ab");
        assert_eq!(f.matches.len(), 3);
        assert_eq!(f.current_byte().unwrap().as_usize(), 0);
        assert_eq!(f.next().unwrap().as_usize(), 5);
    }
}
