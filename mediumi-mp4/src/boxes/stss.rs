use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Stss {
    pub header: FullBoxHeader,
    pub sample_numbers: Vec<u32>,
}

impl BaseBox for Stss {
    const BOX_TYPE: BoxType = BoxType::Stss;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.sample_numbers.len() as u32, 32);
        for &n in &self.sample_numbers {
            writer.write_bits(n, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut sample_numbers = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            sample_numbers.push(reader.read_bits(32)?);
        }
        Ok(Self {
            header,
            sample_numbers,
        })
    }
}

impl FullBox for Stss {
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
    fn test_stss_roundtrip() {
        let src = Stss {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            sample_numbers: vec![100, 200, 300],
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stss::parse(&bytes).expect("parse stss");
        assert_eq!(parsed.sample_numbers, vec![100, 200, 300]);
    }
}
