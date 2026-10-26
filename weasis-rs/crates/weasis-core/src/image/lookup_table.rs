//! Lookup tables defined by a LUT Descriptor and LUT Data (DICOM PS3.3 §C.11.1.1, §C.11.2.1.1).
//!
//! Used by the Modality LUT Sequence and the VOI LUT Sequence.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/util/LookupTableUtils.java`
//! (`createLut`) and `weasis-core-img/.../org/weasis/opencv/data/LookupTableCV.java`.

use dicom_dictionary_std::tags;
use dicom_object::InMemDicomObject;

use crate::error::{Error, Result};

/// A table mapping input values to output values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupTable {
    /// Input value of the first entry.
    pub first_mapped_value: i64,
    /// Bits per entry of the output (8 to 16).
    pub bits_per_entry: u16,
    /// Output values.
    pub entries: Vec<i32>,
}

impl LookupTable {
    /// Reads the LUT Descriptor and LUT Data of a Modality or VOI LUT Sequence item.
    /// `signed_input` tells whether the values looked up are signed (Pixel Representation).
    pub fn from_item(item: &InMemDicomObject, signed_input: bool) -> Result<Self> {
        let invalid = |what: &str| Error::PixelData(format!("invalid lookup table: {what}"));
        let descriptor: Vec<i64> = item
            .element(tags::LUT_DESCRIPTOR)
            .ok()
            .and_then(|element| element.to_multi_int::<i64>().ok())
            .filter(|values| values.len() == 3)
            .ok_or_else(|| invalid("LUT Descriptor"))?;
        // PS3.3 §C.11.1.1: 0 entries means 65536. The first mapped value is SS for signed
        // input and US otherwise; implicit VR files cannot tell them apart.
        let length = if descriptor[0] <= 0 {
            descriptor[0] + 0x1_0000
        } else {
            descriptor[0]
        };
        let length = usize::try_from(length).map_err(|_| invalid("number of entries"))?;
        let first_mapped_value = if signed_input && descriptor[1] > 0x7FFF {
            descriptor[1] - 0x1_0000
        } else {
            descriptor[1]
        };
        let bits_per_entry = u16::try_from(descriptor[2]).map_err(|_| invalid("bits per entry"))?;

        let words: Vec<u16> = item
            .element(tags::LUT_DATA)
            .ok()
            .and_then(|element| element.to_multi_int::<u16>().ok())
            .ok_or_else(|| invalid("LUT Data"))?;
        // 8-bit entries may be packed two per 16-bit word (low byte first).
        let entries: Vec<i32> = if bits_per_entry <= 8 && words.len() * 2 == length {
            words
                .iter()
                .flat_map(|word| word.to_le_bytes())
                .map(i32::from)
                .collect()
        } else {
            words.iter().map(|&word| i32::from(word)).collect()
        };
        if entries.len() < length {
            return Err(invalid("fewer entries than declared"));
        }
        Ok(Self {
            first_mapped_value,
            bits_per_entry,
            entries,
        })
    }

    /// Output for an input value; inputs outside the table use the first or last entry
    /// (PS3.3 §C.11.1.1).
    pub fn lookup(&self, value: f64) -> i32 {
        let last = self.entries.len().saturating_sub(1);
        // The saturating cast and clamp keep any input inside the table.
        #[allow(clippy::cast_possible_truncation)]
        let offset = (value.floor() as i64).saturating_sub(self.first_mapped_value);
        let index = usize::try_from(offset).unwrap_or(0).min(last);
        self.entries.get(index).copied().unwrap_or(0)
    }

    /// Smallest and largest output values.
    pub fn output_range(&self) -> (i32, i32) {
        let min = self.entries.iter().copied().min().unwrap_or(0);
        let max = self.entries.iter().copied().max().unwrap_or(0);
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> LookupTable {
        LookupTable {
            first_mapped_value: -2,
            bits_per_entry: 16,
            entries: vec![10, 20, 30],
        }
    }

    #[test]
    fn values_outside_the_table_use_the_first_or_last_entry() {
        let lut = table();
        assert_eq!(lut.lookup(-100.0), 10);
        assert_eq!(lut.lookup(-2.0), 10);
        assert_eq!(lut.lookup(-1.0), 20);
        assert_eq!(lut.lookup(0.0), 30);
        assert_eq!(lut.lookup(500.0), 30);
    }

    #[test]
    fn output_range_spans_the_entries() {
        assert_eq!(table().output_range(), (10, 30));
    }
}
