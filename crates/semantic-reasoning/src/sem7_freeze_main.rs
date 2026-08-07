use std::{fs, path::PathBuf};

use semantic_reasoning::sem7::{
    corpus::{build_manifest, category_counts, generate_language_tasks},
    experiment::{RUN_ID, TASK_SEED},
    integrity::verify_predecessors,
};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let integrity = verify_predecessors(&root).expect("pre-evaluation predecessor integrity");
    let tasks = generate_language_tasks(TASK_SEED);
    let manifest = build_manifest(RUN_ID, TASK_SEED, &tasks);
    let directory = root.join("reports/sem7");
    fs::create_dir_all(&directory).expect("report directory");
    archive_previous_failure(&directory);
    write_json(directory.join("predecessor_integrity.json"), &integrity);
    write_json(directory.join("blind_manifest.json"), &manifest);
    println!("PREDECESSOR_INTEGRITY=PASS");
    println!("FRESH_BLIND_TASKS={}", tasks.len());
    println!("CATEGORY_COUNTS={:?}", category_counts(&tasks));
    println!("BLIND_MANIFEST_SHA256={}", manifest.manifest_sha256);
    println!("EVALUATION_READY=true");
}

fn archive_previous_failure(directory: &std::path::Path) {
    let final_path = directory.join("sem7_final_report.json");
    let Ok(bytes) = fs::read(&final_path) else {
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return;
    };
    if value["sem7_status"] != "FAIL" {
        return;
    }
    let run_id = value["run_id"].as_str().unwrap_or("SEM7-RUN-UNKNOWN");
    let archive = directory.join("failed_runs").join(run_id);
    fs::create_dir_all(&archive).expect("failed run archive");
    for name in [
        "predecessor_integrity.json",
        "blind_manifest.json",
        "sem7_final_report.json",
        "SEM7_REPORT.md",
    ] {
        let source = directory.join(name);
        if source.is_file() {
            fs::copy(source, archive.join(name)).expect("archive failed run artifact");
        }
    }
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize")).expect("write");
}
