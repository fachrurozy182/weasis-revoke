//! Declarative color maps (pseudo-color, fusion overlays, volume presets) and their sampling.
//!
//! A map is a list of stops on one position axis carrying a color curve and an alpha curve.
//! See `weasis-core/docs/ColorMap-Design.md`. In 2D, a `relative` or `absolute` map is spread
//! over the current window ([`ColorMap::byte_table`]); a `fixed` map keeps its colors at fixed
//! modality values ([`ColorMap::sample`]); a `percent` map needs a reference given by the caller
//! (fusion, RT dose; steps 6.4 and 6.7).
//!
//! Java reference: `weasis-core-img/.../opencv/op/lut/colormap/` (`ColorMap`, `ColorMapSampler`,
//! `ColorMapCompiler`, `ColorMapDomain`) and `weasis-core/.../api/image/lut/ColorMapJson.java`.

use serde::Deserialize;

use super::color_space::{ColorSpace, Rgba};
use crate::error::{Error, Result};

/// Built-in 2D color maps, the same file as the Java viewer.
pub const BUILT_IN_COLOR_MAPS_JSON: &str = include_str!("../../../../assets/config/colormaps.json");

/// How stop positions are read.
#[derive(Debug, Clone, PartialEq)]
pub enum DomainKind {
    /// `0.0..=1.0` over the current window.
    Relative,
    /// Physical unit; window/level rescales the map.
    Absolute,
    /// Physical unit; window/level never remaps the colors.
    Fixed,
    /// Percentage of a reference supplied by the caller.
    Percent,
}

/// Range of positions of a map.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorMapDomain {
    /// Kind of positions.
    pub kind: DomainKind,
    /// Unit of absolute and fixed domains (HU, SUVbw, Gy, ...).
    pub unit: Option<String>,
    /// Lowest position of the default range.
    pub min: f64,
    /// Highest position of the default range.
    pub max: f64,
    /// Reference of a percent domain (`max`, `window`, `prescription`).
    pub reference: Option<String>,
}

impl ColorMapDomain {
    fn span(&self) -> f64 {
        self.max - self.min
    }
}

/// Curve shape between stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    /// Straight blend between neighbouring stops.
    Linear,
    /// Each stop holds until the next one.
    Step,
    /// Stops are samples of a table, blended linearly.
    Sampled,
}

/// One control point; a stop may carry only a color or only an alpha.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorStop {
    /// Position on the domain axis.
    pub position: f64,
    /// Color, if the stop has one.
    pub color: Option<Rgba>,
    /// Opacity, if the stop has one.
    pub alpha: Option<f32>,
}

/// A color map.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorMap {
    /// Stable identifier (`weasis.`, `user.` prefix).
    pub id: String,
    /// Displayed name.
    pub name: String,
    /// Menu section.
    pub category: Option<String>,
    /// Modalities in whose menus the map appears; empty for all.
    pub modalities: Vec<String>,
    /// Whether the map is picked when a series of one of `modalities` opens.
    pub is_default: bool,
    /// Kept out of menus but resolvable.
    pub hidden: bool,
    /// Whether positions wrap around the domain (`cyclic` type).
    pub is_cyclic: bool,
    /// Positions axis.
    pub domain: ColorMapDomain,
    /// Blending space.
    pub space: ColorSpace,
    /// Curve shape.
    pub interpolation: Interpolation,
    /// Stops, sorted by position.
    pub stops: Vec<ColorStop>,
    /// Color below the first stop, if any.
    pub below: Option<Rgba>,
    /// Color above the last stop, if any.
    pub above: Option<Rgba>,
    /// Color of NaN values, if any.
    pub nan: Option<Rgba>,
}

