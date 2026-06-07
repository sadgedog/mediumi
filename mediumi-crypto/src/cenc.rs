use crate::ctr::Aes128CtrCipher;
use crate::encrypter::Iv;
use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subsample {
    pub clear: u32,
    pub encrypted: u32,
}

/// Apply `cenc` encryption to `buf` in place.
pub fn apply_cenc_subsamples(
    key: &[u8; 16],
    iv: &[u8; 16],
    subsamples: &[Subsample],
    buf: &mut [u8],
) -> Result<(), Error> {
    // audio (= subsample is empty)
    if subsamples.is_empty() {
        Aes128CtrCipher::new(key, iv).apply_keystream(buf);
        return Ok(());
    }

    // video (subsample is not empty)
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

    let mut cipher = Aes128CtrCipher::new(key, iv);
    let mut offset = 0usize;
    for sub in subsamples {
        offset += sub.clear as usize;
        let enc = sub.encrypted as usize;
        if enc > 0 {
            cipher.apply_keystream(&mut buf[offset..offset + enc]);
            offset += enc;
        }
    }
    Ok(())
}

/// Derive the 16-byte AES-CTR initial counter block for a sample, per
///
/// - **8-byte IV**: high 8 bytes = `base + sample_index`, low 8 bytes = 0 (the
///   block counter, which the cipher advances per 16-byte block). `block_offset`
///   is ignored. senc stores the high 8 bytes.
/// - **16-byte IV**: the whole 16 bytes are treated as one 128-bit
///   number — `base + block_offset`, where `block_offset` is the cumulative
///   count of encrypted cipher blocks across previous samples. The low 8 bytes
///   act as the continuous counter; the high 8 bytes stay a fixed nonce unless
///   the low counter overflows (carries up). senc stores all 16 bytes.
pub fn derive_per_sample_iv(base_iv: &Iv, sample_index: u64, block_offset: u128) -> [u8; 16] {
    match base_iv {
        Iv::Bytes8(b) => {
            let base_hi = u64::from_be_bytes(*b);
            let per_sample = base_hi.wrapping_add(sample_index);
            let mut iv = [0u8; 16];
            iv[0..8].copy_from_slice(&per_sample.to_be_bytes());
            iv // low 8 bytes stay zero — the AES-CTR block counter
        }
        Iv::Bytes16(b) => {
            let base = u128::from_be_bytes(*b);
            base.wrapping_add(block_offset).to_be_bytes()
        }
    }
}

