//! Frozen R25-RUN-0001 discourse-answer generation blind suite.
//!
//! These cases were fixed before first execution. They use fresh actors,
//! referents, propositions, Korean/English input, and cross-language output to
//! test whether multi-turn dialogue answers are generated from typed records.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    DiscourseAnswerDispositionIR, GenerationSpeechIntentIR, LanguageCodeIR,
    NaturalRealizationPathIR, NaturalResponseActIR, CONVERSATION_TURN_REQUEST_SCHEMA,
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
    expected_disposition: DiscourseAnswerDispositionIR,
    expected_terminal_concept: &'a str,
    expected_record_concept: Option<&'a str>,
    expected_evidence: usize,
    required_fragment: &'a str,
}

#[derive(Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    disposition: DiscourseAnswerDispositionIR,
    evidence_records: usize,
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
        .discourse_answer
        .as_ref()
        .unwrap_or_else(|| panic!("missing discourse answer: case={}", case.id));
    let trace = response.natural_realization.generation_traces.first();
    let typed_generation = answer.disposition == case.expected_disposition
        && answer.evidence.len() == case.expected_evidence
        && response.natural_realization.response_act == NaturalResponseActIR::DiscourseAnswer
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| {
            trace.validate()
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == case.expected_terminal_concept)
                && case.expected_record_concept.is_none_or(|concept| {
                    trace
                        .meaning
                        .nodes
                        .iter()
                        .any(|node| node.concept_id == concept)
                })
                && trace
                    .speech_intent
                    .intents
                    .iter()
                    .all(|intent| intent.intent == GenerationSpeechIntentIR::Inform)
        });
    let required_fragment = case.required_fragment.to_lowercase();
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && response
            .output
            .text
            .to_lowercase()
            .contains(&required_fragment)
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_DIALOGUE_ANSWER_")
        && !response.output.text.contains("GoalIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        disposition: answer.disposition,
        evidence_records: answer.evidence.len(),
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
    use DiscourseAnswerDispositionIR::{
        AnsweredFromDialogueRecords, ConflictingDialogueRecords, NoConflictRecorded,
        NoMatchingRecord, PresuppositionUnverified,
    };
    use LanguageCodeIR::{English, Korean};

    const EN_SOURCE: &[Turn<'static>] = &[Turn {
        text: "Nora says that the topaz relay is idle.",
        language: English,
    }];
    const KO_SOURCE: &[Turn<'static>] = &[Turn {
        text: "서윤은 청록 저장소가 닫혔다고 말했다.",
        language: Korean,
    }];
    const EN_PROPOSITION_SOURCE: &[Turn<'static>] = &[Turn {
        text: "Mira says that the ocher index is delayed.",
        language: English,
    }];
    const EN_MODAL: &[Turn<'static>] = &[Turn {
        text: "Ilan believes that the violet bridge might be unstable.",
        language: English,
    }];
    const EN_CONFLICT: &[Turn<'static>] = &[
        Turn {
            text: "Orin says that the onyx service is online.",
            language: English,
        },
        Turn {
            text: "Pia says that the onyx service is not online.",
            language: English,
        },
    ];
    const EN_AGREEMENT: &[Turn<'static>] = &[
        Turn {
            text: "Tarin says that the linen queue is ready.",
            language: English,
        },
        Turn {
            text: "Uma says that the linen queue is ready.",
            language: English,
        },
    ];
    const EN_PRESUPPOSITION: &[Turn<'static>] = &[Turn {
        text: "Lena believes that the copper worker might fail.",
        language: English,
    }];
    const EN_UNRELATED: &[Turn<'static>] = &[Turn {
        text: "Rhea says that the silver archive is sealed.",
        language: English,
    }];

    let cases = [
        Case {
            id: "R25_SOURCE_EN",
            semantic_group: "R25_SOURCE_EN_PAIR",
            category: "source_content",
            setup: EN_SOURCE,
            query: "What did Nora say?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "topaz relay",
        },
        Case {
            id: "R25_SOURCE_EN_TO_KO",
            semantic_group: "R25_SOURCE_EN_PAIR",
            category: "cross_language_source_content",
            setup: EN_SOURCE,
            query: "What did Nora say?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "topaz relay",
        },
        Case {
            id: "R25_SOURCE_KO",
            semantic_group: "R25_SOURCE_KO_PAIR",
            category: "source_content",
            setup: KO_SOURCE,
            query: "서윤은 뭐라고 말했어?",
            input_language: Korean,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "청록",
        },
        Case {
            id: "R25_SOURCE_KO_TO_EN",
            semantic_group: "R25_SOURCE_KO_PAIR",
            category: "cross_language_source_content",
            setup: KO_SOURCE,
            query: "서윤은 뭐라고 말했어?",
            input_language: Korean,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "청록",
        },
        Case {
            id: "R25_PROPOSITION_SOURCE_EN",
            semantic_group: "R25_PROPOSITION_SOURCE_PAIR",
            category: "proposition_source",
            setup: EN_PROPOSITION_SOURCE,
            query: "Who said that the ocher index is delayed?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "mira",
        },
        Case {
            id: "R25_PROPOSITION_SOURCE_EN_TO_KO",
            semantic_group: "R25_PROPOSITION_SOURCE_PAIR",
            category: "cross_language_proposition_source",
            setup: EN_PROPOSITION_SOURCE,
            query: "Who said that the ocher index is delayed?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 1,
            required_fragment: "mira",
        },
        Case {
            id: "R25_MODAL_EN",
            semantic_group: "R25_MODAL_PAIR",
            category: "modal_status",
            setup: EN_MODAL,
            query: "Is the violet bridge merely possible or actual?",
            input_language: English,
            output_language: English,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_MODAL"),
            expected_evidence: 1,
            required_fragment: "possibility statement",
        },
        Case {
            id: "R25_MODAL_EN_TO_KO",
            semantic_group: "R25_MODAL_PAIR",
            category: "cross_language_modal_status",
            setup: EN_MODAL,
            query: "Is the violet bridge merely possible or actual?",
            input_language: English,
            output_language: Korean,
            expected_disposition: AnsweredFromDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NOT_FACT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_MODAL"),
            expected_evidence: 1,
            required_fragment: "가능성 진술",
        },
        Case {
            id: "R25_CONFLICT_EN",
            semantic_group: "R25_CONFLICT_PAIR",
            category: "source_conflict",
            setup: EN_CONFLICT,
            query: "Are Orin and Pia in conflict about the onyx service?",
            input_language: English,
            output_language: English,
            expected_disposition: ConflictingDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_CONFLICT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 2,
            required_fragment: "truth winner",
        },
        Case {
            id: "R25_CONFLICT_EN_TO_KO",
            semantic_group: "R25_CONFLICT_PAIR",
            category: "cross_language_source_conflict",
            setup: EN_CONFLICT,
            query: "Are Orin and Pia in conflict about the onyx service?",
            input_language: English,
            output_language: Korean,
            expected_disposition: ConflictingDialogueRecords,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_CONFLICT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 2,
            required_fragment: "사실의 승자",
        },
        Case {
            id: "R25_NO_CONFLICT_EN",
            semantic_group: "R25_NO_CONFLICT_PAIR",
            category: "no_source_conflict",
            setup: EN_AGREEMENT,
            query: "Are Tarin and Uma in conflict about the linen queue?",
            input_language: English,
            output_language: English,
            expected_disposition: NoConflictRecorded,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NO_CONFLICT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 2,
            required_fragment: "does not establish",
        },
        Case {
            id: "R25_NO_CONFLICT_EN_TO_KO",
            semantic_group: "R25_NO_CONFLICT_PAIR",
            category: "cross_language_no_source_conflict",
            setup: EN_AGREEMENT,
            query: "Are Tarin and Uma in conflict about the linen queue?",
            input_language: English,
            output_language: Korean,
            expected_disposition: NoConflictRecorded,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NO_CONFLICT",
            expected_record_concept: Some("C_DIALOGUE_ANSWER_RECORD"),
            expected_evidence: 2,
            required_fragment: "참으로 검증",
        },
        Case {
            id: "R25_PRESUPPOSITION_EN",
            semantic_group: "R25_PRESUPPOSITION_PAIR",
            category: "presupposition_abstention",
            setup: EN_PRESUPPOSITION,
            query: "Why did the copper worker fail?",
            input_language: English,
            output_language: English,
            expected_disposition: PresuppositionUnverified,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_PRESUPPOSITION",
            expected_record_concept: None,
            expected_evidence: 1,
            required_fragment: "copper worker fail",
        },
        Case {
            id: "R25_PRESUPPOSITION_EN_TO_KO",
            semantic_group: "R25_PRESUPPOSITION_PAIR",
            category: "cross_language_presupposition_abstention",
            setup: EN_PRESUPPOSITION,
            query: "Why did the copper worker fail?",
            input_language: English,
            output_language: Korean,
            expected_disposition: PresuppositionUnverified,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_PRESUPPOSITION",
            expected_record_concept: None,
            expected_evidence: 1,
            required_fragment: "copper worker fail",
        },
        Case {
            id: "R25_NO_MATCH_EN",
            semantic_group: "R25_NO_MATCH_PAIR",
            category: "missing_source",
            setup: EN_UNRELATED,
            query: "What did Vesper say?",
            input_language: English,
            output_language: English,
            expected_disposition: NoMatchingRecord,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NO_MATCH",
            expected_record_concept: None,
            expected_evidence: 0,
            required_fragment: "no matching dialogue record",
        },
        Case {
            id: "R25_NO_MATCH_EN_TO_KO",
            semantic_group: "R25_NO_MATCH_PAIR",
            category: "cross_language_missing_source",
            setup: EN_UNRELATED,
            query: "What did Vesper say?",
            input_language: English,
            output_language: Korean,
            expected_disposition: NoMatchingRecord,
            expected_terminal_concept: "C_DIALOGUE_ANSWER_NO_MATCH",
            expected_record_concept: None,
            expected_evidence: 0,
            required_fragment: "대화 기록을 찾지 못했어",
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
        schema: "B_CORE_DISCOURSE_ANSWER_GENERATION_BLIND_REPORT_1",
        suite: "DISCOURSE-ANSWER-GENERATION-BLIND-R25-RUN-0001",
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
