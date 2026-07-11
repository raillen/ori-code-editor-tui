//! Viewport do buffer com gutter, caret e syntax highlight.

use oride_core::{Buffer, Caret};
use oride_syntax::{line_spans, HighlightSpan};
use ratatui::layout::Rect;
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
    let line_count = view.buffer.line_count();
    let start = view.scroll_y.min(line_count.saturating_sub(1));

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

        let is_cursor_line = line_idx == view.caret.line;
        let mut spans = vec![gutter_span];

        if is_cursor_line {
            spans.extend(paint_line_with_cursor(
                &content,
                line_byte,
                view.caret.column,
                text_width,
                view.highlights,
                theme,
            ));
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
}

fn paint_highlighted_line(
    content: &str,
    line_byte: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
) -> Vec<Span<'static>> {
    let visible = truncate_to_width(content, text_width);
    // Recorta highlights ao prefixo visível (sem hscroll em linhas sem caret)
    let segs = line_spans(&visible, line_byte, highlights);
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
) -> Vec<Span<'static>> {
    let chars: Vec<char> = content.chars().collect();
    let col = column.min(chars.len());
    let hscroll = col.saturating_sub(text_width.saturating_sub(1));
    let end = (hscroll + text_width).min(chars.len());
    let slice: String = chars[hscroll..end].iter().collect();

    // Byte offset do início da fatia visível
    let scroll_bytes: usize = chars.iter().take(hscroll).map(|c| c.len_utf8()).sum();
    let slice_start = line_byte + scroll_bytes;

    let segs = line_spans(&slice, slice_start, highlights);
    let cursor_local = col.saturating_sub(hscroll);

    let mut spans = Vec::new();
    let mut char_pos = 0usize;
    for (text, kind) in segs {
        for ch in text.chars() {
            if char_pos == cursor_local {
                spans.push(Span::styled(ch.to_string(), theme.cursor_style()));
            } else {
                spans.push(Span::styled(ch.to_string(), theme.syntax_style(kind)));
            }
            char_pos += 1;
        }
    }
    // Caret no fim da linha
    if cursor_local >= char_pos {
        spans.push(Span::styled(" ".to_string(), theme.cursor_style()));
        char_pos += 1;
    }
    if char_pos < text_width {
        spans.push(Span::styled(
            " ".repeat(text_width - char_pos),
            theme.editor_style(),
        ));
    }
    spans
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
}
