//! Byte-oriented reader and writer for mp4 box (de)serialization.
//!
//! mp4 boxes are overwhelmingly byte-aligned, so the primary API works in whole
//! big-endian integers and byte slices (`read_u32` / `write_bytes` / …), backed
//! by a plain cursor and `Vec<u8>` — no per-bit work.
//!
//! A few boxes pack sub-byte fields (e.g. `mdhd` language, `sdtp`, `avcC`). For
//! those, [`ByteReader::read_bits`] / [`ByteWriter::write_bits`] keep the legacy
//! bit-level behaviour. They take a byte-aligned fast path whenever the request
//! is a whole number of bytes on a byte boundary, so the common case never pays
//! for the bit loop.

use crate::util::error::Error;

pub struct ByteReader<'a> {
    data: &'a [u8],
    byte_offset: usize,
    bit_offset: u8,
}

impl<'a> ByteReader<'a> {
    /// Create a new reader from a byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_offset: 0,
            bit_offset: 0,
        }
    }

    pub fn remaining_bits(&self) -> usize {
        (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize
    }

    /// Read `n` bits and return as a u32 (MSB first).
    ///
    /// Byte-aligned whole-byte requests (the common case) read directly from the
    /// slice; only genuinely sub-byte fields fall through to the bit loop.
    pub fn read_bits(&mut self, n: u8) -> Result<u32, Error> {
        let remaining = (self.data.len() - self.byte_offset) * 8 - self.bit_offset as usize;
        if (n as usize) > remaining {
            return Err(Error::DataTooShort(n as usize, remaining));
        }

        // Fast path: aligned, whole-byte read.
        if self.bit_offset == 0 && n.is_multiple_of(8) {
            let mut value: u32 = 0;
            for _ in 0..(n / 8) {
                value = (value << 8) | self.data[self.byte_offset] as u32;
                self.byte_offset += 1;
            }
            return Ok(value);
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

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        Ok(self.read_bits(8)? as u8)
    }

    pub fn read_u16(&mut self) -> Result<u16, Error> {
        Ok(self.read_bits(16)? as u16)
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        self.read_bits(32)
    }

    pub fn read_u64(&mut self) -> Result<u64, Error> {
        let hi = self.read_u32()? as u64;
        let lo = self.read_u32()? as u64;
        Ok((hi << 32) | lo)
    }

    /// Read all remaining bytes, returning the data and the current bit offset.
    /// If `bit_offset != 0`, the upper bits of the first byte are masked with 0.
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

    /// Borrow the next `n` bytes from the underlying slice without copying and
    /// advance the cursor. Requires byte alignment (`bit_offset == 0`).
    pub fn read_slice(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.bit_offset != 0 {
            return Err(Error::DataTooShort(n * 8, self.remaining_bits()));
        }
        let end = self.byte_offset + n;
        if end > self.data.len() {
            return Err(Error::DataTooShort(n * 8, self.remaining_bits()));
        }
        let s = &self.data[self.byte_offset..end];
        self.byte_offset = end;
        Ok(s)
    }
}

pub struct ByteWriter {
    data: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
}

impl ByteWriter {
    /// Create a new empty writer
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            current_byte: 0,
            bit_offset: 0,
        }
    }

    /// Create a writer with a pre-reserved capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
            current_byte: 0,
            bit_offset: 0,
        }
    }

    /// Append raw bytes. Requires byte alignment (`bit_offset == 0`).
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.bit_offset, 0, "write_bytes on a non-byte boundary");
        self.data.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, v: u8) {
        debug_assert_eq!(self.bit_offset, 0);
        self.data.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_be_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_be_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_be_bytes());
    }

    /// Current byte length written so far. Valid only on a byte boundary.
    pub fn position(&self) -> usize {
        debug_assert_eq!(self.bit_offset, 0);
        self.data.len()
    }

    /// Overwrite the 4 big-endian bytes at `offset` (used to backpatch a box
    /// size field that was written as a placeholder before the body length was
    /// known).
    pub fn patch_u32(&mut self, offset: usize, value: u32) {
        self.data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    /// Insert `bytes` at `offset`, shifting the tail right. Used only for the
    /// rare promotion of a box header to a 64-bit largesize.
    pub fn insert_bytes(&mut self, offset: usize, bytes: &[u8]) {
        self.data.splice(offset..offset, bytes.iter().copied());
    }

    /// Write `n` bits from a u32 value (MSB first).
    ///
    /// Byte-aligned whole-byte writes (the common case) push whole bytes; only
    /// genuinely sub-byte fields fall through to the bit loop.
    pub fn write_bits(&mut self, value: u32, n: u8) {
        // Fast path: aligned, whole-byte write.
        if self.bit_offset == 0 && n.is_multiple_of(8) {
            for i in (0..(n / 8)).rev() {
                self.data.push((value >> (i as u32 * 8)) as u8);
            }
            return;
        }

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

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bits_fast_and_slow_agree() {
        let data = &[0b1010_1010, 0b1111_1111, 0x12, 0x34];
        // Aligned whole-byte read (fast path).
        let mut r = ByteReader::new(data);
        assert_eq!(r.read_bits(8).unwrap(), 0b1010_1010);
        assert_eq!(r.read_bits(16).unwrap(), 0xFF12);
        // Sub-byte read (slow path) reproduces the original bit behaviour.
        let mut r2 = ByteReader::new(data);
        assert_eq!(r2.read_bits(4).unwrap(), 0b1010);
        assert_eq!(r2.read_bits(5).unwrap(), 0b10101);
        assert_eq!(r2.bit_offset, 1);
        assert_eq!(r2.byte_offset, 1);
    }

    #[test]
    fn read_u_helpers() {
        let data = &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
        let mut r = ByteReader::new(data);
        assert_eq!(r.read_u16().unwrap(), 0x1234);
        assert_eq!(r.read_u8().unwrap(), 0x56);
        let mut r2 = ByteReader::new(data);
        assert_eq!(r2.read_u64().unwrap(), 0x1234_5678_9abc_def0);
    }

    #[test]
    fn read_slice_zero_copy() {
        let data: &[u8] = &[0x11, 0x22, 0x33, 0x44, 0x55];
        let mut r = ByteReader::new(data);
        let s = r.read_slice(2).unwrap();
        assert_eq!(s, &[0x11, 0x22]);
        assert_eq!(s.as_ptr(), data.as_ptr());
        assert_eq!(r.read_slice(3).unwrap(), &[0x33, 0x44, 0x55]);
    }

    #[test]
    fn read_slice_unaligned_errors() {
        let data: &[u8] = &[0xFF, 0xFF];
        let mut r = ByteReader::new(data);
        r.read_bits(3).unwrap();
        assert!(r.read_slice(1).is_err());
    }

    #[test]
    fn write_bits_fast_path_matches_bit_loop() {
        // Whole-byte writes via the fast path must equal the MSB-first bit loop.
        let mut w = ByteWriter::new();
        w.write_bits(0x0102_0304, 32);
        w.write_bits(0xABCD, 16);
        w.write_bits(0x7F, 8);
        assert_eq!(w.finish(), vec![0x01, 0x02, 0x03, 0x04, 0xAB, 0xCD, 0x7F]);
    }

    #[test]
    fn write_bits_sub_byte() {
        let mut w = ByteWriter::new();
        w.write_bits(0b0011_1111, 6);
        w.write_bits(0b0000_1111, 4);
        assert_eq!(w.finish(), vec![0b1111_1111, 0b1100_0000]);
    }

    #[test]
    fn write_u_helpers_are_big_endian() {
        let mut w = ByteWriter::new();
        w.write_u8(0x12);
        w.write_u16(0x3456);
        w.write_u32(0x789a_bcde);
        w.write_bytes(&[0xAA, 0xBB]);
        assert_eq!(
            w.finish(),
            vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xAA, 0xBB]
        );
    }

    #[test]
    fn mixed_sub_byte_then_aligned() {
        // sdtp-style: four 2-bit fields complete a byte, then aligned writes.
        let mut w = ByteWriter::new();
        w.write_bits(0b01, 2);
        w.write_bits(0b10, 2);
        w.write_bits(0b11, 2);
        w.write_bits(0b00, 2);
        w.write_u16(0xBEEF);
        assert_eq!(w.finish(), vec![0b0110_1100, 0xBE, 0xEF]);
    }
}
