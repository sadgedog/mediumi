//! Serialize/Deserialize AC-3 frame

use crate::{
    ac3::{
        bap::{BapParams, compute_bap},
        error::Error,
        mantissa::{MantissaParams, parse_mantissas, write_mantissas},
    },
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

const AC3_SYNCWORD: u16 = 0x0B77;

// Frame size table
// Indexed by [frmsizecod][fscod], value is frame size in 16-bit words.
// fscod: 0=48kHz, 1=44.1kHz, 2=32kHz
const FRMSIZECOD_TABLE: [[u16; 3]; 38] = [
    [64, 69, 96],
    [64, 70, 96],
    [80, 87, 120],
    [80, 88, 120],
    [96, 104, 144],
    [96, 105, 144],
    [112, 121, 168],
    [112, 122, 168],
    [128, 139, 192],
    [128, 140, 192],
    [160, 174, 240],
    [160, 175, 240],
    [192, 208, 288],
    [192, 209, 288],
    [224, 243, 336],
    [224, 244, 336],
    [256, 278, 384],
    [256, 279, 384],
    [320, 348, 480],
    [320, 349, 480],
    [384, 417, 576],
    [384, 418, 576],
    [448, 487, 672],
    [448, 488, 672],
    [512, 557, 768],
    [512, 558, 768],
    [640, 696, 960],
    [640, 697, 960],
    [768, 835, 1152],
    [768, 836, 1152],
    [896, 975, 1344],
    [896, 976, 1344],
    [1024, 1114, 1536],
    [1024, 1115, 1536],
    [1152, 1253, 1728],
    [1152, 1254, 1728],
    [1280, 1393, 1920],
    [1280, 1394, 1920],
];

#[derive(Debug)]
pub struct SyncInfo {
    pub syncword: u16,
    pub crc1: u16,
    pub fscod: u8,
    pub frmsizecod: u8,
}

impl SyncInfo {
    /// Returns the frame size in bytes.
    pub fn frame_size(&self) -> Option<usize> {
        if self.fscod >= 3 || self.frmsizecod >= 38 {
            return None;
        }
        Some(FRMSIZECOD_TABLE[self.frmsizecod as usize][self.fscod as usize] as usize * 2)
    }
}

/// Peek at the syncinfo of an AC-3 frame and return the frame size in bytes.
/// Requires at least 5 bytes (syncword 2 + crc1 2 + fscod/frmsizecod 1).
pub fn peek_frame_size(data: &[u8]) -> Option<usize> {
    if data.len() < 5 {
        return None;
    }
    let syncword = (data[0] as u16) << 8 | data[1] as u16;
    if syncword != AC3_SYNCWORD {
        return None;
    }
    let fscod = data[4] >> 6;
    let frmsizecod = data[4] & 0x3F;
    if fscod >= 3 || frmsizecod >= 38 {
        return None;
    }
    Some(FRMSIZECOD_TABLE[frmsizecod as usize][fscod as usize] as usize * 2)
}

#[derive(Debug)]
pub enum AudioMode {
    /// 1+1 (Ch1, Ch2)
    DualMono {
        dialnorm2: u8,
        compr2: Option<u8>,
        langcod2: Option<u8>,
        audprodi2: Option<(u8, u8)>, // (mixlevel2, roomtyp2)
    },
    /// 1/0 (C)
    Center,
    /// 2/0 (L, R)
    Stereo { dsurmod: u8 },
    /// 3/0 (L, C, R)
    ThreeFront { cmixlev: u8 },
    /// 2/1 (L, R, S)
    StereoSurround { surmixlev: u8 },
    /// 3/1 (L, C, R, S)
    LcrSurround { cmixlev: u8, surmixlev: u8 },
    /// 2/2 (L, R, SL, SR), acmod=0b110
    Quad { surmixlev: u8 },
    /// 3/2 (L, C, R, SL, SR)
    FiveChannel { cmixlev: u8, surmixlev: u8 },
}

impl AudioMode {
    pub fn nfchans(&self) -> u8 {
        match self {
            AudioMode::DualMono { .. } => 2,
            AudioMode::Center => 1,
            AudioMode::Stereo { .. } => 2,
            AudioMode::ThreeFront { .. } => 3,
            AudioMode::StereoSurround { .. } => 3,
            AudioMode::LcrSurround { .. } => 4,
            AudioMode::Quad { .. } => 4,
            AudioMode::FiveChannel { .. } => 5,
        }
    }
}

/// Extended BSI fields after origbs, varying by bsid.
#[derive(Debug)]
pub enum BsiExtension {
    /// bsid=6: Alternate BSI syntax
    AltBsi {
        xbsi1: Option<Xbsi1>,
        xbsi2: Option<Xbsi2>,
    },
    /// bsid != 6: Standard BSI syntax
    Standard {
        timecod1: Option<u16>, // 14 bits
        timecod2: Option<u16>, // 14 bits
    },
}

#[derive(Debug)]
pub struct Xbsi1 {
    pub dmixmod: u8,
    pub ltrtcmixlev: u8,
    pub ltrtsurmixlev: u8,
    pub lorocmixlev: u8,
    pub lorosurmixlev: u8,
}

#[derive(Debug)]
pub struct Xbsi2 {
    pub dsurexmod: u8,
    pub dheadphonmod: u8,
    pub adconvtyp: bool,
    pub xbsi2: u8,
    pub encinfo: bool,
}

#[derive(Debug)]
pub struct Addbsi {
    pub addbsil: u8,
    pub addbsi: Vec<u8>,
}

#[derive(Debug)]
pub struct BitStreamInformation {
    pub bsid: u8,
    pub bsmod: u8,
    pub acmod: u8,
    pub audio_mode: AudioMode,
    pub lfeon: bool,
    pub dialnorm: u8,
    pub compr: Option<u8>,
    pub langcod: Option<u8>,
    pub audprodi: Option<(u8, u8)>, // (mixlevel, roomtyp)
    pub copyrightb: bool,
    pub origbs: bool,
    pub ext: BsiExtension,
    pub addbsi: Option<Addbsi>,
}

#[derive(Debug)]
pub enum CplStrategy {
    /// Using previous audio block coupling strategy (cplstre=0)
    Reuse,
    /// Not using coupling (cplstre=1, cplinu=0)
    NotInUse,
    /// Using coupling (cplstre=1, cplinu=1)
    InUse {
        chincpl: Vec<bool>,
        phsflginu: Option<bool>,
        cplbegf: u8,
        cplendf: u8,
        cplbndstrc: Vec<bool>,
    },
}

// ----------------------------------------------
// Coupling
#[derive(Debug)]
pub struct CplChannelCoord {
    pub mstrcplco: u8,
    pub bands: Vec<(u8, u8)>,
}

#[derive(Debug)]
pub struct CplCoord {
    pub channels: Vec<Option<CplChannelCoord>>,
    pub phsflg: Option<Vec<bool>>,
}

#[derive(Debug)]
pub struct Cpl {
    pub strategy: CplStrategy,
    pub coord: Option<CplCoord>,
}
// ----------------------------------------------

// ----------------------------------------------
// Rematrixing operation (2/0 mode only)
#[derive(Debug)]
pub struct Rematrixing {
    pub rematflg: Option<Vec<bool>>,
}
// ----------------------------------------------

// ----------------------------------------------
// Exponent strategy
#[derive(Debug)]
pub struct ExponentStrategy {
    pub cplexpstr: Option<u8>,
    pub chexpstr: Vec<u8>,
    pub lfeexpstr: Option<bool>,
    pub chbwcod: Vec<Option<u8>>,
}
// ----------------------------------------------

// ----------------------------------------------
// Coupling channel exponents
#[derive(Debug)]
pub struct CouplingChannelExponent {
    pub cplabsexp: u8,
    pub cplexps: Vec<u8>,
}
// ----------------------------------------------

// ----------------------------------------------
// Full bandwidth channels exponents
#[derive(Debug)]
pub struct FullBandwidthChannelExponent {
    pub abs_exp: u8,
    pub exps: Vec<u8>,
    pub gainrng: u8,
}

#[derive(Debug)]
pub struct FullBandwidthChannelExponents {
    pub channels: Vec<Option<FullBandwidthChannelExponent>>, // None = reuse
}
// ----------------------------------------------

// ----------------------------------------------
// Low frequency effects channel
#[derive(Debug)]
pub struct LowFrequencyEffectChannel {
    pub abs_exp: u8,
    pub lfeexps: Vec<u8>,
}
// ----------------------------------------------

// ----------------------------------------------
// Bit-allocation parametric information
#[derive(Debug, Clone)]
pub struct BitAllocParams {
    pub sdcycod: u8,
    pub fdcycod: u8,
    pub sgaincod: u8,
    pub dbpbcod: u8,
    pub floorcod: u8,
}

#[derive(Debug, Clone)]
pub struct SnrOffset {
    pub csnroffst: u8,
    pub cplfsnroffst: Option<u8>,
    pub cplfgaincod: Option<u8>,
    pub fsnroffst: Vec<u8>,
    pub fgaincod: Vec<u8>,
    pub lfefsnroffst: Option<u8>,
    pub lfefgaincod: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct CplLeak {
    pub cplfleak: u8,
    pub cplsleak: u8,
}

#[derive(Debug)]
pub struct BitAllocationParametricInformation {
    pub bai: Option<BitAllocParams>,
    pub snroffst: Option<SnrOffset>,
    pub cplleak: Option<CplLeak>,
}
// ----------------------------------------------

// ----------------------------------------------
// Delta bit allocation information
#[derive(Debug, Clone)]
pub struct DeltBaSegment {
    pub deltoffst: u8,
    pub deltlen: u8,
    pub deltba: u8,
}

#[derive(Debug)]
pub struct DeltaBitAllocationInformation {
    pub cpldeltbae: Option<u8>,
    pub deltbae: Vec<u8>,
    pub cpldeltsegs: Option<Vec<DeltBaSegment>>,
    pub deltsegs: Vec<Option<Vec<DeltBaSegment>>>,
}
// ----------------------------------------------

// ----------------------------------------------
// Inclusion of unused dummy data
#[derive(Debug)]
pub struct UnusedDummyData {
    pub skipl: u16,
    pub skipfld: Vec<u8>,
}
// ----------------------------------------------

// ----------------------------------------------
// Quantized mantissa values
#[derive(Debug)]
pub struct QuantizedMantissaValues {
    pub chmant: Vec<Vec<i32>>,     // [ch][bin], nfchans channels
    pub cplmant: Option<Vec<i32>>, // coupling channel mantissas
    pub lfemant: Option<Vec<i32>>, // LFE channel mantissas
    /// Pre-computed write actions for each bin position.
    /// Recorded at parse time to enable 1-pass write matching parse-side bit layout.
    pub write_actions: Vec<MantissaWriteAction>,
}

/// Write action for a single bin position in the mantissa bitstream.
#[derive(Debug, Clone)]
pub enum MantissaWriteAction {
    /// No bits to write (bap=0)
    None,
    /// Write a group code at this position (first bin of a grouped quantizer)
    WriteGroup { code: u32, bits: u8 },
    /// Skip (non-first member of a group, already written at group's first bin)
    Skip,
    /// Write a single mantissa value
    WriteSingle { code: u32, bits: u8 },
}
// ----------------------------------------------

#[derive(Debug)]
pub struct AudioBlock {
    pub blksw: Vec<bool>,
    pub dithflag: Vec<bool>,
    pub dynrng: Option<u8>,
    pub dynrng2: Option<u8>,
    pub cpl: Cpl,
    pub rematrixing: Rematrixing,
    pub exponent_strategy: ExponentStrategy,
    pub cpl_ch_exps: Option<CouplingChannelExponent>,
    pub fb_ch_exps: FullBandwidthChannelExponents,
    pub lfe_ch_exps: Option<Option<LowFrequencyEffectChannel>>,
    pub bapi: BitAllocationParametricInformation,
    pub deltbai: Option<DeltaBitAllocationInformation>,
    pub unused_dummy: Option<UnusedDummyData>,
    pub mantissas: QuantizedMantissaValues,
}

/// Effective delta BA state: resolved per-channel and coupling delta segments.
/// Unlike `DeltaBitAllocationInformation` (which stores raw parsed data including
/// reuse/none codes), this stores the final resolved segments for BAP computation.
pub struct EffectiveDeltBa {
    pub cpldeltsegs: Option<Vec<DeltBaSegment>>,
    pub deltsegs: Vec<Option<Vec<DeltBaSegment>>>,
}

/// Decoded state carried between audio blocks for cross-block reuse.
pub struct AudblkDecodedState {
    pub cplinu: bool,
    pub chincpl: Vec<bool>,
    pub phsflginu: bool,
    pub ncplbnd: usize,
    pub cplbegf: Option<u8>,
    pub cplendf: Option<u8>,
    pub bai: Option<BitAllocParams>,
    pub snroffst: Option<SnrOffset>,
    pub cplleak: Option<CplLeak>,
    pub eff_deltba: Option<EffectiveDeltBa>,
    pub ch_decoded_exps: Vec<[u8; 256]>,
    pub cpl_decoded_exps: Option<[u8; 256]>,
    pub lfe_decoded_exps: Option<[u8; 256]>,
    pub chbwcod: Vec<Option<u8>>,
}

impl AudblkDecodedState {
    fn initial(nfchans: u8) -> Self {
        Self {
            cplinu: false,
            chincpl: vec![false; nfchans as usize],
            phsflginu: false,
            ncplbnd: 0,
            cplbegf: None,
            cplendf: None,
            bai: None,
            snroffst: None,
            cplleak: None,
            eff_deltba: None,
            ch_decoded_exps: vec![[0u8; 256]; nfchans as usize],
            cpl_decoded_exps: None,
            lfe_decoded_exps: None,
            chbwcod: vec![None; nfchans as usize],
        }
    }
}

#[derive(Debug)]
pub struct AuxiliaryData {
    pub auxdata: Vec<u8>,
    /// Number of valid bits in the last byte (1-8, or 8 if all full bytes).
    pub last_byte_bits: u8,
}

#[derive(Debug)]
pub struct ErrorCheck {
    pub crcrsv: bool,
    pub crc2: u16,
}

#[derive(Debug)]
pub struct Ac3 {
    pub si: SyncInfo,
    pub bsi: BitStreamInformation,
    pub ab: Vec<AudioBlock>,
    pub aux: AuxiliaryData,
    pub ec: ErrorCheck,
}

impl Ac3 {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut writer = BitstreamWriter::new();

        // syncinfo
        let si = &self.si;
        writer.write_bits(si.syncword as u32, 16);
        writer.write_bits(si.crc1 as u32, 16);
        writer.write_bits(si.fscod as u32, 2);
        writer.write_bits(si.frmsizecod as u32, 6);

        // bit stream information
        let bsi = &self.bsi;
        writer.write_bits(bsi.bsid as u32, 5);
        writer.write_bits(bsi.bsmod as u32, 3);
        writer.write_bits(bsi.acmod as u32, 3);

        if (bsi.acmod & 0x1) != 0 && bsi.acmod != 0x1 {
            match &bsi.audio_mode {
                AudioMode::ThreeFront { cmixlev, .. }
                | AudioMode::LcrSurround { cmixlev, .. }
                | AudioMode::FiveChannel { cmixlev, .. } => {
                    writer.write_bits(*cmixlev as u32, 2);
                }
                _ => {}
            }
        }
        if (bsi.acmod & 0x4) != 0 {
            match &bsi.audio_mode {
                AudioMode::StereoSurround { surmixlev, .. }
                | AudioMode::LcrSurround { surmixlev, .. }
                | AudioMode::Quad { surmixlev, .. }
                | AudioMode::FiveChannel { surmixlev, .. } => {
                    writer.write_bits(*surmixlev as u32, 2);
                }
                _ => {}
            }
        }
        if let AudioMode::Stereo { dsurmod } = &bsi.audio_mode {
            writer.write_bits(*dsurmod as u32, 2);
        }

        writer.write_bool(bsi.lfeon);
        writer.write_bits(bsi.dialnorm as u32, 5);
        writer.write_bool(bsi.compr.is_some());
        if let Some(compr) = bsi.compr {
            writer.write_bits(compr as u32, 8);
        }
        writer.write_bool(bsi.langcod.is_some());
        if let Some(langcod) = bsi.langcod {
            writer.write_bits(langcod as u32, 8);
        }
        writer.write_bool(bsi.audprodi.is_some());
        if let Some((mixlevel, roomtyp)) = bsi.audprodi {
            writer.write_bits(mixlevel as u32, 5);
            writer.write_bits(roomtyp as u32, 2);
        }

        if let AudioMode::DualMono {
            dialnorm2,
            compr2,
            langcod2,
            audprodi2,
        } = &bsi.audio_mode
        {
            writer.write_bits(*dialnorm2 as u32, 5);
            writer.write_bool(compr2.is_some());
            if let Some(c) = compr2 {
                writer.write_bits(*c as u32, 8);
            }
            writer.write_bool(langcod2.is_some());
            if let Some(l) = langcod2 {
                writer.write_bits(*l as u32, 8);
            }
            writer.write_bool(audprodi2.is_some());
            if let Some((m, r)) = audprodi2 {
                writer.write_bits(*m as u32, 5);
                writer.write_bits(*r as u32, 2);
            }
        }

        writer.write_bool(bsi.copyrightb);
        writer.write_bool(bsi.origbs);
        match &bsi.ext {
            BsiExtension::AltBsi { xbsi1, xbsi2 } => {
                writer.write_bool(xbsi1.is_some());
                if let Some(x) = xbsi1 {
                    writer.write_bits(x.dmixmod as u32, 2);
                    writer.write_bits(x.ltrtcmixlev as u32, 3);
                    writer.write_bits(x.ltrtsurmixlev as u32, 3);
                    writer.write_bits(x.lorocmixlev as u32, 3);
                    writer.write_bits(x.lorosurmixlev as u32, 3);
                }
                writer.write_bool(xbsi2.is_some());
                if let Some(x) = xbsi2 {
                    writer.write_bits(x.dsurexmod as u32, 2);
                    writer.write_bits(x.dheadphonmod as u32, 2);
                    writer.write_bool(x.adconvtyp);
                    writer.write_bits(x.xbsi2 as u32, 8);
                    writer.write_bool(x.encinfo);
                }
            }
            BsiExtension::Standard { timecod1, timecod2 } => {
                writer.write_bool(timecod1.is_some());
                if let Some(t) = timecod1 {
                    writer.write_bits(*t as u32, 14);
                }
                writer.write_bool(timecod2.is_some());
                if let Some(t) = timecod2 {
                    writer.write_bits(*t as u32, 14);
                }
            }
        }
        writer.write_bool(bsi.addbsi.is_some());
        if let Some(addbsi) = &bsi.addbsi {
            writer.write_bits(addbsi.addbsil as u32, 6);
            for &b in &addbsi.addbsi {
                writer.write_bits(b as u32, 8);
            }
        }

        // audio blocks
        let nfchans = bsi.audio_mode.nfchans();
        let mut prev = AudblkDecodedState::initial(nfchans);

        for ab in &self.ab {
            Self::write_audblk(&mut writer, nfchans, bsi.acmod, bsi.lfeon, ab, &mut prev)?;
        }

        // auxdata
        let aux = &self.aux;
        if !aux.auxdata.is_empty() {
            let full_bytes = if aux.last_byte_bits < 8 && aux.last_byte_bits > 0 {
                aux.auxdata.len() - 1
            } else {
                aux.auxdata.len()
            };
            for &b in &aux.auxdata[..full_bytes] {
                writer.write_bits(b as u32, 8);
            }
            if aux.last_byte_bits > 0 && aux.last_byte_bits < 8 {
                let last = *aux
                    .auxdata
                    .last()
                    .ok_or(Error::InvalidState("auxdata empty but last_byte_bits > 0"))?;
                writer.write_bits(last as u32, aux.last_byte_bits);
            }
        }

        // errorcheck
        writer.write_bool(self.ec.crcrsv);
        writer.write_bits(self.ec.crc2 as u32, 16);

        Ok(writer.finish())
    }

    // Parse an AC-3 frame
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);

        // syncinfo
        let si = Self::parse_si(&mut reader)?;

        // bsi
        let bsi = Self::parse_bsi(&mut reader)?;

        let nfchans = bsi.audio_mode.nfchans();

        // audblk ×6
        let mut audblks = Vec::with_capacity(6);
        let mut prev = AudblkDecodedState::initial(nfchans);

        for blk_idx in 0..6 {
            // Skip data (unused dummy) in a previous block may have consumed
            // all remaining bits in the frame.
            if reader.remaining_bits() < 5 {
                break;
            }
            let result =
                Self::parse_audblk(&mut reader, nfchans, bsi.acmod, bsi.lfeon, si.fscod, &prev);
            let (blk, state) = match result {
                Ok(v) => v,
                Err(_) if blk_idx > 0 => {
                    // AC-3 encoders may pack meaningful audio data in early blocks
                    // and fill remaining blocks with padding/garbage.
                    // this with error concealment. We stop parsing blocks here.
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            };
            audblks.push(blk);
            prev = state;
        }

        // Parse auxdata and errorcheck if enough bits remain
        let aux = if reader.remaining_bits() >= 17 {
            Self::parse_auxdata(&mut reader)?
        } else {
            AuxiliaryData {
                auxdata: vec![],
                last_byte_bits: 0,
            }
        };
        let ec = if reader.remaining_bits() >= 17 {
            Self::parse_errorcheck(&mut reader)?
        } else {
            ErrorCheck {
                crcrsv: false,
                crc2: 0,
            }
        };

        Ok(Ac3 {
            si,
            bsi,
            ab: audblks,
            aux,
            ec,
        })
    }

    /// Parse a sync information
    fn parse_si(reader: &mut BitstreamReader<'_>) -> Result<SyncInfo, Error> {
        Ok(SyncInfo {
            syncword: reader.read_bits(16)? as u16,
            crc1: reader.read_bits(16)? as u16,
            fscod: reader.read_bits(2)? as u8,
            frmsizecod: reader.read_bits(6)? as u8,
        })
    }

    /// Parse a bit stream information
    fn parse_bsi(reader: &mut BitstreamReader<'_>) -> Result<BitStreamInformation, Error> {
        let bsid = reader.read_bits(5)? as u8;
        let bsmod = reader.read_bits(3)? as u8;
        let acmod = reader.read_bits(3)? as u8;

        let cmixlev = if (acmod & 0x1) != 0 && acmod != 0x1 {
            Some(reader.read_bits(2)? as u8)
        } else {
            None
        };
        let surmixlev = if (acmod & 0x4) != 0 {
            Some(reader.read_bits(2)? as u8)
        } else {
            None
        };
        let dsurmod = if acmod == 0x2 {
            Some(reader.read_bits(2)? as u8)
        } else {
            None
        };

        let lfeon = reader.read_bit()?;
        let dialnorm = reader.read_bits(5)? as u8;
        let compre = reader.read_bit()?;
        let compr = if compre {
            Some(reader.read_bits(8)? as u8)
        } else {
            None
        };
        let langcode = reader.read_bit()?;
        let langcod = if langcode {
            Some(reader.read_bits(8)? as u8)
        } else {
            None
        };
        let audprodie = reader.read_bit()?;
        let audprodi = if audprodie {
            Some((reader.read_bits(5)? as u8, reader.read_bits(2)? as u8))
        } else {
            None
        };

        let audio_mode = match acmod {
            0b000 => {
                let dialnorm2 = reader.read_bits(5)? as u8;
                let compr2e = reader.read_bit()?;
                let compr2 = if compr2e {
                    Some(reader.read_bits(8)? as u8)
                } else {
                    None
                };
                let langcod2e = reader.read_bit()?;
                let langcod2 = if langcod2e {
                    Some(reader.read_bits(8)? as u8)
                } else {
                    None
                };
                let audprodi2e = reader.read_bit()?;
                let audprodi2 = if audprodi2e {
                    Some((reader.read_bits(5)? as u8, reader.read_bits(2)? as u8))
                } else {
                    None
                };
                AudioMode::DualMono {
                    dialnorm2,
                    compr2,
                    langcod2,
                    audprodi2,
                }
            }
            0b001 => AudioMode::Center,
            0b010 => AudioMode::Stereo {
                dsurmod: dsurmod.ok_or(Error::InvalidAcmod(acmod))?,
            },
            0b011 => AudioMode::ThreeFront {
                cmixlev: cmixlev.ok_or(Error::InvalidAcmod(acmod))?,
            },
            0b100 => AudioMode::StereoSurround {
                surmixlev: surmixlev.ok_or(Error::InvalidAcmod(acmod))?,
            },
            0b101 => AudioMode::LcrSurround {
                cmixlev: cmixlev.ok_or(Error::InvalidAcmod(acmod))?,
                surmixlev: surmixlev.ok_or(Error::InvalidAcmod(acmod))?,
            },
            0b110 => AudioMode::Quad {
                surmixlev: surmixlev.ok_or(Error::InvalidAcmod(acmod))?,
            },
            0b111 => AudioMode::FiveChannel {
                cmixlev: cmixlev.ok_or(Error::InvalidAcmod(acmod))?,
                surmixlev: surmixlev.ok_or(Error::InvalidAcmod(acmod))?,
            },
            _ => return Err(Error::InvalidAcmod(acmod)),
        };

        let copyrightb = reader.read_bit()?;
        let origbs = reader.read_bit()?;

        let ext = if bsid == 6 {
            // Alternate BSI syntax
            let xbsi1e = reader.read_bit()?;
            let xbsi1 = if xbsi1e {
                Some(Xbsi1 {
                    dmixmod: reader.read_bits(2)? as u8,
                    ltrtcmixlev: reader.read_bits(3)? as u8,
                    ltrtsurmixlev: reader.read_bits(3)? as u8,
                    lorocmixlev: reader.read_bits(3)? as u8,
                    lorosurmixlev: reader.read_bits(3)? as u8,
                })
            } else {
                None
            };
            let xbsi2e = reader.read_bit()?;
            let xbsi2 = if xbsi2e {
                Some(Xbsi2 {
                    dsurexmod: reader.read_bits(2)? as u8,
                    dheadphonmod: reader.read_bits(2)? as u8,
                    adconvtyp: reader.read_bit()?,
                    xbsi2: reader.read_bits(8)? as u8,
                    encinfo: reader.read_bit()?,
                })
            } else {
                None
            };
            BsiExtension::AltBsi { xbsi1, xbsi2 }
        } else {
            // Standard BSI syntax
            let timecod1e = reader.read_bit()?;
            let timecod1 = if timecod1e {
                Some(reader.read_bits(14)? as u16)
            } else {
                None
            };
            let timecod2e = reader.read_bit()?;
            let timecod2 = if timecod2e {
                Some(reader.read_bits(14)? as u16)
            } else {
                None
            };
            BsiExtension::Standard { timecod1, timecod2 }
        };

        let addbsie = reader.read_bit()?;
        let addbsi = if addbsie {
            let addbsil = reader.read_bits(6)? as u8;
            let len = addbsil as usize + 1;
            let mut data = Vec::with_capacity(len);
            for _ in 0..len {
                data.push(reader.read_bits(8)? as u8);
            }
            Some(Addbsi {
                addbsil,
                addbsi: data,
            })
        } else {
            None
        };

        Ok(BitStreamInformation {
            bsid,
            bsmod,
            acmod,
            audio_mode,
            lfeon,
            dialnorm,
            compr,
            langcod,
            audprodi,
            copyrightb,
            origbs,
            ext,
            addbsi,
        })
    }

    /// Parse an audio block
    fn parse_audblk(
        reader: &mut BitstreamReader<'_>,
        nfchans: u8,
        acmod: u8,
        lfeon: bool,
        fscod: u8,
        prev: &AudblkDecodedState,
    ) -> Result<(AudioBlock, AudblkDecodedState), Error> {
        // blksw[ch] - 1 bit each
        let mut blksw = Vec::with_capacity(nfchans as usize);
        for _ in 0..nfchans {
            blksw.push(reader.read_bit()?);
        }
        // dithflag[ch] - 1 bit each
        let mut dithflag = Vec::with_capacity(nfchans as usize);
        for _ in 0..nfchans {
            dithflag.push(reader.read_bit()?);
        }

        let dynrnge = reader.read_bit()?;
        let dynrng = if dynrnge {
            Some(reader.read_bits(8)? as u8)
        } else {
            None
        };

        let dynrng2 = if acmod == 0 {
            let dynrng2e = reader.read_bit()?;
            if dynrng2e {
                Some(reader.read_bits(8)? as u8)
            } else {
                None
            }
        } else {
            None
        };

        // ------------------------------------------------------------------------------------------
        // coupling strategy
        let cplstre = reader.read_bit()?;
        let mut cplinu = prev.cplinu;
        let mut chincpl: Vec<bool> = prev.chincpl.to_vec();
        let mut phsflginu = prev.phsflginu;
        let mut ncplbnd = prev.ncplbnd;

        let strategy = if cplstre {
            cplinu = reader.read_bit()?;
            if cplinu {
                chincpl = Vec::with_capacity(nfchans as usize);
                for _ in 0..nfchans {
                    chincpl.push(reader.read_bit()?);
                }
                phsflginu = if acmod == 0x2 {
                    reader.read_bit()?
                } else {
                    false
                };
                let cplbegf = reader.read_bits(4)? as u8;
                let cplendf = reader.read_bits(4)? as u8;
                let ncplsubnd = (3 + cplendf)
                    .checked_sub(cplbegf)
                    .ok_or(Error::InvalidState(
                        "invalid coupling range: ncplsubnd would be negative",
                    ))? as usize;
                let mut cplbndstrc = Vec::with_capacity(ncplsubnd.saturating_sub(1));
                for _ in 1..ncplsubnd {
                    cplbndstrc.push(reader.read_bit()?);
                }
                // ncplbnd: number of coupling bands
                // ncplbnd = (ncplsubnd – (cplbndstrc[1] + ... + cplbndstrc[ncplsubnd – 1]))
                //         = ncplsubnd - {number of true in cplbndstrc}
                //         = 1 + cplbndstrc.len() - {number of true in cplbndstrc}
                //         = 1 + {number of false in cplbndstrc}
                ncplbnd = 1;
                for &strc in &cplbndstrc {
                    if !strc {
                        ncplbnd += 1;
                    }
                }
                CplStrategy::InUse {
                    chincpl: chincpl.clone(),
                    phsflginu: if acmod == 0x2 { Some(phsflginu) } else { None },
                    cplbegf,
                    cplendf,
                    cplbndstrc,
                }
            } else {
                // reset coupling
                chincpl = vec![false; nfchans as usize];
                phsflginu = false;
                ncplbnd = 0;
                CplStrategy::NotInUse
            }
        } else {
            CplStrategy::Reuse
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // coupling coordinates
        let coord = if cplinu {
            let mut channels = Vec::with_capacity(nfchans as usize);
            let mut cplcoe = Vec::with_capacity(nfchans as usize);
            for (_ch, &in_cpl) in chincpl.iter().enumerate().take(nfchans as usize) {
                if in_cpl {
                    let has_coe = reader.read_bit()?;
                    cplcoe.push(has_coe);
                    if has_coe {
                        let mstrcplco = reader.read_bits(2)? as u8;
                        let mut bands = Vec::with_capacity(ncplbnd);
                        for _ in 0..ncplbnd {
                            let cplcoexp = reader.read_bits(4)? as u8;
                            let cplcomant = reader.read_bits(4)? as u8;
                            bands.push((cplcoexp, cplcomant));
                        }
                        channels.push(Some(CplChannelCoord { mstrcplco, bands }));
                    } else {
                        channels.push(None);
                    }
                } else {
                    cplcoe.push(false);
                    channels.push(None);
                }
            }

            let phsflg = if acmod == 0x2
                && phsflginu
                && (cplcoe.first() == Some(&true) || cplcoe.get(1) == Some(&true))
            {
                let mut flags = Vec::with_capacity(ncplbnd);
                for _ in 0..ncplbnd {
                    flags.push(reader.read_bit()?);
                }
                Some(flags)
            } else {
                None
            };

            Some(CplCoord { channels, phsflg })
        } else {
            None
        };

        let cpl = Cpl { strategy, coord };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // rematrixing (2/0 mode only)
        let rematflg = if acmod == 0x2 {
            let rematstr = reader.read_bit()?;
            if rematstr {
                let rematcplbegf = match &cpl.strategy {
                    CplStrategy::InUse { cplbegf, .. } => Some(*cplbegf),
                    _ => prev.cplbegf,
                };
                let nrematbnds = if !cplinu || rematcplbegf.is_none_or(|f| f > 2) {
                    4
                } else if rematcplbegf.is_some_and(|f| f > 0) {
                    3
                } else {
                    2
                };
                let mut flags = Vec::with_capacity(nrematbnds);
                for _ in 0..nrematbnds {
                    flags.push(reader.read_bit()?);
                }
                Some(flags)
            } else {
                None // reuse previous
            }
        } else {
            None
        };
        let rematrixing = Rematrixing { rematflg };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // exponent strategies
        let cplexpstr = if cplinu {
            Some(reader.read_bits(2)? as u8)
        } else {
            None
        };
        let mut chexpstr = Vec::with_capacity(nfchans as usize);
        for _ in 0..nfchans {
            chexpstr.push(reader.read_bits(2)? as u8);
        }
        let lfeexpstr = if lfeon {
            Some(reader.read_bit()?)
        } else {
            None
        };

        // channel bandwidth code
        let mut chbwcod = Vec::with_capacity(nfchans as usize);
        for ch in 0..nfchans as usize {
            if chexpstr[ch] != 0 {
                // not reuse
                if !chincpl[ch] {
                    chbwcod.push(Some(reader.read_bits(6)? as u8));
                } else {
                    chbwcod.push(None);
                }
            } else {
                // reuse
                chbwcod.push(prev.chbwcod.get(ch).copied().flatten());
            }
        }

        let exponent_strategy = ExponentStrategy {
            cplexpstr,
            chexpstr: chexpstr.clone(),
            lfeexpstr,
            chbwcod: chbwcod.clone(),
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // coupling channel exponents
        let cplbegf = match &cpl.strategy {
            CplStrategy::InUse { cplbegf, .. } => Some(*cplbegf),
            _ => prev.cplbegf,
        };
        let cplendf = match &cpl.strategy {
            CplStrategy::InUse { cplendf, .. } => Some(*cplendf),
            _ => prev.cplendf,
        };

        let cpl_ch_exps = if cplinu {
            let cplexpstr_val =
                cplexpstr.ok_or(Error::InvalidState("cplexpstr missing while cplinu"))?;
            if cplexpstr_val != 0 {
                // new coupling exponents
                let cplabsexp = reader.read_bits(4)? as u8;
                let ncplsubnd = (3 + cplendf.ok_or(Error::InvalidState("cplendf missing"))?
                    - cplbegf.ok_or(Error::InvalidState("cplbegf missing"))?)
                    as usize;
                // ncplgrps = (cplendmant – cplstrtmant) / 3;   // for D15 mode
                //          = (cplendmant – cplstrtmant) / 6;   // for D25 mode
                //          = (cplendmant – cplstrtmant) / 12;  // for D45 mode
                // cplstrtmant = (cplbegf * 12) + 37
                // cplendmant = ((cplendf + 3) * 12) + 37
                // ncplsubnd = 3 + cplendf - cplbegf
                // cplendmant - cplstrtmant = 12 * ncplsubnd
                let ncplgrps = match cplexpstr_val {
                    1 => ncplsubnd * 4, // D15
                    2 => ncplsubnd * 2, // D25
                    3 => ncplsubnd,     // D45
                    _ => return Err(Error::InvalidState("invalid cplexpstr")),
                };
                let mut cplexps = Vec::with_capacity(ncplgrps);
                for _ in 0..ncplgrps {
                    cplexps.push(reader.read_bits(7)? as u8);
                }
                Some(CouplingChannelExponent { cplabsexp, cplexps })
            } else {
                None // reuse previous block's coupling exponents
            }
        } else {
            None // coupling not in use
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // full bandwidth channel exponents
        let mut fb_channels: Vec<Option<FullBandwidthChannelExponent>> =
            Vec::with_capacity(nfchans as usize);
        for ch in 0..nfchans as usize {
            // not reuse
            if chexpstr[ch] != 0 {
                let abs_exp = reader.read_bits(4)? as u8;
                // endmant:   number of mantissa coefficients
                // coupled:   cplstrtmant = (cplbegf * 12) + 37
                // uncoupled: endmant[ch] = chbwcod[ch] * 3 + 73
                let endmant = if chincpl[ch] {
                    37 + 12 * cplbegf.ok_or(Error::InvalidState("cplbegf missing"))? as usize
                } else {
                    73 + 3 * chbwcod[ch].ok_or(Error::InvalidState("chbwcod missing"))? as usize
                };
                let nchgrps = match chexpstr[ch] {
                    1 => (endmant - 1) / 3,      // D15: (endmant-1+3-3)/3
                    2 => (endmant - 1 + 3) / 6,  // D25: (endmant-1+6-3)/6
                    3 => (endmant - 1 + 9) / 12, // D45: (endmant-1+12-3)/12
                    _ => unreachable!(),
                };
                let mut exps = Vec::with_capacity(nchgrps);
                for _ in 0..nchgrps {
                    exps.push(reader.read_bits(7)? as u8);
                }
                let gainrng = reader.read_bits(2)? as u8;
                fb_channels.push(Some(FullBandwidthChannelExponent {
                    abs_exp,
                    exps,
                    gainrng,
                }));
            } else {
                fb_channels.push(None); // reuse
            }
        }
        let fb_ch_exps = FullBandwidthChannelExponents {
            channels: fb_channels,
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // Low frequency effects exponents
        let lfe_ch_exps: Option<Option<LowFrequencyEffectChannel>> = if lfeon {
            if lfeexpstr.is_some_and(|s| s) {
                let abs_exp = reader.read_bits(4)? as u8;
                let nlfegrps: usize = 2;
                let mut lfeexps = Vec::with_capacity(nlfegrps);
                for _ in 0..nlfegrps {
                    lfeexps.push(reader.read_bits(7)? as u8);
                }
                Some(Some(LowFrequencyEffectChannel { abs_exp, lfeexps }))
            } else {
                Some(None) // reuse
            }
        } else {
            None // no LFE channel
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // Bit-allocation parametric information
        let bai = if reader.read_bit()? {
            Some(BitAllocParams {
                sdcycod: reader.read_bits(2)? as u8,
                fdcycod: reader.read_bits(2)? as u8,
                sgaincod: reader.read_bits(2)? as u8,
                dbpbcod: reader.read_bits(2)? as u8,
                floorcod: reader.read_bits(3)? as u8,
            })
        } else {
            None
        };

        // SNR offset
        let snroffst = if reader.read_bit()? {
            let csnroffst = reader.read_bits(6)? as u8;
            let (cplfsnroffst, cplfgaincod) = if cplinu {
                (
                    Some(reader.read_bits(4)? as u8),
                    Some(reader.read_bits(3)? as u8),
                )
            } else {
                (None, None)
            };
            let mut fsnroffst = Vec::with_capacity(nfchans as usize);
            let mut fgaincod = Vec::with_capacity(nfchans as usize);
            for _ in 0..nfchans {
                fsnroffst.push(reader.read_bits(4)? as u8);
                fgaincod.push(reader.read_bits(3)? as u8);
            }
            let (lfefsnroffst, lfefgaincod) = if lfeon {
                (
                    Some(reader.read_bits(4)? as u8),
                    Some(reader.read_bits(3)? as u8),
                )
            } else {
                (None, None)
            };
            Some(SnrOffset {
                csnroffst,
                cplfsnroffst,
                cplfgaincod,
                fsnroffst,
                fgaincod,
                lfefsnroffst,
                lfefgaincod,
            })
        } else {
            None
        };

        // Coupling leak
        let cplleak = if cplinu {
            if reader.read_bit()? {
                Some(CplLeak {
                    cplfleak: reader.read_bits(3)? as u8,
                    cplsleak: reader.read_bits(3)? as u8,
                })
            } else {
                None
            }
        } else {
            None
        };

        let bit_allocation_parametric_information = BitAllocationParametricInformation {
            bai: bai.clone(),
            snroffst: snroffst.clone(),
            cplleak: cplleak.clone(),
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // Delta bit allocation information
        let deltbai = if reader.read_bit()? {
            let cpldeltbae = if cplinu {
                Some(reader.read_bits(2)? as u8)
            } else {
                None
            };
            let mut deltbae = Vec::with_capacity(nfchans as usize);
            for _ in 0..nfchans {
                deltbae.push(reader.read_bits(2)? as u8);
            }

            // coupling delta segments (cpldeltbae == 1 means "new info follows")
            let cpldeltsegs = if cplinu && cpldeltbae == Some(1) {
                let cpldeltnseg = reader.read_bits(3)? as usize;
                let mut segs = Vec::with_capacity(cpldeltnseg + 1);
                for _ in 0..=cpldeltnseg {
                    segs.push(DeltBaSegment {
                        deltoffst: reader.read_bits(5)? as u8,
                        deltlen: reader.read_bits(4)? as u8,
                        deltba: reader.read_bits(3)? as u8,
                    });
                }
                Some(segs)
            } else {
                None
            };

            // per-channel delta segments (deltbae[ch] == 1: "new info follows")
            let mut deltsegs = Vec::with_capacity(nfchans as usize);
            for (_ch, &mode) in deltbae.iter().enumerate().take(nfchans as usize) {
                if mode == 1 {
                    let deltnseg = reader.read_bits(3)? as usize;
                    let mut segs = Vec::with_capacity(deltnseg + 1);
                    for _ in 0..=deltnseg {
                        segs.push(DeltBaSegment {
                            deltoffst: reader.read_bits(5)? as u8,
                            deltlen: reader.read_bits(4)? as u8,
                            deltba: reader.read_bits(3)? as u8,
                        });
                    }
                    deltsegs.push(Some(segs));
                } else {
                    deltsegs.push(None);
                }
            }

            Some(DeltaBitAllocationInformation {
                cpldeltbae,
                deltbae,
                cpldeltsegs,
                deltsegs,
            })
        } else {
            None
        };

        // Resolve effective delta BA: handle reuse (0), new (1), none (2)
        let eff_deltba = if let Some(ref dba) = deltbai {
            // deltbaie==1: resolve each channel's deltbae
            // cpldeltbae: 0=reuse prev, 1=new (already in cpldeltsegs), 2=none
            let eff_cpldeltsegs = match dba.cpldeltbae {
                Some(0) => prev.eff_deltba.as_ref().and_then(|p| p.cpldeltsegs.clone()),
                Some(1) => dba.cpldeltsegs.clone(),
                _ => None, // 2=none, 3=reserved, or not coupling
            };
            // deltbae[ch]: 0=reuse prev, 1=new (already in deltsegs), 2=none
            let mut eff_deltsegs = Vec::with_capacity(nfchans as usize);
            for ch in 0..nfchans as usize {
                let seg = match dba.deltbae[ch] {
                    0 => prev
                        .eff_deltba
                        .as_ref()
                        .and_then(|p| p.deltsegs.get(ch).cloned())
                        .flatten(),
                    1 => dba.deltsegs[ch].clone(),
                    _ => None, // 2=none, 3=reserved
                };
                eff_deltsegs.push(seg);
            }
            Some(EffectiveDeltBa {
                cpldeltsegs: eff_cpldeltsegs,
                deltsegs: eff_deltsegs,
            })
        } else {
            // deltbaie==0: reuse entire previous effective delta BA
            prev.eff_deltba.as_ref().map(|p| EffectiveDeltBa {
                cpldeltsegs: p.cpldeltsegs.clone(),
                deltsegs: p.deltsegs.clone(),
            })
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // Inclusion of unused dummy data
        let unused_dummy = if reader.read_bit()? {
            let skipl = reader.read_bits(9)? as u16;
            let skip_bytes = skipl as usize;
            let mut skipfld = Vec::with_capacity(skip_bytes);
            for _ in 0..skip_bytes {
                if reader.remaining_bits() >= 8 {
                    skipfld.push(reader.read_bits(8)? as u8);
                } else {
                    reader.skip_bits(reader.remaining_bits());
                    break;
                }
            }
            Some(UnusedDummyData { skipl, skipfld })
        } else {
            None
        };
        // ------------------------------------------------------------------------------------------

        // ------------------------------------------------------------------------------------------
        // Quantized mantissa values

        // nchmant[ch] = endmant[ch]
        let mut nchmant = Vec::with_capacity(nfchans as usize);
        for ch in 0..nfchans as usize {
            let endmant = if chincpl[ch] {
                37 + 12 * cplbegf.ok_or(Error::InvalidState("cplbegf missing"))? as usize
            } else {
                73 + 3 * chbwcod[ch].ok_or(Error::InvalidState("chbwcod missing"))? as usize
            };
            nchmant.push(endmant);
        }

        // ncplmant = 12 * ncplsubnd
        //          = 12 * (3 + cplendf - cplbegf)
        let ncplmant = if cplinu {
            let begf = cplbegf.ok_or(Error::InvalidState("cplbegf missing"))? as usize;
            let endf = cplendf.ok_or(Error::InvalidState("cplendf missing"))? as usize;
            12 * (3 + endf - begf)
        } else {
            0
        };

        // Decode exponents: grouped -> per-bin
        // Full bandwidth channels
        let mut ch_decoded_exps = prev.ch_decoded_exps.clone();
        for ch in 0..nfchans as usize {
            if chexpstr[ch] != 0 {
                let fb = fb_ch_exps.channels[ch]
                    .as_ref()
                    .ok_or(Error::InvalidState("fb exponent missing"))?;
                ch_decoded_exps[ch] =
                    Self::decode_exponents(fb.abs_exp, &fb.exps, chexpstr[ch], nchmant[ch]);
            }
        }

        // Coupling channel
        let cpl_decoded_exps = if cplinu {
            let cplexpstr_val =
                cplexpstr.ok_or(Error::InvalidState("cplexpstr missing while cplinu"))?;
            if cplexpstr_val != 0 {
                let cpl_exp = cpl_ch_exps
                    .as_ref()
                    .ok_or(Error::InvalidState("cpl exponent missing"))?;
                // Coupling channel absolute exponent is
                // stored as 4 bits but represents even values (0,2,4,...,30),
                // so it must be left-shifted by 1 before use.
                Some(Self::decode_exponents(
                    cpl_exp.cplabsexp << 1,
                    &cpl_exp.cplexps,
                    cplexpstr_val,
                    ncplmant,
                ))
            } else {
                // Reuse previous block's decoded coupling exponents
                Some(
                    prev.cpl_decoded_exps
                        .ok_or(Error::InvalidState("no previous cpl exponents for reuse"))?,
                )
            }
        } else {
            None
        };

        // LFE channel (always D15, nlfemant=7)
        let lfe_decoded_exps = if lfeon {
            if lfeexpstr.is_some_and(|s| s) {
                let lfe = lfe_ch_exps
                    .as_ref()
                    .and_then(|o| o.as_ref())
                    .ok_or(Error::InvalidState("lfe exponent missing"))?;
                Some(Self::decode_exponents(lfe.abs_exp, &lfe.lfeexps, 1, 7))
            } else {
                // Reuse previous block's decoded LFE exponents
                Some(
                    prev.lfe_decoded_exps
                        .ok_or(Error::InvalidState("no previous lfe exponents for reuse"))?,
                )
            }
        } else {
            None
        };

        // Resolve bai/snroffst/cplleak: use current if present, otherwise previous block
        let eff_bai = bai
            .as_ref()
            .or(prev.bai.as_ref())
            .ok_or(Error::InvalidState("bai missing (no current or previous)"))?;
        let eff_snroffst =
            snroffst
                .as_ref()
                .or(prev.snroffst.as_ref())
                .ok_or(Error::InvalidState(
                    "snroffst missing (no current or previous)",
                ))?;

        // Helper: convert DeltBaSegment to tuple for compute_bap
        let segs_to_tuples = |segs: &[DeltBaSegment]| -> Vec<(u8, u8, u8)> {
            segs.iter()
                .map(|s| (s.deltoffst, s.deltlen, s.deltba))
                .collect()
        };

        // Resolve effective cplleak for coupling BAP computation
        let eff_cplleak = cplleak
            .as_ref()
            .or(prev.cplleak.as_ref())
            .map(|cl| (cl.cplfleak, cl.cplsleak));

        // Per-channel bap
        let mut ch_bap = Vec::with_capacity(nfchans as usize);
        for ch in 0..nfchans as usize {
            let params = BapParams {
                sdcycod: eff_bai.sdcycod,
                fdcycod: eff_bai.fdcycod,
                sgaincod: eff_bai.sgaincod,
                dbpbcod: eff_bai.dbpbcod,
                floorcod: eff_bai.floorcod,
                csnroffst: eff_snroffst.csnroffst,
                fsnroffst: eff_snroffst.fsnroffst[ch],
                fgaincod: eff_snroffst.fgaincod[ch],
                fscod,
            };
            let deltba_tuples = eff_deltba.as_ref().and_then(|d| {
                d.deltsegs
                    .get(ch)
                    .and_then(|s| s.as_ref().map(|segs| segs_to_tuples(segs)))
            });
            ch_bap.push(compute_bap(
                &ch_decoded_exps[ch],
                0,
                nchmant[ch],
                &params,
                deltba_tuples.as_deref(),
                None, // cplleak only for coupling channel
            ));
        }

        // Coupling channel bap
        let cpl_bap = if cplinu {
            let cplstrtmant =
                37 + 12 * cplbegf.ok_or(Error::InvalidState("cplbegf missing"))? as usize;
            let cplendmant = cplstrtmant + ncplmant;
            let params = BapParams {
                sdcycod: eff_bai.sdcycod,
                fdcycod: eff_bai.fdcycod,
                sgaincod: eff_bai.sgaincod,
                dbpbcod: eff_bai.dbpbcod,
                floorcod: eff_bai.floorcod,
                csnroffst: eff_snroffst.csnroffst,
                fsnroffst: eff_snroffst
                    .cplfsnroffst
                    .ok_or(Error::InvalidState("cplfsnroffst missing"))?,
                fgaincod: eff_snroffst
                    .cplfgaincod
                    .ok_or(Error::InvalidState("cplfgaincod missing"))?,
                fscod,
            };
            let deltba_tuples = eff_deltba
                .as_ref()
                .and_then(|d| d.cpldeltsegs.as_ref().map(|segs| segs_to_tuples(segs)));
            Some(compute_bap(
                cpl_decoded_exps
                    .as_ref()
                    .ok_or(Error::InvalidState("cpl decoded exps missing"))?,
                cplstrtmant,
                cplendmant,
                &params,
                deltba_tuples.as_deref(),
                eff_cplleak,
            ))
        } else {
            None
        };

        // LFE channel bap
        let lfe_bap = if lfeon {
            let params = BapParams {
                sdcycod: eff_bai.sdcycod,
                fdcycod: eff_bai.fdcycod,
                sgaincod: eff_bai.sgaincod,
                dbpbcod: eff_bai.dbpbcod,
                floorcod: eff_bai.floorcod,
                csnroffst: eff_snroffst.csnroffst,
                fsnroffst: eff_snroffst
                    .lfefsnroffst
                    .ok_or(Error::InvalidState("lfefsnroffst missing"))?,
                fgaincod: eff_snroffst
                    .lfefgaincod
                    .ok_or(Error::InvalidState("lfefgaincod missing"))?,
                fscod,
            };
            Some(compute_bap(
                lfe_decoded_exps
                    .as_ref()
                    .ok_or(Error::InvalidState("lfe decoded exps missing"))?,
                0,
                7,
                &params,
                None,
                None, // no cplleak for LFE
            ))
        } else {
            None
        };

        let ch_bap_with_mant: Vec<(Vec<u8>, usize)> =
            ch_bap.into_iter().zip(nchmant.iter().copied()).collect();
        let cpl_bap_pair = cpl_bap.as_deref().map(|b| (b, ncplmant));
        let mant_params = MantissaParams {
            channels: &ch_bap_with_mant,
            chincpl: &chincpl,
            coupling: cpl_bap_pair,
            lfe: lfe_bap.as_deref(),
        };
        let mantissas = parse_mantissas(reader, &mant_params)?;
        // ------------------------------------------------------------------------------------------

        let state = AudblkDecodedState {
            cplinu,
            chincpl,
            phsflginu,
            ncplbnd,
            cplbegf,
            cplendf,
            bai: bai.or_else(|| prev.bai.clone()),
            snroffst: snroffst.or_else(|| prev.snroffst.clone()),
            cplleak: cplleak.or_else(|| prev.cplleak.clone()),
            eff_deltba,
            ch_decoded_exps,
            cpl_decoded_exps,
            lfe_decoded_exps,
            chbwcod,
        };

        Ok((
            AudioBlock {
                blksw,
                dithflag,
                dynrng,
                dynrng2,
                cpl,
                rematrixing,
                exponent_strategy,
                cpl_ch_exps,
                fb_ch_exps,
                lfe_ch_exps,
                bapi: bit_allocation_parametric_information,
                deltbai,
                unused_dummy,
                mantissas,
            },
            state,
        ))
    }

    /// auxdata: variable-length data between the last audblk and errorcheck.
    fn parse_auxdata(reader: &mut BitstreamReader<'_>) -> Result<AuxiliaryData, Error> {
        let remaining = reader.remaining_bits();
        let aux_bits = remaining.saturating_sub(17);
        let aux_full_bytes = aux_bits / 8;
        let aux_leftover_bits = aux_bits % 8;
        let mut auxdata =
            Vec::with_capacity(aux_full_bytes + if aux_leftover_bits > 0 { 1 } else { 0 });
        for _ in 0..aux_full_bytes {
            auxdata.push(reader.read_bits(8)? as u8);
        }
        if aux_leftover_bits > 0 {
            auxdata.push(reader.read_bits(aux_leftover_bits as u8)? as u8);
        }
        let last_byte_bits = if aux_leftover_bits > 0 {
            aux_leftover_bits as u8
        } else if aux_full_bytes > 0 {
            8
        } else {
            0
        };
        Ok(AuxiliaryData {
            auxdata,
            last_byte_bits,
        })
    }

    /// errorcheck: crcrsv (1 bit) + crc2 (16 bits)
    fn parse_errorcheck(reader: &mut BitstreamReader<'_>) -> Result<ErrorCheck, Error> {
        Ok(ErrorCheck {
            crcrsv: reader.read_bit()?,
            crc2: reader.read_bits(16)? as u16,
        })
    }

    /// Write an audio block to the bitstream.
    fn write_audblk(
        writer: &mut BitstreamWriter,
        nfchans: u8,
        acmod: u8,
        lfeon: bool,
        ab: &AudioBlock,
        prev: &mut AudblkDecodedState,
    ) -> Result<(), Error> {
        // blksw, dithflag
        for &b in &ab.blksw {
            writer.write_bool(b);
        }
        for &b in &ab.dithflag {
            writer.write_bool(b);
        }

        // dynrng
        writer.write_bool(ab.dynrng.is_some());
        if let Some(d) = ab.dynrng {
            writer.write_bits(d as u32, 8);
        }
        // dynrng2 (dual mono only)
        if acmod == 0 {
            writer.write_bool(ab.dynrng2.is_some());
            if let Some(d) = ab.dynrng2 {
                writer.write_bits(d as u32, 8);
            }
        }

        // coupling strategy
        let mut cplinu = prev.cplinu;
        let mut chincpl = prev.chincpl.clone();
        let mut ncplbnd = prev.ncplbnd;
        match &ab.cpl.strategy {
            CplStrategy::Reuse => {
                writer.write_bool(false); // cplstre=0
            }
            CplStrategy::NotInUse => {
                writer.write_bool(true); // cplstre=1
                writer.write_bool(false); // cplinu=0
                cplinu = false;
                chincpl = vec![false; nfchans as usize];
                ncplbnd = 0;
            }
            CplStrategy::InUse {
                chincpl: ci,
                phsflginu,
                cplbegf,
                cplendf,
                cplbndstrc,
            } => {
                writer.write_bool(true); // cplstre=1
                writer.write_bool(true); // cplinu=1
                cplinu = true;
                chincpl = ci.clone();
                for &b in ci {
                    writer.write_bool(b);
                }
                if acmod == 0x2 {
                    writer.write_bool(
                        phsflginu.ok_or(Error::InvalidState("phsflginu must be set for stereo"))?,
                    );
                }
                writer.write_bits(*cplbegf as u32, 4);
                writer.write_bits(*cplendf as u32, 4);
                for &b in cplbndstrc {
                    writer.write_bool(b);
                }
                ncplbnd = 1 + cplbndstrc.iter().filter(|&&b| !b).count();
            }
        }

        // coupling coordinates
        if let Some(ref coord) = ab.cpl.coord {
            for ch_coord in &coord.channels {
                if let Some(cc) = ch_coord {
                    writer.write_bool(true); // cplcoe=1
                    writer.write_bits(cc.mstrcplco as u32, 2);
                    for &(exp, mant) in &cc.bands {
                        writer.write_bits(exp as u32, 4);
                        writer.write_bits(mant as u32, 4);
                    }
                } else {
                    writer.write_bool(false); // cplcoe=0
                }
            }
            if let Some(ref flags) = coord.phsflg {
                for &f in flags {
                    writer.write_bool(f);
                }
            }
        }

        // rematrixing (2/0 mode only)
        if acmod == 0x2 {
            let has_remat = ab.rematrixing.rematflg.is_some();
            writer.write_bool(has_remat);
            if let Some(ref flags) = ab.rematrixing.rematflg {
                for &f in flags {
                    writer.write_bool(f);
                }
            }
        }

        // exponent strategy
        let es = &ab.exponent_strategy;
        if cplinu {
            writer.write_bits(
                es.cplexpstr
                    .ok_or(Error::InvalidState("cplexpstr missing"))? as u32,
                2,
            );
        }
        for &s in &es.chexpstr {
            writer.write_bits(s as u32, 2);
        }
        if lfeon {
            writer.write_bool(
                es.lfeexpstr
                    .ok_or(Error::InvalidState("lfeexpstr missing"))?,
            );
        }

        // channel bandwidth code
        for (ch, (&expstr, &incpl)) in es.chexpstr.iter().zip(chincpl.iter()).enumerate() {
            if expstr != 0 && !incpl {
                writer.write_bits(
                    es.chbwcod[ch].ok_or(Error::InvalidState("chbwcod missing"))? as u32,
                    6,
                );
            }
        }

        // coupling channel exponents
        if let Some(ref cpl_exp) = ab.cpl_ch_exps {
            writer.write_bits(cpl_exp.cplabsexp as u32, 4);
            for &e in &cpl_exp.cplexps {
                writer.write_bits(e as u32, 7);
            }
        }

        // full bandwidth channel exponents
        for ch_exp in ab.fb_ch_exps.channels.iter().flatten() {
            writer.write_bits(ch_exp.abs_exp as u32, 4);
            for &e in &ch_exp.exps {
                writer.write_bits(e as u32, 7);
            }
            writer.write_bits(ch_exp.gainrng as u32, 2);
        }

        // LFE exponents
        if let Some(Some(lfe)) = &ab.lfe_ch_exps {
            writer.write_bits(lfe.abs_exp as u32, 4);
            for &e in &lfe.lfeexps {
                writer.write_bits(e as u32, 7);
            }
        }

        // Bit-allocation parametric information
        let bapi = &ab.bapi;
        writer.write_bool(bapi.bai.is_some());
        if let Some(ref bai) = bapi.bai {
            writer.write_bits(bai.sdcycod as u32, 2);
            writer.write_bits(bai.fdcycod as u32, 2);
            writer.write_bits(bai.sgaincod as u32, 2);
            writer.write_bits(bai.dbpbcod as u32, 2);
            writer.write_bits(bai.floorcod as u32, 3);
        }

        // SNR offset
        writer.write_bool(bapi.snroffst.is_some());
        if let Some(ref snr) = bapi.snroffst {
            writer.write_bits(snr.csnroffst as u32, 6);
            if cplinu {
                writer.write_bits(
                    snr.cplfsnroffst
                        .ok_or(Error::InvalidState("cplfsnroffst missing"))?
                        as u32,
                    4,
                );
                writer.write_bits(
                    snr.cplfgaincod
                        .ok_or(Error::InvalidState("cplfgaincod missing"))?
                        as u32,
                    3,
                );
            }
            for ch in 0..nfchans as usize {
                writer.write_bits(snr.fsnroffst[ch] as u32, 4);
                writer.write_bits(snr.fgaincod[ch] as u32, 3);
            }
            if lfeon {
                writer.write_bits(
                    snr.lfefsnroffst
                        .ok_or(Error::InvalidState("lfefsnroffst missing"))?
                        as u32,
                    4,
                );
                writer.write_bits(
                    snr.lfefgaincod
                        .ok_or(Error::InvalidState("lfefgaincod missing"))?
                        as u32,
                    3,
                );
            }
        }

        // Coupling leak
        if cplinu {
            writer.write_bool(bapi.cplleak.is_some());
            if let Some(ref cl) = bapi.cplleak {
                writer.write_bits(cl.cplfleak as u32, 3);
                writer.write_bits(cl.cplsleak as u32, 3);
            }
        }

        // Delta bit allocation
        writer.write_bool(ab.deltbai.is_some());
        if let Some(ref dba) = ab.deltbai {
            if cplinu {
                writer.write_bits(
                    dba.cpldeltbae
                        .ok_or(Error::InvalidState("cpldeltbae missing"))?
                        as u32,
                    2,
                );
            }
            for &d in &dba.deltbae {
                writer.write_bits(d as u32, 2);
            }
            // coupling delta segments
            if let Some(ref segs) = dba.cpldeltsegs {
                writer.write_bits((segs.len() - 1) as u32, 3);
                for s in segs {
                    writer.write_bits(s.deltoffst as u32, 5);
                    writer.write_bits(s.deltlen as u32, 4);
                    writer.write_bits(s.deltba as u32, 3);
                }
            }
            // per-channel delta segments
            for segs in dba.deltsegs.iter().flatten() {
                writer.write_bits((segs.len() - 1) as u32, 3);
                for s in segs {
                    writer.write_bits(s.deltoffst as u32, 5);
                    writer.write_bits(s.deltlen as u32, 4);
                    writer.write_bits(s.deltba as u32, 3);
                }
            }
        }

        // Unused dummy data (skip)
        writer.write_bool(ab.unused_dummy.is_some());
        if let Some(ref dummy) = ab.unused_dummy {
            writer.write_bits(dummy.skipl as u32, 9);
            for &b in &dummy.skipfld {
                writer.write_bits(b as u32, 8);
            }
        }

        let cplbegf = match &ab.cpl.strategy {
            CplStrategy::InUse { cplbegf, .. } => Some(*cplbegf),
            _ => prev.cplbegf,
        };
        let cplendf = match &ab.cpl.strategy {
            CplStrategy::InUse { cplendf, .. } => Some(*cplendf),
            _ => prev.cplendf,
        };

        // Mantissas
        write_mantissas(writer, &ab.mantissas);

        // Update prev state
        prev.cplinu = cplinu;
        prev.chincpl = chincpl;
        prev.ncplbnd = ncplbnd;
        prev.cplbegf = cplbegf;
        prev.cplendf = cplendf;
        for (ch, &expstr) in es.chexpstr.iter().enumerate().take(nfchans as usize) {
            if expstr != 0 {
                prev.chbwcod[ch] = es.chbwcod[ch];
            }
        }
        Ok(())
    }

    /// Decode grouped differential exponents into per-bin absolute exponents.
    fn decode_exponents(absexp: u8, gexp: &[u8], expstr: u8, ncoefs: usize) -> [u8; 256] {
        let ngrps = gexp.len();

        // grpsize = 1 for D15, 2 for D25, 4 for D45
        let grpsize: usize = match expstr {
            1 => 1,
            2 => 2,
            3 => 4,
            _ => unreachable!(),
        };

        // Unpack mapped values from 7-bit groups
        let mut dexp = [0i32; 256 * 3];
        for grp in 0..ngrps {
            let expacc = gexp[grp] as i32;
            dexp[grp * 3] = expacc / 25;
            let rem = expacc % 25;
            dexp[grp * 3 + 1] = rem / 5;
            dexp[grp * 3 + 2] = rem % 5;
        }

        // Remove mapping bias (dexp = mapped_value - 2)
        for d in dexp[..(ngrps * 3)].iter_mut() {
            *d -= 2;
        }

        // Convert differentials to absolutes
        //   aexp[i] = prevexp + dexp[i]
        let mut aexp = [0i32; 256 * 3];
        let mut prevexp = absexp as i32;
        for (a, d) in aexp.iter_mut().zip(dexp.iter()).take(ngrps * 3) {
            *a = prevexp + d;
            prevexp = *a;
        }

        // Expand to full absolute exponent array using grpsize
        // exp[0] = absexp
        // exp[(i * grpsize) + j + 1] = aexp[i]
        let mut exp = [0u8; 256];
        exp[0] = absexp;
        for (i, &a) in aexp[..(ngrps * 3)].iter().enumerate() {
            for j in 0..grpsize {
                let pos = i * grpsize + j + 1;
                if pos < ncoefs {
                    exp[pos] = a as u8;
                }
            }
        }

        exp
    }
}
