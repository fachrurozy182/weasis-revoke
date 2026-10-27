//! Placing a new instance in its series or in a sub-series split off by the splitting rules.
//!
//! The original series keeps the Series Instance UID as key; the parts split off are keyed
//! `#<n>.<Series Instance UID>` with n = 1, 2, ... A new image goes to the first of these
//! series whose first image it resembles (see [`SplittingRules`]), else to a new part.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomModel.java`
//! (`applySplittingRules`, `handleRegularSeries`, `findSimilarOrSplit`, `isSimilar`,
//! `createSplitOrReplaceSeries`, `handleRegularSpecialElement`).

use dicom_dictionary_std::tags;

use super::{DicomModel, NewInstance, SeriesNode, SplittingRules};
use crate::dicom::ObjectKind;

impl DicomModel {
    /// Adds an instance to the sub-series chosen by the splitting rules. `new.series_key` must
    /// be the Series Instance UID. Returns `false` for a duplicate instance.
    pub fn add_split_instance(&mut self, mut new: NewInstance, rules: &SplittingRules) -> bool {
        let series_uid = new.series_key.clone();
        let part = self.choose_part(&new, &series_uid, rules);
        if part > 0 {
            new.series_key = part_key(part, &series_uid);
        }
        let added = self.add_instance(new);
        if part > 0 {
            if let Some(original) = self.series_mut(&series_uid) {
                original.split_number.get_or_insert(1);
            }
            if let Some(series) = self.series_mut(&part_key(part, &series_uid)) {
                series.split_number = Some(part + 1);
            }
        }
        added
    }

    /// 0 for the original series, else the number of the part.
    fn choose_part(&self, new: &NewInstance, series_uid: &str, rules: &SplittingRules) -> u32 {
        let Some(original) = self.series(series_uid).filter(|s| !s.instances.is_empty()) else {
            return 0;
        };
        match new.kind {
            // One object per series, so each can be opened on its own.
            ObjectKind::Video
            | ObjectKind::EncapsulatedDocument
            | ObjectKind::StructuredReport
            | ObjectKind::Waveform => self.next_part(series_uid),
            ObjectKind::Image => {
                let multi_frame = number_of_frames(new) > 1;
                let modality = original.attributes.get(tags::MODALITY);
                let rules = rules.rules_for(modality, multi_frame);
                if is_similar(original, new, rules) {
                    return 0;
                }
                (1..)
                    .map_while(|part| self.series(&part_key(part, series_uid)).map(|s| (part, s)))
                    .find(|(_, series)| is_similar(series, new, rules))
                    .map_or_else(|| self.next_part(series_uid), |(part, _)| part)
            }
            // Objects shown through the images they reference stay together.
            _ => 0,
        }
    }

    pub(super) fn next_part(&self, series_uid: &str) -> u32 {
        (1..)
            .find(|&part| self.series(&part_key(part, series_uid)).is_none())
            .unwrap_or(1)
    }
}

pub(super) fn part_key(part: u32, series_uid: &str) -> String {
    format!("#{part}.{series_uid}")
}

/// Whether a new image may join a series, compared with the first instance of the series.
fn is_similar(series: &SeriesNode, new: &NewInstance, rules: &[super::Rule]) -> bool {
    let Some(first) = series.instances.first() else {
        return true;
    };
    let concatenation = first.attributes.get(tags::CONCATENATION_UID);
    if concatenation.is_some() && concatenation == new.instance.get(tags::CONCATENATION_UID) {
        return true;
    }
    first.kind == new.kind
        && rules
            .iter()
            .all(|rule| rule.matches(&first.attributes, &new.instance))
}

fn number_of_frames(new: &NewInstance) -> u32 {
    new.instance
        .get(tags::NUMBER_OF_FRAMES)
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Attributes;

    fn image(sop: &str, values: &[(dicom_object::Tag, &str)]) -> NewInstance {
        let mut instance = Attributes::new();
        for (tag, value) in values {
            instance.set(*tag, *value);
        }
        let mut series = Attributes::new();
        series.set(tags::MODALITY, "CT");
        NewInstance {
            patient_key: "P".to_owned(),
            study_uid: "1.2".to_owned(),
            series_key: "1.2.3".to_owned(),
            sop_instance_uid: sop.to_owned(),
            series,
            instance,
            ..NewInstance::default()
        }
    }

    fn keys(model: &DicomModel) -> Vec<(String, Option<u32>, usize)> {
        model
            .study("1.2")
            .expect("study")
            .series
            .iter()
            .map(|s| (s.key.clone(), s.split_number, s.instances.len()))
            .collect()
    }

    #[test]
    fn images_with_another_kernel_go_to_a_part_shared_by_their_likes() {
        let rules = SplittingRules::built_in();
        let mut model = DicomModel::new();
        for (sop, kernel) in [("1", "B30f"), ("2", "B70f"), ("3", "B30f"), ("4", "B70f")] {
            model.add_split_instance(image(sop, &[(tags::CONVOLUTION_KERNEL, kernel)]), rules);
        }
        assert_eq!(
            keys(&model),
            [
                ("1.2.3".to_owned(), Some(1), 2),
                ("#1.1.2.3".to_owned(), Some(2), 2)
            ]
        );
    }

    #[test]
    fn a_concatenation_stays_in_one_series() {
        let rules = SplittingRules::built_in();
        let mut model = DicomModel::new();
        for sop in ["a", "b"] {
            let values = [
                (tags::NUMBER_OF_FRAMES, "10"),
                (tags::SOP_INSTANCE_UID, sop),
                (tags::CONCATENATION_UID, "9.9"),
            ];
            model.add_split_instance(image(sop, &values), rules);
        }
        assert_eq!(keys(&model), [("1.2.3".to_owned(), None, 2)]);
    }

    #[test]
    fn each_structured_report_gets_its_own_series() {
        let rules = SplittingRules::built_in();
        let mut model = DicomModel::new();
        for sop in ["a", "b", "c"] {
            let report = NewInstance {
                kind: ObjectKind::StructuredReport,
                ..image(sop, &[])
            };
            model.add_split_instance(report, rules);
        }
        let parts: Vec<_> = keys(&model).into_iter().map(|(k, n, _)| (k, n)).collect();
        assert_eq!(
            parts,
            [
                ("1.2.3".to_owned(), Some(1)),
                ("#1.1.2.3".to_owned(), Some(2)),
                ("#2.1.2.3".to_owned(), Some(3))
            ]
        );
    }
}
