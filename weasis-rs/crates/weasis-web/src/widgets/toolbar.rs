//! A row of tool buttons.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/util/WtoolBar.java`.

use leptos::prelude::*;

/// A horizontal toolbar; it starts with a separator, like `WtoolBar`.
#[component]
pub fn Toolbar(
    /// Name of the toolbar (shown in the toolbar menu).
    #[prop(into)]
    name: String,
    /// Buttons and separators.
    children: Children,
) -> impl IntoView {
    view! {
        <div class="w-toolbar" role="toolbar" aria-label=name>
            <ToolbarSeparator />
            {children()}
        </div>
    }
}

/// A vertical line between groups of toolbar buttons.
#[component]
pub fn ToolbarSeparator() -> impl IntoView {
    view! { <span class="w-toolbar-separator" role="separator"></span> }
}
