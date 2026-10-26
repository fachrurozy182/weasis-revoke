//! Turning a parsed DICOM file into an instance of the model tree.
//!
//! The attributes of each level (see [`Level::attributes`]) are copied as text. Sequences are
//! skipped here; the modules that need them (PR, KO, SEG, ...) read them from the object.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomMediaIO.java`
//! (`writeInstanceTags`) and `weasis-dicom/weasis-dicom-explorer/.../LoadDicom.java`.

use dicom_dictionary_std::tags;

use super::{DicomFile, frame_macro, item_text, object_kind};
use crate::error::{Error, Result};
use crate::model::{Attributes, Level, NO_VALUE, NewInstance, PatientIdentity, SplittingRules};

/// Builds the model instance of a parsed file.
///
/// Fails when the SOP Instance UID is missing: it is mandatory (Type 1) in every IOD and
/// identifies the object in the model.
pub fn new_instance(file: &DicomFile) -> Result<NewInstance> {
    let sop_instance_uid = file
        .text(tags::SOP_INSTANCE_UID)
        .ok_or_else(|| Error::UnreadableDicom("missing SOP Instance UID".to_owned()))?;

    let identity = PatientIdentity::new(
        file.text(tags::PATIENT_ID).as_deref(),
        file.text(tags::ISSUER_OF_PATIENT_ID).as_deref(),
        file.text(tags::PATIENT_NAME).as_deref(),
    );
    // NOTE: Study and Series Instance UIDs are Type 1, but some non-conformant files miss them.
    // They are grouped under "UNKNOWN" instead of being rejected, so they can still be viewed.
    let study_uid = file
        .text(tags::STUDY_INSTANCE_UID)
        .unwrap_or_else(|| NO_VALUE.to_owned());
    let series_uid = file
        .text(tags::SERIES_INSTANCE_UID)
        .unwrap_or_else(|| NO_VALUE.to_owned());

    let mut instance = level_attributes(file, Level::Instance);
    instance.set(tags::TRANSFER_SYNTAX_UID, file.transfer_syntax());
    let extra_tags = [
        tags::INSTANCE_NUMBER,
        tags::NUMBER_OF_FRAMES,
        tags::CONCATENATION_UID,
        tags::CONCATENATION_FRAME_OFFSET_NUMBER,
    ];
    for tag in extra_tags
        .into_iter()
        .chain(SplittingRules::built_in().tags())
    {
        if let Some(value) = file.text(tag) {
            instance.set(tag, value);
        }
    }
    set_plane_of_first_frame(file, &mut instance);

    Ok(NewInstance {
        patient_key: identity.pseudo_uid(),
        study_uid,
        // The sub-series is chosen when the instance is added (`DicomModel::add_split_instance`).
        series_key: series_uid,
        sop_instance_uid,
        kind: object_kind(file),
        patient: level_attributes(file, Level::Patient),
        study: level_attributes(file, Level::Study),
        series: level_attributes(file, Level::Series),
        instance,
    })
}

/// Image Position and Orientation (Patient) of the first frame, used to sort the stack: from
/// the Plane Position / Plane Orientation functional groups of enhanced objects, else from the
/// data set (PS3.3 §C.7.6.2).
fn set_plane_of_first_frame(file: &DicomFile, instance: &mut Attributes) {
    let planes = [
        (tags::PLANE_POSITION_SEQUENCE, tags::IMAGE_POSITION_PATIENT),
        (
            tags::PLANE_ORIENTATION_SEQUENCE,
            tags::IMAGE_ORIENTATION_PATIENT,
        ),
    ];
    for (group, tag) in planes {
        let value = frame_macro(file, 0, group)
            .and_then(|item| item_text(item, tag))
            .or_else(|| file.text(tag));
        if let Some(value) = value {
            instance.set(tag, value);
        }
    }
}

/// Copies the non-empty text attributes of one level.
fn level_attributes(file: &DicomFile, level: Level) -> Attributes {
    let mut attributes = Attributes::new();
    for &tag in level.attributes() {
        if let Some(value) = file.text(tag) {
            attributes.set(tag, value);
        }
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;
    use crate::model::DicomModel;

    fn instance_of(path: &str) -> NewInstance {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        new_instance(&read_dicom(&bytes).expect("readable")).expect("instance")
    }

    #[test]
    fn copies_the_attributes_of_each_level() {
        let new = instance_of("synthetic/ct-series/CT001.dcm");

        assert_eq!(new.patient_key, "WRS-0001WEASIS^SYNTHETIC");
        assert_eq!(new.study_uid, "2.25.1234567890.1");
        assert_eq!(new.series_key, "2.25.1234567890.10");
        assert_eq!(new.sop_instance_uid, "2.25.1234567890.10.1");
        assert_eq!(new.patient.get(tags::PATIENT_SEX), Some("O"));
        assert_eq!(new.study.get(tags::ACCESSION_NUMBER), Some("WRS-ACC-1"));
        assert_eq!(new.series.get(tags::MODALITY), Some("CT"));
        assert_eq!(new.instance.get(tags::INSTANCE_NUMBER), Some("1"));
        assert_eq!(
            new.instance.get(tags::TRANSFER_SYNTAX_UID),
            Some("1.2.840.10008.1.2.1")
        );
    }

    #[test]
    fn synthetic_study_builds_one_patient_with_all_series() {
        let mut model = DicomModel::new();
        let files = [
            "synthetic/ct-series/CT001.dcm",
            "synthetic/ct-series/CT002.dcm",
            "synthetic/pet-series/PT001.dcm",
            "synthetic/cr-monochrome1.dcm",
        ];
        for file in files {
            assert!(model.add_instance(instance_of(file)));
        }

        assert_eq!(model.patients().len(), 1);
        let study = model.study("2.25.1234567890.1").expect("study");
        assert_eq!(study.series.len(), 3);
        assert_eq!(
            model
                .series("2.25.1234567890.10")
                .map(|s| s.instances.len()),
            Some(2)
        );
    }

    #[test]
    fn a_uniform_ct_series_is_not_split() {
        let rules = crate::model::SplittingRules::built_in();
        let mut model = DicomModel::new();
        for slice in 1..=16 {
            let new = instance_of(&format!("synthetic/ct-series/CT{slice:03}.dcm"));
            assert_eq!(new.kind, crate::dicom::ObjectKind::Image);
            assert_eq!(
                new.instance.get(tags::IMAGE_TYPE),
                Some("ORIGINAL\\PRIMARY\\AXIAL")
            );
            model.add_split_instance(new, rules);
        }
        let study = model.study("2.25.1234567890.1").expect("study");
        assert_eq!(study.series.len(), 1);
        assert_eq!(study.series[0].split_number, None);
    }

    #[test]
    fn a_reversed_ct_series_is_sorted_back_by_slice_position() {
        use crate::model::{SortKey, sort_instances};
        let mut model = DicomModel::new();
        for slice in (1..=16).rev() {
            model.add_instance(instance_of(&format!(
                "synthetic/ct-series/CT{slice:03}.dcm"
            )));
        }
        let mut stack = model
            .series("2.25.1234567890.10")
            .expect("series")
            .instances
            .clone();
        sort_instances(&mut stack, SortKey::SlicePosition);
        let numbers: Vec<&str> = stack
            .iter()
            .filter_map(|i| i.attributes.get(tags::INSTANCE_NUMBER))
            .collect();
        let expected: Vec<String> = (1..=16).map(|n| n.to_string()).collect();
        assert_eq!(numbers, expected);
    }
}
