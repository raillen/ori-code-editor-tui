//! Sessão leve: workspace + lista de arquivos abertos.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub workspace: PathBuf,
    pub files: Vec<PathBuf>,
    /// Índice na lista `files` (não DocumentId).
    pub active_index: usize,
}

impl Session {
    #[must_use]
    pub fn path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("oride").join("session.toml"))
    }

    pub fn load() -> Option<Self> {
        let path = Self::path()?;
        let text = fs::read_to_string(path).ok()?;
        toml::from_str(&text).ok()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, text)
    }

    pub fn from_workspace(workspace: &Path, files: Vec<PathBuf>, active_index: usize) -> Self {
        Self {
            workspace: workspace.to_path_buf(),
            files,
            active_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_toml() {
        let s = Session {
            workspace: PathBuf::from("/tmp/proj"),
            files: vec![PathBuf::from("/tmp/proj/a.md")],
            active_index: 0,
        };
        let t = toml::to_string(&s).unwrap();
        let back: Session = toml::from_str(&t).unwrap();
        assert_eq!(back.workspace, s.workspace);
        assert_eq!(back.files.len(), 1);
    }
}
