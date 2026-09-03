//! Frozen R25-RUN-0001 diagnostic suite.
//!
//! The suite uses only the existing public conversation and JSON command
//! surfaces so it can be compiled and hashed before the R25 product repair.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    ConversationTurnResponseIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const EVIDENCE_SCHEMA: &str = "B_CORE_CONDITION_EVIDENCE_REQUEST_1";

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Debug, Clone)]
struct DeferredView {
    commitment_id: String,
    condition_sha256: String,
    status: String,
    serialized: String,
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

fn turn(
    api: &mut CognitiveApi,
    id: &str,
    turn: u64,
    text: &str,
    language: LanguageCodeIR,
) -> ConversationTurnResponseIR {
    api.process_conversation_turn(&request(id, turn, text, language))
        .expect("conversation turn")
}

fn deferred(response: &ConversationTurnResponseIR) -> Vec<DeferredView> {
    let value = serde_json::to_value(response).expect("response value");
    value
        .pointer("/conversation_state/deferred_action_commitments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(DeferredView {
                commitment_id: item.get("commitment_id")?.as_str()?.to_string(),
                condition_sha256: item.get("condition_sha256")?.as_str()?.to_string(),
                status: item.get("status")?.as_str()?.to_string(),
                serialized: serde_json::to_string(item).ok()?.to_uppercase(),
            })
        })
        .collect()
}

fn pending_count(response: &ConversationTurnResponseIR) -> usize {
    deferred(response)
        .iter()
        .filter(|item| item.status == "CONDITION_PENDING")
        .count()
}

fn receipt_sha256(
    evidence_id: &str,
    conversation_id: &str,
    commitment_id: &str,
    condition_sha256: &str,
    disposition: &str,
    source: &str,
) -> String {
    let canonical = [
        EVIDENCE_SCHEMA,
        evidence_id,
        conversation_id,
        commitment_id,
        condition_sha256,
        disposition,
        source,
    ]
    .join("\0");
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

fn submit(
    api: &mut CognitiveApi,
    evidence_id: &str,
    conversation_id: &str,
    commitment: &DeferredView,
    condition_sha256: &str,
    disposition: &str,
    source: &str,
) -> (bool, String) {
    let receipt = receipt_sha256(
        evidence_id,
        conversation_id,
        &commitment.commitment_id,
        condition_sha256,
        disposition,
        source,
    );
    let command = serde_json::json!({
        "operation": "SUBMIT_CONDITION_EVIDENCE",
        "request": {
            "schema": EVIDENCE_SCHEMA,
            "evidence_id": evidence_id,
            "conversation_id": conversation_id,
            "commitment_id": commitment.commitment_id,
            "condition_sha256": condition_sha256,
            "disposition": disposition,
            "source": source,
            "verifier_receipt_sha256": receipt
        }
    });
    let output = api
        .execute_command_json(&command.to_string())
        .unwrap_or_else(|error| format!("{{\"transport_error\":\"{error:?}\"}}"));
    let ok = serde_json::from_str::<Value>(&output)
        .ok()
        .and_then(|value| value.get("ok").and_then(Value::as_bool))
        .unwrap_or(false);
    (ok, output.to_uppercase())
}

fn persistence_case(
    id: &str,
    text: &str,
    language: LanguageCodeIR,
    condition_token: &str,
    action_token: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let response = turn(&mut api, id, 1, text, language);
    let items = deferred(&response);
    Row {
        id: id.to_string(),
        category: "pending_commitment_persistence".to_string(),
        pass: response.conversation_state.active_goals.is_empty()
            && items.len() == 1
            && items[0].status == "CONDITION_PENDING"
            && items[0]
                .serialized
                .contains(&condition_token.to_uppercase())
            && items[0].serialized.contains(action_token),
        trace: vec![
            format!(
                "pending={} active={}",
                pending_count(&response),
                response.conversation_state.active_goals.len()
            ),
            items
                .first()
                .map_or_else(|| "NONE".to_string(), |item| item.serialized.clone()),
        ],
    }
}

fn unverified_language_case(
    id: &str,
    setup: &str,
    assertion: &str,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    turn(&mut api, id, 1, setup, language);
    let response = turn(&mut api, id, 2, assertion, language);
    Row {
        id: id.to_string(),
        category: "language_claim_cannot_activate".to_string(),
        pass: pending_count(&response) == 1 && response.conversation_state.active_goals.is_empty(),
        trace: vec![
            format!("pending={}", pending_count(&response)),
            format!("active={}", response.conversation_state.active_goals.len()),
            response.output.text,
        ],
    }
}

fn verified_activation_case(
    id: &str,
    setup: &str,
    language: LanguageCodeIR,
    source: &str,
    action_token: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "verified_evidence_activation".to_string(),
            pass: false,
            trace: vec!["NO_PENDING_COMMITMENT".to_string()],
        };
    };
    let (accepted, receipt) = submit(
        &mut api,
        &format!("{id}-E1"),
        id,
        &commitment,
        &commitment.condition_sha256,
        "VERIFIED_SATISFIED",
        source,
    );
    let response = turn(&mut api, id, 2, "okay", language);
    let active = &response.conversation_state.active_goals;
    Row {
        id: id.to_string(),
        category: "verified_evidence_activation".to_string(),
        pass: accepted
            && pending_count(&response) == 0
            && active.len() == 1
            && serde_json::to_string(active)
                .expect("active json")
                .to_uppercase()
                .contains(action_token)
            && receipt.contains("ACTIVATED"),
        trace: vec![
            format!(
                "accepted={accepted} pending={} active={}",
                pending_count(&response),
                active.len()
            ),
            receipt,
        ],
    }
}

