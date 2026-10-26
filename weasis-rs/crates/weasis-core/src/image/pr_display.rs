//! Where and how a Presentation State shows an image: Displayed Area (PS3.3 §C.10.4) and
//! Spatial Transformation (PS3.3 §C.10.6), and the mapping of DISPLAY annotation units back to
//! image pixels.
//!
//! Image coordinates are continuous pixels with the origin at the top left corner of the top
//! left pixel (the PIXEL annotation units of PS3.3 §C.10.5.1.1).
//!
//! Java reference: `weasis-dicom/weasis-dicom-viewer2d/src/main/java/org/weasis/dicom/viewer2d/PRManager.java`
//! (`DisplayAreaProcessor`, `ZoomProcessor`, `PixelSpacingProcessor`, `createInverseTransform`)
//! and `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/PresentationStateReader.java`.

use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;
use glam::DVec2;

use super::ImageReference;
use crate::dicom::{item_f64, item_text};

/// How the displayed area is sized on screen (Presentation Size Mode).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeMode {
    /// Fit the view.
    ScaleToFit,
    /// Physical size on the screen (needs a calibrated screen).
    TrueSize,
    /// Magnified by a ratio of image pixels to screen pixels.
    Magnify(f64),
}

/// One item of the Displayed Area Selection Sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayedArea {
    /// Images it applies to; empty for all.
    pub images: Vec<ImageReference>,
    /// Displayed Area Top Left Hand Corner, 1-based pixel (column, row); may lie outside.
    pub top_left: (i32, i32),
    /// Displayed Area Bottom Right Hand Corner, 1-based pixel (column, row).
    pub bottom_right: (i32, i32),
    /// Presentation Size Mode.
    pub size_mode: SizeMode,
    /// Presentation Pixel Spacing (row spacing, column spacing) in mm.
    pub pixel_spacing: Option<(f64, f64)>,
    /// Presentation Pixel Aspect Ratio (vertical, horizontal), when there is no spacing.
    pub aspect_ratio: Option<(i32, i32)>,
}

impl DisplayedArea {
    /// The items of a Presentation State.
    pub fn list(object: &InMemDicomObject) -> Vec<Self> {
        object
            .element(tags::DISPLAYED_AREA_SELECTION_SEQUENCE)
            .ok()
            .and_then(|element| element.items())
            .unwrap_or_default()
            .iter()
            .filter_map(Self::read)
            .collect()
    }

    fn read(item: &InMemDicomObject) -> Option<Self> {
        let pair = |tag| {
            let values = ints(item, tag);
            Some((*values.first()?, *values.get(1)?))
        };
        let spacing = item
            .element(tags::PRESENTATION_PIXEL_SPACING)
            .ok()
            .and_then(|e| e.to_multi_float64().ok())
            .filter(|v| v.len() == 2 && v[0] > 0.0 && v[1] > 0.0)
            .map(|v| (v[0], v[1]));
        let size_mode = match item_text(item, tags::PRESENTATION_SIZE_MODE)
            .unwrap_or_default()
            .to_uppercase()
            .as_str()
        {
            "TRUE SIZE" => SizeMode::TrueSize,
            "MAGNIFY" => SizeMode::Magnify(
                item_f64(item, tags::PRESENTATION_PIXEL_MAGNIFICATION_RATIO).unwrap_or(1.0),
            ),
            _ => SizeMode::ScaleToFit,
        };
        Some(Self {
            images: ImageReference::list(item),
            top_left: pair(tags::DISPLAYED_AREA_TOP_LEFT_HAND_CORNER)?,
            bottom_right: pair(tags::DISPLAYED_AREA_BOTTOM_RIGHT_HAND_CORNER)?,
            size_mode,
            pixel_spacing: spacing,
            aspect_ratio: pair(tags::PRESENTATION_PIXEL_ASPECT_RATIO).filter(|_| spacing.is_none()),
        })
    }

    /// The area in image coordinates: origin and size, in pixels.
    pub fn bounds(&self) -> (DVec2, DVec2) {
        let top_left = DVec2::new(f64::from(self.top_left.0), f64::from(self.top_left.1));
        let bottom_right = DVec2::new(
            f64::from(self.bottom_right.0),
            f64::from(self.bottom_right.1),
        );
        let (min, max) = (top_left.min(bottom_right), top_left.max(bottom_right));
        // From the corner of the first pixel to the far corner of the last one.
        (min - DVec2::ONE, max - min + DVec2::ONE)
    }
}

