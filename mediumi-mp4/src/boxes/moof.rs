use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, mfhd::Mfhd, traf::Traf},
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
        self.mfhd.write_box(writer);
        for traf in &self.trafs {
            traf.write_box(writer);
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
                Mp4Box::Traf(t) => trafs.push(*t),
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
                sgpds: Vec::new(),
                subs: Vec::new(),
                saizs: Vec::new(),
                saios: Vec::new(),
                meta: None,
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
}
