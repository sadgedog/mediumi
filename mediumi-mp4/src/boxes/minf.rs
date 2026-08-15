use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, dinf::Dinf, hmhd::Hmhd, nmhd::Nmhd, smhd::Smhd,
        stbl::Stbl, sthd::Sthd, vmhd::Vmhd,
    },
    types::BoxType,
};
use mediumi_util::bytestream::ByteWriter;

#[derive(Debug)]
pub struct Minf {
    pub vmhd: Option<Vmhd>,
    pub smhd: Option<Smhd>,
    pub hmhd: Option<Hmhd>,
    pub sthd: Option<Sthd>,
    pub nmhd: Option<Nmhd>,
    pub dinf: Option<Dinf>,
    pub stbl: Option<Box<Stbl>>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Minf {
    const BOX_TYPE: BoxType = BoxType::Minf;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        if let Some(ref b) = self.vmhd {
            b.write_box(writer);
        }
        if let Some(ref b) = self.smhd {
            b.write_box(writer);
        }
        if let Some(ref b) = self.hmhd {
            b.write_box(writer);
        }
        if let Some(ref b) = self.sthd {
            b.write_box(writer);
        }
        if let Some(ref b) = self.nmhd {
            b.write_box(writer);
        }
        if let Some(ref b) = self.dinf {
            b.write_box(writer);
        }
        if let Some(ref b) = self.stbl {
            b.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut vmhd: Option<Vmhd> = None;
        let mut smhd: Option<Smhd> = None;
        let mut hmhd: Option<Hmhd> = None;
        let mut sthd: Option<Sthd> = None;
        let mut nmhd: Option<Nmhd> = None;
        let mut dinf: Option<Dinf> = None;
        let mut stbl: Option<Box<Stbl>> = None;
        let mut others: Vec<Vec<u8>> = Vec::new();
        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Vmhd(b) => vmhd = Some(b),
                Mp4Box::Smhd(b) => smhd = Some(b),
                Mp4Box::Hmhd(b) => hmhd = Some(b),
                Mp4Box::Sthd(b) => sthd = Some(b),
                Mp4Box::Nmhd(b) => nmhd = Some(b),
                Mp4Box::Dinf(b) => dinf = Some(b),
                Mp4Box::Stbl(b) => stbl = Some(b),
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            vmhd,
            smhd,
            hmhd,
            sthd,
            nmhd,
            dinf,
            stbl,
            others,
        })
    }
}
