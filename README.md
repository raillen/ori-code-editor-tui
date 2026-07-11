# Oride

**Oride** (Ori + IDE) is a modular **terminal code editor** focused on
[OriScript](https://github.com/raillen/ori-script), with a navigable project
tree, collapsible embedded terminal, and first-class support for Markdown,
HTML, CSS, and JavaScript.

Status: **`0.1.0-alpha.4`** — P2 + Markdown + **polimento** (find, clipboard, help, session).  
Repo: [raillen/ori-code-editor-tui](https://github.com/raillen/ori-code-editor-tui).  
Docs: [design](docs/design.md) · [config](docs/config.md) · [markdown](docs/markdown.md) · [polish](docs/polish.md).

## Goals (0.1 “mini IDE”)

- Multi-tab editor with undo/redo
- Project tree (create files/folders) + icons (Nerd Fonts)
- Embedded terminal (toggle with keyboard)
- **Syntax highlight (tree-sitter):** `.oris`, Markdown (+ derivados/MDX), `.html`, `.css`, `.js`
- **Markdown:** soft wrap, comment `<!-- -->`, continuação de listas, highlight rico
- Config / keymaps / themes via **TOML**
- OriScript intelligence via `oriscript lsp` (PATH) — planned P3
- Command palette, fuzzy open, git status in tree

## Build & run

```bash
cargo build --release
./target/release/oride                  # CWD as workspace + empty buffer
./target/release/oride path/to/file     # open file
./target/release/oride path/to/dir      # open folder as workspace
./target/release/oride --version
./target/release/oride --demo
./target/release/oride README.md --stat
```

### Keys (defaults — rebind in TOML)

| Key | Action |
|-----|--------|
| Type / Enter / Backspace / Delete | Edit |
| Arrows, Home, End, PgUp/PgDn | Move |
| `Ctrl+S` | Save (needs a path) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+N` / `Ctrl+W` | New tab / close tab (2× if dirty) |
| `Ctrl+PgUp` / `Ctrl+PgDn` | Prev / next tab |
| `Ctrl+B` | **Focus tree** (show + navigate) |
| `Ctrl+E` | **Focus editor** |
| `Ctrl+Shift+B` | Hide/show tree panel |
| `Ctrl+O` | **Open project folder** (path prompt) |
| `Ctrl+\`` | Toggle embedded terminal |
| `Ctrl+P` | Fuzzy open file |
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+Shift+N` / `Ctrl+Shift+F` | New file / folder (tree) |
| `F5` | Refresh tree + git |
| `Alt+Z` | Toggle soft wrap (default on for Markdown) |
| `Ctrl+/` | Toggle line comment (`<!-- -->` in MD) |
| `Ctrl+F` / `F3` | Find / next match |
| `Ctrl+H` | Replace |
| `Ctrl+C` / `V` / `X` | Copy / paste / cut |
| `Ctrl+Shift+S` | Save all |
| `Ctrl+G` | Help (keybindings) |
| `Esc` or `Ctrl+Q` | Close overlay / unfocus / quit (2× if dirty) |

**Tree (focused):** `↑↓`/`jk` · `Enter` open/expand · `←→`/`hl` · `Space` toggle · `Tab`/`Esc` → editor.  
**Terminal:** type when focused · `Esc` → editor.  
**Icons:** Nerd Font glyphs (ASCII fallback exists in code).

### Config

```bash
mkdir -p ~/.config/oride
cp assets/config.example.toml ~/.config/oride/config.toml
# project overlay:
mkdir -p .oride && cp assets/config.example.toml .oride/config.toml
```

See [`docs/config.md`](docs/config.md).

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Workspace layout

```text
crates/
  oride-core/     # rope buffer, documents/tabs, undo
  oride-config/   # TOML load/merge
  oride-keymap/   # chords → actions
  oride-fs/       # project tree, create file/dir, icons
  oride-git/      # git status porcelain for tree badges
  oride-terminal/ # embedded PTY panel
  oride-syntax/   # tree-sitter highlight engine
  tree-sitter-oriscript/  # vendored OriScript grammar
  oride-ui/       # ratatui widgets
  oride-app/      # composition + event loop
  oride/          # binary CLI
docs/
  design.md       # architecture & roadmap
  config.md       # TOML reference
```

## Relation to OriScript

Oride is a **separate repository**. It does not vendor the OriScript compiler.
Language intelligence uses the `oriscript` CLI / LSP on `PATH`.

## License

MIT — see [LICENSE](LICENSE).
