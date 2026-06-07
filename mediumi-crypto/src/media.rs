use crate::box_walk::next_box;
use crate::cbc::{Aes128CbcPatternCipher, apply_cbcs_subsamples};
use crate::cenc::{apply_cenc_subsamples, derive_per_sample_iv, encrypted_block_count};
use crate::encrypter::{Encrypter, Iv, Mode};
use crate::error::Error;
use crate::senc::{build_senc_entry_cbcs, build_senc_entry_cenc, senc_entry_size};
use crate::subsample::{self, CodecKind};
use crate::track::build_track_table;
use mediumi_mp4::boxes::{
    FullBoxHeader,
    saio::Saio,
    saiz::Saiz,
    senc::{SENC_FLAG_USE_SUBSAMPLES, Senc},
    traf::Traf,
};
use mediumi_mp4::{BoxHeader, Mp4Box, demuxer, types::BoxType};

pub(crate) fn enc_media(
    enc: &mut Encrypter,
    moov_bytes: &[u8],
    moof_bytes: &[u8],
    mdat: &mut [u8],
    mdat_header_size: usize,
) -> Result<Vec<u8>, Error> {
    let moov_boxes = demuxer::demux(moov_bytes)?;
    let tracks = build_track_table(&moov_boxes)?;

    let mut moof_boxes = demuxer::demux(moof_bytes)?;
    let moof_idx = moof_boxes
        .iter()
        .position(|b| matches!(b, Mp4Box::Moof(_)))
        .ok_or(Error::NoMoof)?;

    let mut trafs_with_senc: Vec<usize> = Vec::new();

    let mdat_payload_pos = moof_bytes.len() as u64 + mdat_header_size as u64;

    {
        let Mp4Box::Moof(moof) = &mut moof_boxes[moof_idx] else {
            return Err(Error::NoMoof);
        };
        for (traf_idx, traf) in moof.trafs.iter_mut().enumerate() {
            let Some(codec) = tracks.get(&traf.tfhd.track_id) else {
                continue;
            };
            let ranges = sample_ranges(traf, mdat_payload_pos)?;
            if ranges.is_empty() {
                continue;
            }

            let iv = enc.mode.iv();
            let cbcs = match enc.mode {
                Mode::Cbcs { .. } => {
                    let (crypt_bb, skip_bb) = codec.cbcs_pattern();
                    Some((Aes128CbcPatternCipher::new(&enc.key), crypt_bb, skip_bb))
                }
                Mode::Cenc { .. } => None,
            };

            // cbcs full-sample audio (Mp4a) is decryptable from the tenc constant
            // IV alone, so it carries no senc/saiz/saio: encrypt in place but emit
            // no auxiliary boxes. Everything else (all cenc, cbcs video) needs senc.
            let emit_senc = !(cbcs.is_some() && matches!(codec, CodecKind::Mp4a));

            let mut senc_entries = Vec::with_capacity(ranges.len());
            let mut saiz_sizes = Vec::with_capacity(ranges.len());
            let mut use_subsamples = false;

            for (offset, size) in &ranges {
                let end = offset + size;
                if end > mdat.len() {
                    return Err(Error::SampleOutOfBounds {
                        end,
                        mdat_len: mdat.len(),
                    });
                }
                let sample = &mut mdat[*offset..end];
                let subs = subsample::plan(codec, sample)?;

                let entry = match &cbcs {
                    Some((cipher, crypt_bb, skip_bb)) => {
                        apply_cbcs_subsamples(
                            cipher,
                            &iv.to_block16(),
                            *crypt_bb,
                            *skip_bb,
                            &subs,
                            sample,
                        )?;
                        build_senc_entry_cbcs(&subs)
                    }
                    None => {
                        let iv16 =
                            derive_per_sample_iv(&iv, enc.next_sample_index, enc.next_block_offset);
                        apply_cenc_subsamples(&enc.key, &iv16, &subs, sample)?;
                        if matches!(iv, Iv::Bytes16(_)) {
                            enc.next_block_offset += encrypted_block_count(&subs, sample.len());
                        }
                        build_senc_entry_cenc(&iv, &iv16, &subs)
                    }
                };
                enc.next_sample_index += 1;

                if !emit_senc {
                    continue;
                }
                if !subs.is_empty() {
                    use_subsamples = true;
                }
                let entry_size = senc_entry_size(&entry);
                if entry_size > u8::MAX as usize {
                    return Err(Error::SencEntryTooLarge(entry_size));
                }
                saiz_sizes.push(entry_size as u8);
                senc_entries.push(entry);
            }

            if !emit_senc {
                continue; // cbcs audio: encrypted, no auxiliary boxes
            }

            let sample_count = senc_entries.len() as u32;
            let flags = if use_subsamples {
                SENC_FLAG_USE_SUBSAMPLES
            } else {
                0
            };
            traf.sencs.push(Senc {
                header: FullBoxHeader { version: 0, flags },
                entries: senc_entries,
            });
            traf.saizs.push(Saiz {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                aux_info_type: None,
                aux_info_type_parameter: None,
                default_sample_info_size: 0,
                sample_count,
                sample_info_sizes: saiz_sizes,
            });
            traf.saios.push(Saio {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                aux_info_type: None,
                aux_info_type_parameter: None,
                entry_count: 1,
                offset: vec![0], // placeholder, patched below
            });
            trafs_with_senc.push(traf_idx);
        }
    }

    if trafs_with_senc.is_empty() {
        // Nothing matched (no encryptable track) — return moof unchanged.
        return Ok(moof_boxes[moof_idx].to_bytes());
    }

    // Serialize to learn where each senc lands within the moof.
    let pass1 = moof_boxes[moof_idx].to_bytes();
    let senc_offsets = scan_senc_box_offsets(&pass1);
    if senc_offsets.len() != trafs_with_senc.len() {
        return Err(Error::SaioFixupFailed);
    }

    // Adding senc/saiz/saio grew the moof, which pushes the mdat (and thus every
    // sample) further from the moof start. `trun.data_offset` is measured from
    // the moof start, so shift each one by the growth.
    let delta = (pass1.len() as i64 - moof_bytes.len() as i64) as i32;
    {
        let Mp4Box::Moof(moof) = &mut moof_boxes[moof_idx] else {
            return Err(Error::NoMoof);
        };
        for traf in &mut moof.trafs {
            for trun in &mut traf.truns {
                if let Some(offset) = trun.data_offset {
                    trun.data_offset = Some(offset + delta);
                }
            }
        }
        // Patch each saio.offset to the first aux-info byte of the matching
        // senc: senc_box_start + 8 (box header) + 4 (FullBoxHeader) + 4 (sample_count).
        // This offset is moof-relative (NOT segment/file absolute)
        for (i, &traf_idx) in trafs_with_senc.iter().enumerate() {
            let payload_offset = (senc_offsets[i] + 16) as u64;
            if let Some(saio) = moof.trafs[traf_idx].saios.last_mut() {
                saio.offset = vec![payload_offset];
            }
        }
    }

    // Re-serialize with correct offsets (box sizes unchanged).
    Ok(moof_boxes[moof_idx].to_bytes())
}

