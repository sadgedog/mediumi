//! Mux TS packets back into TS byte stream.
//!
//! Note: This muxer preserves the original TS packet interleaving.

use crate::api::{error::Error, ts_demuxer::Demuxed};

pub fn mux(demuxed: &Demuxed) -> Result<Vec<u8>, Error> {
    let mut output = Vec::with_capacity(demuxed.packets.len() * 188);

    for packet in &demuxed.packets {
        output.extend_from_slice(&packet.to_bytes());
    }

    Ok(output)
}