impl ColorMap {
    /// Color and alpha at a domain value.
    pub fn sample(&self, value: f64) -> Rgba {
        if value.is_nan() {
            return self.nan.unwrap_or(Rgba::TRANSPARENT);
        }
        let value = if self.is_cyclic {
            self.wrap(value)
        } else {
            value
        };
        let first = self.stops.first().map_or(0.0, |s| s.position);
        let last = self.stops.last().map_or(0.0, |s| s.position);
        if !self.is_cyclic {
            if let Some(below) = self.below.filter(|_| value < first) {
                return below;
            }
            if let Some(above) = self.above.filter(|_| value > last) {
                return above;
            }
        }
        self.color_at(value).with_alpha(self.alpha_at(value))
    }

    /// 256 RGBA entries spread evenly over the domain, both ends included: the table applied
    /// to the 8-bit output of the window (`ColorMapCompiler.toByteLut`).
    pub fn byte_table(&self) -> Vec<[u8; 4]> {
        (0..=255u8)
            .map(|i| self.sample(self.domain.min + f64::from(i) / 255.0 * self.domain.span()))
            .map(Rgba::to_bytes)
            .collect()
    }

    fn wrap(&self, value: f64) -> f64 {
        let span = self.domain.span();
        self.domain.min + (value - self.domain.min).rem_euclid(span)
    }

    fn color_at(&self, value: f64) -> Rgba {
        let stops: Vec<(f64, Rgba)> = self
            .stops
            .iter()
            .filter_map(|s| s.color.map(|c| (s.position, c)))
            .collect();
        self.curve_at(&stops, value, Rgba::WHITE, |a, b, t| {
            self.space.mix(a, b, t)
        })
    }

    fn alpha_at(&self, value: f64) -> f32 {
        let stops: Vec<(f64, f32)> = self
            .stops
            .iter()
            .filter_map(|s| s.alpha.map(|a| (s.position, a)))
            .collect();
        #[allow(clippy::cast_possible_truncation)]
        self.curve_at(&stops, value, 1.0, |a, b, t| a + (b - a) * t as f32)
    }

    /// Value of a piecewise curve: constant outside the stops, blended (or held, for `Step`)
    /// between them, continuing across the domain end for cyclic maps.
    fn curve_at<T: Copy>(
        &self,
        stops: &[(f64, T)],
        value: f64,
        empty: T,
        mix: impl Fn(T, T, f64) -> T,
    ) -> T {
        let Some(&(_, first)) = stops.first() else {
            return empty;
        };
        let count = stops.len();
        let is_step = self.interpolation == Interpolation::Step;
        // Index of the last stop at or before the value; duplicates resolve to the later stop.
        let segment = stops.iter().rposition(|(position, _)| *position <= value);
        let across_seam = self.is_cyclic && count > 1 && segment.is_none_or(|i| i == count - 1);
        if across_seam {
            let (last_position, last) = stops[count - 1];
            let span = self.domain.span();
            let gap = stops[0].0 + span - last_position;
            let offset = if value >= last_position {
                value - last_position
            } else {
                value + span - last_position
            };
            let t = if gap > 0.0 && !is_step {
                offset / gap
            } else {
                0.0
            };
            return mix(last, first, t);
        }
        match segment {
            None => first,
            Some(i) if is_step || i == count - 1 => stops[i].1,
            Some(i) => {
                let (p0, c0) = stops[i];
                let (p1, c1) = stops[i + 1];
                mix(c0, c1, (value - p0) / (p1 - p0))
            }
        }
    }
}

/// Parses a color map document: `{"schema": 1, "maps": [...]}`, a bare array or one map.
pub fn parse_color_maps(json: &str) -> Result<Vec<ColorMap>> {
    let invalid = |error: String| Error::Configuration(format!("color maps: {error}"));
    let document: Document = serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
    let maps = match document {
        Document::Envelope { maps } => maps,
        Document::List(maps) => maps,
        Document::One(map) => vec![*map],
    };
    maps.into_iter()
        .map(|map| map.into_color_map().map_err(invalid))
        .collect()
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Document {
    Envelope { maps: Vec<MapJson> },
    List(Vec<MapJson>),
    One(Box<MapJson>),
}

#[derive(Deserialize)]
struct MapJson {
    id: String,
    name: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default, rename = "type")]
    map_type: Option<String>,
    #[serde(default)]
    modality: Vec<String>,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    domain: Option<DomainJson>,
    #[serde(default)]
    space: Option<String>,
    #[serde(default)]
    interpolation: Option<String>,
    stops: Vec<StopJson>,
    #[serde(default)]
    outside: Option<OutsideJson>,
}

