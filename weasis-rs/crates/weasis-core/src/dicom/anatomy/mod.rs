//! Anatomy of an image: its body part and the region groups it belongs to.
//!
//! The anatomy comes from the Anatomic Region Sequence (a coded concept) or, failing that,
//! from Body Part Examined (a defined term of PS3.16 Table L-1). Region groups (head, chest, ...)
//! come from `assets/config/regionGroups.json`, the same file as the Java viewer.
//!
//! NOTE: only the body parts of CID 4031 are known; the surface and other parts of the Java
//! model (used by the Dicomizer for dermatology and dentistry) are ported in step 6.11.
//!
//! Java reference: `weasis-dicom-tools/.../org/weasis/dicom/ref/` (`AnatomicRegion`,
//! `AnatomicBuilder`, `AnatomySelector`, `RegionGroup`, `RegionGroups`).

mod body_parts;

use std::collections::HashMap;
use std::sync::OnceLock;

use dicom_dictionary_std::tags;

use super::{DicomFile, first_item, item_text};

/// A body part of DICOM PS3.16 CID 4031.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyPart {
    /// Name of the Java enum constant, used by `regionGroups.json`.
    pub name: &'static str,
    /// Coding scheme designator (`SCT` or `DCM`).
    pub scheme: &'static str,
    /// Code value.
    pub code: &'static str,
    /// Body Part Examined defined term, empty when there is none.
    pub legacy_code: &'static str,
    /// Whether the part has a left and a right side.
    pub paired: bool,
}

/// Coarse regions used to select presets and hanging protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionGroup {
    /// Head.
    Head,
    /// Neck.
    Neck,
    /// Chest.
    Chest,
    /// Breast.
    Breast,
    /// Abdomen.
    Abdomen,
    /// Pelvis.
    Pelvis,
    /// A limb, whichever it is.
    Extremity,
    /// Upper limb.
    UpperExtremity,
    /// Lower limb.
    LowerExtremity,
    /// Whole body: lies in every group.
    WholeBody,
    /// Spine.
    Spine,
}

impl RegionGroup {
    const ALL: [(Self, &'static str); 11] = [
        (Self::Head, "HEAD"),
        (Self::Neck, "NECK"),
        (Self::Chest, "CHEST"),
        (Self::Breast, "BREAST"),
        (Self::Abdomen, "ABDOMEN"),
        (Self::Pelvis, "PELVIS"),
        (Self::Extremity, "EXTREMITY"),
        (Self::UpperExtremity, "UPPER_EXTREMITY"),
        (Self::LowerExtremity, "LOWER_EXTREMITY"),
        (Self::WholeBody, "WHOLE_BODY"),
        (Self::Spine, "SPINE"),
    ];

    /// Group of a name such as `CHEST` (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_uppercase();
        Self::ALL
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(group, _)| *group)
    }

    /// Whether an image of the `imaged` group lies in this group.
    pub fn includes(self, imaged: Self) -> bool {
        self == imaged
            || imaged == Self::WholeBody
            || (self == Self::Extremity
                && matches!(imaged, Self::UpperExtremity | Self::LowerExtremity))
    }
}

/// The anatomy of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnatomicRegion {
    /// Coding scheme of the concept, e.g. `SCT`; empty for an unknown Body Part Examined term.
    pub scheme: String,
    /// Code value, or the Body Part Examined term when it is not a known body part.
    pub code: String,
    /// The known body part, if any.
    pub body_part: Option<&'static BodyPart>,
}

impl AnatomicRegion {
    /// Reads the anatomy of a file: Anatomic Region Sequence first, then Body Part Examined.
    pub fn from_file(file: &DicomFile) -> Option<Self> {
        Self::from_region_sequence(file).or_else(|| Self::from_body_part_examined(file))
    }

    fn from_region_sequence(file: &DicomFile) -> Option<Self> {
        let item = first_item(file.object(), tags::ANATOMIC_REGION_SEQUENCE)?;
        let code = item_text(item, tags::CODE_VALUE)?;
        let scheme = item_text(item, tags::CODING_SCHEME_DESIGNATOR).unwrap_or_default();
        let body_part = if scheme == "SRT" {
            retired_snomed_rt(&code)
        } else {
            body_part_by_code(&scheme, &code)
        };
        Some(match body_part {
            Some(part) => Self::of_body_part(part),
            None => Self {
                scheme,
                code,
                body_part: None,
            },
        })
    }

    fn from_body_part_examined(file: &DicomFile) -> Option<Self> {
        let term = file.text(tags::BODY_PART_EXAMINED)?;
        // NOTE: Body Part Examined is CS (upper case, PS3.5 §6.2); lower-case values written by
        // some modalities are matched too, as in the Java viewer.
        let body_part = body_part_by_legacy_code(&term)
            .or_else(|| body_part_by_legacy_code(&term.to_uppercase()));
        Some(match body_part {
            Some(part) => Self::of_body_part(part),
            None => Self {
                scheme: String::new(),
                code: term,
                body_part: None,
            },
        })
    }

    fn of_body_part(part: &'static BodyPart) -> Self {
        Self {
            scheme: part.scheme.to_owned(),
            code: part.code.to_owned(),
            body_part: Some(part),
        }
    }

    /// Region groups that contain this anatomy.
    pub fn groups(&self) -> &'static [RegionGroup] {
        self.body_part
            .and_then(|part| region_groups().get(part.name))
            .map_or(&[], Vec::as_slice)
    }

    /// Whether this anatomy lies in `group`.
    pub fn is_in(&self, group: RegionGroup) -> bool {
        self.groups().iter().any(|&imaged| group.includes(imaged))
    }

    /// Body Part Examined term of the anatomy (or the unknown term itself).
    pub fn legacy_code(&self) -> &str {
        match self.body_part {
            Some(part) => part.legacy_code,
            None if self.scheme.is_empty() => &self.code,
            None => "",
        }
    }
}

