//! A compact, inspectable native language circuit.
//!
//! The circuit stores lexical and construction knowledge, never completed
//! replies.  It turns one utterance into entity, event, scope, reference, and
//! response-goal IR before any planner or surface realizer is consulted.

use std::collections::BTreeSet;

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compositional_semantics::{
    CandidateDispositionIR, CompositionalAnalysisIR, InterpretationCandidateIR,
};
use crate::language_knowledge::LanguageCodeIR;

pub const NATIVE_LANGUAGE_CIRCUIT_SCHEMA: &str = "B_CORE_NATIVE_LANGUAGE_CIRCUIT_IR_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeEventScopeIR {
    Live,
    Conditional,
    Prohibited,
    Reported,
    Possible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeDiscourseRelationIR {
    Sequence,
    Contrast,
    Condition,
    Concession,
    Correction,
    Cause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeReferenceKindIR {
    IntraTurnAnaphora,
    ExplicitPriorTheme,
    ContrastiveRetarget,
    ClarificationAnswer,
    CausalTarget,
    SetMember,
    PluralContextSet,
    OperationEllipsis,
    EventOrdinal,
    VerifiedResultTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeResponseGoalIR {
    PlanActions,
    AnswerVerifiedResult,
    AskClarification,
    Acknowledge,
}

/// A semantic response-plan refinement.  This is deliberately separate from
/// surface realization: it records *why* an answer is needed so that later
/// modules cannot reinterpret a report as a query, or a result query as a
/// generic discourse answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NativeResponseModeIR {
    Plan,
    Clarification,
    Acknowledgement,
    ReportedOutcome,
    CompetingOutcomeReports,
    VerificationStatusQuery,
    EvidenceResultQuery,
    SourceCertaintyQuery,
    OutcomeAlternativeQuery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeEntityIR {
    pub entity_id: String,
    pub surface: String,
    pub canonical_concept: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub rejected_by_contrast: bool,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeEventIR {
    pub event_id: String,
    pub canonical_predicate: String,
    pub predicate_surface: String,
    pub intent: PlanIntentIR,
    pub scope: NativeEventScopeIR,
    pub theme_entity_ids: Vec<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeReferenceBindingIR {
    pub binding_id: String,
    pub kind: NativeReferenceKindIR,
    pub source_surface: String,
    pub target_entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_goal_id: Option<String>,
    pub confidence_millis: u16,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeGoalIR {
    pub goal_id: String,
    pub source_event_id: String,
    pub canonical_predicate: String,
    pub intent: PlanIntentIR,
    pub subject: String,
    pub subject_concepts: Vec<String>,
    pub confidence_millis: u16,
    pub selection_reasons: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRelationEdgeIR {
    pub relation_id: String,
    pub kind: NativeDiscourseRelationIR,
    pub source_id: String,
    pub target_id: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeTurnIR {
    pub schema: String,
    pub language: LanguageCodeIR,
    pub source_sha256: String,
    pub entities: Vec<NativeEntityIR>,
    pub events: Vec<NativeEventIR>,
    pub relations: Vec<NativeRelationEdgeIR>,
    pub reference_bindings: Vec<NativeReferenceBindingIR>,
    pub selected_live_goals: Vec<NativeGoalIR>,
    pub response_goal: NativeResponseGoalIR,
    pub response_mode: NativeResponseModeIR,
    pub unresolved: Vec<String>,
    pub selected_semantic_sha256: String,
    pub circuit_sha256: String,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeContextGoalIR {
    pub goal_id: String,
    pub intent: PlanIntentIR,
    pub canonical_predicate: String,
    pub subject: String,
    pub introduced_turn: u64,
    pub discourse_focused: bool,
    /// Whether an ellipsed follow-up may inherit this operation. Historical
    /// result records remain queryable but cannot silently become new work.
    pub operation_replayable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeContextEntityIR {
    pub referent_id: String,
    pub surface: String,
    pub introduced_turn: u64,
    pub last_mentioned_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeContextReferentIR {
    pub referent_id: String,
    pub semantic_summary: String,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeDialogueContextIR {
    pub active_goals: Vec<NativeContextGoalIR>,
    pub active_entities: Vec<NativeContextEntityIR>,
    pub active_referents: Vec<NativeContextReferentIR>,
}

impl NativeTurnIR {
    pub(crate) fn add_boundary_ambiguity(&mut self, reason: &str) {
        if !reason.trim().is_empty() && !self.unresolved.iter().any(|item| item == reason) {
            self.unresolved.push(reason.to_string());
            self.unresolved.sort();
            self.response_goal = NativeResponseGoalIR::AskClarification;
            self.response_mode = NativeResponseModeIR::Clarification;
            self.circuit_sha256 = native_turn_sha256(self);
        }
    }

    /// Rebase an ellipsed clarification answer onto the operation stored by
    /// the pending question. The answer surface selects the entity, while the
    /// QUD restores the operation. Clarification provenance remains in the
    /// conversation binding, so a stale operation-ellipsis binding must not
    /// compete with it.
    pub(crate) fn apply_resolved_clarification(
        &mut self,
        source: &str,
        canonical_predicate: &str,
        intent: PlanIntentIR,
    ) {
        if canonical_predicate.trim().is_empty() || source.trim().is_empty() {
            return;
        }
        if self.selected_live_goals.is_empty() {
            let mut candidates = self
                .entities
                .iter()
                .filter(|entity| !entity.rejected_by_contrast);
            let Some(entity) = candidates.next().cloned() else {
                return;
            };
            if candidates.next().is_some() {
                return;
            }
            let event_id = "NE-CLARIFICATION-ANSWER".to_string();
            self.events.push(NativeEventIR {
                event_id: event_id.clone(),
                canonical_predicate: canonical_predicate.to_string(),
                predicate_surface: source.to_string(),
                intent,
                scope: NativeEventScopeIR::Live,
                theme_entity_ids: vec![entity.entity_id.clone()],
                start_byte: 0,
                end_byte: source.len(),
                confidence_millis: 980,
                evidence: vec![
                    "PENDING_QUD_RESTORES_OPERATION".to_string(),
                    "CLARIFICATION_SURFACE_SELECTS_ARGUMENT_ONLY".to_string(),
                    "INHERITED_PREDICATE_HAS_NO_NEW_EXECUTION_AUTHORITY".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
            self.selected_live_goals.push(NativeGoalIR {
                goal_id: "NG-CLARIFICATION-ANSWER".to_string(),
                source_event_id: event_id,
                canonical_predicate: canonical_predicate.to_string(),
                intent,
                subject: entity.surface,
                subject_concepts: vec![entity.canonical_concept],
                confidence_millis: 980,
                selection_reasons: vec![
                    "RESOLVED_CLARIFICATION_OWNS_OPERATION".to_string(),
                    "ANSWER_SELECTS_UNIQUE_TYPED_THEME".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
            self.reference_bindings.clear();
            self.reference_bindings.push(NativeReferenceBindingIR {
                binding_id: "NB-CLARIFICATION-ANSWER".to_string(),
                kind: NativeReferenceKindIR::ClarificationAnswer,
                source_surface: "PENDING_QUD_ANSWER".to_string(),
                target_entity_id: entity.entity_id,
                inherited_goal_id: None,
                confidence_millis: 980,
                evidence: vec![
                    "CLARIFICATION_ANSWER_SELECTS_TYPED_THEME".to_string(),
                    "PENDING_QUD_RESTORES_OPERATION".to_string(),
                ],
            });
            self.unresolved.clear();
            self.response_goal = NativeResponseGoalIR::PlanActions;
            self.response_mode = NativeResponseModeIR::Plan;
            self.selected_semantic_sha256 = selected_semantic_sha256(self);
            self.circuit_sha256 = native_turn_sha256(self);
            debug_assert!(self.validate_for_source(source));
            return;
        }
        if self.selected_live_goals.len() != 1 {
            return;
        }
        let source_event_id = self.selected_live_goals[0].source_event_id.clone();
        let Some(event) = self.events.iter_mut().find(|event| {
            event.event_id == source_event_id && event.scope == NativeEventScopeIR::Live
        }) else {
            return;
        };
        event.canonical_predicate = canonical_predicate.to_string();
        event.intent = intent;
        event
            .evidence
            .push("PENDING_QUD_RESTORES_OPERATION".to_string());
        event.evidence.sort();
        event.evidence.dedup();
        let clarification_target = event.theme_entity_ids.first().cloned();
        let goal = &mut self.selected_live_goals[0];
        goal.canonical_predicate = canonical_predicate.to_string();
        goal.intent = intent;
        goal.selection_reasons
            .push("RESOLVED_CLARIFICATION_OWNS_OPERATION".to_string());
        goal.selection_reasons.sort();
        goal.selection_reasons.dedup();
        self.reference_bindings.clear();
        if let Some(target_entity_id) = clarification_target {
            self.reference_bindings.push(NativeReferenceBindingIR {
                binding_id: "NB-CLARIFICATION-ANSWER".to_string(),
                kind: NativeReferenceKindIR::ClarificationAnswer,
                source_surface: "PENDING_QUD_ANSWER".to_string(),
                target_entity_id,
                inherited_goal_id: None,
                confidence_millis: 980,
                evidence: vec![
                    "CLARIFICATION_ANSWER_SELECTS_TYPED_THEME".to_string(),
                    "PENDING_QUD_RESTORES_OPERATION".to_string(),
                ],
            });
        }
        self.unresolved.clear();
        self.response_goal = NativeResponseGoalIR::PlanActions;
        self.response_mode = NativeResponseModeIR::Plan;
        self.selected_semantic_sha256 = selected_semantic_sha256(self);
        self.circuit_sha256 = native_turn_sha256(self);
        debug_assert!(self.validate_for_source(source));
    }

    /// Materialize a request selected by the compositional grammar into the
    /// native event/goal IR when the native lexical path found no competing
    /// goal. This is a one-way, fill-only arbitration step: an existing native
    /// goal or unresolved ambiguity is never overwritten.
    pub(crate) fn absorb_selected_compositional_goals(
        &mut self,
        source: &str,
        analysis: &CompositionalAnalysisIR,
    ) -> bool {
        let selected = analysis.selected_candidates();
        let viable_count = analysis
            .candidates
            .iter()
            .filter(|candidate| candidate.disposition == CandidateDispositionIR::Viable)
            .count();
        if selected.is_empty()
            || viable_count != selected.len()
            || !analysis.modal_scope_graph.conditionals.is_empty()
            || analysis
                .goal_graph
                .as_ref()
                .is_some_and(|graph| !graph.conditions.is_empty() || !graph.prohibitions.is_empty())
            || selected.iter().any(|candidate| {
                candidate.disposition != CandidateDispositionIR::Viable
                    || !candidate.external_execution_authorized
            })
        {
            return false;
        }

        // The lexical circuit may know the requested operation while lacking
        // an entity class for its free-form theme (for example, "error
        // cause").  A unique compositional frame may fill that one missing
        // argument, but it may not replace an operation, resolve competing
        // candidates, or erase any other ambiguity.
        if self.reconcile_single_unbound_theme(source, analysis, &selected) {
            return true;
        }

        if self.response_goal != NativeResponseGoalIR::Acknowledge
            || self.response_mode != NativeResponseModeIR::Acknowledgement
            || !self.selected_live_goals.is_empty()
            || !self.unresolved.is_empty()
        {
            return false;
        }

        let source_lower = source.to_lowercase();
        let mut staged_entities = self.entities.clone();
        let mut staged_events = Vec::new();
        let mut staged_goals = Vec::new();
        for candidate in selected {
            let Some(frame) = analysis
                .frames
                .iter()
                .find(|frame| frame.frame_id == candidate.source_frame_id)
            else {
                return false;
            };
            let subject = candidate.subject.trim();
            if subject.is_empty() {
                return false;
            }
            let subject_lower = subject.to_lowercase();
            let mut theme_entity_ids = staged_entities
                .iter()
                .filter(|entity| {
                    if entity.rejected_by_contrast {
                        return false;
                    }
                    let entity_lower = entity.surface.to_lowercase();
                    subject_lower.contains(&entity_lower)
                        || entity_lower.contains(subject_lower.as_str())
                })
                .map(|entity| entity.entity_id.clone())
                .collect::<Vec<_>>();
            if theme_entity_ids.is_empty() {
                let Some(start_byte) = source_lower.find(&subject_lower) else {
                    return false;
                };
                let end_byte = start_byte + subject.len();
                if end_byte > source.len() {
                    return false;
                }
                let entity_id = format!("NX{:03}", staged_entities.len() + 1);
                staged_entities.push(NativeEntityIR {
                    entity_id: entity_id.clone(),
                    surface: source[start_byte..end_byte].to_string(),
                    canonical_concept: context_subject_concept(subject),
                    start_byte,
                    end_byte,
                    rejected_by_contrast: false,
                    confidence_millis: candidate.score_millis,
                    evidence: vec![
                        "COMPOSITIONAL_GOAL_THEME".to_string(),
                        format!("COMPOSITIONAL_CANDIDATE:{}", candidate.candidate_id),
                    ],
                });
                theme_entity_ids.push(entity_id);
            }
            theme_entity_ids.sort();
            theme_entity_ids.dedup();

            let predicate_lower = frame.predicate_surface.to_lowercase();
            let Some(start_byte) = source_lower.find(&predicate_lower) else {
                return false;
            };
            let end_byte = start_byte + frame.predicate_surface.len();
            if end_byte > source.len() {
                return false;
            }
            let event_id = format!("NE-COMP-{:03}", staged_events.len() + 1);
            staged_events.push(NativeEventIR {
                event_id: event_id.clone(),
                canonical_predicate: frame.canonical_predicate.clone(),
                predicate_surface: source[start_byte..end_byte].to_string(),
                intent: candidate.intent,
                scope: NativeEventScopeIR::Live,
                theme_entity_ids: theme_entity_ids.clone(),
                start_byte,
                end_byte,
                confidence_millis: candidate.score_millis,
                evidence: vec![
                    "CENTRAL_COMPOSITIONAL_GOAL_MATERIALIZATION".to_string(),
                    format!("COMPOSITIONAL_CANDIDATE:{}", candidate.candidate_id),
                    "LANGUAGE_HAS_NO_EXECUTION_AUTHORITY".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
            let selected_entities = theme_entity_ids
                .iter()
                .filter_map(|entity_id| {
                    staged_entities
                        .iter()
                        .find(|entity| &entity.entity_id == entity_id)
                })
                .collect::<Vec<_>>();
            staged_goals.push(NativeGoalIR {
                goal_id: format!("NG-COMP-{:03}", staged_goals.len() + 1),
                source_event_id: event_id,
                canonical_predicate: frame.canonical_predicate.clone(),
                intent: candidate.intent,
                subject: selected_entities
                    .iter()
                    .map(|entity| entity.surface.as_str())
                    .collect::<Vec<_>>()
                    .join(if self.language == LanguageCodeIR::Korean {
                        "와 "
                    } else {
                        " and "
                    }),
                subject_concepts: selected_entities
                    .iter()
                    .map(|entity| entity.canonical_concept.clone())
                    .collect(),
                confidence_millis: candidate.score_millis,
                selection_reasons: vec![
                    "COMPOSITIONAL_GRAMMAR_SELECTED_REQUEST".to_string(),
                    "NATIVE_GOAL_SLOT_WAS_EMPTY".to_string(),
                    "CENTRAL_FILL_ONLY_ARBITRATION".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
        }

        self.entities = staged_entities;
        self.events.extend(staged_events);
        self.selected_live_goals = staged_goals;
        self.response_goal = NativeResponseGoalIR::PlanActions;
        self.response_mode = NativeResponseModeIR::Plan;
        self.selected_semantic_sha256 = selected_semantic_sha256(self);
        self.circuit_sha256 = native_turn_sha256(self);
        debug_assert!(self.validate_for_source(source));
        true
    }

    fn reconcile_single_unbound_theme(
        &mut self,
        source: &str,
        analysis: &CompositionalAnalysisIR,
        selected: &[&InterpretationCandidateIR],
    ) -> bool {
        if self.response_goal != NativeResponseGoalIR::AskClarification
            || !self.selected_live_goals.is_empty()
            || selected.len() != 1
            || self.unresolved.len() != 1
        {
            return false;
        }
        let candidate = selected[0];
        let Some(frame) = analysis
            .frames
            .iter()
            .find(|frame| frame.frame_id == candidate.source_frame_id)
        else {
            return false;
        };
        let mut matching_events = self.events.iter().enumerate().filter(|(_, event)| {
            event.scope == NativeEventScopeIR::Live
                && event.theme_entity_ids.is_empty()
                && event.canonical_predicate == frame.canonical_predicate
                && event.intent == candidate.intent
        });
        let Some((event_index, event)) = matching_events.next() else {
            return false;
        };
        if matching_events.next().is_some()
            || self.unresolved[0] != format!("UNBOUND_THEME:{}", event.event_id)
        {
            return false;
        }
        let subject = candidate.subject.trim();
        if subject.is_empty() {
            return false;
        }
        let source_lower = source.to_lowercase();
        let subject_lower = subject.to_lowercase();
        let Some(start_byte) = source_lower.find(&subject_lower) else {
            return false;
        };
        let end_byte = start_byte + subject.len();
        if end_byte > source.len() {
            return false;
        }

        let entity_id = format!("NX{:03}", self.entities.len() + 1);
        let entity = NativeEntityIR {
            entity_id: entity_id.clone(),
            surface: source[start_byte..end_byte].to_string(),
            canonical_concept: context_subject_concept(subject),
            start_byte,
            end_byte,
            rejected_by_contrast: false,
            confidence_millis: candidate.score_millis,
            evidence: vec![
                "COMPOSITIONAL_UNBOUND_THEME_FILL".to_string(),
                format!("COMPOSITIONAL_CANDIDATE:{}", candidate.candidate_id),
            ],
        };
        let event = &mut self.events[event_index];
        event.theme_entity_ids.push(entity_id);
        event.confidence_millis = event.confidence_millis.max(candidate.score_millis);
        event
            .evidence
            .push("COMPOSITIONAL_GRAMMAR_BINDS_MISSING_THEME".to_string());
        event.evidence.sort();
        event.evidence.dedup();
        let event_id = event.event_id.clone();
        self.entities.push(entity.clone());
        self.selected_live_goals.push(NativeGoalIR {
            goal_id: "NG-COMP-BIND-001".to_string(),
            source_event_id: event_id,
            canonical_predicate: frame.canonical_predicate.clone(),
            intent: candidate.intent,
            subject: entity.surface,
            subject_concepts: vec![entity.canonical_concept],
            confidence_millis: candidate.score_millis,
            selection_reasons: vec![
                "LEXICAL_OPERATION_COMPOSITIONAL_THEME".to_string(),
                "UNIQUE_FILL_ONLY_ARGUMENT_BINDING".to_string(),
            ],
            semantic_authority: false,
            external_execution_authorized: false,
        });
        self.unresolved.clear();
        self.response_goal = NativeResponseGoalIR::PlanActions;
        self.response_mode = NativeResponseModeIR::Plan;
        self.selected_semantic_sha256 = selected_semantic_sha256(self);
        self.circuit_sha256 = native_turn_sha256(self);
        debug_assert!(self.validate_for_source(source));
        true
    }

    /// Refine only the response boundary after a downstream typed analyzer has
    /// established that the turn is asking about, or reporting, action state.
    /// This method deliberately cannot create events, goals, facts, or
    /// execution evidence.  It is a fill-only repair for the circuit's broad
    /// acknowledgement default, so an earlier plan or clarification decision
    /// always wins.
    pub(crate) fn refine_response_boundary(
        &mut self,
        source: &str,
        mode: NativeResponseModeIR,
    ) -> bool {
        let answer_mode = matches!(
            mode,
            NativeResponseModeIR::ReportedOutcome
                | NativeResponseModeIR::CompetingOutcomeReports
                | NativeResponseModeIR::VerificationStatusQuery
                | NativeResponseModeIR::EvidenceResultQuery
                | NativeResponseModeIR::SourceCertaintyQuery
                | NativeResponseModeIR::OutcomeAlternativeQuery
        );
        if !answer_mode
            || self.response_goal != NativeResponseGoalIR::Acknowledge
            || self.response_mode != NativeResponseModeIR::Acknowledgement
            || !self.selected_live_goals.is_empty()
            || !self.unresolved.is_empty()
        {
            return false;
        }

        let selected_semantic_before = self.selected_semantic_sha256.clone();
        self.response_goal = NativeResponseGoalIR::AnswerVerifiedResult;
        self.response_mode = mode;
        self.circuit_sha256 = native_turn_sha256(self);
        debug_assert_eq!(self.selected_semantic_sha256, selected_semantic_before);
        debug_assert!(self.validate_for_source(source));
        true
    }

    /// Reclassify predicate-looking words inside a future evidence condition
    /// as conditional state, not an immediate action.  The notification policy
    /// itself is acknowledged by the dialogue layer; it never authorizes a
    /// current check or an external execution.
    pub(crate) fn apply_future_notification_boundary(&mut self, source: &str) -> bool {
        let lower = source.to_lowercase();
        let korean_condition_end = [
            "검증되면",
            "확인되면",
            "입증되면",
            "생기면",
            "나오면",
            "확실해지면",
            "확보되면",
        ]
        .iter()
        .filter_map(|marker| lower.find(marker).map(|start| start + marker.len()))
        .max();
        let korean_notification_start = ["알려", "통지"]
            .iter()
            .filter_map(|marker| lower.find(marker))
            .min();
        let english_notification_start = ["tell me when", "let me know when", "notify me when"]
            .iter()
            .filter_map(|marker| lower.find(marker))
            .min();

        let mut changed = false;
        for event in &mut self.events {
            let conditional_state = korean_condition_end.is_some_and(|end| event.start_byte < end)
                || korean_notification_start.is_some_and(|start| {
                    event.start_byte >= start && event.intent == PlanIntentIR::Explain
                })
                || english_notification_start.is_some_and(|start| {
                    event.start_byte >= start && event.intent == PlanIntentIR::Explain
                });
            if event.scope == NativeEventScopeIR::Live && conditional_state {
                event.scope = NativeEventScopeIR::Conditional;
                event
                    .evidence
                    .push("FUTURE_EVIDENCE_NOTIFICATION_CONDITION".to_string());
                event.evidence.sort();
                event.evidence.dedup();
                changed = true;
            }
        }
        if !changed {
            return false;
        }

        self.selected_live_goals.retain(|goal| {
            self.events.iter().any(|event| {
                event.event_id == goal.source_event_id && event.scope == NativeEventScopeIR::Live
            })
        });
        if self.selected_live_goals.is_empty() && self.unresolved.is_empty() {
            self.response_goal = NativeResponseGoalIR::Acknowledge;
            self.response_mode = NativeResponseModeIR::Acknowledgement;
        }
        self.selected_semantic_sha256 = selected_semantic_sha256(self);
        self.circuit_sha256 = native_turn_sha256(self);
        debug_assert!(self.validate_for_source(source));
        true
    }

    pub fn authoritative_live_goals(&self) -> Option<&[NativeGoalIR]> {
        (self.unresolved.is_empty()
            && !self.selected_live_goals.is_empty()
            && self
                .selected_live_goals
                .iter()
                .all(|goal| goal.confidence_millis >= 800))
        .then_some(self.selected_live_goals.as_slice())
    }

    pub fn authoritative_single_live_goal(&self) -> Option<&NativeGoalIR> {
        self.authoritative_live_goals()
            .filter(|goals| goals.len() == 1)
            .map(|goals| &goals[0])
    }

    pub fn validate_for_source(&self, source: &str) -> bool {
        if self.schema != NATIVE_LANGUAGE_CIRCUIT_SCHEMA
            || self.source_sha256 != sha256_text(source)
            || self.semantic_authority
            || self.language_can_execute
            || self.circuit_sha256 != native_turn_sha256(self)
            || self.selected_semantic_sha256 != selected_semantic_sha256(self)
        {
            return false;
        }
        let entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.entity_id.as_str())
            .collect::<BTreeSet<_>>();
        if entity_ids.len() != self.entities.len()
            || self.entities.iter().any(|entity| {
                entity.entity_id.is_empty()
                    || entity.surface.trim().is_empty()
                    || entity.canonical_concept.trim().is_empty()
                    || (!entity.evidence.iter().any(|item| {
                        item.starts_with("DIALOGUE_CONTEXT_GOAL:")
                            || item.starts_with("DIALOGUE_CONTEXT_ENTITY:")
                    }) && (entity.start_byte >= entity.end_byte
                        || entity.end_byte > source.len()))
                    || (entity.evidence.iter().any(|item| {
                        item.starts_with("DIALOGUE_CONTEXT_GOAL:")
                            || item.starts_with("DIALOGUE_CONTEXT_ENTITY:")
                    }) && (entity.start_byte != 0 || entity.end_byte != 0))
                    || entity.confidence_millis > 1_000
                    || entity.evidence.is_empty()
            })
        {
            return false;
        }
        let event_ids = self
            .events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        if event_ids.len() != self.events.len()
            || self.events.iter().any(|event| {
                event.event_id.is_empty()
                    || event.canonical_predicate.is_empty()
                    || event.predicate_surface.is_empty()
                    || event.start_byte >= event.end_byte
                    || event.end_byte > source.len()
                    || event.confidence_millis > 1_000
                    || event.evidence.is_empty()
                    || event.semantic_authority
                    || event.external_execution_authorized
                    || event
                        .theme_entity_ids
                        .iter()
                        .any(|entity_id| !entity_ids.contains(entity_id.as_str()))
            })
        {
            return false;
        }
        self.reference_bindings.iter().all(|binding| {
            !binding.binding_id.is_empty()
                && !binding.source_surface.is_empty()
                && entity_ids.contains(binding.target_entity_id.as_str())
                && binding
                    .inherited_goal_id
                    .as_ref()
                    .is_none_or(|goal_id| !goal_id.trim().is_empty())
                && binding.confidence_millis <= 1_000
                && !binding.evidence.is_empty()
        }) && self.selected_live_goals.iter().all(|goal| {
            event_ids.contains(goal.source_event_id.as_str())
                && self.events.iter().any(|event| {
                    event.event_id == goal.source_event_id
                        && event.scope == NativeEventScopeIR::Live
                        && event.canonical_predicate == goal.canonical_predicate
                        && event.intent == goal.intent
                })
                && !goal.subject.trim().is_empty()
                && !goal.subject_concepts.is_empty()
                && goal.confidence_millis <= 1_000
                && !goal.selection_reasons.is_empty()
                && !goal.semantic_authority
                && !goal.external_execution_authorized
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionLexeme {
    surface: &'static str,
    canonical_predicate: &'static str,
    intent: PlanIntentIR,
}

const ACTION_LEXEMES: &[ActionLexeme] = &[
    ActionLexeme {
        surface: "walk me through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "walk us through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "walk you through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "walk them through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "walk him through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "walk her through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk me through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk us through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk you through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk them through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk him through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "talk her through",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "take a look at",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "take a look",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "diagnostic pass",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "work out",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "find out",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "look into",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "investigate",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "diagnose",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "inspect",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "check",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "checking",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "chek",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "review",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "repair",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "fix",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "explain",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "explanation",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "describe",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "delete",
        canonical_predicate: "DELETE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "remove",
        canonical_predicate: "REMOVE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "modify",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "change",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "touch",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "execute",
        canonical_predicate: "EXECUTE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "run",
        canonical_predicate: "EXECUTE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "원인을 확인",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "조사",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "진단",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "분석",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "검사",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "확인",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "살펴",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "좁혀",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "좁히",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "원인을 찾",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "알아봐",
        canonical_predicate: "INVESTIGATE",
        intent: PlanIntentIR::Investigate,
    },
    ActionLexeme {
        surface: "수리",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "복구",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "되살",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "고쳐",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "고치",
        canonical_predicate: "REPAIR",
        intent: PlanIntentIR::Repair,
    },
    ActionLexeme {
        surface: "설명",
        canonical_predicate: "EXPLAIN",
        intent: PlanIntentIR::Explain,
    },
    ActionLexeme {
        surface: "삭제",
        canonical_predicate: "DELETE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "변경",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "바꾸",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "수정",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "건드리",
        canonical_predicate: "MODIFY",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "실행",
        canonical_predicate: "EXECUTE",
        intent: PlanIntentIR::Execute,
    },
    ActionLexeme {
        surface: "수행",
        canonical_predicate: "EXECUTE",
        intent: PlanIntentIR::Execute,
    },
];

const CONNECTORS: &[&str] = &[
    "하고 지금",
    "하고 ",
    " and now ",
    " 하되 ",
    "하되 ",
    " 말고 ",
    "말고 ",
    ", but ",
    " but ",
    ", then ",
    " then ",
    "—",
    ";",
    ".",
];

const CONDITIONAL_MARKERS: &[&str] = &[
    "only if",
    "only when",
    "unless",
    "if ",
    "provided that",
    "even if",
    "although",
    "경우에만",
    "때에만",
    "때만",
    "더라도",
    "아니면",
    "있으면",
    "됐으면",
    "되면",
    "라면",
    "다면",
];

const PROHIBITION_MARKERS: &[&str] = &[
    "do not ",
    "don't ",
    "must not ",
    "never ",
    "하지",
    "지 마",
    "지마",
    "지는 마",
    "지는 말",
    "말아",
    "금지",
];

pub(crate) fn contains_explicit_prohibition(source: &str) -> bool {
    let lower = source.to_lowercase();
    PROHIBITION_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

const REPORTED_SCOPE_MARKERS: &[&str] = &[
    " said ",
    " says ",
    " told ",
    " reported ",
    "according to ",
    "라고 말",
    "다고 말",
    "라고 보고",
    "다고 보고",
];

const POSSIBLE_SCOPE_MARKERS: &[&str] = &[
    "might ",
    "may need",
    "could need",
    "possibly ",
    "perhaps ",
    "수도 있",
    "가능성이",
];

#[derive(Debug, Clone)]
struct ActionMatch {
    start: usize,
    end: usize,
    knowledge_surface: String,
    canonical_predicate: String,
    intent: PlanIntentIR,
    inherited_goal_id: Option<String>,
}

#[derive(Debug, Clone)]
struct TokenSpan {
    start: usize,
    end: usize,
    surface: String,
}

#[derive(Debug, Clone)]
struct ContextEntityCandidate {
    referent_id: String,
    surface: String,
    canonical_concept: String,
    introduced_turn: u64,
    last_mentioned_turn: u64,
    source_order: usize,
}

pub struct NativeLanguageCircuit;

impl NativeLanguageCircuit {
    pub fn analyze(&self, source: &str) -> NativeTurnIR {
        self.analyze_with_context(source, &NativeDialogueContextIR::default())
    }

    pub fn analyze_with_context(
        &self,
        source: &str,
        context: &NativeDialogueContextIR,
    ) -> NativeTurnIR {
        let language = if source
            .chars()
            .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
        {
            LanguageCodeIR::Korean
        } else {
            LanguageCodeIR::English
        };
        let lower = source.to_lowercase();
        let mut entities = extract_entities(source, &lower);
        let context_entities = recent_context_entities(context);
        let ordinal_reference = ordinal_goal_index(&lower).is_some();
        let set_member_reference = set_member_selection(&lower).is_some();
        let plural_context_reference = contains_plural_context_reference_marker(&lower);
        let unique_context_goal = uniquely_recent_replayable_context_goal(&context.active_goals);
        let task_continuation_reference = !set_member_reference
            && !ordinal_reference
            && !plural_context_reference
            && (operation_ellipsis_marker(&lower).is_some()
                || continuation_constraint_marker(&lower).is_some());
        let discourse_level_answer_query =
            asks_what_is_certain(&lower) || asks_outcome_alternative(&lower);
        let context_entity_ambiguity = entities.is_empty()
            && contains_context_reference_marker(&lower)
            && !ordinal_reference
            && !set_member_reference
            && !discourse_level_answer_query
            && !plural_context_reference
            && !(task_continuation_reference && unique_context_goal.is_some())
            && context_entities.len() > 1;
        let contextual_discourse_entity = (entities.is_empty()
            && contains_context_reference_marker(&lower)
            && !ordinal_reference
            && !discourse_level_answer_query
            && context_entities.len() == 1)
            .then(|| context_entities.first())
            .flatten();
        let contextual_deictic_goal = if entities.is_empty()
            && contextual_discourse_entity.is_none()
            && contains_context_reference_marker(&lower)
            && (context_entities.is_empty() || task_continuation_reference)
        {
            unique_context_goal
        } else {
            None
        };
        if let Some(entity) = contextual_discourse_entity {
            entities.push(NativeEntityIR {
                entity_id: "NX001".to_string(),
                surface: entity.surface.clone(),
                canonical_concept: entity.canonical_concept.clone(),
                start_byte: 0,
                end_byte: 0,
                rejected_by_contrast: false,
                confidence_millis: 950,
                evidence: vec![format!("DIALOGUE_CONTEXT_ENTITY:{}", entity.referent_id)],
            });
        }
        if let Some(goal) = contextual_deictic_goal {
            entities.push(NativeEntityIR {
                entity_id: "NX001".to_string(),
                surface: goal.subject.clone(),
                canonical_concept: context_subject_concept(&goal.subject),
                start_byte: 0,
                end_byte: 0,
                rejected_by_contrast: false,
                confidence_millis: 960,
                evidence: vec![format!("DIALOGUE_CONTEXT_GOAL:{}", goal.goal_id)],
            });
        }
        let context_set_entity_ids = if entities.is_empty()
            && (plural_context_reference || set_member_reference)
            && context_entities.len() > 1
        {
            context_entities
                .iter()
                .map(|entity| {
                    let entity_id = format!("NX{:03}", entities.len() + 1);
                    entities.push(NativeEntityIR {
                        entity_id: entity_id.clone(),
                        surface: entity.surface.clone(),
                        canonical_concept: entity.canonical_concept.clone(),
                        start_byte: 0,
                        end_byte: 0,
                        rejected_by_contrast: false,
                        confidence_millis: 960,
                        evidence: vec![
                            format!("DIALOGUE_CONTEXT_ENTITY:{}", entity.referent_id),
                            "CONTEXT_SET_MEMBER".to_string(),
                        ],
                    });
                    entity_id
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut actions = action_matches(&lower);
        suppress_explanation_complement_actions(&mut actions, &lower);
        suppress_nominal_action_modifiers(&mut actions, &lower);
        if continuation_constraint_marker(&lower).is_some() {
            suppress_korean_nominal_argument_actions(&mut actions, &lower);
        }
        collapse_controlled_action_complements(&mut actions, &lower);
        if let Some((start, end, marker)) = retarget_carrier_marker(&lower) {
            let prior =
                uniquely_recent_replayable_context_goal(&context.active_goals).or_else(|| {
                    set_member_selection(&lower)
                        .is_some()
                        .then(|| shared_replayable_context_operation(&context.active_goals))
                        .flatten()
                });
            if let Some(prior) = prior {
                actions.clear();
                actions.push(ActionMatch {
                    start,
                    end,
                    knowledge_surface: format!("RETARGET_OPERATION:{marker}"),
                    canonical_predicate: prior.canonical_predicate.clone(),
                    intent: prior.intent,
                    inherited_goal_id: Some(prior.goal_id.clone()),
                });
            }
        } else if let Some((start, end, marker)) = operation_ellipsis_marker(&lower) {
            let prior =
                uniquely_recent_replayable_context_goal(&context.active_goals).or_else(|| {
                    set_member_selection(&lower)
                        .is_some()
                        .then(|| shared_replayable_context_operation(&context.active_goals))
                        .flatten()
                });
            if let Some(prior) = prior {
                if actions.is_empty() {
                    actions.push(ActionMatch {
                        start,
                        end,
                        knowledge_surface: format!("OPERATION_ELLIPSIS:{marker}"),
                        canonical_predicate: prior.canonical_predicate.clone(),
                        intent: prior.intent,
                        inherited_goal_id: Some(prior.goal_id.clone()),
                    });
                } else if let Some(action) = actions
                    .iter_mut()
                    .find(|action| action.start < end && action.end > start)
                {
                    action.knowledge_surface = format!("OPERATION_ELLIPSIS:{marker}");
                    action.canonical_predicate = prior.canonical_predicate.clone();
                    action.intent = prior.intent;
                    action.inherited_goal_id = Some(prior.goal_id.clone());
                }
            }
        }
        if let Some((start, end, marker)) = continuation_constraint_marker(&lower) {
            if let Some(prior) = uniquely_recent_replayable_context_goal(&context.active_goals) {
                if !actions.iter().any(|action| {
                    action.inherited_goal_id.as_deref() == Some(prior.goal_id.as_str())
                        && action.intent == prior.intent
                }) {
                    actions.push(ActionMatch {
                        start,
                        end,
                        knowledge_surface: format!("CONSTRAINT_CONTINUATION:{marker}"),
                        canonical_predicate: prior.canonical_predicate.clone(),
                        intent: prior.intent,
                        inherited_goal_id: Some(prior.goal_id.clone()),
                    });
                }
            }
        }
        actions = non_overlapping_action_matches(actions);
        let ordinal_context_entity = ordinal_goal_index(&lower)
            .and_then(|ordinal| context_entities.get(ordinal.saturating_sub(1)));
        let ordinal_context = ordinal_goal_index(&lower).and_then(|ordinal| {
            let mut goals = context.active_goals.iter().collect::<Vec<_>>();
            goals.sort_by(|left, right| {
                left.introduced_turn
                    .cmp(&right.introduced_turn)
                    .then_with(|| left.goal_id.cmp(&right.goal_id))
            });
            goals.get(ordinal.saturating_sub(1)).copied()
        });
        // An ordinal whose lexical head is "issue", "task", or "problem"
        // denotes the ordered goal history.  A bare "first/second one" still
        // denotes the locally active entity set.  Keeping those domains
        // separate prevents a topic-return request from collapsing to the
        // latest entity without breaking local set-member references.
        let ordinal_target = if ordinal_denotes_goal_history(&lower) {
            ordinal_context
                .map(|goal| {
                    (
                        goal.subject.clone(),
                        context_subject_concept(&goal.subject),
                        format!("DIALOGUE_CONTEXT_GOAL:{}", goal.goal_id),
                    )
                })
                .or_else(|| {
                    ordinal_context_entity.map(|entity| {
                        (
                            entity.surface.clone(),
                            entity.canonical_concept.clone(),
                            format!("DIALOGUE_CONTEXT_ENTITY:{}", entity.referent_id),
                        )
                    })
                })
        } else {
            ordinal_context_entity
                .map(|entity| {
                    (
                        entity.surface.clone(),
                        entity.canonical_concept.clone(),
                        format!("DIALOGUE_CONTEXT_ENTITY:{}", entity.referent_id),
                    )
                })
                .or_else(|| {
                    ordinal_context.map(|goal| {
                        (
                            goal.subject.clone(),
                            context_subject_concept(&goal.subject),
                            format!("DIALOGUE_CONTEXT_GOAL:{}", goal.goal_id),
                        )
                    })
                })
        };
        let ordinal_entity_id = ordinal_target.map(|(surface, canonical_concept, evidence)| {
            let entity_id = format!("NX{:03}", entities.len() + 1);
            entities.push(NativeEntityIR {
                entity_id: entity_id.clone(),
                surface,
                canonical_concept,
                start_byte: 0,
                end_byte: 0,
                rejected_by_contrast: false,
                confidence_millis: 960,
                evidence: vec![evidence],
            });
            entity_id
        });
        let verified_result_query = requests_verified_result(&lower);
        let action_outcome_report = reports_completed_user_action(&lower);
        let competing_outcome_reports = reports_competing_outcomes(&lower);
        let answer_mode = if action_outcome_report {
            Some(NativeResponseModeIR::ReportedOutcome)
        } else if competing_outcome_reports {
            Some(NativeResponseModeIR::CompetingOutcomeReports)
        } else if asks_what_is_certain(&lower) {
            Some(NativeResponseModeIR::SourceCertaintyQuery)
        } else if asks_outcome_alternative(&lower) {
            Some(NativeResponseModeIR::OutcomeAlternativeQuery)
        } else if asks_for_result_contents(&lower) || asks_for_findings_over_plan(&lower) {
            Some(NativeResponseModeIR::EvidenceResultQuery)
        } else if asks_verification_status(&lower) {
            Some(NativeResponseModeIR::VerificationStatusQuery)
        } else if verified_result_query {
            Some(NativeResponseModeIR::EvidenceResultQuery)
        } else {
            None
        };
        if verified_result_query || action_outcome_report {
            // A predicate inside a lifecycle question denotes the queried
            // prior event; it is not a fresh imperative action.
            actions.clear();
        }
        let verified_result_context = verified_result_query
            .then(|| {
                context
                    .active_goals
                    .iter()
                    .filter(|goal| is_lifecycle_query_target(goal.intent))
                    .max_by_key(|goal| (goal.discourse_focused, goal.introduced_turn))
            })
            .flatten();
        let result_target_skips_newer_non_lifecycle_goal =
            verified_result_context.is_some_and(|selected| {
                context.active_goals.iter().any(|goal| {
                    goal.introduced_turn > selected.introduced_turn
                        && !is_lifecycle_query_target(goal.intent)
                })
            });
        let verified_result_entity_id = verified_result_context.map(|goal| {
            let entity_id = format!("NX{:03}", entities.len() + 1);
            entities.push(NativeEntityIR {
                entity_id: entity_id.clone(),
                surface: goal.subject.clone(),
                canonical_concept: context_subject_concept(&goal.subject),
                start_byte: 0,
                end_byte: 0,
                rejected_by_contrast: false,
                confidence_millis: 970,
                evidence: vec![format!("DIALOGUE_CONTEXT_GOAL:{}", goal.goal_id)],
            });
            entity_id
        });
        let mut events = Vec::new();
        let mut bindings = Vec::new();
        if let Some((goal, entity_id)) = verified_result_context.zip(verified_result_entity_id) {
            let mut evidence = vec![
                "LIFECYCLE_QUERY_TARGETS_PRIOR_ACTION".to_string(),
                "CLAIM_REJECTION_DOES_NOT_REPLACE_RESULT_TARGET".to_string(),
            ];
            if result_target_skips_newer_non_lifecycle_goal {
                evidence.push("RESULT_TARGET_RESUMES_BEYOND_NEWER_NON_LIFECYCLE_GOAL".to_string());
            }
            bindings.push(NativeReferenceBindingIR {
                binding_id: "NB001".to_string(),
                kind: NativeReferenceKindIR::VerifiedResultTarget,
                source_surface: "VERIFIED_RESULT_QUERY".to_string(),
                target_entity_id: entity_id,
                inherited_goal_id: Some(goal.goal_id.clone()),
                confidence_millis: 970,
                evidence,
            });
        }
        let mut unresolved = Vec::new();
        if context_entity_ambiguity {
            unresolved.push("AMBIGUOUS_DIALOGUE_CONTEXT_ENTITY".to_string());
        }
        if actions.is_empty()
            && !verified_result_query
            && !action_outcome_report
            && !competing_outcome_reports
            && underspecified_problem_disclosure(&lower, &entities)
        {
            unresolved.push("UNDERSPECIFIED_PROBLEM_DISCLOSURE".to_string());
        }
        let coordinated_groups = coordinated_entity_groups(&entities, &lower);
        let mut last_coordinated_group = Vec::<String>::new();

        for (index, action) in actions.iter().enumerate() {
            let previous_end = index
                .checked_sub(1)
                .and_then(|previous| actions.get(previous))
                .map_or(0, |previous| previous.end);
            let next_start = actions
                .get(index + 1)
                .map_or(source.len(), |next| next.start);
            let left = connector_left_boundary(&lower, previous_end, action.start);
            // A sequencing connective between two objects does not terminate
            // the theme span when the utterance contains only one predicate
            // ("check A and then B"). Connector boundaries separate actions,
            // not coordinated arguments of the same action.
            let right = if actions.len() == 1 {
                source.len()
            } else {
                connector_right_boundary(&lower, action.end, next_start)
            };
            let scope_text = &lower[left..right];
            let marker_text = &lower[left..next_start];
            let action_offset = action.start.saturating_sub(left);
            let replacement_suppresses_action = ["rather than", "말고"]
                .iter()
                .filter_map(|marker| scope_text.find(marker))
                .any(|marker_offset| marker_offset < action_offset)
                || action_rejected_by_following_korean_correction(
                    marker_text,
                    action_offset,
                    action.end.saturating_sub(left),
                );
            let scope = if action
                .knowledge_surface
                .starts_with("CONSTRAINT_CONTINUATION:")
            {
                NativeEventScopeIR::Live
            } else if replacement_suppresses_action
                || PROHIBITION_MARKERS
                    .iter()
                    .any(|marker| scope_text.contains(marker))
            {
                NativeEventScopeIR::Prohibited
            } else if REPORTED_SCOPE_MARKERS
                .iter()
                .any(|marker| scope_text.contains(marker))
            {
                NativeEventScopeIR::Reported
            } else if POSSIBLE_SCOPE_MARKERS
                .iter()
                .any(|marker| scope_text.contains(marker))
            {
                NativeEventScopeIR::Possible
            } else if CONDITIONAL_MARKERS
                .iter()
                .any(|marker| scope_text.contains(marker))
            {
                NativeEventScopeIR::Conditional
            } else {
                NativeEventScopeIR::Live
            };

            let mut local = entities
                .iter()
                .filter(|entity| {
                    !entity.rejected_by_contrast
                        && entity.start_byte < entity.end_byte
                        && entity.start_byte >= left
                        && entity.end_byte <= right
                })
                .map(|entity| entity.entity_id.clone())
                .collect::<Vec<_>>();
            if action.inherited_goal_id.is_some() {
                local.retain(|entity_id| {
                    entities
                        .iter()
                        .find(|entity| &entity.entity_id == entity_id)
                        .is_none_or(|entity| {
                            !matches!(
                                entity.canonical_concept.as_str(),
                                "C_TASK" | "C_ISSUE" | "C_PROBLEM" | "C_EXPLANATION"
                            )
                        })
                });
            }
            if local.len() > 1 && !entities_are_coordinated(&local, &entities, &lower) {
                let has_concrete_theme = local.iter().any(|entity_id| {
                    entities
                        .iter()
                        .find(|entity| &entity.entity_id == entity_id)
                        .is_some_and(|entity| {
                            !matches!(
                                entity.canonical_concept.as_str(),
                                "C_TASK" | "C_ISSUE" | "C_PROBLEM" | "C_EXPLANATION"
                            )
                        })
                });
                if has_concrete_theme {
                    local.retain(|entity_id| {
                        entities
                            .iter()
                            .find(|entity| &entity.entity_id == entity_id)
                            .is_none_or(|entity| {
                                !matches!(
                                    entity.canonical_concept.as_str(),
                                    "C_TASK" | "C_ISSUE" | "C_PROBLEM" | "C_EXPLANATION"
                                )
                            })
                    });
                }
            }
            let local = if local.len() > 1 && entities_are_coordinated(&local, &entities, &lower) {
                last_coordinated_group = local.clone();
                local
            } else if local.len() > 1 {
                nearest_entity_ids(action, &local, &entities)
            } else {
                local
            };
            let (theme_entity_ids, inherited_reference) = if !context_set_entity_ids.is_empty()
                && contains_plural_context_reference_marker(marker_text)
            {
                (
                    context_set_entity_ids.clone(),
                    Some(NativeReferenceKindIR::PluralContextSet),
                )
            } else if let Some(entity_id) = ordinal_entity_id.as_ref() {
                (
                    vec![entity_id.clone()],
                    Some(if contains_correction_construction(&lower) {
                        NativeReferenceKindIR::ContrastiveRetarget
                    } else {
                        NativeReferenceKindIR::EventOrdinal
                    }),
                )
            } else if let Some(select_last) = set_member_selection(scope_text) {
                let group = nearest_coordinated_group(action, &coordinated_groups, &entities)
                    .or_else(|| {
                        (!last_coordinated_group.is_empty()).then_some(&last_coordinated_group)
                    });
                if let Some(group) = group {
                    let selected = if select_last {
                        group.last()
                    } else {
                        group.first()
                    };
                    (
                        vec![selected.expect("checked non-empty").clone()],
                        Some(NativeReferenceKindIR::SetMember),
                    )
                } else if !context_set_entity_ids.is_empty() {
                    let selected = if select_last {
                        context_set_entity_ids.last()
                    } else {
                        context_set_entity_ids.first()
                    };
                    (
                        vec![selected.expect("checked non-empty context set").clone()],
                        Some(NativeReferenceKindIR::SetMember),
                    )
                } else {
                    (Vec::new(), None)
                }
            } else if action.inherited_goal_id.is_some() && !local.is_empty() {
                (
                    local,
                    Some(if contains_correction_construction(&lower) {
                        NativeReferenceKindIR::ContrastiveRetarget
                    } else {
                        NativeReferenceKindIR::OperationEllipsis
                    }),
                )
            } else if !local.is_empty() && contains_deictic_marker(marker_text) {
                let kind = if entities.iter().any(|entity| entity.rejected_by_contrast) {
                    NativeReferenceKindIR::ContrastiveRetarget
                } else {
                    NativeReferenceKindIR::IntraTurnAnaphora
                };
                (local, Some(kind))
            } else if !local.is_empty() {
                (local, None)
            } else {
                let prior_focus = entities
                    .iter()
                    .rfind(|entity| {
                        !entity.rejected_by_contrast
                            && (entity.start_byte < action.start
                                || entity.evidence.iter().any(|item| {
                                    item.starts_with("DIALOGUE_CONTEXT_GOAL:")
                                        || item.starts_with("DIALOGUE_CONTEXT_ENTITY:")
                                }))
                    })
                    .map(|entity| entity.entity_id.clone());
                let forward_focus = entities
                    .iter()
                    .filter(|entity| {
                        !entity.rejected_by_contrast
                            && entity.start_byte > action.end
                            && entity.start_byte < entity.end_byte
                    })
                    .min_by_key(|entity| entity.start_byte)
                    .map(|entity| entity.entity_id.clone());
                let focus = prior_focus.or(forward_focus);
                match focus {
                    Some(entity_id) => {
                        let kind = if contains_causal_marker(marker_text) {
                            NativeReferenceKindIR::OperationEllipsis
                        } else if entities.iter().any(|entity| entity.rejected_by_contrast) {
                            NativeReferenceKindIR::ContrastiveRetarget
                        } else if entities.iter().any(|entity| {
                            entity.entity_id == entity_id
                                && entity.start_byte == entity.end_byte
                                && entity.evidence.iter().any(|item| {
                                    item.starts_with("DIALOGUE_CONTEXT_GOAL:")
                                        || item.starts_with("DIALOGUE_CONTEXT_ENTITY:")
                                })
                        }) && !contains_deictic_marker(marker_text)
                        {
                            NativeReferenceKindIR::OperationEllipsis
                        } else if contains_deictic_marker(marker_text)
                            && entities.iter().any(|entity| {
                                entity.entity_id == entity_id && entity.end_byte <= left
                            })
                        {
                            NativeReferenceKindIR::ExplicitPriorTheme
                        } else {
                            NativeReferenceKindIR::IntraTurnAnaphora
                        };
                        (vec![entity_id], Some(kind))
                    }
                    None => (Vec::new(), None),
                }
            };
            let event_id = format!("NE{:03}", index + 1);
            if let Some(kind) = inherited_reference {
                let primary_target = theme_entity_ids
                    .first()
                    .expect("reference target must exist")
                    .clone();
                let evidence = if kind == NativeReferenceKindIR::ExplicitPriorTheme {
                    vec![
                        "EXPLICIT_DEICTIC_RESUMES_PRIOR_THEME".to_string(),
                        "CROSS_BOUNDARY_THEME_BINDING".to_string(),
                    ]
                } else {
                    vec![
                        "SAME_TURN_DISCOURSE_FOCUS".to_string(),
                        "EXPLICIT_CONSTRUCTION_MARKER".to_string(),
                    ]
                };
                let inherited_goal_id = action
                    .inherited_goal_id
                    .clone()
                    .or_else(|| ordinal_context.map(|goal| goal.goal_id.clone()))
                    .or_else(|| contextual_deictic_goal.map(|goal| goal.goal_id.clone()));
                let binding_targets = if kind == NativeReferenceKindIR::PluralContextSet {
                    theme_entity_ids.clone()
                } else {
                    vec![primary_target.clone()]
                };
                for target in binding_targets {
                    if !bindings.iter().any(|binding| {
                        binding.kind == kind
                            && binding.target_entity_id == target
                            && binding.inherited_goal_id == inherited_goal_id
                    }) {
                        bindings.push(NativeReferenceBindingIR {
                            binding_id: format!("NB{:03}", bindings.len() + 1),
                            kind,
                            source_surface: reference_marker(marker_text).to_string(),
                            target_entity_id: target,
                            inherited_goal_id: inherited_goal_id.clone(),
                            confidence_millis: 900,
                            evidence: evidence.clone(),
                        });
                    }
                }
                if kind == NativeReferenceKindIR::EventOrdinal
                    && action.inherited_goal_id.is_some()
                    && operation_ellipsis_marker(marker_text).is_some()
                {
                    bindings.push(NativeReferenceBindingIR {
                        binding_id: format!("NB{:03}", bindings.len() + 1),
                        kind: NativeReferenceKindIR::OperationEllipsis,
                        source_surface: "SAME_OPERATION".to_string(),
                        target_entity_id: primary_target,
                        inherited_goal_id,
                        confidence_millis: 900,
                        evidence: vec![
                            "OPERATION_INHERITED_INDEPENDENTLY_OF_ORDINAL_TARGET".to_string(),
                            "COMPOSITIONAL_REFERENCE_BINDING".to_string(),
                        ],
                    });
                }
            }
            let confidence_millis = if theme_entity_ids.is_empty() {
                550
            } else {
                930
            };
            if scope == NativeEventScopeIR::Live && theme_entity_ids.is_empty() {
                unresolved.push(format!("UNBOUND_THEME:{event_id}"));
            }
            events.push(NativeEventIR {
                event_id,
                canonical_predicate: action.canonical_predicate.clone(),
                predicate_surface: source[action.start..action.end].to_string(),
                intent: action.intent,
                scope,
                theme_entity_ids,
                start_byte: action.start,
                end_byte: action.end,
                confidence_millis,
                evidence: vec![
                    format!("ACTION_KNOWLEDGE:{}", action.knowledge_surface),
                    format!("SCOPE:{scope:?}"),
                    "CONSTRUCTION_NOT_SENTENCE_TEMPLATE".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
        }

        // A turn-initial agreement followed by an explicit prohibition is a
        // discourse correction, not merely an isolated negative command.
        // Keep the prohibited event as semantic constraint evidence while
        // recording which contextual entity the speaker has just retargeted.
        // This construction-level rule transfers across predicates and entity
        // types; no completed sentence is stored.
        if acknowledges_prior_turn(&lower) && !context.active_entities.is_empty() {
            let prohibited_theme_ids = events
                .iter()
                .filter(|event| event.scope == NativeEventScopeIR::Prohibited)
                .flat_map(|event| event.theme_entity_ids.iter())
                .cloned()
                .collect::<BTreeSet<_>>();
            if prohibited_theme_ids.len() == 1 {
                let target = prohibited_theme_ids
                    .into_iter()
                    .next()
                    .expect("checked singleton prohibited theme");
                if !bindings.iter().any(|binding| {
                    binding.kind == NativeReferenceKindIR::ContrastiveRetarget
                        && binding.target_entity_id == target
                }) {
                    bindings.push(NativeReferenceBindingIR {
                        binding_id: format!("NB{:03}", bindings.len() + 1),
                        kind: NativeReferenceKindIR::ContrastiveRetarget,
                        source_surface: "ACKNOWLEDGED_CONSTRAINT".to_string(),
                        target_entity_id: target,
                        inherited_goal_id: None,
                        confidence_millis: 940,
                        evidence: vec![
                            "TURN_INITIAL_ACKNOWLEDGEMENT_LINKS_PRIOR_DISCOURSE".to_string(),
                            "EXPLICIT_PROHIBITION_RETARGETS_CONTEXT_ENTITY".to_string(),
                        ],
                    });
                }
            }
        }

        let mut selected_live_goals = Vec::new();
        for event in events
            .iter()
            .filter(|event| event.scope == NativeEventScopeIR::Live)
        {
            if event.theme_entity_ids.is_empty() {
                continue;
            }
            let selected_entities = event
                .theme_entity_ids
                .iter()
                .filter_map(|entity_id| {
                    entities
                        .iter()
                        .find(|entity| &entity.entity_id == entity_id)
                })
                .collect::<Vec<_>>();
            let subject = selected_entities
                .iter()
                .map(|entity| entity.surface.as_str())
                .collect::<Vec<_>>()
                .join(if language == LanguageCodeIR::Korean {
                    "와 "
                } else {
                    " and "
                });
            selected_live_goals.push(NativeGoalIR {
                goal_id: format!("NG{:03}", selected_live_goals.len() + 1),
                source_event_id: event.event_id.clone(),
                canonical_predicate: event.canonical_predicate.clone(),
                intent: event.intent,
                subject,
                subject_concepts: selected_entities
                    .iter()
                    .map(|entity| entity.canonical_concept.clone())
                    .collect(),
                confidence_millis: event.confidence_millis,
                selection_reasons: vec![
                    "EVENT_SCOPE_IS_LIVE".to_string(),
                    "THEME_BOUND_TO_EXPLICIT_DISCOURSE_ENTITY".to_string(),
                    "CONDITIONAL_AND_PROHIBITED_EVENTS_EXCLUDED".to_string(),
                ],
                semantic_authority: false,
                external_execution_authorized: false,
            });
        }
        unresolved.sort();
        unresolved.dedup();
        let mut seen_goal_semantics = BTreeSet::new();
        selected_live_goals.retain(|goal| {
            let mut concepts = goal.subject_concepts.clone();
            concepts.sort();
            seen_goal_semantics.insert((goal.intent, goal.canonical_predicate.clone(), concepts))
        });
        if selected_live_goals.len() > 1 && !multi_goal_relation_licensed(&lower) {
            unresolved.push("UNLICENSED_MULTI_GOAL_COMPETITION".to_string());
        }
        let relations = build_relations(&events, &lower);
        let response_goal =
            if verified_result_query || action_outcome_report || competing_outcome_reports {
                NativeResponseGoalIR::AnswerVerifiedResult
            } else if !unresolved.is_empty() {
                NativeResponseGoalIR::AskClarification
            } else if selected_live_goals.is_empty() {
                NativeResponseGoalIR::Acknowledge
            } else {
                NativeResponseGoalIR::PlanActions
            };
        let response_mode = answer_mode.unwrap_or(match response_goal {
            NativeResponseGoalIR::PlanActions => NativeResponseModeIR::Plan,
            NativeResponseGoalIR::AskClarification => NativeResponseModeIR::Clarification,
            NativeResponseGoalIR::Acknowledge => NativeResponseModeIR::Acknowledgement,
            NativeResponseGoalIR::AnswerVerifiedResult => NativeResponseModeIR::EvidenceResultQuery,
        });
        entities.sort_by_key(|entity| entity.start_byte);
        let mut turn = NativeTurnIR {
            schema: NATIVE_LANGUAGE_CIRCUIT_SCHEMA.to_string(),
            language,
            source_sha256: sha256_text(source),
            entities,
            events,
            relations,
            reference_bindings: bindings,
            selected_live_goals,
            response_goal,
            response_mode,
            unresolved,
            selected_semantic_sha256: String::new(),
            circuit_sha256: String::new(),
            semantic_authority: false,
            language_can_execute: false,
        };
        turn.selected_semantic_sha256 = selected_semantic_sha256(&turn);
        turn.circuit_sha256 = native_turn_sha256(&turn);
        debug_assert!(turn.validate_for_source(source));
        turn
    }
}

fn action_matches(lower: &str) -> Vec<ActionMatch> {
    let mut candidates = Vec::new();
    for lexeme in ACTION_LEXEMES {
        for (start, _) in lower.match_indices(lexeme.surface) {
            let end = start + lexeme.surface.len();
            if lexeme.surface.is_ascii() && !ascii_word_boundaries(lower, start, end) {
                continue;
            }
            candidates.push(ActionMatch {
                start,
                end,
                knowledge_surface: lexeme.surface.to_string(),
                canonical_predicate: lexeme.canonical_predicate.to_string(),
                intent: lexeme.intent,
                inherited_goal_id: None,
            });
        }
    }
    candidates.extend(constructional_action_matches(lower));
    candidates.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
    });
    let mut selected = Vec::<ActionMatch>::new();
    for candidate in candidates {
        if selected
            .iter()
            .any(|known| candidate.start < known.end && candidate.end > known.start)
        {
            continue;
        }
        selected.push(candidate);
    }
    selected.sort_by_key(|candidate| candidate.start);
    selected
}

/// Projects an event from a productive grammatical construction. The lexical
/// inventory identifies semantic heads; determiners, recipients, and arbitrary
/// modifiers may vary independently. This is deliberately not a collection of
/// complete utterance templates.
fn constructional_action_matches(lower: &str) -> Vec<ActionMatch> {
    let tokens = token_spans(lower);
    let mut matches = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.surface.as_str(), "take" | "have") {
            if let Some(head) = tokens
                .iter()
                .skip(index + 1)
                .take(4)
                .find(|candidate| candidate.surface == "look")
            {
                let between = &lower[token.end..head.start];
                if !between.contains(['.', '?', '!', ';']) {
                    matches.push(ActionMatch {
                        start: token.start,
                        end: head.end,
                        knowledge_surface: "LIGHT_VERB+INVESTIGATION_HEAD".to_string(),
                        canonical_predicate: "INVESTIGATE".to_string(),
                        intent: PlanIntentIR::Investigate,
                        inherited_goal_id: None,
                    });
                }
            }
        }
        if token.surface == "help" && directive_control_head(lower, token.start) {
            if let Some(head) = tokens
                .iter()
                .skip(index + 1)
                .take(4)
                .find(|candidate| candidate.surface == "understand")
            {
                matches.push(ActionMatch {
                    start: token.start,
                    end: head.end,
                    knowledge_surface: "REQUEST_CONTROL+UNDERSTAND".to_string(),
                    canonical_predicate: "EXPLAIN".to_string(),
                    intent: PlanIntentIR::Explain,
                    inherited_goal_id: None,
                });
            }
        }
    }
    // Korean adnominal morphology is generated from the lexical stem class.
    // The construction (`ACTION-ㄹ + method/procedure/order`) is productive;
    // only the stem's inflection class is lexical knowledge.
    for (stem, canonical_predicate, intent) in [
        ("고치", "REPAIR", PlanIntentIR::Repair),
        ("좁히", "INVESTIGATE", PlanIntentIR::Investigate),
    ] {
        let Some(adnominal) = korean_rieul_adnominal(stem) else {
            continue;
        };
        for (start, _) in lower.match_indices(&adnominal) {
            let end = start + adnominal.len();
            if ["방법", "절차", "순서"]
                .iter()
                .any(|head| lower[end..].trim_start().starts_with(head))
            {
                matches.push(ActionMatch {
                    start,
                    end,
                    knowledge_surface: format!("KOREAN_ADNOMINAL:{stem}"),
                    canonical_predicate: canonical_predicate.to_string(),
                    intent,
                    inherited_goal_id: None,
                });
            }
        }
    }
    matches
}

fn korean_rieul_adnominal(stem: &str) -> Option<String> {
    const HANGUL_BASE: u32 = 0xAC00;
    const HANGUL_END: u32 = 0xD7A3;
    const JONGSEONG_COUNT: u32 = 28;
    const RIEUL_JONGSEONG: u32 = 8;
    let mut characters = stem.chars().collect::<Vec<_>>();
    let last = characters.pop()?;
    let code = u32::from(last);
    if !(HANGUL_BASE..=HANGUL_END).contains(&code) {
        return None;
    }
    let syllable = code - HANGUL_BASE;
    if !syllable.is_multiple_of(JONGSEONG_COUNT) {
        return None;
    }
    let inflected = char::from_u32(code + RIEUL_JONGSEONG)?;
    characters.push(inflected);
    Some(characters.into_iter().collect())
}

fn directive_control_head(lower: &str, start: usize) -> bool {
    let prefix = lower[..start].trim();
    prefix.is_empty()
        || matches!(
            prefix,
            "please" | "could you" | "can you" | "would you" | "will you"
        )
}

/// In a control/complement construction, the semantic head of the requested
/// operation is selected by the typed relation, not by whichever verb happens
/// to be nearest. Examples include an epistemic carrier governing an English
/// infinitival action and a Korean action modifying a method noun.
fn collapse_controlled_action_complements(actions: &mut Vec<ActionMatch>, lower: &str) {
    let mut suppressed = BTreeSet::new();
    for (left_index, left) in actions.iter().enumerate() {
        for (right_index, right) in actions.iter().enumerate().skip(left_index + 1) {
            if left.end > right.start {
                continue;
            }
            let between = &lower[left.end..right.start];
            let english_infinitival_action = left.intent == PlanIntentIR::Investigate
                && between
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .any(|pair| pair == ["how", "to"]);
            if english_infinitival_action {
                suppressed.insert(left_index);
            }
            let korean_method_complement = left.intent != right.intent
                && ["방법", "절차", "순서"]
                    .iter()
                    .any(|head| between.contains(head));
            if korean_method_complement {
                suppressed.insert(right_index);
            }
        }
    }
    let mut index = 0;
    actions.retain(|_| {
        let retain = !suppressed.contains(&index);
        index += 1;
        retain
    });
}

/// A Korean action noun carrying a case/topic particle is an argument of the
/// following predicate, not a second requested event. Scope is then assigned
/// to the governing predicate (for example, a prohibited application).
fn suppress_korean_nominal_argument_actions(actions: &mut Vec<ActionMatch>, lower: &str) {
    let mut suppressed = BTreeSet::new();
    for (index, pair) in actions.windows(2).enumerate() {
        let between = &lower[pair[0].end..pair[1].start];
        let grammatical_tail = between.trim_start();
        if ["은", "는", "이", "가", "을", "를"]
            .iter()
            .any(|particle| grammatical_tail.starts_with(particle))
            && !between.contains(['.', '?', '!', ';'])
        {
            suppressed.insert(index);
        }
    }
    if let Some((governor_start, _, _)) = continuation_constraint_marker(lower) {
        for (index, action) in actions.iter().enumerate() {
            if action.end > governor_start {
                continue;
            }
            let between = &lower[action.end..governor_start];
            if ["은", "는", "이", "가", "을", "를"]
                .iter()
                .any(|particle| between.trim_start().starts_with(particle))
                && !between.contains(['.', '?', '!', ';'])
            {
                suppressed.insert(index);
            }
        }
    }
    let mut index = 0;
    actions.retain(|_| {
        let retain = !suppressed.contains(&index);
        index += 1;
        retain
    });
}

fn non_overlapping_action_matches(mut actions: Vec<ActionMatch>) -> Vec<ActionMatch> {
    fn priority(action: &ActionMatch) -> u8 {
        if action.knowledge_surface.starts_with("RETARGET_OPERATION:") {
            3
        } else if action
            .knowledge_surface
            .starts_with("CONSTRAINT_CONTINUATION:")
            || action.knowledge_surface.starts_with("OPERATION_ELLIPSIS:")
        {
            2
        } else {
            1
        }
    }

    actions.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| priority(right).cmp(&priority(left)))
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
    });
    let mut selected = Vec::<ActionMatch>::new();
    for candidate in actions {
        let overlaps = selected
            .iter()
            .position(|known| candidate.start < known.end && candidate.end > known.start);
        if let Some(index) = overlaps {
            if priority(&candidate) > priority(&selected[index]) {
                selected[index] = candidate;
            }
        } else {
            selected.push(candidate);
        }
    }
    selected.sort_by_key(|action| action.start);
    selected
}

fn suppress_explanation_complement_actions(actions: &mut Vec<ActionMatch>, lower: &str) {
    let Some(explanation) = actions
        .iter()
        .find(|action| action.intent == PlanIntentIR::Explain)
        .cloned()
    else {
        return;
    };
    actions.retain(|action| {
        if action.intent == PlanIntentIR::Explain
            || action.end > explanation.start
            || action.end > lower.len()
        {
            return true;
        }
        let complement = &lower[action.end..explanation.start];
        !["는 이유", "보는 이유", "하는 이유", "한 이유", "할 이유"]
            .iter()
            .any(|shape| complement.contains(shape))
    });
    let complement_cue = ["why ", "what ", "how ", "왜 ", "뭘 ", "무엇을 "]
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min();
    let nominal_complement = actions.iter().any(|action| {
        action.intent != PlanIntentIR::Explain
            && action.start > explanation.end
            && lower[explanation.end..action.start].contains(" of ")
    });
    if nominal_complement {
        actions.retain(|action| action.intent == PlanIntentIR::Explain);
    } else if complement_cue.is_some_and(|start| start < explanation.start) {
        actions.retain(|action| {
            action.intent == PlanIntentIR::Explain
                || action.end <= complement_cue.expect("checked complement cue")
        });
    } else if explanation.start == 0 && complement_cue.is_some() {
        actions.retain(|action| action.intent == PlanIntentIR::Explain);
    }
}

fn suppress_nominal_action_modifiers(actions: &mut Vec<ActionMatch>, lower: &str) {
    // In compounds such as "recovery procedure", recovery denotes the kind
    // of artifact to design; it is not an imperative to recover immediately.
    // The head predicate remains available to the compositional analyzer.
    let plans_an_artifact = ["절차를 설계", "계획을 설계", "방법을 설계"]
        .iter()
        .any(|shape| lower.contains(shape));
    if plans_an_artifact {
        actions.retain(|action| {
            !(action.intent == PlanIntentIR::Repair
                && action.predicate_surface_or_knowledge_is_korean_recovery())
        });
    }
}

impl ActionMatch {
    fn predicate_surface_or_knowledge_is_korean_recovery(&self) -> bool {
        self.knowledge_surface == "복구"
    }
}

fn retarget_carrier_marker(lower: &str) -> Option<(usize, usize, &'static str)> {
    if (lower.contains("switch the") || lower.contains("change the")) && lower.contains("target to")
    {
        let start = lower
            .find("switch the")
            .or_else(|| lower.find("change the"))?;
        let end = lower.find("target to")? + "target to".len();
        return Some((start, end, "TARGET_TO"));
    }
    let compositional_retarget = contains_correction_construction(lower)
        && (ascii_word_position(lower, "target").is_some() || lower.contains("대상"));
    if compositional_retarget {
        if let Some((start, end)) = ascii_word_position(lower, "target") {
            return Some((start, end, "CORRECTION+TARGET_ROLE"));
        }
        if let Some(start) = lower.find("대상") {
            return Some((start, start + "대상".len(), "교정+대상_역할"));
        }
    }
    let target = lower.find("대상")?;
    let suffix = &lower[target..];
    let (offset, marker) = ["바꿔", "바꾸"]
        .into_iter()
        .filter_map(|marker| suffix.find(marker).map(|offset| (offset, marker)))
        .min_by_key(|(offset, _)| *offset)?;
    Some((target, target + offset + marker.len(), "대상_변경"))
}

fn continuation_constraint_marker(lower: &str) -> Option<(usize, usize, &'static str)> {
    [
        "read-only",
        "only observe",
        "leave everything untouched",
        "without applying",
        "without changing",
        "without modifying",
        "do not change",
        "don't change",
        "do not modify",
        "don't modify",
        "just prepare the steps",
        "읽기만",
        "관찰만",
        "바꾸지는 마",
        "수정은 아직",
        "실행하지 말고",
        "순서만 준비",
        "적용하지",
    ]
    .into_iter()
    .filter_map(|marker| {
        lower
            .find(marker)
            .map(|start| (start, start + marker.len(), marker))
    })
    .min_by_key(|(start, _, _)| *start)
}

fn operation_ellipsis_marker(lower: &str) -> Option<(usize, usize, &'static str)> {
    if set_member_selection(lower).is_some() {
        for continuation in ["continue", "proceed", "resume"] {
            if let Some((start, end)) = ascii_word_position(lower, continuation) {
                return Some((start, end, "CONTINUATION+SET_MEMBER"));
            }
        }
        if let Some(start) = ["계속", "이어가", "진행"]
            .into_iter()
            .filter_map(|continuation| lower.find(continuation))
            .min()
        {
            let marker = if lower[start..].starts_with("계속") {
                "계속"
            } else if lower[start..].starts_with("이어가") {
                "이어가"
            } else {
                "진행"
            };
            return Some((start, start + marker.len(), "연속+집합_구성원"));
        }
    }
    [
        "apply that operation",
        "apply the operation",
        "apply the action",
        "do the same to",
        "do the same",
        "same investigation",
        "start with",
        "do that",
        "do this",
        "do it",
        "go ahead with",
        "똑같이 해",
        "똑같이",
        "같은 조사를",
        "같은 조사",
        "같은 작업을",
        "같은 작업",
        "같이 해",
        "그거 먼저",
        "그것부터",
        "그거부터",
        "뒤의 것부터",
        "앞의 것부터",
        "그대로 진행",
        "i meant",
        "meant the",
        "instead",
        "말한 거야",
        "말한 거",
    ]
    .into_iter()
    .filter_map(|marker| {
        lower
            .find(marker)
            .map(|start| (start, start + marker.len(), marker))
    })
    .min_by_key(|(start, _, _)| *start)
}

fn ordinal_goal_index(lower: &str) -> Option<usize> {
    let mut mentions = [
        (
            1,
            [
                "first issue",
                "issue one",
                "problem one",
                "first one",
                "first target",
                "first task",
                "first subject",
                "1st",
                "1번 문제",
                "첫 번째",
                "첫번째",
            ],
        ),
        (
            2,
            [
                "second issue",
                "issue two",
                "problem two",
                "second one",
                "second target",
                "second task",
                "second subject",
                "2nd",
                "2번 문제",
                "두 번째",
                "두번째",
            ],
        ),
        (
            3,
            [
                "third issue",
                "issue three",
                "problem three",
                "third one",
                "third target",
                "third task",
                "third subject",
                "3rd",
                "3번 문제",
                "세 번째",
                "세번째",
            ],
        ),
    ]
    .into_iter()
    .flat_map(|(ordinal, markers)| {
        markers.into_iter().flat_map(move |marker| {
            lower
                .match_indices(marker)
                .map(move |(start, matched)| (start, start + matched.len(), ordinal))
        })
    })
    .collect::<Vec<_>>();
    mentions.sort_unstable();
    mentions.dedup();
    mentions
        .iter()
        .find(|(start, end, _)| !ordinal_mention_is_rejected(lower, *start, *end))
        .map(|(_, _, ordinal)| *ordinal)
}

fn ordinal_denotes_goal_history(lower: &str) -> bool {
    [
        "first issue",
        "second issue",
        "third issue",
        "issue one",
        "issue two",
        "issue three",
        "first problem",
        "second problem",
        "third problem",
        "problem one",
        "problem two",
        "problem three",
        "first task",
        "first subject",
        "second task",
        "third task",
        "문제",
        "작업",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn ordinal_mention_is_rejected(lower: &str, start: usize, end: usize) -> bool {
    let prefix = &lower[..start];
    let suffix = &lower[end..];
    let english_negation = prefix
        .match_indices("not")
        .filter(|(index, matched)| ascii_word_boundaries(lower, *index, *index + matched.len()))
        .last()
        .is_some_and(|(index, _)| {
            let between = lower[index + "not".len()..start].trim();
            between.is_empty() || between == "the"
        });
    let korean_replacement = ["아니라", "말고"].iter().any(|marker| {
        suffix.find(marker).is_some_and(|offset| {
            suffix[..offset]
                .trim_matches(|c: char| !c.is_alphanumeric())
                .chars()
                .count()
                <= 2
        })
    });
    english_negation || korean_replacement
}

fn contains_correction_construction(lower: &str) -> bool {
    let starts_with_no = lower
        .strip_prefix("no")
        .is_some_and(|rest| rest.starts_with(|character: char| !character.is_alphanumeric()));
    starts_with_no
        || [
            "i meant",
            "meant the",
            "rather than",
            "instead",
            "대신",
            "아니라",
            "말고",
            "말한 거",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn action_rejected_by_following_korean_correction(
    scope_text: &str,
    action_offset: usize,
    action_end_offset: usize,
) -> bool {
    action_offset < action_end_offset
        && action_end_offset <= scope_text.len()
        && ["말고", "아니라"].iter().any(|marker| {
            scope_text[action_end_offset..]
                .find(marker)
                .is_some_and(|offset| offset <= 8)
        })
}

fn context_subject_concept(subject: &str) -> String {
    extract_entities(subject, &subject.to_lowercase())
        .into_iter()
        .find(|entity| !entity.rejected_by_contrast)
        .map(|entity| entity.canonical_concept)
        .unwrap_or_else(|| {
            let normalized = subject
                .to_lowercase()
                .chars()
                .map(|character| {
                    if character.is_alphanumeric() {
                        character
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            format!("{}::C_CONTEXT_ENTITY", normalized.trim_matches('_'))
        })
}

pub(crate) fn subjects_share_context_concept(left: &str, right: &str) -> bool {
    fn normalized_subject(surface: &str) -> String {
        surface
            .to_lowercase()
            .split_whitespace()
            .map(|token| token.trim_matches(|character: char| !character.is_alphanumeric()))
            .filter(|token| !token.is_empty() && !matches!(*token, "the" | "a" | "an"))
            .collect::<Vec<_>>()
            .join(" ")
    }
    normalized_subject(left) == normalized_subject(right)
        || context_subject_concept(left) == context_subject_concept(right)
}

fn ascii_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|character| !character.is_ascii_alphanumeric())
        && after.is_none_or(|character| !character.is_ascii_alphanumeric())
}

fn token_spans(source: &str) -> Vec<TokenSpan> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, character) in source.char_indices() {
        let word = character.is_alphanumeric() || matches!(character, '_' | '-');
        match (start, word) {
            (None, true) => start = Some(index),
            (Some(token_start), false) => {
                tokens.push(TokenSpan {
                    start: token_start,
                    end: index,
                    surface: source[token_start..index].to_string(),
                });
                start = None;
            }
            _ => {}
        }
    }
    if let Some(token_start) = start {
        tokens.push(TokenSpan {
            start: token_start,
            end: source.len(),
            surface: source[token_start..].to_string(),
        });
    }
    tokens
}

fn extract_entities(source: &str, lower: &str) -> Vec<NativeEntityIR> {
    let tokens = token_spans(source);
    let mut spans = Vec::<(usize, usize, String, String, Vec<String>)>::new();
    let mut covered_resource_tokens = BTreeSet::new();
    for (index, token) in tokens.iter().enumerate() {
        if !is_proper_entity_token(&token.surface) {
            continue;
        }
        if let Some(next) = tokens.get(index + 1) {
            if let Some(kind) = resource_kind(&next.surface) {
                spans.push((
                    token.start,
                    next.end,
                    format!("{} {}", token.surface, resource_surface(&next.surface)),
                    format!("{}::{kind}", token.surface.to_lowercase()),
                    vec!["PROPER_NAME_PLUS_RESOURCE_TYPE".to_string()],
                ));
                covered_resource_tokens.insert(index + 1);
                continue;
            }
        }
        spans.push((
            token.start,
            token.end,
            token.surface.clone(),
            format!("{}::C_ENTITY", token.surface.to_lowercase()),
            vec!["PROPER_NAME_ENTITY".to_string()],
        ));
    }
    for (index, token) in tokens.iter().enumerate() {
        if covered_resource_tokens.contains(&index) {
            continue;
        }
        if let Some(kind) = resource_kind(&token.surface) {
            spans.push((
                token.start,
                token.end,
                resource_surface(&token.surface),
                kind.to_string(),
                vec!["RESOURCE_TYPE_ENTITY".to_string()],
            ));
        }
    }
    spans.sort_by_key(|span| span.0);
    spans.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    spans
        .into_iter()
        .enumerate()
        .map(
            |(index, (start, end, surface, canonical_concept, mut evidence))| {
                let rejected = contrastively_rejected(lower, start, end);
                if rejected {
                    evidence.push("CONTRASTIVE_REJECTION".to_string());
                }
                NativeEntityIR {
                    entity_id: format!("NX{:03}", index + 1),
                    surface,
                    canonical_concept,
                    start_byte: start,
                    end_byte: end,
                    rejected_by_contrast: rejected,
                    confidence_millis: if rejected { 950 } else { 900 },
                    evidence,
                }
            },
        )
        .collect()
}

fn recent_context_entities(context: &NativeDialogueContextIR) -> Vec<ContextEntityCandidate> {
    let mut candidates = context
        .active_entities
        .iter()
        .enumerate()
        .map(|(source_order, entity)| ContextEntityCandidate {
            referent_id: entity.referent_id.clone(),
            surface: entity.surface.clone(),
            canonical_concept: context_subject_concept(&entity.surface),
            introduced_turn: entity.introduced_turn,
            last_mentioned_turn: entity.last_mentioned_turn,
            source_order,
        })
        .collect::<Vec<_>>();
    // Focused typed entities are the resolved discourse projection. Referent
    // summaries are a fallback source, not a peer source to union back into
    // that projection; unioning them reintroduced incidental nouns and turned
    // a unique ellipsis target into an artificial ambiguity.
    if candidates.is_empty() {
        for (referent_order, referent) in context.active_referents.iter().enumerate() {
            let lower = referent.semantic_summary.to_lowercase();
            candidates.extend(
                extract_entities(&referent.semantic_summary, &lower)
                    .into_iter()
                    .enumerate()
                    .map(|(entity_order, entity)| ContextEntityCandidate {
                        referent_id: format!("{}:{}", referent.referent_id, entity.entity_id),
                        surface: entity.surface,
                        canonical_concept: entity.canonical_concept,
                        introduced_turn: referent.introduced_turn,
                        last_mentioned_turn: referent.last_referenced_turn,
                        source_order: referent_order * 32 + entity_order,
                    }),
            );
        }
    }
    let Some(latest_turn) = candidates
        .iter()
        .map(|candidate| candidate.last_mentioned_turn)
        .max()
    else {
        return Vec::new();
    };
    candidates.retain(|candidate| candidate.last_mentioned_turn == latest_turn);
    candidates.sort_by_key(|candidate| (candidate.introduced_turn, candidate.source_order));
    let mut seen = BTreeSet::new();
    candidates.retain(|candidate| seen.insert(candidate.surface.trim().to_lowercase()));
    candidates
}

fn uniquely_recent_replayable_context_goal(
    goals: &[NativeContextGoalIR],
) -> Option<&NativeContextGoalIR> {
    let focused = goals
        .iter()
        .filter(|goal| goal.operation_replayable && goal.discourse_focused)
        .collect::<Vec<_>>();
    if semantically_unique_goal(&focused) {
        return focused.into_iter().max_by_key(|goal| goal.introduced_turn);
    }
    let latest_turn = goals.iter().map(|goal| goal.introduced_turn).max()?;
    let recent = goals
        .iter()
        .filter(|goal| goal.operation_replayable && goal.introduced_turn == latest_turn)
        .collect::<Vec<_>>();
    semantically_unique_goal(&recent)
        .then(|| recent.into_iter().max_by_key(|goal| &goal.goal_id))
        .flatten()
}

fn semantically_unique_goal(goals: &[&NativeContextGoalIR]) -> bool {
    let Some(first) = goals.first() else {
        return false;
    };
    goals.iter().all(|goal| {
        goal.intent == first.intent
            && goal.canonical_predicate == first.canonical_predicate
            && subjects_share_context_concept(&goal.subject, &first.subject)
    })
}

/// A selector may resolve the target independently from the operation. In
/// that case operation inheritance is licensed only when every live candidate
/// agrees on the operation; differing operations remain ambiguous.
fn shared_replayable_context_operation(
    goals: &[NativeContextGoalIR],
) -> Option<&NativeContextGoalIR> {
    let latest_turn = goals
        .iter()
        .filter(|goal| goal.operation_replayable)
        .map(|goal| goal.introduced_turn)
        .max()?;
    let candidates = goals
        .iter()
        .filter(|goal| goal.operation_replayable && goal.introduced_turn == latest_turn)
        .collect::<Vec<_>>();
    let first = candidates.first()?;
    candidates
        .iter()
        .all(|goal| {
            goal.intent == first.intent && goal.canonical_predicate == first.canonical_predicate
        })
        .then(|| {
            candidates
                .into_iter()
                .max_by_key(|goal| (goal.discourse_focused, &goal.goal_id))
        })
        .flatten()
}

fn is_proper_entity_token(surface: &str) -> bool {
    let Some(first) = surface.chars().next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    let lower = surface.to_ascii_lowercase();
    let action_head = ACTION_LEXEMES.iter().any(|lexeme| {
        lexeme
            .surface
            .split_whitespace()
            .next()
            .is_some_and(|head| head == lower)
    });
    !action_head
        && !matches!(
            lower.as_str(),
            "i" | "the"
                | "a"
                | "an"
                | "yes"
                | "yeah"
                | "okay"
                | "ok"
                | "um"
                | "uh"
                | "then"
                | "wait"
                | "did"
                | "is"
                | "are"
                | "am"
                | "does"
                | "what"
                | "which"
                | "who"
                | "why"
                | "when"
                | "where"
                | "how"
                | "so"
                | "but"
                | "and"
                | "right"
                | "different"
                | "actually"
                | "can"
                | "will"
                | "have"
                | "has"
                | "was"
                | "were"
                | "no"
                | "this"
                | "that"
                | "for"
                | "inspect"
                | "investigate"
                | "check"
                | "review"
                | "repair"
                | "fix"
                | "explain"
                | "describe"
                | "delete"
                | "remove"
                | "run"
                | "execute"
                | "apply"
                | "find"
                | "take"
                | "do"
                | "go"
                | "not"
                | "even"
                | "unless"
                | "only"
                | "could"
                | "would"
                | "tell"
                | "someone"
                | "set"
                | "provide"
                | "draft"
                | "switch"
                | "keep"
                | "start"
                | "please"
                | "help"
                | "continue"
                | "proceed"
                | "resume"
        )
}

fn acknowledges_prior_turn(text: &str) -> bool {
    let text = text.trim_start();
    [
        "right,", "right ", "correct,", "correct ", "yes,", "yes ", "yeah,", "yeah ", "맞아,",
        "맞아 ", "그래,", "그래 ", "응,", "응 ",
    ]
    .iter()
    .any(|marker| text.starts_with(marker))
}

fn is_lifecycle_query_target(intent: PlanIntentIR) -> bool {
    !matches!(
        intent,
        PlanIntentIR::Explain | PlanIntentIR::Communicate | PlanIntentIR::Plan
    )
}

fn resource_kind(surface: &str) -> Option<&'static str> {
    let lower = surface.to_lowercase();
    [
        ("cache", "C_CACHE"),
        ("log", "C_LOG"),
        ("queue", "C_QUEUE"),
        ("service", "C_SERVICE"),
        ("server", "C_SERVER"),
        ("worker", "C_WORKER"),
        ("gateway", "C_GATEWAY"),
        ("scheduler", "C_SCHEDULER"),
        ("relay", "C_RELAY"),
        ("pipeline", "C_PIPELINE"),
        ("report", "C_REPORT"),
        ("index", "C_INDEX"),
        ("migration", "C_MIGRATION"),
        ("file", "C_FILE"),
        ("folder", "C_FOLDER"),
        ("code", "C_SOURCE_CODE"),
        ("result", "C_RESULT"),
        ("answer", "C_RESPONSE"),
        ("response", "C_RESPONSE"),
        ("explanation", "C_EXPLANATION"),
        ("task", "C_TASK"),
        ("issue", "C_ISSUE"),
        ("problem", "C_PROBLEM"),
        ("캐시", "C_CACHE"),
        ("로그", "C_LOG"),
        ("큐", "C_QUEUE"),
        ("서비스", "C_SERVICE"),
        ("서버", "C_SERVER"),
        ("워커", "C_WORKER"),
        ("게이트웨이", "C_GATEWAY"),
        ("보고서", "C_REPORT"),
        ("인덱스", "C_INDEX"),
        ("마이그레이션", "C_MIGRATION"),
        ("파일", "C_FILE"),
        ("폴더", "C_FOLDER"),
        ("코드", "C_SOURCE_CODE"),
        ("결과", "C_RESULT"),
        ("답", "C_RESPONSE"),
        ("답변", "C_RESPONSE"),
        ("설명", "C_EXPLANATION"),
        ("작업", "C_TASK"),
        ("문제", "C_PROBLEM"),
    ]
    .into_iter()
    .find_map(|(surface, kind)| {
        (lower == surface
            || lower == format!("{surface}s")
            || lower.starts_with(surface) && korean_particle(&lower[surface.len()..]))
        .then_some(kind)
    })
}

fn resource_surface(surface: &str) -> String {
    let lower = surface.to_lowercase();
    [
        "migration",
        "service",
        "worker",
        "report",
        "problem",
        "issue",
        "queue",
        "cache",
        "index",
        "result",
        "task",
        "folder",
        "file",
        "code",
        "log",
        "마이그레이션",
        "서비스",
        "보고서",
        "인덱스",
        "워커",
        "캐시",
        "로그",
        "결과",
        "작업",
        "문제",
        "폴더",
        "파일",
        "코드",
        "큐",
    ]
    .into_iter()
    .find(|candidate| lower.starts_with(candidate))
    .unwrap_or(surface)
    .to_string()
}

fn korean_particle(remainder: &str) -> bool {
    matches!(
        remainder,
        "" | "은"
            | "는"
            | "이"
            | "가"
            | "을"
            | "를"
            | "에"
            | "에서"
            | "에게"
            | "부터"
            | "까지"
            | "의"
            | "보다"
            | "처럼"
            | "에도"
            | "에는"
            | "만"
            | "와"
            | "과"
            | "야"
            | "로"
            | "으로"
    )
}

fn contrastively_rejected(lower: &str, start: usize, end: usize) -> bool {
    let prefix = lower[..start].chars().rev().take(24).collect::<String>();
    let prefix = prefix.chars().rev().collect::<String>();
    let suffix = lower[end..].chars().take(24).collect::<String>();
    prefix.trim_end().ends_with("not the")
        || prefix.trim_end().ends_with("not")
        || suffix.trim_start().starts_with("말고")
}

fn connector_left_boundary(lower: &str, start: usize, end: usize) -> usize {
    let segment = &lower[start..end];
    let stored_boundary = CONNECTORS
        .iter()
        .filter_map(|connector| {
            segment
                .rfind(connector)
                .map(|offset| start + offset + connector.len())
        })
        .max()
        .unwrap_or(start);
    let immediate_clause_boundary = [" and ", " 그리고 "]
        .iter()
        .filter_map(|connector| {
            let offset = segment.rfind(connector)?;
            segment[offset + connector.len()..]
                .trim()
                .is_empty()
                .then_some(start + offset + connector.len())
        })
        .max()
        .unwrap_or(start);
    stored_boundary.max(immediate_clause_boundary)
}

fn connector_right_boundary(lower: &str, start: usize, end: usize) -> usize {
    let segment = &lower[start..end];
    CONNECTORS
        .iter()
        .filter_map(|connector| segment.find(connector).map(|offset| start + offset))
        .min()
        .unwrap_or(end)
}

fn nearest_entity_ids(
    action: &ActionMatch,
    candidates: &[String],
    entities: &[NativeEntityIR],
) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|entity_id| {
            entities
                .iter()
                .find(|entity| &entity.entity_id == entity_id)
                .map(|entity| {
                    let distance = if entity.end_byte <= action.start {
                        action.start - entity.end_byte
                    } else {
                        entity.start_byte.saturating_sub(action.end)
                    };
                    (distance, entity_id.clone())
                })
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, entity_id)| vec![entity_id])
        .unwrap_or_default()
}

fn entities_are_coordinated(
    entity_ids: &[String],
    entities: &[NativeEntityIR],
    lower: &str,
) -> bool {
    entity_ids.windows(2).all(|pair| {
        let left = entities.iter().find(|entity| entity.entity_id == pair[0]);
        let right = entities.iter().find(|entity| entity.entity_id == pair[1]);
        left.zip(right).is_some_and(|(left, right)| {
            let between = &lower[left.end_byte..right.start_byte];
            between.contains(" and ")
                || between.contains(" together with ")
                || between.trim() == "와"
                || between.trim() == "과"
                || between.contains("와 ")
                || between.contains("과 ")
                || lower[..left.end_byte].ends_with('와')
                || lower[..left.end_byte].ends_with('과')
        })
    })
}

fn coordinated_entity_groups(entities: &[NativeEntityIR], lower: &str) -> Vec<Vec<String>> {
    let explicit = entities
        .iter()
        .filter(|entity| {
            !entity.rejected_by_contrast
                && entity.start_byte < entity.end_byte
                && entity.end_byte <= lower.len()
        })
        .collect::<Vec<_>>();
    let mut groups = Vec::<Vec<String>>::new();
    for pair in explicit.windows(2) {
        let ids = vec![pair[0].entity_id.clone(), pair[1].entity_id.clone()];
        if !entities_are_coordinated(&ids, entities, lower) {
            continue;
        }
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.last() == Some(&pair[0].entity_id))
        {
            group.push(pair[1].entity_id.clone());
        } else {
            groups.push(ids);
        }
    }
    groups
}

fn nearest_coordinated_group<'a>(
    action: &ActionMatch,
    groups: &'a [Vec<String>],
    entities: &[NativeEntityIR],
) -> Option<&'a Vec<String>> {
    groups.iter().min_by_key(|group| {
        group
            .iter()
            .filter_map(|entity_id| {
                entities
                    .iter()
                    .find(|entity| &entity.entity_id == entity_id)
            })
            .map(|entity| {
                if entity.end_byte <= action.start {
                    action.start - entity.end_byte
                } else {
                    entity.start_byte.saturating_sub(action.end)
                }
            })
            .min()
            .unwrap_or(usize::MAX)
    })
}

fn set_member_selection(text: &str) -> Option<bool> {
    let tokens = token_spans(text);
    let mut mentions = Vec::<(usize, usize, bool)>::new();
    for (index, token) in tokens.iter().enumerate() {
        let normalized = token.surface.to_lowercase();
        let english_selection = match normalized.as_str() {
            "former" => Some(false),
            "latter" => Some(true),
            "first" => tokens
                .get(index + 1)
                .is_some_and(|next| english_referential_head(&next.surface))
                .then_some(false),
            "second" => tokens
                .get(index + 1)
                .is_some_and(|next| english_referential_head(&next.surface))
                .then_some(true),
            _ => None,
        };
        if let Some(select_last) = english_selection {
            let end = tokens.get(index + 1).map_or(token.end, |next| next.end);
            mentions.push((token.start, end, select_last));
        }

        let korean_selection = if normalized == "전자" || normalized.starts_with("앞의") {
            Some(false)
        } else if normalized == "후자" || normalized.starts_with("뒤의") {
            Some(true)
        } else if normalized == "앞"
            && tokens
                .get(index + 1)
                .is_some_and(|next| korean_referential_head(&next.surface))
        {
            Some(false)
        } else if normalized == "뒤"
            && tokens
                .get(index + 1)
                .is_some_and(|next| korean_referential_head(&next.surface))
        {
            Some(true)
        } else {
            None
        };
        if let Some(select_last) = korean_selection {
            let end = tokens.get(index + 1).map_or(token.end, |next| next.end);
            mentions.push((token.start, end, select_last));
        }
    }
    mentions.sort_unstable_by_key(|(start, _, _)| *start);
    mentions
        .into_iter()
        .find(|(start, end, _)| !ordinal_mention_is_rejected(text, *start, *end))
        .map(|(_, _, select_last)| select_last)
}

fn english_referential_head(surface: &str) -> bool {
    matches!(surface.to_lowercase().as_str(), "item" | "one" | "target")
}

fn korean_referential_head(surface: &str) -> bool {
    ["항목", "대상", "것"]
        .iter()
        .any(|head| surface.starts_with(head))
}

fn ascii_word_position(text: &str, word: &str) -> Option<(usize, usize)> {
    text.match_indices(word)
        .map(|(start, matched)| (start, start + matched.len()))
        .find(|(start, end)| ascii_word_boundaries(text, *start, *end))
}

fn contains_deictic_marker(text: &str) -> bool {
    [
        "that one",
        "this one",
        "that operation",
        "that action",
        "그걸",
        "그것",
        "그거",
        "이걸",
        "이것",
        "이거",
        "그 작업",
        "그럼",
        "there",
        "anything",
        "아무것도",
        "아무 것도",
    ]
    .iter()
    .any(|marker| text.contains(marker))
        || text
            .match_indices("it")
            .any(|(start, marker)| ascii_word_boundaries(text, start, start + marker.len()))
        || text
            .trim_end_matches(|character: char| !character.is_alphanumeric())
            .split_whitespace()
            .next_back()
            .is_some_and(|word| word == "that" || word == "this")
}

fn contains_plural_context_reference_marker(text: &str) -> bool {
    [
        "either of them",
        "both of them",
        "both items",
        "those two",
        "them",
        "둘 다",
        "둘을",
        "그 둘",
        "두 개 모두",
        "그것들",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn contains_context_reference_marker(text: &str) -> bool {
    contains_deictic_marker(text)
        || contains_plural_context_reference_marker(text)
        || set_member_selection(text).is_some()
        || continuation_constraint_marker(text).is_some()
        || operation_ellipsis_marker(text).is_some()
        || contains_causal_marker(text)
        || ["keep ", "continue ", "계속"]
            .iter()
            .any(|marker| text.contains(marker))
        || ordinal_goal_index(text).is_some()
}

fn contains_causal_marker(text: &str) -> bool {
    ["why", "cause", "reason", "원인", "왜"]
        .iter()
        .any(|marker| text.contains(marker))
}

fn multi_goal_relation_licensed(text: &str) -> bool {
    let alternatives = [
        " either ",
        "either ",
        " or ",
        " 또는 ",
        " 혹은 ",
        " 아니면 ",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    !alternatives
        && [
            " and ",
            " then ",
            " but ",
            " together with ",
            "하고 ",
            "하되 ",
            "말고 ",
            "한 뒤",
            "후에 ",
            "먼저 ",
        ]
        .iter()
        .any(|marker| text.contains(marker))
}

fn requests_verified_result(text: &str) -> bool {
    let verification = text.contains("verified")
        || text.contains("verification")
        || text.contains("검증")
        || text.contains("확인된")
        || text.contains("확정된");
    let result = text.contains("actual result")
        || text.contains("verified result")
        || text.contains("result was")
        || text.contains(" result")
        || text.contains("실제 결과")
        || text.contains("결과가")
        || text.contains("결과는")
        || text.contains("결과");
    let completion_or_outcome = [
        "finished",
        "finish",
        "finsh",
        "done",
        "completed",
        "complete",
        "repaired",
        "fixed",
        "succeed",
        "failed",
        "끝났",
        "끝난",
        "완료",
        "수리했",
        "수리된",
        "고쳤",
        "된 거",
        "됐",
        "성공",
        "실패",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    let interrogative = text.trim_end().ends_with('?')
        || [
            "right",
            "맞지",
            "거지",
            "거야",
            "건 있어",
            "what you found",
            "찾아낸 게",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    let asks_about_completed_action = ["did you ", "did we ", "did it "]
        .iter()
        .any(|prefix| text.trim_start().starts_with(prefix))
        && ACTION_LEXEMES
            .iter()
            .any(|lexeme| text.contains(lexeme.surface));
    let plan_result_boundary_query = interrogative
        && ["plan", "planned", "계획", "예정", "조사안"]
            .iter()
            .any(|marker| text.contains(marker))
        && ["result", "outcome", "결과", "성과"]
            .iter()
            .any(|marker| text.contains(marker));
    let evidence_status_directive = evidence_status_answer_directive(text);
    let observed_findings_query = asks_for_observed_findings(text);
    (verification && (result || completion_or_outcome || interrogative))
        || (interrogative && completion_or_outcome)
        || asks_for_findings_over_plan(text)
        || asks_about_completed_action
        || asks_what_is_certain(text)
        || asks_outcome_alternative(text)
        || plan_result_boundary_query
        || evidence_status_directive
        || observed_findings_query
}

fn asks_for_observed_findings(text: &str) -> bool {
    let interrogative = text.trim_end().ends_with('?')
        || [
            "what did you find",
            "what was found",
            "뭘 찾았",
            "뭐가 나왔",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    let evidence_or_finding = [
        "finding",
        "findings",
        "observed",
        "confirmed",
        "evidence",
        "관찰 결과",
        "관찰된",
        "확인된",
        "확정된",
        "발견",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    interrogative && evidence_or_finding
}

fn asks_for_result_contents(text: &str) -> bool {
    let interrogative = text.trim_end().ends_with('?')
        || ["tell me", "말해", "알려"]
            .iter()
            .any(|marker| text.contains(marker));
    let content_question = [
        "what result",
        "which result",
        "what repair result",
        "what verified result",
        "결과는 뭐",
        "결과가 뭐",
        "결과는 무엇",
        "결과가 무엇",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    interrogative && content_question
}

fn evidence_status_answer_directive(text: &str) -> bool {
    let trimmed = text.trim_start();
    let response_constructor = [
        "say ",
        "state ",
        "separate ",
        "distinguish ",
        "tell me ",
        "answer ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
        || [" say ", " state ", " separate ", " distinguish "]
            .iter()
            .any(|marker| text.contains(marker))
        || ["답해", "말해", "구분해", "분리해"]
            .iter()
            .any(|ending| text.trim_end_matches(['.', '!', '?']).ends_with(ending));
    let evidence_axis = [
        "evidence",
        "verified fact",
        "established",
        "suspected",
        "proof",
        "근거",
        "검증된 사실",
        "확립",
        "의심",
        "추정",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    response_constructor && evidence_axis
}

fn asks_for_findings_over_plan(text: &str) -> bool {
    let contrasts_plan = (text.contains("plan")
        && ["not ", "instead", "rather than"]
            .iter()
            .any(|marker| text.contains(marker)))
        || text.contains("only a plan")
        || text.contains("only the plan")
        || (text.contains("계획")
            && ["말고", "대신", "뿐", "뿐인"]
                .iter()
                .any(|marker| text.contains(marker)));
    let asks_for_evidence = [
        "finding",
        "found",
        "observed",
        "confirmed",
        "evidence",
        "찾아낸",
        "발견",
        "관찰",
        "확정",
        "증거",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    contrasts_plan && asks_for_evidence
}

fn asks_what_is_certain(text: &str) -> bool {
    [
        "what is certain",
        "what's certain",
        "what do we know for certain",
        "확실한 건 뭐",
        "확실한 게 뭐",
        "뭐가 확실",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn asks_outcome_alternative(text: &str) -> bool {
    let interrogative = text.trim_end().ends_with('?')
        || ["right", "맞지", "거지", "거야"]
            .iter()
            .any(|marker| text.contains(marker));
    let success = ["succeed", "success", "성공"]
        .iter()
        .any(|marker| text.contains(marker));
    let failure = ["fail", "failure", "실패"]
        .iter()
        .any(|marker| text.contains(marker));
    let existential_set = ["anything", "any result", "뭐라도", "건 있어", "게 있어"]
        .iter()
        .any(|marker| text.contains(marker))
        && [
            "repair", "fix", "complete", "finish", "result", "수리", "고친", "완료", "결과",
        ]
        .iter()
        .any(|marker| text.contains(marker));
    interrogative && ((success && failure) || existential_set)
}

fn asks_verification_status(text: &str) -> bool {
    let verification = text.contains("verified")
        || text.contains("verification")
        || text.contains("검증")
        || text.contains("확인된 결과");
    let interrogative = text.trim_end().ends_with('?')
        || ["right", "맞지", "거지", "거야"]
            .iter()
            .any(|marker| text.contains(marker));
    verification && interrogative
}

fn reports_completed_user_action(text: &str) -> bool {
    let first_person = [
        "i just ",
        "i already ",
        "i have ",
        "i've ",
        "내가 방금",
        "내가 이미",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    let completed_action = [
        "repaired",
        "fixed",
        "checked",
        "investigated",
        "수리했",
        "고쳤",
        "확인했",
        "조사했",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    first_person && completed_action && !text.trim_end().ends_with('?')
}

fn underspecified_problem_disclosure(text: &str, entities: &[NativeEntityIR]) -> bool {
    if entities.is_empty()
        || text.trim_end().ends_with('?')
        || REPORTED_SCOPE_MARKERS
            .iter()
            .any(|marker| text.contains(marker))
    {
        return false;
    }
    let problem_state = [
        "acting up",
        "seems wrong",
        "seems odd",
        "is broken",
        "malfunction",
        "이상하네",
        "이상해",
        "문제가 있",
        "고장",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    let personal_affect = [
        "tired",
        "exhausted",
        "drained",
        "frustrated",
        "지친",
        "진이 빠",
        "힘들",
        "답답",
        "짜증",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    problem_state && !personal_affect
}

fn reports_competing_outcomes(text: &str) -> bool {
    let reports = [" says ", " said ", " reports ", " reported ", "다고 했"]
        .iter()
        .any(|marker| text.contains(marker));
    let positive = ["succeed", "success", "passed", "성공", "통과"]
        .iter()
        .any(|marker| text.contains(marker));
    let negative = ["fail", "failure", "실패"]
        .iter()
        .any(|marker| text.contains(marker));
    reports && positive && negative
}

fn reference_marker(text: &str) -> &'static str {
    if operation_ellipsis_marker(text).is_some() {
        "SAME_OPERATION"
    } else if ordinal_goal_index(text).is_some() {
        "EVENT_ORDINAL"
    } else if text.contains("former") || text.contains("전자") {
        "former"
    } else if text.contains("후자") {
        "후자"
    } else if text.contains("second item") || text.contains("두 번째 것") {
        "SECOND_SET_MEMBER"
    } else if text.contains("latter") {
        "latter"
    } else if text.contains("그걸") || text.contains("그것") {
        "그것"
    } else if text.contains("that one") {
        "that one"
    } else if text.contains("why") || text.contains("왜") || text.contains("원인") {
        "CAUSE_REFERENCE"
    } else {
        "DISCOURSE_FOCUS"
    }
}

fn build_relations(events: &[NativeEventIR], lower: &str) -> Vec<NativeRelationEdgeIR> {
    events
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let between = &lower[pair[0].end_byte..pair[1].start_byte];
            let kind = if between.contains("but") || between.contains("하되") {
                NativeDiscourseRelationIR::Contrast
            } else if between.contains("말고") {
                NativeDiscourseRelationIR::Correction
            } else if pair[1].scope == NativeEventScopeIR::Conditional {
                NativeDiscourseRelationIR::Condition
            } else if contains_causal_marker(between) {
                NativeDiscourseRelationIR::Cause
            } else {
                NativeDiscourseRelationIR::Sequence
            };
            NativeRelationEdgeIR {
                relation_id: format!("NR{:03}", index + 1),
                kind,
                source_id: pair[0].event_id.clone(),
                target_id: pair[1].event_id.clone(),
                evidence: vec![format!("CONNECTIVE:{}", between.trim())],
            }
        })
        .collect()
}

fn selected_semantic_sha256(turn: &NativeTurnIR) -> String {
    let payload = turn
        .selected_live_goals
        .iter()
        .map(|goal| {
            (
                goal.canonical_predicate.as_str(),
                goal.intent,
                goal.subject_concepts.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    sha256_serialized(&payload)
}

fn native_turn_sha256(turn: &NativeTurnIR) -> String {
    sha256_serialized(&(
        turn.schema.as_str(),
        turn.language,
        turn.source_sha256.as_str(),
        turn.entities.as_slice(),
        turn.events.as_slice(),
        turn.relations.as_slice(),
        turn.reference_bindings.as_slice(),
        turn.selected_live_goals.as_slice(),
        turn.response_goal,
        turn.response_mode,
        turn.unresolved.as_slice(),
        turn.selected_semantic_sha256.as_str(),
        turn.semantic_authority,
        turn.language_can_execute,
    ))
}

fn sha256_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn sha256_serialized(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("native language IR must serialize");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(source: &str) -> NativeGoalIR {
        let turn = NativeLanguageCircuit.analyze(source);
        assert!(turn.validate_for_source(source), "{turn:#?}");
        turn.authoritative_single_live_goal()
            .expect("one live goal")
            .clone()
    }

    #[test]
    fn condition_scope_excludes_deferred_action_in_both_languages() {
        let english = selected(
            "Inspect Alder log now, but repair the Bramble queue only if the cache is stale",
        );
        let korean =
            selected("Alder 로그는 지금 조사하되 캐시가 오래됐을 때만 Bramble 큐를 수리해");
        assert_eq!(english.intent, PlanIntentIR::Investigate);
        assert_eq!(korean.intent, PlanIntentIR::Investigate);
        assert!(english.subject.contains("Alder"));
        assert!(korean.subject.contains("Alder"));
        assert!(!english.subject.contains("Bramble"));
        assert!(!korean.subject.contains("Bramble"));
        assert_eq!(english.subject_concepts, korean.subject_concepts);
    }

    #[test]
    fn concessive_prohibition_preserves_live_explanation() {
        for source in [
            "Even if Cobalt cache failed, do not delete it; explain why it failed",
            "Cobalt 캐시가 실패했더라도 그걸 삭제하지 말고 왜 실패했는지 설명해",
        ] {
            let turn = NativeLanguageCircuit.analyze(source);
            let goal = turn
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("live explanation: {turn:#?}"));
            assert_eq!(goal.intent, PlanIntentIR::Explain, "{turn:#?}");
            assert!(goal.subject.contains("Cobalt"));
            assert!(turn
                .events
                .iter()
                .any(|event| event.scope == NativeEventScopeIR::Prohibited));
        }
    }

    #[test]
    fn contrastive_retarget_and_causal_ellipsis_bind_discourse_focus() {
        let correction = selected("Not the Delta index—the Elm queue. Repair that one");
        assert_eq!(correction.intent, PlanIntentIR::Repair);
        assert!(correction.subject.contains("Elm"));
        assert!(!correction.subject.contains("Delta"));

        let causal = selected("The Fennel service keeps timing out. Find out why");
        assert_eq!(causal.intent, PlanIntentIR::Investigate);
        assert!(causal.subject.contains("Fennel"));
    }

    #[test]
    fn demonstrative_determiner_is_not_promoted_as_the_action_target() {
        let turn =
            NativeLanguageCircuit.analyze("That answer was too long. Explain it again concisely");
        let goal = turn
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("response correction: {turn:#?}"));
        assert_eq!(goal.intent, PlanIntentIR::Explain);
        assert!(goal.subject.to_lowercase().contains("answer"), "{turn:#?}");
        assert!(!goal.subject.to_lowercase().contains("that"), "{turn:#?}");
        assert!(turn.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::ExplicitPriorTheme
                || binding.kind == NativeReferenceKindIR::IntraTurnAnaphora
        }));
    }

    #[test]
    fn dialogue_context_supplies_elliptical_operation_and_ordinal_target() {
        let context = NativeDialogueContextIR {
            active_goals: vec![
                NativeContextGoalIR {
                    goal_id: "GOAL-1".to_string(),
                    intent: PlanIntentIR::Investigate,
                    canonical_predicate: "INVESTIGATE".to_string(),
                    subject: "Kestrel worker".to_string(),
                    introduced_turn: 1,
                    discourse_focused: false,
                    operation_replayable: true,
                },
                NativeContextGoalIR {
                    goal_id: "GOAL-2".to_string(),
                    intent: PlanIntentIR::Investigate,
                    canonical_predicate: "INVESTIGATE".to_string(),
                    subject: "Quartz queue".to_string(),
                    introduced_turn: 2,
                    discourse_focused: false,
                    operation_replayable: true,
                },
            ],
            ..NativeDialogueContextIR::default()
        };
        let mut focused_context = context.clone();
        focused_context.active_goals[0].discourse_focused = true;
        let resumed = NativeLanguageCircuit.analyze_with_context("Review it", &focused_context);
        let resumed_goal = resumed
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("focused context: {resumed:#?}"));
        assert_eq!(resumed_goal.subject, "Kestrel worker");

        for source in [
            "Do the same to the Linen queue",
            "For the Linen queue, do the same",
        ] {
            let repeated = NativeLanguageCircuit.analyze_with_context(source, &context);
            let repeated_goal = repeated
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("inherited operation for {source}: {repeated:#?}"));
            assert_eq!(repeated_goal.intent, PlanIntentIR::Investigate);
            assert!(
                repeated_goal.subject.contains("Linen"),
                "{source}: {repeated:#?}"
            );
            assert!(
                !repeated_goal.subject.contains("For"),
                "preposition must not become a semantic entity: {repeated:#?}"
            );
            assert!(repeated.reference_bindings.iter().any(|binding| {
                binding.kind == NativeReferenceKindIR::OperationEllipsis
                    && binding.inherited_goal_id.as_deref() == Some("GOAL-2")
            }));
        }

        let mut ordinal_context_with_newer_focus = context.clone();
        ordinal_context_with_newer_focus.active_entities = vec![NativeContextEntityIR {
            referent_id: "ENTITY-LATEST".to_string(),
            surface: "Quartz queue".to_string(),
            introduced_turn: 2,
            last_mentioned_turn: 2,
        }];
        let ordinal = NativeLanguageCircuit.analyze_with_context(
            "Go back to the first issue and explain why it failed",
            &ordinal_context_with_newer_focus,
        );
        let ordinal_goal = ordinal
            .authoritative_single_live_goal()
            .expect("ordinal target");
        assert_eq!(ordinal_goal.intent, PlanIntentIR::Explain);
        assert!(ordinal_goal.subject.contains("Kestrel"));
        assert!(ordinal.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::EventOrdinal
                && binding.inherited_goal_id.as_deref() == Some("GOAL-1")
        }));

        let verified = NativeLanguageCircuit.analyze_with_context(
            "그 주장은 필요 없어. 실제 결과가 검증됐는지 알려줘",
            &context,
        );
        assert_eq!(
            verified.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
        assert!(verified.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::VerifiedResultTarget
                && binding.inherited_goal_id.as_deref() == Some("GOAL-2")
        }));
    }

    #[test]
    fn discourse_entities_bind_unique_ellipsis_but_preserve_plural_ambiguity() {
        let unique = NativeDialogueContextIR {
            active_entities: vec![NativeContextEntityIR {
                referent_id: "TREF-1".to_string(),
                surface: "Aster cache".to_string(),
                introduced_turn: 1,
                last_mentioned_turn: 1,
            }],
            ..NativeDialogueContextIR::default()
        };
        let causal = NativeLanguageCircuit.analyze_with_context("Yes, find out why.", &unique);
        let goal = causal
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("causal binding: {causal:#?}"));
        assert_eq!(goal.subject.to_lowercase(), "aster cache");

        let ambiguous = NativeDialogueContextIR {
            active_entities: vec![
                NativeContextEntityIR {
                    referent_id: "TREF-1".to_string(),
                    surface: "Aster cache".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
                NativeContextEntityIR {
                    referent_id: "TREF-2".to_string(),
                    surface: "Dune queue".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
            ],
            ..NativeDialogueContextIR::default()
        };
        let repair = NativeLanguageCircuit.analyze_with_context("Fix that.", &ambiguous);
        assert_eq!(repair.response_goal, NativeResponseGoalIR::AskClarification);
        assert!(repair
            .unresolved
            .iter()
            .any(|reason| reason == "AMBIGUOUS_DIALOGUE_CONTEXT_ENTITY"));

        let ordinal =
            NativeLanguageCircuit.analyze_with_context("Explain the second one.", &ambiguous);
        let goal = ordinal
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("ordinal binding: {ordinal:#?}"));
        assert_eq!(goal.subject.to_lowercase(), "dune queue");

        let outcome =
            NativeLanguageCircuit.analyze_with_context("So did it succeed or fail?", &ambiguous);
        assert_eq!(
            outcome.response_mode,
            NativeResponseModeIR::OutcomeAlternativeQuery
        );
        assert!(outcome.unresolved.is_empty(), "{outcome:#?}");
    }

    #[test]
    fn correction_constructions_reject_old_ordinals_and_keep_one_live_goal() {
        assert_eq!(
            ordinal_goal_index("wait, i meant the second one, not the first"),
            Some(2)
        );
        assert_eq!(
            ordinal_goal_index("잠깐, 첫 번째가 아니라 두 번째를 말한 거야"),
            Some(2)
        );

        let context = NativeDialogueContextIR {
            active_goals: vec![NativeContextGoalIR {
                goal_id: "PRIOR-EXPLAIN".to_string(),
                intent: PlanIntentIR::Explain,
                canonical_predicate: "EXPLAIN".to_string(),
                subject: "Aster cache".to_string(),
                introduced_turn: 2,
                discourse_focused: true,
                operation_replayable: true,
            }],
            active_entities: vec![
                NativeContextEntityIR {
                    referent_id: "TREF-1".to_string(),
                    surface: "Aster cache".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
                NativeContextEntityIR {
                    referent_id: "TREF-2".to_string(),
                    surface: "Dune queue".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
            ],
            ..NativeDialogueContextIR::default()
        };
        for text in [
            "Wait, I meant the second one, not the first.",
            "잠깐, 첫 번째가 아니라 두 번째를 말한 거야.",
        ] {
            let corrected = NativeLanguageCircuit.analyze_with_context(text, &context);
            let goal = corrected
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("correction failed: {corrected:#?}"));
            assert_eq!(goal.intent, PlanIntentIR::Explain, "{corrected:#?}");
            assert_eq!(goal.subject.to_lowercase(), "dune queue", "{corrected:#?}");
            assert_eq!(corrected.reference_bindings.len(), 1, "{corrected:#?}");
            assert_eq!(
                corrected.reference_bindings[0].kind,
                NativeReferenceKindIR::ContrastiveRetarget,
                "{corrected:#?}"
            );
        }

        let replacement = NativeLanguageCircuit
            .analyze_with_context("아니, 수리 말고 첫 번째 원인만 설명해.", &context);
        let goal = replacement
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("replacement failed: {replacement:#?}"));
        assert_eq!(goal.intent, PlanIntentIR::Explain, "{replacement:#?}");
        assert!(replacement.events.iter().any(|event| {
            event.intent == PlanIntentIR::Repair && event.scope == NativeEventScopeIR::Prohibited
        }));
        assert_eq!(
            replacement.reference_bindings[0].kind,
            NativeReferenceKindIR::ContrastiveRetarget,
            "{replacement:#?}"
        );
    }

    #[test]
    fn acknowledged_prohibition_retargets_context_without_authorizing_action() {
        let context = NativeDialogueContextIR {
            active_entities: vec![NativeContextEntityIR {
                referent_id: "TREF-1".to_string(),
                surface: "Alder cache".to_string(),
                introduced_turn: 1,
                last_mentioned_turn: 3,
            }],
            ..NativeDialogueContextIR::default()
        };
        for source in [
            "Right, do not touch the Alder cache.",
            "맞아, Aster 캐시는 건드리지 마.",
        ] {
            let turn = NativeLanguageCircuit.analyze_with_context(source, &context);
            assert_eq!(turn.response_goal, NativeResponseGoalIR::Acknowledge);
            assert!(turn.selected_live_goals.is_empty(), "{turn:#?}");
            assert!(turn.events.iter().any(|event| {
                event.scope == NativeEventScopeIR::Prohibited
                    && !event.external_execution_authorized
            }));
            assert!(turn.reference_bindings.iter().any(|binding| {
                binding.kind == NativeReferenceKindIR::ContrastiveRetarget
                    && binding.source_surface == "ACKNOWLEDGED_CONSTRAINT"
            }));
        }
    }

    #[test]
    fn korean_particle_bearing_prohibition_remains_a_constraint() {
        let context = NativeDialogueContextIR {
            active_entities: vec![NativeContextEntityIR {
                referent_id: "TREF-CACHE".to_string(),
                surface: "Aster 캐시".to_string(),
                introduced_turn: 1,
                last_mentioned_turn: 2,
            }],
            ..NativeDialogueContextIR::default()
        };
        let source = "대신 아무것도 바꾸지는 마.";
        let turn = NativeLanguageCircuit.analyze_with_context(source, &context);
        assert!(turn.validate_for_source(source), "{turn:#?}");
        assert_eq!(turn.response_goal, NativeResponseGoalIR::Acknowledge);
        assert!(turn.selected_live_goals.is_empty(), "{turn:#?}");
        assert!(turn.events.iter().any(|event| {
            event.canonical_predicate == "MODIFY"
                && event.scope == NativeEventScopeIR::Prohibited
                && event.theme_entity_ids.len() == 1
        }));
    }

    #[test]
    fn plural_constraint_binds_the_context_set_without_guessing() {
        let context = NativeDialogueContextIR {
            active_entities: vec![
                NativeContextEntityIR {
                    referent_id: "TREF-CACHE".to_string(),
                    surface: "Alder cache".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
                NativeContextEntityIR {
                    referent_id: "TREF-QUEUE".to_string(),
                    surface: "Birch queue".to_string(),
                    introduced_turn: 1,
                    last_mentioned_turn: 1,
                },
            ],
            ..NativeDialogueContextIR::default()
        };
        let source = "And do not modify either of them.";
        let turn = NativeLanguageCircuit.analyze_with_context(source, &context);
        assert!(turn.validate_for_source(source), "{turn:#?}");
        assert!(turn.unresolved.is_empty(), "{turn:#?}");
        assert!(turn.events.iter().any(|event| {
            event.scope == NativeEventScopeIR::Prohibited && event.theme_entity_ids.len() == 2
        }));
        assert_eq!(
            turn.reference_bindings
                .iter()
                .filter(|binding| binding.kind == NativeReferenceKindIR::PluralContextSet)
                .count(),
            2
        );
    }

    #[test]
    fn problem_disclosure_and_action_report_choose_response_goals_without_execution() {
        let disclosure = NativeLanguageCircuit.analyze("The Alder cache is acting up again...");
        assert_eq!(
            disclosure.response_goal,
            NativeResponseGoalIR::AskClarification
        );
        assert!(disclosure.selected_live_goals.is_empty());
        assert!(!disclosure.language_can_execute);

        let report = NativeLanguageCircuit.analyze("I just repaired it myself.");
        assert_eq!(
            report.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
        assert!(report.selected_live_goals.is_empty());
        assert!(!report.language_can_execute);
    }

    #[test]
    fn plan_result_questions_and_evidence_status_directives_select_answer_mode() {
        for surface in [
            "We only have a Saffron queue plan, not an outcome, correct?",
            "If evidence is absent, state that no fact is established for the Topaz worker.",
            "Separate verified facts about the Umber cache from suspected claims.",
            "검증 근거가 없으면 Violet 서비스에서 확립된 사실이 없다고 답해.",
        ] {
            let turn = NativeLanguageCircuit.analyze(surface);
            assert_eq!(
                turn.response_goal,
                NativeResponseGoalIR::AnswerVerifiedResult,
                "surface={surface}; turn={turn:#?}"
            );
            assert!(matches!(
                turn.response_mode,
                NativeResponseModeIR::EvidenceResultQuery
                    | NativeResponseModeIR::VerificationStatusQuery
            ));
            assert!(turn.selected_live_goals.is_empty(), "surface={surface}");
            assert!(turn.events.iter().all(|event| {
                !event.semantic_authority && !event.external_execution_authorized
            }));
        }
    }

    #[test]
    fn response_modes_generalize_across_result_report_and_evidence_constructions() {
        for (source, expected) in [
            (
                "I already checked the Rowan queue.",
                NativeResponseModeIR::ReportedOutcome,
            ),
            (
                "What do we know for certain?",
                NativeResponseModeIR::SourceCertaintyQuery,
            ),
            (
                "Was it a success or a failure?",
                NativeResponseModeIR::OutcomeAlternativeQuery,
            ),
            (
                "Has that been verified?",
                NativeResponseModeIR::VerificationStatusQuery,
            ),
            (
                "Give me the findings instead of the plan.",
                NativeResponseModeIR::EvidenceResultQuery,
            ),
            (
                "계획 대신 발견한 증거를 말해 줘.",
                NativeResponseModeIR::EvidenceResultQuery,
            ),
            (
                "성공한 건지 실패한 건지 알려줘?",
                NativeResponseModeIR::OutcomeAlternativeQuery,
            ),
        ] {
            let turn = NativeLanguageCircuit.analyze(source);
            assert!(turn.validate_for_source(source), "{source}: {turn:#?}");
            assert_eq!(
                turn.response_goal,
                NativeResponseGoalIR::AnswerVerifiedResult,
                "{source}: {turn:#?}"
            );
            assert_eq!(turn.response_mode, expected, "{source}: {turn:#?}");
        }
    }

    #[test]
    fn construction_knowledge_transfers_across_order_and_paraphrase() {
        for source in [
            "If Dune is not healthy, fix Ember; review the Flint report now",
            "Inspect the Flint report now; unless Dune is healthy, repair the Ember worker",
            "원인을 확인해 줘. Navy 서비스가 계속 시간 초과돼",
        ] {
            let goal = selected(source);
            assert_eq!(
                goal.intent,
                PlanIntentIR::Investigate,
                "{source}: {goal:#?}"
            );
            assert!(
                goal.subject.contains("Flint") || goal.subject.contains("Navy"),
                "{source}: {goal:#?}"
            );
        }
    }

    #[test]
    fn korean_postposed_condition_excludes_only_its_consequent() {
        let source = "Flint 보고서는 지금 조사하고 Dune 서비스가 정상이 아니면 Ember 워커를 수리해";
        let turn = NativeLanguageCircuit.analyze(source);
        let goal = turn
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("postposed condition: {turn:#?}"));
        assert_eq!(goal.intent, PlanIntentIR::Investigate);
        assert!(goal.subject.contains("Flint"), "{turn:#?}");
        assert!(turn.events.iter().any(|event| {
            event.intent == PlanIntentIR::Repair && event.scope == NativeEventScopeIR::Conditional
        }));
    }

    #[test]
    fn coordinated_set_survives_member_selection_and_clause_order() {
        for source in [
            "Review the Rose cache together with the Sienna queue, then fix just the second item",
            "Repair only the latter, but first inspect the Rose cache and the Sienna queue",
            "Rose 캐시와 Sienna 큐를 함께 확인하고 두 번째 것만 고쳐",
        ] {
            let turn = NativeLanguageCircuit.analyze(source);
            let goals = turn
                .authoritative_live_goals()
                .unwrap_or_else(|| panic!("coordinated set: {turn:#?}"));
            assert_eq!(goals.len(), 2, "{turn:#?}");
            let investigate = goals
                .iter()
                .find(|goal| goal.intent == PlanIntentIR::Investigate)
                .expect("investigation");
            let repair = goals
                .iter()
                .find(|goal| goal.intent == PlanIntentIR::Repair)
                .expect("repair");
            assert!(investigate.subject.contains("Rose"), "{turn:#?}");
            assert!(investigate.subject.contains("Sienna"), "{turn:#?}");
            assert!(!repair.subject.contains("Rose"), "{turn:#?}");
            assert!(repair.subject.contains("Sienna"), "{turn:#?}");
        }
    }

    #[test]
    fn ordered_pair_and_cross_turn_correction_bind_the_intended_theme() {
        let scoped = NativeLanguageCircuit.analyze(
            "the server is slow and the worker is healthy. inspect the former but never delete the latter",
        );
        let goal = scoped
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("ordered scoped goal: {scoped:#?}"));
        assert_eq!(goal.intent, PlanIntentIR::Investigate);
        assert!(
            goal.subject.to_lowercase().contains("server"),
            "{scoped:#?}"
        );
        assert!(!goal.subject.to_lowercase().contains("worker"));
        assert!(scoped.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::SetMember
                && binding.source_surface.to_lowercase().contains("former")
        }));

        let context = NativeDialogueContextIR {
            active_goals: vec![NativeContextGoalIR {
                goal_id: "GOAL-CACHE".to_string(),
                intent: PlanIntentIR::Execute,
                canonical_predicate: "DELETE".to_string(),
                subject: "cache".to_string(),
                introduced_turn: 1,
                discourse_focused: false,
                operation_replayable: true,
            }],
            ..NativeDialogueContextIR::default()
        };
        let corrected = NativeLanguageCircuit
            .analyze_with_context("No, review it rather than remove it", &context);
        let corrected_goal = corrected
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("corrected goal: {corrected:#?}"));
        assert_eq!(corrected_goal.intent, PlanIntentIR::Investigate);
        assert_eq!(corrected_goal.subject.to_lowercase(), "cache");
        assert!(corrected.events.iter().any(|event| {
            event.intent == PlanIntentIR::Execute && event.scope == NativeEventScopeIR::Prohibited
        }));
        assert!(corrected.reference_bindings.iter().any(|binding| {
            binding.kind == NativeReferenceKindIR::ExplicitPriorTheme
                && binding.inherited_goal_id.as_deref() == Some("GOAL-CACHE")
        }));
    }

    #[test]
    fn compositional_request_fills_an_empty_native_goal_without_overwrite() {
        for source in [
            "Arrange the Birch queue before the Cedar cache.",
            "Harbor 워커 복구 절차를 설계해.",
        ] {
            let analysis =
                crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(source);
            let mut turn = NativeLanguageCircuit.analyze(source);
            assert!(turn.selected_live_goals.is_empty(), "source={source}");
            assert!(turn.absorb_selected_compositional_goals(source, &analysis));
            assert_eq!(turn.response_goal, NativeResponseGoalIR::PlanActions);
            assert!(turn.authoritative_live_goals().is_some());
            assert!(turn.validate_for_source(source), "{turn:#?}");
            assert!(turn.events.iter().all(|event| {
                !event.semantic_authority && !event.external_execution_authorized
            }));
        }

        let source = "Inspect the Larch log.";
        let analysis =
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(source);
        let mut existing = NativeLanguageCircuit.analyze(source);
        let before = existing.circuit_sha256.clone();
        assert!(!existing.absorb_selected_compositional_goals(source, &analysis));
        assert_eq!(existing.circuit_sha256, before);

        let answer_source = "Tell me whether the actual Quartz result was verified.";
        let answer_analysis =
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(answer_source);
        let mut answer = NativeLanguageCircuit.analyze(answer_source);
        assert_eq!(
            answer.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
        let answer_before = answer.circuit_sha256.clone();
        assert!(!answer.absorb_selected_compositional_goals(answer_source, &answer_analysis));
        assert_eq!(answer.circuit_sha256, answer_before);
        assert_eq!(
            answer.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
    }

    #[test]
    fn partial_conditional_lattice_is_never_collapsed_into_a_native_goal() {
        let source =
            "Inspect the cache and if the cache is stale or damaged and invalid, repair the cache.";
        let analysis =
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(source);
        let mut turn = NativeLanguageCircuit.analyze(source);
        let before = turn.circuit_sha256.clone();
        assert!(!turn.absorb_selected_compositional_goals(source, &analysis));
        assert_eq!(turn.circuit_sha256, before);
    }

    #[test]
    fn typed_response_boundary_refinement_is_fill_only_and_semantically_invariant() {
        let source = "Is an outcome established for the Quartz relay?";
        let mut turn = NativeLanguageCircuit.analyze(source);
        assert_eq!(turn.response_goal, NativeResponseGoalIR::Acknowledge);
        let selected_semantic_before = turn.selected_semantic_sha256.clone();
        let events_before = turn.events.clone();
        assert!(turn.refine_response_boundary(source, NativeResponseModeIR::EvidenceResultQuery));
        assert_eq!(
            turn.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
        assert_eq!(
            turn.response_mode,
            NativeResponseModeIR::EvidenceResultQuery
        );
        assert_eq!(turn.selected_semantic_sha256, selected_semantic_before);
        assert_eq!(turn.events, events_before);
        assert!(turn.selected_live_goals.is_empty());
        assert!(turn.validate_for_source(source));

        let planned_source = "Inspect the Quartz relay.";
        let mut planned = NativeLanguageCircuit.analyze(planned_source);
        let circuit_before = planned.circuit_sha256.clone();
        assert!(!planned
            .refine_response_boundary(planned_source, NativeResponseModeIR::EvidenceResultQuery));
        assert_eq!(planned.response_goal, NativeResponseGoalIR::PlanActions);
        assert_eq!(planned.circuit_sha256, circuit_before);
    }
}
