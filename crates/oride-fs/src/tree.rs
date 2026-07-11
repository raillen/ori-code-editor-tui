//! Árvore de diretórios expansível.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreeError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("path already exists: {0}")]
    AlreadyExists(PathBuf),
}

#[derive(Debug, Clone)]
struct Node {
    name: String,
    path: PathBuf,
    is_dir: bool,
    expanded: bool,
    /// `None` = ainda não carregado.
    children: Option<Vec<Node>>,
}

/// Árvore de projeto com expand/collapse preguiçoso.
#[derive(Debug, Clone)]
pub struct ProjectTree {
    root: PathBuf,
    root_name: String,
    show_hidden: bool,
    /// Nó virtual da raiz (sempre expandido).
    children: Vec<Node>,
    /// Índice flat da seleção atual.
    selected: usize,
}

/// Linha achatada para renderização.
#[derive(Debug, Clone)]
pub struct TreeRow {
    pub depth: usize,
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    File,
    Directory,
}

impl ProjectTree {
    pub fn open(root: impl AsRef<Path>, show_hidden: bool) -> Result<Self, TreeError> {
        let root = fs::canonicalize(root.as_ref())?;
        if !root.is_dir() {
            return Err(TreeError::NotDirectory(root));
        }
        let root_name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let mut tree = Self {
            root: root.clone(),
            root_name,
            show_hidden,
            children: read_dir_nodes(&root, show_hidden)?,
            selected: 0,
        };
        // Prefetch first level already done; expand root conceptually
        let _ = &mut tree;
        Ok(tree)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn root_name(&self) -> &str {
        &self.root_name
    }

    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn set_selected(&mut self, index: usize) {
        let len = self.flat_rows().len();
        if len == 0 {
            self.selected = 0;
        } else {
            self.selected = index.min(len - 1);
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.flat_rows().len() as isize;
        if len == 0 {
            return;
        }
        let next = (self.selected as isize + delta).rem_euclid(len) as usize;
        self.selected = next;
    }

    #[must_use]
    pub fn selected_row(&self) -> Option<TreeRow> {
        self.flat_rows().into_iter().nth(self.selected)
    }

    /// Achata a árvore para a UI (inclui linha da raiz depth 0).
    #[must_use]
    pub fn flat_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        rows.push(TreeRow {
            depth: 0,
            name: self.root_name.clone(),
            path: self.root.clone(),
            is_dir: true,
            expanded: true,
        });
        flatten_nodes(&self.children, 1, &mut rows);
        rows
    }

    /// Enter: expande dir ou devolve path de arquivo para abrir.
    pub fn activate_selected(&mut self) -> Result<Option<PathBuf>, TreeError> {
        let row = match self.selected_row() {
            Some(r) => r,
            None => return Ok(None),
        };
        if row.path == self.root {
            return Ok(None);
        }
        if row.is_dir {
            self.toggle_path(&row.path)?;
            Ok(None)
        } else {
            Ok(Some(row.path))
        }
    }

    pub fn toggle_selected(&mut self) -> Result<(), TreeError> {
        if let Some(row) = self.selected_row() {
            if row.is_dir && row.path != self.root {
                self.toggle_path(&row.path)?;
            }
        }
        Ok(())
    }

    fn toggle_path(&mut self, path: &Path) -> Result<(), TreeError> {
        if path == self.root {
            return Ok(());
        }
        toggle_in_nodes(&mut self.children, path, self.show_hidden)?;
        Ok(())
    }

    /// Recarrega filhos a partir do disco (mantém expanded flags quando possível).
    pub fn refresh(&mut self) -> Result<(), TreeError> {
        let expanded = collect_expanded(&self.children);
        self.children = read_dir_nodes(&self.root, self.show_hidden)?;
        restore_expanded(&mut self.children, &expanded, self.show_hidden)?;
        let len = self.flat_rows().len();
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
        }
        Ok(())
    }

    /// Cria arquivo ou pasta sob o diretório selecionado (ou o próprio se for dir).
    pub fn create_under_selection(
        &mut self,
        kind: CreateKind,
        name: &str,
    ) -> Result<PathBuf, TreeError> {
        let parent = self
            .selected_row()
            .map(|r| {
                if r.is_dir {
                    r.path
                } else {
                    r.path.parent().unwrap_or(&self.root).to_path_buf()
                }
            })
            .unwrap_or_else(|| self.root.clone());
        let created = create_path_under(&parent, kind, name)?;
        // Garante parent expandido
        if parent != self.root {
            ensure_expanded(&mut self.children, &parent, self.show_hidden)?;
        }
        self.refresh()?;
        // Seleciona o criado
        if let Some(idx) = self.flat_rows().iter().position(|r| r.path == created) {
            self.selected = idx;
        }
        Ok(created)
    }
}

