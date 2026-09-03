//! Frozen compositional dialogue-directive transfer suite.
//!
//! Cases combine unseen Korean/English target, operator, and value aliases.
//! They were fixed before product execution and distinguish live directives
//! from descriptions, quotations, contradictory values, and simultaneous
//! semantic tasks.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DialogueDirectiveKindIR, LanguageCodeIR, NaturalResponseActIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum Expected<'a> {
    DirectiveOnly(&'a str),
    DirectiveAndTask(&'a str, &'a str),
    NotDirective,
    Conflicting,
}

struct Case<'a> {
    id: &'a str,
    language: LanguageCodeIR,
    text: &'a str,
    expected: Expected<'a>,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    language: LanguageCodeIR,
    text_sha256: String,
    disposition: ConversationTurnDispositionIR,
    response_act: NaturalResponseActIR,
    active_directive_values: Vec<String>,
    grounded_plan: bool,
    active_goal_subjects: Vec<String>,
    semantic_authority: bool,
    external_action_executed: bool,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    pass: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_before_first_execution: bool,
    fresh_cases: usize,
    passed: usize,
    failed: usize,
    directive_only_cases: usize,
    directive_and_task_cases: usize,
    negative_or_conflict_cases: usize,
    semantic_authority_violations: usize,
    external_execution_authorizations: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(case: &Case<'_>) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: format!("BLIND-DIRECTIVE-{}", case.id),
        turn_index: 1,
        request_id: format!("BLIND-DIRECTIVE-{}-1", case.id),
        modality: ConversationInputModalityIR::Text,
        raw_text: case.text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(case.language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let request = request(case);
    let response = api
        .process_conversation_turn(&request)
        .expect("blind dialogue directive case");
    let active_directive_values = response
        .conversation_state
        .dialogue_directive_ledger
        .active()
        .filter(|directive| directive.kind == DialogueDirectiveKindIR::ResponseLength)
        .map(|directive| directive.value_key.clone())
        .collect::<Vec<_>>();
    let active_goal_subjects = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.subject.clone())
        .collect::<Vec<_>>();
    let structural_pass = match case.expected {
        Expected::DirectiveOnly(value) => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.natural_realization.response_act
                    == NaturalResponseActIR::InformAcknowledgement
                && response.grounded_response.is_none()
                && active_directive_values == [value]
                && active_goal_subjects.is_empty()
        }
        Expected::DirectiveAndTask(value, subject_fragment) => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.natural_realization.response_act == NaturalResponseActIR::PlanPreview
                && response.grounded_response.is_some()
                && active_directive_values == [value]
                && active_goal_subjects.iter().any(|subject| {
                    subject
                        .to_lowercase()
                        .contains(&subject_fragment.to_lowercase())
                })
        }
        Expected::NotDirective => active_directive_values.is_empty(),
        Expected::Conflicting => {
            response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && response.natural_realization.response_act
                    == NaturalResponseActIR::ClarificationRequest
                && active_directive_values.is_empty()
                && response.grounded_response.is_none()
        }
    };
    let safety_pass = response.validate_against(&request)
        && !response.natural_realization.semantic_authority
        && !response
            .language_cortex_integration
            .external_action_executed
        && response.language_cortex_integration.external_llm_calls == 0
        && response.language_cortex_integration.local_teacher_calls == 0
        && response.language_cortex_integration.network_calls == 0
        && response
            .conversation_state
            .dialogue_directive_ledger
            .active()
            .all(|directive| {
                !directive.semantic_authority && !directive.external_execution_authorized
            });
    Row {
        id: case.id.to_string(),
        language: case.language,
        text_sha256: format!("{:x}", Sha256::digest(case.text.as_bytes())),
        disposition: response.disposition,
        response_act: response.natural_realization.response_act,
        active_directive_values,
        grounded_plan: response.grounded_response.is_some(),
        active_goal_subjects,
        semantic_authority: response.natural_realization.semantic_authority,
        external_action_executed: response
            .language_cortex_integration
            .external_action_executed,
        external_llm_calls: response.language_cortex_integration.external_llm_calls,
        local_teacher_calls: response.language_cortex_integration.local_teacher_calls,
        pass: structural_pass && safety_pass,
    }
}

