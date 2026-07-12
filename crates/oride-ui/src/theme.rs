//! Tema UI — defaults, config TOML e cores de syntax.

use oride_config::{SyntaxColorsConfig, ThemeUiConfig};
use oride_syntax::HighlightKind;
use ratatui::style::{Color, Modifier, Style};
use thiserror::Error;

use crate::color::{parse_color, ColorParseError};

#[derive(Debug, Clone, Copy)]
pub struct SyntaxTheme {
    pub comment: Color,
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub type_name: Color,
    pub function: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub variable: Color,
    pub constant: Color,
    pub property: Color,
    pub tag: Color,
    pub attribute: Color,
    // Markdown
    pub heading: Color,
    pub emphasis: Color,
    pub strong: Color,
    pub link: Color,
    pub code: Color,
    pub list_marker: Color,
    pub quote: Color,
}

impl Default for SyntaxTheme {
    fn default() -> Self {
        Self::from_config(&SyntaxColorsConfig::default()).unwrap_or(Self {
            comment: Color::DarkGray,
            keyword: Color::Magenta,
            string: Color::Green,
            number: Color::Yellow,
            type_name: Color::Cyan,
            function: Color::Blue,
            operator: Color::Reset,
            punctuation: Color::DarkGray,
            variable: Color::Reset,
            constant: Color::Yellow,
            property: Color::Cyan,
            tag: Color::Red,
            attribute: Color::Yellow,
            heading: Color::Magenta,
            emphasis: Color::Cyan,
            strong: Color::Yellow,
            link: Color::Blue,
            code: Color::Green,
            list_marker: Color::Yellow,
            quote: Color::DarkGray,
        })
    }
}

impl SyntaxTheme {
    pub fn from_config(cfg: &SyntaxColorsConfig) -> Result<Self, ThemeBuildError> {
        Ok(Self {
            comment: parse_field("syntax.comment", &cfg.comment)?,
            keyword: parse_field("syntax.keyword", &cfg.keyword)?,
            string: parse_field("syntax.string", &cfg.string)?,
            number: parse_field("syntax.number", &cfg.number)?,
            type_name: parse_field("syntax.type_name", &cfg.type_name)?,
            function: parse_field("syntax.function", &cfg.function)?,
            operator: parse_field("syntax.operator", &cfg.operator)?,
            punctuation: parse_field("syntax.punctuation", &cfg.punctuation)?,
            variable: parse_field("syntax.variable", &cfg.variable)?,
            constant: parse_field("syntax.constant", &cfg.constant)?,
            property: parse_field("syntax.property", &cfg.property)?,
            tag: parse_field("syntax.tag", &cfg.tag)?,
            attribute: parse_field("syntax.attribute", &cfg.attribute)?,
            heading: parse_field("syntax.heading", &cfg.heading)?,
            emphasis: parse_field("syntax.emphasis", &cfg.emphasis)?,
            strong: parse_field("syntax.strong", &cfg.strong)?,
            link: parse_field("syntax.link", &cfg.link)?,
            code: parse_field("syntax.code", &cfg.code)?,
            list_marker: parse_field("syntax.list_marker", &cfg.list_marker)?,
            quote: parse_field("syntax.quote", &cfg.quote)?,
        })
    }
}

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
    pub syntax: SyntaxTheme,
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
            syntax: SyntaxTheme::default(),
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
    /// Constrói tema a partir de `[ui]` + `[syntax]` da config.
    pub fn from_config(cfg: &ThemeUiConfig) -> Result<Self, ThemeBuildError> {
        Self::from_config_parts(cfg, &SyntaxColorsConfig::default())
    }

    pub fn from_config_parts(
        ui: &ThemeUiConfig,
        syntax: &SyntaxColorsConfig,
    ) -> Result<Self, ThemeBuildError> {
        Ok(Self {
            background: parse_field("background", &ui.background)?,
            foreground: parse_field("foreground", &ui.foreground)?,
            line_number: parse_field("line_number", &ui.line_number)?,
            status_bg: parse_field("status_bg", &ui.status_bg)?,
            status_fg: parse_field("status_fg", &ui.status_fg)?,
            status_dirty: parse_field("status_dirty", &ui.status_dirty)?,
            cursor_bg: parse_field("cursor_bg", &ui.cursor_bg)?,
            cursor_fg: parse_field("cursor_fg", &ui.cursor_fg)?,
            gutter_width: ui.gutter_width.max(1),
            syntax: SyntaxTheme::from_config(syntax)?,
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
        // REVERSED garante contraste em qualquer tema; cores explícitas reforçam.
        Style::default()
            .fg(self.cursor_fg)
            .bg(self.cursor_bg)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    }

    /// Seleção da árvore com foco — alto contraste (linha inteira).
    ///
    /// Cores explícitas (não `Reset`/`REVERSED`) para funcionar em dark e light terminals.
    #[must_use]
    pub fn tree_selection_focused(self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    /// Seleção da árvore sem foco — ainda legível, menos “ativa”.
    #[must_use]
    pub fn tree_selection_unfocused(self) -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn syntax_style(self, kind: HighlightKind) -> Style {
        use HighlightKind::*;
        let (fg, bold) = match kind {
            Normal => (self.foreground, false),
            Comment => (self.syntax.comment, false),
            Keyword => (self.syntax.keyword, true),
            String => (self.syntax.string, false),
            Number => (self.syntax.number, false),
            Type => (self.syntax.type_name, false),
            Function => (self.syntax.function, false),
            Operator => (self.syntax.operator, false),
            Punctuation => (self.syntax.punctuation, false),
            Variable => (self.syntax.variable, false),
            Constant => (self.syntax.constant, false),
            Property => (self.syntax.property, false),
            Tag => (self.syntax.tag, false),
            Attribute => (self.syntax.attribute, false),
            Heading => (self.syntax.heading, true),
            Emphasis => (self.syntax.emphasis, false),
            Strong => (self.syntax.strong, true),
            Link => (self.syntax.link, false),
            Code => (self.syntax.code, false),
            ListMarker => (self.syntax.list_marker, true),
            Quote => (self.syntax.quote, false),
        };
        let mut style = Style::default().fg(fg).bg(self.background);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if matches!(kind, Emphasis) {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if matches!(kind, Link) {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        style
    }
}

fn parse_field(field: &'static str, value: &str) -> Result<Color, ThemeBuildError> {
    parse_color(value).map_err(|source| ThemeBuildError { field, source })
}
