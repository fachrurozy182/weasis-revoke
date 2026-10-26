//! Decoding one frame into stored sample values, with the codec chosen by transfer syntax.
//!
//! Codecs (decision D14 in `docs/PLAN.md`): `dicom-pixeldata` for native, deflated and RLE data,
//! `oxiarc-jpeg` for JPEG, `pure_jpegls` for single-component JPEG-LS and `oxideav-jpeg2000` for
//! JPEG 2000 and HTJ2K. Multi-component JPEG-LS has no browser codec and is decoded by the server.
//!
//! The output keeps the stored values; photometric conversions (YBR, palette, planar) are done
//! in step 1.3.c, except where a codec does them itself (lossy JPEG, JPEG 2000 with MCT).
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/DicomImageReader.java` (`readRaster`,
//! `shouldConvertYbr2Rgb`) and `weasis-dicom/weasis-dicom-codec/.../TransferSyntax.java`.

use dicom_dictionary_std::tags;
use dicom_pixeldata::PixelDecoder;

use super::ImageDescriptor;
use super::samples::{self, Samples};
use crate::dicom::{DicomFile, compressed_frame};
use crate::error::{Error, Result};

/// One decoded frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelFrame {
    /// Layout of `samples`. The photometric interpretation and planar configuration describe
    /// the decoded samples, which may differ from the file (e.g. RGB after a lossy JPEG).
    pub descriptor: ImageDescriptor,
    /// Stored sample values.
    pub samples: Samples,
}

/// Codec families, by transfer syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Codec {
    Native,
    JpegLossy,
    JpegLossless,
    JpegLs,
    Jpeg2000,
    Video,
}

fn codec_of(transfer_syntax: &str) -> Option<Codec> {
    let codec = match transfer_syntax {
        "1.2.840.10008.1.2"
        | "1.2.840.10008.1.2.1"
        | "1.2.840.10008.1.2.2"
        | "1.2.840.10008.1.2.1.99"
        | "1.2.840.10008.1.2.5"
        | "1.2.840.10008.1.2.8.1" => Codec::Native,
        "1.2.840.10008.1.2.4.50" | "1.2.840.10008.1.2.4.51" => Codec::JpegLossy,
        "1.2.840.10008.1.2.4.57" | "1.2.840.10008.1.2.4.70" => Codec::JpegLossless,
        "1.2.840.10008.1.2.4.80" | "1.2.840.10008.1.2.4.81" => Codec::JpegLs,
        "1.2.840.10008.1.2.4.90"
        | "1.2.840.10008.1.2.4.91"
        | "1.2.840.10008.1.2.4.92"
        | "1.2.840.10008.1.2.4.93"
        | "1.2.840.10008.1.2.4.201"
        | "1.2.840.10008.1.2.4.202"
        | "1.2.840.10008.1.2.4.203" => Codec::Jpeg2000,
        ts if ts.starts_with("1.2.840.10008.1.2.4.10") => Codec::Video,
        _ => return None,
    };
    Some(codec)
}

/// Decodes `frame` (0-based) of a file with pixel data.
pub fn decode_frame(file: &DicomFile, frame: u32) -> Result<PixelFrame> {
    let descriptor = ImageDescriptor::from_file(file)?;
    if frame >= file.number_of_frames() {
        return Err(Error::PixelData(format!(
            "frame {frame} does not exist ({} frames)",
            file.number_of_frames()
        )));
    }
    if let Some(samples) = float_samples(file, &descriptor, frame)? {
        return Ok(PixelFrame {
            descriptor,
            samples,
        });
    }

    let transfer_syntax = file.transfer_syntax();
    match codec_of(transfer_syntax) {
        Some(Codec::Native) => decode_native(file, descriptor, frame),
        Some(Codec::JpegLossy) => decode_jpeg(file, descriptor, frame, true),
        Some(Codec::JpegLossless) => decode_jpeg(file, descriptor, frame, false),
        Some(Codec::JpegLs) => decode_jpeg_ls(file, descriptor, frame),
        Some(Codec::Jpeg2000) => decode_jpeg_2000(file, descriptor, frame),
        Some(Codec::Video) => Err(Error::PixelData(
            "a video is played as a stream, not decoded frame by frame".to_owned(),
        )),
        None => Err(Error::UnsupportedTransferSyntax(transfer_syntax.to_owned())),
    }
}

fn decode_native(
    file: &DicomFile,
    mut descriptor: ImageDescriptor,
    frame: u32,
) -> Result<PixelFrame> {
    let is_rle = file.transfer_syntax() == "1.2.840.10008.1.2.5";
    if descriptor.bits_allocated == 1 {
        return decode_bit_packed(file, descriptor, frame);
    }
    if descriptor.photometric_interpretation == "YBR_FULL_422" && !is_rle {
        return decode_native_ybr_422(file, descriptor, frame);
    }
    let decoded = file
        .object()
        .decode_pixel_data_frame(frame)
        .map_err(|error| Error::PixelData(error.to_string()))?;
    // The RLE decoder returns interleaved samples whatever the Planar Configuration says.
    descriptor.is_planar =
        decoded.planar_configuration() == dicom_pixeldata::PlanarConfiguration::PixelFirst;
    let samples = samples::from_le_bytes(decoded.data(), &descriptor)?;
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

/// Raw bytes of the whole native Pixel Data.
fn native_pixel_bytes(file: &DicomFile) -> Result<std::borrow::Cow<'_, [u8]>> {
    file.object()
        .element(tags::PIXEL_DATA)
        .map_err(|error| Error::PixelData(error.to_string()))?
        .to_bytes()
        .map_err(|error| Error::PixelData(error.to_string()))
}

/// 1-bit frames (binary segmentations). In native data they are not byte-aligned: a frame
/// starts at bit `frame × samples per frame` of the whole Pixel Data (PS3.5 §8.1.1).
/// Encapsulated frames (Deflated Image Frame Compression) are each decoded on their own.
fn decode_bit_packed(
    file: &DicomFile,
    descriptor: ImageDescriptor,
    frame: u32,
) -> Result<PixelFrame> {
    let count = descriptor.sample_count();
    let is_encapsulated = file
        .object()
        .element(tags::PIXEL_DATA)
        .is_ok_and(|element| element.value().fragments().is_some());
    let samples = if is_encapsulated {
        let decoded = file
            .object()
            .decode_pixel_data_frame(frame)
            .map_err(|error| Error::PixelData(error.to_string()))?;
        samples::unpack_bits(decoded.data(), 0, count)
    } else {
        let first_bit =
            usize::try_from(frame).map_err(|e| Error::PixelData(e.to_string()))? * count;
        samples::unpack_bits(&native_pixel_bytes(file)?, first_bit, count)
    };
    let samples = Samples::U8(samples);
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

/// Native YBR_FULL_422 stores two samples per pixel (`Y1 Y2 Cb Cr` per pixel pair, PS3.3
/// §C.7.6.3.1.2), which `dicom-pixeldata` does not handle; the chroma is upsampled by `to_rgb`.
fn decode_native_ybr_422(
    file: &DicomFile,
    descriptor: ImageDescriptor,
    frame: u32,
) -> Result<PixelFrame> {
    if descriptor.bits_allocated != 8 {
        return Err(Error::PixelData(
            "YBR_FULL_422 is supported for 8-bit samples only".to_owned(),
        ));
    }
    let bytes = native_pixel_bytes(file)?;
    let frame_length = descriptor.pixel_count() * 2;
    let start = usize::try_from(frame).map_err(|e| Error::PixelData(e.to_string()))? * frame_length;
    let frame_bytes = bytes
        .get(start..start + frame_length)
        .ok_or_else(|| Error::PixelData("pixel data is too short".to_owned()))?;
    let samples = Samples::U8(frame_bytes.to_vec());
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

fn decode_jpeg(
    file: &DicomFile,
    mut descriptor: ImageDescriptor,
    frame: u32,
    is_lossy: bool,
) -> Result<PixelFrame> {
    let bytes = compressed_frame(file, frame)?;
    let codec_error = |error: oxiarc_jpeg::JpegError| Error::PixelData(error.to_string());
    let mut decoder = if is_lossy {
        // Lossy JPEG: the decoder converts YCbCr to RGB according to the JPEG header, as the
        // IJG decoder used by the Java viewer does.
        oxiarc_jpeg::Decoder::new(&bytes[..])
    } else {
        oxiarc_jpeg::Decoder::with_options(&bytes[..], oxiarc_jpeg::DecodeOptions::raw())
    };
    let info = decoder.read_info().map_err(codec_error)?;
    let values: Vec<u16> = if info.precision > 8 {
        decoder.decode_u16().map_err(codec_error)?
    } else {
        decoder
            .decode()
            .map_err(codec_error)?
            .into_iter()
            .map(u16::from)
            .collect()
    };
    if is_lossy && descriptor.samples_per_pixel == 3 {
        descriptor.photometric_interpretation = "RGB".to_owned();
    }
    // A JPEG stream is always interleaved, whatever the Planar Configuration says.
    descriptor.is_planar = false;
    let samples = samples::from_unsigned_codec(values, &descriptor);
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

fn decode_jpeg_ls(
    file: &DicomFile,
    mut descriptor: ImageDescriptor,
    frame: u32,
) -> Result<PixelFrame> {
    if descriptor.samples_per_pixel != 1 {
        return Err(Error::NeedsServerDecoding(format!(
            "JPEG-LS with {} components",
            descriptor.samples_per_pixel
        )));
    }
    let bytes = compressed_frame(file, frame)?;
    let (values, _, _) = jpegls::decode(&bytes, descriptor.columns, descriptor.rows)
        .map_err(|error| Error::PixelData(format!("{error:?}")))?;
    descriptor.is_planar = false;
    let samples = samples::from_unsigned_codec(values, &descriptor);
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

fn decode_jpeg_2000(
    file: &DicomFile,
    mut descriptor: ImageDescriptor,
    frame: u32,
) -> Result<PixelFrame> {
    let bytes = compressed_frame(file, frame)?;
    let image = oxideav_jpeg2000::decode_j2k(&bytes)
        .map_err(|error| Error::PixelData(format!("{error:?}")))?;
    let pixel_count = image.components.first().map_or(0, |c| c.samples.len());
    let mut values = Vec::with_capacity(pixel_count * image.components.len());
    for index in 0..pixel_count {
        values.extend(image.components.iter().filter_map(|c| c.samples.get(index)));
    }
    // PS3.5 §8.2.4: with YBR_ICT / YBR_RCT the codestream's multi-component transform has been
    // inverted by the decoder, so the samples are RGB.
    if matches!(
        descriptor.photometric_interpretation.as_str(),
        "YBR_ICT" | "YBR_RCT"
    ) {
        descriptor.photometric_interpretation = "RGB".to_owned();
    }
    descriptor.is_planar = false;
    let samples = samples::from_i32_values(values, &descriptor);
    Ok(PixelFrame {
        descriptor,
        samples,
    })
}

/// Float Pixel Data and Double Float Pixel Data (parametric maps), as `f32`.
fn float_samples(
    file: &DicomFile,
    descriptor: &ImageDescriptor,
    frame: u32,
) -> Result<Option<Samples>> {
    let object = file.object();
    let values: Vec<f32> = if let Ok(element) = object.element(tags::FLOAT_PIXEL_DATA) {
        element
            .to_multi_float32()
            .map_err(|error| Error::PixelData(error.to_string()))?
    } else if let Ok(element) = object.element(tags::DOUBLE_FLOAT_PIXEL_DATA) {
        // NOTE: doubles are narrowed to f32, enough for display and measurements.
        #[allow(clippy::cast_possible_truncation)]
        element
            .to_multi_float64()
            .map_err(|error| Error::PixelData(error.to_string()))?
            .into_iter()
            .map(|value| value as f32)
            .collect()
    } else {
        return Ok(None);
    };
    let count = descriptor.sample_count();
    let start = usize::try_from(frame).map_err(|e| Error::PixelData(e.to_string()))? * count;
    let frame_values = values
        .get(start..start + count)
        .ok_or_else(|| Error::PixelData("float pixel data is too short".to_owned()))?;
    Ok(Some(Samples::F32(frame_values.to_vec())))
}
