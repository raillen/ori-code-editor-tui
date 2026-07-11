//! Tema UI mínimo (P0.2 — cores fixas; TOML chega em P0.3).

use ratatui::style::{Color, Style};

#[derive(Debug, Clone, Copy)]
pub struct UiTheme {
    pub background: Color,
    pub foreground: Color,
    pub line_number: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub status_dirty: Color,
    pub cursor_bg: Color,
    pub cursor_fg: Color,
    pub gutter_width: u16,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Reset,
            line_number: Color::DarkGray,
            status_bg: Color::DarkGray,
            status_fg: Color::White,
            status_dirty: Color::Yellow,
            cursor_bg: Color::White,
            cursor_fg: Color::Black,
            gutter_width: 5,
        }
    }
}

impl UiTheme {
    #[must_use]
    pub fn editor_style(self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }

    #[must_use]
    pub fn gutter_style(self) -> Style {
        Style::default().fg(self.line_number).bg(self.background)
    }

    #[must_use]
    pub fn status_style(self) -> Style {
        Style::default().fg(self.status_fg).bg(self.status_bg)
    }

    #[must_use]
    pub fn cursor_style(self) -> Style {
        Style::default().fg(self.cursor_fg).bg(self.cursor_bg)
    }
}
