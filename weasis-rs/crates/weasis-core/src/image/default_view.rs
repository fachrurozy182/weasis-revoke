//! A frame rendered as it first shows: color frames as they are, grayscale frames through the
//! modality LUT and the first preset of their list, MONOCHROME1 inverted; and its reduction to a
//! thumbnail.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/Thumbnail.java`
//! (`loadThumbnail`, `createThumbnail`) and `ImageElement.getRenderedImage`.

use std::sync::OnceLock;

use super::{
    LutShape, ModalityLut, PresetContext, PresetLayers, VoiLut, VoiParams, preset_list, render_gray,
};
use crate::dicom::DicomFile;
use crate::dicom::anatomy::AnatomicRegion;
use crate::dicom::pixel::{ImageDescriptor, PixelFrame, Samples, decode_frame, to_rgb};
use crate::error::Result;
use dicom_dictionary_std::tags;

/// An 8-bit RGBA image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Red, green, blue and alpha of each pixel, row by row.
    pub rgba: Vec<u8>,
}

/// `frame` (0-based) rendered with its default display.
pub fn render_default(file: &DicomFile, frame: u32) -> Result<RgbaImage> {
    let decoded = to_rgb(decode_frame(file, frame)?, file)?;
    let (width, height) = (decoded.descriptor.columns, decoded.descriptor.rows);
    let rgba = if decoded.descriptor.samples_per_pixel == 3 {
        color_rgba(&decoded)
    } else {
        let modality = ModalityLut::from_file(file, frame, &decoded.descriptor);
        let voi = default_voi(file, frame, &decoded, &modality);
        let gray = render_gray(&decoded, &modality, &voi);
        gray.iter().flat_map(|&v| [v, v, v, 255]).collect()
    };
    Ok(RgbaImage {
        width,
        height,
        rgba,
    })
}

/// The image reduced to fit in `max` × `max`, keeping its proportions; each pixel is the
/// average of the pixels it covers. Smaller images are kept as they are.
pub fn fit_within(image: &RgbaImage, max: u32) -> RgbaImage {
    let largest = image.width.max(image.height);
    if largest <= max || largest == 0 {
        return image.clone();
    }
    let scale = f64::from(max) / f64::from(largest);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let size = |length: u32| ((f64::from(length) * scale).round() as u32).max(1);
    let (width, height) = (size(image.width), size(image.height));
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        let (y0, y1) = span(y, height, image.height);
        for x in 0..width {
            let (x0, x1) = span(x, width, image.width);
            let mut sum = [0u32; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy as usize * image.width as usize + sx as usize) * 4;
                    for (total, value) in sum.iter_mut().zip(&image.rgba[at..at + 4]) {
                        *total += u32::from(*value);
                    }
                }
            }
            let count = (y1 - y0) * (x1 - x0);
            #[allow(clippy::cast_possible_truncation)]
            rgba.extend(sum.map(|total| ((total + count / 2) / count) as u8));
        }
    }
    RgbaImage {
        width,
        height,
        rgba,
    }
}

/// Source pixels covered by target pixel `index` of `target` pixels over `source` pixels.
fn span(index: u32, target: u32, source: u32) -> (u32, u32) {
    let start = u64::from(index) * u64::from(source) / u64::from(target);
    let end = (u64::from(index + 1) * u64::from(source)).div_ceil(u64::from(target));
    #[allow(clippy::cast_possible_truncation)]
    (start as u32, (end as u32).max(start as u32 + 1).min(source))
}

/// Interleaved RGB samples as RGBA, 16-bit samples reduced to their high byte.
fn color_rgba(frame: &PixelFrame) -> Vec<u8> {
    let shift = frame.descriptor.bits_stored.saturating_sub(8);
    let byte = |index: usize| {
        let value = frame.samples.get_f64(index).unwrap_or(0.0).max(0.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let value = (value as u32 >> shift).min(255) as u8;
        value
    };
    (0..frame.samples.len() / 3)
        .flat_map(|pixel| {
            [
                byte(pixel * 3),
                byte(pixel * 3 + 1),
                byte(pixel * 3 + 2),
                255,
            ]
        })
        .collect()
}

/// The first preset of the frame: its own window, else the configured default of its modality,
/// else the range of its values.
fn default_voi(
    file: &DicomFile,
    frame: u32,
    decoded: &PixelFrame,
    modality_lut: &ModalityLut,
) -> VoiParams {
    static PRESETS: OnceLock<PresetLayers> = OnceLock::new();
    let presets = PRESETS.get_or_init(|| PresetLayers::built_in().unwrap_or_default());
    let descriptor: &ImageDescriptor = &decoded.descriptor;
    let modality = file.text(tags::MODALITY).unwrap_or_default();
    let anatomy = AnatomicRegion::from_file(file);
    let context = PresetContext {
        modality: &modality,
        bits_stored: descriptor.bits_stored,
        has_rescale: !modality_lut.is_identity(),
        anatomy: anatomy.as_ref(),
        image_range: value_range(&decoded.samples, modality_lut),
        series_range: None,
    };
    let voi = VoiLut::from_file(file, frame, descriptor);
    let first = preset_list(&context, &voi, presets).into_iter().next();
    let (window, level, shape) = first.map_or((1.0, 0.0, LutShape::Linear), |p| {
        (p.window, p.level, p.shape)
    });
    VoiParams {
        window,
        level,
        shape,
        invert: descriptor.photometric_interpretation == "MONOCHROME1",
    }
}

/// Smallest and largest modality value of the frame, padding excluded.
fn value_range(samples: &Samples, modality: &ModalityLut) -> (f64, f64) {
    let values = (0..samples.len())
        .filter_map(|index| samples.get_f64(index))
        .filter(|&stored| !modality.is_padding(stored))
        .map(|stored| modality.apply(stored));
    let (min, max) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    if min <= max { (min, max) } else { (0.0, 0.0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn file(path: &str) -> DicomFile {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        read_dicom(&std::fs::read(full).expect("test data")).expect("readable")
    }

    #[test]
    fn a_grayscale_frame_uses_its_window() {
        let image = render_default(&file("pydicom/CT_small.dcm"), 0).expect("rendered");
        assert_eq!((image.width, image.height), (128, 128));
        assert_eq!(image.rgba.len(), 128 * 128 * 4);
        let distinct: std::collections::BTreeSet<u8> =
            image.rgba.iter().step_by(4).copied().collect();
        assert!(distinct.len() > 50, "a real range of grays");
    }

    #[test]
    fn color_frames_keep_their_colors() {
        let image = render_default(&file("pydicom/SC_rgb_rle.dcm"), 0).expect("rendered");
        let first = &image.rgba[..4];
        assert_eq!(first[3], 255);
        assert!(
            image.rgba.chunks(4).any(|p| p[0] != p[1] || p[1] != p[2]),
            "not gray"
        );
    }

    #[test]
    fn a_thumbnail_keeps_the_proportions() {
        let image = RgbaImage {
            width: 400,
            height: 200,
            rgba: (0..400 * 200)
                .flat_map(|i| {
                    if i % 2 == 0 {
                        [0, 0, 0, 255]
                    } else {
                        [200, 200, 200, 255]
                    }
                })
                .collect(),
        };
        let thumbnail = fit_within(&image, 100);
        assert_eq!((thumbnail.width, thumbnail.height), (100, 50));
        assert_eq!(&thumbnail.rgba[..4], &[100, 100, 100, 255], "averaged");
        assert_eq!(
            fit_within(&thumbnail, 256),
            thumbnail,
            "small images are kept"
        );
    }
}
