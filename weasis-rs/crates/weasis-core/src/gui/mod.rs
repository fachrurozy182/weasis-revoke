//! Interface logic shared by every view: keyboard shortcuts.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/`.

mod key_binding;
mod shortcut_defaults;
mod shortcut_manager;

pub use key_binding::{KeyBinding, key, key_code_from_browser, key_text, modifier, to_extended};
pub use shortcut_defaults::id;
pub use shortcut_manager::{
    PREFERENCE_NODE, ShortcutContext, ShortcutEntry, ShortcutManager, preference_node_name,
};
