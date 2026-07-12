//! Busca no buffer ativo — case, acentos, replace e replace-all.

/// Uma ocorrência: intervalo em bytes no texto original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct FindState {
    pub query: String,
    pub replace: String,
    pub matches: Vec<MatchRange>,
    pub current: usize,
    pub case_sensitive: bool,
    /// Ignora acentos (á≈a, ç≈c, …) na busca.
    pub ignore_accents: bool,
    /// Mostra campo de replace na barra compacta.
    pub show_replace: bool,
    /// true = editando replace; false = editando query.
    pub focus_replace: bool,
}

impl Default for FindState {
    fn default() -> Self {
        Self {
            query: String::new(),
            replace: String::new(),
            matches: Vec::new(),
            current: 0,
            case_sensitive: false,
            ignore_accents: true,
            show_replace: false,
            focus_replace: false,
        }
    }
}

impl FindState {
    pub fn recompute(&mut self, haystack: &str) {
        self.matches = find_all(
            haystack,
            &self.query,
            self.case_sensitive,
            self.ignore_accents,
        );
        if self.matches.is_empty() {
            self.current = 0;
        } else {
            self.current = self.current.min(self.matches.len() - 1);
        }
    }

    #[must_use]
    pub fn current_match(&self) -> Option<MatchRange> {
        self.matches.get(self.current).copied()
    }

    pub fn next(&mut self) -> Option<MatchRange> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = (self.current + 1) % self.matches.len();
        self.current_match()
    }

    pub fn prev(&mut self) -> Option<MatchRange> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current == 0 {
            self.current = self.matches.len() - 1;
        } else {
            self.current -= 1;
        }
        self.current_match()
    }

    pub fn toggle_case(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn toggle_accents(&mut self) {
        self.ignore_accents = !self.ignore_accents;
    }

    #[must_use]
    pub fn status(&self) -> String {
        let flags = format!(
            "case:{} accent:{}",
            if self.case_sensitive { "on" } else { "off" },
            if self.ignore_accents { "ign" } else { "on" }
        );
        if self.query.is_empty() {
            return format!(
                "find · {flags} · digite · Alt+C case · Alt+A acentos · Ctrl+H replace"
            );
        }
        if self.matches.is_empty() {
            return format!("find: \"{}\" — 0 · {flags}", self.query);
        }
        format!(
            "find: \"{}\" — {}/{} · {flags}",
            self.query,
            self.current + 1,
            self.matches.len()
        )
    }

    #[must_use]
    pub fn options_label(&self) -> String {
        format!(
            "[{}] case  [{}] acentos  · Enter próximo · Tab campo · Alt+Enter 1× · Ctrl+Alt+Enter all · Esc",
            if self.case_sensitive { "x" } else { " " },
            if self.ignore_accents { "x" } else { " " }
        )
    }
}

/// Normaliza para comparação: lower opcional + strip de acentos opcional.
fn fold_char(c: char, case_sensitive: bool, ignore_accents: bool) -> char {
    let c = if case_sensitive {
        c
    } else {
        c.to_lowercase().next().unwrap_or(c)
    };
    if !ignore_accents {
        return c;
    }
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

fn fold_string(s: &str, case_sensitive: bool, ignore_accents: bool) -> String {
    s.chars()
        .map(|c| fold_char(c, case_sensitive, ignore_accents))
        .collect()
}

/// Busca todas as ocorrências com offsets no **texto original**.
pub fn find_all(
    haystack: &str,
    query: &str,
    case_sensitive: bool,
    ignore_accents: bool,
) -> Vec<MatchRange> {
    if query.is_empty() {
        return Vec::new();
    }
    let folded_q = fold_string(query, case_sensitive, ignore_accents);
    if folded_q.is_empty() {
        return Vec::new();
    }
    let q_chars: Vec<char> = folded_q.chars().collect();
    let q_len = q_chars.len();

    // Mapear cada char folded → (byte_start, byte_end) no original
    let mut orig: Vec<(usize, usize, char)> = Vec::new();
    for (byte_idx, ch) in haystack.char_indices() {
        let end = byte_idx + ch.len_utf8();
        let f = fold_char(ch, case_sensitive, ignore_accents);
        orig.push((byte_idx, end, f));
    }

    let mut out = Vec::new();
    let n = orig.len();
    if n < q_len {
        return out;
    }
    let mut i = 0;
    while i + q_len <= n {
        let mut ok = true;
        for (k, qc) in q_chars.iter().enumerate() {
            if orig[i + k].2 != *qc {
                ok = false;
                break;
            }
        }
        if ok {
            let start = orig[i].0;
            let end = orig[i + q_len - 1].1;
            out.push(MatchRange { start, end });
            i += q_len.max(1);
        } else {
            i += 1;
        }
    }
    out
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
        f.case_sensitive = false;
        f.ignore_accents = false;
        f.recompute("ab x AB y ab");
        assert_eq!(f.matches.len(), 3);
        assert_eq!(f.current_match().unwrap().start, 0);
        assert_eq!(f.next().unwrap().start, 5);
    }

    #[test]
    fn finds_ignore_accents() {
        let ranges = find_all("ação e acao", "acao", false, true);
        assert_eq!(ranges.len(), 2);
        assert_eq!(&"ação e acao"[ranges[0].start..ranges[0].end], "ação");
    }

    #[test]
    fn case_sensitive_skips() {
        let ranges = find_all("Ab ab AB", "ab", true, false);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 3);
    }
}
