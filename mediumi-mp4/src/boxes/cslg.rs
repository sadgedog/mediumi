use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct Cslg {
    pub header: FullBoxHeader,
    pub composition_to_dts_shift: i64,
    pub least_decode_to_display_delta: i64,
    pub greatest_decode_to_display_delta: i64,
    pub composition_start_time: i64,
    pub composition_end_time: i64,
}

impl BaseBox for Cslg {
    const BOX_TYPE: BoxType = BoxType::Cslg;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        let write = |w: &mut ByteWriter, v: i64, version: u8| {
            if version == 1 {
                w.write_bits((v >> 32) as u32, 32);
                w.write_bits(v as u32, 32);
            } else {
                w.write_bits(v as u32, 32);
            }
        };
        write(writer, self.composition_to_dts_shift, self.header.version);
        write(
            writer,
            self.least_decode_to_display_delta,
            self.header.version,
        );
        write(
            writer,
            self.greatest_decode_to_display_delta,
            self.header.version,
        );
        write(writer, self.composition_start_time, self.header.version);
        write(writer, self.composition_end_time, self.header.version);
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let read = |r: &mut ByteReader, version: u8| -> Result<i64, Error> {
            if version == 1 {
                let high = (r.read_bits(32)? as i64) << 32;
                let low = r.read_bits(32)? as i64;
                Ok(high | low)
            } else {
                Ok(r.read_bits(32)? as i32 as i64)
            }
        };
        let composition_to_dts_shift = read(&mut reader, header.version)?;
        let least_decode_to_display_delta = read(&mut reader, header.version)?;
        let greatest_decode_to_display_delta = read(&mut reader, header.version)?;
        let composition_start_time = read(&mut reader, header.version)?;
        let composition_end_time = read(&mut reader, header.version)?;
        Ok(Self {
            header,
            composition_to_dts_shift,
            least_decode_to_display_delta,
            greatest_decode_to_display_delta,
            composition_start_time,
            composition_end_time,
        })
    }
}

impl FullBox for Cslg {
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
    fn test_cslg_v0_roundtrip() {
        let src = Cslg {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            composition_to_dts_shift: 0,
            least_decode_to_display_delta: -100,
            greatest_decode_to_display_delta: 100,
            composition_start_time: 0,
            composition_end_time: 1000,
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Cslg::parse(&bytes).expect("parse cslg");
        assert_eq!(parsed.least_decode_to_display_delta, -100);
    }
}
