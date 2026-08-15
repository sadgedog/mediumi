use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug, Clone)]
pub struct StscEntry {
    pub first_chunk: u32,
    pub samples_per_chunk: u32,
    pub sample_description_index: u32,
}

#[derive(Debug)]
pub struct Stsc {
    pub header: FullBoxHeader,
    pub entries: Vec<StscEntry>,
}

impl BaseBox for Stsc {
    const BOX_TYPE: BoxType = BoxType::Stsc;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            writer.write_bits(e.first_chunk, 32);
            writer.write_bits(e.samples_per_chunk, 32);
            writer.write_bits(e.sample_description_index, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entries.push(StscEntry {
                first_chunk: reader.read_bits(32)?,
                samples_per_chunk: reader.read_bits(32)?,
                sample_description_index: reader.read_bits(32)?,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Stsc {
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
    fn test_stsc_roundtrip() {
        let src = Stsc {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![StscEntry {
                first_chunk: 1,
                samples_per_chunk: 10,
                sample_description_index: 1,
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stsc::parse(&bytes).expect("parse stsc");
        assert_eq!(parsed.entries[0].samples_per_chunk, 10);
    }
}