/// Cria path relativo a `parent` (nome simples, sem separadores).
pub fn create_path_under(
    parent: &Path,
    kind: CreateKind,
    name: &str,
) -> Result<PathBuf, TreeError> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(TreeError::InvalidName(name.to_string()));
    }
    let path = parent.join(name);
    if path.exists() {
        return Err(TreeError::AlreadyExists(path));
    }
    match kind {
        CreateKind::File => {
            if let Some(p) = path.parent() {
                fs::create_dir_all(p)?;
            }
            fs::write(&path, "")?;
        }
        CreateKind::Directory => {
            fs::create_dir_all(&path)?;
        }
    }
    Ok(path)
}

/// Lista arquivos regulares sob root (para fuzzy open), relativo ao root.
pub fn list_files_recursive(root: &Path, show_hidden: bool) -> Result<Vec<PathBuf>, TreeError> {
    let mut out = Vec::new();
    walk_files(root, root, show_hidden, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_files(
    root: &Path,
    dir: &Path,
    show_hidden: bool,
    out: &mut Vec<PathBuf>,
) -> Result<(), TreeError> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name_s = name.to_string_lossy();
        if !show_hidden && name_s.starts_with('.') {
            continue;
        }
        // skip heavy dirs
        if name_s == "target" || name_s == "node_modules" || name_s == ".git" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_files(root, &path, show_hidden, out)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            out.push(rel);
        }
    }
    Ok(())
}

fn read_dir_nodes(dir: &Path, show_hidden: bool) -> Result<Vec<Node>, TreeError> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by(|a, b| {
        let ad = a.path().is_dir();
        let bd = b.path().is_dir();
        bd.cmp(&ad).then_with(|| a.file_name().cmp(&b.file_name()))
    });
    let mut nodes = Vec::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if name == "target" || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        let is_dir = path.is_dir();
        nodes.push(Node {
            name,
            path,
            is_dir,
            expanded: false,
            children: None,
        });
    }
    Ok(nodes)
}

fn flatten_nodes(nodes: &[Node], depth: usize, out: &mut Vec<TreeRow>) {
    for n in nodes {
        out.push(TreeRow {
            depth,
            name: n.name.clone(),
            path: n.path.clone(),
            is_dir: n.is_dir,
            expanded: n.expanded,
        });
        if n.is_dir && n.expanded {
            if let Some(ch) = &n.children {
                flatten_nodes(ch, depth + 1, out);
            }
        }
    }
}

fn toggle_in_nodes(nodes: &mut [Node], path: &Path, show_hidden: bool) -> Result<bool, TreeError> {
    for n in nodes.iter_mut() {
        if n.path == path {
            if !n.is_dir {
                return Ok(true);
            }
            n.expanded = !n.expanded;
            if n.expanded && n.children.is_none() {
                n.children = Some(read_dir_nodes(&n.path, show_hidden)?);
            }
            return Ok(true);
        }
        if n.is_dir {
            if let Some(ch) = n.children.as_mut() {
                if toggle_in_nodes(ch, path, show_hidden)? {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn ensure_expanded(nodes: &mut [Node], path: &Path, show_hidden: bool) -> Result<bool, TreeError> {
    for n in nodes.iter_mut() {
        if n.path == path {
            if n.is_dir {
                n.expanded = true;
                if n.children.is_none() {
                    n.children = Some(read_dir_nodes(&n.path, show_hidden)?);
                }
            }
            return Ok(true);
        }
        if path.starts_with(&n.path) && n.is_dir {
            n.expanded = true;
            if n.children.is_none() {
                n.children = Some(read_dir_nodes(&n.path, show_hidden)?);
            }
            if let Some(ch) = n.children.as_mut() {
                if ensure_expanded(ch, path, show_hidden)? {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn collect_expanded(nodes: &[Node]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for n in nodes {
        if n.expanded {
            out.push(n.path.clone());
        }
        if let Some(ch) = &n.children {
            out.extend(collect_expanded(ch));
        }
    }
    out
}

fn restore_expanded(
    nodes: &mut [Node],
    expanded: &[PathBuf],
    show_hidden: bool,
) -> Result<(), TreeError> {
    for n in nodes.iter_mut() {
        if n.is_dir && expanded.iter().any(|p| p == &n.path) {
            n.expanded = true;
            n.children = Some(read_dir_nodes(&n.path, show_hidden)?);
            if let Some(ch) = n.children.as_mut() {
                restore_expanded(ch, expanded, show_hidden)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_file_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let tree = ProjectTree::open(dir.path(), false).unwrap();
        assert_eq!(tree.flat_rows().len(), 1); // root only
        let f = create_path_under(dir.path(), CreateKind::File, "a.oris").unwrap();
        assert!(f.is_file());
        let mut tree = ProjectTree::open(dir.path(), false).unwrap();
        tree.refresh().unwrap();
        assert!(tree.flat_rows().iter().any(|r| r.name == "a.oris"));
    }

    #[test]
    fn expand_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.oris"), "x").unwrap();
        let mut tree = ProjectTree::open(dir.path(), false).unwrap();
        // select src (index 1)
        tree.set_selected(1);
        assert!(tree.activate_selected().unwrap().is_none());
        let rows = tree.flat_rows();
        assert!(rows.iter().any(|r| r.name == "main.oris"));
    }
}
