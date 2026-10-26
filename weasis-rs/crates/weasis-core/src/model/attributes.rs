//! Attribute values attached to a node of the model tree.
//!
//! Values are kept in their DICOM string form (multiple values separated by `\`, PS3.5 §6.4).
//! Typed reading (dates, numbers) is done by the code that needs it.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/Taggable.java`
//! and `TagReadable.java`.

use std::collections::BTreeMap;

use dicom_object::Tag;

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
}
