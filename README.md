# Oride

**Oride** (Ori + IDE) is a modular **terminal code editor** focused on
[OriScript](https://github.com/raillen/ori-script), with a navigable project
tree, collapsible embedded terminal, and first-class support for Markdown,
HTML, CSS, and JavaScript.

Status: **`0.1.0-alpha.6`** — mini-IDE TUI contida (editor, tree, terminal, git/SCM, find, LSP OriScript, MD preview, splits, mouse opt-in).  
Repo: [raillen/ori-code-editor-tui](https://github.com/raillen/ori-code-editor-tui).  
Docs: [design](docs/design.md) · [config](docs/config.md) · [markdown](docs/markdown.md) · **[roadmap alpha.6+](docs/planning/alpha6-roadmap.md)**.

## Goals (produto contido)

- Tudo no TUI — **sem** preview HTML/browser, **sem** macros (remoção planejada), **sem** host de plugins externos
- Multi-tab, tree, terminal PTY, find (buffer + project), git status/SCM, session leve
- **First-class languages (alvo):** OriScript, Ori-lang, Markdown, HTML, CSS, JS/TS, Rust, Python, Nim, Ruby  
  (hoje estáveis: OriScript + MD/HTML/CSS/JS; demais no [roadmap](docs/planning/alpha6-roadmap.md) L1)
- Config TOML · keymaps · **OriScript LSP** (`oriscript lsp` no `PATH`)
- MD preview **no terminal** (texto + placeholders; imagens via protocolo do terminal = planejado)
- Links no preview → abrir no **navegador do sistema** (planejado M1)
- Mouse **opt-in** (`mouse = false` default)

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
| `Ctrl+Shift+V` / `Alt+P` | **Markdown preview** (segue scroll do editor · `Alt+↑/↓` fine) |
| `Ctrl+Alt+V` / `H` | Split editor vertical / horizontal |
| `F6` / `Ctrl+Alt+W` | Next pane / close pane |
| `Ctrl+Alt+↑/↓` / `U` | Multi-cursor add / clear |
| Find `Alt+R` | Toggle regex search (buffer e projeto) |
| `Ctrl+F` / `F3` | Find mini-modal / next |
| `Ctrl+H` | Replace (mesmo modal; Tab troca campo) |
| `Alt+C` | Toggle **case sensitive** (no find) |
| `Alt+A` | Toggle **ignorar acentos** (á≈a; no find) |
| `Alt+W` | Toggle **palavra completa** (UI ≠ GUI) |
| `Alt+R` | Toggle regex (buffer e projeto) |
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
**Mouse (default off):** ativar com `mouse = true` no TOML ou **View → Enable / disable mouse** (palette). Com on: clique = caret · drag = seleção · duplo = palavra · scroll por painel.

### Extras úteis

| Key | Action |
|-----|--------|
| `F8` | Surround seleção com par `()[]{}…` |
| `Ctrl+Shift+T` | Multi-picker (buffers + cmds + files) |
| `Ctrl+Shift+U` | Histórico de undo |
| View → Enable mouse | Liga captura de mouse (ou `mouse = true`) |

Macros (`F9`/`F10`) estão **deprecated** e serão removidas (anti-bloat).

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
