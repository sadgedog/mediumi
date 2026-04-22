use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, tfdt::Tfdt, tfhd::Tfhd, trun::Trun, write_child_box},
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Traf {
    pub tfhd: Tfhd,
    pub tfdt: Option<Tfdt>,
    pub truns: Vec<Trun>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Traf {
    const BOX_TYPE: BoxType = BoxType::Traf;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        write_child_box(writer, Tfhd::BOX_TYPE, |w| self.tfhd.to_bytes(w));
        if let Some(ref tfdt) = self.tfdt {
            write_child_box(writer, BoxType::Tfdt, |w| tfdt.to_bytes(w));
        }
        for trun in &self.truns {
            write_child_box(writer, Trun::BOX_TYPE, |w| trun.to_bytes(w));
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut tfhd: Option<Tfhd> = None;
        let mut tfdt: Option<Tfdt> = None;
        let mut truns = Vec::new();
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Tfhd(t) => {
                    if tfhd.is_some() {
                        return Err(Error::DuplicateBox("tfhd"));
                    }
                    tfhd = Some(t);
                }
                Mp4Box::Tfdt(t) => {
                    if tfdt.is_some() {
                        return Err(Error::DuplicateBox("tfdt"));
                    }
                    tfdt = Some(t);
                }
                Mp4Box::Trun(t) => truns.push(t),
                _ => others.push(raw.to_vec()),
            }
        }
        // tfhd is required
        let tfhd = tfhd.ok_or(Error::MissingRequiredBox("tfhd"))?;

        Ok(Self {
            tfhd,
            tfdt,
            truns,
            others,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{FullBoxHeader, trun::TrunSample};

    fn build_traf_bytes() -> Vec<u8> {
        let traf = Traf {
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
                base_media_decode_time: 1000,
            }),
            truns: vec![Trun {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0x100, // sample_duration only
                },
                sample_count: 1,
                data_offset: None,
                first_sample_flags: None,
                samples: vec![TrunSample {
                    sample_duration: Some(1024),
                    sample_size: None,
                    sample_flags: None,
                    sample_composition_time_offset: None,
                }],
            }],
            others: Vec::new(),
        };
        let mut w = BitstreamWriter::new();
        traf.to_bytes(&mut w);
        w.finish()
    }

    #[test]
    fn test_traf_roundtrip() {
        let bytes = build_traf_bytes();
        let parsed = Traf::parse(&bytes).expect("failed to parse traf");

        assert_eq!(parsed.tfhd.track_id, 1);
        assert_eq!(
            parsed.tfdt.as_ref().map(|t| t.base_media_decode_time),
            Some(1000)
        );
        assert_eq!(parsed.truns.len(), 1);
        assert_eq!(parsed.truns[0].samples.len(), 1);
        assert_eq!(parsed.truns[0].samples[0].sample_duration, Some(1024));
        assert!(parsed.others.is_empty());

        let mut w = BitstreamWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), bytes);
    }

    #[test]
    fn test_traf_with_unknown_child_preserved() {
        let mut raw = Vec::new();
        // tfhd: size=16, type='tfhd', body(version+flags+track_id)
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x10]);
        raw.extend_from_slice(b"tfhd");
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // version + flags
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x07]); // track_id = 7
        // free: size=8, type='free', no body
        raw.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
        raw.extend_from_slice(b"free");

        let parsed = Traf::parse(&raw).expect("failed to parse traf");
        assert_eq!(parsed.tfhd.track_id, 7);
        assert_eq!(parsed.others.len(), 1);
        assert_eq!(
            parsed.others[0],
            [0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e']
        );

        let mut w = BitstreamWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), raw);
    }

    #[test]
    fn test_traf_missing_tfhd_errors() {
        let raw = [0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e'];
        let err = Traf::parse(&raw).unwrap_err();
        assert_eq!(err, Error::MissingRequiredBox("tfhd"));
    }
}
