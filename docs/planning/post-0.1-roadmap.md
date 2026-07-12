# Roadmap pós–0.1 — plano de implementação

**Status do produto hoje:** `0.1.0-alpha.5` (P0–P4 + P3 LSP básicos).  
**Este doc:** ordem e fatias do que vem **depois** do freeze de mini-IDE, com decisões já tomadas.

**Precedência:** decisões aqui > notas genéricas em `docs/design.md` § P1.

---

## Decisões de produto (fixadas)

| # | Tema | Decisão |
|---|------|---------|
| 1 | **Preview Markdown** | Painel **read-only no TUI** com render **ANSI** (não browser) |
| 2 | **Search in project** | Implementar com **`Ctrl+Shift+F`**; backend **`rg` se no PATH** + **fallback 100% Rust** |
| 3 | **Splits** | **Adiar** para minor/major futuras (não no próximo cluster imediato) |
| 4 | **Multi-cursor** | **Adiar** junto com splits |
| 5 | **Plugins externos** | **Incrementar** a partir do 0.1: traits + hooks, sem host Lua/WASM ainda |
| 6 | **Injections em fences** | **Sim** — highlight da linguagem *dentro* de ` ```lang ` em Markdown |

### Ordem de prioridade de implementação

```text
P5  Search in project          ← próximo a implementar
P6  Fence language injections  ← MD/docs
P7  MD preview ANSI (painel)   ← preview TUI
P8  Plugin surface (incremento)
——  depois (minor/major) ——
P9+ Splits de buffer
P9+ Multi-cursor
P9+ Preview browser / MDX JSX / etc. (opcional)
```

SemVer sugerido (produto editor):

| Versão | Conteúdo |
|--------|----------|
| **0.1.x** | bugfix / polish fino do alpha atual |
| **0.2.0** | P5 + P6 (+ P7 se couber) — search + MD highlight rico + preview |
| **0.3.0** | P8 plugins built-in estáveis + mais LSP |
| **0.4+ / 1.x** | splits, multi-cursor, host externo de plugins |

---

## P5 — Search in project (`Ctrl+Shift+F`)

**Status:** implementado em `0.1.0-alpha.5+` (crate `oride-search` + UI).

### Objetivo

Buscar texto (e opcionalmente regex) em **todo o workspace**, listar hits, saltar para arquivo+linha.

### UX

| Elemento | Comportamento |
|----------|----------------|
| Atalho | `ctrl+shift+f` → action `project_find` |
| UI | Barra/modal compacto (estilo find de buffer) **ou** painel inferior tipo diagnostics |
| Query | digitar · `Alt+C` case · `Alt+R` regex (paridade com find local) |
| Lista | `path:line:col  trecho` · ↑↓ · Enter abre + posiciona caret |
| Esc | fecha; não altera buffer se cancelar |
| Status | `N hits · backend: rg\|rust` |

### Arquitetura

```text
oride-search/  (crate novo, sem UI)
  ProjectSearch::run(root, QueryOptions) -> Vec<SearchHit>
    ├─ try_ripgrep()  se `which rg` / spawn ok
    └─ fallback_walk()  walkdir + regex/literal, skip target/node_modules/.git
oride-app
  Overlay::ProjectFind { query, selected, hits, opts }
  Action::ProjectFind
