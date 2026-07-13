# Syntax highlight

Oride usa **tree-sitter** (e pipeline MD próprio) para colorir o buffer ativo.

## Linguagens first-class

**Normativo:** [`docs/planning/alpha6-roadmap.md`](planning/alpha6-roadmap.md) §3.

| LanguageId | Extensões | Grammar / motor | Estado (alpha.6) |
|------------|-----------|-----------------|------------------|
| `oriscript` | `.oris` | `tree-sitter-oriscript` (vendored) | **first-class** |
| `ori` / ori-lang | `.orl` (a confirmar) | grammar monorepo / path | **L1 planejado** |
| `markdown` | `.md`, … | `tree-sitter-md` + inject | **first-class** |
| `mdx` | `.mdx` | como MD | parcial |
| `html` | `.html`, `.htm` | `tree-sitter-html` | **first-class** |
| `css` | `.css` | `tree-sitter-css` | **first-class** |
| `javascript` | `.js`, `.mjs`, `.cjs`, `.jsx` | `tree-sitter-javascript` | **first-class** |
| `typescript` | `.ts`, `.tsx` | grammar TS (L1) ou JS aprox. | **L1 planejado** |
| `rust` | `.rs` | tree-sitter-rust | **L1 planejado** |
| `python` | `.py` | tree-sitter-python | **L1 planejado** |
| `nim` | `.nim` | tree-sitter-nim se estável | **L1 planejado** |
| `ruby` | `.rb` | tree-sitter-ruby | **L1 planejado** |
| `plain` | outras | — | sem highlight |

A linguagem ativa aparece na status line.

**Fence inject (MD):** conteúdo de ` ```lang ` re-highlight com a grammar da lang (hoje: oris/js/html/css; L1 expande).

## Como funciona

1. `detect_language(path)` escolhe o id.
2. `HighlightEngine` reparseia quando o texto muda.
3. Nós do AST → `HighlightKind` → cores em `UiTheme.syntax`.
4. Markdown blocks usam grammar MD; inlines e injects complementam.

## Crates

- `oride-syntax` — engine + kinds + detecção + MD preview lines
- `tree-sitter-oriscript` — binding da grammar OriScript
- grammars externas via crates `tree-sitter-*` (L1)

## Limitações

- Reparse completo por edit (não incremental) — ok para arquivos médios
- Cores de syntax no TOML: parcial (E1.4 no roadmap)
- TypeScript ainda pode mapear para grammar JS até L1.1
- Semantic tokens / multi-LSP: L2 no roadmap (opt-in)

## Validação

```bash
cargo test -p oride-syntax
```
