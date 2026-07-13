# Changelog

## Unreleased

- (próximo: languages first-class, MD links, images via terminal protocol — ver `docs/planning/alpha6-roadmap.md`)
- Remoção planejada de **macros** (anti-bloat)

## 0.1.0-alpha.6

Baseline mini-IDE TUI contida. Plano normativo: [`docs/planning/alpha6-roadmap.md`](docs/planning/alpha6-roadmap.md).

### Added (mouse)

- **Mouse completo** (**default off** — `mouse = true` no TOML ou **View → Enable mouse** / palette)
- Clique → caret · drag → seleção · duplo → palavra · triplo → linha
- Clique árvore/abas/SCM/terminal/menu · scroll por painel · botão direito → which-key
- Capture só quando ligado (não rouba scroll do emulador no default)

### Added (navigation / git UX)

- **Surround** (`F8`) · **Multi-picker** (`Ctrl+Shift+T`) · **Undo history** (`Ctrl+Shift+U`)
- **SCM panel** (`Ctrl+Shift+G`) · buffer picker · jump list · blame · diff read-only (`F2`)
- Menu bar, context banner, which-key, welcome, find mini-modal

### Added (find)

- Palavra completa (`Alt+W`) · modal Find/Replace legível · project find (`Ctrl+Shift+F`, rg+fallback)

### Added (markdown)

- Preview TUI + placeholders de imagem · task lists/tabelas/frontmatter/strike · fence inject oris/js/html/css
- Preview segue scroll do editor

### Added (P5–P9 stack)

- Plugins built-in · splits · multi-cursor · terminal PTY interativo · LSP OriScript (alpha.5+)

### Fixed

- Mouse drag lag (drain/coalesce events) · double-click word · editor scroll follows caret

### Docs

- Plano **alpha.6+** contido (sem macros, sem preview HTML/browser)

### Note

- Macros (`F9`/`F10`) presentes nesta tag mas **marcadas para remoção** (R1 no roadmap); não expandir

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
