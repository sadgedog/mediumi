//! Sample entry navigation helpers.
//!
//! `stsd.entries[i]` is now a typed [`SampleEntry`] with the codec-family-specific
//! header (Visual / Audio) parsed and nested boxes (avcC, esds, dac3, ...) kept as
//! raw bytes per [`NestedBox`]. This module exposes a small lookup helper for the
//! common case of "give me the codec config bytes for this track".

use crate::boxes::stsd::SampleEntry;
use crate::boxes::trak::Trak;

/// Walk `trak.mdia.minf.stbl.stsd.entries` and return the body bytes of the first
/// nested box whose 4-byte type matches `config_fourcc`. Returns `None` if not found,
/// if `stbl` is missing, or if no entry has any nested boxes.
///
/// Typical use: pair the codec-specific config box fourcc with its codec crate's parser.
///
/// ```text
///   avcC (H.264) → mediumi_h264::avcc::AvccConfig::parse
///   esds (AAC)   → MPEG-4 Systems descriptor parser
///   dac3 (AC-3)  → mediumi_ac3 (AC3SpecificBox)
/// ```
pub fn find_codec_config<'a>(trak: &'a Trak, config_fourcc: &[u8; 4]) -> Option<&'a [u8]> {
    let stbl = trak.mdia.minf.stbl.as_ref()?;
    for entry in &stbl.stsd.entries {
        if let Some(bytes) = find_in_entry(entry, config_fourcc) {
            return Some(bytes);
        }
    }
    None
}

fn find_in_entry<'a>(entry: &'a SampleEntry, target: &[u8; 4]) -> Option<&'a [u8]> {
    entry
        .nested
        .iter()
        .find(|n| &n.box_type == target)
        .map(|n| n.body.as_slice())
}
