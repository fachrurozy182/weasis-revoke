//! Decoding of text that uses ISO 2022 code extensions (DICOM PS3.5 §6.1.2.5).
//!
//! Japanese, Korean and Chinese files switch character sets inside a value with escape
//! sequences, e.g. `Specific Character Set = \ISO 2022 IR 87`. `dicom-rs` does not handle code
//! extensions: it decodes such values byte by byte (as ISO-IR 6/100, or ISO-IR 13 for half-width
//! katakana). This module recovers the original bytes from that text and decodes them with the
//! character sets designated by the escape sequences.
//!
//! Java reference: dcm4che `org.dcm4che3.data.SpecificCharacterSet` (used through
//! `weasis-dicom-tools`) and `weasis-dicom-codec/.../display/CharsetEncoding.java`.

use encoding_rs::{EUC_JP, EUC_KR, Encoding, GBK};

const ESC: u8 = 0x1B;

/// Whether the Specific Character Set values use code extensions with multi-byte or Japanese
/// character sets, which `dicom-rs` cannot decode by itself.
pub fn needs_code_extension_decoding(specific_character_set: &[String]) -> bool {
    specific_character_set.iter().any(|code| {
        matches!(
            code.trim(),
            "ISO 2022 IR 13"
                | "ISO 2022 IR 87"
                | "ISO 2022 IR 159"
                | "ISO 2022 IR 149"
                | "ISO 2022 IR 58"
        )
    })
}

/// Decodes a value read by `dicom-rs` from a data set with ISO 2022 code extensions.
///
/// `first_character_set` is value 1 of Specific Character Set, which defines the initial
/// state. Person names (`is_person_name`) return to the initial state at each `^` and `=`
/// (PS3.5 §6.1.2.5.3). If the original bytes cannot be recovered, the text is returned as is.
pub fn decode_code_extensions(
    text: &str,
    first_character_set: &str,
    is_person_name: bool,
) -> String {
    match recover_bytes(text) {
        Some(bytes) => Iso2022Decoder::new(first_character_set, is_person_name).decode(&bytes),
        None => text.to_owned(),
    }
}

/// Inverts the byte-by-byte decoding of `dicom-rs`: ISO-IR 6/100 (one char per byte) and
/// ISO-IR 13 (JIS X 0201, half-width katakana at U+FF61..U+FF9F).
fn recover_bytes(text: &str) -> Option<Vec<u8>> {
    text.chars()
        .map(|character| match u32::from(character) {
            code @ 0..=0xFF => u8::try_from(code).ok(),
            code @ 0xFF61..=0xFF9F => u8::try_from(code - 0xFF61 + 0xA1).ok(),
            0x203E => Some(0x7E), // OVERLINE in JIS X 0201 Roman
            _ => None,
        })
        .collect()
}

/// Character set designated to G0 (bytes 0x21..0x7E).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G0 {
    /// ISO-IR 6.
    Ascii,
    /// ISO-IR 14, JIS X 0201 Roman.
    JisRoman,
    /// ISO-IR 87, JIS X 0208 (two bytes per character).
    Jis0208,
    /// ISO-IR 159, JIS X 0212 (two bytes per character).
    Jis0212,
}

/// Character set designated to G1 (bytes 0xA1..0xFE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G1 {
    None,
    /// ISO-IR 13, JIS X 0201 Katakana.
    Katakana,
    /// ISO-IR 149, KS X 1001 (two bytes per character).
    KsX1001,
    /// ISO-IR 58, GB 2312 (two bytes per character).
    Gb2312,
    /// Single-byte sets ISO-IR 100, 101, ... (upper half of an 8-bit encoding).
    SingleByte(&'static Encoding),
}

struct Iso2022Decoder {
    initial: (G0, G1),
    g0: G0,
    g1: G1,
    is_person_name: bool,
    output: String,
}

impl Iso2022Decoder {
    fn new(first_character_set: &str, is_person_name: bool) -> Self {
        // PS3.3 C.12.1.1.2: value 1 designates the initial G0 and G1 sets.
        let initial = match first_character_set.trim() {
            "ISO 2022 IR 13" => (G0::JisRoman, G1::Katakana),
            code => (G0::Ascii, single_byte_g1(code).unwrap_or(G1::None)),
        };
        Self {
            initial,
            g0: initial.0,
            g1: initial.1,
            is_person_name,
            output: String::new(),
        }
    }

