//! slice_a (Slice Data Partition A Layer)

use crate::{
    error::Error,
    nal::NalUnitType,
    pps::Pps,
    slice_header::SliceHeader,
    sps::Sps,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct SliceA {
    pub slice_header: SliceHeader,
    pub slice_id: u32,
    pub slice_data: Vec<u8>,
    pub slice_data_bit_offset: u8,
}

impl SliceA {
    pub fn to_bytes(&self, sps: &Sps, pps: &Pps) -> Result<Vec<u8>, Error> {
        let mut writer = BitstreamWriter::new();
        self.slice_header.to_bytes(&mut writer, sps, pps)?;
        writer.write_ue(self.slice_id);
        writer.write_remaining_bytes(&self.slice_data, self.slice_data_bit_offset);
        Ok(writer.finish())
    }

    pub fn parse(data: &[u8], sps: &Sps, pps: &Pps, nal_ref_idc: u8) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let slice_header =
            SliceHeader::parse(&mut reader, sps, pps, NalUnitType::SliceA, nal_ref_idc)?;
        let slice_id = reader.read_ue()?;
        let (slice_data, slice_data_bit_offset) = reader.read_remaining_bytes();

        Ok(Self {
            slice_header,
            slice_id,
            slice_data,
            slice_data_bit_offset,
        })
    }
}
