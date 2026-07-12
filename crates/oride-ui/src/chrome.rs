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

// ── Mini modal genérico ─────────────────────────────────────────

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

// ── Find / Replace modal (layout espaçado, legível) ─────────────

pub struct FindModalView<'a> {
    pub query: &'a str,
    pub replace: &'a str,
    pub show_replace: bool,
    pub focus_replace: bool,
    /// Ex.: "3 / 12" ou "0 matches" ou "digite para buscar"
    pub match_label: &'a str,
    pub case_sensitive: bool,
    pub ignore_accents: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub error: Option<&'a str>,
}

pub fn render_find_modal(frame: &mut Frame, area: Rect, view: &FindModalView<'_>) {
    let width = (area.width * 3 / 4)
        .clamp(48, 72)
        .min(area.width.saturating_sub(2));
    // campos + blank + matches + blank + 4 opções + blank + 3 hints
    let content_rows: u16 = if view.show_replace { 14 } else { 13 };
    let height = (content_rows + 2)
        .min(area.height.saturating_sub(2))
        .max(12);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 4;
    let rect = Rect::new(x, y, width, height);
    frame.render_widget(Clear, rect);

    let block = Block::default()
        .title(" Find / Replace ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black).fg(Color::White));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let w = inner.width as usize;
    let dim = Style::default().fg(Color::DarkGray).bg(Color::Black);
    let normal = Style::default().fg(Color::White).bg(Color::Black);
    let focus = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let ok = Style::default().fg(Color::Green).bg(Color::Black);
    let err = Style::default()
        .fg(Color::Black)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD);
    let key = Style::default().fg(Color::Cyan).bg(Color::Black);

    let mut lines: Vec<Line> = Vec::new();

    // Campo Find
    let q_st = if view.focus_replace { normal } else { focus };
    let q_cursor = if view.focus_replace { "" } else { "▌" };
    lines.push(Line::from(vec![
        Span::styled(pad_row("  Find", 10), dim),
        Span::styled(
            pad_row(
                &format!(" {}{} ", view.query, q_cursor),
                w.saturating_sub(10),
            ),
            q_st,
        ),
    ]));

    // Campo Replace (opcional)
    if view.show_replace {
        let r_st = if view.focus_replace { focus } else { normal };
        let r_cursor = if view.focus_replace { "▌" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(pad_row("  Replace", 10), dim),
            Span::styled(
                pad_row(
                    &format!(" {}{} ", view.replace, r_cursor),
                    w.saturating_sub(10),
                ),
                r_st,
            ),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            pad_row("  (Ctrl+H ou Tab → campo Replace)", w),
            dim,
        )));
    }

    lines.push(Line::from(Span::styled(pad_row("", w), normal)));

    // Contagem de matches (linha própria, destaque)
    if let Some(e) = view.error {
        lines.push(Line::from(Span::styled(
            pad_row(&format!("  ! {e}"), w),
            err,
        )));
    } else {
        lines.push(Line::from(Span::styled(
            pad_row(&format!("  {}", view.match_label), w),
            ok,
        )));
    }

    lines.push(Line::from(Span::styled(pad_row("", w), normal)));
    lines.push(Line::from(Span::styled(
        pad_row("  Opções  (atalho liga/desliga)", w),
        dim,
    )));

    // Uma opção por linha: [x] rótulo … atalho
    for (on, label, shortcut) in [
        (view.case_sensitive, "Case sensitive", "Alt+C"),
        (view.ignore_accents, "Ignorar acentos (á≈a)", "Alt+A"),
        (view.whole_word, "Palavra completa", "Alt+W"),
        (view.use_regex, "Regex", "Alt+R"),
    ] {
        let mark = if on { "[x]" } else { "[ ]" };
        let mark_st = if on {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            dim
        };
        let left = format!("  {mark}  {label}");
        let left_w = left.chars().count();
        let right = shortcut;
        let right_w = right.chars().count();
        let gap = w.saturating_sub(left_w + right_w).max(1);
        lines.push(Line::from(vec![
            Span::styled(format!("  {mark}"), mark_st),
            Span::styled(format!("  {label}"), normal),
            Span::styled(" ".repeat(gap.saturating_sub(2)), normal),
            Span::styled(right.to_string(), key),
        ]));
    }

    lines.push(Line::from(Span::styled(pad_row("", w), normal)));
    lines.push(Line::from(Span::styled(
        pad_row("  Enter next · F3 / Shift+F3 · Tab campo", w),
        dim,
    )));
    lines.push(Line::from(Span::styled(
        pad_row("  Alt+Enter replace 1 · Ctrl+Alt+Enter all · Esc", w),
        dim,
    )));

    // Ajusta altura real ao número de linhas
    let show_n = (inner.height as usize).min(lines.len());
    frame.render_widget(Paragraph::new(lines[..show_n].to_vec()), inner);
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
