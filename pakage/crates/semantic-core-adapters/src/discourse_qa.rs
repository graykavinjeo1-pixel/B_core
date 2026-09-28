//! Evidence-bounded question answering over conversation discourse state.
//!
//! This module does not answer from surface plausibility or general knowledge.
//! It compiles a bounded family of dialogue questions into typed queries and
//! answers only from the attribution, modality, and revision records already
//! present in `ConversationStateIR`.  A report that somebody knows, believes,
//! or observed a proposition remains a report; it is never promoted to truth.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::attribution::{AttributionAttitudeIR, EpistemicStatusIR};
use crate::conversation::ConversationStateIR;
use crate::epistemic::{BeliefRecordIR, BeliefRecordStatusIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::ModalWorldIR;

pub const DISCOURSE_QUERY_SCHEMA: &str = "B_CORE_DISCOURSE_QUERY_IR_1";
pub const DISCOURSE_ANSWER_SCHEMA: &str = "B_CORE_DISCOURSE_ANSWER_IR_46";
const MAX_ANSWER_EVIDENCE: usize = 16;
const MAX_ANSWER_CLAIMS: usize = 16;

/// One question under discussion, not an answer cache. Re-expression must
/// query the current evidence ledger; prior output is never evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerFocusIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_proposition: Option<SharedPropositionFocusIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposition_belief_id: Option<String>,
    pub query: DiscourseQueryIR,
    pub answered_turn: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub described_event: Option<DescribedEventReferenceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clarification_act: Option<crate::world_dialogue::WorldClarificationActIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_inquiry: Option<crate::utterance_intent::DecisionInquiryIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribedEventReferenceIR {
    pub belief_id: String,
    pub event_id: String,
}

/// A typed description under discussion, independent of who reported it.
/// The finite predicate form is retained until a complete tense/aspect IR is
/// available: matching predicate senses alone must not merge past and present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropositionMeaningIR {
    pub kind: crate::proposition_content::DescriptionKindIR,
    pub predicate_ids: Vec<String>,
    pub predicate_form: String,
    pub negated: bool,
    pub roles: BTreeMap<crate::proposition_content::ContentSlotIR, String>,
    pub modal_world: ModalWorldIR,
    pub polarity: crate::attribution::AttributedPropositionPolarityIR,
}

