//! Hash-bound, dialogue-local discourse centering.
//!
//! Focus is a reference prior derived from typed clause/proposition structure.
//! It never establishes truth, mutates semantic concepts, or grants execution
//! authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clause_graph::{ClauseFunctionIR, ClauseRelationKindIR};

pub const DISCOURSE_FOCUS_STATE_SCHEMA: &str = "B_CORE_DISCOURSE_FOCUS_STATE_IR_1";
pub const MAX_DISCOURSE_FOCUS_NODES: usize = 16;
pub const MAX_DISCOURSE_FOCUS_TRANSITIONS: usize = 32;
pub const MAX_DISCOURSE_FOCUS_TURN_DISTANCE: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseFocusStatusIR {
    Primary,
    Secondary,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseFocusSourceIR {
    GroundedGoal,
    DeferredGoal,
    ActivatedGoal,
    Proposition,
    ExplicitTopic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseFocusTransitionKindIR {
    Continue,
    Shift,
    ExplicitReturn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseFocusNodeIR {
    pub focus_id: String,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id_hint: Option<String>,
    pub source: DiscourseFocusSourceIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_frame_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_clause_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_function: Option<ClauseFunctionIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governing_relation: Option<ClauseRelationKindIR>,
    pub status: DiscourseFocusStatusIR,
    pub salience_millis: u16,
    pub introduced_turn: u64,
    pub last_focused_turn: u64,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseFocusTransitionIR {
    pub transition_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_focus_id: Option<String>,
    pub resulting_focus_id: String,
    pub kind: DiscourseFocusTransitionKindIR,
    pub turn_index: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_relation: Option<ClauseRelationKindIR>,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscourseFocusCandidateIR {
    pub surface: String,
    pub concept_id_hint: Option<String>,
    pub source: DiscourseFocusSourceIR,
    pub source_frame_id: Option<String>,
    pub source_clause_id: Option<String>,
    pub clause_function: Option<ClauseFunctionIR>,
    pub governing_relation: Option<ClauseRelationKindIR>,
    pub salience_millis: u16,
    pub source_order: usize,
    pub evidence: Vec<String>,
}

impl DiscourseFocusCandidateIR {
    pub fn explicit_topic(surface: &str, concept_id_hint: Option<&str>) -> Self {
        Self {
            surface: surface.trim().to_string(),
            concept_id_hint: concept_id_hint.map(ToString::to_string),
            source: DiscourseFocusSourceIR::ExplicitTopic,
            source_frame_id: None,
            source_clause_id: None,
            clause_function: None,
            governing_relation: None,
            salience_millis: 1_000,
            source_order: 0,
            evidence: vec!["EXPLICIT_TOPIC_MANAGEMENT".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseFocusStateIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_focus_id: Option<String>,
    pub nodes: Vec<DiscourseFocusNodeIR>,
    pub transitions: Vec<DiscourseFocusTransitionIR>,
}

impl Default for DiscourseFocusStateIR {
    fn default() -> Self {
        Self {
            schema: DISCOURSE_FOCUS_STATE_SCHEMA.to_string(),
            current_focus_id: None,
            nodes: Vec::new(),
            transitions: Vec::new(),
        }
    }
}

impl DiscourseFocusStateIR {
    pub fn current(&self) -> Option<&DiscourseFocusNodeIR> {
        let current = self.current_focus_id.as_deref()?;
        self.nodes.iter().find(|node| node.focus_id == current)
    }

    pub fn apply_turn(&mut self, turn_index: u64, candidates: &[DiscourseFocusCandidateIR]) {
        let mut distinct = BTreeMap::<String, &DiscourseFocusCandidateIR>::new();
        for candidate in candidates.iter().filter(|candidate| {
            !candidate.surface.trim().is_empty()
                && candidate.salience_millis <= 1_000
                && candidate.salience_millis > 0
        }) {
            let identity = focus_identity(&candidate.surface, candidate.concept_id_hint.as_deref());
            distinct
                .entry(identity)
                .and_modify(|existing| {
                    if candidate
                        .salience_millis
                        .cmp(&existing.salience_millis)
                        .then_with(|| candidate.source_order.cmp(&existing.source_order))
                        .is_gt()
                    {
                        *existing = candidate;
                    }
                })
                .or_insert(candidate);
        }
        if distinct.is_empty() {
            return;
        }
        let selected_identity = distinct
            .iter()
            .max_by(|(left_identity, left), (right_identity, right)| {
                left.salience_millis
                    .cmp(&right.salience_millis)
                    .then_with(|| left.source_order.cmp(&right.source_order))
                    .then_with(|| right_identity.cmp(left_identity))
            })
            .map(|(identity, _)| identity.clone())
            .expect("non-empty focus candidates");
        let prior_focus_id = self.current_focus_id.clone();
        for node in &mut self.nodes {
            node.status = DiscourseFocusStatusIR::Background;
        }
        let mut selected_focus_id = String::new();
        let mut selected_relation = None;
        let mut selected_evidence = Vec::new();
        for (identity, candidate) in distinct {
            let focus_id = focus_id(&identity);
            let selected = identity == selected_identity;
            let status = if selected {
                DiscourseFocusStatusIR::Primary
            } else {
                DiscourseFocusStatusIR::Secondary
            };
            let introduced_turn = self
                .nodes
                .iter()
                .find(|node| node.focus_id == focus_id)
                .map_or(turn_index, |node| node.introduced_turn);
            self.nodes.retain(|node| node.focus_id != focus_id);
            self.nodes.push(DiscourseFocusNodeIR {
                focus_id: focus_id.clone(),
                surface: candidate.surface.trim().to_string(),
                concept_id_hint: candidate.concept_id_hint.clone(),
                source: candidate.source,
                source_frame_id: candidate.source_frame_id.clone(),
                source_clause_id: candidate.source_clause_id.clone(),
                clause_function: candidate.clause_function,
                governing_relation: candidate.governing_relation,
                status,
                salience_millis: candidate.salience_millis,
                introduced_turn,
                last_focused_turn: turn_index,
                semantic_authority: false,
                external_execution_authorized: false,
            });
            if selected {
                selected_focus_id = focus_id;
                selected_relation = candidate.governing_relation;
                selected_evidence = candidate.evidence.clone();
            }
        }
        self.current_focus_id = Some(selected_focus_id.clone());
        let kind = if prior_focus_id.as_deref() == Some(selected_focus_id.as_str()) {
            DiscourseFocusTransitionKindIR::Continue
        } else if candidates
            .iter()
            .any(|candidate| candidate.source == DiscourseFocusSourceIR::ExplicitTopic)
        {
            DiscourseFocusTransitionKindIR::ExplicitReturn
        } else {
            DiscourseFocusTransitionKindIR::Shift
        };
        selected_evidence.push("SEMANTIC_AUTHORITY:false".to_string());
        selected_evidence.push("EXTERNAL_EXECUTION_AUTHORIZED:false".to_string());
        selected_evidence.sort();
        selected_evidence.dedup();
        self.transitions.push(DiscourseFocusTransitionIR {
            transition_id: format!(
                "FOCUS-TRANSITION-{turn_index:06}-{:02}",
                self.transitions.len() + 1
            ),
            prior_focus_id,
            resulting_focus_id: selected_focus_id,
            kind,
            turn_index,
            clause_relation: selected_relation,
            evidence: selected_evidence,
            semantic_authority: false,
            external_execution_authorized: false,
        });
        self.nodes.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| right.last_focused_turn.cmp(&left.last_focused_turn))
                .then_with(|| right.salience_millis.cmp(&left.salience_millis))
                .then_with(|| left.focus_id.cmp(&right.focus_id))
        });
        self.nodes.truncate(MAX_DISCOURSE_FOCUS_NODES);
        let live_ids = self
            .nodes
            .iter()
            .map(|node| node.focus_id.as_str())
            .collect::<BTreeSet<_>>();
        self.transitions.retain(|transition| {
            live_ids.contains(transition.resulting_focus_id.as_str())
                && transition
                    .prior_focus_id
                    .as_deref()
                    .is_none_or(|prior| live_ids.contains(prior))
        });
        if self.transitions.len() > MAX_DISCOURSE_FOCUS_TRANSITIONS {
            let excess = self.transitions.len() - MAX_DISCOURSE_FOCUS_TRANSITIONS;
            self.transitions.drain(..excess);
        }
        debug_assert!(self.validate(turn_index));
    }

    /// Restores an already typed focus node after an explicit topic resume.
    /// The node must still be live; missing snapshots fail closed.
    pub fn restore_topic_focus(
        &mut self,
        turn_index: u64,
        focus_id: &str,
        evidence: &[String],
    ) -> bool {
        let Some(selected_index) = self.nodes.iter().position(|node| node.focus_id == focus_id)
        else {
            return false;
        };
        let prior_focus_id = self.current_focus_id.clone();
        for node in &mut self.nodes {
            node.status = DiscourseFocusStatusIR::Background;
        }
        let selected = &mut self.nodes[selected_index];
        selected.status = DiscourseFocusStatusIR::Primary;
        selected.last_focused_turn = turn_index;
        self.current_focus_id = Some(focus_id.to_string());
        let mut transition_evidence = evidence.to_vec();
        transition_evidence.push("TOPIC_CONTEXT_FOCUS_RESTORED:true".to_string());
        transition_evidence.push("SEMANTIC_AUTHORITY:false".to_string());
        transition_evidence.push("EXTERNAL_EXECUTION_AUTHORIZED:false".to_string());
        transition_evidence.sort();
        transition_evidence.dedup();
        self.transitions.push(DiscourseFocusTransitionIR {
            transition_id: format!(
                "FOCUS-TRANSITION-{turn_index:06}-{:02}",
                self.transitions.len() + 1
            ),
            prior_focus_id,
            resulting_focus_id: focus_id.to_string(),
            kind: DiscourseFocusTransitionKindIR::ExplicitReturn,
            turn_index,
            clause_relation: selected.governing_relation,
            evidence: transition_evidence,
            semantic_authority: false,
            external_execution_authorized: false,
        });
        self.nodes.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| right.last_focused_turn.cmp(&left.last_focused_turn))
                .then_with(|| right.salience_millis.cmp(&left.salience_millis))
                .then_with(|| left.focus_id.cmp(&right.focus_id))
        });
        if self.transitions.len() > MAX_DISCOURSE_FOCUS_TRANSITIONS {
            let excess = self.transitions.len() - MAX_DISCOURSE_FOCUS_TRANSITIONS;
            self.transitions.drain(..excess);
        }
        debug_assert!(self.validate(turn_index));
        true
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        if self.schema != DISCOURSE_FOCUS_STATE_SCHEMA
            || self.nodes.len() > MAX_DISCOURSE_FOCUS_NODES
            || self.transitions.len() > MAX_DISCOURSE_FOCUS_TRANSITIONS
        {
            return false;
        }
        let node_ids = self
            .nodes
            .iter()
            .map(|node| node.focus_id.as_str())
            .collect::<BTreeSet<_>>();
        let transition_ids = self
            .transitions
            .iter()
            .map(|transition| transition.transition_id.as_str())
            .collect::<BTreeSet<_>>();
        let primary = self
            .nodes
            .iter()
            .filter(|node| node.status == DiscourseFocusStatusIR::Primary)
            .collect::<Vec<_>>();
        if node_ids.len() != self.nodes.len()
            || transition_ids.len() != self.transitions.len()
            || primary.len() > 1
            || self.current_focus_id.is_some() != (primary.len() == 1)
            || self.current_focus_id.as_deref()
                != primary.first().map(|node| node.focus_id.as_str())
        {
            return false;
        }
        self.nodes.iter().all(|node| {
            !node.focus_id.trim().is_empty()
                && !node.surface.trim().is_empty()
                && node.salience_millis > 0
                && node.salience_millis <= 1_000
                && node.introduced_turn > 0
                && node.last_focused_turn >= node.introduced_turn
                && node.last_focused_turn <= completed_turns
                && !node.semantic_authority
                && !node.external_execution_authorized
        }) && self.transitions.iter().all(|transition| {
            !transition.transition_id.trim().is_empty()
                && node_ids.contains(transition.resulting_focus_id.as_str())
                && transition
                    .prior_focus_id
                    .as_deref()
                    .is_none_or(|prior| node_ids.contains(prior))
                && transition.turn_index > 0
                && transition.turn_index <= completed_turns
                && !transition.evidence.is_empty()
                && !transition.semantic_authority
                && !transition.external_execution_authorized
        })
    }
}

fn focus_identity(surface: &str, concept_id_hint: Option<&str>) -> String {
    concept_id_hint.map_or_else(
        || {
            surface
                .to_lowercase()
                .split(|character: char| !character.is_alphanumeric() && character != '_')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("_")
        },
        |concept| concept.to_string(),
    )
}

fn focus_id(identity: &str) -> String {
    let digest = Sha256::digest(identity.as_bytes());
    format!(
        "FOCUS-{:02X}{:02X}{:02X}{:02X}",
        digest[0], digest[1], digest[2], digest[3]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(surface: &str, score: u16, order: usize) -> DiscourseFocusCandidateIR {
        DiscourseFocusCandidateIR {
            surface: surface.to_string(),
            concept_id_hint: None,
            source: DiscourseFocusSourceIR::GroundedGoal,
            source_frame_id: Some(format!("FRAME-{order}")),
            source_clause_id: Some(format!("CLAUSE-{order}")),
            clause_function: Some(ClauseFunctionIR::Coordinate),
            governing_relation: Some(ClauseRelationKindIR::Sequence),
            salience_millis: score,
            source_order: order,
            evidence: vec!["CLAUSE_GRAPH_CENTERING".to_string()],
        }
    }

    #[test]
    fn one_primary_center_is_hash_bound_and_non_authoritative() {
        let mut state = DiscourseFocusStateIR::default();
        state.apply_turn(1, &[candidate("cache", 850, 0), candidate("queue", 930, 1)]);
        assert!(state.validate(1));
        let current = state.current().expect("current focus");
        assert_eq!(current.surface, "queue");
        assert!(!current.semantic_authority);
        assert!(!current.external_execution_authorized);
    }

    #[test]
    fn empty_social_turn_preserves_the_prior_center() {
        let mut state = DiscourseFocusStateIR::default();
        state.apply_turn(1, &[candidate("queue", 930, 0)]);
        let before = state.current_focus_id.clone();
        state.apply_turn(2, &[]);
        assert_eq!(state.current_focus_id, before);
        assert!(state.validate(2));
    }
}
