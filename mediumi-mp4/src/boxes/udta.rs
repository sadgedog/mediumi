use crate::{
    boxes::{BaseBox, BoxIter, Error},
    types::BoxType,
    util::bytestream::ByteWriter,
};

#[derive(Debug)]
pub struct Udta {
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Udta {
    const BOX_TYPE: BoxType = BoxType::Udta;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (_child, raw) = item?;
            others.push(raw.to_vec());
        }
        Ok(Self { others })
    }
}
