use crate::cenc::Subsample;
use crate::error::Error;
use mediumi_h264::nal::{NalUnit, NalUnitType};
use mediumi_h264::pps::Pps;
use mediumi_h264::slice_header::SliceHeader;
use mediumi_h264::sps::Sps;
use mediumi_util::bitstream::BitstreamReader;
use std::collections::HashMap;

/// NAL unit types carrying parameter sets.
const NAL_SPS: u8 = 7;
const NAL_PPS: u8 = 8;

fn is_vcl(nal_type: u8) -> bool {
    matches!(nal_type, 1 | 5)
}

/// The SPS/PPS available while planning a track's samples.
///
/// A slice header references a PPS by `pic_parameter_set_id`, and that PPS in
/// turn references an SPS by `seq_parameter_set_id`; both are needed to know how
/// long the slice header (the clear leader) is. The maps are seeded from `avcC`
/// and then updated in place as in-band SPS/PPS NALs (types 7/8) are encountered
/// while walking a sample. In-band sets replace same-id `avcC` entries and
/// persist across the samples of a segment, matching how `avc3`/CMAF streams
/// carry their parameter sets in the bitstream rather than only in `avcC`.
#[derive(Debug, Clone, Default)]
pub struct ParamSets {
    sps: HashMap<u32, Sps>, // keyed by seq_parameter_set_id
    pps: HashMap<u32, Pps>, // keyed by pic_parameter_set_id
}

impl ParamSets {
    /// Parse an SPS NAL body (the bytes after the NAL header byte) and store it,
    /// silently ignoring a body that fails to parse.
    pub fn ingest_sps(&mut self, nal_body: &[u8]) {
        let rbsp = NalUnit::remove_emulation_prevention_bytes(nal_body);
        if let Ok(sps) = Sps::parse(&rbsp) {
            self.sps.insert(sps.seq_parameter_set_id as u32, sps);
        }
    }

    /// Parse a PPS NAL body (the bytes after the NAL header byte) and store it.
    /// A PPS references an SPS by `seq_parameter_set_id`; if that SPS is not yet
    /// known (or the body fails to parse) the PPS is silently ignored.
    pub fn ingest_pps(&mut self, nal_body: &[u8]) {
        let rbsp = NalUnit::remove_emulation_prevention_bytes(nal_body);
        let Some(seq_id) = pps_seq_parameter_set_id(&rbsp) else {
            return;
        };
        let Some(sps) = self.sps.get(&seq_id) else {
            return;
        };
        if let Ok(pps) = Pps::parse(&rbsp, sps) {
            self.pps.insert(pps.pic_parameter_set_id, pps);
        }
    }

    /// Resolve the (SPS, PPS) pair a slice references via `pic_parameter_set_id`.
    fn select(&self, pps_id: u32) -> Option<(&Sps, &Pps)> {
        let pps = self.pps.get(&pps_id)?;
        let sps = self.sps.get(&pps.seq_parameter_set_id)?;
        Some((sps, pps))
    }
}

/// Read `seq_parameter_set_id` (the second `ue(v)` field) from a PPS RBSP so the
/// referenced SPS can be looked up before the full PPS is parsed.
fn pps_seq_parameter_set_id(rbsp: &[u8]) -> Option<u32> {
    let mut reader = BitstreamReader::new(rbsp);
    reader.read_ue().ok()?; // pic_parameter_set_id
    reader.read_ue().ok() // seq_parameter_set_id
}

