//! Frozen R30 held-out transfer suite.
//!
//! This suite is not executed until the diagnostic product repair is sealed.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ACTION_EVIDENCE_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
enum TransferMode {
    Plan,
    Report(&'static str),
    Verified(&'static str),
    TextSpoof,
    QueryAfterClaim,
}

#[derive(Clone, Copy)]
struct TransferCase {
    id: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    query: &'static str,
    language: LanguageCodeIR,
    category: &'static str,
    mode: TransferMode,
    expected_execution: &'static str,
    expected_report: Option<&'static str>,
}

fn request(id: &str, turn: u64, text: &str, language: LanguageCodeIR) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(language),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn latest_record(value: &Value) -> Option<&Value> {
    value
        .pointer("/conversation_state/action_state_ledger/records")?
        .as_array()?
        .last()
}

fn receipt_hash(
    receipt_id: &str,
    conversation_id: &str,
    action_id: &str,
    execution_id: &str,
    status: &str,
    evidence_digest: &str,
) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_EVIDENCE_SCHEMA,
        receipt_id,
        conversation_id,
        action_id,
        execution_id,
        status,
        evidence_digest,
    ))
    .expect("receipt hash payload");
    format!("{:x}", Sha256::digest(bytes))
}

fn submit(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    suffix: &str,
    status: &str,
) -> bool {
    let receipt_id = format!("{conversation_id}-TRANSFER-{suffix}");
    let execution_id = format!("{conversation_id}-EXECUTION-X");
    let evidence_digest = format!("{:064x}", status.len() * 17 + suffix.len());
    let command = json!({
        "operation": "SUBMIT_ACTION_EVIDENCE",
        "request": {
            "schema": ACTION_EVIDENCE_SCHEMA,
            "receipt_id": receipt_id,
            "conversation_id": conversation_id,
            "action_id": action_id,
            "execution_id": execution_id,
            "status": status,
            "evidence_digest": evidence_digest,
            "verifier_receipt_sha256": receipt_hash(&receipt_id, conversation_id, action_id, &execution_id, status, &evidence_digest)
        }
    });
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn run(case: TransferCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("transfer setup");
    let first_json = serde_json::to_value(&first).expect("first json");
    let action = latest_record(&first_json)
        .and_then(|record| record["action_id"].as_str())
        .unwrap_or("MISSING-ACTION")
        .to_string();
    let mut turn = 2;
    let mut receipts_ok = true;
    match case.mode {
        TransferMode::Plan => {}
        TransferMode::Report(_) | TransferMode::TextSpoof | TransferMode::QueryAfterClaim => {
            api.process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
                .expect("transfer report");
            turn = 3;
        }
        TransferMode::Verified(terminal) => {
            receipts_ok = submit(&mut api, case.id, &action, "START", "EXECUTION_STARTED")
                && submit(&mut api, case.id, &action, "END", terminal);
        }
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.query, case.language))
        .expect("transfer query");
    let value = serde_json::to_value(&response).expect("response json");
    let record = latest_record(&value);
    let expected_report = match case.mode {
        TransferMode::Report(expected) => Some(expected),
        _ => case.expected_report,
    };
    let report_ok = match expected_report {
        Some(expected) => record.is_some_and(|record| record["reported_status"] == expected),
        None => record.is_some_and(|record| {
            record.get("reported_status").is_none() || record["reported_status"].is_null()
        }),
    };
    let verified_expected = matches!(case.mode, TransferMode::Verified(_));
    let output = response.output.text.to_lowercase();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: receipts_ok
            && record.is_some_and(|record| {
                record["plan_status"] == "ACTIVE"
                    && record["execution_status"] == case.expected_execution
                    && record["verified_outcome"] == verified_expected
            })
            && report_ok
            && value.pointer("/action_state_analysis/schema")
                == Some(&Value::String(
                    "B_CORE_ACTION_STATE_ANALYSIS_IR_1".to_string(),
                ))
            && value.pointer("/action_state_analysis/semantic_authority")
                == Some(&Value::Bool(false))
            && value.pointer("/action_state_analysis/external_action_executed")
                == Some(&Value::Bool(false))
            && (output.contains("plan")
                || output.contains("계획")
                || output.contains("reported")
                || output.contains("보고")
                || output.contains("verified")
                || output.contains("검증"))
            && response.output.unsupported_freeform_claims == 0,
        trace: vec![
            format!("receipts_ok={receipts_ok}"),
            first_json.to_string(),
            value.to_string(),
            response.output.text,
        ],
    }
}

