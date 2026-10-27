//! Content of the DICOM Explorer panel: the running imports, the patient selector, then the
//! studies of the selected patient, each with its series, which are selected with the mouse and
//! the keyboard.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/DicomExplorer.java`,
//! `PatientPane.java`, `StudyPane.java`, `SeriesPane.java` and `ThumbnailMouseAndKeyAdapter.java`.

use dicom_dictionary_std::tags;
use leptos::prelude::*;
use weasis_core::dicom::ObjectKind;
use weasis_core::explorer::SeriesSelection;
use weasis_core::gui::{ShortcutTable, id};
use weasis_core::model::{
    DicomModel, SeriesNode, SortingTime, StudyNode, compare_patients, compare_series,
    compare_studies, patient_label, study_label,
};

use super::Importer;
use super::loading_task::LoadingTaskPanel;
use super::series_menu::{SeriesMenu, SeriesMenuAt};
use super::thumbnails::Thumbnails;
use crate::i18n::tr;
use crate::shortcuts::{Shortcuts, event_binding};
use crate::widgets::{MenuChoice, SearchableComboBox};

/// Width of a thumbnail in pixels (`Thumbnail.DEFAULT_SIZE`).
// TODO(7.6): the size chosen in the preferences.
const THUMBNAIL_SIZE: u32 = 144;

/// What an explorer shortcut does to the selection.
#[derive(Debug, Clone, Copy)]
enum SelectionKey {
    Open,
    Next,
    Previous,
    First,
    Last,
    All,
}

/// State shared by the panes of the explorer.
#[derive(Clone, Copy)]
struct ExplorerState {
    selection: RwSignal<SeriesSelection>,
    /// Keys of the shown series, in screen order.
    shown: Memo<Vec<String>>,
    menu: RwSignal<Option<SeriesMenuAt>>,
    thumbnails: Thumbnails,
}

/// The panel.
// TODO(2.5.e): identity masking of the labels. TODO(3.13): the key object button.
#[component]
pub fn DicomExplorer(importer: Importer) -> impl IntoView {
    let model = importer.model;
    let patient = RwSignal::new(String::new());
    let patients = Signal::derive(move || model.with(patient_choices));
    // The first patient loaded is shown, and another one when the shown patient goes away.
    Effect::new(move |_| {
        let choices = patients.get();
        let current = patient.get_untracked();
        if !choices.iter().any(|c| c.value == current)
            && let Some(first) = choices.first()
        {
            patient.set(first.value.clone());
        }
    });
    let studies = Memo::new(move |_| model.with(|m| sorted_studies(m, &patient.get())));
    let shown = Memo::new(move |_| {
        studies.with(|s| {
            s.iter()
                .flat_map(|study| study.series.iter().map(|series| series.key.clone()))
                .collect()
        })
    });
    let state = ExplorerState {
        selection: RwSignal::new(SeriesSelection::default()),
        shown,
        menu: RwSignal::new(None),
        thumbnails: importer.thumbnails,
    };
    // Removed or hidden series leave the selection.
    Effect::new(move |_| shown.with(|s| state.selection.update(|sel| sel.retain_shown(s))));
    let on_key = selection_keys(state);
    view! {
        <div class="w-dicom-explorer">
            <For each=move || importer.tasks.get() key=|task| (task.id, task.aborting) let:task>
                <LoadingTaskPanel importer=importer task=task />
            </For>
            <SearchableComboBox
                choices=patients
                selected=patient
                label=tr("DicomExplorer.search_patient")
                placeholder=tr("DicomExplorer.search_patient")
            />
            <div
                class="w-patient-pane"
                tabindex="0"
                role="listbox"
                aria-multiselectable="true"
                aria-label=tr("DicomExplorer.title")
                on:keydown=on_key
            >
                // Keyed by the series too, so that a study shows the series added to it.
                <For each=move || studies.get() key=study_key let:study>
                    <StudyPane study=study state=state />
                </For>
            </div>
            <SeriesMenu importer=importer selection=state.selection at=state.menu />
        </div>
    }
}

/// The explorer shortcuts acting on the selection.
fn selection_keys(state: ExplorerState) -> impl Fn(leptos::ev::KeyboardEvent) {
    let table = ShortcutTable::default()
        .with(id::EXPLORER_OPEN, SelectionKey::Open)
        .with(id::EXPLORER_SELECT_NEXT, SelectionKey::Next)
        .with(id::EXPLORER_SELECT_PREVIOUS, SelectionKey::Previous)
        .with(id::EXPLORER_SELECT_LAST, SelectionKey::Last)
        .with(id::EXPLORER_SELECT_LAST_ALT, SelectionKey::Last)
        .with(id::EXPLORER_SELECT_FIRST, SelectionKey::First)
        .with(id::EXPLORER_SELECT_FIRST_ALT, SelectionKey::First)
        .with(id::EXPLORER_SELECT_ALL, SelectionKey::All);
    let shortcuts = use_context::<Shortcuts>();
    move |event| {
        let Some(shortcuts) = shortcuts else {
            return;
        };
        let (key_code, modifiers) = event_binding(&event);
        let key = shortcuts
            .manager
            .with_untracked(|m| table.find(m, key_code, modifiers).copied());
        let Some(key) = key else {
            return;
        };
        event.prevent_default();
        let shown = state.shown.get_untracked();
        state.selection.update(|selection| match key {
            // TODO(2.6): open the selected series in the default viewer.
            SelectionKey::Open => {}
            SelectionKey::Next => selection.select_next(&shown),
            SelectionKey::Previous => selection.select_previous(&shown),
            SelectionKey::First => selection.select_first(&shown),
            SelectionKey::Last => selection.select_last(&shown),
            SelectionKey::All => selection.select_all(&shown),
        });
    }
}

