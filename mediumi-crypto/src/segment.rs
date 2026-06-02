//! Segment-level helpers.
//!
//! - [`enc_init_segment`] encrypts a whole init segment.
//! - [`enc_media_segment`] encrypts a whole media segment.

use crate::encrypter::Encrypter;
use crate::error::Error;
use crate::{initial, media};
use mediumi_mp4::types::BoxType;
use mediumi_mp4::{BoxHeader, BoxSize, Mp4Box, demuxer};
use std::io::Write;

pub struct MediaSegmentParts<'a> {
    pub prefix: &'a [u8],
    pub moof: &'a [u8],
    pub mdat: &'a mut [u8],
}

pub(crate) fn enc_init_segment<W: Write>(
    enc: &Encrypter,
    init_bytes: &[u8],
    out: &mut W,
) -> Result<(), Error> {
    let boxes = demuxer::demux(init_bytes)?;
    let mut found_moov = false;
    for b in &boxes {
        match b {
            Mp4Box::Moov(_) => {
                out.write_all(&initial::enc_init(enc, &b.to_bytes())?)?;
                found_moov = true;
            }
            _ => out.write_all(&b.to_bytes())?,
        }
    }
    if !found_moov {
        return Err(Error::NoMoov);
    }
    Ok(())
}

pub(crate) fn enc_media_segment<W: Write>(
    enc: &mut Encrypter,
    init_bytes: &[u8],
    media_bytes: &mut [u8],
    out: &mut W,
) -> Result<(), Error> {
    let moov = extract_moov(init_bytes)?;
    let parts = split_media_segment(media_bytes)?;
    let new_moof = media::enc_media(enc, moov, parts.moof, parts.mdat)?;

    out.write_all(parts.prefix)?;
    out.write_all(&new_moof)?;
    out.write_all(&((parts.mdat.len() + 8) as u32).to_be_bytes())?;
    out.write_all(b"mdat")?;
    out.write_all(parts.mdat)?; // streamed directly — no copy
    Ok(())
}

fn extract_moov(init: &[u8]) -> Result<&[u8], Error> {
    let mut offset = 0usize;
    while offset < init.len() {
        let header = BoxHeader::parse(&init[offset..]).map_err(Error::Mp4)?;
        let total = match header.box_size {
            BoxSize::Normal(s) => s as usize,
            BoxSize::Large(s) => s as usize,
            BoxSize::ExtendsToEnd => init.len() - offset,
        };
        if total == 0 || offset + total > init.len() {
            return Err(Error::Mp4(mediumi_mp4::Error::DataTooShort));
        }
        if header.box_type == BoxType::Moov {
            return Ok(&init[offset..offset + total]);
        }
        offset += total;
    }
    Err(Error::NoMoov)
}

pub fn split_media_segment(segment: &mut [u8]) -> Result<MediaSegmentParts<'_>, Error> {
    let mut offset = 0usize;
    let mut moof: Option<(usize, usize)> = None; // (start, total len)
    let mut mdat: Option<(usize, usize, usize)> = None; // (start, header size, total len)

    while offset < segment.len() {
        let header = BoxHeader::parse(&segment[offset..]).map_err(Error::Mp4)?;
        let total = match header.box_size {
            BoxSize::Normal(s) => s as usize,
            BoxSize::Large(s) => s as usize,
            BoxSize::ExtendsToEnd => segment.len() - offset,
        };
        if total == 0 || offset + total > segment.len() {
            return Err(Error::Mp4(mediumi_mp4::Error::DataTooShort));
        }
        match header.box_type {
            BoxType::Moof => moof = Some((offset, total)),
            BoxType::Mdat if moof.is_some() => {
                mdat = Some((offset, header.header_size, total));
                break;
            }
            _ => {}
        }
        offset += total;
    }

    let (moof_start, moof_len) = moof.ok_or(Error::NoMoof)?;
    let (mdat_start, mdat_hdr, mdat_total) = mdat.ok_or(Error::NoMdatAfterMoof)?;
    if mdat_start != moof_start + moof_len {
        return Err(Error::NoMdatAfterMoof);
    }

    // [prefix][moof][mdat header][mdat payload]
    let (prefix, rest) = segment.split_at_mut(moof_start);
    let (moof_slice, mdat_box) = rest.split_at_mut(moof_len);
    let mdat_payload = &mut mdat_box[mdat_hdr..mdat_total];
    Ok(MediaSegmentParts {
        prefix,
        moof: moof_slice,
        mdat: mdat_payload,
    })
}
