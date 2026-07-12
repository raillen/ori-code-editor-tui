# Plano de polish UX / ND-acessibilidade (Oride)

**Status:** implementado (S+A) — mouse e tier B futuros  
**Princípio:** descoberta > memorização; um foco óbvio; no máx. 3 regiões “sempre on”; Esc sempre sai.

---

## Decisões do produto

| Item | Decisão |
|------|---------|
| **Mouse completo** | **Futuro** (após restante deste plano) — ver § Futuro |
| **Menu bar** (File/Edit/…) | **Urgente — implementar** |
| **Painel SCM/git à direita** | **Urgente — implementar** (status de arquivos, não 2ª project tree) |
| **Tier S + A** (Helix/LazyVim/Micro) | **Implementar** |
| **Tier B** | **Futuro** |
| **Terminal** | Corrigir para uso real (como shell do sistema) |

---

## Tier S (implementar)

| ID | Feature | Notas |
|----|---------|--------|
| **U0** | Terminal usável | PTY interativo; Ctrl+C/D/… no shell; foco óbvio; erros visíveis |
| **U1** | Context banner | Faixa “FOCUS: EDITOR \| TREE \| TERM \| SCM” de alto contraste |
| **U1b** | Status limpa | Linha estável (file · Ln/Col · git branch); mensagem efêmera separada |
| **U2** | Menu bar | File / Edit / View / Go / Git / Help + atalho à direita |
| **U2b** | Which-key | Após prefixo (ex. Space ou `Ctrl+G` hold flow): lista próximos binds |
| **U2c** | Palette + F1 óbvios | Menu Help + banner first-run / hint permanente |
| **U4** | Find/replace mini-modal | Modal compacto centrado (não competir com status/terminal) |

## Tier A (implementar)

| ID | Feature | Notas |
|----|---------|--------|
| **U5** | Welcome / 5 atalhos | Overlay 1ª vez ou Help “Essential” |
| **U6** | SCM panel direito | Lista sujos M/A/D/?; Enter open; retrátil `Ctrl+Shift+G` |
| **U7a** | Buffer picker | Lista tabs abertas (fuzzy) |
| **U7b** | Jump list | Last locations (Ctrl+O / Ctrl+I ou menu Go) |
| **U7c** | Git blame na status | 1 linha `git blame -L` no arquivo ativo |
| **U7d** | Search count / hits | Find local e project com N/M legível (já parcial) |
| **U7e** | Diff read-only | Modal/painel `git diff -- path` a partir do SCM |

## Tier B

| Item | Status |
|------|--------|
| Surround / change around | **MVP** `F8` |
| Undo history panel | **MVP** `Ctrl+Shift+U` (não árvore ramificada) |
| Multi-source picker | **MVP** `Ctrl+Shift+T` (buf+cmd+file) |
| Macros | **MVP** `F9`/`F10` |
| Full vim modal default | **futuro** |
| Inlay hints densos | **futuro** (LSP) |
| Session workspace avançada | **futuro** |
| Telescope monstro | **futuro** |

## Mouse

| Item | Status |
|------|--------|
| EnableMouseCapture | **feito** |
| Clique = caret; drag = seleção | **feito** |
| Duplo/triplo clique | **feito** (palavra/linha) |
| Clique árvore/abas/terminal/SCM/menu | **feito** |
| Scroll wheel por painel | **feito** |
| Config `mouse = true/false` | **feito** |

---

## Ordem de implementação (DAG)

```text
U0 terminal ─────────────────────────────┐
U1 banner + status ──────────────────────┼─► U2 menu bar ─► U2b which-key
                                         │         │
                                         │         ▼
                                         │    U4 find mini-modal
                                         │         │
                                         ▼         ▼
                                    U5 welcome (pode ir com menu Help)
                                         │
                                         ▼
                                    U6 SCM panel direito
                                         │
                    ┌────────────────────┼────────────────────┐
                    ▼                    ▼                    ▼
                 U7a buffer           U7b jump            U7c blame
                 picker               list
                                         │
                                         ▼
                                    U7e diff read-only
                                    U7d search polish
```

**Um dev:** U0 → U1 → U2 → U2b/U2c → U4 → U5 → U6 → U7*.

---

## Fatias técnicas

### U0 Terminal

- Spawn shell **interativo** (`-i` / login quando fizer sentido)  
- Com foco Terminal: encaminhar **Ctrl+A–Z**, Enter, setas, Backspace, Tab, Delete  
- Esc (sem ctrl) → Editor  
- Toggle: re-spawn se morto; **sempre** focar; borda ciano + título “TERMINAL · digite · Esc=editor”  
- Superfície de erro de write/PTY na status  
- Resize PTY em todo draw do painel  

### U1 Banner + status

- `render_context_banner`: 1 linha, bg Cyan/Black bold  
- Status: `title ●  Ln n, Col m  git:branch  lsp?`  
- Message: só se `status_message` set; senão hint curto `F1 help · Ctrl+Shift+P cmds`  

### U2 Menu bar

- Linha 0 do frame: ` File  Edit  View  Go  Git  Help `  
- Alt+F / clique futuro: abre dropdown  
- Teclado: Left/Right menu, Down abre, Enter executa Action  
- Itens mapeiam para `Action` / plugin commands existentes  

### U2b Which-key

- Overlay ao pressionar `Space` (editor, sem typing?) **ou** chord `ctrl+g` mostra grupos  
- MVP: `Action::WhichKey` → lista prefixos File/Edit/… com binds filtrados  

### U4 Find modal

- `render_mini_modal` 40% width, height 6–8, centrado  
- Find + optional Replace + flags `[c]ase [r]e [a]ccent`  
- Project find reutiliza chrome  

### U6 SCM

- `show_scm: bool`, largura config  
- Lista de `status_map` sorted  
- Enter → open file; `r` refresh  
- Não duplicar project tree  

### U7*

- Buffer picker: overlay com tab titles  
- Jump list: `Vec<Jump>` em app (path, byte offset), push on jump/goto  
- Blame: `git blame -L n,n --porcelain`  
- Diff: `git diff -- path` em overlay scrollable  

---

## Gates

- `cargo fmt` · `clippy -D warnings` · `cargo test --workspace`  
- Atalhos no default keymap + **menu** + F1  
- CHANGELOG  
- ND: texto de contexto em português/inglês curto, alto contraste  

## Skills

`clean-code`, `rust`, `living-docs`; UI TUI legível.

---

## Aceite deste plano

- [x] Terminal aceita `ls`, `Ctrl+C`, typing normal com foco  
- [x] Menu bar acessa Save, Palette, Help, Git panel, Terminal  
- [x] Banner mostra foco atual  
- [x] SCM à direita lista arquivos sujos  
- [x] Buffer picker + jump list + blame na status  
- [x] Mouse **não** bloqueia; está documentado como futuro  

## Aceite — mouse

- [x] Clique editor = caret  
- [x] Drag = seleção  
- [x] Clique painéis = foco  
- [x] Scroll wheel  
- [x] Docs + `mouse = true` (default)  
