# Plano: TUI code editor “IDE mini” (estilo Micro) para OriScript

## Context

Queremos um **editor de código em terminal (TUI)**, inspirado no [Micro](https://micro-editor.github.io/), focado em **OriScript**, mas extensível a outras linguagens via highlight / completions / suggestions (plugins). Diferente do Micro “puro”, o produto inclui:

- Árvore de projeto navegável (criar pastas/subpastas/arquivos)
- Terminal embutido colapsável/expansível por atalho
- Suporte nativo a **Markdown, HTML, CSS, JS** (+ **OriScript** como linguagem de primeira classe)
- Configuração de atalhos, tema e linguagem via arquivo de config
- Ícones de arquivo/pasta (Nerd Fonts no TUI)
- Arquitetura **modular em Rust** (crates + traits) para features e plugins

**Decisões já tomadas:**

| Decisão | Escolha |
|----------|---------|
| Superfície | **TUI** (Ratatui / estilo Micro–Helix) |
| Local | **Repo/workspace separado** do OriScript |
| Escopo 0.1 | **“IDE mini” completa** (multi-tab, find/replace, git na árvore, LSP OriScript, temas, keymaps, command palette) |

**Por que repo separado:** o monorepo OriScript já é compiler + VM + LSP + extensions VS Code/Zed. O editor é um produto com ciclo de release, deps e UX próprios. Integração com OriScript via **PATH + `oriscript lsp`** (CLI first, alinhado a `docs/planning/tooling-ecosystem-plan.md` e `lsp-mvp-design.md`).

**Nome de trabalho:** `oride` (Ori + IDE). Pode ser renomeado antes do primeiro commit (`oriedit`, `spark`, etc.).

---

## Recomendação de stack

| Camada | Crate / abordagem | Por quê |
|--------|-------------------|---------|
| UI | `ratatui` + `crossterm` | Padrão TUI Rust 2024–26 |
| Buffer de texto | `ropey` | Rope eficiente, undo-friendly |
| Highlight | `tree-sitter` + grammars embutidas | Incremental; `.oris` já tem grammar em `editors/zed-oriscript/tree-sitter-oriscript` |
| Terminal embutido | `portable-pty` + parser VTE (`vte`) | PTY real + shell interativo |
| Config | **TOML** (`serde` + `toml`) | Comentários, legível, ecossistema Rust; JSON só se export/import for necessário |
| Keymap | crate próprio + TOML | Camadas: defaults → user → buffer-local |
| LSP client | JSON-RPC stdio (sem tower no host se possível) | Consome `oriscript lsp` e futuros servers |
| Git | `git2` **ou** subprocess `git status --porcelain` | Status na árvore; porcelain é mais simples e estável no MVP |
| Fuzzy / palette | `nucleo` ou `fuzzy-matcher` | Command palette + file finder |
| Watch FS | `notify` | Reload / refresh da árvore |
| Clipboard | `arboard` + OSC52 (SSH) | Como Micro |
| Ícones | mapa extensão → glyph Nerd Font | Fallback ASCII se fonte não detectada |
| Async I/O | `tokio` (LSP, PTY, notify) | Event loop UI síncrono + canais |

**Config: preferir TOML** (`~/.config/oride/config.toml` + `./.oride/config.toml` no projeto). Suportar comentários e overlays; evitar JSON como formato primário.

---

## Arquitetura modular (workspace)

```text
oride/                          # novo repositório
  Cargo.toml                    # workspace
  README.md
  docs/
    design.md                   # este plano (cópia canônica)
    plugin-api.md
    keymaps.md
  crates/
    oride-core/                 # Document, Buffer(rope), Selection, Undo, tabs
    oride-fs/                   # Project tree, create/rename/delete, watch
    oride-config/               # load/merge TOML, schema versionado
    oride-keymap/               # KeyChord → Action (enum + string para plugins)
    oride-theme/                # cores + estilos semânticos (syntax, ui, git)
    oride-syntax/               # LanguageId + tree-sitter queries
    oride-lsp/                  # client multi-server (stdio)
    oride-terminal/             # painel PTY, resize, focus
    oride-git/                  # status por path (M/A/D/? )
    oride-search/               # find/replace buffer + projeto (rg opcional)
    oride-plugin/               # trait Plugin + host + built-ins
    oride-ui/                   # widgets ratatui (tree, editor, status, palette, term)
    oride-app/                  # composition, event loop, layout
    oride/                      # binário CLI: `oride [path]`
  plugins/                      # built-ins (crates, feature = "builtin")
    lang-oriscript/
    lang-markdown/
    lang-web/                   # html, css, js
  assets/
    icons.toml                  # extensão → nerd glyph
    themes/
      default.toml
      dark.toml
    grammars/                   # tree-sitter vendored ou build script
```

### Princípios de modularidade

1. **Core sem UI** — `oride-core` não depende de `ratatui`. Testável sem TTY.
2. **Ações como dados** — teclas e command palette disparam `Action` (enum estável + string livre para plugins).
3. **LanguageProvider trait** — highlight, indents, comment string, LSP command, completions “offline”.
4. **Plugin host** no 0.1 = **crates Rust built-in** + trait; **API externa (Lua/WASM)** no 0.2+ (sem travar ABI nativo frágil).
5. **Fail closed** — LSP/Git/PTY com falha viram status line, não crash.
6. **CLI first com OriScript** — inteligência de `.oris` via `oriscript lsp` no PATH; sem reimplementar checker.

### Fluxo de eventos (alto nível)

```text
crossterm events ──► App
                      ├─ Keymap → Action
                      ├─ focus: Tree | Editor | Terminal | Palette | Dialog
                      ├─ DocumentStore (tabs, dirty, undo)
                      ├─ channels ◄── tokio: LSP / PTY / notify / git
                      └─ render(oride-ui)
```

Layout padrão:

```text
┌──────────┬─────────────────────────────┐
│  Tree    │  Tabs | buffer              │
│  📁 src  │  1:main.oris ●              │
│  📄 .oris│  … editor …                 │
│          ├─────────────────────────────┤
│          │  Diagnostics / search hits  │  (opcional, toggle)
├──────────┴─────────────────────────────┤
│  Terminal (toggle: Ctrl+`)  ▾ / ▸     │  altura 0 | 30% | maximizado
├────────────────────────────────────────┤
│  status: mode | lang | git | lsp | ln  │
└────────────────────────────────────────┘
```

---

## Funcionalidades pedidas (0.1)

| # | Feature | Notas de implementação |
|---|---------|------------------------|
| 1 | Editor multi-tab | `DocumentId`, dirty `●`, close com confirm se dirty |
| 2 | Árvore de projeto | Expand/collapse, keyboard + mouse se disponível |
| 3 | Criar pasta/arquivo | Dialog inline ou prompt na status; validar path |
| 4 | Terminal embutido | Toggle atalho; resize altura; focus cycle |
| 5 | Highlight nativo | `.oris`, `.md`, `.html`, `.css`, `.js` via tree-sitter |
| 6 | Completions / suggestions | LSP OriScript + keywords offline por linguagem |
| 7 | Config TOML | theme, language, keys, terminal shell, tree width |
| 8 | Ícones | `icons.toml` + Nerd Font; fallback |
| 9 | Temas | cores UI + scopes syntax |
| 10 | Keymaps custom | rebind de Actions; layers |
| 11 | Find/replace | buffer atual; regex opcional |
| 12 | Git status na árvore | M/A/D/? cores no nome |
| 13 | LSP OriScript | spawna `oriscript lsp` se `oris.proj` ou `.oris` |
| 14 | Command palette | fuzzy de Actions + “open file” |

---

## Funcionalidades **recomendadas** (além do pedido)

Prioridade para caber no 0.1 “IDE mini” sem virar monólito:

### P0 — incluir no 0.1 (alto valor / baixo-médio custo)

| Feature | Por quê |
|---------|---------|
| **Command palette** (`Ctrl+Shift+P`) | Descoberta de comandos; reduz dependência de memorizar keys |
| **Fuzzy open file** (`Ctrl+P`) | Essencial com árvore; fluxo Micro+VS Code |
| **Undo/redo por documento** | Não negociável em editor de código |
| **Status line** | Lang, LSP ready/error, branch, dirty, Ln:Col, encoding |
| **Painel de diagnostics** | Lista erros do LSP; jump com Enter |
| **Save all / “modified” indicator** | Multi-tab seguro |
| **Reload se arquivo mudou no disco** | `notify` + prompt se dirty |
| **Soft wrap** (toggle; default on em `.md`) | Markdown legível |
| **Comment toggle** (`Ctrl+/`) | Por LanguageProvider |
| **Bracket match + auto-indent** | Edição diária |
| **Clipboard sistema + OSC52** | Local e SSH |
| **Which-key / help overlay** (`Ctrl+G` ou `?`) | Onboarding estilo Micro |
| **Session leve** | Reabrir última pasta + lista de tabs (sem full workspace state) |
| **`.editorconfig` básico** | indent_size, tab/spaces, eol |

### P1 — logo após 0.1 (vale planejar a API já)

| Feature | Por quê |
|---------|---------|
| **Split horizontal/vertical de buffers** | Micro tem; complexifica layout — após layout estável |
| **Search in project** (`rg` ou walk+grep) | “Find in files” |
| **Multi-cursor** | Marca do Micro; caro em rope+LSP — pós-0.1 |
| **Markdown preview** (split read-only) | Diferencial DX docs |
| **Git gutter** (linha) + stage hunk | Além do ícone na árvore |
| **Format on save** | Via LSP `formatting` (já no OriScript) |
| **Rename / new file from tree context** | UX árvore completa |
| **Plugins externos (Lua ou WASM)** | Extensibilidade real sem recompilar |
| **DAP / debugger** | Só depois de stack traces OriScript estáveis |
| **Detecção automática de Nerd Font** | Mensagem amigável no first-run |

### Explicitamente **fora** do 0.1

- Debugger completo, remote collab, AI chat embutido, marketplace de plugins, GUI, multi-root workspace, remote SSH host (só editar via SSH local com OSC52).

---

## Plugins: modelo em duas camadas

### 0.1 — Built-in providers (Rust)

```rust
// oride-plugin (conceitual)
pub trait LanguageProvider: Send + Sync {
    fn id(&self) -> &str;                    // "oriscript"
    fn extensions(&self) -> &[&str];         // [".oris"]
    fn highlight_query(&self) -> Option<&str>;
    fn comment_token(&self) -> Option<&str>; // "//"
    fn lsp_command(&self) -> Option<LspSpawn>; // ["oriscript", "lsp"]
    fn offline_completions(&self, ctx: &CompletionCtx) -> Vec<CompletionItem>;
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_action(&mut self, action: &str, ctx: &mut PluginCtx) -> PluginResult;
    fn commands(&self) -> &[CommandMeta];    // aparecem na palette
}
```

Registro estático no binário (ou `inventory` / explicit `register` em `main`):

- `lang-oriscript` — grammar + LSP `oriscript lsp`
- `lang-markdown`, `lang-web` — highlight + indent; sem LSP no 0.1 (opcional `vscode-html` etc. depois)

### 0.2+ — Host externo

| Opção | Prós | Contras |
|-------|------|---------|
| **Lua** (como Micro) | Familiar, scripts simples | FFI/embed, sandbox fraco |
| **WASM** (estilo Lapce/Extism) | Sandbox, multi-lang | Mais infra |
| **Rhai** | 100% Rust embed | Menos ecossistema |

**Recomendação:** desenhar `PluginCtx` estável no 0.1; escolher **WASM ou Lua no 0.2** sem quebrar Actions/TOML.

---

## Configuração (TOML)

Exemplo de superfície (não normativa até implementar):

```toml
# ~/.config/oride/config.toml
theme = "dark"
soft_wrap = false
show_line_numbers = true
icons = true                 # false → ASCII
file_icons = true

