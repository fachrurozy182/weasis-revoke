//! The main window: menu bar, toolbars, docked panels around the viewers, status bar.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`.

mod menu;
mod menus;

use leptos::prelude::*;

use menu::MenuBar;
use menus::{action, main_menus};

/// Online tutorials (`weasis.help.online` of the launcher preferences).
const ONLINE_HELP_URL: &str = "https://nroduit.github.io/en/tutorials/";
/// Issue tracker of Weasis.
const BUG_REPORT_URL: &str = "https://github.com/nroduit/Weasis/issues";

/// The main window.
#[component]
pub fn MainWindow() -> impl IntoView {
    let anonymize = RwSignal::new(false);
    let on_action = Callback::new(move |id: &'static str| match id {
        action::ONLINE_HELP => open_in_new_tab(ONLINE_HELP_URL),
        action::BUG_REPORT => open_in_new_tab(BUG_REPORT_URL),
        // TODO(2.3, 2.10, 7.6): shortcuts page, resource monitor, about box, preferences.
        other => leptos::logging::log!("menu action not available yet: {other}"),
    });
    view! {
        <div class="w-main-window">
            <header class="w-main-header">
                <MenuBar menus=main_menus(anonymize) on_action=on_action />
            </header>
            <div class="w-toolbars"></div>
            <main class="w-main-area"></main>
        </div>
    }
}

fn open_in_new_tab(url: &str) {
    if let Err(error) = window().open_with_url_and_target(url, "_blank") {
        leptos::logging::error!("cannot open {url}: {error:?}");
    }
}
