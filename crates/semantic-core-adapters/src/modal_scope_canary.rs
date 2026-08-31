use semantic_core_adapters::{
    CognitiveApi, ConditionalKindIR, ConversationInputModalityIR, ConversationTurnRequestIR,
    LanguageCodeIR, ModalIllocutionIR, ModalNegationScopeIR, ModalSemanticAnalyzer, ModalWorldIR,
    PragmaticContextIR, PragmaticReasoner, CONVERSATION_TURN_REQUEST_SCHEMA,
};
use serde::Serialize;

#[derive(Clone, Copy)]
struct ModalCase {
    id: &'static str,
    family: &'static str,
    text: &'static str,
    world: ModalWorldIR,
    illocution: ModalIllocutionIR,
    operator_count: usize,
    conditional: Option<ConditionalKindIR>,
    negation: Option<ModalNegationScopeIR>,
    ambiguity: bool,
    authority: bool,
}

#[derive(Serialize)]
struct CanaryRow {
    id: String,
    family: String,
    input: String,
    pass: bool,
    observed: String,
}

#[derive(Serialize)]
struct CanaryReport {
    schema: &'static str,
    status: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    english_surface: usize,
    korean_surface: usize,
    pragmatic_projection: usize,
    safety_and_cross_turn: usize,
    external_llm_calls: usize,
    network_calls: usize,
    rows: Vec<CanaryRow>,
}

