//! Decoding pixel data into stored sample values.

mod color;
mod decode;
mod descriptor;
mod samples;

pub use color::to_rgb;
pub use decode::{PixelFrame, decode_frame};
pub use descriptor::ImageDescriptor;
pub use samples::Samples;
