//! Detecção de linguagem por path/extensão.

use std::path::Path;

/// Linguagens com highlight nativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LanguageId {
    #[default]
    Plain,
    OriScript,
    Markdown,
    /// MDX / MD com JSX — highlight como Markdown (sem parse JSX completo).
    Mdx,
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
            Self::Mdx => "mdx",
            Self::Html => "html",
            Self::Css => "css",
            Self::JavaScript => "javascript",
        }
    }

    /// Soft wrap padrão recomendado.
    #[must_use]
    pub fn default_soft_wrap(self) -> bool {
        matches!(self, Self::Markdown | Self::Mdx)
    }

    /// É família Markdown (md / mdx / derivados)?
    #[must_use]
    pub fn is_markdown_family(self) -> bool {
        matches!(self, Self::Markdown | Self::Mdx)
    }

    /// Token de comentário de linha (ou HTML comment open para MD).
    #[must_use]
    pub fn line_comment(self) -> Option<&'static str> {
        match self {
            Self::Markdown | Self::Mdx | Self::Html => Some("<!-- "),
            Self::OriScript | Self::JavaScript | Self::Css => Some("// "),
            Self::Plain => None,
        }
    }

    /// Sufixo de comentário HTML (MD).
    #[must_use]
    pub fn block_comment_close(self) -> Option<&'static str> {
        match self {
            Self::Markdown | Self::Mdx | Self::Html => Some(" -->"),
            _ => None,
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

    // nomes sem extensão / especiais
    if matches!(
        name.as_str(),
        "readme" | "changelog" | "history" | "license" | "authors" | "contributing"
    ) {
        return LanguageId::Markdown;
    }
    if name.ends_with(".md") || name.contains("readme") {
        // readme.pt-br etc.
        if name.contains('.') {
            // fall through to extension
        }
    }

    match ext.as_str() {
        "oris" => LanguageId::OriScript,
        // Markdown e derivados
        "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "mdwn" | "mdtxt" | "mdtext" | "rmd"
        | "qmd" => LanguageId::Markdown,
        "mdx" => LanguageId::Mdx,
        "html" | "htm" => LanguageId::Html,
        "css" => LanguageId::Css,
        "js" | "mjs" | "cjs" | "jsx" => LanguageId::JavaScript,
        "ts" | "tsx" => LanguageId::JavaScript,
        _ => {
            // README.md already handled; bare README
            if name.starts_with("readme") {
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

    #[test]
    fn detects_markdown_derivatives() {
        assert_eq!(
            detect_language(Some(Path::new("a.md"))),
            LanguageId::Markdown
        );
        assert_eq!(
            detect_language(Some(Path::new("doc.markdown"))),
            LanguageId::Markdown
        );
        assert_eq!(detect_language(Some(Path::new("x.mdx"))), LanguageId::Mdx);
        assert_eq!(
            detect_language(Some(Path::new("n.qmd"))),
            LanguageId::Markdown
        );
        assert_eq!(
            detect_language(Some(Path::new("README"))),
            LanguageId::Markdown
        );
        assert!(LanguageId::Markdown.default_soft_wrap());
        assert!(LanguageId::Mdx.is_markdown_family());
    }
}