/// Sample byte ranges within the mdat payload, accumulated from trun.
///
///   [moof[ traf(video)  traf(audio) ]]  [mdat[ video │ audio ]]
///             │              │                  ▲        ▲
///             │              └ data_offset ─────│────────┘
///             └ data_offset ────────────────────┘
///
///   offset in mdat = data_offset − mdat_payload_pos   (= moof size + mdat header)
fn sample_ranges(traf: &Traf, mdat_payload_pos: u64) -> Result<Vec<(usize, usize)>, Error> {
    let base = traf.tfhd.base_data_offset.unwrap_or(0);

    let mut out = Vec::new();
    // Running absolute position (moof-relative) for truns that omit data_offset.
    let mut running_abs = base;
    for trun in &traf.truns {
        let abs_start = match trun.data_offset {
            Some(off) => base
                .checked_add_signed(off as i64)
                .ok_or(Error::SampleOutOfBounds {
                    end: 0,
                    mdat_len: mdat_payload_pos as usize,
                })?,
            None => running_abs,
        };
        let mut cursor =
            abs_start
                .checked_sub(mdat_payload_pos)
                .ok_or(Error::SampleOutOfBounds {
                    end: abs_start as usize,
                    mdat_len: mdat_payload_pos as usize,
                })? as usize;

        for sample in &trun.samples {
            let size = sample
                .sample_size
                .or(traf.tfhd.default_sample_size)
                .ok_or(Error::MissingSampleSize)? as usize;
            out.push((cursor, size));
            cursor += size;
        }
        running_abs = mdat_payload_pos + cursor as u64;
    }
    Ok(out)
}

