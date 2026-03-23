//! Demux raw TS byte stream into individual TS packet structs.

use crate::{api::error::Error, ts, ts::packet::Packet};

#[derive(Debug)]
pub struct Demuxed {
    pub packets: Vec<Packet>,
}

pub fn demux(data: &[u8]) -> Result<Demuxed, Error> {
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

    Ok(Demuxed { packets })
}
