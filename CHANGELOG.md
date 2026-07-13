# Changelog

## Unreleased

### Added (mouse + tier B)

- **Mouse completo** (default on, `mouse = false` no TOML desliga):
  - Clique editor → caret; drag → seleção; duplo → palavra; triplo → linha
  - Clique árvore/abas/SCM/terminal/menu → foco / ativar
  - Scroll wheel por painel (editor, árvore, SCM)
  - Botão direito → which-key
- **Surround** (`F8`): envolve seleção com `()[]{}<>"'``
- **Macro** (`F9` grava/para · `F10` play)
- **Multi-picker** (`Ctrl+Shift+T`): buffers + commands + files
- **Undo history** (`Ctrl+Shift+U`): lista resumos e desfaz até o item
- **Fora deste corte (tier B restante):** full vim modal, inlay hints densos, telescope monstro, session workspace avançada

### Added (find)

- **Palavra completa** no find/replace (`Alt+W`): não casa substring dentro de identificador (ex.: `UI` em `GUI`)
- **Modal Find/Replace redesenhado**: campos separados, 1 opção por linha com atalho, contagem isolada (menos “amontoado”)
- Labels do modal mais claros: case / accent / word / re

### Added (markdown preview)

- **Placeholder de imagem** `![alt](path)`: card com alt, path e status (local ✓/✗ ou URL remota)
- Paths de imagem relativos à pasta do `.md` aberto
- Preview texto mais rico: task lists, tabelas, setext headings, frontmatter, `~~strike~~`, `__bold__`, autolink, links `→ url`

### Fixed

- **Mouse drag selection lag**: drena a fila de eventos e coalesca `Drag` antes de redesenhar (antes: 1 evento / frame ~100ms)
- **Duplo-clique na palavra**: tolera ±1 célula; seleciona palavra mesmo no fim do token; Unicode (`olá`); não inicia drag após double-click
- **Preview Markdown** segue o scroll do editor (não fica preso no topo); `Alt+↑/↓` só faz ajuste fino
- **Scroll do editor** acompanha o cursor de novo (viewport real + soft-wrap); cursor não some abaixo/acima da tela

### Added (UX polish — tiers S+A)

- **Terminal usável como shell do sistema**: spawn interativo (`-i`), Ctrl+A–Z no PTY (Ctrl+C/D/…), Esc→editor, borda ciano + erros de PTY
- **Menu bar** File/Edit/View/Go/Git/Help (`Alt+F`…, ↑↓ Enter, Esc)
- **Context banner** de alto contraste: `FOCUS: EDITOR|TREE|TERMINAL|SCM`
- **Status limpa**: file · Ln/Col · git:branch · blame · hint `F1 · Ctrl+Shift+P`
- **Which-key** (`Alt+/`) e **welcome essentials** na 1ª sessão
- **Find/replace mini-modal** centrado (`Ctrl+F` / `Ctrl+H`)
- **SCM panel direito** (`Ctrl+Shift+G`): arquivos sujos M/A/D/? · Enter abre · `d` diff · `r` refresh
- **Buffer picker** (`Ctrl+Shift+O`), **jump list** (`Ctrl+Alt+O/I`), **git blame** na status, **diff read-only** (`F2`)
- **Mouse completo**: documentado como **futuro** (pós S+A) em `docs/planning/ux-polish-plan.md`
- **Tier B** (surround, undo tree, macros…): futuro

### Added (P5)

- **Find in project** (`Ctrl+Shift+F`): crate `oride-search` com backend `rg` + fallback Rust (`ignore`)
- Lista de hits com Enter → abre arquivo e posiciona caret
- Opções `Alt+C` case · `Alt+R` regex no project find
- `ctrl+shift+d` = nova pasta na árvore (antes `ctrl+shift+f`, liberado para project find)

### Added (P6)

- **Language injections** em fences Markdown: ` ```oris `/`js`/`html`/`css` com highlight da grammar correspondente

### Added (P7)

- **Preview Markdown ANSI/TUI** (`Ctrl+Shift+V` / `Alt+P`): painel read-only ao lado do editor
- Scroll do preview: `Alt+↑/↓` / `Alt+PgUp/PgDn`

### Added (P8)

- Crate **`oride-plugin`**: `LanguageProvider`, `Plugin`, `PluginCtx`, `PluginHost`
- Providers built-in (oris/md/html/css/js) — comentário e soft wrap via host
- Plugins na palette: **word count**, **show file path**
- Hooks `OnOpen` / `OnSave` (lifecycle silencioso por default)
- Docs: `docs/plugin-api.md`

### Added (P9 splits + multi-cursor)

