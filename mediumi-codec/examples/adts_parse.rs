use mediumi_codec::api::adts;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.aac"
    ))
    .expect("failed to read input aac file");

    let result = adts::Processor::parse(&input).expect("failed to parse");

    println!(
        "{} ADTS Frames ({}) {}",
        "-".repeat(20),
        result.adts_frames.len(),
        "-".repeat(20)
    );
    for (i, frame) in result.adts_frames.iter().enumerate() {
        println!(
            "[{}] profile: {}, sf_idx: {}, ch: {}, frame_length: {}, payload: {} bytes",
            i,
            frame.profile,
            frame.sampling_frequency_index,
            frame.channel_configuration,
            frame.aac_frame_length,
            frame.payload.len(),
        );
    }
}
