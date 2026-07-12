# Oride

**Oride** (Ori + IDE) is a modular **terminal code editor** focused on
[OriScript](https://github.com/raillen/ori-script), with a navigable project
tree, collapsible embedded terminal, and first-class support for Markdown,
HTML, CSS, and JavaScript.

Status: **`0.1.0-alpha.5`** — P2 + **P4 polish** + **P3 LSP** (diagnostics/complete/hover/goto/format).  
Repo: [raillen/ori-code-editor-tui](https://github.com/raillen/ori-code-editor-tui).  
Docs: [design](docs/design.md) · [config](docs/config.md) · [markdown](docs/markdown.md) · [polish](docs/polish.md).

## Goals (0.1 “mini IDE”)

- Multi-tab editor with undo/redo
- Project tree (create files/folders) + icons (Nerd Fonts)
- Embedded terminal (toggle + resize)
- **Syntax highlight (tree-sitter):** `.oris`, Markdown, HTML, CSS, JS
- Config / keymaps / themes / `[lsp]` via **TOML**
- **OriScript LSP** via `oriscript lsp` on `PATH`
- Command palette, fuzzy open, git status, find/replace (regex), session
- Built-in **plugin surface** (`oride-plugin`; e.g. palette “Plugin: word count”)

## Build & run

```bash
cargo build --release
./scripts/install.sh                    # → ~/.local/bin/oride
./target/release/oride                  # CWD as workspace + empty buffer
./target/release/oride path/to/file     # open file
./target/release/oride path/to/dir      # open folder as workspace
./target/release/oride --version
```

### Keys (defaults — rebind in TOML)

| Key | Action |
|-----|--------|
| Type / Enter / Backspace / Delete | Edit |
| Arrows, Home, End, PgUp/PgDn | Move |
| `Shift`+arrows/Home/End | Extend selection (multi-line) |
| `Ctrl+Shift+Home` / `End` | Select to doc start/end |
| `Ctrl+A` | Select all |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` / `F12` / `Alt+Shift+S` | **Save as…** (path browser · Enter salva) |
| `Ctrl+Alt+S` | **Save all** |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+N` / `Ctrl+W` | New tab / close tab (2× if dirty) |
| `Ctrl+PgUp` / `Ctrl+PgDn` | Previous / next tab |
| `Alt+←` / `Alt+→` | Previous / next tab |
| (tab bar) | Aba ativa = chip **branco** (fundo sólido) |
| `Ctrl+B` / `Ctrl+E` | Focus tree / editor |
| `Ctrl+O` | **Open folder** (`F2` / `Ctrl+Enter` / `Ctrl+O` confirma) |
| `Ctrl+P` | **Open file** (navigate dirs/files) |
| `Ctrl+"` / `Ctrl+'` / `Ctrl+\`` | Toggle **terminal** (interativo; digite com foco · Esc=editor) |
| `Ctrl+Shift+G` | **SCM panel** (direita · Enter abre · `d` diff) |
| `Ctrl+Shift+O` | **Buffer picker** (tabs abertas) |
| `Ctrl+Alt+O` / `I` | Jump back / forward |
| `Alt+F/E/V/G/I/H` | **Menu bar** File/Edit/View/Go/Git/Help |
| `Alt+/` | **Which-key** (atalhos essenciais) |
| `F1` / `Ctrl+G` / `Ctrl+Shift+/` | **List all keybindings** (filter · ↑↓ · Esc) |
| `F2` | **Git diff** do arquivo ativo |
| `Ctrl+Space` / `Ctrl+K` / `F4` | LSP complete / hover / goto |
| `Ctrl+Shift+I` / `Ctrl+Shift+M` | LSP format / diagnostics panel |
| `Alt+=` / `Alt+-` | Terminal taller / shorter |
| `Ctrl+R` | Reload file from disk |
| `Ctrl+Shift+F` | **Find in project** (rg ou fallback Rust) |
| `Ctrl+Shift+V` / `Alt+P` | **Markdown preview** (painel TUI) |
| `Ctrl+Alt+V` / `H` | Split editor vertical / horizontal |
| `F6` / `Ctrl+Alt+W` | Next pane / close pane |
| `Ctrl+Alt+↑/↓` / `U` | Multi-cursor add / clear |
| Find `Alt+R` | Toggle regex search (buffer e projeto) |
| `Ctrl+F` / `F3` | Find bar (footer) / next |
| `Ctrl+H` | Replace (same bar) |
| `Alt+C` / `Alt+A` | Toggle case / ignore accents (in find) |
| `Alt+Enter` / `Ctrl+Alt+Enter` | Replace one / replace all |
| `Ctrl+C` / `V` / `X` | Copy / paste / cut |
| `Alt+Z` | Soft wrap |
| `Ctrl+/` | Toggle comment |
| `Esc` or `Ctrl+Q` | Close overlay / quit |

**Browser (`Ctrl+O` / `Ctrl+P` / Save as):** linha ciano = seleção · `↑↓` · `Enter` entra/abre (save as: **Enter salva**) · `F2`/`Ctrl+O` confirma pasta · digite filtra/nome.

**Tree (focused):** `↑↓`/`jk` · `Enter` open/expand · `←→`/`hl` · `Space` toggle · `Tab`/`Esc` → editor.  
**Terminal:** shell interativo com foco no painel · `Ctrl+C` vai pro shell · `Esc` → editor · `Ctrl+"` fecha.  
**SCM:** lista working tree dirty (não é 2ª project tree).  
**Icons:** Nerd Font glyphs (ASCII fallback exists in code).  
**Mouse:** planejado (futuro) — ver `docs/planning/ux-polish-plan.md`.

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
