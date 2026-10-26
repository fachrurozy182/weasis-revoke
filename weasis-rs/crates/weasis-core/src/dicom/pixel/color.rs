//! Conversion of color frames to interleaved RGB (DICOM PS3.3 §C.7.6.3.1.2).
//!
//! - planar samples (`RRR... GGG... BBB...`) become interleaved (`RGB RGB ...`);
//! - YBR_FULL and YBR_FULL_422 become RGB, upsampling the 4:2:2 chroma of native data;
//! - PALETTE COLOR becomes 8-bit RGB through the Red/Green/Blue Palette Color Lookup Tables,
//!   normal or segmented (PS3.3 §C.7.9).
//!
//! Grayscale frames are returned unchanged: their display goes through the LUTs of step 1.4.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/util/PaletteColorUtils.java`,
//! `.../util/LookupTableUtils.java` (`lutData`, `InflateSegmentedLut`) and
//! `weasis-core-img/.../org/weasis/opencv/op/ImageConversion.java`.

use dicom_dictionary_std::tags;
use dicom_object::Tag;

use super::{PixelFrame, Samples};
use crate::dicom::DicomFile;
use crate::error::{Error, Result};

/// Converts a decoded color frame to interleaved RGB; grayscale frames are returned as is.
pub fn to_rgb(frame: PixelFrame, file: &DicomFile) -> Result<PixelFrame> {
    let photometric = frame.descriptor.photometric_interpretation.clone();
    match photometric.as_str() {
        "MONOCHROME1" | "MONOCHROME2" => Ok(frame),
        "RGB" => Ok(interleave(frame)),
        "YBR_FULL" | "YBR_FULL_422" => ybr_to_rgb(interleave(frame)),
        "PALETTE COLOR" => apply_palette(frame, file),
        other => Err(Error::PixelData(format!(
            "unsupported Photometric Interpretation {other}"
        ))),
    }
}

/// Planar Configuration 1 to 0.
fn interleave(mut frame: PixelFrame) -> PixelFrame {
    if !frame.descriptor.is_planar {
        return frame;
    }
    let planes = usize::from(frame.descriptor.samples_per_pixel);
    frame.samples = match frame.samples {
        Samples::U8(values) => Samples::U8(interleave_planes(&values, planes)),
        Samples::U16(values) => Samples::U16(interleave_planes(&values, planes)),
        other => other,
    };
    frame.descriptor.is_planar = false;
    frame
}

fn interleave_planes<T: Copy>(values: &[T], planes: usize) -> Vec<T> {
    let plane_length = values.len() / planes.max(1);
    (0..plane_length)
        .flat_map(|pixel| {
            (0..planes).filter_map(move |plane| values.get(plane * plane_length + pixel))
        })
        .copied()
        .collect()
}

fn ybr_to_rgb(mut frame: PixelFrame) -> Result<PixelFrame> {
    let Samples::U8(values) = &frame.samples else {
        return Err(Error::PixelData(
            "YBR color is supported for 8-bit samples only".to_owned(),
        ));
    };
    let pixel_count = frame.descriptor.pixel_count();
    // Native YBR_FULL_422 stores Y1 Y2 Cb Cr for each pair of pixels (two samples per pixel);
    // decoded JPEG streams already have three samples per pixel.
    let full = if values.len() == pixel_count * 2 {
        upsample_422(values)
    } else {
        values.clone()
    };
    let rgb = full
        .chunks_exact(3)
        .flat_map(|ybr| ybr_pixel_to_rgb(ybr[0], ybr[1], ybr[2]))
        .collect();
    frame.samples = Samples::U8(rgb);
    frame.descriptor.photometric_interpretation = "RGB".to_owned();
    Ok(frame)
}

/// `Y1 Y2 Cb Cr` per pixel pair to `Y Cb Cr` per pixel (chroma repeated).
fn upsample_422(values: &[u8]) -> Vec<u8> {
    values
        .chunks_exact(4)
        .flat_map(|pair| [pair[0], pair[2], pair[3], pair[1], pair[2], pair[3]])
        .collect()
}

