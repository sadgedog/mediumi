//! PPS (Picture Parameter Set) parser
//!
//! RBSP data when nal_unit_type = 8 (PPS)
//! All fields below profile/level are encoded using Exp-Golomb coding
//!
//! PPS construction
//! ┌───────────────────────────────────────────────────────┐
//! │  pic_parameter_set_id: ue(v)                          │
//! │  seq_parameter_set_id: ue(v)                          │
//! │  entropy_coding_mode_flag (1 bit)                     │
//! │  bottom_field_pic_order_in_frame_present_flag (1 bit) │
//! │  num_slice_groups_minus1: ue(v)                       │
//! ├───────────────────────────────────────────────────────┤
//!
//! If num_slice_groups_minus1 > 0
//! ┌───────────────────────────────────────────────────────┐
//! │  slice_group_map_type: ue(v)                          │ <- 0..6
//! ├───────────────────────────────────────────────────────┤
//!
//! If slice_group_map_type = 0
//! ┌───────────────────────────────────────────────────────┐
//! │  run_length_minus1: ue(v)                             │
//! ├───────────────────────────────────────────────────────┤
//!
//! If slice_group_map_type = 2
//! ┌───────────────────────────────────────────────────────┐
//! │  top_left: ue(v)                                      │
//! │  bottom_right: ue(v)                                  │
//! ├───────────────────────────────────────────────────────┤
//!
//! If slice_group_map_type = 3 | 4 | 5
//! ┌───────────────────────────────────────────────────────┐
//! │  slice_group_change_direction_flag: u(1)              │
//! │  slice_group_change_rate_minus1: ue(v)                │
//! ├───────────────────────────────────────────────────────┤
//!
//! If slice_group_map_type = 6
//! ┌───────────────────────────────────────────────────────┐
//! │  pic_size_in_map_units_minus1: ue(1)                  │
//! │  slice_group_id: u(v)                                 │
//! ├───────────────────────────────────────────────────────┤
//!
//! ┌───────────────────────────────────────────────────────┐
//! │  num_ref_idx_l0_default_active_minus1: ue(v)          │
//! │  num_ref_idx_l1_default_active_minus1: ue(v)          │
//! │  weighted_pred_flag (1 bit)                           │
//! │  weighted_bipred_idc (2 bits)                         │
//! │  pic_init_qp_minus26: se(v)                           │
//! │  pic_init_qs_minus26: se(v)                           │
//! │  chroma_qp_index_offset: se(v)                        │
//! │  deblocking_filter_control_present_flag (1 bit)       │
//! │  constrained_intra_pred_flag (1 bit)                  │
//! │  redundant_pic_cnt_present_flag (1 bit)               │
//! ├───────────────────────────────────────────────────────┤
//!
//! If more_rbsp_data()
//! ┌───────────────────────────────────────────────────────┐
//! │  transform_8x8_mode_flag (1 bit)                      │
//! │  pic_scaling_matrix_present_flag (1 bit)              │
//! │  pic_scaling_list_present_flag[i] (1 bit)             │
//! │  scaling_list (variable)                              │
//! │  second_chroma_qp_index_offset: se(v)                 │
//! └───────────────────────────────────────────────────────┘

use crate::{
    util::bitstream::{BitstreamReader, BitstreamWriter},
    {error::Error, sps::Sps},
};

#[derive(Debug)]
pub enum SliceGroup {
    Type0 {
        slice_group_map_type: u32,
        run_length_minus1: Vec<u32>,
    },
    Type2 {
        slice_group_map_type: u32,
        top_left: Vec<u32>,
        bottom_right: Vec<u32>,
    },
    Type3_5 {
        slice_group_map_type: u32,
        slice_group_change_direction_flag: bool,
        slice_group_change_rate_minus1: u32,
    },
    Type6 {
        slice_group_map_type: u32,
        pic_size_in_map_units_minus1: u32,
        slice_group_id: Vec<u32>,
    },
}

