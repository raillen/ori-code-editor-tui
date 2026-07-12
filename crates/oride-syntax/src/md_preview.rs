//! Preview Markdown → linhas semânticas (sem ratatui).
//!
//! O UI mapeia `PreviewStyle` para cores. Não é HTML; é “ANSI-like” em TUI.

/// Estilo de um segmento de preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewStyle {
    Normal,
    Heading(u8),
    Bold,
    Italic,
    Code,
    Link,
    Quote,
    ListMarker,
    Hr,
    FenceLang,
}

/// Uma linha do painel de preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewLine {
    pub segments: Vec<(String, PreviewStyle)>,
}

impl PreviewLine {
    fn plain(s: impl Into<String>) -> Self {
        Self {
            segments: vec![(s.into(), PreviewStyle::Normal)],
        }
    }

    fn styled(s: impl Into<String>, style: PreviewStyle) -> Self {
        Self {
            segments: vec![(s.into(), style)],
        }
    }

    fn empty() -> Self {
        Self {
            segments: vec![(" ".into(), PreviewStyle::Normal)],
        }
    }
}

/// Renderiza Markdown simples em linhas de preview.
#[must_use]
pub fn render_preview_lines(source: &str) -> Vec<PreviewLine> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_lang = String::new();

    for raw in source.lines() {
        let line = raw;

        // fences
        if let Some(rest) = line.strip_prefix("```") {
            if in_fence {
                in_fence = false;
                fence_lang.clear();
                out.push(PreviewLine::styled("───", PreviewStyle::Hr));
            } else {
                in_fence = true;
                fence_lang = rest.trim().to_string();
                let label = if fence_lang.is_empty() {
                    "code".into()
                } else {
                    fence_lang.clone()
                };
                out.push(PreviewLine::styled(
                    format!("┌ {label}"),
                    PreviewStyle::FenceLang,
                ));
            }
            continue;
        }
        if in_fence {
            out.push(PreviewLine::styled(format!("│ {line}"), PreviewStyle::Code));
            continue;
        }

        // HR
        let t = line.trim();
        if matches!(t, "---" | "***" | "___")
            || (t.len() >= 3 && t.chars().all(|c| c == '-' || c == '*' || c == '_'))
        {
            out.push(PreviewLine::styled("────────", PreviewStyle::Hr));
            continue;
        }

        // headings ATX
        if let Some((level, text)) = parse_atx_heading(line) {
            let prefix = "#".repeat(level as usize);
            out.push(PreviewLine {
                segments: vec![
                    (format!("{prefix} "), PreviewStyle::Heading(level)),
                    (text.to_string(), PreviewStyle::Heading(level)),
                ],
            });
            continue;
        }

        // blockquote
        if let Some(rest) = line.trim_start().strip_prefix("> ") {
            out.push(PreviewLine {
                segments: vec![
                    ("│ ".into(), PreviewStyle::Quote),
                    (rest.to_string(), PreviewStyle::Quote),
                ],
            });
            continue;
        }
        if line.trim_start() == ">" {
            out.push(PreviewLine::styled("│", PreviewStyle::Quote));
            continue;
        }

        // list unordered
        if let Some(rest) = strip_ul(line) {
            let mut segs = vec![(" • ".into(), PreviewStyle::ListMarker)];
            segs.extend(inline_segments(rest));
            out.push(PreviewLine { segments: segs });
            continue;
        }

        // list ordered "1. "
        if let Some((num, rest)) = strip_ol(line) {
            let mut segs = vec![(format!(" {num}. "), PreviewStyle::ListMarker)];
            segs.extend(inline_segments(rest));
            out.push(PreviewLine { segments: segs });
            continue;
        }

        // empty
        if line.trim().is_empty() {
            out.push(PreviewLine::empty());
            continue;
        }

        // paragraph with inline
        out.push(PreviewLine {
            segments: inline_segments(line),
        });
    }

    if out.is_empty() {
        out.push(PreviewLine::plain("(vazio)"));
    }
    out
}

