use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Stz2 {
    pub header: FullBoxHeader,
    pub field_size: u8,
    pub sample_count: u32,
    pub entry_sizes: Vec<u32>,
}

impl BaseBox for Stz2 {
    const BOX_TYPE: BoxType = BoxType::Stz2;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(0, 24); // reserved
        writer.write_bits(self.field_size as u32, 8);
        writer.write_bits(self.sample_count, 32);
        for &s in &self.entry_sizes {
            writer.write_bits(s, self.field_size);
        }
        // For field_size == 4 with odd sample_count, pad 4 bits to byte alignment.
        if self.field_size == 4 && self.entry_sizes.len() % 2 == 1 {
            writer.write_bits(0, 4);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let _ = reader.read_bits(24)?; // reserved
        let field_size = reader.read_bits(8)? as u8;
        if !matches!(field_size, 4 | 8 | 16) {
            return Err(Error::UnsupportedValue {
                field: "stz2.field_size",
                value: field_size as u32,
            });
        }
        let sample_count = reader.read_bits(32)?;
        let mut entry_sizes = Vec::with_capacity(sample_count as usize);
        for _ in 0..sample_count {
            entry_sizes.push(reader.read_bits(field_size)?);
        }
        Ok(Self {
            header,
            field_size,
            sample_count,
            entry_sizes,
        })
    }
}

impl FullBox for Stz2 {
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
    fn test_stz2_field16_roundtrip() {
        let src = Stz2 {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            field_size: 16,
            sample_count: 2,
            entry_sizes: vec![100, 200],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stz2::parse(&bytes).expect("parse stz2");
        assert_eq!(parsed.entry_sizes, vec![100, 200]);
    }

    #[test]
    fn test_stz2_field8_roundtrip() {
        let src = Stz2 {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            field_size: 8,
            sample_count: 4,
            entry_sizes: vec![1, 2, 3, 4],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stz2::parse(&bytes).expect("parse stz2");
        assert_eq!(parsed.entry_sizes, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_stz2_field4_odd_sample_count_roundtrip() {
        let src = Stz2 {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            field_size: 4,
            sample_count: 3,
            entry_sizes: vec![1, 2, 15],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stz2::parse(&bytes).expect("parse stz2");
        assert_eq!(parsed.entry_sizes, vec![1, 2, 15]);
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_stz2_invalid_field_size_errors() {
        // Build raw bytes manually with field_size = 12 (reserved).
        let mut w = ByteWriter::new();
        FullBoxHeader {
            version: 0,
            flags: 0,
        }
        .to_bytes(&mut w);
        w.write_bits(0, 24); // reserved
        w.write_bits(12, 8); // field_size = 12 (invalid)
        w.write_bits(0, 32); // sample_count = 0
        let bytes = w.finish();
        let err = Stz2::parse(&bytes).expect_err("must reject invalid field_size");
        assert_eq!(
            err,
            Error::UnsupportedValue {
                field: "stz2.field_size",
                value: 12,
            }
        );
    }
}