#[derive(Debug)]
pub struct MoreRbspData {
    pub transform_8x8_mode_flag: bool,
    pub pic_scaling_matrix_present_flag: bool,
    pub pic_scaling_list_present_flag: Option<Vec<bool>>,
    pub scaling_list_4x4: Option<[[u8; 16]; 6]>,
    pub scaling_list_8x8: Option<[[u8; 64]; 6]>,
    pub second_chroma_qp_index_offset: i32,
}

#[derive(Debug)]
pub struct Pps {
    pub pic_parameter_set_id: u32,
    pub seq_parameter_set_id: u32,
    pub entropy_coding_mode_flag: bool,
    pub bottom_field_pic_order_in_frame_present_flag: bool,
    pub num_slice_groups_minus1: u32,
    pub slice_group: Option<SliceGroup>,
    pub num_ref_idx_l0_default_active_minus1: u32,
    pub num_ref_idx_l1_default_active_minus1: u32,
    pub weighted_pred_flag: bool,
    pub weighted_bipred_idc: u8,
    pub pic_init_qp_minus26: i32,
    pub pic_init_qs_minus26: i32,
    pub chroma_qp_index_offset: i32,
    pub deblocking_filter_control_present_flag: bool,
    pub constrained_intra_pred_flag: bool,
    pub redundant_pic_cnt_present_flag: bool,
    pub more_rbsp_data: Option<MoreRbspData>,
}

impl Pps {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        writer.write_ue(self.pic_parameter_set_id);
        writer.write_ue(self.seq_parameter_set_id);
        writer.write_bool(self.entropy_coding_mode_flag);
        writer.write_bool(self.bottom_field_pic_order_in_frame_present_flag);
        writer.write_ue(self.num_slice_groups_minus1);

        if let Some(sg) = &self.slice_group {
            match sg {
                SliceGroup::Type0 {
                    slice_group_map_type,
                    run_length_minus1,
                } => {
                    writer.write_ue(*slice_group_map_type);
                    for v in run_length_minus1 {
                        writer.write_ue(*v);
                    }
                }
                SliceGroup::Type2 {
                    slice_group_map_type,
                    top_left,
                    bottom_right,
                } => {
                    writer.write_ue(*slice_group_map_type);
                    for (t, b) in top_left.iter().zip(bottom_right.iter()) {
                        writer.write_ue(*t);
                        writer.write_ue(*b);
                    }
                }
                SliceGroup::Type3_5 {
                    slice_group_map_type,
                    slice_group_change_direction_flag,
                    slice_group_change_rate_minus1,
                } => {
                    writer.write_ue(*slice_group_map_type);
                    writer.write_bool(*slice_group_change_direction_flag);
                    writer.write_ue(*slice_group_change_rate_minus1);
                }
                SliceGroup::Type6 {
                    slice_group_map_type,
                    pic_size_in_map_units_minus1,
                    slice_group_id,
                } => {
                    writer.write_ue(*slice_group_map_type);
                    writer.write_ue(*pic_size_in_map_units_minus1);
                    let bits = ((self.num_slice_groups_minus1 + 1) as f64).log2().ceil() as u8;
                    for id in slice_group_id {
                        writer.write_bits(*id, bits);
                    }
                }
            }
        }

        writer.write_ue(self.num_ref_idx_l0_default_active_minus1);
        writer.write_ue(self.num_ref_idx_l1_default_active_minus1);
        writer.write_bool(self.weighted_pred_flag);
        writer.write_bits(self.weighted_bipred_idc as u32, 2);
        writer.write_se(self.pic_init_qp_minus26);
        writer.write_se(self.pic_init_qs_minus26);
        writer.write_se(self.chroma_qp_index_offset);
        writer.write_bool(self.deblocking_filter_control_present_flag);
        writer.write_bool(self.constrained_intra_pred_flag);
        writer.write_bool(self.redundant_pic_cnt_present_flag);

