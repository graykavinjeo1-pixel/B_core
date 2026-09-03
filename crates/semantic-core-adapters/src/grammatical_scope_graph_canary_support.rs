//! Shared research-only harness for the frozen R56 grammatical-scope suite.

use std::collections::{BTreeMap, BTreeSet};

use semantic_core_adapters::{CandidateDispositionIR, CompositionalSemanticAnalyzer};
use serde::Serialize;
use serde_json::Value;

pub(crate) type KindCount = (&'static str, usize);

pub(crate) struct Case {
    pub id: &'static str,
    pub category: &'static str,
    pub text: &'static str,
    pub expected_quantifier: Option<&'static str>,
    pub expected_kind_minima: &'static [KindCount],
    pub minimum_ambiguities: usize,
    pub expected_frames: usize,
    pub expected_selected: usize,
    pub expected_blocked: usize,
    pub expected_authorized: usize,
    pub expect_shared_primary_argument: Option<bool>,
}

#[derive(Debug, Serialize)]
struct Row {
    id: String,
    category: String,
    frame_count: usize,
    selected_count: usize,
    blocked_count: usize,
    authorized_count: usize,
    quantifiers: Vec<String>,
    role_nodes: Vec<String>,
    relative_attachments: Vec<String>,
    scope_kind_counts: BTreeMap<String, usize>,
    ambiguity_count: usize,
    graph_contract_valid: bool,
    semantic_authority: bool,
    external_execution_authorized: bool,
    pass: bool,
}

fn graph_contract(graph: &Value) -> bool {
    if graph["schema"] != "B_CORE_GRAMMATICAL_SCOPE_GRAPH_IR_1"
        || graph["semantic_authority"] != false
        || graph["external_execution_authorized"] != false
        || graph["graph_sha256"]
            .as_str()
            .is_none_or(|digest| digest.len() != 64)
    {
        return false;
    }
    let Some(nodes) = graph["nodes"].as_array() else {
        return false;
    };
    let Some(edges) = graph["edges"].as_array() else {
        return false;
    };
    let node_ids = nodes
        .iter()
        .filter_map(|node| node["node_id"].as_str())
        .collect::<BTreeSet<_>>();
    node_ids.len() == nodes.len()
        && nodes.iter().all(|node| {
            node["semantic_authority"] == false
                && node["external_execution_authorized"] == false
                && node["evidence_surface"]
                    .as_str()
                    .is_some_and(|surface| !surface.trim().is_empty())
                && node["confidence_millis"]
                    .as_u64()
                    .is_some_and(|confidence| confidence <= 1_000)
        })
        && edges.iter().all(|edge| {
            edge["source_node_id"]
                .as_str()
                .is_some_and(|id| node_ids.contains(id))
                && edge["target_node_id"]
                    .as_str()
                    .is_some_and(|id| node_ids.contains(id))
                && edge["source_node_id"] != edge["target_node_id"]
        })
}

fn run(case: Case) -> Row {
    let analysis = CompositionalSemanticAnalyzer.analyze(case.text);
    let value = serde_json::to_value(&analysis).expect("analysis json");
    let role = &analysis.semantic_role_graph;
    let quantifiers = role
        .quantifier_scopes
        .iter()
        .map(|scope| format!("{:?}", scope.quantifier).to_uppercase())
        .collect::<Vec<_>>();
    let role_nodes = role
        .nodes
        .iter()
        .map(|node| format!("{}:{:?}:{}", node.node_id, node.kind, node.surface))
        .collect::<Vec<_>>();
    let relative_attachments = role
        .relative_clause_attachments
        .iter()
        .map(|attachment| {
            format!(
                "{}:{}:{}:{}",
                attachment.head_node_id,
                attachment.normalized_predicate,
                attachment.negated,
                attachment.evidence_surface
            )
        })
        .collect::<Vec<_>>();
    let graph = &value["grammatical_scope_graph"];
    let graph_present = graph.is_object();
    let mut scope_kind_counts = BTreeMap::new();
    if let Some(nodes) = graph["nodes"].as_array() {
        for kind in nodes.iter().filter_map(|node| node["kind"].as_str()) {
            *scope_kind_counts.entry(kind.to_string()).or_insert(0) += 1;
        }
    }
    let ambiguity_count = graph["unresolved_ambiguities"]
        .as_array()
        .map_or(0, Vec::len);
    let graph_contract_valid = graph_contract(graph);
    let blocked_count = analysis
        .candidates
        .iter()
        .filter(|candidate| candidate.disposition == CandidateDispositionIR::BlockedByNegation)
        .count();
    let authorized_count = analysis
        .frames
        .iter()
        .filter(|frame| frame.external_execution_authorized)
        .count();
    let primary_argument_ids = analysis
        .frames
        .iter()
        .filter_map(|frame| {
            role.primary_argument_for_frame(&frame.frame_id)
                .map(|node| node.node_id.as_str())
        })
        .collect::<Vec<_>>();
    let shared_match = case.expect_shared_primary_argument.is_none_or(|expected| {
        primary_argument_ids.len() == analysis.frames.len()
            && (primary_argument_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == 1)
                == expected
    });
    let kinds_match = case.expected_kind_minima.iter().all(|(kind, minimum)| {
        scope_kind_counts.get(*kind).copied().unwrap_or_default() >= *minimum
    });
    let quantifier_match = case
        .expected_quantifier
        .is_none_or(|expected| quantifiers.iter().any(|observed| observed == expected));
    let semantic_authority = graph_present && graph["semantic_authority"].as_bool().unwrap_or(true);
    let external_execution_authorized = graph["external_execution_authorized"]
        .as_bool()
        .unwrap_or(false);
    let pass = graph_contract_valid
        && kinds_match
        && quantifier_match
        && ambiguity_count >= case.minimum_ambiguities
        && analysis.frames.len() == case.expected_frames
        && analysis.selected_candidates().len() == case.expected_selected
        && blocked_count == case.expected_blocked
        && authorized_count == case.expected_authorized
        && shared_match
        && !semantic_authority
        && !external_execution_authorized;
    Row {
        id: case.id.to_string(),
        category: case.category.to_string(),
        frame_count: analysis.frames.len(),
        selected_count: analysis.selected_candidates().len(),
        blocked_count,
        authorized_count,
        quantifiers,
        role_nodes,
        relative_attachments,
        scope_kind_counts,
        ambiguity_count,
        graph_contract_valid,
        semantic_authority,
        external_execution_authorized,
        pass,
    }
}

pub(crate) fn emit(suite: &str, cases: Vec<Case>) {
    let rows = cases.into_iter().map(run).collect::<Vec<_>>();
    let passed = rows.iter().filter(|row| row.pass).count();
    let output = serde_json::json!({
        "schema": "B_CORE_R56_GRAMMATICAL_SCOPE_GRAPH_CANARY_1",
        "suite": suite,
        "total": rows.len(),
        "passed": passed,
        "failed": rows.len() - passed,
        "graph_contracts_valid": rows.iter().filter(|row| row.graph_contract_valid).count(),
        "authority_violations": rows.iter().filter(|row| row.semantic_authority || row.external_execution_authorized).count(),
        "external_llm_calls": 0,
        "local_teacher_calls": 0,
        "network_calls": 0,
        "recursive_source_mutations": 0,
        "rows": rows,
    });
    println!("{}", serde_json::to_string(&output).expect("canary json"));
    if passed != output["total"].as_u64().unwrap_or_default() as usize {
        std::process::exit(1);
    }
}