/// YBR_FULL to RGB with the ITU-R BT.601 full-range equations of PS3.3 §C.7.6.3.1.2.
fn ybr_pixel_to_rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let y = f32::from(y);
    let cb = f32::from(cb) - 128.0;
    let cr = f32::from(cr) - 128.0;
    [
        to_byte(y + 1.402 * cr),
        to_byte(y - 0.344_136 * cb - 0.714_136 * cr),
        to_byte(y + 1.772 * cb),
    ]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    // Clamped to 0..=255 first, so the cast cannot truncate.
    value.round().clamp(0.0, 255.0) as u8
}

fn apply_palette(mut frame: PixelFrame, file: &DicomFile) -> Result<PixelFrame> {
    let red = PaletteTable::read(file, PaletteTags::RED)?;
    let green = PaletteTable::read(file, PaletteTags::GREEN)?;
    let blue = PaletteTable::read(file, PaletteTags::BLUE)?;
    let indices: Vec<i64> = (0..frame.samples.len())
        .filter_map(|index| frame.samples.get_f64(index))
        .map(|value| value as i64)
        .collect();
    let rgb = indices
        .iter()
        .flat_map(|&index| [red.lookup(index), green.lookup(index), blue.lookup(index)])
        .collect();
    frame.samples = Samples::U8(rgb);
    frame.descriptor.samples_per_pixel = 3;
    frame.descriptor.bits_allocated = 8;
    frame.descriptor.bits_stored = 8;
    frame.descriptor.is_signed = false;
    frame.descriptor.photometric_interpretation = "RGB".to_owned();
    Ok(frame)
}

/// Tags of one color channel of a palette.
struct PaletteTags {
    descriptor: Tag,
    data: Tag,
    segmented_data: Tag,
}

impl PaletteTags {
    const RED: Self = Self {
        descriptor: tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
        data: tags::RED_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        segmented_data: tags::SEGMENTED_RED_PALETTE_COLOR_LOOKUP_TABLE_DATA,
    };
    const GREEN: Self = Self {
        descriptor: tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
        data: tags::GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        segmented_data: tags::SEGMENTED_GREEN_PALETTE_COLOR_LOOKUP_TABLE_DATA,
    };
    const BLUE: Self = Self {
        descriptor: tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DESCRIPTOR,
        data: tags::BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA,
        segmented_data: tags::SEGMENTED_BLUE_PALETTE_COLOR_LOOKUP_TABLE_DATA,
    };
}

/// One channel of a palette, reduced to 8-bit entries like in the Java viewer.
struct PaletteTable {
    first_mapped_value: i64,
    entries: Vec<u8>,
}

impl PaletteTable {
    fn read(file: &DicomFile, tags: PaletteTags) -> Result<Self> {
        let object = file.object();
        let descriptor: Vec<i64> = object
            .element(tags.descriptor)
            .ok()
            .and_then(|element| element.to_multi_int::<i64>().ok())
            .filter(|values| values.len() == 3)
            .ok_or_else(|| Error::PixelData("missing palette descriptor".to_owned()))?;
        // PS3.3 §C.7.6.3.1.5: 0 entries means 65536, the first mapped value may be signed (SS).
        let length = if descriptor[0] <= 0 {
            descriptor[0] + 0x1_0000
        } else {
            descriptor[0]
        };
        let length = usize::try_from(length).map_err(|e| Error::PixelData(e.to_string()))?;
        let bits = descriptor[2];

        let words = |tag: Tag| {
            object
                .element(tag)
                .ok()
                .and_then(|element| element.to_multi_int::<u16>().ok())
        };
        let entries = if let Some(words) = words(tags.data) {
            entries_from_words(&words, length, bits)?
        } else if let Some(segments) = words(tags.segmented_data) {
            inflate_segmented(&segments, length)?
        } else {
            return Err(Error::PixelData("missing palette data".to_owned()));
        };
        Ok(Self {
            first_mapped_value: descriptor[1],
            entries,
        })
    }

    /// Entry for a stored value; values outside the table use the first or last entry.
    fn lookup(&self, value: i64) -> u8 {
        let last = self.entries.len().saturating_sub(1);
        let index = usize::try_from(value - self.first_mapped_value)
            .unwrap_or(0)
            .min(last);
        self.entries.get(index).copied().unwrap_or(0)
    }
}

