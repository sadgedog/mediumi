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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort => write!(f, "AC-3 data too short"),
            Error::InvalidSyncword(val) => {
                write!(f, "Invalid syncword: expected 0x0B77, got 0x{:04X}", val)
            }
            Error::InvalidFrameSize => write!(f, "Invalid frame size"),
            Error::InvalidAcmod(val) => write!(f, "Invalid acmod: {}", val),
            Error::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Error::Bitstream(e) => write!(f, "Bitstream error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<util::error::Error> for Error {
    fn from(e: util::error::Error) -> Self {
        Error::Bitstream(e)
    }
}
