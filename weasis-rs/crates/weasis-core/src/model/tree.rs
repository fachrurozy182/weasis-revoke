//! The model tree: patients contain studies, studies contain series, series contain instances.
//!
//! Each node is identified by a key: the patient pseudo UID (see [`PatientIdentity`]), the
//! Study Instance UID, the sub-series UID (the Series Instance UID, or `#<n>.<Series Instance UID>`
//! for the parts split off by [`SplittingRules`]) and the SOP Instance UID. Children keep their insertion order; sorting is
//! done by the explorer and the viewer.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomModel.java`
//! and `weasis-core/src/main/java/org/weasis/core/api/media/data/MediaSeriesGroupNode.java`.
//!
//! [`PatientIdentity`]: super::PatientIdentity
//! [`SplittingRules`]: super::SplittingRules

use super::Attributes;
use crate::dicom::ObjectKind;
use serde::{Deserialize, Serialize};

/// A patient node.
#[derive(Debug, Clone, PartialEq)]
pub struct PatientNode {
    /// Patient pseudo UID.
    pub key: String,
    /// Patient-level attributes.
    pub attributes: Attributes,
    /// Studies of this patient.
    pub studies: Vec<StudyNode>,
}

/// A study node.
#[derive(Debug, Clone, PartialEq)]
pub struct StudyNode {
    /// Study Instance UID.
    pub uid: String,
    /// Study-level attributes.
    pub attributes: Attributes,
    /// Series of this study.
    pub series: Vec<SeriesNode>,
}

/// A series node (a sub-series once series splitting is applied).
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesNode {
    /// Sub-series UID.
    pub key: String,
    /// Series-level attributes.
    pub attributes: Attributes,
    /// Number of the part once the series is split (1 for the original series).
    pub split_number: Option<u32>,
    /// Instances of this series.
    pub instances: Vec<InstanceNode>,
}

/// One DICOM object.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceNode {
    /// SOP Instance UID.
    pub sop_instance_uid: String,
    /// Kind of object.
    pub kind: ObjectKind,
    /// Instance-level attributes.
    pub attributes: Attributes,
}

/// An instance to add, with the keys and attributes of every level it belongs to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewInstance {
    /// Patient pseudo UID.
    pub patient_key: String,
    /// Study Instance UID.
    pub study_uid: String,
    /// Sub-series UID.
    pub series_key: String,
    /// SOP Instance UID.
    pub sop_instance_uid: String,
    /// Kind of object.
    pub kind: ObjectKind,
    /// Attributes for the patient node.
    pub patient: Attributes,
    /// Attributes for the study node.
    pub study: Attributes,
    /// Attributes for the series node.
    pub series: Attributes,
    /// Attributes of the instance itself.
    pub instance: Attributes,
}

/// The whole tree of loaded DICOM objects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DicomModel {
    patients: Vec<PatientNode>,
}

impl DicomModel {
    /// Creates an empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Patients in insertion order.
    pub fn patients(&self) -> &[PatientNode] {
        &self.patients
    }

    /// Adds an instance, creating its patient, study and series nodes when needed.
    ///
    /// Nodes that already exist keep their attributes; only missing values are filled in.
    /// When the study is already loaded under another patient (patient attributes that differ
    /// within one exam), the instance joins that patient, as in the Java viewer.
    /// Returns `false` when an instance with the same SOP Instance UID is already in the series.
    pub fn add_instance(&mut self, new: NewInstance) -> bool {
        let patient_key = self
            .patients
            .iter()
            .find(|p| p.studies.iter().any(|s| s.uid == new.study_uid))
            .map_or_else(|| new.patient_key.clone(), |p| p.key.clone());

        let patient = get_or_insert(
            &mut self.patients,
            |p| p.key == patient_key,
            || PatientNode {
                key: patient_key.clone(),
                attributes: Attributes::new(),
                studies: Vec::new(),
            },
        );
        patient.attributes.fill_missing(&new.patient);

        let study = get_or_insert(
            &mut patient.studies,
            |s| s.uid == new.study_uid,
            || StudyNode {
                uid: new.study_uid.clone(),
                attributes: Attributes::new(),
                series: Vec::new(),
            },
        );
        study.attributes.fill_missing(&new.study);

        let series = get_or_insert(
            &mut study.series,
            |s| s.key == new.series_key,
            || SeriesNode {
                key: new.series_key.clone(),
                attributes: Attributes::new(),
                split_number: None,
                instances: Vec::new(),
            },
        );
        series.attributes.fill_missing(&new.series);

        let is_duplicate = series
            .instances
            .iter()
            .any(|instance| instance.sop_instance_uid == new.sop_instance_uid);
        if !is_duplicate {
            series.instances.push(InstanceNode {
                sop_instance_uid: new.sop_instance_uid,
                kind: new.kind,
                attributes: new.instance,
            });
        }
        !is_duplicate
    }

    /// Finds a patient by its pseudo UID.
    pub fn patient(&self, key: &str) -> Option<&PatientNode> {
        self.patients.iter().find(|p| p.key == key)
    }

