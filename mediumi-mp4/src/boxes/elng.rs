use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct Elng {
    pub header: FullBoxHeader,
    pub extended_language: String,
}

impl BaseBox for Elng {
    const BOX_TYPE: BoxType = BoxType::Elng;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        for &b in self.extended_language.as_bytes() {
            writer.write_bits(b as u32, 8);
        }
        writer.write_bits(0, 8); // null terminator
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let mut bytes = Vec::new();
        loop {
            let b = reader.read_bits(8)? as u8;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        let extended_language = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        Ok(Self {
            header,
            extended_language,
        })
    }
}

impl FullBox for Elng {
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
    fn test_elng_roundtrip() {
        let src = Elng {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            extended_language: "en-US".to_string(),
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Elng::parse(&bytes).expect("parse elng");
        assert_eq!(parsed.extended_language, "en-US");
    }
}
