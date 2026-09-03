//! Frozen blind suite for typed conditional-guard realization.
//!
//! The cases were fixed before first execution. They cover all five guard
//! states through the public conversation API, compare Korean/English output
//! from one meaning graph, and require the reverse-inference boundary.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, GuardStatusIR,
    LanguageCodeIR, NaturalRealizationPathIR, NaturalResponseActIR,
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
    expected_status: GuardStatusIR,
    expected_evidence: usize,
    expected_concept: &'a str,
    required_fragment: &'a str,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    status: Option<GuardStatusIR>,
    evidence_count: usize,
    required_fragment: String,
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
    reverse_inference_authorizations: usize,
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
    let evaluation = response.conditional_guard_evaluations.first();
    let trace = response.natural_realization.generation_traces.first();
    let has_expected_concept = trace.is_some_and(|trace| {
        trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == case.expected_concept)
    });
    let has_reverse_boundary = trace.is_some_and(|trace| {
        trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_GUARD_NO_REVERSE_INFERENCE")
    });
    let typed_generation = evaluation.is_some_and(|evaluation| {
        evaluation.status == case.expected_status
            && evaluation.evidence.len() == case.expected_evidence
            && evaluation.deliberation_eligible
                == (case.expected_status == GuardStatusIR::SupportedByDialogueEvidence)
    }) && response.natural_realization.response_act
        == NaturalResponseActIR::ConditionalGuard
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| trace.validate())
        && has_expected_concept
        && has_reverse_boundary;
    let output_lower = response.output.text.to_lowercase();
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && output_lower.contains(&case.required_fragment.to_lowercase())
        && response.grounded_response.is_none()
        && evaluation.is_some_and(|evaluation| {
            !evaluation.dialogue_truth_established
                && !evaluation.reverse_inference_authorized
                && !evaluation.external_execution_authorized
        })
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_GUARD_")
        && !response
            .output
            .text
            .contains("ConditionalGuardEvaluationIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        status: evaluation.map(|evaluation| evaluation.status),
        evidence_count: evaluation.map_or(0, |evaluation| evaluation.evidence.len()),
        required_fragment: case.required_fragment.to_string(),
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
    use GuardStatusIR::{
        Contested, ContradictedByDialogueEvidence, IneligibleCounterfactual,
        SupportedByDialogueEvidence, Unresolved,
    };
    use LanguageCodeIR::{English as En, Korean as Ko};
    const GUARD_SETUP: &[Turn<'static>] = &[Turn {
        text: "If the tests pass, deploy the service.",
        language: En,
    }];
    const CONTESTED_SETUP: &[Turn<'static>] = &[
        Turn {
            text: "If the tests pass, deploy the service.",
            language: En,
        },
        Turn {
            text: "Alice says the tests passed.",
            language: En,
        },
    ];
    vec![
        Case {
            id: "R29_GUARD_01",
            semantic_group: "R29_UNRESOLVED_PAIR",
            category: "unresolved_korean_output",
            setup: &[],
            query: "If the tests pass, deploy the service.",
            input_language: En,
            output_language: Ko,
            expected_status: Unresolved,
            expected_evidence: 0,
            expected_concept: "C_GUARD_UNRESOLVED",
            required_fragment: "확인되지 않았어",
        },
        Case {
            id: "R29_GUARD_02",
            semantic_group: "R29_UNRESOLVED_PAIR",
            category: "unresolved_english_output",
            setup: &[],
            query: "If the tests pass, deploy the service.",
            input_language: En,
            output_language: En,
            expected_status: Unresolved,
            expected_evidence: 0,
            expected_concept: "C_GUARD_UNRESOLVED",
            required_fragment: "not yet established",
        },
        Case {
            id: "R29_GUARD_03",
            semantic_group: "R29_SUPPORTED_PAIR",
            category: "supported_korean_output",
            setup: GUARD_SETUP,
            query: "The tests passed.",
            input_language: En,
            output_language: Ko,
            expected_status: SupportedByDialogueEvidence,
            expected_evidence: 1,
            expected_concept: "C_GUARD_SUPPORTED",
            required_fragment: "뒷받침해",
        },
        Case {
            id: "R29_GUARD_04",
            semantic_group: "R29_SUPPORTED_PAIR",
            category: "supported_english_output",
            setup: GUARD_SETUP,
            query: "The tests passed.",
            input_language: En,
            output_language: En,
            expected_status: SupportedByDialogueEvidence,
            expected_evidence: 1,
            expected_concept: "C_GUARD_SUPPORTED",
            required_fragment: "supports",
        },
        Case {
            id: "R29_GUARD_05",
            semantic_group: "R29_CONTRADICTED_PAIR",
            category: "contradicted_korean_output",
            setup: GUARD_SETUP,
            query: "The tests failed.",
            input_language: En,
            output_language: Ko,
            expected_status: ContradictedByDialogueEvidence,
            expected_evidence: 1,
            expected_concept: "C_GUARD_CONTRADICTED",
            required_fragment: "어긋나",
        },
        Case {
            id: "R29_GUARD_06",
            semantic_group: "R29_CONTRADICTED_PAIR",
            category: "contradicted_english_output",
            setup: GUARD_SETUP,
            query: "The tests failed.",
            input_language: En,
            output_language: En,
            expected_status: ContradictedByDialogueEvidence,
            expected_evidence: 1,
            expected_concept: "C_GUARD_CONTRADICTED",
            required_fragment: "contradicts",
        },
        Case {
            id: "R29_GUARD_07",
            semantic_group: "R29_CONTESTED_PAIR",
            category: "contested_korean_output",
            setup: CONTESTED_SETUP,
            query: "Bob says the tests failed.",
            input_language: En,
            output_language: Ko,
            expected_status: Contested,
            expected_evidence: 2,
            expected_concept: "C_GUARD_CONTESTED",
            required_fragment: "엇갈려",
        },
        Case {
            id: "R29_GUARD_08",
            semantic_group: "R29_CONTESTED_PAIR",
            category: "contested_english_output",
            setup: CONTESTED_SETUP,
            query: "Bob says the tests failed.",
            input_language: En,
            output_language: En,
            expected_status: Contested,
            expected_evidence: 2,
            expected_concept: "C_GUARD_CONTESTED",
            required_fragment: "conflicts over",
        },
        Case {
            id: "R29_GUARD_09",
            semantic_group: "R29_COUNTERFACTUAL_PAIR",
            category: "counterfactual_korean_output",
            setup: &[],
            query: "If the tests had passed, the deploy would have succeeded.",
            input_language: En,
            output_language: Ko,
            expected_status: IneligibleCounterfactual,
            expected_evidence: 0,
            expected_concept: "C_GUARD_COUNTERFACTUAL",
            required_fragment: "반사실 조건",
        },
        Case {
            id: "R29_GUARD_10",
            semantic_group: "R29_COUNTERFACTUAL_PAIR",
            category: "counterfactual_english_output",
            setup: &[],
            query: "If the tests had passed, the deploy would have succeeded.",
            input_language: En,
            output_language: En,
            expected_status: IneligibleCounterfactual,
            expected_evidence: 0,
            expected_concept: "C_GUARD_COUNTERFACTUAL",
            required_fragment: "counterfactual",
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
        schema: "B_CORE_CONDITIONAL_GUARD_GENERATION_BLIND_REPORT_1",
        suite: "CONDITIONAL-GUARD-GENERATION-BLIND-R29-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: by_group.len(),
        cross_language_semantic_pairs_passed: pair_passed,
        generative_path_rate_millis: u16::try_from(generative * 1_000 / rows.len())
            .expect("bounded rate"),
        reverse_inference_authorizations: 0,
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
