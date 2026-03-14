//! ADTS (Audio Data Transport Stream) parser
//!
//! ADTS Frame construction
//! ┌───────────────────────────────────────────────┐
//! │  ADTS Header (7 bytes, or 9 bytes with CRC)   │
//! ├───────────────────────────────────────────────┤
//! │  Raw AAC Frame (variable)                     │
//! └───────────────────────────────────────────────┘
//!
//! ADTS Header construction
//! ┌───────────────────────────────────────────────┐
//! │  syncword (12 bits)                           │ <- Fixed at 0xFFF
//! │  ID (1 bit)                                   │ <- 0: MPEG-4, 1: MPEG-2
//! │  layer (2 bits)                               │ <- Fixed at 0b00
//! │  protection_absent (1 bit)                    │ <- 0: CRC present, 1: no CRC
//! │  profile (2 bits)                             │ <- 0: Main, 1: LC, 2: SSR, 3: Reserved
//! │  sampling_frequency_index (4 bits)            │ <- e.g. 3: 48000Hz, 4: 44100Hz
//! │  private_bit (1 bit)                          │
//! │  channel_configuration (3 bits)               │ <- e.g. 1: mono, 2: stereo
//! │  original_copy (1 bit)                        │
//! │  home (1 bit)                                 │
//! │  copyright_identification_bit (1 bit)         │
//! │  copyright_identification_start (1 bit)       │
//! │  aac_frame_length (13 bits)                   │ <- Length of the entire frame including header
//! │  adts_buffer_fullness (11 bits)               │ <- 0x7FF: VBR
//! │  number_of_raw_data_blocks_in_frame (2 bits)  │
//! ├───────────────────────────────────────────────┤
//! │  CRC (16 bits, optional)                      │ <- Present if protection_absent == 0
//! └───────────────────────────────────────────────┘

use crate::aac::error::Error;

#[derive(Debug)]
pub struct Adts {
    pub id: bool,
    pub protection_absent: bool,
    pub profile: u8,
    pub sampling_frequency_index: u8,
    pub private_bit: bool,
    pub channel_configuration: u8,
    pub original_copy: bool,
    pub home: bool,
    pub copyright_identification_bit: bool,
    pub copyright_identification_start: bool,
    pub aac_frame_length: u16,
    pub adts_buffer_fullness: u16,
    pub number_of_raw_data_blocks_in_frame: u8,
    pub crc: Option<u16>,
    pub payload: Vec<u8>,
}

impl Adts {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![
            0xFF,
            0xF0 | (self.id as u8) << 3 | self.protection_absent as u8,
            self.profile << 6
                | self.sampling_frequency_index << 2
                | (self.private_bit as u8) << 1
                | (self.channel_configuration >> 2) & 1,
            (self.channel_configuration & 0b11) << 6
                | (self.original_copy as u8) << 5
                | (self.home as u8) << 4
                | (self.copyright_identification_bit as u8) << 3
                | (self.copyright_identification_start as u8) << 2
                | ((self.aac_frame_length >> 11) & 0b11) as u8,
            (self.aac_frame_length >> 3) as u8,
            ((self.aac_frame_length & 0b111) as u8) << 5
                | ((self.adts_buffer_fullness >> 6) & 0b11111) as u8,
            ((self.adts_buffer_fullness & 0b111111) as u8) << 2
                | self.number_of_raw_data_blocks_in_frame & 0b11,
        ];
        if let Some(crc) = self.crc {
            buf.push((crc >> 8) as u8);
            buf.push(crc as u8);
        }
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Parse a single ADTS frame from a byte slice
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < 7 {
            return Err(Error::DataTooShort);
        }

        let syncword = (data[0] as u16) << 4 | (data[1] >> 4) as u16;
        if syncword != 0xFFF {
            return Err(Error::InvalidSyncword(syncword));
        }

        let id = ((data[1] & 0b0000_1000) >> 3) != 0;
        let layer = (data[1] & 0b0000_0110) >> 1;
        if layer != 0 {
            return Err(Error::InvalidLayer(layer));
        }

        let protection_absent = (data[1] & 0b0000_0001) != 0;
        let profile = (data[2] & 0b1100_0000) >> 6;
        let sampling_frequency_index = (data[2] & 0b0011_1100) >> 2;
        let private_bit = ((data[2] & 0b0000_0010) >> 1) != 0;
        let channel_configuration = ((data[2] & 0b0000_0001) << 2) | ((data[3] & 0b1100_0000) >> 6);
        let original_copy = ((data[3] & 0b0010_0000) >> 5) != 0;
        let home = ((data[3] & 0b0001_0000) >> 4) != 0;
        let copyright_identification_bit = ((data[3] & 0b0000_1000) >> 3) != 0;
        let copyright_identification_start = ((data[3] & 0b0000_0100) >> 2) != 0;
        let aac_frame_length = (((data[3] & 0b0000_0011) as u16) << 11)
            | ((data[4] as u16) << 3)
            | (((data[5] & 0b1110_0000) as u16) >> 5);
        let adts_buffer_fullness =
            (((data[5] & 0b0001_1111) as u16) << 6) | (((data[6] & 0b1111_1100) as u16) >> 2);
        let number_of_raw_data_blocks_in_frame = data[6] & 0b0000_0011;

