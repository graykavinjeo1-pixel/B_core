//! Frozen R35-TRANSFER-0001 held-out transfer suite.
//!
//! This file is frozen with the R35 diagnostic and must remain unopened by the
//! product repair loop until the diagnostic reaches 28/28. It varies names,
//! predicates, ordinal/topic paraphrases, correction wording, and ambiguity.

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
    turns: &'static [&'static str],
    establishment_turns: &'static [usize],
    follow: &'static str,
    required: &'static [&'static str],
    rejected: &'static [&'static str],
    kind: &'static str,
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
        .filter(|group| group["kind"] == kind && group["semantic_authority"] == false)
        .count()
}

fn run(case: Case) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut establishment_ok = true;
    for (index, text) in case.turns.iter().enumerate() {
        let response = api
            .process_conversation_turn(&request(
                case.id,
                u64::try_from(index + 1).expect("bounded turn"),
                text,
                case.language,
            ))
            .expect("held-out discourse turn");
        if case.establishment_turns.contains(&(index + 1)) {
            establishment_ok &= binding(&value(&response), "PLURAL_PROPOSITION_REFERENCE", 2);
        }
    }
    let response = api
        .process_conversation_turn(&request(
            case.id,
            u64::try_from(case.turns.len() + 1).expect("bounded follow"),
            case.follow,
            case.language,
        ))
        .expect("held-out selection");
    let observed = value(&response);
    let lower = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let kind = if case.kind == "ACTION" {
        "PLURAL_EVENT_REFERENCE"
    } else {
        "PLURAL_PROPOSITION_REFERENCE"
    };
    let semantic_match = case
        .required
        .iter()
        .all(|term| lower.contains(&term.to_lowercase()))
        && case
            .rejected
            .iter()
            .all(|term| !lower.contains(&term.to_lowercase()));
    let expected = if case.ambiguous {
        response.disposition == ConversationTurnDispositionIR::ClarificationRequired
            && !binding(&observed, kind, 2)
            && !response
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty()
    } else {
        response.disposition == ConversationTurnDispositionIR::Grounded
            && binding(&observed, kind, 2)
            && semantic_match
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass: establishment_ok
            && group_count(&observed, case.kind) >= 2
            && expected
            && response.grounded_realization.validate()
            && response.grounded_realization.unsupported_claims == 0
            && !response.grounded_realization.semantic_authority
            && !response.grounded_realization.external_action_executed,
        trace: vec![observed.to_string()],
    }
}

