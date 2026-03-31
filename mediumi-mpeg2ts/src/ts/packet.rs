//! TS packet parser
//!
//! TS packet construction
//! Size of a TS packet is fixed at 188 bytes
//! ```text
//! ┌──────────────────────────────────────┐
//! │  Header(4 bytes)                     │
//! ├──────────────────────────────────────┤
//! │  Adaptation Field (variable length)  │
//! ├──────────────────────────────────────┤
//! │  Payload (variable length)           │
//! └──────────────────────────────────────┘
//! ```
//!
//! Header
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  sync_byte (1 byte)                     │ <- Fixed at 0x47
//! │  transport_error_indicator (1 bit)      │ <- Must be 0. if 1, this packet may contain error.
//! │  payload_unit_start_indicator (1 bit)   │ <- If 1, payload starts a new PES
//! │  transport_priority (1 bit)             │ <- If 1, higher priority
//! │  pid (13 bits)                          │ <- Program identifier
//! │  transport_scrambling_control (2 bits)  │ <- 00: not scrambled, 01 | 10 | 11: user-defined
//! │  adaptation_field_control (2 bits)      │ <- 00: reserved, 01: only payload, 10: only AF, 11: AF + payload
//! │  continuity_counter (4 bits)            │ <- Increments 0..15, wraps around
//! └─────────────────────────────────────────┘
//! ```
//!
//! Adaptation Field
//! If AFC is 10 or 11, this field is present after Header.
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  adaptation_field_length (1 byte)                      │ <- Length of this field (not include itself)
//! ├────────────────────────────────────────────────────────┤
//!
//! If adaptation_field_length > 0:
//! ┌────────────────────────────────────────────────────────┐
//! │  discontinuity_indicator (1 bit)                       │ <- If 1, this TS packet doesn't guarantee continuity
//! │  random_access_indicator (1 bit)                       │ <- If 1, this packet is a random access point (mostly, decoder can start from here,typically contains I-frame)
//! │  elementary_stream_priority_indicator (1 bit)          │ <- If 1, ES of this packet has higher priority
//! │  PCR_flag (1 bit)                                      │ <- If 1, AF contains PCR
//! │  OPCR_flag (1 bit)                                     │ <- If 1, AF contains OPCR
//! │  splicing_point_flag (1 bit)                           │ <- If 1, AF contains splice_countdown
//! │  transport_private_data_flag (1 bit)                   │ <- If 1, AF contains private data
//! │  adaptation_field_extension_flag (1 bit)               │ <- If 1, AF contains extension field
//! │  optional_adaptation_field (variable length)           │ <- Optional Adaptation Field
//! │  stuffing_byte (remaining bytes)                       │ <- fixed at 0xFF
//! └────────────────────────────────────────────────────────┘
//!
//! Optional Adaptation Field
//! if PCR_flag == 1
//! ┌────────────────────────────────────────────────────────┐
//! │  program_clock_reference_base (33 bits)                │ <- Pcr base (90kHz)
//! │  reserved (6 bits)                                     │ <- Fixed at 0b111111
//! │  program_clock_reference_extension (9 bits)            │ <- Pcr extension (27MHz)
//! ├────────────────────────────────────────────────────────┤
//!
//! PCR is a reference clock for synchronizing decoder's system time clock
//! PCR(i) = PCR_base(i) * 300 + PCR_ext(i)
//!
//! if OPCR_flag == 1
//! │  original_program_clock_reference_base (33 bits)       │ <- OPCR base (90kHz)
//! │  reserved (6 bits)                                     │ <- Fixed at 0b111111
//! │  original_program_clock_reference_extension (9 bits)   │ <- OPCR extension (27MHz)
//! ├────────────────────────────────────────────────────────┤
//!
//! OPCR preserves the original PCR value when a stream is re-multiplexed.
//! Used for maintaining timing information from the original source.
//! OPCR(i) = OPCR_base(i) * 300 + OPCR_ext(i)
//!
//! if splicing_point_flag == 1
//! │  splice_countdown (1 byte)                             │ <- Countdown to splice point(for insertion like cm, ads)
//! ├────────────────────────────────────────────────────────┤
//!
//! if transport_private_data_flag == 1
//! │  transport_private_data_length (1 byte)                │ <- private data length
//! │  private_data_byte (remaining bytes)                   │ <- private data
//! ├────────────────────────────────────────────────────────┤
//!
//! if adaptation_field_extension_flag == 1
//! │  adaptation_field_extension_length (1 byte)            │ <- extension length
//! │  ltw_flag (1 bit)                                      │ <- If 1, AF contains legal time window(LTW)
//! │  piecewise_rate_flag (1 bit)                           │ <- If 1, AF contains piecewise rate field
//! │  seamless_splice_flag (1 bit)                          │ <- If 1, AF contains seamless splice
//! │  reserved (5 bits)                                     │ <- Fixed at 0b11111
//! ├────────────────────────────────────────────────────────┤
//!
//! if ltw_flag == 1
//! │  ltw_valid_flag (1 bit)                                │ <- If 1, ltw_offset is valid
//! │  ltw_offset (15 bits)                                  │ <- legal time offset
//! ├────────────────────────────────────────────────────────┤
//!
//! if piecewise_rate_flag == 1
//! │  reserved (2 bits)                                     │ <- Fixed at 0b11
//! │  piecewise_rate (22 bits)                              │ <- Bitrate while reaching next LTW
//! ├────────────────────────────────────────────────────────┤
//!
//! if seamless_splice_flag == 1
//! │  splice_type (4 bits)                                  │ <- ES Condition for splicing
//! │  DTS_next_AU[32..30] (3 bits)                          │ <- DTS_next_AU: Decode time for Next Access Unit after splice point
//! │  marker_bit (1 bit)                                    │
//! │  DTS_next_AU[29..15] (15 bits)                         │
//! │  marker_bit (1 bit)                                    │
//! │  DTS_next_AU[14..0] (15 bits)                          │
//! │  marker_bit (1 bit)                                    │
//! │  reserved (remaining bytes)                            │
//! └────────────────────────────────────────────────────────┘
//!
//! Payload
//! ┌─────────────────────────────┐
//! │  payload (variable length)  │
//! └─────────────────────────────┘
//! ```

