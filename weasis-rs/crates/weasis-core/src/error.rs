//! Errors returned by `weasis-core`.

/// Result type of `weasis-core`.
pub type Result<T> = std::result::Result<T, Error>;

/// An error of `weasis-core`. Messages are meant to be shown to the user as they are.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a DICOM file, or the file is corrupted.
    #[error("not a readable DICOM file: {0}")]
    UnreadableDicom(String),

    /// The pixel data cannot be read or decoded.
    #[error("cannot read the pixel data: {0}")]
    PixelData(String),
}
