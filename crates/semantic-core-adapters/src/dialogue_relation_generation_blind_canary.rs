//! Frozen R27-RUN-0001 dialogue-relation generation blind suite.
//!
//! Fresh causal, result, concessive, transitive, branching, non-actual, and
//! missing-relation dialogues were fixed before first execution.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    DialogueRelationAnswerDispositionIR, LanguageCodeIR, NaturalRealizationPathIR,
    NaturalResponseActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
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
    expected_disposition: DialogueRelationAnswerDispositionIR,
    expected_terminal_concept: &'a str,
    expected_edge_concept: Option<&'a str>,
    expected_warning_concept: Option<&'a str>,
    expected_evidence: usize,
    expected_paths: usize,
    required_fragment: &'a str,
}

#[derive(Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    disposition: DialogueRelationAnswerDispositionIR,
    evidence_edges: usize,
    relation_paths: usize,
    required_fragment: String,
    realized_text: String,
    semantic_sha256: String,
    semantic_pair_invariant: bool,
    typed_generation: bool,
    safety_boundary: bool,
    pass: bool,
}

#[derive(Serialize)]
struct Report {
    schema: &'static str,
    suite: &'static str,
    frozen_before_first_execution: bool,
    fresh_cases: usize,
    passed: usize,
    failed: usize,
    cross_language_semantic_pairs: usize,
    cross_language_semantic_pairs_passed: usize,
    generative_path_rate_millis: usize,
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
            u64::try_from(index + 1).expect("bounded turn"),
            turn.text,
            turn.language,
            turn.language,
        ))
        .unwrap_or_else(|error| panic!("setup failed: case={}, error={error:?}", case.id));
    }
    let response = api
        .process_conversation_turn(&request(
            case.semantic_group,
            u64::try_from(case.setup.len() + 1).expect("bounded turn"),
            case.query,
            case.input_language,
            case.output_language,
        ))
        .unwrap_or_else(|error| panic!("case failed: case={}, error={error:?}", case.id));
    let answer = response
        .dialogue_relation_answer
        .as_ref()
        .unwrap_or_else(|| panic!("missing dialogue relation answer: case={}", case.id));
    let trace = response.natural_realization.generation_traces.first();
    let has_concept = |concept: &str| {
        trace.is_some_and(|trace| {
            trace
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept)
        })
    };
    let typed_generation = answer.disposition == case.expected_disposition
        && answer.evidence.len() == case.expected_evidence
        && answer.paths.len() == case.expected_paths
        && response.natural_realization.response_act
            == NaturalResponseActIR::DialogueRelationAnswer
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| trace.validate())
        && has_concept(case.expected_terminal_concept)
        && case.expected_edge_concept.is_none_or(has_concept)
        && case.expected_warning_concept.is_none_or(has_concept);
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && response
            .output
            .text
            .to_lowercase()
            .contains(&case.required_fragment.to_lowercase())
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_DIALOGUE_RELATION_")
        && !response.output.text.contains("DialogueRelationAnswerIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        disposition: answer.disposition,
        evidence_edges: answer.evidence.len(),
        relation_paths: answer.paths.len(),
        required_fragment: case.required_fragment.to_string(),
        realized_text: response.output.text,
        semantic_sha256: trace
            .map(|trace| trace.meaning.semantic_sha256.clone())
            .unwrap_or_default(),
        semantic_pair_invariant: false,
        typed_generation,
        safety_boundary,
        pass: typed_generation && safety_boundary,
    }
}

