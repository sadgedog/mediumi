#[derive(Debug, PartialEq)]
pub enum Error {
    BufferTooShort { expected: usize, actual: usize },
    InvalidStartCode(usize),
    InvalidPtsDtsFlags(usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BufferTooShort { expected, actual } => {
                write!(
                    f,
                    "Buffer is too short: expected at least {} bytes, got {} bytes",
                    expected, actual
                )
            }
            Error::InvalidStartCode(val) => {
                write!(f, "Invalid start code: expected 0x00_00_01, got {}", val)
            }
            Error::InvalidPtsDtsFlags(val) => {
                write!(
                    f,
                    "Invalid PTS_DTS_FLAGS: expected 0b10 or 0b11, got {}",
                    val
                )
            }
        }
    }
}

impl std::error::Error for Error {}
