use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, dref::Dref},
    types::BoxType,
};
use mediumi_util::bytestream::ByteWriter;

#[derive(Debug)]
pub struct Dinf {
    pub dref: Dref,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Dinf {
    const BOX_TYPE: BoxType = BoxType::Dinf;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.dref.write_box(writer);
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut dref: Option<Dref> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Dref(d) => {
                    if dref.is_some() {
                        return Err(Error::DuplicateBox("dref"));
                    }
                    dref = Some(d);
                }
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            dref: dref.ok_or(Error::MissingRequiredBox("dref"))?,
            others,
        })
    }
}
