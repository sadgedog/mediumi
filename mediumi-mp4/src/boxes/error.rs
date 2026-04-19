#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooShort,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort => write!(f, "box data too short"),
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::util::error::Error> for Error {
    fn from(e: crate::util::error::Error) -> Self {
        match e {
            crate::util::error::Error::DataTooShort(_, _) => Error::DataTooShort,
        }
    }
}
