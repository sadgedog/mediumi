//! Mux demuxed data (PES packets) back into TS byte stream.
//!
//! Reconstructs 188-byte TS packets from demuxed PAT, PMT, and PES streams.
//!
//! Note: This muxer does not preserve the original TS packet interleaving.
//! Each PES is written as consecutive TS packets, which changes the packet
//! ordering compared to the original stream where audio/video packets are interleaved.
//! PAT/PMT are also deduplicated to a single instance.

use crate::api::{
    error::Error,
    pes_demuxer::{Demuxed, Stream},
};

/// Mux demuxed data back to TS byte stream
pub fn mux(demuxed: &Demuxed) -> Result<Vec<u8>, Error> {
    let mut output = Vec::new();

    // PAT
    mux_section(&demuxed.pat.to_bytes(), 0x0000, &mut output);

    // PMT
    let pmt_pid = demuxed.pat.programs[0].pid;
    mux_section(&demuxed.pmt.to_bytes(), pmt_pid, &mut output);

    // PES streams (fragments reuse mode)
    for stream in &demuxed.streams {
        mux_stream(stream, &mut output)?;
    }

    Ok(output)
}

/// Mux PSI section (PAT/PMT) into TS packets
fn mux_section(section: &[u8], pid: u16, output: &mut Vec<u8>) {
    let mut packet = [0xFF; 188];

    // Sync byte
    packet[0] = 0x47;
    // PUSI=1, PID
    packet[1] = 0b0100_0000 | ((pid >> 8) as u8 & 0b0001_1111);
    packet[2] = pid as u8;
    // No AF, has payload, continuity_counter=0
    packet[3] = 0b0001_0000;
    // Pointer field
    packet[4] = 0x00;
    // Section data
    packet[5..5 + section.len()].copy_from_slice(section);

    output.extend_from_slice(&packet);
}

/// Mux PES stream using original TS fragment metadata
fn mux_stream(stream: &Stream, output: &mut Vec<u8>) -> Result<(), Error> {
    let pes_header_bytes = stream.pes.pes_header.to_bytes();
    let pes_bytes: Vec<u8> = [&pes_header_bytes[..], &stream.pes.pes_payload[..]].concat();

    let mut offset = 0;
    for fragment in &stream.fragments {
        let mut packet = [0xFF; 188];

        // TS header (4 bytes)
        let header_bytes = fragment.ts_header.to_bytes();
        packet[0..4].copy_from_slice(&header_bytes);

        let mut pos = 4;

        // Adaptation field
        if let Some(af) = &fragment.adaptation_field {
            let af_bytes = af.to_bytes();
            packet[pos..pos + af_bytes.len()].copy_from_slice(&af_bytes);
            pos += af_bytes.len();
        }

        // Payload (remaining PES bytes)
        let payload_len = 188 - pos;
        let end = (offset + payload_len).min(pes_bytes.len());
        if offset < end {
            packet[pos..pos + (end - offset)].copy_from_slice(&pes_bytes[offset..end]);
        }
        offset = end;

        output.extend_from_slice(&packet);
    }

    Ok(())
}
