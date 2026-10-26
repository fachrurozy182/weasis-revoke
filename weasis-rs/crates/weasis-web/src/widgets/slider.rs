//! A slider whose title shows its value.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/gui/util/JSliderW.java` and
//! `SliderChangeListener` (title "name: value").

use leptos::prelude::*;

/// A horizontal slider with a title and optional labels under the track.
#[component]
pub fn Slider(
    /// Name shown in the title.
    #[prop(into)]
    name: String,
    /// Smallest value.
    min: i32,
    /// Largest value.
    max: i32,
    /// Current value, read and written by the slider.
    value: RwSignal<i32>,
    /// Number of intervals between labels; 0 hides the labels.
    #[prop(default = 4)]
    label_division: i32,
) -> impl IntoView {
    let title = {
        let name = name.clone();
        move || format!("{name}: {}", value.get())
    };
    let labels = (label_division > 0).then(|| {
        (0..=label_division)
            .map(|i| min + (max - min) * i / label_division)
            .map(|v| view! { <span>{v}</span> })
            .collect_view()
    });
    view! {
        <div class="w-slider">
            <div class="w-slider-title">{title}</div>
            <input
                type="range"
                min=min
                max=max
                aria-label=name
                prop:value=move || value.get().to_string()
                on:input=move |event| {
                    if let Ok(v) = event_target_value(&event).parse() {
                        value.set(v);
                    }
                }
            />
            <div class="w-slider-labels small">{labels}</div>
        </div>
    }
}
