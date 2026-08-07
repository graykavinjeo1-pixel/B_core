use std::{fs, path::PathBuf};

use semantic_reasoning::sem6::{
    corpus::{build_task_manifest, generate_knowledge_sets},
    firewall::ForagingFirewall,
    integrity::verify_predecessors,
    model::{ForagingEnvironment, QueryCategory},
};
use serde_json::json;

const RUN_ID: &str = "SEM6-RUN-0001";
const TASK_SEED: u64 = 0x5e6_2026_0808;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let integrity = verify_predecessors(&root).expect("pre-network predecessor integrity");
    let sets = generate_knowledge_sets(TASK_SEED);
    let sem6a = build_task_manifest(
        RUN_ID,
        TASK_SEED,
        ForagingEnvironment::SealedCorpusA,
        &sets.sealed_tasks,
    )
    .expect("sealed manifest");
    let sem6b = build_task_manifest(
        RUN_ID,
        TASK_SEED ^ 0x1a6e_600d,
        ForagingEnvironment::ControlledLiveB,
        &sets.live_tasks,
    )
    .expect("live manifest");
    let mut firewall = ForagingFirewall::new(Vec::new());
    let requests = sets
        .live_tasks
        .iter()
        .map(|task| {
            firewall.propose_request(
                &task.visible,
                match task.visible.domain {
                    semantic_reasoning::sem6::model::KnowledgeDomain::ProgrammingApi => {
                        QueryCategory::GetApiContract
                    }
                    semantic_reasoning::sem6::model::KnowledgeDomain::MathematicalFormal => {
                        QueryCategory::GetFormalRule
                    }
                    _ => QueryCategory::GetStandardSemantics,
                },
            )
        })
        .collect::<Vec<_>>();
    assert!(requests.iter().all(|request| request.sanitized));
    let live_source_intent = json!({
        "frozen_before_network": true,
        "http_method": "GET",
        "allowlisted_sources": [
            {"source_id":"LIVE-RUST-I64","url":"https://doc.rust-lang.org/std/primitive.i64.html","authority":"OFFICIAL_DOCUMENTATION"},
            {"source_id":"LIVE-RFC4648","url":"https://www.rfc-editor.org/rfc/rfc4648","authority":"OFFICIAL_STANDARD"},
            {"source_id":"LIVE-RFC9110","url":"https://www.rfc-editor.org/rfc/rfc9110","authority":"OFFICIAL_STANDARD"},
            {"source_id":"LIVE-RFC2616-STALE","url":"https://www.rfc-editor.org/rfc/rfc2616","authority":"OFFICIAL_STANDARD_DEPRECATED"},
            {"source_id":"LIVE-RFC8259","url":"https://www.rfc-editor.org/rfc/rfc8259","authority":"OFFICIAL_STANDARD"},
            {"source_id":"LIVE-RFC3986","url":"https://www.rfc-editor.org/rfc/rfc3986","authority":"OFFICIAL_STANDARD"},
            {"source_id":"LIVE-DLMF-FLOOR","url":"https://dlmf.nist.gov/4.2","authority":"INSTITUTIONAL_REFERENCE"}
        ],
        "remote_write": false,
        "authenticated_account_mutation": false,
        "download_execution": false,
        "solution_foraging": false,
    });
    let directory = root.join("reports/sem6");
    fs::create_dir_all(&directory).expect("report directory");
    write_json(directory.join("predecessor_integrity.json"), &integrity);
    write_json(
        directory.join("sem6a_corpus_manifest.json"),
        &json!({"task_manifest": sem6a, "documents": sets.sealed_documents}),
    );
    write_json(directory.join("sem6b_live_task_manifest.json"), &sem6b);
    write_json(directory.join("foraging_requests.json"), &requests);
    write_json(
        directory.join("live_source_intent.json"),
        &live_source_intent,
    );
    println!("PRE_NETWORK_INTEGRITY=PASS");
    println!("SEM6A_TASKS={}", sets.sealed_tasks.len());
    println!("SEM6B_TASKS={}", sets.live_tasks.len());
    println!("SEM6B_MANIFEST_SHA256={}", sem6b.manifest_sha256);
    println!("LIVE_RETRIEVAL_READY=true");
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) {
    let bytes = serde_json::to_vec_pretty(value).expect("serialize");
    fs::write(path, bytes).expect("write");
}
