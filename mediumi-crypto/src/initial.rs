use crate::encrypter::{Encrypter, Mode};
use crate::error::Error;
use crate::subsample;
use mediumi_mp4::boxes::{
    FullBoxHeader, frma::Frma, moov::Moov, schi::Schi, schm::Schm, sinf::Sinf, tenc::Tenc,
};
use mediumi_mp4::sample_entry::wrap_with_sinf;
use mediumi_mp4::{Mp4Box, demuxer, muxer};

pub(crate) fn enc_init(enc: &Encrypter, moov_bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut boxes = demuxer::demux(moov_bytes)?;
    let mut found_moov = false;
    for b in &mut boxes {
        if let Mp4Box::Moov(moov) = b {
            enc_init_moov(enc, moov)?;
            found_moov = true;
        }
    }
    if !found_moov {
        return Err(Error::NoMoov);
    }
    Ok(muxer::mux(&boxes))
}

pub(crate) fn enc_init_moov(enc: &Encrypter, moov: &mut Moov) -> Result<(), Error> {
    let mut wrapped = 0usize;
    for trak in &mut moov.traks {
        let Some(stbl) = trak.mdia.minf.stbl.as_mut() else {
            continue;
        };
        let Some(entry) = stbl.stsd.entries.first_mut() else {
            continue;
        };
        let original = entry.box_type;
        if !subsample::is_encryptable(&original) {
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

    if wrapped == 0 {
        return Err(Error::NoEncryptableTrack);
    }
    Ok(())
}

fn build_sinf(enc: &Encrypter, original_box_type: [u8; 4]) -> Sinf {
    let (scheme_type, tenc) = match &enc.mode {
        // cenc: AES-CTR, per-sample IV (8 or 16 bytes), no pattern → tenc version 0.
        Mode::Cenc { iv } => (
            *b"cenc",
            Tenc {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                default_crypt_byte_block: 0,
                default_skip_byte_block: 0,
                default_is_protected: 1,
                default_per_sample_iv_size: iv.size(), // 8 or 16
                default_kid: enc.key_id,
                default_constant_iv: None,
            },
        ),
        // cbcs: AES-CBC, constant IV (8 or 16 bytes), codec-specific pattern → tenc version 1.
        Mode::Cbcs { iv } => {
            let (crypt_bb, skip_bb) = subsample::media_kind(&original_box_type)
                .map_or((0, 0), |kind| kind.cbcs_pattern());
            (
                *b"cbcs",
                Tenc {
                    header: FullBoxHeader {
                        version: 1,
                        flags: 0,
                    },
                    default_crypt_byte_block: crypt_bb,
                    default_skip_byte_block: skip_bb,
                    default_is_protected: 1,
                    default_per_sample_iv_size: 0,
                    default_kid: enc.key_id,
                    default_constant_iv: Some(iv.as_bytes().to_vec()), // 8 or 16 bytes
                },
            )
        }
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