/// A token selecting anatomies: a region group (`CHEST`), a coded concept (`SCT:10200004`) or a
/// Body Part Examined term (`BRAIN`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnatomySelector {
    /// All anatomies of a group.
    Group(RegionGroup),
    /// One coded concept.
    Code {
        /// Coding scheme designator.
        scheme: String,
        /// Code value.
        code: String,
    },
}

impl AnatomySelector {
    /// Parses a token; `None` when it names no known group, scheme or term.
    pub fn parse(token: &str) -> Option<Self> {
        let text = token.trim();
        if let Some((scheme, code)) = text.split_once(':') {
            let (scheme, code) = (scheme.trim(), code.trim());
            if scheme.is_empty() || code.is_empty() {
                return None;
            }
            if scheme.eq_ignore_ascii_case("SRT") {
                return retired_snomed_rt(code).map(Self::of_body_part);
            }
            return Some(Self::Code {
                scheme: scheme.to_uppercase(),
                code: code.to_owned(),
            });
        }
        if let Some(group) = RegionGroup::from_name(text) {
            return Some(Self::Group(group));
        }
        body_part_by_legacy_code(&text.to_uppercase()).map(Self::of_body_part)
    }

    fn of_body_part(part: &'static BodyPart) -> Self {
        Self::Code {
            scheme: part.scheme.to_owned(),
            code: part.code.to_owned(),
        }
    }

    /// Whether `region` is selected.
    pub fn matches(&self, region: &AnatomicRegion) -> bool {
        match self {
            Self::Group(group) => region.is_in(*group),
            Self::Code { scheme, code } => region.scheme == *scheme && region.code == *code,
        }
    }
}

/// Whether one of `tokens` selects `region`. A token that is no group, code or known term is
/// compared with the Body Part Examined term of the image.
pub fn selects(tokens: &[String], region: Option<&AnatomicRegion>) -> bool {
    let Some(region) = region else {
        return false;
    };
    tokens
        .iter()
        .any(|token| match AnatomySelector::parse(token) {
            Some(selector) => selector.matches(region),
            None => token.trim().eq_ignore_ascii_case(region.legacy_code()),
        })
}

fn body_part_by_code(scheme: &str, code: &str) -> Option<&'static BodyPart> {
    body_parts::BODY_PARTS
        .iter()
        .find(|part| part.scheme == scheme && part.code == code)
}

fn body_part_by_legacy_code(term: &str) -> Option<&'static BodyPart> {
    let term = term.trim();
    if term.is_empty() {
        return None;
    }
    body_parts::BODY_PARTS
        .iter()
        .find(|part| part.legacy_code == term)
}

fn retired_snomed_rt(code: &str) -> Option<&'static BodyPart> {
    let name = body_parts::RETIRED_SNOMED_RT
        .iter()
        .find(|(srt, _)| *srt == code.trim())?
        .1;
    body_parts::BODY_PARTS.iter().find(|part| part.name == name)
}

/// Body part name → groups, from `regionGroups.json` (parsed once).
fn region_groups() -> &'static HashMap<String, Vec<RegionGroup>> {
    static GROUPS: OnceLock<HashMap<String, Vec<RegionGroup>>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Document {
            groups: HashMap<String, Vec<String>>,
        }
        let json = include_str!("../../../../../assets/config/regionGroups.json");
        // The file is part of the build and covered by tests: a parse error is a build defect.
        let document: Document = serde_json::from_str(json).unwrap_or(Document {
            groups: HashMap::new(),
        });
        let mut by_part: HashMap<String, Vec<RegionGroup>> = HashMap::new();
        for (group_name, parts) in document.groups {
            let Some(group) = RegionGroup::from_name(&group_name) else {
                continue;
            };
            for part in parts {
                by_part.entry(part).or_default().push(group);
            }
        }
        by_part
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(term: &str) -> AnatomicRegion {
        let part = body_part_by_legacy_code(term).expect("known term");
        AnatomicRegion::of_body_part(part)
    }

    #[test]
    fn region_groups_file_is_read() {
        assert!(region("BRAIN").is_in(RegionGroup::Head));
        assert!(!region("BRAIN").is_in(RegionGroup::Chest));
        assert!(region("LUNG").is_in(RegionGroup::Chest));
    }

    #[test]
    fn extremity_includes_upper_and_lower_limbs() {
        assert!(region("HAND").is_in(RegionGroup::Extremity));
        assert!(region("WHOLEBODY").is_in(RegionGroup::Abdomen));
    }

    #[test]
    fn selectors_accept_groups_codes_and_terms() {
        let chest = region("CHEST");
        assert!(
            AnatomySelector::parse("chest")
                .expect("group")
                .matches(&chest)
        );
        assert!(
            AnatomySelector::parse("SCT:816094009")
                .expect("code")
                .matches(&chest)
        );
        assert!(
            AnatomySelector::parse("CHEST")
                .expect("term")
                .matches(&chest)
        );
        assert!(
            AnatomySelector::parse("SRT:T-28000")
                .expect("retired")
                .matches(&region("LUNG"))
        );
        assert_eq!(AnatomySelector::parse("NOT-A-PART"), None);
    }

    #[test]
    fn unknown_tokens_compare_with_body_part_examined() {
        let unknown = AnatomicRegion {
            scheme: String::new(),
            code: "KNEECAP".to_owned(),
            body_part: None,
        };
        assert!(selects(&["kneecap".to_owned()], Some(&unknown)));
        assert!(!selects(&["HEAD".to_owned()], None));
    }
}
