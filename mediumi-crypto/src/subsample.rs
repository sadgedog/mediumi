pub mod avc;
pub mod mp4a;

use crate::cenc::Subsample;
use crate::error::Error;
use mediumi_h264::{pps::Pps, sps::Sps};

#[derive(Debug, Clone)]
pub enum CodecKind {
    /// AAC (`mp4a`): full-sample encryption, no subsamples.
    Mp4a,
    /// AVC / H.264 (`avc1` / `avc3`): length-prefixed NAL units.
    /// `length_size` is `avcC.length_size_minus_one + 1` (1, 2, or 4).
    /// `sps`/`pps` (parsed from avcC) let the AVC planner measure each VCL
    /// NAL's slice-header length and leave exactly that in the clear.
    /// When `None` (avcC missing or unparsable) the planner
    /// falls back to a fixed 32-byte clear leader.
    Avc {
        length_size: u8,
        sps: Option<Box<Sps>>,
        pps: Option<Box<Pps>>,
    },
}

impl CodecKind {
    /// cbcs pattern `(crypt_byte_block, skip_byte_block)` in 16-byte blocks.
    /// Video uses 1:9 (encrypt 1 of every 10 blocks); audio uses 0:0 (no
    /// pattern — every full block is encrypted).
    pub fn cbcs_pattern(&self) -> (u8, u8) {
        match self {
            CodecKind::Avc { .. } => (1, 9),
            CodecKind::Mp4a => (0, 0),
        }
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
