//! Frozen R25-RUN-0002 held-out transfer suite.
//!
//! Compile and hash before product changes. Do not semantically execute until
//! the diagnostic suite has been repaired.

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
    id: String,
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
        .expect("held-out turn")
}

fn deferred(response: &ConversationTurnResponseIR) -> Vec<DeferredView> {
    serde_json::to_value(response)
        .expect("response value")
        .pointer("/conversation_state/deferred_action_commitments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(DeferredView {
                id: item.get("commitment_id")?.as_str()?.to_string(),
                condition_sha256: item.get("condition_sha256")?.as_str()?.to_string(),
                status: item.get("status")?.as_str()?.to_string(),
                serialized: serde_json::to_string(item).ok()?.to_uppercase(),
            })
        })
        .collect()
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
    conversation_id: &str,
    commitment: &DeferredView,
    evidence_id: &str,
    condition_sha256: &str,
    disposition: &str,
    source: &str,
) -> (bool, String) {
    let command = serde_json::json!({
        "operation": "SUBMIT_CONDITION_EVIDENCE",
        "request": {
            "schema": EVIDENCE_SCHEMA,
            "evidence_id": evidence_id,
            "conversation_id": conversation_id,
            "commitment_id": commitment.id,
            "condition_sha256": condition_sha256,
            "disposition": disposition,
            "source": source,
            "verifier_receipt_sha256": receipt_sha256(
                evidence_id,
                conversation_id,
                &commitment.id,
                condition_sha256,
                disposition,
                source,
            )
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

fn pending_or_claim_case(
    id: &str,
    setup: &str,
    follow_up: Option<&str>,
    language: LanguageCodeIR,
    required: &[&str],
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let first = turn(&mut api, id, 1, setup, language);
    let response = follow_up.map_or(first, |text| turn(&mut api, id, 2, text, language));
    let items = deferred(&response);
    Row {
        id: id.to_string(),
        category: "held_out_pending_and_language_boundary".to_string(),
        pass: response.conversation_state.active_goals.is_empty()
            && items
                .iter()
                .filter(|item| item.status == "CONDITION_PENDING")
                .count()
                == 1
            && required.iter().all(|token| {
                items
                    .iter()
                    .any(|item| item.serialized.contains(&token.to_uppercase()))
            }),
        trace: vec![
            format!("active={}", response.conversation_state.active_goals.len()),
            items
                .iter()
                .map(|item| item.serialized.clone())
                .collect::<Vec<_>>()
                .join(" | "),
        ],
    }
}

fn activation_case(
    id: &str,
    setup: &str,
    language: LanguageCodeIR,
    source: &str,
    action: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "held_out_verified_activation".to_string(),
            pass: false,
            trace: vec!["NO_PENDING".to_string()],
        };
    };
    let (ok, receipt) = submit(
        &mut api,
        id,
        &commitment,
        &format!("{id}-E1"),
        &commitment.condition_sha256,
        "VERIFIED_SATISFIED",
        source,
    );
    let state = turn(&mut api, id, 2, "thanks", language);
    let active_json = serde_json::to_string(&state.conversation_state.active_goals)
        .expect("active json")
        .to_uppercase();
    Row {
        id: id.to_string(),
        category: "held_out_verified_activation".to_string(),
        pass: ok
            && state.conversation_state.active_goals.len() == 1
            && active_json.contains(action)
            && deferred(&state)
                .iter()
                .any(|item| item.status == "ACTIVATED")
            && receipt.contains("ACTIVATED"),
        trace: vec![
            format!(
                "ok={ok} active={}",
                state.conversation_state.active_goals.len()
            ),
            receipt,
        ],
    }
}

fn invalid_or_replay_case(id: &str, setup: &str, language: LanguageCodeIR, replay: bool) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "held_out_integrity_and_replay".to_string(),
            pass: false,
            trace: vec!["NO_PENDING".to_string()],
        };
    };
    let evidence_id = format!("{id}-E1");
    let first_hash = if replay {
        commitment.condition_sha256.clone()
    } else {
        "f".repeat(64)
    };
    let (first, first_receipt) = submit(
        &mut api,
        id,
        &commitment,
        &evidence_id,
        &first_hash,
        "VERIFIED_SATISFIED",
        "TRUSTED_VERIFIER",
    );
    let (second, second_receipt) = if replay {
        submit(
            &mut api,
            id,
            &commitment,
            &evidence_id,
            &commitment.condition_sha256,
            "VERIFIED_SATISFIED",
            "TRUSTED_VERIFIER",
        )
    } else {
        (false, String::new())
    };
    let state = turn(&mut api, id, 2, "okay", language);
    let pass = if replay {
        first && !second && state.conversation_state.active_goals.len() == 1
    } else {
        !first
            && state.conversation_state.active_goals.is_empty()
            && deferred(&state)
                .iter()
                .any(|item| item.status == "CONDITION_PENDING")
    };
    Row {
        id: id.to_string(),
        category: "held_out_integrity_and_replay".to_string(),
        pass,
        trace: vec![
            format!("first={first} second={second}"),
            first_receipt,
            second_receipt,
        ],
    }
}

