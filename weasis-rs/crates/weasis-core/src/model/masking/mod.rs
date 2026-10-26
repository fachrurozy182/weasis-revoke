//! Identity masking: hides patient identity in what is displayed (demos, teaching, screen
//! sharing). It is a display-time substitution, not a de-identification: stored data never
//! changes. See `weasis-core/docs/Identity-Masking.md`.
//!
//! One run uses one salt (random bytes from the caller) for all profiles, so the same patient
//! gets the same pseudonym and the same date offset everywhere.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/IdentityMask.java`
//! and `TagView.resolveText`.

mod config;

use dicom_core::VR;
use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry, VirtualVr};
use dicom_dictionary_std::StandardDataDictionary;
use dicom_object::Tag;
use sha2::{Digest, Sha256};

pub use config::{AnonymizationAction, MaskedTag, MaskingModel, MaskingProfile, TagCategory};

/// What a mask makes of a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaskedValue {
    /// The value to show (unchanged, pseudonymized or shifted).
    Show(String),
    /// Show nothing.
    Clear,
    /// Hide the attribute; a display falls back to its next candidate attribute.
    Remove,
}

/// A profile in force, with the salt of the run.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityMask<'a> {
    model: &'a MaskingModel,
    profile: &'a MaskingProfile,
    salt: Vec<u8>,
    date_shift_days: i64,
}

impl<'a> IdentityMask<'a> {
    /// A mask applying `profile`, with the salt of the run (e.g. 16 random bytes).
    pub fn new(model: &'a MaskingModel, profile: &'a MaskingProfile, salt: &[u8]) -> Self {
        Self {
            model,
            profile,
            salt: salt.to_vec(),
            date_shift_days: if profile.shifts_dates {
                date_shift_days(salt)
            } else {
                0
            },
        }
    }

    /// The profile in force.
    pub fn profile(&self) -> &MaskingProfile {
        self.profile
    }

    /// Action for a tag: its own action in the profile, else that of its category.
    pub fn action_for(&self, tag: &MaskedTag) -> AnonymizationAction {
        self.profile
            .tag_actions
            .get(tag)
            .copied()
            .unwrap_or_else(|| self.profile.action_for_category(self.model.category(tag)))
    }

    /// What to show for a DICOM value (in its DICOM string form).
    pub fn apply(&self, tag: Tag, value: &str) -> MaskedValue {
        let masked = MaskedTag::Dicom(tag);
        match self.action_for(&masked) {
            AnonymizationAction::Remove => MaskedValue::Remove,
            AnonymizationAction::Clear => MaskedValue::Clear,
            AnonymizationAction::Keep => MaskedValue::Show(value.to_owned()),
            AnonymizationAction::Pseudonymize => MaskedValue::Show(self.pseudonym(tag, value)),
            AnonymizationAction::Shift => MaskedValue::Show(self.shift(tag, value)),
        }
    }

    /// First 4 bytes of SHA-256(salt + value) in upper-case hexadecimal; a person name keeps
    /// the PN structure.
    fn pseudonym(&self, tag: Tag, value: &str) -> String {
        if value.trim().is_empty() {
            return value.to_owned();
        }
        let mut digest = Sha256::new();
        digest.update(&self.salt);
        digest.update(value.as_bytes());
        let token: String = digest.finalize()[..4]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect();
        if vr_of(tag) == Some(VR::PN) {
            format!("ANONYMOUS^{token}")
        } else {
            token
        }
    }

    /// Dates (DA, and the date of DT) are moved by the offset; times are unchanged.
    fn shift(&self, tag: Tag, value: &str) -> String {
        match vr_of(tag) {
            Some(VR::DA | VR::DT) => match value
                .get(0..8)
                .and_then(|date| shift_date(date, self.date_shift_days))
            {
                Some(shifted) => format!("{shifted}{}", &value[8..]),
                None => value.to_owned(),
            },
            _ => value.to_owned(),
        }
    }
}

fn vr_of(tag: Tag) -> Option<VR> {
    match StandardDataDictionary
        .by_tag(tag)
        .map(DataDictionaryEntry::vr)
    {
        Some(VirtualVr::Exact(vr)) => Some(vr),
        _ => None,
    }
}

/// 30 to 329 days, from the salt (`Arrays.hashCode` of Java).
fn date_shift_days(salt: &[u8]) -> i64 {
    let hash = salt.iter().fold(1i32, |hash, &byte| {
        hash.wrapping_mul(31).wrapping_add(i32::from(byte as i8))
    });
    30 + i64::from(hash.rem_euclid(300))
}

/// A `YYYYMMDD` date moved by some days.
fn shift_date(date: &str, days: i64) -> Option<String> {
    let year: i64 = date.get(0..4)?.parse().ok()?;
    let month: i64 = date.get(4..6)?.parse().ok()?;
    let day: i64 = date.get(6..8)?.parse().ok()?;
    let (y, m, d) = civil_from_days(days_from_civil(year, month, day) + days);
    Some(format!("{y:04}{m:02}{d:02}"))
}

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use dicom_dictionary_std::tags;

    use super::*;

    fn mask<'a>(model: &'a MaskingModel, id: &str) -> IdentityMask<'a> {
        IdentityMask::new(model, model.profile(id).expect("profile"), b"fixed salt")
    }

    #[test]
    fn the_display_profile_pseudonymizes_identifiers() {
        let model = MaskingModel::built_in().expect("valid");
        let display = mask(&model, "display");
        let MaskedValue::Show(name) = display.apply(tags::PATIENT_NAME, "Doe^John") else {
            panic!("name shown");
        };
        assert!(
            name.starts_with("ANONYMOUS^") && name.len() == "ANONYMOUS^".len() + 8,
            "{name}"
        );
        // Deterministic within a run.
        assert_eq!(
            display.apply(tags::PATIENT_NAME, "Doe^John"),
            MaskedValue::Show(name)
        );
        assert_eq!(
            display.apply(tags::PATIENT_BIRTH_DATE, "19700101"),
            MaskedValue::Remove
        );
        assert_eq!(
            display.apply(tags::MODALITY, "CT"),
            MaskedValue::Show("CT".to_owned())
        );
    }

    #[test]
    fn the_teaching_profile_shifts_dates_only() {
        let model = MaskingModel::built_in().expect("valid");
        let teaching = mask(&model, "teaching");
        let days = date_shift_days(b"fixed salt");
        assert!((30..330).contains(&days));
        let expected = shift_date("20260101", days).expect("date");
        assert_eq!(
            teaching.apply(tags::STUDY_DATE, "20260101"),
            MaskedValue::Show(expected)
        );
        assert_eq!(
            teaching.apply(tags::STUDY_TIME, "1200"),
            MaskedValue::Show("1200".to_owned())
        );
    }

    #[test]
    fn dates_shift_across_months_and_leap_years() {
        assert_eq!(shift_date("20240228", 1).as_deref(), Some("20240229"));
        assert_eq!(shift_date("20231231", 1).as_deref(), Some("20240101"));
        assert_eq!(shift_date("20260101", 365).as_deref(), Some("20270101"));
        assert_eq!(shift_date("bad", 1), None);
    }

    #[test]
    fn the_shift_matches_java_arrays_hash_code() {
        // Arrays.hashCode(new byte[0]) == 1, so the shift is 30 + 1.
        assert_eq!(date_shift_days(&[]), 31);
        // Arrays.hashCode(new byte[]{-1, 2}) == (31 + -1) * 31 + 2 == 932; 932 % 300 == 32.
        assert_eq!(date_shift_days(&[0xFF, 2]), 62);
    }
}