/// Number of 16-byte cipher blocks a sample's encrypted data consumes.
pub fn encrypted_block_count(subsamples: &[Subsample], sample_len: usize) -> u128 {
    let encrypted: u64 = if subsamples.is_empty() {
        sample_len as u64
    } else {
        subsamples.iter().map(|s| s.encrypted as u64).sum()
    };
    encrypted.div_ceil(16) as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_subsamples_encrypts_whole_buffer() {
        let key = [0x42u8; 16];
        let iv = [0x13u8; 16];
        let plain: [u8; 64] = std::array::from_fn(|i| i as u8);

        let mut via_empty = plain;
        apply_cenc_subsamples(&key, &iv, &[], &mut via_empty).unwrap();

        let mut via_direct = plain;
        Aes128CtrCipher::new(&key, &iv).apply_keystream(&mut via_direct);

        assert_eq!(via_empty, via_direct);
        assert_ne!(via_empty, plain);
    }

    #[test]
    fn clear_spans_untouched_keystream_continuous() {
        let key = [0x01u8; 16];
        let iv = [0x02u8; 16];
        let plain: [u8; 16] = std::array::from_fn(|i| (i + 1) as u8);

        // 5 clear, 3 enc, 4 clear, 4 enc
        let subs = [
            Subsample {
                clear: 5,
                encrypted: 3,
            },
            Subsample {
                clear: 4,
                encrypted: 4,
            },
        ];
        let mut buf = plain;
        apply_cenc_subsamples(&key, &iv, &subs, &mut buf).unwrap();

        assert_eq!(&buf[0..5], &plain[0..5]); // clear
        assert_eq!(&buf[8..12], &plain[8..12]); // clear
        assert_ne!(&buf[5..8], &plain[5..8]); // enc
        assert_ne!(&buf[12..16], &plain[12..16]); // enc

        // cenc: keystream advances only over encrypted spans → feeding the two
        // encrypted spans as one 7-byte stream must match.
        let mut continuous = [0u8; 7];
        continuous[0..3].copy_from_slice(&plain[5..8]);
        continuous[3..7].copy_from_slice(&plain[12..16]);
        Aes128CtrCipher::new(&key, &iv).apply_keystream(&mut continuous);
        assert_eq!(&buf[5..8], &continuous[0..3]);
        assert_eq!(&buf[12..16], &continuous[3..7]);
    }

    #[test]
    fn byte_mismatch_rejected_and_buf_untouched() {
        let key = [0u8; 16];
        let iv = [0u8; 16];
        let plain = [0xAAu8; 10];
        let mut buf = plain;
        let subs = [Subsample {
            clear: 5,
            encrypted: 6,
        }]; // total 11 != 10

        let r = apply_cenc_subsamples(&key, &iv, &subs, &mut buf);
        assert!(matches!(
            r,
            Err(Error::SubsampleByteMismatch {
                expected: 10,
                actual: 11
            })
        ));
        assert_eq!(buf, plain);
    }

    #[test]
    fn per_sample_iv_counter_8byte() {
        // 8-byte IV: per-sample IV in the HIGH 8 bytes, block counter (0) in low 8.
        let base = Iv::Bytes8([0u8; 8]);
        assert_eq!(
            derive_per_sample_iv(&base, 0, 0),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            derive_per_sample_iv(&base, 257, 0),
            [0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn per_sample_iv_adds_to_base_high_8() {
        let base = Iv::Bytes8([0, 0, 0, 0, 0, 0, 0, 10]);
        let iv = derive_per_sample_iv(&base, 5, 0);
        // high 8 = base_high(10) + 5 = 15, low 8 = block counter 0
        assert_eq!(&iv[0..8], &[0, 0, 0, 0, 0, 0, 0, 15]);
        assert_eq!(&iv[8..16], &[0u8; 8]);
    }

    #[test]
    fn derive_16_adds_block_offset_128bit() {
        // 16-byte IV: whole 16B = base + block_offset (128-bit add).
        let base = Iv::Bytes16([0u8; 16]);
        // offset 0 → base unchanged
        assert_eq!(derive_per_sample_iv(&base, 0, 0), [0u8; 16]);
        // offset 5 lands in the low 8 bytes (block counter region)
        assert_eq!(
            derive_per_sample_iv(&base, 0, 5),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5]
        );
    }

    #[test]
    fn derive_16_low_overflow_carries_to_high() {
        // low 8 bytes = 0xFFFF...F, +1 must carry into the high 8 bytes (nonce).
        let mut b = [0u8; 16];
        b[8..16].copy_from_slice(&u64::MAX.to_be_bytes()); // low 8 = max
        let base = Iv::Bytes16(b);
        let iv = derive_per_sample_iv(&base, 0, 1);
        // high 8 becomes ...0001, low 8 wraps to 0
        assert_eq!(&iv[0..8], &[0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(&iv[8..16], &[0u8; 8]);
    }

    #[test]
    fn derive_16_nonce_preserved_without_overflow() {
        // high 8 = a fixed nonce; small block_offset must not touch it.
        let mut b = [0u8; 16];
        b[0..8].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
        let base = Iv::Bytes16(b);
        let iv = derive_per_sample_iv(&base, 0, 123);
        assert_eq!(&iv[0..8], &[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&iv[8..16], &123u64.to_be_bytes());
    }

    #[test]
    fn encrypted_block_count_cases() {
        // full sample (no subsamples) → ceil(len/16)
        assert_eq!(encrypted_block_count(&[], 0), 0);
        assert_eq!(encrypted_block_count(&[], 1), 1);
        assert_eq!(encrypted_block_count(&[], 16), 1);
        assert_eq!(encrypted_block_count(&[], 17), 2);
        // subsamples → ceil(sum(encrypted)/16); clear bytes ignored
        let subs = [
            Subsample {
                clear: 5,
                encrypted: 100,
            },
            Subsample {
                clear: 5,
                encrypted: 200,
            },
        ];
        // 300 encrypted bytes → ceil(300/16) = 19
        assert_eq!(encrypted_block_count(&subs, 310), 19);
    }
}
