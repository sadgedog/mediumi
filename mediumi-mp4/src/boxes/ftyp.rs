use crate::{
    boxes::{BaseBox, Error},
    types::BoxType,
    util::bitstream::{BitstreamReader, BitstreamWriter},
};

#[derive(Debug, PartialEq)]
pub struct Brand([u8; 4]);

impl Brand {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("????")
    }
}

#[derive(Debug)]
pub struct Ftyp {
    pub major_brand: Brand,
    pub minor_version: [u8; 4],
    pub compatible_brands: Vec<Brand>,
}

impl BaseBox for Ftyp {
    const BOX_TYPE: BoxType = BoxType::Ftyp;

    fn to_bytes(&self, writer: &mut BitstreamWriter) {
        for b in &self.major_brand.0 {
            writer.write_bits(*b as u32, 8);
        }

        for b in &self.minor_version {
            writer.write_bits(*b as u32, 8);
        }

        for brand in &self.compatible_brands {
            for b in &brand.0 {
                writer.write_bits(*b as u32, 8);
            }
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BitstreamReader::new(data);
        let major_brand = Brand([
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
        ]);

        let minor_version = [
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
            reader.read_bits(8)? as u8,
        ];

        let remaining = data.len() - 8;
        if !remaining.is_multiple_of(4) {
            return Err(Error::InvalidCompatibleBrandsLength(remaining));
        }

        let mut compatible_brands = Vec::with_capacity(remaining / 4);
        for _ in 0..(remaining / 4) {
            compatible_brands.push(Brand([
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
                reader.read_bits(8)? as u8,
            ]));
        }

        Ok(Self {
            major_brand,
            minor_version,
            compatible_brands,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ftyp_roundtrip() {
        let data = [
            b'f', b't', b'y', b'p', // major_brand: ftyp
            0x00, 0x00, 0x00, 0x01, // minor_version: 1
            b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', // compatible_brands: [ftyp, isom]
        ];
        let ftyp = Ftyp::parse(&data).expect("failed to parse ftyp");

        let mut writer = BitstreamWriter::new();
        ftyp.to_bytes(&mut writer);
        let output = writer.finish();

        assert_eq!(output, data);
    }
}
