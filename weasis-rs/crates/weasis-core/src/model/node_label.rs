//! Text naming a patient, a study or a series in the explorer: the first attribute of a list
//! that has a value, "?" when none has.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomModel.java`
//! (the `TagView` of the `patient`, `study` and `series` tree nodes) and `MediaSeriesGroupNode.toString`.

use dicom_dictionary_std::tags;

use super::{Attributes, IdentityMask};
use crate::image::TagView;

/// Shown for a node without any of its attributes (`MediaSeriesGroupNode.no_val`).
const NO_VALUE: &str = "?";

/// Patient name, else ID.
pub fn patient_label(attributes: &Attributes, mask: Option<&IdentityMask>) -> String {
    label(&[tags::PATIENT_NAME, tags::PATIENT_ID], attributes, mask)
}

/// Study date, else accession number, ID or description.
pub fn study_label(attributes: &Attributes, mask: Option<&IdentityMask>) -> String {
    let candidates = [
        tags::STUDY_DATE,
        tags::ACCESSION_NUMBER,
        tags::STUDY_ID,
        tags::STUDY_DESCRIPTION,
    ];
    label(&candidates, attributes, mask)
}

/// Series description, else number or time.
pub fn series_label(attributes: &Attributes, mask: Option<&IdentityMask>) -> String {
    let candidates = [
        tags::SERIES_DESCRIPTION,
        tags::SERIES_NUMBER,
        tags::SERIES_TIME,
    ];
    label(&candidates, attributes, mask)
}

fn label(
    candidates: &[dicom_object::Tag],
    attributes: &Attributes,
    mask: Option<&IdentityMask>,
) -> String {
    let text = TagView::of(candidates).masked_text(&[attributes], mask);
    if text.is_empty() {
        NO_VALUE.to_owned()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_present_attribute_names_the_node() {
        let mut attributes = Attributes::new();
        assert_eq!(patient_label(&attributes, None), "?");
        attributes.set(tags::PATIENT_ID, "123");
        assert_eq!(patient_label(&attributes, None), "123");
        attributes.set(tags::PATIENT_NAME, "Doe^John");
        assert_eq!(
            patient_label(&attributes, None),
            "Doe, John",
            "formatted like Java"
        );
        let mut study = Attributes::new();
        study.set(tags::STUDY_DESCRIPTION, "Head");
        assert_eq!(study_label(&study, None), "Head");
    }
}
