use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, edts::Edts, mdia::Mdia, meta::Meta, tkhd::Tkhd,
        tref::Tref, trgr::Trgr, udta::Udta,
    },
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Trak {
    pub tkhd: Tkhd,
    pub tref: Option<Tref>,
    pub trgr: Option<Trgr>,
    pub edts: Option<Edts>,
    pub meta: Option<Meta>,
    pub mdia: Box<Mdia>,
    pub udta: Option<Udta>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Trak {
    const BOX_TYPE: BoxType = BoxType::Trak;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.tkhd.write_box(writer);
        if let Some(ref b) = self.tref {
            b.write_box(writer);
        }
        if let Some(ref b) = self.trgr {
            b.write_box(writer);
        }
        if let Some(ref b) = self.edts {
            b.write_box(writer);
        }
        if let Some(ref b) = self.meta {
            b.write_box(writer);
        }
        self.mdia.write_box(writer);
        if let Some(ref b) = self.udta {
            b.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut tkhd: Option<Tkhd> = None;
        let mut tref: Option<Tref> = None;
        let mut trgr: Option<Trgr> = None;
        let mut edts: Option<Edts> = None;
        let mut meta: Option<Meta> = None;
        let mut mdia: Option<Box<Mdia>> = None;
        let mut udta: Option<Udta> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Tkhd(b) => {
                    if tkhd.is_some() {
                        return Err(Error::DuplicateBox("tkhd"));
                    }
                    tkhd = Some(b);
                }
                Mp4Box::Tref(b) => {
                    if tref.is_some() {
                        return Err(Error::DuplicateBox("tref"));
                    }
                    tref = Some(b);
                }
                Mp4Box::Trgr(b) => {
                    if trgr.is_some() {
                        return Err(Error::DuplicateBox("trgr"));
                    }
                    trgr = Some(b);
                }
                Mp4Box::Edts(b) => {
                    if edts.is_some() {
                        return Err(Error::DuplicateBox("edts"));
                    }
                    edts = Some(b);
                }
                Mp4Box::Meta(b) => {
                    if meta.is_some() {
                        return Err(Error::DuplicateBox("meta"));
                    }
                    meta = Some(b);
                }
                Mp4Box::Mdia(b) => {
                    if mdia.is_some() {
                        return Err(Error::DuplicateBox("mdia"));
                    }
                    mdia = Some(b);
                }
                Mp4Box::Udta(b) => {
                    if udta.is_some() {
                        return Err(Error::DuplicateBox("udta"));
                    }
                    udta = Some(b);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            tkhd: tkhd.ok_or(Error::MissingRequiredBox("tkhd"))?,
            tref,
            trgr,
            edts,
            meta,
            mdia: mdia.ok_or(Error::MissingRequiredBox("mdia"))?,
            udta,
            others,
        })
    }
}
