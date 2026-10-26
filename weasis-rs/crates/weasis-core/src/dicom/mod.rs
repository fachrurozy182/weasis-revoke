//! Reading and interpreting DICOM objects.

mod charset;
mod instance;
mod object_kind;
mod reader;

pub use instance::new_instance;
pub use object_kind::{ObjectKind, object_kind};
pub use reader::{DicomFile, read_dicom};
