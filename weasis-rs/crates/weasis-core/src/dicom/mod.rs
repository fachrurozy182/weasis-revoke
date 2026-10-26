//! Reading and interpreting DICOM objects.

pub mod anatomy;
mod charset;
mod dicomdir;
mod frames;
mod functional_groups;
mod instance;
mod object_kind;
pub mod pixel;
mod reader;

pub use dicomdir::{DirInstance, DirSeries, read_dicomdir, resolve_file_id};
pub use frames::compressed_frame;
pub use functional_groups::{first_item, frame_macro, item_f64, item_text};
pub use instance::new_instance;
pub use object_kind::{ObjectKind, object_kind};
pub use reader::{DicomFile, read_dicom};
