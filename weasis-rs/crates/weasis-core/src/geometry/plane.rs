//! Anatomical plane of an image, from the direction cosines of Image Orientation (Patient).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/geometry/ImageOrientation.java`
//! (`getPlan`, `hasSameOrientation`).

use glam::DVec3;

use super::PatientAxis;

/// Minimum cosine between the normals of two oblique planes considered to have the same
/// orientation (about 18°).
const SAME_OBLIQUE_ORIENTATION: f64 = 0.95;

/// Plane of an image in the patient coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePlane {
    /// Rows and columns along the right-left and anterior-posterior axes.
    Axial,
    /// Rows and columns along the anterior-posterior and feet-head axes.
    Sagittal,
    /// Rows and columns along the right-left and feet-head axes.
    Coronal,
    /// Any other orientation.
    Oblique,
}

impl ImagePlane {
    /// Plane of the row and column direction cosines.
    pub fn from_orientation(row: DVec3, column: DVec3) -> Self {
        use PatientAxis::{X, Y, Z};
        match (
            PatientAxis::of_direction(row),
            PatientAxis::of_direction(column),
        ) {
            (Some(X), Some(Y)) | (Some(Y), Some(X)) => Self::Axial,
            (Some(X), Some(Z)) | (Some(Z), Some(X)) => Self::Coronal,
            (Some(Y), Some(Z)) | (Some(Z), Some(Y)) => Self::Sagittal,
            _ => Self::Oblique,
        }
    }

    /// Upper-case name, as the Java enum constant.
    pub fn name(self) -> &'static str {
        match self {
            Self::Axial => "AXIAL",
            Self::Sagittal => "SAGITTAL",
            Self::Coronal => "CORONAL",
            Self::Oblique => "OBLIQUE",
        }
    }
}

/// Whether two images have the same orientation: the same standard plane, or for oblique
/// images nearly parallel planes (used to synchronize views).
pub fn same_orientation(first: (DVec3, DVec3), second: (DVec3, DVec3)) -> bool {
    let plane = ImagePlane::from_orientation(first.0, first.1);
    if plane != ImagePlane::Oblique {
        return plane == ImagePlane::from_orientation(second.0, second.1);
    }
    let normal = |(row, column): (DVec3, DVec3)| row.cross(column).normalize_or_zero();
    // NOTE: the Java code sums the component-wise product, i.e. the dot product of the normals.
    normal(first).dot(normal(second)) > SAME_OBLIQUE_ORIENTATION
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(row: [f64; 3], column: [f64; 3]) -> ImagePlane {
        ImagePlane::from_orientation(DVec3::from(row), DVec3::from(column))
    }

    #[test]
    fn standard_orientations_give_their_plane() {
        assert_eq!(plane([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), ImagePlane::Axial);
        assert_eq!(
            plane([0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
            ImagePlane::Sagittal
        );
        assert_eq!(
            plane([1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            ImagePlane::Coronal
        );
    }

    #[test]
    fn a_tilt_within_the_threshold_keeps_the_plane() {
        // Rows tilted by 30° (cos = 0.866) are still axial; by 45° (0.707) they are oblique.
        assert_eq!(plane([0.866, 0.0, 0.5], [0.0, 1.0, 0.0]), ImagePlane::Axial);
        assert_eq!(
            plane([0.707, 0.0, 0.707], [0.0, 1.0, 0.0]),
            ImagePlane::Oblique
        );
    }

    #[test]
    fn oblique_planes_match_when_nearly_parallel() {
        let oblique = (DVec3::new(0.707, 0.0, 0.707), DVec3::Y);
        let close = (DVec3::new(0.766, 0.0, 0.643), DVec3::Y);
        let axial = (DVec3::X, DVec3::Y);
        assert!(same_orientation(oblique, close));
        assert!(!same_orientation(
            oblique,
            (DVec3::new(0.707, 0.0, -0.707), DVec3::Y)
        ));
        assert!(same_orientation(axial, (DVec3::Y, DVec3::X)));
    }
}