fn cases() -> Vec<ModalCase> {
    use ConditionalKindIR::{Counterfactual, Hypothetical, Indicative, Unless};
    use ModalIllocutionIR::{
        ConditionalDirective, CounterfactualReflection, ModalStatement, PoliteRequest,
    };
    use ModalNegationScopeIR::{Operator, Proposition};
    use ModalWorldIR::{
        Ability, Counterfactual as CounterfactualWorld, Desired, EpistemicCertain,
        EpistemicPossible, EpistemicProbable, Hypothetical as HypotheticalWorld, Intended,
        Normative, Predicted,
    };
    vec![
        ModalCase {
            id: "EN_MIGHT",
            family: "ENGLISH",
            text: "The build might fail.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_PROBABLY",
            family: "ENGLISH",
            text: "The build will probably fail.",
            world: Predicted,
            illocution: ModalStatement,
            operator_count: 2,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_DEFINITELY",
            family: "ENGLISH",
            text: "The build will definitely pass.",
            world: Predicted,
            illocution: ModalStatement,
            operator_count: 2,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_MUST",
            family: "ENGLISH",
            text: "You must delete the cache.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_MUST_NOT",
            family: "ENGLISH",
            text: "You must not delete the cache.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_NOT_REQUIRED",
            family: "ENGLISH",
            text: "You do not have to delete the cache.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Operator),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_DESIRE",
            family: "ENGLISH",
            text: "I want to leave early.",
            world: Desired,
            illocution: ModalIllocutionIR::Wish,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_INTENT",
            family: "ENGLISH",
            text: "I intend to deploy tomorrow.",
            world: Intended,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_ABILITY",
            family: "ENGLISH",
            text: "The worker can retry.",
            world: Ability,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_PREDICTION",
            family: "ENGLISH",
            text: "It will rain tomorrow.",
            world: Predicted,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_CONDITIONAL_DIRECTIVE",
            family: "ENGLISH",
            text: "If the tests pass, deploy the service.",
            world: HypotheticalWorld,
            illocution: ConditionalDirective,
            operator_count: 0,
            conditional: Some(Indicative),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_COUNTERFACTUAL",
            family: "ENGLISH",
            text: "If the backup had existed, the restore would have succeeded.",
            world: CounterfactualWorld,
            illocution: CounterfactualReflection,
            operator_count: 0,
            conditional: Some(Counterfactual),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_POLITE_REQUEST",
            family: "ENGLISH",
            text: "Could you delete the cache?",
            world: Ability,
            illocution: PoliteRequest,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: true,
        },
        ModalCase {
            id: "EN_MAY_AMBIGUOUS",
            family: "ENGLISH",
            text: "The service may restart.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: true,
            authority: false,
        },
        ModalCase {
            id: "EN_MIGHT_NOT",
            family: "ENGLISH",
            text: "The build might not pass.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_NOT_LIKELY",
            family: "ENGLISH",
            text: "The build is not likely to pass.",
            world: EpistemicProbable,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Operator),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_CANNOT",
            family: "ENGLISH",
            text: "The worker cannot retry.",
            world: Ability,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "EN_UNLESS",
            family: "ENGLISH",
            text: "Unless the tests pass, stop the deployment.",
            world: HypotheticalWorld,
            illocution: ConditionalDirective,
            operator_count: 0,
            conditional: Some(Unless),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_MIGHT",
            family: "KOREAN",
            text: "빌드가 실패할 수도 있다.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_PROBABLY",
            family: "KOREAN",
            text: "빌드가 아마도 실패한다.",
            world: EpistemicProbable,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_CERTAIN",
            family: "KOREAN",
            text: "빌드는 확실히 통과한다.",
            world: EpistemicCertain,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_MUST",
            family: "KOREAN",
            text: "캐시를 삭제해야 한다.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_PROHIBITION",
            family: "KOREAN",
            text: "캐시를 삭제하면 안 된다.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_NOT_REQUIRED",
            family: "KOREAN",
            text: "캐시를 삭제할 필요는 없다.",
            world: Normative,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Operator),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_DESIRE",
            family: "KOREAN",
            text: "오늘은 일찍 가고 싶다.",
            world: Desired,
            illocution: ModalIllocutionIR::Wish,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_INTENT",
            family: "KOREAN",
            text: "내일 배포하려고 한다.",
            world: Intended,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_ABILITY",
            family: "KOREAN",
            text: "작업자가 다시 시도할 수 있다.",
            world: Ability,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_PREDICTION",
            family: "KOREAN",
            text: "내일 비가 올 것이다.",
            world: Predicted,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_CONDITIONAL_DIRECTIVE",
            family: "KOREAN",
            text: "테스트가 통과하면 서비스를 배포해.",
            world: HypotheticalWorld,
            illocution: ConditionalDirective,
            operator_count: 0,
            conditional: Some(Hypothetical),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_COUNTERFACTUAL",
            family: "KOREAN",
            text: "백업이 있었더라면 복구가 성공했을 텐데.",
            world: CounterfactualWorld,
            illocution: CounterfactualReflection,
            operator_count: 0,
            conditional: Some(Counterfactual),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_POLITE_REQUEST",
            family: "KOREAN",
            text: "캐시를 삭제해 줄 수 있어?",
            world: Ability,
            illocution: PoliteRequest,
            operator_count: 1,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: true,
        },
        ModalCase {
            id: "KO_NESTED",
            family: "KOREAN",
            text: "캐시를 삭제해야 할 수도 있다.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 2,
            conditional: None,
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_POSSIBLE_NEGATED",
            family: "KOREAN",
            text: "빌드가 통과하지 못할 수도 있다.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_ABILITY_NEGATED",
            family: "KOREAN",
            text: "작업자가 다시 시도할 수 없다.",
            world: Ability,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Proposition),
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_UNLESS",
            family: "KOREAN",
            text: "테스트가 통과하지 않으면 배포를 멈춰.",
            world: HypotheticalWorld,
            illocution: ConditionalDirective,
            operator_count: 0,
            conditional: Some(Unless),
            negation: None,
            ambiguity: false,
            authority: false,
        },
        ModalCase {
            id: "KO_IMPOSSIBLE",
            family: "KOREAN",
            text: "서비스가 재시작할 가능성이 없다.",
            world: EpistemicPossible,
            illocution: ModalStatement,
            operator_count: 1,
            conditional: None,
            negation: Some(Operator),
            ambiguity: false,
            authority: false,
        },
    ]
}

fn modal_rows() -> Vec<CanaryRow> {
    cases()
        .into_iter()
        .map(|case| {
            let graph = ModalSemanticAnalyzer.analyze(case.text);
            let conditional = graph.conditionals.first().map(|item| item.kind);
            let negation = graph.operators.first().and_then(|operator| {
                (operator.negation_scope != ModalNegationScopeIR::None)
                    .then_some(operator.negation_scope)
            });
            let pass = graph.root_world == case.world
                && graph.illocution == case.illocution
                && graph.operators.len() == case.operator_count
                && conditional == case.conditional
                && negation == case.negation
                && graph.unresolved_ambiguities.is_empty() != case.ambiguity
                && graph.external_execution_authorized == case.authority
                && !graph.dialogue_truth_established
                && graph.validate();
            CanaryRow {
                id: case.id.to_string(),
                family: case.family.to_string(),
                input: case.text.to_string(),
                pass,
                observed: format!(
                    "world={:?};illocution={:?};operators={};conditional={:?};negation={:?};ambiguities={};authority={}",
                    graph.root_world,
                    graph.illocution,
                    graph.operators.len(),
                    conditional,
                    negation,
                    graph.unresolved_ambiguities.len(),
                    graph.external_execution_authorized
                ),
            }
        })
        .collect()
}

