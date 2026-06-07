//! NAL-aware subsample planner for AVC / H.264.
//!
//! Walks length-prefixed NAL units and classifies each by `nal_unit_type`.
//! VCL slice NALs (type 1/5) larger than 48 bytes get a 32-byte clear leader
//! (NAL header + slice header) with the rest encrypted; the leader lets hardware
//! decoders (FairPlay / Widevine L1) parse the slice header before decryption,
//! per Apple HLS Sample Encryption and CENC v3. VCL NALs of 48 bytes or fewer,
//! and all non-VCL NALs, stay fully clear and fold into the next encrypted NAL's
//! clear prefix.

use crate::cenc::Subsample;
use crate::error::Error;

/// VCL NAL clear leader: the `nal_unit_type` byte plus the 31 bytes that follow
/// stay unencrypted (Apple HLS Sample Encryption / CENC v3). The slice header
/// fits inside this leader, so hardware decoders can parse it before decryption.
const VCL_CLEAR_LEADER: u64 = 32;
/// A VCL NAL of this size (bytes after the length prefix) or fewer is left
/// completely unencrypted: 32-byte leader + one 16-byte protected block = 48.
const MIN_ENCRYPTABLE_NAL: usize = 48;

/// NAL types that Apple HLS Sample Encryption / CENC v3 encrypt: 1 (non-IDR
/// coded slice) and 5 (IDR coded slice) only. Data-partition (2–4) and
/// SVC/MVC extension (19/20) NALs MUST NOT be encrypted under Apple's spec and
/// are not produced by mainstream AVC encoders.
fn is_vcl(nal_type: u8) -> bool {
    matches!(nal_type, 1 | 5)
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
        if is_vcl(nal_type) && nal_len > MIN_ENCRYPTABLE_NAL {
            // clear = accumulated clear bytes + this NAL's length prefix + 32-byte leader
            // [ pending clear ][ len ][ 32-byte leader ][ encrypted slice data ]
            //  └──────────────── clear ────────────────┘└─────── encrypted ────┘
            let clear = pending_clear + ls as u64 + VCL_CLEAR_LEADER;
            let encrypted = nal_len as u64 - VCL_CLEAR_LEADER;
            push_clear_split(&mut out, clear, encrypted);
            pending_clear = 0;
        } else {
            // non-VCL NAL, or a VCL NAL ≤ 48 bytes → entire NAL (prefix + body) stays clear
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
    fn idr_over_48_gets_32_byte_leader() {
        // IDR with 64-byte body → nal_len 65 > 48 → 32-byte leader, rest encrypted.
        let sample = lp_nal(4, 5, 64);
        let plan = plan(&sample, 4).unwrap();
        // clear = length prefix(4) + 32-byte leader = 36, encrypted = 65 - 32 = 33
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 36,
                encrypted: 33
            }]
        );
    }

    #[test]
    fn non_idr_over_48_gets_32_byte_leader() {
        // non-IDR with 50-byte body → nal_len 51 > 48 (2-byte length prefix).
        let sample = lp_nal(2, 1, 50);
        let plan = plan(&sample, 2).unwrap();
        // clear = prefix(2) + 32 = 34, encrypted = 51 - 32 = 19
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 34,
                encrypted: 19
            }]
        );
    }

    #[test]
    fn vcl_48_bytes_or_fewer_fully_clear() {
        // nal_len exactly 48 (47-byte body) → completely unencrypted.
        let sample = lp_nal(4, 5, 47);
        // whole NAL clear: prefix(4) + nal_len(48) = 52
        assert_eq!(
            plan(&sample, 4).unwrap(),
            vec![Subsample {
                clear: 52,
                encrypted: 0
            }]
        );

        // nal_len 49 (48-byte body) → just over threshold → leader + encrypt.
        let sample = lp_nal(4, 5, 48);
        // clear = 4 + 32 = 36, encrypted = 49 - 32 = 17
        assert_eq!(
            plan(&sample, 4).unwrap(),
            vec![Subsample {
                clear: 36,
                encrypted: 17
            }]
        );
    }

    #[test]
    fn sps_pps_idr_merges_clear_prefix() {
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 7, 10)); // SPS: 4+1+10 = 15
        sample.extend(lp_nal(4, 8, 4)); //  PPS: 4+1+4  = 9
        sample.extend(lp_nal(4, 5, 100)); // IDR: nal_len 101 > 48
        let plan = plan(&sample, 4).unwrap();
        // clear = 15 + 9 + prefix(4) + 32 = 60, encrypted = 101 - 32 = 69
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 60,
                encrypted: 69
            }]
        );
    }

    #[test]
    fn data_partition_and_extension_not_encrypted() {
        // type 2 (data partition A) is NOT VCL for encryption (Apple/CENC v3
        // encrypt only 1/5) → stays clear, folded into the next IDR's prefix.
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 2, 30)); // data partition: 4+1+30 = 35 clear
        sample.extend(lp_nal(4, 5, 100)); // IDR: nal_len 101 > 48
        let plan = plan(&sample, 4).unwrap();
        // clear = 35 + prefix(4) + 32 = 71, encrypted = 69
        assert_eq!(
            plan,
            vec![Subsample {
                clear: 71,
                encrypted: 69
            }]
        );
    }

    #[test]
    fn trailing_non_vcl_is_clear_only() {
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 1, 60)); // VCL: nal_len 61 > 48
        sample.extend(lp_nal(4, 6, 8)); //  SEI trailing: 4+1+8 = 13 clear
        let plan = plan(&sample, 4).unwrap();
        assert_eq!(
            plan,
            vec![
                Subsample {
                    clear: 36, // prefix(4) + 32-byte leader
                    encrypted: 29
                }, // 61 - 32
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
        sample.extend(lp_nal(4, 5, 60)); //  IDR: nal_len 61 > 48
        // total clear = 65550 + prefix(4) + 32 = 65586 > 65535 → split
        let plan = plan(&sample, 4).unwrap();
        assert_eq!(
            plan,
            vec![
                Subsample {
                    clear: 65535,
                    encrypted: 0
                },
                Subsample {
                    clear: 65586 - 65535,
                    encrypted: 29
                }, // 51, 61 - 32
            ]
        );
    }
}
