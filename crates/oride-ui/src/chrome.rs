//! Chrome UX: menu bar, context banner, which-key, mini-modal.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;
use crate::tree::pad_row;

// ── Context banner ──────────────────────────────────────────────

pub fn render_context_banner(frame: &mut Frame, area: Rect, focus_label: &str, hint: &str) {
    if area.height == 0 {
        return;
    }
    let w = area.width as usize;
    let text = pad_row(&format!(" {focus_label}  ·  {hint} "), w);
    let style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), area);
}

// ── Menu bar ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    /// Atalho exibido à direita (só visual).
    pub shortcut: String,
    /// Id estável: action name ou `plugin:word_count`.
    pub action_id: String,
}

#[derive(Debug, Clone)]
pub struct MenuColumn {
    pub title: String,
    /// Tecla de acesso com Alt (ex. 'f' para File).
    pub hotkey: char,
    pub items: Vec<MenuItem>,
}

pub fn render_menu_bar(
    frame: &mut Frame,
    area: Rect,
    menus: &[MenuColumn],
    open_idx: Option<usize>,
) {
    if area.height == 0 {
        return;
    }
    let style = Style::default()
        .fg(Color::White)
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let mut spans = Vec::new();
    for (i, m) in menus.iter().enumerate() {
        let active = open_idx == Some(i);
        let st = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            style
        };
        spans.push(Span::styled(format!(" {} ", m.title), st));
    }
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let w = area.width as usize;
    if used < w {
        spans.push(Span::styled(" ".repeat(w - used), style));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(Block::default().style(style)),
        area,
    );
}

/// Dropdown sob o menu `open_idx`.
pub fn render_menu_dropdown(
    frame: &mut Frame,
    full: Rect,
    menus: &[MenuColumn],
    open_idx: usize,
    selected: usize,
) {
    let Some(menu) = menus.get(open_idx) else {
        return;
    };
    // posição x aproximada pelo índice
    let mut x = full.x + 1;
    for (i, m) in menus.iter().enumerate() {
        if i == open_idx {
            break;
        }
        x = x.saturating_add((m.title.chars().count() as u16) + 2);
    }
    let width = menu
        .items
        .iter()
        .map(|it| it.label.chars().count() + it.shortcut.chars().count() + 6)
        .max()
        .unwrap_or(24)
        .min(full.width.saturating_sub(2) as usize) as u16;
    let height = (menu.items.len() as u16 + 2).min(full.height.saturating_sub(1));
    let y = full.y.saturating_add(1);
    let rect = Rect::new(
        x.min(full.x + full.width.saturating_sub(width)),
        y,
        width,
        height,
    );
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let mut lines = Vec::new();
    for (i, it) in menu.items.iter().enumerate() {
        let mark = if i == selected { "▶ " } else { "  " };
        let row = format!(
            "{mark}{:<w$}  {}",
            it.label,
            it.shortcut,
            w = (inner.width as usize).saturating_sub(it.shortcut.chars().count() + 4)
        );
        let st = if i == selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };
        lines.push(Line::from(Span::styled(
            pad_row(&row, inner.width as usize),
            st,
        )));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Which-key ───────────────────────────────────────────────────

pub fn render_which_key(frame: &mut Frame, area: Rect, title: &str, rows: &[(String, String)]) {
    let height = (rows.len() as u16 + 3)
        .min(area.height.saturating_sub(2))
        .max(5);
    let width = (area.width * 3 / 5)
        .max(40)
        .min(area.width.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height.saturating_sub(height + 1);
    let rect = Rect::new(x, y, width, height);
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let mut lines = Vec::new();
    for (k, v) in rows {
        let row = format!(" {k:<16} {v}");
        lines.push(Line::from(Span::styled(
            pad_row(&row, inner.width as usize),
            Style::default().fg(Color::White).bg(Color::Black),
        )));
    }
    lines.push(Line::from(Span::styled(
        pad_row(" Esc fecha", inner.width as usize),
        Style::default().fg(Color::DarkGray).bg(Color::Black),
    )));
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Mini modal (find) ───────────────────────────────────────────

pub struct MiniModalView<'a> {
    pub title: &'a str,
    pub lines: &'a [String],
    pub selected: usize,
}

pub fn render_mini_modal(
    frame: &mut Frame,
    area: Rect,
    view: &MiniModalView<'_>,
    _theme: &UiTheme,
) {
    let width = (area.width * 2 / 3)
        .max(36)
        .min(area.width.saturating_sub(2));
    let height = (view.lines.len() as u16 + 2)
        .clamp(6, 12)
        .min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 3;
    let rect = Rect::new(x, y, width, height);
    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(format!(" {} ", view.title))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let mut lines = Vec::new();
    for (i, l) in view.lines.iter().enumerate() {
        let st = if i == view.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };
        lines.push(Line::from(Span::styled(
            pad_row(l, inner.width as usize),
            st,
        )));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── SCM list panel ──────────────────────────────────────────────

pub struct ScmItem {
    pub badge: char,
    pub path: String,
}

pub fn render_scm_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    items: &[ScmItem],
    selected: usize,
    focused: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let border = if focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let t = if focused {
        format!(" {title} · FOCO ")
    } else {
        format!(" {title} ")
    };
    let block = Block::default()
        .title(t)
        .borders(Borders::ALL)
        .border_style(border)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let h = inner.height as usize;
    let w = inner.width as usize;
    let start = selected.saturating_sub(h.saturating_sub(1));
    let mut lines = Vec::new();
    for row in 0..h {
        let idx = start + row;
        if let Some(it) = items.get(idx) {
            let mark = if idx == selected { "▶" } else { " " };
            let text = pad_row(&format!("{mark}{} {}", it.badge, it.path), w);
            let st = if idx == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White).bg(Color::Black)
            };
            lines.push(Line::from(Span::styled(text, st)));
        } else {
            lines.push(Line::from(Span::styled(
                " ".repeat(w),
                Style::default().bg(Color::Black),
            )));
        }
    }
    if items.is_empty() {
        lines = vec![Line::from(Span::styled(
            pad_row(" (working tree clean) ", w),
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ))];
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
