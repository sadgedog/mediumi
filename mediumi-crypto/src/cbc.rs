use aes::cipher::{BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Block};

use crate::cenc::Subsample;
use crate::error::Error;

/// AES-128 block cipher wrapper for cbcs pattern encryption.
pub struct Aes128CbcPatternCipher {
    cipher: Aes128,
}

impl Aes128CbcPatternCipher {
    pub fn new(key: &[u8; 16]) -> Self {
        Self {
            cipher: Aes128::new(key.into()),
        }
    }

    /// Encrypt one protected span in place
    pub fn apply_pattern(
        &self,
        constant_iv: &[u8; 16],
        crypt_byte_block: u8,
        skip_byte_block: u8,
        span: &mut [u8],
    ) {
        let full_blocks = (span.len() / 16) as u64;
        if full_blocks == 0 {
            return; // whole span is a sub-block remainder → clear
        }
        // 0:0 → encrypt every full block as one continuous CBC run.
        let (crypt, cycle) = if crypt_byte_block == 0 {
            (full_blocks, full_blocks)
        } else {
            (
                crypt_byte_block as u64,
                crypt_byte_block as u64 + skip_byte_block as u64,
            )
        };

        let mut prev = *constant_iv; // chaining input; only updated on crypt blocks
        for b in 0..full_blocks {
            if b % cycle < crypt {
                let off = (b as usize) * 16;
                let mut block = Block::default();
                block.copy_from_slice(&span[off..off + 16]);
                for i in 0..16 {
                    block[i] ^= prev[i]; // CBC: plaintext XOR previous cipher block
                }
                self.cipher.encrypt_block(&mut block);
                span[off..off + 16].copy_from_slice(&block[..]);
                prev.copy_from_slice(&block[..]); // next crypt block's IV
            }
            // skip block: left in the clear, `prev` unchanged.
        }
    }
}

