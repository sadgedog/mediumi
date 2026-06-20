use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug)]
pub struct Tkhd {
    pub header: FullBoxHeader,
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub layer: i16,
    pub alternate_group: i16,
    pub volume: i16,
    pub matrix: [u32; 9],
    pub width: u32,
    pub height: u32,
}

impl BaseBox for Tkhd {
    const BOX_TYPE: BoxType = BoxType::Tkhd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);

        if self.header.version == 1 {
            writer.write_bits((self.creation_time >> 32) as u32, 32);
            writer.write_bits(self.creation_time as u32, 32);
            writer.write_bits((self.modification_time >> 32) as u32, 32);
            writer.write_bits(self.modification_time as u32, 32);
            writer.write_bits(self.track_id, 32);
            writer.write_bits(0, 32); // reserved
            writer.write_bits((self.duration >> 32) as u32, 32);
            writer.write_bits(self.duration as u32, 32);
        } else {
            writer.write_bits(self.creation_time as u32, 32);
            writer.write_bits(self.modification_time as u32, 32);
            writer.write_bits(self.track_id, 32);
            writer.write_bits(0, 32); // reserved
            writer.write_bits(self.duration as u32, 32);
        }

        writer.write_bits(0, 32); // reserved[2]
        writer.write_bits(0, 32);
        writer.write_bits(self.layer as u16 as u32, 16);
        writer.write_bits(self.alternate_group as u16 as u32, 16);
        writer.write_bits(self.volume as u16 as u32, 16);
        writer.write_bits(0, 16); // reserved
        for &v in &self.matrix {
            writer.write_bits(v, 32);
        }
        writer.write_bits(self.width, 32);
        writer.write_bits(self.height, 32);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let (creation_time, modification_time, track_id, duration) = if header.version == 1 {
            let ct_high = (reader.read_bits(32)? as u64) << 32;
            let ct_low = reader.read_bits(32)? as u64;
            let mt_high = (reader.read_bits(32)? as u64) << 32;
            let mt_low = reader.read_bits(32)? as u64;
            let tid = reader.read_bits(32)?;
            let _ = reader.read_bits(32)?; // reserved
            let dt_high = (reader.read_bits(32)? as u64) << 32;
            let dt_low = reader.read_bits(32)? as u64;
            (ct_high | ct_low, mt_high | mt_low, tid, dt_high | dt_low)
        } else {
            let ct = reader.read_bits(32)? as u64;
            let mt = reader.read_bits(32)? as u64;
            let tid = reader.read_bits(32)?;
            let _ = reader.read_bits(32)?; // reserved
            let dt = reader.read_bits(32)? as u64;
            (ct, mt, tid, dt)
        };

        let _ = reader.read_bits(32)?; // reserved[2]
        let _ = reader.read_bits(32)?;
        let layer = reader.read_bits(16)? as i16;
        let alternate_group = reader.read_bits(16)? as i16;
        let volume = reader.read_bits(16)? as i16; // track_is_audio -> 0x0100, other -> 0
        let _ = reader.read_bits(16)?; // reserved
        let matrix = [
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
            reader.read_bits(32)?,
        ];
        let width = reader.read_bits(32)?;
        let height = reader.read_bits(32)?;

        Ok(Self {
            header,
            creation_time,
            modification_time,
            track_id,
            duration,
            layer,
            alternate_group,
            volume,
            matrix,
            width,
            height,
        })
    }
}

impl FullBox for Tkhd {
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
    use crate::boxes::mvhd::UNITY_MATRIX;

    #[test]
    fn test_tkhd_v0_roundtrip() {
        let src = Tkhd {
            header: FullBoxHeader {
                version: 0,
                flags: 0x7,
            },
            creation_time: 0,
            modification_time: 0,
            track_id: 1,
            duration: 1000,
            layer: 0,
            alternate_group: 0,
            volume: 0x0100,
            matrix: UNITY_MATRIX,
            width: 1920 << 16,
            height: 1080 << 16,
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 84);
        let parsed = Tkhd::parse(&bytes).expect("parse tkhd v0");
        assert_eq!(parsed.track_id, 1);
        assert_eq!(parsed.width, 1920 << 16);

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_tkhd_v1_roundtrip() {
        let src = Tkhd {
            header: FullBoxHeader {
                version: 1,
                flags: 0x3,
            },
            creation_time: 0x1111_1111_1111_1111,
            modification_time: 0,
            track_id: 2,
            duration: 0x0000_0000_DEAD_BEEF,
            layer: 0,
            alternate_group: 1,
            volume: 0,
            matrix: UNITY_MATRIX,
            width: 0,
            height: 0,
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        assert_eq!(bytes.len(), 96);
        let parsed = Tkhd::parse(&bytes).expect("parse tkhd v1");
        assert_eq!(parsed.creation_time, 0x1111_1111_1111_1111);
        assert_eq!(parsed.duration, 0x0000_0000_DEAD_BEEF);
    }
}
