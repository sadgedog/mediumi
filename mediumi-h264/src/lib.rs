//! H.264 codec processor for parsing and serializing Annex.B byte streams.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_h264::Processor;
//!
//! let data = std::fs::read("input.h264")?;
//! let processor = Processor::parse(&data)?;
//! let output = processor.to_bytes();
//! # Ok(())
//! # }
//! ```

pub mod annex_b;
pub mod aud;
pub mod error;
pub mod filler_data;
pub mod idr;
pub mod nal;
pub mod non_idr;
pub mod pps;
pub mod sei;
pub mod slice_a;
pub mod slice_b;
pub mod slice_c;
pub mod slice_header;
pub mod sps;
pub mod sps_ext;
pub mod util;

use crate::{
    annex_b::{StartCode, parse_all},
    aud::Aud,
    error::Error,
    filler_data::FillerData,
    idr::IDR,
    nal::{NalUnit, NalUnitType},
    non_idr::NonIDR,
    pps::Pps,
    sei::Sei,
    slice_a::SliceA,
    slice_b::SliceB,
    slice_c::SliceC,
    sps::Sps,
    sps_ext::SpsExt,
};

#[derive(Debug)]
pub enum NalData {
    NonIdr(StartCode, u8, Box<NonIDR>),
    SliceA(StartCode, u8, Box<SliceA>),
    SliceB(StartCode, u8, Box<SliceB>),
    SliceC(StartCode, u8, Box<SliceC>),
    Idr(StartCode, u8, Box<IDR>),
    Sei(StartCode, u8, Box<Sei>),
    Sps(StartCode, u8, Box<Sps>),
    Pps(StartCode, u8, Box<Pps>),
    Aud(StartCode, u8, Aud),
    EOSeq(StartCode, u8),
    EOStream(StartCode, u8),
    FillerData(StartCode, u8, FillerData),
    SpsExt(StartCode, u8, SpsExt),
    Raw(StartCode, u8, NalUnitType, Vec<u8>), // start_code, nal_ref_idc, type, rbsp
}

#[derive(Debug)]
pub struct Processor {
    pub nal_units: Vec<NalData>,
}

impl Processor {
    /// Write codec data
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        let mut last_sps: Option<&Sps> = None;
        let mut last_pps: Option<&Pps> = None;

