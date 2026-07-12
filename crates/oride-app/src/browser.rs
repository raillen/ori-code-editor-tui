//! Navegador de paths no filesystem (abrir pasta / arquivo).

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseMode {
    /// Só diretórios; confirmar pasta atual como workspace.
    Folder,
    /// Arquivos + diretórios; Enter em arquivo abre.
    File,
    /// Navega pastas e confirma `cwd/name_buffer` como destino de save.
    SaveAs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowseEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_parent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBrowser {
    pub cwd: PathBuf,
    pub mode: BrowseMode,
    pub entries: Vec<BrowseEntry>,
    pub selected: usize,
    /// Filtro digitável (substring case-insensitive) — em SaveAs é o nome do arquivo.
    pub filter: String,
}

#[derive(Debug, Clone)]
pub enum BrowseAction {
    /// Continua navegando (refresh/seleção).
    Stay,
    /// Usuário escolheu um arquivo.
    OpenFile(PathBuf),
    /// Usuário confirmou a pasta atual (ou a selecionada se for dir).
    OpenFolder(PathBuf),
    /// Caminho completo para salvar como.
    SaveAsPath(PathBuf),
}

impl PathBrowser {
    pub fn new(start: impl AsRef<Path>, mode: BrowseMode) -> Self {
        let cwd = fs::canonicalize(start.as_ref()).unwrap_or_else(|_| start.as_ref().to_path_buf());
        let mut b = Self {
            cwd,
            mode,
            entries: Vec::new(),
            selected: 0,
            filter: String::new(),
        };
        b.refresh();
        b
    }

    pub fn refresh(&mut self) {
        let mut entries = Vec::new();
        // parent
        if let Some(parent) = self.cwd.parent() {
            entries.push(BrowseEntry {
                name: "..".into(),
                path: parent.to_path_buf(),
                is_dir: true,
                is_parent: true,
            });
        }

        let rd = fs::read_dir(&self.cwd);
        if let Ok(rd) = rd {
            let mut items: Vec<_> = rd.filter_map(|e| e.ok()).collect();
            items.sort_by(|a, b| {
                let ad = a.path().is_dir();
                let bd = b.path().is_dir();
                bd.cmp(&ad).then_with(|| a.file_name().cmp(&b.file_name()))
            });
            for ent in items {
                let path = ent.path();
                let is_dir = path.is_dir();
                if matches!(self.mode, BrowseMode::Folder | BrowseMode::SaveAs) && !is_dir {
                    continue;
                }
                // skip heavy/noise
                let name = ent.file_name().to_string_lossy().into_owned();
                if name == "target" || name == "node_modules" || name == ".git" {
                    continue;
                }
                if name.starts_with('.') && name != ".." {
                    // show hidden optionally later; skip dotfiles for clarity
                    continue;
                }
                entries.push(BrowseEntry {
                    name,
                    path,
                    is_dir,
                    is_parent: false,
                });
            }
        }

        self.entries = entries;
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let n = if self.mode == BrowseMode::SaveAs {
            self.entries.len()
        } else {
            self.visible_entries().len()
        };
        if n == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(n - 1);
        }
    }

    #[must_use]
    pub fn visible_entries(&self) -> Vec<&BrowseEntry> {
        if self.filter.is_empty() {
            return self.entries.iter().collect();
        }
        let q = self.filter.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|e| e.name.to_ascii_lowercase().contains(&q))
            .collect()
    }

    pub fn move_selection(&mut self, delta: isize) {
        let n = if self.mode == BrowseMode::SaveAs {
            self.entries.len()
        } else {
            self.visible_entries().len()
        } as isize;
        if n == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(n) as usize;
    }

    #[must_use]
    pub fn selected_entry(&self) -> Option<&BrowseEntry> {
        if self.mode == BrowseMode::SaveAs {
            self.entries.get(self.selected)
        } else {
            self.visible_entries().get(self.selected).copied()
        }
    }

    /// Enter: entra em dir, ou confirma arquivo, ou sobe com ...
    pub fn activate(&mut self) -> BrowseAction {
        let Some(entry) = self.selected_entry().cloned() else {
            return BrowseAction::Stay;
        };
        if entry.is_dir {
            // SaveAs: ao entrar em pasta, NÃO limpa o nome do arquivo
            let keep_name = matches!(self.mode, BrowseMode::SaveAs).then(|| self.filter.clone());
            self.cwd = entry.path;
            if let Some(name) = keep_name {
                self.filter = name;
            } else {
                self.filter.clear();
            }
            self.selected = 0;
            self.refresh();
            BrowseAction::Stay
        } else if self.mode == BrowseMode::File {
            BrowseAction::OpenFile(entry.path)
        } else {
            BrowseAction::Stay
        }
    }

    /// Confirma pasta atual como workspace (Folder) ou path de save (SaveAs).
    pub fn confirm_folder(&self) -> BrowseAction {
        match self.mode {
            BrowseMode::SaveAs => {
                let name = self.filter.trim();
                if name.is_empty() || name.contains('/') || name == ".." {
                    return BrowseAction::Stay;
                }
                BrowseAction::SaveAsPath(self.cwd.join(name))
            }
            BrowseMode::Folder => {
                if let Some(e) = self.selected_entry() {
                    if e.is_dir && !e.is_parent {
                        return BrowseAction::OpenFolder(e.path.clone());
                    }
                }
                BrowseAction::OpenFolder(self.cwd.clone())
            }
            BrowseMode::File => {
                if let Some(e) = self.selected_entry() {
                    if e.is_dir && !e.is_parent {
                        return BrowseAction::OpenFolder(e.path.clone());
                    }
                }
                BrowseAction::OpenFolder(self.cwd.clone())
            }
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(p) = self.cwd.parent() {
            let keep_name = matches!(self.mode, BrowseMode::SaveAs).then(|| self.filter.clone());
            self.cwd = p.to_path_buf();
            if let Some(name) = keep_name {
                self.filter = name;
            } else {
                self.filter.clear();
            }
            self.selected = 0;
            self.refresh();
        }
    }

    #[must_use]
    pub fn title(&self) -> String {
        match self.mode {
            BrowseMode::Folder => format!("abrir pasta — {}", self.cwd.display()),
            BrowseMode::File => format!("abrir arquivo — {}", self.cwd.display()),
            BrowseMode::SaveAs => format!("salvar como — {}", self.cwd.display()),
        }
    }

    #[must_use]
    pub fn query_display(&self) -> String {
        match self.mode {
            BrowseMode::SaveAs => self.filter.clone(),
            _ => self.filter.clone(),
        }
    }

    #[must_use]
    pub fn hint(&self) -> &'static str {
        match self.mode {
            BrowseMode::Folder => {
                "↑↓ · Enter entra pasta · F2/Ctrl+Enter/Ctrl+O abre esta pasta · Esc"
            }
            BrowseMode::File => "↑↓ · Enter abre arquivo/entra · digite filtra · Esc",
            BrowseMode::SaveAs => {
                "↑↓ pastas · digite NOME · Enter/Ctrl+S salva · → entra pasta · Esc"
            }
        }
    }

    #[must_use]
    pub fn list_labels(&self) -> Vec<String> {
        // Em SaveAs, filtro é o nome do arquivo — não filtra a lista
        let entries: Vec<&BrowseEntry> = if self.mode == BrowseMode::SaveAs {
            self.entries.iter().collect()
        } else {
            self.visible_entries()
        };
        entries
            .iter()
            .map(|e| {
                if e.is_parent {
                    format!("📁 {}", e.name)
                } else if e.is_dir {
                    format!("📁 {}/", e.name)
                } else {
                    format!("📄 {}", e.name)
                }
            })
            .collect()
    }

    #[must_use]
    pub fn selected_index_for_display(&self) -> usize {
        if self.mode == BrowseMode::SaveAs {
            self.selected.min(self.entries.len().saturating_sub(1))
        } else {
            self.selected
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_parent_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        let b = PathBrowser::new(dir.path(), BrowseMode::Folder);
        assert!(b.entries.iter().any(|e| e.name == "sub"));
        assert!(!b.entries.iter().any(|e| e.name == "a.txt")); // folder mode
        let b2 = PathBrowser::new(dir.path(), BrowseMode::File);
        assert!(b2.entries.iter().any(|e| e.name == "a.txt"));
    }

    #[test]
    fn save_as_lists_dirs_only_and_confirms_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        let mut b = PathBrowser::new(dir.path(), BrowseMode::SaveAs);
        b.filter = "novo.txt".into();
        assert!(b.entries.iter().any(|e| e.name == "sub"));
        assert!(!b.entries.iter().any(|e| e.name == "a.txt"));
        match b.confirm_folder() {
            BrowseAction::SaveAsPath(p) => {
                assert_eq!(p, dir.path().join("novo.txt"));
            }
            other => panic!("expected SaveAsPath, got {other:?}"),
        }
    }
}
