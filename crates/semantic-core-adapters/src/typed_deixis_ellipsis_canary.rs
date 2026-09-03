//! Frozen R28-RUN-0001 typed deixis and ellipsis diagnostic.
//!
//! The public conversation API is the only system under test. The suite uses
//! structural possessive, demonstrative, zero-argument, and predicate ellipsis
//! expectations rather than whole-sentence dispatch.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
enum ExpectedMode {
    Grounded,
    Deferred,
    Clarify,
}

#[derive(Clone, Copy)]
struct Case<'a> {
    id: &'a str,
    category: &'a str,
    setup: Option<&'a str>,
    bridges: &'a [&'a str],
    follow: &'a str,
    target: &'a str,
    rejected: &'a str,
    expected_binding: &'a str,
    mode: ExpectedMode,
    language: LanguageCodeIR,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    trace: Vec<String>,
    pass: bool,
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
        max_plan_steps: 20,
    }
}

fn binding_kind(response: &semantic_core_adapters::ConversationTurnResponseIR) -> String {
    response
        .reference_resolution
        .discourse_bindings
        .last()
        .and_then(|binding| serde_json::to_value(binding).ok())
        .and_then(|binding| binding["kind"].as_str().map(ToString::to_string))
        .unwrap_or_default()
}

fn binding_is_safe(response: &semantic_core_adapters::ConversationTurnResponseIR) -> bool {
    response
        .reference_resolution
        .discourse_bindings
        .last()
        .is_some_and(|binding| {
            binding
                .evidence
                .contains(&"SEMANTIC_AUTHORITY:false".to_string())
                && binding
                    .evidence
                    .contains(&"EXTERNAL_EXECUTION_AUTHORIZED:false".to_string())
        })
}

fn selected_subject(response: &semantic_core_adapters::ConversationTurnResponseIR) -> String {
    response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .map(|candidate| candidate.subject.to_lowercase())
        .unwrap_or_default()
}

fn deferred_subject(response: &semantic_core_adapters::ConversationTurnResponseIR) -> String {
    response
        .conversation_state
        .deferred_action_commitments
        .last()
        .map(|commitment| commitment.action.subject.to_lowercase())
        .unwrap_or_default()
}

fn run_case(case: Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let mut turn = 1_u64;
    if let Some(setup) = case.setup {
        api.process_conversation_turn(&request(case.id, turn, setup, case.language))
            .expect("setup turn");
        turn += 1;
    }
    for bridge in case.bridges {
        api.process_conversation_turn(&request(case.id, turn, bridge, case.language))
            .expect("bridge turn");
        turn += 1;
    }
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("follow turn");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let subject = selected_subject(&response);
    let deferred = deferred_subject(&response);
    let binding = binding_kind(&response);
    let target = case.target.to_lowercase();
    let rejected = case.rejected.to_lowercase();
    let target_resolved = target.is_empty() || resolved.contains(&target);
    let rejected_absent = rejected.is_empty() || !resolved.contains(&rejected);
    let pass = match case.mode {
        ExpectedMode::Grounded => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.grounded_response.is_some()
                && binding == case.expected_binding
                && binding_is_safe(&response)
                && target_resolved
                && rejected_absent
                && subject.contains(&target)
        }
        ExpectedMode::Deferred => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && binding == case.expected_binding
                && binding_is_safe(&response)
                && target_resolved
                && rejected_absent
                && deferred.contains(&target)
                && response
                    .conversation_state
                    .deferred_action_commitments
                    .last()
                    .is_some_and(|commitment| commitment.is_pending())
        }
        ExpectedMode::Clarify => {
            response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && response.grounded_response.is_none()
                && response.reference_resolution.resolved_semantic_text == case.follow
                && response
                    .reference_resolution
                    .ambiguous_reference_surfaces
                    .iter()
                    .any(|surface| surface == case.expected_binding)
        }
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        trace: vec![
            format!("resolved={resolved:?}"),
            format!("subject={subject:?}"),
            format!("deferred={deferred:?}"),
            format!("binding={binding:?}"),
            format!("disposition={:?}", response.disposition),
        ],
        pass,
    }
}

