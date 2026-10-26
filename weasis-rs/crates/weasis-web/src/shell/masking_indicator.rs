//! Badge shown at the end of the menu bar while session masking is on, so that whoever looks at
//! the screen knows the identities are substitutes; a click offers to turn masking off.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/util/MaskingIndicator.java`.

use leptos::prelude::*;

use crate::i18n::tr;

/// Text of the badge, also the suffix of the window title.
// TODO(7.6): append the session profile when it is not the display profile.
pub fn masking_label() -> &'static str {
    tr("masking.indicator")
}

/// The badge; `masking` is the session masking switch.
#[component]
pub fn MaskingIndicator(masking: RwSignal<bool>) -> impl IntoView {
    let menu = RwSignal::new(false);
    view! {
        <Show when=move || masking.get()>
            <div class="w-indicator">
                <button
                    type="button"
                    class="w-masking-indicator small"
                    title=plain_text(tr("masking.indicator.tip"))
                    on:click=move |_| menu.set(true)
                >
                    {masking_label()}
                </button>
                <Show when=move || menu.get()>
                    <div class="w-backdrop" on:click=move |_| menu.set(false)></div>
                    <ul class="w-menu w-indicator-menu" role="menu">
                        <li role="menuitem" class="w-menu-item" on:click=move |_| {
                            menu.set(false);
                            masking.set(false);
                        }>
                            {tr("masking.indicator.off")}
                        </li>
                    </ul>
                </Show>
            </div>
        </Show>
    }
}

/// A Swing `<html>` text as plain text for a `title` attribute: `<br>` becomes a line break and
/// the other tags are dropped.
fn plain_text(html: &str) -> String {
    let html = html.replace("<br>", "\n");
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swing_html_becomes_plain_text() {
        assert_eq!(
            plain_text("<html>A <b>bold</b><br>line</html>"),
            "A bold\nline"
        );
    }
}
