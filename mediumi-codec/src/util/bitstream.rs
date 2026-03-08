//! Bit-level reader and writer for codec bitstream parsing
//!
//! Provides bit-granularity read/write operations and Exp-Golomb coding (ue(v), se(v))

use crate::util::error::Error;

pub struct BitstreamReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8,
}

impl<'a> BitstreamReader<'a> {
    /// Create a new reader from a byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    /// Read `n` bits and return as a u32 (MSB first)
    pub fn read_bits(&mut self, n: u8) -> Result<u32, Error> {
        let remaining = (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize;
        if (n as usize) > remaining {
            return Err(Error::DataTooShort(n as usize, remaining));
        }

        let mut value: u32 = 0;
        for _ in 0..n {
            let bit = (self.data[self.byte_offset] >> (7 - self.bit_offset)) & 1;
            value = (value << 1) | bit as u32;
            self.bit_offset += 1;
            if self.bit_offset == 8 {
                self.bit_offset = 0;
                self.byte_offset += 1;
            }
        }
        Ok(value)
    }

    /// Read a single bit as a boolean
    pub fn read_bit(&mut self) -> Result<bool, Error> {
        Ok(self.read_bits(1)? == 1)
    }

    /// Read an unsigned Exp-Golomb coded value: ue(v)
    /// Exp-Golomb: ue(v)
    /// e.g. value = 3
    ///      code = value + 1 = 4 (0b100)
    ///      bits_length = 3
    ///      leading_zeros = bits_length - 1 = 2
    ///      0b00_100
    pub fn read_ue(&mut self) -> Result<u32, Error> {
        let mut leading_zeros: u32 = 0;
        loop {
            let bit = self.read_bits(1)?;
            if bit == 1 {
                break;
            }
            leading_zeros += 1;
        }
        if leading_zeros == 0 {
            return Ok(0);
        }
        let remaining = self.read_bits(leading_zeros as u8)?;
        Ok((1 << leading_zeros) - 1 + remaining)
    }

    /// Read a signed Exp-Golomb coded value: se(v)
    /// Exp-Golomb: se(v)
    /// e.g. ue=3
    ///      (3+1)/2 = 2 (odd)
    ///      ue=4
    ///      -(4/2) = -2 (even)
    pub fn read_se(&mut self) -> Result<i32, Error> {
        let code = self.read_ue()?;
        if code % 2 == 0 {
            Ok(-(code as i32 / 2))
        } else {
            Ok((code as i32 + 1) / 2)
        }
    }

    /// Read all remaining bytes, returning the data and the current bit offset
    /// If bit_offset != 0, the upper bits are masked with 0
    /// e.g. bit_offset = 2, data = 0b11111111
    ///      return (0b00111111, 2)
    pub fn read_remaining_bytes(&mut self) -> (Vec<u8>, u8) {
        let mut remaining = Vec::new();

        if self.bit_offset != 0 {
            let bits_left = 8 - self.bit_offset;
            let mask = (1u8 << bits_left) - 1;
            let partial = self.data[self.byte_offset] & mask;
            remaining.push(partial);
            self.byte_offset += 1;
        }

        remaining.extend_from_slice(&self.data[self.byte_offset..]);
        let bit_offset = self.bit_offset;
        self.byte_offset = self.data.len();
        self.bit_offset = 0;

        (remaining, bit_offset)
    }

    pub fn remaining_bits(&self) -> usize {
        (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
    }

    pub fn has_more_rbsp_data(&self) -> bool {
        let remaining = self.remaining_bits();
        if remaining == 0 {
            return false;
        }
        // check trailing_bits(e.g. 1000_0000...)
        let last_byte = self.data[self.data.len() - 1];
        if last_byte == 0 {
            return true;
        }
        let trailing_zeros = last_byte.trailing_zeros() as usize;
        let stop_bit_pos = (self.data.len() - 1) * 8 + (7 - trailing_zeros);
        let current_pos = self.byte_offset * 8 + self.bit_offset as usize;

        stop_bit_pos > current_pos
    }
}

pub struct BitstreamWriter {
    data: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
}

impl BitstreamWriter {
    /// Create a new empty writer
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            current_byte: 0,
            bit_offset: 0,
        }
    }

    /// Write `n` bits from a u32 value (MSB first)
    pub fn write_bits(&mut self, value: u32, n: u8) {
        for i in (0..n).rev() {
            let bit = ((value >> i) & 1) as u8;
            self.current_byte |= bit << (7 - self.bit_offset);
            self.bit_offset += 1;
            if self.bit_offset == 8 {
                self.data.push(self.current_byte);
                self.current_byte = 0;
                self.bit_offset = 0;
            }
        }
    }

    /// Write an unsigned Exp-Golomb coded value: ue(v)
    pub fn write_ue(&mut self, value: u32) {
        let code = value + 1;
        let bits = 32 - code.leading_zeros(); // bit length of code 
        let leading_zeros = bits - 1;

        // add 0 (number of leading_zeros)
        self.write_bits(0, leading_zeros as u8);
        // add code
        self.write_bits(code, bits as u8);
    }

    /// Write a signed Exp-Golomb coded value: se(v)
    pub fn write_se(&mut self, value: i32) {
        let code = if value <= 0 {
            (-value * 2) as u32
        } else {
            (value * 2 - 1) as u32
        };
        self.write_ue(code);
    }

    /// Write a single bit from a boolean
    pub fn write_bool(&mut self, value: bool) {
        self.write_bits(value as u32, 1);
    }

