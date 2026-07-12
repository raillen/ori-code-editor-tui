//! Preview Markdown → linhas semânticas (sem ratatui).
//!
//! O UI mapeia `PreviewStyle` para cores. Não é HTML; é “ANSI-like” em TUI.
//! Imagens viram **placeholders** (alt + path + existe?); sem bitmap.

use std::path::{Path, PathBuf};

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
    /// Moldura do card de imagem.
    Image,
    /// Texto alt da imagem.
    ImageAlt,
    /// Path / URL da imagem.
    ImagePath,
    /// Arquivo local encontrado.
    ImageOk,
    /// Arquivo local ausente.
    ImageMissing,
    /// Célula / linha de tabela.
    Table,
    /// ~~riscado~~
    Strike,
    /// Texto secundário / dica.
    Dim,
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

    fn multi(segments: Vec<(String, PreviewStyle)>) -> Self {
        if segments.is_empty() {
            Self::empty()
        } else {
            Self { segments }
        }
    }
}

/// Renderiza Markdown simples (sem base path — imagens sem check de disco).
#[must_use]
pub fn render_preview_lines(source: &str) -> Vec<PreviewLine> {
    render_preview_lines_in(source, None)
}

/// Como [`render_preview_lines`], resolvendo paths de imagem relativos a `base_dir`
/// (normalmente o diretório do arquivo `.md` aberto).
#[must_use]
pub fn render_preview_lines_in(source: &str, base_dir: Option<&Path>) -> Vec<PreviewLine> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_lang = String::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut idx = 0usize;

    // frontmatter YAML simples no topo
    if lines.first().map(|l| l.trim() == "---").unwrap_or(false) {
        out.push(PreviewLine::styled("── frontmatter ──", PreviewStyle::Dim));
        idx = 1;
        while idx < lines.len() {
            if lines[idx].trim() == "---" {
                out.push(PreviewLine::styled("────────", PreviewStyle::Hr));
                idx += 1;
                break;
            }
            out.push(PreviewLine::styled(
                format!("  {}", lines[idx]),
                PreviewStyle::Dim,
            ));
            idx += 1;
        }
    }

    while idx < lines.len() {
        let line = lines[idx];

        // fences
        if let Some(rest) = line.strip_prefix("```") {
            if in_fence {
                in_fence = false;
                fence_lang.clear();
                out.push(PreviewLine::styled("└───", PreviewStyle::Hr));
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
            idx += 1;
            continue;
        }
        if in_fence {
            out.push(PreviewLine::styled(format!("│ {line}"), PreviewStyle::Code));
            idx += 1;
            continue;
        }

        // setext heading: Title\n=== or ---
        if idx + 1 < lines.len() {
            let next = lines[idx + 1].trim();
            if !line.trim().is_empty()
                && next.len() >= 2
                && (next.chars().all(|c| c == '=') || next.chars().all(|c| c == '-'))
            {
                let level = if next.starts_with('=') { 1u8 } else { 2u8 };
                out.push(PreviewLine {
                    segments: vec![(line.trim().to_string(), PreviewStyle::Heading(level))],
                });
                if level == 1 {
                    out.push(PreviewLine::styled("════════", PreviewStyle::Heading(1)));
                } else {
                    out.push(PreviewLine::styled("────────", PreviewStyle::Heading(2)));
                }
                idx += 2;
                continue;
            }
        }

        // HR (só se não for setext — já tratado)
        let t = line.trim();
        if is_hr(t) {
            out.push(PreviewLine::styled("────────────", PreviewStyle::Hr));
            idx += 1;
            continue;
        }

        // headings ATX
        if let Some((level, text)) = parse_atx_heading(line) {
            let mark = match level {
                1 => "█ ",
                2 => "▓ ",
                3 => "▒ ",
                _ => "· ",
            };
            let mut segs = vec![(mark.into(), PreviewStyle::Heading(level))];
            segs.extend(inline_segments(text, base_dir, false));
            out.push(PreviewLine::multi(segs));
            if level <= 2 {
                out.push(PreviewLine::empty());
            }
            idx += 1;
            continue;
        }

        // blockquote (com inline)
        if let Some(rest) = line.trim_start().strip_prefix("> ") {
            let mut segs = vec![("│ ".into(), PreviewStyle::Quote)];
            segs.extend(inline_segments(rest, base_dir, false));
            // força quote nos inlines de texto normal
            for s in &mut segs {
                if matches!(s.1, PreviewStyle::Normal) {
                    s.1 = PreviewStyle::Quote;
                }
            }
            out.push(PreviewLine::multi(segs));
            idx += 1;
            continue;
        }
        if line.trim_start() == ">" {
            out.push(PreviewLine::styled("│", PreviewStyle::Quote));
            idx += 1;
            continue;
        }

        // table row
        if is_table_row(line) {
            // skip separator |---|
            if is_table_sep(line) {
                out.push(PreviewLine::styled(
                    format!("  {}", normalize_table_sep(line)),
                    PreviewStyle::Dim,
                ));
            } else {
                out.push(PreviewLine::styled(
                    format!("  {}", format_table_row(line)),
                    PreviewStyle::Table,
                ));
            }
            idx += 1;
            continue;
        }

        // task list
        if let Some((done, rest)) = strip_task(line) {
            let mark = if done { " ☑ " } else { " ☐ " };
            let mut segs = vec![(mark.into(), PreviewStyle::ListMarker)];
            segs.extend(inline_segments(rest, base_dir, false));
            out.push(PreviewLine::multi(segs));
            idx += 1;
            continue;
        }

        // list unordered
        if let Some(rest) = strip_ul(line) {
            let mut segs = vec![(" • ".into(), PreviewStyle::ListMarker)];
            segs.extend(inline_segments(rest, base_dir, false));
            out.push(PreviewLine::multi(segs));
            idx += 1;
            continue;
        }

        // list ordered
        if let Some((num, rest)) = strip_ol(line) {
            let mut segs = vec![(format!(" {num}. "), PreviewStyle::ListMarker)];
            segs.extend(inline_segments(rest, base_dir, false));
            out.push(PreviewLine::multi(segs));
            idx += 1;
            continue;
        }

        // empty
        if line.trim().is_empty() {
            out.push(PreviewLine::empty());
            idx += 1;
            continue;
        }

        // linha só com imagem → card multi-linha
        if let Some((alt, url)) = parse_standalone_image(line) {
            out.extend(image_card(&alt, &url, base_dir));
            idx += 1;
            continue;
        }

        // parágrafo com inline (imagens inline → card compacto no fluxo)
        out.push(PreviewLine::multi(inline_segments(line, base_dir, true)));
        idx += 1;
    }

    if out.is_empty() {
        out.push(PreviewLine::plain("(vazio)"));
    }
    out
}