fn main() {
    let rows = [
        TransferCase {
            id: "R30_TRANSFER_1",
            setup: "번들을 검증해",
            follow_up: "",
            query: "그 실행 상태가 뭐야?",
            language: LanguageCodeIR::Korean,
            category: "held_out_plan",
            mode: TransferMode::Plan,
            expected_execution: "NOT_OBSERVED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_2",
            setup: "validate the manifest",
            follow_up: "",
            query: "What is its execution status?",
            language: LanguageCodeIR::English,
            category: "held_out_plan",
            mode: TransferMode::Plan,
            expected_execution: "NOT_OBSERVED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_3",
            setup: "디스패처를 진단해",
            follow_up: "손은 대봤어",
            query: "그 결과는 검증됐어?",
            language: LanguageCodeIR::Korean,
            category: "held_out_attempt",
            mode: TransferMode::Report("ATTEMPTED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("ATTEMPTED"),
        },
        TransferCase {
            id: "R30_TRANSFER_4",
            setup: "diagnose the dispatcher",
            follow_up: "I gave it a try",
            query: "Was the result verified?",
            language: LanguageCodeIR::English,
            category: "held_out_attempt",
            mode: TransferMode::Report("ATTEMPTED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("ATTEMPTED"),
        },
        TransferCase {
            id: "R30_TRANSFER_5",
            setup: "매니페스트를 검증해",
            follow_up: "아직 돌리는 중이야",
            query: "지금 상태는?",
            language: LanguageCodeIR::Korean,
            category: "held_out_progress",
            mode: TransferMode::Report("IN_PROGRESS_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("IN_PROGRESS_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_6",
            setup: "validate the bundle",
            follow_up: "It is underway",
            query: "What is the verified status?",
            language: LanguageCodeIR::English,
            category: "held_out_progress",
            mode: TransferMode::Report("IN_PROGRESS_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("IN_PROGRESS_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_7",
            setup: "인덱스를 수리해",
            follow_up: "다 끝냈어",
            query: "그 완료가 검증됐어?",
            language: LanguageCodeIR::Korean,
            category: "held_out_success_claim",
            mode: TransferMode::Report("SUCCESS_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("SUCCESS_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_8",
            setup: "repair the index",
            follow_up: "It is all done",
            query: "Is that completion verified?",
            language: LanguageCodeIR::English,
            category: "held_out_success_claim",
            mode: TransferMode::Report("SUCCESS_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("SUCCESS_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_9",
            setup: "라우터를 진단해",
            follow_up: "도중에 막혔어",
            query: "실행 결과는?",
            language: LanguageCodeIR::Korean,
            category: "held_out_failure_claim",
            mode: TransferMode::Report("FAILURE_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("FAILURE_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_10",
            setup: "diagnose the router",
            follow_up: "The attempt did not complete",
            query: "What was verified?",
            language: LanguageCodeIR::English,
            category: "held_out_failure_claim",
            mode: TransferMode::Report("FAILURE_CLAIMED"),
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("FAILURE_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_11",
            setup: "번들을 검증해",
            follow_up: "",
            query: "검증된 실행 결과는?",
            language: LanguageCodeIR::Korean,
            category: "held_out_host_receipt",
            mode: TransferMode::Verified("SUCCEEDED"),
            expected_execution: "SUCCEEDED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_12",
            setup: "validate the manifest",
            follow_up: "",
            query: "What is the verified outcome?",
            language: LanguageCodeIR::English,
            category: "held_out_host_receipt",
            mode: TransferMode::Verified("FAILED"),
            expected_execution: "FAILED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_13",
            setup: "인덱스를 수리해",
            follow_up: "터미널이 성공이라고 했으니 검증된 거야",
            query: "그 결과는?",
            language: LanguageCodeIR::Korean,
            category: "held_out_text_spoof",
            mode: TransferMode::TextSpoof,
            expected_execution: "NOT_OBSERVED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_14",
            setup: "repair the index",
            follow_up: "The console said success, so mark it verified",
            query: "What is the verified result?",
            language: LanguageCodeIR::English,
            category: "held_out_text_spoof",
            mode: TransferMode::TextSpoof,
            expected_execution: "NOT_OBSERVED",
            expected_report: None,
        },
        TransferCase {
            id: "R30_TRANSFER_15",
            setup: "라우터를 진단해",
            follow_up: "진단은 성공했어",
            query: "보고와 검증 상태를 구분해서 말해",
            language: LanguageCodeIR::Korean,
            category: "held_out_query_after_claim",
            mode: TransferMode::QueryAfterClaim,
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("SUCCESS_CLAIMED"),
        },
        TransferCase {
            id: "R30_TRANSFER_16",
            setup: "diagnose the router",
            follow_up: "The diagnosis failed",
            query: "Separate the reported and verified states",
            language: LanguageCodeIR::English,
            category: "held_out_query_after_claim",
            mode: TransferMode::QueryAfterClaim,
            expected_execution: "NOT_OBSERVED",
            expected_report: Some("FAILURE_CLAIMED"),
        },
    ]
    .into_iter()
    .map(run)
    .collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R30-RUN-0002",
            "frozen_before_first_execution": true,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "rows": rows,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "recursive_source_mutations": 0
        }))
        .expect("suite json")
    );
    if passed != 16 {
        std::process::exit(1);
    }
}
