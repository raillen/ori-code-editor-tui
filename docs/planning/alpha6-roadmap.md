# Oride — plano de implementação `0.1.0-alpha.6` e além

**Status:** normativo a partir de 2026-07-13  
**Release atual definida:** **`0.1.0-alpha.6`**  
**Precedência:** este doc > `post-0.1-roadmap.md` (histórico) > notas soltas em `design.md`  
**Produto:** TUI IDE **contida** (tudo no processo Oride + terminal do usuário). Sem bloat.

---

## 1. Princípios (anti-bloat)

| # | Princípio | Implicação |
|---|-----------|------------|
| P1 | **Contido no TUI** | Features vivem em ratatui/crossterm/PTY; não spawna browser de preview MD |
| P2 | **Um conceito por PR** | Fatias pequenas; sem “framework de plugins externos” cedo |
| P3 | **First-class languages finitas** | Só a lista §3; resto = Plain ou highlight genérico depois |
| P4 | **Git via CLI** | Sem libgit2 até dor real; porcelain + blame + diff já existem |
| P5 | **Fail closed / status line** | LSP/git/PTY falham com mensagem, não crash |
| P6 | **Docs no mesmo slice** | Spec/README/CHANGELOG junto da feature user-facing |
| P7 | **Preferir maturar o que existe** | Antes de novo painel, polish do fluxo já shipped |

### Explicitamente **fora de escopo** (não implementar)

- Macros (record/play) — **remover** se ainda no código; não expandir  
- Preview Markdown **HTML/browser** / Typora-like externo  
- Host de plugins Lua/WASM/dynload  
- Full vim modal default  
- Undo tree ramificado visual  
- Telescope multi-source monstro  
- DAP/debugger, collab, cloud  
- Inlay hints densos (salvo se LSP OriScript entregar e for 1 toggle simples)  
- Replace-in-project “IDE monstro” (só se caber em fatia mínima depois de languages)

---

## 2. Baseline `0.1.0-alpha.6` (o que já conta como feito)

Congela e documenta o estado **já no tree** sob a versão **0.1.0-alpha.6**:

| Área | Estado |
|------|--------|
| Core editor | rope, tabs, undo/redo, seleção, soft wrap, comment toggle |
| Árvore | expand, create file/dir, git badges |
| Terminal | PTY interativo, resize, foco, Ctrl no shell |
| Find | buffer (case/accent/word/regex) + project (`rg`/fallback) |
| Git | status tree, SCM panel, blame status, diff read-only |
| MD | highlight + fence inject + preview TUI + image **placeholder** |
| LSP | OriScript (diagnostics, complete, hover, goto, format) |
| Layout | menu, banner, splits (2), multi-cursor, mouse **opt-in** (`mouse=false`) |
| UX | which-key, welcome, buffer picker, jump list, multi-picker MVP, surround MVP |
| Plugins | built-in `LanguageProvider` + 2 commands (sem host externo) |

**Gate de release alpha.6:** `cargo fmt` · `clippy -D warnings` · `cargo test --workspace` · version bump · CHANGELOG com seção **0.1.0-alpha.6** · docs sync (este plano).

---

## 3. Linguagens first-class (alvo)

Ordem de maturidade por linguagem: **detect → highlight → comment/indent → (opcional) LSP**.

| ID | Extensões típicas | Highlight | LanguageProvider | LSP no Oride |
|----|-------------------|-----------|------------------|--------------|
| **OriScript** | `.oris` | tree-sitter (já) | já | `oriscript lsp` (já) |
| **Ori (ori-lang)** | `.orl` (confirmar monorepo) | **adicionar** grammar/TS ou queries | sim | se CLI/LSP existir no PATH; senão skip |
| **Markdown** | `.md`, … | já (+ inject) | já | não |
| **HTML** | `.html`, `.htm` | já | já | não no alpha |
| **CSS** | `.css` | já | já | não no alpha |
| **JavaScript** | `.js`, `.mjs`, `.cjs`, `.jsx` | já | já | não no alpha |
| **TypeScript** | `.ts`, `.tsx` | **separar** de JS se grammar TS disponível; senão JS grammar + id `typescript` | sim | opcional `typescript-language-server` depois |
| **Rust** | `.rs` | tree-sitter-rust | sim | opcional `rust-analyzer` depois |
| **Python** | `.py` | tree-sitter-python | sim | opcional `pylsp`/`pyright` depois |
| **Nim** | `.nim` | tree-sitter-nim (se crate estável) ou highlight razoável | sim | opcional |
| **Ruby** | `.rb` | tree-sitter-ruby | sim | opcional |

