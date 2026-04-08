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
pub mod nal;
pub mod non_idr;
pub mod pps;
pub mod slice_header;
pub mod sps;
pub mod util;

use crate::{
    annex_b::{StartCode, parse_all},
    aud::Aud,
    error::Error,
    nal::{NalUnit, NalUnitType},
    non_idr::NonIDR,
    pps::Pps,
    sps::Sps,
};

#[derive(Debug)]
pub enum NalData {
    NonIdr(StartCode, u8, Box<NonIDR>),
    Sps(StartCode, u8, Box<Sps>),
    Pps(StartCode, u8, Box<Pps>),
    Aud(StartCode, u8, Aud),
    EOSeq(StartCode, u8),
    EOStream(StartCode, u8),
    Raw(StartCode, u8, NalUnitType, Vec<u8>), // start_code, nal_ref_idc, type, rbsp
}

#[derive(Debug)]
pub struct Processor {
    pub nal_units: Vec<NalData>,
}

impl Processor {
    /// Write codec data
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for nal in &self.nal_units {
            match nal {
                NalData::Sps(sc, nri, sps) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(&NalUnitType::SPS));
                    buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                        &sps.to_bytes(),
                    ));
                }
                NalData::Pps(sc, nri, pps) => {
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
                NalData::Raw(sc, nri, nal_type, rbsp) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(nal_type));
                    buf.extend_from_slice(rbsp);
                }
                _ => {}
            }
        }
        buf
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
