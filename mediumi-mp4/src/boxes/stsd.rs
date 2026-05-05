use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct Stsd {
    pub header: FullBoxHeader,
    pub entries: Vec<Vec<u8>>,
}

impl BaseBox for Stsd {
    const BOX_TYPE: BoxType = BoxType::Stsd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            for &b in e {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let (rest, _) = reader.read_remaining_bytes();

        let mut entries: Vec<Vec<u8>> = Vec::with_capacity(entry_count as usize);
        let mut offset = 0usize;
        for _ in 0..entry_count {
            if rest.len() < offset + 8 {
                return Err(Error::DataTooShort);
            }
            let size = u32::from_be_bytes([
                rest[offset],
                rest[offset + 1],
                rest[offset + 2],
                rest[offset + 3],
            ]) as usize;
            let take = match size {
                0 => rest.len() - offset,
                1 => {
                    if rest.len() < offset + 16 {
                        return Err(Error::DataTooShort);
                    }
                    let high = u32::from_be_bytes([
                        rest[offset + 8],
                        rest[offset + 9],
                        rest[offset + 10],
                        rest[offset + 11],
                    ]) as u64;
                    let low = u32::from_be_bytes([
                        rest[offset + 12],
                        rest[offset + 13],
                        rest[offset + 14],
                        rest[offset + 15],
                    ]) as u64;
                    ((high << 32) | low) as usize
                }
                _ => size,
            };
            if rest.len() < offset + take {
                return Err(Error::DataTooShort);
            }
            entries.push(rest[offset..offset + take].to_vec());
            offset += take;
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Stsd {
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
    fn test_stsd_roundtrip() {
        let entry = vec![
            0x00, 0x00, 0x00, 0x10, b'a', b'v', b'c', b'1', // header
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let src = Stsd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![entry.clone()],
        };
        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Stsd::parse(&bytes).expect("parse stsd");
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0], entry);
    }
}
