use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Padb {
    pub header: FullBoxHeader,
    pub sample_count: u32,
    pub padding_bits: Vec<u8>,
}

impl BaseBox for Padb {
    const BOX_TYPE: BoxType = BoxType::Padb;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.sample_count, 32);
        let pairs = self.sample_count.div_ceil(2);
        for i in 0..pairs {
            let i1 = (i * 2) as usize;
            let i2 = i1 + 1;
            let p1 = *self.padding_bits.get(i1).unwrap_or(&0) & 0x7;
            let p2 = *self.padding_bits.get(i2).unwrap_or(&0) & 0x7;
            writer.write_bits(0, 1); // reserved
            writer.write_bits(p1 as u32, 3);
            writer.write_bits(0, 1); // reserved
            writer.write_bits(p2 as u32, 3);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sample_count = reader.read_bits(32)?;
        let pairs = sample_count.div_ceil(2);
        let mut padding_bits = Vec::with_capacity(sample_count as usize);
        for i in 0..pairs {
            let _ = reader.read_bits(1)?; // reserved
            let p1 = reader.read_bits(3)? as u8;
            padding_bits.push(p1);
            if i * 2 + 1 < sample_count {
                let _ = reader.read_bits(1)?; // reserved
                let p2 = reader.read_bits(3)? as u8;
                padding_bits.push(p2);
            }
        }
        Ok(Self {
            header,
            sample_count,
            padding_bits,
        })
    }
}

impl FullBox for Padb {
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
    fn test_padb_roundtrip() {
        let src = Padb {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            sample_count: 4,
            padding_bits: vec![1, 2, 3, 4],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Padb::parse(&bytes).expect("parse padb");
        assert_eq!(parsed.padding_bits, vec![1, 2, 3, 4]);
    }
}
