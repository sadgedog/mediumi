use crate::encrypter::{Encrypter, Mode};
use crate::error::Error;
use mediumi_mp4::boxes::{
    FullBoxHeader, frma::Frma, schi::Schi, schm::Schm, sinf::Sinf, tenc::Tenc,
};
use mediumi_mp4::sample_entry::wrap_with_sinf;
use mediumi_mp4::{Mp4Box, demuxer, muxer};

/// CMAF `cenc` per-sample IV size (8-byte IV in the high half of the counter).
const CENC_PER_SAMPLE_IV_SIZE: u8 = 8;
/// `cbcs` pattern: encrypt 1 of every 10 16-byte blocks
const CBCS_CRYPT_BYTE_BLOCK: u8 = 1;
const CBCS_SKIP_BYTE_BLOCK: u8 = 9;

pub(crate) fn enc_init(enc: &Encrypter, moov_bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut boxes = demuxer::demux(moov_bytes)?;
    let mut found_moov = false;
    let mut wrapped = 0usize;

    for b in &mut boxes {
        let Mp4Box::Moov(moov) = b else { continue };
        found_moov = true;

        for trak in &mut moov.traks {
            let Some(stbl) = trak.mdia.minf.stbl.as_mut() else {
                continue;
            };
            let Some(entry) = stbl.stsd.entries.first_mut() else {
                continue;
            };
            let original = entry.box_type;
            if !is_encryptable(&original) {
                continue;
            }
            let sinf = build_sinf(enc, original);
            if wrap_with_sinf(entry, &sinf).is_some() {
                wrapped += 1;
            }
        }

        for input in &enc.pssh_inputs {
            moov.pssh.push(input.to_pssh());
        }
    }

    if !found_moov {
        return Err(Error::NoMoov);
    }
    if wrapped == 0 {
        return Err(Error::NoEncryptableTrack);
    }

    Ok(muxer::mux(&boxes))
}

fn is_encryptable(fourcc: &[u8; 4]) -> bool {
    matches!(fourcc, b"avc1" | b"avc3" | b"mp4a")
}

fn build_sinf(enc: &Encrypter, original_box_type: [u8; 4]) -> Sinf {
    let (scheme_type, tenc) = match enc.mode {
        // cenc: AES-CTR, per-sample IV, no pattern → tenc version 0.
        Mode::Cenc => (
            *b"cenc",
            Tenc {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                default_crypt_byte_block: 0,
                default_skip_byte_block: 0,
                default_is_protected: 1,
                default_per_sample_iv_size: CENC_PER_SAMPLE_IV_SIZE,
                default_kid: enc.key_id,
                default_constant_iv: None,
            },
        ),
        // cbcs: AES-CBC, constant IV, 1:9 pattern → tenc version 1 (the pattern
        // nibbles are only emitted for version 1) with per_sample_iv_size = 0
        // so the constant IV in tenc applies to every sample.
        Mode::Cbcs => (
            *b"cbcs",
            Tenc {
                header: FullBoxHeader {
                    version: 1,
                    flags: 0,
                },
                default_crypt_byte_block: CBCS_CRYPT_BYTE_BLOCK,
                default_skip_byte_block: CBCS_SKIP_BYTE_BLOCK,
                default_is_protected: 1,
                default_per_sample_iv_size: 0,
                default_kid: enc.key_id,
                default_constant_iv: Some(enc.iv.to_vec()),
            },
        ),
    };
    Sinf {
        frma: Frma {
            data_format: original_box_type,
        },
        schm: Schm {
            header: FullBoxHeader {
                version: 0,
                flags: 0,
            },
            scheme_type,
            scheme_version: 0x0001_0000,
            scheme_uri: None,
        },
        schi: Some(Schi {
            tenc: Some(tenc),
            others: Vec::new(),
        }),
        others: Vec::new(),
    }
}
