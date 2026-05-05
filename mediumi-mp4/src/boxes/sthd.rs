use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Sthd {
    pub header: FullBoxHeader,
}

impl BaseBox for Sthd {
    const BOX_TYPE: BoxType = BoxType::Sthd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        Ok(Self { header })
    }
}

impl FullBox for Sthd {
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
    fn test_sthd_roundtrip() {
        let src = Sthd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 4);
        let _parsed = Sthd::parse(&bytes).expect("parse sthd");
    }
}