fn is_hr(t: &str) -> bool {
    matches!(t, "---" | "***" | "___" | "* * *" | "- - -")
        || (t.len() >= 3
            && t.chars()
                .all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
            && t.chars().filter(|c| *c != ' ').count() >= 3
            && !t.contains('|'))
}

fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.matches('|').count() >= 2
}

fn is_table_sep(line: &str) -> bool {
    let t = line.trim().trim_matches('|');
    !t.is_empty()
        && t.chars()
            .all(|c| c == '-' || c == ':' || c == '|' || c == ' ')
}

fn format_table_row(line: &str) -> String {
    let cells: Vec<&str> = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    cells.join(" │ ")
}

fn normalize_table_sep(line: &str) -> String {
    let n = line.matches('|').count().saturating_sub(1).max(1);
    (0..n).map(|_| "───").collect::<Vec<_>>().join("─┼─")
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
        if (level as usize) == bytes.len() {
            return Some((level, ""));
        }
        return None;
    }
    let text = trimmed[level as usize..].trim_start();
    Some((level, text))
}

fn strip_task(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    for (p, done) in [
        ("- [ ] ", false),
        ("- [x] ", true),
        ("- [X] ", true),
        ("* [ ] ", false),
        ("* [x] ", true),
        ("* [X] ", true),
        ("+ [ ] ", false),
        ("+ [x] ", true),
    ] {
        if let Some(r) = t.strip_prefix(p) {
            return Some((done, r));
        }
    }
    None
}

