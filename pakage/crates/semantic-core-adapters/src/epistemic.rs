//! Bounded, temporally versioned epistemic ledger for conversation state.
//!
//! The ledger records what a dialogue participant or attributed source was
//! represented as asserting. It is not a world-fact database. No ledger record
//! establishes truth or grants execution authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::attribution::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, EpistemicStatusIR,
};
use crate::modality::ModalWorldIR;

pub const EPISTEMIC_LEDGER_SCHEMA: &str = "B_CORE_EPISTEMIC_LEDGER_IR_2";
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
        if is_retraction_surface(turn_surface) {
            self.retract_referenced(turn_index, turn_surface, referenced_referent_ids);
        }
        let explicit_revision = is_revision_surface(turn_surface);
        let mut bindings = Vec::new();
        for (index, observation) in observations.iter().enumerate() {
            let belief_id = format!("BELIEF-{turn_index:06}-{:02}", index + 1);
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
                if let Some(record_index) = self
                    .latest_active_source_record_for_subject(&observation.source_actor, &signature)
                    .or_else(|| self.latest_active_source_record(&observation.source_actor))
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
