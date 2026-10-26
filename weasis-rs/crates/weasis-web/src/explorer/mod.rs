//! The DICOM explorer: loading local data and the patients, studies and series loaded.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/`.

mod dicom_explorer;
mod import_report;
mod importer;
mod loading_task;

pub use dicom_explorer::DicomExplorer;
pub use import_report::ImportReport;
pub use importer::Importer;
