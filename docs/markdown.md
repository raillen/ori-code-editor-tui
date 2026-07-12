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
4. **Injections (P6):** conteúdo de ` ```lang ` re-highlight com grammar da linguagem

Aliases de fence suportados: `oris` / `oriscript`, `js` / `javascript`, `html`, `css` (e sinônimos em `fence_lang_alias`).

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

## Futuro (plano pós-0.1)

Decisões e fatias: **[`docs/planning/post-0.1-roadmap.md`](planning/post-0.1-roadmap.md)** (P6 injections, P7 preview ANSI).

| Item | Descrição | Fatia |
|------|-----------|--------|
| **Highlight em code fences** | Injection: ` ```oris `/`js`/… com grammar da linguagem | **P6** |
| **Preview Markdown ANSI** | Painel TUI read-only (`Ctrl+Shift+V` / `Alt+P`) | **P7 feito** |
| **Parse JSX real no MDX** | Além do highlight MD genérico | depois de P6/P7 |
| Preview HTML/browser | Opcional; não prioritário | major futura |

Outros desejáveis: wiki-links, frontmatter YAML, outline de headings na palette.

## Validação

```bash
cargo test -p oride-syntax
# fixture:
# # Título
# **bold** `code`
# - item
```
