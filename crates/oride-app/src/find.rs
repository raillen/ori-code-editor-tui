//! Busca no buffer ativo — case, acentos, regex, replace e replace-all.

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
    /// Ignora acentos (á≈a, ç≈c, …) na busca literal (não aplica em regex).
    pub ignore_accents: bool,
    /// Interpreta a query como regex Rust.
    pub use_regex: bool,
    /// Erro de compilação regex (status).
    pub regex_error: Option<String>,
    pub show_replace: bool,
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
            use_regex: false,
            regex_error: None,
            show_replace: false,
            focus_replace: false,
        }
    }
}

impl FindState {
    pub fn recompute(&mut self, haystack: &str) {
        self.regex_error = None;
        self.matches = if self.use_regex {
            match find_all_regex(haystack, &self.query, self.case_sensitive) {
                Ok(m) => m,
                Err(e) => {
                    self.regex_error = Some(e);
                    Vec::new()
                }
            }
        } else {
            find_all(
                haystack,
                &self.query,
                self.case_sensitive,
                self.ignore_accents,
            )
        };
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

    pub fn toggle_regex(&mut self) {
        self.use_regex = !self.use_regex;
    }

    #[must_use]
    pub fn status(&self) -> String {
        let flags = format!(
            "case:{} accent:{} re:{}",
            if self.case_sensitive { "on" } else { "off" },
            if self.ignore_accents { "ign" } else { "on" },
            if self.use_regex { "on" } else { "off" }
        );
        if let Some(err) = &self.regex_error {
            return format!("find regex error: {err}");
        }
        if self.query.is_empty() {
            return format!(
                "find · {flags} · Alt+C case · Alt+A acentos · Alt+R regex · Ctrl+H replace"
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
            "[{}]case [{}]accent [{}]re · Enter next · Alt+Enter repl · Ctrl+Alt+Enter all · Esc",
            if self.case_sensitive { "x" } else { " " },
            if self.ignore_accents { "x" } else { " " },
            if self.use_regex { "x" } else { " " }
        )
    }
}

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

/// Busca literal com offsets no texto original.
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

/// Busca por regex (crate `regex`).
pub fn find_all_regex(
    haystack: &str,
    pattern: &str,
    case_sensitive: bool,
) -> Result<Vec<MatchRange>, String> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = regex::RegexBuilder::new(pattern);
    builder.case_insensitive(!case_sensitive);
    let re = builder.build().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for m in re.find_iter(haystack) {
        out.push(MatchRange {
            start: m.start(),
            end: m.end(),
        });
    }
    Ok(out)
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
    }

    #[test]
    fn finds_ignore_accents() {
        let ranges = find_all("ação e acao", "acao", false, true);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn case_sensitive_skips() {
        let ranges = find_all("Ab ab AB", "ab", true, false);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 3);
    }

    #[test]
    fn regex_digits() {
        let ranges = find_all_regex("a1 b22 c", r"\d+", false).unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 1);
        assert_eq!(ranges[1].start, 4);
    }
}
