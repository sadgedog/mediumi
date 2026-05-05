use crate::{
    boxes::{BaseBox, BoxIter, FullBox, FullBoxHeader, Mp4Box, error::Error, hdlr::Hdlr},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Meta {
    pub header: FullBoxHeader,
    pub hdlr: Hdlr,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Meta {
    const BOX_TYPE: BoxType = BoxType::Meta;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        self.hdlr.write_box(writer);
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let mut hdlr: Option<Hdlr> = None;
        let mut others = Vec::new();
        for item in BoxIter::new(&data[4..]) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Hdlr(h) => {
                    if hdlr.is_some() {
                        return Err(Error::DuplicateBox("hdlr"));
                    }
                    hdlr = Some(h);
                }
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            header,
            hdlr: hdlr.ok_or(Error::MissingRequiredBox("hdlr"))?,
            others,
        })
    }
}

impl FullBox for Meta {
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

    fn minimal_hdlr_bytes() -> Vec<u8> {
        vec![
            0x00, 0x00, 0x00, 0x21, // size = 33
            b'h', b'd', b'l', b'r', // type
            0x00, 0x00, 0x00, 0x00, // version + flags
            0x00, 0x00, 0x00, 0x00, // pre_defined
            b'p', b'i', b'c', b't', // handler_type
            0x00, 0x00, 0x00, 0x00, //
            0x00, 0x00, 0x00, 0x00, // reserved[3]
            0x00, 0x00, 0x00, 0x00, //
            0x00, // name = "" + null
        ]
    }

    #[test]
    fn test_meta_with_hdlr_only_roundtrip() {
        // Meta = FullBoxHeader + hdlr
        let mut data = vec![0x00, 0x00, 0x00, 0x00]; // v + f
        data.extend(minimal_hdlr_bytes());

        let meta = Meta::parse(&data).expect("parse meta");
        assert_eq!(meta.header.version, 0);
        assert_eq!(meta.hdlr.handler_type, u32::from_be_bytes(*b"pict"));
        assert!(meta.others.is_empty());

        let mut w = BitstreamWriter::new();
        meta.to_bytes(&mut w);
        assert_eq!(w.finish(), data);
    }

    #[test]
    fn test_meta_missing_hdlr_errors() {
        // none hdlr -> MissingRequiredBox error
        let data = [
            0x00, 0x00, 0x00, 0x00, // FullBoxHeader
            0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
        ];
        let err = Meta::parse(&data).unwrap_err();
        assert_eq!(err, Error::MissingRequiredBox("hdlr"));
    }
}
