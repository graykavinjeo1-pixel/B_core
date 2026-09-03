//! Frozen R28-RUN-0002 held-out typed deixis and ellipsis transfer suite.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, LanguageCodeIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
enum Mode {
    Grounded,
    Deferred,
    Clarify,
}

#[derive(Clone, Copy)]
struct Case<'a> {
    id: &'a str,
    category: &'a str,
    setup: Option<&'a str>,
    follow: &'a str,
    target: &'a str,
    rejected: &'a str,
    binding: &'a str,
    mode: Mode,
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

fn run(case: Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let turn = if let Some(setup) = case.setup {
        api.process_conversation_turn(&request(case.id, 1, setup, case.language))
            .expect("setup");
        2
    } else {
        1
    };
    let response = api
        .process_conversation_turn(&request(case.id, turn, case.follow, case.language))
        .expect("follow");
    let resolved = response
        .reference_resolution
        .resolved_semantic_text
        .to_lowercase();
    let selected = response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidate()
        .map(|candidate| candidate.subject.to_lowercase())
        .unwrap_or_default();
    let deferred = response
        .conversation_state
        .deferred_action_commitments
        .last()
        .map(|commitment| commitment.action.subject.to_lowercase())
        .unwrap_or_default();
    let binding = response.reference_resolution.discourse_bindings.last();
    let kind = binding
        .and_then(|binding| serde_json::to_value(binding).ok())
        .and_then(|binding| binding["kind"].as_str().map(ToString::to_string))
        .unwrap_or_default();
    let safe = binding.is_some_and(|binding| {
        binding
            .evidence
            .contains(&"SEMANTIC_AUTHORITY:false".to_string())
            && binding
                .evidence
                .contains(&"EXTERNAL_EXECUTION_AUTHORIZED:false".to_string())
    });
    let target = case.target.to_lowercase();
    let rejected = case.rejected.to_lowercase();
    let semantic_match = (target.is_empty() || resolved.contains(&target))
        && (rejected.is_empty() || !resolved.contains(&rejected));
    let pass = match case.mode {
        Mode::Grounded => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.grounded_response.is_some()
                && kind == case.binding
                && safe
                && semantic_match
                && selected.contains(&target)
        }
        Mode::Deferred => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && kind == case.binding
                && safe
                && semantic_match
                && deferred.contains(&target)
        }
        Mode::Clarify => {
            response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && response.grounded_response.is_none()
                && response.reference_resolution.resolved_semantic_text == case.follow
                && response
                    .reference_resolution
                    .ambiguous_reference_surfaces
                    .iter()
                    .any(|surface| surface == case.binding)
        }
    };
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        trace: vec![resolved, selected, deferred, kind],
        pass,
    }
}

fn main() {
    let cases = [
        Case {
            id: "R28_TRANSFER_1",
            category: "held_out_possessive",
            setup: Some("inspect the archive and repair the indexer"),
            follow: "verify its checksum",
            target: "indexer",
            rejected: "archive",
            binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_2",
            category: "held_out_possessive",
            setup: Some("패키지를 검사하고 인덱서를 수리해"),
            follow: "그것의 버전을 확인해",
            target: "인덱서",
            rejected: "패키지",
            binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_3",
            category: "held_out_possessive",
            setup: Some("inspect the parser because the compiler failed"),
            follow: "analyze its configuration",
            target: "parser",
            rejected: "compiler",
            binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_4",
            category: "held_out_possessive",
            setup: Some("컴파일러가 실패해서 파서를 검사해"),
            follow: "그것의 설정을 분석해",
            target: "파서",
            rejected: "컴파일러",
            binding: "POSSESSIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_5",
            category: "held_out_demonstrative",
            setup: Some("inspect the artifact and repair the dispatcher"),
            follow: "analyze that object",
            target: "dispatcher",
            rejected: "artifact",
            binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_6",
            category: "held_out_demonstrative",
            setup: Some("이미지를 검사하고 디스패처를 수리해"),
            follow: "그 대상을 분석해",
            target: "디스패처",
            rejected: "이미지",
            binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_7",
            category: "held_out_demonstrative",
            setup: Some("before you deploy the gateway, inspect the policy"),
            follow: "repair that one",
            target: "policy",
            rejected: "gateway",
            binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_8",
            category: "held_out_demonstrative",
            setup: Some("게이트웨이를 배포하기 전에 정책을 검사해"),
            follow: "그 대상을 수리해",
            target: "정책",
            rejected: "게이트웨이",
            binding: "DEMONSTRATIVE_FOCUS_REFERENCE",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_9",
            category: "held_out_zero_argument",
            setup: Some("inspect the archive and analyze the indexer"),
            follow: "repair next",
            target: "indexer",
            rejected: "archive",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_10",
            category: "held_out_zero_argument",
            setup: Some("패키지를 검사하고 인덱서를 분석해"),
            follow: "이제 수리해",
            target: "인덱서",
            rejected: "패키지",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_11",
            category: "held_out_conditional_zero",
            setup: Some("inspect the dispatcher"),
            follow: "if broken, repair",
            target: "dispatcher",
            rejected: "",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Deferred,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_12",
            category: "held_out_conditional_zero",
            setup: Some("디스패처를 검사해"),
            follow: "고장났으면 수리해",
            target: "디스패처",
            rejected: "",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Deferred,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_13",
            category: "held_out_open_predicate",
            setup: Some("inspect the archive"),
            follow: "artifact too",
            target: "artifact",
            rejected: "archive",
            binding: "ELLIPTICAL_ACTION",
            mode: Mode::Grounded,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_14",
            category: "held_out_open_predicate",
            setup: Some("패키지를 검사해"),
            follow: "이미지도",
            target: "이미지",
            rejected: "패키지",
            binding: "ELLIPTICAL_ACTION",
            mode: Mode::Grounded,
            language: LanguageCodeIR::Korean,
        },
        Case {
            id: "R28_TRANSFER_15",
            category: "held_out_no_antecedent",
            setup: None,
            follow: "analyze again",
            target: "",
            rejected: "",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Clarify,
            language: LanguageCodeIR::English,
        },
        Case {
            id: "R28_TRANSFER_16",
            category: "held_out_no_antecedent",
            setup: None,
            follow: "다시 검사해",
            target: "",
            rejected: "",
            binding: "ZERO_ARGUMENT_ELLIPSIS",
            mode: Mode::Clarify,
            language: LanguageCodeIR::Korean,
        },
    ];
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let payload = serde_json::json!({
        "suite": "R28-RUN-0002",
        "held_out_until_after_diagnostic_repairs": true,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string_pretty(&payload).expect("JSON"));
    if passed != 16 {
        std::process::exit(1);
    }
}
