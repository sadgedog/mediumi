use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mediumi_mp4::{Mp4Box, demuxer, muxer};

fn check_with_ffmpeg(path: &Path) -> bool {
    let result = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .expect("failed to execute ffmpeg");

    let stderr = String::from_utf8_lossy(&result.stderr);
    if !stderr.is_empty() {
        eprintln!("ffmpeg errors:\n{}", stderr);
        return false;
    }
    result.status.success()
}

fn tmp_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{}_tmp.{}", stem, ext))
}

fn mux_file(label: &str, input: &Path) -> Option<(Vec<Mp4Box>, Vec<u8>)> {
    let original = match fs::read(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[{}] read error: {}", label, e);
            return None;
        }
    };
    let boxes = match demuxer::demux(&original) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[{}] demux error: {:?}", label, e);
            return None;
        }
    };
    let muxed = muxer::mux(&boxes);
    Some((boxes, muxed))
}

fn check_standalone(label: &str, input: &Path) {
    let Some((boxes, muxed)) = mux_file(label, input) else {
        return;
    };

    let output_path = tmp_path(input);
    fs::write(&output_path, &muxed).expect("failed to write muxed result");
    let ok = check_with_ffmpeg(&output_path);
    fs::remove_file(&output_path).expect("failed to remove output file");

    if ok {
        println!(
            "[{}] mux ok ({} bytes, {} top-level boxes)",
            label,
            muxed.len(),
            boxes.len()
        );
    } else {
        eprintln!("[{}] ffmpeg check failed!", label);
    }
}

fn check_fragmented(label: &str, init: &Path, segment: &Path) {
    let Some((init_boxes, init_muxed)) = mux_file(label, init) else {
        return;
    };
    let Some((seg_boxes, seg_muxed)) = mux_file(label, segment) else {
        return;
    };

    let mut combined = init_muxed.clone();
    combined.extend_from_slice(&seg_muxed);

    let parent = init.parent().unwrap_or_else(|| Path::new("."));
    let output_path = parent.join("fmp4_combined_tmp.mp4");
    fs::write(&output_path, &combined).expect("failed to write combined result");
    let ok = check_with_ffmpeg(&output_path);
    fs::remove_file(&output_path).expect("failed to remove output file");

    if ok {
        println!(
            "[{}] mux ok (init: {} bytes / {} boxes, segment: {} bytes / {} boxes)",
            label,
            init_muxed.len(),
            init_boxes.len(),
            seg_muxed.len(),
            seg_boxes.len(),
        );
    } else {
        eprintln!("[{}] ffmpeg check failed!", label);
    }
}

fn main() {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data");
    check_standalone("MP4 ", &PathBuf::from(format!("{}/test.mp4", base)));
    check_fragmented(
        "fMP4",
        &PathBuf::from(format!("{}/test_init.m4s", base)),
        &PathBuf::from(format!("{}/test.m4s", base)),
    );
}