**Regra de contensão:** no ciclo alpha.6→0.2 só **highlight + provider + fence inject**. Multi-LSP genérico = fatia própria (L2), não bloqueia languages.

**Fence inject MD:** aliases para todas as langs first-class (` ```rust `, ` ```python `, ` ```nim `, ` ```ruby `, ` ```oris `, ` ```orl `, …).

---

## 4. Roadmap por fatias (DAG)

```text
R0  Release hygiene alpha.6 ─────────────────────────────┐
R1  Remove macros / anti-bloat cleanup ──────────────────┤
                                                          │
L1  Languages matrix (highlight+provider+fences) ────────┼─► L2 LSP multi (opt-in, 1 server/config)
                                                          │
M1  MD links → system browser (in-TUI hit + open) ───────┤
M2  MD images via terminal graphics (Kitty/Sixel/iterm) ─┤
                                                          │
E1  Editor polish (session layout, replace-project min) ─┤
G1  Git mínimo (stage+commit CLI) ───────────────────────┤
                                                          ▼
                    0.2.0 “languages + MD media + hygiene”
```

### R0 — Release `0.1.0-alpha.6` (este slice de docs/versão)

| Entrega | Gate |
|---------|------|
| `workspace.package.version = 0.1.0-alpha.6` | Cargo |
| CHANGELOG: mover Unreleased → `## 0.1.0-alpha.6` | leitura |
| README status line = alpha.6 | ok |
| Este plano + pointer nos docs antigos | ok |
| `cargo test --workspace` + clippy | CI local |

### R1 — Anti-bloat / higiene

| ID | Entrega | Não fazer |
|----|---------|-----------|
| **R1.1** | Remover actions/UI de **macro** (F9/F10, menu, keymap, estado) | “melhorar macros” |
| **R1.2** | Remover menções a preview HTML/browser do docs | implementar browser |
| **R1.3** | Marcar multi-picker/surround como estáveis MVP (sem expandir) | telescope monstro |
| **R1.4** | Changelog + help keybinds sem macros | — |

### L1 — Languages first-class (prioridade alta)

| ID | Entrega | Gate |
|----|---------|------|
| **L1.0** | `LanguageId` + `detect_language` para rust/python/ts/nim/ruby/ori-lang | testes path |
| **L1.1** | Deps tree-sitter oficiais (rust, python, typescript, ruby; nim se crate ok; ori-lang grammar path/submodule se existir) | compile size sanity |
| **L1.2** | Queries highlight mínimas (keyword/string/comment/function) por lang | snapshots ou asserts spans |
| **L1.3** | `LanguageProvider` + comment syntax + soft_wrap default | toggle comment em fixture |
| **L1.4** | Fence inject aliases MD para todas | teste inject |
| **L1.5** | Docs `syntax.md` + README tabela langs | living-docs |

**Ordem de implementação sugerida (custo/benefício):**  
Rust → Python → TypeScript → Ruby → Nim → Ori-lang (grammar do monorepo).

**Δ binário:** tree-sitter grammars aumentam o binário; aceitar strip release; não embutir 20 langs extras.

### L2 — LSP (depois de L1; contido)

| ID | Entrega | Gate |
|----|---------|------|
| **L2.1** | Config `[lsp.servers]` map lang → argv (default só oriscript) | TOML |
| **L2.2** | Um client ativo por workspace **ou** N clients preguiçosos por lang aberta | sem crash se offline |
| **L2.3** | Paridade mínima: diagnostics + hover + goto (complete se trivial) | smoke |
| **L2.4** | **Não** obrigar rust-analyzer/etc. no CI | skip se binário ausente |

### M1 — Links clicáveis no preview MD (in-TUI)

| ID | Entrega | Gate |
|----|---------|------|
| **M1.1** | Preview guarda spans de link com URL + rect por linha | unit |
| **M1.2** | Clique (mouse on) no preview → `xdg-open` / `open` / `cmd start` na URL ou path | manual |
| **M1.3** | Teclado: Enter com caret na linha do link (ou ação “Open link under cursor”) se mouse off | status |
| **M1.4** | Só `http(s):`, `mailto:`, paths relativos seguros (sem shell injection) | testes URL |

**Não** é preview no browser do MD; só **abre o alvo do link** no sistema.

### M2 — Imagens no terminal (opcional, best-effort)

