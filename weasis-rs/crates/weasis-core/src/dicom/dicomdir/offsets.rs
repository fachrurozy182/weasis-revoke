//! Byte offsets of the items of the Directory Record Sequence.
//!
//! Directory records point to each other by their offset in the file (PS3.3 §F.3.2.1), which
//! the parsed data set does not keep. A DICOMDIR is always encoded in Explicit VR Little Endian
//! (PS3.10 §8.6), so the raw bytes are walked here just enough to find where each item starts.
//!
//! Java reference: dcm4che `org.dcm4che3.media.DicomDirReader` (records read by offset).

use crate::error::{Error, Result};

/// Directory Record Sequence (0004,1220).
const DIRECTORY_RECORD_SEQUENCE: (u16, u16) = (0x0004, 0x1220);
const ITEM: (u16, u16) = (0xFFFE, 0xE000);
const ITEM_DELIMITATION: (u16, u16) = (0xFFFE, 0xE00D);
const SEQUENCE_DELIMITATION: (u16, u16) = (0xFFFE, 0xE0DD);
const UNDEFINED_LENGTH: u32 = 0xFFFF_FFFF;
/// VRs with a 2-byte reserved field and a 4-byte length (PS3.5 Table 7.1-1).
const LONG_VRS: [&[u8; 2]; 10] = [
    b"OB", b"OD", b"OF", b"OL", b"OV", b"OW", b"SQ", b"UC", b"UN", b"UR",
];
const LONG_TEXT_VRS: [&[u8; 2]; 3] = [b"UT", b"SV", b"UV"];

/// Offsets from the start of the file of the items of the Directory Record Sequence, in order.
pub fn record_offsets(bytes: &[u8]) -> Result<Vec<u64>> {
    let mut reader = Reader {
        bytes,
        position: data_start(bytes)?,
    };
    while reader.position < bytes.len() {
        let header = reader.element_header()?;
        if header.tag == DIRECTORY_RECORD_SEQUENCE {
            return reader.item_offsets(header.length);
        }
        reader.skip_value(&header)?;
    }
    Err(invalid("no Directory Record Sequence"))
}

/// Position after the "DICM" prefix; the File Meta Information is Explicit VR Little Endian too.
fn data_start(bytes: &[u8]) -> Result<usize> {
    if bytes.get(128..132) == Some(b"DICM") {
        Ok(132)
    } else if bytes.get(0..4) == Some(b"DICM") {
        Ok(4)
    } else {
        Err(invalid("no DICM prefix"))
    }
}

fn invalid(reason: &str) -> Error {
    Error::UnreadableDicom(format!("invalid DICOMDIR: {reason}"))
}

struct Header {
    tag: (u16, u16),
    is_sequence: bool,
    length: u32,
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.position + N;
        let value = self
            .bytes
            .get(self.position..end)
            .and_then(|slice| <[u8; N]>::try_from(slice).ok())
            .ok_or_else(|| invalid("truncated file"))?;
        self.position = end;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16> {
        self.take::<2>().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Result<u32> {
        self.take::<4>().map(u32::from_le_bytes)
    }

    fn tag(&mut self) -> Result<(u16, u16)> {
        Ok((self.u16()?, self.u16()?))
    }

    /// Header of a data element, or of an item / delimitation (tag and 4-byte length only).
    fn element_header(&mut self) -> Result<Header> {
        let tag = self.tag()?;
        if tag.0 == 0xFFFE {
            return Ok(Header {
                tag,
                is_sequence: false,
                length: self.u32()?,
            });
        }
        let vr = self.take::<2>()?;
        let length = if LONG_VRS.contains(&&vr) || LONG_TEXT_VRS.contains(&&vr) {
            self.u16()?;
            self.u32()?
        } else {
            u32::from(self.u16()?)
        };
        Ok(Header {
            tag,
            is_sequence: &vr == b"SQ",
            length,
        })
    }

    fn skip(&mut self, length: u32) -> Result<()> {
        let end = self.position + length as usize;
        if end > self.bytes.len() {
            return Err(invalid("value beyond the end of the file"));
        }
        self.position = end;
        Ok(())
    }

    fn skip_value(&mut self, header: &Header) -> Result<()> {
        if header.length != UNDEFINED_LENGTH {
            return self.skip(header.length);
        }
        // A sequence, or encapsulated pixel data (fragments) in an Icon Image Sequence.
        loop {
            let item = self.element_header()?;
            match item.tag {
                SEQUENCE_DELIMITATION => return Ok(()),
                ITEM if header.is_sequence => self.skip_item(item.length)?,
                ITEM => self.skip(item.length)?,
                _ => return Err(invalid("unexpected element in a sequence")),
            }
        }
    }

    fn skip_item(&mut self, length: u32) -> Result<()> {
        if length != UNDEFINED_LENGTH {
            return self.skip(length);
        }
        loop {
            let header = self.element_header()?;
            if header.tag == ITEM_DELIMITATION {
                return Ok(());
            }
            self.skip_value(&header)?;
        }
    }

    fn item_offsets(&mut self, sequence_length: u32) -> Result<Vec<u64>> {
        let end =
            (sequence_length != UNDEFINED_LENGTH).then(|| self.position + sequence_length as usize);
        let mut offsets = Vec::new();
        while end.is_none_or(|end| self.position < end) {
            let start = self.position as u64;
            let item = self.element_header()?;
            match item.tag {
                SEQUENCE_DELIMITATION => break,
                ITEM => {
                    offsets.push(start);
                    self.skip_item(item.length)?;
                }
                _ => return Err(invalid("unexpected element in the record sequence")),
            }
        }
        Ok(offsets)
    }
}
