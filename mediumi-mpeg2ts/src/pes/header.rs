//! PES header parser
//!
//! PES header construction
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  packet_start_code_prefix (3 bytes)             │ <- Fixed at 0x00_00_01
//! │  stream_id (1 byte)                             │ <- Indicate the kind of elemental stream
//! │  PES_packet_length (2 bytes)                    │ <- Byte count after this field
//! │  extension (variable)                           │ <- Extension
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! Extension (Standard)
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  '10' (2 bits)                                  │ <- Fixed at 0b10
//! │  PES_scrambling_control (2 bits)                │ <- Indicate that this PES is scrambling or not
//! │  PES_priority (1 bit)                           │ <- If 1, this PES has higher priority
//! │  data_alignment_indicator (1 bit)               │ <- If 1, PES payload starts at a video start code or audio syncword
//! │  copyright (1 bit)                              │ <- If 1, this content is protected by copyright
//! │  original_or_copy (1 bit)                       │ <- Original or copy
//! │  PTS_DTS_flags (2 bit)                          │ <- Indicate this PES header includes PTS, PTS & DTS or not
//! │  ESCR_flag (1 bit)                              │ <- If 1, this PES header includes ESCR field
//! │  ES_rate_flag (1 bit)                           │ <- If 1, this PES header includes ES_rate field
//! │  DSM_trick_mode_flag (1 bit)                    │ <- If 1, this PES header includes DSM_trick_mode field
//! │  additional_copy_info_flag (1 bit)              │ <- If 1, this PES header includes additional copy right information
//! │  PES_CRC_flag (1 bit)                           │ <- If 1, this PES header includes CRC field
//! │  PES_extension_flag (1 bit)                     │ <- If 1, this PES header includes PES extension field
//! │  PES_header_data_length (1 byte)                │ <- PES header length
//! ├─────────────────────────────────────────────────┤
//!
//! if PTS_DTS_flags == '10'
//! │  '0010' (4 bits)                                │ <- Fixed at 0b0010
//! │  PTS[32..30] (2 bits)                           │ <- PTS: Presentation time stamp (33 bits)
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  PTS[29..15] (15 bits)                          │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  PTS[14..0] (15 bits)                           │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! ├─────────────────────────────────────────────────┤
//!
//! if PTS_DTS_flags == '11'
//! │  '0011' (4 bits)                                │ <- Fixed at 0b0011
//! │  PTS[32..30] (2 bits)                           │ <- PTS: Presentation time stamp (33 bits)
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  PTS[29..15] (15 bits)                          │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  PTS[14..0] (15 bits)                           │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  '0010' (4 bits)                                │ <- Fixed at 0b0010
//! │  DTS[32..30] (2 bits)                           │ <- DTS: Decoding time stamp (33 bits)
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  DTS[29..15] (15 bits)                          │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  DTS[14..0] (15 bits)                           │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! ├─────────────────────────────────────────────────┤
//!
//! if ESCR_flag == '1'
//! │  reserved (2 bits)                              │ <- Fixed at 0b11
//! │  ESCR_base[32..30] (2 bits)                     │ <- ESCR: Elementary stream clock reference (6 bytes)
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  ESCR_base[29..15] (15 bits)                    │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  ESCR_base[14..0] (15 bits)                     │
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  ESCR_extension (9 bits)                        │ <- Fixed at 0b0010
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! ├─────────────────────────────────────────────────┤
//! ESCR = ESCR_base * 300 + ESCR_extension
//!
//! if ES_rate_flag == '1'
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  ES_rate (22 bits)                              │ <- Indicate the rate which decoder receive
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! ├─────────────────────────────────────────────────┤
//!
//! if DSM_trick_mode_flag == '1'
//! │  trick_mode_control (3 bits)                    │ <- Identify the trick mode
//! │  trick_mode_data (5 bits)                       │ <- Depends on trick_mode_control
//! ├─────────────────────────────────────────────────┤
//!
//! if additional_copy_info_flag == '1'
//! │  marker_bit (1 bit)                             │ <- Fixed at 1
//! │  additional_copy_info (7 bits)                  │ <- Additional Copy right information
//! ├─────────────────────────────────────────────────┤
//!
//! if PES_CRC_flag == '1'
//! │  previous_PES_packet_CRC (16 bits)              │ <- CRC of the previous PES packet
//! ├─────────────────────────────────────────────────┤
//!
//! if PES_extension_flag == '1'
//! │  PES_private_data_flag (1 bit)                  │ <- If 1, PES extension includes private data
//! │  pack_header_field_flag (1 bit)                 │ <- If 1, PES extension includes pack header
//! │  program_packet_sequence_counter_flag (1 bit)   │ <- If 1, PES extension includes sequence counter
//! │  P-STD_buffer_flag (1 bit)                      │ <- If 1, PES extension includes P-STD buffer info
//! │  reserved (3 bits)                              │ <- Reserved
//! │  PES_extension_flag_2 (1 bit)                   │ <- If 1, PES extension includes extension field 2
//! ├─────────────────────────────────────────────────┤
//!
//!   if PES_private_data_flag == '1'
//!   │  PES_private_data (128 bits)                  │ <- 16 bytes of private data
//!   ├───────────────────────────────────────────────┤
//!
//!   if pack_header_field_flag == '1'
//!   │  pack_field_length (8 bits)                   │ <- Length of pack_header
//!   │  pack_header (variable)                       │ <- Pack header data (typically not used in TS)
//!   ├───────────────────────────────────────────────┤
//!
//!   if program_packet_sequence_counter_flag == '1'
//!   │  marker_bit (1 bit)                           │ <- Fixed at 1
//!   │  program_packet_sequence_counter (7 bits)     │ <- Sequence counter
//!   │  marker_bit (1 bit)                           │ <- Fixed at 1
//!   │  MPEG1_MPEG2_identifier (1 bit)               │ <- If 1, MPEG1; if 0, MPEG2
//!   │  original_stuff_length (6 bits)               │ <- Length of stuffing bytes
//!   ├───────────────────────────────────────────────┤
//!
//!   if P-STD_buffer_flag == '1'
//!   │  '01' (2 bits)                                │ <- Fixed at 0b01
//!   │  P-STD_buffer_scale (1 bit)                   │ <- If 0, unit is 128 bytes; if 1, unit is 1024 bytes
//!   │  P-STD_buffer_size (13 bits)                  │ <- Buffer size bound
//!   ├───────────────────────────────────────────────┤
//!
//!   if PES_extension_flag_2 == '1'
//!   │  marker_bit (1 bit)                           │ <- Fixed at 1
//!   │  PES_extension_field_length (7 bits)          │ <- Length of extension field data
//!   │  reserved (variable)                          │ <- Extension field data
//!   └───────────────────────────────────────────────┘
//!
//! │  stuffing_byte (variable)                       │ <- 0xFF padding to fill PES_header_data_length
//! └─────────────────────────────────────────────────┘
//! ```
//!
//!
//! Extension (DataOnly, Padding)
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  PES_packet_data_byte/padding_byte (variable)   │ <- PES payload of this packet (it doesn't be parsed here)
//! └─────────────────────────────────────────────────┘
//! ```
//!

use crate::pes::error::Error;

fn encode_timestamp(ts: u64, prefix: u8) -> [u8; 5] {
    [
        (prefix << 4) | (((ts >> 30) as u8 & 0b0000_0111) << 1) | 0b0000_0001,
        (ts >> 22) as u8,
        (((ts >> 15) as u8 & 0b0111_1111) << 1) | 0b0000_0001,
        (ts >> 7) as u8,
        ((ts as u8 & 0b0111_1111) << 1) | 0b0000_0001,
    ]
}

#[derive(Debug, PartialEq)]
pub enum StreamId {
    ProgramStreamMap,       // 0xBC
    PrivateStream1,         // 0xBD
    PaddingStream,          // 0xBE
    PrivateStream2,         // 0xBF
    Audio(u8),              // 0xC0 - 0xDF
    Video(u8),              // 0xE0 - 0xEF
    EcmStream,              // 0xF0
    EmmStream,              // 0xF1
    DsmCcStream,            // 0xF2
    Mheg,                   // 0xF3
    H2221TypeA,             // 0xF4
    H2221TypeB,             // 0xF5
    H2221TypeC,             // 0xF6
    H2221TypeD,             // 0xF7
    H2221TypeE,             // 0xF8
    AncillaryStream,        // 0xF9
    SlPacketizedStream,     // 0xFA
    FlexMuxStream,          // 0xFB
    Reserved(u8),           // 0xFC - 0xFE
    ProgramStreamDirectory, // 0xFF
}

impl From<u8> for StreamId {
    fn from(value: u8) -> Self {
        match value {
            0xBC => StreamId::ProgramStreamMap,
            0xBD => StreamId::PrivateStream1,
            0xBE => StreamId::PaddingStream,
            0xBF => StreamId::PrivateStream2,
            v @ 0xC0..=0xDF => StreamId::Audio(v - 0xC0),
            v @ 0xE0..=0xEF => StreamId::Video(v - 0xE0),
            0xF0 => StreamId::EcmStream,
            0xF1 => StreamId::EmmStream,
            0xF2 => StreamId::DsmCcStream,
            0xF3 => StreamId::Mheg,
            0xF4 => StreamId::H2221TypeA,
            0xF5 => StreamId::H2221TypeB,
            0xF6 => StreamId::H2221TypeC,
            0xF7 => StreamId::H2221TypeD,
            0xF8 => StreamId::H2221TypeE,
            0xF9 => StreamId::AncillaryStream,
            0xFA => StreamId::SlPacketizedStream,
            0xFB => StreamId::FlexMuxStream,
            0xFF => StreamId::ProgramStreamDirectory,
            v => StreamId::Reserved(v),
        }
    }
}

impl From<&StreamId> for u8 {
    fn from(id: &StreamId) -> u8 {
        match id {
            StreamId::ProgramStreamMap => 0xBC,
            StreamId::PrivateStream1 => 0xBD,
            StreamId::PaddingStream => 0xBE,
            StreamId::PrivateStream2 => 0xBF,
            StreamId::Audio(n) => 0xC0 + n,
            StreamId::Video(n) => 0xE0 + n,
            StreamId::EcmStream => 0xF0,
            StreamId::EmmStream => 0xF1,
            StreamId::DsmCcStream => 0xF2,
            StreamId::Mheg => 0xF3,
            StreamId::H2221TypeA => 0xF4,
            StreamId::H2221TypeB => 0xF5,
            StreamId::H2221TypeC => 0xF6,
            StreamId::H2221TypeD => 0xF7,
            StreamId::H2221TypeE => 0xF8,
            StreamId::AncillaryStream => 0xF9,
            StreamId::SlPacketizedStream => 0xFA,
            StreamId::FlexMuxStream => 0xFB,
            StreamId::ProgramStreamDirectory => 0xFF,
            StreamId::Reserved(v) => *v,
        }
    }
}

#[derive(Debug)]
pub enum Timestamps {
    Pts(u64),
    PtsDts { pts: u64, dts: u64 },
}

#[derive(Debug)]
pub enum TrickMode {
    FastForward {
        trick_mode_control: u8,
        field_id: u8,
        intra_slice_refresh: bool,
        frequency_truncation: u8,
    },
    SlowMotion {
        trick_mode_control: u8,
        rep_ctrl: u8,
    },
    FreezeFrame {
        trick_mode_control: u8,
        field_id: u8,
    },
    FastReverse {
        trick_mode_control: u8,
        field_id: u8,
        intra_slice_refresh: bool,
        frequency_truncation: u8,
    },
    SlowReverse {
        trick_mode_control: u8,
        rep_ctrl: u8,
    },
    Reserved {
        trick_mode_control: u8,
        reserved: u8,
    },
}

impl From<&TrickMode> for u8 {
    fn from(tm: &TrickMode) -> u8 {
        match tm {
            TrickMode::FastForward {
                trick_mode_control,
                field_id,
                intra_slice_refresh,
                frequency_truncation,
            } => {
                (trick_mode_control << 5)
                    | (field_id << 3)
                    | ((*intra_slice_refresh as u8) << 2)
                    | frequency_truncation
            }
            TrickMode::SlowMotion {
                trick_mode_control,
                rep_ctrl,
            } => (trick_mode_control << 5) | rep_ctrl,
            TrickMode::FreezeFrame {
                trick_mode_control,
                field_id,
            } => (trick_mode_control << 5) | (field_id << 3),
            TrickMode::FastReverse {
                trick_mode_control,
                field_id,
                intra_slice_refresh,
                frequency_truncation,
            } => {
                (trick_mode_control << 5)
                    | (field_id << 3)
                    | ((*intra_slice_refresh as u8) << 2)
                    | frequency_truncation
            }
            TrickMode::SlowReverse {
                trick_mode_control,
                rep_ctrl,
            } => (trick_mode_control << 5) | rep_ctrl,
            TrickMode::Reserved {
                trick_mode_control,
                reserved,
            } => (trick_mode_control << 5) | reserved,
        }
    }
}

impl From<u8> for TrickMode {
    fn from(value: u8) -> Self {
        let trick_mode_control = (value & 0b1110_0000) >> 5;

        match trick_mode_control {
            0b000 => TrickMode::FastForward {
                trick_mode_control,
                field_id: (value & 0b0001_1000) >> 3,
                intra_slice_refresh: ((value & 0b0000_0100) >> 2) != 0,
                frequency_truncation: (value & 0b0000_0011),
            },
            0b001 => TrickMode::SlowMotion {
                trick_mode_control,
                rep_ctrl: value & 0b0001_1111,
            },
            0b010 => TrickMode::FreezeFrame {
                trick_mode_control,
                field_id: (value & 0b0001_1000) >> 3,
            },
            0b011 => TrickMode::FastReverse {
                trick_mode_control,
                field_id: (value & 0b0001_1000) >> 3,
                intra_slice_refresh: ((value & 0b0000_0100) >> 2) != 0,
                frequency_truncation: (value & 0b0000_0011),
            },
            0b100 => TrickMode::SlowReverse {
                trick_mode_control,
                rep_ctrl: value & 0b0001_1111,
            },
            _ => TrickMode::Reserved {
                trick_mode_control,
                reserved: value & 0b0001_1111,
            },
        }
    }
}

#[derive(Debug)]
pub struct PesExtension {
    pub pes_private_data_flag: bool,
    pub pack_header_field_flag: bool,
    pub program_packet_sequence_counter_flag: bool,
    pub p_std_buffer_flag: bool,
    pub pes_extension_flag_2: bool,
    pub pes_private_data: Option<[u8; 16]>,
    pub pack_field_length: Option<u8>,
    pub pack_header: Option<Vec<u8>>, // MEMO: Typically not use for TS (skip detail)
    pub program_packet_sequence_counter: Option<u8>,
    pub mpeg1_mpeg2_identifier: Option<bool>,
    pub original_stuff_length: Option<u8>,
    pub p_std_buffer_scale: Option<bool>,
    pub p_std_buffer_size: Option<u16>,
    pub pes_extension_field_length: Option<u8>,
}

#[derive(Debug)]
pub struct StandardExtension {
    pub pes_scrambling_control: u8,
    pub pes_priority: bool,
    pub data_alignment_indicator: bool,
    pub copyright: bool,
    pub original_or_copy: bool,
    pub pts_dts_flags: u8,
    pub escr_flag: bool,
    pub es_rate_flag: bool,
    pub dsm_trick_mode_flag: bool,
    pub additional_copy_info_flag: bool,
    pub pes_crc_flag: bool,
    pub pes_extension_flag: bool,
    pub pes_header_data_length: u8,
    pub time_stamps: Option<Timestamps>,
    pub escr: Option<u64>,
    pub es_rate: Option<u32>,
    pub trick_mode: Option<TrickMode>,
    pub additional_copy_info: Option<u8>,
    pub previous_pes_packet_crc: Option<u16>,
    pub pes_extension: Option<PesExtension>,
}

impl StandardExtension {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header_data = Vec::with_capacity(self.pes_header_data_length as usize);

        // timestamps
        match &self.time_stamps {
            Some(Timestamps::Pts(pts)) => {
                header_data.extend_from_slice(&encode_timestamp(*pts, 0b0010));
            }
            Some(Timestamps::PtsDts { pts, dts }) => {
                header_data.extend_from_slice(&encode_timestamp(*pts, 0b0011));
                header_data.extend_from_slice(&encode_timestamp(*dts, 0b0001));
            }
            None => {}
        }

        // ESCR (6 bytes)
        if let Some(escr) = self.escr {
            let base = escr / 300;
            let ext = (escr % 300) as u16;
            header_data.push(
                0b1100_0000
                    | (((base >> 30) as u8 & 0b0000_0111) << 3)
                    | 0b0000_0100
                    | ((base >> 28) as u8 & 0b0000_0011),
            );
            header_data.push((base >> 20) as u8);
            header_data.push(
                (((base >> 15) as u8 & 0b0001_1111) << 3)
                    | 0b0000_0100
                    | ((base >> 13) as u8 & 0b0000_0011),
            );
            header_data.push((base >> 5) as u8);
            header_data.push(
                (((base as u8) & 0b0001_1111) << 3)
                    | 0b0000_0100
                    | ((ext >> 7) as u8 & 0b0000_0011),
            );
            header_data.push(((ext as u8) << 1) | 0b0000_0001);
        }

        // ES_rate (3 bytes)
        if let Some(es_rate) = self.es_rate {
            header_data.push(0b1000_0000 | ((es_rate >> 15) as u8 & 0b0111_1111));
            header_data.push((es_rate >> 7) as u8);
            header_data.push(((es_rate as u8) << 1) | 0b0000_0001);
        }

        // trick_mode (1 byte)
        if let Some(tm) = &self.trick_mode {
            header_data.push(u8::from(tm));
        }

        // additional_copy_info (1 byte)
        if let Some(aci) = self.additional_copy_info {
            header_data.push(0b1000_0000 | (aci & 0b0111_1111));
        }

        // previous_pes_packet_crc (2 bytes)
        if let Some(crc) = self.previous_pes_packet_crc {
            header_data.push((crc >> 8) as u8);
            header_data.push(crc as u8);
        }

        // PES extension
        if let Some(ext) = &self.pes_extension {
            header_data.push(
                ((ext.pes_private_data_flag as u8) << 7)
                    | ((ext.pack_header_field_flag as u8) << 6)
                    | ((ext.program_packet_sequence_counter_flag as u8) << 5)
                    | ((ext.p_std_buffer_flag as u8) << 4)
                    | 0b0000_1110
                    | (ext.pes_extension_flag_2 as u8),
            );

            if let Some(ppd) = &ext.pes_private_data {
                header_data.extend_from_slice(ppd);
            }

            if let (Some(len), Some(data)) = (ext.pack_field_length, &ext.pack_header) {
                header_data.push(len);
                header_data.extend_from_slice(data);
            }

            if let (Some(counter), Some(mmi), Some(osl)) = (
                ext.program_packet_sequence_counter,
                ext.mpeg1_mpeg2_identifier,
                ext.original_stuff_length,
            ) {
                header_data.push(0b1000_0000 | (counter & 0b0111_1111));
                header_data.push(0b1000_0000 | ((mmi as u8) << 6) | (osl & 0b0011_1111));
            }

            if let (Some(scale), Some(size)) = (ext.p_std_buffer_scale, ext.p_std_buffer_size) {
                header_data
                    .push(0b0100_0000 | ((scale as u8) << 5) | ((size >> 8) as u8 & 0b0001_1111));
                header_data.push(size as u8);
            }

            if let Some(len) = ext.pes_extension_field_length {
                header_data.push(0b1000_0000 | (len & 0b0111_1111));
            }
        }

        // Stuffing to match original pes_header_data_length
        let data_len = header_data.len();
        if data_len < self.pes_header_data_length as usize {
            header_data.resize(self.pes_header_data_length as usize, 0b1111_1111);
        }

        // Build: flags(2 bytes) + pes_header_data_length(1 byte) + header_data
        let mut buf = Vec::with_capacity(3 + header_data.len());

        // '10'(2) | scrambling(2) | priority(1) | alignment(1) | copyright(1) | original(1)
        buf.push(
            0b1000_0000
                | (self.pes_scrambling_control << 4)
                | ((self.pes_priority as u8) << 3)
                | ((self.data_alignment_indicator as u8) << 2)
                | ((self.copyright as u8) << 1)
                | (self.original_or_copy as u8),
        );

        // pts_dts(2) | escr(1) | es_rate(1) | trick_mode(1) | aci(1) | crc(1) | ext(1)
        buf.push(
            (self.pts_dts_flags << 6)
                | ((self.escr_flag as u8) << 5)
                | ((self.es_rate_flag as u8) << 4)
                | ((self.dsm_trick_mode_flag as u8) << 3)
                | ((self.additional_copy_info_flag as u8) << 2)
                | ((self.pes_crc_flag as u8) << 1)
                | (self.pes_extension_flag as u8),
        );

        buf.push(header_data.len() as u8);
        buf.extend_from_slice(&header_data);

        buf
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 3 {
            return Err(Error::BufferTooShort {
                expected: 3,
                actual: data.len(),
            });
        }

        let pes_scrambling_control = (data[0] & 0b0011_0000) >> 4;
        let pes_priority = ((data[0] & 0b0000_1000) >> 3) != 0;
        let data_alignment_indicator = ((data[0] & 0b0000_0100) >> 2) != 0;
        let copyright = ((data[0] & 0b0000_0010) >> 1) != 0;
        let original_or_copy = (data[0] & 0b0000_0001) != 0;
        let pts_dts_flags = (data[1] & 0b1100_0000) >> 6;
        let escr_flag = ((data[1] & 0b0010_0000) >> 5) != 0;
        let es_rate_flag = ((data[1] & 0b0001_0000) >> 4) != 0;
        let dsm_trick_mode_flag = ((data[1] & 0b0000_1000) >> 3) != 0;
        let additional_copy_info_flag = ((data[1] & 0b0000_0100) >> 2) != 0;
        let pes_crc_flag = ((data[1] & 0b0000_0010) >> 1) != 0;
        let pes_extension_flag = (data[1] & 0b0000_0001) != 0;
        let pes_header_data_length = data[2];
        if data.len() < 3 + pes_header_data_length as usize {
            return Err(Error::BufferTooShort {
                expected: 3 + pes_header_data_length as usize,
                actual: data.len(),
            });
        }

        let mut pos = 3;
        let time_stamps = match pts_dts_flags {
            0b00 => None,
            0b10 => {
                let pts = ((data[pos] as u64 & 0b0000_1110) << 29)
                    | ((data[pos + 1] as u64) << 22)
                    | ((data[pos + 2] as u64 & 0b1111_1110) << 14)
                    | ((data[pos + 3] as u64) << 7)
                    | ((data[pos + 4] as u64) >> 1);
                pos += 5;
                Some(Timestamps::Pts(pts))
            }
            0b11 => {
                let pts = ((data[pos] as u64 & 0b0000_1110) << 29)
                    | ((data[pos + 1] as u64) << 22)
                    | ((data[pos + 2] as u64 & 0b1111_1110) << 14)
                    | ((data[pos + 3] as u64) << 7)
                    | ((data[pos + 4] as u64) >> 1);
                pos += 5;
                let dts = ((data[pos] as u64 & 0b0000_1110) << 29)
                    | ((data[pos + 1] as u64) << 22)
                    | ((data[pos + 2] as u64 & 0b1111_1110) << 14)
                    | ((data[pos + 3] as u64) << 7)
                    | ((data[pos + 4] as u64) >> 1);

                pos += 5;
                Some(Timestamps::PtsDts { pts, dts })
            }
            _ => return Err(Error::InvalidPtsDtsFlags(pts_dts_flags as usize)),
        };

        let escr = if escr_flag {
            let base = ((data[pos] as u64 & 0b0011_1000) << 27)
                | ((data[pos] as u64 & 0b0000_0011) << 28)
                | ((data[pos + 1] as u64) << 20)
                | ((data[pos + 2] as u64 & 0b1111_1000) << 12)
                | ((data[pos + 2] as u64 & 0b0000_0011) << 13)
                | ((data[pos + 3] as u64) << 5)
                | ((data[pos + 4] as u64 & 0b1111_1000) >> 3);
            let ext = ((data[pos + 4] as u64 & 0b0000_0011) << 7) | ((data[pos + 5] as u64) >> 1);
            pos += 6;
            Some(base * 300 + ext)
        } else {
            None
        };

        let es_rate = if es_rate_flag {
            let es_rate = ((data[pos] as u32 & 0b0111_1111) << 15)
                | (data[pos + 1] as u32) << 7
                | (data[pos + 2] as u32) >> 1;
            pos += 3;
            Some(es_rate)
        } else {
            None
        };

        let trick_mode = if dsm_trick_mode_flag {
            let tm = TrickMode::from(data[pos]);
            pos += 1;
            Some(tm)
        } else {
            None
        };

        let additional_copy_info = if additional_copy_info_flag {
            let aci = data[pos] & 0b0111_1111;
            pos += 1;
            Some(aci)
        } else {
            None
        };

        let previous_pes_packet_crc = if pes_crc_flag {
            let pppc = (data[pos] as u16) << 8 | data[pos + 1] as u16;
            pos += 2;
            Some(pppc)
        } else {
            None
        };

        let pes_extension = if pes_extension_flag {
            let pes_private_data_flag = (data[pos] & 0b1000_0000) != 0;
            let pack_header_field_flag = (data[pos] & 0b0100_0000) != 0;
            let program_packet_sequence_counter_flag = (data[pos] & 0b0010_0000) != 0;
            let p_std_buffer_flag = (data[pos] & 0b0001_0000) != 0;
            let pes_extension_flag_2 = (data[pos] & 0b0000_0001) != 0;
            pos += 1;

            let pes_private_data = if pes_private_data_flag {
                let mut ppd = [0u8; 16];
                ppd.copy_from_slice(&data[pos..pos + 16]);
                pos += 16;
                Some(ppd)
            } else {
                None
            };

            let (pack_field_length, pack_header) = if pack_header_field_flag {
                let pack_field_length = data[pos];
                let pack_header = data[pos + 1..pos + 1 + pack_field_length as usize].to_vec();
                pos += 1 + pack_field_length as usize;
                (Some(pack_field_length), Some(pack_header))
            } else {
                (None, None)
            };

            let (program_packet_sequence_counter, mpeg1_mpeg2_identifier, original_stuff_length) =
                if program_packet_sequence_counter_flag {
                    let ppsc = data[pos] & 0b0111_1111;
                    let mmi = (data[pos + 1] & 0b0100_0000) != 0;
                    let osl = data[pos + 1] & 0b0011_1111;
                    pos += 2;
                    (Some(ppsc), Some(mmi), Some(osl))
                } else {
                    (None, None, None)
                };

            let (p_std_buffer_scale, p_std_buffer_size) = if p_std_buffer_flag {
                let scale = (data[pos] & 0b0010_0000) != 0;
                let size = ((data[pos] & 0b0001_1111) as u16) << 8 | (data[pos + 1] as u16);
                pos += 2;
                (Some(scale), Some(size))
            } else {
                (None, None)
            };

            let pes_extension_field_length = if pes_extension_flag_2 {
                let length = data[pos] & 0b0111_1111;
                Some(length)
            } else {
                None
            };

            Some(PesExtension {
                pes_private_data_flag,
                pack_header_field_flag,
                program_packet_sequence_counter_flag,
                p_std_buffer_flag,
                pes_extension_flag_2,
                pes_private_data,
                pack_field_length,
                pack_header,
                program_packet_sequence_counter,
                mpeg1_mpeg2_identifier,
                original_stuff_length,
                p_std_buffer_scale,
                p_std_buffer_size,
                pes_extension_field_length,
            })
        } else {
            None
        };

        let end = 3 + pes_header_data_length as usize;
        if pos > end {
            return Err(Error::BufferTooShort {
                expected: pos,
                actual: end,
            });
        }

        Ok(StandardExtension {
            pes_scrambling_control,
            pes_priority,
            data_alignment_indicator,
            copyright,
            original_or_copy,
            pts_dts_flags,
            escr_flag,
            es_rate_flag,
            dsm_trick_mode_flag,
            additional_copy_info_flag,
            pes_crc_flag,
            pes_extension_flag,
            pes_header_data_length,
            time_stamps,
            escr,
            es_rate,
            trick_mode,
            additional_copy_info,
            previous_pes_packet_crc,
            pes_extension,
        })
    }
}

