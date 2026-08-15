use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct Stsz {
    pub header: FullBoxHeader,
    pub sample_size: u32,
    pub sample_count: u32,
    pub entry_sizes: Vec<u32>,
}

impl BaseBox for Stsz {
    const BOX_TYPE: BoxType = BoxType::Stsz;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.sample_size, 32);
        writer.write_bits(self.sample_count, 32);
        if self.sample_size == 0 {
            for &s in &self.entry_sizes {
                writer.write_bits(s, 32);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sample_size = reader.read_bits(32)?;
        let sample_count = reader.read_bits(32)?;
        let entry_sizes = if sample_size == 0 {
            let mut v = Vec::with_capacity(sample_count as usize);
            for _ in 0..sample_count {
                v.push(reader.read_bits(32)?);
            }
            v
        } else {
            Vec::new()
        };
        Ok(Self {
            header,
            sample_size,
            sample_count,
            entry_sizes,
        })
    }
}

impl FullBox for Stsz {
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
    fn test_stsz_variable_roundtrip() {
        let src = Stsz {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            sample_size: 0,
            sample_count: 3,
            entry_sizes: vec![100, 200, 150],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stsz::parse(&bytes).expect("parse stsz");
        assert_eq!(parsed.entry_sizes, vec![100, 200, 150]);
    }
}
