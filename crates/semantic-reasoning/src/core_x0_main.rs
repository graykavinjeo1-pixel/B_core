use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repository root")
        .to_path_buf();
    let outcome = semantic_reasoning::core_x0::run_evaluation(&root).expect("CORE-X0 evaluation");
    println!(
        "CORE_BEHAVIOR_PARITY={}\nCORE_ONLY_RUNTIME_CANARY_PASS={}\nLANGUAGE_ADAPTER_DOCK_PASS={}\nGENERIC_CAPABILITY_DOCK_PASS={}",
        outcome.core_behavior_parity,
        outcome.core_runtime_canary_pass,
        outcome.language_adapter_dock_pass,
        outcome.generic_capability_dock_pass
    );
}
