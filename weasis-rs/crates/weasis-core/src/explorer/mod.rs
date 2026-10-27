//! Loading DICOM data into the explorer: local files, folders, DICOMDIR and ZIP archives.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/`.

mod archive_import;
mod local_import;
mod related_series;
mod series_selection;

pub use archive_import::{dicomdir_files, dicomdir_selection, read_archive};
pub use local_import::{
    FileKind, HEAD_LENGTH, ImportEvent, ImportRequest, ImportedFile, classify, read_file,
};
pub use related_series::related_series;
pub use series_selection::SeriesSelection;
