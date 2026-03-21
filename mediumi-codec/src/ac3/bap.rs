//! Bit Allocation Algorithm

// slowdec[sdcycod] (Table 7.6)
const SLOWDEC: [u16; 4] = [0x0F, 0x11, 0x13, 0x15];

// fastdec[fdcycod] (Table 7.7)
const FASTDEC: [u16; 4] = [0x3F, 0x53, 0x67, 0x7B];

// slowgain[sgaincod] (Table 7.8)
const SLOWGAIN: [u16; 4] = [0x540, 0x4D8, 0x478, 0x410];

// dbpbtab[dbpbcod] (Table 7.9)
const DBPBTAB: [u16; 4] = [0x000, 0x0700, 0x0900, 0x0B00];

// floortab[floorcod] (Table 7.10)
const FLOORTAB: [i16; 8] = [0x2F0, 0x2B0, 0x270, 0x230, 0x1F0, 0x170, 0x0F0, -0x0800];

// fastgain[fgaincod] (Table 7.11)
const FASTGAIN: [u16; 8] = [0x080, 0x100, 0x180, 0x200, 0x280, 0x300, 0x380, 0x400];

// Hearing threshold table hth[fscod][band] (Table 7.15)
const HTH: [[i16; 50]; 3] = [
    // 48kHz (fscod=0)
    [
        0x04D0, 0x04D0, 0x0440, 0x0400, 0x03E0, 0x03C0, 0x03B0, 0x03B0, 0x03A0, 0x03A0, 0x03A0,
        0x03A0, 0x03A0, 0x0390, 0x0390, 0x0390, 0x0380, 0x0380, 0x0370, 0x0370, 0x0360, 0x0360,
        0x0350, 0x0350, 0x0340, 0x0340, 0x0330, 0x0320, 0x0310, 0x0300, 0x02F0, 0x02F0, 0x02F0,
        0x02F0, 0x0300, 0x0310, 0x0340, 0x0390, 0x03E0, 0x0420, 0x0460, 0x0490, 0x04A0, 0x0460,
        0x0440, 0x0440, 0x0520, 0x0800, 0x0840, 0x0840,
    ],
    // 44.1kHz (fscod=1)
    [
        0x04F0, 0x04F0, 0x0460, 0x0410, 0x03E0, 0x03D0, 0x03C0, 0x03B0, 0x03B0, 0x03A0, 0x03A0,
        0x03A0, 0x03A0, 0x03A0, 0x0390, 0x0390, 0x0390, 0x0380, 0x0380, 0x0380, 0x0370, 0x0370,
        0x0360, 0x0360, 0x0350, 0x0350, 0x0340, 0x0340, 0x0320, 0x0310, 0x0300, 0x02F0, 0x02F0,
        0x02F0, 0x02F0, 0x0300, 0x0320, 0x0350, 0x0390, 0x03E0, 0x0420, 0x0450, 0x04A0, 0x0490,
        0x0460, 0x0440, 0x0480, 0x0630, 0x0840, 0x0840,
    ],
    // 32kHz (fscod=2)
    [
        0x0580, 0x0580, 0x04B0, 0x0450, 0x0420, 0x03F0, 0x03E0, 0x03D0, 0x03C0, 0x03B0, 0x03B0,
        0x03B0, 0x03A0, 0x03A0, 0x03A0, 0x03A0, 0x03A0, 0x03A0, 0x03A0, 0x03A0, 0x0390, 0x0390,
        0x0390, 0x0390, 0x0380, 0x0380, 0x0380, 0x0370, 0x0360, 0x0350, 0x0340, 0x0330, 0x0320,
        0x0310, 0x0300, 0x02F0, 0x02F0, 0x02F0, 0x0300, 0x0310, 0x0330, 0x0350, 0x03C0, 0x0410,
        0x0470, 0x04A0, 0x0460, 0x0440, 0x0450, 0x04E0,
    ],
];

// bndtab[band]: first mantissa number in each band (Table 7.12)
const BNDTAB: [usize; 50] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 31, 34, 37, 40, 43, 46, 49, 55, 61, 67, 73, 79, 85, 97, 109, 121, 133, 157, 181,
    205, 229,
];

// bndsz[band]: width of each band in number of mantissas (Table 7.12)
const BNDSZ: [usize; 50] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3,
    3, 3, 3, 6, 6, 6, 6, 6, 6, 12, 12, 12, 12, 24, 24, 24, 24, 24,
];

