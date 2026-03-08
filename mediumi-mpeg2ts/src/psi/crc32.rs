const CRC32_TABLE: [u32; 256] = {
    let poly: u32 = 0x04C1_1DB7;
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut j = 0;
        while j < 8 {
            if crc & 0x8000_0000 != 0 {
                crc = (crc << 1) ^ poly;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc = (crc << 8) ^ CRC32_TABLE[((crc >> 24) as u8 ^ byte) as usize];
    }
    crc
}

pub fn verify(data: &[u8]) -> bool {
    crc32(data) == 0x0000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_known_value() {
        let data = b"123456789";
        assert_eq!(crc32(data), 0x0376_E6E7);
    }

    #[test]
    fn test_verify_valid() {
        let data = b"123456789";
        let checksum = crc32(data).to_be_bytes();
        let mut section = data.to_vec();
        section.extend_from_slice(&checksum);
        assert!(verify(&section));
    }

    #[test]
    fn test_verify_invalid() {
        let mut section = b"123456789".to_vec();
        section.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        assert!(!verify(&section));
    }
}
