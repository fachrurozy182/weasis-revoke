//! Content of objects that are not images: encapsulated documents (PDF, CDA, 3D models) and
//! video streams.
//!
//! A video is played by the browser when it is H.264 in an MP4 container; MPEG-2, HEVC and bare
//! H.264 streams are transcoded by the server (decision D14).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomEncapDocElement.java`
//! and `DicomVideoElement.java`.

use dicom_dictionary_std::tags;

use super::DicomFile;
use crate::error::{Error, Result};

/// An encapsulated document (Encapsulated Document Module, PS3.3 §C.24.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncapsulatedDocument {
    /// MIME Type of Encapsulated Document, e.g. `application/pdf`.
    pub mime_type: String,
    /// The document.
    pub bytes: Vec<u8>,
}

impl EncapsulatedDocument {
    /// Reads the document of a file.
    pub fn from_file(file: &DicomFile) -> Result<Self> {
        let mut bytes = file
            .object()
            .element(tags::ENCAPSULATED_DOCUMENT)
            .ok()
            .and_then(|element| element.to_bytes().ok())
            .ok_or_else(|| Error::UnreadableDicom("no Encapsulated Document".to_owned()))?
            .into_owned();
        // The value is padded to an even length; Encapsulated Document Length, when present,
        // gives the real size (PS3.3 §C.24.2.1).
        let length = file
            .object()
            .element(tags::ENCAPSULATED_DOCUMENT_LENGTH)
            .ok()
            .and_then(|element| element.to_int::<usize>().ok());
        if let Some(length) = length.filter(|&l| l <= bytes.len()) {
            bytes.truncate(length);
        }
        Ok(Self {
            mime_type: file
                .text(tags::MIME_TYPE_OF_ENCAPSULATED_DOCUMENT)
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
            bytes,
        })
    }

    /// File extension for the MIME type, to name a downloaded copy.
    pub fn extension(&self) -> &'static str {
        match self.mime_type.to_lowercase().as_str() {
            "application/pdf" => "pdf",
            "text/xml" | "application/xml" => "xml",
            "model/stl" | "application/sla" => "stl",
            "model/obj" => "obj",
            "model/mtl" => "mtl",
            "text/plain" => "txt",
            _ => "bin",
        }
    }
}

/// Who plays a video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPlayback {
    /// The browser plays the stream as it is (`video/mp4`).
    Browser,
    /// The server transcodes the stream to H.264 in MP4 first.
    ServerTranscode,
}

/// The video stream of an object encoded with a video transfer syntax (PS3.5 §8.2.5 to §8.2.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStream {
    /// Transfer Syntax UID of the stream.
    pub transfer_syntax: String,
    /// The stream.
    pub bytes: Vec<u8>,
}

impl VideoStream {
    /// Reads the stream of a file: the fragments of the Pixel Data, joined.
    ///
    /// NOTE: the Java viewer keeps the last fragment only, which holds the whole stream in the
    /// common case; a stream split over several fragments (allowed since 2019) is joined here.
    pub fn from_file(file: &DicomFile) -> Result<Self> {
        let element = file
            .object()
            .element(tags::PIXEL_DATA)
            .map_err(|e| Error::PixelData(e.to_string()))?;
        let fragments = element
            .value()
            .fragments()
            .ok_or_else(|| Error::PixelData("a video must be encapsulated".to_owned()))?;
        Ok(Self {
            transfer_syntax: file.transfer_syntax().to_owned(),
            bytes: fragments.concat(),
        })
    }

    /// Whether the transfer syntax is H.264 (MPEG-4 AVC, `.102` to `.106` and their
    /// fragmentable variants).
    pub fn is_h264(&self) -> bool {
        let Some(code) = self.transfer_syntax.strip_prefix("1.2.840.10008.1.2.4.") else {
            return false;
        };
        let main = code.split('.').next().unwrap_or_default();
        matches!(main, "102" | "103" | "104" | "105" | "106")
    }

    /// Whether the stream is an MP4 file (ISO base media format: an `ftyp` box first).
    pub fn is_mp4(&self) -> bool {
        self.bytes.get(4..8) == Some(b"ftyp")
    }

    /// Who plays the stream.
    pub fn playback(&self) -> VideoPlayback {
        if self.is_h264() && self.is_mp4() {
            VideoPlayback::Browser
        } else {
            VideoPlayback::ServerTranscode
        }
    }

    /// MIME type of the stream as stored.
    pub fn mime_type(&self) -> &'static str {
        if self.is_mp4() {
            "video/mp4"
        } else if self.is_h264() {
            "video/h264"
        } else {
            "video/mpeg"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;

    fn file(path: &str) -> DicomFile {
        let full = format!(
            "{}/../../testdata/synthetic/{path}",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        read_dicom(&bytes).expect("readable")
    }

    #[test]
    fn the_pdf_is_extracted() {
        let document = EncapsulatedDocument::from_file(&file("encapsulated-pdf.dcm")).expect("PDF");
        assert_eq!(document.mime_type, "application/pdf");
        assert_eq!(document.extension(), "pdf");
        assert!(document.bytes.starts_with(b"%PDF-1.4"));
        // Without Encapsulated Document Length the padding byte stays, as in Java.
        assert!(document.bytes.ends_with(b"%%EOF\n\0"));
    }

    #[test]
    fn h264_in_mp4_plays_in_the_browser() {
        let video = VideoStream::from_file(&file("video-mpeg4-h264.dcm")).expect("video");
        assert!(video.is_h264() && video.is_mp4());
        assert_eq!(video.playback(), VideoPlayback::Browser);
        assert_eq!(video.mime_type(), "video/mp4");
    }

    #[test]
    fn mpeg2_is_transcoded_by_the_server() {
        let video = VideoStream::from_file(&file("video-mpeg2.dcm")).expect("video");
        assert!(!video.is_h264());
        assert_eq!(video.playback(), VideoPlayback::ServerTranscode);
        // An MPEG program stream starts with a pack header.
        assert_eq!(video.bytes.get(0..4), Some(&[0x00, 0x00, 0x01, 0xBA][..]));
    }

    #[test]
    fn images_have_no_video_stream() {
        assert!(VideoStream::from_file(&file("cr-monochrome1.dcm")).is_err());
    }
}
