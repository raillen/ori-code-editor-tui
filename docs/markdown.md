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

## Preview Markdown (TUI)

Atalho: `Ctrl+Shift+V` / `Alt+P`. Painel read-only ao lado do editor; **segue o scroll**.

### O que renderiza (texto)

| Elemento | No preview |
|----------|------------|
| Headings `#` / setext | Hierarquia visual + espaço |
| Listas / task lists | `•` · `☐` / `☑` |
| Blockquote | `│` + itálico/quote |
| Code fence | Bloco `┌ lang` … `│` … `└` |
| Inline `code` **bold** *italic* ~~strike~~ | Estilos dedicados |
| Links `[t](url)` | `t → url` (url truncada) |
| Tabelas `\|` | Linhas com `│` |
| Frontmatter `---` | Bloco dim no topo |
| **Imagens** `![alt](path)` | **Placeholder** (não bitmap) |

### Imagens (placeholder)

Linha só com imagem vira card:

```text
┌ 🖼  imagem
│  alt text
│  /path/relativo.png
│  ✓ arquivo local encontrado   (ou ✗ / URL remota)
└
```

- Paths relativos resolvem contra a **pasta do `.md` aberto**
- `http(s)://` → nota “URL remota · não embutida”
- Inline no meio do parágrafo → chip compacto `🖼 alt`

**Não** desenha PNG/JPEG no terminal (limitação TUI). Viewer externo = futuro opcional.

## Limitações atuais

- Soft wrap: scroll vertical usa linhas lógicas + altura visual
- Preview de imagem = placeholder, não pixels
- Tabelas sem alinhamento de colunas avançado

## Futuro (plano pós-0.1)

Decisões e fatias: **[`docs/planning/post-0.1-roadmap.md`](planning/post-0.1-roadmap.md)**.

| Item | Descrição | Status |
|------|-----------|--------|
| Highlight em code fences | Injection oris/js/… | feito (P6) |
| Preview Markdown ANSI | Painel TUI | feito + placeholders |
| Abrir imagem no viewer do SO | `xdg-open` no path | futuro |
| Protocolo Kitty/Sixel | imagem no terminal | opcional / frágil |
| Preview HTML/browser | imagens reais | major futura |
| Parse JSX no MDX | além do MD genérico | futuro |

Outros desejáveis: wiki-links, outline de headings na palette.

## Validação

```bash
cargo test -p oride-syntax
# fixture:
# # Título
# **bold** `code`
# - item
```
