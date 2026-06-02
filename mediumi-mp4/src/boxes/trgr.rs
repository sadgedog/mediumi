use crate::{
    boxes::{BaseBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct TrackGroupTypeBox {
    pub track_group_type: [u8; 4],
    pub header: FullBoxHeader,
    pub track_group_id: u32,
    pub remaining: Vec<u8>,
}

#[derive(Debug)]
pub struct Trgr {
    pub groups: Vec<TrackGroupTypeBox>,
}

impl BaseBox for Trgr {
    const BOX_TYPE: BoxType = BoxType::Trgr;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        for g in &self.groups {
            // size(4) + box_type(4) + version+flags(4) + track_group_id(4) + remaining
            let total = 16u32 + g.remaining.len() as u32;
            writer.write_bits(total, 32);
            for &b in &g.track_group_type {
                writer.write_bits(b as u32, 8);
            }
            g.header.to_bytes(writer);
            writer.write_bits(g.track_group_id, 32);
            for &b in &g.remaining {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let mut groups = Vec::new();
        while reader.remaining_bits() >= 64 {
            let size = reader.read_bits(32)?;
            let track_group_type = [
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
            ];
            let body_len = match size {
                0 => reader.remaining_bits() / 8,
                1 => {
                    let high = reader.read_bits(32)? as u64;
                    let low = reader.read_bits(32)? as u64;
                    (((high << 32) | low) as usize).saturating_sub(16)
                }
                _ => (size as usize).saturating_sub(8),
            };
            // body must contain at least FullBoxHeader(4) + track_group_id(4)
            if body_len < 8 {
                return Err(Error::DataTooShort);
            }
            let header = FullBoxHeader::parse(&mut reader)?;
            let track_group_id = reader.read_bits(32)?;
            let remaining_len = body_len - 8;
            let mut remaining = Vec::with_capacity(remaining_len);
            for _ in 0..remaining_len {
                remaining.push(reader.read_bits(8)? as u8);
            }
            groups.push(TrackGroupTypeBox {
                track_group_type,
                header,
                track_group_id,
                remaining,
            });
        }
        Ok(Self { groups })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trgr_roundtrip() {
        let src = Trgr {
            groups: vec![
                TrackGroupTypeBox {
                    track_group_type: *b"msrc",
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    track_group_id: 42,
                    remaining: vec![],
                },
                TrackGroupTypeBox {
                    track_group_type: *b"ster",
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    track_group_id: 7,
                    remaining: vec![0xAA, 0xBB, 0xCC, 0xDD],
                },
            ],
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        // 16 + (16 + 4) = 36
        assert_eq!(bytes.len(), 36);

        let parsed = Trgr::parse(&bytes).expect("parse trgr");
        assert_eq!(parsed.groups.len(), 2);
        assert_eq!(&parsed.groups[0].track_group_type, b"msrc");
        assert_eq!(parsed.groups[0].track_group_id, 42);
        assert!(parsed.groups[0].remaining.is_empty());
        assert_eq!(&parsed.groups[1].track_group_type, b"ster");
        assert_eq!(parsed.groups[1].track_group_id, 7);
        assert_eq!(parsed.groups[1].remaining, vec![0xAA, 0xBB, 0xCC, 0xDD]);

        let mut w2 = BitstreamWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
