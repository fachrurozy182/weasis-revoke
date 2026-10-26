//! Decoded pixel samples and the conversions from codec output to stored values.
//!
//! Stored values follow DICOM PS3.5 §8.1.1: only the low `Bits Stored` bits of each sample are
//! significant. Higher bits are cleared for unsigned samples and filled with the sign bit for
//! signed samples (they may contain old embedded overlays).
//!
//! Java reference: `weasis-core-img/.../org/weasis/opencv/data/PlanarImage.java` (sample types)
//! and `weasis-dicom-tools/.../org/dcm4che3/img/DicomImageReader.java`.

use super::ImageDescriptor;
use crate::error::{Error, Result};

/// Stored sample values of one frame, interleaved per pixel (`R G B R G B ...` for color).
#[derive(Debug, Clone, PartialEq)]
pub enum Samples {
    /// Unsigned 8-bit (and 1-bit, unpacked to 0/1).
    U8(Vec<u8>),
    /// Unsigned 16-bit.
    U16(Vec<u16>),
    /// Signed 16-bit (also signed 8-bit).
    I16(Vec<i16>),
    /// Unsigned 32-bit, e.g. RT dose grids.
    U32(Vec<u32>),
    /// Signed 32-bit.
    I32(Vec<i32>),
    /// Float and Double Float Pixel Data.
    F32(Vec<f32>),
}

impl Samples {
    /// Number of samples.
    pub fn len(&self) -> usize {
        match self {
            Self::U8(values) => values.len(),
            Self::U16(values) => values.len(),
            Self::I16(values) => values.len(),
            Self::U32(values) => values.len(),
            Self::I32(values) => values.len(),
            Self::F32(values) => values.len(),
        }
    }

    /// Whether there is no sample.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sample at `index` as `f64`, for statistics and tests.
    pub fn get_f64(&self, index: usize) -> Option<f64> {
        match self {
            Self::U8(values) => values.get(index).map(|&v| f64::from(v)),
            Self::U16(values) => values.get(index).map(|&v| f64::from(v)),
            Self::I16(values) => values.get(index).map(|&v| f64::from(v)),
            Self::U32(values) => values.get(index).map(|&v| f64::from(v)),
            Self::I32(values) => values.get(index).map(|&v| f64::from(v)),
            Self::F32(values) => values.get(index).map(|&v| f64::from(v)),
        }
    }
}

/// Builds stored samples from little-endian bytes of native pixel data.
pub fn from_le_bytes(bytes: &[u8], descriptor: &ImageDescriptor) -> Result<Samples> {
    let bits = descriptor.bits_stored;
    match (descriptor.bits_allocated, descriptor.is_signed) {
        (1, _) => Ok(Samples::U8(unpack_bits(
            bytes,
            0,
            descriptor.sample_count(),
        ))),
        (8, false) => Ok(Samples::U8(
            bytes.iter().map(|&v| mask_u8(v, bits)).collect(),
        )),
        (8, true) => Ok(Samples::I16(
            bytes
                .iter()
                .map(|&v| i16::from(sign_extend_u8(v, bits)))
                .collect(),
        )),
        (16, false) => Ok(Samples::U16(
            words(bytes).map(|v| mask_u16(v, bits)).collect(),
        )),
        (16, true) => Ok(Samples::I16(
            words(bytes).map(|v| sign_extend_u16(v, bits)).collect(),
        )),
        (32, false) => Ok(Samples::U32(
            dwords(bytes).map(|v| mask_u32(v, bits)).collect(),
        )),
        (32, true) => Ok(Samples::I32(
            dwords(bytes).map(|v| sign_extend_u32(v, bits)).collect(),
        )),
        (allocated, _) => Err(Error::PixelData(format!(
            "unsupported Bits Allocated {allocated}"
        ))),
    }
}

/// Builds stored samples from the unsigned output of a JPEG or JPEG-LS decoder.
pub fn from_unsigned_codec(values: Vec<u16>, descriptor: &ImageDescriptor) -> Samples {
    let bits = descriptor.bits_stored;
    if descriptor.is_signed {
        Samples::I16(
            values
                .into_iter()
                .map(|v| sign_extend_u16(v, bits))
                .collect(),
        )
    } else if descriptor.bits_allocated <= 8 {
        // Values of an 8-bit codec always fit in a byte.
        Samples::U8(values.into_iter().map(|v| v.to_le_bytes()[0]).collect())
    } else {
        Samples::U16(values.into_iter().map(|v| mask_u16(v, bits)).collect())
    }
}

