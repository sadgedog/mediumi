//! PMT Parser
//!
//! PMT is the payload when ts pid = pid of pat
//!
//! PMT construction
//! ┌─────────────────────────────────────┐
//! │  table_id (1 byte)                  │ <- Fixed at 0x02
//! │  section_syntax_indicator (1 bit)   │ <- Fixed at 1
//! │  '0' (1 bit)                        │ <- Fixed at 0
//! │  reserved (2 bits)                  │ <- Fixed at 0b11
//! │  section_length (12 bits)           │ <- Byte count after this field (includes crc32)
//! │  program_number (16 bits)           │ <- program_id
//! │  reserved (2 bits)                  │ <- Fixed at 0b11
//! │  version_number (5 bits)            │ <- Increments when PMT content changes (0-31)
//! │  current_next_indicator (1 bit)     │ <- If 1, current PMT is applicable
//! │  section_number (1 byte)            │ <- Fixed at 0x00
//! │  last_section_number (1 byte)       │ <- Fixed at 0x00
//! │  reserved (3 bits)                  │ <- Fixed at 0b111
//! │  PCR_PID (13 bits)                  │ <- PID of TS packets containing PCR for this program
//! │  reserved (4 bits)                  │ <- Fixed at 0b1111
//! │  program_info_length (12 bits)      │ <- Length of program descriptor
//! │  descriptors (variable)             │ <- See below (repeating)
//! │  streams (variable)                 │ <- See below
//! │  crc32 (32 bits)                    │ <- CRC32 over entire section
//! └─────────────────────────────────────┘
//!
//! Descriptor
//! ┌─────────────────────────────────────┐
//! │  descriptor_tag (1 byte)            │ <- Indentifier of descriptor type
//! │  descriptor_length (1 byte)         │ <- Length of descriptor
//! │  descriptor_data (N * 1 byte)       │ <- Descriptor data
//! └─────────────────────────────────────┘
//!
//! Stream
//! ┌─────────────────────────────────────┐
//! │  stream_type (1 byte)               │ <- Codec type(H.264, AAC, ...)
//! │  reserved (3 bit)                   │ <- Fixed at 0b111
//! │  elementary_pid (13 bits)           │ <- PID of TS packets carrying this ES
//! │  reserved (4 bits)                  │ <- Fixed at 0b1111
//! │  es_info_length (12 bits)           │ <- Length of descriptor
//! │  descriptor (12 bits)               │ <- descriptor (same as above descriptor)
//! └─────────────────────────────────────┘
//!

use crate::psi::{crc32, error::Error};

#[derive(Debug)]
pub struct Descriptor {
    pub tag: u8,
    pub length: u8,
    pub data: Vec<u8>,
}

impl Descriptor {
    /// Reconstruct descriptor
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(2 + self.data.len());
        buf.push(self.tag);
        buf.push(self.data.len() as u8);
        buf.extend_from_slice(&self.data);
        buf
    }

    /// Parse descriptor
    fn parse(data: &[u8]) -> Vec<Self> {
        let mut descriptors = Vec::new();
        let mut pos = 0;

        while pos + 2 <= data.len() {
            let tag = data[pos];
            let length = data[pos + 1];
            pos += 2;

            if pos + length as usize > data.len() {
                break;
            }

            let desc_data = data[pos..pos + length as usize].to_vec();
            pos += length as usize;

            descriptors.push(Descriptor {
                tag,
                length,
                data: desc_data,
            });
        }

        descriptors
    }
}

