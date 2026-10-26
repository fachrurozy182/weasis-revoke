//! Configured window/level presets: model, JSON documents and layers.
//!
//! Presets are read from JSON documents (`assets/config/windowPresets.json` for the built-in
//! layer, then the site and user layers). A later layer replaces a preset of the same id in
//! place and appends new ones. See `weasis-dicom/weasis-dicom-codec/docs/Window-Presets.md`.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/.../display/WindowPreset.java`,
//! `WindowPresetJson.java` and `WindowPresetRegistry.java`.

use serde::Deserialize;

use super::LutShape;
use crate::dicom::anatomy::{AnatomicRegion, selects};
use crate::error::{Error, Result};

/// Built-in presets, the same file as the Java viewer.
pub const BUILT_IN_PRESETS_JSON: &str =
    include_str!("../../../../assets/config/windowPresets.json");

/// Range the window and level of a percent preset refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercentOf {
    /// Value range of the image.
    Image,
    /// Smallest / Largest Pixel Value in Series.
    Series,
}

/// Unit of a preset's window and level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Modality values (e.g. Hounsfield units).
    Absolute,
    /// Percent of a value range.
    Percent(PercentOf),
}

/// Conditions an image must meet for a preset to be offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct When {
    /// Minimum Bits Stored (default 9: presets make no sense for 8-bit images).
    pub min_bits_stored: u16,
    /// Whether the image needs a Modality LUT (rescale or table).
    pub requires_rescale: bool,
    /// Anatomy tokens the image must match; empty for any anatomy.
    pub body_parts: Vec<String>,
    /// Anatomy tokens that only move the preset up the list.
    pub preferred_body_parts: Vec<String>,
}

impl Default for When {
    fn default() -> Self {
        Self {
            min_bits_stored: 9,
            requires_rescale: false,
            body_parts: Vec::new(),
            preferred_body_parts: Vec::new(),
        }
    }
}

impl When {
    /// Whether an image meets the conditions.
    pub fn matches(
        &self,
        bits_stored: u16,
        has_rescale: bool,
        anatomy: Option<&AnatomicRegion>,
    ) -> bool {
        bits_stored >= self.min_bits_stored
            && (!self.requires_rescale || has_rescale)
            && (self.body_parts.is_empty() || selects(&self.body_parts, anatomy))
    }

    /// Whether the preset belongs first in the list for this anatomy.
    pub fn prefers(&self, anatomy: Option<&AnatomicRegion>) -> bool {
        selects(&self.preferred_body_parts, anatomy) || selects(&self.body_parts, anatomy)
    }
}

/// One configured preset.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowPreset {
    /// Stable id (`weasis.`, `site.` or `user.` prefix).
    pub id: String,
    /// Displayed name.
    pub name: String,
    /// Modalities it applies to (upper case); empty for all.
    pub modalities: Vec<String>,
    /// Not offered when true.
    pub hidden: bool,
    /// Window width.
    pub window: f64,
    /// Window center.
    pub level: f64,
    /// Unit of window and level.
    pub domain: Domain,
    /// LUT shape.
    pub shape: LutShape,
    /// Shortcut digit, 3 to 9.
    pub key: Option<char>,
    /// Conditions.
    pub when: When,
    /// Whether it is the default when the image carries no window.
    pub is_default: bool,
}

impl WindowPreset {
    /// Whether the preset applies to a modality.
    pub fn applies_to(&self, modality: &str) -> bool {
        self.modalities.is_empty() || self.modalities.iter().any(|m| m == modality)
    }

    /// Window and level for an image: as configured, or computed from a value range for
    /// percent presets. `None` when the range gives no valid window.
    pub fn resolve(&self, range_min: f64, range_max: f64) -> Option<(f64, f64)> {
        match self.domain {
            Domain::Absolute => Some((self.window, self.level)),
            Domain::Percent(_) => {
                let range = range_max - range_min;
                let window = range * self.window / 100.0;
                let level = range_min + range * self.level / 100.0;
                (window.is_finite() && window > 0.0 && level.is_finite()).then_some((window, level))
            }
        }
    }
}

/// Presets merged from successive layers (built-in, site, user), with the layer of each.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetLayers {
    presets: Vec<(WindowPreset, usize)>,
}

impl PresetLayers {
    /// The built-in layer only.
    pub fn built_in() -> Result<Self> {
        let mut layers = Self::default();
        layers.add_layer(parse_presets(BUILT_IN_PRESETS_JSON)?);
        Ok(layers)
    }

    /// Adds a layer: a preset with a known id replaces it in place, others are appended.
    pub fn add_layer(&mut self, presets: Vec<WindowPreset>) {
        let layer = self.presets.iter().map(|(_, l)| l + 1).max().unwrap_or(0);
        for preset in presets {
            match self.presets.iter_mut().find(|(p, _)| p.id == preset.id) {
                Some(existing) => *existing = (preset, layer),
                None => self.presets.push((preset, layer)),
            }
        }
    }

    /// Presets with their layer, in merged order.
    pub fn presets(&self) -> &[(WindowPreset, usize)] {
        &self.presets
    }
}

