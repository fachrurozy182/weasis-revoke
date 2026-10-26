//! The window/level presets offered for one image, in the order of the Java viewer:
//!
//! 1. the windows of the image (its presentation state's first, step 1.11), then its VOI LUTs;
//! 2. *Auto Level*, the value range of the image;
//! 3. the configured presets the image meets, those preferred for its anatomy first.
//!
//! The first entry is the default. A configured preset flagged `default` takes the first place
//! only when the image carries neither a window nor a VOI LUT.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/lut/PresetWindowLevel.java`
//! (`PresetCollectionBuilder`) and `weasis-dicom-codec/.../display/WindowPresetRegistry.java`
//! (`getPresets`, `build`, `keyOwners`, `selectDefault`).

use super::window_preset::Domain;
use super::{LutShape, PercentOf, PresetLayers, VoiLut, WindowPreset, sequence_window};
use crate::dicom::anatomy::AnatomicRegion;

/// Suffix of the presets carried by the image (`dicomKeyWord` in the Java viewer).
const DICOM_SUFFIX: &str = " [DICOM]";

/// A window/level ready to apply.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetWindowLevel {
    /// Displayed name.
    pub name: String,
    /// Window width.
    pub window: f64,
    /// Window center.
    pub level: f64,
    /// LUT shape.
    pub shape: LutShape,
    /// Keyboard shortcut digit.
    pub key: Option<char>,
}

/// What the preset list needs to know about the image.
#[derive(Debug, Clone)]
pub struct PresetContext<'a> {
    /// Modality, e.g. "CT".
    pub modality: &'a str,
    /// Bits Stored.
    pub bits_stored: u16,
    /// Whether the image has a Modality LUT (rescale or table).
    pub has_rescale: bool,
    /// Anatomy of the image.
    pub anatomy: Option<&'a AnatomicRegion>,
    /// Smallest and largest modality value of the image.
    pub image_range: (f64, f64),
    /// Smallest and largest modality value of the series, when the image carries it.
    pub series_range: Option<(f64, f64)>,
}

/// Builds the preset list of an image from its VOI attributes and the configured presets.
pub fn preset_list(
    context: &PresetContext,
    voi: &VoiLut,
    configured: &PresetLayers,
) -> Vec<PresetWindowLevel> {
    let mut list = image_presets(voi);
    let carried_by_image = !list.is_empty();
    list.push(auto_level(context.image_range, &voi.function));

    let mut configured_list = configured_presets(context, configured);
    let fallback = if carried_by_image {
        None
    } else {
        configured_list
            .iter()
            .position(|(_, is_default)| *is_default)
    };
    if let Some(index) = fallback {
        let (preset, _) = configured_list.remove(index);
        list.insert(0, preset);
    }
    list.extend(configured_list.into_iter().map(|(preset, _)| preset));
    list
}

/// Windows and VOI LUTs of the image; the first two get the keys 1 and 2.
fn image_presets(voi: &VoiLut) -> Vec<PresetWindowLevel> {
    let mut list: Vec<PresetWindowLevel> = Vec::new();
    for (index, window) in voi.windows.iter().enumerate() {
        let name = window
            .explanation
            .clone()
            .unwrap_or_else(|| format!("Default {}", index + 1));
        let preset = PresetWindowLevel {
            name: name + DICOM_SUFFIX,
            window: window.width,
            level: window.center,
            shape: voi.function.clone(),
            key: None,
        };
        let is_duplicate = list.iter().any(|p| {
            p.window == preset.window && p.level == preset.level && p.shape == preset.shape
        });
        if !is_duplicate {
            list.push(preset);
        }
    }
    for (index, (table, explanation)) in voi.tables.iter().enumerate() {
        let (window, level) = sequence_window(table);
        let name = explanation
            .clone()
            .unwrap_or_else(|| format!("VOI LUT {index}"));
        list.push(PresetWindowLevel {
            name: name + DICOM_SUFFIX,
            window,
            level,
            shape: LutShape::Sequence(table.clone()),
            key: None,
        });
    }
    for (preset, key) in list.iter_mut().zip(['1', '2']) {
        preset.key = Some(key);
    }
    list
}

/// The full value range of the image, key 0. LINEAR becomes LINEAR_EXACT so that the window
/// covers exactly the range (PS3.3 C.11.2.1.3.2).
fn auto_level(range: (f64, f64), function: &LutShape) -> PresetWindowLevel {
    let (min, max) = range;
    let shape = match function {
        LutShape::Linear => LutShape::LinearExact,
        other => other.clone(),
    };
    PresetWindowLevel {
        name: "Auto Level [Image]".to_owned(),
        window: (max - min).max(1.0),
        level: (min + max) / 2.0,
        shape,
        key: Some('0'),
    }
}

