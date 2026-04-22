use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct SubSampleInfo {
    pub subsample_size: u32,
    pub subsample_priority: u8,
    pub discardable: u8,
    pub codec_specific_parameters: u32,
}

#[derive(Debug)]
pub struct SubsEntry {
    pub sample_delta: u32,
    pub subsample_count: u16,
    pub subsamples: Vec<SubSampleInfo>,
}

#[derive(Debug)]
pub struct Subs {
    pub header: FullBoxHeader,
    pub entry_count: u32,
    pub entries: Vec<SubsEntry>,
}

impl BaseBox for Subs {
    const BOX_TYPE: BoxType = BoxType::Subs;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entry_count, 32);
        for entry in &self.entries {
            writer.write_bits(entry.sample_delta, 32);
            writer.write_bits(entry.subsample_count as u32, 16);
            for info in &entry.subsamples {
                if self.header.version == 1 {
                    writer.write_bits(info.subsample_size, 32);
                } else {
                    writer.write_bits(info.subsample_size, 16);
                }
                writer.write_bits(info.subsample_priority as u32, 8);
                writer.write_bits(info.discardable as u32, 8);
                writer.write_bits(info.codec_specific_parameters, 32);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let sample_delta = reader.read_bits(32)?;
            let subsample_count = reader.read_bits(16)? as u16;

            let mut subsamples = Vec::with_capacity(subsample_count as usize);
            for _ in 0..subsample_count {
                let subsample_size = if header.version == 1 {
                    reader.read_bits(32)?
                } else {
                    reader.read_bits(16)?
                };
                let subsample_priority = reader.read_bits(8)? as u8;
                let discardable = reader.read_bits(8)? as u8;
                let codec_specific_parameters = reader.read_bits(32)?;
                subsamples.push(SubSampleInfo {
                    subsample_size,
                    subsample_priority,
                    discardable,
                    codec_specific_parameters,
                });
            }

            entries.push(SubsEntry {
                sample_delta,
                subsample_count,
                subsamples,
            });
        }

        Ok(Self {
            header,
            entry_count,
            entries,
        })
    }
}

impl FullBox for Subs {
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
    fn test_subs_v0_roundtrip() {
        // v=0, 1 entry, 2 subsamples (subsample_size = u16)
        let data = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry 0
            0x00, 0x00, 0x00, 0x05, // sample_delta = 5
            0x00, 0x02, // subsample_count = 2
            // subsample[0]
            0x00, 0x10, // size = 16 (u16)
            0x01, // priority
            0x00, // discardable
            0x00, 0x00, 0x00, 0x07, // codec_specific = 7
            // subsample[1]
            0x00, 0x20, // size = 32 (u16)
            0x02, // priority
            0x01, // discardable
            0x00, 0x00, 0x00, 0x08, // codec_specific = 8
        ];
        let subs = Subs::parse(&data).expect("parse subs v0");
        assert_eq!(subs.header.version, 0);
        assert_eq!(subs.entry_count, 1);
        assert_eq!(subs.entries.len(), 1);

        let e = &subs.entries[0];
        assert_eq!(e.sample_delta, 5);
        assert_eq!(e.subsample_count, 2);
        assert_eq!(e.subsamples.len(), 2);

        assert_eq!(e.subsamples[0].subsample_size, 16);
        assert_eq!(e.subsamples[0].subsample_priority, 1);
        assert_eq!(e.subsamples[0].discardable, 0);
        assert_eq!(e.subsamples[0].codec_specific_parameters, 7);

        assert_eq!(e.subsamples[1].subsample_size, 32);
        assert_eq!(e.subsamples[1].subsample_priority, 2);
        assert_eq!(e.subsamples[1].discardable, 1);
        assert_eq!(e.subsamples[1].codec_specific_parameters, 8);

        let mut w = BitstreamWriter::new();
        subs.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_subs_v1_roundtrip() {
        // v=1, 1 entry, 1 subsample (subsample_size = u32)
        let data = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry 0
            0x00, 0x00, 0x00, 0x01, // sample_delta = 1
            0x00, 0x01, // subsample_count = 1
            // subsample[0]
            0x00, 0x01, 0x00, 0x00, // size = 65536
            0x05, // priority
            0x00, // discardable
            0xDE, 0xAD, 0xBE, 0xEF, // codec_specific
        ];
        let subs = Subs::parse(&data).expect("parse subs v1");
        assert_eq!(subs.header.version, 1);
        assert_eq!(subs.entries.len(), 1);
        assert_eq!(subs.entries[0].subsamples[0].subsample_size, 65536);
        assert_eq!(
            subs.entries[0].subsamples[0].codec_specific_parameters,
            0xDEADBEEF
        );

        let mut w = BitstreamWriter::new();
        subs.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_subs_empty_entry_count() {
        // v=0, entry_count = 0
        let data = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ];
        let subs = Subs::parse(&data).expect("parse empty subs");
        assert_eq!(subs.entry_count, 0);
        assert!(subs.entries.is_empty());

        let mut w = BitstreamWriter::new();
        subs.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_subs_entry_with_zero_subsamples() {
        // v=0, 1 entry with 0 subsamples
        let data = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            0x00, 0x00, 0x00, 0x03, // sample_delta = 3
            0x00, 0x00, // subsample_count = 0
        ];
        let subs = Subs::parse(&data).expect("parse subs with empty subsamples");
        assert_eq!(subs.entries[0].sample_delta, 3);
        assert_eq!(subs.entries[0].subsample_count, 0);
        assert!(subs.entries[0].subsamples.is_empty());

        let mut w = BitstreamWriter::new();
        subs.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }
}
