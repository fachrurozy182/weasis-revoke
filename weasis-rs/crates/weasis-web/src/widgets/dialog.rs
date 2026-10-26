//! A modal dialog.
//!
//! Java reference: Swing `JDialog` / `JOptionPane` with the FlatLaf title pane.

use leptos::prelude::*;

/// A modal dialog with a title bar, content and a button row; shown while `open` is true.
#[component]
pub fn Dialog(
    /// Title.
    #[prop(into)]
    title: String,
    /// Whether the dialog is shown; set to false by the close button.
    open: RwSignal<bool>,
    /// Content, then the buttons.
    children: ChildrenFn,
) -> impl IntoView {
    let title = StoredValue::new(title);
    view! {
        <Show when=move || open.get()>
            <div class="w-modal-backdrop">
                <div class="w-dialog" role="dialog" aria-modal="true" aria-label=title.get_value()>
                    <div class="w-dialog-title">
                        <span>{title.get_value()}</span>
                        <button
                            type="button"
                            class="w-dialog-close"
                            aria-label="Close"
                            on:click=move |_| open.set(false)
                        >
                            "×"
                        </button>
                    </div>
                    <div class="w-dialog-body">{children()}</div>
                </div>
            </div>
        </Show>
    }
}