        if let Some(mrd) = &self.more_rbsp_data {
            writer.write_bool(mrd.transform_8x8_mode_flag);
            writer.write_bool(mrd.pic_scaling_matrix_present_flag);
            if let Some(present_flags) = &mrd.pic_scaling_list_present_flag {
                for (i, &present) in present_flags.iter().enumerate() {
                    writer.write_bool(present);
                    if present {
                        if i < 6 {
                            if let Some(lists) = &mrd.scaling_list_4x4 {
                                Sps::write_scaling_list(&mut writer, &lists[i]);
                            }
                        } else if let Some(lists) = &mrd.scaling_list_8x8 {
                            Sps::write_scaling_list(&mut writer, &lists[i - 6]);
                        }
                    }
                }
            }
            writer.write_se(mrd.second_chroma_qp_index_offset);
        }

        writer.write_bits(1, 1); // rbsp_stop_one_bit
        writer.finish()
    }

    pub fn parse(data: &[u8], sps: &Sps) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let pic_parameter_set_id = reader.read_ue()?;
        let seq_parameter_set_id = reader.read_ue()?;
        let entropy_coding_mode_flag = reader.read_bit()?;
        let bottom_field_pic_order_in_frame_present_flag = reader.read_bit()?;
        let num_slice_groups_minus1 = reader.read_ue()?;
        let slice_group = if num_slice_groups_minus1 > 0 {
            let slice_group_map_type = reader.read_ue()?;
            match slice_group_map_type {
                0 => {
                    let mut run_length_minus1 = Vec::new();
                    for _ in 0..=num_slice_groups_minus1 {
                        run_length_minus1.push(reader.read_ue()?);
                    }
                    Some(SliceGroup::Type0 {
                        slice_group_map_type,
                        run_length_minus1,
                    })
                }
                2 => {
                    let mut top_left = Vec::new();
                    let mut bottom_right = Vec::new();
                    for _ in 0..num_slice_groups_minus1 {
                        top_left.push(reader.read_ue()?);
                        bottom_right.push(reader.read_ue()?);
                    }
                    Some(SliceGroup::Type2 {
                        slice_group_map_type,
                        top_left,
                        bottom_right,
                    })
                }
                3..=5 => {
                    let slice_group_change_direction_flag = reader.read_bit()?;
                    let slice_group_change_rate_minus1 = reader.read_ue()?;
                    Some(SliceGroup::Type3_5 {
                        slice_group_map_type,
                        slice_group_change_direction_flag,
                        slice_group_change_rate_minus1,
                    })
                }
                6 => {
                    let pic_size_in_map_units_minus1 = reader.read_ue()?;
                    let bits = ((num_slice_groups_minus1 + 1) as f64).log2().ceil() as u8;
                    let mut slice_group_id = Vec::new();
                    for _ in 0..=pic_size_in_map_units_minus1 {
                        slice_group_id.push(reader.read_bits(bits)?);
                    }
                    Some(SliceGroup::Type6 {
                        slice_group_map_type,
                        pic_size_in_map_units_minus1,
                        slice_group_id,
                    })
                }
                _ => return Err(Error::InvalidSliceGroupMapType(slice_group_map_type)),
            }
        } else {
            None
        };

        let num_ref_idx_l0_default_active_minus1 = reader.read_ue()?;
        let num_ref_idx_l1_default_active_minus1 = reader.read_ue()?;
        let weighted_pred_flag = reader.read_bit()?;
        let weighted_bipred_idc = reader.read_bits(2)? as u8;
        let pic_init_qp_minus26 = reader.read_se()?;
        let pic_init_qs_minus26 = reader.read_se()?;
        let chroma_qp_index_offset = reader.read_se()?;
        let deblocking_filter_control_present_flag = reader.read_bit()?;
        let constrained_intra_pred_flag = reader.read_bit()?;
        let redundant_pic_cnt_present_flag = reader.read_bit()?;

        let more_rbsp_data = if reader.has_more_rbsp_data() {
            let transform_8x8_mode_flag = reader.read_bit()?;
            let pic_scaling_matrix_present_flag = reader.read_bit()?;
            let (pic_scaling_list_present_flag, scaling_list_4x4, scaling_list_8x8) =
                if pic_scaling_matrix_present_flag {
                    let chroma_format_idc = sps
                        .high_profile
                        .as_ref()
                        .ok_or(Error::MissingHighProfileData)?
                        .chroma_format_idc;
                    let num_lists = 6 + if transform_8x8_mode_flag {
                        if chroma_format_idc != 3 { 2 } else { 6 }
                    } else {
                        0
                    };
                    let mut present_flags = Vec::new();
                    let mut scaling_lists_4x4 = [[16u8; 16]; 6];
                    let mut scaling_lists_8x8 = [[16u8; 64]; 6];
                    for i in 0..num_lists {
                        let present = reader.read_bit()?;
                        present_flags.push(present);
                        if present {
                            if i < 6 {
                                scaling_lists_4x4[i] = Sps::parse_scaling_list(&mut reader)?;
                            } else {
                                scaling_lists_8x8[i - 6] = Sps::parse_scaling_list(&mut reader)?;
                            }
                        }
                    }
                    (
                        Some(present_flags),
                        Some(scaling_lists_4x4),
                        if transform_8x8_mode_flag {
                            Some(scaling_lists_8x8)
                        } else {
                            None
                        },
                    )
                } else {
                    (None, None, None)
                };
            let second_chroma_qp_index_offset = reader.read_se()?;
            Some(MoreRbspData {
                transform_8x8_mode_flag,
                pic_scaling_matrix_present_flag,
                pic_scaling_list_present_flag,
                scaling_list_4x4,
                scaling_list_8x8,
                second_chroma_qp_index_offset,
            })
        } else {
            None
        };

        Ok(Self {
            pic_parameter_set_id,
            seq_parameter_set_id,
            entropy_coding_mode_flag,
            bottom_field_pic_order_in_frame_present_flag,
            num_slice_groups_minus1,
            slice_group,
            num_ref_idx_l0_default_active_minus1,
            num_ref_idx_l1_default_active_minus1,
            weighted_pred_flag,
            weighted_bipred_idc,
            pic_init_qp_minus26,
            pic_init_qs_minus26,
            chroma_qp_index_offset,
            deblocking_filter_control_present_flag,
            constrained_intra_pred_flag,
            redundant_pic_cnt_present_flag,
            more_rbsp_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sps::{HighProfileData, PicOrderCnt, Sps};

    fn build_sps() -> Sps {
        Sps {
            profile_idc: 66,
            constraint_flags: 0b110000,
            level_idc: 31,
            seq_parameter_set_id: 0,
            high_profile: None,
            log2_max_frame_num_minus4: 0,
            pic_order_cnt: PicOrderCnt::Type0 {
                log2_max_pic_order_cnt_lsb_minus4: 2,
            },
            max_num_ref_frames: 4,
            gaps_in_frame_num_value_allowed_flag: false,
            pic_width_in_mbs_minus1: 119,
            pic_height_in_map_units_minus1: 67,
            frame_mbs_only_flag: true,
            mb_adaptive_frame_field_flag: None,
            direct_8x8_inference_flag: true,
            frame_cropping: None,
            vui: None,
        }
    }

    fn build_high_sps() -> Sps {
        let mut sps = build_sps();
        sps.profile_idc = 100;
        sps.high_profile = Some(HighProfileData {
            chroma_format_idc: 1,
            separate_colour_plane_flag: None,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            qpprime_y_zero_transform_bypass_flag: false,
            seq_scaling_matrix_present_flag: false,
            scaling_lists_4x4: None,
            scaling_lists_8x8: None,
        });
        sps
    }

    fn build_pps() -> Pps {
        Pps {
            pic_parameter_set_id: 0,
            seq_parameter_set_id: 0,
            entropy_coding_mode_flag: true,
            bottom_field_pic_order_in_frame_present_flag: false,
            num_slice_groups_minus1: 0,
            slice_group: None,
            num_ref_idx_l0_default_active_minus1: 3,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_pred_flag: true,
            weighted_bipred_idc: 2,
            pic_init_qp_minus26: -3,
            pic_init_qs_minus26: 0,
            chroma_qp_index_offset: -2,
            deblocking_filter_control_present_flag: true,
            constrained_intra_pred_flag: false,
            redundant_pic_cnt_present_flag: false,
            more_rbsp_data: None,
        }
    }

    fn assert_roundtrip(pps: &Pps, sps: &Sps) {
        let bytes1 = pps.to_bytes();
        let parsed = Pps::parse(&bytes1, sps).expect("failed to parse");
        let bytes2 = parsed.to_bytes();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_roundtrip_basic() {
        let sps = build_sps();
        let pps = build_pps();
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_cavlc() {
        let sps = build_sps();
        let mut pps = build_pps();
        pps.entropy_coding_mode_flag = false;
        pps.weighted_pred_flag = false;
        pps.weighted_bipred_idc = 0;
        pps.pic_init_qp_minus26 = 0;
        pps.chroma_qp_index_offset = 0;
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_more_rbsp_data() {
        let sps = build_high_sps();
        let mut pps = build_pps();
        pps.more_rbsp_data = Some(MoreRbspData {
            transform_8x8_mode_flag: true,
            pic_scaling_matrix_present_flag: false,
            pic_scaling_list_present_flag: None,
            scaling_list_4x4: None,
            scaling_list_8x8: None,
            second_chroma_qp_index_offset: -2,
        });
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_with_scaling_lists() {
        let sps = build_high_sps();
        let mut pps = build_pps();
        pps.more_rbsp_data = Some(MoreRbspData {
            transform_8x8_mode_flag: true,
            pic_scaling_matrix_present_flag: true,
            pic_scaling_list_present_flag: Some(vec![
                false, false, false, false, false, false, false, false,
            ]),
            scaling_list_4x4: Some([[16u8; 16]; 6]),
            scaling_list_8x8: Some([[16u8; 64]; 6]),
            second_chroma_qp_index_offset: -1,
        });
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_slice_group_type0() {
        let sps = build_sps();
        let mut pps = build_pps();
        pps.num_slice_groups_minus1 = 1;
        pps.slice_group = Some(SliceGroup::Type0 {
            slice_group_map_type: 0,
            run_length_minus1: vec![100, 200],
        });
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_slice_group_type2() {
        let sps = build_sps();
        let mut pps = build_pps();
        pps.num_slice_groups_minus1 = 2;
        pps.slice_group = Some(SliceGroup::Type2 {
            slice_group_map_type: 2,
            top_left: vec![0, 10],
            bottom_right: vec![5, 15],
        });
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_slice_group_type3() {
        let sps = build_sps();
        let mut pps = build_pps();
        pps.num_slice_groups_minus1 = 1;
        pps.slice_group = Some(SliceGroup::Type3_5 {
            slice_group_map_type: 3,
            slice_group_change_direction_flag: true,
            slice_group_change_rate_minus1: 10,
        });
        assert_roundtrip(&pps, &sps);
    }

    #[test]
    fn test_roundtrip_slice_group_type6() {
        let sps = build_sps();
        let mut pps = build_pps();
        pps.num_slice_groups_minus1 = 3;
        pps.slice_group = Some(SliceGroup::Type6 {
            slice_group_map_type: 6,
            pic_size_in_map_units_minus1: 3,
            slice_group_id: vec![0, 1, 2, 3],
        });
        assert_roundtrip(&pps, &sps);
    }
}
