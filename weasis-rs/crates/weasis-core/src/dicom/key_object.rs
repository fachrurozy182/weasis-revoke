//! Key Object Selection documents (KO): lists of flagged images (PS3.3 §A.35.4), including the
//! rejection notes that mark images as rejected (IHE Imaging Object Change Management).
//!
//! The references come from the Current Requested Procedure Evidence Sequence (Hierarchical
//! SOP Instance Reference Macro, PS3.3 Table C.17-3). They can be edited (toggling the key
//! image flag of an image) before the document is written again in step 4.x.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/AbstractKOSpecialElement.java`,
//! `KOSpecialElement.java` and `RejectedKOSpecialElement.java`.

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};

use super::{ContentOrder, DicomFile, first_item, item_text};

/// Concept names of a KO that rejects images (CID 7011 "Rejected for ..." and the IOCM titles).
const REJECTION_TITLES: [&str; 4] = [
    "Rejected for Quality Reasons",
    "Rejected for Patient Safety Reasons",
    "Incorrect Modality Worklist Entry",
    "Data Retention Policy Expired",
];

/// An image referenced by a Key Object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyImage {
    /// Study Instance UID.
    pub study_instance_uid: String,
    /// Series Instance UID.
    pub series_instance_uid: String,
    /// Referenced SOP Instance UID.
    pub sop_instance_uid: String,
    /// Referenced SOP Class UID.
    pub sop_class_uid: Option<String>,
    /// Referenced Frame Numbers (1-based); empty for the whole image.
    pub frames: Vec<i32>,
}

/// A parsed Key Object Selection document.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyObject {
    /// SOP Instance UID of the document.
    pub sop_instance_uid: String,
    /// Series Instance UID of the document.
    pub series_instance_uid: String,
    /// Label shown in menus: "KO [instance number] " and the first text of the content.
    pub label: String,
    /// Code Meaning of the Concept Name (document title), e.g. "Of Interest".
    pub title: Option<String>,
    /// Whether the document rejects its images.
    pub is_rejection: bool,
    /// Sort order (newest first).
    pub order: ContentOrder,
    /// Referenced images, in document order.
    pub images: Vec<KeyImage>,
}

impl KeyObject {
    /// Reads a Key Object Selection document.
    pub fn from_file(file: &DicomFile) -> Self {
        let object: &InMemDicomObject = file.object();
        let title = first_item(object, tags::CONCEPT_NAME_CODE_SEQUENCE)
            .and_then(|item| item_text(item, tags::CODE_MEANING));
        let is_rejection = title
            .as_deref()
            .is_some_and(|t| REJECTION_TITLES.iter().any(|r| r.eq_ignore_ascii_case(t)));
        Self {
            sop_instance_uid: file.text(tags::SOP_INSTANCE_UID).unwrap_or_default(),
            series_instance_uid: file.text(tags::SERIES_INSTANCE_UID).unwrap_or_default(),
            label: label(file),
            title,
            is_rejection,
            order: ContentOrder::from_file(file),
            images: key_images(object),
        }
    }

    /// Whether the document flags a frame (1-based, `None` for any) of an image of a series.
    pub fn contains(
        &self,
        series_instance_uid: &str,
        sop_instance_uid: &str,
        frame: Option<i32>,
    ) -> bool {
        self.images.iter().any(|image| {
            image.series_instance_uid == series_instance_uid
                && image.sop_instance_uid == sop_instance_uid
                && (image.frames.is_empty() || frame.is_none_or(|f| image.frames.contains(&f)))
        })
    }

    /// Whether the document references an image of the series.
    pub fn references_series(&self, series_instance_uid: &str) -> bool {
        self.images
            .iter()
            .any(|i| i.series_instance_uid == series_instance_uid)
    }

    /// Number of flagged images, a multi-frame image counting its listed frames.
    pub fn selected_count(&self) -> usize {
        self.images.iter().map(|i| i.frames.len().max(1)).sum()
    }

    /// Flags an image (or some frames of it). Returns `false` when it was already flagged.
    pub fn add(&mut self, image: KeyImage) -> bool {
        let Some(existing) = self.find(&image.series_instance_uid, &image.sop_instance_uid) else {
            self.images.push(image);
            return true;
        };
        // NOTE: the Java viewer replaces a whole-image reference by the added frames, which
        // unflags the other frames; here a flagged whole image stays flagged.
        if image.frames.is_empty() || existing.frames.is_empty() {
            return false;
        }
        let before = existing.frames.len();
        existing.frames.extend(image.frames);
        existing.frames.sort_unstable();
        existing.frames.dedup();
        existing.frames.len() != before
    }

    /// Unflags an image (or some frames of it). Returns `false` when it was not flagged.
    pub fn remove(&mut self, image: &KeyImage) -> bool {
        let Some(existing) = self.find(&image.series_instance_uid, &image.sop_instance_uid) else {
            return false;
        };
        if !image.frames.is_empty() && !existing.frames.is_empty() {
            existing.frames.retain(|f| !image.frames.contains(f));
            if !existing.frames.is_empty() {
                return true;
            }
        }
        self.images.retain(|i| {
            i.series_instance_uid != image.series_instance_uid
                || i.sop_instance_uid != image.sop_instance_uid
        });
        true
    }

