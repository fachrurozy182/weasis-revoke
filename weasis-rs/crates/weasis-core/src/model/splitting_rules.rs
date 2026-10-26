//! Rules that split a DICOM series into sub-series of images that can be shown as one stack.
//!
//! An image joins a sub-series when, for every rule of its modality, its value equals the value
//! of the first image of that sub-series. A rule with a condition tolerates a different value
//! when the condition does not hold for the new image.
//!
//! NOTE: values are compared in their DICOM string form, value by value, and numerically when
//! both parse as numbers; the Java viewer compares typed values, which gives the same result.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/utils/SplittingRules.java`
//! and `SplittingModalityRules.java`.

use std::sync::OnceLock;

use dicom_dictionary_std::tags;
use dicom_object::Tag;

use super::Attributes;
use crate::geometry::{ImagePlane, parse_orientation};

/// Modality whose rules apply when the series modality has none.
const DEFAULT_MODALITY: &str = "DEFAULT";

/// The value a rule or a condition looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKey {
    /// A DICOM attribute of the instance.
    Tag(Tag),
    /// The plane derived from Image Orientation (Patient) (`ImageOrientationPlane` in Java).
    ImagePlane,
}

impl RuleKey {
    /// Value of an instance, in DICOM string form.
    pub fn value(self, attributes: &Attributes) -> Option<String> {
        match self {
            Self::Tag(tag) => attributes.get(tag).map(str::to_owned),
            Self::ImagePlane => {
                let (row, column) =
                    parse_orientation(attributes.get(tags::IMAGE_ORIENTATION_PATIENT)?)?;
                Some(ImagePlane::from_orientation(row, column).name().to_owned())
            }
        }
    }
}

/// A test on one value of the new instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Compares a value with an expected one.
    Test {
        /// Value tested.
        key: RuleKey,
        /// `true` for "contains" (any value contains the text), `false` for "equals".
        contains: bool,
        /// Case-insensitive comparison.
        ignore_case: bool,
        /// Inverts the result.
        negate: bool,
        /// Expected value.
        expected: String,
    },
    /// Holds when all the conditions hold (or none, if negated).
    AllOf(Vec<Condition>, bool),
    /// Holds when one of the conditions holds (or none holds, if negated).
    AnyOf(Vec<Condition>, bool),
}

impl Condition {
    /// Whether the condition holds for an instance.
    pub fn holds(&self, attributes: &Attributes) -> bool {
        match self {
            Self::Test {
                key,
                contains,
                ignore_case,
                negate,
                expected,
            } => {
                let value = key.value(attributes);
                let result = if *contains {
                    contains_text(value.as_deref(), expected, *ignore_case)
                } else {
                    values_equal(value.as_deref(), Some(expected), *ignore_case)
                };
                result != *negate
            }
            Self::AllOf(conditions, negate) => {
                conditions.iter().all(|c| c.holds(attributes)) != *negate
            }
            Self::AnyOf(conditions, negate) => {
                conditions.iter().any(|c| c.holds(attributes)) != *negate
            }
        }
    }
}

/// One value that must be the same in all the images of a sub-series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Value compared.
    pub key: RuleKey,
    /// When set, a different value only splits when this condition holds for the new image.
    pub condition: Option<Condition>,
}

impl Rule {
    fn new(key: RuleKey) -> Self {
        Self {
            key,
            condition: None,
        }
    }

    /// Whether a new image may join a sub-series whose first image is `first`.
    pub fn matches(&self, first: &Attributes, new: &Attributes) -> bool {
        values_equal(
            self.key.value(first).as_deref(),
            self.key.value(new).as_deref(),
            false,
        ) || self.condition.as_ref().is_some_and(|c| !c.holds(new))
    }
}

/// Rules of one modality, for single-frame and multi-frame images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalityRules {
    /// Modality (upper case) or `DEFAULT`.
    pub modality: String,
    /// Rules for single-frame images.
    pub single_frame: Vec<Rule>,
    /// Rules for multi-frame images.
    pub multi_frame: Vec<Rule>,
}

