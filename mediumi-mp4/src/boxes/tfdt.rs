use crate::{
    boxes::{BaseBox, Error, FullBox, FullBoxHeader},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Tfdt {
    pub header: FullBoxHeader,
    pub base_media_decode_time: u64,
}

impl BaseBox for Tfdt {
    const BOX_TYPE: BoxType = BoxType::Tfdt;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_bits(self.header.version as u32, 8);
        writer.write_bits(self.header.flags, 24);
        if self.header.version == 1 {
            writer.write_bits((self.base_media_decode_time >> 32) as u32, 32);
            writer.write_bits(self.base_media_decode_time as u32, 32);
        } else {
            writer.write_bits(self.base_media_decode_time as u32, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader {
            version: reader.read_bits(8)? as u8,
            flags: reader.read_bits(24)?,
        };
        let base_media_decode_time = if header.version == 1 {
            let high = reader.read_bits(32)? as u64;
            let low = reader.read_bits(32)? as u64;
            (high << 32) | low
        } else {
            reader.read_bits(32)? as u64
        };
        Ok(Self {
            header,
            base_media_decode_time,
        })
    }
}

impl FullBox for Tfdt {
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
    fn test_tfdt_v0_roundtrip() {
        // version=0: base_media_decode_time is 32-bit
        let data = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags
            0x01, 0x02, 0x03, 0x04, // base_media_decode_time (u32)
        ];
        let tfdt = Tfdt::parse(&data).expect("failed to parse tfdt v0");
        assert_eq!(tfdt.header.version, 0);
        assert_eq!(tfdt.header.flags, 0);
        assert_eq!(tfdt.base_media_decode_time, 0x01020304);

        let mut writer = BitstreamWriter::new();
        tfdt.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }

    #[test]
    fn test_tfdt_v1_roundtrip() {
        // version=1: base_media_decode_time is 64-bit
        let data = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x01, // base_media_decode_time upper 32bit
            0x02, 0x03, 0x04, 0x05, // base_media_decode_time lower 32bit
        ];
        let tfdt = Tfdt::parse(&data).expect("failed to parse tfdt v1");
        assert_eq!(tfdt.header.version, 1);
        assert_eq!(tfdt.header.flags, 0);
        assert_eq!(tfdt.base_media_decode_time, 0x0000_0001_0203_0405);

        let mut writer = BitstreamWriter::new();
        tfdt.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }
}
