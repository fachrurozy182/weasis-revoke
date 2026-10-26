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
use crate::explorer::{
    DicomExplorer, ImportDialog, ImportReport, Importer, carries_files, chosen_files,
    dropped_entries, files_of,
};
use crate::help::online_help_url;
use crate::i18n::tr;
use crate::shortcuts::Shortcuts;

/// Issue tracker of Weasis.
const BUG_REPORT_URL: &str = "https://github.com/nroduit/Weasis/issues";

/// The main window.
#[component]
pub fn MainWindow() -> impl IntoView {
    let anonymize = RwSignal::new(false);
    let importer = Importer::new();
    provide_context(importer);
    let docking = Docking::new(dockables(importer));
    let file_input = NodeRef::<leptos::html::Input>::new();
    let import_dialog = RwSignal::new(false);
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
        action::OPEN_DICOM => {
            if let Some(input) = file_input.get() {
                input.click();
            }
        }
        action::IMPORT_DICOM => import_dialog.set(true),
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
        <div
            class="w-main-window"
            class:w-maximized=move || maximized.get()
            // Files and folders dropped from the system go to the DICOM explorer, folders read
            // recursively; tabs dragged inside the window are left to the dock.
            on:dragover=move |event| {
                if event.data_transfer().is_some_and(|data| carries_files(&data)) {
                    event.prevent_default();
                }
            }
            on:drop=move |event| {
                let Some(data) = event.data_transfer().filter(carries_files) else {
                    return;
                };
                event.prevent_default();
                let entries = dropped_entries(&data);
                leptos::task::spawn_local(async move { importer.import(files_of(entries).await) });
            }
        >
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
            <ImportReport messages=importer.report />
            <ImportDialog open=import_dialog importer=importer />
            // File ▸ Open ▸ DICOM: every file can be chosen, DICOM files often have no extension.
            // TODO(2.6): open the series in the default viewer, like `View2dFactory`.
            <input
                node_ref=file_input
                type="file"
                multiple
                hidden
                on:change=move |event| {
                    let input: web_sys::HtmlInputElement = event_target(&event);
                    importer.import(chosen_files(&input));
                    input.set_value("");
                }
            />
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
// TODO(3.14): the viewer tools fill their panels.
fn dockables(importer: Importer) -> Vec<Dockable> {
    let dockable = |id, key: &str, explorer, side, content| Dockable {
        id,
        title: tr(key).to_owned(),
        explorer,
        side,
        content,
    };
    let explorer = ViewFn::from(move || view! { <DicomExplorer importer=importer /> });
    let empty = || ViewFn::from(|| ());
    vec![
        dockable(
            "dicom-explorer",
            "DicomExplorer.title",
            true,
            DockSide::Left,
            explorer,
        ),
        dockable(
            "image-tools",
            "ImageTool.img_tool",
            false,
            DockSide::Right,
            empty(),
        ),
        dockable(
            "display",
            "DisplayTool.display",
            false,
            DockSide::Right,
            empty(),
        ),
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
        let checked = RwSignal::new(false);
        // Set at once, then on each change of the layout; the menu flips the check box itself
        // but the layout stays the reference.
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
