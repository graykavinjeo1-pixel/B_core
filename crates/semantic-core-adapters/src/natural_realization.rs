//! Typed, evidence-bound natural surface realization.
//!
//! This layer selects wording from already typed dialogue and reasoning state.
//! It cannot add semantic authority, advance execution state, or turn a
//! language report into verified evidence.

use std::collections::BTreeSet;

use dockable_semantic_core::{
    PlanIntentIR, SemanticPlanBundleIR, SemanticPlanEventIR, SemanticPlanGoalIR,
    SemanticPlanProjectionIR, SemanticPlanRelationKindIR,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::{
    ActionExecutionStatusIR, ActionPlanStatusIR, ActionReportedStatusIR, ActionSetQuantifierIR,
    ActionSetTruthIR, ActionStateAnalysisIR, ActionStateLedgerIR, ActionStatePredicateIR,
};
use crate::conditional_guard::ConditionalGuardEvaluationIR;
use crate::conversation::{
    DialogueDirectiveIR, DialogueDirectiveKindIR, DiscourseEventIR, DiscourseFunctionIR,
    DiscourseGroupUpdateIR, DiscourseGroupUpdateOperationIR, TopicTransitionIR,
};
use crate::definition_grounding::DefinitionGroundingIR;
use crate::discourse_qa::DiscourseAnswerIR;
use crate::discourse_relations::DialogueRelationAnswerIR;
use crate::generative_language::{
    generate_action_set_answer_from_knowledge, generate_affect_support_from_knowledge,
    generate_clarification_from_knowledge, generate_conditional_guard_from_knowledge,
    generate_continuation_gate_followup_from_knowledge, generate_continuation_gate_from_knowledge,
    generate_definition_grounding_from_knowledge, generate_dialogue_relation_answer_from_knowledge,
    generate_dialogue_response_from_knowledge, generate_discourse_answer_from_knowledge,
    generate_discourse_group_update_from_knowledge, generate_inform_acknowledgement_from_knowledge,
    generate_interaction_boundary_from_knowledge, generate_lifecycle_status_from_knowledge,
    generate_plan_exclusion_from_knowledge, generate_plan_interpretation_from_knowledge,
    generate_plan_preview_from_knowledge, generate_plan_preview_from_knowledge_with_directive,
    generate_temporal_answer_from_knowledge, generate_topic_transition_from_knowledge,
    generate_user_feedback_from_knowledge, GenerationActionSetPredicateIR,
    GenerationActionSetQuantifierIR, GenerationActionSetTruthIR, GenerationAffectKindIR,
    GenerationClarificationKindIR, GenerationContinuationGateFollowupIR,
    GenerationDialogueResponseKindIR, GenerationDiscourseGroupUpdateKindIR,
    GenerationLifecycleClaimIR, GenerationPlanInterpretationKindIR, GenerationUserFeedbackKindIR,
    GenerativeLanguageIR,
};
use crate::language_knowledge::LanguageCodeIR;
use crate::native_language_circuit::{NativeResponseModeIR, NativeTurnIR};
use crate::nonliteral::{NonliteralAnalysisIR, ReadingSelectionIR};
use crate::plan_result_boundary::{
    PlanResultBoundaryIR, PlanResultQueryFocusIR, ResultAvailabilityIR,
};
use crate::pragmatic_memory::PendingContinuationGateIR;
use crate::pragmatics::{
    ContinuationDecisionGateIR, GoalCommitmentIR, IllocutionaryCommitmentGraphIR,
    InferredPragmaticGoalIR, UserFeedbackIR, UserFeedbackKindIR,
};
use crate::temporal::TemporalAnswerIR;

pub const NATURAL_REALIZATION_SCHEMA: &str = "B_CORE_NATURAL_REALIZATION_IR_5";
pub const NATURAL_REALIZATION_COVERAGE_SCHEMA: &str = "B_CORE_NATURAL_REALIZATION_COVERAGE_IR_1";

// A bounded dialogue-relation answer may carry up to 48 evidence clauses plus
// typed path and safety boundaries. Retain every bounded clause.
const MAX_SENTENCES: usize = 64;
const MAX_SOURCE_REFS: usize = 32;
const MAX_REALIZED_CHARS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalResponseActIR {
    PlanPreview,
    InterpretationBoundary,
    PlanResultStatus,
    ResultAbsence,
    ActionState,
    InformAcknowledgement,
    UserFeedback,
    AffectSupport,
    ClarificationRequest,
    TopicTransition,
    DefinitionGrounding,
    DiscourseGroupUpdate,
    ConditionalGuard,
    TemporalAnswer,
    DialogueRelationAnswer,
    DiscourseAnswer,
    ContinuationGate,
    InteractionBoundary,
    SocialBackchannel,
    HoldFloor,
}

/// Origin of a response obligation.  Modules contribute candidates; this
/// central vocabulary is the only place that defines their precedence.  The
/// order in which modules run or candidates are appended is therefore unable
/// to change the selected primary response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalResponseSourceIR {
    NativeAnswer,
    DialogueDirective,
    ConditionalGuard,
    NativePlan,
    NativeAcknowledgement,
    StandaloneAffect,
    NonliteralInterpretation,
    Clarification,
    PlanResult,
    DefinitionGrounding,
    DiscourseGroupUpdate,
    ActionState,
    TemporalAnswer,
    DialogueRelationAnswer,
    DiscourseAnswer,
    TopicTransition,
    ContinuationGate,
    InteractionBoundary,
    ResultReference,
    HoldFloor,
    SocialBackchannel,
    UserFeedback,
    Affect,
    Inform,
    GroundedPlan,
    Fallback,
}

