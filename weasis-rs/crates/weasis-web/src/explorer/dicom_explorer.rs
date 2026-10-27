//! Content of the DICOM Explorer panel: the running imports, the patient selector, then the
//! studies of the selected patient, each with its series.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/DicomExplorer.java`,
//! `PatientPane.java`, `StudyPane.java` and `SeriesPane.java`.

use dicom_dictionary_std::tags;
use leptos::prelude::*;
use weasis_core::model::{
    DicomModel, SeriesNode, SortingTime, StudyNode, compare_patients, compare_series,
    compare_studies, patient_label, study_label,
};

use super::Importer;
use super::loading_task::LoadingTaskPanel;
use crate::i18n::tr;
use crate::widgets::{MenuChoice, SearchableComboBox};

/// Width of a thumbnail in pixels (`Thumbnail.DEFAULT_SIZE`).
// TODO(7.6): the size chosen in the preferences.
const THUMBNAIL_SIZE: u32 = 144;

/// The panel.
// TODO(2.5.e): identity masking of the labels. TODO(3.13): the key object button.
#[component]
pub fn DicomExplorer(importer: Importer) -> impl IntoView {
    let model = importer.model;
    let selected = RwSignal::new(String::new());
    let patients = Signal::derive(move || model.with(patient_choices));
    // The first patient loaded is shown, and another one when the shown patient goes away.
    Effect::new(move |_| {
        let choices = patients.get();
        let current = selected.get_untracked();
        if !choices.iter().any(|c| c.value == current)
            && let Some(first) = choices.first()
        {
            selected.set(first.value.clone());
        }
    });
    let studies = move || model.with(|m| sorted_studies(m, &selected.get()));
    view! {
        <div class="w-dicom-explorer">
            <For each=move || importer.tasks.get() key=|task| (task.id, task.aborting) let:task>
                <LoadingTaskPanel importer=importer task=task />
            </For>
            <SearchableComboBox
                choices=patients
                selected=selected
                label=tr("DicomExplorer.search_patient")
                placeholder=tr("DicomExplorer.search_patient")
            />
            <div class="w-patient-pane">
                <For each=studies key=|study| study.uid.clone() let:study>
                    <StudyPane study=study />
                </For>
            </div>
        </div>
    }
}

/// A study: its title, then its series.
#[component]
fn StudyPane(study: StudyNode) -> impl IntoView {
    let title = study_label(&study.attributes, None);
    let mut series: Vec<SeriesNode> = study.series.into_iter().filter(is_shown).collect();
    series.sort_by(compare_series);
    view! {
        <fieldset class="w-study-pane">
            <legend>{title}</legend>
            <div class="w-series-grid">
                {series.into_iter().map(|s| view! { <SeriesPane series=s /> }).collect_view()}
            </div>
        </fieldset>
    }
}

/// A series: its thumbnail and description.
#[component]
fn SeriesPane(series: SeriesNode) -> impl IntoView {
    let description = series
        .attributes
        .get(tags::SERIES_DESCRIPTION)
        .unwrap_or_default()
        .to_owned();
    let size = format!("{THUMBNAIL_SIZE}px");
    let tooltip = description.clone();
    view! {
        <div class="w-series-pane" data-series=series.key.clone()>
            // TODO(2.5.c): the thumbnail image.
            <div class="w-thumbnail" style:width=size.clone() style:height=size.clone()></div>
            <span class="mini w-series-label" style:width=size title=tooltip>
                {description}
            </span>
        </div>
    }
}

/// The patients, sorted, as choices of the selector.
fn patient_choices(model: &DicomModel) -> Vec<MenuChoice> {
    let mut patients: Vec<_> = model.patients().iter().collect();
    patients.sort_by(|a, b| compare_patients(a, b));
    patients
        .into_iter()
        .map(|p| MenuChoice {
            value: p.key.clone(),
            label: patient_label(&p.attributes, None),
        })
        .collect()
}

/// Studies of a patient that have a series to show, sorted.
// TODO(7.6): the study order chosen in the preferences.
fn sorted_studies(model: &DicomModel, patient: &str) -> Vec<StudyNode> {
    let Some(patient) = model.patient(patient) else {
        return Vec::new();
    };
    let mut studies: Vec<StudyNode> = patient
        .studies
        .iter()
        .filter(|s| s.series.iter().any(is_shown))
        .cloned()
        .collect();
    studies.sort_by(|a, b| compare_studies(a, b, SortingTime::default()));
    studies
}

/// Series of presentation states, key objects, segmentations and RT objects are used through the
/// images they reference and have no pane (`DicomModel.isHiddenModality`).
fn is_shown(series: &SeriesNode) -> bool {
    series
        .instances
        .first()
        .is_some_and(|i| !i.kind.is_hidden())
}
