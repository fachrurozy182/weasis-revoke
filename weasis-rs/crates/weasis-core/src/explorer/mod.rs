//! Loading DICOM data into the explorer: local files, folders, DICOMDIR and ZIP archives.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/`.

mod local_import;

pub use local_import::{FileKind, HEAD_LENGTH, ImportEvent, ImportedFile, classify, read_file};
