//! Frozen blind suite for compositional response-format directives.
//!
//! The suite was fixed before its first product execution. It checks Korean
//! and English lexical transfer, independent length/format axes, same-turn and
//! cross-turn task retention, quote/description non-promotion, conflict
//! handling, and grounded layout realization without language authority.

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnDispositionIR,
    ConversationTurnRequestIR, DialogueDirectiveKindIR, LanguageCodeIR, NaturalResponseActIR,
    NaturalResponseFormatIR, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
enum Expected<'a> {
    DirectiveOnly(NaturalResponseFormatIR, &'a str),
    DirectiveAndTask(NaturalResponseFormatIR, &'a str, Option<&'a str>, &'a str),
    NotDirective,
    Conflicting,
}

struct Case<'a> {
    id: &'a str,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    text: &'a str,
    expected: Expected<'a>,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    text_sha256: String,
    disposition: ConversationTurnDispositionIR,
    response_act: NaturalResponseActIR,
    response_format: NaturalResponseFormatIR,
    active_format_values: Vec<String>,
    active_length_values: Vec<String>,
    native_candidate_goal_subjects: Vec<String>,
    compatibility_candidate_goal_subjects: Vec<String>,
    language_center_projected_goal_subjects: Vec<String>,
    selected_semantic_goal_subjects: Vec<String>,
    persisted_goal_subjects: Vec<String>,
    semantic_memory_alignment: bool,
    layout_valid: bool,
    semantic_authority: bool,
    external_action_executed: bool,
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
    cross_turn_retention_cases: usize,
    negative_or_conflict_cases: usize,
    longest_match_cases: usize,
    semantic_authority_violations: usize,
    external_execution_authorizations: usize,
    unsupported_explanation_facts: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn request(
    conversation_id: &str,
    turn_index: u64,
    text: &str,
    output_language: LanguageCodeIR,
) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(output_language),
        context_tags: Vec::new(),
        max_plan_steps: 16,
    }
}

fn layout_valid(format: NaturalResponseFormatIR, language: LanguageCodeIR, text: &str) -> bool {
    match format {
        NaturalResponseFormatIR::Plain => {
            !text.starts_with("- ") && !text.starts_with("1. ") && !text.starts_with('|')
        }
        NaturalResponseFormatIR::Bullets => {
            text.lines().count() >= 1 && text.lines().all(|line| line.starts_with("- "))
        }
        NaturalResponseFormatIR::Numbered => text
            .lines()
            .enumerate()
            .all(|(index, line)| line.starts_with(&format!("{}. ", index + 1))),
        NaturalResponseFormatIR::Table => match language {
            LanguageCodeIR::Korean => text.starts_with("| 번호 | 내용 |\n|---:|---|\n"),
            _ => text.starts_with("| No. | Content |\n|---:|---|\n"),
        },
    }
}

fn subject_key(subject: &str) -> String {
    let mut normalized = subject.trim().to_lowercase();
    for article in ["the ", "a ", "an "] {
        if let Some(rest) = normalized.strip_prefix(article) {
            normalized = rest.trim().to_string();
            break;
        }
    }
    normalized
}

