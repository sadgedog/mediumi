use mediumi_mp4::{BoxHeader, BoxSize};

/// Parse the box header at `offset` within `buf[..end]`,
/// returning the header and the box's total byte size.
pub(crate) fn next_box(buf: &[u8], offset: usize, end: usize) -> Option<(BoxHeader, usize)> {
    if offset + 8 > end {
        return None;
    }
    let header = BoxHeader::parse(&buf[offset..end]).ok()?;
    let total = match header.box_size {
        BoxSize::Normal(s) => s as usize,
        BoxSize::Large(s) => s as usize,
        BoxSize::ExtendsToEnd => end - offset,
    };
    if total < header.header_size || offset + total > end {
        return None;
    }
    Some((header, total))
}

pub(crate) struct TopLevelBoxes<'a> {
    buf: &'a [u8],
    offset: usize,
}

impl<'a> TopLevelBoxes<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, offset: 0 }
    }
}

impl Iterator for TopLevelBoxes<'_> {
    /// `(header, start_offset, total_size)`
    type Item = (BoxHeader, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (header, total) = next_box(self.buf, self.offset, self.buf.len())?;
        let start = self.offset;
        self.offset += total;
        Some((header, start, total))
    }
}
