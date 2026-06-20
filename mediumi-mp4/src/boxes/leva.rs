use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
    util::bytestream::{ByteReader, ByteWriter},
};

#[derive(Debug, Clone)]
pub enum Assignment {
    Type0 {
        grouping_type: [u8; 4],
    },
    Type1 {
        grouping_type: [u8; 4],
        grouping_type_parameter: u32,
    },
    Type2,
    Type3,
    Type4 {
        sub_track_id: u32,
    },
}

impl Assignment {
    fn type_code(&self) -> u8 {
        match self {
            Assignment::Type0 { .. } => 0,
            Assignment::Type1 { .. } => 1,
            Assignment::Type2 => 2,
            Assignment::Type3 => 3,
            Assignment::Type4 { .. } => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LevaEntry {
    pub track_id: u32,
    pub padding_flag: u8,
    pub assignment: Assignment,
}

#[derive(Debug)]
pub struct Leva {
    pub header: FullBoxHeader,
    pub levels: Vec<LevaEntry>,
}

impl BaseBox for Leva {
    const BOX_TYPE: BoxType = BoxType::Leva;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.levels.len() as u32, 8);
        for lv in &self.levels {
            writer.write_bits(lv.track_id, 32);
            writer.write_bits((lv.padding_flag & 1) as u32, 1);
            writer.write_bits(lv.assignment.type_code() as u32, 7);
            match &lv.assignment {
                Assignment::Type0 { grouping_type } => {
                    for &b in grouping_type {
                        writer.write_bits(b as u32, 8);
                    }
                }
                Assignment::Type1 {
                    grouping_type,
                    grouping_type_parameter,
                } => {
                    for &b in grouping_type {
                        writer.write_bits(b as u32, 8);
                    }
                    writer.write_bits(*grouping_type_parameter, 32);
                }
                Assignment::Type2 | Assignment::Type3 => {}
                Assignment::Type4 { sub_track_id } => {
                    writer.write_bits(*sub_track_id, 32);
                }
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let level_count = reader.read_bits(8)?;
        let mut levels = Vec::with_capacity(level_count as usize);
        for _ in 0..level_count {
            let track_id = reader.read_bits(32)?;
            let padding_flag = reader.read_bits(1)? as u8;
            let assignment_type = reader.read_bits(7)? as u8;
            let assignment = match assignment_type {
                0 => Assignment::Type0 {
                    grouping_type: [
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                    ],
                },
                1 => Assignment::Type1 {
                    grouping_type: [
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                        reader.read_bits(8)? as u8,
                    ],
                    grouping_type_parameter: reader.read_bits(32)?,
                },
                2 => Assignment::Type2,
                3 => Assignment::Type3,
                4 => Assignment::Type4 {
                    sub_track_id: reader.read_bits(32)?,
                },
                _ => {
                    return Err(Error::UnsupportedValue {
                        field: "leva.assignment_type",
                        value: assignment_type as u32,
                    });
                }
            };
            levels.push(LevaEntry {
                track_id,
                padding_flag,
                assignment,
            });
        }
        Ok(Self { header, levels })
    }
}

impl FullBox for Leva {
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
    fn test_leva_type0_roundtrip() {
        let src = Leva {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            levels: vec![LevaEntry {
                track_id: 1,
                padding_flag: 0,
                assignment: Assignment::Type0 {
                    grouping_type: *b"seig",
                },
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Leva::parse(&bytes).expect("parse leva");
        match &parsed.levels[0].assignment {
            Assignment::Type0 { grouping_type } => {
                assert_eq!(grouping_type, b"seig");
            }
            _ => panic!("expected GroupingType"),
        }
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_leva_type1_roundtrip() {
        let src = Leva {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            levels: vec![LevaEntry {
                track_id: 2,
                padding_flag: 1,
                assignment: Assignment::Type1 {
                    grouping_type: *b"roll",
                    grouping_type_parameter: 0xDEADBEEF,
                },
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Leva::parse(&bytes).expect("parse leva");
        match &parsed.levels[0].assignment {
            Assignment::Type1 {
                grouping_type,
                grouping_type_parameter,
            } => {
                assert_eq!(grouping_type, b"roll");
                assert_eq!(*grouping_type_parameter, 0xDEADBEEF);
            }
            _ => panic!("expected GroupingTypeParameter"),
        }
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_leva_type2_and_type3_roundtrip() {
        let src = Leva {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            levels: vec![
                LevaEntry {
                    track_id: 1,
                    padding_flag: 0,
                    assignment: Assignment::Type2,
                },
                LevaEntry {
                    track_id: 2,
                    padding_flag: 0,
                    assignment: Assignment::Type3,
                },
            ],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Leva::parse(&bytes).expect("parse leva");
        assert!(matches!(parsed.levels[0].assignment, Assignment::Type2));
        assert!(matches!(parsed.levels[1].assignment, Assignment::Type3));
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }

    #[test]
    fn test_leva_type4_roundtrip() {
        let src = Leva {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            levels: vec![LevaEntry {
                track_id: 5,
                padding_flag: 0,
                assignment: Assignment::Type4 {
                    sub_track_id: 0x1234_5678,
                },
            }],
        };
        let mut w = ByteWriter::new();
        src.to_bytes(&mut w);
        let bytes = w.finish();
        let parsed = Leva::parse(&bytes).expect("parse leva");
        match &parsed.levels[0].assignment {
            Assignment::Type4 { sub_track_id } => {
                assert_eq!(*sub_track_id, 0x1234_5678);
            }
            _ => panic!("expected SubTrack"),
        }
        let mut w2 = ByteWriter::new();
        parsed.to_bytes(&mut w2);
        assert_eq!(w2.finish(), bytes);
    }
}
