//! Which transfer syntaxes can be decoded with the pure-Rust codecs chosen in decision D14.
//!
//! The same test runs natively (`cargo test`) and inside WebAssembly
//! (`cargo test --target wasm32-unknown-unknown`), so it proves what the browser can decode by
//! itself (decision D3 in `docs/PLAN.md`). Pixel accuracy against the C codecs (libjpeg,
//! CharLS, OpenJPEG) was measured in step 0.4; see `docs/PROGRESS.md`.
//!
//! Video transfer syntaxes (MPEG-2, H.264, HEVC) are not listed: their pixel data is a video
//! stream that is played by the browser, not decoded into frames by `weasis-core`.
//!
//! The decoding helpers below are a feasibility check only. The real codec module is built in
//! step 1.3 and this test will then call it instead.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/.../TransferSyntax.java` and
//! `weasis-dicom-tools/.../org/dcm4che3/img/DicomImageReader.java`.

use dicom_dictionary_std::tags;
use dicom_object::file::ReadPreamble;
use dicom_object::{DefaultDicomObject, OpenFileOptions};
use dicom_pixeldata::PixelDecoder;
use wasm_bindgen_test::wasm_bindgen_test;

/// The codec expected to decode a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Codec {
    /// Native, deflated or RLE data, decoded by `dicom-pixeldata`.
    DicomPixelData,
    /// JPEG baseline, extended and lossless, decoded by `oxiarc-jpeg`.
    Jpeg,
    /// Single-component JPEG-LS, decoded by `pure_jpegls`.
    JpegLs,
    /// JPEG 2000 and HTJ2K, decoded by `oxideav-jpeg2000`.
    Jpeg2000,
    /// No browser codec: the server has to decode it.
    Server,
}

struct Sample {
    file: &'static str,
    bytes: &'static [u8],
    rows: u32,
    columns: u32,
    codec: Codec,
}

/// Embeds a file from `weasis-rs/testdata/` (WebAssembly tests have no file system).
macro_rules! sample {
    ($file:literal, $rows:literal x $columns:literal, $codec:ident) => {
        Sample {
            file: $file,
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../testdata/",
                $file
            )),
            rows: $rows,
            columns: $columns,
            codec: Codec::$codec,
        }
    };
}

/// One sample per transfer syntax listed in `testdata/README.md`.
const SAMPLES: &[Sample] = &[
    // 1.2.840.10008.1.2 Implicit VR Little Endian
    sample!("pydicom/MR_small_implicit.dcm", 64 x 64, DicomPixelData),
    // 1.2.840.10008.1.2.1 Explicit VR Little Endian
    sample!("pydicom/MR_small.dcm", 64 x 64, DicomPixelData),
    // 1.2.840.10008.1.2.2 Explicit VR Big Endian
    sample!("pydicom/MR_small_bigendian.dcm", 64 x 64, DicomPixelData),
    // 1.2.840.10008.1.2.1.99 Deflated Explicit VR Little Endian
    sample!("pydicom/image_dfl.dcm", 512 x 512, DicomPixelData),
    // 1.2.840.10008.1.2.8.1 Deflated Image Frame Compression
    sample!("pydicom/liver_deflate.dcm", 512 x 512, DicomPixelData),
    // 1.2.840.10008.1.2.5 RLE Lossless
    sample!("pydicom/MR_small_RLE.dcm", 64 x 64, DicomPixelData),
    // 1.2.840.10008.1.2.4.50 JPEG Baseline
    sample!("pydicom/SC_rgb_jpeg_dcmtk.dcm", 100 x 100, Jpeg),
    // 1.2.840.10008.1.2.4.51 JPEG Extended 12-bit
    sample!("pydicom/JPGExtended.dcm", 1024 x 256, Jpeg),
    // 1.2.840.10008.1.2.4.70 JPEG Lossless SV1
    sample!("pydicom/JPEG-LL.dcm", 1024 x 256, Jpeg),
    sample!("weasis-dicom-tools/CT-JPEGLosslessSV1.dcm", 512 x 512, Jpeg),
    // 1.2.840.10008.1.2.4.80 JPEG-LS Lossless
    sample!("pydicom/MR_small_jpeg_ls_lossless.dcm", 64 x 64, JpegLs),
    sample!("pydicom/JLSL_16_15_1_1F.dcm", 128 x 128, JpegLs),
    sample!("weasis-dicom-tools/palette-multiframe-jpeg-ls.dcm", 430 x 600, JpegLs),
    // 1.2.840.10008.1.2.4.81 JPEG-LS Near-Lossless, three components (RGB)
    sample!("pydicom/SC_rgb_jls_lossy_line.dcm", 100 x 100, Server),
    // 1.2.840.10008.1.2.4.90 JPEG 2000 Lossless
    sample!("pydicom/MR_small_jp2klossless.dcm", 64 x 64, Jpeg2000),
    // 1.2.840.10008.1.2.4.91 JPEG 2000
    sample!("pydicom/US1_J2KI.dcm", 480 x 640, Jpeg2000),
    sample!("weasis-dicom-tools/jpeg2000-multiframe-multifragments.dcm", 256 x 256, Jpeg2000),
    // 1.2.840.10008.1.2.4.201 HTJ2K Lossless
    sample!("pydicom/HTJ2KLossless_08_RGB.dcm", 480 x 640, Jpeg2000),
    // 1.2.840.10008.1.2.4.203 HTJ2K
    sample!("pydicom/HTJ2K_08_RGB.dcm", 480 x 640, Jpeg2000),
];

