//! Shortcut ids bound to values (actions, directions, predicates), resolved against the current
//! bindings at each key event, so that a binding changed in the preferences applies at once.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/ShortcutTable.java`
//! and `ShortcutActions.java` (a table of actions).

use super::shortcut_manager::ShortcutManager;

/// Ids and their values, in the order they were added: when two ids share a binding, the first
/// one wins.
#[derive(Debug, Clone)]
pub struct ShortcutTable<T> {
    entries: Vec<(&'static str, T)>,
}

impl<T> Default for ShortcutTable<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> ShortcutTable<T> {
    /// Adds an id with its value.
    pub fn with(mut self, id: &'static str, value: T) -> Self {
        self.entries.push((id, value));
        self
    }

    /// Value of the first id the key event matches.
    pub fn find(&self, shortcuts: &ShortcutManager, key_code: u32, modifiers: u32) -> Option<&T> {
        self.entries
            .iter()
            .find(|(id, _)| shortcuts.matches(id, key_code, modifiers))
            .map(|(_, value)| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::id::{DICOM_PREV_SERIES, DRAW_CANCEL_DRAWING, VIEWER_ESCAPE};
    use crate::gui::key_binding::{key, modifier};

    #[test]
    fn the_first_matching_id_wins() {
        let shortcuts = ShortcutManager::with_defaults(false);
        let table = ShortcutTable::default()
            .with(DRAW_CANCEL_DRAWING, "cancel")
            .with(VIEWER_ESCAPE, "reset")
            .with(DICOM_PREV_SERIES, "previous");
        assert_eq!(table.find(&shortcuts, key::ESCAPE, 0), Some(&"cancel"));
        assert_eq!(table.find(&shortcuts, key::LEFT, 0), Some(&"previous"));
        assert_eq!(table.find(&shortcuts, key::LEFT, modifier::META), None);
    }
}
