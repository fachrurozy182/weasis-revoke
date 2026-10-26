//! A page showing every base component, opened with `#gallery`. It is the fixture of the
//! end-to-end tests of the components and a quick visual check of the theme.

use leptos::prelude::*;

use crate::shell::{Viewer, ViewerArea, Viewers};
use crate::widgets::{
    Button, ButtonKind, CollapsiblePanel, Dialog, DropDownButton, MenuChoice, Slider, ToggleButton,
    Toolbar, ToolbarSeparator,
};

/// The component gallery.
#[component]
pub fn Gallery() -> impl IntoView {
    let pressed = RwSignal::new(0);
    let selected = RwSignal::new(false);
    let layout = RwSignal::new("1x1".to_owned());
    let zoom = RwSignal::new(100);
    let dialog = RwSignal::new(false);
    let viewers = Viewers::new();
    let opened = StoredValue::new(0);
    let open_viewer = Callback::new(move |()| {
        opened.update_value(|n| *n += 1);
        let title = format!("Viewer {}", opened.get_value());
        let text = title.clone();
        viewers.open(Viewer {
            title,
            tooltip: None,
            content: ViewFn::from(move || view! { <p>{text.clone()}</p> }),
        });
    });
    let layouts = ["1x1", "1x2", "2x2"]
        .into_iter()
        .map(MenuChoice::new)
        .collect();
    view! {
        <div class="gallery">
            <Toolbar name="Gallery toolbar">
                <Button
                    label="Press"
                    kind=ButtonKind::Toolbar
                    on_click=Callback::new(move |()| pressed.update(|n| *n += 1))
                />
                <ToggleButton label="Toggle" selected=selected kind=ButtonKind::Toolbar />
                <ToolbarSeparator />
                <DropDownButton choices=layouts selected=layout title="Layout" />
            </Toolbar>
            <p data-testid="state">
                {move || format!("pressed {} | toggle {} | layout {}", pressed.get(), selected.get(), layout.get())}
            </p>
            <div class="gallery-row">
                <Button label="Standard" />
                <Button label="Primary" kind=ButtonKind::Primary />
                <Button label="Disabled" disabled=true />
                <Button label="Open dialog" on_click=Callback::new(move |()| dialog.set(true)) />
            </div>
            <div class="gallery-panel">
                <CollapsiblePanel title="Zoom">
                    <Slider name="Zoom" min=25 max=400 value=zoom />
                </CollapsiblePanel>
            </div>
            <div class="gallery-viewers">
                <div class="gallery-row">
                    <Button label="Open viewer" on_click=open_viewer />
                </div>
                <ViewerArea viewers=viewers />
            </div>
            <Dialog title="About Weasis" open=dialog>
                <p>"A modal dialog."</p>
                <div class="w-dialog-buttons">
                    <Button label="OK" kind=ButtonKind::Primary on_click=Callback::new(move |()| dialog.set(false)) />
                </div>
            </Dialog>
        </div>
    }
}
