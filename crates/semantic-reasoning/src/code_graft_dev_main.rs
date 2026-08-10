use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let report_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join("b-core-code-graft-01"));
    let source_root = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("D:/SYNAPSE_CODING_KNOWLEDGE_A2/generations/CBAF8D5548446D1D3165E4712A450F56A529AABB929AE757EAAC59596E51140C")
    });
    if let Err(error) = semantic_reasoning::code_graft::run_development(&report_dir, &source_root) {
        eprintln!("B_CORE_CODE_GRAFT_DEV_ERROR:{error}");
        std::process::exit(1);
    }
}
