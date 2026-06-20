//! Byte-offset box walking.
//!
//! ```no_run
//! use mediumi_mp4::{walk::BoxWalker, types::BoxType};
//!
//! # fn main() -> Result<(), mediumi_mp4::Error> {
//! # let data: &[u8] = &[];
//! for info in BoxWalker::new(data) {
//!     let info = info?; // surfaces a malformed box instead of silently stopping
//!     if info.box_type == BoxType::Moov {
//!         let moov = &data[info.start_offset..info.end_offset()];
//!         // ...
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::Error;
use crate::boxes::{BoxHeader, BoxSize};
use crate::types::BoxType;

#[derive(Debug, Clone, PartialEq)]
pub struct BoxInfo {
    pub box_type: BoxType,
    pub start_offset: usize,
    pub total_size: usize,
    pub header_size: usize,
    /// `uuid` extended type.
    pub usertype: Option<[u8; 16]>,
}

impl BoxInfo {
    pub fn end_offset(&self) -> usize {
        self.start_offset + self.total_size
    }

    pub fn payload_offset(&self) -> usize {
        self.start_offset + self.header_size
    }
}

/// Parse just the box header at `offset` within `buf[..end]`,
/// returning its position and size without parsing the payload.
pub fn parse_box_info(buf: &[u8], offset: usize, end: usize) -> Result<BoxInfo, Error> {
    let slice = buf.get(offset..end).ok_or(Error::DataTooShort)?;
    let header = BoxHeader::parse(slice)?;
    let total = match header.box_size {
        BoxSize::Normal(s) => s as usize,
        BoxSize::Large(s) => s as usize,
        BoxSize::ExtendsToEnd => end - offset,
    };
    if total < header.header_size || offset + total > end {
        return Err(Error::DataTooShort);
    }
    Ok(BoxInfo {
        box_type: header.box_type,
        start_offset: offset,
        total_size: total,
        header_size: header.header_size,
        usertype: header.usertype,
    })
}

pub struct BoxWalker<'a> {
    buf: &'a [u8],
    offset: usize,
    end: usize,
}

impl<'a> BoxWalker<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            offset: 0,
            end: buf.len(),
        }
    }

    /// Walk the boxes within `[start, end)` — typically a container box's
    /// payload region, to enumerate its children.
    pub fn within(buf: &'a [u8], start: usize, end: usize) -> Self {
        Self {
            buf,
            offset: start,
            end,
        }
    }

    /// Walk the children of `parent` (the boxes filling its payload region).
    pub fn children(buf: &'a [u8], parent: &BoxInfo) -> Self {
        Self::within(buf, parent.payload_offset(), parent.end_offset())
    }
}

impl Iterator for BoxWalker<'_> {
    type Item = Result<BoxInfo, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.end {
            return None; // range fully consumed — clean stop, not an error
        }
        match parse_box_info(self.buf, self.offset, self.end) {
            Ok(info) => {
                self.offset += info.total_size;
                Some(Ok(info))
            }
            Err(e) => {
                self.offset = self.end; // surface the error once, then stop
                Some(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal box: 4-byte big-endian size + 4-byte type + padding payload.
    fn boxed(box_type: &[u8; 4], payload_len: usize) -> Vec<u8> {
        let total = 8 + payload_len;
        let mut v = (total as u32).to_be_bytes().to_vec();
        v.extend_from_slice(box_type);
        v.extend(std::iter::repeat_n(0u8, payload_len));
        v
    }

    #[test]
    fn walks_top_level_boxes_in_order() {
        let mut buf = boxed(b"ftyp", 8);
        buf.extend(boxed(b"moov", 16));
        buf.extend(boxed(b"mdat", 4));

        let infos: Vec<_> = BoxWalker::new(&buf)
            .collect::<Result<_, _>>()
            .expect("well-formed boxes");
        assert_eq!(infos.len(), 3);
        assert_eq!(infos[0].box_type, BoxType::Ftyp);
        assert_eq!(infos[0].start_offset, 0);
        assert_eq!(infos[0].total_size, 16);
        assert_eq!(infos[1].box_type, BoxType::Moov);
        assert_eq!(infos[1].start_offset, 16);
        assert_eq!(infos[1].end_offset(), 40);
        assert_eq!(infos[2].box_type, BoxType::Mdat);
        assert_eq!(infos[2].start_offset, 40);
    }

    #[test]
    fn children_walks_nested_payload() {
        // moof { traf { senc } }
        let senc = boxed(b"senc", 4);
        let mut traf_payload = senc.clone();
        let mut traf = (8 + traf_payload.len() as u32).to_be_bytes().to_vec();
        traf.extend_from_slice(b"traf");
        traf.append(&mut traf_payload);
        let mut moof = (8 + traf.len() as u32).to_be_bytes().to_vec();
        moof.extend_from_slice(b"moof");
        moof.extend_from_slice(&traf);

        let moof_info = BoxWalker::new(&moof).next().unwrap().unwrap();
        assert_eq!(moof_info.box_type, BoxType::Moof);

        let traf_info = BoxWalker::children(&moof, &moof_info)
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(traf_info.box_type, BoxType::Traf);
        assert_eq!(traf_info.start_offset, 8);

        let senc_info = BoxWalker::children(&moof, &traf_info)
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(senc_info.box_type, BoxType::Senc);
        assert_eq!(senc_info.start_offset, 16);
    }

    #[test]
    fn surfaces_truncated_trailing_box_then_stops() {
        let mut buf = boxed(b"ftyp", 8);
        // A box claiming size 100 but with only a few bytes left → an Err.
        buf.extend_from_slice(&100u32.to_be_bytes());
        buf.extend_from_slice(b"moov");

        let mut walker = BoxWalker::new(&buf);
        assert_eq!(walker.next().unwrap().unwrap().box_type, BoxType::Ftyp);
        assert_eq!(walker.next(), Some(Err(Error::DataTooShort)));
        assert!(walker.next().is_none()); // stops after the error
    }
}
