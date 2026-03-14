use crate::{ac3::error::Error, util::bitstream::BitstreamReader};

#[derive(Debug)]
pub struct SyncInfo {
    pub syncword: u16,
    pub crc1: u16,
    pub fscod: u8,
    pub frmsizecod: u8,
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
    pub audio_mode: AudioMode,
    pub lfeon: bool,
    pub dialnorm: u8,
    pub compr: Option<u8>,
    pub langcod: Option<u8>,
    pub audprodi: Option<(u8, u8)>, // (mixlevel, roomtyp)
    pub copyrightb: bool,
    pub origbs: bool,
    pub xbsi1: Option<Xbsi1>,
    pub xbsi2: Option<Xbsi2>,
    pub addbsi: Option<Addbsi>,
}

#[derive(Debug)]
pub struct AudioBlock {
    pub ab_0: Vec<u8>,
    pub ab_1: Vec<u8>,
    pub ab_2: Vec<u8>,
    pub ab_3: Vec<u8>,
    pub ab_4: Vec<u8>,
    pub ab_5: Vec<u8>,
}

#[derive(Debug)]
pub struct AuxiliaryData {}

#[derive(Debug)]
pub struct ErrorCheck {
    pub crcrsv: bool,
    pub crc2: u16,
}

#[derive(Debug)]
pub struct Ac3 {
    pub si: SyncInfo,
    pub bsi: BitStreamInformation,
    pub ab: AudioBlock,
    pub aux: AuxiliaryData,
    pub ec: ErrorCheck,
}

impl Ac3 {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);

        // syncinfo
        let si = Self::parse_si(&mut reader)?;

        // bsi
        let bsi = Self::parse_bsi(&mut reader)?;

        // audblk

        // auxdata

        // errorcheck

        todo!()
    }

    fn parse_si(reader: &mut BitstreamReader<'_>) -> Result<SyncInfo, Error> {
        Ok(SyncInfo {
            syncword: reader.read_bits(16)? as u16,
            crc1: reader.read_bits(16)? as u16,
            fscod: reader.read_bits(2)? as u8,
            frmsizecod: reader.read_bits(6)? as u8,
        })
    }

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
            audio_mode,
            lfeon,
            dialnorm,
            compr,
            langcod,
            audprodi,
            copyrightb,
            origbs,
            xbsi1,
            xbsi2,
            addbsi,
        })
    }
}
