use crate::{
    BaseBox, Error, FullBox,
    boxes::FullBoxHeader,
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

/// `senc` flag: each entry carries explicit subsample information.
pub const SENC_FLAG_USE_SUBSAMPLES: u32 = 0x000002;

/// Per-subsample clear/protected byte split
#[derive(Debug, Clone, PartialEq)]
pub struct SubsampleEntry {
    pub bytes_of_clear_data: u16,
    pub bytes_of_protected_data: u32,
}

/// One sample's encryption auxiliary info: the IV plus optional subsamples.
#[derive(Debug, Clone, PartialEq)]
pub struct SencEntry {
    /// Per-sample IV. Length is `per_sample_iv_size` (0 / 8 / 16).
    pub iv: Vec<u8>,
    /// Present iff `senc.header.flags & SENC_FLAG_USE_SUBSAMPLES`.
    pub subsamples: Option<Vec<SubsampleEntry>>,
}

/// Sample Encryption Box (`senc`)
/// Carries per-sample IVs and (optionally) subsample partitions inside a `traf`.
/// The per-sample IV size is **not stored in the box** — spec-wise it comes from
/// `tenc.default_per_sample_iv_size`. When that context isn't available,
/// [`Senc::parse`] infers the IV size from the box length:
///   - no subsamples: `iv_size = (body_len - 8) / sample_count` (exact)
///   - subsamples: tries 16 / 8 / 0 and picks the one that consumes the body exactly
///
/// When the size is known (e.g. from `tenc`), prefer [`Senc::parse_with_iv_size`].
#[derive(Debug, Clone, PartialEq)]
pub struct Senc {
    pub header: FullBoxHeader,
    pub entries: Vec<SencEntry>,
}

impl Senc {
    /// Parse with a caller-supplied per-sample IV size (the spec-correct source
    /// is `tenc.default_per_sample_iv_size`).
    pub fn parse_with_iv_size(data: &[u8], per_sample_iv_size: u8) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sample_count = reader.read_bits(32)? as usize;
        let use_subsamples = header.flags & SENC_FLAG_USE_SUBSAMPLES != 0;

        let mut entries = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let iv = reader.read_slice(per_sample_iv_size as usize)?.to_vec();
            let subsamples = if use_subsamples {
                let count = reader.read_bits(16)? as usize;
                let mut subs = Vec::with_capacity(count);
                for _ in 0..count {
                    let clear = reader.read_bits(16)? as u16;
                    let protected = reader.read_bits(32)?;
                    subs.push(SubsampleEntry {
                        bytes_of_clear_data: clear,
                        bytes_of_protected_data: protected,
                    });
                }
                Some(subs)
            } else {
                None
            };
            entries.push(SencEntry { iv, subsamples });
        }

        Ok(Self { header, entries })
    }

    /// Infer the per-sample IV size from the box length when no `tenc` context
    /// is available. See the struct docs for the strategy.
    fn infer_iv_size(data: &[u8]) -> Result<u8, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let sample_count = reader.read_bits(32)? as usize;
        let use_subsamples = header.flags & SENC_FLAG_USE_SUBSAMPLES != 0;
        // body = FullBoxHeader(4) + sample_count(4) + entries
        let entries_len = data.len().checked_sub(8).ok_or(Error::DataTooShort)?;

        if sample_count == 0 {
            return Ok(0);
        }

        if !use_subsamples {
            // entries_len == sample_count * iv_size
            if entries_len % sample_count != 0 {
                return Err(Error::UnsupportedValue {
                    field: "senc.per_sample_iv_size",
                    value: entries_len as u32,
                });
            }
            let iv = entries_len / sample_count;
            if !matches!(iv, 0 | 8 | 16) {
                return Err(Error::UnsupportedValue {
                    field: "senc.per_sample_iv_size",
                    value: iv as u32,
                });
            }
            return Ok(iv as u8);
        }

        // With subsamples the length is under-determined; try the valid IV sizes
        // and pick the one that consumes the body exactly.
        for &candidate in &[16u8, 8, 0] {
            if fits_with_subsamples(data, sample_count, candidate as usize) {
                return Ok(candidate);
            }
        }
        Err(Error::UnsupportedValue {
            field: "senc.per_sample_iv_size",
            value: entries_len as u32,
        })
    }
}

/// Walk `sample_count` subsample entries assuming `iv_size`, returning true iff
/// the walk lands exactly at the end of `data`.
fn fits_with_subsamples(data: &[u8], sample_count: usize, iv_size: usize) -> bool {
    let mut pos = 8usize; // skip FullBoxHeader(4) + sample_count(4)
    for _ in 0..sample_count {
        // [IV(0|8|16)] [subsample_count(2B)] [subsample * n]
        //                                     └→[clear(2B)+protected(4B) * n]
        pos += iv_size;
        if pos + 2 > data.len() {
            return false;
        }
        let n = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2 + n * 6;
        if pos > data.len() {
            return false;
        }
    }
    pos == data.len()
}

