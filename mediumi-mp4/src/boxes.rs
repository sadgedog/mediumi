pub mod avcc;
pub mod co64;
pub mod cslg;
pub mod ctts;
pub mod dinf;
pub mod dref;
pub mod edts;
pub mod elng;
pub mod elst;
pub mod error;
pub mod frma;
pub mod ftyp;
pub mod hdlr;
pub mod hmhd;
pub mod leva;
pub mod mdat;
pub mod mdhd;
pub mod mdia;
pub mod mehd;
pub mod meta;
pub mod mfhd;
pub mod minf;
pub mod moof;
pub mod moov;
pub mod mvex;
pub mod mvhd;
pub mod nmhd;
pub mod padb;
pub mod pssh;
pub mod saio;
pub mod saiz;
pub mod sbgp;
pub mod schi;
pub mod schm;
pub mod sdtp;
pub mod senc;
pub mod sgpd;
pub mod sinf;
pub mod smhd;
pub mod stbl;
pub mod stco;
pub mod stdp;
pub mod sthd;
pub mod stsc;
pub mod stsd;
pub mod stsh;
pub mod stss;
pub mod stsz;
pub mod stts;
pub mod stz2;
pub mod subs;
pub mod tenc;
pub mod tfdt;
pub mod tfhd;
pub mod tkhd;
pub mod traf;
pub mod trak;
pub mod tref;
pub mod trex;
pub mod trgr;
pub mod trun;
pub mod udta;
pub mod vmhd;

