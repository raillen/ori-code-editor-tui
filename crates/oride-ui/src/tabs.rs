//! Barra de tabs — highlight por **célula** (bg sólido da aba ativa).

use oride_core::TabSummary;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::theme::UiTheme;

fn active_tab_style() -> Style {
    // Branco sólido = máximo contraste em praticamente qualquer terminal.
    Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

fn inactive_tab_style() -> Style {
    Style::default().fg(Color::White).bg(Color::DarkGray)
}

fn bar_style() -> Style {
    Style::default().fg(Color::DarkGray).bg(Color::Black)
}

/// Pinta a barra de abas escrevendo direto no `Buffer` (bg por célula).
pub fn render_tabs(frame: &mut Frame, area: Rect, tabs: &[TabSummary], _theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    paint_tabs(frame.buffer_mut(), area, tabs);
}

fn paint_tabs(buf: &mut Buffer, area: Rect, tabs: &[TabSummary]) {
    // Fundo da barra
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ");
                cell.set_style(bar_style());
            }
        }
    }

    let y = area.y;
    let mut x = area.x;
    let end_x = area.x.saturating_add(area.width);

    if tabs.is_empty() {
        write_styled(buf, x, y, end_x, " (no tabs) ", bar_style());
        return;
    }

    for (i, tab) in tabs.iter().enumerate() {
        if x >= end_x {
            break;
        }
        if i > 0 {
            x = write_styled(buf, x, y, end_x, " ", bar_style());
        }
        let dirty = if tab.dirty { "●" } else { "" };
        let mark = if tab.active { "▸" } else { " " };
        let label = format!("{mark}{n}:{title}{dirty}", n = i + 1, title = tab.title);
        // padding interno do chip
        let chip = format!(" {label} ");
        let style = if tab.active {
            active_tab_style()
        } else {
            inactive_tab_style()
        };
        x = write_styled(buf, x, y, end_x, &chip, style);
    }
}

/// Escreve `text` a partir de `x` até `end_x` com estilo; retorna o próximo x.
fn write_styled(buf: &mut Buffer, mut x: u16, y: u16, end_x: u16, text: &str, style: Style) -> u16 {
    for ch in text.chars() {
        if x >= end_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
        // wide chars: avança 1 coluna (simplificado; suficiente p/ ASCII+BMP)
        x = x.saturating_add(1);
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use oride_core::DocumentStore;

    #[test]
    fn active_style_sets_background() {
        let s = active_tab_style();
        assert_eq!(s.bg, Some(Color::White));
        assert_eq!(s.fg, Some(Color::Black));
    }

    #[test]
    fn paints_active_cells_with_white_bg() {
        let mut store = DocumentStore::new();
        store.open_empty();
        let b = store.open_empty();
        store.set_active(b).unwrap();
        let tabs = store.tab_summaries();
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        paint_tabs(&mut buf, Rect::new(0, 0, 40, 1), &tabs);
        let mut found_white = false;
        for x in 0..40 {
            if let Some(cell) = buf.cell((x, 0)) {
                if cell.style().bg == Some(Color::White) {
                    found_white = true;
                    break;
                }
            }
        }
        assert!(found_white, "active tab must paint white background cells");
    }
}
