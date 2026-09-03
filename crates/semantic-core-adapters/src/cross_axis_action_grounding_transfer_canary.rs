//! Frozen R32 held-out transfer suite.
//!
//! This file is not executed until the R32 diagnostic repair passes.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    language: LanguageCodeIR,
    target_index: Option<usize>,
    expected_kind: &'static str,
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
        max_plan_steps: 16,
    }
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("transfer setup");
    let setup_value = serde_json::to_value(&setup).expect("setup json");
    let ids = setup_value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .map(|records| {
            records
                .iter()
                .filter_map(|record| record["action_id"].as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
        .expect("transfer follow-up");
    let value = serde_json::to_value(&response).expect("response json");
    let claims = value
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array);
    let expected = case.target_index.and_then(|index| ids.get(index));
    let pass = if let Some(target) = expected {
        value
            .pointer("/action_state_analysis/target_action_ids")
            .and_then(Value::as_array)
            .is_some_and(|selected| selected.len() == 1 && selected[0] == target.as_str())
            && claims.is_some_and(|claims| {
                claims.iter().any(|claim| {
                    claim["kind"] == case.expected_kind
                        && claim["evidence_refs"]
                            .as_array()
                            .is_some_and(|refs| refs.iter().any(|item| item == target.as_str()))
                })
            })
    } else {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && value
                .pointer("/action_state_analysis/target_action_ids")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
    };
    Row {
        id: case.id.to_string(),
        category: if expected.is_some() {
            "heldout_cross_axis_binding".to_string()
        } else {
            "heldout_ambiguity".to_string()
        },
        pass: pass
            && response.grounded_realization.validate()
            && response.grounded_realization.unsupported_claims == 0,
        trace: vec![value.to_string()],
    }
}

fn main() {
    let cases = [
        Case {
            id: "R32_X_EN_1",
            setup: "repair the decoder, then repair the scheduler, then repair the encoder",
            follow_up: "What is the status of the second task?",
            language: LanguageCodeIR::English,
            target_index: Some(1),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_EN_2",
            setup: "inspect the atlas, then inspect the beacon, then inspect the cipher",
            follow_up: "What was the outcome of the last action?",
            language: LanguageCodeIR::English,
            target_index: Some(2),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_KO_1",
            setup: "디코더를 수리하고 스케줄러를 수리한 뒤 인코더를 수리해",
            follow_up: "두 번째 작업의 상태는?",
            language: LanguageCodeIR::Korean,
            target_index: Some(1),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_KO_2",
            setup: "아틀라스를 검사하고 비콘을 검사한 뒤 암호기를 검사해",
            follow_up: "마지막 작업의 결과는?",
            language: LanguageCodeIR::Korean,
            target_index: Some(2),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_REPORT_EN_1",
            setup: "repair the relay, then repair the bridge, then repair the gateway",
            follow_up: "The first task is all done",
            language: LanguageCodeIR::English,
            target_index: Some(0),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_REPORT_EN_2",
            setup: "inspect the index, then inspect the journal, then inspect the kernel",
            follow_up: "I completed the last action",
            language: LanguageCodeIR::English,
            target_index: Some(2),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_REPORT_KO_1",
            setup: "릴레이를 수리하고 브리지를 수리한 뒤 게이트웨이를 수리해",
            follow_up: "첫 번째 작업은 다 끝났어",
            language: LanguageCodeIR::Korean,
            target_index: Some(0),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_REPORT_KO_2",
            setup: "인덱스를 검사하고 저널을 검사한 뒤 커널을 검사해",
            follow_up: "마지막 작업은 완료했어",
            language: LanguageCodeIR::Korean,
            target_index: Some(2),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_CROSS_1",
            setup: "캐시를 수리하고 큐를 수리한 뒤 워커를 수리해",
            follow_up: "What is the result of the second action?",
            language: LanguageCodeIR::English,
            target_index: Some(1),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_CROSS_2",
            setup: "repair the cache, then repair the queue, then repair the worker",
            follow_up: "두 번째 작업의 결과는?",
            language: LanguageCodeIR::Korean,
            target_index: Some(1),
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_CROSS_3",
            setup: "로그를 검사하고 매니페스트를 검사한 뒤 아카이브를 검사해",
            follow_up: "The third task finished",
            language: LanguageCodeIR::English,
            target_index: Some(2),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_CROSS_4",
            setup: "inspect the log, then inspect the manifest, then inspect the archive",
            follow_up: "세 번째 작업은 끝났어",
            language: LanguageCodeIR::Korean,
            target_index: Some(2),
            expected_kind: "LANGUAGE_REPORT",
        },
        Case {
            id: "R32_X_AMBIG_EN_1",
            setup: "repair the relay and repair the bridge",
            follow_up: "Was any action successful?",
            language: LanguageCodeIR::English,
            target_index: None,
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_AMBIG_EN_2",
            setup: "inspect the journal and inspect the kernel",
            follow_up: "What is the execution status?",
            language: LanguageCodeIR::English,
            target_index: None,
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_AMBIG_KO_1",
            setup: "릴레이를 수리하고 브리지를 수리해",
            follow_up: "성공한 작업이 있어?",
            language: LanguageCodeIR::Korean,
            target_index: None,
            expected_kind: "EVIDENCE_ABSENCE",
        },
        Case {
            id: "R32_X_AMBIG_KO_2",
            setup: "저널을 검사하고 커널을 검사해",
            follow_up: "실행 상태는?",
            language: LanguageCodeIR::Korean,
            target_index: None,
            expected_kind: "EVIDENCE_ABSENCE",
        },
    ];
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R32-TRANSFER-0001",
            "frozen_before_product_changes":true,
            "held_out_until_diagnostic_pass":true,
            "external_llm_calls":0,
            "local_teacher_calls":0,
            "recursive_source_mutations":0,
            "total":total,
            "passed":passed,
            "failed":total-passed,
            "rows":rows
        }))
        .expect("suite json")
    );
    if passed != total {
        std::process::exit(1);
    }
}
