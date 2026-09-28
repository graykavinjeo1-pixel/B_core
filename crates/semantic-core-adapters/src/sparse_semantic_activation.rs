//! Generic sparse activation over approved semantic graphs.
//!
//! The graph records canonical meaning structure supplied by an authority-owning
//! planner.  Language examples may provide schema coverage but can never assert
//! that a causal edge, threshold, metric, recommendation, or intervention is true.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SPARSE_SEMANTIC_GRAPH_SCHEMA: &str = "B_CORE_SPARSE_SEMANTIC_GRAPH_IR_1";
pub const SPARSE_SEMANTIC_ACTIVATION_SCHEMA: &str = "B_CORE_SPARSE_SEMANTIC_ACTIVATION_IR_1";
const MAX_GRAPH_NODES: usize = 256;
const MAX_GRAPH_EDGES: usize = 512;
const MAX_ACTIVE_NODES: usize = 64;
const MAX_ACTIVE_EDGES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticNodeKindIR {
    Observation,
    Deficit,
    State,
    Transition,
    Mechanism,
    Mediator,
    Moderator,
    Constraint,
    Bottleneck,
    Value,
    Goal,
    Threshold,
    Intervention,
    Outcome,
    Metric,
    Risk,
    Alternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticEdgeKindIR {
    Causes,
    TransitionsTo,
    Mediates,
    Moderates,
    Constrains,
    ConflictsWith,
    ThresholdTriggers,
    Relieves,
    Enables,
    Blocks,
    ProducesSideEffect,
    Measures,
    AlternativeTo,
    Supports,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseSemanticNodeIR {
    pub node_id: String,
    pub kind: SemanticNodeKindIR,
    /// Opaque identity owned by Approved Canonical IR, never surface text.
    pub canonical_ref: String,
    /// Planner supplied priority in the inclusive range 0..=1000.
    pub salience_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseSemanticEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub kind: SemanticEdgeKindIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseSemanticGraphIR {
    pub schema: String,
    pub graph_id: String,
    pub nodes: Vec<SparseSemanticNodeIR>,
    pub edges: Vec<SparseSemanticEdgeIR>,
    pub provenance_refs: Vec<String>,
    /// Example-derived graphs must never elevate these authority flags.
    pub causal_truth_established: bool,
    pub semantic_authority: bool,
    pub external_action_authorized: bool,
    pub graph_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseActivationRequestIR {
    pub focus_node_ids: Vec<String>,
    pub required_kinds: Vec<SemanticNodeKindIR>,
    pub max_hops: u8,
    pub max_active_nodes: usize,
    pub max_active_edges: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SparseSemanticActivationIR {
    pub schema: String,
    pub source_graph_id: String,
    pub source_graph_sha256: String,
    pub active_node_ids: Vec<String>,
    pub active_edge_ids: Vec<String>,
    pub omitted_node_count: usize,
    pub omitted_edge_count: usize,
    pub semantic_authority: bool,
    pub external_action_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SparseSemanticErrorIR {
    InvalidSchema,
    InvalidIdentity,
    DuplicateIdentity,
    MissingEndpoint,
    InvalidStructuralContract,
    ResourceBound,
    AuthorityEscalation,
    HashMismatch,
    UnknownFocus,
    UnsatisfiedRequiredKind,
    InsufficientActivationBudget,
}

impl SparseSemanticGraphIR {
    pub fn seal(mut self) -> Self {
        self.graph_sha256 = graph_sha256(&self);
        self
    }

    pub fn validate(&self) -> Result<(), SparseSemanticErrorIR> {
        if self.schema != SPARSE_SEMANTIC_GRAPH_SCHEMA {
            return Err(SparseSemanticErrorIR::InvalidSchema);
        }
        if !valid_id(&self.graph_id)
            || self.nodes.is_empty()
            || self.edges.is_empty()
            || self.provenance_refs.is_empty()
            || self.provenance_refs.iter().any(|value| !valid_id(value))
        {
            return Err(SparseSemanticErrorIR::InvalidIdentity);
        }
        if self.nodes.len() > MAX_GRAPH_NODES || self.edges.len() > MAX_GRAPH_EDGES {
            return Err(SparseSemanticErrorIR::ResourceBound);
        }
        if self.causal_truth_established
            || self.semantic_authority
            || self.external_action_authorized
        {
            return Err(SparseSemanticErrorIR::AuthorityEscalation);
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();
        let canonical_refs = self
            .nodes
            .iter()
            .map(|node| node.canonical_ref.as_str())
            .collect::<BTreeSet<_>>();
        let edge_ids = self
            .edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<BTreeSet<_>>();
        if node_ids.len() != self.nodes.len()
            || canonical_refs.len() != self.nodes.len()
            || edge_ids.len() != self.edges.len()
            || self.nodes.iter().any(|node| {
                !valid_id(&node.node_id)
                    || !valid_id(&node.canonical_ref)
                    || node.salience_millis > 1000
            })
            || self.edges.iter().any(|edge| !valid_id(&edge.edge_id))
        {
            return Err(SparseSemanticErrorIR::DuplicateIdentity);
        }
        if self.edges.iter().any(|edge| {
            edge.source_node_id == edge.target_node_id
                || !node_ids.contains(edge.source_node_id.as_str())
                || !node_ids.contains(edge.target_node_id.as_str())
        }) {
            return Err(SparseSemanticErrorIR::MissingEndpoint);
        }
        if self
            .edges
            .iter()
            .any(|edge| !self.edge_contract_holds(edge))
        {
            return Err(SparseSemanticErrorIR::InvalidStructuralContract);
        }
        if self.graph_sha256 != graph_sha256(self) {
            return Err(SparseSemanticErrorIR::HashMismatch);
        }
        Ok(())
    }

    fn edge_contract_holds(&self, edge: &SparseSemanticEdgeIR) -> bool {
        let source = self.node_kind(&edge.source_node_id);
        let target = self.node_kind(&edge.target_node_id);
        match edge.kind {
            SemanticEdgeKindIR::ThresholdTriggers => {
                source == Some(SemanticNodeKindIR::Threshold)
                    && matches!(
                        target,
                        Some(SemanticNodeKindIR::Transition | SemanticNodeKindIR::Outcome)
                    )
            }
            SemanticEdgeKindIR::Constrains => matches!(
                source,
                Some(SemanticNodeKindIR::Constraint | SemanticNodeKindIR::Bottleneck)
            ),
            SemanticEdgeKindIR::Mediates => source == Some(SemanticNodeKindIR::Mediator),
            SemanticEdgeKindIR::Moderates => source == Some(SemanticNodeKindIR::Moderator),
            SemanticEdgeKindIR::Measures => {
                source == Some(SemanticNodeKindIR::Metric)
                    || target == Some(SemanticNodeKindIR::Metric)
            }
            SemanticEdgeKindIR::ConflictsWith | SemanticEdgeKindIR::AlternativeTo => {
                matches!(
                    source,
                    Some(
                        SemanticNodeKindIR::Value
                            | SemanticNodeKindIR::Goal
                            | SemanticNodeKindIR::Alternative
                    )
                ) && matches!(
                    target,
                    Some(
                        SemanticNodeKindIR::Value
                            | SemanticNodeKindIR::Goal
                            | SemanticNodeKindIR::Alternative
                    )
                )
            }
            _ => true,
        }
    }

    fn node_kind(&self, node_id: &str) -> Option<SemanticNodeKindIR> {
        self.nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .map(|node| node.kind)
    }

    pub fn activate(
        &self,
        request: &SparseActivationRequestIR,
    ) -> Result<SparseSemanticActivationIR, SparseSemanticErrorIR> {
        self.validate()?;
        if request.focus_node_ids.is_empty()
            || request.max_hops as usize > MAX_GRAPH_NODES
            || request.max_active_nodes == 0
            || request.max_active_nodes > MAX_ACTIVE_NODES
            || request.max_active_edges > MAX_ACTIVE_EDGES
        {
            return Err(SparseSemanticErrorIR::ResourceBound);
        }
        let nodes_by_id = self
            .nodes
            .iter()
            .map(|node| (node.node_id.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        if request
            .focus_node_ids
            .iter()
            .any(|id| !nodes_by_id.contains_key(id.as_str()))
        {
            return Err(SparseSemanticErrorIR::UnknownFocus);
        }
        let required_kinds = request
            .required_kinds
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if required_kinds
            .iter()
            .any(|kind| !self.nodes.iter().any(|node| node.kind == *kind))
        {
            return Err(SparseSemanticErrorIR::UnsatisfiedRequiredKind);
        }

        let adjacency = undirected_adjacency(self);
        let mut distances: BTreeMap<&str, usize> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for focus in &request.focus_node_ids {
            distances.insert(focus, 0);
            queue.push_back(focus.as_str());
        }
        while let Some(current) = queue.pop_front() {
            let depth = distances[current];
            if depth >= request.max_hops as usize {
                continue;
            }
            for next in adjacency.get(current).into_iter().flatten().copied() {
                if !distances.contains_key(next) {
                    distances.insert(next, depth + 1);
                    queue.push_back(next);
                }
            }
        }

        let mut selected = request
            .focus_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for kind in required_kinds {
            let candidate = self
                .nodes
                .iter()
                .filter(|node| node.kind == kind)
                .min_by_key(|node| {
                    (
                        distances
                            .get(node.node_id.as_str())
                            .copied()
                            .unwrap_or(usize::MAX),
                        Reverse(node.salience_millis),
                        node.node_id.as_str(),
                    )
                })
                .expect("required kind existence checked");
            selected.insert(candidate.node_id.as_str());
        }
        if selected.len() > request.max_active_nodes {
            return Err(SparseSemanticErrorIR::InsufficientActivationBudget);
        }

        let mut candidates = self
            .nodes
            .iter()
            .filter_map(|node| {
                distances.get(node.node_id.as_str()).map(|distance| {
                    (
                        *distance,
                        Reverse(node.salience_millis),
                        node.node_id.as_str(),
                    )
                })
            })
            .collect::<Vec<_>>();
        candidates.sort();
        for (_, _, node_id) in candidates {
            if selected.len() >= request.max_active_nodes {
                break;
            }
            selected.insert(node_id);
        }

        let mut active_node_ids = selected.into_iter().map(str::to_string).collect::<Vec<_>>();
        active_node_ids.sort();
        let selected_set = active_node_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut active_edge_ids = self
            .edges
            .iter()
            .filter(|edge| {
                selected_set.contains(edge.source_node_id.as_str())
                    && selected_set.contains(edge.target_node_id.as_str())
            })
            .map(|edge| edge.edge_id.clone())
            .collect::<Vec<_>>();
        active_edge_ids.sort();
        active_edge_ids.truncate(request.max_active_edges);

        Ok(SparseSemanticActivationIR {
            schema: SPARSE_SEMANTIC_ACTIVATION_SCHEMA.to_string(),
            source_graph_id: self.graph_id.clone(),
            source_graph_sha256: self.graph_sha256.clone(),
            omitted_node_count: self.nodes.len() - active_node_ids.len(),
            omitted_edge_count: self.edges.len() - active_edge_ids.len(),
            active_node_ids,
            active_edge_ids,
            semantic_authority: false,
            external_action_authorized: false,
        })
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 160
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
}

fn undirected_adjacency(graph: &SparseSemanticGraphIR) -> BTreeMap<&str, Vec<&str>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &graph.edges {
        adjacency
            .entry(edge.source_node_id.as_str())
            .or_default()
            .push(edge.target_node_id.as_str());
        adjacency
            .entry(edge.target_node_id.as_str())
            .or_default()
            .push(edge.source_node_id.as_str());
    }
    adjacency
}

fn graph_sha256(value: &SparseSemanticGraphIR) -> String {
    let mut canonical = value.clone();
    canonical.graph_sha256.clear();
    canonical
        .nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    canonical
        .edges
        .sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    canonical.provenance_refs.sort();
    let bytes = serde_json::to_vec(&canonical).expect("sparse semantic graph must serialize");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: SemanticNodeKindIR, salience: u16) -> SparseSemanticNodeIR {
        SparseSemanticNodeIR {
            node_id: id.to_string(),
            kind,
            canonical_ref: format!("canonical.{id}"),
            salience_millis: salience,
        }
    }

    fn edge(
        id: &str,
        source: &str,
        target: &str,
        kind: SemanticEdgeKindIR,
    ) -> SparseSemanticEdgeIR {
        SparseSemanticEdgeIR {
            edge_id: id.to_string(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
            kind,
        }
    }

    fn graph() -> SparseSemanticGraphIR {
        SparseSemanticGraphIR {
            schema: SPARSE_SEMANTIC_GRAPH_SCHEMA.to_string(),
            graph_id: "graph.threshold_bottleneck".to_string(),
            nodes: vec![
                node("metric", SemanticNodeKindIR::Metric, 500),
                node("bottleneck", SemanticNodeKindIR::Bottleneck, 850),
                node("goal", SemanticNodeKindIR::Goal, 700),
                node("threshold", SemanticNodeKindIR::Threshold, 1000),
                node("transition", SemanticNodeKindIR::Transition, 950),
                node("outcome", SemanticNodeKindIR::Outcome, 900),
                node("unrelated", SemanticNodeKindIR::Observation, 50),
            ],
            edges: vec![
                edge("e.metric", "metric", "goal", SemanticEdgeKindIR::Measures),
                edge(
                    "e.limit",
                    "bottleneck",
                    "goal",
                    SemanticEdgeKindIR::Constrains,
                ),
                edge(
                    "e.threshold",
                    "threshold",
                    "transition",
                    SemanticEdgeKindIR::ThresholdTriggers,
                ),
                edge(
                    "e.result",
                    "transition",
                    "outcome",
                    SemanticEdgeKindIR::Causes,
                ),
                edge("e.link", "goal", "threshold", SemanticEdgeKindIR::Supports),
                edge(
                    "e.noise",
                    "unrelated",
                    "metric",
                    SemanticEdgeKindIR::Supports,
                ),
            ],
            provenance_refs: vec!["approved.canonical.1".to_string()],
            causal_truth_established: false,
            semantic_authority: false,
            external_action_authorized: false,
            graph_sha256: String::new(),
        }
        .seal()
    }

    #[test]
    fn sparse_activation_keeps_required_local_semantics_and_omits_noise() {
        let graph = graph();
        graph.validate().unwrap();
        let activation = graph
            .activate(&SparseActivationRequestIR {
                focus_node_ids: vec!["threshold".to_string()],
                required_kinds: vec![SemanticNodeKindIR::Outcome],
                max_hops: 2,
                max_active_nodes: 4,
                max_active_edges: 8,
            })
            .unwrap();
        assert!(activation
            .active_node_ids
            .contains(&"threshold".to_string()));
        assert!(activation.active_node_ids.contains(&"outcome".to_string()));
        assert!(activation
            .active_node_ids
            .contains(&"transition".to_string()));
        assert!(!activation
            .active_node_ids
            .contains(&"unrelated".to_string()));
        assert!(activation.omitted_node_count > 0);
        assert!(!activation.semantic_authority);
        assert!(!activation.external_action_authorized);
    }

    #[test]
    fn activation_fails_closed_for_bad_authority_focus_and_budget() {
        let graph = graph();
        let mut elevated = graph.clone();
        elevated.semantic_authority = true;
        elevated = elevated.seal();
        assert_eq!(
            elevated.validate(),
            Err(SparseSemanticErrorIR::AuthorityEscalation)
        );
        assert_eq!(
            graph.activate(&SparseActivationRequestIR {
                focus_node_ids: vec!["missing".to_string()],
                required_kinds: vec![],
                max_hops: 1,
                max_active_nodes: 2,
                max_active_edges: 2,
            }),
            Err(SparseSemanticErrorIR::UnknownFocus)
        );
        assert_eq!(
            graph.activate(&SparseActivationRequestIR {
                focus_node_ids: vec!["threshold".to_string()],
                required_kinds: vec![SemanticNodeKindIR::Outcome],
                max_hops: 1,
                max_active_nodes: 1,
                max_active_edges: 2,
            }),
            Err(SparseSemanticErrorIR::InsufficientActivationBudget)
        );
    }

    #[test]
    fn structural_edge_contract_rejects_untyped_threshold_source() {
        let mut graph = graph();
        graph.edges[2].source_node_id = "goal".to_string();
        graph = graph.seal();
        assert_eq!(
            graph.validate(),
            Err(SparseSemanticErrorIR::InvalidStructuralContract)
        );
    }
}
