use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, meta::Meta, mvhd::Mvhd},
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Moov {
    pub mvhd: Mvhd,
    pub meta: Option<Meta>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Moov {
    const BOX_TYPE: BoxType = BoxType::Moov;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.mvhd.write_box(writer);
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
        let mut mvhd: Option<Mvhd> = None;
        let mut meta: Option<Meta> = None;
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
                Mp4Box::Meta(m) => {
                    if meta.is_some() {
                        return Err(Error::DuplicateBox("meta"));
                    }
                    meta = Some(m);
                }
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            mvhd: mvhd.ok_or(Error::MissingRequiredBox("mvhd"))?,
            meta,
            others,
        })
    }
}
