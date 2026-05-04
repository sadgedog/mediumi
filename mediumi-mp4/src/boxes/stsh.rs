use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug, Clone, PartialEq)]
pub struct StshEntry {
    pub shadowed_sample_number: u32,
    pub sync_sample_number: u32,
}

#[derive(Debug)]
pub struct Stsh {
    pub header: FullBoxHeader,
    pub entries: Vec<StshEntry>,
}

impl BaseBox for Stsh {
    const BOX_TYPE: BoxType = BoxType::Stsh;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            writer.write_bits(e.shadowed_sample_number, 32);
            writer.write_bits(e.sync_sample_number, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            entries.push(StshEntry {
                shadowed_sample_number: reader.read_bits(32)?,
                sync_sample_number: reader.read_bits(32)?,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Stsh {
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
    fn test_stsh_roundtrip() {
        let src = Stsh {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![StshEntry {
                shadowed_sample_number: 1,
                sync_sample_number: 2,
            }],
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stsh::parse(&bytes).expect("parse stsh");
        assert_eq!(
            parsed.entries,
            vec![StshEntry {
                shadowed_sample_number: 1,
                sync_sample_number: 2
            }]
        );
    }
}
