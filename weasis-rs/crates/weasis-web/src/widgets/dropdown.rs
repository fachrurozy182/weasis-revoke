//! A button opening a menu of exclusive choices below it.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/DropDownButton.java`
//! and `GroupPopup` / `GroupRadioMenu`.

use leptos::prelude::*;

/// One entry of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuChoice {
    /// Value given back when chosen.
    pub value: String,
    /// Text shown.
    pub label: String,
}

impl MenuChoice {
    /// An entry whose value is its text.
    pub fn new(label: &str) -> Self {
        Self {
            value: label.to_owned(),
            label: label.to_owned(),
        }
    }
}

/// A button showing the current choice; the menu closes on a choice or a click outside.
#[component]
pub fn DropDownButton(
    /// Entries of the menu.
    choices: Vec<MenuChoice>,
    /// Value of the selected entry.
    selected: RwSignal<String>,
    /// Tooltip.
    #[prop(optional, into)]
    title: Option<String>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let label_choices = choices.clone();
    let label = move || {
        let value = selected.get();
        label_choices
            .iter()
            .find(|c| c.value == value)
            .map_or(value, |c| c.label.clone())
    };
    let items = choices
        .into_iter()
        .map(|choice| {
            let value = choice.value.clone();
            let is_selected = {
                let value = value.clone();
                move || selected.get() == value
            };
            view! {
                <li
                    role="menuitemradio"
                    class="w-menu-item"
                    aria-checked=move || is_selected().to_string()
                    on:click=move |_| {
                        selected.set(value.clone());
                        open.set(false);
                    }
                >
                    {choice.label}
                </li>
            }
        })
        .collect_view();
    view! {
        <div class="w-dropdown">
            <button
                type="button"
                class="w-button w-button-toolbar w-dropdown-button"
                class:w-selected=move || open.get()
                aria-haspopup="menu"
                aria-expanded=move || open.get().to_string()
                title=title
                on:click=move |_| open.update(|o| *o = !*o)
            >
                {label}
                <span class="w-dropdown-arrow" aria-hidden="true"></span>
            </button>
            <Show when=move || open.get()>
                <div class="w-backdrop" on:click=move |_| open.set(false)></div>
            </Show>
            <ul class="w-menu" role="menu" class:w-hidden=move || !open.get()>
                {items}
            </ul>
        </div>
    }
}