#[derive(Deserialize)]
struct DomainJson {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    reference: Option<String>,
}

#[derive(Deserialize)]
struct StopJson {
    pos: f64,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    alpha: Option<f32>,
}

#[derive(Deserialize)]
struct OutsideJson {
    #[serde(default)]
    low: Option<String>,
    #[serde(default)]
    high: Option<String>,
    #[serde(default)]
    nan: Option<String>,
}

impl MapJson {
    fn into_color_map(self) -> std::result::Result<ColorMap, String> {
        let id = self.id;
        let domain = parse_domain(self.domain).map_err(|e| format!("map '{id}': {e}"))?;
        let mut stops = self
            .stops
            .into_iter()
            .map(|stop| {
                let color = match stop.color.as_deref() {
                    Some(text) => Some(
                        Rgba::parse(text).ok_or_else(|| format!("map '{id}': bad color {text}"))?,
                    ),
                    None => None,
                };
                Ok(ColorStop {
                    position: stop.pos,
                    color,
                    alpha: stop.alpha.map(|a| a.clamp(0.0, 1.0)),
                })
            })
            .collect::<std::result::Result<Vec<_>, String>>()?;
        if stops.is_empty() {
            return Err(format!("map '{id}' has no stop"));
        }
        // Stable sort: two stops at the same position keep their order and make a hard edge.
        stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        let lower = |text: Option<String>| text.unwrap_or_default().to_lowercase();
        let outside = self.outside;
        let outside_color = |pick: fn(&OutsideJson) -> Option<&String>| {
            outside
                .as_ref()
                .and_then(pick)
                .and_then(|text| Rgba::parse(text))
        };
        Ok(ColorMap {
            name: self.name,
            category: self.category,
            modalities: self
                .modality
                .into_iter()
                .map(|m| m.trim().to_uppercase())
                .collect(),
            is_default: self.default,
            hidden: self.hidden,
            is_cyclic: lower(self.map_type) == "cyclic",
            domain,
            space: match lower(self.space).as_str() {
                "hsl" => ColorSpace::Hsl,
                "lab" => ColorSpace::Lab,
                _ => ColorSpace::Rgb,
            },
            interpolation: match lower(self.interpolation).as_str() {
                "step" => Interpolation::Step,
                "sampled" => Interpolation::Sampled,
                _ => Interpolation::Linear,
            },
            stops,
            below: outside_color(|o| o.low.as_ref()),
            above: outside_color(|o| o.high.as_ref()),
            nan: outside_color(|o| o.nan.as_ref()),
            id,
        })
    }
}

