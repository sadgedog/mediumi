use crate::error::Error;

#[derive(Debug)]
pub enum Mode {
    Cenc,
    Cbcs,
}

#[derive(Debug)]
pub struct Encrypter {
    pub mode: Mode,
    pub key_id: [u8; 16],
    pub key: [u8; 16],
    pub iv: [u8; 16],
}

impl Encrypter {
    pub fn new(mode: Mode, key_id: [u8; 16], key: [u8; 16], iv: [u8; 16]) -> Self {
        Self {
            mode,
            key_id,
            key,
            iv,
        }
    }

    pub fn enc_initial(&self, moov: &[u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn enc_media(&self, moov: &[u8], moof: &[u8], mdat: &mut [u8]) -> Result<Vec<u8>, Error> {
        todo!()
    }
}
