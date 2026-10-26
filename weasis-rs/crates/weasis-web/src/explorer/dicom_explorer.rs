//! Content of the DICOM Explorer panel: the running imports, then the loaded patients and their
//! series.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomExplorer.java`.

use dicom_dictionary_std::tags;
use leptos::prelude::*;
use weasis_core::model::PatientNode;

use super::Importer;
use super::loading_task::LoadingTaskPanel;

/// The panel.
// TODO(2.5): the real explorer (patient selector, study and series panes with thumbnails).
#[component]
pub fn DicomExplorer(importer: Importer) -> impl IntoView {
    let patients = move || importer.model.with(|m| m.patients().to_vec());
    view! {
        <div class="w-dicom-explorer">
            <For each=move || importer.tasks.get() key=|task| (task.id, task.aborting) let:task>
                <LoadingTaskPanel importer=importer task=task />
            </For>
            <ul class="w-patient-list">
                <For each=patients key=|p| p.key.clone() let:patient>
                    <li>{summary(&patient)}</li>
                </For>
            </ul>
        </div>
    }
}

/// "Name — n series".
fn summary(patient: &PatientNode) -> String {
    let name = patient
        .attributes
        .get(tags::PATIENT_NAME)
        .unwrap_or_default();
    let series: usize = patient.studies.iter().map(|s| s.series.len()).sum();
    format!("{name} — {series} series")
}
