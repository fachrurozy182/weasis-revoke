//! Importing through a DICOMDIR or from a ZIP archive.
//!
//! NOTE: the desktop application builds the series from the DICOMDIR records, then reads the
//! files; here the referenced files are read directly, which gives the same model from the
//! attributes of the files themselves.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/imp/DicomDirLoader.java`
//! and `DicomZipMediaIO.java` (`loadDicomZip`).

use super::local_import::{FileKind, HEAD_LENGTH, ImportedFile, classify, read_file};
use crate::dicom::{DICOMDIR, read_dicomdir, read_zip, resolve_file_id, root_dicomdir};
use crate::error::{Error, Result};

/// Paths of the files a DICOMDIR references, among `files` (paths relative to its folder), in
/// record order and without duplicates.
pub fn dicomdir_files(dicomdir: &[u8], files: &[String]) -> Result<Vec<String>> {
    let mut paths: Vec<String> = Vec::new();
    for series in read_dicomdir(dicomdir)? {
        for instance in series.instances {
            if let Some(path) = resolve_file_id(&instance.file_id, files)
                && !paths.iter().any(|p| p == path)
            {
                paths.push(path.to_owned());
            }
        }
    }
    Ok(paths)
}

/// Files of a folder to read through its DICOMDIR, as indexes into `paths` (path of each file in
/// the folder): the files referenced by the shallowest DICOMDIR, whose bytes `read` gives.
/// `None` when there is no readable DICOMDIR or it references none of the files.
pub fn dicomdir_selection(
    paths: &[String],
    read: impl FnOnce(usize) -> Option<Vec<u8>>,
) -> Option<Vec<usize>> {
    let (index, root) = paths
        .iter()
        .enumerate()
        .filter_map(|(i, path)| {
            let (root, name) = path.rsplit_once('/').unwrap_or(("", path));
            name.eq_ignore_ascii_case(DICOMDIR).then_some((i, root))
        })
        // Depth of the folder of the DICOMDIR.
        .min_by_key(|(_, root)| match root.is_empty() {
            true => 0,
            false => root.matches('/').count() + 1,
        })?;
    let prefix = if root.is_empty() {
        String::new()
    } else {
        format!("{root}/")
    };
    let relative: Vec<String> = paths
        .iter()
        .map(|p| p.strip_prefix(&prefix).unwrap_or_default().to_owned())
        .collect();
    let listed = dicomdir_files(&read(index)?, &relative).ok()?;
    let selected: Vec<usize> = listed
        .iter()
        .filter_map(|path| relative.iter().position(|r| r == path))
        .collect();
    (!selected.is_empty()).then_some(selected)
}

