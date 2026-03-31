use std::fs;
use std::process::Command;

use mediumi_mpeg2ts::api;

fn check_with_ffmpeg(path: &str) -> bool {
    let result = Command::new("ffmpeg")
        .args(["-v", "error", "-i", path, "-f", "null", "-"])
        .output()
        .expect("failed to execute ffmpeg");

    let stderr = String::from_utf8_lossy(&result.stderr);
    if !stderr.is_empty() {
        eprintln!("ffmpeg errors:\n{}", stderr);
        return false;
    }
    result.status.success()
}

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ts"
    ))
    .expect("failed to read input ts file");
    let demuxed = api::pes_demuxer::demux(&input).expect("failed to demux");
    let muxed = api::pes_muxer::mux(&demuxed).expect("failed to mux");

    let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/ts_output.ts");
    // write muxed data
    fs::write(output_path, &muxed).expect("failed to write muxed result");

    if !check_with_ffmpeg(output_path) {
        eprintln!("ffmpeg check failed!");
    }
    // remove muxed data
    fs::remove_file(output_path).expect("failed to remove output file");

    println!("pes mux ok ({} pes)", demuxed.streams.len());
}
