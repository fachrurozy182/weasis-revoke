//! Attribute values attached to a node of the model tree.
//!
//! Values are kept in their DICOM string form (multiple values separated by `\`, PS3.5 §6.4).
//! Typed reading (dates, numbers) is done by the code that needs it.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/Taggable.java`
//! and `TagReadable.java`.

use std::collections::BTreeMap;

use dicom_object::Tag;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Attribute values of one node, keyed by DICOM tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attributes(BTreeMap<Tag, String>);

impl Attributes {
    /// Creates an empty set of attributes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Value of an attribute, if present.
    pub fn get(&self, tag: Tag) -> Option<&str> {
        self.0.get(&tag).map(String::as_str)
    }

    /// Sets or replaces the value of an attribute.
    pub fn set(&mut self, tag: Tag, value: impl Into<String>) {
        self.0.insert(tag, value.into());
    }

    /// Copies the values of `other` that are not present yet; existing values are kept.
    pub fn fill_missing(&mut self, other: &Attributes) {
        for (tag, value) in &other.0 {
            self.0.entry(*tag).or_insert_with(|| value.clone());
        }
    }

    /// Iterates over the attributes in tag order.
    pub fn iter(&self) -> impl Iterator<Item = (Tag, &str)> {
        self.0.iter().map(|(tag, value)| (*tag, value.as_str()))
    }
}

/// Serialized as a map of `GGGGEEEE` hexadecimal tags to values, e.g. to cross the import worker.
impl Serialize for Attributes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(
            self.0
                .iter()
                .map(|(tag, value)| (format!("{:04X}{:04X}", tag.0, tag.1), value)),
        )
    }
}

impl<'de> Deserialize<'de> for Attributes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, String>::deserialize(deserializer)?;
        let mut attributes = Attributes::new();
        for (key, value) in map {
            let tag = u32::from_str_radix(&key, 16)
                .ok()
                .filter(|_| key.len() == 8)
                .ok_or_else(|| D::Error::custom(format!("invalid tag {key}")))?;
            attributes.set(Tag((tag >> 16) as u16, tag as u16), value);
        }
        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dicom_dictionary_std::tags;

    #[test]
    fn fill_missing_keeps_existing_values() {
        let mut first = Attributes::new();
        first.set(tags::STUDY_DESCRIPTION, "Head");
        let mut second = Attributes::new();
        second.set(tags::STUDY_DESCRIPTION, "Other");
        second.set(tags::STUDY_ID, "7");

        first.fill_missing(&second);

        assert_eq!(first.get(tags::STUDY_DESCRIPTION), Some("Head"));
        assert_eq!(first.get(tags::STUDY_ID), Some("7"));
    }

    #[test]
    fn attributes_round_trip_through_json() {
        let mut attributes = Attributes::new();
        attributes.set(tags::PATIENT_NAME, "Doe^John");
        let json = serde_json::to_string(&attributes).expect("serializable");
        assert_eq!(json, r#"{"00100010":"Doe^John"}"#);
        assert_eq!(
            serde_json::from_str::<Attributes>(&json).ok(),
            Some(attributes)
        );
        assert!(serde_json::from_str::<Attributes>(r#"{"0010":"x"}"#).is_err());
    }
}
