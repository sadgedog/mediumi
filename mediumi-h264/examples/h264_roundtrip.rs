use mediumi_h264::Processor;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.h264"
    ))
    .expect("failed to read input h264 file");

    // parse
    let result = Processor::parse(&input).expect("failed to parse");
    // write
    let output = result.to_bytes();

    // length check
    assert_eq!(
        input.len(),
        output.len(),
        "roundtrip size mismatch: input={} output={}",
        input.len(),
        output.len()
    );
    // binary compativility
    assert_eq!(input, output, "roundtrip byte mismatch");

    println!("roundtrip ok ({} bytes)", input.len());
}
