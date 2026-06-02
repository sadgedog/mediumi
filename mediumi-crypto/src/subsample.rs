pub mod avc;
pub mod mp4a;

use crate::cenc::Subsample;
use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecKind {
    /// AAC (`mp4a`): full-sample encryption, no subsamples.
    Mp4a,
    /// AVC / H.264 (`avc1` / `avc3`): length-prefixed NAL units.
    /// `length_size` is `avcC.length_size_minus_one + 1` (1, 2, or 4).
    Avc { length_size: u8 },
}

/// Build a subsample plan for one sample.
///
/// An **empty** result means "encrypt the whole sample with no subsamples" (audio).
/// A **non-empty** result drives a subsample-style senc entry (video).
pub fn plan(codec: CodecKind, sample: &[u8]) -> Result<Vec<Subsample>, Error> {
    match codec {
        CodecKind::Mp4a => mp4a::plan(sample),
        CodecKind::Avc { length_size } => avc::plan(sample, length_size),
    }
}
