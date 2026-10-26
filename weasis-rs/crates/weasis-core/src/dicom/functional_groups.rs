//! Functional group macros of enhanced multi-frame objects (DICOM PS3.3 §C.7.6.16).
//!
//! An attribute of a frame can be in the Per-frame Functional Groups Sequence (item of that
//! frame), in the Shared Functional Groups Sequence, or, for classic objects, at the top level of
//! the data set. Callers look up the macro sequence they need, e.g. Pixel Value Transformation.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/.../utils/DicomMediaUtils.java`
//! (`writePerFrameFunctionalGroupsSequence`) and `DicomMediaIO.writeSharedFunctionalGroupsSequence`.

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};

use super::DicomFile;

/// The first item of the macro sequence `macro_tag` that applies to `frame`: per-frame first,
/// then shared. `None` for classic objects or when the macro is absent.
pub fn frame_macro(file: &DicomFile, frame: u32, macro_tag: Tag) -> Option<&InMemDicomObject> {
    let object = file.object();
    let per_frame = object
        .element(tags::PER_FRAME_FUNCTIONAL_GROUPS_SEQUENCE)
        .ok()
        .and_then(|element| element.items())
        .and_then(|frames| frames.get(usize::try_from(frame).ok()?))
        .and_then(|group| first_item(group, macro_tag));
    per_frame.or_else(|| {
        object
            .element(tags::SHARED_FUNCTIONAL_GROUPS_SEQUENCE)
            .ok()
            .and_then(|element| element.items())
            .and_then(|groups| groups.first())
            .and_then(|group| first_item(group, macro_tag))
    })
}

/// First item of a sequence attribute of `object`.
pub fn first_item(object: &InMemDicomObject, sequence: Tag) -> Option<&InMemDicomObject> {
    object.element(sequence).ok()?.items()?.first()
}

/// A number of an item, `None` when absent or not numeric.
pub fn item_f64(item: &InMemDicomObject, tag: Tag) -> Option<f64> {
    item.element(tag).ok()?.to_float64().ok()
}

/// Text of an item without padding, `None` when absent or empty.
pub fn item_text(item: &InMemDicomObject, tag: Tag) -> Option<String> {
    let value = item.element(tag).ok()?.to_str().ok()?;
    let trimmed = value.trim_matches([' ', '\0']);
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}
