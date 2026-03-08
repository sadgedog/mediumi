use mediumi_mpeg2ts::api;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ts"
    ))
    .expect("failed to read input ts file");

    let result = api::ts_decoder::decode(&input).expect("failed to decode");

    for (i, packet) in result.packets.iter().enumerate() {
        println!("{} Stream {} {}", "=".repeat(40), i, "=".repeat(40));

        println!("TS Header: {:?}", packet.header);
        println!("TS AdaptationField: {:?}", packet.adaptation_field);
        println!("TS Payload: {:?}", packet.payload);
    }
}
