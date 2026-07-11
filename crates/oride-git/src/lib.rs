//! Status git via `git status --porcelain` (sem libgit2).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Status simplificado para a árvore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
    Conflict,
}

impl GitFileStatus {
    #[must_use]
    pub fn badge(self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Untracked => '?',
            Self::Renamed => 'R',
            Self::Conflict => 'U',
        }
    }
}

/// Mapa path relativo ao root do repo → status (pior status se múltiplos).
pub fn status_map(cwd: &Path) -> HashMap<PathBuf, GitFileStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-z"])
        .current_dir(cwd)
        .output();

    let Ok(output) = output else {
        return HashMap::new();
    };
    if !output.status.success() {
        return HashMap::new();
    }

    let mut map = HashMap::new();
    // -z: records separated by NUL; each "XY path" or rename "XY\0old\0new"
    for rec in output.stdout.split(|b| *b == 0) {
        if rec.len() < 3 {
            continue;
        }
        let xy = &rec[..2];
        let path_bytes = &rec[3..]; // skip "XY "
                                    // renames may have extra; take first path component
        let path_str = String::from_utf8_lossy(path_bytes);
        let path = PathBuf::from(path_str.trim());
        if path.as_os_str().is_empty() {
            continue;
        }
        let status = classify(xy);
        map.entry(path)
            .and_modify(|s| *s = worse(*s, status))
            .or_insert(status);
    }
    map
}

/// Branch atual ou `None`.
pub fn current_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn classify(xy: &[u8]) -> GitFileStatus {
    let x = xy[0] as char;
    let y = xy[1] as char;
    if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
        return GitFileStatus::Conflict;
    }
    if x == '?' || y == '?' {
        return GitFileStatus::Untracked;
    }
    if x == 'R' || y == 'R' {
        return GitFileStatus::Renamed;
    }
    if x == 'A' || y == 'A' {
        return GitFileStatus::Added;
    }
    if x == 'D' || y == 'D' {
        return GitFileStatus::Deleted;
    }
    if x == 'M' || y == 'M' {
        return GitFileStatus::Modified;
    }
    GitFileStatus::Modified
}

fn worse(a: GitFileStatus, b: GitFileStatus) -> GitFileStatus {
    use GitFileStatus::*;
    let rank = |s: GitFileStatus| match s {
        Conflict => 5,
        Deleted => 4,
        Modified => 3,
        Renamed => 2,
        Added => 1,
        Untracked => 0,
    };
    if rank(b) > rank(a) {
        b
    } else {
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_modified() {
        assert_eq!(classify(b" M"), GitFileStatus::Modified);
        assert_eq!(classify(b"??"), GitFileStatus::Untracked);
        assert_eq!(classify(b"A "), GitFileStatus::Added);
    }
}
