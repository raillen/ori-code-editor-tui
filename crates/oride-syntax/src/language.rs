//! Detecção de linguagem por path/extensão.

use std::path::Path;

/// Linguagens com highlight nativo no P2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LanguageId {
    #[default]
    Plain,
    OriScript,
    Markdown,
    Html,
    Css,
    JavaScript,
}

impl LanguageId {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::OriScript => "oriscript",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Css => "css",
            Self::JavaScript => "javascript",
        }
    }
}

/// Detecta linguagem a partir do path do arquivo.
#[must_use]
pub fn detect_language(path: Option<&Path>) -> LanguageId {
    let Some(path) = path else {
        return LanguageId::Plain;
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "oris" => LanguageId::OriScript,
        "md" | "markdown" | "mdown" => LanguageId::Markdown,
        "html" | "htm" => LanguageId::Html,
        "css" => LanguageId::Css,
        "js" | "mjs" | "cjs" | "jsx" => LanguageId::JavaScript,
        "ts" | "tsx" => LanguageId::JavaScript, // approx
        _ => {
            if name == "readme" {
                LanguageId::Markdown
            } else {
                LanguageId::Plain
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_extensions() {
        assert_eq!(
            detect_language(Some(Path::new("main.oris"))),
            LanguageId::OriScript
        );
        assert_eq!(
            detect_language(Some(Path::new("a.md"))),
            LanguageId::Markdown
        );
        assert_eq!(
            detect_language(Some(Path::new("x.JS"))),
            LanguageId::JavaScript
        );
        assert_eq!(
            detect_language(Some(&PathBuf::from("noext"))),
            LanguageId::Plain
        );
    }
}
