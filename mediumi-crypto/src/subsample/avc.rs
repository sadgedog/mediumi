//! NAL-aware subsample planner for AVC / H.264.
//!
//! Walks length-prefixed NAL units and classifies each by `nal_unit_type`.
//! VCL slice NALs (type 1/5) get a clear leader of `NAL header byte + the actual
//! slice-header length` (parsed from SPS/PPS), leaving only the slice data
//! encrypted. A VCL NAL whose slice header cannot be parsed is a hard error.
//! A VCL NAL whose protected slice data would be ≤ 16 bytes, and all non-VCL NALs,
//! stay fully clear and fold into the next encrypted NAL's clear prefix.

use crate::cenc::Subsample;
use crate::error::Error;
use mediumi_h264::nal::{NalUnit, NalUnitType};
use mediumi_h264::pps::Pps;
use mediumi_h264::slice_header::SliceHeader;
use mediumi_h264::sps::Sps;
use mediumi_h264::util::bitstream::BitstreamReader;

/// A VCL NAL whose protected (post-leader) slice data is this many bytes or
/// fewer is left completely unencrypted (cbcs encrypts nothing below one 16-byte
/// block; Apple: "48 bytes or fewer is completely unencrypted").
const MIN_ENCRYPTED_BYTES: usize = 16;

fn is_vcl(nal_type: u8) -> bool {
    matches!(nal_type, 1 | 5)
}

pub fn plan(
    sample: &[u8],
    length_size: u8,
    sps: Option<&Sps>,
    pps: Option<&Pps>,
) -> Result<Vec<Subsample>, Error> {
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

        let header_byte = sample[prefix_end];
        let nal_type = header_byte & 0b0001_1111;
        let nal_ref_idc = (header_byte >> 5) & 0b11;
        if is_vcl(nal_type) {
            // clear leader = NAL header byte + slice header.
            let sh = slice_header_len(
                &sample[prefix_end + 1..nal_end],
                nal_type,
                nal_ref_idc,
                sps,
                pps,
            )
            .ok_or(Error::SliceHeaderParseFailed)?;
            let leader = 1 + sh; // NAL header byte + slice header bytes
            if nal_len <= leader + MIN_ENCRYPTED_BYTES {
                // protected slice data ≤ 16 bytes → leave the whole NAL clear
                pending_clear += ls as u64 + nal_len as u64;
            } else {
                // [ pending clear ][ len ][ NALhdr + slice header ][ slice data ]
                //  └──────────────── clear ──────────────────────┘└ encrypted ─┘
                let clear = pending_clear + ls as u64 + leader as u64;
                let encrypted = nal_len as u64 - leader as u64;
                push_clear_split(&mut out, clear, encrypted);
                pending_clear = 0;
            }
        } else {
            // non-VCL NAL → entire NAL (prefix + body) stays clear
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

/// Measure the slice-header byte length of a VCL NAL (the clear leader that　follows the NAL header byte).
fn slice_header_len(
    nal_body: &[u8],
    nal_type: u8,
    nal_ref_idc: u8,
    sps: Option<&Sps>,
    pps: Option<&Pps>,
) -> Option<usize> {
    let sps = sps?;
    let pps = pps?;
    // slice_header() is parsed over the RBSP (emulation-prevention bytes removed).
    let rbsp = NalUnit::remove_emulation_prevention_bytes(nal_body);
    let mut reader = BitstreamReader::new(&rbsp);
    let nut = NalUnitType::from(nal_type);
    SliceHeader::parse(&mut reader, sps, pps, nut, nal_ref_idc).ok()?;
    // Bytes consumed by the slice header, rounded up to a byte boundary (slice
    // data is byte-aligned; cabac_alignment bits round into this final byte).
    let consumed_bits = rbsp.len() * 8 - reader.remaining_bits();
    let sh_bytes_rbsp = consumed_bits.div_ceil(8);
    // Convert the RBSP length back to a length in the original NAL bytes,
    // re-adding any emulation-prevention bytes inside the slice header.
    Some(rbsp_len_to_raw_len(nal_body, sh_bytes_rbsp))
}

/// Map a byte length measured in the RBSP (emulation-prevention bytes removed)
/// back to the corresponding length in the original NAL bytes. Each `00 00 03`
/// emulation sequence is 3 raw bytes for 2 RBSP bytes, so it is re-added here.
/// When the requested RBSP length lands mid-sequence the whole 3 bytes are
/// consumed, which only ever makes the clear leader slightly larger.
fn rbsp_len_to_raw_len(raw: &[u8], rbsp_len: usize) -> usize {
    let mut rbsp_count = 0;
    let mut i = 0;
    while i < raw.len() && rbsp_count < rbsp_len {
        if i + 3 < raw.len() && raw[i] == 0 && raw[i + 1] == 0 && raw[i + 2] == 3 && raw[i + 3] <= 3
        {
            // 00 00 03 → two RBSP bytes (00 00); consume all three raw bytes.
            rbsp_count += 2;
            i += 3;
        } else {
            rbsp_count += 1;
            i += 1;
        }
    }
    i
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
            plan(&[0u8; 4], 3, None, None),
            Err(Error::InvalidLengthSize(3))
        ));
    }

    #[test]
    fn non_vcl_only_is_fully_clear() {
        // SPS + PPS + SEI (all non-VCL) → entirely clear; no slice header needed,
        // so SPS/PPS being None is fine.
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 7, 10)); // SPS: 4+1+10 = 15
        sample.extend(lp_nal(4, 8, 4)); //  PPS: 4+1+4  = 9
        sample.extend(lp_nal(4, 6, 8)); //  SEI: 4+1+8  = 13
        // total clear = 15 + 9 + 13 = 37
        assert_eq!(
            plan(&sample, 4, None, None).unwrap(),
            vec![Subsample {
                clear: 37,
                encrypted: 0
            }]
        );
    }

    #[test]
    fn truncated_and_zero_length() {
        assert!(matches!(
            plan(&[0, 0], 4, None, None),
            Err(Error::TruncatedNal)
        ));
        assert!(matches!(
            plan(&[0, 0, 0, 0], 4, None, None),
            Err(Error::ZeroLengthNal)
        ));
        let truncated = vec![0, 0, 0, 100, 0x65, 0xAA];
        assert!(matches!(
            plan(&truncated, 4, None, None),
            Err(Error::TruncatedNal)
        ));
    }

    #[test]
    fn large_non_vcl_clear_splits_at_u16() {
        // A huge SEI (non-VCL) → clear-only, split at the u16 senc clear limit.
        let huge = u16::MAX as usize + 10;
        let sample = lp_nal(4, 6, huge); // clear = 4 + 1 + 65545 = 65550
        let plan = plan(&sample, 4, None, None).unwrap();
        assert_eq!(
            plan,
            vec![
                Subsample {
                    clear: 65535,
                    encrypted: 0
                },
                Subsample {
                    clear: 65550 - 65535,
                    encrypted: 0
                },
            ]
        );
    }
}
