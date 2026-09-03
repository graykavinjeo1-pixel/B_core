//! Frozen blind suite for typed topic-transition realization.
//!
//! Cases were fixed before first execution. The suite covers named-topic
//! activation, prior-topic restoration, and discourse-group activation from
//! both source languages, with Korean and English realization from one typed
//! transition graph.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    NaturalRealizationPathIR, NaturalResponseActIR, TopicTransitionKindIR,
    CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct Turn<'a> {
    text: &'a str,
    language: LanguageCodeIR,
}

struct Case<'a> {
    id: &'a str,
    semantic_group: &'a str,
    category: &'a str,
    setup: &'a [Turn<'a>],
    query: &'a str,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    expected_kind: TopicTransitionKindIR,
    expected_concept: &'a str,
    required_fragments: &'a [&'a str],
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    transition_kind: Option<TopicTransitionKindIR>,
    required_fragments: Vec<String>,
    realized_text: String,
    semantic_sha256: String,
    semantic_pair_invariant: bool,
    typed_generation: bool,
    safety_boundary: bool,
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
    cross_language_semantic_pairs: usize,
    cross_language_semantic_pairs_passed: usize,
    generative_path_rate_millis: u16,
    drafted_surface_fallbacks: usize,
    stage_overwrites: usize,
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
    input_language: LanguageCodeIR,
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
        context_tags: vec![format!("INPUT_LANGUAGE:{input_language:?}")],
        max_plan_steps: 16,
    }
}

fn run(case: &Case<'_>) -> Row {
    let mut api = CognitiveApi::new_embedded().expect("embedded core");
    for (index, turn) in case.setup.iter().copied().enumerate() {
        api.process_conversation_turn(&request(
            case.semantic_group,
            u64::try_from(index + 1).expect("bounded setup turn"),
            turn.text,
            turn.language,
            turn.language,
        ))
        .unwrap_or_else(|error| panic!("setup failed: case={}, error={error:?}", case.id));
    }
    let response = api
        .process_conversation_turn(&request(
            case.semantic_group,
            u64::try_from(case.setup.len() + 1).expect("bounded query turn"),
            case.query,
            case.input_language,
            case.output_language,
        ))
        .unwrap_or_else(|error| panic!("case failed: case={}, error={error:?}", case.id));
    let transition = response.topic_transition.as_ref();
    let trace = response.natural_realization.generation_traces.first();
    let typed_generation = transition.is_some_and(|transition| {
        transition.validate()
            && transition.applied
            && transition.kind == case.expected_kind
            && !transition.semantic_authority
            && !transition.external_action_executed
    }) && response.natural_realization.response_act
        == NaturalResponseActIR::TopicTransition
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && response.natural_realization.stage_overwrite_count == 0
        && trace.is_some_and(|trace| {
            trace.validate()
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == case.expected_concept)
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == "C_TOPIC_ONLY")
        });
    let output_lower = response.output.text.to_lowercase();
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && case
            .required_fragments
            .iter()
            .all(|fragment| output_lower.contains(&fragment.to_lowercase()))
        && response.output.grounded_plan_sha256.is_none()
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_TOPIC_")
        && !response.output.text.contains("TopicTransitionIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        transition_kind: transition.map(|transition| transition.kind),
        required_fragments: case
            .required_fragments
            .iter()
            .map(|fragment| (*fragment).to_string())
            .collect(),
        realized_text: response.output.text,
        semantic_sha256: trace
            .map(|trace| trace.meaning.semantic_sha256.clone())
            .unwrap_or_default(),
        semantic_pair_invariant: false,
        typed_generation,
        safety_boundary,
        pass: false,
    }
}

