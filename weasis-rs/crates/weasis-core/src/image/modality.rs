//! Modality LUT: from stored values to modality values such as Hounsfield units
//! (DICOM PS3.3 §C.11.1), and the pixel padding range (PS3.3 §C.7.5.1.1.2).
//!
//! The transform is either Rescale Slope/Intercept or a Modality LUT Sequence. For enhanced
//! multi-frame objects it comes from the Pixel Value Transformation functional group of each
//! frame.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/lut/ModalityLutModule.java`,
//! `.../img/util/RescaleUtils.java` (padding) and `.../img/DicomImageAdapter.java`.

use std::ops::RangeInclusive;

use dicom_dictionary_std::tags;

use super::LookupTable;
use crate::dicom::pixel::ImageDescriptor;
use crate::dicom::{DicomFile, first_item, frame_macro, item_f64, item_text};

/// Modalities for which a Modality LUT Sequence is ignored when the Pixel Intensity
/// Relationship is LOG or DISP: their values are already display values.
const LUT_RESTRICTED_MODALITIES: [&str; 2] = ["XA", "XRF"];

/// How stored values become modality values.
#[derive(Debug, Clone, PartialEq)]
pub enum ModalityTransform {
    /// `value × slope + intercept`; identity when slope is 1 and intercept 0.
    Rescale {
        /// Rescale Slope.
        slope: f64,
        /// Rescale Intercept.
        intercept: f64,
    },
    /// Modality LUT Sequence.
    Table(LookupTable),
}

/// The modality transform of one frame and its pixel padding.
#[derive(Debug, Clone, PartialEq)]
pub struct ModalityLut {
    /// Transform of stored values.
    pub transform: ModalityTransform,
    /// Rescale Type, e.g. "HU" (defaults to "US", unspecified).
    pub rescale_type: Option<String>,
    /// Stored values that are padding (outside the patient), not real data.
    pub padding: Option<RangeInclusive<i64>>,
}

impl ModalityLut {
    /// Reads the modality transform of `frame`.
    pub fn from_file(file: &DicomFile, frame: u32, descriptor: &ImageDescriptor) -> Self {
        let transform = modality_lut_sequence(file, descriptor)
            .map(ModalityTransform::Table)
            .unwrap_or_else(|| rescale(file, frame));
        let rescale_type = frame_macro(file, frame, tags::PIXEL_VALUE_TRANSFORMATION_SEQUENCE)
            .and_then(|item| item_text(item, tags::RESCALE_TYPE))
            .or_else(|| file.text(tags::RESCALE_TYPE));
        Self {
            transform,
            rescale_type,
            padding: padding_range(file, descriptor),
        }
    }

    /// Modality value of a stored value.
    pub fn apply(&self, stored: f64) -> f64 {
        match &self.transform {
            ModalityTransform::Rescale { slope, intercept } => stored * slope + intercept,
            ModalityTransform::Table(table) => f64::from(table.lookup(stored)),
        }
    }

    /// Whether a stored value is pixel padding.
    pub fn is_padding(&self, stored: f64) -> bool {
        self.padding.as_ref().is_some_and(|range| {
            #[allow(clippy::cast_possible_truncation)]
            let value = stored as i64;
            range.contains(&value)
        })
    }

    /// Whether the transform leaves values unchanged.
    pub fn is_identity(&self) -> bool {
        matches!(
            self.transform,
            ModalityTransform::Rescale { slope, intercept } if slope == 1.0 && intercept == 0.0
        )
    }
}

/// Rescale Slope/Intercept: from the frame's functional group, else from the data set.
fn rescale(file: &DicomFile, frame: u32) -> ModalityTransform {
    let from_group =
        frame_macro(file, frame, tags::PIXEL_VALUE_TRANSFORMATION_SEQUENCE).map(|item| {
            (
                item_f64(item, tags::RESCALE_SLOPE),
                item_f64(item, tags::RESCALE_INTERCEPT),
            )
        });
    let (slope, intercept) = from_group.unwrap_or_else(|| {
        let number = |tag| file.object().element(tag).ok()?.to_float64().ok();
        (number(tags::RESCALE_SLOPE), number(tags::RESCALE_INTERCEPT))
    });
    // NOTE: a zero slope would erase the image; like dcm4che, it is treated as missing.
    ModalityTransform::Rescale {
        slope: slope.filter(|s| *s != 0.0).unwrap_or(1.0),
        intercept: intercept.unwrap_or(0.0),
    }
}

