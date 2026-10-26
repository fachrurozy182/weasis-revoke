//! Position of an image plane in patient space.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/geometry/GeometryOfSlice.java`
//! and `DicomImageElement.getSliceGeometry`.

use glam::{DVec2, DVec3};

/// Spacing below which a pixel cannot be mapped back from patient space.
const MIN_SPACING: f64 = 0.000_01;
/// Maximum |cos| between row and column for them to be orthogonal.
const ORTHOGONALITY_TOLERANCE: f64 = 0.005;

/// Geometry of one image (Image Plane Module, PS3.3 §C.7.6.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliceGeometry {
    /// Direction of the rows (first three values of Image Orientation (Patient)).
    pub row: DVec3,
    /// Direction of the columns.
    pub column: DVec3,
    /// Center of the top left pixel (Image Position (Patient)), in mm.
    pub top_left: DVec3,
    /// Spacing between rows, between columns, and between slices, in mm.
    pub spacing: DVec3,
    /// Slice thickness in mm.
    pub slice_thickness: f64,
    /// Number of rows, of columns and of slices.
    pub dimensions: DVec3,
}

impl SliceGeometry {
    /// Geometry of an image as displayed: non-square pixels are stretched so that pixels become
    /// square with the smaller of the two sizes, which changes the number of rows or columns.
    ///
    /// `pixel_size` is the (horizontal, vertical) size of a pixel in mm. A missing slice thickness
    /// is replaced by the pixel size, as in the Java viewer.
    pub fn for_display(
        (row, column): (DVec3, DVec3),
        top_left: DVec3,
        (rows, columns): (u32, u32),
        pixel_size: DVec2,
        slice_thickness: Option<f64>,
    ) -> Self {
        let size = pixel_size.x.min(pixel_size.y);
        let stretch = pixel_size / size;
        Self {
            row,
            column,
            top_left,
            spacing: DVec3::new(size, size, slice_thickness.unwrap_or(size)),
            slice_thickness: slice_thickness.unwrap_or(size),
            dimensions: DVec3::new(
                f64::from(rows) * stretch.y,
                f64::from(columns) * stretch.x,
                1.0,
            ),
        }
    }

    /// Unit normal of the plane (row × column).
    pub fn normal(&self) -> DVec3 {
        self.row.cross(self.column).normalize_or_zero()
    }

    /// Whether the rows and the columns are perpendicular.
    pub fn is_row_column_orthogonal(&self) -> bool {
        self.row.dot(self.column).abs() <= ORTHOGONALITY_TOLERANCE
    }

    /// Patient position of an image point (x along the rows, y along the columns, in pixels).
    ///
    /// NOTE: the Java code scales x by the spacing between rows and y by the spacing between
    /// columns; this only differs for non-square pixels, which the display geometry never has.
    pub fn to_patient(&self, point: DVec2) -> DVec3 {
        self.top_left
            + self.row * (self.spacing.y * point.x)
            + self.column * (self.spacing.x * point.y)
    }

    /// Image point of a patient position, projected on the plane.
    pub fn to_image(&self, position: DVec3) -> Option<DVec2> {
        if self.spacing.x < MIN_SPACING || self.spacing.y < MIN_SPACING {
            return None;
        }
        let offset = position - self.top_left;
        Some(DVec2::new(
            offset.dot(self.row) / self.spacing.y,
            offset.dot(self.column) / self.spacing.x,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axial(pixel_size: DVec2) -> SliceGeometry {
        SliceGeometry::for_display(
            (DVec3::X, DVec3::Y),
            DVec3::new(-100.0, -100.0, 20.0),
            (256, 128),
            pixel_size,
            None,
        )
    }

    #[test]
    fn image_and_patient_points_map_both_ways() {
        let geometry = axial(DVec2::splat(0.5));
        let position = geometry.to_patient(DVec2::new(10.0, 4.0));
        assert_eq!(position, DVec3::new(-95.0, -98.0, 20.0));
        assert_eq!(geometry.to_image(position), Some(DVec2::new(10.0, 4.0)));
        assert_eq!(geometry.normal(), DVec3::Z);
        assert_eq!(geometry.slice_thickness, 0.5);
    }

    #[test]
    fn non_square_pixels_are_stretched_to_the_smaller_size() {
        // Pixels twice as wide as high: twice as many columns once displayed.
        let geometry = axial(DVec2::new(1.0, 0.5));
        assert_eq!(geometry.dimensions, DVec3::new(256.0, 256.0, 1.0));
        assert_eq!(geometry.spacing.truncate(), DVec2::splat(0.5));
    }
}
