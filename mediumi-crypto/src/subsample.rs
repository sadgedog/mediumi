pub mod avc;
pub mod mp4a;

use crate::cenc::Subsample;
use crate::error::Error;
use mediumi_h264::{pps::Pps, sps::Sps};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaKind {
    Video,
    Audio,
}

impl MediaKind {
    /// cbcs pattern `(crypt_byte_block, skip_byte_block)`: video encrypts 1 of
    /// every 10 blocks (1:9); audio has no pattern (0:0 → every full block).
    pub(crate) fn cbcs_pattern(self) -> (u8, u8) {
        match self {
            MediaKind::Video => (1, 9),
            MediaKind::Audio => (0, 0),
        }
    }
}

/// Classify a sample-entry fourcc, in either its clear (`avc1`/`avc3`/`mp4a`)
/// or already-encrypted (`encv`/`enca`) form. `None` = unsupported codec.
pub(crate) fn media_kind(fourcc: &[u8; 4]) -> Option<MediaKind> {
    match fourcc {
        b"avc1" | b"avc3" | b"encv" => Some(MediaKind::Video),
        b"mp4a" | b"enca" => Some(MediaKind::Audio),
        _ => None,
    }
}

/// True for a *clear* sample entry we can encrypt; the already-encrypted
/// `encv`/`enca` aliases are excluded so they aren't wrapped twice.
pub(crate) fn is_encryptable(fourcc: &[u8; 4]) -> bool {
    !matches!(fourcc, b"encv" | b"enca") && media_kind(fourcc).is_some()
}

#[derive(Debug, Clone)]
pub enum CodecKind {
    /// AAC (`mp4a`): full-sample encryption, no subsamples.
    Mp4a,
    /// AVC / H.264 (`avc1` / `avc3`): length-prefixed NAL units.
    /// `length_size` is `avcC.length_size_minus_one + 1` (1, 2, or 4).
    Avc {
        length_size: u8,
        sps: Option<Box<Sps>>,
        pps: Option<Box<Pps>>,
    },
}

impl CodecKind {
    fn media_kind(&self) -> MediaKind {
        match self {
            CodecKind::Avc { .. } => MediaKind::Video,
            CodecKind::Mp4a => MediaKind::Audio,
        }
    }

    /// cbcs pattern for this codec.
    pub fn cbcs_pattern(&self) -> (u8, u8) {
        self.media_kind().cbcs_pattern()
    }
}

/// Build a subsample plan for one sample.
///
/// An **empty** result means "encrypt the whole sample with no subsamples" (audio).
/// A **non-empty** result drives a subsample-style senc entry (video).
pub fn plan(codec: &CodecKind, sample: &[u8]) -> Result<Vec<Subsample>, Error> {
    match codec {
        CodecKind::Mp4a => mp4a::plan(sample),
        CodecKind::Avc {
            length_size,
            sps,
            pps,
        } => avc::plan(sample, *length_size, sps.as_deref(), pps.as_deref()),
    }
}
