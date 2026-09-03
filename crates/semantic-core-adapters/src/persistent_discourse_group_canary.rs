//! Frozen R34-RUN-0001 persistent discourse-group diagnostic.
//!
//! This suite is frozen before the R34 product mechanism exists. It tests
//! action and attributed-speaker groups across interruptions, corrections,
//! a third speaker, pragmatic status paraphrases, and fail-closed ambiguity
//! through the public conversational API. Case text is evaluator-only.

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
enum ActionMode {
    Query,
    Report,
}

#[derive(Clone, Copy)]
struct ActionCase {
    id: &'static str,
    setup: &'static str,
    interruptions: &'static [&'static str],
    follow: &'static str,
    language: LanguageCodeIR,
    category: &'static str,
    mode: ActionMode,
}

#[derive(Clone, Copy)]
struct SpeakerCase {
    id: &'static str,
    first: &'static str,
    second: &'static str,
    establishment: &'static str,
    later_turns: &'static [&'static str],
    follow: &'static str,
    required: &'static [&'static str],
    rejected: Option<&'static str>,
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

fn action_ids(observed: &Value) -> Vec<String> {
    observed
        .pointer("/conversation_state/action_state_ledger/records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| record["action_id"].as_str().map(str::to_string))
        .collect()
}

fn targets(observed: &Value) -> Vec<String> {
    observed
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|target| target.as_str().map(str::to_string))
        .collect()
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

fn persistent_group(observed: &Value, kind: &str, size: usize) -> bool {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|group| {
                group["kind"] == kind
                    && group["member_keys"]
                        .as_array()
                        .is_some_and(|members| members.len() == size)
                    && group["semantic_authority"] == false
                    && group["external_execution_authorized"] == false
            })
        })
}

fn claims_cover(observed: &Value, ids: &[String], kind: &str) -> bool {
    observed
        .pointer("/grounded_realization/claims")
        .and_then(Value::as_array)
        .is_some_and(|claims| {
            ids.iter().all(|id| {
                claims.iter().any(|claim| {
                    claim["kind"] == kind
                        && claim["verified"] == false
                        && claim["evidence_refs"]
                            .as_array()
                            .is_some_and(|refs| refs.iter().any(|item| item == id))
                })
            })
        })
}

fn safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response.grounded_realization.validate()
        && response.grounded_realization.realized_text == response.output.text
        && response.grounded_realization.unsupported_claims == 0
        && !response.grounded_realization.semantic_authority
        && !response.grounded_realization.external_action_executed
}