fn parse_atx_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let mut level = 0u8;
    let bytes = trimmed.as_bytes();
    while (level as usize) < bytes.len() && bytes[level as usize] == b'#' && level < 6 {
        level += 1;
    }
    if level == 0 {
        return None;
    }
    if (level as usize) < bytes.len() && bytes[level as usize] != b' ' {
        // ###no space — still accept if end
        if (level as usize) == bytes.len() {
            return Some((level, ""));
        }
        return None;
    }
    let text = trimmed[level as usize..].trim_start();
    Some((level, text))
}

fn strip_ul(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for p in ["- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(p) {
            return Some(r);
        }
    }
    // task list
    for p in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] "] {
        if let Some(r) = t.strip_prefix(p) {
            return Some(r);
        }
    }
    None
}

fn strip_ol(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    let mut i = 0;
    let b = t.as_bytes();
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= b.len() {
        return None;
    }
    if b[i] != b'.' {
        return None;
    }
    if i + 1 < b.len() && b[i + 1] == b' ' {
        return Some((&t[..i], &t[i + 2..]));
    }
    None
}

/// Inline: `code`, **bold**, *italic*, [link](url) — best-effort left-to-right.
fn inline_segments(text: &str) -> Vec<(String, PreviewStyle)> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut buf = String::new();

    let flush = |buf: &mut String, out: &mut Vec<(String, PreviewStyle)>, style: PreviewStyle| {
        if !buf.is_empty() {
            out.push((std::mem::take(buf), style));
        }
    };

    while i < chars.len() {
        // code `...`
        if chars[i] == '`' {
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '`' {
                i += 1;
            }
            let code: String = chars[start..i].iter().collect();
            out.push((code, PreviewStyle::Code));
            if i < chars.len() {
                i += 1; // closing `
            }
            continue;
        }
        // **bold**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 2;
            let start = i;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                i += 1;
            }
            let bold: String = chars[start..i].iter().collect();
            out.push((bold, PreviewStyle::Bold));
            if i + 1 < chars.len() {
                i += 2;
            }
            continue;
        }
        // *italic*
        if chars[i] == '*' {
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '*' {
                i += 1;
            }
            let it: String = chars[start..i].iter().collect();
            out.push((it, PreviewStyle::Italic));
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        // [text](url)
        if chars[i] == '[' {
            if let Some((label, url, next)) = parse_link(&chars, i) {
                flush(&mut buf, &mut out, PreviewStyle::Normal);
                out.push((label, PreviewStyle::Link));
                if !url.is_empty() {
                    out.push((format!(" ({url})"), PreviewStyle::Link));
                }
                i = next;
                continue;
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    flush(&mut buf, &mut out, PreviewStyle::Normal);
    if out.is_empty() {
        out.push((String::new(), PreviewStyle::Normal));
    }
    out
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    // [label](url)
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let mut i = start + 1;
    let label_start = i;
    while i < chars.len() && chars[i] != ']' {
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let label: String = chars[label_start..i].iter().collect();
    i += 1;
    if chars.get(i) != Some(&'(') {
        return None;
    }
    i += 1;
    let url_start = i;
    while i < chars.len() && chars[i] != ')' {
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let url: String = chars[url_start..i].iter().collect();
    i += 1;
    Some((label, url, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_list() {
        let lines = render_preview_lines("# Title\n\n- item **bold**\n");
        assert!(lines[0]
            .segments
            .iter()
            .any(|(_, s)| matches!(s, PreviewStyle::Heading(1))));
        assert!(lines.iter().any(|l| {
            l.segments
                .iter()
                .any(|(t, s)| t.contains("item") || matches!(s, PreviewStyle::ListMarker))
        }));
    }

    #[test]
    fn fence_block() {
        let lines = render_preview_lines("```oris\nfn main() {}\n```\n");
        assert!(lines
            .iter()
            .any(|l| l.segments.iter().any(|(_, s)| *s == PreviewStyle::Code)));
        assert!(lines.iter().any(|l| {
            l.segments
                .iter()
                .any(|(t, s)| *s == PreviewStyle::FenceLang && t.contains("oris"))
        }));
    }

    #[test]
    fn inline_code() {
        let lines = render_preview_lines("use `code` here\n");
        assert!(lines[0]
            .segments
            .iter()
            .any(|(t, s)| t == "code" && *s == PreviewStyle::Code));
    }
}
