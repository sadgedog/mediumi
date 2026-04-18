//! SPS (Sequence Parameter Set)
//!
//! RBSP data when nal_unit_type = 7 (SPS)
//! All fields below profile/level are encoded using Exp-Golomb coding
//!
//! ```text
//! SPS construction
//! ┌───────────────────────────────────────────────────────┐
//! │  profile_idc (8 bits)                                 │ <- Profile (e.g. 66: Baseline, 77: Main,100: High)
//! │  constraint_flags (6 bits)                            │ <- constraint_set0..5_flag
//! │  reserved_zero_2bits (2 bits)                         │ <- Must be 0
//! │  level_idc (8 bits)                                   │ <- Level (e.g. 31: 3.1, 40: 4.0)
//! │  seq_parameter_set_id: ue(v)                          │ <- SPS identifier (0..31)
//! ├───────────────────────────────────────────────────────┤
//!
//! If profile_idc = 100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
//! ┌───────────────────────────────────────────────────────┐
//! │  chroma_format_idc: ue(v)                             │ <- 0: Monochrome, 1: 4:2:0, 2: 4:2:2, 3: 4:4:4
//! │  separate_colour_plane_flag (1 bit)                   │ <- Present only if chroma_format_idc == 3
//! │  bit_depth_luma_minus8: ue(v)                         │
//! │  bit_depth_chroma_minus8: ue(v)                       │
//! │  qpprime_y_zero_transform_bypass_flag (1 bit)         │
//! │  seq_scaling_matrix_present_flag (1 bit)              │ <- If 1, followed by scaling lists
//! │  scaling_lists (variable)                             │ <- 4x4 (x6) + 8x8 (x2 or x6), delta-coded via se(v)
//! ├───────────────────────────────────────────────────────┤
//!
//! ┌───────────────────────────────────────────────────────┐
//! │  log2_max_frame_num_minus4: ue(v)                     │
//! │  pic_order_cnt_type: ue(v)                            │ <- 0, 1, or 2
//! ├───────────────────────────────────────────────────────┤
//!
//! If pic_order_cnt_type = 0
//! ┌───────────────────────────────────────────────────────┐
//! │    log2_max_pic_order_cnt_lsb_minus4: ue(v)           │
//! ├───────────────────────────────────────────────────────┤
//!
//! If pic_order_cnt_type = 1
//! ┌───────────────────────────────────────────────────────┐
//! │    delta_pic_order_always_zero_flag (1 bit)           │
//! │    offset_for_non_ref_pic: se(v)                      │
//! │    offset_for_top_to_bottom_field: se(v)              │
//! │    num_ref_frames_in_pic_order_cnt_cycle: ue(v)       │
//! │    offset_for_ref_frame[i]: se(v)                     │ <- Repeated
//! ├───────────────────────────────────────────────────────┤
//!
//! If pic_order_cnt_type = 2
//! ┌───────────────────────────────────────────────────────┐
//! │    (no additional fields)                             │
//! ├───────────────────────────────────────────────────────┤
//!
//! ┌───────────────────────────────────────────────────────┐
//! │  max_num_ref_frames: ue(v)                            │
//! │  gaps_in_frame_num_value_allowed_flag (1 bit)         │
//! │  pic_width_in_mbs_minus1: ue(v)                       │ <- width = (value + 1) * 16
//! │  pic_height_in_map_units_minus1: ue(v)                │ <- height = (value + 1) * 16
//! │  frame_mbs_only_flag (1 bit)                          │ <- If 0, followed by mb_adaptive_frame_field_flag
//! │  mb_adaptive_frame_field_flag (1 bit)                 │ <- Present only if frame_mbs_only_flag == 0
//! │  direct_8x8_inference_flag (1 bit)                    │
//! ├───────────────────────────────────────────────────────┤
//! │  frame_cropping_flag (1 bit)                          │ <- If 1, followed by cropping offsets
//! │  frame_crop_left_offset: ue(v)                        │
//! │  frame_crop_right_offset: ue(v)                       │
//! │  frame_crop_top_offset: ue(v)                         │
//! │  frame_crop_bottom_offset: ue(v)                      │
//! ├───────────────────────────────────────────────────────┤
//! │  vui_parameters_present_flag (1 bit)                  │ <- If 1, followed by VUI parameters
//! │  vui_parameters (variable)                            │
//! └───────────────────────────────────────────────────────┘
//! ```

