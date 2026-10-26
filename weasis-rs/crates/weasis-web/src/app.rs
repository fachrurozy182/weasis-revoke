//! Root component of the web application.
//!
//! The main window from `WeasisWin` replaces the placeholder content in step 2.2.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`

use leptos::prelude::*;

use crate::gallery::Gallery;
use crate::theme::Theme;

/// Application version shown on the page, taken from `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Root component mounted on `<body>`.
#[component]
pub fn App() -> impl IntoView {
    // `#gallery` shows the base components (fixture of the end-to-end tests).
    let gallery = window()
        .location()
        .hash()
        .is_ok_and(|hash| hash == "#gallery");
    view! {
        <ThemeStyle />
        {if gallery {
            view! { <Gallery /> }.into_any()
        } else {
            view! {
                <main>
                    <h1>"Weasis"</h1>
                    <p>"Web DICOM viewer, version " {VERSION}</p>
                </main>
            }
                .into_any()
        }}
    }
}

/// Declares the variables of the Weasis theme for the style sheet.
#[component]
fn ThemeStyle() -> impl IntoView {
    // The theme is compiled in and tested: an error here is a programming mistake, shown in the
    // console, and the page keeps the browser defaults.
    let css = Theme::weasis()
        .map(|theme| theme.css())
        .unwrap_or_else(|error| {
            leptos::logging::error!("invalid Weasis theme: {error}");
            String::new()
        });
    view! { <style id="weasis-theme">{css}</style> }
}
