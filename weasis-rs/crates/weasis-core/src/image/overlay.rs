//! Overlays: 1-bit graphics or text drawn over an image (Overlay Plane Module, PS3.3 §C.9.2).
//!
//! An overlay is either a plane in Overlay Data (60xx,3000), one bit per pixel, or (retired
//! since 2004, still found in old files) embedded in the unused high bits of the pixel data.
//! The 16 repeating groups 6000 to 601E are read; all of them are displayed.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/data/OverlayData.java`,
//! `EmbeddedOverlay.java` and `weasis-dicom/weasis-dicom-codec/.../display/OverlayOp.java`.

use dicom_core::Tag;
use dicom_pixeldata::PixelDecoder;

use super::Rgba;
use crate::dicom::DicomFile;
use crate::dicom::pixel::ImageDescriptor;
use crate::error::{Error, Result};

/// Number of overlay repeating groups (6000, 6002, ... 601E).
const OVERLAY_GROUPS: u16 = 16;

/// One overlay plane read from Overlay Data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayPlane {
    /// Group of the overlay (0x6000 to 0x601E).
    pub group: u16,
    /// Overlay Rows.
    pub rows: u32,
    /// Overlay Columns.
    pub columns: u32,
    /// First image frame (1-based) the overlay applies to.
    pub image_frame_origin: u32,
    /// Number of overlay frames.
    pub frames: u32,
    /// Position of the first overlay pixel in the image, 1-based (row, column); may be negative.
    pub origin: (i32, i32),
    /// Overlay bits, the first pixel in the least significant bit of the first byte.
    pub data: Vec<u8>,
}

impl OverlayPlane {
    fn read(file: &DicomFile, group: u16) -> Option<Self> {
        let element = |element: u16| Tag(group, element);
        let data = file
            .object()
            .element(element(0x3000))
            .ok()?
            .to_bytes()
            .ok()?
            .into_owned();
        let int = |tag: Tag, default: u32| {
            file.object()
                .element(tag)
                .ok()
                .and_then(|e| e.to_int::<u32>().ok())
                .unwrap_or(default)
        };
        let origin = file
            .object()
            .element(element(0x0050))
            .ok()
            .and_then(|e| e.to_multi_int::<i32>().ok())
            .and_then(|values| Some((*values.first()?, *values.get(1)?)))
            .unwrap_or((1, 1));
        Some(Self {
            group,
            rows: int(element(0x0010), 0),
            columns: int(element(0x0011), 0),
            image_frame_origin: int(element(0x0051), 1),
            frames: int(element(0x0015), 1),
            origin,
            data,
        })
    }

    /// Index of the overlay frame shown with an image frame (0-based), if any.
    fn frame_for(&self, image_frame: u32) -> Option<u32> {
        let index = i64::from(image_frame) - i64::from(self.image_frame_origin) + 1;
        u32::try_from(index).ok().filter(|&i| i < self.frames)
    }

    /// Sets the mask pixels covered by the overlay bits of one image frame.
    fn draw(&self, mask: &mut [bool], width: usize, image_frame: u32) {
        let Some(frame) = self.frame_for(image_frame) else {
            return;
        };
        let (rows, columns) = (self.rows as usize, self.columns as usize);
        let frame_offset = rows * columns * frame as usize;
        let height = mask.len() / width.max(1);
        for overlay_row in 0..rows {
            let Some(row) = shift(overlay_row, self.origin.0).filter(|&r| r < height) else {
                continue;
            };
            for overlay_column in 0..columns {
                let Some(column) = shift(overlay_column, self.origin.1).filter(|&c| c < width)
                else {
                    continue;
                };
                let bit = frame_offset + overlay_row * columns + overlay_column;
                if self
                    .data
                    .get(bit / 8)
                    .is_some_and(|byte| byte & (1 << (bit % 8)) != 0)
                {
                    mask[row * width + column] = true;
                }
            }
        }
    }
}

/// Image coordinate of an overlay coordinate, with a 1-based origin.
fn shift(index: usize, origin: i32) -> Option<usize> {
    let position = i64::try_from(index).ok()? + i64::from(origin) - 1;
    usize::try_from(position).ok()
}

