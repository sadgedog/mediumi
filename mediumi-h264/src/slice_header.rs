// slice_layer_without_partitioning_rbsp -> NonIDR

use crate::{
    error::Error,
    nal::NalUnitType,
    pps::{Pps, SliceGroup},
    sps::{self, Sps},
    util::bitstream::{BitstreamReader, BitstreamWriter},
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
    Idc0 { abs_diff_pic_num_minus1: u32 },
    Idc1 { abs_diff_pic_num_minus1: u32 },
    Idc2 { long_term_pic_num: u32 },
    // idc=3: terminator (not stored, written in to_bytes)
}

#[derive(Debug)]
pub enum MvcModificationCommand {
    Idc0 { abs_diff_pic_num_minus1: u32 },
    Idc1 { abs_diff_pic_num_minus1: u32 },
    Idc2 { long_term_pic_num: u32 },
    // idc=3: terminator (not stored, written in to_bytes)
    Idc4 { abs_diff_view_idx_minus1: u32 },
    Idc5 { abs_diff_view_idx_minus1: u32 },
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
    fn write_commands(writer: &mut BitstreamWriter, list: &Option<RefPicListMvcModificationList>) {
        if let Some(list) = list {
            writer.write_bool(list.flag);
            if list.flag {
                for cmd in &list.commands {
                    match cmd {
                        MvcModificationCommand::Idc0 {
                            abs_diff_pic_num_minus1,
                        } => {
                            writer.write_ue(0); // modification_of_pic_nums_idc
                            writer.write_ue(*abs_diff_pic_num_minus1);
                        }
                        MvcModificationCommand::Idc1 {
                            abs_diff_pic_num_minus1,
                        } => {
                            writer.write_ue(1);
                            writer.write_ue(*abs_diff_pic_num_minus1);
                        }
                        MvcModificationCommand::Idc2 { long_term_pic_num } => {
                            writer.write_ue(2);
                            writer.write_ue(*long_term_pic_num);
                        }
                        MvcModificationCommand::Idc4 {
                            abs_diff_view_idx_minus1,
                        } => {
                            writer.write_ue(4);
                            writer.write_ue(*abs_diff_view_idx_minus1);
                        }
                        MvcModificationCommand::Idc5 {
                            abs_diff_view_idx_minus1,
                        } => {
                            writer.write_ue(5);
                            writer.write_ue(*abs_diff_view_idx_minus1);
                        }
                    }
                }
                writer.write_ue(3); // terminator
            }
        }
    }

    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        Self::write_commands(writer, &self.l0);
        Self::write_commands(writer, &self.l1);
    }

    fn parse_commands(
        reader: &mut BitstreamReader,
    ) -> Result<RefPicListMvcModificationList, Error> {
        let flag = reader.read_bit()?;
        let mut commands = Vec::new();

        if flag {
            loop {
                let idc = reader.read_ue()?;
                match idc {
                    0 => commands.push(MvcModificationCommand::Idc0 {
                        abs_diff_pic_num_minus1: reader.read_ue()?,
                    }),
                    1 => commands.push(MvcModificationCommand::Idc1 {
                        abs_diff_pic_num_minus1: reader.read_ue()?,
                    }),
                    2 => commands.push(MvcModificationCommand::Idc2 {
                        long_term_pic_num: reader.read_ue()?,
                    }),
                    3 => break,
                    4 => commands.push(MvcModificationCommand::Idc4 {
                        abs_diff_view_idx_minus1: reader.read_ue()?,
                    }),
                    5 => commands.push(MvcModificationCommand::Idc5 {
                        abs_diff_view_idx_minus1: reader.read_ue()?,
                    }),
                    _ => return Err(Error::InvalidModificationOfPicNumsIdc(idc)),
                }
            }
        }

        Ok(RefPicListMvcModificationList { flag, commands })
    }

    pub fn parse(reader: &mut BitstreamReader, slice_type: &SliceType) -> Result<Self, Error> {
        // l0: slice_type != I && != SI
        let l0 = if slice_type != &SliceType::I && slice_type != &SliceType::SI {
            Some(Self::parse_commands(reader)?)
        } else {
            None
        };

        // l1: slice_type == B
        let l1 = if slice_type == &SliceType::B {
            Some(Self::parse_commands(reader)?)
        } else {
            None
        };

        Ok(Self { l0, l1 })
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
    fn write_commands(writer: &mut BitstreamWriter, list: &Option<RefPicListModificationList>) {
        if let Some(list) = list {
            writer.write_bool(list.flag);
            if list.flag {
                for cmd in &list.commands {
                    match cmd {
                        ModificationCommand::Idc0 {
                            abs_diff_pic_num_minus1,
                        } => {
                            writer.write_ue(0);
                            writer.write_ue(*abs_diff_pic_num_minus1);
                        }
                        ModificationCommand::Idc1 {
                            abs_diff_pic_num_minus1,
                        } => {
                            writer.write_ue(1);
                            writer.write_ue(*abs_diff_pic_num_minus1);
                        }
                        ModificationCommand::Idc2 { long_term_pic_num } => {
                            writer.write_ue(2);
                            writer.write_ue(*long_term_pic_num);
                        }
                    }
                }
                writer.write_ue(3); // terminator
            }
        }
    }

    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        Self::write_commands(writer, &self.l0);
        Self::write_commands(writer, &self.l1);
    }

    fn parse_commands(reader: &mut BitstreamReader) -> Result<RefPicListModificationList, Error> {
        let flag = reader.read_bit()?;
        let mut commands = Vec::new();

        if flag {
            loop {
                let idc = reader.read_ue()?;
                match idc {
                    0 => commands.push(ModificationCommand::Idc0 {
                        abs_diff_pic_num_minus1: reader.read_ue()?,
                    }),
                    1 => commands.push(ModificationCommand::Idc1 {
                        abs_diff_pic_num_minus1: reader.read_ue()?,
                    }),
                    2 => commands.push(ModificationCommand::Idc2 {
                        long_term_pic_num: reader.read_ue()?,
                    }),
                    3 => break,
                    _ => return Err(Error::InvalidModificationOfPicNumsIdc(idc)),
                }
            }
        }

        Ok(RefPicListModificationList { flag, commands })
    }

    pub fn parse(reader: &mut BitstreamReader, slice_type: &SliceType) -> Result<Self, Error> {
        let l0 = if slice_type != &SliceType::I && slice_type != &SliceType::SI {
            Some(Self::parse_commands(reader)?)
        } else {
            None
        };

        let l1 = if slice_type == &SliceType::B {
            Some(Self::parse_commands(reader)?)
        } else {
            None
        };

        Ok(Self { l0, l1 })
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
        slice_type: &SliceType,
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
    pub l0: Option<Vec<WeightEntry>>,          // num_ref_idx_l0_active_minus1 + 1 entries
    pub l1: Option<Vec<WeightEntry>>,          // slice_type % 5 == 1
}

