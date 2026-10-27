//! Context menu of a series in the explorer: select the related series, remove the selected
//! series, or the study or patient of a single selected series, rebuild its thumbnail.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/ThumbnailMouseAndKeyAdapter.java`
//! (`showContextMenu`).

use leptos::prelude::*;
use weasis_core::explorer::MediaPosition;
use weasis_core::explorer::{SeriesSelection, related_series};

use super::Importer;
use super::dicom_explorer::image_count;

use crate::i18n::tr;

/// Series whose menu is open, and where.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesMenuAt {
    /// Key of the series.
    pub series: String,
    /// Pointer position in the page.
    pub x: i32,
    /// Pointer position in the page.
    pub y: i32,
}

/// The menu, shown while `at` is set.
// TODO(2.5.d, 2.6, 2.7): merge and phases, open and add, DICOM information.
#[component]
pub fn SeriesMenu(
    importer: Importer,
    selection: RwSignal<SeriesSelection>,
    at: RwSignal<Option<SeriesMenuAt>>,
) -> impl IntoView {
    move || {
        at.get().map(|SeriesMenuAt { series, x, y }| {
            let dismiss = move || at.set(None);
            let model = importer.model;
            let single = selection.with_untracked(|s| s.selected().len() == 1);
            // Rebuilding the thumbnail is offered for a series of several images.
            let several_images = model.with_untracked(|m| {
                m.series(&series).is_some_and(|s| image_count(s) > 1)
            });
            let item = |key: &'static str, action: Box<dyn Fn() + Send + Sync>| {
                view! {
                    <li role="menuitem" class="w-menu-item" on:click=move |_| {
                        dismiss();
                        action();
                    }>
                        {tr(key)}
                    </li>
                }
            };
            let related = |same_axis: bool| {
                let series = series.clone();
                Box::new(move || {
                    let keys = model.with_untracked(|m| {
                        let study = m.study_of_series(&series)?;
                        let node = study.series.iter().find(|s| s.key == series)?;
                        Some(related_series(study, node, same_axis))
                    });
                    if let Some(keys) = keys.filter(|k| !k.is_empty()) {
                        selection.update(|s| s.set(keys));
                    }
                }) as Box<dyn Fn() + Send + Sync>
            };
            let remove_series = Box::new(move || {
                let keys = selection.with_untracked(|s| s.selected().to_vec());
                model.update(|m| keys.iter().for_each(|k| _ = m.remove_series(k)));
                selection.update(SeriesSelection::clear);
            });
            let remove_study = {
                let series = series.clone();
                Box::new(move || {
                    let study = model.with_untracked(|m| m.study_of_series(&series).map(|s| s.uid.clone()));
                    if let Some(study) = study {
                        model.update(|m| _ = m.remove_study(&study));
                    }
                    selection.update(SeriesSelection::clear);
                })
            };
            let remove_patient = {
                let series = series.clone();
                Box::new(move || {
                    let patient = model.with_untracked(|m| {
                        let study = m.study_of_series(&series)?;
                        m.patient_of_study(&study.uid).map(|p| p.key.clone())
                    });
                    if let Some(patient) = patient {
                        model.update(|m| _ = m.remove_patient(&patient));
                    }
                    selection.update(SeriesSelection::clear);
                })
            };
            view! {
                <div class="w-backdrop" on:click=move |_| dismiss() on:contextmenu=move |e| {
                    e.prevent_default();
                    dismiss();
                }></div>
                <ul class="w-menu w-context-menu" role="menu" style=format!("left: {x}px; top: {y}px")>
                    {single.then(|| view! {
                        {item("DicomExplorer.sel_rel_series", related(false))}
                        {item("DicomExplorer.sel_rel_series_axis", related(true))}
                        <li role="separator" class="w-menu-separator"></li>
                    })}
                    {item("DicomExplorer.rem_series", remove_series)}
                    {single.then(|| view! {
                        {item("DicomExplorer.rem_study", remove_study)}
                        {item("DicomExplorer.rem_pat", remove_patient)}
                    })}
                    {(single && several_images).then(|| {
                        let rebuild = |key: &'static str, position: MediaPosition| {
                            let series = series.clone();
                            item(key, Box::new(move || importer.rebuild_thumbnail(&series, position)))
                        };
                        view! {
                            <li role="separator" class="w-menu-separator"></li>
                            <li role="menuitem" class="w-menu-item w-submenu" aria-haspopup="menu">
                                <span>{tr("DicomExplorer.build_thumb")}</span>
                                <span class="w-submenu-arrow" aria-hidden="true"></span>
                                <ul class="w-menu" role="menu">
                                    {rebuild("DicomExplorer.from_1", MediaPosition::First)}
                                    {rebuild("DicomExplorer.from_mid", MediaPosition::Middle)}
                                    {rebuild("DicomExplorer.from_last", MediaPosition::Last)}
                                </ul>
                            </li>
                        }
                    })}
                </ul>
            }
        })
    }
}