impl NaturalResponseSourceIR {
    fn precedence(self) -> u16 {
        match self {
            Self::NativeAnswer => 240,
            Self::DialogueDirective => 235,
            Self::ConditionalGuard => 230,
            Self::NativePlan => 220,
            Self::NativeAcknowledgement => 210,
            Self::NonliteralInterpretation => 205,
            Self::StandaloneAffect => 200,
            // A required ambiguity is a fail-closed boundary, not another
            // stylistic proposal. It must outrank every answer/plan/
            // acknowledgement candidate so a lower module cannot silently
            // turn an unresolved reference into a claim or action.
            Self::Clarification => 250,
            Self::PlanResult => 180,
            Self::DefinitionGrounding => 170,
            Self::DiscourseGroupUpdate => 160,
            Self::ActionState => 150,
            Self::TemporalAnswer => 140,
            Self::DialogueRelationAnswer => 130,
            Self::DiscourseAnswer => 120,
            Self::TopicTransition => 110,
            Self::ContinuationGate => 100,
            Self::InteractionBoundary => 90,
            Self::ResultReference => 80,
            Self::HoldFloor => 70,
            Self::SocialBackchannel => 60,
            Self::UserFeedback => 50,
            Self::Affect => 40,
            Self::Inform => 30,
            Self::GroundedPlan => 20,
            Self::Fallback => 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalResponseCandidateIR {
    pub source: NaturalResponseSourceIR,
    pub response_act: NaturalResponseActIR,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

impl NaturalResponseCandidateIR {
    pub fn new(
        source: NaturalResponseSourceIR,
        response_act: NaturalResponseActIR,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            source,
            response_act,
            evidence: vec![evidence.into()],
            semantic_authority: false,
            external_action_executed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalResponseArbitrationIR {
    pub candidates: Vec<NaturalResponseCandidateIR>,
    pub selected_source: NaturalResponseSourceIR,
    pub selected_act: NaturalResponseActIR,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub arbitration_sha256: String,
}

impl NaturalResponseArbitrationIR {
    pub fn validate(&self) -> bool {
        let sources = self
            .candidates
            .iter()
            .map(|candidate| candidate.source)
            .collect::<BTreeSet<_>>();
        let selected = self
            .candidates
            .iter()
            .max_by_key(|candidate| (candidate.source.precedence(), candidate.source));
        !self.candidates.is_empty()
            && self.candidates.len() <= 32
            && sources.len() == self.candidates.len()
            && self
                .candidates
                .windows(2)
                .all(|window| window[0].source < window[1].source)
            && self.candidates.iter().all(|candidate| {
                !candidate.evidence.is_empty()
                    && candidate
                        .evidence
                        .iter()
                        .all(|evidence| !evidence.trim().is_empty())
                    && !candidate.semantic_authority
                    && !candidate.external_action_executed
            })
            && selected.is_some_and(|candidate| {
                candidate.source == self.selected_source
                    && candidate.response_act == self.selected_act
            })
            && !self.semantic_authority
            && !self.language_can_execute
            && self.arbitration_sha256 == natural_response_arbitration_sha256(self)
    }
}

pub fn arbitrate_natural_response(
    mut candidates: Vec<NaturalResponseCandidateIR>,
) -> NaturalResponseArbitrationIR {
    candidates.sort_by_key(|candidate| candidate.source);
    candidates.dedup_by_key(|candidate| candidate.source);
    let selected = candidates
        .iter()
        .max_by_key(|candidate| (candidate.source.precedence(), candidate.source))
        .expect("response arbitration requires at least one candidate");
    let selected_source = selected.source;
    let selected_act = selected.response_act;
    let mut arbitration = NaturalResponseArbitrationIR {
        candidates,
        selected_source,
        selected_act,
        semantic_authority: false,
        language_can_execute: false,
        arbitration_sha256: String::new(),
    };
    arbitration.arbitration_sha256 = natural_response_arbitration_sha256(&arbitration);
    debug_assert!(arbitration.validate());
    arbitration
}

pub fn natural_response_arbitration_sha256(arbitration: &NaturalResponseArbitrationIR) -> String {
    let mut canonical = arbitration.clone();
    canonical.arbitration_sha256.clear();
    content_sha256(&canonical)
}

/// The role of one response move in a composed answer.  A turn may need to
/// preserve a relational or discourse move without allowing it to replace the
/// task-bearing move.  The role is language independent; Korean/English
/// wording is selected only after this plan has been fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalResponseMoveRoleIR {
    RelationalSupport,
    DiscourseBridge,
    PrimaryTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalResponseMoveIR {
    pub move_index: usize,
    pub role: NaturalResponseMoveRoleIR,
    pub response_act: NaturalResponseActIR,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalResponseFormatIR {
    #[default]
    Plain,
    Bullets,
    Numbered,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalResponsePlanIR {
    pub moves: Vec<NaturalResponseMoveIR>,
    pub primary_move_index: usize,
    #[serde(default)]
    pub response_format: NaturalResponseFormatIR,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
}

impl NaturalResponsePlanIR {
    pub fn validate(&self) -> bool {
        if self.moves.is_empty()
            || self.moves.len() > 4
            || self.primary_move_index >= self.moves.len()
            || self.semantic_authority
            || self.language_can_execute
        {
            return false;
        }
        let unique_acts = self
            .moves
            .iter()
            .map(|response_move| response_move.response_act)
            .collect::<BTreeSet<_>>();
        unique_acts.len() == self.moves.len()
            && self.moves.iter().enumerate().all(|(index, response_move)| {
                response_move.move_index == index
                    && !response_move.evidence.is_empty()
                    && !response_move.semantic_authority
                    && !response_move.external_action_executed
                    && ((index == self.primary_move_index
                        && response_move.role == NaturalResponseMoveRoleIR::PrimaryTask)
                        || (index != self.primary_move_index
                            && response_move.role != NaturalResponseMoveRoleIR::PrimaryTask))
            })
    }

    pub fn primary_act(&self) -> NaturalResponseActIR {
        self.moves[self.primary_move_index].response_act
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalSentenceFunctionIR {
    Acknowledge,
    DescribePlan,
    StateEvidenceBoundary,
    AnswerStatus,
    RequestClarification,
    ManageDialogue,
    SupportAffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalRealizationPathIR {
    Generative,
    Hybrid,
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalSentenceIR {
    pub sentence_index: usize,
    pub function: NaturalSentenceFunctionIR,
    pub surface: String,
    pub source_refs: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NaturalRealizationObligationKindIR {
    ResponseMove,
    SelectedPlanEvent,
    ProhibitedPlanEvent,
    SelectedEventRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalRealizationObligationIR {
    pub obligation_id: String,
    pub kind: NaturalRealizationObligationKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_move_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_act: Option<NaturalResponseActIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_relation_id: Option<String>,
    pub supporting_generation_trace_sha256s: Vec<String>,
    pub satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalRealizationCoverageIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_goal_sha256: Option<String>,
    pub obligations: Vec<NaturalRealizationObligationIR>,
    pub omitted_required_obligations: usize,
    pub orphan_generation_traces: usize,
    pub coverage_sha256: String,
}

impl NaturalRealizationCoverageIR {
    fn validate_internal(
        &self,
        response_plan: &NaturalResponsePlanIR,
        generation_traces: &[GenerativeLanguageIR],
    ) -> bool {
        let obligation_ids = self
            .obligations
            .iter()
            .map(|obligation| obligation.obligation_id.as_str())
            .collect::<BTreeSet<_>>();
        let trace_hashes = generation_traces
            .iter()
            .map(|trace| trace.generation_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let response_move_obligations = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::ResponseMove
            })
            .collect::<Vec<_>>();
        let bound_response_trace_hashes = response_move_obligations
            .iter()
            .flat_map(|obligation| {
                obligation
                    .supporting_generation_trace_sha256s
                    .iter()
                    .map(String::as_str)
            })
            .collect::<Vec<_>>();
        let bound_response_trace_set = bound_response_trace_hashes
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        self.schema == NATURAL_REALIZATION_COVERAGE_SCHEMA
            && !self.obligations.is_empty()
            && self.obligations.len() <= 256
            && obligation_ids.len() == self.obligations.len()
            && self.omitted_required_obligations == 0
            && self.orphan_generation_traces == 0
            && response_move_obligations.len() == response_plan.moves.len()
            && response_move_obligations
                .iter()
                .filter_map(|obligation| obligation.response_move_index)
                .collect::<BTreeSet<_>>()
                .len()
                == response_plan.moves.len()
            && response_move_obligations.iter().all(|obligation| {
                obligation
                    .response_move_index
                    .and_then(|index| response_plan.moves.get(index))
                    .is_some_and(|response_move| {
                        obligation.response_act == Some(response_move.response_act)
                    })
                    && obligation.semantic_event_id.is_none()
                    && obligation.semantic_relation_id.is_none()
            })
            && bound_response_trace_hashes.len() == generation_traces.len()
            && bound_response_trace_set == trace_hashes
            && self.obligations.iter().all(|obligation| {
                !obligation.obligation_id.trim().is_empty()
                    && obligation.satisfied
                    && !obligation.supporting_generation_trace_sha256s.is_empty()
                    && obligation
                        .supporting_generation_trace_sha256s
                        .iter()
                        .all(|hash| trace_hashes.contains(hash.as_str()))
                    && match obligation.kind {
                        NaturalRealizationObligationKindIR::ResponseMove => {
                            obligation.response_move_index.is_some()
                                && obligation.response_act.is_some()
                                && obligation.semantic_event_id.is_none()
                                && obligation.semantic_relation_id.is_none()
                        }
                        NaturalRealizationObligationKindIR::SelectedPlanEvent
                        | NaturalRealizationObligationKindIR::ProhibitedPlanEvent => {
                            obligation.response_move_index.is_none()
                                && obligation.response_act.is_none()
                                && obligation
                                    .semantic_event_id
                                    .as_ref()
                                    .is_some_and(|event_id| !event_id.trim().is_empty())
                                && obligation.semantic_relation_id.is_none()
                        }
                        NaturalRealizationObligationKindIR::SelectedEventRelation => {
                            obligation.response_move_index.is_none()
                                && obligation.response_act.is_none()
                                && obligation.semantic_event_id.is_none()
                                && obligation
                                    .semantic_relation_id
                                    .as_ref()
                                    .is_some_and(|relation_id| !relation_id.trim().is_empty())
                                && obligation.supporting_generation_trace_sha256s.len() == 2
                        }
                    }
            })
            && self.coverage_sha256 == natural_realization_coverage_sha256(self)
    }

    pub fn validate_against(
        &self,
        response_plan: &NaturalResponsePlanIR,
        generation_traces: &[GenerativeLanguageIR],
        semantic_goal: Option<&SemanticPlanGoalIR>,
    ) -> bool {
        if !self.validate_internal(response_plan, generation_traces) {
            return false;
        }
        let selected_obligation_ids = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::SelectedPlanEvent
            })
            .filter_map(|obligation| obligation.semantic_event_id.as_deref())
            .collect::<BTreeSet<_>>();
        let prohibited_obligation_ids = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::ProhibitedPlanEvent
            })
            .filter_map(|obligation| obligation.semantic_event_id.as_deref())
            .collect::<BTreeSet<_>>();
        let relation_obligation_ids = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::SelectedEventRelation
            })
            .filter_map(|obligation| obligation.semantic_relation_id.as_deref())
            .collect::<BTreeSet<_>>();
        let Some(goal) = semantic_goal else {
            return self.semantic_goal_sha256.is_none()
                && selected_obligation_ids.is_empty()
                && prohibited_obligation_ids.is_empty()
                && relation_obligation_ids.is_empty();
        };
        let selected_ids = goal
            .selected_live_event_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let prohibited_ids = goal
            .events
            .iter()
            .filter(|event| event.projection == SemanticPlanProjectionIR::Prohibited)
            .map(|event| event.event_id.as_str())
            .collect::<BTreeSet<_>>();
        let required_relation_ids = goal
            .relations
            .iter()
            .filter(|relation| {
                relation_requires_realization_coverage(relation.relation)
                    && selected_ids.contains(relation.source_event_id.as_str())
                    && selected_ids.contains(relation.target_event_id.as_str())
            })
            .map(|relation| relation.relation_id.as_str())
            .collect::<BTreeSet<_>>();
        let selected_event_bindings_valid = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::SelectedPlanEvent
            })
            .all(|obligation| {
                let Some(event_id) = obligation.semantic_event_id.as_deref() else {
                    return false;
                };
                obligation.supporting_generation_trace_sha256s.len() == 1
                    && trace_for_hash(
                        generation_traces,
                        &obligation.supporting_generation_trace_sha256s[0],
                    )
                    .is_some_and(|trace| {
                        trace_has_semantic_event_grounding(trace, event_id, &goal.semantic_sha256)
                    })
            });
        let prohibited_event_bindings_valid = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::ProhibitedPlanEvent
            })
            .all(|obligation| {
                let Some(event_id) = obligation.semantic_event_id.as_deref() else {
                    return false;
                };
                let grounding = format!("SEMANTIC_PLAN_PROHIBITED_EVENT:{event_id}");
                obligation.supporting_generation_trace_sha256s.len() == 1
                    && trace_for_hash(
                        generation_traces,
                        &obligation.supporting_generation_trace_sha256s[0],
                    )
                    .is_some_and(|trace| trace_has_grounding(trace, &grounding))
            });
        let relation_bindings_valid = self
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == NaturalRealizationObligationKindIR::SelectedEventRelation
            })
            .all(|obligation| {
                let Some(relation_id) = obligation.semantic_relation_id.as_deref() else {
                    return false;
                };
                let Some(relation) = goal
                    .relations
                    .iter()
                    .find(|relation| relation.relation_id == relation_id)
                else {
                    return false;
                };
                let source_index = generation_traces.iter().position(|trace| {
                    trace_has_semantic_event_grounding(
                        trace,
                        &relation.source_event_id,
                        &goal.semantic_sha256,
                    )
                });
                let target_index = generation_traces.iter().position(|trace| {
                    trace_has_semantic_event_grounding(
                        trace,
                        &relation.target_event_id,
                        &goal.semantic_sha256,
                    )
                });
                let (Some(source_index), Some(target_index)) = (source_index, target_index) else {
                    return false;
                };
                let expected_hashes = [
                    generation_traces[source_index].generation_sha256.as_str(),
                    generation_traces[target_index].generation_sha256.as_str(),
                ]
                .into_iter()
                .collect::<BTreeSet<_>>();
                let bound_hashes = obligation
                    .supporting_generation_trace_sha256s
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                expected_hashes == bound_hashes
                    && match relation.relation {
                        SemanticPlanRelationKindIR::Sequence
                        | SemanticPlanRelationKindIR::TemporalBefore => source_index < target_index,
                        SemanticPlanRelationKindIR::Coordination => true,
                        _ => false,
                    }
            });
        self.semantic_goal_sha256.as_deref() == Some(goal.semantic_sha256.as_str())
            && selected_obligation_ids == selected_ids
            && prohibited_obligation_ids == prohibited_ids
            && relation_obligation_ids == required_relation_ids
            && selected_event_bindings_valid
            && prohibited_event_bindings_valid
            && relation_bindings_valid
    }
}

pub fn natural_realization_coverage_sha256(coverage: &NaturalRealizationCoverageIR) -> String {
    let mut canonical = coverage.clone();
    canonical.coverage_sha256.clear();
    content_sha256(&canonical)
}

impl NaturalSentenceIR {
    fn validate(&self) -> bool {
        let refs = self.source_refs.iter().collect::<BTreeSet<_>>();
        !self.surface.trim().is_empty()
            && self.surface.chars().count() <= MAX_REALIZED_CHARS
            && !self.source_refs.is_empty()
            && self.source_refs.len() <= MAX_SOURCE_REFS
            && refs.len() == self.source_refs.len()
            && self
                .source_refs
                .iter()
                .all(|source| !source.trim().is_empty() && source.len() <= 384)
            && !self.semantic_authority
            && !self.external_action_executed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalRealizationIR {
    pub schema: String,
    pub response_arbitration: NaturalResponseArbitrationIR,
    pub response_plan: NaturalResponsePlanIR,
    /// Compatibility projection for callers that have not yet migrated to the
    /// ordered response plan.  It is always equal to `response_plan.primary_act()`.
    pub response_act: NaturalResponseActIR,
    pub language: LanguageCodeIR,
    pub sentences: Vec<NaturalSentenceIR>,
    pub realized_text: String,
    pub realization_path: NaturalRealizationPathIR,
    pub generation_traces: Vec<GenerativeLanguageIR>,
    pub coverage: NaturalRealizationCoverageIR,
    pub stage_overwrite_count: usize,
    pub unsupported_claims: usize,
    pub empty_promises: usize,
    pub internal_ir_leaks: usize,
    pub violations: Vec<String>,
    pub faithful: bool,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub external_action_executed: bool,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub network_calls: usize,
    pub realization_sha256: String,
}

impl NaturalRealizationIR {
    pub fn validate(&self) -> bool {
        let indices = self
            .sentences
            .iter()
            .map(|sentence| sentence.sentence_index)
            .collect::<BTreeSet<_>>();
        let generated_sentences = self
            .generation_traces
            .iter()
            .flat_map(|trace| split_sentences(&trace.morphology.realized_text))
            .collect::<Vec<_>>();
        let generated_surface = render_response_surface(
            self.response_plan.response_format,
            self.language,
            &generated_sentences,
        );
        self.schema == NATURAL_REALIZATION_SCHEMA
            && self.response_arbitration.validate()
            && self.response_plan.validate()
            && self.response_arbitration.selected_act == self.response_plan.primary_act()
            && self.response_act == self.response_plan.primary_act()
            && !self.realized_text.trim().is_empty()
            && self.realized_text.chars().count() <= MAX_REALIZED_CHARS
            && !self.sentences.is_empty()
            && self.sentences.len() <= MAX_SENTENCES
            && indices.len() == self.sentences.len()
            && self
                .sentences
                .iter()
                .enumerate()
                .all(|(index, sentence)| sentence.sentence_index == index && sentence.validate())
            && self
                .sentences
                .iter()
                .all(|sentence| self.realized_text.contains(sentence.surface.trim()))
            && self.generation_traces.iter().all(|trace| {
                trace.validate()
                    && trace.morphology.language == self.language
                    && split_sentences(&trace.morphology.realized_text)
                        .iter()
                        .all(|surface| self.realized_text.contains(surface.trim()))
                    && self.sentences.iter().any(|sentence| {
                        sentence
                            .source_refs
                            .contains(&format!("GENERATIVE_LANGUAGE:{}", trace.generation_sha256))
                    })
            })
            && self
                .coverage
                .validate_internal(&self.response_plan, &self.generation_traces)
            && match self.realization_path {
                NaturalRealizationPathIR::Generative => {
                    !self.generation_traces.is_empty() && self.realized_text == generated_surface
                }
                NaturalRealizationPathIR::Hybrid => !self.generation_traces.is_empty(),
                NaturalRealizationPathIR::Legacy => self.generation_traces.is_empty(),
            }
            && self.stage_overwrite_count == 0
            && self.unsupported_claims == 0
            && self.empty_promises == 0
            && self.internal_ir_leaks == 0
            && self.violations.is_empty()
            && self.faithful
            && !self.semantic_authority
            && !self.language_can_execute
            && !self.external_action_executed
            && self.external_llm_calls == 0
            && self.local_teacher_calls == 0
            && self.network_calls == 0
            && self.realization_sha256 == natural_realization_sha256(self)
    }

    pub fn validate_output(
        &self,
        language: LanguageCodeIR,
        text: &str,
        unsupported: usize,
    ) -> bool {
        self.validate()
            && self.language == language
            && self.realized_text == text
            && self.unsupported_claims == unsupported
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ContinuationGateRealizationSourceIR<'a> {
    Initial(&'a ContinuationDecisionGateIR),
    PendingDecision(&'a PendingContinuationGateIR),
    ProxyEvidence(&'a PendingContinuationGateIR),
}

#[derive(Clone, Copy)]
pub(crate) struct NaturalRealizationSources<'a> {
    pub response_arbitration: &'a NaturalResponseArbitrationIR,
    pub language: LanguageCodeIR,
    pub raw_input: &'a str,
    pub native_language_circuit: &'a NativeTurnIR,
    pub semantic_goal: Option<&'a SemanticPlanGoalIR>,
    pub semantic_plan_bundle: Option<&'a SemanticPlanBundleIR>,
    pub inferred_goal: Option<&'a InferredPragmaticGoalIR>,
    pub nonliteral_analysis: &'a NonliteralAnalysisIR,
    pub plan_result_boundary: &'a PlanResultBoundaryIR,
    pub action_analysis: &'a ActionStateAnalysisIR,
    pub action_ledger: &'a ActionStateLedgerIR,
    pub continuation_gate: Option<ContinuationGateRealizationSourceIR<'a>>,
    pub user_feedback: Option<&'a UserFeedbackIR>,
    pub discourse_group_update: Option<&'a DiscourseGroupUpdateIR>,
    pub discourse_events: &'a [DiscourseEventIR],
    pub topic_transition: Option<&'a TopicTransitionIR>,
    pub clarification_kind: Option<GenerationClarificationKindIR>,
    pub clarification_detail: Option<&'a str>,
    pub definition_grounding: &'a DefinitionGroundingIR,
    pub guard_evaluations: &'a [ConditionalGuardEvaluationIR],
    pub illocutionary_commitments: &'a IllocutionaryCommitmentGraphIR,
    pub withdrawn_goal_ids: &'a [String],
    pub withdrawn_deferred_ids: &'a [String],
    pub discourse_answer: Option<&'a DiscourseAnswerIR>,
    pub dialogue_relation_answer: Option<&'a DialogueRelationAnswerIR>,
    pub temporal_answer: Option<&'a TemporalAnswerIR>,
    pub source_refs: &'a [String],
    pub dialogue_directives: &'a [DialogueDirectiveIR],
    pub unsupported_claims: usize,
}

#[derive(Debug, Clone, Copy)]
struct ResponseMoveTraceRangeIR {
    move_index: usize,
    trace_start: usize,
    trace_end: usize,
}

/// Compose independent response obligations before wording is chosen.  This
/// replaces winner-takes-all arbitration for the common cases where a user
/// both expresses affect and requests work, or restores a topic and requests
/// work in the same turn.  Auxiliary moves can add surface material, but they
/// cannot replace, alter, or authorize the primary task move.
fn compose_response_plan(sources: &NaturalRealizationSources<'_>) -> NaturalResponsePlanIR {
    let response_length_directive = active_response_length_directive(sources);
    let directive_ref = response_length_directive
        .map(|directive| format!("DIALOGUE_DIRECTIVE:{}", directive.directive_id));
    let mut plan = compose_response_plan_from_signals(
        sources.response_arbitration.selected_act,
        sources.user_feedback.is_some(),
        affect_surface_present(sources.raw_input),
        sources
            .topic_transition
            .is_some_and(|transition| transition.applied),
        response_length_directive.is_some_and(|directive| directive.value_key == "CONCISE"),
        directive_ref.as_deref(),
    );
    if let Some(directive) = active_response_format_directive(sources) {
        plan.response_format = response_format(directive);
        plan.moves[plan.primary_move_index].evidence.push(format!(
            "DIALOGUE_DIRECTIVE_FORMAT:{}:{}",
            directive.value_key, directive.directive_id
        ));
    }
    debug_assert!(plan.validate());
    plan
}

fn active_response_length_directive<'a>(
    sources: &'a NaturalRealizationSources<'_>,
) -> Option<&'a DialogueDirectiveIR> {
    sources.dialogue_directives.iter().find(|directive| {
        directive.is_active()
            && directive.kind == DialogueDirectiveKindIR::ResponseLength
            && directive.target_key == "ASSISTANT_RESPONSE"
    })
}

fn active_response_format_directive<'a>(
    sources: &'a NaturalRealizationSources<'_>,
) -> Option<&'a DialogueDirectiveIR> {
    sources.dialogue_directives.iter().find(|directive| {
        directive.is_active()
            && directive.kind == DialogueDirectiveKindIR::ResponseFormat
            && directive.target_key == "ASSISTANT_RESPONSE"
    })
}

fn response_format(directive: &DialogueDirectiveIR) -> NaturalResponseFormatIR {
    match directive.value_key.as_str() {
        "BULLETS" => NaturalResponseFormatIR::Bullets,
        "NUMBERED" => NaturalResponseFormatIR::Numbered,
        "TABLE" => NaturalResponseFormatIR::Table,
        _ => NaturalResponseFormatIR::Plain,
    }
}

fn render_response_surface(
    format: NaturalResponseFormatIR,
    language: LanguageCodeIR,
    sentences: &[String],
) -> String {
    match format {
        NaturalResponseFormatIR::Plain => sentences.join(" "),
        NaturalResponseFormatIR::Bullets => sentences
            .iter()
            .map(|sentence| format!("- {}", sentence.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        NaturalResponseFormatIR::Numbered => sentences
            .iter()
            .enumerate()
            .map(|(index, sentence)| format!("{}. {}", index + 1, sentence.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        NaturalResponseFormatIR::Table => {
            let header = match language {
                LanguageCodeIR::Korean => "| 번호 | 내용 |\n|---:|---|",
                _ => "| No. | Content |\n|---:|---|",
            };
            let rows = sentences
                .iter()
                .enumerate()
                .map(|(index, sentence)| format!("| {} | {} |", index + 1, sentence.trim()))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{header}\n{rows}")
        }
    }
}

fn compose_response_plan_from_signals(
    primary_act: NaturalResponseActIR,
    user_feedback_present: bool,
    affect_present: bool,
    topic_transition_applied: bool,
    concise: bool,
    directive_ref: Option<&str>,
) -> NaturalResponsePlanIR {
    let mut moves = Vec::new();
    let primary_is_task_bearing = !matches!(
        primary_act,
        NaturalResponseActIR::InformAcknowledgement
            | NaturalResponseActIR::AffectSupport
            | NaturalResponseActIR::InterpretationBoundary
            | NaturalResponseActIR::TopicTransition
            | NaturalResponseActIR::SocialBackchannel
            | NaturalResponseActIR::HoldFloor
    );
    if primary_is_task_bearing
        && primary_act != NaturalResponseActIR::UserFeedback
        && user_feedback_present
    {
        moves.push(NaturalResponseMoveIR {
            move_index: moves.len(),
            role: NaturalResponseMoveRoleIR::RelationalSupport,
            response_act: NaturalResponseActIR::UserFeedback,
            evidence: vec![
                "TYPED_USER_FEEDBACK".to_string(),
                "AUXILIARY_MOVE_CANNOT_REPLACE_PRIMARY_TASK".to_string(),
            ],
            semantic_authority: false,
            external_action_executed: false,
        });
    } else if !concise
        && primary_is_task_bearing
        && primary_act != NaturalResponseActIR::AffectSupport
        && affect_present
    {
        moves.push(NaturalResponseMoveIR {
            move_index: moves.len(),
            role: NaturalResponseMoveRoleIR::RelationalSupport,
            response_act: NaturalResponseActIR::AffectSupport,
            evidence: vec![
                "TYPED_AFFECT_SIGNAL".to_string(),
                "AUXILIARY_MOVE_CANNOT_REPLACE_PRIMARY_TASK".to_string(),
            ],
            semantic_authority: false,
            external_action_executed: false,
        });
    }
    if !concise
        && primary_is_task_bearing
        && primary_act != NaturalResponseActIR::TopicTransition
        && topic_transition_applied
    {
        moves.push(NaturalResponseMoveIR {
            move_index: moves.len(),
            role: NaturalResponseMoveRoleIR::DiscourseBridge,
            response_act: NaturalResponseActIR::TopicTransition,
            evidence: vec![
                "APPLIED_TOPIC_TRANSITION".to_string(),
                "DISCOURSE_BRIDGE_PRESERVES_PRIMARY_TASK".to_string(),
            ],
            semantic_authority: false,
            external_action_executed: false,
        });
    }
    let primary_move_index = moves.len();
    let mut primary_evidence = vec!["CENTRAL_RESPONSE_ARBITRATION_PRIMARY".to_string()];
    if let Some(directive_ref) = directive_ref {
        primary_evidence.push(directive_ref.to_string());
    }
    moves.push(NaturalResponseMoveIR {
        move_index: primary_move_index,
        role: NaturalResponseMoveRoleIR::PrimaryTask,
        response_act: primary_act,
        evidence: primary_evidence,
        semantic_authority: false,
        external_action_executed: false,
    });
    let plan = NaturalResponsePlanIR {
        moves,
        primary_move_index,
        response_format: NaturalResponseFormatIR::Plain,
        semantic_authority: false,
        language_can_execute: false,
    };
    debug_assert!(plan.validate());
    plan
}

pub(crate) fn build_natural_realization(
    sources: NaturalRealizationSources<'_>,
) -> NaturalRealizationIR {
    let response_plan = compose_response_plan(&sources);
    let primary_act = sources.response_arbitration.selected_act;
    let mut generation_traces = Vec::new();
    let mut move_trace_ranges = Vec::new();
    let mut move_texts = Vec::new();
    for response_move in response_plan
        .moves
        .iter()
        .filter(|response_move| response_move.role != NaturalResponseMoveRoleIR::PrimaryTask)
    {
        let trace_start = generation_traces.len();
        let generated = match response_move.response_act {
            NaturalResponseActIR::UserFeedback => {
                let feedback = sources
                    .user_feedback
                    .expect("a feedback support move must retain its typed source");
                generate_user_feedback_from_knowledge(
                    sources.language,
                    map_user_feedback_kind(feedback.kind),
                    &feedback.target_surface,
                    &feedback.evidence_clause_ids,
                )
                .expect("typed user-feedback realization knowledge must be complete")
            }
            NaturalResponseActIR::AffectSupport => generate_affect_support_from_knowledge(
                sources.language,
                affect_kind(sources.raw_input),
            )
            .expect("built-in affect realization knowledge must be complete"),
            NaturalResponseActIR::TopicTransition => {
                let transition = sources
                    .topic_transition
                    .filter(|transition| transition.applied)
                    .expect("a discourse bridge must retain an applied topic transition");
                generate_topic_transition_from_knowledge(sources.language, transition)
                    .expect("typed topic-transition realization knowledge must be complete")
            }
            _ => unreachable!("only composable auxiliary acts may precede the primary move"),
        };
        move_texts.push((
            response_move.response_act,
            generated.morphology.realized_text.clone(),
        ));
        generation_traces.push(generated);
        move_trace_ranges.push(ResponseMoveTraceRangeIR {
            move_index: response_move.move_index,
            trace_start,
            trace_end: generation_traces.len(),
        });
    }
    let primary_trace_start = generation_traces.len();
    let primary_text = match primary_act {
        NaturalResponseActIR::PlanPreview => {
            let generated = generate_plan_preview_response(&sources);
            let generated_text = generated
                .iter()
                .map(|trace| trace.morphology.realized_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            generation_traces.extend(generated);
            generated_text
        }
        NaturalResponseActIR::InterpretationBoundary => {
            let generated = generate_nonliteral_interpretation_response(&sources)
                .expect("an interpretation-boundary act must retain typed nonliteral evidence");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::PlanResultStatus | NaturalResponseActIR::ResultAbsence => {
            let generated = generate_plan_result_boundary(
                sources.language,
                primary_act,
                sources.plan_result_boundary,
                sources.action_ledger,
            )
            .unwrap_or_else(|| {
                vec![generate_lifecycle_status_from_knowledge(
                    sources.language,
                    native_evidence_subject(&sources),
                    &[
                        GenerationLifecycleClaimIR::ResultUnavailable,
                        GenerationLifecycleClaimIR::UntrustedEvidenceMention,
                    ],
                    &format!(
                        "NATIVE_EVIDENCE_BOUNDARY:{}",
                        sources.native_language_circuit.circuit_sha256
                    ),
                )
                .expect("the generic evidence boundary must have realization knowledge")]
            });
            let generated_text = generated
                .iter()
                .map(|trace| trace.morphology.realized_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            generation_traces.extend(generated);
            generated_text
        }
        NaturalResponseActIR::ActionState => {
            let generated = if sources.native_language_circuit.response_mode
                == NativeResponseModeIR::CompetingOutcomeReports
            {
                vec![generate_lifecycle_status_from_knowledge(
                    sources.language,
                    if sources.language == LanguageCodeIR::Korean {
                        "두 보고"
                    } else {
                        "the reports"
                    },
                    &[
                        GenerationLifecycleClaimIR::ConflictingReports,
                        GenerationLifecycleClaimIR::ReportsNotVerified,
                    ],
                    &format!(
                        "NATIVE_CONFLICTING_REPORTS:{}",
                        sources.native_language_circuit.circuit_sha256
                    ),
                )
                .expect("conflicting-report realization knowledge must be complete")]
            } else {
                generate_action_state_response(
                    sources.language,
                    sources.action_analysis,
                    sources.action_ledger,
                )
                .unwrap_or_else(|| {
                    vec![generate_lifecycle_status_from_knowledge(
                        sources.language,
                        native_evidence_subject(&sources),
                        &[
                            GenerationLifecycleClaimIR::UntrustedEvidenceMention,
                            GenerationLifecycleClaimIR::ExecutionStateUnchanged,
                        ],
                        &format!(
                            "NATIVE_REPORTED_STATE:{}",
                            sources.native_language_circuit.circuit_sha256
                        ),
                    )
                    .expect("the reported-state boundary must have realization knowledge")]
                })
            };
            let generated_text = generated
                .iter()
                .map(|trace| trace.morphology.realized_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            generation_traces.extend(generated);
            generated_text
        }
        NaturalResponseActIR::InformAcknowledgement => {
            let generated =
                generate_inform_acknowledgement_from_knowledge(sources.language, sources.raw_input)
                    .expect("built-in report/evidence realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::UserFeedback => {
            let feedback = sources
                .user_feedback
                .expect("a user-feedback response act must retain its typed feedback source");
            let generated = generate_user_feedback_from_knowledge(
                sources.language,
                map_user_feedback_kind(feedback.kind),
                &feedback.target_surface,
                &feedback.evidence_clause_ids,
            )
            .expect("typed user-feedback realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::AffectSupport => {
            let generated = generate_affect_support_from_knowledge(
                sources.language,
                affect_kind(sources.raw_input),
            )
            .expect("built-in affect realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::HoldFloor | NaturalResponseActIR::SocialBackchannel => {
            let response = dialogue_response_kind(primary_act, sources.discourse_events);
            let generated = generate_dialogue_response_from_knowledge(sources.language, response)
                .expect("built-in dialogue-management knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::ContinuationGate => {
            let generated = match sources
                .continuation_gate
                .expect("a continuation-gate response act must retain its typed source")
            {
                ContinuationGateRealizationSourceIR::Initial(gate) => {
                    generate_continuation_gate_from_knowledge(
                        sources.language,
                        &gate.current_task,
                        &gate.required_benefit,
                        &gate.supporting_clause_ids,
                    )
                }
                ContinuationGateRealizationSourceIR::PendingDecision(gate) => {
                    let refs = vec![
                        format!("PENDING_GATE:SOURCE_TURN:{}", gate.source_turn),
                        format!("PENDING_GATE:STATUS:{:?}", gate.status),
                    ];
                    generate_continuation_gate_followup_from_knowledge(
                        sources.language,
                        &gate.task,
                        &gate.required_benefit,
                        &refs,
                        GenerationContinuationGateFollowupIR::PendingDecision,
                    )
                }
                ContinuationGateRealizationSourceIR::ProxyEvidence(gate) => {
                    let refs = vec![
                        format!("PENDING_GATE:SOURCE_TURN:{}", gate.source_turn),
                        format!("PENDING_GATE:STATUS:{:?}", gate.status),
                    ];
                    generate_continuation_gate_followup_from_knowledge(
                        sources.language,
                        &gate.task,
                        &gate.required_benefit,
                        &refs,
                        GenerationContinuationGateFollowupIR::ProxyEvidence,
                    )
                }
            }
            .expect("typed continuation-gate realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::DiscourseGroupUpdate => {
            let update = sources
                .discourse_group_update
                .filter(|update| update.applied)
                .expect("a discourse-group response act must retain an applied typed update");
            let refs = vec![
                format!("DISCOURSE_GROUP_UPDATE:{}", update.update_sha256),
                format!("DISCOURSE_GROUP_REVISION:{}", update.revision),
            ];
            let generated = generate_discourse_group_update_from_knowledge(
                sources.language,
                map_discourse_group_update_kind(update.operation),
                update.after_member_keys.len(),
                &refs,
            )
            .expect("typed discourse-group realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::ClarificationRequest => {
            let generated = generate_clarification_from_knowledge(
                sources.language,
                sources
                    .clarification_kind
                    .unwrap_or(GenerationClarificationKindIR::MissingDetails),
                sources.clarification_detail,
                sources.source_refs,
            )
            .expect("typed clarification realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::DefinitionGrounding => {
            let generated = generate_definition_grounding_from_knowledge(
                sources.language,
                sources.definition_grounding,
                sources.source_refs,
            )
            .expect("a definition-grounding response act must retain typed grounding");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::ConditionalGuard => {
            let generated = sources
                .guard_evaluations
                .iter()
                .map(|evaluation| {
                    generate_conditional_guard_from_knowledge(
                        sources.language,
                        evaluation,
                        sources.source_refs,
                    )
                    .expect("a conditional-guard response act must retain valid typed evaluations")
                })
                .collect::<Vec<_>>();
            assert!(
                !generated.is_empty(),
                "a conditional-guard response act requires at least one typed evaluation"
            );
            let generated_text = generated
                .iter()
                .map(|trace| trace.morphology.realized_text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            generation_traces.extend(generated);
            generated_text
        }
        NaturalResponseActIR::InteractionBoundary => {
            let generated = generate_interaction_boundary_from_knowledge(
                sources.language,
                sources.illocutionary_commitments,
                sources.withdrawn_goal_ids,
                sources.withdrawn_deferred_ids,
                sources.source_refs,
            )
            .expect("an interaction-boundary act must retain typed illocutionary knowledge");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::DiscourseAnswer => {
            let generated = generate_discourse_answer_from_knowledge(
                sources.language,
                sources
                    .discourse_answer
                    .expect("a discourse-answer response act must retain its typed answer"),
                sources.source_refs,
            )
            .expect("typed discourse-answer realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::DialogueRelationAnswer => {
            let generated = generate_dialogue_relation_answer_from_knowledge(
                sources.language,
                sources
                    .dialogue_relation_answer
                    .expect("a dialogue-relation response act must retain its typed answer"),
                sources.source_refs,
            )
            .expect("typed dialogue-relation realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::TemporalAnswer => {
            let generated = generate_temporal_answer_from_knowledge(
                sources.language,
                sources
                    .temporal_answer
                    .expect("a temporal-answer response act must retain its typed answer"),
                sources.source_refs,
            )
            .expect("typed temporal-answer realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
        NaturalResponseActIR::TopicTransition => {
            let transition = sources
                .topic_transition
                .filter(|transition| transition.applied)
                .expect("a topic-transition response act must retain an applied transition");
            let generated = generate_topic_transition_from_knowledge(sources.language, transition)
                .expect("typed topic-transition realization knowledge must be complete");
            let generated_text = generated.morphology.realized_text.clone();
            generation_traces.push(generated);
            generated_text
        }
    };
    move_trace_ranges.push(ResponseMoveTraceRangeIR {
        move_index: response_plan.primary_move_index,
        trace_start: primary_trace_start,
        trace_end: generation_traces.len(),
    });
    move_texts.push((primary_act, primary_text));
    let mut source_refs = sources
        .source_refs
        .iter()
        .filter(|source| !source.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    source_refs.sort();
    source_refs.dedup();
    for trace in &generation_traces {
        source_refs.push(format!("GENERATIVE_LANGUAGE:{}", trace.generation_sha256));
    }
    source_refs.sort();
    source_refs.dedup();
    source_refs.truncate(MAX_SOURCE_REFS);
    if source_refs.is_empty() {
        source_refs.push("LANGUAGE_INPUT:CURRENT_TURN".to_string());
    }
    let sentences = move_texts
        .iter()
        .flat_map(|(act, surface)| {
            split_sentences(surface)
                .into_iter()
                .map(|sentence| (*act, sentence))
                .collect::<Vec<_>>()
        })
        .take(MAX_SENTENCES)
        .enumerate()
        .map(|(sentence_index, (act, surface))| NaturalSentenceIR {
            sentence_index,
            function: sentence_function(act),
            surface,
            source_refs: source_refs.clone(),
            semantic_authority: false,
            external_action_executed: false,
        })
        .collect::<Vec<_>>();
    let sentence_surfaces = sentences
        .iter()
        .map(|sentence| sentence.surface.clone())
        .collect::<Vec<_>>();
    let text = render_response_surface(
        response_plan.response_format,
        sources.language,
        &sentence_surfaces,
    );
    let internal_ir_leaks = internal_ir_leak_count(&text);
    let empty_promises = empty_promise_count(&text);
    let coverage = build_natural_realization_coverage(
        &response_plan,
        &generation_traces,
        &move_trace_ranges,
        if primary_act == NaturalResponseActIR::PlanPreview {
            sources.semantic_goal
        } else {
            None
        },
    );
    let mut violations = Vec::new();
    if text.trim().is_empty() {
        violations.push("EMPTY_REALIZATION".to_string());
    }
    if text.chars().count() > MAX_REALIZED_CHARS {
        violations.push("REALIZATION_TOO_LONG".to_string());
    }
    if sentences.is_empty() {
        violations.push("NO_SENTENCE_PLAN".to_string());
    }
    if internal_ir_leaks > 0 {
        violations.push("INTERNAL_IR_LEAK".to_string());
    }
    if empty_promises > 0 {
        violations.push("EMPTY_META_PROMISE".to_string());
    }
    if sources.unsupported_claims > 0 {
        violations.push("UNSUPPORTED_CLAIM".to_string());
    }
    if coverage.omitted_required_obligations > 0 {
        violations.push("OMITTED_REQUIRED_SEMANTIC_OBLIGATION".to_string());
    }
    if coverage.orphan_generation_traces > 0 {
        violations.push("ORPHAN_GENERATION_TRACE".to_string());
    }
    let faithful = violations.is_empty();
    debug_assert!(!generation_traces.is_empty());
    let realization_path = NaturalRealizationPathIR::Generative;
    let mut realization = NaturalRealizationIR {
        schema: NATURAL_REALIZATION_SCHEMA.to_string(),
        response_arbitration: sources.response_arbitration.clone(),
        response_plan,
        response_act: primary_act,
        language: sources.language,
        sentences,
        realized_text: text,
        realization_path,
        generation_traces,
        coverage,
        stage_overwrite_count: 0,
        unsupported_claims: sources.unsupported_claims,
        empty_promises,
        internal_ir_leaks,
        violations,
        faithful,
        semantic_authority: false,
        language_can_execute: false,
        external_action_executed: false,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_calls: 0,
        realization_sha256: String::new(),
    };
    realization.realization_sha256 = natural_realization_sha256(&realization);
    realization
}

fn native_evidence_subject<'a>(sources: &'a NaturalRealizationSources<'a>) -> &'a str {
    sources
        .native_language_circuit
        .entities
        .iter()
        .find(|entity| !entity.rejected_by_contrast)
        .map(|entity| entity.surface.as_str())
        .filter(|surface| !surface.trim().is_empty())
        .unwrap_or(match sources.language {
            LanguageCodeIR::Korean => "보고된 결과",
            _ => "the reported outcome",
        })
}

fn dialogue_response_kind(
    act: NaturalResponseActIR,
    events: &[DiscourseEventIR],
) -> GenerationDialogueResponseKindIR {
    if act == NaturalResponseActIR::HoldFloor {
        return GenerationDialogueResponseKindIR::HoldFloor;
    }
    for (function, response) in [
        (
            DiscourseFunctionIR::Greeting,
            GenerationDialogueResponseKindIR::Greeting,
        ),
        (
            DiscourseFunctionIR::Gratitude,
            GenerationDialogueResponseKindIR::Gratitude,
        ),
        (
            DiscourseFunctionIR::Farewell,
            GenerationDialogueResponseKindIR::Farewell,
        ),
    ] {
        if events.iter().any(|event| event.function == function) {
            return response;
        }
    }
    GenerationDialogueResponseKindIR::Backchannel
}

pub fn natural_realization_sha256(realization: &NaturalRealizationIR) -> String {
    let mut canonical = realization.clone();
    canonical.realization_sha256.clear();
    content_sha256(&canonical)
}

fn sentence_function(act: NaturalResponseActIR) -> NaturalSentenceFunctionIR {
    match act {
        NaturalResponseActIR::PlanPreview => NaturalSentenceFunctionIR::DescribePlan,
        NaturalResponseActIR::PlanResultStatus
        | NaturalResponseActIR::ResultAbsence
        | NaturalResponseActIR::ActionState
        | NaturalResponseActIR::TemporalAnswer
        | NaturalResponseActIR::DialogueRelationAnswer
        | NaturalResponseActIR::DiscourseAnswer => NaturalSentenceFunctionIR::AnswerStatus,
        NaturalResponseActIR::ClarificationRequest => {
            NaturalSentenceFunctionIR::RequestClarification
        }
        NaturalResponseActIR::AffectSupport => NaturalSentenceFunctionIR::SupportAffect,
        NaturalResponseActIR::InformAcknowledgement => NaturalSentenceFunctionIR::Acknowledge,
        NaturalResponseActIR::TopicTransition
        | NaturalResponseActIR::DiscourseGroupUpdate
        | NaturalResponseActIR::SocialBackchannel
        | NaturalResponseActIR::HoldFloor => NaturalSentenceFunctionIR::ManageDialogue,
        _ => NaturalSentenceFunctionIR::StateEvidenceBoundary,
    }
}

#[cfg(test)]
fn realize_plan_preview(
    language: LanguageCodeIR,
    subject: &str,
    raw_input: &str,
    intent: PlanIntentIR,
) -> GenerativeLanguageIR {
    let normalized_subject = normalize_plan_subject(language, subject.trim(), raw_input, intent);
    let restored = restore_grounded_capitalization(&normalized_subject, raw_input);
    let generation_language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    generate_plan_preview_from_knowledge(
        generation_language,
        &restored,
        intent,
        &format!("PLAN_SUBJECT:{}:{intent:?}", content_sha256(&restored)),
    )
    .expect("built-in plan realization knowledge must cover every PlanIntentIR")
}

fn generate_nonliteral_interpretation_response(
    sources: &NaturalRealizationSources<'_>,
) -> Option<GenerativeLanguageIR> {
    if sources.nonliteral_analysis.has_sarcasm() {
        return Some(
            generate_plan_interpretation_from_knowledge(
                sources.language,
                GenerationPlanInterpretationKindIR::SarcasmBoundary,
                match sources.language {
                    LanguageCodeIR::Korean => "표면적인 칭찬과 앞서 말한 실패 상태",
                    _ => "the surface praise and the stated failure state",
                },
                None,
                &["NONLITERAL:SARCASM:SEMANTIC_INCONGRUITY".to_string()],
            )
            .expect("a selected sarcasm reading must remain generatively realizable"),
        );
    }
    let expression = sources
        .nonliteral_analysis
        .expressions
        .iter()
        .find(|expression| expression.selected_reading == ReadingSelectionIR::Figurative)?;
    Some(
        generate_plan_interpretation_from_knowledge(
            sources.language,
            GenerationPlanInterpretationKindIR::FigurativeBoundary,
            &expression.surface_text,
            Some(figurative_concept_surface(
                sources.language,
                &expression.figurative_concept,
            )),
            &[
                format!("NONLITERAL_KIND:{:?}", expression.kind),
                format!("FIGURATIVE_CONCEPT:{}", expression.figurative_concept),
            ],
        )
        .expect("a selected figurative reading must remain generatively realizable"),
    )
}

fn generate_plan_preview_response(
    sources: &NaturalRealizationSources<'_>,
) -> Vec<GenerativeLanguageIR> {
    let semantic_goal = sources
        .semantic_goal
        .expect("a plan-preview response act must retain its typed semantic goal");
    let semantic_plan_bundle = sources
        .semantic_plan_bundle
        .expect("a plan-preview response act must retain its typed semantic plan bundle");
    assert!(
        semantic_plan_bundle.validate_against(semantic_goal),
        "a plan preview may only realize a validated semantic plan bundle"
    );
    let concise_directive = active_response_length_directive(sources)
        .filter(|directive| directive.value_key == "CONCISE");
    let mut generated = Vec::new();
    for event_id in &semantic_goal.selected_live_event_ids {
        let event = semantic_goal
            .events
            .iter()
            .find(|event| &event.event_id == event_id)
            .expect("validated semantic goal must retain each selected event");
        let subject = semantic_event_subject(semantic_goal, event)
            .expect("a selected semantic event must retain a realizable subject");
        let subject = restore_grounded_capitalization(&subject, sources.raw_input);
        let binding = semantic_plan_bundle
            .event_plan_bindings
            .iter()
            .find(|binding| &binding.event_id == event_id)
            .expect("validated semantic plan bundle must bind each selected event");
        let event_ref = format!(
            "SEMANTIC_PLAN_EVENT:{}:{}:{}",
            event.event_id, binding.plan_sha256, semantic_goal.semantic_sha256
        );
        let interpretation_kind = sources.inferred_goal.and_then(|goal| {
            ((event.event_id.starts_with("SEMANTIC-SUPPLEMENT-PRAGMATIC-")
                || (goal.intent == event.intent && goal.subject.eq_ignore_ascii_case(&subject)))
                && !goal.external_execution_authorized)
                .then_some(match (goal.commitment, goal.intent) {
                    (
                        GoalCommitmentIR::Suggestion,
                        PlanIntentIR::Repair
                        | PlanIntentIR::Create
                        | PlanIntentIR::Execute
                        | PlanIntentIR::Plan,
                    ) => Some(GenerationPlanInterpretationKindIR::Suggestion),
                    (GoalCommitmentIR::ImplicitRequest, PlanIntentIR::Repair) => {
                        Some(GenerationPlanInterpretationKindIR::ImplicitRepair)
                    }
                    (GoalCommitmentIR::ImplicitRequest, PlanIntentIR::Plan) => {
                        Some(GenerationPlanInterpretationKindIR::ImplicitPlanning)
                    }
                    _ => None,
                })
                .flatten()
        });
        if let Some(kind) = interpretation_kind {
            let mut refs = sources
                .inferred_goal
                .map(|goal| goal.basis_clause_ids.clone())
                .unwrap_or_default();
            refs.push(event_ref);
            generated.push(
                generate_plan_interpretation_from_knowledge(
                    sources.language,
                    kind,
                    &subject,
                    None,
                    &refs,
                )
                .expect("an inferred semantic event must remain generatively realizable"),
            );
        } else {
            let trace = if let Some(directive) = concise_directive {
                generate_plan_preview_from_knowledge_with_directive(
                    sources.language,
                    &subject,
                    event.intent,
                    &event_ref,
                    Some(&format!("DIALOGUE_DIRECTIVE:{}", directive.directive_id)),
                    true,
                )
            } else {
                generate_plan_preview_from_knowledge(
                    sources.language,
                    &subject,
                    event.intent,
                    &event_ref,
                )
            };
            generated
                .push(trace.expect("selected semantic events must use known plan constructions"));
        }
    }

    let plan_trace_count = semantic_goal.selected_live_event_ids.len();
    if plan_trace_count > 1 {
        generated.push(
            generate_lifecycle_status_from_knowledge(
                sources.language,
                match sources.language {
                    LanguageCodeIR::Korean => "이 계획",
                    _ => "this plan",
                },
                &[GenerationLifecycleClaimIR::NoVerifiedExecutionOrResult],
                &format!("PLAN_SET_BOUNDARY:{}", semantic_plan_bundle.bundle_sha256),
            )
            .expect("a multi-goal plan must retain a generated result boundary"),
        );
    }

    let mut excluded_subjects = BTreeSet::new();
    for event in semantic_goal
        .events
        .iter()
        .filter(|event| event.projection == SemanticPlanProjectionIR::Prohibited)
    {
        let Some(subject) = semantic_event_subject(semantic_goal, event) else {
            continue;
        };
        let subject = subject.trim();
        if subject.is_empty() || !excluded_subjects.insert(subject.to_lowercase()) {
            continue;
        }
        let refs = vec![
            format!("SEMANTIC_PLAN_PROHIBITED_EVENT:{}", event.event_id),
            format!("SEMANTIC_PLAN_GOAL:{}", semantic_goal.semantic_sha256),
        ];
        generated.push(
            generate_plan_exclusion_from_knowledge(sources.language, subject, &refs)
                .expect("a blocked typed goal must remain generatively excludable"),
        );
    }
    generated
}

fn semantic_event_subject(
    goal: &SemanticPlanGoalIR,
    event: &SemanticPlanEventIR,
) -> Option<String> {
    let labels = event
        .goal_subject_argument_ids
        .iter()
        .filter_map(|argument_id| {
            goal.arguments
                .iter()
                .find(|argument| &argument.argument_id == argument_id)
        })
        .map(|argument| argument.grounded_label.trim())
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    (!labels.is_empty()).then(|| labels.join(" & "))
}

fn build_natural_realization_coverage(
    response_plan: &NaturalResponsePlanIR,
    generation_traces: &[GenerativeLanguageIR],
    move_trace_ranges: &[ResponseMoveTraceRangeIR],
    semantic_goal: Option<&SemanticPlanGoalIR>,
) -> NaturalRealizationCoverageIR {
    let mut obligations = Vec::new();
    for response_move in &response_plan.moves {
        let trace_hashes = move_trace_ranges
            .iter()
            .find(|range| range.move_index == response_move.move_index)
            .filter(|range| {
                range.trace_start < range.trace_end && range.trace_end <= generation_traces.len()
            })
            .map(|range| {
                generation_traces[range.trace_start..range.trace_end]
                    .iter()
                    .map(|trace| trace.generation_sha256.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        obligations.push(NaturalRealizationObligationIR {
            obligation_id: format!("RESPONSE-MOVE-{:03}", response_move.move_index + 1),
            kind: NaturalRealizationObligationKindIR::ResponseMove,
            response_move_index: Some(response_move.move_index),
            response_act: Some(response_move.response_act),
            semantic_event_id: None,
            semantic_relation_id: None,
            satisfied: !trace_hashes.is_empty(),
            supporting_generation_trace_sha256s: trace_hashes,
        });
    }
    if let Some(goal) = semantic_goal {
        for event_id in &goal.selected_live_event_ids {
            let prefix = format!("SEMANTIC_PLAN_EVENT:{event_id}:");
            let trace_hashes = generation_traces
                .iter()
                .filter(|trace| trace_has_grounding_prefix(trace, &prefix))
                .map(|trace| trace.generation_sha256.clone())
                .collect::<Vec<_>>();
            obligations.push(NaturalRealizationObligationIR {
                obligation_id: format!("SELECTED-PLAN-EVENT-{event_id}"),
                kind: NaturalRealizationObligationKindIR::SelectedPlanEvent,
                response_move_index: None,
                response_act: None,
                semantic_event_id: Some(event_id.clone()),
                semantic_relation_id: None,
                satisfied: trace_hashes.len() == 1,
                supporting_generation_trace_sha256s: trace_hashes,
            });
        }
        for event in goal
            .events
            .iter()
            .filter(|event| event.projection == SemanticPlanProjectionIR::Prohibited)
        {
            let grounding = format!("SEMANTIC_PLAN_PROHIBITED_EVENT:{}", event.event_id);
            let trace_hashes = generation_traces
                .iter()
                .filter(|trace| trace_has_grounding(trace, &grounding))
                .map(|trace| trace.generation_sha256.clone())
                .collect::<Vec<_>>();
            obligations.push(NaturalRealizationObligationIR {
                obligation_id: format!("PROHIBITED-PLAN-EVENT-{}", event.event_id),
                kind: NaturalRealizationObligationKindIR::ProhibitedPlanEvent,
                response_move_index: None,
                response_act: None,
                semantic_event_id: Some(event.event_id.clone()),
                semantic_relation_id: None,
                satisfied: trace_hashes.len() == 1,
                supporting_generation_trace_sha256s: trace_hashes,
            });
        }
        let selected = goal
            .selected_live_event_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for relation in goal.relations.iter().filter(|relation| {
            relation_requires_realization_coverage(relation.relation)
                && selected.contains(relation.source_event_id.as_str())
                && selected.contains(relation.target_event_id.as_str())
        }) {
            let source_prefix = format!("SEMANTIC_PLAN_EVENT:{}:", relation.source_event_id);
            let target_prefix = format!("SEMANTIC_PLAN_EVENT:{}:", relation.target_event_id);
            let source_index = generation_traces
                .iter()
                .position(|trace| trace_has_grounding_prefix(trace, &source_prefix));
            let target_index = generation_traces
                .iter()
                .position(|trace| trace_has_grounding_prefix(trace, &target_prefix));
            let satisfied = match (source_index, target_index, relation.relation) {
                (Some(source), Some(target), SemanticPlanRelationKindIR::Sequence)
                | (Some(source), Some(target), SemanticPlanRelationKindIR::TemporalBefore) => {
                    source < target
                }
                (Some(_), Some(_), SemanticPlanRelationKindIR::Coordination) => true,
                _ => false,
            };
            let trace_hashes = [source_index, target_index]
                .into_iter()
                .flatten()
                .map(|index| generation_traces[index].generation_sha256.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            obligations.push(NaturalRealizationObligationIR {
                obligation_id: format!("SELECTED-EVENT-RELATION-{}", relation.relation_id),
                kind: NaturalRealizationObligationKindIR::SelectedEventRelation,
                response_move_index: None,
                response_act: None,
                semantic_event_id: None,
                semantic_relation_id: Some(relation.relation_id.clone()),
                supporting_generation_trace_sha256s: trace_hashes,
                satisfied,
            });
        }
    }
    let response_bound_trace_hashes = obligations
        .iter()
        .filter(|obligation| obligation.kind == NaturalRealizationObligationKindIR::ResponseMove)
        .flat_map(|obligation| obligation.supporting_generation_trace_sha256s.iter())
        .collect::<BTreeSet<_>>();
    let orphan_generation_traces = generation_traces
        .iter()
        .filter(|trace| !response_bound_trace_hashes.contains(&trace.generation_sha256))
        .count();
    let omitted_required_obligations = obligations
        .iter()
        .filter(|obligation| !obligation.satisfied)
        .count();
    let mut coverage = NaturalRealizationCoverageIR {
        schema: NATURAL_REALIZATION_COVERAGE_SCHEMA.to_string(),
        semantic_goal_sha256: semantic_goal.map(|goal| goal.semantic_sha256.clone()),
        obligations,
        omitted_required_obligations,
        orphan_generation_traces,
        coverage_sha256: String::new(),
    };
    coverage.coverage_sha256 = natural_realization_coverage_sha256(&coverage);
    coverage
}

fn trace_has_grounding(trace: &GenerativeLanguageIR, grounding: &str) -> bool {
    trace.meaning.nodes.iter().any(|node| {
        node.grounding_refs
            .iter()
            .any(|reference| reference == grounding)
    })
}

fn trace_for_hash<'a>(
    generation_traces: &'a [GenerativeLanguageIR],
    trace_sha256: &str,
) -> Option<&'a GenerativeLanguageIR> {
    generation_traces
        .iter()
        .find(|trace| trace.generation_sha256 == trace_sha256)
}

fn trace_has_semantic_event_grounding(
    trace: &GenerativeLanguageIR,
    event_id: &str,
    semantic_goal_sha256: &str,
) -> bool {
    let prefix = format!("SEMANTIC_PLAN_EVENT:{event_id}:");
    trace.meaning.nodes.iter().any(|node| {
        node.grounding_refs.iter().any(|reference| {
            reference.starts_with(&prefix) && reference.ends_with(semantic_goal_sha256)
        })
    })
}

fn trace_has_grounding_prefix(trace: &GenerativeLanguageIR, prefix: &str) -> bool {
    trace.meaning.nodes.iter().any(|node| {
        node.grounding_refs
            .iter()
            .any(|reference| reference.starts_with(prefix))
    })
}

fn relation_requires_realization_coverage(relation: SemanticPlanRelationKindIR) -> bool {
    matches!(
        relation,
        SemanticPlanRelationKindIR::Coordination
            | SemanticPlanRelationKindIR::Sequence
            | SemanticPlanRelationKindIR::TemporalBefore
    )
}

fn figurative_concept_surface(language: LanguageCodeIR, concept_id: &str) -> &'static str {
    match (language, concept_id) {
        (LanguageCodeIR::Korean, "C_PROGRESS_BLOCKED") => "진행이 막힌 상태",
        (LanguageCodeIR::Korean, "C_GOAL_DRIFT") => "목표에서 벗어난 상태",
        (LanguageCodeIR::Korean, "C_PROGRESS_BLOCKER") => "진행을 막는 문제",
        (LanguageCodeIR::Korean, "C_NO_PRODUCTIVE_PATH") => "생산적인 경로가 없는 상태",
        (LanguageCodeIR::Korean, "C_THROUGHPUT_CONSTRAINT") => "처리량을 제한하는 병목",
        (LanguageCodeIR::Korean, "C_CRITICAL_INCIDENT") => "심각한 문제 상태",
        (LanguageCodeIR::Korean, _) => "실제 진행상의 문제",
        (_, "C_PROGRESS_BLOCKED") => "blocked progress",
        (_, "C_GOAL_DRIFT") => "drift away from the goal",
        (_, "C_PROGRESS_BLOCKER") => "a blocker to progress",
        (_, "C_NO_PRODUCTIVE_PATH") => "the absence of a productive path",
        (_, "C_THROUGHPUT_CONSTRAINT") => "a throughput constraint",
        (_, "C_CRITICAL_INCIDENT") => "a critical problem state",
        (_, _) => "an actual problem state",
    }
}

#[cfg(test)]
fn normalize_plan_subject(
    language: LanguageCodeIR,
    parsed_subject: &str,
    raw_input: &str,
    intent: PlanIntentIR,
) -> String {
    if language == LanguageCodeIR::Korean {
        if let Some(subject) = korean_request_subject(raw_input, intent) {
            return subject;
        }
        return parsed_subject.to_string();
    }
    let trimmed = parsed_subject.trim();
    let lower = trimmed.to_lowercase();
    for prefix in [
        "find the cause of the ",
        "find the cause of ",
        "investigate the ",
        "investigate ",
        "repair the ",
        "repair ",
        "explain the ",
        "explain ",
        "describe the ",
        "describe ",
        "create the ",
        "create ",
        "run the ",
        "run ",
    ] {
        if lower.starts_with(prefix) {
            let candidate = trimmed[prefix.len()..].trim();
            if !candidate.is_empty() {
                return candidate.to_string();
            }
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
fn korean_request_subject(raw_input: &str, intent: PlanIntentIR) -> Option<String> {
    let markers: &[&str] = match intent {
        PlanIntentIR::Repair => &[
            "을 수리",
            "를 수리",
            "을 고쳐",
            "를 고쳐",
            "을 수정",
            "를 수정",
        ],
        PlanIntentIR::Investigate => &[
            "을 조사",
            "를 조사",
            "을 분석",
            "를 분석",
            "을 확인",
            "를 확인",
        ],
        PlanIntentIR::Create => &[
            "을 생성",
            "를 생성",
            "을 만들",
            "를 만들",
            "을 작성",
            "를 작성",
        ],
        PlanIntentIR::Execute => &["을 실행", "를 실행", "을 수행", "를 수행"],
        PlanIntentIR::Explain | PlanIntentIR::Communicate => {
            &["을 설명", "를 설명", "을 요약", "를 요약"]
        }
        PlanIntentIR::Learn => &["을 학습", "를 학습", "을 배워", "를 배워"],
        PlanIntentIR::Plan => &["을 계획", "를 계획", "을 추천", "를 추천"],
    };
    let trimmed = raw_input.trim().trim_end_matches(['.', '!', '?', '。']);
    markers.iter().find_map(|marker| {
        let index = trimmed.rfind(marker)?;
        let candidate = trimmed[..index].trim();
        (!candidate.is_empty()
            && candidate.chars().count() <= 120
            && !candidate.contains(['.', '?', '!', '。']))
        .then(|| candidate.to_string())
    })
}

fn generate_plan_result_boundary(
    language: LanguageCodeIR,
    act: NaturalResponseActIR,
    boundary: &PlanResultBoundaryIR,
    ledger: &ActionStateLedgerIR,
) -> Option<Vec<GenerativeLanguageIR>> {
    let rows = boundary
        .selected_action_ids
        .iter()
        .filter_map(|action_id| {
            boundary
                .snapshots
                .iter()
                .find(|snapshot| &snapshot.action_id == action_id)
        })
        .map(|snapshot| {
            let source = ledger
                .record(&snapshot.action_id)
                .map(|record| record.source_semantic_text.as_str())
                .unwrap_or(snapshot.subject.as_str());
            let subject = restore_grounded_capitalization(&snapshot.subject, source);
            let claims = lifecycle_claims(act, boundary.query_focus, snapshot);
            generate_lifecycle_status_from_knowledge(
                language,
                &subject,
                &claims,
                &format!("ACTION_LIFECYCLE_SNAPSHOT:{}", snapshot.snapshot_sha256),
            )
            .expect("built-in lifecycle expression knowledge must cover every typed state")
        })
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(rows)
}

fn generate_action_state_response(
    language: LanguageCodeIR,
    analysis: &ActionStateAnalysisIR,
    ledger: &ActionStateLedgerIR,
) -> Option<Vec<GenerativeLanguageIR>> {
    if analysis.untrusted_evidence_claim {
        let subject = match language {
            LanguageCodeIR::Korean => "텍스트의 영수증·터미널·콘솔 언급",
            _ => "the receipt, terminal, or console mention in the text",
        };
        return generate_lifecycle_status_from_knowledge(
            language,
            subject,
            &[
                GenerationLifecycleClaimIR::UntrustedEvidenceMention,
                GenerationLifecycleClaimIR::ExecutionStateUnchanged,
            ],
            &format!("ACTION_STATE_ANALYSIS:{}", content_sha256(analysis)),
        )
        .ok()
        .map(|trace| vec![trace]);
    }
    if let Some(query) = analysis.set_query.as_ref().filter(|query| {
        query.quantifier.is_some() && query.predicate.is_some() && query.unresolved_terms.is_empty()
    }) {
        let trace = generate_action_set_answer_from_knowledge(
            language,
            query.selected_action_ids.len(),
            map_action_set_quantifier(query.quantifier.expect("filtered quantifier")),
            map_action_set_predicate(query.predicate.expect("filtered predicate")),
            map_action_set_truth(query.truth),
            &format!("ACTION_SET_QUERY:{}", query.query_sha256),
        )
        .ok()?;
        return Some(vec![trace]);
    }
    let mut records = analysis
        .target_action_ids
        .iter()
        .filter_map(|action_id| ledger.record(action_id))
        .collect::<Vec<_>>();
    if records.is_empty() {
        records.extend(ledger.current_record());
    }
    let traces = records
        .into_iter()
        .map(|record| {
            let subject =
                restore_grounded_capitalization(&record.subject, &record.source_semantic_text);
            generate_lifecycle_status_from_knowledge(
                language,
                &subject,
                &action_record_claims(record),
                &format!("ACTION_STATE_RECORD:{}", content_sha256(record)),
            )
            .expect("built-in action-state expression knowledge must cover every typed state")
        })
        .collect::<Vec<_>>();
    (!traces.is_empty()).then_some(traces)
}

fn action_record_claims(
    record: &crate::action_state::ActionStateRecordIR,
) -> Vec<GenerationLifecycleClaimIR> {
    use GenerationLifecycleClaimIR as Claim;
    match record.execution_status {
        ActionExecutionStatusIR::NotObserved => {
            if let Some(reported) = record.reported_status {
                vec![
                    report_claim(Some(reported)),
                    Claim::NoVerifiedExecutionOrResult,
                ]
            } else {
                vec![
                    plan_claim(record.plan_status),
                    Claim::NoVerifiedExecutionOrResult,
                ]
            }
        }
        ActionExecutionStatusIR::InProgress => {
            vec![Claim::ExecutionInProgress, Claim::FinalResultUnavailable]
        }
        ActionExecutionStatusIR::Succeeded => vec![Claim::VerifiedSuccess],
        ActionExecutionStatusIR::Failed => vec![Claim::VerifiedFailure],
    }
}

fn map_action_set_quantifier(value: ActionSetQuantifierIR) -> GenerationActionSetQuantifierIR {
    match value {
        ActionSetQuantifierIR::All => GenerationActionSetQuantifierIR::All,
        ActionSetQuantifierIR::Any => GenerationActionSetQuantifierIR::Any,
        ActionSetQuantifierIR::None => GenerationActionSetQuantifierIR::None,
    }
}

fn map_action_set_predicate(value: ActionStatePredicateIR) -> GenerationActionSetPredicateIR {
    match value {
        ActionStatePredicateIR::ActivePlan => GenerationActionSetPredicateIR::ActivePlan,
        ActionStatePredicateIR::ReportedCompletion => {
            GenerationActionSetPredicateIR::ReportedCompletion
        }
        ActionStatePredicateIR::ReportedFailure => GenerationActionSetPredicateIR::ReportedFailure,
        ActionStatePredicateIR::UnverifiedExecution => {
            GenerationActionSetPredicateIR::UnverifiedExecution
        }
        ActionStatePredicateIR::VerifiedExecution => {
            GenerationActionSetPredicateIR::VerifiedExecution
        }
        ActionStatePredicateIR::VerifiedSuccess => GenerationActionSetPredicateIR::VerifiedSuccess,
        ActionStatePredicateIR::VerifiedFailure => GenerationActionSetPredicateIR::VerifiedFailure,
        ActionStatePredicateIR::VerifiedInProgress => {
            GenerationActionSetPredicateIR::VerifiedInProgress
        }
    }
}

fn map_action_set_truth(value: ActionSetTruthIR) -> GenerationActionSetTruthIR {
    match value {
        ActionSetTruthIR::True => GenerationActionSetTruthIR::True,
        ActionSetTruthIR::False => GenerationActionSetTruthIR::False,
        ActionSetTruthIR::Unknown | ActionSetTruthIR::NotApplicable => {
            GenerationActionSetTruthIR::Unknown
        }
    }
}

fn map_user_feedback_kind(value: UserFeedbackKindIR) -> GenerationUserFeedbackKindIR {
    match value {
        UserFeedbackKindIR::Unhelpful => GenerationUserFeedbackKindIR::Unhelpful,
        UserFeedbackKindIR::Misunderstood => GenerationUserFeedbackKindIR::Misunderstood,
        UserFeedbackKindIR::MissedPoint => GenerationUserFeedbackKindIR::MissedPoint,
        UserFeedbackKindIR::TooVerbose => GenerationUserFeedbackKindIR::TooVerbose,
        UserFeedbackKindIR::TooBrief => GenerationUserFeedbackKindIR::TooBrief,
        UserFeedbackKindIR::Incorrect => GenerationUserFeedbackKindIR::Incorrect,
    }
}

fn map_discourse_group_update_kind(
    value: DiscourseGroupUpdateOperationIR,
) -> GenerationDiscourseGroupUpdateKindIR {
    match value {
        DiscourseGroupUpdateOperationIR::AddMember => {
            GenerationDiscourseGroupUpdateKindIR::AddMember
        }
        DiscourseGroupUpdateOperationIR::RemoveMember => {
            GenerationDiscourseGroupUpdateKindIR::RemoveMember
        }
        DiscourseGroupUpdateOperationIR::MergeGroups => {
            GenerationDiscourseGroupUpdateKindIR::MergeGroups
        }
        DiscourseGroupUpdateOperationIR::Unresolved => {
            unreachable!("an applied discourse-group update cannot be unresolved")
        }
    }
}

fn lifecycle_claims(
    act: NaturalResponseActIR,
    focus: PlanResultQueryFocusIR,
    snapshot: &crate::plan_result_boundary::ActionLifecycleSnapshotIR,
) -> Vec<GenerationLifecycleClaimIR> {
    use GenerationLifecycleClaimIR as Claim;
    if act == NaturalResponseActIR::ResultAbsence
        && snapshot.result_availability == ResultAvailabilityIR::Unavailable
    {
        return vec![Claim::ResultUnavailable, plan_claim(snapshot.plan_status)];
    }
    match snapshot.execution_status {
        ActionExecutionStatusIR::NotObserved if snapshot.reported_status.is_some() => {
            vec![
                report_claim(snapshot.reported_status),
                if focus == PlanResultQueryFocusIR::ReportedVersusResult {
                    Claim::ResultUnavailable
                } else {
                    Claim::NoVerifiedExecutionOrResult
                },
            ]
        }
        ActionExecutionStatusIR::NotObserved => vec![
            plan_claim(snapshot.plan_status),
            Claim::NoVerifiedExecutionOrResult,
        ],
        ActionExecutionStatusIR::InProgress => {
            vec![Claim::ExecutionInProgress, Claim::FinalResultUnavailable]
        }
        ActionExecutionStatusIR::Succeeded => vec![Claim::VerifiedSuccess],
        ActionExecutionStatusIR::Failed => vec![Claim::VerifiedFailure],
    }
}

fn plan_claim(status: ActionPlanStatusIR) -> GenerationLifecycleClaimIR {
    match status {
        ActionPlanStatusIR::Active => GenerationLifecycleClaimIR::ActivePlan,
        ActionPlanStatusIR::Superseded => GenerationLifecycleClaimIR::SupersededPlan,
        ActionPlanStatusIR::Withdrawn => GenerationLifecycleClaimIR::WithdrawnPlan,
    }
}

fn report_claim(status: Option<ActionReportedStatusIR>) -> GenerationLifecycleClaimIR {
    match status {
        Some(ActionReportedStatusIR::Attempted) => GenerationLifecycleClaimIR::ReportedAttempt,
        Some(ActionReportedStatusIR::InProgressClaimed) => {
            GenerationLifecycleClaimIR::ReportedInProgress
        }
        Some(ActionReportedStatusIR::SuccessClaimed) => GenerationLifecycleClaimIR::ReportedSuccess,
        Some(ActionReportedStatusIR::FailureClaimed) => GenerationLifecycleClaimIR::ReportedFailure,
        None => GenerationLifecycleClaimIR::NoUserReport,
    }
}

fn affect_kind(raw_input: &str) -> GenerationAffectKindIR {
    let lower = raw_input.to_lowercase();
    if lower.contains("화나") || lower.contains("화가 나") || lower.contains("angry") {
        GenerationAffectKindIR::Angry
    } else if lower.contains("걱정")
        || lower.contains("불안")
        || lower.contains("worried")
        || lower.contains("worry")
    {
        GenerationAffectKindIR::Worried
    } else if lower.contains("속상") || lower.contains("hurt") || lower.contains("upset") {
        GenerationAffectKindIR::Hurt
    } else if lower.contains("짜증") || lower.contains("킹받") || lower.contains("annoy") {
        GenerationAffectKindIR::Annoyed
    } else {
        GenerationAffectKindIR::Frustrated
    }
}

fn affect_surface_present(raw_input: &str) -> bool {
    let lower = raw_input.to_lowercase();
    [
        "답답",
        "frustrating",
        "frustrated",
        "화나",
        "화가 나",
        "angry",
        "속상",
        "hurt",
        "upset",
        "불안",
        "걱정",
        "worried",
        "worrying",
        "짜증",
        "킹받",
        "annoying",
        "annoyed",
    ]
    .iter()
    .any(|surface| lower.contains(surface))
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for character in text.trim().chars() {
        current.push(character);
        if matches!(character, '.' | '!' | '?' | '。' | '！' | '？' | '\n') {
            let sentence = current.trim().to_string();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            current.clear();
        }
    }
    let sentence = current.trim().to_string();
    if !sentence.is_empty() {
        sentences.push(sentence);
    }
    sentences
}

fn internal_ir_leak_count(text: &str) -> usize {
    let lower = text.to_lowercase();
    [
        "goalir",
        "planir",
        "compositional_goal_graph",
        "success_claimed",
        "not_observed",
        "plan_versus_result",
        "reported_versus_result",
        "execution_versus_plan",
        "evidence_absence",
        "resultreference",
    ]
    .iter()
    .filter(|marker| lower.contains(**marker))
    .count()
}

fn empty_promise_count(text: &str) -> usize {
    let lower = text.to_lowercase();
    [
        "단계별로 안내하겠다",
        "단계별로 안내할게",
        "곧 알려줄게",
        "will guide you step by step",
        "i'll provide guidance later",
        "acknowledge your emotion",
        "감정을 인정한다",
    ]
    .iter()
    .filter(|marker| lower.contains(**marker))
    .count()
}

fn restore_grounded_capitalization(text: &str, source: &str) -> String {
    let acronyms = grounded_capitalized_tokens(source);
    if acronyms.is_empty() {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut token = String::new();
    let flush = |token: &mut String, output: &mut String| {
        if let Some(surface) = acronyms
            .iter()
            .find(|surface| surface.eq_ignore_ascii_case(token))
        {
            output.push_str(surface);
        } else {
            output.push_str(token);
        }
        token.clear();
    };
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
        } else {
            flush(&mut token, &mut output);
            output.push(character);
        }
    }
    flush(&mut token, &mut output);
    output
}

fn grounded_capitalized_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let flush = |token: &mut String, tokens: &mut Vec<String>| {
        let has_upper = token
            .chars()
            .any(|character| character.is_ascii_uppercase());
        let first_upper = token
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase());
        let remaining_lower = token
            .chars()
            .skip(1)
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
        if token.len() >= 2
            && token.len() <= 32
            && has_upper
            && (first_upper && remaining_lower
                || token
                    .chars()
                    .filter(|character| character.is_ascii_uppercase())
                    .count()
                    >= 2)
            && !tokens.contains(token)
        {
            tokens.push(token.clone());
        }
        token.clear();
    };
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
        } else {
            flush(&mut token, &mut tokens);
        }
    }
    flush(&mut token, &mut tokens);
    tokens
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable natural realization");
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_state::{ActionStateAnalyzer, ActionStateRecordIR};

    fn action_record(action_id: &str, subject: &str) -> ActionStateRecordIR {
        ActionStateRecordIR {
            action_id: action_id.to_string(),
            goal_id: action_id.to_string(),
            canonical_predicate: "EXECUTE".to_string(),
            predicate_surface: "run".to_string(),
            subject: subject.to_string(),
            source_semantic_text: format!("run {subject}"),
            plan_status: ActionPlanStatusIR::Active,
            execution_status: ActionExecutionStatusIR::NotObserved,
            reported_status: None,
            execution_id: None,
            execution_evidence_ids: Vec::new(),
            introduced_turn: 1,
            last_update_turn: 1,
            external_execution_authorized: false,
            external_action_execution_observed: false,
            verified_outcome: false,
            semantic_authority: false,
        }
    }

    #[test]
    fn internal_ir_and_empty_promises_are_rejected() {
        assert_eq!(internal_ir_leak_count("GoalIR says SUCCESS_CLAIMED"), 2);
        assert_eq!(empty_promise_count("단계별로 안내할게"), 1);
    }

    #[test]
    fn response_arbitration_is_order_independent_and_retains_suppressed_candidates() {
        let candidates = vec![
            NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::Fallback,
                NaturalResponseActIR::InteractionBoundary,
                "fallback",
            ),
            NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::Inform,
                NaturalResponseActIR::InformAcknowledgement,
                "inform",
            ),
            NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::Clarification,
                NaturalResponseActIR::ClarificationRequest,
                "unresolved binding",
            ),
            NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::NativePlan,
                NaturalResponseActIR::PlanPreview,
                "fully bound live goal",
            ),
        ];
        let expected = arbitrate_natural_response(candidates.clone());
        assert!(expected.validate());
        assert_eq!(expected.candidates.len(), 4);
        assert_eq!(
            expected.selected_source,
            NaturalResponseSourceIR::Clarification
        );
        assert_eq!(
            expected.selected_act,
            NaturalResponseActIR::ClarificationRequest
        );

        for rotation in 0..candidates.len() {
            let mut permuted = candidates.clone();
            permuted.rotate_left(rotation);
            if rotation % 2 == 1 {
                permuted.reverse();
            }
            let actual = arbitrate_natural_response(permuted);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn response_plan_preserves_auxiliary_moves_without_overwriting_primary_task() {
        let plan = compose_response_plan_from_signals(
            NaturalResponseActIR::PlanPreview,
            false,
            true,
            true,
            false,
            None,
        );
        assert!(plan.validate());
        assert_eq!(plan.primary_act(), NaturalResponseActIR::PlanPreview);
        assert_eq!(
            plan.moves
                .iter()
                .map(|response_move| response_move.response_act)
                .collect::<Vec<_>>(),
            vec![
                NaturalResponseActIR::AffectSupport,
                NaturalResponseActIR::TopicTransition,
                NaturalResponseActIR::PlanPreview,
            ]
        );
        assert_eq!(
            plan.moves[0].role,
            NaturalResponseMoveRoleIR::RelationalSupport
        );
        assert_eq!(
            plan.moves[1].role,
            NaturalResponseMoveRoleIR::DiscourseBridge
        );
        assert_eq!(plan.moves[2].role, NaturalResponseMoveRoleIR::PrimaryTask);
    }

    #[test]
    fn acknowledgement_does_not_gain_unlicensed_auxiliary_task_moves() {
        let plan = compose_response_plan_from_signals(
            NaturalResponseActIR::InformAcknowledgement,
            false,
            true,
            true,
            false,
            None,
        );
        assert!(plan.validate());
        assert_eq!(plan.moves.len(), 1);
        assert_eq!(
            plan.moves[0].response_act,
            NaturalResponseActIR::InformAcknowledgement
        );
    }

    #[test]
    fn response_plan_composition_is_closed_over_act_and_signal_cross_product() {
        let acts = [
            NaturalResponseActIR::PlanPreview,
            NaturalResponseActIR::InterpretationBoundary,
            NaturalResponseActIR::PlanResultStatus,
            NaturalResponseActIR::ResultAbsence,
            NaturalResponseActIR::ActionState,
            NaturalResponseActIR::InformAcknowledgement,
            NaturalResponseActIR::UserFeedback,
            NaturalResponseActIR::AffectSupport,
            NaturalResponseActIR::ClarificationRequest,
            NaturalResponseActIR::TopicTransition,
            NaturalResponseActIR::DefinitionGrounding,
            NaturalResponseActIR::DiscourseGroupUpdate,
            NaturalResponseActIR::ConditionalGuard,
            NaturalResponseActIR::TemporalAnswer,
            NaturalResponseActIR::DialogueRelationAnswer,
            NaturalResponseActIR::DiscourseAnswer,
            NaturalResponseActIR::ContinuationGate,
            NaturalResponseActIR::InteractionBoundary,
            NaturalResponseActIR::SocialBackchannel,
            NaturalResponseActIR::HoldFloor,
        ];
        let mut checked = 0;
        for primary_act in acts {
            for user_feedback_present in [false, true] {
                for affect_present in [false, true] {
                    for topic_transition_applied in [false, true] {
                        for concise in [false, true] {
                            let plan = compose_response_plan_from_signals(
                                primary_act,
                                user_feedback_present,
                                affect_present,
                                topic_transition_applied,
                                concise,
                                concise.then_some("DIALOGUE_DIRECTIVE:TEST-CONCISE"),
                            );
                            assert!(plan.validate(), "{plan:#?}");
                            assert_eq!(plan.primary_act(), primary_act, "{plan:#?}");
                            assert_eq!(
                                plan.moves
                                    .iter()
                                    .filter(|response_move| {
                                        response_move.role == NaturalResponseMoveRoleIR::PrimaryTask
                                    })
                                    .count(),
                                1,
                                "{plan:#?}"
                            );
                            if concise {
                                assert!(
                                    plan.moves.iter().all(|response_move| {
                                        response_move.response_act
                                            == NaturalResponseActIR::UserFeedback
                                            || response_move.response_act == primary_act
                                    }),
                                    "{plan:#?}"
                                );
                                assert!(plan.moves[plan.primary_move_index]
                                    .evidence
                                    .iter()
                                    .any(|evidence| evidence == "DIALOGUE_DIRECTIVE:TEST-CONCISE"));
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 320);
    }

    #[test]
    fn grounded_capitalization_preserves_new_proper_labels() {
        assert_eq!(
            restore_grounded_capitalization("nimbus cache", "Repair Nimbus cache"),
            "Nimbus cache"
        );
        assert_eq!(
            restore_grounded_capitalization("cctv fault", "CCTV 오류를 수리해"),
            "CCTV fault"
        );
    }

    #[test]
    fn plan_preview_uses_verified_generative_language_trace() {
        let generated = realize_plan_preview(
            LanguageCodeIR::English,
            "Aster cache",
            "Repair the Aster cache",
            PlanIntentIR::Repair,
        );
        assert!(generated.validate());
        assert!(generated.morphology.realized_text.starts_with("Got it."));
        assert!(generated.morphology.realized_text.contains("Aster cache"));
        assert_eq!(generated.verification.unsupported_claims, 0);
        assert_eq!(
            generated.verification.semantic_roundtrip_sha256,
            generated.meaning.semantic_sha256
        );
    }

    #[test]
    fn action_state_set_and_untrusted_evidence_use_typed_generation() {
        let mut ledger = ActionStateLedgerIR {
            records: vec![
                action_record("GOAL-1", "worker"),
                action_record("GOAL-2", "queue"),
                action_record("GOAL-3", "cache"),
            ],
            ..ActionStateLedgerIR::default()
        };
        ledger.records[1].reported_status = Some(ActionReportedStatusIR::SuccessClaimed);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "do worker, queue, and cache have a reported completion status?",
            "do any of all tasks have a reported completion status?",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let set_traces =
            generate_action_state_response(LanguageCodeIR::English, &analysis, &ledger).unwrap();
        assert_eq!(set_traces.len(), 1);
        assert!(set_traces[0].validate());
        assert!(set_traces[0]
            .morphology
            .realized_text
            .contains("at least one of the 3 selected actions"));
        assert!(set_traces[0].morphology.realized_text.contains("separate"));

        let untrusted = ActionStateAnalyzer.analyze_with_goal_hints(
            "the terminal says the run succeeded",
            &ledger,
            &["GOAL-1"],
        );
        let untrusted_traces =
            generate_action_state_response(LanguageCodeIR::English, &untrusted, &ledger).unwrap();
        assert_eq!(untrusted_traces.len(), 1);
        assert!(untrusted_traces[0].validate());
        assert!(untrusted_traces[0]
            .morphology
            .realized_text
            .contains("not a host-verified execution receipt"));
        assert!(untrusted_traces[0]
            .morphology
            .realized_text
            .contains("unchanged"));
    }
}