        for nal in &self.nal_units {
            match nal {
                NalData::NonIdr(sc, nri, non_idr) => {
                    if let (Some(sps), Some(pps)) = (last_sps, last_pps) {
                        buf.extend_from_slice(sc.as_bytes());
                        buf.push(nri << 5 | u8::from(&NalUnitType::NonIDR));
                        buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                            &non_idr.to_bytes(sps, pps)?,
                        ));
                    }
                }
                NalData::SliceA(sc, nri, slice_a) => {
                    if let (Some(sps), Some(pps)) = (last_sps, last_pps) {
                        buf.extend_from_slice(sc.as_bytes());
                        buf.push(nri << 5 | u8::from(&NalUnitType::SliceA));
                        buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                            &slice_a.to_bytes(sps, pps)?,
                        ));
                    }
                }
                NalData::SliceB(sc, nri, slice_b) => {
                    if let (Some(sps), Some(pps)) = (last_sps, last_pps) {
                        buf.extend_from_slice(sc.as_bytes());
                        buf.push(nri << 5 | u8::from(&NalUnitType::SliceB));
                        buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                            &slice_b.to_bytes(sps, pps)?,
                        ));
                    }
                }
                NalData::SliceC(sc, nri, slice_c) => {
                    if let (Some(sps), Some(pps)) = (last_sps, last_pps) {
                        buf.extend_from_slice(sc.as_bytes());
                        buf.push(nri << 5 | u8::from(&NalUnitType::SliceC));
                        buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                            &slice_c.to_bytes(sps, pps)?,
                        ));
                    }
                }
                NalData::Idr(sc, nri, idr) => {
                    if let (Some(sps), Some(pps)) = (last_sps, last_pps) {
                        buf.extend_from_slice(sc.as_bytes());
                        buf.push(nri << 5 | u8::from(&NalUnitType::IDR));
                        buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                            &idr.to_bytes(sps, pps)?,
                        ));
                    }
                }
                NalData::Sei(sc, nri, sei) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::SEI));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &sei.to_bytes(),
                    ));
                }
                NalData::Sps(sc, nri, sps) => {
                    last_sps = Some(sps);
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::SPS));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &sps.to_bytes(),
                    ));
                }
                NalData::Pps(sc, nri, pps) => {
                    last_pps = Some(pps);
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::PPS));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &pps.to_bytes(),
                    ));
                }
                NalData::Aud(sc, nri, aud) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::AUD));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &aud.to_bytes(),
                    ));
                }
                NalData::EOSeq(sc, nri) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::EOSeq));
                }
                NalData::EOStream(sc, nri) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::EOStream));
                }
                NalData::FillerData(sc, nri, filler) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::FillerData));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &filler.to_bytes(),
                    ));
                }
                NalData::SpsExt(sc, nri, sps_ext) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::SPSExt));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &sps_ext.to_bytes(),
                    ));
                }
                NalData::Raw(sc, nri, nal_type, rbsp) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(nal_type));
                    buf.extend_from_slice(rbsp);
                }
            }
        }

        Ok(buf)
    }

    /// parse codec for 1 PES
    pub fn parse(pes_payload: &[u8]) -> Result<Self, Error> {
        let annex_b_list = parse_all(pes_payload)?;
        let mut nal_units = Vec::with_capacity(annex_b_list.len());
        let mut last_sps: Option<Sps> = None;
        let mut last_pps: Option<Pps> = None;

        for ab in annex_b_list {
            let sc = ab.start_code;
            let nri = ab.nal_unit.header.nal_ref_idc;
            let nal_type = ab.nal_unit.header.nal_unit_type;

            match nal_type {
                NalUnitType::NonIDR => {
                    if let (Some(sps), Some(pps)) = (&last_sps, &last_pps) {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let non_idr = NonIDR::parse(&rbsp, sps, pps, nri)?;
                        nal_units.push(NalData::NonIdr(sc, nri, Box::new(non_idr)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::SliceA => {
                    if let (Some(sps), Some(pps)) = (&last_sps, &last_pps) {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let slice_a = SliceA::parse(&rbsp, sps, pps, nri)?;
                        nal_units.push(NalData::SliceA(sc, nri, Box::new(slice_a)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::SliceB => {
                    if let (Some(sps), Some(pps)) = (&last_sps, &last_pps) {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let slice_b = SliceB::parse(&rbsp, sps, pps)?;
                        nal_units.push(NalData::SliceB(sc, nri, Box::new(slice_b)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::SliceC => {
                    if let (Some(sps), Some(pps)) = (&last_sps, &last_pps) {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let slice_c = SliceC::parse(&rbsp, sps, pps)?;
                        nal_units.push(NalData::SliceC(sc, nri, Box::new(slice_c)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::IDR => {
                    if let (Some(sps), Some(pps)) = (&last_sps, &last_pps) {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let idr = IDR::parse(&rbsp, sps, pps, nri)?;
                        nal_units.push(NalData::Idr(sc, nri, Box::new(idr)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::SEI => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let sei = Sei::parse(&rbsp)?;
                    nal_units.push(NalData::Sei(sc, nri, Box::new(sei)));
                }
                NalUnitType::SPS => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let sps = Sps::parse(&rbsp)?;
                    last_sps = Some(sps.clone());
                    nal_units.push(NalData::Sps(sc, nri, Box::new(sps)));
                }
                NalUnitType::PPS => {
                    if let Some(sps) = &last_sps {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let pps = Pps::parse(&rbsp, sps)?;
                        last_pps = Some(pps.clone());
                        nal_units.push(NalData::Pps(sc, nri, Box::new(pps)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
                }
                NalUnitType::AUD => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let aud = Aud::parse(&rbsp)?;
                    nal_units.push(NalData::Aud(sc, nri, aud));
                }
                NalUnitType::EOSeq => {
                    nal_units.push(NalData::EOSeq(sc, nri));
                }
                NalUnitType::EOStream => {
                    nal_units.push(NalData::EOStream(sc, nri));
                }
                NalUnitType::FillerData => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let filler = FillerData::parse(&rbsp);
                    nal_units.push(NalData::FillerData(sc, nri, filler));
                }
                NalUnitType::SPSExt => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let sps_ext = SpsExt::parse(&rbsp)?;
                    nal_units.push(NalData::SpsExt(sc, nri, sps_ext));
                }
                NalUnitType::Unknown(v) => {
                    return Err(Error::InvalidNalUnitType(v));
                }
                _ => {
                    nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                }
            }
        }

        Ok(Self { nal_units })
    }
}
