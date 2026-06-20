use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Nmhd {
    pub header: FullBoxHeader,
}

impl BaseBox for Nmhd {
    const BOX_TYPE: BoxType = BoxType::Nmhd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        Ok(Self { header })
    }
}

impl FullBox for Nmhd {
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
    fn test_nmhd_roundtrip() {
        let src = Nmhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 4);
        let _parsed = Nmhd::parse(&bytes).expect("parse nmhd");
    }
}
