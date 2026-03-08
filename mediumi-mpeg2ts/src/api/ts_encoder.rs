//! Encode Decoded data (TS units) back into TS byte stream.
//!
//! Note: This encoder preserve the original TS packet interleaving

use crate::api::error::Error;
use crate::api::ts_decoder::Decoded;

pub fn encode(decoded: &Decoded) -> Result<Vec<u8>, Error> {
    let mut output = Vec::with_capacity(decoded.packets.len() * 188);

    for packet in &decoded.packets {
        output.extend_from_slice(&packet.to_bytes());
    }

    Ok(output)
}