[editor]
tab_size = 4
insert_spaces = true
format_on_save = false

[terminal]
shell = "/bin/zsh"           # default $SHELL
default_height = 10          # linhas; 0 = colapsado
toggle_key = "ctrl+`"

[tree]
width = 28
show_hidden = false
git_status = true

[lsp]
oriscript_command = ["oriscript", "lsp"]
# timeout_ms = 10000

[keys]
"ctrl+s" = "save"
"ctrl+p" = "open_file_fuzzy"
"ctrl+shift+p" = "command_palette"
"ctrl+`" = "toggle_terminal"
"ctrl+b" = "toggle_tree"
"ctrl+f" = "find"
"ctrl+h" = "replace"
"f2" = "rename_in_tree"      # p1 se não der tempo

[theme.ui]
background = "#1a1b26"
foreground = "#c0caf5"
# ...
```

Projeto local: `.oride/config.toml` sobrescreve user (merge profundo de seções).

---

## Integração OriScript

| Capacidade | Fonte |
|------------|--------|
| Diagnostics / hover / goto / completion / format | `oriscript lsp` (stdio) |
| Detectar projeto | `oris.proj` na raiz aberta ou ancestral |
| Grammar highlight | copiar/adaptar `tree-sitter-oriscript` do monorepo (submodule ou crate path opcional) |
| Run | Action `run_project` → `oriscript run` no terminal embutido ou job buffer |

O editor **não** linka `oris-*` no 0.1 (evita acoplar releases). Opcional depois: crate `oris-lsp-types` se compartilharmos tipos.

---

## Plano de implementação por fatias (PRs)

Cada PR = um conceito; ordem topologicamente segura.

### Fase 0 — Fundação (1–2 PRs)

| ID | Entrega | Gate |
|----|---------|------|
| **P0.1** | Workspace Cargo, bin `oride`, `oride-core` (rope + undo + seleção), testes unitários | `cargo test` |
| **P0.2** | `oride-ui` + loop: abrir arquivo, editar, salvar, quit, status line | demo TUI manual |
| **P0.3** | `oride-config` TOML + `oride-keymap` + `oride-theme` default | rebind `ctrl+s` via TOML |

### Fase 1 — IDE shell

| ID | Entrega | Gate |
|----|---------|------|
| **P1.1** | Multi-tab + dirty + close confirm | 3 tabs smoke |
| **P1.2** | `oride-fs` árvore + expand + open file + create file/dir | criar `src/a.oris` pela UI |
| **P1.3** | Ícones + git status na árvore | repo git real |
| **P1.4** | Terminal embutido toggle/focus/resize | `ls` interativo |
| **P1.5** | Command palette + fuzzy open | `Ctrl+P` / palette |

### Fase 2 — Linguagens

| ID | Entrega | Gate |
|----|---------|------|
| **P2.1** | `oride-syntax` tree-sitter + MD/HTML/CSS/JS | highlight visual |
| **P2.2** | Provider OriScript + grammar | `.oris` colorido |
| **P2.3** | Comment toggle, indent, soft wrap md | edição confortável |
| **P2.4** | Find/replace no buffer | regex opcional |

### Fase 3 — Inteligência

| ID | Entrega | Gate |
|----|---------|------|
| **P3.1** | `oride-lsp` client + diagnostics panel | `oriscript lsp` com fixture |
| **P3.2** | Hover / completion / goto (UI) | projeto `.oris` de exemplo |
| **P3.3** | Format (LSP) + format on save config | roundtrip `oriscript fmt` |

### Fase 4 — Polimento 0.1

| ID | Entrega | Gate |
|----|---------|------|
| **P4.1** | Help overlay, clipboard OSC52, editorconfig, session | checklist UX |
| **P4.2** | README, install script, `docs/plugin-api.md` (trait 0.1) | first-run docs |
| **P4.3** | Suite de testes headless (core/keymap/config/fs) + smoke script | CI verde |

**SemVer produto editor:** começar `0.1.0-alpha` até P3 estável; `0.1.0` com P4.

---

## Estrutura de diretórios no primeiro commit

```text
oride/
  Cargo.toml
  README.md
  LICENSE
  crates/oride-core/...
  crates/oride/...
  docs/design.md
