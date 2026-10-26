//! Graphic annotations of a Presentation State: graphic layers, graphic objects and text objects
//! (Graphic Annotation Module PS3.3 §C.10.5, Graphic Layer Module §C.10.7).
//!
//! Points are returned in image coordinates (see [`AnnotationSpace`]); drawing them is the job
//! of the viewer, which also builds the curves of INTERPOLATED graphics.
//!
//! Java reference: `weasis-dicom/weasis-dicom-viewer2d/src/main/java/org/weasis/dicom/viewer2d/PRManager.java`
//! (`GraphicAnnotationReader`, `TextObjectProcessor`) and
//! `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/pr/PrGraphicUtil.java`.
//
// TODO(4.9): read the Weasis graphic model stored in the private tag (7107,7001) of the PR
// files Weasis writes, which then replaces these annotations.

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};
use glam::DVec2;

use super::{AnnotationSpace, ImageReference, Rgba};
use crate::dicom::{first_item, item_f64, item_text};

/// A graphic layer: the order and the recommended color of its annotations.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicLayer {
    /// Graphic Layer name.
    pub name: String,
    /// Graphic Layer Order: higher layers are drawn over lower ones.
    pub order: i32,
    /// Recommended display color (CIELab, else RGB, else grayscale); `None` for the viewer's
    /// default annotation color.
    pub color: Option<Rgba>,
    /// Graphic Layer Description.
    pub description: Option<String>,
}

impl GraphicLayer {
    /// The layers of a Presentation State.
    pub fn list(object: &InMemDicomObject) -> Vec<Self> {
        items(object, tags::GRAPHIC_LAYER_SEQUENCE)
            .iter()
            .filter_map(|item| {
                Some(Self {
                    name: item_text(item, tags::GRAPHIC_LAYER)?,
                    order: ints(item, tags::GRAPHIC_LAYER_ORDER)
                        .first()
                        .copied()
                        .unwrap_or(0),
                    color: layer_color(item),
                    description: item_text(item, tags::GRAPHIC_LAYER_DESCRIPTION),
                })
            })
            .collect()
    }
}

/// Shape of a graphic object, in image coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphicShape {
    /// A single point.
    Point(DVec2),
    /// Connected straight lines; closed when the last point equals the first.
    Polyline(Vec<DVec2>),
    /// A smooth curve through the points.
    Interpolated(Vec<DVec2>),
    /// Center and radius.
    Circle {
        /// Center.
        center: DVec2,
        /// Radius.
        radius: f64,
    },
    /// Center, half axes and rotation of the major axis (radians, clockwise on screen).
    Ellipse {
        /// Center.
        center: DVec2,
        /// Half of the major and of the minor axis.
        radii: DVec2,
        /// Angle of the major axis.
        rotation: f64,
    },
}

/// Line style of a graphic or text object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    /// Color; the layer color when the object has none.
    pub color: Option<Rgba>,
    /// Line Thickness in screen pixels.
    pub thickness: f64,
    /// Dashed rather than solid.
    pub dashed: bool,
}

/// One item of the Graphic Object Sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicObject {
    /// Shape.
    pub shape: GraphicShape,
    /// Graphic Filled.
    pub filled: bool,
    /// Style.
    pub style: LineStyle,
    /// Graphic Group ID, grouping objects that belong together.
    pub group: Option<i32>,
}

/// One item of the Text Object Sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct TextObject {
    /// Lines of the Unformatted Text Value.
    pub lines: Vec<String>,
    /// Point the text refers to, when its anchor is visible.
    pub anchor: Option<DVec2>,
    /// Bounding box (top left, bottom right) of the text.
    pub bounding_box: Option<(DVec2, DVec2)>,
    /// Style (color and thickness of the anchor line).
    pub style: LineStyle,
}

/// The graphics and texts of one Graphic Annotation Sequence item.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    /// Graphic Layer the annotation belongs to.
    pub layer: String,
    /// Images it applies to; empty for all.
    pub images: Vec<ImageReference>,
    /// Graphic objects.
    pub graphics: Vec<GraphicObject>,
    /// Text objects.
    pub texts: Vec<TextObject>,
}

impl Annotation {
    /// The annotations of a Presentation State whose layer is declared (others are ignored, as
    /// in the Java viewer), with points mapped by `space`.
    pub fn list(object: &InMemDicomObject, space: &AnnotationSpace) -> Vec<Self> {
        let layers: Vec<String> = GraphicLayer::list(object)
            .into_iter()
            .map(|l| l.name)
            .collect();
        items(object, tags::GRAPHIC_ANNOTATION_SEQUENCE)
            .iter()
            .filter_map(|item| {
                let layer = item_text(item, tags::GRAPHIC_LAYER).filter(|l| layers.contains(l))?;
                Some(Self {
                    layer,
                    images: ImageReference::list(item),
                    graphics: items(item, tags::GRAPHIC_OBJECT_SEQUENCE)
                        .iter()
                        .filter_map(|graphic| graphic_object(graphic, space))
                        .collect(),
                    texts: items(item, tags::TEXT_OBJECT_SEQUENCE)
                        .iter()
                        .map(|text| text_object(text, space))
                        .collect(),
                })
            })
            .collect()
    }
}

