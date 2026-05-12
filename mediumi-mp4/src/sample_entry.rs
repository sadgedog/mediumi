use crate::boxes::stsd::SampleEntry;
use crate::boxes::trak::Trak;

/// Walk `trak.mdia.minf.stbl.stsd.entries` and return the body bytes of the first
/// nested box whose 4-byte type matches `config_fourcc`.
pub fn find_codec_config<'a>(trak: &'a Trak, config_fourcc: &[u8; 4]) -> Option<&'a [u8]> {
    let stbl = trak.mdia.minf.stbl.as_ref()?;
    for entry in &stbl.stsd.entries {
        if let Some(bytes) = find_in_entry(entry, config_fourcc) {
            return Some(bytes);
        }
    }
    None
}

fn find_in_entry<'a>(entry: &'a SampleEntry, target: &[u8; 4]) -> Option<&'a [u8]> {
    entry
        .nested
        .iter()
        .find(|n| &n.box_type == target)
        .map(|n| n.body.as_slice())
}
