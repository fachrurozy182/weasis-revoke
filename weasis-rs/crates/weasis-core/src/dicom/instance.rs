//! Turning a parsed DICOM file into an instance of the model tree.
//!
//! The attributes of each level (see [`Level::attributes`]) are copied as text. Sequences are
//! skipped here; the modules that need them (PR, KO, SEG, ...) read them from the object.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomMediaIO.java`
//! (`writeInstanceTags`) and `weasis-dicom/weasis-dicom-explorer/.../LoadDicom.java`.

use dicom_dictionary_std::tags;
use dicom_object::{DefaultDicomObject, Tag};

use super::DicomFile;
use crate::error::{Error, Result};
use crate::model::{Attributes, Level, NO_VALUE, NewInstance, PatientIdentity};

/// Builds the model instance of a parsed file.
///
/// Fails when the SOP Instance UID is missing: it is mandatory (Type 1) in every IOD and
/// identifies the object in the model.
pub fn new_instance(file: &DicomFile) -> Result<NewInstance> {
    let object = file.object();
    let sop_instance_uid = text(object, tags::SOP_INSTANCE_UID)
        .ok_or_else(|| Error::UnreadableDicom("missing SOP Instance UID".to_owned()))?;

    let identity = PatientIdentity::new(
        text(object, tags::PATIENT_ID).as_deref(),
        text(object, tags::ISSUER_OF_PATIENT_ID).as_deref(),
        text(object, tags::PATIENT_NAME).as_deref(),
    );
    // NOTE: Study and Series Instance UIDs are Type 1, but some non-conformant files miss them.
    // They are grouped under "UNKNOWN" instead of being rejected, so they can still be viewed.
    let study_uid = text(object, tags::STUDY_INSTANCE_UID).unwrap_or_else(|| NO_VALUE.to_owned());
    let series_uid = text(object, tags::SERIES_INSTANCE_UID).unwrap_or_else(|| NO_VALUE.to_owned());

    let mut instance = level_attributes(object, Level::Instance);
    instance.set(tags::TRANSFER_SYNTAX_UID, file.transfer_syntax());
    for tag in [
        tags::INSTANCE_NUMBER,
        tags::CONCATENATION_UID,
        tags::CONCATENATION_FRAME_OFFSET_NUMBER,
    ] {
        if let Some(value) = text(object, tag) {
            instance.set(tag, value);
        }
    }

    Ok(NewInstance {
        patient_key: identity.pseudo_uid(),
        study_uid,
        // A series is split into sub-series in step 1.7; until then they share the same key.
        series_key: series_uid,
        sop_instance_uid,
        patient: level_attributes(object, Level::Patient),
        study: level_attributes(object, Level::Study),
        series: level_attributes(object, Level::Series),
        instance,
    })
}

/// Copies the non-empty text attributes of one level.
fn level_attributes(object: &DefaultDicomObject, level: Level) -> Attributes {
    let mut attributes = Attributes::new();
    for &tag in level.attributes() {
        if let Some(value) = text(object, tag) {
            attributes.set(tag, value);
        }
    }
    attributes
}

/// Text value of an attribute without padding, or `None` when absent, empty or a sequence.
fn text(object: &DefaultDicomObject, tag: Tag) -> Option<String> {
    let element = object.element(tag).ok()?;
    let value = element.to_str().ok()?;
    // Values are padded with a space or NUL to an even length (PS3.5 §6.2).
    let trimmed = value.trim_matches([' ', '\0']);
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
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
}