fn graphic_object(item: &InMemDicomObject, space: &AnnotationSpace) -> Option<GraphicObject> {
    let display = is_display(item, tags::GRAPHIC_ANNOTATION_UNITS);
    let points: Vec<DVec2> = floats(item, tags::GRAPHIC_DATA)
        .chunks_exact(2)
        .map(|p| space.to_image(DVec2::new(p[0], p[1]), display))
        .collect();
    let kind = item_text(item, tags::GRAPHIC_TYPE)?.to_uppercase();
    let shape = match (kind.as_str(), points.as_slice()) {
        ("POINT", [point, ..]) => GraphicShape::Point(*point),
        ("POLYLINE", [_, _, ..]) => GraphicShape::Polyline(points),
        ("INTERPOLATED", [_, _, ..]) => GraphicShape::Interpolated(points),
        ("CIRCLE", [center, edge, ..]) => GraphicShape::Circle {
            center: *center,
            radius: center.distance(*edge),
        },
        ("ELLIPSE", [major_a, major_b, minor_a, minor_b, ..]) => GraphicShape::Ellipse {
            center: (*major_a + *major_b) / 2.0,
            radii: DVec2::new(major_a.distance(*major_b), minor_a.distance(*minor_b)) / 2.0,
            rotation: (*major_b - *major_a).to_angle(),
        },
        _ => return None,
    };
    let style = line_style(item);
    // A Fill Style color, when present, replaces the line color (as in the Java viewer).
    let fill_color = first_item(item, tags::FILL_STYLE_SEQUENCE).and_then(pattern_color);
    Some(GraphicObject {
        shape,
        filled: is_yes(item, tags::GRAPHIC_FILLED),
        style: LineStyle {
            color: fill_color.or(style.color),
            ..style
        },
        group: ints(item, tags::GRAPHIC_GROUP_ID).first().copied(),
    })
}

fn text_object(item: &InMemDicomObject, space: &AnnotationSpace) -> TextObject {
    let point = |tag: Tag, units: Tag| {
        let values = floats(item, tag);
        let [x, y] = values[..] else {
            return None;
        };
        Some(space.to_image(DVec2::new(x, y), is_display(item, units)))
    };
    let units = tags::BOUNDING_BOX_ANNOTATION_UNITS;
    let corners = point(tags::BOUNDING_BOX_TOP_LEFT_HAND_CORNER, units)
        .zip(point(tags::BOUNDING_BOX_BOTTOM_RIGHT_HAND_CORNER, units));
    TextObject {
        lines: item_text(item, tags::UNFORMATTED_TEXT_VALUE)
            .map(|text| text.lines().map(str::to_owned).collect())
            .unwrap_or_default(),
        anchor: point(tags::ANCHOR_POINT, tags::ANCHOR_POINT_ANNOTATION_UNITS)
            .filter(|_| is_yes(item, tags::ANCHOR_POINT_VISIBILITY)),
        // A rotated or flipped display may swap the corners.
        bounding_box: corners.map(|(a, b)| (a.min(b), a.max(b))),
        style: line_style(item),
    }
}

fn line_style(item: &InMemDicomObject) -> LineStyle {
    let style = first_item(item, tags::LINE_STYLE_SEQUENCE);
    LineStyle {
        color: style.and_then(pattern_color),
        thickness: style
            .and_then(|s| item_f64(s, tags::LINE_THICKNESS))
            .unwrap_or(1.0),
        dashed: style
            .and_then(|s| item_text(s, tags::LINE_DASHING_STYLE))
            .is_some_and(|v| v.eq_ignore_ascii_case("DASHED")),
    }
}

/// Pattern On Color CIELab Value with Pattern On Opacity.
fn pattern_color(style: &InMemDicomObject) -> Option<Rgba> {
    let lab = ints(style, tags::PATTERN_ON_COLOR_CIE_LAB_VALUE);
    let [l, a, b] = lab[..] else {
        return None;
    };
    let color = Rgba::from_dicom_lab([l, a, b].map(to_u16));
    #[allow(clippy::cast_possible_truncation)]
    let opacity =
        item_f64(style, tags::PATTERN_ON_OPACITY).map_or(1.0, |o| o.clamp(0.0, 1.0) as f32);
    Some(color.with_alpha(opacity))
}

