//! Every image transfer syntax decodes with the codecs of `weasis_core::dicom::pixel`.
//!
//! The same test runs natively (`cargo test`) and inside WebAssembly
//! (`cargo test --target wasm32-unknown-unknown`), so it proves what the browser can decode by
//! itself (decisions D3 and D14 in `docs/PLAN.md`). Pixel accuracy is checked natively by
//! `pixel_decoding.rs`.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/.../TransferSyntax.java`

use wasm_bindgen_test::wasm_bindgen_test;
use weasis_core::dicom::pixel::{decode_frame, to_rgb};
use weasis_core::dicom::read_dicom;
use weasis_core::error::Error;

/// Whether the browser can decode a sample by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Support {
    Browser,
    /// Must be decoded by the server (decision D14).
    Server,
}

struct Sample {
    file: &'static str,
    bytes: &'static [u8],
    rows: u32,
    columns: u32,
    support: Support,
}

/// Embeds a file from `weasis-rs/testdata/` (WebAssembly tests have no file system).
macro_rules! sample {
    ($file:literal, $rows:literal x $columns:literal, $support:ident) => {
        Sample {
            file: $file,
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../testdata/",
                $file
            )),
            rows: $rows,
            columns: $columns,
            support: Support::$support,
        }
    };
}

/// One sample per transfer syntax listed in `testdata/README.md`.
#[rustfmt::skip]
const SAMPLES: &[Sample] = &[
    sample!("pydicom/MR_small_implicit.dcm", 64 x 64, Browser),                // Implicit VR LE
    sample!("pydicom/MR_small.dcm", 64 x 64, Browser),                         // Explicit VR LE
    sample!("pydicom/MR_small_bigendian.dcm", 64 x 64, Browser),               // Explicit VR BE
    sample!("pydicom/image_dfl.dcm", 512 x 512, Browser),                      // Deflated
    sample!("pydicom/liver_deflate.dcm", 512 x 512, Browser),                  // Deflated Image Frame
    sample!("pydicom/MR_small_RLE.dcm", 64 x 64, Browser),                     // RLE
    sample!("weasis-dicom-tools/ybrFull-RLE.dcm", 480 x 640, Browser),         // RLE, YBR_FULL
    sample!("pydicom/SC_rgb_jpeg_dcmtk.dcm", 100 x 100, Browser),              // JPEG Baseline
    sample!("pydicom/JPGExtended.dcm", 1024 x 256, Browser),                   // JPEG Extended 12-bit
    sample!("pydicom/JPEG-LL.dcm", 1024 x 256, Browser),                       // JPEG Lossless SV1
    sample!("pydicom/MR_small_jpeg_ls_lossless.dcm", 64 x 64, Browser),        // JPEG-LS Lossless
    sample!("weasis-dicom-tools/palette-multiframe-jpeg-ls.dcm", 430 x 600, Browser),
    sample!("pydicom/SC_rgb_jls_lossy_line.dcm", 100 x 100, Server),           // JPEG-LS, RGB
    sample!("pydicom/MR_small_jp2klossless.dcm", 64 x 64, Browser),            // JPEG 2000 Lossless
    sample!("pydicom/US1_J2KI.dcm", 480 x 640, Browser),                       // JPEG 2000
    sample!("pydicom/HTJ2KLossless_08_RGB.dcm", 480 x 640, Browser),           // HTJ2K Lossless
    sample!("pydicom/HTJ2K_08_RGB.dcm", 480 x 640, Browser),                   // HTJ2K
];

/// Decodes the first frame to displayable samples and returns its size.
fn decode(sample: &Sample) -> Result<(u32, u32), Error> {
    let file = read_dicom(sample.bytes)?;
    let frame = to_rgb(decode_frame(&file, 0)?, &file)?;
    Ok((frame.descriptor.rows, frame.descriptor.columns))
}

fn check(sample: &Sample) -> Result<(), String> {
    match (sample.support, decode(sample)) {
        (Support::Browser, Ok(size)) if size == (sample.rows, sample.columns) => Ok(()),
        (Support::Browser, Ok((rows, columns))) => Err(format!("decoded as {rows}x{columns}")),
        (Support::Browser, Err(error)) => Err(error.to_string()),
        (Support::Server, Err(Error::NeedsServerDecoding(_))) => Ok(()),
        (Support::Server, other) => Err(format!("expected NeedsServerDecoding, got {other:?}")),
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
