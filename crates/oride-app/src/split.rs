//! Estado de split do editor (até 2 panes no MVP).

use oride_core::DocumentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// Painéis lado a lado.
    Vertical,
    /// Painéis um em cima do outro.
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorPane {
    pub doc_id: DocumentId,
    pub scroll_y: usize,
}

/// Layout de splits: 1 pane (single) ou 2 panes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitState {
    pub panes: Vec<EditorPane>,
    pub focused: usize,
    pub orientation: SplitOrientation,
}

impl SplitState {
    #[must_use]
    pub fn single(doc_id: DocumentId) -> Self {
        Self {
            panes: vec![EditorPane {
                doc_id,
                scroll_y: 0,
            }],
            focused: 0,
            orientation: SplitOrientation::Vertical,
        }
    }

    #[must_use]
    pub fn focused_pane(&self) -> &EditorPane {
        &self.panes[self.focused.min(self.panes.len().saturating_sub(1))]
    }

    pub fn focused_pane_mut(&mut self) -> &mut EditorPane {
        let i = self.focused.min(self.panes.len().saturating_sub(1));
        &mut self.panes[i]
    }

    #[must_use]
    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// Abre segundo pane com o mesmo documento (ou `doc_id` dado).
    pub fn split(&mut self, orientation: SplitOrientation, doc_id: DocumentId) {
        self.orientation = orientation;
        if self.panes.len() >= 2 {
            // já split: só reorienta e atualiza o secundário
            if let Some(p) = self.panes.get_mut(1) {
                p.doc_id = doc_id;
            }
            return;
        }
        let scroll = self.focused_pane().scroll_y;
        self.panes.push(EditorPane {
            doc_id,
            scroll_y: scroll,
        });
        self.focused = 1;
    }

    pub fn focus_next(&mut self) {
        if self.panes.is_empty() {
            return;
        }
        self.focused = (self.focused + 1) % self.panes.len();
    }

    pub fn close_focused(&mut self) -> bool {
        if self.panes.len() <= 1 {
            return false;
        }
        self.panes.remove(self.focused);
        if self.focused >= self.panes.len() {
            self.focused = self.panes.len() - 1;
        }
        true
    }

    /// Garante que o pane focado aponta para `doc_id` (ex.: após next tab).
    pub fn set_focused_doc(&mut self, doc_id: DocumentId) {
        self.focused_pane_mut().doc_id = doc_id;
        self.focused_pane_mut().scroll_y = 0;
    }

    pub fn sync_scroll(&mut self, scroll_y: usize) {
        self.focused_pane_mut().scroll_y = scroll_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_close() {
        let id0 = DocumentId::from_raw(0);
        let id1 = DocumentId::from_raw(1);
        let mut s = SplitState::single(id0);
        assert!(!s.is_split());
        s.split(SplitOrientation::Vertical, id1);
        assert!(s.is_split());
        assert_eq!(s.focused, 1);
        assert!(s.close_focused());
        assert!(!s.is_split());
        assert_eq!(s.focused_pane().doc_id, id0);
    }
}
