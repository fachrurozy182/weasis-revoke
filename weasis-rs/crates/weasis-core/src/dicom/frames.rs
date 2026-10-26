//! Compressed bytes of one frame in encapsulated pixel data (DICOM PS3.5 §A.4).
//!
//! A frame can span several fragments. Like the Java viewer, the Basic Offset Table is not
//! used (it is often empty or wrong); fragments are mapped to frames as follows:
//! - at least as many frames as fragments: one fragment per frame;
//! - a single frame: all fragments together;
//! - otherwise: a frame starts at each fragment that begins with a JPEG or JPEG 2000 marker.
//!
//! Video transfer syntaxes hold one stream for all frames and are not read through here.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/DicomImageReader.java`
//! (`buildFragmentedStream`, `identifyFrameFragments`).

use std::borrow::Cow;

use dicom_dictionary_std::tags;

use super::DicomFile;
use crate::error::{Error, Result};

/// Compressed bytes of `frame` (0-based).
pub fn compressed_frame(file: &DicomFile, frame: u32) -> Result<Cow<'_, [u8]>> {
    let frame_count = file.number_of_frames();
    if frame >= frame_count {
        return Err(pixel_error(format!(
            "frame {frame} does not exist ({frame_count} frames)"
        )));
    }
    let fragments = file
        .object()
        .element(tags::PIXEL_DATA)
        .ok()
        .and_then(|element| element.value().fragments())
        .ok_or_else(|| pixel_error("pixel data is not encapsulated".to_owned()))?;
    let Some(last) = fragments.len().checked_sub(1) else {
        return Err(pixel_error(
            "encapsulated pixel data has no fragment".to_owned(),
        ));
    };

    let frame = usize::try_from(frame).map_err(|e| pixel_error(e.to_string()))?;
    let frame_count = usize::try_from(frame_count).map_err(|e| pixel_error(e.to_string()))?;

    if frame_count >= fragments.len() {
        return Ok(Cow::Borrowed(&fragments[frame.min(last)]));
    }
    if frame_count == 1 {
        return Ok(Cow::Owned(fragments.concat()));
    }

    let starts: Vec<usize> = fragments
        .iter()
        .enumerate()
        .filter(|(_, fragment)| starts_codestream(fragment))
        .map(|(index, _)| index)
        .collect();
    if starts.len() != frame_count {
        return Err(pixel_error(format!(
            "cannot match {} fragments to {frame_count} frames",
            fragments.len()
        )));
    }
    let end = starts.get(frame + 1).copied().unwrap_or(fragments.len());
    Ok(Cow::Owned(fragments[starts[frame]..end].concat()))
}

/// Whether a fragment begins a JPEG / JPEG-LS (SOI), JPEG 2000 codestream (SOC) or JP2 file.
fn starts_codestream(fragment: &[u8]) -> bool {
    const JP2_SIGNATURE: &[u8] = b"\0\0\0\x0CjP  ";
    fragment.starts_with(&[0xFF, 0xD8])
        || fragment.starts_with(&[0xFF, 0x4F])
        || fragment.starts_with(JP2_SIGNATURE)
}

fn pixel_error(message: String) -> Error {
    Error::PixelData(message)
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

    #[test]
    fn every_frame_of_a_multiframe_jpeg_2000_is_a_complete_codestream() {
        let file = file("weasis-dicom-tools/jpeg2000-multiframe-multifragments.dcm");
        for frame in 0..file.number_of_frames() {
            let bytes = compressed_frame(&file, frame).expect("frame");
            let image = oxideav_jpeg2000::decode_j2k(&bytes).expect("decodable");
            assert_eq!((image.width, image.height), (256, 256), "frame {frame}");
        }
    }

    #[test]
    fn every_frame_of_a_multiframe_jpeg_is_a_complete_image() {
        let file = file("pydicom/examples_ybr_color.dcm");
        assert_eq!(file.number_of_frames(), 30);
        for frame in 0..file.number_of_frames() {
            let bytes = compressed_frame(&file, frame).expect("frame");
            let mut decoder = oxiarc_jpeg::Decoder::new(&bytes[..]);
            assert!(decoder.decode().is_ok(), "frame {frame}");
        }
    }

    #[test]
    fn frame_out_of_range_is_an_error() {
        let file = file("pydicom/JPEG2000.dcm");
        assert!(compressed_frame(&file, 0).is_ok());
        assert!(matches!(
            compressed_frame(&file, 1),
            Err(Error::PixelData(_))
        ));
    }

    #[test]
    fn native_pixel_data_is_not_encapsulated() {
        let file = file("pydicom/MR_small.dcm");
        assert!(compressed_frame(&file, 0).is_err());
    }
}
