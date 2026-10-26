//! Device library of burned-in identity: regions to hide on every image of an acquisition
//! device, normalized to the frame size they were drawn on.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/PixelMask.java`,
//! `MaskRegion.java`, `MaskingModel.readMask` and
//! `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/PixelMaskMatcher.java`.

use serde::Deserialize;

use super::TagCategory;

/// Relative tolerance on the aspect ratio of an image compared with the reference frame.
const RATIO_TOLERANCE: f64 = 0.01;

/// A region of a pixel mask, in fractions of the reference frame (0 to 1).
#[derive(Debug, Clone, PartialEq)]
pub enum MaskRegion {
    /// A rectangle (x, y, width, height).
    Rect([f64; 4], TagCategory),
    /// The ellipse inscribed in a rectangle (x, y, width, height).
    Ellipse([f64; 4], TagCategory),
    /// A polygon (x, y pairs).
    Polygon(Vec<(f64, f64)>, TagCategory),
}

impl MaskRegion {
    /// Category of identity the region hides; a profile that keeps it shows the pixels.
    pub fn category(&self) -> TagCategory {
        match self {
            Self::Rect(_, category) | Self::Ellipse(_, category) | Self::Polygon(_, category) => {
                *category
            }
        }
    }
}

/// Device the mask applies to: each value set must match, `*` at the end matches a prefix.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceKey {
    /// Modality.
    pub modality: Option<String>,
    /// Station Name.
    pub station_name: Option<String>,
    /// Manufacturer.
    pub manufacturer: Option<String>,
    /// Manufacturer's Model Name.
    pub model_name: Option<String>,
    /// Institution Name.
    pub institution_name: Option<String>,
}

impl DeviceKey {
    fn patterns(&self) -> [&Option<String>; 5] {
        [
            &self.modality,
            &self.station_name,
            &self.manufacturer,
            &self.model_name,
            &self.institution_name,
        ]
    }

    /// Number of values set: a more specific key wins.
    pub fn specificity(&self) -> usize {
        self.patterns().iter().filter(|p| p.is_some()).count()
    }

    /// Whether the values of a device (in the same order as the fields) match.
    pub fn matches(&self, device: &DeviceKey) -> bool {
        self.patterns()
            .iter()
            .zip(device.patterns())
            .all(|(pattern, value)| accepts(pattern.as_deref(), value.as_deref()))
    }
}

fn accepts(pattern: Option<&str>, value: Option<&str>) -> bool {
    let Some(pattern) = pattern else {
        return true;
    };
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return false;
    };
    let (expected, actual) = (pattern.trim().to_uppercase(), value.to_uppercase());
    match expected.strip_suffix('*') {
        Some(prefix) => actual.starts_with(prefix),
        None => actual == expected,
    }
}

/// One entry of the device library.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelMask {
    /// Stable id.
    pub id: String,
    /// Displayed name.
    pub name: String,
    /// Device the regions belong to.
    pub device: DeviceKey,
    /// Columns and rows of the frame the regions were drawn on.
    pub reference: (u32, u32),
    /// Regions to hide.
    pub regions: Vec<MaskRegion>,
    /// Profiles the mask is restricted to; empty for all.
    pub profiles: Vec<String>,
    /// Whether the entry is in use.
    pub enabled: bool,
}

impl PixelMask {
    /// Why the mask does not apply to an image, `None` when it applies.
    pub fn rejection(
        &self,
        device: &DeviceKey,
        columns: u32,
        rows: u32,
        profile: Option<&str>,
    ) -> Option<Rejection> {
        if !self.enabled {
            Some(Rejection::Disabled)
        } else if !self.device.matches(device) {
            Some(Rejection::Device)
        } else if !self.fits(columns, rows) {
            Some(Rejection::Geometry)
        } else if profile
            .is_some_and(|p| !self.profiles.is_empty() && !self.profiles.iter().any(|id| id == p))
        {
            Some(Rejection::Profile)
        } else {
            None
        }
    }

    /// Whether an image has the aspect ratio of the reference frame: a banner sits in a band of
    /// the layout, so the regions are never stretched onto another shape.
    pub fn fits(&self, columns: u32, rows: u32) -> bool {
        if columns == 0 || rows == 0 {
            return false;
        }
        let reference = f64::from(self.reference.0) / f64::from(self.reference.1);
        let ratio = f64::from(columns) / f64::from(rows);
        (ratio - reference).abs() <= RATIO_TOLERANCE * reference
    }
}

/// Why a pixel mask does not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The entry is disabled.
    Disabled,
    /// Another device.
    Device,
    /// Another frame shape.
    Geometry,
    /// Restricted to other profiles.
    Profile,
}

