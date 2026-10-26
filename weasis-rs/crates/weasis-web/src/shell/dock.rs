//! The docked panels around the viewers: one panel per side with a tab per dockable, a strip of
//! minimized dockables on each side, a splitter to resize a side, and tabs dragged to another
//! side to move them.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/docking/UIManager.java` and
//! `PluginTool.java` (DockingFrames `CControl` with its minimize and close actions).

use leptos::prelude::*;

use super::dock_layout::{DockLayout, DockMode, DockSide};

/// Key of the layout in the browser storage (decision D8).
const STORAGE_KEY: &str = "weasis.dock-layout";

/// A panel that can be docked on a side of the main window.
#[derive(Clone)]
pub struct Dockable {
    /// Stable identifier, also the id of its View menu item.
    pub id: &'static str,
    /// Title of its tab and of its menu item.
    pub title: String,
    /// Listed in View ▸ Explorer instead of View ▸ Tools.
    pub explorer: bool,
    /// Default side.
    pub side: DockSide,
    /// Content.
    pub content: ViewFn,
}

/// State shared by the dock components.
#[derive(Clone, Copy)]
pub struct Docking {
    /// Where the dockables are.
    pub layout: RwSignal<DockLayout>,
    dockables: StoredValue<Vec<Dockable>>,
    /// Dockable whose tab is being dragged.
    dragging: RwSignal<Option<&'static str>>,
}

impl Docking {
    /// Registers the dockables over the layout saved in the browser, and saves every change.
    pub fn new(dockables: Vec<Dockable>) -> Self {
        let mut layout = load_layout().unwrap_or_default();
        for dockable in &dockables {
            layout.register(dockable.id, dockable.side, DockMode::Docked);
        }
        let layout = RwSignal::new(layout);
        Effect::new(move |_| layout.with(save_layout));
        Self {
            layout,
            dockables: StoredValue::new(dockables),
            dragging: RwSignal::new(None),
        }
    }

    /// The dockables, in registration order.
    pub fn dockables(&self) -> Vec<Dockable> {
        self.dockables.get_value()
    }

    /// The static id of the dockable `id`, if it is one.
    pub fn find(&self, id: &str) -> Option<&'static str> {
        self.dockables
            .with_value(|d| d.iter().find(|d| d.id == id).map(|d| d.id))
    }

    /// Dockables of a side in a mode.
    fn on_side(&self, side: DockSide, mode: DockMode) -> Vec<&'static str> {
        self.layout.with(|l| {
            l.on_side(side, mode)
                .into_iter()
                .filter_map(|id| self.find(id))
                .collect()
        })
    }

    fn title(&self, id: &str) -> String {
        self.dockables.with_value(|d| {
            d.iter()
                .find(|d| d.id == id)
                .map(|d| d.title.clone())
                .unwrap_or_default()
        })
    }

    fn content(&self, id: &str) -> Option<ViewFn> {
        self.dockables
            .with_value(|d| d.iter().find(|d| d.id == id).map(|d| d.content.clone()))
    }
}

/// The panel of a side: tabs of its docked dockables and the content of the selected one.
#[component]
pub fn DockPanel(docking: Docking, side: DockSide) -> impl IntoView {
    let layout = docking.layout;
    let active = Memo::new(move |_| layout.with(|l| l.active(side).map(str::to_owned)));
    let tabs = move || {
        docking
            .on_side(side, DockMode::Docked)
            .into_iter()
            .map(|id| view! { <DockTab docking=docking id=id selected=Signal::derive(move || active.get().as_deref() == Some(id)) /> })
            .collect_view()
    };
    let dimension = if side == DockSide::Bottom {
        "height"
    } else {
        "width"
    };
    let style = move || format!("{dimension}: {}px", layout.with(|l| l.size(side)));
    view! {
        <Show when=move || active.get().is_some()>
            <section
                class=format!("w-dock w-dock-{}", side_name(side))
                style=style
                aria-label=move || active.get().map(|id| docking.title(&id))
            >
                <Splitter docking=docking side=side />
                <div class="w-dock-header">
                    <div class="w-dock-tabs" role="tablist">{tabs}</div>
                    <button
                        type="button"
                        class="w-dock-button w-dock-minimize"
                        title="Minimize"
                        on:click=move |_| if let Some(id) = active.get() { layout.update(|l| l.minimize(&id)) }
                    ></button>
                    <button
                        type="button"
                        class="w-dock-button w-dock-close"
                        title="Close"
                        on:click=move |_| if let Some(id) = active.get() { layout.update(|l| l.close(&id)) }
                    ></button>
                </div>
                <div class="w-dock-content" role="tabpanel">
                    {move || active.get().and_then(|id| docking.content(&id)).map(|content| content.run())}
                </div>
            </section>
        </Show>
    }
}

