//! A running import in the explorer: a stop button, a busy indicator and its message.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/LoadingTaskPanel.java`.

use leptos::prelude::*;

use super::Importer;
use super::importer::LoadingTask;
use crate::i18n::tr;

/// The panel of one task.
#[component]
pub fn LoadingTaskPanel(importer: Importer, task: LoadingTask) -> impl IntoView {
    let message = if task.aborting {
        tr("LoadingTaskPanel.abording")
    } else {
        tr("DicomExplorer.loading")
    };
    view! {
        <div class="w-loading-task" role="status">
            <button
                type="button"
                class="w-loading-stop"
                title=tr("LoadingTaskPanel.stop_process")
                on:click=move |_| importer.cancel(task.id)
            ></button>
            <span class="w-spinner" aria-hidden="true"></span>
            <span class="micro">{message}</span>
        </div>
    }
}
