use crate::cenc::Subsample;
use crate::error::Error;

pub fn plan(_sample: &[u8]) -> Result<Vec<Subsample>, Error> {
    Ok(Vec::new())
}