// baptab[] (Table 7.16)
const BAPTAB: [u8; 64] = [
    0, 1, 1, 1, 1, 1, 2, 2, 3, 3, 3, 4, 4, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 8, 9, 9, 9, 9,
    10, 10, 10, 10, 11, 11, 11, 11, 12, 12, 12, 12, 13, 13, 13, 13, 14, 14, 14, 14, 14, 14, 14, 14,
    15, 15, 15, 15, 15, 15, 15, 15, 15,
];

// masktab[bin]: bin → band mapping (Table 7.13)
const MASKTAB: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 28, 28, 29, 29, 29, 30, 30, 30, 31, 31, 31, 32, 32, 32, 33, 33, 33, 34, 34, 34, 35,
    35, 35, 35, 35, 35, 36, 36, 36, 36, 36, 36, 37, 37, 37, 37, 37, 37, 38, 38, 38, 38, 38, 38, 39,
    39, 39, 39, 39, 39, 40, 40, 40, 40, 40, 40, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 43, 44,
    44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
    45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46,
    46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 46, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47,
    47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 47, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48,
    48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49,
    49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 49, 0, 0, 0,
];

// latab[] (Table 7.18: Log Addition Table)
// address = min(abs(c) >> 1, 255)
const LATAB: [u8; 256] = [
    0x40, 0x3F, 0x3E, 0x3D, 0x3C, 0x3B, 0x3A, 0x39, 0x38, 0x37, 0x36, 0x35, 0x34, 0x34, 0x33, 0x32,
    0x31, 0x30, 0x2F, 0x2F, 0x2E, 0x2D, 0x2C, 0x2C, 0x2B, 0x2A, 0x29, 0x29, 0x28, 0x27, 0x26, 0x26,
    0x25, 0x24, 0x24, 0x23, 0x23, 0x22, 0x21, 0x21, 0x20, 0x20, 0x1F, 0x1E, 0x1E, 0x1D, 0x1D, 0x1C,
    0x1C, 0x1B, 0x1B, 0x1A, 0x1A, 0x19, 0x19, 0x18, 0x18, 0x17, 0x17, 0x16, 0x16, 0x15, 0x15, 0x15,
    0x14, 0x14, 0x13, 0x13, 0x13, 0x12, 0x12, 0x12, 0x11, 0x11, 0x11, 0x10, 0x10, 0x10, 0x0F, 0x0F,
    0x0F, 0x0E, 0x0E, 0x0E, 0x0D, 0x0D, 0x0D, 0x0D, 0x0C, 0x0C, 0x0C, 0x0C, 0x0B, 0x0B, 0x0B, 0x0B,
    0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x09, 0x09, 0x09, 0x09, 0x09, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x05, 0x05,
    0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x02, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn masktab(bin: usize) -> usize {
    MASKTAB[bin] as usize
}

/// logadd(a, b): log-domain addition (Section 7.2.2.3)
fn logadd(a: i32, b: i32) -> i32 {
    let c = a - b;
    let address = ((c.abs() >> 1) as usize).min(255);
    if c >= 0 {
        a + LATAB[address] as i32
    } else {
        b + LATAB[address] as i32
    }
}

/// calc_lowcomp(a, b0, b1, bin) (Section 7.2.2.4)
fn calc_lowcomp(a: i32, b0: i32, b1: i32, bin: usize) -> i32 {
    if bin < 7 {
        if b0 + 256 == b1 {
            384
        } else if b0 > b1 {
            (a - 64).max(0)
        } else {
            a
        }
    } else if bin < 20 {
        if b0 + 256 == b1 {
            320
        } else if b0 > b1 {
            (a - 64).max(0)
        } else {
            a
        }
    } else {
        (a - 128).max(0)
    }
}

/// Bit allocation parameters from the bitstream (Section 7.2.2.1).
pub struct BapParams {
    pub sdcycod: u8,   // slow decay code
    pub fdcycod: u8,   // fast decay code
    pub sgaincod: u8,  // slow gain code
    pub dbpbcod: u8,   // dB/bit code
    pub floorcod: u8,  // floor code
    pub fgaincod: u8,  // fast gain code
    pub csnroffst: u8, // coarse SNR offset
    pub fsnroffst: u8, // fine SNR offset
    pub fscod: u8,     // sample rate code
}

/// Compute bit allocation pointers (bap[]) for a range of bins.
pub fn compute_bap(
    exps: &[u8],
    start: usize,
    end: usize,
    params: &BapParams,
    deltba: Option<&[(u8, u8, u8)]>,
    cplleak: Option<(u8, u8)>,
) -> Vec<u8> {
    let nbins = end - start;
    if nbins == 0 || end > 256 {
        return vec![0u8; nbins];
    }

    let sdecay = SLOWDEC[params.sdcycod as usize] as i32;
    let fdecay = FASTDEC[params.fdcycod as usize] as i32;
    let sgain = SLOWGAIN[params.sgaincod as usize] as i32;
    let dbknee = DBPBTAB[params.dbpbcod as usize] as i32;
    let floor = FLOORTAB[params.floorcod as usize] as i32;
    let fgain = FASTGAIN[params.fgaincod as usize] as i32;
    let snroffset = (((params.csnroffst as i32 - 15) << 4) + params.fsnroffst as i32) << 2;

    let mut psd = [0i32; 256];
    for i in 0..nbins {
        psd[start + i] = 3072 - ((exps[i] as i32) << 7);
    }

    let bndstrt = masktab(start);
    let bndend = masktab(end - 1) + 1;

    // lastbin = min(bndtab[k] + bndsz[k], end)
    let mut bndpsd = [0i32; 50];
    {
        let mut j = start;
        let mut k = bndstrt;
        loop {
            let lastbin = (BNDTAB[k] + BNDSZ[k]).min(end);
            bndpsd[k] = psd[j];
            j += 1;
            while j < lastbin {
                bndpsd[k] = logadd(bndpsd[k], psd[j]);
                j += 1;
            }
            k += 1;
            if end <= lastbin {
                break;
            }
        }
    }

    let mut excite = [0i32; 50];
    let mut begin;
    let mut fastleak = 0i32;
    let mut slowleak = 0i32;

    if let Some((cplfleak, cplsleak)) = cplleak {
        fastleak = ((cplfleak as i32) << 8) + 768;
        slowleak = ((cplsleak as i32) << 8) + 768;
    }

    if bndstrt == 0 {
        // FBW and LFE channels
        let is_lfe = bndend == 7;
        let mut lowcomp = 0i32;

        lowcomp = calc_lowcomp(lowcomp, bndpsd[0], bndpsd[1], 0);
        excite[0] = bndpsd[0] - fgain - lowcomp;
        lowcomp = calc_lowcomp(lowcomp, bndpsd[1], bndpsd[2], 1);
        excite[1] = bndpsd[1] - fgain - lowcomp;

        begin = 7;
        for bin in 2..7 {
            if !is_lfe || bin != 6 {
                lowcomp = calc_lowcomp(lowcomp, bndpsd[bin], bndpsd[bin + 1], bin);
            }
            fastleak = bndpsd[bin] - fgain;
            slowleak = bndpsd[bin] - sgain;
            excite[bin] = fastleak - lowcomp;
            if (!is_lfe || bin != 6) && bndpsd[bin] <= bndpsd[bin + 1] {
                begin = bin + 1;
                break;
            }
        }

        let end1 = bndend.min(22);
        for bin in begin..end1 {
            if !is_lfe || bin != 6 {
                lowcomp = calc_lowcomp(lowcomp, bndpsd[bin], bndpsd[bin + 1], bin);
            }
            fastleak = (fastleak - fdecay).max(bndpsd[bin] - fgain);
            slowleak = (slowleak - sdecay).max(bndpsd[bin] - sgain);
            excite[bin] = (fastleak - lowcomp).max(slowleak);
        }
        begin = 22;
    } else {
        // Coupling channel
        begin = bndstrt;
    }

    for bin in begin..bndend {
        fastleak = (fastleak - fdecay).max(bndpsd[bin] - fgain);
        slowleak = (slowleak - sdecay).max(bndpsd[bin] - sgain);
        excite[bin] = fastleak.max(slowleak);
    }

    let mut mask = [0i32; 50];
    for bin in bndstrt..bndend {
        if bndpsd[bin] < dbknee {
            excite[bin] += (dbknee - bndpsd[bin]) >> 2;
        }
        mask[bin] = excite[bin].max(HTH[params.fscod as usize][bin] as i32);
    }

    if let Some(segs) = deltba {
        let mut band = 0;
        for &(deltoffst, deltlen, deltba_val) in segs {
            band += deltoffst as usize;
            let delta = if deltba_val >= 4 {
                (deltba_val as i32 - 3) << 7
            } else {
                (deltba_val as i32 - 4) << 7
            };
            for _ in 0..deltlen {
                if band < bndend {
                    mask[band] += delta;
                }
                band += 1;
            }
        }
    }

    // lastbin = min(bndtab[j] + bndsz[j], end)
    let mut bap = vec![0u8; nbins];
    {
        let mut i = start;
        let mut j = bndstrt;
        loop {
            let lastbin = (BNDTAB[j] + BNDSZ[j]).min(end);
            let mut m = mask[j] - snroffset - floor;
            if m < 0 {
                m = 0;
            }
            m = (m & 0x1FE0) + floor;
            while i < lastbin {
                let address = ((psd[i] - m) >> 5).clamp(0, 63) as usize;
                bap[i - start] = BAPTAB[address];
                i += 1;
            }
            j += 1;
            if end <= lastbin {
                break;
            }
        }
    }

    bap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masktab_one_bin_per_band() {
        for bin in 0..28 {
            assert_eq!(masktab(bin), bin, "bin {}", bin);
        }
    }

    #[test]
    fn test_masktab_multi_bin_bands() {
        assert_eq!(masktab(28), 28);
        assert_eq!(masktab(30), 28);
        assert_eq!(masktab(31), 29);
        assert_eq!(masktab(33), 29);
    }

    #[test]
    fn test_masktab_boundaries() {
        for band in 0..50 {
            let start = BNDTAB[band];
            let end = BNDTAB[band] + BNDSZ[band];
            assert_eq!(masktab(start), band, "band {} start", band);
            assert_eq!(masktab(end - 1), band, "band {} end", band);
        }
    }

    #[test]
    fn test_compute_bap_high_snr() {
        let exps = vec![0u8; 10];
        let params = BapParams {
            sdcycod: 0,
            fdcycod: 0,
            sgaincod: 0,
            dbpbcod: 0,
            floorcod: 0,
            csnroffst: 63,
            fsnroffst: 15,
            fgaincod: 0,
            fscod: 0,
        };
        let bap = compute_bap(&exps, 30, 40, &params, None, None);
        for (i, &b) in bap.iter().enumerate() {
            assert!(b > 0, "bin {} expected bap>0, got 0", i);
            assert!(b <= 15, "bin {} bap out of range: {}", i, b);
        }
    }

    #[test]
    fn test_compute_bap_zero_energy() {
        let exps = vec![24u8; 10];
        let params = BapParams {
            sdcycod: 0,
            fdcycod: 0,
            sgaincod: 0,
            dbpbcod: 0,
            floorcod: 7,
            csnroffst: 15,
            fsnroffst: 0,
            fgaincod: 0,
            fscod: 0,
        };
        let bap = compute_bap(&exps, 0, 10, &params, None, None);
        for (i, &b) in bap.iter().enumerate() {
            assert_eq!(b, 0, "bin {} expected bap=0, got {}", i, b);
        }
    }

    #[test]
    fn test_compute_bap_empty() {
        let params = BapParams {
            sdcycod: 0,
            fdcycod: 0,
            sgaincod: 0,
            dbpbcod: 0,
            floorcod: 0,
            csnroffst: 15,
            fsnroffst: 0,
            fgaincod: 0,
            fscod: 0,
        };
        let bap = compute_bap(&[], 0, 0, &params, None, None);
        assert!(bap.is_empty());
    }

    #[test]
    fn test_compute_bap_full_range() {
        let exps = vec![5u8; 253];
        let params = BapParams {
            sdcycod: 2,
            fdcycod: 1,
            sgaincod: 1,
            dbpbcod: 2,
            floorcod: 4,
            csnroffst: 31,
            fsnroffst: 7,
            fgaincod: 4,
            fscod: 1,
        };
        let bap = compute_bap(&exps, 0, 253, &params, None, None);
        assert_eq!(bap.len(), 253);
    }

    #[test]
    fn test_compute_bap_coupling() {
        let exps = vec![8u8; 120];
        let params = BapParams {
            sdcycod: 2,
            fdcycod: 1,
            sgaincod: 1,
            dbpbcod: 2,
            floorcod: 4,
            csnroffst: 31,
            fsnroffst: 7,
            fgaincod: 4,
            fscod: 1,
        };
        let bap = compute_bap(&exps, 37, 157, &params, None, None);
        assert_eq!(bap.len(), 120);
    }
}
