//! Order of the patients, studies and series in the explorer.
//!
//! NOTE: names and descriptions are compared ignoring case rather than with the collator of the
//! user's language used by the desktop application.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/DicomSorter.java`.

use std::cmp::Ordering;

use dicom_dictionary_std::tags;
use dicom_object::Tag;
use serde::{Deserialize, Serialize};

use super::{Attributes, PatientNode, SeriesNode, StudyNode};

const ENCAPSULATED_SOP_CLASS_PREFIX: &str = "1.2.840.10008.5.1.4.1.1.88";
const DOCUMENT_SOP_CLASS_PREFIX: &str = "1.2.840.10008.5.1.4.1.1.104";

/// Order of the studies by date.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortingTime {
    /// Oldest first.
    Chronological,
    /// Newest first (default).
    #[default]
    InverseChronological,
}

/// Patients by name, then those without name.
pub fn compare_patients(a: &PatientNode, b: &PatientNode) -> Ordering {
    compare_text(&a.attributes, &b.attributes, tags::PATIENT_NAME)
}

/// Studies by date and time, then ID, then description; missing values last.
pub fn compare_studies(a: &StudyNode, b: &StudyNode, order: SortingTime) -> Ordering {
    let date = |s: &StudyNode| date_time(&s.attributes, tags::STUDY_DATE, tags::STUDY_TIME);
    let by_date = match (date(a), date(b)) {
        (Some(x), Some(y)) if order == SortingTime::Chronological => x.cmp(&y),
        (Some(x), Some(y)) => y.cmp(&x),
        (x, y) => missing_last(x.is_some(), y.is_some()),
    };
    by_date
        .then_with(|| compare_text(&a.attributes, &b.attributes, tags::STUDY_ID))
        .then_with(|| compare_text(&a.attributes, &b.attributes, tags::STUDY_DESCRIPTION))
}

/// Series by number, split part, date and time, description, then their first instance;
/// reports and documents last.
pub fn compare_series(a: &SeriesNode, b: &SeriesNode) -> Ordering {
    is_document(a)
        .cmp(&is_document(b))
        .then_with(|| series_number(a).cmp(&series_number(b)))
        .then_with(|| {
            a.split_number
                .unwrap_or(0)
                .cmp(&b.split_number.unwrap_or(0))
        })
        .then_with(|| series_date_time(a).cmp(&series_date_time(b)))
        .then_with(|| description(a).cmp(&description(b)))
        .then_with(|| compare_first_instances(a, b))
}

fn series_number(series: &SeriesNode) -> i64 {
    number(&series.attributes, tags::SERIES_NUMBER).unwrap_or(i64::MAX)
}

/// Date and time, `None` sorting last as `LocalDateTime.MAX` does.
fn series_date_time(series: &SeriesNode) -> (bool, String) {
    let value = date_time(&series.attributes, tags::SERIES_DATE, tags::SERIES_TIME);
    (value.is_none(), value.unwrap_or_default())
}

fn description(series: &SeriesNode) -> String {
    let value = series
        .attributes
        .get(tags::SERIES_DESCRIPTION)
        .unwrap_or_default();
    value.to_lowercase()
}

/// Structured reports and encapsulated documents go after the images.
fn is_document(series: &SeriesNode) -> bool {
    let sop_class = series
        .attributes
        .get(tags::SOP_CLASS_UID)
        .unwrap_or_default();
    let modality = series.attributes.get(tags::MODALITY).unwrap_or_default();
    sop_class.starts_with(ENCAPSULATED_SOP_CLASS_PREFIX)
        || sop_class.starts_with(DOCUMENT_SOP_CLASS_PREFIX)
        || matches!(modality, "SR" | "DOC")
}

/// Acquisition, content or secondary capture date and time, slice location, Stack ID.
fn compare_first_instances(a: &SeriesNode, b: &SeriesNode) -> Ordering {
    let (Some(a), Some(b)) = (a.instances.first(), b.instances.first()) else {
        return missing_last(!a.instances.is_empty(), !b.instances.is_empty());
    };
    let (a, b) = (&a.attributes, &b.attributes);
    let media_time = |at: &Attributes| {
        let value = [
            (tags::ACQUISITION_DATE, tags::ACQUISITION_TIME),
            (tags::CONTENT_DATE, tags::CONTENT_TIME),
            (
                tags::DATE_OF_SECONDARY_CAPTURE,
                tags::TIME_OF_SECONDARY_CAPTURE,
            ),
        ]
        .into_iter()
        .find_map(|(date, time)| date_time(at, date, time));
        (value.is_none(), value.unwrap_or_default())
    };
    let location = |at: &Attributes| decimal(at, tags::SLICE_LOCATION).unwrap_or(f64::MAX);
    media_time(a)
        .cmp(&media_time(b))
        .then_with(|| location(a).total_cmp(&location(b)))
        .then_with(|| stack_id(a).cmp(&stack_id(b)))
}

