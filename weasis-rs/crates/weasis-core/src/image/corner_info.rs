//! Text shown in the four corners of a view: which attributes, in which slots, and how their
//! values are formatted.
//!
//! A slot names one or more attributes; the first one with a value is shown. Its format puts the
//! value in a text with `$V`, optionally followed by `:l$25$` (cut after 25 characters) or
//! `:f$#,##0.##$` (decimal pattern).
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/display/ModalityView.java`,
//! `CornerInfoData.java`, `CornerDisplay.java`, `weasis-core/.../api/media/data/TagView.java`,
//! `TagW.java` (`getFormattedText`, `formatValue`) and `weasis-dicom-codec/.../TagD.java`
//! (`getDicomPersonName`).
//
// TODO(7.6): add the site layer (`attributesView.json`) per modality with the configuration files.

use std::sync::OnceLock;

use dicom_core::VR;
use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry, VirtualVr};
use dicom_dictionary_std::{StandardDataDictionary, tags};
use dicom_object::Tag;

use crate::model::{Attributes, IdentityMask, MaskedValue};

/// Number of text lines of a corner.
pub const CORNER_LINES: usize = 7;

/// A corner of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    /// Top left: the patient.
    TopLeft,
    /// Top right: the study.
    TopRight,
    /// Bottom left: left free for the view information (window, zoom, ...).
    BottomLeft,
    /// Bottom right: the series and the image.
    BottomRight,
}

impl Corner {
    /// The four corners.
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];
}

/// One line of a corner: candidate attributes and the format of the first one with a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    /// Candidate attributes, in order of preference.
    pub tags: Vec<Tag>,
    /// Format with `$V`; `None` for the value alone.
    pub format: Option<String>,
}

impl TagView {
    fn new(format: Option<&str>, tags: &[Tag]) -> Self {
        Self {
            tags: tags.to_vec(),
            format: format.map(str::to_owned),
        }
    }

    /// Text of the line for an image, looking up each attribute in `sources` (e.g. the instance,
    /// series, study and patient attributes); empty when no attribute has a value.
    pub fn text(&self, sources: &[&Attributes]) -> String {
        self.masked_text(sources, None)
    }

    /// Text of the line with identity masking: a removed attribute gives way to the next
    /// candidate, a cleared one empties the line.
    pub fn masked_text(&self, sources: &[&Attributes], mask: Option<&IdentityMask>) -> String {
        for &tag in &self.tags {
            let Some(value) = sources.iter().find_map(|attributes| attributes.get(tag)) else {
                continue;
            };
            let value = match mask.map(|m| m.apply(tag, value)) {
                None => value.to_owned(),
                Some(MaskedValue::Show(shown)) => shown,
                Some(MaskedValue::Clear) => return String::new(),
                Some(MaskedValue::Remove) => continue,
            };
            let text = format_value(tag, &value, self.format.as_deref());
            if !text.trim().is_empty() {
                return text;
            }
        }
        String::new()
    }
}

/// The lines of the four corners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CornerLayout {
    corners: [[Option<TagView>; CORNER_LINES]; 4],
}

impl CornerLayout {
    /// The built-in layout of the Java viewer (DEFAULT modality).
    pub fn built_in() -> &'static Self {
        static LAYOUT: OnceLock<CornerLayout> = OnceLock::new();
        LAYOUT.get_or_init(built_in_layout)
    }

    /// The lines of a corner; `None` for an empty line.
    pub fn lines(&self, corner: Corner) -> &[Option<TagView>; CORNER_LINES] {
        &self.corners[corner as usize]
    }

    /// Attributes used by the layout; they are kept on each instance.
    pub fn tags(&self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = self
            .corners
            .iter()
            .flatten()
            .flatten()
            .flat_map(|view| view.tags.iter().copied())
            .collect();
        tags.sort_unstable();
        tags.dedup();
        tags
    }
}

// The labels are the English texts of `ModalityView.*` in the Java `messages.properties`.
fn built_in_layout() -> CornerLayout {
    let line = |format: Option<&str>, tags: &[Tag]| Some(TagView::new(format, tags));
    let desc25 = Some("$V:l$25$");
    let mut corners: [[Option<TagView>; CORNER_LINES]; 4] = Default::default();
    corners[Corner::TopLeft as usize] = [
        line(None, &[tags::PATIENT_NAME]),
        line(None, &[tags::PATIENT_BIRTH_DATE]),
        line(Some("ID: $V"), &[tags::PATIENT_ID]),
        line(Some("Sex: $V"), &[tags::PATIENT_SEX]),
        line(None, &[tags::PATIENT_AGE]),
        None,
        None,
    ];
    corners[Corner::TopRight as usize] = [
        line(None, &[tags::INSTITUTION_NAME]),
        line(desc25, &[tags::STUDY_DESCRIPTION]),
        line(Some("Study ID: $V"), &[tags::STUDY_ID]),
        line(Some("Ac. Nb: $V"), &[tags::ACCESSION_NUMBER]),
        line(
            Some("Acq.: $V"),
            &[
                tags::ACQUISITION_DATE,
                tags::CONTENT_DATE,
                tags::DATE_OF_SECONDARY_CAPTURE,
                tags::SERIES_DATE,
                tags::STUDY_DATE,
            ],
        ),
        line(
            Some("Acq.: $V"),
            &[
                tags::ACQUISITION_TIME,
                tags::CONTENT_TIME,
                tags::TIME_OF_SECONDARY_CAPTURE,
                tags::SERIES_TIME,
                tags::STUDY_TIME,
            ],
        ),
        None,
    ];
    corners[Corner::BottomRight as usize] = [
        None,
        line(Some("Series Nb: $V"), &[tags::SERIES_NUMBER]),
        line(
            Some("Laterality: $V"),
            &[
                tags::FRAME_LATERALITY,
                tags::IMAGE_LATERALITY,
                tags::LATERALITY,
            ],
        ),
        line(desc25, &[tags::CONTRAST_BOLUS_AGENT]),
        line(desc25, &[tags::SERIES_DESCRIPTION]),
        line(
            Some("Thickness: $V:f$#,##0.##$ mm"),
            &[tags::SLICE_THICKNESS],
        ),
        line(Some("Location: $V:f$#,##0.##$ mm"), &[tags::SLICE_LOCATION]),
    ];
    CornerLayout { corners }
}

/// Text of a value in DICOM string form, put in its format.
///
/// NOTE: dates and times are written `YYYY-MM-DD` and `HH:MM:SS`; the Java viewer uses the
/// format of the user's locale, which the web interface applies when it localizes the texts.
pub fn format_value(tag: Tag, value: &str, format: Option<&str>) -> String {
    let vr = match StandardDataDictionary
        .by_tag(tag)
        .map(DataDictionaryEntry::vr)
    {
        Some(VirtualVr::Exact(vr)) => Some(vr),
        _ => None,
    };
    let text = match vr {
        Some(VR::PN) => person_name(value),
        Some(VR::DA) => date(value),
        Some(VR::TM) => time(value),
        _ => value
            .split('\\')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\\"),
    };
    let is_decimal = matches!(vr, Some(VR::DS | VR::FL | VR::FD));
    match format {
        Some(format) if format.trim() != "$V" => apply_format(&text, is_decimal, format),
        _ => text,
    }
}

/// Puts a value in a format: `$V`, `$V:l$<limit>$` or `$V:f$<decimal pattern>$`.
fn apply_format(value: &str, is_decimal: bool, format: &str) -> String {
    let Some(index) = format.find("$V") else {
        return value.to_owned();
    };
    let (prefix, rest) = (&format[..index], &format[index + 2..]);
    let (value, suffix) = match parse_option(rest) {
        Some(('l', pattern, suffix)) => {
            let limit: usize = pattern.parse().unwrap_or(usize::MAX);
            let cut = match value.char_indices().nth(limit) {
                Some((end, _)) => format!("{}...", &value[..end]),
                None => value.to_owned(),
            };
            (cut, suffix)
        }
        // NOTE: a decimal pattern on a non-decimal value is dropped; the Java viewer leaves
        // part of the pattern in the text.
        Some(('f', pattern, suffix)) => {
            let formatted = value
                .parse::<f64>()
                .ok()
                .filter(|_| is_decimal)
                .map_or_else(|| value.to_owned(), |number| decimal(number, pattern));
            (formatted, suffix)
        }
        _ => (value.to_owned(), rest),
    };
    format!("{prefix}{value}{suffix}")
}

/// `:<kind>$<pattern>$` at the start of `rest`: the kind, the pattern and what follows.
fn parse_option(rest: &str) -> Option<(char, &str, &str)> {
    let mut chars = rest.strip_prefix(':')?.chars();
    let kind = chars.next()?;
    let after_kind = chars.as_str().strip_prefix('$')?;
    let end = after_kind.find('$')?;
    Some((kind, &after_kind[..end], &after_kind[end + 1..]))
}

/// A number written with a `DecimalFormat` pattern such as `#,##0.##` or `0.0`: grouping when
/// the pattern has a comma, as many decimals as the pattern allows (optional ones trimmed),
/// rounded half to even like Java.
fn decimal(number: f64, pattern: &str) -> String {
    let fraction = pattern.split_once('.').map_or("", |(_, f)| f);
    let max_decimals = fraction.chars().filter(|c| matches!(c, '0' | '#')).count();
    let min_decimals = fraction.chars().filter(|&c| c == '0').count();
    let rounded = round_half_even(number, max_decimals);
    let mut text = format!("{rounded:.max_decimals$}");
    // Optional decimals ('#') are dropped when they are trailing zeros.
    let mut optional = max_decimals - min_decimals;
    while optional > 0 && text.ends_with('0') {
        text.pop();
        optional -= 1;
    }
    if text.ends_with('.') {
        text.pop();
    }
    if pattern.contains(',') {
        text = group_thousands(&text);
    }
    text
}

