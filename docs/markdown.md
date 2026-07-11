# Markdown no Oride

Suporte nativo a Markdown e derivados (antes do LSP).

## Extensões

| Extensão | LanguageId |
|----------|------------|
| `.md`, `.markdown`, `.mdown`, `.mkd`, `.mkdn`, `.mdwn`, `.mdtxt`, `.mdtext` | `markdown` |
| `.rmd`, `.qmd` | `markdown` |
| `.mdx` | `mdx` (highlight como MD) |
| `README`, `CHANGELOG`, `LICENSE`, … | `markdown` |

## Highlight (atual)

Pipeline **tree-sitter-md**:

1. Grammar **block** (headings, listas, code fences, tables, quotes)
2. Grammar **inline** (bold, italic, links, `code` spans)
3. Queries oficiais + fallback por `node.kind()`

Cores dedicadas: heading, emphasis, strong, link, code, list marker, quote.

## Funções de edição (atual)

| Ação | Atalho default | Comportamento |
|------|----------------|---------------|
| Soft wrap | `Alt+Z` | Liga/desliga quebra visual; **default on** ao abrir MD |
| Toggle comment | `Ctrl+/` | `<!-- linha -->` em MD/HTML; `//` em código |
| Enter em lista | `Enter` | Continua `- `, `* `, `1. `, `- [ ] `, `> ` |
| Enter em marcador vazio | `Enter` | Sai da lista (remove o marcador) |

Também na command palette: **Toggle soft wrap**, **Toggle comment**.

## Limitações atuais

- Soft wrap: scroll ainda é por linha lógica

## Futuro (não no 0.1)

Planejado **após** o freeze de polimento 0.1 / LSP básico — **não** bloquear o roadmap atual:

| Item | Descrição | Quando (estimado) |
|------|-----------|-------------------|
| **Preview renderizado do Markdown** | Painel split (ou overlay) com HTML/ANSI renderizado do buffer `.md` | pós-0.1 (P1 design) |
| **Highlight da linguagem dentro de code fences** | Injections tree-sitter: conteúdo de ` ```rust ` colorido com grammar da linguagem | pós-0.1 / polish syntax |
| **Parse JSX real no MDX** | Grammar/injections para JSX em `.mdx` (não só highlight MD) | pós-0.1 language pack |

Outros desejáveis MD (também futuros): wiki-links, frontmatter YAML colorido, outline de headings na palette.

## Validação

```bash
cargo test -p oride-syntax
# fixture:
# # Título
# **bold** `code`
# - item
```
