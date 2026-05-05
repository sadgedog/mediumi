use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Hdlr {
    pub header: FullBoxHeader,
    pub pre_defined: u32,
    pub handler_type: u32,
    pub reserved: [u32; 3],
    pub name: String,
}

impl BaseBox for Hdlr {
    const BOX_TYPE: BoxType = BoxType::Hdlr;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.pre_defined, 32);
        writer.write_bits(self.handler_type, 32);
        for &r in &self.reserved {
            writer.write_bits(r, 32);
        }
        for &b in self.name.as_bytes() {
            writer.write_bits(b as u32, 8);
        }
        writer.write_bits(0, 8); // null terminator
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let pre_defined = reader.read_bits(32)?;
        let handler_type = reader.read_bits(32)?;
        let reserved = [
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
        ];

        // read until 0x00 appear (null terminated utf-8 string)
        let mut name_bytes = Vec::new();
        loop {
            let b = reader.read_bits(8)? as u8;
            if b == 0 {
                break;
            }
            name_bytes.push(b);
        }
        let name = String::from_utf8(name_bytes).map_err(|_| Error::InvalidUtf8)?;

        Ok(Self {
            header,
            pre_defined,
            handler_type,
            reserved,
            name,
        })
    }
}

impl FullBox for Hdlr {
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
    fn test_hdlr_with_name_roundtrip() {
        // handler_type='vide', name="VideoHandler"
        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, // version + flags
            0x00, 0x00, 0x00, 0x00, // pre_defined
            b'v', b'i', b'd', b'e', // handler_type = 'vide'
            0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x00, // reserved[3]
            0x00, 0x00, 0x00, 0x00, //
        ];
        data.extend_from_slice(b"VideoHandler\0");

        let hdlr = Hdlr::parse(&data).expect("parse hdlr with name");
        assert_eq!(hdlr.handler_type, u32::from_be_bytes(*b"vide"));
        assert_eq!(hdlr.name, "VideoHandler");

        let mut w = BitstreamWriter::new();
        hdlr.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }
}
