use mediumi_mpeg2ts::api;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ts"
    ))
    .expect("failed to read input ts file");

    let result = api::pes_demuxer::demux(&input).expect("failed to demux");

    println!("PAT: {:?}", result.pat);
    println!("PMT: {:?}\n", result.pmt);
    for (i, stream) in result.streams.iter().enumerate() {
        println!("{} Stream {} {}", "=".repeat(40), i, "=".repeat(40));
        for fragment in &stream.fragments {
            println!("TS Header: {:?}", fragment.ts_header);
            println!("TS AdaptationField: {:?}", fragment.adaptation_field);
        }

        println!("PES Header len: {:?}", stream.pes.pes_header);
        println!("PES Payload len: {:?}\n", stream.pes.pes_payload.len());
    }
}
