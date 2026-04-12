//! SEI (Supplemental enhancement information)

use crate::{
    error::Error,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug)]
pub struct SeiMessage {
    pub payload_type: u32,
    pub payload_size: u32,
    pub payload: Vec<u8>,
}

impl SeiMessage {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        // payload_type:
        let mut t = self.payload_type;
        while t >= 255 {
            writer.write_bits(0xFF, 8);
            t -= 255;
        }
        writer.write_bits(t, 8);

        // payload_size
        let mut s = self.payload_size;
        while s >= 255 {
            writer.write_bits(0xFF, 8);
            s -= 255;
        }
        writer.write_bits(s, 8);

        // payload bytes
        for &byte in &self.payload {
            writer.write_bits(byte as u32, 8);
        }
    }

    pub fn parse(reader: &mut BitstreamReader) -> Result<Self, Error> {
        // payload_type
        let mut payload_type: u32 = 0;
        loop {
            let byte = reader.read_bits(8)?;
            if byte == 0xFF {
                payload_type += 255;
            } else {
                payload_type += byte;
                break;
            }
        }

        // payload_size
        let mut payload_size: u32 = 0;
        loop {
            let byte = reader.read_bits(8)?;
            if byte == 0xFF {
                payload_size += 255;
            } else {
                payload_size += byte;
                break;
            }
        }

        let size = payload_size as usize;
        let mut payload = Vec::with_capacity(size);
        for _ in 0..size {
            payload.push(reader.read_bits(8)? as u8);
        }

        Ok(Self {
            payload_type,
            payload_size,
            payload,
        })
    }
}

#[derive(Debug)]
pub struct Sei {
    pub sei_message: Vec<SeiMessage>,
}

impl Sei {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        for msg in &self.sei_message {
            msg.to_bytes(&mut writer);
        }
        // rbsp_trailing_bits: stop bit + alignment zeros
        writer.write_bits(1, 1);
        writer.finish()
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let mut sei_message = Vec::new();

        while reader.has_more_rbsp_data() {
            sei_message.push(SeiMessage::parse(&mut reader)?);
        }

        Ok(Self { sei_message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_sei_rbsp(messages: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let sei = Sei {
            sei_message: messages
                .iter()
                .map(|(t, p)| SeiMessage {
                    payload_type: *t,
                    payload_size: p.len() as u32,
                    payload: p.clone(),
                })
                .collect(),
        };
        sei.to_bytes()
    }

    #[test]
    fn test_roundtrip() {
        let input = build_sei_rbsp(&[
            (0, vec![0xAA]),
            (1, vec![0xBB, 0xCC]),
            (5, vec![0xDD, 0xEE, 0xFF]),
        ]);
        let parsed = Sei::parse(&input).expect("failed to parse");
        assert_eq!(parsed.sei_message.len(), 3);
        assert_eq!(parsed.sei_message[0].payload_type, 0);
        assert_eq!(parsed.sei_message[1].payload_type, 1);
        assert_eq!(parsed.sei_message[2].payload_type, 5);
        assert_eq!(parsed.to_bytes(), input);
    }
}