// Based on ISO/IEC 13818-1 (2nd edition). And added some typical stream types (H.264, H.265).
#[derive(Debug, PartialEq)]
pub enum StreamType {
    Mpeg1Video,           // 0x01
    H262,                 // 0x02
    Mpeg1Audio,           // 0x03
    Mpeg2Audio,           // 0x04
    Mpeg2PrivateSections, // 0x05
    Mpeg2PrivateData,     // 0x06
    Mheg,                 // 0x07
    DsmCc,                // 0x08
    H222_1,               // 0x09
    DsmCcTypeA,           // 0x0A
    DsmCcTypeB,           // 0x0B
    DsmCcTypeC,           // 0x0C
    DsmCcTypeD,           // 0x0D
    H222_0,               // 0x0E
    AdtsAac,              // 0x0F
    Mpeg4Video,           // 0x10
    LatmAac,              // 0x11
    Mpeg4SlPes,           // 0x12
    Mpeg4SlSection,       // 0x13
    SynchronizedDownload, // 0x14
    H264,                 // 0x1B
    H265,                 // 0x24
    Reserved(u8),         // Otherwise
    UserPrivate(u8),      // 0x80-0xFF
}

impl From<u8> for StreamType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => StreamType::Mpeg1Video,
            0x02 => StreamType::H262,
            0x03 => StreamType::Mpeg1Audio,
            0x04 => StreamType::Mpeg2Audio,
            0x05 => StreamType::Mpeg2PrivateSections,
            0x06 => StreamType::Mpeg2PrivateData,
            0x07 => StreamType::Mheg,
            0x08 => StreamType::DsmCc,
            0x09 => StreamType::H222_1,
            0x0A => StreamType::DsmCcTypeA,
            0x0B => StreamType::DsmCcTypeB,
            0x0C => StreamType::DsmCcTypeC,
            0x0D => StreamType::DsmCcTypeD,
            0x0E => StreamType::H222_0,
            0x0F => StreamType::AdtsAac,
            0x10 => StreamType::Mpeg4Video,
            0x11 => StreamType::LatmAac,
            0x12 => StreamType::Mpeg4SlPes,
            0x13 => StreamType::Mpeg4SlSection,
            0x14 => StreamType::SynchronizedDownload,
            0x1B => StreamType::H264,
            0x24 => StreamType::H265,
            v @ 0x80..=0xFF => StreamType::UserPrivate(v),
            v => StreamType::Reserved(v),
        }
    }
}

impl From<&StreamType> for u8 {
    fn from(st: &StreamType) -> u8 {
        match st {
            StreamType::Mpeg1Video => 0x01,
            StreamType::H262 => 0x02,
            StreamType::Mpeg1Audio => 0x03,
            StreamType::Mpeg2Audio => 0x04,
            StreamType::Mpeg2PrivateSections => 0x05,
            StreamType::Mpeg2PrivateData => 0x06,
            StreamType::Mheg => 0x07,
            StreamType::DsmCc => 0x08,
            StreamType::H222_1 => 0x09,
            StreamType::DsmCcTypeA => 0x0A,
            StreamType::DsmCcTypeB => 0x0B,
            StreamType::DsmCcTypeC => 0x0C,
            StreamType::DsmCcTypeD => 0x0D,
            StreamType::H222_0 => 0x0E,
            StreamType::AdtsAac => 0x0F,
            StreamType::Mpeg4Video => 0x10,
            StreamType::LatmAac => 0x11,
            StreamType::Mpeg4SlPes => 0x12,
            StreamType::Mpeg4SlSection => 0x13,
            StreamType::SynchronizedDownload => 0x14,
            StreamType::H264 => 0x1B,
            StreamType::H265 => 0x24,
            StreamType::Reserved(v) | StreamType::UserPrivate(v) => *v,
        }
    }
}

#[derive(Debug)]
pub struct PmtStream {
    pub stream_type: StreamType,
    pub elementary_pid: u16,
    pub es_info_length: u16,
    pub descriptors: Vec<Descriptor>,
}

impl PmtStream {
    /// Reconstruct PMT stream
    pub fn to_bytes(&self) -> Vec<u8> {
        let desc_bytes: Vec<u8> = self.descriptors.iter().flat_map(|d| d.to_bytes()).collect();
        let es_info_length = desc_bytes.len() as u16;
        let mut buf = Vec::with_capacity(5 + desc_bytes.len());
        buf.push(u8::from(&self.stream_type));
        buf.push(0b1110_0000 | ((self.elementary_pid >> 8) as u8 & 0b0001_1111));
        buf.push(self.elementary_pid as u8);
        buf.push(0b1111_0000 | ((es_info_length >> 8) as u8 & 0b0000_1111));
        buf.push(es_info_length as u8);
        buf.extend_from_slice(&desc_bytes);
        buf
    }

