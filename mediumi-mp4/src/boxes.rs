pub mod error;
pub mod ftyp;
pub mod hdlr;
pub mod mdat;
pub mod meta;
pub mod mfhd;
pub mod moof;
pub mod mvhd;
pub mod saio;
pub mod saiz;
pub mod sbgp;
pub mod sgpd;
pub mod subs;
pub mod tfdt;
pub mod tfhd;
pub mod traf;
pub mod trun;

use crate::{
    boxes::{
        error::Error, ftyp::Ftyp, hdlr::Hdlr, mdat::Mdat, meta::Meta, mfhd::Mfhd, moof::Moof,
        mvhd::Mvhd, saio::Saio, saiz::Saiz, sbgp::Sbgp, sgpd::Sgpd, subs::Subs, tfdt::Tfdt,
        tfhd::Tfhd, traf::Traf, trun::Trun,
    },
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

pub trait BaseBox: Sized {
    const BOX_TYPE: BoxType;
    fn to_bytes(&self, writer: &mut BitstreamWriter);
    fn write_box(&self, writer: &mut BitstreamWriter) {
        write_child_box(writer, Self::BOX_TYPE, |w| self.to_bytes(w));
    }
    fn parse(data: &[u8]) -> Result<Self, Error>;
}

pub trait FullBox: BaseBox {
    fn version(&self) -> u8;
    fn flags(&self) -> u32;
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoxSize {
    Normal(u32),
    Large(u64),
    ExtendsToEnd,
}

#[derive(Debug, Clone)]
pub struct BoxHeader {
    pub box_size: BoxSize,
    pub box_type: BoxType,
    pub usertype: Option<[u8; 16]>,
    pub header_size: usize,
}

impl BoxHeader {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        match self.box_size {
            BoxSize::Normal(s) => {
                writer.write_bits(s, 32);
            }
            BoxSize::Large(_) => {
                writer.write_bits(1, 32);
            }
            BoxSize::ExtendsToEnd => {
                writer.write_bits(0, 32);
            }
        }

        let type_bytes: [u8; 4] = (&self.box_type).into();
        for b in &type_bytes {
            writer.write_bits(*b as u32, 8);
        }

        if let BoxSize::Large(s) = self.box_size {
            writer.write_bits((s >> 32) as u32, 32); // upper 32bits of largesize
            writer.write_bits(s as u32, 32); // lower 32bits of largesize
        }

        if let Some(usertype) = &self.usertype {
            for b in usertype {
                writer.write_bits(*b as u32, 8);
            }
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let size = reader.read_bits(32)?;
        let box_type = BoxType::from([
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
        ]);

        let (box_size, mut header_size) = match size {
            0 => (BoxSize::ExtendsToEnd, 8),
            1 => {
                // largesize: 64-bit size follows
                let high = reader.read_bits(32)? as u64;
                let low = reader.read_bits(32)? as u64;
                (BoxSize::Large((high << 32) | low), 16)
            }
            _ => (BoxSize::Normal(size), 8),
        };

        let usertype = if box_type == BoxType::Uuid {
            let mut ut = [0u8; 16];
            for b in ut.iter_mut() {
                *b = reader.read_bits(8)? as u8;
            }
            header_size += 16;
            Some(ut)
        } else {
            None
        };

        Ok(Self {
            box_size,
            box_type,
            usertype,
            header_size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FullBoxHeader {
    pub version: u8,
    pub flags: u32,
}

impl FullBoxHeader {
    pub fn to_bytes(&self, writer: &mut BitstreamWriter) {
        writer.write_bits(self.version as u32, 8);
        writer.write_bits(self.flags, 24);
    }

    pub fn parse(reader: &mut BitstreamReader) -> Result<Self, Error> {
        let version = reader.read_bits(8)? as u8;
        let flags = reader.read_bits(24)?;
        Ok(Self { version, flags })
    }
}

fn write_child_box<F: FnOnce(&mut BitstreamWriter)>(
    out: &mut BitstreamWriter,
    box_type: BoxType,
    body_fn: F,
) {
    let mut body_writer = BitstreamWriter::new();
    body_fn(&mut body_writer);
    let body = body_writer.finish();

    let body_len = body.len() as u64;
    let normal_total = 8_u64 + body_len;
    let header = if normal_total <= u32::MAX as u64 {
        BoxHeader {
            box_size: BoxSize::Normal(normal_total as u32),
            box_type,
            usertype: None,
            header_size: 8,
        }
    } else {
        BoxHeader {
            box_size: BoxSize::Large(16 + body_len),
            box_type,
            usertype: None,
            header_size: 16,
        }
    };
    header.to_bytes(out);
    for &b in &body {
        out.write_bits(b as u32, 8);
    }
}

/// Unknown box
#[derive(Debug)]
pub struct UnknownBox {
    pub header: BoxHeader,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum Mp4Box {
    Ftyp(Ftyp),
    Mdat(Mdat),
    Mvhd(Mvhd),
    Hdlr(Hdlr),
    Moof(Moof),
    Mfhd(Mfhd),
    Traf(Box<Traf>),
    Subs(Subs),
    Tfhd(Tfhd),
    Trun(Trun),
    Sbgp(Sbgp),
    Sgpd(Sgpd),
    Saiz(Saiz),
    Saio(Saio),
    Tfdt(Tfdt),
    Meta(Meta),
    Unknown(UnknownBox),
}

impl Mp4Box {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = BitstreamWriter::new();
        match self {
            Mp4Box::Ftyp(b) => b.write_box(&mut writer),
            Mp4Box::Mdat(b) => b.write_box(&mut writer),
            Mp4Box::Mvhd(b) => b.write_box(&mut writer),
            Mp4Box::Hdlr(b) => b.write_box(&mut writer),
            Mp4Box::Moof(b) => b.write_box(&mut writer),
            Mp4Box::Mfhd(b) => b.write_box(&mut writer),
            Mp4Box::Traf(b) => b.write_box(&mut writer),
            Mp4Box::Subs(b) => b.write_box(&mut writer),
            Mp4Box::Saiz(b) => b.write_box(&mut writer),
            Mp4Box::Saio(b) => b.write_box(&mut writer),
            Mp4Box::Tfhd(b) => b.write_box(&mut writer),
            Mp4Box::Trun(b) => b.write_box(&mut writer),
            Mp4Box::Sbgp(b) => b.write_box(&mut writer),
            Mp4Box::Sgpd(b) => b.write_box(&mut writer),
            Mp4Box::Tfdt(b) => b.write_box(&mut writer),
            Mp4Box::Meta(b) => b.write_box(&mut writer),
            Mp4Box::Unknown(u) => {
                u.header.to_bytes(&mut writer);
                for &byte in &u.payload {
                    writer.write_bits(byte as u32, 8);
                }
            }
        }
        writer.finish()
    }

    pub fn parse(data: &[u8]) -> Result<(Self, usize), Error> {
        let header = BoxHeader::parse(data)?;

        let total: usize = match header.box_size {
            BoxSize::Normal(s) => s as usize,
            BoxSize::Large(s) => s as usize,
            BoxSize::ExtendsToEnd => data.len(),
        };
        if data.len() < total {
            return Err(Error::DataTooShort);
        }
        let body = &data[header.header_size..total];

        let parsed = match &header.box_type {
            BoxType::Ftyp => Mp4Box::Ftyp(Ftyp::parse(body)?),
            BoxType::Mdat => Mp4Box::Mdat(Mdat::parse(body)?),
            BoxType::Mvhd => Mp4Box::Mvhd(Mvhd::parse(body)?),
            BoxType::Hdlr => Mp4Box::Hdlr(Hdlr::parse(body)?),
            BoxType::Moof => Mp4Box::Moof(Moof::parse(body)?),
            BoxType::Mfhd => Mp4Box::Mfhd(Mfhd::parse(body)?),
            BoxType::Traf => Mp4Box::Traf(Box::new(Traf::parse(body)?)),
            BoxType::Subs => Mp4Box::Subs(Subs::parse(body)?),
            BoxType::Saiz => Mp4Box::Saiz(Saiz::parse(body)?),
            BoxType::Saio => Mp4Box::Saio(Saio::parse(body)?),
            BoxType::Tfhd => Mp4Box::Tfhd(Tfhd::parse(body)?),
            BoxType::Trun => Mp4Box::Trun(Trun::parse(body)?),
            BoxType::Sbgp => Mp4Box::Sbgp(Sbgp::parse(body)?),
            BoxType::Sgpd => Mp4Box::Sgpd(Sgpd::parse(body)?),
            BoxType::Meta => Mp4Box::Meta(Meta::parse(body)?),
            BoxType::Tfdt => Mp4Box::Tfdt(Tfdt::parse(body)?),
            _ => Mp4Box::Unknown(UnknownBox {
                header: header.clone(),
                payload: body.to_vec(),
            }),
        };
        Ok((parsed, total))
    }
}

pub struct BoxIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> BoxIter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for BoxIter<'a> {
    type Item = Result<(Mp4Box, &'a [u8]), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        match Mp4Box::parse(&self.data[self.offset..]) {
            Ok((child, consumed)) => {
                let raw = &self.data[self.offset..self.offset + consumed];
                self.offset += consumed;
                Some(Ok((child, raw)))
            }
            Err(e) => {
                self.offset = self.data.len();
                Some(Err(e))
            }
        }
    }
}

/// Parse all box
pub fn parse_all(data: &[u8]) -> Result<Vec<Mp4Box>, Error> {
    BoxIter::new(data)
        .map(|item| item.map(|(b, _)| b))
        .collect()
}
