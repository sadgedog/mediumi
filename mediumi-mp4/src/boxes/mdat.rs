use crate::{boxes::Error, util::bitstream::BitstreamWriter};

#[derive(Debug)]
pub struct Mdat {
    pub payload: Vec<u8>,
}

impl Mdat {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        for b in &self.payload {
            writer.write_bits(*b as u32, 8);
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            payload: data.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdat_roundtrip() {
        let data = [0xDE, 0xAD, 0xBE, 0xED];
        let mdat = Mdat::parse(&data).expect("failed to parse mdat");
        assert_eq!(mdat.payload, data);

        let mut writer = BitstreamWriter::new();
        mdat.to_bytes(&mut writer);
        let output = writer.finish();
        assert_eq!(output, data);
    }
}
