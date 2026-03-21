use crate::{adts::Adts, error::Error};

#[derive(Debug)]
pub struct Processor {
    pub adts_frames: Vec<Adts>,
}

impl Processor {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        for f in &self.adts_frames {
            buf.extend_from_slice(&f.to_bytes());
        }

        buf
    }

    pub fn parse(pes_payload: &[u8]) -> Result<Self, Error> {
        let mut adts_frames = Vec::new();
        let mut offset = 0;

        while offset < pes_payload.len() {
            let frame = Adts::parse(&pes_payload[offset..])?;
            let frame_length = frame.aac_frame_length as usize;
            if frame_length == 0 {
                return Err(Error::DataTooShort);
            }
            offset += frame_length;
            adts_frames.push(frame);
        }

        Ok(Self { adts_frames })
    }
}
