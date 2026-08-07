use std::{fs, path::PathBuf};

use semantic_reasoning::{
    sem9::model::FreshBlindManifest,
    sem9r1::{
        integrity::{build_run0001_receipt, freeze_failed_candidate, verify_r1_predecessor},
        tasks::{
            build_run0002_manifest, generate_run0002_tasks, verify_freshness_against_run0001,
            RUN0002_ID,
        },
    },
};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    if let Err(error) = verify_r1_predecessor(&root) {
        eprintln!("SEM9_R1_STATUS=FAIL\nDISPOSITION={error}");
        std::process::exit(1);
    }
    let r1_directory = root.join("reports/sem9-r1");
    if r1_directory.exists()
        && fs::read_dir(&r1_directory).is_ok_and(|mut entries| entries.any(|entry| entry.is_ok()))
    {
        eprintln!("SEM9_R1_STATUS=FAIL\nDISPOSITION=R1_REPORT_DIRECTORY_NOT_EMPTY");
        std::process::exit(1);
    }
    fs::create_dir_all(&r1_directory).expect("create R1 directory");
    let receipt = build_run0001_receipt(&root).expect("build RUN-0001 receipt");
    let candidate_freeze = freeze_failed_candidate(&root).expect("freeze failed candidate");
    let (fresh, adversarial) = generate_run0002_tasks();
    let manifest = build_run0002_manifest(&fresh, &adversarial);
    let run0001: FreshBlindManifest = serde_json::from_slice(
        &fs::read(root.join("reports/sem9/fresh_blind_manifest.json"))
            .expect("read RUN-0001 manifest"),
    )
    .expect("parse RUN-0001 manifest");
    verify_freshness_against_run0001(&run0001, &manifest).expect("freshness");
    write_json(
        root.join("reports/sem9/run-0001_failure_receipt.json"),
        &receipt,
    );
    write_json(
        r1_directory.join("failed_candidate_freeze.json"),
        &candidate_freeze,
    );
    write_json(
        r1_directory.join("run0002_fresh_blind_manifest.json"),
        &manifest,
    );
    println!("SEM9_R1_FREEZE_STATUS=PASS");
    println!("RUN0001_RECEIPT_SHA256={}", receipt.receipt_sha256);
    println!("RUN0002_ID={RUN0002_ID}");
    println!("RUN0002_FRESH_BLIND_TASKS={}", manifest.fresh_tasks.len());
    println!(
        "RUN0002_ADVERSARIAL_TASKS={}",
        manifest.adversarial_tasks.len()
    );
    println!("RUN0002_BLIND_MANIFEST_SHA256={}", manifest.manifest_sha256);
    println!(
        "FAILED_CANDIDATE_SOURCE_SHA256={}",
        candidate_freeze.failed_candidate_source_sha256
    );
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec_pretty(value).expect("serialize"))
        .expect("write frozen artifact");
}
