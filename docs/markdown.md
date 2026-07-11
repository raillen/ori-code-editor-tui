# Markdown no Oride

Suporte nativo a Markdown e derivados (antes do LSP).

## Extensões

| Extensão | LanguageId |
|----------|------------|
| `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt`, `.mdtext` | `markdown` |
| `.rmd`, `.qmd` | `markdown` |
| `.mdx` | `mdx` (highlight como MD) |
| `README`, `CHANGELOG`, `LICENSE`, … | `markdown` |

## Highlight

Pipeline **tree-sitter-md**:

1. Grammar **block** (headings, listas, code fences, tables, quotes)
2. Grammar **inline** (bold, italic, links, `code` spans)
3. Queries oficiais + fallback por `node.kind()`

Cores dedicadas: heading, emphasis, strong, link, code, list marker, quote.

## Funções de edição

| Ação | Atalho default | Comportamento |
|------|----------------|---------------|
| Soft wrap | `Alt+Z` | Liga/desliga quebra visual; **default on** ao abrir MD |
| Toggle comment | `Ctrl+/` | `<!-- linha -->` em MD/HTML; `//` em código |
| Enter em lista | `Enter` | Continua `- `, `* `, `1. `, `- [ ] `, `> ` |
| Enter em marcador vazio | `Enter` | Sai da lista (remove o marcador) |

Também na command palette: **Toggle soft wrap**, **Toggle comment**.

## Limitações

- MDX: sem parse JSX (só highlight MD)
- Soft wrap: scroll ainda é por linha lógica
- Sem preview renderizado (pós-0.1)
- Code fences não re-highlight a linguagem interna (P3+/injections)

## Validação

```bash
cargo test -p oride-syntax
# fixture mental:
# # Título
# **bold** `code`
# - item
```
