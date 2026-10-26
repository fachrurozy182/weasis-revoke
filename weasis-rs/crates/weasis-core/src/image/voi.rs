//! VOI transformation: from modality values to display intensities (DICOM PS3.3 §C.11.2).
//!
//! The window (center/width) selects the range of values shown. The LUT shape decides how
//! values inside the window are spread: the DICOM functions LINEAR, LINEAR_EXACT and SIGMOID,
//! the Weasis functions SIGMOID_NORM, LOG and LOG_INV, or a VOI LUT Sequence stretched over
//! the window. Results are normalized to `0.0..=1.0`.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/util/LookupTableUtils.java`
//! (`createVoiLut`), `.../img/lut/VoiLutModule.java`, `.../img/lut/PresetWindowLevel.java`
//! (`buildPresetFromLutData`) and `weasis-core-img/.../opencv/op/lut/LutShape.java`.

use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;

use super::LookupTable;
use crate::dicom::pixel::ImageDescriptor;
use crate::dicom::{DicomFile, frame_macro, item_text};

/// Steepness of the Weasis sigmoid and logarithmic shapes (`SIGMOID_FACTOR`, `LOG_EXP_FACTOR`
/// divided by `FACTOR_DIVISOR` in the Java code).
const LOG_EXP_FACTOR: f64 = 2.0;

/// How values inside the window are mapped to intensities.
#[derive(Debug, Clone, PartialEq)]
pub enum LutShape {
    /// DICOM LINEAR (PS3.3 §C.11.2.1.2.1), the default.
    Linear,
    /// DICOM LINEAR_EXACT (PS3.3 §C.11.2.1.3.2).
    LinearExact,
    /// DICOM SIGMOID (PS3.3 §C.11.2.1.3.1).
    Sigmoid,
    /// Sigmoid rescaled so that the window bounds are black and white (Weasis).
    SigmoidNormalized,
    /// Logarithmic (Weasis).
    Log,
    /// Exponential, the inverse of `Log` (Weasis).
    LogInverse,
    /// VOI LUT Sequence stretched over the window.
    Sequence(LookupTable),
}

impl LutShape {
    /// Shape named by VOI LUT Function (0028,1056); unknown values fall back to LINEAR.
    pub fn from_voi_lut_function(function: &str) -> Self {
        match function.trim() {
            "LINEAR_EXACT" => Self::LinearExact,
            "SIGMOID" => Self::Sigmoid,
            _ => Self::Linear,
        }
    }
}

/// One window stored in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiWindow {
    /// Window Center.
    pub center: f64,
    /// Window Width.
    pub width: f64,
    /// Window Center & Width Explanation, if any.
    pub explanation: Option<String>,
}

/// VOI attributes of one frame: windows, function and VOI LUT Sequence tables.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiLut {
    /// Windows from Window Center/Width.
    pub windows: Vec<VoiWindow>,
    /// Shape from VOI LUT Function, applied to `windows`.
    pub function: LutShape,
    /// Tables of the VOI LUT Sequence with their LUT Explanation.
    pub tables: Vec<(LookupTable, Option<String>)>,
}

impl VoiLut {
    /// Reads the VOI attributes of `frame`, from the Frame VOI LUT functional group of enhanced
    /// objects or from the data set.
    pub fn from_file(file: &DicomFile, frame: u32, descriptor: &ImageDescriptor) -> Self {
        let object: &InMemDicomObject = file.object();
        match frame_macro(file, frame, tags::FRAME_VOILUT_SEQUENCE) {
            Some(group) => Self {
                tables: Self::from_item(object, descriptor.is_signed).tables,
                ..Self::from_item(group, descriptor.is_signed)
            },
            None => Self::from_item(object, descriptor.is_signed),
        }
    }

