//! Barra de status inferior — estável + mensagem efêmera.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;
use crate::tree::pad_row;

#[derive(Debug, Clone)]
pub struct StatusModel {
    pub title: String,
    pub dirty: bool,
    pub line: usize,
    pub column: usize,
    pub git_branch: Option<String>,
    pub blame: Option<String>,
    pub message: Option<String>,
    /// Hint permanente (atalhos essenciais).
    pub help_hint: String,
}

pub fn render_status(frame: &mut Frame, area: Rect, model: &StatusModel, theme: &UiTheme) {
    if area.height == 0 {
        return;
    }
    let w = area.width as usize;
    let dirty = if model.dirty { "●" } else { " " };
    let branch = model
        .git_branch
        .as_deref()
        .map(|b| format!("  git:{b}"))
        .unwrap_or_default();
    let blame = model
        .blame
        .as_deref()
        .map(|b| format!("  {b}"))
        .unwrap_or_default();

    // Linha estável à esquerda; mensagem ou hint à direita
    let left = format!(
        " {}{}  Ln {}, Col {}{}{} ",
        model.title,
        dirty,
        model.line + 1,
        model.column + 1,
        branch,
        blame
    );
    let right = model
        .message
        .clone()
        .unwrap_or_else(|| model.help_hint.clone());

    let left_w = left.chars().count();
    let right_w = right.chars().count();
    let mut spans = vec![Span::styled(left, theme.status_style())];
    if left_w + right_w + 1 < w {
        let pad = w - left_w - right_w;
        spans.push(Span::styled(" ".repeat(pad), theme.status_style()));
        let rst = if model.message.is_some() {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.status_style()
        };
        spans.push(Span::styled(right, rst));
    } else {
        spans = vec![Span::styled(
            pad_row(
                &format!(
                    " {}{} L{}:{}  {}",
                    model.title,
                    dirty,
                    model.line + 1,
                    model.column + 1,
                    model.message.as_deref().unwrap_or("")
                ),
                w,
            ),
            theme.status_style(),
        )];
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(Block::default().style(theme.status_style())),
        area,
    );
}
