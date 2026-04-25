use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Sgpd {
    pub header: FullBoxHeader,
    pub grouping_type: u32,
    pub default_length: Option<u32>,                   // only v==1
    pub default_sample_description_index: Option<u32>, // only v>=2
    pub entry_count: u32,
    pub entries: Vec<u8>, // raw data (TODO: parse detail)
}

impl BaseBox for Sgpd {
    const BOX_TYPE: BoxType = BoxType::Sgpd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.grouping_type, 32);
        if let Some(v) = self.default_length {
            writer.write_bits(v, 32);
        }
        if let Some(v) = self.default_sample_description_index {
            writer.write_bits(v, 32);
        }
        writer.write_bits(self.entry_count, 32);
        for b in &self.entries {
            writer.write_bits(*b as u32, 8);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let grouping_type = reader.read_bits(32)?;

        let default_length = if header.version == 1 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };

        let default_sample_description_index = if header.version >= 2 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };

        let entry_count = reader.read_bits(32)?;
        let (entries, _) = reader.read_remaining_bytes();

        Ok(Self {
            header,
            grouping_type,
            default_length,
            default_sample_description_index,
            entry_count,
            entries,
        })
    }
}

impl FullBox for Sgpd {
    fn version(&self) -> u8 {
        self.header.version
    }

    fn flags(&self) -> u32 {
        self.header.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgpd_v0_roundtrip() {
        // v=0: grouping_type='roll', entry_count=1, entry body=[0x00, 0x02] (roll_distance=2)
        let data = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags
            b'r', b'o', b'l', b'l', // grouping_type
            0x00, 0x00, 0x00, 0x01, // entry_count
            0x00, 0x02, // entry 0 raw
        ];
        let sgpd = Sgpd::parse(&data).expect("parse sgpd v0");
        assert_eq!(sgpd.header.version, 0);
        assert_eq!(sgpd.grouping_type, u32::from_be_bytes(*b"roll"));
        assert_eq!(sgpd.default_length, None);
        assert_eq!(sgpd.default_sample_description_index, None);
        assert_eq!(sgpd.entry_count, 1);
        assert_eq!(sgpd.entries, [0x00, 0x02]);

        let mut w = BitstreamWriter::new();
        sgpd.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_sgpd_v1_roundtrip() {
        // v=1: grouping_type='seig', default_length=20, entry_count=1
        let data = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags
            b's', b'e', b'i', b'g', // grouping_type
            0x00, 0x00, 0x00, 0x14, // default_length = 20
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry body (20B): reserved + is_protected + iv_size + KID(16)
            0x00, 0x00, 0x01, 0x08, //
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, //
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
        ];
        let sgpd = Sgpd::parse(&data).expect("parse sgpd v1");
        assert_eq!(sgpd.header.version, 1);
        assert_eq!(sgpd.grouping_type, u32::from_be_bytes(*b"seig"));
        assert_eq!(sgpd.default_length, Some(20));
        assert_eq!(sgpd.default_sample_description_index, None);
        assert_eq!(sgpd.entry_count, 1);
        assert_eq!(sgpd.entries.len(), 20);

        let mut w = BitstreamWriter::new();
        sgpd.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_sgpd_v2_roundtrip() {
        // v=2: default_sample_description_index
        let data = [
            0x02, // version = 2
            0x00, 0x00, 0x00, // flags
            b'r', b'a', b'p', b' ', // grouping_type
            0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            0x80, // entry 0 (raw: num_leading_samples_known=1, num_leading_samples=0)
        ];
        let sgpd = Sgpd::parse(&data).expect("parse sgpd v2");
        assert_eq!(sgpd.header.version, 2);
        assert_eq!(sgpd.default_length, None);
        assert_eq!(sgpd.default_sample_description_index, Some(1));
        assert_eq!(sgpd.entries, [0x80]);

        let mut w = BitstreamWriter::new();
        sgpd.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }
}
