use crate::error::Error;
use crate::pssh::PsshInput;
use crate::{initial, media, segment};

/// Per-sample IV base (cenc) or constant IV (cbcs). 8 or 16 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Iv {
    Bytes8([u8; 8]),
    Bytes16([u8; 16]),
}

impl Iv {
    /// Raw IV bytes as stored in senc / tenc (8 or 16).
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Iv::Bytes8(b) => b,
            Iv::Bytes16(b) => b,
        }
    }

    /// IV size in bytes (8 or 16).
    pub fn size(&self) -> u8 {
        match self {
            Iv::Bytes8(_) => 8,
            Iv::Bytes16(_) => 16,
        }
    }

    /// Zero-extend to a 16-byte block: an 8-byte IV occupies the high 8 bytes,
    /// the low 8 bytes are zero. Used as the AES-CTR counter base and as the
    /// cbcs AES-CBC IV (which is always a 16-byte block).
    pub fn to_block16(&self) -> [u8; 16] {
        match self {
            Iv::Bytes8(b) => {
                let mut out = [0u8; 16];
                out[0..8].copy_from_slice(b);
                out
            }
            Iv::Bytes16(b) => *b,
        }
    }
}

/// Encryption scheme + its IV. cenc carries a per-sample IV base; cbcs carries
/// a constant IV. Bundling the IV with the mode makes invalid combinations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// AES-CTR, per-sample IV (8 or 16 bytes is the base; see `derive_per_sample_iv`).
    Cenc { iv: Iv },
    /// AES-CBC + pattern, constant IV (8 or 16 bytes; carried in tenc).
    Cbcs { iv: Iv },
}

impl Mode {
    /// The scheme's IV (per-sample base for cenc, constant for cbcs).
    pub fn iv(&self) -> &Iv {
        match self {
            Mode::Cenc { iv } | Mode::Cbcs { iv } => iv,
        }
    }

    /// scheme_type / aux_info_type 4cc.
    pub fn scheme_fourcc(&self) -> [u8; 4] {
        match self {
            Mode::Cenc { .. } => *b"cenc",
            Mode::Cbcs { .. } => *b"cbcs",
        }
    }
}

/// Encrypter
#[derive(Debug)]
pub struct Encrypter {
    pub mode: Mode,
    /// `tenc.default_KID`
    pub key_id: [u8; 16],
    /// AES-128 content key
    pub key: [u8; 16],
    /// Pssh inputs to attach to the moov during `enc_initial`.
    pub pssh_inputs: Vec<PsshInput>,
    /// Per-sample IV counter (8-byte cenc): added to the high 8 bytes of the
    /// IV per sample. Unused by cbcs and by 16-byte cenc.
    pub next_sample_index: u64,
    /// Cumulative cipher-block offset:
    /// the next sample's IV = base_iv (128-bit) + this. Advanced by the number
    /// of encrypted blocks per sample. Unused by 8-byte cenc and cbcs.
    pub next_block_offset: u128,
}

impl Encrypter {
    pub fn new(mode: Mode, key_id: [u8; 16], key: [u8; 16]) -> Self {
        Self {
            mode,
            key_id,
            key,
            pssh_inputs: Vec::new(),
            next_sample_index: 0,
            next_block_offset: 0,
        }
    }

    /// Append a DRM-system pssh entry
    pub fn add_pssh(&mut self, input: PsshInput) {
        self.pssh_inputs.push(input);
    }

    /// Encrypt an init segment
    /// need splitted moov box byte streams
    pub fn enc_init(&self, moov: &[u8]) -> Result<Vec<u8>, Error> {
        initial::enc_init(self, moov)
    }

    /// Encrypt an init segment
    /// need splitted moov, moof, mdat box byte streams
    pub fn enc_media(
        &mut self,
        moov: &[u8],
        moof: &[u8],
        mdat: &mut [u8],
    ) -> Result<Vec<u8>, Error> {
        media::enc_media(self, moov, moof, mdat)
    }

    /// Encrypt an init segment
    pub fn enc_init_segment<W: std::io::Write>(
        &self,
        init_bytes: &[u8],
        out: &mut W,
    ) -> Result<(), Error> {
        segment::enc_init_segment(self, init_bytes, out)
    }

    /// Encrypt an init segment (return Vec<u8>)
    pub fn enc_init_segment_to_vec(&self, init_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        self.enc_init_segment(init_bytes, &mut buf)?;
        Ok(buf)
    }

    /// Encrypt a segment
    pub fn enc_media_segment<W: std::io::Write>(
        &mut self,
        init_bytes: &[u8],
        media_bytes: &mut [u8],
        out: &mut W,
    ) -> Result<(), Error> {
        segment::enc_media_segment(self, init_bytes, media_bytes, out)
    }

    /// Encrypt a segment (return Vec<u8>)
    pub fn enc_media_segment_to_vec(
        &mut self,
        init_bytes: &[u8],
        media_bytes: &mut [u8],
    ) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::with_capacity(media_bytes.len());
        self.enc_media_segment(init_bytes, media_bytes, &mut buf)?;
        Ok(buf)
    }
}
