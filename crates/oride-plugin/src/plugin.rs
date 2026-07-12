//! Plugin built-in: comandos + hooks.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Metadado de comando exposto na command palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMeta {
    /// Id estável (`word_count`).
    pub id: &'static str,
    /// Rótulo na palette (`Word count`).
    pub label: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginHook {
    OnOpen,
    OnSave,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("{0}")]
    Message(String),
}

pub type PluginResult = Result<(), PluginError>;

/// Contexto que o app implementa — plugins não conhecem `App`.
pub trait PluginCtx {
    fn set_status(&mut self, msg: &str);
    fn workspace_root(&self) -> &Path;
    fn active_path(&self) -> Option<PathBuf>;
    fn active_buffer_text(&self) -> String;
    /// Se o buffer está dirty.
    fn active_is_dirty(&self) -> bool;
}

/// Plugin embutido (Rust estático).
pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn commands(&self) -> &'static [CommandMeta];
    /// Hook opcional (default: no-op).
    fn on_hook(&self, _hook: PluginHook, _ctx: &mut dyn PluginCtx) {}
    fn run_command(&self, id: &str, ctx: &mut dyn PluginCtx) -> PluginResult;
}

/// Plugin de exemplo: contagem de palavras/linhas do buffer ativo.
pub struct WordCountPlugin;

const WORD_COUNT_CMDS: &[CommandMeta] = &[CommandMeta {
    id: "word_count",
    label: "Plugin: word count",
    description: "Conta palavras e linhas do buffer ativo",
}];

impl Plugin for WordCountPlugin {
    fn name(&self) -> &'static str {
        "word-count"
    }

    fn commands(&self) -> &'static [CommandMeta] {
        WORD_COUNT_CMDS
    }

    fn on_hook(&self, hook: PluginHook, ctx: &mut dyn PluginCtx) {
        // leve: não spammar status em todo open/save
        let _ = (hook, ctx);
    }

    fn run_command(&self, id: &str, ctx: &mut dyn PluginCtx) -> PluginResult {
        if id != "word_count" {
            return Err(PluginError::UnknownCommand(id.into()));
        }
        let text = ctx.active_buffer_text();
        let lines = text.lines().count();
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        let msg = format!("words={words} · lines={lines} · chars={chars}");
        ctx.set_status(&msg);
        Ok(())
    }
}

/// Plugin que anuncia open/save (útil para validar hooks em testes e UX).
#[derive(Default)]
pub struct LifecyclePlugin {
    pub announce: bool,
}

impl Plugin for LifecyclePlugin {
    fn name(&self) -> &'static str {
        "lifecycle"
    }

    fn commands(&self) -> &'static [CommandMeta] {
        &[]
    }

    fn on_hook(&self, hook: PluginHook, ctx: &mut dyn PluginCtx) {
        if !self.announce {
            return;
        }
        match hook {
            PluginHook::OnOpen => {
                if let Some(p) = ctx.active_path() {
                    let msg = format!("plugin: opened {}", p.display());
                    ctx.set_status(&msg);
                }
            }
            PluginHook::OnSave => {
                if let Some(p) = ctx.active_path() {
                    let msg = format!("plugin: saved {}", p.display());
                    ctx.set_status(&msg);
                }
            }
        }
    }

    fn run_command(&self, id: &str, _ctx: &mut dyn PluginCtx) -> PluginResult {
        Err(PluginError::UnknownCommand(id.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct FakeCtx {
        status: String,
        text: String,
        path: Option<PathBuf>,
    }

    impl PluginCtx for FakeCtx {
        fn set_status(&mut self, msg: &str) {
            self.status = msg.to_string();
        }
        fn workspace_root(&self) -> &Path {
            Path::new("/tmp")
        }
        fn active_path(&self) -> Option<PathBuf> {
            self.path.clone()
        }
        fn active_buffer_text(&self) -> String {
            self.text.clone()
        }
        fn active_is_dirty(&self) -> bool {
            false
        }
    }

    #[test]
    fn word_count_command() {
        let mut ctx = FakeCtx {
            status: String::new(),
            text: "one two three\nfour".into(),
            path: None,
        };
        WordCountPlugin.run_command("word_count", &mut ctx).unwrap();
        assert!(ctx.status.contains("words=4"));
        assert!(ctx.status.contains("lines=2"));
    }
}
