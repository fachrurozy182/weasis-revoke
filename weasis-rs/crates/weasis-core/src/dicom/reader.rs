//! Reading a DICOM file from bytes.
//!
//! The whole file is parsed, including the pixel data, which is decoded later on demand
//! (step 1.3). Files with or without the 128-byte preamble are accepted, as well as files without
//! File Meta Information (written by old devices).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomMediaIO.java`
//! (`readMetaData`) and `weasis-dicom-tools/.../org/dcm4che3/img/stream/DicomFileInputStream.java`.

use dicom_core::VR;
use dicom_dictionary_std::tags;
use dicom_object::file::ReadPreamble;
use dicom_object::{
    DefaultDicomObject, FileMetaTableBuilder, InMemDicomObject, OpenFileOptions, Tag,
};
use dicom_transfer_syntax_registry::entries::{
    EXPLICIT_VR_LITTLE_ENDIAN, IMPLICIT_VR_LITTLE_ENDIAN,
};

use super::charset;
use crate::error::{Error, Result};

/// A parsed DICOM file.
#[derive(Debug, Clone)]
pub struct DicomFile {
    object: DefaultDicomObject,
    has_pixel_data: bool,
    number_of_frames: u32,
    /// Values of Specific Character Set (0008,0005), empty when absent.
    specific_character_set: Vec<String>,
}

impl DicomFile {
    /// Text value of an attribute without padding, or `None` when absent, empty or a sequence.
    ///
    /// Values using ISO 2022 code extensions (Japanese, Korean, Chinese) are decoded here
    /// because `dicom-rs` does not support them (see [`charset`]).
    pub fn text(&self, tag: Tag) -> Option<String> {
        let element = self.object.element(tag).ok()?;
        let value = element.to_str().ok()?;
        let decoded = if charset::needs_code_extension_decoding(&self.specific_character_set) {
            let first = self
                .specific_character_set
                .first()
                .map_or("", String::as_str);
            charset::decode_code_extensions(&value, first, element.vr() == VR::PN)
        } else {
            value.into_owned()
        };
        // Values are padded with a space or NUL to an even length (PS3.5 §6.2).
        let trimmed = decoded.trim_matches([' ', '\0']);
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }

    /// The parsed data set with its File Meta Information.
    pub fn object(&self) -> &DefaultDicomObject {
        &self.object
    }

    /// Whether the file contains Pixel Data, Float Pixel Data or Double Float Pixel Data.
    pub fn has_pixel_data(&self) -> bool {
        self.has_pixel_data
    }

    /// Number of frames; 1 for a single-frame image and 0 when there is no pixel data.
    pub fn number_of_frames(&self) -> u32 {
        self.number_of_frames
    }

    /// Transfer Syntax UID from the File Meta Information.
    pub fn transfer_syntax(&self) -> &str {
        self.object.meta().transfer_syntax().trim_end_matches('\0')
    }
}

/// Parses a DICOM file.
pub fn read_dicom(bytes: &[u8]) -> Result<DicomFile> {
    let object = if has_dicm_prefix(bytes) {
        OpenFileOptions::new()
            .read_preamble(ReadPreamble::Auto)
            .from_reader(bytes)
            .map_err(|error| Error::UnreadableDicom(error.to_string()))?
    } else {
        read_without_file_meta(bytes)?
    };

    let has_pixel_data = [
        tags::PIXEL_DATA,
        tags::FLOAT_PIXEL_DATA,
        tags::DOUBLE_FLOAT_PIXEL_DATA,
    ]
    .into_iter()
    .any(|tag| object.element(tag).is_ok());

    let declared_frames = object
        .element(tags::NUMBER_OF_FRAMES)
        .ok()
        .and_then(|element| element.to_int::<u32>().ok())
        .unwrap_or(0);
    // Number of Frames is only present in multi-frame objects: a single image has one frame.
    let number_of_frames = match (has_pixel_data, declared_frames) {
        (false, _) => 0,
        (true, 0) => 1,
        (true, frames) => frames,
    };

    let specific_character_set = object
        .element(tags::SPECIFIC_CHARACTER_SET)
        .ok()
        .and_then(|element| element.to_multi_str().ok().map(|values| values.to_vec()))
        .unwrap_or_default();

    Ok(DicomFile {
        object,
        has_pixel_data,
        number_of_frames,
        specific_character_set,
    })
}

