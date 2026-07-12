//! Viewport do buffer com gutter, caret, seleção multi-linha, soft wrap e syntax.

use oride_core::{Buffer, Caret, Selection};
use oride_syntax::{line_spans, HighlightKind, HighlightSpan};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

#[derive(Debug, Clone)]
pub struct EditorView<'a> {
    pub buffer: &'a Buffer,
    pub caret: Caret,
    /// Seleção atual (anchor/head em bytes).
    pub selection: Selection,
    /// Primeira **linha lógica** visível (0-based).
    pub scroll_y: usize,
    pub show_line_numbers: bool,
    pub highlights: &'a [HighlightSpan],
    pub show_cursor: bool,
    /// Quebra visual de linhas longas (Markdown default).
    pub soft_wrap: bool,
}

struct CursorPaint {
    spans: Vec<Span<'static>>,
    cursor_col: u16,
}

#[derive(Clone, Copy)]
struct SelPaint {
    active: bool,
    start: usize,
    end: usize,
}

impl SelPaint {
    fn from_selection(sel: Selection) -> Self {
        Self {
            active: !sel.is_empty(),
            start: sel.start().as_usize(),
            end: sel.end().as_usize(),
        }
    }

    fn contains(self, byte: usize) -> bool {
        self.active && byte >= self.start && byte < self.end
    }
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
    if text_width == 0 {
        return;
    }
    let visible_rows = area.height as usize;
    let line_count = view.buffer.line_count().max(1);
    let start = view.scroll_y.min(line_count.saturating_sub(1));

    let sel = SelPaint::from_selection(view.selection);

    let mut cursor_pos: Option<Position> = None;
    let mut lines: Vec<Line> = Vec::with_capacity(visible_rows);
    let mut visual_row = 0usize;
    let mut logical = start;

    while visual_row < visible_rows && logical < line_count {
        let content = view.buffer.line_text(logical).unwrap_or_default();
        let line_byte = view
            .buffer
            .line_to_byte(logical)
            .map(|o| o.as_usize())
            .unwrap_or(0);

        let chunks = if view.soft_wrap {
            wrap_chunks(&content, text_width)
        } else if view.show_cursor && logical == view.caret.line {
            let col = view.caret.column.min(content.chars().count());
            let hscroll = col.saturating_sub(text_width.saturating_sub(1));
            vec![chunk_from_col(&content, hscroll, text_width)]
        } else {
            vec![truncate_to_width(&content, text_width)]
        };

        for (wi, chunk) in chunks.iter().enumerate() {
            if visual_row >= visible_rows {
                break;
            }

            let col_offset: usize = if view.soft_wrap {
                wi * text_width
            } else if view.show_cursor && logical == view.caret.line {
                let col = view.caret.column.min(content.chars().count());
                col.saturating_sub(text_width.saturating_sub(1))
            } else {
                0
            };

            let chunk_byte = line_byte
                + content
                    .chars()
                    .take(col_offset)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();

            let gutter_span = if gutter > 0 {
                let num = if wi == 0 {
                    format!(
                        "{:>width$} ",
                        logical + 1,
                        width = (gutter as usize).saturating_sub(1)
                    )
                } else {
                    format!(
                        "{:>width$} ",
                        "",
                        width = (gutter as usize).saturating_sub(1)
                    )
                };
                Span::styled(num, theme.gutter_style())
            } else {
                Span::raw("")
            };

            let on_cursor_line = view.show_cursor && logical == view.caret.line;
            let caret_in_chunk = on_cursor_line
                && view.caret.column >= col_offset
                && (view.soft_wrap && view.caret.column < col_offset + text_width
                    || !view.soft_wrap
                    || wi == chunks.len() - 1 && view.caret.column >= col_offset);

            let mut spans = vec![gutter_span];
            if on_cursor_line && caret_in_chunk {
                let local_col = view.caret.column.saturating_sub(col_offset);
                let painted = paint_chunk_with_cursor(
                    chunk,
                    chunk_byte,
                    local_col,
                    text_width,
                    view.highlights,
                    theme,
                    sel,
                );
                spans.extend(painted.spans);
                let x = area.x + gutter + painted.cursor_col;
                let y = area.y + visual_row as u16;
                if x < area.x + area.width && y < area.y + area.height {
                    cursor_pos = Some(Position { x, y });
                }
            } else {
                spans.extend(paint_highlighted_line(
                    chunk,
                    chunk_byte,
                    text_width,
                    view.highlights,
                    theme,
                    sel,
                ));
            }

            lines.push(Line::from(spans));
            visual_row += 1;
        }
        logical += 1;
    }

    while lines.len() < visible_rows {
        lines.push(Line::from(Span::styled(
            " ".repeat(area.width as usize),
            theme.editor_style(),
        )));
    }

    let widget = Paragraph::new(lines).block(Block::default().style(theme.editor_style()));
    frame.render_widget(widget, area);

    if let Some(pos) = cursor_pos {
        frame.set_cursor_position(pos);
    }
}

