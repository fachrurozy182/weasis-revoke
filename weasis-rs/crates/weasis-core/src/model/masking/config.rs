//! Identity masking configuration: the category of each tag and the masking profiles, merged
//! from JSON documents (built-in, DICOM classification, site, user).
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/MaskingModel.java`,
//! `MaskingModelRegistry.java`, `MaskingProfile.java`, `TagCategory.java` and
//! `AnonymizationAction.java`; documents `identityMasking.json` and `identityMasking-dicom.json`.

use std::collections::HashMap;

use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry};
use dicom_dictionary_std::StandardDataDictionary;
use dicom_object::Tag;
use serde::Deserialize;

use super::pixel_mask::{PixelMask, PixelMaskJson};
use crate::error::{Error, Result};

/// Built-in profiles and Weasis-internal tags.
const BUILT_IN_MASKING_JSON: &str =
    include_str!("../../../../../assets/config/identityMasking.json");
/// Classification of the DICOM tags.
const DICOM_MASKING_JSON: &str =
    include_str!("../../../../../assets/config/identityMasking-dicom.json");

/// What kind of information a tag carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TagCategory {
    /// Identifies the patient directly (name, ID, accession number).
    DirectId,
    /// Sex, age, weight, size.
    PatientCharacteristic,
    /// Dates and times of the exam.
    Date,
    /// Birth date and time.
    BirthDate,
    /// Institution and department.
    Institution,
    /// Station and device serial number.
    Device,
    /// Study, series and protocol descriptions.
    Descriptor,
    /// Free text comments.
    FreeText,
    /// Anything else.
    Other,
}

/// What happens to a value, named after the DICOM PS3.15 Annex E action codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnonymizationAction {
    /// Shown as it is (K).
    Keep,
    /// Replaced by a pseudonym (D).
    Pseudonymize,
    /// Shown empty (Z).
    Clear,
    /// Hidden: the next candidate tag is shown instead (X).
    Remove,
    /// Dates moved by the session offset (C).
    Shift,
}

/// A tag named in a masking document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MaskedTag {
    /// A DICOM attribute.
    Dicom(Tag),
    /// A Weasis-internal value, e.g. `PatientPseudoUID`.
    Internal(String),
}

impl MaskedTag {
    /// Parses a DICOM keyword, a hexadecimal tag (`(0010,0010)`, `00100010`) or
    /// `weasis:<keyword>`.
    ///
    /// NOTE: private tags (with `@creator`) are not supported yet and are ignored.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(keyword) = text.strip_prefix("weasis:") {
            return Some(Self::Internal(keyword.to_owned()));
        }
        let hex: String = text.chars().filter(char::is_ascii_hexdigit).collect();
        let looks_hex = hex.len() == 8
            && text
                .chars()
                .all(|c| c.is_ascii_hexdigit() || "(), ".contains(c));
        if looks_hex {
            let group = u16::from_str_radix(&hex[..4], 16).ok()?;
            let element = u16::from_str_radix(&hex[4..], 16).ok()?;
            return (group % 2 == 0).then_some(Self::Dicom(Tag(group, element)));
        }
        StandardDataDictionary
            .by_name(text)
            .map(|entry| Self::Dicom(entry.tag_range().inner()))
    }
}

/// One use case of masking: an action per category, optional per-tag actions.
#[derive(Debug, Clone, PartialEq)]
pub struct MaskingProfile {
    /// Stable id, e.g. `display`.
    pub id: String,
    /// Displayed name.
    pub name: String,
    /// Names per language code.
    pub labels: HashMap<String, String>,
    /// Action per category; KEEP when absent.
    pub actions: HashMap<TagCategory, AnonymizationAction>,
    /// Actions of single tags, overriding their category.
    pub tag_actions: HashMap<MaskedTag, AnonymizationAction>,
    /// Whether dates are moved by the session offset.
    pub shifts_dates: bool,
    /// Whether the profile is offered when producing an image.
    pub offer_in_export: bool,
}

impl MaskingProfile {
    /// Action for a category.
    pub fn action_for_category(&self, category: TagCategory) -> AnonymizationAction {
        self.actions
            .get(&category)
            .copied()
            .unwrap_or(AnonymizationAction::Keep)
    }

    /// Whether the profile hides direct identifiers; every valid profile does.
    pub fn hides_direct_identifiers(&self) -> bool {
        self.action_for_category(TagCategory::DirectId) != AnonymizationAction::Keep
    }
}

/// The merged masking configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaskingModel {
    categories: HashMap<MaskedTag, TagCategory>,
    profiles: Vec<MaskingProfile>,
    masks: Vec<PixelMask>,
    session_profile: Option<String>,
    ai_profile: Option<String>,
    locked: bool,
}

impl MaskingModel {
    /// The built-in profiles with the DICOM classification.
    pub fn built_in() -> Result<Self> {
        let mut model = Self::default();
        model.add_layer(BUILT_IN_MASKING_JSON, false)?;
        model.add_layer(DICOM_MASKING_JSON, false)?;
        Ok(model)
    }

    /// Merges a later document: same tags and profile ids are replaced. A user document
    /// (`user = true`) is ignored when an earlier document is locked. Invalid entries are
    /// skipped, so a mistake cannot switch masking off.
    pub fn add_layer(&mut self, json: &str, user: bool) -> Result<()> {
        if user && self.locked {
            return Ok(());
        }
        let document: DocumentJson = serde_json::from_str(json)
            .map_err(|e| Error::Configuration(format!("identity masking: {e}")))?;
        if document.schema.unwrap_or(1) > 1 {
            return Err(Error::Configuration(
                "identity masking: newer schema".to_owned(),
            ));
        }
        for rule in document.tags {
            if let Some(tag) = MaskedTag::parse(&rule.tag) {
                self.categories.insert(tag, rule.category);
            }
        }
        for entry in document.profiles {
            let base = entry
                .extends
                .as_deref()
                .and_then(|id| self.profile(id))
                .cloned();
            let profile = entry.into_profile(base);
            // NOTE: like the Java viewer, a profile that would show direct identifiers is refused.
            if !profile.hides_direct_identifiers() {
                continue;
            }
            match self.profiles.iter_mut().find(|p| p.id == profile.id) {
                Some(existing) => *existing = profile,
                None => self.profiles.push(profile),
            }
        }
        // A default naming an unknown profile is refused, the earlier one stays.
        if let Some(id) = document
            .session_profile
            .filter(|id| self.profile(id).is_some())
        {
            self.session_profile = Some(id);
        }
        if let Some(id) = document.ai_profile.filter(|id| self.profile(id).is_some()) {
            self.ai_profile = Some(id);
        }
        // Each entry is read on its own: an invalid one is skipped, not the whole document.
        let masks = document.masks.into_iter().filter_map(|entry| {
            serde_json::from_value::<PixelMaskJson>(entry)
                .ok()?
                .into_mask()
        });
        for mask in masks {
            self.masks.retain(|m| m.id != mask.id);
            self.masks.push(mask);
        }
        self.locked |= document.locked;
        Ok(())
    }

    /// Category of a tag; OTHER when not classified.
    pub fn category(&self, tag: &MaskedTag) -> TagCategory {
        self.categories
            .get(tag)
            .copied()
            .unwrap_or(TagCategory::Other)
    }

    /// A profile by id.
    pub fn profile(&self, id: &str) -> Option<&MaskingProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Profiles in document order.
    pub fn profiles(&self) -> &[MaskingProfile] {
        &self.profiles
    }

    /// The profile of session masking (the first profile when none is set).
    pub fn session_profile(&self) -> Option<&MaskingProfile> {
        self.session_profile
            .as_deref()
            .and_then(|id| self.profile(id))
            .or(self.profiles.first())
    }

    /// The profile of requests sent to an AI service.
    pub fn ai_profile(&self) -> Option<&MaskingProfile> {
        self.ai_profile.as_deref().and_then(|id| self.profile(id))
    }

    /// The device library, later layers last.
    pub fn pixel_masks(&self) -> &[PixelMask] {
        &self.masks
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentJson {
    #[serde(default)]
    schema: Option<u32>,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    tags: Vec<TagRuleJson>,
    #[serde(default)]
    profiles: Vec<ProfileJson>,
    #[serde(default)]
    session_profile: Option<String>,
    #[serde(default)]
    ai_profile: Option<String>,
    #[serde(default)]
    masks: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct TagRuleJson {
    tag: String,
    category: TagCategory,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileJson {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default, rename = "label")]
    labels: HashMap<String, String>,
    #[serde(default)]
    actions: HashMap<TagCategory, AnonymizationAction>,
    #[serde(default)]
    tag_actions: HashMap<String, AnonymizationAction>,
    #[serde(default)]
    shifts_dates: Option<bool>,
    #[serde(default)]
    offer_in_export: Option<bool>,
}

impl ProfileJson {
    /// The profile, starting from `base` when it extends another one.
    fn into_profile(self, base: Option<MaskingProfile>) -> MaskingProfile {
        let mut profile = base.unwrap_or_else(|| MaskingProfile {
            id: self.id.clone(),
            name: self.id.clone(),
            labels: HashMap::new(),
            actions: HashMap::new(),
            tag_actions: HashMap::new(),
            shifts_dates: false,
            offer_in_export: false,
        });
        profile.id = self.id;
        if let Some(name) = self.name {
            profile.name = name;
        }
        profile.labels.extend(self.labels);
        profile.actions.extend(self.actions);
        profile.tag_actions.extend(
            self.tag_actions
                .into_iter()
                .filter_map(|(tag, action)| Some((MaskedTag::parse(&tag)?, action))),
        );
        profile.shifts_dates = self.shifts_dates.unwrap_or(profile.shifts_dates);
        profile.offer_in_export = self.offer_in_export.unwrap_or(profile.offer_in_export);
        profile
    }
}

#[cfg(test)]
mod tests {
    use dicom_dictionary_std::tags;

    use super::*;

    #[test]
    fn built_in_profiles_and_classification_are_read() {
        let model = MaskingModel::built_in().expect("valid");
        let ids: Vec<&str> = model.profiles().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["display", "teaching", "publication", "ai-request"]);
        assert_eq!(
            model.session_profile().map(|p| p.id.as_str()),
            Some("display")
        );
        assert_eq!(
            model.ai_profile().map(|p| p.id.as_str()),
            Some("ai-request")
        );
        assert_eq!(
            model.category(&MaskedTag::Dicom(tags::PATIENT_NAME)),
            TagCategory::DirectId
        );
        assert_eq!(
            model.category(&MaskedTag::Dicom(tags::STUDY_DATE)),
            TagCategory::Date
        );
        assert_eq!(
            model.category(&MaskedTag::Dicom(tags::MODALITY)),
            TagCategory::Other
        );
        assert_eq!(
            model.category(&MaskedTag::Internal("PatientPseudoUID".to_owned())),
            TagCategory::DirectId
        );
    }

    #[test]
    fn tags_are_named_by_keyword_or_number() {
        let name = Some(MaskedTag::Dicom(tags::PATIENT_NAME));
        assert_eq!(MaskedTag::parse("PatientName"), name);
        assert_eq!(MaskedTag::parse("(0010,0010)"), name);
        assert_eq!(MaskedTag::parse("00100010"), name);
        assert_eq!(MaskedTag::parse("(0009,0010)"), None, "private");
        assert_eq!(MaskedTag::parse("NotAKeyword"), None);
    }

    #[test]
    fn site_layers_extend_and_cannot_unmask_identifiers() {
        let mut model = MaskingModel::built_in().expect("valid");
        model
            .add_layer(
                r#"{"locked": true, "sessionProfile": "site.demo", "profiles": [
                    {"id": "site.demo", "extends": "publication", "name": "Demo",
                     "actions": {"DESCRIPTOR": "CLEAR"}},
                    {"id": "display", "actions": {"DIRECT_ID": "KEEP"}}]}"#,
                false,
            )
            .expect("valid");
        let demo = model.session_profile().expect("site profile");
        assert_eq!(demo.id, "site.demo");
        assert_eq!(
            demo.action_for_category(TagCategory::Date),
            AnonymizationAction::Remove
        );
        assert_eq!(
            demo.action_for_category(TagCategory::Descriptor),
            AnonymizationAction::Clear
        );
        let display = model.profile("display").expect("kept");
        assert!(
            display.hides_direct_identifiers(),
            "the unsafe override is refused"
        );

        // The site document is locked: a user document is ignored.
        model
            .add_layer(r#"{"sessionProfile": "teaching"}"#, true)
            .expect("ignored");
        assert_eq!(
            model.session_profile().map(|p| p.id.as_str()),
            Some("site.demo")
        );
    }
}
