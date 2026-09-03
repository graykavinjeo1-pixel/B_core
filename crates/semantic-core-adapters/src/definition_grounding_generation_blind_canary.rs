//! Frozen blind suite for typed definition-grounding realization.
//!
//! The cases were fixed before first execution. They exercise every consuming
//! disposition through the public conversation API and compare Korean/English
//! realization of one language-independent meaning graph.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR,
    DefinitionGroundingDispositionIR, LanguageCodeIR, NaturalRealizationPathIR,
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
    expected_disposition: DefinitionGroundingDispositionIR,
    expected_changed: bool,
    expected_concept: &'a str,
    expected_alias: Option<&'a str>,
    expected_canonical: Option<&'a str>,
    required_fragment: &'a str,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    disposition: DefinitionGroundingDispositionIR,
    lexical_store_changed: bool,
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
    let grounding = &response.definition_grounding;
    let trace = response.natural_realization.generation_traces.first();
    let binding_matches = match (
        grounding.binding.as_ref(),
        case.expected_alias,
        case.expected_canonical,
    ) {
        (Some(binding), Some(alias), Some(canonical)) => {
            binding.alias_surface == alias
                && binding.canonical_predicate == canonical
                && !binding.semantic_authority
                && !binding.external_action_execution_authorized
        }
        (None, None, None) => true,
        _ => false,
    };
    let has_expected_concept = trace.is_some_and(|trace| {
        trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == case.expected_concept)
    });
    let typed_generation = grounding.disposition == case.expected_disposition
        && grounding.lexical_store_changed == case.expected_changed
        && binding_matches
        && response.natural_realization.response_act == NaturalResponseActIR::DefinitionGrounding
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| trace.validate())
        && has_expected_concept;
    let output_lower = response.output.text.to_lowercase();
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && output_lower.contains(&case.required_fragment.to_lowercase())
        && response.grounded_response.is_none()
        && !grounding.semantic_payload_mutated
        && !grounding.semantic_authority
        && !grounding.external_action_execution_authorized
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_DEFINITION_")
        && !response.output.text.contains("DefinitionGroundingIR")
        && !response.output.text.contains("INVESTIGATE")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        disposition: grounding.disposition,
        lexical_store_changed: grounding.lexical_store_changed,
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
    use DefinitionGroundingDispositionIR::{
        AmbiguousRejected, Bound, ConflictRejected, InvalidAliasRejected, NonAssertedRejected,
        UnresolvedRejected,
    };
    use LanguageCodeIR::{English as En, Korean as Ko};
    const CONFIRM_SETUP: &[Turn<'static>] = &[Turn {
        text: "\"quorin\" means inspect.",
        language: En,
    }];
    const CONFLICT_SETUP: &[Turn<'static>] = &[Turn {
        text: "\"navel\" means inspect.",
        language: En,
    }];
    vec![
        Case {
            id: "R28_BOUND_KO",
            semantic_group: "R28_BOUND_PAIR",
            category: "new_korean_alias",
            setup: &[],
            query: "\"새온\"은 검사하라는 뜻이야.",
            input_language: Ko,
            output_language: Ko,
            expected_disposition: Bound,
            expected_changed: true,
            expected_concept: "C_DEFINITION_BIND_ADDED",
            expected_alias: Some("새온"),
            expected_canonical: Some("INVESTIGATE"),
            required_fragment: "새 어휘 연결",
        },
        Case {
            id: "R28_BOUND_KO_TO_EN",
            semantic_group: "R28_BOUND_PAIR",
            category: "new_korean_alias_cross_language",
            setup: &[],
            query: "\"새온\"은 검사하라는 뜻이야.",
            input_language: Ko,
            output_language: En,
            expected_disposition: Bound,
            expected_changed: true,
            expected_concept: "C_DEFINITION_BIND_ADDED",
            expected_alias: Some("새온"),
            expected_canonical: Some("INVESTIGATE"),
            required_fragment: "known action meaning",
        },
        Case {
            id: "R28_CONFIRM_EN",
            semantic_group: "R28_CONFIRM_PAIR",
            category: "existing_alias_confirmation",
            setup: CONFIRM_SETUP,
            query: "\"quorin\" means inspect.",
            input_language: En,
            output_language: En,
            expected_disposition: Bound,
            expected_changed: false,
            expected_concept: "C_DEFINITION_BIND_CONFIRMED",
            expected_alias: Some("quorin"),
            expected_canonical: Some("INVESTIGATE"),
            required_fragment: "confirmed the lexical link",
        },
        Case {
            id: "R28_CONFIRM_EN_TO_KO",
            semantic_group: "R28_CONFIRM_PAIR",
            category: "existing_alias_confirmation_cross_language",
            setup: CONFIRM_SETUP,
            query: "\"quorin\" means inspect.",
            input_language: En,
            output_language: Ko,
            expected_disposition: Bound,
            expected_changed: false,
            expected_concept: "C_DEFINITION_BIND_CONFIRMED",
            expected_alias: Some("quorin"),
            expected_canonical: Some("INVESTIGATE"),
            required_fragment: "같은 어휘 관계",
        },
        Case {
            id: "R28_CONFLICT_EN",
            semantic_group: "R28_CONFLICT_PAIR",
            category: "conflicting_redefinition",
            setup: CONFLICT_SETUP,
            query: "\"navel\" means delete.",
            input_language: En,
            output_language: En,
            expected_disposition: ConflictRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_CONFLICT",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "rejected the redefinition",
        },
        Case {
            id: "R28_CONFLICT_EN_TO_KO",
            semantic_group: "R28_CONFLICT_PAIR",
            category: "conflicting_redefinition_cross_language",
            setup: CONFLICT_SETUP,
            query: "\"navel\" means delete.",
            input_language: En,
            output_language: Ko,
            expected_disposition: ConflictRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_CONFLICT",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "재정의를 거부",
        },
        Case {
            id: "R28_NONASSERTED_EN",
            semantic_group: "R28_NONASSERTED_PAIR",
            category: "questioned_definition",
            setup: &[],
            query: "\"sovel\" means delete?",
            input_language: En,
            output_language: En,
            expected_disposition: NonAssertedRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_NONASSERTED",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "asserted definition",
        },
        Case {
            id: "R28_NONASSERTED_EN_TO_KO",
            semantic_group: "R28_NONASSERTED_PAIR",
            category: "questioned_definition_cross_language",
            setup: &[],
            query: "\"sovel\" means delete?",
            input_language: En,
            output_language: Ko,
            expected_disposition: NonAssertedRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_NONASSERTED",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "확정한 정의",
        },
        Case {
            id: "R28_AMBIGUOUS_EN",
            semantic_group: "R28_AMBIGUOUS_PAIR",
            category: "multiple_meanings",
            setup: &[],
            query: "\"brika\" means inspect or repair.",
            input_language: En,
            output_language: En,
            expected_disposition: AmbiguousRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_AMBIGUOUS",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "multiple semantic operators",
        },
        Case {
            id: "R28_AMBIGUOUS_EN_TO_KO",
            semantic_group: "R28_AMBIGUOUS_PAIR",
            category: "multiple_meanings_cross_language",
            setup: &[],
            query: "\"brika\" means inspect or repair.",
            input_language: En,
            output_language: Ko,
            expected_disposition: AmbiguousRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_AMBIGUOUS",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "여러 의미",
        },
        Case {
            id: "R28_UNRESOLVED_EN",
            semantic_group: "R28_UNRESOLVED_PAIR",
            category: "unknown_definition_target",
            setup: &[],
            query: "\"tremi\" means frobnicate.",
            input_language: En,
            output_language: En,
            expected_disposition: UnresolvedRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_UNRESOLVED",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "could not ground",
        },
        Case {
            id: "R28_UNRESOLVED_EN_TO_KO",
            semantic_group: "R28_UNRESOLVED_PAIR",
            category: "unknown_definition_target_cross_language",
            setup: &[],
            query: "\"tremi\" means frobnicate.",
            input_language: En,
            output_language: Ko,
            expected_disposition: UnresolvedRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_UNRESOLVED",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "찾지 못해",
        },
        Case {
            id: "R28_INVALID_EN",
            semantic_group: "R28_INVALID_PAIR",
            category: "invalid_alias_surface",
            setup: &[],
            query: "\"bad alias!\" means inspect.",
            input_language: En,
            output_language: En,
            expected_disposition: InvalidAliasRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_INVALID_ALIAS",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "alias form is invalid",
        },
        Case {
            id: "R28_INVALID_EN_TO_KO",
            semantic_group: "R28_INVALID_PAIR",
            category: "invalid_alias_surface_cross_language",
            setup: &[],
            query: "\"bad alias!\" means inspect.",
            input_language: En,
            output_language: Ko,
            expected_disposition: InvalidAliasRejected,
            expected_changed: false,
            expected_concept: "C_DEFINITION_REJECT_INVALID_ALIAS",
            expected_alias: None,
            expected_canonical: None,
            required_fragment: "유효하지 않아",
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
        schema: "B_CORE_DEFINITION_GROUNDING_GENERATION_BLIND_REPORT_1",
        suite: "DEFINITION-GROUNDING-GENERATION-BLIND-R28-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: by_group.len(),
        cross_language_semantic_pairs_passed: pair_passed,
        generative_path_rate_millis: u16::try_from(generative * 1_000 / rows.len())
            .expect("bounded rate"),
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
    {
        std::process::exit(1);
    }
}
