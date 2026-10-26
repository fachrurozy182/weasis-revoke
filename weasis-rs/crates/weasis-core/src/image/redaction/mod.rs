//! Redaction of identity burned into the pixels: which regions to hide on an image, and how
//! they are filled.
//!
//! An image mask overrides the series mask; without either, the device library proposes the
//! regions of its matching entry, keeping those whose category the profile in force keeps.
//! Regions are filled with the median color of a thin ring of pixels around them.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/Redaction.java`,
//! `PixelPrivacy.java` and `.../display/RedactionRenderer.java`.
//
// TODO(7.6): convert library entries to and from the `masks:` block of a Karnak profile
// (`KarnakMasks`), offered by the preference page of the device masks.

use dicom_dictionary_std::tags;
use glam::DVec2;

use crate::dicom::pixel::Samples;

mod burn;
use crate::model::{
    AnonymizationAction, Attributes, DeviceKey, IdentityMask, MaskRegion, MaskingModel,
    best_pixel_mask,
};

/// Modalities whose images often carry burned-in identity.
const HIGH_RISK_MODALITIES: [&str; 8] = ["US", "XA", "XC", "SC", "OT", "ES", "GM", "OP"];

/// Whether the pixels may show identity (Burned In Annotation, Recognizable Visual Features).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelPrivacy {
    /// One of the attributes says YES.
    BurnedIn,
    /// The attributes say NO.
    Clean,
    /// Both attributes are absent (they are Type 3): silence is never "clean".
    Unknown,
}

impl PixelPrivacy {
    /// Verdict of an image.
    pub fn of(attributes: &Attributes) -> Self {
        let burned_in = attributes.get(tags::BURNED_IN_ANNOTATION);
        let features = attributes.get(tags::RECOGNIZABLE_VISUAL_FEATURES);
        let yes = |value: Option<&str>| value.is_some_and(|v| v.trim().eq_ignore_ascii_case("YES"));
        if yes(burned_in) || yes(features) {
            Self::BurnedIn
        } else if burned_in.is_none() && features.is_none() {
            Self::Unknown
        } else {
            Self::Clean
        }
    }

    /// Whether an output of the image should warn that identity may be visible.
    pub fn requires_review(attributes: &Attributes, modality: Option<&str>) -> bool {
        match Self::of(attributes) {
            Self::BurnedIn => true,
            Self::Clean => false,
            Self::Unknown => modality.is_some_and(|m| HIGH_RISK_MODALITIES.contains(&m)),
        }
    }
}

/// A region to hide, in image pixels.
#[derive(Debug, Clone, PartialEq)]
pub enum RedactionShape {
    /// Rectangle (x, y, width, height).
    Rect([f64; 4]),
    /// Ellipse inscribed in a rectangle (x, y, width, height).
    Ellipse([f64; 4]),
    /// Polygon.
    Polygon(Vec<DVec2>),
}

impl RedactionShape {
    /// A library region scaled to an image of `columns` × `rows`.
    pub fn from_region(region: &MaskRegion, columns: u32, rows: u32) -> Self {
        let size = DVec2::new(f64::from(columns), f64::from(rows));
        let scale = |[x, y, w, h]: [f64; 4]| [x * size.x, y * size.y, w * size.x, h * size.y];
        match region {
            MaskRegion::Rect(rect, _) => Self::Rect(scale(*rect)),
            MaskRegion::Ellipse(rect, _) => Self::Ellipse(scale(*rect)),
            MaskRegion::Polygon(points, _) => Self::Polygon(
                points
                    .iter()
                    .map(|&(x, y)| DVec2::new(x, y) * size)
                    .collect(),
            ),
        }
    }

