//! slice_c (Slice Data Partition C Layer)

use crate::{
    error::Error,
    pps::Pps,
    sps::Sps,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct SliceC {
    pub slice_id: u32,
    pub colour_plane_id: Option<u8>,
    pub redundant_pic_cnt: Option<u32>,
    pub slice_data: Vec<u8>,
    pub slice_data_bit_offset: u8,
}

impl SliceC {
    pub fn to_bytes(&self, _sps: &Sps, _pps: &Pps) -> Result<Vec<u8>, Error> {
        let mut writer = BitstreamWriter::new();
        writer.write_ue(self.slice_id);
        if let Some(colour_plane_id) = self.colour_plane_id {
            writer.write_bits(colour_plane_id as u32, 2);
        }
        if let Some(redundant_pic_cnt) = self.redundant_pic_cnt {
            writer.write_ue(redundant_pic_cnt);
        }
        writer.write_remaining_bytes(&self.slice_data, self.slice_data_bit_offset);
        Ok(writer.finish())
    }

    pub fn parse(data: &[u8], sps: &Sps, pps: &Pps) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let slice_id = reader.read_ue()?;

        let colour_plane_id = if sps
            .high_profile
            .as_ref()
            .and_then(|hp| hp.separate_colour_plane_flag)
            == Some(true)
        {
            Some(reader.read_bits(2)? as u8)
        } else {
            None
        };

        let redundant_pic_cnt = if pps.redundant_pic_cnt_present_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };

        let (slice_data, slice_data_bit_offset) = reader.read_remaining_bytes();

        Ok(Self {
            slice_id,
            colour_plane_id,
            redundant_pic_cnt,
            slice_data,
            slice_data_bit_offset,
        })
    }
}
