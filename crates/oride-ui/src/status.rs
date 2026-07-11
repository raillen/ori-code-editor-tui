//! Barra de status inferior.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

#[derive(Debug, Clone)]
pub struct StatusModel {
    pub title: String,
    pub dirty: bool,
    pub line: usize,
    pub column: usize,
    pub message: Option<String>,
    pub help_hint: String,
}

pub fn render_status(frame: &mut Frame, area: Rect, model: &StatusModel, theme: &UiTheme) {
    let dirty = if model.dirty {
        Span::styled(" ●", theme.status_style().fg(theme.status_dirty))
    } else {
        Span::raw("")
    };

    let left = Line::from(vec![
        Span::styled(format!(" {}", model.title), theme.status_style()),
        dirty,
        Span::styled(
            format!("  Ln {}, Col {}", model.line + 1, model.column + 1),
            theme.status_style(),
        ),
    ]);

    let right_text = model.message.as_deref().unwrap_or(model.help_hint.as_str());
    let right = Span::styled(format!(" {right_text} "), theme.status_style());

    // Duas faixas: principal + hint compacto se couber
    let width = area.width as usize;
    let left_s = left.width();
    let mut line = left;
    if left_s + right.width() + 1 < width {
        let pad = width.saturating_sub(left_s + right.width());
        line.spans
            .push(Span::styled(" ".repeat(pad), theme.status_style()));
        line.spans.push(right);
    }

    let widget = Paragraph::new(line).block(Block::default().style(theme.status_style()));
    frame.render_widget(widget, area);
}