    /// Whether the pixel whose top left corner is (x, y) is covered: its center is inside.
    ///
    /// NOTE: OpenCV `fillPoly`, used by the Java viewer, also fills pixels the outline only
    /// touches; edges may differ by one pixel.
    fn covers(&self, x: i64, y: i64) -> bool {
        #[allow(clippy::cast_precision_loss)]
        let center = DVec2::new(x as f64 + 0.5, y as f64 + 0.5);
        match self {
            Self::Rect([rx, ry, w, h]) => {
                (*rx..rx + w).contains(&center.x) && (*ry..ry + h).contains(&center.y)
            }
            Self::Ellipse([rx, ry, w, h]) => {
                let radii = DVec2::new(w / 2.0, h / 2.0);
                let offset = (center - DVec2::new(rx + radii.x, ry + radii.y)) / radii;
                offset.length_squared() <= 1.0
            }
            Self::Polygon(points) => even_odd(points, center),
        }
    }

    /// Pixel bounds (x0, y0, x1, y1), exclusive end.
    #[allow(clippy::cast_possible_truncation)]
    fn bounds(&self) -> [i64; 4] {
        let [x0, y0, x1, y1] = match self {
            Self::Rect([x, y, w, h]) | Self::Ellipse([x, y, w, h]) => [*x, *y, x + w, y + h],
            Self::Polygon(points) => points.iter().fold(
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                |[a, b, c, d], p| [a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)],
            ),
        };
        [
            x0.floor() as i64,
            y0.floor() as i64,
            x1.ceil() as i64,
            y1.ceil() as i64,
        ]
    }
}