use crate::ts::error::Error;

#[derive(Debug)]
pub struct Header {
    pub sync_byte: u8,
    pub transport_error_indicator: bool,
    pub payload_unit_start_indicator: bool,
    pub transport_priority: bool,
    pub pid: u16,
    pub transport_scrambling_control: u8,
    pub adaptation_field_control: u8,
    pub continuity_counter: u8,
}

impl Header {
    /// Reconstruct TS packet header
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            self.sync_byte,
            ((self.transport_error_indicator as u8) << 7)
                | ((self.payload_unit_start_indicator as u8) << 6)
                | ((self.transport_priority as u8) << 5)
                | ((self.pid >> 8) as u8 & 0b0001_1111),
            self.pid as u8,
            (self.transport_scrambling_control << 6)
                | (self.adaptation_field_control << 4)
                | (self.continuity_counter & 0b0000_1111),
        ]
    }

    /// Parse a TS packet header
    fn parse(packet: &[u8]) -> Result<Self, Error> {
        // ts packet must start with 0x47
        if packet[0] != 0x47 {
            return Err(Error::InvalidSyncByte(packet[0]));
        }

        Ok(Header {
            sync_byte: packet[0],
            transport_error_indicator: (packet[1] & 0b1000_0000) != 0,
            payload_unit_start_indicator: (packet[1] & 0b0100_0000) != 0,
            transport_priority: (packet[1] & 0b0010_0000) != 0,
            pid: (((packet[1] & 0b0001_1111) as u16) << 8 | (packet[2]) as u16),
            transport_scrambling_control: (packet[3] & 0b1100_0000) >> 6,
            adaptation_field_control: (packet[3] & 0b0011_0000) >> 4,
            continuity_counter: packet[3] & 0b0000_1111,
        })
    }
}

#[derive(Debug)]
pub struct AdaptationFieldFlags {
    pub discontinuity_indicator: bool,
    pub random_access_indicator: bool,
    pub elementary_stream_priority_indicator: bool,
    pub pcr_flag: bool,
    pub opcr_flag: bool,
    pub splicing_point_flag: bool,
    pub transport_private_data_flag: bool,
    pub adaptation_field_extension_flag: bool,
}

