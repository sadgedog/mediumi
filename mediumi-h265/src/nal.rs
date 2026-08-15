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
            _ => NalUnitType::Unspecified(value),
        }
    }
}

impl From<&NalUnitType> for u8 {
    fn from(nal_unit_type: &NalUnitType) -> Self {
        match nal_unit_type {
            NalUnitType::TrailN => 0,
            // TODO: Add all other NAL unit types as per H.265 specification
            _ => match nal_unit_type {
                NalUnitType::Unspecified(value) => *value,
                _ => 0,
            },
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
}