    /// Finds a study by its Study Instance UID, in any patient.
    pub fn study(&self, uid: &str) -> Option<&StudyNode> {
        self.patients
            .iter()
            .flat_map(|p| &p.studies)
            .find(|s| s.uid == uid)
    }

    /// Finds a series by its sub-series UID, in any study.
    pub fn series(&self, key: &str) -> Option<&SeriesNode> {
        self.patients
            .iter()
            .flat_map(|p| &p.studies)
            .flat_map(|s| &s.series)
            .find(|s| s.key == key)
    }

    /// The study holding a series.
    pub fn study_of_series(&self, key: &str) -> Option<&StudyNode> {
        self.patients
            .iter()
            .flat_map(|p| &p.studies)
            .find(|s| s.series.iter().any(|series| series.key == key))
    }

    /// The patient holding a study.
    pub fn patient_of_study(&self, uid: &str) -> Option<&PatientNode> {
        self.patients
            .iter()
            .find(|p| p.studies.iter().any(|s| s.uid == uid))
    }

    pub(super) fn study_of_series_mut(&mut self, key: &str) -> Option<&mut StudyNode> {
        self.patients
            .iter_mut()
            .flat_map(|p| &mut p.studies)
            .find(|s| s.series.iter().any(|series| series.key == key))
    }

    pub(super) fn series_mut(&mut self, key: &str) -> Option<&mut SeriesNode> {
        self.patients
            .iter_mut()
            .flat_map(|p| &mut p.studies)
            .flat_map(|s| &mut s.series)
            .find(|s| s.key == key)
    }

    /// Removes a patient with all its studies.
    pub fn remove_patient(&mut self, key: &str) -> Option<PatientNode> {
        take_first(&mut self.patients, |p| p.key == key)
    }

    /// Removes a study with all its series. The patient is kept, even if it becomes empty.
    pub fn remove_study(&mut self, uid: &str) -> Option<StudyNode> {
        self.patients
            .iter_mut()
            .find_map(|p| take_first(&mut p.studies, |s| s.uid == uid))
    }

    /// Removes a series with all its instances. The study is kept, even if it becomes empty.
    pub fn remove_series(&mut self, key: &str) -> Option<SeriesNode> {
        self.patients
            .iter_mut()
            .flat_map(|p| &mut p.studies)
            .find_map(|s| take_first(&mut s.series, |series| series.key == key))
    }

    /// Moves the studies of patient `old_key` to patient `new_key` and removes `old_key`.
    ///
    /// When `new_key` does not exist yet, the old patient is renamed. Does nothing when
    /// `old_key` is missing or both keys are equal.
    pub fn merge_patients(&mut self, old_key: &str, new_key: &str) {
        if old_key == new_key {
            return;
        }
        let Some(mut old) = self.remove_patient(old_key) else {
            return;
        };
        match self.patients.iter_mut().find(|p| p.key == new_key) {
            Some(target) => {
                target.attributes.fill_missing(&old.attributes);
                for study in old.studies {
                    merge_study_into(&mut target.studies, study);
                }
            }
            None => {
                old.key = new_key.to_owned();
                self.patients.push(old);
            }
        }
    }

    /// Moves the series of study `old_uid` to study `new_uid` and removes `old_uid`.
    ///
    /// When `new_uid` does not exist yet, the old study is renamed in place. Does nothing when
    /// `old_uid` is missing or both UIDs are equal.
    pub fn merge_studies(&mut self, old_uid: &str, new_uid: &str) {
        if old_uid == new_uid || self.study(old_uid).is_none() {
            return;
        }
        if self.study(new_uid).is_none() {
            if let Some(study) = self.study_mut(old_uid) {
                study.uid = new_uid.to_owned();
            }
            return;
        }
        if let Some(old) = self.remove_study(old_uid)
            && let Some(target) = self.study_mut(new_uid)
        {
            target.attributes.fill_missing(&old.attributes);
            target.series.extend(old.series);
        }
    }

    fn study_mut(&mut self, uid: &str) -> Option<&mut StudyNode> {
        self.patients
            .iter_mut()
            .flat_map(|p| &mut p.studies)
            .find(|s| s.uid == uid)
    }
}

/// Adds a study to a patient, joining it with an existing study of the same UID.
fn merge_study_into(studies: &mut Vec<StudyNode>, study: StudyNode) {
    match studies.iter_mut().find(|s| s.uid == study.uid) {
        Some(existing) => {
            existing.attributes.fill_missing(&study.attributes);
            existing.series.extend(study.series);
        }
        None => studies.push(study),
    }
}

/// Returns the first item matching `is_match`, appending `create()` when there is none.
fn get_or_insert<T>(
    items: &mut Vec<T>,
    is_match: impl Fn(&T) -> bool,
    create: impl FnOnce() -> T,
) -> &mut T {
    let index = match items.iter().position(is_match) {
        Some(index) => index,
        None => {
            items.push(create());
            items.len() - 1
        }
    };
    &mut items[index]
}

