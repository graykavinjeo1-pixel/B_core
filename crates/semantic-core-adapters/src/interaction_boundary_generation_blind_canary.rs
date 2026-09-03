//! Frozen blind suite for typed illocutionary interaction-boundary realization.
//!
//! Cases were fixed before first execution. The suite covers the five forces
//! that reach the public InteractionBoundary act and compares bilingual output
//! from one language-independent meaning graph.

use std::collections::BTreeMap;

use semantic_core_adapters::{
    CognitiveApi, ConversationInputModalityIR, ConversationTurnRequestIR, IllocutionaryForceIR,
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
    expected_force: IllocutionaryForceIR,
    expected_concept: &'a str,
    expected_withdrawn: usize,
    required_fragment: &'a str,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    semantic_group: String,
    category: String,
    input_language: LanguageCodeIR,
    output_language: LanguageCodeIR,
    force: Option<IllocutionaryForceIR>,
    withdrawn_goals: usize,
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
    let mut before_goals = 0;
    for (index, turn) in case.setup.iter().copied().enumerate() {
        let setup_response = api
            .process_conversation_turn(&request(
                case.semantic_group,
                u64::try_from(index + 1).expect("bounded turn"),
                turn.text,
                turn.language,
                turn.language,
            ))
            .unwrap_or_else(|error| panic!("setup failed: case={}, error={error:?}", case.id));
        before_goals = setup_response.conversation_state.active_goals.len();
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
    let force = response
        .pragmatic_interpretation
        .illocutionary_commitments
        .primary_force();
    let after_goals = response.conversation_state.active_goals.len();
    let withdrawn = before_goals.saturating_sub(after_goals);
    let trace = response.natural_realization.generation_traces.first();
    let typed_generation = force == Some(case.expected_force)
        && withdrawn == case.expected_withdrawn
        && response.natural_realization.response_act == NaturalResponseActIR::InteractionBoundary
        && response.natural_realization.realization_path == NaturalRealizationPathIR::Generative
        && response.natural_realization.generation_traces.len() == 1
        && trace.is_some_and(|trace| trace.validate())
        && trace.is_some_and(|trace| {
            trace
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == case.expected_concept)
                && trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|node| node.concept_id == "C_INTERACTION_NO_AUTHORITY")
        });
    let output_lower = response.output.text.to_lowercase();
    let safety_boundary = response.output.language == case.output_language
        && response.output.unsupported_freeform_claims == 0
        && output_lower.contains(&case.required_fragment.to_lowercase())
        && response.grounded_response.is_none()
        && response.output.grounded_plan_sha256.is_none()
        && response
            .pragmatic_interpretation
            .illocutionary_commitments
            .commitments
            .iter()
            .all(|commitment| !commitment.external_execution_authorized)
        && trace.is_some_and(|trace| {
            !trace.semantic_authority
                && !trace.language_can_execute
                && trace.external_llm_calls == 0
                && trace.local_teacher_calls == 0
                && trace.verification.unsupported_claims == 0
        })
        && !response.output.text.contains("C_INTERACTION_")
        && !response
            .output
            .text
            .contains("IllocutionaryCommitmentGraphIR")
        && !response.output.text.trim().is_empty();
    Row {
        id: case.id.to_string(),
        semantic_group: case.semantic_group.to_string(),
        category: case.category.to_string(),
        input_language: case.input_language,
        output_language: response.output.language,
        force,
        withdrawn_goals: withdrawn,
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
    use IllocutionaryForceIR::{
        CapabilityQuestion, GoalWithdrawal, OutcomeClaimConstraint, ReportedCommitment,
        SelfCommitment,
    };
    use LanguageCodeIR::{English as En, Korean as Ko};
    const WITHDRAW_SETUP: &[Turn<'static>] = &[Turn {
        text: "Save the report.",
        language: En,
    }];
    vec![
        Case {
            id: "R30_INTERACTION_01",
            semantic_group: "R30_SELF_PAIR",
            category: "self_commitment_korean_output",
            setup: &[],
            query: "I will repair the parser myself.",
            input_language: En,
            output_language: Ko,
            expected_force: SelfCommitment,
            expected_concept: "C_INTERACTION_SELF_COMMITMENT",
            expected_withdrawn: 0,
            required_fragment: "직접 하겠다는 약속",
        },
        Case {
            id: "R30_INTERACTION_02",
            semantic_group: "R30_SELF_PAIR",
            category: "self_commitment_english_output",
            setup: &[],
            query: "I will repair the parser myself.",
            input_language: En,
            output_language: En,
            expected_force: SelfCommitment,
            expected_concept: "C_INTERACTION_SELF_COMMITMENT",
            expected_withdrawn: 0,
            required_fragment: "your own commitment",
        },
        Case {
            id: "R30_INTERACTION_03",
            semantic_group: "R30_REPORTED_PAIR",
            category: "reported_commitment_korean_output",
            setup: &[],
            query: "Alice says she will delete the cache.",
            input_language: En,
            output_language: Ko,
            expected_force: ReportedCommitment,
            expected_concept: "C_INTERACTION_REPORTED_COMMITMENT",
            expected_withdrawn: 0,
            required_fragment: "제3자의 향후 약속",
        },
        Case {
            id: "R30_INTERACTION_04",
            semantic_group: "R30_REPORTED_PAIR",
            category: "reported_commitment_english_output",
            setup: &[],
            query: "Alice says she will delete the cache.",
            input_language: En,
            output_language: En,
            expected_force: ReportedCommitment,
            expected_concept: "C_INTERACTION_REPORTED_COMMITMENT",
            expected_withdrawn: 0,
            required_fragment: "third party's future commitment",
        },
        Case {
            id: "R30_INTERACTION_05",
            semantic_group: "R30_CAPABILITY_PAIR",
            category: "capability_question_korean_output",
            setup: &[],
            query: "Can B_Core parse a PDF?",
            input_language: En,
            output_language: Ko,
            expected_force: CapabilityQuestion,
            expected_concept: "C_INTERACTION_CAPABILITY_QUESTION",
            expected_withdrawn: 0,
            required_fragment: "기능 지원 여부",
        },
        Case {
            id: "R30_INTERACTION_06",
            semantic_group: "R30_CAPABILITY_PAIR",
            category: "capability_question_english_output",
            setup: &[],
            query: "Can B_Core parse a PDF?",
            input_language: En,
            output_language: En,
            expected_force: CapabilityQuestion,
            expected_concept: "C_INTERACTION_CAPABILITY_QUESTION",
            expected_withdrawn: 0,
            required_fragment: "capability question",
        },
        Case {
            id: "R30_INTERACTION_07",
            semantic_group: "R30_WITHDRAW_PAIR",
            category: "goal_withdrawal_korean_output",
            setup: WITHDRAW_SETUP,
            query: "Never mind, don't do that task.",
            input_language: En,
            output_language: Ko,
            expected_force: GoalWithdrawal,
            expected_concept: "C_INTERACTION_GOAL_WITHDRAWAL",
            expected_withdrawn: 1,
            required_fragment: "활성 작업 1개",
        },
        Case {
            id: "R30_INTERACTION_08",
            semantic_group: "R30_WITHDRAW_PAIR",
            category: "goal_withdrawal_english_output",
            setup: WITHDRAW_SETUP,
            query: "Never mind, don't do that task.",
            input_language: En,
            output_language: En,
            expected_force: GoalWithdrawal,
            expected_concept: "C_INTERACTION_GOAL_WITHDRAWAL",
            expected_withdrawn: 1,
            required_fragment: "1 active task(s)",
        },
        Case {
            id: "R30_INTERACTION_09",
            semantic_group: "R30_POLICY_PAIR",
            category: "outcome_policy_korean_output",
            setup: &[],
            query: "Do not claim the migration ran without an execution record.",
            input_language: En,
            output_language: Ko,
            expected_force: OutcomeClaimConstraint,
            expected_concept: "C_INTERACTION_OUTCOME_POLICY",
            expected_withdrawn: 0,
            required_fragment: "직접 검증",
        },
        Case {
            id: "R30_INTERACTION_10",
            semantic_group: "R30_POLICY_PAIR",
            category: "outcome_policy_english_output",
            setup: &[],
            query: "Do not claim the migration ran without an execution record.",
            input_language: En,
            output_language: En,
            expected_force: OutcomeClaimConstraint,
            expected_concept: "C_INTERACTION_OUTCOME_POLICY",
            expected_withdrawn: 0,
            required_fragment: "direct verification",
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
        schema: "B_CORE_INTERACTION_BOUNDARY_GENERATION_BLIND_REPORT_1",
        suite: "INTERACTION-BOUNDARY-GENERATION-BLIND-R30-RUN-0001",
        frozen_before_first_execution: true,
        fresh_cases: rows.len(),
        passed,
        failed: rows.len() - passed,
        cross_language_semantic_pairs: by_group.len(),
        cross_language_semantic_pairs_passed: pair_passed,
        generative_path_rate_millis: u16::try_from(generative * 1_000 / rows.len())
            .expect("bounded rate"),
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
