use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Saiz {
    pub header: FullBoxHeader,
    pub aux_info_type: Option<u32>,
    pub aux_info_type_parameter: Option<u32>,
    pub default_sample_info_size: u8,
    pub sample_count: u32,
    pub sample_info_sizes: Vec<u8>,
}

impl BaseBox for Saiz {
    const BOX_TYPE: BoxType = BoxType::Saiz;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);

        if self.header.flags & 0x01 != 0 {
            if let Some(v) = self.aux_info_type {
                writer.write_bits(v, 32);
            }
            if let Some(v) = self.aux_info_type_parameter {
                writer.write_bits(v, 32);
            }
        }

        writer.write_bits(self.default_sample_info_size as u32, 8);
        writer.write_bits(self.sample_count, 32);

        if self.default_sample_info_size == 0 {
            for &size in &self.sample_info_sizes {
                writer.write_bits(size as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let (aux_info_type, aux_info_type_parameter) = if header.flags & 0x01 != 0 {
            (Some(reader.read_bits(32)?), Some(reader.read_bits(32)?))
        } else {
            (None, None)
        };

        let default_sample_info_size = reader.read_bits(8)? as u8;
        let sample_count = reader.read_bits(32)?;

        let sample_info_sizes = if default_sample_info_size == 0 {
            let mut sizes = Vec::with_capacity(sample_count as usize);
            for _ in 0..sample_count {
                sizes.push(reader.read_bits(8)? as u8);
            }
            sizes
        } else {
            Vec::new()
        };

        Ok(Self {
            header,
            aux_info_type,
            aux_info_type_parameter,
            default_sample_info_size,
            sample_count,
            sample_info_sizes,
        })
    }
}

impl FullBox for Saiz {
    fn version(&self) -> u8 {
        self.header.version
    }

    fn flags(&self) -> u32 {
        self.header.flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saiz_fixed_size_roundtrip() {
        // flags=0 (aux_info_type absent), default_sample_info_size=16,
        // sample_count=3
        let data = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            0x10, // default_sample_info_size = 16
            0x00, 0x00, 0x00, 0x03, // sample_count = 3
        ];
        let saiz = Saiz::parse(&data).expect("parse fixed-size saiz");
        assert_eq!(saiz.header.flags, 0);
        assert_eq!(saiz.aux_info_type, None);
        assert_eq!(saiz.aux_info_type_parameter, None);
        assert_eq!(saiz.default_sample_info_size, 16);
        assert_eq!(saiz.sample_count, 3);
        assert!(saiz.sample_info_sizes.is_empty());

        let mut w = BitstreamWriter::new();
        saiz.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }
}
