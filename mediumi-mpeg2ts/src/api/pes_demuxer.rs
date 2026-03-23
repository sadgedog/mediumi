//! Demux TS packets into PAT, PMT and PES streams.

use std::collections::{BTreeMap, HashSet};

use crate::{api::error::Error, pes, psi::pat::Pat, psi::pmt::Pmt, ts};

#[derive(Debug)]
pub struct TsFragment {
    pub ts_header: ts::packet::Header,
    pub adaptation_field: Option<ts::packet::AdaptationField>,
}

#[derive(Debug)]
pub struct PesPacket {
    pub pes_header: pes::header::Header,
    pub pes_payload: Vec<u8>,
}

#[derive(Debug)]
pub struct Stream {
    pub fragments: Vec<TsFragment>,
    pub pes: PesPacket,
}

#[derive(Debug)]
pub struct Demuxed {
    pub pat: Pat,
    pub pmt: Pmt,
    pub streams: Vec<Stream>,
}

#[derive(Debug)]
struct Assembler {
    fragments: Vec<TsFragment>,
    buffer: Vec<u8>,
    started: bool,
}

/// Assemble TS packets into PES stream fo a single PID
impl Assembler {
    fn new() -> Self {
        Self {
            fragments: Vec::new(),
            buffer: Vec::new(),
            started: false,
        }
    }

    fn push(&mut self, packet: ts::packet::Packet) -> Option<Result<Stream, Error>> {
        let pusi = packet.header.payload_unit_start_indicator;
        let fragment = TsFragment {
            ts_header: packet.header,
            adaptation_field: packet.adaptation_field,
        };

        if pusi {
            let completed = if self.started {
                Some(self.build())
            } else {
                None
            };
            self.fragments.clear();
            self.buffer.clear();
            self.fragments.push(fragment);
            self.buffer.extend_from_slice(&packet.payload);
            self.started = true;
            completed
        } else if self.started {
            self.fragments.push(fragment);
            self.buffer.extend_from_slice(&packet.payload);
            None
        } else {
            None
        }
    }

    fn flush(&mut self) -> Option<Result<Stream, Error>> {
        if self.started {
            self.started = false;
            Some(self.build())
        } else {
            None
        }
    }

    fn build(&mut self) -> Result<Stream, Error> {
        let (pes_header, consumed) = pes::header::Header::parse(&self.buffer)?;
        let pes_payload = self.buffer[consumed..].to_vec();
        let fragments = std::mem::take(&mut self.fragments);
        let pes = PesPacket {
            pes_header,
            pes_payload,
        };
        Ok(Stream { fragments, pes })
    }
}

/// Demux TS packets
pub fn demux(data: &[u8]) -> Result<Demuxed, Error> {
    if !data.len().is_multiple_of(188) {
        return Err(Error::InvalidPacketsLength(data.len()));
    }

    let packet_num = data.len() / 188;
    let mut pat: Option<Pat> = None;
    let mut pmt: Option<Pmt> = None;
    let mut pmt_pids: HashSet<u16> = HashSet::new();
    let mut assemblers: BTreeMap<u16, Assembler> = BTreeMap::new();
    let mut streams: Vec<Stream> = Vec::new();

    for i in 0..packet_num {
        let start = i * 188;
        let packet = ts::packet::Packet::parse(&data[start..start + 188])?;
        let pid = packet.header.pid;

        // PAT
        if pid == 0x0000 && packet.header.payload_unit_start_indicator {
            let pointer_field = packet.payload[0] as usize;
            let parsed = Pat::parse(&packet.payload[1 + pointer_field..])?;
            for program in &parsed.programs {
                pmt_pids.insert(program.pid);
            }
            pat = Some(parsed);
            continue;
        }

        // PMT
        if pmt_pids.contains(&pid) && packet.header.payload_unit_start_indicator {
            let pointer_field = packet.payload[0] as usize;
            let parsed = Pmt::parse(&packet.payload[1 + pointer_field..])?;
            for stream in &parsed.streams {
                assemblers
                    .entry(stream.elementary_pid)
                    .or_insert_with(Assembler::new);
            }
            pmt = Some(parsed);
            continue;
        }

        // Stream
        if let Some(assembler) = assemblers.get_mut(&pid)
            && let Some(result) = assembler.push(packet)
        {
            streams.push(result?);
        }
    }

    // Flush remaining PES
    for assembler in assemblers.values_mut() {
        if let Some(result) = assembler.flush() {
            streams.push(result?);
        }
    }

    Ok(Demuxed {
        pat: pat.ok_or(Error::PatNotFound)?,
        pmt: pmt.ok_or(Error::PmtNotFound)?,
        streams,
    })
}
