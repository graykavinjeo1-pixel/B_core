//! Frozen R34-TRANSFER-0001 held-out persistent-group transfer suite.
//!
//! Do not semantically execute this binary until R34-RUN-0001 passes.

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
    setup: &'static str,
    breaks: &'static [&'static str],
    follow: &'static str,
    language: LanguageCodeIR,
    report: bool,
}

#[derive(Clone, Copy)]
struct SpeakerCase {
    id: &'static str,
    first: &'static str,
    second: &'static str,
    establish: &'static str,
    later: &'static [&'static str],
    follow: &'static str,
    required: &'static [&'static str],
    rejected: Option<&'static str>,
    language: LanguageCodeIR,
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

fn ids(observed: &Value, pointer: &str, field: &str) -> Vec<String> {
    observed
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item[field].as_str().map(str::to_string))
        .collect()
}

fn binding(observed: &Value, kind: &str) -> bool {
    observed
        .pointer("/reference_resolution/discourse_bindings")
        .and_then(Value::as_array)
        .is_some_and(|bindings| {
            bindings.iter().any(|item| {
                item["kind"] == kind
                    && item["referent_ids"]
                        .as_array()
                        .is_some_and(|members| members.len() == 2)
            })
        })
}

fn group(observed: &Value, kind: &str) -> bool {
    observed
        .pointer("/conversation_state/active_discourse_groups")
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|item| {
                item["kind"] == kind
                    && item["member_keys"]
                        .as_array()
                        .is_some_and(|members| members.len() == 2)
                    && item["semantic_authority"] == false
                    && item["external_execution_authorized"] == false
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
        .expect("setup");
    let action_ids = ids(
        &value(&setup),
        "/conversation_state/action_state_ledger/records",
        "action_id",
    );
    let mut turn = 2_u64;
    for pause in case.breaks {
        api.process_conversation_turn(&request(case.id, turn, pause, case.language))
            .expect("pause");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("held-out action follow");
    let observed = value(&response);
    let targets = observed
        .pointer("/action_state_analysis/target_action_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let reports = observed
        .pointer("/action_state_analysis/detected_reports")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    Row {
        id: case.id.to_string(),
        category: if case.report {
            "heldout_group_completion_paraphrase"
        } else if case.breaks.is_empty() {
            "heldout_status_query_paraphrase"
        } else {
            "heldout_interrupted_action_group"
        }
        .to_string(),
        pass: action_ids.len() == 2
            && targets == action_ids
            && binding(&observed, "PLURAL_EVENT_REFERENCE")
            && group(&observed, "ACTION")
            && (!case.report || reports == 2)
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
        .process_conversation_turn(&request(case.id, 3, case.establish, case.language))
        .expect("establish group");
    let established_ok = binding(&value(&established), "PLURAL_PROPOSITION_REFERENCE");
    let mut turn = 4_u64;
    for later in case.later {
        api.process_conversation_turn(&request(case.id, turn, later, case.language))
            .expect("later turn");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("held-out speaker follow");
    let observed = value(&response);
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    Row {
        id: case.id.to_string(),
        category: "heldout_persistent_speaker_group".to_string(),
        pass: established_ok
            && binding(&observed, "PLURAL_PROPOSITION_REFERENCE")
            && group(&observed, "ATTRIBUTED_PROPOSITION")
            && case
                .required
                .iter()
                .all(|term| resolved.contains(&term.to_lowercase()))
            && case
                .rejected
                .is_none_or(|term| !resolved.contains(&term.to_lowercase()))
            && response.disposition == ConversationTurnDispositionIR::Grounded
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn ambiguity(id: &str, turns: &[&str], follow: &str, language: LanguageCodeIR) -> Row {
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
        .expect("ambiguity follow");
    let observed = value(&response);
    Row {
        id: id.to_string(),
        category: "heldout_group_ambiguity".to_string(),
        pass: response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && !binding(&observed, "PLURAL_EVENT_REFERENCE")
            && !binding(&observed, "PLURAL_PROPOSITION_REFERENCE")
            && safe(&response),
        trace: vec![observed.to_string()],
    }
}

fn main() {
    const EN_PAUSE: &[&str] = &["yep", "thanks", "umm", "hold", "okay"];
    const KO_PAUSE: &[&str] = &["네", "감사", "흠", "잠시", "응"];
    let mut rows = vec![
        action_case(ActionCase {
            id: "R34X_ACTION_EN_1",
            setup: "analyze the gateway and repair the scheduler",
            breaks: EN_PAUSE,
            follow: "How is that earlier pair coming along?",
            language: LanguageCodeIR::English,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_ACTION_EN_2",
            setup: "inspect the registry and repair the channel",
            breaks: EN_PAUSE,
            follow: "Catch me up on those two tasks",
            language: LanguageCodeIR::English,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_ACTION_KO_1",
            setup: "게이트웨이를 분석하고 스케줄러를 수리해",
            breaks: KO_PAUSE,
            follow: "앞서 말한 두 건은 어떻게 되어가?",
            language: LanguageCodeIR::Korean,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_ACTION_KO_2",
            setup: "레지스트리를 확인하고 채널을 수리해",
            breaks: KO_PAUSE,
            follow: "그 작업 둘의 현황을 말해줘",
            language: LanguageCodeIR::Korean,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_QUERY_EN_1",
            setup: "analyze the gateway and repair the scheduler",
            breaks: &[],
            follow: "How are the two tasks progressing?",
            language: LanguageCodeIR::English,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_QUERY_EN_2",
            setup: "inspect the registry and repair the channel",
            breaks: &[],
            follow: "What's the progress on the pair?",
            language: LanguageCodeIR::English,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_QUERY_KO_1",
            setup: "게이트웨이를 분석하고 스케줄러를 수리해",
            breaks: &[],
            follow: "두 작업의 진행은 어때?",
            language: LanguageCodeIR::Korean,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_QUERY_KO_2",
            setup: "레지스트리를 확인하고 채널을 수리해",
            breaks: &[],
            follow: "양쪽은 어디까지 진행됐어?",
            language: LanguageCodeIR::Korean,
            report: false,
        }),
        action_case(ActionCase {
            id: "R34X_REPORT_EN_1",
            setup: "analyze the gateway and repair the scheduler",
            breaks: &[],
            follow: "Both jobs have been taken care of",
            language: LanguageCodeIR::English,
            report: true,
        }),
        action_case(ActionCase {
            id: "R34X_REPORT_EN_2",
            setup: "inspect the registry and repair the channel",
            breaks: &[],
            follow: "I'm through with the pair",
            language: LanguageCodeIR::English,
            report: true,
        }),
        action_case(ActionCase {
            id: "R34X_REPORT_KO_1",
            setup: "게이트웨이를 분석하고 스케줄러를 수리해",
            breaks: &[],
            follow: "두 건 모두 처리 끝냈어",
            language: LanguageCodeIR::Korean,
            report: true,
        }),
        action_case(ActionCase {
            id: "R34X_REPORT_KO_2",
            setup: "레지스트리를 확인하고 채널을 수리해",
            breaks: &[],
            follow: "작업 둘 다 마무리됐어",
            language: LanguageCodeIR::Korean,
            report: true,
        }),
        speaker_case(SpeakerCase {
            id: "R34X_SPEAKER_EN_1",
            first: "Iris says that the gateway is unstable.",
            second: "Jules reports that the scheduler is saturated.",
            establish: "Compare their reports",
            later: EN_PAUSE,
            follow: "Summarize what that pair said",
            required: &["iris", "jules"],
            rejected: None,
            language: LanguageCodeIR::English,
        }),
        speaker_case(SpeakerCase {
            id: "R34X_SPEAKER_KO_1",
            first: "이현은 게이트웨이가 불안정하다고 말했다.",
            second: "주원은 스케줄러가 포화됐다고 보고했다.",
            establish: "그들의 보고를 비교해",
            later: KO_PAUSE,
            follow: "그 둘이 말한 내용을 요약해",
            required: &["이현", "주원"],
            rejected: None,
            language: LanguageCodeIR::Korean,
        }),
        speaker_case(SpeakerCase {
            id: "R34X_SPEAKER_EN_2",
            first: "Iris says that the gateway is unstable.",
            second: "Jules reports that the scheduler is saturated.",
            establish: "Compare their reports",
            later: &[
                "Mara says that the registry is incomplete.",
                "Correction: Iris says that the gateway is stable.",
            ],
            follow: "Recheck the original pair's reports",
            required: &["stable", "saturated"],
            rejected: Some("unstable"),
            language: LanguageCodeIR::English,
        }),
        speaker_case(SpeakerCase {
            id: "R34X_SPEAKER_KO_2",
            first: "이현은 게이트웨이가 불안정하다고 말했다.",
            second: "주원은 스케줄러가 포화됐다고 보고했다.",
            establish: "그들의 보고를 비교해",
            later: &[
                "마루는 레지스트리가 불완전하다고 말했다.",
                "정정하면 이현은 게이트웨이가 안정적이라고 말했다.",
            ],
            follow: "처음 묶은 두 사람의 보고를 다시 확인해",
            required: &["안정", "포화"],
            rejected: Some("불안정"),
            language: LanguageCodeIR::Korean,
        }),
    ];
    rows.extend([
        ambiguity(
            "R34X_AMBIG_EN_1",
            &[
                "Iris says that the gateway is unstable.",
                "Jules says that the scheduler is saturated.",
                "Mara says that the registry is incomplete.",
            ],
            "Review those two claims",
            LanguageCodeIR::English,
        ),
        ambiguity(
            "R34X_AMBIG_KO_1",
            &[
                "이현은 게이트웨이가 불안정하다고 말했다.",
                "주원은 스케줄러가 포화됐다고 말했다.",
                "마루는 레지스트리가 불완전하다고 말했다.",
            ],
            "그 두 사람의 주장을 검토해",
            LanguageCodeIR::Korean,
        ),
        ambiguity(
            "R34X_UNBOUND_EN",
            &[],
            "Catch me up on the pair",
            LanguageCodeIR::English,
        ),
        ambiguity(
            "R34X_UNBOUND_KO",
            &[],
            "양쪽 작업의 현황을 알려줘",
            LanguageCodeIR::Korean,
        ),
    ]);
    let passed = rows.iter().filter(|row| row.pass).count();
    let total = rows.len();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "suite":"R34-TRANSFER-0001",
            "held_out_until_after_diagnostic_pass":true,
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
