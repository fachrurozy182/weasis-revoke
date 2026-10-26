//! Menu bar: menus of actions, check items and submenus, opened by a click and switched by
//! hovering while one is open, like a Swing `JMenuBar`.
//!
//! Java reference: Swing `JMenuBar`, `JMenu`, `JMenuItem`, `JCheckBoxMenuItem` and
//! `weasis-core/src/main/java/org/weasis/core/api/gui/util/DynamicMenu.java`.

use leptos::prelude::*;

/// An entry of a menu.
#[derive(Clone)]
pub enum MenuItem {
    /// Runs the action `id`.
    Action {
        /// Action identifier given to the menu bar handler.
        id: &'static str,
        /// Text.
        label: String,
        /// Keyboard shortcut shown on the right, e.g. "Alt+P".
        accelerator: Option<&'static str>,
    },
    /// A check box; the handler receives its id after the state changed.
    Check {
        /// Action identifier.
        id: &'static str,
        /// Text.
        label: String,
        /// Tooltip.
        title: Option<String>,
        /// State.
        checked: RwSignal<bool>,
    },
    /// A submenu shown on hover; disabled while empty.
    Submenu {
        /// Text.
        label: String,
        /// Entries.
        items: Vec<MenuItem>,
    },
    /// A line between groups.
    Separator,
}

/// A menu of the bar.
#[derive(Clone)]
pub struct Menu {
    /// Title in the bar.
    pub label: String,
    /// Entries.
    pub items: Vec<MenuItem>,
}

/// The menu bar; `on_action` receives the id of the chosen action.
#[component]
pub fn MenuBar(menus: Vec<Menu>, on_action: Callback<&'static str>) -> impl IntoView {
    let open = RwSignal::new(None::<usize>);
    let close = move || open.set(None);
    let bar = menus
        .into_iter()
        .enumerate()
        .map(|(index, menu)| {
            let is_open = move || open.get() == Some(index);
            view! {
                <li class="w-menubar-entry" class:w-open=is_open>
                    <button
                        type="button"
                        class="w-menubar-title"
                        aria-haspopup="menu"
                        aria-expanded=move || is_open().to_string()
                        on:click=move |_| open.update(|o| *o = if *o == Some(index) { None } else { Some(index) })
                        on:mouseenter=move |_| {
                            if open.get().is_some() {
                                open.set(Some(index));
                            }
                        }
                    >
                        {menu.label.clone()}
                    </button>
                    <Show when=is_open>
                        <MenuList items=menu.items.clone() label=menu.label.clone() on_action=on_action close=Callback::new(move |()| close()) />
                    </Show>
                </li>
            }
        })
        .collect_view();
    view! {
        <Show when=move || open.get().is_some()>
            <div class="w-backdrop" on:click=move |_| close()></div>
        </Show>
        <ul class="w-menubar" role="menubar" on:keydown=move |event| {
            if event.key() == "Escape" {
                close();
            }
        }>
            {bar}
        </ul>
    }
}

/// The entries of an open menu or submenu.
#[component]
fn MenuList(
    items: Vec<MenuItem>,
    label: String,
    on_action: Callback<&'static str>,
    close: Callback<()>,
) -> impl IntoView {
    let entries = items
        .into_iter()
        .map(|item| match item {
            MenuItem::Action { id, label, accelerator } => view! {
                <li
                    role="menuitem"
                    class="w-menu-item"
                    on:click=move |_| {
                        close.run(());
                        on_action.run(id);
                    }
                >
                    <span>{label}</span>
                    <span class="w-accelerator">{accelerator}</span>
                </li>
            }
            .into_any(),
            MenuItem::Check { id, label, title, checked } => view! {
                <li
                    role="menuitemcheckbox"
                    class="w-menu-item w-check-item"
                    title=title
                    aria-checked=move || checked.get().to_string()
                    on:click=move |_| {
                        checked.update(|c| *c = !*c);
                        close.run(());
                        on_action.run(id);
                    }
                >
                    {label}
                </li>
            }
            .into_any(),
            MenuItem::Submenu { label, items } => {
                let empty = items.is_empty();
                view! {
                    <li role="menuitem" class="w-menu-item w-submenu" aria-haspopup="menu" aria-disabled=empty.to_string()>
                        <span>{label.clone()}</span>
                        <span class="w-submenu-arrow" aria-hidden="true"></span>
                        {(!empty).then(|| view! { <MenuList items=items label=label on_action=on_action close=close /> })}
                    </li>
                }
                .into_any()
            }
            MenuItem::Separator => view! { <li role="separator" class="w-menu-separator"></li> }.into_any(),
        })
        .collect_view();
    view! {
        <ul class="w-menu" role="menu" aria-label=label>
            {entries}
        </ul>
    }
}