/// Modality LUT Sequence, unless the values are already display values (XA/XRF LOG or DISP).
fn modality_lut_sequence(file: &DicomFile, descriptor: &ImageDescriptor) -> Option<LookupTable> {
    let item = first_item(file.object(), tags::MODALITY_LUT_SEQUENCE)?;
    let modality = file.text(tags::MODALITY).unwrap_or_default();
    let relationship = file
        .text(tags::PIXEL_INTENSITY_RELATIONSHIP)
        .unwrap_or_default()
        .to_uppercase();
    let is_display_value = LUT_RESTRICTED_MODALITIES.contains(&modality.as_str())
        && matches!(relationship.as_str(), "LOG" | "DISP");
    if is_display_value {
        return None;
    }
    LookupTable::from_item(item, descriptor.is_signed).ok()
}

/// Pixel Padding Value, with Pixel Padding Range Limit when present.
fn padding_range(file: &DicomFile, descriptor: &ImageDescriptor) -> Option<RangeInclusive<i64>> {
    let read = |tag| {
        let value = file.object().element(tag).ok()?.to_int::<i64>().ok()?;
        Some(as_stored_value(value, descriptor))
    };
    let value = read(tags::PIXEL_PADDING_VALUE)?;
    let limit = read(tags::PIXEL_PADDING_RANGE_LIMIT).unwrap_or(value);
    Some(value.min(limit)..=value.max(limit))
}

/// The padding attributes are US or SS like the pixels (PS3.3 §C.7.5.1.1.2), but some files
/// write a negative value as US: it is read back as a signed number of Bits Allocated.
fn as_stored_value(value: i64, descriptor: &ImageDescriptor) -> i64 {
    let bits = u32::from(descriptor.bits_allocated.clamp(1, 32));
    let half = 1i64 << (bits - 1);
    if descriptor.is_signed && value >= half {
        value - (half << 1)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::pixel::decode_frame;
    use crate::dicom::read_dicom;

    fn modality_lut(path: &str) -> (DicomFile, ModalityLut) {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        let file = read_dicom(&bytes).expect("readable");
        let descriptor = ImageDescriptor::from_file(&file).expect("image");
        let lut = ModalityLut::from_file(&file, 0, &descriptor);
        (file, lut)
    }

    /// Sum of the modality values of the first frame.
    fn modality_sum(file: &DicomFile, lut: &ModalityLut) -> f64 {
        let frame = decode_frame(file, 0).expect("decodable");
        (0..frame.samples.len())
            .filter_map(|i| frame.samples.get_f64(i))
            .map(|value| lut.apply(value))
            .sum()
    }

    #[test]
    fn rescale_slope_and_intercept_give_hounsfield_units() {
        let (file, lut) = modality_lut("pydicom/CT_small.dcm");
        assert_eq!(
            lut.transform,
            ModalityTransform::Rescale {
                slope: 1.0,
                intercept: -1024.0
            }
        );
        assert_eq!(lut.rescale_type, None);
        // Reference: pydicom apply_modality_lut.
        assert_eq!(modality_sum(&file, &lut), -1_950_906.0);
    }

    #[test]
    fn enhanced_ct_takes_the_rescale_from_its_functional_groups() {
        let (_, lut) = modality_lut("pydicom/eCT_Supplemental.dcm");
        assert_eq!(
            lut.transform,
            ModalityTransform::Rescale {
                slope: 1.0,
                intercept: -1024.0
            }
        );
    }

    #[test]
    fn modality_lut_sequence_is_applied() {
        let (file, lut) = modality_lut("pydicom/mlut_18.dcm");
        assert!(matches!(lut.transform, ModalityTransform::Table(_)));
        assert_eq!(modality_sum(&file, &lut), 8_697_190_668.0);
    }

    #[test]
    fn padding_written_as_unsigned_is_read_as_a_signed_stored_value() {
        let (_, lut) = modality_lut("weasis-dicom-tools/mono2-CT-16bit.dcm");
        assert_eq!(lut.padding, Some(-2000..=-2000));
        assert!(lut.is_padding(-2000.0));
        assert!(!lut.is_padding(0.0));
    }

    #[test]
    fn missing_rescale_is_the_identity() {
        let (_, lut) = modality_lut("pydicom/MR_small.dcm");
        assert!(lut.is_identity());
    }
}