/// Apply `cbcs` encryption to `buf` in place.
pub fn apply_cbcs_subsamples(
    cipher: &Aes128CbcPatternCipher,
    constant_iv: &[u8; 16],
    crypt_byte_block: u8,
    skip_byte_block: u8,
    subsamples: &[Subsample],
    buf: &mut [u8],
) -> Result<(), Error> {
    if subsamples.is_empty() {
        cipher.apply_pattern(constant_iv, crypt_byte_block, skip_byte_block, buf);
        return Ok(());
    }

    let total: u64 = subsamples
        .iter()
        .map(|s| s.clear as u64 + s.encrypted as u64)
        .sum();
    if total != buf.len() as u64 {
        return Err(Error::SubsampleByteMismatch {
            expected: buf.len(),
            actual: total as usize,
        });
    }

    let mut offset = 0usize;
    for sub in subsamples {
        offset += sub.clear as usize;
        let enc = sub.encrypted as usize;
        if enc > 0 {
            cipher.apply_pattern(
                constant_iv,
                crypt_byte_block,
                skip_byte_block,
                &mut buf[offset..offset + enc],
            );
            offset += enc;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // NIST SP 800-38A F.2.1/F.2.2 — AES-128-CBC.
    const KEY: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];
    const IV: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f,
    ];
    const PT1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    const CT1: [u8; 16] = [
        0x76, 0x49, 0xab, 0xac, 0x81, 0x19, 0xb2, 0x46, 0xce, 0xe9, 0x8e, 0x9b, 0x12, 0xe9, 0x19,
        0x7d,
    ];
    const PT2: [u8; 16] = [
        0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac, 0x45, 0xaf, 0x8e,
        0x51,
    ];
    const CT2: [u8; 16] = [
        0x50, 0x86, 0xcb, 0x9b, 0x50, 0x72, 0x19, 0xee, 0x95, 0xdb, 0x11, 0x3a, 0x91, 0x76, 0x78,
        0xb2,
    ];
    const PT3: [u8; 16] = [
        0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11, 0xe5, 0xfb, 0xc1, 0x19, 0x1a, 0x0a, 0x52,
        0xef,
    ];
    const CT3: [u8; 16] = [
        0x73, 0xbe, 0xd6, 0xb8, 0xe3, 0xc1, 0x74, 0x3b, 0x71, 0x16, 0xe6, 0x9e, 0x22, 0x22, 0x95,
        0x16,
    ];
    const PT4: [u8; 16] = [
        0xf6, 0x9f, 0x24, 0x45, 0xdf, 0x4f, 0x9b, 0x17, 0xad, 0x2b, 0x41, 0x7b, 0xe6, 0x6c, 0x37,
        0x10,
    ];
    const CT4: [u8; 16] = [
        0x3f, 0xf1, 0xca, 0xa1, 0x68, 0x1f, 0xac, 0x09, 0x12, 0x0e, 0xca, 0x30, 0x75, 0x86, 0xe1,
        0xa7,
    ];

    fn concat(blocks: &[[u8; 16]]) -> Vec<u8> {
        blocks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    /// Encrypt one AES block (used to derive expected pattern-chaining values).
    fn aes_block(pt_xor_iv: [u8; 16]) -> [u8; 16] {
        let mut block = Block::default();
        block.copy_from_slice(&pt_xor_iv);
        Aes128::new((&KEY).into()).encrypt_block(&mut block);
        block.as_slice().try_into().unwrap()
    }

    fn xor(a: [u8; 16], b: [u8; 16]) -> [u8; 16] {
        std::array::from_fn(|i| a[i] ^ b[i])
    }

    #[test]
    fn full_cbc_matches_nist_vector() {
        // crypt=0 (0:0) → continuous CBC over all blocks. Must match NIST CBC.
        let mut buf = concat(&[PT1, PT2, PT3, PT4]);
        Aes128CbcPatternCipher::new(&KEY).apply_pattern(&IV, 0, 0, &mut buf);
        assert_eq!(buf, concat(&[CT1, CT2, CT3, CT4]));
    }

    #[test]
    fn pattern_1_9_encrypts_only_first_of_ten() {
        // 10 blocks, 1:9 → only block 0 is encrypted, blocks 1..10 stay clear.
        let plain = concat(&[PT1, PT2, PT3, PT4, PT1, PT2, PT3, PT4, PT1, PT2]);
        let mut buf = plain.clone();
        Aes128CbcPatternCipher::new(&KEY).apply_pattern(&IV, 1, 9, &mut buf);
        assert_eq!(&buf[0..16], &CT1); // block 0 encrypted (CBC from IV)
        assert_eq!(&buf[16..], &plain[16..]); // blocks 1..10 untouched
    }

    #[test]
    fn pattern_chaining_skips_skip_blocks() {
        // 1:1 over 4 blocks → crypt blocks 0 and 2, skip blocks 1 and 3.
        // block0 = CBC(IV, PT1) = CT1
        // block2 = CBC(prev=block0_cipher=CT1, PT3) — chain skips the skip block.
        let mut buf = concat(&[PT1, PT2, PT3, PT4]);
        Aes128CbcPatternCipher::new(&KEY).apply_pattern(&IV, 1, 1, &mut buf);

        let expected_block2 = aes_block(xor(PT3, CT1));
        assert_eq!(&buf[0..16], &CT1); // crypt
        assert_eq!(&buf[16..32], &PT2); // skip → clear
        assert_eq!(&buf[32..48], &expected_block2); // crypt, IV = block0 cipher
        assert_eq!(&buf[48..64], &PT4); // skip → clear
    }

    #[test]
    fn remainder_left_clear() {
        // 17 bytes: one full block (encrypted) + 1 trailing byte (clear).
        let mut buf = PT1.to_vec();
        buf.push(0xAB);
        Aes128CbcPatternCipher::new(&KEY).apply_pattern(&IV, 0, 0, &mut buf);
        assert_eq!(&buf[0..16], &CT1);
        assert_eq!(buf[16], 0xAB); // remainder untouched
    }

    #[test]
    fn sub_block_span_fully_clear() {
        // span shorter than one block → entirely clear.
        let mut buf = [0x11u8; 15];
        Aes128CbcPatternCipher::new(&KEY).apply_pattern(&IV, 0, 0, &mut buf);
        assert_eq!(buf, [0x11u8; 15]);
    }

    #[test]
    fn cbcs_subsamples_spans_restart_from_constant_iv() {
        // Two protected spans (each one block), separated by a clear span. Both
        // must restart CBC from the same constant IV → both encrypt PT1 → CT1.
        let mut buf = Vec::new();
        buf.extend_from_slice(&PT1); // protected span A
        buf.extend_from_slice(&[0xCC; 8]); // clear span
        buf.extend_from_slice(&PT1); // protected span B
        let subs = [
            Subsample {
                clear: 0,
                encrypted: 16,
            },
            Subsample {
                clear: 8,
                encrypted: 16,
            },
        ];
        let cipher = Aes128CbcPatternCipher::new(&KEY);
        apply_cbcs_subsamples(&cipher, &IV, 0, 0, &subs, &mut buf).unwrap();
        assert_eq!(&buf[0..16], &CT1); // span A
        assert_eq!(&buf[16..24], &[0xCC; 8]); // clear untouched
        assert_eq!(&buf[24..40], &CT1); // span B — same constant IV → same CT
    }

    #[test]
    fn empty_subsamples_is_full_sample() {
        let mut via_empty = concat(&[PT1, PT2]);
        let cipher = Aes128CbcPatternCipher::new(&KEY);
        apply_cbcs_subsamples(&cipher, &IV, 0, 0, &[], &mut via_empty).unwrap();

        let mut via_direct = concat(&[PT1, PT2]);
        cipher.apply_pattern(&IV, 0, 0, &mut via_direct);
        assert_eq!(via_empty, via_direct);
        assert_eq!(via_empty, concat(&[CT1, CT2]));
    }

    #[test]
    fn byte_mismatch_rejected_and_buf_untouched() {
        let plain = [0xAAu8; 10];
        let mut buf = plain;
        let subs = [Subsample {
            clear: 5,
            encrypted: 6,
        }]; // total 11 != 10
        let cipher = Aes128CbcPatternCipher::new(&KEY);
        let r = apply_cbcs_subsamples(&cipher, &IV, 1, 9, &subs, &mut buf);
        assert!(matches!(
            r,
            Err(Error::SubsampleByteMismatch {
                expected: 10,
                actual: 11
            })
        ));
        assert_eq!(buf, plain);
    }
}
