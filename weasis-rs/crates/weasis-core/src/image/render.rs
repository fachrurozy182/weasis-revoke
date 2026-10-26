//! Rendering a grayscale frame to 8-bit display intensities:
//! stored value → modality LUT → VOI → 0..=255, with pixel padding shown black.
//!
//! For integer samples the whole chain is computed once per stored value of the frame's range
//! and applied as a table, which keeps window/level changes fast on large images.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/ImageRendering.java`
//! (`processIntegerDataForVoi`, `StoredValueRamp`).

use super::{ModalityLut, VoiParams};
use crate::dicom::pixel::{PixelFrame, Samples};

/// Largest stored value range rendered through a table (covers all 16-bit images).
const MAX_TABLE_LENGTH: i64 = 1 << 20;

/// 8-bit intensities of a grayscale frame, one per pixel.
pub fn render_gray(frame: &PixelFrame, modality: &ModalityLut, voi: &VoiParams) -> Vec<u8> {
    let intensity = |stored: f64| {
        if modality.is_padding(stored) {
            0
        } else {
            to_byte(voi.intensity(modality.apply(stored)))
        }
    };
    match integer_values(&frame.samples) {
        Some(values) => render_with_table(&values, intensity),
        None => (0..frame.samples.len())
            .filter_map(|index| frame.samples.get_f64(index))
            .map(intensity)
            .collect(),
    }
}

/// Integer samples as `i64`, `None` for float samples.
fn integer_values(samples: &Samples) -> Option<Vec<i64>> {
    let values = match samples {
        Samples::U8(values) => values.iter().map(|&v| i64::from(v)).collect(),
        Samples::U16(values) => values.iter().map(|&v| i64::from(v)).collect(),
        Samples::I16(values) => values.iter().map(|&v| i64::from(v)).collect(),
        Samples::U32(values) => values.iter().map(|&v| i64::from(v)).collect(),
        Samples::I32(values) => values.iter().map(|&v| i64::from(v)).collect(),
        Samples::F32(_) => return None,
    };
    Some(values)
}

/// Computes `intensity` once per value of the range, unless the range is very large.
fn render_with_table(values: &[i64], intensity: impl Fn(f64) -> u8) -> Vec<u8> {
    let (Some(&min), Some(&max)) = (values.iter().min(), values.iter().max()) else {
        return Vec::new();
    };
    #[allow(clippy::cast_precision_loss)]
    let to_f64 = |value: i64| value as f64;
    if max - min >= MAX_TABLE_LENGTH {
        return values.iter().map(|&v| intensity(to_f64(v))).collect();
    }
    let table: Vec<u8> = (min..=max).map(|v| intensity(to_f64(v))).collect();
    values
        .iter()
        .map(|&v| {
            let index = usize::try_from(v - min).unwrap_or(0);
            table.get(index).copied().unwrap_or(0)
        })
        .collect()
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(intensity: f64) -> u8 {
    // Intensities are clamped to 0..=1 by VoiParams, so the cast cannot truncate.
    (intensity * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::pixel::{ImageDescriptor, decode_frame};
    use crate::dicom::read_dicom;
    use crate::image::{LutShape, VoiLut};

    fn rendered_sum(path: &str, invert: bool) -> (usize, u64) {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        let file = read_dicom(&bytes).expect("readable");
        let descriptor = ImageDescriptor::from_file(&file).expect("image");
        let frame = decode_frame(&file, 0).expect("decodable");
        let modality = ModalityLut::from_file(&file, 0, &descriptor);
        let voi_lut = VoiLut::from_file(&file, 0, &descriptor);
        let window = voi_lut.windows.first().expect("window");
        let voi = VoiParams {
            window: window.width,
            level: window.center,
            shape: LutShape::Linear,
            invert,
        };
        let pixels = render_gray(&frame, &modality, &voi);
        (pixels.len(), pixels.iter().map(|&p| u64::from(p)).sum())
    }

    #[test]
    fn window_level_matches_the_dicom_linear_function() {
        // Reference: pydicom apply_windowing (first window, LINEAR) rescaled to 0..=255.
        let (count, sum) = rendered_sum("synthetic/ct-series/CT008.dcm", false);
        assert_eq!(count, 4096);
        assert!(
            sum.abs_diff(REFERENCE_CT008_SUM) <= count as u64,
            "sum {sum}"
        );
    }

    #[test]
    fn monochrome1_is_inverted() {
        let (count, sum) = rendered_sum("synthetic/cr-monochrome1.dcm", true);
        assert!(
            sum.abs_diff(REFERENCE_CR_INVERTED_SUM) <= count as u64,
            "sum {sum}"
        );
    }

    #[test]
    fn voi_lut_sequence_matches_the_table() {
        let full = format!(
            "{}/../../testdata/pydicom/vlut_04.dcm",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        let file = read_dicom(&bytes).expect("readable");
        let descriptor = ImageDescriptor::from_file(&file).expect("image");
        let (table, _) = VoiLut::from_file(&file, 0, &descriptor)
            .tables
            .into_iter()
            .next()
            .expect("VOI LUT Sequence");
        let (window, level) = crate::image::sequence_window(&table);
        let voi = VoiParams {
            window,
            level,
            shape: LutShape::Sequence(table),
            invert: false,
        };
        let modality = ModalityLut::from_file(&file, 0, &descriptor);
        let pixels = render_gray(&decode_frame(&file, 0).expect("decodable"), &modality, &voi);
        let sum: u64 = pixels.iter().map(|&p| u64::from(p)).sum();
        // Reference: pydicom apply_voi_lut, output / 65535 × 255.
        assert!(sum.abs_diff(33_772_018) <= pixels.len() as u64, "sum {sum}");
    }

    #[test]
    fn padding_pixels_are_black() {
        let modality = ModalityLut {
            transform: crate::image::ModalityTransform::Rescale {
                slope: 1.0,
                intercept: 0.0,
            },
            rescale_type: None,
            padding: Some(-2000..=-2000),
        };
        let voi = VoiParams {
            window: 100.0,
            level: -2000.0,
            shape: LutShape::Linear,
            invert: true,
        };
        let frame = PixelFrame {
            descriptor: ImageDescriptor {
                rows: 1,
                columns: 2,
                samples_per_pixel: 1,
                bits_allocated: 16,
                bits_stored: 16,
                is_signed: true,
                photometric_interpretation: "MONOCHROME1".to_owned(),
                is_planar: false,
            },
            samples: Samples::I16(vec![-2000, -1999]),
        };
        assert_eq!(render_gray(&frame, &modality, &voi)[0], 0);
    }

    /// pydicom `apply_windowing` of the first window, normalized to its output range, × 255.
    const REFERENCE_CT008_SUM: u64 = 276_624;
    /// Same, inverted (MONOCHROME1).
    const REFERENCE_CR_INVERTED_SUM: u64 = 2_089_472;
}
