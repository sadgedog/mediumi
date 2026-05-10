use crate::boxes::{Mp4Box, error::Error, traf::Traf, trak::Trak, trex::Trex};

#[derive(Debug)]
pub struct SampleLocation {
    pub offset: u64,
    pub size: u64,
}

impl SampleLocation {
    pub fn slice<'a>(&self, data: &'a [u8]) -> Option<&'a [u8]> {
        let start = self.offset as usize;
        let end = start.checked_add(self.size as usize)?;
        if end > data.len() {
            return None;
        }
        Some(&data[start..end])
    }
}

/// `mdia.hdlr.handler_type` as a 4-byte fourcc (e.g. `b"vide"` / `b"soun"` / `b"subt"`).
pub fn handler_fourcc(trak: &Trak) -> [u8; 4] {
    trak.mdia.hdlr.handler_type.to_be_bytes()
}

/// Visit every `Trak` reachable from box list.
pub fn iter_traks(boxes: &[Mp4Box]) -> impl Iterator<Item = &Trak> {
    boxes
        .iter()
        .filter_map(|b| match b {
            Mp4Box::Moov(moov) => Some(moov.traks.iter()),
            _ => None,
        })
        .flatten()
}

/// Visit every `Traf` reachable from box list.
pub fn iter_trafs(boxes: &[Mp4Box]) -> impl Iterator<Item = (u64, &Traf)> + '_ {
    let mut moof_offset = 0u64;
    boxes
        .iter()
        .filter_map(move |b| {
            let cur = moof_offset;
            moof_offset += b.to_bytes().len() as u64;
            if let Mp4Box::Moof(moof) = b {
                Some((cur, moof))
            } else {
                None
            }
        })
        .flat_map(|(offset, moof)| moof.trafs.iter().map(move |traf| (offset, traf)))
}

/// Slice the sample bytes of a non-fragmented track out of the original mp4 file buffer.
pub fn track_samples<'a>(trak: &Trak, file: &'a [u8]) -> Result<Vec<&'a [u8]>, Error> {
    trak_sample_locations(trak)?
        .iter()
        .map(|loc| loc.slice(file).ok_or(Error::DataTooShort))
        .collect()
}

/// Slice the sample bytes of one `traf` (one fragment) out of the segment file buffer.
pub fn traf_samples<'a>(
    traf: &Traf,
    moof_file_offset: u64,
    file: &'a [u8],
    trex: Option<&Trex>,
) -> Result<Vec<&'a [u8]>, Error> {
    traf_sample_locations(traf, moof_file_offset, trex)?
        .iter()
        .map(|loc| loc.slice(file).ok_or(Error::DataTooShort))
        .collect()
}

/// Sample locations for non-fragmented mp4 mdat.
pub fn trak_sample_locations(trak: &Trak) -> Result<Vec<SampleLocation>, Error> {
    let stbl = trak
        .mdia
        .minf
        .stbl
        .as_ref()
        .ok_or(Error::MissingRequiredBox("stbl"))?;

    // Collect sample sizes from below.
    // stsz -> sample_size | entry_sizes (64 bits)
    // stz2 -> entry_sizes (4 | 8 | 16 bits)
    let sample_sizes: Vec<u64> = if let Some(stsz) = &stbl.stsz {
        if stsz.sample_size != 0 {
            vec![stsz.sample_size as u64; stsz.sample_count as usize]
        } else {
            stsz.entry_sizes.iter().map(|&s| s as u64).collect()
        }
    } else if let Some(stz2) = &stbl.stz2 {
        stz2.entry_sizes.iter().map(|&s| s as u64).collect()
    } else {
        return Err(Error::MissingRequiredBox("stsz/stz2"));
    };

    // Collect chunk offsets from below.
    // co64 -> chunk_offsets (64 bits)
    // stbl -> stco -> chunk_offsets (32 bits)
    let chunk_offsets: Vec<u64> = if let Some(co64) = &stbl.co64 {
        co64.chunk_offsets.clone()
    } else {
        stbl.stco.chunk_offsets.iter().map(|&o| o as u64).collect()
    };

    // Sample-to-chunk run-length mapping.
    let stsc_entries = &stbl.stsc.entries;
    if stsc_entries.is_empty() {
        return Err(Error::MissingRequiredBox("stsc.entries"));
    }

    let mut locations = Vec::with_capacity(sample_sizes.len());
    let mut sample_idx = 0;
    let mut chunk_idx = 0;
    let mut stsc_idx = 0;

    while sample_idx < sample_sizes.len() {
        // Sync sample-to-chunk & current chunk. (stsc.first_chunk : 1-indexed, chunk_idx: 0-indexed.)
        while stsc_idx + 1 < stsc_entries.len()
            && stsc_entries[stsc_idx + 1].first_chunk <= chunk_idx + 1
        {
            stsc_idx += 1;
        }

        // number of samples in this chunk
        let samples_per_chunk = stsc_entries[stsc_idx].samples_per_chunk as usize;

        if chunk_idx as usize >= chunk_offsets.len() {
            return Err(Error::DataTooShort);
        }
        let mut offset_in_current_chunk = chunk_offsets[chunk_idx as usize];

        for _ in 0..samples_per_chunk {
            if sample_idx >= sample_sizes.len() {
                break;
            }
            let size = sample_sizes[sample_idx];
            locations.push(SampleLocation {
                offset: offset_in_current_chunk,
                size,
            });
            offset_in_current_chunk = offset_in_current_chunk
                .checked_add(size)
                .ok_or(Error::DataTooShort)?;
            sample_idx += 1;
        }

        chunk_idx += 1;
    }

    Ok(locations)
}

const BASE_DATA_OFFSET_PRESENT: u32 = 0x000001;
const TFHD_DURATION_IS_EMPTY: u32 = 0x010000;
const TRUN_DATA_OFFSET_PRESENT: u32 = 0x000001;

/// Sample locations for fragmented mp4 mdat.
pub fn traf_sample_locations(
    traf: &Traf,
    moof_file_offset: u64,
    trex: Option<&Trex>,
) -> Result<Vec<SampleLocation>, Error> {
    let tfhd = &traf.tfhd;
    let tf_flags = tfhd.header.flags;

    if tf_flags & TFHD_DURATION_IS_EMPTY != 0 {
        return Ok(Vec::new());
    }

    let traf_base = if tf_flags & BASE_DATA_OFFSET_PRESENT != 0 {
        tfhd.base_data_offset
            .ok_or(Error::MissingRequiredBox("tfhd.base_data_offset"))?
    } else {
        moof_file_offset
    };

    let mut locations = Vec::new();
    let mut cursor = traf_base;

    for trun in &traf.truns {
        if trun.header.flags & TRUN_DATA_OFFSET_PRESENT != 0 {
            let dof = trun
                .data_offset
                .ok_or(Error::MissingRequiredBox("trun.data_offset"))?;
            cursor = if dof >= 0 {
                traf_base
                    .checked_add(dof as u64)
                    .ok_or(Error::DataTooShort)?
            } else {
                traf_base
                    .checked_sub((-(dof as i64)) as u64)
                    .ok_or(Error::DataTooShort)?
            }
        }

        for sample in &trun.samples {
            let size = sample
                .sample_size
                .or(tfhd.default_sample_size)
                .or(trex.map(|t| t.default_sample_size))
                .ok_or(Error::MissingRequiredBox("sample_size"))? as u64;
            locations.push(SampleLocation {
                offset: cursor,
                size,
            });
            cursor = cursor.checked_add(size).ok_or(Error::DataTooShort)?;
        }
    }

    Ok(locations)
}
