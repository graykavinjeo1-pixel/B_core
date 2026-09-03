//! Frozen R40-RUN-0001 topic-anchored deixis and ellipsis diagnostic.
//!
//! The suite observes only the public conversation API. Expectations are
//! structural: an exact live topic/group anchor, typed selector, selected
//! member keys, safe authority flags, and fail-closed unresolved outcomes.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy)]
struct Step<'a> {
    text: &'a str,
    language: LanguageCodeIR,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

fn request(id: &str, turn: u64, step: Step<'_>) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: id.to_string(),
        turn_index: turn,
        request_id: format!("{id}-{turn}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: step.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(step.language),
        context_tags: Vec::new(),
        max_plan_steps: 20,
    }
}

fn run(
    api: &mut CognitiveApi,
    id: &str,
    turn: &mut u64,
    step: Step<'_>,
) -> semantic_core_adapters::ConversationTurnResponseIR {
    let response = api
        .process_conversation_turn(&request(id, *turn, step))
        .expect("conversation turn");
    *turn += 1;
    response
}

fn value(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn anchor(value: &Value) -> Option<&Value> {
    value.pointer("/reference_resolution/topic_anchored_resolution")
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|item| item.to_lowercase())
        .collect()
}

fn active_group<'a>(value: &'a Value, group_id: &str) -> Option<&'a Value> {
    value
        .pointer("/conversation_state/active_discourse_groups")?
        .as_array()?
        .iter()
        .find(|group| group["group_id"] == group_id)
}

fn selected_action_subjects(value: &Value, anchor: &Value) -> Vec<String> {
    let selected = strings(anchor.get("selected_member_keys"));
    let mut subjects = value
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|record| {
            record["goal_id"]
                .as_str()
                .is_some_and(|id| selected.contains(&id.to_lowercase()))
        })
        .filter_map(|record| record["subject"].as_str())
        .map(|subject| subject.to_lowercase())
        .collect::<Vec<_>>();
    subjects.sort();
    subjects
}

fn safe_applied_anchor(value: &Value, expected_kind: &str, selector: &str) -> bool {
    let Some(anchor) = anchor(value) else {
        return false;
    };
    let Some(group_id) = anchor["group_id"].as_str() else {
        return false;
    };
    let Some(group) = active_group(value, group_id) else {
        return false;
    };
    let Some(topic) = value.pointer("/conversation_state/active_topics/0") else {
        return false;
    };
    anchor["schema"] == "B_CORE_TOPIC_ANCHORED_REFERENCE_IR_1"
        && anchor["applied"] == true
        && anchor["kind"] == expected_kind
        && anchor["selector"] == selector
        && anchor["topic_id"] == topic["topic_id"]
        && anchor["topic_sha256"] == topic["topic_sha256"]
        && anchor["group_id"] == group["group_id"]
        && anchor["group_revision"] == group["revision"]
        && anchor["membership_sha256"] == group["membership_sha256"]
        && anchor["member_keys"] == group["member_keys"]
        && anchor["semantic_authority"] == false
        && anchor["external_execution_authorized"] == false
        && anchor["resolution_sha256"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
}

fn action_case(
    id: &str,
    category: &str,
    steps: &[Step<'_>],
    expected_subjects: &[&str],
    expected_kind: &str,
    selector: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut response = None;
    for step in steps {
        response = Some(run(&mut api, id, &mut turn, *step));
    }
    let response = response.expect("steps");
    let observed = value(&response);
    let selected = anchor(&observed)
        .map(|item| selected_action_subjects(&observed, item))
        .unwrap_or_default();
    let mut expected = expected_subjects
        .iter()
        .map(|subject| subject.to_lowercase())
        .collect::<Vec<_>>();
    expected.sort();
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && safe_applied_anchor(&observed, expected_kind, selector)
            && selected == expected
            && response
                .reference_resolution
                .discourse_bindings
                .iter()
                .all(|binding| {
                    binding
                        .evidence
                        .contains(&"SEMANTIC_AUTHORITY:false".to_string())
                        && binding
                            .evidence
                            .contains(&"EXTERNAL_EXECUTION_AUTHORIZED:false".to_string())
                })
            && response.grounded_realization.unsupported_claims == 0,
        trace: vec![observed.to_string()],
    }
}

fn speaker_case(
    id: &str,
    category: &str,
    steps: &[Step<'_>],
    expected_sources: &[&str],
    expected_kind: &str,
    selector: &str,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut response = None;
    for step in steps {
        response = Some(run(&mut api, id, &mut turn, *step));
    }
    let response = response.expect("steps");
    let observed = value(&response);
    let selected = anchor(&observed)
        .map(|item| strings(item.get("selected_member_keys")))
        .unwrap_or_default();
    let mut expected = expected_sources
        .iter()
        .map(|source| source.to_lowercase())
        .collect::<Vec<_>>();
    expected.sort();
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: id.to_string(),
        category: category.to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::Grounded
            && safe_applied_anchor(&observed, expected_kind, selector)
            && selected == expected
            && expected.iter().all(|source| resolved.contains(source))
            && response.grounded_realization.unsupported_claims == 0,
        trace: vec![observed.to_string()],
    }
}

fn unresolved_case(id: &str, steps: &[Step<'_>], selector: &str, expected_term: &str) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1;
    let mut response = None;
    for step in steps {
        response = Some(run(&mut api, id, &mut turn, *step));
    }
    let response = response.expect("steps");
    let observed = value(&response);
    let unresolved = anchor(&observed);
    Row {
        id: id.to_string(),
        category: "topic_anchor_fail_closed".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && response.grounded_response.is_none()
            && response.reference_resolution.original_semantic_text
                == response.reference_resolution.resolved_semantic_text
            && unresolved.is_some_and(|anchor| {
                anchor["applied"] == false
                    && anchor["kind"] == "UNRESOLVED"
                    && anchor["selector"] == selector
                    && strings(anchor.get("selected_member_keys")).is_empty()
                    && strings(anchor.get("unresolved_terms"))
                        .contains(&expected_term.to_lowercase())
                    && anchor["semantic_authority"] == false
                    && anchor["external_execution_authorized"] == false
            }),
        trace: vec![observed.to_string()],
    }
}

fn action_steps<'a>(
    setup: &'a str,
    activate: &'a str,
    switch: &'a str,
    restore: &'a str,
    follow: &'a str,
    language: LanguageCodeIR,
) -> [Step<'a>; 5] {
    [setup, activate, switch, restore, follow].map(|text| Step { text, language })
}

#[allow(clippy::too_many_arguments)]
fn speaker_steps<'a>(
    first: &'a str,
    second: &'a str,
    establish: &'a str,
    activate: &'a str,
    switch: &'a str,
    restore: &'a str,
    follow: &'a str,
    language: LanguageCodeIR,
) -> [Step<'a>; 7] {
    [first, second, establish, activate, switch, restore, follow]
        .map(|text| Step { text, language })
}

