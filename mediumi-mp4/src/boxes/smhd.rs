use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Smhd {
    pub header: FullBoxHeader,
    pub balance: i16,
}

impl BaseBox for Smhd {
    const BOX_TYPE: BoxType = BoxType::Smhd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.balance as u16 as u32, 16);
        writer.write_bits(0, 16); // reserved
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let balance = reader.read_bits(16)? as i16;
        let _ = reader.read_bits(16)?; // reserved = 0
        Ok(Self { header, balance })
    }
}

impl FullBox for Smhd {
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
    fn test_smhd_roundtrip() {
        let src = Smhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            balance: 0,
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 8);
        let parsed = Smhd::parse(&bytes).expect("parse smhd");
        assert_eq!(parsed.balance, 0);
    }
}