use crate::{
    error::Error,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug, Clone)]
pub struct HighProfileData {
    pub chroma_format_idc: u8,
    pub separate_colour_plane_flag: Option<bool>,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub qpprime_y_zero_transform_bypass_flag: bool,
    pub seq_scaling_matrix_present_flag: bool,
    pub scaling_lists_4x4: Option<[[u8; 16]; 6]>,
    pub scaling_lists_8x8: Option<[[u8; 64]; 6]>,
}

#[derive(Debug, Clone)]
pub enum PicOrderCnt {
    Type0 {
        log2_max_pic_order_cnt_lsb_minus4: u8,
    },
    Type1 {
        delta_pic_order_always_zero_flag: bool,
        offset_for_non_ref_pic: i32,
        offset_for_top_to_bottom_field: i32,
        num_ref_frames_in_pic_order_cnt_cycle: u8,
        offset_for_ref_frame: Vec<i32>,
    },
    Type2,
}

#[derive(Debug, Clone)]
pub struct FrameCropping {
    pub left_offset: u32,
    pub right_offset: u32,
    pub top_offset: u32,
    pub bottom_offset: u32,
}

#[derive(Debug, Clone)]
pub struct HrdParameters {
    pub cpb_cnt_minus1: u32,
    pub bit_rate_scale: u8,
    pub cpb_size_scale: u8,
    pub bit_rate_value_minus1: Vec<u32>,
    pub cpb_size_value_minus1: Vec<u32>,
    pub cbr_flag: Vec<bool>,
    pub initial_cpb_removal_delay_length_minus1: u8,
    pub cpb_removal_delay_length_minus1: u8,
    pub dpb_output_delay_length_minus1: u8,
    pub time_offset_length: u8,
}

#[derive(Debug, Clone)]
pub struct AspectRatioInfo {
    pub aspect_ratio_idc: u8,
    pub sar_width: Option<u16>, // present only if aspect_ratio_idc == 255 (Extended_SAR)
    pub sar_height: Option<u16>, // present only if aspect_ratio_idc == 255 (Extended_SAR)
}

#[derive(Debug, Clone)]
pub struct ColourDescription {
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

#[derive(Debug, Clone)]
pub struct VideoSignalType {
    pub video_format: u8,
    pub video_full_range_flag: bool,
    pub colour_description: Option<ColourDescription>,
}

#[derive(Debug, Clone)]
pub struct ChromaLocInfo {
    pub chroma_sample_loc_type_top_field: u32,
    pub chroma_sample_loc_type_bottom_field: u32,
}

#[derive(Debug, Clone)]
pub struct TimingInfo {
    pub num_units_in_tick: u32,
    pub time_scale: u32,
    pub fixed_frame_rate_flag: bool,
}

#[derive(Debug, Clone)]
pub struct BitstreamRestriction {
    pub motion_vectors_over_pic_boundaries_flag: bool,
    pub max_bytes_per_pic_denom: u32,
    pub max_bits_per_mb_denom: u32,
    pub log2_max_mv_length_horizontal: u32,
    pub log2_max_mv_length_vertical: u32,
    pub max_num_reorder_frames: u32,
    pub max_dec_frame_buffering: u32,
}

#[derive(Debug, Clone)]
pub struct VuiHrd {
    pub nal_hrd_parameters: Option<HrdParameters>,
    pub vcl_hrd_parameters: Option<HrdParameters>,
    pub low_delay_hrd_flag: bool,
}

#[derive(Debug, Clone)]
pub struct VuiParameters {
    pub aspect_ratio_info: Option<AspectRatioInfo>,
    pub overscan_appropriate_flag: Option<bool>,
    pub video_signal_type: Option<VideoSignalType>,
    pub chroma_loc_info: Option<ChromaLocInfo>,
    pub timing_info: Option<TimingInfo>,
    pub hrd: Option<VuiHrd>,
    pub pic_struct_present_flag: bool,
    pub bitstream_restriction: Option<BitstreamRestriction>,
}

#[derive(Debug, Clone)]
pub struct Sps {
    pub profile_idc: u8,
    pub constraint_flags: u8,
    pub level_idc: u8,
    pub seq_parameter_set_id: u8,
    pub high_profile: Option<HighProfileData>,
    pub log2_max_frame_num_minus4: u8,
    pub pic_order_cnt: PicOrderCnt,
    pub max_num_ref_frames: u8,
    pub gaps_in_frame_num_value_allowed_flag: bool,
    pub pic_width_in_mbs_minus1: u32,
    pub pic_height_in_map_units_minus1: u32,
    pub frame_mbs_only_flag: bool,
    pub mb_adaptive_frame_field_flag: Option<bool>,
    pub direct_8x8_inference_flag: bool,
    pub frame_cropping: Option<FrameCropping>,
    pub vui: Option<VuiParameters>,
}

impl Sps {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        writer.write_bits(self.profile_idc as u32, 8);
        writer.write_bits(self.constraint_flags as u32, 6);
        writer.write_bits(0, 2); // reserved_zero_2bits
        writer.write_bits(self.level_idc as u32, 8);
        writer.write_ue(self.seq_parameter_set_id as u32);

