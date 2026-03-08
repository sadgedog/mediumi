use mediumi_codec::api::adts;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.aac"
    ))
    .expect("failed to read input aac file");

    let result = adts::Processor::parse(&input).expect("failed to parse");
    let output = result.to_bytes();

    assert_eq!(
        input.len(),
        output.len(),
        "roundtrip size mismatch: input={} output={}",
        input.len(),
        output.len()
    );
    assert_eq!(input, output, "roundtrip byte mismatch");

    println!(
        "roundtrip ok ({} bytes, {} frames)",
        input.len(),
        result.adts_frames.len()
    );
}