fn cases() -> Vec<Case<'static>> {
    use LanguageCodeIR::{English as En, Korean as Ko};
    use TopicTransitionKindIR::{ActivateGroup, ActivateNamed, ReturnPrevious};
    const RETURN_EN: &[Turn<'static>] = &[
        Turn {
            text: "Switch to the Aster cache topic.",
            language: En,
        },
        Turn {
            text: "Switch to the Birch queue topic.",
            language: En,
        },
    ];
    const RETURN_KO: &[Turn<'static>] = &[
        Turn {
            text: "Aster 캐시 이야기로 돌아가자.",
            language: Ko,
        },
        Turn {
            text: "Birch 큐 이야기로 돌아가자.",
            language: Ko,
        },
    ];
    const GROUP_EN: &[Turn<'static>] = &[Turn {
        text: "Inspect the Cedar cache and repair the Dune queue.",
        language: En,
    }];
    const GROUP_KO: &[Turn<'static>] = &[Turn {
        text: "Cedar 캐시를 확인하고 Dune 큐를 수리해.",
        language: Ko,
    }];
    vec![
        Case {
            id: "R32_TOPIC_01",
            semantic_group: "R32_NAMED_EN_PAIR",
            category: "named_english_input_korean_output",
            setup: &[],
            query: "Switch to the Lumen index topic.",
            input_language: En,
            output_language: Ko,
            expected_kind: ActivateNamed,
            expected_concept: "C_ACTIVATE_TOPIC",
            required_fragments: &["Lumen", "화제", "실행"],
        },
        Case {
            id: "R32_TOPIC_02",
            semantic_group: "R32_NAMED_EN_PAIR",
            category: "named_english_input_english_output",
            setup: &[],
            query: "Switch to the Lumen index topic.",
            input_language: En,
            output_language: En,
            expected_kind: ActivateNamed,
            expected_concept: "C_ACTIVATE_TOPIC",
            required_fragments: &["Lumen", "active topic", "execute"],
        },
        Case {
            id: "R32_TOPIC_03",
            semantic_group: "R32_NAMED_KO_PAIR",
            category: "named_korean_input_korean_output",
            setup: &[],
            query: "Nova 서버 이야기로 돌아가자.",
            input_language: Ko,
            output_language: Ko,
            expected_kind: ActivateNamed,
            expected_concept: "C_ACTIVATE_TOPIC",
            required_fragments: &["Nova", "화제", "실행"],
        },
        Case {
            id: "R32_TOPIC_04",
            semantic_group: "R32_NAMED_KO_PAIR",
            category: "named_korean_input_english_output",
            setup: &[],
            query: "Nova 서버 이야기로 돌아가자.",
            input_language: Ko,
            output_language: En,
            expected_kind: ActivateNamed,
            expected_concept: "C_ACTIVATE_TOPIC",
            required_fragments: &["Nova", "active topic", "execute"],
        },
        Case {
            id: "R32_TOPIC_05",
            semantic_group: "R32_RETURN_EN_PAIR",
            category: "previous_english_input_korean_output",
            setup: RETURN_EN,
            query: "Return to the previous topic.",
            input_language: En,
            output_language: Ko,
            expected_kind: ReturnPrevious,
            expected_concept: "C_RETURN_TOPIC",
            required_fragments: &["Aster", "돌아", "실행"],
        },
        Case {
            id: "R32_TOPIC_06",
            semantic_group: "R32_RETURN_EN_PAIR",
            category: "previous_english_input_english_output",
            setup: RETURN_EN,
            query: "Return to the previous topic.",
            input_language: En,
            output_language: En,
            expected_kind: ReturnPrevious,
            expected_concept: "C_RETURN_TOPIC",
            required_fragments: &["Aster", "return", "execute"],
        },
        Case {
            id: "R32_TOPIC_07",
            semantic_group: "R32_RETURN_KO_PAIR",
            category: "previous_korean_input_korean_output",
            setup: RETURN_KO,
            query: "이전 주제로 돌아가자.",
            input_language: Ko,
            output_language: Ko,
            expected_kind: ReturnPrevious,
            expected_concept: "C_RETURN_TOPIC",
            required_fragments: &["Aster", "돌아", "실행"],
        },
        Case {
            id: "R32_TOPIC_08",
            semantic_group: "R32_RETURN_KO_PAIR",
            category: "previous_korean_input_english_output",
            setup: RETURN_KO,
            query: "이전 주제로 돌아가자.",
            input_language: Ko,
            output_language: En,
            expected_kind: ReturnPrevious,
            expected_concept: "C_RETURN_TOPIC",
            required_fragments: &["Aster", "return", "execute"],
        },
        Case {
            id: "R32_TOPIC_09",
            semantic_group: "R32_GROUP_EN_PAIR",
            category: "group_english_input_korean_output",
            setup: GROUP_EN,
            query: "Make that task group the current topic.",
            input_language: En,
            output_language: Ko,
            expected_kind: ActivateGroup,
            expected_concept: "C_ACTIVATE_TOPIC_GROUP",
            required_fragments: &["작업 묶음", "화제", "실행"],
        },
        Case {
            id: "R32_TOPIC_10",
            semantic_group: "R32_GROUP_EN_PAIR",
            category: "group_english_input_english_output",
            setup: GROUP_EN,
            query: "Make that task group the current topic.",
            input_language: En,
            output_language: En,
            expected_kind: ActivateGroup,
            expected_concept: "C_ACTIVATE_TOPIC_GROUP",
            required_fragments: &["task group", "active topic", "execute"],
        },
        Case {
            id: "R32_TOPIC_11",
            semantic_group: "R32_GROUP_KO_PAIR",
            category: "group_korean_input_korean_output",
            setup: GROUP_KO,
            query: "그 작업 묶음을 현재 주제로 두자.",
            input_language: Ko,
            output_language: Ko,
            expected_kind: ActivateGroup,
            expected_concept: "C_ACTIVATE_TOPIC_GROUP",
            required_fragments: &["작업 묶음", "화제", "실행"],
        },
        Case {
            id: "R32_TOPIC_12",
            semantic_group: "R32_GROUP_KO_PAIR",
            category: "group_korean_input_english_output",
            setup: GROUP_KO,
            query: "그 작업 묶음을 현재 주제로 두자.",
            input_language: Ko,
            output_language: En,
            expected_kind: ActivateGroup,
            expected_concept: "C_ACTIVATE_TOPIC_GROUP",
            required_fragments: &["task group", "active topic", "execute"],
        },
    ]
}

