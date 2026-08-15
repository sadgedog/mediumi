use crate::{BaseBox, Error, FullBox, boxes::FullBoxHeader, types::BoxType};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

/// Scheme Type Box (`schm`)
/// Identifies the protection scheme (e.g. `cenc` / `cbcs`) and its version.
#[derive(Debug, PartialEq)]
pub struct Schm {
    pub header: FullBoxHeader,
    pub scheme_type: [u8; 4],
    pub scheme_version: u32,
    /// Present only when `flags & 0x01` is set. Null-terminated scheme URI bytes.
    pub scheme_uri: Option<Vec<u8>>,
}

impl BaseBox for Schm {
    const BOX_TYPE: BoxType = BoxType::Schm;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        for &b in &self.scheme_type {
            writer.write_bits(b as u32, 8);
        }
        writer.write_bits(self.scheme_version, 32);
        if self.header.flags & 0x01 != 0
            && let Some(uri) = &self.scheme_uri
        {
            for &b in uri {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let scheme_type: [u8; 4] = reader
            .read_slice(4)?
            .try_into()
            .map_err(|_| Error::DataTooShort)?;
        let scheme_version = reader.read_bits(32)?;
        let scheme_uri = if header.flags & 0x01 != 0 {
            let (rest, _) = reader.read_remaining_bytes();
            Some(rest)
        } else {
            None
        };

        Ok(Self {
            header,
            scheme_type,
            scheme_version,
            scheme_uri,
        })
    }
}

impl FullBox for Schm {
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
    fn schm_cenc_roundtrip() {
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
        schm.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Schm::parse(&bytes).expect("failed to parse schm");
        assert_eq!(parsed, schm);
    }

    #[test]
    fn schm_with_uri_roundtrip() {
        let schm = Schm {
            header: FullBoxHeader {
                version: 0,
                flags: 0x01,
            },
            scheme_type: *b"cbcs",
            scheme_version: 0x0001_0000,
            scheme_uri: Some(b"https://example.com\0".to_vec()),
        };
        let mut w = ByteWriter::new();
        schm.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Schm::parse(&bytes).expect("failed to parse schm");
        assert_eq!(parsed, schm);
    }
}
