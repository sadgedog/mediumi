use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, meta::Meta, mvex::Mvex, mvhd::Mvhd, pssh::Pssh,
        trak::Trak, udta::Udta,
    },
    types::BoxType,
};
use mediumi_util::bytestream::ByteWriter;

#[derive(Debug)]
pub struct Moov {
    pub mvhd: Mvhd,
    pub traks: Vec<Trak>,
    pub mvex: Option<Mvex>,
    pub meta: Option<Meta>,
    pub udta: Option<Udta>,
    pub pssh: Vec<Pssh>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Moov {
    const BOX_TYPE: BoxType = BoxType::Moov;

    fn to_bytes(&self, writer: &mut ByteWriter) {
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
        for p in &self.pssh {
            p.write_box(writer);
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
        let mut pssh: Vec<Pssh> = Vec::new();
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
                Mp4Box::Pssh(p) => pssh.push(p),
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            mvhd: mvhd.ok_or(Error::MissingRequiredBox("mvhd"))?,
            traks,
            mvex,
            meta,
            udta,
            pssh,
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
            pssh: Vec::new(),
            others: Vec::new(),
        };
        let mut w = ByteWriter::new();
        moov.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Moov::parse(&bytes).expect("parse moov");
        assert_eq!(parsed.mvhd.timescale, 1000);
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_moov_with_pssh_roundtrip() {
        let pssh = Pssh {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            system_id: [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
                0x0E, 0x0F,
            ],
            key_ids: Vec::new(),
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let moov = Moov {
            mvhd: sample_mvhd(),
            traks: Vec::new(),
            mvex: None,
            meta: None,
            udta: None,
            pssh: vec![pssh],
            others: Vec::new(),
        };
        let mut w = ByteWriter::new();
        moov.to_bytes(&mut w);
        let bytes = w.finish();

        let parsed = Moov::parse(&bytes).expect("parse moov with pssh");
        let mut w2 = ByteWriter::new();
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
