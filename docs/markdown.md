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

**Não** abre preview HTML/browser (fora de escopo). Imagens hoje = placeholder no TUI.

## Limitações atuais

- Soft wrap: scroll vertical usa linhas lógicas + altura visual
- Preview de imagem = placeholder (não pixels), exceto quando **M2** (protocolo do terminal) estiver on
- Tabelas sem alinhamento de colunas avançado
- Links ainda não são clicáveis (planejado **M1**)

## Roadmap MD (normativo)

Ver **[`docs/planning/alpha6-roadmap.md`](planning/alpha6-roadmap.md)** seções **M1** / **M2**.

| Item | Descrição | Status |
|------|-----------|--------|
| Highlight + fence inject | oris/js/html/css (+ langs L1) | feito / expandir L1 |
| Preview Markdown TUI | painel read-only | feito + placeholders |
| **Links → browser do sistema** | clique/ação no preview; não é “preview no browser” | **M1 planejado** |
| **Imagens no terminal** | Kitty/Sixel/iTerm best-effort; flag off default | **M2 planejado** |
| Preview HTML/browser do documento | — | **fora de escopo** |
| MDX/JSX real | — | fora (contido) |

## Validação

```bash
cargo test -p oride-syntax
# fixture:
# # Título
# **bold** `code`
# - item
```
