//! Painel inferior do terminal embutido.

use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

pub fn render_terminal_panel(
    frame: &mut Frame,
    area: Rect,
    lines: &[String],
    focused: bool,
    theme: &UiTheme,
) {
    if area.height == 0 {
        return;
    }
    let title = if focused {
        " terminal (focused) "
    } else {
        " terminal "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::TOP)
        .style(theme.editor_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let max = inner.height as usize;
    let start = lines.len().saturating_sub(max);
    let visible: Vec<Line> = lines[start..]
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();
    frame.render_widget(Paragraph::new(visible).style(theme.editor_style()), inner);
}
