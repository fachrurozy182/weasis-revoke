//! Reading local files for the import: which files are DICOM (or ZIP archives, DICOMDIR,
//! graphic sidecars), and the model instances a file holds. The import worker runs these and
//! reports an [`ImportEvent`] per file.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! (`readFile`, `integrate`) and `DicomCodec.FILE_EXTENSIONS`.

use serde::{Deserialize, Serialize};

use crate::dicom::{ObjectKind, new_instance, read_dicom};
use crate::model::NewInstance;

/// Bytes to read from the start of a file to tell its kind: the DICOM preamble and prefix.
pub const HEAD_LENGTH: usize = 132;
const DICOM_EXTENSIONS: [&str; 4] = ["dcm", "dic", "dicm", "dicom"];
const ZIP_EXTENSIONS: [&str; 1] = ["zip"];
const DICM: &[u8] = b"DICM";
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
/// Graphics of an image saved next to it (`<file>.xml`).
const SIDECAR_EXTENSION: &str = "xml";

/// What a file is, for the import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A DICOM file.
    Dicom,
    /// A ZIP archive of DICOM files.
    Zip,
    /// A DICOMDIR, imported on its own rather than with the files of its folder.
    Dicomdir,
    /// Graphics of an image (step 4.9).
    Sidecar,
    /// Anything else, skipped.
    Other,
}

/// Kind of a file from its name, then from its first [`HEAD_LENGTH`] bytes.
pub fn classify(name: &str, head: &[u8]) -> FileKind {
    if name.eq_ignore_ascii_case("DICOMDIR") {
        return FileKind::Dicomdir;
    }
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some(ext) if DICOM_EXTENSIONS.contains(&ext) => return FileKind::Dicom,
        Some(ext) if ZIP_EXTENSIONS.contains(&ext) => return FileKind::Zip,
        Some(SIDECAR_EXTENSION) => return FileKind::Sidecar,
        _ => {}
    }
    if head.get(128..HEAD_LENGTH) == Some(DICM) {
        FileKind::Dicom
    } else if head.starts_with(ZIP_MAGIC) {
        FileKind::Zip
    } else {
        FileKind::Other
    }
}

/// What the import worker found in one file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportedFile {
    /// Index of the file in the import request.
    pub file: usize,
    /// Instances to add to the model.
    pub instances: Vec<NewInstance>,
    /// Valid DICOM objects that are not displayed (DICOMDIR, unknown objects).
    pub unsupported: u32,
    /// Files that could not be read, with the reason.
    pub errors: Vec<String>,
    /// An encrypted archive whose password is missing or wrong.
    pub password_required: bool,
}

/// How the import worker reads the files of a request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportRequest {
    /// Each file on its own; `password` opens encrypted ZIP archives.
    Files {
        /// Password of the archives.
        password: Option<String>,
    },
    /// The files of a folder holding a DICOMDIR, with their path from that folder: only the
    /// files the DICOMDIR references are read.
    Dicomdir {
        /// Path of each file, components separated by `/`.
        paths: Vec<String>,
    },
}

/// Messages of the import worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportEvent {
    /// The worker is started and takes requests; one sent before is lost.
    Ready,
    /// A file of the request was read.
    Read(ImportedFile),
    /// No DICOMDIR, or none of the files it references, was found in the folder.
    DicomdirEmpty,
    /// Every file of the request was read.
    Finished,
}

/// Reads the DICOM file at index `file` of a request.
pub fn read_file(file: usize, name: &str, bytes: &[u8]) -> ImportedFile {
    let mut imported = ImportedFile {
        file,
        ..ImportedFile::default()
    };
    match read_dicom(bytes).and_then(|dicom| new_instance(&dicom)) {
        Ok(instance)
            if matches!(
                instance.kind,
                ObjectKind::Directory | ObjectKind::Unsupported
            ) =>
        {
            imported.unsupported += 1;
        }
        Ok(instance) => imported.instances.push(instance),
        Err(error) => imported.errors.push(format!("{name}: {error}")),
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
    fn files_are_classified_by_extension_then_content() {
        let mut dicom_head = vec![0; 128];
        dicom_head.extend_from_slice(b"DICM");
        assert_eq!(classify("IMG0001", &dicom_head), FileKind::Dicom);
        assert_eq!(classify("a.DCM", &[]), FileKind::Dicom);
        assert_eq!(classify("study.zip", &[]), FileKind::Zip);
        assert_eq!(classify("archive", b"PK\x03\x04rest"), FileKind::Zip);
        assert_eq!(classify("dicomdir", &[]), FileKind::Dicomdir);
        assert_eq!(classify("a.dcm.xml", &[]), FileKind::Sidecar);
        assert_eq!(classify("notes.txt", b"hello"), FileKind::Other);
    }

    #[test]
    fn a_dicom_file_gives_its_instance_and_bad_files_an_error() {
        let read = read_file(3, "ct.dcm", &test_file("pydicom/CT_small.dcm"));
        assert_eq!((read.file, read.instances.len()), (3, 1));
        assert!(read.errors.is_empty());
        let bad = read_file(0, "bad.dcm", b"not a DICOM file");
        assert_eq!(bad.instances.len(), 0);
        assert!(bad.errors[0].starts_with("bad.dcm: "), "{:?}", bad.errors);
        let json = serde_json::to_string(&ImportEvent::Read(read.clone())).expect("serializable");
        assert_eq!(
            serde_json::from_str::<ImportEvent>(&json).ok(),
            Some(ImportEvent::Read(read))
        );
    }
}
