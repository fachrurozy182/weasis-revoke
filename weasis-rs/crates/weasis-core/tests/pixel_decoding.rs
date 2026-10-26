//! Decoded pixel values match a reference decoder for every image in `testdata/`.
//!
//! The reference statistics (sample count, minimum, maximum, sum of the first frame) were
//! computed with pydicom 3.0.1 and the C codecs of pylibjpeg (libjpeg, OpenJPEG, CharLS):
//! `pydicom.pixels.pixel_array(path, index=0, raw=True)`, or `raw=False` for lossy JPEG / JPEG
//! 2000 color images, which those codecs convert to RGB like ours.
//!
//! Lossy codecs may round differently: their values must stay within 1 of the reference.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/DicomImageReader.java`

// Test helpers may panic: a missing test file is a broken checkout, not a recoverable error.
#![allow(clippy::panic)]

use weasis_core::dicom::pixel::{Samples, decode_frame, to_rgb};
use weasis_core::dicom::read_dicom;
use weasis_core::error::Error;

/// Expected statistics of the first frame.
struct Reference {
    file: &'static str,
    count: usize,
    min: f64,
    max: f64,
    sum: f64,
    is_lossy: bool,
}

const fn exact(file: &'static str, count: usize, min: f64, max: f64, sum: f64) -> Reference {
    Reference {
        file,
        count,
        min,
        max,
        sum,
        is_lossy: false,
    }
}

const fn lossy(file: &'static str, count: usize, min: f64, max: f64, sum: f64) -> Reference {
    Reference {
        file,
        count,
        min,
        max,
        sum,
        is_lossy: true,
    }
}

