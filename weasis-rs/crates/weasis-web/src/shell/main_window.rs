//! The main window: menu bar with its indicators, toolbars, docked panels around the viewers.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`.

use leptos::prelude::*;

use super::dock::{DockPanel, DockStrip, Dockable, Docking, DropZones};
use super::dock_layout::{DockMode, DockSide};
use super::docking_keys::{PanelList, docking_keys};
use super::masking_indicator::{MaskingIndicator, masking_label};
use super::menu::{MenuBar, MenuItem};
use super::menus::{action, main_menus};
use super::shortcut_page::shortcut_page;
use super::viewers::{ViewerArea, Viewers};
use crate::i18n::tr;
use crate::shortcuts::Shortcuts;

/// Online tutorials (`weasis.help.online` of the launcher preferences).
// TODO(7.6): read `weasis.help.online` from the preferences.
const ONLINE_HELP_URL: &str = "https://nroduit.github.io/en/tutorials/";
/// Issue tracker of Weasis.
const BUG_REPORT_URL: &str = "https://github.com/nroduit/Weasis/issues";

/// The main window.
#[component]
pub fn MainWindow() -> impl IntoView {
    let anonymize = RwSignal::new(false);
    let docking = Docking::new(dockables());
    let (tools, explorers) = dockable_menu_items(docking);
    let viewers = Viewers::new();
    provide_context(viewers);
    let shortcuts = Shortcuts::new();
    provide_context(shortcuts);
    let maximized = RwSignal::new(false);
    let panel_list = RwSignal::new(false);
    docking_keys(shortcuts, viewers, maximized, panel_list);
    Effect::new(move |_| document().set_title(&window_title(anonymize.get())));
    let on_action = Callback::new(move |id: &'static str| match id {
        action::ONLINE_HELP => open_in_new_tab(&online_help_url("")),
        action::SHORTCUTS => open_page(&shortcuts.manager.with(shortcut_page)),
        action::BUG_REPORT => open_in_new_tab(BUG_REPORT_URL),
        id if docking.find(id).is_some() => docking.layout.update(|l| {
            if l.placement(id).is_some_and(|p| p.mode == DockMode::Closed) {
                l.show(id);
            } else {
                l.close(id);
            }
        }),
        // TODO(2.10, 7.6): resource monitor, about box, preferences.
        other => leptos::logging::log!("menu action not available yet: {other}"),
    });
    view! {
        <div class="w-main-window" class:w-maximized=move || maximized.get()>
            <header class="w-main-header">
                <MenuBar menus=main_menus(anonymize, tools, explorers) on_action=on_action />
                // TODO(7.2): the capture button (screenshots and recordings of the selected viewer).
                <MaskingIndicator masking=anonymize />
            </header>
            <div class="w-toolbars"></div>
            <main class="w-main-area">
                <DockStrip docking=docking side=DockSide::Left />
                <DockPanel docking=docking side=DockSide::Left />
                <div class="w-center">
                    <div class="w-viewers">
                        <ViewerArea viewers=viewers />
                        <DropZones docking=docking />
                    </div>
                    <DockPanel docking=docking side=DockSide::Bottom />
                    <DockStrip docking=docking side=DockSide::Bottom />
                </div>
                <DockPanel docking=docking side=DockSide::Right />
                <DockStrip docking=docking side=DockSide::Right />
            </main>
            <PanelList open=panel_list viewers=viewers docking=docking />
        </div>
    }
}

/// "Weasis v<version>", followed by the masking badge text while masking is on.
fn window_title(masking: bool) -> String {
    let title = format!("Weasis v{}", env!("CARGO_PKG_VERSION"));
    if masking {
        format!("{title} — {}", masking_label())
    } else {
        title
    }
}

/// The dockables of the main window with their default side.
// TODO(2.4, 3.x): the DICOM explorer and the viewer tools fill these panels.
fn dockables() -> Vec<Dockable> {
    let dockable = |id, key: &str, explorer, side| Dockable {
        id,
        title: tr(key).to_owned(),
        explorer,
        side,
        content: ViewFn::from(|| ()),
    };
    vec![
        dockable(
            "dicom-explorer",
            "DicomExplorer.title",
            true,
            DockSide::Left,
        ),
        dockable("image-tools", "ImageTool.img_tool", false, DockSide::Right),
        dockable("display", "DisplayTool.display", false, DockSide::Right),
    ]
}

/// Check items of View ▸ Tools and View ▸ Explorer, checked while their dockable is not closed.
fn dockable_menu_items(docking: Docking) -> (Vec<MenuItem>, Vec<MenuItem>) {
    let (mut tools, mut explorers) = (Vec::new(), Vec::new());
    for dockable in docking.dockables() {
        let open = move || {
            docking.layout.with(|l| {
                l.placement(dockable.id)
                    .is_some_and(|p| p.mode != DockMode::Closed)
            })
        };
        let checked = RwSignal::new(open());
        // The menu flips the check box itself; the layout stays the reference.
        Effect::new(move |_| checked.set(open()));
        let item = MenuItem::Check {
            id: dockable.id,
            label: dockable.title,
            title: None,
            checked,
        };
        if dockable.explorer {
            explorers.push(item);
        } else {
            tools.push(item);
        }
    }
    (tools, explorers)
}

/// Page of the online tutorials about `topic` (the index when empty).
///
/// NOTE: the desktop application adds its version (`?v=4.8.0`) for the documentation of that
/// version; the version of this rewrite does not number Weasis releases, so none is added.
pub(super) fn online_help_url(topic: &str) -> String {
    format!("{ONLINE_HELP_URL}{topic}")
}

/// Opens a generated HTML document in a new tab.
fn open_page(html: &str) {
    let parts = js_sys::Array::of1(&js_sys::JsString::from(html));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/html");
    let url = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
        .and_then(|blob| web_sys::Url::create_object_url_with_blob(&blob));
    match url {
        Ok(url) => open_in_new_tab(&url),
        Err(error) => leptos::logging::error!("cannot create the page: {error:?}"),
    }
}

fn open_in_new_tab(url: &str) {
    if let Err(error) = window().open_with_url_and_target(url, "_blank") {
        leptos::logging::error!("cannot open {url}: {error:?}");
    }
}
