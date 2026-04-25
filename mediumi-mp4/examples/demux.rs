use mediumi_mp4::boxes::{BoxSize, Mp4Box, parse_all};

fn run(label: &str, path: &str) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[{}] failed to read {}: {}", label, path, e);
            return;
        }
    };

    println!(
        "{} {} ({}, {} bytes) {}",
        "-".repeat(20),
        label,
        path,
        data.len(),
        "-".repeat(20)
    );

    let boxes = match parse_all(&data) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[{}] parse error: {:?}", label, e);
            return;
        }
    };

    for (i, b) in boxes.iter().enumerate() {
        match b {
            Mp4Box::Ftyp(ftyp) => {
                let major = ftyp.major_brand.as_str();
                let compat: Vec<&str> = ftyp.compatible_brands.iter().map(|b| b.as_str()).collect();
                println!(
                    "[{}] type: 'ftyp', major: '{}', compatible: {:?}",
                    i, major, compat
                );
            }
            Mp4Box::Mdat(mdat) => {
                println!(
                    "[{}] type: 'mdat', payload: {} bytes",
                    i,
                    mdat.payload.len()
                );
            }
            Mp4Box::Mfhd(m) => {
                println!(
                    "[{}] type: 'mfhd', sequence_number: {}",
                    i, m.sequence_number
                );
            }
            Mp4Box::Moof(m) => {
                println!(
                    "[{}] type: 'moof', mfhd.seq: {}, trafs: {}, others: {}",
                    i,
                    m.mfhd.sequence_number,
                    m.trafs.len(),
                    m.others.len()
                );
            }
            Mp4Box::Traf(t) => {
                println!(
                    "[{}] type: 'traf', track_id: {}, truns: {}",
                    i,
                    t.tfhd.track_id,
                    t.truns.len()
                );
            }
            Mp4Box::Tfhd(t) => {
                println!("[{}] type: 'tfhd', track_id: {}", i, t.track_id);
            }
            Mp4Box::Tfdt(t) => {
                println!(
                    "[{}] type: 'tfdt', base_media_decode_time: {}",
                    i, t.base_media_decode_time
                );
            }
            Mp4Box::Trun(t) => {
                println!("[{}] type: 'trun', sample_count: {}", i, t.samples.len());
            }
            Mp4Box::Sbgp(s) => {
                println!(
                    "[{}] type: 'sbgp', grouping_type: {:#010x}, entries: {}",
                    i, s.grouping_type, s.entry_count
                );
            }
            Mp4Box::Sgpd(s) => {
                println!(
                    "[{}] type: 'sgpd', grouping_type: {:#010x}, entries: {}, bytes: {}",
                    i,
                    s.grouping_type,
                    s.entry_count,
                    s.entries.len()
                );
            }
            Mp4Box::Subs(s) => {
                let total_subsamples: usize = s.entries.iter().map(|e| e.subsamples.len()).sum();
                println!(
                    "[{}] type: 'subs', entries: {}, total_subsamples: {}",
                    i, s.entry_count, total_subsamples
                );
            }
            Mp4Box::Saiz(s) => {
                let aux = s
                    .aux_info_type
                    .map(|t| {
                        let bytes = t.to_be_bytes();
                        format!("'{}'", std::str::from_utf8(&bytes).unwrap_or("????"))
                    })
                    .unwrap_or_else(|| "none".to_string());
                println!(
                    "[{}] type: 'saiz', aux_info_type: {}, default_size: {}, sample_count: {}, per_sample: {}",
                    i,
                    aux,
                    s.default_sample_info_size,
                    s.sample_count,
                    s.sample_info_sizes.len()
                );
            }
            Mp4Box::Saio(s) => {
                let aux = s
                    .aux_info_type
                    .map(|t| {
                        let bytes = t.to_be_bytes();
                        format!("'{}'", std::str::from_utf8(&bytes).unwrap_or("????"))
                    })
                    .unwrap_or_else(|| "none".to_string());
                let width = if s.header.version == 0 { 32 } else { 64 };
                println!(
                    "[{}] type: 'saio', aux_info_type: {}, entry_count: {}, offset_width: {}-bit",
                    i, aux, s.entry_count, width
                );
            }
            Mp4Box::Meta(m) => {
                let ht = m.hdlr.handler_type.to_be_bytes();
                let ht_str = std::str::from_utf8(&ht).unwrap_or("????");
                println!(
                    "[{}] type: 'meta', hdlr.handler_type: '{}', hdlr.name: {:?}, others: {}",
                    i,
                    ht_str,
                    m.hdlr.name,
                    m.others.len()
                );
            }
            Mp4Box::Hdlr(h) => {
                let ht = h.handler_type.to_be_bytes();
                let ht_str = std::str::from_utf8(&ht).unwrap_or("????");
                println!(
                    "[{}] type: 'hdlr', handler_type: '{}', name: {:?}",
                    i, ht_str, h.name
                );
            }
            Mp4Box::Mvhd(m) => {
                let secs = if m.timescale > 0 {
                    m.duration as f64 / m.timescale as f64
                } else {
                    0.0
                };
                println!(
                    "[{}] type: 'mvhd', version: {}, timescale: {}, duration: {} ({:.3}s), next_track_id: {}",
                    i, m.header.version, m.timescale, m.duration, secs, m.next_track_id
                );
            }
            Mp4Box::Moov(m) => {
                let secs = if m.mvhd.timescale > 0 {
                    m.mvhd.duration as f64 / m.mvhd.timescale as f64
                } else {
                    0.0
                };
                println!(
                    "[{}] type: 'moov', mvhd.timescale: {}, mvhd.duration: {} ({:.3}s), meta: {}, others: {}",
                    i,
                    m.mvhd.timescale,
                    m.mvhd.duration,
                    secs,
                    m.meta.is_some(),
                    m.others.len()
                );
            }
            Mp4Box::Unknown(u) => {
                let size_str = match u.header.box_size {
                    BoxSize::Normal(s) => format!("{}", s),
                    BoxSize::Large(s) => format!("{} (large)", s),
                    BoxSize::ExtendsToEnd => "end".to_string(),
                };
                let type_bytes: [u8; 4] = (&u.header.box_type).into();
                let type_str = std::str::from_utf8(&type_bytes).unwrap_or("????");
                println!(
                    "[{}] type: '{}', box_size: {}, payload: {} bytes",
                    i,
                    type_str,
                    size_str,
                    u.payload.len()
                );
            }
        }
    }
}

fn main() {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data");
    run("MP4", &format!("{}/test.mp4", base));
    run("fMP4 init", &format!("{}/test_init.m4s", base));
    run("fMP4 segment", &format!("{}/test.m4s", base));
}