    fn decode(mut self, bytes: &[u8]) -> String {
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            index += if byte == ESC {
                self.escape(&bytes[index + 1..])
            } else if byte < 0x80 {
                self.g0_character(&bytes[index..])
            } else {
                self.g1_character(&bytes[index..])
            };
        }
        self.output
    }

    /// Applies an escape sequence; returns the number of bytes consumed, ESC included.
    fn escape(&mut self, sequence: &[u8]) -> usize {
        let designation = match sequence {
            [b'(', b'B', ..] => Some((2, Some(G0::Ascii), None)),
            [b'(', b'J', ..] => Some((2, Some(G0::JisRoman), None)),
            [b')', b'I', ..] => Some((2, None, Some(G1::Katakana))),
            [b'$', b'B', ..] => Some((2, Some(G0::Jis0208), None)),
            [b'$', b'(', b'D', ..] => Some((3, Some(G0::Jis0212), None)),
            [b'$', b')', b'C', ..] => Some((3, None, Some(G1::KsX1001))),
            [b'$', b')', b'A', ..] => Some((3, None, Some(G1::Gb2312))),
            [b'-', final_byte, ..] => {
                single_byte_by_final(*final_byte).map(|g1| (2, None, Some(g1)))
            }
            _ => None,
        };
        match designation {
            Some((length, g0, g1)) => {
                self.g0 = g0.unwrap_or(self.g0);
                self.g1 = g1.unwrap_or(self.g1);
                1 + length
            }
            // Unknown escape sequence: keep the ESC byte visible rather than dropping text.
            None => {
                self.output.push(char::from(ESC));
                1
            }
        }
    }

    /// Decodes one G0 character; returns the number of bytes consumed.
    fn g0_character(&mut self, bytes: &[u8]) -> usize {
        let byte = bytes[0];
        let is_two_byte_set = matches!(self.g0, G0::Jis0208 | G0::Jis0212);
        // Control characters and spaces are never part of a two-byte character.
        if is_two_byte_set && byte > 0x20 && bytes.len() >= 2 {
            let euc = [byte | 0x80, bytes[1] | 0x80];
            let decoded = match self.g0 {
                G0::Jis0212 => decode_with(EUC_JP, &[0x8F, euc[0], euc[1]]),
                _ => decode_with(EUC_JP, &euc),
            };
            self.output.push_str(&decoded);
            return 2;
        }
        let character = match (self.g0, byte) {
            (G0::JisRoman, 0x5C) => '\u{A5}',   // YEN SIGN
            (G0::JisRoman, 0x7E) => '\u{203E}', // OVERLINE
            _ => char::from(byte),
        };
        self.output.push(character);
        self.reset_after_delimiter(byte);
        1
    }

    /// Decodes one G1 character; returns the number of bytes consumed.
    fn g1_character(&mut self, bytes: &[u8]) -> usize {
        let byte = bytes[0];
        match self.g1 {
            G1::Katakana => {
                let katakana = char::from_u32(0xFF61 + u32::from(byte) - 0xA1);
                self.output
                    .push(katakana.unwrap_or(char::REPLACEMENT_CHARACTER));
                1
            }
            G1::KsX1001 | G1::Gb2312 if bytes.len() >= 2 => {
                let encoding = if self.g1 == G1::KsX1001 { EUC_KR } else { GBK };
                self.output.push_str(&decode_with(encoding, &bytes[..2]));
                2
            }
            G1::SingleByte(encoding) => {
                self.output.push_str(&decode_with(encoding, &[byte]));
                1
            }
            _ => {
                self.output.push(char::REPLACEMENT_CHARACTER);
                1
            }
        }
    }

    /// PS3.5 §6.1.2.5.3: the initial sets are restored after control characters, and after
    /// `^` and `=` in person names.
    fn reset_after_delimiter(&mut self, byte: u8) {
        let is_delimiter = matches!(byte, b'\r' | b'\n' | b'\t' | 0x0C | b'\\')
            || (self.is_person_name && matches!(byte, b'^' | b'='));
        if is_delimiter {
            (self.g0, self.g1) = self.initial;
        }
    }
}

