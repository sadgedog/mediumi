use crate::error::Error;
use crate::subsample::{CodecKind, ParamSets};
use mediumi_mp4::{Mp4Box, find_codec_config, iter_traks};
use std::collections::HashMap;

/// Build `track_id -> CodecKind` from the moov's traks. Tracks with an
/// unsupported sample entry will be skipped. AVC tracks carry their parsed SPS/PPS
/// (from avcC) so the subsample planner can measure slice-header lengths.
pub(crate) fn build_track_table(moov_boxes: &[Mp4Box]) -> Result<HashMap<u32, CodecKind>, Error> {
    let mut tracks = HashMap::new();
    for trak in iter_traks(moov_boxes) {
        let Some(stbl) = trak.mdia.minf.stbl.as_ref() else {
            continue;
        };
        let Some(entry) = stbl.stsd.entries.first() else {
            continue;
        };
        let codec = match &entry.box_type {
            b"avc1" | b"avc3" | b"encv" => {
                let cfg = find_codec_config(trak, b"avcC").ok_or(Error::MissingAvcc)?;
                // avcC: cfg_version(1) profile(1) compat(1) level(1)
                //       | 6-bit reserved + 2-bit length_size_minus_one
                if cfg.len() < 5 {
                    return Err(Error::MissingAvcc);
                }
                CodecKind::Avc {
                    length_size: (cfg[4] & 0b0000_0011) + 1,
                    params: parse_avcc_params(cfg),
                }
            }
            b"mp4a" | b"enca" => CodecKind::Mp4a,
            _ => continue,
        };
        tracks.insert(trak.tkhd.track_id, codec);
    }
    Ok(tracks)
}

/// Seed a [`ParamSets`] with every SPS and PPS carried in an `avcC` record.
/// Entries that fail to parse are skipped; a malformed record just yields fewer
/// (or no) parameter sets, and any missing set is later supplied in-band or, if
/// never available, surfaces as a slice-header parse error at plan time.
fn parse_avcc_params(avcc: &[u8]) -> ParamSets {
    let mut params = ParamSets::default();
    // avcC: configVer(1) profile(1) compat(1) level(1) lengthSizeM1(1)
    //       numSPS(1) [len(2) SPS NAL]... numPPS(1) [len(2) PPS NAL]...
    let Some(()) = walk_avcc(avcc, &mut params) else {
        return params;
    };
    params
}

/// Walk the SPS then PPS lists of an `avcC` record, ingesting each NAL body.
/// Returns `None` on a truncated record (partial ingest is kept).
fn walk_avcc(avcc: &[u8], params: &mut ParamSets) -> Option<()> {
    if avcc.len() < 6 {
        return None;
    }
    let num_sps = avcc[5] & 0b0001_1111;
    let mut p = 6usize;
    for _ in 0..num_sps {
        let len = u16::from_be_bytes([*avcc.get(p)?, *avcc.get(p + 1)?]) as usize;
        p += 2;
        let nal = avcc.get(p..p + len)?;
        p += len;
        if nal.len() > 1 {
            params.ingest_sps(&nal[1..]);
        }
    }
    let num_pps = *avcc.get(p)?;
    p += 1;
    for _ in 0..num_pps {
        let len = u16::from_be_bytes([*avcc.get(p)?, *avcc.get(p + 1)?]) as usize;
        p += 2;
        let nal = avcc.get(p..p + len)?;
        p += len;
        if nal.len() > 1 {
            params.ingest_pps(&nal[1..]);
        }
    }
    Some(())
}