fn parse_domain(json: Option<DomainJson>) -> std::result::Result<ColorMapDomain, String> {
    let Some(json) = json else {
        return Ok(ColorMapDomain {
            kind: DomainKind::Relative,
            unit: None,
            min: 0.0,
            max: 1.0,
            reference: None,
        });
    };
    let kind = match json.kind.unwrap_or_default().to_lowercase().as_str() {
        "absolute" => DomainKind::Absolute,
        "fixed" => DomainKind::Fixed,
        "percent" => DomainKind::Percent,
        _ => DomainKind::Relative,
    };
    let (min, max) = if kind == DomainKind::Relative {
        (0.0, 1.0)
    } else {
        (json.min.unwrap_or(0.0), json.max.unwrap_or(1.0))
    };
    if !(min.is_finite() && max.is_finite() && min < max) {
        return Err("domain range must be finite with min < max".to_owned());
    }
    if kind == DomainKind::Percent && json.reference.is_none() {
        return Err("a percent domain needs a reference".to_owned());
    }
    Ok(ColorMapDomain {
        kind,
        unit: json.unit,
        min,
        max,
        reference: json.reference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn built_in(id: &str) -> ColorMap {
        parse_color_maps(BUILT_IN_COLOR_MAPS_JSON)
            .expect("valid built-in maps")
            .into_iter()
            .find(|map| map.id == id)
            .unwrap_or_else(|| panic!("no map {id}"))
    }

    #[test]
    fn all_built_in_maps_are_read() {
        assert_eq!(
            parse_color_maps(BUILT_IN_COLOR_MAPS_JSON)
                .expect("valid")
                .len(),
            40
        );
    }

    #[test]
    fn dicom_well_known_palettes_match_ps3_6() {
        // Reference: pydicom apply_color_lut(palette=...) of the PS3.6 Annex B palettes:
        // (index, [r, g, b]) at the start, middle and end.
        let cases = [
            (
                "weasis.dicom.hot-iron",
                [[0, 0, 0], [255, 0, 0], [255, 255, 255]],
            ),
            (
                "weasis.dicom.pet",
                [[0, 0, 0], [128, 0, 255], [255, 255, 255]],
            ),
            (
                "weasis.dicom.hot-metal-blue",
                [[0, 0, 0], [116, 17, 97], [255, 255, 255]],
            ),
            (
                "weasis.dicom.pet-20-step",
                [[0, 0, 0], [80, 192, 80], [255, 255, 255]],
            ),
        ];
        for (id, expected) in cases {
            let table = built_in(id).byte_table();
            for (index, rgb) in [0, 128, 255].into_iter().zip(expected) {
                let actual = &table[index][..3];
                let close = actual.iter().zip(rgb).all(|(a, e)| a.abs_diff(e) <= 1);
                assert!(close, "{id}[{index}] = {actual:?}, expected {rgb:?}");
            }
        }
    }

    #[test]
    fn dicom_well_known_palettes_match_over_the_whole_table() {
        // Reference: per-channel sums of the 256 entries of pydicom apply_color_lut.
        let cases = [
            ("weasis.dicom.hot-iron", [48896, 16257, 8319]),
            ("weasis.dicom.pet", [32577, 24386, 32578]),
            ("weasis.dicom.hot-metal-blue", [31960, 17527, 26095]),
            ("weasis.dicom.pet-20-step", [33014, 29603, 22051]),
        ];
        for (id, expected) in cases {
            let table = built_in(id).byte_table();
            for (channel, expected_sum) in expected.into_iter().enumerate() {
                let sum: u32 = table.iter().map(|entry| u32::from(entry[channel])).sum();
                // At most one unit of rounding per entry.
                assert!(
                    sum.abs_diff(expected_sum) <= 256,
                    "{id} channel {channel}: {sum}"
                );
            }
        }
    }

    #[test]
    fn fixed_maps_keep_colors_at_modality_values() {
        let tissues = built_in("weasis.ct-tissues");
        assert_eq!(tissues.domain.kind, DomainKind::Fixed);
        assert_eq!(tissues.interpolation, Interpolation::Step);
        // Two values inside the same band get the same color.
        let first = tissues.stops[1].position;
        assert_eq!(
            tissues.sample(first).to_bytes(),
            tissues.sample(first + 0.1).to_bytes()
        );
    }

    #[test]
    fn transfer_maps_carry_an_alpha_curve() {
        let pet = built_in("weasis.pet-suv");
        assert_eq!(pet.sample(0.05).alpha, 0.0);
        assert_eq!(pet.sample(10.0).to_bytes(), [255, 255, 255, 255]);
    }

    #[test]
    fn cyclic_maps_wrap_around() {
        let map = parse_color_maps(
            r##"{"id": "c", "name": "C", "type": "cyclic",
                 "stops": [{"pos": 0.25, "color": "#ff0000"}, {"pos": 0.75, "color": "#0000ff"}]}"##,
        )
        .expect("valid")
        .remove(0);
        // Halfway across the seam between 0.75 and 1.25 (= 0.25).
        assert_eq!(map.sample(1.0).to_bytes(), map.sample(0.0).to_bytes());
        assert_eq!(map.sample(0.0).to_bytes(), [128, 0, 128, 255]);
    }
}
