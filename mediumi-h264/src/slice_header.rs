// slice_layer_without_partitioning_rbsp -> NonIDR

use crate::{error::Error, pps::Pps, sps::Sps, util::bitstream::BitstreamReader};

#[derive(Debug)]
pub struct FieldFlags {
    pub field_pic_flag: bool,
    pub bottom_field_flag: Option<bool>,
}

#[derive(Debug)]
pub struct PicOrderCnt {
    pub pic_order_cnt_lsb: u32,
    pub delta_pic_order_cnt_bottom: Option<i32>,
}

#[derive(Debug)]
pub struct NumRefIdx {
    pub num_ref_idx_active_override_flag: bool,
    pub num_ref_idx_10_active_minus1: Option<u32>,
    pub num_ref_idx_l1_active_minus1: Option<u32>,
}

#[derive(Debug)]
pub enum RefPicListMod {
    RefPicListMvcModification {},
    RefPicListModification {},
}

#[derive(Debug)]
pub struct PredWeightTable {}

#[derive(Debug)]
pub struct DecRefPicMarking {}

#[derive(Debug)]
pub struct DeblockingFilter {
    pub disable_deblocking_filter_idc: u32,
    pub slice_alpha_c0_offset_div2: Option<i32>,
    pub slice_beta_offset_div2: Option<i32>,
}

#[derive(Debug)]
pub struct SliceHeader {
    pub first_mb_in_slice: u32,
    pub slice_type: u32,
    pub pic_parameter_set_id: u32,
    pub colour_plane_id: Option<u8>,
    pub frame_num: u16,
    pub field_flags: Option<FieldFlags>,
    pub idr_pic_id: Option<u32>,
    pub pic_order_cnt: Option<PicOrderCnt>,
    pub delta_pic_order_cnt: Option<Vec<i32>>,
    pub redundant_pic_cnt: Option<u32>,
    pub direct_spatial_mv_pred_flag: Option<bool>,
    pub num_ref_idx: Option<NumRefIdx>,
    pub ref_pic_list_mod: RefPicListMod,
    pub pred_weight_table: Option<PredWeightTable>,
    pub dec_ref_pic_marking: Option<DecRefPicMarking>,
    pub cabac_init_idc: Option<u32>,
    pub slice_qp_delta: i32,
    pub sp_for_switch_flag: Option<bool>,
    pub slice_qs_delta: Option<i32>,
    pub deblocking_filter: Option<DeblockingFilter>,
    pub slice_group_change_cycle: Option<u32>,
}

impl SliceHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8], sps: &Sps, pps: &Pps) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);

        todo!()
    }
}
