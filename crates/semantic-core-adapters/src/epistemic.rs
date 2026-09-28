//! Bounded, temporally versioned epistemic ledger for conversation state.
//!
//! The ledger records what a dialogue participant or attributed source was
//! represented as asserting. It is not a world-fact database. No ledger record
//! establishes truth or grants execution authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::attribution::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, EpistemicStatusIR,
};
use crate::modality::ModalWorldIR;

pub const EPISTEMIC_LEDGER_SCHEMA: &str = "B_CORE_EPISTEMIC_LEDGER_IR_3";
const MAX_BELIEF_RECORDS: usize = 64;
const MAX_BELIEF_REVISIONS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SemanticStateValueIR {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemporalAnchorIR {
    Unspecified,
    Past,
    Present,
    Future,
}

impl SemanticStateValueIR {
    fn inverted(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionSignatureIR {
    pub subject_key: String,
    pub temporal_anchor: TemporalAnchorIR,
    #[serde(default)]
    pub modal_world: ModalWorldIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_axis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_value: Option<SemanticStateValueIR>,
    pub normalized_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BeliefRecordStatusIR {
    Active,
    Contested,
    Superseded,
    Retracted,
}

impl BeliefRecordStatusIR {
    pub fn is_reference_active(self) -> bool {
        matches!(self, Self::Active | Self::Contested)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefRecordIR {
    #[serde(default)]
    pub content: crate::proposition_content::PropositionContentIR,
    pub belief_id: String,
    pub origin_referent_id: String,
    pub source_actor: String,
    pub proposition_surface: String,
    pub proposition_polarity: AttributedPropositionPolarityIR,
    pub signature: PropositionSignatureIR,
    pub attribution_attitude: AttributionAttitudeIR,
    pub epistemic_status: EpistemicStatusIR,
    pub status: BeliefRecordStatusIR,
    pub introduced_turn: u64,
    pub last_updated_turn: u64,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BeliefRevisionKindIR {
    Contradicts,
    Supersedes,
    Reaffirms,
    Retracts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefRevisionIR {
    pub revision_id: String,
    pub kind: BeliefRevisionKindIR,
    pub prior_belief_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_belief_id: Option<String>,
    pub turn_index: u64,
    pub evidence_surface: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicObservationIR {
    pub origin_referent_id: String,
    pub source_actor: String,
    pub proposition_surface: String,
    pub proposition_polarity: AttributedPropositionPolarityIR,
    #[serde(default)]
    pub modal_world: ModalWorldIR,
    pub attribution_attitude: AttributionAttitudeIR,
    pub epistemic_status: EpistemicStatusIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicLedgerIR {
    pub schema: String,
    pub records: Vec<BeliefRecordIR>,
    pub revisions: Vec<BeliefRevisionIR>,
    pub unresolved_conflicts: Vec<String>,
}

impl Default for EpistemicLedgerIR {
    fn default() -> Self {
        Self {
            schema: EPISTEMIC_LEDGER_SCHEMA.to_string(),
            records: Vec::new(),
            revisions: Vec::new(),
            unresolved_conflicts: Vec::new(),
        }
    }
}

impl EpistemicLedgerIR {
    pub fn record(&self, belief_id: &str) -> Option<&BeliefRecordIR> {
        self.records
            .iter()
            .find(|record| record.belief_id == belief_id)
    }

    pub fn active_record_for_referent(&self, referent_id: &str) -> Option<&BeliefRecordIR> {
        self.records.iter().find(|record| {
            record.origin_referent_id == referent_id && record.status.is_reference_active()
        })
    }

    pub fn apply_turn(
        &mut self,
        turn_index: u64,
        turn_surface: &str,
        referenced_referent_ids: &[String],
        observations: &[EpistemicObservationIR],
    ) -> Vec<(String, String)> {
        self.apply_turn_with_event_focus(
            turn_index,
            turn_surface,
            referenced_referent_ids,
            observations,
            None,
        )
    }

    pub(crate) fn apply_turn_with_event_focus(
        &mut self,
        turn_index: u64,
        turn_surface: &str,
        referenced_referent_ids: &[String],
        observations: &[EpistemicObservationIR],
        event_focus: Option<&str>,
    ) -> Vec<(String, String)> {
        let compositional_analysis =
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(turn_surface);
        self.apply_turn_with_event_focus_and_compositional_analysis(
            turn_index,
            turn_surface,
            referenced_referent_ids,
            observations,
            event_focus,
            &compositional_analysis,
        )
    }

    /// Applies a turn using analysis of this exact source surface supplied by
    /// the caller.  Conversation commit uses this to share one immutable
    /// source analysis between the response-prohibition boundary and its
    /// source-level causal-observation admission check.  Callers must never
    /// substitute a reference-resolved or otherwise rewritten surface here.
    pub(crate) fn apply_turn_with_event_focus_and_compositional_analysis(
        &mut self,
        turn_index: u64,
        turn_surface: &str,
        referenced_referent_ids: &[String],
        observations: &[EpistemicObservationIR],
        event_focus: Option<&str>,
        compositional_analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    ) -> Vec<(String, String)> {
        self.apply_turn_with_event_focus_and_source_analysis(
            turn_index,
            turn_surface,
            referenced_referent_ids,
            observations,
            event_focus,
            compositional_analysis,
            None,
        )
    }

    /// Like `apply_turn_with_event_focus_and_compositional_analysis`, with an
    /// optional content record compiled from exactly `turn_surface`.  The
    /// trusted record is used only for an observation whose surface exactly
    /// equals the turn; final conversation-state validation still recompiles
    /// every persisted record from its source before publishing the state.
    pub(crate) fn apply_turn_with_event_focus_and_source_analysis(
        &mut self,
        turn_index: u64,
        turn_surface: &str,
        referenced_referent_ids: &[String],
        observations: &[EpistemicObservationIR],
        event_focus: Option<&str>,
        compositional_analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
        trusted_turn_content: Option<&crate::proposition_content::PropositionContentIR>,
    ) -> Vec<(String, String)> {
        // A scoped response prohibition changes what may be said, not what
        // happened. Keep it in the turn/directive history; do not let legacy
        // proposition extraction turn its complement into a belief revision.
        if crate::discourse_qa::is_response_operation_batch(turn_surface)
            || !crate::conversation_contract::response_prohibition(
                turn_surface,
                compositional_analysis,
            )
            .is_empty()
        {
            return Vec::new();
        }
        if is_retraction_surface(turn_surface) {
            self.retract_referenced(turn_index, turn_surface, referenced_referent_ids);
        }
        let turn_revision = is_revision_surface(turn_surface)
            || crate::proposition_content::is_event_correction_surface(turn_surface);
        let mut bindings = Vec::new();
        let turn_record_floor = self.records.len();
        for (index, observation) in observations.iter().enumerate() {
            // A turn may contain a preface, a correction and unrelated facts.
            // Revision force belongs to its observation, not every sentence in
            // the turn. Preserve whole-turn attribution compatibility only for
            // a single extracted proposition (e.g. "Alice now says ...").
            let explicit_revision = is_revision_surface(&observation.proposition_surface)
                || crate::proposition_content::is_event_correction_surface(
                    &observation.proposition_surface,
                )
                || (observations.len() == 1 && turn_revision);
            let belief_id = format!("BELIEF-{turn_index:06}-{:02}", index + 1);
            let mut content = trusted_turn_content
                .filter(|_| observation.proposition_surface == turn_surface)
                .cloned()
                .unwrap_or_else(|| {
                    crate::proposition_content::PropositionContentIR::compile(
                        &observation.proposition_surface,
                    )
                });
            let needs_context =
                content.events.iter().any(|e| e.has_references()) || explicit_revision;
            let mut event_revision_target = None;
            let event_correction_requested = explicit_revision
                && crate::proposition_content::is_event_report(&observation.proposition_surface);
            if needs_context && observation.modal_world == ModalWorldIR::Actual {
                let candidates = self
                    .records
                    .iter()
                    .enumerate()
                    .filter(|(record_index, r)| {
                        r.status == BeliefRecordStatusIR::Active
                            && r.signature.modal_world == ModalWorldIR::Actual
                            && normalized_source(&r.source_actor)
                                == normalized_source(&observation.source_actor)
                            && (r.introduced_turn < turn_index
                                || (*record_index >= turn_record_floor
                                    && *record_index < self.records.len()))
                            && (explicit_revision
                                || r.introduced_turn + 1 == turn_index
                                || event_focus == Some(r.belief_id.as_str())
                                || (*record_index >= turn_record_floor
                                    && *record_index < self.records.len()))
                            && r.content.validate_source(&r.proposition_surface)
                            && r.content.events.len() == 1
                    })
                    .filter_map(|(_, r)| {
                        let mut sources = r.content.context_sources.clone();
                        sources.push(crate::proposition_content::EventSourceIR {
                            belief_id: r.belief_id.clone(),
                            source_actor: r.source_actor.clone(),
                            source_proposition: r.proposition_surface.clone(),
                        });
                        let compiled =
                            crate::proposition_content::PropositionContentIR::compile_contextual(
                                &observation.proposition_surface,
                                &sources,
                            )?;
                        Some((r.belief_id.clone(), compiled))
                    })
                    .collect::<Vec<_>>();
                if candidates.len() == 1 {
                    let (id, compiled) = candidates.into_iter().next().unwrap();
                    content = compiled;
                    if explicit_revision {
                        event_revision_target = Some(id);
                    }
                }
            }
            let signature = proposition_signature_in_world(
                &observation.proposition_surface,
                observation.proposition_polarity,
                observation.modal_world,
            );
            let mut new_status = BeliefRecordStatusIR::Active;
            let comparable = self
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| record.status.is_reference_active())
                .filter(|(_, record)| signatures_comparable(&record.signature, &signature))
                .map(|(record_index, _)| record_index)
                .collect::<Vec<_>>();
            let mut had_same_source_match = false;
            for record_index in comparable {
                let same_source = normalized_source(&self.records[record_index].source_actor)
                    == normalized_source(&observation.source_actor);
                let equivalent =
                    signatures_equivalent(&self.records[record_index].signature, &signature);
                let contradictory =
                    signatures_contradict(&self.records[record_index].signature, &signature);
                if same_source && (equivalent || contradictory) {
                    had_same_source_match = true;
                }
                let prior_id = self.records[record_index].belief_id.clone();
                if equivalent && same_source {
                    self.records[record_index].status = BeliefRecordStatusIR::Superseded;
                    self.records[record_index].last_updated_turn = turn_index;
                    self.push_revision(
                        BeliefRevisionKindIR::Reaffirms,
                        &prior_id,
                        Some(&belief_id),
                        turn_index,
                        turn_surface,
                    );
                } else if contradictory {
                    self.push_revision(
                        BeliefRevisionKindIR::Contradicts,
                        &prior_id,
                        Some(&belief_id),
                        turn_index,
                        turn_surface,
                    );
                    if same_source
                        && (explicit_revision
                            || observation.attribution_attitude == AttributionAttitudeIR::Correct)
                    {
                        self.records[record_index].status = BeliefRecordStatusIR::Superseded;
                        self.records[record_index].last_updated_turn = turn_index;
                        self.push_revision(
                            BeliefRevisionKindIR::Supersedes,
                            &prior_id,
                            Some(&belief_id),
                            turn_index,
                            turn_surface,
                        );
                    } else {
                        self.records[record_index].status = BeliefRecordStatusIR::Contested;
                        self.records[record_index].last_updated_turn = turn_index;
                        new_status = BeliefRecordStatusIR::Contested;
                        self.unresolved_conflicts.push(format!(
                            "{}<->{}:{}",
                            prior_id, belief_id, signature.subject_key
                        ));
                    }
                }
            }
            if (explicit_revision
                || observation.attribution_attitude == AttributionAttitudeIR::Correct)
                && !had_same_source_match
            {
                if let Some(record_index) = event_revision_target
                    .as_ref()
                    .and_then(|id| self.records.iter().position(|r| &r.belief_id == id))
                    .or_else(|| {
                        (!event_correction_requested)
                            .then(|| {
                                self.latest_active_source_record_for_subject(
                                    &observation.source_actor,
                                    &signature,
                                )
                                .or_else(|| {
                                    self.latest_active_source_record(&observation.source_actor)
                                })
                            })
                            .flatten()
                    })
                {
                    let prior_id = self.records[record_index].belief_id.clone();
                    self.records[record_index].status = BeliefRecordStatusIR::Superseded;
                    self.records[record_index].last_updated_turn = turn_index;
                    self.push_revision(
                        BeliefRevisionKindIR::Supersedes,
                        &prior_id,
                        Some(&belief_id),
                        turn_index,
                        turn_surface,
                    );
                }
            }
            self.records.push(BeliefRecordIR {
                content,
                belief_id: belief_id.clone(),
                origin_referent_id: observation.origin_referent_id.clone(),
                source_actor: observation.source_actor.clone(),
                proposition_surface: observation.proposition_surface.clone(),
                proposition_polarity: observation.proposition_polarity,
                signature,
                attribution_attitude: observation.attribution_attitude,
                epistemic_status: observation.epistemic_status,
                status: new_status,
                introduced_turn: turn_index,
                last_updated_turn: turn_index,
                dialogue_truth_established: false,
                external_execution_authorized: false,
            });
            bindings.push((observation.origin_referent_id.clone(), belief_id));
        }
        self.reconcile_contested_records();
        self.prune();
        self.unresolved_conflicts.sort();
        self.unresolved_conflicts.dedup();
        debug_assert!(self.validate(turn_index));
        bindings
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        if self.schema != EPISTEMIC_LEDGER_SCHEMA
            || self.records.len() > MAX_BELIEF_RECORDS
            || self.revisions.len() > MAX_BELIEF_REVISIONS
        {
            return false;
        }
        let record_ids = self
            .records
            .iter()
            .map(|record| record.belief_id.as_str())
            .collect::<BTreeSet<_>>();
        let record_indices = self
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| (record.belief_id.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let referent_ids = self
            .records
            .iter()
            .map(|record| record.origin_referent_id.as_str())
            .collect::<BTreeSet<_>>();
        let revision_ids = self
            .revisions
            .iter()
            .map(|revision| revision.revision_id.as_str())
            .collect::<BTreeSet<_>>();
        record_ids.len() == self.records.len()
            && referent_ids.len() == self.records.len()
            && revision_ids.len() == self.revisions.len()
            && self.records.iter().all(|record| {
                let Some(&record_index) = record_indices.get(record.belief_id.as_str()) else {
                    return false;
                };
                !record.belief_id.trim().is_empty()
                    && !record.origin_referent_id.trim().is_empty()
                    && !record.source_actor.trim().is_empty()
                    && !record.proposition_surface.trim().is_empty()
                    && !record.signature.subject_key.trim().is_empty()
                    && !record.signature.normalized_fingerprint.trim().is_empty()
                    && record.introduced_turn > 0
                    && record.introduced_turn <= record.last_updated_turn
                    && record.last_updated_turn <= completed_turns
                    && !record.dialogue_truth_established
                    && !record.external_execution_authorized
                    && record.content.validate_source(&record.proposition_surface)
                    && record.content.context_sources.iter().all(|s| {
                        record_indices
                            .get(s.belief_id.as_str())
                            .is_some_and(|&p_index| {
                                let source = &self.records[p_index];
                                source.source_actor == s.source_actor
                                    && normalized_source(&source.source_actor)
                                        == normalized_source(&record.source_actor)
                                    && source.proposition_surface == s.source_proposition
                                    && (source.introduced_turn < record.introduced_turn
                                        || (source.introduced_turn == record.introduced_turn
                                            && p_index < record_index))
                            })
                    })
                    && record.content.context_sources.windows(2).all(|pair| {
                        record_indices
                            .get(pair[0].belief_id.as_str())
                            .zip(record_indices.get(pair[1].belief_id.as_str()))
                            .is_some_and(|(&left_index, &right_index)| {
                                let left = &self.records[left_index];
                                let right = &self.records[right_index];
                                left.introduced_turn < right.introduced_turn
                                    || (left.introduced_turn == right.introduced_turn
                                        && left_index < right_index)
                            })
                    })
            })
            && self.revisions.iter().all(|revision| {
                !revision.revision_id.trim().is_empty()
                    && record_ids.contains(revision.prior_belief_id.as_str())
                    && revision
                        .new_belief_id
                        .as_deref()
                        .is_none_or(|id| record_ids.contains(id))
                    && revision.turn_index > 0
                    && revision.turn_index <= completed_turns
            })
    }

    fn retract_referenced(
        &mut self,
        turn_index: u64,
        surface: &str,
        referenced_referent_ids: &[String],
    ) {
        let targets = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.status.is_reference_active()
                    && referenced_referent_ids.contains(&record.origin_referent_id)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for index in targets {
            self.records[index].status = BeliefRecordStatusIR::Retracted;
            self.records[index].last_updated_turn = turn_index;
            let belief_id = self.records[index].belief_id.clone();
            self.push_revision(
                BeliefRevisionKindIR::Retracts,
                &belief_id,
                None,
                turn_index,
                surface,
            );
        }
    }

    fn latest_active_source_record(&self, source: &str) -> Option<usize> {
        let source = normalized_source(source);
        self.records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.status.is_reference_active()
                    && normalized_source(&record.source_actor) == source
            })
            .max_by_key(|(_, record)| (record.introduced_turn, record.belief_id.as_str()))
            .map(|(index, _)| index)
    }

    fn latest_active_source_record_for_subject(
        &self,
        source: &str,
        signature: &PropositionSignatureIR,
    ) -> Option<usize> {
        let source = normalized_source(source);
        self.records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.status.is_reference_active()
                    && normalized_source(&record.source_actor) == source
                    && record.signature.subject_key == signature.subject_key
                    && record.signature.modal_world == signature.modal_world
                    && temporal_anchors_compatible(
                        record.signature.temporal_anchor,
                        signature.temporal_anchor,
                    )
            })
            .max_by_key(|(_, record)| (record.introduced_turn, record.belief_id.as_str()))
            .map(|(index, _)| index)
    }

    fn reconcile_contested_records(&mut self) {
        let active = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| record.status.is_reference_active())
            .map(|(index, record)| (index, record.signature.clone()))
            .collect::<Vec<_>>();
        let resolved = active
            .iter()
            .filter(|(index, _)| self.records[*index].status == BeliefRecordStatusIR::Contested)
            .filter(|(index, signature)| {
                !active.iter().any(|(other_index, other_signature)| {
                    index != other_index && signatures_contradict(signature, other_signature)
                })
            })
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        for index in resolved {
            self.records[index].status = BeliefRecordStatusIR::Active;
        }
    }

    fn push_revision(
        &mut self,
        kind: BeliefRevisionKindIR,
        prior_belief_id: &str,
        new_belief_id: Option<&str>,
        turn_index: u64,
        surface: &str,
    ) {
        if self.revisions.iter().any(|revision| {
            revision.kind == kind
                && revision.prior_belief_id == prior_belief_id
                && revision.new_belief_id.as_deref() == new_belief_id
        }) {
            return;
        }
        self.revisions.push(BeliefRevisionIR {
            revision_id: format!("REV-{turn_index:06}-{:03}", self.revisions.len() + 1),
            kind,
            prior_belief_id: prior_belief_id.to_string(),
            new_belief_id: new_belief_id.map(ToString::to_string),
            turn_index,
            evidence_surface: surface.trim().to_string(),
        });
    }

    fn prune(&mut self) {
        if self.records.len() > MAX_BELIEF_RECORDS {
            self.records.sort_by(|left, right| {
                right
                    .last_updated_turn
                    .cmp(&left.last_updated_turn)
                    .then_with(|| right.belief_id.cmp(&left.belief_id))
            });
            self.records.truncate(MAX_BELIEF_RECORDS);
            self.records
                .sort_by(|left, right| left.belief_id.cmp(&right.belief_id));
        }
        // A retained contextual result cannot outlive the inputs required to
        // replay it. Cascade eviction within the bounded dialogue ledger.
        loop {
            let ids = self
                .records
                .iter()
                .map(|r| r.belief_id.clone())
                .collect::<BTreeSet<_>>();
            let before = self.records.len();
            self.records.retain(|r| {
                r.content
                    .context_sources
                    .iter()
                    .all(|s| ids.contains(&s.belief_id))
            });
            if before == self.records.len() {
                break;
            }
        }
        let retained = self
            .records
            .iter()
            .map(|record| record.belief_id.as_str())
            .collect::<BTreeSet<_>>();
        self.revisions.retain(|revision| {
            retained.contains(revision.prior_belief_id.as_str())
                && revision
                    .new_belief_id
                    .as_deref()
                    .is_none_or(|id| retained.contains(id))
        });
        if self.revisions.len() > MAX_BELIEF_REVISIONS {
            let remove = self.revisions.len() - MAX_BELIEF_REVISIONS;
            self.revisions.drain(..remove);
        }
        let valid_conflict_ids = self
            .records
            .iter()
            .filter(|record| record.status == BeliefRecordStatusIR::Contested)
            .map(|record| record.belief_id.as_str())
            .collect::<BTreeSet<_>>();
        self.unresolved_conflicts.retain(|conflict| {
            let mut ids = conflict
                .split(['<', '>', ':'])
                .filter(|part| part.starts_with("BELIEF-"));
            ids.next().zip(ids.next()).is_some_and(|(left, right)| {
                valid_conflict_ids.contains(left) && valid_conflict_ids.contains(right)
            })
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct StateLexeme {
    axis: &'static str,
    value: SemanticStateValueIR,
    forms: &'static [&'static str],
}

const STATE_LEXEMES: &[StateLexeme] = &[
    StateLexeme {
        axis: "operational_state",
        value: SemanticStateValueIR::Positive,
        forms: &["up", "running", "online", "정상", "작동", "가동"],
    },
    StateLexeme {
        axis: "operational_state",
        value: SemanticStateValueIR::Negative,
        forms: &[
            "down",
            "stopped",
            "offline",
            "abnormal",
            "멈",
            "중단",
            "죽",
            "비정상",
        ],
    },
    StateLexeme {
        axis: "outcome",
        value: SemanticStateValueIR::Positive,
        forms: &[
            "success",
            "succeed",
            "succeeds",
            "succeeded",
            "pass",
            "passes",
            "passed",
            "성공",
            "통과",
        ],
    },
    StateLexeme {
        axis: "outcome",
        value: SemanticStateValueIR::Negative,
        forms: &["failure", "failed", "fails", "실패", "오류"],
    },
    StateLexeme {
        axis: "integrity",
        value: SemanticStateValueIR::Positive,
        forms: &["healthy", "valid", "intact", "온전", "유효"],
    },
    StateLexeme {
        axis: "integrity",
        value: SemanticStateValueIR::Negative,
        forms: &[
            "corrupt",
            "corrupted",
            "broken",
            "invalid",
            "unhealthy",
            "손상",
            "깨",
            "오염",
        ],
    },
    StateLexeme {
        axis: "readiness",
        value: SemanticStateValueIR::Positive,
        forms: &["ready", "prepared", "준비"],
    },
    StateLexeme {
        axis: "readiness",
        value: SemanticStateValueIR::Negative,
        forms: &["unready", "not-ready", "미준비", "준비되지"],
    },
    StateLexeme {
        axis: "completion",
        value: SemanticStateValueIR::Positive,
        forms: &[
            "complete",
            "completed",
            "finish",
            "finished",
            "done",
            "완료",
            "끝",
        ],
    },
    StateLexeme {
        axis: "completion",
        value: SemanticStateValueIR::Negative,
        forms: &["incomplete", "unfinished", "pending", "미완", "보류"],
    },
    StateLexeme {
        axis: "availability",
        value: SemanticStateValueIR::Positive,
        forms: &["available", "present", "존재", "사용가능"],
    },
    StateLexeme {
        axis: "availability",
        value: SemanticStateValueIR::Negative,
        forms: &["unavailable", "missing", "absent", "없"],
    },
    StateLexeme {
        axis: "freshness",
        value: SemanticStateValueIR::Positive,
        forms: &["fresh", "current", "up-to-date", "최신", "신선"],
    },
    StateLexeme {
        axis: "freshness",
        value: SemanticStateValueIR::Negative,
        forms: &["stale", "outdated", "obsolete", "오래된", "낡"],
    },
    StateLexeme {
        axis: "truth",
        value: SemanticStateValueIR::Positive,
        forms: &["true", "correct", "사실", "맞"],
    },
    StateLexeme {
        axis: "truth",
        value: SemanticStateValueIR::Negative,
        forms: &["false", "incorrect", "거짓", "틀"],
    },
    StateLexeme {
        axis: "enablement",
        value: SemanticStateValueIR::Positive,
        forms: &["enabled", "active", "활성", "켜"],
    },
    StateLexeme {
        axis: "enablement",
        value: SemanticStateValueIR::Negative,
        forms: &["disabled", "inactive", "비활성", "꺼"],
    },
];

pub fn proposition_signature(
    proposition: &str,
    polarity: AttributedPropositionPolarityIR,
) -> PropositionSignatureIR {
    proposition_signature_in_world(proposition, polarity, ModalWorldIR::Actual)
}

pub fn proposition_signature_in_world(
    proposition: &str,
    polarity: AttributedPropositionPolarityIR,
    modal_world: ModalWorldIR,
) -> PropositionSignatureIR {
    let tokens = semantic_tokens(proposition);
    let matched = STATE_LEXEMES
        .iter()
        .flat_map(|lexeme| {
            tokens.iter().enumerate().flat_map(move |(index, token)| {
                lexeme.forms.iter().filter_map(move |form| {
                    token_matches_state_form(token, form).then_some((lexeme, index, form.len()))
                })
            })
        })
        .max_by_key(|(_, _, form_len)| *form_len)
        .map(|(lexeme, index, _)| (lexeme, index));
    let subject_end = matched.map_or(tokens.len(), |(_, index)| index);
    let subject_key = tokens[..subject_end]
        .iter()
        .rev()
        .find_map(|token| normalized_subject_token(token))
        .or_else(|| {
            tokens
                .first()
                .and_then(|token| normalized_subject_token(token))
        })
        .unwrap_or_else(|| "unknown_subject".to_string());
    let (state_axis, state_value) = matched.map_or((None, None), |(lexeme, _)| {
        let value = if polarity == AttributedPropositionPolarityIR::Negative {
            lexeme.value.inverted()
        } else {
            lexeme.value
        };
        (Some(lexeme.axis.to_string()), Some(value))
    });
    PropositionSignatureIR {
        subject_key,
        temporal_anchor: temporal_anchor(&tokens),
        modal_world,
        state_axis,
        state_value,
        normalized_fingerprint: tokens.join(" "),
    }
}

fn signatures_comparable(left: &PropositionSignatureIR, right: &PropositionSignatureIR) -> bool {
    left.modal_world == right.modal_world
        && temporal_anchors_compatible(left.temporal_anchor, right.temporal_anchor)
        && (left.normalized_fingerprint == right.normalized_fingerprint
            || (left.subject_key == right.subject_key
                && left.state_axis.is_some()
                && left.state_axis == right.state_axis))
}

fn signatures_equivalent(left: &PropositionSignatureIR, right: &PropositionSignatureIR) -> bool {
    left.modal_world == right.modal_world
        && temporal_anchors_compatible(left.temporal_anchor, right.temporal_anchor)
        && (left.normalized_fingerprint == right.normalized_fingerprint
            || (left.subject_key == right.subject_key
                && left.state_axis.is_some()
                && left.state_axis == right.state_axis
                && left.state_value == right.state_value))
}

fn signatures_contradict(left: &PropositionSignatureIR, right: &PropositionSignatureIR) -> bool {
    left.modal_world == right.modal_world
        && temporal_anchors_compatible(left.temporal_anchor, right.temporal_anchor)
        && left.subject_key == right.subject_key
        && left.state_axis.is_some()
        && left.state_axis == right.state_axis
        && left.state_value.is_some()
        && right.state_value.is_some()
        && left.state_value != right.state_value
}

fn temporal_anchor(tokens: &[String]) -> TemporalAnchorIR {
    if tokens.iter().any(|token| {
        ["yesterday", "previously", "earlier", "어제", "이전", "아까"]
            .iter()
            .any(|marker| token.contains(marker))
    }) {
        TemporalAnchorIR::Past
    } else if tokens.iter().any(|token| {
        ["now", "today", "currently", "지금", "오늘", "현재", "이제"]
            .iter()
            .any(|marker| token.contains(marker))
    }) {
        TemporalAnchorIR::Present
    } else if tokens.iter().any(|token| {
        ["tomorrow", "later", "future", "내일", "향후", "나중"]
            .iter()
            .any(|marker| token.contains(marker))
    }) {
        TemporalAnchorIR::Future
    } else {
        TemporalAnchorIR::Unspecified
    }
}

fn temporal_anchors_compatible(left: TemporalAnchorIR, right: TemporalAnchorIR) -> bool {
    left == right || left == TemporalAnchorIR::Unspecified || right == TemporalAnchorIR::Unspecified
}

fn semantic_tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn token_matches_state_form(token: &str, form: &str) -> bool {
    if form.is_ascii() {
        token == form
    } else {
        token.contains(form)
    }
}

fn normalized_subject_token(token: &str) -> Option<String> {
    if [
        "the", "a", "an", "is", "are", "was", "were", "has", "have", "did", "does", "not", "no",
        "now", "actually", "that", "것", "이", "그", "저",
    ]
    .contains(&token)
    {
        return None;
    }
    let mut normalized = token.to_string();
    for suffix in ["은", "는", "이", "가", "을", "를", "도"] {
        if normalized.ends_with(suffix) && normalized.len() > suffix.len() {
            normalized.truncate(normalized.len() - suffix.len());
            break;
        }
    }
    (!normalized.is_empty()).then_some(normalized)
}

fn normalized_source(source: &str) -> String {
    source
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn is_retraction_surface(text: &str) -> bool {
    let normalized = text.to_lowercase();
    [
        "retract",
        "withdraw",
        "take back",
        "취소",
        "철회",
        "거둬",
        "거두",
        "번복",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn is_revision_surface(text: &str) -> bool {
    let normalized = text.to_lowercase();
    if normalized.starts_with("no, ") {
        return true;
    }
    [
        " now ",
        "now ",
        "actually",
        "corrected",
        "corrects",
        "correction",
        "revised",
        "instead",
        "이제",
        "지금은",
        "사실은",
        "정정",
        "바로잡",
        "아니,",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_context_is_bounded_source_bound_and_evicted_with_its_inputs() {
        use crate::proposition_content::ContentSlotIR;
        let mut ledger = EpistemicLedgerIR::default();
        let first = "Mira lent a letter to Sol.";
        ledger.apply_turn(1, first, &[], &[observation("P1", "McKay", first)]);
        let second = "Sol read it at school.";
        ledger.apply_turn_with_event_focus(
            3,
            second,
            &[],
            &[observation("P3", "mckay", second)],
            Some("BELIEF-000001-01"),
        );
        assert_eq!(
            ledger.records[1].content.events[0].roles[&ContentSlotIR::Theme],
            "letter"
        );
        assert!(ledger.validate(3));
        let mut tampered = ledger.clone();
        tampered.records[1].content.context_sources[0].source_actor = "OTHER".into();
        assert!(!tampered.validate(3));
        for turn in 4..75 {
            let text = format!("Item{turn} says ready");
            ledger.apply_turn(
                turn,
                &text,
                &[],
                &[observation(&format!("P{turn}"), "OTHER", &text)],
            );
            assert!(ledger.validate(turn));
        }
        assert!(ledger.record("BELIEF-000003-01").is_none());
    }

    #[test]
    fn unmatched_event_correction_does_not_replace_an_unrelated_record() {
        let mut ledger = EpistemicLedgerIR::default();
        let first = "수아는 서점에서 편지를 읽었어.";
        ledger.apply_turn(1, first, &[], &[observation("P1", "USER", first)]);
        let second = "아니, 공원이 아니라 학교에서 읽었어.";
        ledger.apply_turn(2, second, &[], &[observation("P2", "USER", second)]);
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Active);
        assert!(ledger.records[1].content.events.is_empty());
        assert!(ledger.validate(2));
    }

    #[test]
    fn revision_force_does_not_leak_to_sibling_observations() {
        use crate::proposition_content::ContentSlotIR;
        for (source, correction, slot, expected) in [
            (
                "유림은 공원에서 편지를 읽었어.",
                "공원이 아니고 집이야",
                ContentSlotIR::Location,
                "집",
            ),
            (
                "유림은 공원에서 편지를 읽었어.",
                "편지가 아니고 신문이야",
                ContentSlotIR::Theme,
                "신문",
            ),
            (
                "유림은 공원에서 편지를 읽었어.",
                "유림이 아니고 다원이야",
                ContentSlotIR::Agent,
                "다원",
            ),
            (
                "Mira read a letter at the harbor.",
                "It was the garden, not the harbor",
                ContentSlotIR::Location,
                "garden",
            ),
            (
                "Mira read a letter at the harbor.",
                "It was the newspaper, not the letter",
                ContentSlotIR::Theme,
                "newspaper",
            ),
        ] {
            for correction_first in [false, true] {
                let mut ledger = EpistemicLedgerIR::default();
                ledger.apply_turn(1, source, &[], &[observation("P1", "USER", source)]);
                let before = ledger.records[0].content.events[0].clone();
                let preface = observation("P2-PREFACE", "USER", "앞의 설명을 잘못 말했네");
                let replacement = observation("P2-REVISION", "USER", correction);
                let observations = if correction_first {
                    vec![replacement, preface]
                } else {
                    vec![preface, replacement]
                };
                let turn = observations
                    .iter()
                    .map(|o| o.proposition_surface.as_str())
                    .collect::<Vec<_>>()
                    .join(". ");
                ledger.apply_turn(2, &turn, &[], &observations);
                assert_eq!(
                    ledger.records[0].status,
                    BeliefRecordStatusIR::Superseded,
                    "{turn}"
                );
                let current = ledger
                    .records
                    .iter()
                    .find(|r| r.origin_referent_id == "P2-REVISION")
                    .unwrap();
                assert_eq!(current.status, BeliefRecordStatusIR::Active);
                assert_eq!(current.content.events.len(), 1, "{turn}");
                let event = &current.content.events[0];
                assert_eq!(event.roles[&slot], expected);
                assert_eq!(event.lexical_entry_ids, before.lexical_entry_ids);
                for (role, value) in &before.roles {
                    if *role != slot {
                        assert_eq!(event.roles[role], *value);
                    }
                }
                assert_eq!(
                    current.content.context_sources[0].belief_id,
                    "BELIEF-000001-01"
                );
                assert!(ledger.validate(2));
            }
        }
    }

    #[test]
    fn nominal_revision_requires_unique_same_source_event_and_role() {
        for (second_source, second_text, expected_events) in [
            ("USER", "Sol read a newspaper at the harbor.", 0),
            ("OTHER", "Sol read a newspaper at the harbor.", 1),
        ] {
            let mut ledger = EpistemicLedgerIR::default();
            let first = "Mira read a letter at the harbor.";
            ledger.apply_turn(1, first, &[], &[observation("P1", "USER", first)]);
            ledger.apply_turn(
                2,
                second_text,
                &[],
                &[observation("P2", second_source, second_text)],
            );
            let correction = "It was the garden, not the harbor.";
            ledger.apply_turn(3, correction, &[], &[observation("P3", "USER", correction)]);
            assert_eq!(ledger.records[2].content.events.len(), expected_events);
            assert_eq!(ledger.records[1].status, BeliefRecordStatusIR::Active);
            if expected_events == 0 {
                assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Active);
            }
            assert!(ledger.validate(3));
        }
    }

    fn observation(id: &str, source: &str, proposition: &str) -> EpistemicObservationIR {
        EpistemicObservationIR {
            origin_referent_id: id.to_string(),
            source_actor: source.to_string(),
            proposition_surface: proposition.to_string(),
            proposition_polarity: AttributedPropositionPolarityIR::Positive,
            modal_world: ModalWorldIR::Actual,
            attribution_attitude: AttributionAttitudeIR::Say,
            epistemic_status: EpistemicStatusIR::Reported,
        }
    }

    #[test]
    fn explicit_same_source_update_supersedes_opposite_state() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Alice says server down",
            &[],
            &[observation("P1", "Alice", "server down")],
        );
        ledger.apply_turn(
            2,
            "Alice now says server up",
            &[],
            &[observation("P2", "Alice", "server up")],
        );
        assert!(ledger.validate(2));
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Superseded);
        assert_eq!(ledger.records[1].status, BeliefRecordStatusIR::Active);
        assert!(ledger
            .revisions
            .iter()
            .any(|revision| revision.kind == BeliefRevisionKindIR::Supersedes));
    }

    #[test]
    fn explicit_correction_supersedes_latest_same_source_without_a_known_state_axis() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Nora reports that the build failed",
            &[],
            &[observation("P1", "Nora", "the build failed")],
        );
        ledger.apply_turn(
            2,
            "Correction: Nora reports that the build succeeded",
            &[],
            &[observation("P2", "Nora", "the build succeeded")],
        );

        assert!(ledger.validate(2));
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Superseded);
        assert_eq!(ledger.records[1].status, BeliefRecordStatusIR::Active);
        assert!(ledger.revisions.iter().any(|revision| {
            revision.kind == BeliefRevisionKindIR::Supersedes
                && revision.prior_belief_id == ledger.records[0].belief_id
                && revision.new_belief_id.as_deref() == Some(ledger.records[1].belief_id.as_str())
        }));
    }

    #[test]
    fn explicit_correction_prefers_same_source_and_subject_over_newer_unrelated_subject() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Jisu says the cache is corrupted",
            &[],
            &[observation("P1", "Jisu", "cache corrupted")],
        );
        ledger.apply_turn(
            2,
            "Jisu says the worker is idle",
            &[],
            &[observation("P2", "Jisu", "worker idle")],
        );
        ledger.apply_turn(
            3,
            "Correction: Jisu says the cache is healthy",
            &[],
            &[observation("P3", "Jisu", "cache healthy")],
        );

        assert!(ledger.validate(3));
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Superseded);
        assert_eq!(ledger.records[1].status, BeliefRecordStatusIR::Active);
        assert_eq!(ledger.records[2].status, BeliefRecordStatusIR::Active);
        assert!(ledger.revisions.iter().any(|revision| {
            revision.kind == BeliefRevisionKindIR::Supersedes
                && revision.prior_belief_id == ledger.records[0].belief_id
                && revision.new_belief_id.as_deref() == Some(ledger.records[2].belief_id.as_str())
        }));
    }

    #[test]
    fn different_sources_remain_contested_without_selecting_truth() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Alice says server down",
            &[],
            &[observation("P1", "Alice", "server down")],
        );
        ledger.apply_turn(
            2,
            "Bob says server up",
            &[],
            &[observation("P2", "Bob", "server up")],
        );
        assert!(ledger.validate(2));
        assert!(ledger
            .records
            .iter()
            .all(|record| record.status == BeliefRecordStatusIR::Contested));
        assert!(ledger
            .records
            .iter()
            .all(|record| !record.dialogue_truth_established));
    }

    #[test]
    fn same_turn_explicit_subject_reference_object_binds_in_source_order() {
        let mut ledger = EpistemicLedgerIR::default();
        let first = "Alice read a note.";
        let second = "Bob read it.";
        assert!(
            crate::proposition_content::PropositionContentIR::compile(second).events[0]
                .has_references()
        );
        assert!(
            crate::proposition_content::PropositionContentIR::compile_contextual(
                second,
                &[crate::proposition_content::EventSourceIR {
                    belief_id: "X".into(),
                    source_actor: "USER".into(),
                    source_proposition: first.into()
                }]
            )
            .is_some()
        );
        ledger.apply_turn_with_event_focus(
            1,
            "Alice read a note and Bob read it.",
            &[],
            &[
                observation("P1", "USER", first),
                observation("P2", "USER", second),
            ],
            None,
        );
        assert_eq!(ledger.records.len(), 2);
        assert_eq!(ledger.records[1].content.context_sources.len(), 1);
        assert_eq!(
            ledger.records[1].content.events[0].roles
                [&crate::proposition_content::ContentSlotIR::Theme],
            "note"
        );
        assert!(ledger.validate(1));
    }

    #[test]
    fn korean_same_turn_source_preserves_literal_and_typed_object_identity() {
        use crate::proposition_content::ContentSlotIR;

        for (first, second, expected_theme) in [
            (
                "도윤은 책을 읽었어.",
                "도윤은 그것을 민서에게 주었어.",
                "책",
            ),
            (
                "라온은 지도를 읽었어.",
                "라온은 그것을 유나에게 주었어.",
                "지도",
            ),
        ] {
            let mut ledger = EpistemicLedgerIR::default();
            ledger.apply_turn_with_event_focus(
                1,
                &format!("{first} {second}"),
                &[],
                &[
                    observation("FIRST", "USER", first),
                    observation("SECOND", "USER", second),
                ],
                None,
            );
            let record = &ledger.records[1];
            assert_eq!(record.proposition_surface, second);
            assert_eq!(record.content.context_sources.len(), 1);
            assert_eq!(record.content.context_sources[0].source_proposition, first);
            assert_eq!(
                record.content.events[0].roles[&ContentSlotIR::Theme],
                expected_theme
            );
            assert!(ledger.validate(1));
        }
    }

    #[test]
    fn same_turn_reference_stays_unresolved_without_or_with_ambiguous_sources() {
        use crate::proposition_content::ContentSlotIR;

        let mut absent = EpistemicLedgerIR::default();
        let unresolved = "라온은 그것을 유나에게 주었어.";
        absent.apply_turn(
            1,
            unresolved,
            &[],
            &[observation("ONLY", "USER", unresolved)],
        );
        assert_eq!(
            absent.records[0].content.events[0].roles[&ContentSlotIR::Theme],
            "그것"
        );
        assert!(absent.records[0].content.context_sources.is_empty());
        assert!(absent.validate(1));

        let mut ambiguous = EpistemicLedgerIR::default();
        let first = "수아는 책을 읽었어.";
        let second = "수아는 편지를 읽었어.";
        let reference = "수아는 그것을 민서에게 주었어.";
        ambiguous.apply_turn_with_event_focus(
            1,
            &format!("{first} {second} {reference}"),
            &[],
            &[
                observation("FIRST", "USER", first),
                observation("SECOND", "USER", second),
                observation("REFERENCE", "USER", reference),
            ],
            None,
        );
        assert_eq!(
            ambiguous.records[2].content.events[0].roles[&ContentSlotIR::Theme],
            "그것"
        );
        assert!(ambiguous.records[2].content.context_sources.is_empty());
        assert!(ambiguous.validate(1));
    }

    #[test]
    fn contextual_source_order_rejects_future_self_and_cyclic_bindings() {
        use crate::proposition_content::EventSourceIR;

        let mut future = EpistemicLedgerIR::default();
        let first = "Alice read a note.";
        let second = "Bob read a letter.";
        future.apply_turn(1, first, &[], &[observation("FIRST", "USER", first)]);
        future.apply_turn(2, second, &[], &[observation("SECOND", "USER", second)]);
        let later = future.records[1].clone();
        future.records[0].content =
            crate::proposition_content::PropositionContentIR::compile_contextual(
                first,
                &[EventSourceIR {
                    belief_id: later.belief_id,
                    source_actor: later.source_actor,
                    source_proposition: later.proposition_surface,
                }],
            )
            .expect("future source content compiles");
        assert!(future.records[0].content.validate_source(first));
        assert!(!future.validate(2));

        let mut self_reference = EpistemicLedgerIR::default();
        self_reference.apply_turn(1, first, &[], &[observation("FIRST", "USER", first)]);
        let own = self_reference.records[0].clone();
        self_reference.records[0].content =
            crate::proposition_content::PropositionContentIR::compile_contextual(
                first,
                &[EventSourceIR {
                    belief_id: own.belief_id,
                    source_actor: own.source_actor,
                    source_proposition: own.proposition_surface,
                }],
            )
            .expect("self source content compiles");
        assert!(self_reference.records[0].content.validate_source(first));
        assert!(!self_reference.validate(1));

        let mut cycle = EpistemicLedgerIR::default();
        cycle.apply_turn(1, first, &[], &[observation("FIRST", "USER", first)]);
        cycle.apply_turn(2, second, &[], &[observation("SECOND", "USER", second)]);
        let left = cycle.records[0].clone();
        let right = cycle.records[1].clone();
        cycle.records[0].content =
            crate::proposition_content::PropositionContentIR::compile_contextual(
                first,
                &[EventSourceIR {
                    belief_id: right.belief_id,
                    source_actor: right.source_actor,
                    source_proposition: right.proposition_surface,
                }],
            )
            .expect("left cycle source content compiles");
        cycle.records[1].content =
            crate::proposition_content::PropositionContentIR::compile_contextual(
                second,
                &[EventSourceIR {
                    belief_id: left.belief_id,
                    source_actor: left.source_actor,
                    source_proposition: left.proposition_surface,
                }],
            )
            .expect("right cycle source content compiles");
        assert!(cycle.records[0].content.validate_source(first));
        assert!(cycle.records[1].content.validate_source(second));
        assert!(!cycle.validate(2));

        let mut same_turn = EpistemicLedgerIR::default();
        same_turn.apply_turn(
            1,
            first,
            &[],
            &[
                observation("FIRST", "USER", first),
                observation("SECOND", "USER", second),
            ],
        );
        let later_same_turn = same_turn.records[1].clone();
        same_turn.records[0].content =
            crate::proposition_content::PropositionContentIR::compile_contextual(
                first,
                &[EventSourceIR {
                    belief_id: later_same_turn.belief_id,
                    source_actor: later_same_turn.source_actor,
                    source_proposition: later_same_turn.proposition_surface,
                }],
            )
            .expect("same-turn later source content compiles");
        assert!(same_turn.records[0].content.validate_source(first));
        assert!(!same_turn.validate(1));
    }

    #[test]
    fn stale_and_not_stale_are_opposite_values_on_one_freshness_axis() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Mina says the cache is stale",
            &[],
            &[observation("P1", "Mina", "the cache is stale")],
        );
        let mut not_stale = observation("P2", "Joon", "the cache is not stale");
        not_stale.proposition_polarity = AttributedPropositionPolarityIR::Negative;
        ledger.apply_turn(2, "Joon says the cache is not stale", &[], &[not_stale]);
        assert!(ledger.validate(2));
        assert_eq!(ledger.unresolved_conflicts.len(), 1);
        assert!(ledger.records.iter().all(|record| {
            record.status == BeliefRecordStatusIR::Contested
                && record.signature.state_axis.as_deref() == Some("freshness")
        }));
    }

    #[test]
    fn explicit_reference_retraction_deactivates_record() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Alice says server down",
            &[],
            &[observation("P1", "Alice", "server down")],
        );
        ledger.apply_turn(2, "Alice retracts that claim", &["P1".to_string()], &[]);
        assert!(ledger.validate(2));
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Retracted);
        assert!(ledger
            .revisions
            .iter()
            .any(|revision| revision.kind == BeliefRevisionKindIR::Retracts));
    }

    #[test]
    fn proposition_negation_inverts_known_state_value() {
        let signature = proposition_signature(
            "deployment did not finish",
            AttributedPropositionPolarityIR::Negative,
        );
        assert_eq!(signature.state_axis.as_deref(), Some("completion"));
        assert_eq!(signature.state_value, Some(SemanticStateValueIR::Negative));
    }

    #[test]
    fn explicit_past_and_present_states_are_not_a_logical_conflict() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Alice says yesterday server down",
            &[],
            &[observation("P1", "Alice", "yesterday server down")],
        );
        ledger.apply_turn(
            2,
            "Alice says today server up",
            &[],
            &[observation("P2", "Alice", "today server up")],
        );
        assert!(ledger.validate(2));
        assert!(ledger
            .records
            .iter()
            .all(|record| record.status == BeliefRecordStatusIR::Active));
        assert!(ledger.revisions.is_empty());
    }

    #[test]
    fn retracting_one_side_resolves_the_remaining_contested_record() {
        let mut ledger = EpistemicLedgerIR::default();
        ledger.apply_turn(
            1,
            "Alice says server down",
            &[],
            &[observation("P1", "Alice", "server down")],
        );
        ledger.apply_turn(
            2,
            "Bob says server up",
            &[],
            &[observation("P2", "Bob", "server up")],
        );
        ledger.apply_turn(3, "Bob retracts that claim", &["P2".to_string()], &[]);
        assert!(ledger.validate(3));
        assert_eq!(ledger.records[0].status, BeliefRecordStatusIR::Active);
        assert_eq!(ledger.records[1].status, BeliefRecordStatusIR::Retracted);
        assert!(ledger.unresolved_conflicts.is_empty());
    }

    #[test]
    fn possible_and_actual_states_do_not_form_a_logical_contradiction() {
        let mut ledger = EpistemicLedgerIR::default();
        let actual = observation("P-1", "Alice", "server is up");
        ledger.apply_turn(1, "Alice says the server is up", &[], &[actual]);
        let mut possible = observation("P-2", "Alice", "server is down");
        possible.modal_world = ModalWorldIR::EpistemicPossible;
        ledger.apply_turn(2, "Alice says the server might be down", &[], &[possible]);
        assert_eq!(ledger.records.len(), 2);
        assert!(ledger.revisions.is_empty());
        assert!(ledger.unresolved_conflicts.is_empty());
        assert_eq!(
            ledger.records[1].signature.modal_world,
            ModalWorldIR::EpistemicPossible
        );
    }

    #[test]
    fn opposite_states_inside_the_same_possible_world_can_be_contested() {
        let mut ledger = EpistemicLedgerIR::default();
        let mut up = observation("P-1", "Alice", "server is up");
        up.modal_world = ModalWorldIR::EpistemicPossible;
        ledger.apply_turn(1, "server might be up", &[], &[up]);
        let mut down = observation("P-2", "Bob", "server is down");
        down.modal_world = ModalWorldIR::EpistemicPossible;
        ledger.apply_turn(2, "server might be down", &[], &[down]);
        assert_eq!(ledger.unresolved_conflicts.len(), 1);
        assert!(ledger.records.iter().all(|record| {
            record.status == BeliefRecordStatusIR::Contested
                && record.signature.modal_world == ModalWorldIR::EpistemicPossible
        }));
    }
}