fn read_object(sample: &Sample) -> Result<DefaultDicomObject, String> {
    OpenFileOptions::new()
        .read_preamble(ReadPreamble::Auto)
        .from_reader(sample.bytes)
        .map_err(|error| format!("cannot read: {error}"))
}

/// Compressed bytes of the first frame (DICOM PS3.5 A.4: a frame may span several fragments).
fn first_frame_bytes(object: &DefaultDicomObject) -> Result<Vec<u8>, String> {
    let pixel_data = object
        .element(tags::PIXEL_DATA)
        .map_err(|error| format!("no pixel data: {error}"))?
        .value();
    let fragments = pixel_data
        .fragments()
        .ok_or("pixel data is not encapsulated")?;
    let first = fragments.first().ok_or("no fragment")?;

    // A new frame starts with a JPEG SOI or a JPEG 2000 SOC marker.
    let starts_new_frame = |fragment: &&Vec<u8>| {
        fragment.starts_with(&[0xFF, 0xD8]) || fragment.starts_with(&[0xFF, 0x4F])
    };
    let mut frame = first.clone();
    for fragment in fragments
        .iter()
        .skip(1)
        .take_while(|f| !starts_new_frame(f))
    {
        frame.extend_from_slice(fragment);
    }
    Ok(frame)
}

/// Decodes the first frame with the given codec and returns its size as (rows, columns).
fn decode_first_frame(object: &DefaultDicomObject, codec: Codec) -> Result<(u32, u32), String> {
    match codec {
        Codec::DicomPixelData => {
            let frame = object
                .decode_pixel_data_frame(0)
                .map_err(|e| e.to_string())?;
            Ok((frame.rows(), frame.columns()))
        }
        Codec::Jpeg => {
            let bytes = first_frame_bytes(object)?;
            let mut decoder = oxiarc_jpeg::Decoder::new(&bytes[..]);
            let info = decoder.read_info().map_err(|e| e.to_string())?;
            decoder.decode_u16().map_err(|e| e.to_string())?;
            Ok((u32::from(info.height), u32::from(info.width)))
        }
        Codec::JpegLs => {
            let bytes = first_frame_bytes(object)?;
            let rows = object_u32(object, tags::ROWS)?;
            let columns = object_u32(object, tags::COLUMNS)?;
            let (_, width, height) =
                jpegls::decode(&bytes, columns, rows).map_err(|e| format!("{e:?}"))?;
            Ok((height, width))
        }
        Codec::Jpeg2000 => {
            let bytes = first_frame_bytes(object)?;
            let image = oxideav_jpeg2000::decode_j2k(&bytes).map_err(|e| format!("{e:?}"))?;
            Ok((image.height, image.width))
        }
        Codec::Server => Err("no browser codec".to_owned()),
    }
}

fn object_u32(object: &DefaultDicomObject, tag: dicom_object::Tag) -> Result<u32, String> {
    let element = object
        .element(tag)
        .map_err(|error| format!("missing {tag}: {error}"))?;
    element
        .to_int::<u32>()
        .map_err(|error| format!("invalid {tag}: {error}"))
}

/// Checks a sample; `Server` samples are tried with every browser codec and must all fail.
fn check(sample: &Sample) -> Result<(), String> {
    let object = read_object(sample)?;
    if sample.codec == Codec::Server {
        let browser_codecs = [Codec::Jpeg, Codec::JpegLs, Codec::Jpeg2000];
        return match browser_codecs
            .into_iter()
            .find(|c| decode_first_frame(&object, *c).is_ok())
        {
            Some(codec) => Err(format!("now decodes with {codec:?}: update the table")),
            None => Ok(()),
        };
    }
    match decode_first_frame(&object, sample.codec)? {
        size if size == (sample.rows, sample.columns) => Ok(()),
        (rows, columns) => Err(format!("decoded as {rows}x{columns}")),
    }
}

#[test]
#[wasm_bindgen_test]
fn transfer_syntax_support_matches_expectation() {
    let failures: Vec<String> = SAMPLES
        .iter()
        .filter_map(|sample| {
            check(sample)
                .err()
                .map(|error| format!("{}: {error}", sample.file))
        })
        .collect();

    assert!(
        failures.is_empty(),
        "unexpected codec support:\n{}",
        failures.join("\n")
    );
}
