//! Viewport do buffer com gutter, caret e syntax highlight.

use oride_core::{Buffer, Caret};
use oride_syntax::{line_spans, HighlightKind, HighlightSpan};
use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

#[derive(Debug, Clone)]
pub struct EditorView<'a> {
    pub buffer: &'a Buffer,
    pub caret: Caret,
    /// Primeira linha visível (0-based).
    pub scroll_y: usize,
    pub show_line_numbers: bool,
    /// Spans de highlight do documento inteiro (bytes).
    pub highlights: &'a [HighlightSpan],
    /// Desenha caret + posiciona cursor do terminal (só no painel focado).
    pub show_cursor: bool,
}

/// Resultado do paint da linha do caret (posição local do cursor no texto).
struct CursorPaint {
    spans: Vec<Span<'static>>,
    /// Coluna no texto visível (0-based, após hscroll).
    cursor_col: u16,
}

pub fn render_editor(frame: &mut Frame, area: Rect, view: &EditorView<'_>, theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let gutter = if view.show_line_numbers {
        theme.gutter_width.min(area.width)
    } else {
        0
    };
    let text_width = area.width.saturating_sub(gutter) as usize;
    let visible_rows = area.height as usize;
    let line_count = view.buffer.line_count().max(1);
    let start = view.scroll_y.min(line_count.saturating_sub(1));

    let mut cursor_pos: Option<Position> = None;
    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows);

    for row in 0..visible_rows {
        let line_idx = start + row;
        if line_idx >= line_count {
            lines.push(Line::from(Span::styled(
                " ".repeat(area.width as usize),
                theme.editor_style(),
            )));
            continue;
        }

        let content = view.buffer.line_text(line_idx).unwrap_or_default();
        let line_byte = view
            .buffer
            .line_to_byte(line_idx)
            .map(|o| o.as_usize())
            .unwrap_or(0);

        let gutter_span = if gutter > 0 {
            let num = format!(
                "{:>width$} ",
                line_idx + 1,
                width = (gutter as usize).saturating_sub(1)
            );
            Span::styled(num, theme.gutter_style())
        } else {
            Span::raw("")
        };

        let is_cursor_line = view.show_cursor && line_idx == view.caret.line;
        let mut spans = vec![gutter_span];

        if is_cursor_line {
            let painted = paint_line_with_cursor(
                &content,
                line_byte,
                view.caret.column,
                text_width,
                view.highlights,
                theme,
            );
            spans.extend(painted.spans);
            let x = area.x + gutter + painted.cursor_col;
            let y = area.y + row as u16;
            if x < area.x + area.width && y < area.y + area.height {
                cursor_pos = Some(Position { x, y });
            }
        } else {
            spans.extend(paint_highlighted_line(
                &content,
                line_byte,
                text_width,
                view.highlights,
                theme,
            ));
        }

        lines.push(Line::from(spans));
    }

    let widget = Paragraph::new(lines).block(Block::default().style(theme.editor_style()));
    frame.render_widget(widget, area);

    if let Some(pos) = cursor_pos {
        frame.set_cursor_position(pos);
    }
}

fn paint_highlighted_line(
    content: &str,
    line_byte: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
) -> Vec<Span<'static>> {
    if text_width == 0 {
        return Vec::new();
    }
    let visible = truncate_to_width(content, text_width);
    let segs = if visible.is_empty() {
        Vec::new()
    } else {
        line_spans(&visible, line_byte, highlights)
    };
    let mut spans = Vec::new();
    let mut painted = 0usize;
    for (text, kind) in segs {
        if painted >= text_width {
            break;
        }
        let remain = text_width - painted;
        let chunk: String = text.chars().take(remain).collect();
        painted += chunk.chars().count();
        spans.push(Span::styled(chunk, theme.syntax_style(kind)));
    }
    if painted < text_width {
        spans.push(Span::styled(
            " ".repeat(text_width - painted),
            theme.editor_style(),
        ));
    }
    if spans.is_empty() {
        spans.push(Span::styled(" ".repeat(text_width), theme.editor_style()));
    }
    spans
}

