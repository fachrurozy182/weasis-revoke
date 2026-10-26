//! DICOM information model: the Patient → Study → Series → Instance tree.

mod attributes;
mod level;
mod patient_identity;
mod plane;
mod sort;
mod splitting_rules;
mod sub_series;
mod tree;

pub use attributes::Attributes;
pub use level::Level;
pub use patient_identity::{NO_VALUE, PatientIdentity};
pub use plane::ImagePlane;
pub use sort::{SortKey, slice_position, sort_instances};
pub use splitting_rules::{Condition, ModalityRules, Rule, RuleKey, SplittingRules};
pub use tree::{DicomModel, InstanceNode, NewInstance, PatientNode, SeriesNode, StudyNode};
