use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Saio {
    pub header: FullBoxHeader,
    pub aux_info_type: Option<u32>,
    pub aux_info_type_parameter: Option<u32>,
    pub entry_count: u32,
    pub offset: Vec<u64>,
}

impl BaseBox for Saio {
    const BOX_TYPE: BoxType = BoxType::Saio;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);

        if self.header.flags & 1 != 0 {
            if let Some(v) = self.aux_info_type {
                writer.write_bits(v, 32);
            }
            if let Some(v) = self.aux_info_type_parameter {
                writer.write_bits(v, 32);
            }
        }

        writer.write_bits(self.entry_count, 32);

        if self.header.version == 0 {
            for &off in &self.offset {
                writer.write_bits(off as u32, 32);
            }
        } else {
            for &off in &self.offset {
                writer.write_bits((off >> 32) as u32, 32);
                writer.write_bits(off as u32, 32);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let (aux_info_type, aux_info_type_parameter) = if header.flags & 1 != 0 {
            (Some(reader.read_bits(32)?), Some(reader.read_bits(32)?))
        } else {
            (None, None)
        };

        let entry_count = reader.read_bits(32)?;

        let offset = if header.version == 0 {
            let mut offsets = Vec::with_capacity(entry_count as usize);
            for _ in 0..entry_count {
                offsets.push(reader.read_bits(32)? as u64);
            }
            offsets
        } else {
            let mut offsets = Vec::with_capacity(entry_count as usize);
            for _ in 0..entry_count {
                let high = reader.read_bits(32)? as u64;
                let low = reader.read_bits(32)? as u64;
                offsets.push((high << 32) | low);
            }
            offsets
        };

        Ok(Self {
            header,
            aux_info_type,
            aux_info_type_parameter,
            entry_count,
            offset,
        })
    }
}

impl FullBox for Saio {
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
    fn test_saio_v0_roundtrip() {
        let data = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x03, // entry_count = 3
            0x00, 0x00, 0x01, 0x00, // offset[0] = 256
            0x00, 0x00, 0x02, 0x00, // offset[1] = 512
            0x00, 0x00, 0x03, 0x00, // offset[2] = 768
        ];
        let saio = Saio::parse(&data).expect("parse saio v0");
        assert_eq!(saio.header.version, 0);
        assert_eq!(saio.header.flags, 0);
        assert_eq!(saio.aux_info_type, None);
        assert_eq!(saio.aux_info_type_parameter, None);
        assert_eq!(saio.entry_count, 3);
        assert_eq!(saio.offset, [256u64, 512, 768]);

        let mut w = BitstreamWriter::new();
        saio.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_saio_v1_roundtrip() {
        let data = [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x02, // entry_count = 2
            // offset[0] = 0x0000_0001_0000_0000 (4 GiB)
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, //
            // offset[1] = 0x0000_0002_DEAD_BEEF
            0x00, 0x00, 0x00, 0x02, 0xDE, 0xAD, 0xBE, 0xEF,
        ];
        let saio = Saio::parse(&data).expect("parse saio v1");
        assert_eq!(saio.header.version, 1);
        assert_eq!(saio.entry_count, 2);
        assert_eq!(
            saio.offset,
            [0x0000_0001_0000_0000u64, 0x0000_0002_DEAD_BEEFu64]
        );

        let mut w = BitstreamWriter::new();
        saio.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }
}
