use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Co64 {
    pub header: FullBoxHeader,
    pub chunk_offsets: Vec<u64>,
}

impl BaseBox for Co64 {
    const BOX_TYPE: BoxType = BoxType::Co64;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.chunk_offsets.len() as u32, 32);
        for &o in &self.chunk_offsets {
            writer.write_bits((o >> 32) as u32, 32);
            writer.write_bits(o as u32, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut chunk_offsets = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let high = (reader.read_bits(32)? as u64) << 32;
            let low = reader.read_bits(32)? as u64;
            chunk_offsets.push(high | low);
        }
        Ok(Self {
            header,
            chunk_offsets,
        })
    }
}

impl FullBox for Co64 {
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
    fn test_co64_roundtrip() {
        let src = Co64 {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            chunk_offsets: vec![100, 200, 300],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Co64::parse(&bytes).expect("parse co64");
        assert_eq!(parsed.chunk_offsets, vec![100, 200, 300]);
    }
}