/// Splitting rules of all modalities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplittingRules {
    modalities: Vec<ModalityRules>,
}

impl SplittingRules {
    /// Rules of the Java viewer (DEFAULT, CT, PT and MR).
    // TODO(7.6): add the site layer (`seriesSplittingRules.json`) with the configuration files.
    pub fn built_in() -> &'static Self {
        static RULES: OnceLock<SplittingRules> = OnceLock::new();
        RULES.get_or_init(built_in_rules)
    }

    /// Rules for an image of a series: those of its modality, else the DEFAULT ones.
    pub fn rules_for(&self, modality: Option<&str>, multi_frame: bool) -> &[Rule] {
        let find = |name: &str| self.modalities.iter().find(|m| m.modality == name);
        let Some(rules) = modality.and_then(find).or_else(|| find(DEFAULT_MODALITY)) else {
            return &[];
        };
        if multi_frame {
            &rules.multi_frame
        } else {
            &rules.single_frame
        }
    }

    /// DICOM attributes the rules and their conditions read; they are kept on each instance.
    pub fn tags(&self) -> Vec<Tag> {
        let mut tags = Vec::new();
        let mut add = |key: RuleKey| match key {
            RuleKey::Tag(tag) if !tags.contains(&tag) => tags.push(tag),
            RuleKey::ImagePlane if !tags.contains(&tags::IMAGE_ORIENTATION_PATIENT) => {
                tags.push(tags::IMAGE_ORIENTATION_PATIENT);
            }
            _ => {}
        };
        for modality in &self.modalities {
            for rule in modality.single_frame.iter().chain(&modality.multi_frame) {
                add(rule.key);
                if let Some(condition) = &rule.condition {
                    condition_keys(condition, &mut add);
                }
            }
        }
        tags
    }
}

fn condition_keys(condition: &Condition, add: &mut impl FnMut(RuleKey)) {
    match condition {
        Condition::Test { key, .. } => add(*key),
        Condition::AllOf(children, _) | Condition::AnyOf(children, _) => {
            children.iter().for_each(|c| condition_keys(c, add));
        }
    }
}

fn built_in_rules() -> SplittingRules {
    let tag_rules = |list: &[Tag]| list.iter().map(|&t| Rule::new(RuleKey::Tag(t))).collect();
    let default = ModalityRules {
        modality: DEFAULT_MODALITY.to_owned(),
        single_frame: tag_rules(&[
            tags::IMAGE_TYPE,
            tags::SOP_CLASS_UID,
            tags::CONTRAST_BOLUS_AGENT,
        ]),
        multi_frame: tag_rules(&[
            tags::IMAGE_TYPE,
            tags::SOP_INSTANCE_UID,
            tags::FRAME_TYPE,
            tags::STACK_ID,
        ]),
    };
    // The plane splits the series, except for projection images (scout views).
    let plane = Rule {
        key: RuleKey::ImagePlane,
        condition: Some(Condition::AllOf(
            vec![Condition::Test {
                key: RuleKey::Tag(tags::IMAGE_TYPE),
                contains: true,
                ignore_case: true,
                negate: true,
                expected: "PROJECTION".to_owned(),
            }],
            false,
        )),
    };
    let extend = |modality: &str, tags: &[Tag], extra: Option<&Rule>| {
        let mut rules = default.clone();
        rules.modality = modality.to_owned();
        rules.single_frame.extend(tag_rules(tags));
        rules.single_frame.extend(extra.cloned());
        rules
    };
    let ct = extend(
        "CT",
        &[tags::CONVOLUTION_KERNEL, tags::GANTRY_DETECTOR_TILT],
        Some(&plane),
    );
    let pt = extend(
        "PT",
        &[tags::CONVOLUTION_KERNEL, tags::GANTRY_DETECTOR_TILT],
        None,
    );
    let mr = extend(
        "MR",
        &[
            tags::SCANNING_SEQUENCE,
            tags::SEQUENCE_VARIANT,
            tags::SCAN_OPTIONS,
            tags::REPETITION_TIME,
            tags::ECHO_TIME,
            tags::FLIP_ANGLE,
        ],
        Some(&plane),
    );
    SplittingRules {
        modalities: vec![default, ct, pt, mr],
    }
}

