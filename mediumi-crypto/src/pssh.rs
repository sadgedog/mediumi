#[derive(Debug)]
pub struct PsshInput {
    pub system_id: [u8; 16],
    pub key_ids: Vec<[u8; 16]>,
    pub data: Vec<u8>,
}

impl PsshInput {
    pub fn to_pssh(&self) -> PsshInput {
        todo!()
    }
}
