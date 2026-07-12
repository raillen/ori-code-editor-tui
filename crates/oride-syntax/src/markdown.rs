//! Highlight Markdown com grammars block + inline (tree-sitter-md)
//! + **injections** de linguagem em code fences (` ```lang `).

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::highlight::highlight_language_slice;
use crate::kind::HighlightKind;
use crate::language::LanguageId;
use crate::HighlightSpan;

/// Fence com linguagem conhecida e intervalo do **conteúdo** (sem delimitadores).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FenceRegion {
    pub language: LanguageId,
    pub content_start: usize,
    pub content_end: usize,
}

/// Coleta spans de um documento Markdown (block structure + inlines + fence inject).
pub fn collect_markdown_spans(source: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    if source.is_empty() {
        return spans;
    }

    let mut block_parser = Parser::new();
    let block_lang = tree_sitter_md::LANGUAGE.into();
    if block_parser.set_language(&block_lang).is_err() {
        return spans;
    }
    let Some(block_tree) = block_parser.parse(source, None) else {
        return spans;
    };

    // 1) Queries oficiais do grammar block
    if let Ok(query) = Query::new(&block_lang, tree_sitter_md::HIGHLIGHT_QUERY_BLOCK) {
        query_spans(
            &query,
            block_tree.root_node(),
            source.as_bytes(),
            0,
            &mut spans,
        );
    }

    // 2) Fallback / reforço: nós estruturais
    walk_named(block_tree.root_node(), source, &mut spans);

    // 3) Inline: cada nó `inline` reparseado com INLINE_LANGUAGE + query inline
    let mut inline_parser = Parser::new();
    let inline_lang = tree_sitter_md::INLINE_LANGUAGE.into();
    if inline_parser.set_language(&inline_lang).is_ok() {
        let inline_query = Query::new(&inline_lang, tree_sitter_md::HIGHLIGHT_QUERY_INLINE).ok();
        let mut cursor = block_tree.root_node().walk();
        collect_inlines(
            block_tree.root_node(),
            source,
            &mut inline_parser,
            inline_query.as_ref(),
            &mut cursor,
            &mut spans,
        );
    }

    // 4) P6: inject highlight da linguagem dentro de ```lang
    for fence in extract_fenced_regions(block_tree.root_node(), source) {
        if fence.content_start >= fence.content_end || fence.content_end > source.len() {
            continue;
        }
        let slice = &source[fence.content_start..fence.content_end];
        let injected = highlight_language_slice(fence.language, slice, fence.content_start);
        spans.extend(injected);
    }

    spans.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| (a.end - a.start).cmp(&(b.end - b.start)))
    });
    spans
}

/// Extrai regiões de conteúdo de fences com linguagem mapeável.
pub fn extract_fenced_regions(root: tree_sitter::Node, source: &str) -> Vec<FenceRegion> {
    let mut out = Vec::new();
    walk_fences(root, source, &mut out);
    out
}

fn walk_fences(node: tree_sitter::Node, source: &str, out: &mut Vec<FenceRegion>) {
    if node.kind() == "fenced_code_block" {
        if let Some(region) = fence_region_from_node(node, source) {
            out.push(region);
        }
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        walk_fences(child, source, out);
    }
}

fn fence_region_from_node(node: tree_sitter::Node, source: &str) -> Option<FenceRegion> {
    let mut lang_id = None;
    let mut content_start = None;
    let mut content_end = None;
    let mut c = node.walk();
    for child in node.children(&mut c) {
        match child.kind() {
            "info_string" | "language" => {
                let t = child.utf8_text(source.as_bytes()).ok()?.trim();
                // info_string pode ser "rust" ou "rust title"
                let first = t.split_whitespace().next().unwrap_or(t);
                lang_id = fence_lang_alias(first);
            }
            "code_fence_content" => {
                content_start = Some(child.start_byte());
                content_end = Some(child.end_byte().min(source.len()));
            }
            _ => {}
        }
    }
    // Fallback: se não achou nó content, tenta heurística entre delimitadores
    if content_start.is_none() {
        if let Some((s, e)) = heuristic_fence_content(node, source) {
            content_start = Some(s);
            content_end = Some(e);
        }
    }
    let language = lang_id?;
    let content_start = content_start?;
    let content_end = content_end?;
    if content_start >= content_end {
        return None;
    }
    Some(FenceRegion {
        language,
        content_start,
        content_end,
    })
}