fn pragmatic_row(
    id: &str,
    text: &str,
    expect_goal: bool,
    expect_authority: bool,
    expect_clarification: bool,
) -> CanaryRow {
    let interpretation = PragmaticReasoner.interpret(text, &PragmaticContextIR::default());
    let goal = interpretation.inferred_goal.as_ref();
    let pass = goal.is_some() == expect_goal
        && goal.is_some_and(|item| item.external_execution_authorized) == expect_authority
        && interpretation.compositional_analysis.clarification_required == expect_clarification
        && !interpretation
            .compositional_analysis
            .modal_scope_graph
            .dialogue_truth_established;
    CanaryRow {
        id: id.to_string(),
        family: "PRAGMATIC_PROJECTION".to_string(),
        input: text.to_string(),
        pass,
        observed: format!(
            "goal={};authority={};clarification={};speech={:?}",
            goal.is_some(),
            goal.is_some_and(|item| item.external_execution_authorized),
            interpretation.compositional_analysis.clarification_required,
            interpretation.speech_act
        ),
    }
}

fn pragmatic_rows() -> Vec<CanaryRow> {
    vec![
        pragmatic_row(
            "PRAG_EN_NESTED_NONGOAL",
            "We might need to delete the cache.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_KO_NESTED_NONGOAL",
            "캐시를 삭제해야 할 수도 있다.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_OBLIGATION",
            "You must delete the cache.",
            true,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_KO_OBLIGATION",
            "캐시를 삭제해야 한다.",
            true,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_CONDITIONAL",
            "If the tests pass, deploy the service.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_KO_CONDITIONAL",
            "테스트가 통과하면 서비스를 배포해.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_POLITE",
            "Could you delete the cache?",
            true,
            true,
            false,
        ),
        pragmatic_row(
            "PRAG_KO_POLITE",
            "캐시를 삭제해 줄 수 있어?",
            true,
            true,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_DESIRE",
            "I want to delete the cache.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_NOT_REQUIRED",
            "You do not have to delete the cache.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_PROHIBITION",
            "You must not delete the cache.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_KO_PROHIBITION",
            "캐시를 삭제하면 안 된다.",
            false,
            false,
            false,
        ),
        pragmatic_row(
            "PRAG_EN_SHOULD_AMBIGUOUS",
            "You should delete the cache.",
            false,
            false,
            true,
        ),
    ]
}

fn request(conversation_id: &str, turn_index: u64, text: &str) -> ConversationTurnRequestIR {
    ConversationTurnRequestIR {
        schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        turn_index,
        request_id: format!("{conversation_id}-{turn_index}"),
        modality: ConversationInputModalityIR::Text,
        raw_text: text.to_string(),
        input_confidence_millis: 1_000,
        alternatives: Vec::new(),
        output_language: Some(LanguageCodeIR::English),
        context_tags: Vec::new(),
        max_plan_steps: 12,
    }
}