        if let Some(hp) = &self.high_profile {
            writer.write_ue(hp.chroma_format_idc as u32);
            if let Some(flag) = hp.separate_colour_plane_flag {
                writer.write_bool(flag);
            }
            writer.write_ue(hp.bit_depth_luma_minus8 as u32);
            writer.write_ue(hp.bit_depth_chroma_minus8 as u32);
            writer.write_bool(hp.qpprime_y_zero_transform_bypass_flag);
            writer.write_bool(hp.seq_scaling_matrix_present_flag);
            if hp.seq_scaling_matrix_present_flag {
                let num_lists = if hp.chroma_format_idc != 3 { 8 } else { 12 };
                for i in 0..num_lists {
                    if i < 6 {
                        if let Some(lists) = &hp.scaling_lists_4x4 {
                            let is_default = lists[i] == [16u8; 16];
                            writer.write_bool(!is_default);
                            if !is_default {
                                Self::write_scaling_list(&mut writer, &lists[i]);
                            }
                        } else {
                            writer.write_bool(false);
                        }
                    } else if let Some(lists) = &hp.scaling_lists_8x8 {
                        let is_default = lists[i - 6] == [16u8; 64];
                        writer.write_bool(!is_default);
                        if !is_default {
                            Self::write_scaling_list(&mut writer, &lists[i - 6]);
                        }
                    } else {
                        writer.write_bool(false);
                    }
                }
            }
        }

        writer.write_ue(self.log2_max_frame_num_minus4 as u32);
        match &self.pic_order_cnt {
            PicOrderCnt::Type0 {
                log2_max_pic_order_cnt_lsb_minus4,
            } => {
                writer.write_ue(0);
                writer.write_ue(*log2_max_pic_order_cnt_lsb_minus4 as u32);
            }
            PicOrderCnt::Type1 {
                delta_pic_order_always_zero_flag,
                offset_for_non_ref_pic,
                offset_for_top_to_bottom_field,
                num_ref_frames_in_pic_order_cnt_cycle,
                offset_for_ref_frame,
            } => {
                writer.write_ue(1);
                writer.write_bool(*delta_pic_order_always_zero_flag);
                writer.write_se(*offset_for_non_ref_pic);
                writer.write_se(*offset_for_top_to_bottom_field);
                writer.write_ue(*num_ref_frames_in_pic_order_cnt_cycle as u32);
                for offset in offset_for_ref_frame {
                    writer.write_se(*offset);
                }
            }
            PicOrderCnt::Type2 => {
                writer.write_ue(2);
            }
        }

