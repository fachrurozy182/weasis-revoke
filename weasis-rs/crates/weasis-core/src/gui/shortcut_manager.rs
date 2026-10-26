//! The registry of the configurable keyboard shortcuts: an entry per stable id with its context,
//! default binding and current binding; conflicts between overlapping contexts; the bindings the
//! user changed, stored per operating system.
//!
//! Descriptions, categories and context names are message keys (`ShortcutManager.*` of the core
//! bundle), translated by the interface.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/ShortcutManager.java`
//! and `weasis-core/docs/Shortcut-Management.md`.

use std::collections::BTreeMap;

use super::key_binding::KeyBinding;

/// Prefix of the preference node of the shortcuts.
pub const PREFERENCE_NODE: &str = "keyboard.shortcuts";
const KEY_CODE_SUFFIX: &str = ".keyCode";
const MODIFIER_SUFFIX: &str = ".modifier";

/// Where a shortcut is active. Contexts form a tree; two shortcuts conflict only when their
/// contexts overlap (one is the other or one of its ancestors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutContext {
    /// Any view of a viewer (root).
    ViewCanvas,
    /// The DICOM 2D viewer.
    DicomViewer,
    /// The MPR views of the DICOM viewer.
    Mpr,
    /// The 3D volume viewer.
    Viewer3d,
    /// The thumbnails of the DICOM explorer (separate root).
    DicomExplorer,
}

impl ShortcutContext {
    /// Parent context, `None` for a root.
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::ViewCanvas | Self::DicomExplorer => None,
            Self::DicomViewer | Self::Viewer3d => Some(Self::ViewCanvas),
            Self::Mpr => Some(Self::DicomViewer),
        }
    }

    /// Message key of the name shown in the preferences.
    pub fn name_key(self) -> &'static str {
        match self {
            Self::ViewCanvas => "ShortcutManager.ctx.view_canvas",
            Self::DicomViewer => "ShortcutManager.ctx.dicom_viewer",
            Self::Mpr => "ShortcutManager.ctx.mpr",
            Self::Viewer3d => "ShortcutManager.ctx.viewer_3d",
            Self::DicomExplorer => "ShortcutManager.ctx.dicom_explorer",
        }
    }

    /// Whether shortcuts of both contexts can be active at the same time.
    pub fn overlaps_with(self, other: Self) -> bool {
        self.is_within(other) || other.is_within(self)
    }

    /// `self` is `ancestor` or one of its descendants.
    fn is_within(self, ancestor: Self) -> bool {
        std::iter::successors(Some(self), |c| c.parent()).any(|c| c == ancestor)
    }
}

/// A configurable shortcut.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutEntry {
    /// Stable identifier, e.g. `viewer.zoomIn`.
    pub id: String,
    /// Message key of the description.
    pub description: String,
    /// Message key of the category grouping it in the preferences.
    pub category: String,
    /// Where it is active.
    pub context: ShortcutContext,
    /// Binding of the application.
    pub default_binding: KeyBinding,
    /// Binding in use.
    pub binding: KeyBinding,
}

impl ShortcutEntry {
    /// Whether the user changed the binding.
    pub fn is_modified(&self) -> bool {
        self.binding != self.default_binding
    }
}

/// The shortcuts, in registration order.
#[derive(Debug, Clone, Default)]
pub struct ShortcutManager {
    entries: Vec<ShortcutEntry>,
    /// Preferences already loaded, applied to the shortcuts registered later (plugins).
    loaded: Option<BTreeMap<String, String>>,
}

impl ShortcutManager {
    /// Adds or replaces a shortcut; it takes the user's binding when preferences were loaded.
    pub fn register(
        &mut self,
        id: &str,
        description: &str,
        category: &str,
        context: ShortcutContext,
        default_binding: KeyBinding,
    ) {
        let mut entry = ShortcutEntry {
            id: id.to_owned(),
            description: description.to_owned(),
            category: category.to_owned(),
            context,
            default_binding,
            binding: default_binding,
        };
        if let Some(preferences) = &self.loaded {
            apply_preference(&mut entry, preferences);
        }
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(existing) => *existing = entry,
            None => self.entries.push(entry),
        }
    }

    /// The shortcuts, in registration order.
    pub fn entries(&self) -> &[ShortcutEntry] {
        &self.entries
    }

    /// A shortcut.
    pub fn entry(&self, id: &str) -> Option<&ShortcutEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Binding in use; [`KeyBinding::NONE`] for an unknown id.
    pub fn binding(&self, id: &str) -> KeyBinding {
        self.entry(id).map_or(KeyBinding::NONE, |e| e.binding)
    }

    /// Whether a key event fires the shortcut `id`.
    pub fn matches(&self, id: &str, key_code: u32, modifiers: u32) -> bool {
        self.binding(id).matches(key_code, modifiers)
    }

    /// Changes a binding; false for an unknown id.
    pub fn set_shortcut(&mut self, id: &str, binding: KeyBinding) -> bool {
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(entry) => {
                entry.binding = binding;
                true
            }
            None => false,
        }
    }

    /// Restores every default binding.
    pub fn reset_all_to_defaults(&mut self) {
        for entry in &mut self.entries {
            entry.binding = entry.default_binding;
        }
    }

    /// The other shortcuts bound to `binding` in a context overlapping `context`.
    pub fn find_conflicts(
        &self,
        exclude_id: &str,
        binding: KeyBinding,
        context: ShortcutContext,
    ) -> Vec<&ShortcutEntry> {
        if !binding.is_assigned() {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|e| e.id != exclude_id && e.binding == binding)
            .filter(|e| context.overlaps_with(e.context))
            .collect()
    }

    /// Applies the bindings stored in a preference node (`<id>.keyCode`, `<id>.modifier`).
    pub fn load_preferences(&mut self, node: BTreeMap<String, String>) {
        for entry in &mut self.entries {
            apply_preference(entry, &node);
        }
        self.loaded = Some(node);
    }

    /// The preference node to store: only the bindings that differ from their default.
    pub fn preferences(&self) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .filter(|e| e.is_modified())
            .flat_map(|e| {
                [
                    (format!("{}{KEY_CODE_SUFFIX}", e.id), e.binding.key_code),
                    (format!("{}{MODIFIER_SUFFIX}", e.id), e.binding.modifiers),
                ]
            })
            .map(|(key, value)| (key, value.to_string()))
            .collect()
    }
}

