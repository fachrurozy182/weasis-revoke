//! Identity of a patient, used to group files of the same patient in the model tree.
//!
//! IHE RAD TF-2 4.16.4.2.2.5.3 (Basic Image Review): only images with exactly the same Patient
//! ID and Patient's Name may be displayed together. The pseudo UID therefore combines the Patient
//! ID, its issuer and the normalized name.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/utils/PatientComparator.java`

/// Value used by the Java viewer when a mandatory attribute is missing (`TagW.NO_VALUE`).
pub const NO_VALUE: &str = "UNKNOWN";

/// Patient attributes that identify a patient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatientIdentity {
    patient_id: String,
    issuer_of_patient_id: String,
    name: String,
}

impl PatientIdentity {
    /// Normalizes the raw attribute values; missing values are given as `None`.
    pub fn new(patient_id: Option<&str>, issuer: Option<&str>, name: Option<&str>) -> Self {
        Self {
            patient_id: patient_id.unwrap_or(NO_VALUE).trim().to_owned(),
            issuer_of_patient_id: issuer.unwrap_or_default().trim().to_owned(),
            name: normalize_person_name(&name.unwrap_or(NO_VALUE).trim().to_uppercase()),
        }
    }

    /// Key of the patient node in the model tree.
    // TODO(7.6): let the user choose the attributes, like the Java preference
    // `patientComparator.buildPatientPseudoUID`.
    pub fn pseudo_uid(&self) -> String {
        // Same concatenation as the Java viewer, without separator, so that keys stay compatible.
        format!(
            "{}{}{}",
            self.patient_id, self.issuer_of_patient_id, self.name
        )
    }
}

/// Removes the empty trailing components (`^`) and component groups (`=`) of a person name,
/// which are not significant (DICOM PS3.5 §6.2.1): `DOE^JANE^^^` and `DOE^JANE` are the same.
fn normalize_person_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for character in name.chars() {
        if character == '=' {
            // Empty components at the end of a component group.
            trim_end_matches_in_place(&mut result, '^');
        }
        result.push(character);
    }
    let trimmed_len = result.trim_end_matches(['^', '=']).len();
    result.truncate(trimmed_len);
    result
}

fn trim_end_matches_in_place(text: &mut String, character: char) {
    let len = text.trim_end_matches(character).len();
    text.truncate(len);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pseudo_uid_concatenates_id_issuer_and_name() {
        let identity = PatientIdentity::new(Some(" 123 "), Some("HOSP"), Some("Doe^Jane"));
        assert_eq!(identity.pseudo_uid(), "123HOSPDOE^JANE");
    }

    #[test]
    fn missing_values_use_the_java_defaults() {
        let identity = PatientIdentity::new(None, None, None);
        assert_eq!(identity.pseudo_uid(), "UNKNOWNUNKNOWN");
    }

    #[test]
    fn name_comparison_ignores_case_and_empty_trailing_components() {
        let a = PatientIdentity::new(Some("1"), None, Some("doe^jane^^^"));
        let b = PatientIdentity::new(Some("1"), None, Some("DOE^JANE"));
        assert_eq!(a, b);
    }

    #[test]
    fn empty_components_before_a_group_separator_are_removed() {
        assert_eq!(
            normalize_person_name("DOE^JANE^^=YAMADA^^="),
            "DOE^JANE=YAMADA"
        );
        assert_eq!(normalize_person_name("=="), "");
    }
}
