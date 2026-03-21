use crate::util;

#[derive(Debug)]
pub enum Error {
    DataTooShort,
    InvalidSyncword(u16),
    InvalidFrameSize,
    InvalidAcmod(u8),
    InvalidState(&'static str),
    Bitstream(util::error::Error),
}

impl From<util::error::Error> for Error {
    fn from(e: util::error::Error) -> Self {
        Error::Bitstream(e)
    }
}
