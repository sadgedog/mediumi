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

    pub fn skip_bits(&mut self, n: usize) -> usize {
        let remaining = self.remaining_bits();
        let actual = n.min(remaining);
        let new_pos = self.byte_offset * 8 + self.bit_offset as usize + actual;
        self.byte_offset = new_pos / 8;
        self.bit_offset = (new_pos % 8) as u8;
        actual
    }

    pub fn remaining_bits(&self) -> usize {
        (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
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
