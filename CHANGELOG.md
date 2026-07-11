# Changelog

## 0.1.0-alpha.3

### Added

- **P2** — tree-sitter syntax highlight for `.oris`, Markdown, HTML, CSS, JavaScript
- Crate `oride-syntax` + vendored `tree-sitter-oriscript`
- Language id in status line; syntax colors on editor viewport

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
