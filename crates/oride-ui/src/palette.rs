//! Overlay de command palette / browser / find.

use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;
use crate::tree::pad_row;

pub struct PaletteView<'a> {
    pub title: &'a str,
    pub query: &'a str,
    pub items: &'a [String],
    pub selected: usize,
    /// Texto de ajuda sob a lista (atalhos do modal).
    pub hint: &'a str,
}

impl<'a> PaletteView<'a> {
    #[must_use]
    pub fn simple(title: &'a str, query: &'a str, items: &'a [String], selected: usize) -> Self {
        Self {
            title,
            query,
            items,
            selected,
            hint: "",
        }
    }
}

pub fn render_palette(frame: &mut Frame, area: Rect, view: &PaletteView<'_>, _theme: &UiTheme) {
    let width = (area.width * 4 / 5)
        .max(40)
        .min(area.width.saturating_sub(2));
    let height = (area.height * 2 / 3)
        .max(10)
        .min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 4;
    let rect = Rect::new(x, y, width, height);

    frame.render_widget(Clear, rect);

    let block = Block::default()
        .title(format!(" {} ", view.title))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let hint_h = if view.hint.is_empty() { 0 } else { 1 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(hint_h),
        ])
        .split(inner);

    let prompt_w = chunks[0].width as usize;
    let prompt_text = pad_row(&format!("> {}▌", view.query), prompt_w);
    let prompt = Paragraph::new(Line::from(Span::styled(
        prompt_text,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Gray)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(prompt, chunks[0]);

    let list_area = chunks[1];
    let list_h = list_area.height as usize;
    let list_w = list_area.width as usize;
    if list_h == 0 || list_w == 0 {
        return;
    }

    let start = if view.items.is_empty() {
        0
    } else {
        view.selected
            .saturating_sub(list_h.saturating_sub(1))
            .min(view.items.len().saturating_sub(1))
    };

    let mut lines = Vec::with_capacity(list_h);
    let mut cursor_pos: Option<Position> = None;

    for row in 0..list_h {
        let idx = start + row;
        if idx >= view.items.len() {
            lines.push(Line::from(Span::styled(
                " ".repeat(list_w),
                Style::default().bg(Color::Black).fg(Color::DarkGray),
            )));
            continue;
        }
        let selected = idx == view.selected;
        let mark = if selected { "▶ " } else { "  " };
        let text = pad_row(&format!("{mark}{}", view.items[idx]), list_w);
        let style = if selected {
            // Linha inteira ciano — alto contraste (igual à árvore)
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };
        lines.push(Line::from(Span::styled(text, style)));
        if selected {
            cursor_pos = Some(Position {
                x: list_area.x,
                y: list_area.y + row as u16,
            });
        }
    }

    frame.render_widget(Paragraph::new(lines), list_area);

    if hint_h > 0 {
        let hint = pad_row(view.hint, chunks[2].width as usize);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(Color::Yellow).bg(Color::Black),
            ))),
            chunks[2],
        );
    }

    if let Some(pos) = cursor_pos {
        frame.set_cursor_position(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_keeps_width() {
        let s = pad_row("▶ item", 12);
        assert_eq!(s.chars().count(), 12);
    }
}
