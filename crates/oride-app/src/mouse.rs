//! Hit-test e helpers de mouse (sem event loop).

use oride_core::{Buffer, ByteOffset, Caret};
use ratatui::layout::Rect;

/// Retângulos do último frame (coordenadas de tela).
#[derive(Debug, Clone, Default)]
pub struct HitRegions {
    pub menu: Rect,
    pub tree: Option<Rect>,
    pub tabs: Option<Rect>,
    pub editor: Option<Rect>,
    pub scm: Option<Rect>,
    pub terminal: Option<Rect>,
    pub gutter: u16,
    pub text_width: u16,
    pub soft_wrap: bool,
    pub scroll_y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    Menu,
    Tree,
    Tabs,
    Editor,
    Scm,
    Terminal,
    Outside,
}

impl HitRegions {
    #[must_use]
    pub fn at(&self, x: u16, y: u16) -> HitTarget {
        if contains(self.menu, x, y) {
            return HitTarget::Menu;
        }
        if let Some(r) = self.tree {
            if contains(r, x, y) {
                return HitTarget::Tree;
            }
        }
        if let Some(r) = self.tabs {
            if contains(r, x, y) {
                return HitTarget::Tabs;
            }
        }
        if let Some(r) = self.editor {
            if contains(r, x, y) {
                return HitTarget::Editor;
            }
        }
        if let Some(r) = self.scm {
            if contains(r, x, y) {
                return HitTarget::Scm;
            }
        }
        if let Some(r) = self.terminal {
            if contains(r, x, y) {
                return HitTarget::Terminal;
            }
        }
        HitTarget::Outside
    }
}

fn contains(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x.saturating_add(r.width) && y >= r.y && y < r.y.saturating_add(r.height)
}

/// Converte posição no painel de texto → caret (linha/coluna lógicas).
#[must_use]
pub fn screen_to_caret(
    buffer: &Buffer,
    hits: &HitRegions,
    screen_x: u16,
    screen_y: u16,
) -> Option<Caret> {
    let ed = hits.editor?;
    // permite 1 célula fora da borda (arrasto saindo do painel)
    let x = screen_x.clamp(ed.x, ed.x.saturating_add(ed.width.saturating_sub(1)));
    let y = screen_y.clamp(ed.y, ed.y.saturating_add(ed.height.saturating_sub(1)));

    let local_x = x.saturating_sub(ed.x);
    let local_y = y.saturating_sub(ed.y) as usize;
    let col_in_text = local_x.saturating_sub(hits.gutter) as usize;
    let text_width = hits.text_width.max(1) as usize;
    let line_count = buffer.line_count().max(1);

    if !hits.soft_wrap {
        let line = (hits.scroll_y + local_y).min(line_count.saturating_sub(1));
        let content = buffer.line_text(line).unwrap_or_default();
        let col = col_in_text.min(content.chars().count());
        return Some(Caret::new(line, col));
    }

    let mut visual = 0usize;
    let mut logical = hits.scroll_y;
    while logical < line_count {
        let content = buffer.line_text(logical).unwrap_or_default();
        let rows = line_visual_rows(&content, text_width);
        if visual + rows > local_y {
            let row_in_line = local_y - visual;
            let col = (row_in_line * text_width + col_in_text).min(content.chars().count());
            return Some(Caret::new(logical, col));
        }
        visual += rows;
        logical += 1;
    }
    let line = line_count.saturating_sub(1);
    let content = buffer.line_text(line).unwrap_or_default();
    Some(Caret::new(line, content.chars().count()))
}

fn line_visual_rows(content: &str, width: usize) -> usize {
    let w = width.max(1);
    let n = content.chars().count();
    if n == 0 {
        1
    } else {
        n.div_ceil(w)
    }
}

/// Índice de linha flat na árvore a partir do y do mouse.
#[must_use]
pub fn tree_row_at(tree_area: Rect, tree_scroll: usize, y: u16) -> Option<usize> {
    if y < tree_area.y.saturating_add(1) {
        return None;
    }
    let inner_y = y.saturating_sub(tree_area.y.saturating_add(1)) as usize;
    Some(tree_scroll + inner_y)
}

/// Índice de item SCM.
#[must_use]
pub fn list_row_at(area: Rect, scroll: usize, y: u16) -> Option<usize> {
    if y < area.y.saturating_add(1) {
        return None;
    }
    let inner_y = y.saturating_sub(area.y.saturating_add(1)) as usize;
    Some(scroll + inner_y)
}

/// Índice de menu (File/Edit/…) pela coordenada x.
#[must_use]
pub fn menu_index_at(titles: &[&str], menu_area: Rect, x: u16) -> Option<usize> {
    if x < menu_area.x || x >= menu_area.x.saturating_add(menu_area.width) {
        return None;
    }
    let mut cx = menu_area.x + 1;
    for (i, t) in titles.iter().enumerate() {
        let w = (t.chars().count() as u16) + 2;
        if x >= cx && x < cx + w {
            return Some(i);
        }
        cx = cx.saturating_add(w);
    }
    None
}

