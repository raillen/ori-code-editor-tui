//! Tema UI — defaults e carga a partir de `ThemeUiConfig`.

use oride_config::ThemeUiConfig;
use ratatui::style::{Color, Style};
use thiserror::Error;

use crate::color::{parse_color, ColorParseError};

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

#[derive(Debug, Error)]
#[error("theme field `{field}`: {source}")]
pub struct ThemeBuildError {
    pub field: &'static str,
    #[source]
    pub source: ColorParseError,
}

impl UiTheme {
    /// Constrói tema a partir da seção `[ui]` da config.
    pub fn from_config(cfg: &ThemeUiConfig) -> Result<Self, ThemeBuildError> {
        Ok(Self {
            background: parse_field("background", &cfg.background)?,
            foreground: parse_field("foreground", &cfg.foreground)?,
            line_number: parse_field("line_number", &cfg.line_number)?,
            status_bg: parse_field("status_bg", &cfg.status_bg)?,
            status_fg: parse_field("status_fg", &cfg.status_fg)?,
            status_dirty: parse_field("status_dirty", &cfg.status_dirty)?,
            cursor_bg: parse_field("cursor_bg", &cfg.cursor_bg)?,
            cursor_fg: parse_field("cursor_fg", &cfg.cursor_fg)?,
            gutter_width: cfg.gutter_width.max(1),
        })
    }

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

fn parse_field(field: &'static str, value: &str) -> Result<Color, ThemeBuildError> {
    parse_color(value).map_err(|source| ThemeBuildError { field, source })
}