```

Scaffold inicial só com `oride-core` + binário vazio/`hello buffer` — depois preencher conforme fases.

---

## Riscos e mitigações

| Risco | Mitigação |
|-------|-----------|
| Terminal embutido (PTY) complexo no TUI | Isolar crate cedo; fallback “abrir `$SHELL` externo” se PTY falhar |
| Tree-sitter build / grammars | Vendor grammars + `cc` build; CI com cache |
| Multi-cursor / splits atrasam 0.1 | Fora do escopo; API de Selection já lista de ranges se possível |
| Acoplar ao monorepo OriScript | Só LSP/PATH; grammar copiada com NOTICE |
| Plugin ABI nativo | Não expor `cdylib` no 0.1; só traits internos |
| Escopo “IDE mini” estourar | Cortes: multi-cursor, project search, preview md, rename tree → P1 |

---

## Verification (como validar o 0.1)

1. **Unit:** `oride-core` (insert/delete/undo), keymap resolve, config merge, tree create path.
2. **Integração headless:** abrir buffer de fixture, apply actions sem TTY (`App::from_test`).
3. **Manual TUI checklist:**
   - Abrir pasta com `oris.proj` + `main.oris`
   - Highlight `.oris` / `.md` / `.html`
   - Criar subpasta e arquivo pela árvore
   - Terminal: toggle, rodar `oriscript run`, colapsar
   - Introduzir erro de tipo → diagnostic no painel
   - Completion / hover / goto
   - Rebind tecla em `config.toml` e reiniciar (ou hot-reload se implementado)
   - Git: modificar arquivo → status `M` na árvore
   - Multi-tab dirty save
4. **CI:** `cargo fmt`, `clippy -D warnings`, `cargo test --workspace` no repo `oride`.

---

## Relação com este worktree

Este worktree (`micro-like-editor`) é o **OriScript**. O editor **não** substitui este repo.

Próximo passo de execução (após aprovação):

1. Criar repositório/workspace **`oride`** (path a confirmar, ex. `~/Documentos/Projetos/oride` ou sibling de `projetos-ori-script`).
2. Scaffold Fase 0 (workspace + `oride-core` + bin).
3. Copiar/adaptar este plano para `oride/docs/design.md`.
4. Opcional: link no README do OriScript (“Editor TUI oficial: oride”).

---

## Resumo da recomendação

- **TUI modular em Rust** com workspace de crates e **Actions** como lingua franca.
- **TOML** para config/keymaps/themes.
- **Tree-sitter + LSP** (não reinventar checker).
- **Plugins 0.1 = LanguageProvider/Plugin em Rust**; externos no 0.2.
- **0.1 “IDE mini”** = árvore + tabs + terminal + git tree + palette + find/replace + highlight nativo + LSP OriScript + temas/keys — **sem** multi-cursor, splits e marketplace.
- **Extras prioritários:** palette, fuzzy open, diagnostics panel, undo, clipboard, help, editorconfig, session leve.