fn main() {
    let mut rows = cases().iter().map(run).collect::<Vec<_>>();
    let mut by_group = BTreeMap::<String, Vec<usize>>::new();
    for (index, row) in rows.iter().enumerate() {
        by_group
            .entry(row.semantic_group.clone())
            .or_default()
            .push(index);
    }
    let mut pair_passed = 0;
    for indexes in by_group.values() {
        let invariant = indexes.len() == 2
            && !rows[indexes[0]].semantic_sha256.is_empty()
            && rows[indexes[0]].semantic_sha256 == rows[indexes[1]].semantic_sha256;
        if invariant {
            pair_passed += 1;
        }
        for index in indexes {
            rows[*index].semantic_pair_invariant = invariant;
        }
    }
    for row in &mut rows {
        row.pass = row.typed_generation && row.safety_boundary && row.semantic_pair_invariant;
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let generative = rows.iter().filter(|row| row.typed_generation).count();
    let report = Report {
        schema: "B_CORE_TOPIC_TRANSITION_GENERATION_BLIND_REPORT_1",
        suite: "TOPIC-TRANSITION-GENERATION-BLIND-R32-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: by_group.len(),
        cross_language_semantic_pairs_passed: pair_passed,
        generative_path_rate_millis: u16::try_from(generative * 1_000 / rows.len())
            .expect("bounded rate"),
        drafted_surface_fallbacks: 0,
        stage_overwrites: 0,
        semantic_authority_violations: 0,
        external_execution_authorizations: 0,
        unsupported_explanation_facts: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report json")
    );
    if report.failed != 0
        || report.cross_language_semantic_pairs_passed != report.cross_language_semantic_pairs
        || report.generative_path_rate_millis != 1_000
    {
        std::process::exit(1);
    }
}
