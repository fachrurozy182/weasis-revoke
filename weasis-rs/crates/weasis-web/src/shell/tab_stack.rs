//! Order and focus of the viewer tabs: after a close, the most recently selected remaining tab is
//! shown, like the focus history of DockingFrames.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/editor/image/ViewerPlugin.java`
//! (close action with `getFocusHistory().getNewestOn`, `CloseOthersAction`).

/// Identifier of a viewer tab.
pub type TabId = u32;

/// The tabs of the viewer area.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TabStack {
    /// Tabs in display order.
    order: Vec<TabId>,
    /// Tabs from the least to the most recently selected.
    history: Vec<TabId>,
}

impl TabStack {
    /// Tabs in display order.
    pub fn tabs(&self) -> &[TabId] {
        &self.order
    }

    /// The selected tab.
    pub fn selected(&self) -> Option<TabId> {
        self.history.last().copied()
    }

    /// Adds a tab at the end and selects it.
    pub fn open(&mut self, id: TabId) {
        if !self.order.contains(&id) {
            self.order.push(id);
        }
        self.select(id);
    }

    /// Selects a tab.
    pub fn select(&mut self, id: TabId) {
        if self.order.contains(&id) {
            self.history.retain(|&t| t != id);
            self.history.push(id);
        }
    }

    /// Removes a tab; the previously selected one is shown again.
    pub fn close(&mut self, id: TabId) {
        self.order.retain(|&t| t != id);
        self.history.retain(|&t| t != id);
    }

    /// Removes every tab but `keep`, which is selected; returns the removed tabs.
    pub fn close_others(&mut self, keep: TabId) -> Vec<TabId> {
        let removed = self.order.iter().copied().filter(|&t| t != keep).collect();
        self.order.retain(|&t| t == keep);
        self.history.retain(|&t| t == keep);
        removed
    }

    /// Removes every tab; returns them.
    pub fn close_all(&mut self) -> Vec<TabId> {
        self.history.clear();
        std::mem::take(&mut self.order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> TabStack {
        let mut stack = TabStack::default();
        for id in 1..=3 {
            stack.open(id);
        }
        stack
    }

    #[test]
    fn opening_selects_the_new_tab() {
        let stack = stack();
        assert_eq!(stack.tabs(), [1, 2, 3]);
        assert_eq!(stack.selected(), Some(3));
    }

    #[test]
    fn closing_shows_the_previously_selected_tab() {
        let mut stack = stack();
        stack.select(1);
        stack.select(3);
        stack.close(3);
        assert_eq!(stack.selected(), Some(1), "not the neighbour 2");
        stack.close(1);
        assert_eq!(stack.selected(), Some(2));
        stack.close(2);
        assert_eq!(stack.selected(), None);
    }

    #[test]
    fn close_others_and_close_all() {
        let mut stack = stack();
        assert_eq!(stack.close_others(2), [1, 3]);
        assert_eq!((stack.tabs(), stack.selected()), (&[2][..], Some(2)));
        assert_eq!(stack.close_all(), [2]);
        assert_eq!(stack, TabStack::default());
    }
}
