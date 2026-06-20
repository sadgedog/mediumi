//! Thin wrapper around AES-128 in CTR mode (the `cenc` cipher primitive).

use aes::Aes128;
use aes::cipher::{KeyIvInit, StreamCipher};

type Aes128Ctr128BE = ctr::Ctr128BE<Aes128>;

pub struct Aes128CtrCipher {
    inner: Aes128Ctr128BE,
}

impl Aes128CtrCipher {
    pub fn new(key: &[u8; 16], iv: &[u8; 16]) -> Self {
        Self {
            inner: Aes128Ctr128BE::new(key.into(), iv.into()),
        }
    }

    /// XOR the AES-CTR keystream into `buf` in place.
    pub fn apply_keystream(&mut self, buf: &mut [u8]) {
        self.inner.apply_keystream(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NIST SP 800-38A F.5.1/F.5.2 — AES-128-CTR.
    const KEY: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];
    // Initial counter block.
    const CTR0: [u8; 16] = [
        0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe,
        0xff,
    ];
    const PT1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    const CT1: [u8; 16] = [
        0x87, 0x4d, 0x61, 0x91, 0xb6, 0x20, 0xe3, 0x26, 0x1b, 0xef, 0x68, 0x64, 0x99, 0x0d, 0xb6,
        0xce,
    ];
    const PT2: [u8; 16] = [
        0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac, 0x45, 0xaf, 0x8e,
        0x51,
    ];
    const CT2: [u8; 16] = [
        0x98, 0x06, 0xf6, 0x6b, 0x79, 0x70, 0xfd, 0xff, 0x86, 0x17, 0x18, 0x7b, 0xb9, 0xff, 0xfd,
        0xff,
    ];
    const PT3: [u8; 16] = [
        0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11, 0xe5, 0xfb, 0xc1, 0x19, 0x1a, 0x0a, 0x52,
        0xef,
    ];
    const CT3: [u8; 16] = [
        0x5a, 0xe4, 0xdf, 0x3e, 0xdb, 0xd5, 0xd3, 0x5e, 0x5b, 0x4f, 0x09, 0x02, 0x0d, 0xb0, 0x3e,
        0xab,
    ];
    const PT4: [u8; 16] = [
        0xf6, 0x9f, 0x24, 0x45, 0xdf, 0x4f, 0x9b, 0x17, 0xad, 0x2b, 0x41, 0x7b, 0xe6, 0x6c, 0x37,
        0x10,
    ];
    const CT4: [u8; 16] = [
        0x1e, 0x03, 0x1d, 0xda, 0x2f, 0xbe, 0x03, 0xd1, 0x79, 0x21, 0x70, 0xa0, 0xf3, 0x00, 0x9c,
        0xee,
    ];

    fn concat(blocks: &[[u8; 16]]) -> Vec<u8> {
        blocks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    #[test]
    fn encrypt_matches_nist_vector() {
        // F.5.1 CTR-AES128.Encrypt: counter increments by one per 16-byte block.
        let mut buf = concat(&[PT1, PT2, PT3, PT4]);
        Aes128CtrCipher::new(&KEY, &CTR0).apply_keystream(&mut buf);
        assert_eq!(buf, concat(&[CT1, CT2, CT3, CT4]));
    }

    #[test]
    fn decrypt_matches_nist_vector() {
        // F.5.2 CTR-AES128.Decrypt: CTR decryption is the same keystream XOR.
        let mut buf = concat(&[CT1, CT2, CT3, CT4]);
        Aes128CtrCipher::new(&KEY, &CTR0).apply_keystream(&mut buf);
        assert_eq!(buf, concat(&[PT1, PT2, PT3, PT4]));
    }

    #[test]
    fn split_keystream_calls_match_single_call() {
        // The keystream must stay continuous across calls (the cipher carries the
        // counter), so encrypting in pieces equals encrypting in one shot — the
        // property cenc relies on when stepping over subsample boundaries.
        let plain = concat(&[PT1, PT2, PT3, PT4]);

        let mut whole = plain.clone();
        Aes128CtrCipher::new(&KEY, &CTR0).apply_keystream(&mut whole);

        let mut piecewise = plain.clone();
        let mut cipher = Aes128CtrCipher::new(&KEY, &CTR0);
        let (a, b) = piecewise.split_at_mut(19); // split mid-block, not on a boundary
        cipher.apply_keystream(a);
        cipher.apply_keystream(b);

        assert_eq!(piecewise, whole);
        assert_eq!(piecewise, concat(&[CT1, CT2, CT3, CT4]));
    }
}