    /// Flags the image when it is not flagged, unflags it otherwise.
    pub fn toggle(&mut self, image: KeyImage) -> bool {
        if self.contains(
            &image.series_instance_uid,
            &image.sop_instance_uid,
            image.frames.first().copied(),
        ) {
            self.remove(&image)
        } else {
            self.add(image)
        }
    }

    fn find(&mut self, series: &str, sop: &str) -> Option<&mut KeyImage> {
        self.images
            .iter_mut()
            .find(|i| i.series_instance_uid == series && i.sop_instance_uid == sop)
    }
}

fn label(file: &DicomFile) -> String {
    let mut label = String::from("KO ");
    if let Some(number) = file.text(tags::INSTANCE_NUMBER) {
        label.push_str(&format!("[{number}] "));
    }
    let text = first_item(file.object(), tags::CONTENT_SEQUENCE)
        .and_then(|item| file.item_text(item, tags::TEXT_VALUE));
    label.push_str(text.as_deref().unwrap_or_default());
    label.trim_end().to_owned()
}

fn key_images(object: &InMemDicomObject) -> Vec<KeyImage> {
    let mut images = Vec::new();
    for study in items(object, tags::CURRENT_REQUESTED_PROCEDURE_EVIDENCE_SEQUENCE) {
        let study_uid = item_text(study, tags::STUDY_INSTANCE_UID).unwrap_or_default();
        for series in items(study, tags::REFERENCED_SERIES_SEQUENCE) {
            let series_uid = item_text(series, tags::SERIES_INSTANCE_UID).unwrap_or_default();
            for sop in items(series, tags::REFERENCED_SOP_SEQUENCE) {
                let Some(sop_uid) = item_text(sop, tags::REFERENCED_SOP_INSTANCE_UID) else {
                    continue;
                };
                images.push(KeyImage {
                    study_instance_uid: study_uid.clone(),
                    series_instance_uid: series_uid.clone(),
                    sop_instance_uid: sop_uid,
                    sop_class_uid: item_text(sop, tags::REFERENCED_SOP_CLASS_UID),
                    frames: sop
                        .element(tags::REFERENCED_FRAME_NUMBER)
                        .ok()
                        .and_then(|e| e.to_multi_int::<i32>().ok())
                        .unwrap_or_default(),
                });
            }
        }
    }
    images
}

fn items(parent: &InMemDicomObject, sequence: Tag) -> &[InMemDicomObject] {
    parent
        .element(sequence)
        .ok()
        .and_then(|element| element.items())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn key_object() -> KeyObject {
        let path = format!(
            "{}/../../testdata/synthetic/ko-for-teaching.dcm",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        KeyObject::from_file(&read_dicom(&bytes).expect("readable"))
    }

    fn image(sop: &str, frames: &[i32]) -> KeyImage {
        KeyImage {
            study_instance_uid: "1".to_owned(),
            series_instance_uid: "1.1".to_owned(),
            sop_instance_uid: sop.to_owned(),
            sop_class_uid: None,
            frames: frames.to_vec(),
        }
    }

    #[test]
    fn the_synthetic_document_flags_ct_images() {
        let ko = key_object();
        assert!(!ko.is_rejection);
        assert!(ko.label.starts_with("KO [1]"), "{}", ko.label);
        assert!(!ko.images.is_empty());
        let first = &ko.images[0];
        assert!(ko.references_series(&first.series_instance_uid));
        assert!(ko.contains(&first.series_instance_uid, &first.sop_instance_uid, Some(1)));
        assert_eq!(ko.selected_count(), ko.images.len());
    }

    #[test]
    fn frames_are_merged_and_removed() {
        let mut ko = KeyObject {
            images: Vec::new(),
            ..key_object()
        };
        assert!(ko.add(image("a", &[2])));
        assert!(ko.add(image("a", &[5])));
        assert!(!ko.add(image("a", &[5])), "already flagged");
        assert_eq!(ko.selected_count(), 2);
        assert!(ko.contains("1.1", "a", Some(5)));
        assert!(!ko.contains("1.1", "a", Some(3)));

        assert!(ko.remove(&image("a", &[2])));
        assert_eq!(ko.images[0].frames, [5]);
        assert!(ko.remove(&image("a", &[5])));
        assert!(ko.images.is_empty(), "no frame left");
        assert!(!ko.remove(&image("a", &[])));
    }

    #[test]
    fn toggle_flags_then_unflags() {
        let mut ko = KeyObject {
            images: Vec::new(),
            ..key_object()
        };
        assert!(ko.toggle(image("b", &[])));
        assert!(ko.contains("1.1", "b", None));
        assert!(ko.toggle(image("b", &[])));
        assert!(!ko.contains("1.1", "b", None));
    }
}