fn rejected_or_contradicted_case(
    id: &str,
    setup: &str,
    language: LanguageCodeIR,
    mismatch: bool,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "mismatched_or_contradicted_evidence".to_string(),
            pass: false,
            trace: vec!["NO_PENDING_COMMITMENT".to_string()],
        };
    };
    let condition = if mismatch {
        "0".repeat(64)
    } else {
        commitment.condition_sha256.clone()
    };
    let disposition = if mismatch {
        "VERIFIED_SATISFIED"
    } else {
        "VERIFIED_CONTRADICTED"
    };
    let (accepted, receipt) = submit(
        &mut api,
        &format!("{id}-E1"),
        id,
        &commitment,
        &condition,
        disposition,
        "TRUSTED_VERIFIER",
    );
    let response = turn(&mut api, id, 2, "okay", language);
    let status = deferred(&response)
        .first()
        .map(|item| item.status.clone())
        .unwrap_or_default();
    let expected = if mismatch {
        !accepted && status == "CONDITION_PENDING"
    } else {
        accepted && status == "CONTRADICTED"
    };
    Row {
        id: id.to_string(),
        category: "mismatched_or_contradicted_evidence".to_string(),
        pass: expected && response.conversation_state.active_goals.is_empty(),
        trace: vec![format!("accepted={accepted} status={status}"), receipt],
    }
}

fn cancellation_case(id: &str, setup: &str, cancel: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    turn(&mut api, id, 1, setup, language);
    let response = turn(&mut api, id, 2, cancel, language);
    let items = deferred(&response);
    Row {
        id: id.to_string(),
        category: "pending_commitment_withdrawal".to_string(),
        pass: pending_count(&response) == 0
            && response.conversation_state.active_goals.is_empty()
            && items.iter().any(|item| item.status == "WITHDRAWN"),
        trace: vec![
            format!(
                "pending={} active={}",
                pending_count(&response),
                response.conversation_state.active_goals.len()
            ),
            items
                .iter()
                .map(|item| item.status.clone())
                .collect::<Vec<_>>()
                .join(","),
            response.output.text,
        ],
    }
}

fn exactly_once_case(id: &str, setup: &str, language: LanguageCodeIR, source: &str) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "exactly_once_activation".to_string(),
            pass: false,
            trace: vec!["NO_PENDING_COMMITMENT".to_string()],
        };
    };
    let evidence_id = format!("{id}-E1");
    let (first, first_receipt) = submit(
        &mut api,
        &evidence_id,
        id,
        &commitment,
        &commitment.condition_sha256,
        "VERIFIED_SATISFIED",
        source,
    );
    let (second, second_receipt) = submit(
        &mut api,
        &evidence_id,
        id,
        &commitment,
        &commitment.condition_sha256,
        "VERIFIED_SATISFIED",
        source,
    );
    let response = turn(&mut api, id, 2, "okay", language);
    Row {
        id: id.to_string(),
        category: "exactly_once_activation".to_string(),
        pass: first && !second && response.conversation_state.active_goals.len() == 1,
        trace: vec![
            format!(
                "first={first} second={second} active={}",
                response.conversation_state.active_goals.len()
            ),
            first_receipt,
            second_receipt,
        ],
    }
}

