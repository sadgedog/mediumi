use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Hmhd {
    pub header: FullBoxHeader,
    pub max_pdu_size: u16,
    pub avg_pdu_size: u16,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,
}

impl BaseBox for Hmhd {
    const BOX_TYPE: BoxType = BoxType::Hmhd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.max_pdu_size as u32, 16);
        writer.write_bits(self.avg_pdu_size as u32, 16);
        writer.write_bits(self.max_bitrate, 32);
        writer.write_bits(self.avg_bitrate, 32);
        writer.write_bits(0, 32); // reserved
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let max_pdu_size = reader.read_bits(16)? as u16;
        let avg_pdu_size = reader.read_bits(16)? as u16;
        let max_bitrate = reader.read_bits(32)?;
        let avg_bitrate = reader.read_bits(32)?;
        let _ = reader.read_bits(32)?; // reserved
        Ok(Self {
            header,
            max_pdu_size,
            avg_pdu_size,
            max_bitrate,
            avg_bitrate,
        })
    }
}

impl FullBox for Hmhd {
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
    fn test_hmhd_roundtrip() {
        let src = Hmhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            max_pdu_size: 1500,
            avg_pdu_size: 1400,
            max_bitrate: 100_000,
            avg_bitrate: 80_000,
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 20);
        let parsed = Hmhd::parse(&bytes).expect("parse hmhd");
        assert_eq!(parsed.max_pdu_size, 1500);
        assert_eq!(parsed.max_bitrate, 100_000);
    }
}
