use mediumi_crypto::{Encrypter, Iv, Mode, PsshInput};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Fixture base name (`<ASSET>_init.m4s` / `<ASSET>_segment.m4s`).
const ASSET: &str = "h264";

const KEY_ID: [u8; 16] = [0x11; 16];
const CONTENT_KEY: [u8; 16] = [0x33; 16];
const CONSTANT_IV: [u8; 16] = [0x22; 16]; // cbcs constant IV
/// Hex of `CONTENT_KEY`, passed to ffmpeg's `-decryption_key`.
const KEY_HEX: &str = "33333333333333333333333333333333";
const WIDEVINE_SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

fn main() -> Result<(), Box<dyn Error>> {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data"));
    let result = run(dir);
    cleanup(dir);
    result
}

fn run(dir: &Path) -> Result<(), Box<dyn Error>> {
    let init = fs::read(dir.join(format!("{ASSET}_init.m4s")))?;
    let seg = fs::read(dir.join(format!("{ASSET}_segment.m4s")))?;

    for (scheme, mode) in [
        (
            "cenc",
            Mode::Cenc {
                iv: Iv::Bytes8([0; 8]),
            },
        ),
        (
            "cbcs",
            Mode::Cbcs {
                iv: Iv::Bytes16(CONSTANT_IV),
            },
        ),
    ] {
        println!("[{scheme}] encrypting {ASSET}_init.m4s + {ASSET}_segment.m4s ...");
        let mut enc = Encrypter::new(mode, KEY_ID, CONTENT_KEY);
        enc.add_pssh(PsshInput {
            system_id: WIDEVINE_SYSTEM_ID,
            key_ids: vec![KEY_ID],
            data: b"<test-pssh>".to_vec(),
        });

        let enc_init = enc.enc_init_segment_to_vec(&init)?;
        let mut seg_buf = seg.clone(); // encrypted in place
        let enc_seg = enc.enc_media_segment_to_vec(&init, &mut seg_buf)?;

        let init_path = dir.join(format!("enc_{scheme}_init.m4s"));
        let seg_path = dir.join(format!("enc_{scheme}_segment.m4s"));
        fs::write(&init_path, &enc_init)?;
        fs::write(&seg_path, &enc_seg)?;
        println!(
            "  wrote enc_{scheme}_init.m4s ({} B) + enc_{scheme}_segment.m4s ({} B), {} samples encrypted",
            enc_init.len(),
            enc_seg.len(),
            enc.next_sample_index,
        );

        verify(dir, scheme, &enc_init, &enc_seg)?;
        println!("  [{scheme}] ffmpeg decrypt verification OK\n");
    }
    Ok(())
}

fn verify(dir: &Path, scheme: &str, init: &[u8], seg: &[u8]) -> Result<(), Box<dyn Error>> {
    let full = dir.join(format!("enc_{scheme}_full.mp4"));
    let mut bytes = Vec::with_capacity(init.len() + seg.len());
    bytes.extend_from_slice(init);
    bytes.extend_from_slice(seg);
    fs::write(&full, &bytes)?;

    let probe = Command::new("ffprobe")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-show_entries",
            "stream=codec_type,codec_name",
            "-of",
            "compact=p=0",
        ])
        .arg(&full)
        .output()?;
    print!("  ffprobe: {}", String::from_utf8_lossy(&probe.stdout));

    let dec = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-decryption_key",
            KEY_HEX,
        ])
        .arg("-i")
        .arg(&full)
        .args(["-f", "null", "-"])
        .output()?;
    let _ = fs::remove_file(&full);

    if !dec.status.success() || !dec.stderr.is_empty() {
        return Err(format!(
            "[{scheme}] ffmpeg decrypt-decode failed:\n{}",
            String::from_utf8_lossy(&dec.stderr)
        )
        .into());
    }
    Ok(())
}

/// Remove every `enc_*` artifact this example may have written.
fn cleanup(dir: &Path) {
    for scheme in ["cenc", "cbcs"] {
        for suffix in ["init.m4s", "segment.m4s", "full.mp4"] {
            let _ = fs::remove_file(dir.join(format!("enc_{scheme}_{suffix}")));
        }
    }
}
