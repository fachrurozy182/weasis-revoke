//! Display shutters: the parts of an image outside the shutter are painted with one color
//! (Display Shutter Module PS3.3 §C.7.6.11, Bitmap Display Shutter Module §C.7.6.15).
//!
//! An image or a Presentation State may combine a rectangle, a circle and a polygon; only the
//! pixels inside all of them are shown. A Presentation State may instead use an overlay plane as
//! a bitmap shutter.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/util/DicomObjectUtil.java`
//! (`getShutterShape`, `getShutterColor`) and
//! `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/display/ShutterOp.java`.

use dicom_core::Tag;
use dicom_dictionary_std::tags;
use glam::DVec2;

use super::{OverlayPlane, Rgba, paint_overlay};
use crate::dicom::DicomFile;

/// Geometric shutter of an image; coordinates are 0-based pixels (x = column, y = row).
#[derive(Debug, Clone, PartialEq)]
pub struct Shutter {
    /// Left, top, right and bottom edges (included).
    pub rectangle: Option<[f64; 4]>,
    /// Center and radius.
    pub circle: Option<(DVec2, f64)>,
    /// Vertices.
    pub polygon: Option<Vec<DVec2>>,
    /// Color of the hidden pixels.
    pub color: Rgba,
}

impl Shutter {
    /// The shutter of an image or a Presentation State; `None` when it has no valid shape.
    pub fn from_file(file: &DicomFile) -> Option<Self> {
        let shape = file.text(tags::SHUTTER_SHAPE)?;
        let has = |name: &str| shape.split('\\').any(|value| value.trim() == name);
        // NOTE: "RECTANGLE" is not a defined term; some writers use it for RECTANGULAR.
        let shutter = Self {
            rectangle: (has("RECTANGULAR") || has("RECTANGLE"))
                .then(|| rectangle(file))
                .flatten(),
            circle: has("CIRCULAR").then(|| circle(file)).flatten(),
            polygon: has("POLYGONAL").then(|| polygon(file)).flatten(),
            color: shutter_color(file),
        };
        (shutter.rectangle.is_some() || shutter.circle.is_some() || shutter.polygon.is_some())
            .then_some(shutter)
    }

    /// Whether a pixel is shown (inside every shape).
    pub fn shows(&self, x: f64, y: f64) -> bool {
        let in_rectangle = self.rectangle.is_none_or(|[left, top, right, bottom]| {
            (left..=right).contains(&x) && (top..=bottom).contains(&y)
        });
        let in_circle = self.circle.is_none_or(|(center, radius)| {
            center.distance_squared(DVec2::new(x, y)) <= radius * radius
        });
        let in_polygon = self
            .polygon
            .as_ref()
            .is_none_or(|vertices| polygon_contains(vertices, DVec2::new(x, y)));
        in_rectangle && in_circle && in_polygon
    }

    /// Paints the hidden pixels of an RGBA image (4 bytes per pixel, `width` pixels per row).
    pub fn paint(&self, rgba: &mut [u8], width: usize) {
        let mask: Vec<bool> = (0..rgba.len() / 4)
            .map(|pixel| {
                #[allow(clippy::cast_precision_loss)]
                let (x, y) = ((pixel % width) as f64, (pixel / width) as f64);
                !self.shows(x, y)
            })
            .collect();
        paint_overlay(rgba, &mask, self.color);
    }
}

/// The overlay plane a Presentation State uses as bitmap shutter, with the shutter color.
///
/// NOTE: the Java viewer paints all the overlay planes of the state in the shutter color when
/// Shutter Overlay Group is present; only the plane of that group is used here (PS3.3 §C.7.6.15).
pub fn bitmap_shutter(file: &DicomFile, planes: &[OverlayPlane]) -> Option<(OverlayPlane, Rgba)> {
    let group = file
        .object()
        .element(tags::SHUTTER_OVERLAY_GROUP)
        .ok()?
        .to_int::<u16>()
        .ok()?;
    let plane = planes.iter().find(|p| p.group == group)?.clone();
    Some((plane, shutter_color(file)))
}

/// Shutter Presentation Color CIELab Value, else the gray of Shutter Presentation Value
/// (16-bit P-value), else black.
pub fn shutter_color(file: &DicomFile) -> Rgba {
    let to_u16 = |value: i32| u16::try_from(value.clamp(0, 0xFFFF)).unwrap_or_default();
    let lab = ints(file, tags::SHUTTER_PRESENTATION_COLOR_CIE_LAB_VALUE);
    if let [l, a, b] = lab[..] {
        return Rgba::from_dicom_lab([l, a, b].map(to_u16));
    }
    let p_value = ints(file, tags::SHUTTER_PRESENTATION_VALUE)
        .first()
        .copied()
        .map_or(0, to_u16);
    let gray = f32::from(p_value >> 8) / 255.0;
    Rgba::opaque(gray, gray, gray)
}

fn rectangle(file: &DicomFile) -> Option<[f64; 4]> {
    let edge = |tag: Tag| f64::from(ints(file, tag).first().copied().unwrap_or(0)) - 1.0;
    let [left, top, right, bottom] = [
        tags::SHUTTER_LEFT_VERTICAL_EDGE,
        tags::SHUTTER_UPPER_HORIZONTAL_EDGE,
        tags::SHUTTER_RIGHT_VERTICAL_EDGE,
        tags::SHUTTER_LOWER_HORIZONTAL_EDGE,
    ]
    .map(edge);
    // Like `Rectangle2D.setFrameFromDiagonal`, the corners may come in any order.
    let rectangle = [
        left.min(right),
        top.min(bottom),
        left.max(right),
        top.max(bottom),
    ];
    (rectangle[2] > rectangle[0] && rectangle[3] > rectangle[1]).then_some(rectangle)
}

