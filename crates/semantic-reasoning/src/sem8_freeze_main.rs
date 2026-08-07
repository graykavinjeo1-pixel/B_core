use std::{fs, path::PathBuf};

use semantic_reasoning::sem8::{
    catalog::{build_source_manifest, extract_source_mechanisms},
    experiment::{RUN_ID, TASK_SEED},
    integrity::verify_predecessors,
    model::SourceSplit,
    tasks::{build_target_manifest, generate_transfer_tasks},
};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let integrity = verify_predecessors(&root).expect("pre-evaluation predecessor integrity");
    let catalog = extract_source_mechanisms();
    let dev = build_source_manifest(RUN_ID, SourceSplit::Development, &catalog);
    let blind = build_source_manifest(RUN_ID, SourceSplit::Blind, &catalog);
    let tasks = generate_transfer_tasks(TASK_SEED);
    let targets = build_target_manifest(RUN_ID, TASK_SEED, &tasks);
    let directory = root.join("reports/sem8");
    fs::create_dir_all(&directory).expect("report directory");
    write_json(directory.join("predecessor_integrity.json"), &integrity);
    write_json(directory.join("transfer_dev_source_manifest.json"), &dev);
    write_json(
        directory.join("transfer_blind_source_manifest.json"),
        &blind,
    );
    write_json(directory.join("blind_target_manifest.json"), &targets);
    println!("PREDECESSOR_INTEGRITY=PASS");
    println!("FRESH_BLIND_TRANSFER_TASKS={}", tasks.len());
    println!("TRANSFER_DEV_SOURCES={}", dev.entries.len());
    println!("TRANSFER_BLIND_SOURCES={}", blind.entries.len());
    println!("BLIND_TARGET_MANIFEST_SHA256={}", targets.manifest_sha256);
    println!("EVALUATION_READY=true");
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize")).expect("write");
}
