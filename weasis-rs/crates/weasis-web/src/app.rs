//! Root component of the web application.
//!
//! Placeholder page for step 0.6; the main window from `WeasisWin` replaces it in step 2.2.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`

use leptos::prelude::*;

/// Application version shown on the page, taken from `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Root component mounted on `<body>`.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"Weasis"</h1>
            <p>"Web DICOM viewer, version " {VERSION}</p>
        </main>
    }
}