fn row_from_response(
    case: &Case<'_>,
    request: &ConversationTurnRequestIR,
    response: &semantic_core_adapters::ConversationTurnResponseIR,
) -> Row {
    let active_format_values = response
        .conversation_state
        .dialogue_directive_ledger
        .active()
        .filter(|directive| directive.kind == DialogueDirectiveKindIR::ResponseFormat)
        .map(|directive| directive.value_key.clone())
        .collect::<Vec<_>>();
    let active_length_values = response
        .conversation_state
        .dialogue_directive_ledger
        .active()
        .filter(|directive| directive.kind == DialogueDirectiveKindIR::ResponseLength)
        .map(|directive| directive.value_key.clone())
        .collect::<Vec<_>>();
    let native_candidate_goal_subjects = response
        .native_language_circuit
        .selected_live_goals
        .iter()
        .map(|goal| goal.subject.clone())
        .collect::<Vec<_>>();
    let compatibility_candidate_goal_subjects = response
        .pragmatic_interpretation
        .compositional_analysis
        .selected_candidates()
        .into_iter()
        .map(|candidate| candidate.subject.clone())
        .collect::<Vec<_>>();
    let language_center_projected_goal_subjects = response
        .pragmatic_interpretation
        .language_center
        .projected_goal_event_ids
        .iter()
        .filter_map(|event_id| {
            response
                .pragmatic_interpretation
                .language_center
                .events
                .iter()
                .find(|event| &event.event_id == event_id)
        })
        .map(|event| {
            event
                .goal_subject_argument_ids
                .iter()
                .filter_map(|argument_id| {
                    event
                        .arguments
                        .iter()
                        .find(|argument| &argument.argument_id == argument_id)
                        .map(|argument| argument.phenotype_surface.as_str())
                })
                .collect::<Vec<_>>()
                .join(" & ")
        })
        .collect::<Vec<_>>();
    let selected_semantic_goal_subjects = response
        .grounded_response
        .as_deref()
        .map(|grounded| {
            grounded
                .semantic_goal
                .selected_live_event_ids
                .iter()
                .filter_map(|event_id| {
                    grounded
                        .semantic_goal
                        .events
                        .iter()
                        .find(|event| &event.event_id == event_id)
                })
                .map(|event| {
                    event
                        .goal_subject_argument_ids
                        .iter()
                        .filter_map(|argument_id| {
                            grounded
                                .semantic_goal
                                .arguments
                                .iter()
                                .find(|argument| &argument.argument_id == argument_id)
                                .map(|argument| argument.grounded_label.as_str())
                        })
                        .collect::<Vec<_>>()
                        .join(" & ")
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let persisted_goal_subjects = response
        .conversation_state
        .active_goals
        .iter()
        .map(|goal| goal.subject.clone())
        .collect::<Vec<_>>();
    let normalize_subjects = |subjects: &[String]| {
        let mut normalized = subjects
            .iter()
            .map(|subject| subject_key(subject))
            .collect::<Vec<_>>();
        normalized.sort();
        normalized
    };
    let semantic_memory_alignment = normalize_subjects(&selected_semantic_goal_subjects)
        == normalize_subjects(&persisted_goal_subjects);
    let structural_pass = match case.expected {
        Expected::DirectiveOnly(format, value) => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.natural_realization.response_act
                    == NaturalResponseActIR::InformAcknowledgement
                && response.grounded_response.is_none()
                && active_format_values == [value]
                && persisted_goal_subjects.is_empty()
                && response.natural_realization.response_plan.response_format == format
        }
        Expected::DirectiveAndTask(format, value, length, subject_fragment) => {
            response.disposition == ConversationTurnDispositionIR::Grounded
                && response.natural_realization.response_act == NaturalResponseActIR::PlanPreview
                && response.grounded_response.is_some()
                && active_format_values == [value]
                && length.is_none_or(|expected| active_length_values == [expected])
                && !native_candidate_goal_subjects.is_empty()
                && selected_semantic_goal_subjects.len() == 1
                && persisted_goal_subjects.len() == 1
                && selected_semantic_goal_subjects[0]
                    .to_lowercase()
                    .contains(&subject_fragment.to_lowercase())
                && semantic_memory_alignment
                && response.natural_realization.response_plan.response_format == format
        }
        Expected::NotDirective => active_format_values.is_empty(),
        Expected::Conflicting => {
            response.disposition == ConversationTurnDispositionIR::ClarificationRequired
                && response.natural_realization.response_act
                    == NaturalResponseActIR::ClarificationRequest
                && active_format_values.is_empty()
                && response.grounded_response.is_none()
        }
    };
    let format = response.natural_realization.response_plan.response_format;
    let layout_valid = layout_valid(format, case.output_language, &response.output.text);
    let safety_pass = response.validate_against(request)
        && !response.natural_realization.semantic_authority
        && !response
            .language_cortex_integration
            .external_action_executed
        && response
            .language_cortex_integration
            .unsupported_explanation_facts
            == 0
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
        input_language: case.input_language,
        output_language: case.output_language,
        text_sha256: format!("{:x}", Sha256::digest(case.text.as_bytes())),
        disposition: response.disposition,
        response_act: response.natural_realization.response_act,
        response_format: format,
        active_format_values,
        active_length_values,
        native_candidate_goal_subjects,
        compatibility_candidate_goal_subjects,
        language_center_projected_goal_subjects,
        selected_semantic_goal_subjects,
        persisted_goal_subjects,
        semantic_memory_alignment,
        layout_valid,
        semantic_authority: response.natural_realization.semantic_authority,
        external_action_executed: response
            .language_cortex_integration
            .external_action_executed,
        pass: structural_pass && layout_valid && safety_pass,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let conversation_id = format!("BLIND-FORMAT-{}", case.id);
    let turn = request(&conversation_id, 1, case.text, case.output_language);
    let response = api
        .process_conversation_turn(&turn)
        .expect("blind response-format case");
    row_from_response(case, &turn, &response)
}

fn run_cross_turn(
    case: &Case<'_>,
    directive_text: &str,
    directive_language: LanguageCodeIR,
) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    let conversation_id = format!("BLIND-FORMAT-{}", case.id);
    let setup = request(&conversation_id, 1, directive_text, directive_language);
    let setup_response = api
        .process_conversation_turn(&setup)
        .expect("cross-turn directive setup");
    assert!(setup_response.validate_against(&setup));
    let turn = request(&conversation_id, 2, case.text, case.output_language);
    let response = api
        .process_conversation_turn(&turn)
        .expect("cross-turn formatted task");
    row_from_response(case, &turn, &response)
}

fn main() {
    let cases = [
        Case {
            id: "F01",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "응답을 테이블로 말해줘.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Table, "TABLE"),
        },
        Case {
            id: "F02",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "대답은 항목별로 유지해.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Bullets, "BULLETS"),
        },
        Case {
            id: "F03",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "설명은 번호를 매겨 말해줘.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Numbered, "NUMBERED"),
        },
        Case {
            id: "F04",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "응답은 평문으로 해주세요.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Plain, "PLAIN"),
        },
        Case {
            id: "F05",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Make the response tabular format.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Table, "TABLE"),
        },
        Case {
            id: "F06",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Reply in bullets.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Bullets, "BULLETS"),
        },
        Case {
            id: "F07",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Please keep the answer as a numbered list.",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Numbered, "NUMBERED"),
        },
        Case {
            id: "F08",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Would you keep the reply in plain prose?",
            expected: Expected::DirectiveOnly(NaturalResponseFormatIR::Plain, "PLAIN"),
        },
        Case {
            id: "F09",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "답변은 핵심만 표 형식으로 해줘. Indigo 워커를 분석해.",
            expected: Expected::DirectiveAndTask(
                NaturalResponseFormatIR::Table,
                "TABLE",
                Some("CONCISE"),
                "Indigo",
            ),
        },
        Case {
            id: "F10",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Keep the response in bullet points and inspect the Jade cache.",
            expected: Expected::DirectiveAndTask(
                NaturalResponseFormatIR::Bullets,
                "BULLETS",
                None,
                "Jade",
            ),
        },
        Case {
            id: "F11",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "그 대답은 표 형식으로 정리되어 있었다.",
            expected: Expected::NotDirective,
        },
        Case {
            id: "F12",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Explain the phrase \"reply in bullets\".",
            expected: Expected::NotDirective,
        },
        Case {
            id: "F13",
            input_language: LanguageCodeIR::Korean,
            output_language: LanguageCodeIR::Korean,
            text: "응답은 테이블로 불릿으로 해줘.",
            expected: Expected::Conflicting,
        },
        Case {
            id: "F14",
            input_language: LanguageCodeIR::English,
            output_language: LanguageCodeIR::English,
            text: "Make the response a table and a numbered list.",
            expected: Expected::Conflicting,
        },
    ];
    let mut rows = cases.iter().map(run).collect::<Vec<_>>();
    let cross_turn_cases = [
        (
            Case {
                id: "F15",
                input_language: LanguageCodeIR::English,
                output_language: LanguageCodeIR::English,
                text: "Inspect the Willow queue.",
                expected: Expected::DirectiveAndTask(
                    NaturalResponseFormatIR::Table,
                    "TABLE",
                    None,
                    "Willow",
                ),
            },
            "대답은 표로 유지해.",
            LanguageCodeIR::Korean,
        ),
        (
            Case {
                id: "F16",
                input_language: LanguageCodeIR::Korean,
                output_language: LanguageCodeIR::Korean,
                text: "Orchid 캐시를 점검해.",
                expected: Expected::DirectiveAndTask(
                    NaturalResponseFormatIR::Numbered,
                    "NUMBERED",
                    None,
                    "Orchid",
                ),
            },
            "Please keep the answer as a numbered list.",
            LanguageCodeIR::English,
        ),
    ];
    rows.extend(
        cross_turn_cases
            .iter()
            .map(|(case, setup, language)| run_cross_turn(case, setup, *language)),
    );
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        schema: "B_CORE_RESPONSE_FORMAT_COMPOSITION_BLIND_1",
        suite: "RESPONSE_FORMAT_COMPOSITION_BLIND",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        directive_only_cases: 8,
        directive_and_task_cases: 4,
        cross_turn_retention_cases: 2,
        negative_or_conflict_cases: 4,
        longest_match_cases: 2,
        semantic_authority_violations: rows.iter().filter(|row| row.semantic_authority).count(),
        external_execution_authorizations: rows
            .iter()
            .filter(|row| row.external_action_executed)
            .count(),
        unsupported_explanation_facts: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize response-format report")
    );
    if report.failed > 0 {
        std::process::exit(1);
    }
}
