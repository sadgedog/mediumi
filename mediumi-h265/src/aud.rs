use crate::error::Error;
use mediumi_util::bitstream::BitstreamReader;

#[derive(Debug, PartialEq)]
pub enum PicType {
    I,   // slice_type 0
    PI,  // slice_type 1
    BPI, // slice_type 2
}

impl TryFrom<u8> for PicType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::I),
            1 => Ok(Self::PI),
            2 => Ok(Self::BPI),
            _ => Err(Error::InvalidPicType(value)),
        }
    }
}

impl From<&PicType> for u8 {
    fn from(value: &PicType) -> Self {
        match value {
            PicType::I => 0,
            PicType::PI => 1,
            PicType::BPI => 2,
        }
    }
}

#[derive(Debug)]
pub struct Aud {
    pub pic_type: PicType,
}

impl Aud {
    pub fn to_bytes(&self) -> Vec<u8> {
        let pic_type_value: u8 = (&self.pic_type).into();
        // trailing bits: rbsp stop one bit (1) + alignment zero bits (4)
        vec![pic_type_value << 5 | 0b0001_0000]
    }

    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.is_empty() {
            return Err(Error::DataTooShort);
        }

        let mut reader = BitstreamReader::new(data);
        let pic_type_value = reader.read_bits(3)? as u8;
        let pic_type = PicType::try_from(pic_type_value)?;

        let trailing_bits = reader.read_bits(5)? as u8;
        if trailing_bits != 0b0001_0000 {
            return Err(Error::InvalidTrailingBits);
        }

        Ok(Self { pic_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aud_roundtrip() {
        let aud = Aud {
            pic_type: PicType::PI,
        };

        let bytes = aud.to_bytes();
        let parsed_aud = Aud::parse(&bytes).expect("Failed to parse AUD");

        assert_eq!(aud.pic_type, parsed_aud.pic_type);
    }
}
