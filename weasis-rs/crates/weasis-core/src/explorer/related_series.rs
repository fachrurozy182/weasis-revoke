//! Series sharing the Frame of Reference of a series, optionally with the same orientation.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/main/ThumbnailMouseAndKeyAdapter.java`
//! (`selectRelatedSeries`) and `ImageOrientation.hasSameOrientation`.

use dicom_dictionary_std::tags;
use glam::DVec3;

use crate::geometry::{parse_orientation, same_orientation};
use crate::model::{SeriesNode, StudyNode};

/// Keys of the series of `study` in the frame of reference of `series`, `series` included;
/// empty when it has no Frame of Reference UID.
pub fn related_series(study: &StudyNode, series: &SeriesNode, same_axis: bool) -> Vec<String> {
    let Some(frame) = series.attributes.get(tags::FRAME_OF_REFERENCE_UID) else {
        return Vec::new();
    };
    let axis = orientation(series);
    study
        .series
        .iter()
        .filter(|s| s.attributes.get(tags::FRAME_OF_REFERENCE_UID) == Some(frame))
        .filter(|s| {
            !same_axis
                || matches!((axis, orientation(s)), (Some(a), Some(b)) if same_orientation(a, b))
        })
        .map(|s| s.key.clone())
        .collect()
}

/// Orientation of the middle image.
fn orientation(series: &SeriesNode) -> Option<(DVec3, DVec3)> {
    let middle = series.instances.get(series.instances.len() / 2)?;
    parse_orientation(middle.attributes.get(tags::IMAGE_ORIENTATION_PATIENT)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::ObjectKind;
    use crate::model::{Attributes, InstanceNode};

    fn series(key: &str, frame: Option<&str>, orientation: &str) -> SeriesNode {
        let mut attributes = Attributes::new();
        if let Some(frame) = frame {
            attributes.set(tags::FRAME_OF_REFERENCE_UID, frame);
        }
        let mut instance = Attributes::new();
        instance.set(tags::IMAGE_ORIENTATION_PATIENT, orientation);
        SeriesNode {
            key: key.to_owned(),
            attributes,
            split_number: None,
            instances: vec![InstanceNode {
                sop_instance_uid: format!("{key}.1"),
                kind: ObjectKind::Image,
                attributes: instance,
            }],
        }
    }

    #[test]
    fn series_of_the_same_frame_and_axis() {
        let axial = "1\\0\\0\\0\\1\\0";
        let coronal = "1\\0\\0\\0\\0\\-1";
        let study = StudyNode {
            uid: "1".to_owned(),
            attributes: Attributes::new(),
            series: vec![
                series("ax", Some("F"), axial),
                series("cor", Some("F"), coronal),
                series("other", Some("G"), axial),
                series("none", None, axial),
            ],
        };
        assert_eq!(
            related_series(&study, &study.series[0], false),
            ["ax", "cor"]
        );
        assert_eq!(related_series(&study, &study.series[0], true), ["ax"]);
        assert!(related_series(&study, &study.series[3], false).is_empty());
    }
}
