#[derive(Debug)]
pub struct FillerData {
    pub ff_byte_count: usize,
}

impl FillerData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0xFF; self.ff_byte_count];
        buf.push(0x80); // trailing_bits
        buf
    }

    pub fn parse(data: &[u8]) -> Self {
        let ff_byte_count = data.iter().take_while(|&&b| b == 0xFF).count();
        Self { ff_byte_count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let cases = [0, 1, 5, 100];
        for count in cases {
            let mut input = vec![0xFF; count];
            input.push(0x80); // trailing bits
            let filler = FillerData::parse(&input);
            assert_eq!(filler.ff_byte_count, count);
            assert_eq!(filler.to_bytes(), input);
        }
    }
}
