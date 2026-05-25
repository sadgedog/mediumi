use mediumi_mp4::{BoxSize, Mp4Box, demuxer};

fn fourcc(bytes: &[u8; 4]) -> String {
    std::str::from_utf8(bytes).unwrap_or("????").to_string()
}

fn fourcc_u32(v: u32) -> String {
    fourcc(&v.to_be_bytes())
}

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

    let boxes = match demuxer::demux(&data) {
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
            Mp4Box::Moov(m) => {
                let secs = if m.mvhd.timescale > 0 {
                    m.mvhd.duration as f64 / m.mvhd.timescale as f64
                } else {
                    0.0
                };
                println!(
                    "[{}] type: 'moov', mvhd.timescale: {}, mvhd.duration: {} ({:.3}s), traks: {}, mvex: {}, meta: {}, udta: {}, others: {}",
                    i,
                    m.mvhd.timescale,
                    m.mvhd.duration,
                    secs,
                    m.traks.len(),
                    m.mvex.is_some(),
                    m.meta.is_some(),
                    m.udta.is_some(),
                    m.others.len(),
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
            Mp4Box::Trak(t) => {
                println!(
                    "[{}] type: 'trak', track_id: {}, edts: {}, tref: {}, trgr: {}, meta: {}, udta: {}",
                    i,
                    t.tkhd.track_id,
                    t.edts.is_some(),
                    t.tref.is_some(),
                    t.trgr.is_some(),
                    t.meta.is_some(),
                    t.udta.is_some(),
                );
            }
            Mp4Box::Tkhd(t) => {
                println!(
                    "[{}] type: 'tkhd', track_id: {}, duration: {}, layer: {}, alt_group: {}, volume: 0x{:04x}, dimensions: {}x{} (16.16 fixed)",
                    i,
                    t.track_id,
                    t.duration,
                    t.layer,
                    t.alternate_group,
                    t.volume as u16,
                    t.width >> 16,
                    t.height >> 16,
                );
            }
            Mp4Box::Tref(r) => {
                let kinds: Vec<String> = r
                    .references
                    .iter()
                    .map(|x| fourcc(&x.reference_type))
                    .collect();
                println!(
                    "[{}] type: 'tref', references: {} {:?}",
                    i,
                    r.references.len(),
                    kinds
                );
            }
            Mp4Box::Trgr(r) => {
                let kinds: Vec<String> = r
                    .groups
                    .iter()
                    .map(|x| fourcc(&x.track_group_type))
                    .collect();
                println!(
                    "[{}] type: 'trgr', groups: {} {:?}",
                    i,
                    r.groups.len(),
                    kinds
                );
            }
            Mp4Box::Edts(e) => {
                println!(
                    "[{}] type: 'edts', elst.entries: {}, others: {}",
                    i,
                    e.elst.as_ref().map(|x| x.entries.len()).unwrap_or(0),
                    e.others.len(),
                );
            }
            Mp4Box::Elst(e) => {
                println!(
                    "[{}] type: 'elst', version: {}, entries: {}",
                    i,
                    e.header.version,
                    e.entries.len()
                );
            }
            Mp4Box::Mdia(m) => {
                let ht = fourcc_u32(m.hdlr.handler_type);
                println!(
                    "[{}] type: 'mdia', mdhd.timescale: {}, mdhd.duration: {}, hdlr: '{}', elng: {}",
                    i,
                    m.mdhd.timescale,
                    m.mdhd.duration,
                    ht,
                    m.elng.is_some(),
                );
            }
            Mp4Box::Mdhd(m) => {
                let secs = if m.timescale > 0 {
                    m.duration as f64 / m.timescale as f64
                } else {
                    0.0
                };
                println!(
                    "[{}] type: 'mdhd', version: {}, timescale: {}, duration: {} ({:.3}s), language: 0x{:04x}",
                    i, m.header.version, m.timescale, m.duration, secs, m.language
                );
            }
            Mp4Box::Hdlr(h) => {
                println!(
                    "[{}] type: 'hdlr', handler_type: '{}', name: {:?}",
                    i,
                    fourcc_u32(h.handler_type),
                    h.name
                );
            }
            Mp4Box::Elng(e) => {
                println!(
                    "[{}] type: 'elng', extended_language: {:?}",
                    i, e.extended_language
                );
            }
            Mp4Box::Minf(m) => {
                let mh = if m.vmhd.is_some() {
                    "vmhd"
                } else if m.smhd.is_some() {
                    "smhd"
                } else if m.hmhd.is_some() {
                    "hmhd"
                } else if m.sthd.is_some() {
                    "sthd"
                } else if m.nmhd.is_some() {
                    "nmhd"
                } else {
                    "none"
                };
                println!(
                    "[{}] type: 'minf', media_header: {}, dinf: {}, stbl: {}",
                    i,
                    mh,
                    m.dinf.is_some(),
                    m.stbl.is_some(),
                );
            }
            Mp4Box::Vmhd(v) => {
                println!(
                    "[{}] type: 'vmhd', graphicsmode: {}, opcolor: {:?}",
                    i, v.graphicsmode, v.opcolor
                );
            }
            Mp4Box::Smhd(s) => {
                println!("[{}] type: 'smhd', balance: {}", i, s.balance);
            }
            Mp4Box::Hmhd(h) => {
                println!(
                    "[{}] type: 'hmhd', max_pdu: {}, avg_pdu: {}, max_bitrate: {}, avg_bitrate: {}",
                    i, h.max_pdu_size, h.avg_pdu_size, h.max_bitrate, h.avg_bitrate
                );
            }
            Mp4Box::Sthd(_) => {
                println!("[{}] type: 'sthd' (subtitle media header, no body)", i);
            }
            Mp4Box::Nmhd(_) => {
                println!("[{}] type: 'nmhd' (null media header, no body)", i);
            }
            Mp4Box::Dinf(d) => {
                println!(
                    "[{}] type: 'dinf', dref.entries: {}, others: {}",
                    i,
                    d.dref.entries.len(),
                    d.others.len()
                );
            }
            Mp4Box::Dref(d) => {
                println!("[{}] type: 'dref', entries: {}", i, d.entries.len());
            }
            Mp4Box::Stbl(s) => {
                println!(
                    "[{}] type: 'stbl', stsd.entries: {}, stts.entries: {}, stsc.entries: {}, stco/co64: {}/{}, stsz/stz2: {}/{}, optional: ctts={} cslg={} stss={} stsh={} padb={} stdp={} sdtp={}, sbgp={} sgpd={} subs={} saiz={} saio={}",
                    i,
                    s.stsd.entries.len(),
                    s.stts.entries.len(),
                    s.stsc.entries.len(),
                    s.stco.chunk_offsets.len(),
                    s.co64.as_ref().map(|x| x.chunk_offsets.len()).unwrap_or(0),
                    s.stsz.as_ref().map(|x| x.entry_sizes.len()).unwrap_or(0),
                    s.stz2.as_ref().map(|x| x.entry_sizes.len()).unwrap_or(0),
                    s.ctts.is_some(),
                    s.cslg.is_some(),
                    s.stss.is_some(),
                    s.stsh.is_some(),
                    s.padb.is_some(),
                    s.stdp.is_some(),
                    s.sdtp.is_some(),
                    s.sbgps.len(),
                    s.sgpds.len(),
                    s.subs.len(),
                    s.saizs.len(),
                    s.saios.len(),
                );
            }
            Mp4Box::Stsd(s) => {
                println!("[{}] type: 'stsd', entries: {}", i, s.entries.len());
            }
            Mp4Box::Stts(s) => {
                let total: u64 = s.entries.iter().map(|e| e.sample_count as u64).sum();
                println!(
                    "[{}] type: 'stts', entries: {}, total_samples: {}",
                    i,
                    s.entries.len(),
                    total
                );
            }
            Mp4Box::Ctts(c) => {
                println!("[{}] type: 'ctts', entries: {}", i, c.entries.len());
            }
            Mp4Box::Cslg(c) => {
                println!(
                    "[{}] type: 'cslg', dts_shift: {}, [{}, {}], composition: [{}, {}]",
                    i,
                    c.composition_to_dts_shift,
                    c.least_decode_to_display_delta,
                    c.greatest_decode_to_display_delta,
                    c.composition_start_time,
                    c.composition_end_time,
                );
            }
            Mp4Box::Stsc(s) => {
                println!("[{}] type: 'stsc', entries: {}", i, s.entries.len());
            }
            Mp4Box::Stsz(s) => {
                println!(
                    "[{}] type: 'stsz', sample_size: {}, sample_count: {}, entry_sizes: {}",
                    i,
                    s.sample_size,
                    s.sample_count,
                    s.entry_sizes.len()
                );
            }
            Mp4Box::Stz2(s) => {
                println!(
                    "[{}] type: 'stz2', field_size: {}, sample_count: {}",
                    i, s.field_size, s.sample_count
                );
            }
            Mp4Box::Stco(s) => {
                println!(
                    "[{}] type: 'stco', chunk_offsets: {}",
                    i,
                    s.chunk_offsets.len()
                );
            }
            Mp4Box::Co64(s) => {
                println!(
                    "[{}] type: 'co64', chunk_offsets: {}",
                    i,
                    s.chunk_offsets.len()
                );
            }
            Mp4Box::Stss(s) => {
                println!(
                    "[{}] type: 'stss', sync_samples: {}",
                    i,
                    s.sample_numbers.len()
                );
            }
            Mp4Box::Stsh(s) => {
                println!("[{}] type: 'stsh', entries: {}", i, s.entries.len());
            }
            Mp4Box::Padb(p) => {
                println!(
                    "[{}] type: 'padb', sample_count: {}, padding_bits: {}",
                    i,
                    p.sample_count,
                    p.padding_bits.len()
                );
            }
            Mp4Box::Stdp(s) => {
                println!("[{}] type: 'stdp', priorities: {}", i, s.priorities.len());
            }
            Mp4Box::Sdtp(s) => {
                println!("[{}] type: 'sdtp', entries: {}", i, s.entries.len());
            }
            Mp4Box::Udta(u) => {
                println!("[{}] type: 'udta', children (raw): {}", i, u.others.len());
            }
            Mp4Box::Mvex(m) => {
                println!(
                    "[{}] type: 'mvex', mehd: {}, trexs: {}, leva: {}, others: {}",
                    i,
                    m.mehd.is_some(),
                    m.trexs.len(),
                    m.leva.is_some(),
                    m.others.len()
                );
            }
            Mp4Box::Mehd(m) => {
                println!(
                    "[{}] type: 'mehd', fragment_duration: {}",
                    i, m.fragment_duration
                );
            }
            Mp4Box::Trex(t) => {
                println!(
                    "[{}] type: 'trex', track_id: {}, default_sample_description_index: {}, default_sample_duration: {}, default_sample_size: {}, default_sample_flags: 0x{:08x}",
                    i,
                    t.track_id,
                    t.default_sample_description_index,
                    t.default_sample_duration,
                    t.default_sample_size,
                    t.default_sample_flags,
                );
            }
            Mp4Box::Leva(l) => {
                println!("[{}] type: 'leva', levels: {}", i, l.levels.len());
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
                println!(
                    "[{}] type: 'trun', sample_count: {}, data_offset: {:?}, first_sample_flags: {:?}",
                    i, t.sample_count, t.data_offset, t.first_sample_flags
                );
            }
            Mp4Box::Subs(s) => {
                let total: usize = s.entries.iter().map(|e| e.subsamples.len()).sum();
                println!(
                    "[{}] type: 'subs', entries: {}, total_subsamples: {}",
                    i, s.entry_count, total
                );
            }
            Mp4Box::Sbgp(s) => {
                println!(
                    "[{}] type: 'sbgp', grouping_type: '{}', entries: {}",
                    i,
                    fourcc_u32(s.grouping_type),
                    s.entry_count
                );
            }
            Mp4Box::Sgpd(s) => {
                println!(
                    "[{}] type: 'sgpd', grouping_type: '{}', entries: {}",
                    i,
                    fourcc_u32(s.grouping_type),
                    s.entry_count,
                );
            }
            Mp4Box::Saiz(s) => {
                let aux = s
                    .aux_info_type
                    .map(|t| format!("'{}'", fourcc_u32(t)))
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
                    .map(|t| format!("'{}'", fourcc_u32(t)))
                    .unwrap_or_else(|| "none".to_string());
                let width = if s.header.version == 0 { 32 } else { 64 };
                println!(
                    "[{}] type: 'saio', aux_info_type: {}, entry_count: {}, offset_width: {}-bit",
                    i, aux, s.entry_count, width
                );
            }
            Mp4Box::Meta(m) => {
                println!(
                    "[{}] type: 'meta', hdlr.handler_type: '{}', hdlr.name: {:?}, others: {}",
                    i,
                    fourcc_u32(m.hdlr.handler_type),
                    m.hdlr.name,
                    m.others.len()
                );
            }
            Mp4Box::Pssh(p) => {
                println!(
                    "[{}] type: 'pssh', version: {}, system_id: {:02x?}, key_ids: {}, data: {} bytes",
                    i,
                    p.header.version,
                    p.system_id,
                    p.key_ids.len(),
                    p.data.len()
                );
            }
            Mp4Box::Unknown(u) => {
                let size_str = match u.header.box_size {
                    BoxSize::Normal(s) => format!("{}", s),
                    BoxSize::Large(s) => format!("{} (large)", s),
                    BoxSize::ExtendsToEnd => "end".to_string(),
                };
                let type_bytes: [u8; 4] = (&u.header.box_type).into();
                println!(
                    "[{}] type: '{}', box_size: {}, payload: {} bytes",
                    i,
                    fourcc(&type_bytes),
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