#[derive(Debug)]
pub enum Extension {
    Standard(StandardExtension),
    DataOnly,
    Padding,
}

#[derive(Debug)]
pub struct Header {
    pub packet_start_code_prefix: u32,
    pub stream_id: StreamId,
    pub pes_packet_length: u16,
    pub extension: Extension,
}

impl Header {
    /// Reconstruct PES header
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![
            // packet_start_code_prefix (3 bytes)
            (self.packet_start_code_prefix >> 16) as u8,
            (self.packet_start_code_prefix >> 8) as u8,
            self.packet_start_code_prefix as u8,
            // stream_id (1 byte)
            u8::from(&self.stream_id),
            // pes_packet_length (2 bytes)
            (self.pes_packet_length >> 8) as u8,
            self.pes_packet_length as u8,
        ];

        // extension
        match &self.extension {
            Extension::Standard(ext) => {
                buf.extend_from_slice(&ext.to_bytes());
            }
            Extension::DataOnly | Extension::Padding => {}
        }

        buf
    }

    /// Parse PES header
    pub fn parse(data: &[u8]) -> Result<(Self, usize), Error> {
        if data.len() < 6 {
            return Err(Error::BufferTooShort {
                expected: 6,
                actual: data.len(),
            });
        }

        let packet_start_code_prefix =
            (data[0] as u32) << 16 | (data[1] as u32) << 8 | (data[2] as u32);
        if packet_start_code_prefix != 0x00_00_01 {
            return Err(Error::InvalidStartCode(packet_start_code_prefix as usize));
        }

        let stream_id = StreamId::from(data[3]);
        let pes_packet_length = (data[4] as u16) << 8 | (data[5] as u16);

        let (extension, consumed) = match stream_id {
            // Padding
            StreamId::PaddingStream => (Extension::Padding, 6),
            // DataOnly
            StreamId::ProgramStreamMap
            | StreamId::PrivateStream2
            | StreamId::EcmStream
            | StreamId::EmmStream
            | StreamId::ProgramStreamDirectory
            | StreamId::DsmCcStream
            | StreamId::H2221TypeE => (Extension::DataOnly, 6),
            // Standard: 6(common header) + 3(flags + length) + pes_header_data_length
            _ => {
                let standard = StandardExtension::parse(&data[6..])?;
                let consumed = 6 + 3 + standard.pes_header_data_length as usize;
                (Extension::Standard(standard), consumed)
            }
        };

        Ok((
            Header {
                packet_start_code_prefix,
                stream_id,
                pes_packet_length,
                extension,
            },
            consumed,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pes_header_with_pts() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code_prefix
            0xE0, // stream_id (video)
            0x00, 0x00, // PES_packet_length (0 = unbounded)
            0x80, // flags1: '10'=marker, scrambling=00, priority=0, alignment=0, copyright=0, original=0
            0x80, // flags2: PTS=10, DTS=00, other flags=0
            0x05, // PES_header_data_length=5 (PTS only)
            // PTS: 90000 (0x15F90)
            0x21, 0x00, 0x05, 0xBF, 0x21,
        ];

        let result = Header::parse(&data);
        assert!(result.is_ok());

        let (header, consumed) = result.unwrap();
        assert_eq!(header.packet_start_code_prefix, 0x000001);
        assert!(matches!(header.stream_id, StreamId::Video(0)));
        assert_eq!(header.pes_packet_length, 0);
        assert_eq!(consumed, 14); // 6 + 3 + 5

        match &header.extension {
            Extension::Standard(ext) => {
                assert_eq!(ext.pts_dts_flags, 0b10);
                assert_eq!(ext.pes_header_data_length, 5);
                assert!(matches!(ext.time_stamps, Some(Timestamps::Pts(90000))));
            }
            _ => panic!("Expected Standard extension"),
        }
    }

    #[test]
    fn test_pes_header_with_pts_dts() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code
            0xE0, // stream_id (video)
            0x00, 0x64, // PES_packet_length=100
            0x84, // flags1: marker=10, alignment=1
            0xC0, // flags2: PTS_DTS=11 (both)
            0x0A, // header_data_length=10 (5 for PTS + 5 for DTS)
            // PTS: 90000 (0x15F90) prefix=0011
            0x31, 0x00, 0x05, 0xBF, 0x21,
            // DTS: 45000 (0xAFC8) prefix=0001
            0x11, 0x00, 0x03, 0x5F, 0x91,
        ];

        let result = Header::parse(&data);
        assert!(result.is_ok());

        let (header, consumed) = result.unwrap();
        assert_eq!(header.pes_packet_length, 100);
        assert_eq!(consumed, 19); // 6 + 3 + 10

        match &header.extension {
            Extension::Standard(ext) => {
                assert!(ext.data_alignment_indicator);
                assert_eq!(ext.pts_dts_flags, 0b11);
                assert!(matches!(
                    ext.time_stamps,
                    Some(Timestamps::PtsDts {
                        pts: 90000,
                        dts: 45000
                    })
                ));
            }
            _ => panic!("Expected Standard extension"),
        }
    }

    #[test]
    fn test_pes_header_no_timestamp() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code_prefix
            0xC0, // stream_id (audio)
            0x00, 0x10, // PES_packet_length=16
            0x80, // flags1: marker=10, scrambling=00, priority=0, alignment=0, copyright=0, original=0
            0x00, // flags2: PTS_DTS=00, other flags=0
            0x00, // PES_header_data_length=0
        ];

        let (header, consumed) = Header::parse(&data).unwrap();
        assert!(matches!(header.stream_id, StreamId::Audio(0)));
        assert_eq!(header.pes_packet_length, 16);
        assert_eq!(consumed, 9); // 6 + 3 + 0

        match &header.extension {
            Extension::Standard(ext) => {
                assert_eq!(ext.pts_dts_flags, 0b00);
                assert_eq!(ext.pes_header_data_length, 0);
                assert!(ext.time_stamps.is_none());
                assert!(ext.escr.is_none());
                assert!(ext.es_rate.is_none());
                assert!(ext.trick_mode.is_none());
                assert!(ext.pes_extension.is_none());
            }
            _ => panic!("Expected Standard extension"),
        }
    }

    #[test]
    fn test_pes_header_padding_stream() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code_prefix
            0xBE, // stream_id (padding)
            0x00, 0x08, // PES_packet_length=8
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // padding bytes
        ];

        let (header, consumed) = Header::parse(&data).unwrap();
        assert!(matches!(header.stream_id, StreamId::PaddingStream));
        assert_eq!(header.pes_packet_length, 8);
        assert_eq!(consumed, 6);
        assert!(matches!(header.extension, Extension::Padding));
    }

    #[test]
    fn test_pes_header_data_only_stream() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code_prefix
            0xBF, // stream_id (private_stream_2)
            0x00, 0x04, // PES_packet_length=4
            0x01, 0x02, 0x03, 0x04, // data bytes
        ];

        let (header, consumed) = Header::parse(&data).unwrap();
        assert!(matches!(header.stream_id, StreamId::PrivateStream2));
        assert_eq!(header.pes_packet_length, 4);
        assert_eq!(consumed, 6);
        assert!(matches!(header.extension, Extension::DataOnly));
    }

    #[test]
    fn test_pes_header_invalid_start_code() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x02, // invalid start_code_prefix
            0xE0, 0x00, 0x00,
        ];

        let result = Header::parse(&data);
        assert_eq!(result.unwrap_err(), Error::InvalidStartCode(0x000002));
    }

    #[test]
    fn test_pes_header_too_short() {
        let data = vec![0x00, 0x00, 0x01];

        let result = Header::parse(&data);
        assert_eq!(
            result.unwrap_err(),
            Error::BufferTooShort {
                expected: 6,
                actual: 3,
            }
        );
    }

    #[test]
    fn test_mux() {
        #[rustfmt::skip]
        let data = vec![
            0x00, 0x00, 0x01, // start_code
            0xE0, // stream_id (video)
            0x00, 0x64, // PES_packet_length=100
            0x84, // flags1: marker=10, alignment=1
            0xC0, // flags2: PTS_DTS=11 (both)
            0x0A, // header_data_length=10 (5 for PTS + 5 for DTS)
            // PTS: 90000 (0x15F90) prefix=0011
            0x31, 0x00, 0x05, 0xBF, 0x21,
            // DTS: 45000 (0xAFC8) prefix=0001
            0x11, 0x00, 0x03, 0x5F, 0x91,
        ];

        let pes_header = Header::parse(&data).unwrap();
        let reconstructed = pes_header.0.to_bytes();
        assert_eq!(reconstructed, data.to_vec());
    }
}
