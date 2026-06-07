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
