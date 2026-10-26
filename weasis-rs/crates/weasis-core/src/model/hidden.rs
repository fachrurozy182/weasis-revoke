//! Objects that are not shown as series but through the images they reference: Key Objects
//! (including rejection notes) and Presentation States, kept per patient.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/HiddenSeriesManager.java`,
//! `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomModel.java`
//! (`getKoSpecialElements`, `getRejectionKoSpecialElement`, `getPrSpecialElements`) and
//! `AbstractKOSpecialElement.getKoSpecialElements`.

use crate::dicom::KeyObject;
use crate::image::PresentationState;

/// Hidden objects of the loaded patients.
#[derive(Debug, Clone, Default)]
pub struct HiddenObjects {
    key_objects: Vec<(String, KeyObject)>,
    presentation_states: Vec<(String, PresentationState)>,
}

impl HiddenObjects {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a Key Object of a patient (pseudo UID); a document already known is replaced.
    pub fn add_key_object(&mut self, patient_key: &str, key_object: KeyObject) {
        self.key_objects
            .retain(|(_, ko)| ko.sop_instance_uid != key_object.sop_instance_uid);
        self.key_objects.push((patient_key.to_owned(), key_object));
    }

    /// Adds a Presentation State of a patient.
    pub fn add_presentation_state(&mut self, patient_key: &str, state: PresentationState) {
        self.presentation_states
            .push((patient_key.to_owned(), state));
    }

    /// Forgets the objects of a patient (when the patient is closed).
    pub fn remove_patient(&mut self, patient_key: &str) {
        self.key_objects.retain(|(key, _)| key != patient_key);
        self.presentation_states
            .retain(|(key, _)| key != patient_key);
    }

    /// Key image documents (not rejections) of a patient that reference a series, or all of
    /// them when `series` is `None`, newest first.
    pub fn key_objects(&self, patient_key: &str, series: Option<&str>) -> Vec<&KeyObject> {
        self.documents(patient_key, series, false)
    }

    /// Rejection notes of a patient that reference a series (or all), newest first.
    pub fn rejections(&self, patient_key: &str, series: Option<&str>) -> Vec<&KeyObject> {
        self.documents(patient_key, series, true)
    }

    /// The most recent rejection note of a frame (1-based) of an image, if it is rejected.
    pub fn rejection_for(
        &self,
        patient_key: &str,
        series: &str,
        sop: &str,
        frame: Option<i32>,
    ) -> Option<&KeyObject> {
        self.rejections(patient_key, Some(series))
            .into_iter()
            .find(|ko| ko.contains(series, sop, frame))
    }

    /// Presentation States that apply to a frame (1-based) of an image, newest first.
    pub fn presentation_states_for(
        &self,
        patient_key: &str,
        series: &str,
        sop: &str,
        frame: i32,
    ) -> Vec<&PresentationState> {
        let mut states: Vec<&PresentationState> = self
            .presentation_states
            .iter()
            .filter(|(key, pr)| key == patient_key && pr.applies_to(series, sop, frame))
            .map(|(_, pr)| pr)
            .collect();
        states.sort_by(|a, b| a.order.newest_first(&b.order));
        states
    }

    fn documents(
        &self,
        patient_key: &str,
        series: Option<&str>,
        rejections: bool,
    ) -> Vec<&KeyObject> {
        let mut documents: Vec<&KeyObject> = self
            .key_objects
            .iter()
            .filter(|(key, ko)| {
                key == patient_key
                    && ko.is_rejection == rejections
                    && series.is_none_or(|s| ko.references_series(s))
            })
            .map(|(_, ko)| ko)
            .collect();
        documents.sort_by(|a, b| a.order.newest_first(&b.order));
        documents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::{ContentOrder, KeyImage};

    fn document(uid: &str, date: &str, rejection: bool, sop: &str) -> KeyObject {
        KeyObject {
            sop_instance_uid: uid.to_owned(),
            series_instance_uid: "9".to_owned(),
            label: uid.to_owned(),
            title: None,
            is_rejection: rejection,
            order: ContentOrder {
                date_time: Some(date.to_owned()),
                series_number: None,
            },
            images: vec![KeyImage {
                study_instance_uid: "1".to_owned(),
                series_instance_uid: "1.1".to_owned(),
                sop_instance_uid: sop.to_owned(),
                sop_class_uid: None,
                frames: Vec::new(),
            }],
        }
    }

    #[test]
    fn documents_are_listed_newest_first_per_kind() {
        let mut hidden = HiddenObjects::new();
        hidden.add_key_object("P", document("old", "20250101000000", false, "a"));
        hidden.add_key_object("P", document("new", "20260101000000", false, "a"));
        hidden.add_key_object("P", document("reject", "20260101000000", true, "b"));
        hidden.add_key_object("Q", document("other", "20270101000000", false, "a"));

        let labels: Vec<&str> = hidden
            .key_objects("P", Some("1.1"))
            .iter()
            .map(|k| k.label.as_str())
            .collect();
        assert_eq!(labels, ["new", "old"]);
        assert!(hidden.key_objects("P", Some("2.2")).is_empty());
        assert_eq!(
            hidden
                .rejection_for("P", "1.1", "b", None)
                .map(|k| k.label.as_str()),
            Some("reject")
        );
        assert_eq!(hidden.rejection_for("P", "1.1", "a", None), None);

        hidden.remove_patient("P");
        assert!(hidden.key_objects("P", None).is_empty());
        assert_eq!(hidden.key_objects("Q", None).len(), 1);
    }
}