fn round_half_even(number: f64, decimals: usize) -> f64 {
    let scale = 10f64.powi(i32::try_from(decimals).unwrap_or(0));
    let scaled = number * scale;
    let rounded = scaled.round();
    let result = if (scaled - scaled.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - scaled.signum()
    } else {
        rounded
    };
    result / scale
}

fn group_thousands(text: &str) -> String {
    let (sign, unsigned) = text
        .strip_prefix('-')
        .map_or(("", text), |rest| ("-", rest));
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (index, digit) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    match fraction {
        Some(fraction) => format!("{sign}{grouped}.{fraction}"),
        None => format!("{sign}{grouped}"),
    }
}

/// "Family^Given^Middle^Prefix^Suffix" written "Family, Given Middle, Prefix, Suffix", for each
/// component group (alphabetic, ideographic, phonetic) separated by `=`.
fn person_name(value: &str) -> String {
    value
        .split('=')
        .map(|group| {
            let mut text = String::new();
            for (index, part) in group.split('^').enumerate() {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if !text.is_empty() || index > 0 {
                    text.push_str(if index == 1 || index >= 3 { ", " } else { " " });
                }
                text.push_str(part);
            }
            text.trim_start_matches([',', ' ']).to_owned()
        })
        .collect::<Vec<_>>()
        .join("=")
}

