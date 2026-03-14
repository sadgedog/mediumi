use crate::util;

#[derive(Debug)]
pub enum Error {
    InvalidAcmod(u8),
    Bitstream(util::error::Error),
}

impl From<util::error::Error> for Error {
    fn from(e: util::error::Error) -> Self {
        Error::Bitstream(e)
    }
}
