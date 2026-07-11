//! Motor de highlight: parse tree-sitter → spans por byte.

use tree_sitter::{Parser, Tree};

use crate::kind::HighlightKind;
use crate::language::LanguageId;

/// Intervalo semi-aberto `[start, end)` em bytes UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

/// Engine reutilizável (parser + cache de árvore por conteúdo).
pub struct HighlightEngine {
    parser: Parser,
    language: LanguageId,
    source: String,
    tree: Option<Tree>,
    spans: Vec<HighlightSpan>,
}

impl Default for HighlightEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HighlightEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            language: LanguageId::Plain,
            source: String::new(),
            tree: None,
            spans: Vec::new(),
        }
    }

    #[must_use]
    pub fn language(&self) -> LanguageId {
        self.language
    }

    #[must_use]
    pub fn spans(&self) -> &[HighlightSpan] {
        &self.spans
    }

    /// Atualiza highlight se linguagem/fonte mudaram.
    pub fn update(&mut self, language: LanguageId, source: &str) {
        if self.language == language && self.source == source {
            return;
        }
        self.language = language;
        self.source = source.to_string();
        self.rehighlight();
    }

    fn rehighlight(&mut self) {
        self.spans.clear();
        self.tree = None;
        if self.language == LanguageId::Plain || self.source.is_empty() {
            return;
        }
        let lang = match language_ts(self.language) {
            Some(l) => l,
            None => return,
        };
        if self.parser.set_language(&lang).is_err() {
            return;
        }
        let tree = match self.parser.parse(&self.source, None) {
            Some(t) => t,
            None => return,
        };
        collect_spans(tree.root_node(), &self.source, &mut self.spans);
        // Preferir spans menores (folhas) quando sobrepõem: ordenar por start, depois por comprimento asc
        self.spans.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then_with(|| (a.end - a.start).cmp(&(b.end - b.start)))
        });
        self.tree = Some(tree);
    }
}

fn language_ts(id: LanguageId) -> Option<tree_sitter::Language> {
    let lang = match id {
        LanguageId::Plain => return None,
        LanguageId::OriScript => tree_sitter_oriscript::LANGUAGE.into(),
        LanguageId::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        LanguageId::Html => tree_sitter_html::LANGUAGE.into(),
        LanguageId::Css => tree_sitter_css::LANGUAGE.into(),
        LanguageId::Markdown => tree_sitter_md::LANGUAGE.into(),
    };
    Some(lang)
}

fn collect_spans(node: tree_sitter::Node, source: &str, out: &mut Vec<HighlightSpan>) {
    // Visita nós nomeados; se tem kind mapeável e é “folha nomeada” ou tem texto direto, emite.
    if node.is_named() {
        if let Some(kind) = HighlightKind::from_node_kind(node.kind()) {
            // Só emite se não tem filho nomeado (preferir folhas) ou é token-like
            let mut cursor = node.walk();
            let has_named_child = node.named_children(&mut cursor).next().is_some();
            if !has_named_child || is_token_like(node.kind()) {
                let start = node.start_byte();
                let end = node.end_byte().min(source.len());
                if start < end {
                    out.push(HighlightSpan { start, end, kind });
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_spans(child, source, out);
    }
}

fn is_token_like(kind: &str) -> bool {
    matches!(
        kind,
        "keyword"
            | "identifier"
            | "number"
            | "string"
            | "operator"
            | "punctuation"
            | "type_builtin"
            | "constant_builtin"
            | "line_comment"
            | "block_comment"
            | "comment"
            | "tag_name"
            | "attribute_name"
            | "property_identifier"
    ) || kind.len() == 1
}

/// Fatias de uma linha (byte range da linha no source) → segmentos (texto, kind).
/// `line_start`/`line_end` em bytes no source global; `line_text` sem `\n`.
#[must_use]
pub fn line_spans<'a>(
    line_text: &'a str,
    line_start: usize,
    highlights: &[HighlightSpan],
) -> Vec<(&'a str, HighlightKind)> {
    let line_end = line_start + line_text.len();
    if line_text.is_empty() {
        return vec![];
    }

    // Eventos de mudança de kind
    let mut cuts: Vec<(usize, Option<HighlightKind>)> = vec![(0, None)];
    for h in highlights {
        if h.end <= line_start || h.start >= line_end {
            continue;
        }
        let s = h.start.saturating_sub(line_start).min(line_text.len());
        let e = h.end.saturating_sub(line_start).min(line_text.len());
        if s < e {
            cuts.push((s, Some(h.kind)));
            cuts.push((e, None));
        }
    }
    cuts.push((line_text.len(), None));
    cuts.sort_by_key(|(o, _)| *o);

    // Resolve kind em cada offset: última highlight ativa
    // Abordagem simples: para cada segmento entre cuts únicos, pega o highlight mais específico cobrindo o mid
    let mut points: Vec<usize> = cuts.iter().map(|(o, _)| *o).collect();
    points.sort_unstable();
    points.dedup();

    let mut out = Vec::new();
    for w in points.windows(2) {
        let a = w[0];
        let b = w[1];
        if a >= b {
            continue;
        }
        let mid = a + (b - a) / 2;
        let abs = line_start + mid;
        let kind = highlights
            .iter()
            .filter(|h| h.start <= abs && abs < h.end)
            .min_by_key(|h| h.end - h.start)
            .map(|h| h.kind)
            .unwrap_or(HighlightKind::Normal);
        // boundary safe split
        if !line_text.is_char_boundary(a) || !line_text.is_char_boundary(b) {
            continue;
        }
        out.push((&line_text[a..b], kind));
    }

    // Merge adjacents same kind
    let mut merged: Vec<(&str, HighlightKind)> = Vec::new();
    for (text, kind) in out {
        if let Some(last) = merged.last_mut() {
            if last.1 == kind {
                // cannot extend &str easily without indices — push separate ok
                // For display, adjacent same-kind spans are fine
            }
        }
        if text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if last.1 == kind {
                // skip merge of str slices; keep separate
            }
        }
        merged.push((text, kind));
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_oriscript_keywords() {
        let mut eng = HighlightEngine::new();
        let src = "fn main() {\n  let x = 1\n}\n";
        eng.update(LanguageId::OriScript, src);
        assert!(!eng.spans().is_empty(), "expected spans for oriscript");
        assert!(
            eng.spans()
                .iter()
                .any(|s| s.kind == HighlightKind::Keyword || s.kind == HighlightKind::Number),
            "spans: {:?}",
            eng.spans()
        );
    }

    #[test]
    fn highlights_javascript() {
        let mut eng = HighlightEngine::new();
        eng.update(LanguageId::JavaScript, "const x = \"hi\"; // c\n");
        assert!(eng.spans().iter().any(|s| s.kind == HighlightKind::String));
    }

    #[test]
    fn line_spans_splits() {
        let highlights = vec![
            HighlightSpan {
                start: 0,
                end: 2,
                kind: HighlightKind::Keyword,
            },
            HighlightSpan {
                start: 3,
                end: 6,
                kind: HighlightKind::Number,
            },
        ];
        let parts = line_spans("fn 123", 0, &highlights);
        assert!(parts.iter().any(|(_, k)| *k == HighlightKind::Keyword));
        assert!(parts.iter().any(|(_, k)| *k == HighlightKind::Number));
    }
}