        writer.write_ue(self.max_num_ref_frames as u32);
        writer.write_bool(self.gaps_in_frame_num_value_allowed_flag);
        writer.write_ue(self.pic_width_in_mbs_minus1);
        writer.write_ue(self.pic_height_in_map_units_minus1);
        writer.write_bool(self.frame_mbs_only_flag);
        if let Some(flag) = self.mb_adaptive_frame_field_flag {
            writer.write_bool(flag);
        }
        writer.write_bool(self.direct_8x8_inference_flag);

        if let Some(cropping) = &self.frame_cropping {
            writer.write_bool(true);
            writer.write_ue(cropping.left_offset);
            writer.write_ue(cropping.right_offset);
            writer.write_ue(cropping.top_offset);
            writer.write_ue(cropping.bottom_offset);
        } else {
            writer.write_bool(false);
        }

        if let Some(vui) = &self.vui {
            writer.write_bool(true);

            if let Some(ari) = &vui.aspect_ratio_info {
                writer.write_bool(true);
                writer.write_bits(ari.aspect_ratio_idc as u32, 8);
                if ari.aspect_ratio_idc == 255
                    && let (Some(w), Some(h)) = (ari.sar_width, ari.sar_height)
                {
                    writer.write_bits(w as u32, 16);
                    writer.write_bits(h as u32, 16);
                }
            } else {
                writer.write_bool(false);
            }

            if let Some(flag) = vui.overscan_appropriate_flag {
                writer.write_bool(true);
                writer.write_bool(flag);
            } else {
                writer.write_bool(false);
            }

            if let Some(vst) = &vui.video_signal_type {
                writer.write_bool(true);
                writer.write_bits(vst.video_format as u32, 3);
                writer.write_bool(vst.video_full_range_flag);
                if let Some(cd) = &vst.colour_description {
                    writer.write_bool(true);
                    writer.write_bits(cd.colour_primaries as u32, 8);
                    writer.write_bits(cd.transfer_characteristics as u32, 8);
                    writer.write_bits(cd.matrix_coefficients as u32, 8);
                } else {
                    writer.write_bool(false);
                }
            } else {
                writer.write_bool(false);
            }

            if let Some(cli) = &vui.chroma_loc_info {
                writer.write_bool(true);
                writer.write_ue(cli.chroma_sample_loc_type_top_field);
                writer.write_ue(cli.chroma_sample_loc_type_bottom_field);
            } else {
                writer.write_bool(false);
            }

            if let Some(ti) = &vui.timing_info {
                writer.write_bool(true);
                writer.write_bits(ti.num_units_in_tick, 32);
                writer.write_bits(ti.time_scale, 32);
                writer.write_bool(ti.fixed_frame_rate_flag);
            } else {
                writer.write_bool(false);
            }

            if let Some(hrd) = &vui.hrd {
                if let Some(nal) = &hrd.nal_hrd_parameters {
                    writer.write_bool(true);
                    Self::write_hrd_params(&mut writer, nal);
                } else {
                    writer.write_bool(false);
                }
                if let Some(vcl) = &hrd.vcl_hrd_parameters {
                    writer.write_bool(true);
                    Self::write_hrd_params(&mut writer, vcl);
                } else {
                    writer.write_bool(false);
                }
                writer.write_bool(hrd.low_delay_hrd_flag);
            } else {
                writer.write_bool(false); // nal_hrd_parameters_present_flag
                writer.write_bool(false); // vcl_hrd_parameters_present_flag
            }

            writer.write_bool(vui.pic_struct_present_flag);

            if let Some(bsr) = &vui.bitstream_restriction {
                writer.write_bool(true);
                writer.write_bool(bsr.motion_vectors_over_pic_boundaries_flag);
                writer.write_ue(bsr.max_bytes_per_pic_denom);
                writer.write_ue(bsr.max_bits_per_mb_denom);
                writer.write_ue(bsr.log2_max_mv_length_horizontal);
                writer.write_ue(bsr.log2_max_mv_length_vertical);
                writer.write_ue(bsr.max_num_reorder_frames);
                writer.write_ue(bsr.max_dec_frame_buffering);
            } else {
                writer.write_bool(false);
            }

            writer.write_bits(1, 1); // rbsp_stop_one_bit
        } else {
            writer.write_bool(false);
            writer.write_bits(1, 1); // rbsp_stop_one_bit
        }

