use crate::boxes::BaseBox;
use crate::boxes::sinf::Sinf;
use crate::boxes::stsd::{NestedBox, SampleEntry};
use crate::boxes::trak::Trak;
use crate::util::bitstream::BitstreamWriter;

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

/// Apply the "protected sample entry" transformation
///
/// ```text
/// moov
/// └── trak
///     └── mdia → minf → stbl
///         └── stsd
///             └── SampleEntry <- rewrite this fields
///                 ├── box_type (avc1 -> encv)
///                 └── nested
///                     ├── avcC (retain)
///                     └── sinf (created by caller)
/// ```
pub fn wrap_with_sinf(entry: &mut SampleEntry, sinf: &Sinf) -> Option<[u8; 4]> {
    let box_type: [u8; 4] = match &entry.box_type {
        b"avc1" | b"avc3" | b"hev1" | b"hvc1" | b"av01" | b"vp08" | b"vp09" | b"mp4v" => *b"encv",
        b"mp4a" | b"ac-3" | b"ec-3" | b"opus" | b"alac" | b"flaC" => *b"enca",
        _ => return None,
    };
    entry.box_type = box_type;

    // BaseBox::to_bytes writes the box BODY only (no size/box_type header).
    // The stsd serialiser wraps each NestedBox with its own size+type prefix.
    let mut w = BitstreamWriter::new();
    sinf.to_bytes(&mut w);
    entry.nested.push(NestedBox {
        box_type: *b"sinf",
        body: w.finish(),
    });
    Some(box_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::FullBoxHeader;
    use crate::boxes::frma::Frma;
    use crate::boxes::schi::Schi;
    use crate::boxes::schm::Schm;
    use crate::boxes::stsd::{AudioSampleEntry, SampleEntryKind, VisualSampleEntry};
    use crate::boxes::tenc::Tenc;

    fn sample_sinf(original_box_type: [u8; 4]) -> Sinf {
        Sinf {
            frma: Frma {
                data_format: original_box_type,
            },
            schm: Schm {
                header: FullBoxHeader {
                    version: 0,
                    flags: 0,
                },
                scheme_type: *b"cenc",
                scheme_version: 0x0001_0000,
                scheme_uri: None,
            },
            schi: Some(Schi {
                tenc: Some(Tenc {
                    header: FullBoxHeader {
                        version: 0,
                        flags: 0,
                    },
                    default_crypt_byte_block: 0,
                    default_skip_byte_block: 0,
                    default_is_protected: 1,
                    default_per_sample_iv_size: 8,
                    default_kid: [0x42; 16],
                    default_constant_iv: None,
                }),
                others: Vec::new(),
            }),
            others: Vec::new(),
        }
    }

    fn avc1_entry() -> SampleEntry {
        SampleEntry {
            box_type: *b"avc1",
            data_reference_index: 1,
            kind: SampleEntryKind::Visual(VisualSampleEntry {
                pre_defined1: 0,
                width: 1920,
                height: 1080,
                horizresolution: 0x0048_0000,
                vertresolution: 0x0048_0000,
                frame_count: 1,
                compressorname: String::new(),
                depth: 24,
                pre_defined2: -1,
            }),
            nested: vec![NestedBox {
                box_type: *b"avcC",
                body: vec![0x01, 0x42, 0xC0, 0x1E],
            }],
        }
    }

    fn mp4a_entry() -> SampleEntry {
        SampleEntry {
            box_type: *b"mp4a",
            data_reference_index: 1,
            kind: SampleEntryKind::Audio(AudioSampleEntry {
                channelcount: 2,
                samplesize: 16,
                pre_defined: 0,
                samplerate: 48000u32 << 16,
            }),
            nested: vec![NestedBox {
                box_type: *b"esds",
                body: vec![0x00, 0x00, 0x00, 0x00],
            }],
        }
    }

    #[test]
    fn wrap_avc1_renames_to_encv_and_appends_sinf() {
        let mut entry = avc1_entry();
        let result = wrap_with_sinf(&mut entry, &sample_sinf(*b"avc1"));
        assert_eq!(result, Some(*b"encv"));
        assert_eq!(entry.box_type, *b"encv");
        // avcC kept + sinf appended
        assert_eq!(entry.nested.len(), 2);
        assert_eq!(&entry.nested[0].box_type, b"avcC");
        assert_eq!(&entry.nested[1].box_type, b"sinf");
        // sinf body is parseable back to the typed Sinf (avcC-style explicit parse)
        let parsed = Sinf::parse(&entry.nested[1].body).expect("parse nested sinf");
        assert_eq!(parsed.frma.data_format, *b"avc1");
        assert_eq!(parsed.schm.scheme_type, *b"cenc");
    }

    #[test]
    fn wrap_mp4a_renames_to_enca() {
        let mut entry = mp4a_entry();
        let result = wrap_with_sinf(&mut entry, &sample_sinf(*b"mp4a"));
        assert_eq!(result, Some(*b"enca"));
        assert_eq!(entry.box_type, *b"enca");
        assert_eq!(entry.nested.len(), 2);
        assert_eq!(&entry.nested[0].box_type, b"esds");
        assert_eq!(&entry.nested[1].box_type, b"sinf");
    }

    #[test]
    fn wrap_unsupported_codec_returns_none_and_leaves_entry_untouched() {
        let mut entry = SampleEntry {
            box_type: *b"xxxx",
            data_reference_index: 1,
            kind: SampleEntryKind::Other { body: Vec::new() },
            nested: Vec::new(),
        };
        let result = wrap_with_sinf(&mut entry, &sample_sinf(*b"xxxx"));
        assert_eq!(result, None);
        assert_eq!(entry.box_type, *b"xxxx");
        assert!(entry.nested.is_empty());
    }
}
