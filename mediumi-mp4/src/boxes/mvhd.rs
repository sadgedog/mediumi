use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader},
    types::{self, BoxType},
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

pub const UNITY_MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];

#[derive(Debug)]
pub struct Mvhd {
    pub header: FullBoxHeader,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
    pub rate: u32,
    pub volume: u16,
    pub matrix: [u32; 9],
    pub next_track_id: u32,
}

impl BaseBox for Mvhd {
    const BOX_TYPE: types::BoxType = BoxType::Mvhd;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
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

        writer.write_bits(self.rate, 32);
        writer.write_bits(self.volume as u32, 16);
        // const bit(16) reserved = 0
        writer.write_bits(0, 16);
        // const unsigned int(32)[2] reserved = 0
        writer.write_bits(0, 32);
        writer.write_bits(0, 32);
        for &v in &self.matrix {
            writer.write_bits(v, 32);
        }
        // bit(32)[6] pre_defined = 0
        for _ in 0..6 {
            writer.write_bits(0, 32);
        }
        writer.write_bits(self.next_track_id, 32);
    }

    fn parse(data: &[u8]) -> Result<Self, super::error::Error> {
        let mut reader = BitstreamReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;

        let (creation_time, modification_time, timescale, duration) = if header.version == 1 {
            let ct_high = (reader.read_bits(32)? as u64) << 32;
            let ct_low = reader.read_bits(32)? as u64;
            let ct = ct_high | ct_low;
            let mt_high = (reader.read_bits(32)? as u64) << 32;
            let mt_low = reader.read_bits(32)? as u64;
            let mt = mt_high | mt_low;
            let ts = reader.read_bits(32)?;
            let dt_high = (reader.read_bits(32)? as u64) << 32;
            let dt_low = reader.read_bits(32)? as u64;
            let dt = dt_high | dt_low;
            (ct, mt, ts, dt)
        } else {
            let ct = reader.read_bits(32)? as u64;
            let mt = reader.read_bits(32)? as u64;
            let ts = reader.read_bits(32)?;
            let dt = reader.read_bits(32)? as u64;
            (ct, mt, ts, dt)
        };

        let rate = reader.read_bits(32)?;
        let volume = reader.read_bits(16)? as u16;
        // const bit(16) reserved = 0 — read & discard
        let _ = reader.read_bits(16)?;
        // const unsigned int(32)[2] reserved = 0 — read & discard
        let _ = reader.read_bits(32)?;
        let _ = reader.read_bits(32)?;
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
        // bit(32)[6] pre_defined — read & discard
        for _ in 0..6 {
            let _ = reader.read_bits(32)?;
        }
        let next_track_id = reader.read_bits(32)?;

        Ok(Self {
            header,
            creation_time,
            modification_time,
            timescale,
            duration,
            rate,
            volume,
            matrix,
            next_track_id,
        })
    }
}

impl FullBox for Mvhd {
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

    fn sample_mvhd(version: u8) -> Mvhd {
        Mvhd {
            header: FullBoxHeader { version, flags: 0 },
            creation_time: 0x1122_3344_5566_7788,
            modification_time: 0x99AA_BBCC_DDEE_FF00,
            timescale: 1000,
            duration: 0x0000_0000_DEAD_BEEF,
            rate: 0x0001_0000,
            volume: 0x0100,
            matrix: UNITY_MATRIX,
            next_track_id: 3,
        }
    }

    #[test]
    fn test_mvhd_v0_roundtrip() {
        // v0 narrows 64-bit fields to 32-bit on the wire
        let mut src = sample_mvhd(0);
        src.creation_time = 0x5566_7788;
        src.modification_time = 0xDDEE_FF00;
        src.duration = 0xDEAD_BEEF;

        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        // 4 (FullBox) + 16 (times32x4) + 4 (rate) + 2 (vol) + 2 + 8 (reserved) + 36 (matrix) + 24 (pre_defined) + 4 (next_id)
        assert_eq!(bytes.len(), 100);

        let parsed = Mvhd::parse(&bytes).expect("parse mvhd v0");
        assert_eq!(parsed.header.version, 0);
        assert_eq!(parsed.creation_time, src.creation_time);
        assert_eq!(parsed.modification_time, src.modification_time);
        assert_eq!(parsed.timescale, src.timescale);
        assert_eq!(parsed.duration, src.duration);
        assert_eq!(parsed.rate, src.rate);
        assert_eq!(parsed.volume, src.volume);
        assert_eq!(parsed.matrix, UNITY_MATRIX);
        assert_eq!(parsed.next_track_id, src.next_track_id);

        let mut w2 = BitstreamWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_mvhd_v1_roundtrip() {
        // v1 keeps 64-bit creation/modification/duration
        let src = sample_mvhd(1);

        let mut w = BitstreamWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        // 4 + 8+8+4+8 (times) + 4 + 2 + 2 + 8 + 36 + 24 + 4
        assert_eq!(bytes.len(), 112);

        let parsed = Mvhd::parse(&bytes).expect("parse mvhd v1");
        assert_eq!(parsed.header.version, 1);
        assert_eq!(parsed.creation_time, 0x1122_3344_5566_7788);
        assert_eq!(parsed.modification_time, 0x99AA_BBCC_DDEE_FF00);
        assert_eq!(parsed.timescale, 1000);
        assert_eq!(parsed.duration, 0x0000_0000_DEAD_BEEF);
        assert_eq!(parsed.matrix, UNITY_MATRIX);
        assert_eq!(parsed.next_track_id, 3);

        let mut w2 = BitstreamWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
