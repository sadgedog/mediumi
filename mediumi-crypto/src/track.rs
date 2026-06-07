use crate::error::Error;
use crate::subsample::CodecKind;
use mediumi_h264::{nal::NalUnit, pps::Pps, sps::Sps};
use mediumi_mp4::{Mp4Box, find_codec_config, iter_traks};
use std::collections::HashMap;

/// Build `track_id -> CodecKind` from the moov's traks. Tracks with an
/// unsupported sample entry are skipped. AVC tracks carry their parsed SPS/PPS
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
                let (sps, pps) = parse_avcc_sps_pps(cfg);
                CodecKind::Avc {
                    length_size: (cfg[4] & 0b0000_0011) + 1,
                    sps,
                    pps,
                }
            }
            b"mp4a" | b"enca" => CodecKind::Mp4a,
            _ => continue,
        };
        tracks.insert(trak.tkhd.track_id, codec);
    }
    Ok(tracks)
}

/// Extract and parse the first SPS and PPS from an `avcC` configuration record.
/// Returns `(None, None)` if the record is malformed or parsing fails, so the
/// AVC planner falls back to a fixed clear leader instead of erroring.
fn parse_avcc_sps_pps(avcc: &[u8]) -> (Option<Box<Sps>>, Option<Box<Pps>>) {
    fn inner(avcc: &[u8]) -> Option<(Box<Sps>, Box<Pps>)> {
        // avcC: configVer(1) profile(1) compat(1) level(1) lengthSizeM1(1)
        //       numSPS(1) [len(2) SPS NAL]... numPPS(1) [len(2) PPS NAL]...
        if avcc.len() < 6 {
            return None;
        }
        let num_sps = avcc[5] & 0x1f;
        let mut p = 6usize;
        let mut sps: Option<Sps> = None;
        for _ in 0..num_sps {
            let len = u16::from_be_bytes([*avcc.get(p)?, *avcc.get(p + 1)?]) as usize;
            p += 2;
            let nal = avcc.get(p..p + len)?;
            p += len;
            if sps.is_none() && nal.len() > 1 {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal[1..]);
                sps = Sps::parse(&rbsp).ok();
            }
        }
        let sps = sps?;
        let num_pps = *avcc.get(p)?;
        p += 1;
        let mut pps: Option<Pps> = None;
        for _ in 0..num_pps {
            let len = u16::from_be_bytes([*avcc.get(p)?, *avcc.get(p + 1)?]) as usize;
            p += 2;
            let nal = avcc.get(p..p + len)?;
            p += len;
            if pps.is_none() && nal.len() > 1 {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal[1..]);
                pps = Pps::parse(&rbsp, &sps).ok();
            }
        }
        Some((Box::new(sps), Box::new(pps?)))
    }
    match inner(avcc) {
        Some((sps, pps)) => (Some(sps), Some(pps)),
        None => (None, None),
    }
}