/// Builds stored samples from values already interpreted by a codec (JPEG 2000), using the
/// smallest type that holds them: the codec, not the header, decides the sign (PS3.5 §8.2.4).
pub fn from_i32_values(values: Vec<i32>, descriptor: &ImageDescriptor) -> Samples {
    let fits = |min: i32, max: i32| values.iter().all(|v| (min..=max).contains(v));
    if descriptor.bits_allocated <= 8 && fits(0, i32::from(u8::MAX)) {
        Samples::U8(
            values
                .into_iter()
                .filter_map(|v| u8::try_from(v).ok())
                .collect(),
        )
    } else if fits(0, i32::from(u16::MAX)) && !descriptor.is_signed {
        Samples::U16(
            values
                .into_iter()
                .filter_map(|v| u16::try_from(v).ok())
                .collect(),
        )
    } else if fits(i32::from(i16::MIN), i32::from(i16::MAX)) {
        Samples::I16(
            values
                .into_iter()
                .filter_map(|v| i16::try_from(v).ok())
                .collect(),
        )
    } else {
        Samples::I32(values)
    }
}

/// Unpacks `count` 1-bit samples starting at bit `first_bit`, least significant bit first
/// (PS3.5 §8.1.1, Figure D-1).
pub fn unpack_bits(bytes: &[u8], first_bit: usize, count: usize) -> Vec<u8> {
    (first_bit..first_bit + count)
        .map(|bit| bytes.get(bit / 8).map_or(0, |byte| (byte >> (bit % 8)) & 1))
        .collect()
}

fn words(bytes: &[u8]) -> impl Iterator<Item = u16> + '_ {
    bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn dwords(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn mask_u8(value: u8, bits: u16) -> u8 {
    if bits >= 8 {
        value
    } else {
        value & ((1u8 << bits) - 1)
    }
}

fn mask_u16(value: u16, bits: u16) -> u16 {
    if bits >= 16 {
        value
    } else {
        value & ((1u16 << bits) - 1)
    }
}

fn mask_u32(value: u32, bits: u16) -> u32 {
    if bits >= 32 {
        value
    } else {
        value & ((1u32 << bits) - 1)
    }
}

fn sign_extend_u8(value: u8, bits: u16) -> i8 {
    let shift = 8u16.saturating_sub(bits);
    i8::from_le_bytes([value << shift]) >> shift
}

fn sign_extend_u16(value: u16, bits: u16) -> i16 {
    let shift = 16u16.saturating_sub(bits);
    i16::from_le_bytes((value << shift).to_le_bytes()) >> shift
}

fn sign_extend_u32(value: u32, bits: u16) -> i32 {
    let shift = 32u16.saturating_sub(bits);
    i32::from_le_bytes((value << shift).to_le_bytes()) >> shift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(bits_allocated: u16, bits_stored: u16, is_signed: bool) -> ImageDescriptor {
        ImageDescriptor {
            rows: 1,
            columns: 2,
            samples_per_pixel: 1,
            bits_allocated,
            bits_stored,
            is_signed,
            photometric_interpretation: "MONOCHROME2".to_owned(),
            is_planar: false,
        }
    }

    #[test]
    fn unused_high_bits_are_cleared_for_unsigned_samples() {
        // 12-bit value 0x0FFF with an overlay bit set in bit 15.
        let samples = from_le_bytes(&[0xFF, 0x8F, 0x01, 0x00], &descriptor(16, 12, false));
        assert_eq!(samples.expect("valid"), Samples::U16(vec![0x0FFF, 1]));
    }

    #[test]
    fn signed_samples_are_sign_extended_from_bits_stored() {
        // 9-bit two's complement: 0x1FF is -1, 0x0FF is 255.
        let samples = from_le_bytes(&[0xFF, 0x01, 0xFF, 0x00], &descriptor(16, 9, true));
        assert_eq!(samples.expect("valid"), Samples::I16(vec![-1, 255]));
    }

    #[test]
    fn one_bit_samples_are_unpacked_least_significant_bit_first() {
        let mut descriptor = descriptor(1, 1, false);
        descriptor.columns = 10;
        let samples = from_le_bytes(&[0b0000_0101, 0b0000_0010], &descriptor);
        assert_eq!(
            samples.expect("valid"),
            Samples::U8(vec![1, 0, 1, 0, 0, 0, 0, 0, 0, 1])
        );
    }

    #[test]
    fn codec_values_use_the_smallest_type() {
        let unsigned = from_i32_values(vec![0, 4095], &descriptor(16, 12, false));
        assert_eq!(unsigned, Samples::U16(vec![0, 4095]));
        let signed = from_i32_values(vec![-1024, 3071], &descriptor(16, 12, true));
        assert_eq!(signed, Samples::I16(vec![-1024, 3071]));
    }
}