fn date(value: &str) -> String {
    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    match (digits.get(0..4), digits.get(4..6), digits.get(6..8)) {
        (Some(y), Some(m), Some(d)) => format!("{y}-{m}-{d}"),
        _ => value.to_owned(),
    }
}

fn time(value: &str) -> String {
    let digits: String = value
        .split('.')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let field = |range: std::ops::Range<usize>| digits.get(range).unwrap_or("00");
    if digits.len() < 2 {
        return value.to_owned();
    }
    format!("{}:{}:{}", field(0..2), field(2..4), field(4..6))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_put_in_their_format() {
        assert_eq!(
            format_value(tags::PATIENT_ID, "123", Some("ID: $V")),
            "ID: 123"
        );
        assert_eq!(
            format_value(
                tags::SLICE_THICKNESS,
                "2.500",
                Some("Thickness: $V:f$#,##0.##$ mm")
            ),
            "Thickness: 2.5 mm"
        );
        assert_eq!(
            format_value(tags::SLICE_LOCATION, "-1234.567", Some("$V:f$#,##0.##$")),
            "-1,234.57"
        );
        assert_eq!(
            format_value(tags::STUDY_DESCRIPTION, "abcdefghij", Some("$V:l$4$")),
            "abcd..."
        );
    }

    #[test]
    fn names_dates_and_times_are_readable() {
        assert_eq!(
            format_value(tags::PATIENT_NAME, "Doe^John^Q^Dr", None),
            "Doe, John Q, Dr"
        );
        assert_eq!(format_value(tags::PATIENT_NAME, "^John", None), "John");
        assert_eq!(
            format_value(tags::STUDY_DATE, "20260131", None),
            "2026-01-31"
        );
        assert_eq!(format_value(tags::STUDY_TIME, "0930", None), "09:30:00");
    }

    #[test]
    fn decimals_follow_the_pattern() {
        assert_eq!(decimal(2.0, "#,##0.##"), "2");
        assert_eq!(decimal(0.125, "0.00"), "0.12", "half to even");
        assert_eq!(decimal(1_000_000.5, "#,##0.0"), "1,000,000.5");
    }

    #[test]
    fn the_first_attribute_with_a_value_is_shown() {
        let mut instance = Attributes::new();
        instance.set(tags::CONTENT_DATE, "20260102");
        let mut study = Attributes::new();
        study.set(tags::STUDY_DATE, "20260101");
        let date_line = CornerLayout::built_in().lines(Corner::TopRight)[4]
            .clone()
            .expect("line");
        assert_eq!(date_line.text(&[&instance, &study]), "Acq.: 2026-01-02");
        assert_eq!(date_line.text(&[&study]), "Acq.: 2026-01-01");
        assert_eq!(date_line.text(&[]), "");
    }

    #[test]
    fn masking_removes_or_pseudonymizes_lines() {
        use crate::model::MaskingModel;
        let model = MaskingModel::built_in().expect("valid");
        let publication = IdentityMask::new(
            &model,
            model.profile("publication").expect("profile"),
            b"salt",
        );
        let mut patient = Attributes::new();
        patient.set(tags::PATIENT_ID, "123");
        let mut study = Attributes::new();
        study.set(tags::STUDY_DATE, "20260101");
        let top_left = CornerLayout::built_in().lines(Corner::TopLeft);
        let id_line = top_left[2].clone().expect("line");
        let id = id_line.masked_text(&[&patient], Some(&publication));
        assert!(id.starts_with("ID: ") && !id.contains("123"), "{id}");
        // Dates are removed by this profile, so the acquisition date line is empty.
        let date_line = CornerLayout::built_in().lines(Corner::TopRight)[4]
            .clone()
            .expect("line");
        assert_eq!(date_line.masked_text(&[&study], Some(&publication)), "");
        assert_eq!(date_line.masked_text(&[&study], None), "Acq.: 2026-01-01");
    }
}
