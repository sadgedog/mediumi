use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, elng::Elng, hdlr::Hdlr, mdhd::Mdhd, minf::Minf},
    types::BoxType,
};
use mediumi_util::bytestream::ByteWriter;

#[derive(Debug)]
pub struct Mdia {
    pub mdhd: Mdhd,
    pub hdlr: Hdlr,
    pub minf: Box<Minf>,
    pub elng: Option<Elng>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Mdia {
    const BOX_TYPE: BoxType = BoxType::Mdia;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.mdhd.write_box(writer);
        self.hdlr.write_box(writer);
        self.minf.write_box(writer);
        if let Some(ref e) = self.elng {
            e.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut mdhd: Option<Mdhd> = None;
        let mut hdlr: Option<Hdlr> = None;
        let mut minf: Option<Box<Minf>> = None;
        let mut elng: Option<Elng> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Mdhd(b) => {
                    if mdhd.is_some() {
                        return Err(Error::DuplicateBox("mdhd"));
                    }
                    mdhd = Some(b);
                }
                Mp4Box::Hdlr(b) => {
                    if hdlr.is_some() {
                        return Err(Error::DuplicateBox("hdlr"));
                    }
                    hdlr = Some(b);
                }
                Mp4Box::Minf(b) => {
                    if minf.is_some() {
                        return Err(Error::DuplicateBox("minf"));
                    }
                    minf = Some(b);
                }
                Mp4Box::Elng(b) => {
                    if elng.is_some() {
                        return Err(Error::DuplicateBox("elng"));
                    }
                    elng = Some(b);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            mdhd: mdhd.ok_or(Error::MissingRequiredBox("mdhd"))?,
            hdlr: hdlr.ok_or(Error::MissingRequiredBox("hdlr"))?,
            minf: minf.ok_or(Error::MissingRequiredBox("minf"))?,
            elng,
            others,
        })
    }
}