    /// Flush any remaining bits and return the completed byte buffer
    pub fn finish(mut self) -> Vec<u8> {
        if self.bit_offset > 0 {
            self.data.push(self.current_byte);
        }
        self.data
    }

    /// Write raw bytes previously captured by `BitstreamReader::read_remaining_bytes`
    pub fn write_remaining_bytes(&mut self, data: &[u8], bit_offset: u8) {
        if data.is_empty() {
            return;
        }
        if bit_offset > 0 {
            let bits_left = 8 - bit_offset;
            let mask = (1u8 << bits_left) - 1;
            self.write_bits((data[0] & mask) as u32, bits_left);
            for &byte in &data[1..] {
                self.write_bits(byte as u32, 8);
            }
        } else {
            for &byte in data {
                self.write_bits(byte as u32, 8);
            }
        }
    }
}

impl Default for BitstreamWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bits() {
        let data = &[0b10101010, 0b11111111];
        let mut reader = BitstreamReader::new(data);
        let r_1 = reader.read_bits(4).unwrap();
        assert_eq!(r_1, 0b1010);
        assert_eq!(reader.bit_offset, 4);
        assert_eq!(reader.byte_offset, 0);

        let r_2 = reader.read_bits(5).unwrap();
        assert_eq!(r_2, 0b10101);
        assert_eq!(reader.bit_offset, 1);
        assert_eq!(reader.byte_offset, 1);
    }

    #[test]
    fn test_read_bit() {
        let data = &[0b10101010, 0b11111111];
        let mut reader = BitstreamReader::new(data);
        let r_1 = reader.read_bit().unwrap();
        assert!(r_1);
        assert_eq!(reader.bit_offset, 1);
        assert_eq!(reader.byte_offset, 0);

        let r_2 = reader.read_bit().unwrap();
        assert!(!r_2);
        assert_eq!(reader.bit_offset, 2);
        assert_eq!(reader.byte_offset, 0);

        for _ in 0..7 {
            let _ = reader.read_bit().unwrap();
        }
        assert_eq!(reader.bit_offset, 1);
        assert_eq!(reader.byte_offset, 1);
    }

    #[test]
    fn test_read_ue() {
        let data = &[0b10101010, 0b11111111];
        let mut reader = BitstreamReader::new(data);
        assert_eq!(reader.read_ue().unwrap(), 0); // 1
        assert_eq!(reader.read_ue().unwrap(), 1); // 010
        assert_eq!(reader.read_ue().unwrap(), 0); // 1
        assert_eq!(reader.read_ue().unwrap(), 1); // 010
    }

    #[test]
    fn test_read_se() {
        let data = &[0b01100100, 0b00101000];
        let mut reader = BitstreamReader::new(data);
        assert_eq!(reader.read_se().unwrap(), -1); // 011 → ue=2 → se=-1
        assert_eq!(reader.read_se().unwrap(), 2); // 00100 → ue=3 → se=2
        assert_eq!(reader.read_se().unwrap(), -2); // 00101 → ue=4 → se=-2
    }

    #[test]
    fn test_write_bits() {
        let mut writer = BitstreamWriter::new();
        writer.write_bits(0b0011_1111, 6);
        writer.write_bits(0b0000_1111, 4);
        let bytes = writer.finish();
        assert_eq!(bytes, vec![0b1111_1111, 0b1100_0000]);
    }

    #[test]
    fn test_write_bit() {
        let mut writer = BitstreamWriter::new();
        writer.write_bool(true);
        let bytes = writer.finish();
        assert_eq!(bytes, vec![0b1000_0000]);
    }

    #[test]
    fn test_write_ue() {
        let mut writer = BitstreamWriter::new();
        writer.write_ue(0); // 1                                                                         
        writer.write_ue(1); // 010                                                                       
        writer.write_ue(2); // 011                                                                       
        writer.write_ue(3); // 00100
        let bytes = writer.finish();
        // 1010_0110 0100_0000
        assert_eq!(bytes, vec![0b1010_0110, 0b0100_0000]);
    }

    #[test]
    fn test_write_se() {
        let mut writer = BitstreamWriter::new();
        writer.write_se(0); // ue=0 → 1
        writer.write_se(1); // ue=1 → 010
        writer.write_se(-1); // ue=2 → 011
        writer.write_se(2); // ue=3 → 00100
        writer.write_se(-2); // ue=4 → 00101
        let bytes = writer.finish();
        // 1010_0110 0100_0010 1000_0000
        assert_eq!(bytes, vec![0b1010_0110, 0b0100_0010, 0b1000_0000]);
    }

    #[test]
    fn test_ue_roundtrip() {
        let mut writer = BitstreamWriter::new();
        let values = [0xDE, 0xAD, 0xBE, 0xEF];
        for &v in &values {
            writer.write_ue(v);
        }
        let bytes = writer.finish();

        let mut reader = BitstreamReader::new(&bytes);
        for &v in &values {
            assert_eq!(reader.read_ue().unwrap(), v);
        }
    }

    #[test]
    fn test_se_roundtrip() {
        let mut writer = BitstreamWriter::new();
        let values = [0xDE, 0xAD, 0xBE, 0xEF];
        for &v in &values {
            writer.write_se(v);
        }
        let bytes = writer.finish();

        let mut reader = BitstreamReader::new(&bytes);
        for &v in &values {
            assert_eq!(reader.read_se().unwrap(), v);
        }
    }
}
