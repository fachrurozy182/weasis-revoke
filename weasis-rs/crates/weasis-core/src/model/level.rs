//! Levels of the DICOM information model and the attributes kept at each level.
//!
//! When a file is loaded, the attributes listed for a level are copied to the matching node of
//! the model tree (patient, study, series) or kept on the instance. The lists come from the
//! static `tagManager` registrations of the Java viewer. Attributes needed by the corner
//! annotations are added to the instance level in step 1.12.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/TagD.java`
//! (`Level`) and `.../codec/DicomMediaIO.java` (static `tagManager` block).

use dicom_dictionary_std::tags;
use dicom_object::Tag;

/// A level of the DICOM information model (PS3.3 §A.1.2), from the widest to the narrowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// Patient.
    Patient,
    /// Study.
    Study,
    /// Series (in the model tree: a sub-series, see step 1.7).
    Series,
    /// One DICOM object (SOP instance).
    Instance,
    /// One frame of a multi-frame instance.
    Frame,
}

impl Level {
    /// Attributes copied from each file to the node of this level.
    pub fn attributes(self) -> &'static [Tag] {
        match self {
            Level::Patient => PATIENT_ATTRIBUTES,
            Level::Study => STUDY_ATTRIBUTES,
            Level::Series => SERIES_ATTRIBUTES,
            Level::Instance => INSTANCE_ATTRIBUTES,
            Level::Frame => &[],
        }
    }
}

const PATIENT_ATTRIBUTES: &[Tag] = &[
    tags::PATIENT_ID,
    tags::PATIENT_NAME,
    tags::PATIENT_BIRTH_DATE,
    tags::PATIENT_BIRTH_TIME,
    tags::PATIENT_SEX,
    tags::ISSUER_OF_PATIENT_ID,
    tags::PATIENT_WEIGHT,
    tags::PATIENT_COMMENTS,
];

// Study Comments is retired in DICOM but still present in older files, so it is kept.
#[allow(deprecated)]
const STUDY_ATTRIBUTES: &[Tag] = &[
    tags::STUDY_INSTANCE_UID,
    tags::STUDY_ID,
    tags::STUDY_DATE,
    tags::STUDY_TIME,
    tags::STUDY_DESCRIPTION,
    tags::STUDY_COMMENTS,
    tags::ACCESSION_NUMBER,
    tags::MODALITIES_IN_STUDY,
    tags::NUMBER_OF_STUDY_RELATED_INSTANCES,
    tags::NUMBER_OF_STUDY_RELATED_SERIES,
];

// NOTE: Patient's Age is a patient attribute in DICOM, but it is kept per series because its
// value depends on the acquisition date (same choice as the Java viewer).
const SERIES_ATTRIBUTES: &[Tag] = &[
    tags::SERIES_INSTANCE_UID,
    tags::MODALITY,
    tags::PATIENT_AGE,
    tags::SERIES_DESCRIPTION,
    tags::SOP_CLASS_UID,
    tags::RETRIEVE_AE_TITLE,
    tags::REFERRING_PHYSICIAN_NAME,
    tags::INSTITUTION_NAME,
    tags::INSTITUTIONAL_DEPARTMENT_NAME,
    tags::STATION_NAME,
    tags::MANUFACTURER,
    tags::ANATOMICAL_ORIENTATION_TYPE,
    tags::MANUFACTURER_MODEL_NAME,
    tags::SERIES_NUMBER,
    tags::NUMBER_OF_FRAMES,
    tags::SERIES_DATE,
    tags::SERIES_TIME,
    tags::PERFORMED_PROCEDURE_STEP_START_DATE,
    tags::PERFORMED_PROCEDURE_STEP_START_TIME,
    tags::PREFERRED_PLAYBACK_SEQUENCING,
    tags::KVP,
    tags::BODY_PART_EXAMINED,
    tags::FRAME_OF_REFERENCE_UID,
    tags::NUMBER_OF_SERIES_RELATED_INSTANCES,
    tags::LATERALITY,
];

const INSTANCE_ATTRIBUTES: &[Tag] = &[
    // Identification of the parent groups.
    tags::PATIENT_ID,
    tags::PATIENT_NAME,
    tags::PATIENT_BIRTH_DATE,
    tags::ISSUER_OF_PATIENT_ID,
    tags::STUDY_INSTANCE_UID,
    tags::SERIES_INSTANCE_UID,
    tags::MODALITY,
    // Instance attributes.
    tags::GANTRY_DETECTOR_TILT,
    tags::PATIENT_ORIENTATION,
    tags::SLICE_LOCATION,
    tags::SLICE_THICKNESS,
    tags::ACQUISITION_DATE,
    tags::ACQUISITION_TIME,
    tags::CONTENT_DATE,
    tags::CONTENT_TIME,
    tags::DIFFUSION_B_VALUE,
    tags::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT,
    tags::PIXEL_DATA_PROVIDER_URL,
    tags::ANATOMIC_REGION_SEQUENCE,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_from_patient_to_frame() {
        assert!(Level::Patient < Level::Study);
        assert!(Level::Series < Level::Instance);
        assert!(Level::Instance < Level::Frame);
    }

    #[test]
    fn each_group_level_keeps_its_identifying_attribute() {
        assert!(Level::Patient.attributes().contains(&tags::PATIENT_ID));
        assert!(
            Level::Study
                .attributes()
                .contains(&tags::STUDY_INSTANCE_UID)
        );
        assert!(
            Level::Series
                .attributes()
                .contains(&tags::SERIES_INSTANCE_UID)
        );
    }
}
