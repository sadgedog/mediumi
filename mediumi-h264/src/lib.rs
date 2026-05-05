//! H.264 codec processor for parsing and serializing Annex.B byte streams.
//!
//! # Example (Annex B)
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_h264::Processor;
//!
//! let data = std::fs::read("input.h264")?;
//! let processor = Processor::from_annex_b(&data)?;
//! let output = processor.to_annex_b()?;
//! # Ok(())
//! # }
//! ```

pub mod annex_b;
pub mod aud;
pub mod avcc;
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
    avcc::AvccConfig,
    error::Error,
    filler_data::FillerData,
    idr::Idr,
    nal::{NalUnit, NalUnitType},
    non_idr::NonIdr,
    pps::Pps,
    sei::Sei,
    slice_a::SliceA,
    slice_b::SliceB,
    slice_c::SliceC,
    sps::Sps,
    sps_ext::SpsExt,
    util::bitstream::BitstreamReader,
};

#[derive(Debug)]
pub enum NalData {
    Unspecified(StartCode, u8, NalUnitType, Vec<u8>), // not implemented, raw data
    NonIdr(StartCode, u8, Box<NonIdr>),
    SliceA(StartCode, u8, Box<SliceA>),
    SliceB(StartCode, u8, Box<SliceB>),
    SliceC(StartCode, u8, Box<SliceC>),
    Idr(StartCode, u8, Box<Idr>),
    Sei(StartCode, u8, Box<Sei>),
    Sps(StartCode, u8, Box<Sps>),
    Pps(StartCode, u8, Box<Pps>),
    Aud(StartCode, u8, Aud),
    EOSeq(StartCode, u8),
    EOStream(StartCode, u8),
    FillerData(StartCode, u8, FillerData),
    SpsExt(StartCode, u8, SpsExt),
    PrefixNalUnit(StartCode, u8, NalUnitType, Vec<u8>), // not implemented, raw data
    SubsetSps(StartCode, u8, NalUnitType, Vec<u8>),     // not implemented, raw data
    Dps(StartCode, u8, NalUnitType, Vec<u8>),           // not implemented, raw data
    Reserved(StartCode, u8, NalUnitType, Vec<u8>),      // not implemented, raw data
    Aux(StartCode, u8, NalUnitType, Vec<u8>),           // not implemented, raw data
    SliceExt(StartCode, u8, NalUnitType, Vec<u8>),      // not implemented, raw data
    DepthExt(StartCode, u8, NalUnitType, Vec<u8>),      // not implemented, raw data
    Unknown(StartCode, u8, NalUnitType, Vec<u8>),       // not implemented, raw data
    Raw(StartCode, u8, NalUnitType, Vec<u8>),           // start_code, nal_ref_idc, type, rbsp
}

#[derive(Debug)]
pub struct Processor {
    pub nal_units: Vec<NalData>,
}

impl Processor {
    /// Serialize as an Annex.B byte stream (start code prefixed).
    pub fn to_annex_b(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        let mut last_sps: Option<&Sps> = None;
        let mut last_pps: Option<&Pps> = None;

        for nal in &self.nal_units {
            if let Some(bytes) = serialize_nal(nal, &mut last_sps, &mut last_pps)? {
                buf.extend_from_slice(start_code_of(nal).as_bytes());
                buf.extend_from_slice(&bytes);
            }
        }

        Ok(buf)
    }

    /// Serialize length-prefixed NAL Units
    pub fn to_avcc(&self, base: &AvccConfig) -> Result<(Vec<u8>, AvccConfig), Error> {
        let mut config = base.clone();
        config.sps_nalus.clear();
        config.pps_nalus.clear();

        let length_size = config.nal_length_size();
        let mut buf = Vec::new();
        let mut last_sps: Option<&Sps> = None;
        let mut last_pps: Option<&Pps> = None;

        for nal in &self.nal_units {
            let Some(bytes) = serialize_nal(nal, &mut last_sps, &mut last_pps)? else {
                continue;
            };
            match nal {
                NalData::Sps(_, _, sps) => {
                    config.avc_profile_indication = sps.profile_idc;
                    config.profile_compatibility = sps.constraint_flags;
                    config.avc_level_indication = sps.level_idc;
                    config.sps_nalus.push(bytes);
                }
                NalData::Pps(_, _, _) => {
                    config.pps_nalus.push(bytes);
                }
                _ => {
                    let len_bytes = (bytes.len() as u32).to_be_bytes();
                    buf.extend_from_slice(&len_bytes[4 - length_size..]);
                    buf.extend_from_slice(&bytes);
                }
            }
        }

        Ok((buf, config))
    }

