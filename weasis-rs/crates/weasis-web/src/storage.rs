//! Values kept in the browser between sessions (decision D8): the dock layout, the shortcuts the
//! user changed. Storage can be missing (private mode) or full; the value then simply is not kept.

use leptos::prelude::window;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The value stored under `key`; none when absent, unreadable or invalid.
pub fn load<T: DeserializeOwned>(key: &str) -> Option<T> {
    let text = window().local_storage().ok()??.get_item(key).ok()??;
    serde_json::from_str(&text).ok()
}

/// Stores a value under `key`.
pub fn save<T: Serialize>(key: &str, value: &T) {
    let storage = window().local_storage().ok().flatten();
    if let (Some(storage), Ok(text)) = (storage, serde_json::to_string(value)) {
        let _ = storage.set_item(key, &text);
    }
}
