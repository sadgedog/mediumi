use crate::error::Error;
use mediumi_util::bitstream::{BitstreamReader, BitstreamWriter};

#[derive(Debug, PartialEq)]
pub enum NalUnitType {
    TrailN,
    TrailR,
    TsaN,
    TsaR,
    StsaN,
    StsaR,
    RadlN,
    RadlR,
    RaslN,
    RaslR,
    RsvVclN10,
    RsvVclR11,
    RsvVclN12,
    RsvVclR13,
    RsvVclN14,
    RsvVclR15,
    BlaWLp,
    BlaWRadl,
    BlaNLp,
    IdrWRadl,
    IdrNLp,
    CraNut,
    RsvIrapVcl22,
    RsvIrapVcl23,
    RsvVcl24,
    RsvVcl25,
    RsvVcl26,
    RsvVcl27,
    RsvVcl28,
    RsvVcl29,
    RsvVcl30,
    RsvVcl31,
    VpsNut,
    SpsNut,
    PpsNut,
    AudNut,
    EosNut,
    EobNut,
    FdNut,
    PrefixSeiNut,
    SuffixSeiNut,
    RsvNvcl41,
    RsvNvcl42,
    RsvNvcl43,
    RsvNvcl44,
    RsvNvcl45,
    RsvNvcl46,
    RsvNvcl47,
    Unspecified(u8),
}

impl From<u8> for NalUnitType {
    fn from(value: u8) -> Self {
        match value {
            0 => NalUnitType::TrailN,
            1 => NalUnitType::TrailR,
            2 => NalUnitType::TsaN,
            3 => NalUnitType::TsaR,
            4 => NalUnitType::StsaN,
            5 => NalUnitType::StsaR,
            6 => NalUnitType::RadlN,
            7 => NalUnitType::RadlR,
            8 => NalUnitType::RaslN,
            9 => NalUnitType::RaslR,
            10 => NalUnitType::RsvVclN10,
            11 => NalUnitType::RsvVclR11,
            12 => NalUnitType::RsvVclN12,
            13 => NalUnitType::RsvVclR13,
            14 => NalUnitType::RsvVclN14,
            15 => NalUnitType::RsvVclR15,
            16 => NalUnitType::BlaWLp,
            17 => NalUnitType::BlaWRadl,
            18 => NalUnitType::BlaNLp,
            19 => NalUnitType::IdrWRadl,
            20 => NalUnitType::IdrNLp,
            21 => NalUnitType::CraNut,
            22 => NalUnitType::RsvIrapVcl22,
            23 => NalUnitType::RsvIrapVcl23,
            24 => NalUnitType::RsvVcl24,
            25 => NalUnitType::RsvVcl25,
            26 => NalUnitType::RsvVcl26,
            27 => NalUnitType::RsvVcl27,
            28 => NalUnitType::RsvVcl28,
            29 => NalUnitType::RsvVcl29,
            30 => NalUnitType::RsvVcl30,
            31 => NalUnitType::RsvVcl31,
            32 => NalUnitType::VpsNut,
            33 => NalUnitType::SpsNut,
            34 => NalUnitType::PpsNut,
            35 => NalUnitType::AudNut,
            36 => NalUnitType::EosNut,
            37 => NalUnitType::EobNut,
            38 => NalUnitType::FdNut,
            39 => NalUnitType::PrefixSeiNut,
            40 => NalUnitType::SuffixSeiNut,
            41 => NalUnitType::RsvNvcl41,
            42 => NalUnitType::RsvNvcl42,
            43 => NalUnitType::RsvNvcl43,
            44 => NalUnitType::RsvNvcl44,
            45 => NalUnitType::RsvNvcl45,
            46 => NalUnitType::RsvNvcl46,
            47 => NalUnitType::RsvNvcl47,
            _ => NalUnitType::Unspecified(value),
        }
    }
}

