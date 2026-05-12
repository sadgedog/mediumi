use mediumi::h264::Processor;
use mediumi::mp4::boxes::trex::Trex;
use mediumi::mp4::{
    AvccConfig, Mp4Box, demuxer, find_codec_config, handler_fourcc, iter_trafs, iter_traks,
    track_samples, traf_samples,
};

fn main() {
    // Non-fragmented mp4
    {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/test.mp4");
        let data = std::fs::read(path).expect("failed to read mp4");

        let boxes = demuxer::demux(&data).expect("failed to demux mp4");

        let trak = iter_traks(&boxes)
            .find(|t| &handler_fourcc(t) == b"vide")
            .expect("no video track in mp4");

        let avcc_bytes =
            find_codec_config(trak, b"avcC").expect("no avcC inside the video sample entry");
        let cfg = AvccConfig::parse(avcc_bytes).expect("failed to parse avcC");
        let samples = track_samples(trak, &data).expect("failed to extract samples");

        let processor = Processor::from_avcc(
            &samples,
            cfg.length_size_minus_one,
            &cfg.sps_nalus,
            &cfg.pps_nalus,
        )
        .expect("failed to parse AVCC samples");

        let out = processor
            .to_avcc(cfg.length_size_minus_one)
            .expect("failed to serialize AVCC");

        let mut re_cfg = cfg.clone();
        re_cfg.sps_nalus = out.sps_nalus;
        re_cfg.pps_nalus = out.pps_nalus;
        re_cfg.avc_profile_indication = out.avc_profile_indication;
        re_cfg.profile_compatibility = out.profile_compatibility;
        re_cfg.avc_level_indication = out.avc_level_indication;

        let original_concat: Vec<u8> = samples.iter().flat_map(|s| s.iter().copied()).collect();
        assert_eq!(
            out.bytes.len(),
            original_concat.len(),
            "AVCC sample byte length mismatch: orig={}, re={}",
            original_concat.len(),
            out.bytes.len(),
        );
        assert_eq!(out.bytes, original_concat, "AVCC sample bytes differ");

        assert_eq!(re_cfg.length_size_minus_one, cfg.length_size_minus_one);
        assert_eq!(re_cfg.configuration_version, cfg.configuration_version);
        assert_eq!(re_cfg.avc_profile_indication, cfg.avc_profile_indication);
        assert_eq!(re_cfg.profile_compatibility, cfg.profile_compatibility);
        assert_eq!(re_cfg.avc_level_indication, cfg.avc_level_indication);
        assert_eq!(re_cfg.sps_nalus, cfg.sps_nalus);
        assert_eq!(re_cfg.pps_nalus, cfg.pps_nalus);
        assert_eq!(re_cfg.to_bytes(), avcc_bytes, "avcC bytes drifted");

        println!(
            "AVCC roundtrip ok ({} samples, {} bytes; sps={}, pps={}, profile={}, level={})",
            samples.len(),
            out.bytes.len(),
            cfg.sps_nalus.len(),
            cfg.pps_nalus.len(),
            cfg.avc_profile_indication,
            cfg.avc_level_indication,
        );
    }

    // Fragmented mp4 (init + media segment)
    {
        let init_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/test_init.m4s");
        let seg_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/test.m4s");
        let init_data = std::fs::read(init_path).expect("failed to read init segment");
        let seg_data = std::fs::read(seg_path).expect("failed to read media segment");

        let init_boxes = demuxer::demux(&init_data).expect("failed to demux init segment");
        let seg_boxes = demuxer::demux(&seg_data).expect("failed to demux media segment");

        let trak = iter_traks(&init_boxes)
            .find(|t| &handler_fourcc(t) == b"vide")
            .expect("no video track in init segment");
        let video_track_id = trak.tkhd.track_id;

        let avcc_bytes =
            find_codec_config(trak, b"avcC").expect("no avcC inside the video sample entry");
        let cfg = AvccConfig::parse(avcc_bytes).expect("failed to parse avcC");

        let trexs: Vec<&Trex> = init_boxes
            .iter()
            .filter_map(|b| match b {
                Mp4Box::Moov(m) => m.mvex.as_ref(),
                _ => None,
            })
            .flat_map(|mvex| mvex.trexs.iter())
            .collect();
        let trex = trexs.iter().copied().find(|t| t.track_id == video_track_id);

        let mut samples: Vec<&[u8]> = Vec::new();
        for (moof_offset, traf) in iter_trafs(&seg_boxes) {
            if traf.tfhd.track_id != video_track_id {
                continue;
            }
            let traf_sample_bytes = traf_samples(traf, moof_offset, &seg_data, trex)
                .expect("failed to extract traf samples");
            samples.extend(traf_sample_bytes);
        }

        let processor = Processor::from_avcc(
            &samples,
            cfg.length_size_minus_one,
            &cfg.sps_nalus,
            &cfg.pps_nalus,
        )
        .expect("failed to parse AVCC samples");

        let out = processor
            .to_avcc(cfg.length_size_minus_one)
            .expect("failed to serialize AVCC");

        let mut re_cfg = cfg.clone();
        re_cfg.sps_nalus = out.sps_nalus;
        re_cfg.pps_nalus = out.pps_nalus;
        re_cfg.avc_profile_indication = out.avc_profile_indication;
        re_cfg.profile_compatibility = out.profile_compatibility;
        re_cfg.avc_level_indication = out.avc_level_indication;

        let original_concat: Vec<u8> = samples.iter().flat_map(|s| s.iter().copied()).collect();
        assert_eq!(
            out.bytes.len(),
            original_concat.len(),
            "fMP4 AVCC sample byte length mismatch: orig={}, re={}",
            original_concat.len(),
            out.bytes.len(),
        );
        assert_eq!(out.bytes, original_concat, "fMP4 AVCC sample bytes differ");

        assert_eq!(re_cfg.length_size_minus_one, cfg.length_size_minus_one);
        assert_eq!(re_cfg.configuration_version, cfg.configuration_version);
        assert_eq!(re_cfg.avc_profile_indication, cfg.avc_profile_indication);
        assert_eq!(re_cfg.profile_compatibility, cfg.profile_compatibility);
        assert_eq!(re_cfg.avc_level_indication, cfg.avc_level_indication);
        assert_eq!(re_cfg.sps_nalus, cfg.sps_nalus);
        assert_eq!(re_cfg.pps_nalus, cfg.pps_nalus);
        assert_eq!(re_cfg.to_bytes(), avcc_bytes, "avcC bytes drifted");

        println!(
            "fMP4 AVCC roundtrip ok ({} samples, {} bytes; sps={}, pps={}, profile={}, level={})",
            samples.len(),
            out.bytes.len(),
            cfg.sps_nalus.len(),
            cfg.pps_nalus.len(),
            cfg.avc_profile_indication,
            cfg.avc_level_indication,
        );
    }
}
