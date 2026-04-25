use crate::util;

#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooShort,
    InvalidCompatibleBrandsLength(usize),
    MissingRequiredBox(&'static str),
    DuplicateBox(&'static str),
    InvalidUtf8,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort => write!(f, "box data too short"),
            Error::InvalidCompatibleBrandsLength(val) => {
                write!(f, "invalid compatible brands length for ftyp, got {}", val)
            }
            Error::MissingRequiredBox(name) => {
                write!(f, "required child box '{}' is missing", name)
            }
            Error::DuplicateBox(name) => {
                write!(f, "duplicate child box '{}'", name)
            }
            Error::InvalidUtf8 => write!(f, "invalid UTF-8 string in box body"),
        }
    }
}

impl std::error::Error for Error {}

impl From<util::error::Error> for Error {
    fn from(e: util::error::Error) -> Self {
        match e {
            util::error::Error::DataTooShort(_, _) => Error::DataTooShort,
        }
    }
}
