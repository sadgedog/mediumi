use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

pub const SELF_CONTAINED: u32 = 0x000001;

#[derive(Debug)]
pub enum DataEntry {
    Url {
        flags: u32,
        location: Option<String>,
    },
    Urn {
        flags: u32,
        name: String,
        location: Option<String>,
    },
    Unknown {
        box_type: [u8; 4],
        header: FullBoxHeader,
        payload: Vec<u8>,
    },
}

#[derive(Debug)]
pub struct Dref {
    pub header: FullBoxHeader,
    pub entries: Vec<DataEntry>,
}

impl BaseBox for Dref {
    const BOX_TYPE: BoxType = BoxType::Dref;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            match e {
                DataEntry::Url { flags, location } => {
                    let body_len = 4 + location.as_ref().map(|s| s.len() + 1).unwrap_or(0);
                    let total = 8u32 + body_len as u32;
                    writer.write_bits(total, 32);
                    for &b in b"url " {
                        writer.write_bits(b as u32, 8);
                    }
                    writer.write_bits(0, 8); // version = 0
                    writer.write_bits(*flags, 24);
                    if let Some(s) = location {
                        write_cstr(writer, s);
                    }
                }
                DataEntry::Urn {
                    flags,
                    name,
                    location,
                } => {
                    let body_len =
                        4 + name.len() + 1 + location.as_ref().map(|s| s.len() + 1).unwrap_or(0);
                    let total = 8u32 + body_len as u32;
                    writer.write_bits(total, 32);
                    for &b in b"urn " {
                        writer.write_bits(b as u32, 8);
                    }
                    writer.write_bits(0, 8); // version = 0
                    writer.write_bits(*flags, 24);
                    write_cstr(writer, name);
                    if let Some(s) = location {
                        write_cstr(writer, s);
                    }
                }
                DataEntry::Unknown {
                    box_type,
                    header,
                    payload,
                } => {
                    let body_len = 4 + payload.len();
                    let total = 8u32 + body_len as u32;
                    writer.write_bits(total, 32);
                    for &b in box_type {
                        writer.write_bits(b as u32, 8);
                    }
                    header.to_bytes(writer);
                    for &b in payload {
                        writer.write_bits(b as u32, 8);
                    }
                }
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)?;
        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let size = reader.read_bits(32)?;
            let box_type = [
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
            ];
            let body_len = match size {
                0 => reader.remaining_bits() / 8,
                1 => {
                    let high = reader.read_bits(32)? as u64;
                    let low = reader.read_bits(32)? as u64;
                    (((high << 32) | low) as usize).saturating_sub(16)
                }
                _ => (size as usize).saturating_sub(8),
            };
            if body_len < 4 {
                return Err(Error::DataTooShort);
            }
            let entry_header = FullBoxHeader::parse(&mut reader)?;
            let payload_len = body_len - 4;
            let mut payload = Vec::with_capacity(payload_len);
            for _ in 0..payload_len {
                payload.push(reader.read_bits(8)? as u8);
            }

            let entry = match &box_type {
                b"url " => {
                    let location = if payload.is_empty() {
                        None
                    } else {
                        Some(parse_full_cstr(&payload)?)
                    };
                    DataEntry::Url {
                        flags: entry_header.flags,
                        location,
                    }
                }
                b"urn " => {
                    let (name, rest) = split_first_cstr(&payload)?;
                    let location = if rest.is_empty() {
                        None
                    } else {
                        Some(parse_full_cstr(rest)?)
                    };
                    DataEntry::Urn {
                        flags: entry_header.flags,
                        name,
                        location,
                    }
                }
                _ => DataEntry::Unknown {
                    box_type,
                    header: entry_header,
                    payload,
                },
            };
            entries.push(entry);
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Dref {
    fn version(&self) -> u8 {
        self.header.version
    }
    fn flags(&self) -> u32 {
        self.header.flags
    }
}

fn write_cstr(writer: &mut ByteWriter, s: &str) {
    for &b in s.as_bytes() {
        writer.write_bits(b as u32, 8);
    }
    writer.write_bits(0, 8);
}

fn parse_full_cstr(bytes: &[u8]) -> Result<String, Error> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).map_err(|_| Error::InvalidUtf8)
}

fn split_first_cstr(bytes: &[u8]) -> Result<(String, &[u8]), Error> {
    match bytes.iter().position(|&b| b == 0) {
        Some(end) => {
            let s = String::from_utf8(bytes[..end].to_vec()).map_err(|_| Error::InvalidUtf8)?;
            Ok((s, &bytes[end + 1..]))
        }
        None => {
            let s = String::from_utf8(bytes.to_vec()).map_err(|_| Error::InvalidUtf8)?;
            Ok((s, &[]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dref_url_self_contained_roundtrip() {
        let src = Dref {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![DataEntry::Url {
                flags: SELF_CONTAINED,
                location: None,
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        // dref FullBox(4) + entry_count(4) + entry: size(4)+'url '(4)+ver/flags(4) = 20
        assert_eq!(bytes.len(), 20);

        let parsed = Dref::parse(&bytes).expect("parse dref");
        match &parsed.entries[0] {
            DataEntry::Url { flags, location } => {
                assert_eq!(*flags, SELF_CONTAINED);
                assert!(location.is_none());
            }
            _ => panic!("expected url entry"),
        }

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_dref_url_with_location_roundtrip() {
        let src = Dref {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![DataEntry::Url {
                flags: 0,
                location: Some("file.mp4".to_string()),
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();

        let parsed = Dref::parse(&bytes).expect("parse dref");
        match &parsed.entries[0] {
            DataEntry::Url { flags, location } => {
                assert_eq!(*flags, 0);
                assert_eq!(location.as_deref(), Some("file.mp4"));
            }
            _ => panic!("expected url entry"),
        }

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_dref_urn_roundtrip() {
        let src = Dref {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![DataEntry::Urn {
                flags: 0,
                name: "urn:example:123".to_string(),
                location: Some("http://example.com/x".to_string()),
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();

        let parsed = Dref::parse(&bytes).expect("parse dref");
        match &parsed.entries[0] {
            DataEntry::Urn {
                flags,
                name,
                location,
            } => {
                assert_eq!(*flags, 0);
                assert_eq!(name, "urn:example:123");
                assert_eq!(location.as_deref(), Some("http://example.com/x"));
            }
            _ => panic!("expected urn entry"),
        }

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_dref_mixed_roundtrip() {
        let src = Dref {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            entries: vec![
                DataEntry::Url {
                    flags: SELF_CONTAINED,
                    location: None,
                },
                DataEntry::Url {
                    flags: 0,
                    location: Some("ext.mp4".to_string()),
                },
                DataEntry::Urn {
                    flags: 0,
                    name: "urn:foo".to_string(),
                    location: None,
                },
            ],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();

        let parsed = Dref::parse(&bytes).expect("parse dref");
        assert_eq!(parsed.entries.len(), 3);

        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