fn circle(file: &DicomFile) -> Option<(DVec2, f64)> {
    // Center of Circular Shutter is (row, column), 1-based.
    let center = ints(file, tags::CENTER_OF_CIRCULAR_SHUTTER);
    let (&row, &column) = (center.first()?, center.get(1)?);
    let radius = f64::from(*ints(file, tags::RADIUS_OF_CIRCULAR_SHUTTER).first()?);
    (radius > 0.0).then(|| {
        (
            DVec2::new(f64::from(column) - 1.0, f64::from(row) - 1.0),
            radius,
        )
    })
}

fn polygon(file: &DicomFile) -> Option<Vec<DVec2>> {
    // Vertices are (row, column) pairs, 1-based.
    let values = ints(file, tags::VERTICES_OF_THE_POLYGONAL_SHUTTER);
    let vertices: Vec<DVec2> = values
        .chunks_exact(2)
        .map(|p| DVec2::new(f64::from(p[1]) - 1.0, f64::from(p[0]) - 1.0))
        .collect();
    (vertices.len() >= 3 && signed_area(&vertices).abs() > f64::EPSILON).then_some(vertices)
}

fn signed_area(vertices: &[DVec2]) -> f64 {
    let next = vertices.iter().cycle().skip(1);
    vertices
        .iter()
        .zip(next)
        .map(|(a, b)| a.perp_dot(*b))
        .sum::<f64>()
        / 2.0
}

/// Even-odd rule; points on an edge are inside, as the filled polygon of the Java viewer.
fn polygon_contains(vertices: &[DVec2], point: DVec2) -> bool {
    let next = vertices.iter().cycle().skip(1);
    let mut inside = false;
    for (&a, &b) in vertices.iter().zip(next) {
        if on_segment(a, b, point) {
            return true;
        }
        if (a.y > point.y) != (b.y > point.y) {
            let x = a.x + (point.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if point.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn on_segment(a: DVec2, b: DVec2, point: DVec2) -> bool {
    (b - a).perp_dot(point - a).abs() <= 1e-9
        && point.x >= a.x.min(b.x)
        && point.x <= a.x.max(b.x)
        && point.y >= a.y.min(b.y)
        && point.y <= a.y.max(b.y)
}

/// Integer values of an attribute; empty when absent or not numeric.
fn ints(file: &DicomFile, tag: Tag) -> Vec<i32> {
    file.object()
        .element(tag)
        .ok()
        .and_then(|element| element.to_multi_int::<i32>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::read_dicom;
    use crate::image::overlay_planes;

    fn file(path: &str) -> DicomFile {
        let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
        read_dicom(&bytes).expect("readable")
    }

    #[test]
    fn combined_shapes_show_only_their_intersection() {
        let shutter = Shutter::from_file(&file("synthetic/xa-shutter.dcm")).expect("shutter");
        assert_eq!(shutter.rectangle, Some([4.0, 8.0, 59.0, 55.0]));
        assert!(shutter.shows(32.0, 32.0));
        assert!(shutter.shows(40.0, 40.0));
        assert!(!shutter.shows(2.0, 32.0), "left of the rectangle");
        assert!(!shutter.shows(32.0, 6.0), "above the rectangle");
        assert!(!shutter.shows(10.0, 10.0), "outside the circle");
        assert!(!shutter.shows(50.0, 50.0), "outside the diamond");
    }

    #[test]
    fn the_lab_color_is_converted_to_rgb() {
        let shutter = Shutter::from_file(&file("synthetic/xa-shutter.dcm")).expect("shutter");
        let [r, g, b, a] = shutter.color.to_bytes();
        assert!(r >= 254 && g <= 1 && b <= 1 && a == 255, "{r} {g} {b}");

        let mut rgba = vec![9; 64 * 64 * 4];
        shutter.paint(&mut rgba, 64);
        assert_eq!(&rgba[0..3], &[r, g, b], "corner hidden");
        let center = (32 * 64 + 32) * 4;
        assert_eq!(&rgba[center..center + 3], &[9, 9, 9], "center shown");
    }

    #[test]
    fn a_bitmap_shutter_uses_only_its_overlay_group() {
        let pr = file("synthetic/pr-bitmap-shutter.dcm");
        assert_eq!(Shutter::from_file(&pr), None, "no geometric shape");
        let (plane, color) = bitmap_shutter(&pr, &overlay_planes(&pr)).expect("bitmap shutter");
        assert_eq!(plane.group, 0x6000);
        assert_eq!(color.to_bytes(), [128, 128, 128, 255]);
        let mut mask = vec![false; 64 * 64];
        plane.draw(&mut mask, 64, 0);
        assert_eq!(
            mask.iter().filter(|&&hidden| hidden).count(),
            64 * 64 - 32 * 32
        );
    }

    #[test]
    fn polygons_include_their_edges() {
        let square = [
            DVec2::ZERO,
            DVec2::new(4.0, 0.0),
            DVec2::new(4.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        assert!(polygon_contains(&square, DVec2::new(2.0, 2.0)));
        assert!(polygon_contains(&square, DVec2::new(4.0, 2.0)));
        assert!(!polygon_contains(&square, DVec2::new(5.0, 2.0)));
    }
}
