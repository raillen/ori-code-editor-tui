# Syntax highlight (P2)

Oride usa **tree-sitter** para colorir o buffer ativo.

## Linguagens

| Extensão | LanguageId | Grammar |
|----------|------------|---------|
| `.oris` | `oriscript` | `tree-sitter-oriscript` (vendored) |
| `.md` | `markdown` | `tree-sitter-md` |
| `.html` / `.htm` | `html` | `tree-sitter-html` |
| `.css` | `css` | `tree-sitter-css` |
| `.js` / `.mjs` / `.jsx` | `javascript` | `tree-sitter-javascript` |
| outras | `plain` | sem highlight |

A linguagem ativa aparece na status line.

## Como funciona

1. `detect_language(path)` escolhe o grammar.
2. `HighlightEngine` reparseia quando o texto muda.
3. Nós nomeados do AST viram `HighlightKind` (keyword, string, comment, …).
4. O viewport pinta spans com cores de `UiTheme.syntax`.

## Crates

- `oride-syntax` — engine + kinds + detecção
- `tree-sitter-oriscript` — binding C da grammar OriScript

## Limitações (P2)

- Reparse completo (não incremental por edit) — ok para arquivos médios
- Cores de syntax ainda não vêm do TOML (só UI base)
- TypeScript usa o grammar JS (aproximado)
- Sem semantic tokens / LSP ainda (P3)
