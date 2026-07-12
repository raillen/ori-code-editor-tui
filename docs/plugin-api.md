# Plugin API (P8 — built-in)

O Oride **não** carrega plugins externos (Lua/WASM). Extensões são crates Rust
embutidos no binário, registrados em `PluginHost` no boot.

Crate: **`oride-plugin`**.

## LanguageProvider

Metadados de linguagem (comentário, soft wrap, dica de LSP). O highlight
continua em `oride-syntax`.

```rust
pub trait LanguageProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn language_id(&self) -> LanguageId;
    fn extensions(&self) -> &'static [&'static str];
    fn comment_open(&self) -> Option<&'static str>;
    fn comment_close(&self) -> Option<&'static str>;
    fn lsp_command(&self) -> Option<&'static [&'static str]>;
    fn default_soft_wrap(&self) -> bool;
}
```

Providers built-in: plain, oriscript, markdown, mdx, html, css, javascript.

Uso no app: `plugin_host.language(lang)` em toggle comment e soft wrap default.

## Plugin + PluginCtx

```rust
pub trait PluginCtx {
    fn set_status(&mut self, msg: &str);
    fn workspace_root(&self) -> &Path;
    fn active_path(&self) -> Option<PathBuf>;
    fn active_buffer_text(&self) -> String;
    fn active_is_dirty(&self) -> bool;
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn commands(&self) -> &'static [CommandMeta];
    fn on_hook(&self, hook: PluginHook, ctx: &mut dyn PluginCtx);
    fn run_command(&self, id: &str, ctx: &mut dyn PluginCtx) -> PluginResult;
}
```

Hooks: `OnOpen` (após defaults de linguagem ao abrir), `OnSave` (após save ok).

### Built-ins atuais

| Plugin | Comandos | Hooks |
|--------|----------|--------|
| `word-count` | **Plugin: word count** (palette) | — |
| `show-path` | **Plugin: show file path** (palette) | — |
| `lifecycle` | — | silencioso (pode anunciar em testes) |

## Command palette

`Ctrl+Shift+P` lista actions nativas **e** labels de plugins. Enter em
`Plugin: word count` executa o comando.

## Host

```rust
let host = oride_plugin::builtin_host();
host.palette_commands();
host.run_command("word_count", &mut ctx);
host.dispatch_hook(PluginHook::OnOpen, &mut ctx);
```

## 0.3+ (não neste slice)

- Host **Lua** ou **WASM** (ADR)
- API estável versionada para plugins de terceiros
- Marketplace

Ver também: `docs/planning/post-0.1-roadmap.md` § P8.
