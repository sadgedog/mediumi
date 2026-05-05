use crate::{
    boxes::{
        BaseBox, BoxIter, Error, Mp4Box, co64::Co64, cslg::Cslg, ctts::Ctts, padb::Padb,
        saio::Saio, saiz::Saiz, sbgp::Sbgp, sdtp::Sdtp, sgpd::Sgpd, stco::Stco, stdp::Stdp,
        stsc::Stsc, stsd::Stsd, stsh::Stsh, stss::Stss, stsz::Stsz, stts::Stts, stz2::Stz2,
        subs::Subs,
    },
    types::BoxType,
    util::bitstream::BitstreamWriter,
};

#[derive(Debug)]
pub struct Stbl {
    pub stsd: Stsd,
    pub stts: Stts,
    pub ctts: Option<Ctts>,
    pub cslg: Option<Cslg>,
    pub stsc: Stsc,
    pub stsz: Option<Stsz>,
    pub stz2: Option<Stz2>,
    pub stco: Stco,
    pub co64: Option<Co64>,
    pub stss: Option<Stss>,
    pub stsh: Option<Stsh>,
    pub padb: Option<Padb>,
    pub stdp: Option<Stdp>,
    pub sdtp: Option<Sdtp>,
    pub sbgps: Vec<Sbgp>,
    pub sgpds: Vec<Sgpd>,
    pub subs: Vec<Subs>,
    pub saizs: Vec<Saiz>,
    pub saios: Vec<Saio>,
    pub others: Vec<Vec<u8>>,
}

impl BaseBox for Stbl {
    const BOX_TYPE: BoxType = BoxType::Stbl;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        self.stsd.write_box(writer);
        self.stts.write_box(writer);
        if let Some(ref b) = self.ctts {
            b.write_box(writer);
        }
        if let Some(ref b) = self.cslg {
            b.write_box(writer);
        }
        self.stsc.write_box(writer);
        if let Some(ref b) = self.stsz {
            b.write_box(writer);
        }
        if let Some(ref b) = self.stz2 {
            b.write_box(writer);
        }
        self.stco.write_box(writer);
        if let Some(ref b) = self.co64 {
            b.write_box(writer);
        }
        if let Some(ref b) = self.stss {
            b.write_box(writer);
        }
        if let Some(ref b) = self.stsh {
            b.write_box(writer);
        }
        if let Some(ref b) = self.padb {
            b.write_box(writer);
        }
        if let Some(ref b) = self.stdp {
            b.write_box(writer);
        }
        if let Some(ref b) = self.sdtp {
            b.write_box(writer);
        }
        for b in &self.sbgps {
            b.write_box(writer);
        }
        for b in &self.sgpds {
            b.write_box(writer);
        }
        for b in &self.subs {
            b.write_box(writer);
        }
        for b in &self.saizs {
            b.write_box(writer);
        }
        for b in &self.saios {
            b.write_box(writer);
        }
        for raw in &self.others {
            for &b in raw {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut stsd: Option<Stsd> = None;
        let mut stts: Option<Stts> = None;
        let mut ctts: Option<Ctts> = None;
        let mut cslg: Option<Cslg> = None;
        let mut stsc: Option<Stsc> = None;
        let mut stsz: Option<Stsz> = None;
        let mut stz2: Option<Stz2> = None;
        let mut stco: Option<Stco> = None;
        let mut co64: Option<Co64> = None;
        let mut stss: Option<Stss> = None;
        let mut stsh: Option<Stsh> = None;
        let mut padb: Option<Padb> = None;
        let mut stdp: Option<Stdp> = None;
        let mut sdtp: Option<Sdtp> = None;
        let mut sbgps: Vec<Sbgp> = Vec::new();
        let mut sgpds: Vec<Sgpd> = Vec::new();
        let mut subs: Vec<Subs> = Vec::new();
        let mut saizs: Vec<Saiz> = Vec::new();
        let mut saios: Vec<Saio> = Vec::new();
        let mut others: Vec<Vec<u8>> = Vec::new();

        for item in BoxIter::new(data) {
            let (child, raw) = item?;
            match child {
                Mp4Box::Stsd(b) => {
                    if stsd.is_some() {
                        return Err(Error::DuplicateBox("stsd"));
                    }
                    stsd = Some(b);
                }
                Mp4Box::Stts(b) => {
                    if stts.is_some() {
                        return Err(Error::DuplicateBox("stts"));
                    }
                    stts = Some(b);
                }
                Mp4Box::Ctts(b) => {
                    if ctts.is_some() {
                        return Err(Error::DuplicateBox("ctts"));
                    }
                    ctts = Some(b);
                }
                Mp4Box::Cslg(b) => {
                    if cslg.is_some() {
                        return Err(Error::DuplicateBox("cslg"));
                    }
                    cslg = Some(b);
                }
                Mp4Box::Stsc(b) => {
                    if stsc.is_some() {
                        return Err(Error::DuplicateBox("stsc"));
                    }
                    stsc = Some(b);
                }
                Mp4Box::Stsz(b) => {
                    if stsz.is_some() {
                        return Err(Error::DuplicateBox("stsz"));
                    }
                    stsz = Some(b);
                }
                Mp4Box::Stz2(b) => {
                    if stz2.is_some() {
                        return Err(Error::DuplicateBox("stz2"));
                    }
                    stz2 = Some(b);
                }
                Mp4Box::Stco(b) => {
                    if stco.is_some() {
                        return Err(Error::DuplicateBox("stco"));
                    }
                    stco = Some(b);
                }
                Mp4Box::Co64(b) => {
                    if co64.is_some() {
                        return Err(Error::DuplicateBox("co64"));
                    }
                    co64 = Some(b);
                }
                Mp4Box::Stss(b) => {
                    if stss.is_some() {
                        return Err(Error::DuplicateBox("stss"));
                    }
                    stss = Some(b);
                }
                Mp4Box::Stsh(b) => {
                    if stsh.is_some() {
                        return Err(Error::DuplicateBox("stsh"));
                    }
                    stsh = Some(b);
                }
                Mp4Box::Padb(b) => {
                    if padb.is_some() {
                        return Err(Error::DuplicateBox("padb"));
                    }
                    padb = Some(b);
                }
                Mp4Box::Stdp(b) => {
                    if stdp.is_some() {
                        return Err(Error::DuplicateBox("stdp"));
                    }
                    stdp = Some(b);
                }
                Mp4Box::Sdtp(b) => {
                    if sdtp.is_some() {
                        return Err(Error::DuplicateBox("sdtp"));
                    }
                    sdtp = Some(b);
                }
                Mp4Box::Sbgp(b) => sbgps.push(b),
                Mp4Box::Sgpd(b) => sgpds.push(b),
                Mp4Box::Subs(b) => subs.push(b),
                Mp4Box::Saiz(b) => saizs.push(b),
                Mp4Box::Saio(b) => saios.push(b),
                _ => others.push(raw.to_vec()),
            }
        }
        Ok(Self {
            stsd: stsd.ok_or(Error::MissingRequiredBox("stsd"))?,
            stts: stts.ok_or(Error::MissingRequiredBox("stts"))?,
            ctts,
            cslg,
            stsc: stsc.ok_or(Error::MissingRequiredBox("stsc"))?,
            stsz,
            stz2,
            stco: stco.ok_or(Error::MissingRequiredBox("stco"))?,
            co64,
            stss,
            stsh,
            padb,
            stdp,
            sdtp,
            sbgps,
            sgpds,
            subs,
            saizs,
            saios,
            others,
        })
    }
}