fn safety_rows() -> Vec<CanaryRow> {
    let mut rows = Vec::new();

    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    let possible = api
        .process_conversation_turn(&request("WORLD-SEPARATION", 1, "The server might be down."))
        .expect("possible state");
    let actual = api
        .process_conversation_turn(&request("WORLD-SEPARATION", 2, "The server is up."))
        .expect("actual state");
    rows.push(CanaryRow {
        id: "SAFETY_ACTUAL_POSSIBLE_SEPARATION".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "possible down -> actual up".to_string(),
        pass: possible.conversation_state.epistemic_ledger.records.len() == 1
            && actual.conversation_state.epistemic_ledger.records.len() == 2
            && actual
                .conversation_state
                .epistemic_ledger
                .unresolved_conflicts
                .is_empty(),
        observed: format!(
            "records={};conflicts={}",
            actual.conversation_state.epistemic_ledger.records.len(),
            actual
                .conversation_state
                .epistemic_ledger
                .unresolved_conflicts
                .len()
        ),
    });

    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    api.process_conversation_turn(&request(
        "POSSIBLE-CONFLICT",
        1,
        "Alice says the server might be up.",
    ))
    .expect("Alice possibility");
    let contested = api
        .process_conversation_turn(&request(
            "POSSIBLE-CONFLICT",
            2,
            "Bob says the server might be down.",
        ))
        .expect("Bob possibility");
    rows.push(CanaryRow {
        id: "SAFETY_SAME_WORLD_CROSS_SOURCE_CONTEST".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "Alice possible up -> Bob possible down".to_string(),
        pass: contested
            .conversation_state
            .epistemic_ledger
            .unresolved_conflicts
            .len()
            == 1
            && contested
                .conversation_state
                .epistemic_ledger
                .records
                .iter()
                .all(|record| record.signature.modal_world == ModalWorldIR::EpistemicPossible),
        observed: format!(
            "records={};conflicts={}",
            contested.conversation_state.epistemic_ledger.records.len(),
            contested
                .conversation_state
                .epistemic_ledger
                .unresolved_conflicts
                .len()
        ),
    });

    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    let guarded = api
        .process_conversation_turn(&request(
            "GUARDED-ACTION",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guarded action");
    rows.push(CanaryRow {
        id: "SAFETY_UNSATISFIED_GUARD_NO_ACTION".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "If the tests pass, deploy the service.".to_string(),
        pass: guarded.conversation_state.active_goals.is_empty()
            && guarded.pragmatic_interpretation.inferred_goal.is_none()
            && !guarded
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .conditionals[0]
                .condition_satisfied,
        observed: format!(
            "goals={};beliefs={}",
            guarded.conversation_state.active_goals.len(),
            guarded.conversation_state.epistemic_ledger.records.len()
        ),
    });

    let mut api = CognitiveApi::new_embedded().expect("embedded API");
    let polite = api
        .process_conversation_turn(&request("POLITE-ACTION", 1, "Could you delete the cache?"))
        .expect("polite action");
    rows.push(CanaryRow {
        id: "SAFETY_POLITE_REQUEST_IS_GOAL_NOT_BELIEF".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "Could you delete the cache?".to_string(),
        pass: polite.conversation_state.active_goals.len() == 1
            && polite
                .conversation_state
                .epistemic_ledger
                .records
                .is_empty()
            && polite.conversation_state.active_goals[0].external_execution_authorized,
        observed: format!(
            "goals={};beliefs={};authority={}",
            polite.conversation_state.active_goals.len(),
            polite.conversation_state.epistemic_ledger.records.len(),
            polite
                .conversation_state
                .active_goals
                .first()
                .is_some_and(|goal| goal.external_execution_authorized)
        ),
    });

    let mut modal_tamper = ModalSemanticAnalyzer.analyze("The build might fail.");
    modal_tamper.dialogue_truth_established = true;
    rows.push(CanaryRow {
        id: "SAFETY_MODAL_TRUTH_TAMPER_REJECTED".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "set dialogue_truth_established=true".to_string(),
        pass: !modal_tamper.validate(),
        observed: format!("valid={}", modal_tamper.validate()),
    });

    let mut conditional_tamper =
        ModalSemanticAnalyzer.analyze("If the tests pass, deploy the service.");
    conditional_tamper.conditionals[0].reverse_inference_authorized = true;
    rows.push(CanaryRow {
        id: "SAFETY_REVERSE_INFERENCE_TAMPER_REJECTED".to_string(),
        family: "SAFETY_AND_CROSS_TURN".to_string(),
        input: "set reverse_inference_authorized=true".to_string(),
        pass: !conditional_tamper.validate(),
        observed: format!("valid={}", conditional_tamper.validate()),
    });
    rows
}

fn main() {
    let mut rows = modal_rows();
    rows.extend(pragmatic_rows());
    rows.extend(safety_rows());
    let passed = rows.iter().filter(|row| row.pass).count();
    let report = CanaryReport {
        schema: "B_CORE_MODAL_SCOPE_CANARY_V1",
        status: if passed == rows.len() { "PASS" } else { "FAIL" },
        total: rows.len(),
        passed,
        failed: rows.len() - passed,
        english_surface: rows.iter().filter(|row| row.family == "ENGLISH").count(),
        korean_surface: rows.iter().filter(|row| row.family == "KOREAN").count(),
        pragmatic_projection: rows
            .iter()
            .filter(|row| row.family == "PRAGMATIC_PROJECTION")
            .count(),
        safety_and_cross_turn: rows
            .iter()
            .filter(|row| row.family == "SAFETY_AND_CROSS_TURN")
            .count(),
        external_llm_calls: 0,
        network_calls: 0,
        rows,
    };
    println!(
        "{}",
        serde_json::to_string(&report).expect("serialize canary report")
    );
    if report.failed != 0 {
        std::process::exit(1);
    }
}
