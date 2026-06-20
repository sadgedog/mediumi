use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Trex {
    pub header: FullBoxHeader,
    pub track_id: u32,
    pub default_sample_description_index: u32,
    pub default_sample_duration: u32,
    pub default_sample_size: u32,
    pub default_sample_flags: u32,
}

impl BaseBox for Trex {
    const BOX_TYPE: BoxType = BoxType::Trex;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.track_id, 32);
        writer.write_bits(self.default_sample_description_index, 32);
        writer.write_bits(self.default_sample_duration, 32);
        writer.write_bits(self.default_sample_size, 32);
        writer.write_bits(self.default_sample_flags, 32);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        Ok(Self {
            header,
            track_id: reader.read_bits(32)?,
            default_sample_description_index: reader.read_bits(32)?,
            default_sample_duration: reader.read_bits(32)?,
            default_sample_size: reader.read_bits(32)?,
            default_sample_flags: reader.read_bits(32)?,
        })
    }
}

impl FullBox for Trex {
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
    fn test_trex_roundtrip() {
        let src = Trex {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            track_id: 1,
            default_sample_description_index: 1,
            default_sample_duration: 1024,
            default_sample_size: 0,
            default_sample_flags: 0x010_00000,
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Trex::parse(&bytes).expect("parse trex");
        assert_eq!(parsed.default_sample_duration, 1024);
    }
}