/// The entry for an image: among those that apply, the most specific device key, then the most
/// recent layer (later in `masks`).
pub fn best_pixel_mask<'a>(
    masks: &'a [PixelMask],
    device: &DeviceKey,
    (columns, rows): (u32, u32),
    profile: Option<&str>,
) -> Option<&'a PixelMask> {
    masks
        .iter()
        .enumerate()
        .filter(|(_, m)| m.rejection(device, columns, rows, profile).is_none())
        .max_by_key(|(index, m)| (m.device.specificity(), *index))
        .map(|(_, m)| m)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PixelMaskJson {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "match")]
    device: DeviceJson,
    reference: ReferenceJson,
    #[serde(default)]
    regions: Vec<RegionJson>,
    #[serde(default)]
    profiles: Vec<String>,
    #[serde(default)]
    enabled: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DeviceJson {
    modality: Option<String>,
    station_name: Option<String>,
    manufacturer: Option<String>,
    model_name: Option<String>,
    institution_name: Option<String>,
}

#[derive(Deserialize)]
struct ReferenceJson {
    columns: u32,
    rows: u32,
}

#[derive(Deserialize)]
struct RegionJson {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    category: Option<TagCategory>,
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    #[serde(default)]
    w: f64,
    #[serde(default)]
    h: f64,
    #[serde(default)]
    points: Vec<f64>,
}

impl PixelMaskJson {
    /// The entry, or `None` when it is invalid (skipped like in the Java viewer).
    pub(super) fn into_mask(self) -> Option<PixelMask> {
        if self.reference.columns == 0 || self.reference.rows == 0 {
            return None;
        }
        let text =
            |value: Option<String>| value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        Some(PixelMask {
            name: text(self.name).unwrap_or_else(|| self.id.clone()),
            id: self.id,
            device: DeviceKey {
                modality: text(self.device.modality),
                station_name: text(self.device.station_name),
                manufacturer: text(self.device.manufacturer),
                model_name: text(self.device.model_name),
                institution_name: text(self.device.institution_name),
            },
            reference: (self.reference.columns, self.reference.rows),
            regions: self
                .regions
                .into_iter()
                .filter_map(RegionJson::into_region)
                .collect(),
            profiles: self.profiles,
            enabled: self.enabled.unwrap_or(true),
        })
    }
}

impl RegionJson {
    fn into_region(self) -> Option<MaskRegion> {
        // A region without a category hides a direct identifier, as in the Java viewer.
        let category = self.category.unwrap_or(TagCategory::DirectId);
        let rect = [self.x, self.y, self.w, self.h];
        match self.kind.to_lowercase().as_str() {
            "rect" if self.w > 0.0 && self.h > 0.0 => Some(MaskRegion::Rect(rect, category)),
            "ellipse" if self.w > 0.0 && self.h > 0.0 => Some(MaskRegion::Ellipse(rect, category)),
            "polygon" if self.points.len() >= 6 && self.points.len().is_multiple_of(2) => {
                Some(MaskRegion::Polygon(
                    self.points.chunks_exact(2).map(|p| (p[0], p[1])).collect(),
                    category,
                ))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(id: &str, device: DeviceKey) -> PixelMask {
        PixelMask {
            id: id.to_owned(),
            name: id.to_owned(),
            device,
            reference: (800, 600),
            regions: vec![MaskRegion::Rect(
                [0.0, 0.0, 1.0, 0.1],
                TagCategory::DirectId,
            )],
            profiles: Vec::new(),
            enabled: true,
        }
    }

    fn device(modality: &str, station: &str) -> DeviceKey {
        DeviceKey {
            modality: Some(modality.to_owned()),
            station_name: Some(station.to_owned()),
            ..DeviceKey::default()
        }
    }

    #[test]
    fn the_most_specific_matching_device_wins() {
        let any_us = mask(
            "us",
            DeviceKey {
                modality: Some("US".to_owned()),
                ..DeviceKey::default()
            },
        );
        let station = mask("echo", device("US", "ECHO*"));
        let masks = [any_us, station];
        let image = device("us", "Echo-3");
        assert_eq!(
            best_pixel_mask(&masks, &image, (1600, 1200), None).map(|m| m.id.as_str()),
            Some("echo")
        );
        let other = device("US", "CARDIO");
        assert_eq!(
            best_pixel_mask(&masks, &other, (800, 600), None).map(|m| m.id.as_str()),
            Some("us")
        );
    }

    #[test]
    fn another_shape_or_profile_is_rejected() {
        let mut entry = mask("m", DeviceKey::default());
        assert_eq!(
            entry.rejection(&DeviceKey::default(), 800, 800, None),
            Some(Rejection::Geometry)
        );
        entry.profiles = vec!["teaching".to_owned()];
        assert_eq!(
            entry.rejection(&DeviceKey::default(), 400, 300, Some("display")),
            Some(Rejection::Profile)
        );
        assert_eq!(
            entry.rejection(&DeviceKey::default(), 400, 300, Some("teaching")),
            None
        );
    }
}
