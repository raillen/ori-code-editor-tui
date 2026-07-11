//! Overlay de command palette / fuzzy open.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

pub struct PaletteView<'a> {
    pub title: &'a str,
    pub query: &'a str,
    pub items: &'a [String],
    pub selected: usize,
}

pub fn render_palette(frame: &mut Frame, area: Rect, view: &PaletteView<'_>, theme: &UiTheme) {
    // Centro 60% x 50%
    let width = (area.width * 3 / 5).max(30).min(area.width);
    let height = (area.height / 2).max(8).min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 3;
    let rect = Rect::new(x, y, width, height);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(format!(" {} ", view.title))
        .borders(Borders::ALL)
        .style(theme.status_style());
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("> ", theme.status_style()),
        Span::styled(
            view.query,
            theme.status_style().add_modifier(Modifier::BOLD),
        ),
        Span::styled("▌", theme.status_style()),
    ]));
    frame.render_widget(prompt, chunks[0]);

    let list_h = chunks[1].height as usize;
    let start = view.selected.saturating_sub(list_h.saturating_sub(1));
    let mut lines = Vec::new();
    for (i, item) in view.items.iter().enumerate().skip(start).take(list_h) {
        let style = if i == view.selected {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.editor_style()
        };
        lines.push(Line::from(Span::styled(format!(" {item}"), style)));
    }
    frame.render_widget(Paragraph::new(lines), chunks[1]);
}