- **Split** vertical/horizontal: `Ctrl+Alt+V` / `Ctrl+Alt+H` (até 2 panes)
- Troca de pane: `F6` / `Ctrl+Alt+←→` · fechar pane: `Ctrl+Alt+W`
- **Multi-cursor**: `Ctrl+Alt+↑/↓` adiciona · digite em todos · `Ctrl+Alt+U` limpa
- Carets extras em amarelo; primário mantém estilo de cursor

## 0.1.0-alpha.5

### Added (P4 final)

- `.editorconfig` (indent_style / indent_size)
- Reload on disk change (`notify`) + `Ctrl+R` + prompt se dirty
- Terminal resize (`Alt+=` / `Alt+-`)
- Find **regex** (`Alt+R`)
- Config sections `[tree]` `[terminal]` `[lsp]` `[syntax]` + `format_on_save`
- Clipboard **OSC52** (SSH)
- CI workflow + `scripts/install.sh`
- `docs/plugin-api.md`

### Added (P3 LSP)

- Crate `oride-lsp` — cliente stdio JSON-RPC
- Diagnostics panel (`Ctrl+Shift+M`)
- Completion (`Ctrl+Space`) · Hover (`Ctrl+K`) · Goto (`F4`) · Format (`Ctrl+Shift+I`)
- Sync didOpen/didChange/didSave para buffers `.oris`

## 0.1.0-alpha.4

### Added (P4 polish)

- Lista completa de keybinds (`F1` / `Ctrl+G` / `Ctrl+Shift+/`) — filtro + scroll
- Find compacto no rodapé + replace/replace-all, case e acentos (`Ctrl+F`, `Ctrl+H`, `Alt+C`/`Alt+A`)
- Clipboard copy/paste/cut (`Ctrl+C/V/X`) via arboard + fallback interno
- Seleção multi-linha (Shift+setas/Home/End, Ctrl+Shift+Home/End, Ctrl+A) com highlight azul
- Save as (`Ctrl+Shift+S`) · Save all (`Ctrl+Alt+S`)
- Terminal toggle (`Ctrl+"` / `Ctrl+'`)
- **Browser de paths** para abrir pasta/arquivo (F2 / Ctrl+Enter / Ctrl+O confirma pasta)
- **Save as** via browser: digite o nome · **Enter** ou Ctrl+S salva · → entra pasta
- Highlight de linha selecionada nos modais (fundo ciano full-width)
- Aba ativa com fundo **branco** (chip no buffer); atalhos `Ctrl+PgUp/PgDn`, `Alt+←/→`
- Session leve: restaura workspace e abas; salva ao sair
- Docs: `docs/polish.md`; Markdown **futuro** em `docs/markdown.md`

### Fixed

- Seleção / copy-paste / select-all / save-as / confirmar pasta em terminais sem Ctrl+Enter
- Highlight visual da aba ativa e da seleção no editor

## 0.1.0-alpha.3

### Added

- **P2** — tree-sitter syntax highlight for `.oris`, Markdown, HTML, CSS, JavaScript
- Crate `oride-syntax` + vendored `tree-sitter-oriscript`
- Language id in status line; syntax colors on editor viewport
- **Markdown rico** — block+inline queries, headings/links/code/lists/quotes
- Derivados: `.mdx`, `.qmd`, `.rmd`, `.markdown`, README bare, etc.
- Soft wrap (`Alt+Z`, default on em MD)
- Toggle comment (`Ctrl+/`, HTML comments em MD)
- Continuação de listas Markdown no Enter
- Docs: `docs/markdown.md`

### Fixed (P2 polish)

- Cursor visível (cell invertida + `set_cursor_position` no terminal)
- Atalhos explícitos: `Ctrl+B` foco árvore, `Ctrl+E` foco editor (`Ctrl+Shift+B` oculta painel)
- Navegação da árvore: ↑↓/jk, ←→/hl, Enter, Space, Home/End
- `Ctrl+O` / palette “Open folder…” — abrir pasta de projeto no sistema
- Highlight de seleção da árvore (linha inteira ciano)

## 0.1.0-alpha.2

### Added

- **P1.1** — multi-tab bar, next/prev/close/new, dirty close confirm
- **P1.2** — project tree (`oride-fs`): expand, open file, create file/folder
- **P1.3** — Nerd Font icons + git status badges (`oride-git`)
- **P1.4** — embedded terminal panel (`oride-terminal`, PTY), toggle/focus
- **P1.5** — command palette + fuzzy open file
- Open directory as workspace (`oride .`)

## 0.1.0-alpha.1

### Added

- **P0.1** — workspace, `oride-core` (rope, undo, documents), headless CLI
- **P0.2** — minimal TUI (`oride-ui`, `oride-app`): open/edit/save/quit, status line
- **P0.3** — TOML config layers (defaults → `~/.config/oride` → `.oride/`), keymaps, UI theme colors
- Example config: `assets/config.example.toml`
- Docs: `docs/design.md`, `docs/config.md`
