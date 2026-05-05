//! AVCDecoderConfigurationRecord (avcC) parser / serializer.

use crate::{
    error::Error,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug, Clone)]
pub struct AvccConfig {
    pub configuration_version: u8,
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: u8,
    pub sps_nalus: Vec<Vec<u8>>,
    pub pps_nalus: Vec<Vec<u8>>,
    pub extension: Option<Extension>,
}

#[derive(Debug, Clone)]
pub struct Extension {
    pub chroma_format: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub sps_ext_nalus: Vec<Vec<u8>>,
}

impl AvccConfig {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        writer.write_bits(self.configuration_version as u32, 8);
        writer.write_bits(self.avc_profile_indication as u32, 8);
        writer.write_bits(self.profile_compatibility as u32, 8);
        writer.write_bits(self.avc_level_indication as u32, 8);

        writer.write_bits(0b11_1111, 6);
        writer.write_bits((self.length_size_minus_one & 0b11) as u32, 2);

        writer.write_bits(0b111, 3);
        writer.write_bits(self.sps_nalus.len() as u32, 5);
        write_nal_array(&mut writer, &self.sps_nalus);

        writer.write_bits(self.pps_nalus.len() as u32, 8);
        write_nal_array(&mut writer, &self.pps_nalus);

        if let Some(ext) = &self.extension {
            writer.write_bits(0b11_1111, 6);
            writer.write_bits((ext.chroma_format & 0b11) as u32, 2);
            writer.write_bits(0b1_1111, 5);
            writer.write_bits((ext.bit_depth_luma_minus8 & 0b111) as u32, 3);
            writer.write_bits(0b1_1111, 5);
            writer.write_bits((ext.bit_depth_chroma_minus8 & 0b111) as u32, 3);
            writer.write_bits(ext.sps_ext_nalus.len() as u32, 8);
            write_nal_array(&mut writer, &ext.sps_ext_nalus);
        }

        writer.finish()
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 7 {
            return Err(Error::DataTooShort);
        }
        let mut reader = BitstreamReader::new(data);
        let configuration_version = reader.read_bits(8)? as u8;
        let avc_profile_indication = reader.read_bits(8)? as u8;
        let profile_compatibility = reader.read_bits(8)? as u8;
        let avc_level_indication = reader.read_bits(8)? as u8;
        let _reserved = reader.read_bits(6)?;
        let length_size_minus_one = reader.read_bits(2)? as u8;
        let _reserved = reader.read_bits(3)?;

        let num_of_sps = reader.read_bits(5)? as usize;
        let sps_nalus = read_nal_array(&mut reader, num_of_sps)?;

        let num_of_pps = reader.read_bits(8)? as usize;
        let pps_nalus = read_nal_array(&mut reader, num_of_pps)?;

        let extension = if !matches!(avc_profile_indication, 66 | 77 | 88) {
            let _reserved = reader.read_bits(6)?;
            let chroma_format = reader.read_bits(2)? as u8;
            let _reserved = reader.read_bits(5)?;
            let bit_depth_luma_minus8 = reader.read_bits(3)? as u8;
            let _reserved = reader.read_bits(5)?;
            let bit_depth_chroma_minus8 = reader.read_bits(3)? as u8;
            let num_of_sps_ext = reader.read_bits(8)? as usize;
            let sps_ext_nalus = read_nal_array(&mut reader, num_of_sps_ext)?;
            Some(Extension {
                chroma_format,
                bit_depth_luma_minus8,
                bit_depth_chroma_minus8,
                sps_ext_nalus,
            })
        } else {
            None
        };

        Ok(Self {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            sps_nalus,
            pps_nalus,
            extension,
        })
    }

    /// Length prefix size used in mp4 sample bytes.
    pub fn nal_length_size(&self) -> usize {
        (self.length_size_minus_one as usize) + 1
    }
}

fn write_nal_array(writer: &mut BitstreamWriter, nalus: &[Vec<u8>]) {
    for nal in nalus {
        writer.write_bits(nal.len() as u32, 16);
        for &b in nal {
            writer.write_bits(b as u32, 8);
        }
    }
}

fn read_nal_array(reader: &mut BitstreamReader, count: usize) -> Result<Vec<Vec<u8>>, Error> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = reader.read_bits(16)? as usize;
        out.push(reader.read_slice(len)?.to_vec());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_roundtrip(cfg: &AvccConfig) {
        let bytes = cfg.to_bytes();
        let parsed = AvccConfig::parse(&bytes).expect("parse");
        let bytes2 = parsed.to_bytes();
        assert_eq!(bytes, bytes2, "byte-exact roundtrip failed");
        assert_eq!(parsed.configuration_version, cfg.configuration_version);
        assert_eq!(parsed.avc_profile_indication, cfg.avc_profile_indication);
        assert_eq!(parsed.profile_compatibility, cfg.profile_compatibility);
        assert_eq!(parsed.avc_level_indication, cfg.avc_level_indication);
        assert_eq!(parsed.length_size_minus_one, cfg.length_size_minus_one);
        assert_eq!(parsed.sps_nalus, cfg.sps_nalus);
        assert_eq!(parsed.pps_nalus, cfg.pps_nalus);
        match (&parsed.extension, &cfg.extension) {
            (Some(p), Some(c)) => {
                assert_eq!(p.chroma_format, c.chroma_format);
                assert_eq!(p.bit_depth_luma_minus8, c.bit_depth_luma_minus8);
                assert_eq!(p.bit_depth_chroma_minus8, c.bit_depth_chroma_minus8);
                assert_eq!(p.sps_ext_nalus, c.sps_ext_nalus);
            }
            (None, None) => {}
            (a, b) => panic!("extension mismatch: {:?} vs {:?}", a, b),
        }
    }

    #[test]
    fn extension_with_sps_ext_roundtrip() {
        let cfg = AvccConfig {
            configuration_version: 1,
            avc_profile_indication: 1,
            profile_compatibility: 0x00,
            avc_level_indication: 40,
            length_size_minus_one: 3,
            sps_nalus: vec![vec![0x67, 0x6E, 0x00, 0x28]],
            pps_nalus: vec![vec![0x68, 0xEB, 0xE3]],
            extension: Some(Extension {
                chroma_format: 1,
                bit_depth_luma_minus8: 2,
                bit_depth_chroma_minus8: 2,
                sps_ext_nalus: vec![vec![0x6D, 0xAA, 0xBB]],
            }),
        };
        assert_roundtrip(&cfg);
    }

    #[test]
    fn parse_too_short() {
        let data = [0x01, 0x42, 0xC0];
        let err = AvccConfig::parse(&data).unwrap_err();
        assert_eq!(err, Error::DataTooShort);
    }
}
