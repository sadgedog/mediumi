use mediumi_codec::api::ac3::Processor;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ac3"
    ))
    .expect("failed to read input ac3 file");

    let result = Processor::parse(&input).expect("failed to parse");
    let output = result.to_bytes().expect("failed to serialize");

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
        result.ac3_frames.len(),
    );
}
