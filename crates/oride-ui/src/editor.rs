//! Viewport do buffer com gutter, caret, seleção multi-linha, soft wrap e syntax.

use oride_core::{Buffer, Caret, Selection};
use oride_syntax::{line_spans, HighlightKind, HighlightSpan};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

#[derive(Debug, Clone)]
pub struct EditorView<'a> {
    pub buffer: &'a Buffer,
    pub caret: Caret,
    /// Seleção atual (anchor/head em bytes).
    pub selection: Selection,
    /// Cursores extras (multi-cursor), como carets linha/coluna.
    pub extra_carets: &'a [Caret],
    /// Primeira **linha lógica** visível (0-based).
    pub scroll_y: usize,
    pub show_line_numbers: bool,
    pub highlights: &'a [HighlightSpan],
    pub show_cursor: bool,
    /// Quebra visual de linhas longas (Markdown default).
    pub soft_wrap: bool,
    /// Borda do painel quando em split focado.
    pub focused_pane: bool,
}

struct CursorPaint {
    spans: Vec<Span<'static>>,
    /// Coluna do caret primário no chunk (para set_cursor_position).
    primary_cursor_col: Option<u16>,
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

    let border = if view.focused_pane {
        Borders::ALL
    } else {
        Borders::NONE
    };
    let block = Block::default()
        .borders(border)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(theme.editor_style());
    let inner = if view.focused_pane {
        let i = block.inner(area);
        frame.render_widget(block, area);
        i
    } else {
        area
    };

    let gutter = if view.show_line_numbers {
        theme.gutter_width.min(inner.width)
    } else {
        0
    };
    let text_width = inner.width.saturating_sub(gutter) as usize;
    if text_width == 0 {
        return;
    }
    let visible_rows = inner.height as usize;
    let line_count = view.buffer.line_count().max(1);
    let start = view.scroll_y.min(line_count.saturating_sub(1));

    let sel = SelPaint::from_selection(view.selection);
    let extra_cols_on_line: Vec<(usize, usize)> = view
        .extra_carets
        .iter()
        .map(|c| (c.line, c.column))
        .collect();

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

        let primary_on_line = view.show_cursor && logical == view.caret.line;
        let extras_on_line: Vec<usize> = extra_cols_on_line
            .iter()
            .filter(|(l, _)| *l == logical)
            .map(|(_, c)| *c)
            .collect();

        let chunks = if view.soft_wrap {
            wrap_chunks(&content, text_width)
        } else if primary_on_line {
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
            } else if primary_on_line {
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

            let caret_in_chunk = primary_on_line
                && view.caret.column >= col_offset
                && (view.soft_wrap && view.caret.column < col_offset + text_width
                    || !view.soft_wrap
                    || wi == chunks.len() - 1 && view.caret.column >= col_offset);

            let mut spans = vec![gutter_span];
            let extra_locals: Vec<usize> = extras_on_line
                .iter()
                .filter(|c| **c >= col_offset && **c < col_offset + text_width)
                .map(|c| c.saturating_sub(col_offset))
                .collect();

            if caret_in_chunk || !extra_locals.is_empty() {
                let local_col = if caret_in_chunk {
                    Some(view.caret.column.saturating_sub(col_offset))
                } else {
                    None
                };
                let painted = paint_chunk_with_carets(
                    chunk,
                    chunk_byte,
                    local_col,
                    &extra_locals,
                    text_width,
                    view.highlights,
                    theme,
                    sel,
                );
                spans.extend(painted.spans);
                if let Some(cc) = painted.primary_cursor_col {
                    let x = inner.x + gutter + cc;
                    let y = inner.y + visual_row as u16;
                    if x < inner.x + inner.width && y < inner.y + inner.height {
                        cursor_pos = Some(Position { x, y });
                    }
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
            " ".repeat(inner.width as usize),
            theme.editor_style(),
        )));
    }

    let widget = Paragraph::new(lines).block(Block::default().style(theme.editor_style()));
    frame.render_widget(widget, inner);

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
        return paint_chars(
            content,
            line_byte,
            text_width,
            highlights,
            theme,
            None,
            &[],
            sel,
        )
        .spans;
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

#[allow(clippy::too_many_arguments)]
fn paint_chunk_with_carets(
    content: &str,
    line_byte: usize,
    primary_col: Option<usize>,
    extra_cols: &[usize],
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
        primary_col,
        extra_cols,
        sel,
    )
}

fn secondary_cursor_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

#[allow(clippy::too_many_arguments)]
fn paint_chars(
    content: &str,
    line_byte: usize,
    text_width: usize,
    highlights: &[HighlightSpan],
    theme: &UiTheme,
    primary_col: Option<usize>,
    extra_cols: &[usize],
    sel: SelPaint,
) -> CursorPaint {
    if text_width == 0 {
        return CursorPaint {
            spans: Vec::new(),
            primary_cursor_col: None,
        };
    }

    let chars: Vec<char> = content.chars().collect();
    let mut spans = Vec::new();
    let mut byte_off = line_byte;
    for (i, ch) in chars.iter().enumerate() {
        if i >= text_width {
            break;
        }
        let kind = kind_at(byte_off, highlights);
        let style = if primary_col == Some(i) {
            theme.cursor_style()
        } else if extra_cols.contains(&i) {
            secondary_cursor_style()
        } else if sel.contains(byte_off) {
            selection_style(theme)
        } else {
            theme.syntax_style(kind)
        };
        spans.push(Span::styled(ch.to_string(), style));
        byte_off += ch.len_utf8();
    }

    let mut painted = chars.len().min(text_width);
    let eol_primary = primary_col.is_some_and(|c| c >= chars.len());
    let eol_extra = extra_cols.iter().any(|&c| c >= chars.len());
    if (eol_primary || eol_extra) && painted < text_width {
        let style = if eol_primary {
            theme.cursor_style()
        } else {
            secondary_cursor_style()
        };
        spans.push(Span::styled(" ".to_string(), style));
        painted += 1;
    }
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
        spans.push(Span::styled(
            " ".to_string(),
            if primary_col.is_some() {
                theme.cursor_style()
            } else {
                theme.editor_style()
            },
        ));
        if text_width > 1 {
            spans.push(Span::styled(
                " ".repeat(text_width - 1),
                theme.editor_style(),
            ));
        }
    }

    let primary_cursor_col =
        primary_col.map(|c| (c as u16).min(text_width.saturating_sub(1) as u16));
    CursorPaint {
        spans,
        primary_cursor_col,
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
        let painted = paint_chunk_with_carets("hello", 0, Some(2), &[], 20, &[], &theme, sel);
        assert_eq!(painted.primary_cursor_col, Some(2));
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
        let painted = paint_chunk_with_carets("hello", 0, Some(0), &[], 20, &[], &theme, sel);
        assert_eq!(painted.spans[1].style.bg, Some(Color::Blue));
        assert_eq!(painted.spans[2].style.bg, Some(Color::Blue));
        assert_eq!(painted.spans[3].style.bg, Some(Color::Blue));
        let _ = ByteOffset::new(0);
    }
}
