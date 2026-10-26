//! Reading a DICOMDIR: the patients, studies and series of a file-set, and the files that hold
//! their instances (Basic Directory IOD, PS3.3 §F.3; media storage, PS3.10).
//!
//! Records form a tree linked by byte offsets: each record points to its next sibling and to its
//! first child. Inactive records (Record In-use Flag = 0) and PRIVATE records are skipped.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/imp/DicomDirLoader.java`
//! and `DicomDirFileResolver.java`.

mod offsets;

use std::collections::{HashMap, HashSet};

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};

use super::{DicomFile, read_dicom};
use crate::error::{Error, Result};
use crate::model::{Attributes, Level, NO_VALUE};

/// A series listed in a DICOMDIR, with the attributes of its patient and study records.
#[derive(Debug, Clone, PartialEq)]
pub struct DirSeries {
    /// Attributes of the patient record.
    pub patient: Attributes,
    /// Attributes of the study record.
    pub study: Attributes,
    /// Attributes of the series record.
    pub series: Attributes,
    /// Instances, in record order.
    pub instances: Vec<DirInstance>,
    /// Icon Image Sequence item of the series record, else of its first instance record.
    // TODO(2.5): decode the icon for the explorer thumbnail.
    pub icon: Option<InMemDicomObject>,
}

/// An instance listed in a DICOMDIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirInstance {
    /// Referenced SOP Instance UID in File.
    pub sop_instance_uid: String,
    /// Instance Number of the record, if any.
    pub instance_number: Option<i32>,
    /// Referenced File ID: path components relative to the DICOMDIR folder.
    pub file_id: Vec<String>,
}

/// Series of a DICOMDIR that reference at least one file, in record order.
pub fn read_dicomdir(bytes: &[u8]) -> Result<Vec<DirSeries>> {
    let file = read_dicom(bytes)?;
    let items = file
        .object()
        .element(tags::DIRECTORY_RECORD_SEQUENCE)
        .ok()
        .and_then(|element| element.items())
        .ok_or_else(|| Error::UnreadableDicom("DICOMDIR without records".to_owned()))?;
    let offsets = offsets::record_offsets(bytes)?;
    if offsets.len() != items.len() {
        return Err(Error::UnreadableDicom(
            "DICOMDIR records do not match their offsets".to_owned(),
        ));
    }
    let directory = Directory {
        file: &file,
        records: offsets.into_iter().zip(items).collect(),
    };
    let mut series = Vec::new();
    let root = offset(
        file.object(),
        tags::OFFSET_OF_THE_FIRST_DIRECTORY_RECORD_OF_THE_ROOT_DIRECTORY_ENTITY,
    );
    for patient in directory.siblings(root, "PATIENT") {
        for study in directory.siblings(offset(patient, LOWER), "STUDY") {
            for record in directory.siblings(offset(study, LOWER), "SERIES") {
                directory.add_series(&mut series, [patient, study, record]);
            }
        }
    }
    series.retain(|s: &DirSeries| !s.instances.is_empty());
    Ok(series)
}

const LOWER: Tag = tags::OFFSET_OF_REFERENCED_LOWER_LEVEL_DIRECTORY_ENTITY;

struct Directory<'a> {
    file: &'a DicomFile,
    records: HashMap<u64, &'a InMemDicomObject>,
}

impl<'a> Directory<'a> {
    /// In-use records of one type, from `first` following the next-record offsets.
    fn siblings(
        &self,
        first: u64,
        record_type: &'a str,
    ) -> impl Iterator<Item = &'a InMemDicomObject> + '_ {
        let mut visited = HashSet::new();
        let mut next = first;
        std::iter::from_fn(move || {
            // Offset 0 ends the list; a loop in a corrupted file ends it too.
            while next != 0 && visited.insert(next) {
                let record = *self.records.get(&next)?;
                next = offset(record, tags::OFFSET_OF_THE_NEXT_DIRECTORY_RECORD);
                if is_in_use(record)
                    && self.text(record, tags::DIRECTORY_RECORD_TYPE).as_deref()
                        == Some(record_type)
                {
                    return Some(record);
                }
            }
            None
        })
    }

    /// Children of a series that reference a file, whatever their type (IMAGE, SR DOCUMENT, ...).
    fn instances(&self, series: &InMemDicomObject) -> Vec<(DirInstance, &'a InMemDicomObject)> {
        let mut visited = HashSet::new();
        let mut next = offset(series, LOWER);
        let mut instances = Vec::new();
        while next != 0 && visited.insert(next) {
            let Some(&record) = self.records.get(&next) else {
                break;
            };
            next = offset(record, tags::OFFSET_OF_THE_NEXT_DIRECTORY_RECORD);
            let private =
                self.text(record, tags::DIRECTORY_RECORD_TYPE).as_deref() == Some("PRIVATE");
            let uid = self.text(record, tags::REFERENCED_SOP_INSTANCE_UID_IN_FILE);
            let file_id = self.text(record, tags::REFERENCED_FILE_ID);
            if let (true, false, Some(uid), Some(file_id)) =
                (is_in_use(record), private, uid, file_id)
            {
                let instance = DirInstance {
                    sop_instance_uid: uid,
                    instance_number: record
                        .element(tags::INSTANCE_NUMBER)
                        .ok()
                        .and_then(|e| e.to_int().ok()),
                    file_id: file_id.split('\\').map(|c| c.trim().to_owned()).collect(),
                };
                instances.push((instance, record));
            }
        }
        instances
    }

