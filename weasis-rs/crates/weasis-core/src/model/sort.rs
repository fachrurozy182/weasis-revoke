//! Ordering of the images of a series (the stack shown by the viewer).
//!
//! NOTE: the Java comparators treat a missing value as equal to anything, which is not a total
//! order; here images without the value are kept after the others, in their original order.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/SortSeriesStack.java`
//! and `.../utils/DicomMediaUtils.java` (`computeSlicePosition`).

use std::cmp::Ordering;

use dicom_dictionary_std::tags;

use super::{Attributes, InstanceNode};

/// Criteria offered to sort a stack, in the order of the Java menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Instance Number (the default).
    InstanceNumber,
    /// Position along the image normal (Image Position · normal of Image Orientation).
    SlicePosition,
    /// Slice Location.
    SliceLocation,
    /// Content Time.
    ContentTime,
    /// Acquisition Time.
    AcquisitionTime,
    /// Diffusion b-value.
    DiffusionBValue,
}

impl SortKey {
    /// All criteria.
    pub const ALL: [Self; 6] = [
        Self::InstanceNumber,
        Self::SlicePosition,
        Self::SliceLocation,
        Self::ContentTime,
        Self::AcquisitionTime,
        Self::DiffusionBValue,
    ];

    /// Value of an instance for this criterion.
    pub fn value(self, attributes: &Attributes) -> Option<f64> {
        match self {
            Self::InstanceNumber => number(attributes.get(tags::INSTANCE_NUMBER)?),
            Self::SlicePosition => slice_position(attributes),
            Self::SliceLocation => number(attributes.get(tags::SLICE_LOCATION)?),
            Self::ContentTime => seconds_of_day(attributes.get(tags::CONTENT_TIME)?),
            Self::AcquisitionTime => seconds_of_day(attributes.get(tags::ACQUISITION_TIME)?),
            Self::DiffusionBValue => number(attributes.get(tags::DIFFUSION_B_VALUE)?),
        }
    }
}

/// Sorts instances by a criterion (stable).
pub fn sort_instances(instances: &mut [InstanceNode], key: SortKey) {
    instances.sort_by(|a, b| compare(key.value(&a.attributes), key.value(&b.attributes)));
}

/// Ascending, with missing values last.
fn compare(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Signed distance of the image plane along its normal; Slice Location when the orientation
/// is missing (e.g. NM images), as in `DicomMediaUtils.computeSlicePosition`.
pub fn slice_position(attributes: &Attributes) -> Option<f64> {
    let position = numbers(attributes.get(tags::IMAGE_POSITION_PATIENT)?);
    let orientation = attributes
        .get(tags::IMAGE_ORIENTATION_PATIENT)
        .map(numbers)
        .filter(|values| values.len() == 6);
    match (position.as_slice(), orientation) {
        ([x, y, z], Some(o)) => {
            let normal = [
                o[1] * o[5] - o[2] * o[4],
                o[2] * o[3] - o[0] * o[5],
                o[0] * o[4] - o[1] * o[3],
            ];
            Some(normal[0] * x + normal[1] * y + normal[2] * z)
        }
        _ => number(attributes.get(tags::SLICE_LOCATION)?),
    }
}

/// A decimal or integer string (DS, IS).
fn number(text: &str) -> Option<f64> {
    text.trim().parse().ok().filter(|v: &f64| v.is_finite())
}

/// The values of a multi-valued DS, separated by `\` (PS3.5 §6.4).
fn numbers(text: &str) -> Vec<f64> {
    text.split('\\').filter_map(number).collect()
}

/// Seconds since midnight of a TM value `HHMMSS.FFFFFF` (PS3.5 §6.2), also accepting the
/// ACR-NEMA form `HH:MM:SS.FFFFFF`.
fn seconds_of_day(text: &str) -> Option<f64> {
    let compact: String = text.trim().chars().filter(|c| *c != ':').collect();
    let (whole, fraction) = compact.split_once('.').unwrap_or((&compact, ""));
    let field = |range: std::ops::Range<usize>| -> Option<f64> {
        match whole.get(range) {
            Some(digits) => digits.parse::<u32>().ok().map(f64::from),
            None => Some(0.0),
        }
    };
    if whole.len() < 2 || !whole.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let fraction: f64 = if fraction.is_empty() {
        0.0
    } else {
        format!("0.{fraction}").parse().ok()?
    };
    Some(field(0..2)? * 3600.0 + field(2..4)? * 60.0 + field(4..6)? + fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(uid: &str, values: &[(dicom_object::Tag, &str)]) -> InstanceNode {
        let mut attributes = Attributes::new();
        for (tag, value) in values {
            attributes.set(*tag, *value);
        }
        InstanceNode {
            sop_instance_uid: uid.to_owned(),
            kind: crate::dicom::ObjectKind::Image,
            attributes,
        }
    }

    fn order(instances: &[InstanceNode]) -> Vec<&str> {
        instances
            .iter()
            .map(|i| i.sop_instance_uid.as_str())
            .collect()
    }

    #[test]
    fn instance_numbers_sort_numerically_with_missing_ones_last() {
        let mut stack = vec![
            instance("a", &[(tags::INSTANCE_NUMBER, "10")]),
            instance("b", &[]),
            instance("c", &[(tags::INSTANCE_NUMBER, "2")]),
        ];
        sort_instances(&mut stack, SortKey::InstanceNumber);
        assert_eq!(order(&stack), ["c", "a", "b"]);
    }

    #[test]
    fn slice_position_follows_the_image_normal() {
        // Axial images: the normal is +Z, so the position is the Z coordinate.
        let axial = "1\\0\\0\\0\\1\\0";
        let mut stack = vec![
            instance(
                "high",
                &[
                    (tags::IMAGE_POSITION_PATIENT, "-32\\-32\\10"),
                    (tags::IMAGE_ORIENTATION_PATIENT, axial),
                ],
            ),
            instance(
                "low",
                &[
                    (tags::IMAGE_POSITION_PATIENT, "-32\\-32\\-5.5"),
                    (tags::IMAGE_ORIENTATION_PATIENT, axial),
                ],
            ),
        ];
        sort_instances(&mut stack, SortKey::SlicePosition);
        assert_eq!(order(&stack), ["low", "high"]);
        assert_eq!(slice_position(&stack[0].attributes), Some(-5.5));
    }

    #[test]
    fn slice_location_replaces_a_missing_orientation() {
        let nm = instance(
            "nm",
            &[
                (tags::IMAGE_POSITION_PATIENT, "0\\0\\0"),
                (tags::SLICE_LOCATION, "7.5"),
            ],
        );
        assert_eq!(slice_position(&nm.attributes), Some(7.5));
    }

    #[test]
    fn times_are_read_in_both_forms() {
        assert_eq!(seconds_of_day("103015.5"), Some(37815.5));
        assert_eq!(seconds_of_day("10:30:15"), Some(37815.0));
        assert_eq!(seconds_of_day("1030"), Some(37800.0));
        assert_eq!(seconds_of_day("noon"), None);
    }
}