fn paint_line_with_cursor(
    content: &str,
    line_byte: usize,
    column: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
) -> CursorPaint {
    if text_width == 0 {
        return CursorPaint {
            spans: Vec::new(),
            cursor_col: 0,
        };
    }

    let chars: Vec<char> = content.chars().collect();
    // Coluna do caret limitada ao fim da linha (caret pode ficar após último char)
    let col = column.min(chars.len());
    let hscroll = col.saturating_sub(text_width.saturating_sub(1));
    let cursor_local = (col - hscroll) as u16;

    // Kinds por caractere no slice visível
    let end = (hscroll + text_width).min(chars.len());
    let visible_chars = &chars[hscroll..end];
    let scroll_bytes: usize = chars.iter().take(hscroll).map(|c| c.len_utf8()).sum();

    let mut spans = Vec::new();
    let mut byte_off = line_byte + scroll_bytes;
    for (i, ch) in visible_chars.iter().enumerate() {
        let kind = kind_at(byte_off, highlights);
        let style = if i as u16 == cursor_local {
            theme.cursor_style()
        } else {
            theme.syntax_style(kind)
        };
        spans.push(Span::styled(ch.to_string(), style));
        byte_off += ch.len_utf8();
    }

    // Caret além do último caractere da linha (fim da linha)
    let mut painted = visible_chars.len();
    if cursor_local as usize >= painted {
        spans.push(Span::styled(" ".to_string(), theme.cursor_style()));
        painted += 1;
    }

    if painted < text_width {
        spans.push(Span::styled(
            " ".repeat(text_width - painted),
            theme.editor_style(),
        ));
    }

    // Fallback: se a linha ficou sem spans (conteúdo vazio)
    if spans.is_empty() {
        spans.push(Span::styled(" ".to_string(), theme.cursor_style()));
        if text_width > 1 {
            spans.push(Span::styled(
                " ".repeat(text_width - 1),
                theme.editor_style(),
            ));
        }
    }

    CursorPaint {
        spans,
        cursor_col: cursor_local.min(text_width.saturating_sub(1) as u16),
    }
}

fn kind_at(byte: usize, highlights: &[HighlightSpan]) -> HighlightKind {
    highlights
        .iter()
        .filter(|h| h.start <= byte && byte < h.end)
        .min_by_key(|h| h.end - h.start)
        .map(|h| h.kind)
        .unwrap_or(HighlightKind::Normal)
}

fn truncate_to_width(s: &str, width: usize) -> String {
    s.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_respects_char_count() {
        assert_eq!(truncate_to_width("abcdef", 3), "abc");
        assert_eq!(truncate_to_width("✨x", 1), "✨");
    }

    #[test]
    fn paint_empty_line_fills_width() {
        let theme = UiTheme::default();
        let spans = paint_highlighted_line("", 0, 10, &[], &theme);
        let w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(w, 10);
    }

    #[test]
    fn cursor_paint_marks_column() {
        let theme = UiTheme::default();
        let painted = paint_line_with_cursor("hello", 0, 2, 20, &[], &theme);
        assert_eq!(painted.cursor_col, 2);
        // 3º span (index 2) deve ser o caret sobre 'l'
        assert!(painted.spans.len() >= 3);
        assert_eq!(painted.spans[2].content.as_ref(), "l");
        // caret cell should not use plain editor style
        assert_ne!(painted.spans[2].style, theme.editor_style());
    }

    #[test]
    fn cursor_at_eol_on_empty_line() {
        let theme = UiTheme::default();
        let painted = paint_line_with_cursor("", 0, 0, 10, &[], &theme);
        assert_eq!(painted.cursor_col, 0);
        assert!(!painted.spans.is_empty());
    }
}