    /// Adds the instances of a series record, merging records of the same series as Java does.
    fn add_series(
        &self,
        all: &mut Vec<DirSeries>,
        [patient, study, series]: [&InMemDicomObject; 3],
    ) {
        let study_uid = self
            .text(study, tags::STUDY_INSTANCE_UID)
            .unwrap_or_else(|| NO_VALUE.to_owned());
        let series_uid = self
            .text(series, tags::SERIES_INSTANCE_UID)
            .unwrap_or_else(|| NO_VALUE.to_owned());
        let position = all.iter().position(|s| {
            s.study.get(tags::STUDY_INSTANCE_UID) == Some(&study_uid)
                && s.series.get(tags::SERIES_INSTANCE_UID) == Some(&series_uid)
        });
        let index = position.unwrap_or_else(|| {
            let mut series_attributes = self.attributes(series, Level::Series);
            series_attributes.set(tags::SERIES_INSTANCE_UID, series_uid.as_str());
            let mut study_attributes = self.attributes(study, Level::Study);
            study_attributes.set(tags::STUDY_INSTANCE_UID, study_uid.as_str());
            all.push(DirSeries {
                patient: self.attributes(patient, Level::Patient),
                study: study_attributes,
                series: series_attributes,
                instances: Vec::new(),
                icon: icon(series).cloned(),
            });
            all.len() - 1
        });
        let target = &mut all[index];
        for (instance, record) in self.instances(series) {
            let known = target.instances.iter().any(|i| {
                i.sop_instance_uid == instance.sop_instance_uid
                    && i.instance_number == instance.instance_number
            });
            if !known {
                if target.icon.is_none() {
                    target.icon = icon(record).cloned();
                }
                target.instances.push(instance);
            }
        }
    }

    fn attributes(&self, record: &InMemDicomObject, level: Level) -> Attributes {
        let mut attributes = Attributes::new();
        for &tag in level.attributes() {
            if let Some(value) = self.text(record, tag) {
                attributes.set(tag, value);
            }
        }
        attributes
    }

    fn text(&self, record: &InMemDicomObject, tag: Tag) -> Option<String> {
        self.file.item_text(record, tag)
    }
}

/// Record In-use Flag (retired): 0 marks an inactive record; absent means in use.
fn is_in_use(record: &InMemDicomObject) -> bool {
    record
        .element(tags::RECORD_IN_USE_FLAG)
        .ok()
        .and_then(|e| e.to_int::<u32>().ok())
        != Some(0)
}

fn offset(record: &InMemDicomObject, tag: Tag) -> u64 {
    record
        .element(tag)
        .ok()
        .and_then(|e| e.to_int::<u64>().ok())
        .unwrap_or(0)
}

fn icon(record: &InMemDicomObject) -> Option<&InMemDicomObject> {
    record
        .element(tags::ICON_IMAGE_SEQUENCE)
        .ok()?
        .items()?
        .first()
}

/// Path of a Referenced File ID among the files of the DICOMDIR folder (paths relative to it,
/// separated by `/`). As in the Java viewer, a lower-case name, or a single file with an added
/// extension, is accepted when the exact name is missing.
///
/// Components that could leave the folder (`..`, separators, drive letters) are rejected.
/// NOTE: symbolic links are not checked here; the server checks them when it reads the files.
pub fn resolve_file_id<'a>(file_id: &[String], files: &'a [String]) -> Option<&'a str> {
    let safe = |c: &String| !c.is_empty() && c != "." && c != ".." && !c.contains(['/', '\\', ':']);
    if file_id.is_empty() || !file_id.iter().all(safe) {
        return None;
    }
    let path = file_id.join("/");
    let lower = path.to_lowercase();
    let find = |wanted: &str| files.iter().find(|f| *f == wanted).map(String::as_str);
    find(&path).or_else(|| find(&lower)).or_else(|| {
        let mut with_extension = files
            .iter()
            .filter(|f| f.starts_with(&format!("{path}.")) || f.starts_with(&format!("{lower}.")))
            .filter(|f| !f[path.len() + 1..].contains('/'));
        match (with_extension.next(), with_extension.next()) {
            (Some(only), None) => Some(only.as_str()),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests;
