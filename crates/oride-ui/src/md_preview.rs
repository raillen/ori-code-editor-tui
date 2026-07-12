//! Painel de preview Markdown (linhas semânticas → ratatui).

use oride_syntax::{PreviewLine, PreviewStyle};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::UiTheme;

pub struct MdPreviewView<'a> {
    pub title: &'a str,
    pub lines: &'a [PreviewLine],
    pub scroll: usize,
}

pub fn render_md_preview(frame: &mut Frame, area: Rect, view: &MdPreviewView<'_>, theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let block = Block::default()
        .title(format!(" {} ", view.title))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .style(theme.editor_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let width = inner.width as usize;
    if visible == 0 || width == 0 {
        return;
    }

    let start = view.scroll.min(view.lines.len().saturating_sub(1));
    let mut rat_lines = Vec::with_capacity(visible);
    for i in 0..visible {
        let idx = start + i;
        if idx >= view.lines.len() {
            rat_lines.push(Line::from(Span::styled(
                " ".repeat(width),
                theme.editor_style(),
            )));
            continue;
        }
        let pl = &view.lines[idx];
        let mut spans = Vec::new();
        let mut used = 0usize;
        for (text, style) in &pl.segments {
            if used >= width {
                break;
            }
            let remain = width - used;
            let chunk: String = text.chars().take(remain).collect();
            used += chunk.chars().count();
            spans.push(Span::styled(chunk, map_style(*style, theme)));
        }
        if used < width {
            spans.push(Span::styled(" ".repeat(width - used), theme.editor_style()));
        }
        if spans.is_empty() {
            spans.push(Span::styled(" ".repeat(width), theme.editor_style()));
        }
        rat_lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(rat_lines).wrap(Wrap { trim: false }), inner);
}

fn map_style(style: PreviewStyle, theme: &UiTheme) -> Style {
    match style {
        PreviewStyle::Normal => theme.editor_style(),
        PreviewStyle::Heading(1) => Style::default()
            .fg(theme.syntax.heading)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        PreviewStyle::Heading(_) => Style::default()
            .fg(theme.syntax.heading)
            .add_modifier(Modifier::BOLD),
        PreviewStyle::Bold => Style::default()
            .fg(theme.syntax.strong)
            .add_modifier(Modifier::BOLD),
        PreviewStyle::Italic => Style::default()
            .fg(theme.syntax.emphasis)
            .add_modifier(Modifier::ITALIC),
        PreviewStyle::Code => Style::default().fg(theme.syntax.code).bg(Color::Black),
        PreviewStyle::Link => Style::default()
            .fg(theme.syntax.link)
            .add_modifier(Modifier::UNDERLINED),
        PreviewStyle::Quote => Style::default().fg(theme.syntax.quote),
        PreviewStyle::ListMarker => Style::default()
            .fg(theme.syntax.list_marker)
            .add_modifier(Modifier::BOLD),
        PreviewStyle::Hr => Style::default().fg(Color::DarkGray),
        PreviewStyle::FenceLang => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    }
}
