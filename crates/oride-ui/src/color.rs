//! Parse de cores a partir de strings de config.

use ratatui::style::Color;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid color `{0}` (use name, reset, or #RRGGBB)")]
pub struct ColorParseError(pub String);

/// Aceita `reset`, nomes ANSI, ou `#RRGGBB` / `#RGB`.
pub fn parse_color(s: &str) -> Result<Color, ColorParseError> {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    let color = match lower.as_str() {
        "reset" | "default" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "white" => Color::White,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        hex if hex.starts_with('#') => {
            parse_hex(&hex[1..]).ok_or_else(|| ColorParseError(t.into()))?
        }
        _ => return Err(ColorParseError(t.into())),
    };
    Ok(color)
}

fn parse_hex(hex: &str) -> Option<Color> {
    let (r, g, b) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_hex() {
        assert_eq!(parse_color("white").unwrap(), Color::White);
        assert_eq!(parse_color("#ff0000").unwrap(), Color::Rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0").unwrap(), Color::Rgb(0, 255, 0));
    }
}