impl From<&NalUnitType> for u8 {
    fn from(nal_unit_type: &NalUnitType) -> Self {
        match nal_unit_type {
            NalUnitType::TrailN => 0,
            NalUnitType::TrailR => 1,
            NalUnitType::TsaN => 2,
            NalUnitType::TsaR => 3,
            NalUnitType::StsaN => 4,
            NalUnitType::StsaR => 5,
            NalUnitType::RadlN => 6,
            NalUnitType::RadlR => 7,
            NalUnitType::RaslN => 8,
            NalUnitType::RaslR => 9,
            NalUnitType::RsvVclN10 => 10,
            NalUnitType::RsvVclR11 => 11,
            NalUnitType::RsvVclN12 => 12,
            NalUnitType::RsvVclR13 => 13,
            NalUnitType::RsvVclN14 => 14,
            NalUnitType::RsvVclR15 => 15,
            NalUnitType::BlaWLp => 16,
            NalUnitType::BlaWRadl => 17,
            NalUnitType::BlaNLp => 18,
            NalUnitType::IdrWRadl => 19,
            NalUnitType::IdrNLp => 20,
            NalUnitType::CraNut => 21,
            NalUnitType::RsvIrapVcl22 => 22,
            NalUnitType::RsvIrapVcl23 => 23,
            NalUnitType::RsvVcl24 => 24,
            NalUnitType::RsvVcl25 => 25,
            NalUnitType::RsvVcl26 => 26,
            NalUnitType::RsvVcl27 => 27,
            NalUnitType::RsvVcl28 => 28,
            NalUnitType::RsvVcl29 => 29,
            NalUnitType::RsvVcl30 => 30,
            NalUnitType::RsvVcl31 => 31,
            NalUnitType::VpsNut => 32,
            NalUnitType::SpsNut => 33,
            NalUnitType::PpsNut => 34,
            NalUnitType::AudNut => 35,
            NalUnitType::EosNut => 36,
            NalUnitType::EobNut => 37,
            NalUnitType::FdNut => 38,
            NalUnitType::PrefixSeiNut => 39,
            NalUnitType::SuffixSeiNut => 40,
            NalUnitType::RsvNvcl41 => 41,
            NalUnitType::RsvNvcl42 => 42,
            NalUnitType::RsvNvcl43 => 43,
            NalUnitType::RsvNvcl44 => 44,
            NalUnitType::RsvNvcl45 => 45,
            NalUnitType::RsvNvcl46 => 46,
            NalUnitType::RsvNvcl47 => 47,
            NalUnitType::Unspecified(v) => *v,
        }
    }
}

#[derive(Debug)]
pub struct Header {
    pub forbidden_zero_bit: u8,     // 1b
    pub nal_unit_type: NalUnitType, // 6b
    pub nuh_layer_id: u8,           // 6b
    pub nuh_temporal_id_plus1: u8,  // 3b
}

impl Header {
    // Serialize NAL Unit header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        writer.write_bits(self.forbidden_zero_bit as u32, 1);
        writer.write_bits(u8::from(&self.nal_unit_type) as u32, 6);
        writer.write_bits(self.nuh_layer_id as u32, 6);
        writer.write_bits(self.nuh_temporal_id_plus1 as u32, 3);

        writer.finish()
    }

    // Parse NAL Unit header
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 2 {
            return Err(Error::DataTooShort);
        }

        let mut reader = BitstreamReader::new(data);
        let forbidden_zero_bit = reader.read_bits(1)? as u8;
        if forbidden_zero_bit != 0 {
            return Err(Error::InvalidForbiddenZeroBit);
        }

        let nal_unit_type = reader.read_bits(6)? as u8;
        let nuh_layer_id = reader.read_bits(6)? as u8;
        let nuh_temporal_id_plus1 = reader.read_bits(3)? as u8;

        Ok(Self {
            forbidden_zero_bit,
            nal_unit_type: NalUnitType::from(nal_unit_type),
            nuh_layer_id,
            nuh_temporal_id_plus1,
        })
    }
}

#[derive(Debug)]
pub struct NalUnit {
    pub header: Header,
    pub rbsp: Vec<u8>,
}

impl NalUnit {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut nal_unit_bytes = self.header.to_bytes();
        nal_unit_bytes.extend_from_slice(&self.rbsp);
        nal_unit_bytes
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let header = Header::parse(data)?;
        let rbsp = data[2..].to_vec();

        Ok(Self { header, rbsp })
    }
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

/// Insert emulation prevention bytes: any 0x00 0x00 followed by 0x00..=0x03
/// gets a 0x03 inserted so the payload never contains a start-code prefix.
/// e.g. [0x00, 0x00, 0x01] -> [0x00, 0x00, 0x03, 0x01]
pub fn insert_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(data.len());
    let mut zeros = 0usize;

    for &b in data {
        if zeros >= 2 && b <= 0x03 {
            buf.push(0x03);
            zeros = 0;
        }
        buf.push(b);
        if b == 0x00 {
            zeros += 1;
        } else {
            zeros = 0;
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nal_unit_header_roundtrip() {
        let header = Header {
            forbidden_zero_bit: 0,
            nal_unit_type: NalUnitType::TrailN,
            nuh_layer_id: 0,
            nuh_temporal_id_plus1: 1,
        };

        let bytes = header.to_bytes();
        let parsed_header = Header::parse(&bytes).expect("Failed to parse NAL unit header");

        assert_eq!(header.forbidden_zero_bit, parsed_header.forbidden_zero_bit);
        assert_eq!(header.nal_unit_type, parsed_header.nal_unit_type);
        assert_eq!(header.nuh_layer_id, parsed_header.nuh_layer_id);
        assert_eq!(
            header.nuh_temporal_id_plus1,
            parsed_header.nuh_temporal_id_plus1
        );
    }

    #[test]
    fn test_emulation_prevention_roundtrip() {
        let sodb = vec![0x00, 0x00, 0x01, 0xaa, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff];
        let with_epb = insert_emulation_prevention_bytes(&sodb);
        assert!(
            !with_epb
                .windows(3)
                .any(|w| w[0] == 0 && w[1] == 0 && w[2] <= 1)
        );
        let restored = remove_emulation_prevention_bytes(&with_epb);
        assert_eq!(sodb, restored);
    }
}
