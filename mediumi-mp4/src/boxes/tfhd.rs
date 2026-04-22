use crate::{
    boxes::{BaseBox, Error, FullBox, FullBoxHeader},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

const BASE_DATA_OFFSET_PRESENT: u32 = 0x000001;
const SAMPLE_DESCRIPTION_INDEX_PRESENT: u32 = 0x000002;
const DEFAULT_SAMPLE_DURATION_PRESENT: u32 = 0x000008;
const DEFAULT_SAMPLE_SIZE_PRESENT: u32 = 0x000010;
const DEFAULT_SAMPLE_FLAGS_PRESENT: u32 = 0x000020;

#[derive(Debug)]
pub struct Tfhd {
    pub header: FullBoxHeader,
    pub track_id: u32,
    pub base_data_offset: Option<u64>,
    pub sample_description_index: Option<u32>,
    pub default_sample_duration: Option<u32>,
    pub default_sample_size: Option<u32>,
    pub default_sample_flags: Option<u32>,
}

impl BaseBox for Tfhd {
    const BOX_TYPE: BoxType = BoxType::Tfhd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_bits(self.header.version as u32, 8);
        writer.write_bits(self.header.flags, 24);
        writer.write_bits(self.track_id, 32);
        if let Some(v) = self.base_data_offset {
            writer.write_bits((v >> 32) as u32, 32);
            writer.write_bits(v as u32, 32);
        }
        if let Some(v) = self.sample_description_index {
            writer.write_bits(v, 32);
        }
        if let Some(v) = self.default_sample_duration {
            writer.write_bits(v, 32);
        }
        if let Some(v) = self.default_sample_size {
            writer.write_bits(v, 32);
        }
        if let Some(v) = self.default_sample_flags {
            writer.write_bits(v, 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let track_id = reader.read_bits(32)?;

        let base_data_offset = if header.flags & BASE_DATA_OFFSET_PRESENT != 0 {
            let high = reader.read_bits(32)? as u64;
            let low = reader.read_bits(32)? as u64;
            Some((high << 32) | low)
        } else {
            None
        };
        let sample_description_index = if header.flags & SAMPLE_DESCRIPTION_INDEX_PRESENT != 0 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };
        let default_sample_duration = if header.flags & DEFAULT_SAMPLE_DURATION_PRESENT != 0 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };
        let default_sample_size = if header.flags & DEFAULT_SAMPLE_SIZE_PRESENT != 0 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };
        let default_sample_flags = if header.flags & DEFAULT_SAMPLE_FLAGS_PRESENT != 0 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };

        Ok(Self {
            header,
            track_id,
            base_data_offset,
            sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }
}

impl FullBox for Tfhd {
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
    fn test_tfhd_minimal_roundtrip() {
        let data = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
        ];
        let tfhd = Tfhd::parse(&data).expect("failed to parse tfhd");
        assert_eq!(tfhd.header.version, 0);
        assert_eq!(tfhd.header.flags, 0);
        assert_eq!(tfhd.track_id, 1);
        assert_eq!(tfhd.base_data_offset, None);
        assert_eq!(tfhd.sample_description_index, None);
        assert_eq!(tfhd.default_sample_duration, None);
        assert_eq!(tfhd.default_sample_size, None);
        assert_eq!(tfhd.default_sample_flags, None);

        let mut writer = BitstreamWriter::new();
        tfhd.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }

    #[test]
    fn test_tfhd_all_optional_fields_roundtrip() {
        // flags = 0x01 | 0x02 | 0x08 | 0x10 | 0x20 = 0x3B
        let data = [
            0x00, // version
            0x00, 0x00, 0x3B, // flags
            0x00, 0x00, 0x00, 0x02, // track_id = 2
            0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, // base_data_offset
            0x00, 0x00, 0x00, 0x01, // sample_description_index
            0x00, 0x00, 0x04, 0x00, // default_sample_duration = 1024
            0x00, 0x00, 0x10, 0x00, // default_sample_size = 4096
            0x00, 0x00, 0x00, 0x00, // default_sample_flags
        ];
        let tfhd = Tfhd::parse(&data).expect("failed to parse tfhd");
        assert_eq!(tfhd.header.flags, 0x3B);
        assert_eq!(tfhd.track_id, 2);
        assert_eq!(tfhd.base_data_offset, Some(0x0000_0001_0203_0405));
        assert_eq!(tfhd.sample_description_index, Some(1));
        assert_eq!(tfhd.default_sample_duration, Some(1024));
        assert_eq!(tfhd.default_sample_size, Some(4096));
        assert_eq!(tfhd.default_sample_flags, Some(0));

        let mut writer = BitstreamWriter::new();
        tfhd.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }

    #[test]
    fn test_tfhd_typical_fragmented_roundtrip() {
        // flags = 0x08 | 0x10 | 0x20 = 0x38
        let data = [
            0x00, // version
            0x00, 0x00, 0x38, // flags
            0x00, 0x00, 0x00, 0x01, // track_id
            0x00, 0x00, 0x04, 0x00, // default_sample_duration
            0x00, 0x00, 0x10, 0x00, // default_sample_size
            0x01, 0x01, 0x00, 0x00, // default_sample_flags
        ];
        let tfhd = Tfhd::parse(&data).expect("failed to parse tfhd");
        assert_eq!(tfhd.base_data_offset, None);
        assert_eq!(tfhd.sample_description_index, None);
        assert_eq!(tfhd.default_sample_duration, Some(1024));

        let mut writer = BitstreamWriter::new();
        tfhd.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }
}
