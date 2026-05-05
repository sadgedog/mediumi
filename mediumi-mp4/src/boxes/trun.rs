use crate::{
    boxes::{BaseBox, Error, FullBox, FullBoxHeader},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

const DATA_OFFSET_PRESENT: u32 = 0x000001;
const FIRST_SAMPLE_FLAGS_PRESENT: u32 = 0x000004;
const SAMPLE_DURATION_PRESENT: u32 = 0x000100;
const SAMPLE_SIZE_PRESENT: u32 = 0x000200;
const SAMPLE_FLAGS_PRESENT: u32 = 0x000400;
const SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT: u32 = 0x000800;

#[derive(Debug, PartialEq)]
pub struct TrunSample {
    pub sample_duration: Option<u32>,
    pub sample_size: Option<u32>,
    pub sample_flags: Option<u32>,
    pub sample_composition_time_offset: Option<i64>,
}

#[derive(Debug)]
pub struct Trun {
    pub header: FullBoxHeader,
    pub sample_count: u32,
    pub data_offset: Option<i32>,
    pub first_sample_flags: Option<u32>,
    pub samples: Vec<TrunSample>,
}

impl BaseBox for Trun {
    const BOX_TYPE: BoxType = BoxType::Trun;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_bits(self.header.version as u32, 8);
        writer.write_bits(self.header.flags, 24);
        writer.write_bits(self.sample_count, 32);
        if let Some(v) = self.data_offset {
            writer.write_bits(v as u32, 32);
        }
        if let Some(v) = self.first_sample_flags {
            writer.write_bits(v, 32);
        }
        for sample in &self.samples {
            if let Some(v) = sample.sample_duration {
                writer.write_bits(v, 32);
            }
            if let Some(v) = sample.sample_size {
                writer.write_bits(v, 32);
            }
            if let Some(v) = sample.sample_flags {
                writer.write_bits(v, 32);
            }
            if let Some(v) = sample.sample_composition_time_offset {
                if self.header.version == 1 {
                    writer.write_bits((v as i32) as u32, 32);
                } else {
                    writer.write_bits(v as u32, 32);
                }
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sample_count = reader.read_bits(32)?;

        let data_offset = if header.flags & DATA_OFFSET_PRESENT != 0 {
            Some(reader.read_bits(32)? as i32)
        } else {
            None
        };
        let first_sample_flags = if header.flags & FIRST_SAMPLE_FLAGS_PRESENT != 0 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };

        let mut samples = Vec::with_capacity(sample_count as usize);
        for _ in 0..sample_count {
            let sample_duration = if header.flags & SAMPLE_DURATION_PRESENT != 0 {
                Some(reader.read_bits(32)?)
            } else {
                None
            };
            let sample_size = if header.flags & SAMPLE_SIZE_PRESENT != 0 {
                Some(reader.read_bits(32)?)
            } else {
                None
            };
            let sample_flags = if header.flags & SAMPLE_FLAGS_PRESENT != 0 {
                Some(reader.read_bits(32)?)
            } else {
                None
            };
            let sample_composition_time_offset =
                if header.flags & SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT != 0 {
                    let raw = reader.read_bits(32)?;
                    if header.version == 1 {
                        Some((raw as i32) as i64)
                    } else {
                        Some(raw as i64)
                    }
                } else {
                    None
                };
            samples.push(TrunSample {
                sample_duration,
                sample_size,
                sample_flags,
                sample_composition_time_offset,
            });
        }

        Ok(Self {
            header,
            sample_count,
            data_offset,
            first_sample_flags,
            samples,
        })
    }
}

impl FullBox for Trun {
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
    fn test_trun_minimal_roundtrip() {
        // version=0, flags=0, sample_count=0
        let data = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ];
        let trun = Trun::parse(&data).expect("failed to parse trun");
        assert_eq!(trun.header.version, 0);
        assert_eq!(trun.header.flags, 0);
        assert_eq!(trun.data_offset, None);
        assert_eq!(trun.first_sample_flags, None);
        assert!(trun.samples.is_empty());

        let mut writer = BitstreamWriter::new();
        trun.to_bytes(&mut writer);
        assert_eq!(writer.finish(), data);
    }
}
