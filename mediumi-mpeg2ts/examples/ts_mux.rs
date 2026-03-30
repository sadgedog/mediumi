use mediumi_mpeg2ts::api;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ts"
    ))
    .expect("failed to read input ts file");

    let demuxed = api::ts_demuxer::demux(&input).expect("failed to demux");
    let output = api::ts_muxer::mux(&demuxed).expect("failed to mux");

    assert_eq!(
        input.len(),
        output.len(),
        "roundtrip size mismatch: input={} output={}",
        input.len(),
        output.len()
    );
    assert_eq!(input, output, "failed to roundtrip");

    println!("roundtrip ok ({} packets)", input.len() / 188);
}
