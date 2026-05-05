use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, elst::Elst},
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Edts {
    pub elst: Option<Elst>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Edts {
    const BOX_TYPE: BoxType = BoxType::Edts;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        if let Some(ref e) = self.elst {
            e.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut elst: Option<Elst> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Elst(e) => {
                    if elst.is_some() {
                        return Err(Error::DuplicateBox("elst"));
                    }
                    elst = Some(e);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self { elst, others })
    }
}
