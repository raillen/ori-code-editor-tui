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
    if !contains(ed, screen_x, screen_y) {
        return None;
    }
    let local_x = screen_x.saturating_sub(ed.x);
    let local_y = screen_y.saturating_sub(ed.y) as usize;
    let col_in_text = local_x.saturating_sub(hits.gutter) as usize;
    let text_width = hits.text_width.max(1) as usize;
    let line_count = buffer.line_count().max(1);

    if !hits.soft_wrap {
        let line = (hits.scroll_y + local_y).min(line_count.saturating_sub(1));
        let content = buffer.line_text(line).unwrap_or_default();
        let col = col_in_text.min(content.chars().count());
        return Some(Caret::new(line, col));
    }

    // soft wrap: caminha linhas lógicas contando rows visuais
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
        if visual > local_y + 1000 {
            break;
        }
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
        // borda/título
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

/// Expande seleção para palavra sob o caret.
#[must_use]
pub fn word_bounds(buffer: &Buffer, at: ByteOffset) -> (ByteOffset, ByteOffset) {
    let text = buffer.as_string();
    let bytes = text.as_bytes();
    let mut i = at.as_usize().min(bytes.len());
    if i > 0 && i == bytes.len() {
        i -= 1;
        while i > 0 && !text.is_char_boundary(i) {
            i -= 1;
        }
    }
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    // se não for word char, só caret
    if i >= bytes.len() || !is_word(bytes[i]) {
        return (at, at);
    }
    let mut start = i;
    while start > 0 {
        let prev = start - 1;
        let mut p = prev;
        while p > 0 && !text.is_char_boundary(p) {
            p -= 1;
        }
        if !is_word(bytes[p]) {
            break;
        }
        start = p;
    }
    let mut end = i;
    while end < bytes.len() {
        if !text.is_char_boundary(end) {
            end += 1;
            continue;
        }
        if !is_word(bytes[end]) {
            break;
        }
        end += 1;
    }
    (ByteOffset::new(start), ByteOffset::new(end))
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
        let (s, e) = word_bounds(&buf, ByteOffset::new(5)); // inside bar_baz
        assert_eq!(&buf.as_string()[s.as_usize()..e.as_usize()], "bar_baz");
    }
}