/// Configured presets offered for the image, each with whether it is the default.
fn configured_presets(
    context: &PresetContext,
    configured: &PresetLayers,
) -> Vec<(PresetWindowLevel, bool)> {
    let mut offered: Vec<(&WindowPreset, usize)> = configured
        .presets()
        .iter()
        .filter(|(preset, _)| {
            !preset.hidden
                && preset.applies_to(context.modality)
                && preset
                    .when
                    .matches(context.bits_stored, context.has_rescale, context.anatomy)
        })
        .map(|(preset, layer)| (preset, *layer))
        .collect();
    // Stable sort: the presets of the image anatomy move up, the others keep their order.
    offered.sort_by_key(|(preset, _)| !preset.when.prefers(context.anatomy));

    let default_id = owner(&offered, |p| p.is_default);
    offered
        .iter()
        .filter_map(|(preset, _)| {
            let range = match preset.domain {
                Domain::Percent(PercentOf::Series) => {
                    context.series_range.unwrap_or(context.image_range)
                }
                _ => context.image_range,
            };
            let (window, level) = preset.resolve(range.0, range.1)?;
            // A shared key goes to one preset only.
            let key = preset
                .key
                .filter(|&key| owner(&offered, |p| p.key == Some(key)) == Some(&preset.id));
            let is_default = default_id == Some(&preset.id);
            Some((
                PresetWindowLevel {
                    name: preset.name.clone(),
                    window,
                    level,
                    shape: preset.shape.clone(),
                    key,
                },
                is_default,
            ))
        })
        .collect()
}

/// Id of the offered preset matching `predicate` in the latest layer; the first one when that
/// layer has several (`LayeredEntries.pick` in the Java viewer).
fn owner<'a>(
    offered: &[(&'a WindowPreset, usize)],
    predicate: impl Fn(&WindowPreset) -> bool,
) -> Option<&'a String> {
    let layer = offered
        .iter()
        .filter(|(preset, _)| predicate(preset))
        .map(|(_, layer)| *layer)
        .max()?;
    offered
        .iter()
        .find(|(preset, l)| *l == layer && predicate(preset))
        .map(|(preset, _)| &preset.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{VoiWindow, parse_presets};

    fn ct(anatomy: Option<&AnatomicRegion>) -> PresetContext<'_> {
        PresetContext {
            modality: "CT",
            bits_stored: 12,
            has_rescale: true,
            anatomy,
            image_range: (-1024.0, 3071.0),
            series_range: None,
        }
    }

    fn no_voi() -> VoiLut {
        VoiLut {
            windows: Vec::new(),
            function: LutShape::Linear,
            tables: Vec::new(),
        }
    }

    fn names(list: &[PresetWindowLevel]) -> Vec<&str> {
        list.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn image_windows_come_first_then_auto_level_then_configured_presets() {
        let voi = VoiLut {
            windows: vec![
                VoiWindow {
                    center: 40.0,
                    width: 400.0,
                    explanation: Some("SOFT TISSUE".to_owned()),
                },
                VoiWindow {
                    center: 300.0,
                    width: 1800.0,
                    explanation: None,
                },
            ],
            function: LutShape::Linear,
            tables: Vec::new(),
        };
        let list = preset_list(&ct(None), &voi, &PresetLayers::built_in().expect("valid"));
        assert_eq!(
            &names(&list)[..4],
            [
                "SOFT TISSUE [DICOM]",
                "Default 2 [DICOM]",
                "Auto Level [Image]",
                "Brain"
            ]
        );
        assert_eq!(list[0].key, Some('1'));
        assert_eq!(list[2].key, Some('0'));
        assert_eq!(list[2].shape, LutShape::LinearExact);
        assert_eq!((list[2].window, list[2].level), (4095.0, 1023.5));
    }

    #[test]
    fn presets_of_the_image_anatomy_move_up() {
        let chest = AnatomicRegion::from_term("CHEST");
        let list = preset_list(
            &ct(Some(&chest)),
            &no_voi(),
            &PresetLayers::built_in().expect("valid"),
        );
        assert_eq!(&names(&list)[1..3], ["Mediastinum", "Lung"]);
    }

    #[test]
    fn eight_bit_images_get_no_configured_preset() {
        let mut context = ct(None);
        context.bits_stored = 8;
        let list = preset_list(
            &context,
            &no_voi(),
            &PresetLayers::built_in().expect("valid"),
        );
        assert_eq!(names(&list), ["Auto Level [Image]"]);
    }

    #[test]
    fn default_preset_leads_only_when_the_image_has_no_window() {
        let mut layers = PresetLayers::built_in().expect("valid");
        layers.add_layer(
            parse_presets(r#"[{"id": "user.ct.d", "name": "Mine", "modality": "CT", "window": 10, "level": 5, "default": true}]"#)
                .expect("valid"),
        );
        let list = preset_list(&ct(None), &no_voi(), &layers);
        assert_eq!(list[0].name, "Mine");
    }
}
