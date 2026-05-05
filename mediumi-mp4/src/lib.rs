//! mp4 demuxer / muxer.
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_mp4::{demuxer, muxer};
//!
//! let data = std::fs::read("input.mp4")?;
//! let boxes = demuxer::demux(&data)?;
//! let output = muxer::mux(&boxes);
//! std::fs::write("output.mp4", output)?;
//! # Ok(())
//! # }
//! ```

pub mod boxes;
pub mod types;
pub mod util;

pub use boxes::error::Error;
pub use boxes::{BaseBox, BoxHeader, BoxSize, FullBox, Mp4Box};

pub mod demuxer {
    use crate::boxes::{Mp4Box, error::Error, parse_all};

    /// Parse a complete mp4 byte stream into a list of top-level boxes.
    pub fn demux(data: &[u8]) -> Result<Vec<Mp4Box>, Error> {
        parse_all(data)
    }
}

pub mod muxer {
    use crate::boxes::{Mp4Box, to_bytes_all};

    /// Serialize a list of top-level boxes back into an mp4 byte stream.
    pub fn mux(boxes: &[Mp4Box]) -> Vec<u8> {
        to_bytes_all(boxes)
    }
}