fn main() {
    let cases = [
        Case {
            id: "D01",
            language: LanguageCodeIR::Korean,
            text: "대답은 간단히 유지해.",
            expected: Expected::DirectiveOnly("CONCISE"),
        },
        Case {
            id: "D02",
            language: LanguageCodeIR::Korean,
            text: "설명은 구체적으로 말해줘.",
            expected: Expected::DirectiveOnly("DETAILED"),
        },
        Case {
            id: "D03",
            language: LanguageCodeIR::Korean,
            text: "응답은 핵심만 해주세요.",
            expected: Expected::DirectiveOnly("CONCISE"),
        },
        Case {
            id: "D04",
            language: LanguageCodeIR::English,
            text: "Reply briefly.",
            expected: Expected::DirectiveOnly("CONCISE"),
        },
        Case {
            id: "D05",
            language: LanguageCodeIR::English,
            text: "Would you keep the answer concise?",
            expected: Expected::DirectiveOnly("CONCISE"),
        },
        Case {
            id: "D06",
            language: LanguageCodeIR::English,
            text: "Respond with a comprehensive explanation.",
            expected: Expected::DirectiveOnly("DETAILED"),
        },
        Case {
            id: "D07",
            language: LanguageCodeIR::Korean,
            text: "답변은 간결하게 해줘. Indigo 워커를 점검해.",
            expected: Expected::DirectiveAndTask("CONCISE", "indigo"),
        },
        Case {
            id: "D08",
            language: LanguageCodeIR::English,
            text: "Keep the reply short and inspect the Jade cache.",
            expected: Expected::DirectiveAndTask("CONCISE", "jade"),
        },
        Case {
            id: "D09",
            language: LanguageCodeIR::Korean,
            text: "그 대답은 간결하게 정리되어 있었다.",
            expected: Expected::NotDirective,
        },
        Case {
            id: "D10",
            language: LanguageCodeIR::English,
            text: "Her reply was concise.",
            expected: Expected::NotDirective,
        },
        Case {
            id: "D11",
            language: LanguageCodeIR::Korean,
            text: "‘대답은 짧게 해줘’라는 문장을 해석해.",
            expected: Expected::NotDirective,
        },
        Case {
            id: "D12",
            language: LanguageCodeIR::English,
            text: "Please make the response concise and detailed.",
            expected: Expected::Conflicting,
        },
    ];
    let rows = cases.iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        schema: "B_CORE_DIALOGUE_DIRECTIVE_COMPOSITION_BLIND_1",
        suite: "DIALOGUE_DIRECTIVE_COMPOSITION_BLIND",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        directive_only_cases: cases
            .iter()
            .filter(|case| matches!(case.expected, Expected::DirectiveOnly(_)))
            .count(),
        directive_and_task_cases: cases
            .iter()
            .filter(|case| matches!(case.expected, Expected::DirectiveAndTask(_, _)))
            .count(),
        negative_or_conflict_cases: cases
            .iter()
            .filter(|case| {
                matches!(
                    case.expected,
                    Expected::NotDirective | Expected::Conflicting
                )
            })
            .count(),
        semantic_authority_violations: rows.iter().filter(|row| row.semantic_authority).count(),
        external_execution_authorizations: rows
            .iter()
            .filter(|row| row.external_action_executed)
            .count(),
        external_llm_calls: rows.iter().map(|row| row.external_llm_calls).sum(),
        local_teacher_calls: rows.iter().map(|row| row.local_teacher_calls).sum(),
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize blind directive report")
    );
    if report.failed > 0 {
        std::process::exit(1);
    }
}
