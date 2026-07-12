# Plugin API (0.1 — built-in)

No 0.1 o Oride **não** carrega plugins externos. Extensões de linguagem
são crates Rust embutidos no binário (`oride-syntax`, providers futuros).

## Traits planejados (estáveis em espírito)

```rust
pub trait LanguageProvider: Send + Sync {
    fn id(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn comment_token(&self) -> Option<&str>;
    fn lsp_command(&self) -> Option<Vec<String>>;
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn commands(&self) -> &[CommandMeta];
}
```

## 0.2+

Host externo (Lua ou WASM) reutilizará `Action` string + `PluginCtx`
sem quebrar keymaps TOML.

## LSP (P3)

Inteligência OriScript via `oriscript lsp` (stdio), configurável em
`[lsp]` no TOML — ver `docs/config.md`.