impl PredWeightTable {
    fn write_entries(writer: &mut BitstreamWriter, entries: &Option<Vec<WeightEntry>>) {
        if let Some(entries) = entries {
            for entry in entries {
                writer.write_bool(entry.luma_weight_flag);
                if let (Some(w), Some(o)) = (entry.luma_weight, entry.luma_offset) {
                    writer.write_se(w);
                    writer.write_se(o);
                }
                if let Some(chroma_weight_flag) = entry.chroma_weight_flag {
                    writer.write_bool(chroma_weight_flag);
                    if let Some(chroma) = &entry.chroma_weight {
                        for (w, o) in chroma {
                            writer.write_se(*w);
                            writer.write_se(*o);
                        }
                    }
                }
            }
        }
    }

    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_ue(self.luma_log2_weight_denom);
        if let Some(chroma_log2_weight_denom) = self.chroma_log2_weight_denom {
            writer.write_ue(chroma_log2_weight_denom);
        }
        Self::write_entries(writer, &self.l0);
        Self::write_entries(writer, &self.l1);
    }

    pub fn parse(
        reader: &mut BitstreamReader,
        chroma_array_type: u8,
        num_ref_idx_l0: u32,
        num_ref_idx_l1: u32,
    ) -> Result<Self, Error> {
        let luma_log2_weight_denom = reader.read_ue()?;
        let chroma_log2_weight_denom = if chroma_array_type != 0 {
            Some(reader.read_ue()?)
        } else {
            None
        };

        let l0 = Some(Self::parse_entries(
            reader,
            chroma_array_type,
            num_ref_idx_l0,
        )?);
        let l1 = if num_ref_idx_l1 > 0 {
            Some(Self::parse_entries(
                reader,
                chroma_array_type,
                num_ref_idx_l1,
            )?)
        } else {
            None
        };

        Ok(Self {
            luma_log2_weight_denom,
            chroma_log2_weight_denom,
            l0,
            l1,
        })
    }

    fn parse_entries(
        reader: &mut BitstreamReader,
        chroma_array_type: u8,
        num_ref_idx: u32,
    ) -> Result<Vec<WeightEntry>, Error> {
        let mut entries = Vec::new();

        for _ in 0..=num_ref_idx {
            let luma_weight_flag = reader.read_bit()?;
            let (luma_weight, luma_offset) = if luma_weight_flag {
                (Some(reader.read_se()?), Some(reader.read_se()?))
            } else {
                (None, None)
            };

            let (chroma_weight_flag, chroma_weight) = if chroma_array_type != 0 {
                let flag = reader.read_bit()?;
                if flag {
                    let cb_weight = reader.read_se()?;
                    let cb_offset = reader.read_se()?;
                    let cr_weight = reader.read_se()?;
                    let cr_offset = reader.read_se()?;
                    (
                        Some(flag),
                        Some([(cb_weight, cb_offset), (cr_weight, cr_offset)]),
                    )
                } else {
                    (Some(flag), None)
                }
            } else {
                (None, None)
            };

            entries.push(WeightEntry {
                luma_weight_flag,
                luma_weight,
                luma_offset,
                chroma_weight_flag,
                chroma_weight,
            });
        }

        Ok(entries)
    }
}