fn main() {
    let rows = vec![
        persistence_case(
            "R25_PENDING_1",
            "내가 승인하면 서비스를 배포해",
            LanguageCodeIR::Korean,
            "승인",
            "DEPLOY",
        ),
        persistence_case(
            "R25_PENDING_2",
            "검증이 끝난 뒤에만 보고서를 저장해",
            LanguageCodeIR::Korean,
            "검증",
            "EXECUTE",
        ),
        persistence_case(
            "R25_PENDING_3",
            "Deploy the service only after I approve it.",
            LanguageCodeIR::English,
            "APPROV",
            "DEPLOY",
        ),
        persistence_case(
            "R25_PENDING_4",
            "Save the report once verification succeeds.",
            LanguageCodeIR::English,
            "VERIFICATION",
            "EXECUTE",
        ),
        unverified_language_case(
            "R25_CLAIM_1",
            "내가 승인하면 서비스를 배포해",
            "승인했어",
            LanguageCodeIR::Korean,
        ),
        unverified_language_case(
            "R25_CLAIM_2",
            "검증이 끝난 뒤에만 보고서를 저장해",
            "검증이 끝났어",
            LanguageCodeIR::Korean,
        ),
        unverified_language_case(
            "R25_CLAIM_3",
            "Deploy the service only after I approve it.",
            "I approved it.",
            LanguageCodeIR::English,
        ),
        unverified_language_case(
            "R25_CLAIM_4",
            "Save the report once verification succeeds.",
            "Verification succeeded.",
            LanguageCodeIR::English,
        ),
        verified_activation_case(
            "R25_ACTIVATE_1",
            "내가 승인하면 서비스를 배포해",
            LanguageCodeIR::Korean,
            "AUTHORIZATION_RECEIPT",
            "DEPLOY",
        ),
        verified_activation_case(
            "R25_ACTIVATE_2",
            "검증이 끝난 뒤에만 보고서를 저장해",
            LanguageCodeIR::Korean,
            "TRUSTED_VERIFIER",
            "EXECUTE",
        ),
        verified_activation_case(
            "R25_ACTIVATE_3",
            "Deploy the service only after I approve it.",
            LanguageCodeIR::English,
            "AUTHORIZATION_RECEIPT",
            "DEPLOY",
        ),
        verified_activation_case(
            "R25_ACTIVATE_4",
            "Save the report once verification succeeds.",
            LanguageCodeIR::English,
            "EXECUTION_RECEIPT",
            "EXECUTE",
        ),
        rejected_or_contradicted_case(
            "R25_REJECT_1",
            "내가 승인하면 서비스를 배포해",
            LanguageCodeIR::Korean,
            true,
        ),
        rejected_or_contradicted_case(
            "R25_REJECT_2",
            "Deploy the service only after I approve it.",
            LanguageCodeIR::English,
            true,
        ),
        rejected_or_contradicted_case(
            "R25_REJECT_3",
            "검증이 끝난 뒤에만 보고서를 저장해",
            LanguageCodeIR::Korean,
            false,
        ),
        rejected_or_contradicted_case(
            "R25_REJECT_4",
            "Save the report once verification succeeds.",
            LanguageCodeIR::English,
            false,
        ),
        cancellation_case(
            "R25_CANCEL_1",
            "내가 승인하면 서비스를 배포해",
            "그 조건부 요청은 취소해",
            LanguageCodeIR::Korean,
        ),
        cancellation_case(
            "R25_CANCEL_2",
            "검증이 끝난 뒤에만 보고서를 저장해",
            "됐어, 그 대기 요청은 그만해",
            LanguageCodeIR::Korean,
        ),
        cancellation_case(
            "R25_CANCEL_3",
            "Deploy the service only after I approve it.",
            "Never mind, cancel that pending request.",
            LanguageCodeIR::English,
        ),
        cancellation_case(
            "R25_CANCEL_4",
            "Save the report once verification succeeds.",
            "Withdraw that conditional request.",
            LanguageCodeIR::English,
        ),
        exactly_once_case(
            "R25_ONCE_1",
            "내가 승인하면 서비스를 배포해",
            LanguageCodeIR::Korean,
            "AUTHORIZATION_RECEIPT",
        ),
        exactly_once_case(
            "R25_ONCE_2",
            "검증이 끝난 뒤에만 보고서를 저장해",
            LanguageCodeIR::Korean,
            "TRUSTED_VERIFIER",
        ),
        exactly_once_case(
            "R25_ONCE_3",
            "Deploy the service only after I approve it.",
            LanguageCodeIR::English,
            "AUTHORIZATION_RECEIPT",
        ),
        exactly_once_case(
            "R25_ONCE_4",
            "Save the report once verification succeeds.",
            LanguageCodeIR::English,
            "EXECUTION_RECEIPT",
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "suite": "R25-RUN-0001",
            "frozen_before_product_changes": true,
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
    if passed != 24 {
        std::process::exit(1);
    }
}
