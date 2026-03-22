//! H.264 codec processor for parsing and serializing Annex.B byte streams.
//!
//! Parses a PES payload into Annex B NAL units and extracts known parameter sets (e.g. SPS).
//! Can serialize back to bytes via `to_bytes`.

use crate::{
    annex_b::{StartCode, parse_all},
    error::Error,
    nal::{NalUnit, NalUnitType},
    pps::Pps,
    sps::Sps,
};

#[derive(Debug)]
pub enum NalData {
    Sps(StartCode, u8, Box<Sps>),
    Pps(StartCode, u8, Box<Pps>),
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
                NalData::Raw(sc, nri, nal_type, rbsp) => {
                    buf.extend_from_slice(sc.as_bytes());
                    buf.push(nri << 5 | u8::from(nal_type));
                    buf.extend_from_slice(rbsp);
                }
            }
        }
        buf
    }

    /// parse codec for 1 PES
    pub fn parse(pes_payload: &[u8]) -> Result<Self, Error> {
        let annex_b_list = parse_all(pes_payload)?;
        let mut nal_units = Vec::with_capacity(annex_b_list.len());
        let mut last_sps: Option<Sps> = None;

        for ab in annex_b_list {
            let sc = ab.start_code;
            let nri = ab.nal_unit.header.nal_ref_idc;
            let nal_type = ab.nal_unit.header.nal_unit_type;

            match nal_type {
                NalUnitType::SPS => {
                    let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                    let sps = Sps::parse(&rbsp)?;
                    last_sps = Some(sps.clone());
                    nal_units.push(NalData::Sps(sc, nri, Box::new(sps)));
                }
                NalUnitType::PPS => {
                    if let Some(ref sps) = last_sps {
                        let rbsp = NalUnit::remove_emulation_prevention_bytes(&ab.nal_unit.rbsp);
                        let pps = Pps::parse(&rbsp, sps)?;
                        nal_units.push(NalData::Pps(sc, nri, Box::new(pps)));
                    } else {
                        nal_units.push(NalData::Raw(sc, nri, nal_type, ab.nal_unit.rbsp));
                    }
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
