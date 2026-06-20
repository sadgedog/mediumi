use crate::{
    BaseBox, Error, FullBox,
    boxes::FullBoxHeader,
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

/// Track Encryption Box (`tenc`)
/// Default per-track encryption parameters. Version 0 is used by the `cenc`
/// scheme; version 1 adds the pattern (`crypt`/`skip`) fields used by `cbcs`.
#[derive(Debug, PartialEq)]
pub struct Tenc {
    pub header: FullBoxHeader,
    /// High nibble of the pattern byte (version 1 only; 0 for version 0).
    pub default_crypt_byte_block: u8,
    /// Low nibble of the pattern byte (version 1 only; 0 for version 0).
    pub default_skip_byte_block: u8,
    pub default_is_protected: u8,
    pub default_per_sample_iv_size: u8,
    pub default_kid: [u8; 16],
    /// Present when `default_is_protected == 1 && default_per_sample_iv_size == 0`
    /// (the `cbcs` constant-IV case).
    pub default_constant_iv: Option<Vec<u8>>,
}

impl BaseBox for Tenc {
    const BOX_TYPE: BoxType = BoxType::Tenc;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        // reserved
        writer.write_bits(0, 8);
        // pattern byte (version 1) or reserved (version 0)
        if self.header.version == 0 {
            writer.write_bits(0, 8);
        } else {
            writer.write_bits((self.default_crypt_byte_block & 0x0F) as u32, 4);
            writer.write_bits((self.default_skip_byte_block & 0x0F) as u32, 4);
        }
        writer.write_bits(self.default_is_protected as u32, 8);
        writer.write_bits(self.default_per_sample_iv_size as u32, 8);
        for &b in &self.default_kid {
            writer.write_bits(b as u32, 8);
        }
        if self.default_is_protected == 1
            && self.default_per_sample_iv_size == 0
            && let Some(iv) = &self.default_constant_iv
        {
            writer.write_bits(iv.len() as u32, 8);
            for &b in iv {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let _reserved = reader.read_bits(8)?;
        let pattern_byte = reader.read_bits(8)? as u8;
        let (default_crypt_byte_block, default_skip_byte_block) = if header.version == 0 {
            (0, 0)
        } else {
            (pattern_byte >> 4, pattern_byte & 0x0F)
        };
        let default_is_protected = reader.read_bits(8)? as u8;
        let default_per_sample_iv_size = reader.read_bits(8)? as u8;
        let default_kid: [u8; 16] = reader
            .read_slice(16)?
            .try_into()
            .map_err(|_| Error::DataTooShort)?;
        let default_constant_iv = if default_is_protected == 1 && default_per_sample_iv_size == 0 {
            let iv_size = reader.read_bits(8)? as usize;
            Some(reader.read_slice(iv_size)?.to_vec())
        } else {
            None
        };

        Ok(Self {
            header,
            default_crypt_byte_block,
            default_skip_byte_block,
            default_is_protected,
            default_per_sample_iv_size,
            default_kid,
            default_constant_iv,
        })
    }
}

impl FullBox for Tenc {
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

    const KID: [u8; 16] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ];

    #[test]
    fn tenc_v0_cenc_roundtrip() {
        let tenc = Tenc {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            default_crypt_byte_block: 0,
            default_skip_byte_block: 0,
            default_is_protected: 1,
            default_per_sample_iv_size: 8,
            default_kid: KID,
            default_constant_iv: None,
        };
        let mut w = ByteWriter::new();
        tenc.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Tenc::parse(&bytes).expect("failed to parse tenc v0");
        assert_eq!(parsed, tenc);
    }

    #[test]
    fn tenc_v1_cbcs_constant_iv_roundtrip() {
        let tenc = Tenc {
            header: FullBoxHeader {
                version: 1,
                flags: 0,
            },
            default_crypt_byte_block: 1,
            default_skip_byte_block: 9,
            default_is_protected: 1,
            default_per_sample_iv_size: 0,
            default_kid: KID,
            default_constant_iv: Some(vec![0xAA; 16]),
        };
        let mut w = ByteWriter::new();
        tenc.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Tenc::parse(&bytes).expect("failed to parse tenc v1");
        assert_eq!(parsed, tenc);
    }
}