fn heuristic_fence_content(node: tree_sitter::Node, source: &str) -> Option<(usize, usize)> {
    // entre primeira e última linha do bloco, sem a linha de abertura/fechamento
    let text = node.utf8_text(source.as_bytes()).ok()?;
    let start = node.start_byte();
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    let rest: String = lines.collect();
    if rest.is_empty() {
        return None;
    }
    // remove última linha se for só ```
    let rest = rest.trim_end_matches('\n');
    let rest = if rest.ends_with("```") {
        rest.trim_end_matches("```").trim_end_matches('\n')
    } else {
        rest
    };
    let inner_start = start + first.len();
    let inner_end = inner_start + rest.len();
    if inner_start < inner_end && inner_end <= source.len() {
        Some((inner_start, inner_end))
    } else {
        None
    }
}

/// Mapeia alias do info string → LanguageId suportado no Oride.
#[must_use]
pub fn fence_lang_alias(name: &str) -> Option<LanguageId> {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "oris" | "oriscript" | "ori" => Some(LanguageId::OriScript),
        "js" | "javascript" | "jsx" | "mjs" | "cjs" => Some(LanguageId::JavaScript),
        "html" | "htm" => Some(LanguageId::Html),
        "css" | "scss" => Some(LanguageId::Css),
        // rust e outros sem grammar no binário: None (fica só paint de code block)
        _ => None,
    }
}

fn query_spans(
    query: &Query,
    root: tree_sitter::Node,
    source: &[u8],
    byte_offset: usize,
    out: &mut Vec<HighlightSpan>,
) {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, source);
    while let Some(m) = matches.next() {
        for cap in m.captures {
            let name = query.capture_names()[cap.index as usize];
            let kind = HighlightKind::from_capture_name(name);
            if kind == HighlightKind::Normal && name == "none" {
                continue;
            }
            let start = cap.node.start_byte() + byte_offset;
            let end = cap.node.end_byte() + byte_offset;
            if start < end {
                out.push(HighlightSpan { start, end, kind });
            }
        }
    }
}

fn walk_named(node: tree_sitter::Node, source: &str, out: &mut Vec<HighlightSpan>) {
    if node.is_named() {
        if let Some(kind) = HighlightKind::from_node_kind(node.kind()) {
            // Emite nós “token-like” ou estruturais MD mesmo com filhos
            if is_md_paint_node(node.kind()) {
                let start = node.start_byte();
                let end = node.end_byte().min(source.len());
                if start < end {
                    out.push(HighlightSpan { start, end, kind });
                }
            }
        }
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        walk_named(child, source, out);
    }
}

fn is_md_paint_node(kind: &str) -> bool {
    matches!(
        kind,
        "atx_h1_marker"
            | "atx_h2_marker"
            | "atx_h3_marker"
            | "atx_h4_marker"
            | "atx_h5_marker"
            | "atx_h6_marker"
            | "setext_h1_underline"
            | "setext_h2_underline"
            | "list_marker_plus"
            | "list_marker_minus"
            | "list_marker_star"
            | "list_marker_dot"
            | "list_marker_parenthesis"
            | "task_list_marker_checked"
            | "task_list_marker_unchecked"
            | "block_quote_marker"
            | "fenced_code_block_delimiter"
            | "info_string"
            | "language"
            | "code_span_delimiter"
            | "emphasis_delimiter"
            | "code_fence_content"
            | "indented_code_block"
            | "fenced_code_block"
            | "code_span"
            | "link_destination"
            | "uri_autolink"
            | "email_autolink"
            | "pipe_table_delimiter_cell"
            | "pipe_table_align_left"
            | "pipe_table_align_right"
    )
}

fn collect_inlines(
    node: tree_sitter::Node,
    source: &str,
    inline_parser: &mut Parser,
    inline_query: Option<&Query>,
    _cursor: &mut tree_sitter::TreeCursor,
    out: &mut Vec<HighlightSpan>,
) {
    if node.kind() == "inline" {
        let start = node.start_byte();
        let end = node.end_byte().min(source.len());
        if start < end {
            let slice = &source[start..end];
            if let Some(tree) = inline_parser.parse(slice, None) {
                if let Some(q) = inline_query {
                    query_spans(q, tree.root_node(), slice.as_bytes(), start, out);
                }
                walk_named_offset(tree.root_node(), slice, start, out);
            }
        }
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        collect_inlines(child, source, inline_parser, inline_query, _cursor, out);
    }
}

fn walk_named_offset(
    node: tree_sitter::Node,
    source: &str,
    base: usize,
    out: &mut Vec<HighlightSpan>,
) {
    if node.is_named() {
        if let Some(kind) = HighlightKind::from_node_kind(node.kind()) {
            if is_md_paint_node(node.kind())
                || matches!(
                    node.kind(),
                    "emphasis"
                        | "strong_emphasis"
                        | "strikethrough"
                        | "code_span"
                        | "image"
                        | "inline_link"
                        | "shortcut_link"
                        | "full_reference_link"
                        | "link_text"
                        | "link_label"
                        | "link_title"
                        | "image_description"
                )
            {
                let start = base + node.start_byte();
                let end = (base + node.end_byte()).min(base + source.len());
                if start < end {
                    out.push(HighlightSpan { start, end, kind });
                }
            }
        }
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        walk_named_offset(child, source, base, out);
    }
}