| ID | Entrega | Gate |
|----|---------|------|
| **M2.1** | Detectar capability: Kitty graphics / iTerm2 inline / Sixel (um backend MVP: **Kitty** primeiro) | feature detect |
| **M2.2** | No preview, se local file image + capability: render **inline** (altura limitada, ex. 8–12 células) | manual Kitty |
| **M2.3** | Fallback: card placeholder atual (✓/✗ path) | não regredir |
| **M2.4** | Config `markdown.terminal_images = true` default **false** até estável | TOML |
| **M2.5** | Docs: “requer Kitty/WezTerm com protocol X; GNOME Terminal = placeholder” | ok |

**Não** abrir viewer externo de imagem como feature principal (pode ser ação secundária “Open externally” no mesmo card se trivial).

### E1 — Editor polish contido

| ID | Entrega | Gate |
|----|---------|------|
| **E1.1** | Session: restaurar scroll_y + soft_wrap + show_tree/scm/term heights | roundtrip |
| **E1.2** | Project find: glob opcional simples (`*.rs`) | teste |
| **E1.3** | Replace-in-project **mínimo** (lista hits → confirm all / one) **só se** L1 estável; senão adiar 0.2.1 | fail closed |
| **E1.4** | Syntax colors from TOML (map HighlightKind → cor) se ainda incompleto | visual |

### G1 — Git mínimo (CLI)

| ID | Entrega | Gate |
|----|---------|------|
| **G1.1** | SCM: `s` stage path, `u` unstage | status refresh |
| **G1.2** | Commit message prompt → `git commit -m` | dirty tree clean |
| **G1.3** | Sem push forçado; `git push` só se ação explícita + status | sem default push |

---

## 5. Critérios de “first-class” (Definition of Done por linguagem)

Uma linguagem L está **first-class** quando:

1. `detect_language` estável por extensão  
2. Highlight não-vazio em fixture mínima  
3. Toggle comment correto  
4. Aparece em fence inject MD  
5. Listada em README/syntax.md  
6. (Opcional L2) LSP documentado em config, não obrigatório  

---

## 6. SemVer (produto Oride)

| Versão | Conteúdo |
|--------|----------|
| **0.1.0-alpha.6** | Congela baseline atual + este plano + hygiene R0/R1 início |
| **0.1.0-alpha.7+** | L1 languages em fatias; M1 links |
| **0.2.0** | L1 completo + M1 + M2 (images terminal best-effort) + E1.1–E1.2 + G1 opcional |
| **0.3.0** | L2 multi-LSP opt-in + polish |
| **≥0.4 / 1.0** | só com API estável e suite de regressão |

**Não** pular para 1.0 enquanto grammars/LSP ainda “best effort”.

---

## 7. Skills / validação por fatia

- Sempre: `clean-code`, `rust`, `living-docs`  
- Linguagens/highlight: + disciplina de `compiler-dev` leve (tests + CHANGELOG)  
- Gate:

```bash
cd /path/to/oride
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Release build size (observar, não bloquear cedo):

```bash
cargo build --release -p oride && ls -lh target/release/oride
```

---

## 8. Ordem de execução recomendada (um dev)

1. **R0** version + CHANGELOG + sync docs ← **agora**  
2. **R1.1** remover macros  
3. **L1** Rust → Python → TS → Ruby → Nim → Ori-lang  
4. **M1** links clicáveis  
5. **M2** Kitty images (flag off default)  
6. **E1.1** session layout  
7. **G1** se ainda couber no 0.2  

---

## 9. Riscos

| Risco | Mitigação |
|-------|-----------|
| Binário cresce com grammars | Só langs da lista; strip release; sem grammars “por precaução” |
| tree-sitter-nim frágil | Fallback highlight simples ou adiar Nim 1 minor |
| ori-lang grammar fora do repo | path dependency opcional / vendor mínimo; não quebrar build se ausente |
| Protocolos de imagem divergentes | Um backend (Kitty); fallback placeholder |
| Multi-LSP complexidade | L2 só após L1; um server por vez no MVP |

---

## 10. Checklist de sync de docs (R0)

- [x] `docs/planning/alpha6-roadmap.md` (este arquivo)  
- [x] `Cargo.toml` version alpha.6  
- [x] `CHANGELOG.md` seção 0.1.0-alpha.6  
- [x] `README.md` status  
- [x] `docs/planning/post-0.1-roadmap.md` → pointer “superseded”  
- [x] `docs/planning/ux-polish-plan.md` status atual  
- [x] `docs/markdown.md` futuro alinhado (sem browser preview)  
- [x] `docs/syntax.md` tabela langs alvo  
- [x] `docs/plugin-api.md` nota anti-bloat  

---

_Gerado para o ciclo alpha.6; atualizar checkboxes conforme PRs fecharem._