```

### Fatias (PRs)

| ID | Entrega | Gate |
|----|---------|------|
| **P5.1** | Crate `oride-search` + fallback Rust + testes com tempdir | `cargo test -p oride-search` |
| **P5.2** | Backend `rg` (JSON ou linha `--vimgrep`) + detecção automática | teste mock ou skip se sem `rg` |
| **P5.3** | UI `Ctrl+Shift+F` + jump + keybind default + help list | E2E manual + unit overlay |
| **P5.4** | Opções: case, regex, globs simples (`*.oris`) | docs `config.md` / polish |

### Fora de P5

- Replace in project (depois)
- Índice persistente / watcher de index

### Dependências workspace

- `walkdir` ou `ignore` (respeitar `.gitignore` no fallback — preferir `ignore`)
- `regex` (já no workspace)

---

## P6 — Language injections em code fences

**Status:** implementado (MVP) — `collect_markdown_spans` + `highlight_language_slice`.

### O que é (resumo)

Em Markdown, o bloco:

````markdown
```rust
fn main() {}
```
````

hoje é um “pedaço de code” monócromo. **Injection** = rodar a grammar **Rust** (ou OriScript, JS…) **só nos bytes internos** do fence e misturar spans no highlight do editor.

Não executa código; não é preview.

### Objetivo

- Detectar `info string` do fence (`rust`, `rs`, `oris`, `oriscript`, `js`, `javascript`, `html`, `css`, `md`…)
- Mapear → `LanguageId` / grammar já existente em `oride-syntax`
- Produzir `HighlightSpan` com offsets **absolutos no buffer**

### Arquitetura

```text
oride-syntax
  markdown.rs (ou injections.rs)
    extract_fences(source) -> Vec<Fence { byte_range, lang }>
    inject_highlight(source, base_md_spans) -> Vec<HighlightSpan>
      for each fence:
        run highlight engine for lang on slice
        remap local offsets += fence.start
oride-app / HighlightEngine
  se language == Markdown|Mdx: pipeline MD + inject
```

### Fatias

| ID | Entrega | Gate |
|----|---------|------|
| **P6.1** | Extrator de fences (tree-sitter-md ou regex conservadora) + testes offset | unit tests |
| **P6.2** | Injection para langs já no Oride (oris, js, html, css) | snapshot/visual fixture |
| **P6.3** | Soft wrap + seleção não corrompem spans (offsets byte estáveis) | regressão editor |
| **P6.4** | Docs `markdown.md` — tabela de aliases ` ```lang ` | living-docs |

### Limitações conscientes (P6)

- Sem grammar nova (ex. Python) — só o que já existe no binário  
- MDX JSX real continua fora  
- Nested fences / info strings exóticas: best-effort  

---

## P7 — Preview Markdown ANSI (painel TUI)

**Status:** implementado — `md_preview` + painel 55/45 · `Ctrl+Shift+V` / `Alt+P`.

### Objetivo

Com buffer MD focado (ou toggle explícito), mostrar **painel read-only** com texto “quase renderizado” em ANSI: headings em bold/cor, listas com bullets, links sublinhados, code em monoespaçado/cor.

### UX

| Elemento | Comportamento |
|----------|----------------|
| Atalho | `ctrl+shift+v` (ou `alt+p`) → `toggle_md_preview` |
| Layout | Coluna direita **ou** painel sob o editor (sem splits genéricos de N buffers) |
| Conteúdo | Sempre o buffer MD **ativo**; scroll independente |
| Edição | Continua no editor; preview atualiza em debounce (~100–200 ms) |
| Não-MD | status “preview só para Markdown” |

**Nota:** isto **não** é split de buffers arbitrário (P9). É um **viewport derivado** do doc atual.

### Arquitetura

```text
oride-markdown/  (ou módulo em oride-syntax)
  render_ansi(source) -> String  // ou Vec<Line> com estilos
oride-ui
  render_md_preview(frame, area, lines)
oride-app
  show_md_preview: bool
  preview_scroll: usize
```

Render mínimo v1:

- ATx → bold + cor heading  
- listas `-` / `1.` → prefixo `•` / número  
- `inline code` e fences → cor `code`  
- `**bold**` / `*italic*` best-effort  
- links `[t](u)` → `t` underlined  

### Fatias

| ID | Entrega | Gate |
|----|---------|------|
| **P7.1** | `render_ansi` puro + testes de fixture MD | unit |
| **P7.2** | Widget + toggle + layout 50/50 editor\|preview | manual TUI |
| **P7.3** | Debounce + scroll preview + status | polish |
| **P7.4** | Integração com P6 (code blocks no preview com cor) | opcional no mesmo minor |

