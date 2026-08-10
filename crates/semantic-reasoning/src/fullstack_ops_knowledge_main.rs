use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let report_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("reports").join("b-core-code-graft-04-fullstack-ops"));
    let source_root = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("D:/SYNAPSE_CODING_KNOWLEDGE_A2/generations/BF9F10C31F6504473050028280C1DCB22AC54CFD29DFC9215EC29DDD42BBE52C")
    });
    match semantic_reasoning::fullstack_ops_knowledge::run_absorption(
        &report_dir,
        &source_root,
    ) {
        Ok(report) => println!(
            "B_CORE_FULLSTACK_OPS_ABSORPTION={} FRONTEND={} BACKEND={} OPERATIONS={} ATOMS={} RECIPES={}",
            report.status,
            report.frontend_source_count,
            report.backend_source_count,
            report.operations_source_count,
            report.promoted_knowledge_atoms,
            report.promoted_composition_recipes,
        ),
        Err(error) => {
            eprintln!("B_CORE_FULLSTACK_OPS_ABSORPTION_ERROR:{error}");
            std::process::exit(1);
        }
    }
}
