use mediumi_mp4::boxes::parse_all;

fn roundtrip(label: &str, path: &str) {
    let original = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[{}] read error: {}", label, e);
            return;
        }
    };

    let boxes = match parse_all(&original) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[{}] parse error: {:?}", label, e);
            return;
        }
    };

    let mut serialized = Vec::new();
    for b in &boxes {
        serialized.extend(b.to_bytes());
    }

    if serialized == original {
        println!(
            "[{}] OK  ({} bytes, {} top-level boxes)",
            label,
            original.len(),
            boxes.len()
        );
        return;
    }

    // mismatch
    let first_diff = original
        .iter()
        .zip(serialized.iter())
        .position(|(a, b)| a != b);
    println!(
        "[{}] NG  (orig={} bytes, round={} bytes, first diff at: {:?})",
        label,
        original.len(),
        serialized.len(),
        first_diff
    );

    // diagnostic: print each top-level box's size comparison
    let mut offset = 0;
    for (i, b) in boxes.iter().enumerate() {
        let bytes = b.to_bytes();
        let original_slice_len = bytes.len().min(original.len().saturating_sub(offset));
        let original_slice = &original[offset..offset + original_slice_len];
        let matches = &bytes[..original_slice_len] == original_slice;
        println!(
            "    [{}] roundtrip len={} matches_orig={} offset={}",
            i,
            bytes.len(),
            matches,
            offset
        );
        offset += bytes.len();
    }
}

fn main() {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data");
    roundtrip("MP4         ", &format!("{}/test.mp4", base));
    roundtrip("fMP4 init   ", &format!("{}/test_init.m4s", base));
    roundtrip("fMP4 segment", &format!("{}/test.m4s", base));
}
