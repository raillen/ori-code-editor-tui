# Polimento 0.1 (P4) + LSP (P3)

## Feito (alpha.5)

### P4

| Feature | Notas |
|---------|--------|
| Lista de keybinds | `F1` / `Ctrl+G` / `Ctrl+Shift+/` |
| Find compacto | case, acentos, **regex (`Alt+R`)**, replace all |
| Seleção multi-linha | Shift+…, Ctrl+A, highlight azul |
| Clipboard | arboard + **OSC52** + buffer interno |
| Save as / save all | browser path |
| Terminal | toggle + **Alt+= / Alt+-** altura |
| `.editorconfig` | indent_style / indent_size ao abrir |
| Reload disco | `notify` + prompt se dirty · `Ctrl+R` |
| Config | `[editor]` `[tree]` `[terminal]` `[lsp]` `[syntax]` |
| CI + install | `.github/workflows/ci.yml` · `scripts/install.sh` |
| Session | workspace + tabs |

### P3 (LSP OriScript)

| Feature | Atalho |
|---------|--------|
| Spawn `oriscript lsp` | `[lsp]` config |
| Diagnostics panel | `Ctrl+Shift+M` |
| Completion | `Ctrl+Space` |
| Hover | `Ctrl+K` |
| Go to definition | `F4` |
| Format document | `Ctrl+Shift+I` (+ `format_on_save`) |

## Pós-0.1

- Preview MD, fence injections, MDX JSX
- Search in project, splits, multi-cursor
- Plugins externos