fn cancel_or_contradict_case(
    id: &str,
    setup: &str,
    turn_text: Option<&str>,
    language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup_response = turn(&mut api, id, 1, setup, language);
    let Some(commitment) = deferred(&setup_response).into_iter().next() else {
        return Row {
            id: id.to_string(),
            category: "held_out_retirement".to_string(),
            pass: false,
            trace: vec!["NO_PENDING".to_string()],
        };
    };
    let (accepted, receipt, response) = if let Some(cancel) = turn_text {
        (true, String::new(), turn(&mut api, id, 2, cancel, language))
    } else {
        let (ok, receipt) = submit(
            &mut api,
            id,
            &commitment,
            &format!("{id}-E1"),
            &commitment.condition_sha256,
            "VERIFIED_CONTRADICTED",
            "TRUSTED_VERIFIER",
        );
        (ok, receipt, turn(&mut api, id, 2, "okay", language))
    };
    let statuses = deferred(&response)
        .iter()
        .map(|item| item.status.clone())
        .collect::<Vec<_>>();
    Row {
        id: id.to_string(),
        category: "held_out_retirement".to_string(),
        pass: accepted
            && response.conversation_state.active_goals.is_empty()
            && statuses
                .iter()
                .any(|status| matches!(status.as_str(), "WITHDRAWN" | "CONTRADICTED"))
            && !statuses.iter().any(|status| status == "CONDITION_PENDING"),
        trace: vec![format!("statuses={statuses:?}"), receipt],
    }
}

fn main() {
    let rows = vec![
        pending_or_claim_case(
            "R25_TRANSFER_1",
            "관리자가 허가한 경우에만 아카이브를 삭제해",
            None,
            LanguageCodeIR::Korean,
            &["허가", "DELETE"],
        ),
        pending_or_claim_case(
            "R25_TRANSFER_2",
            "Publish the bundle after the checksum is verified.",
            None,
            LanguageCodeIR::English,
            &["CHECKSUM", "DEPLOY"],
        ),
        pending_or_claim_case(
            "R25_TRANSFER_3",
            "백업이 확인된 뒤에만 캐시를 지워",
            Some("백업이 확인됐어"),
            LanguageCodeIR::Korean,
            &["백업", "DELETE"],
        ),
        pending_or_claim_case(
            "R25_TRANSFER_4",
            "Remove the snapshot only when approval exists.",
            Some("Approval exists."),
            LanguageCodeIR::English,
            &["APPROVAL", "DELETE"],
        ),
        activation_case(
            "R25_TRANSFER_5",
            "담당자가 허락한 뒤에만 스냅샷을 삭제해",
            LanguageCodeIR::Korean,
            "AUTHORIZATION_RECEIPT",
            "DELETE",
        ),
        activation_case(
            "R25_TRANSFER_6",
            "백업이 검증된 뒤에만 캐시를 지워",
            LanguageCodeIR::Korean,
            "TRUSTED_VERIFIER",
            "DELETE",
        ),
        activation_case(
            "R25_TRANSFER_7",
            "Publish the bundle after the checksum is verified.",
            LanguageCodeIR::English,
            "EXECUTION_RECEIPT",
            "DEPLOY",
        ),
        activation_case(
            "R25_TRANSFER_8",
            "Remove the snapshot only when approval exists.",
            LanguageCodeIR::English,
            "AUTHORIZATION_RECEIPT",
            "DELETE",
        ),
        invalid_or_replay_case(
            "R25_TRANSFER_9",
            "백업이 검증된 뒤에만 캐시를 지워",
            LanguageCodeIR::Korean,
            false,
        ),
        invalid_or_replay_case(
            "R25_TRANSFER_10",
            "Publish the bundle after the checksum is verified.",
            LanguageCodeIR::English,
            false,
        ),
        invalid_or_replay_case(
            "R25_TRANSFER_11",
            "담당자가 허락한 뒤에만 스냅샷을 삭제해",
            LanguageCodeIR::Korean,
            true,
        ),
        invalid_or_replay_case(
            "R25_TRANSFER_12",
            "Remove the snapshot only when approval exists.",
            LanguageCodeIR::English,
            true,
        ),
        cancel_or_contradict_case(
            "R25_TRANSFER_13",
            "백업이 검증된 뒤에만 캐시를 지워",
            Some("그 대기 중인 요청은 취소해"),
            LanguageCodeIR::Korean,
        ),
        cancel_or_contradict_case(
            "R25_TRANSFER_14",
            "Publish the bundle after the checksum is verified.",
            Some("Cancel that deferred request."),
            LanguageCodeIR::English,
        ),
        cancel_or_contradict_case(
            "R25_TRANSFER_15",
            "담당자가 허락한 뒤에만 스냅샷을 삭제해",
            None,
            LanguageCodeIR::Korean,
        ),
        cancel_or_contradict_case(
            "R25_TRANSFER_16",
            "Remove the snapshot only when approval exists.",
            None,
            LanguageCodeIR::English,
        ),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "suite": "R25-RUN-0002",
            "held_out_until_after_diagnostic_repairs": true,
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
