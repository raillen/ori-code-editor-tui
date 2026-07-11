//! Barra de tabs.

use oride_core::TabSummary;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

pub fn render_tabs(frame: &mut Frame, area: Rect, tabs: &[TabSummary], theme: &UiTheme) {
    if area.height == 0 {
        return;
    }
    let mut spans = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", theme.editor_style()));
        }
        let dirty = if tab.dirty { " ●" } else { "" };
        let label = format!(" {}{} ", tab.title, dirty);
        let style = if tab.active {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.editor_style()
        };
        spans.push(Span::styled(label, style));
    }
    if spans.is_empty() {
        spans.push(Span::styled(" (no tabs) ", theme.gutter_style()));
    }
    let widget =
        Paragraph::new(Line::from(spans)).block(Block::default().style(theme.editor_style()));
    frame.render_widget(widget, area);
}
