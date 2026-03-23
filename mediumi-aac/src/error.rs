#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooShort,
    InvalidSyncword(u16),
    InvalidLayer(u8),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort => {
                write!(f, "ADTS data too short")
            }
            Error::InvalidSyncword(val) => {
                write!(f, "Invalid Syncword: expected 0xFFF, got 0x{:03X}", val)
            }
            Error::InvalidLayer(val) => {
                write!(f, "Invalid Layer: expected 0b00, got 0b{:02b}", val)
            }
        }
    }
}

impl std::error::Error for Error {}
