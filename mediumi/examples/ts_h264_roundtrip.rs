use mediumi::h264::Processor;
use mediumi::mpeg2ts::api::pes_demuxer;
use mediumi::mpeg2ts::psi::pmt::StreamType;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/test.ts");
    let data = std::fs::read(path).expect("failed to read ts");

    let demuxed = pes_demuxer::demux(&data).expect("failed to demux ts");

    let h264_pid = demuxed
        .pmt
        .streams
        .iter()
        .find(|s| s.stream_type == StreamType::H264)
        .map(|s| s.elementary_pid)
        .expect("no H.264 stream in PMT");

    let mut annex_b = Vec::new();
    let mut pes_packet_count = 0usize;
    for stream in &demuxed.streams {
        let pid = stream
            .fragments
            .first()
            .map(|f| f.ts_header.pid)
            .unwrap_or(0);
        if pid != h264_pid {
            continue;
        }
        annex_b.extend_from_slice(&stream.pes.pes_payload);
        pes_packet_count += 1;
    }
    assert!(
        !annex_b.is_empty(),
        "no PES payload found for H.264 PID {:#06x}",
        h264_pid
    );

    let processor = Processor::from_annex_b(&annex_b).expect("failed to parse Annex.B");
    let re_annex_b = processor.to_annex_b().expect("failed to serialize Annex.B");

    assert_eq!(
        re_annex_b.len(),
        annex_b.len(),
        "Annex.B size mismatch: orig={}, re={}",
        annex_b.len(),
        re_annex_b.len(),
    );
    assert_eq!(re_annex_b, annex_b, "Annex.B bytes differ");

    println!(
        "TS → H.264 roundtrip ok ({} PES packets, {} annex_b bytes, {} NAL units; H.264 PID {:#06x})",
        pes_packet_count,
        re_annex_b.len(),
        processor.nal_units.len(),
        h264_pid,
    );
}
