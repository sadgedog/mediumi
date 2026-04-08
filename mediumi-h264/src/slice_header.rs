// slice_layer_without_partitioning_rbsp -> NonIDR

use crate::{
    error::Error,
    nal::NalUnitType,
    pps::Pps,
    sps::{self, Sps},
    util::bitstream::BitstreamReader,
};

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
    pub num_ref_idx_l0_active_minus1: Option<u32>,
    pub num_ref_idx_l1_active_minus1: Option<u32>,
}

#[derive(Debug)]
pub enum ModificationCommand {
    ShortTermSubtract(u32), // idc=0: abs_diff_pic_num_minus1
    ShortTermAdd(u32),      // idc=1: abs_diff_pic_num_minus1
    LongTerm(u32),          // idc=2: long_term_pic_num
                            // idc=3: terminator (not stored, written in to_bytes)
}

#[derive(Debug)]
pub enum MvcModificationCommand {
    ShortTermSubtract(u32), // idc=0: abs_diff_pic_num_minus1
    ShortTermAdd(u32),      // idc=1: abs_diff_pic_num_minus1
    LongTerm(u32),          // idc=2: long_term_pic_num
    InterViewSubtract(u32), // idc=4: abs_diff_view_idx_minus1
    InterViewAdd(u32),      // idc=5: abs_diff_view_idx_minus1
                            // idc=3: terminator (not stored, written in to_bytes)
}

#[derive(Debug)]
pub struct RefPicListMvcModificationList {
    pub flag: bool,
    pub commands: Vec<MvcModificationCommand>,
}

#[derive(Debug)]
pub struct RefPicListMvcModification {
    pub l0: Option<RefPicListMvcModificationList>, // slice_type % 5 != 2 && != 4
    pub l1: Option<RefPicListMvcModificationList>, // slice_type % 5 == 1
}

impl RefPicListMvcModification {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(reader: &mut BitstreamReader, slice_type: SliceType) -> Result<Self, Error> {
        // l0, l1 のパース
        todo!()
    }
}

#[derive(Debug)]
pub struct RefPicListModificationList {
    pub flag: bool,
    pub commands: Vec<ModificationCommand>,
}

#[derive(Debug)]
pub struct RefPicListModification {
    pub l0: Option<RefPicListModificationList>, // slice_type % 5 != 2 && != 4
    pub l1: Option<RefPicListModificationList>, // slice_type % 5 == 1
}

impl RefPicListModification {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(reader: &mut BitstreamReader, slice_type: SliceType) -> Result<Self, Error> {
        // l0, l1 のパース (idc=4,5 追加)
        todo!()
    }
}

#[derive(Debug)]
pub enum RefPicListMod {
    MvcModification(RefPicListMvcModification), // nal_unit_type == 20, 21
    Modification(RefPicListModification),       // other nal_unit_type
}

impl RefPicListMod {
    pub fn parse(
        reader: &mut BitstreamReader,
        slice_type: SliceType,
        nal_unit_type: &NalUnitType,
    ) -> Result<Self, Error> {
        match nal_unit_type {
            NalUnitType::SliceExt | NalUnitType::DepthExt => Ok(Self::MvcModification(
                RefPicListMvcModification::parse(reader, slice_type)?,
            )),
            _ => Ok(Self::Modification(RefPicListModification::parse(
                reader, slice_type,
            )?)),
        }
    }
}

#[derive(Debug)]
pub struct WeightEntry {
    pub luma_weight_flag: bool,
    pub luma_weight: Option<i32>,
    pub luma_offset: Option<i32>,
    pub chroma_weight_flag: Option<bool>,
    pub chroma_weight: Option<[(i32, i32); 2]>, // [Cb, Cr] each (weight, offset)
}

#[derive(Debug)]
pub struct PredWeightTable {
    pub luma_log2_weight_denom: u32,
    pub chroma_log2_weight_denom: Option<u32>, // ChromaArrayType != 0
    pub l0: Vec<WeightEntry>,                  // num_ref_idx_l0_active_minus1 + 1 entries
    pub l1: Option<Vec<WeightEntry>>,          // slice_type % 5 == 1
}

impl PredWeightTable {}

#[derive(Debug)]
pub enum MemoryManagementControlOp {
    ShortTermUnused(u32),          // mmco=1: difference_of_pic_nums_minus1
    LongTermUnused(u32),           // mmco=2: long_term_pic_num
    ShortTermToLongTerm(u32, u32), // mmco=3: difference_of_pic_nums_minus1, long_term_frame_idx
    MaxLongTermFrameIdx(u32),      // mmco=4: max_long_term_frame_idx_plus1
    ClearAll,                      // mmco=5
    AssignLongTermFrameIdx(u32),   // mmco=6: long_term_frame_idx
                                   // mmco=0: terminator (not stored, written in to_bytes)
}

