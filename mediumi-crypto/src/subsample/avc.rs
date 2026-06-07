//! NAL-aware subsample planner for AVC / H.264.
//!
//! Walks length-prefixed NAL units and classifies each by `nal_unit_type`.
//! VCL slice NALs get a `(prefix + 1, rest)` clear/encrypted split (NAL header byte stays clear)
//! non-VCL NALs are fully clear and folded into the next VCL NAL's clear prefix.

use crate::cenc::Subsample;
use crate::error::Error;

/// H.264 VCL NAL types: 1 (non-IDR), 2–4 (data partitions), 5 (IDR),
/// 19 (auxiliary coded picture), 20 (slice extension).
fn is_vcl(nal_type: u8) -> bool {
    matches!(nal_type, 1..=5 | 19 | 20)
}

pub fn plan(sample: &[u8], length_size: u8) -> Result<Vec<Subsample>, Error> {
    if !matches!(length_size, 1 | 2 | 4) {
        return Err(Error::InvalidLengthSize(length_size));
    }
    let ls = length_size as usize;
    let mut out: Vec<Subsample> = Vec::new();
    let mut offset = 0usize;
    let mut pending_clear: u64 = 0;

    while offset < sample.len() {
        if offset + ls > sample.len() {
            return Err(Error::TruncatedNal);
        }
        let mut nal_len = 0usize;
        for i in 0..ls {
            // get NAL Unit length from length_size & sample
            nal_len = (nal_len << 8) | sample[offset + i] as usize;
        }
        if nal_len == 0 {
            return Err(Error::ZeroLengthNal);
        }
        let prefix_end = offset + ls;
        let nal_end = prefix_end.checked_add(nal_len).ok_or(Error::TruncatedNal)?;
        if nal_end > sample.len() {
            return Err(Error::TruncatedNal);
        }

        let nal_type = sample[prefix_end] & 0b0001_1111;
        if is_vcl(nal_type) {
            // clear = accumulated non-VCL bytes + this NAL's length prefix + NAL header byte
            // [ pending non-VCL ][ len ][ header ][ slice payload ]
            //  └────────────── clear ────────────┘└── encrypted ──┘
            let clear = pending_clear + ls as u64 + 1;
            let encrypted = (nal_len - 1) as u64;
            push_clear_split(&mut out, clear, encrypted);
            pending_clear = 0;
        } else {
            // entire non-VCL NAL (prefix + body) stays clear
            pending_clear += ls as u64 + nal_len as u64;
        }
        offset = nal_end;
    }

    // trailing non-VCL bytes (sample ends without a VCL after them)
    if pending_clear > 0 {
        push_clear_split(&mut out, pending_clear, 0);
    }
    Ok(out)
}

/// `bytes_of_clear_data` in senc is u16. Keep our in-memory `Subsample.clear`
/// (u32) below that limit by splitting oversized clear runs into clear-only
/// subsamples, attaching `encrypted` to the final piece.
fn push_clear_split(out: &mut Vec<Subsample>, clear: u64, encrypted: u64) {
    let mut remaining = clear;
    while remaining > u16::MAX as u64 {
        out.push(Subsample {
            clear: u16::MAX as u32,
            encrypted: 0,
        });
        remaining -= u16::MAX as u64;
    }
    out.push(Subsample {
        clear: remaining as u32,
        encrypted: encrypted as u32,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `[length BE (length_size)][nal_header][body...]`
    fn lp_nal(length_size: u8, nal_type: u8, body_len: usize) -> Vec<u8> {
        let mut nal = Vec::new();
        let header = 0x60 | (nal_type & 0x1F); // nal_ref_idc=3
        let total = 1 + body_len;
        let prefix = (total as u32).to_be_bytes();
        nal.extend_from_slice(&prefix[4 - length_size as usize..]);
        nal.push(header);
        nal.extend(std::iter::repeat_n(0xAAu8, body_len));
        nal
    }

    #[test]
    fn invalid_length_size() {
        assert!(matches!(
            plan(&[0u8; 4], 3),
            Err(Error::InvalidLengthSize(3))
        ));
    }

    #[test]
    fn single_idr_4byte_prefix() {
        let sample = lp_nal(4, 5, 8); // IDR, 8-byte body
        let plan = plan(&sample, 4).unwrap();
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 5,
                encrypted: 8
            }]
        );
    }

    #[test]
    fn single_non_idr_2byte_prefix() {
        let sample = lp_nal(2, 1, 50);
        let plan = plan(&sample, 2).unwrap();
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 3,
                encrypted: 50
            }]
        );
    }

    #[test]
    fn sps_pps_idr_merges_clear_prefix() {
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 7, 10)); // SPS: 4+1+10 = 15
        sample.extend(lp_nal(4, 8, 4)); //  PPS: 4+1+4  = 9
        sample.extend(lp_nal(4, 5, 100)); // IDR: clear prefix 5, enc 100
        let plan = plan(&sample, 4).unwrap();
        // clear = 15 + 9 + 5 = 29, encrypted = 100
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 29,
                encrypted: 100
            }]
        );
    }

    #[test]
    fn trailing_non_vcl_is_clear_only() {
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 1, 20)); // VCL
        sample.extend(lp_nal(4, 6, 8)); //  SEI trailing
        let plan = plan(&sample, 4).unwrap();
        assert_eq!(
            plan,
            vec![
                Subsample {
                    clear: 5,
                    encrypted: 20
                },
                Subsample {
                    clear: 13,
                    encrypted: 0
                },
            ]
        );
    }

    #[test]
    fn truncated_and_zero_length() {
        assert!(matches!(plan(&[0, 0], 4), Err(Error::TruncatedNal)));
        assert!(matches!(plan(&[0, 0, 0, 0], 4), Err(Error::ZeroLengthNal)));
        let truncated = vec![0, 0, 0, 100, 0x65, 0xAA];
        assert!(matches!(plan(&truncated, 4), Err(Error::TruncatedNal)));
    }

    #[test]
    fn large_clear_splits_at_u16() {
        let huge = u16::MAX as usize + 10; // SEI body
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 6, huge)); // clear = 4 + 1 + 65545 = 65550
        sample.extend(lp_nal(4, 5, 16)); //  IDR: clear prefix 5, enc 16
        // total clear = 65550 + 5 = 65555 > 65535 → split
        let plan = plan(&sample, 4).unwrap();
        assert_eq!(
            plan,
            vec![
                Subsample {
                    clear: 65535,
                    encrypted: 0
                },
                Subsample {
                    clear: 65555 - 65535,
                    encrypted: 16
                },
            ]
        );
    }
}