    /// Parse an Annex.B byte stream
    pub fn from_annex_b(pes_payload: &[u8]) -> Result<Self, Error> {
        let annex_b_list = parse_all(pes_payload)?;
        let mut nal_units = Vec::with_capacity(annex_b_list.len());
        let mut last_sps: Option<Sps> = None;
        let mut last_pps: Option<Pps> = None;

        for ab in annex_b_list {
            push_nal(
                &mut nal_units,
                &mut last_sps,
                &mut last_pps,
                ab.start_code,
                ab.nal_unit,
            )?;
        }

        Ok(Self { nal_units })
    }

    /// Parse length-prefixed NAL Units
    pub fn from_avcc(samples: &[&[u8]], config: &AvccConfig) -> Result<Self, Error> {
        let mut nal_units = Vec::new();
        let mut last_sps: Option<Sps> = None;
        let mut last_pps: Option<Pps> = None;

        for sps_bytes in &config.sps_nalus {
            let nal_unit = NalUnit::parse(sps_bytes)?;
            push_nal(
                &mut nal_units,
                &mut last_sps,
                &mut last_pps,
                // Length prefixed format doesn't  hold start-code(sc).
                // Insert dummy data for signature.
                StartCode::FourBytes,
                nal_unit,
            )?;
        }

        for pps_bytes in &config.pps_nalus {
            let nal_unit = NalUnit::parse(pps_bytes)?;
            push_nal(
                &mut nal_units,
                &mut last_sps,
                &mut last_pps,
                StartCode::FourBytes,
                nal_unit,
            )?;
        }

        let length_bits = (config.nal_length_size() * 8) as u8;
        for sample in samples {
            let mut reader = BitstreamReader::new(sample);
            while reader.remaining_bits() >= length_bits as usize {
                let nal_len = reader.read_bits(length_bits)? as usize;
                let nal_bytes = reader.read_slice(nal_len)?;
                let nal_unit = NalUnit::parse(nal_bytes)?;
                push_nal(
                    &mut nal_units,
                    &mut last_sps,
                    &mut last_pps,
                    StartCode::FourBytes,
                    nal_unit,
                )?;
            }
        }

        Ok(Self { nal_units })
    }
}

