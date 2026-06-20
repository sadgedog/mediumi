use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug, Clone)]
pub struct ElstEntry {
    pub segment_duration: u64,
    pub media_time: i64,
    pub media_rate_integer: i16,
    pub media_rate_fraction: i16,
}

#[derive(Debug)]
pub struct Elst {
    pub header: FullBoxHeader,
    pub entries: Vec<ElstEntry>,
}

impl BaseBox for Elst {
    const BOX_TYPE: BoxType = BoxType::Elst;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            if self.header.version == 1 {
                writer.write_bits((e.segment_duration >> 32) as u32, 32);
                writer.write_bits(e.segment_duration as u32, 32);
                writer.write_bits((e.media_time >> 32) as u32, 32);
                writer.write_bits(e.media_time as u32, 32);
            } else {
                writer.write_bits(e.segment_duration as u32, 32);
                writer.write_bits(e.media_time as u32, 32);
            }
            writer.write_bits(e.media_rate_integer as u32, 16);
            writer.write_bits(e.media_rate_fraction as u32, 16);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let (segment_duration, media_time) = if header.version == 1 {
                let sd_high = (reader.read_bits(32)? as u64) << 32;
                let sd_low = reader.read_bits(32)? as u64;
                let mt_high = (reader.read_bits(32)? as i64) << 32;
                let mt_low = reader.read_bits(32)? as i64;
                (sd_high | sd_low, mt_high | mt_low)
            } else {
                let sd = reader.read_bits(32)? as u64;
                let mt = reader.read_bits(32)? as i32 as i64;
                (sd, mt)
            };
            let media_rate_integer = reader.read_bits(16)? as i16;
            let media_rate_fraction = reader.read_bits(16)? as i16;
            entries.push(ElstEntry {
                segment_duration,
                media_time,
                media_rate_integer,
                media_rate_fraction,
            });
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Elst {
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
    fn test_elst_v0_roundtrip() {
        let src = Elst {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![ElstEntry {
                segment_duration: 1000,
                media_time: -1,
                media_rate_integer: 1,
                media_rate_fraction: 0,
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Elst::parse(&bytes).expect("parse elst");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].media_time, -1);
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
