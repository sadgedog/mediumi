pub mod aud;
pub mod error;
pub mod nal;

use crate::aud::Aud;
use crate::error::Error;

#[derive(Debug)]
pub enum StartCode {
    ThreeBytes, // [0x00, 0x00, 0x01]
    FourBytes,  // [0x00, 0x00, 0x00, 0x01]
}

#[derive(Debug)]
pub enum NalData {
    Aud(StartCode, Aud),
}

fn serialize_nal<'a>(nal: &'a NalData) -> Result<Option<Vec<u8>>, Error> {
    todo!()
}