/// A study: its title, then its series.
#[component]
fn StudyPane(study: StudyNode, state: ExplorerState) -> impl IntoView {
    let title = study_label(&study.attributes, None);
    view! {
        <fieldset class="w-study-pane">
            <legend>{title}</legend>
            <div class="w-series-grid">
                {study.series.into_iter().map(|s| view! { <SeriesPane series=s state=state /> }).collect_view()}
            </div>
        </fieldset>
    }
}

/// A series: its thumbnail and description.
#[component]
fn SeriesPane(series: SeriesNode, state: ExplorerState) -> impl IntoView {
    let description = series
        .attributes
        .get(tags::SERIES_DESCRIPTION)
        .unwrap_or_default()
        .to_owned();
    let size = format!("{THUMBNAIL_SIZE}px");
    let tooltip = description.clone();
    let count = image_count(&series);
    let split = series.split_number.map(|n| format!("#{n}"));
    let kind = series
        .instances
        .first()
        .map_or(ObjectKind::Image, |i| i.kind);
    let key = series.key;
    let image = {
        let key = key.clone();
        move || state.thumbnails.images.with(|i| i.get(&key).cloned())
    };
    let selected = {
        let key = key.clone();
        move || state.selection.with(|s| s.contains(&key))
    };
    let on_press = {
        let key = key.clone();
        move |event: leptos::ev::MouseEvent| {
            if event.button() == 2 {
                state.selection.update(|s| s.add(&key));
                return;
            }
            let shown = state.shown.get_untracked();
            // The menu shortcut modifier: Cmd on macOS, Ctrl elsewhere.
            let toggle = event.ctrl_key() || event.meta_key();
            state
                .selection
                .update(|s| s.click(&key, &shown, toggle, event.shift_key()));
        }
    };
    let on_menu = {
        let key = key.clone();
        move |event: leptos::ev::MouseEvent| {
            event.prevent_default();
            state.menu.set(Some(SeriesMenuAt {
                series: key.clone(),
                x: event.client_x(),
                y: event.client_y(),
            }));
        }
    };
    view! {
        <div
            class="w-series-pane"
            role="option"
            aria-selected=move || selected().to_string()
            data-series=key
            on:mousedown=on_press
            on:contextmenu=on_menu
        >
            // The image, or the kind of series; the number of images and the split part over it.
            <div class="w-thumbnail" style:width=size.clone() style:height=size.clone()>
                {move || match image() {
                    Some(url) => view! { <img src=url alt="" /> }.into_any(),
                    None => view! { <span class="w-thumbnail-kind">{kind_label(kind)}</span> }.into_any(),
                }}
                <span class="w-thumbnail-count">{count}</span>
                <span class="w-thumbnail-split">{split}</span>
            </div>
            <span class="mini w-series-label" style:width=size title=tooltip>
                {description}
            </span>
        </div>
    }
}

/// Number of images of a series: the frames of its instances (`MediaSeries.size`).
pub(super) fn image_count(series: &SeriesNode) -> u32 {
    series
        .instances
        .iter()
        .map(|i| {
            let frames = i.attributes.get(tags::NUMBER_OF_FRAMES);
            frames.and_then(|n| n.trim().parse().ok()).unwrap_or(1)
        })
        .sum()
}

/// Kind of a series shown while it has no thumbnail image (`Thumbnail.buildThumbnail`).
fn kind_label(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Video => tr("Thumbnail.video"),
        ObjectKind::StructuredReport => tr("Thumbnail.dicom_sr"),
        ObjectKind::EncapsulatedDocument => tr("Thumbnail.pdf"),
        ObjectKind::Waveform => "ECG",
        ObjectKind::Audio => tr("Thumbnail.audio"),
        _ => tr("Thumbnail.img"),
    }
}

fn study_key(study: &StudyNode) -> (String, Vec<String>) {
    let series = study.series.iter().map(|s| s.key.clone()).collect();
    (study.uid.clone(), series)
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

/// Studies of a patient with their shown series, both sorted; studies without one are left out.
// TODO(7.6): the study order chosen in the preferences.
fn sorted_studies(model: &DicomModel, patient: &str) -> Vec<StudyNode> {
    let Some(patient) = model.patient(patient) else {
        return Vec::new();
    };
    let mut studies: Vec<StudyNode> = patient
        .studies
        .iter()
        .map(|study| {
            let mut study = study.clone();
            study.series.retain(is_shown);
            study.series.sort_by(compare_series);
            study
        })
        .filter(|study| !study.series.is_empty())
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
