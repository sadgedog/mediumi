//! ISO-BMFF box definitions.

pub mod error;

pub use error::Error;

use crate::util::bitstream::{BitstreamReader, BitstreamWriter};

#[derive(Debug, PartialEq)]
pub enum BoxType {
    Ftyp,
    Pdin,
    Moov,
    Mvhd,
    Meta,
    Trak,
    Tkhd,
    Tref,
    Trgr,
    Edts,
    Elst,
    Mdia,
    Mdhd,
    Hdlr,
    Elng,
    Minf,
    Vmhd,
    Smhd,
    Hmhd,
    Sthd,
    Nmhd,
    Dinf,
    Dref,
    Stbl,
    Stsd,
    Stts,
    Ctts,
    Cslg,
    Stsc,
    Stsz,
    Stz2,
    Stco,
    Co64,
    Stss,
    Stsh,
    Padb,
    Stdp,
    Sdtp,
    Sbgp,
    Sgpd,
    Subs,
    Saiz,
    Saio,
    Udta,
    Mvex,
    Mehd,
    Trex,
    Leva,
    Moof,
    Mfhd,
    Traf,
    Tfhd,
    Trun,
    Tfdt,
    Mfra,
    Tfra,
    Mfro,
    Mdat,
    Free,
    Skip,
    Cprt,
    Tsel,
    Strk,
    Stri,
    Strd,
    Iloc,
    Ipro,
    Sinf,
    Frma,
    Schm,
    Schi,
    Iinf,
    Xml,
    Bxml,
    Pitm,
    Fiin,
    Paen,
    Fire,
    Fpar,
    Fecr,
    Segr,
    Gitn,
    Idat,
    Iref,
    Meco,
    Mere,
    Styp,
    Sidx,
    Ssix,
    Prft,
    Unknown([u8; 4]),
}

impl From<[u8; 4]> for BoxType {
    fn from(value: [u8; 4]) -> Self {
        match &value {
            b"ftyp" => BoxType::Ftyp,
            b"pdin" => BoxType::Pdin,
            b"moov" => BoxType::Moov,
            b"mvhd" => BoxType::Mvhd,
            b"meta" => BoxType::Meta,
            b"trak" => BoxType::Trak,
            b"tkhd" => BoxType::Tkhd,
            b"tref" => BoxType::Tref,
            b"trgr" => BoxType::Trgr,
            b"edts" => BoxType::Edts,
            b"elst" => BoxType::Elst,
            b"mdia" => BoxType::Mdia,
            b"mdhd" => BoxType::Mdhd,
            b"hdlr" => BoxType::Hdlr,
            b"elng" => BoxType::Elng,
            b"minf" => BoxType::Minf,
            b"vmhd" => BoxType::Vmhd,
            b"smhd" => BoxType::Smhd,
            b"hmhd" => BoxType::Hmhd,
            b"sthd" => BoxType::Sthd,
            b"nmhd" => BoxType::Nmhd,
            b"dinf" => BoxType::Dinf,
            b"dref" => BoxType::Dref,
            b"stbl" => BoxType::Stbl,
            b"stsd" => BoxType::Stsd,
            b"stts" => BoxType::Stts,
            b"ctts" => BoxType::Ctts,
            b"cslg" => BoxType::Cslg,
            b"stsc" => BoxType::Stsc,
            b"stsz" => BoxType::Stsz,
            b"stz2" => BoxType::Stz2,
            b"stco" => BoxType::Stco,
            b"co64" => BoxType::Co64,
            b"stss" => BoxType::Stss,
            b"stsh" => BoxType::Stsh,
            b"padb" => BoxType::Padb,
            b"stdp" => BoxType::Stdp,
            b"sdtp" => BoxType::Sdtp,
            b"sbgp" => BoxType::Sbgp,
            b"sgpd" => BoxType::Sgpd,
            b"subs" => BoxType::Subs,
            b"saiz" => BoxType::Saiz,
            b"saio" => BoxType::Saio,
            b"udta" => BoxType::Udta,
            b"mvex" => BoxType::Mvex,
            b"mehd" => BoxType::Mehd,
            b"trex" => BoxType::Trex,
            b"leva" => BoxType::Leva,
            b"moof" => BoxType::Moof,
            b"mfhd" => BoxType::Mfhd,
            b"traf" => BoxType::Traf,
            b"tfhd" => BoxType::Tfhd,
            b"trun" => BoxType::Trun,
            b"tfdt" => BoxType::Tfdt,
            b"mfra" => BoxType::Mfra,
            b"tfra" => BoxType::Tfra,
            b"mfro" => BoxType::Mfro,
            b"mdat" => BoxType::Mdat,
            b"free" => BoxType::Free,
            b"skip" => BoxType::Skip,
            b"cprt" => BoxType::Cprt,
            b"tsel" => BoxType::Tsel,
            b"strk" => BoxType::Strk,
            b"stri" => BoxType::Stri,
            b"strd" => BoxType::Strd,
            b"iloc" => BoxType::Iloc,
            b"ipro" => BoxType::Ipro,
            b"sinf" => BoxType::Sinf,
            b"frma" => BoxType::Frma,
            b"schm" => BoxType::Schm,
            b"schi" => BoxType::Schi,
            b"iinf" => BoxType::Iinf,
            b"xml " => BoxType::Xml,
            b"bxml" => BoxType::Bxml,
            b"pitm" => BoxType::Pitm,
            b"fiin" => BoxType::Fiin,
            b"paen" => BoxType::Paen,
            b"fire" => BoxType::Fire,
            b"fpar" => BoxType::Fpar,
            b"fecr" => BoxType::Fecr,
            b"segr" => BoxType::Segr,
            b"gitn" => BoxType::Gitn,
            b"idat" => BoxType::Idat,
            b"iref" => BoxType::Iref,
            b"meco" => BoxType::Meco,
            b"mere" => BoxType::Mere,
            b"styp" => BoxType::Styp,
            b"sidx" => BoxType::Sidx,
            b"ssix" => BoxType::Ssix,
            b"prft" => BoxType::Prft,
            _ => BoxType::Unknown(value),
        }
    }
}

