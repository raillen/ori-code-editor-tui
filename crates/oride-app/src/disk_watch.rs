//! Watch de arquivos abertos (reload se mudarem no disco).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::SystemTime;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Eventos de mudança em path observado.
#[derive(Debug, Clone)]
pub struct DiskChange {
    pub path: PathBuf,
}

pub struct DiskWatch {
    _watcher: Option<RecommendedWatcher>,
    rx: Option<Receiver<DiskChange>>,
    /// Último mtime que nós mesmos gravamos (evita auto-reload do próprio save).
    ignore_mtime: HashMap<PathBuf, SystemTime>,
}

impl DiskWatch {
    pub fn start(workspace: &Path) -> Self {
        let (tx, rx) = mpsc::channel::<DiskChange>();
        let mut watcher = match notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(ev) = res {
                if matches!(
                    ev.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                ) {
                    for path in ev.paths {
                        let _ = tx.send(DiskChange { path });
                    }
                }
            }
        }) {
            Ok(w) => w,
            Err(_) => {
                return Self {
                    _watcher: None,
                    rx: None,
                    ignore_mtime: HashMap::new(),
                };
            }
        };
        let _ = watcher.watch(workspace, RecursiveMode::Recursive);
        Self {
            _watcher: Some(watcher),
            rx: Some(rx),
            ignore_mtime: HashMap::new(),
        }
    }

    pub fn mark_saved(&mut self, path: &Path) {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                self.ignore_mtime.insert(path.to_path_buf(), mtime);
            }
        }
    }

    pub fn poll(&mut self) -> Vec<PathBuf> {
        let Some(rx) = &self.rx else {
            return Vec::new();
        };
        let mut out = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(ch) => {
                    if let Ok(meta) = std::fs::metadata(&ch.path) {
                        if let Ok(mtime) = meta.modified() {
                            if self.ignore_mtime.get(&ch.path) == Some(&mtime) {
                                continue;
                            }
                        }
                    }
                    if ch.path.is_file() && !out.contains(&ch.path) {
                        out.push(ch.path);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}