#[derive(Debug)]
pub enum MemoryManagementControlOp {
    Mmco1 {
        difference_of_pic_nums_minus1: u32,
    },
    Mmco2 {
        long_term_pic_num: u32,
    },
    Mmco3 {
        difference_of_pic_nums_minus1: u32,
        long_term_frame_idx: u32,
    },
    Mmco4 {
        max_long_term_frame_idx_plus1: u32,
    },
    Mmco5,
    Mmco6 {
        long_term_frame_idx: u32,
    },
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

impl DecRefPicMarking {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        match self {
            Self::Idr {
                no_output_of_prior_pics_flag,
                long_term_reference_flag,
            } => {
                writer.write_bool(*no_output_of_prior_pics_flag);
                writer.write_bool(*long_term_reference_flag);
            }
            Self::NonIdr {
                adaptive_ref_pic_marking_mode_flag,
                commands,
            } => {
                writer.write_bool(*adaptive_ref_pic_marking_mode_flag);
                if let Some(cmds) = commands {
                    for cmd in cmds {
                        match cmd {
                            MemoryManagementControlOp::Mmco1 {
                                difference_of_pic_nums_minus1,
                            } => {
                                writer.write_ue(1); // memory_management_control_operation
                                writer.write_ue(*difference_of_pic_nums_minus1);
                            }
                            MemoryManagementControlOp::Mmco2 { long_term_pic_num } => {
                                writer.write_ue(2);
                                writer.write_ue(*long_term_pic_num);
                            }
                            MemoryManagementControlOp::Mmco3 {
                                difference_of_pic_nums_minus1,
                                long_term_frame_idx,
                            } => {
                                writer.write_ue(3);
                                writer.write_ue(*difference_of_pic_nums_minus1);
                                writer.write_ue(*long_term_frame_idx);
                            }
                            MemoryManagementControlOp::Mmco4 {
                                max_long_term_frame_idx_plus1,
                            } => {
                                writer.write_ue(4);
                                writer.write_ue(*max_long_term_frame_idx_plus1);
                            }
                            MemoryManagementControlOp::Mmco5 => {
                                writer.write_ue(5);
                            }
                            MemoryManagementControlOp::Mmco6 {
                                long_term_frame_idx,
                            } => {
                                writer.write_ue(6);
                                writer.write_ue(*long_term_frame_idx);
                            }
                        }
                    }
                    writer.write_ue(0); // terminator
                }
            }
        }
    }

