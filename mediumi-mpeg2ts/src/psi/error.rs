#[derive(Debug, PartialEq)]
pub enum Error {
    BufferTooShort {
        expected: usize,
        actual: usize,
    },
    InvalidTableId {
        expected: u8,
        actual: u8,
    },
    InvalidSectionSyntaxIndicator,
    InvalidSectionLength(u16),
    InvalidSectionNumber {
        section_number: u8,
        last_section_number: u8,
    },
    Crc32Mismatch {
        expected: u32,
        actual: u32,
    },
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
            Error::InvalidTableId { expected, actual } => {
                write!(f, "Invalid table_id: expected {}, got {}", expected, actual)
            }
            Error::InvalidSectionSyntaxIndicator => {
                write!(f, "Invalid section_syntax_indicator: expected true")
            }
            Error::InvalidSectionLength(len) => {
                write!(f, "Invalid section_length: {}", len)
            }
            Error::InvalidSectionNumber {
                section_number,
                last_section_number,
            } => {
                write!(
                    f,
                    "Invalid section_number: section_number({}) > last_section_number({})",
                    section_number, last_section_number
                )
            }
            Error::Crc32Mismatch { expected, actual } => {
                write!(f, "CRC32 mismatch: expected {}, got {}", expected, actual)
            }
        }
    }
}

impl std::error::Error for Error {}
