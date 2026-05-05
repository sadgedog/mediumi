use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, meta::Meta, saio::Saio, saiz::Saiz, sbgp::Sbgp,
        sgpd::Sgpd, subs::Subs, tfdt::Tfdt, tfhd::Tfhd, trun::Trun,
    },
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Traf {
    pub tfhd: Tfhd,
    pub truns: Vec<Trun>,
    pub sbgps: Vec<Sbgp>,
    pub sgpds: Vec<Sgpd>,
    pub subs: Vec<Subs>,
    pub saizs: Vec<Saiz>,
    pub saios: Vec<Saio>,
    pub tfdt: Option<Tfdt>,
    pub meta: Option<Meta>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Traf {
    const BOX_TYPE: BoxType = BoxType::Traf;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.tfhd.write_box(writer);
        if let Some(ref tfdt) = self.tfdt {
            tfdt.write_box(writer);
        }
        for trun in &self.truns {
            trun.write_box(writer);
        }
        for sbgp in &self.sbgps {
            sbgp.write_box(writer);
        }
        for sgpd in &self.sgpds {
            sgpd.write_box(writer);
        }
        for subs in &self.subs {
            subs.write_box(writer);
        }
        for saiz in &self.saizs {
            saiz.write_box(writer);
        }
        for saio in &self.saios {
            saio.write_box(writer);
        }
        if let Some(ref meta) = self.meta {
            meta.write_box(writer);
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
        let mut meta: Option<Meta> = None;
        let mut truns = Vec::new();
        let mut sbgps = Vec::new();
        let mut sgpds = Vec::new();
        let mut subs = Vec::new();
        let mut saizs = Vec::new();
        let mut saios = Vec::new();
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
                Mp4Box::Trun(t) => truns.push(t),
                Mp4Box::Sbgp(s) => sbgps.push(s),
                Mp4Box::Sgpd(s) => sgpds.push(s),
                Mp4Box::Subs(s) => subs.push(s),
                Mp4Box::Saiz(s) => saizs.push(s),
                Mp4Box::Saio(s) => saios.push(s),
                Mp4Box::Tfdt(t) => {
                    if tfdt.is_some() {
                        return Err(Error::DuplicateBox("tfdt"));
                    }
                    tfdt = Some(t);
                }
                Mp4Box::Meta(m) => {
                    if meta.is_some() {
                        return Err(Error::DuplicateBox("meta"));
                    }
                    meta = Some(m);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        // tfhd is required
        let tfhd = tfhd.ok_or(Error::MissingRequiredBox("tfhd"))?;

        Ok(Self {
            tfhd,
            truns,
            sbgps,
            sgpds,
            subs,
            saizs,
            saios,
            tfdt,
            meta,
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
            sbgps: Vec::new(),
            sgpds: Vec::new(),
            subs: Vec::new(),
            saizs: Vec::new(),
            saios: Vec::new(),
            meta: None,
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
}