/// Image Rotation (clockwise degrees: 0, 90, 180 or 270) and Image Horizontal Flip. The flip
/// is applied first (PS3.3 §C.10.6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpatialTransform {
    /// Clockwise rotation in degrees.
    pub rotation: i32,
    /// Horizontal flip.
    pub flip: bool,
}

impl SpatialTransform {
    /// Reads the module of a Presentation State.
    pub fn read(object: &InMemDicomObject) -> Self {
        Self {
            rotation: ints(object, tags::IMAGE_ROTATION)
                .first()
                .copied()
                .unwrap_or(0),
            flip: item_text(object, tags::IMAGE_HORIZONTAL_FLIP)
                .is_some_and(|v| v.eq_ignore_ascii_case("Y")),
        }
    }
}

/// Maps annotation points to image coordinates for one displayed area and transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotationSpace {
    /// Origin of the displayed area in image coordinates.
    pub origin: DVec2,
    /// Size of the displayed area in pixels.
    pub size: DVec2,
    /// Rotation and flip of the display.
    pub transform: SpatialTransform,
}

impl AnnotationSpace {
    /// Space of a whole image of `columns` × `rows` pixels, or of a displayed area.
    pub fn new(
        columns: u32,
        rows: u32,
        area: Option<&DisplayedArea>,
        transform: SpatialTransform,
    ) -> Self {
        let (origin, size) = area.map_or_else(
            || (DVec2::ZERO, DVec2::new(f64::from(columns), f64::from(rows))),
            DisplayedArea::bounds,
        );
        Self {
            origin,
            size,
            transform,
        }
    }

    /// Image coordinates of a point given in PIXEL units (unchanged) or DISPLAY units.
    ///
    /// DISPLAY units are fractions of the displayed area as shown, after flip and rotation
    /// (PS3.3 §C.10.5.1.1): the rotation and the flip are undone in the unit square, then the
    /// point is scaled to the area.
    ///
    /// NOTE: the Java viewer scales by the area before undoing the rotation around (0.5, 0.5),
    /// which misplaces DISPLAY annotations of rotated or flipped states; the order here follows
    /// the standard.
    pub fn to_image(&self, point: DVec2, display_units: bool) -> DVec2 {
        if !display_units {
            return point;
        }
        let center = DVec2::splat(0.5);
        let angle = -f64::from(self.transform.rotation).to_radians();
        let mut unit = center + DVec2::from_angle(angle).rotate(point - center);
        if self.transform.flip {
            unit.x = 1.0 - unit.x;
        }
        self.origin + unit * self.size
    }
}

fn ints(item: &InMemDicomObject, tag: dicom_object::Tag) -> Vec<i32> {
    item.element(tag)
        .ok()
        .and_then(|e| e.to_multi_int::<i32>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space(rotation: i32, flip: bool) -> AnnotationSpace {
        AnnotationSpace::new(200, 100, None, SpatialTransform { rotation, flip })
    }

    fn near(a: DVec2, b: DVec2) -> bool {
        a.abs_diff_eq(b, 1e-9)
    }

    #[test]
    fn display_units_are_fractions_of_the_area() {
        let space = space(0, false);
        assert_eq!(
            space.to_image(DVec2::new(0.5, 0.25), true),
            DVec2::new(100.0, 25.0)
        );
        assert_eq!(
            space.to_image(DVec2::new(3.0, 4.0), false),
            DVec2::new(3.0, 4.0)
        );
    }

    #[test]
    fn rotation_and_flip_are_undone() {
        // Rotated 90° clockwise, the displayed top left corner is the image bottom left one.
        assert!(near(
            space(90, false).to_image(DVec2::ZERO, true),
            DVec2::new(0.0, 100.0)
        ));
        // Flipped, the displayed top left corner is the image top right one.
        assert!(near(
            space(0, true).to_image(DVec2::ZERO, true),
            DVec2::new(200.0, 0.0)
        ));
        // Flip first, then rotation: undone in reverse order.
        assert!(near(
            space(90, true).to_image(DVec2::ZERO, true),
            DVec2::new(200.0, 100.0)
        ));
    }

    #[test]
    fn a_displayed_area_covers_its_corner_pixels() {
        let area = DisplayedArea {
            images: Vec::new(),
            top_left: (11, 21),
            bottom_right: (60, 40),
            size_mode: SizeMode::ScaleToFit,
            pixel_spacing: None,
            aspect_ratio: None,
        };
        assert_eq!(
            area.bounds(),
            (DVec2::new(10.0, 20.0), DVec2::new(50.0, 20.0))
        );
        let space = AnnotationSpace::new(200, 100, Some(&area), SpatialTransform::default());
        assert_eq!(space.to_image(DVec2::ONE, true), DVec2::new(60.0, 40.0));
    }
}
