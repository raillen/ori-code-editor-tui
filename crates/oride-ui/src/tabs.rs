//! Barra de tabs.

use oride_core::TabSummary;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

pub fn render_tabs(frame: &mut Frame, area: Rect, tabs: &[TabSummary], _theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let mut spans = Vec::new();
    // fundo da barra
    let bar_bg = Style::default().bg(Color::DarkGray).fg(Color::White);

    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", bar_bg));
        }
        let dirty = if tab.dirty { " ●" } else { "" };
        // número da aba (1-based) ajuda navegação
        let label = if tab.active {
            format!(" ▶{}:{}{} ", i + 1, tab.title, dirty)
        } else {
            format!("  {}:{}{} ", i + 1, tab.title, dirty)
        };
        let style = if tab.active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(" (no tabs) ", bar_bg));
    }

    // completa largura da barra
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let width = area.width as usize;
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), bar_bg));
    }

    let widget = Paragraph::new(Line::from(spans)).block(Block::default().style(bar_bg));
    frame.render_widget(widget, area);
}
