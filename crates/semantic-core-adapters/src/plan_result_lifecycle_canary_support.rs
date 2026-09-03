//! Frozen R60 plan/result lifecycle evaluator support.
//!
//! The evaluator observes the public response as JSON so it can be frozen
//! before the product exposes a typed plan/result boundary. Language input may
//! select a response axis, but only verifier-bound host receipts may advance
//! the verified execution/result axis.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const ACTION_EVIDENCE_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";

#[derive(Debug, Clone, Copy)]
pub enum PriorState {
    PlanOnly,
    LanguageReport(&'static str),
    VerifiedRunning,
    VerifiedSuccess,
    VerifiedFailure,
    Withdrawn(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub setup: &'static str,
    pub prior: PriorState,
    pub query: &'static str,
    pub language: LanguageCodeIR,
    pub expected_focus: &'static str,
    pub expected_plan: &'static str,
    pub expected_report: Option<&'static str>,
    pub expected_execution: &'static str,
    pub expected_result: &'static str,
    pub expected_snapshot_count: usize,
    pub expected_selected_count: usize,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    suite: String,
    frozen_before_product_changes: bool,
    held_out_until_diagnostic_passes: bool,
    passed: usize,
    failed: usize,
    total: usize,
    rows: Vec<Row>,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
}

fn request(case: &Case, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: case.id.to_string(),
        turn_index,
        request_id: format!("{}-{turn_index}", case.id),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(case.language),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
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

fn submit_receipt(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    suffix: &str,
    status: &str,
) -> bool {
    let receipt_id = format!("{conversation_id}-R60-{suffix}");
    let execution_id = format!("{conversation_id}-EXECUTION-1");
    let evidence_digest = format!("{:064x}", suffix.len() + status.len());
    let verifier_receipt_sha256 = receipt_hash(
        &receipt_id,
        conversation_id,
        action_id,
        &execution_id,
        status,
        &evidence_digest,
    );
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
            "verifier_receipt_sha256": verifier_receipt_sha256
        }
    });
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn action_ids(value: &Value) -> Vec<String> {
    value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| record["action_id"].as_str().map(str::to_string))
        .collect()
}

fn tamper_rejected(
    response: &ConversationTurnResponseIR,
    request: &ConversationTurnRequestIR,
) -> bool {
    let mut value = serde_json::to_value(response).expect("response json");
    let Some(result) = value.pointer_mut("/plan_result_boundary/snapshots/0/result_availability")
    else {
        return false;
    };
    *result = if result == "VERIFIED_SUCCESS" {
        Value::String("UNAVAILABLE".to_string())
    } else {
        Value::String("VERIFIED_SUCCESS".to_string())
    };
    serde_json::from_value::<ConversationTurnResponseIR>(value)
        .ok()
        .is_some_and(|tampered| !tampered.validate_against(request))
}

fn evaluate(case: &Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_request = request(case, 1, case.setup);
    let setup = api
        .process_conversation_turn(&setup_request)
        .expect("setup turn");
    let setup_json = serde_json::to_value(&setup).expect("setup json");
    let ids = action_ids(&setup_json);
    let action_id = ids
        .last()
        .cloned()
        .unwrap_or_else(|| "MISSING-ACTION".to_string());
    let mut query_turn = 2;
    let mut prior_applied = true;
    match case.prior {
        PriorState::PlanOnly => {}
        PriorState::LanguageReport(text) | PriorState::Withdrawn(text) => {
            let prior_request = request(case, 2, text);
            prior_applied = api.process_conversation_turn(&prior_request).is_ok();
            query_turn = 3;
        }
        PriorState::VerifiedRunning => {
            prior_applied =
                submit_receipt(&mut api, case.id, &action_id, "START", "EXECUTION_STARTED");
        }
        PriorState::VerifiedSuccess | PriorState::VerifiedFailure => {
            prior_applied =
                submit_receipt(&mut api, case.id, &action_id, "START", "EXECUTION_STARTED");
            let terminal = if matches!(case.prior, PriorState::VerifiedSuccess) {
                "SUCCEEDED"
            } else {
                "FAILED"
            };
            prior_applied &= submit_receipt(&mut api, case.id, &action_id, "END", terminal);
        }
    }

    let query_request = request(case, query_turn, case.query);
    let response = api
        .process_conversation_turn(&query_request)
        .expect("query turn");
    let value = serde_json::to_value(&response).expect("query json");
    let boundary = &value["plan_result_boundary"];
    let snapshots = boundary["snapshots"].as_array();
    let selected = boundary["selected_action_ids"].as_array();
    let matching_snapshots = snapshots
        .into_iter()
        .flatten()
        .filter(|snapshot| {
            snapshot["plan_status"] == case.expected_plan
                && snapshot["execution_status"] == case.expected_execution
                && snapshot["result_availability"] == case.expected_result
                && match case.expected_report {
                    Some(expected) => snapshot["reported_status"] == expected,
                    None => {
                        snapshot.get("reported_status").is_none()
                            || snapshot["reported_status"].is_null()
                    }
                }
        })
        .count();
    let pass = prior_applied
        && response.schema == "B_CORE_CONVERSATION_TURN_RESPONSE_15"
        && boundary["schema"] == "B_CORE_PLAN_RESULT_BOUNDARY_IR_1"
        && boundary["query_focus"] == case.expected_focus
        && snapshots.is_some_and(|items| items.len() == case.expected_snapshot_count)
        && matching_snapshots == case.expected_snapshot_count
        && selected.is_some_and(|items| items.len() == case.expected_selected_count)
        && boundary["semantic_authority"] == false
        && boundary["external_action_executed"] == false
        && boundary["source_text_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && boundary["ledger_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && boundary["boundary_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
        && response.validate_against(&query_request)
        && tamper_rejected(&response, &query_request)
        && response.output.unsupported_freeform_claims == 0;

    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace: vec![
            format!("prior_applied={prior_applied}"),
            format!("setup_action_ids={ids:?}"),
            value.to_string(),
            response.output.text,
        ],
    }
}

pub fn emit(suite: &str, held_out: bool, cases: &[Case]) {
    let rows = cases.iter().map(evaluate).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        suite: suite.to_string(),
        frozen_before_product_changes: true,
        held_out_until_diagnostic_passes: held_out,
        passed,
        failed: rows.len() - passed,
        total: rows.len(),
        rows,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report json")
    );
    if passed != cases.len() {
        std::process::exit(1);
    }
}