/// Return the NAL byte stream (without start code).
fn serialize_nal<'a>(
    nal: &'a NalData,
    last_sps: &mut Option<&'a Sps>,
    last_pps: &mut Option<&'a Pps>,
) -> Result<Option<Vec<u8>>, Error> {
    let mut buf = Vec::new();
    match nal {
        NalData::Unspecified(_, nri, nal_type, rbsp) => {
            buf.push(nri << 5 | u8::from(nal_type));
            buf.extend_from_slice(rbsp);
        }
        NalData::NonIdr(_, nri, non_idr) => {
            let (Some(sps), Some(pps)) = (*last_sps, *last_pps) else {
                return Ok(None);
            };
            buf.push(nri << 5 | u8::from(&NalUnitType::NonIdr));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &non_idr.to_bytes(sps, pps)?,
            ));
        }
        NalData::SliceA(_, nri, slice_a) => {
            let (Some(sps), Some(pps)) = (*last_sps, *last_pps) else {
                return Ok(None);
            };
            buf.push(nri << 5 | u8::from(&NalUnitType::SliceA));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &slice_a.to_bytes(sps, pps)?,
            ));
        }
        NalData::SliceB(_, nri, slice_b) => {
            let (Some(sps), Some(pps)) = (*last_sps, *last_pps) else {
                return Ok(None);
            };
            buf.push(nri << 5 | u8::from(&NalUnitType::SliceB));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &slice_b.to_bytes(sps, pps)?,
            ));
        }
        NalData::SliceC(_, nri, slice_c) => {
            let (Some(sps), Some(pps)) = (*last_sps, *last_pps) else {
                return Ok(None);
            };
            buf.push(nri << 5 | u8::from(&NalUnitType::SliceC));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &slice_c.to_bytes(sps, pps)?,
            ));
        }
        NalData::Idr(_, nri, idr) => {
            let (Some(sps), Some(pps)) = (*last_sps, *last_pps) else {
                return Ok(None);
            };
            buf.push(nri << 5 | u8::from(&NalUnitType::Idr));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &idr.to_bytes(sps, pps)?,
            ));
        }
        NalData::Sei(_, nri, sei) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::Sei));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(&sei.to_bytes()));
        }
        NalData::Sps(_, nri, sps) => {
            *last_sps = Some(sps);
            buf.push(nri << 5 | u8::from(&NalUnitType::Sps));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(&sps.to_bytes()));
        }
        NalData::Pps(_, nri, pps) => {
            *last_pps = Some(pps);
            buf.push(nri << 5 | u8::from(&NalUnitType::Pps));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(&pps.to_bytes()));
        }
        NalData::Aud(_, nri, aud) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::Aud));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(&aud.to_bytes()));
        }
        NalData::EOSeq(_, nri) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::EOSeq));
        }
        NalData::EOStream(_, nri) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::EOStream));
        }
        NalData::FillerData(_, nri, filler) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::FillerData));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &filler.to_bytes(),
            ));
        }
        NalData::SpsExt(_, nri, sps_ext) => {
            buf.push(nri << 5 | u8::from(&NalUnitType::SpsExt));
            buf.extend_from_slice(&NalUnit::attach_emulation_prevention_bytes(
                &sps_ext.to_bytes(),
            ));
        }
        NalData::PrefixNalUnit(_, nri, nal_type, rbsp)
        | NalData::SubsetSps(_, nri, nal_type, rbsp)
        | NalData::Dps(_, nri, nal_type, rbsp)
        | NalData::Reserved(_, nri, nal_type, rbsp)
        | NalData::Aux(_, nri, nal_type, rbsp)
        | NalData::SliceExt(_, nri, nal_type, rbsp)
        | NalData::DepthExt(_, nri, nal_type, rbsp)
        | NalData::Unknown(_, nri, nal_type, rbsp)
        | NalData::Raw(_, nri, nal_type, rbsp) => {
            buf.push(nri << 5 | u8::from(nal_type));
            buf.extend_from_slice(rbsp);
        }
    }
    Ok(Some(buf))
}

fn start_code_of(nal: &NalData) -> StartCode {
    match nal {
        NalData::Unspecified(sc, ..)
        | NalData::PrefixNalUnit(sc, ..)
        | NalData::SubsetSps(sc, ..)
        | NalData::Dps(sc, ..)
        | NalData::Reserved(sc, ..)
        | NalData::Aux(sc, ..)
        | NalData::SliceExt(sc, ..)
        | NalData::DepthExt(sc, ..)
        | NalData::Unknown(sc, ..)
        | NalData::Raw(sc, ..) => *sc,
        NalData::NonIdr(sc, ..) => *sc,
        NalData::SliceA(sc, ..) => *sc,
        NalData::SliceB(sc, ..) => *sc,
        NalData::SliceC(sc, ..) => *sc,
        NalData::Idr(sc, ..) => *sc,
        NalData::Sei(sc, ..) => *sc,
        NalData::Sps(sc, ..) => *sc,
        NalData::Pps(sc, ..) => *sc,
        NalData::Aud(sc, ..) => *sc,
        NalData::EOSeq(sc, ..) => *sc,
        NalData::EOStream(sc, ..) => *sc,
        NalData::FillerData(sc, ..) => *sc,
        NalData::SpsExt(sc, ..) => *sc,
    }
}