/// Prefixo de lista Markdown na linha (ex.: `- `, `* `, `1. `, `> `, `- [ ] `).
#[must_use]
pub fn list_prefix(line: &str) -> Option<&str> {
    let trimmed_start = line.len() - line.trim_start().len();
    let rest = &line[trimmed_start..];
    // block quote
    if let Some(r) = rest.strip_prefix("> ") {
        let _ = r;
        return Some(&line[..trimmed_start + 2]);
    }
    if rest == ">" {
        return Some(&line[..trimmed_start + 1]);
    }
    // task list
    for p in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] "] {
        if rest.starts_with(p) {
            return Some(&line[..trimmed_start + p.len()]);
        }
    }
    // unordered
    for p in ["- ", "* ", "+ "] {
        if rest.starts_with(p) {
            return Some(&line[..trimmed_start + p.len()]);
        }
    }
    // ordered "1. " / "12. "
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i + 1 < bytes.len() && bytes[i] == b'.' && bytes[i + 1] == b' ' {
        return Some(&line[..trimmed_start + i + 2]);
    }
    None
}

/// Continuação ao pressionar Enter numa linha de lista: devolve o prefixo a inserir
/// após `\n`, ou `None` se a linha for só o marcador (sair da lista).
#[must_use]
pub fn continue_list_on_enter(current_line: &str) -> Option<String> {
    let prefix = list_prefix(current_line)?;
    let after = &current_line[prefix.len()..];
    if after.trim().is_empty() {
        // linha só com marcador → não continua (caller apaga o marcador se quiser)
        return None;
    }
    // task list: ao continuar, usa unchecked
    let cont = if prefix.contains("[x]") || prefix.contains("[X]") || prefix.contains("[ ]") {
        let indent = prefix.len() - prefix.trim_start().len();
        let bullet = if prefix.trim_start().starts_with('*') {
            "*"
        } else {
            "-"
        };
        format!("{}{bullet} [ ] ", " ".repeat(indent))
    } else if let Some(rest) = prefix.trim_start().strip_suffix(". ") {
        // ordered: increment number
        if let Ok(n) = rest.trim().parse::<u64>() {
            let indent = prefix.len() - prefix.trim_start().len();
            format!("{}{}. ", " ".repeat(indent), n + 1)
        } else {
            prefix.to_string()
        }
    } else {
        prefix.to_string()
    };
    Some(cont)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HighlightKind;

    #[test]
    fn highlights_headings_and_code() {
        let src = "# Title\n\nHello **bold** and `code`\n\n```rust\nfn x() {}\n```\n\n- item\n";
        let spans = collect_markdown_spans(src);
        assert!(!spans.is_empty(), "expected markdown spans");
        assert!(
            spans.iter().any(|s| s.kind == HighlightKind::Heading
                || s.kind == HighlightKind::Code
                || s.kind == HighlightKind::Strong
                || s.kind == HighlightKind::ListMarker),
            "kinds: {:?}",
            spans.iter().map(|s| s.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn list_continue() {
        assert_eq!(continue_list_on_enter("- hello"), Some("- ".into()));
        assert_eq!(continue_list_on_enter("- "), None);
        assert_eq!(continue_list_on_enter("1. first"), Some("2. ".into()));
        assert_eq!(continue_list_on_enter("- [x] done"), Some("- [ ] ".into()));
    }
}

#[cfg(test)]
mod injection_tests {
    use super::*;
    use crate::HighlightKind;

    #[test]
    fn injects_oriscript_keywords_inside_fence() {
        let src = "```oris
fn main() {}
```
";
        let spans = collect_markdown_spans(src);
        // conteúdo do fence deve ter spans de identificador/var (injection),
        // não só o paint genérico "Code" do MD
        let content = spans.iter().any(|s| {
            s.start >= 8
                && matches!(
                    s.kind,
                    HighlightKind::Variable
                        | HighlightKind::Keyword
                        | HighlightKind::Function
                        | HighlightKind::Punctuation
                )
        });
        assert!(
            content,
            "expected injected language spans inside fence, spans={spans:?}"
        );
    }

    #[test]
    fn fence_alias_oris() {
        assert_eq!(fence_lang_alias("oriscript"), Some(LanguageId::OriScript));
        assert_eq!(fence_lang_alias("js"), Some(LanguageId::JavaScript));
        assert_eq!(fence_lang_alias("python"), None);
    }
}
