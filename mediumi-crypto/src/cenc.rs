//! CENC `cenc` scheme — AES-128-CTR sample-level encryption with subsamples.
//!
//! In the `cenc` scheme the AES-CTR keystream is consumed only over the
//! encrypted spans; clear spans are skipped (the counter does NOT advance
//! through them). Per-sample IVs are derived from a base IV plus a big-endian
//! sample counter in the low 8 bytes.

use crate::ctr::Aes128CtrCipher;
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

/// Derive the 16-byte AES-CTR initial counter block for sample `sample_index`.
pub fn derive_per_sample_iv(base_iv: &[u8; 16], sample_index: u64) -> [u8; 16] {
    let base_hi = u64::from_be_bytes(base_iv[0..8].try_into().unwrap());
    let per_sample = base_hi.wrapping_add(sample_index);
    let mut iv = [0u8; 16];
    iv[0..8].copy_from_slice(&per_sample.to_be_bytes());
    // iv[8..16] stays zero — the AES-CTR block counter.
    iv
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
    fn per_sample_iv_counter() {
        // per-sample IV in the HIGH 8 bytes, block counter (0) in the low 8.
        let base = [0u8; 16];
        assert_eq!(
            derive_per_sample_iv(&base, 0),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            derive_per_sample_iv(&base, 257),
            [0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn per_sample_iv_adds_to_base_high_8() {
        let base = [
            0, 0, 0, 0, 0, 0, 0, 10, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ];
        let iv = derive_per_sample_iv(&base, 5);
        // high 8 = base_high(10) + 5 = 15, low 8 = block counter 0
        assert_eq!(&iv[0..8], &[0, 0, 0, 0, 0, 0, 0, 15]);
        assert_eq!(&iv[8..16], &[0u8; 8]);
    }
}