#[derive(Debug)]
pub enum DecRefPicMarking {
    Idr {
        no_output_of_prior_pics_flag: bool,
        long_term_reference_flag: bool,
    },
    NonIdr {
        adaptive_ref_pic_marking_mode_flag: bool,
        commands: Option<Vec<MemoryManagementControlOp>>, // None if flag is false
    },
}

impl DecRefPicMarking {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceType {
    P,  // 0, 5
    B,  // 1, 6
    I,  // 2, 7
    SP, // 3, 8
    SI, // 4, 9
}

impl TryFrom<u32> for SliceType {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value % 5 {
            0 => Ok(Self::P),
            1 => Ok(Self::B),
            2 => Ok(Self::I),
            3 => Ok(Self::SP),
            4 => Ok(Self::SI),
            _ => unreachable!(),
        }
    }
}

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

    pub fn parse(
        data: &[u8],
        sps: &Sps,
        pps: &Pps,
        nal_unit_type: NalUnitType,
    ) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);

        let first_mb_in_slice = reader.read_ue()?;
        let slice_type = reader.read_ue()?;
        let pic_parameter_set_id = reader.read_ue()?;
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
        let frame_num = reader.read_bits(sps.log2_max_frame_num_minus4 + 4)? as u16;
        let field_flags = if !sps.frame_mbs_only_flag {
            let field_pic_flag = reader.read_bit()?;
            let bottom_field_flag = if field_pic_flag {
                Some(reader.read_bit()?)
            } else {
                None
            };
            Some(FieldFlags {
                field_pic_flag,
                bottom_field_flag,
            })
        } else {
            None
        };

        let idr_pic_id = if nal_unit_type == NalUnitType::IDR {
            Some(reader.read_ue()?)
        } else {
            None
        };

        let field_pic_flag = field_flags.as_ref().map_or(false, |f| f.field_pic_flag);

        let (pic_order_cnt, delta_pic_order_cnt) = match &sps.pic_order_cnt {
            sps::PicOrderCnt::Type0 {
                log2_max_pic_order_cnt_lsb_minus4,
            } => {
                let pic_order_cnt_lsb = reader.read_bits(log2_max_pic_order_cnt_lsb_minus4 + 4)?;
                let delta_pic_order_cnt_bottom =
                    if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic_flag {
                        Some(reader.read_se()?)
                    } else {
                        None
                    };
                (
                    Some(PicOrderCnt {
                        pic_order_cnt_lsb,
                        delta_pic_order_cnt_bottom,
                    }),
                    None,
                )
            }
            sps::PicOrderCnt::Type1 {
                delta_pic_order_always_zero_flag,
                ..
            } => {
                if !delta_pic_order_always_zero_flag {
                    let mut deltas = vec![reader.read_se()?]; // delta_pic_order_cnt[0]
                    if pps.bottom_field_pic_order_in_frame_present_flag && !field_pic_flag {
                        deltas.push(reader.read_se()?); // delta_pic_order_cnt[1]
                    }
                    (None, Some(deltas))
                } else {
                    (None, None)
                }
            }
            sps::PicOrderCnt::Type2 => (None, None),
        };

        let redundant_pic_cnt = if pps.redundant_pic_cnt_present_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };

        let st = SliceType::try_from(slice_type)?;
        let direct_spatial_mv_pred_flag = if st == SliceType::B {
            Some(reader.read_bit()?)
        } else {
            None
        };

        let num_ref_idx = if st == SliceType::P || st == SliceType::SP || st == SliceType::B {
            let num_ref_idx_active_override_flag = reader.read_bit()?;
            let (num_ref_idx_l0_active_minus1, num_ref_idx_l1_active_minus1) =
                if num_ref_idx_active_override_flag {
                    let l0 = reader.read_ue()?;
                    let l1 = if st == SliceType::B {
                        Some(reader.read_ue()?)
                    } else {
                        None
                    };
                    (Some(l0), l1)
                } else {
                    (None, None)
                };
            Some(NumRefIdx {
                num_ref_idx_active_override_flag,
                num_ref_idx_l0_active_minus1,
                num_ref_idx_l1_active_minus1,
            })
        } else {
            None
        };

        let ref_pic_list_mod = RefPicListMod::parse(&mut reader, st, &nal_unit_type)?;

        todo!()
    }
}