    /// Reads the VOI attributes of one item: a data set, a functional group or a Softcopy VOI
    /// LUT Sequence item of a Presentation State.
    pub fn from_item(item: &InMemDicomObject, signed_input: bool) -> Self {
        let numbers = |tag| -> Vec<f64> {
            item.element(tag)
                .ok()
                .and_then(|e| e.to_multi_float64().ok())
                .unwrap_or_default()
        };
        let explanations: Vec<String> = item_text(item, tags::WINDOW_CENTER_WIDTH_EXPLANATION)
            .map(|text| text.split('\\').map(str::to_owned).collect())
            .unwrap_or_default();

        let windows = numbers(tags::WINDOW_CENTER)
            .into_iter()
            .zip(numbers(tags::WINDOW_WIDTH))
            .enumerate()
            .filter(|(_, (_, width))| *width > 0.0)
            .map(|(index, (center, width))| VoiWindow {
                center,
                width,
                explanation: explanations
                    .get(index)
                    .filter(|e| !e.trim().is_empty())
                    .cloned(),
            })
            .collect();
        let function = item_text(item, tags::VOILUT_FUNCTION)
            .map(|text| LutShape::from_voi_lut_function(&text))
            .unwrap_or(LutShape::Linear);

        let tables = item
            .element(tags::VOILUT_SEQUENCE)
            .ok()
            .and_then(|element| element.items())
            .unwrap_or_default()
            .iter()
            .filter_map(|lut| {
                let table = LookupTable::from_item(lut, signed_input).ok()?;
                Some((table, item_text(lut, tags::LUT_EXPLANATION)))
            })
            .collect();
        Self {
            windows,
            function,
            tables,
        }
    }
}

/// Window (width, center) that covers the input range of a VOI LUT Sequence table.
pub fn sequence_window(table: &LookupTable) -> (f64, f64) {
    let entries = i64::try_from(table.entries.len()).unwrap_or(i64::MAX);
    let first = table.first_mapped_value as f64;
    let width = (entries.saturating_sub(1)) as f64;
    (width, first + width / 2.0)
}

/// Window, level and shape applied to a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiParams {
    /// Window width (at least 1).
    pub window: f64,
    /// Window center (level).
    pub level: f64,
    /// LUT shape.
    pub shape: LutShape,
    /// Whether high values are dark: MONOCHROME1 XOR the user's "invert" action.
    pub invert: bool,
}

impl VoiParams {
    /// Intensity of a modality value, from 0.0 (black) to 1.0 (white).
    pub fn intensity(&self, value: f64) -> f64 {
        let window = self.window.max(1.0);
        let center = self.level;
        let y = match &self.shape {
            LutShape::Linear => linear(value, center - 0.5, window - 1.0),
            LutShape::LinearExact => linear(value, center, window),
            LutShape::Sigmoid => 1.0 / (1.0 + (-4.0 * (value - center) / window).exp()),
            LutShape::SigmoidNormalized => {
                let sigmoid = |x: f64| 1.0 / (1.0 + (-4.0 * (x - center) / window).exp());
                normalized(sigmoid, value, center, window)
            }
            LutShape::Log => {
                let log = |x: f64| (LOG_EXP_FACTOR * (1.0 + (x - center) / window)).ln();
                normalized(log, value, center, window)
            }
            LutShape::LogInverse => {
                let exponential = |x: f64| (LOG_EXP_FACTOR * (x - center) / window).exp();
                normalized(exponential, value, center, window)
            }
            LutShape::Sequence(table) => sequence(table, value, center, window),
        };
        let y = if y.is_nan() { 0.0 } else { y.clamp(0.0, 1.0) };
        if self.invert { 1.0 - y } else { y }
    }
}

/// Linear ramp across `[center - width / 2, center + width / 2]`; a zero width is a threshold.
fn linear(value: f64, center: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return if value <= center { 0.0 } else { 1.0 };
    }
    (value - (center - width / 2.0)) / width
}

