//! Painel da árvore de projeto.

use oride_fs::{file_icon, TreeRow};
use oride_git::GitFileStatus;
use ratatui::layout::Rect;
use ratatui::style::Style;
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
    let border = if view.focused {
        Borders::ALL
    } else {
        Borders::RIGHT
    };
    let title = if view.focused {
        format!(" {} [foco] ", view.title)
    } else {
        format!(" {} ", view.title)
    };
    let block = Block::default()
        .title(title)
        .borders(border)
        .style(theme.editor_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    if visible == 0 {
        return;
    }
    let row_count = view.rows.len();
    let start = if row_count == 0 {
        0
    } else {
        view.scroll.min(row_count.saturating_sub(1))
    };

    let mut lines = Vec::new();
    for row_i in 0..visible {
        let idx = start + row_i;
        if idx >= row_count {
            lines.push(Line::from(""));
            continue;
        }
        let row = &view.rows[idx];
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
                "▾ "
            } else {
                "▸ "
            }
        } else {
            "  "
        };

        let text = format!("{indent}{marker}{icon} {}{git_badge}", row.name);
        let style = if idx == view.selected {
            if view.focused {
                theme.tree_selection_focused()
            } else {
                theme.tree_selection_unfocused()
            }
        } else if git_badge.contains('M') {
            Style::default().fg(theme.status_dirty)
        } else {
            theme.editor_style()
        };
        lines.push(Line::from(Span::styled(text, style)));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
