//! A combo box whose list is filtered by the text typed in it.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/util/SearchableComboBox.java`
//! (an editable `JComboBox` filtering its items).

use leptos::prelude::*;

use super::MenuChoice;

/// The combo box: `selected` holds the value of the chosen entry.
#[component]
pub fn SearchableComboBox(
    /// Entries, in order.
    #[prop(into)]
    choices: Signal<Vec<MenuChoice>>,
    /// Value of the selected entry.
    selected: RwSignal<String>,
    /// Accessible name.
    #[prop(into)]
    label: String,
    /// Text shown while the field is empty.
    #[prop(optional, into)]
    placeholder: String,
) -> impl IntoView {
    // Text typed by the user; `None` while the field shows the selected entry.
    let typed = RwSignal::new(None::<String>);
    let open = RwSignal::new(false);
    let highlighted = RwSignal::new(0usize);
    let selected_label = move || {
        let value = selected.get();
        choices.with(|c| c.iter().find(|c| c.value == value).map(|c| c.label.clone()))
    };
    let shown = move || {
        let filter = typed.get().unwrap_or_default().to_lowercase();
        choices.with(|c| {
            c.iter()
                .filter(|c| c.label.to_lowercase().contains(&filter))
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let choose = move |choice: &MenuChoice| {
        selected.set(choice.value.clone());
        typed.set(None);
        open.set(false);
    };
    let on_key = move |event: leptos::ev::KeyboardEvent| {
        let count = shown().len();
        match event.key().as_str() {
            "ArrowDown" if count > 0 => {
                event.prevent_default();
                open.set(true);
                highlighted.update(|h| *h = (*h + 1).min(count - 1));
            }
            "ArrowUp" => {
                event.prevent_default();
                highlighted.update(|h| *h = h.saturating_sub(1));
            }
            "Enter" => {
                if let Some(choice) = shown().get(highlighted.get_untracked()) {
                    choose(choice);
                }
            }
            "Escape" => {
                typed.set(None);
                open.set(false);
            }
            _ => {}
        }
    };
    view! {
        <div class="w-combobox">
            <input
                type="text"
                role="combobox"
                aria-label=label.clone()
                aria-expanded=move || open.get().to_string()
                placeholder=placeholder
                prop:value=move || typed.get().or_else(selected_label).unwrap_or_default()
                on:focus=move |_| open.set(true)
                on:blur=move |_| {
                    typed.set(None);
                    open.set(false);
                }
                on:input=move |e| {
                    typed.set(Some(event_target_value(&e)));
                    highlighted.set(0);
                    open.set(true);
                }
                on:keydown=on_key
            />
            <Show when=move || open.get() && !shown().is_empty()>
                <ul class="w-menu w-combobox-list" role="listbox" aria-label=label.clone()>
                    {move || {
                        shown()
                            .into_iter()
                            .enumerate()
                            .map(|(index, choice)| {
                                let is_selected = choice.value == selected.get();
                                let text = choice.label.clone();
                                view! {
                                    <li
                                        role="option"
                                        class="w-menu-item"
                                        class:w-highlighted=move || highlighted.get() == index
                                        aria-selected=is_selected.to_string()
                                        // Before the blur of the field, which closes the list.
                                        on:mousedown=move |e| {
                                            e.prevent_default();
                                            choose(&choice);
                                        }
                                    >
                                        {text}
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </Show>
        </div>
    }
}