/// Recommended Display CIELab Value, else RGB Value, else Grayscale Value (16-bit P-value).
fn layer_color(layer: &InMemDicomObject) -> Option<Rgba> {
    let lab = ints(layer, tags::GRAPHIC_LAYER_RECOMMENDED_DISPLAY_CIE_LAB_VALUE);
    if let [l, a, b] = lab[..] {
        return Some(Rgba::from_dicom_lab([l, a, b].map(to_u16)));
    }
    // NOTE: the RGB value is retired (PS3.3 §C.10.7); still read for old files, like Java.
    #[allow(deprecated)]
    let rgb = ints(layer, tags::GRAPHIC_LAYER_RECOMMENDED_DISPLAY_RGB_VALUE);
    let channel = |v: i32| f32::from(u8::try_from(v.clamp(0, 255)).unwrap_or_default()) / 255.0;
    if let [r, g, b] = rgb[..] {
        return Some(Rgba::opaque(channel(r), channel(g), channel(b)));
    }
    let gray = ints(
        layer,
        tags::GRAPHIC_LAYER_RECOMMENDED_DISPLAY_GRAYSCALE_VALUE,
    );
    gray.first().map(|&p| {
        let value = channel(i32::from(to_u16(p) >> 8));
        Rgba::opaque(value, value, value)
    })
}

fn is_display(item: &InMemDicomObject, units: Tag) -> bool {
    item_text(item, units).is_some_and(|u| u.eq_ignore_ascii_case("DISPLAY"))
}

fn is_yes(item: &InMemDicomObject, tag: Tag) -> bool {
    item_text(item, tag).is_some_and(|v| v.eq_ignore_ascii_case("Y"))
}

fn to_u16(value: i32) -> u16 {
    u16::try_from(value.clamp(0, 0xFFFF)).unwrap_or_default()
}

fn items(parent: &InMemDicomObject, sequence: Tag) -> &[InMemDicomObject] {
    parent
        .element(sequence)
        .ok()
        .and_then(|element| element.items())
        .unwrap_or_default()
}

fn ints(item: &InMemDicomObject, tag: Tag) -> Vec<i32> {
    item.element(tag)
        .ok()
        .and_then(|e| e.to_multi_int::<i32>().ok())
        .unwrap_or_default()
}

fn floats(item: &InMemDicomObject, tag: Tag) -> Vec<f64> {
    item.element(tag)
        .ok()
        .and_then(|e| e.to_multi_float64().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use dicom_core::{DataElement, PrimitiveValue, VR};

    use super::*;
    use crate::image::SpatialTransform;

    fn graphic(kind: &str, data: &[f32], fill_lab: Option<[u16; 3]>) -> InMemDicomObject {
        let mut item = InMemDicomObject::from_element_iter([
            DataElement::new(tags::GRAPHIC_ANNOTATION_UNITS, VR::CS, "PIXEL"),
            DataElement::new(tags::GRAPHIC_TYPE, VR::CS, kind),
            DataElement::new(
                tags::GRAPHIC_DATA,
                VR::FL,
                PrimitiveValue::F32(data.iter().copied().collect()),
            ),
        ]);
        if let Some(lab) = fill_lab {
            let style = InMemDicomObject::from_element_iter([DataElement::new(
                tags::PATTERN_ON_COLOR_CIE_LAB_VALUE,
                VR::US,
                PrimitiveValue::U16(lab.iter().copied().collect()),
            )]);
            item.put(DataElement::new(
                tags::FILL_STYLE_SEQUENCE,
                VR::SQ,
                dicom_core::value::DataSetSequence::from(vec![style]),
            ));
        }
        item
    }

    fn space() -> AnnotationSpace {
        AnnotationSpace::new(100, 100, None, SpatialTransform::default())
    }

    #[test]
    fn circles_and_ellipses_are_measured_from_their_points() {
        let circle = graphic_object(
            &graphic("CIRCLE", &[10.0, 10.0, 13.0, 14.0], None),
            &space(),
        );
        assert_eq!(
            circle.map(|g| g.shape),
            Some(GraphicShape::Circle {
                center: DVec2::splat(10.0),
                radius: 5.0
            })
        );
        let data = [0.0, 10.0, 20.0, 10.0, 10.0, 5.0, 10.0, 15.0];
        let ellipse = graphic_object(&graphic("ELLIPSE", &data, None), &space());
        assert_eq!(
            ellipse.map(|g| g.shape),
            Some(GraphicShape::Ellipse {
                center: DVec2::new(10.0, 10.0),
                radii: DVec2::new(10.0, 5.0),
                rotation: 0.0
            })
        );
    }

    #[test]
    fn the_fill_color_replaces_the_line_color() {
        // DICOM L*a*b* of pure white: L* = 100, a* = b* = 0 (offset by 128).
        let white = [65535, 32896, 32896];
        let object = graphic_object(&graphic("POINT", &[1.0, 2.0], Some(white)), &space());
        let color = object.and_then(|g| g.style.color).map(Rgba::to_bytes);
        assert_eq!(color, Some([255, 255, 255, 255]));
    }

    #[test]
    fn unknown_or_incomplete_graphics_are_skipped() {
        assert_eq!(
            graphic_object(&graphic("CIRCLE", &[1.0, 1.0], None), &space()),
            None
        );
        assert_eq!(
            graphic_object(&graphic("SPLINE", &[1.0, 1.0, 2.0, 2.0], None), &space()),
            None
        );
    }
}
