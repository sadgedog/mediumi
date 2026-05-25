use mediumi_mp4::boxes::{FullBoxHeader, pssh::Pssh};

#[derive(Debug, PartialEq)]
pub struct PsshInput {
    pub system_id: [u8; 16],
    pub key_ids: Vec<[u8; 16]>,
    pub data: Vec<u8>,
}

impl PsshInput {
    pub fn to_pssh(&self) -> Pssh {
        let version = if self.key_ids.is_empty() { 0 } else { 1 };
        Pssh {
            header: FullBoxHeader { version, flags: 0 }, // flags is fixed at 0
            system_id: self.system_id,
            key_ids: self.key_ids.clone(),
            data: self.data.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM_ID: [u8; 16] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
        0x0F,
    ];

    #[test]
    fn parse_pssh_input() {
        let input = PsshInput {
            system_id: SYSTEM_ID,
            key_ids: vec![[0x00_u8; 16]],
            data: vec![],
        };
        let parsed = input.to_pssh();
        let pssh = Pssh {
            header: FullBoxHeader {
                version: 1,
                flags: 0,
            },
            system_id: SYSTEM_ID,
            key_ids: vec![[0x00_u8; 16]],
            data: vec![],
        };

        assert_eq!(parsed, pssh);
    }
}
