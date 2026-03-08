//! PAT parser
//!
//! PAT is the payload when ts pid = 0x0000
//!
//! PAT construction
//! ┌─────────────────────────────────────┐
//! │  table_id (1 byte)                  │ <- Fixed at 0x00
//! │  section_syntax_indicator (1 bit)   │ <- Fixed at 1
//! │  '0' (1 bit)                        │ <- Fixed at 0
//! │  reserved (2 bits)                  │ <- Fixed at 0b11
//! │  section_length (12 bits)           │ <- Byte count after this field (includes crc32)
//! │  transport_stream_id (16 bits)      │ <- Identifier to distinguish this TS from others
//! │  reserved (2 bits)                  │ <- Fixed at 0b11
//! │  version_number (5 bits)            │ <- Increments when PAT content changes (0-31)
//! │  current_next_indicator (1 bit)     │ <- If 1, current PAT is applicable
//! │  section_number (1 byte)            │ <- First section is 0x00, increments after that
//! │  last_section_number (1 byte)       │ <- Last section number
//! │  programs                           │ <- See below
//! │  crc32 (32 bits)                    │ <- CRC32 over entire section
//! └─────────────────────────────────────┘
//!
//! Programs
//! ┌─────────────────────────────────────┐
//! │  program_number (2 bytes)           │ <- 0: network PID, otherwise: PMT PID follows
//! │  reserved (3 bits)                  │ <- Fixed at 0b111
//! ├─────────────────────────────────────┤
//!
//! if program_number == 0
//! │  network_pid (13 bits)              │ <- PID of NIT
//! ├─────────────────────────────────────┤
//!
//! if program_number != 0
//! │  program_map_pid (13 bits)          │ <- PID of PMT for this program
//! └─────────────────────────────────────┘
//!

use crate::psi::crc32;
use crate::psi::error::Error;

#[derive(Debug)]
pub struct PatProgram {
    pub program_number: u16,
    pub pid: u16,
}
#[derive(Debug)]
pub struct Pat {
    pub table_id: u8,
    pub section_syntax_indicator: bool,
    pub section_length: u16,
    pub transport_stream_id: u16,
    pub version_number: u8,
    pub current_next_indicator: bool,
    pub section_number: u8,
    pub last_section_number: u8,
    pub programs: Vec<PatProgram>,
    pub crc32: u32,
}

impl Pat {
    /// Parse PAT
    pub fn parse(payload: &[u8]) -> Result<Self, Error> {
        // 12 bytes = 8(table_id ~ last_section_number) bytes + 4(crc32) bytes
        if payload.len() < 12 {
            return Err(Error::BufferTooShort {
                expected: 12,
                actual: payload.len(),
            });
        }

        let table_id = payload[0];
        if table_id != 0x00 {
            return Err(Error::InvalidTableId {
                expected: 0x00,
                actual: table_id,
            });
        }

        let section_syntax_indicator = (payload[1] & 0b1000_0000) != 0;
        if !section_syntax_indicator {
            return Err(Error::InvalidSectionSyntaxIndicator);
        }

        let section_length = ((payload[1] & 0b0000_1111) as u16) << 8 | payload[2] as u16;
        // section_length must be at least 9: header(5) + crc32(4)
        if section_length < 9 {
            return Err(Error::InvalidSectionLength(section_length));
        }
        // payload must contains entire section
        if payload.len() < 3 + section_length as usize {
            return Err(Error::BufferTooShort {
                expected: 3 + section_length as usize,
                actual: payload.len(),
            });
        }
        // programs area must be a multiple of 4 bytes
        if !(section_length as usize - 9).is_multiple_of(4) {
            return Err(Error::InvalidSectionLength(section_length));
        }

        let transport_stream_id = (payload[3] as u16) << 8 | (payload[4]) as u16;
        let version_number = (payload[5] & 0b0011_1110) >> 1;
        let current_next_indicator = (payload[5] & 0b0000_0001) != 0;
        let section_number = payload[6];
        let last_section_number = payload[7];

        if section_number > last_section_number {
            return Err(Error::InvalidSectionNumber {
                section_number,
                last_section_number,
            });
        }

        // programs length = (section_length - 9(header + crc32)) / 4(single program has 4B)
        let pg_len = (section_length as usize - 9) / 4;
        let mut programs = Vec::with_capacity(pg_len);

        let mut pos = 8;

        for _ in 0..(pg_len) {
            let program_number = ((payload[pos] as u16) << 8) | (payload[pos + 1] as u16);
            let pid = (((payload[pos + 2] & 0b0001_1111) as u16) << 8) | (payload[pos + 3] as u16);
            programs.push(PatProgram {
                program_number,
                pid,
            });
            pos += 4;
        }

        let crc32 = ((payload[pos] as u32) << 24)
            | ((payload[pos + 1] as u32) << 16)
            | ((payload[pos + 2] as u32) << 8)
            | (payload[pos + 3] as u32);

        // Section data including CRC32 (table_id through CRC32)
        let section_data = &payload[0..pos + 4];
        if !crc32::verify(section_data) {
            return Err(Error::Crc32Mismatch {
                expected: crc32::crc32(section_data),
                actual: crc32,
            });
        }

        Ok(Pat {
            table_id,
            section_syntax_indicator,
            section_length,
            transport_stream_id,
            version_number,
            current_next_indicator,
            section_number,
            last_section_number,
            programs,
            crc32,
        })
    }