impl PropositionMeaningIR {
    fn from_record(record: &BeliefRecordIR) -> Option<Self> {
        let event =
            crate::proposition_content::described_event(&record.proposition_surface, false)?;
        if event.has_references()
            || event.has_unbound_participant()
            || !record.content.context_sources.is_empty()
            || record.content.events != [event.clone()]
            || !record.content.validate_source(&record.proposition_surface)
        {
            return None;
        }
        Some(Self {
            kind: event.kind,
            predicate_ids: event.lexical_entry_ids,
            predicate_form: event.predicate_surface,
            negated: event.negated,
            roles: event.roles,
            modal_world: record.signature.modal_world,
            polarity: record.proposition_polarity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedPropositionFocusIR {
    pub meaning: PropositionMeaningIR,
    pub belief_ids: Vec<String>,
}

impl SharedPropositionFocusIR {
    fn validate(&self) -> bool {
        (2..=MAX_ANSWER_EVIDENCE).contains(&self.belief_ids.len())
            && self.belief_ids.iter().all(|id| !id.is_empty())
            && self.belief_ids.windows(2).all(|pair| pair[0] < pair[1])
            && !self.meaning.predicate_ids.is_empty()
            && !self.meaning.predicate_form.is_empty()
    }

    fn validate_in(&self, state: &ConversationStateIR) -> bool {
        self.validate()
            && self.belief_ids.iter().all(|id| {
                state
                    .epistemic_ledger
                    .record(id)
                    .and_then(PropositionMeaningIR::from_record)
                    .as_ref()
                    == Some(&self.meaning)
            })
    }
}

impl DescribedEventReferenceIR {
    pub fn validate_in(&self, state: &ConversationStateIR) -> bool {
        state.epistemic_ledger.records.iter().any(|r| {
            r.belief_id == self.belief_id
                && r.content.events.iter().any(|e| e.event_id == self.event_id)
                && r.content.validate_source(&r.proposition_surface)
        })
    }
}

impl AnswerFocusIR {
    pub(crate) fn validate_proposition_in(&self, state: &ConversationStateIR) -> bool {
        self.shared_proposition
            .as_ref()
            .is_none_or(|p| p.validate_in(state))
            && self.proposition_belief_id.as_ref().is_none_or(|id| {
                state.epistemic_ledger.records.iter().any(|r| {
                    &r.belief_id == id && r.content.validate_source(&r.proposition_surface)
                })
            })
            && self.clarification_act.as_ref().is_none_or(|act| {
                self.shared_proposition.is_none()
                    && self.proposition_belief_id.is_none()
                    && self.described_event.is_none()
                    && state
                        .dialogue_world
                        .pending_reference
                        .as_ref()
                        .is_some_and(|gap| {
                            act.validate_source(gap, &self.query.original_text, self.answered_turn)
                        })
            })
    }
    pub fn validate(&self, completed_turns: u64) -> bool {
        self.answered_turn > 0
            && self.decision_inquiry.as_ref().is_none_or(|inquiry| {
                inquiry.validate()
                    && inquiry.source_text == self.query.original_text
                    && inquiry
                        .clarification_reply
                        .as_ref()
                        .is_none_or(|r| r.turn == self.answered_turn)
                    && inquiry.explanation_of.as_ref().is_none_or(|o| {
                        o.asked_turn < self.answered_turn && self.answered_turn - o.asked_turn <= 3
                    })
                    && self.shared_proposition.is_none()
                    && self.proposition_belief_id.is_none()
                    && self.described_event.is_none()
                    && self.clarification_act.is_none()
            })
            && self.shared_proposition.as_ref().is_none_or(|p| {
                p.validate()
                    && self.proposition_belief_id.is_none()
                    && self.described_event.is_none()
            })
            && self
                .clarification_act
                .as_ref()
                .is_none_or(|a| a.turn == self.answered_turn)
            && self.answered_turn <= completed_turns
            && self.query.schema == DISCOURSE_QUERY_SCHEMA
            && !self.query.original_text.trim().is_empty()
            && self.query.original_text.chars().count() <= 4096
            && self.query.topic_terms.len() <= 64
            && self
                .proposition_belief_id
                .as_ref()
                .is_none_or(|id| !id.is_empty())
            && self
                .described_event
                .as_ref()
                .is_none_or(|r| !r.belief_id.is_empty() && !r.event_id.is_empty())
    }
}

/// Binding of an omitted relational argument to attributed memory. This is a
/// reference receipt, not an answer cache or an assertion that the report is true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextualContentTargetIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_proposition: Option<SharedPropositionFocusIR>,
    pub belief_id: String,
    pub slot: crate::proposition_content::ContentSlotIR,
    pub resolved_after_turn: u64,
    pub introduced_turn: u64,
}

impl ContextualContentTargetIR {
    fn includes_belief(&self, id: &str) -> bool {
        self.shared_proposition
            .as_ref()
            .map_or(self.belief_id == id, |p| {
                p.belief_ids.iter().any(|b| b == id)
            })
    }

    fn validates_answer(&self, answer: &DiscourseAnswerIR) -> bool {
        !self.belief_id.is_empty()
            && self.shared_proposition.as_ref().is_none_or(|p| {
                p.validate()
                    && p.belief_ids.first() == Some(&self.belief_id)
                    && attribution_reference_slot(&answer.query) == Some(self.slot)
            })
            && self.introduced_turn > 0
            && self.introduced_turn <= self.resolved_after_turn
            && (crate::proposition_content::contextual_content_slot(&answer.query.original_text)
                == Some(self.slot)
                || attribution_reference_slot(&answer.query) == Some(self.slot))
            && answer.event_summary.is_none()
            && answer.world_reasoning.is_none()
            && answer.world_memory_update.is_none()
            && answer.world_clarification.is_none()
            && answer.decision_inquiry.is_none()
            && answer
                .content_projection
                .as_ref()
                .is_none_or(|p| p.belief_id == self.belief_id && p.binding.slot == self.slot)
            && answer
                .evidence
                .iter()
                .all(|e| self.includes_belief(&e.belief_id))
    }

    fn validate_in(&self, state: &ConversationStateIR) -> bool {
        self.resolved_after_turn <= state.completed_turns
            && state
                .completed_turns
                .saturating_sub(self.resolved_after_turn)
                <= 1
            && state.epistemic_ledger.records.iter().any(|r| {
                r.belief_id == self.belief_id
                    && r.introduced_turn == self.introduced_turn
                    && if let Some(shared) = &self.shared_proposition {
                        shared.validate_in(state)
                    } else if matches!(
                        self.slot,
                        crate::proposition_content::ContentSlotIR::Source
                            | crate::proposition_content::ContentSlotIR::Summary
                    ) {
                        source_record_available(r)
                    } else {
                        contextual_record_available(r)
                    }
            })
    }
}

fn contextual_record_available(record: &BeliefRecordIR) -> bool {
    record.status == BeliefRecordStatusIR::Active
        && record.signature.modal_world == ModalWorldIR::Actual
        && record.proposition_polarity
            == crate::attribution::AttributedPropositionPolarityIR::Positive
        && record.content.interaction_preference.is_none()
        && record.content.validate_source(&record.proposition_surface)
}

/// Choose the referent before looking for the requested relation. A newer
/// proposition without a cause must never make an older cause look relevant.
fn contextual_record(state: &ConversationStateIR) -> Option<&BeliefRecordIR> {
    contextual_record_for(state, contextual_record_available)
}

fn source_record_available(record: &BeliefRecordIR) -> bool {
    // A disputed proposition still has a recorded source. Disagreement is
    // not retraction and cannot erase who made the attributed statement.
    record.status.is_reference_active()
        && record.content.interaction_preference.is_none()
        && record.content.validate_source(&record.proposition_surface)
}

fn source_context_target(state: &ConversationStateIR) -> Option<ContextualContentTargetIR> {
    let shared = state
        .answer_focus
        .as_ref()
        .filter(|f| {
            f.validate(state.completed_turns)
                && state.completed_turns.saturating_sub(f.answered_turn) <= 3
                && !state.epistemic_ledger.records.iter().any(|r| {
                    r.introduced_turn > f.answered_turn
                        && r.content.interaction_preference.is_none()
                })
        })
        .and_then(|f| f.shared_proposition.as_ref());
    let (meaning, fallback) = if let Some(shared) = shared {
        if !shared.validate_in(state) {
            return None;
        }
        (Some(shared.meaning.clone()), None)
    } else {
        let record = contextual_record_for(state, source_record_available)?;
        (PropositionMeaningIR::from_record(record), Some(record))
    };
    let mut records = if let Some(meaning) = &meaning {
        state
            .epistemic_ledger
            .records
            .iter()
            .filter(|r| {
                source_record_available(r)
                    && PropositionMeaningIR::from_record(r).as_ref() == Some(meaning)
            })
            .collect::<Vec<_>>()
    } else {
        vec![fallback?]
    };
    if records.is_empty() || records.len() > MAX_ANSWER_EVIDENCE {
        return None;
    }
    records.sort_by(|a, b| a.belief_id.cmp(&b.belief_id));
    let first = records[0];
    Some(ContextualContentTargetIR {
        shared_proposition: if records.len() > 1 {
            Some(SharedPropositionFocusIR {
                meaning: meaning?,
                belief_ids: records.iter().map(|r| r.belief_id.clone()).collect(),
            })
        } else {
            None
        },
        belief_id: first.belief_id.clone(),
        slot: crate::proposition_content::ContentSlotIR::Source,
        resolved_after_turn: state.completed_turns,
        introduced_turn: first.introduced_turn,
    })
}

fn contextual_record_for(
    state: &ConversationStateIR,
    available: fn(&BeliefRecordIR) -> bool,
) -> Option<&BeliefRecordIR> {
    let focus = state.answer_focus.as_ref().filter(|f| {
        f.validate(state.completed_turns)
            && state.completed_turns.saturating_sub(f.answered_turn) <= 3
    });
    let focused_id = focus.and_then(|f| {
        f.proposition_belief_id
            .as_deref()
            .or_else(|| f.described_event.as_ref().map(|e| e.belief_id.as_str()))
    });
    let newer_records = focus.is_some_and(|f| {
        state.epistemic_ledger.records.iter().any(|r| {
            r.introduced_turn > f.answered_turn
                && r.introduced_turn <= state.completed_turns
                && r.content.interaction_preference.is_none()
        })
    });
    if let Some(id) = focused_id.filter(|_| !newer_records) {
        return state
            .epistemic_ledger
            .records
            .iter()
            .find(|r| r.belief_id == id && available(r));
    }
    // Include unavailable/modal candidates in ambiguity detection. Their
    // absence of usable evidence cannot select some other candidate for us.
    let mut records = state
        .epistemic_ledger
        .records
        .iter()
        .filter(|r| {
            r.introduced_turn == state.completed_turns && r.content.interaction_preference.is_none()
        })
        .collect::<Vec<_>>();
    let all = records.clone();
    records.retain(|part| {
        !all.iter().any(|whole| {
            whole.belief_id != part.belief_id
                && whole.source_actor == part.source_actor
                && whole.signature.modal_world == part.signature.modal_world
                && whole.proposition_polarity == part.proposition_polarity
                && {
                    let span = crate::proposition_content::reported_event_surface(
                        &whole.proposition_surface,
                    );
                    span != whole.proposition_surface
                        && [
                            Some(span),
                            whole.proposition_surface.strip_prefix(span),
                            whole.proposition_surface.strip_suffix(span),
                        ]
                        .into_iter()
                        .flatten()
                        .any(|component| {
                            component
                                .trim()
                                .trim_end_matches(['.', '!', '?'])
                                .eq_ignore_ascii_case(
                                    part.proposition_surface
                                        .trim()
                                        .trim_end_matches(['.', '!', '?']),
                                )
                        })
                }
        })
    });
    (records.len() == 1)
        .then(|| records[0])
        .filter(|r| available(r))
}

/// Small compositional metalanguage grammar: operation + optional reference,
/// repetition and manner. Any content word/new topic or negation rejects the
/// binding. This does not dispatch whole sentences or supply answer content.
pub(crate) fn is_answer_reformulation(text: &str) -> bool {
    let lower = text.to_lowercase();
    let tokens = lower
        .split(|c: char| c.is_whitespace() || matches!(c, '.' | '?' | '!' | ','))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() || tokens.len() > 24 {
        return false;
    }
    let operation = tokens.iter().any(|word| {
        matches!(
            *word,
            "explain"
                | "repeat"
                | "rephrase"
                | "restate"
                | "summarize"
                | "설명해"
                | "설명해줘"
                | "말해"
                | "말해줘"
                | "요약해"
                | "요약해줘"
        )
    });
    let backward = tokens.iter().any(|word| {
        matches!(
            *word,
            "again"
                | "that"
                | "it"
                | "previous"
                | "repeat"
                | "rephrase"
                | "restate"
                | "다시"
                | "그걸"
                | "그것을"
                | "이전"
                | "방금"
                | "아까"
        )
    });
    operation
        && backward
        && tokens.iter().all(|word| {
            matches!(
                *word,
                "explain"
                    | "repeat"
                    | "rephrase"
                    | "restate"
                    | "summarize"
                    | "again"
                    | "that"
                    | "it"
                    | "previous"
                    | "answer"
                    | "explanation"
                    | "the"
                    | "your"
                    | "please"
                    | "briefly"
                    | "simply"
                    | "clearly"
                    | "in"
                    | "detail"
                    | "more"
                    | "less"
                    | "설명해"
                    | "설명해줘"
                    | "말해"
                    | "말해줘"
                    | "요약해"
                    | "요약해줘"
                    | "다시"
                    | "그걸"
                    | "그것을"
                    | "이전"
                    | "방금"
                    | "아까"
                    | "답변을"
                    | "설명을"
                    | "핵심만"
                    | "짧게"
                    | "간단히"
                    | "자세히"
                    | "좀"
            )
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseQueryKindIR {
    SourceContent,
    PropositionSources,
    ActualityStatus,
    ModalStatus,
    ConflictStatus,
    PresuppositionCheck,
    MissingExplanationTarget,
    MissingComparisonOperands,
}

/// Recognize an unfilled argument, not a failure to know the answer. Every
/// remaining token must be grammatical/deictic; an unfamiliar content word
/// conservatively counts as an explicit target, never as an empty slot.
pub(crate) fn unbound_explanation_target(text: &str) -> Option<DiscourseQueryKindIR> {
    if text.contains(['"', '“', '”', '`']) {
        return None;
    }
    let lower = text.to_lowercase();
    let preference = crate::proposition_content::interaction_preference(text);
    let mut clause = preference
        .as_ref()
        .filter(|p| p.desired == crate::proposition_content::InteractionModeIR::Explanation)
        .map_or(lower.as_str(), |p| p.desired_surface.as_str())
        .trim()
        .trim_end_matches(['.', '?', '!']);
    if clause.contains(['.', ';', ',']) {
        return None;
    }
    for prefix in ["this time ", "이번엔 ", "지금은 "] {
        clause = clause.strip_prefix(prefix).unwrap_or(clause);
    }
    let words = clause.split_whitespace().collect::<Vec<_>>();
    let explanation = words.iter().any(|w| {
        matches!(*w, "why" | "explain" | "explanation" | "왜")
            || w.strip_prefix("설명").is_some_and(|ending| {
                matches!(
                    ending,
                    "" | "만" | "을" | "해줘" | "해주세요" | "해줄래" | "해줄래요"
                )
            })
    });
    if !explanation || words.is_empty() {
        return None;
    }
    let comparison = words.iter().any(|w| {
        matches!(
            *w,
            "difference" | "differences" | "차이" | "차이를" | "차이가" | "차이는"
        )
    });
    let grammatical_only = words.iter().all(|w| {
        matches!(
            *w,
            "왜" | "이런"
                | "그런"
                | "이"
                | "그"
                | "그거"
                | "그걸"
                | "그것을"
                | "이유"
                | "이유도"
                | "이유를"
                | "차이"
                | "차이를"
                | "차이가"
                | "차이는"
                | "나는지"
                | "좀"
                | "다시"
                | "그냥"
                | "듣고"
                | "싶어"
                | "싶어요"
                | "설명"
                | "설명만"
                | "설명을"
                | "설명해줘"
                | "설명해주세요"
                | "설명해줄래"
                | "설명해줄래요"
                | "why"
                | "explain"
                | "explanation"
                | "please"
                | "just"
                | "again"
                | "can"
                | "could"
                | "would"
                | "you"
                | "to"
                | "me"
                | "it"
                | "that"
                | "this"
                | "the"
                | "a"
                | "reason"
                | "difference"
                | "differences"
        )
    });
    grammatical_only.then_some(if comparison {
        DiscourseQueryKindIR::MissingComparisonOperands
    } else {
        DiscourseQueryKindIR::MissingExplanationTarget
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryTemporalScopeIR {
    Current,
    Historical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PresuppositionKindIR {
    EventOccurred,
    StateHolds,
    FactiveComplement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresuppositionIR {
    pub kind: PresuppositionKindIR,
    pub surface_text: String,
    pub dialogue_truth_established: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseQueryIR {
    pub schema: String,
    pub original_text: String,
    pub kind: DiscourseQueryKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_source: Option<String>,
    #[serde(default)]
    pub requested_attitudes: Vec<AttributionAttitudeIR>,
    #[serde(default)]
    pub topic_terms: Vec<String>,
    pub temporal_scope: QueryTemporalScopeIR,
    #[serde(default)]
    pub presuppositions: Vec<PresuppositionIR>,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseAnswerDispositionIR {
    AnsweredFromDialogueRecords,
    MultipleDialogueRecords,
    ConflictingDialogueRecords,
    NoConflictRecorded,
    DialogueTruthNotEstablished,
    PresuppositionUnverified,
    NoMatchingRecord,
    AmbiguousQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnswerClaimKindIR {
    SourceAttributedContent,
    SourceAttitude,
    ModalWorldClassification,
    ConflictObserved,
    NoConflictObserved,
    DialogueTruthNotEstablished,
    PresuppositionNotEstablished,
    NoMatchingDialogueRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseAnswerEvidenceIR {
    pub belief_id: String,
    pub source_actor: String,
    pub proposition_surface: String,
    pub attitude: AttributionAttitudeIR,
    pub epistemic_status: EpistemicStatusIR,
    pub modal_world: ModalWorldIR,
    pub record_status: BeliefRecordStatusIR,
    pub introduced_turn: u64,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseAnswerClaimIR {
    pub claim_id: String,
    pub kind: AnswerClaimKindIR,
    pub subject: String,
    pub value: String,
    #[serde(default)]
    pub evidence_belief_ids: Vec<String>,
}

/// Information structure, not truth status. The speaker need not repeat the
/// already shared source of an ordinary recall answer. Attribution remains in
/// the claim/evidence IR, and non-shared or qualified sources stay explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerSourceFramingIR {
    Explicit,
    SharedDialogueRecall,
}

/// A retained planning product, not cached answer text or an execution receipt.
/// The owning conversation state controls whether any recorded goal is still live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedPlanIR {
    pub conversation_id: String,
    pub semantic_goal: dockable_semantic_core::SemanticPlanGoalIR,
    pub bundle: dockable_semantic_core::SemanticPlanBundleIR,
    pub discourse_goals: Vec<crate::conversation::ConversationGoalFrameIR>,
}

impl RecordedPlanIR {
    pub fn validate(&self) -> bool {
        !self.conversation_id.is_empty()
            && self.bundle.validate_against(&self.semantic_goal)
            && self.discourse_goals.len() == self.bundle.plans.len()
            && !self.discourse_goals.is_empty()
            && self
                .discourse_goals
                .iter()
                .zip(&self.bundle.plans)
                .enumerate()
                .all(|(index, (g, p))| {
                    g.intent == p.intent
                        && !p.steps.is_empty()
                        && p.steps.iter().all(|s| s.target == g.subject)
                        && self.semantic_goal.events.iter().any(|event| {
                            event.event_id == self.semantic_goal.selected_live_event_ids[index]
                                && event
                                    .predicate_concept_id
                                    .strip_prefix("C_")
                                    .unwrap_or(&event.predicate_concept_id)
                                    == g.canonical_predicate
                        })
                })
    }

    fn method_target(
        &self,
        text: &str,
        analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
    ) -> Option<usize> {
        use crate::compositional_semantics::{
            FrameMoodIR, FramePolarityIR, FrameTemporalReferenceIR,
        };
        if crate::proposition_content::requested_content_slots(text)
            != [crate::proposition_content::ContentSlotIR::Manner]
            || text.contains(['"', '“', '”', '`'])
            || analysis.frames.len() != 1
        {
            return None;
        }
        let frame = &analysis.frames[0];
        if frame.mood != FrameMoodIR::Interrogative
            || frame.polarity != FramePolarityIR::Positive
            || frame.temporal_reference == FrameTemporalReferenceIR::Past
            || frame.embedded_under_quote
            || !analysis
                .clause_graph
                .node_for_frame(&frame.frame_id)?
                .function
                .permits_independent_directive()
        {
            return None;
        }
        let roles = &analysis.semantic_role_graph;
        if !roles.quantifier_scopes.is_empty() {
            return None;
        }
        let event = roles
            .nodes
            .iter()
            .find(|n| n.source_frame_id.as_deref() == Some(frame.frame_id.as_str()))?;
        let mut targets_in_question = Vec::new();
        for edge in roles
            .role_edges
            .iter()
            .filter(|e| e.event_node_id == event.node_id)
        {
            let argument = roles
                .nodes
                .iter()
                .find(|n| n.node_id == edge.argument_node_id)?;
            match edge.role {
                crate::semantic_roles::SemanticRoleKindIR::Agent => {
                    if !crate::proposition_content::speaker_or_addressee_reference(
                        &argument.normalized_label,
                    ) {
                        return None;
                    }
                }
                crate::semantic_roles::SemanticRoleKindIR::Theme
                | crate::semantic_roles::SemanticRoleKindIR::Patient => {
                    targets_in_question.push(argument.normalized_label.as_str())
                }
                _ => return None,
            }
        }
        if targets_in_question.len() > 1
            || (!frame.theme.is_empty() && targets_in_question.is_empty())
        {
            return None;
        }
        let mut targets = self.discourse_goals.iter().enumerate().filter(|(_, g)| {
            g.canonical_predicate == frame.canonical_predicate
                && targets_in_question.first().is_none_or(|target| {
                    crate::semantic_roles::normalize_argument(&g.subject) == *target
                })
        });
        let (index, _) = targets.next()?;
        targets.next().is_none().then_some(index)
    }

    pub(crate) fn answer_method(
        &self,
        text: &str,
        analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
        state: &ConversationStateIR,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        let selected_index = self.method_target(text, analysis)?;
        let method = PlanMethodAnswerIR {
            recorded: self.clone(),
            selected_index,
        };
        if !method.matches_active_state(state) || !method.validate(text) {
            return None;
        }
        let mut answer = DiscourseQaEngine.unanswered(text, language);
        answer.plan_method = Some(Box::new(method));
        answer.disposition = DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
        answer.claims.clear();
        answer.refresh_structured_preview();
        answer.validate().then_some(answer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMethodAnswerIR {
    pub recorded: RecordedPlanIR,
    pub selected_index: usize,
}

impl PlanMethodAnswerIR {
    pub fn validate(&self, source: &str) -> bool {
        self.recorded.validate()
            && self.recorded.method_target(
                source,
                &crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(source),
            ) == Some(self.selected_index)
    }

    pub(crate) fn matches_active_state(&self, state: &ConversationStateIR) -> bool {
        self.recorded.conversation_id == state.conversation_id
            && self
                .recorded
                .discourse_goals
                .get(self.selected_index)
                .is_some_and(|goal| {
                    state.active_goals.iter().any(|g| {
                        g.goal_id == goal.goal_id
                            && g.subject == goal.subject
                            && g.canonical_predicate == goal.canonical_predicate
                    })
                })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseAnswerIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_method: Option<Box<PlanMethodAnswerIR>>,
    /// The understood question survives both successful retrieval and absence.
    /// It is query structure, never an observed event or an answer claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub described_query: Option<crate::proposition_content::DescribedEventIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub korean_nominal_forms: Vec<crate::korean_nominal::KoreanNominalFormIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_gap: Option<crate::proposition_content::EventReferenceGapIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_constraint_conflict: Option<ResponseConstraintConflictIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_request: Option<crate::proposition_content::QuestionRequestIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_request: Option<crate::proposition_content::ContentRequestIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contextual_target: Option<ContextualContentTargetIR>,
    /// Ordered answers to independently requested matrix operations. Nested
    /// batches are forbidden; each leaf retains its own query and evidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub response_parts: Vec<DiscourseAnswerIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_summary: Option<crate::generative_language::EventSummaryIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_inquiry: Option<crate::utterance_intent::DecisionInquiryIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reformulated_request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_projection: Option<crate::proposition_content::ContentProjectionIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_reasoning: Option<crate::world_dialogue::WorldReasoningIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_memory_update: Option<crate::world_dialogue::WorldMemoryUpdateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_clarification: Option<crate::world_dialogue::WorldClarificationIR>,
    pub schema: String,
    pub query: DiscourseQueryIR,
    pub disposition: DiscourseAnswerDispositionIR,
    pub evidence: Vec<DiscourseAnswerEvidenceIR>,
    pub claims: Vec<DiscourseAnswerClaimIR>,
    pub language: LanguageCodeIR,
    /// Legacy diagnostic view, not the final conversational output. Batch
    /// answers carry a canonical meaning preview; the language cortex builds
    /// the actual reply after discourse and affective policy are selected.
    pub realized_text: String,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
    pub unsupported_claims: usize,
}

impl DiscourseAnswerIR {
    /// The requested relation remains meaningful even when its value is absent.
    /// No missing slot establishes or denies the event described by a question.
    pub(crate) fn unknown_content_slot(&self) -> Option<crate::proposition_content::ContentSlotIR> {
        if self.disposition != DiscourseAnswerDispositionIR::NoMatchingRecord
            || !self.evidence.is_empty()
            || self.content_projection.is_some()
        {
            return None;
        }
        let slots = crate::proposition_content::requested_content_slots(&self.query.original_text);
        (slots.len() == 1).then(|| slots[0])
    }
    pub(crate) fn missing_property_owner(&self) -> Option<&str> {
        use crate::proposition_content::{ContentSlotIR, DescriptionKindIR};
        if self.disposition != DiscourseAnswerDispositionIR::NoMatchingRecord
            || !self.evidence.is_empty()
            || self.content_projection.is_some()
        {
            return None;
        }
        let query = self.described_query.as_ref()?;
        (query.kind == DescriptionKindIR::State
            && query.lexical_entry_ids.is_empty()
            && !query.negated
            && !query.has_references()
            && query.roles.len() == 1)
            .then(|| query.roles.get(&ContentSlotIR::Theme).map(String::as_str))
            .flatten()
    }

    /// A view of already selected speech-content evidence, not new reasoning.
    /// Qualification stays on the full ledger path when a simple attributed
    /// clause cannot express it faithfully.
    pub(crate) fn spoken_content(&self) -> Option<Vec<crate::generative_language::EventSummaryIR>> {
        if self.query.kind != DiscourseQueryKindIR::SourceContent
            || !matches!(
                self.disposition,
                DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                    | DiscourseAnswerDispositionIR::MultipleDialogueRecords
            )
            || self.content_projection.is_some()
            || self.evidence.is_empty()
            || self.claims != answer_claims(&self.query, &self.evidence).1
        {
            return None;
        }
        self.evidence
            .iter()
            .map(|e| {
                if !SPEECH_REPORT_ATTITUDES.contains(&e.attitude)
                    || e.epistemic_status != EpistemicStatusIR::Reported
                    || e.modal_world != ModalWorldIR::Actual
                    || e.record_status != BeliefRecordStatusIR::Active
                {
                    return None;
                }
                let event =
                    crate::proposition_content::described_event(&e.proposition_surface, false)?;
                if event.has_unbound_participant() {
                    return None;
                }
                Some(crate::generative_language::EventSummaryIR {
                    omitted_roles: vec![],
                    belief_id: e.belief_id.clone(),
                    source_actor: e.source_actor.clone(),
                    source_proposition: e.proposition_surface.clone(),
                    context_sources: vec![],
                    event,
                })
            })
            .collect()
    }

    pub(crate) fn focused_shared_proposition(
        &self,
        state: &ConversationStateIR,
    ) -> Option<SharedPropositionFocusIR> {
        if self.evidence.len() < 2
            || self.focused_event().is_some()
            || self.focused_proposition().is_some()
        {
            return None;
        }
        let mut records = self
            .evidence
            .iter()
            .map(|e| {
                let r = state.epistemic_ledger.record(&e.belief_id)?;
                (evidence_from_record(r) == *e).then_some(r)
            })
            .collect::<Option<Vec<_>>>()?;
        records.sort_by(|a, b| a.belief_id.cmp(&b.belief_id));
        let meaning = PropositionMeaningIR::from_record(records[0])?;
        if records
            .iter()
            .any(|r| PropositionMeaningIR::from_record(r).as_ref() != Some(&meaning))
        {
            return None;
        }
        let focus = SharedPropositionFocusIR {
            meaning,
            belief_ids: records.iter().map(|r| r.belief_id.clone()).collect(),
        };
        focus.validate().then_some(focus)
    }
    /// The source operator asks for an actor, not a repeat of the proposition.
    /// Only a shared, identified proposition licenses a coordinated actor answer;
    /// distinct propositions retain their source/content associations in output.
    /// These values are selected from evidence, never from question tokens.
    pub(crate) fn focused_source_values(&self) -> Option<BTreeMap<String, BTreeSet<String>>> {
        if self.query.kind != DiscourseQueryKindIR::PropositionSources
            || !matches!(
                self.disposition,
                DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                    | DiscourseAnswerDispositionIR::MultipleDialogueRecords
            )
            || self.content_projection.is_some()
            || self.event_summary.is_some()
            || !self.response_parts.is_empty()
            || self.claims != answer_claims(&self.query, &self.evidence).1
        {
            return None;
        }
        let proposition = &self.evidence.first()?.proposition_surface;
        if self.evidence.iter().any(|e| {
            e.record_status != BeliefRecordStatusIR::Active
                || matches!(
                    e.epistemic_status,
                    EpistemicStatusIR::Denied | EpistemicStatusIR::Doubted
                )
                || e.proposition_surface != *proposition
                || (!self.query.requested_attitudes.is_empty()
                    && !self.query.requested_attitudes.contains(&e.attitude))
        }) {
            return None;
        }
        let mut sources = BTreeMap::<String, (String, BTreeSet<String>)>::new();
        for e in &self.evidence {
            sources
                .entry(normalize_actor(&e.source_actor))
                .or_insert_with(|| (e.source_actor.clone(), BTreeSet::new()))
                .1
                .insert(e.belief_id.clone());
        }
        Some(sources.into_values().collect())
    }

    pub(crate) fn structured_meaning_preview(&self) -> Option<String> {
        let kind = if self.plan_method.is_some() {
            "RECORDED_PLAN_METHOD"
        } else if self.event_summary.is_some() {
            "EVENT_RECAP"
        } else if self.decision_inquiry.is_some() {
            if self
                .decision_inquiry
                .as_ref()
                .is_some_and(|i| i.assessment.is_some())
            {
                "CONDITIONAL_ACTION_BENEFIT"
            } else {
                "DECISION_INPUT_GAP"
            }
        } else if self.world_clarification.is_some() {
            "WORLD_REFERENCE_GAP"
        } else if self.world_memory_update.is_some() {
            "WORLD_MEMORY_UPDATE"
        } else if self.world_reasoning.is_some() {
            "WORLD_DECISION"
        } else if matches!(
            self.query.kind,
            DiscourseQueryKindIR::MissingExplanationTarget
                | DiscourseQueryKindIR::MissingComparisonOperands
        ) {
            "INFORMATION_TARGET_GAP"
        } else {
            return None;
        };
        Some(format!("{kind}: {}", self.query.original_text))
    }

    pub(crate) fn refresh_structured_preview(&mut self) {
        if let Some(preview) = self.structured_meaning_preview() {
            self.realized_text = preview;
        }
    }

    pub(crate) fn source_framing(&self) -> AnswerSourceFramingIR {
        let Some(projection) = &self.content_projection else {
            return AnswerSourceFramingIR::Explicit;
        };
        let recalled_cause = projection.binding.slot
            == crate::proposition_content::ContentSlotIR::Cause
            && self.query.requested_source.as_deref() == Some("DIALOGUE_USER")
            && crate::proposition_content::self_reported_cause_target(&self.query.original_text)
                .is_some();
        let shared = (projection.binding.event_id.is_some() || recalled_cause)
            && !self.dialogue_truth_established
            && !self.external_execution_authorized
            && self.query.requested_attitudes.is_empty()
            && self.query.presuppositions.is_empty()
            && !self.evidence.is_empty()
            && self.evidence.iter().all(|e| {
                e.source_actor == "DIALOGUE_USER"
                    && e.attitude == AttributionAttitudeIR::Say
                    && e.epistemic_status == EpistemicStatusIR::Reported
                    && e.modal_world == ModalWorldIR::Actual
                    && e.record_status == BeliefRecordStatusIR::Active
                    && !e.dialogue_truth_established
                    && !e.external_execution_authorized
            })
            && projection.all_projections().all(|p| {
                p.source_actor == "DIALOGUE_USER"
                    && self.evidence.iter().any(|e| {
                        e.belief_id == p.belief_id && e.proposition_surface == p.source_proposition
                    })
            });
        if shared {
            AnswerSourceFramingIR::SharedDialogueRecall
        } else {
            AnswerSourceFramingIR::Explicit
        }
    }
    pub(crate) fn focused_proposition(&self) -> Option<String> {
        let mut ids = self
            .evidence
            .iter()
            .map(|e| &e.belief_id)
            .collect::<BTreeSet<_>>();
        if ids.len() == 1 {
            ids.pop_first().cloned()
        } else {
            None
        }
    }
    pub(crate) fn focused_event(&self) -> Option<DescribedEventReferenceIR> {
        if std::iter::once(self).chain(&self.response_parts).any(|a| {
            a.content_projection
                .as_ref()
                .is_some_and(|p| !p.co_answers.is_empty())
        }) {
            return None;
        }
        let mut events = std::iter::once(self)
            .chain(&self.response_parts)
            .filter_map(|answer| {
                answer
                    .event_summary
                    .as_ref()
                    .map(|s| DescribedEventReferenceIR {
                        belief_id: s.belief_id.clone(),
                        event_id: s.event.event_id.clone(),
                    })
                    .or_else(|| {
                        answer.content_projection.as_ref().and_then(|p| {
                            if !p.co_answers.is_empty() {
                                return None;
                            }
                            p.binding
                                .event_id
                                .as_ref()
                                .map(|id| DescribedEventReferenceIR {
                                    belief_id: p.belief_id.clone(),
                                    event_id: id.clone(),
                                })
                        })
                    })
            });
        let first = events.next()?;
        events.all(|event| event == first).then_some(first)
    }

    pub fn validate(&self) -> bool {
        if let Some(method) = &self.plan_method {
            let mut expected =
                DiscourseQaEngine.unanswered(&self.query.original_text, self.language);
            expected.plan_method = Some(method.clone());
            expected.disposition = DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
            expected.claims.clear();
            expected.refresh_structured_preview();
            return method.validate(&self.query.original_text) && self == &expected;
        }
        if (self.described_query.is_some()
            || self.disposition == DiscourseAnswerDispositionIR::NoMatchingRecord)
            && self.described_query
                != crate::proposition_content::described_event(&self.query.original_text, true)
        {
            return false;
        }
        if self.korean_nominal_forms.len() > 64
            || self
                .korean_nominal_forms
                .iter()
                .any(|f| !f.validate() || !self.evidence.iter().any(|e| f.matches(&e.source_actor)))
            || !self.korean_nominal_forms.windows(2).all(|p| p[0] < p[1])
        {
            return false;
        }
        if self.structured_meaning_preview().is_some_and(|preview| {
            self.realized_text != preview
                || !matches!(
                    self.language,
                    LanguageCodeIR::Korean | LanguageCodeIR::English
                )
        }) {
            return false;
        }
        if let Some(gap) = &self.reference_gap {
            return gap.choices().is_some()
                && self.schema == DISCOURSE_ANSWER_SCHEMA
                && self.query
                    == DiscourseQaEngine
                        .unanswered(&gap.source_question, self.language)
                        .query
                && self.disposition == DiscourseAnswerDispositionIR::AmbiguousQuery
                && self.evidence.is_empty()
                && self.claims.is_empty()
                && self.response_parts.is_empty()
                && self.content_projection.is_none()
                && self.content_request.is_none()
                && self.question_request.is_none()
                && self.contextual_target.is_none()
                && self.event_summary.is_none()
                && self.world_reasoning.is_none()
                && self.world_memory_update.is_none()
                && self.world_clarification.is_none()
                && self.decision_inquiry.is_none()
                && self.reformulated_request.is_none()
                && self.response_constraint_conflict.is_none()
                && !self.dialogue_truth_established
                && !self.external_execution_authorized
                && self.unsupported_claims == 0;
        }
        if let Some(conflict) = &self.response_constraint_conflict {
            return conflict.validate()
                && self.schema == DISCOURSE_ANSWER_SCHEMA
                && self.query
                    == DiscourseQaEngine
                        .unanswered(&conflict.query_text, self.language)
                        .query
                && self.disposition == DiscourseAnswerDispositionIR::AmbiguousQuery
                && self.evidence.is_empty()
                && self.claims.is_empty()
                && self.response_parts.is_empty()
                && self.question_request.is_none()
                && self.content_request.is_none()
                && self.contextual_target.is_none()
                && self.content_projection.is_none()
                && self.event_summary.is_none()
                && self.world_reasoning.is_none()
                && self.world_memory_update.is_none()
                && self.world_clarification.is_none()
                && self.decision_inquiry.is_none()
                && self.reformulated_request.is_none()
                && !self.dialogue_truth_established
                && !self.external_execution_authorized
                && self.unsupported_claims == 0;
        }
        if self.question_request.as_ref().is_some_and(|request| {
            !request.validate()
                || request.question_text != self.query.original_text
                || !self.response_parts.is_empty()
                || self.reformulated_request.is_some()
                || self.world_reasoning.is_some()
                || self.world_memory_update.is_some()
                || self.world_clarification.is_some()
                || self.decision_inquiry.is_some()
        }) {
            return false;
        }
        if !self.response_parts.is_empty() {
            return self.validate_response_parts();
        }
        if self.content_request
            != crate::proposition_content::content_request(&self.query.original_text)
        {
            return false;
        }
        if self.content_request.as_ref().is_some_and(|request| {
            !request.validate()
                || request.source_text != self.query.original_text
                || self
                    .content_projection
                    .as_ref()
                    .is_some_and(|p| p.binding.slot != request.slot)
                || (request.target_surface.is_some() && self.contextual_target.is_some())
        }) {
            return false;
        }
        if self
            .contextual_target
            .as_ref()
            .is_some_and(|t| !t.validates_answer(self))
        {
            return false;
        }
        if let Some(summary) = &self.event_summary {
            if !summary.can_realize(self.language)
                || self.content_projection.is_some()
                || self.world_reasoning.is_some()
                || self.world_memory_update.is_some()
                || self.world_clarification.is_some()
                || self.decision_inquiry.is_some()
                || self.reformulated_request.is_some()
                || self.disposition != DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                || crate::proposition_content::requested_content_slot(&self.query.original_text)
                    != Some(crate::proposition_content::ContentSlotIR::Summary)
                || !crate::proposition_content::is_deictic_event_recap(&self.query.original_text)
                || self.evidence.len() != 1
                || !self.evidence.iter().any(|e| {
                    e.belief_id == summary.belief_id
                        && e.source_actor == summary.source_actor
                        && e.proposition_surface == summary.source_proposition
                        && e.modal_world == ModalWorldIR::Actual
                })
                || self.claims
                    != vec![claim(
                        1,
                        AnswerClaimKindIR::SourceAttributedContent,
                        &summary.source_actor,
                        &serde_json::to_string(&summary.event).unwrap_or_default(),
                        vec![summary.belief_id.clone()],
                    )]
            {
                return false;
            }
        }
        if matches!(
            self.query.kind,
            DiscourseQueryKindIR::MissingExplanationTarget
                | DiscourseQueryKindIR::MissingComparisonOperands
        ) && self
            .decision_inquiry
            .as_ref()
            .is_none_or(|i| i.explanation_of.is_none())
            && (unbound_explanation_target(&self.query.original_text) != Some(self.query.kind)
                || self.disposition != DiscourseAnswerDispositionIR::AmbiguousQuery
                || !self.evidence.is_empty()
                || !self.claims.is_empty()
                || !self.query.topic_terms.is_empty()
                || self.content_projection.is_some()
                || self.world_reasoning.is_some()
                || self.world_memory_update.is_some()
                || self.world_clarification.is_some()
                || self.decision_inquiry.is_some()
                || self.reformulated_request.is_some())
        {
            return false;
        }
        if let Some(inquiry) = &self.decision_inquiry {
            if !inquiry.validate()
                || self.world_clarification.is_some()
                || self.world_memory_update.is_some()
                || self.world_reasoning.is_some()
                || self.content_projection.is_some()
                || self.reformulated_request.is_some()
                || !self.evidence.is_empty()
                || !self.claims.is_empty()
                || self.query.original_text != inquiry.source_text
                || self.disposition
                    != if inquiry.assessment.is_some() {
                        DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                    } else {
                        DiscourseAnswerDispositionIR::AmbiguousQuery
                    }
            {
                return false;
            }
        }
        if let Some(c) = &self.world_clarification {
            if !c.validate()
                || self.world_memory_update.is_some()
                || self.world_reasoning.is_some()
                || self.content_projection.is_some()
                || self.reformulated_request.is_some()
                || !self.evidence.is_empty()
                || !self.claims.is_empty()
                || self.query.original_text != c.response_source()
                || self.disposition != DiscourseAnswerDispositionIR::AmbiguousQuery
            {
                return false;
            }
        }
        if let Some(update) = &self.world_memory_update {
            if self.world_reasoning.is_some()
                || self.content_projection.is_some()
                || self.reformulated_request.is_some()
                || !self.evidence.is_empty()
                || !self.claims.is_empty()
                || update.source_text != self.query.original_text
                || self.disposition != DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                || !crate::generative_language::world_update_language_available(
                    self.language,
                    update,
                )
            {
                return false;
            }
        }
        if let Some(world) = &self.world_reasoning {
            if self.content_projection.is_some()
                || self.reformulated_request.is_some()
                || !self.evidence.is_empty()
                || !self.claims.is_empty()
                || self.disposition != world.answer_disposition()
                || !world.matches_question(&self.query.original_text)
                || !crate::generative_language::world_decision_language_available(
                    self.language,
                    world,
                )
            {
                return false;
            }
        }
        if self
            .reformulated_request
            .as_ref()
            .is_some_and(|text| !is_answer_reformulation(text))
        {
            return false;
        }
        if let Some(projection) = &self.content_projection {
            if projection
                .all_projections()
                .any(|p| p.elaboration_event.is_some())
                && !projection
                    .all_projections()
                    .all(|p| p.elaboration_event.is_some())
            {
                return false;
            }
            if !projection.validate()
                || !projection.all_projections().all(|p| {
                    self.evidence.iter().any(|evidence| {
                        evidence.belief_id == p.belief_id
                            && evidence.source_actor == p.source_actor
                            && evidence.proposition_surface == p.source_proposition
                    })
                })
            {
                return false;
            }
            if projection.binding.event_id.is_some()
                && !projection.matches_question(&self.query.original_text)
            {
                return false;
            }
            if crate::proposition_content::requested_content_slot(&self.query.original_text)
                != Some(projection.binding.slot)
                || self.claims.len()
                    != projection
                        .all_projections()
                        .map(|p| {
                            p.all_bindings().count() + usize::from(p.elaboration_event.is_some())
                        })
                        .sum::<usize>()
                || !projection.all_projections().all(|p| {
                    let detail_valid = p.elaboration_event.as_ref().is_none_or(|event| {
                        self.question_request
                            .as_ref()
                            .and_then(|q| q.response_manner)
                            == Some(crate::proposition_content::ResponseMannerIR::Detailed)
                            && serde_json::to_string(event).is_ok_and(|value| {
                                self.claims.iter().any(|claim| {
                                    claim.kind == AnswerClaimKindIR::SourceAttributedContent
                                        && claim.subject == p.source_actor
                                        && claim.value == value
                                        && claim.evidence_belief_ids == [p.belief_id.clone()]
                                })
                            })
                    });
                    detail_valid
                        && p.all_bindings().all(|binding| {
                            self.claims.iter().any(|claim| {
                                claim.kind == AnswerClaimKindIR::SourceAttributedContent
                                    && claim.subject == p.source_actor
                                    && claim.value == binding.value
                                    && claim.evidence_belief_ids == [p.belief_id.clone()]
                            })
                        })
                })
            {
                return false;
            }
        }
        if self.schema != DISCOURSE_ANSWER_SCHEMA
            || self.query.schema != DISCOURSE_QUERY_SCHEMA
            || self.evidence.len() > MAX_ANSWER_EVIDENCE
            || self.claims.len() > MAX_ANSWER_CLAIMS
            || self.realized_text.trim().is_empty()
            || self.dialogue_truth_established
            || self.external_execution_authorized
            || self.unsupported_claims != 0
            || self
                .query
                .presuppositions
                .iter()
                .any(|item| item.dialogue_truth_established)
        {
            return false;
        }
        let evidence_ids = self
            .evidence
            .iter()
            .map(|item| item.belief_id.as_str())
            .collect::<BTreeSet<_>>();
        if evidence_ids.len() != self.evidence.len()
            || self.evidence.iter().any(|item| {
                item.belief_id.trim().is_empty()
                    || item.source_actor.trim().is_empty()
                    || item.proposition_surface.trim().is_empty()
                    || item.dialogue_truth_established
                    || item.external_execution_authorized
            })
        {
            return false;
        }
        let claim_ids = self
            .claims
            .iter()
            .map(|claim| claim.claim_id.as_str())
            .collect::<BTreeSet<_>>();
        claim_ids.len() == self.claims.len()
            && self.claims.iter().all(|claim| {
                !claim.claim_id.trim().is_empty()
                    && !claim.subject.trim().is_empty()
                    && !claim.value.trim().is_empty()
                    && claim
                        .evidence_belief_ids
                        .iter()
                        .all(|id| evidence_ids.contains(id.as_str()))
            })
    }
}

/// Partition source-owned response operations and independent task frames.
/// Tasks are retained separately, never dropped to manufacture an answer-only
/// batch. Conditional, reported, quoted and unparsed roots reject this partition.
struct ResponseOperationPlan {
    queries: Vec<String>,
    questions: BTreeMap<String, crate::proposition_content::QuestionRequestIR>,
    prohibited_slots: Vec<crate::proposition_content::ContentSlotIR>,
    task_frame_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseConstraintConflictIR {
    pub source_text: String,
    pub query_text: String,
}

impl ResponseConstraintConflictIR {
    pub fn validate(&self) -> bool {
        response_operation_queries(&self.source_text).is_some_and(|plan| {
            plan.queries.contains(&self.query_text) && plan.conflicts_with(&self.query_text)
        })
    }
}

impl ResponseOperationPlan {
    fn conflicts_with(&self, query: &str) -> bool {
        crate::proposition_content::requested_content_slots(query)
            .iter()
            .any(|slot| self.prohibited_slots.contains(slot))
    }
}

fn response_operation_queries(text: &str) -> Option<ResponseOperationPlan> {
    use crate::compositional_semantics::{FrameMoodIR, FramePolarityIR};
    if text.contains(['"', '“', '”', '‘', '’', '`']) {
        return None;
    }
    let analysis = crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(text);
    // Every source span belongs to a parsed clause or an explicit connector.
    // Recognizing two operations cannot hide an unparsed third instruction.
    let mut spans = analysis
        .clause_graph
        .nodes
        .iter()
        .map(|n| (n.source_start_byte, n.source_end_byte))
        .collect::<Vec<_>>();
    spans.sort_unstable();
    let connector_only = |gap: &str| {
        let gap = gap.trim_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
        gap.is_empty()
            || analysis.clause_graph.edges.iter().any(|edge| {
                matches!(
                    edge.relation,
                    crate::clause_graph::ClauseRelationKindIR::Coordination
                        | crate::clause_graph::ClauseRelationKindIR::Sequence
                ) && edge.marker_surface == gap
            })
    };
    let mut covered = 0;
    for (start, end) in spans {
        if start > covered && !connector_only(text.get(covered..start)?) {
            return None;
        }
        covered = covered.max(end);
    }
    if !connector_only(text.get(covered..)?) {
        return None;
    }
    let mut queries = Vec::new();
    let mut questions = BTreeMap::new();
    let mut matrix_count = 0;
    let mut prohibited_slots = Vec::new();
    let mut task_frame_ids = Vec::new();
    for frame in &analysis.frames {
        let node = analysis.clause_graph.node_for_frame(&frame.frame_id)?;
        if node.function == crate::clause_graph::ClauseFunctionIR::ContentComplement {
            continue;
        }
        let question = crate::proposition_content::question_request_for_frame(
            text,
            &analysis,
            &frame.frame_id,
        );
        if !node.function.permits_independent_directive()
            || frame.embedded_under_quote
            || !matches!(
                frame.mood,
                FrameMoodIR::Interrogative | FrameMoodIR::Imperative
            )
        {
            return None;
        }
        let response_content = frame.intent_hint == dockable_semantic_core::PlanIntentIR::Explain
            || matches!(
                frame.intent_hint,
                dockable_semantic_core::PlanIntentIR::Communicate
                    | dockable_semantic_core::PlanIntentIR::Investigate
            ) && (question.is_some()
                || !crate::conversation_contract::response_prohibition(
                    &node.source_text,
                    &crate::compositional_semantics::CompositionalSemanticAnalyzer
                        .analyze(&node.source_text),
                )
                .is_empty());
        if !response_content
            || crate::conversation_contract::has_other_recipient(&analysis, &frame.frame_id)
        {
            // Retain independently scoped tasks as a separate product, never
            // reinterpret them as answer content or silently discard them.
            if frame.mood != FrameMoodIR::Imperative
                || frame.polarity != FramePolarityIR::Positive
                || !frame.external_execution_authorized
                // The current task partition does not resolve nominal versus
                // clausal coordination inside one task span. A second
                // conjunction without its own parsed clause is ambiguous,
                // not proof that the whole trailing instruction was consumed.
                || node.source_text.split_whitespace().any(|word| {
                    matches!(word.trim_matches(|c: char| c.is_ascii_punctuation()).to_lowercase().as_str(), "and" | "then" | "but")
                })
            {
                return None;
            }
            task_frame_ids.push(frame.frame_id.clone());
            matrix_count += 1;
            continue;
        }
        if frame.polarity == FramePolarityIR::Positive {
            queries.push(node.source_text.clone());
            if let Some(question) = question {
                questions.insert(node.source_text.clone(), question);
            }
        } else {
            let mut slots = crate::proposition_content::requested_content_slots(&node.source_text);
            if let Some(slot) = crate::proposition_content::nominal_response_slot(&frame.theme) {
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
            }
            if slots.is_empty() {
                return None;
            }
            for slot in slots {
                if !prohibited_slots.contains(&slot) {
                    prohibited_slots.push(slot);
                }
            }
        }
        matrix_count += 1;
    }
    (matrix_count >= 2 && (1..=8).contains(&queries.len())).then_some(ResponseOperationPlan {
        queries,
        questions,
        prohibited_slots,
        task_frame_ids,
    })
}

pub(crate) fn is_response_operation_batch(text: &str) -> bool {
    response_operation_queries(text).is_some_and(|plan| plan.task_frame_ids.is_empty())
}

pub(crate) fn mixed_response_task_frames(text: &str) -> Option<Vec<String>> {
    let plan = response_operation_queries(text)?;
    (!plan.task_frame_ids.is_empty()).then_some(plan.task_frame_ids)
}

fn response_part_evidence(
    parts: &[DiscourseAnswerIR],
) -> Option<(Vec<DiscourseAnswerEvidenceIR>, Vec<DiscourseAnswerClaimIR>)> {
    let mut evidence: Vec<DiscourseAnswerEvidenceIR> = Vec::new();
    let mut claims = Vec::new();
    for part in parts {
        for item in &part.evidence {
            if let Some(existing) = evidence.iter().find(|e| e.belief_id == item.belief_id) {
                if existing != item {
                    return None;
                }
            } else {
                evidence.push(item.clone());
            }
        }
        for item in &part.claims {
            let mut item = item.clone();
            item.claim_id = format!("RESPONSE-CLAIM-{:03}", claims.len() + 1);
            claims.push(item);
        }
    }
    (evidence.len() <= MAX_ANSWER_EVIDENCE && claims.len() <= MAX_ANSWER_CLAIMS)
        .then_some((evidence, claims))
}

/// A clause observation and its enclosing causal observation from the same
/// utterance can denote one event. Prefer the enclosing source only when the
/// exact event span and provenance agree; never merge different turns/speakers.
fn source_event_subsumes(whole: &BeliefRecordIR, part: &BeliefRecordIR) -> bool {
    let span = crate::proposition_content::reported_event_surface(&whole.proposition_surface);
    whole.belief_id != part.belief_id
        && whole.introduced_turn == part.introduced_turn
        && whole.source_actor == part.source_actor
        && whole.signature.modal_world == part.signature.modal_world
        && whole.proposition_polarity == part.proposition_polarity
        && span != whole.proposition_surface
        && span
            .trim_end_matches(['.', '!', '?', ' '])
            .eq_ignore_ascii_case(
                part.proposition_surface
                    .trim_end_matches(['.', '!', '?', ' ']),
            )
        && whole.content.events.len() == 1
        && part.content.events.len() == 1
        && whole.content.events[0].lexical_entry_ids == part.content.events[0].lexical_entry_ids
        && whole.content.events[0].negated == part.content.events[0].negated
        && whole.content.events[0].roles.iter().all(|(role, value)| {
            part.content.events[0]
                .roles
                .get(role)
                .is_some_and(|p| value.eq_ignore_ascii_case(p))
        })
        && whole.content.events[0].roles.len() == part.content.events[0].roles.len()
}

fn response_parts_disposition(parts: &[DiscourseAnswerIR]) -> DiscourseAnswerDispositionIR {
    if parts.iter().all(|p| {
        matches!(
            p.disposition,
            DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                | DiscourseAnswerDispositionIR::MultipleDialogueRecords
        )
    }) {
        if parts
            .iter()
            .any(|p| p.disposition == DiscourseAnswerDispositionIR::MultipleDialogueRecords)
        {
            DiscourseAnswerDispositionIR::MultipleDialogueRecords
        } else {
            DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        }
    } else {
        DiscourseAnswerDispositionIR::AmbiguousQuery
    }
}

fn response_parts_preview(parts: &[DiscourseAnswerIR]) -> Option<String> {
    parts
        .iter()
        .map(response_part_preview)
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join(" "))
}

/// Deterministic projection of the already selected meaning. This is not a
/// sentence draft or an answer cache. Validation recomputes this cheap view
/// from typed fields, never accepting caller-provided prose as semantic proof.
fn response_part_preview(part: &DiscourseAnswerIR) -> Option<String> {
    if let Some(preview) = part.structured_meaning_preview() {
        return Some(preview);
    }
    if let Some(projection) = &part.content_projection {
        return Some(
            projection
                .all_projections()
                .flat_map(|p| p.all_bindings())
                .map(|binding| binding.value.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if part.claims.is_empty() {
        return Some(format!("{:?}", part.disposition));
    }
    Some(
        part.claims
            .iter()
            .map(|claim| claim.value.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

impl DiscourseAnswerIR {
    fn validate_response_parts(&self) -> bool {
        if !(1..=8).contains(&self.response_parts.len())
            || self.schema != DISCOURSE_ANSWER_SCHEMA
            || self.query.schema != DISCOURSE_QUERY_SCHEMA
            || self.event_summary.is_some()
            || self.contextual_target.is_some()
            || self.content_request.is_some()
            || self.question_request.is_some()
            || self.content_projection.is_some()
            || self.world_reasoning.is_some()
            || self.world_memory_update.is_some()
            || self.world_clarification.is_some()
            || self.decision_inquiry.is_some()
            || self.reformulated_request.is_some()
            || self.dialogue_truth_established
            || self.external_execution_authorized
            || self.unsupported_claims != 0
            || self.response_parts.iter().any(|p| {
                !p.response_parts.is_empty()
                    || p.language != self.language
                    || !p.validate()
                    || p.world_reasoning.is_some()
                    || p.world_memory_update.is_some()
                    || p.world_clarification.is_some()
                    || p.decision_inquiry.is_some()
            })
        {
            return false;
        }
        self.query
            == DiscourseQaEngine
                .unanswered(&self.query.original_text, self.language)
                .query
            && self.response_parts.iter().all(|part| {
                response_parts_preview(std::slice::from_ref(part)).as_deref()
                    == Some(part.realized_text.as_str())
            })
            && response_operation_queries(&self.query.original_text).is_some_and(|plan| {
                plan.queries
                    == self
                        .response_parts
                        .iter()
                        .map(|p| {
                            p.question_request.as_ref().map_or_else(
                                || p.query.original_text.clone(),
                                |q| q.source_text.clone(),
                            )
                        })
                        .collect::<Vec<_>>()
                    && self.response_parts.iter().all(|part| {
                        let source = part
                            .question_request
                            .as_ref()
                            .map_or(part.query.original_text.as_str(), |q| {
                                q.source_text.as_str()
                            });
                        if !plan.conflicts_with(source) {
                            part.question_request.as_ref() == plan.questions.get(source)
                                && part.content_projection.as_ref().is_none_or(|projection| {
                                    projection.all_projections().all(|p| {
                                        p.elaboration_event.as_ref().is_none_or(|event| {
                                            event.roles.keys().all(|role| {
                                                !plan.prohibited_slots.contains(role)
                                                    || p.elaboration_omitted_roles.contains(role)
                                            })
                                        })
                                    })
                                })
                        } else {
                            part.disposition == DiscourseAnswerDispositionIR::AmbiguousQuery
                                && part.evidence.is_empty()
                                && part.claims.is_empty()
                                && part.content_projection.is_none()
                                && part.event_summary.is_none()
                                && part.response_constraint_conflict.as_ref().is_some_and(|c| {
                                    c.source_text == self.query.original_text
                                        && c.query_text == source
                                })
                        }
                    })
            })
            && response_part_evidence(&self.response_parts)
                .is_some_and(|(e, c)| e == self.evidence && c == self.claims)
            && self.disposition == response_parts_disposition(&self.response_parts)
            && response_parts_preview(&self.response_parts).as_deref()
                == Some(self.realized_text.as_str())
    }

    pub(crate) fn validate_response_part_memory(&self, state: &ConversationStateIR) -> bool {
        if std::iter::once(self).chain(&self.response_parts).any(|p| {
            (p.query.kind == DiscourseQueryKindIR::PropositionSources
                && p.korean_nominal_forms != source_nominal_forms(state, &p.evidence))
                || p.korean_nominal_forms.iter().any(|form| {
                    !state
                        .active_typed_entities
                        .iter()
                        .any(|entity| entity.korean_nominal_forms.contains(form))
                })
                || p.reference_gap
                    .as_ref()
                    .is_some_and(|gap| !gap.live_in(state))
                || p.contextual_target
                    .as_ref()
                    .is_some_and(|t| !t.validate_in(state))
                || p.contextual_target
                    .as_ref()
                    .filter(|t| t.shared_proposition.is_some())
                    .is_some_and(|t| {
                        let expected = matching_records(&p.query, state)
                            .into_iter()
                            .filter(|r| t.includes_belief(&r.belief_id))
                            .map(|r| r.belief_id.as_str())
                            .collect::<BTreeSet<_>>();
                        let observed = p
                            .evidence
                            .iter()
                            .map(|e| e.belief_id.as_str())
                            .collect::<BTreeSet<_>>();
                        expected != observed
                            || p.evidence.iter().any(|e| {
                                state
                                    .epistemic_ledger
                                    .record(&e.belief_id)
                                    .is_none_or(|r| evidence_from_record(r) != *e)
                            })
                    })
        }) {
            return false;
        }
        self.response_parts.iter().all(|p| {
            p.evidence.iter().all(|e| {
                state.epistemic_ledger.records.iter().any(|r| {
                    evidence_from_record(r) == *e
                        && r.status == BeliefRecordStatusIR::Active
                        && r.content.validate_source(&r.proposition_surface)
                })
            }) && p.event_summary.as_ref().is_none_or(|s| {
                state.epistemic_ledger.records.iter().any(|r| {
                    r.belief_id == s.belief_id
                        && r.content.events == [s.event.clone()]
                        && r.signature.modal_world == ModalWorldIR::Actual
                        && r.content.context_sources == s.context_sources
                })
            }) && p.content_projection.as_ref().is_none_or(|c| {
                state.epistemic_ledger.records.iter().any(|r| {
                    r.belief_id == c.belief_id
                        && c.bindings_grounded_in(&r.content)
                        && (r.signature.modal_world == ModalWorldIR::Actual
                            || crate::cognitive::source_content_projection_is_reported(p, c, r))
                        && r.content.context_sources == c.context_sources
                })
            })
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DiscourseQaEngine;

impl DiscourseQaEngine {
    fn answer_question_request(
        &self,
        request: crate::proposition_content::QuestionRequestIR,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
        prohibited_slots: &[crate::proposition_content::ContentSlotIR],
    ) -> Option<DiscourseAnswerIR> {
        // Consume the compiled request. The legacy event-query backend still
        // parses question_text; it cannot reinterpret the matrix as a task.
        let mut answer = self
            .answer(&request.question_text, state, language)
            .unwrap_or_else(|| self.unanswered(&request.question_text, language));
        answer.question_request = Some(request);
        self.plan_answer_detail(&mut answer, language, prohibited_slots);
        answer.validate().then_some(answer)
    }

    fn plan_answer_detail(
        &self,
        answer: &mut DiscourseAnswerIR,
        language: LanguageCodeIR,
        prohibited_slots: &[crate::proposition_content::ContentSlotIR],
    ) {
        use crate::generative_language::EventSummaryIR;
        use crate::proposition_content::{
            ContentProjectionIR, PropositionContentIR, ResponseMannerIR,
        };
        if answer
            .question_request
            .as_ref()
            .and_then(|q| q.response_manner)
            != Some(ResponseMannerIR::Detailed)
        {
            return;
        }
        let Some(projection) = answer.content_projection.as_mut() else {
            return;
        };
        // Select all supporting content before any sentence is generated.
        // Missing grammar leaves the existing grounded answer intact; it does
        // not trigger repeated generation or invent an explanatory relation.
        let summaries = projection
            .all_projections()
            .map(|p| {
                let content = PropositionContentIR::compile_contextual(
                    &p.source_proposition,
                    &p.context_sources,
                )?;
                let event = content
                    .events
                    .into_iter()
                    .find(|e| Some(&e.event_id) == p.binding.event_id.as_ref())?;
                let summary = EventSummaryIR {
                    omitted_roles: prohibited_slots
                        .iter()
                        .filter(|role| event.roles.contains_key(role))
                        .copied()
                        .collect(),
                    belief_id: p.belief_id.clone(),
                    source_actor: p.source_actor.clone(),
                    source_proposition: p.source_proposition.clone(),
                    context_sources: p.context_sources.clone(),
                    event,
                };
                summary.can_realize(language).then_some(summary)
            })
            .collect::<Option<Vec<_>>>();
        let Some(summaries) = summaries else {
            return;
        };
        if answer.claims.len() + summaries.len() > MAX_ANSWER_CLAIMS {
            return;
        }
        let new_claims = summaries
            .iter()
            .enumerate()
            .map(|(i, summary)| {
                Some(claim(
                    answer.claims.len() + i + 1,
                    AnswerClaimKindIR::SourceAttributedContent,
                    &summary.source_actor,
                    &serde_json::to_string(&summary.event).ok()?,
                    vec![summary.belief_id.clone()],
                ))
            })
            .collect::<Option<Vec<_>>>();
        let Some(new_claims) = new_claims else {
            return;
        };
        let ContentProjectionIR {
            elaboration_event,
            elaboration_omitted_roles,
            co_answers,
            ..
        } = projection;
        *elaboration_event = Some(summaries[0].event.clone());
        *elaboration_omitted_roles = summaries[0].omitted_roles.clone();
        for (p, summary) in co_answers.iter_mut().zip(&summaries[1..]) {
            p.elaboration_event = Some(summary.event.clone());
            p.elaboration_omitted_roles = summary.omitted_roles.clone();
        }
        answer.claims.extend(new_claims);
    }

    pub(crate) fn answer_response_operations(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        let plan = response_operation_queries(text)?;
        let mut answer = self.unanswered(text, language);
        for query in &plan.queries {
            let mut part = if plan.conflicts_with(query) {
                let mut gap = self.unanswered(query, language);
                gap.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
                gap.claims.clear();
                gap.content_request = None;
                gap.response_constraint_conflict = Some(ResponseConstraintConflictIR {
                    source_text: text.to_string(),
                    query_text: query.clone(),
                });
                gap
            } else if let Some(request) = plan.questions.get(query) {
                self.answer_question_request(
                    request.clone(),
                    state,
                    language,
                    &plan.prohibited_slots,
                )?
            } else {
                self.answer(query, state, language)
                    .unwrap_or_else(|| self.unanswered(query, language))
            };
            part.realized_text = response_part_preview(&part)?;
            answer.response_parts.push(part);
        }
        let (evidence, claims) = response_part_evidence(&answer.response_parts)?;
        answer.evidence = evidence;
        answer.claims = claims;
        answer.disposition = response_parts_disposition(&answer.response_parts);
        answer.realized_text = response_parts_preview(&answer.response_parts)?;
        answer.validate().then_some(answer)
    }

    /// Called only after the regular answer paths fail. A prior substantive
    /// topic/record blocks this first-version gap classifier: lack of causal
    /// evidence about a known target must not be mislabeled a missing target.
    pub(crate) fn clarify_unbound_explanation(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        if state.is_some_and(|s| {
            s.active_subject.is_some()
                || s.answer_focus.is_some()
                || s.epistemic_ledger.records.iter().any(|r| {
                    r.status == BeliefRecordStatusIR::Active
                        && r.content.interaction_preference.is_none()
                })
        }) {
            return None;
        }
        let kind = unbound_explanation_target(text)?;
        let mut answer = self.unanswered(text, language);
        answer.query.kind = kind;
        answer.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
        answer.claims.clear();
        answer.refresh_structured_preview();
        answer.validate().then_some(answer)
    }

    /// A missing answer is a typed gap, not an explanation plan. Keep this
    /// distinct from a successful answer in evaluation and downstream APIs.
    pub fn unanswered(&self, text: &str, language: LanguageCodeIR) -> DiscourseAnswerIR {
        DiscourseAnswerIR {
            plan_method: None,
            described_query: crate::proposition_content::described_event(text, true),
            korean_nominal_forms: Vec::new(),
            reference_gap: None,
            response_constraint_conflict: None,
            question_request: None,
            content_request: crate::proposition_content::content_request(text.trim()),
            contextual_target: None,
            response_parts: vec![],
            event_summary: None,
            decision_inquiry: None,
            reformulated_request: None,
            content_projection: None,
            world_reasoning: None,
            world_memory_update: None,
            world_clarification: None,
            schema: DISCOURSE_ANSWER_SCHEMA.to_string(),
            query: DiscourseQueryIR {
                schema: DISCOURSE_QUERY_SCHEMA.to_string(),
                original_text: text.to_string(),
                kind: DiscourseQueryKindIR::SourceContent,
                requested_source: None,
                requested_attitudes: vec![],
                topic_terms: vec![],
                temporal_scope: QueryTemporalScopeIR::Current,
                presuppositions: vec![],
                confidence_millis: 800,
            },
            disposition: DiscourseAnswerDispositionIR::NoMatchingRecord,
            evidence: vec![],
            claims: vec![DiscourseAnswerClaimIR {
                claim_id: "ANSWER-GAP".to_string(),
                kind: AnswerClaimKindIR::NoMatchingDialogueRecord,
                subject: "requested information".to_string(),
                value: "not available in dialogue evidence".to_string(),
                evidence_belief_ids: vec![],
            }],
            language,
            realized_text: match language {
                LanguageCodeIR::Korean => "현재 근거로는 그 질문에 답할 수 없어.",
                _ => "The available evidence does not answer that question.",
            }
            .to_string(),
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        }
    }

    pub(crate) fn reformulate(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        if !is_answer_reformulation(text) {
            return None;
        }
        let state = state?;
        let focus = state.answer_focus.as_ref()?;
        if !focus.validate(state.completed_turns)
            || state.completed_turns.saturating_sub(focus.answered_turn) > 3
        {
            return None;
        }
        // A prior question supplies a referent, not authority to replace the
        // current operation. Repeating an answer and recapping its event are
        // different requests even when both use "that".
        if crate::proposition_content::requested_content_slot(text).is_some_and(|slot| {
            Some(slot)
                != crate::proposition_content::requested_content_slot(&focus.query.original_text)
        }) {
            return self.answer(text, Some(state), language);
        }
        let mut answer = self
            .answer(&focus.query.original_text, Some(state), language)
            .unwrap_or_else(|| self.unanswered(&focus.query.original_text, language));
        // A knowledge gap retains the question's topic, not the wording of the
        // feedback. Source/evidence records themselves are always freshly read.
        if answer.disposition == DiscourseAnswerDispositionIR::NoMatchingRecord {
            answer.query.topic_terms = focus.query.topic_terms.clone();
        }
        answer.reformulated_request = Some(text.to_string());
        Some(answer)
    }

    pub fn parse(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
    ) -> Option<DiscourseQueryIR> {
        let normalized = normalize_space(&text.to_lowercase());
        let content_ellipsis = quotative_content_question(&normalized);
        if normalized.is_empty() || (!looks_like_question(&normalized) && !content_ellipsis) {
            return None;
        }
        let known_sources = state.map_or_else(Vec::new, known_sources);
        let generic_certainty = generic_certainty_question(&normalized);
        let outcome_alternative = outcome_alternative_question(&normalized);
        let mut source = source_in_query(&normalized, &known_sources)
            .or_else(|| extract_unknown_source(&normalized));
        let mut requested_attitudes = requested_attitudes(&normalized);
        if content_ellipsis {
            requested_attitudes = SPEECH_REPORT_ATTITUDES.to_vec();
        }
        let temporal_scope = if contains_any(
            &normalized,
            &[
                "before",
                "earlier",
                "previously",
                "originally",
                "전에",
                "이전에",
                "처음에는",
                "아까",
            ],
        ) {
            QueryTemporalScopeIR::Historical
        } else {
            QueryTemporalScopeIR::Current
        };

        let kind = if generic_certainty || outcome_alternative {
            DiscourseQueryKindIR::ActualityStatus
        } else if hypothetical_or_conditional_question(&normalized) {
            // The relation is scoped by a non-actual world: retain the
            // question as a modal dialogue query instead of manufacturing an
            // actual-world event presupposition.
            DiscourseQueryKindIR::ModalStatus
        } else if presuppositional_question(&normalized) {
            DiscourseQueryKindIR::PresuppositionCheck
        } else if conflict_question(&normalized) {
            DiscourseQueryKindIR::ConflictStatus
        } else if modal_status_question(&normalized) {
            DiscourseQueryKindIR::ModalStatus
        } else if actuality_question(&normalized) {
            DiscourseQueryKindIR::ActualityStatus
        } else if proposition_source_question(&normalized) {
            DiscourseQueryKindIR::PropositionSources
        } else if content_ellipsis || source_content_question(&normalized, source.as_deref()) {
            DiscourseQueryKindIR::SourceContent
        } else {
            return None;
        };
        if matches!(
            kind,
            DiscourseQueryKindIR::ConflictStatus | DiscourseQueryKindIR::PropositionSources
        ) {
            source = None;
        }
        if !matches!(
            kind,
            DiscourseQueryKindIR::SourceContent | DiscourseQueryKindIR::PropositionSources
        ) {
            requested_attitudes.clear();
        }
        let mut topic_terms = if generic_certainty || outcome_alternative || content_ellipsis {
            Vec::new()
        } else {
            query_topic_terms(&normalized, source.as_deref())
        };
        if kind == DiscourseQueryKindIR::PropositionSources {
            topic_terms.retain(|term| !proposition_deictic(term));
        }
        topic_terms.sort();
        topic_terms.dedup();
        let presuppositions = if kind == DiscourseQueryKindIR::PresuppositionCheck {
            vec![PresuppositionIR {
                kind: if contains_any(&normalized, &["know that", "realize that", "알고", "깨달"])
                {
                    PresuppositionKindIR::FactiveComplement
                } else if contains_any(&normalized, &["why", "when", "how", "왜", "언제", "어떻게"])
                {
                    PresuppositionKindIR::EventOccurred
                } else {
                    PresuppositionKindIR::StateHolds
                },
                surface_text: presupposed_surface(&normalized),
                dialogue_truth_established: false,
            }]
        } else {
            Vec::new()
        };
        Some(DiscourseQueryIR {
            schema: DISCOURSE_QUERY_SCHEMA.to_string(),
            original_text: text.trim().to_string(),
            kind,
            requested_source: source,
            requested_attitudes,
            topic_terms,
            temporal_scope,
            presuppositions,
            confidence_millis: if state.is_some() { 900 } else { 760 },
        })
    }

    pub fn answer(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        if let Some(request) = crate::proposition_content::question_request(text) {
            return self.answer_question_request(request, state, language, &[]);
        }
        self.answer_query_body(text, state, language)
    }

    // Keep request-envelope routing separate from record-query construction.
    // An inner question must not retain an unused outer record-query frame
    // (including its large answer temporaries) on the native CLI stack.
    fn answer_query_body(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        if crate::proposition_content::content_request(text).is_some() {
            return self
                .answer_content(text, state, language)
                .or_else(|| Some(self.unanswered(text, language)));
        }
        if matches!(
            crate::proposition_content::requested_content_slot(text),
            Some(
                crate::proposition_content::ContentSlotIR::Intention
                    | crate::proposition_content::ContentSlotIR::Condition
            )
        ) {
            if let Some(answer) = self.answer_content(text, state, language) {
                return Some(answer);
            }
        }
        if let Some(answer) = self.answer_described_event(text, state, language) {
            return Some(answer);
        }
        let parsed_query = self.parse(text, state);
        if !parsed_query.as_ref().is_some_and(|query| {
            matches!(
                query.kind,
                DiscourseQueryKindIR::SourceContent | DiscourseQueryKindIR::PropositionSources
            )
        }) {
            if let Some(answer) = self.answer_content(text, state, language) {
                return Some(answer);
            }
        }
        let query = parsed_query?;
        let target = if let Some(slot) = attribution_reference_slot(&query) {
            let Some(mut target) = state.and_then(source_context_target) else {
                let mut gap = self.unanswered(text, language);
                gap.query = query;
                gap.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
                gap.claims.clear();
                return Some(gap);
            };
            target.slot = slot;
            Some(target)
        } else {
            None
        };
        let mut matching = state.map_or_else(Vec::new, |state| matching_records(&query, state));
        if let Some(target) = &target {
            matching.retain(|r| target.includes_belief(&r.belief_id));
        }
        matching.sort_by(|left, right| {
            right
                .introduced_turn
                .cmp(&left.introduced_turn)
                .then_with(|| left.belief_id.cmp(&right.belief_id))
        });
        matching.truncate(MAX_ANSWER_EVIDENCE);
        let evidence = matching
            .iter()
            .map(|record| evidence_from_record(record))
            .collect::<Vec<_>>();
        // Missing causal knowledge is not evidence of a faulty user premise.
        // Preserve presupposition checks when modal/negative records actually
        // conflict with that premise; otherwise report the missing answer.
        if query.kind == DiscourseQueryKindIR::PresuppositionCheck
            && crate::proposition_content::requested_content_slot(text)
                == Some(crate::proposition_content::ContentSlotIR::Cause)
            && (evidence.is_empty()
                || matching.iter().all(|record| {
                    record.signature.modal_world == ModalWorldIR::Actual
                        && record.proposition_polarity
                            == crate::attribution::AttributedPropositionPolarityIR::Positive
                }))
        {
            return Some(self.unanswered(text, language));
        }
        let (disposition, mut claims) = answer_claims(&query, &evidence);
        claims.truncate(MAX_ANSWER_CLAIMS);
        let realized_text = realize_answer(language, &query, disposition, &evidence);
        let korean_nominal_forms =
            state.map_or_else(Vec::new, |s| source_nominal_forms(s, &evidence));
        let answer = DiscourseAnswerIR {
            plan_method: None,
            described_query: crate::proposition_content::described_event(
                &query.original_text,
                true,
            ),
            korean_nominal_forms,
            reference_gap: None,
            response_constraint_conflict: None,
            question_request: None,
            content_request: None,
            contextual_target: target,
            response_parts: vec![],
            event_summary: None,
            decision_inquiry: None,
            reformulated_request: None,
            content_projection: None,
            world_reasoning: None,
            world_memory_update: None,
            world_clarification: None,
            schema: DISCOURSE_ANSWER_SCHEMA.to_string(),
            query,
            disposition,
            evidence,
            claims,
            language,
            realized_text,
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        };
        debug_assert!(answer.validate());
        Some(answer)
    }

    fn answer_content(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        use crate::proposition_content::{requested_content_slot, ContentProjectionIR};
        let slot = requested_content_slot(text)?;
        let state = state?;
        if slot == crate::proposition_content::ContentSlotIR::Summary {
            return self.answer_event_summary(text, state, language);
        }
        if matches!(
            slot,
            crate::proposition_content::ContentSlotIR::Intention
                | crate::proposition_content::ContentSlotIR::Condition
        ) {
            return self.answer_dialogue_goal(text, state, language, slot);
        }
        let contextual = crate::proposition_content::contextual_content_slot(text) == Some(slot);
        let target = if contextual {
            contextual_record(state)
        } else {
            None
        };
        if contextual && target.is_none() {
            return Some(self.unanswered(text, language));
        }
        let mut query = self.unanswered(text, language).query;
        query.topic_terms = query_topic_terms(&text.to_lowercase(), None);
        let recalled_property = crate::proposition_content::self_reported_cause_target(text);
        if recalled_property.is_some() {
            query.requested_source = Some("DIALOGUE_USER".into());
            query.topic_terms.clear();
        }
        let request = crate::proposition_content::content_request(text);
        if let Some(named) = request.as_ref().and_then(|r| r.target_surface.as_ref()) {
            query.topic_terms = query_topic_terms(named, None);
        }
        let mut records = if let Some(record) = target {
            vec![record]
        } else {
            matching_records(&query, state)
        };
        if let Some(property) = recalled_property {
            let ids = crate::proposition_content::stative_predicate_ids(property);
            records.retain(|record| {
                let effect =
                    crate::proposition_content::reported_event_surface(&record.proposition_surface);
                effect.split_whitespace().next_back().is_some_and(|word| {
                    crate::proposition_content::stative_predicate_ids(
                        word.trim_end_matches(['.', '!']),
                    )
                    .iter()
                    .any(|id| ids.contains(id))
                })
            });
        }
        if request.as_ref().is_some_and(|r| r.target_surface.is_some()) {
            // A shared word is not identity of an explicit target. Require all
            // its content terms in the effect proposition, not the causal tail.
            records.retain(|r| {
                let terms = normalized_terms(crate::proposition_content::reported_event_surface(
                    &r.proposition_surface,
                ));
                !query.topic_terms.is_empty() && query.topic_terms.iter().all(|t| terms.contains(t))
            });
        }
        let query_frames = crate::compositional_semantics::CompositionalSemanticAnalyzer
            .analyze(text)
            .frames;
        let predicates = query_frames
            .iter()
            .filter(|frame| {
                !matches!(
                    frame.intent_hint,
                    dockable_semantic_core::PlanIntentIR::Explain
                        | dockable_semantic_core::PlanIntentIR::Communicate
                )
            })
            .map(|frame| frame.canonical_predicate.as_str())
            .collect::<BTreeSet<_>>();
        if query.topic_terms.is_empty() {
            let latest = records.iter().map(|record| record.introduced_turn).max();
            records.retain(|record| Some(record.introduced_turn) == latest);
        }
        // Possible/counterfactual worlds cannot answer a question about actuality.
        records.retain(|record| {
            record.signature.modal_world == ModalWorldIR::Actual
                && record.proposition_polarity
                    == crate::attribution::AttributedPropositionPolarityIR::Positive
                && record.status == BeliefRecordStatusIR::Active
        });
        let mut projections = Vec::new();
        for record in &records {
            if !record.content.validate_source(&record.proposition_surface) {
                continue;
            }
            for binding in record
                .content
                .bindings
                .iter()
                // Descriptive event roles have a dedicated same-event join.
                // Never send them back through loose topic/predicate matching.
                .filter(|binding| binding.slot == slot && binding.event_id.is_none())
            {
                if binding.predicate.as_deref().is_some_and(|predicate| {
                    !predicates.is_empty() && !predicates.contains(predicate)
                }) {
                    continue;
                }
                projections.push(ContentProjectionIR {
                    elaboration_omitted_roles: vec![],
                    elaboration_event: None,
                    event_perspective: None,
                    co_answers: vec![],
                    additional_bindings: vec![],
                    context_sources: record.content.context_sources.clone(),
                    belief_id: record.belief_id.clone(),
                    source_actor: record.source_actor.clone(),
                    source_proposition: record.proposition_surface.clone(),
                    binding: binding.clone(),
                    reference_context: None,
                    reference_bindings: vec![],
                });
            }
        }
        let mut answer = self.unanswered(text, language);
        answer.contextual_target = target.map(|r| ContextualContentTargetIR {
            shared_proposition: None,
            belief_id: r.belief_id.clone(),
            slot,
            resolved_after_turn: state.completed_turns,
            introduced_turn: r.introduced_turn,
        });
        if projections.len() != 1 {
            return (contextual || request.is_some() || recalled_property.is_some())
                .then_some(answer);
        }
        let projection = projections.remove(0);
        let record = records
            .iter()
            .find(|record| record.belief_id == projection.belief_id)?;
        answer.query = query;
        answer.disposition = DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
        answer.evidence = vec![evidence_from_record(record)];
        answer.claims = vec![claim(
            1,
            AnswerClaimKindIR::SourceAttributedContent,
            &projection.source_actor,
            &projection.binding.value,
            vec![projection.belief_id.clone()],
        )];
        answer.realized_text = projection.binding.value.clone();
        answer.content_projection = Some(projection);
        Some(answer)
    }

    /// Recap the immediately preceding unambiguous event or the bounded
    /// event focus. Never search an arbitrary old event to fill a response.
    fn answer_event_summary(
        &self,
        text: &str,
        state: &ConversationStateIR,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        let mut answer = self.unanswered(text, language);
        // Only a deictic recap is currently bound here. Explicit event/topic
        // selection and multi-event aggregation must not be guessed.
        if !crate::proposition_content::is_deictic_event_recap(text) {
            return Some(answer);
        }
        let focus = state
            .answer_focus
            .as_ref()
            .filter(|f| {
                f.validate(state.completed_turns)
                    && state.completed_turns.saturating_sub(f.answered_turn) <= 3
            })
            .and_then(|f| f.described_event.as_ref());
        let mut candidates = state
            .epistemic_ledger
            .records
            .iter()
            .filter(|r| {
                r.status == BeliefRecordStatusIR::Active
                    && r.signature.modal_world == ModalWorldIR::Actual
                    && r.content.validate_source(&r.proposition_surface)
                    && r.content.events.len() == 1
                    && (r.proposition_polarity
                        != crate::attribution::AttributedPropositionPolarityIR::Negative
                        || r.content.events[0].negated
                        || !r.content.context_sources.is_empty())
                    && if let Some(f) = focus {
                        r.belief_id == f.belief_id && r.content.events[0].event_id == f.event_id
                    } else {
                        r.introduced_turn == state.completed_turns
                    }
            })
            .collect::<Vec<_>>();
        let sources = candidates.clone();
        candidates.retain(|part| {
            !sources
                .iter()
                .any(|whole| source_event_subsumes(whole, part))
        });
        if candidates.len() != 1 {
            if candidates.len() > 1 {
                answer.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
            }
            return Some(answer);
        }
        let r = candidates[0];
        let summary = crate::generative_language::EventSummaryIR {
            omitted_roles: vec![],
            belief_id: r.belief_id.clone(),
            source_actor: r.source_actor.clone(),
            source_proposition: r.proposition_surface.clone(),
            context_sources: r.content.context_sources.clone(),
            event: r.content.events[0].clone(),
        };
        if !summary.can_realize(language) {
            return Some(answer);
        }
        answer.disposition = DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
        answer.evidence = vec![evidence_from_record(r)];
        answer.claims = vec![claim(
            1,
            AnswerClaimKindIR::SourceAttributedContent,
            &summary.source_actor,
            &serde_json::to_string(&summary.event).ok()?,
            vec![summary.belief_id.clone()],
        )];
        answer.event_summary = Some(summary);
        answer.refresh_structured_preview();
        Some(answer)
    }

    /// Resolve a role gap by joining all explicit constraints on ONE attributed
    /// event. Ellipsis reuses a source reference, never prior generated text.
    fn answer_dialogue_goal(
        &self,
        text: &str,
        state: &ConversationStateIR,
        language: LanguageCodeIR,
        slot: crate::proposition_content::ContentSlotIR,
    ) -> Option<DiscourseAnswerIR> {
        use crate::proposition_content::{ContentProjectionIR, ContentSlotIR};
        let lower = text.to_lowercase();
        if slot == ContentSlotIR::Intention
            && !lower.split(|c: char| !c.is_alphanumeric()).any(|w| {
                matches!(
                    w,
                    "내가" | "제가" | "나는" | "난" | "저는" | "i" | "we" | "my" | "our"
                )
            })
            && !crate::proposition_content::attributed_interaction_preference_query(text)
        {
            let mut gap = self.unanswered(text, language);
            gap.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
            return Some(gap);
        }
        let mut candidates = state
            .epistemic_ledger
            .records
            .iter()
            .filter(|r| {
                r.status == BeliefRecordStatusIR::Active
                    && r.source_actor == "DIALOGUE_USER"
                    && r.content.validate_source(&r.proposition_surface)
            })
            .flat_map(|r| {
                r.content
                    .bindings
                    .iter()
                    .filter(move |b| b.slot == slot)
                    .map(move |b| (r, b))
            })
            .filter(|(_, b)| {
                slot != ContentSlotIR::Condition
                    || !lower.contains("계속")
                    || b.predicate.as_ref().is_some_and(|p| p.contains("계속"))
            })
            .collect::<Vec<_>>();
        if slot == ContentSlotIR::Intention {
            let latest = candidates.iter().map(|(r, _)| r.introduced_turn).max();
            candidates.retain(|(r, _)| Some(r.introduced_turn) == latest);
            // A clause and its containing utterance may preserve the same wish.
            // Prefer the source that also carries its exclusions; equivalent
            // content is not an ambiguous second preference. Keep both sources
            // in the ledger, and keep genuinely different wishes ambiguous.
            candidates.sort_by(|(a, x), (b, y)| {
                x.value.cmp(&y.value).then_with(|| {
                    b.content
                        .interaction_preference
                        .as_ref()
                        .map_or(0, |p| p.excluded.len())
                        .cmp(
                            &a.content
                                .interaction_preference
                                .as_ref()
                                .map_or(0, |p| p.excluded.len()),
                        )
                })
            });
            candidates.dedup_by(|(_, a), (_, b)| a.value == b.value);
        }
        let mut answer = self.unanswered(text, language);
        if candidates.len() != 1 {
            return Some(answer);
        }
        let (record, binding) = candidates[0];
        answer.disposition = DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords;
        answer.evidence = vec![evidence_from_record(record)];
        answer.claims = vec![claim(
            1,
            AnswerClaimKindIR::SourceAttributedContent,
            &record.source_actor,
            &binding.value,
            vec![record.belief_id.clone()],
        )];
        answer.realized_text = binding.value.clone();
        answer.content_projection = Some(ContentProjectionIR {
            elaboration_omitted_roles: vec![],
            elaboration_event: None,
            event_perspective: None,
            co_answers: vec![],
            belief_id: record.belief_id.clone(),
            source_actor: record.source_actor.clone(),
            source_proposition: record.proposition_surface.clone(),
            binding: binding.clone(),
            additional_bindings: vec![],
            context_sources: record.content.context_sources.clone(),
            reference_context: None,
            reference_bindings: vec![],
        });
        Some(answer)
    }

    pub(crate) fn answer_described_event(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
        language: LanguageCodeIR,
    ) -> Option<DiscourseAnswerIR> {
        use crate::proposition_content::{
            described_event, requested_content_slot, ContentProjectionIR, ContentSlotIR,
        };
        let mut query = described_event(text, true)?;
        // Understanding the requested slot does not depend on already having
        // an answer. A bound open-property question remains that question in
        // an empty memory, rather than falling into a generic presupposition scan.
        if query.kind == crate::proposition_content::DescriptionKindIR::State
            && query.lexical_entry_ids.is_empty()
            && !query.has_references()
            && state.is_none_or(|s| {
                !s.epistemic_ledger
                    .records
                    .iter()
                    .any(|r| !r.content.events.is_empty())
            })
        {
            return Some(self.unanswered(text, language));
        }
        // Questions about who said/believed a proposition belong to the source
        // ledger, not the actor/argument roles inside the proposition.
        let question_body =
            crate::conversation::topic_question_parts(text).map_or(text, |(_, body)| body);
        if self.parse(question_body, state).is_some_and(|q| {
            matches!(
                q.kind,
                DiscourseQueryKindIR::SourceContent | DiscourseQueryKindIR::PropositionSources
            )
        }) {
            return None;
        }
        let slot = requested_content_slot(text)?;
        if !matches!(
            slot,
            ContentSlotIR::Agent
                | ContentSlotIR::Theme
                | ContentSlotIR::Property
                | ContentSlotIR::Recipient
                | ContentSlotIR::Source
                | ContentSlotIR::Location
                | ContentSlotIR::Time
                | ContentSlotIR::Duration
        ) {
            return None;
        }
        let state = state?;
        // With no descriptive memory this module does not claim unrelated paths.
        if !state
            .epistemic_ledger
            .records
            .iter()
            .any(|r| !r.content.events.is_empty())
        {
            return None;
        }
        let elliptical = query.lexical_entry_ids.is_empty()
            && query.topic_constraint.is_none()
            && !query.has_references();
        let discourse_center = state
            .discourse_focus
            .validate(state.completed_turns)
            .then(|| state.discourse_focus.current())
            .flatten()
            .filter(|c| {
                state.completed_turns.saturating_sub(c.last_focused_turn)
                    <= crate::discourse_focus::MAX_DISCOURSE_FOCUS_TURN_DISTANCE
            });
        let state_center = discourse_center
            .filter(|_| query.kind == crate::proposition_content::DescriptionKindIR::State);
        let centered_bearers = state_center
            .map(|c| {
                state
                    .epistemic_ledger
                    .records
                    .iter()
                    .filter(|r| r.last_updated_turn == c.last_focused_turn)
                    .flat_map(|r| r.content.events.iter())
                    .filter(|e| e.kind == crate::proposition_content::DescriptionKindIR::State)
                    .filter_map(|e| e.roles.get(&ContentSlotIR::Theme))
                    .collect::<BTreeSet<_>>()
                    .len()
            })
            .unwrap_or(0);
        let focus = state
            .answer_focus
            .as_ref()
            .filter(|f| {
                f.validate(state.completed_turns)
                    && state.completed_turns.saturating_sub(f.answered_turn) <= 3
                    && discourse_center.is_none_or(|c| c.last_focused_turn <= f.answered_turn)
            })
            .and_then(|f| f.described_event.as_ref());
        let mut reference_context = None;
        let mut reference_bindings = Vec::new();
        if query.has_references() {
            let topic_query = &query;
            let mut antecedents = state
                .epistemic_ledger
                .records
                .iter()
                .filter(|r| {
                    r.status == BeliefRecordStatusIR::Active
                        && r.signature.modal_world == ModalWorldIR::Actual
                        && !matches!(
                            r.epistemic_status,
                            EpistemicStatusIR::Denied | EpistemicStatusIR::Doubted
                        )
                        && r.content.validate_source(&r.proposition_surface)
                })
                .flat_map(|r| {
                    r.content.events.iter().filter_map(move |e| {
                        let selected = if topic_query.topic_constraint.is_some() {
                            e.matches_topic(topic_query)
                        } else if let Some(f) = focus {
                            f.belief_id == r.belief_id && f.event_id == e.event_id
                        } else if let Some(center) = state_center {
                            use crate::discourse_focus::DiscourseFocusSourceIR;
                            e.kind == crate::proposition_content::DescriptionKindIR::State
                                && match center.source {
                                    // A ranking among simultaneous introductions
                                    // cannot silently remove reference ambiguity.
                                    DiscourseFocusSourceIR::Proposition if centered_bearers > 1 => {
                                        r.last_updated_turn == center.last_focused_turn
                                    }
                                    DiscourseFocusSourceIR::Proposition => {
                                        r.last_updated_turn == center.last_focused_turn
                                            && e.roles.get(&ContentSlotIR::Theme).is_some_and(
                                                |bearer| {
                                                    bearer.eq_ignore_ascii_case(&center.surface)
                                                },
                                            )
                                    }
                                    DiscourseFocusSourceIR::ExplicitTopic => {
                                        e.roles.get(&ContentSlotIR::Theme).is_some_and(|bearer| {
                                            bearer.eq_ignore_ascii_case(&center.surface)
                                        })
                                    }
                                    _ => false,
                                }
                        } else if let Some(center) = discourse_center {
                            // A question without an answer is not an observed
                            // event or a topic shift. Use the retained center's
                            // provenance rather than requiring the preceding
                            // turn itself to contain another observation.
                            use crate::discourse_focus::DiscourseFocusSourceIR;
                            e.kind == crate::proposition_content::DescriptionKindIR::Event
                                && match center.source {
                                    DiscourseFocusSourceIR::Proposition => {
                                        r.last_updated_turn == center.last_focused_turn
                                    }
                                    DiscourseFocusSourceIR::ExplicitTopic => e
                                        .roles
                                        .values()
                                        .any(|v| v.eq_ignore_ascii_case(&center.surface)),
                                    _ => false,
                                }
                        } else {
                            r.introduced_turn == state.completed_turns
                        };
                        selected.then_some((r, e))
                    })
                })
                .collect::<Vec<_>>();
            let sources = antecedents.clone();
            antecedents.retain(|(part, _)| {
                !sources
                    .iter()
                    .any(|(whole, _)| source_event_subsumes(whole, part))
            });
            let mut gap = self.unanswered(text, language);
            gap.disposition = if antecedents.is_empty() {
                DiscourseAnswerDispositionIR::NoMatchingRecord
            } else {
                DiscourseAnswerDispositionIR::AmbiguousQuery
            };
            if antecedents.is_empty() || antecedents.len() > 8 {
                return Some(gap);
            }
            let contexts = antecedents
                .into_iter()
                .map(
                    |(record, event)| crate::proposition_content::EventReferenceContextIR {
                        belief_id: record.belief_id.clone(),
                        source_actor: record.source_actor.clone(),
                        source_proposition: record.proposition_surface.clone(),
                        event_id: event.event_id.clone(),
                        focused_slot: focus
                            .filter(|f| {
                                f.belief_id == record.belief_id && f.event_id == event.event_id
                            })
                            .and(state.answer_focus.as_ref())
                            .and_then(|f| {
                                let question = described_event(&f.query.original_text, true)?;
                                event.source_role_for_query(
                                    &question,
                                    requested_content_slot(&f.query.original_text)?,
                                )
                            }),
                        context_sources: record.content.context_sources.clone(),
                    },
                )
                .collect::<Vec<_>>();
            gap.reference_gap =
                crate::proposition_content::EventReferenceGapIR::from_contexts(text, &contexts);
            if gap.reference_gap.is_some() {
                gap.claims.clear();
            }
            let mut resolutions = Vec::new();
            for context in contexts {
                let Some((resolved, bindings)) = context.resolve(&query) else {
                    return Some(gap);
                };
                resolutions.push((context, resolved, bindings));
            }
            // Different records can establish the same referent. A set of
            // answers is not a license to guess between different referents.
            if resolutions
                .iter()
                .any(|(_, resolved, _)| resolved != &resolutions[0].1)
            {
                return Some(gap);
            }
            let (context, resolved, bindings) = resolutions.remove(0);
            query = resolved;
            reference_bindings = bindings;
            reference_context = Some(context);
        }
        let mut candidates = Vec::new();
        for record in &state.epistemic_ledger.records {
            if record.status != BeliefRecordStatusIR::Active
                || record.signature.modal_world != ModalWorldIR::Actual
                || matches!(
                    record.epistemic_status,
                    EpistemicStatusIR::Denied | EpistemicStatusIR::Doubted
                )
                || !record.content.validate_source(&record.proposition_surface)
            {
                continue;
            }
            for event in &record.content.events {
                // An unresolved pronoun in a report cannot answer as a literal
                // entity. Report-side cross-turn binding is a separate operation.
                if event.has_references() {
                    continue;
                }
                if record.proposition_polarity
                    == crate::attribution::AttributedPropositionPolarityIR::Negative
                    && !event.negated
                    && record.content.context_sources.is_empty()
                {
                    continue;
                }
                if !event.matches(&query, slot) {
                    continue;
                }
                if elliptical {
                    if let Some(focus) = focus {
                        if focus.belief_id != record.belief_id || focus.event_id != event.event_id {
                            continue;
                        }
                    } else if record.introduced_turn != state.completed_turns {
                        // An unbound fragment may refer to the immediately prior
                        // report only. No arbitrary newest match across topics.
                        continue;
                    }
                }
                candidates.push((record, event));
            }
        }
        let sources = candidates.clone();
        candidates.retain(|(part, _)| {
            !sources
                .iter()
                .any(|(whole, _)| source_event_subsumes(whole, part))
        });
        let mut answer = self.unanswered(text, language);
        let slots = crate::proposition_content::requested_content_slots(text);
        if candidates.is_empty()
            || candidates.len() > 8
            || (candidates.len() > 1
                && (slots.len() != 1
                    || candidates
                        .iter()
                        .any(|(r, _)| r.source_actor != candidates[0].0.source_actor)))
        {
            if candidates.len() > 1 {
                answer.disposition = DiscourseAnswerDispositionIR::AmbiguousQuery;
            }
            return Some(answer);
        }
        let mut projections = Vec::new();
        for (record, event) in candidates {
            let (event, event_perspective) = event.query_view(&query)?;
            let Some(binding) = event.bindings().into_iter().find(|b| b.slot == slot) else {
                return Some(answer);
            };
            projections.push(ContentProjectionIR {
                elaboration_omitted_roles: vec![],
                elaboration_event: None,
                event_perspective,
                co_answers: vec![],
                additional_bindings: slots
                    .iter()
                    .skip(1)
                    .map(|s| event.bindings().into_iter().find(|b| b.slot == *s))
                    .collect::<Option<Vec<_>>>()?,
                context_sources: record.content.context_sources.clone(),
                belief_id: record.belief_id.clone(),
                source_actor: record.source_actor.clone(),
                source_proposition: record.proposition_surface.clone(),
                binding,
                reference_context: reference_context.clone(),
                reference_bindings: reference_bindings.clone(),
            });
            if !answer
                .evidence
                .iter()
                .any(|e| e.belief_id == record.belief_id)
            {
                answer.evidence.push(evidence_from_record(record));
            }
        }
        let mut projection = projections.remove(0);
        projection.co_answers = projections;
        answer.disposition = if projection.co_answers.is_empty() {
            DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        } else {
            DiscourseAnswerDispositionIR::MultipleDialogueRecords
        };
        answer.claims = projection
            .all_projections()
            .flat_map(|p| p.all_bindings().map(move |b| (p, b)))
            .enumerate()
            .map(|(i, (p, b))| {
                claim(
                    i + 1,
                    AnswerClaimKindIR::SourceAttributedContent,
                    &p.source_actor,
                    &b.value,
                    vec![p.belief_id.clone()],
                )
            })
            .collect();
        answer.realized_text = projection.binding.value.clone();
        answer.content_projection = Some(projection);
        Some(answer)
    }
}

fn answer_claims(
    query: &DiscourseQueryIR,
    evidence: &[DiscourseAnswerEvidenceIR],
) -> (DiscourseAnswerDispositionIR, Vec<DiscourseAnswerClaimIR>) {
    let evidence_ids = evidence
        .iter()
        .map(|item| item.belief_id.clone())
        .collect::<Vec<_>>();
    match query.kind {
        DiscourseQueryKindIR::SourceContent | DiscourseQueryKindIR::PropositionSources => {
            if evidence.is_empty() {
                return (
                    if query.requested_source.is_none()
                        && query.kind == DiscourseQueryKindIR::SourceContent
                    {
                        DiscourseAnswerDispositionIR::AmbiguousQuery
                    } else {
                        DiscourseAnswerDispositionIR::NoMatchingRecord
                    },
                    vec![claim(
                        1,
                        AnswerClaimKindIR::NoMatchingDialogueRecord,
                        query.requested_source.as_deref().unwrap_or("QUERY"),
                        "NO_MATCHING_DIALOGUE_RECORD",
                        Vec::new(),
                    )],
                );
            }
            let mut claims = Vec::new();
            for (index, item) in evidence.iter().enumerate() {
                claims.push(claim(
                    index * 2 + 1,
                    AnswerClaimKindIR::SourceAttributedContent,
                    &item.source_actor,
                    &item.proposition_surface,
                    vec![item.belief_id.clone()],
                ));
                claims.push(claim(
                    index * 2 + 2,
                    AnswerClaimKindIR::SourceAttitude,
                    &item.source_actor,
                    &format!("{:?}:{:?}", item.attitude, item.epistemic_status),
                    vec![item.belief_id.clone()],
                ));
            }
            (
                if evidence.len() == 1 {
                    DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                } else {
                    DiscourseAnswerDispositionIR::MultipleDialogueRecords
                },
                claims,
            )
        }
        DiscourseQueryKindIR::ActualityStatus => (
            DiscourseAnswerDispositionIR::DialogueTruthNotEstablished,
            vec![claim(
                1,
                AnswerClaimKindIR::DialogueTruthNotEstablished,
                "DIALOGUE_STATE",
                if evidence.is_empty() {
                    "NO_MATCHING_EVIDENCE_AND_TRUTH_NOT_ESTABLISHED"
                } else {
                    "MATCHING_REPORTS_EXIST_BUT_TRUTH_NOT_ESTABLISHED"
                },
                evidence_ids,
            )],
        ),
        DiscourseQueryKindIR::ModalStatus => {
            if evidence.is_empty() {
                return (
                    DiscourseAnswerDispositionIR::NoMatchingRecord,
                    vec![claim(
                        1,
                        AnswerClaimKindIR::NoMatchingDialogueRecord,
                        "DIALOGUE_STATE",
                        "NO_MATCHING_MODAL_RECORD",
                        Vec::new(),
                    )],
                );
            }
            let mut claims = evidence
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    claim(
                        index + 1,
                        AnswerClaimKindIR::ModalWorldClassification,
                        &item.proposition_surface,
                        &format!("{:?}", item.modal_world),
                        vec![item.belief_id.clone()],
                    )
                })
                .collect::<Vec<_>>();
            claims.push(claim(
                claims.len() + 1,
                AnswerClaimKindIR::DialogueTruthNotEstablished,
                "DIALOGUE_STATE",
                "MODAL_CLASSIFICATION_IS_NOT_WORLD_TRUTH",
                evidence_ids,
            ));
            (
                if evidence.len() == 1 {
                    DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
                } else {
                    DiscourseAnswerDispositionIR::MultipleDialogueRecords
                },
                claims,
            )
        }
        DiscourseQueryKindIR::ConflictStatus => {
            let conflicted = evidence
                .iter()
                .filter(|item| item.record_status == BeliefRecordStatusIR::Contested)
                .count()
                >= 2;
            if conflicted {
                (
                    DiscourseAnswerDispositionIR::ConflictingDialogueRecords,
                    vec![claim(
                        1,
                        AnswerClaimKindIR::ConflictObserved,
                        "DIALOGUE_SOURCES",
                        "CONFLICT_PRESERVED_NO_TRUTH_WINNER",
                        evidence_ids,
                    )],
                )
            } else {
                (
                    DiscourseAnswerDispositionIR::NoConflictRecorded,
                    vec![claim(
                        1,
                        AnswerClaimKindIR::NoConflictObserved,
                        "DIALOGUE_SOURCES",
                        "NO_MATCHING_ACTIVE_CONFLICT",
                        evidence_ids,
                    )],
                )
            }
        }
        DiscourseQueryKindIR::MissingExplanationTarget
        | DiscourseQueryKindIR::MissingComparisonOperands => {
            (DiscourseAnswerDispositionIR::AmbiguousQuery, vec![])
        }
        DiscourseQueryKindIR::PresuppositionCheck => (
            DiscourseAnswerDispositionIR::PresuppositionUnverified,
            vec![claim(
                1,
                AnswerClaimKindIR::PresuppositionNotEstablished,
                "QUERY_PRESUPPOSITION",
                query
                    .presuppositions
                    .first()
                    .map_or("UNVERIFIED", |item| item.surface_text.as_str()),
                evidence_ids,
            )],
        ),
    }
}

fn claim(
    index: usize,
    kind: AnswerClaimKindIR,
    subject: &str,
    value: &str,
    evidence_belief_ids: Vec<String>,
) -> DiscourseAnswerClaimIR {
    DiscourseAnswerClaimIR {
        claim_id: format!("ANSWER-CLAIM-{index:02}"),
        kind,
        subject: subject.to_string(),
        value: value.to_string(),
        evidence_belief_ids,
    }
}

fn matching_records<'a>(
    query: &DiscourseQueryIR,
    state: &'a ConversationStateIR,
) -> Vec<&'a BeliefRecordIR> {
    state
        .epistemic_ledger
        .records
        .iter()
        .filter(|record| match query.temporal_scope {
            QueryTemporalScopeIR::Current => record.status.is_reference_active(),
            QueryTemporalScopeIR::Historical => true,
        })
        .filter(|record| {
            query.requested_source.as_deref().is_none_or(|source| {
                normalize_actor(source) == normalize_actor(&record.source_actor)
            })
        })
        .filter(|record| {
            query.requested_attitudes.is_empty()
                || query
                    .requested_attitudes
                    .contains(&record.attribution_attitude)
        })
        .filter(|record| record_topic_score(record, &query.topic_terms) > 0)
        .collect()
}

fn evidence_from_record(record: &BeliefRecordIR) -> DiscourseAnswerEvidenceIR {
    DiscourseAnswerEvidenceIR {
        belief_id: record.belief_id.clone(),
        source_actor: record.source_actor.clone(),
        proposition_surface: record.proposition_surface.clone(),
        attitude: record.attribution_attitude,
        epistemic_status: record.epistemic_status,
        modal_world: record.signature.modal_world,
        record_status: record.status,
        introduced_turn: record.introduced_turn,
        dialogue_truth_established: false,
        external_execution_authorized: false,
    }
}

fn record_topic_score(record: &BeliefRecordIR, topic_terms: &[String]) -> usize {
    if topic_terms.is_empty() {
        return 1;
    }
    let proposition_terms = normalized_terms(&record.proposition_surface);
    let mut score = topic_terms
        .iter()
        .filter(|term| proposition_terms.contains(*term))
        .count();
    if topic_terms
        .iter()
        .any(|term| term == &record.signature.subject_key)
    {
        score += 2;
    }
    score
}

fn known_sources(state: &ConversationStateIR) -> Vec<String> {
    let mut sources = state
        .epistemic_ledger
        .records
        .iter()
        .map(|record| record.source_actor.clone())
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    sources.dedup_by(|left, right| normalize_actor(left) == normalize_actor(right));
    sources
}

fn source_in_query(text: &str, sources: &[String]) -> Option<String> {
    sources
        .iter()
        .find(|source| contains_actor_surface(text, &normalize_actor(source)))
        .cloned()
}

fn contains_actor_surface(text: &str, actor: &str) -> bool {
    if actor.is_empty() {
        return false;
    }
    if !actor.is_ascii() {
        return text.contains(actor);
    }
    text.match_indices(actor).any(|(start, _)| {
        let before = &text[..start];
        let after = &text[start + actor.len()..];
        let left_boundary = before
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        let right_boundary = after
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        let korean_particle = [
            "는", "은", "이", "가", "을", "를", "의", "에", "도", "와", "과",
        ]
        .iter()
        .any(|particle| after.starts_with(particle));
        left_boundary && (right_boundary || korean_particle)
    })
}

fn extract_unknown_source(text: &str) -> Option<String> {
    for prefix in ["what did ", "what does ", "what do "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            for marker in [
                " say", " claim", " report", " believe", " think", " know", " want", " expect",
            ] {
                if let Some(end) = rest.find(marker) {
                    let actor = rest[..end].trim();
                    if !actor.is_empty() && actor.split_whitespace().count() <= 4 {
                        return Some(actor.to_string());
                    }
                }
            }
        }
    }
    if let Some(rest) = text.strip_prefix("what is ") {
        if let Some(end) = rest.find("'s ") {
            let actor = rest[..end].trim();
            if !actor.is_empty() {
                return Some(actor.to_string());
            }
        }
    }
    for marker in [
        "는 뭐",
        "은 뭐",
        "가 뭐",
        "이 뭐",
        "는 무엇",
        "은 무엇",
        "가 무엇",
        "이 무엇",
    ] {
        if let Some(end) = text.find(marker) {
            let actor = text[..end]
                .split_whitespace()
                .next_back()
                .unwrap_or_default()
                .trim();
            if !actor.is_empty() && !matches!(actor, "누구" | "누가") {
                return Some(actor.to_string());
            }
        }
    }
    None
}

fn requested_attitudes(text: &str) -> Vec<AttributionAttitudeIR> {
    let mut attitudes = Vec::new();
    if contains_any(text, &["believe", "belief", "think", "믿", "생각"]) {
        attitudes.extend([AttributionAttitudeIR::Believe, AttributionAttitudeIR::Think]);
    }
    if contains_any(
        text,
        &["know", "knew", "knowledge", "안다고", "알고", "알았"],
    ) {
        attitudes.push(AttributionAttitudeIR::Know);
    }
    if contains_any(text, &["want", "wanted", "원해", "원했", "바라"]) {
        attitudes.push(AttributionAttitudeIR::Want);
    }
    if contains_any(text, &["expect", "expected", "예상", "기대"]) {
        attitudes.push(AttributionAttitudeIR::Expect);
    }
    if contains_any(
        text,
        &[
            "say",
            "said",
            "statement",
            "말",
            "report",
            "보고",
            "claim",
            "주장",
        ],
    ) {
        attitudes.extend(SPEECH_REPORT_ATTITUDES);
    }
    attitudes.sort();
    attitudes.dedup();
    attitudes
}

const SPEECH_REPORT_ATTITUDES: [AttributionAttitudeIR; 4] = [
    AttributionAttitudeIR::Say,
    AttributionAttitudeIR::Report,
    AttributionAttitudeIR::Claim,
    AttributionAttitudeIR::Correct,
];

pub(crate) fn query_topic_terms(text: &str, source: Option<&str>) -> Vec<String> {
    let source_terms = source.map_or_else(BTreeSet::new, normalized_terms);
    // The inflected matrix request is a function, not the topic being asked
    // about. Use its parsed role instead of adding every ending to stop words.
    let analysis = crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(text);
    let matrix_terms = analysis
        .frames
        .iter()
        .filter(|f| {
            f.external_execution_authorized
                && f.intent_hint == dockable_semantic_core::PlanIntentIR::Explain
                && analysis
                    .clause_graph
                    .node_for_frame(&f.frame_id)
                    .is_some_and(|n| n.function.permits_independent_directive())
        })
        .filter_map(|f| {
            text.get(f.source_start_byte..)?
                .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
                .next()
        })
        .filter_map(normalize_term)
        .collect::<BTreeSet<_>>();
    normalized_terms(text)
        .into_iter()
        .filter(|term| !source_terms.contains(term))
        .filter(|term| !matrix_terms.contains(term))
        .filter(|term| !is_query_function_term(term))
        .collect()
}

fn is_query_function_term(term: &str) -> bool {
    QUERY_STOP_WORDS.contains(&term)
        || [
            "say", "said", "report", "believ", "think", "know", "knew", "want", "expect", "realiz",
            "discover",
        ]
        .iter()
        .any(|stem| term.is_ascii() && term.starts_with(stem))
        || [
            "말", "보고", "믿", "생각", "알", "원", "예상", "기대", "깨달", "발견", "있어", "있는",
        ]
        .iter()
        .any(|stem| !term.is_ascii() && term.contains(stem))
}

const QUERY_STOP_WORDS: &[&str] = &[
    "one",
    "뭘",
    "이유만",
    "이유",
    "원인",
    "그",
    "설명해",
    "설명",
    "explain",
    "reason",
    "cause",
    "only",
    "what",
    "which",
    "who",
    "did",
    "does",
    "do",
    "is",
    "are",
    "was",
    "were",
    "it",
    "that",
    "the",
    "a",
    "an",
    "about",
    "according",
    "actually",
    "really",
    "true",
    "fact",
    "known",
    "know",
    "knew",
    "say",
    "said",
    "claim",
    "claimed",
    "report",
    "reported",
    "believe",
    "belief",
    "think",
    "thought",
    "possible",
    "possibility",
    "merely",
    "actual",
    "modal",
    "prediction",
    "hypothetical",
    "counterfactual",
    "conflict",
    "disagree",
    "why",
    "when",
    "how",
    "before",
    "earlier",
    "previously",
    "or",
    "and",
    "to",
    "of",
    "in",
    "on",
    "for",
    "from",
    "with",
    "뭐",
    "무엇",
    "누가",
    "누구",
    "어떤",
    "했어",
    "말했어",
    "말해",
    "주장",
    "보고",
    "믿어",
    "믿음",
    "생각",
    "알아",
    "알고",
    "사실",
    "실제로",
    "정말",
    "확실해",
    "확인",
    "가능성",
    "가능",
    "가정",
    "반사실",
    "충돌",
    "상충",
    "반대",
    "반대로",
    "둘",
    "둘이",
    "어느",
    "쪽",
    "왜",
    "언제",
    "어떻게",
    "전에",
    "이전에",
    "대한",
    "대해",
    "인지",
    "이야",
];

fn normalized_terms(text: &str) -> BTreeSet<String> {
    text.split(|character: char| {
        character.is_whitespace()
            || character.is_ascii_punctuation()
            || matches!(character, '‘' | '’' | '“' | '”' | '「' | '」' | '『' | '』')
    })
    .filter_map(normalize_term)
    .collect()
}

fn normalize_term(raw: &str) -> Option<String> {
    let mut term = raw.trim().to_lowercase();
    if term.is_empty() {
        return None;
    }
    // Function words already have a grammatical role. Stripping a noun case
    // suffix from e.g. nominative WH 누가 would invent a content term 누.
    if QUERY_STOP_WORDS.contains(&term.as_str()) {
        return Some(term);
    }
    if !term.is_ascii() {
        for suffix in [
            "이라고",
            "라고",
            "이라는",
            "라는",
            "인지",
            "이야",
            "인가",
            "에서",
            "에게",
            "으로",
            "는",
            "은",
            "이",
            "가",
            "을",
            "를",
            "의",
            "에",
            "도",
        ] {
            if term.ends_with(suffix) && term.len() > suffix.len() {
                term.truncate(term.len() - suffix.len());
                break;
            }
        }
    } else {
        term = term.trim_end_matches("'s").to_string();
    }
    (!term.is_empty()).then_some(term)
}

fn source_nominal_forms(
    state: &ConversationStateIR,
    evidence: &[DiscourseAnswerEvidenceIR],
) -> Vec<crate::korean_nominal::KoreanNominalFormIR> {
    let mut forms = Vec::new();
    for entity in &state.active_typed_entities {
        if evidence
            .iter()
            .any(|e| normalize_actor(&e.source_actor) == entity.normalized_label)
        {
            crate::korean_nominal::merge_forms(&mut forms, &entity.korean_nominal_forms);
        }
    }
    forms
}

fn normalize_actor(actor: &str) -> String {
    normalize_space(&actor.to_lowercase())
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_string()
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_question(text: &str) -> bool {
    if crate::grammatical_scope::embedded_information_statement(text).is_some() {
        return false;
    }
    let embedded_wh_directive = [
        "learn how ",
        "explain how ",
        "show how ",
        "tell me how ",
        "describe how ",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix));
    if embedded_wh_directive && !text.ends_with('?') {
        return false;
    }
    text.ends_with('?')
        || contains_any(
            text,
            &[
                "what ",
                "who ",
                "is it ",
                "are they ",
                "do we ",
                "why ",
                "when ",
                "how ",
                "뭐",
                "무엇",
                "누가",
                "사실이야",
                "확실해",
                "왜 ",
                "언제 ",
                "어떻게 ",
            ],
        )
}

fn source_content_question(text: &str, source: Option<&str>) -> bool {
    // A fully consumed event query owns its predicate. Entity names and
    // objects cannot supply an attribution verb through substring overlap.
    let event = crate::proposition_content::described_event(text, true);
    let predicate = event
        .as_ref()
        .map_or(text, |e| e.predicate_surface.as_str());
    source.is_some()
        && contains_any(text, &["what", "뭐", "무엇", "어떤", "내용"])
        && contains_any(
            predicate,
            &[
                "say", "said", "claim", "report", "believe", "think", "know", "want", "expect",
                "말", "주장", "보고", "믿", "생각", "알", "원", "예상", "기대",
            ],
        )
}

fn proposition_source_question(text: &str) -> bool {
    let event = crate::proposition_content::described_event(text, true);
    let predicate = event
        .as_ref()
        .map_or(text, |e| e.predicate_surface.as_str());
    contains_any(text, &["who ", "who's ", "누가", "누구"])
        && contains_any(
            predicate,
            &[
                "say", "said", "claim", "report", "believe", "think", "know", "말", "주장", "보고",
                "믿", "생각", "알",
            ],
        )
}

pub(crate) fn owns_proposition_reference(text: &str) -> bool {
    let lower = normalize_space(&text.to_lowercase());
    quotative_content_question(&lower)
        || (looks_like_question(&lower) && proposition_source_question(&lower))
}

/// WH complement + quotative case, or an English WH gap in a reported-speech
/// construction. This consumes the whole fragment; it contains no answer data.
pub(crate) fn quotative_content_question(text: &str) -> bool {
    let lower = text
        .trim()
        .trim_end_matches(['?', '？', '.', '!'])
        .trim()
        .to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    if let [word] = words.as_slice() {
        let word = word.strip_suffix('요').unwrap_or(word);
        return word.strip_suffix("이라고") == Some("무엇")
            || word.strip_suffix("라고") == Some("뭐");
    }
    let report = |word: &str| matches!(word, "said" | "reported" | "claimed");
    match words.as_slice() {
        [predicate, "what"] => report(predicate),
        ["what", auxiliary, predicate] if matches!(*auxiliary, "was" | "is") => report(predicate),
        _ => false,
    }
}

fn attribution_reference_slot(
    query: &DiscourseQueryIR,
) -> Option<crate::proposition_content::ContentSlotIR> {
    if source_reference_query(query) {
        Some(crate::proposition_content::ContentSlotIR::Source)
    } else if query.kind == DiscourseQueryKindIR::SourceContent
        && quotative_content_question(&query.original_text)
    {
        Some(crate::proposition_content::ContentSlotIR::Summary)
    } else {
        None
    }
}

// Lexical anaphors have a proposition role only under the source-query
// operator. They supply no source or answer, and do not globally mean an action.
fn proposition_deictic(term: &str) -> bool {
    matches!(
        term,
        "that" | "it" | "so" | "그것" | "그거" | "그걸" | "그렇게"
    )
}

fn source_reference_query(query: &DiscourseQueryIR) -> bool {
    query.kind == DiscourseQueryKindIR::PropositionSources
        && query.topic_terms.is_empty()
        && normalized_terms(&query.original_text)
            .iter()
            .any(|term| proposition_deictic(term))
}

fn actuality_question(text: &str) -> bool {
    let explicit = contains_any(
        text,
        &[
            "actually true",
            "really true",
            "is it true",
            "do we know whether",
            "what is actually known",
            "is that a fact",
            "사실이야",
            "사실인가",
            "실제로 사실",
            "정말 사실",
            "확실해",
            "확인됐",
            "실제로 확인",
        ],
    );
    let english_actuality = text
        .split_whitespace()
        .next()
        .is_some_and(|word| matches!(word, "is" | "are" | "was" | "were"))
        && contains_any(text, &["actually", "really"]);
    let korean_actuality = contains_any(text, &["실제로", "정말"])
        && (text.ends_with('?') || contains_any(text, &["어?", "아?", "야?", "니?", "나?", "까?"]));
    explicit || english_actuality || korean_actuality
}

fn generic_certainty_question(text: &str) -> bool {
    if text.split_whitespace().next() == Some("which")
        && text
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word == "true")
    {
        return true;
    }
    contains_any(
        text,
        &[
            "what is certain",
            "what's certain",
            "what do we know for certain",
            "확실한 건 뭐",
            "확실한 게 뭐",
            "뭐가 확실",
        ],
    )
}

fn outcome_alternative_question(text: &str) -> bool {
    looks_like_question(text)
        && ((contains_any(text, &["succeed", "success"])
            && contains_any(text, &["fail", "failure"]))
            || (text.contains("성공") && text.contains("실패")))
}

fn modal_status_question(text: &str) -> bool {
    contains_any(
        text,
        &[
            "possible or actual",
            "possibility or fact",
            "merely possible",
            "hypothetical or actual",
            "prediction or fact",
            "counterfactual or actual",
            "counterfactual or fact",
            "what kind of possibility",
            "what modal",
            "가능성이야",
            "가능성인지",
            "사실인지",
            "가정이야",
            "반사실",
            "예측이야",
        ],
    )
}

fn conflict_question(text: &str) -> bool {
    contains_any(
        text,
        &[
            "in conflict",
            "conflicting",
            "disagree",
            "contradict",
            "different accounts",
            "충돌",
            "상충",
            "서로 달라",
            "다르게 말",
            "누구 말이 달라",
            "반대로 말",
            "서로 반대",
            "둘이 반대",
        ],
    )
}

fn presuppositional_question(text: &str) -> bool {
    (!quoted_question_is_only_embedded(text)
        && contains_any(text, &["why ", "when ", "how ", "왜 ", "언제 ", "어떻게 "])
        && !nonpast_method_question(text)
        && !hypothetical_or_conditional_question(text)
        && !contains_any(text, &["why does", "why do", "왜 믿", "왜 생각"]))
        || contains_any(
            text,
            &[
                "did realize that",
                "did discover that",
                "did know that",
                " realize that",
                " discover that",
                " know that",
                "깨달았",
                "발견했",
            ],
        )
}

/// A question scoped inside a hypothetical or conditional world asks for a
/// choice/implication under that assumption. Its open relation must not turn
/// the assumed event into an actual-world presupposition.
fn hypothetical_or_conditional_question(text: &str) -> bool {
    use crate::compositional_semantics::CompositionalSemanticAnalyzer;

    let analysis = CompositionalSemanticAnalyzer.analyze(text);
    matches!(
        analysis.modal_scope_graph.root_world,
        ModalWorldIR::Hypothetical | ModalWorldIR::Counterfactual
    )
}

/// Korean `어떨까` is an outcome-seeking question form. It can follow a
/// conditional antecedent without naming a manner/cause slot, so keep it in
/// the modal/discourse owner instead of sending the unresolved surface to the
/// factual world deliberator. This is deliberately a suffix-level grammar
/// check; ordinary hypothetical premise queries remain world-owned.
fn hypothetical_outcome_question(text: &str) -> bool {
    let normalized = text.trim().trim_end_matches(['?', '？', '.', '!']).trim();
    normalized.ends_with("어떨까") || normalized.ends_with("어떨까요")
}

/// Shared routing boundary for non-actual questions. World deliberation must
/// not treat an advice/manner question inside a hypothetical as a closed-world
/// premise query before discourse QA can own its answer.
pub(crate) fn non_actual_world_question(text: &str) -> bool {
    let lower = text.to_lowercase();
    looks_like_question(text)
        && !quoted_question_is_only_embedded(text)
        && (matches!(
            crate::proposition_content::requested_content_slot(text),
            Some(
                crate::proposition_content::ContentSlotIR::Manner
                    | crate::proposition_content::ContentSlotIR::Cause
            )
        ) || contains_any(&lower, &["how ", "why ", "어떻게 ", "왜 "])
            || hypothetical_outcome_question(&lower))
        && hypothetical_or_conditional_question(&lower)
}

/// A positive, non-past method question is not an execution report. Reuse the
/// parsed predicate's mood and time instead of granting `how` occurrence force.
/// A quoted question in a declarative report belongs to reported content;
/// only a terminal question mark promotes the matrix clause to a live query.
fn quoted_question_is_only_embedded(text: &str) -> bool {
    if !text.contains(['"', '“', '”', '‘', '’']) || !text.contains(['?', '？']) {
        return false;
    }
    let analysis = crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(text);
    let Some(frame) = analysis.frames.first() else {
        return false;
    };
    frame.embedded_under_quote
        && analysis.frames.iter().all(|candidate| {
            candidate.embedded_under_quote
                || candidate.mood == crate::compositional_semantics::FrameMoodIR::Reported
        })
}

fn nonpast_method_question(text: &str) -> bool {
    use crate::compositional_semantics::{
        CompositionalSemanticAnalyzer, FrameMoodIR, FramePolarityIR, FrameTemporalReferenceIR,
    };
    if crate::proposition_content::requested_content_slots(text)
        != [crate::proposition_content::ContentSlotIR::Manner]
    {
        return false;
    }
    let analysis = CompositionalSemanticAnalyzer.analyze(text);
    let [frame] = analysis.frames.as_slice() else {
        return false;
    };
    frame.mood == FrameMoodIR::Interrogative
        && frame.temporal_reference != FrameTemporalReferenceIR::Past
        && frame.polarity == FramePolarityIR::Positive
        && !frame.embedded_under_quote
        && analysis
            .clause_graph
            .node_for_frame(&frame.frame_id)
            .is_some_and(|node| node.function.permits_independent_directive())
}

fn presupposed_surface(text: &str) -> String {
    for prefix in [
        "why did ",
        "when did ",
        "how did ",
        "왜 ",
        "언제 ",
        "어떻게 ",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return rest.trim_end_matches('?').trim().to_string();
        }
    }
    text.trim_end_matches('?').trim().to_string()
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn realize_answer(
    language: LanguageCodeIR,
    query: &DiscourseQueryIR,
    disposition: DiscourseAnswerDispositionIR,
    evidence: &[DiscourseAnswerEvidenceIR],
) -> String {
    if query.kind == DiscourseQueryKindIR::ModalStatus && !evidence.is_empty() {
        let records = evidence
            .iter()
            .map(|item| match language {
                LanguageCodeIR::Korean => format!(
                    "‘{}’는 {} 기록이야",
                    item.proposition_surface,
                    korean_modal_world(item.modal_world)
                ),
                _ => format!(
                    "‘{}’ is recorded as {}",
                    item.proposition_surface,
                    english_modal_world(item.modal_world)
                ),
            })
            .collect::<Vec<_>>()
            .join("; ");
        return match language {
            LanguageCodeIR::Korean => {
                format!("대화 기록상 {records}. 이 분류는 실제 세계의 사실 확정이 아니야.")
            }
            _ => format!(
                "According to the dialogue record, {records}. This modal classification does not establish actual-world truth."
            ),
        };
    }
    if query.kind == DiscourseQueryKindIR::PropositionSources && !evidence.is_empty() {
        let records = evidence
            .iter()
            .map(|item| match language {
                LanguageCodeIR::Korean => format!(
                    "{}가 ‘{}’를 {:?} 상태로 남겼어",
                    item.source_actor, item.proposition_surface, item.epistemic_status
                ),
                _ => format!(
                    "{} is the source of the {:?} record ‘{}’",
                    item.source_actor, item.epistemic_status, item.proposition_surface
                ),
            })
            .collect::<Vec<_>>()
            .join("; ");
        return match language {
            LanguageCodeIR::Korean => {
                format!("대화 기록상 {records}. 출처 식별이지 사실 확정은 아니야.")
            }
            _ => format!(
                "According to the dialogue record, {records}. This identifies recorded sources; it does not establish the proposition as fact."
            ),
        };
    }
    match (language, disposition) {
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords)
        | (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::MultipleDialogueRecords) => {
            let records = evidence
                .iter()
                .map(|item| {
                    format!(
                        "{}는 ‘{}’라고 {:?} 상태로 남아 있어",
                        item.source_actor, item.proposition_surface, item.epistemic_status
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("대화 기록상 {records}. 이것은 출처별 발화·태도 기록이며 사실 확정은 아니야.")
        }
        (_, DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords)
        | (_, DiscourseAnswerDispositionIR::MultipleDialogueRecords) => {
            let records = evidence
                .iter()
                .map(|item| {
                    format!(
                        "{} is recorded as {:?} ‘{}’",
                        item.source_actor, item.epistemic_status, item.proposition_surface
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!("According to the dialogue record, {records}. These are source-attributed records, not established facts.")
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::DialogueTruthNotEstablished) => {
            if evidence.is_empty() {
                "일치하는 검증 기록이 없고, 대화 상태에도 그 내용을 사실로 확정한 근거가 없어. 사실이라고 답할 수 없어.".to_string()
            } else {
                format!(
                    "관련 발화 기록은 {}개 있지만 모두 출처의 주장·믿음·관찰 기록일 뿐 대화에서 검증된 사실은 아니야. 따라서 실제로 참이라고 확정할 수 없어.",
                    evidence.len()
                )
            }
        }
        (_, DiscourseAnswerDispositionIR::DialogueTruthNotEstablished) => {
            if evidence.is_empty() {
                "There is no matching verified record, and the conversation state does not establish that proposition as true.".to_string()
            } else {
                format!(
                    "There are {} matching source-attributed record(s), but none is established as dialogue-grounded truth, so I cannot say it is actually true.",
                    evidence.len()
                )
            }
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::ConflictingDialogueRecords) => {
            let sources = evidence.iter().map(|item| item.source_actor.as_str()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>().join(", ");
            format!("{sources}의 관련 기록이 서로 충돌한 상태야. 어느 출처도 사실 승자로 선택하지 않았어.")
        }
        (_, DiscourseAnswerDispositionIR::ConflictingDialogueRecords) => {
            let sources = evidence.iter().map(|item| item.source_actor.as_str()).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>().join(", ");
            format!("The matching records from {sources} are in conflict. No source has been selected as the truth winner.")
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::NoConflictRecorded) => {
            "현재 일치하는 활성 기록에서는 출처 간 충돌이 확인되지 않아. 이것이 명제의 참을 의미하지는 않아.".to_string()
        }
        (_, DiscourseAnswerDispositionIR::NoConflictRecorded) => {
            "No matching active source conflict is recorded. That does not establish the proposition itself as true.".to_string()
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::PresuppositionUnverified) => {
            let premise = query.presuppositions.first().map_or("질문의 전제", |item| item.surface_text.as_str());
            format!("질문은 ‘{premise}’를 전제로 하지만 그 전제는 대화에서 사실로 검증되지 않았어. 전제를 몰래 받아들이지 않고 왜·언제를 답하지 않을게.")
        }
        (_, DiscourseAnswerDispositionIR::PresuppositionUnverified) => {
            let premise = query.presuppositions.first().map_or("the question premise", |item| item.surface_text.as_str());
            format!("The question presupposes ‘{premise}’, but that premise is not established as true in the dialogue. I will not answer why or when by silently accepting it.")
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::NoMatchingRecord) => {
            "조건에 맞는 대화 기록을 찾지 못했어. 없는 출처나 내용을 추측해서 채우지 않을게.".to_string()
        }
        (_, DiscourseAnswerDispositionIR::NoMatchingRecord) => {
            "I found no matching dialogue record. I will not invent a source or proposition to fill the gap.".to_string()
        }
        (LanguageCodeIR::Korean, DiscourseAnswerDispositionIR::AmbiguousQuery) => {
            "어느 출처나 주장을 묻는지 하나로 정해지지 않아. 대상 출처나 내용을 지정해줘.".to_string()
        }
        (_, DiscourseAnswerDispositionIR::AmbiguousQuery) => {
            "The question does not identify one source or proposition. Please specify the source or content.".to_string()
        }
    }
}

fn english_modal_world(world: ModalWorldIR) -> &'static str {
    match world {
        ModalWorldIR::Actual => "an actual-world assertion",
        ModalWorldIR::EpistemicPossible => "an epistemic possibility",
        ModalWorldIR::EpistemicProbable => "an epistemic probability",
        ModalWorldIR::EpistemicCertain => "source-presented certainty",
        ModalWorldIR::Normative => "a normative claim",
        ModalWorldIR::Ability => "an ability claim",
        ModalWorldIR::Desired => "a desired world",
        ModalWorldIR::Intended => "an intended world",
        ModalWorldIR::Predicted => "a prediction",
        ModalWorldIR::Hypothetical => "a hypothetical world",
        ModalWorldIR::Counterfactual => "a counterfactual world",
        ModalWorldIR::Questioned => "a questioned proposition",
    }
}

fn korean_modal_world(world: ModalWorldIR) -> &'static str {
    match world {
        ModalWorldIR::Actual => "현실 세계 주장",
        ModalWorldIR::EpistemicPossible => "인식적 가능성",
        ModalWorldIR::EpistemicProbable => "인식적 개연성",
        ModalWorldIR::EpistemicCertain => "출처가 확실하다고 제시한 주장",
        ModalWorldIR::Normative => "규범 주장",
        ModalWorldIR::Ability => "능력 주장",
        ModalWorldIR::Desired => "희망 세계",
        ModalWorldIR::Intended => "의도 세계",
        ModalWorldIR::Predicted => "예측",
        ModalWorldIR::Hypothetical => "가정 세계",
        ModalWorldIR::Counterfactual => "반사실 세계",
        ModalWorldIR::Questioned => "의문으로 제시된 명제",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribution::{AttributedPropositionPolarityIR, AttributionAttitudeIR};
    use crate::epistemic::{EpistemicLedgerIR, EpistemicObservationIR};

    #[test]
    fn answer_sets_preserve_attribution_bounds_and_reject_nested_or_forged_members() {
        let records = [
            (
                "DIALOGUE_USER",
                "Mira read a letter.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "DIALOGUE_USER",
                "Noel read a letter.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ];
        let question = "Who read a letter?";
        let state = state_with(&records);
        let answer = DiscourseQaEngine
            .answer(question, Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(answer.validate());
        let projection = answer.content_projection.as_ref().unwrap();
        assert_eq!(projection.co_answers.len(), 1);
        let mut nested = projection.clone();
        nested.co_answers[0].co_answers.push(projection.clone());
        assert!(!nested.validate());
        let mut wrong_source = projection.clone();
        wrong_source.co_answers[0].source_actor = "Alice".into();
        assert!(!wrong_source.validate());
        let mut missing_evidence = answer.clone();
        missing_evidence.evidence.pop();
        assert!(!missing_evidence.validate());
        let mut wrong_query = answer.clone();
        wrong_query.query.original_text = "Who read a newspaper?".into();
        assert!(!wrong_query.validate());
        let mut split_sources = records;
        split_sources[1].0 = "Alice";
        let split = state_with(&split_sources);
        assert!(DiscourseQaEngine
            .answer(question, Some(&split), LanguageCodeIR::English)
            .unwrap()
            .content_projection
            .is_none());
        let mut possible = state;
        possible.epistemic_ledger.records[1].signature.modal_world =
            ModalWorldIR::EpistemicPossible;
        let actual_only = DiscourseQaEngine
            .answer(question, Some(&possible), LanguageCodeIR::English)
            .unwrap();
        assert!(actual_only
            .content_projection
            .unwrap()
            .co_answers
            .is_empty());
    }

    #[test]
    fn explanation_argument_gaps_are_not_missing_knowledge() {
        for (raw, kind, ko, en) in [
            (
                "짧게만 말하지 말고, 이번엔 이유도 설명해줘.",
                DiscourseQueryKindIR::MissingExplanationTarget,
                "무엇을 설명해줄까?",
                "What should I explain?",
            ),
            (
                "아직 아무것도 바꾸지 말고, 왜 이런 차이가 나는지 설명만 듣고 싶어.",
                DiscourseQueryKindIR::MissingComparisonOperands,
                "무엇과 무엇을 비교해줄까?",
                "Which two things should I compare?",
            ),
            (
                "Could you explain the difference?",
                DiscourseQueryKindIR::MissingComparisonOperands,
                "무엇과 무엇을 비교해줄까?",
                "Which two things should I compare?",
            ),
            (
                "Please explain that.",
                DiscourseQueryKindIR::MissingExplanationTarget,
                "무엇을 설명해줄까?",
                "What should I explain?",
            ),
        ] {
            assert_eq!(unbound_explanation_target(raw), Some(kind), "{raw}");
            let mut semantic_hash = None;
            for (lang, expected) in [(LanguageCodeIR::Korean, ko), (LanguageCodeIR::English, en)] {
                let mut answer = DiscourseQaEngine
                    .clarify_unbound_explanation(raw, None, lang)
                    .expect("missing target");
                assert!(answer.validate());
                assert_eq!(
                    Some(answer.realized_text.clone()),
                    answer.structured_meaning_preview()
                );
                assert!(answer.evidence.is_empty() && answer.claims.is_empty());
                let generated =
                    crate::generative_language::generate_information_target_question(lang, kind)
                        .unwrap();
                assert!(generated.validate());
                assert_eq!(generated.morphology.realized_text, expected);
                if let Some(prior) = semantic_hash.as_ref() {
                    assert_eq!(&generated.meaning.semantic_sha256, prior);
                }
                semantic_hash = Some(generated.meaning.semantic_sha256);
                answer.query.original_text = "Please explain entropy.".into();
                assert!(!answer.validate(), "target-gap evidence must be replayable");
            }
        }
        for raw in [
            "왜 서버가 멈췄어?",
            "Please explain entropy.",
            "Explain the difference between A and B.",
            "이유를 설명한 사람은 누구야?",
            "Who said explain that?",
        ] {
            assert!(
                unbound_explanation_target(raw).is_none(),
                "explicit/unknown target: {raw}"
            );
        }
        let state = state_with(&[(
            "DIALOGUE_USER",
            "서버가 멈췄어.",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        assert!(DiscourseQaEngine
            .clarify_unbound_explanation("왜?", Some(&state), LanguageCodeIR::Korean)
            .is_none());
    }

    fn state_with(
        records: &[(&str, &str, ModalWorldIR, AttributionAttitudeIR)],
    ) -> ConversationStateIR {
        let mut state = ConversationStateIR {
            schema: crate::conversation::CONVERSATION_STATE_SCHEMA.to_string(),
            conversation_id: "QA-TEST".to_string(),
            completed_turns: 0,
            active_subject: None,
            answer_focus: None,
            dialogue_world: Default::default(),
            active_referents: Vec::new(),
            active_topics: Vec::new(),
            discourse_focus: Default::default(),
            topic_context_graph: Default::default(),
            active_goals: Vec::new(),
            active_discourse_programs: Vec::new(),
            action_state_ledger: Default::default(),
            deferred_action_commitments: Vec::new(),
            active_discourse_referents: Vec::new(),
            active_discourse_groups: Vec::new(),
            active_typed_entities: Vec::new(),
            epistemic_ledger: EpistemicLedgerIR::default(),
            temporal_graph: crate::temporal::TemporalGraphIR::default(),
            conditional_guard_store: Default::default(),
            dialogue_relation_graph: Default::default(),
            dialogue_directive_ledger: Default::default(),
            last_guard_evaluations: Vec::new(),
            preferred_language: Some(LanguageCodeIR::English),
            pending_question: None,
            topic_pending_questions: Vec::new(),
            unresolved_reference_count: 0,
            state_sha256: String::new(),
        };
        for (index, (source, proposition, world, attitude)) in records.iter().enumerate() {
            let turn = u64::try_from(index + 1).expect("bounded test turn");
            state.epistemic_ledger.apply_turn(
                turn,
                proposition,
                &[],
                &[EpistemicObservationIR {
                    origin_referent_id: format!("P-{turn}"),
                    source_actor: (*source).to_string(),
                    proposition_surface: (*proposition).to_string(),
                    proposition_polarity: AttributedPropositionPolarityIR::Positive,
                    modal_world: *world,
                    attribution_attitude: *attitude,
                    epistemic_status: match attitude {
                        AttributionAttitudeIR::Know => EpistemicStatusIR::PresentedAsKnown,
                        AttributionAttitudeIR::Believe | AttributionAttitudeIR::Think => {
                            EpistemicStatusIR::Believed
                        }
                        _ => EpistemicStatusIR::Reported,
                    },
                }],
            );
            state.completed_turns = turn;
        }
        state
    }

    #[test]
    fn source_anaphor_uses_focus_and_preserves_missing_ambiguous_and_modal_boundaries() {
        let mut state = state_with(&[
            (
                "Alice",
                "the client is worried",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "Bob",
                "the nurse is tired",
                ModalWorldIR::EpistemicPossible,
                AttributionAttitudeIR::Believe,
            ),
        ]);
        let first_id = state.epistemic_ledger.records[0].belief_id.clone();
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: Some(first_id.clone()),
            query: DiscourseQaEngine
                .parse("What did Alice say?", Some(&state))
                .unwrap(),
            answered_turn: state.completed_turns,
            described_event: None,
            clarification_act: None,
            decision_inquiry: None,
        });
        for query in ["Who said that?", "Who said it?", "누가 그렇게 말했어?"] {
            let answer = DiscourseQaEngine
                .answer(query, Some(&state), LanguageCodeIR::English)
                .unwrap();
            assert!(answer.validate());
            assert_eq!(answer.evidence.len(), 1);
            assert_eq!(answer.evidence[0].belief_id, first_id);
            let mut forged = answer.clone();
            forged.contextual_target.as_mut().unwrap().belief_id = "unrelated".into();
            assert!(!forged.validate());
        }
        // An explicit content argument wins over old focus; 'that' here can
        // introduce that argument instead of naming the previous proposition.
        let explicit = DiscourseQaEngine
            .answer(
                "Who believes that the nurse is tired?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert_eq!(explicit.evidence[0].source_actor, "Bob");
        assert!(explicit.contextual_target.is_none());
        state.answer_focus = None;
        let modal = DiscourseQaEngine
            .answer("Who believes that?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert_eq!(modal.evidence[0].source_actor, "Bob");
        assert_eq!(
            modal.evidence[0].modal_world,
            ModalWorldIR::EpistemicPossible
        );
        assert!(!modal.dialogue_truth_established);
        // Two current proposition candidates cannot be silently narrowed by
        // the availability of one desired attitude or answer.
        state.epistemic_ledger.records[0].introduced_turn = state.completed_turns;
        for context in [None, Some(&state)] {
            let gap = DiscourseQaEngine
                .answer("Who said that?", context, LanguageCodeIR::English)
                .unwrap();
            assert_eq!(
                gap.disposition,
                DiscourseAnswerDispositionIR::AmbiguousQuery
            );
            assert!(gap.evidence.is_empty());
            assert!(gap.validate());
        }
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: Some(first_id),
            query: DiscourseQaEngine
                .parse("What did Alice say?", Some(&state))
                .unwrap(),
            answered_turn: state.completed_turns,
            described_event: None,
            clarification_act: None,
            decision_inquiry: None,
        });
        state.epistemic_ledger.records[0].status = BeliefRecordStatusIR::Retracted;
        let gap = DiscourseQaEngine
            .answer("Who said that?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert_eq!(
            gap.disposition,
            DiscourseAnswerDispositionIR::AmbiguousQuery
        );
        assert!(gap.evidence.is_empty());
    }

    #[test]
    fn source_identity_grouping_does_not_duplicate_case_variants() {
        let state = state_with(&[
            (
                "McKay",
                "the client is worried",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "mckay",
                "the client is worried",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let answer = DiscourseQaEngine
            .answer(
                "Who said the client is worried?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert!(answer.validate());
        let values = answer.focused_source_values().unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values.values().next().unwrap().len(), answer.evidence.len());
        assert!(values.keys().all(|k| k.to_lowercase() == "mckay"));
    }

    #[test]
    fn source_focus_realizes_evidence_values_without_asserting_the_embedded_proposition() {
        use crate::generative_language::generate_discourse_answer_from_knowledge;
        for (sources, proposition, question, language, expected) in [
            (
                vec!["관리인"],
                "간호사가 피곤하다",
                "누가 말했어?",
                LanguageCodeIR::Korean,
                "관리인이야.",
            ),
            (
                vec!["기자"],
                "학생이 불안하다",
                "누가 말했어?",
                LanguageCodeIR::Korean,
                "기자야.",
            ),
            (
                vec!["Alice", "Bob"],
                "the client is worried",
                "Who said the client is worried?",
                LanguageCodeIR::English,
                "Alice and Bob.",
            ),
            (
                vec!["DIALOGUE_USER"],
                "the client is worried",
                "Who said that?",
                LanguageCodeIR::English,
                "You.",
            ),
        ] {
            let rows = sources
                .iter()
                .map(|s| {
                    (
                        *s,
                        proposition,
                        ModalWorldIR::Actual,
                        AttributionAttitudeIR::Say,
                    )
                })
                .collect::<Vec<_>>();
            let state = state_with(&rows);
            let answer = DiscourseQaEngine
                .answer(question, Some(&state), language)
                .unwrap();
            assert!(answer.validate());
            let values = answer.focused_source_values().unwrap();
            assert_eq!(values.len(), sources.len());
            let generated =
                generate_discourse_answer_from_knowledge(language, &answer, &[]).unwrap();
            assert_eq!(generated.morphology.realized_text, expected);
            assert!(!generated.morphology.realized_text.contains(proposition));
            assert_eq!(generated.verification.unsupported_claims, 0);
            assert!(!answer.dialogue_truth_established);
            let mut forged = answer.clone();
            forged.evidence[0].source_actor = "Mallory".into();
            assert!(forged.focused_source_values().is_none());
            for status in [EpistemicStatusIR::Denied, EpistemicStatusIR::Doubted] {
                let mut qualified = answer.clone();
                qualified.evidence[0].epistemic_status = status;
                qualified.claims = answer_claims(&qualified.query, &qualified.evidence).1;
                assert!(
                    qualified.focused_source_values().is_none(),
                    "source qualification must not disappear"
                );
            }
        }
        let state = state_with(&[
            (
                "Alice",
                "the client is worried",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "Bob",
                "the client is tired",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let distinct = DiscourseQaEngine
            .answer(
                "Who said something about the client?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert_eq!(distinct.evidence.len(), 2);
        assert!(
            distinct.focused_source_values().is_none(),
            "different propositions must retain their source/content associations"
        );
        let content = DiscourseQaEngine
            .answer("What did Alice say?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(content.focused_source_values().is_none());
        let gap = DiscourseQaEngine
            .answer("Who said that?", None, LanguageCodeIR::English)
            .unwrap();
        assert!(gap.focused_source_values().is_none());
    }

    #[test]
    fn shared_proposition_focus_preserves_sources_and_replays_membership() {
        let mut state = state_with(&[
            (
                "Alice",
                "the visitor is tired.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "Bob",
                "the visitor is tired",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let answer = DiscourseQaEngine
            .answer(
                "Who said the visitor is tired?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .unwrap();
        let shared = answer
            .focused_shared_proposition(&state)
            .expect("punctuation is not a different proposition");
        assert_eq!(shared.belief_ids.len(), 2);
        assert!(shared.validate_in(&state));
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: Some(shared.clone()),
            proposition_belief_id: None,
            query: answer.query,
            answered_turn: state.completed_turns,
            described_event: None,
            clarification_act: None,
            decision_inquiry: None,
        });
        let next = DiscourseQaEngine
            .answer("Who said that?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(next.validate());
        assert_eq!(next.evidence.len(), 2);
        assert!(next.validate_response_part_memory(&state));
        let mut incomplete = next.clone();
        incomplete.evidence.pop();
        incomplete.claims = answer_claims(&incomplete.query, &incomplete.evidence).1;
        assert!(!incomplete.validate_response_part_memory(&state));
        state.epistemic_ledger.records[1].status = BeliefRecordStatusIR::Contested;
        let disputed = DiscourseQaEngine
            .answer("Who said that?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert_eq!(
            disputed.evidence.len(),
            2,
            "disagreement must not erase attribution"
        );
        assert!(!disputed.dialogue_truth_established);
        assert_eq!(
            next.contextual_target
                .as_ref()
                .unwrap()
                .shared_proposition
                .as_ref(),
            Some(&shared)
        );
        state.epistemic_ledger.records[0].status = BeliefRecordStatusIR::Retracted;
        let next = DiscourseQaEngine
            .answer("Who said that?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert_eq!(next.evidence.len(), 1);
        assert_eq!(next.evidence[0].source_actor, "Bob");
        assert!(!next.dialogue_truth_established);
        let mut forged = state.clone();
        forged
            .answer_focus
            .as_mut()
            .unwrap()
            .shared_proposition
            .as_mut()
            .unwrap()
            .meaning
            .negated = true;
        assert!(!forged
            .answer_focus
            .as_ref()
            .unwrap()
            .validate_proposition_in(&forged));
        assert!(source_context_target(&forged).is_none());
        state.completed_turns += 4;
        assert!(source_context_target(&state).is_none());
    }

    #[test]
    fn shared_proposition_identity_keeps_scope_arguments_and_finite_form() {
        for proposition in ["I am tired", "he is tired", "그 사람은 피곤하다"] {
            let state = state_with(&[(
                "Alice",
                proposition,
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            )]);
            assert!(
                PropositionMeaningIR::from_record(&state.epistemic_ledger.records[0]).is_none(),
                "unbound participant: {proposition}"
            );
        }
        for (other, world) in [
            ("the nurse is tired", ModalWorldIR::Actual),
            ("the visitor was tired", ModalWorldIR::Actual),
            ("the visitor is not tired", ModalWorldIR::Actual),
            ("the visitor is tired", ModalWorldIR::EpistemicPossible),
            (
                "the visitor is tired because the journey was long",
                ModalWorldIR::Actual,
            ),
        ] {
            let state = state_with(&[
                (
                    "Alice",
                    "the visitor is tired",
                    ModalWorldIR::Actual,
                    AttributionAttitudeIR::Say,
                ),
                ("Bob", other, world, AttributionAttitudeIR::Say),
            ]);
            let first =
                PropositionMeaningIR::from_record(&state.epistemic_ledger.records[0]).unwrap();
            assert_ne!(
                PropositionMeaningIR::from_record(&state.epistemic_ledger.records[1]).as_ref(),
                Some(&first),
                "{other}"
            );
        }
    }

    #[test]
    fn quotative_question_is_a_composed_content_request_not_a_topic_or_command() {
        for noun in ["뭐", "무엇"] {
            let case = if noun == "뭐" { "라고" } else { "이라고" };
            for courtesy in ["", "요"] {
                let text = format!("{noun}{case}{courtesy}?");
                let query = DiscourseQaEngine.parse(&text, None).unwrap();
                assert_eq!(query.kind, DiscourseQueryKindIR::SourceContent);
                assert!(query.topic_terms.is_empty());
                assert_eq!(
                    attribution_reference_slot(&query),
                    Some(crate::proposition_content::ContentSlotIR::Summary)
                );
            }
        }
        for verb in ["said", "reported", "claimed"] {
            for query in [format!("{verb} what?"), format!("What was {verb}?")] {
                assert!(quotative_content_question(&query));
            }
        }
        for text in [
            "뭐",
            "무엇",
            "'뭐라고?'",
            "무엇이라고 말하지 마",
            "뭐라고 생각해?",
            "What was repaired?",
            "He said what happened.",
        ] {
            assert!(!quotative_content_question(text), "{text}");
        }
    }

    #[test]
    fn quotative_content_binds_memory_and_preserves_qualification() {
        let state = state_with(&[(
            "관리인",
            "간호사가 피곤하다",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer("뭐라고요?", Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        assert!(answer.validate());
        assert_eq!(answer.evidence.len(), 1);
        assert!(answer.spoken_content().is_some());
        assert_eq!(
            answer.contextual_target.as_ref().unwrap().slot,
            crate::proposition_content::ContentSlotIR::Summary
        );
        let gap = DiscourseQaEngine
            .answer("뭐라고?", None, LanguageCodeIR::Korean)
            .unwrap();
        assert_eq!(
            gap.disposition,
            DiscourseAnswerDispositionIR::AmbiguousQuery
        );
        assert!(gap.evidence.is_empty());
        for status in [
            EpistemicStatusIR::Denied,
            EpistemicStatusIR::Doubted,
            EpistemicStatusIR::Believed,
        ] {
            let mut qualified = answer.clone();
            qualified.evidence[0].epistemic_status = status;
            qualified.claims = answer_claims(&qualified.query, &qualified.evidence).1;
            assert!(qualified.spoken_content().is_none());
        }
        let mut forged = answer.clone();
        forged.evidence[0].proposition_surface = "학생이 불안하다".into();
        assert!(forged.spoken_content().is_none());
    }

    #[test]
    fn source_question_returns_attributed_content_not_truth() {
        let state = state_with(&[(
            "Alice",
            "the server is down",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer("What did Alice say?", Some(&state), LanguageCodeIR::English)
            .expect("recognized source question");
        assert_eq!(
            answer.disposition,
            DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        );
        assert_eq!(answer.evidence[0].source_actor, "Alice");
        assert!(!answer.dialogue_truth_established);
        assert!(answer.validate());
    }

    #[test]
    fn source_framing_separates_shared_recall_from_qualified_evidence() {
        for (source, attitude, expected) in [
            (
                "DIALOGUE_USER",
                AttributionAttitudeIR::Say,
                AnswerSourceFramingIR::SharedDialogueRecall,
            ),
            (
                "Arin",
                AttributionAttitudeIR::Say,
                AnswerSourceFramingIR::Explicit,
            ),
            (
                "DIALOGUE_USER",
                AttributionAttitudeIR::Hear,
                AnswerSourceFramingIR::Explicit,
            ),
            (
                "DIALOGUE_USER",
                AttributionAttitudeIR::Think,
                AnswerSourceFramingIR::Explicit,
            ),
        ] {
            let state =
                state_with(&[(source, "Mira read a note.", ModalWorldIR::Actual, attitude)]);
            let answer = DiscourseQaEngine
                .answer("Who read a note?", Some(&state), LanguageCodeIR::English)
                .unwrap();
            assert!(answer.content_projection.is_some());
            assert_eq!(answer.source_framing(), expected);
            let before = answer.clone();
            let trace = crate::generative_language::generate_discourse_answer_from_knowledge(
                LanguageCodeIR::English,
                &answer,
                &[],
            )
            .unwrap();
            assert!(trace.validate());
            assert_eq!(answer, before);
            assert!(!answer.dialogue_truth_established);
            assert_eq!(
                trace
                    .meaning
                    .nodes
                    .iter()
                    .any(|n| n.concept_id.starts_with("C_CONTENT_RECALL_")),
                expected == AnswerSourceFramingIR::SharedDialogueRecall
            );
            if source == "Arin" {
                assert!(trace.morphology.realized_text.contains("Arin"));
            }
            let mut missing = answer.clone();
            missing.evidence.clear();
            assert_eq!(missing.source_framing(), AnswerSourceFramingIR::Explicit);
        }
    }

    #[test]
    fn dialogue_participant_reference_uses_language_possessive() {
        let state = state_with(&[(
            "DIALOGUE_USER",
            "Mira read a note.",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer("Who read a note?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        // This tests lexical participant realization, not a truth inference.
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let evidence = &answer.evidence[0];
            let surface = crate::generative_language::dialogue_attribution_surface(
                language,
                DiscourseQueryKindIR::SourceContent,
                evidence,
            );
            assert!(!surface.contains("DIALOGUE_USER"));
            assert!(surface.starts_with(if language == LanguageCodeIR::Korean {
                "네 "
            } else {
                "your "
            }));
            assert_eq!(evidence.source_actor, "DIALOGUE_USER");
            assert!(!evidence.dialogue_truth_established);
        }
    }

    #[test]
    fn described_event_queries_join_roles_and_abstain_on_ambiguity() {
        use crate::proposition_content::ContentSlotIR;
        let state = state_with(&[
            (
                "DIALOGUE_USER",
                "민수는 지연에게 책을 빌려줬어.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "DIALOGUE_USER",
                "하린은 도윤에게 우산을 빌려줬어.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        for (q, expected) in [
            ("민수는 누구에게 책을 빌려줬어?", Some("지연")),
            ("누가 도윤에게 우산을 빌려줬어?", Some("하린")),
            ("누가 책을 빌려줬어?", Some("민수")),
            ("민수는 누구에게 우산을 빌려줬어?", None),
        ] {
            let answer = DiscourseQaEngine
                .answer(q, Some(&state), LanguageCodeIR::Korean)
                .unwrap();
            assert_eq!(
                answer
                    .content_projection
                    .as_ref()
                    .map(|p| p.binding.value.as_str()),
                expected,
                "{q}: {answer:?}"
            );
            assert!(answer.validate(), "{q}");
        }
        let answer = DiscourseQaEngine
            .answer(
                "민수는 누구에게 책을 빌려줬어?",
                Some(&state),
                LanguageCodeIR::Korean,
            )
            .unwrap();
        let p = answer.content_projection.unwrap();
        assert_eq!(p.binding.slot, ContentSlotIR::Recipient);
        assert!(!p.matches_question("하린은 누구에게 책을 빌려줬어?"));
        let open_answer = DiscourseQaEngine
            .answer("누가 빌려줬어?", Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        assert!(open_answer.validate());
        assert_eq!(
            open_answer
                .content_projection
                .unwrap()
                .all_projections()
                .map(|p| p.binding.value.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["민수".into(), "하린".into()])
        );
    }

    #[test]
    fn described_event_focus_is_a_revalidated_source_reference() {
        let mut state = state_with(&[
            (
                "DIALOGUE_USER",
                "Mina lent a book to Jin yesterday.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "DIALOGUE_USER",
                "Nora lent a pen to Sol today.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let answer = DiscourseQaEngine
            .answer(
                "To whom did Mina lend a book?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .unwrap();
        let p = answer.content_projection.unwrap();
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: None,
            query: answer.query,
            answered_turn: 2,
            clarification_act: None,
            decision_inquiry: None,
            described_event: Some(DescribedEventReferenceIR {
                belief_id: p.belief_id.clone(),
                event_id: p.binding.event_id.unwrap(),
            }),
        });
        for (q, value) in [("What?", "book"), ("When?", "yesterday"), ("Who?", "Mina")] {
            let a = DiscourseQaEngine
                .answer(q, Some(&state), LanguageCodeIR::English)
                .unwrap();
            assert_eq!(a.content_projection.unwrap().binding.value, value);
        }
        state
            .epistemic_ledger
            .records
            .iter_mut()
            .find(|r| r.belief_id == p.belief_id)
            .unwrap()
            .status = BeliefRecordStatusIR::Retracted;
        let a = DiscourseQaEngine
            .answer("When?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(a.content_projection.is_none());
    }

    #[test]
    fn focused_korean_event_accepts_a_relative_role_followup() {
        let mut state = state_with(&[(
            "DIALOGUE_USER",
            "다희는 연구실에서 문서를 읽었어.",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let initial = DiscourseQaEngine
            .answer("다희는 뭘 읽었어?", Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        let projection = initial.content_projection.unwrap();
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: None,
            query: initial.query,
            answered_turn: state.completed_turns,
            clarification_act: None,
            decision_inquiry: None,
            described_event: Some(DescribedEventReferenceIR {
                belief_id: projection.belief_id,
                event_id: projection.binding.event_id.unwrap(),
            }),
        });
        let followup = DiscourseQaEngine
            .answer(
                "읽은 장소도 알려줄래?",
                Some(&state),
                LanguageCodeIR::Korean,
            )
            .unwrap();
        assert_eq!(
            followup
                .content_projection
                .as_ref()
                .map(|p| p.binding.value.as_str()),
            Some("연구실"),
            "{followup:?}"
        );
        assert!(followup.validate());
    }

    #[test]
    fn described_events_preserve_source_queries_and_nonactual_boundaries() {
        let state = state_with(&[(
            "Alice",
            "Mina lent a book to Jin.",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer("What did Alice say?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(answer.content_projection.is_none());
        assert_eq!(answer.query.requested_source.as_deref(), Some("Alice"));
        assert_eq!(answer.evidence.len(), 1);
        let mut possible = state.clone();
        possible.epistemic_ledger.records[0].signature.modal_world =
            ModalWorldIR::EpistemicPossible;
        let answer = DiscourseQaEngine
            .answer(
                "Who lent a book to Jin?",
                Some(&possible),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert!(answer.content_projection.is_none());
        let mut denied = state;
        denied.epistemic_ledger.records[0].proposition_polarity =
            crate::attribution::AttributedPropositionPolarityIR::Negative;
        let answer = DiscourseQaEngine
            .answer(
                "Who lent a book to Jin?",
                Some(&denied),
                LanguageCodeIR::English,
            )
            .unwrap();
        assert!(answer.content_projection.is_none());
    }

    #[test]
    fn event_references_join_distinct_events_and_revalidate_antecedents() {
        let mut state = state_with(&[
            (
                "DIALOGUE_USER",
                "Mina lent a book to Jin.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "DIALOGUE_USER",
                "Nora read a book in the library.",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let initial = DiscourseQaEngine
            .answer("Who lent a book?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        let p = initial.content_projection.unwrap();
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: None,
            query: initial.query,
            answered_turn: 2,
            clarification_act: None,
            decision_inquiry: None,
            described_event: Some(DescribedEventReferenceIR {
                belief_id: p.belief_id.clone(),
                event_id: p.binding.event_id.unwrap(),
            }),
        });
        let a = DiscourseQaEngine
            .answer("Who read it?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(a.validate());
        let mut projected = a.content_projection.unwrap();
        assert_eq!(projected.binding.value, "Nora");
        assert_eq!(projected.reference_bindings[0].value, "book");
        assert_eq!(
            projected.reference_context.as_ref().unwrap().belief_id,
            p.belief_id
        );
        projected.reference_bindings[0].value = "pen".into();
        assert!(!projected.matches_question("Who read it?"));
        state.epistemic_ledger.records[0].status = BeliefRecordStatusIR::Retracted;
        let a = DiscourseQaEngine
            .answer("Who read it?", Some(&state), LanguageCodeIR::English)
            .unwrap();
        assert!(a.content_projection.is_none());
    }

    #[test]
    fn event_person_reference_needs_unique_participant_or_focused_role() {
        let mut state = state_with(&[(
            "DIALOGUE_USER",
            "민수는 지연에게 책을 빌려줬어.",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let q = "그 사람은 누구에게 책을 빌려줬어?";
        let a = DiscourseQaEngine
            .answer(q, Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        assert!(
            a.content_projection.is_none(),
            "two people without a focused role"
        );
        let initial = DiscourseQaEngine
            .answer("누가 책을 빌려줬어?", Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        let p = initial.content_projection.unwrap();
        state.answer_focus = Some(AnswerFocusIR {
            shared_proposition: None,
            proposition_belief_id: None,
            query: initial.query,
            answered_turn: 1,
            clarification_act: None,
            decision_inquiry: None,
            described_event: Some(DescribedEventReferenceIR {
                belief_id: p.belief_id,
                event_id: p.binding.event_id.unwrap(),
            }),
        });
        let a = DiscourseQaEngine
            .answer(q, Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        assert!(a.validate());
        assert_eq!(a.content_projection.unwrap().binding.value, "지연");
        state.completed_turns = 5;
        let a = DiscourseQaEngine
            .answer(q, Some(&state), LanguageCodeIR::Korean)
            .unwrap();
        assert!(a.content_projection.is_none(), "expired focus");
    }

    #[test]
    fn factive_source_status_does_not_promote_complement_to_truth() {
        let state = state_with(&[(
            "Alice",
            "the server is down",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Know,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "What does Alice know?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("recognized factive query");
        assert_eq!(
            answer.evidence[0].epistemic_status,
            EpistemicStatusIR::PresentedAsKnown
        );
        assert!(answer.realized_text.contains("not established facts"));
        assert!(!answer.evidence[0].dialogue_truth_established);
    }

    #[test]
    fn actuality_question_abstains_even_when_a_report_matches() {
        let state = state_with(&[(
            "Alice",
            "the server is down",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "Is the server actually true that it is down?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("actuality query");
        assert_eq!(
            answer.disposition,
            DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
        );
        assert_eq!(answer.evidence.len(), 1);
    }

    #[test]
    fn natural_actuality_word_order_is_recognized_without_claiming_truth() {
        let state = state_with(&[
            (
                "Mina",
                "the worker is blocked",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "Jules",
                "the worker is healthy",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        let answer = DiscourseQaEngine
            .answer(
                "is the worker actually blocked?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("natural actuality query");
        assert!(!answer.dialogue_truth_established);
        assert!(!answer.external_execution_authorized);
        assert_eq!(answer.unsupported_claims, 0);
        assert_eq!(answer.evidence.len(), 2);
    }

    #[test]
    fn generic_certainty_and_outcome_alternatives_query_all_current_reports() {
        let state = state_with(&[
            (
                "Mina",
                "the cache succeeded",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
            (
                "Jisoo",
                "the cache failed",
                ModalWorldIR::Actual,
                AttributionAttitudeIR::Say,
            ),
        ]);
        for text in ["What is certain right now?", "So did it succeed or fail?"] {
            let answer = DiscourseQaEngine
                .answer(text, Some(&state), LanguageCodeIR::English)
                .expect("certainty query");
            assert_eq!(
                answer.disposition,
                DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
            );
            assert_eq!(answer.evidence.len(), 2);
            assert!(!answer.dialogue_truth_established);
        }
    }

    #[test]
    fn possible_record_is_classified_without_becoming_actual() {
        let state = state_with(&[(
            "Alice",
            "the server might be down",
            ModalWorldIR::EpistemicPossible,
            AttributionAttitudeIR::Believe,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "Is the server merely possible or actual?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("modal query");
        assert_eq!(
            answer.evidence[0].modal_world,
            ModalWorldIR::EpistemicPossible
        );
        assert!(answer
            .claims
            .iter()
            .any(|claim| claim.kind == AnswerClaimKindIR::DialogueTruthNotEstablished));
    }

    #[test]
    fn presuppositional_why_question_does_not_accept_its_premise() {
        let state = state_with(&[(
            "Alice",
            "the server might have failed",
            ModalWorldIR::EpistemicPossible,
            AttributionAttitudeIR::Believe,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "Why did the server fail?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("presuppositional query");
        assert_eq!(
            answer.disposition,
            DiscourseAnswerDispositionIR::PresuppositionUnverified
        );
        assert!(!answer.query.presuppositions[0].dialogue_truth_established);
    }

    #[test]
    fn hypothetical_advice_question_does_not_reject_assumed_context() {
        let engine = DiscourseQaEngine;
        for (language, text) in [
            (LanguageCodeIR::Korean, "너라면 어떻게 하겠어?"),
            (
                LanguageCodeIR::English,
                "If you were me, how would you handle it?",
            ),
            (
                LanguageCodeIR::English,
                "If the server failed, how would you recover?",
            ),
            (
                LanguageCodeIR::English,
                "If the server failed, how did Mira inspect the file?",
            ),
        ] {
            let query = engine
                .parse(text, None)
                .expect("question remains understood");
            assert_ne!(
                query.kind,
                DiscourseQueryKindIR::PresuppositionCheck,
                "{text}"
            );
            assert!(query.presuppositions.is_empty(), "{text}");
            let answer = engine.answer(text, None, language).expect("advice gap");
            assert_ne!(
                answer.disposition,
                DiscourseAnswerDispositionIR::PresuppositionUnverified,
                "{text}"
            );
            assert!(answer.query.presuppositions.is_empty(), "{text}");
        }
    }

    #[test]
    fn factual_past_question_still_requires_verified_premise() {
        for text in [
            "How did Mira inspect the file?",
            "Why did Mira inspect the file?",
            "어떻게 미라가 파일을 조사했어?",
        ] {
            let query = DiscourseQaEngine
                .parse(text, None)
                .expect("factual question");
            assert_eq!(
                query.kind,
                DiscourseQueryKindIR::PresuppositionCheck,
                "{text}"
            );
            assert!(!query.presuppositions.is_empty(), "{text}");
        }
    }

    #[test]
    fn hypothetical_boundary_covers_negative_and_elliptical_questions() {
        for text in [
            "너라면 어떻게 안 하겠어?",
            "If it happened, how would you respond?",
        ] {
            assert!(
                hypothetical_or_conditional_question(&text.to_lowercase()),
                "{text}"
            );
            assert!(!presuppositional_question(&text.to_lowercase()), "{text}");
        }
        assert!(!hypothetical_or_conditional_question(
            "how did mira inspect the file?"
        ));
        assert!(non_actual_world_question(
            "If the server failed, how did Mira inspect the file?"
        ));
    }

    #[test]
    fn korean_hypothetical_outcome_questions_stay_with_modal_discourse_owner() {
        for text in [
            "만약 세아가 문서를 읽었다면 어떨까?",
            "만약 유나가 잡지를 읽었다면 어떨까요?",
        ] {
            assert!(hypothetical_or_conditional_question(text), "{text}");
            assert!(non_actual_world_question(text), "{text}");
            assert!(!presuppositional_question(text), "{text}");
        }
        for text in [
            "만약 alpha가 active라면 beta는 safe인가?",
            "\"만약 세아가 문서를 읽었다면 어떨까?\"라고 물었어.",
            "세아가 문서를 읽었어?",
        ] {
            assert!(!non_actual_world_question(text), "{text}");
        }
    }

    #[test]
    fn unknown_source_is_not_fabricated() {
        let state = state_with(&[(
            "Alice",
            "the server is down",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "What did Charlie say?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("recognized unknown source question");
        assert_eq!(
            answer.disposition,
            DiscourseAnswerDispositionIR::NoMatchingRecord
        );
        assert!(answer.evidence.is_empty());
    }

    #[test]
    fn ascii_source_prefix_does_not_capture_a_longer_unknown_name() {
        let state = state_with(&[(
            "Ann",
            "the server is down",
            ModalWorldIR::Actual,
            AttributionAttitudeIR::Say,
        )]);
        let answer = DiscourseQaEngine
            .answer(
                "What did Annabelle say?",
                Some(&state),
                LanguageCodeIR::English,
            )
            .expect("recognized source question");
        assert_eq!(
            answer.disposition,
            DiscourseAnswerDispositionIR::NoMatchingRecord
        );
        assert_eq!(answer.query.requested_source.as_deref(), Some("annabelle"));
        assert!(answer.evidence.is_empty());
    }

    #[test]
    fn embedded_how_directive_is_not_misclassified_as_a_question() {
        assert!(!looks_like_question("learn how the delta api works"));
        assert!(!looks_like_question("explain how the cache works"));
        assert!(looks_like_question("how did the cache fail?"));
    }
}