/// Reads the DICOM files of the ZIP archive at index `file` of a request: through the DICOMDIR
/// at its root when it has one, else every DICOM entry, archives inside included.
pub fn read_archive(
    file: usize,
    name: &str,
    bytes: &[u8],
    password: Option<&[u8]>,
) -> ImportedFile {
    let mut imported = ImportedFile {
        file,
        ..ImportedFile::default()
    };
    let entries = match read_zip(bytes, password) {
        Ok(entries) => entries,
        Err(Error::PasswordRequired) => {
            imported.password_required = true;
            return imported;
        }
        Err(error) => {
            imported.errors.push(format!("{name}: {error}"));
            return imported;
        }
    };
    let paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let selected = match root_dicomdir(&entries).map(|d| dicomdir_files(&d.bytes, &paths)) {
        Some(Ok(listed)) if !listed.is_empty() => listed,
        // NOTE: an unreadable DICOMDIR does not stop the import: every entry is tried instead.
        _ => paths,
    };
    for entry in entries.iter().filter(|e| selected.contains(&e.path)) {
        let entry_name = format!("{name}/{}", entry.path);
        let head = &entry.bytes[..entry.bytes.len().min(HEAD_LENGTH)];
        let read = match classify(&entry.path, head) {
            FileKind::Dicom => read_file(file, &entry_name, &entry.bytes),
            FileKind::Zip => read_archive(file, &entry_name, &entry.bytes, password),
            FileKind::Dicomdir | FileKind::Sidecar | FileKind::Other => continue,
        };
        imported.instances.extend(read.instances);
        imported.unsupported += read.unsupported;
        imported.errors.extend(read.errors);
        imported.password_required |= read.password_required;
    }
    imported
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_file(name: &str) -> Vec<u8> {
        let path = format!("{}/../../testdata/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(path).expect("test data")
    }

    #[test]
    fn an_archive_gives_the_instances_of_its_files() {
        let read = read_archive(2, "ct.zip", &test_file("synthetic/ct-series.zip"), None);
        assert_eq!(read.file, 2);
        assert!(!read.instances.is_empty());
        assert!(read.errors.is_empty(), "{:?}", read.errors);
        assert!(!read.password_required);
        let encrypted = test_file("synthetic/ct-series-encrypted.zip");
        assert!(read_archive(0, "e.zip", &encrypted, None).password_required);
        let read = read_archive(0, "e.zip", &encrypted, Some(b"weasis"));
        assert_eq!(read.instances.len(), 1, "traditional ZIP encryption");
    }

    #[test]
    fn a_dicomdir_lists_the_files_it_references() {
        let dicomdir = test_file("pydicom/dicomdir-tiny-alpha/DICOMDIR");
        let files = vec![
            "PT000000/ST000000/SE000000/IM000000".to_owned(),
            "README".to_owned(),
        ];
        let listed = dicomdir_files(&dicomdir, &files).expect("readable");
        assert_eq!(listed, ["PT000000/ST000000/SE000000/IM000000"]);
        assert!(dicomdir_files(b"not dicom", &files).is_err());
    }

    #[test]
    fn the_shallowest_dicomdir_of_a_folder_selects_its_files() {
        let paths: Vec<String> = [
            "cd/README",
            "cd/sub/DICOMDIR",
            "cd/DICOMDIR",
            "cd/PT000000/ST000000/SE000000/IM000001",
        ]
        .map(str::to_owned)
        .to_vec();
        let dicomdir = test_file("pydicom/dicomdir-tiny-alpha/DICOMDIR");
        let read = |index: usize| (index == 2).then(|| dicomdir.clone());
        assert_eq!(dicomdir_selection(&paths, read), Some(vec![3]));
        assert_eq!(
            dicomdir_selection(&paths[..2], |_| Some(dicomdir.clone())),
            None
        );
        assert_eq!(
            dicomdir_selection(&paths[3..], |_| None),
            None,
            "no DICOMDIR"
        );
    }

    #[test]
    fn an_encrypted_archive_asks_for_its_password() {
        use std::io::{Cursor, Write};
        use zip::write::SimpleFileOptions;
        use zip::{AesMode, ZipWriter};
        let options = SimpleFileOptions::default().with_aes_encryption(AesMode::Aes256, "secret");
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("inner/ct.dcm", options).expect("entry");
        writer
            .write_all(&test_file("pydicom/CT_small.dcm"))
            .expect("content");
        // An archive inside the archive is read too.
        let inner = writer.finish().expect("archive").into_inner();
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer.start_file("nested.zip", options).expect("entry");
        writer.write_all(&inner).expect("content");
        let bytes = writer.finish().expect("archive").into_inner();

        assert!(read_archive(0, "a.zip", &bytes, None).password_required);
        assert!(read_archive(0, "a.zip", &bytes, Some(b"wrong")).password_required);
        let read = read_archive(0, "a.zip", &bytes, Some(b"secret"));
        assert_eq!((read.instances.len(), read.password_required), (1, false));
    }

    #[test]
    fn a_broken_archive_is_an_error() {
        let read = read_archive(0, "bad.zip", b"PK\x03\x04broken", None);
        assert_eq!(read.errors.len(), 1);
        assert!(read.errors[0].starts_with("bad.zip: "));
    }
}
