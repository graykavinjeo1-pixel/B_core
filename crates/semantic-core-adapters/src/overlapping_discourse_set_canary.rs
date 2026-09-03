//! Frozen R35-RUN-0001 overlapping discourse-set diagnostic.
//!
//! Frozen before the R35 product mechanism exists. The suite exercises two
//! coexisting action groups, overlapping attributed-speaker groups, ordinal
//! and topic-keyed restoration, correction scoped by both source and topic,
//! bilingual selection, and fail-closed multi-group ambiguity through the
//! public conversational API. Case text is evaluator-only.

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
struct ActionCase {
    id: &'static str,
    turns: &'static [&'static str],
    follow: &'static str,
    required: &'static [&'static str],
    rejected: &'static [&'static str],
    ambiguous: bool,
    language: LanguageCodeIR,
    category: &'static str,
}

#[derive(Clone, Copy)]
struct SpeakerCase {
    id: &'static str,
    turns: &'static [&'static str],
    establishment_turns: &'static [usize],
    follow: &'static str,
    required: &'static [&'static str],
    rejected: &'static [&'static str],
    ambiguous: bool,
    language: LanguageCodeIR,
    category: &'static str,
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

fn value(response: &semantic_core_adapters::ConversationTurnResponseIR) -> Value {
    serde_json::to_value(response).expect("response json")
}

fn binding(observed: &Value, kind: &str, size: usize) -> bool {
    observed
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|item| {
                item["kind"] == kind
                    && item["referent_ids"]
                        .as_array()
                        .is_some_and(|ids| ids.len() == size)
            })
        })
}

fn group_count(observed: &Value, kind: &str) -> usize {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|group| {
            group["kind"] == kind
                && group["semantic_authority"] == false
                && group["external_execution_authorized"] == false
        })
        .count()
}

fn safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn terms_match(text: &str, required: &[&str], rejected: &[&str]) -> bool {
    let lower = text.to_lowercase();
    required
        .iter()
        .all(|term| lower.contains(&term.to_lowercase()))
        && rejected
            .iter()
            .all(|term| !lower.contains(&term.to_lowercase()))
}

fn action_case(case: ActionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in case.turns.iter().enumerate() {
        api.process_conversation_turn(&request(
            case.id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            case.language,
        ))
        .expect("action discourse turn");
    }
    let response = api
        .process_conversation_turn(&request(
            case.id,
            u64::try_from(case.turns.len() + 1).expect("bounded follow turn"),
            case.follow,
            case.language,
        ))
        .expect("action group selection");
    let observed = value(&response);
    let resolved = &response.reference_resolution.resolved_semantic_text;
    let ambiguity = &response.reference_resolution.ambiguous_reference_surfaces;
    let expected = if case.ambiguous {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && !binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
            && !ambiguity.is_empty()
            && observed
                .pointer("/action_state_analysis/target_action_ids")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
    } else {
        response.disposition == ConversationTurnDispositionIR::Grounded
            && binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
            && terms_match(resolved, case.required, case.rejected)
            && observed
                .pointer("/action_state_analysis/target_action_ids")
                .and_then(Value::as_array)
                .is_some_and(|targets| targets.len() == 2)
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: group_count(&observed, "ACTION") >= 2 && expected && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn speaker_case(case: SpeakerCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut establishment_ok = true;
    let mut trace = Vec::new();
    for (index, text) in case.turns.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn"),
                text,
                case.language,
            ))
            .expect("speaker discourse turn");
        if case.establishment_turns.contains(&(index + 1)) {
            let observed = value(&response);
            establishment_ok &= binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2);
            trace.push(observed.to_string());
        }
    }
    let response = api
        .process_conversation_turn(&request(
            case.id,
            u64::try_from(case.turns.len() + 1).expect("bounded follow turn"),
            case.follow,
            case.language,
        ))
        .expect("speaker group selection");
    let observed = value(&response);
    let resolved = &response.reference_resolution.resolved_semantic_text;
    let ambiguity = &response.reference_resolution.ambiguous_reference_surfaces;
    let expected = if case.ambiguous {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && !binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && !ambiguity.is_empty()
    } else {
        response.disposition == ConversationTurnDispositionIR::Grounded
            && binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && terms_match(resolved, case.required, case.rejected)
    };
    trace.push(observed.to_string());
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: establishment_ok
            && group_count(&observed, "ATTRIBUTED_PROPOSITION") >= 2
            && expected
            && safe(&response),
        trace,
    }
}

