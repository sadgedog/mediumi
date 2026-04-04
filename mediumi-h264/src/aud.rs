use crate::{error::Error, util::bitstream::BitstreamReader};

#[derive(Debug, PartialEq)]
pub enum PrimaryPicType {
    I,       // 0: slice_type 2, 7
    PI,      // 1: slice_type 0, 2, 5, 7
    PBI,     // 2: slice_type 0, 1, 2, 5, 6, 7
    SI,      // 3: slice_type 4, 9
    SpSi,    // 4: slice_type 3, 4, 8, 9
    ISi,     // 5: slice_type 2, 4, 7, 9
    PISpSi,  // 6: slice_type 0, 2, 3, 4, 5, 7, 8, 9
    PBISpSi, // 7: slice_type 0, 1, 2, 3, 4, 5, 6, 7, 8, 9
}

impl TryFrom<u8> for PrimaryPicType {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::I),       // I slice
            1 => Ok(Self::PI),      // P, I slice
            2 => Ok(Self::PBI),     // P, B, I slice
            3 => Ok(Self::SI),      // S, I slice
            4 => Ok(Self::SpSi),    // SP, SI slice
            5 => Ok(Self::ISi),     // I, SI slice
            6 => Ok(Self::PISpSi),  // P, I, SP, SI slice
            7 => Ok(Self::PBISpSi), // P, B, I, SP, SI slice
            _ => Err(Error::InvalidPrimaryPicType(value)),
        }
    }
}

impl From<&PrimaryPicType> for u8 {
    fn from(value: &PrimaryPicType) -> Self {
        match value {
            PrimaryPicType::I => 0,
            PrimaryPicType::PI => 1,
            PrimaryPicType::PBI => 2,
            PrimaryPicType::SI => 3,
            PrimaryPicType::SpSi => 4,
            PrimaryPicType::ISi => 5,
            PrimaryPicType::PISpSi => 6,
            PrimaryPicType::PBISpSi => 7,
        }
    }
}

#[derive(Debug)]
pub struct Aud {
    pub primary_pic_type: PrimaryPicType,
}

impl Aud {
    pub fn to_bytes(&self) -> Vec<u8> {
        let v: u8 = (&self.primary_pic_type).into();
        vec![v << 5 | 0b0001_0000] // rbsp stop one bit & aligment zero bits
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let primary_pic_type = PrimaryPicType::try_from(reader.read_bits(3)? as u8)?;
        Ok(Self { primary_pic_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let cases: &[(u8, PrimaryPicType)] = &[
            (0b000_10000, PrimaryPicType::I),       // 0
            (0b001_10000, PrimaryPicType::PI),      // 1
            (0b010_10000, PrimaryPicType::PBI),     // 2
            (0b011_10000, PrimaryPicType::SI),      // 3
            (0b100_10000, PrimaryPicType::SpSi),    // 4
            (0b101_10000, PrimaryPicType::ISi),     // 5
            (0b110_10000, PrimaryPicType::PISpSi),  // 6
            (0b111_10000, PrimaryPicType::PBISpSi), // 7
        ];

        for (byte, expected_type) in cases {
            let input = vec![*byte];
            let aud = Aud::parse(&input).expect("failed to parse aud");
            assert_eq!(aud.primary_pic_type, *expected_type);
            assert_eq!(Aud::to_bytes(&aud), input);
        }
    }
}
