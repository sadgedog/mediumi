use mediumi_codec::ac3::frame::CplStrategy;
use mediumi_codec::api::ac3::Processor;

fn main() {
    let input = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/test.ac3"
    ))
    .expect("failed to read input ac3 file");

    let result = Processor::parse(&input);
    match result {
        Ok(processor) => {
            for (i, frame) in processor.ac3_frames.iter().enumerate() {
                let si = &frame.si;
                let bsi = &frame.bsi;
                println!(
                    "[{:3}] size={:3} fscod={} frmsizecod={:2} bsid={:2} bsmod={} acmod={} lfeon={} dialnorm={:2} audblks={}",
                    i,
                    si.frame_size().unwrap_or(0),
                    si.fscod,
                    si.frmsizecod,
                    bsi.bsid,
                    bsi.bsmod,
                    bsi.acmod,
                    bsi.lfeon as u8,
                    bsi.dialnorm,
                    frame.ab.len(),
                );
                for (blk_idx, ab) in frame.ab.iter().enumerate() {
                    let cpl_str = match &ab.cpl.strategy {
                        CplStrategy::Reuse => "reuse".to_string(),
                        CplStrategy::NotInUse => "off".to_string(),
                        CplStrategy::InUse {
                            cplbegf,
                            cplendf,
                            chincpl,
                            ..
                        } => {
                            format!(
                                "on(begf={},endf={},chincpl={:?})",
                                cplbegf, cplendf, chincpl
                            )
                        }
                    };
                    let es = &ab.exponent_strategy;
                    let mant = &ab.mantissas;
                    let ch_mant_len: Vec<usize> = mant.chmant.iter().map(|m| m.len()).collect();
                    let cpl_mant_len = mant.cplmant.as_ref().map(|m| m.len());
                    println!(
                        "  blk[{}] blksw={:?} dith={:?} dynrng={:?} cpl={} chexpstr={:?} chbwcod={:?} chmant={:?} cplmant={:?}",
                        blk_idx,
                        ab.blksw,
                        ab.dithflag,
                        ab.dynrng,
                        cpl_str,
                        es.chexpstr,
                        es.chbwcod,
                        ch_mant_len,
                        cpl_mant_len,
                    );
                }
            }
            println!("--- {} frames ---", processor.ac3_frames.len());
        }
        Err(e) => {
            eprintln!("parse error: {:?}", e);
            std::process::exit(1);
        }
    }
}
