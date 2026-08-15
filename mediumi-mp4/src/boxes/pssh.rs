use crate::{BaseBox, Error, FullBox, boxes::FullBoxHeader, types::BoxType};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug, PartialEq)]
pub struct Pssh {
    pub header: FullBoxHeader,
    pub system_id: [u8; 16],
    pub key_ids: Vec<[u8; 16]>,
    pub data: Vec<u8>,
}

impl BaseBox for Pssh {
    const BOX_TYPE: BoxType = BoxType::Pssh;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        for &b in &self.system_id {
            writer.write_bits(b as u32, 8);
        }
        if self.header.version >= 1 {
            writer.write_bits(self.key_ids.len() as u32, 32);
            for kid in &self.key_ids {
                for &b in kid {
                    writer.write_bits(b as u32, 8);
                }
            }
        }
        writer.write_bits(self.data.len() as u32, 32);
        for &b in &self.data {
            writer.write_bits(b as u32, 8);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let system_id: [u8; 16] = reader
            .read_slice(16)?
            .try_into()
            .map_err(|_| Error::DataTooShort)?;
        let key_ids = if header.version >= 1 {
            let kid_count = reader.read_bits(32)? as usize;
            let mut ids = Vec::with_capacity(kid_count);
            for _ in 0..kid_count {
                let kid: [u8; 16] = reader
                    .read_slice(16)?
                    .try_into()
                    .map_err(|_| Error::DataTooShort)?;
                ids.push(kid);
            }
            ids
        } else {
            Vec::new()
        };

        let data_size = reader.read_bits(32)? as usize;
        let payload = reader.read_slice(data_size)?.to_vec();
        Ok(Self {
            header,
            system_id,
            key_ids,
            data: payload,
        })
    }
}

impl FullBox for Pssh {
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

    const SYSTEM_ID: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F,
    ];

    #[test]
    fn pssh_v0_roundtrip() {
        let pssh = Pssh {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            system_id: SYSTEM_ID,
            key_ids: Vec::new(),
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };

        let mut w = ByteWriter::new();
        pssh.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Pssh::parse(&bytes).expect("failed to parse pssh");
        assert_eq!(parsed, pssh);
    }
}
