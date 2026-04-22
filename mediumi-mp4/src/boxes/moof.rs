use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, mfhd::Mfhd, traf::Traf, write_child_box},
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Moof {
    pub mfhd: Mfhd,
    pub trafs: Vec<Traf>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Moof {
    const BOX_TYPE: BoxType = BoxType::Moof;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        write_child_box(writer, Mfhd::BOX_TYPE, |w| self.mfhd.to_bytes(w));
        for traf in &self.trafs {
            write_child_box(writer, Traf::BOX_TYPE, |w| traf.to_bytes(w));
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut mfhd: Option<Mfhd> = None;
        let mut trafs = Vec::new();
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Mfhd(m) => {
                    if mfhd.is_some() {
                        return Err(Error::DuplicateBox("mfhd"));
                    }
                    mfhd = Some(m);
                }
                Mp4Box::Traf(t) => trafs.push(t),
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            mfhd: mfhd.ok_or(Error::MissingRequiredBox("mfhd"))?,
            trafs,
            others,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{FullBoxHeader, tfdt::Tfdt, tfhd::Tfhd, trun::Trun};

    fn build_moof_bytes() -> Vec<u8> {
        let moof = Moof {
            mfhd: Mfhd {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                sequence_number: 42,
            },
            trafs: vec![Traf {
                tfhd: Tfhd {
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    track_id: 1,
                    base_data_offset: None,
                    sample_description_index: None,
                    default_sample_duration: None,
                    default_sample_size: None,
                    default_sample_flags: None,
                },
                tfdt: Some(Tfdt {
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    base_media_decode_time: 2000,
                }),
                truns: vec![Trun {
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    sample_count: 0,
                    data_offset: None,
                    first_sample_flags: None,
                    samples: Vec::new(),
                }],
                sbgps: Vec::new(),
                others: Vec::new(),
            }],
            others: Vec::new(),
        };
        let mut w = BitstreamWriter::new();
        moof.to_bytes(&mut w);
        w.finish()
    }

    #[test]
    fn test_moof_roundtrip() {
        let bytes = build_moof_bytes();
        let parsed = Moof::parse(&bytes).expect("failed to parse moof");

        assert_eq!(parsed.mfhd.sequence_number, 42);
        assert_eq!(parsed.trafs.len(), 1);
        assert_eq!(parsed.trafs[0].tfhd.track_id, 1);
        assert_eq!(
            parsed.trafs[0]
                .tfdt
                .as_ref()
                .map(|t| t.base_media_decode_time),
            Some(2000)
        );
        assert_eq!(parsed.trafs[0].truns.len(), 1);
        assert!(parsed.others.is_empty());

        let mut w = BitstreamWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), bytes);
    }

    #[test]
    fn test_moof_missing_mfhd_errors() {
        let raw: [u8; 0] = [];
        let err = Moof::parse(&raw).unwrap_err();
        assert_eq!(err, Error::MissingRequiredBox("mfhd"));
    }

    #[test]
    fn test_moof_with_unknown_child_preserved() {
        // mfhd + 'free' box
        let mut raw = Vec::new();
        // mfhd: size=16, type='mfhd', body(version+flags+seq=7)
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x10]);
        raw.extend_from_slice(b"mfhd");
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // version + flags
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x07]); // sequence_number
        // free: size=8, type='free', no body
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
        raw.extend_from_slice(b"free");

        let parsed = Moof::parse(&raw).expect("failed to parse moof");
        assert_eq!(parsed.mfhd.sequence_number, 7);
        assert_eq!(parsed.others.len(), 1);
        assert_eq!(
            parsed.others[0],
            [0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e']
        );

        let mut w = BitstreamWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), raw);
    }
}
