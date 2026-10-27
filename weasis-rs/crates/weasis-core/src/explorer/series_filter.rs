//! Filter of the series shown in the explorer, acting on one dimension at a time: text in the
//! series and study attributes, one study, or a set of modalities.
//!
//! NOTE: the text is matched against the name of the body part and of its region groups ("chest"
//! finds a lung CT), where the desktop application uses the translated meaning of the code; the
//! anatomy is the Body Part Examined of the series, its fallback when no image is loaded.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/SeriesFilter.java`.

use dicom_dictionary_std::tags;
use serde::{Deserialize, Serialize};

use crate::dicom::anatomy::{AnatomicRegion, RegionGroup};
use crate::model::{Attributes, SeriesNode, StudyNode};

/// Series attributes searched by the text filter, with the Series Number.
const SERIES_TEXT_TAGS: [dicom_object::Tag; 5] = [
    tags::SERIES_DESCRIPTION,
    tags::MODALITY,
    tags::BODY_PART_EXAMINED,
    tags::PROTOCOL_NAME,
    tags::SERIES_INSTANCE_UID,
];
/// Study attributes searched by the text filter.
const STUDY_TEXT_TAGS: [dicom_object::Tag; 4] = [
    tags::STUDY_DESCRIPTION,
    tags::STUDY_ID,
    tags::ACCESSION_NUMBER,
    tags::STUDY_INSTANCE_UID,
];

/// The dimension the filter acts on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterMode {
    /// Text in the series and study attributes.
    #[default]
    Text,
    /// One study.
    Date,
    /// A set of modalities.
    Modality,
}

impl FilterMode {
    /// The three modes, in menu order.
    pub const ALL: [Self; 3] = [Self::Text, Self::Date, Self::Modality];
}

/// The filter; only the criterion of its mode applies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeriesFilter {
    mode: FilterMode,
    /// Lower-case text.
    text: String,
    /// Study Instance UID.
    study: Option<String>,
    modalities: Vec<String>,
}

impl SeriesFilter {
    /// An empty filter of a mode.
    pub fn new(mode: FilterMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    /// The active dimension.
    pub fn mode(&self) -> FilterMode {
        self.mode
    }

    /// Switches the dimension, clearing the criteria (modes are exclusive).
    pub fn set_mode(&mut self, mode: FilterMode) {
        *self = Self::new(mode);
    }

    /// Whether the active mode has a criterion.
    pub fn is_active(&self) -> bool {
        match self.mode {
            FilterMode::Text => !self.text.is_empty(),
            FilterMode::Date => self.study.is_some(),
            FilterMode::Modality => !self.modalities.is_empty(),
        }
    }

    /// The searched text, lower case.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets the searched text.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.trim().to_lowercase();
    }

    /// The study shown, `None` for all.
    pub fn study(&self) -> Option<&str> {
        self.study.as_deref()
    }

    /// Shows one study, or all with `None`.
    pub fn set_study(&mut self, study: Option<String>) {
        self.study = study;
    }

    /// The modalities shown.
    pub fn modalities(&self) -> &[String] {
        &self.modalities
    }

    /// Sets the modalities from a text such as "CT, PT" (commas or spaces, any case).
    pub fn set_modalities(&mut self, text: &str) {
        self.modalities.clear();
        for token in text.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
            let modality = token.to_uppercase();
            if !self.modalities.contains(&modality) {
                self.modalities.push(modality);
            }
        }
    }

    /// Whether a series of `study` passes the filter.
    pub fn test(&self, series: &SeriesNode, study: &StudyNode) -> bool {
        match self.mode {
            FilterMode::Text => self.text.is_empty() || self.matches_text(series, study),
            FilterMode::Date => self.study.as_deref().is_none_or(|uid| uid == study.uid),
            FilterMode::Modality => {
                let modality = series.attributes.get(tags::MODALITY).unwrap_or_default();
                self.modalities.is_empty() || self.modalities.iter().any(|m| m == modality)
            }
        }
    }

    fn matches_text(&self, series: &SeriesNode, study: &StudyNode) -> bool {
        let any = |attributes: &Attributes, tags: &[dicom_object::Tag]| {
            tags.iter().any(|&tag| self.contains(attributes.get(tag)))
        };
        any(&series.attributes, &SERIES_TEXT_TAGS)
            || self.contains(series.attributes.get(tags::SERIES_NUMBER).map(str::trim))
            || self.matches_anatomy(series)
            || any(&study.attributes, &STUDY_TEXT_TAGS)
    }

    /// The body part, or a region group it lies in.
    fn matches_anatomy(&self, series: &SeriesNode) -> bool {
        let Some(term) = series.attributes.get(tags::BODY_PART_EXAMINED) else {
            return false;
        };
        let anatomy = AnatomicRegion::from_term(term);
        let part = anatomy.body_part.map(|p| p.name.replace('_', " "));
        self.contains(part.as_deref())
            || RegionGroup::NAMES.iter().any(|(group, name)| {
                anatomy.is_in(*group) && self.contains(Some(&name.replace('_', " ")))
            })
    }

    fn contains(&self, value: Option<&str>) -> bool {
        value.is_some_and(|v| v.to_lowercase().contains(&self.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(values: &[(dicom_object::Tag, &str)]) -> Attributes {
        let mut attributes = Attributes::new();
        for (tag, value) in values {
            attributes.set(*tag, *value);
        }
        attributes
    }

    fn series(values: &[(dicom_object::Tag, &str)]) -> SeriesNode {
        SeriesNode {
            key: String::new(),
            attributes: attributes(values),
            split_number: None,
            instances: Vec::new(),
        }
    }

    fn study(uid: &str, description: &str) -> StudyNode {
        StudyNode {
            uid: uid.to_owned(),
            attributes: attributes(&[(tags::STUDY_DESCRIPTION, description)]),
            series: Vec::new(),
        }
    }

    #[test]
    fn text_searches_series_study_and_anatomy() {
        let lung = series(&[
            (tags::SERIES_NUMBER, " 12"),
            (tags::BODY_PART_EXAMINED, "LUNG"),
            (tags::MODALITY, "CT"),
        ]);
        let head = study("1", "Head trauma");
        let mut filter = SeriesFilter::default();
        assert!(filter.test(&lung, &head), "no text, all pass");
        for text in ["ct", "12", "TRAUMA", "chest", "lung"] {
            filter.set_text(text);
            assert!(filter.test(&lung, &head), "{text}");
        }
        filter.set_text("pelvis");
        assert!(!filter.test(&lung, &head));
        assert!(filter.is_active());
    }

    #[test]
    fn date_and_modality_modes() {
        let ct = series(&[(tags::MODALITY, "CT")]);
        let mut filter = SeriesFilter::new(FilterMode::Date);
        filter.set_study(Some("2".to_owned()));
        assert!(!filter.test(&ct, &study("1", "")));
        assert!(filter.test(&ct, &study("2", "")));
        filter.set_mode(FilterMode::Modality);
        assert!(!filter.is_active(), "switching mode clears");
        filter.set_modalities("pt, ct ct");
        assert_eq!(filter.modalities(), ["PT", "CT"]);
        assert!(filter.test(&ct, &study("1", "")));
        filter.set_modalities("MR");
        assert!(!filter.test(&ct, &study("1", "")));
    }
}
