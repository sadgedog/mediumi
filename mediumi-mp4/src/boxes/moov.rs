use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, meta::Meta, mvex::Mvex, mvhd::Mvhd, trak::Trak, udta::Udta,
    },
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Moov {
    pub mvhd: Mvhd,
    pub traks: Vec<Trak>,
    pub mvex: Option<Mvex>,
    pub meta: Option<Meta>,
    pub udta: Option<Udta>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Moov {
    const BOX_TYPE: BoxType = BoxType::Moov;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.mvhd.write_box(writer);
        for t in &self.traks {
            t.write_box(writer);
        }
        if let Some(ref m) = self.mvex {
            m.write_box(writer);
        }
        if let Some(ref m) = self.meta {
            m.write_box(writer);
        }
        if let Some(ref u) = self.udta {
            u.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut mvhd: Option<Mvhd> = None;
        let mut traks: Vec<Trak> = Vec::new();
        let mut mvex: Option<Mvex> = None;
        let mut meta: Option<Meta> = None;
        let mut udta: Option<Udta> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Mvhd(m) => {
                    if mvhd.is_some() {
                        return Err(Error::DuplicateBox("mvhd"));
                    }
                    mvhd = Some(m);
                }
                Mp4Box::Trak(t) => traks.push(*t),
                Mp4Box::Mvex(m) => {
                    if mvex.is_some() {
                        return Err(Error::DuplicateBox("mvex"));
                    }
                    mvex = Some(m);
                }
                Mp4Box::Meta(m) => {
                    if meta.is_some() {
                        return Err(Error::DuplicateBox("meta"));
                    }
                    meta = Some(m);
                }
                Mp4Box::Udta(u) => {
                    if udta.is_some() {
                        return Err(Error::DuplicateBox("udta"));
                    }
                    udta = Some(u);
                }
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            mvhd: mvhd.ok_or(Error::MissingRequiredBox("mvhd"))?,
            traks,
            mvex,
            meta,
            udta,
            others,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{FullBoxHeader, mvhd::UNITY_MATRIX};

    fn sample_mvhd() -> Mvhd {
        Mvhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            creation_time: 0,
            modification_time: 0,
            timescale: 1000,
            duration: 5000,
            rate: 0x0001_0000,
            volume: 0x0100,
            matrix: UNITY_MATRIX,
            next_track_id: 2,
        }
    }

    #[test]
    fn test_moov_with_mvhd_only_roundtrip() {
        let moov = Moov {
            mvhd: sample_mvhd(),
            traks: Vec::new(),
            mvex: None,
            meta: None,
            udta: None,
            others: Vec::new(),
        };
        let mut w = BitstreamWriter::new();
        moov.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Moov::parse(&bytes).expect("parse moov");
        assert_eq!(parsed.mvhd.timescale, 1000);
        let mut w2 = BitstreamWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_moov_missing_mvhd_errors() {
        let data = [0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e'];
        let err = Moov::parse(&data).unwrap_err();
        assert_eq!(err, Error::MissingRequiredBox("mvhd"));
    }
}
