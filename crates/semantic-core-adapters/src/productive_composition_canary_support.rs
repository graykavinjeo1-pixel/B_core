//! Shared harness for the frozen R45 productive-composition canaries.
//!
//! This is research-only executable support. Product code is exercised only
//! through the public compositional analyzer API.

use std::collections::BTreeSet;

use dockable_semantic_core::PlanIntentIR;
use semantic_core_adapters::{
    CompositionalSemanticAnalyzer, LanguageCodeIR, PredicateLexemeIR, PREDICATE_LEXEME_SCHEMA,
};
use serde::Serialize;
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum LearnedPredicate {
    KoreanInvestigate,
    EnglishInvestigate,
    KoreanRepair,
    EnglishRepair,
}

impl LearnedPredicate {
    fn build(self) -> PredicateLexemeIR {
        let (id, language, surface, canonical, intent, definition) = match self {
            Self::KoreanInvestigate => (
                "P-R45-KO-MURU",
                LanguageCodeIR::Korean,
                "무루",
                "INVESTIGATE",
                PlanIntentIR::Investigate,
                "inspect an object without changing it",
            ),
            Self::EnglishInvestigate => (
                "P-R45-EN-NEXEL",
                LanguageCodeIR::English,
                "nexel",
                "INVESTIGATE",
                PlanIntentIR::Investigate,
                "inspect an object without changing it",
            ),
            Self::KoreanRepair => (
                "P-R45-KO-GARAM",
                LanguageCodeIR::Korean,
                "가람",
                "REPAIR",
                PlanIntentIR::Repair,
                "repair an object that has a defect",
            ),
            Self::EnglishRepair => (
                "P-R45-EN-VORDA",
                LanguageCodeIR::English,
                "vorda",
                "REPAIR",
                PlanIntentIR::Repair,
                "repair an object that has a defect",
            ),
        };
        PredicateLexemeIR {
            schema: PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: id.to_string(),
            language,
            surface_forms: vec![surface.to_string()],
            canonical_predicate: canonical.to_string(),
            intent_hint: intent,
            definition: definition.to_string(),
            confidence_millis: 920,
        }
    }
}

pub(crate) struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub text: &'static str,
    pub learned: &'static [LearnedPredicate],
    pub expected_frames: &'static [(&'static str, &'static str)],
    pub expected_binding_directions: &'static [&'static str],
    pub expected_binding_relations: &'static [&'static str],
    pub expected_blocked: usize,
    pub expected_authorized_frames: usize,
    pub expected_goal_nodes: usize,
    pub expected_goal_relations: &'static [&'static str],
    pub expect_same_primary_argument: Option<bool>,
    pub expected_quantifier: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Row {
    id: String,
    category: String,
    pass: bool,
    trace: Vec<String>,
}

fn binding_contract(binding: &Value, graph: &Value) -> bool {
    let Some(binding_id) = binding.get("binding_id").and_then(Value::as_str) else {
        return false;
    };
    let Some(provider) = binding
        .get("provider_event_node_id")
        .and_then(Value::as_str)
    else {
        return false;
    };
    let Some(dependent) = binding
        .get("dependent_event_node_id")
        .and_then(Value::as_str)
    else {
        return false;
    };
    let Some(argument) = binding.get("argument_node_id").and_then(Value::as_str) else {
        return false;
    };
    let Some(role) = binding.get("role").and_then(Value::as_str) else {
        return false;
    };
    let confidence = binding
        .get("confidence_millis")
        .and_then(Value::as_u64)
        .unwrap_or(1_001);
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let edges = graph
        .get("role_edges")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let event_exists = |id: &str| {
        nodes
            .iter()
            .any(|node| node["node_id"] == id && node["kind"] == "EVENT")
    };
    let role_exists = |event: &str| {
        edges.iter().any(|edge| {
            edge["event_node_id"] == event
                && edge["argument_node_id"] == argument
                && edge["role"] == role
        })
    };
    !binding_id.trim().is_empty()
        && provider != dependent
        && event_exists(provider)
        && event_exists(dependent)
        && nodes
            .iter()
            .any(|node| node["node_id"] == argument && node["kind"] == "ENTITY")
        && role_exists(provider)
        && role_exists(dependent)
        && binding
            .get("evidence_surface")
            .and_then(Value::as_str)
            .is_some_and(|evidence| !evidence.trim().is_empty())
        && confidence <= 1_000
        && binding["syntactically_licensed"] == true
        && binding["semantic_authority"] == false
        && binding["external_execution_authorized"] == false
}

