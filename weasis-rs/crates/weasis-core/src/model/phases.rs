//! Separating the phases of a 4D series (several images at each position, one per phase) into
//! series of their own, and merging back the parts of a split series.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! (`calculateSamplingRateFor4d`, `seriesPostProcessing`) and `DicomModel.java` (`mergeSeries`).

use dicom_dictionary_std::tags;

use super::sub_series::part_key;
use super::{DicomModel, InstanceNode, SeriesNode, SortKey, slice_position, sort_instances};

/// Distance below which two slice positions are the same, in millimeters.
const SAME_POSITION_MM: f64 = 0.05;
/// Largest number of phases separated without being asked.
const MAX_AUTOMATIC_PHASES: u32 = 7;

impl DicomModel {
    /// Number of phases of a series: the images at the position of the first one (by slice
    /// position), when they are more than one and less than half of the series; else 1.
    pub fn phase_count(&self, key: &str) -> u32 {
        self.series(key).map_or(1, |s| phase_count(&by_position(s)))
    }

    /// Moves each phase of a series to a series of its own, the first phase staying in place;
    /// without `force`, only for 2 to 7 phases. Returns whether the series was separated.
    pub fn separate_phases(&mut self, key: &str, force: bool) -> bool {
        let Some(series) = self.series(key) else {
            return false;
        };
        let images = by_position(series);
        let phases = phase_count(&images);
        if phases < 2 || (phases > MAX_AUTOMATIC_PHASES && !force) {
            return false;
        }
        let series_uid = series_uid(series);
        let template = SeriesNode {
            instances: Vec::new(),
            ..series.clone()
        };
        let mut groups: Vec<Vec<InstanceNode>> = vec![Vec::new(); phases as usize];
        for (index, image) in images.into_iter().enumerate() {
            groups[index % phases as usize].push(image);
        }
        let mut groups = groups.into_iter();
        if let Some(original) = self.series_mut(key) {
            original.instances = groups.next().unwrap_or_default();
            original.split_number.get_or_insert(1);
        }
        for instances in groups {
            let part = self.next_part(&series_uid);
            let new = SeriesNode {
                key: part_key(part, &series_uid),
                split_number: Some(part + 1),
                instances,
                ..template.clone()
            };
            if let Some(study) = self.study_of_series_mut(key) {
                study.series.push(new);
            }
        }
        true
    }

    /// Moves the instances of split parts of one series into the part with the smallest split
    /// number and removes the others. Parts of different series are left as they are.
    pub fn merge_series(&mut self, keys: &[String]) -> bool {
        let parts: Vec<&SeriesNode> = keys.iter().filter_map(|k| self.series(k)).collect();
        let Some(first) = parts.first() else {
            return false;
        };
        let uid = series_uid(first);
        if parts.len() < 2 || parts.iter().any(|p| series_uid(p) != uid) {
            return false;
        }
        let base = parts
            .iter()
            .min_by_key(|p| p.split_number.unwrap_or(u32::MAX))
            .map(|p| p.key.clone())
            .unwrap_or_default();
        let mut moved = Vec::new();
        for key in keys.iter().filter(|k| **k != base) {
            if let Some(series) = self.remove_series(key) {
                moved.extend(series.instances);
            }
        }
        if let Some(base) = self.series_mut(&base) {
            base.instances.extend(moved);
            sort_instances(&mut base.instances, SortKey::InstanceNumber);
        }
        true
    }
}

/// Series Instance UID of a series or split part.
fn series_uid(series: &SeriesNode) -> String {
    let uid = series.attributes.get(tags::SERIES_INSTANCE_UID);
    uid.map_or_else(|| series.key.clone(), str::to_owned)
}

fn by_position(series: &SeriesNode) -> Vec<InstanceNode> {
    let mut images = series.instances.clone();
    sort_instances(&mut images, SortKey::SlicePosition);
    images
}

fn phase_count(images: &[InstanceNode]) -> u32 {
    let position = |i: &InstanceNode| slice_position(&i.attributes);
    let Some(first) = images.first().and_then(position) else {
        return 1;
    };
    let same = images
        .iter()
        .take_while(|i| position(i).is_some_and(|p| (p - first).abs() < SAME_POSITION_MM))
        .count();
    if same > 1 && same < images.len() / 2 {
        u32::try_from(same).unwrap_or(1)
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::ObjectKind;
    use crate::model::{Attributes, NewInstance};

    /// A 4D CT: `positions` slices × `phases` images, phase by phase.
    fn ct_4d(positions: usize, phases: usize) -> DicomModel {
        let mut model = DicomModel::new();
        let mut series = Attributes::new();
        series.set(tags::SERIES_INSTANCE_UID, "1.2.3");
        for phase in 0..phases {
            for slice in 0..positions {
                let number = phase * positions + slice + 1;
                let mut instance = Attributes::new();
                instance.set(tags::INSTANCE_NUMBER, number.to_string());
                instance.set(tags::IMAGE_ORIENTATION_PATIENT, "1\\0\\0\\0\\1\\0");
                instance.set(tags::IMAGE_POSITION_PATIENT, format!("0\\0\\{slice}"));
                model.add_instance(NewInstance {
                    patient_key: "P".to_owned(),
                    study_uid: "1.2".to_owned(),
                    series_key: "1.2.3".to_owned(),
                    sop_instance_uid: format!("i{number}"),
                    kind: ObjectKind::Image,
                    series: series.clone(),
                    instance,
                    ..NewInstance::default()
                });
            }
        }
        model
    }

    fn parts(model: &DicomModel) -> Vec<(String, Option<u32>, usize)> {
        let study = model.study("1.2").expect("study");
        study
            .series
            .iter()
            .map(|s| (s.key.clone(), s.split_number, s.instances.len()))
            .collect()
    }

    #[test]
    fn phases_are_separated_then_merged_back() {
        let mut model = ct_4d(10, 3);
        assert_eq!(model.phase_count("1.2.3"), 3);
        assert!(model.separate_phases("1.2.3", false));
        assert_eq!(
            parts(&model),
            [
                ("1.2.3".to_owned(), Some(1), 10),
                ("#1.1.2.3".to_owned(), Some(2), 10),
                ("#2.1.2.3".to_owned(), Some(3), 10)
            ]
        );
        assert_eq!(model.phase_count("1.2.3"), 1, "one image per position now");
        let keys: Vec<String> = parts(&model).into_iter().map(|(k, _, _)| k).collect();
        assert!(model.merge_series(&keys));
        assert_eq!(parts(&model), [("1.2.3".to_owned(), Some(1), 30)]);
    }

    #[test]
    fn many_phases_need_to_be_asked() {
        let mut model = ct_4d(20, 9);
        assert_eq!(model.phase_count("1.2.3"), 9);
        assert!(!model.separate_phases("1.2.3", false));
        assert!(model.separate_phases("1.2.3", true));
        assert_eq!(parts(&model).len(), 9);
        let mut plain = ct_4d(10, 1);
        assert!(!plain.separate_phases("1.2.3", true), "one phase");
    }
}
