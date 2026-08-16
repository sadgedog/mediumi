//! H.265/HEVC parser and serializer.
//!
//! Parsing is layered:
//!
//! ```text
//! [annex_b]  start code split           (Annex B stream ⇔ NAL units)
//! [nal]      2-byte header / RBSP       (NAL unit ⇔ header + raw payload)
//! [lib]      per-type dispatch          (NalUnitType → structured payload)
//! ```

pub mod annex_b;
pub mod aud;
pub mod error;
pub mod nal;

use crate::annex_b::{AnnexB, StartCode};
use crate::aud::Aud;
use crate::error::Error;
use crate::nal::{Header, NalUnit, NalUnitType};

/// Structured payload of a NAL unit, dispatched by `nal_unit_type`.
#[derive(Debug)]
pub enum NalPayload {
    Aud(Aud),
    Unknown(Vec<u8>),
}

impl NalPayload {
    /// Dispatch on the NAL unit type and parse the payload.
    pub fn parse(nal_unit_type: &NalUnitType, payload: &[u8]) -> Result<Self, Error> {
        match nal_unit_type {
            NalUnitType::AudNut => {
                let rbsp = nal::remove_emulation_prevention_bytes(payload);
                Ok(NalPayload::Aud(Aud::parse(&rbsp)?))
            }
            _ => Ok(NalPayload::Unknown(payload.to_vec())),
        }
    }

    /// Serialize the payload back to NAL payload bytes (emulation prevention
    /// bytes re-inserted for structured variants).
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            NalPayload::Aud(aud) => nal::insert_emulation_prevention_bytes(&aud.to_bytes()),
            NalPayload::Unknown(raw) => raw.clone(),
        }
    }
}

/// A fully parsed NAL unit: 2-byte header + structured payload.
#[derive(Debug)]
pub struct NalData {
    pub header: Header,
    pub payload: NalPayload,
}

impl NalData {
    /// Parse one NAL unit (without start code / length prefix).
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let nal = NalUnit::parse(data)?;
        let payload = NalPayload::parse(&nal.header.nal_unit_type, &nal.rbsp)?;
        Ok(Self {
            header: nal.header,
            payload,
        })
    }

    /// Serialize back to NAL unit bytes (header + payload).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.header.to_bytes();
        buf.extend_from_slice(&self.payload.to_bytes());
        buf
    }
}

/// One entry of an Annex B stream: start code + fully parsed NAL unit.
#[derive(Debug)]
pub struct AnnexBNal {
    pub start_code: StartCode,
    pub nal: NalData,
}

impl AnnexBNal {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.start_code.as_bytes());
        buf.extend_from_slice(&self.nal.to_bytes());
        buf
    }
}

/// Parse a raw Annex B byte stream into fully parsed NAL units.
pub fn parse_annex_b(data: &[u8]) -> Result<Vec<AnnexBNal>, Error> {
    annex_b::parse_all(data)?
        .into_iter()
        .map(
            |AnnexB {
                 start_code,
                 nal_unit,
             }| {
                let payload = NalPayload::parse(&nal_unit.header.nal_unit_type, &nal_unit.rbsp)?;
                Ok(AnnexBNal {
                    start_code,
                    nal: NalData {
                        header: nal_unit.header,
                        payload,
                    },
                })
            },
        )
        .collect()
}

/// Serialize parsed NAL units back into an Annex B byte stream.
pub fn serialize_annex_b(nals: &[AnnexBNal]) -> Vec<u8> {
    nals.iter().flat_map(|n| n.to_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aud::PicType;

    // AUD NAL: header(type=35, layer=0, tid+1=1) = [0x46, 0x01], pic_type=PI = [0x30]
    const AUD_NAL: &[u8] = &[0x46, 0x01, 0x30];

    #[test]
    fn nal_data_parses_aud() {
        let nal = NalData::parse(AUD_NAL).expect("parse");
        assert_eq!(nal.header.nal_unit_type, NalUnitType::AudNut);
        let NalPayload::Aud(aud) = &nal.payload else {
            panic!("expected Aud payload, got {:?}", nal.payload);
        };
        assert_eq!(aud.pic_type, PicType::PI);
        assert_eq!(nal.to_bytes(), AUD_NAL);
    }

    #[test]
    fn nal_data_preserves_unknown_type() {
        let bytes = [0x42, 0x01, 0xde, 0xad, 0xbe, 0xef];
        let nal = NalData::parse(&bytes).expect("parse");
        assert_eq!(nal.header.nal_unit_type, NalUnitType::SpsNut);
        assert!(matches!(&nal.payload, NalPayload::Unknown(raw) if raw == &bytes[2..]));
        assert_eq!(nal.to_bytes(), bytes);
    }

    #[test]
    fn annex_b_stream_full_roundtrip() {
        let mut stream = Vec::new();
        stream.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        stream.extend_from_slice(AUD_NAL);
        stream.extend_from_slice(&[0x00, 0x00, 0x01]);
        stream.extend_from_slice(&[0x42, 0x01, 0xde, 0xad]);

        let nals = parse_annex_b(&stream).expect("parse_annex_b");
        assert_eq!(nals.len(), 2);
        assert!(matches!(nals[0].nal.payload, NalPayload::Aud(_)));
        assert!(matches!(nals[1].nal.payload, NalPayload::Unknown(_)));

        assert_eq!(serialize_annex_b(&nals), stream);
    }
}
