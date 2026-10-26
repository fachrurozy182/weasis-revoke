//! What kind of DICOM object a file holds, which decides how it is shown.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomMediaIO.java`
//! (`setMimeType`, `setDicomSpecialType`, `DCM_ELEMENT_FACTORIES`) and the element factories of
//! the `weasis-dicom-rt`, `-sr`, `-au` and `-wave` modules.

use dicom_dictionary_std::tags;

use super::DicomFile;

/// Media Storage Directory Storage: a DICOMDIR, read in step 1.10 (PS3.4 Annex B.5).
const MEDIA_STORAGE_DIRECTORY: &str = "1.2.840.10008.1.3.10";
/// Segmentation Storage.
const SEGMENTATION: &str = "1.2.840.10008.5.1.4.1.1.66.4";
/// Color Palette Storage.
const COLOR_PALETTE: &str = "1.2.840.10008.5.1.4.1.1.39.1";
/// Prefix of the MPEG-2, MPEG-4 and HEVC transfer syntaxes (1.2.840.10008.1.2.4.100 to .109).
const VIDEO_TRANSFER_SYNTAX_PREFIX: &str = "1.2.840.10008.1.2.4.10";

/// The kind of a DICOM object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// Single or multi-frame image.
    Image,
    /// Image encoded as a video stream (MPEG-2, MPEG-4, HEVC).
    Video,
    /// Segmentation (SEG), drawn over the images it references.
    Segmentation,
    /// Presentation State (PR), applied to the images it references.
    PresentationState,
    /// Key Object Selection (KO), a list of flagged images.
    KeyObject,
    /// Radiotherapy structure set, plan or dose without pixel data.
    Radiotherapy,
    /// Structured Report (SR).
    StructuredReport,
    /// Waveform such as an ECG or hemodynamic recording.
    Waveform,
    /// Audio recording.
    Audio,
    /// Encapsulated document (PDF, CDA, ...).
    EncapsulatedDocument,
    /// Color Palette: not displayed, its palette is added to the color maps (step 1.6).
    ColorPalette,
    /// DICOMDIR: not displayed, it lists other files (step 1.10).
    Directory,
    /// Valid DICOM object that Weasis does not display.
    Unsupported,
}

impl ObjectKind {
    /// Kinds that have no series of their own in the explorer: they are used through the
    /// images they reference.
    pub fn is_hidden(self) -> bool {
        matches!(
            self,
            Self::Segmentation | Self::PresentationState | Self::KeyObject | Self::Radiotherapy
        )
    }
}

/// Finds the kind of a parsed file.
pub fn object_kind(file: &DicomFile) -> ObjectKind {
    let sop_class = media_storage_sop_class(file);
    if sop_class.as_deref() == Some(MEDIA_STORAGE_DIRECTORY) {
        return ObjectKind::Directory;
    }

    if file.has_pixel_data() {
        return if file
            .transfer_syntax()
            .starts_with(VIDEO_TRANSFER_SYNTAX_PREFIX)
        {
            ObjectKind::Video
        } else if sop_class.as_deref() == Some(SEGMENTATION) {
            ObjectKind::Segmentation
        } else {
            ObjectKind::Image
        };
    }

    // An ECG, for instance, can also carry an encapsulated PDF: the document wins.
    if file
        .text(tags::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT)
        .is_some()
    {
        return ObjectKind::EncapsulatedDocument;
    }
    let by_modality = file
        .text(tags::MODALITY)
        .and_then(|modality| kind_of_modality(&modality));
    if let Some(kind) = by_modality {
        return kind;
    }
    if sop_class.as_deref() == Some(COLOR_PALETTE) {
        return ObjectKind::ColorPalette;
    }
    ObjectKind::Unsupported
}

/// Kind of an object without pixel data, from its Modality.
fn kind_of_modality(modality: &str) -> Option<ObjectKind> {
    let kind = match modality {
        "PR" => ObjectKind::PresentationState,
        "KO" => ObjectKind::KeyObject,
        "SEG" => ObjectKind::Segmentation,
        "RTSTRUCT" | "RTPLAN" | "RTDOSE" => ObjectKind::Radiotherapy,
        "SR" => ObjectKind::StructuredReport,
        "ECG" | "HD" => ObjectKind::Waveform,
        "AU" => ObjectKind::Audio,
        _ => return None,
    };
    Some(kind)
}

/// Media Storage SOP Class UID from the File Meta Information, or the SOP Class UID of the
/// data set for files read without File Meta Information.
fn media_storage_sop_class(file: &DicomFile) -> Option<String> {
    let from_meta = file
        .object()
        .meta()
        .media_storage_sop_class_uid()
        .trim_end_matches('\0')
        .trim();
    if from_meta.is_empty() {
        file.text(tags::SOP_CLASS_UID)
    } else {
        Some(from_meta.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn kind_of(path: &str) -> ObjectKind {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        object_kind(&read_dicom(&bytes).expect("readable"))
    }

    #[test]
    fn classifies_every_kind_of_test_object() {
        let cases = [
            ("synthetic/ct-series/CT001.dcm", ObjectKind::Image),
            ("pydicom/emri_small.dcm", ObjectKind::Image),
            ("synthetic/video-mpeg4-h264.dcm", ObjectKind::Video),
            ("synthetic/video-mpeg2.dcm", ObjectKind::Video),
            ("synthetic/seg-sphere.dcm", ObjectKind::Segmentation),
            ("synthetic/pr-annotation.dcm", ObjectKind::PresentationState),
            (
                "weasis-dicom-tools/prLUTs.dcm",
                ObjectKind::PresentationState,
            ),
            ("synthetic/ko-for-teaching.dcm", ObjectKind::KeyObject),
            ("pydicom/rtstruct.dcm", ObjectKind::Radiotherapy),
            ("pydicom/rtplan.dcm", ObjectKind::Radiotherapy),
            ("pydicom/test-SR.dcm", ObjectKind::StructuredReport),
            ("pydicom/waveform_ecg.dcm", ObjectKind::Waveform),
            ("synthetic/audio-440hz.dcm", ObjectKind::Audio),
            (
                "synthetic/encapsulated-pdf.dcm",
                ObjectKind::EncapsulatedDocument,
            ),
            (
                "pydicom/dicomdir-tiny-alpha/DICOMDIR",
                ObjectKind::Directory,
            ),
            (
                "weasis-dicom-tools/NeurosurgeryPlan.dcm",
                ObjectKind::Unsupported,
            ),
        ];
        for (path, expected) in cases {
            assert_eq!(kind_of(path), expected, "{path}");
        }
    }

    #[test]
    fn rt_dose_with_pixel_data_is_an_image() {
        // As in the Java viewer, the dose grid is displayed as an image series.
        assert_eq!(kind_of("pydicom/rtdose.dcm"), ObjectKind::Image);
    }

    #[test]
    fn hidden_kinds_are_the_ones_applied_to_images() {
        assert!(ObjectKind::PresentationState.is_hidden());
        assert!(!ObjectKind::Image.is_hidden());
        assert!(!ObjectKind::StructuredReport.is_hidden());
    }
}
