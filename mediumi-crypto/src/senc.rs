//! `senc` entry construction (and its `saiz` per-sample size) for the cenc and
//! cbcs schemes.

use crate::cenc::Subsample;
use crate::encrypter::Iv;
use mediumi_mp4::boxes::senc::{SencEntry, SubsampleEntry};

/// cenc senc entry: stores the per-sample IV plus the subsample map. The IV
/// length matches `tenc.default_per_sample_iv_size`: 8 bytes (the high half of
/// the counter block) for an 8-byte IV, or all 16 bytes for a 16-byte IV.
pub(crate) fn build_senc_entry_cenc(iv: &Iv, iv16: &[u8; 16], subs: &[Subsample]) -> SencEntry {
    let iv_bytes = match iv {
        Iv::Bytes8(_) => iv16[0..8].to_vec(),
        Iv::Bytes16(_) => iv16.to_vec(),
    };
    SencEntry {
        iv: iv_bytes,
        subsamples: build_subsample_entries(subs),
    }
}

/// cbcs senc entry: the constant IV lives in tenc, so the entry carries no
/// per-sample IV — only the subsample map.
pub(crate) fn build_senc_entry_cbcs(subs: &[Subsample]) -> SencEntry {
    SencEntry {
        iv: Vec::new(),
        subsamples: build_subsample_entries(subs),
    }
}

fn build_subsample_entries(subs: &[Subsample]) -> Option<Vec<SubsampleEntry>> {
    if subs.is_empty() {
        None
    } else {
        Some(
            subs.iter()
                .map(|s| SubsampleEntry {
                    bytes_of_clear_data: s.clear as u16,
                    bytes_of_protected_data: s.encrypted,
                })
                .collect(),
        )
    }
}

/// Serialized size of a senc entry (the value recorded in `saiz`):
/// `iv + N * [ subsample_count(2) + subsample.len() * (clear(2) + enc(4)) ]`.
pub(crate) fn senc_entry_size(entry: &SencEntry) -> usize {
    let mut size = entry.iv.len();
    if let Some(subs) = &entry.subsamples {
        size += 2 + subs.len() * 6;
    }
    size
}