        writer.finish()
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let profile_idc = reader.read_bits(8)? as u8;
        let constraint_flags = reader.read_bits(6)? as u8;
        let reserved_zero_2bits = reader.read_bits(2)?;
        if reserved_zero_2bits != 0b00 {
            return Err(Error::InvalidReservedData(reserved_zero_2bits));
        }
        let level_idc = reader.read_bits(8)? as u8;
        let seq_parameter_set_id = reader.read_ue()? as u8;

        let high_profile = if Self::is_high_profile(profile_idc) {
            let chroma_format_idc = reader.read_ue()? as u8;
            let separate_colour_plane_flag = if chroma_format_idc == 3 {
                Some(reader.read_bit()?)
            } else {
                None
            };
            let bit_depth_luma_minus8 = reader.read_ue()? as u8;
            let bit_depth_chroma_minus8 = reader.read_ue()? as u8;
            let qpprime_y_zero_transform_bypass_flag = reader.read_bit()?;
            let seq_scaling_matrix_present_flag = reader.read_bit()?;
            let (scaling_lists_4x4, scaling_lists_8x8) = if seq_scaling_matrix_present_flag {
                let num_lists = if chroma_format_idc != 3 { 8 } else { 12 };
                let mut scaling_lists_4x4 = [[16u8; 16]; 6];
                let mut scaling_lists_8x8 = [[16u8; 64]; 6];
                for i in 0..num_lists {
                    let present = reader.read_bit()?;
                    if present {
                        if i < 6 {
                            scaling_lists_4x4[i] = Self::parse_scaling_list(&mut reader)?;
                        } else {
                            scaling_lists_8x8[i - 6] = Self::parse_scaling_list(&mut reader)?;
                        }
                    }
                }
                (Some(scaling_lists_4x4), Some(scaling_lists_8x8))
            } else {
                (None, None)
            };

            Some(HighProfileData {
                chroma_format_idc,
                separate_colour_plane_flag,
                bit_depth_luma_minus8,
                bit_depth_chroma_minus8,
                qpprime_y_zero_transform_bypass_flag,
                seq_scaling_matrix_present_flag,
                scaling_lists_4x4,
                scaling_lists_8x8,
            })
        } else {
            None
        };

        let log2_max_frame_num_minus4 = reader.read_ue()? as u8;
        let pic_order_cnt_type = reader.read_ue()?;
        let pic_order_cnt = if pic_order_cnt_type == 0 {
            let log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()? as u8;
            PicOrderCnt::Type0 {
                log2_max_pic_order_cnt_lsb_minus4,
            }
        } else if pic_order_cnt_type == 1 {
            let delta_pic_order_always_zero_flag = reader.read_bit()?;
            let offset_for_non_ref_pic = reader.read_se()?;
            let offset_for_top_to_bottom_field = reader.read_se()?;
            let num_ref_frames_in_pic_order_cnt_cycle = reader.read_ue()? as u8;
            let mut offset_for_ref_frame = Vec::new();
            for _ in 0..num_ref_frames_in_pic_order_cnt_cycle {
                offset_for_ref_frame.push(reader.read_se()?);
            }
            PicOrderCnt::Type1 {
                delta_pic_order_always_zero_flag,
                offset_for_non_ref_pic,
                offset_for_top_to_bottom_field,
                num_ref_frames_in_pic_order_cnt_cycle,
                offset_for_ref_frame,
            }
        } else if pic_order_cnt_type == 2 {
            PicOrderCnt::Type2
        } else {
            return Err(Error::InvalidPicOrderCntType(pic_order_cnt_type));
        };

        let max_num_ref_frames = reader.read_ue()? as u8;
        let gaps_in_frame_num_value_allowed_flag = reader.read_bit()?;
        let pic_width_in_mbs_minus1 = reader.read_ue()?;
        let pic_height_in_map_units_minus1 = reader.read_ue()?;
        let frame_mbs_only_flag = reader.read_bit()?;
        let mb_adaptive_frame_field_flag = if !frame_mbs_only_flag {
            Some(reader.read_bit()?)
        } else {
            None
        };