#[derive(Debug)]
pub struct Pcr {
    pub program_clock_reference_base: u64,
    pub program_clock_reference_extension: u16,
}

#[derive(Debug)]
pub struct Opcr {
    pub original_program_clock_reference_base: u64,
    pub original_program_clock_reference_extension: u16,
}

#[derive(Debug)]
pub struct PrivateData {
    pub transport_private_data_length: u8,
    pub private_data_byte: Vec<u8>,
}

#[derive(Debug)]
pub struct AdaptationFieldExtension {
    pub adaptation_field_extension_length: u8,
    pub ltw_flag: bool,
    pub piecewise_rate_flag: bool,
    pub seamless_splice_flag: bool,
    pub ltw_valid_flag: Option<bool>,
    pub ltw_offset: Option<u16>,
    pub piecewise_rate: Option<u32>,
    pub splice_type: Option<u8>,
    pub dts_next_au: Option<u64>,
}

#[derive(Debug)]
pub struct AdaptationField {
    pub adaptation_field_length: u8,
    pub flags: Option<AdaptationFieldFlags>,
    pub pcr: Option<Pcr>,
    pub opcr: Option<Opcr>,
    pub splice_countdown: Option<u8>,
    pub private_data: Option<PrivateData>,
    pub extension: Option<AdaptationFieldExtension>,
    pub stuffing_byte: Option<Vec<u8>>,
}

