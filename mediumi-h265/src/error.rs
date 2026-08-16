#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooShort,
    InvalidForbiddenZeroBit,
    InvalidPicType(u8),
    InvalidTrailingBits,
    InvalidStartCode(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort => write!(f, "Data too short"),
            Error::InvalidForbiddenZeroBit => write!(f, "Invalid forbidden_zero_bit value"),
            Error::InvalidPicType(value) => write!(f, "Invalid picture type: {}", value),
            Error::InvalidTrailingBits => write!(f, "Invalid trailing bits"),
            Error::InvalidStartCode(value) => write!(f, "Invalid start code: {value:#010x}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<mediumi_util::error::Error> for Error {
    fn from(e: mediumi_util::error::Error) -> Self {
        match e {
            mediumi_util::error::Error::DataTooShort(_, _) => Error::DataTooShort,
        }
    }
}
