use crate::{error::Error, nal::NalUnitType, pps::Pps, slice_header::SliceHeader, sps::Sps};

#[derive(Debug)]
pub struct NonIDR {
    pub slice_header: SliceHeader,
    pub slice_data: Vec<u8>,
}

impl NonIDR {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8], sps: &Sps, pps: &Pps) -> Result<Self, Error> {
        let nal_unit_type = NalUnitType::IDR;
        let slice_header = SliceHeader::parse(data, sps, pps, nal_unit_type)?;
        todo!()
    }
}
