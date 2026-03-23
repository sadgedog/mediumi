//! Annex.B format parser
//!
//! Annex.B format construction
//! ```text
//! ┌───────────────────────────────────────────┐
//! │  Start Code(0x00_00_00 or 0x00_00_00_01)  │
//! ├───────────────────────────────────────────┤
//! │  NAL Unit (variable)                      │
//! └───────────────────────────────────────────┘
//! ```

use crate::{error::Error, nal::NalUnit};

const START_CODE_3B: &[u8; 3] = &[0x00, 0x00, 0x01];
const START_CODE_4B: &[u8; 4] = &[0x00, 0x00, 0x00, 0x01];

#[derive(Debug, PartialEq)]
pub enum StartCode {
    ThreeBytes, // [0x00, 0x00, 0x01]
    FourBytes,  // [0x00, 0x00, 0x00, 0x01]
}

impl StartCode {
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            StartCode::ThreeBytes => &[0x00, 0x00, 0x01],
            StartCode::FourBytes => &[0x00, 0x00, 0x00, 0x01],
        }
    }
}

#[derive(Debug)]
pub struct AnnexB {
    pub start_code: StartCode,
    pub nal_unit: NalUnit,
}

impl AnnexB {
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

    /// Serialize the start code and NAL unit back into a byte stream
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.start_code.as_bytes());
        buf.extend_from_slice(&self.nal_unit.to_bytes());
        buf
    }
}

/// Parse a PES payload into multiple Annex B entries by splitting at start code boundaries
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

    #[test]
    fn test_parser_with_3b_start_code() {
        let data = [0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB];
        let result = AnnexB::parse(&data).unwrap();
        assert_eq!(result.start_code, StartCode::ThreeBytes);
        assert_eq!(result.nal_unit.header.nal_unit_type, NalUnitType::IDR);
        assert_eq!(result.nal_unit.header.nal_ref_idc, 3);
        assert_eq!(result.nal_unit.rbsp, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_parser_with_4b_start_code() {
        let data = [0x00, 0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB];
        let result = AnnexB::parse(&data).unwrap();
        assert_eq!(result.start_code, StartCode::FourBytes);
        assert_eq!(result.nal_unit.header.nal_unit_type, NalUnitType::IDR);
        assert_eq!(result.nal_unit.header.nal_ref_idc, 3);
        assert_eq!(result.nal_unit.rbsp, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_invalid_start_code() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF, 0x65, 0xAA, 0xBB];
        let result = AnnexB::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_to_bytes() {
        let data = [0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB];
        let result = AnnexB::parse(&data).unwrap();
        let raw_data = AnnexB::to_bytes(&result);
        assert_eq!(raw_data, data);
    }
}
