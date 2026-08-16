//! Annex.B format parser / serializer.
//!
//! Annex.B format construction
//! ```text
//! ┌───────────────────────────────────────────┐
//! │  Start Code(0x00_00_01 or 0x00_00_00_01)  │
//! ├───────────────────────────────────────────┤
//! │  NAL Unit (variable)                      │
//! └───────────────────────────────────────────┘
//! ```

use crate::{error::Error, nal::NalUnit};

const START_CODE_3B: &[u8; 3] = &[0x00, 0x00, 0x01];
const START_CODE_4B: &[u8; 4] = &[0x00, 0x00, 0x00, 0x01];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StartCode {
    ThreeBytes, // [0x00, 0x00, 0x01]
    FourBytes,  // [0x00, 0x00, 0x00, 0x01]
}

impl StartCode {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            StartCode::ThreeBytes => START_CODE_3B,
            StartCode::FourBytes => START_CODE_4B,
        }
    }
}

#[derive(Debug)]
pub struct AnnexB {
    pub start_code: StartCode,
    pub nal_unit: NalUnit,
}

impl AnnexB {
    /// Serialize the start code and NAL unit back into a byte stream
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.start_code.as_bytes());
        buf.extend_from_slice(&self.nal_unit.to_bytes());
        buf
    }

    /// Parse a single Annex B entry from a byte slice starting with a start code
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 4 {
            return Err(Error::DataTooShort);
        }

        if data.starts_with(START_CODE_3B) {
            let nal_unit = NalUnit::parse(&data[3..])?;
            Ok(Self {
                start_code: StartCode::ThreeBytes,
                nal_unit,
            })
        } else if data.starts_with(START_CODE_4B) {
            let nal_unit = NalUnit::parse(&data[4..])?;
            Ok(Self {
                start_code: StartCode::FourBytes,
                nal_unit,
            })
        } else {
            let value = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            Err(Error::InvalidStartCode(value))
        }
    }
}

/// Parse a byte stream into multiple Annex B entries by splitting at start code boundaries
pub fn parse_all(data: &[u8]) -> Result<Vec<AnnexB>, Error> {
    let mut result = Vec::new();
    let positions = find_start_codes(data);

    for i in 0..positions.len() {
        let start = positions[i];
        let end = if i + 1 < positions.len() {
            positions[i + 1]
        } else {
            data.len()
        };
        result.push(AnnexB::parse(&data[start..end])?);
    }

    Ok(result)
}

/// Scan the byte slice and return the positions of all start codes (3-byte or 4-byte)
fn find_start_codes(data: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        if data[i] == 0x00 && data[i + 1] == 0x00 {
            // 0x00, 0x00, 0x01
            if data[i + 2] == 0x01 {
                positions.push(i);
                i += 3;
            // 0x00, 0x00, 0x00, 0x01
            } else if i + 3 < data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                positions.push(i);
                i += 4;
            } else {
                i += 1
            }
        } else {
            i += 1
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nal::NalUnitType;

    // AUD NAL: header(type=35, layer=0, tid+1=1) = [0x46, 0x01], payload pic_type=PI = [0x30]
    const AUD_NAL: &[u8] = &[0x46, 0x01, 0x30];

    #[test]
    fn test_annex_b_roundtrip() {
        let mut stream = Vec::new();
        stream.extend_from_slice(START_CODE_4B);
        stream.extend_from_slice(AUD_NAL);
        stream.extend_from_slice(START_CODE_3B);
        stream.extend_from_slice(&[0x42, 0x01, 0xde, 0xad]); // SPS(33) 相当の未知ボディ

        let entries = parse_all(&stream).expect("parse_all");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].start_code, StartCode::FourBytes);
        assert_eq!(
            entries[0].nal_unit.header.nal_unit_type,
            NalUnitType::AudNut
        );
        assert_eq!(entries[1].start_code, StartCode::ThreeBytes);
        assert_eq!(
            entries[1].nal_unit.header.nal_unit_type,
            NalUnitType::SpsNut
        );

        let rebuilt: Vec<u8> = entries.iter().flat_map(|e| e.to_bytes()).collect();
        assert_eq!(rebuilt, stream);
    }

    #[test]
    fn test_invalid_start_code() {
        assert!(matches!(
            AnnexB::parse(&[0x11, 0x22, 0x33, 0x44]),
            Err(Error::InvalidStartCode(0x11223344))
        ));
    }
}