        let direct_8x8_inference_flag = reader.read_bit()?;
        let frame_cropping_flag = reader.read_bit()?;
        let frame_cropping = if frame_cropping_flag {
            let left_offset = reader.read_ue()?;
            let right_offset = reader.read_ue()?;
            let top_offset = reader.read_ue()?;
            let bottom_offset = reader.read_ue()?;
            Some(FrameCropping {
                left_offset,
                right_offset,
                top_offset,
                bottom_offset,
            })
        } else {
            None
        };

        let vui_parameters_present_flag = reader.read_bit()?;
        let vui = if vui_parameters_present_flag {
            let aspect_ratio_info = if reader.read_bit()? {
                let idc = reader.read_bits(8)? as u8;
                let (sar_width, sar_height) = if idc == 255 {
                    (
                        Some(reader.read_bits(16)? as u16),
                        Some(reader.read_bits(16)? as u16),
                    )
                } else {
                    (None, None)
                };
                Some(AspectRatioInfo {
                    aspect_ratio_idc: idc,
                    sar_width,
                    sar_height,
                })
            } else {
                None
            };

            let overscan_appropriate_flag = if reader.read_bit()? {
                Some(reader.read_bit()?)
            } else {
                None
            };

            let video_signal_type = if reader.read_bit()? {
                let video_format = reader.read_bits(3)? as u8;
                let video_full_range_flag = reader.read_bit()?;
                let colour_description = if reader.read_bit()? {
                    Some(ColourDescription {
                        colour_primaries: reader.read_bits(8)? as u8,
                        transfer_characteristics: reader.read_bits(8)? as u8,
                        matrix_coefficients: reader.read_bits(8)? as u8,
                    })
                } else {
                    None
                };
                Some(VideoSignalType {
                    video_format,
                    video_full_range_flag,
                    colour_description,
                })
            } else {
                None
            };

            let chroma_loc_info = if reader.read_bit()? {
                Some(ChromaLocInfo {
                    chroma_sample_loc_type_top_field: reader.read_ue()?,
                    chroma_sample_loc_type_bottom_field: reader.read_ue()?,
                })
            } else {
                None
            };

            let timing_info = if reader.read_bit()? {
                Some(TimingInfo {
                    num_units_in_tick: reader.read_bits(32)?,
                    time_scale: reader.read_bits(32)?,
                    fixed_frame_rate_flag: reader.read_bit()?,
                })
            } else {
                None
            };

            let nal_hrd_parameters = if reader.read_bit()? {
                Some(Self::parse_hrd_params(&mut reader)?)
            } else {
                None
            };

            let vcl_hrd_parameters = if reader.read_bit()? {
                Some(Self::parse_hrd_params(&mut reader)?)
            } else {
                None
            };

            let hrd = if nal_hrd_parameters.is_some() || vcl_hrd_parameters.is_some() {
                Some(VuiHrd {
                    nal_hrd_parameters,
                    vcl_hrd_parameters,
                    low_delay_hrd_flag: reader.read_bit()?,
                })
            } else {
                None
            };

            let pic_struct_present_flag = reader.read_bit()?;

            let bitstream_restriction = if reader.read_bit()? {
                Some(BitstreamRestriction {
                    motion_vectors_over_pic_boundaries_flag: reader.read_bit()?,
                    max_bytes_per_pic_denom: reader.read_ue()?,
                    max_bits_per_mb_denom: reader.read_ue()?,
                    log2_max_mv_length_horizontal: reader.read_ue()?,
                    log2_max_mv_length_vertical: reader.read_ue()?,
                    max_num_reorder_frames: reader.read_ue()?,
                    max_dec_frame_buffering: reader.read_ue()?,
                })
            } else {
                None
            };

            Some(VuiParameters {
                aspect_ratio_info,
                overscan_appropriate_flag,
                video_signal_type,
                chroma_loc_info,
                timing_info,
                hrd,
                pic_struct_present_flag,
                bitstream_restriction,
            })
        } else {
            None
        };

