# Changelog

## 0.1.0-alpha.4

### Added (P4 polish)

- Help overlay (`Ctrl+H`)
- Find / find next-prev / replace (`Ctrl+F`, `F3`, `Ctrl+Shift+H`)
- Clipboard copy/paste/cut (`Ctrl+C/V/X`) via arboard + fallback interno
- Save as (`Ctrl+Shift+S`) · Save all (`Ctrl+Alt+S`)
- Terminal toggle (`Ctrl+"` / `Ctrl+'`)
- **Browser de paths** para abrir pasta/arquivo (navegar dirs, filtrar, `Ctrl+Enter` confirma pasta)
- Session leve: restaura workspace e abas; salva ao sair
- Docs: `docs/polish.md`; Markdown **futuro** em `docs/markdown.md`

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
