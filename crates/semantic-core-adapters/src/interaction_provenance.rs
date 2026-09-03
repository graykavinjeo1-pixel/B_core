//! Cross-layer provenance for one conversational response.
//!
//! The graph joins language input, semantic goals, plans, language-only
//! reports, trusted host observations, terminal verification, results, and
//! realized claims without granting any of those language artifacts semantic
//! or execution authority.  It is derived from typed state; prose is never
//! parsed back into evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use dockable_semantic_core::PlanIR;

use crate::action_state::{
    ActionEvidenceAuditIR, ActionEvidenceStatusIR, ActionExecutionStatusIR,
    ActionLanguageReportRecordIR, ActionStateLedgerIR, ActionStateRecordIR,
};
use crate::grounded_realization::{
    ClaimSupportStatusIR, EvidenceGroundedRealizationIR, GroundedClaimIR, GroundedClaimKindIR,
};

pub const INTERACTION_PROVENANCE_GRAPH_SCHEMA: &str = "B_CORE_INTERACTION_PROVENANCE_GRAPH_IR_1";

const MAX_PROVENANCE_NODES: usize = 1_024;
const MAX_PROVENANCE_EDGES: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionProvenanceNodeKindIR {
    LanguageInput,
    SemanticGoal,
    PlannedAction,
    LanguageReport,
    ExecutionObservation,
    VerificationReceipt,
    VerifiedResult,
    RealizedClaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InteractionProvenanceRelationIR {
    InputGroundsGoal,
    GoalProjectsPlan,
    ReportDescribesPlan,
    SupersedesReport,
    ObservationStartsExecution,
    VerificationVerifiesObservation,
    VerificationEstablishesResult,
    SourceGroundsClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionProvenanceNodeIR {
    pub node_id: String,
    pub kind: InteractionProvenanceNodeKindIR,
    pub source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    pub turn_index: u64,
    pub content_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<ActionExecutionStatusIR>,
    pub verified: bool,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub node_sha256: String,
}

impl InteractionProvenanceNodeIR {
    fn validate(&self, completed_turns: u64) -> bool {
        let outcome_valid = match self.kind {
            InteractionProvenanceNodeKindIR::ExecutionObservation => {
                self.outcome == Some(ActionExecutionStatusIR::InProgress) && self.verified
            }
            InteractionProvenanceNodeKindIR::VerificationReceipt
            | InteractionProvenanceNodeKindIR::VerifiedResult => {
                matches!(
                    self.outcome,
                    Some(ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed)
                ) && self.verified
            }
            InteractionProvenanceNodeKindIR::RealizedClaim => self.outcome.is_none(),
            _ => self.outcome.is_none() && !self.verified,
        };
        valid_id(&self.node_id)
            && valid_id(&self.source_id)
            && self.action_id.as_deref().is_none_or(valid_id)
            && self.turn_index > 0
            && self.turn_index <= completed_turns
            && valid_digest(&self.content_sha256)
            && outcome_valid
            && !self.semantic_authority
            && !self.external_action_executed
            && valid_digest(&self.node_sha256)
            && self.node_sha256 == interaction_provenance_node_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionProvenanceEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation: InteractionProvenanceRelationIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub edge_sha256: String,
}

impl InteractionProvenanceEdgeIR {
    fn validate(&self) -> bool {
        valid_id(&self.edge_id)
            && valid_id(&self.source_node_id)
            && valid_id(&self.target_node_id)
            && self.source_node_id != self.target_node_id
            && self.action_id.as_deref().is_none_or(valid_id)
            && !self.semantic_authority
            && !self.external_action_executed
            && valid_digest(&self.edge_sha256)
            && self.edge_sha256 == interaction_provenance_edge_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionProvenanceGraphIR {
    pub schema: String,
    pub conversation_id: String,
    pub current_request_id: String,
    pub completed_turns: u64,
    #[serde(default)]
    pub nodes: Vec<InteractionProvenanceNodeIR>,
    #[serde(default)]
    pub edges: Vec<InteractionProvenanceEdgeIR>,
    pub unsupported_links: usize,
    pub semantic_authority: bool,
    pub language_can_advance_execution: bool,
    pub external_action_executed: bool,
    pub graph_sha256: String,
}

impl InteractionProvenanceGraphIR {
    pub fn validate(&self) -> bool {
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect::<BTreeSet<_>>();
        let edge_ids = self
            .edges
            .iter()
            .map(|edge| edge.edge_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == INTERACTION_PROVENANCE_GRAPH_SCHEMA
            && valid_id(&self.conversation_id)
            && valid_id(&self.current_request_id)
            && self.completed_turns > 0
            && !self.nodes.is_empty()
            && self.nodes.len() <= MAX_PROVENANCE_NODES
            && self.edges.len() <= MAX_PROVENANCE_EDGES
            && node_ids.len() == self.nodes.len()
            && edge_ids.len() == self.edges.len()
            && self
                .nodes
                .iter()
                .all(|node| node.validate(self.completed_turns))
            && self.edges.iter().all(|edge| {
                edge.validate()
                    && node_ids.contains(edge.source_node_id.as_str())
                    && node_ids.contains(edge.target_node_id.as_str())
                    && self.edge_types_are_valid(edge)
            })
            && self.claim_sources_are_complete()
            && self.execution_chains_are_complete()
            && self.unsupported_links == 0
            && !self.semantic_authority
            && !self.language_can_advance_execution
            && !self.external_action_executed
            && valid_digest(&self.graph_sha256)
            && self.graph_sha256 == interaction_provenance_graph_sha256(self)
    }

    pub fn validate_against(
        &self,
        realization: &EvidenceGroundedRealizationIR,
        ledger: &ActionStateLedgerIR,
    ) -> bool {
        if !self.validate()
            || !realization.validate()
            || !ledger.validate(self.completed_turns)
            || self
                .nodes
                .iter()
                .filter(|node| node.kind == InteractionProvenanceNodeKindIR::RealizedClaim)
                .count()
                != realization.claims.len()
        {
            return false;
        }
        let claims_match = realization.claims.iter().all(|claim| {
            self.nodes.iter().any(|node| {
                node.kind == InteractionProvenanceNodeKindIR::RealizedClaim
                    && node.source_id == claim.claim_id
                    && node.content_sha256 == content_sha256(claim)
                    && node.verified == claim.verified
            })
        });
        let reports_match = ledger.language_report_history.iter().all(|report| {
            self.nodes.iter().any(|node| {
                node.kind == InteractionProvenanceNodeKindIR::LanguageReport
                    && node.source_id == report.report_id
                    && node.content_sha256 == report.report_sha256
            })
        });
        let audits_match = ledger.evidence_audit_history.iter().all(|audit| {
            self.nodes.iter().any(|node| {
                node.source_id == audit.receipt_id && node.content_sha256 == audit.audit_sha256
            })
        });
        claims_match && reports_match && audits_match
    }

    fn node(&self, node_id: &str) -> Option<&InteractionProvenanceNodeIR> {
        self.nodes.iter().find(|node| node.node_id == node_id)
    }

    fn edge_types_are_valid(&self, edge: &InteractionProvenanceEdgeIR) -> bool {
        let Some(source) = self.node(&edge.source_node_id) else {
            return false;
        };
        let Some(target) = self.node(&edge.target_node_id) else {
            return false;
        };
        let pair = (source.kind, target.kind);
        let relation_valid = match edge.relation {
            InteractionProvenanceRelationIR::InputGroundsGoal => {
                pair == (
                    InteractionProvenanceNodeKindIR::LanguageInput,
                    InteractionProvenanceNodeKindIR::SemanticGoal,
                )
            }
            InteractionProvenanceRelationIR::GoalProjectsPlan => {
                pair == (
                    InteractionProvenanceNodeKindIR::SemanticGoal,
                    InteractionProvenanceNodeKindIR::PlannedAction,
                )
            }
            InteractionProvenanceRelationIR::ReportDescribesPlan => {
                pair == (
                    InteractionProvenanceNodeKindIR::LanguageReport,
                    InteractionProvenanceNodeKindIR::PlannedAction,
                )
            }
            InteractionProvenanceRelationIR::SupersedesReport => {
                pair == (
                    InteractionProvenanceNodeKindIR::LanguageReport,
                    InteractionProvenanceNodeKindIR::LanguageReport,
                )
            }
            InteractionProvenanceRelationIR::ObservationStartsExecution => {
                pair == (
                    InteractionProvenanceNodeKindIR::ExecutionObservation,
                    InteractionProvenanceNodeKindIR::PlannedAction,
                )
            }
            InteractionProvenanceRelationIR::VerificationVerifiesObservation => {
                pair == (
                    InteractionProvenanceNodeKindIR::VerificationReceipt,
                    InteractionProvenanceNodeKindIR::ExecutionObservation,
                )
            }
            InteractionProvenanceRelationIR::VerificationEstablishesResult => {
                pair == (
                    InteractionProvenanceNodeKindIR::VerificationReceipt,
                    InteractionProvenanceNodeKindIR::VerifiedResult,
                )
            }
            InteractionProvenanceRelationIR::SourceGroundsClaim => {
                target.kind == InteractionProvenanceNodeKindIR::RealizedClaim
                    && source.kind != InteractionProvenanceNodeKindIR::RealizedClaim
            }
        };
        let action_valid = match (&source.action_id, &target.action_id, &edge.action_id) {
            (Some(source), Some(target), Some(edge)) => source == target && target == edge,
            (None, None, None) => matches!(
                edge.relation,
                InteractionProvenanceRelationIR::InputGroundsGoal
                    | InteractionProvenanceRelationIR::GoalProjectsPlan
                    | InteractionProvenanceRelationIR::SourceGroundsClaim
            ),
            (Some(source), None, Some(edge)) | (None, Some(source), Some(edge)) => source == edge,
            _ => false,
        };
        relation_valid && action_valid
    }

    fn claim_sources_are_complete(&self) -> bool {
        self.nodes
            .iter()
            .filter(|node| node.kind == InteractionProvenanceNodeKindIR::RealizedClaim)
            .all(|claim| {
                let sources = self
                    .edges
                    .iter()
                    .filter(|edge| {
                        edge.relation == InteractionProvenanceRelationIR::SourceGroundsClaim
                            && edge.target_node_id == claim.node_id
                    })
                    .filter_map(|edge| self.node(&edge.source_node_id))
                    .collect::<Vec<_>>();
                !sources.is_empty()
                    && (!claim.verified
                        || sources.iter().all(|source| {
                            matches!(
                                source.kind,
                                InteractionProvenanceNodeKindIR::ExecutionObservation
                                    | InteractionProvenanceNodeKindIR::VerifiedResult
                            )
                        }))
            })
    }

    fn execution_chains_are_complete(&self) -> bool {
        let observations_valid = self
            .nodes
            .iter()
            .filter(|node| node.kind == InteractionProvenanceNodeKindIR::ExecutionObservation)
            .all(|observation| {
                self.edges.iter().any(|edge| {
                    edge.source_node_id == observation.node_id
                        && edge.relation
                            == InteractionProvenanceRelationIR::ObservationStartsExecution
                })
            });
        let verification_valid = self
            .nodes
            .iter()
            .filter(|node| node.kind == InteractionProvenanceNodeKindIR::VerificationReceipt)
            .all(|verification| {
                self.edges.iter().any(|edge| {
                    edge.source_node_id == verification.node_id
                        && edge.relation
                            == InteractionProvenanceRelationIR::VerificationVerifiesObservation
                }) && self.edges.iter().any(|edge| {
                    edge.source_node_id == verification.node_id
                        && edge.relation
                            == InteractionProvenanceRelationIR::VerificationEstablishesResult
                })
            });
        let results_valid = self
            .nodes
            .iter()
            .filter(|node| node.kind == InteractionProvenanceNodeKindIR::VerifiedResult)
            .all(|result| {
                self.edges.iter().any(|edge| {
                    edge.target_node_id == result.node_id
                        && edge.relation
                            == InteractionProvenanceRelationIR::VerificationEstablishesResult
                })
            });
        observations_valid && verification_valid && results_valid
    }
}

pub struct InteractionProvenanceSources<'a> {
    pub conversation_id: &'a str,
    pub request_id: &'a str,
    pub raw_language_input: &'a str,
    pub turn_index: u64,
    pub grounded_plan: Option<&'a PlanIR>,
    pub action_ledger: &'a ActionStateLedgerIR,
    pub grounded_realization: &'a EvidenceGroundedRealizationIR,
}

pub fn build_interaction_provenance(
    source: InteractionProvenanceSources<'_>,
) -> InteractionProvenanceGraphIR {
    let mut builder = ProvenanceBuilder::new(&source);
    builder.add_action_chains(source.action_ledger);
    builder.add_reports(source.action_ledger);
    builder.add_execution_evidence(source.action_ledger);
    builder.add_claims(source.grounded_realization, source.action_ledger);
    let mut graph = InteractionProvenanceGraphIR {
        schema: INTERACTION_PROVENANCE_GRAPH_SCHEMA.to_string(),
        conversation_id: source.conversation_id.to_string(),
        current_request_id: source.request_id.to_string(),
        completed_turns: source.turn_index,
        nodes: builder.nodes.into_values().collect(),
        edges: builder.edges.into_values().collect(),
        unsupported_links: 0,
        semantic_authority: false,
        language_can_advance_execution: false,
        external_action_executed: false,
        graph_sha256: String::new(),
    };
    graph.graph_sha256 = interaction_provenance_graph_sha256(&graph);
    debug_assert!(
        graph.validate_against(source.grounded_realization, source.action_ledger),
        "invalid interaction provenance graph: {graph:#?}\nrealization: {:#?}\nledger: {:#?}",
        source.grounded_realization,
        source.action_ledger
    );
    graph
}

struct ProvenanceBuilder {
    current_request_node_id: String,
    current_plan_node_id: Option<String>,
    current_turn: u64,
    nodes: BTreeMap<String, InteractionProvenanceNodeIR>,
    edges: BTreeMap<String, InteractionProvenanceEdgeIR>,
}

impl ProvenanceBuilder {
    fn new(source: &InteractionProvenanceSources<'_>) -> Self {
        let current_request_node_id = format!("INPUT-{}", source.request_id);
        let mut builder = Self {
            current_request_node_id: current_request_node_id.clone(),
            current_plan_node_id: None,
            current_turn: source.turn_index,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        };
        builder.add_node(
            current_request_node_id.clone(),
            InteractionProvenanceNodeKindIR::LanguageInput,
            source.request_id.to_string(),
            None,
            source.turn_index,
            content_sha256(&source.raw_language_input),
            None,
            false,
        );
        if let Some(plan) = source.grounded_plan {
            let goal_node_id = format!("GOALIR-{}", source.request_id);
            let plan_node_id = format!("PLANIR-{}", source.request_id);
            builder.add_node(
                goal_node_id.clone(),
                InteractionProvenanceNodeKindIR::SemanticGoal,
                plan.goal_id.clone(),
                None,
                source.turn_index,
                content_sha256(&(
                    plan.goal_id.as_str(),
                    plan.intent,
                    plan.structurally_validated,
                )),
                None,
                false,
            );
            builder.add_node(
                plan_node_id.clone(),
                InteractionProvenanceNodeKindIR::PlannedAction,
                plan.goal_id.clone(),
                None,
                source.turn_index,
                plan.plan_sha256.clone(),
                None,
                false,
            );
            builder.add_edge(
                current_request_node_id.clone(),
                goal_node_id.clone(),
                InteractionProvenanceRelationIR::InputGroundsGoal,
                None,
            );
            builder.add_edge(
                goal_node_id,
                plan_node_id.clone(),
                InteractionProvenanceRelationIR::GoalProjectsPlan,
                None,
            );
            builder.current_plan_node_id = Some(plan_node_id);
        }
        builder
    }

    fn add_action_chains(&mut self, ledger: &ActionStateLedgerIR) {
        for record in &ledger.records {
            let input_id = if record.introduced_turn == self.current_turn {
                self.current_request_node_id.clone()
            } else {
                let node_id = format!("INPUT-ACTION-{}", record.action_id);
                self.add_node(
                    node_id.clone(),
                    InteractionProvenanceNodeKindIR::LanguageInput,
                    format!("REQUEST-{}", record.action_id),
                    Some(record.action_id.clone()),
                    record.introduced_turn,
                    content_sha256(&record.source_semantic_text),
                    None,
                    false,
                );
                node_id
            };
            let goal_id = goal_node_id(record);
            let plan_id = plan_node_id(record);
            self.add_node(
                goal_id.clone(),
                InteractionProvenanceNodeKindIR::SemanticGoal,
                record.goal_id.clone(),
                Some(record.action_id.clone()),
                record.introduced_turn,
                content_sha256(&(
                    record.goal_id.as_str(),
                    record.canonical_predicate.as_str(),
                    record.subject.as_str(),
                    record.source_semantic_text.as_str(),
                )),
                None,
                false,
            );
            self.add_node(
                plan_id.clone(),
                InteractionProvenanceNodeKindIR::PlannedAction,
                record.action_id.clone(),
                Some(record.action_id.clone()),
                record.introduced_turn,
                content_sha256(&(
                    record.action_id.as_str(),
                    record.goal_id.as_str(),
                    record.canonical_predicate.as_str(),
                    record.subject.as_str(),
                    record.plan_status,
                    record.external_execution_authorized,
                )),
                None,
                false,
            );
            self.add_edge(
                input_id,
                goal_id.clone(),
                InteractionProvenanceRelationIR::InputGroundsGoal,
                Some(record.action_id.clone()),
            );
            self.add_edge(
                goal_id,
                plan_id,
                InteractionProvenanceRelationIR::GoalProjectsPlan,
                Some(record.action_id.clone()),
            );
        }
    }

    fn add_reports(&mut self, ledger: &ActionStateLedgerIR) {
        let mut previous_by_action = BTreeMap::<String, String>::new();
        for report in &ledger.language_report_history {
            let report_id = report_node_id(report);
            self.add_node(
                report_id.clone(),
                InteractionProvenanceNodeKindIR::LanguageReport,
                report.report_id.clone(),
                Some(report.action_id.clone()),
                report.turn_index,
                report.report_sha256.clone(),
                None,
                false,
            );
            self.add_edge(
                report_id.clone(),
                format!("PLAN-{}", report.action_id),
                InteractionProvenanceRelationIR::ReportDescribesPlan,
                Some(report.action_id.clone()),
            );
            if let Some(previous) =
                previous_by_action.insert(report.action_id.clone(), report_id.clone())
            {
                self.add_edge(
                    report_id,
                    previous,
                    InteractionProvenanceRelationIR::SupersedesReport,
                    Some(report.action_id.clone()),
                );
            }
        }
    }

    fn add_execution_evidence(&mut self, ledger: &ActionStateLedgerIR) {
        let mut observation_by_execution = BTreeMap::<(String, String), String>::new();
        for audit in &ledger.evidence_audit_history {
            match audit.status {
                ActionEvidenceStatusIR::ExecutionStarted => {
                    let observation_id = observation_node_id(audit);
                    self.add_node(
                        observation_id.clone(),
                        InteractionProvenanceNodeKindIR::ExecutionObservation,
                        audit.receipt_id.clone(),
                        Some(audit.action_id.clone()),
                        audit.accepted_turn,
                        audit.audit_sha256.clone(),
                        Some(ActionExecutionStatusIR::InProgress),
                        true,
                    );
                    self.add_edge(
                        observation_id.clone(),
                        format!("PLAN-{}", audit.action_id),
                        InteractionProvenanceRelationIR::ObservationStartsExecution,
                        Some(audit.action_id.clone()),
                    );
                    observation_by_execution.insert(
                        (audit.action_id.clone(), audit.execution_id.clone()),
                        observation_id,
                    );
                }
                ActionEvidenceStatusIR::Succeeded | ActionEvidenceStatusIR::Failed => {
                    let verification_id = verification_node_id(audit);
                    let result_id = result_node_id(audit);
                    let outcome = Some(audit.resulting_execution_status);
                    self.add_node(
                        verification_id.clone(),
                        InteractionProvenanceNodeKindIR::VerificationReceipt,
                        audit.receipt_id.clone(),
                        Some(audit.action_id.clone()),
                        audit.accepted_turn,
                        audit.audit_sha256.clone(),
                        outcome,
                        true,
                    );
                    self.add_node(
                        result_id.clone(),
                        InteractionProvenanceNodeKindIR::VerifiedResult,
                        format!("RESULT-{}", audit.receipt_id),
                        Some(audit.action_id.clone()),
                        audit.accepted_turn,
                        content_sha256(&(
                            audit.action_id.as_str(),
                            audit.execution_id.as_str(),
                            audit.resulting_execution_status,
                            audit.audit_sha256.as_str(),
                        )),
                        outcome,
                        true,
                    );
                    if let Some(observation_id) = observation_by_execution
                        .get(&(audit.action_id.clone(), audit.execution_id.clone()))
                    {
                        self.add_edge(
                            verification_id.clone(),
                            observation_id.clone(),
                            InteractionProvenanceRelationIR::VerificationVerifiesObservation,
                            Some(audit.action_id.clone()),
                        );
                    }
                    self.add_edge(
                        verification_id,
                        result_id,
                        InteractionProvenanceRelationIR::VerificationEstablishesResult,
                        Some(audit.action_id.clone()),
                    );
                }
            }
        }
    }

    fn add_claims(
        &mut self,
        realization: &EvidenceGroundedRealizationIR,
        ledger: &ActionStateLedgerIR,
    ) {
        for claim in &realization.claims {
            let claim_node_id = format!("CLAIMNODE-{}", claim.claim_id);
            let claim_actions = claim_action_ids(claim, ledger);
            self.add_node(
                claim_node_id.clone(),
                InteractionProvenanceNodeKindIR::RealizedClaim,
                claim.claim_id.clone(),
                (claim_actions.len() == 1).then(|| claim_actions[0].clone()),
                claim
                    .source_turns
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(self.current_turn),
                content_sha256(claim),
                None,
                claim.verified,
            );
            let mut sources = self.sources_for_claim(claim, &claim_actions, ledger);
            if sources.is_empty() {
                sources.push((self.current_request_node_id.clone(), None));
            }
            for (source_id, action_id) in sources {
                self.add_edge(
                    source_id,
                    claim_node_id.clone(),
                    InteractionProvenanceRelationIR::SourceGroundsClaim,
                    action_id,
                );
            }
        }
    }

    fn sources_for_claim(
        &self,
        claim: &GroundedClaimIR,
        action_ids: &[String],
        ledger: &ActionStateLedgerIR,
    ) -> Vec<(String, Option<String>)> {
        if claim.support_status == ClaimSupportStatusIR::VerifiedEvidence {
            return action_ids
                .iter()
                .filter_map(|action_id| {
                    let record = ledger.record(action_id)?;
                    let audit = ledger
                        .evidence_audit_history
                        .iter()
                        .rev()
                        .find(|audit| audit.action_id == *action_id)?;
                    let source_id = match record.execution_status {
                        ActionExecutionStatusIR::InProgress => observation_node_id(audit),
                        ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed => {
                            result_node_id(audit)
                        }
                        ActionExecutionStatusIR::NotObserved => return None,
                    };
                    Some((source_id, Some(action_id.clone())))
                })
                .collect();
        }
        if claim.support_status == ClaimSupportStatusIR::ReportedOnly
            || claim.kind == GroundedClaimKindIR::LanguageReport
        {
            return action_ids
                .iter()
                .filter_map(|action_id| {
                    ledger
                        .language_report_history
                        .iter()
                        .rev()
                        .find(|report| report.action_id == *action_id)
                        .map(|report| (report_node_id(report), Some(action_id.clone())))
                })
                .collect();
        }
        if !action_ids.is_empty() {
            return action_ids
                .iter()
                .map(|action_id| (format!("PLAN-{action_id}"), Some(action_id.clone())))
                .collect();
        }
        if matches!(
            claim.kind,
            GroundedClaimKindIR::PlanStatus
                | GroundedClaimKindIR::ActionSetEvaluation
                | GroundedClaimKindIR::EvidenceAbsence
        ) {
            if action_ids.is_empty() && claim.kind == GroundedClaimKindIR::PlanStatus {
                return self
                    .current_plan_node_id
                    .iter()
                    .map(|node_id| (node_id.clone(), None))
                    .collect();
            }
            return action_ids
                .iter()
                .map(|action_id| (format!("PLAN-{action_id}"), Some(action_id.clone())))
                .collect();
        }
        Vec::new()
    }

    #[allow(clippy::too_many_arguments)]
    fn add_node(
        &mut self,
        node_id: String,
        kind: InteractionProvenanceNodeKindIR,
        source_id: String,
        action_id: Option<String>,
        turn_index: u64,
        content_sha256: String,
        outcome: Option<ActionExecutionStatusIR>,
        verified: bool,
    ) {
        let mut node = InteractionProvenanceNodeIR {
            node_id: node_id.clone(),
            kind,
            source_id,
            action_id,
            turn_index,
            content_sha256,
            outcome,
            verified,
            semantic_authority: false,
            external_action_executed: false,
            node_sha256: String::new(),
        };
        node.node_sha256 = interaction_provenance_node_sha256(&node);
        self.nodes.entry(node_id).or_insert(node);
    }

    fn add_edge(
        &mut self,
        source_node_id: String,
        target_node_id: String,
        relation: InteractionProvenanceRelationIR,
        action_id: Option<String>,
    ) {
        let identity = content_sha256(&(
            source_node_id.as_str(),
            target_node_id.as_str(),
            relation,
            &action_id,
        ));
        let edge_id = format!("EDGE-{}", &identity[..16]);
        let mut edge = InteractionProvenanceEdgeIR {
            edge_id: edge_id.clone(),
            source_node_id,
            target_node_id,
            relation,
            action_id,
            semantic_authority: false,
            external_action_executed: false,
            edge_sha256: String::new(),
        };
        edge.edge_sha256 = interaction_provenance_edge_sha256(&edge);
        self.edges.entry(edge_id).or_insert(edge);
    }
}

fn claim_action_ids(claim: &GroundedClaimIR, ledger: &ActionStateLedgerIR) -> Vec<String> {
    ledger
        .records
        .iter()
        .filter(|record| {
            claim
                .evidence_refs
                .iter()
                .any(|evidence| evidence == &record.action_id || evidence == &record.goal_id)
        })
        .map(|record| record.action_id.clone())
        .collect()
}

fn goal_node_id(record: &ActionStateRecordIR) -> String {
    format!("GOALNODE-{}", record.action_id)
}

fn plan_node_id(record: &ActionStateRecordIR) -> String {
    format!("PLAN-{}", record.action_id)
}

fn report_node_id(report: &ActionLanguageReportRecordIR) -> String {
    format!("REPORTNODE-{}", report.report_id)
}

fn observation_node_id(audit: &ActionEvidenceAuditIR) -> String {
    format!("OBSERVATION-{}", audit.receipt_id)
}

fn verification_node_id(audit: &ActionEvidenceAuditIR) -> String {
    format!("VERIFICATION-{}", audit.receipt_id)
}

fn result_node_id(audit: &ActionEvidenceAuditIR) -> String {
    format!("RESULTNODE-{}", audit.receipt_id)
}

pub fn interaction_provenance_node_sha256(node: &InteractionProvenanceNodeIR) -> String {
    content_sha256(&(
        node.node_id.as_str(),
        node.kind,
        node.source_id.as_str(),
        &node.action_id,
        node.turn_index,
        node.content_sha256.as_str(),
        node.outcome,
        node.verified,
        node.semantic_authority,
        node.external_action_executed,
    ))
}

pub fn interaction_provenance_edge_sha256(edge: &InteractionProvenanceEdgeIR) -> String {
    content_sha256(&(
        edge.edge_id.as_str(),
        edge.source_node_id.as_str(),
        edge.target_node_id.as_str(),
        edge.relation,
        &edge.action_id,
        edge.semantic_authority,
        edge.external_action_executed,
    ))
}

pub fn interaction_provenance_graph_sha256(graph: &InteractionProvenanceGraphIR) -> String {
    content_sha256(&(
        graph.schema.as_str(),
        graph.conversation_id.as_str(),
        graph.current_request_id.as_str(),
        graph.completed_turns,
        &graph.nodes,
        &graph.edges,
        graph.unsupported_links,
        graph.semantic_authority,
        graph.language_can_advance_execution,
        graph.external_action_executed,
    ))
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("bounded provenance payload serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 320
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_state::{
        action_evidence_receipt_sha256, ActionEvidenceRequestIR, ActionPlanSeedIR,
        ActionReportedStatusIR, ActionStateAnalyzer, ACTION_EVIDENCE_REQUEST_SCHEMA,
    };
    use crate::grounded_realization::{
        build_evidence_grounded_realization, GroundedRealizationSources,
    };
    use crate::language_knowledge::LanguageCodeIR;

    fn ledger_with_plan() -> ActionStateLedgerIR {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[ActionPlanSeedIR {
            action_id: "GOAL-1".to_string(),
            goal_id: "GOAL-1".to_string(),
            canonical_predicate: "INVESTIGATE".to_string(),
            predicate_surface: "inspect".to_string(),
            subject: "queue".to_string(),
            source_semantic_text: "inspect the queue".to_string(),
            introduced_turn: 1,
            external_execution_authorized: true,
        }]);
        ledger
    }

    fn evidence(status: ActionEvidenceStatusIR, suffix: &str) -> ActionEvidenceRequestIR {
        let mut request = ActionEvidenceRequestIR {
            schema: ACTION_EVIDENCE_REQUEST_SCHEMA.to_string(),
            receipt_id: format!("RECEIPT-{suffix}"),
            conversation_id: "CHAT-1".to_string(),
            action_id: "GOAL-1".to_string(),
            execution_id: "EXEC-1".to_string(),
            status,
            evidence_digest: format!("{:064x}", suffix.len()),
            verifier_receipt_sha256: String::new(),
        };
        request.verifier_receipt_sha256 = action_evidence_receipt_sha256(&request);
        request
    }

    fn realization(
        analysis: &crate::action_state::ActionStateAnalysisIR,
        ledger: &ActionStateLedgerIR,
        turn: u64,
    ) -> EvidenceGroundedRealizationIR {
        build_evidence_grounded_realization(GroundedRealizationSources {
            language: LanguageCodeIR::English,
            realized_text: "The status is grounded.",
            turn_index: turn,
            plan: None,
            action_analysis: analysis,
            action_ledger: ledger,
            competing_outcome_reports: false,
            epistemic_ledger: None,
            discourse_group_update: None,
            topic_transition: None,
            active_topic: None,
            topic_anchored_reference: None,
            discourse_answer: None,
            dialogue_relation_answer: None,
            temporal_answer: None,
            guard_evaluations: &[],
            evidence_absence: false,
            source_unsupported_claims: 0,
        })
    }

    #[test]
    fn language_report_cannot_create_verified_result_chain() {
        let mut ledger = ledger_with_plan();
        let analysis = ActionStateAnalyzer.analyze("I completed it", &ledger);
        assert!(ledger.apply_language_report(analysis.detected_report.as_ref().expect("report"), 2));
        let realized = realization(&analysis, &ledger, 2);
        let graph = build_interaction_provenance(InteractionProvenanceSources {
            conversation_id: "CHAT-1",
            request_id: "REQ-2",
            raw_language_input: "I completed it",
            turn_index: 2,
            grounded_plan: None,
            action_ledger: &ledger,
            grounded_realization: &realized,
        });
        assert!(graph.validate_against(&realized, &ledger));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == InteractionProvenanceNodeKindIR::LanguageReport));
        assert!(!graph.nodes.iter().any(|node| matches!(
            node.kind,
            InteractionProvenanceNodeKindIR::ExecutionObservation
                | InteractionProvenanceNodeKindIR::VerifiedResult
        )));
    }

    #[test]
    fn terminal_result_requires_start_and_terminal_audits() {
        let mut ledger = ledger_with_plan();
        assert!(ledger
            .apply_evidence(
                &evidence(ActionEvidenceStatusIR::ExecutionStarted, "START"),
                1
            )
            .is_some());
        assert!(ledger
            .apply_evidence(&evidence(ActionEvidenceStatusIR::Succeeded, "DONE"), 1)
            .is_some());
        let analysis = ActionStateAnalyzer.analyze("what is the verified result?", &ledger);
        let realized = realization(&analysis, &ledger, 2);
        let graph = build_interaction_provenance(InteractionProvenanceSources {
            conversation_id: "CHAT-1",
            request_id: "REQ-2",
            raw_language_input: "what is the verified result?",
            turn_index: 2,
            grounded_plan: None,
            action_ledger: &ledger,
            grounded_realization: &realized,
        });
        assert!(graph.validate_against(&realized, &ledger));
        assert!(graph.edges.iter().any(|edge| {
            edge.relation == InteractionProvenanceRelationIR::VerificationEstablishesResult
        }));
    }

    #[test]
    fn graph_and_node_tampering_fail_validation() {
        let ledger = ledger_with_plan();
        let analysis = ActionStateAnalyzer.analyze("what is the plan?", &ledger);
        let realized = realization(&analysis, &ledger, 1);
        let graph = build_interaction_provenance(InteractionProvenanceSources {
            conversation_id: "CHAT-1",
            request_id: "REQ-1",
            raw_language_input: "what is the plan?",
            turn_index: 1,
            grounded_plan: None,
            action_ledger: &ledger,
            grounded_realization: &realized,
        });
        assert!(graph.validate());
        let mut tampered = graph.clone();
        tampered.nodes[0].verified = true;
        assert!(!tampered.validate());
        let mut tampered = graph;
        tampered.unsupported_links = 1;
        assert!(!tampered.validate());
    }

    #[test]
    fn report_revision_is_preserved_without_execution_authority() {
        let mut ledger = ledger_with_plan();
        let first = ActionStateAnalyzer.analyze("I completed it", &ledger);
        assert!(ledger.apply_language_report(first.detected_report.as_ref().unwrap(), 2));
        let mut second = first.detected_report.unwrap();
        second.reported_status = ActionReportedStatusIR::FailureClaimed;
        second.source_surface = "Correction: it failed".to_string();
        assert!(ledger.apply_language_report(&second, 3));
        assert_eq!(ledger.language_report_history.len(), 2);
        assert_eq!(
            ledger.record("GOAL-1").unwrap().execution_status,
            ActionExecutionStatusIR::NotObserved
        );
        assert!(ledger.validate(3));
    }
}
