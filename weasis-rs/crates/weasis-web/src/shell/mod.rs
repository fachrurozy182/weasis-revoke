//! The main window: menu bar, toolbars, docked panels around the viewers, status bar.
//!
//! Java reference: `weasis-base/weasis-base-ui/src/main/java/org/weasis/base/ui/gui/WeasisWin.java`.

mod dock;
mod dock_layout;
mod masking_indicator;
mod menu;
mod menus;
mod tab_stack;
mod viewers;

use leptos::prelude::*;

use crate::i18n::tr;
use dock::{DockPanel, DockStrip, Dockable, Docking, DropZones};
use dock_layout::{DockMode, DockSide};
use masking_indicator::{MaskingIndicator, masking_label};
use menu::{MenuBar, MenuItem};
use menus::{action, main_menus};
pub use viewers::{Viewer, ViewerArea, Viewers};

/// Online tutorials (`weasis.help.online` of the launcher preferences).
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
    Effect::new(move |_| document().set_title(&window_title(anonymize.get())));
    let on_action = Callback::new(move |id: &'static str| match id {
        action::ONLINE_HELP => open_in_new_tab(ONLINE_HELP_URL),
        action::BUG_REPORT => open_in_new_tab(BUG_REPORT_URL),
        id if docking.find(id).is_some() => docking.layout.update(|l| {
            if l.placement(id).is_some_and(|p| p.mode == DockMode::Closed) {
                l.show(id);
            } else {
                l.close(id);
            }
        }),
        // TODO(2.3, 2.10, 7.6): shortcuts page, resource monitor, about box, preferences.
        other => leptos::logging::log!("menu action not available yet: {other}"),
    });
    view! {
        <div class="w-main-window">
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

fn open_in_new_tab(url: &str) {
    if let Err(error) = window().open_with_url_and_target(url, "_blank") {
        leptos::logging::error!("cannot open {url}: {error:?}");
    }
}