        Ok(Self {
            profile_idc,
            constraint_flags,
            level_idc,
            seq_parameter_set_id,
            high_profile,
            log2_max_frame_num_minus4,
            pic_order_cnt,
            max_num_ref_frames,
            gaps_in_frame_num_value_allowed_flag,
            pic_width_in_mbs_minus1,
            pic_height_in_map_units_minus1,
            frame_mbs_only_flag,
            mb_adaptive_frame_field_flag,
            direct_8x8_inference_flag,
            frame_cropping,
            vui,
        })
    }

    fn is_high_profile(profile_idc: u8) -> bool {
        matches!(
            profile_idc,
            100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135
        )
    }

    pub fn write_scaling_list(writer: &mut BitstreamWriter, list: &[u8]) {
        let mut last_scale: i32 = 8;
        for &scale in list {
            let delta = (scale as i32 - last_scale + 256) % 256;
            let delta = if delta > 128 { delta - 256 } else { delta };
            writer.write_se(delta);
            last_scale = scale as i32;
        }
    }

    pub fn parse_scaling_list<const N: usize>(
        reader: &mut BitstreamReader,
    ) -> Result<[u8; N], Error> {
        let mut scaling_list = [0u8; N];
        let mut last_scale: i32 = 8;
        let mut next_scale: i32 = 8;

        for item in scaling_list.iter_mut() {
            if next_scale != 0 {
                let delta_scale = reader.read_se()?;
                next_scale = (last_scale + delta_scale + 256) % 256;
            }
            *item = if next_scale == 0 {
                last_scale as u8
            } else {
                next_scale as u8
            };
            last_scale = *item as i32;
        }

        Ok(scaling_list)
    }

    fn write_hrd_params(writer: &mut BitstreamWriter, hrd: &HrdParameters) {
        writer.write_ue(hrd.cpb_cnt_minus1);
        writer.write_bits(hrd.bit_rate_scale as u32, 4);
        writer.write_bits(hrd.cpb_size_scale as u32, 4);
        for i in 0..=hrd.cpb_cnt_minus1 as usize {
            writer.write_ue(hrd.bit_rate_value_minus1[i]);
            writer.write_ue(hrd.cpb_size_value_minus1[i]);
            writer.write_bool(hrd.cbr_flag[i]);
        }
        writer.write_bits(hrd.initial_cpb_removal_delay_length_minus1 as u32, 5);
        writer.write_bits(hrd.cpb_removal_delay_length_minus1 as u32, 5);
        writer.write_bits(hrd.dpb_output_delay_length_minus1 as u32, 5);
        writer.write_bits(hrd.time_offset_length as u32, 5);
    }

    fn parse_hrd_params(reader: &mut BitstreamReader) -> Result<HrdParameters, Error> {
        let cpb_cnt_minus1 = reader.read_ue()?;
        let bit_rate_scale = reader.read_bits(4)? as u8;
        let cpb_size_scale = reader.read_bits(4)? as u8;

        let mut bit_rate_value_minus1 = Vec::with_capacity(cpb_cnt_minus1 as usize + 1);
        let mut cpb_size_value_minus1 = Vec::with_capacity(cpb_cnt_minus1 as usize + 1);
        let mut cbr_flag = Vec::with_capacity(cpb_cnt_minus1 as usize + 1);
        for _ in 0..=cpb_cnt_minus1 {
            bit_rate_value_minus1.push(reader.read_ue()?);
            cpb_size_value_minus1.push(reader.read_ue()?);
            cbr_flag.push(reader.read_bit()?);
        }

        let initial_cpb_removal_delay_length_minus1 = reader.read_bits(5)? as u8;
        let cpb_removal_delay_length_minus1 = reader.read_bits(5)? as u8;
        let dpb_output_delay_length_minus1 = reader.read_bits(5)? as u8;
        let time_offset_length = reader.read_bits(5)? as u8;

        Ok(HrdParameters {
            cpb_cnt_minus1,
            bit_rate_scale,
            cpb_size_scale,
            bit_rate_value_minus1,
            cpb_size_value_minus1,
            cbr_flag,
            initial_cpb_removal_delay_length_minus1,
            cpb_removal_delay_length_minus1,
            dpb_output_delay_length_minus1,
            time_offset_length,
        })
    }

