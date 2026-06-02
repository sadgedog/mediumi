use crate::{
    BaseBox, Error,
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

/// Records the codingname (box_type) of the unprotected sample entry that the
/// enclosing `encv` / `enca` is shadowing (e.g. `b"avc1"` / `b"mp4a"`).
#[derive(Debug, PartialEq)]
pub struct Frma {
    pub data_format: [u8; 4],
}

impl BaseBox for Frma {
    const BOX_TYPE: BoxType = BoxType::Frma;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        for &b in &self.data_format {
            writer.write_bits(b as u32, 8);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let data_format: [u8; 4] = reader
            .read_slice(4)?
            .try_into()
            .map_err(|_| Error::DataTooShort)?;
        Ok(Self { data_format })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frma_roundtrip() {
        let frma = Frma {
            data_format: *b"avc1",
        };
        let mut w = BitstreamWriter::new();
        frma.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Frma::parse(&bytes).expect("failed to parse frma");
        assert_eq!(parsed, frma);
    }
}
