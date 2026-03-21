#[derive(Debug, PartialEq)]
pub enum Error {
    DataTooShort(usize, usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataTooShort(expected, actual) => {
                write!(
                    f,
                    "Data Too Short: expected at least {} bytes, got {} bytes",
                    expected, actual
                )
            }
        }
    }
}

impl std::error::Error for Error {}
