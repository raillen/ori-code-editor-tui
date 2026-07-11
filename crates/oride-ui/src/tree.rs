//! Painel da árvore de projeto.

use oride_fs::{file_icon, TreeRow};
use oride_git::GitFileStatus;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

pub struct TreeView<'a> {
    pub title: &'a str,
    pub rows: &'a [TreeRow],
    pub selected: usize,
    pub scroll: usize,
    pub use_nerd_icons: bool,
    pub git: &'a std::collections::HashMap<std::path::PathBuf, GitFileStatus>,
    pub workspace_root: &'a std::path::Path,
    pub focused: bool,
}

pub fn render_tree(frame: &mut Frame, area: Rect, view: &TreeView<'_>, theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let border_style = if view.focused {
        Style::default()
            .fg(Color::Cyan)
            .bg(theme.background)
            .add_modifier(Modifier::BOLD)
    } else {
        theme.editor_style()
    };

    let title = if view.focused {
        format!(" {} ● FOCO ", view.title)
    } else {
        format!(" {} ", view.title)
    };

    let block = Block::default()
        .title(title)
        .borders(if view.focused {
            Borders::ALL
        } else {
            Borders::RIGHT | Borders::TOP | Borders::BOTTOM
        })
        .border_style(border_style)
        .style(theme.editor_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let row_width = inner.width as usize;
    if visible == 0 || row_width == 0 {
        return;
    }

    let row_count = view.rows.len();
    let start = if row_count == 0 {
        0
    } else {
        view.scroll.min(row_count.saturating_sub(1))
    };

    let mut lines = Vec::new();
    let mut cursor_pos: Option<Position> = None;

    for row_i in 0..visible {
        let idx = start + row_i;
        if idx >= row_count {
            // linha vazia com fundo do painel
            lines.push(Line::from(Span::styled(
                " ".repeat(row_width),
                theme.editor_style(),
            )));
            continue;
        }

        let row = &view.rows[idx];
        let selected = idx == view.selected;
        let icon = file_icon(&row.path, row.is_dir, row.expanded, view.use_nerd_icons);
        let indent = "  ".repeat(row.depth);
        let rel = row
            .path
            .strip_prefix(view.workspace_root)
            .unwrap_or(&row.path);
        let git_badge = view
            .git
            .get(rel)
            .or_else(|| view.git.get(&row.path))
            .map(|s| format!(" {}", s.badge()))
            .unwrap_or_default();

        let marker = if row.is_dir {
            if row.expanded {
                "▾"
            } else {
                "▸"
            }
        } else {
            "·"
        };

        // Prefixo de seleção bem visível
        let cursor_mark = if selected {
            if view.focused {
                "▶ "
            } else {
                "▷ "
            }
        } else {
            "  "
        };

        // Linha inteira preenchida — highlight visível mesmo com nome curto
        let text = pad_row(
            &format!(
                "{cursor_mark}{indent}{marker} {icon} {}{git_badge}",
                row.name
            ),
            row_width,
        );

        let style = if selected {
            if view.focused {
                theme.tree_selection_focused()
            } else {
                theme.tree_selection_unfocused()
            }
        } else if !git_badge.is_empty() && git_badge.contains('M') {
            Style::default().fg(Color::Yellow).bg(theme.background)
        } else if row.is_dir {
            Style::default()
                .fg(Color::Cyan)
                .bg(theme.background)
                .add_modifier(Modifier::BOLD)
        } else {
            theme.editor_style()
        };

        lines.push(Line::from(Span::styled(text, style)));

        if selected && view.focused {
            cursor_pos = Some(Position {
                x: inner.x,
                y: inner.y + row_i as u16,
            });
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // Cursor do terminal na linha selecionada (reforço visual)
    if let Some(pos) = cursor_pos {
        frame.set_cursor_position(pos);
    }
}

/// Preenche (ou corta) `text` para exatamente `width` colunas de caractere.
#[must_use]
pub fn pad_row(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        text.chars().take(width).collect()
    } else {
        let mut s = text.to_string();
        s.push_str(&" ".repeat(width - len));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_row_fills_width() {
        let s = pad_row("▶ src", 10);
        assert_eq!(s.chars().count(), 10);
        assert!(s.starts_with("▶ src"));
        assert!(s.ends_with(' '));
    }

    #[test]
    fn pad_row_truncates() {
        let s = pad_row("abcdefghij", 4);
        assert_eq!(s, "abcd");
    }
}