fn push_nal(
    nal_units: &mut Vec<NalData>,
    last_sps: &mut Option<Sps>,
    last_pps: &mut Option<Pps>,
    sc: StartCode,
    nal_unit: NalUnit,
) -> Result<(), Error> {
    let nri = nal_unit.header.nal_ref_idc;
    let nal_type = nal_unit.header.nal_unit_type;
    match nal_type {
        NalUnitType::Unspecified(_) => {
            nal_units.push(NalData::Unspecified(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::NonIdr => {
            if let (Some(sps), Some(pps)) = (last_sps.as_ref(), last_pps.as_ref()) {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let non_idr = NonIdr::parse(&rbsp, sps, pps, nri)?;
                nal_units.push(NalData::NonIdr(sc, nri, Box::new(non_idr)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::SliceA => {
            if let (Some(sps), Some(pps)) = (last_sps.as_ref(), last_pps.as_ref()) {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let slice_a = SliceA::parse(&rbsp, sps, pps, nri)?;
                nal_units.push(NalData::SliceA(sc, nri, Box::new(slice_a)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::SliceB => {
            if let (Some(sps), Some(pps)) = (last_sps.as_ref(), last_pps.as_ref()) {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let slice_b = SliceB::parse(&rbsp, sps, pps)?;
                nal_units.push(NalData::SliceB(sc, nri, Box::new(slice_b)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::SliceC => {
            if let (Some(sps), Some(pps)) = (last_sps.as_ref(), last_pps.as_ref()) {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let slice_c = SliceC::parse(&rbsp, sps, pps)?;
                nal_units.push(NalData::SliceC(sc, nri, Box::new(slice_c)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::Idr => {
            if let (Some(sps), Some(pps)) = (last_sps.as_ref(), last_pps.as_ref()) {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let idr = Idr::parse(&rbsp, sps, pps, nri)?;
                nal_units.push(NalData::Idr(sc, nri, Box::new(idr)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::Sei => {
            let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
            let sei = Sei::parse(&rbsp)?;
            nal_units.push(NalData::Sei(sc, nri, Box::new(sei)));
        }
        NalUnitType::Sps => {
            let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
            let sps = Sps::parse(&rbsp)?;
            *last_sps = Some(sps.clone());
            nal_units.push(NalData::Sps(sc, nri, Box::new(sps)));
        }
        NalUnitType::Pps => {
            if let Some(sps) = last_sps.as_ref() {
                let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
                let pps = Pps::parse(&rbsp, sps)?;
                *last_pps = Some(pps.clone());
                nal_units.push(NalData::Pps(sc, nri, Box::new(pps)));
            } else {
                nal_units.push(NalData::Raw(sc, nri, nal_type, nal_unit.rbsp));
            }
        }
        NalUnitType::Aud => {
            let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
            let aud = Aud::parse(&rbsp)?;
            nal_units.push(NalData::Aud(sc, nri, aud));
        }
        NalUnitType::EOSeq => nal_units.push(NalData::EOSeq(sc, nri)),
        NalUnitType::EOStream => nal_units.push(NalData::EOStream(sc, nri)),
        NalUnitType::FillerData => {
            let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
            let filler = FillerData::parse(&rbsp);
            nal_units.push(NalData::FillerData(sc, nri, filler));
        }
        NalUnitType::SpsExt => {
            let rbsp = NalUnit::remove_emulation_prevention_bytes(&nal_unit.rbsp);
            let sps_ext = SpsExt::parse(&rbsp)?;
            nal_units.push(NalData::SpsExt(sc, nri, sps_ext));
        }
        NalUnitType::PrefixNalUnit => {
            nal_units.push(NalData::PrefixNalUnit(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::SubsetSps => {
            nal_units.push(NalData::SubsetSps(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::Dps => {
            nal_units.push(NalData::Dps(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::Reserved(_) => {
            nal_units.push(NalData::Reserved(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::Aux => {
            nal_units.push(NalData::Aux(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::SliceExt => {
            nal_units.push(NalData::SliceExt(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::DepthExt => {
            nal_units.push(NalData::DepthExt(sc, nri, nal_type, nal_unit.rbsp));
        }
        NalUnitType::Unknown(_) => {
            nal_units.push(NalData::Unknown(sc, nri, nal_type, nal_unit.rbsp));
        }
    }

    Ok(())
}
