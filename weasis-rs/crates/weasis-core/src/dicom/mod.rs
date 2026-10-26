//! Reading and interpreting DICOM objects.

mod charset;
mod instance;
mod reader;

pub use instance::new_instance;
pub use reader::{DicomFile, read_dicom};
