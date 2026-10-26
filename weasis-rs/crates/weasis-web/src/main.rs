//! Weasis web frontend: the user interface running in the browser (Leptos + WebAssembly).
//!
//! Java reference: `weasis-base/weasis-base-ui` (`WeasisWin`) and the viewer/explorer modules.

mod app;
mod gallery;
mod i18n;
mod shell;
mod theme;
mod widgets;

fn main() {
    // Show Rust panics in the browser console instead of an opaque "unreachable" error.
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
