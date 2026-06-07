use std::fmt;

#[derive(Debug)]
pub enum Error {
    SubsampleByteMismatch { expected: usize, actual: usize },
    SencEntryTooLarge(usize),
    TruncatedNal,
    ZeroLengthNal,
    InvalidLengthSize(u8),
    SliceHeaderParseFailed,
    Mp4(mediumi_mp4::Error),
    NoMoov,
    NoEncryptableTrack,
    NoMoof,
    NoMdatAfterMoof,
    MissingAvcc,
    MissingSampleSize,
    SampleOutOfBounds { end: usize, mdat_len: usize },
    SaioFixupFailed,
    Io(std::io::Error),
}

impl From<mediumi_mp4::Error> for Error {
    fn from(e: mediumi_mp4::Error) -> Self {
        Error::Mp4(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SubsampleByteMismatch { expected, actual } => write!(
                f,
                "subsample byte total ({actual}) does not match buffer length ({expected})"
            ),
            Error::SencEntryTooLarge(n) => {
                write!(f, "senc entry size {n} exceeds the saiz u8 limit")
            }
            Error::TruncatedNal => write!(f, "NAL length prefix or body truncated"),
            Error::ZeroLengthNal => write!(f, "encountered a zero-length NAL unit"),
            Error::InvalidLengthSize(v) => {
                write!(f, "invalid avcC length size {v} (expected 1, 2, or 4)")
            }
            Error::SliceHeaderParseFailed => {
                write!(
                    f,
                    "failed to parse a VCL NAL slice header (need valid SPS/PPS)"
                )
            }
            Error::Mp4(e) => write!(f, "mp4: {e}"),
            Error::NoMoov => write!(f, "moov box not found"),
            Error::NoEncryptableTrack => write!(f, "moov has no encryptable track"),
            Error::NoMoof => write!(f, "moof box not found"),
            Error::NoMdatAfterMoof => write!(f, "moof not followed by mdat in media segment"),
            Error::MissingAvcc => write!(f, "AVC track missing avcC configuration"),
            Error::MissingSampleSize => write!(f, "could not determine trun sample size"),
            Error::SampleOutOfBounds { end, mdat_len } => {
                write!(f, "sample end {end} exceeds mdat length {mdat_len}")
            }
            Error::SaioFixupFailed => write!(f, "saio offset fixup could not locate senc"),
            Error::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for Error {}
