use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug, Clone)]
pub struct CttsEntry {
    pub sample_count: u32,
    pub sample_offset: i32,
}

#[derive(Debug)]
pub struct Ctts {
    pub header: FullBoxHeader,
    pub entries: Vec<CttsEntry>,
}

impl BaseBox for Ctts {
    const BOX_TYPE: BoxType = BoxType::Ctts;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            writer.write_bits(e.sample_count, 32);
            writer.write_bits(e.sample_offset as u32, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let sample_count = reader.read_bits(32)?;
            let sample_offset = reader.read_bits(32)? as i32;
            let _ = header.version;
            entries.push(CttsEntry {
                sample_count,
                sample_offset,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Ctts {
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
    fn test_ctts_roundtrip() {
        let src = Ctts {
            header: FullBoxHeader {
                version: 1,
                flags: 0,
            },
            entries: vec![CttsEntry {
                sample_count: 1,
                sample_offset: -512,
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Ctts::parse(&bytes).expect("parse ctts");
        assert_eq!(parsed.entries[0].sample_offset, -512);
    }
}
