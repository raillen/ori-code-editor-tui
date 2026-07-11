//! Barra de tabs.

use oride_core::TabSummary;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::theme::UiTheme;

/// Estilo da aba **ativa**: fundo sólido de alto contraste (chip/pill).
///
/// Cores explícitas (não `Reset`) para funcionar em dark e light terminals —
/// o destaque é o **background da aba inteira**, não um marcador solto.
fn active_tab_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Aba inativa: mesmo fundo da barra, texto mais fraco.
fn inactive_tab_style() -> Style {
    Style::default().fg(Color::Gray).bg(Color::Black)
}

fn bar_style() -> Style {
    Style::default().fg(Color::DarkGray).bg(Color::Black)
}

pub fn render_tabs(frame: &mut Frame, area: Rect, tabs: &[TabSummary], _theme: &UiTheme) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // Preenche a barra inteira primeiro — evita “buracos” sem bg.
    frame.render_widget(Block::default().style(bar_style()), area);

    let mut spans: Vec<Span> = Vec::new();

    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            // Separador entre abas (não herda o fundo ciano da ativa)
            spans.push(Span::styled(" ", bar_style()));
        }

        let dirty = if tab.dirty { " ●" } else { "" };
        // Espaços laterais fazem o chip de fundo ficar largo o suficiente
        // para ler “esta é a aba ativa” sem depender de seta/símbolo.
        let label = format!("  {n}:{title}{dirty}  ", n = i + 1, title = tab.title);

        let style = if tab.active {
            active_tab_style()
        } else {
            inactive_tab_style()
        };
        spans.push(Span::styled(label, style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(" (no tabs) ", bar_style()));
    }

    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let width = area.width as usize;
    if used < width {
        spans.push(Span::styled(" ".repeat(width - used), bar_style()));
    } else if used > width {
        // Trunca a linha se as abas não cabem (char-count; suficiente p/ TUI).
        let mut truncated = Vec::new();
        let mut remain = width;
        for span in spans {
            let n = span.content.chars().count();
            if remain == 0 {
                break;
            }
            if n <= remain {
                remain -= n;
                truncated.push(span);
            } else {
                let cut: String = span.content.chars().take(remain).collect();
                truncated.push(Span::styled(cut, span.style));
                break;
            }
        }
        spans = truncated;
    }

    // Sem Block no Paragraph: o fundo já veio do fill acima; spans controlam
    // o bg de cada aba (a ativa é o chip ciano completo).
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_style_sets_background() {
        let s = active_tab_style();
        assert_eq!(s.bg, Some(Color::Cyan));
        assert_eq!(s.fg, Some(Color::Black));
    }

    #[test]
    fn inactive_differs_from_active() {
        assert_ne!(active_tab_style().bg, inactive_tab_style().bg);
    }
}