fn selection_style(_theme: &UiTheme) -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

fn wrap_chunks(content: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if content.is_empty() {
        return vec![String::new()];
    }
    let chars: Vec<char> = content.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + width).min(chars.len());
        out.push(chars[i..end].iter().collect());
        i = end;
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn chunk_from_col(content: &str, start_col: usize, width: usize) -> String {
    content.chars().skip(start_col).take(width).collect()
}

fn paint_highlighted_line(
    content: &str,
    line_byte: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
    sel: SelPaint,
) -> Vec<Span<'static>> {
    if text_width == 0 {
        return Vec::new();
    }
    // Pintura char-a-char quando há seleção para bg contínuo multi-linha
    if sel.active {
        return paint_chars(content, line_byte, text_width, highlights, theme, None, sel).spans;
    }
    let segs = if content.is_empty() {
        Vec::new()
    } else {
        line_spans(content, line_byte, highlights)
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

fn paint_chunk_with_cursor(
    content: &str,
    line_byte: usize,
    column: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
    sel: SelPaint,
) -> CursorPaint {
    paint_chars(
        content,
        line_byte,
        text_width,
        highlights,
        theme,
        Some(column),
        sel,
    )
}

fn paint_chars(
    content: &str,
    line_byte: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
    cursor_col: Option<usize>,
    sel: SelPaint,
) -> CursorPaint {
    if text_width == 0 {
        return CursorPaint {
            spans: Vec::new(),
            cursor_col: 0,
        };
    }

    let chars: Vec<char> = content.chars().collect();
    let col = cursor_col.unwrap_or(usize::MAX).min(chars.len());
    let cursor_local = if cursor_col.is_some() { col as u16 } else { 0 };

    let mut spans = Vec::new();
    let mut byte_off = line_byte;
    for (i, ch) in chars.iter().enumerate() {
        if i >= text_width {
            break;
        }
        let kind = kind_at(byte_off, highlights);
        let style = if cursor_col.is_some() && i == col {
            theme.cursor_style()
        } else if sel.contains(byte_off) {
            selection_style(theme)
        } else {
            theme.syntax_style(kind)
        };
        spans.push(Span::styled(ch.to_string(), style));
        byte_off += ch.len_utf8();
    }

    let mut painted = chars.len().min(text_width);
    if cursor_col.is_some() && col >= chars.len() && painted < text_width {
        let style = if sel.contains(byte_off) {
            selection_style(theme)
        } else {
            theme.cursor_style()
        };
        spans.push(Span::styled(" ".to_string(), style));
        painted += 1;
    }
    // Preenche resto da linha: se a seleção cruza o fim da linha (multi-linha),
    // pinta o padding com cor de seleção (estilo VS Code).
    if painted < text_width {
        let pad = text_width - painted;
        let rest_selected = sel.contains(byte_off);
        let style = if rest_selected {
            selection_style(theme)
        } else {
            theme.editor_style()
        };
        spans.push(Span::styled(" ".repeat(pad), style));
    }
    if spans.is_empty() {
        let style = if cursor_col.is_some() {
            theme.cursor_style()
        } else {
            theme.editor_style()
        };
        spans.push(Span::styled(" ".to_string(), style));
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
    use oride_core::ByteOffset;

    #[test]
    fn truncate_respects_char_count() {
        assert_eq!(truncate_to_width("abcdef", 3), "abc");
        assert_eq!(truncate_to_width("✨x", 1), "✨");
    }

    #[test]
    fn wrap_chunks_splits() {
        let w = wrap_chunks("abcdefghij", 4);
        assert_eq!(w, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn paint_empty_line_fills_width() {
        let theme = UiTheme::default();
        let sel = SelPaint {
            active: false,
            start: 0,
            end: 0,
        };
        let spans = paint_highlighted_line("", 0, 10, &[], &theme, sel);
        let w: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        assert_eq!(w, 10);
    }

    #[test]
    fn cursor_paint_marks_column() {
        let theme = UiTheme::default();
        let sel = SelPaint {
            active: false,
            start: 0,
            end: 0,
        };
        let painted = paint_chunk_with_cursor("hello", 0, 2, 20, &[], &theme, sel);
        assert_eq!(painted.cursor_col, 2);
        assert_eq!(painted.spans[2].content.as_ref(), "l");
    }

    #[test]
    fn selection_paints_blue_background() {
        let theme = UiTheme::default();
        // seleciona "ell" em "hello" (bytes 1..4)
        let sel = SelPaint {
            active: true,
            start: 1,
            end: 4,
        };
        let painted = paint_chunk_with_cursor("hello", 0, 0, 20, &[], &theme, sel);
        assert_eq!(painted.spans[1].style.bg, Some(Color::Blue));
        assert_eq!(painted.spans[2].style.bg, Some(Color::Blue));
        assert_eq!(painted.spans[3].style.bg, Some(Color::Blue));
        let _ = ByteOffset::new(0);
    }
}
