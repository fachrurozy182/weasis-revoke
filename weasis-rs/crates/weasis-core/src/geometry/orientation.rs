//! Patient directions (R/L, A/P, F/H) and the orientation labels drawn on the image sides.
//!
//! DICOM patient coordinates (PS3.3 §C.7.6.2.1.1): +X to the patient's left, +Y posterior,
//! +Z head. Quadrupeds use their own letters (PS3.3 §C.7.6.1.1.1, Anatomical Orientation Type).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/geometry/PatientOrientation.java`,
//! `ImageOrientation.java` (`getOrientation`) and
//! `weasis-dicom/weasis-dicom-viewer2d/src/main/java/org/weasis/dicom/viewer2d/InfoLayer.java`
//! (`buildOrientationFromVectors`, `buildOrientationFromPatientOrientation`).

use glam::{DQuat, DVec3};

/// How far a direction cosine may deviate from a patient axis and still be on that axis.
const OBLIQUITY_THRESHOLD: f64 = 0.8;
/// Components below this are ignored when naming a direction.
const NEGLIGIBLE: f64 = 0.0001;

/// Axis of the patient coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatientAxis {
    /// Right-left.
    X,
    /// Anterior-posterior.
    Y,
    /// Feet-head.
    Z,
}

impl PatientAxis {
    /// Color of the axis in the viewer (blue, red, green), as RGB.
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::X => [0x00, 0xAE, 0xEF],
            Self::Y => [0xED, 0x1C, 0x24],
            Self::Z => [0x0D, 0xB1, 0x4B],
        }
    }

    /// The axis a direction cosine follows, or `None` when it is oblique.
    pub fn of_direction(v: DVec3) -> Option<Self> {
        let a = v.abs();
        if a.x > OBLIQUITY_THRESHOLD && a.x > a.y && a.x > a.z {
            Some(Self::X)
        } else if a.y > OBLIQUITY_THRESHOLD && a.y > a.x && a.y > a.z {
            Some(Self::Y)
        } else if a.z > OBLIQUITY_THRESHOLD && a.z > a.x && a.z > a.y {
            Some(Self::Z)
        } else {
            None
        }
    }
}

/// A direction relative to the patient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Patient's right (-X).
    Right,
    /// Patient's left (+X).
    Left,
    /// Anterior, ventral for quadrupeds (-Y).
    Anterior,
    /// Posterior, dorsal for quadrupeds (+Y).
    Posterior,
    /// Feet, caudal for quadrupeds (-Z).
    Feet,
    /// Head, cranial for quadrupeds (+Z).
    Head,
}

impl Direction {
    const ALL: [Self; 6] = [
        Self::Right,
        Self::Left,
        Self::Anterior,
        Self::Posterior,
        Self::Feet,
        Self::Head,
    ];

    /// Direction of the sign of a component along an axis.
    pub fn along(axis: PatientAxis, component: f64) -> Self {
        let negative = component < 0.0;
        match axis {
            PatientAxis::X if negative => Self::Right,
            PatientAxis::X => Self::Left,
            PatientAxis::Y if negative => Self::Anterior,
            PatientAxis::Y => Self::Posterior,
            PatientAxis::Z if negative => Self::Feet,
            PatientAxis::Z => Self::Head,
        }
    }

    /// Axis of the direction.
    pub fn axis(self) -> PatientAxis {
        match self {
            Self::Right | Self::Left => PatientAxis::X,
            Self::Anterior | Self::Posterior => PatientAxis::Y,
            Self::Feet | Self::Head => PatientAxis::Z,
        }
    }

    /// Opposite direction.
    pub fn opposite(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Anterior => Self::Posterior,
            Self::Posterior => Self::Anterior,
            Self::Feet => Self::Head,
            Self::Head => Self::Feet,
        }
    }

    /// Code used in Patient Orientation (0020,0020): R, L, A, P, F, H, or for quadrupeds
    /// RT, LE, V, D, CD, CR.
    pub fn code(self, quadruped: bool) -> &'static str {
        match (self, quadruped) {
            (Self::Right, false) => "R",
            (Self::Left, false) => "L",
            (Self::Anterior, false) => "A",
            (Self::Posterior, false) => "P",
            (Self::Feet, false) => "F",
            (Self::Head, false) => "H",
            (Self::Right, true) => "RT",
            (Self::Left, true) => "LE",
            (Self::Anterior, true) => "V",
            (Self::Posterior, true) => "D",
            (Self::Feet, true) => "CD",
            (Self::Head, true) => "CR",
        }
    }
}

/// Codes of a direction vector, the main component first (e.g. `"L P H"` for an oblique one);
/// empty for a null vector.
pub fn direction_codes(v: DVec3, quadruped: bool) -> String {
    let axes = [PatientAxis::X, PatientAxis::Y, PatientAxis::Z];
    let components = v.to_array();
    let mut magnitudes = v.abs().to_array();
    let mut codes = Vec::new();
    // On a tie, X comes before Y and Y before Z, as in the Java viewer.
    while let Some(i) = (0..3)
        .find(|&i| magnitudes[i] > NEGLIGIBLE && magnitudes.iter().all(|&m| magnitudes[i] >= m))
    {
        codes.push(Direction::along(axes[i], components[i]).code(quadruped));
        magnitudes[i] = 0.0;
    }
    codes.join(" ")
}

