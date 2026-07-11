# Oride

**Oride** (Ori + IDE) is a modular **terminal code editor** focused on
[OriScript](https://github.com/raillen/ori-script), with a navigable project
tree, collapsible embedded terminal, and first-class support for Markdown,
HTML, CSS, and JavaScript.

Status: **`0.1.0-alpha.1`** — Phase **P0.2**: minimal TUI (open / edit / save / quit).
Design: [`docs/design.md`](docs/design.md).

## Goals (0.1 “mini IDE”)

- Multi-tab editor with undo/redo
- Project tree (create files/folders) + icons (Nerd Fonts)
- Embedded terminal (toggle with keyboard)
- Syntax highlight: `.oris`, `.md`, `.html`, `.css`, `.js`
- Config / keymaps / themes via **TOML**
- OriScript intelligence via `oriscript lsp` (PATH)
- Command palette, fuzzy open, find/replace, git status in tree

## Build & run

```bash
cargo build --release
./target/release/oride                  # empty buffer
./target/release/oride path/to/file     # open file
./target/release/oride --version
./target/release/oride --demo
./target/release/oride README.md --stat
```

### Keys (P0.2)

| Key | Action |
|-----|--------|
| Type / Enter / Backspace / Delete | Edit |
| Arrows, Home, End, PgUp/PgDn | Move |
| `Ctrl+S` | Save (needs a path) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Esc` or `Ctrl+Q` | Quit (twice if dirty) |

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Workspace layout

```text
crates/
  oride-core/   # rope buffer, document, selection, undo (no UI)
  oride-ui/     # ratatui widgets (editor + status)
  oride-app/    # event loop + key dispatch
  oride/        # binary CLI
docs/
  design.md     # architecture & roadmap
```

## Relation to OriScript

Oride is a **separate repository**. It does not vendor the OriScript compiler.
Language intelligence uses the `oriscript` CLI / LSP on `PATH`.

## License

MIT — see [LICENSE](LICENSE).