    /// Reconstruct PAT
    pub fn to_bytes(&self) -> Vec<u8> {
        // header(8) + programs(n * 4) + crc32(4)
        let mut buf = Vec::with_capacity(8 + self.programs.len() * 4 + 4);

        // table_id
        buf.push(self.table_id);

        // section_syntax_indicator(1) | '0'(1) | reserved(2) | section_length(12)
        let section_length = (5 + self.programs.len() * 4 + 4) as u16;
        buf.push(
            (if self.section_syntax_indicator { 0b1000_0000 } else { 0b0000_0000 })
              | 0b0011_0000  // '0' + reserved
              | ((section_length >> 8) as u8 & 0b0000_1111),
        );
        buf.push(section_length as u8);

        // transport_stream_id
        buf.push((self.transport_stream_id >> 8) as u8);
        buf.push(self.transport_stream_id as u8);

        // reserved(2) | version_number(5) | current_next_indicator(1)
        buf.push(
            0b1100_0000
                | ((self.version_number & 0b0001_1111) << 1)
                | if self.current_next_indicator {
                    0b0000_0001
                } else {
                    0b0000_0000
                },
        );

        // section_number, last_section_number
        buf.push(self.section_number);
        buf.push(self.last_section_number);

        // programs
        for pg in &self.programs {
            buf.push((pg.program_number >> 8) as u8);
            buf.push(pg.program_number as u8);
            buf.push(0b1110_0000 | ((pg.pid >> 8) as u8 & 0b0001_1111));
            buf.push(pg.pid as u8);
        }

        // crc32 (recalculate)
        let crc = crc32::crc32(&buf);
        buf.extend_from_slice(&crc.to_be_bytes());

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // table_id=0x00, section_syntax_indicator=1, '0'=0, reserved=11,
    // section_length=0x00D, transport_stream_id=0x0001,
    // reserved=11, version_number=00000, current_next_indicator=1,
    // section_number=0x00, last_section_number=0x00,
    // program_number=0x0001, reserved=111, pid=0x1000,
    // crc32=0x2AB104B2
    const PAT_DATA: [u8; 16] = [
        0x00, // table_id
        0xB0, // section_syntax_indicator(1) | '0'(0) | reserved(11) | section_length_upper(0000)
        0x0D, // section_length_lower
        0x00, 0x01, // transport_stream_id
        0xC1, // reserved(11) | version_number(00000) | current_next_indicator(1)
        0x00, // section_number
        0x00, // last_section_number
        0x00, 0x01, // program_number
        0xF0, 0x00, // reserved(111) | pid(1_0000_0000_0000)
        0x2A, 0xB1, 0x04, 0xB2, // crc32
    ];

    #[test]
    fn test_parse_pat() {
        let pat = Pat::parse(&PAT_DATA).unwrap();

        assert_eq!(pat.table_id, 0x00);
        assert!(pat.section_syntax_indicator);
        assert_eq!(pat.section_length, 0x0D);
        assert_eq!(pat.transport_stream_id, 0x0001);
        assert_eq!(pat.version_number, 0x00);
        assert!(pat.current_next_indicator);
        assert_eq!(pat.section_number, 0x00);
        assert_eq!(pat.last_section_number, 0x00);
        assert_eq!(pat.programs.len(), 1);
        assert_eq!(pat.programs[0].program_number, 0x0001);
        assert_eq!(pat.programs[0].pid, 0x1000);
        assert_eq!(pat.crc32, 0x2AB104B2);
    }

    #[test]
    fn test_invalid_table_id() {
        let mut data = PAT_DATA.to_vec();
        data[0] = 0x01;
        // fix crc32 is not needed since table_id check comes first
        let result = Pat::parse(&data);
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidTableId {
                expected: 0x00,
                actual: 0x01,
            }
        );
    }

    #[test]
    fn test_invalid_section_syntax_indicator() {
        let mut data = PAT_DATA.to_vec();
        data[1] = 0x30; // section_syntax_indicator = 0
        let result = Pat::parse(&data);
        assert_eq!(result.unwrap_err(), Error::InvalidSectionSyntaxIndicator);
    }

    #[test]
    fn test_buffer_too_short() {
        let data = &PAT_DATA[..4];
        let result = Pat::parse(data);
        assert_eq!(
            result.unwrap_err(),
            Error::BufferTooShort {
                expected: 12,
                actual: 4,
            }
        );
    }

    #[test]
    fn test_crc32_mismatch() {
        let mut data = PAT_DATA.to_vec();
        data[15] = 0x00; // corrupt crc32
        let result = Pat::parse(&data);
        assert!(matches!(result.unwrap_err(), Error::Crc32Mismatch { .. }));
    }

    #[test]
    fn test_encode_pat() {
        let pat = Pat::parse(&PAT_DATA).unwrap();

        let reconstructed = pat.to_bytes();
        assert_eq!(reconstructed, PAT_DATA.to_vec());
    }
}