    pub fn parse(reader: &mut BitstreamReader, is_idr: bool) -> Result<Self, Error> {
        if is_idr {
            let no_output_of_prior_pics_flag = reader.read_bit()?;
            let long_term_reference_flag = reader.read_bit()?;
            Ok(Self::Idr {
                no_output_of_prior_pics_flag,
                long_term_reference_flag,
            })
        } else {
            let adaptive_ref_pic_marking_mode_flag = reader.read_bit()?;
            let commands = if adaptive_ref_pic_marking_mode_flag {
                let mut cmds = Vec::new();
                loop {
                    let mmco = reader.read_ue()?;
                    match mmco {
                        0 => break,
                        1 => cmds.push(MemoryManagementControlOp::Mmco1 {
                            difference_of_pic_nums_minus1: reader.read_ue()?,
                        }),
                        2 => cmds.push(MemoryManagementControlOp::Mmco2 {
                            long_term_pic_num: reader.read_ue()?,
                        }),
                        3 => cmds.push(MemoryManagementControlOp::Mmco3 {
                            difference_of_pic_nums_minus1: reader.read_ue()?,
                            long_term_frame_idx: reader.read_ue()?,
                        }),
                        4 => cmds.push(MemoryManagementControlOp::Mmco4 {
                            max_long_term_frame_idx_plus1: reader.read_ue()?,
                        }),
                        5 => cmds.push(MemoryManagementControlOp::Mmco5),
                        6 => cmds.push(MemoryManagementControlOp::Mmco6 {
                            long_term_frame_idx: reader.read_ue()?,
                        }),
                        _ => return Err(Error::InvalidMemoryManagementControlOp(mmco)),
                    }
                }
                Some(cmds)
            } else {
                None
            };
            Ok(Self::NonIdr {
                adaptive_ref_pic_marking_mode_flag,
                commands,
            })
        }
    }
}

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
    pub fn to_bytes(
        &self,
        writer: &mut BitstreamWriter,
        sps: &Sps,
        pps: &Pps,
    ) -> Result<(), Error> {
        writer.write_ue(self.first_mb_in_slice);
        writer.write_ue(self.slice_type);
        writer.write_ue(self.pic_parameter_set_id);

        if let Some(colour_plane_id) = self.colour_plane_id {
            writer.write_bits(colour_plane_id as u32, 2);
        }

        writer.write_bits(self.frame_num as u32, sps.log2_max_frame_num_minus4 + 4);

        if let Some(field_flags) = &self.field_flags {
            writer.write_bool(field_flags.field_pic_flag);
            if let Some(bottom_field_flag) = field_flags.bottom_field_flag {
                writer.write_bool(bottom_field_flag);
            }
        }

        if let Some(idr_pid_id) = self.idr_pic_id {
            writer.write_ue(idr_pid_id);
        }

        if let Some(poc) = &self.pic_order_cnt
            && let sps::PicOrderCnt::Type0 {
                log2_max_pic_order_cnt_lsb_minus4,
            } = &sps.pic_order_cnt
        {
            writer.write_bits(poc.pic_order_cnt_lsb, log2_max_pic_order_cnt_lsb_minus4 + 4);
            if let Some(delta) = poc.delta_pic_order_cnt_bottom {
                writer.write_se(delta);
            }
        }

        if let Some(deltas) = &self.delta_pic_order_cnt {
            for delta in deltas {
                writer.write_se(*delta);
            }
        }

        if let Some(redundant_pic_cnt) = self.redundant_pic_cnt {
            writer.write_ue(redundant_pic_cnt);
        }

        let st = SliceType::try_from(self.slice_type)?;
        if st == SliceType::B
            && let Some(direct_spatial_mv_pred_flag) = self.direct_spatial_mv_pred_flag
        {
            writer.write_bool(direct_spatial_mv_pred_flag);
        }

        if matches!(st, SliceType::P | SliceType::SP | SliceType::B)
            && let Some(nri) = &self.num_ref_idx
        {
            writer.write_bool(nri.num_ref_idx_active_override_flag);

            if let Some(num_ref_idx_l0_active_minus1) = nri.num_ref_idx_l0_active_minus1 {
                writer.write_ue(num_ref_idx_l0_active_minus1);
            }

            if let Some(num_ref_idx_l1_active_minus1) = nri.num_ref_idx_l1_active_minus1 {
                writer.write_ue(num_ref_idx_l1_active_minus1);
            }
        }

        match &self.ref_pic_list_mod {
            RefPicListMod::MvcModification(mvc) => mvc.to_bytes(writer),
            RefPicListMod::Modification(m) => m.to_bytes(writer),
        }

        if let Some(pwt) = &self.pred_weight_table {
            pwt.to_bytes(writer);
        }

        if let Some(drpm) = &self.dec_ref_pic_marking {
            drpm.to_bytes(writer);
        }

        if let Some(cabac_init_idc) = self.cabac_init_idc {
            writer.write_ue(cabac_init_idc);
        }

        writer.write_se(self.slice_qp_delta);

        if let Some(sp_for_switch_flag) = self.sp_for_switch_flag {
            writer.write_bool(sp_for_switch_flag);
        }

        if let Some(slice_qs_delta) = self.slice_qs_delta {
            writer.write_se(slice_qs_delta);
        }

        if let Some(dfcpf) = &self.deblocking_filter {
            writer.write_ue(dfcpf.disable_deblocking_filter_idc);
            if let (Some(slice_alpha_c0_offset_div2), Some(slice_beta_offset_div2)) = (
                dfcpf.slice_alpha_c0_offset_div2,
                dfcpf.slice_beta_offset_div2,
            ) {
                writer.write_se(slice_alpha_c0_offset_div2);
                writer.write_se(slice_beta_offset_div2);
            }
        }

        if let Some(slice_group_change_cycle) = self.slice_group_change_cycle
            && let Some(SliceGroup::Type3_5 {
                slice_group_change_rate_minus1,
                ..
            }) = &pps.slice_group
        {
            let pic_size_in_map_units =
                (sps.pic_width_in_mbs_minus1 + 1) * (sps.pic_height_in_map_units_minus1 + 1);
            let slice_group_change_rate = slice_group_change_rate_minus1 + 1;
            let bits = ((pic_size_in_map_units as f64 / slice_group_change_rate as f64 + 1.0)
                .log2()
                .ceil()) as u8;
            writer.write_bits(slice_group_change_cycle, bits);
        }

        Ok(())
    }

    pub fn parse(
        reader: &mut BitstreamReader,
        sps: &Sps,
        pps: &Pps,
        nal_unit_type: NalUnitType,
        nal_ref_idc: u8,
    ) -> Result<Self, Error> {
        let is_idr = nal_unit_type == NalUnitType::IDR;

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

        let field_pic_flag = field_flags.as_ref().is_some_and(|f| f.field_pic_flag);

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

        let num_ref_idx = if matches!(st, SliceType::P | SliceType::SP | SliceType::B) {
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

        let ref_pic_list_mod = RefPicListMod::parse(reader, &st, &nal_unit_type)?;

        let pred_weight_table = if (pps.weighted_pred_flag
            && (matches!(st, SliceType::P | SliceType::SP)))
            || (pps.weighted_bipred_idc == 1 && st == SliceType::B)
        {
            let chroma_array_type = sps.high_profile.as_ref().map_or(0, |hp| {
                if hp.separate_colour_plane_flag == Some(true) {
                    0
                } else {
                    hp.chroma_format_idc
                }
            });

            let num_ref_idx_l0 = num_ref_idx
                .as_ref()
                .and_then(|n| n.num_ref_idx_l0_active_minus1)
                .unwrap_or(pps.num_ref_idx_l0_default_active_minus1);

            let num_ref_idx_l1 = num_ref_idx
                .as_ref()
                .and_then(|n| n.num_ref_idx_l1_active_minus1)
                .unwrap_or(pps.num_ref_idx_l1_default_active_minus1);

            Some(PredWeightTable::parse(
                reader,
                chroma_array_type,
                num_ref_idx_l0,
                num_ref_idx_l1,
            )?)
        } else {
            None
        };

        let dec_ref_pic_marking = if nal_ref_idc != 0 {
            Some(DecRefPicMarking::parse(reader, is_idr)?)
        } else {
            None
        };

        let cabac_init_idc =
            if pps.entropy_coding_mode_flag && st != SliceType::I && st != SliceType::SI {
                Some(reader.read_ue()?)
            } else {
                None
            };

        let slice_qp_delta = reader.read_se()?;

        let (sp_for_switch_flag, slice_qs_delta) = if matches!(st, SliceType::SP | SliceType::SI) {
            let sp_flag = if st == SliceType::SP {
                Some(reader.read_bit()?)
            } else {
                None
            };
            let qs_delta = reader.read_se()?;

            (sp_flag, Some(qs_delta))
        } else {
            (None, None)
        };

        let deblocking_filter = if pps.deblocking_filter_control_present_flag {
            let disable_deblocking_filter_idc = reader.read_ue()?;
            let (slice_alpha_c0_offset_div2, slice_beta_offset_div2) =
                if disable_deblocking_filter_idc != 1 {
                    (Some(reader.read_se()?), Some(reader.read_se()?))
                } else {
                    (None, None)
                };
            Some(DeblockingFilter {
                disable_deblocking_filter_idc,
                slice_alpha_c0_offset_div2,
                slice_beta_offset_div2,
            })
        } else {
            None
        };

        let slice_group_change_cycle = if pps.num_slice_groups_minus1 > 0 {
            match &pps.slice_group {
                Some(SliceGroup::Type3_5 {
                    slice_group_change_rate_minus1,
                    ..
                }) => {
                    // PicSizeInMapUnits = PicWidthInMbs * PicHeightInMapUnits
                    //                   = (ic_width_in_mbs_minus1 + 1) * pic_height_in_map_units_minus1 + 1
                    // SliceGroupChangeRate = slice_group_change_rate_minus1 + 1
                    // bit width = Ceil(Log2(PicSizeInMapUnits / SliceGroupChangeRate + 1))
                    let pic_size_in_map_units = (sps.pic_width_in_mbs_minus1 + 1)
                        * (sps.pic_height_in_map_units_minus1 + 1);
                    let slice_group_change_rate = slice_group_change_rate_minus1 + 1;
                    let bits = ((pic_size_in_map_units as f64 / slice_group_change_rate as f64
                        + 1.0)
                        .log2()
                        .ceil()) as u8;
                    Some(reader.read_bits(bits)?)
                }
                _ => None,
            }
        } else {
            None
        };

        Ok(Self {
            first_mb_in_slice,
            slice_type,
            pic_parameter_set_id,
            colour_plane_id,
            frame_num,
            field_flags,
            idr_pic_id,
            pic_order_cnt,
            delta_pic_order_cnt,
            redundant_pic_cnt,
            direct_spatial_mv_pred_flag,
            num_ref_idx,
            ref_pic_list_mod,
            pred_weight_table,
            dec_ref_pic_marking,
            cabac_init_idc,
            slice_qp_delta,
            sp_for_switch_flag,
            slice_qs_delta,
            deblocking_filter,
            slice_group_change_cycle,
        })
    }
}
