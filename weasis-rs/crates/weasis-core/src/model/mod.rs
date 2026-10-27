//! DICOM information model: the Patient → Study → Series → Instance tree.

mod attributes;
mod dicom_sorter;
mod hidden;
mod level;
mod masking;
mod node_label;
mod patient_identity;
mod phases;
mod sort;
mod splitting_rules;
mod sub_series;
mod tree;

pub use attributes::Attributes;
pub use dicom_sorter::{SortingTime, compare_patients, compare_series, compare_studies};
pub use hidden::HiddenObjects;
pub use level::Level;
pub use masking::{
    AnonymizationAction, DeviceKey, IdentityMask, MaskRegion, MaskedTag, MaskedValue, MaskingModel,
    MaskingProfile, PixelMask, Rejection, TagCategory, best_pixel_mask,
};
pub use node_label::{patient_label, series_label, study_label};
pub use patient_identity::{NO_VALUE, PatientIdentity};
pub use sort::{SortKey, slice_position, sort_instances};
pub use splitting_rules::{Condition, ModalityRules, Rule, RuleKey, SplittingRules};
pub use tree::{DicomModel, InstanceNode, NewInstance, PatientNode, SeriesNode, StudyNode};
