//! Ultrasound regions: areas of an ultrasound image with their own physical calibration
//! (US Region Calibration Module, PS3.3 §C.8.5.5).
//!
//! An ultrasound image has no Pixel Spacing; distances are measured with the calibration of the
//! region. Like the Java viewer, a single spatial calibration (in cm) is used for the whole
//! image when all the spatial regions agree.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/utils/Ultrasound.java`
//! and `DicomImageElement.initPixelValueUnit`.

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};

use super::{DicomFile, item_f64};

/// Physical Units X/Y Direction value for centimeters.
const UNITS_CM: u16 = 3;

/// One item of the Sequence of Ultrasound Regions.
#[derive(Debug, Clone, PartialEq)]
pub struct UltrasoundRegion {
    /// Region Location Min X0, Min Y0, Max X1, Max Y1 (pixels, inclusive).
    pub bounds: [u32; 4],
    /// Region Spatial Format (1 = 2D, 2 = M-mode, 3 = spectral, ...).
    pub spatial_format: Option<u16>,
    /// Region Data Type (1 = tissue, 2 = color flow, 3 = PW spectral Doppler, ...).
    pub data_type: Option<u16>,
    /// Physical Units X Direction.
    pub units_x: Option<u16>,
    /// Physical Units Y Direction.
    pub units_y: Option<u16>,
    /// Physical Delta X: physical size of one pixel along X.
    pub delta_x: Option<f64>,
    /// Physical Delta Y.
    pub delta_y: Option<f64>,
    /// Reference Pixel X0 and Y0: the pixel of the physical origin.
    pub reference_pixel: Option<(i32, i32)>,
    /// Reference Pixel Physical Value X and Y.
    pub reference_value: Option<(f64, f64)>,
}

impl UltrasoundRegion {
    fn read(item: &InMemDicomObject) -> Self {
        let int = |tag: Tag| item.element(tag).ok().and_then(|e| e.to_int::<i64>().ok());
        let unsigned = |tag: Tag| int(tag).and_then(|v| u16::try_from(v).ok());
        let pixel = |tag: Tag| int(tag).and_then(|v| u32::try_from(v).ok()).unwrap_or(0);
        let reference_pixel = int(tags::REFERENCE_PIXEL_X0)
            .zip(int(tags::REFERENCE_PIXEL_Y0))
            .and_then(|(x, y)| Some((i32::try_from(x).ok()?, i32::try_from(y).ok()?)));
        Self {
            bounds: [
                pixel(tags::REGION_LOCATION_MIN_X0),
                pixel(tags::REGION_LOCATION_MIN_Y0),
                pixel(tags::REGION_LOCATION_MAX_X1),
                pixel(tags::REGION_LOCATION_MAX_Y1),
            ],
            spatial_format: unsigned(tags::REGION_SPATIAL_FORMAT),
            data_type: unsigned(tags::REGION_DATA_TYPE),
            units_x: unsigned(tags::PHYSICAL_UNITS_X_DIRECTION),
            units_y: unsigned(tags::PHYSICAL_UNITS_Y_DIRECTION),
            delta_x: item_f64(item, tags::PHYSICAL_DELTA_X),
            delta_y: item_f64(item, tags::PHYSICAL_DELTA_Y),
            reference_pixel,
            reference_value: item_f64(item, tags::REFERENCE_PIXEL_PHYSICAL_VALUE_X)
                .zip(item_f64(item, tags::REFERENCE_PIXEL_PHYSICAL_VALUE_Y)),
        }
    }

    /// The unit of both axes when they share one.
    pub fn units(&self) -> Option<u16> {
        self.units_x.filter(|&x| Some(x) == self.units_y)
    }

    /// Whether a pixel lies in the region.
    pub fn contains(&self, x: u32, y: u32) -> bool {
        let [x0, y0, x1, y1] = self.bounds;
        (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
    }
}

/// The regions of an image.
pub fn ultrasound_regions(file: &DicomFile) -> Vec<UltrasoundRegion> {
    file.object()
        .element(tags::SEQUENCE_OF_ULTRASOUND_REGIONS)
        .ok()
        .and_then(|element| element.items())
        .unwrap_or_default()
        .iter()
        .map(UltrasoundRegion::read)
        .collect()
}

/// Pixel size in cm (horizontal, vertical) shared by all the regions calibrated in cm; `None`
/// when there is none, when they differ, or when the pixels would not be square (the image would
/// be stretched).
pub fn unique_spatial_calibration(regions: &[UltrasoundRegion]) -> Option<(f64, f64)> {
    let mut spatial = regions.iter().filter(|r| r.units() == Some(UNITS_CM));
    let first = spatial.next()?;
    if spatial.any(|r| (r.delta_x, r.delta_y) != (first.delta_x, first.delta_y)) {
        return None;
    }
    let (x, y) = (first.delta_x?.abs(), first.delta_y?.abs());
    ((x - y).abs() <= 1e-9).then_some((x, y))
}

/// Name of a Physical Units X/Y Direction value (PS3.3 §C.8.5.5.1.15).
pub fn units_name(units: u16) -> &'static str {
    match units {
        1 => "Percent",
        2 => "dB",
        3 => "cm",
        4 => "seconds",
        5 => "hertz(seconds-1)",
        6 => "dB/seconds",
        7 => "cm/sec",
        8 => "cm2",
        9 => "cm2/sec",
        10 => "cm3",
        11 => "cm3/sec",
        12 => "degrees",
        _ => "None",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn region(units: u16, delta: (f64, f64)) -> UltrasoundRegion {
        UltrasoundRegion {
            bounds: [0, 0, 99, 99],
            spatial_format: Some(1),
            data_type: Some(1),
            units_x: Some(units),
            units_y: Some(units),
            delta_x: Some(delta.0),
            delta_y: Some(delta.1),
            reference_pixel: None,
            reference_value: None,
        }
    }

    #[test]
    fn regions_of_an_ultrasound_image_are_read() {
        let path = format!(
            "{}/../../testdata/synthetic/us-regions.dcm",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        let regions = ultrasound_regions(&read_dicom(&bytes).expect("readable"));
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].bounds, [0, 0, 159, 79]);
        assert_eq!(regions[1].units(), None, "seconds over cm/s");
        assert_eq!(unique_spatial_calibration(&regions), Some((0.025, 0.025)));
    }

    #[test]
    fn one_calibration_is_used_only_when_all_spatial_regions_agree() {
        let doppler = region(7, (0.5, 0.5));
        let same = [
            region(3, (-0.02, 0.02)),
            region(3, (-0.02, 0.02)),
            doppler.clone(),
        ];
        assert_eq!(unique_spatial_calibration(&same), Some((0.02, 0.02)));
        let different = [region(3, (0.02, 0.02)), region(3, (0.03, 0.03))];
        assert_eq!(unique_spatial_calibration(&different), None);
        assert_eq!(
            unique_spatial_calibration(&[region(3, (0.02, 0.04))]),
            None,
            "not square"
        );
        assert_eq!(unique_spatial_calibration(&[doppler]), None);
    }

    #[test]
    fn a_region_contains_its_bounds() {
        let r = region(3, (0.1, 0.1));
        assert!(r.contains(0, 99));
        assert!(!r.contains(100, 0));
        assert_eq!(units_name(3), "cm");
    }
}