/// An overlay stored in bit `bit_position` of the pixel samples (retired, PS3.5 §8.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbeddedOverlay {
    /// Group of the overlay.
    pub group: u16,
    /// Bit of the stored samples holding the overlay.
    pub bit_position: u16,
}

/// Overlay planes of an image (all 16 groups).
pub fn overlay_planes(file: &DicomFile) -> Vec<OverlayPlane> {
    groups()
        .filter_map(|group| OverlayPlane::read(file, group))
        .collect()
}

/// Overlay planes of a Presentation State: only those assigned to a graphic layer by Overlay
/// Activation Layer (60xx,1001) are displayed (PS3.3 §C.11.7).
pub fn presentation_overlay_planes(file: &DicomFile) -> Vec<OverlayPlane> {
    groups()
        .filter(|&group| file.text(Tag(group, 0x1001)).is_some())
        .filter_map(|group| OverlayPlane::read(file, group))
        .collect()
}

/// Embedded overlays of an image. Those whose bit is inside Bits Stored are ignored: the bit
/// belongs to the pixel value.
pub fn embedded_overlays(file: &DicomFile, descriptor: &ImageDescriptor) -> Vec<EmbeddedOverlay> {
    let int = |tag: Tag| {
        file.object()
            .element(tag)
            .ok()
            .and_then(|e| e.to_int::<u16>().ok())
    };
    groups()
        .filter(|&group| int(Tag(group, 0x0100)).is_some_and(|bits| bits != 1))
        .map(|group| EmbeddedOverlay {
            group,
            bit_position: int(Tag(group, 0x0102)).unwrap_or(0),
        })
        .filter(|overlay| overlay.bit_position >= descriptor.bits_stored)
        .collect()
}

/// Mask of the pixels covered by the overlays of one frame (0-based), row by row;
/// `None` when there are no overlays.
///
/// `extra` holds the overlay planes of an applied Presentation State.
pub fn overlay_mask(
    file: &DicomFile,
    frame: u32,
    descriptor: &ImageDescriptor,
    extra: &[OverlayPlane],
) -> Result<Option<Vec<bool>>> {
    let planes = overlay_planes(file);
    let embedded = embedded_overlays(file, descriptor);
    if planes.is_empty() && embedded.is_empty() && extra.is_empty() {
        return Ok(None);
    }
    let width = descriptor.columns as usize;
    let mut mask = vec![false; descriptor.pixel_count()];
    if !embedded.is_empty() {
        draw_embedded(file, frame, descriptor, &embedded, &mut mask)?;
    }
    for plane in extra.iter().chain(&planes) {
        plane.draw(&mut mask, width, frame);
    }
    Ok(Some(mask))
}

/// Embedded overlays need the high bits that decoding clears, so the frame is read again raw.
///
/// NOTE: the bits are read from the first sample of each pixel; embedded overlays only exist
/// in single-sample images of native or RLE transfer syntaxes.
fn draw_embedded(
    file: &DicomFile,
    frame: u32,
    descriptor: &ImageDescriptor,
    embedded: &[EmbeddedOverlay],
    mask: &mut [bool],
) -> Result<()> {
    let decoded = file
        .object()
        .decode_pixel_data_frame(frame)
        .map_err(|error| Error::PixelData(error.to_string()))?;
    let bytes = decoded.data();
    let sample_bytes = usize::from(descriptor.bits_allocated.div_ceil(8));
    let stride = sample_bytes * usize::from(descriptor.samples_per_pixel);
    let bits: u32 = embedded.iter().map(|o| 1u32 << o.bit_position).sum();
    for (pixel, set) in mask.iter_mut().enumerate() {
        let Some(sample) = bytes.get(pixel * stride..pixel * stride + sample_bytes) else {
            break;
        };
        let value = sample
            .iter()
            .rev()
            .fold(0u32, |value, &byte| (value << 8) | u32::from(byte));
        *set |= value & bits != 0;
    }
    Ok(())
}