pub(crate) fn run(case: Case) -> Row {
    let learned = case
        .learned
        .iter()
        .copied()
        .map(LearnedPredicate::build)
        .collect::<Vec<_>>();
    let analysis = CompositionalSemanticAnalyzer.analyze_with_predicates(case.text, &learned);
    let graph = &analysis.semantic_role_graph;
    let graph_value = serde_json::to_value(graph).expect("semantic graph json");
    let bindings_value = graph_value.get("shared_argument_bindings");
    let bindings = bindings_value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    let mut ordered_frames = analysis.frames.iter().collect::<Vec<_>>();
    ordered_frames.sort_by_key(|frame| frame.source_start_byte);
    let observed_frames = ordered_frames
        .iter()
        .map(|frame| {
            let argument = graph.primary_argument_for_frame(&frame.frame_id);
            (
                frame.canonical_predicate.as_str(),
                argument.map(|node| node.normalized_label.as_str()),
                argument.map(|node| node.node_id.as_str()),
            )
        })
        .collect::<Vec<_>>();
    let frames_match = observed_frames.len() == case.expected_frames.len()
        && observed_frames.iter().zip(case.expected_frames.iter()).all(
            |((canonical, argument, _), (expected_canonical, expected_argument))| {
                canonical == expected_canonical && *argument == Some(*expected_argument)
            },
        );

    let argument_ids = observed_frames
        .iter()
        .filter_map(|(_, _, node_id)| *node_id)
        .collect::<Vec<_>>();
    let same_argument_match = case.expect_same_primary_argument.is_none_or(|expected| {
        argument_ids.len() == observed_frames.len()
            && (argument_ids.iter().copied().collect::<BTreeSet<_>>().len() == 1) == expected
    });

    let mut observed_directions = bindings
        .iter()
        .filter_map(|binding| binding.get("direction").and_then(Value::as_str))
        .collect::<Vec<_>>();
    observed_directions.sort_unstable();
    let mut expected_directions = case.expected_binding_directions.to_vec();
    expected_directions.sort_unstable();
    let mut observed_binding_relations = bindings
        .iter()
        .filter_map(|binding| binding.get("relation").and_then(Value::as_str))
        .collect::<Vec<_>>();
    observed_binding_relations.sort_unstable();
    let mut expected_binding_relations = case.expected_binding_relations.to_vec();
    expected_binding_relations.sort_unstable();

    let blocked = analysis
        .candidates
        .iter()
        .filter(|candidate| format!("{:?}", candidate.disposition) == "BlockedByNegation")
        .count();
    let authorized_frames = analysis
        .frames
        .iter()
        .filter(|frame| frame.external_execution_authorized)
        .count();
    let goal_nodes = analysis
        .goal_graph
        .as_ref()
        .map_or(0, |goal| goal.nodes.len());
    let mut goal_relations = analysis
        .goal_graph
        .as_ref()
        .map(|goal| {
            goal.edges
                .iter()
                .map(|edge| format!("{:?}", edge.relation).to_uppercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    goal_relations.sort();
    let mut expected_goal_relations = case
        .expected_goal_relations
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    expected_goal_relations.sort();

    let quantifier_match = case.expected_quantifier.is_none_or(|expected| {
        let Some(argument) = argument_ids.first() else {
            return false;
        };
        graph_value
            .get("quantifier_scopes")
            .and_then(Value::as_array)
            .is_some_and(|scopes| {
                scopes.iter().any(|scope| {
                    scope["target_node_id"] == **argument && scope["quantifier"] == expected
                })
            })
    });
    let binding_ids = bindings
        .iter()
        .filter_map(|binding| binding.get("binding_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let binding_contracts = bindings_value.is_some()
        && bindings.len() == case.expected_binding_directions.len()
        && binding_ids.len() == bindings.len()
        && bindings
            .iter()
            .all(|binding| binding_contract(binding, &graph_value));

    let pass = graph.validate()
        && frames_match
        && same_argument_match
        && binding_contracts
        && observed_directions == expected_directions
        && observed_binding_relations == expected_binding_relations
        && blocked == case.expected_blocked
        && authorized_frames == case.expected_authorized_frames
        && goal_nodes == case.expected_goal_nodes
        && goal_relations == expected_goal_relations
        && quantifier_match
        && analysis
            .frames
            .iter()
            .filter(|frame| {
                frame.embedded_under_quote || format!("{:?}", frame.polarity) == "Negative"
            })
            .all(|frame| !frame.external_execution_authorized);

    let frame_trace = observed_frames
        .iter()
        .map(|(canonical, argument, node)| {
            format!(
                "{canonical}:{}:{}",
                argument.unwrap_or("<NONE>"),
                node.unwrap_or("<NONE>")
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        pass,
        trace: vec![
            format!("frames={frame_trace}"),
            format!(
                "bindings={};directions={};relations={}",
                bindings.len(),
                observed_directions.join(","),
                observed_binding_relations.join(",")
            ),
            format!(
                "blocked={blocked};authorized={authorized_frames};goal_nodes={goal_nodes};coverage={}",
                graph.structural_coverage_millis
            ),
        ],
    }
}

pub(crate) fn emit(label: &str, cases: Vec<Case>) {
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    println!("{}", serde_json::to_string_pretty(&rows).expect("rows"));
    println!("{label}_PASSED={passed}/{}", rows.len());
    if passed != rows.len() {
        std::process::exit(1);
    }
}
