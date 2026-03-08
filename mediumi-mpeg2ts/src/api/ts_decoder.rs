//! Parse raw TS byte stream into individual TS packet structs.

use crate::ts;
use crate::ts::packet::Packet;

use crate::api::error::Error;

#[derive(Debug)]
pub struct Decoded {
    pub packets: Vec<Packet>,
}

pub fn decode(data: &[u8]) -> Result<Decoded, Error> {
    if !data.len().is_multiple_of(188) {
        return Err(Error::InvalidPacketsLength(data.len()));
    }

    let packet_num = data.len() / 188;
    let mut packets = Vec::with_capacity(packet_num);

    for i in 0..packet_num {
        let start = i * 188;
        let ts_packet = ts::packet::Packet::parse(&data[start..start + 188])?;
        packets.push(ts_packet);
    }

    Ok(Decoded { packets })
}
