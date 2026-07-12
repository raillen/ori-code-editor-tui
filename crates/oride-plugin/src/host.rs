//! Host: registro estático de languages + plugins.

use oride_syntax::LanguageId;

use crate::language::{builtin_languages, provider_for, LanguageProvider};
use crate::plugin::{
    CommandMeta, LifecyclePlugin, Plugin, PluginCtx, PluginHook, PluginResult, WordCountPlugin,
};

/// Host embutido no binário.
pub struct PluginHost {
    languages: Vec<&'static dyn LanguageProvider>,
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginHost {
    #[must_use]
    pub fn new(
        languages: Vec<&'static dyn LanguageProvider>,
        plugins: Vec<Box<dyn Plugin>>,
    ) -> Self {
        Self { languages, plugins }
    }

    #[must_use]
    pub fn language(&self, id: LanguageId) -> &dyn LanguageProvider {
        for p in &self.languages {
            if p.language_id() == id {
                return *p;
            }
        }
        provider_for(id)
    }

    /// Labels para a command palette (`Plugin: word count`).
    #[must_use]
    pub fn palette_commands(&self) -> Vec<CommandMeta> {
        let mut out = Vec::new();
        for p in &self.plugins {
            out.extend_from_slice(p.commands());
        }
        out
    }

    pub fn dispatch_hook(&self, hook: PluginHook, ctx: &mut dyn PluginCtx) {
        for p in &self.plugins {
            p.on_hook(hook, ctx);
        }
    }

    pub fn run_command(&self, command_id: &str, ctx: &mut dyn PluginCtx) -> PluginResult {
        for p in &self.plugins {
            if p.commands().iter().any(|c| c.id == command_id) {
                return p.run_command(command_id, ctx);
            }
            // também aceita label exato
            if p.commands().iter().any(|c| c.label == command_id) {
                let id = p
                    .commands()
                    .iter()
                    .find(|c| c.label == command_id)
                    .map(|c| c.id)
                    .unwrap_or(command_id);
                return p.run_command(id, ctx);
            }
        }
        Err(crate::plugin::PluginError::UnknownCommand(
            command_id.to_string(),
        ))
    }

    /// Resolve id a partir do label da palette.
    #[must_use]
    pub fn command_id_for_label(&self, label: &str) -> Option<&'static str> {
        for p in &self.plugins {
            for c in p.commands() {
                if c.label == label {
                    return Some(c.id);
                }
            }
        }
        None
    }
}

/// Host padrão do Oride.
#[must_use]
pub fn builtin_host() -> PluginHost {
    PluginHost::new(
        builtin_languages(),
        vec![
            Box::new(WordCountPlugin),
            Box::new(LifecyclePlugin { announce: false }),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginCtx;
    use std::path::{Path, PathBuf};

    struct Ctx {
        status: String,
        text: String,
    }
    impl PluginCtx for Ctx {
        fn set_status(&mut self, msg: &str) {
            self.status = msg.into();
        }
        fn workspace_root(&self) -> &Path {
            Path::new(".")
        }
        fn active_path(&self) -> Option<PathBuf> {
            None
        }
        fn active_buffer_text(&self) -> String {
            self.text.clone()
        }
        fn active_is_dirty(&self) -> bool {
            false
        }
    }

    #[test]
    fn palette_lists_word_count() {
        let host = builtin_host();
        let cmds = host.palette_commands();
        assert!(cmds.iter().any(|c| c.id == "word_count"));
    }

    #[test]
    fn run_by_label() {
        let host = builtin_host();
        let mut ctx = Ctx {
            status: String::new(),
            text: "a b".into(),
        };
        host.run_command("Plugin: word count", &mut ctx).unwrap();
        assert!(ctx.status.contains("words=2"));
    }
}
