//! Typed, non-authoritative causal topology carried between dialogue and core deliberation.
//!
//! This module represents how an approved reasoning problem is structured.  It
//! does not infer a causal law from text, promote supplied examples to factual
//! knowledge, or authorize an intervention.  Cyclic feedback is represented
//! explicitly; callers must not mislabel every topology as a DAG.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CAUSAL_DIALOGUE_TOPOLOGY_SCHEMA: &str = "B_CORE_CAUSAL_DIALOGUE_TOPOLOGY_IR_1";
const MAX_NODES: usize = 64;
const MAX_EDGES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalTopologyKindIR {
    CounterfactualAlternative,
    CommonCauseConfounder,
    ReinforcingFeedbackLoop,
    CompetingHypothesesDiagnostic,
    ConstraintTradeoff,
    CausalCascade,
    ThresholdAccumulation,
    MetricGoalDistortion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalTopologyNodeKindIR {
    ObservedState,
    ObservedOutcome,
    CounterfactualState,
    CounterfactualOutcome,
    LatentFactor,
    Hypothesis,
    DiscriminatingEvidence,
    Goal,
    Constraint,
    Intervention,
    IntermediateState,
    Threshold,
    Metric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalTopologyEdgeKindIR {
    Causes,
    Propagates,
    AlternativeOf,
    CommonCause,
    RejectedDirectCause,
    Amplifies,
    BreaksLoop,
    Explains,
    Supports,
    Refutes,
    Constrains,
    ThresholdTriggers,
    OptimizesProxy,
    Undermines,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalTopologyNodeIR {
    pub node_id: String,
    pub kind: CausalTopologyNodeKindIR,
    /// Opaque canonical proposition identity owned by the approved meaning/core.
    pub proposition_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalTopologyEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub kind: CausalTopologyEdgeKindIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalDialogueTopologyIR {
    pub schema: String,
    pub topology_id: String,
    pub kind: CausalTopologyKindIR,
    pub nodes: Vec<CausalTopologyNodeIR>,
    pub edges: Vec<CausalTopologyEdgeIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intervention_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_hypothesis_node_id: Option<String>,
    #[serde(default)]
    pub immutable_goal_node_ids: Vec<String>,
    #[serde(default)]
    pub relaxed_goal_node_ids: Vec<String>,
    pub provenance_refs: Vec<String>,
    /// These fields must remain false for language/example-derived topologies.
    pub causal_truth_established: bool,
    pub semantic_authority: bool,
    pub external_action_authorized: bool,
    pub topology_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CausalTopologyValidationErrorIR {
    InvalidSchema,
    InvalidIdentity,
    ResourceBound,
    DuplicateIdentity,
    MissingEndpoint,
    AuthorityEscalation,
    InvalidTopologyContract,
    HashMismatch,
}

impl CausalDialogueTopologyIR {
    pub fn seal(mut self) -> Self {
        self.topology_sha256 = topology_sha256(&self);
        self
    }

    pub fn validate(&self) -> Result<(), CausalTopologyValidationErrorIR> {
        if self.schema != CAUSAL_DIALOGUE_TOPOLOGY_SCHEMA {
            return Err(CausalTopologyValidationErrorIR::InvalidSchema);
        }
        if !valid_id(&self.topology_id)
            || self.provenance_refs.is_empty()
            || self.provenance_refs.iter().any(|value| !valid_id(value))
            || self.nodes.is_empty()
            || self.edges.is_empty()
        {
            return Err(CausalTopologyValidationErrorIR::InvalidIdentity);
        }
        if self.nodes.len() > MAX_NODES || self.edges.len() > MAX_EDGES {
            return Err(CausalTopologyValidationErrorIR::ResourceBound);
        }
        if self.causal_truth_established
            || self.semantic_authority
            || self.external_action_authorized
        {
            return Err(CausalTopologyValidationErrorIR::AuthorityEscalation);
        }

        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();
        let proposition_ids = self
            .nodes
            .iter()
            .map(|node| node.proposition_id.as_str())
            .collect::<BTreeSet<_>>();
        let edge_ids = self
            .edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<BTreeSet<_>>();
        if node_ids.len() != self.nodes.len()
            || proposition_ids.len() != self.nodes.len()
            || edge_ids.len() != self.edges.len()
            || self
                .nodes
                .iter()
                .any(|node| !valid_id(&node.node_id) || !valid_id(&node.proposition_id))
            || self.edges.iter().any(|edge| !valid_id(&edge.edge_id))
        {
            return Err(CausalTopologyValidationErrorIR::DuplicateIdentity);
        }
        if self.edges.iter().any(|edge| {
            edge.source_node_id == edge.target_node_id
                || !node_ids.contains(edge.source_node_id.as_str())
                || !node_ids.contains(edge.target_node_id.as_str())
        }) {
            return Err(CausalTopologyValidationErrorIR::MissingEndpoint);
        }
        if self
            .intervention_node_id
            .as_deref()
            .is_some_and(|id| !node_ids.contains(id))
            || self
                .selected_hypothesis_node_id
                .as_deref()
                .is_some_and(|id| !node_ids.contains(id))
            || self
                .immutable_goal_node_ids
                .iter()
                .chain(&self.relaxed_goal_node_ids)
                .any(|id| !node_ids.contains(id.as_str()))
        {
            return Err(CausalTopologyValidationErrorIR::MissingEndpoint);
        }
        if !self.validate_kind_contract() {
            return Err(CausalTopologyValidationErrorIR::InvalidTopologyContract);
        }
        if self.topology_sha256 != topology_sha256(self) {
            return Err(CausalTopologyValidationErrorIR::HashMismatch);
        }
        Ok(())
    }

    fn validate_kind_contract(&self) -> bool {
        match self.kind {
            CausalTopologyKindIR::CounterfactualAlternative => {
                count_nodes(self, CausalTopologyNodeKindIR::ObservedOutcome) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::CounterfactualState) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::CounterfactualOutcome) >= 1
                    && has_edge(self, CausalTopologyEdgeKindIR::AlternativeOf)
                    && self.edges.iter().any(|edge| {
                        edge.kind == CausalTopologyEdgeKindIR::Causes
                            && node_kind(self, &edge.source_node_id)
                                == Some(CausalTopologyNodeKindIR::CounterfactualState)
                            && node_kind(self, &edge.target_node_id)
                                == Some(CausalTopologyNodeKindIR::CounterfactualOutcome)
                    })
            }
            CausalTopologyKindIR::CommonCauseConfounder => {
                let latent_ids = self
                    .nodes
                    .iter()
                    .filter(|node| node.kind == CausalTopologyNodeKindIR::LatentFactor)
                    .map(|node| node.node_id.as_str())
                    .collect::<Vec<_>>();
                latent_ids.iter().any(|latent| {
                    self.edges
                        .iter()
                        .filter(|edge| {
                            edge.kind == CausalTopologyEdgeKindIR::CommonCause
                                && edge.source_node_id == *latent
                        })
                        .map(|edge| edge.target_node_id.as_str())
                        .collect::<BTreeSet<_>>()
                        .len()
                        >= 2
                }) && has_edge(self, CausalTopologyEdgeKindIR::RejectedDirectCause)
            }
            CausalTopologyKindIR::ReinforcingFeedbackLoop => {
                has_directed_cycle(self, CausalTopologyEdgeKindIR::Amplifies)
                    && self.intervention_node_id.as_deref().is_some_and(|id| {
                        node_kind(self, id) == Some(CausalTopologyNodeKindIR::Intervention)
                            && self.edges.iter().any(|edge| {
                                edge.kind == CausalTopologyEdgeKindIR::BreaksLoop
                                    && edge.source_node_id == id
                            })
                    })
            }
            CausalTopologyKindIR::CompetingHypothesesDiagnostic => {
                count_nodes(self, CausalTopologyNodeKindIR::Hypothesis) >= 2
                    && count_nodes(self, CausalTopologyNodeKindIR::DiscriminatingEvidence) >= 1
                    && has_edge(self, CausalTopologyEdgeKindIR::Explains)
                    && has_edge(self, CausalTopologyEdgeKindIR::Refutes)
                    && self
                        .selected_hypothesis_node_id
                        .as_deref()
                        .is_some_and(|id| {
                            node_kind(self, id) == Some(CausalTopologyNodeKindIR::Hypothesis)
                        })
            }
            CausalTopologyKindIR::ConstraintTradeoff => {
                let goals = self
                    .nodes
                    .iter()
                    .filter(|node| node.kind == CausalTopologyNodeKindIR::Goal)
                    .map(|node| node.node_id.as_str())
                    .collect::<BTreeSet<_>>();
                goals.len() >= 2
                    && count_nodes(self, CausalTopologyNodeKindIR::Constraint) >= 1
                    && has_edge(self, CausalTopologyEdgeKindIR::Constrains)
                    && !self.immutable_goal_node_ids.is_empty()
                    && !self.relaxed_goal_node_ids.is_empty()
                    && self
                        .immutable_goal_node_ids
                        .iter()
                        .all(|id| goals.contains(id.as_str()))
                    && self
                        .relaxed_goal_node_ids
                        .iter()
                        .all(|id| goals.contains(id.as_str()))
                    && self
                        .immutable_goal_node_ids
                        .iter()
                        .all(|id| !self.relaxed_goal_node_ids.contains(id))
            }
            CausalTopologyKindIR::CausalCascade => longest_causal_path(self) >= 3,
            CausalTopologyKindIR::ThresholdAccumulation => {
                count_nodes(self, CausalTopologyNodeKindIR::IntermediateState) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::Threshold) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::ObservedOutcome) >= 1
                    && self.edges.iter().any(|edge| {
                        edge.kind == CausalTopologyEdgeKindIR::ThresholdTriggers
                            && node_kind(self, &edge.source_node_id)
                                == Some(CausalTopologyNodeKindIR::Threshold)
                            && node_kind(self, &edge.target_node_id)
                                == Some(CausalTopologyNodeKindIR::ObservedOutcome)
                    })
            }
            CausalTopologyKindIR::MetricGoalDistortion => {
                count_nodes(self, CausalTopologyNodeKindIR::Metric) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::Goal) >= 1
                    && count_nodes(self, CausalTopologyNodeKindIR::ObservedOutcome) >= 1
                    && has_edge(self, CausalTopologyEdgeKindIR::OptimizesProxy)
                    && has_edge(self, CausalTopologyEdgeKindIR::Undermines)
            }
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 160
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
}

fn count_nodes(value: &CausalDialogueTopologyIR, kind: CausalTopologyNodeKindIR) -> usize {
    value.nodes.iter().filter(|node| node.kind == kind).count()
}

fn node_kind(value: &CausalDialogueTopologyIR, node_id: &str) -> Option<CausalTopologyNodeKindIR> {
    value
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .map(|node| node.kind)
}

fn has_edge(value: &CausalDialogueTopologyIR, kind: CausalTopologyEdgeKindIR) -> bool {
    value.edges.iter().any(|edge| edge.kind == kind)
}

fn adjacency_for(
    value: &CausalDialogueTopologyIR,
    allowed: impl Fn(CausalTopologyEdgeKindIR) -> bool,
) -> BTreeMap<&str, Vec<&str>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &value.edges {
        if allowed(edge.kind) {
            adjacency
                .entry(edge.source_node_id.as_str())
                .or_default()
                .push(edge.target_node_id.as_str());
        }
    }
    adjacency
}

fn has_directed_cycle(value: &CausalDialogueTopologyIR, kind: CausalTopologyEdgeKindIR) -> bool {
    let adjacency = adjacency_for(value, |candidate| candidate == kind);
    for start in adjacency.keys().copied() {
        let mut queue = VecDeque::from([(start, 0usize)]);
        let mut seen = BTreeSet::new();
        while let Some((current, depth)) = queue.pop_front() {
            if depth > 0 && current == start {
                return true;
            }
            if depth > value.nodes.len() || !seen.insert((current, depth)) {
                continue;
            }
            for next in adjacency.get(current).into_iter().flatten() {
                queue.push_back((next, depth + 1));
            }
        }
    }
    false
}

fn longest_causal_path(value: &CausalDialogueTopologyIR) -> usize {
    let adjacency = adjacency_for(value, |kind| {
        matches!(
            kind,
            CausalTopologyEdgeKindIR::Causes | CausalTopologyEdgeKindIR::Propagates
        )
    });
    let mut longest = 0;
    for start in value.nodes.iter().map(|node| node.node_id.as_str()) {
        let mut queue = VecDeque::from([(start, 0usize, BTreeSet::from([start]))]);
        while let Some((current, depth, seen)) = queue.pop_front() {
            longest = longest.max(depth);
            for next in adjacency.get(current).into_iter().flatten().copied() {
                if !seen.contains(next) {
                    let mut next_seen = seen.clone();
                    next_seen.insert(next);
                    queue.push_back((next, depth + 1, next_seen));
                }
            }
        }
    }
    longest
}

fn topology_sha256(value: &CausalDialogueTopologyIR) -> String {
    let mut canonical = value.clone();
    canonical.topology_sha256.clear();
    canonical
        .nodes
        .sort_by(|left, right| left.node_id.cmp(&right.node_id));
    canonical
        .edges
        .sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    canonical.immutable_goal_node_ids.sort();
    canonical.relaxed_goal_node_ids.sort();
    canonical.provenance_refs.sort();
    let bytes = serde_json::to_vec(&canonical).expect("causal topology must serialize");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: CausalTopologyNodeKindIR) -> CausalTopologyNodeIR {
        CausalTopologyNodeIR {
            node_id: id.to_string(),
            kind,
            proposition_id: format!("prop.{id}"),
        }
    }

    fn edge(
        id: &str,
        source: &str,
        target: &str,
        kind: CausalTopologyEdgeKindIR,
    ) -> CausalTopologyEdgeIR {
        CausalTopologyEdgeIR {
            edge_id: id.to_string(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
            kind,
        }
    }

    fn base(
        kind: CausalTopologyKindIR,
        nodes: Vec<CausalTopologyNodeIR>,
        edges: Vec<CausalTopologyEdgeIR>,
    ) -> CausalDialogueTopologyIR {
        CausalDialogueTopologyIR {
            schema: CAUSAL_DIALOGUE_TOPOLOGY_SCHEMA.to_string(),
            topology_id: format!("topology.{kind:?}"),
            kind,
            nodes,
            edges,
            intervention_node_id: None,
            selected_hypothesis_node_id: None,
            immutable_goal_node_ids: Vec::new(),
            relaxed_goal_node_ids: Vec::new(),
            provenance_refs: vec!["approved.canonical.fixture".to_string()],
            causal_truth_established: false,
            semantic_authority: false,
            external_action_authorized: false,
            topology_sha256: String::new(),
        }
    }

    #[test]
    fn validates_all_eight_topology_contracts() {
        let counterfactual = base(
            CausalTopologyKindIR::CounterfactualAlternative,
            vec![
                node("actual", CausalTopologyNodeKindIR::ObservedOutcome),
                node("alternative", CausalTopologyNodeKindIR::CounterfactualState),
                node("simulated", CausalTopologyNodeKindIR::CounterfactualOutcome),
            ],
            vec![
                edge(
                    "alt",
                    "alternative",
                    "actual",
                    CausalTopologyEdgeKindIR::AlternativeOf,
                ),
                edge(
                    "sim",
                    "alternative",
                    "simulated",
                    CausalTopologyEdgeKindIR::Causes,
                ),
            ],
        )
        .seal();
        assert_eq!(counterfactual.validate(), Ok(()));

        let confounder = base(
            CausalTopologyKindIR::CommonCauseConfounder,
            vec![
                node("z", CausalTopologyNodeKindIR::LatentFactor),
                node("x", CausalTopologyNodeKindIR::ObservedState),
                node("y", CausalTopologyNodeKindIR::ObservedOutcome),
            ],
            vec![
                edge("zx", "z", "x", CausalTopologyEdgeKindIR::CommonCause),
                edge("zy", "z", "y", CausalTopologyEdgeKindIR::CommonCause),
                edge(
                    "xy_rejected",
                    "x",
                    "y",
                    CausalTopologyEdgeKindIR::RejectedDirectCause,
                ),
            ],
        )
        .seal();
        assert_eq!(confounder.validate(), Ok(()));

        let mut feedback = base(
            CausalTopologyKindIR::ReinforcingFeedbackLoop,
            vec![
                node("a", CausalTopologyNodeKindIR::ObservedState),
                node("b", CausalTopologyNodeKindIR::IntermediateState),
                node("c", CausalTopologyNodeKindIR::IntermediateState),
                node("break", CausalTopologyNodeKindIR::Intervention),
            ],
            vec![
                edge("ab", "a", "b", CausalTopologyEdgeKindIR::Amplifies),
                edge("bc", "b", "c", CausalTopologyEdgeKindIR::Amplifies),
                edge("ca", "c", "a", CausalTopologyEdgeKindIR::Amplifies),
                edge(
                    "break_a",
                    "break",
                    "a",
                    CausalTopologyEdgeKindIR::BreaksLoop,
                ),
            ],
        );
        feedback.intervention_node_id = Some("break".to_string());
        let feedback = feedback.seal();
        assert_eq!(feedback.validate(), Ok(()));

        let mut hypotheses = base(
            CausalTopologyKindIR::CompetingHypothesesDiagnostic,
            vec![
                node("h1", CausalTopologyNodeKindIR::Hypothesis),
                node("h2", CausalTopologyNodeKindIR::Hypothesis),
                node("o", CausalTopologyNodeKindIR::ObservedOutcome),
                node("e", CausalTopologyNodeKindIR::DiscriminatingEvidence),
            ],
            vec![
                edge("h1o", "h1", "o", CausalTopologyEdgeKindIR::Explains),
                edge("h2o", "h2", "o", CausalTopologyEdgeKindIR::Explains),
                edge("eh1", "e", "h1", CausalTopologyEdgeKindIR::Refutes),
                edge("eh2", "e", "h2", CausalTopologyEdgeKindIR::Supports),
            ],
        );
        hypotheses.selected_hypothesis_node_id = Some("h2".to_string());
        let hypotheses = hypotheses.seal();
        assert_eq!(hypotheses.validate(), Ok(()));

        let mut tradeoff = base(
            CausalTopologyKindIR::ConstraintTradeoff,
            vec![
                node("goal_quality", CausalTopologyNodeKindIR::Goal),
                node("goal_cost", CausalTopologyNodeKindIR::Goal),
                node("goal_time", CausalTopologyNodeKindIR::Goal),
                node("constraint", CausalTopologyNodeKindIR::Constraint),
            ],
            vec![
                edge(
                    "cq",
                    "constraint",
                    "goal_quality",
                    CausalTopologyEdgeKindIR::Constrains,
                ),
                edge(
                    "cc",
                    "constraint",
                    "goal_cost",
                    CausalTopologyEdgeKindIR::Constrains,
                ),
                edge(
                    "ct",
                    "constraint",
                    "goal_time",
                    CausalTopologyEdgeKindIR::Constrains,
                ),
            ],
        );
        tradeoff.immutable_goal_node_ids = vec!["goal_time".to_string()];
        tradeoff.relaxed_goal_node_ids = vec!["goal_cost".to_string()];
        let tradeoff = tradeoff.seal();
        assert_eq!(tradeoff.validate(), Ok(()));

        let cascade = base(
            CausalTopologyKindIR::CausalCascade,
            vec![
                node("s0", CausalTopologyNodeKindIR::ObservedState),
                node("s1", CausalTopologyNodeKindIR::IntermediateState),
                node("s2", CausalTopologyNodeKindIR::IntermediateState),
                node("s3", CausalTopologyNodeKindIR::ObservedOutcome),
            ],
            vec![
                edge("s0s1", "s0", "s1", CausalTopologyEdgeKindIR::Propagates),
                edge("s1s2", "s1", "s2", CausalTopologyEdgeKindIR::Propagates),
                edge("s2s3", "s2", "s3", CausalTopologyEdgeKindIR::Causes),
            ],
        )
        .seal();
        assert_eq!(cascade.validate(), Ok(()));

        let threshold = base(
            CausalTopologyKindIR::ThresholdAccumulation,
            vec![
                node("load", CausalTopologyNodeKindIR::IntermediateState),
                node("limit", CausalTopologyNodeKindIR::Threshold),
                node("failure", CausalTopologyNodeKindIR::ObservedOutcome),
            ],
            vec![
                edge(
                    "accumulates",
                    "load",
                    "limit",
                    CausalTopologyEdgeKindIR::Causes,
                ),
                edge(
                    "tips",
                    "limit",
                    "failure",
                    CausalTopologyEdgeKindIR::ThresholdTriggers,
                ),
            ],
        )
        .seal();
        assert_eq!(threshold.validate(), Ok(()));

        let metric_distortion = base(
            CausalTopologyKindIR::MetricGoalDistortion,
            vec![
                node("proxy", CausalTopologyNodeKindIR::Metric),
                node("goal", CausalTopologyNodeKindIR::Goal),
                node("harm", CausalTopologyNodeKindIR::ObservedOutcome),
            ],
            vec![
                edge(
                    "proxy_goal",
                    "proxy",
                    "goal",
                    CausalTopologyEdgeKindIR::OptimizesProxy,
                ),
                edge(
                    "goal_harm",
                    "goal",
                    "harm",
                    CausalTopologyEdgeKindIR::Undermines,
                ),
            ],
        )
        .seal();
        assert_eq!(metric_distortion.validate(), Ok(()));
    }

    #[test]
    fn rejects_missing_contract_parts_tampering_and_authority_escalation() {
        let value = base(
            CausalTopologyKindIR::CommonCauseConfounder,
            vec![
                node("z", CausalTopologyNodeKindIR::LatentFactor),
                node("x", CausalTopologyNodeKindIR::ObservedState),
                node("y", CausalTopologyNodeKindIR::ObservedOutcome),
            ],
            vec![
                edge("zx", "z", "x", CausalTopologyEdgeKindIR::CommonCause),
                edge("zy", "z", "y", CausalTopologyEdgeKindIR::CommonCause),
                edge(
                    "xy",
                    "x",
                    "y",
                    CausalTopologyEdgeKindIR::RejectedDirectCause,
                ),
            ],
        )
        .seal();
        assert_eq!(value.validate(), Ok(()));

        let mut missing_rejection = value.clone();
        missing_rejection
            .edges
            .retain(|edge| edge.kind != CausalTopologyEdgeKindIR::RejectedDirectCause);
        missing_rejection = missing_rejection.seal();
        assert_eq!(
            missing_rejection.validate(),
            Err(CausalTopologyValidationErrorIR::InvalidTopologyContract)
        );

        let mut escalated = value.clone();
        escalated.causal_truth_established = true;
        escalated = escalated.seal();
        assert_eq!(
            escalated.validate(),
            Err(CausalTopologyValidationErrorIR::AuthorityEscalation)
        );

        let mut tampered = value;
        tampered.nodes[0].proposition_id = "prop.tampered".to_string();
        assert_eq!(
            tampered.validate(),
            Err(CausalTopologyValidationErrorIR::HashMismatch)
        );
    }
}
