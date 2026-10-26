//! Order of the special objects of a series (KO, PR, SR, ...): newest content first.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/DicomSpecialElement.java`
//! (`ORDER_BY_DATE`).

use std::cmp::Ordering;

use dicom_dictionary_std::tags;

use super::DicomFile;

/// What special objects are sorted by.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContentOrder {
    /// Content Date and Time as `YYYYMMDDHHMMSS.F…`, else Series Date and Time.
    pub date_time: Option<String>,
    /// Series Number.
    pub series_number: Option<i32>,
}

impl ContentOrder {
    /// Reads the order of a file.
    pub fn from_file(file: &DicomFile) -> Self {
        let date_time = |date, time| {
            let date: String = file.text(date)?;
            let time = file.text(time).unwrap_or_default().replace(':', "");
            // Seconds may be omitted (PS3.5 §6.2 TM): pad so that values compare as text.
            Some(format!("{date}{time:0<6}"))
        };
        Self {
            date_time: date_time(tags::CONTENT_DATE, tags::CONTENT_TIME)
                .or_else(|| date_time(tags::SERIES_DATE, tags::SERIES_TIME)),
            series_number: file
                .text(tags::SERIES_NUMBER)
                .and_then(|n| n.trim().parse().ok()),
        }
    }

    /// Newest first, then by increasing Series Number.
    pub fn newest_first(&self, other: &Self) -> Ordering {
        let by_date = match (&self.date_time, &other.date_time) {
            (Some(a), Some(b)) => b.cmp(a),
            _ => Ordering::Equal,
        };
        let by_number = match (self.series_number, other.series_number) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => Ordering::Equal,
        };
        by_date.then(by_number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(date_time: Option<&str>, series_number: Option<i32>) -> ContentOrder {
        ContentOrder {
            date_time: date_time.map(str::to_owned),
            series_number,
        }
    }

    #[test]
    fn newer_content_comes_first_then_lower_series_numbers() {
        let mut list = [
            order(Some("20260101120000"), Some(2)),
            order(Some("20260102080000"), Some(9)),
            order(Some("20260101120000"), Some(1)),
        ];
        list.sort_by(ContentOrder::newest_first);
        let numbers: Vec<_> = list.iter().filter_map(|o| o.series_number).collect();
        assert_eq!(numbers, [9, 1, 2]);
    }
}
