//! Cross-lines: where an image of another series cuts the plane of the displayed image.
//!
//! The source image is moved into the space of the displayed image (the "localizer"): its
//! origin at the localizer top left pixel and its axes along the localizer rows, columns and
//! normal. Points with z = 0 are then in the localizer plane.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/geometry/LocalizerPoster.java`,
//! `IntersectSlice.java`, `IntersectVolume.java` and
//! `weasis-dicom/weasis-dicom-viewer2d/src/main/java/org/weasis/dicom/viewer2d/View2d.java` (`getCrossLine`).

use glam::{DMat3, DVec2, DVec3};

use super::SliceGeometry;

/// Tolerance on the length of a direction cosine (squared length within 1 ± 0.001).
const UNIT_TOLERANCE: f64 = 0.001;
/// Maximum cosine between row and column.
const ORTHOGONALITY_TOLERANCE: f64 = 0.005;
/// Edges whose ends differ less in z are parallel to the plane (`MathUtil.DOUBLE_EPSILON`).
const PARALLEL_TOLERANCE: f64 = 1e-9;

/// The 12 edges of a box, as corner indices: top face, bottom face, then the sides.
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// The displayed image on which cross-lines are drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Localizer {
    geometry: SliceGeometry,
    into_localizer: DMat3,
}

impl Localizer {
    /// `None` when the row and column of the image are not orthogonal unit vectors.
    pub fn new(geometry: &SliceGeometry) -> Option<Self> {
        if !valid_direction_cosines(geometry) {
            return None;
        }
        let normal = geometry.row.cross(geometry.column);
        Some(Self {
            geometry: *geometry,
            into_localizer: DMat3::from_cols(geometry.row, geometry.column, normal).transpose(),
        })
    }

    /// Line (or rectangle when coplanar) where the plane of `source` cuts the localizer, in
    /// localizer pixels. `None` when they do not meet (`IntersectSlice`).
    pub fn slice_line(&self, source: &SliceGeometry) -> Option<Vec<DVec2>> {
        let corners = self.in_localizer_space(&rectangle_corners(source)?);
        let crossing: Vec<bool> = (0..4)
            .map(|i| crosses_plane(corners[i], corners[(i + 1) % 4]))
            .collect();
        let points = if crossing.iter().all(|&c| c) {
            corners.to_vec()
        } else if (0..4).any(|i| crossing[i] && (crossing[(i + 1) % 4] || crossing[(i + 2) % 4])) {
            // Two opposite or adjacent edges cross the plane: join the crossing points.
            (0..4)
                .filter(|&i| crossing[i])
                .map(|i| plane_crossing(corners[i], corners[(i + 1) % 4]))
                .collect()
        } else {
            return None;
        };
        Some(points.into_iter().map(|p| self.pixel_of(p)).collect())
    }

    /// Outline of the slab of `source` (its plane ± half the slice thickness) in the
    /// localizer plane, as a convex polygon in localizer pixels (`IntersectVolume`).
    pub fn slab_outline(&self, source: &SliceGeometry) -> Option<Vec<DVec2>> {
        let corners = self.in_localizer_space(&box_corners(source)?);
        let mut points: Vec<DVec3> = BOX_EDGES
            .iter()
            .filter(|&&(a, b)| crosses_plane(corners[a], corners[b]))
            .map(|&(a, b)| plane_crossing(corners[a], corners[b]))
            .collect();
        if points.is_empty() {
            return None;
        }
        // Order the points by angle around their centroid to get a convex polygon.
        let center = points.iter().sum::<DVec3>() / points.len() as f64;
        let angle = |p: &DVec3| (p.y - center.y).atan2(p.x - center.x);
        points.sort_by(|a, b| angle(a).total_cmp(&angle(b)));
        Some(points.into_iter().map(|p| self.pixel_of(p)).collect())
    }

    fn in_localizer_space<const N: usize>(&self, corners: &[DVec3; N]) -> [DVec3; N] {
        corners.map(|c| self.into_localizer * (c - self.geometry.top_left))
    }

    /// Localizer pixel of a point of the localizer plane.
    ///
    /// The scale factors (n - 1) / n and the half-pixel shift come from the original
    /// PixelMed code and are kept so that the lines match the Java viewer.
    fn pixel_of(&self, point: DVec3) -> DVec2 {
        let [rows, columns, _] = self.geometry.dimensions.to_array();
        let spacing = self.geometry.spacing;
        DVec2::new(
            point.x / spacing.y * ((rows - 1.0) / rows) + 0.5,
            point.y / spacing.x * ((columns - 1.0) / columns) + 0.5,
        )
    }
}

/// Points to draw for an image of another series on the localizer, as `View2d.getCrossLine`:
/// two points for a line, more for a polygon. `slab` selects the slice thickness outline.
pub fn cross_line(localizer: &Localizer, source: &SliceGeometry, slab: bool) -> Option<Vec<DVec2>> {
    if !source.is_row_column_orthogonal() {
        return None;
    }
    let mut points = if slab {
        localizer.slab_outline(source)?
    } else {
        localizer.slice_line(source)?
    };
    // A line crossing the plane at a corner gives the same point twice.
    if points.len() > 1 && points[points.len() - 1] == points[points.len() - 2] {
        points.remove(points.len() - 2);
    }
    Some(points)
}

