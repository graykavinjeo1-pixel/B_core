//! Shared research-only harness for the frozen R53 diagnostic.

use std::collections::BTreeSet;

use semantic_core_adapters::{
    CompositionalSemanticAnalyzer, QuantifierKindIR, SemanticRoleGraphIR, SemanticRoleKindIR,
    SharedArgumentDirectionIR,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub(crate) struct ArgumentExpectation {
    role: SemanticRoleKindIR,
    label: &'static str,
    quantifier: Option<QuantifierKindIR>,
}

impl ArgumentExpectation {
    pub(crate) const fn new(
        role: SemanticRoleKindIR,
        label: &'static str,
        quantifier: Option<QuantifierKindIR>,
    ) -> Self {
        Self {
            role,
            label,
            quantifier,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FrameExpectation {
    canonical_predicate: &'static str,
    arguments: &'static [ArgumentExpectation],
    external_execution_authorized: bool,
}

impl FrameExpectation {
    pub(crate) const fn new(
        canonical_predicate: &'static str,
        arguments: &'static [ArgumentExpectation],
        external_execution_authorized: bool,
    ) -> Self {
        Self {
            canonical_predicate,
            arguments,
            external_execution_authorized,
        }
    }
}

pub(crate) struct Case {
    id: &'static str,
    category: &'static str,
    text: &'static str,
    frames: &'static [FrameExpectation],
    shared_argument_bindings: usize,
    goal_nodes: usize,
}

impl Case {
    pub(crate) const fn new(
        id: &'static str,
        category: &'static str,
        text: &'static str,
        frames: &'static [FrameExpectation],
        shared_argument_bindings: usize,
        goal_nodes: usize,
    ) -> Self {
        Self {
            id,
            category,
            text,
            frames,
            shared_argument_bindings,
            goal_nodes,
        }
    }
}

#[derive(Serialize)]
struct Row {
    id: String,
    category: String,
    frames: Vec<String>,
    shared_argument_bindings: usize,
    shared_directions: Vec<String>,
    goal_nodes: usize,
    structural_coverage_millis: u16,
    graph_valid: bool,
    authority_violation: bool,
    pass: bool,
}

#[derive(Serialize)]
struct Summary {
    schema: &'static str,
    suite: &'static str,
    total: usize,
    passed: usize,
    failed: usize,
    graph_contracts_valid: usize,
    authority_violations: usize,
    external_llm_calls: usize,
    local_teacher_calls: usize,
    network_calls: usize,
    recursive_source_mutations: usize,
    rows: Vec<Row>,
}

fn normalized_arguments(
    graph: &SemanticRoleGraphIR,
    frame_id: &str,
) -> BTreeSet<(SemanticRoleKindIR, String, Option<QuantifierKindIR>)> {
    graph
        .arguments_for_frame(frame_id)
        .into_iter()
        .filter(|(role, _)| *role != SemanticRoleKindIR::Agent)
        .map(|(role, node)| {
            let quantifier = graph
                .quantifier_scopes
                .iter()
                .find(|scope| scope.target_node_id == node.node_id)
                .map(|scope| scope.quantifier);
            (role, node.normalized_label.clone(), quantifier)
        })
        .collect()
}

fn expected_arguments(
    expected: &FrameExpectation,
) -> BTreeSet<(SemanticRoleKindIR, String, Option<QuantifierKindIR>)> {
    expected
        .arguments
        .iter()
        .map(|argument| {
            (
                argument.role,
                argument.label.to_string(),
                argument.quantifier,
            )
        })
        .collect()
}

fn run(case: &Case) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let graph = &analysis.semantic_role_graph;
    let mut ordered = analysis.frames.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|frame| frame.source_start_byte);
    let frame_contract = ordered.len() == case.frames.len()
        && ordered.iter().zip(case.frames).all(|(observed, expected)| {
            observed.canonical_predicate == expected.canonical_predicate
                && observed.external_execution_authorized == expected.external_execution_authorized
                && normalized_arguments(graph, &observed.frame_id) == expected_arguments(expected)
        });
    let goal_nodes = analysis
        .goal_graph
        .as_ref()
        .map_or(0, |goal| goal.nodes.len());
    let graph_valid = graph.validate() && analysis.clause_graph.validate(case.text);
    let authority_violation = analysis
        .frames
        .iter()
        .any(|frame| frame.embedded_under_quote && frame.external_execution_authorized)
        || graph
            .shared_argument_bindings
            .iter()
            .any(|binding| binding.semantic_authority || binding.external_execution_authorized);
    let binding_contract = graph.shared_argument_bindings.len() == case.shared_argument_bindings
        && graph.shared_argument_bindings.iter().all(|binding| {
            binding.syntactically_licensed
                && !binding.semantic_authority
                && !binding.external_execution_authorized
        });
    let pass = graph_valid
        && frame_contract
        && binding_contract
        && goal_nodes == case.goal_nodes
        && !authority_violation;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        frames: ordered
            .iter()
            .map(|frame| {
                let arguments = normalized_arguments(graph, &frame.frame_id)
                    .into_iter()
                    .map(|(role, label, quantifier)| format!("{role:?}:{label}:{quantifier:?}"))
                    .collect::<Vec<_>>()
                    .join("|");
                format!(
                    "{}[{}]:authorized={}",
                    frame.canonical_predicate, arguments, frame.external_execution_authorized
                )
            })
            .collect(),
        shared_argument_bindings: graph.shared_argument_bindings.len(),
        shared_directions: graph
            .shared_argument_bindings
            .iter()
            .map(|binding| match binding.direction {
                SharedArgumentDirectionIR::Forward => "FORWARD".to_string(),
                SharedArgumentDirectionIR::Backward => "BACKWARD".to_string(),
            })
            .collect(),
        goal_nodes,
        structural_coverage_millis: graph.structural_coverage_millis,
        graph_valid,
        authority_violation,
        pass,
    }
}

pub(crate) fn emit(suite: &'static str, cases: &[Case]) {
    let rows = cases.iter().map(run).collect::<Vec<_>>();
    let total = rows.len();
    let passed = rows.iter().filter(|row| row.pass).count();
    let graph_contracts_valid = rows.iter().filter(|row| row.graph_valid).count();
    let authority_violations = rows.iter().filter(|row| row.authority_violation).count();
    println!(
        "{}",
        serde_json::to_string_pretty(&Summary {
            schema: "B_CORE_R53_PREDICATE_ARGUMENT_COMPOSITION_CANARY_1",
            suite,
            total,
            passed,
            failed: total - passed,
            graph_contracts_valid,
            authority_violations,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            network_calls: 0,
            recursive_source_mutations: 0,
            rows,
        })
        .expect("serialize R53 diagnostic")
    );
    if passed != total {
        std::process::exit(1);
    }
}