    /// Parse PMT stream
    fn parse(data: &[u8]) -> Vec<Self> {
        let mut streams = Vec::new();
        let mut pos = 0;

        while pos + 5 <= data.len() {
            let stream_type = StreamType::from(data[pos]);
            let elementary_pid = ((data[pos + 1] & 0b0001_1111) as u16) << 8 | data[pos + 2] as u16;
            let es_info_length = ((data[pos + 3] & 0b0000_1111) as u16) << 8 | data[pos + 4] as u16;
            pos += 5;

            let descriptors = Descriptor::parse(&data[pos..pos + es_info_length as usize]);
            pos += es_info_length as usize;

            streams.push(PmtStream {
                stream_type,
                elementary_pid,
                es_info_length,
                descriptors,
            });
        }

        streams
    }
}

#[derive(Debug)]
pub struct Pmt {
    pub table_id: u8,
    pub section_syntax_indicator: bool,
    pub section_length: u16,
    pub program_number: u16,
    pub version_number: u8,
    pub current_next_indicator: bool,
    pub section_number: u8,
    pub last_section_number: u8,
    pub pcr_pid: u16,
    pub program_info_length: u16,
    pub program_descriptors: Vec<Descriptor>,
    pub streams: Vec<PmtStream>,
    pub crc32: u32,
}

impl Pmt {
    /// Parse PMT
    pub fn parse(payload: &[u8]) -> Result<Self, Error> {
        // 16 bytes = 12(table_id ~ program_info_length) bytes + 4(crc32) bytes
        if payload.len() < 16 {
            return Err(Error::BufferTooShort {
                expected: 16,
                actual: payload.len(),
            });
        }

        let table_id = payload[0];
        if table_id != 0x02 {
            return Err(Error::InvalidTableId {
                expected: 0x02,
                actual: table_id,
            });
        }

        let section_syntax_indicator = (payload[1] & 0b1000_0000) != 0;
        if !section_syntax_indicator {
            return Err(Error::InvalidSectionSyntaxIndicator);
        }

        let section_length = ((payload[1] & 0b0000_1111) as u16) << 8 | payload[2] as u16;
        // section_length must be at least 13: header(9) + crc32(4)
        if section_length < 13 {
            return Err(Error::InvalidSectionLength(section_length));
        }
        // payload must contains entire section
        if payload.len() < 3 + section_length as usize {
            return Err(Error::BufferTooShort {
                expected: 3 + section_length as usize,
                actual: payload.len(),
            });
        }

        let program_number = ((payload[3] as u16) << 8) | (payload[4] as u16);
        let version_number = (payload[5] & 0b0011_1110) >> 1;
        let current_next_indicator = (payload[5] & 0b0000_0001) != 0;
        let section_number = payload[6];
        let last_section_number = payload[7];
        let pcr_pid = ((payload[8] & 0b0001_1111) as u16) << 8 | payload[9] as u16;
        let program_info_length = ((payload[10] & 0b0000_1111) as u16) << 8 | payload[11] as u16;

        let descriptor_end = 12 + program_info_length as usize;
        // table_id ~ section_length(3) + section_length - crc32(4)
        let stream_end = 3 + section_length as usize - 4;
        if descriptor_end > stream_end {
            return Err(Error::InvalidSectionLength(section_length));
        }

        let program_descriptors = Descriptor::parse(&payload[12..descriptor_end]);
        let streams = PmtStream::parse(&payload[descriptor_end..stream_end]);

        let pos = stream_end;
        let crc32 = ((payload[pos] as u32) << 24)
            | ((payload[pos + 1] as u32) << 16)
            | ((payload[pos + 2] as u32) << 8)
            | (payload[pos + 3] as u32);

        let section_data = &payload[0..pos + 4];
        if !crc32::verify(section_data) {
            return Err(Error::Crc32Mismatch {
                expected: crc32::crc32(section_data),
                actual: crc32,
            });
        }

        Ok(Pmt {
            table_id,
            section_syntax_indicator,
            section_length,
            program_number,
            version_number,
            current_next_indicator,
            section_number,
            last_section_number,
            pcr_pid,
            program_info_length,
            program_descriptors,
            streams,
            crc32,
        })
    }

