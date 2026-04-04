use mediumi_h264::{NalData, Processor};

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.h264"
    ))
    .expect("failed to read input h264 file");

    let result = Processor::parse(&input).expect("failed to parse");

    println!(
        "{} NAL Units ({}) {}",
        "-".repeat(20),
        result.nal_units.len(),
        "-".repeat(20)
    );
    for (i, nal) in result.nal_units.iter().enumerate() {
        match nal {
            NalData::Sps(sc, nri, sps) => {
                println!(
                    "[{}] Type: SPS, StartCode: {:?}, NRI: {}, SPS: {:?}",
                    i, sc, nri, sps
                );
            }
            NalData::Pps(sc, nri, pps) => {
                println!(
                    "[{}] Type: PPS, StartCode: {:?}, NRI: {}, PPS: {:?}",
                    i, sc, nri, pps
                );
            }
            NalData::Aud(sc, nri, aud) => {
                println!(
                    "[{}] Type: AUD, StartCode: {:?}, NRI: {}, AUD: {:?}",
                    i, sc, nri, aud
                );
            }
            NalData::Raw(_, nri, nal_type, rbsp) => {
                println!(
                    "[{}] Type: {:?}, NRI: {}, RBSP size: {} bytes",
                    i,
                    nal_type,
                    nri,
                    rbsp.len(),
                );
            }
        }
    }
}