pub fn plan(
    sample: &[u8],
    length_size: u8,
    params: &mut ParamSets,
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
        let nal_body = &sample[prefix_end + 1..nal_end];

        // In-band parameter sets update the working set for this and later
        // samples; they remain clear (handled by the non-VCL branch below).
        match nal_type {
            NAL_SPS => params.ingest_sps(nal_body),
            NAL_PPS => params.ingest_pps(nal_body),
            _ => {}
        }

        if is_vcl(nal_type) {
            // clear leader = NAL header byte + slice header. Everything after
            // the leader is recorded as the encrypted region, even when it is
            // shorter than one cipher block.
            let sh = slice_header_len(nal_body, nal_type, nal_ref_idc, params)
                .ok_or(Error::SliceHeaderParseFailed)?;
            let leader = 1 + sh; // NAL header byte + slice header bytes
            // [ pending clear ][ len ][ NALhdr + slice header ][ slice data ]
            //  └──────────────── clear ──────────────────────┘└ encrypted ─┘
            let clear = pending_clear + ls as u64 + leader as u64;
            let encrypted = nal_len as u64 - leader as u64;
            push_clear_split(&mut out, clear, encrypted);
            pending_clear = 0;
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

/// Slice headers are far smaller than this. Converting only a bounded prefix of
/// the NAL body to RBSP avoids copying/scanning the entire (multi-KB) frame per
/// sample just to read a few tens of header bytes.
const SLICE_HEADER_SCAN_CAP: usize = 256;

/// Measure the slice-header byte length of a VCL NAL (the clear leader that
/// follows the NAL header byte). The SPS/PPS are selected from `params` via the
/// slice's own `pic_parameter_set_id`, so an in-band PPS overriding the `avcC`
/// one is honoured.
fn slice_header_len(
    nal_body: &[u8],
    nal_type: u8,
    nal_ref_idc: u8,
    params: &ParamSets,
) -> Option<usize> {
    let measure = |rbsp: &[u8]| -> Option<usize> {
        let (sps, pps) = params.select(slice_pic_parameter_set_id(rbsp)?)?;
        let mut reader = BitstreamReader::new(rbsp);
        let nut = NalUnitType::from(nal_type);
        SliceHeader::parse(&mut reader, sps, pps, nut, nal_ref_idc).ok()?;
        let consumed_bits = rbsp.len() * 8 - reader.remaining_bits();
        Some(consumed_bits.div_ceil(8))
    };

    let cap = nal_body.len().min(SLICE_HEADER_SCAN_CAP);
    let sh_bytes_rbsp = match measure(&NalUnit::remove_emulation_prevention_bytes(
        &nal_body[..cap],
    )) {
        Some(n) => n,
        None if cap < nal_body.len() => {
            measure(&NalUnit::remove_emulation_prevention_bytes(nal_body))?
        }
        None => return None,
    };
    // Convert the RBSP length back to a length in the original NAL bytes,
    // re-adding any emulation-prevention bytes inside the slice header.
    Some(rbsp_len_to_raw_len(nal_body, sh_bytes_rbsp))
}

/// Read `pic_parameter_set_id` (the third `ue(v)` field, after
/// `first_mb_in_slice` and `slice_type`) from a slice-header RBSP.
fn slice_pic_parameter_set_id(rbsp: &[u8]) -> Option<u32> {
    let mut reader = BitstreamReader::new(rbsp);
    reader.read_ue().ok()?; // first_mb_in_slice
    reader.read_ue().ok()?; // slice_type
    reader.read_ue().ok() // pic_parameter_set_id
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
            plan(&[0u8; 4], 3, &mut ParamSets::default()),
            Err(Error::InvalidLengthSize(3))
        ));
    }

    #[test]
    fn non_vcl_only_is_fully_clear() {
        // SPS + PPS + SEI (all non-VCL) → entirely clear; no slice header needed,
        // so an empty ParamSets is fine (the SPS/PPS here are junk bodies that
        // fail to parse and are simply ignored).
        let mut sample = Vec::new();
        sample.extend(lp_nal(4, 7, 10)); // SPS: 4+1+10 = 15
        sample.extend(lp_nal(4, 8, 4)); //  PPS: 4+1+4  = 9
        sample.extend(lp_nal(4, 6, 8)); //  SEI: 4+1+8  = 13
        // total clear = 15 + 9 + 13 = 37
        assert_eq!(
            plan(&sample, 4, &mut ParamSets::default()).unwrap(),
            vec![Subsample {
                clear: 37,
                encrypted: 0
            }]
        );
    }

    #[test]
    fn truncated_and_zero_length() {
        assert!(matches!(
            plan(&[0, 0], 4, &mut ParamSets::default()),
            Err(Error::TruncatedNal)
        ));
        assert!(matches!(
            plan(&[0, 0, 0, 0], 4, &mut ParamSets::default()),
            Err(Error::ZeroLengthNal)
        ));
        let truncated = vec![0, 0, 0, 100, 0x65, 0xAA];
        assert!(matches!(
            plan(&truncated, 4, &mut ParamSets::default()),
            Err(Error::TruncatedNal)
        ));
    }

    #[test]
    fn large_non_vcl_clear_splits_at_u16() {
        // A huge SEI (non-VCL) → clear-only, split at the u16 senc clear limit.
        let huge = u16::MAX as usize + 10;
        let sample = lp_nal(4, 6, huge); // clear = 4 + 1 + 65545 = 65550
        let plan = plan(&sample, 4, &mut ParamSets::default()).unwrap();
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