/// Find each `senc` box's start offset by walking the serialized moof's box tree
/// moof → traf → senc
fn scan_senc_box_offsets(buf: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let Ok(moof_hdr) = BoxHeader::parse(buf) else {
        return out;
    };
    // moof's children → find each traf.
    let mut offset = moof_hdr.header_size;
    while let Some((header, total)) = next_box(buf, offset, buf.len()) {
        if header.box_type == BoxType::Traf {
            // this traf's children → find its senc.
            let traf_end = offset + total;
            let mut inner = offset + header.header_size;
            while let Some((ih, itotal)) = next_box(buf, inner, traf_end) {
                if ih.box_type == BoxType::Senc {
                    out.push(inner);
                }
                inner += itotal;
            }
        }
        offset += total;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mediumi_mp4::boxes::{
        tfhd::Tfhd,
        trun::{Trun, TrunSample},
    };

    /// A traf with a single trun whose samples start at `data_offset`
    /// (moof-relative). `sizes` are per-sample sizes (None → default).
    fn traf_with(
        track_id: u32,
        data_offset: i32,
        default_sample_size: Option<u32>,
        sizes: &[Option<u32>],
    ) -> Traf {
        Traf {
            tfhd: Tfhd {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                track_id,
                base_data_offset: None,
                sample_description_index: None,
                default_sample_duration: None,
                default_sample_size,
                default_sample_flags: None,
            },
            truns: vec![Trun {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                sample_count: sizes.len() as u32,
                data_offset: Some(data_offset),
                first_sample_flags: None,
                samples: sizes
                    .iter()
                    .map(|&s| TrunSample {
                        sample_duration: None,
                        sample_size: s,
                        sample_flags: None,
                        sample_composition_time_offset: None,
                    })
                    .collect(),
            }],
            sbgps: Vec::new(),
            sgpds: Vec::new(),
            subs: Vec::new(),
            sencs: Vec::new(),
            saizs: Vec::new(),
            saios: Vec::new(),
            tfdt: None,
            meta: None,
            others: Vec::new(),
        }
    }

    // moof size 500 + mdat box header 8 → mdat payload begins 508 from moof start.
    const PAYLOAD_POS: u64 = 508;

    #[test]
    fn single_track_starts_at_payload_front() {
        // The sole trun points at the mdat payload front → offsets start at 0
        // (identical to the previous cursor=0 behaviour).
        let traf = traf_with(1, PAYLOAD_POS as i32, None, &[Some(100), Some(200)]);
        let ranges = sample_ranges(&traf, PAYLOAD_POS).unwrap();
        assert_eq!(ranges, vec![(0, 100), (100, 200)]);
    }

    #[test]
    fn muxed_tracks_land_at_distinct_offsets() {
        // Video occupies mdat[0..300], audio follows at mdat[300..].
        let video = traf_with(1, PAYLOAD_POS as i32, None, &[Some(100), Some(200)]);
        let audio = traf_with(2, (PAYLOAD_POS + 300) as i32, None, &[Some(50)]);

        let v = sample_ranges(&video, PAYLOAD_POS).unwrap();
        let a = sample_ranges(&audio, PAYLOAD_POS).unwrap();

        assert_eq!(v, vec![(0, 100), (100, 200)]); // mdat[0..300]
        assert_eq!(a, vec![(300, 50)]); // mdat[300..350] — no overlap with video
    }

    #[test]
    fn falls_back_to_default_sample_size() {
        let traf = traf_with(1, PAYLOAD_POS as i32, Some(64), &[None, None]);
        let ranges = sample_ranges(&traf, PAYLOAD_POS).unwrap();
        assert_eq!(ranges, vec![(0, 64), (64, 64)]);
    }

    #[test]
    fn rejects_offset_before_payload() {
        // data_offset pointing before the mdat payload start is rejected rather
        // than silently underflowing.
        let traf = traf_with(1, (PAYLOAD_POS - 100) as i32, None, &[Some(10)]);
        assert!(matches!(
            sample_ranges(&traf, PAYLOAD_POS),
            Err(Error::SampleOutOfBounds { .. })
        ));
    }
}
