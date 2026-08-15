use crate::{
    boxes::{BaseBox, FullBox, FullBoxHeader, error::Error},
    types::BoxType,
};
use mediumi_util::bytestream::{ByteReader, ByteWriter};

#[derive(Debug)]
pub struct Stsd {
    pub header: FullBoxHeader,
    pub entries: Vec<SampleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleEntry {
    pub box_type: [u8; 4],
    pub data_reference_index: u16,
    pub kind: SampleEntryKind,
    pub nested: Vec<NestedBox>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleEntryKind {
    Visual(VisualSampleEntry),
    Audio(AudioSampleEntry),
    /// Unsupported codecs
    Other {
        body: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualSampleEntry {
    pub pre_defined1: u16,
    pub width: u16,
    pub height: u16,
    pub horizresolution: u32,
    pub vertresolution: u32,
    pub frame_count: u16,
    pub compressorname: String,
    pub depth: u16,
    pub pre_defined2: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSampleEntry {
    pub channelcount: u16,
    pub samplesize: u16,
    pub pre_defined: u16,
    pub samplerate: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedBox {
    pub box_type: [u8; 4],
    pub body: Vec<u8>,
}

impl BaseBox for Stsd {
    const BOX_TYPE: BoxType = BoxType::Stsd;

    fn to_bytes(&self, writer: &mut ByteWriter) {
        self.header.to_bytes(writer);
        writer.write_bits(self.entries.len() as u32, 32);
        for e in &self.entries {
            e.to_bytes(writer);
        }
    }

    fn parse(data: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(data);
        let header = FullBoxHeader::parse(&mut reader)?;
        let entry_count = reader.read_bits(32)? as usize;

        let mut entries: Vec<SampleEntry> = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            // Box header: size (32 bits) + type (4 bytes)
            let size_field = reader.read_bits(32)?;
            let box_type: [u8; 4] = reader
                .read_slice(4)?
                .try_into()
                .map_err(|_| Error::DataTooShort)?;

            let body_len = match size_field {
                0 => reader.remaining_bits() / 8,
                1 => {
                    let high = reader.read_bits(32)? as u64;
                    let low = reader.read_bits(32)? as u64;
                    let total = ((high << 32) | low) as usize;
                    total.checked_sub(16).ok_or(Error::DataTooShort)?
                }
                _ => (size_field as usize)
                    .checked_sub(8)
                    .ok_or(Error::DataTooShort)?,
            };
            let body = reader.read_slice(body_len)?;
            entries.push(SampleEntry::parse(box_type, body)?);
        }
        Ok(Self { header, entries })
    }
}

impl FullBox for Stsd {
    fn version(&self) -> u8 {
        self.header.version
    }
    fn flags(&self) -> u32 {
        self.header.flags
    }
}

impl SampleEntry {
    fn to_bytes(&self, writer: &mut ByteWriter) {
        // 8(box header) + 8(SampleEntry common) + ...
        let total_size =
            8 + 8 + self.kind_size() + self.nested.iter().map(|n| 8 + n.body.len()).sum::<usize>();

        writer.write_bits(total_size as u32, 32);
        for &b in &self.box_type {
            writer.write_bits(b as u32, 8);
        }
        // SampleEntry common
        for _ in 0..6 {
            writer.write_bits(0, 8);
        }
        writer.write_bits(self.data_reference_index as u32, 16);

        match &self.kind {
            SampleEntryKind::Visual(v) => v.to_bytes(writer),
            SampleEntryKind::Audio(a) => a.to_bytes(writer),
            SampleEntryKind::Other { body } => {
                for &b in body {
                    writer.write_bits(b as u32, 8);
                }
            }
        }

        for n in &self.nested {
            let nbox_size = 8 + n.body.len();
            writer.write_bits(nbox_size as u32, 32);
            for &b in &n.box_type {
                writer.write_bits(b as u32, 8);
            }
            for &b in &n.body {
                writer.write_bits(b as u32, 8);
            }
        }
    }

    fn parse(box_type: [u8; 4], body: &[u8]) -> Result<Self, Error> {
        let mut reader = ByteReader::new(body);
        // SampleEntry common: 6 reserved + 2 data_reference_index.
        let _reserved = reader.read_slice(6)?;
        let data_reference_index = reader.read_bits(16)? as u16;

        match &box_type {
            b"avc1" | b"avc3" | b"hev1" | b"hvc1" | b"av01" | b"vp08" | b"vp09" | b"mp4v"
            | b"encv" => {
                let visual = VisualSampleEntry::parse(&mut reader)?;
                let nested = parse_nested_boxes(&mut reader)?;
                Ok(Self {
                    box_type,
                    data_reference_index,
                    kind: SampleEntryKind::Visual(visual),
                    nested,
                })
            }
            b"mp4a" | b"ac-3" | b"ec-3" | b"opus" | b"alac" | b"flaC" | b"enca" => {
                let audio = AudioSampleEntry::parse(&mut reader)?;
                let nested = parse_nested_boxes(&mut reader)?;
                Ok(Self {
                    box_type,
                    data_reference_index,
                    kind: SampleEntryKind::Audio(audio),
                    nested,
                })
            }
            _ => {
                // Unsupported codec family — keep the post-common body opaque.
                let (rest, _) = reader.read_remaining_bytes();
                Ok(Self {
                    box_type,
                    data_reference_index,
                    kind: SampleEntryKind::Other { body: rest },
                    nested: Vec::new(),
                })
            }
        }
    }

    fn kind_size(&self) -> usize {
        match &self.kind {
            SampleEntryKind::Visual(_) => 70,
            SampleEntryKind::Audio(_) => 20,
            SampleEntryKind::Other { body } => body.len(),
        }
    }
}

fn parse_nested_boxes(reader: &mut ByteReader) -> Result<Vec<NestedBox>, Error> {
    let mut nested = Vec::new();
    while reader.remaining_bits() >= 64 {
        // box header: size (32) + type (4)
        let size = reader.read_bits(32)? as usize;
        let box_type: [u8; 4] = reader
            .read_slice(4)?
            .try_into()
            .map_err(|_| Error::DataTooShort)?;
        if size < 8 {
            return Err(Error::DataTooShort);
        }
        let body = reader.read_slice(size - 8)?.to_vec();
        nested.push(NestedBox { box_type, body });
    }
    if reader.remaining_bits() != 0 {
        return Err(Error::DataTooShort);
    }
    Ok(nested)
}

impl VisualSampleEntry {
    fn parse(reader: &mut ByteReader) -> Result<Self, Error> {
        let pre_defined1 = reader.read_bits(16)? as u16;
        let _reserved = reader.read_bits(16)?; // reserved (= 0)
        for _ in 0..3 {
            let _ = reader.read_bits(32)?; // pre_defined[3] (= 0)
        }
        let width = reader.read_bits(16)? as u16;
        let height = reader.read_bits(16)? as u16;
        let horizresolution = reader.read_bits(32)?;
        let vertresolution = reader.read_bits(32)?;
        let _reserved = reader.read_bits(32)?; // reserved (= 0)
        let frame_count = reader.read_bits(16)? as u16;

        // compressorname: 32-byte Pascal string (1 byte length + 31 byte content padded with 0).
        let cn = reader.read_slice(32)?;
        let len = (cn[0] as usize).min(31);
        let compressorname = String::from_utf8_lossy(&cn[1..1 + len]).into_owned();

        let depth = reader.read_bits(16)? as u16;
        let pre_defined2 = reader.read_bits(16)? as i16;
        Ok(Self {
            pre_defined1,
            width,
            height,
            horizresolution,
            vertresolution,
            frame_count,
            compressorname,
            depth,
            pre_defined2,
        })
    }

    fn to_bytes(&self, writer: &mut ByteWriter) {
        writer.write_bits(self.pre_defined1 as u32, 16);
        writer.write_bits(0, 16); // reserved
        for _ in 0..3 {
            writer.write_bits(0, 32); // pre_defined[3]
        }
        writer.write_bits(self.width as u32, 16);
        writer.write_bits(self.height as u32, 16);
        writer.write_bits(self.horizresolution, 32);
        writer.write_bits(self.vertresolution, 32);
        writer.write_bits(0, 32); // reserved
        writer.write_bits(self.frame_count as u32, 16);

        let cn = self.compressorname.as_bytes();
        let len = cn.len().min(31);
        let mut buf = [0u8; 32];
        buf[0] = len as u8;
        buf[1..1 + len].copy_from_slice(&cn[..len]);
        for byte in buf {
            writer.write_bits(byte as u32, 8);
        }

        writer.write_bits(self.depth as u32, 16);
        writer.write_bits((self.pre_defined2 as u16) as u32, 16);
    }
}

impl AudioSampleEntry {
    fn parse(reader: &mut ByteReader) -> Result<Self, Error> {
        for _ in 0..2 {
            let _ = reader.read_bits(32)?; // reserved × 2 (= 0)
        }
        let channelcount = reader.read_bits(16)? as u16;
        let samplesize = reader.read_bits(16)? as u16;
        let pre_defined = reader.read_bits(16)? as u16;
        let _reserved = reader.read_bits(16)?; // reserved (= 0)
        let samplerate = reader.read_bits(32)?;
        Ok(Self {
            channelcount,
            samplesize,
            pre_defined,
            samplerate,
        })
    }

    fn to_bytes(&self, writer: &mut ByteWriter) {
        for _ in 0..2 {
            writer.write_bits(0, 32); // reserved × 2
        }
        writer.write_bits(self.channelcount as u32, 16);
        writer.write_bits(self.samplesize as u32, 16);
        writer.write_bits(self.pre_defined as u32, 16);
        writer.write_bits(0, 16); // reserved
        writer.write_bits(self.samplerate, 32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_bytes(box_type: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let size = 8 + body.len();
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(&(size as u32).to_be_bytes());
        out.extend_from_slice(box_type);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn other_entry_roundtrip() {
        // Unknown sample entry: SampleEntry common (8) + opaque body.
        let mut body = Vec::new();
        body.extend_from_slice(&[0u8; 6]); // reserved
        body.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
        body.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // opaque

        let entry_bytes = box_bytes(b"unkn", &body);
        let stsd_body = build_stsd_body(std::slice::from_ref(&entry_bytes));
        let parsed = Stsd::parse(&stsd_body).expect("parse");
        assert_eq!(parsed.entries.len(), 1);

        let entry = &parsed.entries[0];
        assert_eq!(&entry.box_type, b"unkn");
        assert_eq!(entry.data_reference_index, 1);
        match &entry.kind {
            SampleEntryKind::Other { body } => assert_eq!(body, &[0xDE, 0xAD, 0xBE, 0xEF]),
            _ => panic!("expected Other"),
        }
        assert!(entry.nested.is_empty());

        let mut w = ByteWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), stsd_body);
    }

    #[test]
    fn avc1_entry_roundtrip() {
        // avc1 sample entry with VisualSampleEntry header + 1 nested avcC box.
        let mut visual = Vec::new();
        visual.extend_from_slice(&[0u8; 6]); // SampleEntry reserved
        visual.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index = 1

        // VisualSampleEntry (70 bytes)
        visual.extend_from_slice(&0u16.to_be_bytes()); // pre_defined1
        visual.extend_from_slice(&0u16.to_be_bytes()); // reserved
        for _ in 0..3 {
            visual.extend_from_slice(&0u32.to_be_bytes()); // pre_defined[3]
        }
        visual.extend_from_slice(&1280u16.to_be_bytes()); // width
        visual.extend_from_slice(&720u16.to_be_bytes()); // height
        visual.extend_from_slice(&0x00480000u32.to_be_bytes()); // horizres = 72 dpi
        visual.extend_from_slice(&0x00480000u32.to_be_bytes()); // vertres
        visual.extend_from_slice(&0u32.to_be_bytes()); // reserved
        visual.extend_from_slice(&1u16.to_be_bytes()); // frame_count = 1
        // compressorname: length=4 "h264" + 27 zeros
        visual.push(4u8);
        visual.extend_from_slice(b"h264");
        visual.extend(std::iter::repeat_n(0u8, 27));
        visual.extend_from_slice(&0x0018u16.to_be_bytes()); // depth = 24
        visual.extend_from_slice(&(-1i16).to_be_bytes()); // pre_defined2 = -1

        // 1 nested avcC box (8 byte header + 4 byte body)
        visual.extend_from_slice(&12u32.to_be_bytes());
        visual.extend_from_slice(b"avcC");
        visual.extend_from_slice(&[0x01, 0x42, 0xC0, 0x1E]);

        let entry_bytes = box_bytes(b"avc1", &visual);
        let stsd_body = build_stsd_body(std::slice::from_ref(&entry_bytes));
        let parsed = Stsd::parse(&stsd_body).expect("parse");

        let entry = &parsed.entries[0];
        assert_eq!(&entry.box_type, b"avc1");
        assert_eq!(entry.data_reference_index, 1);
        match &entry.kind {
            SampleEntryKind::Visual(v) => {
                assert_eq!(v.width, 1280);
                assert_eq!(v.height, 720);
                assert_eq!(v.horizresolution, 0x00480000);
                assert_eq!(v.frame_count, 1);
                assert_eq!(v.compressorname, "h264");
                assert_eq!(v.depth, 0x0018);
                assert_eq!(v.pre_defined2, -1);
            }
            _ => panic!("expected Visual"),
        }
        assert_eq!(entry.nested.len(), 1);
        assert_eq!(&entry.nested[0].box_type, b"avcC");
        assert_eq!(entry.nested[0].body, vec![0x01, 0x42, 0xC0, 0x1E]);

        let mut w = ByteWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), stsd_body);
    }

    #[test]
    fn mp4a_entry_roundtrip() {
        // mp4a sample entry with AudioSampleEntry header + 1 nested esds box.
        let mut audio = Vec::new();
        audio.extend_from_slice(&[0u8; 6]); // reserved
        audio.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index

        // AudioSampleEntry (20 bytes)
        for _ in 0..2 {
            audio.extend_from_slice(&0u32.to_be_bytes()); // reserved[2]
        }
        audio.extend_from_slice(&2u16.to_be_bytes()); // channelcount = 2
        audio.extend_from_slice(&16u16.to_be_bytes()); // samplesize = 16
        audio.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
        audio.extend_from_slice(&0u16.to_be_bytes()); // reserved
        audio.extend_from_slice(&((48000u32) << 16).to_be_bytes()); // samplerate (16.16)

        // 1 nested esds box
        audio.extend_from_slice(&10u32.to_be_bytes());
        audio.extend_from_slice(b"esds");
        audio.extend_from_slice(&[0x42, 0x00]);

        let entry_bytes = box_bytes(b"mp4a", &audio);
        let stsd_body = build_stsd_body(&[entry_bytes]);
        let parsed = Stsd::parse(&stsd_body).expect("parse");

        let entry = &parsed.entries[0];
        assert_eq!(&entry.box_type, b"mp4a");
        match &entry.kind {
            SampleEntryKind::Audio(a) => {
                assert_eq!(a.channelcount, 2);
                assert_eq!(a.samplesize, 16);
                assert_eq!(a.samplerate, 48000u32 << 16);
            }
            _ => panic!("expected Audio"),
        }
        assert_eq!(entry.nested.len(), 1);
        assert_eq!(&entry.nested[0].box_type, b"esds");

        let mut w = ByteWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), stsd_body);
    }

    #[test]
    fn encv_entry_with_sinf_roundtrip() {
        use crate::boxes::frma::Frma;
        use crate::boxes::schi::Schi;
        use crate::boxes::schm::Schm;
        use crate::boxes::sinf::Sinf;
        use crate::boxes::tenc::Tenc;

        // Build a typed sinf and serialize it to a full box (size + 'sinf' + body).
        let sinf = Sinf {
            frma: Frma {
                data_format: *b"avc1",
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
        };
        let mut sinf_w = ByteWriter::new();
        sinf.write_box(&mut sinf_w);
        let sinf_box = sinf_w.finish();

        // encv body = VisualSampleEntry header + nested avcC + nested sinf.
        let mut visual = Vec::new();
        visual.extend_from_slice(&[0u8; 6]); // SampleEntry reserved
        visual.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
        visual.extend_from_slice(&0u16.to_be_bytes()); // pre_defined1
        visual.extend_from_slice(&0u16.to_be_bytes()); // reserved
        for _ in 0..3 {
            visual.extend_from_slice(&0u32.to_be_bytes());
        }
        visual.extend_from_slice(&1920u16.to_be_bytes()); // width
        visual.extend_from_slice(&1080u16.to_be_bytes()); // height
        visual.extend_from_slice(&0x00480000u32.to_be_bytes());
        visual.extend_from_slice(&0x00480000u32.to_be_bytes());
        visual.extend_from_slice(&0u32.to_be_bytes());
        visual.extend_from_slice(&1u16.to_be_bytes()); // frame_count
        visual.push(0u8); // compressorname length = 0
        visual.extend(std::iter::repeat_n(0u8, 31));
        visual.extend_from_slice(&0x0018u16.to_be_bytes()); // depth
        visual.extend_from_slice(&(-1i16).to_be_bytes()); // pre_defined2
        // nested avcC
        visual.extend_from_slice(&12u32.to_be_bytes());
        visual.extend_from_slice(b"avcC");
        visual.extend_from_slice(&[0x01, 0x42, 0xC0, 0x1E]);
        // nested sinf
        visual.extend_from_slice(&sinf_box);

        let entry_bytes = box_bytes(b"encv", &visual);
        let stsd_body = build_stsd_body(std::slice::from_ref(&entry_bytes));
        let parsed = Stsd::parse(&stsd_body).expect("parse");

        let entry = &parsed.entries[0];
        assert_eq!(&entry.box_type, b"encv");
        // encv body is parsed as a visual sample entry
        match &entry.kind {
            SampleEntryKind::Visual(v) => {
                assert_eq!(v.width, 1920);
                assert_eq!(v.height, 1080);
            }
            _ => panic!("expected Visual"),
        }
        // nested: avcC + sinf (both kept as raw NestedBox)
        assert_eq!(entry.nested.len(), 2);
        assert_eq!(&entry.nested[0].box_type, b"avcC");
        assert_eq!(&entry.nested[1].box_type, b"sinf");

        // the nested sinf body parses back to the typed Sinf (avcC-style explicit parse)
        let parsed_sinf = Sinf::parse(&entry.nested[1].body).expect("parse nested sinf");
        assert_eq!(parsed_sinf, sinf);

        // stsd byte-exact roundtrip
        let mut w = ByteWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), stsd_body);
    }

    #[test]
    fn enca_entry_with_sinf_roundtrip() {
        use crate::boxes::frma::Frma;
        use crate::boxes::schi::Schi;
        use crate::boxes::schm::Schm;
        use crate::boxes::sinf::Sinf;
        use crate::boxes::tenc::Tenc;

        // Build a typed sinf (frma = original mp4a) and serialize to a full box.
        let sinf = Sinf {
            frma: Frma {
                data_format: *b"mp4a",
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
        };
        let mut sinf_w = ByteWriter::new();
        sinf.write_box(&mut sinf_w);
        let sinf_box = sinf_w.finish();

        // enca body = AudioSampleEntry header + nested esds + nested sinf.
        let mut audio = Vec::new();
        audio.extend_from_slice(&[0u8; 6]); // SampleEntry reserved
        audio.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
        for _ in 0..2 {
            audio.extend_from_slice(&0u32.to_be_bytes()); // reserved[2]
        }
        audio.extend_from_slice(&2u16.to_be_bytes()); // channelcount
        audio.extend_from_slice(&16u16.to_be_bytes()); // samplesize
        audio.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
        audio.extend_from_slice(&0u16.to_be_bytes()); // reserved
        audio.extend_from_slice(&((48000u32) << 16).to_be_bytes()); // samplerate
        // nested esds
        audio.extend_from_slice(&10u32.to_be_bytes());
        audio.extend_from_slice(b"esds");
        audio.extend_from_slice(&[0x42, 0x00]);
        // nested sinf
        audio.extend_from_slice(&sinf_box);

        let entry_bytes = box_bytes(b"enca", &audio);
        let stsd_body = build_stsd_body(std::slice::from_ref(&entry_bytes));
        let parsed = Stsd::parse(&stsd_body).expect("parse");

        let entry = &parsed.entries[0];
        assert_eq!(&entry.box_type, b"enca");
        // enca body is parsed as an audio sample entry
        match &entry.kind {
            SampleEntryKind::Audio(a) => {
                assert_eq!(a.channelcount, 2);
                assert_eq!(a.samplerate, 48000u32 << 16);
            }
            _ => panic!("expected Audio"),
        }
        // nested: esds + sinf
        assert_eq!(entry.nested.len(), 2);
        assert_eq!(&entry.nested[0].box_type, b"esds");
        assert_eq!(&entry.nested[1].box_type, b"sinf");

        // the nested sinf body parses back to the typed Sinf
        let parsed_sinf = Sinf::parse(&entry.nested[1].body).expect("parse nested sinf");
        assert_eq!(parsed_sinf, sinf);

        // stsd byte-exact roundtrip
        let mut w = ByteWriter::new();
        parsed.to_bytes(&mut w);
        assert_eq!(w.finish(), stsd_body);
    }

    fn build_stsd_body(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        // FullBoxHeader (version=0, flags=0)
        out.extend_from_slice(&0u32.to_be_bytes());
        // entry_count
        out.extend_from_slice(&(entries.len() as u32).to_be_bytes());
        for e in entries {
            out.extend_from_slice(e);
        }
        out
    }
}
