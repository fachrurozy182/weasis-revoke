//! The keyboard shortcuts of the session: the registry with the user's bindings of this operating
//! system, kept in the browser, and the conversion of browser key events.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/ShortcutManager.java`
//! (`getInstance`, `loadPreferences`, `savePreferences`).

use std::collections::BTreeMap;

use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use weasis_core::gui::{ShortcutManager, key_code_from_browser, modifier, preference_node_name};

use crate::storage;

/// The registry, shared through the Leptos context.
#[derive(Clone, Copy)]
pub struct Shortcuts {
    /// Bindings in use; every change is saved.
    pub manager: RwSignal<ShortcutManager>,
}

impl Shortcuts {
    /// The default shortcuts of this platform with the bindings the user changed.
    // TODO(7.6): read and write the node with the other preferences (server or local).
    pub fn new() -> Self {
        let os = os_name();
        let key = format!("weasis.{}", preference_node_name(os));
        let mut manager = ShortcutManager::with_defaults(os == "macos");
        if let Some(node) = storage::load::<BTreeMap<String, String>>(&key) {
            manager.load_preferences(node);
        }
        let manager = RwSignal::new(manager);
        Effect::new(move |_| manager.with(|m| storage::save(&key, &m.preferences())));
        Self { manager }
    }
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self::new()
    }
}

/// AWT key code and extended modifiers of a browser key event.
pub fn event_binding(event: &KeyboardEvent) -> (u32, u32) {
    let held = [
        (event.shift_key(), modifier::SHIFT),
        (event.ctrl_key(), modifier::CTRL),
        (event.meta_key(), modifier::META),
        (event.alt_key(), modifier::ALT),
        (event.get_modifier_state("AltGraph"), modifier::ALT_GRAPH),
    ];
    let modifiers = held
        .iter()
        .filter(|(down, _)| *down)
        .fold(0, |all, (_, mask)| all | mask);
    let key_code = key_code_from_browser(&event.key(), &event.code());
    (key_code, modifiers)
}

/// "macos", "windows" or "linux", as the shortcut preference nodes are named.
fn os_name() -> &'static str {
    let platform = window().navigator().platform().unwrap_or_default();
    if platform.starts_with("Mac") || platform.starts_with("iP") {
        "macos"
    } else if platform.starts_with("Win") {
        "windows"
    } else {
        "linux"
    }
}