/// Labels at the left and top sides of a displayed image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientationLabels {
    /// Codes at the left side, main direction first, separated by spaces.
    pub left: String,
    /// Codes at the top side.
    pub top: String,
}

/// Labels from the row and column direction cosines (Image Orientation (Patient)), for an image
/// rotated clockwise by `rotation_degrees` and possibly flipped horizontally.
pub fn labels_from_vectors(
    row: DVec3,
    column: DVec3,
    rotation_degrees: f64,
    flip: bool,
    quadruped: bool,
) -> OrientationLabels {
    // The left side shows where the row comes from, the top where the column comes from.
    let rotation = DQuat::from_axis_angle(
        row.cross(column).normalize_or_zero(),
        -rotation_degrees.to_radians(),
    );
    let mut left = rotation * -row;
    let top = rotation * -column;
    if flip {
        left = -left;
    }
    OrientationLabels {
        left: direction_codes(left, quadruped),
        top: direction_codes(top, quadruped),
    }
}

/// Labels from Patient Orientation (0020,0020), the direction of the rows and of the columns
/// (e.g. `["A", "FR"]`). `None` unless there are two values; the attribute cannot follow a
/// rotation, so rotated images get no labels.
pub fn labels_from_patient_orientation(
    values: &[&str],
    rotation_degrees: f64,
    flip: bool,
    quadruped: bool,
) -> Option<OrientationLabels> {
    let [rows, columns] = values else {
        return None;
    };
    if rotation_degrees != 0.0 {
        return None;
    }
    let left = if flip {
        (*rows).to_owned()
    } else {
        opposite_codes(rows, quadruped)
    };
    Some(OrientationLabels {
        left,
        top: opposite_codes(columns, quadruped),
    })
}

/// Opposite of a sequence of codes, e.g. `"FR"` → `"HL"`; unknown codes are dropped.
///
/// NOTE: the Java viewer cuts the value before each upper-case letter, which loses the two-letter
/// quadruped codes (RT, LE, CD, CR); here the known codes are matched, longest first.
pub fn opposite_codes(codes: &str, quadruped: bool) -> String {
    let mut rest = codes.trim();
    let mut opposite = String::new();
    while !rest.is_empty() {
        let found = Direction::ALL
            .iter()
            .map(|d| (d, d.code(quadruped)))
            .filter(|(_, code)| rest.starts_with(code))
            .max_by_key(|(_, code)| code.len());
        match found {
            Some((direction, code)) => {
                opposite.push_str(direction.opposite().code(quadruped));
                rest = &rest[code.len()..];
            }
            None => rest = rest.get(1..).unwrap_or_default(),
        }
    }
    opposite
}

#[cfg(test)]
mod tests {
    use super::*;

    const AXIAL_ROW: DVec3 = DVec3::X;
    const AXIAL_COLUMN: DVec3 = DVec3::Y;

    #[test]
    fn an_axial_image_shows_right_and_anterior() {
        let labels = labels_from_vectors(AXIAL_ROW, AXIAL_COLUMN, 0.0, false, false);
        assert_eq!((labels.left.as_str(), labels.top.as_str()), ("R", "A"));
    }

    #[test]
    fn rotation_and_flip_move_the_labels() {
        let rotated = labels_from_vectors(AXIAL_ROW, AXIAL_COLUMN, 90.0, false, false);
        assert_eq!((rotated.left.as_str(), rotated.top.as_str()), ("P", "R"));
        let flipped = labels_from_vectors(AXIAL_ROW, AXIAL_COLUMN, 0.0, true, false);
        assert_eq!(flipped.left, "L");
    }

    #[test]
    fn oblique_directions_list_the_main_component_first() {
        let v = DVec3::new(0.3, -0.9, 0.2);
        assert_eq!(direction_codes(v, false), "A L H");
        assert_eq!(direction_codes(v, true), "V LE CR");
        assert_eq!(direction_codes(DVec3::ZERO, false), "");
    }

    #[test]
    fn patient_orientation_gives_the_opposite_sides() {
        let labels = labels_from_patient_orientation(&["A", "FR"], 0.0, false, false);
        assert_eq!(
            labels,
            Some(OrientationLabels {
                left: "P".to_owned(),
                top: "HL".to_owned()
            })
        );
        assert_eq!(opposite_codes("LECD", true), "RTCR");
        assert_eq!(
            labels_from_patient_orientation(&["A"], 0.0, false, false),
            None
        );
        assert_eq!(
            labels_from_patient_orientation(&["A", "F"], 90.0, false, false),
            None
        );
    }
}
