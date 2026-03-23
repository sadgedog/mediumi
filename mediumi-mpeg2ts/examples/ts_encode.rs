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

    let demuxed = api::ts_demuxer::demux(&input).expect("failed to demux");
    let output = api::ts_muxer::mux(&demuxed).expect("failed to mux");

    let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/ts_output.ts");
    fs::write(output_path, &output).expect("failed to write encoded result");

    if !check_with_ffmpeg(output_path) {
        eprintln!("ffmpeg check failed!");
    }
}
