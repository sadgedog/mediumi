use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, leva::Leva, mehd::Mehd, trex::Trex},
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Mvex {
    pub mehd: Option<Mehd>,
    pub trexs: Vec<Trex>,
    pub leva: Option<Leva>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Mvex {
    const BOX_TYPE: BoxType = BoxType::Mvex;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        if let Some(ref m) = self.mehd {
            m.write_box(writer);
        }
        for t in &self.trexs {
            t.write_box(writer);
        }
        if let Some(ref l) = self.leva {
            l.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut mehd: Option<Mehd> = None;
        let mut trexs: Vec<Trex> = Vec::new();
        let mut leva: Option<Leva> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Mehd(m) => {
                    if mehd.is_some() {
                        return Err(Error::DuplicateBox("mehd"));
                    }
                    mehd = Some(m);
                }
                Mp4Box::Trex(t) => trexs.push(t),
                Mp4Box::Leva(l) => {
                    if leva.is_some() {
                        return Err(Error::DuplicateBox("leva"));
                    }
                    leva = Some(l);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            mehd,
            trexs,
            leva,
            others,
        })
    }
}