impl BaseBox for Senc {
    const BOX_TYPE: BoxType = BoxType::Senc;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        let use_subsamples = self.header.flags & SENC_FLAG_USE_SUBSAMPLES != 0;
        for entry in &self.entries {
            for &b in &entry.iv {
                writer.write_bits(b as u32, 8);
            }
            if use_subsamples {
                let subs = entry.subsamples.as_deref().unwrap_or(&[]);
                writer.write_bits(subs.len() as u32, 16);
                for s in subs {
                    writer.write_bits(s.bytes_of_clear_data as u32, 16);
                    writer.write_bits(s.bytes_of_protected_data, 32);
                }
            }
        }
    }

    /// Parse, inferring the per-sample IV size from the box length (no `tenc`
    /// context). Use [`Senc::parse_with_iv_size`] when the size is known.
    fn parse(data: &[u8]) -> Result<Self, Error> {
        let iv_size = Self::infer_iv_size(data)?;
        Self::parse_with_iv_size(data, iv_size)
    }
}

impl FullBox for Senc {
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

    fn fbh(flags: u32) -> FullBoxHeader {
        FullBoxHeader { version: 0, flags }
    }

    #[test]
    fn senc_no_subsamples_iv8_infers_and_roundtrips() {
        let senc = Senc {
            header: fbh(0),
            entries: vec![
                SencEntry {
                    iv: vec![0, 0, 0, 0, 0, 0, 0, 1],
                    subsamples: None,
                },
                SencEntry {
                    iv: vec![0, 0, 0, 0, 0, 0, 0, 2],
                    subsamples: None,
                },
            ],
        };
        let mut w = ByteWriter::new();
        senc.to_bytes(&mut w);
        let bytes = w.finish();

        // parse() infers iv_size = (body-8)/sample_count = exact
        let parsed = Senc::parse(&bytes).expect("parse (infer)");
        assert_eq!(parsed, senc);
        // explicit path matches too
        assert_eq!(Senc::parse_with_iv_size(&bytes, 8).unwrap(), senc);
    }

    #[test]
    fn senc_no_subsamples_iv16_infers() {
        let senc = Senc {
            header: fbh(0),
            entries: vec![SencEntry {
                iv: (0..16).collect(),
                subsamples: None,
            }],
        };
        let mut w = ByteWriter::new();
        senc.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Senc::parse(&bytes).expect("parse (infer iv16)");
        assert_eq!(parsed, senc);
    }

    #[test]
    fn senc_with_subsamples_iv8_infers_and_roundtrips() {
        let senc = Senc {
            header: fbh(SENC_FLAG_USE_SUBSAMPLES),
            entries: vec![
                SencEntry {
                    iv: vec![1, 2, 3, 4, 5, 6, 7, 8],
                    subsamples: Some(vec![
                        SubsampleEntry {
                            bytes_of_clear_data: 5,
                            bytes_of_protected_data: 100,
                        },
                        SubsampleEntry {
                            bytes_of_clear_data: 13,
                            bytes_of_protected_data: 0,
                        },
                    ]),
                },
                SencEntry {
                    iv: vec![8, 7, 6, 5, 4, 3, 2, 1],
                    subsamples: Some(vec![SubsampleEntry {
                        bytes_of_clear_data: 3,
                        bytes_of_protected_data: 40,
                    }]),
                },
            ],
        };
        let mut w = ByteWriter::new();
        senc.to_bytes(&mut w);
        let bytes = w.finish();

        // parse() tries 16/8/0 and validates exact consumption → 8
        let parsed = Senc::parse(&bytes).expect("parse (infer subsamples)");
        assert_eq!(parsed, senc);
        assert_eq!(Senc::parse_with_iv_size(&bytes, 8).unwrap(), senc);
    }

    #[test]
    fn senc_with_subsamples_iv16_infers_and_roundtrips() {
        let senc = Senc {
            header: fbh(SENC_FLAG_USE_SUBSAMPLES),
            entries: vec![
                SencEntry {
                    iv: (0..16).collect(),
                    subsamples: Some(vec![
                        SubsampleEntry {
                            bytes_of_clear_data: 5,
                            bytes_of_protected_data: 100,
                        },
                        SubsampleEntry {
                            bytes_of_clear_data: 13,
                            bytes_of_protected_data: 0,
                        },
                    ]),
                },
                SencEntry {
                    iv: (16..32).collect(),
                    subsamples: Some(vec![SubsampleEntry {
                        bytes_of_clear_data: 3,
                        bytes_of_protected_data: 40,
                    }]),
                },
            ],
        };
        let mut w = ByteWriter::new();
        senc.to_bytes(&mut w);
        let bytes = w.finish();

        // candidates are tried 16 -> 8 -> 0, so iv16 is matched first and fits.
        let parsed = Senc::parse(&bytes).expect("parse (infer subsamples iv16)");
        assert_eq!(parsed, senc);
        assert_eq!(Senc::parse_with_iv_size(&bytes, 16).unwrap(), senc);
    }

    #[test]
    fn senc_empty_sample_count() {
        let senc = Senc {
            header: fbh(SENC_FLAG_USE_SUBSAMPLES),
            entries: Vec::new(),
        };
        let mut w = ByteWriter::new();
        senc.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Senc::parse(&bytes).expect("parse empty");
        assert_eq!(parsed, senc);
    }
}
