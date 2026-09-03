//! Bounded cross-turn causal and concessive discourse relations.
//!
//! These records describe how dialogue turns are connected.  They are not a
//! mechanism model and never establish causal truth, semantic authority, or
//! permission to act.  The graph is conversation-local, capacity bounded, and
//! content hashed so that corrupt state fails closed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::attribution::AttributedPropositionPolarityIR;
use crate::conversation::{DiscourseReferentKindIR, DynamicDiscourseReferentIR};
use crate::epistemic::{BeliefRecordStatusIR, BeliefRevisionKindIR, EpistemicLedgerIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::ModalWorldIR;

pub const DIALOGUE_RELATION_GRAPH_SCHEMA: &str = "B_CORE_DIALOGUE_RELATION_GRAPH_IR_2";
pub const DIALOGUE_RELATION_ANSWER_SCHEMA: &str = "B_CORE_DIALOGUE_RELATION_ANSWER_IR_2";
pub const MAX_DIALOGUE_RELATIONS: usize = 32;
pub const MAX_RELATION_TURN_DISTANCE: u64 = 8;
const MAX_RELATION_EVIDENCE: usize = MAX_DIALOGUE_RELATION_PATHS * MAX_DIALOGUE_RELATION_PATH_HOPS;
pub const MAX_DIALOGUE_RELATION_PATHS: usize = 8;
pub const MAX_DIALOGUE_RELATION_PATH_HOPS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueRelationKindIR {
    Cause,
    Consequence,
    Concession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueRelationStatusIR {
    Active,
    SourceInactive,
    TargetInactive,
    BothInactive,
}

impl DialogueRelationStatusIR {
    fn from_endpoint_activity(source_active: bool, target_active: bool) -> Self {
        match (source_active, target_active) {
            (true, true) => Self::Active,
            (false, true) => Self::SourceInactive,
            (true, false) => Self::TargetInactive,
            (false, false) => Self::BothInactive,
        }
    }

    pub fn is_active(self) -> bool {
        self == Self::Active
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationEdgeIR {
    pub relation_id: String,
    pub kind: DialogueRelationKindIR,
    pub source_referent_id: String,
    pub target_referent_id: String,
    pub source_belief_id: String,
    pub target_belief_id: String,
    pub source_belief_status: BeliefRecordStatusIR,
    pub target_belief_status: BeliefRecordStatusIR,
    pub source_modal_world: ModalWorldIR,
    pub target_modal_world: ModalWorldIR,
    pub source_polarity: AttributedPropositionPolarityIR,
    pub target_polarity: AttributedPropositionPolarityIR,
    pub source_turn: u64,
    pub target_turn: u64,
    pub source_summary: String,
    pub target_summary: String,
    pub connector_surface: String,
    pub confidence_millis: u16,
    pub status: DialogueRelationStatusIR,
    pub last_updated_turn: u64,
    pub dialogue_claim_only: bool,
    pub causal_truth_established: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl DialogueRelationEdgeIR {
    fn validate(&self, completed_turns: u64) -> bool {
        !self.relation_id.trim().is_empty()
            && !self.source_referent_id.trim().is_empty()
            && !self.target_referent_id.trim().is_empty()
            && !self.source_belief_id.trim().is_empty()
            && !self.target_belief_id.trim().is_empty()
            && self.source_referent_id != self.target_referent_id
            && self.source_turn > 0
            && self.source_turn <= self.target_turn
            && self.target_turn <= completed_turns
            && self.last_updated_turn >= self.target_turn
            && self.last_updated_turn <= completed_turns
            && !self.source_summary.trim().is_empty()
            && !self.target_summary.trim().is_empty()
            && !self.connector_surface.trim().is_empty()
            && self.confidence_millis <= 1_000
            && self.dialogue_claim_only
            && !self.causal_truth_established
            && !self.semantic_authority
            && !self.external_execution_authorized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationGraphIR {
    pub schema: String,
    pub relations: Vec<DialogueRelationEdgeIR>,
    pub graph_sha256: String,
}

impl Default for DialogueRelationGraphIR {
    fn default() -> Self {
        let mut graph = Self {
            schema: DIALOGUE_RELATION_GRAPH_SCHEMA.to_string(),
            relations: Vec::new(),
            graph_sha256: String::new(),
        };
        graph.refresh_hash();
        graph
    }
}

impl DialogueRelationGraphIR {
    pub fn validate(&self, completed_turns: u64) -> bool {
        let ids = self
            .relations
            .iter()
            .map(|edge| edge.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == DIALOGUE_RELATION_GRAPH_SCHEMA
            && self.relations.len() <= MAX_DIALOGUE_RELATIONS
            && ids.len() == self.relations.len()
            && self
                .relations
                .iter()
                .all(|edge| edge.validate(completed_turns))
            && self.graph_sha256.len() == 64
            && self.graph_sha256 == relation_graph_hash(self)
    }

    pub fn validate_with_ledger(&self, completed_turns: u64, ledger: &EpistemicLedgerIR) -> bool {
        self.validate(completed_turns)
            && self.relations.iter().all(|edge| {
                let Some(source) = ledger.record(&edge.source_belief_id) else {
                    return false;
                };
                let Some(target) = ledger.record(&edge.target_belief_id) else {
                    return false;
                };
                let expected_status = DialogueRelationStatusIR::from_endpoint_activity(
                    relation_endpoint_is_active(ledger, &edge.source_belief_id),
                    relation_endpoint_is_active(ledger, &edge.target_belief_id),
                );
                source.origin_referent_id == edge.source_referent_id
                    && target.origin_referent_id == edge.target_referent_id
                    && source.status == edge.source_belief_status
                    && target.status == edge.target_belief_status
                    && source.signature.modal_world == edge.source_modal_world
                    && target.signature.modal_world == edge.target_modal_world
                    && source.proposition_polarity == edge.source_polarity
                    && target.proposition_polarity == edge.target_polarity
                    && edge.status == expected_status
            })
    }

    pub fn has_active_relations(&self) -> bool {
        self.relations.iter().any(|edge| edge.status.is_active())
    }

    pub fn apply_turn(
        &mut self,
        turn_index: u64,
        text: &str,
        used_referent_ids: &[String],
        prior_referents: &[DynamicDiscourseReferentIR],
        current_propositions: &[DynamicDiscourseReferentIR],
    ) {
        let resolution =
            resolve_relation_antecedent(prior_referents, turn_index.saturating_sub(1), text);
        let targets = current_propositions
            .iter()
            .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
            .collect::<Vec<_>>();
        let cross_turn = resolution.kind.and_then(|kind| {
            let source_id = resolution.referent_ids.first().filter(|id| {
                resolution.referent_ids.len() == 1 && used_referent_ids.contains(id)
            })?;
            let source = prior_referents
                .iter()
                .find(|referent| &referent.referent_id == source_id)?;
            let [target] = targets.as_slice() else {
                return None;
            };
            let target_summary = relation_target_surface(text, &resolution.connector_surface)
                .unwrap_or_else(|| target.semantic_summary.clone());
            relation_edge(
                turn_index,
                kind,
                resolution.connector_surface.clone(),
                source,
                target,
                target_summary,
                resolution.confidence_millis,
            )
        });
        let same_turn = || {
            let (kind, connector) = embedded_relation_marker(text)?;
            let [source, target] = targets.as_slice() else {
                return None;
            };
            relation_edge(
                turn_index,
                kind,
                connector.clone(),
                source,
                target,
                embedded_relation_target_surface(text, &connector)
                    .unwrap_or_else(|| target.semantic_summary.clone()),
                920,
            )
        };
        let Some(edge) = cross_turn.or_else(same_turn) else {
            return;
        };
        self.relations
            .retain(|existing| existing.relation_id != edge.relation_id);
        self.relations.push(edge);
        sort_relations_for_retention(&mut self.relations);
        self.relations.truncate(MAX_DIALOGUE_RELATIONS);
        self.refresh_hash();
    }

    pub fn synchronize_with_ledger(&mut self, turn_index: u64, ledger: &EpistemicLedgerIR) {
        self.relations.retain(|edge| {
            ledger.record(&edge.source_belief_id).is_some()
                && ledger.record(&edge.target_belief_id).is_some()
        });
        for edge in &mut self.relations {
            let source = ledger
                .record(&edge.source_belief_id)
                .expect("retained source belief exists");
            let target = ledger
                .record(&edge.target_belief_id)
                .expect("retained target belief exists");
            let next_status = DialogueRelationStatusIR::from_endpoint_activity(
                relation_endpoint_is_active(ledger, &edge.source_belief_id),
                relation_endpoint_is_active(ledger, &edge.target_belief_id),
            );
            if edge.status != next_status
                || edge.source_belief_status != source.status
                || edge.target_belief_status != target.status
            {
                edge.last_updated_turn = turn_index;
            }
            edge.status = next_status;
            edge.source_belief_status = source.status;
            edge.target_belief_status = target.status;
            edge.source_modal_world = source.signature.modal_world;
            edge.target_modal_world = target.signature.modal_world;
            edge.source_polarity = source.proposition_polarity;
            edge.target_polarity = target.proposition_polarity;
        }
        sort_relations_for_retention(&mut self.relations);
        self.relations.truncate(MAX_DIALOGUE_RELATIONS);
        self.refresh_hash();
    }

    fn refresh_hash(&mut self) {
        self.graph_sha256 = relation_graph_hash(self);
    }
}

/// A relation attaches to a proposition, not to one particular repetition of
/// that proposition.  The epistemic ledger retains every repetition as a new
/// belief record and marks the prior equivalent record `Superseded`.  Follow
/// only explicit `Reaffirms` links so that a repeated proposition keeps its
/// earlier discourse relations active, while corrections and retractions still
/// deactivate them.
fn relation_endpoint_is_active(ledger: &EpistemicLedgerIR, belief_id: &str) -> bool {
    let mut current_id = belief_id;
    let mut visited = BTreeSet::new();
    for _ in 0..=ledger.records.len() {
        if !visited.insert(current_id) {
            return false;
        }
        let Some(record) = ledger.record(current_id) else {
            return false;
        };
        if record.status.is_reference_active() {
            return true;
        }
        if record.status != BeliefRecordStatusIR::Superseded {
            return false;
        }
        let Some(successor_id) = ledger
            .revisions
            .iter()
            .filter(|revision| {
                revision.kind == BeliefRevisionKindIR::Reaffirms
                    && revision.prior_belief_id == current_id
            })
            .max_by_key(|revision| revision.turn_index)
            .and_then(|revision| revision.new_belief_id.as_deref())
        else {
            return false;
        };
        current_id = successor_id;
    }
    false
}

fn relation_edge(
    turn_index: u64,
    kind: DialogueRelationKindIR,
    connector_surface: String,
    source: &DynamicDiscourseReferentIR,
    target: &DynamicDiscourseReferentIR,
    target_summary: String,
    confidence_millis: u16,
) -> Option<DialogueRelationEdgeIR> {
    Some(DialogueRelationEdgeIR {
        relation_id: format!("DREL-{turn_index:06}-01"),
        kind,
        source_referent_id: source.referent_id.clone(),
        target_referent_id: target.referent_id.clone(),
        source_belief_id: source.belief_record_id.clone()?,
        target_belief_id: target.belief_record_id.clone()?,
        source_belief_status: BeliefRecordStatusIR::Active,
        target_belief_status: BeliefRecordStatusIR::Active,
        source_modal_world: source.modal_world?,
        target_modal_world: target.modal_world?,
        source_polarity: source.proposition_polarity?,
        target_polarity: target.proposition_polarity?,
        source_turn: source.introduced_turn,
        target_turn: target.introduced_turn,
        source_summary: source.semantic_summary.clone(),
        target_summary,
        connector_surface,
        confidence_millis,
        status: DialogueRelationStatusIR::Active,
        last_updated_turn: turn_index,
        dialogue_claim_only: true,
        causal_truth_established: false,
        semantic_authority: false,
        external_execution_authorized: false,
    })
}

fn sort_relations_for_retention(relations: &mut [DialogueRelationEdgeIR]) {
    relations.sort_by(|left, right| {
        right
            .status
            .is_active()
            .cmp(&left.status.is_active())
            .then_with(|| right.target_turn.cmp(&left.target_turn))
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
}

fn relation_graph_hash(graph: &DialogueRelationGraphIR) -> String {
    let bytes = serde_json::to_vec(&(&graph.schema, &graph.relations))
        .expect("dialogue relation graph is serializable");
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationAntecedentResolution {
    pub detected: bool,
    pub kind: Option<DialogueRelationKindIR>,
    pub connector_surface: String,
    pub referent_ids: Vec<String>,
    pub ambiguous_surfaces: Vec<String>,
    pub confidence_millis: u16,
}

pub fn resolve_relation_antecedent(
    referents: &[DynamicDiscourseReferentIR],
    completed_turns: u64,
    text: &str,
) -> RelationAntecedentResolution {
    let Some((kind, marker)) = leading_relation_marker(text) else {
        return RelationAntecedentResolution {
            detected: false,
            kind: None,
            connector_surface: String::new(),
            referent_ids: Vec::new(),
            ambiguous_surfaces: Vec::new(),
            confidence_millis: 0,
        };
    };
    let mut eligible = referents
        .iter()
        .filter(|referent| {
            referent.kind == DiscourseReferentKindIR::Proposition
                && completed_turns.saturating_sub(referent.introduced_turn)
                    <= MAX_RELATION_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    let latest_turn = eligible.iter().map(|item| item.introduced_turn).max();
    eligible.retain(|item| Some(item.introduced_turn) == latest_turn);
    eligible.sort_by(|left, right| left.referent_id.cmp(&right.referent_id));
    let (referent_ids, ambiguous_surfaces, confidence_millis) = if eligible.len() == 1 {
        (vec![eligible[0].referent_id.clone()], Vec::new(), 940)
    } else {
        (
            Vec::new(),
            vec![format!("DISCOURSE_RELATION_ANTECEDENT:{marker}")],
            0,
        )
    };
    RelationAntecedentResolution {
        detected: true,
        kind: Some(kind),
        connector_surface: marker,
        referent_ids,
        ambiguous_surfaces,
        confidence_millis,
    }
}

pub fn relation_connector_contains_anaphoric_that(text: &str) -> bool {
    leading_relation_marker(text)
        .is_some_and(|(_, marker)| marker.to_ascii_lowercase().contains("that"))
}

fn leading_relation_marker(text: &str) -> Option<(DialogueRelationKindIR, String)> {
    let trimmed = text.trim_start();
    if trimmed.starts_with(['\'', '"', '‘', '“', '`']) {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let markers = [
        (DialogueRelationKindIR::Cause, "because of that"),
        (DialogueRelationKindIR::Cause, "for that reason"),
        (DialogueRelationKindIR::Cause, "그 때문에"),
        (DialogueRelationKindIR::Cause, "그런 이유로"),
        (DialogueRelationKindIR::Consequence, "as a result"),
        (DialogueRelationKindIR::Consequence, "therefore"),
        (DialogueRelationKindIR::Consequence, "consequently"),
        (DialogueRelationKindIR::Consequence, "그래서"),
        (DialogueRelationKindIR::Consequence, "따라서"),
        (DialogueRelationKindIR::Consequence, "그 결과"),
        (DialogueRelationKindIR::Concession, "nevertheless"),
        (DialogueRelationKindIR::Concession, "nonetheless"),
        (DialogueRelationKindIR::Concession, "despite that"),
        (DialogueRelationKindIR::Concession, "even so"),
        (DialogueRelationKindIR::Concession, "그럼에도 불구하고"),
        (DialogueRelationKindIR::Concession, "그럼에도"),
        (DialogueRelationKindIR::Concession, "그런데도"),
        (DialogueRelationKindIR::Concession, "그래도"),
    ];
    markers.into_iter().find_map(|(kind, marker)| {
        lower
            .strip_prefix(marker)
            .filter(|tail| {
                tail.is_empty()
                    || tail
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_whitespace() || [',', '.', ':', ';'].contains(&ch))
            })
            .map(|_| {
                let surface = trimmed.get(..marker.len()).unwrap_or(marker).to_string();
                (kind, surface)
            })
    })
}

fn embedded_relation_marker(text: &str) -> Option<(DialogueRelationKindIR, String)> {
    let lower = text.to_lowercase();
    let markers = [
        (DialogueRelationKindIR::Cause, "because of that"),
        (DialogueRelationKindIR::Cause, "for that reason"),
        (DialogueRelationKindIR::Cause, "그 때문에"),
        (DialogueRelationKindIR::Cause, "그런 이유로"),
        (DialogueRelationKindIR::Consequence, "as a result"),
        (DialogueRelationKindIR::Consequence, "therefore"),
        (DialogueRelationKindIR::Consequence, "그래서"),
        (DialogueRelationKindIR::Consequence, "따라서"),
        (DialogueRelationKindIR::Concession, "nevertheless"),
        (DialogueRelationKindIR::Concession, "그럼에도"),
    ];
    markers.into_iter().find_map(|(kind, marker)| {
        let start = lower.find(marker)?;
        let prefix = lower[..start].trim_end();
        (prefix.is_empty()
            || prefix.ends_with('.')
            || prefix.ends_with('?')
            || prefix.ends_with('!')
            || prefix.ends_with(';'))
        .then(|| {
            let surface = text.get(start..start + marker.len()).unwrap_or(marker);
            (kind, surface.to_string())
        })
    })
}

fn relation_target_surface(text: &str, connector: &str) -> Option<String> {
    let tail = text
        .trim_start()
        .get(connector.len()..)?
        .trim_start_matches([' ', '\t', ',', '.', ':', ';']);
    let target = tail.trim().trim_end_matches(['.', '?', '!']).trim();
    (!target.is_empty()).then(|| target.to_string())
}

fn embedded_relation_target_surface(text: &str, connector: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let start = lower.find(&connector.to_lowercase())? + connector.len();
    let tail = text
        .get(start..)?
        .trim_start_matches([' ', '\t', ',', '.', ':', ';']);
    let target = tail.trim().trim_end_matches(['.', '?', '!']).trim();
    (!target.is_empty()).then(|| target.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueRelationQueryKindIR {
    CauseOf,
    ConsequenceOf,
    ConcessionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationQueryIR {
    pub original_text: String,
    pub kind: DialogueRelationQueryKindIR,
    pub topic_terms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueRelationAnswerDispositionIR {
    AnsweredFromDialogueRelation,
    AnsweredFromDialoguePath,
    MultipleDialogueRelations,
    NoMatchingDialogueRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationEvidenceIR {
    pub relation_id: String,
    pub kind: DialogueRelationKindIR,
    pub source_belief_id: String,
    pub target_belief_id: String,
    pub source_belief_status: BeliefRecordStatusIR,
    pub target_belief_status: BeliefRecordStatusIR,
    pub source_modal_world: ModalWorldIR,
    pub target_modal_world: ModalWorldIR,
    pub source_polarity: AttributedPropositionPolarityIR,
    pub target_polarity: AttributedPropositionPolarityIR,
    pub source_summary: String,
    pub target_summary: String,
    pub source_turn: u64,
    pub target_turn: u64,
    pub dialogue_claim_only: bool,
    pub causal_truth_established: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationPathIR {
    pub path_id: String,
    pub relation_ids: Vec<String>,
    pub root_referent_id: String,
    pub terminal_referent_id: String,
    pub root_summary: String,
    pub terminal_summary: String,
    pub hop_count: usize,
    pub confidence_millis: u16,
    pub contains_nonactual_world: bool,
    pub contains_contested_endpoint: bool,
    pub truncated_by_hop_limit: bool,
    pub dialogue_claim_only: bool,
    pub causal_truth_established: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueRelationAnswerIR {
    pub schema: String,
    pub query: DialogueRelationQueryIR,
    pub disposition: DialogueRelationAnswerDispositionIR,
    pub evidence: Vec<DialogueRelationEvidenceIR>,
    pub paths: Vec<DialogueRelationPathIR>,
    pub language: LanguageCodeIR,
    pub realized_text: String,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
    pub unsupported_claims: usize,
}

impl DialogueRelationAnswerIR {
    pub fn validate(&self) -> bool {
        let ids = self
            .evidence
            .iter()
            .map(|item| item.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == DIALOGUE_RELATION_ANSWER_SCHEMA
            && !self.realized_text.trim().is_empty()
            && self.evidence.len() <= MAX_RELATION_EVIDENCE
            && self.paths.len() <= MAX_DIALOGUE_RELATION_PATHS
            && ids.len() == self.evidence.len()
            && !self.dialogue_truth_established
            && !self.external_execution_authorized
            && self.unsupported_claims == 0
            && self.evidence.iter().all(|item| {
                !item.relation_id.trim().is_empty()
                    && !item.source_belief_id.trim().is_empty()
                    && !item.target_belief_id.trim().is_empty()
                    && !item.source_summary.trim().is_empty()
                    && !item.target_summary.trim().is_empty()
                    && item.source_turn <= item.target_turn
                    && item.dialogue_claim_only
                    && !item.causal_truth_established
                    && !item.semantic_authority
                    && !item.external_execution_authorized
            })
            && self.paths.iter().all(|path| {
                !path.path_id.trim().is_empty()
                    && path.hop_count == path.relation_ids.len()
                    && path.hop_count > 0
                    && path.hop_count <= MAX_DIALOGUE_RELATION_PATH_HOPS
                    && !path.root_referent_id.trim().is_empty()
                    && !path.terminal_referent_id.trim().is_empty()
                    && !path.root_summary.trim().is_empty()
                    && !path.terminal_summary.trim().is_empty()
                    && path.confidence_millis <= 1_000
                    && path.relation_ids.iter().all(|id| ids.contains(id.as_str()))
                    && path.dialogue_claim_only
                    && !path.causal_truth_established
                    && !path.semantic_authority
                    && !path.external_execution_authorized
            })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DialogueRelationQaEngine;

impl DialogueRelationQaEngine {
    pub fn answer(
        &self,
        text: &str,
        graph: Option<&DialogueRelationGraphIR>,
        language: LanguageCodeIR,
    ) -> Option<DialogueRelationAnswerIR> {
        let query = parse_relation_query(text)?;
        let candidates = graph.map_or_else(Vec::new, |graph| relation_paths(&query, graph));
        let disposition = match candidates.as_slice() {
            [] => DialogueRelationAnswerDispositionIR::NoMatchingDialogueRelation,
            [candidate] if candidate.edges.len() == 1 => {
                DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation
            }
            [_] => DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath,
            _ => DialogueRelationAnswerDispositionIR::MultipleDialogueRelations,
        };
        let mut unique_edges = Vec::new();
        let mut seen_relation_ids = BTreeSet::new();
        for candidate in &candidates {
            for edge in &candidate.edges {
                if seen_relation_ids.insert(edge.relation_id.as_str()) {
                    unique_edges.push(*edge);
                }
            }
        }
        let evidence = unique_edges
            .into_iter()
            .map(|edge| DialogueRelationEvidenceIR {
                relation_id: edge.relation_id.clone(),
                kind: edge.kind,
                source_belief_id: edge.source_belief_id.clone(),
                target_belief_id: edge.target_belief_id.clone(),
                source_belief_status: edge.source_belief_status,
                target_belief_status: edge.target_belief_status,
                source_modal_world: edge.source_modal_world,
                target_modal_world: edge.target_modal_world,
                source_polarity: edge.source_polarity,
                target_polarity: edge.target_polarity,
                source_summary: edge.source_summary.clone(),
                target_summary: edge.target_summary.clone(),
                source_turn: edge.source_turn,
                target_turn: edge.target_turn,
                dialogue_claim_only: true,
                causal_truth_established: false,
                semantic_authority: false,
                external_execution_authorized: false,
            })
            .collect::<Vec<_>>();
        let paths = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| path_ir(index, candidate))
            .collect::<Vec<_>>();
        let answer = DialogueRelationAnswerIR {
            schema: DIALOGUE_RELATION_ANSWER_SCHEMA.to_string(),
            realized_text: realize_relation_answer(
                language,
                &query,
                disposition,
                &evidence,
                &paths,
            ),
            query,
            disposition,
            evidence,
            paths,
            language,
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        };
        debug_assert!(answer.validate());
        Some(answer)
    }
}

#[derive(Debug, Clone)]
struct RelationPathCandidate<'a> {
    edges: Vec<&'a DialogueRelationEdgeIR>,
    truncated_by_hop_limit: bool,
}

fn relation_paths<'a>(
    query: &DialogueRelationQueryIR,
    graph: &'a DialogueRelationGraphIR,
) -> Vec<RelationPathCandidate<'a>> {
    let mut seeds = graph
        .relations
        .iter()
        .filter(|edge| edge.status.is_active())
        .filter(|edge| relation_kind_matches(query.kind, edge.kind))
        .filter(|edge| relation_topic_matches(query, edge))
        .collect::<Vec<_>>();
    seeds.sort_by(|left, right| {
        right
            .target_turn
            .cmp(&left.target_turn)
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    let mut paths = Vec::new();
    for seed in seeds {
        if paths.len() >= MAX_DIALOGUE_RELATION_PATHS {
            break;
        }
        match query.kind {
            DialogueRelationQueryKindIR::CauseOf => {
                extend_backward(graph, vec![seed], &mut paths);
            }
            DialogueRelationQueryKindIR::ConsequenceOf => {
                extend_forward(graph, vec![seed], &mut paths);
            }
            DialogueRelationQueryKindIR::ConcessionOutcome => {
                paths.push(RelationPathCandidate {
                    edges: vec![seed],
                    truncated_by_hop_limit: false,
                });
            }
        }
    }
    paths.sort_by_key(path_key);
    paths.dedup_by(|left, right| path_key(left) == path_key(right));
    paths.truncate(MAX_DIALOGUE_RELATION_PATHS);
    paths
}

fn extend_backward<'a>(
    graph: &'a DialogueRelationGraphIR,
    path: Vec<&'a DialogueRelationEdgeIR>,
    output: &mut Vec<RelationPathCandidate<'a>>,
) {
    if output.len() >= MAX_DIALOGUE_RELATION_PATHS {
        return;
    }
    let first = path.first().expect("non-empty relation path");
    let mut incoming = graph
        .relations
        .iter()
        .filter(|edge| edge.status.is_active())
        .filter(|edge| is_causal_edge(edge.kind))
        .filter(|edge| edge.target_referent_id == first.source_referent_id)
        .filter(|edge| !path.iter().any(|item| item.relation_id == edge.relation_id))
        .collect::<Vec<_>>();
    incoming.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    if incoming.is_empty() || path.len() >= MAX_DIALOGUE_RELATION_PATH_HOPS {
        output.push(RelationPathCandidate {
            edges: path,
            truncated_by_hop_limit: !incoming.is_empty(),
        });
        return;
    }
    for edge in incoming {
        let mut extended = path.clone();
        extended.insert(0, edge);
        extend_backward(graph, extended, output);
        if output.len() >= MAX_DIALOGUE_RELATION_PATHS {
            break;
        }
    }
}

fn extend_forward<'a>(
    graph: &'a DialogueRelationGraphIR,
    path: Vec<&'a DialogueRelationEdgeIR>,
    output: &mut Vec<RelationPathCandidate<'a>>,
) {
    if output.len() >= MAX_DIALOGUE_RELATION_PATHS {
        return;
    }
    let last = path.last().expect("non-empty relation path");
    let mut outgoing = graph
        .relations
        .iter()
        .filter(|edge| edge.status.is_active())
        .filter(|edge| is_causal_edge(edge.kind))
        .filter(|edge| edge.source_referent_id == last.target_referent_id)
        .filter(|edge| !path.iter().any(|item| item.relation_id == edge.relation_id))
        .collect::<Vec<_>>();
    outgoing.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    if outgoing.is_empty() || path.len() >= MAX_DIALOGUE_RELATION_PATH_HOPS {
        output.push(RelationPathCandidate {
            edges: path,
            truncated_by_hop_limit: !outgoing.is_empty(),
        });
        return;
    }
    for edge in outgoing {
        let mut extended = path.clone();
        extended.push(edge);
        extend_forward(graph, extended, output);
        if output.len() >= MAX_DIALOGUE_RELATION_PATHS {
            break;
        }
    }
}

fn path_ir(index: usize, candidate: &RelationPathCandidate<'_>) -> DialogueRelationPathIR {
    let first = candidate.edges.first().expect("non-empty relation path");
    let last = candidate.edges.last().expect("non-empty relation path");
    DialogueRelationPathIR {
        path_id: format!("DREL-PATH-{:02}", index + 1),
        relation_ids: candidate
            .edges
            .iter()
            .map(|edge| edge.relation_id.clone())
            .collect(),
        root_referent_id: first.source_referent_id.clone(),
        terminal_referent_id: last.target_referent_id.clone(),
        root_summary: first.source_summary.clone(),
        terminal_summary: last.target_summary.clone(),
        hop_count: candidate.edges.len(),
        confidence_millis: candidate
            .edges
            .iter()
            .map(|edge| edge.confidence_millis)
            .min()
            .unwrap_or_default(),
        contains_nonactual_world: candidate.edges.iter().any(|edge| {
            edge.source_modal_world != ModalWorldIR::Actual
                || edge.target_modal_world != ModalWorldIR::Actual
        }),
        contains_contested_endpoint: candidate.edges.iter().any(|edge| {
            edge.source_belief_status == BeliefRecordStatusIR::Contested
                || edge.target_belief_status == BeliefRecordStatusIR::Contested
        }),
        truncated_by_hop_limit: candidate.truncated_by_hop_limit,
        dialogue_claim_only: true,
        causal_truth_established: false,
        semantic_authority: false,
        external_execution_authorized: false,
    }
}

fn path_key(candidate: &RelationPathCandidate<'_>) -> String {
    candidate
        .edges
        .iter()
        .map(|edge| edge.relation_id.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn is_causal_edge(kind: DialogueRelationKindIR) -> bool {
    matches!(
        kind,
        DialogueRelationKindIR::Cause | DialogueRelationKindIR::Consequence
    )
}

fn parse_relation_query(text: &str) -> Option<DialogueRelationQueryIR> {
    let normalized = normalize_text(text);
    let kind = if contains_any(
        &normalized,
        &[
            "despite",
            "even though",
            "in spite",
            "불구",
            "그럼에도",
            "그래도",
        ],
    ) {
        DialogueRelationQueryKindIR::ConcessionOutcome
    } else if contains_any(
        &normalized,
        &[
            "what resulted",
            "what happened because",
            "what followed",
            "결과",
            "무슨 결과",
            "어떤 결과",
            "무슨 일이 생겼",
        ],
    ) {
        DialogueRelationQueryKindIR::ConsequenceOf
    } else if contains_any(
        &normalized,
        &[
            "why",
            "what caused",
            "reason for",
            "cause of",
            "왜",
            "원인",
            "이유",
        ],
    ) {
        DialogueRelationQueryKindIR::CauseOf
    } else {
        return None;
    };
    if !looks_like_question(text, &normalized) {
        return None;
    }
    Some(DialogueRelationQueryIR {
        original_text: text.trim().to_string(),
        kind,
        topic_terms: query_topic_terms(&normalized),
    })
}

fn relation_kind_matches(query: DialogueRelationQueryKindIR, edge: DialogueRelationKindIR) -> bool {
    match query {
        DialogueRelationQueryKindIR::CauseOf => {
            matches!(
                edge,
                DialogueRelationKindIR::Cause | DialogueRelationKindIR::Consequence
            )
        }
        DialogueRelationQueryKindIR::ConsequenceOf => {
            matches!(
                edge,
                DialogueRelationKindIR::Cause | DialogueRelationKindIR::Consequence
            )
        }
        DialogueRelationQueryKindIR::ConcessionOutcome => {
            edge == DialogueRelationKindIR::Concession
        }
    }
}

fn relation_topic_matches(query: &DialogueRelationQueryIR, edge: &DialogueRelationEdgeIR) -> bool {
    if query.topic_terms.is_empty() {
        return true;
    }
    let haystack = match query.kind {
        DialogueRelationQueryKindIR::CauseOf => &edge.target_summary,
        DialogueRelationQueryKindIR::ConsequenceOf
        | DialogueRelationQueryKindIR::ConcessionOutcome => &edge.source_summary,
    };
    let haystack_terms = content_terms(haystack);
    query.topic_terms.iter().all(|query_term| {
        haystack_terms.iter().any(|term| {
            term == query_term
                || (term.is_ascii() && query_term.is_ascii() && english_stem(term) == query_term)
                || (!term.is_ascii() && !query_term.is_ascii() && korean_stem(term) == *query_term)
        })
    })
}

fn query_topic_terms(text: &str) -> Vec<String> {
    let stop = [
        "what",
        "why",
        "is",
        "was",
        "were",
        "did",
        "do",
        "does",
        "the",
        "a",
        "an",
        "caused",
        "cause",
        "reason",
        "for",
        "of",
        "resulted",
        "happened",
        "because",
        "followed",
        "from",
        "despite",
        "even",
        "though",
        "in",
        "spite",
        "무엇",
        "뭐",
        "왜",
        "어떤",
        "무슨",
        "원인",
        "이유",
        "결과",
        "때문",
        "그럼에도",
        "불구하고",
        "인가",
        "했나",
        "했어",
        "생겼나",
        "생겼어",
    ];
    let mut terms = content_terms(text)
        .into_iter()
        .filter(|term| !stop.contains(&term.as_str()))
        .map(|term| {
            if term.is_ascii() {
                english_stem(&term).to_string()
            } else {
                korean_stem(&term)
            }
        })
        .filter(|term| !stop.contains(&term.as_str()))
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn content_terms(text: &str) -> Vec<String> {
    normalize_text(text)
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| !ch.is_alphanumeric())
                .to_string()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn english_stem(term: &str) -> &str {
    ["ing", "ed", "es", "s"]
        .into_iter()
        .find_map(|suffix| term.strip_suffix(suffix).filter(|stem| stem.len() >= 3))
        .unwrap_or(term)
}

fn korean_stem(term: &str) -> String {
    let predicate_suffixes = [
        "었다고",
        "았다고",
        "였다고",
        "한다고",
        "된다고",
        "다고",
        "라고",
        "했어",
        "됐어",
        "였어",
        "았어",
        "었어",
        "어요",
        "아요",
        "어",
        "아",
    ];
    if let Some(stem) = predicate_suffixes.into_iter().find_map(|suffix| {
        term.strip_suffix(suffix)
            .filter(|stem| stem.chars().count() >= 2)
    }) {
        return stem.to_string();
    }
    let suffixes = [
        "에서는",
        "으로",
        "에서",
        "에게",
        "까지",
        "부터",
        "보다",
        "처럼",
        "에도",
        "은",
        "는",
        "이",
        "가",
        "을",
        "를",
        "에",
        "의",
        "와",
        "과",
        "로",
        "도",
        "만",
    ];
    suffixes
        .into_iter()
        .find_map(|suffix| {
            term.strip_suffix(suffix)
                .filter(|stem| stem.chars().count() >= 2)
                .map(str::to_string)
        })
        .unwrap_or_else(|| term.to_string())
}

fn realize_relation_answer(
    language: LanguageCodeIR,
    query: &DialogueRelationQueryIR,
    disposition: DialogueRelationAnswerDispositionIR,
    evidence: &[DialogueRelationEvidenceIR],
    paths: &[DialogueRelationPathIR],
) -> String {
    if evidence.is_empty() {
        return match language {
            LanguageCodeIR::Korean => {
                "대화 기록에서 그 관계를 뒷받침하는 연결을 찾지 못했어. 추측해서 원인을 만들지는 않을게."
                    .to_string()
            }
            _ => "I found no matching relation in the dialogue record, so I will not invent one."
                .to_string(),
        };
    }
    let rows = paths
        .iter()
        .map(|path| {
            let mut nodes = vec![path.root_summary.clone()];
            for relation_id in &path.relation_ids {
                if let Some(edge) = evidence
                    .iter()
                    .find(|edge| &edge.relation_id == relation_id)
                {
                    nodes.push(edge.target_summary.clone());
                }
            }
            nodes
                .iter()
                .map(|node| format!("‘{node}’"))
                .collect::<Vec<_>>()
                .join(" → ")
        })
        .collect::<Vec<_>>()
        .join("; ");
    let plurality = if disposition == DialogueRelationAnswerDispositionIR::MultipleDialogueRelations
    {
        match language {
            LanguageCodeIR::Korean => "여러 연결이 기록돼 있어: ",
            _ => "The dialogue records multiple links: ",
        }
    } else {
        ""
    };
    let bounded_warning = if paths.iter().any(|path| path.truncated_by_hop_limit) {
        match language {
            LanguageCodeIR::Korean => " 경로가 안전 한도에서 잘려 더 먼 원인은 포함하지 않았어.",
            _ => " The path reached the safety limit, so more distant links are omitted.",
        }
    } else {
        ""
    };
    let world_warning = if paths.iter().any(|path| path.contains_nonactual_world) {
        match language {
            LanguageCodeIR::Korean => {
                " 비현실·가정 세계의 명제가 포함되어 실제 사건 경로로 볼 수 없어."
            }
            _ => " It contains a non-actual proposition and is not an actual-event path.",
        }
    } else {
        ""
    };
    let contested_warning = if paths.iter().any(|path| path.contains_contested_endpoint) {
        match language {
            LanguageCodeIR::Korean => " 또한 대화 안에서 다투어지는 명제가 포함돼 있어.",
            _ => " It also contains a proposition contested in the dialogue.",
        }
    } else {
        ""
    };
    match (language, query.kind) {
        (LanguageCodeIR::Korean, DialogueRelationQueryKindIR::CauseOf) => format!(
            "{plurality}대화에서는 {rows}를 이유 경로로 연결했어. 이는 대화상의 주장일 뿐, 실제 인과가 검증됐다는 뜻은 아니야.{bounded_warning}{world_warning}{contested_warning}"
        ),
        (LanguageCodeIR::Korean, DialogueRelationQueryKindIR::ConsequenceOf) => format!(
            "{plurality}대화에서는 {rows}를 결과 경로로 연결했어. 이는 대화 기록이며 실제 인과 검증은 아니야.{bounded_warning}{world_warning}{contested_warning}"
        ),
        (LanguageCodeIR::Korean, DialogueRelationQueryKindIR::ConcessionOutcome) => format!(
            "{plurality}대화에서는 {rows}를 어려움에도 성립한 결과로 연결했어. 두 명제를 보존할 뿐 사실 여부를 새로 확정하지는 않아.{world_warning}{contested_warning}"
        ),
        (_, DialogueRelationQueryKindIR::CauseOf) => format!(
            "{plurality}The dialogue links {rows} as a reason path. This records what was claimed; it does not verify actual causation.{bounded_warning}{world_warning}{contested_warning}"
        ),
        (_, DialogueRelationQueryKindIR::ConsequenceOf) => format!(
            "{plurality}The dialogue links {rows} as a result path. This is a dialogue record, not verified causation.{bounded_warning}{world_warning}{contested_warning}"
        ),
        (_, DialogueRelationQueryKindIR::ConcessionOutcome) => format!(
            "{plurality}The dialogue links {rows} as an outcome that held despite the prior proposition. It preserves both claims without establishing either as fact.{world_warning}{contested_warning}"
        ),
    }
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn looks_like_question(original: &str, normalized: &str) -> bool {
    original.trim_end().ends_with('?')
        || contains_any(
            normalized,
            &[
                "what ",
                "why ",
                "did ",
                "does ",
                "is there",
                "무엇",
                "뭐",
                "왜",
                "무슨",
                "어떤",
                "원인은",
                "이유는",
                "결과는",
                "인가",
                "했나",
                "생겼나",
            ],
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribution::{
        AttributedPropositionPolarityIR, AttributionAttitudeIR, EpistemicStatusIR,
    };
    use crate::epistemic::EpistemicObservationIR;
    use crate::modality::ModalWorldIR;

    fn proposition(id: &str, turn: u64, summary: &str) -> DynamicDiscourseReferentIR {
        DynamicDiscourseReferentIR {
            referent_id: id.to_string(),
            kind: DiscourseReferentKindIR::Proposition,
            topic_id: None,
            semantic_summary: summary.to_string(),
            attributed_source: None,
            attribution_attitude: None,
            epistemic_status: None,
            proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
            modal_world: Some(ModalWorldIR::Actual),
            belief_record_id: Some(format!("BELIEF-{id}")),
            introduced_turn: turn,
            last_referenced_turn: turn,
            external_execution_authorized: false,
        }
    }

    fn observation(id: &str, summary: &str) -> EpistemicObservationIR {
        EpistemicObservationIR {
            origin_referent_id: id.to_string(),
            source_actor: "dialogue-user".to_string(),
            proposition_surface: summary.to_string(),
            proposition_polarity: AttributedPropositionPolarityIR::Positive,
            modal_world: ModalWorldIR::Actual,
            attribution_attitude: AttributionAttitudeIR::Say,
            epistemic_status: EpistemicStatusIR::Reported,
        }
    }

    fn ledger_proposition(
        id: &str,
        belief_id: &str,
        turn: u64,
        summary: &str,
    ) -> DynamicDiscourseReferentIR {
        let mut referent = proposition(id, turn, summary);
        referent.belief_record_id = Some(belief_id.to_string());
        referent
    }

    #[test]
    fn relation_graph_records_claim_without_causal_authority() {
        let prior = vec![proposition("P1", 1, "the cache is corrupt")];
        let current = vec![proposition("P2", 2, "the service is slow")];
        let mut graph = DialogueRelationGraphIR::default();
        graph.apply_turn(
            2,
            "Because of that, the service is slow.",
            &["P1".to_string()],
            &prior,
            &current,
        );
        assert!(graph.validate(2));
        assert_eq!(graph.relations.len(), 1);
        let edge = &graph.relations[0];
        assert_eq!(edge.kind, DialogueRelationKindIR::Cause);
        assert!(!edge.causal_truth_established);
        assert!(!edge.semantic_authority);
        assert!(!edge.external_execution_authorized);
    }

    #[test]
    fn same_turn_multiple_antecedents_fail_closed() {
        let prior = vec![
            proposition("P1", 1, "the cache is corrupt"),
            proposition("P2", 1, "the network is congested"),
        ];
        let resolution = resolve_relation_antecedent(&prior, 1, "Therefore, latency rose.");
        assert!(resolution.detected);
        assert!(resolution.referent_ids.is_empty());
        assert_eq!(resolution.ambiguous_surfaces.len(), 1);
    }

    #[test]
    fn quoted_connector_does_not_create_a_relation() {
        let prior = vec![proposition("P1", 1, "the cache is corrupt")];
        let resolution = resolve_relation_antecedent(
            &prior,
            1,
            "‘Because of that, the service is slow’ is a quote.",
        );
        assert!(!resolution.detected);
    }

    #[test]
    fn causal_query_returns_a_two_hop_path_without_semantic_authority() {
        let p1 = proposition("P1", 1, "Atlas cache failure");
        let p2 = proposition("P2", 2, "Atlas service latency increase");
        let p3 = proposition("P3", 3, "Atlas request queue growth");
        let mut graph = DialogueRelationGraphIR::default();
        graph.apply_turn(
            2,
            "Because of that, Atlas service latency increase.",
            &["P1".to_string()],
            std::slice::from_ref(&p1),
            std::slice::from_ref(&p2),
        );
        graph.apply_turn(
            3,
            "Therefore, Atlas request queue growth.",
            &["P2".to_string()],
            &[p1, p2],
            std::slice::from_ref(&p3),
        );

        let answer = DialogueRelationQaEngine
            .answer(
                "Why Atlas request queue growth?",
                Some(&graph),
                LanguageCodeIR::English,
            )
            .expect("relation query");
        assert!(answer.validate());
        assert_eq!(
            answer.disposition,
            DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath
        );
        assert_eq!(answer.paths.len(), 1);
        assert_eq!(answer.paths[0].hop_count, 2);
        assert!(!answer.paths[0].semantic_authority);
        assert!(!answer.paths[0].external_execution_authorized);
    }

    #[test]
    fn reaffirmed_target_preserves_multiple_independent_reason_paths() {
        let mut ledger = EpistemicLedgerIR::default();
        let b1 = ledger.apply_turn(
            1,
            "Maroon cache failure",
            &[],
            &[observation("P1", "Maroon cache failure")],
        )[0]
        .1
        .clone();
        let b2 = ledger.apply_turn(
            2,
            "Maroon latency increase",
            &[],
            &[observation("P2", "Maroon latency increase")],
        )[0]
        .1
        .clone();
        let p1 = ledger_proposition("P1", &b1, 1, "Maroon cache failure");
        let p2 = ledger_proposition("P2", &b2, 2, "Maroon latency increase");
        let mut graph = DialogueRelationGraphIR::default();
        graph.apply_turn(
            2,
            "Because of that, Maroon latency increase",
            &["P1".to_string()],
            std::slice::from_ref(&p1),
            std::slice::from_ref(&p2),
        );

        let b3 = ledger.apply_turn(
            3,
            "Maroon network congestion",
            &[],
            &[observation("P3", "Maroon network congestion")],
        )[0]
        .1
        .clone();
        let b4 = ledger.apply_turn(
            4,
            "Maroon latency increase",
            &[],
            &[observation("P4", "Maroon latency increase")],
        )[0]
        .1
        .clone();
        let p3 = ledger_proposition("P3", &b3, 3, "Maroon network congestion");
        let p4 = ledger_proposition("P4", &b4, 4, "Maroon latency increase");
        graph.apply_turn(
            4,
            "Because of that, Maroon latency increase",
            &["P3".to_string()],
            &[p1, p2, p3.clone()],
            std::slice::from_ref(&p4),
        );
        graph.synchronize_with_ledger(4, &ledger);

        assert!(graph.validate_with_ledger(4, &ledger));
        assert_eq!(graph.relations.len(), 2);
        assert!(graph.relations.iter().all(|edge| edge.status.is_active()));
        let answer = DialogueRelationQaEngine
            .answer(
                "Why Maroon latency increase?",
                Some(&graph),
                LanguageCodeIR::English,
            )
            .expect("relation query");
        assert_eq!(
            answer.disposition,
            DialogueRelationAnswerDispositionIR::MultipleDialogueRelations
        );
        assert_eq!(answer.paths.len(), 2);
        assert_eq!(answer.evidence.len(), 2);
        assert!(!answer.dialogue_truth_established);
        assert!(!answer.external_execution_authorized);
    }

    #[test]
    fn already_stemmed_query_term_is_not_stemmed_twice() {
        let p1 = proposition("P1", 1, "Flint migration high cost");
        let p2 = proposition("P2", 2, "Flint rollout continued");
        let p3 = proposition("P3", 3, "Flint readiness growth");
        let mut graph = DialogueRelationGraphIR::default();
        graph.apply_turn(
            2,
            "Even so, Flint rollout continued.",
            &["P1".to_string()],
            std::slice::from_ref(&p1),
            std::slice::from_ref(&p2),
        );
        graph.apply_turn(
            3,
            "Therefore, Flint readiness growth.",
            &["P2".to_string()],
            &[p1, p2],
            std::slice::from_ref(&p3),
        );

        let answer = DialogueRelationQaEngine
            .answer(
                "Why Flint readiness growth?",
                Some(&graph),
                LanguageCodeIR::English,
            )
            .expect("relation query");
        assert_eq!(
            answer.disposition,
            DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation
        );
        assert_eq!(answer.paths[0].hop_count, 1);
        assert_eq!(answer.evidence.len(), 1);
        assert_eq!(answer.evidence[0].kind, DialogueRelationKindIR::Consequence);
    }

    #[test]
    fn embedded_same_turn_relation_is_dialogue_only() {
        let source = proposition("P1", 1, "the cache failed");
        let target = proposition("P2", 1, "the worker is blocked");
        let mut graph = DialogueRelationGraphIR::default();
        graph.apply_turn(
            1,
            "Nora believes the cache failed. Because of that, Nora says the worker is blocked.",
            &[],
            &[],
            &[source, target],
        );

        assert!(graph.validate(1));
        assert_eq!(graph.relations.len(), 1);
        let edge = &graph.relations[0];
        assert_eq!(edge.kind, DialogueRelationKindIR::Cause);
        assert_eq!(edge.source_referent_id, "P1");
        assert_eq!(edge.target_referent_id, "P2");
        assert!(!edge.causal_truth_established);
        assert!(!edge.semantic_authority);
        assert!(!edge.external_execution_authorized);
    }
}
