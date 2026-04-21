//! ISO-BMFF box definitions.
pub mod error;
pub mod ftyp;
pub mod mdat;
use crate::boxes::{ftyp::Ftyp, mdat::Mdat};
use crate::types::BoxType;
use crate::util::bitstream::{BitstreamReader, BitstreamWriter};
pub use error::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum BoxSize {
    Normal(u32),
    Large(u64),
    ExtendsToEnd,
}

#[derive(Debug, Clone)]
pub struct BoxHeader {
    pub box_size: BoxSize,
    pub box_type: BoxType,
    pub header_size: usize,
}

impl BoxHeader {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        match self.box_size {
            BoxSize::Normal(s) => {
                writer.write_bits(s, 32);
            }
            BoxSize::Large(_) => {
                writer.write_bits(1, 32);
            }
            BoxSize::ExtendsToEnd => {
                writer.write_bits(0, 32);
            }
        }

        let type_bytes: [u8; 4] = (&self.box_type).into();
        for b in &type_bytes {
            writer.write_bits(*b as u32, 8);
        }

        if let BoxSize::Large(s) = self.box_size {
            writer.write_bits((s >> 32) as u32, 32); // upper 32bits of largesize
            writer.write_bits(s as u32, 32); // lower 32bits of largesize
        }
    }

    pub fn parse(reader: &mut BitstreamReader) -> Result<Self, Error> {
        let size = reader.read_bits(32)?;
        let box_type = BoxType::from([
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
        ]);

        let (box_size, header_size) = match size {
            0 => (BoxSize::ExtendsToEnd, 8),
            1 => {
                // largesize: 64-bit size follows
                let high = reader.read_bits(32)? as u64;
                let low = reader.read_bits(32)? as u64;
                (BoxSize::Large((high << 32) | low), 16)
            }
            _ => (BoxSize::Normal(size as u32), 8),
        };

        Ok(Self {
            box_size,
            box_type,
            header_size,
        })
    }
}

#[derive(Debug)]
pub struct UnknownBox {
    pub header: BoxHeader,
    pub payload: Vec<u8>,
}

/// Top-level box variants.
#[derive(Debug)]
pub enum Mp4Box {
    Ftyp(Ftyp),
    Mdat(Mdat),
    Unknown(UnknownBox),
}

impl Mp4Box {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8]) -> Result<(Self, usize), Error> {
        let mut reader = BitstreamReader::new(data);
        let header = BoxHeader::parse(&mut reader)?;

        let total: usize = match header.box_size {
            BoxSize::Normal(s) => s as usize,
            BoxSize::Large(s) => s as usize,
            BoxSize::ExtendsToEnd => data.len(),
        };
        if data.len() < total {
            return Err(Error::DataTooShort);
        }
        let payload = &data[header.header_size..total];

        let parsed = match &header.box_type {
            BoxType::Ftyp => {
                let mut payload_reader = BitstreamReader::new(payload);
                Mp4Box::Ftyp(Ftyp::parse(payload, &mut payload_reader)?)
            }
            BoxType::Mdat => Mp4Box::Mdat(Mdat::parse(payload)?),
            _ => Mp4Box::Unknown(UnknownBox {
                header: header.clone(),
                payload: payload.to_vec(),
            }),
        };

        Ok((parsed, total))
    }
}

pub fn parse_all(data: &[u8]) -> Result<Vec<Mp4Box>, Error> {
    let mut boxes = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (b, consumed) = Mp4Box::parse(&data[offset..])?;
        offset += consumed;
        boxes.push(b);
    }
    Ok(boxes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bitstream::BitstreamWriter;

    fn roundtrip_header(input: &[u8]) {
        let mut reader = BitstreamReader::new(input);
        let header = BoxHeader::parse(&mut reader).expect("failed to parse header");

        let mut writer = BitstreamWriter::new();
        header.to_bytes(&mut writer);
        let output = writer.finish();

        assert_eq!(output, input);
    }

    #[test]
    fn test_header_normal() {
        // size = 32, type = "ftyp"
        let data = [
            0x00, 0x00, 0x00, 0x20, // size = 32
            b'f', b't', b'y', b'p', // type = "ftyp"
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::Normal(32));
        assert_eq!(header.box_type, BoxType::Ftyp);
        assert_eq!(header.header_size, 8);

        roundtrip_header(&data);
    }

    #[test]
    fn test_header_large() {
        // size = 1 (largesize marker), type = "mdat", largesize = 0x1_0000_0000 (4 GiB)
        let data = [
            0x00, 0x00, 0x00, 0x01, // size = 1 (largesize marker)
            b'm', b'd', b'a', b't', // type = "mdat"
            0x00, 0x00, 0x00, 0x01, // largesize upper 32bit
            0x00, 0x00, 0x00, 0x00, // largesize lower 32bit
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::Large(0x1_0000_0000));
        assert_eq!(header.box_type, BoxType::Mdat);
        assert_eq!(header.header_size, 16);

        roundtrip_header(&data);
    }

    #[test]
    fn test_header_extends_to_end() {
        // size = 0 (extends to end of file), type = "mdat"
        let data = [
            0x00, 0x00, 0x00, 0x00, // size = 0
            b'm', b'd', b'a', b't', // type = "mdat"
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::ExtendsToEnd);
        assert_eq!(header.box_type, BoxType::Mdat);
        assert_eq!(header.header_size, 8);

        roundtrip_header(&data);
    }
}