### Fora de P7

- HTML real / browser  
- Math, mermaid, HTML embutido  
- Editar pelo preview  

---

## P8 — Plugins: incremento (sem host externo)

### Objetivo

Deixar o 0.1 **preparado** para 0.2/0.3: superfície de extensão **built-in**, documentada, sem Lua/WASM ainda.

### Incrementos concretos

| ID | Entrega | Gate |
|----|---------|------|
| **P8.1** | Trait `LanguageProvider` em crate `oride-plugin` (ou `oride-lang`) + registro estático no bin | compile + 1 provider MD/ORIS refatorado |
| **P8.2** | `PluginCtx` mínimo: `open_path`, `set_status`, `active_buffer_text` (read) | testes fake plugin |
| **P8.3** | Commands de plugin na **command palette** (string action ids) | palette lista |
| **P8.4** | Hooks: `on_save`, `on_open` (built-ins: format_on_save já existe — alinhar) | docs plugin-api |
| **P8.5** | (0.3+) escolher **Lua ou WASM** — ADR em `docs/planning/` | design only até lá |

### Não fazer em P8

- Carregar `.so` / scripts do usuário  
- Marketplace  
- API instável exposta como estável  

Atualizar `docs/plugin-api.md` a cada fatia.

---

## Adiado (minor/major futuras)

### P9a — Splits de buffer

- N viewports editáveis, focus cycle, H/V split  
- Modelo de “tab groups” ou “pane → document id”  
- **Depende de:** layout engine refatorado (P7 pode ensaiar painel fixo, mas splits genéricos são outro salto)

### P9b — Multi-cursor

- `Vec<Selection>` + edit batch + undo group  
- Render N carets  
- **Depende de:** modelo de seleção estável; idealmente **depois** de splits (foco por pane)

### Explicitamente depois / opcional

- Preview MD no browser  
- Replace in project  
- Fence grammars extras  
- MDX JSX real  
- Git gutter / stage hunk  
- DAP  

---

## DAG de dependências

```text
P5.1 → P5.2 → P5.3 → P5.4
                ↓
P6.1 → P6.2 → P6.3 → P6.4
                ↓
         P7.1 → P7.2 → P7.3 → (P7.4 usa P6)
P8.1 → P8.2 → P8.3 → P8.4  (paralelo a P5–P7)
P9a / P9b  ── depois de 0.2/0.3 ──
```

**Paralelismo útil:** P5 ∥ P6 ∥ P8.1 no início de 0.2.  
**Sequência recomendada se um dev só:** P5 completo → P6 → P7 → P8.

---

## Gates de qualidade (todas as fatias)

1. Spec/docs no mesmo slice (`living-docs`)  
2. `cargo fmt` · `clippy -D warnings` · `cargo test --workspace`  
3. Keybind em `default_key_bindings` + aparece em **F1** (lista dinâmica)  
4. CHANGELOG user-facing  
5. Um PR ≈ um ID (P5.1, P6.2, …)  

Skills: `clean-code`, `rust`, `living-docs`; UI TUI com cuidado de layout; search com `ignore`/segurança de path.

---

## Primeiro passo imediato

**Implementar P5.1** — crate `oride-search` com fallback Rust + testes, sem UI.

Depois: P5.3 amarra `Ctrl+Shift+F` no app.

---

## Checklist de aceite 0.2.0 (proposta)

- [ ] `Ctrl+Shift+F` encontra string em multi-arquivo (rg e fallback)  
- [ ] Enter no hit abre arquivo na linha certa  
- [ ] Fence ` ```oris ` / ` ```js ` com keywords coloridos  
- [ ] Toggle preview MD ANSI ao lado/abaixo do editor  
- [ ] `LanguageProvider` + docs plugin-api atualizados  
- [ ] CI verde  

---

*Gerado a partir das decisões do usuário (preview ANSI, Ctrl+Shift+F, rg+fallback Rust, injections sim, plugins incrementar, splits/multi-cursor depois).*