/// Removes and returns the first item matching `is_match`.
fn take_first<T>(items: &mut Vec<T>, is_match: impl Fn(&T) -> bool) -> Option<T> {
    let index = items.iter().position(is_match)?;
    Some(items.remove(index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dicom_dictionary_std::tags;

    fn instance(patient: &str, study: &str, series: &str, sop: &str) -> NewInstance {
        NewInstance {
            patient_key: patient.to_owned(),
            study_uid: study.to_owned(),
            series_key: series.to_owned(),
            sop_instance_uid: sop.to_owned(),
            ..NewInstance::default()
        }
    }

    #[test]
    fn instances_are_grouped_by_patient_study_and_series() {
        let mut model = DicomModel::new();
        model.add_instance(instance("P1", "ST1", "SE1", "I1"));
        model.add_instance(instance("P1", "ST1", "SE1", "I2"));
        model.add_instance(instance("P1", "ST1", "SE2", "I3"));
        model.add_instance(instance("P2", "ST2", "SE3", "I4"));

        assert_eq!(model.patients().len(), 2);
        assert_eq!(model.study("ST1").map(|s| s.series.len()), Some(2));
        assert_eq!(model.series("SE1").map(|s| s.instances.len()), Some(2));
    }

    #[test]
    fn a_known_study_keeps_its_patient_even_if_patient_attributes_differ() {
        let mut model = DicomModel::new();
        model.add_instance(instance("P1", "ST", "SE", "I1"));
        model.add_instance(instance("P1-TYPO", "ST", "SE", "I2"));

        assert_eq!(model.patients().len(), 1);
        assert_eq!(model.series("SE").map(|s| s.instances.len()), Some(2));
    }

    #[test]
    fn duplicate_sop_instance_is_ignored() {
        let mut model = DicomModel::new();
        assert!(model.add_instance(instance("P", "ST", "SE", "I")));
        assert!(!model.add_instance(instance("P", "ST", "SE", "I")));
        assert_eq!(model.series("SE").map(|s| s.instances.len()), Some(1));
    }

    #[test]
    fn group_attributes_come_from_the_first_file_and_are_completed_later() {
        let mut model = DicomModel::new();
        let mut first = instance("P", "ST", "SE", "I1");
        first.study.set(tags::STUDY_DESCRIPTION, "Head");
        let mut second = instance("P", "ST", "SE", "I2");
        second.study.set(tags::STUDY_DESCRIPTION, "Other");
        second.study.set(tags::STUDY_ID, "7");
        model.add_instance(first);
        model.add_instance(second);

        let study = model.study("ST").map(|s| &s.attributes);
        assert_eq!(
            study.and_then(|a| a.get(tags::STUDY_DESCRIPTION)),
            Some("Head")
        );
        assert_eq!(study.and_then(|a| a.get(tags::STUDY_ID)), Some("7"));
    }

    #[test]
    fn removing_a_series_keeps_its_study() {
        let mut model = DicomModel::new();
        model.add_instance(instance("P", "ST", "SE", "I"));
        assert!(model.remove_series("SE").is_some());
        assert_eq!(model.study("ST").map(|s| s.series.len()), Some(0));
        assert!(model.remove_series("SE").is_none());
    }

    #[test]
    fn merging_patients_joins_studies_with_the_same_uid() {
        let mut model = DicomModel::new();
        model.add_instance(instance("OLD", "ST1", "SE1", "I1"));
        model.add_instance(instance("OLD", "ST2", "SE2", "I2"));
        model.add_instance(instance("NEW", "ST1", "SE3", "I3"));

        model.merge_patients("OLD", "NEW");

        assert!(model.patient("OLD").is_none());
        let studies = model.patient("NEW").map(|p| p.studies.len());
        assert_eq!(studies, Some(2));
        assert_eq!(model.study("ST1").map(|s| s.series.len()), Some(2));
    }

    #[test]
    fn merging_into_a_missing_patient_renames_it() {
        let mut model = DicomModel::new();
        model.add_instance(instance("OLD", "ST", "SE", "I"));
        model.merge_patients("OLD", "NEW");
        assert_eq!(model.patients().len(), 1);
        assert!(model.patient("NEW").is_some());
    }

    #[test]
    fn merging_studies_moves_the_series() {
        let mut model = DicomModel::new();
        model.add_instance(instance("P", "OLD", "SE1", "I1"));
        model.add_instance(instance("P", "NEW", "SE2", "I2"));

        model.merge_studies("OLD", "NEW");

        assert!(model.study("OLD").is_none());
        assert_eq!(model.study("NEW").map(|s| s.series.len()), Some(2));
    }

    #[test]
    fn merging_into_a_missing_study_renames_it() {
        let mut model = DicomModel::new();
        model.add_instance(instance("P", "OLD", "SE", "I"));
        model.merge_studies("OLD", "NEW");
        assert!(model.study("NEW").is_some());
        assert!(model.study("OLD").is_none());
    }
}
