use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct Stdp {
    pub header: FullBoxHeader,
    pub priorities: Vec<u16>,
}

impl BaseBox for Stdp {
    const BOX_TYPE: BoxType = BoxType::Stdp;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        for &p in &self.priorities {
            writer.write_bits(p as u32, 16);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let mut priorities = Vec::new();
        while reader.remaining_bits() >= 16 {
            priorities.push(reader.read_bits(16)? as u16);
        }
        Ok(Self { header, priorities })
    }
}

impl FullBox for Stdp {
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
    fn test_stdp_roundtrip() {
        let src = Stdp {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            priorities: vec![100, 200, 300],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stdp::parse(&bytes).expect("parse stdp");
        assert_eq!(parsed.priorities, vec![100, 200, 300]);
    }
}
