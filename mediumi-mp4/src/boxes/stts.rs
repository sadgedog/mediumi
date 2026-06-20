use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug, Clone)]
pub struct SttsEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

#[derive(Debug)]
pub struct Stts {
    pub header: FullBoxHeader,
    pub entries: Vec<SttsEntry>,
}

impl BaseBox for Stts {
    const BOX_TYPE: BoxType = BoxType::Stts;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            writer.write_bits(e.sample_count, 32);
            writer.write_bits(e.sample_delta, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entries.push(SttsEntry {
                sample_count: reader.read_bits(32)?,
                sample_delta: reader.read_bits(32)?,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Stts {
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
    fn test_stts_roundtrip() {
        let src = Stts {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![SttsEntry {
                sample_count: 100,
                sample_delta: 1024,
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stts::parse(&bytes).expect("parse stts");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].sample_delta, 1024);
    }
}
