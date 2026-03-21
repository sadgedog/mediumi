//! Mantissa Quantization and Decoding

use crate::ac3::error::Error;
use crate::ac3::frame::QuantizedMantissaValues;
use crate::util::bitstream::BitstreamReader;

// qntztab[bap]: mantissa bits per bap value (Table 7.18)
#[allow(dead_code)]
const QNTZTAB: [f32; 16] = [
    0.0, 1.67, 2.33, 3.0, 3.5, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0,
];

/// Grouped quantizer context (Section 7.3.5).
///
/// Groups are shared across exponent sets within a block:
///   bap=1 (3-level): 3 mantissa codes in 5 bits
///   bap=2 (5-level): 3 mantissa codes in 7 bits
///   bap=4 (11-level): 2 mantissa codes in 7 bits
struct GroupedQuantizer {
    q1_codes: [i32; 3],
    q1_idx: usize, // 3 = need new group
    q2_codes: [i32; 3],
    q2_idx: usize,
    q4_codes: [i32; 2],
    q4_idx: usize, // 2 = need new group
}

impl GroupedQuantizer {
    fn new() -> Self {
        Self {
            q1_codes: [0; 3],
            q1_idx: 3,
            q2_codes: [0; 3],
            q2_idx: 3,
            q4_codes: [0; 2],
            q4_idx: 2,
        }
    }

    /// Unpack a single mantissa from the bitstream based on bap value.
    fn unpack(&mut self, reader: &mut BitstreamReader, bap: u8) -> Result<i32, Error> {
        match bap {
            // bap=0: no bits allocated
            0 => Ok(0),

            // bap=1: 3-level, 3 values in 5 bits
            // Decoder:
            //   mantissa_code[a] = truncate(group_code / 9)
            //   mantissa_code[b] = truncate((group_code % 9) / 3)
            //   mantissa_code[c] = (group_code % 9) % 3
            // code ∈ {0,1,2} → value = code - 1 ∈ {-1, 0, 1}
            1 => {
                if self.q1_idx >= 3 {
                    let gc = reader.read_bits(5)? as i32;
                    self.q1_codes[0] = gc / 9 - 1;
                    self.q1_codes[1] = (gc % 9) / 3 - 1;
                    self.q1_codes[2] = (gc % 3) - 1;
                    self.q1_idx = 0;
                }
                let val = self.q1_codes[self.q1_idx];
                self.q1_idx += 1;
                Ok(val)
            }

            // bap=2: 5-level, 3 values in 7 bits
            // Decoder:
            //   mantissa_code[a] = truncate(group_code / 25)
            //   mantissa_code[b] = truncate((group_code % 25) / 5)
            //   mantissa_code[c] = (group_code % 25) % 5
            // code ∈ {0..4} → value = code - 2 ∈ {-2..2}
            2 => {
                if self.q2_idx >= 3 {
                    let gc = reader.read_bits(7)? as i32;
                    self.q2_codes[0] = gc / 25 - 2;
                    self.q2_codes[1] = (gc % 25) / 5 - 2;
                    self.q2_codes[2] = (gc % 5) - 2;
                    self.q2_idx = 0;
                }
                let val = self.q2_codes[self.q2_idx];
                self.q2_idx += 1;
                Ok(val)
            }

            // bap=3: 7-level, 3 bits
            // code ∈ {0..6} → value = code - 3 ∈ {-3..3}
            3 => {
                let code = reader.read_bits(3)? as i32;
                Ok(code - 3)
            }

            // bap=4: 11-level, 2 values in 7 bits
            // Decoder:
            //   mantissa_code[a] = truncate(group_code / 11)
            //   mantissa_code[b] = group_code % 11
            // code ∈ {0..10} → value = code - 5 ∈ {-5..5}
            4 => {
                if self.q4_idx >= 2 {
                    let gc = reader.read_bits(7)? as i32;
                    self.q4_codes[0] = gc / 11 - 5;
                    self.q4_codes[1] = (gc % 11) - 5;
                    self.q4_idx = 0;
                }
                let val = self.q4_codes[self.q4_idx];
                self.q4_idx += 1;
                Ok(val)
            }

            // bap=5: 15-level, 4 bits
            // code ∈ {0..14} → value = code - 7 ∈ {-7..7}
            5 => {
                let code = reader.read_bits(4)? as i32;
                Ok(code - 7)
            }

            // bap=6..=15: asymmetric quantization
            // qntztab[bap] bits, two's complement fractional
            // value = code - 2^(qntztab[bap] - 1)
            6..=15 => {
                let nbits = QNTZTAB[bap as usize] as u8;
                let code = reader.read_bits(nbits)? as i32;
                Ok(code - (1 << (nbits - 1)))
            }

            _ => Err(Error::InvalidState("invalid bap")),
        }
    }
}