impl AdaptationField {
    /// Reconstruct adaptation field
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.adaptation_field_length as usize + 1);

        // adaptation_field_length
        buf.push(self.adaptation_field_length);

        let flags = match &self.flags {
            Some(f) => f,
            None => return buf, // length-only AF
        };

        // flags byte
        buf.push(
            ((flags.discontinuity_indicator as u8) << 7)
                | ((flags.random_access_indicator as u8) << 6)
                | ((flags.elementary_stream_priority_indicator as u8) << 5)
                | ((flags.pcr_flag as u8) << 4)
                | ((flags.opcr_flag as u8) << 3)
                | ((flags.splicing_point_flag as u8) << 2)
                | ((flags.transport_private_data_flag as u8) << 1)
                | (flags.adaptation_field_extension_flag as u8),
        );

        // PCR (6 bytes)
        if let Some(pcr) = &self.pcr {
            let base = pcr.program_clock_reference_base;
            let ext = pcr.program_clock_reference_extension;
            buf.push((base >> 25) as u8);
            buf.push((base >> 17) as u8);
            buf.push((base >> 9) as u8);
            buf.push((base >> 1) as u8);
            buf.push(
                ((base & 0b0000_0001) as u8) << 7 | 0b0111_1110 | ((ext >> 8) as u8 & 0b0000_0001),
            );
            buf.push(ext as u8);
        }

        // OPCR (6 bytes)
        if let Some(opcr) = &self.opcr {
            let base = opcr.original_program_clock_reference_base;
            let ext = opcr.original_program_clock_reference_extension;
            buf.push((base >> 25) as u8);
            buf.push((base >> 17) as u8);
            buf.push((base >> 9) as u8);
            buf.push((base >> 1) as u8);
            buf.push(
                ((base & 0b0000_0001) as u8) << 7 | 0b0111_1110 | ((ext >> 8) as u8 & 0b0000_0001),
            );
            buf.push(ext as u8);
        }

        // splice_countdown
        if let Some(sc) = self.splice_countdown {
            buf.push(sc);
        }

        // private_data
        if let Some(pd) = &self.private_data {
            buf.push(pd.private_data_byte.len() as u8);
            buf.extend_from_slice(&pd.private_data_byte);
        }

        // extension
        if let Some(ext) = &self.extension {
            buf.push(ext.adaptation_field_extension_length);
            buf.push(
                ((ext.ltw_flag as u8) << 7)
                    | ((ext.piecewise_rate_flag as u8) << 6)
                    | ((ext.seamless_splice_flag as u8) << 5)
                    | 0b0001_1111,
            );

            if let (Some(valid), Some(offset)) = (ext.ltw_valid_flag, ext.ltw_offset) {
                buf.push(((valid as u8) << 7) | ((offset >> 8) as u8 & 0b0111_1111));
                buf.push(offset as u8);
            }

            if let Some(rate) = ext.piecewise_rate {
                buf.push(0b1100_0000 | ((rate >> 16) as u8 & 0b0011_1111));
                buf.push((rate >> 8) as u8);
                buf.push(rate as u8);
            }

            if let (Some(st), Some(dts)) = (ext.splice_type, ext.dts_next_au) {
                buf.push((st << 4) | (((dts >> 30) as u8 & 0b0000_0111) << 1) | 0b0000_0001);
                buf.push((dts >> 22) as u8);
                buf.push((((dts >> 15) as u8 & 0b0111_1111) << 1) | 0b0000_0001);
                buf.push((dts >> 7) as u8);
                buf.push(((dts as u8 & 0b0111_1111) << 1) | 0b0000_0001);
            }
        }

        // stuffing bytes
        if let Some(stuffing) = &self.stuffing_byte {
            buf.extend_from_slice(stuffing);
        }

        buf
    }

    /// Parse a TS packet adaptation_field
    fn parse(af: &[u8]) -> Result<(Self, usize), Error> {
        let adaptation_field_length = af[0];

        if af.len() < (adaptation_field_length as usize) + 1 {
            return Err(Error::BufferTooShort {
                expected: (adaptation_field_length) as usize + 1,
                actual: af.len(),
            });
        }

        let flags = if adaptation_field_length > 0 {
            AdaptationFieldFlags {
                discontinuity_indicator: (af[1] & 0b1000_0000) != 0,
                random_access_indicator: (af[1] & 0b0100_0000) != 0,
                elementary_stream_priority_indicator: (af[1] & 0b0010_0000) != 0,
                pcr_flag: (af[1] & 0b0001_0000) != 0,
                opcr_flag: (af[1] & 0b0000_1000) != 0,
                splicing_point_flag: (af[1] & 0b0000_0100) != 0,
                transport_private_data_flag: (af[1] & 0b0000_0010) != 0,
                adaptation_field_extension_flag: (af[1] & 0b0000_0001) != 0,
            }
        } else {
            // only AFL
            return Ok((
                AdaptationField {
                    adaptation_field_length,
                    flags: None,
                    pcr: None,
                    opcr: None,
                    splice_countdown: None,
                    private_data: None,
                    extension: None,
                    stuffing_byte: None,
                },
                1, // consumed bytes
            ));
        };

        let mut pos = 2;

        let pcr = if flags.pcr_flag {
            let base = (af[pos] as u64) << 25
                | (af[pos + 1] as u64) << 17
                | (af[pos + 2] as u64) << 9
                | (af[pos + 3] as u64) << 1
                | (af[pos + 4] as u64) >> 7;
            let extension = ((af[pos + 4] & 0b0000_0001) as u16) << 8 | af[pos + 5] as u16; // reserved 6 bits
            pos += 6;
            Some(Pcr {
                program_clock_reference_base: base,
                program_clock_reference_extension: extension,
            })
        } else {
            None
        };

        let opcr = if flags.opcr_flag {
            let base = (af[pos] as u64) << 25
                | (af[pos + 1] as u64) << 17
                | (af[pos + 2] as u64) << 9
                | (af[pos + 3] as u64) << 1
                | (af[pos + 4] as u64) >> 7;
            let extension = ((af[pos + 4] & 0b0000_0001) as u16) << 8 | af[pos + 5] as u16; // reserved 6 bits
            pos += 6;
            Some(Opcr {
                original_program_clock_reference_base: base,
                original_program_clock_reference_extension: extension,
            })
        } else {
            None
        };

        let splice_countdown = if flags.splicing_point_flag {
            let sc = af[pos];
            pos += 1;
            Some(sc)
        } else {
            None
        };

        let private_data = if flags.transport_private_data_flag {
            let length = af[pos];
            if pos + 1 + length as usize > adaptation_field_length as usize + 1 {
                return Err(Error::BufferTooShort {
                    expected: pos + 1 + length as usize,
                    actual: adaptation_field_length as usize + 1,
                });
            }
            let data = af[pos + 1..pos + 1 + length as usize].to_vec();
            pos += 1 + length as usize;
            Some(PrivateData {
                transport_private_data_length: length,
                private_data_byte: data,
            })
        } else {
            None
        };

        let extension = if flags.adaptation_field_extension_flag {
            let adaptation_field_extension_length = af[pos];
            let ltw_flag = (af[pos + 1] & 0b1000_0000) != 0;
            let piecewise_rate_flag = (af[pos + 1] & 0b0100_0000) != 0;
            let seamless_splice_flag = (af[pos + 1] & 0b0010_0000) != 0; // remaining 5bits are reserved
            pos += 2;

            let (ltw_valid_flag, ltw_offset) = if ltw_flag {
                let flag = af[pos] & 0b1000_0000 != 0;
                let value = ((af[pos] & 0b0111_1111) as u16) << 8 | af[pos + 1] as u16;
                pos += 2;
                (Some(flag), Some(value))
            } else {
                (None, None)
            };

            let piecewise_rate = if piecewise_rate_flag {
                let rate = ((af[pos] & 0b0011_1111) as u32) << 16
                    | (af[pos + 1] as u32) << 8
                    | af[pos + 2] as u32;
                pos += 3;
                Some(rate)
            } else {
                None
            };

            let (splice_type, dts_next_au) = if seamless_splice_flag {
                let tp = (af[pos] & 0b1111_0000) >> 4;
                let dts_32_30 = ((af[pos] & 0b0000_1110) >> 1) as u64;
                let dts_29_15 = (((af[pos + 1] as u64) << 7) | ((af[pos + 2] as u64) >> 1))
                    & 0b0111_1111_1111_1111;
                let dts_14_0 = (((af[pos + 3] as u64) << 7) | ((af[pos + 4] as u64) >> 1))
                    & 0b0111_1111_1111_1111;
                pos += 5;
                (
                    Some(tp),
                    Some((dts_32_30 << 30) | (dts_29_15 << 15) | dts_14_0),
                )
            } else {
                (None, None)
            };

            Some(AdaptationFieldExtension {
                adaptation_field_extension_length,
                ltw_flag,
                piecewise_rate_flag,
                seamless_splice_flag,
                ltw_valid_flag,
                ltw_offset,
                piecewise_rate,
                splice_type,
                dts_next_au,
            })
        } else {
            None
        };

        let stuffing_byte = if pos < adaptation_field_length as usize + 1 {
            let start = pos;
            let end = adaptation_field_length as usize + 1;
            Some(af[start..end].to_vec())
        } else {
            None
        };

        Ok((
            AdaptationField {
                adaptation_field_length,
                flags: Some(flags),
                pcr,
                opcr,
                splice_countdown,
                private_data,
                extension,
                stuffing_byte,
            },
            adaptation_field_length as usize + 1,
        ))
    }
}

