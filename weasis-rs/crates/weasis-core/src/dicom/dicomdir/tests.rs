use super::*;

const FOLDER: &str = "pydicom/dicomdir-tiny-alpha";

fn dicomdir_bytes() -> Vec<u8> {
    let path = format!(
        "{}/../../testdata/{FOLDER}/DICOMDIR",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

/// Files of the test file-set, relative to its folder.
fn folder_files() -> Vec<String> {
    let root = format!("{}/../../testdata/{FOLDER}", env!("CARGO_MANIFEST_DIR"));
    let series = format!("{root}/PT000000/ST000000/SE000000");
    let mut files: Vec<String> = std::fs::read_dir(&series)
        .unwrap_or_else(|e| panic!("cannot list {series}: {e}"))
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .map(|name| format!("PT000000/ST000000/SE000000/{name}"))
        .collect();
    files.push("DICOMDIR".to_owned());
    files
}

#[test]
fn record_offsets_match_pydicom() {
    let offsets = offsets::record_offsets(&dicomdir_bytes()).expect("offsets");
    assert_eq!(offsets.len(), 53);
    assert_eq!(offsets[..3], [422, 516, 718]);
    assert_eq!(offsets.last(), Some(&12822));
}

#[test]
fn the_file_set_lists_one_series_whose_files_all_exist() {
    let series = read_dicomdir(&dicomdir_bytes()).expect("DICOMDIR");
    assert_eq!(series.len(), 1);
    let only = &series[0];
    assert_eq!(only.patient.get(tags::PATIENT_ID), Some("12345678"));
    assert_eq!(only.instances.len(), 50);
    assert_eq!(
        only.instances[0].file_id,
        ["PT000000", "ST000000", "SE000000", "IM000000"]
    );
    let files = folder_files();
    let missing: Vec<_> = only
        .instances
        .iter()
        .filter(|i| resolve_file_id(&i.file_id, &files).is_none())
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn inactive_records_are_skipped() {
    // Clear the Record In-use Flag (0004,1410) of the second image record (offset 1110).
    let mut bytes = dicomdir_bytes();
    let flag = [0x04, 0x00, 0x10, 0x14, b'U', b'S', 0x02, 0x00];
    let start = 1110
        + bytes[1110..]
            .windows(flag.len())
            .position(|w| w == flag)
            .expect("in-use flag");
    bytes[start + 8..start + 10].copy_from_slice(&[0, 0]);

    let series = read_dicomdir(&bytes).expect("DICOMDIR");
    let names: Vec<&str> = series[0]
        .instances
        .iter()
        .filter_map(|i| i.file_id.last().map(String::as_str))
        .collect();
    assert_eq!(names.len(), 49);
    assert!(!names.contains(&"IM000001"));
}

#[test]
fn file_ids_resolve_like_the_java_viewer() {
    let files: Vec<String> = [
        "dir/IMG1",
        "dir/img2",
        "dir/IMG3.dcm",
        "dir/IMG4.a",
        "dir/IMG4.b",
    ]
    .map(str::to_owned)
    .to_vec();
    let id = |parts: &[&str]| parts.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        resolve_file_id(&id(&["DIR", "IMG1"]), &files),
        None,
        "case of folders"
    );
    assert_eq!(
        resolve_file_id(&id(&["dir", "IMG1"]), &files),
        Some("dir/IMG1")
    );
    assert_eq!(
        resolve_file_id(&id(&["DIR", "IMG2"]), &files),
        Some("dir/img2")
    );
    assert_eq!(
        resolve_file_id(&id(&["dir", "IMG3"]), &files),
        Some("dir/IMG3.dcm")
    );
    assert_eq!(
        resolve_file_id(&id(&["dir", "IMG4"]), &files),
        None,
        "ambiguous"
    );
    assert_eq!(resolve_file_id(&id(&["..", "dir", "IMG1"]), &files), None);
    assert_eq!(resolve_file_id(&id(&["dir/IMG1"]), &files), None);
}