fn main() {
    let mut rows = vec![
        action_case(ActionCase {
            id: "R35_ACTION_ORD_EN_1",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
            ],
            follow: "Where does the first pair of tasks stand?",
            required: &["cache", "queue"],
            rejected: &["worker", "server"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "ordinal_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_ORD_EN_2",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
            ],
            follow: "What is the progress of the second task pair?",
            required: &["worker", "server"],
            rejected: &["cache", "queue"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "ordinal_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_ORD_KO_1",
            turns: &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 서버를 수리해",
            ],
            follow: "첫 번째 작업 묶음은 어디까지 됐어?",
            required: &["캐시", "큐"],
            rejected: &["워커", "서버"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "ordinal_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_ORD_KO_2",
            turns: &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 서버를 수리해",
            ],
            follow: "두 번째 작업 묶음의 현황을 알려줘",
            required: &["워커", "서버"],
            rejected: &["캐시", "큐"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "ordinal_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_TOPIC_EN_1",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
            ],
            follow: "Where does the cache and queue task pair stand?",
            required: &["cache", "queue"],
            rejected: &["worker", "server"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "topic_keyed_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_TOPIC_KO_1",
            turns: &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 서버를 수리해",
            ],
            follow: "캐시와 큐 작업 묶음의 진척을 알려줘",
            required: &["캐시", "큐"],
            rejected: &["워커", "서버"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "topic_keyed_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_TOPIC_EN_2",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
                "Back to the cache topic.",
            ],
            follow: "Where does this topic's task pair stand?",
            required: &["cache", "queue"],
            rejected: &["worker", "server"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "topic_keyed_action_group_selection",
        }),
        action_case(ActionCase {
            id: "R35_ACTION_TOPIC_KO_2",
            turns: &[
                "inspect the cache and repair the queue",
                "워커를 분석하고 서버를 수리해",
                "캐시 주제로 돌아가자",
            ],
            follow: "이 주제의 작업 묶음은 어디까지 됐어?",
            required: &["cache", "queue"],
            rejected: &["워커", "서버"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "topic_keyed_action_group_selection",
        }),
    ];

    rows.extend([
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_ORD_EN_1",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob reports that the queue is blocked.",
                "Cara says that the worker is idle.",
                "Compare Alice's and Bob's reports.",
                "Compare Bob's and Cara's reports.",
            ],
            establishment_turns: &[4, 5],
            follow: "Review the first pair's reports.",
            required: &["alice", "bob", "cache", "queue"],
            rejected: &["cara", "worker"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "ordinal_overlapping_speaker_group",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_ORD_EN_2",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob reports that the queue is blocked.",
                "Cara says that the worker is idle.",
                "Compare Alice's and Bob's reports.",
                "Compare Bob's and Cara's reports.",
            ],
            establishment_turns: &[4, 5],
            follow: "Review the second pair's reports.",
            required: &["bob", "cara", "queue", "worker"],
            rejected: &["alice", "cache"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "ordinal_overlapping_speaker_group",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_ORD_KO_1",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 막혔다고 보고했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수와 누리의 보고를 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "첫 번째 묶음의 보고를 검토해",
            required: &["민수", "지수", "캐시", "큐"],
            rejected: &["누리", "워커"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "ordinal_overlapping_speaker_group",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_ORD_KO_2",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 막혔다고 보고했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수와 누리의 보고를 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "두 번째 묶음의 말을 다시 비교해",
            required: &["지수", "누리", "큐", "워커"],
            rejected: &["민수", "캐시"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "ordinal_overlapping_speaker_group",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_TOPIC_EN_1",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the cache is corrupt.",
                "Cara says that the worker is idle.",
                "Dan says that the worker is blocked.",
                "Compare Alice's and Bob's reports.",
                "Compare Cara's and Dan's reports.",
                "Back to the cache topic.",
            ],
            establishment_turns: &[5, 6],
            follow: "Compare this topic's pair of reports.",
            required: &["alice", "bob", "stale", "corrupt"],
            rejected: &["cara", "dan", "idle", "blocked"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "topic_linked_speaker_group_restoration",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_TOPIC_EN_2",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the cache is corrupt.",
                "Cara says that the worker is idle.",
                "Dan says that the worker is blocked.",
                "Compare Alice's and Bob's reports.",
                "Compare Cara's and Dan's reports.",
                "Back to the worker topic.",
            ],
            establishment_turns: &[5, 6],
            follow: "Review the pair associated with this topic.",
            required: &["cara", "dan", "idle", "blocked"],
            rejected: &["alice", "bob", "stale", "corrupt"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "topic_linked_speaker_group_restoration",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_TOPIC_KO_1",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 캐시가 손상됐다고 말했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "하람은 워커가 막혔다고 말했다.",
                "민수와 지수의 보고를 비교해",
                "누리와 하람의 보고를 비교해",
                "캐시 주제로 돌아가자",
            ],
            establishment_turns: &[5, 6],
            follow: "이 주제에 연결된 두 사람의 보고를 비교해",
            required: &["민수", "지수", "오래됐", "손상"],
            rejected: &["누리", "하람", "유휴", "막혔"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "topic_linked_speaker_group_restoration",
        }),
        speaker_case(SpeakerCase {
            id: "R35_SPEAKER_TOPIC_KO_2",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 캐시가 손상됐다고 말했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "하람은 워커가 막혔다고 말했다.",
                "민수와 지수의 보고를 비교해",
                "누리와 하람의 보고를 비교해",
                "워커 주제로 돌아가자",
            ],
            establishment_turns: &[5, 6],
            follow: "현재 주제의 화자 묶음을 다시 검토해",
            required: &["누리", "하람", "유휴", "막혔"],
            rejected: &["민수", "지수", "오래됐", "손상"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "topic_linked_speaker_group_restoration",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CORRECTION_SCOPE_EN_1",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the cache is corrupt.",
                "Compare Alice's and Bob's reports.",
                "Bob says that the worker is idle.",
                "Cara says that the worker is blocked.",
                "Compare Bob's and Cara's reports.",
                "Correction: Bob says that the cache is healthy.",
            ],
            establishment_turns: &[3, 6],
            follow: "Review the first pair's reports.",
            required: &["alice", "bob", "stale", "healthy"],
            rejected: &["corrupt", "cara", "blocked"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "overlap_correction_scoped_by_topic",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CORRECTION_SCOPE_EN_2",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the cache is corrupt.",
                "Compare Alice's and Bob's reports.",
                "Bob says that the worker is idle.",
                "Cara says that the worker is blocked.",
                "Compare Bob's and Cara's reports.",
                "Correction: Bob says that the cache is healthy.",
            ],
            establishment_turns: &[3, 6],
            follow: "Review the second pair's reports.",
            required: &["bob", "cara", "idle", "blocked"],
            rejected: &["healthy", "alice", "stale"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "overlap_correction_scoped_by_topic",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CORRECTION_SCOPE_KO_1",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 캐시가 손상됐다고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수는 워커가 유휴 상태라고 말했다.",
                "누리는 워커가 막혔다고 말했다.",
                "지수와 누리의 보고를 비교해",
                "정정하면 지수는 캐시가 정상이라고 말했다.",
            ],
            establishment_turns: &[3, 6],
            follow: "첫 번째 묶음의 보고를 검토해",
            required: &["민수", "지수", "오래됐", "정상"],
            rejected: &["손상", "누리", "막혔"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "overlap_correction_scoped_by_topic",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CORRECTION_SCOPE_KO_2",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 캐시가 손상됐다고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수는 워커가 유휴 상태라고 말했다.",
                "누리는 워커가 막혔다고 말했다.",
                "지수와 누리의 보고를 비교해",
                "정정하면 지수는 캐시가 정상이라고 말했다.",
            ],
            establishment_turns: &[3, 6],
            follow: "두 번째 묶음의 말을 비교해",
            required: &["지수", "누리", "유휴", "막혔"],
            rejected: &["정상", "민수", "오래됐"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "overlap_correction_scoped_by_topic",
        }),
        speaker_case(SpeakerCase {
            id: "R35_AMBIG_EN_1",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the queue is blocked.",
                "Cara says that the worker is idle.",
                "Compare Alice's and Bob's reports.",
                "Compare Bob's and Cara's reports.",
            ],
            establishment_turns: &[4, 5],
            follow: "Review one of the pairs' reports.",
            required: &[],
            rejected: &[],
            ambiguous: true,
            language: LanguageCodeIR::English,
            category: "multiple_group_ambiguity_fails_closed",
        }),
        speaker_case(SpeakerCase {
            id: "R35_AMBIG_EN_2",
            turns: &[
                "Nora says that the cache is stale.",
                "Omar says that the queue is blocked.",
                "Pia says that the worker is idle.",
                "Compare Nora's and Omar's claims.",
                "Compare Omar's and Pia's claims.",
            ],
            establishment_turns: &[4, 5],
            follow: "Compare either speaker pair.",
            required: &[],
            rejected: &[],
            ambiguous: true,
            language: LanguageCodeIR::English,
            category: "multiple_group_ambiguity_fails_closed",
        }),
        speaker_case(SpeakerCase {
            id: "R35_AMBIG_KO_1",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 막혔다고 말했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수와 누리의 보고를 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "두 묶음 중 하나의 보고를 검토해",
            required: &[],
            rejected: &[],
            ambiguous: true,
            language: LanguageCodeIR::Korean,
            category: "multiple_group_ambiguity_fails_closed",
        }),
        speaker_case(SpeakerCase {
            id: "R35_AMBIG_KO_2",
            turns: &[
                "하람은 캐시가 오래됐다고 말했다.",
                "누리는 큐가 막혔다고 말했다.",
                "다온은 워커가 유휴 상태라고 말했다.",
                "하람과 누리의 주장을 비교해",
                "누리와 다온의 주장을 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "어느 화자 묶음이든 비교해",
            required: &[],
            rejected: &[],
            ambiguous: true,
            language: LanguageCodeIR::Korean,
            category: "multiple_group_ambiguity_fails_closed",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CROSS_LANGUAGE_1",
            turns: &[
                "Alice says that the cache is stale.",
                "Bob says that the queue is blocked.",
                "Cara says that the worker is idle.",
                "Compare Alice's and Bob's reports.",
                "Compare Bob's and Cara's reports.",
            ],
            establishment_turns: &[4, 5],
            follow: "첫 번째 화자 묶음의 보고를 비교해",
            required: &["alice", "bob", "cache", "queue"],
            rejected: &["cara", "worker"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "cross_language_group_identity",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CROSS_LANGUAGE_2",
            turns: &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 큐가 막혔다고 말했다.",
                "누리는 워커가 유휴 상태라고 말했다.",
                "민수와 지수의 보고를 비교해",
                "지수와 누리의 보고를 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "Review the second speaker pair's reports.",
            required: &["지수", "누리", "큐", "워커"],
            rejected: &["민수", "캐시"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "cross_language_group_identity",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CROSS_LANGUAGE_3",
            turns: &[
                "Nora says that the cache is stale.",
                "Omar says that the cache is corrupt.",
                "Pia says that the worker is idle.",
                "Quin says that the worker is blocked.",
                "Compare Nora's and Omar's reports.",
                "Compare Pia's and Quin's reports.",
                "캐시 주제로 돌아가자",
            ],
            establishment_turns: &[5, 6],
            follow: "현재 주제의 두 사람 보고를 검토해",
            required: &["nora", "omar", "stale", "corrupt"],
            rejected: &["pia", "quin", "idle", "blocked"],
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "cross_language_group_identity",
        }),
        speaker_case(SpeakerCase {
            id: "R35_CROSS_LANGUAGE_4",
            turns: &[
                "하람은 캐시가 오래됐다고 말했다.",
                "누리는 캐시가 손상됐다고 말했다.",
                "다온은 워커가 유휴 상태라고 말했다.",
                "라온은 워커가 막혔다고 말했다.",
                "하람과 누리의 보고를 비교해",
                "다온과 라온의 보고를 비교해",
                "Back to the worker topic.",
            ],
            establishment_turns: &[5, 6],
            follow: "Review this topic's speaker pair.",
            required: &["다온", "라온", "유휴", "막혔"],
            rejected: &["하람", "누리", "오래됐", "손상"],
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "cross_language_group_identity",
        }),
    ]);

    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({
        "suite": "R35-RUN-0001",
        "frozen_before_product_changes": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("report"));
    if passed != report["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
