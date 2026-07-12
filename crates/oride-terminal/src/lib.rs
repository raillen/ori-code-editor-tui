//! Painel de terminal embutido (PTY) — shell interativo.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("pty: {0}")]
    Pty(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Terminal embutido com scrollback (ANSI stripped para o painel texto).
pub struct EmbeddedTerminal {
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    scrollback: String,
    pub visible: bool,
    pub height_lines: u16,
    pub last_error: Option<String>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

impl EmbeddedTerminal {
    /// Spawna shell **interativo** em `cwd`.
    pub fn spawn(cwd: &Path, cols: u16, rows: u16) -> Result<Self, TerminalError> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(2),
                cols: cols.max(20),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Pty(e.to_string()))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = CommandBuilder::new(&shell);
        if shell.contains("bash")
            || shell.contains("zsh")
            || shell.contains("fish")
            || shell.ends_with("/sh")
        {
            cmd.arg("-i");
        }
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Pty(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Pty(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Pty(e.to_string()))?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(15)),
                }
            }
        });

        Ok(Self {
            writer,
            rx,
            scrollback: String::new(),
            visible: false,
            height_lines: 12,
            last_error: None,
            _child: child,
            _master: pair.master,
        })
    }

    pub fn poll_output(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    let stripped = strip_ansi_escapes::strip(&chunk);
                    let text = String::from_utf8_lossy(&stripped);
                    self.scrollback.push_str(&text);
                    const MAX: usize = 400_000;
                    if self.scrollback.len() > MAX {
                        let drain = self.scrollback.len() - MAX;
                        self.scrollback.drain(..drain);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.last_error = Some("shell encerrado".into());
                    break;
                }
            }
        }
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        match self
            .writer
            .write_all(data)
            .and_then(|_| self.writer.flush())
        {
            Ok(()) => {
                self.last_error = None;
                Ok(())
            }
            Err(e) => {
                self.last_error = Some(e.to_string());
                Err(TerminalError::Io(e))
            }
        }
    }

    pub fn write_str(&mut self, s: &str) -> Result<(), TerminalError> {
        self.write_bytes(s.as_bytes())
    }

    #[must_use]
    pub fn visible_lines(&self, max_lines: usize) -> Vec<String> {
        let lines: Vec<&str> = self.scrollback.lines().collect();
        let start = lines.len().saturating_sub(max_lines.max(1));
        lines[start..].iter().map(|s| (*s).to_string()).collect()
    }

    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let _ = self._master.resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn grow(&mut self, delta: u16) {
        self.height_lines = self.height_lines.saturating_add(delta).clamp(3, 40);
        self.visible = true;
    }

    pub fn shrink(&mut self, delta: u16) {
        self.height_lines = self.height_lines.saturating_sub(delta).max(3);
    }
}
