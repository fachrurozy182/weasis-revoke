//! The docking shortcuts: maximize and restore the selected viewer, close it, go to the next or
//! previous tab, list the panels to pick one.
//!
//! NOTE: browsers keep Ctrl+W, Ctrl+N, Ctrl+Tab and Ctrl+Shift+Tab for themselves, so the default
//! keys of Close, Normalize and the tab navigation never reach the page; they are kept, to share
//! the preferences with the desktop application, and the user can bind other keys. Externalize
//! has no meaning in a browser tab and does nothing.
//!
//! Java reference: `WeasisWin.applyDockingShortcuts` (DockingFrames `CControl` key properties and
//! `DockableSelector`).

use leptos::ev;
use leptos::prelude::*;
use weasis_core::gui::{ShortcutTable, id};

use super::dock::Docking;
use super::viewers::Viewers;
use super::working_area::WorkingArea;
use crate::i18n::tr;
use crate::shortcuts::{Shortcuts, event_binding};

/// What a docking shortcut does.
#[derive(Debug, Clone, Copy)]
enum DockCommand {
    ToggleMaximize,
    Normalize,
    Close,
    NextTab,
    PreviousTab,
    PanelList,
}

/// Listens to the docking shortcuts on the whole page. `maximized` hides the panels and the other
/// tab groups; `panel_list` opens the list of panels.
pub fn docking_keys(
    shortcuts: Shortcuts,
    viewers: Viewers,
    maximized: RwSignal<bool>,
    panel_list: RwSignal<bool>,
) {
    let table = ShortcutTable::default()
        .with(id::DOCKING_MAXIMIZE, DockCommand::ToggleMaximize)
        .with(id::DOCKING_NORMALIZE, DockCommand::Normalize)
        .with(id::DOCKING_CLOSE, DockCommand::Close)
        .with(id::DOCKING_NEXT_TAB, DockCommand::NextTab)
        .with(id::DOCKING_PREV_TAB, DockCommand::PreviousTab)
        .with(id::DOCKING_PANEL_LIST, DockCommand::PanelList);
    let handle = window_event_listener(ev::keydown, move |event| {
        let (key_code, modifiers) = event_binding(&event);
        let command = shortcuts
            .manager
            .with_untracked(|m| table.find(m, key_code, modifiers).copied());
        let Some(command) = command else {
            return;
        };
        event.prevent_default();
        let area = viewers.area;
        match command {
            DockCommand::ToggleMaximize => maximized.update(|m| *m = !*m),
            DockCommand::Normalize => maximized.set(false),
            DockCommand::Close => {
                if let Some(tab) = area.with_untracked(WorkingArea::selected) {
                    area.update(|a| a.close(tab));
                    viewers.forget(vec![tab]);
                }
            }
            DockCommand::NextTab => area.update(|a| a.select_next(true)),
            DockCommand::PreviousTab => area.update(|a| a.select_next(false)),
            DockCommand::PanelList => panel_list.set(true),
        }
    });
    on_cleanup(move || handle.remove());
}

/// The open viewers and the tool panels; a click shows the chosen one.
///
/// NOTE: DockingFrames cycles through the panels while the keys are held and shows the one
/// selected on release; a list chosen from with the mouse or the keyboard is used here.
#[component]
pub fn PanelList(open: RwSignal<bool>, viewers: Viewers, docking: Docking) -> impl IntoView {
    let close = move || open.set(false);
    let entries = move || {
        let tabs = viewers.area.with(|a| {
            let groups = a.groups();
            groups
                .iter()
                .filter_map(|&g| a.group(g))
                .flat_map(|s| s.tabs().to_vec())
                .collect::<Vec<_>>()
        });
        let viewer_items = tabs.into_iter().map(move |tab| {
            view! {
                <li role="menuitem" class="w-menu-item" on:click=move |_| {
                    close();
                    viewers.area.update(|a| a.select(tab));
                }>
                    {viewers.title(tab)}
                </li>
            }
        });
        let panel_items = docking.dockables().into_iter().map(move |dockable| {
            view! {
                <li role="menuitem" class="w-menu-item" on:click=move |_| {
                    close();
                    docking.layout.update(|l| l.show(dockable.id));
                }>
                    {dockable.title}
                </li>
            }
        });
        (viewer_items.collect_view(), panel_items.collect_view())
    };
    view! {
        <Show when=move || open.get()>
            <div class="w-backdrop" on:click=move |_| close()></div>
            <ul class="w-menu w-panel-list" role="menu" aria-label=tr("ShortcutManager.dock_panel_list")>
                {move || {
                    let (viewer_items, panel_items) = entries();
                    view! {
                        {viewer_items}
                        <li role="separator" class="w-menu-separator"></li>
                        {panel_items}
                    }
                }}
            </ul>
        </Show>
    }
}
