//! NAL Unit Parser
//!
//! NAL Unit construction
//! ```text
//! ┌───────────────────────────┐
//! │  NAL Unit Header(1 byte)  │
//! ├───────────────────────────┤
//! │  RBSP (variable)          │
//! └───────────────────────────┘
//! ```
//!
//! Nal Unit Header construction
//! ```text
//! ┌──────────────────────────────┐
//! │  forbidden_zero_bit (1 bit)  │ <- Must be 0
//! │  nal_ref_idc (2 bits)        │ <- Reference priority (0: disposable, 1..3: higher)
//! │  nal_unit_type (5 bits)      │ <- NAL unit type (e.g. 1: Non-IDR, 5: IDR, 7: SPS, 8: PPS)
//! └──────────────────────────────┘
//! ```

use crate::error::Error;

#[derive(Debug, PartialEq)]
pub enum NalUnitType {
    Unspecified(u8), // 0, 24..31
    NonIDR,          // 1
    SliceA,          // 2
    SliceB,          // 3
    SliceC,          // 4
    IDR,             // 5
    SEI,             // 6: Supplemental Enhancement Information
    SPS,             // 7: Sequence Parameter Set
    PPS,             // 8: Picture Parameter Set
    AUD,             // 9: Access Unit Delimiter
    EOSeq,           // 10: End Of Sequence
    EOStream,        // 11: End Of Stream
    FillerData,      // 12
    SPSExt,          // 13: SPS Extension
    PrefixNalUnit,   // 14
    SubsetSPS,       // 15
    DPS,             // 16: Depth Parameter Set
    Reserved(u8),    // 17..18, 22..23
    AUX,             // 19: Auxiliary Picture
    SliceExt,        // 20
    DepthExt,        // 21
    Unknown(u8),     // other invalid NAL Unit Type
}

impl From<u8> for NalUnitType {
    fn from(value: u8) -> NalUnitType {
        match value {
            0 | 24..=31 => NalUnitType::Unspecified(value),
            1 => NalUnitType::NonIDR,
            2 => NalUnitType::SliceA,
            3 => NalUnitType::SliceB,
            4 => NalUnitType::SliceC,
            5 => NalUnitType::IDR,
            6 => NalUnitType::SEI,
            7 => NalUnitType::SPS,
            8 => NalUnitType::PPS,
            9 => NalUnitType::AUD,
            10 => NalUnitType::EOSeq,
            11 => NalUnitType::EOStream,
            12 => NalUnitType::FillerData,
            13 => NalUnitType::SPSExt,
            14 => NalUnitType::PrefixNalUnit,
            15 => NalUnitType::SubsetSPS,
            16 => NalUnitType::DPS,
            17..=18 | 22..=23 => NalUnitType::Reserved(value),
            19 => NalUnitType::AUX,
            20 => NalUnitType::SliceExt,
            21 => NalUnitType::DepthExt,
            _ => NalUnitType::Unknown(value),
        }
    }
}

impl From<&NalUnitType> for u8 {
    fn from(value: &NalUnitType) -> u8 {
        match value {
            NalUnitType::Unspecified(v) => *v,
            NalUnitType::NonIDR => 1,
            NalUnitType::SliceA => 2,
            NalUnitType::SliceB => 3,
            NalUnitType::SliceC => 4,
            NalUnitType::IDR => 5,
            NalUnitType::SEI => 6,
            NalUnitType::SPS => 7,
            NalUnitType::PPS => 8,
            NalUnitType::AUD => 9,
            NalUnitType::EOSeq => 10,
            NalUnitType::EOStream => 11,
            NalUnitType::FillerData => 12,
            NalUnitType::SPSExt => 13,
            NalUnitType::PrefixNalUnit => 14,
            NalUnitType::SubsetSPS => 15,
            NalUnitType::DPS => 16,
            NalUnitType::Reserved(v) => *v,
            NalUnitType::AUX => 19,
            NalUnitType::SliceExt => 20,
            NalUnitType::DepthExt => 21,
            NalUnitType::Unknown(v) => *v,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Header {
    pub nal_ref_idc: u8,
    pub nal_unit_type: NalUnitType,
}

#[derive(Debug, PartialEq)]
pub struct NalUnit {
    pub header: Header,
    pub rbsp: Vec<u8>,
}

impl NalUnit {
    /// Parse the header byte and RBSP from a byte slice (without start code)
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.is_empty() {
            return Err(Error::DataTooShort);
        }

        let forbidden_zero_bit = (data[0] & 0b1000_0000) >> 7;
        if forbidden_zero_bit != 0 {
            return Err(Error::InvalidForbiddenZeroBit);
        }

        let nal_ref_idc = (data[0] & 0b0110_0000) >> 5;
        let nal_unit_type = NalUnitType::from(data[0] & 0b0001_1111);
        let header = Header {
            nal_ref_idc,
            nal_unit_type,
        };
        let rbsp = data[1..].to_vec();

        Ok(Self { header, rbsp })
    }

    /// Serialize the header and RBSP back into a byte slice
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.push(self.header.nal_ref_idc << 5 | u8::from(&self.header.nal_unit_type));
        buf.extend_from_slice(&self.rbsp);
        buf
    }

    /// Remove emulation prevention bytes (0x03) from RBSP to obtain the raw bitstream (SODB)
    /// e.g. [0x00, 0x00, 0x03, 0x01] -> [0x00, 0x00, 0x01]
    pub fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if i + 2 < data.len()
                && data[i] == 0x00
                && data[i + 1] == 0x00
                && data[i + 2] == 0x03
                && i + 3 < data.len()
                && data[i + 3] <= 0x03
            {
                buf.push(data[i]);
                buf.push(data[i + 1]);
                i += 3; // skip 0x03
            } else {
                buf.push(data[i]);
                i += 1;
            }
        }
        buf
    }

    /// Insert emulation prevention bytes (0x03) into raw bitstream to prevent start code collision
    /// e.g. [0x00, 0x00, 0x01] -> [0x00, 0x00, 0x03, 0x01]
    pub fn attach_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] <= 0x03 {
                buf.extend_from_slice(&[0x00, 0x00, 0x03]);
                buf.push(data[i + 2]);
                i += 3;
            } else {
                buf.push(data[i]);
                i += 1;
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let data = [0x65, 0xAA, 0xBB];
        let result = NalUnit::parse(&data).unwrap();
        assert_eq!(result.header.nal_ref_idc, 3);
        assert_eq!(result.header.nal_unit_type, NalUnitType::IDR);
        assert_eq!(result.header.nal_ref_idc, 3);
        assert_eq!(result.rbsp, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_data_too_short() {
        let data = [];
        let result = NalUnit::parse(&data);
        assert_eq!(result.err(), Some(Error::DataTooShort));
    }

    #[test]
    fn test_invalid_forbidden_zero_bit() {
        let data = [0x80, 0xAA, 0xBB];
        let result = NalUnit::parse(&data);
        assert_eq!(result.err(), Some(Error::InvalidForbiddenZeroBit));
    }

    #[test]
    fn test_to_bytes() {
        let data = [0x65, 0xAA, 0xBB];
        let result = NalUnit::parse(&data).unwrap();
        let raw_data = NalUnit::to_bytes(&result);
        assert_eq!(raw_data, data);
    }

    #[test]
    fn test_epb_round_trip() {
        let original = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00, 0x03,
        ];
        let with_epb = NalUnit::attach_emulation_prevention_bytes(&original);
        let restored = NalUnit::remove_emulation_prevention_bytes(&with_epb);
        assert_eq!(original, restored);
    }
}
