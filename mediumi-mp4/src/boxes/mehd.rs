use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Mehd {
    pub header: FullBoxHeader,
    pub fragment_duration: u64,
}

impl BaseBox for Mehd {
    const BOX_TYPE: BoxType = BoxType::Mehd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        if self.header.version == 1 {
            writer.write_bits((self.fragment_duration >> 32) as u32, 32);
            writer.write_bits(self.fragment_duration as u32, 32);
        } else {
            writer.write_bits(self.fragment_duration as u32, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let fragment_duration = if header.version == 1 {
            let high = (reader.read_bits(32)? as u64) << 32;
            let low = reader.read_bits(32)? as u64;
            high | low
        } else {
            reader.read_bits(32)? as u64
        };
        Ok(Self {
            header,
            fragment_duration,
        })
    }
}

impl FullBox for Mehd {
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
    fn test_mehd_v0_roundtrip() {
        let src = Mehd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            fragment_duration: 12345,
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Mehd::parse(&bytes).expect("parse mehd");
        assert_eq!(parsed.fragment_duration, 12345);
    }
}