fn main() {
    let cases = [
        Case {
            id: "R28_POS_1",
            category: "possessive_focus",
            setup: Some("analyze the cache and repair the queue"),
            bridges: &[],
            follow: "inspect its status",
            target: "queue",
            rejected: "cache",
            expected_binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_POS_2",
            category: "possessive_focus",
            setup: Some("캐시를 분석하고 큐를 수리해"),
            bridges: &[],
            follow: "그것의 상태를 검사해",
            target: "큐",
            rejected: "캐시",
            expected_binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_POS_3",
            category: "possessive_focus",
            setup: Some("inspect the worker because the service failed"),
            bridges: &[],
            follow: "analyze its configuration",
            target: "worker",
            rejected: "service",
            expected_binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_POS_4",
            category: "possessive_focus",
            setup: Some("서비스가 실패했기 때문에 워커를 검사해"),
            bridges: &[],
            follow: "그것의 설정을 분석해",
            target: "워커",
            rejected: "서비스",
            expected_binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_DEMO_1",
            category: "demonstrative_nominal",
            setup: Some("inspect the bundle and repair the snapshot"),
            bridges: &[],
            follow: "analyze that object",
            target: "snapshot",
            rejected: "bundle",
            expected_binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_DEMO_2",
            category: "demonstrative_nominal",
            setup: Some("묶음을 검사하고 스냅샷을 수리해"),
            bridges: &[],
            follow: "그 대상을 분석해",
            target: "스냅샷",
            rejected: "묶음",
            expected_binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_DEMO_3",
            category: "demonstrative_nominal",
            setup: Some("before you deploy the service, inspect the manifest"),
            bridges: &[],
            follow: "repair that one",
            target: "manifest",
            rejected: "service",
            expected_binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_DEMO_4",
            category: "demonstrative_nominal",
            setup: Some("서비스를 배포하기 전에 매니페스트를 검사해"),
            bridges: &[],
            follow: "그 대상을 수리해",
            target: "매니페스트",
            rejected: "서비스",
            expected_binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_ZERO_1",
            category: "zero_argument",
            setup: Some("analyze the cache and repair the queue"),
            bridges: &[],
            follow: "then inspect again",
            target: "queue",
            rejected: "cache",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_ZERO_2",
            category: "zero_argument",
            setup: Some("캐시를 분석하고 큐를 수리해"),
            bridges: &[],
            follow: "그다음 다시 검사해",
            target: "큐",
            rejected: "캐시",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_ZERO_3",
            category: "zero_argument",
            setup: Some("inspect the server and analyze the log"),
            bridges: &["thanks"],
            follow: "repair next",
            target: "log",
            rejected: "server",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_ZERO_4",
            category: "zero_argument",
            setup: Some("파일을 검사하고 폴더를 분석해"),
            bridges: &["고마워"],
            follow: "이제 수리해",
            target: "폴더",
            rejected: "파일",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_COND_1",
            category: "conditional_zero_argument",
            setup: Some("inspect the worker"),
            bridges: &[],
            follow: "if stale, repair",
            target: "worker",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Deferred,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_COND_2",
            category: "conditional_zero_argument",
            setup: Some("워커를 검사해"),
            bridges: &[],
            follow: "오래됐으면 수리해",
            target: "워커",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Deferred,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_COND_3",
            category: "conditional_zero_argument",
            setup: Some("analyze the snapshot"),
            bridges: &[],
            follow: "if broken, inspect again",
            target: "snapshot",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Deferred,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_COND_4",
            category: "conditional_zero_argument",
            setup: Some("스냅샷을 분석해"),
            bridges: &[],
            follow: "깨졌으면 다시 검사해",
            target: "스냅샷",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Deferred,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_PRED_1",
            category: "open_vocabulary_predicate_ellipsis",
            setup: Some("inspect the cache"),
            bridges: &[],
            follow: "bundle too",
            target: "bundle",
            rejected: "cache",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_PRED_2",
            category: "open_vocabulary_predicate_ellipsis",
            setup: Some("캐시를 검사해"),
            bridges: &[],
            follow: "묶음도",
            target: "묶음",
            rejected: "캐시",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_PRED_3",
            category: "open_vocabulary_predicate_ellipsis",
            setup: Some("repair the queue"),
            bridges: &[],
            follow: "same for snapshot",
            target: "snapshot",
            rejected: "queue",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_PRED_4",
            category: "open_vocabulary_predicate_ellipsis",
            setup: Some("큐를 수리해"),
            bridges: &[],
            follow: "스냅샷도",
            target: "스냅샷",
            rejected: "큐",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_AMBIG_1",
            category: "ellipsis_ambiguity",
            setup: Some("inspect the cache and repair the queue"),
            bridges: &[],
            follow: "bundle too",
            target: "",
            rejected: "",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Clarify,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_AMBIG_2",
            category: "ellipsis_ambiguity",
            setup: Some("캐시를 검사하고 큐를 수리해"),
            bridges: &[],
            follow: "묶음도",
            target: "",
            rejected: "",
            expected_binding: "ELLIPTICAL_ACTION",
            mode: ExpectedMode::Clarify,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_AMBIG_3",
            category: "ellipsis_without_antecedent",
            setup: None,
            bridges: &[],
            follow: "repair next",
            target: "",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Clarify,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_AMBIG_4",
            category: "ellipsis_without_antecedent",
            setup: None,
            bridges: &[],
            follow: "이제 수리해",
            target: "",
            rejected: "",
            expected_binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: ExpectedMode::Clarify,
            language: LanguageCodeIR::Korean,
        },
    ];
    let rows = cases.into_iter().map(run_case).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R28-RUN-0001",
        "frozen_before_product_changes": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("JSON"));
    if passed != 24 {
        std::process::exit(1);
    }
}