impl From<&BoxType> for [u8; 4] {
    fn from(value: &BoxType) -> Self {
        match value {
            BoxType::Ftyp => *b"ftyp",
            BoxType::Pdin => *b"pdin",
            BoxType::Moov => *b"moov",
            BoxType::Mvhd => *b"mvhd",
            BoxType::Meta => *b"meta",
            BoxType::Trak => *b"trak",
            BoxType::Tkhd => *b"tkhd",
            BoxType::Tref => *b"tref",
            BoxType::Trgr => *b"trgr",
            BoxType::Edts => *b"edts",
            BoxType::Elst => *b"elst",
            BoxType::Mdia => *b"mdia",
            BoxType::Mdhd => *b"mdhd",
            BoxType::Hdlr => *b"hdlr",
            BoxType::Elng => *b"elng",
            BoxType::Minf => *b"minf",
            BoxType::Vmhd => *b"vmhd",
            BoxType::Smhd => *b"smhd",
            BoxType::Hmhd => *b"hmhd",
            BoxType::Sthd => *b"sthd",
            BoxType::Nmhd => *b"nmhd",
            BoxType::Dinf => *b"dinf",
            BoxType::Dref => *b"dref",
            BoxType::Stbl => *b"stbl",
            BoxType::Stsd => *b"stsd",
            BoxType::Stts => *b"stts",
            BoxType::Ctts => *b"ctts",
            BoxType::Cslg => *b"cslg",
            BoxType::Stsc => *b"stsc",
            BoxType::Stsz => *b"stsz",
            BoxType::Stz2 => *b"stz2",
            BoxType::Stco => *b"stco",
            BoxType::Co64 => *b"co64",
            BoxType::Stss => *b"stss",
            BoxType::Stsh => *b"stsh",
            BoxType::Padb => *b"padb",
            BoxType::Stdp => *b"stdp",
            BoxType::Sdtp => *b"sdtp",
            BoxType::Sbgp => *b"sbgp",
            BoxType::Sgpd => *b"sgpd",
            BoxType::Subs => *b"subs",
            BoxType::Saiz => *b"saiz",
            BoxType::Saio => *b"saio",
            BoxType::Udta => *b"udta",
            BoxType::Mvex => *b"mvex",
            BoxType::Mehd => *b"mehd",
            BoxType::Trex => *b"trex",
            BoxType::Leva => *b"leva",
            BoxType::Moof => *b"moof",
            BoxType::Mfhd => *b"mfhd",
            BoxType::Traf => *b"traf",
            BoxType::Tfhd => *b"tfhd",
            BoxType::Trun => *b"trun",
            BoxType::Tfdt => *b"tfdt",
            BoxType::Mfra => *b"mfra",
            BoxType::Tfra => *b"tfra",
            BoxType::Mfro => *b"mfro",
            BoxType::Mdat => *b"mdat",
            BoxType::Free => *b"free",
            BoxType::Skip => *b"skip",
            BoxType::Cprt => *b"cprt",
            BoxType::Tsel => *b"tsel",
            BoxType::Strk => *b"strk",
            BoxType::Stri => *b"stri",
            BoxType::Strd => *b"strd",
            BoxType::Iloc => *b"iloc",
            BoxType::Ipro => *b"ipro",
            BoxType::Sinf => *b"sinf",
            BoxType::Frma => *b"frma",
            BoxType::Schm => *b"schm",
            BoxType::Schi => *b"schi",
            BoxType::Iinf => *b"iinf",
            BoxType::Xml => *b"xml ",
            BoxType::Bxml => *b"bxml",
            BoxType::Pitm => *b"pitm",
            BoxType::Fiin => *b"fiin",
            BoxType::Paen => *b"paen",
            BoxType::Fire => *b"fire",
            BoxType::Fpar => *b"fpar",
            BoxType::Fecr => *b"fecr",
            BoxType::Segr => *b"segr",
            BoxType::Gitn => *b"gitn",
            BoxType::Idat => *b"idat",
            BoxType::Iref => *b"iref",
            BoxType::Meco => *b"meco",
            BoxType::Mere => *b"mere",
            BoxType::Styp => *b"styp",
            BoxType::Sidx => *b"sidx",
            BoxType::Ssix => *b"ssix",
            BoxType::Prft => *b"prft",
            BoxType::Unknown(v) => *v,
        }
    }
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
    pub box_type: [u8; 4],
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

