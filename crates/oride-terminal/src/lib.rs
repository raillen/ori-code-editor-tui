//! Painel de terminal embutido (PTY).

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

/// Terminal embutido com scrollback em texto (ANSI stripped).
pub struct EmbeddedTerminal {
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    scrollback: String,
    pub visible: bool,
    /// Altura em linhas do painel (0 = colapsado visualmente; visible ainda pode ser true).
    pub height_lines: u16,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    // keep master alive
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

// portable-pty child/master are opaque trait objects from the crate API.

impl EmbeddedTerminal {
    /// Spawna `$SHELL` ou `/bin/sh` em `cwd`.
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
        cmd.cwd(cwd);
        // login-ish env
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Pty(e.to_string()))?;
        // drop slave on pair end

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
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        });

        Ok(Self {
            writer,
            rx,
            scrollback: String::new(),
            visible: false,
            height_lines: 10,
            _child: child,
            _master: pair.master,
        })
    }

    /// Drena saída pendente do PTY.
    pub fn poll_output(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    let stripped = strip_ansi_escapes::strip(&chunk);
                    let text = String::from_utf8_lossy(&stripped);
                    self.scrollback.push_str(&text);
                    // limita scrollback
                    const MAX: usize = 200_000;
                    if self.scrollback.len() > MAX {
                        let drain = self.scrollback.len() - MAX;
                        self.scrollback.drain(..drain);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn write_str(&mut self, s: &str) -> Result<(), TerminalError> {
        self.write_bytes(s.as_bytes())
    }

    /// Últimas `max_lines` linhas do scrollback.
    #[must_use]
    pub fn visible_lines(&self, max_lines: usize) -> Vec<String> {
        let lines: Vec<&str> = self.scrollback.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        lines[start..].iter().map(|s| (*s).to_string()).collect()
    }

    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        // best-effort; portable-pty MasterPty has resize
        let _ = self._master.resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
