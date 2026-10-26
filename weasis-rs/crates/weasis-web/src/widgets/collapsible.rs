//! A titled section that can be folded.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/CollapsiblePanel.java`.

use leptos::prelude::*;

/// A section with a full-width header button that shows or hides its content.
#[component]
pub fn CollapsiblePanel(
    /// Title of the section.
    #[prop(into)]
    title: String,
    /// Whether the content is shown at first.
    #[prop(default = true)]
    expanded: bool,
    /// Content of the section.
    children: Children,
) -> impl IntoView {
    let expanded = RwSignal::new(expanded);
    view! {
        <section class="w-collapsible">
            <button
                type="button"
                class="w-collapsible-header"
                aria-expanded=move || expanded.get().to_string()
                on:click=move |_| expanded.update(|e| *e = !*e)
            >
                <span class="w-disclosure" class:w-open=move || expanded.get() aria-hidden="true"></span>
                {title}
            </button>
            <div class="w-collapsible-content" class:w-hidden=move || !expanded.get()>
                {children()}
            </div>
        </section>
    }
}