/// Numeric Stack IDs compare as numbers; a missing one sorts last.
fn stack_id(attributes: &Attributes) -> String {
    match attributes.get(tags::STACK_ID).map(str::trim) {
        None => "zzz".to_owned(),
        Some(id) => id
            .parse::<i32>()
            .map_or_else(|_| id.to_owned(), |n| format!("{n:010}")),
    }
}

/// Text of a tag ignoring case, missing values last.
fn compare_text(a: &Attributes, b: &Attributes, tag: Tag) -> Ordering {
    match (a.get(tag), b.get(tag)) {
        (Some(x), Some(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
        (x, y) => missing_last(x.is_some(), y.is_some()),
    }
}

fn missing_last(a_present: bool, b_present: bool) -> Ordering {
    b_present.cmp(&a_present)
}

fn number(attributes: &Attributes, tag: Tag) -> Option<i64> {
    attributes.get(tag)?.trim().parse().ok()
}

fn decimal(attributes: &Attributes, tag: Tag) -> Option<f64> {
    attributes.get(tag)?.split('\\').next()?.trim().parse().ok()
}

/// Date (DA) and time (TM) as "YYYYMMDDHHMMSSffffff", which sorts like the moment it denotes;
/// the time is optional (midnight). PS3.5 Table 6.2-1.
fn date_time(attributes: &Attributes, date: Tag, time: Tag) -> Option<String> {
    let digits = |text: &str| {
        text.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
    };
    let date = digits(attributes.get(date)?);
    if date.len() != 8 {
        return None;
    }
    let mut time = digits(attributes.get(time).unwrap_or_default());
    time.truncate(12);
    Some(format!("{date}{time:0<12}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(values: &[(Tag, &str)]) -> Attributes {
        let mut attributes = Attributes::new();
        for (tag, value) in values {
            attributes.set(*tag, *value);
        }
        attributes
    }

    fn study(values: &[(Tag, &str)]) -> StudyNode {
        StudyNode {
            uid: String::new(),
            attributes: attributes(values),
            series: Vec::new(),
        }
    }

    fn series(values: &[(Tag, &str)], split_number: Option<u32>) -> SeriesNode {
        SeriesNode {
            key: String::new(),
            attributes: attributes(values),
            split_number,
            instances: Vec::new(),
        }
    }

    #[test]
    fn studies_are_newest_first_by_default_and_undated_last() {
        let old = study(&[(tags::STUDY_DATE, "20200101")]);
        let new = study(&[(tags::STUDY_DATE, "20240101"), (tags::STUDY_TIME, "0800")]);
        let undated = study(&[(tags::STUDY_ID, "1")]);
        let order = SortingTime::default();
        assert_eq!(compare_studies(&new, &old, order), Ordering::Less);
        assert_eq!(
            compare_studies(&old, &new, SortingTime::Chronological),
            Ordering::Less
        );
        assert_eq!(compare_studies(&old, &undated, order), Ordering::Less);
    }

    #[test]
    fn series_follow_their_number_with_reports_last() {
        let first = series(&[(tags::SERIES_NUMBER, "2")], None);
        let part = series(&[(tags::SERIES_NUMBER, "2")], Some(2));
        let later = series(&[(tags::SERIES_NUMBER, "10")], None);
        let unnumbered = series(&[], None);
        let report = series(&[(tags::SERIES_NUMBER, "1"), (tags::MODALITY, "SR")], None);
        let mut list = [report, unnumbered, later, part, first];
        list.sort_by(compare_series);
        let numbers: Vec<_> = list
            .iter()
            .map(|s| (s.attributes.get(tags::SERIES_NUMBER), s.split_number))
            .collect();
        assert_eq!(
            numbers,
            [
                (Some("2"), None),
                (Some("2"), Some(2)),
                (Some("10"), None),
                (None, None),
                (Some("1"), None)
            ]
        );
    }

    #[test]
    fn dates_and_times_sort_like_moments() {
        let at = |d, t| attributes(&[(tags::STUDY_DATE, d), (tags::STUDY_TIME, t)]);
        let key = |a: &Attributes| date_time(a, tags::STUDY_DATE, tags::STUDY_TIME);
        assert_eq!(
            key(&at("20240101", "0930")).as_deref(),
            Some("20240101093000000000")
        );
        assert_eq!(
            key(&at("2024.01.01", "09:30:15.5")).as_deref(),
            Some("20240101093015500000")
        );
        assert_eq!(key(&at("2024", "")), None);
        assert_eq!(
            stack_id(&attributes(&[(tags::STACK_ID, "7")])),
            "0000000007"
        );
    }
}