/// A tab; a click selects it, a drag moves its dockable to the side it is dropped on.
#[component]
fn DockTab(docking: Docking, id: &'static str, selected: Signal<bool>) -> impl IntoView {
    view! {
        <button
            type="button"
            role="tab"
            class="w-dock-tab"
            draggable="true"
            aria-selected=move || selected.get().to_string()
            on:click=move |_| docking.layout.update(|l| l.show(id))
            on:dragstart=move |event| {
                // Firefox starts a drag only when it carries data.
                if let Some(data) = event.data_transfer() {
                    let _ = data.set_data("text/plain", id);
                }
                docking.dragging.set(Some(id));
            }
            on:dragend=move |_| docking.dragging.set(None)
        >
            {docking.title(id)}
        </button>
    }
}

/// The edge between a side panel and the viewers, dragged to resize the side.
#[component]
fn Splitter(docking: Docking, side: DockSide) -> impl IntoView {
    let splitter = NodeRef::<leptos::html::Div>::new();
    // Pointer position and side size when the drag started.
    let start = StoredValue::new(None::<(f64, f64)>);
    let position = move |event: &leptos::ev::PointerEvent| match side {
        DockSide::Bottom => f64::from(event.client_y()),
        _ => f64::from(event.client_x()),
    };
    view! {
        <div
            node_ref=splitter
            class="w-dock-splitter"
            role="separator"
            on:pointerdown=move |event| {
                if let Some(element) = splitter.get() {
                    let _ = element.set_pointer_capture(event.pointer_id());
                }
                start.set_value(Some((position(&event), docking.layout.with(|l| l.size(side)))));
            }
            on:pointermove=move |event| {
                if let Some((from, size)) = start.get_value() {
                    let delta = position(&event) - from;
                    // The left panel grows to the right; the others grow towards the viewers.
                    let size = if side == DockSide::Left { size + delta } else { size - delta };
                    docking.layout.update(|l| l.resize(side, size));
                }
            }
            on:pointerup=move |_| start.set_value(None)
        ></div>
    }
}

/// The minimized dockables of a side; a click docks one again.
///
/// NOTE: DockingFrames shows a minimized dockable in a flyout over the viewers; here a click
/// docks it back, which keeps the viewers uncovered.
#[component]
pub fn DockStrip(docking: Docking, side: DockSide) -> impl IntoView {
    let minimized = move || docking.on_side(side, DockMode::Minimized);
    view! {
        <Show when=move || !minimized().is_empty()>
            <div class=format!("w-dock-strip w-dock-strip-{}", side_name(side))>
                <For each=minimized key=|id| *id let:id>
                    <button
                        type="button"
                        class="w-dock-strip-button"
                        on:click=move |_| docking.layout.update(|l| l.show(id))
                    >
                        {docking.title(id)}
                    </button>
                </For>
            </div>
        </Show>
    }
}

/// Targets shown over the viewers while a tab is dragged: dropping on one moves the dockable to
/// that side.
#[component]
pub fn DropZones(docking: Docking) -> impl IntoView {
    let zones = DockSide::ALL.map(|side| {
        view! {
            <div
                class=format!("w-drop-zone w-drop-zone-{}", side_name(side))
                aria-label=format!("Dock {}", side_name(side))
                on:dragover=move |event| event.prevent_default()
                on:drop=move |event| {
                    event.prevent_default();
                    if let Some(id) = docking.dragging.get_untracked() {
                        docking.layout.update(|l| l.move_to(id, side));
                    }
                    docking.dragging.set(None);
                }
            ></div>
        }
    });
    view! { <Show when=move || docking.dragging.get().is_some()>{zones.clone()}</Show> }
}

fn side_name(side: DockSide) -> &'static str {
    match side {
        DockSide::Left => "left",
        DockSide::Right => "right",
        DockSide::Bottom => "bottom",
    }
}

/// The saved layout; none when storage is unavailable (private mode) or the value is invalid.
fn load_layout() -> Option<DockLayout> {
    let text = window()
        .local_storage()
        .ok()??
        .get_item(STORAGE_KEY)
        .ok()??;
    serde_json::from_str(&text).ok()
}

fn save_layout(layout: &DockLayout) {
    let storage = window().local_storage().ok().flatten();
    if let (Some(storage), Ok(text)) = (storage, serde_json::to_string(layout)) {
        // Storage may be full or blocked; the layout then simply is not kept.
        let _ = storage.set_item(STORAGE_KEY, &text);
    }
}
