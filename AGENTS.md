# AGENTS.md — Oride

Guia para agentes que implementam o editor TUI **Oride**.

**Precedência:** este arquivo > skills Grok > defaults do modelo.

## Produto

| Conceito | Nome |
|----------|------|
| Editor | **Oride** |
| CLI | `oride` |
| Crates | `oride-*` |
| Config | TOML (`~/.config/oride/`, `.oride/`) |
| Design | `docs/design.md` |
| Versão | SemVer a partir de `0.1.0-alpha.x` |

## Skills (obrigatórias neste repo)

| Skill | Quando |
|-------|--------|
| **`rust`** | Todo código (workspace, `Result`, clippy/fmt) |
| **`clean-code`** | Módulos, nomes, KISS, anti-primitivo |
| **`living-docs`** | README / `docs/` / comportamento user-facing |

**Não usar:** `compiler-dev`, `lang-interpreted`, `ori-testing` (são do monorepo OriScript).  
Inteligência de linguagem virá via **LSP PATH** (`oriscript lsp`), não linkando `oris-*`.

Não misturar lógica de UI (`ratatui`) em `oride-core` / `oride-config` / `oride-keymap`.

## Invariantes

1. **Core sem UI** — `oride-core` testável sem TTY.
2. **Actions como dados** — keymaps e palette disparam ações nomeadas.
3. **Fail closed** — LSP/Git/PTY falham com status, não crash.
4. **OriScript via PATH** — não linkar crates `oris-*` no 0.1.
5. **Um conceito por PR** — seguir fases em `docs/design.md`.

## Validação

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Fases (resumo)

| Fase | Foco |
|------|------|
| P0 | core + TUI + config/keymap/theme |
| P1 | tabs, árvore, terminal, palette (alpha.2) |
| P2 | tree-sitter + Markdown rico (alpha.3) |
| P3 | LSP OriScript |
| P4 | **polimento 0.1** (help, find, clipboard, session, …) |

Markdown futuro (preview, fence injections, MDX JSX): `docs/markdown.md` — **não** misturar no P3/P4.