/// BAP arrays and mantissa counts for all channels in a block.
pub struct MantissaParams<'a> {
    /// Per-channel BAP arrays and mantissa counts: (bap, nchmant)
    pub channels: &'a [(Vec<u8>, usize)],
    /// Which channels are in coupling
    pub chincpl: &'a [bool],
    /// Coupling channel BAP and mantissa count
    pub coupling: Option<(&'a [u8], usize)>,
    /// LFE channel BAP (nlfemant is always 7)
    pub lfe: Option<&'a [u8]>,
}

/// Parse quantized mantissa values from the bitstream (Section 7.3).
///
/// Reading order (A/52 Section 5.4.3):
///   for each fbw channel:
///     read ch mantissas for bins 0..nchmant[ch]
///     if ch is first coupled channel:
///       read coupling mantissas for bins 0..ncplmant
///   read LFE mantissas (7 bins)
///
/// Grouped quantizers (bap=1,2,4) share state across all channels
/// and coupling within the same block (Section 7.3.5).
pub fn parse_mantissas(
    reader: &mut BitstreamReader,
    params: &MantissaParams,
) -> Result<QuantizedMantissaValues, Error> {
    let mut gq = GroupedQuantizer::new();
    let mut got_cplchan = false;

    let mut chmant = Vec::with_capacity(params.channels.len());
    let mut cplmant = None;

    for (ch, (bap, nchmant)) in params.channels.iter().enumerate() {
        let mut mant = Vec::with_capacity(*nchmant);
        for &b in &bap[..*nchmant] {
            mant.push(gq.unpack(reader, b)?);
        }
        chmant.push(mant);

        // Coupling mantissas after first coupled channel
        if params.chincpl[ch] && !got_cplchan {
            let (cpl_bap, ncplmant) = params
                .coupling
                .ok_or(Error::InvalidState("cpl_bap missing"))?;
            let mut mant = Vec::with_capacity(ncplmant);
            for &b in &cpl_bap[..ncplmant] {
                mant.push(gq.unpack(reader, b)?);
            }
            cplmant = Some(mant);
            got_cplchan = true;
        }
    }

    // LFE mantissas
    let lfemant = if let Some(lfe_bap) = params.lfe {
        let nlfemant = 7;
        let mut mant = Vec::with_capacity(nlfemant);
        for &b in &lfe_bap[..nlfemant] {
            mant.push(gq.unpack(reader, b)?);
        }
        Some(mant)
    } else {
        None
    };

    Ok(QuantizedMantissaValues {
        chmant,
        cplmant,
        lfemant,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bitstream::BitstreamWriter;

    fn make_reader(bits: &[(u32, u8)]) -> Vec<u8> {
        let mut w = BitstreamWriter::new();
        for &(val, n) in bits {
            w.write_bits(val, n);
        }
        w.finish()
    }

    // bap=0: no bits
    #[test]
    fn test_bap0() {
        let data = make_reader(&[]);
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 0).unwrap(), 0);
    }

    // bap=1: 3-level, 3 values in 5 bits
    #[test]
    fn test_bap1_group() {
        // values (0, 1, -1) → codes (1, 2, 0) → group = 1*9 + 2*3 + 0 = 15
        let data = make_reader(&[(15, 5)]); // 01111
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 1);
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), -1);
    }

    #[test]
    fn test_bap1_two_groups() {
        // group1: (1,1,1) → codes (2,2,2) → 2*9+2*3+2 = 26
        // group2: (-1,-1,-1) → codes (0,0,0) → 0
        let data = make_reader(&[(26, 5), (0, 5)]); // 11010 00000
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        for _ in 0..3 {
            assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 1);
        }
        for _ in 0..3 {
            assert_eq!(gq.unpack(&mut reader, 1).unwrap(), -1);
        }
    }

    // bap=2: 5-level, 3 values in 7 bits
    #[test]
    fn test_bap2_group() {
        // values (0, 2, -2) → codes (2, 4, 0) → 2*25 + 4*5 + 0 = 70
        let data = make_reader(&[(70, 7)]); // 1000110
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), 2);
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), -2);
    }

    // bap=3: 7-level, 3 bits
    #[test]
    fn test_bap3() {
        let data = make_reader(&[(0, 3), (3, 3), (6, 3)]); // 000 011 110
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 3).unwrap(), -3);
        assert_eq!(gq.unpack(&mut reader, 3).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 3).unwrap(), 3);
    }

    // bap=4: 11-level, 2 values in 7 bits
    #[test]
    fn test_bap4_group() {
        // values (3, -5) → codes (8, 0) → 8*11 + 0 = 88
        let data = make_reader(&[(88, 7)]); // 1011000
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 4).unwrap(), 3);
        assert_eq!(gq.unpack(&mut reader, 4).unwrap(), -5);
    }

    // bap=5: 15-level, 4 bits
    #[test]
    fn test_bap5() {
        let data = make_reader(&[(0, 4), (7, 4), (14, 4)]); // 0000 0111 1110
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 5).unwrap(), -7);
        assert_eq!(gq.unpack(&mut reader, 5).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 5).unwrap(), 7);
    }

    // bap=6: asymmetric, 5 bits
    #[test]
    fn test_bap6() {
        let data = make_reader(&[(0, 5), (16, 5), (31, 5)]); // 00000 10000 11111
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 6).unwrap(), -16);
        assert_eq!(gq.unpack(&mut reader, 6).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 6).unwrap(), 15);
    }

    // bap=15: asymmetric, 16 bits
    #[test]
    fn test_bap15() {
        let data = make_reader(&[(0, 16), (32768, 16)]); // 0x0000 0x8000
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        assert_eq!(gq.unpack(&mut reader, 15).unwrap(), -32768);
        assert_eq!(gq.unpack(&mut reader, 15).unwrap(), 0);
    }

    // Group sharing across bap types
    #[test]
    fn test_mixed_grouped_quantizers() {
        // bap=1 group: (0,0,0) → codes (1,1,1) → 1*9+1*3+1 = 13
        // bap=2 group: (0,0,0) → codes (2,2,2) → 2*25+2*5+2 = 62
        let data = make_reader(&[(13, 5), (62, 7)]); // 01101 0111110
        let mut reader = BitstreamReader::new(&data);
        let mut gq = GroupedQuantizer::new();
        // consume 2 of 3 from bap=1 group
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 0);
        // consume 1 of 3 from bap=2 group
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), 0);
        // bap=1: 1 remaining in group
        assert_eq!(gq.unpack(&mut reader, 1).unwrap(), 0);
        // bap=2: 2 remaining in group
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), 0);
        assert_eq!(gq.unpack(&mut reader, 2).unwrap(), 0);
    }

    // parse_mantissas
    #[test]
    fn test_parse_mantissas_simple() {
        // bap=[3, 3, 5]: code=4→1, code=2→-1, code=10→3
        let data = make_reader(&[(4, 3), (2, 3), (10, 4)]); // 100 010 1010
        let mut reader = BitstreamReader::new(&data);
        let channels = vec![(vec![3, 3, 5], 3)];
        let params = MantissaParams {
            channels: &channels,
            chincpl: &[false],
            coupling: None,
            lfe: None,
        };
        let result = parse_mantissas(&mut reader, &params).unwrap();
        assert_eq!(result.chmant, vec![vec![1, -1, 3]]);
        assert!(result.cplmant.is_none());
        assert!(result.lfemant.is_none());
    }

    #[test]
    fn test_parse_mantissas_all_bap0() {
        let data = make_reader(&[]);
        let mut reader = BitstreamReader::new(&data);
        let channels = vec![(vec![0, 0, 0, 0], 4)];
        let params = MantissaParams {
            channels: &channels,
            chincpl: &[false],
            coupling: None,
            lfe: None,
        };
        let result = parse_mantissas(&mut reader, &params).unwrap();
        assert_eq!(result.chmant, vec![vec![0, 0, 0, 0]]);
    }
}