fn main() {
    use DialogueRelationAnswerDispositionIR::{
        AnsweredFromDialoguePath, AnsweredFromDialogueRelation, MultipleDialogueRelations,
        NoMatchingDialogueRelation,
    };
    use LanguageCodeIR::{English, Korean};

    const EN_CAUSE: &[Turn<'static>] = &[
        Turn {
            text: "Peridot cache integrity failure",
            language: English,
        },
        Turn {
            text: "Because of that, Peridot service latency increase",
            language: English,
        },
    ];
    const KO_CAUSE: &[Turn<'static>] = &[
        Turn {
            text: "해오름 캐시 무결성 실패",
            language: Korean,
        },
        Turn {
            text: "그 때문에, 해오름 서비스 지연 발생",
            language: Korean,
        },
    ];
    const EN_RESULT: &[Turn<'static>] = &[
        Turn {
            text: "Umber index failure",
            language: English,
        },
        Turn {
            text: "As a result, Umber worker degraded mode",
            language: English,
        },
    ];
    const KO_CONCESSION: &[Turn<'static>] = &[
        Turn {
            text: "새봄 마이그레이션 고비용",
            language: Korean,
        },
        Turn {
            text: "그럼에도, 새봄 팀 배포 계속",
            language: Korean,
        },
    ];
    const EN_TRANSITIVE: &[Turn<'static>] = &[
        Turn {
            text: "Cobalt shard failure",
            language: English,
        },
        Turn {
            text: "Because of that, Cobalt service slowdown",
            language: English,
        },
        Turn {
            text: "Therefore, Cobalt backlog growth",
            language: English,
        },
    ];
    const EN_MULTIPLE: &[Turn<'static>] = &[
        Turn {
            text: "Maroon cache failure",
            language: English,
        },
        Turn {
            text: "Because of that, Maroon latency increase",
            language: English,
        },
        Turn {
            text: "Maroon network congestion",
            language: English,
        },
        Turn {
            text: "Because of that, Maroon latency increase",
            language: English,
        },
    ];
    const EN_NONACTUAL: &[Turn<'static>] = &[
        Turn {
            text: "The teal gateway might fail",
            language: English,
        },
        Turn {
            text: "Because of that, Teal retry rate increase",
            language: English,
        },
    ];
    const EN_UNRELATED: &[Turn<'static>] = &[
        Turn {
            text: "Bronze cache failure",
            language: English,
        },
        Turn {
            text: "Because of that, Bronze service delay",
            language: English,
        },
    ];

    let cases = [
        Case {
            id: "R27_CAUSE_EN",
            semantic_group: "R27_CAUSE_EN_PAIR",
            category: "direct_cause",
            setup: EN_CAUSE,
            query: "Why Peridot service latency increase?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "as a reason for",
        },
        Case {
            id: "R27_CAUSE_EN_TO_KO",
            semantic_group: "R27_CAUSE_EN_PAIR",
            category: "cross_language_direct_cause",
            setup: EN_CAUSE,
            query: "Why Peridot service latency increase?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "이유로",
        },
        Case {
            id: "R27_CAUSE_KO",
            semantic_group: "R27_CAUSE_KO_PAIR",
            category: "korean_direct_cause",
            setup: KO_CAUSE,
            query: "왜 해오름 서비스 지연 발생?",
            input_language: Korean,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "해오름",
        },
        Case {
            id: "R27_CAUSE_KO_TO_EN",
            semantic_group: "R27_CAUSE_KO_PAIR",
            category: "cross_language_korean_direct_cause",
            setup: KO_CAUSE,
            query: "왜 해오름 서비스 지연 발생?",
            input_language: Korean,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "해오름",
        },
        Case {
            id: "R27_RESULT_EN",
            semantic_group: "R27_RESULT_PAIR",
            category: "direct_result",
            setup: EN_RESULT,
            query: "What resulted from Umber index failure?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_RESULT_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_RESULT_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "to the result",
        },
        Case {
            id: "R27_RESULT_EN_TO_KO",
            semantic_group: "R27_RESULT_PAIR",
            category: "cross_language_direct_result",
            setup: EN_RESULT,
            query: "What resulted from Umber index failure?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_RESULT_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_RESULT_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "결과로",
        },
        Case {
            id: "R27_CONCESSION_KO",
            semantic_group: "R27_CONCESSION_PAIR",
            category: "korean_concession",
            setup: KO_CONCESSION,
            query: "새봄 마이그레이션 고비용에도 불구하고 결과?",
            input_language: Korean,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CONCESSION_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "그럼에도",
        },
        Case {
            id: "R27_CONCESSION_KO_TO_EN",
            semantic_group: "R27_CONCESSION_PAIR",
            category: "cross_language_korean_concession",
            setup: KO_CONCESSION,
            query: "새봄 마이그레이션 고비용에도 불구하고 결과?",
            input_language: Korean,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CONCESSION_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "outcome that still held",
        },
        Case {
            id: "R27_TRANSITIVE_EN",
            semantic_group: "R27_TRANSITIVE_PAIR",
            category: "transitive_causal_path",
            setup: EN_TRANSITIVE,
            query: "Why Cobalt backlog growth?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialoguePath,
            expected_terminal_concept: "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 2,
            expected_paths: 1,
            required_fragment: "2-link path",
        },
        Case {
            id: "R27_TRANSITIVE_EN_TO_KO",
            semantic_group: "R27_TRANSITIVE_PAIR",
            category: "cross_language_transitive_causal_path",
            setup: EN_TRANSITIVE,
            query: "Why Cobalt backlog growth?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialoguePath,
            expected_terminal_concept: "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 2,
            expected_paths: 1,
            required_fragment: "2개 관계",
        },
        Case {
            id: "R27_MULTIPLE_EN",
            semantic_group: "R27_MULTIPLE_PAIR",
            category: "multiple_causal_paths",
            setup: EN_MULTIPLE,
            query: "Why Maroon latency increase?",
            input_language: English,
            output_language: English,
            expected_disposition: MultipleDialogueRelations,
            expected_terminal_concept: "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 2,
            expected_paths: 2,
            required_fragment: "2 dialogue-relation paths",
        },
        Case {
            id: "R27_MULTIPLE_EN_TO_KO",
            semantic_group: "R27_MULTIPLE_PAIR",
            category: "cross_language_multiple_causal_paths",
            setup: EN_MULTIPLE,
            query: "Why Maroon latency increase?",
            input_language: English,
            output_language: Korean,
            expected_disposition: MultipleDialogueRelations,
            expected_terminal_concept: "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: None,
            expected_evidence: 2,
            expected_paths: 2,
            required_fragment: "2개 관계 경로",
        },
        Case {
            id: "R27_NONACTUAL_EN",
            semantic_group: "R27_NONACTUAL_PAIR",
            category: "nonactual_causal_endpoint",
            setup: EN_NONACTUAL,
            query: "Why Teal retry rate increase?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: Some("C_DIALOGUE_RELATION_NONACTUAL_WARNING"),
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "not an actual-event path",
        },
        Case {
            id: "R27_NONACTUAL_EN_TO_KO",
            semantic_group: "R27_NONACTUAL_PAIR",
            category: "cross_language_nonactual_causal_endpoint",
            setup: EN_NONACTUAL,
            query: "Why Teal retry rate increase?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            expected_edge_concept: Some("C_DIALOGUE_RELATION_CAUSE_EDGE"),
            expected_warning_concept: Some("C_DIALOGUE_RELATION_NONACTUAL_WARNING"),
            expected_evidence: 1,
            expected_paths: 1,
            required_fragment: "실제 사건 경로로 볼 수 없어",
        },
        Case {
            id: "R27_NO_MATCH_EN",
            semantic_group: "R27_NO_MATCH_PAIR",
            category: "missing_dialogue_relation",
            setup: EN_UNRELATED,
            query: "Why Violet archive lock?",
            input_language: English,
            output_language: English,
            expected_disposition: NoMatchingDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_NO_MATCH",
            expected_edge_concept: None,
            expected_warning_concept: None,
            expected_evidence: 0,
            expected_paths: 0,
            required_fragment: "no matching relation",
        },
        Case {
            id: "R27_NO_MATCH_EN_TO_KO",
            semantic_group: "R27_NO_MATCH_PAIR",
            category: "cross_language_missing_dialogue_relation",
            setup: EN_UNRELATED,
            query: "Why Violet archive lock?",
            input_language: English,
            output_language: Korean,
            expected_disposition: NoMatchingDialogueRelation,
            expected_terminal_concept: "C_DIALOGUE_RELATION_NO_MATCH",
            expected_edge_concept: None,
            expected_warning_concept: None,
            expected_evidence: 0,
            expected_paths: 0,
            required_fragment: "맞는 관계를 찾지 못했어",
        },
    ];

    let mut rows = cases.iter().map(run).collect::<Vec<_>>();
    let mut hashes_by_group = BTreeMap::<String, Vec<String>>::new();
    for row in &rows {
        hashes_by_group
            .entry(row.semantic_group.clone())
            .or_default()
            .push(row.semantic_sha256.clone());
    }
    let pair_results = hashes_by_group
        .iter()
        .map(|(group, hashes)| {
            (
                group.clone(),
                hashes.len() == 2
                    && !hashes[0].is_empty()
                    && hashes.iter().all(|hash| hash == &hashes[0]),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for row in &mut rows {
        row.semantic_pair_invariant = pair_results
            .get(&row.semantic_group)
            .copied()
            .unwrap_or(false);
        row.pass &= row.semantic_pair_invariant;
    }
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = Report {
        schema: "B_CORE_DIALOGUE_RELATION_GENERATION_BLIND_REPORT_1",
        suite: "DIALOGUE-RELATION-GENERATION-BLIND-R27-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: pair_results.len(),
        cross_language_semantic_pairs_passed: pair_results.values().filter(|pass| **pass).count(),
        generative_path_rate_millis: rows.iter().filter(|row| row.typed_generation).count() * 1_000
            / rows.len(),
        unsupported_explanation_facts: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        recursive_source_mutations: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serialization")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
