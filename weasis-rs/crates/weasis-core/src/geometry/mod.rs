//! Patient geometry: directions, planes and orientation labels of images.
//!
//! Vectors are `glam::DVec3` in the DICOM patient coordinate system (decision D15).

mod orientation;
mod plane;

use glam::DVec3;

pub use orientation::{
    Direction, OrientationLabels, PatientAxis, direction_codes, labels_from_patient_orientation,
    labels_from_vectors, opposite_codes,
};
pub use plane::{ImagePlane, same_orientation};

/// Row and column direction cosines of an Image Orientation (Patient) value (six numbers
/// separated by `\`). `None` unless there are exactly six numbers.
pub fn parse_orientation(text: &str) -> Option<(DVec3, DVec3)> {
    let values: Vec<f64> = text
        .split('\\')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<_>>()?;
    let [rx, ry, rz, cx, cy, cz] = values.try_into().ok()?;
    Some((DVec3::new(rx, ry, rz), DVec3::new(cx, cy, cz)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_needs_six_numbers() {
        assert_eq!(
            parse_orientation("1\\0\\0\\0\\ 1\\0"),
            Some((DVec3::X, DVec3::Y))
        );
        assert_eq!(parse_orientation("1\\0\\0\\0\\1"), None);
        assert_eq!(parse_orientation("1\\0\\0\\0\\1\\x"), None);
    }
}
