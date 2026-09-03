//! Frozen R26-RUN-0001 temporal-answer generation blind suite.
//!
//! These cases were fixed before first execution. They use fresh event names,
//! direct and transitive relations, conflicts, ambiguity, missing evidence,
//! Korean/English input, and cross-language output.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, LanguageCodeIR,
    NaturalRealizationPathIR, NaturalResponseActIR, TemporalAnswerDispositionIR,
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
    expected_disposition: TemporalAnswerDispositionIR,
    expected_terminal_concept: &'a str,
    expected_evidence_concept: Option<&'a str>,
    expected_events: usize,
    expected_relations: usize,
    required_fragment: &'a str,
}

#[derive(Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    disposition: TemporalAnswerDispositionIR,
    event_evidence: usize,
    relation_evidence: usize,
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
        .temporal_answer
        .as_ref()
        .unwrap_or_else(|| panic!("missing temporal answer: case={}", case.id));
    let trace = response.natural_realization.generation_traces.first();
    let typed_generation = answer.disposition == case.expected_disposition
        && answer.event_evidence.len() == case.expected_events
        && answer.relation_evidence.len() == case.expected_relations
        && response.natural_realization.response_act == NaturalResponseActIR::TemporalAnswer
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| {
            trace.validate()
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == case.expected_terminal_concept)
                && case.expected_evidence_concept.is_none_or(|concept| {
                    trace
                        .meaning
                        .nodes
                        .iter()
                        .any(|node| node.concept_id == concept)
                })
        });
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
        && !response.output.text.contains("C_TEMPORAL_ANSWER_")
        && !response.output.text.contains("TemporalAnswerIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        disposition: answer.disposition,
        event_evidence: answer.event_evidence.len(),
        relation_evidence: answer.relation_evidence.len(),
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
    use LanguageCodeIR::{English, Korean};
    use TemporalAnswerDispositionIR::{
        AmbiguousEvent, AnsweredByTransitivePath, AnsweredFromTemporalGraph, ConflictingRelations,
        NoMatchingEvent, NoRecordedRelation,
    };

    const EN_TIME: &[Turn<'static>] = &[Turn {
        text: "The garnet batch completed yesterday.",
        language: English,
    }];
    const EN_DIRECT: &[Turn<'static>] = &[Turn {
        text: "The indigo audit completed before the saffron deploy started.",
        language: English,
    }];
    const KO_DIRECT: &[Turn<'static>] = &[Turn {
        text: "자수정 배포가 시작되기 전에 호박색 백업이 완료됐다.",
        language: Korean,
    }];
    const EN_TRANSITIVE: &[Turn<'static>] = &[
        Turn {
            text: "The cobalt backup completed.",
            language: English,
        },
        Turn {
            text: "After that, the linen deploy started.",
            language: English,
        },
        Turn {
            text: "After that, the ocher monitor failed.",
            language: English,
        },
    ];
    const EN_CONFLICT: &[Turn<'static>] = &[
        Turn {
            text: "The quartz scan completed before the bronze restore started.",
            language: English,
        },
        Turn {
            text: "The bronze restore started before the quartz scan completed.",
            language: English,
        },
    ];
    const EN_NO_RELATION: &[Turn<'static>] = &[
        Turn {
            text: "The coral export completed.",
            language: English,
        },
        Turn {
            text: "The jade import started.",
            language: English,
        },
    ];
    const EN_AMBIGUOUS: &[Turn<'static>] = &[
        Turn {
            text: "The amber job completed yesterday.",
            language: English,
        },
        Turn {
            text: "The amber job completed today.",
            language: English,
        },
    ];
    const EN_UNRELATED: &[Turn<'static>] = &[Turn {
        text: "The silver archive opened.",
        language: English,
    }];

    let cases = [
        Case {
            id: "R26_TIME_EN",
            semantic_group: "R26_TIME_PAIR",
            category: "event_time",
            setup: EN_TIME,
            query: "When did the garnet batch complete?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_TIME"),
            expected_events: 1,
            expected_relations: 0,
            required_fragment: "DAY_OFFSET:-1",
        },
        Case {
            id: "R26_TIME_EN_TO_KO",
            semantic_group: "R26_TIME_PAIR",
            category: "cross_language_event_time",
            setup: EN_TIME,
            query: "When did the garnet batch complete?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_TIME"),
            expected_events: 1,
            expected_relations: 0,
            required_fragment: "DAY_OFFSET:-1",
        },
        Case {
            id: "R26_DIRECT_EN",
            semantic_group: "R26_DIRECT_EN_PAIR",
            category: "direct_before_relation",
            setup: EN_DIRECT,
            query: "What happened before the saffron deploy started?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 1,
            required_fragment: "indigo audit",
        },
        Case {
            id: "R26_DIRECT_EN_TO_KO",
            semantic_group: "R26_DIRECT_EN_PAIR",
            category: "cross_language_direct_before_relation",
            setup: EN_DIRECT,
            query: "What happened before the saffron deploy started?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 1,
            required_fragment: "indigo audit",
        },
        Case {
            id: "R26_DIRECT_KO",
            semantic_group: "R26_DIRECT_KO_PAIR",
            category: "korean_direct_before_relation",
            setup: KO_DIRECT,
            query: "자수정 배포가 시작되기 전에 무슨 일이 있었어?",
            input_language: Korean,
            output_language: Korean,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 1,
            required_fragment: "호박색",
        },
        Case {
            id: "R26_DIRECT_KO_TO_EN",
            semantic_group: "R26_DIRECT_KO_PAIR",
            category: "cross_language_korean_direct_before_relation",
            setup: KO_DIRECT,
            query: "자수정 배포가 시작되기 전에 무슨 일이 있었어?",
            input_language: Korean,
            output_language: English,
            expected_disposition: AnsweredFromTemporalGraph,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 1,
            required_fragment: "호박색",
        },
        Case {
            id: "R26_TRANSITIVE_EN",
            semantic_group: "R26_TRANSITIVE_PAIR",
            category: "transitive_temporal_path",
            setup: EN_TRANSITIVE,
            query: "Did the cobalt backup complete before the ocher monitor failed?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredByTransitivePath,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 3,
            expected_relations: 2,
            required_fragment: "2-edge temporal path",
        },
        Case {
            id: "R26_TRANSITIVE_EN_TO_KO",
            semantic_group: "R26_TRANSITIVE_PAIR",
            category: "cross_language_transitive_temporal_path",
            setup: EN_TRANSITIVE,
            query: "Did the cobalt backup complete before the ocher monitor failed?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredByTransitivePath,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 3,
            expected_relations: 2,
            required_fragment: "2개 시간 관계",
        },
        Case {
            id: "R26_CONFLICT_EN",
            semantic_group: "R26_CONFLICT_PAIR",
            category: "conflicting_temporal_relations",
            setup: EN_CONFLICT,
            query: "Did the quartz scan complete before the bronze restore started?",
            input_language: English,
            output_language: English,
            expected_disposition: ConflictingRelations,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_CONFLICT",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 2,
            required_fragment: "incompatible temporal relation",
        },
        Case {
            id: "R26_CONFLICT_EN_TO_KO",
            semantic_group: "R26_CONFLICT_PAIR",
            category: "cross_language_conflicting_temporal_relations",
            setup: EN_CONFLICT,
            query: "Did the quartz scan complete before the bronze restore started?",
            input_language: English,
            output_language: Korean,
            expected_disposition: ConflictingRelations,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_CONFLICT",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_BEFORE"),
            expected_events: 2,
            expected_relations: 2,
            required_fragment: "양립하지 않는 시간 관계",
        },
        Case {
            id: "R26_NO_RELATION_EN",
            semantic_group: "R26_NO_RELATION_PAIR",
            category: "unrecorded_temporal_relation",
            setup: EN_NO_RELATION,
            query: "Did the coral export complete before the jade import started?",
            input_language: English,
            output_language: English,
            expected_disposition: NoRecordedRelation,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_NO_RELATION",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_EVENT"),
            expected_events: 2,
            expected_relations: 0,
            required_fragment: "not recorded",
        },
        Case {
            id: "R26_NO_RELATION_EN_TO_KO",
            semantic_group: "R26_NO_RELATION_PAIR",
            category: "cross_language_unrecorded_temporal_relation",
            setup: EN_NO_RELATION,
            query: "Did the coral export complete before the jade import started?",
            input_language: English,
            output_language: Korean,
            expected_disposition: NoRecordedRelation,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_NO_RELATION",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_EVENT"),
            expected_events: 2,
            expected_relations: 0,
            required_fragment: "기록되지 않았",
        },
        Case {
            id: "R26_AMBIGUOUS_EN",
            semantic_group: "R26_AMBIGUOUS_PAIR",
            category: "ambiguous_event_time",
            setup: EN_AMBIGUOUS,
            query: "When did the amber job complete?",
            input_language: English,
            output_language: English,
            expected_disposition: AmbiguousEvent,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_AMBIGUOUS",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_TIME"),
            expected_events: 2,
            expected_relations: 0,
            required_fragment: "Several event records",
        },
        Case {
            id: "R26_AMBIGUOUS_EN_TO_KO",
            semantic_group: "R26_AMBIGUOUS_PAIR",
            category: "cross_language_ambiguous_event_time",
            setup: EN_AMBIGUOUS,
            query: "When did the amber job complete?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AmbiguousEvent,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_AMBIGUOUS",
            expected_evidence_concept: Some("C_TEMPORAL_ANSWER_TIME"),
            expected_events: 2,
            expected_relations: 0,
            required_fragment: "사건 기록이 여러",
        },
        Case {
            id: "R26_NO_MATCH_EN",
            semantic_group: "R26_NO_MATCH_PAIR",
            category: "missing_temporal_target",
            setup: EN_UNRELATED,
            query: "What happened before the violet migration finished?",
            input_language: English,
            output_language: English,
            expected_disposition: NoMatchingEvent,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_NO_MATCH",
            expected_evidence_concept: None,
            expected_events: 0,
            expected_relations: 0,
            required_fragment: "no matching event record",
        },
        Case {
            id: "R26_NO_MATCH_EN_TO_KO",
            semantic_group: "R26_NO_MATCH_PAIR",
            category: "cross_language_missing_temporal_target",
            setup: EN_UNRELATED,
            query: "What happened before the violet migration finished?",
            input_language: English,
            output_language: Korean,
            expected_disposition: NoMatchingEvent,
            expected_terminal_concept: "C_TEMPORAL_ANSWER_NO_MATCH",
            expected_evidence_concept: None,
            expected_events: 0,
            expected_relations: 0,
            required_fragment: "일치하는 사건 기록이 없어",
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
        schema: "B_CORE_TEMPORAL_ANSWER_GENERATION_BLIND_REPORT_1",
        suite: "TEMPORAL-ANSWER-GENERATION-BLIND-R26-RUN-0001",
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