/// Parses a presets document: `{"schema": 1, "presets": [...]}` or a bare array.
pub fn parse_presets(json: &str) -> Result<Vec<WindowPreset>> {
    let invalid = |error: String| Error::Configuration(format!("window presets: {error}"));
    let document: Document = serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
    let entries = match document {
        Document::Envelope { presets } => presets,
        Document::List(presets) => presets,
    };
    entries
        .into_iter()
        .map(|entry| entry.into_preset().map_err(invalid))
        .collect()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Document {
    Envelope { presets: Vec<PresetJson> },
    List(Vec<PresetJson>),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetJson {
    id: String,
    name: String,
    #[serde(default)]
    modality: OneOrMany,
    #[serde(default)]
    hidden: bool,
    window: f64,
    level: f64,
    #[serde(default)]
    domain: Option<DomainJson>,
    #[serde(default)]
    shape: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    when: Option<WhenJson>,
    #[serde(default)]
    default: bool,
}

#[derive(Deserialize)]
struct DomainJson {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    reference: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhenJson {
    #[serde(default)]
    min_bits_stored: Option<u16>,
    #[serde(default)]
    requires_rescale: bool,
    #[serde(default)]
    body_part: OneOrMany,
    #[serde(default)]
    prefer_body_part: OneOrMany,
}

/// A JSON string or array of strings.
#[derive(Deserialize, Default)]
#[serde(untagged)]
enum OneOrMany {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_codes(self) -> Vec<String> {
        let values = match self {
            Self::None => Vec::new(),
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        };
        values
            .into_iter()
            .map(|v| v.trim().to_uppercase())
            .filter(|v| !v.is_empty())
            .collect()
    }
}

impl PresetJson {
    fn into_preset(self) -> std::result::Result<WindowPreset, String> {
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err("a preset needs an id and a name".to_owned());
        }
        if !self.window.is_finite() || self.window <= 0.0 || !self.level.is_finite() {
            return Err(format!(
                "preset '{}' has an invalid window or level",
                self.id
            ));
        }
        let domain = match self.domain {
            Some(DomainJson {
                kind: Some(kind),
                reference,
            }) if kind.eq_ignore_ascii_case("percent") => {
                let of = match reference.as_deref() {
                    Some(r) if r.eq_ignore_ascii_case("series") => PercentOf::Series,
                    _ => PercentOf::Image,
                };
                Domain::Percent(of)
            }
            _ => Domain::Absolute,
        };
        let when = self.when.map_or_else(When::default, |w| When {
            min_bits_stored: w.min_bits_stored.unwrap_or(9),
            requires_rescale: w.requires_rescale,
            body_parts: w.body_part.into_codes(),
            preferred_body_parts: w.prefer_body_part.into_codes(),
        });
        Ok(WindowPreset {
            modalities: self.modality.into_codes(),
            hidden: self.hidden,
            window: self.window,
            level: self.level,
            domain,
            shape: shape_from_json(self.shape.as_deref()),
            key: self.key.and_then(|k| k.trim().chars().next()),
            when,
            is_default: self.default,
            id: self.id,
            name: self.name,
        })
    }
}

/// Shape names of the JSON: the DICOM VOI LUT Functions and the Weasis extensions, in any case,
/// with `-` or `_`. Unknown names fall back to LINEAR, like the Java viewer.
fn shape_from_json(text: Option<&str>) -> LutShape {
    let name = text
        .unwrap_or_default()
        .trim()
        .replace('-', "_")
        .to_uppercase();
    match name.as_str() {
        "LINEAR_EXACT" => LutShape::LinearExact,
        "SIGMOID" => LutShape::Sigmoid,
        "SIGMOID_NORM" => LutShape::SigmoidNormalized,
        "LOG" => LutShape::Log,
        "LOG_INV" => LutShape::LogInverse,
        _ => LutShape::Linear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_presets_are_read() {
        let layers = PresetLayers::built_in().expect("valid built-in presets");
        let lung = layers
            .presets()
            .iter()
            .map(|(p, _)| p)
            .find(|p| p.id == "weasis.ct.lung")
            .expect("lung preset");
        assert_eq!(
            (lung.window, lung.level, lung.key),
            (1500.0, -600.0, Some('7'))
        );
        assert_eq!(lung.modalities, vec!["CT"]);
        assert_eq!(lung.when.preferred_body_parts, vec!["CHEST"]);
    }

    #[test]
    fn a_later_layer_replaces_a_preset_in_place() {
        let mut layers = PresetLayers::built_in().expect("valid");
        let position = layers
            .presets()
            .iter()
            .position(|(p, _)| p.id == "weasis.ct.lung");
        let user = parse_presets(
            r#"[{"id": "weasis.ct.lung", "name": "My lung", "window": 1600, "level": -550},
                {"id": "user.mr.t2", "name": "T2", "modality": "mr", "window": 50,
                 "level": 50, "domain": {"kind": "percent", "reference": "series"}}]"#,
        )
        .expect("valid");
        layers.add_layer(user);
        let presets = layers.presets();
        assert_eq!(
            presets.iter().position(|(p, _)| p.id == "weasis.ct.lung"),
            position
        );
        let last = presets.last().expect("appended");
        assert_eq!(last.0.domain, Domain::Percent(PercentOf::Series));
        assert_eq!(
            (last.0.modalities.clone(), last.1),
            (vec!["MR".to_owned()], 1)
        );
    }

    #[test]
    fn percent_presets_resolve_against_a_range() {
        let preset = parse_presets(
            r#"[{"id": "p", "name": "P", "window": 50, "level": 25, "domain": {"kind": "percent"}}]"#,
        )
        .expect("valid")
        .remove(0);
        assert_eq!(preset.resolve(-1000.0, 3000.0), Some((2000.0, 0.0)));
        assert_eq!(preset.resolve(10.0, 10.0), None);
    }

    #[test]
    fn invalid_presets_are_rejected() {
        assert!(parse_presets(r#"[{"id": "x", "name": "X", "window": 0, "level": 1}]"#).is_err());
        assert!(parse_presets("not json").is_err());
    }
}
