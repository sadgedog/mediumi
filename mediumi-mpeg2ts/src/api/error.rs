#[derive(Debug)]
pub enum Error {
    Ts(crate::ts::error::Error),
    Pes(crate::pes::error::Error),
    Psi(crate::psi::error::Error),
    InvalidPacketsLength(usize),
    PatNotFound,
    PmtNotFound,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Ts(e) => write!(f, "TS packet error: {}", e),
            Error::Pes(e) => write!(f, "PES error: {}", e),
            Error::Psi(e) => write!(f, "PSI error: {}", e),
            Error::InvalidPacketsLength(len) => {
                write!(
                    f,
                    "Invalid TS data length: expected 188 multiple, got {}",
                    len
                )
            }
            Error::PatNotFound => write!(f, "PAT not found"),
            Error::PmtNotFound => write!(f, "PMT not found"),
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::ts::error::Error> for Error {
    fn from(e: crate::ts::error::Error) -> Self {
        Error::Ts(e)
    }
}

impl From<crate::pes::error::Error> for Error {
    fn from(e: crate::pes::error::Error) -> Self {
        Error::Pes(e)
    }
}

impl From<crate::psi::error::Error> for Error {
    fn from(e: crate::psi::error::Error) -> Self {
        Error::Psi(e)
    }
}
