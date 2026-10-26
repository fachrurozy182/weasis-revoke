//! Decoding pixel data into stored sample values.

mod decode;
mod descriptor;
mod samples;

pub use decode::{PixelFrame, decode_frame};
pub use descriptor::ImageDescriptor;
pub use samples::Samples;
