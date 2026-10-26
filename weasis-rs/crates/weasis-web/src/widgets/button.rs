//! Push and toggle buttons.
//!
//! Java reference: Swing `JButton` / `JToggleButton` with the FlatLaf button types
//! (`FlatClientProperties.BUTTON_TYPE`: default, borderless/toolbar).

use leptos::prelude::*;

/// Look of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonKind {
    /// Bordered button of dialogs and panels.
    #[default]
    Standard,
    /// The default button of a dialog (highlighted).
    Primary,
    /// Borderless button of toolbars, highlighted on hover.
    Toolbar,
}

impl ButtonKind {
    fn class(self) -> &'static str {
        match self {
            Self::Standard => "w-button",
            Self::Primary => "w-button w-button-primary",
            Self::Toolbar => "w-button w-button-toolbar",
        }
    }
}

/// A push button.
#[component]
pub fn Button(
    /// Text of the button.
    #[prop(into)]
    label: String,
    /// Look of the button.
    #[prop(optional)]
    kind: ButtonKind,
    /// Tooltip.
    #[prop(optional, into)]
    title: Option<String>,
    /// Whether the button can be pressed.
    #[prop(optional, into)]
    disabled: Signal<bool>,
    /// Action.
    #[prop(optional, into)]
    on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=kind.class()
            title=title
            disabled=move || disabled.get()
            on:click=move |_| {
                if let Some(action) = on_click {
                    action.run(());
                }
            }
        >
            {label}
        </button>
    }
}

/// A button that stays pressed (selected) until pressed again.
#[component]
pub fn ToggleButton(
    /// Text of the button.
    #[prop(into)]
    label: String,
    /// Selection state, read and written by the button.
    selected: RwSignal<bool>,
    /// Look of the button.
    #[prop(optional)]
    kind: ButtonKind,
    /// Tooltip.
    #[prop(optional, into)]
    title: Option<String>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=kind.class()
            class:w-selected=move || selected.get()
            aria-pressed=move || selected.get().to_string()
            title=title
            on:click=move |_| selected.update(|value| *value = !*value)
        >
            {label}
        </button>
    }
}
