use crate::{
    boxes::{BaseBox, Error, FullBox, FullBoxHeader},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Mfhd {
    pub header: FullBoxHeader,
    pub sequence_number: u32,
}

impl BaseBox for Mfhd {
    const BOX_TYPE: BoxType = BoxType::Mfhd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_bits(self.header.version as u32, 8);
        writer.write_bits(self.header.flags, 24);
        writer.write_bits(self.sequence_number, 32);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sequence_number = reader.read_bits(32)?;
        Ok(Self {
            header,
            sequence_number,
        })
    }
}

impl FullBox for Mfhd {
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
    fn test_mfhd_roundtrip() {
        // version=0, flags=0, sequence_number=0x01020304
        let data = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x01, 0x02, 0x03, 0x04, // sequence_number
        ];
        let mfhd = Mfhd::parse(&data).expect("failed to parse mfhd");
        assert_eq!(mfhd.header.version, 0);
        assert_eq!(mfhd.header.flags, 0);
        assert_eq!(mfhd.sequence_number, 0x01020304);

        let mut writer = BitstreamWriter::new();
        mfhd.to_bytes(&mut writer);
        let output = writer.finish();
        assert_eq!(output, data);
    }
}