fn strip_ul(line: &str) -> Option<&str> {
    let t = line.trim_start();
    for p in ["- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(p) {
            // task lists handled elsewhere
            if r.starts_with("[ ]") || r.starts_with("[x]") || r.starts_with("[X]") {
                return None;
            }
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

/// Linha que é só `![alt](url)` (espaços ok).
fn parse_standalone_image(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if !t.starts_with("![") {
        return None;
    }
    let chars: Vec<char> = t.chars().collect();
    let (alt, url, next) = parse_image(&chars, 0)?;
    if next == chars.len() {
        Some((alt, url))
    } else {
        None
    }
}

fn image_card(alt: &str, url: &str, base_dir: Option<&Path>) -> Vec<PreviewLine> {
    let alt_show = if alt.is_empty() {
        "(sem texto alt)"
    } else {
        alt
    };
    let (path_line, status_style, status_note) = describe_image_target(url, base_dir);
    vec![
        PreviewLine::styled("┌ 🖼  imagem", PreviewStyle::Image),
        PreviewLine::multi(vec![
            ("│  ".into(), PreviewStyle::Image),
            (alt_show.into(), PreviewStyle::ImageAlt),
        ]),
        PreviewLine::multi(vec![
            ("│  ".into(), PreviewStyle::Image),
            (path_line, PreviewStyle::ImagePath),
        ]),
        PreviewLine::multi(vec![
            ("│  ".into(), PreviewStyle::Image),
            (status_note, status_style),
        ]),
        PreviewLine::styled("└", PreviewStyle::Image),
    ]
}

fn describe_image_target(url: &str, base_dir: Option<&Path>) -> (String, PreviewStyle, String) {
    let url = url.trim();
    if url.is_empty() {
        return (
            "(sem path)".into(),
            PreviewStyle::ImageMissing,
            "⚠ path vazio".into(),
        );
    }
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        return (
            truncate_mid(url, 48),
            PreviewStyle::Dim,
            "🔗 URL remota · não embutida no TUI".into(),
        );
    }
    // local path
    let path = PathBuf::from(url);
    let resolved = if path.is_absolute() {
        path
    } else if let Some(base) = base_dir {
        base.join(&path)
    } else {
        path
    };
    let display = truncate_mid(&resolved.display().to_string(), 48);
    if resolved.is_file() {
        (
            display,
            PreviewStyle::ImageOk,
            "✓ arquivo local encontrado".into(),
        )
    } else if resolved.exists() {
        (
            display,
            PreviewStyle::ImageMissing,
            "⚠ path existe mas não é arquivo".into(),
        )
    } else {
        (
            display,
            PreviewStyle::ImageMissing,
            "✗ arquivo não encontrado".into(),
        )
    }
}

fn truncate_mid(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1) / 2;
    let left: String = s.chars().take(keep).collect();
    let right: String = s.chars().skip(n - keep).collect();
    format!("{left}…{right}")
}