        for b in &self.box_type {
            writer.write_bits(*b as u32, 8);
        }

        if let BoxSize::Large(s) = self.box_size {
            writer.write_bits((s >> 32) as u32, 32); // upper 32bits of largesize
            writer.write_bits(s as u32, 32); // lower 32bits of largesize
        }
    }

    pub fn parse(reader: &mut BitstreamReader) -> Result<Self, Error> {
        let size = reader.read_bits(32)?;
        let box_type = [
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
        ];

        let (box_size, header_size) = match size {
            0 => (BoxSize::ExtendsToEnd, 8),
            1 => {
                // largesize: 64-bit size follows
                let high = reader.read_bits(32)? as u64;
                let low = reader.read_bits(32)? as u64;
                (BoxSize::Large((high << 32) | low), 16)
            }
            _ => (BoxSize::Normal(size as u32), 8),
        };

        Ok(Self {
            box_size,
            box_type,
            header_size,
        })
    }
}

#[derive(Debug)]
pub struct UnknownBox {
    pub header: BoxHeader,
    pub payload: Vec<u8>,
}

/// Top-level box variants.
#[derive(Debug)]
pub enum Mp4Box {
    Unknown(UnknownBox),
}

impl Mp4Box {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8]) -> Result<(Self, usize), Error> {
        let mut reader = BitstreamReader::new(data);
        let header = BoxHeader::parse(&mut reader)?;

        // Resolve total size. For ExtendsToEnd, the box consumes the rest of `data`.
        let total: usize = match header.box_size {
            BoxSize::Normal(s) => s as usize,
            BoxSize::Large(s) => s as usize,
            BoxSize::ExtendsToEnd => data.len(),
        };
        if data.len() < total {
            return Err(Error::DataTooShort);
        }
        let payload = &data[header.header_size..total];

        let parsed = match &header.box_type {
            _ => Mp4Box::Unknown(UnknownBox {
                header: header.clone(),
                payload: payload.to_vec(),
            }),
        };

        Ok((parsed, total))
    }
}

/// Parse all top-level boxes from a byte slice.
pub fn parse_all(data: &[u8]) -> Result<Vec<Mp4Box>, Error> {
    let mut boxes = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (b, consumed) = Mp4Box::parse(&data[offset..])?;
        offset += consumed;
        boxes.push(b);
    }
    Ok(boxes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::bitstream::BitstreamWriter;

    fn roundtrip_header(input: &[u8]) {
        let mut reader = BitstreamReader::new(input);
        let header = BoxHeader::parse(&mut reader).expect("failed to parse header");

        let mut writer = BitstreamWriter::new();
        header.to_bytes(&mut writer);
        let output = writer.finish();

        assert_eq!(output, input);
    }

    #[test]
    fn test_header_normal() {
        // size = 32, type = "ftyp"
        let data = [
            0x00, 0x00, 0x00, 0x20, // size = 32
            b'f', b't', b'y', b'p', // type = "ftyp"
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::Normal(32));
        assert_eq!(&header.box_type, b"ftyp");
        assert_eq!(header.header_size, 8);

        roundtrip_header(&data);
    }

    #[test]
    fn test_header_large() {
        // size = 1 (largesize marker), type = "mdat", largesize = 0x1_0000_0000 (4 GiB)
        let data = [
            0x00, 0x00, 0x00, 0x01, // size = 1 (largesize marker)
            b'm', b'd', b'a', b't', // type = "mdat"
            0x00, 0x00, 0x00, 0x01, // largesize upper 32bit
            0x00, 0x00, 0x00, 0x00, // largesize lower 32bit
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::Large(0x1_0000_0000));
        assert_eq!(&header.box_type, b"mdat");
        assert_eq!(header.header_size, 16);

        roundtrip_header(&data);
    }

    #[test]
    fn test_header_extends_to_end() {
        // size = 0 (extends to end of file), type = "mdat"
        let data = [
            0x00, 0x00, 0x00, 0x00, // size = 0
            b'm', b'd', b'a', b't', // type = "mdat"
        ];
        let mut reader = BitstreamReader::new(&data);
        let header = BoxHeader::parse(&mut reader).unwrap();
        assert_eq!(header.box_size, BoxSize::ExtendsToEnd);
        assert_eq!(&header.box_type, b"mdat");
        assert_eq!(header.header_size, 8);

        roundtrip_header(&data);
    }
}
