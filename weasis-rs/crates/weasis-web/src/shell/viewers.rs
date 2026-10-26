//! The viewer area: a tab per open viewer, closed from its tab or from the tab context menu.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/editor/image/ViewerPlugin.java`
//! (a `DefaultSingleCDockable` in the DockingFrames `CWorkingArea`) and `WeasisWin.java`
//! (`setSelectedPlugin`).

use std::collections::BTreeMap;

use leptos::prelude::*;

use super::tab_stack::{TabId, TabStack};
use crate::i18n::tr;

/// An open viewer.
#[derive(Clone)]
pub struct Viewer {
    /// Title of its tab.
    pub title: String,
    /// Tooltip of its tab.
    pub tooltip: Option<String>,
    /// Content.
    pub content: ViewFn,
}

/// The open viewers, shared with the explorers and menus that open them (Leptos context).
// TODO(2.2.d): tab groups of the working area (split right/down, move to another group).
#[derive(Clone, Copy)]
pub struct Viewers {
    stack: RwSignal<TabStack>,
    viewers: StoredValue<BTreeMap<TabId, Viewer>>,
    next_id: StoredValue<TabId>,
}

impl Viewers {
    /// No viewer.
    pub fn new() -> Self {
        Self {
            stack: RwSignal::new(TabStack::default()),
            viewers: StoredValue::new(BTreeMap::new()),
            next_id: StoredValue::new(1),
        }
    }

    /// Opens a viewer in a new selected tab.
    pub fn open(&self, viewer: Viewer) -> TabId {
        let id = self.next_id.get_value();
        self.next_id.set_value(id + 1);
        self.viewers.update_value(|v| {
            v.insert(id, viewer);
        });
        self.stack.update(|s| s.open(id));
        id
    }

    fn title(&self, id: TabId) -> Option<String> {
        self.viewers
            .with_value(|v| v.get(&id).map(|viewer| viewer.title.clone()))
    }

    fn close(&self, ids: Vec<TabId>) {
        self.viewers
            .update_value(|v| ids.iter().for_each(|id| _ = v.remove(id)));
    }
}

impl Default for Viewers {
    fn default() -> Self {
        Self::new()
    }
}

/// The tabs of the open viewers and their content; the content of hidden tabs is kept.
#[component]
pub fn ViewerArea(viewers: Viewers) -> impl IntoView {
    let stack = viewers.stack;
    let tabs = move || stack.with(|s| s.tabs().to_vec());
    let selected = move |id| stack.with(|s| s.selected() == Some(id));
    // Tab whose context menu is open, with the pointer position.
    let context_menu = RwSignal::new(None::<(TabId, i32, i32)>);
    let close = move |id| {
        stack.update(|s| s.close(id));
        viewers.close(vec![id]);
    };
    view! {
        <Show when=move || !tabs().is_empty()>
            <div class="w-viewer-tabs" role="tablist">
                <For each=tabs key=|id| *id let:id>
                    <div
                        role="tab"
                        class="w-viewer-tab"
                        title=viewers.viewers.with_value(|v| v.get(&id).and_then(|viewer| viewer.tooltip.clone()))
                        aria-selected=move || selected(id).to_string()
                        on:click=move |_| stack.update(|s| s.select(id))
                        on:contextmenu=move |event| {
                            event.prevent_default();
                            context_menu.set(Some((id, event.client_x(), event.client_y())));
                        }
                    >
                        <span>{viewers.title(id)}</span>
                        <button
                            type="button"
                            class="w-viewer-tab-close"
                            title=tr("WeasisAboutBox.close")
                            on:click=move |event| {
                                event.stop_propagation();
                                close(id);
                            }
                        ></button>
                    </div>
                </For>
            </div>
        </Show>
        <div class="w-viewer-content">
            <For each=tabs key=|id| *id let:id>
                <div class="w-viewer" role="tabpanel" hidden=move || !selected(id)>
                    {viewers.viewers.with_value(|v| v.get(&id).map(|viewer| viewer.content.run()))}
                </div>
            </For>
        </div>
        {move || context_menu.get().map(|(id, x, y)| {
            let dismiss = move || context_menu.set(None);
            view! {
                <div class="w-backdrop" on:click=move |_| dismiss()></div>
                <ul class="w-menu w-context-menu" role="menu" style=format!("left: {x}px; top: {y}px")>
                    <li role="menuitem" class="w-menu-item" on:click=move |_| {
                        dismiss();
                        let removed = stack.try_update(|s| s.close_others(id)).unwrap_or_default();
                        viewers.close(removed);
                    }>
                        {tr("ViewerPlugin.close_other")}
                    </li>
                    <li role="menuitem" class="w-menu-item" on:click=move |_| {
                        dismiss();
                        let removed = stack.try_update(TabStack::close_all).unwrap_or_default();
                        viewers.close(removed);
                    }>
                        {tr("ViewerPlugin.close_all")}
                    </li>
                </ul>
            }
        })}
    }
}
