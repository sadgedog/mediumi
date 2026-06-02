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

    /// XOR the AES-CTR keystream into `buf` in place. The counter advances by
    /// `ceil(buf.len() / 16)` blocks; calling this repeatedly on the same
    /// cipher continues the keystream across non-contiguous spans.
    pub fn apply_keystream(&mut self, buf: &mut [u8]) {
        self.inner.apply_keystream(buf);
    }
}
