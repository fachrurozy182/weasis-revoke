//! Messages shown once an import ends: files that could not be read, and valid files skipped
//! because no viewer shows them (which the user can choose not to be told again).
//!
//! NOTE: the desktop application links its log files from the error message; here the errors
//! are written to the browser console, so no link is shown.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadDicom.java`
//! (`done`, `getErrorPanel`, `getUnsupportedPanel`).

use std::collections::VecDeque;

use leptos::prelude::*;

use crate::i18n::tr;
use crate::storage;
use crate::widgets::{Button, ButtonKind, Dialog};

/// Preference: when false, skipped files are not reported.
const SHOW_UNSUPPORTED_SOP_CLASS_INFO: &str = "weasis.show.unsupported.sop.class.info";

/// A message of the end of an import.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportMessage {
    /// Number of files that could not be read.
    Errors(usize),
    /// Number of valid files that no viewer shows.
    Unsupported(u32),
}

/// Shows the messages one after the other.
#[component]
pub fn ImportReport(messages: RwSignal<VecDeque<ReportMessage>>) -> impl IntoView {
    let show_unsupported =
        move || storage::load::<bool>(SHOW_UNSUPPORTED_SOP_CLASS_INFO) != Some(false);
    // Skipped files are dropped at once when the user asked not to be told.
    Effect::new(move |_| {
        let first = messages.with(|m| m.front().copied());
        if matches!(first, Some(ReportMessage::Unsupported(_))) && !show_unsupported() {
            messages.update(|m| _ = m.pop_front());
        }
    });
    let open = RwSignal::new(false);
    Effect::new(move |_| open.set(messages.with(|m| !m.is_empty())));
    // Closing the dialog, by its button or its title bar, goes to the next message.
    Effect::new(move |previous: Option<bool>| {
        let is_open = open.get();
        if previous == Some(true) && !is_open {
            messages.update(|m| _ = m.pop_front());
        }
        is_open
    });
    let dont_show_again = RwSignal::new(false);
    let text = move || match messages.with(|m| m.front().copied()) {
        Some(ReportMessage::Errors(count)) => {
            tr("dicom.file.cannot.be.read").replace("%d", &count.to_string())
        }
        Some(ReportMessage::Unsupported(count)) => tr("dicom.file.unsupported.sop.class")
            .replace("%d", &count.to_string())
            .replace("<br>", "\n"),
        None => String::new(),
    };
    let is_unsupported = move || {
        matches!(
            messages.with(|m| m.front().copied()),
            Some(ReportMessage::Unsupported(_))
        )
    };
    view! {
        <Dialog title=tr("DicomImport.imp_dicom") open=open>
            <p class="w-report-message">{text}</p>
            <Show when=is_unsupported>
                <label class="w-check">
                    <input type="checkbox" prop:checked=dont_show_again on:change=move |e| dont_show_again.set(event_target_checked(&e)) />
                    {tr("dicom.unsupported.dont.show.again")}
                </label>
            </Show>
            <div class="w-dialog-buttons">
                <Button label="OK" kind=ButtonKind::Primary on_click=Callback::new(move |()| {
                    if is_unsupported() && dont_show_again.get_untracked() {
                        storage::save(SHOW_UNSUPPORTED_SOP_CLASS_INFO, &false);
                    }
                    open.set(false);
                }) />
            </div>
        </Dialog>
    }
}
