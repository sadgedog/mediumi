//! Bit-level reader and writer for codec bitstream parsing

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
}
