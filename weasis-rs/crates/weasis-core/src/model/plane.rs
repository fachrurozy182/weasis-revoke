//! Anatomical plane of an image, from the direction cosines of Image Orientation (Patient).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/geometry/ImageOrientation.java`
//! (`getPlan`).

/// How far a direction cosine may deviate from a patient axis and still be on that axis.
const OBLIQUITY_THRESHOLD: f64 = 0.8;

/// Plane of an image in the patient coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImagePlane {
    /// Rows and columns along the right-left and anterior-posterior axes.
    Axial,
    /// Rows and columns along the anterior-posterior and head-feet axes.
    Sagittal,
    /// Rows and columns along the right-left and head-feet axes.
    Coronal,
    /// Any other orientation.
    Oblique,
}

impl ImagePlane {
    /// Plane of the row and column direction cosines (the six values of Image Orientation
    /// (Patient), PS3.3 §C.7.6.2.1.1).
    pub fn from_orientation(orientation: &[f64; 6]) -> Self {
        let row = main_axis(&orientation[0..3]);
        let column = main_axis(&orientation[3..6]);
        match (row, column) {
            (Some(Axis::X), Some(Axis::Y)) | (Some(Axis::Y), Some(Axis::X)) => Self::Axial,
            (Some(Axis::X), Some(Axis::Z)) | (Some(Axis::Z), Some(Axis::X)) => Self::Coronal,
            (Some(Axis::Y), Some(Axis::Z)) | (Some(Axis::Z), Some(Axis::Y)) => Self::Sagittal,
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

/// Patient axes: X right to left, Y anterior to posterior, Z feet to head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}

/// The axis a direction cosine follows, or `None` when it is oblique.
fn main_axis(vector: &[f64]) -> Option<Axis> {
    let [x, y, z] = [vector[0].abs(), vector[1].abs(), vector[2].abs()];
    if x > OBLIQUITY_THRESHOLD && x > y && x > z {
        Some(Axis::X)
    } else if y > OBLIQUITY_THRESHOLD && y > x && y > z {
        Some(Axis::Y)
    } else if z > OBLIQUITY_THRESHOLD && z > x && z > y {
        Some(Axis::Z)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_orientations_give_their_plane() {
        let plane = |o: [f64; 6]| ImagePlane::from_orientation(&o);
        assert_eq!(plane([1.0, 0.0, 0.0, 0.0, 1.0, 0.0]), ImagePlane::Axial);
        assert_eq!(plane([0.0, 1.0, 0.0, 0.0, 0.0, -1.0]), ImagePlane::Sagittal);
        assert_eq!(plane([1.0, 0.0, 0.0, 0.0, 0.0, -1.0]), ImagePlane::Coronal);
    }

    #[test]
    fn a_tilt_within_the_threshold_keeps_the_plane() {
        // Rows tilted by 30° (cos = 0.866) are still axial; by 45° (0.707) they are oblique.
        let tilted = |c: f64, s: f64| ImagePlane::from_orientation(&[c, 0.0, s, 0.0, 1.0, 0.0]);
        assert_eq!(tilted(0.866, 0.5), ImagePlane::Axial);
        assert_eq!(tilted(0.707, 0.707), ImagePlane::Oblique);
    }
}
