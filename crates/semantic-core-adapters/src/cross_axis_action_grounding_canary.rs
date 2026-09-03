//! Frozen R32 diagnostic suite, revision 2.
//!
//! Every action case combines a composed multi-goal turn, cross-turn reference
//! resolution, pragmatic report/query classification, typed action state, and
//! claim-level realization.  The suite observes only the public API.
//! Revision 1's four ambiguity prompts contained a salient discourse focus;
//! revision 2 replaces only those pre-product prompts with genuinely unbound
//! multi-action status questions.

use semantic_core_adapters::{
    action_evidence_receipt_sha256, ActionEvidenceRequestIR, ActionEvidenceStatusIR, CognitiveApi,
    ConversationInputModalityIR, ConversationTurnDispositionIR, ConversationTurnRequestIR,
    LanguageCodeIR, ACTION_EVIDENCE_REQUEST_SCHEMA, CONVERSATION_TURN_REQUEST_SCHEMA,
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
struct SequenceCase {
    id: &'static str,
    setup: &'static str,
    follow_up: &'static str,
    language: LanguageCodeIR,
    target_index: usize,
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

fn response_json(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn ledger_records(value: &Value) -> &[Value] {
    value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn action_ids(value: &Value) -> Vec<String> {
    ledger_records(value)
        .iter()
        .filter_map(|record| record["action_id"].as_str().map(str::to_string))
        .collect()
}

fn inherited_goal(value: &Value) -> Option<&str> {
    value
        .pointer("/reference_resolution/discourse_bindings")?
        .as_array()?
        .iter()
        .find(|binding| binding["kind"] == "EVENT_ORDINAL_REFERENCE")?
        .get("inherited_goal_id")?
        .as_str()
}

fn target_is_unique(value: &Value, target: &str) -> bool {
    value
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.len() == 1 && ids[0] == target)
}

fn claim_has(value: &Value, kind: &str, evidence: &str, verified: bool) -> bool {
    value
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array)
        .is_some_and(|claims| {
            claims.iter().any(|claim| {
                claim["kind"] == kind
                    && claim["verified"] == verified
                    && claim["evidence_refs"]
                        .as_array()
                        .is_some_and(|refs| refs.iter().any(|item| item == evidence))
            })
        })
}

fn realization_safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn ordinal_query(case: SequenceCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("sequence setup");
    let setup_value = response_json(&setup);
    let ids = action_ids(&setup_value);
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
        .expect("ordinal query");
    let value = response_json(&response);
    let target = ids.get(case.target_index).cloned().unwrap_or_default();
    Row {
        id: case.id.to_string(),
        category: "ordinal_reference_to_typed_status".to_string(),
        pass: ids.len() == 3
            && inherited_goal(&value) == Some(target.as_str())
            && target_is_unique(&value, &target)
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && claim_has(&value, "PLAN_STATUS", &target, false)
            && claim_has(&value, "EVIDENCE_ABSENCE", &target, false)
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

fn ordinal_report(case: SequenceCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("sequence setup");
    let ids = action_ids(&response_json(&setup));
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
        .expect("ordinal report");
    let value = response_json(&response);
    let target = ids.get(case.target_index).cloned().unwrap_or_default();
    let report_isolated = ledger_records(&value)
        .iter()
        .enumerate()
        .all(|(index, record)| {
            if index == case.target_index {
                record["reported_status"] == "SUCCESS_CLAIMED"
            } else {
                record.get("reported_status").is_none() || record["reported_status"].is_null()
            }
        });
    Row {
        id: case.id.to_string(),
        category: "ordinal_reference_to_language_report".to_string(),
        pass: ids.len() == 3
            && inherited_goal(&value) == Some(target.as_str())
            && target_is_unique(&value, &target)
            && report_isolated
            && claim_has(&value, "LANGUAGE_REPORT", &target, false)
            && !claim_has(&value, "VERIFIED_EXECUTION", &target, true)
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

fn submit(
    api: &mut CognitiveApi,
    conversation_id: &str,
    action_id: &str,
    suffix: &str,
    status: ActionEvidenceStatusIR,
) -> bool {
    let mut receipt = ActionEvidenceRequestIR {
        schema: ACTION_EVIDENCE_REQUEST_SCHEMA.to_string(),
        receipt_id: format!("{conversation_id}-R32-{suffix}"),
        conversation_id: conversation_id.to_string(),
        action_id: action_id.to_string(),
        execution_id: format!("{conversation_id}-EXECUTION"),
        status,
        evidence_digest: format!("{:064x}", suffix.len() * 37),
        verifier_receipt_sha256: String::new(),
    };
    receipt.verifier_receipt_sha256 = action_evidence_receipt_sha256(&receipt);
    let command = json!({"operation":"SUBMIT_ACTION_EVIDENCE", "request": receipt});
    api.execute_command_json(&command.to_string())
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|response| response["ok"] == true)
}

fn ordinal_verified(case: SequenceCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("sequence setup");
    let ids = action_ids(&response_json(&setup));
    let target = ids.get(case.target_index).cloned().unwrap_or_default();
    let receipts = submit(
        &mut api,
        case.id,
        &target,
        "START",
        ActionEvidenceStatusIR::ExecutionStarted,
    ) && submit(
        &mut api,
        case.id,
        &target,
        "END",
        ActionEvidenceStatusIR::Succeeded,
    );
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
        .expect("verified ordinal query");
    let value = response_json(&response);
    Row {
        id: case.id.to_string(),
        category: "ordinal_reference_to_verified_execution".to_string(),
        pass: ids.len() == 3
            && receipts
            && inherited_goal(&value) == Some(target.as_str())
            && target_is_unique(&value, &target)
            && claim_has(&value, "VERIFIED_EXECUTION", &target, true)
            && !response.output.text.to_lowercase().contains("not recorded")
            && !response.output.text.contains("아직 없어")
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

#[derive(Clone, Copy)]
struct TopicCase {
    id: &'static str,
    setup: &'static str,
    shift: &'static str,
    report: &'static str,
    language: LanguageCodeIR,
    target_term: &'static str,
    target_index: usize,
}

fn topic_report(case: TopicCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("topic setup");
    let ids = action_ids(&response_json(&setup));
    api.process_conversation_turn(&request(case.id, 2, case.shift, case.language))
        .expect("topic shift");
    let response = api
        .process_conversation_turn(&request(case.id, 3, case.report, case.language))
        .expect("deictic report");
    let value = response_json(&response);
    let target = ids.get(case.target_index).cloned().unwrap_or_default();
    Row {
        id: case.id.to_string(),
        category: "topic_focus_to_deictic_action_report".to_string(),
        pass: ids.len() >= 2
            && response
                .reference_resolution
                .resolved_semantic_text
                .to_lowercase()
                .contains(case.target_term)
            && target_is_unique(&value, &target)
            && claim_has(&value, "LANGUAGE_REPORT", &target, false)
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

#[derive(Clone, Copy)]
struct EvidenceCase {
    id: &'static str,
    statement: &'static str,
    question: &'static str,
    language: LanguageCodeIR,
    claim_kind: &'static str,
}

fn attributed_evidence(case: EvidenceCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.statement, case.language))
        .expect("attributed evidence");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.question, case.language))
        .expect("evidence question");
    let value = response_json(&response);
    let claim_safe = value
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array)
        .is_some_and(|claims| {
            claims.iter().any(|claim| claim["kind"] == case.claim_kind)
                && claims.iter().all(|claim| {
                    claim["verified"] == false
                        && claim["semantic_authority"] == false
                        && claim["external_action_executed"] == false
                })
        });
    Row {
        id: case.id.to_string(),
        category: "attributed_relation_to_non_authoritative_realization".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && claim_safe
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

fn ambiguity_case(case: SequenceCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("ambiguous setup");
    let response = api
        .process_conversation_turn(&request(case.id, 2, case.follow_up, case.language))
        .expect("ambiguous query");
    let value = response_json(&response);
    let no_target = value
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    Row {
        id: case.id.to_string(),
        category: "ambiguous_action_reference_fails_closed".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && no_target
            && !value.to_string().contains("SUCCESS_CLAIMED")
            && !value.to_string().contains("VERIFIED_OBSERVED")
            && realization_safe(&response),
        trace: vec![value.to_string()],
    }
}

fn main() {
    let en = "repair the cache, then repair the queue, then repair the worker";
    let ko = "캐시를 수리하고 큐를 수리한 뒤 워커를 수리해";
    let mut rows = vec![
        ordinal_query(SequenceCase {
            id: "R32_QUERY_EN_1",
            setup: en,
            follow_up: "What is the status of the first action?",
            language: LanguageCodeIR::English,
            target_index: 0,
        }),
        ordinal_query(SequenceCase {
            id: "R32_QUERY_EN_2",
            setup: en,
            follow_up: "What is the execution result of the second action?",
            language: LanguageCodeIR::English,
            target_index: 1,
        }),
        ordinal_query(SequenceCase {
            id: "R32_QUERY_KO_1",
            setup: ko,
            follow_up: "첫 번째 작업의 상태는?",
            language: LanguageCodeIR::Korean,
            target_index: 0,
        }),
        ordinal_query(SequenceCase {
            id: "R32_QUERY_KO_2",
            setup: ko,
            follow_up: "두 번째 작업의 실행 결과는?",
            language: LanguageCodeIR::Korean,
            target_index: 1,
        }),
        ordinal_report(SequenceCase {
            id: "R32_REPORT_EN_1",
            setup: en,
            follow_up: "I completed the second action",
            language: LanguageCodeIR::English,
            target_index: 1,
        }),
        ordinal_report(SequenceCase {
            id: "R32_REPORT_EN_2",
            setup: en,
            follow_up: "The third action finished",
            language: LanguageCodeIR::English,
            target_index: 2,
        }),
        ordinal_report(SequenceCase {
            id: "R32_REPORT_KO_1",
            setup: ko,
            follow_up: "두 번째 작업은 완료했어",
            language: LanguageCodeIR::Korean,
            target_index: 1,
        }),
        ordinal_report(SequenceCase {
            id: "R32_REPORT_KO_2",
            setup: ko,
            follow_up: "세 번째 작업은 끝났어",
            language: LanguageCodeIR::Korean,
            target_index: 2,
        }),
        ordinal_verified(SequenceCase {
            id: "R32_VERIFY_EN_1",
            setup: en,
            follow_up: "Was the first action completed?",
            language: LanguageCodeIR::English,
            target_index: 0,
        }),
        ordinal_verified(SequenceCase {
            id: "R32_VERIFY_EN_2",
            setup: en,
            follow_up: "What was the result of the third action?",
            language: LanguageCodeIR::English,
            target_index: 2,
        }),
        ordinal_verified(SequenceCase {
            id: "R32_VERIFY_KO_1",
            setup: ko,
            follow_up: "첫 번째 작업은 완료됐어?",
            language: LanguageCodeIR::Korean,
            target_index: 0,
        }),
        ordinal_verified(SequenceCase {
            id: "R32_VERIFY_KO_2",
            setup: ko,
            follow_up: "세 번째 작업의 결과는?",
            language: LanguageCodeIR::Korean,
            target_index: 2,
        }),
        topic_report(TopicCase {
            id: "R32_TOPIC_EN_1",
            setup: "repair the cache and repair the queue",
            shift: "let's return to the cache",
            report: "I completed it",
            language: LanguageCodeIR::English,
            target_term: "cache",
            target_index: 0,
        }),
        topic_report(TopicCase {
            id: "R32_TOPIC_EN_2",
            setup: "repair the parser and repair the worker",
            shift: "back to the worker topic",
            report: "It finished",
            language: LanguageCodeIR::English,
            target_term: "worker",
            target_index: 1,
        }),
        topic_report(TopicCase {
            id: "R32_TOPIC_KO_1",
            setup: "캐시를 수리하고 큐를 수리해",
            shift: "캐시 이야기로 돌아가자",
            report: "그건 완료했어",
            language: LanguageCodeIR::Korean,
            target_term: "캐시",
            target_index: 0,
        }),
        topic_report(TopicCase {
            id: "R32_TOPIC_KO_2",
            setup: "파서를 수리하고 워커를 수리해",
            shift: "워커 이야기로 돌아가자",
            report: "그 작업은 끝났어",
            language: LanguageCodeIR::Korean,
            target_term: "워커",
            target_index: 1,
        }),
        attributed_evidence(EvidenceCase {
            id: "R32_EVIDENCE_EN_1",
            statement: "Alice says that the backup completed before the deploy started.",
            question: "What happened before the deploy started?",
            language: LanguageCodeIR::English,
            claim_kind: "TEMPORAL_RELATION",
        }),
        attributed_evidence(EvidenceCase {
            id: "R32_EVIDENCE_EN_2",
            statement:
                "Nora believes the cache failed. Because of that, she says the worker is blocked.",
            question: "Why is the worker blocked?",
            language: LanguageCodeIR::English,
            claim_kind: "DIALOGUE_RELATION",
        }),
        attributed_evidence(EvidenceCase {
            id: "R32_EVIDENCE_KO_1",
            statement: "민수는 배포가 시작되기 전에 백업이 완료됐다고 말했다.",
            question: "배포가 시작되기 전에 무슨 일이 있었어?",
            language: LanguageCodeIR::Korean,
            claim_kind: "TEMPORAL_RELATION",
        }),
        attributed_evidence(EvidenceCase {
            id: "R32_EVIDENCE_KO_2",
            statement: "지수는 캐시가 실패했다고 믿는다. 그 때문에 워커가 막혔다고 말했다.",
            question: "왜 워커가 막혔어?",
            language: LanguageCodeIR::Korean,
            claim_kind: "DIALOGUE_RELATION",
        }),
    ];
    rows.extend([
        ambiguity_case(SequenceCase {
            id: "R32_AMBIG_EN_1",
            setup: "repair the cache and repair the queue",
            follow_up: "Was any action completed?",
            language: LanguageCodeIR::English,
            target_index: 0,
        }),
        ambiguity_case(SequenceCase {
            id: "R32_AMBIG_EN_2",
            setup: "repair the parser and repair the worker",
            follow_up: "What is the execution result?",
            language: LanguageCodeIR::English,
            target_index: 0,
        }),
        ambiguity_case(SequenceCase {
            id: "R32_AMBIG_KO_1",
            setup: "캐시를 수리하고 큐를 수리해",
            follow_up: "완료된 작업이 있어?",
            language: LanguageCodeIR::Korean,
            target_index: 0,
        }),
        ambiguity_case(SequenceCase {
            id: "R32_AMBIG_KO_2",
            setup: "파서를 수리하고 워커를 수리해",
            follow_up: "실행 결과는?",
            language: LanguageCodeIR::Korean,
            target_index: 0,
        }),
    ]);
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R32-RUN-0002",
            "frozen_before_product_changes":true,
            "revision_1_disposition":"REPLACED_PRE_PRODUCT_INVALID_FOCUS_ORACLE",
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
