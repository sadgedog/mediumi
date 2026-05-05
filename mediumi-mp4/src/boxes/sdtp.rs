use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdtpEntry {
    pub is_leading: u8,
    pub sample_depends_on: u8,
    pub sample_is_depended_on: u8,
    pub sample_has_redundancy: u8,
}

#[derive(Debug)]
pub struct Sdtp {
    pub header: FullBoxHeader,
    pub entries: Vec<SdtpEntry>,
}

impl BaseBox for Sdtp {
    const BOX_TYPE: BoxType = BoxType::Sdtp;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        for e in &self.entries {
            writer.write_bits(e.is_leading as u32 & 0x3, 2);
            writer.write_bits(e.sample_depends_on as u32 & 0x3, 2);
            writer.write_bits(e.sample_is_depended_on as u32 & 0x3, 2);
            writer.write_bits(e.sample_has_redundancy as u32 & 0x3, 2);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let mut entries = Vec::new();
        while reader.remaining_bits() >= 8 {
            let is_leading = reader.read_bits(2)? as u8;
            let sample_depends_on = reader.read_bits(2)? as u8;
            let sample_is_depended_on = reader.read_bits(2)? as u8;
            let sample_has_redundancy = reader.read_bits(2)? as u8;
            entries.push(SdtpEntry {
                is_leading,
                sample_depends_on,
                sample_is_depended_on,
                sample_has_redundancy,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Sdtp {
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
    fn test_sdtp_roundtrip() {
        let src = Sdtp {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![SdtpEntry {
                is_leading: 0,
                sample_depends_on: 1,
                sample_is_depended_on: 2,
                sample_has_redundancy: 0,
            }],
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Sdtp::parse(&bytes).expect("parse sdtp");
        assert_eq!(
            parsed.entries,
            vec![SdtpEntry {
                is_leading: 0,
                sample_depends_on: 1,
                sample_is_depended_on: 2,
                sample_has_redundancy: 0,
            }]
        );
    }
}
