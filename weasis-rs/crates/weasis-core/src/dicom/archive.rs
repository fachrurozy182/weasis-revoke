//! Opening a ZIP archive of DICOM files.
//!
//! The archive is read in memory. When it holds a `DICOMDIR` at its root, the caller imports
//! through it ([`read_dicomdir`](super::read_dicomdir)); otherwise every file is tried as DICOM,
//! as `LoadLocalDicom` does with the extracted folder in the Java viewer.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/imp/DicomZipMediaIO.java`
//! (`loadDicomZip`).

use std::io::{Cursor, Read};
use std::path::Component;

use zip::ZipArchive;
use zip::result::ZipError;

use crate::error::{Error, Result};

/// Name of the index file of a DICOM file-set (PS3.10 §8.6).
pub const DICOMDIR: &str = "DICOMDIR";

/// A file of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    /// Path inside the archive, components separated by `/`.
    pub path: String,
    /// Uncompressed content.
    pub bytes: Vec<u8>,
}

/// Files of a ZIP archive, in archive order. Folders are left out, as well as entries whose
/// path would leave the extraction folder (`..`); a leading `/` is dropped.
///
/// Fails with [`Error::PasswordRequired`] when an entry is encrypted and `password` is missing
/// or wrong, so that the caller can ask for it.
pub fn read_zip(bytes: &[u8], password: Option<&[u8]>) -> Result<Vec<ArchiveEntry>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(archive_error)?;
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = match password {
            Some(password) => archive.by_index_decrypt(index, password),
            None => archive.by_index(index),
        }
        .map_err(archive_error)?;
        let Some(path) = file.enclosed_name().and_then(|p| relative_path(&p)) else {
            continue;
        };
        if file.is_dir() {
            continue;
        }
        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|e| archive_error(e.into()))?;
        entries.push(ArchiveEntry {
            path,
            bytes: content,
        });
    }
    Ok(entries)
}

/// The `DICOMDIR` at the root of the archive, if any.
pub fn root_dicomdir(entries: &[ArchiveEntry]) -> Option<&ArchiveEntry> {
    entries.iter().find(|entry| entry.path == DICOMDIR)
}

/// Normal components only, joined with `/`.
fn relative_path(path: &std::path::Path) -> Option<String> {
    let parts: Option<Vec<&str>> = path
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    parts.filter(|p| !p.is_empty()).map(|p| p.join("/"))
}

fn archive_error(error: ZipError) -> Error {
    match error {
        ZipError::InvalidPassword => Error::PasswordRequired,
        ZipError::UnsupportedArchive(reason) if reason == ZipError::PASSWORD_REQUIRED => {
            Error::PasswordRequired
        }
        other => Error::Archive(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;
    use zip::{AesMode, ZipWriter};

    use super::*;
    use crate::dicom::read_dicom;

    fn zip_of(files: &[(&str, &[u8])], options: SimpleFileOptions) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in files {
            writer.start_file(*name, options).expect("entry");
            writer.write_all(content).expect("content");
        }
        writer.finish().expect("zip").into_inner()
    }

    #[test]
    fn the_synthetic_ct_archive_holds_sixteen_dicom_files() {
        let path = format!(
            "{}/../../testdata/synthetic/ct-series.zip",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
        let entries = read_zip(&bytes, None).expect("archive");
        assert_eq!(entries.len(), 16);
        assert_eq!(entries[0].path, "ct-series/CT001.dcm");
        assert!(entries.iter().all(|e| read_dicom(&e.bytes).is_ok()));
        assert_eq!(root_dicomdir(&entries), None);
    }

    #[test]
    fn encrypted_archives_need_their_password() {
        let options = SimpleFileOptions::default().with_aes_encryption(AesMode::Aes256, "secret");
        let bytes = zip_of(&[("DICOMDIR", b"index")], options);
        assert!(matches!(
            read_zip(&bytes, None),
            Err(Error::PasswordRequired)
        ));
        assert!(matches!(
            read_zip(&bytes, Some(b"wrong")),
            Err(Error::PasswordRequired)
        ));
        let entries = read_zip(&bytes, Some(b"secret")).expect("decrypted");
        assert_eq!(
            root_dicomdir(&entries).map(|e| e.bytes.as_slice()),
            Some(&b"index"[..])
        );
    }

    #[test]
    fn entries_outside_the_archive_folder_are_skipped() {
        let bytes = zip_of(
            &[
                ("../evil.dcm", b"x"),
                ("/abs.dcm", b"y"),
                ("ok/a.dcm", b"z"),
            ],
            SimpleFileOptions::default(),
        );
        let paths: Vec<String> = read_zip(&bytes, None)
            .expect("archive")
            .into_iter()
            .map(|e| e.path)
            .collect();
        assert_eq!(paths, ["abs.dcm", "ok/a.dcm"]);
    }
}
