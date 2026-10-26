//! Reading and interpreting DICOM objects.

mod reader;

pub use reader::{DicomFile, read_dicom};
