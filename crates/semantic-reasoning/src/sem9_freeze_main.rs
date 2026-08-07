use std::{fs, path::PathBuf};

use semantic_reasoning::sem9::{
    experiment::{RUN_ID, TASK_SEED},
    integrity::{build_protected_core_manifest, verify_predecessors},
    reporting::preserve_failed_run,
    tasks::{build_manifest, generate_adversarial_tasks, generate_fresh_tasks},
};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    if let Err(error) = verify_predecessors(&root) {
        eprintln!("SEM9_STATUS=FAIL\nDISPOSITION={error}");
        std::process::exit(1);
    }
    let report_dir = root.join("reports/sem9");
    if report_dir.exists()
        && fs::read_dir(&report_dir).is_ok_and(|mut entries| entries.any(|entry| entry.is_ok()))
    {
        preserve_failed_run(&root, "REPLACED_BEFORE_NEW_SEM9_FREEZE").expect("archive prior run");
    }
    fs::create_dir_all(&report_dir).expect("create report directory");
    let fresh = generate_fresh_tasks(TASK_SEED);
    let adversarial = generate_adversarial_tasks(TASK_SEED);
    let manifest = build_manifest(RUN_ID, TASK_SEED, &fresh, &adversarial);
    let protected =
        build_protected_core_manifest(&root, RUN_ID).expect("build protected core manifest");
    write_json(report_dir.join("fresh_blind_manifest.json"), &manifest);
    write_json(report_dir.join("protected_core_manifest.json"), &protected);
    println!("SEM9_FREEZE_STATUS=PASS");
    println!("RUN_ID={RUN_ID}");
    println!("FRESH_BLIND_TASKS={}", manifest.fresh_tasks.len());
    println!(
        "ADVERSARIAL_BLIND_TASKS={}",
        manifest.adversarial_tasks.len()
    );
    println!("BLIND_MANIFEST_SHA256={}", manifest.manifest_sha256);
    println!("PROTECTED_CORE_SHA256={}", protected.manifest_sha256);
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) {
    let bytes = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, bytes).expect("write frozen artifact");
}