    /// Reconstruct PMT
    pub fn to_bytes(&self) -> Vec<u8> {
        let prog_desc_bytes: Vec<u8> = self
            .program_descriptors
            .iter()
            .flat_map(|d| d.to_bytes())
            .collect();
        let stream_bytes: Vec<u8> = self.streams.iter().flat_map(|s| s.to_bytes()).collect();

        // section_length = 9(header after section_length) + prog_desc + streams + 4(crc32)
        let section_length = (9 + prog_desc_bytes.len() + stream_bytes.len() + 4) as u16;
        let program_info_length = prog_desc_bytes.len() as u16;

        let mut buf = Vec::with_capacity(3 + section_length as usize);

        // table_id
        buf.push(self.table_id);

        // section_syntax_indicator(1) | '0'(1) | reserved(2) | section_length(12)
        buf.push(
            (if self.section_syntax_indicator {
                0b1000_0000
            } else {
                0b0000_0000
            }) | 0b0011_0000
                | ((section_length >> 8) as u8 & 0b0000_1111),
        );
        buf.push(section_length as u8);

        // program_number
        buf.push((self.program_number >> 8) as u8);
        buf.push(self.program_number as u8);

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

        // reserved(3) | PCR_PID(13)
        buf.push(0b1110_0000 | ((self.pcr_pid >> 8) as u8 & 0b0001_1111));
        buf.push(self.pcr_pid as u8);

        // reserved(4) | program_info_length(12)
        buf.push(0b1111_0000 | ((program_info_length >> 8) as u8 & 0b0000_1111));
        buf.push(program_info_length as u8);

        // program descriptors
        buf.extend_from_slice(&prog_desc_bytes);

        // streams
        buf.extend_from_slice(&stream_bytes);

        // crc32
        let crc = crc32::crc32(&buf);
        buf.extend_from_slice(&crc.to_be_bytes());

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PMT with 2 streams: H.264 (PID=0x0100) + AAC (PID=0x0101)
    // program_number=0x0001, PCR_PID=0x0100
    // program descriptor: tag=0x01, data=[0x01, 0x02, 0x03, 0x04]
    // stream 1 descriptor: tag=0x28, data=[0x01, 0x02]
    fn build_pmt_data() -> Vec<u8> {
        #[rustfmt::skip]
        let mut data = vec![
            0x02,       // table_id
            0xB0, 0x21, // section_syntax_indicator(1) | '0'(0) | reserved(11) | section_length(33)
            0x00, 0x01, // program_number
            0xC1,       // reserved(11) | version_number(00000) | current_next_indicator(1)
            0x00,       // section_number
            0x00,       // last_section_number
            0xE1, 0x00, // reserved(111) | PCR_PID(0x0100)
            0xF0, 0x06, // reserved(1111) | program_info_length(6)
            // Program descriptor: Registration descriptor
            0x01,                   // descriptor_tag
            0x04,                   // descriptor_length
            0x01, 0x02, 0x03, 0x04,
            // Stream 1: H.264, PID=0x0100
            0x1B,       // stream_type (H.264)
            0xE1, 0x00, // reserved(111) | elementary_pid(0x0100)
            0xF0, 0x04, // reserved(1111) | es_info_length(4)
            0x01,       // descriptor_tag
            0x02,       // descriptor_length
            0x01, 0x02, // descriptor_data
            // Stream 2: AAC, PID=0x0101
            0x0F,       // stream_type (ADTS AAC)
            0xE1, 0x01, // reserved(111) | elementary_pid(0x0101)
            0xF0, 0x00, // reserved(1111) | es_info_length(0)
        ];
        let checksum = crc32::crc32(&data);
        data.extend_from_slice(&checksum.to_be_bytes());
        data
    }

    #[test]
    fn test_parse_pmt() {
        let data = build_pmt_data();
        let pmt = Pmt::parse(&data).unwrap();

        assert_eq!(pmt.table_id, 0x02);
        assert!(pmt.section_syntax_indicator);
        assert_eq!(pmt.section_length, 33);
        assert_eq!(pmt.program_number, 0x0001);
        assert_eq!(pmt.version_number, 0);
        assert!(pmt.current_next_indicator);
        assert_eq!(pmt.section_number, 0);
        assert_eq!(pmt.last_section_number, 0);
        assert_eq!(pmt.pcr_pid, 0x0100);
        assert_eq!(pmt.program_info_length, 6);

        // Program descriptor: tag=0x01, data=[0x01, 0x02, 0x03, 0x04]
        assert_eq!(pmt.program_descriptors.len(), 1);
        assert_eq!(pmt.program_descriptors[0].tag, 0x01);
        assert_eq!(pmt.program_descriptors[0].length, 0x04);
        assert_eq!(
            pmt.program_descriptors[0].data,
            vec![0x01, 0x02, 0x03, 0x04]
        );

        assert_eq!(pmt.streams.len(), 2);
        // Stream 1: H.264 with descriptor
        assert_eq!(pmt.streams[0].stream_type, StreamType::H264);
        assert_eq!(pmt.streams[0].elementary_pid, 0x0100);
        assert_eq!(pmt.streams[0].es_info_length, 4);
        assert_eq!(pmt.streams[0].descriptors.len(), 1);
        assert_eq!(pmt.streams[0].descriptors[0].tag, 0x01);
        assert_eq!(pmt.streams[0].descriptors[0].length, 0x02);
        assert_eq!(pmt.streams[0].descriptors[0].data, vec![0x01, 0x02]);
        // Stream 2: AAC without descriptor
        assert_eq!(pmt.streams[1].stream_type, StreamType::AdtsAac);
        assert_eq!(pmt.streams[1].elementary_pid, 0x0101);
        assert_eq!(pmt.streams[1].es_info_length, 0);
        assert_eq!(pmt.streams[1].descriptors.len(), 0);
    }

    #[test]
    fn test_invalid_table_id() {
        let mut data = build_pmt_data();
        data[0] = 0x00;
        let result = Pmt::parse(&data);
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidTableId {
                expected: 0x02,
                actual: 0x00,
            }
        );
    }

    #[test]
    fn test_invalid_section_syntax_indicator() {
        let mut data = build_pmt_data();
        data[1] = 0x30; // section_syntax_indicator = 0
        let result = Pmt::parse(&data);
        assert_eq!(result.unwrap_err(), Error::InvalidSectionSyntaxIndicator);
    }

    #[test]
    fn test_buffer_too_short() {
        let data = build_pmt_data();
        let result = Pmt::parse(&data[..8]);
        assert_eq!(
            result.unwrap_err(),
            Error::BufferTooShort {
                expected: 16,
                actual: 8,
            }
        );
    }

    #[test]
    fn test_crc32_mismatch() {
        let mut data = build_pmt_data();
        let last = data.len() - 1;
        data[last] ^= 0xFF; // corrupt CRC32
        let result = Pmt::parse(&data);
        assert!(matches!(result.unwrap_err(), Error::Crc32Mismatch { .. }));
    }

    #[test]
    fn test_encode_pmt() {
        let data = build_pmt_data();
        let pmt = Pmt::parse(&data).unwrap();

        let reconstructed = pmt.to_bytes();
        assert_eq!(reconstructed, data);
    }
}
