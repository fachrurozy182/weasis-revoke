//! Decoding of text attributes in the character sets of DICOM PS3.3 C.12.1.1.2.
//!
//! The decoding itself is done by `dicom-rs`; this test checks that names written by real
//! devices in each character set reach the model as the expected Unicode text. Expected values
//! come from `testdata/pydicom/charset/FileInfo.txt` (decoded with pydicom).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/.../display/CharsetEncoding.java` (list of
//! supported character sets) and dcm4che `SpecificCharacterSet`.

// Test helpers may panic: a missing test file is a broken checkout, not a recoverable error.
#![allow(clippy::panic)]

use dicom_dictionary_std::tags;
use weasis_core::dicom::{new_instance, read_dicom};

fn patient_name(file: &str) -> Option<String> {
    let path = format!(
        "{}/../../testdata/pydicom/charset/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let instance = new_instance(&read_dicom(&bytes).ok()?).ok()?;
    instance.patient.get(tags::PATIENT_NAME).map(str::to_owned)
}

/// (file, Specific Character Set, expected Patient's Name)
const CASES: &[(&str, &str, &str)] = &[
    ("chrFren.dcm", "ISO_IR 100", "Buc^Jérôme"),
    ("chrGerm.dcm", "ISO_IR 100", "Äneas^Rüdiger"),
    ("chrGreek.dcm", "ISO_IR 126", "Διονυσιος"),
    ("chrRuss.dcm", "ISO_IR 144", "Люкceмбypг"),
    ("chrArab.dcm", "ISO_IR 127", "قباني^لنزار"),
    ("chrHbrw.dcm", "ISO_IR 138", "שרון^דבורה"),
    ("chrX1.dcm", "ISO_IR 192", "Wang^XiaoDong=王^小東="),
    ("chrX2.dcm", "GB18030", "Wang^XiaoDong=王^小东="),
    (
        "chrH31.dcm",
        "\\ISO 2022 IR 87",
        "Yamada^Tarou=山田^太郎=やまだ^たろう",
    ),
    (
        "chrH32.dcm",
        "ISO 2022 IR 13\\ISO 2022 IR 87",
        "ﾔﾏﾀﾞ^ﾀﾛｳ=山田^太郎=やまだ^たろう",
    ),
    ("chrJapMulti.dcm", "\\ISO 2022 IR 87", "やまだ^たろう"),
    ("chrKoreanMulti.dcm", "\\ISO 2022 IR 149", "김희중"),
    (
        "chrI2.dcm",
        "\\ISO 2022 IR 149",
        "Hong^Gildong=洪^吉洞=홍^길동",
    ),
];

#[test]
fn patient_names_are_decoded_in_every_character_set() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(file, charset, expected)| {
            let actual = patient_name(file);
            (actual.as_deref() != Some(expected))
                .then(|| format!("{file} ({charset}): expected {expected}, got {actual:?}"))
        })
        .collect();

    assert!(
        failures.is_empty(),
        "wrong decoding:\n{}",
        failures.join("\n")
    );
}