    /// Width
    pub fn width(&self) -> u32 {
        (self.pic_width_in_mbs_minus1 + 1) * 16
    }

    /// Height
    pub fn height(&self) -> u32 {
        (self.pic_height_in_map_units_minus1 + 1) * 16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// to_bytes -> parse -> to_bytes roundtrip helper
    fn assert_roundtrip(sps: &Sps) {
        let bytes1 = sps.to_bytes();
        let parsed = Sps::parse(&bytes1).expect("failed to parse");
        let bytes2 = parsed.to_bytes();
        assert_eq!(bytes1, bytes2);
    }

    fn build_sps() -> Sps {
        Sps {
            profile_idc: 66, // Baseline
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
            pic_width_in_mbs_minus1: 119,       // 1920px
            pic_height_in_map_units_minus1: 67, // 1088px
            frame_mbs_only_flag: true,
            mb_adaptive_frame_field_flag: None,
            direct_8x8_inference_flag: true,
            frame_cropping: None,
            vui: None,
        }
    }

    #[test]
    fn test_roundtrip_baseline() {
        assert_roundtrip(&build_sps());
    }

    #[test]
    fn test_roundtrip_with_frame_cropping() {
        let mut sps = build_sps();
        sps.frame_cropping = Some(FrameCropping {
            left_offset: 0,
            right_offset: 0,
            top_offset: 0,
            bottom_offset: 4, // 1088 -> 1080
        });
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_interlaced() {
        let mut sps = build_sps();
        sps.frame_mbs_only_flag = false;
        sps.mb_adaptive_frame_field_flag = Some(true);
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_pic_order_cnt_type1() {
        let mut sps = build_sps();
        sps.pic_order_cnt = PicOrderCnt::Type1 {
            delta_pic_order_always_zero_flag: false,
            offset_for_non_ref_pic: -2,
            offset_for_top_to_bottom_field: 0,
            num_ref_frames_in_pic_order_cnt_cycle: 2,
            offset_for_ref_frame: vec![1, -1],
        };
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_pic_order_cnt_type2() {
        let mut sps = build_sps();
        sps.pic_order_cnt = PicOrderCnt::Type2;
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_high_profile() {
        let mut sps = build_sps();
        sps.profile_idc = 100; // High
        sps.high_profile = Some(HighProfileData {
            chroma_format_idc: 1, // 4:2:0
            separate_colour_plane_flag: None,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            qpprime_y_zero_transform_bypass_flag: false,
            seq_scaling_matrix_present_flag: false,
            scaling_lists_4x4: None,
            scaling_lists_8x8: None,
        });
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_high_profile_with_scaling_lists() {
        let mut sps = build_sps();
        sps.profile_idc = 100;
        sps.high_profile = Some(HighProfileData {
            chroma_format_idc: 1,
            separate_colour_plane_flag: None,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            qpprime_y_zero_transform_bypass_flag: false,
            seq_scaling_matrix_present_flag: true,
            scaling_lists_4x4: Some([[16u8; 16]; 6]), // default
            scaling_lists_8x8: Some([[16u8; 64]; 6]), // default
        });
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_roundtrip_high_profile_444() {
        let mut sps = build_sps();
        sps.profile_idc = 244; // High 4:4:4
        sps.high_profile = Some(HighProfileData {
            chroma_format_idc: 3, // 4:4:4
            separate_colour_plane_flag: Some(false),
            bit_depth_luma_minus8: 2,
            bit_depth_chroma_minus8: 2,
            qpprime_y_zero_transform_bypass_flag: false,
            seq_scaling_matrix_present_flag: false,
            scaling_lists_4x4: None,
            scaling_lists_8x8: None,
        });
        assert_roundtrip(&sps);
    }

    #[test]
    fn test_width_height() {
        let sps = build_sps();
        assert_eq!(sps.width(), 1920);
        assert_eq!(sps.height(), 1088);
    }
}