#[rustfmt::skip]
const REFERENCES: &[Reference] = &[
    // Native, all endiannesses and VR encodings.
    exact("pydicom/CT_small.dcm", 16384, 128.0, 2191.0, 14_826_310.0),
    exact("pydicom/MR_small.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    exact("pydicom/MR_small_implicit.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    exact("pydicom/MR_small_bigendian.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    exact("pydicom/MR-SIEMENS-DICOM-WithOverlays.dcm", 234_256, 0.0, 1123.0, 28_033_480.0),
    exact("pydicom/eCT_Supplemental.dcm", 262_144, 0.0, 1196.0, 100_826_003.0),
    exact("pydicom/emri_small.dcm", 4096, 0.0, 425.0, 590_962.0),
    exact("pydicom/examples_overlay.dcm", 145_200, 0.0, 1123.0, 27_833_052.0),
    exact("pydicom/examples_palette.dcm", 280_000, 0.0, 255.0, 15_024_554.0),
    exact("pydicom/US1_UNCR.dcm", 921_600, 0.0, 255.0, 31_821_736.0),
    exact("pydicom/rtdose.dcm", 100, 795_000.0, 1_254_000.0, 101_378_000.0),
    exact("pydicom/parametric_map_float.dcm", 16384, 0.0, 0.941_579_163_074_493_4, 9617.0854),
    exact("synthetic/cr-monochrome1.dcm", 16384, 0.0, 4095.0, 33_538_176.0),
    exact("synthetic/ct-series/CT008.dcm", 4096, 24.0, 2024.0, 2_570_304.0),
    exact("synthetic/pet-series/PT004.dcm", 1024, 0.0, 18752.0, 854_432.0),
    exact("synthetic/seg-sphere.dcm", 4096, 0.0, 1.0, 1396.0),
    exact("weasis-dicom-tools/US-RGB-8-epicard-Banded-bigendian.dcm", 921_600, 0.0, 255.0, 24_503_132.0),
    exact("weasis-dicom-tools/imageForPrLUTs.dcm", 262_144, 0.0, 255.0, 37_201_440.0),
    exact("weasis-dicom-tools/mono2-CT-16bit.dcm", 262_144, -2000.0, 4095.0, 42_617_199.0),
    exact("weasis-dicom-tools/overlay.dcm", 262_144, 0.0, 4095.0, 7_492_969.0),
    exact("weasis-dicom-tools/signed-raw-9bit.dcm", 262_144, 0.0, 255.0, 33_804_530.0),
    // RLE.
    exact("pydicom/MR_small_RLE.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    exact("pydicom/SC_rgb_rle.dcm", 30000, 0.0, 255.0, 3_831_000.0),
    exact("weasis-dicom-tools/ybrFull-RLE.dcm", 921_600, 0.0, 255.0, 90_146_600.0),
    // JPEG.
    lossy("pydicom/SC_rgb_jpeg_dcmtk.dcm", 30000, 0.0, 255.0, 3_832_000.0),
    lossy("pydicom/examples_ybr_color.dcm", 230_400, 0.0, 220.0, 2_184_910.0),
    lossy("pydicom/JPGExtended.dcm", 262_144, 0.0, 264.0, 3_770_427.0),
    exact("pydicom/JPEG-LL.dcm", 262_144, 0.0, 278.0, 3_596_452.0),
    exact("pydicom/JPGLosslessP14SV1_1s_1f_8b.dcm", 786_432, 0.0, 255.0, 13_572_107.0),
    exact("weasis-dicom-tools/CT-JPEGLosslessSV1.dcm", 262_144, 0.0, 3037.0, 80_826_877.0),
    exact("weasis-dicom-tools/MR-JPEGLosslessSV1.dcm", 65536, 0.0, 48.0, 978_248.0),
    // JPEG-LS.
    exact("pydicom/MR_small_jpeg_ls_lossless.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    exact("pydicom/JLSL_16_15_1_1F.dcm", 16384, -16384.0, 16383.0, -66_250_735.0),
    exact("weasis-dicom-tools/palette-multiframe-jpeg-ls.dcm", 258_000, 2.0, 255.0, 34_109_796.0),
    // JPEG 2000 and HTJ2K.
    exact("pydicom/MR_small_jp2klossless.dcm", 4096, 127.0, 2145.0, 2_125_338.0),
    lossy("pydicom/JPEG2000.dcm", 262_144, -30.0, 245.0, 3_527_976.0),
    lossy("pydicom/US1_J2KI.dcm", 921_600, 0.0, 255.0, 31_970_895.0),
    lossy("weasis-dicom-tools/jpeg2000-multiframe-multifragments.dcm", 65536, 0.0, 4124.0, 30_990_710.0),
    exact("pydicom/HTJ2KLossless_08_RGB.dcm", 921_600, 0.0, 255.0, 117_137_474.0),
    lossy("pydicom/HTJ2K_08_RGB.dcm", 921_600, 0.0, 255.0, 117_138_982.0),
];

fn decode_statistics(file: &str) -> Result<(usize, f64, f64, f64), String> {
    let path = format!("{}/../../testdata/{file}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let dicom = read_dicom(&bytes).map_err(|e| e.to_string())?;
    let frame = decode_frame(&dicom, 0).map_err(|e| e.to_string())?;
    let values: Vec<f64> = (0..frame.samples.len())
        .filter_map(|index| frame.samples.get_f64(index))
        .collect();
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok((values.len(), min, max, values.iter().sum()))
}

fn check(reference: &Reference) -> Result<(), String> {
    let (count, min, max, sum) = decode_statistics(reference.file)?;
    if count != reference.count {
        return Err(format!("{count} samples instead of {}", reference.count));
    }
    // Lossy: each sample may differ by 1, so the sum by at most one per sample.
    let (value_tolerance, sum_tolerance) = if reference.is_lossy {
        (1.0, count as f64)
    } else {
        (1e-6, 1e-2)
    };
    let close = |actual: f64, expected: f64, tolerance: f64| (actual - expected).abs() <= tolerance;
    if !close(min, reference.min, value_tolerance)
        || !close(max, reference.max, value_tolerance)
        || !close(sum, reference.sum, sum_tolerance)
    {
        return Err(format!(
            "min/max/sum {min}/{max}/{sum}, expected {}/{}/{}",
            reference.min, reference.max, reference.sum
        ));
    }
    Ok(())
}

#[test]
fn decoded_values_match_the_reference_decoder() {
    let failures: Vec<String> = REFERENCES
        .iter()
        .filter_map(|r| check(r).err().map(|error| format!("{}: {error}", r.file)))
        .collect();
    assert!(
        failures.is_empty(),
        "wrong decoding:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_frame_of_a_multiframe_image_is_decoded() {
    let path = format!(
        "{}/../../testdata/pydicom/emri_small.dcm",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let dicom = read_dicom(&bytes).expect("readable");
    for frame in 0..dicom.number_of_frames() {
        let decoded = decode_frame(&dicom, frame).expect("decodable");
        assert!(matches!(decoded.samples, Samples::U16(_)), "frame {frame}");
    }
}

#[test]
fn multi_component_jpeg_ls_is_left_to_the_server() {
    let path = format!(
        "{}/../../testdata/pydicom/SC_rgb_jls_lossy_line.dcm",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let result = decode_frame(&read_dicom(&bytes).expect("readable"), 0);
    assert!(matches!(result, Err(Error::NeedsServerDecoding(_))));
}

/// Color frames after `to_rgb`: (file, sample count, Σ value × (index mod 3 + 1), tolerance per
/// sample). The weighted sum detects swapped channels and wrong interleaving. Reference:
/// pydicom `pixel_array(raw=False)`, palette via `apply_color_lut` reduced to 8 bits.
/// YBR conversions may round differently from pydicom by 1.
#[rustfmt::skip]
const COLOR_REFERENCES: &[(&str, usize, f64, f64)] = &[
    ("pydicom/SC_ybr_full_422_uncompressed.dcm", 30000, 7_665_100.0, 1.0),
    ("weasis-dicom-tools/ybr422-raw.dcm", 1_332_000, 41_792_624.0, 1.0),
    ("weasis-dicom-tools/ybrFull-RLE.dcm", 921_600, 69_034_253.0, 1.0),
    ("pydicom/examples_palette.dcm", 840_000, 37_085_216.0, 0.0),
    ("weasis-dicom-tools/palette-multiframe-jpeg-ls.dcm", 774_000, 23_865_639.0, 0.0),
    ("weasis-dicom-tools/US-RGB-8-epicard-Banded-bigendian.dcm", 921_600, 42_628_238.0, 0.0),
    ("pydicom/US1_UNCR.dcm", 921_600, 60_061_545.0, 0.0),
];

fn weighted_rgb_sum(file: &str) -> Result<(usize, f64), String> {
    let path = format!("{}/../../testdata/{file}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let dicom = read_dicom(&bytes).map_err(|e| e.to_string())?;
    let frame = decode_frame(&dicom, 0).map_err(|e| e.to_string())?;
    let rgb = to_rgb(frame, &dicom).map_err(|e| e.to_string())?;
    if rgb.descriptor.photometric_interpretation != "RGB" || rgb.descriptor.is_planar {
        return Err(format!("not interleaved RGB: {:?}", rgb.descriptor));
    }
    let count = rgb.samples.len();
    let sum = (0..count)
        .filter_map(|i| rgb.samples.get_f64(i).map(|v| v * ((i % 3) as f64 + 1.0)))
        .sum();
    Ok((count, sum))
}

#[test]
fn color_frames_are_converted_to_interleaved_rgb() {
    let failures: Vec<String> = COLOR_REFERENCES
        .iter()
        .filter_map(|&(file, count, sum, tolerance)| {
            let result = weighted_rgb_sum(file).and_then(|(actual_count, actual_sum)| {
                // Weights go up to 3, so each sample may move the sum by 3 × tolerance.
                let max_difference = 3.0 * tolerance * count as f64;
                if actual_count != count || (actual_sum - sum).abs() > max_difference.max(1e-6) {
                    Err(format!(
                        "{actual_count} samples, sum {actual_sum}; expected {count}, {sum}"
                    ))
                } else {
                    Ok(())
                }
            });
            result.err().map(|error| format!("{file}: {error}"))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "wrong color conversion:\n{}",
        failures.join("\n")
    );
}