/// Índice de tab aproximado (largura igual).
#[must_use]
pub fn tab_index_at(tabs_area: Rect, tab_count: usize, x: u16) -> Option<usize> {
    if tab_count == 0 || !contains(tabs_area, x, tabs_area.y) {
        return None;
    }
    let w = (tabs_area.width as usize / tab_count).max(1);
    let local = x.saturating_sub(tabs_area.x) as usize;
    Some((local / w).min(tab_count - 1))
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Expande seleção para palavra sob o offset (ou imediatamente à esquerda).
///
/// Clicar no **fim** da palavra (caret após o último char) ainda seleciona a palavra.
#[must_use]
pub fn word_bounds(buffer: &Buffer, at: ByteOffset) -> (ByteOffset, ByteOffset) {
    let text = buffer.as_string();
    if text.is_empty() {
        return (ByteOffset::new(0), ByteOffset::new(0));
    }

    let mut i = at.as_usize().min(text.len());
    // garante boundary
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }

    // Se estamos no fim ou em não-palavra, recua 1 char
    let on_word = text[i..].chars().next().is_some_and(is_word_char);
    if !on_word {
        if i == 0 {
            return (ByteOffset::new(i), ByteOffset::new(i));
        }
        let mut p = i - 1;
        while p > 0 && !text.is_char_boundary(p) {
            p -= 1;
        }
        if text[p..].chars().next().is_some_and(is_word_char) {
            i = p;
        } else {
            return (ByteOffset::new(i), ByteOffset::new(i));
        }
    }

    // expand start
    let mut start = i;
    while start > 0 {
        let mut p = start - 1;
        while p > 0 && !text.is_char_boundary(p) {
            p -= 1;
        }
        if !text[p..].chars().next().is_some_and(is_word_char) {
            break;
        }
        start = p;
    }

    // expand end
    let mut end = i;
    for c in text[i..].chars() {
        if !is_word_char(c) {
            break;
        }
        end += c.len_utf8();
    }

    (ByteOffset::new(start), ByteOffset::new(end))
}

/// Clique multiplo: mesma célula (±1) em ≤450ms.
#[must_use]
pub fn is_multi_click(
    last: Option<(std::time::Instant, u16, u16)>,
    x: u16,
    y: u16,
    max_ms: u128,
) -> bool {
    let Some((t, lx, ly)) = last else {
        return false;
    };
    if t.elapsed().as_millis() > max_ms {
        return false;
    }
    let dx = (lx as i32 - x as i32).unsigned_abs();
    let dy = (ly as i32 - y as i32).unsigned_abs();
    dx <= 1 && dy <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_editor_and_tree() {
        let h = HitRegions {
            menu: Rect::new(0, 0, 80, 1),
            tree: Some(Rect::new(0, 2, 20, 20)),
            editor: Some(Rect::new(20, 3, 60, 18)),
            ..Default::default()
        };
        assert_eq!(h.at(5, 5), HitTarget::Tree);
        assert_eq!(h.at(30, 10), HitTarget::Editor);
        assert_eq!(h.at(0, 0), HitTarget::Menu);
    }

    #[test]
    fn screen_to_caret_simple() {
        let buf = Buffer::from_text("abc\ndef\n");
        let h = HitRegions {
            editor: Some(Rect::new(0, 0, 40, 10)),
            gutter: 0,
            text_width: 40,
            scroll_y: 0,
            soft_wrap: false,
            ..Default::default()
        };
        let c = screen_to_caret(&buf, &h, 2, 1).unwrap();
        assert_eq!(c.line, 1);
        assert_eq!(c.column, 2);
    }

    #[test]
    fn word_bounds_ident() {
        let buf = Buffer::from_text("foo bar_baz");
        let (s, e) = word_bounds(&buf, ByteOffset::new(5)); // 'b' of bar_baz
        assert_eq!(&buf.as_string()[s.as_usize()..e.as_usize()], "bar_baz");
    }

    #[test]
    fn word_bounds_at_end_of_word() {
        // offset após "foo" (índice 3 = espaço) → ainda pega "foo"
        let buf = Buffer::from_text("foo bar");
        let (s, e) = word_bounds(&buf, ByteOffset::new(3));
        assert_eq!(&buf.as_string()[s.as_usize()..e.as_usize()], "foo");
    }

    #[test]
    fn word_bounds_unicode() {
        let buf = Buffer::from_text("olá mundo");
        // 'l' de olá
        let (s, e) = word_bounds(&buf, ByteOffset::new(2));
        assert_eq!(&buf.as_string()[s.as_usize()..e.as_usize()], "olá");
    }

    #[test]
    fn multi_click_tolerance() {
        use std::time::Instant;
        let t = Instant::now();
        assert!(is_multi_click(Some((t, 10, 5)), 11, 5, 450));
        assert!(!is_multi_click(Some((t, 10, 5)), 15, 5, 450));
    }
}