/// 16-bit entries keep their high byte; 8-bit entries are either one per 16-bit word (low
/// byte) or packed two per word (LookupTableUtils.processLutData).
fn entries_from_words(words: &[u16], length: usize, bits: i64) -> Result<Vec<u8>> {
    let entries: Vec<u8> = if bits == 16 {
        words.iter().map(|word| word.to_be_bytes()[0]).collect()
    } else if words.len() == length {
        words.iter().map(|word| word.to_le_bytes()[0]).collect()
    } else {
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    };
    if entries.len() < length {
        return Err(Error::PixelData(format!(
            "palette has {} entries instead of {length}",
            entries.len()
        )));
    }
    Ok(entries)
}

/// Expands Segmented Palette Color Lookup Table Data (PS3.3 §C.7.9.2) to 8-bit entries.
fn inflate_segmented(segments: &[u16], length: usize) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(length);
    inflate_segments(segments, 0, None, 0, &mut output)?;
    if output.len() != length {
        return Err(Error::PixelData(format!(
            "segmented palette has {} entries instead of {length}",
            output.len()
        )));
    }
    Ok(output
        .into_iter()
        .map(|value| value.to_be_bytes()[0])
        .collect())
}

/// Inflates segments from `position`; `count` limits the number of segments (indirect
/// segments). Returns the last value written, needed by the next linear segment.
fn inflate_segments(
    segments: &[u16],
    mut position: usize,
    count: Option<usize>,
    mut last_value: u16,
    output: &mut Vec<u16>,
) -> Result<u16> {
    let malformed = || Error::PixelData("malformed segmented palette".to_owned());
    let read = |position: &mut usize| {
        let value = segments.get(*position).copied().ok_or_else(malformed);
        *position += 1;
        value
    };
    let mut done = 0;
    while count.map_or(position < segments.len(), |count| done < count) {
        let opcode = read(&mut position)?;
        let length = usize::from(read(&mut position)?);
        match opcode {
            // Discrete segment: `length` values.
            0 => {
                for _ in 0..length {
                    last_value = read(&mut position)?;
                    output.push(last_value);
                }
            }
            // Linear segment: `length` values from the previous value to the end value.
            1 => {
                let end = i64::from(read(&mut position)?);
                let start = i64::from(last_value);
                let steps = i64::try_from(length).map_err(|_| malformed())?.max(1);
                for step in 1..=steps {
                    let value = start + (end - start) * step / steps;
                    output.push(u16::try_from(value).map_err(|_| malformed())?);
                }
                last_value = u16::try_from(end).map_err(|_| malformed())?;
            }
            // Indirect segment: replays `length` segments at a 32-bit offset (no nesting).
            2 if count.is_none() => {
                let low = u32::from(read(&mut position)?);
                let high = u32::from(read(&mut position)?);
                let offset = usize::try_from(low | (high << 16)).map_err(|_| malformed())?;
                last_value = inflate_segments(segments, offset, Some(length), last_value, output)?;
            }
            _ => return Err(malformed()),
        }
        done += 1;
    }
    Ok(last_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_ybr_is_gray_and_extremes_saturate() {
        assert_eq!(ybr_pixel_to_rgb(128, 128, 128), [128, 128, 128]);
        assert_eq!(ybr_pixel_to_rgb(255, 128, 255), [255, 164, 255]);
    }

    #[test]
    fn ybr_422_pairs_share_their_chroma() {
        assert_eq!(
            upsample_422(&[10, 20, 30, 40]),
            vec![10, 30, 40, 20, 30, 40]
        );
    }

    #[test]
    fn planes_are_interleaved() {
        assert_eq!(
            interleave_planes(&[1, 2, 3, 4, 5, 6], 3),
            vec![1, 3, 5, 2, 4, 6]
        );
    }

    #[test]
    fn segmented_palette_with_discrete_linear_and_indirect_segments() {
        // Discrete [0x0000, 0x1000], linear to 0x4000 in 3 steps, then replay the first
        // (discrete) segment through an indirect segment at offset 0.
        let segments = [0, 2, 0x0000, 0x1000, 1, 3, 0x4000, 2, 1, 0, 0];
        let entries = inflate_segmented(&segments, 7).expect("valid");
        assert_eq!(entries, vec![0x00, 0x10, 0x20, 0x30, 0x40, 0x00, 0x10]);
    }

    #[test]
    fn sixteen_bit_palette_entries_keep_their_high_byte() {
        let entries = entries_from_words(&[0x0000, 0xFF00, 0x80FF], 3, 16).expect("valid");
        assert_eq!(entries, vec![0x00, 0xFF, 0x80]);
    }
}