/// Inline: code, bold, italic, strike, image, link.
/// `expand_images`: se true, imagem vira `🖼 alt` compacto (não card).
fn inline_segments(
    text: &str,
    base_dir: Option<&Path>,
    expand_images: bool,
) -> Vec<(String, PreviewStyle)> {
    let _ = base_dir; // reservado para tooltips futuros
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
        // image ![alt](url) — antes de link
        if chars[i] == '!' && chars.get(i + 1) == Some(&'[') {
            if let Some((alt, url, next)) = parse_image(&chars, i) {
                flush(&mut buf, &mut out, PreviewStyle::Normal);
                if expand_images {
                    let label = if alt.is_empty() {
                        "🖼".into()
                    } else {
                        format!("🖼 {alt}")
                    };
                    out.push((label, PreviewStyle::Image));
                    if !url.is_empty() {
                        out.push((format!(" ({})", truncate_mid(&url, 24)), PreviewStyle::Dim));
                    }
                } else {
                    out.push((
                        format!("🖼 {}", if alt.is_empty() { "img" } else { &alt }),
                        PreviewStyle::Image,
                    ));
                }
                i = next;
                continue;
            }
        }

        // code `...`
        if chars[i] == '`' {
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '`' {
                i += 1;
            }
            let code: String = chars[start..i].iter().collect();
            out.push((format!(" {code} "), PreviewStyle::Code));
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        // ~~strike~~
        if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' {
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 2;
            let start = i;
            while i + 1 < chars.len() && !(chars[i] == '~' && chars[i + 1] == '~') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push((s, PreviewStyle::Strike));
            if i + 1 < chars.len() {
                i += 2;
            }
            continue;
        }

        // **bold** or __bold__
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let mark = chars[i];
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 2;
            let start = i;
            while i + 1 < chars.len() && !(chars[i] == mark && chars[i + 1] == mark) {
                i += 1;
            }
            let bold: String = chars[start..i].iter().collect();
            out.push((bold, PreviewStyle::Bold));
            if i + 1 < chars.len() {
                i += 2;
            }
            continue;
        }

        // *italic* or _italic_ (não confundir com __)
        if (chars[i] == '*' || chars[i] == '_') && chars.get(i + 1) != Some(&chars[i]) {
            let mark = chars[i];
            // _word_ mid-word: skip if alnum before
            if mark == '_' && i > 0 && chars[i - 1].is_alphanumeric() {
                buf.push(chars[i]);
                i += 1;
                continue;
            }
            flush(&mut buf, &mut out, PreviewStyle::Normal);
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != mark {
                i += 1;
            }
            let it: String = chars[start..i].iter().collect();
            out.push((it, PreviewStyle::Italic));
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        // [text](url) link
        if chars[i] == '[' {
            if let Some((label, url, next)) = parse_link(&chars, i) {
                flush(&mut buf, &mut out, PreviewStyle::Normal);
                out.push((label, PreviewStyle::Link));
                if !url.is_empty() {
                    out.push((format!(" → {}", truncate_mid(&url, 28)), PreviewStyle::Dim));
                }
                i = next;
                continue;
            }
        }

        // autolink <http...>
        if chars[i] == '<' {
            if let Some((url, next)) = parse_autolink(&chars, i) {
                flush(&mut buf, &mut out, PreviewStyle::Normal);
                out.push((url, PreviewStyle::Link));
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

fn parse_image(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    // ![alt](url)
    if chars.get(start) != Some(&'!') || chars.get(start + 1) != Some(&'[') {
        return None;
    }
    let mut i = start + 2;
    let alt_start = i;
    while i < chars.len() && chars[i] != ']' {
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let alt: String = chars[alt_start..i].iter().collect();
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
    Some((alt, url.trim().to_string(), i))
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let mut i = start + 1;
    let label_start = i;
    let mut depth = 1i32;
    while i < chars.len() {
        if chars[i] == '[' {
            depth += 1;
        } else if chars[i] == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        i += 1;
    }
    if i >= chars.len() || depth != 0 {
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
    Some((label, url.trim().to_string(), i))
}

fn parse_autolink(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'<') {
        return None;
    }
    let mut i = start + 1;
    let s = i;
    while i < chars.len() && chars[i] != '>' {
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let url: String = chars[s..i].iter().collect();
    if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")) {
        return None;
    }
    Some((url, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
            .any(|(t, s)| t.contains("code") && *s == PreviewStyle::Code));
    }

    #[test]
    fn image_placeholder_card() {
        let lines = render_preview_lines("![Diagrama](./arch.png)\n");
        assert!(
            lines.iter().any(|l| {
                l.segments
                    .iter()
                    .any(|(t, s)| *s == PreviewStyle::Image && t.contains("imagem"))
            }),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| {
            l.segments
                .iter()
                .any(|(t, s)| *s == PreviewStyle::ImageAlt && t.contains("Diagrama"))
        }));
        assert!(lines.iter().any(|l| {
            l.segments.iter().any(|(_, s)| {
                matches!(
                    s,
                    PreviewStyle::ImageMissing | PreviewStyle::ImageOk | PreviewStyle::Dim
                )
            })
        }));
    }

    #[test]
    fn image_local_exists() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("pic.png");
        std::fs::File::create(&img)
            .unwrap()
            .write_all(b"fake")
            .unwrap();
        let md = dir.path().join("doc.md");
        std::fs::write(&md, "![x](pic.png)\n").unwrap();
        let lines = render_preview_lines_in("![x](pic.png)\n", Some(dir.path()));
        assert!(
            lines
                .iter()
                .any(|l| { l.segments.iter().any(|(_, s)| *s == PreviewStyle::ImageOk) }),
            "{lines:?}"
        );
    }

    #[test]
    fn remote_image_note() {
        let lines = render_preview_lines("![logo](https://example.com/a.png)\n");
        assert!(lines.iter().any(|l| {
            l.segments
                .iter()
                .any(|(t, _)| t.contains("URL remota") || t.contains("https://"))
        }));
    }

    #[test]
    fn task_list_and_table() {
        let src = "- [x] done\n- [ ] todo\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let lines = render_preview_lines(src);
        assert!(lines.iter().any(|l| {
            l.segments
                .iter()
                .any(|(t, _)| t.contains('☑') || t.contains('☐'))
        }));
        assert!(lines
            .iter()
            .any(|l| l.segments.iter().any(|(_, s)| *s == PreviewStyle::Table)));
    }

    #[test]
    fn link_and_strike() {
        let lines = render_preview_lines("see [docs](./a.md) and ~~old~~\n");
        assert!(lines[0]
            .segments
            .iter()
            .any(|(t, s)| t == "docs" && *s == PreviewStyle::Link));
        assert!(lines[0]
            .segments
            .iter()
            .any(|(t, s)| t == "old" && *s == PreviewStyle::Strike));
    }

    #[test]
    fn inline_image_compact() {
        let lines = render_preview_lines("antes ![a](b.png) depois\n");
        assert!(lines[0]
            .segments
            .iter()
            .any(|(t, s)| *s == PreviewStyle::Image && t.contains('🖼')));
    }
}