/// Equal when both are missing, or have the same values (as text or as numbers).
fn values_equal(a: Option<&str>, b: Option<&str>, ignore_case: bool) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            let (a, b): (Vec<&str>, Vec<&str>) =
                (split_values(a).collect(), split_values(b).collect());
            a.len() == b.len()
                && a.iter()
                    .zip(&b)
                    .all(|(x, y)| value_equal(x, y, ignore_case))
        }
        _ => false,
    }
}

fn value_equal(a: &str, b: &str, ignore_case: bool) -> bool {
    if a == b || (ignore_case && a.eq_ignore_ascii_case(b)) {
        return true;
    }
    matches!((a.parse::<f64>(), b.parse::<f64>()), (Ok(x), Ok(y)) if x == y)
}

/// Whether one of the values contains the text; a missing value contains nothing.
fn contains_text(value: Option<&str>, text: &str, ignore_case: bool) -> bool {
    let Some(value) = value else {
        return false;
    };
    if ignore_case {
        let text = text.to_lowercase();
        split_values(value).any(|v| v.to_lowercase().contains(&text))
    } else {
        split_values(value).any(|v| v.contains(text))
    }
}

/// The values of a multi-valued attribute, separated by `\` (PS3.5 §6.4).
fn split_values(text: &str) -> impl Iterator<Item = &str> {
    text.split('\\').map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(values: &[(Tag, &str)]) -> Attributes {
        let mut attributes = Attributes::new();
        for (tag, value) in values {
            attributes.set(*tag, *value);
        }
        attributes
    }

    #[test]
    fn modalities_without_rules_use_the_default_ones() {
        let rules = SplittingRules::built_in();
        assert_eq!(rules.rules_for(Some("CR"), false).len(), 3);
        assert_eq!(rules.rules_for(None, true).len(), 4);
        assert_eq!(rules.rules_for(Some("MR"), false).len(), 10);
    }

    #[test]
    fn values_compare_as_text_or_numbers() {
        assert!(values_equal(Some("2.50\\1"), Some("2.5\\1.0"), false));
        assert!(!values_equal(
            Some("ORIGINAL\\PRIMARY"),
            Some("ORIGINAL"),
            false
        ));
        assert!(!values_equal(Some("B30f"), None, false));
        assert!(values_equal(None, None, false));
    }

    #[test]
    fn the_plane_only_splits_images_that_are_not_projections() {
        let ct = SplittingRules::built_in().rules_for(Some("CT"), false);
        let plane = ct
            .iter()
            .find(|r| r.key == RuleKey::ImagePlane)
            .expect("plane rule");
        let axial = attributes(&[(tags::IMAGE_ORIENTATION_PATIENT, "1\\0\\0\\0\\1\\0")]);
        let coronal = attributes(&[(tags::IMAGE_ORIENTATION_PATIENT, "1\\0\\0\\0\\0\\-1")]);
        let mut scout = coronal.clone();
        scout.set(tags::IMAGE_TYPE, "ORIGINAL\\PRIMARY\\LOCALIZER\\projection");

        assert!(plane.matches(&axial, &axial));
        assert!(!plane.matches(&axial, &coronal));
        assert!(plane.matches(&axial, &scout));
    }

    #[test]
    fn the_tags_of_the_rules_are_listed_once() {
        let tags = SplittingRules::built_in().tags();
        assert!(tags.contains(&tags::CONVOLUTION_KERNEL));
        assert!(tags.contains(&tags::IMAGE_ORIENTATION_PATIENT));
        assert_eq!(tags.iter().filter(|&&t| t == tags::IMAGE_TYPE).count(), 1);
    }
}
