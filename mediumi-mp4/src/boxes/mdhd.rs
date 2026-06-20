use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Mdhd {
    pub header: FullBoxHeader,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub language: u16,
}

impl BaseBox for Mdhd {
    const BOX_TYPE: BoxType = BoxType::Mdhd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);

        if self.header.version == 1 {
            writer.write_bits((self.creation_time >> 32) as u32, 32);
            writer.write_bits(self.creation_time as u32, 32);
            writer.write_bits((self.modification_time >> 32) as u32, 32);
            writer.write_bits(self.modification_time as u32, 32);
            writer.write_bits(self.timescale, 32);
            writer.write_bits((self.duration >> 32) as u32, 32);
            writer.write_bits(self.duration as u32, 32);
        } else {
            writer.write_bits(self.creation_time as u32, 32);
            writer.write_bits(self.modification_time as u32, 32);
            writer.write_bits(self.timescale, 32);
            writer.write_bits(self.duration as u32, 32);
        }

        writer.write_bits(0, 1); // pad = 0
        writer.write_bits(self.language as u32, 15);
        writer.write_bits(0, 16); // pre_defined = 0
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let (creation_time, modification_time, timescale, duration) = if header.version == 1 {
            let ct_high = (reader.read_bits(32)? as u64) << 32;
            let ct_low = reader.read_bits(32)? as u64;
            let mt_high = (reader.read_bits(32)? as u64) << 32;
            let mt_low = reader.read_bits(32)? as u64;
            let ts = reader.read_bits(32)?;
            let dt_high = (reader.read_bits(32)? as u64) << 32;
            let dt_low = reader.read_bits(32)? as u64;
            (ct_high | ct_low, mt_high | mt_low, ts, dt_high | dt_low)
        } else {
            let ct = reader.read_bits(32)? as u64;
            let mt = reader.read_bits(32)? as u64;
            let ts = reader.read_bits(32)?;
            let dt = reader.read_bits(32)? as u64;
            (ct, mt, ts, dt)
        };

        let _ = reader.read_bits(1)?; // pad
        let language = reader.read_bits(15)? as u16;
        let _ = reader.read_bits(16)?; // pre_defined

        Ok(Self {
            header,
            creation_time,
            modification_time,
            timescale,
            duration,
            language,
        })
    }
}

impl FullBox for Mdhd {
    fn version(&self) -> u8 {
        self.header.version
    }
    fn flags(&self) -> u32 {
        self.header.flags
    }
}

/// Encode 3-letter ISO-639-2/T language code as packed 15-bit value (3 * 5 bits).
/// Each letter is `byte - 0x60` (so 'a' -> 1, 'z' -> 26).
pub fn pack_language(code: &[u8; 3]) -> u16 {
    let a = (code[0].saturating_sub(0x60) & 0x1F) as u16;
    let b = (code[1].saturating_sub(0x60) & 0x1F) as u16;
    let c = (code[2].saturating_sub(0x60) & 0x1F) as u16;
    (a << 10) | (b << 5) | c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdhd_v0_roundtrip() {
        let src = Mdhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            creation_time: 0,
            modification_time: 0,
            timescale: 48_000,
            duration: 192_000,
            language: pack_language(b"eng"),
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 24);
        let parsed = Mdhd::parse(&bytes).expect("parse mdhd v0");
        assert_eq!(parsed.timescale, 48_000);
        assert_eq!(parsed.language, pack_language(b"eng"));
    }
}
