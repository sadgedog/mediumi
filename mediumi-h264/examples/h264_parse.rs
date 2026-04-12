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
            NalData::NonIdr(sc, nri, non_idr) => {
                println!(
                    "[{}] Type: NonIDR, StartCode: {:?}, NRI: {}, SliceHeader: {:?}",
                    i, sc, nri, non_idr.slice_header
                );
            }
            NalData::SliceA(sc, nri, slice_a) => {
                println!(
                    "[{}] Type: SliceA, StartCode: {:?}, NRI: {}, slice_id: {}",
                    i, sc, nri, slice_a.slice_id
                );
            }
            NalData::SliceB(sc, nri, slice_b) => {
                println!(
                    "[{}] Type: SliceB, StartCode: {:?}, NRI: {}, slice_id: {}",
                    i, sc, nri, slice_b.slice_id
                );
            }
            NalData::SliceC(sc, nri, slice_c) => {
                println!(
                    "[{}] Type: SliceC, StartCode: {:?}, NRI: {}, slice_id: {}",
                    i, sc, nri, slice_c.slice_id
                );
            }
            NalData::Idr(sc, nri, idr) => {
                println!(
                    "[{}] Type: IDR, StartCode: {:?}, NRI: {}, SliceHeader: {:?}",
                    i, sc, nri, idr.slice_header
                );
            }
            NalData::Sei(sc, nri, sei) => {
                println!(
                    "[{}] Type: SEI, StartCode: {:?}, NRI: {}, messages: {}",
                    i,
                    sc,
                    nri,
                    sei.sei_message.len()
                );
            }
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
            NalData::EOSeq(sc, nri) => {
                println!("[{}] Type: EOSeq, StartCode: {:?}, NRI: {}", i, sc, nri);
            }
            NalData::EOStream(sc, nri) => {
                println!("[{}] Type: EOStream, StartCode: {:?}, NRI: {}", i, sc, nri);
            }
            NalData::FillerData(sc, nri, filler) => {
                println!(
                    "[{}] Type: FillerData, StartCode: {:?}, NRI: {}, ff_bytes: {}",
                    i, sc, nri, filler.ff_byte_count
                );
            }
            NalData::SpsExt(sc, nri, sps_ext) => {
                println!(
                    "[{}] Type: SPSExt, StartCode: {:?}, NRI: {}, SpsExt: {:?}",
                    i, sc, nri, sps_ext
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
