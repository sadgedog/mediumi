use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Stco {
    pub header: FullBoxHeader,
    pub chunk_offsets: Vec<u32>,
}

impl BaseBox for Stco {
    const BOX_TYPE: BoxType = BoxType::Stco;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.chunk_offsets.len() as u32, 32);
        for &o in &self.chunk_offsets {
            writer.write_bits(o, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut chunk_offsets = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            chunk_offsets.push(reader.read_bits(32)?);
        }
        Ok(Self {
            header,
            chunk_offsets,
        })
    }
}

impl FullBox for Stco {
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
    fn test_stco_roundtrip() {
        let src = Stco {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            chunk_offsets: vec![100, 200, 300],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stco::parse(&bytes).expect("parse stco");
        assert_eq!(parsed.chunk_offsets, vec![100, 200, 300]);
    }
}
