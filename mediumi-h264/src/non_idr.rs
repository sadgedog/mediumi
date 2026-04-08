use crate::{
    error::Error, nal::NalUnitType, pps::Pps, slice_header::SliceHeader, sps::Sps,
    util::bitstream::BitstreamReader,
};

#[derive(Debug)]
pub struct NonIDR {
    pub slice_header: SliceHeader,
    pub slice_data: Vec<u8>,
}

impl NonIDR {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8], sps: &Sps, pps: &Pps, nal_ref_idc: u8) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let slice_header =
            SliceHeader::parse(&mut reader, sps, pps, NalUnitType::NonIDR, nal_ref_idc)?;
        let slice_data = reader.read_remaining_bytes().0;

        Ok(Self {
            slice_header,
            slice_data,
        })
    }
}