#[derive(Debug)]
pub struct Packet {
    pub header: Header,
    pub adaptation_field: Option<AdaptationField>,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Reconstruct raw TS packet data
    pub fn to_bytes(&self) -> [u8; 188] {
        let mut buf = [0xFF; 188];
        let mut pos = 4;
        buf[0..4].copy_from_slice(&self.header.to_bytes());
        if let Some(af) = &self.adaptation_field {
            let af_bytes = af.to_bytes();
            buf[pos..pos + af_bytes.len()].copy_from_slice(&af_bytes);
            pos += af_bytes.len();
        }
        buf[pos..pos + self.payload.len()].copy_from_slice(&self.payload);
        buf
    }

    /// Parse a TS packet
    pub fn parse(packet: &[u8]) -> Result<Self, Error> {
        if packet.len() != 188 {
            return Err(Error::InvalidTsPacketLength(packet.len()));
        }

        let header = Header::parse(packet)?;
        let afc = header.adaptation_field_control;
        let (adaptation_field, payload) = match afc {
            0b01 => (None, packet[4..188].to_vec()), // only payload
            0b10 => {
                // only AF
                let (af, _) = AdaptationField::parse(&packet[4..])?;
                (Some(af), vec![])
            }
            0b11 => {
                // AF + payload
                let (af, consumed) = AdaptationField::parse(&packet[4..])?;
                let payload = packet[4 + consumed..188].to_vec();
                (Some(af), payload)
            }
            0b00 => {
                // reserved
                return Err(Error::InvalidAfc);
            }
            _ => unreachable!(),
        };

        Ok(Packet {
            header,
            adaptation_field,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header() {
        let mut packet = vec![0; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0011_0001; // TSC=00, AFC=11, CC=1

        let header = Header::parse(&packet);
        assert!(header.is_ok());

        let result = header.unwrap();
        assert_eq!(result.sync_byte, 0x47);
        assert!(!result.transport_error_indicator);
        assert!(result.payload_unit_start_indicator);
        assert!(result.transport_priority);
        assert_eq!(result.pid, 1);
        assert_eq!(result.transport_scrambling_control, 0);
        assert_eq!(result.continuity_counter, 1);
    }

    #[test]
    fn test_af_only() {
        let mut packet = vec![0xFF; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0010_0001; // AFC=10, CC=1
        packet[4] = 183; // AFL=183
        packet[5] = 0b0000_0000; // All Af flags=false

        let ts_packet = Packet::parse(&packet);
        assert!(ts_packet.is_ok());
        assert_eq!(ts_packet.unwrap().payload, vec![]);
    }

    #[test]
    fn test_payload_only() {
        let mut packet = vec![0xFF; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0001_0001; // AFC=01, CC=1

        let ts_packet = Packet::parse(&packet);
        assert!(ts_packet.is_ok());
        assert_eq!(ts_packet.unwrap().payload, vec![0xFF; 184]);
    }

    #[test]
    fn test_af_and_payload() {
        let mut packet = vec![0xFF; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0011_0001; // AFC=11, CC=1
        packet[4] = 10; // AFL=10
        packet[5] = 0b0000_0000; // All Af flags=false

        let ts_packet = Packet::parse(&packet);
        assert!(ts_packet.is_ok());
        assert_eq!(ts_packet.unwrap().payload, vec![0xFF; 173]);
    }

    #[test]
    fn test_invalid_sync_byte() {
        let mut packet = vec![0; 188];
        packet[0] = 0x00;
        let header = Header::parse(&packet);
        assert!(header.is_err());
        assert_eq!(header.unwrap_err(), Error::InvalidSyncByte(0x00));
    }

    #[test]
    fn test_invalid_af_length() {
        let mut packet = vec![0; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0011_0001; // TSC=00, AFC=11, CC=1
        packet[4] = 184; // AFL=184

        let ts_packet = Packet::parse(&packet);
        assert!(ts_packet.is_err());
        assert_eq!(
            ts_packet.unwrap_err(),
            Error::BufferTooShort {
                expected: 185,
                actual: 184
            }
        );
    }

    #[test]
    fn test_mux_payload_only() {
        let mut packet = vec![0xAB; 188];
        packet[0] = 0x47;
        packet[1] = 0b0100_0000; // PUSI=1
        packet[2] = 0x01; // PID=1
        packet[3] = 0b0001_0101; // AFC=01 (payload only), CC=5

        let ts_packet = Packet::parse(&packet).unwrap();
        assert!(ts_packet.adaptation_field.is_none());

        let mut reconstructed = Vec::new();
        reconstructed.extend_from_slice(&ts_packet.header.to_bytes());
        reconstructed.extend_from_slice(&ts_packet.payload);
        assert_eq!(reconstructed, packet);
    }

    #[test]
    fn test_mux_with_af_and_payload() {
        let mut packet = vec![0xFF; 188];
        packet[0] = 0x47;
        packet[1] = 0b0110_0000; // TEI=0, PUSI=1, TP=1
        packet[2] = 0b0000_0001; // PID=1
        packet[3] = 0b0011_0001; // AFC=11, CC=1
        packet[4] = 10; // AFL=10
        packet[5] = 0b0000_0000; // All AF flags=false

        let ts_packet = Packet::parse(&packet).unwrap();
        let af = ts_packet.adaptation_field.as_ref().unwrap();

        assert_eq!(&ts_packet.header.to_bytes()[..], &packet[0..4]);
        assert_eq!(&af.to_bytes()[..], &packet[4..15]); // 1(AFL) + 1(flags) + 9(stuffing)

        let mut reconstructed = Vec::new();
        reconstructed.extend_from_slice(&ts_packet.header.to_bytes());
        reconstructed.extend_from_slice(&af.to_bytes());
        reconstructed.extend_from_slice(&ts_packet.payload);
        assert_eq!(reconstructed, packet);
    }
}