/// Paints the masked pixels of an RGBA image (4 bytes per pixel) with a color; a translucent
/// color is blended with the image.
pub fn paint_overlay(rgba: &mut [u8], mask: &[bool], color: Rgba) {
    let [red, green, blue, alpha] = color.to_bytes();
    let alpha = f32::from(alpha) / 255.0;
    for (pixel, _) in rgba
        .chunks_exact_mut(4)
        .zip(mask)
        .filter(|(_, is_set)| **is_set)
    {
        for (channel, value) in pixel.iter_mut().zip([red, green, blue]) {
            *channel = blend(*channel, value, alpha);
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn blend(under: u8, over: u8, alpha: f32) -> u8 {
    // Both terms are in 0..=255 and the weights sum to 1, so the cast cannot truncate.
    (f32::from(over) * alpha + f32::from(under) * (1.0 - alpha)).round() as u8
}

fn groups() -> impl Iterator<Item = u16> {
    (0..OVERLAY_GROUPS).map(|index| 0x6000 + 2 * index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn file(path: &str) -> DicomFile {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        read_dicom(&bytes).expect("readable")
    }

    fn set_pixels(mask: &[bool]) -> usize {
        mask.iter().filter(|&&set| set).count()
    }

    #[test]
    fn overlay_planes_match_pydicom() {
        // Expected counts from pydicom `get_overlay_array`.
        let file = file("pydicom/examples_overlay.dcm");
        let descriptor = ImageDescriptor::from_file(&file).expect("descriptor");
        let mask = overlay_mask(&file, 0, &descriptor, &[]).expect("mask");
        let mask = mask.expect("has overlays");
        assert_eq!(set_pixels(&mask), 222);
        let first_row = mask.iter().position(|&set| set).map(|i| i / 484);
        assert_eq!(first_row, Some(36));
    }

    #[test]
    fn embedded_overlays_are_read_from_the_high_bits() {
        let file = file("weasis-dicom-tools/overlay.dcm");
        let descriptor = ImageDescriptor::from_file(&file).expect("descriptor");
        let embedded = embedded_overlays(&file, &descriptor);
        assert_eq!(
            embedded.iter().map(|o| o.bit_position).collect::<Vec<_>>(),
            [15, 14]
        );
        assert_eq!(overlay_planes(&file).len(), 2);
        // Union of bits 15 and 14 and of the planes 6004 and 6006, computed with pydicom.
        let mask = overlay_mask(&file, 0, &descriptor, &[]).expect("mask");
        assert_eq!(mask.as_deref().map(set_pixels), Some(2278));
    }

    #[test]
    fn presentation_states_only_show_activated_planes() {
        let pr = file("weasis-dicom-tools/prOverlay.dcm");
        let planes = presentation_overlay_planes(&pr);
        assert_eq!(
            planes.iter().map(|p| p.group).collect::<Vec<_>>(),
            [0x6008, 0x600A]
        );
        let mut mask = vec![false; 512 * 512];
        planes[0].draw(&mut mask, 512, 0);
        assert_eq!(set_pixels(&mask), 680);
    }

    #[test]
    fn an_image_without_overlays_has_no_mask() {
        let file = file("pydicom/CT_small.dcm");
        let descriptor = ImageDescriptor::from_file(&file).expect("descriptor");
        assert_eq!(overlay_mask(&file, 0, &descriptor, &[]).expect("ok"), None);
    }

    #[test]
    fn origin_and_frames_place_the_overlay() {
        let plane = OverlayPlane {
            group: 0x6000,
            rows: 1,
            columns: 2,
            image_frame_origin: 2,
            frames: 1,
            origin: (2, 0),
            data: vec![0b11],
        };
        let mut mask = vec![false; 9];
        plane.draw(&mut mask, 3, 0);
        assert_eq!(set_pixels(&mask), 0, "frame 1 has no overlay");
        plane.draw(&mut mask, 3, 1);
        // Row 2, columns 0 and 1 (1-based): column 0 is outside, so only (1, 0) 0-based.
        assert_eq!(
            mask,
            [false, false, false, true, false, false, false, false, false]
        );
    }

    #[test]
    fn translucent_colors_are_blended() {
        let mut rgba = vec![100, 100, 100, 255, 7, 7, 7, 255];
        let red = Rgba::opaque(1.0, 0.0, 0.0).with_alpha(0.5);
        paint_overlay(&mut rgba, &[true, false], red);
        assert_eq!(rgba, [178, 50, 50, 255, 7, 7, 7, 255]);
    }
}