fn decode_with(encoding: &'static Encoding, bytes: &[u8]) -> String {
    encoding.decode_without_bom_handling(bytes).0.into_owned()
}

/// Single-byte character sets usable in G1, by Specific Character Set code.
fn single_byte_g1(code: &str) -> Option<G1> {
    let final_byte = match code {
        "ISO 2022 IR 100" => b'A',
        "ISO 2022 IR 101" => b'B',
        "ISO 2022 IR 109" => b'C',
        "ISO 2022 IR 110" => b'D',
        "ISO 2022 IR 126" => b'F',
        "ISO 2022 IR 127" => b'G',
        "ISO 2022 IR 138" => b'H',
        "ISO 2022 IR 144" => b'L',
        "ISO 2022 IR 148" => b'M',
        "ISO 2022 IR 166" => b'T',
        _ => return None,
    };
    single_byte_by_final(final_byte)
}

/// Single-byte G1 sets by the final byte of their escape sequence `ESC - F` (PS3.3 C.12-3).
fn single_byte_by_final(final_byte: u8) -> Option<G1> {
    // encoding_rs implements the WHATWG encodings: ISO-8859-1 and -9 are decoded as their
    // windows-125x supersets, identical in the range used by G1 (0xA0..0xFF).
    let encoding = match final_byte {
        b'A' => encoding_rs::WINDOWS_1252,
        b'B' => encoding_rs::ISO_8859_2,
        b'C' => encoding_rs::ISO_8859_3,
        b'D' => encoding_rs::ISO_8859_4,
        b'F' => encoding_rs::ISO_8859_7,
        b'G' => encoding_rs::ISO_8859_6,
        b'H' => encoding_rs::ISO_8859_8,
        b'L' => encoding_rs::ISO_8859_5,
        b'M' => encoding_rs::WINDOWS_1254,
        b'T' => encoding_rs::WINDOWS_874,
        _ => return None,
    };
    Some(G1::SingleByte(encoding))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn as_read_by_dicom_rs(bytes: &[u8]) -> String {
        bytes.iter().map(|&b| char::from(b)).collect()
    }

    #[test]
    fn decodes_japanese_kanji_and_hiragana() {
        let raw =
            b"Yamada^Tarou=\x1b$B;3ED\x1b(B^\x1b$BB@O:\x1b(B=\x1b$B$d$^$@\x1b(B^\x1b$B$?$m$&\x1b(B";
        let decoded = decode_code_extensions(&as_read_by_dicom_rs(raw), "", true);
        assert_eq!(decoded, "Yamada^Tarou=山田^太郎=やまだ^たろう");
    }

    #[test]
    fn decodes_korean() {
        let raw = b"\x1b$)C\xb1\xe8\xc8\xf1\xc1\xdf";
        let decoded = decode_code_extensions(&as_read_by_dicom_rs(raw), "", true);
        assert_eq!(decoded, "김희중");
    }

    #[test]
    fn decodes_half_width_katakana_when_value_1_is_iso_ir_13() {
        // dicom-rs already maps ISO-IR 13 bytes to U+FF61..U+FF9F; they must be recovered.
        let read = "\u{FF94}\u{FF8F}\u{FF80}\u{FF9E}^\u{FF80}\u{FF9B}\u{FF73}=\x1b$B;3ED\x1b(J";
        let decoded = decode_code_extensions(read, "ISO 2022 IR 13", true);
        assert_eq!(decoded, "ﾔﾏﾀﾞ^ﾀﾛｳ=山田");
    }

    #[test]
    fn only_iso_2022_multibyte_and_japanese_sets_need_this_decoder() {
        assert!(needs_code_extension_decoding(&[
            String::new(),
            "ISO 2022 IR 87".into()
        ]));
        assert!(!needs_code_extension_decoding(&["ISO_IR 192".into()]));
        assert!(!needs_code_extension_decoding(&["ISO_IR 100".into()]));
    }
}
