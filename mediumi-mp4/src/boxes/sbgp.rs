use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::BitstreamReader,
};

#[derive(Debug)]
pub struct Sbgp {
    pub header: FullBoxHeader,
    pub grouping_type: u32,
    pub grouping_type_parameter: Option<u32>,
    pub entry_count: u32,
    pub sample_count: Vec<u32>,
    pub group_description_index: Vec<u32>,
}

impl BaseBox for Sbgp {
    const BOX_TYPE: crate::types::BoxType = BoxType::Sbgp;

    fn to_bytes(&self, writer: &mut crate::util::bitstream::BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.grouping_type, 32);
        if let Some(v) = self.grouping_type_parameter {
            writer.write_bits(v, 32);
        }
        writer.write_bits(self.entry_count, 32);
        for i in 0..(self.entry_count as usize) {
            writer.write_bits(self.sample_count[i], 32);
            writer.write_bits(self.group_description_index[i], 32);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let grouping_type = reader.read_bits(32)?;
        let grouping_type_parameter = if header.version == 1 {
            Some(reader.read_bits(32)?)
        } else {
            None
        };
        let entry_count = reader.read_bits(32)?;
        let mut sample_count = Vec::new();
        let mut group_description_index = Vec::new();
        for _ in 0..entry_count {
            sample_count.push(reader.read_bits(32)?);
            group_description_index.push(reader.read_bits(32)?);
        }

        Ok(Self {
            header,
            grouping_type,
            grouping_type_parameter,
            entry_count,
            sample_count,
            group_description_index,
        })
    }
}

impl FullBox for Sbgp {
    fn version(&self) -> u8 {
        self.header.version
    }

    fn flags(&self) -> u32 {
        self.header.flags
    }
}
