use std::fmt;

#[derive(Debug)]
pub enum Error {
    // --- cipher / subsample ---
    /// The sum of subsample byte counts did not match the buffer length.
    SubsampleByteMismatch { expected: usize, actual: usize },
    /// A single senc entry would exceed the `saiz` u8 per-sample size limit
    /// (i.e. the sample has too many subsamples).
    SencEntryTooLarge(usize),

    // --- AVC subsample walking ---
    /// A NAL length prefix or NAL body ran past the sample buffer.
    TruncatedNal,
    /// A NAL unit declared a length of zero.
    ZeroLengthNal,
    /// avcC `length_size_minus_one + 1` was not 1, 2, or 4.
    InvalidLengthSize(u8),

    // --- container ---
    /// Underlying `mediumi-mp4` parse / serialize error.
    Mp4(mediumi_mp4::Error),
    /// `moov` box not found in the input.
    NoMoov,
    /// The moov contained no track that could be encrypted (no supported codec
    /// sample entry). Encrypting it would produce a moov that advertises
    /// protection it doesn't carry.
    NoEncryptableTrack,
    /// `moof` box not found in the input.
    NoMoof,
    /// A `moof` was not immediately followed by an `mdat` in the media segment.
    NoMdatAfterMoof,
    /// An AVC track is missing its `avcC` configuration record.
    MissingAvcc,
    /// A `trun` sample size could not be determined (no per-sample size and no
    /// `tfhd` default).
    MissingSampleSize,
    /// A computed sample byte range exceeded the `mdat` buffer.
    SampleOutOfBounds { end: usize, mdat_len: usize },
    /// The serialized moof did not contain the expected `senc` box(es) during
    /// the saio-offset fixup pass.
    SaioFixupFailed,
    /// I/O error while writing an encrypted segment.
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
