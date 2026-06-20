use crate::{
    boxes::{BaseBox, BoxIter, Error, Mp4Box, frma::Frma, schi::Schi, schm::Schm},
    types::BoxType,
    util::bytestream::ByteWriter,
};

/// Protection Scheme Information Box (`sinf`)
/// Inserted as a nested box inside the renamed (`encv` / `enca`) sample entry.
#[derive(Debug, PartialEq)]
pub struct Sinf {
    pub frma: Frma,
    pub schm: Schm,
    pub schi: Option<Schi>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Sinf {
    const BOX_TYPE: BoxType = BoxType::Sinf;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.frma.write_box(writer);
        self.schm.write_box(writer);
        if let Some(schi) = &self.schi {
            schi.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut frma: Option<Frma> = None;
        let mut schm: Option<Schm> = None;
        let mut schi: Option<Schi> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Frma(f) => {
                    if frma.is_some() {
                        return Err(Error::DuplicateBox("frma"));
                    }
                    frma = Some(f);
                }
                Mp4Box::Schm(s) => {
                    if schm.is_some() {
                        return Err(Error::DuplicateBox("schm"));
                    }
                    schm = Some(s);
                }
                Mp4Box::Schi(s) => {
                    if schi.is_some() {
                        return Err(Error::DuplicateBox("schi"));
                    }
                    schi = Some(s);
                }
                _ => others.push(raw.to_vec()),
            }
        }

        Ok(Self {
            frma: frma.ok_or(Error::MissingRequiredBox("frma"))?,
            schm: schm.ok_or(Error::MissingRequiredBox("schm"))?,
            schi,
            others,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{FullBoxHeader, tenc::Tenc};

    fn sample_sinf() -> Sinf {
        Sinf {
            frma: Frma {
                data_format: *b"avc1",
            },
            schm: Schm {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                scheme_type: *b"cenc",
                scheme_version: 0x0001_0000,
                scheme_uri: None,
            },
            schi: Some(Schi {
                tenc: Some(Tenc {
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    default_crypt_byte_block: 0,
                    default_skip_byte_block: 0,
                    default_is_protected: 1,
                    default_per_sample_iv_size: 8,
                    default_kid: [0x42; 16],
                    default_constant_iv: None,
                }),
                others: Vec::new(),
            }),
            others: Vec::new(),
        }
    }

    #[test]
    fn sinf_cenc_roundtrip() {
        let sinf = sample_sinf();
        let mut w = ByteWriter::new();
        sinf.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Sinf::parse(&bytes).expect("failed to parse sinf");
        assert_eq!(parsed, sinf);
    }

    #[test]
    fn sinf_missing_frma_errors() {
        // schm only, no frma
        let schm = Schm {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            scheme_type: *b"cenc",
            scheme_version: 0x0001_0000,
            scheme_uri: None,
        };
        let mut w = ByteWriter::new();
        schm.write_box(&mut w);
        let bytes = w.finish();
        let err = Sinf::parse(&bytes).unwrap_err();
        assert_eq!(err, Error::MissingRequiredBox("frma"));
    }
}