fn action_case(case: ActionCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let setup = api
        .process_conversation_turn(&request(case.id, 1, case.setup, case.language))
        .expect("action setup");
    let ids = action_ids(&value(&setup));
    let mut turn = 2_u64;
    for interruption in case.interruptions {
        api.process_conversation_turn(&request(case.id, turn, interruption, case.language))
            .expect("interruption");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("action group follow-up");
    let observed = value(&response);
    let mode_pass = match case.mode {
        ActionMode::Query => {
            claims_cover(&observed, &ids, "PLAN_STATUS")
                && claims_cover(&observed, &ids, "EVIDENCE_ABSENCE")
        }
        ActionMode::Report => {
            observed
                .pointer("/action_state_analysis/detected_reports")
                .and_then(Value::as_array)
                .is_some_and(|reports| reports.len() == 2)
                && claims_cover(&observed, &ids, "LANGUAGE_REPORT")
                && !observed
                    .pointer("/grounded_realization/claims")
                    .and_then(Value::as_array)
                    .is_some_and(|claims| {
                        claims
                            .iter()
                            .any(|claim| claim["kind"] == "VERIFIED_EXECUTION")
                    })
        }
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: ids.len() == 2
            && targets(&observed) == ids
            && binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
            && persistent_group(&observed, "ACTION", 2)
            && mode_pass
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn speaker_case(case: SpeakerCase) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    api.process_conversation_turn(&request(case.id, 1, case.first, case.language))
        .expect("first speaker");
    api.process_conversation_turn(&request(case.id, 2, case.second, case.language))
        .expect("second speaker");
    let established = api
        .process_conversation_turn(&request(case.id, 3, case.establishment, case.language))
        .expect("group establishment");
    let established_value = value(&established);
    let established_ok = binding(&established_value, "PLURAL_PROPOSITION_REFERENCE", 2);
    let mut turn = 4_u64;
    for later in case.later_turns {
        api.process_conversation_turn(&request(case.id, turn, later, case.language))
            .expect("later discourse turn");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("persistent speaker group query");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: established_ok
            && binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && persistent_group(&observed, "ATTRIBUTED_PROPOSITION", 2)
            && case
                .required
                .iter()
                .all(|term| resolved.contains(&term.to_lowercase()))
            && case
                .rejected
                .is_none_or(|term| !resolved.contains(&term.to_lowercase()))
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&response),
        trace: vec![established_value.to_string(), observed.to_string()],
    }
}

fn fail_closed(id: &str, turns: &[&str], follow: &str, language: LanguageCodeIR) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, text) in turns.iter().enumerate() {
        api.process_conversation_turn(&request(
            id,
            u64::try_from(index + 1).expect("bounded turn"),
            text,
            language,
        ))
        .expect("ambiguity setup");
    }
    let response = api
        .process_conversation_turn(&request(
            id,
            u64::try_from(turns.len() + 1).expect("bounded turn"),
            follow,
            language,
        ))
        .expect("ambiguous group query");
    let observed = value(&response);
    Row {
        id: id.to_string(),
        category: "persistent_group_ambiguity_fails_closed".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && targets(&observed).is_empty()
            && !binding(&observed, "PLURAL_EVENT_REFERENCE", 2)
            && !binding(&observed, "PLURAL_PROPOSITION_REFERENCE", 2)
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    const EN_BREAKS: &[&str] = &["okay", "thanks", "hmm", "wait", "yep"];
    const KO_BREAKS: &[&str] = &["응", "고마워", "음", "잠깐", "알겠어"];
    let mut rows = vec![
        action_case(ActionCase {
            id: "R34_ACTION_BREAK_EN_1",
            setup: "inspect the relay and repair the broker",
            interruptions: EN_BREAKS,
            follow: "Where does the earlier pair of tasks stand?",
            language: LanguageCodeIR::English,
            category: "interrupted_action_group_recall",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_ACTION_BREAK_EN_2",
            setup: "repair the conduit and analyze the journal",
            interruptions: EN_BREAKS,
            follow: "Bring me up to speed on those two actions",
            language: LanguageCodeIR::English,
            category: "interrupted_action_group_recall",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_ACTION_BREAK_KO_1",
            setup: "릴레이를 확인하고 브로커를 수리해",
            interruptions: KO_BREAKS,
            follow: "아까 그 두 작업은 어디까지 됐어?",
            language: LanguageCodeIR::Korean,
            category: "interrupted_action_group_recall",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_ACTION_BREAK_KO_2",
            setup: "도관을 수리하고 저널을 분석해",
            interruptions: KO_BREAKS,
            follow: "그 두 건의 진행 상황을 알려줘",
            language: LanguageCodeIR::Korean,
            category: "interrupted_action_group_recall",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_PARAPHRASE_EN_1",
            setup: "inspect the cache and repair the queue",
            interruptions: &[],
            follow: "Where do both tasks stand?",
            language: LanguageCodeIR::English,
            category: "structural_status_query_paraphrase",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_PARAPHRASE_EN_2",
            setup: "repair the parser and inspect the worker",
            interruptions: &[],
            follow: "Give me a progress update for the pair",
            language: LanguageCodeIR::English,
            category: "structural_status_query_paraphrase",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_PARAPHRASE_KO_1",
            setup: "캐시를 확인하고 큐를 수리해",
            interruptions: &[],
            follow: "두 건은 지금 어디까지 됐어?",
            language: LanguageCodeIR::Korean,
            category: "structural_status_query_paraphrase",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_PARAPHRASE_KO_2",
            setup: "파서를 수리하고 워커를 확인해",
            interruptions: &[],
            follow: "양쪽 작업의 진척을 정리해줘",
            language: LanguageCodeIR::Korean,
            category: "structural_status_query_paraphrase",
            mode: ActionMode::Query,
        }),
        action_case(ActionCase {
            id: "R34_REPORT_EN_1",
            setup: "inspect the cache and repair the queue",
            interruptions: &[],
            follow: "The pair is wrapped up",
            language: LanguageCodeIR::English,
            category: "compositional_completion_report_paraphrase",
            mode: ActionMode::Report,
        }),
        action_case(ActionCase {
            id: "R34_REPORT_EN_2",
            setup: "repair the parser and inspect the worker",
            interruptions: &[],
            follow: "I took care of both tasks",
            language: LanguageCodeIR::English,
            category: "compositional_completion_report_paraphrase",
            mode: ActionMode::Report,
        }),
        action_case(ActionCase {
            id: "R34_REPORT_KO_1",
            setup: "캐시를 확인하고 큐를 수리해",
            interruptions: &[],
            follow: "두 건 다 마무리했어",
            language: LanguageCodeIR::Korean,
            category: "compositional_completion_report_paraphrase",
            mode: ActionMode::Report,
        }),
        action_case(ActionCase {
            id: "R34_REPORT_KO_2",
            setup: "파서를 수리하고 워커를 확인해",
            interruptions: &[],
            follow: "양쪽 작업을 처리했어",
            language: LanguageCodeIR::Korean,
            category: "compositional_completion_report_paraphrase",
            mode: ActionMode::Report,
        }),
    ];

    rows.extend([
        speaker_case(SpeakerCase {
            id: "R34_SPEAKER_BREAK_EN_1",
            first: "Alice says that the relay is unstable.",
            second: "Bob reports that the broker is saturated.",
            establishment: "Compare their reports",
            later_turns: EN_BREAKS,
            follow: "What did those two say?",
            required: &["alice", "bob"],
            rejected: None,
            language: LanguageCodeIR::English,
            category: "interrupted_speaker_group_recall",
        }),
        speaker_case(SpeakerCase {
            id: "R34_SPEAKER_BREAK_EN_2",
            first: "Nora believes that the cache is stale.",
            second: "Omar says that the worker is blocked.",
            establishment: "Analyze both statements",
            later_turns: EN_BREAKS,
            follow: "Review the earlier pair of claims",
            required: &["nora", "omar"],
            rejected: None,
            language: LanguageCodeIR::English,
            category: "interrupted_speaker_group_recall",
        }),
        speaker_case(SpeakerCase {
            id: "R34_SPEAKER_BREAK_KO_1",
            first: "민수는 릴레이가 불안정하다고 말했다.",
            second: "지수는 브로커가 포화됐다고 보고했다.",
            establishment: "그들의 보고를 비교해",
            later_turns: KO_BREAKS,
            follow: "아까 그 둘이 한 말을 검토해",
            required: &["민수", "지수"],
            rejected: None,
            language: LanguageCodeIR::Korean,
            category: "interrupted_speaker_group_recall",
        }),
        speaker_case(SpeakerCase {
            id: "R34_SPEAKER_BREAK_KO_2",
            first: "하람은 캐시가 오래됐다고 믿었다.",
            second: "누리는 워커가 막혔다고 말했다.",
            establishment: "두 사람의 주장을 분석해",
            later_turns: KO_BREAKS,
            follow: "그 두 사람의 말을 다시 살펴봐",
            required: &["하람", "누리"],
            rejected: None,
            language: LanguageCodeIR::Korean,
            category: "interrupted_speaker_group_recall",
        }),
        speaker_case(SpeakerCase {
            id: "R34_THIRD_EN_1",
            first: "Alice says that the relay is unstable.",
            second: "Bob reports that the broker is saturated.",
            establishment: "Compare their reports",
            later_turns: &["Cara says that the journal is incomplete."],
            follow: "Review those two reports",
            required: &["alice", "bob"],
            rejected: Some("cara"),
            language: LanguageCodeIR::English,
            category: "established_group_survives_third_speaker",
        }),
        speaker_case(SpeakerCase {
            id: "R34_THIRD_EN_2",
            first: "Nora believes that the cache is stale.",
            second: "Omar says that the worker is blocked.",
            establishment: "Analyze both statements",
            later_turns: &["Pia reports that the queue is empty."],
            follow: "Compare the earlier pair of statements",
            required: &["nora", "omar"],
            rejected: Some("pia"),
            language: LanguageCodeIR::English,
            category: "established_group_survives_third_speaker",
        }),
        speaker_case(SpeakerCase {
            id: "R34_THIRD_KO_1",
            first: "민수는 릴레이가 불안정하다고 말했다.",
            second: "지수는 브로커가 포화됐다고 보고했다.",
            establishment: "그들의 보고를 비교해",
            later_turns: &["수아는 저널이 불완전하다고 말했다."],
            follow: "아까 그 두 사람의 보고를 검토해",
            required: &["민수", "지수"],
            rejected: Some("수아"),
            language: LanguageCodeIR::Korean,
            category: "established_group_survives_third_speaker",
        }),
        speaker_case(SpeakerCase {
            id: "R34_THIRD_KO_2",
            first: "하람은 캐시가 오래됐다고 믿었다.",
            second: "누리는 워커가 막혔다고 말했다.",
            establishment: "두 사람의 주장을 분석해",
            later_turns: &["마루는 큐가 비었다고 보고했다."],
            follow: "앞서 묶은 두 사람의 말을 비교해",
            required: &["하람", "누리"],
            rejected: Some("마루"),
            language: LanguageCodeIR::Korean,
            category: "established_group_survives_third_speaker",
        }),
        speaker_case(SpeakerCase {
            id: "R34_CORRECT_GROUP_EN_1",
            first: "Alice says that the relay is unstable.",
            second: "Bob reports that the broker is saturated.",
            establishment: "Compare their reports",
            later_turns: &[
                "Correction: Alice says that the relay is stable.",
                "okay",
                "thanks",
            ],
            follow: "Review those two reports again",
            required: &["stable", "saturated"],
            rejected: Some("unstable"),
            language: LanguageCodeIR::English,
            category: "persistent_group_tracks_source_correction",
        }),
        speaker_case(SpeakerCase {
            id: "R34_CORRECT_GROUP_EN_2",
            first: "Nora believes that the cache is stale.",
            second: "Omar says that the worker is blocked.",
            establishment: "Analyze both statements",
            later_turns: &[
                "Actually, Nora believes that the cache is healthy.",
                "hmm",
                "wait",
            ],
            follow: "Revisit the earlier pair of claims",
            required: &["healthy", "blocked"],
            rejected: Some("stale"),
            language: LanguageCodeIR::English,
            category: "persistent_group_tracks_source_correction",
        }),
        speaker_case(SpeakerCase {
            id: "R34_CORRECT_GROUP_KO_1",
            first: "민수는 릴레이가 불안정하다고 말했다.",
            second: "지수는 브로커가 포화됐다고 보고했다.",
            establishment: "그들의 보고를 비교해",
            later_turns: &[
                "정정하면 민수는 릴레이가 안정적이라고 말했다.",
                "응",
                "고마워",
            ],
            follow: "아까 그 둘의 보고를 다시 검토해",
            required: &["안정", "포화"],
            rejected: Some("불안정"),
            language: LanguageCodeIR::Korean,
            category: "persistent_group_tracks_source_correction",
        }),
        speaker_case(SpeakerCase {
            id: "R34_CORRECT_GROUP_KO_2",
            first: "하람은 캐시가 오래됐다고 믿었다.",
            second: "누리는 워커가 막혔다고 말했다.",
            establishment: "두 사람의 주장을 분석해",
            later_turns: &["사실은 하람은 캐시가 정상이라고 믿었다.", "음", "잠깐"],
            follow: "앞서 묶은 두 사람의 말을 다시 비교해",
            required: &["정상", "막혔"],
            rejected: Some("오래됐"),
            language: LanguageCodeIR::Korean,
            category: "persistent_group_tracks_source_correction",
        }),
    ]);

    rows.extend([
        fail_closed(
            "R34_AMBIG_SPEAKER_EN_1",
            &[
                "Alice says that the cache is stale.",
                "Bob says that the worker is blocked.",
                "Cara says that the queue is empty.",
            ],
            "Compare their claims",
            LanguageCodeIR::English,
        ),
        fail_closed(
            "R34_AMBIG_SPEAKER_KO_1",
            &[
                "민수는 캐시가 오래됐다고 말했다.",
                "지수는 워커가 막혔다고 말했다.",
                "수아는 큐가 비었다고 말했다.",
            ],
            "그들의 주장을 비교해",
            LanguageCodeIR::Korean,
        ),
        fail_closed(
            "R34_AMBIG_ACTION_EN",
            &["inspect the cache, repair the queue, and analyze the worker"],
            "Where do the two tasks stand?",
            LanguageCodeIR::English,
        ),
        fail_closed(
            "R34_AMBIG_ACTION_KO",
            &["캐시를 확인하고 큐를 수리한 뒤 워커를 분석해"],
            "두 건은 어디까지 됐어?",
            LanguageCodeIR::Korean,
        ),
    ]);

    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R34-RUN-0001",
            "frozen_before_product_changes":true,
            "external_llm_calls":0,
            "local_teacher_calls":0,
            "network_calls":0,
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
