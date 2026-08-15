use crate::{
    boxes::{BaseBox, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct TrackReferenceTypeBox {
    pub reference_type: [u8; 4],
    pub track_ids: Vec<u32>,
}

#[derive(Debug)]
pub struct Tref {
    pub references: Vec<TrackReferenceTypeBox>,
}

impl BaseBox for Tref {
    const BOX_TYPE: BoxType = BoxType::Tref;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        for r in &self.references {
            let total = 8u32 + 4 * r.track_ids.len() as u32;
            writer.write_bits(total, 32);
            for &b in &r.reference_type {
                writer.write_bits(b as u32, 8);
            }
            for &id in &r.track_ids {
                writer.write_bits(id, 32);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let mut references = Vec::new();
        while reader.remaining_bits() >= 64 {
            let size = reader.read_bits(32)?;
            let reference_type = [
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
            if body_len % 4 != 0 {
                return Err(Error::DataTooShort);
            }
            let count = body_len / 4;
            let mut track_ids = Vec::with_capacity(count);
            for _ in 0..count {
                track_ids.push(reader.read_bits(32)?);
            }
            references.push(TrackReferenceTypeBox {
                reference_type,
                track_ids,
            });
        }
        Ok(Self { references })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tref_roundtrip() {
        let src = Tref {
            references: vec![
                TrackReferenceTypeBox {
                    reference_type: *b"hint",
                    track_ids: vec![1, 2],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"cdsc",
                    track_ids: vec![3],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"font",
                    track_ids: vec![4],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"hind",
                    track_ids: vec![5],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"vdep",
                    track_ids: vec![6],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"vplx",
                    track_ids: vec![7],
                },
                TrackReferenceTypeBox {
                    reference_type: *b"subt",
                    track_ids: vec![8],
                },
            ],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 88);

        let parsed = Tref::parse(&bytes).expect("parse tref failed");
        assert_eq!(parsed.references.len(), 7);
        assert_eq!(&parsed.references[0].reference_type, b"hint");
        assert_eq!(parsed.references[0].track_ids, vec![1, 2]);
        assert_eq!(&parsed.references[1].reference_type, b"cdsc");
        assert_eq!(parsed.references[1].track_ids, vec![3]);
        assert_eq!(&parsed.references[2].reference_type, b"font");
        assert_eq!(parsed.references[2].track_ids, vec![4]);
        assert_eq!(&parsed.references[3].reference_type, b"hind");
        assert_eq!(parsed.references[3].track_ids, vec![5]);
        assert_eq!(&parsed.references[4].reference_type, b"vdep");
        assert_eq!(parsed.references[4].track_ids, vec![6]);
        assert_eq!(&parsed.references[5].reference_type, b"vplx");
        assert_eq!(parsed.references[5].track_ids, vec![7]);
        assert_eq!(&parsed.references[6].reference_type, b"subt");
        assert_eq!(parsed.references[6].track_ids, vec![8]);

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
