//! Carrega e mescla camadas de config.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::model::{Config, RawConfigFile};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("read config {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// `~/.config/oride/config.toml` (ou equivalente XDG).
#[must_use]
pub fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("oride").join("config.toml"))
}

/// Caminhos candidatos de projeto: `start` e ancestrais + `.oride/config.toml`.
#[must_use]
pub fn config_search_roots(start: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut cur = start
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok());

    while let Some(dir) = cur {
        roots.push(dir.join(".oride").join("config.toml"));
        cur = dir.parent().map(Path::to_path_buf);
        if roots.len() > 32 {
            break;
        }
    }
    roots
}

/// Defaults ← user config ← primeiro `.oride/config.toml` encontrado.
pub fn load_merged(workspace_hint: Option<&Path>) -> Result<Config, ConfigError> {
    let mut cfg = Config::default();

    if let Some(user_path) = user_config_path() {
        if user_path.is_file() {
            apply_file(&mut cfg, &user_path)?;
        }
    }

    // Projeto: o mais próximo do hint (primeiro na lista) ganha.
    for path in config_search_roots(workspace_hint) {
        if path.is_file() {
            apply_file(&mut cfg, &path)?;
            break;
        }
    }

    Ok(cfg)
}

fn apply_file(cfg: &mut Config, path: &Path) -> Result<(), ConfigError> {
    let text = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let raw: RawConfigFile = toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    cfg.apply_raw(raw);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn merge_project_overrides_keys() {
        let dir = tempfile::tempdir().unwrap();
        let oride = dir.path().join(".oride");
        fs::create_dir_all(&oride).unwrap();
        let path = oride.join("config.toml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
show_line_numbers = false
[editor]
tab_size = 2
[keys]
"ctrl+s" = "quit"
[ui]
status_dirty = "red"
"#
        )
        .unwrap();

        let mut cfg = Config::default();
        apply_file(&mut cfg, &path).unwrap();
        assert!(!cfg.show_line_numbers);
        assert_eq!(cfg.editor.tab_size, 2);
        assert_eq!(cfg.keys.get("ctrl+s").map(String::as_str), Some("quit"));
        assert_eq!(cfg.theme_ui.status_dirty, "red");
        // default key still present
        assert_eq!(cfg.keys.get("ctrl+z").map(String::as_str), Some("undo"));
    }

    #[test]
    fn load_merged_finds_nested_project() {
        let dir = tempfile::tempdir().unwrap();
        let oride = dir.path().join(".oride");
        fs::create_dir_all(&oride).unwrap();
        fs::write(oride.join("config.toml"), "show_line_numbers = false\n").unwrap();
        let nested = dir.path().join("src");
        fs::create_dir_all(&nested).unwrap();

        let cfg = load_merged(Some(&nested)).unwrap();
        assert!(!cfg.show_line_numbers);
    }
}
