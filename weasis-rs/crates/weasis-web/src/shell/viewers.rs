//! The viewer area: tab groups holding a tab per open viewer, split and resized like the
//! DockingFrames working area.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/editor/image/ViewerPlugin.java`
//! (a `DefaultSingleCDockable` in the `CWorkingArea`) and `WeasisWin.java` (`setSelectedPlugin`).

use std::collections::BTreeMap;

use leptos::prelude::*;

use super::tab_stack::TabId;
use super::viewer_menu::{TabMenu, TabMenuAt};
use super::working_area::{Node, NodeId, WorkingArea};
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
#[derive(Clone, Copy)]
pub struct Viewers {
    pub(super) area: RwSignal<WorkingArea>,
    viewers: StoredValue<BTreeMap<TabId, Viewer>>,
    next_id: StoredValue<TabId>,
}

impl Viewers {
    /// No viewer.
    pub fn new() -> Self {
        Self {
            area: RwSignal::new(WorkingArea::default()),
            viewers: StoredValue::new(BTreeMap::new()),
            next_id: StoredValue::new(1),
        }
    }

    /// Opens a viewer in a new selected tab of the focused group.
    pub fn open(&self, viewer: Viewer) -> TabId {
        let id = self.next_id.get_value();
        self.next_id.set_value(id + 1);
        self.viewers.update_value(|v| {
            v.insert(id, viewer);
        });
        self.area.update(|a| a.open(id));
        id
    }

    pub(super) fn title(&self, id: TabId) -> String {
        self.viewers.with_value(|v| {
            v.get(&id)
                .map(|viewer| viewer.title.clone())
                .unwrap_or_default()
        })
    }

    /// Drops the viewers whose tabs were removed.
    pub(super) fn forget(&self, ids: Vec<TabId>) {
        self.viewers
            .update_value(|v| ids.iter().for_each(|id| _ = v.remove(id)));
    }
}

impl Default for Viewers {
    fn default() -> Self {
        Self::new()
    }
}

/// Shape of a node, without its tabs and ratio, so that a node is rebuilt only when its shape
/// changes.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Group,
    Split {
        down: bool,
        first: NodeId,
        second: NodeId,
    },
}

/// The tab groups and their viewers.
#[component]
pub fn ViewerArea(viewers: Viewers) -> impl IntoView {
    let menu = RwSignal::new(None::<TabMenuAt>);
    let root = Memo::new(move |_| viewers.area.with(WorkingArea::root));
    view! {
        <div class="w-working-area">
            {move || root.get().map(|id| node_view(viewers, menu, id))}
        </div>
        <TabMenu viewers=viewers at=menu />
    }
}

/// A group, or a split of two nodes with a splitter between them.
///
/// NOTE: when a split disappears or a tab changes group, the viewers concerned are rebuilt (their
/// state lives in the model, not in the page), while DockingFrames moves the Swing component.
fn node_view(viewers: Viewers, menu: RwSignal<Option<TabMenuAt>>, id: NodeId) -> AnyView {
    let shape = Memo::new(move |_| {
        viewers.area.with(|a| match a.node(id)? {
            Node::Group(_) => Some(Shape::Group),
            Node::Split {
                down,
                first,
                second,
                ..
            } => Some(Shape::Split {
                down: *down,
                first: *first,
                second: *second,
            }),
        })
    });
    (move || match shape.get() {
        Some(Shape::Group) => view! { <TabGroup viewers=viewers menu=menu id=id /> }.into_any(),
        Some(Shape::Split {
            down,
            first,
            second,
        }) => split_view(viewers, menu, id, down, first, second),
        None => ().into_any(),
    })
    .into_any()
}

fn split_view(
    viewers: Viewers,
    menu: RwSignal<Option<TabMenuAt>>,
    id: NodeId,
    down: bool,
    first: NodeId,
    second: NodeId,
) -> AnyView {
    let ratio = move || {
        viewers.area.with(|a| match a.node(id) {
            Some(Node::Split { ratio, .. }) => *ratio,
            _ => 0.5,
        })
    };
    let container = NodeRef::<leptos::html::Div>::new();
    let divider = NodeRef::<leptos::html::Div>::new();
    let dragging = StoredValue::new(false);
    let on_move = move |event: leptos::ev::PointerEvent| {
        let Some(element) = container.get().filter(|_| dragging.get_value()) else {
            return;
        };
        let bounds = element.get_bounding_client_rect();
        let share = if down {
            (f64::from(event.client_y()) - bounds.top()) / bounds.height()
        } else {
            (f64::from(event.client_x()) - bounds.left()) / bounds.width()
        };
        viewers.area.update(|a| a.resize(id, share));
    };
    view! {
        <div node_ref=container class="w-split" class:w-split-down=down>
            <div class="w-split-part" style:flex-grow=move || ratio().to_string()>
                {node_view(viewers, menu, first)}
            </div>
            <div
                node_ref=divider
                class="w-split-divider"
                role="separator"
                on:pointerdown=move |event| {
                    if let Some(element) = divider.get() {
                        let _ = element.set_pointer_capture(event.pointer_id());
                    }
                    dragging.set_value(true);
                }
                on:pointermove=on_move
                on:pointerup=move |_| dragging.set_value(false)
            ></div>
            <div class="w-split-part" style:flex-grow=move || (1.0 - ratio()).to_string()>
                {node_view(viewers, menu, second)}
            </div>
        </div>
    }
    .into_any()
}

/// The tabs of a group and their content; the content of hidden tabs is kept.
#[component]
fn TabGroup(viewers: Viewers, menu: RwSignal<Option<TabMenuAt>>, id: NodeId) -> impl IntoView {
    let area = viewers.area;
    let tabs = move || area.with(|a| a.group(id).map(|s| s.tabs().to_vec()).unwrap_or_default());
    let selected = move |tab| area.with(|a| a.group(id).and_then(|s| s.selected()) == Some(tab));
    let focused = move || area.with(|a| a.selected().is_some_and(|t| a.group_of(t) == Some(id)));
    let close = move |tab| {
        area.update(|a| a.close(tab));
        viewers.forget(vec![tab]);
    };
    view! {
        <section class="w-tab-group" class:w-focused=focused>
            <Show when=move || !tabs().is_empty()>
                <div class="w-viewer-tabs" role="tablist">
                    <For each=tabs key=|tab| *tab let:tab>
                        <div
                            role="tab"
                            class="w-viewer-tab"
                            title=viewers.viewers.with_value(|v| v.get(&tab).and_then(|viewer| viewer.tooltip.clone()))
                            aria-selected=move || selected(tab).to_string()
                            on:click=move |_| area.update(|a| a.select(tab))
                            on:contextmenu=move |event| {
                                event.prevent_default();
                                menu.set(Some(TabMenuAt { tab, x: event.client_x(), y: event.client_y() }));
                            }
                        >
                            <span>{viewers.title(tab)}</span>
                            <button
                                type="button"
                                class="w-viewer-tab-close"
                                title=tr("WeasisAboutBox.close")
                                on:click=move |event| {
                                    event.stop_propagation();
                                    close(tab);
                                }
                            ></button>
                        </div>
                    </For>
                </div>
            </Show>
            <div class="w-viewer-content">
                <For each=tabs key=|tab| *tab let:tab>
                    <div
                        class="w-viewer"
                        role="tabpanel"
                        hidden=move || !selected(tab)
                        on:pointerdown=move |_| area.update(|a| a.select(tab))
                    >
                        {viewers.viewers.with_value(|v| v.get(&tab).map(|viewer| viewer.content.run()))}
                    </div>
                </For>
            </div>
        </section>
    }
}