fn main() {
    let rows = vec![
        run(Case {
            id: "R35X_ACTION_EN_1",
            turns: &[
                "analyze the gateway and repair the scheduler",
                "inspect the journal and repair the broker",
            ],
            establishment_turns: &[],
            follow: "How is the earliest task pair coming along?",
            required: &["gateway", "scheduler"],
            rejected: &["journal", "broker"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_action_ordinal",
        }),
        run(Case {
            id: "R35X_ACTION_EN_2",
            turns: &[
                "analyze the gateway and repair the scheduler",
                "inspect the journal and repair the broker",
            ],
            establishment_turns: &[],
            follow: "Catch me up on the most recent task pair.",
            required: &["journal", "broker"],
            rejected: &["gateway", "scheduler"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_action_ordinal",
        }),
        run(Case {
            id: "R35X_ACTION_KO_1",
            turns: &[
                "게이트웨이를 분석하고 스케줄러를 수리해",
                "저널을 확인하고 브로커를 수리해",
            ],
            establishment_turns: &[],
            follow: "먼저 묶은 작업 둘의 진행은 어때?",
            required: &["게이트웨이", "스케줄러"],
            rejected: &["저널", "브로커"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_action_ordinal",
        }),
        run(Case {
            id: "R35X_ACTION_KO_2",
            turns: &[
                "게이트웨이를 분석하고 스케줄러를 수리해",
                "저널을 확인하고 브로커를 수리해",
            ],
            establishment_turns: &[],
            follow: "나중 작업 둘의 현황을 알려줘",
            required: &["저널", "브로커"],
            rejected: &["게이트웨이", "스케줄러"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_action_ordinal",
        }),
        run(Case {
            id: "R35X_TOPIC_ACTION_EN_1",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
                "Return to cache.",
            ],
            establishment_turns: &[],
            follow: "What is the status of the task pair for this topic?",
            required: &["cache", "queue"],
            rejected: &["worker", "server"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_topic_action",
        }),
        run(Case {
            id: "R35X_TOPIC_ACTION_EN_2",
            turns: &[
                "inspect the cache and repair the queue",
                "analyze the worker and repair the server",
                "Return to worker.",
            ],
            establishment_turns: &[],
            follow: "How are the jobs linked to this topic progressing?",
            required: &["worker", "server"],
            rejected: &["cache", "queue"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_topic_action",
        }),
        run(Case {
            id: "R35X_TOPIC_ACTION_KO_1",
            turns: &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 서버를 수리해",
                "캐시로 돌아가자",
            ],
            establishment_turns: &[],
            follow: "현재 주제에 속한 작업 둘의 진척은?",
            required: &["캐시", "큐"],
            rejected: &["워커", "서버"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_topic_action",
        }),
        run(Case {
            id: "R35X_TOPIC_ACTION_KO_2",
            turns: &[
                "캐시를 확인하고 큐를 수리해",
                "워커를 분석하고 서버를 수리해",
                "워커로 돌아가자",
            ],
            establishment_turns: &[],
            follow: "이 주제와 연결된 작업 두 건의 현황은?",
            required: &["워커", "서버"],
            rejected: &["캐시", "큐"],
            kind: "ACTION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_topic_action",
        }),
        run(Case {
            id: "R35X_SPEAKER_EN_1",
            turns: &[
                "Eli says that the relay is unstable.",
                "Faye says that the broker is saturated.",
                "Gus says that the journal is incomplete.",
                "Compare Eli's and Faye's claims.",
                "Compare Faye's and Gus's claims.",
            ],
            establishment_turns: &[4, 5],
            follow: "Revisit the earliest speaker pair.",
            required: &["eli", "faye", "relay", "broker"],
            rejected: &["gus", "journal"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_speaker_ordinal",
        }),
        run(Case {
            id: "R35X_SPEAKER_EN_2",
            turns: &[
                "Eli says that the relay is unstable.",
                "Faye says that the broker is saturated.",
                "Gus says that the journal is incomplete.",
                "Compare Eli's and Faye's claims.",
                "Compare Faye's and Gus's claims.",
            ],
            establishment_turns: &[4, 5],
            follow: "Revisit the newer speaker pair.",
            required: &["faye", "gus", "broker", "journal"],
            rejected: &["eli", "relay"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_speaker_ordinal",
        }),
        run(Case {
            id: "R35X_SPEAKER_KO_1",
            turns: &[
                "가람은 릴레이가 불안정하다고 말했다.",
                "나래는 브로커가 포화됐다고 말했다.",
                "다솜은 저널이 불완전하다고 말했다.",
                "가람과 나래의 주장을 비교해",
                "나래와 다솜의 주장을 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "먼저 만든 화자 묶음을 다시 봐",
            required: &["가람", "나래", "릴레이", "브로커"],
            rejected: &["다솜", "저널"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_speaker_ordinal",
        }),
        run(Case {
            id: "R35X_SPEAKER_KO_2",
            turns: &[
                "가람은 릴레이가 불안정하다고 말했다.",
                "나래는 브로커가 포화됐다고 말했다.",
                "다솜은 저널이 불완전하다고 말했다.",
                "가람과 나래의 주장을 비교해",
                "나래와 다솜의 주장을 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "최근 화자 묶음의 보고를 검토해",
            required: &["나래", "다솜", "브로커", "저널"],
            rejected: &["가람", "릴레이"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_speaker_ordinal",
        }),
        run(Case {
            id: "R35X_CORRECT_EN_1",
            turns: &[
                "Iris says that the cache is stale.",
                "Jude says that the cache is corrupt.",
                "Compare Iris's and Jude's reports.",
                "Jude says that the worker is idle.",
                "Kira says that the worker is blocked.",
                "Compare Jude's and Kira's reports.",
                "Actually, Jude says that the cache is healthy.",
            ],
            establishment_turns: &[3, 6],
            follow: "Revisit the older pair's reports.",
            required: &["iris", "jude", "stale", "healthy"],
            rejected: &["corrupt", "kira", "blocked"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_topic_scoped_correction",
        }),
        run(Case {
            id: "R35X_CORRECT_EN_2",
            turns: &[
                "Iris says that the cache is stale.",
                "Jude says that the cache is corrupt.",
                "Compare Iris's and Jude's reports.",
                "Jude says that the worker is idle.",
                "Kira says that the worker is blocked.",
                "Compare Jude's and Kira's reports.",
                "Actually, Jude says that the cache is healthy.",
            ],
            establishment_turns: &[3, 6],
            follow: "Revisit the later pair's reports.",
            required: &["jude", "kira", "idle", "blocked"],
            rejected: &["healthy", "iris", "stale"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::English,
            category: "heldout_topic_scoped_correction",
        }),
        run(Case {
            id: "R35X_CORRECT_KO_1",
            turns: &[
                "라온은 캐시가 오래됐다고 말했다.",
                "마루는 캐시가 손상됐다고 말했다.",
                "라온과 마루의 보고를 비교해",
                "마루는 워커가 유휴 상태라고 말했다.",
                "보라는 워커가 막혔다고 말했다.",
                "마루와 보라의 보고를 비교해",
                "사실은 마루는 캐시가 정상이라고 말했다.",
            ],
            establishment_turns: &[3, 6],
            follow: "앞 화자 묶음의 보고를 검토해",
            required: &["라온", "마루", "오래됐", "정상"],
            rejected: &["손상", "보라", "막혔"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_topic_scoped_correction",
        }),
        run(Case {
            id: "R35X_CORRECT_KO_2",
            turns: &[
                "라온은 캐시가 오래됐다고 말했다.",
                "마루는 캐시가 손상됐다고 말했다.",
                "라온과 마루의 보고를 비교해",
                "마루는 워커가 유휴 상태라고 말했다.",
                "보라는 워커가 막혔다고 말했다.",
                "마루와 보라의 보고를 비교해",
                "사실은 마루는 캐시가 정상이라고 말했다.",
            ],
            establishment_turns: &[3, 6],
            follow: "뒤 화자 묶음의 말을 비교해",
            required: &["마루", "보라", "유휴", "막혔"],
            rejected: &["정상", "라온", "오래됐"],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: false,
            language: LanguageCodeIR::Korean,
            category: "heldout_topic_scoped_correction",
        }),
        run(Case {
            id: "R35X_AMBIG_EN_1",
            turns: &[
                "Lena says that the cache is stale.",
                "Milo says that the queue is blocked.",
                "Nia says that the worker is idle.",
                "Compare Lena's and Milo's reports.",
                "Compare Milo's and Nia's reports.",
            ],
            establishment_turns: &[4, 5],
            follow: "Choose one speaker pair to review.",
            required: &[],
            rejected: &[],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: true,
            language: LanguageCodeIR::English,
            category: "heldout_multigroup_ambiguity",
        }),
        run(Case {
            id: "R35X_AMBIG_EN_2",
            turns: &[
                "Orin says that the cache is stale.",
                "Pru says that the queue is blocked.",
                "Rae says that the worker is idle.",
                "Compare Orin's and Pru's claims.",
                "Compare Pru's and Rae's claims.",
            ],
            establishment_turns: &[4, 5],
            follow: "Review either of the two speaker groups.",
            required: &[],
            rejected: &[],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: true,
            language: LanguageCodeIR::English,
            category: "heldout_multigroup_ambiguity",
        }),
        run(Case {
            id: "R35X_AMBIG_KO_1",
            turns: &[
                "사라는 캐시가 오래됐다고 말했다.",
                "아람은 큐가 막혔다고 말했다.",
                "여울은 워커가 유휴 상태라고 말했다.",
                "사라와 아람의 보고를 비교해",
                "아람과 여울의 보고를 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "두 화자 조 중 아무거나 골라 검토해",
            required: &[],
            rejected: &[],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: true,
            language: LanguageCodeIR::Korean,
            category: "heldout_multigroup_ambiguity",
        }),
        run(Case {
            id: "R35X_AMBIG_KO_2",
            turns: &[
                "이든은 캐시가 오래됐다고 말했다.",
                "자온은 큐가 막혔다고 말했다.",
                "초롱은 워커가 유휴 상태라고 말했다.",
                "이든과 자온의 주장을 비교해",
                "자온과 초롱의 주장을 비교해",
            ],
            establishment_turns: &[4, 5],
            follow: "어느 한 화자 그룹을 검토해",
            required: &[],
            rejected: &[],
            kind: "ATTRIBUTED_PROPOSITION",
            ambiguous: true,
            language: LanguageCodeIR::Korean,
            category: "heldout_multigroup_ambiguity",
        }),
    ];
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = json!({
        "suite": "R35-TRANSFER-0001",
        "held_out_until_after_diagnostic_pass": true,
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