fn valid_direction_cosines(geometry: &SliceGeometry) -> bool {
    (geometry.row.length_squared() - 1.0).abs() <= UNIT_TOLERANCE
        && (geometry.column.length_squared() - 1.0).abs() <= UNIT_TOLERANCE
        && geometry.row.dot(geometry.column) <= ORTHOGONALITY_TOLERANCE
}

/// Corners of the image: top left, top right, bottom right, bottom left.
fn rectangle_corners(source: &SliceGeometry) -> Option<[DVec3; 4]> {
    if !valid_direction_cosines(source) {
        return None;
    }
    let along_row = source.row * (source.dimensions.y * source.spacing.y);
    let along_column = source.column * (source.dimensions.x * source.spacing.x);
    let tl = source.top_left;
    Some([
        tl,
        tl + along_row,
        tl + along_row + along_column,
        tl + along_column,
    ])
}

/// Corners of the slab: the image rectangle moved by ± half the thickness along the normal.
fn box_corners(source: &SliceGeometry) -> Option<[DVec3; 8]> {
    let [a, b, c, d] = rectangle_corners(source)?;
    let half =
        source.row.cross(source.column) * (source.dimensions.z / 2.0 * source.slice_thickness);
    Some([
        a + half,
        b + half,
        c + half,
        d + half,
        a - half,
        b - half,
        c - half,
        d - half,
    ])
}

/// Whether an edge touches or crosses the plane z = 0.
fn crosses_plane(a: DVec3, b: DVec3) -> bool {
    (a.z <= 0.0 && b.z >= 0.0) || (a.z >= 0.0 && b.z <= 0.0)
}

/// Point of the line through `a` and `b` where z = 0; `a` when the edge lies in the plane.
///
/// NOTE: the Java code derives y from z, then x from y, which keeps x = a.x for an edge whose
/// y does not change even when x does; the interpolation here is exact in that case too.
fn plane_crossing(a: DVec3, b: DVec3) -> DVec3 {
    if (b.z - a.z).abs() <= PARALLEL_TOLERANCE {
        return a.with_z(0.0);
    }
    let point = a.lerp(b, -a.z / (b.z - a.z));
    point.with_z(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Coronal scout, 512 × 512 pixels of 0.5 mm, from (-128, 0, 128) towards the feet.
    fn scout() -> Localizer {
        let geometry = SliceGeometry::for_display(
            (DVec3::X, DVec3::NEG_Z),
            DVec3::new(-128.0, 0.0, 128.0),
            (512, 512),
            DVec2::splat(0.5),
            None,
        );
        Localizer::new(&geometry).expect("valid scout")
    }

    /// Axial image of 200 mm centered on the Z axis at height `z`, 5 mm thick.
    fn axial(z: f64) -> SliceGeometry {
        SliceGeometry::for_display(
            (DVec3::X, DVec3::Y),
            DVec3::new(-100.0, -100.0, z),
            (400, 400),
            DVec2::splat(0.5),
            Some(5.0),
        )
    }

    fn assert_near(actual: &[DVec2], expected: &[[f64; 2]]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert!(a.abs_diff_eq(DVec2::from(*e), 1e-9), "{a} != {e:?}");
        }
    }

    #[test]
    fn an_axial_slice_is_a_horizontal_line_on_a_coronal_scout() {
        let line = cross_line(&scout(), &axial(0.0), false).expect("line");
        // Row 256 of the scout (z = 0), from x = 100 to x = -100 mm.
        assert_near(&line, &[[455.609375, 256.0], [56.390625, 256.0]]);
    }

    #[test]
    fn the_slab_shows_the_slice_thickness() {
        let outline = cross_line(&scout(), &axial(0.0), true).expect("outline");
        let top = 251.009_765_625;
        let bottom = 130.5 / 0.5 * (511.0 / 512.0) + 0.5;
        assert_near(
            &outline,
            &[
                [56.390625, top],
                [455.609375, top],
                [455.609375, bottom],
                [56.390625, bottom],
            ],
        );
    }

    #[test]
    fn parallel_planes_do_not_cross() {
        let localizer = Localizer::new(&axial(0.0)).expect("valid");
        assert_eq!(localizer.slice_line(&axial(10.0)), None);
        assert_eq!(
            localizer
                .slice_line(&axial(0.0))
                .map(|corners| corners.len()),
            Some(4)
        );
    }

    #[test]
    fn invalid_direction_cosines_are_rejected() {
        let mut geometry = axial(0.0);
        geometry.row = DVec3::new(2.0, 0.0, 0.0);
        assert_eq!(Localizer::new(&geometry), None);
        assert_eq!(scout().slice_line(&geometry), None);
    }
}
