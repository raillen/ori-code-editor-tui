//! Busca em projeto: tenta `rg`, senão walk Rust com `ignore`.

mod ripgrep;
mod walk;

use std::path::{Path, PathBuf};

use thiserror::Error;

pub use ripgrep::rg_available;

/// Opções de busca no workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub pattern: String,
    pub case_sensitive: bool,
    pub use_regex: bool,
    /// Máximo de hits (protege UI).
    pub max_hits: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            case_sensitive: false,
            use_regex: false,
            max_hits: 500,
        }
    }
}

/// Uma ocorrência no disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    /// 1-based (como editores / rg).
    pub line: usize,
    /// 1-based coluna de caractere no início do match (aprox.).
    pub column: usize,
    /// Linha de texto (sem `\n`).
    pub line_text: String,
}

/// Qual backend produziu os hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBackend {
    Ripgrep,
    RustWalk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub backend: SearchBackend,
    pub truncated: bool,
}

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("empty pattern")]
    EmptyPattern,
    #[error("invalid regex: {0}")]
    Regex(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Executa busca: prefere `rg` se disponível e saudável; senão fallback Rust.
pub fn search_project(root: &Path, query: &SearchQuery) -> Result<SearchResult, SearchError> {
    if query.pattern.trim().is_empty() {
        return Err(SearchError::EmptyPattern);
    }
    let max = query.max_hits.max(1);

    if ripgrep::rg_available() {
        match ripgrep::search_with_rg(root, query, max) {
            Ok(mut hits) => {
                let truncated = hits.len() >= max;
                if hits.len() > max {
                    hits.truncate(max);
                }
                return Ok(SearchResult {
                    hits,
                    backend: SearchBackend::Ripgrep,
                    truncated,
                });
            }
            Err(_) => {
                // cai no fallback
            }
        }
    }

    let mut hits = walk::search_walk(root, query, max)?;
    let truncated = hits.len() >= max;
    if hits.len() > max {
        hits.truncate(max);
    }
    Ok(SearchResult {
        hits,
        backend: SearchBackend::RustWalk,
        truncated,
    })
}

/// Formata hit para lista na UI.
#[must_use]
pub fn format_hit_label(hit: &SearchHit, root: &Path) -> String {
    let rel = hit
        .path
        .strip_prefix(root)
        .unwrap_or(&hit.path)
        .display()
        .to_string();
    let snippet: String = hit.line_text.chars().take(80).collect();
    format!("{rel}:{}:{}  {snippet}", hit.line, hit.column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut f = fs::File::create(dir.path().join("src/a.oris")).unwrap();
        writeln!(f, "fn hello() {{").unwrap();
        writeln!(f, "  print(\"findme\")").unwrap();
        writeln!(f, "}}").unwrap();
        let mut g = fs::File::create(dir.path().join("readme.md")).unwrap();
        writeln!(g, "# Title").unwrap();
        writeln!(g, "findme in docs").unwrap();
        // noise dirs
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        fs::write(dir.path().join("target/debug/x"), "findme noise").unwrap();
        dir
    }

    #[test]
    fn finds_in_multiple_files_skips_target() {
        let dir = fixture();
        let q = SearchQuery {
            pattern: "findme".into(),
            case_sensitive: false,
            use_regex: false,
            max_hits: 100,
        };
        // Força fallback Rust para determinismo (mesmo com rg instalado)
        let hits = walk::search_walk(dir.path(), &q, 100).unwrap();
        assert!(hits.len() >= 2, "hits={hits:?}");
        assert!(hits
            .iter()
            .all(|h| !h.path.to_string_lossy().contains("target")));
        assert!(hits.iter().any(|h| h.line_text.contains("findme")));
    }

    #[test]
    fn regex_mode() {
        let dir = fixture();
        let q = SearchQuery {
            pattern: r"find\w+".into(),
            case_sensitive: false,
            use_regex: true,
            max_hits: 50,
        };
        let hits = walk::search_walk(dir.path(), &q, 50).unwrap();
        assert!(!hits.is_empty());
    }

    #[test]
    fn search_project_returns_backend() {
        let dir = fixture();
        let q = SearchQuery {
            pattern: "hello".into(),
            ..Default::default()
        };
        let r = search_project(dir.path(), &q).unwrap();
        assert!(!r.hits.is_empty());
        assert!(matches!(
            r.backend,
            SearchBackend::Ripgrep | SearchBackend::RustWalk
        ));
    }
}
