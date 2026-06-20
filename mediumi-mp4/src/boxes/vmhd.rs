use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Vmhd {
    pub header: FullBoxHeader,
    pub graphicsmode: u16,
    pub opcolor: [u16; 3],
}

impl BaseBox for Vmhd {
    const BOX_TYPE: BoxType = BoxType::Vmhd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.graphicsmode as u32, 16);
        for &c in &self.opcolor {
            writer.write_bits(c as u32, 16);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let graphicsmode = reader.read_bits(16)? as u16;
        let opcolor = [
            reader.read_bits(16)? as u16,
            reader.read_bits(16)? as u16,
            reader.read_bits(16)? as u16,
        ];
        Ok(Self {
            header,
            graphicsmode,
            opcolor,
        })
    }
}

impl FullBox for Vmhd {
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
    fn test_vmhd_roundtrip() {
        let src = Vmhd {
            header: FullBoxHeader {
                version: 0,
                flags: 1,
            },
            graphicsmode: 0,
            opcolor: [0, 0, 0],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 12); // 4 (FullBox) + 2 + 6
        let parsed = Vmhd::parse(&bytes).expect("parse vmhd");
        assert_eq!(parsed.header.flags, 1);
        assert_eq!(parsed.graphicsmode, 0);
        assert_eq!(parsed.opcolor, [0, 0, 0]);
    }
}
