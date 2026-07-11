//! Viewport do buffer com gutter de números de linha e caret.

use oride_core::{Buffer, Caret};
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
                view.caret.column,
                text_width,
                theme,
            ));
        } else {
            let visible = truncate_to_width(&content, text_width);
            spans.push(Span::styled(visible, theme.editor_style()));
        }

        lines.push(Line::from(spans));
    }

    let widget = Paragraph::new(lines).block(Block::default().style(theme.editor_style()));
    frame.render_widget(widget, area);
}

fn paint_line_with_cursor(
    content: &str,
    column: usize,
    text_width: usize,
    theme: &UiTheme,
) -> Vec<Span<'static>> {
    let chars: Vec<char> = content.chars().collect();
    let col = column.min(chars.len());

    // Scroll horizontal simples: se caret passa da largura, desloca
    let hscroll = col.saturating_sub(text_width.saturating_sub(1));
    let end = (hscroll + text_width).min(chars.len());
    let slice = &chars[hscroll..end];

    let mut spans = Vec::new();
    let cursor_in_view = col >= hscroll && col - hscroll < text_width;

    if cursor_in_view {
        let local = col - hscroll;
        let before: String = slice.iter().take(local).collect();
        let at = slice.get(local).copied();
        let after: String = slice.iter().skip(local + 1).collect();

        if !before.is_empty() {
            spans.push(Span::styled(before, theme.editor_style()));
        }
        let cursor_ch = at.map(|c| c.to_string()).unwrap_or_else(|| " ".into());
        spans.push(Span::styled(cursor_ch, theme.cursor_style()));
        if !after.is_empty() {
            spans.push(Span::styled(after, theme.editor_style()));
        }
        if at.is_none() {
            // caret no fim da linha: já pintamos espaço
        }
    } else {
        let visible: String = slice.iter().collect();
        spans.push(Span::styled(visible, theme.editor_style()));
    }

    // Completa largura visual restante com espaços (fundo)
    let painted: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if painted < text_width {
        spans.push(Span::styled(
            " ".repeat(text_width - painted),
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
}
