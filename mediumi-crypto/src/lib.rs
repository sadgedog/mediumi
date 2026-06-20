//! fmp4 / cmaf media encryption
//!
//! Encrypts fmp4/cmaf init and media segments under the `cenc` (AES-128-CTR)
//! and `cbcs` (AES-128-CBC + 1:9 crypt/skip pattern) schemes, for AVC video
//! and AAC audio (full sample).
//!
//! # CAUTION: IV uniqueness (cenc)
//!
//! For `cenc` (AES-CTR) the per-sample IV must be unique for **every sample
//! across all segments** under a given key — a repeat reuses the keystream and
//! leaks plaintext via XOR (the "many-time pad" failure mode). What matters is
//! that the IV never repeats;
//!
//! **8-byte IV** — zero-padded to the 16-byte counter block:
//! - IV in the high 8 bytes, low 8 bytes are zero
//!   (e.g. `01 23 45 67 89 AB CD EF  00 00 00 00 00 00 00 00`)
//! - high 8 bytes incremented per sample (`base + sample_index`)
//! - low 8 bytes incremented per 16-byte block (by the cipher)
//!
//! **16-byte IV** — used directly as the 16-byte counter block (no padding):
//! - the whole 16 bytes are one 128-bit counter (`base + cumulative_block_offset`)
//! - incremented per 16-byte block across the whole track
//!
//! `cbcs` uses a constant IV and is unaffected.
//!
//! # Example — cenc
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_crypto::{Encrypter, Iv, Mode, PsshInput};
//!
//! const KEY_ID: [u8; 16] = [0x11; 16];
//! const KEY: [u8; 16] = [0x33; 16];
//! const SPECIFIC_DRM_SYSTEM_ID: [u8; 16] = [0xab; 16];
//!
//! // One Encrypter per track; reuse it across every segment (IV uniqueness).
//! let mut enc = Encrypter::new(Mode::Cenc { iv: Iv::Bytes16([0; 16]) }, KEY_ID, KEY);
//!
//! // Attach a pssh
//! enc.add_pssh(PsshInput {
//!     system_id: SPECIFIC_DRM_SYSTEM_ID,
//!     key_ids: vec![KEY_ID],
//!     data: b"<drm-specific-pssh-data>".to_vec(),
//! });
//!
//! // init segment
//! let init = std::fs::read("init.m4s")?;
//! let enc_init = enc.enc_init_segment_to_vec(&init)?;
//!
//! // media segments
//! let mut seg = std::fs::read("segment_1.m4s")?;
//! let enc_seg = enc.enc_media_segment_to_vec(&init, &mut seg)?;
//! # let _ = (enc_init, enc_seg);
//! # Ok(())
//! # }
//! ```
//!
//! # Example — cbcs
//!
//! The init/media calls are identical to cenc — only the [`Mode`] differs (cbcs
//! carries a constant IV in tenc instead of advancing a per-sample counter).
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mediumi_crypto::{Encrypter, Iv, Mode, PsshInput};
//! # const KEY_ID: [u8; 16] = [0x11; 16];
//! # const KEY: [u8; 16] = [0x33; 16];
//! # const SPECIFIC_DRM_SYSTEM_ID: [u8; 16] = [0xab; 16];
//! // cbcs carries a constant IV in tenc (shared by every sample).
//! const CONSTANT_IV: [u8; 16] = [0x00; 16];
//! let mut enc = Encrypter::new(Mode::Cbcs { iv: Iv::Bytes16(CONSTANT_IV) }, KEY_ID, KEY);
//!
//! // Attach a pssh
//! enc.add_pssh(PsshInput {
//!     system_id: SPECIFIC_DRM_SYSTEM_ID,
//!     key_ids: vec![KEY_ID],
//!     data: b"<drm-specific-pssh-data>".to_vec(),
//! });
//!
//! let init = std::fs::read("init.m4s")?;
//! let enc_init = enc.enc_init_segment_to_vec(&init)?;
//! let mut seg = std::fs::read("segment_1.m4s")?;
//! let enc_seg = enc.enc_media_segment_to_vec(&init, &mut seg)?;
//! # let _ = (enc_init, enc_seg);
//! # Ok(())
//! # }
//! ```

pub mod cbc;
pub mod cenc;
pub mod ctr;
pub mod encrypter;
pub mod error;
mod initial;
mod media;
pub mod pssh;
mod segment;
mod senc;
pub mod subsample;
mod track;
pub use cbc::{Aes128CbcPatternCipher, apply_cbcs_subsamples};
pub use cenc::{Subsample, apply_cenc_subsamples, derive_per_sample_iv};
pub use encrypter::{Encrypter, Iv, Mode};
pub use error::Error;
pub use pssh::PsshInput;
pub use segment::{MediaSegmentParts, split_media_segment};
pub use subsample::CodecKind;
