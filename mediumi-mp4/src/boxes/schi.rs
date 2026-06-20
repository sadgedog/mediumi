use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, tenc::Tenc},
    types::BoxType,
    util::bytestream::ByteWriter,
};

/// Scheme Information Box (`schi`)
/// Container for scheme-specific information.
#[derive(Debug, PartialEq)]
pub struct Schi {
    pub tenc: Option<Tenc>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Schi {
    const BOX_TYPE: BoxType = BoxType::Schi;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        if let Some(t) = &self.tenc {
            t.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut tenc: Option<Tenc> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Tenc(t) => {
                    if tenc.is_some() {
                        return Err(Error::DuplicateBox("tenc"));
                    }
                    tenc = Some(t);
                }
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self { tenc, others })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::FullBoxHeader;

    #[test]
    fn schi_with_tenc_roundtrip() {
        let schi = Schi {
            tenc: Some(Tenc {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                default_crypt_byte_block: 0,
                default_skip_byte_block: 0,
                default_is_protected: 1,
                default_per_sample_iv_size: 8,
                default_kid: [0xCD; 16],
                default_constant_iv: None,
            }),
            others: Vec::new(),
        };
        let mut w = ByteWriter::new();
        schi.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Schi::parse(&bytes).expect("failed to parse schi");
        assert_eq!(parsed, schi);
        assert!(parsed.tenc.is_some());
    }

    #[test]
    fn empty_schi_roundtrip() {
        let schi = Schi {
            tenc: None,
            others: Vec::new(),
        };
        let mut w = ByteWriter::new();
        schi.to_bytes(&mut w);
        let bytes = w.finish();
        assert!(bytes.is_empty());
        let parsed = Schi::parse(&bytes).expect("failed to parse empty schi");
        assert_eq!(parsed, schi);
    }
}
