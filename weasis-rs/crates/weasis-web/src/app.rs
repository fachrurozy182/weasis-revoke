//! Root component of the web application.
//!
//! It shows the main window, or the component gallery with `#gallery`.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`

use leptos::prelude::*;

use crate::gallery::Gallery;
use crate::shell::MainWindow;
use crate::theme::Theme;

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
            view! { <MainWindow /> }.into_any()
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
