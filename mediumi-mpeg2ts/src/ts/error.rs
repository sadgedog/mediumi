#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidTsPacketLength(usize),
    InvalidSyncByte(u8),
    BufferTooShort { expected: usize, actual: usize },
    InvalidAfc,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidTsPacketLength(len) => {
                write!(
                    f,
                    "Invalid TS packet length: expected 188 bytes, got {} bytes",
                    len
                )
            }
            Error::InvalidSyncByte(byte) => {
                write!(f, "Invalid sync byte: expected: 0x47, got: 0x{:02X}", byte)
            }
            Error::BufferTooShort { expected, actual } => {
                write!(
                    f,
                    "Buffer is too short, expected at least {} bytes, got {} bytes",
                    expected, actual
                )
            }
            Error::InvalidAfc => {
                write!(
                    f,
                    "Adaptation field control must be 0b01, 0b10, 0b11, got 0b00"
                )
            }
        }
    }
}

impl std::error::Error for Error {}
