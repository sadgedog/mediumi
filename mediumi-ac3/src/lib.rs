//! AC-3 codec processor for parsing and serializing streams.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_ac3::Processor;
//!
//! let data = std::fs::read("input.ac3")?;
//! let processor = Processor::parse(&data)?;
//! let output = processor.to_bytes()?;
//! # Ok(())
//! # }
//! ```

pub mod bap;
pub mod error;
pub mod frame;
pub mod mantissa;
pub mod util;

use crate::{error::Error, frame::Ac3};

#[derive(Debug)]
pub struct Processor {
    pub ac3_frames: Vec<Ac3>,
}

impl Processor {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        for frame in &self.ac3_frames {
            buf.extend_from_slice(&frame.to_bytes()?);
        }

        Ok(buf)
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut ac3_frames = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let remaining = &data[offset..];
            let frame_size = frame::peek_frame_size(remaining).ok_or(Error::InvalidFrameSize)?;
            if remaining.len() < frame_size {
                return Err(Error::DataTooShort);
            }
            let frame = Ac3::parse(&remaining[..frame_size])?;
            offset += frame_size;
            ac3_frames.push(frame);
        }

        Ok(Self { ac3_frames })
    }
}
