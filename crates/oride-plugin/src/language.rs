//! LanguageProvider — metadados de linguagem (comentário, soft wrap, LSP).

use oride_syntax::LanguageId;

/// Provedor de linguagem embutido (sem highlight — isso fica em `oride-syntax`).
pub trait LanguageProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn language_id(&self) -> LanguageId;
    fn extensions(&self) -> &'static [&'static str];
    /// Prefixo de comentário de linha (`// `, `<!-- `).
    fn comment_open(&self) -> Option<&'static str>;
    /// Sufixo (ex. ` -->` em HTML/MD).
    fn comment_close(&self) -> Option<&'static str> {
        None
    }
    /// Comando LSP sugerido (ex. `oriscript lsp`).
    fn lsp_command(&self) -> Option<&'static [&'static str]> {
        None
    }
    fn default_soft_wrap(&self) -> bool {
        false
    }
}

/// Provider trivial por `LanguageId`.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinLang {
    pub id: &'static str,
    pub language_id: LanguageId,
    pub extensions: &'static [&'static str],
    pub comment_open: Option<&'static str>,
    pub comment_close: Option<&'static str>,
    pub lsp: Option<&'static [&'static str]>,
    pub soft_wrap: bool,
}

impl LanguageProvider for BuiltinLang {
    fn id(&self) -> &'static str {
        self.id
    }
    fn language_id(&self) -> LanguageId {
        self.language_id
    }
    fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }
    fn comment_open(&self) -> Option<&'static str> {
        self.comment_open
    }
    fn comment_close(&self) -> Option<&'static str> {
        self.comment_close
    }
    fn lsp_command(&self) -> Option<&'static [&'static str]> {
        self.lsp
    }
    fn default_soft_wrap(&self) -> bool {
        self.soft_wrap
    }
}

pub static LANG_PLAIN: BuiltinLang = BuiltinLang {
    id: "plain",
    language_id: LanguageId::Plain,
    extensions: &[],
    comment_open: None,
    comment_close: None,
    lsp: None,
    soft_wrap: false,
};

pub static LANG_ORIS: BuiltinLang = BuiltinLang {
    id: "oriscript",
    language_id: LanguageId::OriScript,
    extensions: &["oris"],
    comment_open: Some("// "),
    comment_close: None,
    lsp: Some(&["oriscript", "lsp"]),
    soft_wrap: false,
};

pub static LANG_MD: BuiltinLang = BuiltinLang {
    id: "markdown",
    language_id: LanguageId::Markdown,
    extensions: &["md", "markdown", "mdown"],
    comment_open: Some("<!-- "),
    comment_close: Some(" -->"),
    lsp: None,
    soft_wrap: true,
};

pub static LANG_MDX: BuiltinLang = BuiltinLang {
    id: "mdx",
    language_id: LanguageId::Mdx,
    extensions: &["mdx"],
    comment_open: Some("<!-- "),
    comment_close: Some(" -->"),
    lsp: None,
    soft_wrap: true,
};

pub static LANG_HTML: BuiltinLang = BuiltinLang {
    id: "html",
    language_id: LanguageId::Html,
    extensions: &["html", "htm"],
    comment_open: Some("<!-- "),
    comment_close: Some(" -->"),
    lsp: None,
    soft_wrap: false,
};

pub static LANG_CSS: BuiltinLang = BuiltinLang {
    id: "css",
    language_id: LanguageId::Css,
    extensions: &["css"],
    comment_open: Some("/* "),
    comment_close: Some(" */"),
    lsp: None,
    soft_wrap: false,
};

pub static LANG_JS: BuiltinLang = BuiltinLang {
    id: "javascript",
    language_id: LanguageId::JavaScript,
    extensions: &["js", "mjs", "cjs"],
    comment_open: Some("// "),
    comment_close: None,
    lsp: None,
    soft_wrap: false,
};

/// Todos os providers built-in.
pub fn builtin_languages() -> Vec<&'static dyn LanguageProvider> {
    vec![
        &LANG_PLAIN,
        &LANG_ORIS,
        &LANG_MD,
        &LANG_MDX,
        &LANG_HTML,
        &LANG_CSS,
        &LANG_JS,
    ]
}

/// Resolve provider pelo `LanguageId` de `oride-syntax`.
#[must_use]
pub fn provider_for(lang: LanguageId) -> &'static dyn LanguageProvider {
    for p in builtin_languages() {
        if p.language_id() == lang {
            return p;
        }
    }
    &LANG_PLAIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oris_has_lsp_hint() {
        let p = provider_for(LanguageId::OriScript);
        assert_eq!(p.id(), "oriscript");
        assert_eq!(p.lsp_command(), Some(&["oriscript", "lsp"][..]));
        assert_eq!(p.comment_open(), Some("// "));
    }

    #[test]
    fn md_soft_wrap_and_html_comment() {
        let p = provider_for(LanguageId::Markdown);
        assert!(p.default_soft_wrap());
        assert_eq!(p.comment_close(), Some(" -->"));
    }
}