fn main() {
    let en = LanguageCodeIR::English;
    let ko = LanguageCodeIR::Korean;
    let mut rows = Vec::new();

    for (id, steps, expected) in [
        (
            "R40_AORD_EN_1",
            action_steps(
                "inspect cache and repair queue",
                "Make that task group the current topic.",
                "Switch to the backup topic.",
                "Resume the prior topic.",
                "Inspect the second one again.",
                en,
            ),
            "queue",
        ),
        (
            "R40_AORD_EN_2",
            action_steps(
                "repair server and analyze log",
                "Keep that task group as our topic.",
                "Back to the worker topic.",
                "Return to the previous topic.",
                "Check the first task again.",
                en,
            ),
            "server",
        ),
        (
            "R40_AORD_KO_1",
            action_steps(
                "캐시를 확인하고 큐를 수리해",
                "그 작업 묶음을 현재 주제로 두자",
                "백업 주제로 돌아가자",
                "이전 주제로 돌아가자",
                "두 번째 것을 다시 검사해",
                ko,
            ),
            "큐",
        ),
        (
            "R40_AORD_KO_2",
            action_steps(
                "서버를 수리하고 로그를 분석해",
                "그 작업 묶음을 화제로 삼자",
                "워커 주제로 전환하자",
                "직전 주제로 복귀하자",
                "첫 번째 작업을 다시 확인해",
                ko,
            ),
            "서버",
        ),
    ] {
        rows.push(action_case(
            id,
            "anchored_action_ordinal",
            &steps,
            &[expected],
            "ACTION_MEMBER",
            "ORDINAL",
        ));
    }

    for (id, steps, expected) in [
        (
            "R40_APRED_EN_1",
            action_steps(
                "inspect cache and repair queue",
                "Make that task group the current topic.",
                "Switch to the backup topic.",
                "Resume the prior topic.",
                "Analyze the one being repaired.",
                en,
            ),
            "queue",
        ),
        (
            "R40_APRED_EN_2",
            action_steps(
                "repair server and analyze log",
                "Keep that task group as our topic.",
                "Back to the worker topic.",
                "Return to the previous topic.",
                "Inspect the analysis task.",
                en,
            ),
            "log",
        ),
        (
            "R40_APRED_KO_1",
            action_steps(
                "캐시를 확인하고 큐를 수리해",
                "그 작업 묶음을 현재 주제로 두자",
                "백업 주제로 돌아가자",
                "이전 주제로 돌아가자",
                "수리하는 것을 분석해",
                ko,
            ),
            "큐",
        ),
        (
            "R40_APRED_KO_2",
            action_steps(
                "서버를 수리하고 로그를 분석해",
                "그 작업 묶음을 화제로 삼자",
                "워커 주제로 전환하자",
                "직전 주제로 복귀하자",
                "분석 작업을 다시 확인해",
                ko,
            ),
            "로그",
        ),
    ] {
        rows.push(action_case(
            id,
            "anchored_predicate_role",
            &steps,
            &[expected],
            "ACTION_MEMBER",
            "PREDICATE_ROLE",
        ));
    }

    for (id, steps, expected) in [
        (
            "R40_APLUR_EN_1",
            action_steps(
                "inspect cache and repair queue",
                "Make that task group the current topic.",
                "Switch to the backup topic.",
                "Resume the prior topic.",
                "Inspect both again.",
                en,
            ),
            ["cache", "queue"],
        ),
        (
            "R40_APLUR_EN_2",
            action_steps(
                "repair server and analyze log",
                "Keep that task group as our topic.",
                "Back to the worker topic.",
                "Return to the previous topic.",
                "Check them again.",
                en,
            ),
            ["server", "log"],
        ),
        (
            "R40_APLUR_KO_1",
            action_steps(
                "캐시를 확인하고 큐를 수리해",
                "그 작업 묶음을 현재 주제로 두자",
                "백업 주제로 돌아가자",
                "이전 주제로 돌아가자",
                "둘 다 다시 검사해",
                ko,
            ),
            ["캐시", "큐"],
        ),
        (
            "R40_APLUR_KO_2",
            action_steps(
                "서버를 수리하고 로그를 분석해",
                "그 작업 묶음을 화제로 삼자",
                "워커 주제로 전환하자",
                "직전 주제로 복귀하자",
                "그것들을 다시 확인해",
                ko,
            ),
            ["서버", "로그"],
        ),
    ] {
        rows.push(action_case(
            id,
            "anchored_plural_ellipsis",
            &steps,
            &expected,
            "ACTION_GROUP",
            "PLURAL",
        ));
    }

    for (id, language, steps, expected) in [
        (
            "R40_AREV_EN_1",
            en,
            [
                "inspect cache and repair queue",
                "analyze worker",
                "Make the first task group the topic.",
                "Switch to the backup topic.",
                "Attach worker to the first task group.",
                "Resume the prior topic.",
                "Inspect the third one again.",
            ],
            "worker",
        ),
        (
            "R40_AREV_EN_2",
            en,
            [
                "repair server and analyze log",
                "inspect backup",
                "Make the first task group the topic.",
                "Switch to the cache topic.",
                "Include backup in the first task group.",
                "Return to the previous topic.",
                "Check the last one again.",
            ],
            "backup",
        ),
        (
            "R40_AREV_KO_1",
            ko,
            [
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석해",
                "첫 작업 묶음을 주제로 두자",
                "백업 주제로 돌아가자",
                "첫 작업 묶음에 워커를 추가해",
                "이전 주제로 돌아가자",
                "세 번째 것을 다시 검사해",
            ],
            "워커",
        ),
        (
            "R40_AREV_KO_2",
            ko,
            [
                "서버를 수리하고 로그를 분석해",
                "백업을 확인해",
                "첫 작업 묶음을 화제로 삼자",
                "캐시 주제로 전환하자",
                "첫 작업 묶음에 백업을 포함해",
                "직전 주제로 복귀하자",
                "마지막 것을 다시 확인해",
            ],
            "백업",
        ),
    ] {
        let steps = steps.map(|text| Step { text, language });
        rows.push(action_case(
            id,
            "live_revision_member_selection",
            &steps,
            &[expected],
            "ACTION_MEMBER",
            "ORDINAL",
        ));
    }

    for (id, steps, expected) in [
        (
            "R40_SORD_EN_1",
            speaker_steps(
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
                "What did Alice and Bob say?",
                "Make that speaker group the current topic.",
                "Switch to the backup topic.",
                "Resume the prior topic.",
                "What did the second one say?",
                en,
            ),
            "bob",
        ),
        (
            "R40_SORD_EN_2",
            speaker_steps(
                "Mina reports the server is slow.",
                "Jin reports the log is clean.",
                "What did Mina and Jin report?",
                "Keep that speaker group as our topic.",
                "Back to the worker topic.",
                "Return to the previous topic.",
                "Summarize what the first speaker reported.",
                en,
            ),
            "mina",
        ),
        (
            "R40_SORD_KO_1",
            speaker_steps(
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
                "민수와 지수는 뭐라고 말했어?",
                "그 화자 묶음을 현재 주제로 두자",
                "백업 주제로 돌아가자",
                "이전 주제로 돌아가자",
                "두 번째 사람은 뭐라고 말했어?",
                ko,
            ),
            "지수",
        ),
        (
            "R40_SORD_KO_2",
            speaker_steps(
                "영희는 서버가 느리다고 보고했다.",
                "수진은 로그가 깨끗하다고 보고했다.",
                "영희와 수진은 뭐라고 보고했어?",
                "그 화자 묶음을 화제로 삼자",
                "워커 주제로 전환하자",
                "직전 주제로 복귀하자",
                "첫 번째 화자가 보고한 내용을 요약해",
                ko,
            ),
            "영희",
        ),
    ] {
        rows.push(speaker_case(
            id,
            "anchored_speaker_ordinal",
            &steps,
            &[expected],
            "PROPOSITION_MEMBER",
            "ORDINAL",
        ));
    }

    for (id, steps, expected) in [
        (
            "R40_SPLUR_EN_1",
            speaker_steps(
                "Alice says the cache is stale.",
                "Bob says the queue is empty.",
                "What did Alice and Bob say?",
                "Make that speaker group the current topic.",
                "Switch to the backup topic.",
                "Resume the prior topic.",
                "What did they report?",
                en,
            ),
            ["alice", "bob"],
        ),
        (
            "R40_SPLUR_EN_2",
            speaker_steps(
                "Mina reports the server is slow.",
                "Jin reports the log is clean.",
                "What did Mina and Jin report?",
                "Keep that speaker group as our topic.",
                "Back to the worker topic.",
                "Return to the previous topic.",
                "Summarize their reports.",
                en,
            ),
            ["mina", "jin"],
        ),
        (
            "R40_SPLUR_KO_1",
            speaker_steps(
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 비었다고 말했다.",
                "민수와 지수는 뭐라고 말했어?",
                "그 화자 묶음을 현재 주제로 두자",
                "백업 주제로 돌아가자",
                "이전 주제로 돌아가자",
                "그들은 뭐라고 보고했어?",
                ko,
            ),
            ["민수", "지수"],
        ),
        (
            "R40_SPLUR_KO_2",
            speaker_steps(
                "영희는 서버가 느리다고 보고했다.",
                "수진은 로그가 깨끗하다고 보고했다.",
                "영희와 수진은 뭐라고 보고했어?",
                "그 화자 묶음을 화제로 삼자",
                "워커 주제로 전환하자",
                "직전 주제로 복귀하자",
                "그들의 보고를 요약해",
                ko,
            ),
            ["영희", "수진"],
        ),
    ] {
        rows.push(speaker_case(
            id,
            "anchored_speaker_plural",
            &steps,
            &expected,
            "PROPOSITION_GROUP",
            "PLURAL",
        ));
    }

    rows.push(unresolved_case(
        "R40_SAFE_EN_1",
        &action_steps(
            "inspect cache and repair queue",
            "Make that task group the current topic.",
            "Switch to the backup topic.",
            "Resume the prior topic.",
            "Inspect that one again.",
            en,
        ),
        "GENERIC_SINGULAR",
        "AMBIGUOUS_GROUP_MEMBER",
    ));
    rows.push(unresolved_case(
        "R40_SAFE_KO_1",
        &action_steps(
            "캐시를 확인하고 큐를 수리해",
            "그 작업 묶음을 현재 주제로 두자",
            "백업 주제로 돌아가자",
            "이전 주제로 돌아가자",
            "그것을 다시 검사해",
            ko,
        ),
        "GENERIC_SINGULAR",
        "AMBIGUOUS_GROUP_MEMBER",
    ));
    rows.push(unresolved_case(
        "R40_SAFE_EN_2",
        &action_steps(
            "inspect cache and repair queue",
            "Make that task group the current topic.",
            "Switch to the backup topic.",
            "Resume the prior topic.",
            "What did they say?",
            en,
        ),
        "TYPE_MISMATCH",
        "ANCHOR_KIND_MISMATCH",
    ));
    rows.push(unresolved_case(
        "R40_SAFE_KO_2",
        &speaker_steps(
            "민수는 캐시가 오래됐다고 말했다.",
            "지수는 큐가 비었다고 말했다.",
            "민수와 지수는 뭐라고 말했어?",
            "그 화자 묶음을 현재 주제로 두자",
            "백업 주제로 돌아가자",
            "이전 주제로 돌아가자",
            "그것들을 다시 실행해",
            ko,
        ),
        "TYPE_MISMATCH",
        "ANCHOR_KIND_MISMATCH",
    ));

    let passed = rows.iter().filter(|row| row.pass).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite": "R40-RUN-0001",
            "frozen_before_product_changes": true,
            "total": rows.len(),
            "passed": passed,
            "failed": rows.len() - passed,
            "external_llm_calls": 0,
            "local_teacher_calls": 0,
            "network_calls": 0,
            "recursive_source_mutations": 0,
            "rows": rows,
        }))
        .expect("suite json")
    );
}
