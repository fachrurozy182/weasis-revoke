//! Attributes of the Image Pixel Module (DICOM PS3.3 §C.7.6.3) needed to decode pixels.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/stream/ImageDescriptor.java`

use dicom_dictionary_std::tags;
use dicom_object::Tag;

use crate::dicom::DicomFile;
use crate::error::{Error, Result};

/// How the stored samples of a frame are laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageDescriptor {
    /// Rows (height).
    pub rows: u32,
    /// Columns (width).
    pub columns: u32,
    /// 1 for grayscale and palette color, 3 for color.
    pub samples_per_pixel: u16,
    /// Bits allocated per sample: 1, 8, 16, 32 or 64.
    pub bits_allocated: u16,
    /// Bits actually used per sample.
    pub bits_stored: u16,
    /// Whether samples are two's complement signed integers (Pixel Representation = 1).
    pub is_signed: bool,
    /// Photometric Interpretation, e.g. `MONOCHROME2`, `RGB`, `YBR_FULL_422`.
    pub photometric_interpretation: String,
    /// Whether color samples are stored plane by plane (Planar Configuration = 1).
    pub is_planar: bool,
}

impl ImageDescriptor {
    /// Reads the descriptor of a file that has pixel data.
    pub fn from_file(file: &DicomFile) -> Result<Self> {
        let required = |tag: Tag, name: &str| {
            integer(file, tag).ok_or_else(|| Error::PixelData(format!("missing or invalid {name}")))
        };
        let samples_per_pixel = optional_u16(file, tags::SAMPLES_PER_PIXEL).unwrap_or(1);
        let bits_allocated = required(tags::BITS_ALLOCATED, "Bits Allocated")?;
        Ok(Self {
            rows: required(tags::ROWS, "Rows")?,
            columns: required(tags::COLUMNS, "Columns")?,
            samples_per_pixel,
            bits_allocated: to_u16(bits_allocated)?,
            // Float Pixel Data has no Bits Stored: all allocated bits are used.
            bits_stored: optional_u16(file, tags::BITS_STORED).unwrap_or(to_u16(bits_allocated)?),
            is_signed: integer(file, tags::PIXEL_REPRESENTATION) == Some(1),
            photometric_interpretation: file
                .text(tags::PHOTOMETRIC_INTERPRETATION)
                .unwrap_or_else(|| "MONOCHROME2".to_owned()),
            is_planar: samples_per_pixel > 1
                && integer(file, tags::PLANAR_CONFIGURATION) == Some(1),
        })
    }

    /// Number of pixels in one frame.
    pub fn pixel_count(&self) -> usize {
        // u32 to usize never truncates on the 32 and 64-bit targets Weasis runs on.
        self.rows as usize * self.columns as usize
    }

    /// Number of samples in one frame (pixels × samples per pixel).
    pub fn sample_count(&self) -> usize {
        self.pixel_count() * usize::from(self.samples_per_pixel)
    }
}

fn integer(file: &DicomFile, tag: Tag) -> Option<u32> {
    file.object().element(tag).ok()?.to_int::<u32>().ok()
}

fn optional_u16(file: &DicomFile, tag: Tag) -> Option<u16> {
    integer(file, tag).and_then(|value| u16::try_from(value).ok())
}

fn to_u16(value: u32) -> Result<u16> {
    u16::try_from(value).map_err(|_| Error::PixelData(format!("invalid bit depth {value}")))
}
