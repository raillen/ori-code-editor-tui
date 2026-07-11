# Configuração (P0.3)

Oride carrega TOML em camadas (depois sobrescreve o anterior):

1. **Defaults embutidos**
2. **Usuário:** `~/.config/oride/config.toml` (XDG)
3. **Projeto:** primeiro `.oride/config.toml` encontrado subindo a partir do arquivo aberto (ou do CWD)

Exemplo completo: [`assets/config.example.toml`](../assets/config.example.toml).

## Campos

| Campo | Tipo | Default | Efeito |
|-------|------|---------|--------|
| `theme` | string | `"default"` | Nome lógico (cores em `[ui]`) |
| `show_line_numbers` | bool | `true` | Gutter |
| `[editor].tab_size` | u8 | `4` | Largura do Tab com espaços |
| `[editor].insert_spaces` | bool | `true` | Tab → espaços |
| `[ui].*` | cor | ver defaults | Tema TUI |
| `[keys]` | map | bindings P0.2 | Rebind de ações |

### Cores

- Nomes: `reset`, `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `darkgray`, `white`, `lightred`, …
- Hex: `#RGB` ou `#RRGGBB`

### Actions (`[keys]`)

| Id | Comportamento |
|----|----------------|
| `quit` | Sair (2× se dirty) |
| `save` | Salvar path atual |
| `undo` / `redo` | Pilha de edits |
| `insert_newline` / `insert_tab` / `backspace` / `delete` | Edição |
| `move_left` … `move_line_end` | Movimento |
| `move_*_extend` | Movimento com seleção |
| `page_up` / `page_down` | Página |
| `toggle_tree` / `toggle_terminal` | Painéis |
| `focus_tree` / `focus_editor` / `focus_terminal` | Foco |
| `next_tab` / `prev_tab` / `close_tab` / `new_tab` | Tabs |
| `command_palette` / `open_file_fuzzy` | Palette |
| `tree_new_file` / `tree_new_dir` / `tree_refresh` | Árvore |

Chords: `ctrl+s`, `shift+left`, `esc`, `pageup`, `ctrl+shift+p`, … (minúsculas, `+` como separador).

Defaults: `ctrl+b` foco árvore, `ctrl+e` foco editor, `ctrl+o` abrir pasta,
`ctrl+shift+b` mostrar/ocultar árvore, `ctrl+\`` terminal, `ctrl+p` arquivo,
`ctrl+shift+p` comandos.

## Validação

```bash
# Rebind em teste de unidade: crates/oride-app
cargo test -p oride-app rebind_ctrl_s
cargo test -p oride-config
cargo test -p oride-keymap
```
