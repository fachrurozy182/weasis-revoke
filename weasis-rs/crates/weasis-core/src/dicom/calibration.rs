//! Physical size of a pixel, used to measure in mm instead of pixels.
//!
//! The first source found is used: Pixel Spacing; for projection radiography Imager Pixel
//! Spacing (at the detector), corrected by the radiographic magnification when it is known;
//! Nominal Scanned Pixel Spacing; for ultrasound the spatial calibration of its regions. Without
//! any, distances are in pixels and Pixel Aspect Ratio may still stretch the image.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomImageElement.java`
//! (`initPixelConfiguration`, `getMagnifiedPixelSpacing`, `initPixelValueUnit`,
//! `initPixelAspectRatio`).

use dicom_dictionary_std::tags;
use dicom_object::Tag;

use super::{DicomFile, ultrasound_regions, unique_spatial_calibration};
use crate::image::Unit;

/// Modalities whose Imager Pixel Spacing is corrected by the magnification factor.
const PROJECTION_MODALITIES: [&str; 7] = ["CR", "DX", "IO", "MG", "PX", "RF", "XA"];

/// Size of a pixel and where it comes from.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelCalibration {
    /// Horizontal and vertical size of a pixel, in `unit` (relative sizes for pixels).
    pub size: (f64, f64),
    /// Unit of the size.
    pub unit: Unit,
    /// How the size was obtained, e.g. "At Detector", "Magnified" (shown with measurements).
    pub description: Option<String>,
}

impl PixelCalibration {
    /// Calibration of an image.
    pub fn from_file(file: &DicomFile) -> Self {
        let modality = file.text(tags::MODALITY).unwrap_or_default();
        if let Some((row, column)) = spacing(file, tags::PIXEL_SPACING) {
            return Self::millimeters(
                row,
                column,
                file.text(tags::PIXEL_SPACING_CALIBRATION_DESCRIPTION),
            );
        }
        if let Some((row, column)) = spacing(file, tags::IMAGER_PIXEL_SPACING) {
            let factor = PROJECTION_MODALITIES
                .contains(&modality.as_str())
                .then(|| magnification(file))
                .flatten();
            return match factor {
                Some(factor) => {
                    Self::millimeters(row / factor, column / factor, Some("Magnified".to_owned()))
                }
                None => Self::millimeters(row, column, Some("At Detector".to_owned())),
            };
        }
        if let Some((row, column)) = spacing(file, tags::NOMINAL_SCANNED_PIXEL_SPACING) {
            return Self::millimeters(row, column, Some("At scanner".to_owned()));
        }
        if modality == "US"
            && let Some((x, y)) = unique_spatial_calibration(&ultrasound_regions(file))
        {
            return Self {
                size: (x, y),
                unit: Unit::Centimeter,
                description: None,
            };
        }
        Self {
            size: aspect_ratio(file),
            unit: Unit::Pixel,
            description: None,
        }
    }

    /// Pixel Spacing is (row spacing, column spacing): vertical then horizontal.
    fn millimeters(row: f64, column: f64, description: Option<String>) -> Self {
        Self {
            size: (column, row),
            unit: Unit::Millimeter,
            description,
        }
    }

    /// Whether distances can be given in physical units.
    pub fn is_physical(&self) -> bool {
        self.unit != Unit::Pixel
    }
}

/// A pair of positive spacings.
///
/// NOTE: some files store negative spacings; like the Java viewer, they are ignored.
fn spacing(file: &DicomFile, tag: Tag) -> Option<(f64, f64)> {
    let values = file.object().element(tag).ok()?.to_multi_float64().ok()?;
    match values[..] {
        [row, column] if row > 0.0 && column > 0.0 => Some((row, column)),
        _ => None,
    }
}

/// Estimated Radiographic Magnification Factor, else Distance Source to Detector / to Patient.
fn magnification(file: &DicomFile) -> Option<f64> {
    let number = |tag: Tag| file.object().element(tag).ok()?.to_float64().ok();
    number(tags::ESTIMATED_RADIOGRAPHIC_MAGNIFICATION_FACTOR)
        .or_else(|| {
            Some(
                number(tags::DISTANCE_SOURCE_TO_DETECTOR)?
                    / number(tags::DISTANCE_SOURCE_TO_PATIENT)?,
            )
        })
        .filter(|factor| factor.is_finite() && *factor > 0.0)
}

/// Relative pixel size from Pixel Aspect Ratio (vertical \ horizontal), the smaller side being 1.
fn aspect_ratio(file: &DicomFile) -> (f64, f64) {
    let ratio = file
        .object()
        .element(tags::PIXEL_ASPECT_RATIO)
        .ok()
        .and_then(|e| e.to_multi_int::<i32>().ok())
        .unwrap_or_default();
    match ratio[..] {
        [vertical, horizontal] if vertical > 0 && horizontal > 0 && vertical != horizontal => {
            let (v, h) = (f64::from(vertical), f64::from(horizontal));
            if h < v { (1.0, v / h) } else { (h / v, 1.0) }
        }
        _ => (1.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn calibration(path: &str) -> PixelCalibration {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        PixelCalibration::from_file(&read_dicom(&bytes).expect("readable"))
    }

    #[test]
    fn pixel_spacing_gives_millimeters() {
        let ct = calibration("synthetic/ct-series/CT001.dcm");
        assert_eq!((ct.size, ct.unit), ((2.0, 2.0), Unit::Millimeter));
        assert!(ct.is_physical());
    }

    #[test]
    fn imager_spacing_is_at_the_detector() {
        let cr = calibration("synthetic/cr-monochrome1.dcm");
        assert_eq!(cr.size, (0.2, 0.2));
        assert_eq!(cr.description.as_deref(), Some("At Detector"));
    }

    #[test]
    fn ultrasound_uses_its_region_in_centimeters() {
        let us = calibration("synthetic/us-regions.dcm");
        assert_eq!((us.size, us.unit), ((0.025, 0.025), Unit::Centimeter));
    }

    #[test]
    fn without_spacing_distances_are_in_pixels() {
        let xa = calibration("synthetic/xa-shutter.dcm");
        assert_eq!((xa.size, xa.unit), ((1.0, 1.0), Unit::Pixel));
    }
}