use crate::{
    boxes::{
        co64::Co64, cslg::Cslg, ctts::Ctts, dinf::Dinf, dref::Dref, edts::Edts, elng::Elng,
        elst::Elst, error::Error, frma::Frma, ftyp::Ftyp, hdlr::Hdlr, hmhd::Hmhd, leva::Leva,
        mdat::Mdat, mdhd::Mdhd, mdia::Mdia, mehd::Mehd, meta::Meta, mfhd::Mfhd, minf::Minf,
        moof::Moof, moov::Moov, mvex::Mvex, mvhd::Mvhd, nmhd::Nmhd, padb::Padb, pssh::Pssh,
        saio::Saio, saiz::Saiz, sbgp::Sbgp, schi::Schi, schm::Schm, sdtp::Sdtp, senc::Senc,
        sgpd::Sgpd, sinf::Sinf, smhd::Smhd, stbl::Stbl, stco::Stco, stdp::Stdp, sthd::Sthd,
        stsc::Stsc, stsd::Stsd, stsh::Stsh, stss::Stss, stsz::Stsz, stts::Stts, stz2::Stz2,
        subs::Subs, tenc::Tenc, tfdt::Tfdt, tfhd::Tfhd, tkhd::Tkhd, traf::Traf, trak::Trak,
        tref::Tref, trex::Trex, trgr::Trgr, trun::Trun, udta::Udta, vmhd::Vmhd,
    },
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

pub trait BaseBox: Sized {
    const BOX_TYPE: BoxType;
    fn to_bytes(&self, writer: &mut ByteWriter);
    fn write_box(&self, writer: &mut ByteWriter) {
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
    pub fn to_bytes(&self, writer: &mut ByteWriter) {
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
        writer.write_bytes(&type_bytes);

        if let BoxSize::Large(s) = self.box_size {
            writer.write_u64(s); // 64-bit largesize
        }

        if let Some(usertype) = &self.usertype {
            writer.write_bytes(usertype);
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
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

#[derive(Debug, Clone, PartialEq)]
pub struct FullBoxHeader {
    pub version: u8,
    pub flags: u32,
}

impl FullBoxHeader {
    pub fn to_bytes(&self, writer: &mut ByteWriter) {
        writer.write_bits(self.version as u32, 8);
        writer.write_bits(self.flags, 24);
    }

    pub fn parse(reader: &mut ByteReader) -> Result<Self, Error> {
        let version = reader.read_bits(8)? as u8;
        let flags = reader.read_bits(24)?;
        Ok(Self { version, flags })
    }
}

fn write_child_box<F: FnOnce(&mut ByteWriter)>(
    out: &mut ByteWriter,
    box_type: BoxType,
    body_fn: F,
) {
    // Write the header with a placeholder size and the body directly into
    // `out`, then backpatch the size once the body length is known.
    let start = out.position();
    out.write_u32(0); // dummy box size
    let type_bytes: [u8; 4] = (&box_type).into();
    out.write_bytes(&type_bytes);

    body_fn(out);

    let total = out.position() - start;
    if total <= u32::MAX as usize {
        out.patch_u32(start, total as u32);
    } else {
        // Promote to a 64-bit largesize header: size field = 1, 8-byte largesize
        // inserted after the type. The box grows by 8 bytes.
        out.patch_u32(start, 1);
        out.insert_bytes(start + 8, &(total as u64 + 8).to_be_bytes());
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
    Moov(Box<Moov>),
    Mvhd(Mvhd),
    Trak(Box<Trak>),
    Tkhd(Tkhd),
    Tref(Tref),
    Trgr(Trgr),
    Mdia(Box<Mdia>),
    Mdhd(Mdhd),
    Hdlr(Hdlr),
    Minf(Box<Minf>),
    Nmhd(Nmhd),
    Elng(Elng),
    Stbl(Box<Stbl>),
    Stsd(Stsd),
    Stts(Stts),
    Ctts(Ctts),
    Cslg(Cslg),
    Stss(Stss),
    Stsh(Stsh),
    Edts(Edts),
    Sdtp(Sdtp),
    Elst(Elst),
    Dinf(Dinf),
    Dref(Dref),
    Stsz(Stsz),
    Stz2(Stz2),
    Stsc(Stsc),
    Stco(Stco),
    Co64(Co64),
    Padb(Padb),
    Stdp(Stdp),
    Subs(Subs),
    Saiz(Saiz),
    Saio(Saio),
    Mvex(Mvex),
    Mehd(Mehd),
    Trex(Trex),
    Moof(Moof),
    Mfhd(Mfhd),
    Traf(Box<Traf>),
    Tfhd(Tfhd),
    Trun(Trun),
    Tfdt(Tfdt),
    Leva(Leva),
    Sbgp(Sbgp),
    Sgpd(Sgpd),
    Udta(Udta),
    Meta(Meta),
    Vmhd(Vmhd),
    Smhd(Smhd),
    Hmhd(Hmhd),
    Sthd(Sthd),
    Pssh(Pssh),
    Sinf(Sinf),
    Frma(Frma),
    Schm(Schm),
    Schi(Schi),
    Tenc(Tenc),
    Senc(Senc),
    Unknown(UnknownBox),
}

impl Mp4Box {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = ByteWriter::new();
        self.write_into(&mut writer);
        writer.finish()
    }

    pub fn write_into(&self, writer: &mut ByteWriter) {
        match self {
            Mp4Box::Ftyp(b) => b.write_box(writer),
            Mp4Box::Mdat(b) => b.write_box(writer),
            Mp4Box::Moov(b) => b.write_box(writer),
            Mp4Box::Mvhd(b) => b.write_box(writer),
            Mp4Box::Trak(b) => b.write_box(writer),
            Mp4Box::Tkhd(b) => b.write_box(writer),
            Mp4Box::Tref(b) => b.write_box(writer),
            Mp4Box::Trgr(b) => b.write_box(writer),
            Mp4Box::Mdia(b) => b.write_box(writer),
            Mp4Box::Mdhd(b) => b.write_box(writer),
            Mp4Box::Hdlr(b) => b.write_box(writer),
            Mp4Box::Minf(b) => b.write_box(writer),
            Mp4Box::Nmhd(b) => b.write_box(writer),
            Mp4Box::Elng(b) => b.write_box(writer),
            Mp4Box::Stbl(b) => b.write_box(writer),
            Mp4Box::Stsd(b) => b.write_box(writer),
            Mp4Box::Stts(b) => b.write_box(writer),
            Mp4Box::Ctts(b) => b.write_box(writer),
            Mp4Box::Cslg(b) => b.write_box(writer),
            Mp4Box::Stss(b) => b.write_box(writer),
            Mp4Box::Stsh(b) => b.write_box(writer),
            Mp4Box::Edts(b) => b.write_box(writer),
            Mp4Box::Sdtp(b) => b.write_box(writer),
            Mp4Box::Elst(b) => b.write_box(writer),
            Mp4Box::Dinf(b) => b.write_box(writer),
            Mp4Box::Dref(b) => b.write_box(writer),
            Mp4Box::Stsz(b) => b.write_box(writer),
            Mp4Box::Stz2(b) => b.write_box(writer),
            Mp4Box::Stsc(b) => b.write_box(writer),
            Mp4Box::Stco(b) => b.write_box(writer),
            Mp4Box::Co64(b) => b.write_box(writer),
            Mp4Box::Padb(b) => b.write_box(writer),
            Mp4Box::Stdp(b) => b.write_box(writer),
            Mp4Box::Subs(b) => b.write_box(writer),
            Mp4Box::Saiz(b) => b.write_box(writer),
            Mp4Box::Saio(b) => b.write_box(writer),
            Mp4Box::Mvex(b) => b.write_box(writer),
            Mp4Box::Mehd(b) => b.write_box(writer),
            Mp4Box::Trex(b) => b.write_box(writer),
            Mp4Box::Moof(b) => b.write_box(writer),
            Mp4Box::Mfhd(b) => b.write_box(writer),
            Mp4Box::Traf(b) => b.write_box(writer),
            Mp4Box::Tfhd(b) => b.write_box(writer),
            Mp4Box::Trun(b) => b.write_box(writer),
            Mp4Box::Tfdt(b) => b.write_box(writer),
            Mp4Box::Leva(b) => b.write_box(writer),
            Mp4Box::Sbgp(b) => b.write_box(writer),
            Mp4Box::Sgpd(b) => b.write_box(writer),
            Mp4Box::Udta(b) => b.write_box(writer),
            Mp4Box::Meta(b) => b.write_box(writer),
            Mp4Box::Vmhd(b) => b.write_box(writer),
            Mp4Box::Smhd(b) => b.write_box(writer),
            Mp4Box::Hmhd(b) => b.write_box(writer),
            Mp4Box::Sthd(b) => b.write_box(writer),
            Mp4Box::Pssh(b) => b.write_box(writer),
            Mp4Box::Sinf(b) => b.write_box(writer),
            Mp4Box::Frma(b) => b.write_box(writer),
            Mp4Box::Schm(b) => b.write_box(writer),
            Mp4Box::Schi(b) => b.write_box(writer),
            Mp4Box::Tenc(b) => b.write_box(writer),
            Mp4Box::Senc(b) => b.write_box(writer),
            Mp4Box::Unknown(u) => {
                u.header.to_bytes(writer);
                writer.write_bytes(&u.payload);
            }
        }
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
            BoxType::Moov => Mp4Box::Moov(Box::new(Moov::parse(body)?)),
            BoxType::Mvhd => Mp4Box::Mvhd(Mvhd::parse(body)?),
            BoxType::Trak => Mp4Box::Trak(Box::new(Trak::parse(body)?)),
            BoxType::Tkhd => Mp4Box::Tkhd(Tkhd::parse(body)?),
            BoxType::Tref => Mp4Box::Tref(Tref::parse(body)?),
            BoxType::Trgr => Mp4Box::Trgr(Trgr::parse(body)?),
            BoxType::Mdia => Mp4Box::Mdia(Box::new(Mdia::parse(body)?)),
            BoxType::Mdhd => Mp4Box::Mdhd(Mdhd::parse(body)?),
            BoxType::Hdlr => Mp4Box::Hdlr(Hdlr::parse(body)?),
            BoxType::Minf => Mp4Box::Minf(Box::new(Minf::parse(body)?)),
            BoxType::Nmhd => Mp4Box::Nmhd(Nmhd::parse(body)?),
            BoxType::Elng => Mp4Box::Elng(Elng::parse(body)?),
            BoxType::Stbl => Mp4Box::Stbl(Box::new(Stbl::parse(body)?)),
            BoxType::Stsd => Mp4Box::Stsd(Stsd::parse(body)?),
            BoxType::Stts => Mp4Box::Stts(Stts::parse(body)?),
            BoxType::Ctts => Mp4Box::Ctts(Ctts::parse(body)?),
            BoxType::Cslg => Mp4Box::Cslg(Cslg::parse(body)?),
            BoxType::Stss => Mp4Box::Stss(Stss::parse(body)?),
            BoxType::Stsh => Mp4Box::Stsh(Stsh::parse(body)?),
            BoxType::Edts => Mp4Box::Edts(Edts::parse(body)?),
            BoxType::Sdtp => Mp4Box::Sdtp(Sdtp::parse(body)?),
            BoxType::Elst => Mp4Box::Elst(Elst::parse(body)?),
            BoxType::Dinf => Mp4Box::Dinf(Dinf::parse(body)?),
            BoxType::Dref => Mp4Box::Dref(Dref::parse(body)?),
            BoxType::Stsz => Mp4Box::Stsz(Stsz::parse(body)?),
            BoxType::Stz2 => Mp4Box::Stz2(Stz2::parse(body)?),
            BoxType::Stsc => Mp4Box::Stsc(Stsc::parse(body)?),
            BoxType::Stco => Mp4Box::Stco(Stco::parse(body)?),
            BoxType::Co64 => Mp4Box::Co64(Co64::parse(body)?),
            BoxType::Padb => Mp4Box::Padb(Padb::parse(body)?),
            BoxType::Stdp => Mp4Box::Stdp(Stdp::parse(body)?),
            BoxType::Subs => Mp4Box::Subs(Subs::parse(body)?),
            BoxType::Saiz => Mp4Box::Saiz(Saiz::parse(body)?),
            BoxType::Saio => Mp4Box::Saio(Saio::parse(body)?),
            BoxType::Mvex => Mp4Box::Mvex(Mvex::parse(body)?),
            BoxType::Mehd => Mp4Box::Mehd(Mehd::parse(body)?),
            BoxType::Trex => Mp4Box::Trex(Trex::parse(body)?),
            BoxType::Moof => Mp4Box::Moof(Moof::parse(body)?),
            BoxType::Mfhd => Mp4Box::Mfhd(Mfhd::parse(body)?),
            BoxType::Traf => Mp4Box::Traf(Box::new(Traf::parse(body)?)),
            BoxType::Tfhd => Mp4Box::Tfhd(Tfhd::parse(body)?),
            BoxType::Trun => Mp4Box::Trun(Trun::parse(body)?),
            BoxType::Tfdt => Mp4Box::Tfdt(Tfdt::parse(body)?),
            BoxType::Leva => Mp4Box::Leva(Leva::parse(body)?),
            BoxType::Sbgp => Mp4Box::Sbgp(Sbgp::parse(body)?),
            BoxType::Sgpd => Mp4Box::Sgpd(Sgpd::parse(body)?),
            BoxType::Udta => Mp4Box::Udta(Udta::parse(body)?),
            BoxType::Meta => Mp4Box::Meta(Meta::parse(body)?),
            BoxType::Vmhd => Mp4Box::Vmhd(Vmhd::parse(body)?),
            BoxType::Smhd => Mp4Box::Smhd(Smhd::parse(body)?),
            BoxType::Hmhd => Mp4Box::Hmhd(Hmhd::parse(body)?),
            BoxType::Sthd => Mp4Box::Sthd(Sthd::parse(body)?),
            BoxType::Pssh => Mp4Box::Pssh(Pssh::parse(body)?),
            BoxType::Sinf => Mp4Box::Sinf(Sinf::parse(body)?),
            BoxType::Frma => Mp4Box::Frma(Frma::parse(body)?),
            BoxType::Schm => Mp4Box::Schm(Schm::parse(body)?),
            BoxType::Schi => Mp4Box::Schi(Schi::parse(body)?),
            BoxType::Tenc => Mp4Box::Tenc(Tenc::parse(body)?),
            BoxType::Senc => Mp4Box::Senc(Senc::parse(body)?),
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

/// Serialize all box
pub fn to_bytes_all(boxes: &[Mp4Box]) -> Vec<u8> {
    let mut writer = ByteWriter::new();
    for b in boxes {
        b.write_into(&mut writer);
    }
    writer.finish()
}