/// Whether "DICM" is present after the preamble (PS3.10 §7.1) or at the very start.
fn has_dicm_prefix(bytes: &[u8]) -> bool {
    bytes.get(128..132) == Some(b"DICM") || bytes.get(0..4) == Some(b"DICM")
}

/// Reads a bare data set, guessing between Implicit and Explicit VR Little Endian.
///
// NOTE: departs from PS3.10 §7.1, which requires File Meta Information. Old devices wrote data
// sets without it; like dcm4che, the encoding is guessed from the first element.
// Big Endian bare data sets are not supported.
fn read_without_file_meta(bytes: &[u8]) -> Result<DefaultDicomObject> {
    let transfer_syntax = if looks_like_explicit_vr(bytes) {
        EXPLICIT_VR_LITTLE_ENDIAN.erased()
    } else {
        IMPLICIT_VR_LITTLE_ENDIAN.erased()
    };
    let data_set = InMemDicomObject::read_dataset_with_ts(bytes, &transfer_syntax)
        .map_err(|error| Error::UnreadableDicom(error.to_string()))?;

    // Without the "DICM" marker, arbitrary bytes can parse as a data set; a DICOM object must
    // at least identify its SOP Class (PS3.3 C.12.1).
    if data_set.element(tags::SOP_CLASS_UID).is_err() {
        return Err(Error::UnreadableDicom(
            "no DICOM marker and no SOP Class UID".to_owned(),
        ));
    }

    data_set
        .with_meta(FileMetaTableBuilder::new().transfer_syntax(transfer_syntax.uid()))
        .map_err(|error| Error::UnreadableDicom(error.to_string()))
}

/// Explicit VR encodes two uppercase letters (the VR) after the 4-byte tag (PS3.5 §7.1.2).
fn looks_like_explicit_vr(bytes: &[u8]) -> bool {
    bytes
        .get(4..6)
        .is_some_and(|vr| vr.iter().all(u8::is_ascii_uppercase))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testdata(path: &str) -> Vec<u8> {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&full).unwrap_or_else(|error| panic!("cannot read {full}: {error}"))
    }

    #[test]
    fn reads_a_file_with_preamble() {
        let file = read_dicom(&testdata("pydicom/MR_small.dcm")).expect("readable");
        assert!(file.has_pixel_data());
        assert_eq!(file.number_of_frames(), 1);
        assert_eq!(file.transfer_syntax(), "1.2.840.10008.1.2.1");
    }

    #[test]
    fn reads_the_number_of_frames_of_a_multiframe_image() {
        let file = read_dicom(&testdata("pydicom/emri_small.dcm")).expect("readable");
        assert_eq!(file.number_of_frames(), 10);
    }

    #[test]
    fn object_without_pixel_data_has_no_frame() {
        let file = read_dicom(&testdata("pydicom/test-SR.dcm")).expect("readable");
        assert!(!file.has_pixel_data());
        assert_eq!(file.number_of_frames(), 0);
    }

    #[test]
    fn reads_a_data_set_without_file_meta() {
        let file = read_dicom(&testdata("pydicom/rtstruct.dcm")).expect("readable");
        assert_eq!(file.transfer_syntax(), "1.2.840.10008.1.2");
        let modality = file.object().element(tags::MODALITY).expect("modality");
        assert_eq!(modality.to_str().expect("text"), "RTSTRUCT");
    }

    #[test]
    fn rejects_bytes_that_are_not_dicom() {
        let error = read_dicom(b"%PDF-1.4 this is not a DICOM file at all, just some text.");
        assert!(matches!(error, Err(Error::UnreadableDicom(_))));
        assert!(read_dicom(&[]).is_err());
    }
}
