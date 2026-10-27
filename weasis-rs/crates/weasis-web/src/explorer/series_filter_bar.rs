//! The series filter of the explorer: a mode button (full text, study date, modality), then the
//! criterion — text with suggestions, or the study to show — and, while the filter is active,
//! how many series it shows.
//!
//! NOTE: the desktop application waits 250 ms after typing before filtering, to spare relaying
//! out its Swing panes; here the series are filtered at each key.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/DicomExplorer.java`
//! (`setupFilterBar`, `syncFilterToState`, `buildModalitySuggestions`, `updateFilterIndicator`).

use std::collections::BTreeSet;

use dicom_dictionary_std::tags;
use leptos::prelude::*;
use weasis_core::explorer::{FilterMode, SeriesFilter};
use weasis_core::model::{StudyNode, study_label};

use crate::i18n::tr;
use crate::widgets::{DropDownButton, MenuChoice};

/// The bar. `studies` are the studies of the patient with all their shown series; `counts` is
/// (series passing the filter, all series).
#[component]
pub fn SeriesFilterBar(
    filter: RwSignal<SeriesFilter>,
    studies: Signal<Vec<StudyNode>>,
    counts: Signal<(usize, usize)>,
) -> impl IntoView {
    let modes = FilterMode::ALL
        .iter()
        .map(|&mode| MenuChoice {
            value: mode_value(mode).to_owned(),
            label: mode_title(mode).to_owned(),
        })
        .collect();
    let mode = RwSignal::new(mode_value(filter.get_untracked().mode()).to_owned());
    // The menu changes the mode; another patient brings its own filter and mode.
    Effect::new(move |_| {
        let chosen = mode.get();
        if mode_value(filter.with_untracked(SeriesFilter::mode)) != chosen {
            filter.update(|f| f.set_mode(mode_from(&chosen)));
        }
    });
    Effect::new(move |_| mode.set(mode_value(filter.with(SeriesFilter::mode)).to_owned()));
    let title = move || {
        tr("DicomExplorer.filter_mode").replace("{0}", mode_title(filter.with(SeriesFilter::mode)))
    };
    let status = move || {
        let (shown, total) = counts.get();
        tr("DicomExplorer.filter_status")
            .replace("{0}", &shown.to_string())
            .replace("{1}", &total.to_string())
    };
    let is_active = move || filter.with(SeriesFilter::is_active);
    view! {
        <div class="w-filter-bar" class:w-active=is_active>
            <span class="w-filter-mode" title=title>
                <DropDownButton choices=modes selected=mode />
            </span>
            {move || match filter.with(SeriesFilter::mode) {
                FilterMode::Date => view! { <StudyChoice filter=filter studies=studies /> }.into_any(),
                FilterMode::Text | FilterMode::Modality => view! { <TextCriterion filter=filter studies=studies /> }.into_any(),
            }}
        </div>
        <Show when=is_active>
            <div class="w-filter-status small">{status}</div>
        </Show>
    }
}

/// Text searched, or modalities shown, with suggestions from the series of the patient.
#[component]
fn TextCriterion(filter: RwSignal<SeriesFilter>, studies: Signal<Vec<StudyNode>>) -> impl IntoView {
    let mode = filter.with_untracked(SeriesFilter::mode);
    let initial = match mode {
        FilterMode::Modality => filter.with_untracked(|f| f.modalities().join(", ")),
        _ => filter.with_untracked(|f| f.text().to_owned()),
    };
    let placeholder = match mode {
        FilterMode::Modality => tr("DicomExplorer.filter_ph_modality"),
        _ => tr("DicomExplorer.filter_ph_text"),
    };
    let suggestions = move || {
        studies.with(|s| match mode {
            FilterMode::Modality => modality_suggestions(s),
            _ => text_suggestions(s),
        })
    };
    view! {
        <input
            type="search"
            class="w-filter-field"
            list="w-filter-suggestions"
            aria-label=placeholder
            placeholder=placeholder
            value=initial
            on:input=move |e| {
                let value = event_target_value(&e);
                filter.update(|f| match mode {
                    FilterMode::Modality => f.set_modalities(&value),
                    _ => f.set_text(&value),
                });
            }
        />
        <datalist id="w-filter-suggestions">
            {move || suggestions().into_iter().map(|s| view! { <option value=s></option> }).collect_view()}
        </datalist>
    }
}

/// The study shown, or all of them.
#[component]
fn StudyChoice(filter: RwSignal<SeriesFilter>, studies: Signal<Vec<StudyNode>>) -> impl IntoView {
    let selected = move || filter.with(|f| f.study().unwrap_or_default().to_owned());
    view! {
        <select
            class="w-filter-field"
            aria-label=tr("DicomExplorer.filter_ph_date")
            on:change=move |e| {
                let uid = event_target_value(&e);
                filter.update(|f| f.set_study((!uid.is_empty()).then_some(uid)));
            }
        >
            <option value="" selected=move || selected().is_empty()>{tr("DicomExplorer.sel_all_st")}</option>
            {move || {
                studies
                    .get()
                    .into_iter()
                    .map(|study| {
                        let uid = study.uid.clone();
                        let is_selected = move || selected() == uid;
                        view! {
                            <option value=study.uid.clone() selected=is_selected>
                                {study_label(&study.attributes, None)}
                            </option>
                        }
                    })
                    .collect_view()
            }}
        </select>
    }
}

/// Series and study descriptions.
fn text_suggestions(studies: &[StudyNode]) -> Vec<String> {
    let mut values = BTreeSet::new();
    for study in studies {
        values.extend(
            study
                .attributes
                .get(tags::STUDY_DESCRIPTION)
                .map(str::to_owned),
        );
        for series in &study.series {
            values.extend(
                series
                    .attributes
                    .get(tags::SERIES_DESCRIPTION)
                    .map(str::to_owned),
            );
        }
    }
    values
        .into_iter()
        .filter(|v| !v.trim().is_empty())
        .collect()
}

/// Each modality, then the combinations found in a study ("CT, PT").
fn modality_suggestions(studies: &[StudyNode]) -> Vec<String> {
    let per_study: Vec<BTreeSet<String>> = studies
        .iter()
        .map(|study| {
            study
                .series
                .iter()
                .filter_map(|s| s.attributes.get(tags::MODALITY))
                .filter(|m| !m.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect();
    let single: BTreeSet<String> = per_study.iter().flatten().cloned().collect();
    let combinations: BTreeSet<String> = per_study
        .iter()
        .filter(|m| m.len() > 1)
        .map(|m| m.iter().cloned().collect::<Vec<_>>().join(", "))
        .collect();
    single.into_iter().chain(combinations).collect()
}

fn mode_value(mode: FilterMode) -> &'static str {
    match mode {
        FilterMode::Text => "text",
        FilterMode::Date => "date",
        FilterMode::Modality => "modality",
    }
}

fn mode_from(value: &str) -> FilterMode {
    FilterMode::ALL
        .into_iter()
        .find(|&m| mode_value(m) == value)
        .unwrap_or_default()
}

fn mode_title(mode: FilterMode) -> &'static str {
    match mode {
        FilterMode::Text => tr("DicomExplorer.filter_mode_text"),
        FilterMode::Date => tr("DicomExplorer.filter_mode_date"),
        FilterMode::Modality => tr("DicomExplorer.filter_mode_modality"),
    }
}