fn even_odd(points: &[DVec2], p: DVec2) -> bool {
    let next = points.iter().cycle().skip(1);
    let mut inside = false;
    for (&a, &b) in points.iter().zip(next) {
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// The regions hidden on an image, with the library entry they come from.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RedactionMask {
    /// Regions to fill.
    pub shapes: Vec<RedactionShape>,
    /// Id of the library entry, `None` for regions drawn by the user.
    pub origin: Option<String>,
}

impl RedactionMask {
    /// The regions of the library entry matching an image, those whose category the mask in
    /// force does not keep. `None` without a mask in force or without a matching entry.
    pub fn from_library(
        model: &MaskingModel,
        mask: Option<&IdentityMask>,
        device: &DeviceKey,
        (columns, rows): (u32, u32),
    ) -> Option<Self> {
        let mask = mask?;
        let entry = best_pixel_mask(
            model.pixel_masks(),
            device,
            (columns, rows),
            Some(&mask.profile().id),
        )?;
        let shapes: Vec<RedactionShape> = entry
            .regions
            .iter()
            .filter(|region| {
                mask.profile().action_for_category(region.category()) != AnonymizationAction::Keep
            })
            .map(|region| RedactionShape::from_region(region, columns, rows))
            .collect();
        (!shapes.is_empty()).then(|| Self {
            shapes,
            origin: Some(entry.id.clone()),
        })
    }

    /// The mask of an image: its own, else the series one, else the library proposal. An empty
    /// mask is a decision too (it suppresses what lies below).
    pub fn effective(
        image: Option<&Self>,
        series: Option<&Self>,
        library: impl FnOnce() -> Option<Self>,
    ) -> Option<Self> {
        image.or(series).cloned().or_else(library)
    }

    /// Fills the regions of a frame of `width` × `height` pixels with `channels` interleaved
    /// samples per pixel.
    pub fn burn(&self, samples: &mut Samples, width: usize, height: usize, channels: usize) {
        burn::burn_shapes(&self.shapes, samples, (width, height, channels));
    }
}

#[cfg(test)]
mod tests {
    use super::burn::kernel_half_width;
    use super::*;

    #[test]
    fn privacy_follows_the_attributes() {
        let mut attributes = Attributes::new();
        assert_eq!(PixelPrivacy::of(&attributes), PixelPrivacy::Unknown);
        assert!(PixelPrivacy::requires_review(&attributes, Some("US")));
        assert!(!PixelPrivacy::requires_review(&attributes, Some("CT")));
        attributes.set(tags::BURNED_IN_ANNOTATION, "NO");
        assert_eq!(PixelPrivacy::of(&attributes), PixelPrivacy::Clean);
        attributes.set(tags::RECOGNIZABLE_VISUAL_FEATURES, "YES");
        assert_eq!(PixelPrivacy::of(&attributes), PixelPrivacy::BurnedIn);
    }

    #[test]
    fn the_kernel_matches_opencv() {
        let widths: Vec<i64> = (-4..=4).map(kernel_half_width).collect();
        assert_eq!(widths, [0, 3, 3, 4, 4, 4, 3, 3, 0]);
    }

    #[test]
    fn a_region_is_filled_with_the_median_around_it() {
        // 12 × 12 image of value 10, with "text" of value 255 in a 4 × 2 block.
        let (width, height) = (12, 12);
        let mut values = vec![10u8; width * height];
        for y in 5..7 {
            for x in 4..8 {
                values[y * width + x] = 255;
            }
        }
        let mut samples = Samples::U8(values);
        let mask = RedactionMask {
            shapes: vec![RedactionShape::Rect([4.0, 5.0, 4.0, 2.0])],
            origin: None,
        };
        mask.burn(&mut samples, width, height, 1);
        let Samples::U8(burned) = samples else {
            panic!("same type");
        };
        assert!(burned.iter().all(|&v| v == 10));
    }

    #[test]
    fn shapes_cover_pixels_by_their_center() {
        let ellipse = RedactionShape::Ellipse([0.0, 0.0, 10.0, 10.0]);
        assert!(ellipse.covers(5, 5));
        assert!(!ellipse.covers(0, 0));
        let triangle = RedactionShape::Polygon(vec![
            DVec2::ZERO,
            DVec2::new(10.0, 0.0),
            DVec2::new(0.0, 10.0),
        ]);
        assert!(triangle.covers(1, 1));
        assert!(!triangle.covers(8, 8));
    }

    #[test]
    fn the_image_mask_wins_and_the_library_comes_last() {
        let drawn = RedactionMask {
            shapes: vec![RedactionShape::Rect([0.0, 0.0, 1.0, 1.0])],
            origin: None,
        };
        let empty = RedactionMask::default();
        let library = || {
            Some(RedactionMask {
                shapes: Vec::new(),
                origin: Some("lib".to_owned()),
            })
        };
        assert_eq!(
            RedactionMask::effective(Some(&empty), Some(&drawn), library),
            Some(empty.clone())
        );
        assert_eq!(
            RedactionMask::effective(None, Some(&drawn), library),
            Some(drawn.clone())
        );
        assert_eq!(
            RedactionMask::effective(None, None, library).and_then(|m| m.origin),
            Some("lib".to_owned())
        );
    }

    #[test]
    fn the_library_keeps_what_the_profile_keeps() {
        let mut model = MaskingModel::built_in().expect("valid");
        model
            .add_layer(
                r#"{"masks": [{"id": "echo", "match": {"modality": "US"},
                    "reference": {"columns": 800, "rows": 600},
                    "regions": [
                      {"type": "rect", "category": "DIRECT_ID", "x": 0, "y": 0, "w": 0.5, "h": 0.1},
                      {"type": "rect", "category": "DEVICE", "x": 0.5, "y": 0, "w": 0.5, "h": 0.1}]}]}"#,
                false,
            )
            .expect("valid");
        let device = DeviceKey {
            modality: Some("US".to_owned()),
            ..DeviceKey::default()
        };
        let display = IdentityMask::new(&model, model.profile("display").expect("profile"), b"s");
        let shown = RedactionMask::from_library(&model, Some(&display), &device, (400, 300))
            .expect("entry");
        // The display profile keeps the device: only the identity banner is hidden.
        assert_eq!(
            shown.shapes,
            [RedactionShape::Rect([0.0, 0.0, 200.0, 30.0])]
        );
        let publication =
            IdentityMask::new(&model, model.profile("publication").expect("profile"), b"s");
        let all = RedactionMask::from_library(&model, Some(&publication), &device, (400, 300))
            .expect("entry");
        assert_eq!(all.shapes.len(), 2);
        assert_eq!(
            RedactionMask::from_library(&model, None, &device, (400, 300)),
            None
        );
    }
}