/// Name of the preference node of an operating system ("macos", "windows" or "linux"): the
/// platform modifier and the keys reserved by the system differ from one to another.
pub fn preference_node_name(os: &str) -> String {
    format!("{PREFERENCE_NODE}.{os}")
}

fn apply_preference(entry: &mut ShortcutEntry, node: &BTreeMap<String, String>) {
    let read = |suffix| {
        node.get(&format!("{}{suffix}", entry.id))
            .and_then(|v| v.trim().parse::<u32>().ok())
    };
    let (key_code, modifiers) = (read(KEY_CODE_SUFFIX), read(MODIFIER_SUFFIX));
    if key_code.is_some() || modifiers.is_some() {
        entry.binding = KeyBinding::new(
            key_code.unwrap_or(entry.binding.key_code),
            modifiers.unwrap_or(entry.binding.modifiers),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::key_binding::{key, modifier};

    fn manager() -> ShortcutManager {
        let mut manager = ShortcutManager::default();
        let binding = KeyBinding::new('X' as u32, modifier::ALT);
        manager.register("mpr.recenter", "d", "c", ShortcutContext::Mpr, binding);
        manager.register("v3d.x", "d", "c", ShortcutContext::Viewer3d, binding);
        manager.register(
            "view.up",
            "d",
            "c",
            ShortcutContext::ViewCanvas,
            KeyBinding::new(key::UP, 0),
        );
        manager
    }

    #[test]
    fn contexts_overlap_along_their_branch() {
        use ShortcutContext::*;
        assert!(Mpr.overlaps_with(ViewCanvas) && ViewCanvas.overlaps_with(Mpr));
        assert!(Mpr.overlaps_with(DicomViewer));
        assert!(!Mpr.overlaps_with(Viewer3d), "siblings");
        assert!(!DicomExplorer.overlaps_with(ViewCanvas), "separate root");
    }

    #[test]
    fn conflicts_are_limited_to_overlapping_contexts() {
        let manager = manager();
        let alt_x = KeyBinding::new('X' as u32, modifier::ALT);
        let ids = |context| -> Vec<String> {
            let found = manager.find_conflicts("none", alt_x, context);
            found.iter().map(|e| e.id.clone()).collect()
        };
        assert_eq!(ids(ShortcutContext::DicomViewer), ["mpr.recenter"]);
        assert_eq!(ids(ShortcutContext::ViewCanvas), ["mpr.recenter", "v3d.x"]);
        assert!(
            manager
                .find_conflicts("x", KeyBinding::NONE, ShortcutContext::Mpr)
                .is_empty()
        );
    }

    #[test]
    fn only_changed_bindings_are_stored_and_reloaded() {
        let mut manager = manager();
        assert!(manager.preferences().is_empty());
        manager.set_shortcut("view.up", KeyBinding::new('K' as u32, modifier::CTRL));
        let stored = manager.preferences();
        assert_eq!(
            stored.get("view.up.keyCode").map(String::as_str),
            Some("75")
        );
        assert_eq!(
            stored.get("view.up.modifier").map(String::as_str),
            Some("128")
        );

        let mut fresh = self::manager();
        fresh.load_preferences(stored);
        assert!(fresh.matches("view.up", 'K' as u32, modifier::CTRL));
        // A shortcut registered later, by a plugin, gets the stored binding too.
        let node = BTreeMap::from([("late.keyCode".to_owned(), "76".to_owned())]);
        fresh.load_preferences(node);
        fresh.register(
            "late",
            "d",
            "c",
            ShortcutContext::ViewCanvas,
            KeyBinding::NONE,
        );
        assert_eq!(fresh.binding("late").key_code, 'L' as u32);
        fresh.reset_all_to_defaults();
        assert_eq!(fresh.binding("late"), KeyBinding::NONE);
        assert_eq!(preference_node_name("macos"), "keyboard.shortcuts.macos");
    }
}