        let mut payload_start = 7;
        let crc = if !protection_absent {
            payload_start = 9;
            Some(((data[7] as u16) << 8) | (data[8] as u16))
        } else {
            None
        };

        let frame_length = aac_frame_length as usize;
        if data.len() < frame_length {
            return Err(Error::DataTooShort);
        }
        let payload = data[payload_start..frame_length].to_vec();

        Ok(Self {
            id,
            protection_absent,
            profile,
            sampling_frequency_index,
            private_bit,
            channel_configuration,
            original_copy,
            home,
            copyright_identification_bit,
            copyright_identification_start,
            aac_frame_length,
            adts_buffer_fullness,
            number_of_raw_data_blocks_in_frame,
            crc,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_frame() -> Vec<u8> {
        vec![
            0xFF, 0xF1, 0x50, 0x40, 0x01, 0x1F, 0xFC, // 7-byte header
            0xAA, // 1-byte payload (frame_length=8)
        ]
    }

    #[test]
    fn test_parse() {
        let data = build_frame();
        let result = Adts::parse(&data).unwrap();

        assert!(!result.id);
        assert!(result.protection_absent);
        assert_eq!(result.profile, 1); // AAC-LC
        assert_eq!(result.sampling_frequency_index, 4); // 44100Hz
        assert!(!result.private_bit);
        assert_eq!(result.channel_configuration, 1); // mono
        assert!(!result.original_copy);
        assert!(!result.home);
        assert!(!result.copyright_identification_bit);
        assert!(!result.copyright_identification_start);
        assert_eq!(result.aac_frame_length, 8);
        assert_eq!(result.adts_buffer_fullness, 0x7FF);
        assert_eq!(result.number_of_raw_data_blocks_in_frame, 0);
        assert!(result.crc.is_none());
        assert_eq!(result.payload, vec![0xAA]);
    }

    #[test]
    fn test_parse_with_crc() {
        let data = vec![
            0xFF, 0xF0, 0x50, 0x40, 0x01, 0x5F, 0xFC, // 7-byte fixed header
            0xDE, 0xAD, // CRC
            0xBB, // 1-byte payload
        ];
        let result = Adts::parse(&data).unwrap();

        assert!(!result.protection_absent);
        assert_eq!(result.crc, Some(0xDEAD));
        assert_eq!(result.aac_frame_length, 10);
        assert_eq!(result.payload, vec![0xBB]);
    }

    #[test]
    fn test_parse_all_flags_set() {
        let data = vec![0xFF, 0xF9, 0x52, 0xBC, 0x01, 0x1F, 0xFC, 0xCC];
        let result = Adts::parse(&data).unwrap();

        assert!(result.id);
        assert!(result.private_bit);
        assert!(result.original_copy);
        assert!(result.home);
        assert!(result.copyright_identification_bit);
        assert!(result.copyright_identification_start);
    }

    #[test]
    fn test_data_too_short() {
        let data = vec![0xFF, 0xF1, 0x50];
        let result = Adts::parse(&data);
        assert_eq!(result.err(), Some(Error::DataTooShort));
    }

    #[test]
    fn test_data_too_short_for_frame_length() {
        // aac_frame_length = 8, data.len() = 7
        let data = vec![0xFF, 0xF1, 0x50, 0x40, 0x01, 0x1F, 0xFC];
        let result = Adts::parse(&data);
        assert_eq!(result.err(), Some(Error::DataTooShort));
    }

    #[test]
    fn test_invalid_syncword() {
        let data = vec![0x00, 0x00, 0x50, 0x40, 0x01, 0x1F, 0xFC, 0xAA];
        let result = Adts::parse(&data);
        assert!(matches!(result.err(), Some(Error::InvalidSyncword(_))));
    }

    #[test]
    fn test_invalid_layer() {
        // data[1] = 0xF3 → layer = 0b01
        let data = vec![0xFF, 0xF3, 0x50, 0x40, 0x01, 0x1F, 0xFC, 0xAA];
        let result = Adts::parse(&data);
        assert_eq!(result.err(), Some(Error::InvalidLayer(1)));
    }

    #[test]
    fn test_roundtrip() {
        let data = build_frame();
        let parsed = Adts::parse(&data).unwrap();
        let output = parsed.to_bytes();
        assert_eq!(data, output);
    }

    #[test]
    fn test_roundtrip_with_crc() {
        let data = vec![0xFF, 0xF0, 0x50, 0x40, 0x01, 0x5F, 0xFC, 0xDE, 0xAD, 0xBB];
        let parsed = Adts::parse(&data).unwrap();
        let output = parsed.to_bytes();
        assert_eq!(data, output);
    }

    #[test]
    fn test_roundtrip_all_flags_set() {
        let data = vec![0xFF, 0xF9, 0x52, 0xBC, 0x01, 0x1F, 0xFC, 0xCC];
        let parsed = Adts::parse(&data).unwrap();
        let output = parsed.to_bytes();
        assert_eq!(data, output);
    }
}