/// Rescales `function` so that the window bounds map to 0.0 and 1.0.
fn normalized(function: impl Fn(f64) -> f64, value: f64, center: f64, window: f64) -> f64 {
    let low = function(center - window / 2.0);
    let high = function(center + window / 2.0);
    let range = (high - low).abs();
    if range == 0.0 || !range.is_finite() {
        return 0.0;
    }
    (function(value) - low) / range
}

/// VOI LUT Sequence stretched over the window, with linear interpolation between entries.
fn sequence(table: &LookupTable, value: f64, center: f64, window: f64) -> f64 {
    let last = table.entries.len().saturating_sub(1);
    let low = center - window / 2.0;
    let position = if value <= low {
        0.0
    } else if value > center + window / 2.0 {
        last as f64
    } else {
        (value - low) * (last as f64 / window)
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (below, above) = (
        position.floor() as usize,
        (position.ceil() as usize).min(last),
    );
    let entry = |index: usize| f64::from(table.entries.get(index).copied().unwrap_or(0));
    let interpolated = if above == below {
        entry(below)
    } else {
        entry(below) + (position - below as f64) * (entry(above) - entry(below))
    };
    // NOTE: as in the Java viewer, the output is divided by the range of the table without
    // subtracting its minimum (tables start at 0 in practice).
    let (min, max) = table.output_range();
    let range = f64::from(max - min);
    if range == 0.0 {
        0.0
    } else {
        interpolated.round() / range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(shape: LutShape) -> VoiParams {
        VoiParams {
            window: 400.0,
            level: 40.0,
            shape,
            invert: false,
        }
    }

    #[test]
    fn linear_follows_the_dicom_formula() {
        // PS3.3 C.11.2.1.2.1: values <= c - 0.5 - (w - 1) / 2 are black, values above
        // c - 0.5 + (w - 1) / 2 are white.
        let voi = params(LutShape::Linear);
        assert_eq!(voi.intensity(40.0 - 0.5 - 399.0 / 2.0), 0.0);
        assert_eq!(voi.intensity(40.0 - 0.5 + 399.0 / 2.0), 1.0);
        assert!((voi.intensity(39.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn linear_exact_spans_exactly_the_window() {
        let voi = params(LutShape::LinearExact);
        assert_eq!(voi.intensity(-160.0), 0.0);
        assert_eq!(voi.intensity(240.0), 1.0);
        assert_eq!(voi.intensity(40.0), 0.5);
    }

    #[test]
    fn sigmoid_is_half_at_the_center() {
        let voi = params(LutShape::Sigmoid);
        assert!((voi.intensity(40.0) - 0.5).abs() < 1e-12);
        assert!(voi.intensity(-160.0) < 0.12 && voi.intensity(240.0) > 0.88);
    }

    #[test]
    fn normalized_shapes_reach_black_and_white_at_the_window_bounds() {
        for shape in [
            LutShape::SigmoidNormalized,
            LutShape::Log,
            LutShape::LogInverse,
        ] {
            let voi = params(shape.clone());
            assert!(voi.intensity(-160.0).abs() < 1e-9, "{shape:?}");
            assert!((voi.intensity(240.0) - 1.0).abs() < 1e-9, "{shape:?}");
        }
    }

    #[test]
    fn invert_mirrors_the_intensity() {
        let mut voi = params(LutShape::LinearExact);
        voi.invert = true;
        assert_eq!(voi.intensity(-160.0), 1.0);
    }

    #[test]
    fn sequence_window_maps_the_table_one_to_one() {
        let table = LookupTable {
            first_mapped_value: 0,
            bits_per_entry: 16,
            entries: vec![0, 100, 200],
        };
        let (window, level) = sequence_window(&table);
        assert_eq!((window, level), (2.0, 1.0));
        let voi = VoiParams {
            window,
            level,
            shape: LutShape::Sequence(table),
            invert: false,
        };
        assert_eq!(voi.intensity(1.0), 0.5);
        assert_eq!(voi.intensity(2.0), 1.0);
    }
}
