use crate::{
    error::Error,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Alpha {
    pub bit_depth_aux_minus8: u32,
    pub alpha_incr_flag: bool,
    pub alpha_opaque_value: u32,
    pub alpha_transparent_value: u32,
}

#[derive(Debug)]
pub struct SpsExt {
    pub seq_parameter_set_id: u32,
    pub aux_format_idc: u32,
    pub alpha: Option<Alpha>,
    pub additional_extension_flag: bool,
}

impl SpsExt {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        writer.write_ue(self.seq_parameter_set_id);
        writer.write_ue(self.aux_format_idc);
        if let Some(alpha) = &self.alpha {
            writer.write_ue(alpha.bit_depth_aux_minus8);
            writer.write_bool(alpha.alpha_incr_flag);
            writer.write_bits(
                alpha.alpha_opaque_value,
                (alpha.bit_depth_aux_minus8 + 9) as u8,
            );
            writer.write_bits(
                alpha.alpha_transparent_value,
                (alpha.bit_depth_aux_minus8 + 9) as u8,
            );
        }

        writer.write_bool(self.additional_extension_flag);
        writer.write_bits(1, 1);
        writer.finish()
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let seq_parameter_set_id = reader.read_ue()?;
        let aux_format_idc = reader.read_ue()?;
        if aux_format_idc > 3 {
            return Err(Error::InvalidAuxFormatIdc(aux_format_idc));
        }
        let alpha = if aux_format_idc != 0 {
            let bit_depth_aux_minus8 = reader.read_ue()?;
            if bit_depth_aux_minus8 > 8 {
                return Err(Error::InvalidBitDepthAuxMinus8(bit_depth_aux_minus8));
            }
            let alpha_incr_flag = reader.read_bit()?;
            let alpha_opaque_value = reader.read_bits((bit_depth_aux_minus8 + 9) as u8)?;
            let alpha_transparent_value = reader.read_bits((bit_depth_aux_minus8 + 9) as u8)?;
            Some(Alpha {
                bit_depth_aux_minus8,
                alpha_incr_flag,
                alpha_opaque_value,
                alpha_transparent_value,
            })
        } else {
            None
        };

        let additional_extension_flag = reader.read_bit()?;

        Ok(SpsExt {
            seq_parameter_set_id,
            aux_format_idc,
            alpha,
            additional_extension_flag,
        })
    }
}
