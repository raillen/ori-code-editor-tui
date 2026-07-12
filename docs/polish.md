# Polimento 0.1 (P4)

Fatias de DX para candidatar release **0.1.0** (sem LSP ainda).

## Feito (alpha.4)

| Feature | Atalho / notas |
|---------|----------------|
| Lista de keybinds | `F1` / `Ctrl+G` / `Ctrl+Shift+/` — todos os binds do mapa, com filtro |
| Find compacto | `Ctrl+F` barra no rodapé · `F3` next · case/acentos |
| Replace / replace all | `Ctrl+H` · `Alt+Enter` 1× · `Ctrl+Alt+Enter` all |
| Seleção multi-linha | Shift+setas/Home/End · Ctrl+A · highlight azul |
| Copy / Paste / Cut | `Ctrl+C` / `V` / `X` (+ buffer interno) |
| Save as | `Ctrl+Shift+S` — browser · **Enter** salva · `→` entra pasta |
| Save all | `Ctrl+Alt+S` |
| Terminal | `Ctrl+"` (ou `Ctrl+'`) |
| Open folder / file | browser (`Ctrl+O` / `Ctrl+P`) · confirma com **F2**/Ctrl+Enter/Ctrl+O |
| Highlight de modal | linha selecionada ciano |
| Aba ativa | chip branco (bg por célula); `Ctrl+PgUp/PgDn` · `Alt+←/→` |
| Session leve | restaura workspace+tabs; salva ao sair |
| Markdown futuro | `docs/markdown.md` § Futuro |

## Ainda desejável no P4 / pré-0.1.0

- `.editorconfig`
- Reload se arquivo mudou no disco (`notify`)
- Resize altura do terminal (não só toggle)
- Cores de syntax no TOML
- Seções `[tree]` / `[terminal]` / `[lsp]` na config
- CI + `scripts/install.sh`
- Find case-sensitive / regex

## Explicitamente depois

- LSP OriScript (P3)
- Preview MD, fence injections, MDX JSX (`docs/markdown.md`)
