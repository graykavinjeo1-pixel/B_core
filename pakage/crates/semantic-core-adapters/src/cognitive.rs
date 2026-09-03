use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::{
    DeliberationError, DeliberationIR, DeliberationRequestIR, DeliberationRevisionIR,
    DeliberationRevisionRequestIR, DockableCore, ExperienceError, ExperienceIR,
    ExperienceInjectionReceiptIR, ExperienceSnapshotIR, KnowledgeGroundedDeliberationIR,
    MechanismKnowledgeIR, MechanismKnowledgeInjectionReceiptIR, MechanismMemoryError,
    MechanismMemorySnapshotIR, MechanismQueryIR, PlanGoalIR, PlanIR, PlanIntentIR, PlanOperationIR,
    PlanningError, SemanticPlanBundleIR, SemanticPlanEventIR, SemanticPlanGoalIR, PLAN_GOAL_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::action_state::{
    ActionEvidenceReceiptIR, ActionEvidenceRequestIR, ActionExecutionStatusIR,
    ActionReportedStatusIR, ActionSetQuantifierIR, ActionSetTruthIR, ActionStateAnalysisIR,
    ActionStateAnalyzer, ActionStateLedgerIR, ActionStatePredicateIR, ActionStateRecordIR,
};
use crate::clause_graph::{ClauseFunctionIR, ClauseRelationKindIR};
use crate::compositional_semantics::{
    CandidateDispositionIR, GoalGraphRelationKindIR, InterpretationCandidateIR,
    PredicateLexemeError, PredicateLexemeIR, PredicateLexiconSnapshotIR,
};
use crate::conditional_guard::ConditionalGuardEvaluationIR;
use crate::conversation::{
    discourse_program_sha256, discourse_topic_concept_id, guard_condition_expression_sha256,
    topic_matches_subject, ConversationCommitContext, ConversationFrontendError,
    ConversationGoalFrameIR, ConversationMemory, ConversationStateIR,
    ConversationTurnDispositionIR, ConversationTurnRequestIR, DialogueDirectiveCandidateIR,
    DialogueDirectiveIR, DialogueDirectiveKindIR, DiscourseBindingKindIR, DiscourseFunctionIR,
    DiscourseGroupUpdateIR, DiscourseGroupUpdateOperationIR, DiscourseProgramGuardIR,
    DiscourseProgramIR, DiscourseProgramStepIR, DiscourseReferentKindIR,
    DynamicDiscourseReferentIR, GuardConditionExpressionIR, GuardConditionOperatorIR,
    NormalizedUtteranceIR, QuestionAnswerDispositionIR, QuestionAnswerResolutionIR,
    QuestionOptionIR, QuestionUnderDiscussionIR, QuestionUnderDiscussionKindIR,
    ReferenceResolutionIR, TopicTransitionIR, TopicTransitionKindIR, UtteranceNormalizer,
    DISCOURSE_PROGRAM_GUARD_SCHEMA, DISCOURSE_PROGRAM_SCHEMA,
};
use crate::deferred_commitment::{
    condition_sha256, normalize_condition, ConditionEvidenceReceiptIR, ConditionEvidenceRequestIR,
    DeferredActionCommitmentIR, DeferredActionIR, DeferredCommitmentStatusIR,
    DEFERRED_ACTION_COMMITMENT_SCHEMA,
};
use crate::definition_grounding::{
    DefinitionGrounder, DefinitionGroundingDispositionIR, DefinitionGroundingIR,
};
use crate::discourse_focus::{DiscourseFocusCandidateIR, DiscourseFocusSourceIR};
use crate::discourse_qa::{DiscourseAnswerIR, DiscourseQaEngine};
use crate::discourse_relations::{DialogueRelationAnswerIR, DialogueRelationQaEngine};
use crate::generative_language::{
    validate_interaction_boundary_generation_source, GenerationClarificationKindIR,
};
use crate::grounded_realization::{
    build_evidence_grounded_realization, EvidenceGroundedRealizationIR, GroundedRealizationSources,
};
use crate::interaction_provenance::{
    build_interaction_provenance, InteractionProvenanceGraphIR, InteractionProvenanceSources,
};
use crate::knowledge_work::{
    execute_document_work_as_with_reasoning, infer_operation, DocumentKindIR, KnowledgeWorkError,
    KnowledgeWorkOperationIR, KnowledgeWorkProductIR, KnowledgeWorkRequestIR,
    KNOWLEDGE_WORK_RESPONSE_SCHEMA,
};
use crate::language_cortex_integration::{
    build_language_cortex_response_integration, LanguageCortexResponseIntegrationIR,
    LanguageCortexResponseSources,
};
use crate::language_knowledge::{
    LanguageCodeIR, LanguageDialogueDirectiveAnalysisIR, LanguageDialogueDirectiveAxisIR,
    LanguageDialogueDirectiveValueIR, LanguageKnowledgeBase, LanguageKnowledgeEntryIR,
    LanguageKnowledgeError, LanguageKnowledgeStatisticsIR, LanguageUnderstandingIR,
};
use crate::lexical_memory::{
    ActivatedSenseIR, LexemeIR, LexemeSnapshotIR, LexicalMemory, LexicalMemoryError,
    LexicalMemoryStatisticsIR, LexicalOutcomeIR,
};
use crate::long_term_repair::{
    process_long_term_repair_plan, LongTermRepairPlanError, LongTermRepairPlanRequestIR,
    LongTermRepairPlanResponseIR,
};
use crate::mechanism_induction::{
    MechanismInductionDispositionIR, MechanismInductionEngine, MechanismInductionError,
    MechanismInductionIR, MechanismInductionRequestIR,
};
use crate::native_language_circuit::{
    contains_explicit_prohibition, NativeContextEntityIR, NativeContextGoalIR,
    NativeContextReferentIR, NativeDialogueContextIR, NativeEventScopeIR, NativeLanguageCircuit,
    NativeReferenceKindIR, NativeResponseGoalIR, NativeResponseModeIR, NativeTurnIR,
};
use crate::natural_realization::{
    arbitrate_natural_response, build_natural_realization, ContinuationGateRealizationSourceIR,
    NaturalRealizationIR, NaturalRealizationSources, NaturalResponseActIR,
    NaturalResponseCandidateIR, NaturalResponseSourceIR,
};
use crate::plan_result_boundary::{
    build_plan_result_boundary, classify_plan_result_query_focus, PlanResultBoundaryIR,
    PlanResultQueryFocusIR,
};
use crate::pragmatic_intent::PragmaticIntentKindIR;
use crate::pragmatic_memory::{
    PendingContinuationGateIR, PragmaticMemory, PragmaticMemoryError, PragmaticMemoryStateIR,
};
use crate::pragmatics::{
    requests_epistemic_record_update, requests_future_epistemic_notification, ActiveGoalContextIR,
    GoalWithdrawalScopeIR, IllocutionaryForceIR, PendingDeferredContextIR, PragmaticContextIR,
    PragmaticInterpretationIR, PragmaticReasoner, SpeechActIR, UserFeedbackKindIR,
};
use crate::professional_document::{
    process_professional_document, ProfessionalDocumentError, ProfessionalDocumentRequestIR,
    ProfessionalDocumentResponseIR,
};
use crate::raw_mechanism_induction::{
    RawMechanismInductionEngine, RawMechanismInductionError, RawMechanismInductionIR,
    RawMechanismInductionRequestIR,
};
use crate::semantic_roles::SemanticRoleKindIR;
use crate::six_axis_integration::{
    build_six_axis_integration, SixAxisIntegrationIR, SixAxisIntegrationSources,
};
use crate::temporal::{
    TemporalAnswerIR, TemporalQaEngine, TemporalSemanticAnalyzer, TemporalTurnAnalysisIR,
};
use crate::utterance_intent::CommunicativeIntentIR;

pub const NATURAL_LANGUAGE_REQUEST_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_REQUEST_1";
pub const NATURAL_LANGUAGE_RESPONSE_SCHEMA: &str = "B_CORE_NATURAL_LANGUAGE_RESPONSE_2";
pub const CONVERSATION_TURN_RESPONSE_SCHEMA: &str = "B_CORE_CONVERSATION_TURN_RESPONSE_18";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageRequestIR {
    pub schema: String,
    pub request_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_language: Option<LanguageCodeIR>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    pub max_plan_steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageOutputIR {
    pub language: LanguageCodeIR,
    pub text: String,
    pub grounded_plan_sha256: String,
    pub unsupported_freeform_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalLanguageResponseIR {
    pub schema: String,
    pub request_id: String,
    pub understanding: LanguageUnderstandingIR,
    pub pragmatic_interpretation: PragmaticInterpretationIR,
    pub lexical_activations: Vec<ActivatedSenseIR>,
    pub semantic_goal: SemanticPlanGoalIR,
    pub semantic_plan_bundle: SemanticPlanBundleIR,
    /// Compatibility projection of the first selected semantic event.
    pub plan: PlanIR,
    pub output: NaturalLanguageOutputIR,
}

impl NaturalLanguageResponseIR {
    pub fn validate(&self) -> bool {
        self.schema == NATURAL_LANGUAGE_RESPONSE_SCHEMA
            && self.semantic_goal.validate()
            && self
                .semantic_plan_bundle
                .validate_against(&self.semantic_goal)
            && self.semantic_plan_bundle.primary_plan() == Some(&self.plan)
            && self.output.grounded_plan_sha256 == self.plan.plan_sha256
            && self.output.unsupported_freeform_claims == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationalOutputIR {
    pub language: LanguageCodeIR,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounded_plan_sha256: Option<String>,
    pub unsupported_freeform_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationTurnResponseIR {
    pub schema: String,
    pub conversation_id: String,
    pub turn_index: u64,
    pub disposition: ConversationTurnDispositionIR,
    pub normalization: NormalizedUtteranceIR,
    pub native_language_circuit: NativeTurnIR,
    #[serde(default)]
    pub definition_grounding: DefinitionGroundingIR,
    pub reference_resolution: ReferenceResolutionIR,
    pub pragmatic_interpretation: PragmaticInterpretationIR,
    pub action_state_analysis: ActionStateAnalysisIR,
    pub plan_result_boundary: PlanResultBoundaryIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_group_update: Option<DiscourseGroupUpdateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_transition: Option<TopicTransitionIR>,
    pub pragmatic_state: PragmaticMemoryStateIR,
    pub conversation_state: ConversationStateIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounded_response: Option<Box<NaturalLanguageResponseIR>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discourse_answer: Option<DiscourseAnswerIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_relation_answer: Option<DialogueRelationAnswerIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_answer: Option<TemporalAnswerIR>,
    #[serde(default)]
    pub conditional_guard_evaluations: Vec<ConditionalGuardEvaluationIR>,
    pub natural_realization: NaturalRealizationIR,
    pub grounded_realization: EvidenceGroundedRealizationIR,
    pub interaction_provenance: InteractionProvenanceGraphIR,
    pub six_axis_integration: SixAxisIntegrationIR,
    pub language_cortex_integration: LanguageCortexResponseIntegrationIR,
    pub output: ConversationalOutputIR,
}

impl ConversationTurnResponseIR {
    pub fn validate_against(&self, request: &ConversationTurnRequestIR) -> bool {
        self.schema == CONVERSATION_TURN_RESPONSE_SCHEMA
            && self.conversation_id == request.conversation_id
            && self.turn_index == request.turn_index
            && self
                .pragmatic_interpretation
                .pragmatic_intent_graph
                .utterance_intent
                .validate_source(&self.normalization.semantic_surface_text)
            && self
                .native_language_circuit
                .validate_for_source(authoritative_native_source(
                    request,
                    &self.reference_resolution,
                ))
            && self
                .grounded_response
                .as_deref()
                .is_none_or(NaturalLanguageResponseIR::validate)
            && (self.natural_realization.response_act != NaturalResponseActIR::PlanPreview
                || is_quoted_metalinguistic_request(&self.normalization.semantic_surface_text)
                || semantic_plan_matches_current_turn_memory(
                    self.grounded_response.as_deref(),
                    &self.conversation_state,
                    request.turn_index,
                ))
            && self.natural_realization.coverage.validate_against(
                &self.natural_realization.response_plan,
                &self.natural_realization.generation_traces,
                if self.natural_realization.response_act == NaturalResponseActIR::PlanPreview {
                    self.grounded_response
                        .as_deref()
                        .map(|response| &response.semantic_goal)
                } else {
                    None
                },
            )
            && self.reference_resolution.resolution_graph.validate_against(
                &self.reference_resolution.original_semantic_text,
                &self.reference_resolution.resolved_semantic_text,
                self.reference_resolution.discourse_bindings.len(),
            )
            && self.plan_result_boundary.validate_against(
                &self.normalization.semantic_surface_text,
                &self.action_state_analysis,
                &self.conversation_state.action_state_ledger,
            )
            && self
                .language_cortex_integration
                .validate_against(LanguageCortexResponseSources {
                    request,
                    disposition: self.disposition,
                    normalization: &self.normalization,
                    definition_grounding: &self.definition_grounding,
                    reference_resolution: &self.reference_resolution,
                    pragmatic_interpretation: &self.pragmatic_interpretation,
                    action_state_analysis: &self.action_state_analysis,
                    plan_result_boundary: &self.plan_result_boundary,
                    discourse_group_update: self.discourse_group_update.as_ref(),
                    topic_transition: self.topic_transition.as_ref(),
                    pragmatic_state: &self.pragmatic_state,
                    conversation_state: &self.conversation_state,
                    grounded_response: self.grounded_response.as_deref(),
                    discourse_answer: self.discourse_answer.as_ref(),
                    dialogue_relation_answer: self.dialogue_relation_answer.as_ref(),
                    temporal_answer: self.temporal_answer.as_ref(),
                    conditional_guard_evaluations: &self.conditional_guard_evaluations,
                    natural_realization: &self.natural_realization,
                    grounded_realization: &self.grounded_realization,
                    interaction_provenance: &self.interaction_provenance,
                    six_axis_integration: &self.six_axis_integration,
                    output: &self.output,
                })
    }
}

fn semantic_plan_matches_current_turn_memory(
    grounded_response: Option<&NaturalLanguageResponseIR>,
    conversation_state: &ConversationStateIR,
    turn_index: u64,
) -> bool {
    let Some(response) = grounded_response else {
        return true;
    };
    let semantic_goal = &response.semantic_goal;
    let selected = semantic_goal
        .selected_live_event_ids
        .iter()
        .filter_map(|event_id| {
            semantic_goal
                .events
                .iter()
                .find(|event| &event.event_id == event_id)
        })
        .map(|event| {
            let subject = event
                .goal_subject_argument_ids
                .iter()
                .filter_map(|argument_id| {
                    semantic_goal
                        .arguments
                        .iter()
                        .find(|argument| &argument.argument_id == argument_id)
                        .map(|argument| argument.grounded_label.trim().to_lowercase())
                })
                .collect::<Vec<_>>()
                .join(" & ");
            (
                event.intent,
                event
                    .predicate_concept_id
                    .strip_prefix("C_")
                    .unwrap_or(&event.predicate_concept_id)
                    .to_string(),
                subject,
            )
        })
        .collect::<Vec<_>>();
    let mut remembered_goals = conversation_state
        .active_goals
        .iter()
        .filter(|goal| goal.introduced_turn == turn_index)
        .collect::<Vec<_>>();
    remembered_goals.sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
    selected.len() == remembered_goals.len()
        && selected
            .iter()
            .zip(&remembered_goals)
            .all(|((intent, predicate, subject), goal)| {
                *intent == goal.intent
                    && predicate == &goal.canonical_predicate
                    && subjects_semantically_overlap(subject, &goal.subject)
            })
}

/// Select the one textual evidence surface consumed by the native parser.
///
/// Reference resolution is authoritative only when it actually produced an
/// unambiguous binding.  Feeding its compatibility surface to every turn made
/// a no-op resolver an accidental second normalizer and changed predicate and
/// entity spans before central semantic arbitration.  Conversely, parsing the
/// raw deictic surface after a real binding lets stale context compete with the
/// resolved referent.  This boundary gives exactly one parser input to every
/// downstream native operation without granting either module overwrite
/// authority outside its own evidence.
fn authoritative_native_source<'a>(
    request: &'a ConversationTurnRequestIR,
    reference_resolution: &'a ReferenceResolutionIR,
) -> &'a str {
    if reference_resolution.resolved_reference_count > 0
        && reference_resolution.ambiguous_reference_surfaces.is_empty()
        && reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| {
                matches!(
                    binding.kind,
                    DiscourseBindingKindIR::OrderedReference
                        | DiscourseBindingKindIR::LocalOrderedReference
                        | DiscourseBindingKindIR::LocalOrdinalReference
                        | DiscourseBindingKindIR::EllipticalAction
                        | DiscourseBindingKindIR::DiscourseProgramInstantiation
                        | DiscourseBindingKindIR::RepeatedGoal
                        | DiscourseBindingKindIR::CorrectedArgument
                        | DiscourseBindingKindIR::EventReference
                        | DiscourseBindingKindIR::EventOrdinalReference
                        | DiscourseBindingKindIR::PluralEventReference
                        | DiscourseBindingKindIR::PluralEventMemberReference
                        | DiscourseBindingKindIR::ResultReference
                        | DiscourseBindingKindIR::PropositionReference
                        | DiscourseBindingKindIR::PluralPropositionReference
                        | DiscourseBindingKindIR::TopicReference
                        | DiscourseBindingKindIR::TopicAnchoredActionGroupReference
                        | DiscourseBindingKindIR::TopicAnchoredActionMemberReference
                )
            })
    {
        &reference_resolution.resolved_semantic_text
    } else {
        &request.raw_text
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeWorkResponseIR {
    pub schema: String,
    pub request_id: String,
    pub understanding: LanguageUnderstandingIR,
    pub lexical_activations: Vec<ActivatedSenseIR>,
    pub plan: PlanIR,
    pub product: KnowledgeWorkProductIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MechanismInductionResponseIR {
    pub induction: MechanismInductionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injection_receipt: Option<MechanismKnowledgeInjectionReceiptIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMechanismInductionResponseIR {
    pub induction: RawMechanismInductionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injection_receipt: Option<MechanismKnowledgeInjectionReceiptIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiCommandIR {
    DeliberateProblem {
        request: DeliberationRequestIR,
    },
    ReviseDeliberation {
        request: Box<DeliberationRevisionRequestIR>,
    },
    InjectMechanismKnowledge {
        knowledge: MechanismKnowledgeIR,
    },
    ExportMechanismMemorySnapshot,
    ImportMechanismMemorySnapshot {
        snapshot: MechanismMemorySnapshotIR,
    },
    DeliberateWithKnowledge {
        request: DeliberationRequestIR,
        query: MechanismQueryIR,
    },
    InduceAndInjectMechanismKnowledge {
        request: Box<MechanismInductionRequestIR>,
    },
    InduceAndInjectRawMechanismKnowledge {
        request: Box<RawMechanismInductionRequestIR>,
    },
    InjectExperience {
        experience: ExperienceIR,
    },
    ExportExperienceSnapshot,
    ImportExperienceSnapshot {
        snapshot: ExperienceSnapshotIR,
    },
    InjectLanguageKnowledge {
        entry: LanguageKnowledgeEntryIR,
    },
    InjectLexeme {
        lexeme: LexemeIR,
    },
    ExportLexemeSnapshot,
    ImportLexemeSnapshot {
        snapshot: LexemeSnapshotIR,
    },
    RecordLexicalOutcome {
        outcome: LexicalOutcomeIR,
    },
    ProcessNaturalLanguage {
        request: NaturalLanguageRequestIR,
    },
    ProcessConversationTurn {
        request: ConversationTurnRequestIR,
    },
    SubmitConditionEvidence {
        request: ConditionEvidenceRequestIR,
    },
    SubmitActionEvidence {
        request: ActionEvidenceRequestIR,
    },
    ProcessKnowledgeWork {
        request: KnowledgeWorkRequestIR,
    },
    ProcessLongTermRepairPlan {
        request: LongTermRepairPlanRequestIR,
    },
    ProcessProfessionalDocument {
        request: ProfessionalDocumentRequestIR,
    },
    LanguageKnowledgeStatistics,
    LexicalMemoryStatistics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiPayloadIR {
    Deliberation(Box<DeliberationIR>),
    DeliberationRevision(Box<DeliberationRevisionIR>),
    KnowledgeGroundedDeliberation(Box<KnowledgeGroundedDeliberationIR>),
    MechanismKnowledgeInjectionReceipt(MechanismKnowledgeInjectionReceiptIR),
    MechanismKnowledgeInjectionReceipts(Vec<MechanismKnowledgeInjectionReceiptIR>),
    MechanismMemorySnapshot(MechanismMemorySnapshotIR),
    MechanismInductionResponse(Box<MechanismInductionResponseIR>),
    RawMechanismInductionResponse(Box<RawMechanismInductionResponseIR>),
    ExperienceInjectionReceipt(ExperienceInjectionReceiptIR),
    ExperienceInjectionReceipts(Vec<ExperienceInjectionReceiptIR>),
    ExperienceSnapshot(ExperienceSnapshotIR),
    LanguageKnowledgeInserted(bool),
    LexemeInserted(bool),
    LexemeSnapshot(LexemeSnapshotIR),
    LexemeSnapshotImported,
    LexicalOutcomeRecorded,
    NaturalLanguageResponse(Box<NaturalLanguageResponseIR>),
    ConversationTurnResponse(Box<ConversationTurnResponseIR>),
    ConditionEvidenceReceipt(ConditionEvidenceReceiptIR),
    ActionEvidenceReceipt(ActionEvidenceReceiptIR),
    KnowledgeWorkResponse(Box<KnowledgeWorkResponseIR>),
    LongTermRepairPlanResponse(Box<LongTermRepairPlanResponseIR>),
    ProfessionalDocumentResponse(Box<ProfessionalDocumentResponseIR>),
    LanguageKnowledgeStatistics(LanguageKnowledgeStatisticsIR),
    LexicalMemoryStatistics(LexicalMemoryStatisticsIR),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveApiResponseIR {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<CognitiveApiPayloadIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CognitiveApiError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CognitiveApiError {
    CoreLoad,
    InvalidRequest,
    LanguageKnowledge,
    ConversationFrontend,
    ConditionEvidence,
    ActionEvidence,
    PragmaticMemory,
    CompositionalPredicate,
    LexicalMemory,
    KnowledgeWork,
    LongTermRepairPlan,
    ProfessionalDocument,
    Deliberation,
    MechanismMemory,
    MechanismInduction,
    RawMechanismInduction,
    Experience,
    Planning,
    JsonInput,
    JsonOutput,
}

/// A module may only contribute one of these typed facts. It cannot directly
/// enable/disable a peer analyzer, project a plan, or select a response. The
/// central routing receipt below is the sole interpreter of those facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LanguagePipelineSignalIR {
    GroupUpdateOwnsTurn,
    DefinitionOwnsTurn,
    DialogueDirectiveOwnsTurn,
    FutureNotificationOwnsTurn,
    NativeGoalOwnsTurn,
    ActionStateCandidate,
    ActionStateOwnsTurn,
    PlanResultCandidate,
    PlanResultOwnsTurn,
    PragmaticForceOwnsSurfaceQuestion,
    ResultReferenceOwnsTurn,
    InitialContinuationGateOwnsTurn,
    ExplicitSelectedRequest,
    ResponseGoalCorrection,
    NormalizedGrounded,
    AmbiguousInput,
    DeicticQueryReferenceSafe,
    ReferencesFullyResolved,
    GroundedDisposition,
    SemanticGoalAvailable,
    DiscourseGroupUpdateApplied,
    QuestionAnswer,
    TopicTransitionApplied,
    TopicTransitionOwnsTurn,
    PendingContinuationGateDecision,
    ProxyEvidenceUpdate,
    InteractionBoundaryOwnsTurn,
    SocialOnly,
    FeedbackOnly,
    AffectOnly,
    InformOnly,
    ConditionalGuardEvidenceCandidate,
    ConditionalGuardOwnsTurn,
}

/// The single routing receipt shared by QA, temporal analysis, semantic-plan
/// projection, memory commit, and final response arbitration. A `BTreeSet`
/// makes the decision independent of analyzer invocation order and prevents a
/// later module from overwriting an earlier module's evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LanguagePipelineRoutingIR {
    active: BTreeSet<LanguagePipelineSignalIR>,
}

impl LanguagePipelineRoutingIR {
    fn from_candidates(
        candidates: impl IntoIterator<Item = Option<LanguagePipelineSignalIR>>,
    ) -> Self {
        Self {
            active: candidates.into_iter().flatten().collect(),
        }
    }

    fn activate_if(&mut self, active: bool, signal: LanguagePipelineSignalIR) {
        if active {
            self.active.insert(signal);
        }
    }

    fn has(&self, signal: LanguagePipelineSignalIR) -> bool {
        self.active.contains(&signal)
    }

    fn common_qa_path_open(&self) -> bool {
        !self.has(LanguagePipelineSignalIR::GroupUpdateOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::DefinitionOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::DialogueDirectiveOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::FutureNotificationOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::NativeGoalOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::ActionStateOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::PragmaticForceOwnsSurfaceQuestion)
            && !self.has(LanguagePipelineSignalIR::ResultReferenceOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::InitialContinuationGateOwnsTurn)
            && self.has(LanguagePipelineSignalIR::NormalizedGrounded)
            && !self.has(LanguagePipelineSignalIR::AmbiguousInput)
    }

    fn allows_temporal_qa(&self) -> bool {
        self.common_qa_path_open()
            && !self.has(LanguagePipelineSignalIR::PlanResultOwnsTurn)
            && self.has(LanguagePipelineSignalIR::DeicticQueryReferenceSafe)
    }

    fn allows_dialogue_relation_qa(&self, temporal_answer_present: bool) -> bool {
        self.common_qa_path_open()
            && !self.has(LanguagePipelineSignalIR::ExplicitSelectedRequest)
            && !self.has(LanguagePipelineSignalIR::ResponseGoalCorrection)
            && self.has(LanguagePipelineSignalIR::ReferencesFullyResolved)
            && !temporal_answer_present
    }

    fn allows_discourse_qa(
        &self,
        temporal_answer_present: bool,
        dialogue_relation_answer_present: bool,
    ) -> bool {
        self.common_qa_path_open()
            && !self.has(LanguagePipelineSignalIR::ExplicitSelectedRequest)
            && !self.has(LanguagePipelineSignalIR::ResponseGoalCorrection)
            && self.has(LanguagePipelineSignalIR::DeicticQueryReferenceSafe)
            && !temporal_answer_present
            && !dialogue_relation_answer_present
    }

    fn allows_temporal_analysis(&self) -> bool {
        !self.has(LanguagePipelineSignalIR::GroupUpdateOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::DefinitionOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::DialogueDirectiveOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::ActionStateOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::PlanResultOwnsTurn)
            && !self.has(LanguagePipelineSignalIR::QuestionAnswer)
            && self.has(LanguagePipelineSignalIR::NormalizedGrounded)
            && !self.has(LanguagePipelineSignalIR::AmbiguousInput)
    }
}

/// Central decision receipt for semantic plan projection. Analyzer modules may
/// contribute blockers, but none of them may render a response or mutate a
/// previously selected plan. Keeping every blocker makes route selection
/// inspectable without restoring the old first-matching-module control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PlanProjectionBlockerIR {
    NonGroundedDisposition,
    NoSemanticGoal,
    DefinitionGrounding,
    DialogueDirective,
    DiscourseGroupUpdate,
    FutureEpistemicNotification,
    PlanResultQuery,
    ActionState,
    QuestionAnswer,
    TopicTransition,
    ContinuationGate,
    ProxyEvidence,
    InteractionBoundary,
    ResultReference,
    SocialOnly,
    FeedbackOnly,
    AffectOnly,
    InformOnly,
    ConditionalGuard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlanProjectionDecisionIR {
    blockers: Vec<PlanProjectionBlockerIR>,
}

impl PlanProjectionDecisionIR {
    fn from_candidates(
        candidates: impl IntoIterator<Item = Option<PlanProjectionBlockerIR>>,
    ) -> Self {
        let mut blockers = candidates.into_iter().flatten().collect::<Vec<_>>();
        blockers.sort();
        blockers.dedup();
        Self { blockers }
    }

    fn allows_plan(&self) -> bool {
        self.blockers.is_empty()
    }

    fn from_routing(routing: &LanguagePipelineRoutingIR) -> Self {
        use LanguagePipelineSignalIR as Signal;

        Self::from_candidates([
            (!routing.has(Signal::GroundedDisposition))
                .then_some(PlanProjectionBlockerIR::NonGroundedDisposition),
            (!routing.has(Signal::SemanticGoalAvailable))
                .then_some(PlanProjectionBlockerIR::NoSemanticGoal),
            routing
                .has(Signal::DefinitionOwnsTurn)
                .then_some(PlanProjectionBlockerIR::DefinitionGrounding),
            routing
                .has(Signal::DialogueDirectiveOwnsTurn)
                .then_some(PlanProjectionBlockerIR::DialogueDirective),
            routing
                .has(Signal::DiscourseGroupUpdateApplied)
                .then_some(PlanProjectionBlockerIR::DiscourseGroupUpdate),
            routing
                .has(Signal::FutureNotificationOwnsTurn)
                .then_some(PlanProjectionBlockerIR::FutureEpistemicNotification),
            routing
                .has(Signal::PlanResultOwnsTurn)
                .then_some(PlanProjectionBlockerIR::PlanResultQuery),
            routing
                .has(Signal::ActionStateOwnsTurn)
                .then_some(PlanProjectionBlockerIR::ActionState),
            routing
                .has(Signal::QuestionAnswer)
                .then_some(PlanProjectionBlockerIR::QuestionAnswer),
            routing
                .has(Signal::TopicTransitionOwnsTurn)
                .then_some(PlanProjectionBlockerIR::TopicTransition),
            routing
                .has(Signal::PendingContinuationGateDecision)
                .then_some(PlanProjectionBlockerIR::ContinuationGate),
            routing
                .has(Signal::ProxyEvidenceUpdate)
                .then_some(PlanProjectionBlockerIR::ProxyEvidence),
            routing
                .has(Signal::InteractionBoundaryOwnsTurn)
                .then_some(PlanProjectionBlockerIR::InteractionBoundary),
            routing
                .has(Signal::ResultReferenceOwnsTurn)
                .then_some(PlanProjectionBlockerIR::ResultReference),
            routing
                .has(Signal::SocialOnly)
                .then_some(PlanProjectionBlockerIR::SocialOnly),
            routing
                .has(Signal::FeedbackOnly)
                .then_some(PlanProjectionBlockerIR::FeedbackOnly),
            routing
                .has(Signal::AffectOnly)
                .then_some(PlanProjectionBlockerIR::AffectOnly),
            routing
                .has(Signal::InformOnly)
                .then_some(PlanProjectionBlockerIR::InformOnly),
            routing
                .has(Signal::ConditionalGuardEvidenceCandidate)
                .then_some(PlanProjectionBlockerIR::ConditionalGuard),
            routing
                .has(Signal::ConditionalGuardOwnsTurn)
                .then_some(PlanProjectionBlockerIR::ConditionalGuard),
        ])
    }
}

fn interaction_boundary_required(interpretation: &PragmaticInterpretationIR) -> bool {
    if interpretation.continuation_gate.is_some() {
        return false;
    }
    validate_interaction_boundary_generation_source(&interpretation.illocutionary_commitments)
}

/// Local, deterministic public API for natural-language planning and bounded
/// experience injection. Language interpretation proposes typed IR; the core
/// planner remains the plan authority and every output sentence is rendered
/// from that validated IR.
pub struct CognitiveApi {
    core: DockableCore,
    language_knowledge: LanguageKnowledgeBase,
    lexical_memory: LexicalMemory,
    utterance_normalizer: UtteranceNormalizer,
    pragmatic_reasoner: PragmaticReasoner,
    discourse_qa: DiscourseQaEngine,
    dialogue_relation_qa: DialogueRelationQaEngine,
    temporal_analyzer: TemporalSemanticAnalyzer,
    temporal_qa: TemporalQaEngine,
    compositional_predicates: Vec<PredicateLexemeIR>,
    pragmatic_memory: PragmaticMemory,
    conversation_memory: ConversationMemory,
    native_dialogue_memory: BTreeMap<String, NativeDialogueContextIR>,
    mechanism_induction: MechanismInductionEngine,
    raw_mechanism_induction: RawMechanismInductionEngine,
}

impl CognitiveApi {
    pub fn new_embedded() -> Result<Self, CognitiveApiError> {
        Ok(Self {
            core: DockableCore::load_embedded().map_err(|_| CognitiveApiError::CoreLoad)?,
            language_knowledge: LanguageKnowledgeBase::default(),
            lexical_memory: LexicalMemory::default(),
            utterance_normalizer: UtteranceNormalizer,
            pragmatic_reasoner: PragmaticReasoner,
            discourse_qa: DiscourseQaEngine,
            dialogue_relation_qa: DialogueRelationQaEngine,
            temporal_analyzer: TemporalSemanticAnalyzer,
            temporal_qa: TemporalQaEngine,
            compositional_predicates: Vec::new(),
            pragmatic_memory: PragmaticMemory::default(),
            conversation_memory: ConversationMemory::default(),
            native_dialogue_memory: BTreeMap::new(),
            mechanism_induction: MechanismInductionEngine,
            raw_mechanism_induction: RawMechanismInductionEngine,
        })
    }

    pub fn inject_experience(
        &mut self,
        experience: ExperienceIR,
    ) -> Result<ExperienceInjectionReceiptIR, CognitiveApiError> {
        self.core
            .inject_experience(experience)
            .map_err(map_experience_error)
    }

    /// Runs the core's bounded causal/epistemic deliberation path. The result
    /// is a typed plan and evidence receipt; this API does not execute external
    /// actions or silently broaden the request's authority.
    pub fn deliberate_problem(
        &self,
        request: &DeliberationRequestIR,
    ) -> Result<DeliberationIR, CognitiveApiError> {
        self.core
            .deliberate_problem(request)
            .map_err(map_deliberation_error)
    }

    pub fn revise_deliberation(
        &self,
        request: &DeliberationRevisionRequestIR,
    ) -> Result<DeliberationRevisionIR, CognitiveApiError> {
        self.core
            .revise_deliberation(request)
            .map_err(map_deliberation_error)
    }

    pub fn inject_mechanism_knowledge(
        &mut self,
        knowledge: MechanismKnowledgeIR,
    ) -> Result<MechanismKnowledgeInjectionReceiptIR, CognitiveApiError> {
        self.core
            .inject_mechanism_knowledge(knowledge)
            .map_err(map_mechanism_memory_error)
    }

    pub fn deliberate_with_knowledge(
        &self,
        request: &DeliberationRequestIR,
        query: &MechanismQueryIR,
    ) -> Result<KnowledgeGroundedDeliberationIR, CognitiveApiError> {
        self.core
            .deliberate_with_knowledge(request, query)
            .map_err(map_mechanism_memory_error)
    }

    /// Compiles a language-bound causal claim only when repeated state
    /// transitions and controls support it, then inserts the resulting typed
    /// mechanism into executable memory. Insufficient or contradictory input
    /// returns a non-compiled receipt and performs no memory mutation.
    pub fn induce_and_inject_mechanism_knowledge(
        &mut self,
        request: &MechanismInductionRequestIR,
    ) -> Result<MechanismInductionResponseIR, CognitiveApiError> {
        let induction = self
            .mechanism_induction
            .compile(request)
            .map_err(map_mechanism_induction_error)?;
        let injection_receipt =
            if induction.disposition == MechanismInductionDispositionIR::Compiled {
                Some(
                    self.core
                        .inject_mechanism_knowledge(
                            induction
                                .knowledge
                                .clone()
                                .ok_or(CognitiveApiError::MechanismInduction)?,
                        )
                        .map_err(map_mechanism_memory_error)?,
                )
            } else {
                None
            };
        Ok(MechanismInductionResponseIR {
            induction,
            injection_receipt,
        })
    }

    /// Builds the proposition vocabulary and typed observations from bounded
    /// raw scalar state maps, then uses the same evidence-bound induction and
    /// executable-memory path as the explicit typed API.
    pub fn induce_and_inject_raw_mechanism_knowledge(
        &mut self,
        request: &RawMechanismInductionRequestIR,
    ) -> Result<RawMechanismInductionResponseIR, CognitiveApiError> {
        let induction = self
            .raw_mechanism_induction
            .compile(request)
            .map_err(map_raw_mechanism_induction_error)?;
        let injection_receipt =
            if induction.induction.disposition == MechanismInductionDispositionIR::Compiled {
                Some(
                    self.core
                        .inject_mechanism_knowledge(
                            induction
                                .induction
                                .knowledge
                                .clone()
                                .ok_or(CognitiveApiError::RawMechanismInduction)?,
                        )
                        .map_err(map_mechanism_memory_error)?,
                )
            } else {
                None
            };
        Ok(RawMechanismInductionResponseIR {
            induction,
            injection_receipt,
        })
    }

    pub fn inject_experience_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let experience =
            serde_json::from_str::<ExperienceIR>(json).map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.inject_experience(experience)?)
            .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn export_experience_snapshot_json(&self) -> Result<String, CognitiveApiError> {
        serde_json::to_string(&self.core.export_experience_snapshot())
            .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn import_experience_snapshot_json(
        &mut self,
        json: &str,
    ) -> Result<String, CognitiveApiError> {
        let snapshot = serde_json::from_str::<ExperienceSnapshotIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(
            &self
                .core
                .import_experience_snapshot(&snapshot)
                .map_err(map_experience_error)?,
        )
        .map_err(|_| CognitiveApiError::JsonOutput)
    }

    pub fn inject_language_knowledge(
        &mut self,
        entry: LanguageKnowledgeEntryIR,
    ) -> Result<bool, CognitiveApiError> {
        self.language_knowledge
            .inject(entry)
            .map_err(map_language_error)
    }

    pub fn inject_compositional_predicate(
        &mut self,
        predicate: PredicateLexemeIR,
    ) -> Result<bool, CognitiveApiError> {
        predicate.validate().map_err(map_predicate_lexeme_error)?;
        if let Some(existing) = self
            .compositional_predicates
            .iter()
            .find(|existing| existing.predicate_id == predicate.predicate_id)
        {
            return if existing == &predicate {
                Ok(false)
            } else {
                Err(CognitiveApiError::CompositionalPredicate)
            };
        }
        self.compositional_predicates.push(predicate);
        self.compositional_predicates
            .sort_by(|left, right| left.predicate_id.cmp(&right.predicate_id));
        Ok(true)
    }

    pub fn export_compositional_predicates(&self) -> PredicateLexiconSnapshotIR {
        PredicateLexiconSnapshotIR::build(self.compositional_predicates.clone())
            .expect("validated in-memory predicate lexicon must serialize")
    }

    pub fn import_compositional_predicates(
        &mut self,
        snapshot: &PredicateLexiconSnapshotIR,
    ) -> Result<(), CognitiveApiError> {
        snapshot.validate().map_err(map_predicate_lexeme_error)?;
        self.compositional_predicates = snapshot.entries.clone();
        Ok(())
    }

    pub fn process(
        &mut self,
        request: &NaturalLanguageRequestIR,
    ) -> Result<NaturalLanguageResponseIR, CognitiveApiError> {
        let pragmatic_interpretation = self.pragmatic_reasoner.interpret_with_predicates(
            &request.text,
            &PragmaticContextIR::default(),
            &self.compositional_predicates,
        );
        let native_language_circuit = NativeLanguageCircuit.analyze(&request.text);
        self.process_with_pragmatics(
            request,
            pragmatic_interpretation,
            Some(&native_language_circuit),
        )
    }

    fn process_with_pragmatics(
        &mut self,
        request: &NaturalLanguageRequestIR,
        pragmatic_interpretation: PragmaticInterpretationIR,
        native_language_circuit: Option<&NativeTurnIR>,
    ) -> Result<NaturalLanguageResponseIR, CognitiveApiError> {
        validate_request(request)?;
        let mut understanding = self
            .language_knowledge
            .understand(&request.text)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.text, &request.context_tags);
        merge_lexical_activations(&mut understanding, &lexical_activations);
        understanding
            .semantic_tags
            .extend(request.context_tags.iter().cloned());
        pragmatic_interpretation.apply_to_understanding(&mut understanding);
        let native_goal_projection_applied = native_language_circuit.is_some_and(|native| {
            native_goal_projection_required(&pragmatic_interpretation, native)
        });
        if let Some(goals) = native_language_circuit
            .filter(|_| native_goal_projection_applied)
            .and_then(NativeTurnIR::authoritative_live_goals)
        {
            let primary = &goals[0];
            understanding.intent = primary.intent;
            understanding.subject = goals
                .iter()
                .map(|goal| goal.subject.as_str())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(" | ");
            understanding.constraints.extend(
                native_language_circuit
                    .expect("goals came from native circuit")
                    .events
                    .iter()
                    .filter(|event| {
                        event.scope != crate::native_language_circuit::NativeEventScopeIR::Live
                    })
                    .map(|event| {
                        format!(
                            "NATIVE_NON_LIVE:{:?}:{}",
                            event.scope, event.canonical_predicate
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            understanding.constraints.sort();
            understanding.constraints.dedup();
            understanding.desired_outcomes = goals
                .iter()
                .map(|goal| format!("NATIVE_GOAL:{}:{}", goal.canonical_predicate, goal.subject))
                .collect();
            for goal in goals {
                understanding
                    .semantic_tags
                    .push(format!("NATIVE_CIRCUIT:GOAL:{}", goal.goal_id));
                understanding
                    .semantic_tags
                    .push(format!("NATIVE_PREDICATE:{}", goal.canonical_predicate));
                understanding.confidence_millis =
                    understanding.confidence_millis.max(goal.confidence_millis);
            }
        }
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let dialogue_directive_analysis = self
            .language_knowledge
            .analyze_dialogue_directives(&request.text);
        let planner_inferred_goal =
            planner_inferred_goal(&pragmatic_interpretation, &dialogue_directive_analysis);
        let semantic_goal = pragmatic_interpretation
            .language_center
            .to_semantic_plan_goal(
                &request.request_id,
                &understanding.semantic_tags,
                request.max_plan_steps,
                &pragmatic_interpretation.compositional_analysis,
                native_language_circuit,
                planner_inferred_goal,
            )
            .ok_or(CognitiveApiError::Planning)?;
        let semantic_plan_bundle = self
            .core
            .generate_semantic_plan(&semantic_goal)
            .map_err(map_planning_error)?;
        let plan = semantic_plan_bundle
            .primary_plan()
            .cloned()
            .ok_or(CognitiveApiError::Planning)?;
        let output_language = request
            .output_language
            .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
            .unwrap_or(match understanding.detected_language {
                LanguageCodeIR::Korean | LanguageCodeIR::Mixed => LanguageCodeIR::Korean,
                _ => LanguageCodeIR::English,
            });
        let output = render_plan(output_language, &understanding, &plan);
        Ok(NaturalLanguageResponseIR {
            schema: NATURAL_LANGUAGE_RESPONSE_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            understanding,
            pragmatic_interpretation,
            lexical_activations,
            semantic_goal,
            semantic_plan_bundle,
            plan,
            output,
        })
    }

    pub fn process_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let request = serde_json::from_str::<NaturalLanguageRequestIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.process(&request)?).map_err(|_| CognitiveApiError::JsonOutput)
    }

    /// Process one turn through the conversational surface layer before the
    /// existing language-independent planner. Empty fillers/backchannels never
    /// create a fake goal, and ambiguous ASR/reference bindings never guess.
    pub fn process_conversation_turn(
        &mut self,
        request: &ConversationTurnRequestIR,
    ) -> Result<ConversationTurnResponseIR, CognitiveApiError> {
        self.conversation_memory
            .validate_turn_order(request)
            .map_err(map_conversation_error)?;
        self.pragmatic_memory
            .validate_turn_order(request)
            .map_err(map_pragmatic_memory_error)?;
        let active_dialogue_directive_tags = self
            .conversation_memory
            .state(&request.conversation_id)
            .into_iter()
            .flat_map(|state| state.dialogue_directive_ledger.active())
            .map(dialogue_directive_tag)
            .collect::<Vec<_>>();
        let normalization = self
            .utterance_normalizer
            .normalize(request)
            .map_err(map_conversation_error)?;
        use LanguagePipelineSignalIR as PipelineSignal;
        let mut pipeline_routing = LanguagePipelineRoutingIR::from_candidates([
            (normalization.disposition == ConversationTurnDispositionIR::Grounded)
                .then_some(PipelineSignal::NormalizedGrounded),
            normalization
                .ambiguous_input
                .then_some(PipelineSignal::AmbiguousInput),
        ]);
        pipeline_routing.activate_if(
            requests_future_epistemic_notification(&normalization.semantic_surface_text),
            PipelineSignal::FutureNotificationOwnsTurn,
        );
        let mut native_dialogue_context = self
            .conversation_memory
            .state(&request.conversation_id)
            .map(|state| {
                let active_topic = state.active_topics.first();
                let mut goals = state
                    .action_state_ledger
                    .records
                    .iter()
                    .map(|record| NativeContextGoalIR {
                        goal_id: record.goal_id.clone(),
                        intent: native_intent_from_predicate(&record.canonical_predicate),
                        canonical_predicate: record.canonical_predicate.clone(),
                        subject: record.subject.clone(),
                        introduced_turn: record.introduced_turn,
                        discourse_focused: active_topic.is_some_and(|topic| {
                            topic_matches_subject(topic, &record.subject)
                                || record
                                    .subject
                                    .to_lowercase()
                                    .contains(&topic.surface.to_lowercase())
                        }),
                        operation_replayable: false,
                    })
                    .chain(state.active_goals.iter().map(|goal| NativeContextGoalIR {
                        goal_id: goal.goal_id.clone(),
                        intent: goal.intent,
                        canonical_predicate: goal.canonical_predicate.clone(),
                        subject: goal.subject.clone(),
                        introduced_turn: goal.introduced_turn,
                        discourse_focused: active_topic.is_some_and(|topic| {
                            topic_matches_subject(topic, &goal.subject)
                                || goal
                                    .subject
                                    .to_lowercase()
                                    .contains(&topic.surface.to_lowercase())
                        }),
                        operation_replayable: true,
                    }))
                    .collect::<Vec<_>>();
                goals.sort_by(|left, right| {
                    left.introduced_turn
                        .cmp(&right.introduced_turn)
                        .then_with(|| left.goal_id.cmp(&right.goal_id))
                        .then_with(|| right.operation_replayable.cmp(&left.operation_replayable))
                });
                goals.dedup_by(|left, right| left.goal_id == right.goal_id);
                let active_entities = if state.active_typed_entities.is_empty() {
                    state
                        .discourse_focus
                        .current_focus_id
                        .as_deref()
                        .and_then(|focus_id| {
                            state
                                .discourse_focus
                                .nodes
                                .iter()
                                .find(|node| node.focus_id == focus_id)
                        })
                        .map(|node| {
                            vec![NativeContextEntityIR {
                                referent_id: node.focus_id.clone(),
                                surface: node.surface.clone(),
                                introduced_turn: node.introduced_turn,
                                last_mentioned_turn: node.last_focused_turn,
                            }]
                        })
                        .unwrap_or_default()
                } else {
                    state
                        .active_typed_entities
                        .iter()
                        .map(|entity| NativeContextEntityIR {
                            referent_id: entity.entity_id.clone(),
                            surface: entity.canonical_surface.clone(),
                            introduced_turn: entity.introduced_turn,
                            last_mentioned_turn: entity.last_mentioned_turn,
                        })
                        .collect()
                };
                NativeDialogueContextIR {
                    active_goals: goals,
                    active_entities,
                    active_referents: state
                        .active_discourse_referents
                        .iter()
                        .map(|referent| NativeContextReferentIR {
                            referent_id: referent.referent_id.clone(),
                            semantic_summary: referent.semantic_summary.clone(),
                            introduced_turn: referent.introduced_turn,
                            last_referenced_turn: referent.last_referenced_turn,
                        })
                        .collect(),
                }
            })
            .unwrap_or_default();
        if let Some(remembered) = self.native_dialogue_memory.get(&request.conversation_id) {
            // Typed discourse state owns which entities remain active. Native
            // mention memory may only restore the user's phenotype spelling
            // for an already selected entity; it must not add another
            // candidate or change identity/salience.
            for active_entity in &mut native_dialogue_context.active_entities {
                if let Some(phenotype) = remembered.active_entities.iter().find(|candidate| {
                    candidate
                        .surface
                        .eq_ignore_ascii_case(&active_entity.surface)
                }) {
                    active_entity.surface = phenotype.surface.clone();
                }
            }
            for remembered_goal in &remembered.active_goals {
                let already_represented = native_dialogue_context.active_goals.iter().any(|goal| {
                    goal.introduced_turn == remembered_goal.introduced_turn
                        && goal.canonical_predicate == remembered_goal.canonical_predicate
                        && (!remembered_goal.operation_replayable || goal.operation_replayable)
                        && (goal.subject.eq_ignore_ascii_case(&remembered_goal.subject)
                            || crate::native_language_circuit::subjects_share_context_concept(
                                &goal.subject,
                                &remembered_goal.subject,
                            ))
                });
                if !already_represented {
                    native_dialogue_context
                        .active_goals
                        .push(remembered_goal.clone());
                }
            }
            native_dialogue_context.active_goals.sort_by(|left, right| {
                left.introduced_turn
                    .cmp(&right.introduced_turn)
                    .then_with(|| left.goal_id.cmp(&right.goal_id))
                    .then_with(|| right.operation_replayable.cmp(&left.operation_replayable))
            });
            native_dialogue_context
                .active_goals
                .dedup_by(|left, right| left.goal_id == right.goal_id);
            // The typed conversation state has already applied discourse
            // focus and salience. Raw native mention memory is a fallback,
            // never a replacement: replacing the focused set here used to
            // reintroduce every noun from the prior sentence and turn a unique
            // ellipsis target into a false ambiguity.
            if native_dialogue_context.active_entities.is_empty()
                && !remembered.active_entities.is_empty()
            {
                native_dialogue_context.active_entities = remembered.active_entities.clone();
                native_dialogue_context.active_referents.clear();
            }
        }
        let definition_grounding = DefinitionGrounder.ground(
            &normalization.semantic_surface_text,
            request.turn_index,
            &self.compositional_predicates,
        );
        if definition_grounding.lexical_store_changed {
            let predicate = definition_grounding
                .binding
                .as_ref()
                .expect("changed lexical binding must exist")
                .predicate_lexeme();
            self.inject_compositional_predicate(predicate)?;
        }
        debug_assert!(definition_grounding.validate());
        let definition_grounding_applies = definition_grounding.consumes_turn();
        let quoted_metalinguistic_request =
            is_quoted_metalinguistic_request(&normalization.semantic_surface_text);
        let discourse_group_update = self.conversation_memory.analyze_discourse_group_update(
            &request.conversation_id,
            &normalization.semantic_surface_text,
            request.turn_index,
        );
        pipeline_routing.activate_if(
            discourse_group_update.is_some(),
            PipelineSignal::GroupUpdateOwnsTurn,
        );
        let discourse_connected_backchannel = normalization.disposition
            == ConversationTurnDispositionIR::BackchannelOnly
            && self
                .conversation_memory
                .state(&request.conversation_id)
                .is_some_and(|state| {
                    state.discourse_focus.current().is_some_and(|focus| {
                        focus.source != DiscourseFocusSourceIR::ExplicitTopic
                            && state.discourse_focus.nodes.len() > 1
                    })
                });
        let topic_transition = self
            .conversation_memory
            .analyze_topic_transition_with_surface(
                &request.conversation_id,
                &normalization.semantic_surface_text,
                &request.raw_text,
                quoted_metalinguistic_request,
            );
        let unresolved_topic_pointer = topic_transition
            .as_ref()
            .is_some_and(|transition| !transition.applied);
        let pending_answer = if normalization.disposition == ConversationTurnDispositionIR::Grounded
            && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
        {
            self.conversation_memory.resolve_pending_question(
                &request.conversation_id,
                &normalization.semantic_surface_text,
            )
        } else {
            QuestionAnswerResolutionIR {
                disposition: QuestionAnswerDispositionIR::NotApplicable,
                resolved_semantic_text: normalization.semantic_surface_text.clone(),
                binding: None,
            }
        };
        let mut reference_resolution = if pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
        {
            ReferenceResolutionIR {
                original_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: Vec::new(),
                topic_anchored_resolution: None,
                resolution_graph:
                    crate::reference_resolution_graph::ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            }
        } else if pending_answer.disposition == QuestionAnswerDispositionIR::Resolved {
            let binding = pending_answer.binding.clone();
            ReferenceResolutionIR {
                original_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_semantic_text: pending_answer.resolved_semantic_text.clone(),
                resolved_reference_count: usize::from(binding.is_some()),
                used_referent_ids: binding
                    .as_ref()
                    .map(|binding| binding.referent_ids.clone())
                    .unwrap_or_default(),
                ambiguous_reference_surfaces: Vec::new(),
                topic_anchored_resolution: None,
                resolution_graph:
                    crate::reference_resolution_graph::ReferenceResolutionGraphIR::default(),
                discourse_bindings: binding.into_iter().collect(),
            }
        } else if definition_grounding_applies {
            ReferenceResolutionIR {
                original_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: Vec::new(),
                topic_anchored_resolution: None,
                resolution_graph:
                    crate::reference_resolution_graph::ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            }
        } else {
            self.conversation_memory.resolve_references(
                &request.conversation_id,
                &pending_answer.resolved_semantic_text,
            )
        };
        if pending_answer.disposition == QuestionAnswerDispositionIR::InvalidOrNonAuthoritative {
            reference_resolution
                .ambiguous_reference_surfaces
                .push("PENDING_QUD_ANSWER".to_string());
            reference_resolution.ambiguous_reference_surfaces.sort();
            reference_resolution.ambiguous_reference_surfaces.dedup();
        }
        if quoted_metalinguistic_request {
            reference_resolution = ReferenceResolutionIR {
                original_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_semantic_text: normalization.semantic_surface_text.clone(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: Vec::new(),
                topic_anchored_resolution: None,
                resolution_graph:
                    crate::reference_resolution_graph::ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            };
        }
        if unresolved_topic_pointer {
            reference_resolution
                .ambiguous_reference_surfaces
                .push("PREVIOUS_TOPIC_STACK".to_string());
        }
        if !reference_resolution.resolution_graph.validate_against(
            &reference_resolution.original_semantic_text,
            &reference_resolution.resolved_semantic_text,
            reference_resolution.discourse_bindings.len(),
        ) {
            reference_resolution.resolution_graph =
                crate::reference_resolution_graph::build_reference_resolution_graph(
                    &reference_resolution.original_semantic_text,
                    &reference_resolution.resolved_semantic_text,
                    &reference_resolution.discourse_bindings,
                    &[],
                    &[],
                );
        }
        // Select one authoritative native evidence surface. A real,
        // unambiguous reference binding owns it; otherwise the resolver has no
        // rewrite authority and the original utterance remains the source.
        let native_source_text =
            authoritative_native_source(request, &reference_resolution).to_string();
        let mut native_language_circuit = NativeLanguageCircuit
            .analyze_with_context(&native_source_text, &native_dialogue_context);
        if pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn) {
            native_language_circuit.apply_future_notification_boundary(&native_source_text);
        }
        if !request.alternatives.is_empty()
            && normalization.disposition == ConversationTurnDispositionIR::ClarificationRequired
        {
            native_language_circuit.add_boundary_ambiguity("FRONTEND_INPUT_ALTERNATIVES");
        }
        debug_assert!(native_language_circuit.validate_for_source(&native_source_text));
        let pragmatic_topic_id = self
            .conversation_memory
            .state(&request.conversation_id)
            .and_then(|state| state.active_topics.first())
            .filter(|topic| topic.explicitly_activated)
            .map(|topic| topic.topic_id.clone());
        let pending_gate_before = self
            .pragmatic_memory
            .pending_gate_in_topic(&request.conversation_id, pragmatic_topic_id.as_deref());
        let mut pragmatic_context = self
            .pragmatic_memory
            .context_in_topic(&request.conversation_id, pragmatic_topic_id.as_deref());
        if pragmatic_context.active_subject.is_none() {
            pragmatic_context.active_subject = self
                .conversation_memory
                .state(&request.conversation_id)
                .and_then(|state| state.active_subject.clone());
        }
        if let Some(state) = self.conversation_memory.state(&request.conversation_id) {
            pragmatic_context.active_goals = state
                .active_goals
                .iter()
                .map(|goal| ActiveGoalContextIR {
                    goal_id: goal.goal_id.clone(),
                    canonical_predicate: goal.canonical_predicate.clone(),
                    subject: goal.subject.clone(),
                })
                .collect();
            pragmatic_context.pending_deferred_commitments = state
                .deferred_action_commitments
                .iter()
                .filter(|commitment| commitment.is_pending())
                .map(|commitment| PendingDeferredContextIR {
                    commitment_id: commitment.commitment_id.clone(),
                    canonical_predicate: commitment.action.canonical_predicate.clone(),
                    subject: commitment.action.subject.clone(),
                    condition_sha256: commitment.condition_sha256.clone(),
                })
                .collect();
            let mut recent_focus_nodes = state.discourse_focus.nodes.iter().collect::<Vec<_>>();
            recent_focus_nodes.sort_by(|left, right| {
                right
                    .last_focused_turn
                    .cmp(&left.last_focused_turn)
                    .then_with(|| right.salience_millis.cmp(&left.salience_millis))
                    .then_with(|| left.focus_id.cmp(&right.focus_id))
            });
            pragmatic_context.recent_subjects = state
                .active_goals
                .iter()
                .map(|goal| goal.subject.trim().to_string())
                .chain(
                    recent_focus_nodes
                        .into_iter()
                        .map(|node| node.surface.trim().to_string()),
                )
                .filter(|surface| !surface.is_empty())
                .fold(Vec::<String>::new(), |mut subjects, surface| {
                    if !subjects
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(&surface))
                    {
                        subjects.push(surface);
                    }
                    subjects
                });
            pragmatic_context.recent_subjects.truncate(4);
        }
        let mut pragmatic_interpretation = self
            .pragmatic_reasoner
            .interpret_with_predicates_and_illocution(
                &reference_resolution.resolved_semantic_text,
                &normalization.semantic_surface_text,
                &pragmatic_context,
                &self.compositional_predicates,
            );
        let dialogue_directive_analysis = self
            .language_knowledge
            .analyze_dialogue_directives(&normalization.semantic_surface_text);
        let dialogue_directive_candidates = dialogue_directive_candidates(
            &pragmatic_interpretation,
            &dialogue_directive_analysis,
            &request.raw_text,
        );
        let feedback_goal_agreement = pragmatic_interpretation.user_feedback.is_none()
            || pragmatic_interpretation
                .inferred_goal
                .as_ref()
                .is_some_and(|inferred| {
                    pragmatic_interpretation
                        .compositional_analysis
                        .selected_candidates()
                        .into_iter()
                        .any(|candidate| {
                            candidate.intent == inferred.intent
                                && crate::native_language_circuit::subjects_share_context_concept(
                                    &candidate.subject,
                                    &inferred.subject,
                                )
                        })
                });
        let compositional_goal_materialized = !pipeline_routing
            .has(PipelineSignal::FutureNotificationOwnsTurn)
            && feedback_goal_agreement
            && native_language_circuit.absorb_selected_compositional_goals(
                &native_source_text,
                &pragmatic_interpretation.compositional_analysis,
            );
        debug_assert!(native_language_circuit.validate_for_source(&native_source_text));
        let explicit_dialogue_directive = !dialogue_directive_analysis.frames.is_empty()
            && dialogue_directive_analysis.unresolved_axes.is_empty();
        let has_non_directive_goal =
            if let Some(goals) = native_language_circuit.authoritative_live_goals() {
                goals
                    .iter()
                    .any(|goal| !is_dialogue_directive_goal(&goal.subject))
            } else {
                pragmatic_interpretation
                    .compositional_analysis
                    .selected_candidates()
                    .into_iter()
                    .any(|candidate| !is_dialogue_directive_goal(&candidate.subject))
            };
        let dialogue_directive_owns_turn = explicit_dialogue_directive && !has_non_directive_goal;
        pipeline_routing.activate_if(
            dialogue_directive_owns_turn,
            PipelineSignal::DialogueDirectiveOwnsTurn,
        );
        if pending_answer.disposition == QuestionAnswerDispositionIR::Resolved {
            let resolved_operation = pragmatic_interpretation
                .compositional_analysis
                .selected_candidates()
                .into_iter()
                .next()
                .and_then(|candidate| {
                    pragmatic_interpretation
                        .compositional_analysis
                        .frames
                        .iter()
                        .find(|frame| frame.frame_id == candidate.source_frame_id)
                        .map(|frame| (frame.canonical_predicate.clone(), candidate.intent))
                });
            if let Some((canonical_predicate, intent)) = resolved_operation {
                native_language_circuit.apply_resolved_clarification(
                    &native_source_text,
                    &canonical_predicate,
                    intent,
                );
            }
        }
        pipeline_routing.activate_if(
            !pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn)
                && (compositional_goal_materialized
                    || native_goal_projection_required(
                        &pragmatic_interpretation,
                        &native_language_circuit,
                    )),
            PipelineSignal::NativeGoalOwnsTurn,
        );
        pipeline_routing.activate_if(
            definition_grounding_applies
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
                && pending_answer.disposition != QuestionAnswerDispositionIR::Resolved,
            PipelineSignal::DefinitionOwnsTurn,
        );
        if pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn) {
            // Legacy analyzers remain evidence producers. Reconciliation is
            // owned by one Language Center boundary rather than a series of
            // call-site mutations across compatibility graphs.
            pragmatic_interpretation
                .reconcile_native_projection(&native_language_circuit, &native_source_text);
        }
        let inherited_action_goal_ids = reference_resolution
            .discourse_bindings
            .iter()
            .filter(|binding| {
                matches!(
                    binding.kind,
                    DiscourseBindingKindIR::EllipticalAction
                        | DiscourseBindingKindIR::DiscourseProgramInstantiation
                        | DiscourseBindingKindIR::RepeatedGoal
                        | DiscourseBindingKindIR::CorrectedArgument
                        | DiscourseBindingKindIR::EventReference
                        | DiscourseBindingKindIR::EventOrdinalReference
                        | DiscourseBindingKindIR::PluralEventMemberReference
                        | DiscourseBindingKindIR::TopicAnchoredActionMemberReference
                )
            })
            .filter_map(|binding| binding.inherited_goal_id.as_deref())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut action_state_analysis = if pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            || pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
        {
            ActionStateAnalysisIR::default()
        } else {
            ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
                &reference_resolution.resolved_semantic_text,
                &normalization.semantic_surface_text,
                self.conversation_memory
                    .state(&request.conversation_id)
                    .map(|state| &state.action_state_ledger)
                    .unwrap_or(&ActionStateLedgerIR::default()),
                &inherited_action_goal_ids,
            )
        };
        if action_state_analysis.has_language_reports()
            && pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
        {
            action_state_analysis = ActionStateAnalysisIR::default();
        }
        if action_state_analysis.target_action_ids.len() == 1 {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    !matches!(
                        surface.to_lowercase().as_str(),
                        "it" | "that" | "its" | "그거" | "그것" | "그 작업"
                    )
                });
        }
        if pragmatic_interpretation
            .illocutionary_commitments
            .outcome_claim_policy
            .is_some()
        {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| !matches!(surface.to_lowercase().as_str(), "it" | "that"));
        }
        if pragmatic_interpretation
            .illocutionary_commitments
            .primary_force()
            == Some(IllocutionaryForceIR::ReportedCommitment)
        {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    !matches!(surface.to_lowercase().as_str(), "he" | "she" | "they")
                });
        }
        if matches!(
            pragmatic_interpretation
                .illocutionary_commitments
                .primary_force(),
            Some(
                IllocutionaryForceIR::AnswerOnlyInformationRequest
                    | IllocutionaryForceIR::GoalWithdrawal
            )
        ) {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| !matches!(surface.to_lowercase().as_str(), "it" | "that"));
        }
        if pragmatic_interpretation
            .pragmatic_intent_graph
            .primary_kind()
            == Some(PragmaticIntentKindIR::GoalCorrection)
            && pragmatic_context.active_goals.len() == 1
        {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| !matches!(surface.to_lowercase().as_str(), "it" | "that"));
        }
        if pragmatic_interpretation
            .pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|intent| {
                intent.communicative_intent == CommunicativeIntentIR::SummaryRequest
                    && intent.prior_context_bound
            })
        {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    !surface
                        .to_uppercase()
                        .starts_with("DISCOURSE_RELATION_ANTECEDENT:")
                });
        }
        if native_language_circuit.response_goal
            == crate::native_language_circuit::NativeResponseGoalIR::AnswerVerifiedResult
            && self
                .conversation_memory
                .state(&request.conversation_id)
                .is_some_and(|state| !state.epistemic_ledger.records.is_empty())
        {
            // A result-status question can quantify over the current evidence
            // set.  A leading consequence connective ("so" / "그래서") is
            // discourse structure, not a demand to choose one proposition as
            // its antecedent before answering the evidence boundary.
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    !surface
                        .to_uppercase()
                        .starts_with("DISCOURSE_RELATION_ANTECEDENT:")
                });
        }
        if let Some(composition) = pragmatic_interpretation
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .filter(|graph| graph.has_selected_authorized_request())
        {
            let selected_surfaces = composition
                .selected_node_ids
                .iter()
                .filter_map(|selected| {
                    composition
                        .nodes
                        .iter()
                        .find(|node| &node.node_id == selected)
                })
                .map(|node| node.source_text.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    let ambiguous = surface.to_lowercase();
                    if ambiguous.contains("possessive_focus_reference")
                        && !composition.suppressed_node_ids.is_empty()
                    {
                        return false;
                    }
                    let deictic = ["it", "that", "this", "그거", "그것", "이거", "이것"]
                        .iter()
                        .find(|token| ambiguous.contains(**token));
                    deictic.is_none_or(|token| {
                        selected_surfaces
                            .split(|character: char| !character.is_alphanumeric())
                            .any(|word| word == *token)
                    })
                });
        }
        if (pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
            || native_language_circuit.response_goal
                == crate::native_language_circuit::NativeResponseGoalIR::AnswerVerifiedResult)
            && !native_language_circuit.reference_bindings.is_empty()
        {
            let resolves_operation_ellipsis = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::OperationEllipsis);
            let resolves_prior_theme = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::ExplicitPriorTheme);
            let resolves_event_ordinal = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::EventOrdinal);
            let resolves_verified_result = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::VerifiedResultTarget);
            let resolves_causal_target =
                native_language_circuit
                    .reference_bindings
                    .iter()
                    .any(|binding| {
                        binding.kind == NativeReferenceKindIR::CausalTarget
                            || binding.source_surface == "CAUSE_REFERENCE"
                    });
            let resolves_set_member = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::SetMember);
            let resolves_contrastive_retarget = native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::ContrastiveRetarget);
            let resolves_bound_ellipsis =
                resolves_operation_ellipsis || resolves_prior_theme || resolves_event_ordinal;
            let prohibits_bound_plural =
                (resolves_event_ordinal || resolves_set_member || resolves_contrastive_retarget)
                    && native_language_circuit
                        .events
                        .iter()
                        .any(|event| event.scope == NativeEventScopeIR::Prohibited);
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    let normalized = surface.to_lowercase();
                    !normalized.contains("demonstrative_focus_reference")
                        && !(resolves_bound_ellipsis
                            && (normalized.contains("elliptical_action")
                                || normalized.contains("same_operation")
                                || normalized == "same"))
                        && !(resolves_bound_ellipsis
                            && native_language_circuit.response_goal
                                == NativeResponseGoalIR::PlanActions
                            && normalized.contains("proposition_reference"))
                        && !(prohibits_bound_plural
                            && matches!(
                                normalized.as_str(),
                                "them" | "either of them" | "둘 다" | "둘 중 어느 것도"
                            ))
                        && !(resolves_event_ordinal
                            && (normalized.contains("event_ordinal")
                                || normalized.contains("previous_topic_stack")
                                || normalized.contains("first")
                                || normalized.contains("첫 번째")))
                        && !(resolves_verified_result
                            && (normalized.contains("proposition_reference")
                                || normalized.contains("result_reference")
                                || (normalized.contains("entity_ontology_reference")
                                    && normalized.contains("report"))
                                || normalized == "그 주장"
                                || normalized == "주장"))
                        && !(resolves_causal_target
                            && (normalized.contains("zero_argument_ellipsis")
                                || normalized.contains("causal_target")))
                        && !(resolves_set_member
                            && (normalized == "latter"
                                || normalized == "the latter"
                                || normalized == "후자"
                                || normalized.contains("second item")
                                || normalized.contains("두 번째 것")))
                        && !matches!(
                            normalized.as_str(),
                            "it" | "that"
                                | "that one"
                                | "this"
                                | "그거"
                                | "그것"
                                | "그걸"
                                | "이거"
                                | "이것"
                                | "why"
                                | "왜"
                                | "원인"
                        )
                });
        }
        if native_language_circuit.response_goal == NativeResponseGoalIR::Acknowledge
            && native_language_circuit.unresolved.is_empty()
            && native_language_circuit.selected_live_goals.is_empty()
            && contains_explicit_prohibition(&normalization.semantic_text)
        {
            // A pure prohibition has no live action to execute.  A plural
            // object may remain lexically underspecified without making the
            // safe acknowledgement itself ambiguous.
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    !matches!(
                        surface.to_lowercase().as_str(),
                        "them" | "either of them" | "둘 다" | "둘 중 어느 것도"
                    )
                });
        }
        pipeline_routing.activate_if(
            pending_gate_before.is_some()
                && is_pending_gate_decision_question(&normalization.semantic_surface_text),
            PipelineSignal::PendingContinuationGateDecision,
        );
        pipeline_routing.activate_if(
            pending_gate_before.is_some()
                && is_proxy_only_evidence_update(&normalization.semantic_surface_text),
            PipelineSignal::ProxyEvidenceUpdate,
        );
        let social_turn_consumes = has_social_dialogue_event(&normalization)
            && !has_explicit_selected_request(&pragmatic_interpretation)
            && native_language_circuit.events.is_empty()
            && native_language_circuit.selected_live_goals.is_empty();
        if social_turn_consumes {
            // A pure social acknowledgement does not consume the embedded
            // complement as a new semantic claim or action.  Its lexical
            // pronouns therefore cannot veto the backchannel or sever the
            // prior topic.
            reference_resolution.ambiguous_reference_surfaces.clear();
        }
        let mut disposition = if normalization.disposition
            == ConversationTurnDispositionIR::ClarificationRequired
            || native_language_circuit.unresolved.iter().any(|reason| {
                matches!(
                    reason.as_str(),
                    "AMBIGUOUS_DIALOGUE_CONTEXT_ENTITY" | "UNDERSPECIFIED_PROBLEM_DISCLOSURE"
                )
            })
            || !reference_resolution.ambiguous_reference_surfaces.is_empty()
            || pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            || pragmatic_interpretation
                .compositional_analysis
                .clarification_required
            || pragmatic_interpretation
                .unresolved_bindings
                .iter()
                .any(|binding| binding == "CURRENT_TASK")
            || unresolved_action_request(&pragmatic_interpretation)
            || !action_state_analysis.unresolved_ambiguities.is_empty()
            || !dialogue_directive_analysis.unresolved_axes.is_empty()
        {
            ConversationTurnDispositionIR::ClarificationRequired
        } else {
            normalization.disposition
        };
        if social_turn_consumes {
            disposition = ConversationTurnDispositionIR::BackchannelOnly;
        }
        if dialogue_directive_owns_turn {
            disposition = ConversationTurnDispositionIR::Grounded;
        }
        if topic_transition
            .as_ref()
            .is_some_and(|transition| transition.applied)
            || (pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision)
                && reference_resolution.ambiguous_reference_surfaces.is_empty())
            || pipeline_routing.has(PipelineSignal::ProxyEvidenceUpdate)
            || (pragmatic_interpretation.continuation_gate.is_some()
                && reference_resolution.ambiguous_reference_surfaces.is_empty())
            || (action_state_analysis.consumes_turn()
                && action_state_analysis.unresolved_ambiguities.is_empty()
                && reference_resolution.ambiguous_reference_surfaces.is_empty())
            || (pragmatic_interpretation
                .pragmatic_intent_graph
                .composition
                .as_ref()
                .is_some_and(|graph| graph.has_selected_conditional_request())
                && reference_resolution.ambiguous_reference_surfaces.is_empty()
                && !pragmatic_interpretation
                    .compositional_analysis
                    .clarification_required)
            || (pragmatic_interpretation
                .pragmatic_intent_graph
                .selected_utterance_intent()
                .is_some_and(|intent| {
                    intent.communicative_intent == CommunicativeIntentIR::AssessmentRequest
                        && intent.expected_response
                            == crate::utterance_intent::ExpectedResponseKindIR::Assessment
                })
                && reference_resolution.ambiguous_reference_surfaces.is_empty())
        {
            disposition = ConversationTurnDispositionIR::Grounded;
        }
        if (pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
            || native_language_circuit.response_goal
                == crate::native_language_circuit::NativeResponseGoalIR::AnswerVerifiedResult)
            && normalization.disposition != ConversationTurnDispositionIR::ClarificationRequired
            && reference_resolution.ambiguous_reference_surfaces.is_empty()
            && !pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            && !pragmatic_interpretation
                .compositional_analysis
                .clarification_required
        {
            disposition = ConversationTurnDispositionIR::Grounded;
        }
        if let Some(update) = &discourse_group_update {
            disposition = if update.applied {
                ConversationTurnDispositionIR::Grounded
            } else {
                ConversationTurnDispositionIR::ClarificationRequired
            };
        }
        if pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn) {
            disposition = ConversationTurnDispositionIR::Grounded;
        }
        let remembered_language = self
            .conversation_memory
            .state(&request.conversation_id)
            .and_then(|state| state.preferred_language);
        let output_language = request
            .output_language
            .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
            .or(remembered_language)
            .unwrap_or_else(|| conversational_language(&request.raw_text));
        let query_function_reference_only =
            !reference_resolution.ambiguous_reference_surfaces.is_empty()
                && reference_resolution
                    .ambiguous_reference_surfaces
                    .iter()
                    .all(|surface| matches!(surface.to_lowercase().as_str(), "it" | "that"));
        pipeline_routing.activate_if(
            reference_resolution
                .discourse_bindings
                .iter()
                .any(|binding| binding.kind == DiscourseBindingKindIR::ResultReference),
            PipelineSignal::ResultReferenceOwnsTurn,
        );
        let selected_authorized_pragmatic_request = pragmatic_interpretation
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .is_some_and(|graph| graph.has_selected_authorized_request());
        let response_goal_correction = pragmatic_interpretation
            .pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|intent| {
                intent.communicative_intent == CommunicativeIntentIR::ResponseGoalCorrection
            });
        pipeline_routing.activate_if(
            response_goal_correction,
            PipelineSignal::ResponseGoalCorrection,
        );
        let problem_disclosure = pragmatic_interpretation
            .pragmatic_intent_graph
            .selected_utterance_intent()
            .is_some_and(|intent| {
                intent.communicative_intent == CommunicativeIntentIR::ProblemDisclosure
            });
        // A reported completion and a request for verified results intentionally share
        // the same native response goal, but only the latter is allowed to consume the
        // prior action ledger as a query.  Treating every result-shaped utterance as a
        // question turned first-person reports into fabricated verification answers.
        let native_verified_result_query = native_language_circuit.response_goal
            == crate::native_language_circuit::NativeResponseGoalIR::AnswerVerifiedResult
            && matches!(
                native_language_circuit.response_mode,
                NativeResponseModeIR::VerificationStatusQuery
                    | NativeResponseModeIR::EvidenceResultQuery
                    | NativeResponseModeIR::SourceCertaintyQuery
                    | NativeResponseModeIR::OutcomeAlternativeQuery
            );
        let plan_result_query_focus = if native_verified_result_query {
            PlanResultQueryFocusIR::VerifiedResult
        } else {
            classify_plan_result_query_focus(&normalization.semantic_surface_text)
        };
        let prior_action_ledger = self
            .conversation_memory
            .state(&request.conversation_id)
            .map(|state| &state.action_state_ledger);
        let prior_action_records_present =
            prior_action_ledger.is_some_and(|ledger| !ledger.records.is_empty());
        let requests_lifecycle_response = pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .is_some_and(|goal| goal.intent == PlanIntentIR::Communicate);
        pipeline_routing.activate_if(
            plan_result_query_focus != PlanResultQueryFocusIR::None
                && prior_action_records_present
                && !pipeline_routing.has(PipelineSignal::ResponseGoalCorrection)
                && !pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn)
                && (native_verified_result_query
                    || normalization
                        .semantic_surface_text
                        .trim_end()
                        .ends_with('?')
                    || pipeline_routing.has(PipelineSignal::ResultReferenceOwnsTurn)
                    || pragmatic_interpretation
                        .illocutionary_commitments
                        .primary_force()
                        == Some(IllocutionaryForceIR::AnswerOnlyInformationRequest)
                    || requests_lifecycle_response),
            PipelineSignal::PlanResultCandidate,
        );
        let prior_plan_result_boundary = pipeline_routing
            .has(PipelineSignal::PlanResultCandidate)
            .then(|| {
                build_plan_result_boundary(
                    &normalization.semantic_surface_text,
                    &action_state_analysis,
                    prior_action_ledger.expect("lifecycle query requires an action ledger"),
                )
            });
        if prior_plan_result_boundary
            .as_ref()
            .is_some_and(|boundary| boundary.selected_action_ids.len() == 1)
        {
            reference_resolution
                .ambiguous_reference_surfaces
                .retain(|surface| {
                    let normalized = surface.to_lowercase();
                    !(normalized.starts_with("entity_ontology_reference:")
                        && normalized.contains("report"))
                });
        }
        if pipeline_routing.has(PipelineSignal::PlanResultCandidate)
            && reference_resolution.ambiguous_reference_surfaces.is_empty()
        {
            disposition = ConversationTurnDispositionIR::Grounded;
        }
        pipeline_routing.activate_if(
            action_state_analysis.consumes_turn()
                && !selected_authorized_pragmatic_request
                && !pipeline_routing.has(PipelineSignal::ResponseGoalCorrection)
                && (!problem_disclosure || action_state_analysis.has_language_reports())
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn),
            PipelineSignal::ActionStateCandidate,
        );
        pipeline_routing.activate_if(
            matches!(
                pragmatic_interpretation
                    .pragmatic_intent_graph
                    .primary_kind(),
                Some(
                    PragmaticIntentKindIR::ConventionalIndirectRequest
                        | PragmaticIntentKindIR::PreferenceRequest
                        | PragmaticIntentKindIR::AdvisorySuggestion
                        | PragmaticIntentKindIR::RhetoricalEvaluation
                        | PragmaticIntentKindIR::SelfOffer
                        | PragmaticIntentKindIR::GoalCorrection
                )
            ),
            PipelineSignal::PragmaticForceOwnsSurfaceQuestion,
        );
        pipeline_routing.activate_if(
            pragmatic_interpretation.continuation_gate.is_some(),
            PipelineSignal::InitialContinuationGateOwnsTurn,
        );
        pipeline_routing.activate_if(
            has_explicit_selected_request(&pragmatic_interpretation),
            PipelineSignal::ExplicitSelectedRequest,
        );
        pipeline_routing.activate_if(
            reference_resolution.ambiguous_reference_surfaces.is_empty()
                || query_function_reference_only,
            PipelineSignal::DeicticQueryReferenceSafe,
        );
        pipeline_routing.activate_if(
            reference_resolution.ambiguous_reference_surfaces.is_empty(),
            PipelineSignal::ReferencesFullyResolved,
        );
        let temporal_answer = if pipeline_routing.allows_temporal_qa() {
            let state = self.conversation_memory.state(&request.conversation_id);
            if query_function_reference_only {
                self.temporal_qa.answer(
                    &normalization.semantic_surface_text,
                    state.map(|state| &state.temporal_graph),
                    output_language,
                )
            } else {
                self.temporal_qa
                    .answer(
                        &reference_resolution.resolved_semantic_text,
                        state.map(|state| &state.temporal_graph),
                        output_language,
                    )
                    .or_else(|| {
                        (reference_resolution.resolved_semantic_text
                            != normalization.semantic_surface_text)
                            .then(|| {
                                self.temporal_qa.answer(
                                    &normalization.semantic_surface_text,
                                    state.map(|state| &state.temporal_graph),
                                    output_language,
                                )
                            })
                            .flatten()
                    })
            }
        } else {
            None
        };
        // A "when did" question presupposes occurrence.  If the only matching
        // event records live in possible/predicted/counterfactual worlds, let
        // the presupposition-aware discourse path abstain instead of answering
        // from a non-actual world as though the event occurred.
        let temporal_answer = temporal_answer.filter(|answer| {
            !(answer.query.kind == crate::temporal::TemporalQueryKindIR::EventTime
                && (answer.disposition
                    == crate::temporal::TemporalAnswerDispositionIR::EventTimeNotRecorded
                    || answer.disposition
                        == crate::temporal::TemporalAnswerDispositionIR::NoMatchingEvent
                    || (!answer.event_evidence.is_empty()
                        && answer.event_evidence.iter().all(|event| {
                            event.modal_world != crate::modality::ModalWorldIR::Actual
                        }))))
        });
        let dialogue_relation_answer = if pipeline_routing
            .allows_dialogue_relation_qa(temporal_answer.is_some())
            && self
                .conversation_memory
                .state(&request.conversation_id)
                .is_some_and(|state| state.dialogue_relation_graph.has_active_relations())
        {
            let state = self.conversation_memory.state(&request.conversation_id);
            self.dialogue_relation_qa
                .answer(
                    &reference_resolution.resolved_semantic_text,
                    state.map(|state| &state.dialogue_relation_graph),
                    output_language,
                )
                .or_else(|| {
                    (reference_resolution.resolved_semantic_text
                        != normalization.semantic_surface_text)
                        .then(|| {
                            self.dialogue_relation_qa.answer(
                                &normalization.semantic_surface_text,
                                state.map(|state| &state.dialogue_relation_graph),
                                output_language,
                            )
                        })
                        .flatten()
                })
        } else {
            None
        };
        let discourse_answer = if pipeline_routing.allows_discourse_qa(
            temporal_answer.is_some(),
            dialogue_relation_answer.is_some(),
        ) {
            let state = self.conversation_memory.state(&request.conversation_id);
            if query_function_reference_only {
                self.discourse_qa.answer(
                    &normalization.semantic_surface_text,
                    state,
                    output_language,
                )
            } else {
                self.discourse_qa
                    .answer(
                        &reference_resolution.resolved_semantic_text,
                        state,
                        output_language,
                    )
                    .or_else(|| {
                        (reference_resolution.resolved_semantic_text
                            != normalization.semantic_surface_text)
                            .then(|| {
                                self.discourse_qa.answer(
                                    &normalization.semantic_surface_text,
                                    state,
                                    output_language,
                                )
                            })
                            .flatten()
                    })
            }
        } else {
            None
        };
        pipeline_routing.activate_if(
            temporal_answer.is_some()
                || dialogue_relation_answer.is_some()
                || discourse_answer.is_some(),
            PipelineSignal::QuestionAnswer,
        );
        let typed_question_answer = pipeline_routing.has(PipelineSignal::QuestionAnswer);
        pipeline_routing.activate_if(
            !typed_question_answer && pipeline_routing.has(PipelineSignal::PlanResultCandidate),
            PipelineSignal::PlanResultOwnsTurn,
        );
        pipeline_routing.activate_if(
            !typed_question_answer
                && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
                && pipeline_routing.has(PipelineSignal::ActionStateCandidate),
            PipelineSignal::ActionStateOwnsTurn,
        );
        let candidate_temporal_analysis = if pipeline_routing.allows_temporal_analysis() {
            let temporal_surface = if reference_resolution.ambiguous_reference_surfaces.is_empty() {
                &reference_resolution.resolved_semantic_text
            } else {
                &normalization.semantic_surface_text
            };
            self.temporal_analyzer.analyze_turn(
                temporal_surface,
                request.turn_index,
                self.conversation_memory
                    .state(&request.conversation_id)
                    .map(|state| &state.temporal_graph),
            )
        } else {
            TemporalTurnAnalysisIR::default()
        };
        let temporal_deictic_reference_resolved =
            !candidate_temporal_analysis.relations.is_empty() && query_function_reference_only;
        if pipeline_routing.has(PipelineSignal::QuestionAnswer)
            || temporal_deictic_reference_resolved
        {
            disposition = ConversationTurnDispositionIR::Grounded;
        }

        let explicit_selected_request = has_explicit_selected_request(&pragmatic_interpretation);
        let planner_inferred_goal =
            planner_inferred_goal(&pragmatic_interpretation, &dialogue_directive_analysis);
        let typed_semantic_goal_available = pragmatic_interpretation
            .language_center
            .to_semantic_plan_goal(
                &request.request_id,
                &request.context_tags,
                request.max_plan_steps,
                &pragmatic_interpretation.compositional_analysis,
                pipeline_routing
                    .has(PipelineSignal::NativeGoalOwnsTurn)
                    .then_some(&native_language_circuit),
                planner_inferred_goal,
            )
            .is_some();
        pipeline_routing.activate_if(
            disposition == ConversationTurnDispositionIR::Grounded,
            PipelineSignal::GroundedDisposition,
        );
        pipeline_routing.activate_if(
            typed_semantic_goal_available,
            PipelineSignal::SemanticGoalAvailable,
        );
        pipeline_routing.activate_if(
            discourse_group_update
                .as_ref()
                .is_some_and(|update| update.applied),
            PipelineSignal::DiscourseGroupUpdateApplied,
        );
        pipeline_routing.activate_if(
            topic_transition.as_ref().is_some_and(|transition| {
                transition.applied && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
            }),
            PipelineSignal::TopicTransitionOwnsTurn,
        );
        pipeline_routing.activate_if(
            topic_transition
                .as_ref()
                .is_some_and(|transition| transition.applied),
            PipelineSignal::TopicTransitionApplied,
        );
        pipeline_routing.activate_if(
            interaction_boundary_required(&pragmatic_interpretation)
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
                && !native_verified_result_query
                && !explicit_selected_request,
            PipelineSignal::InteractionBoundaryOwnsTurn,
        );
        pipeline_routing.activate_if(
            has_social_dialogue_event(&normalization)
                && !explicit_selected_request
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn),
            PipelineSignal::SocialOnly,
        );
        pipeline_routing.activate_if(
            pragmatic_interpretation.user_feedback.is_some()
                && !explicit_selected_request
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn),
            PipelineSignal::FeedbackOnly,
        );
        pipeline_routing.activate_if(
            detect_user_affect(&request.raw_text).is_some()
                && !explicit_selected_request
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn),
            PipelineSignal::AffectOnly,
        );
        pipeline_routing.activate_if(
            pragmatic_interpretation.speech_act == SpeechActIR::Inform
                && pragmatic_interpretation.inferred_goal.is_none()
                && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
                && !pragmatic_interpretation
                    .nonliteral_analysis
                    .expressions
                    .iter()
                    .any(|expression| {
                        expression.selected_reading
                            == crate::nonliteral::ReadingSelectionIR::Figurative
                    }),
            PipelineSignal::InformOnly,
        );
        pipeline_routing.activate_if(
            self.conversation_memory
                .state(&request.conversation_id)
                .is_some_and(|state| !state.conditional_guard_store.guards.is_empty())
                && !explicit_selected_request
                && !pipeline_routing.has(PipelineSignal::QuestionAnswer),
            PipelineSignal::ConditionalGuardEvidenceCandidate,
        );
        let precommit_plan_projection = PlanProjectionDecisionIR::from_routing(&pipeline_routing);

        // No analyzer renders here. This stage either materializes a semantic
        // plan or produces an empty shell for the single final realizer below.
        let mut output = ConversationalOutputIR {
            language: output_language,
            text: String::new(),
            grounded_plan_sha256: None,
            unsupported_freeform_claims: 0,
        };
        let (pending_grounded_response, semantic_subject) =
            if precommit_plan_projection.allows_plan() {
                let mut context_tags = request.context_tags.clone();
                context_tags.extend(normalization.semantic_tags.iter().cloned());
                context_tags.extend(active_dialogue_directive_tags.iter().cloned());
                if let Some(state) = self.conversation_memory.state(&request.conversation_id) {
                    for referent_id in &reference_resolution.used_referent_ids {
                        if let Some(referent) = state
                            .active_referents
                            .iter()
                            .find(|referent| &referent.referent_id == referent_id)
                        {
                            context_tags.push(referent.canonical_concept.clone());
                        }
                    }
                }
                context_tags.sort();
                context_tags.dedup();
                let mut response = self.process_with_pragmatics(
                    &NaturalLanguageRequestIR {
                        schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                        request_id: request.request_id.clone(),
                        text: reference_resolution.resolved_semantic_text.clone(),
                        output_language: Some(output_language),
                        context_tags,
                        max_plan_steps: request.max_plan_steps,
                    },
                    pragmatic_interpretation.clone(),
                    pipeline_routing
                        .has(PipelineSignal::NativeGoalOwnsTurn)
                        .then_some(&native_language_circuit),
                )?;
                response.understanding.original_text = request.raw_text.clone();
                response.understanding.normalized_text =
                    reference_resolution.resolved_semantic_text.clone();
                let subject = response.understanding.subject.clone();
                (Some(Box::new(response)), Some(subject))
            } else {
                (None, None)
            };
        let memory_goal_projection_allowed = pipeline_routing
            .has(PipelineSignal::GroundedDisposition)
            && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::DialogueDirectiveOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::ActionStateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::QuestionAnswer)
            && !quoted_metalinguistic_request
            && (topic_transition.is_none()
                || pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn))
            && !pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision)
            && !pipeline_routing.has(PipelineSignal::ProxyEvidenceUpdate)
            && (!pragmatic_interpretation
                .illocutionary_commitments
                .blocks_current_goal_projection()
                || pragmatic_interpretation
                    .pragmatic_intent_graph
                    .composition
                    .as_ref()
                    .is_some_and(|graph| graph.has_selected_immediate_request()))
            && (pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .conditionals
                .is_empty()
                || pragmatic_interpretation
                    .pragmatic_intent_graph
                    .composition
                    .as_ref()
                    .is_some_and(|graph| graph.has_selected_immediate_request())
                || pragmatic_interpretation
                    .pragmatic_intent_graph
                    .primary_kind()
                    == Some(PragmaticIntentKindIR::PreferenceRequest));
        let grounded_goals = if memory_goal_projection_allowed {
            let source_semantic_text = restore_user_grounded_display_forms(
                &reference_resolution.resolved_semantic_text,
                &request.raw_text,
            );
            pending_grounded_response
                .as_deref()
                .map(|response| {
                    semantic_plan_conversation_goal_frames(
                        &response.semantic_goal,
                        &pragmatic_interpretation,
                        &native_language_circuit,
                        request.turn_index,
                        &source_semantic_text,
                    )
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut deferred_commitments = if pipeline_routing.has(PipelineSignal::GroundedDisposition)
            && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::ActionStateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
            && pragmatic_interpretation
                .illocutionary_commitments
                .commitments
                .iter()
                .any(|commitment| {
                    commitment.force == IllocutionaryForceIR::DeferredConditionalRequest
                }) {
            deferred_action_commitments(
                &pragmatic_interpretation,
                request.turn_index,
                &reference_resolution.resolved_semantic_text,
            )
        } else {
            Vec::new()
        };
        if let Some(selected_subject) = grounded_goals.first().map(|goal| goal.subject.as_str()) {
            for commitment in &mut deferred_commitments {
                if discourse_program_subject_key(&commitment.action.subject)
                    == discourse_program_subject_key(selected_subject)
                {
                    commitment.action.subject = selected_subject.to_string();
                }
            }
        }
        let discourse_program = conversation_discourse_program(
            &pragmatic_interpretation,
            request.turn_index,
            &grounded_goals,
            &deferred_commitments,
        );
        let proposition_referents = if pipeline_routing.has(PipelineSignal::GroundedDisposition)
            && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::ActionStateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::QuestionAnswer)
            && !quoted_metalinguistic_request
            && topic_transition.is_none()
            && !pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision)
        {
            conversation_proposition_referents(&pragmatic_interpretation, request.turn_index)
        } else {
            Vec::new()
        };
        let discourse_focus_candidates = conversation_focus_candidates(
            &pragmatic_interpretation,
            &grounded_goals,
            &proposition_referents,
        );
        let temporal_analysis = if pipeline_routing.has(PipelineSignal::GroundedDisposition) {
            candidate_temporal_analysis
        } else {
            TemporalTurnAnalysisIR::default()
        };
        let temporal_analysis_ref = (!temporal_analysis.events.is_empty()
            || !temporal_analysis.relations.is_empty())
        .then_some(&temporal_analysis);
        let prior_pending_question = self
            .conversation_memory
            .state(&request.conversation_id)
            .and_then(|state| state.pending_question.as_ref());
        let pending_question_topic_id = self
            .conversation_memory
            .state(&request.conversation_id)
            .and_then(|state| state.active_topics.first())
            .map(|topic| topic.topic_id.clone());
        let pending_question_candidate = if disposition
            == ConversationTurnDispositionIR::ClarificationRequired
            && pending_answer.disposition == QuestionAnswerDispositionIR::NotApplicable
        {
            build_pending_question(
                request,
                &normalization,
                &reference_resolution,
                &pragmatic_interpretation,
                &native_language_circuit,
                self.conversation_memory.state(&request.conversation_id),
                pending_question_topic_id.as_deref(),
            )
        } else {
            None
        };
        let pending_question_update =
            if pending_answer.disposition == QuestionAnswerDispositionIR::Resolved {
                Some(None)
            } else if pending_answer.disposition
                == QuestionAnswerDispositionIR::InvalidOrNonAuthoritative
            {
                None
            } else if let Some(question) = pending_question_candidate {
                Some(Some(question))
            } else if prior_pending_question.is_some()
                && disposition == ConversationTurnDispositionIR::Grounded
                && !topic_transition
                    .as_ref()
                    .is_some_and(|transition| transition.applied)
            {
                Some(None)
            } else {
                None
            };
        let no_commit_referents = Vec::new();
        let commit_referent_ids = if unresolved_topic_pointer {
            no_commit_referents.as_slice()
        } else {
            reference_resolution.used_referent_ids.as_slice()
        };
        if pending_answer.disposition == QuestionAnswerDispositionIR::Resolved {
            // Remove the answered QUD before the selected option can activate
            // a different topic during commit. Keeping the old topic-scoped
            // question until after commit would temporarily violate the state
            // invariant even though the answer has already resolved it.
            self.conversation_memory
                .update_pending_question(&request.conversation_id, None)
                .map_err(map_conversation_error)?;
        }
        let mut conversation_state = self
            .conversation_memory
            .commit_turn_with_discourse(
                request,
                ConversationCommitContext {
                    semantic_subject: semantic_subject.as_deref(),
                    used_referent_ids: commit_referent_ids,
                    unresolved_reference_count: if (pipeline_routing
                        .has(PipelineSignal::QuestionAnswer)
                        || temporal_deictic_reference_resolved)
                        && query_function_reference_only
                    {
                        usize::from(normalization.ambiguous_input)
                    } else {
                        reference_resolution.ambiguous_reference_surfaces.len()
                            + usize::from(normalization.ambiguous_input)
                    },
                    language: Some(output_language),
                    grounded_goals: &grounded_goals,
                    proposition_referents: &proposition_referents,
                    temporal_analysis: temporal_analysis_ref,
                    guard_conditionals: (pipeline_routing.has(PipelineSignal::GroundedDisposition)
                        && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
                        && !pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
                        && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
                        && !pipeline_routing.has(PipelineSignal::QuestionAnswer)
                        && (!pragmatic_interpretation
                            .compositional_analysis
                            .modal_scope_graph
                            .conditionals
                            .is_empty()
                            || !proposition_referents.is_empty()))
                    .then_some(
                        pragmatic_interpretation
                            .compositional_analysis
                            .modal_scope_graph
                            .conditionals
                            .as_slice(),
                    ),
                    semantic_role_graph: Some(
                        &pragmatic_interpretation
                            .compositional_analysis
                            .semantic_role_graph,
                    ),
                    attribution_graph: Some(
                        &pragmatic_interpretation
                            .compositional_analysis
                            .attribution_graph,
                    ),
                    discourse_focus_candidates: &discourse_focus_candidates,
                },
            )
            .map_err(map_conversation_error)?;
        if !dialogue_directive_candidates.is_empty() {
            conversation_state = self
                .conversation_memory
                .apply_dialogue_directives(
                    &request.conversation_id,
                    request.turn_index,
                    &dialogue_directive_candidates,
                )
                .map_err(map_conversation_error)?;
        }
        if !deferred_commitments.is_empty() {
            conversation_state = self
                .conversation_memory
                .add_deferred_action_commitments(&request.conversation_id, &deferred_commitments)
                .map_err(map_conversation_error)?;
        }
        if let Some(program) = discourse_program.as_ref() {
            conversation_state = self
                .conversation_memory
                .remember_discourse_program(&request.conversation_id, program)
                .map_err(map_conversation_error)?;
        }
        if let Some(update) = discourse_group_update
            .as_ref()
            .filter(|update| update.applied)
        {
            conversation_state = self
                .conversation_memory
                .apply_discourse_group_update(&request.conversation_id, update, request.turn_index)
                .map_err(map_conversation_error)?;
        }
        if action_state_analysis.has_language_reports() {
            conversation_state = self
                .conversation_memory
                .apply_action_state_analysis(
                    &request.conversation_id,
                    &action_state_analysis,
                    request.turn_index,
                )
                .map_err(map_conversation_error)?;
        }
        let withdrawn_goal_ids = pragmatic_interpretation
            .illocutionary_commitments
            .goal_withdrawal
            .as_ref()
            .map(|withdrawal| {
                let prior_goals = pragmatic_context.active_goals.as_slice();
                match withdrawal.scope {
                    GoalWithdrawalScopeIR::AllActiveGoals => prior_goals
                        .iter()
                        .map(|goal| goal.goal_id.clone())
                        .collect::<Vec<_>>(),
                    GoalWithdrawalScopeIR::EventOrdinal => withdrawal
                        .event_ordinal
                        .and_then(|ordinal| prior_goals.get(ordinal.saturating_sub(1)))
                        .map(|goal| vec![goal.goal_id.clone()])
                        .unwrap_or_default(),
                }
            })
            .unwrap_or_default();
        let withdrawn_deferred_ids = pragmatic_interpretation
            .illocutionary_commitments
            .goal_withdrawal
            .as_ref()
            .map(|withdrawal| {
                let pending = pragmatic_context.pending_deferred_commitments.as_slice();
                match withdrawal.scope {
                    GoalWithdrawalScopeIR::AllActiveGoals => pending
                        .iter()
                        .map(|commitment| commitment.commitment_id.clone())
                        .collect::<Vec<_>>(),
                    GoalWithdrawalScopeIR::EventOrdinal
                        if pragmatic_context.active_goals.is_empty() =>
                    {
                        withdrawal
                            .event_ordinal
                            .and_then(|ordinal| pending.get(ordinal.saturating_sub(1)))
                            .map(|commitment| vec![commitment.commitment_id.clone()])
                            .unwrap_or_default()
                    }
                    GoalWithdrawalScopeIR::EventOrdinal => Vec::new(),
                }
            })
            .unwrap_or_default();
        if !withdrawn_goal_ids.is_empty() {
            conversation_state = self
                .conversation_memory
                .retire_active_goals(&request.conversation_id, &withdrawn_goal_ids)
                .map_err(map_conversation_error)?;
        }
        if !withdrawn_deferred_ids.is_empty() {
            conversation_state = self
                .conversation_memory
                .withdraw_deferred_action_commitments(
                    &request.conversation_id,
                    &withdrawn_deferred_ids,
                )
                .map_err(map_conversation_error)?;
        }
        if let Some(update) = pending_question_update {
            conversation_state = self
                .conversation_memory
                .update_pending_question(&request.conversation_id, update)
                .map_err(map_conversation_error)?;
        }
        if let Some(transition) = topic_transition
            .as_ref()
            .filter(|transition| transition.applied)
        {
            conversation_state = self
                .conversation_memory
                .apply_topic_transition(&request.conversation_id, transition, request.turn_index)
                .map_err(map_conversation_error)?;
        }
        if let Some(reference) = reference_resolution
            .topic_anchored_resolution
            .as_ref()
            .filter(|reference| reference.applied)
        {
            conversation_state = self
                .conversation_memory
                .reassert_topic_anchor(&request.conversation_id, reference, request.turn_index)
                .map_err(map_conversation_error)?;
        }
        let conditional_guard_evaluations = conversation_state
            .last_guard_evaluations
            .iter()
            .filter(|evaluation| evaluation.evaluation_turn == request.turn_index)
            .cloned()
            .collect::<Vec<_>>();
        let current_turn_declares_condition = !pragmatic_interpretation
            .compositional_analysis
            .modal_scope_graph
            .conditionals
            .is_empty();
        let current_turn_supplies_guard_evidence = conditional_guard_evaluations
            .iter()
            .flat_map(|evaluation| &evaluation.evidence)
            .any(|evidence| evidence.introduced_turn == request.turn_index);
        pipeline_routing.activate_if(
            !conditional_guard_evaluations.is_empty()
                && !pipeline_routing.has(PipelineSignal::QuestionAnswer)
                && !explicit_selected_request
                && (current_turn_supplies_guard_evidence
                    || (pragmatic_interpretation.inferred_goal.is_none()
                        && pragmatic_interpretation.continuation_gate.is_none()
                        && !pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
                        && (pragmatic_interpretation.speech_act == SpeechActIR::Inform
                            || current_turn_declares_condition))),
            PipelineSignal::ConditionalGuardOwnsTurn,
        );
        let final_plan_projection = PlanProjectionDecisionIR::from_routing(&pipeline_routing);
        let grounded_response = final_plan_projection
            .allows_plan()
            .then_some(pending_grounded_response)
            .flatten();
        output.grounded_plan_sha256 = grounded_response
            .as_deref()
            .map(|response| response.plan.plan_sha256.clone());
        let plan_result_boundary = prior_plan_result_boundary.unwrap_or_else(|| {
            build_plan_result_boundary(
                &normalization.semantic_surface_text,
                &action_state_analysis,
                &conversation_state.action_state_ledger,
            )
        });
        let typed_response_boundary_mode =
            if pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn) {
                None
            } else if action_state_analysis.has_language_reports()
                || action_state_analysis.untrusted_evidence_claim
            {
                Some(NativeResponseModeIR::ReportedOutcome)
            } else if pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
                && plan_result_boundary.has_lifecycle_query()
            {
                let selected_snapshots = plan_result_boundary
                    .selected_action_ids
                    .iter()
                    .filter_map(|action_id| {
                        plan_result_boundary
                            .snapshots
                            .iter()
                            .find(|snapshot| &snapshot.action_id == action_id)
                    })
                    .collect::<Vec<_>>();
                let verified_result_absent = plan_result_boundary.query_focus
                    == PlanResultQueryFocusIR::VerifiedResult
                    && !selected_snapshots.is_empty()
                    && selected_snapshots.iter().all(|snapshot| {
                        snapshot.result_availability
                            == crate::plan_result_boundary::ResultAvailabilityIR::Unavailable
                    });
                Some(if verified_result_absent {
                    NativeResponseModeIR::EvidenceResultQuery
                } else {
                    NativeResponseModeIR::VerificationStatusQuery
                })
            } else if action_state_analysis.query_requested
                && action_state_analysis.unresolved_ambiguities.is_empty()
            {
                Some(NativeResponseModeIR::VerificationStatusQuery)
            } else if discourse_answer.is_some() {
                Some(NativeResponseModeIR::SourceCertaintyQuery)
            } else {
                None
            };
        if let Some(mode) = typed_response_boundary_mode {
            native_language_circuit.refine_response_boundary(&native_source_text, mode);
        }
        // The native circuit owns the response goal after all parser modules have
        // contributed.  Once it has selected a live plan and the central planner
        // materialized that plan, later legacy dialogue classifiers must not
        // reinterpret the same turn as a definition, acknowledgement, or
        // clarification response.
        let native_plan_response = pipeline_routing.has(PipelineSignal::NativeGoalOwnsTurn)
            && native_language_circuit.response_goal == NativeResponseGoalIR::PlanActions
            && grounded_response.is_some();
        let native_acknowledgement = native_language_circuit.response_goal
            == NativeResponseGoalIR::Acknowledge
            && !pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            && !pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
            && !topic_transition
                .as_ref()
                .is_some_and(|transition| transition.applied)
            && (!has_social_dialogue_event(&normalization)
                || pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn))
            && detect_user_affect(&request.raw_text).is_none()
            && pragmatic_interpretation.user_feedback.is_none()
            && !matches!(
                disposition,
                ConversationTurnDispositionIR::BackchannelOnly
                    | ConversationTurnDispositionIR::HoldFloor
            )
            && temporal_answer.is_none()
            && dialogue_relation_answer.is_none()
            && discourse_answer.is_none()
            && !pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision)
            && !pipeline_routing.has(PipelineSignal::ProxyEvidenceUpdate)
            && pragmatic_interpretation.continuation_gate.is_none()
            && (!pipeline_routing.has(PipelineSignal::InteractionBoundaryOwnsTurn)
                || pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn))
            && (pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn)
                || pragmatic_interpretation
                    .language_center
                    .projected_goal_event_ids
                    .is_empty())
            && pragmatic_interpretation
                .nonliteral_analysis
                .expressions
                .is_empty()
            && (pipeline_routing.has(PipelineSignal::FutureNotificationOwnsTurn)
                || pragmatic_interpretation.inferred_goal.is_none()
                || requests_epistemic_record_update(&normalization.semantic_surface_text))
            // Native Acknowledge is a broad semantic default.  It may arbitrate the
            // final act only when the parser actually grounded a non-live boundary
            // event; otherwise the established repair, metaphor, sarcasm, and plan
            // realization paths retain authority.
            && (disposition != ConversationTurnDispositionIR::ClarificationRequired
                || native_language_circuit.unresolved.is_empty());
        let native_answer_response = (native_language_circuit.response_goal
            == NativeResponseGoalIR::AnswerVerifiedResult
            && !pipeline_routing.has(PipelineSignal::ResponseGoalCorrection)
            && temporal_answer.is_none()
            && dialogue_relation_answer.is_none())
        .then(|| match native_language_circuit.response_mode {
            NativeResponseModeIR::ReportedOutcome
            | NativeResponseModeIR::CompetingOutcomeReports => NaturalResponseActIR::ActionState,
            NativeResponseModeIR::VerificationStatusQuery => {
                if prior_action_records_present {
                    NaturalResponseActIR::PlanResultStatus
                } else {
                    NaturalResponseActIR::ResultAbsence
                }
            }
            NativeResponseModeIR::SourceCertaintyQuery => {
                if discourse_answer.is_some() {
                    NaturalResponseActIR::DiscourseAnswer
                } else {
                    NaturalResponseActIR::ResultAbsence
                }
            }
            NativeResponseModeIR::OutcomeAlternativeQuery
            | NativeResponseModeIR::EvidenceResultQuery => NaturalResponseActIR::ResultAbsence,
            NativeResponseModeIR::Plan
            | NativeResponseModeIR::Clarification
            | NativeResponseModeIR::Acknowledgement => NaturalResponseActIR::ResultAbsence,
        });
        let unambiguous_standalone_affect = detect_user_affect(&request.raw_text).is_some()
            && grounded_response.is_none()
            && native_language_circuit.unresolved.is_empty()
            && reference_resolution.ambiguous_reference_surfaces.is_empty();
        let lifecycle_response_act = (pipeline_routing.has(PipelineSignal::PlanResultOwnsTurn)
            && pipeline_routing.has(PipelineSignal::ReferencesFullyResolved)
            && plan_result_boundary.has_lifecycle_query())
        .then(|| {
            let selected = plan_result_boundary
                .selected_action_ids
                .iter()
                .filter_map(|action_id| {
                    plan_result_boundary
                        .snapshots
                        .iter()
                        .find(|snapshot| &snapshot.action_id == action_id)
                })
                .collect::<Vec<_>>();
            if plan_result_boundary.query_focus == PlanResultQueryFocusIR::VerifiedResult
                && !selected.is_empty()
                && selected.iter().all(|snapshot| {
                    snapshot.result_availability
                        == crate::plan_result_boundary::ResultAvailabilityIR::Unavailable
                })
            {
                NaturalResponseActIR::ResultAbsence
            } else {
                NaturalResponseActIR::PlanResultStatus
            }
        });
        let fallback_response_act = match disposition {
            ConversationTurnDispositionIR::HoldFloor => NaturalResponseActIR::HoldFloor,
            ConversationTurnDispositionIR::BackchannelOnly => {
                NaturalResponseActIR::SocialBackchannel
            }
            ConversationTurnDispositionIR::ClarificationRequired => {
                NaturalResponseActIR::ClarificationRequest
            }
            ConversationTurnDispositionIR::Grounded => NaturalResponseActIR::InteractionBoundary,
        };
        let mut response_candidates = Vec::new();
        if let Some(response_act) = native_answer_response {
            response_candidates.push(NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::NativeAnswer,
                response_act,
                "native response boundary selected an answer mode",
            ));
        }
        if let Some(response_act) = lifecycle_response_act {
            response_candidates.push(NaturalResponseCandidateIR::new(
                NaturalResponseSourceIR::PlanResult,
                response_act,
                "the action ledger supports a lifecycle query",
            ));
        }
        let mut contribute = |applicable: bool,
                              source: NaturalResponseSourceIR,
                              response_act: NaturalResponseActIR,
                              evidence: &'static str| {
            if applicable {
                response_candidates.push(NaturalResponseCandidateIR::new(
                    source,
                    response_act,
                    evidence,
                ));
            }
        };
        contribute(
            pipeline_routing.has(PipelineSignal::DialogueDirectiveOwnsTurn),
            NaturalResponseSourceIR::DialogueDirective,
            NaturalResponseActIR::InformAcknowledgement,
            "a compositional lexical directive updated the central dialogue policy",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::ConditionalGuardOwnsTurn),
            NaturalResponseSourceIR::ConditionalGuard,
            NaturalResponseActIR::ConditionalGuard,
            "conditional guard produced a current-turn evaluation",
        );
        contribute(
            native_plan_response,
            NaturalResponseSourceIR::NativePlan,
            NaturalResponseActIR::PlanPreview,
            "native live goals were materialized into a plan",
        );
        contribute(
            native_acknowledgement,
            NaturalResponseSourceIR::NativeAcknowledgement,
            NaturalResponseActIR::InformAcknowledgement,
            "native boundary selected a non-live acknowledgement",
        );
        contribute(
            unambiguous_standalone_affect,
            NaturalResponseSourceIR::StandaloneAffect,
            NaturalResponseActIR::AffectSupport,
            "affect is grounded and no task response was materialized",
        );
        contribute(
            grounded_response.is_none()
                && (pragmatic_interpretation.nonliteral_analysis.has_sarcasm()
                    || pragmatic_interpretation
                        .nonliteral_analysis
                        .expressions
                        .iter()
                        .any(|expression| {
                            expression.selected_reading
                                == crate::nonliteral::ReadingSelectionIR::Figurative
                        })),
            NaturalResponseSourceIR::NonliteralInterpretation,
            NaturalResponseActIR::InterpretationBoundary,
            "typed nonliteral analysis selected a non-literal reading",
        );
        contribute(
            disposition == ConversationTurnDispositionIR::ClarificationRequired,
            NaturalResponseSourceIR::Clarification,
            NaturalResponseActIR::ClarificationRequest,
            "the resolved turn still contains required ambiguity",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn),
            NaturalResponseSourceIR::DefinitionGrounding,
            NaturalResponseActIR::DefinitionGrounding,
            "a validated lexical definition consumes this turn",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::DiscourseGroupUpdateApplied),
            NaturalResponseSourceIR::DiscourseGroupUpdate,
            NaturalResponseActIR::DiscourseGroupUpdate,
            "a typed discourse-group update was applied",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::ActionStateOwnsTurn),
            NaturalResponseSourceIR::ActionState,
            NaturalResponseActIR::ActionState,
            "the action-state analyzer produced a bounded response",
        );
        contribute(
            temporal_answer.is_some(),
            NaturalResponseSourceIR::TemporalAnswer,
            NaturalResponseActIR::TemporalAnswer,
            "temporal QA produced a supported answer",
        );
        contribute(
            dialogue_relation_answer.is_some(),
            NaturalResponseSourceIR::DialogueRelationAnswer,
            NaturalResponseActIR::DialogueRelationAnswer,
            "dialogue-relation QA produced a supported answer",
        );
        contribute(
            discourse_answer.is_some(),
            NaturalResponseSourceIR::DiscourseAnswer,
            NaturalResponseActIR::DiscourseAnswer,
            "discourse QA produced a supported answer",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::TopicTransitionOwnsTurn),
            NaturalResponseSourceIR::TopicTransition,
            NaturalResponseActIR::TopicTransition,
            "a topic transition was applied without a simultaneous live goal",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision)
                || pipeline_routing.has(PipelineSignal::ProxyEvidenceUpdate)
                || pipeline_routing.has(PipelineSignal::InitialContinuationGateOwnsTurn),
            NaturalResponseSourceIR::ContinuationGate,
            NaturalResponseActIR::ContinuationGate,
            "a continuation decision gate is active",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::InteractionBoundaryOwnsTurn),
            NaturalResponseSourceIR::InteractionBoundary,
            NaturalResponseActIR::InteractionBoundary,
            "illocutionary commitments require an interaction boundary",
        );
        contribute(
            pipeline_routing.has(PipelineSignal::ResultReferenceOwnsTurn)
                && pipeline_routing.has(PipelineSignal::ReferencesFullyResolved),
            NaturalResponseSourceIR::ResultReference,
            NaturalResponseActIR::ResultAbsence,
            "a result reference is bound but no verified result is recorded",
        );
        contribute(
            disposition == ConversationTurnDispositionIR::HoldFloor,
            NaturalResponseSourceIR::HoldFloor,
            NaturalResponseActIR::HoldFloor,
            "the user is retaining the conversational floor",
        );
        contribute(
            disposition == ConversationTurnDispositionIR::BackchannelOnly
                || (has_social_dialogue_event(&normalization) && grounded_response.is_none()),
            NaturalResponseSourceIR::SocialBackchannel,
            NaturalResponseActIR::SocialBackchannel,
            "the turn contains only a social dialogue move",
        );
        contribute(
            pragmatic_interpretation.user_feedback.is_some() && grounded_response.is_none(),
            NaturalResponseSourceIR::UserFeedback,
            NaturalResponseActIR::UserFeedback,
            "typed feedback is present without a materialized task",
        );
        contribute(
            detect_user_affect(&request.raw_text).is_some() && grounded_response.is_none(),
            NaturalResponseSourceIR::Affect,
            NaturalResponseActIR::AffectSupport,
            "affect evidence is present without a materialized task",
        );
        contribute(
            pragmatic_interpretation.speech_act == SpeechActIR::Inform
                && grounded_response.is_none(),
            NaturalResponseSourceIR::Inform,
            NaturalResponseActIR::InformAcknowledgement,
            "the turn is an inform act without a task result",
        );
        contribute(
            grounded_response.is_some(),
            NaturalResponseSourceIR::GroundedPlan,
            NaturalResponseActIR::PlanPreview,
            "the semantic planner materialized a grounded plan",
        );
        contribute(
            true,
            NaturalResponseSourceIR::Fallback,
            fallback_response_act,
            "conversation disposition fallback",
        );
        let response_arbitration = arbitrate_natural_response(response_candidates);
        let natural_response_act = response_arbitration.selected_act;
        output.unsupported_freeform_claims = match natural_response_act {
            NaturalResponseActIR::TemporalAnswer => temporal_answer
                .as_ref()
                .map_or(0, |answer| answer.unsupported_claims),
            NaturalResponseActIR::DialogueRelationAnswer => dialogue_relation_answer
                .as_ref()
                .map_or(0, |answer| answer.unsupported_claims),
            NaturalResponseActIR::DiscourseAnswer => discourse_answer
                .as_ref()
                .map_or(0, |answer| answer.unsupported_claims),
            NaturalResponseActIR::ConditionalGuard => conditional_guard_evaluations
                .iter()
                .map(|evaluation| evaluation.unsupported_claims)
                .sum(),
            _ => 0,
        };
        let mut natural_source_refs = vec![
            format!("REQUEST:{}", request.request_id),
            format!("TURN:{}", request.turn_index),
            format!(
                "NATIVE_LANGUAGE_CIRCUIT:{}",
                native_language_circuit.circuit_sha256
            ),
        ];
        if let Some(response) = grounded_response.as_deref() {
            natural_source_refs.push(format!("PLAN:{}", response.plan.plan_sha256));
        }
        if plan_result_boundary.has_lifecycle_query()
            || pipeline_routing.has(PipelineSignal::ResultReferenceOwnsTurn)
            || matches!(
                natural_response_act,
                NaturalResponseActIR::PlanResultStatus | NaturalResponseActIR::ResultAbsence
            )
        {
            natural_source_refs.push(format!(
                "PLAN_RESULT_BOUNDARY:{}",
                plan_result_boundary.boundary_sha256
            ));
        }
        if pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn) {
            natural_source_refs.push(format!(
                "DEFINITION_GROUNDING:{}",
                definition_grounding.grounding_sha256
            ));
        }
        if let Some(transition) = topic_transition.as_ref() {
            natural_source_refs.push(format!("TOPIC_TRANSITION:{}", transition.transition_sha256));
        }
        natural_source_refs.extend(
            action_state_analysis
                .target_action_ids
                .iter()
                .map(|action_id| format!("ACTION:{action_id}")),
        );
        natural_source_refs.extend(
            conversation_state
                .dialogue_directive_ledger
                .active()
                .map(dialogue_directive_tag),
        );
        natural_source_refs.extend(
            reference_resolution
                .used_referent_ids
                .iter()
                .map(|referent_id| format!("REFERENT:{referent_id}")),
        );
        if let Some(reference) = reference_resolution
            .topic_anchored_resolution
            .as_ref()
            .filter(|reference| reference.applied)
        {
            natural_source_refs.push(format!("TOPIC_REFERENCE:{}", reference.resolution_sha256));
        }
        natural_source_refs.extend(
            reference_resolution
                .discourse_bindings
                .iter()
                .filter_map(|binding| binding.inherited_goal_id.as_deref())
                .map(|goal_id| format!("REFERENCE_GOAL:{goal_id}")),
        );
        natural_source_refs.extend(
            reference_resolution
                .discourse_bindings
                .iter()
                .map(|binding| {
                    format!(
                        "REFERENCE_BINDING:{:?}:{}",
                        binding.kind, binding.source_surface
                    )
                }),
        );
        natural_source_refs.extend(
            reference_resolution
                .ambiguous_reference_surfaces
                .iter()
                .map(|surface| format!("AMBIGUOUS_REFERENCE:{surface}")),
        );
        natural_source_refs.sort();
        natural_source_refs.dedup();
        let continuation_gate_realization_source =
            if pipeline_routing.has(PipelineSignal::PendingContinuationGateDecision) {
                pending_gate_before
                    .as_ref()
                    .map(ContinuationGateRealizationSourceIR::PendingDecision)
            } else if pipeline_routing.has(PipelineSignal::ProxyEvidenceUpdate) {
                pending_gate_before
                    .as_ref()
                    .map(ContinuationGateRealizationSourceIR::ProxyEvidence)
            } else {
                pragmatic_interpretation
                    .continuation_gate
                    .as_ref()
                    .map(ContinuationGateRealizationSourceIR::Initial)
            };
        let (clarification_kind, clarification_detail) =
            if natural_response_act == NaturalResponseActIR::ClarificationRequest {
                let (kind, detail) = clarification_generation_source(
                    output.language,
                    &normalization,
                    &reference_resolution,
                    &pragmatic_interpretation,
                );
                (Some(kind), detail)
            } else {
                (None, None)
            };
        let natural_realization = build_natural_realization(NaturalRealizationSources {
            response_arbitration: &response_arbitration,
            language: output.language,
            raw_input: &request.raw_text,
            native_language_circuit: &native_language_circuit,
            semantic_goal: grounded_response
                .as_deref()
                .map(|response| &response.semantic_goal),
            semantic_plan_bundle: grounded_response
                .as_deref()
                .map(|response| &response.semantic_plan_bundle),
            inferred_goal: pragmatic_interpretation.inferred_goal.as_ref(),
            nonliteral_analysis: &pragmatic_interpretation.nonliteral_analysis,
            plan_result_boundary: &plan_result_boundary,
            action_analysis: &action_state_analysis,
            action_ledger: &conversation_state.action_state_ledger,
            continuation_gate: continuation_gate_realization_source,
            user_feedback: pragmatic_interpretation.user_feedback.as_ref(),
            discourse_group_update: discourse_group_update.as_ref(),
            discourse_events: &normalization.discourse_events,
            topic_transition: topic_transition.as_ref(),
            clarification_kind,
            clarification_detail: clarification_detail.as_deref(),
            definition_grounding: &definition_grounding,
            guard_evaluations: &conditional_guard_evaluations,
            illocutionary_commitments: &pragmatic_interpretation.illocutionary_commitments,
            withdrawn_goal_ids: &withdrawn_goal_ids,
            withdrawn_deferred_ids: &withdrawn_deferred_ids,
            discourse_answer: discourse_answer.as_ref(),
            dialogue_relation_answer: dialogue_relation_answer.as_ref(),
            temporal_answer: temporal_answer.as_ref(),
            source_refs: &natural_source_refs,
            dialogue_directives: &conversation_state.dialogue_directive_ledger.directives,
            unsupported_claims: output.unsupported_freeform_claims,
        });
        output.text = natural_realization.realized_text.clone();
        debug_assert!(
            natural_realization.validate_output(
                output.language,
                &output.text,
                output.unsupported_freeform_claims
            ),
            "invalid natural realization: {natural_realization:#?}"
        );
        let grounded_realization =
            build_evidence_grounded_realization(GroundedRealizationSources {
                language: output.language,
                realized_text: &output.text,
                turn_index: request.turn_index,
                plan: grounded_response.as_deref().map(|response| &response.plan),
                action_analysis: &action_state_analysis,
                action_ledger: &conversation_state.action_state_ledger,
                competing_outcome_reports: native_language_circuit.response_mode
                    == NativeResponseModeIR::CompetingOutcomeReports,
                epistemic_ledger: Some(&conversation_state.epistemic_ledger),
                discourse_group_update: discourse_group_update.as_ref(),
                topic_transition: topic_transition.as_ref(),
                active_topic: conversation_state.active_topics.first(),
                topic_anchored_reference: reference_resolution.topic_anchored_resolution.as_ref(),
                discourse_answer: discourse_answer.as_ref(),
                dialogue_relation_answer: dialogue_relation_answer.as_ref(),
                temporal_answer: temporal_answer.as_ref(),
                guard_evaluations: &conditional_guard_evaluations,
                evidence_absence: pipeline_routing.has(PipelineSignal::ResultReferenceOwnsTurn)
                    || natural_response_act == NaturalResponseActIR::ResultAbsence,
                source_unsupported_claims: output.unsupported_freeform_claims,
            });
        let interaction_provenance = build_interaction_provenance(InteractionProvenanceSources {
            conversation_id: &request.conversation_id,
            request_id: &request.request_id,
            raw_language_input: &request.raw_text,
            turn_index: request.turn_index,
            grounded_plan: grounded_response.as_deref().map(|response| &response.plan),
            action_ledger: &conversation_state.action_state_ledger,
            grounded_realization: &grounded_realization,
        });
        let six_axis_integration = build_six_axis_integration(SixAxisIntegrationSources {
            request_id: &request.request_id,
            turn_index: request.turn_index,
            pragmatic_interpretation: &pragmatic_interpretation,
            conversation_state: &conversation_state,
            reference_resolution: &reference_resolution,
            action_state_analysis: &action_state_analysis,
            plan_result_boundary: &plan_result_boundary,
            grounded_plan: grounded_response.as_deref().map(|response| &response.plan),
            natural_realization: &natural_realization,
            grounded_realization: &grounded_realization,
            interaction_provenance: &interaction_provenance,
            realized_output: &output.text,
        });
        debug_assert!(
            six_axis_integration.validate(),
            "invalid six-axis integration: {six_axis_integration:#?}"
        );
        output.unsupported_freeform_claims = grounded_realization.unsupported_claims;
        let mut memory_interpretation = pragmatic_interpretation.clone();
        if pipeline_routing.has(PipelineSignal::QuestionAnswer)
            || pipeline_routing.has(PipelineSignal::ActionStateOwnsTurn)
            || pipeline_routing.has(PipelineSignal::GroupUpdateOwnsTurn)
            || pipeline_routing.has(PipelineSignal::DefinitionOwnsTurn)
            || pipeline_routing.has(PipelineSignal::TopicTransitionApplied)
        {
            memory_interpretation.inferred_current_task = None;
            memory_interpretation.inferred_goal = None;
            memory_interpretation.continuation_gate = None;
        }
        let memory_topic_id = if topic_transition
            .as_ref()
            .is_some_and(|transition| transition.applied)
        {
            conversation_state
                .active_topics
                .first()
                .filter(|topic| topic.explicitly_activated)
                .map(|topic| topic.topic_id.as_str())
        } else {
            pragmatic_topic_id.as_deref()
        };
        let pragmatic_state = self
            .pragmatic_memory
            .commit_turn_in_topic(request, &memory_interpretation, memory_topic_id)
            .map_err(map_pragmatic_memory_error)?;
        let response_disposition = if discourse_connected_backchannel
            && disposition == ConversationTurnDispositionIR::BackchannelOnly
        {
            ConversationTurnDispositionIR::Grounded
        } else {
            disposition
        };
        let language_cortex_integration =
            build_language_cortex_response_integration(LanguageCortexResponseSources {
                request,
                disposition: response_disposition,
                normalization: &normalization,
                definition_grounding: &definition_grounding,
                reference_resolution: &reference_resolution,
                pragmatic_interpretation: &pragmatic_interpretation,
                action_state_analysis: &action_state_analysis,
                plan_result_boundary: &plan_result_boundary,
                discourse_group_update: discourse_group_update.as_ref(),
                topic_transition: topic_transition.as_ref(),
                pragmatic_state: &pragmatic_state,
                conversation_state: &conversation_state,
                grounded_response: grounded_response.as_deref(),
                discourse_answer: discourse_answer.as_ref(),
                dialogue_relation_answer: dialogue_relation_answer.as_ref(),
                temporal_answer: temporal_answer.as_ref(),
                conditional_guard_evaluations: &conditional_guard_evaluations,
                natural_realization: &natural_realization,
                grounded_realization: &grounded_realization,
                interaction_provenance: &interaction_provenance,
                six_axis_integration: &six_axis_integration,
                output: &output,
            });
        debug_assert!(language_cortex_integration.validate());
        let goal_withdrawal_present = pragmatic_interpretation
            .illocutionary_commitments
            .goal_withdrawal
            .is_some();
        let response = ConversationTurnResponseIR {
            schema: CONVERSATION_TURN_RESPONSE_SCHEMA.to_string(),
            conversation_id: request.conversation_id.clone(),
            turn_index: request.turn_index,
            disposition: response_disposition,
            normalization,
            native_language_circuit,
            definition_grounding,
            reference_resolution,
            pragmatic_interpretation,
            action_state_analysis,
            plan_result_boundary,
            discourse_group_update,
            topic_transition,
            pragmatic_state,
            conversation_state,
            grounded_response,
            discourse_answer,
            dialogue_relation_answer,
            temporal_answer,
            conditional_guard_evaluations,
            natural_realization,
            grounded_realization,
            interaction_provenance,
            six_axis_integration,
            language_cortex_integration,
            output,
        };
        debug_assert!(
            response.validate_against(request),
            "conversation response integration mismatch: {response:#?}"
        );
        if goal_withdrawal_present {
            // A withdrawn operation must not survive in the native ellipsis
            // cache after the authoritative conversation state retired it.
            // Keep entity discourse available, but remove operation authority
            // and do not remember the withdrawal wording as a new task.
            if let Some(state) = self
                .native_dialogue_memory
                .get_mut(&request.conversation_id)
            {
                state.active_goals.clear();
            }
        } else {
            remember_native_dialogue_turn(
                &mut self.native_dialogue_memory,
                &request.conversation_id,
                request.turn_index,
                &response.native_language_circuit,
            );
        }
        Ok(response)
    }

    pub fn conversation_state(&self, conversation_id: &str) -> Option<&ConversationStateIR> {
        self.conversation_memory.state(conversation_id)
    }

    pub fn pragmatic_state(&self, conversation_id: &str) -> Option<&PragmaticMemoryStateIR> {
        self.pragmatic_memory.state(conversation_id)
    }

    pub fn language_knowledge_statistics(&self) -> LanguageKnowledgeStatisticsIR {
        self.language_knowledge.statistics()
    }

    pub fn inject_lexeme(&mut self, lexeme: LexemeIR) -> Result<bool, CognitiveApiError> {
        self.lexical_memory
            .inject(lexeme)
            .map_err(map_lexical_error)
    }

    pub fn export_lexeme_snapshot(&self) -> LexemeSnapshotIR {
        self.lexical_memory.snapshot()
    }

    pub fn import_lexeme_snapshot(
        &mut self,
        snapshot: &LexemeSnapshotIR,
    ) -> Result<(), CognitiveApiError> {
        self.lexical_memory
            .import_snapshot(snapshot)
            .map_err(map_lexical_error)
    }

    pub fn record_lexical_outcome(
        &mut self,
        outcome: &LexicalOutcomeIR,
    ) -> Result<(), CognitiveApiError> {
        self.lexical_memory
            .record_outcome(outcome)
            .map_err(map_lexical_error)
    }

    pub fn lexical_memory_statistics(&self) -> LexicalMemoryStatisticsIR {
        self.lexical_memory.statistics()
    }

    pub fn process_knowledge_work(
        &mut self,
        request: &KnowledgeWorkRequestIR,
    ) -> Result<KnowledgeWorkResponseIR, CognitiveApiError> {
        crate::knowledge_work::validate_request(request).map_err(map_knowledge_work_error)?;
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.command, &request.context_tags);
        merge_lexical_activations(&mut understanding, &lexical_activations);
        let operation =
            lexical_knowledge_operation(infer_operation(&request.command), &lexical_activations);
        let document_kind = lexical_document_kind(&lexical_activations);
        understanding.intent = intent_for_knowledge_operation(operation);
        understanding
            .semantic_tags
            .extend(request.context_tags.iter().cloned());
        understanding
            .semantic_tags
            .push("knowledge_work".to_string());
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: understanding.intent,
                subject: understanding.subject.clone(),
                constraints: understanding.constraints.clone(),
                desired_outcomes: vec![
                    "the requested document operation produces a structurally validated artifact"
                        .to_string(),
                    "every analytical finding remains bound to an observable source location"
                        .to_string(),
                    "only the expert roles required by observed quality criteria are spawned"
                        .to_string(),
                    "rendering occurs only after independent assessment and peer review"
                        .to_string(),
                ],
                context_tags: understanding.semantic_tags.clone(),
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        let product = execute_document_work_as_with_reasoning(
            request,
            operation,
            document_kind,
            Some(&self.core),
            Some(&plan.plan_sha256),
        )
        .map_err(map_knowledge_work_error)?;
        Ok(KnowledgeWorkResponseIR {
            schema: KNOWLEDGE_WORK_RESPONSE_SCHEMA.to_string(),
            request_id: request.request_id.clone(),
            understanding,
            lexical_activations,
            plan,
            product,
        })
    }

    pub fn process_long_term_repair_plan(
        &mut self,
        request: &LongTermRepairPlanRequestIR,
    ) -> Result<LongTermRepairPlanResponseIR, CognitiveApiError> {
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self
            .lexical_memory
            .activate(&request.command, &["long_term_repair_plan".to_string()]);
        merge_lexical_activations(&mut understanding, &lexical_activations);
        understanding.semantic_tags.extend([
            "long_term_repair_plan".to_string(),
            "evidence_bound_document".to_string(),
        ]);
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: dockable_semantic_core::PlanIntentIR::Plan,
                subject: if understanding.subject.trim().is_empty() {
                    "대한민국 공동주택 장기수선계획".to_string()
                } else {
                    understanding.subject
                },
                constraints: vec![
                    "모든 입력파일은 추출 영수증과 해시로 근거에 결합한다".to_string(),
                    "69개 공사종별과 7개 시설군을 빠짐없이 대사한다".to_string(),
                    "금액과 40년 일정은 고정소수점 계산 엔진 결과만 사용한다".to_string(),
                    "누락값은 0이 아니라 확인 필요로 유지한다".to_string(),
                    "법령·공식안내·단지규약 충돌은 자동 은폐하지 않는다".to_string(),
                    "내부 전문가 검토 외 외부 모델을 호출하지 않는다".to_string(),
                ],
                desired_outcomes: vec![
                    "정확히 50개의 A4 페이지 IR과 인쇄 가능한 HTML을 만든다".to_string(),
                    "시설·비용·충당금·집행 증빙을 동일 항목 ID로 연결한다".to_string(),
                    "전문가가 확인해야 할 입력과 법적 판단을 명확히 분리한다".to_string(),
                ],
                context_tags: understanding.semantic_tags,
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        process_long_term_repair_plan(&self.core, request, &plan.plan_sha256)
            .map_err(map_long_term_repair_error)
    }

    pub fn process_professional_document(
        &mut self,
        request: &ProfessionalDocumentRequestIR,
    ) -> Result<ProfessionalDocumentResponseIR, CognitiveApiError> {
        let mut understanding = self
            .language_knowledge
            .understand(&request.command)
            .map_err(map_language_error)?;
        let lexical_activations = self.lexical_memory.activate(
            &request.command,
            &[
                "professional_document".to_string(),
                "long_form_writing".to_string(),
            ],
        );
        merge_lexical_activations(&mut understanding, &lexical_activations);
        understanding.semantic_tags.extend([
            "professional_document".to_string(),
            "evidence_bound_section_synthesis".to_string(),
            "working_memory".to_string(),
            "global_consistency_revision".to_string(),
        ]);
        understanding.semantic_tags.sort();
        understanding.semantic_tags.dedup();
        let plan = self
            .core
            .generate_plan(&PlanGoalIR {
                schema: PLAN_GOAL_SCHEMA.to_string(),
                goal_id: request.request_id.clone(),
                intent: dockable_semantic_core::PlanIntentIR::Create,
                subject: request.title.clone(),
                constraints: vec![
                    format!("exact A4 page budget: {}", request.target_page_count),
                    "every factual paragraph retains an evidence and source-location binding"
                        .to_string(),
                    "missing evidence remains explicit and is never rendered as zero".to_string(),
                    "working memory preserves canonical terms, numeric facts, and open issues"
                        .to_string(),
                    "global consistency is rechecked after every bounded revision".to_string(),
                ],
                desired_outcomes: vec![
                    "produce a dependency-ordered long-form document plan".to_string(),
                    "synthesize each section only from observable evidence".to_string(),
                    "iterate drafting and correction without external model calls".to_string(),
                ],
                context_tags: understanding.semantic_tags,
                max_steps: request.max_plan_steps,
            })
            .map_err(map_planning_error)?;
        process_professional_document(&self.core, request, &plan.plan_sha256)
            .map_err(map_professional_document_error)
    }

    pub fn retained_experience_count(&self) -> usize {
        self.core.retained_experience_count()
    }

    pub fn submit_condition_evidence(
        &mut self,
        request: &ConditionEvidenceRequestIR,
    ) -> Result<ConditionEvidenceReceiptIR, CognitiveApiError> {
        self.conversation_memory
            .apply_condition_evidence(request)
            .map(|(receipt, _)| receipt)
            .map_err(|_| CognitiveApiError::ConditionEvidence)
    }

    pub fn submit_action_evidence(
        &mut self,
        request: &ActionEvidenceRequestIR,
    ) -> Result<ActionEvidenceReceiptIR, CognitiveApiError> {
        self.conversation_memory
            .apply_action_evidence(request)
            .map(|(receipt, _)| receipt)
            .map_err(|_| CognitiveApiError::ActionEvidence)
    }

    pub fn execute_command(&mut self, command: CognitiveApiCommandIR) -> CognitiveApiResponseIR {
        let result = match command {
            CognitiveApiCommandIR::DeliberateProblem { request } => self
                .deliberate_problem(&request)
                .map(|result| CognitiveApiPayloadIR::Deliberation(Box::new(result))),
            CognitiveApiCommandIR::ReviseDeliberation { request } => self
                .revise_deliberation(&request)
                .map(|result| CognitiveApiPayloadIR::DeliberationRevision(Box::new(result))),
            CognitiveApiCommandIR::InjectMechanismKnowledge { knowledge } => self
                .inject_mechanism_knowledge(knowledge)
                .map(CognitiveApiPayloadIR::MechanismKnowledgeInjectionReceipt),
            CognitiveApiCommandIR::ExportMechanismMemorySnapshot => {
                Ok(CognitiveApiPayloadIR::MechanismMemorySnapshot(
                    self.core.export_mechanism_memory_snapshot(),
                ))
            }
            CognitiveApiCommandIR::ImportMechanismMemorySnapshot { snapshot } => self
                .core
                .import_mechanism_memory_snapshot(&snapshot)
                .map(CognitiveApiPayloadIR::MechanismKnowledgeInjectionReceipts)
                .map_err(map_mechanism_memory_error),
            CognitiveApiCommandIR::DeliberateWithKnowledge { request, query } => self
                .deliberate_with_knowledge(&request, &query)
                .map(|result| {
                    CognitiveApiPayloadIR::KnowledgeGroundedDeliberation(Box::new(result))
                }),
            CognitiveApiCommandIR::InduceAndInjectMechanismKnowledge { request } => self
                .induce_and_inject_mechanism_knowledge(&request)
                .map(|result| CognitiveApiPayloadIR::MechanismInductionResponse(Box::new(result))),
            CognitiveApiCommandIR::InduceAndInjectRawMechanismKnowledge { request } => self
                .induce_and_inject_raw_mechanism_knowledge(&request)
                .map(|result| {
                    CognitiveApiPayloadIR::RawMechanismInductionResponse(Box::new(result))
                }),
            CognitiveApiCommandIR::InjectExperience { experience } => self
                .inject_experience(experience)
                .map(CognitiveApiPayloadIR::ExperienceInjectionReceipt),
            CognitiveApiCommandIR::ExportExperienceSnapshot => Ok(
                CognitiveApiPayloadIR::ExperienceSnapshot(self.core.export_experience_snapshot()),
            ),
            CognitiveApiCommandIR::ImportExperienceSnapshot { snapshot } => self
                .core
                .import_experience_snapshot(&snapshot)
                .map(CognitiveApiPayloadIR::ExperienceInjectionReceipts)
                .map_err(map_experience_error),
            CognitiveApiCommandIR::InjectLanguageKnowledge { entry } => self
                .inject_language_knowledge(entry)
                .map(CognitiveApiPayloadIR::LanguageKnowledgeInserted),
            CognitiveApiCommandIR::InjectLexeme { lexeme } => self
                .inject_lexeme(lexeme)
                .map(CognitiveApiPayloadIR::LexemeInserted),
            CognitiveApiCommandIR::ExportLexemeSnapshot => Ok(
                CognitiveApiPayloadIR::LexemeSnapshot(self.export_lexeme_snapshot()),
            ),
            CognitiveApiCommandIR::ImportLexemeSnapshot { snapshot } => self
                .import_lexeme_snapshot(&snapshot)
                .map(|()| CognitiveApiPayloadIR::LexemeSnapshotImported),
            CognitiveApiCommandIR::RecordLexicalOutcome { outcome } => self
                .record_lexical_outcome(&outcome)
                .map(|()| CognitiveApiPayloadIR::LexicalOutcomeRecorded),
            CognitiveApiCommandIR::ProcessNaturalLanguage { request } => self
                .process(&request)
                .map(|response| CognitiveApiPayloadIR::NaturalLanguageResponse(Box::new(response))),
            CognitiveApiCommandIR::ProcessConversationTurn { request } => {
                self.process_conversation_turn(&request).map(|response| {
                    CognitiveApiPayloadIR::ConversationTurnResponse(Box::new(response))
                })
            }
            CognitiveApiCommandIR::SubmitConditionEvidence { request } => self
                .submit_condition_evidence(&request)
                .map(CognitiveApiPayloadIR::ConditionEvidenceReceipt),
            CognitiveApiCommandIR::SubmitActionEvidence { request } => self
                .submit_action_evidence(&request)
                .map(CognitiveApiPayloadIR::ActionEvidenceReceipt),
            CognitiveApiCommandIR::ProcessKnowledgeWork { request } => self
                .process_knowledge_work(&request)
                .map(|response| CognitiveApiPayloadIR::KnowledgeWorkResponse(Box::new(response))),
            CognitiveApiCommandIR::ProcessLongTermRepairPlan { request } => self
                .process_long_term_repair_plan(&request)
                .map(|response| {
                    CognitiveApiPayloadIR::LongTermRepairPlanResponse(Box::new(response))
                }),
            CognitiveApiCommandIR::ProcessProfessionalDocument { request } => self
                .process_professional_document(&request)
                .map(|response| {
                    CognitiveApiPayloadIR::ProfessionalDocumentResponse(Box::new(response))
                }),
            CognitiveApiCommandIR::LanguageKnowledgeStatistics => {
                Ok(CognitiveApiPayloadIR::LanguageKnowledgeStatistics(
                    self.language_knowledge_statistics(),
                ))
            }
            CognitiveApiCommandIR::LexicalMemoryStatistics => Ok(
                CognitiveApiPayloadIR::LexicalMemoryStatistics(self.lexical_memory_statistics()),
            ),
        };
        match result {
            Ok(payload) => CognitiveApiResponseIR {
                ok: true,
                payload: Some(payload),
                error: None,
            },
            Err(error) => CognitiveApiResponseIR {
                ok: false,
                payload: None,
                error: Some(error),
            },
        }
    }

    pub fn execute_command_json(&mut self, json: &str) -> Result<String, CognitiveApiError> {
        let command = serde_json::from_str::<CognitiveApiCommandIR>(json)
            .map_err(|_| CognitiveApiError::JsonInput)?;
        serde_json::to_string(&self.execute_command(command))
            .map_err(|_| CognitiveApiError::JsonOutput)
    }
}

fn unresolved_action_request(interpretation: &PragmaticInterpretationIR) -> bool {
    interpretation.speech_act == SpeechActIR::RequestAction
        && interpretation.inferred_goal.is_none()
        && interpretation.compositional_analysis.frames.is_empty()
        && interpretation.pragmatic_intent_graph.primary.is_none()
        && interpretation
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .is_none_or(|composition| composition.nodes.is_empty())
}

fn validate_request(request: &NaturalLanguageRequestIR) -> Result<(), CognitiveApiError> {
    if request.schema != NATURAL_LANGUAGE_REQUEST_SCHEMA
        || request.request_id.trim().is_empty()
        || request.request_id.len() > 128
        || request.text.trim().is_empty()
        || request.text.len() > 64 * 1024
        || !(5..=32).contains(&request.max_plan_steps)
        || request.context_tags.len() > 64
        || request
            .context_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
    {
        return Err(CognitiveApiError::InvalidRequest);
    }
    Ok(())
}

fn map_experience_error(_: ExperienceError) -> CognitiveApiError {
    CognitiveApiError::Experience
}

fn map_language_error(_: LanguageKnowledgeError) -> CognitiveApiError {
    CognitiveApiError::LanguageKnowledge
}

fn map_conversation_error(_: ConversationFrontendError) -> CognitiveApiError {
    CognitiveApiError::ConversationFrontend
}

fn build_pending_question(
    request: &ConversationTurnRequestIR,
    normalization: &NormalizedUtteranceIR,
    resolution: &ReferenceResolutionIR,
    interpretation: &PragmaticInterpretationIR,
    native: &NativeTurnIR,
    prior_state: Option<&ConversationStateIR>,
    topic_id: Option<&str>,
) -> Option<QuestionUnderDiscussionIR> {
    let language = request
        .output_language
        .filter(|language| matches!(language, LanguageCodeIR::Korean | LanguageCodeIR::English))
        .unwrap_or_else(|| conversational_language(&request.raw_text));
    let (kind, mut options) = if normalization.ambiguous_input {
        (
            QuestionUnderDiscussionKindIR::VoiceAlternative,
            normalization
                .candidates
                .iter()
                .take(8)
                .enumerate()
                .map(|(index, candidate)| QuestionOptionIR {
                    option_id: format!("QOPT-{:06}-{:02}", request.turn_index, index + 1),
                    display_surface: candidate.source_text.clone(),
                    resolved_semantic_text: restore_user_grounded_acronyms(
                        &candidate.normalized_text,
                        &candidate.source_text,
                    ),
                    referent_ids: Vec::new(),
                    intent: None,
                })
                .collect::<Vec<_>>(),
        )
    } else if interpretation.compositional_analysis.clarification_required {
        let analysis = &interpretation.compositional_analysis;
        (
            QuestionUnderDiscussionKindIR::CompetingGoal,
            analysis
                .candidates
                .iter()
                .filter(|candidate| candidate.disposition == CandidateDispositionIR::Viable)
                .take(8)
                .enumerate()
                .filter_map(|(index, candidate)| {
                    let frame = analysis
                        .frames
                        .iter()
                        .find(|frame| frame.frame_id == candidate.source_frame_id)?;
                    let surface = realize_question_candidate(
                        &candidate.subject,
                        &frame.predicate_surface,
                        candidate.intent,
                        language,
                    );
                    Some(QuestionOptionIR {
                        option_id: format!("QOPT-{:06}-{:02}", request.turn_index, index + 1),
                        display_surface: surface.clone(),
                        resolved_semantic_text: restore_user_grounded_acronyms(
                            &surface,
                            &request.raw_text,
                        ),
                        referent_ids: Vec::new(),
                        intent: Some(candidate.intent),
                    })
                })
                .collect::<Vec<_>>(),
        )
    } else if native.response_goal == NativeResponseGoalIR::AskClarification
        && native
            .unresolved
            .iter()
            .any(|reason| reason == "AMBIGUOUS_DIALOGUE_CONTEXT_ENTITY")
        && native
            .events
            .iter()
            .filter(|event| event.scope == NativeEventScopeIR::Live)
            .count()
            == 1
    {
        let event = native
            .events
            .iter()
            .find(|event| event.scope == NativeEventScopeIR::Live)?;
        let prior_state = prior_state?;
        let latest_mention = prior_state
            .active_typed_entities
            .iter()
            .map(|entity| entity.last_mentioned_turn)
            .max()?;
        (
            QuestionUnderDiscussionKindIR::CompetingGoal,
            prior_state
                .active_typed_entities
                .iter()
                .filter(|entity| entity.last_mentioned_turn == latest_mention)
                .take(8)
                .enumerate()
                .map(|(index, entity)| {
                    let surface = realize_question_candidate(
                        &entity.canonical_surface,
                        &event.predicate_surface,
                        event.intent,
                        language,
                    );
                    QuestionOptionIR {
                        option_id: format!("QOPT-{:06}-{:02}", request.turn_index, index + 1),
                        display_surface: entity.canonical_surface.clone(),
                        resolved_semantic_text: surface,
                        referent_ids: vec![entity.entity_id.clone()],
                        intent: Some(event.intent),
                    }
                })
                .collect::<Vec<_>>(),
        )
    } else if resolution
        .ambiguous_reference_surfaces
        .iter()
        .any(|surface| surface == "ELLIPTICAL_ACTION" || surface == "ELLIPTICAL_GOAL")
    {
        let goals = prior_state
            .map(|state| state.active_goals.as_slice())
            .unwrap_or_default();
        let shared_subject = goals
            .iter()
            .map(|goal| goal.subject.trim())
            .filter(|subject| !subject.is_empty())
            .min_by_key(|subject| subject.chars().count())
            .unwrap_or_default();
        (
            QuestionUnderDiscussionKindIR::RepeatedGoal,
            goals
                .iter()
                .take(8)
                .enumerate()
                .map(|(index, goal)| {
                    let surface = realize_question_candidate(
                        shared_subject,
                        &goal.predicate_surface,
                        goal.intent,
                        language,
                    );
                    QuestionOptionIR {
                        option_id: format!("QOPT-{:06}-{:02}", request.turn_index, index + 1),
                        display_surface: surface.clone(),
                        resolved_semantic_text: surface,
                        referent_ids: Vec::new(),
                        intent: Some(goal.intent),
                    }
                })
                .collect::<Vec<_>>(),
        )
    } else if resolution
        .ambiguous_reference_surfaces
        .iter()
        .any(|surface| surface == "Proposition_REFERENCE")
    {
        let propositions = prior_state
            .map(|state| {
                let latest_turn = state
                    .active_discourse_referents
                    .iter()
                    .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
                    .map(|referent| referent.last_referenced_turn)
                    .max();
                state
                    .active_discourse_referents
                    .iter()
                    .filter(|referent| {
                        referent.kind == DiscourseReferentKindIR::Proposition
                            && Some(referent.last_referenced_turn) == latest_turn
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (
            QuestionUnderDiscussionKindIR::PropositionReference,
            propositions
                .iter()
                .take(8)
                .enumerate()
                .map(|(index, referent)| {
                    let surface = referent.semantic_summary.trim().to_string();
                    let resolved = match language {
                        LanguageCodeIR::Korean => format!("{surface} 내용을 설명해"),
                        _ => format!("explain {surface}"),
                    };
                    QuestionOptionIR {
                        option_id: format!("QOPT-{:06}-{:02}", request.turn_index, index + 1),
                        display_surface: surface,
                        resolved_semantic_text: resolved,
                        referent_ids: vec![referent.referent_id.clone()],
                        intent: Some(PlanIntentIR::Explain),
                    }
                })
                .collect::<Vec<_>>(),
        )
    } else {
        return None;
    };
    options.sort_by(|left, right| left.option_id.cmp(&right.option_id));
    options.dedup_by(|left, right| left.resolved_semantic_text == right.resolved_semantic_text);
    if !(2..=8).contains(&options.len()) {
        return None;
    }
    Some(QuestionUnderDiscussionIR {
        question_id: format!("QUD-{:06}-{kind:?}", request.turn_index),
        kind,
        topic_id: topic_id.map(ToString::to_string),
        source_turn: request.turn_index,
        source_request: normalization.semantic_surface_text.clone(),
        external_execution_authorized: interpretation
            .compositional_analysis
            .candidates
            .iter()
            .any(|candidate| candidate.external_execution_authorized),
        options,
    })
}

fn realize_question_candidate(
    subject: &str,
    predicate_surface: &str,
    intent: PlanIntentIR,
    language: LanguageCodeIR,
) -> String {
    let action = localized_question_action(predicate_surface, intent, language);
    match language {
        LanguageCodeIR::Korean => format!("{subject} {action}"),
        _ => format!("{action} {subject}"),
    }
}

fn localized_question_action(
    predicate_surface: &str,
    intent: PlanIntentIR,
    language: LanguageCodeIR,
) -> String {
    let form = predicate_surface.to_lowercase();
    let aliases = match language {
        LanguageCodeIR::Korean => [
            (["open", "열"].as_slice(), "열어"),
            (["read", "읽"].as_slice(), "읽어"),
            (["save", "저장"].as_slice(), "저장해"),
            (["delete", "clear", "삭제", "지우"].as_slice(), "삭제해"),
            (["inspect", "check", "확인", "검사"].as_slice(), "확인해"),
            (["analyze", "분석"].as_slice(), "분석해"),
            (
                ["repair", "fix", "수정", "수리", "고치"].as_slice(),
                "수리해",
            ),
            (["create", "write", "작성", "생성"].as_slice(), "작성해"),
            (["explain", "설명"].as_slice(), "설명해"),
            (["summarize", "요약"].as_slice(), "요약해"),
        ],
        _ => [
            (["open", "열"].as_slice(), "open"),
            (["read", "읽"].as_slice(), "read"),
            (["save", "저장"].as_slice(), "save"),
            (["delete", "clear", "삭제", "지우"].as_slice(), "delete"),
            (["inspect", "check", "확인", "검사"].as_slice(), "inspect"),
            (["analyze", "분석"].as_slice(), "analyze"),
            (
                ["repair", "fix", "수정", "수리", "고치"].as_slice(),
                "repair",
            ),
            (["create", "write", "작성", "생성"].as_slice(), "create"),
            (["explain", "설명"].as_slice(), "explain"),
            (["summarize", "요약"].as_slice(), "summarize"),
        ],
    };
    aliases
        .iter()
        .find(|(needles, _)| needles.iter().any(|needle| form.contains(needle)))
        .map(|(_, action)| (*action).to_string())
        .unwrap_or_else(|| match (language, intent) {
            (LanguageCodeIR::Korean, PlanIntentIR::Explain) => "설명해".to_string(),
            (LanguageCodeIR::Korean, PlanIntentIR::Investigate) => "확인해".to_string(),
            (LanguageCodeIR::Korean, PlanIntentIR::Repair) => "수리해".to_string(),
            (LanguageCodeIR::Korean, PlanIntentIR::Create) => "작성해".to_string(),
            (LanguageCodeIR::Korean, _) => "실행해".to_string(),
            (_, PlanIntentIR::Explain) => "explain".to_string(),
            (_, PlanIntentIR::Investigate) => "inspect".to_string(),
            (_, PlanIntentIR::Repair) => "repair".to_string(),
            (_, PlanIntentIR::Create) => "create".to_string(),
            _ => "execute".to_string(),
        })
}

fn conversation_focus_candidates(
    interpretation: &PragmaticInterpretationIR,
    grounded_goals: &[ConversationGoalFrameIR],
    proposition_referents: &[DynamicDiscourseReferentIR],
) -> Vec<DiscourseFocusCandidateIR> {
    let analysis = &interpretation.compositional_analysis;
    if !grounded_goals.is_empty() {
        let selected = analysis
            .selected_candidates()
            .into_iter()
            .filter_map(|candidate| {
                analysis
                    .frames
                    .iter()
                    .find(|frame| frame.frame_id == candidate.source_frame_id)
                    .map(|frame| (candidate, frame))
            })
            .collect::<Vec<_>>();
        let mut focus_candidates = Vec::new();
        for (index, ((candidate, frame), goal)) in
            selected.into_iter().zip(grounded_goals).enumerate()
        {
            let clause = analysis.clause_graph.node_for_frame(&frame.frame_id);
            let incoming = clause.and_then(|clause| {
                analysis
                    .clause_graph
                    .edges
                    .iter()
                    .filter(|edge| edge.target_clause_id == clause.clause_id)
                    .max_by_key(|edge| focus_relation_weight(edge.relation))
            });
            let clause_function = clause.map(|clause| clause.function);
            let governing_relation = incoming.map(|edge| edge.relation);
            let base = match clause_function {
                Some(ClauseFunctionIR::Main) => 860_u16,
                Some(ClauseFunctionIR::Coordinate) => 840,
                _ => 700,
            };
            let relation_bonus = governing_relation.map_or(0, focus_relation_weight);
            let order_bonus = u16::try_from(index.min(10) * 5).unwrap_or(50);
            focus_candidates.push(DiscourseFocusCandidateIR {
                surface: goal.subject.clone(),
                concept_id_hint: discourse_topic_concept_id(&goal.subject),
                source: DiscourseFocusSourceIR::GroundedGoal,
                source_frame_id: Some(frame.frame_id.clone()),
                source_clause_id: clause.map(|clause| clause.clause_id.clone()),
                clause_function,
                governing_relation,
                salience_millis: base
                    .saturating_add(relation_bonus)
                    .saturating_add(order_bonus)
                    .min(990),
                source_order: index,
                evidence: vec![
                    "CLAUSE_GRAPH_CENTERING".to_string(),
                    format!("CANDIDATE_ID:{}", candidate.candidate_id),
                    format!("SOURCE_FRAME_ID:{}", frame.frame_id),
                ],
            });

            if matches!(
                frame.predicate_surface.to_lowercase().as_str(),
                "compare" | "contrast"
            ) {
                let comparison_peers = analysis
                    .semantic_role_graph
                    .arguments_for_frame(&frame.frame_id)
                    .into_iter()
                    .filter(|(role, node)| {
                        matches!(
                            role,
                            SemanticRoleKindIR::ComparisonPeer | SemanticRoleKindIR::CoTheme
                        ) && !node.normalized_label.eq_ignore_ascii_case(&goal.subject)
                    })
                    .collect::<Vec<_>>();
                for (peer_index, (_, peer)) in comparison_peers.into_iter().enumerate() {
                    focus_candidates.push(DiscourseFocusCandidateIR {
                        surface: peer.normalized_label.clone(),
                        concept_id_hint: discourse_topic_concept_id(&peer.normalized_label),
                        source: DiscourseFocusSourceIR::GroundedGoal,
                        source_frame_id: Some(frame.frame_id.clone()),
                        source_clause_id: clause.map(|clause| clause.clause_id.clone()),
                        clause_function,
                        governing_relation,
                        salience_millis: base
                            .saturating_add(relation_bonus)
                            .saturating_add(order_bonus)
                            .min(990),
                        source_order: grounded_goals.len() + peer_index,
                        evidence: vec![
                            "CLAUSE_GRAPH_CENTERING".to_string(),
                            "PARALLEL_COMPARISON_FOCUS".to_string(),
                            format!("CANDIDATE_ID:{}", candidate.candidate_id),
                            format!("SOURCE_FRAME_ID:{}", frame.frame_id),
                        ],
                    });
                }
            }
        }
        return focus_candidates;
    }

    proposition_referents
        .iter()
        .enumerate()
        .filter_map(|(index, proposition)| {
            let signature = crate::epistemic::proposition_signature(
                &proposition.semantic_summary,
                proposition
                    .proposition_polarity
                    .unwrap_or(crate::attribution::AttributedPropositionPolarityIR::Positive),
            );
            proposition_focus_surface(&proposition.semantic_summary, &signature.subject_key).map(
                |surface| DiscourseFocusCandidateIR {
                    concept_id_hint: discourse_topic_concept_id(&surface),
                    surface,
                    source: DiscourseFocusSourceIR::Proposition,
                    source_frame_id: None,
                    source_clause_id: None,
                    clause_function: None,
                    governing_relation: None,
                    salience_millis: 720_u16
                        .saturating_add(u16::try_from(index.min(10) * 5).unwrap_or(50)),
                    source_order: index,
                    evidence: vec![
                        "PROPOSITION_SUBJECT_CENTERING".to_string(),
                        format!("REFERENT_ID:{}", proposition.referent_id),
                        "DIALOGUE_TRUTH_ESTABLISHED:false".to_string(),
                    ],
                },
            )
        })
        .collect()
}

fn proposition_focus_surface(text: &str, fallback: &str) -> Option<String> {
    let normalized = text.trim().to_lowercase();
    let clean = |surface: &str| {
        surface
            .trim()
            .trim_matches(|character: char| {
                !character.is_alphanumeric() && !character.is_whitespace()
            })
            .strip_prefix("the ")
            .unwrap_or_else(|| surface.trim())
            .trim()
            .to_string()
    };
    let weak = |surface: &str| {
        matches!(
            surface.trim(),
            "" | "unknown_subject" | "this" | "that" | "it" | "we" | "they" | "deployment"
        )
    };

    for marker in [
        " has ", " have ", " keeps ", " kept ", " is ", " are ", " was ", " were ",
    ] {
        if let Some(position) = normalized.find(marker) {
            let candidate = clean(&normalized[..position]);
            if !weak(&candidate) {
                return Some(candidate);
            }
        }
    }
    if let Some(position) = normalized.find(" leave ") {
        let tail = &normalized[position + " leave ".len()..];
        let end = [" like ", " as ", ",", "."]
            .iter()
            .filter_map(|marker| tail.find(marker))
            .min()
            .unwrap_or(tail.len());
        let candidate = clean(&tail[..end]);
        if !weak(&candidate) {
            return Some(candidate);
        }
    }
    let fallback = clean(&fallback.replace('_', " "));
    (!weak(&fallback)).then_some(fallback)
}

fn focus_relation_weight(relation: ClauseRelationKindIR) -> u16 {
    match relation {
        ClauseRelationKindIR::Contrast => 100,
        ClauseRelationKindIR::Sequence | ClauseRelationKindIR::TemporalBefore => 80,
        ClauseRelationKindIR::Coordination => 60,
        ClauseRelationKindIR::Condition
        | ClauseRelationKindIR::Cause
        | ClauseRelationKindIR::Purpose => 40,
    }
}

/// Projects the exact semantic goal accepted by planning into conversation
/// memory. Parser and native-circuit candidates are evidence used before
/// semantic selection; re-reading either candidate set here would give a
/// compatibility view a second chance to erase or replace the selected goal.
fn semantic_plan_conversation_goal_frames(
    semantic_goal: &SemanticPlanGoalIR,
    interpretation: &PragmaticInterpretationIR,
    native: &NativeTurnIR,
    turn_index: u64,
    source_semantic_text: &str,
) -> Vec<ConversationGoalFrameIR> {
    semantic_goal
        .selected_live_event_ids
        .iter()
        .enumerate()
        .filter_map(|(index, event_id)| {
            let event = semantic_goal
                .events
                .iter()
                .find(|event| &event.event_id == event_id)?;
            let semantic_subject = event
                .goal_subject_argument_ids
                .iter()
                .filter_map(|argument_id| {
                    semantic_goal
                        .arguments
                        .iter()
                        .find(|argument| &argument.argument_id == argument_id)
                        .map(|argument| argument.grounded_label.as_str())
                })
                .collect::<Vec<_>>()
                .join(" & ");
            (!semantic_subject.trim().is_empty()).then(|| {
                let canonical_predicate = event
                    .predicate_concept_id
                    .strip_prefix("C_")
                    .unwrap_or(&event.predicate_concept_id)
                    .to_string();
                let subject = semantic_subject;
                ConversationGoalFrameIR {
                    goal_id: format!("GOAL-{turn_index:06}-{:02}", index + 1),
                    intent: event.intent,
                    predicate_surface: semantic_event_predicate_surface(
                        event,
                        &subject,
                        interpretation,
                        native,
                    )
                    .unwrap_or_else(|| canonical_predicate.to_lowercase()),
                    canonical_predicate,
                    subject,
                    source_semantic_text: source_semantic_text.trim().to_string(),
                    introduced_turn: turn_index,
                    last_referenced_turn: turn_index,
                    external_execution_authorized: event.external_execution_authorized,
                }
            })
        })
        .collect()
}

fn semantic_event_predicate_surface(
    semantic_event: &SemanticPlanEventIR,
    semantic_subject: &str,
    interpretation: &PragmaticInterpretationIR,
    native: &NativeTurnIR,
) -> Option<String> {
    interpretation
        .language_center
        .events
        .iter()
        .find(|event| event.event_id == semantic_event.event_id)
        .and_then(|event| {
            interpretation
                .compositional_analysis
                .frames
                .iter()
                .find(|frame| frame.frame_id == event.source_frame_id)
        })
        .map(|frame| frame.predicate_surface.clone())
        .or_else(|| {
            native
                .selected_live_goals
                .iter()
                .find(|goal| {
                    goal.intent == semantic_event.intent
                        && subjects_semantically_overlap(&goal.subject, semantic_subject)
                })
                .and_then(|goal| {
                    native
                        .events
                        .iter()
                        .find(|event| event.event_id == goal.source_event_id)
                })
                .map(|event| event.predicate_surface.clone())
        })
        .or_else(|| {
            interpretation
                .compositional_analysis
                .selected_candidates()
                .into_iter()
                .find(|candidate| {
                    candidate.intent == semantic_event.intent
                        && subjects_semantically_overlap(&candidate.subject, semantic_subject)
                })
                .and_then(|candidate| {
                    interpretation
                        .compositional_analysis
                        .frames
                        .iter()
                        .find(|frame| frame.frame_id == candidate.source_frame_id)
                })
                .map(|frame| frame.predicate_surface.clone())
        })
}

fn remember_native_dialogue_turn(
    memory: &mut BTreeMap<String, NativeDialogueContextIR>,
    conversation_id: &str,
    turn_index: u64,
    turn: &NativeTurnIR,
) {
    let state = memory.entry(conversation_id.to_string()).or_default();
    let explicit_entities = turn
        .entities
        .iter()
        .filter(|entity| {
            entity.start_byte < entity.end_byte
                && !entity.rejected_by_contrast
                && entity.canonical_concept != "C_TASK"
                && entity.canonical_concept != "C_ISSUE"
                && entity.canonical_concept != "C_PROBLEM"
        })
        .collect::<Vec<_>>();
    if !explicit_entities.is_empty() {
        state.active_entities = explicit_entities
            .into_iter()
            .map(|entity| NativeContextEntityIR {
                referent_id: format!("NATIVE:{}:{}", turn_index, entity.entity_id),
                surface: entity.surface.clone(),
                introduced_turn: turn_index,
                last_mentioned_turn: turn_index,
            })
            .collect();
    }
    if !turn.selected_live_goals.is_empty() {
        state.active_goals = turn
            .selected_live_goals
            .iter()
            .map(|goal| NativeContextGoalIR {
                goal_id: format!("NATIVE:{}:{}", turn_index, goal.goal_id),
                intent: goal.intent,
                canonical_predicate: goal.canonical_predicate.clone(),
                subject: goal.subject.clone(),
                introduced_turn: turn_index,
                discourse_focused: true,
                operation_replayable: true,
            })
            .collect();
    }
}

fn native_goal_projection_required(
    interpretation: &PragmaticInterpretationIR,
    native: &NativeTurnIR,
) -> bool {
    let Some(native_goals) = native.authoritative_live_goals() else {
        return false;
    };
    let legacy_surface_is_question = interpretation.speech_act == SpeechActIR::Ask
        || interpretation
            .clauses
            .iter()
            .any(|clause| clause.surface_text.trim_end().ends_with('?'));
    if legacy_surface_is_question
        && native_goals
            .iter()
            .all(|goal| native_subject_is_interrogative_placeholder(&goal.subject))
    {
        return false;
    }
    let legacy_candidates = interpretation.compositional_analysis.selected_candidates();
    if !native.reference_bindings.is_empty() {
        return true;
    }
    // Response-policy directives belong to the dialogue control plane, not
    // the semantic task plane. If a compatibility parser promoted one as an
    // extra task candidate, the complete native live-goal set must own final
    // selection so that the directive cannot leak into SemanticPlanGoalIR.
    if legacy_candidates
        .iter()
        .any(|candidate| is_dialogue_directive_goal(&candidate.subject))
    {
        return true;
    }
    // A fully bound native graph owns scope exclusions.  A larger legacy
    // candidate count is not evidence of greater completeness when the extra
    // candidates are conditional, prohibited, reported, or merely possible.
    // In that situation the count difference is exactly why native projection
    // is required: only live goals may reach the planner.
    let native_has_non_live_events = native
        .events
        .iter()
        .any(|event| event.scope != NativeEventScopeIR::Live);
    if legacy_candidates.len() > native_goals.len() {
        return native_has_non_live_events;
    }
    if legacy_candidates.is_empty() && interpretation.inferred_goal.is_none() {
        return true;
    }
    if native_goals.len() == 1 && legacy_candidates.len() == 1 {
        let native_goal = &native_goals[0];
        let legacy_goal = legacy_candidates[0];
        if legacy_goal.intent != native_goal.intent
            || !subjects_semantically_overlap(&legacy_goal.subject, &native_goal.subject)
        {
            return true;
        }
    }
    // Prefer the typed entity/event binding when both paths found the same
    // operation but the legacy surface extractor selected a discourse marker
    // (for example Korean topic or causal nouns) as its subject.
    if native_goals.len() == 1
        && legacy_candidates.len() == 1
        && interpretation.inferred_goal.as_ref().is_some_and(|legacy| {
            legacy.intent == native_goals[0].intent
                && legacy_subject_is_discourse_placeholder(&legacy.subject)
                && !subjects_semantically_overlap(&legacy.subject, &native_goals[0].subject)
        })
    {
        return true;
    }
    // Preserve a complete legacy plan when it already agrees structurally.
    // Native ownership is required only when the legacy path has no plan or
    // the native circuit supplied an explicit cross-turn binding.
    false
}

fn legacy_subject_is_discourse_placeholder(subject: &str) -> bool {
    matches!(
        subject.trim().to_lowercase().as_str(),
        "말인데" | "원인" | "그런지" | "why" | "reason" | "cause"
    )
}

fn subjects_semantically_overlap(left: &str, right: &str) -> bool {
    let tokens = |subject: &str| {
        subject
            .to_lowercase()
            .split(|character: char| !character.is_alphanumeric())
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
    };
    let left_tokens = tokens(left);
    let right_tokens = tokens(right);
    !left_tokens.is_disjoint(&right_tokens)
        || left.to_lowercase().contains(&right.to_lowercase())
        || right.to_lowercase().contains(&left.to_lowercase())
}

fn native_subject_is_interrogative_placeholder(subject: &str) -> bool {
    matches!(
        subject.trim().to_lowercase().as_str(),
        "what"
            | "who"
            | "where"
            | "when"
            | "why"
            | "how"
            | "which"
            | "무엇"
            | "뭐"
            | "누구"
            | "어디"
            | "언제"
            | "왜"
            | "어떻게"
            | "무슨"
    )
}

fn native_intent_from_predicate(canonical_predicate: &str) -> PlanIntentIR {
    match canonical_predicate.trim().to_uppercase().as_str() {
        "INVESTIGATE" | "INSPECT" | "CHECK" | "ANALYZE" => PlanIntentIR::Investigate,
        "REPAIR" | "FIX" => PlanIntentIR::Repair,
        "EXPLAIN" => PlanIntentIR::Explain,
        "CREATE" | "BUILD" | "WRITE" => PlanIntentIR::Create,
        "LEARN" => PlanIntentIR::Learn,
        "PLAN" => PlanIntentIR::Plan,
        "COMMUNICATE" | "TELL" => PlanIntentIR::Communicate,
        _ => PlanIntentIR::Execute,
    }
}

fn conversation_discourse_program(
    interpretation: &PragmaticInterpretationIR,
    turn_index: u64,
    grounded_goals: &[ConversationGoalFrameIR],
    deferred_commitments: &[DeferredActionCommitmentIR],
) -> Option<DiscourseProgramIR> {
    let analysis = &interpretation.compositional_analysis;
    if analysis.frames.len() < 2 || grounded_goals.is_empty() {
        return None;
    }
    let selected = analysis
        .selected_candidates()
        .into_iter()
        .filter_map(|candidate| {
            analysis
                .frames
                .iter()
                .find(|frame| frame.frame_id == candidate.source_frame_id)
                .map(|frame| (candidate, frame))
        })
        .collect::<Vec<_>>();
    if selected.len() != grounded_goals.len() {
        return None;
    }
    let shared_subject = grounded_goals
        .first()
        .map(|goal| goal.subject.trim().to_string())
        .unwrap_or_default();
    let mut staged_steps = selected
        .iter()
        .zip(grounded_goals)
        .map(|((_, frame), goal)| {
            (
                frame.source_start_byte,
                frame.frame_id.clone(),
                DiscourseProgramStepIR {
                    position: 0,
                    goal: goal.clone(),
                    relation_from_previous: None,
                    guard: None,
                    semantic_authority: false,
                    external_execution_authorized: false,
                },
            )
        })
        .collect::<Vec<_>>();
    for (index, commitment) in deferred_commitments.iter().enumerate() {
        if discourse_program_subject_key(&commitment.action.subject)
            != discourse_program_subject_key(&shared_subject)
        {
            continue;
        }
        let Some(conditional) = analysis
            .modal_scope_graph
            .conditionals
            .iter()
            .find(|conditional| {
                conditional.consequent_is_directive
                    && condition_sha256(&deferred_condition_surface(conditional))
                        == commitment.condition_sha256
            })
            .filter(|conditional| {
                conditional.kind != crate::modality::ConditionalKindIR::Counterfactual
            })
        else {
            continue;
        };
        let Some(parsed_condition) = parse_discourse_guard_condition_expression(
            &conditional.antecedent,
            conditional.antecedent_negated,
        ) else {
            continue;
        };
        if parsed_condition.explicit_subjects.iter().any(|subject| {
            discourse_program_subject_key(subject) != discourse_program_subject_key(&shared_subject)
        }) {
            continue;
        }
        let Some((candidate, frame)) = analysis
            .candidates
            .iter()
            .filter_map(|candidate| {
                analysis
                    .frames
                    .iter()
                    .find(|frame| frame.frame_id == candidate.source_frame_id)
                    .map(|frame| (candidate, frame))
            })
            .filter(|(candidate, frame)| {
                candidate.intent == commitment.action.intent
                    && frame.canonical_predicate == commitment.action.canonical_predicate
                    && !frame.embedded_under_quote
            })
            .max_by_key(|(candidate, _)| candidate.score_millis)
        else {
            continue;
        };
        if staged_steps
            .iter()
            .any(|(_, frame_id, _)| frame_id == &frame.frame_id)
        {
            continue;
        }
        let guard_condition_surface = commitment.condition_surface.clone();
        let canonical_condition_predicate =
            parsed_condition.expression.legacy_predicate().to_string();
        let condition_expression_sha256 =
            guard_condition_expression_sha256(&parsed_condition.expression);
        let guard = DiscourseProgramGuardIR {
            schema: DISCOURSE_PROGRAM_GUARD_SCHEMA.to_string(),
            kind: conditional.kind,
            antecedent_surface: guard_condition_surface.clone(),
            normalized_antecedent: normalize_condition(&guard_condition_surface),
            condition_sha256: condition_sha256(&guard_condition_surface),
            deferred_commitment_id: commitment.commitment_id.clone(),
            canonical_condition_predicate,
            condition_expression: parsed_condition.expression,
            condition_expression_sha256,
            source_subject: shared_subject.clone(),
            antecedent_negated: conditional.antecedent_negated,
            requires_verified_evidence: true,
            semantic_authority: false,
            external_execution_authorized: false,
        };
        staged_steps.push((
            frame.source_start_byte,
            frame.frame_id.clone(),
            DiscourseProgramStepIR {
                position: 0,
                goal: ConversationGoalFrameIR {
                    goal_id: format!("DEFERRED-GOAL-{turn_index:06}-{:02}", index + 1),
                    intent: candidate.intent,
                    canonical_predicate: frame.canonical_predicate.clone(),
                    predicate_surface: frame.predicate_surface.clone(),
                    subject: shared_subject.clone(),
                    source_semantic_text: commitment.action.source_semantic_text.clone(),
                    introduced_turn: turn_index,
                    last_referenced_turn: turn_index,
                    external_execution_authorized: false,
                },
                relation_from_previous: None,
                guard: Some(guard),
                semantic_authority: false,
                external_execution_authorized: false,
            },
        ));
    }
    staged_steps.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let frame_order = staged_steps
        .iter()
        .map(|(_, frame_id, _)| frame_id.clone())
        .collect::<Vec<_>>();
    let steps = staged_steps
        .into_iter()
        .enumerate()
        .map(|(index, (_, _, mut step))| {
            step.position = u16::try_from(index + 1).unwrap_or(u16::MAX);
            step.relation_from_previous = if index == 0 {
                None
            } else if step.guard.is_some() {
                Some(ClauseRelationKindIR::Condition)
            } else {
                analysis
                    .clause_graph
                    .relation_between_frames(&frame_order[index - 1], &frame_order[index])
            };
            step
        })
        .collect::<Vec<_>>();
    let represented_frame_ids = frame_order
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let shared_subject_key = discourse_program_subject_key(&shared_subject);
    let blocked_frame_count = analysis
        .frames
        .iter()
        .filter(|frame| !represented_frame_ids.contains(frame.frame_id.as_str()))
        .filter(|frame| {
            let shares_bound_argument = analysis
                .semantic_role_graph
                .event_node_for_frame(&frame.frame_id)
                .is_some_and(|blocked_event| {
                    represented_frame_ids.iter().any(|represented_frame_id| {
                        analysis
                            .semantic_role_graph
                            .event_node_for_frame(represented_frame_id)
                            .is_some_and(|represented_event| {
                                analysis
                                    .semantic_role_graph
                                    .shared_argument_bindings
                                    .iter()
                                    .any(|binding| {
                                        (binding.provider_event_node_id == blocked_event.node_id
                                            && binding.dependent_event_node_id
                                                == represented_event.node_id)
                                            || (binding.dependent_event_node_id
                                                == blocked_event.node_id
                                                && binding.provider_event_node_id
                                                    == represented_event.node_id)
                                    })
                            })
                    })
                });
            let shares_program_subject = analysis
                .candidates
                .iter()
                .filter(|candidate| candidate.source_frame_id == frame.frame_id)
                .any(|candidate| {
                    discourse_program_subject_key(&candidate.subject) == shared_subject_key
                })
                || analysis
                    .semantic_role_graph
                    .primary_argument_for_frame(&frame.frame_id)
                    .is_some_and(|argument| {
                        discourse_program_subject_key(&argument.normalized_label)
                            == shared_subject_key
                    });
            let is_deferred_work = deferred_commitments.iter().any(|commitment| {
                commitment.action.canonical_predicate == frame.canonical_predicate
                    && analysis.candidates.iter().any(|candidate| {
                        candidate.source_frame_id == frame.frame_id
                            && candidate.intent == commitment.action.intent
                    })
            });
            shares_bound_argument || shares_program_subject || is_deferred_work
        })
        .count();
    let source_frame_count = steps.len() + blocked_frame_count;
    if source_frame_count < 2 {
        return None;
    }
    let subjects = steps
        .iter()
        .map(|step| step.goal.subject.trim().to_lowercase())
        .collect::<BTreeSet<_>>();
    let guarded_step_count = steps.iter().filter(|step| step.guard.is_some()).count();
    let replayable = blocked_frame_count == 0
        && steps.len() == source_frame_count
        && steps.len() >= 2
        && steps.iter().any(|step| step.guard.is_none())
        && subjects.len() == 1
        && steps.iter().enumerate().all(|(index, step)| {
            index == 0 && step.relation_from_previous.is_none() && step.guard.is_none()
                || index > 0
                    && matches!(
                        step.relation_from_previous,
                        Some(
                            ClauseRelationKindIR::Coordination
                                | ClauseRelationKindIR::Sequence
                                | ClauseRelationKindIR::Condition
                                | ClauseRelationKindIR::TemporalBefore
                        )
                    )
        });
    let mut program = DiscourseProgramIR {
        schema: DISCOURSE_PROGRAM_SCHEMA.to_string(),
        program_id: format!("DPROGRAM-{turn_index:06}"),
        source_frame_count,
        blocked_frame_count,
        guarded_step_count,
        shared_subject,
        steps,
        replayable,
        introduced_turn: turn_index,
        last_referenced_turn: turn_index,
        semantic_authority: false,
        external_execution_authorized: false,
        program_sha256: String::new(),
    };
    program.program_sha256 = discourse_program_sha256(&program);
    Some(program)
}

struct ParsedGuardConditionExpression {
    expression: GuardConditionExpressionIR,
    explicit_subjects: Vec<String>,
}

fn parse_discourse_guard_condition_expression(
    antecedent: &str,
    antecedent_negated: bool,
) -> Option<ParsedGuardConditionExpression> {
    let surface = trim_guard_conditional_suffix(antecedent);
    let mut parsed = parse_guard_condition_or(&surface)?;
    if antecedent_negated {
        parsed.expression = negate_guard_condition_expression(parsed.expression);
    }
    parsed.explicit_subjects.sort();
    parsed.explicit_subjects.dedup();
    parsed.expression.validate().then_some(parsed)
}

fn parse_guard_condition_or(surface: &str) -> Option<ParsedGuardConditionExpression> {
    let surface = strip_balanced_outer_parentheses(surface.trim());
    let parts = split_guard_condition_top_level(
        surface,
        &[" or ", " 또는 ", " 혹은 ", " 아니면 ", "거나 "],
    );
    if parts.len() > 1 {
        return combine_guard_condition(
            GuardConditionOperatorIR::Any,
            parts
                .iter()
                .map(|part| parse_guard_condition_and(part))
                .collect::<Option<Vec<_>>>()?,
        );
    }
    parse_guard_condition_and(surface)
}

fn parse_guard_condition_and(surface: &str) -> Option<ParsedGuardConditionExpression> {
    let surface = strip_balanced_outer_parentheses(surface.trim());
    let parts = split_guard_condition_top_level(surface, &[" and ", " 그리고 ", " 및 ", "고 "]);
    if parts.len() > 1 {
        return combine_guard_condition(
            GuardConditionOperatorIR::All,
            parts
                .iter()
                .map(|part| parse_guard_condition_unary(part))
                .collect::<Option<Vec<_>>>()?,
        );
    }
    parse_guard_condition_unary(surface)
}

fn parse_guard_condition_unary(surface: &str) -> Option<ParsedGuardConditionExpression> {
    let trimmed = surface.trim();
    let surface = strip_balanced_outer_parentheses(trimmed);
    if surface != trimmed {
        return parse_guard_condition_or(surface);
    }
    if let Some(rest) = surface
        .strip_prefix("not ")
        .or_else(|| surface.strip_prefix("NOT "))
    {
        let mut parsed = parse_guard_condition_unary(rest)?;
        parsed.expression = negate_guard_condition_expression(parsed.expression);
        return Some(parsed);
    }
    parse_guard_condition_atom(surface)
}

fn parse_guard_condition_atom(surface: &str) -> Option<ParsedGuardConditionExpression> {
    let normalized = surface
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim()
        .to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let negated_stale = normalized.contains("not stale")
        || normalized.contains("isn't stale")
        || normalized.contains("오래되지 않")
        || normalized.contains("오래되지 않았");
    let negated_damaged = normalized.contains("not damaged")
        || normalized.contains("isn't damaged")
        || normalized.contains("손상되지 않")
        || normalized.contains("깨지지 않");
    let negated_empty = normalized.contains("not empty")
        || normalized.contains("isn't empty")
        || normalized.contains("비어 있지 않")
        || normalized.contains("비지 않");
    let negated_error = normalized.contains("no error")
        || normalized.contains("without an error")
        || normalized.contains("without errors")
        || normalized.contains("오류가 없");
    let negated_problem = normalized.contains("no problem")
        || normalized.contains("without a problem")
        || normalized.contains("문제가 없");
    let negated_valid = normalized.contains("not valid")
        || normalized.contains("isn't valid")
        || normalized.contains("유효하지 않")
        || normalized.contains("유효하지");
    let (predicate, negated, subject_marker) =
        if normalized.contains("unhealthy") || normalized.contains("건강하지") {
            ("UNHEALTHY", false, &["unhealthy", "건강하지"][..])
        } else if normalized.contains("stale")
            || normalized.contains("outdated")
            || normalized.contains("오래됐")
            || normalized.contains("오래되")
        {
            (
                "STALE",
                negated_stale,
                &["stale", "outdated", "오래됐", "오래되"][..],
            )
        } else if normalized.contains("damaged")
            || normalized.contains("broken")
            || normalized.contains("corrupt")
            || normalized.contains("손상")
            || normalized.contains("깨졌")
            || normalized.contains("깨지")
        {
            (
                "DAMAGED",
                negated_damaged,
                &["damaged", "broken", "corrupt", "손상", "깨졌", "깨지"][..],
            )
        } else if normalized.contains("empty")
            || normalized.contains("비었")
            || normalized.contains("비어")
        {
            ("EMPTY", negated_empty, &["empty", "비었", "비어"][..])
        } else if normalized.contains("error") || normalized.contains("오류") {
            ("ERROR_PRESENT", negated_error, &["error", "오류"][..])
        } else if normalized.contains("problem") || normalized.contains("문제") {
            ("PROBLEM_PRESENT", negated_problem, &["problem", "문제"][..])
        } else if normalized.contains("invalid") || normalized.contains("무효") {
            ("INVALID", false, &["invalid", "무효"][..])
        } else if normalized.contains("valid") || normalized.contains("유효") {
            ("VALID", negated_valid, &["valid", "유효"][..])
        } else if normalized.contains("healthy") || normalized.contains("건강") {
            ("HEALTHY", false, &["healthy", "건강"][..])
        } else {
            return None;
        };
    let atom = GuardConditionExpressionIR::atom(predicate);
    let expression = if negated {
        negate_guard_condition_expression(atom)
    } else {
        atom
    };
    let explicit_subjects = explicit_guard_atom_subject(&normalized, subject_marker)
        .into_iter()
        .collect();
    Some(ParsedGuardConditionExpression {
        expression,
        explicit_subjects,
    })
}

fn combine_guard_condition(
    operator: GuardConditionOperatorIR,
    parsed_children: Vec<ParsedGuardConditionExpression>,
) -> Option<ParsedGuardConditionExpression> {
    let mut children = Vec::new();
    let mut explicit_subjects = Vec::new();
    for parsed in parsed_children {
        explicit_subjects.extend(parsed.explicit_subjects);
        if parsed.expression.operator == operator {
            children.extend(parsed.expression.children);
        } else {
            children.push(parsed.expression);
        }
    }
    (children.len() >= 2).then(|| ParsedGuardConditionExpression {
        expression: GuardConditionExpressionIR::composite(operator, children),
        explicit_subjects,
    })
}

fn negate_guard_condition_expression(
    expression: GuardConditionExpressionIR,
) -> GuardConditionExpressionIR {
    match expression.operator {
        GuardConditionOperatorIR::Atom => {
            GuardConditionExpressionIR::composite(GuardConditionOperatorIR::Not, vec![expression])
        }
        GuardConditionOperatorIR::Not => expression
            .children
            .into_iter()
            .next()
            .expect("parsed NOT has one child"),
        GuardConditionOperatorIR::All | GuardConditionOperatorIR::Any => {
            let operator = if expression.operator == GuardConditionOperatorIR::All {
                GuardConditionOperatorIR::Any
            } else {
                GuardConditionOperatorIR::All
            };
            GuardConditionExpressionIR::composite(
                operator,
                expression
                    .children
                    .into_iter()
                    .map(negate_guard_condition_expression)
                    .collect(),
            )
        }
    }
}

fn trim_guard_conditional_suffix(surface: &str) -> String {
    let mut trimmed = surface
        .trim()
        .trim_end_matches(['.', '!', '?', ','])
        .trim()
        .to_string();
    let trailing_parentheses = trimmed.chars().rev().take_while(|ch| *ch == ')').count();
    let insertion = trimmed.len().saturating_sub(trailing_parentheses);
    let prefix = trimmed[..insertion].trim_end();
    let stripped = prefix
        .strip_suffix("으면")
        .or_else(|| prefix.strip_suffix('면'))
        .unwrap_or(prefix)
        .to_string();
    if stripped.len() != prefix.len() {
        trimmed.replace_range(..insertion, &stripped);
    }
    trimmed
}

fn strip_balanced_outer_parentheses(mut surface: &str) -> &str {
    loop {
        let trimmed = surface.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        let mut depth = 0_i32;
        let mut closes_at_end = false;
        for (index, character) in trimmed.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        closes_at_end = index + character.len_utf8() == trimmed.len();
                        break;
                    }
                }
                _ => {}
            }
        }
        if !closes_at_end {
            return trimmed;
        }
        surface = &trimmed[1..trimmed.len() - 1];
    }
}

fn split_guard_condition_top_level(surface: &str, markers: &[&str]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut part_start = 0;
    let mut index = 0;
    while index < surface.len() {
        let character = surface[index..]
            .chars()
            .next()
            .expect("index is a character boundary");
        match character {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ => {}
        }
        if depth == 0 {
            if let Some(marker) = markers
                .iter()
                .find(|marker| surface[index..].starts_with(**marker))
            {
                let part = surface[part_start..index].trim();
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                index += marker.len();
                part_start = index;
                continue;
            }
        }
        index += character.len_utf8();
    }
    let tail = surface[part_start..].trim();
    if !tail.is_empty() {
        parts.push(tail.to_string());
    }
    parts
}

fn explicit_guard_atom_subject(surface: &str, predicate_markers: &[&str]) -> Option<String> {
    let is_english = surface.is_ascii();
    let subject = if is_english {
        [" is ", " has ", " contains "]
            .iter()
            .filter_map(|marker| surface.find(marker).map(|position| (position, *marker)))
            .min_by_key(|(position, _)| *position)
            .map(|(position, _)| surface[..position].trim().to_string())
    } else {
        predicate_markers
            .iter()
            .filter_map(|marker| surface.find(marker))
            .min()
            .map(|position| surface[..position].trim().to_string())
    }?;
    let mut subject = subject
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim()
        .to_string();
    if let Some(rest) = subject.strip_prefix("the ") {
        subject = rest.trim().to_string();
    }
    for suffix in [
        "에는", "에서", "에게", "으로", "로", "에", "은", "는", "이", "가", "을", "를",
    ] {
        if let Some(stem) = subject.strip_suffix(suffix) {
            subject = stem.trim().to_string();
            break;
        }
    }
    (!subject.is_empty()
        && !matches!(
            subject.as_str(),
            "it" | "that" | "this" | "그것" | "그게" | "그거" | "오류" | "문제"
        ))
    .then_some(subject)
}

fn conditional_uses_bound_subject(conditional: &crate::modality::ConditionalRelationIR) -> bool {
    let antecedent = conditional.antecedent.trim().to_lowercase();
    antecedent == "it"
        || antecedent.starts_with("it ")
        || antecedent.starts_with("그것")
        || antecedent.starts_with("그게")
}

fn discourse_program_subject_key(subject: &str) -> String {
    let mut normalized = subject
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim()
        .to_lowercase();
    for article in ["the ", "a ", "an "] {
        if let Some(rest) = normalized.strip_prefix(article) {
            normalized = rest.trim().to_string();
            break;
        }
    }
    normalized = normalized
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim()
        .to_string();
    for particle in ['을', '를', '은', '는', '이', '가'] {
        if normalized.ends_with(particle) {
            normalized.pop();
            break;
        }
    }
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn deferred_action_commitments(
    interpretation: &PragmaticInterpretationIR,
    turn_index: u64,
    source_semantic_text: &str,
) -> Vec<DeferredActionCommitmentIR> {
    let analysis = &interpretation.compositional_analysis;
    let mut commitments = analysis
        .modal_scope_graph
        .conditionals
        .iter()
        .filter(|conditional| conditional.consequent_is_directive)
        .enumerate()
        .filter_map(|(index, conditional)| {
            let consequent = conditional.consequent.to_lowercase();
            let (candidate, frame) = analysis
                .candidates
                .iter()
                .filter_map(|candidate| {
                    analysis
                        .frames
                        .iter()
                        .find(|frame| frame.frame_id == candidate.source_frame_id)
                        .map(|frame| (candidate, frame))
                })
                .filter(|(_, frame)| {
                    consequent.contains(&frame.predicate_surface.to_lowercase())
                        || consequent.contains(&frame.canonical_predicate.to_lowercase())
                })
                .max_by_key(|(candidate, _)| candidate.score_millis)
                .or_else(|| {
                    analysis
                        .candidates
                        .iter()
                        .filter_map(|candidate| {
                            analysis
                                .frames
                                .iter()
                                .find(|frame| frame.frame_id == candidate.source_frame_id)
                                .map(|frame| (candidate, frame))
                        })
                        .max_by_key(|(candidate, _)| candidate.score_millis)
                })?;
            let condition_surface = deferred_condition_surface(conditional);
            let action_subject = if let Some(argument) = analysis
                .semantic_role_graph
                .primary_argument_for_frame(&frame.frame_id)
            {
                argument.normalized_label.clone()
            } else if conditional_uses_bound_subject(conditional) {
                analysis
                    .selected_candidates()
                    .into_iter()
                    .find(|selected| selected.source_frame_id != candidate.source_frame_id)
                    .map(|selected| selected.subject.clone())
                    .unwrap_or_else(|| candidate.subject.clone())
            } else {
                candidate.subject.clone()
            };
            Some(DeferredActionCommitmentIR {
                schema: DEFERRED_ACTION_COMMITMENT_SCHEMA.to_string(),
                commitment_id: format!("DEFERRED-{turn_index:06}-{:02}", index + 1),
                normalized_condition: normalize_condition(&condition_surface),
                condition_sha256: condition_sha256(&condition_surface),
                condition_surface,
                action: DeferredActionIR {
                    intent: candidate.intent,
                    canonical_predicate: frame.canonical_predicate.clone(),
                    predicate_surface: frame.predicate_surface.clone(),
                    subject: action_subject,
                    source_semantic_text: source_semantic_text.trim().to_string(),
                    external_execution_authorized_after_verification: true,
                },
                status: DeferredCommitmentStatusIR::ConditionPending,
                introduced_turn: turn_index,
                last_transition_turn: turn_index,
                evidence_ids: Vec::new(),
                activated_goal_id: None,
            })
        })
        .collect::<Vec<_>>();
    if commitments.is_empty()
        && interpretation
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .is_some_and(|graph| graph.has_selected_conditional_request())
    {
        let selected = interpretation
            .pragmatic_intent_graph
            .composition
            .as_ref()
            .and_then(|graph| {
                graph
                    .selected_node_ids
                    .first()
                    .map(|selected| (graph, selected))
            })
            .and_then(|(graph, selected)| {
                graph.nodes.iter().find(|node| &node.node_id == selected)
            });
        if let Some(selected) = selected {
            let condition_surface = source_semantic_text
                .split_once(',')
                .map(|(condition, _)| condition)
                .unwrap_or(source_semantic_text)
                .trim()
                .to_string();
            commitments.push(DeferredActionCommitmentIR {
                schema: DEFERRED_ACTION_COMMITMENT_SCHEMA.to_string(),
                commitment_id: format!("DEFERRED-{turn_index:06}-01"),
                normalized_condition: normalize_condition(&condition_surface),
                condition_sha256: condition_sha256(&condition_surface),
                condition_surface,
                action: DeferredActionIR {
                    intent: selected.intent,
                    canonical_predicate: selected.canonical_predicate.clone(),
                    predicate_surface: analysis
                        .frames
                        .iter()
                        .find(|frame| frame.frame_id == selected.source_frame_id)
                        .map(|frame| frame.predicate_surface.clone())
                        .unwrap_or_else(|| selected.canonical_predicate.to_lowercase()),
                    subject: selected.subject.clone(),
                    source_semantic_text: source_semantic_text.trim().to_string(),
                    external_execution_authorized_after_verification: true,
                },
                status: DeferredCommitmentStatusIR::ConditionPending,
                introduced_turn: turn_index,
                last_transition_turn: turn_index,
                evidence_ids: Vec::new(),
                activated_goal_id: None,
            });
        }
    }
    commitments
}

fn deferred_condition_surface(conditional: &crate::modality::ConditionalRelationIR) -> String {
    let antecedent = conditional.antecedent.trim();
    if conditional.antecedent_negated
        && !antecedent.to_lowercase().contains("not ")
        && !antecedent.contains("않")
        && !antecedent.contains("아니")
    {
        format!("NOT ({antecedent})")
    } else {
        antecedent.to_string()
    }
}

fn conversation_proposition_referents(
    interpretation: &PragmaticInterpretationIR,
    turn_index: u64,
) -> Vec<DynamicDiscourseReferentIR> {
    if interpretation
        .clauses
        .iter()
        .any(|clause| crate::epistemic::is_retraction_surface(&clause.surface_text))
    {
        return Vec::new();
    }
    let problem_disclosure = interpretation
        .pragmatic_intent_graph
        .selected_utterance_intent()
        .is_some_and(|candidate| {
            candidate.communicative_intent
                == crate::utterance_intent::CommunicativeIntentIR::ProblemDisclosure
        });
    if (interpretation.inferred_goal.is_some() && !problem_disclosure)
        || !(matches!(
            interpretation.speech_act,
            SpeechActIR::Inform
                | SpeechActIR::NegativeEvaluation
                | SpeechActIR::ConditionalCommitment
        ) || (problem_disclosure && interpretation.speech_act == SpeechActIR::Ask))
    {
        return Vec::new();
    }
    let attribution_graph = &interpretation.compositional_analysis.attribution_graph;
    if !attribution_graph.attributions.is_empty() {
        return attribution_graph
            .attributions
            .iter()
            .take(4)
            .enumerate()
            .filter_map(|(index, edge)| {
                let actor = attribution_graph.actor(&edge.actor_id)?;
                let proposition = attribution_graph.proposition(&edge.proposition_id)?;
                if !has_semantic_proposition_content(&proposition.surface_text) {
                    return None;
                }
                Some(DynamicDiscourseReferentIR {
                    referent_id: format!("DREF-P-{turn_index:06}-{:02}", index + 1),
                    kind: DiscourseReferentKindIR::Proposition,
                    topic_id: None,
                    semantic_summary: proposition.surface_text.clone(),
                    attributed_source: Some(actor.surface.clone()),
                    attribution_attitude: Some(edge.attitude),
                    epistemic_status: Some(edge.epistemic_status),
                    proposition_polarity: Some(proposition.polarity),
                    modal_world: Some(attributed_modal_world(
                        edge.attitude,
                        &proposition.surface_text,
                    )),
                    belief_record_id: None,
                    introduced_turn: turn_index,
                    last_referenced_turn: turn_index,
                    external_execution_authorized: false,
                })
            })
            .collect();
    }
    interpretation
        .clauses
        .iter()
        .filter(|clause| has_semantic_proposition_content(&clause.surface_text))
        .take(4)
        .enumerate()
        .filter_map(|(index, clause)| {
            let polarity = match clause.polarity {
                crate::pragmatics::PropositionPolarityIR::Positive => {
                    crate::attribution::AttributedPropositionPolarityIR::Positive
                }
                crate::pragmatics::PropositionPolarityIR::Negative => {
                    crate::attribution::AttributedPropositionPolarityIR::Negative
                }
                crate::pragmatics::PropositionPolarityIR::Mixed => return None,
            };
            Some(DynamicDiscourseReferentIR {
                referent_id: format!("DREF-P-{turn_index:06}-{:02}", index + 1),
                kind: DiscourseReferentKindIR::Proposition,
                topic_id: None,
                semantic_summary: clause.surface_text.clone(),
                attributed_source: None,
                attribution_attitude: None,
                epistemic_status: None,
                proposition_polarity: Some(polarity),
                modal_world: Some(
                    crate::modality::ModalSemanticAnalyzer
                        .analyze(&clause.surface_text)
                        .root_world,
                ),
                belief_record_id: None,
                introduced_turn: turn_index,
                last_referenced_turn: turn_index,
                external_execution_authorized: false,
            })
        })
        .collect()
}

fn has_semantic_proposition_content(surface: &str) -> bool {
    surface.chars().any(char::is_alphanumeric)
}

fn attributed_modal_world(
    attitude: crate::attribution::AttributionAttitudeIR,
    proposition_surface: &str,
) -> crate::modality::ModalWorldIR {
    let parsed = crate::modality::ModalSemanticAnalyzer
        .analyze(proposition_surface)
        .root_world;
    if parsed != crate::modality::ModalWorldIR::Actual {
        return parsed;
    }
    match attitude {
        crate::attribution::AttributionAttitudeIR::Want => crate::modality::ModalWorldIR::Desired,
        crate::attribution::AttributionAttitudeIR::Expect => {
            crate::modality::ModalWorldIR::Predicted
        }
        _ => parsed,
    }
}

fn map_pragmatic_memory_error(_: PragmaticMemoryError) -> CognitiveApiError {
    CognitiveApiError::PragmaticMemory
}

fn map_predicate_lexeme_error(_: PredicateLexemeError) -> CognitiveApiError {
    CognitiveApiError::CompositionalPredicate
}

fn map_lexical_error(_: LexicalMemoryError) -> CognitiveApiError {
    CognitiveApiError::LexicalMemory
}

fn map_knowledge_work_error(_: KnowledgeWorkError) -> CognitiveApiError {
    CognitiveApiError::KnowledgeWork
}

fn map_long_term_repair_error(_: LongTermRepairPlanError) -> CognitiveApiError {
    CognitiveApiError::LongTermRepairPlan
}

fn map_professional_document_error(_: ProfessionalDocumentError) -> CognitiveApiError {
    CognitiveApiError::ProfessionalDocument
}

fn map_planning_error(_: PlanningError) -> CognitiveApiError {
    CognitiveApiError::Planning
}

fn map_deliberation_error(_: DeliberationError) -> CognitiveApiError {
    CognitiveApiError::Deliberation
}

fn map_mechanism_memory_error(_: MechanismMemoryError) -> CognitiveApiError {
    CognitiveApiError::MechanismMemory
}

fn map_mechanism_induction_error(_: MechanismInductionError) -> CognitiveApiError {
    CognitiveApiError::MechanismInduction
}

fn map_raw_mechanism_induction_error(_: RawMechanismInductionError) -> CognitiveApiError {
    CognitiveApiError::RawMechanismInduction
}

fn merge_lexical_activations(
    understanding: &mut LanguageUnderstandingIR,
    activations: &[ActivatedSenseIR],
) {
    let had_legacy_match = !understanding.matched_knowledge_ids.is_empty();
    let mut observed_lexemes = std::collections::BTreeSet::new();
    let mut strongest_intent = None::<(dockable_semantic_core::PlanIntentIR, u32)>;
    for activation in activations {
        if !observed_lexemes.insert(activation.lexeme_id.as_str()) {
            continue;
        }
        understanding
            .matched_knowledge_ids
            .push(format!("{}/{}", activation.lexeme_id, activation.sense_id));
        understanding
            .semantic_tags
            .push(activation.canonical_concept.clone());
        understanding
            .semantic_tags
            .extend(activation.semantic_tags.iter().cloned());
        if let Some(intent) = activation.intent_hint {
            if strongest_intent
                .as_ref()
                .is_none_or(|(_, score)| activation.activation_millis > *score)
            {
                strongest_intent = Some((intent, activation.activation_millis));
            }
        }
    }
    if !had_legacy_match {
        if let Some((intent, _)) = strongest_intent {
            understanding.intent = intent;
        }
    }
    understanding.matched_knowledge_ids.sort();
    understanding.matched_knowledge_ids.dedup();
    understanding.semantic_tags.sort();
    understanding.semantic_tags.dedup();
}

fn intent_for_knowledge_operation(
    operation: KnowledgeWorkOperationIR,
) -> dockable_semantic_core::PlanIntentIR {
    match operation {
        KnowledgeWorkOperationIR::Interpret | KnowledgeWorkOperationIR::Analyze => {
            dockable_semantic_core::PlanIntentIR::Investigate
        }
        KnowledgeWorkOperationIR::Write | KnowledgeWorkOperationIR::Revise => {
            dockable_semantic_core::PlanIntentIR::Create
        }
        KnowledgeWorkOperationIR::Plan => dockable_semantic_core::PlanIntentIR::Plan,
    }
}

fn lexical_knowledge_operation(
    fallback: KnowledgeWorkOperationIR,
    activations: &[ActivatedSenseIR],
) -> KnowledgeWorkOperationIR {
    activations
        .iter()
        .filter_map(|activation| {
            let operation = match activation.canonical_concept.as_str() {
                "revise" => KnowledgeWorkOperationIR::Revise,
                "author" => KnowledgeWorkOperationIR::Write,
                "plan" => KnowledgeWorkOperationIR::Plan,
                "analyze" => KnowledgeWorkOperationIR::Analyze,
                _ => return None,
            };
            Some((operation, activation.activation_millis))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(operation, _)| operation)
        .unwrap_or(fallback)
}

fn lexical_document_kind(activations: &[ActivatedSenseIR]) -> Option<DocumentKindIR> {
    activations
        .iter()
        .filter_map(|activation| {
            let kind = match activation.canonical_concept.as_str() {
                "academic_paper" => DocumentKindIR::Paper,
                "business_plan" => DocumentKindIR::BusinessPlan,
                "business_proposal" => DocumentKindIR::BusinessProposal,
                "user_guide" => DocumentKindIR::UserGuide,
                "data_table" => DocumentKindIR::Table,
                "data_chart" => DocumentKindIR::Chart,
                "financial_statement" => DocumentKindIR::FinancialStatement,
                _ => return None,
            };
            Some((kind, activation.activation_millis))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(kind, _)| kind)
}

fn conversational_language(text: &str) -> LanguageCodeIR {
    if text
        .chars()
        .any(|character| matches!(character, '\u{ac00}'..='\u{d7a3}' | '\u{3131}'..='\u{318e}'))
    {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    }
}

fn is_pending_gate_decision_question(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    let continuation = [
        "계속",
        "진행",
        "이어가",
        "continue",
        "keep going",
        "proceed",
        "carry on",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    let decision = normalized.ends_with('?')
        || [
            "해도 돼",
            "해도 될까",
            "해야 해",
            "should we",
            "can we",
            "may we",
            "is it safe to",
        ]
        .iter()
        .any(|marker| normalized.contains(marker));
    continuation && decision
}

fn is_proxy_only_evidence_update(text: &str) -> bool {
    let normalized = text.to_lowercase();
    let proxy = [
        "점수",
        "벤치마크",
        "랭킹",
        "라우팅",
        "지표",
        "score",
        "benchmark",
        "ranking",
        "routing",
        "proxy",
        "metric",
    ]
    .iter()
    .any(|marker| normalized.contains(marker));
    let explicit_action = crate::conversation::contains_explicit_action_surface(&normalized);
    proxy && !explicit_action && !is_pending_gate_decision_question(&normalized)
}

// Quarantined legacy renderers. They are intentionally unreachable from the
// active pipeline while old fixtures are migrated; NaturalRealizationIR is the
// sole production surface-text owner.
#[allow(dead_code)]
fn render_topic_transition(language: LanguageCodeIR, transition: &TopicTransitionIR) -> String {
    if !transition.applied || transition.kind == TopicTransitionKindIR::Unresolved {
        return match language {
            LanguageCodeIR::Korean => {
                "어느 주제를 가리키는지 확정할 수 없어. 대상 주제를 더 구체적으로 말해 줘."
                    .to_string()
            }
            _ => "I cannot determine which topic you mean. Please identify it more precisely."
                .to_string(),
        };
    }
    match language {
        LanguageCodeIR::Korean => format!(
            "좋아. 이제 ‘{}’ 이야기를 현재 화제로 둘게. 이건 대화 초점 변경일 뿐, 작업을 실행했다는 뜻은 아니야.",
            transition.surface
        ),
        _ => format!(
            "Okay. ‘{}’ is now the active topic. This changes the conversation focus; it does not execute any work.",
            transition.surface
        ),
    }
}

#[allow(dead_code)]
fn render_pending_gate_decision(
    language: LanguageCodeIR,
    gate: &PendingContinuationGateIR,
) -> String {
    match language {
        LanguageCodeIR::Korean => format!(
            "아직 필요한 실제 이득 ‘{}’의 달성 여부를 직접 확인하지 못했어. 점수나 대리 지표만으로 ‘{}’ 작업을 계속해도 된다고 판단하지 않을게. 실제 이득을 확인하거나, 확인할 수 없다면 중단 여부를 다시 물어야 해.",
            gate.required_benefit, gate.task
        ),
        _ => format!(
            "The required benefit ‘{}’ is not directly verified yet. I will not authorize continuing ‘{}’ from a score or proxy alone. Verify the real outcome first, or ask whether to stop if it remains unresolved.",
            gate.required_benefit, gate.task
        ),
    }
}

#[allow(dead_code)]
fn render_proxy_evidence_update(
    language: LanguageCodeIR,
    gate: &PendingContinuationGateIR,
) -> String {
    match language {
        LanguageCodeIR::Korean => format!(
            "대리 지표 변화는 기록했지만, 필요한 실제 이득 ‘{}’의 확인으로 간주하지 않을게.",
            gate.required_benefit
        ),
        _ => format!(
            "I recorded the proxy change, but it does not verify the required real benefit ‘{}’.",
            gate.required_benefit
        ),
    }
}

#[allow(dead_code)]
fn render_illocutionary_commitment_response(
    language: LanguageCodeIR,
    interpretation: &PragmaticInterpretationIR,
) -> Option<String> {
    if interpretation.continuation_gate.is_some() {
        return None;
    }
    let graph = &interpretation.illocutionary_commitments;
    let force = graph.primary_force()?;
    let text = match (language, force) {
        (LanguageCodeIR::Korean, IllocutionaryForceIR::SelfCommitment) => {
            "네가 직접 하겠다는 약속으로 이해했어. 이 말을 내가 실행하라는 요청으로 바꾸지는 않았어."
                .to_string()
        }
        (_, IllocutionaryForceIR::SelfCommitment) => {
            "I understood this as your own commitment, not as a request that authorizes me to execute it."
                .to_string()
        }
        (LanguageCodeIR::Korean, IllocutionaryForceIR::ReportedCommitment) => {
            "제3자가 앞으로 하겠다고 말했다는 보고로 이해했어. 그 발언은 내 실행 권한이 아니고, 실제 완료 사실도 아직 아니야."
                .to_string()
        }
        (_, IllocutionaryForceIR::ReportedCommitment) => {
            "I understood this as a report of a third party's commitment. It neither authorizes my execution nor establishes completion."
                .to_string()
        }
        (LanguageCodeIR::Korean, IllocutionaryForceIR::CapabilityQuestion) => {
            "시스템이 그 기능을 지원하는지 묻는 질문으로 이해했어. 실행 요청으로 처리하지 않았고, 지원 여부는 확인 가능한 기능 근거로 판단해야 해."
                .to_string()
        }
        (_, IllocutionaryForceIR::CapabilityQuestion) => {
            "I understood this as a capability question, not an execution request. Capability should be answered from inspectable support evidence."
                .to_string()
        }
        (LanguageCodeIR::Korean, IllocutionaryForceIR::DeferredConditionalRequest) => {
            "조건이 충족된 뒤에만 실행할 수 있는 요청으로 기록했어. 현재는 조건 대기 상태라 실행 목표를 활성화하지 않았어."
                .to_string()
        }
        (_, IllocutionaryForceIR::DeferredConditionalRequest) => {
            "I recorded this as a condition-pending request. The action is not active until its antecedent is verified."
                .to_string()
        }
        (_, IllocutionaryForceIR::AnswerOnlyInformationRequest) => return None,
        (LanguageCodeIR::Korean, IllocutionaryForceIR::GoalWithdrawal) => {
            let scope = graph
                .goal_withdrawal
                .as_ref()
                .and_then(|withdrawal| withdrawal.event_ordinal)
                .map_or_else(|| "지정한 활성 작업".to_string(), |ordinal| format!("{ordinal}번째 활성 작업"));
            format!("{scope}을 철회한 것으로 반영했어. 철회된 작업은 더 이상 활성 목표가 아니야.")
        }
        (_, IllocutionaryForceIR::GoalWithdrawal) => {
            let scope = graph
                .goal_withdrawal
                .as_ref()
                .and_then(|withdrawal| withdrawal.event_ordinal)
                .map_or_else(|| "the specified active work".to_string(), |ordinal| format!("active action {ordinal}"));
            format!("I applied the withdrawal to {scope}. The retired work is no longer an active goal.")
        }
        (LanguageCodeIR::Korean, IllocutionaryForceIR::OutcomeClaimConstraint) => {
            "완료·성공·실행 여부는 확인 기록이나 직접 검증이 있을 때만 그렇게 말하라는 정책으로 반영했어. 근거가 없으면 결과를 완료로 표현하지 않을게."
                .to_string()
        }
        (_, IllocutionaryForceIR::OutcomeClaimConstraint) => {
            "I recorded a verified-outcome-only policy: completion, success, or execution may be claimed only from verification evidence or a recorded receipt."
                .to_string()
        }
        (_, IllocutionaryForceIR::IndirectActionRequest) => return None,
    };
    Some(text)
}

#[allow(dead_code)]
fn render_conversation_grounding(
    language: LanguageCodeIR,
    understanding: &LanguageUnderstandingIR,
    plan: &PlanIR,
    pragmatic_interpretation: &PragmaticInterpretationIR,
    reference_resolution: &ReferenceResolutionIR,
) -> String {
    debug_assert!(plan.structurally_validated);
    if let Some(gate) = &pragmatic_interpretation.continuation_gate {
        return match language {
            LanguageCodeIR::Korean => format!(
                "‘{}’ 작업을 계속할지는 ‘{}’라는 실제 이득이 있는지 먼저 검증해야 한다는 뜻으로 이해했어. 이득이 확인되면 계속하고, 아니면 그 결과를 보고한 뒤 멈출지 물을게. 아직 판단할 증거가 부족해도 추측하지 않고 확인을 요청할게.",
                gate.current_task, gate.required_benefit
            ),
            _ => format!(
                "I understood this as a continuation gate for ‘{}’: first verify the real benefit ‘{}’. Continue if it is supported; otherwise report the result and ask whether to stop. If the evidence remains unresolved, ask instead of guessing.",
                gate.current_task, gate.required_benefit
            ),
        };
    }
    if pragmatic_interpretation.nonliteral_analysis.has_sarcasm() {
        return match language {
            LanguageCodeIR::Korean => "표면적으로는 칭찬처럼 보이지만 앞의 실패·오류 상태와 의미가 충돌하므로, 실제 의도는 긍정 승인이 아니라 부정적 평가나 불만에 가깝다고 이해했어. 이 표현만으로 새 작업 권한을 만들지는 않을게.".to_string(),
            _ => "The surface wording looks like praise, but it conflicts with the stated failure state. I therefore read it as a negative evaluation or complaint, not approval, and I will not derive new action authority from it alone.".to_string(),
        };
    }
    if let Some(expression) = pragmatic_interpretation
        .nonliteral_analysis
        .expressions
        .iter()
        .find(|expression| {
            expression.selected_reading == crate::nonliteral::ReadingSelectionIR::Figurative
        })
    {
        return match language {
            LanguageCodeIR::Korean => format!(
                "‘{}’를 문자 그대로의 행동이 아니라 ‘{}’에 해당하는 비유적 상태로 이해했어. 물리적 의미로 실행하지 않고 실제 막힘이나 문제를 확인할게.",
                expression.surface_text, expression.figurative_concept
            ),
            _ => format!(
                "I understood ‘{}’ figuratively as ‘{}’, not as a literal action. I will inspect the actual blocked or problematic state instead of executing the physical reading.",
                expression.surface_text, expression.figurative_concept
            ),
        };
    }
    if pragmatic_interpretation.user_feedback.is_some() {
        let feedback =
            render_user_feedback_for_explicit_request(language, pragmatic_interpretation);
        let parsed_subject = conversation_display_subject(understanding, reference_resolution);
        let subject = feedback_plan_subject(language, &parsed_subject, pragmatic_interpretation);
        return format!(
            "{feedback} {}",
            render_conversation_plan_preview(language, subject, plan)
        );
    }
    if let Some(affect) = detect_user_affect(&understanding.original_text) {
        let empathy = realize_user_affect(language, affect);
        let explicit_request = pragmatic_interpretation
            .compositional_analysis
            .selected_candidates()
            .iter()
            .any(|candidate| candidate.external_execution_authorized);
        if explicit_request {
            let subject = conversation_display_subject(understanding, reference_resolution);
            return format!(
                "{empathy} {}",
                render_conversation_plan_preview(language, &subject, plan)
            );
        }
        return render_standalone_user_affect(language, affect);
    }
    if let Some(goal) = pragmatic_interpretation
        .inferred_goal
        .as_ref()
        .filter(|goal| !goal.external_execution_authorized)
    {
        return match (language, goal.commitment, goal.intent) {
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::Suggestion,
                _,
            ) => format!(
                "‘{}’라는 개선 제안으로 이해했어. 구현 명령으로 단정하지 않고 기대 효과와 요구사항부터 확인할게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Investigate,
            ) => format!(
                "‘{}’의 원인이나 이유를 알고 싶다는 뜻으로 이해했어. 관찰 가능한 증거부터 확인할게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Repair,
            ) => format!(
                "‘{}’ 상태를 그대로 둘 수 없으니 수리가 필요하다는 뜻으로 이해했어. 원인과 수정 범위를 확인하되, 이 암묵적 표현만으로 외부 변경 권한을 넓히지는 않을게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Explain,
            ) => format!(
                "‘{}’에 대해 근거가 있는 설명이나 요약을 원한다는 뜻으로 이해했어. 확인된 내용과 아직 모르는 부분을 나눠서 답할게.",
                goal.subject
            ),
            (
                LanguageCodeIR::Korean,
                crate::pragmatics::GoalCommitmentIR::ImplicitRequest,
                dockable_semantic_core::PlanIntentIR::Plan,
            ) => format!(
                "‘{}’에 맞는 선택지를 비교해 추천해달라는 뜻으로 이해했어. 제약과 근거를 먼저 확인하고 실행 권한은 별도로 둘게.",
                goal.subject
            ),
            (_, crate::pragmatics::GoalCommitmentIR::Suggestion, _) => format!(
                "I understood ‘{}’ as an improvement suggestion, not automatic authorization to implement it. I'll first clarify its expected benefit and requirements.",
                goal.subject
            ),
            (_, _, dockable_semantic_core::PlanIntentIR::Investigate) => format!(
                "I understood that you want to know the cause or explanation for ‘{}’. I'll start from observable evidence.",
                goal.subject
            ),
            (_, _, _) => format!(
                "I inferred an indirect request concerning ‘{}’. I'll inspect the cause and scope without treating the wording as broader external-mutation authority.",
                goal.subject
            ),
        };
    }
    if let Some(realization) = render_structured_goal_preview(language, pragmatic_interpretation) {
        return realization;
    }
    let subject = conversation_display_subject(understanding, reference_resolution);
    render_conversation_plan_preview(language, &subject, plan)
}

#[allow(dead_code)]
fn render_structured_goal_preview(
    language: LanguageCodeIR,
    interpretation: &PragmaticInterpretationIR,
) -> Option<String> {
    let analysis = &interpretation.compositional_analysis;
    let selected = analysis.selected_candidates();
    let blocked = analysis
        .candidates
        .iter()
        .filter(|candidate| candidate.disposition == CandidateDispositionIR::BlockedByNegation)
        .collect::<Vec<_>>();
    let has_relative_attachment = selected.iter().any(|candidate| {
        analysis
            .semantic_role_graph
            .relative_attachment_for_frame(&candidate.source_frame_id)
            .is_some()
    });
    if (selected.len() + blocked.len() < 2 && !has_relative_attachment) || selected.is_empty() {
        return None;
    }

    let clauses = selected
        .iter()
        .map(|candidate| structured_candidate_clause(language, candidate, analysis))
        .collect::<Vec<_>>();
    let mut joined = clauses.first().cloned().unwrap_or_default();
    for (index, clause) in clauses.iter().enumerate().skip(1) {
        let relation = analysis
            .goal_graph
            .as_ref()
            .and_then(|graph| graph.edges.get(index - 1))
            .map(|edge| edge.relation)
            .unwrap_or(GoalGraphRelationKindIR::Sequence);
        let connector = match (language, relation) {
            (LanguageCodeIR::Korean, GoalGraphRelationKindIR::Coordination) => " 그리고 ",
            (LanguageCodeIR::Korean, GoalGraphRelationKindIR::Sequence) => " 그다음 ",
            (_, GoalGraphRelationKindIR::Coordination) => ", and ",
            (_, GoalGraphRelationKindIR::Sequence) => ", then ",
        };
        joined.push_str(connector);
        joined.push_str(clause);
    }
    let blocked_text = if blocked.is_empty() {
        String::new()
    } else {
        let prohibited = blocked
            .iter()
            .map(|candidate| structured_candidate_clause(language, candidate, analysis))
            .collect::<Vec<_>>()
            .join(if language == LanguageCodeIR::Korean {
                ", "
            } else {
                " and "
            });
        match language {
            LanguageCodeIR::Korean => {
                format!(" 다음 금지 요청은 계획에서 제외했어: {prohibited}.")
            }
            _ => format!(" I excluded the prohibited request to {prohibited} from the plan."),
        }
    };
    Some(match language {
        LanguageCodeIR::Korean => format!(
            "요청을 다음 작업 계획으로 이해했어: {joined}.{blocked_text} 아직 실행 결과는 없고, 각 단계는 실행 후 검증해야 해."
        ),
        _ if blocked.is_empty() => format!(
            "I understood the request as this structured plan: {joined}. These are planned operations, not completed results; each outcome still requires verification."
        ),
        _ => format!(
            "I understood the request as this structured plan: {joined}.{blocked_text} These operations are planned before execution and not executed yet; each outcome still requires verification."
        ),
    })
}

#[allow(dead_code)]
fn structured_candidate_clause(
    language: LanguageCodeIR,
    candidate: &InterpretationCandidateIR,
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
) -> String {
    let predicate_surface = analysis
        .frames
        .iter()
        .find(|frame| frame.frame_id == candidate.source_frame_id)
        .map_or(candidate.desired_outcome.as_str(), |frame| {
            frame.predicate_surface.as_str()
        });
    let action = localized_question_action(predicate_surface, candidate.intent, language);
    let subject = structured_candidate_subject(language, candidate, analysis);
    match language {
        LanguageCodeIR::Korean => format!(
            "‘{}’{} {}",
            subject,
            korean_object_particle(&subject),
            action
        ),
        _ => format!("{} ‘{}’", action, subject),
    }
}

fn structured_candidate_subject(
    language: LanguageCodeIR,
    candidate: &InterpretationCandidateIR,
    analysis: &crate::compositional_semantics::CompositionalAnalysisIR,
) -> String {
    let Some(attachment) = analysis
        .semantic_role_graph
        .relative_attachment_for_frame(&candidate.source_frame_id)
    else {
        return candidate.subject.clone();
    };
    let surface = attachment.evidence_surface.trim();
    if language == LanguageCodeIR::Korean {
        for particle in ["을", "를", "은", "는", "이", "가"] {
            if let Some(stem) = surface.strip_suffix(particle) {
                if !stem.trim().is_empty() {
                    return stem.trim().to_string();
                }
            }
        }
    }
    surface.to_string()
}

fn has_explicit_selected_request(interpretation: &PragmaticInterpretationIR) -> bool {
    interpretation
        .compositional_analysis
        .selected_candidates()
        .iter()
        .any(|candidate| {
            candidate.disposition == crate::compositional_semantics::CandidateDispositionIR::Viable
        })
}

#[allow(dead_code)]
fn render_grounded_inform_acknowledgement(
    language: LanguageCodeIR,
    original_text: &str,
    interpretation: &PragmaticInterpretationIR,
) -> String {
    let attributed = !interpretation
        .compositional_analysis
        .attribution_graph
        .attributions
        .is_empty();
    match (language, attributed) {
        (LanguageCodeIR::Korean, true) => format!(
            "알겠어. “{original_text}”라고 보고된 내용으로 기억할게. 다만 이 대화에서 사실로 확인된 건 아니야."
        ),
        (LanguageCodeIR::Korean, false) => format!(
            "알겠어. 네가 “{original_text}”라고 말한 내용으로 기억할게. 별도 증거가 없으니 아직 사실로 확인된 것으로 취급하진 않을게."
        ),
        (_, true) => format!(
            "Got it. I'll remember “{original_text}” as what was reported, not as an established fact."
        ),
        (_, false) => format!(
            "Got it. I'll remember that you said “{original_text}”, but I won't treat it as an established fact without evidence."
        ),
    }
}

#[allow(dead_code)]
fn render_unrecorded_result(
    language: LanguageCodeIR,
    resolution: &ReferenceResolutionIR,
) -> String {
    let resolved = resolution
        .discourse_bindings
        .iter()
        .find(|binding| binding.kind == DiscourseBindingKindIR::ResultReference)
        .map(|binding| binding.resolved_surface.trim())
        .unwrap_or("result");
    let origin = resolved
        .strip_suffix("의 결과")
        .or_else(|| resolved.strip_prefix("the result of "))
        .unwrap_or(resolved);
    let display = quote_subject_once(origin);
    match language {
        LanguageCodeIR::Korean => format!(
            "{display}에 관해 기록된 실행 결과는 아직 없어. 앞선 턴에서 만든 것은 계획이므로 실제로 관찰된 결과처럼 설명하지 않을게."
        ),
        _ => format!(
            "No execution result is recorded yet for {display}. The earlier turn produced a plan, so I won't present it as an observed outcome."
        ),
    }
}

#[allow(dead_code)]
fn render_action_state_response(
    language: LanguageCodeIR,
    analysis: &ActionStateAnalysisIR,
    ledger: Option<&ActionStateLedgerIR>,
) -> String {
    if analysis.untrusted_evidence_claim {
        return match language {
            LanguageCodeIR::Korean => "텍스트에 영수증·터미널·콘솔 결과가 언급됐지만, 그것은 호스트 검증 채널의 영수증이 아니야. 실행 상태를 완료나 성공으로 승격하지 않았어.".to_string(),
            _ => "The text mentions a receipt, terminal, or console result, but language is not a host-verified execution receipt. I did not promote the action to completed or successful.".to_string(),
        };
    }
    if let Some(query) = analysis
        .set_query
        .as_ref()
        .filter(|query| query.predicate.is_some() && query.quantifier.is_some())
    {
        return render_action_set_evaluation(language, query);
    }
    let target_records = analysis
        .target_action_ids
        .iter()
        .filter_map(|id| ledger.and_then(|ledger| ledger.record(id)))
        .collect::<Vec<_>>();
    if target_records.len() > 1 {
        return render_action_group_response(language, analysis, &target_records);
    }
    let record = analysis
        .target_action_ids
        .first()
        .and_then(|id| ledger.and_then(|ledger| ledger.record(id)))
        .or_else(|| ledger.and_then(ActionStateLedgerIR::current_record));
    if let Some(report) = analysis.language_reports().first() {
        let reported = match (language, report.reported_status) {
            (LanguageCodeIR::Korean, ActionReportedStatusIR::Attempted) => "시도함",
            (LanguageCodeIR::Korean, ActionReportedStatusIR::InProgressClaimed) => "진행 중",
            (LanguageCodeIR::Korean, ActionReportedStatusIR::SuccessClaimed) => "성공·완료",
            (LanguageCodeIR::Korean, ActionReportedStatusIR::FailureClaimed) => "실패",
            (_, ActionReportedStatusIR::Attempted) => "attempted",
            (_, ActionReportedStatusIR::InProgressClaimed) => "in progress",
            (_, ActionReportedStatusIR::SuccessClaimed) => "successful or complete",
            (_, ActionReportedStatusIR::FailureClaimed) => "failed",
        };
        let subject = record.map_or_else(
            || "action".to_string(),
            |record| restore_user_grounded_acronyms(&record.subject, &record.source_semantic_text),
        );
        return match language {
            LanguageCodeIR::Korean => format!(
                "‘{subject}’ 작업을 {reported}이라고 사용자가 보고한 상태로 기록했어. 언어 보고는 검증된 실행 결과가 아니므로 실행 상태 자체는 바꾸지 않았어."
            ),
            _ => format!(
                "I recorded {subject} as reported {reported}. A language report is not a verified execution result, so the observed execution state is unchanged."
            ),
        };
    }
    let Some(record) = record else {
        return match language {
            LanguageCodeIR::Korean => "연결할 실행 계획이나 검증 기록이 없어. 대상을 지정해 줘.".to_string(),
            _ => "There is no action plan or verified execution record to bind this query to. Please identify the target action.".to_string(),
        };
    };
    let subject = restore_user_grounded_acronyms(&record.subject, &record.source_semantic_text);
    match (language, record.execution_status, record.reported_status) {
        (
            LanguageCodeIR::Korean,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::Attempted),
        ) => format!(
            "‘{subject}’은 시도했다는 사용자 보고만 있어. 계획은 남아 있지만 검증된 실행 관찰이나 결과는 없어."
        ),
        (
            LanguageCodeIR::Korean,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::InProgressClaimed),
        ) => format!(
            "‘{subject}’은 진행 중이라는 사용자 보고만 있어. 호스트 영수증으로 검증된 실행 상태는 아직 없어."
        ),
        (
            LanguageCodeIR::Korean,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::SuccessClaimed),
        ) => format!(
            "‘{subject}’은 성공·완료됐다는 사용자 보고가 있지만 검증된 실행 결과는 없어. 보고와 검증 상태를 분리해 유지하고 있어."
        ),
        (
            LanguageCodeIR::Korean,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::FailureClaimed),
        ) => format!(
            "‘{subject}’은 실패했다는 사용자 보고가 있지만 검증된 실행 결과는 없어. 보고와 검증 상태를 분리해 유지하고 있어."
        ),
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::NotObserved, None) => format!(
            "‘{subject}’은 활성 계획 상태야. 검증된 실행 관찰은 없고, 실행 결과는 아직 없어."
        ),
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::InProgress, _) => format!(
            "‘{subject}’은 호스트 검증 영수증 기준으로 실행 중이야. 아직 성공·실패 결과는 확정되지 않았어."
        ),
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::Succeeded, _) => format!(
            "‘{subject}’의 검증된 실행 결과는 성공이야. 시작 및 종료 영수증이 모두 기록돼 있어."
        ),
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::Failed, _) => format!(
            "‘{subject}’의 검증된 실행 결과는 실패야. 시작 및 실패 영수증이 모두 기록돼 있어."
        ),
        (_, ActionExecutionStatusIR::NotObserved, Some(ActionReportedStatusIR::Attempted)) => {
            format!("{subject} was reported as attempted. The plan remains, but there is no verified execution observation or result.")
        }
        (
            _,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::InProgressClaimed),
        ) => format!(
            "{subject} was reported as in progress, but there is no host-verified execution state yet."
        ),
        (
            _,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::SuccessClaimed),
        ) => format!(
            "{subject} was reported as successful or complete, but there is no verified execution result. The reported and verified states remain separate."
        ),
        (
            _,
            ActionExecutionStatusIR::NotObserved,
            Some(ActionReportedStatusIR::FailureClaimed),
        ) => format!(
            "{subject} was reported as failed, but there is no verified execution result. The reported and verified states remain separate."
        ),
        (_, ActionExecutionStatusIR::NotObserved, None) => format!(
            "{subject} is an active plan. No execution result is recorded yet, and there is no verified execution observation."
        ),
        (_, ActionExecutionStatusIR::InProgress, _) => format!(
            "A verified host receipt records {subject} as in progress. No success or failure outcome is verified yet."
        ),
        (_, ActionExecutionStatusIR::Succeeded, _) => format!(
            "The verified execution result for {subject} is success. Both start and terminal receipts are recorded."
        ),
        (_, ActionExecutionStatusIR::Failed, _) => format!(
            "The verified execution result for {subject} is failure. Both start and terminal receipts are recorded."
        ),
    }
}

#[allow(dead_code)]
fn render_definition_grounding(
    language: LanguageCodeIR,
    grounding: &DefinitionGroundingIR,
) -> String {
    match (language, grounding.disposition, grounding.binding.as_ref()) {
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::Bound, Some(binding)) => {
            let change = if grounding.lexical_store_changed {
                "어휘 연결을 추가했어"
            } else {
                "같은 어휘 연결을 확인했어"
            };
            format!(
                "‘{}’를 기존 의미 연산자 ‘{}’에 연결했어. {change}. 의미 payload와 실행 권한은 바꾸지 않았어.",
                binding.alias_surface, binding.canonical_predicate
            )
        }
        (_, DefinitionGroundingDispositionIR::Bound, Some(binding)) => {
            let change = if grounding.lexical_store_changed {
                "I added the lexical binding"
            } else {
                "I confirmed the existing lexical binding"
            };
            format!(
                "I linked ‘{}’ to the existing semantic operator ‘{}’. {change}; the semantic payload and execution authority did not change.",
                binding.alias_surface, binding.canonical_predicate
            )
        }
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::ConflictRejected, _) => {
            "그 표현은 이미 다른 의미에 연결돼 있어 재정의를 거부했어. 기존 의미와 실행 권한은 그대로야.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::ConflictRejected, _) => {
            "I rejected the redefinition because the label already has a different binding. Its existing meaning and execution authority are unchanged.".to_string()
        }
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::NonAssertedRejected, _) => {
            "질문·가정·인용·전언 속 정의는 사용자 자신의 확정 정의로 취급하지 않았어.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::NonAssertedRejected, _) => {
            "I did not treat a questioned, hypothetical, quoted, or reported definition as the user's asserted definition.".to_string()
        }
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::AmbiguousRejected, _) => {
            "정의가 여러 의미 연산자를 함께 가리켜 연결하지 않았어. 하나의 뜻으로 명확히 정의해줘.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::AmbiguousRejected, _) => {
            "The definition points to multiple semantic operators, so I did not bind it. Define one meaning explicitly.".to_string()
        }
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::UnresolvedRejected, _) => {
            "정의에서 기존 의미 연산자를 찾지 못해 어휘 연결을 만들지 않았어.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::UnresolvedRejected, _) => {
            "I could not ground the definition to an existing semantic operator, so I created no lexical binding.".to_string()
        }
        (LanguageCodeIR::Korean, DefinitionGroundingDispositionIR::InvalidAliasRejected, _) => {
            "별칭 형식이 유효하지 않아 연결하지 않았어.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::InvalidAliasRejected, _) => {
            "I rejected the binding because the alias form is invalid.".to_string()
        }
        (_, DefinitionGroundingDispositionIR::NoDefinition, _) => String::new(),
        (_, DefinitionGroundingDispositionIR::Bound, None) => String::new(),
    }
}

#[allow(dead_code)]
fn render_discourse_group_update(
    language: LanguageCodeIR,
    update: &DiscourseGroupUpdateIR,
) -> String {
    let count = update.after_member_keys.len();
    match (language, update.operation) {
        (
            LanguageCodeIR::English | LanguageCodeIR::Mixed | LanguageCodeIR::Unknown,
            DiscourseGroupUpdateOperationIR::AddMember,
        ) => format!(
            "I added the referenced member. That discourse group now contains {count} members."
        ),
        (
            LanguageCodeIR::English | LanguageCodeIR::Mixed | LanguageCodeIR::Unknown,
            DiscourseGroupUpdateOperationIR::RemoveMember,
        ) => format!(
            "I removed the referenced member. That discourse group now contains {count} members."
        ),
        (
            LanguageCodeIR::English | LanguageCodeIR::Mixed | LanguageCodeIR::Unknown,
            DiscourseGroupUpdateOperationIR::MergeGroups,
        ) => format!(
            "I combined the two discourse groups into a new group with {count} distinct members."
        ),
        (LanguageCodeIR::Korean, DiscourseGroupUpdateOperationIR::AddMember) => {
            format!("지정한 대상을 추가했어. 그 담화 묶음은 이제 {count}개 대상을 가리켜.")
        }
        (LanguageCodeIR::Korean, DiscourseGroupUpdateOperationIR::RemoveMember) => {
            format!("지정한 대상을 제외했어. 그 담화 묶음은 이제 {count}개 대상을 가리켜.")
        }
        (LanguageCodeIR::Korean, DiscourseGroupUpdateOperationIR::MergeGroups) => {
            format!("두 담화 묶음을 합쳐 중복 없는 {count}개 대상을 가리키는 새 묶음을 만들었어.")
        }
        (_, DiscourseGroupUpdateOperationIR::Unresolved) => {
            unreachable!("only applied group updates are realized")
        }
    }
}

#[allow(dead_code)]
fn render_action_set_evaluation(
    language: LanguageCodeIR,
    query: &crate::action_state::ActionSetQueryIR,
) -> String {
    let count = query.selected_action_ids.len();
    let truth = match (language, query.truth) {
        (LanguageCodeIR::Korean, ActionSetTruthIR::True) => "맞아",
        (LanguageCodeIR::Korean, ActionSetTruthIR::False) => "아니야",
        (LanguageCodeIR::Korean, _) => "현재 기록만으로는 판단할 수 없어",
        (_, ActionSetTruthIR::True) => "Yes",
        (_, ActionSetTruthIR::False) => "No",
        (_, _) => "The current records do not determine that",
    };
    let scope = match (language, query.quantifier) {
        (LanguageCodeIR::Korean, Some(ActionSetQuantifierIR::All)) => {
            format!("선택된 {count}개 작업 모두")
        }
        (LanguageCodeIR::Korean, Some(ActionSetQuantifierIR::Any)) => {
            format!("선택된 {count}개 작업 중 적어도 하나")
        }
        (LanguageCodeIR::Korean, Some(ActionSetQuantifierIR::None)) => {
            format!("선택된 {count}개 작업 중 어느 것도")
        }
        (_, Some(ActionSetQuantifierIR::All)) => format!("all {count} selected actions"),
        (_, Some(ActionSetQuantifierIR::Any)) => {
            format!("at least one of the {count} selected actions")
        }
        (_, Some(ActionSetQuantifierIR::None)) => {
            format!("none of the {count} selected actions")
        }
        (LanguageCodeIR::Korean, None) => format!("선택된 {count}개 작업"),
        (_, None) => format!("the {count} selected actions"),
    };
    let predicate = match (language, query.predicate) {
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::ActivePlan)) => "활성 계획이야",
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::ReportedCompletion)) => {
            "완료됐다는 사용자 보고가 있어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::ReportedFailure)) => {
            "실패했다는 사용자 보고가 있어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::UnverifiedExecution)) => {
            "검증된 실행 관찰이 없어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::VerifiedExecution)) => {
            "검증된 실행 관찰이 있어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::VerifiedSuccess)) => {
            "검증된 성공 결과가 있어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::VerifiedFailure)) => {
            "검증된 실패 결과가 있어"
        }
        (LanguageCodeIR::Korean, Some(ActionStatePredicateIR::VerifiedInProgress)) => {
            "검증된 실행 중 상태야"
        }
        (_, Some(ActionStatePredicateIR::ActivePlan)) => "are active plans",
        (_, Some(ActionStatePredicateIR::ReportedCompletion)) => "have a user-reported completion",
        (_, Some(ActionStatePredicateIR::ReportedFailure)) => "have a user-reported failure",
        (_, Some(ActionStatePredicateIR::UnverifiedExecution)) => {
            "have no verified execution observation"
        }
        (_, Some(ActionStatePredicateIR::VerifiedExecution)) => {
            "have a verified execution observation"
        }
        (_, Some(ActionStatePredicateIR::VerifiedSuccess)) => "have a verified successful result",
        (_, Some(ActionStatePredicateIR::VerifiedFailure)) => "have a verified failed result",
        (_, Some(ActionStatePredicateIR::VerifiedInProgress)) => {
            "have a verified in-progress state"
        }
        (LanguageCodeIR::Korean, None) => "선택됐어",
        (_, None) => "were selected",
    };
    match language {
        LanguageCodeIR::Korean => format!(
            "{truth}. 현재 행위 원장 기준으로 {scope} {predicate}. 사용자 언어 보고와 호스트 검증 결과는 서로 분리돼 있어."
        ),
        _ => format!(
            "{truth}. According to the action ledger, {scope} {predicate}. User language reports remain separate from host-verified execution results."
        ),
    }
}

#[allow(dead_code)]
fn render_action_group_response(
    language: LanguageCodeIR,
    analysis: &ActionStateAnalysisIR,
    records: &[&ActionStateRecordIR],
) -> String {
    let subjects = records
        .iter()
        .map(|record| restore_user_grounded_acronyms(&record.subject, &record.source_semantic_text))
        .collect::<Vec<_>>();
    if analysis.has_language_reports() {
        let list = subjects
            .iter()
            .map(|subject| format!("‘{subject}’"))
            .collect::<Vec<_>>()
            .join(", ");
        return match language {
            LanguageCodeIR::Korean => format!(
                "{list} 작업을 사용자가 보고한 상태로 함께 기록했어. 언어 보고는 검증된 실행 결과가 아니므로 각 작업의 실행 상태는 승격하지 않았어."
            ),
            _ => format!(
                "I recorded {list} as user-reported outcomes. Language reports are not verified execution results, so none of their execution states was promoted."
            ),
        };
    }
    let rows = records
        .iter()
        .zip(subjects.iter())
        .map(|(record, subject)| {
            let status = match (language, record.execution_status, record.reported_status) {
                (LanguageCodeIR::Korean, ActionExecutionStatusIR::NotObserved, None) => {
                    "활성 계획, 검증된 실행 결과 없음"
                }
                (LanguageCodeIR::Korean, ActionExecutionStatusIR::NotObserved, Some(_)) => {
                    "사용자 보고만 있음, 검증된 실행 결과 없음"
                }
                (LanguageCodeIR::Korean, ActionExecutionStatusIR::InProgress, _) => {
                    "호스트 영수증 기준 실행 중"
                }
                (LanguageCodeIR::Korean, ActionExecutionStatusIR::Succeeded, _) => {
                    "호스트 영수증으로 성공 검증"
                }
                (LanguageCodeIR::Korean, ActionExecutionStatusIR::Failed, _) => {
                    "호스트 영수증으로 실패 검증"
                }
                (_, ActionExecutionStatusIR::NotObserved, None) => {
                    "active plan; no verified execution result"
                }
                (_, ActionExecutionStatusIR::NotObserved, Some(_)) => {
                    "user report only; no verified execution result"
                }
                (_, ActionExecutionStatusIR::InProgress, _) => {
                    "in progress by verified host receipt"
                }
                (_, ActionExecutionStatusIR::Succeeded, _) => "success by verified host receipts",
                (_, ActionExecutionStatusIR::Failed, _) => "failure by verified host receipts",
            };
            format!("‘{subject}’ — {status}")
        })
        .collect::<Vec<_>>()
        .join("; ");
    match language {
        LanguageCodeIR::Korean => format!("선택된 작업들의 상태는 다음과 같아: {rows}."),
        _ => format!("The selected action states are: {rows}."),
    }
}

fn dialogue_directive_candidates(
    pragmatic_interpretation: &PragmaticInterpretationIR,
    lexical_analysis: &LanguageDialogueDirectiveAnalysisIR,
    source_surface: &str,
) -> Vec<DialogueDirectiveCandidateIR> {
    let mut candidates = Vec::new();
    if let Some(feedback) = pragmatic_interpretation.user_feedback.as_ref() {
        let value_key = match feedback.kind {
            UserFeedbackKindIR::TooVerbose => Some("CONCISE"),
            UserFeedbackKindIR::TooBrief => Some("DETAILED"),
            UserFeedbackKindIR::Unhelpful
            | UserFeedbackKindIR::Misunderstood
            | UserFeedbackKindIR::MissedPoint
            | UserFeedbackKindIR::Incorrect => None,
        };
        if let Some(value_key) = value_key {
            candidates.push(DialogueDirectiveCandidateIR::from_surface(
                DialogueDirectiveKindIR::ResponseLength,
                "ASSISTANT_RESPONSE",
                value_key,
                source_surface,
                feedback.confidence_millis,
            ));
        }
    }
    candidates.extend(lexical_analysis.frames.iter().map(|frame| {
        let kind = match frame.axis {
            LanguageDialogueDirectiveAxisIR::ResponseLength => {
                DialogueDirectiveKindIR::ResponseLength
            }
            LanguageDialogueDirectiveAxisIR::ResponseFormat => {
                DialogueDirectiveKindIR::ResponseFormat
            }
        };
        let value_key = match frame.value {
            LanguageDialogueDirectiveValueIR::Concise => "CONCISE",
            LanguageDialogueDirectiveValueIR::Detailed => "DETAILED",
            LanguageDialogueDirectiveValueIR::Bullets => "BULLETS",
            LanguageDialogueDirectiveValueIR::Numbered => "NUMBERED",
            LanguageDialogueDirectiveValueIR::Table => "TABLE",
            LanguageDialogueDirectiveValueIR::Plain => "PLAIN",
        };
        DialogueDirectiveCandidateIR::from_surface(
            kind,
            "ASSISTANT_RESPONSE",
            value_key,
            source_surface,
            frame.confidence_millis,
        )
    }));
    candidates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.target_key.cmp(&right.target_key))
            .then_with(|| right.confidence_millis.cmp(&left.confidence_millis))
            .then_with(|| left.value_key.cmp(&right.value_key))
    });
    candidates
        .dedup_by(|left, right| left.kind == right.kind && left.target_key == right.target_key);
    candidates
}

fn is_dialogue_directive_goal(subject: &str) -> bool {
    contains_any_surface(
        &subject.to_lowercase(),
        &[
            "답변",
            "응답",
            "대답",
            "설명",
            "말해",
            "answer",
            "response",
            "respond",
            "reply",
            "explanation",
        ],
    )
}

fn planner_inferred_goal<'a>(
    interpretation: &'a PragmaticInterpretationIR,
    directive_analysis: &LanguageDialogueDirectiveAnalysisIR,
) -> Option<&'a crate::pragmatics::InferredPragmaticGoalIR> {
    interpretation.inferred_goal.as_ref().filter(|goal| {
        // A typed response-policy directive is conversation control state.
        // Its response noun may be misread as an action subject by a legacy
        // pragmatic fallback, but it must never be appended to task GoalIR.
        directive_analysis.frames.is_empty() || !is_dialogue_directive_goal(&goal.subject)
    })
}

fn dialogue_directive_tag(directive: &DialogueDirectiveIR) -> String {
    format!(
        "DIALOGUE_DIRECTIVE:{:?}:{}:{}",
        directive.kind, directive.target_key, directive.value_key
    )
    .to_uppercase()
}

#[allow(dead_code)]
fn render_user_feedback(
    language: LanguageCodeIR,
    interpretation: &PragmaticInterpretationIR,
) -> String {
    let kind = interpretation
        .user_feedback
        .as_ref()
        .map(|feedback| feedback.kind)
        .unwrap_or(UserFeedbackKindIR::Unhelpful);
    match (language, kind) {
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Unhelpful) => {
            "맞아, 지금 답변이 도움이 되지 않았네. 무엇이 어긋났는지 알려주면 그 기준으로 바로잡을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Misunderstood) => {
            "내가 네 말을 잘못 이해했네. 어느 부분이 달랐는지 짚어주면 바로잡을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::MissedPoint) => {
            "내가 핵심을 놓쳤네. 놓친 요점을 짚어주면 그 기준으로 바로잡을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::TooVerbose) => {
            "맞아, 설명이 너무 길었네. 다음 답변은 핵심만 짧게 말할게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::TooBrief) => {
            "맞아, 설명이 너무 짧았네. 다음에는 필요한 근거와 맥락을 더 자세히 말할게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Incorrect) => {
            "내 답변이 정확하지 않았네. 틀린 부분을 확인해서 바로잡을게.".to_string()
        }
        (_, UserFeedbackKindIR::Unhelpful) => {
            "You're right—the answer wasn't useful enough. Tell me what missed the mark and I'll correct it.".to_string()
        }
        (_, UserFeedbackKindIR::Misunderstood) => {
            "I misunderstood you. Point out what I got wrong and I'll correct it.".to_string()
        }
        (_, UserFeedbackKindIR::MissedPoint) => {
            "I missed your point. Tell me what I misunderstood and I'll correct it.".to_string()
        }
        (_, UserFeedbackKindIR::TooVerbose) => {
            "You're right—the explanation was too long. I'll keep the next answer concise and focused.".to_string()
        }
        (_, UserFeedbackKindIR::TooBrief) => {
            "You're right—the explanation was too brief. I'll add the missing detail and context.".to_string()
        }
        (_, UserFeedbackKindIR::Incorrect) => {
            "My answer was incorrect. I'll identify what was wrong and correct it.".to_string()
        }
    }
}

#[allow(dead_code)]
fn render_user_feedback_for_explicit_request(
    language: LanguageCodeIR,
    interpretation: &PragmaticInterpretationIR,
) -> String {
    let kind = interpretation
        .user_feedback
        .as_ref()
        .map(|feedback| feedback.kind)
        .unwrap_or(UserFeedbackKindIR::Unhelpful);
    match (language, kind) {
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Unhelpful) => {
            "맞아, 방금 답변이 도움이 안 됐네. 이번 요청 기준으로 바로잡을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Misunderstood) => {
            "내가 네 말을 잘못 이해했네. 이번에는 지정한 기준으로 바로잡을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::MissedPoint) => {
            "내가 핵심을 놓쳤네. 이번에는 네가 짚은 요점에 맞출게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::TooVerbose) => {
            "맞아, 설명이 너무 길었네. 이번에는 핵심만 짧게 맞출게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::TooBrief) => {
            "맞아, 설명이 너무 짧았네. 이번에는 필요한 근거와 맥락을 더 자세히 담을게.".to_string()
        }
        (LanguageCodeIR::Korean, UserFeedbackKindIR::Incorrect) => {
            "내 답변이 정확하지 않았네. 이번 요청 기준으로 틀린 부분을 바로잡을게.".to_string()
        }
        (_, UserFeedbackKindIR::Unhelpful) => {
            "You're right—the last answer wasn't useful enough. I'll correct it against this request.".to_string()
        }
        (_, UserFeedbackKindIR::Misunderstood) => {
            "I misunderstood you. I'll correct that against the requirement you just gave.".to_string()
        }
        (_, UserFeedbackKindIR::MissedPoint) => {
            "I missed your point. I'll align this response with the point you just specified.".to_string()
        }
        (_, UserFeedbackKindIR::TooVerbose) => {
            "You're right—the explanation was too long. I'll keep this response concise and focused.".to_string()
        }
        (_, UserFeedbackKindIR::TooBrief) => {
            "You're right—the explanation was too brief. I'll include the needed detail and context this time.".to_string()
        }
        (_, UserFeedbackKindIR::Incorrect) => {
            "My answer was incorrect. I'll correct it against this request.".to_string()
        }
    }
}

#[allow(dead_code)]
fn feedback_plan_subject<'a>(
    language: LanguageCodeIR,
    parsed_subject: &'a str,
    interpretation: &'a PragmaticInterpretationIR,
) -> &'a str {
    let weak_subject = matches!(
        parsed_subject.trim().to_lowercase().as_str(),
        "다시" | "그것" | "it" | "it again" | "it again concisely"
    );
    if !weak_subject {
        return parsed_subject;
    }
    match (
        language,
        interpretation
            .user_feedback
            .as_ref()
            .map(|feedback| feedback.target_surface.as_str()),
    ) {
        (LanguageCodeIR::Korean, Some("answer")) => "답변",
        (LanguageCodeIR::Korean, _) => "설명",
        (_, Some("answer")) => "the answer",
        _ => "the explanation",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UserAffectIR {
    Frustrated,
    Angry,
    Upset,
    Worried,
    Annoyed,
}

fn detect_user_affect(original_text: &str) -> Option<UserAffectIR> {
    let unquoted = strip_quoted_spans(original_text).to_lowercase();
    if contains_any_surface(
        &unquoted,
        &[
            "답답",
            "지친",
            "지쳤",
            "힘들어",
            "frustrating",
            "frustrated",
            "exhausted",
            "tired",
            "worn out",
            "drained",
            "진이 빠",
        ],
    ) {
        Some(UserAffectIR::Frustrated)
    } else if contains_any_surface(&unquoted, &["화나", "화가 나", "angry"]) {
        Some(UserAffectIR::Angry)
    } else if contains_any_surface(&unquoted, &["속상", "upset"]) {
        Some(UserAffectIR::Upset)
    } else if contains_any_surface(&unquoted, &["불안", "걱정", "worried", "worrying"]) {
        Some(UserAffectIR::Worried)
    } else if contains_any_surface(&unquoted, &["짜증", "킹받", "annoying", "annoyed"]) {
        Some(UserAffectIR::Annoyed)
    } else {
        None
    }
}

fn has_social_dialogue_event(normalization: &NormalizedUtteranceIR) -> bool {
    normalization.discourse_events.iter().any(|event| {
        matches!(
            event.function,
            DiscourseFunctionIR::Greeting
                | DiscourseFunctionIR::Gratitude
                | DiscourseFunctionIR::Farewell
        )
    })
}

fn contains_any_surface(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[allow(dead_code)]
fn realize_user_affect(language: LanguageCodeIR, affect: UserAffectIR) -> &'static str {
    match (language, affect) {
        (LanguageCodeIR::Korean, UserAffectIR::Frustrated) => "계속 그러면 정말 답답할 만해.",
        (LanguageCodeIR::Korean, UserAffectIR::Angry) => "또 그러면 화날 만해.",
        (LanguageCodeIR::Korean, UserAffectIR::Upset) => "그건 속상할 만해.",
        (LanguageCodeIR::Korean, UserAffectIR::Worried) => "계속 그러면 불안할 만해.",
        (LanguageCodeIR::Korean, UserAffectIR::Annoyed) => "그건 정말 짜증날 만해.",
        (_, UserAffectIR::Frustrated) => "That sounds genuinely frustrating.",
        (_, UserAffectIR::Angry) => "It makes sense that you are angry about that.",
        (_, UserAffectIR::Upset) => "That is understandably upsetting.",
        (_, UserAffectIR::Worried) => "It makes sense to worry when that keeps happening.",
        (_, UserAffectIR::Annoyed) => "That sounds genuinely annoying.",
    }
}

#[allow(dead_code)]
fn render_standalone_user_affect(language: LanguageCodeIR, affect: UserAffectIR) -> String {
    let empathy = realize_user_affect(language, affect);
    match language {
        LanguageCodeIR::Korean => format!(
            "{empathy} 이 말만으로 변경 작업을 시작했다고 가정하지 않을게. 원하면 먼저 확인할 대상을 같이 좁혀보자."
        ),
        _ => format!(
            "{empathy} I won't assume that you authorized a change from this alone. If you want, we can narrow down the first thing to check."
        ),
    }
}

fn strip_quoted_spans(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut closing_quote = None;
    for character in text.chars() {
        if let Some(closing) = closing_quote {
            if character == closing {
                closing_quote = None;
            }
            result.push(' ');
            continue;
        }
        closing_quote = match character {
            '‘' => Some('’'),
            '“' => Some('”'),
            '"' => Some('"'),
            _ => None,
        };
        if closing_quote.is_some() {
            result.push(' ');
        } else {
            result.push(character);
        }
    }
    result
}

fn is_quoted_metalinguistic_request(text: &str) -> bool {
    let has_quoted_span = text
        .chars()
        .any(|character| matches!(character, '‘' | '“' | '"'));
    if !has_quoted_span {
        return false;
    }
    let unquoted = strip_quoted_spans(text).to_lowercase();
    contains_any_surface(
        &unquoted,
        &[
            "문법",
            "구문",
            "표현",
            "문장",
            "인용",
            "grammar",
            "syntax",
            "wording",
            "phrase",
            "sentence",
            "quotation",
            "quote",
        ],
    ) && contains_any_surface(
        &unquoted,
        &[
            "설명", "분석", "해석", "뜻", "explain", "analyze", "analyse", "parse", "meaning",
            "describe",
        ],
    )
}

#[allow(dead_code)]
fn conversation_display_subject(
    understanding: &LanguageUnderstandingIR,
    reference_resolution: &ReferenceResolutionIR,
) -> String {
    let bound_surface = reference_resolution
        .discourse_bindings
        .iter()
        .find(|binding| {
            matches!(
                binding.kind,
                DiscourseBindingKindIR::EventReference
                    | DiscourseBindingKindIR::EventOrdinalReference
                    | DiscourseBindingKindIR::ResultReference
                    | DiscourseBindingKindIR::PropositionReference
                    | DiscourseBindingKindIR::TopicAnchoredPropositionGroupReference
                    | DiscourseBindingKindIR::TopicAnchoredPropositionMemberReference
            )
        })
        .map(|binding| binding.resolved_surface.trim())
        .filter(|surface| !surface.is_empty());
    restore_user_grounded_acronyms(
        bound_surface.unwrap_or_else(|| understanding.subject.trim()),
        &understanding.original_text,
    )
}

#[allow(dead_code)]
fn render_conversation_plan_preview(
    language: LanguageCodeIR,
    subject: &str,
    plan: &PlanIR,
) -> String {
    let preferred: &[PlanOperationIR] = match plan.intent {
        dockable_semantic_core::PlanIntentIR::Investigate => &[
            PlanOperationIR::ObserveCurrentState,
            PlanOperationIR::RunDiagnostic,
            PlanOperationIR::VerifyOutcome,
        ],
        dockable_semantic_core::PlanIntentIR::Repair
        | dockable_semantic_core::PlanIntentIR::Execute => &[
            PlanOperationIR::ObserveCurrentState,
            PlanOperationIR::ApplySelectedAction,
            PlanOperationIR::VerifyOutcome,
        ],
        dockable_semantic_core::PlanIntentIR::Create => &[
            PlanOperationIR::DerivePostconditions,
            PlanOperationIR::ApplySelectedAction,
            PlanOperationIR::VerifyOutcome,
        ],
        dockable_semantic_core::PlanIntentIR::Learn => &[
            PlanOperationIR::ModelKnowledgeGap,
            PlanOperationIR::GeneralizeLesson,
            PlanOperationIR::VerifyOutcome,
        ],
        dockable_semantic_core::PlanIntentIR::Explain
        | dockable_semantic_core::PlanIntentIR::Communicate => &[
            PlanOperationIR::ObserveCurrentState,
            PlanOperationIR::SynthesizeExplanation,
            PlanOperationIR::CommunicateResult,
        ],
        dockable_semantic_core::PlanIntentIR::Plan => &[
            PlanOperationIR::DerivePostconditions,
            PlanOperationIR::GenerateCandidates,
            PlanOperationIR::VerifyOutcome,
        ],
    };
    let mut operations = preferred
        .iter()
        .copied()
        .filter(|operation| plan.steps.iter().any(|step| step.operation == *operation))
        .collect::<Vec<_>>();
    for operation in plan.steps.iter().map(|step| step.operation) {
        if operations.len() >= 3 {
            break;
        }
        if !operations.contains(&operation) {
            operations.push(operation);
        }
    }
    let realized = operations
        .iter()
        .enumerate()
        .map(|(index, operation)| match language {
            LanguageCodeIR::Korean => {
                format!("{}) {}", index + 1, korean_operation(*operation))
            }
            _ => format!(
                "{}) {}",
                index + 1,
                english_operation(*operation).to_ascii_lowercase()
            ),
        })
        .collect::<Vec<_>>()
        .join(if language == LanguageCodeIR::Korean {
            ", "
        } else {
            "; "
        });
    match language {
        LanguageCodeIR::Korean => {
            let display = quote_subject_once(subject);
            let particle = korean_topic_particle(subject);
            format!(
                "알겠어. {display}{particle} 다음 검증 계획으로 처리할게. {realized}. 아직 실행 결과는 없으므로 이 단계들을 완료된 사실로 말하지 않을게."
            )
        }
        _ => {
            let display = quote_subject_once(subject);
            format!(
                "Got it. For {display}, I'll use this validated plan: {realized}. These are planned operations, not completed results."
            )
        }
    }
}

#[allow(dead_code)]
fn quote_subject_once(subject: &str) -> String {
    if subject.contains('‘') || subject.contains('’') {
        subject.to_string()
    } else {
        format!("‘{subject}’")
    }
}

fn korean_topic_particle(subject: &str) -> &'static str {
    let final_character = subject.chars().rev().find(|character| {
        character.is_alphanumeric()
            || matches!(character, '\u{ac00}'..='\u{d7a3}' | '\u{3131}'..='\u{318e}')
    });
    let has_final_consonant = final_character.is_some_and(|character| {
        if matches!(character, '\u{ac00}'..='\u{d7a3}') {
            (u32::from(character) - 0xac00) % 28 != 0
        } else {
            false
        }
    });
    if has_final_consonant {
        "은"
    } else {
        "는"
    }
}

fn korean_object_particle(subject: &str) -> &'static str {
    if korean_topic_particle(subject) == "은" {
        "을"
    } else {
        "를"
    }
}

fn user_grounded_acronyms(text: &str) -> Vec<String> {
    let mut acronyms = Vec::new();
    let mut token = String::new();
    let flush = |token: &mut String, acronyms: &mut Vec<String>| {
        let uppercase = token
            .chars()
            .filter(|character| character.is_ascii_uppercase())
            .count();
        let has_lowercase = token
            .chars()
            .any(|character| character.is_ascii_lowercase());
        if token.len() <= 16 && uppercase >= 2 && !has_lowercase && !acronyms.contains(token) {
            acronyms.push(token.clone());
        }
        token.clear();
    };
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
        } else {
            flush(&mut token, &mut acronyms);
        }
    }
    flush(&mut token, &mut acronyms);
    acronyms
}

fn restore_user_grounded_acronyms(text: &str, original_text: &str) -> String {
    let acronyms = user_grounded_acronyms(original_text);
    if acronyms.is_empty() {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut token = String::new();
    let flush = |token: &mut String, output: &mut String| {
        if let Some(acronym) = acronyms
            .iter()
            .find(|acronym| acronym.eq_ignore_ascii_case(token))
        {
            output.push_str(acronym);
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

fn restore_user_grounded_display_forms(text: &str, original_text: &str) -> String {
    let mut forms = user_grounded_acronyms(original_text);
    let mut token = String::new();
    let flush = |token: &mut String, forms: &mut Vec<String>| {
        let title_case = token
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
            && token
                .chars()
                .skip(1)
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
        if (2..=32).contains(&token.len()) && title_case && !forms.contains(token) {
            forms.push(token.clone());
        }
        token.clear();
    };
    for character in original_text.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
        } else {
            flush(&mut token, &mut forms);
        }
    }
    flush(&mut token, &mut forms);
    if forms.is_empty() {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len());
    let mut token = String::new();
    let flush = |token: &mut String, output: &mut String| {
        if let Some(form) = forms.iter().find(|form| form.eq_ignore_ascii_case(token)) {
            output.push_str(form);
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

fn clarification_generation_source(
    language: LanguageCodeIR,
    normalization: &NormalizedUtteranceIR,
    resolution: &ReferenceResolutionIR,
    pragmatic_interpretation: &PragmaticInterpretationIR,
) -> (GenerationClarificationKindIR, Option<String>) {
    let has_reference_marker = |marker: &str| {
        resolution
            .ambiguous_reference_surfaces
            .iter()
            .any(|surface| surface == marker)
    };
    if has_reference_marker("PENDING_QUD_ANSWER") {
        return (GenerationClarificationKindIR::PendingChoice, None);
    }
    if has_reference_marker("LOCAL_ORDERED_ANTECEDENT_SET") {
        return (GenerationClarificationKindIR::OrderedPair, None);
    }
    if has_reference_marker("LOCAL_ORDINAL_ANTECEDENT_SET") {
        return (GenerationClarificationKindIR::LocalOrdinal, None);
    }
    if has_reference_marker("EVENT_SEQUENCE_ORDINAL") {
        return (GenerationClarificationKindIR::EventOrdinal, None);
    }
    if has_reference_marker("PREVIOUS_TOPIC_STACK") {
        return (GenerationClarificationKindIR::PreviousTopic, None);
    }
    if pragmatic_interpretation
        .compositional_analysis
        .clarification_required
        && resolution.ambiguous_reference_surfaces.is_empty()
    {
        let detail = match language {
            LanguageCodeIR::Korean => "서로 다른 요청 후보",
            _ => "competing request readings",
        };
        return (
            GenerationClarificationKindIR::CompetingRequest,
            Some(detail.to_string()),
        );
    }
    if pragmatic_interpretation
        .nonliteral_analysis
        .clarification_required
    {
        let detail = pragmatic_interpretation
            .nonliteral_analysis
            .expressions
            .iter()
            .find(|expression| {
                expression.selected_reading == crate::nonliteral::ReadingSelectionIR::Ambiguous
            })
            .map(|expression| expression.surface_text.clone());
        return (GenerationClarificationKindIR::NonliteralReading, detail);
    }
    if normalization.ambiguous_input {
        let separator = if language == LanguageCodeIR::Korean {
            " 또는 "
        } else {
            " or "
        };
        let detail = normalization
            .candidates
            .iter()
            .take(2)
            .map(|candidate| format!("‘{}’", candidate.normalized_text))
            .collect::<Vec<_>>()
            .join(separator);
        return (
            GenerationClarificationKindIR::VoiceAlternative,
            (!detail.is_empty()).then_some(detail),
        );
    }
    if !resolution.ambiguous_reference_surfaces.is_empty() {
        let detail = resolution
            .ambiguous_reference_surfaces
            .iter()
            .map(|surface| {
                let internal_marker = !surface.is_empty()
                    && surface
                        .split_once(':')
                        .map_or(surface.as_str(), |(prefix, _)| prefix)
                        .chars()
                        .all(|character| {
                            character.is_ascii_uppercase()
                                || character.is_ascii_digit()
                                || matches!(character, '_' | ':' | '-')
                        });
                if internal_marker {
                    format!("‘{}’", normalization.semantic_surface_text)
                } else {
                    format!("‘{surface}’")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        return (
            GenerationClarificationKindIR::Reference,
            (!detail.is_empty()).then_some(detail),
        );
    }
    (GenerationClarificationKindIR::MissingDetails, None)
}

#[allow(dead_code)]
fn render_non_grounded_conversation(
    language: LanguageCodeIR,
    disposition: ConversationTurnDispositionIR,
    normalization: &NormalizedUtteranceIR,
    resolution: &ReferenceResolutionIR,
    pragmatic_interpretation: &PragmaticInterpretationIR,
) -> String {
    match (language, disposition) {
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::HoldFloor) => {
            "응, 천천히 말해. 듣고 있어.".to_string()
        }
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::BackchannelOnly) => {
            if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Greeting)
            {
                "안녕! 무엇을 도와줄까?".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Gratitude)
            {
                "천만에. 더 필요한 게 있으면 말해줘.".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Farewell)
            {
                "좋아. 필요할 때 다시 불러줘.".to_string()
            } else {
                "응, 이어서 말해줘.".to_string()
            }
        }
        (LanguageCodeIR::Korean, ConversationTurnDispositionIR::ClarificationRequired) => {
            if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "PENDING_QUD_ANSWER")
            {
                "앞서 물은 선택지 중 어느 쪽인지 직접 지정해줘. 제3자의 말이나 불확실한 표현을 네 선택으로 간주하지 않을게."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "LOCAL_ORDERED_ANTECEDENT_SET")
            {
                "전자와 후자가 가리킬 후보가 둘보다 많아. 기준이 되는 두 대상을 직접 지정해줘."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "LOCAL_ORDINAL_ANTECEDENT_SET")
            {
                "지정한 순서에 해당하는 대상이 없어. 몇 번째 대상인지 다시 확인해줘.".to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "EVENT_SEQUENCE_ORDINAL")
            {
                "그 순서에 해당하는 작업 기록이 없어. 이전 계획의 몇 번째 작업인지 다시 확인해줘."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "PREVIOUS_TOPIC_STACK")
            {
                "돌아갈 이전 화제가 아직 없어. 어떤 주제로 돌아갈지 이름을 말해줘.".to_string()
            } else if pragmatic_interpretation
                .compositional_analysis
                .clarification_required
                && resolution.ambiguous_reference_surfaces.is_empty()
            {
                let competition = pragmatic_interpretation
                    .compositional_analysis
                    .unresolved_competitions
                    .first()
                    .map_or("서로 다른 요청 후보", String::as_str);
                format!(
                    "문장에서 {competition}가 비슷한 강도로 해석돼. 어느 쪽이 실제 요청인지 지정해줘. 인용·가정·금지된 행동은 임의로 실행하지 않을게."
                )
            } else if pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            {
                let surface = pragmatic_interpretation
                    .nonliteral_analysis
                    .expressions
                    .iter()
                    .find(|expression| {
                        expression.selected_reading
                            == crate::nonliteral::ReadingSelectionIR::Ambiguous
                    })
                    .map_or("해당 표현", |expression| {
                        expression.surface_text.as_str()
                    });
                format!(
                    "‘{surface}’를 문자 그대로의 상황으로 말한 건지, 비유적인 문제 상황으로 말한 건지 확인해줘. 어느 쪽도 추측해서 실행하지 않을게."
                )
            } else if normalization.ambiguous_input {
                let choices = normalization
                    .candidates
                    .iter()
                    .take(2)
                    .map(|candidate| format!("‘{}’", candidate.normalized_text))
                    .collect::<Vec<_>>()
                    .join(" 또는 ");
                format!("음성 입력이 {choices}로 들릴 수 있어. 어느 쪽인지 한 번만 확인해줘.")
            } else if !resolution.ambiguous_reference_surfaces.is_empty() {
                format!(
                    "{}가 무엇을 가리키는지 하나만 지정해줘.",
                    resolution
                        .ambiguous_reference_surfaces
                        .iter()
                        .map(|surface| format!("‘{surface}’"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "무엇을 원하는지 조금만 더 말해줘.".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::HoldFloor) => {
            "Take your time—I'm listening.".to_string()
        }
        (_, ConversationTurnDispositionIR::BackchannelOnly) => {
            if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Greeting)
            {
                "Hi! What can I help you with?".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Gratitude)
            {
                "You're welcome. Let me know if you need anything else.".to_string()
            } else if normalization
                .discourse_events
                .iter()
                .any(|event| event.function == crate::conversation::DiscourseFunctionIR::Farewell)
            {
                "Sounds good. Call me again whenever you need me.".to_string()
            } else {
                "Got it. Go on when you're ready.".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::ClarificationRequired) => {
            if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "PENDING_QUD_ANSWER")
            {
                "Please select one of the options from my previous question directly. I will not treat reported or uncertain wording as your choice."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "LOCAL_ORDERED_ANTECEDENT_SET")
            {
                "There are more than two candidates for ‘former’ and ‘latter’. Please name the two intended items directly."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "LOCAL_ORDINAL_ANTECEDENT_SET")
            {
                "There is no item at that ordinal position. Please confirm which numbered item you mean."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "EVENT_SEQUENCE_ORDINAL")
            {
                "There is no recorded action at that position. Please confirm which step of the earlier plan you mean."
                    .to_string()
            } else if resolution
                .ambiguous_reference_surfaces
                .iter()
                .any(|surface| surface == "PREVIOUS_TOPIC_STACK")
            {
                "There is no earlier topic to return to yet. Please name the topic you mean."
                    .to_string()
            } else if pragmatic_interpretation
                .compositional_analysis
                .clarification_required
                && resolution.ambiguous_reference_surfaces.is_empty()
            {
                let competition = pragmatic_interpretation
                    .compositional_analysis
                    .unresolved_competitions
                    .first()
                    .map_or("competing request readings", String::as_str);
                format!(
                    "The sentence supports {competition} with similar strength. Which one is the actual request? I will not execute quoted, hypothetical, or prohibited actions by guessing."
                )
            } else if pragmatic_interpretation
                .nonliteral_analysis
                .clarification_required
            {
                let surface = pragmatic_interpretation
                    .nonliteral_analysis
                    .expressions
                    .iter()
                    .find(|expression| {
                        expression.selected_reading
                            == crate::nonliteral::ReadingSelectionIR::Ambiguous
                    })
                    .map_or("that expression", |expression| {
                        expression.surface_text.as_str()
                    });
                format!(
                    "Did you mean ‘{surface}’ literally, or as a figurative description of a problem? I won't execute either reading by guessing."
                )
            } else if normalization.ambiguous_input {
                let choices = normalization
                    .candidates
                    .iter()
                    .take(2)
                    .map(|candidate| format!("‘{}’", candidate.normalized_text))
                    .collect::<Vec<_>>()
                    .join(" or ");
                format!("The voice input could be {choices}. Which one did you mean?")
            } else if !resolution.ambiguous_reference_surfaces.is_empty() {
                format!(
                    "What does {} refer to?",
                    resolution
                        .ambiguous_reference_surfaces
                        .iter()
                        .map(|surface| format!("‘{surface}’"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            } else {
                "Could you add a little more detail about what you want?".to_string()
            }
        }
        (_, ConversationTurnDispositionIR::Grounded) => String::new(),
    }
}

fn render_plan(
    language: LanguageCodeIR,
    understanding: &LanguageUnderstandingIR,
    plan: &PlanIR,
) -> NaturalLanguageOutputIR {
    let text = match language {
        LanguageCodeIR::Korean => render_korean(understanding, plan),
        _ => render_english(understanding, plan),
    };
    NaturalLanguageOutputIR {
        language,
        text,
        grounded_plan_sha256: plan.plan_sha256.clone(),
        unsupported_freeform_claims: 0,
    }
}

fn render_korean(understanding: &LanguageUnderstandingIR, plan: &PlanIR) -> String {
    let mut lines = vec![format!(
        "요청을 '{}' 의도로 해석했습니다. 대상: {}",
        korean_intent(understanding.intent),
        understanding.subject
    )];
    if !plan.recalled_experiences.is_empty() {
        lines.push(format!(
            "관련 성공 경험 {}건을 계획 근거로 사용합니다.",
            plan.recalled_experiences.len()
        ));
    }
    for (index, step) in plan.steps.iter().enumerate() {
        lines.push(format!(
            "{}. {}",
            index + 1,
            korean_operation(step.operation)
        ));
    }
    lines.push(format!(
        "검증 단계: {} / 계획 해시: {}",
        plan.terminal_verification_step_id, plan.plan_sha256
    ));
    lines.join("\n")
}

fn render_english(understanding: &LanguageUnderstandingIR, plan: &PlanIR) -> String {
    let mut lines = vec![format!(
        "I interpreted the request as '{}'. Subject: {}",
        english_intent(understanding.intent),
        understanding.subject
    )];
    if !plan.recalled_experiences.is_empty() {
        lines.push(format!(
            "The plan uses {} relevant successful experience(s).",
            plan.recalled_experiences.len()
        ));
    }
    for (index, step) in plan.steps.iter().enumerate() {
        lines.push(format!(
            "{}. {}",
            index + 1,
            english_operation(step.operation)
        ));
    }
    lines.push(format!(
        "Verification step: {} / plan hash: {}",
        plan.terminal_verification_step_id, plan.plan_sha256
    ));
    lines.join("\n")
}

fn korean_intent(intent: dockable_semantic_core::PlanIntentIR) -> &'static str {
    match intent {
        dockable_semantic_core::PlanIntentIR::Plan => "계획 생성",
        dockable_semantic_core::PlanIntentIR::Investigate => "조사·진단",
        dockable_semantic_core::PlanIntentIR::Repair => "수리",
        dockable_semantic_core::PlanIntentIR::Create => "생성·구현",
        dockable_semantic_core::PlanIntentIR::Learn => "학습",
        dockable_semantic_core::PlanIntentIR::Explain => "설명",
        dockable_semantic_core::PlanIntentIR::Communicate => "전달",
        dockable_semantic_core::PlanIntentIR::Execute => "실행",
    }
}

fn english_intent(intent: dockable_semantic_core::PlanIntentIR) -> &'static str {
    match intent {
        dockable_semantic_core::PlanIntentIR::Plan => "planning",
        dockable_semantic_core::PlanIntentIR::Investigate => "investigation",
        dockable_semantic_core::PlanIntentIR::Repair => "repair",
        dockable_semantic_core::PlanIntentIR::Create => "creation",
        dockable_semantic_core::PlanIntentIR::Learn => "learning",
        dockable_semantic_core::PlanIntentIR::Explain => "explanation",
        dockable_semantic_core::PlanIntentIR::Communicate => "communication",
        dockable_semantic_core::PlanIntentIR::Execute => "execution",
    }
}

fn korean_operation(operation: PlanOperationIR) -> &'static str {
    match operation {
        PlanOperationIR::ObserveCurrentState => "현재 상태 관찰",
        PlanOperationIR::RecallRelevantExperience => "관련 경험 회상",
        PlanOperationIR::SurfaceAssumptions => "숨은 가정 표면화",
        PlanOperationIR::DerivePostconditions => "완료 조건 도출",
        PlanOperationIR::ModelKnowledgeGap => "지식 공백 모델링",
        PlanOperationIR::GenerateCandidates => "후보 생성",
        PlanOperationIR::GenerateCompetingHypotheses => "경쟁 가설 생성",
        PlanOperationIR::ConstructCausalModel => "인과 모델 구성",
        PlanOperationIR::PredictConsequences => "결과 예측",
        PlanOperationIR::SimulateCounterfactuals => "반사실 시뮬레이션",
        PlanOperationIR::SelectInformationGainAction => "정보가치 행동 선택",
        PlanOperationIR::RunDiagnostic => "진단 실행",
        PlanOperationIR::ValidateCandidates => "후보 검증",
        PlanOperationIR::ApplySelectedAction => "선택 행동 적용",
        PlanOperationIR::VerifyOutcome => "결과 검증",
        PlanOperationIR::ReplanFromObservation => "관찰 기반 재계획",
        PlanOperationIR::CalibrateConfidence => "확신도 보정",
        PlanOperationIR::GeneralizeLesson => "교훈 일반화",
        PlanOperationIR::StoreSuccessfulExperience => "성공 경험 저장",
        PlanOperationIR::SynthesizeExplanation => "설명 합성",
        PlanOperationIR::CommunicateResult => "결과 전달",
    }
}

fn english_operation(operation: PlanOperationIR) -> &'static str {
    match operation {
        PlanOperationIR::ObserveCurrentState => "Observe current state",
        PlanOperationIR::RecallRelevantExperience => "Recall relevant experience",
        PlanOperationIR::SurfaceAssumptions => "Surface hidden assumptions",
        PlanOperationIR::DerivePostconditions => "Derive completion conditions",
        PlanOperationIR::ModelKnowledgeGap => "Model the knowledge gap",
        PlanOperationIR::GenerateCandidates => "Generate candidates",
        PlanOperationIR::GenerateCompetingHypotheses => "Generate competing hypotheses",
        PlanOperationIR::ConstructCausalModel => "Construct a causal model",
        PlanOperationIR::PredictConsequences => "Predict consequences",
        PlanOperationIR::SimulateCounterfactuals => "Simulate counterfactuals",
        PlanOperationIR::SelectInformationGainAction => "Select an information-gain action",
        PlanOperationIR::RunDiagnostic => "Run a diagnostic",
        PlanOperationIR::ValidateCandidates => "Validate candidates",
        PlanOperationIR::ApplySelectedAction => "Apply the selected action",
        PlanOperationIR::VerifyOutcome => "Verify the outcome",
        PlanOperationIR::ReplanFromObservation => "Replan from observations",
        PlanOperationIR::CalibrateConfidence => "Calibrate confidence",
        PlanOperationIR::GeneralizeLesson => "Generalize the lesson",
        PlanOperationIR::StoreSuccessfulExperience => "Store successful experience",
        PlanOperationIR::SynthesizeExplanation => "Synthesize an explanation",
        PlanOperationIR::CommunicateResult => "Communicate the result",
    }
}

#[cfg(test)]
mod tests {
    use dockable_semantic_core::{
        ActionAuthorityIR, AuthorityEnvelopeIR, CausalMechanismIR, DeliberationDispositionIR,
        DeliberationRequestIR, EvidenceIR, ExperienceOutcomeIR, LiteralIR, MechanismKindIR,
        MechanismKnowledgeIR, MechanismQueryIR, DELIBERATION_REQUEST_SCHEMA, EXPERIENCE_SCHEMA,
        MECHANISM_KNOWLEDGE_SCHEMA,
    };

    use super::*;
    use crate::deferred_commitment::{
        condition_evidence_receipt_sha256, ConditionEvidenceDispositionIR,
        ConditionEvidenceSourceIR, CONDITION_EVIDENCE_REQUEST_SCHEMA,
    };
    use crate::knowledge_work::{
        DocumentKindIR, KnowledgeDocumentIR, KnowledgeSourceIR, OutputDirectiveIR, OutputFormatIR,
        OutputModeIR, PlanProposalIR, SourceTextFormatIR, KNOWLEDGE_WORK_REQUEST_SCHEMA,
        PLAN_PROPOSAL_SCHEMA,
    };
    use crate::lexical_memory::{PartOfSpeechIR, SenseIR, LEXEME_SCHEMA};
    use crate::mechanism_induction::{
        PropositionLexemeIR, StateTransitionObservationIR, TransitionArmIR,
        MECHANISM_INDUCTION_REQUEST_SCHEMA,
    };
    use crate::natural_realization::NaturalResponseFormatIR;
    use crate::raw_mechanism_induction::{
        ObservedValueIR, RawMechanismInductionRequestIR, RawStateTransitionObservationIR,
        RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA,
    };

    fn open_language_pipeline_routing() -> LanguagePipelineRoutingIR {
        LanguagePipelineRoutingIR::from_candidates([
            Some(LanguagePipelineSignalIR::NormalizedGrounded),
            Some(LanguagePipelineSignalIR::DeicticQueryReferenceSafe),
            Some(LanguagePipelineSignalIR::ReferencesFullyResolved),
        ])
    }

    #[test]
    fn language_pipeline_routing_centralizes_qa_ownership() {
        let mut open = open_language_pipeline_routing();
        let original = open.clone();
        open.activate_if(false, LanguagePipelineSignalIR::QuestionAnswer);
        assert_eq!(open, original, "inactive evidence cannot mutate routing");
        open.activate_if(true, LanguagePipelineSignalIR::NormalizedGrounded);
        assert_eq!(open, original, "duplicate evidence is idempotent");
        assert!(open.allows_temporal_qa());
        assert!(open.allows_dialogue_relation_qa(false));
        assert!(open.allows_discourse_qa(false, false));
        assert!(!open.allows_dialogue_relation_qa(true));
        assert!(!open.allows_discourse_qa(true, false));
        assert!(!open.allows_discourse_qa(false, true));

        let exclusive_signals = [
            LanguagePipelineSignalIR::GroupUpdateOwnsTurn,
            LanguagePipelineSignalIR::DefinitionOwnsTurn,
            LanguagePipelineSignalIR::FutureNotificationOwnsTurn,
            LanguagePipelineSignalIR::NativeGoalOwnsTurn,
            LanguagePipelineSignalIR::ActionStateOwnsTurn,
            LanguagePipelineSignalIR::PragmaticForceOwnsSurfaceQuestion,
            LanguagePipelineSignalIR::ResultReferenceOwnsTurn,
            LanguagePipelineSignalIR::InitialContinuationGateOwnsTurn,
        ];
        for signal in exclusive_signals {
            let mut routing = open.clone();
            routing.activate_if(true, signal);
            assert!(!routing.allows_temporal_qa());
            assert!(!routing.allows_dialogue_relation_qa(false));
            assert!(!routing.allows_discourse_qa(false, false));
        }

        let mut explicit_request = open.clone();
        explicit_request.activate_if(true, LanguagePipelineSignalIR::ExplicitSelectedRequest);
        assert!(explicit_request.allows_temporal_qa());
        assert!(!explicit_request.allows_dialogue_relation_qa(false));
        assert!(!explicit_request.allows_discourse_qa(false, false));

        let mut plan_result = open;
        plan_result.activate_if(true, LanguagePipelineSignalIR::PlanResultOwnsTurn);
        assert!(!plan_result.allows_temporal_qa());
        assert!(plan_result.allows_dialogue_relation_qa(false));
        assert!(plan_result.allows_discourse_qa(false, false));
    }

    #[test]
    fn plan_projection_retains_all_blockers_without_module_order_dependence() {
        let first_routing = LanguagePipelineRoutingIR::from_candidates([
            Some(LanguagePipelineSignalIR::GroundedDisposition),
            Some(LanguagePipelineSignalIR::FeedbackOnly),
            Some(LanguagePipelineSignalIR::SemanticGoalAvailable),
            Some(LanguagePipelineSignalIR::QuestionAnswer),
            Some(LanguagePipelineSignalIR::FeedbackOnly),
        ]);
        let reordered_routing = LanguagePipelineRoutingIR::from_candidates([
            Some(LanguagePipelineSignalIR::QuestionAnswer),
            Some(LanguagePipelineSignalIR::SemanticGoalAvailable),
            Some(LanguagePipelineSignalIR::FeedbackOnly),
            Some(LanguagePipelineSignalIR::GroundedDisposition),
        ]);
        assert_eq!(first_routing, reordered_routing);
        let first = PlanProjectionDecisionIR::from_routing(&first_routing);
        let reordered = PlanProjectionDecisionIR::from_routing(&reordered_routing);
        assert_eq!(first, reordered);
        assert_eq!(first.blockers.len(), 2);
        assert!(!first.allows_plan());

        let open_routing = LanguagePipelineRoutingIR::from_candidates([
            Some(LanguagePipelineSignalIR::GroundedDisposition),
            Some(LanguagePipelineSignalIR::SemanticGoalAvailable),
        ]);
        let open = PlanProjectionDecisionIR::from_routing(&open_routing);
        assert!(open.allows_plan());

        let mut guarded_routing = open_routing;
        guarded_routing.activate_if(true, LanguagePipelineSignalIR::ConditionalGuardOwnsTurn);
        let guarded = PlanProjectionDecisionIR::from_routing(&guarded_routing);
        assert_eq!(
            guarded.blockers,
            vec![PlanProjectionBlockerIR::ConditionalGuard]
        );
        assert!(!guarded.allows_plan());
    }

    fn experience() -> ExperienceIR {
        ExperienceIR {
            schema: EXPERIENCE_SCHEMA.to_string(),
            experience_id: "EXP-POWERSHELL-PATH-1".to_string(),
            situation: "PowerShell path handling failed during a Rust build".to_string(),
            action: "use LiteralPath and preserve the exact predecessor".to_string(),
            outcome: ExperienceOutcomeIR::Successful,
            outcome_description: "the build completed and the target path remained exact"
                .to_string(),
            semantic_tags: vec![
                "repair".to_string(),
                "powershell".to_string(),
                "path".to_string(),
            ],
            evidence: vec!["exit_code=0".to_string()],
            confidence_millis: 970,
            source_language: Some("en".to_string()),
        }
    }

    fn induction_request(
        request_id: &str,
        knowledge_id: &str,
        mechanism_id: &str,
        cause_id: &str,
        cause_alias: &str,
        effect_id: &str,
        effect_alias: &str,
    ) -> MechanismInductionRequestIR {
        let state = |effect: bool| {
            vec![
                LiteralIR {
                    proposition_id: cause_id.to_string(),
                    value: true,
                },
                LiteralIR {
                    proposition_id: effect_id.to_string(),
                    value: effect,
                },
            ]
        };
        let observation = |suffix: &str, arm, after_effect| StateTransitionObservationIR {
            observation_id: format!("{request_id}-{suffix}"),
            arm,
            before: state(false),
            after: state(after_effect),
            reliability_millis: 950,
            evidence_refs: vec![format!("public-test:{request_id}:{suffix}")],
        };
        MechanismInductionRequestIR {
            schema: MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
            request_id: request_id.to_string(),
            knowledge_id: knowledge_id.to_string(),
            mechanism_id: mechanism_id.to_string(),
            natural_language_statement: format!("{cause_alias} causes {effect_alias}"),
            kind: MechanismKindIR::Inference,
            authority: ActionAuthorityIR::InternalInference,
            authorized: true,
            reversible: true,
            recovery_reference: None,
            semantic_tags: vec!["induced-chain".to_string()],
            proposition_lexicon: vec![
                PropositionLexemeIR {
                    proposition_id: cause_id.to_string(),
                    aliases: vec![cause_alias.to_string()],
                },
                PropositionLexemeIR {
                    proposition_id: effect_id.to_string(),
                    aliases: vec![effect_alias.to_string()],
                },
            ],
            observations: vec![
                observation("POS-1", TransitionArmIR::AppliedSuccess, true),
                observation("POS-2", TransitionArmIR::AppliedSuccess, true),
                observation("CONTROL", TransitionArmIR::NoActionControl, false),
            ],
            minimum_positive_support: 2,
            minimum_confidence_millis: 700,
        }
    }

    #[test]
    fn public_api_injects_experience_and_grounds_korean_plan_and_output() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        assert!(api.inject_experience(experience()).unwrap().inserted);
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-KO-1".to_string(),
                text: "파워쉘 경로 오류를 점검하고 수리 계획 세워줘. ㄱㄱ".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: vec!["powershell".to_string(), "path".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            response.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Repair,
            "pragmatic interpretation: {:#?}",
            response.pragmatic_interpretation
        );
        assert_eq!(response.plan.recalled_experiences.len(), 1);
        assert!(response.output.text.contains("관련 성공 경험 1건"));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn semantic_planner_boundary_preserves_sequence_scope_and_every_selected_event() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-TYPED-SEQUENCE".to_string(),
                text:
                    "Read the file, transform each line, then save it. Do not delete the original."
                        .to_string(),
                output_language: Some(LanguageCodeIR::English),
                context_tags: vec!["typed-planner-boundary".to_string()],
                max_plan_steps: 16,
            })
            .expect("typed multi-event plan");

        assert!(response.validate(), "{response:#?}");
        assert_eq!(response.semantic_goal.selected_live_event_ids.len(), 3);
        assert_eq!(response.semantic_plan_bundle.plans.len(), 3);
        assert_eq!(response.semantic_goal.relations.len(), 2);
        assert!(response.semantic_goal.relations.iter().all(|relation| {
            relation.relation == dockable_semantic_core::SemanticPlanRelationKindIR::Sequence
        }));
        assert!(response.semantic_goal.events.iter().any(|event| {
            event.predicate_concept_id == "DELETE"
                && event.projection == dockable_semantic_core::SemanticPlanProjectionIR::Prohibited
                && !response
                    .semantic_goal
                    .selected_live_event_ids
                    .contains(&event.event_id)
        }));
        assert!(response
            .semantic_plan_bundle
            .validate_against(&response.semantic_goal));
        assert_eq!(
            response
                .semantic_plan_bundle
                .event_plan_bindings
                .iter()
                .map(|binding| binding.event_id.as_str())
                .collect::<Vec<_>>(),
            response
                .semantic_goal
                .selected_live_event_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn natural_realization_coverage_rejects_omitted_semantic_plan_obligations() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let request = conversation_request(
            "CHAT-TYPED-REALIZATION-COVERAGE",
            1,
            "Read the file, transform each line, then save it. Do not delete the original.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("typed realization coverage");
        let semantic_goal = &response
            .grounded_response
            .as_deref()
            .expect("grounded semantic goal")
            .semantic_goal;
        let coverage = &response.natural_realization.coverage;
        assert!(coverage.validate_against(
            &response.natural_realization.response_plan,
            &response.natural_realization.generation_traces,
            Some(semantic_goal),
        ));
        assert_eq!(coverage.omitted_required_obligations, 0);
        assert_eq!(
            coverage
                .obligations
                .iter()
                .filter(|obligation| {
                    obligation.kind
                        == crate::natural_realization::NaturalRealizationObligationKindIR::SelectedPlanEvent
                })
                .count(),
            3
        );
        assert_eq!(
            coverage
                .obligations
                .iter()
                .filter(|obligation| {
                    obligation.kind
                        == crate::natural_realization::NaturalRealizationObligationKindIR::SelectedEventRelation
                })
                .count(),
            2
        );
        assert_eq!(
            coverage
                .obligations
                .iter()
                .filter(|obligation| {
                    obligation.kind
                        == crate::natural_realization::NaturalRealizationObligationKindIR::ProhibitedPlanEvent
                })
                .count(),
            1
        );

        let mut omitted = coverage.clone();
        let removed_index = omitted
            .obligations
            .iter()
            .position(|obligation| {
                obligation.kind
                    == crate::natural_realization::NaturalRealizationObligationKindIR::SelectedPlanEvent
            })
            .expect("selected event obligation");
        omitted.obligations.remove(removed_index);
        omitted.coverage_sha256 =
            crate::natural_realization::natural_realization_coverage_sha256(&omitted);
        assert!(!omitted.validate_against(
            &response.natural_realization.response_plan,
            &response.natural_realization.generation_traces,
            Some(semantic_goal),
        ));
    }

    #[test]
    fn json_api_supports_english_input_and_output() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let request = serde_json::to_string(&NaturalLanguageRequestIR {
            schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
            request_id: "REQ-EN-1".to_string(),
            text: "FYI, please analyze the root cause and plan a repair".to_string(),
            output_language: Some(LanguageCodeIR::English),
            context_tags: vec!["diagnosis".to_string()],
            max_plan_steps: 12,
        })
        .unwrap();
        let response: NaturalLanguageResponseIR =
            serde_json::from_str(&api.process_json(&request).unwrap()).unwrap();
        assert_eq!(response.output.language, LanguageCodeIR::English);
        assert!(response.output.text.contains("Verify the outcome"));
        assert!(response.plan.structurally_validated);
    }

    #[test]
    fn public_snapshot_api_restores_experience_without_semantic_state_mutation() {
        let mut source = CognitiveApi::new_embedded().unwrap();
        source.inject_experience(experience()).unwrap();
        let snapshot = source.export_experience_snapshot_json().unwrap();
        let mut destination = CognitiveApi::new_embedded().unwrap();
        let receipts: Vec<ExperienceInjectionReceiptIR> = serde_json::from_str(
            &destination
                .import_experience_snapshot_json(&snapshot)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(destination.retained_experience_count(), 1);
    }

    #[test]
    fn command_api_keeps_injected_experience_live_for_following_requests() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let injection = api.execute_command(CognitiveApiCommandIR::InjectExperience {
            experience: experience(),
        });
        assert!(injection.ok);
        let response = api.execute_command(CognitiveApiCommandIR::ProcessNaturalLanguage {
            request: NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-COMMAND-1".to_string(),
                text: "Please plan a path repair".to_string(),
                output_language: Some(LanguageCodeIR::English),
                context_tags: vec!["path".to_string()],
                max_plan_steps: 12,
            },
        });
        let Some(CognitiveApiPayloadIR::NaturalLanguageResponse(response)) = response.payload
        else {
            panic!("typed natural-language response")
        };
        assert_eq!(response.plan.recalled_experiences.len(), 1);
    }

    #[test]
    fn public_command_api_runs_compositional_deliberation_and_enforces_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let request = DeliberationRequestIR {
            schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
            request_id: "DELIBERATE-PUBLIC-1".to_string(),
            subject: "localize and repair an observed pipeline failure".to_string(),
            evidence: vec![EvidenceIR {
                evidence_id: "E-FAILURE".to_string(),
                literal: literal("FAILURE_OBSERVED"),
                reliability_millis: 980,
                source_ref: "public-test:failure".to_string(),
            }],
            mechanisms: vec![
                CausalMechanismIR {
                    mechanism_id: "LOCALIZE".to_string(),
                    kind: MechanismKindIR::Inference,
                    prerequisites: vec![literal("FAILURE_OBSERVED")],
                    effects: vec![literal("CAUSE_LOCALIZED")],
                    observes: Vec::new(),
                    authority: ActionAuthorityIR::InternalInference,
                    authorized: true,
                    reversible: true,
                    recovery_reference: None,
                    cost_millis: 10,
                    risk_millis: 0,
                    provenance_refs: vec!["public-test:localizer".to_string()],
                },
                CausalMechanismIR {
                    mechanism_id: "REPAIR".to_string(),
                    kind: MechanismKindIR::Intervention,
                    prerequisites: vec![literal("CAUSE_LOCALIZED")],
                    effects: vec![literal("REPAIRED")],
                    observes: Vec::new(),
                    authority: ActionAuthorityIR::ReversibleMutation,
                    authorized: true,
                    reversible: true,
                    recovery_reference: Some("sealed-predecessor:public-test".to_string()),
                    cost_millis: 50,
                    risk_millis: 10,
                    provenance_refs: vec!["public-test:repair".to_string()],
                },
            ],
            goals: vec![literal("REPAIRED")],
            authority_envelope: AuthorityEnvelopeIR::default(),
            immutable_constraints: Vec::new(),
            max_depth: 4,
            beam_width: 8,
            max_hypotheses: 8,
            max_counterfactuals: 8,
        };
        let response = api.execute_command(CognitiveApiCommandIR::DeliberateProblem { request });
        let Some(CognitiveApiPayloadIR::Deliberation(result)) = response.payload else {
            panic!("typed deliberation response")
        };
        assert_eq!(result.disposition, DeliberationDispositionIR::GoalReachable);
        assert_eq!(
            result.selected_plan.unwrap().mechanism_ids,
            ["LOCALIZE", "REPAIR"]
        );
        assert_eq!(result.external_action_execution_events, 0);
        assert_eq!(result.external_model_calls, 0);
    }

    #[test]
    fn public_api_reuses_stored_executable_knowledge_instead_of_prose() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        for (knowledge_id, mechanism_id, prerequisite, effect) in [
            ("K-LOCALIZE", "LOCALIZE", "FAILURE", "CAUSE"),
            ("K-REPAIR", "REPAIR", "CAUSE", "RESTORED"),
        ] {
            let response = api.execute_command(CognitiveApiCommandIR::InjectMechanismKnowledge {
                knowledge: MechanismKnowledgeIR {
                    schema: MECHANISM_KNOWLEDGE_SCHEMA.to_string(),
                    knowledge_id: knowledge_id.to_string(),
                    mechanism: CausalMechanismIR {
                        mechanism_id: mechanism_id.to_string(),
                        kind: MechanismKindIR::Inference,
                        prerequisites: vec![literal(prerequisite)],
                        effects: vec![literal(effect)],
                        observes: Vec::new(),
                        authority: ActionAuthorityIR::InternalInference,
                        authorized: true,
                        reversible: true,
                        recovery_reference: None,
                        cost_millis: 10,
                        risk_millis: 0,
                        provenance_refs: vec![format!("public-test:{knowledge_id}")],
                    },
                    semantic_tags: vec!["repair".to_string()],
                    validation_evidence_refs: vec![format!("public-test:{knowledge_id}:pass")],
                    confidence_millis: 950,
                },
            });
            assert!(response.ok);
        }
        let response = api.execute_command(CognitiveApiCommandIR::DeliberateWithKnowledge {
            request: DeliberationRequestIR {
                schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                request_id: "KNOWLEDGE-THINK-1".to_string(),
                subject: "reuse and compose executable repair knowledge".to_string(),
                evidence: vec![EvidenceIR {
                    evidence_id: "E-FAILURE".to_string(),
                    literal: literal("FAILURE"),
                    reliability_millis: 990,
                    source_ref: "public-test:failure".to_string(),
                }],
                mechanisms: Vec::new(),
                goals: vec![literal("RESTORED")],
                authority_envelope: AuthorityEnvelopeIR::default(),
                immutable_constraints: Vec::new(),
                max_depth: 4,
                beam_width: 8,
                max_hypotheses: 8,
                max_counterfactuals: 8,
            },
            query: MechanismQueryIR {
                semantic_tags: vec!["repair".to_string()],
                known_proposition_ids: vec!["FAILURE".to_string(), "CAUSE".to_string()],
                goal_proposition_ids: vec!["RESTORED".to_string()],
                max_results: 8,
            },
        });
        let Some(CognitiveApiPayloadIR::KnowledgeGroundedDeliberation(result)) = response.payload
        else {
            panic!("knowledge-grounded deliberation")
        };
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
    }

    #[test]
    fn public_api_induces_stores_and_composes_observed_mechanisms() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for request in [
            induction_request(
                "INDUCE-A-B",
                "K-A-B",
                "M-A-B",
                "SEED_FACT",
                "seed-fact",
                "MIDDLE_FACT",
                "middle-fact",
            ),
            induction_request(
                "INDUCE-B-C",
                "K-B-C",
                "M-B-C",
                "MIDDLE_FACT",
                "middle-fact",
                "FINAL_FACT",
                "final-fact",
            ),
        ] {
            let response =
                api.execute_command(CognitiveApiCommandIR::InduceAndInjectMechanismKnowledge {
                    request: Box::new(request),
                });
            let Some(CognitiveApiPayloadIR::MechanismInductionResponse(response)) =
                response.payload
            else {
                panic!("mechanism induction response")
            };
            assert_eq!(
                response.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert!(response.injection_receipt.unwrap().inserted);
            assert_eq!(response.induction.text_only_authority_events, 0);
        }

        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let result = api
            .deliberate_with_knowledge(
                &DeliberationRequestIR {
                    schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                    request_id: "DELIBERATE-INDUCED-CHAIN".to_string(),
                    subject: "compose mechanisms learned from controlled observations".to_string(),
                    evidence: vec![EvidenceIR {
                        evidence_id: "E-SEED".to_string(),
                        literal: literal("SEED_FACT"),
                        reliability_millis: 990,
                        source_ref: "public-test:seed".to_string(),
                    }],
                    mechanisms: Vec::new(),
                    goals: vec![literal("FINAL_FACT")],
                    authority_envelope: AuthorityEnvelopeIR::default(),
                    immutable_constraints: Vec::new(),
                    max_depth: 4,
                    beam_width: 8,
                    max_hypotheses: 8,
                    max_counterfactuals: 8,
                },
                &MechanismQueryIR {
                    semantic_tags: vec!["induced-chain".to_string()],
                    known_proposition_ids: vec!["SEED_FACT".to_string(), "MIDDLE_FACT".to_string()],
                    goal_proposition_ids: vec!["FINAL_FACT".to_string()],
                    max_results: 8,
                },
            )
            .unwrap();
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            result.deliberation.selected_plan.unwrap().mechanism_ids,
            ["M-A-B", "M-B-C"]
        );
    }

    #[test]
    fn public_api_learns_and_composes_raw_state_maps_without_a_manual_lexicon() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (request_id, cause, effect) in [
            ("RAW-CHAIN-A-B", "seed", "middle"),
            ("RAW-CHAIN-B-C", "middle", "final"),
        ] {
            let state = |effect_value| {
                [
                    (cause.to_string(), ObservedValueIR::Boolean(true)),
                    (effect.to_string(), ObservedValueIR::Boolean(effect_value)),
                ]
                .into_iter()
                .collect()
            };
            let observation = |suffix: &str, arm, after_effect| RawStateTransitionObservationIR {
                observation_id: format!("{request_id}-{suffix}"),
                arm,
                before: state(false),
                after: state(after_effect),
                reliability_millis: 950,
                evidence_refs: vec![format!("public-raw:{request_id}:{suffix}")],
            };
            let response = api.execute_command(
                CognitiveApiCommandIR::InduceAndInjectRawMechanismKnowledge {
                    request: Box::new(RawMechanismInductionRequestIR {
                        schema: RAW_MECHANISM_INDUCTION_REQUEST_SCHEMA.to_string(),
                        request_id: request_id.to_string(),
                        knowledge_id: format!("K-{request_id}"),
                        mechanism_id: format!("M-{request_id}"),
                        natural_language_statement: format!("{cause} causes {effect}"),
                        kind: MechanismKindIR::Inference,
                        authority: ActionAuthorityIR::InternalInference,
                        authorized: true,
                        reversible: true,
                        recovery_reference: None,
                        semantic_tags: vec!["raw-chain".to_string()],
                        observations: vec![
                            observation("P1", TransitionArmIR::AppliedSuccess, true),
                            observation("P2", TransitionArmIR::AppliedSuccess, true),
                            observation("C", TransitionArmIR::NoActionControl, false),
                        ],
                        minimum_positive_support: 2,
                        minimum_confidence_millis: 700,
                    }),
                },
            );
            let Some(CognitiveApiPayloadIR::RawMechanismInductionResponse(response)) =
                response.payload
            else {
                panic!("raw mechanism induction response")
            };
            assert_eq!(
                response.induction.induction.disposition,
                MechanismInductionDispositionIR::Compiled
            );
            assert!(response.injection_receipt.unwrap().inserted);
            assert_eq!(response.induction.explicit_proposition_lexicon_entries, 0);
        }

        let literal = |id: &str| LiteralIR {
            proposition_id: id.to_string(),
            value: true,
        };
        let result = api
            .deliberate_with_knowledge(
                &DeliberationRequestIR {
                    schema: DELIBERATION_REQUEST_SCHEMA.to_string(),
                    request_id: "DELIBERATE-RAW-CHAIN".to_string(),
                    subject: "compose knowledge induced from raw state maps".to_string(),
                    evidence: vec![EvidenceIR {
                        evidence_id: "E-RAW-SEED".to_string(),
                        literal: literal("STATE::SEED"),
                        reliability_millis: 990,
                        source_ref: "public-raw:seed".to_string(),
                    }],
                    mechanisms: Vec::new(),
                    goals: vec![literal("STATE::FINAL")],
                    authority_envelope: AuthorityEnvelopeIR::default(),
                    immutable_constraints: Vec::new(),
                    max_depth: 4,
                    beam_width: 8,
                    max_hypotheses: 8,
                    max_counterfactuals: 8,
                },
                &MechanismQueryIR {
                    semantic_tags: vec!["raw-chain".to_string()],
                    known_proposition_ids: vec![
                        "STATE::SEED".to_string(),
                        "STATE::MIDDLE".to_string(),
                    ],
                    goal_proposition_ids: vec!["STATE::FINAL".to_string()],
                    max_results: 8,
                },
            )
            .unwrap();
        assert_eq!(result.recalled_mechanisms.len(), 2);
        assert_eq!(
            result.deliberation.disposition,
            DeliberationDispositionIR::GoalReachable
        );
        assert_eq!(
            result.deliberation.selected_plan.unwrap().mechanism_ids,
            ["M-RAW-CHAIN-A-B", "M-RAW-CHAIN-B-C"]
        );
    }

    #[test]
    fn public_api_rejects_unbounded_context_before_planning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let error = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "REQ-OVERSIZED".to_string(),
                text: "plan a repair".to_string(),
                output_language: Some(LanguageCodeIR::English),
                context_tags: (0..65).map(|index| format!("tag-{index}")).collect(),
                max_plan_steps: 12,
            })
            .unwrap_err();
        assert_eq!(error, CognitiveApiError::InvalidRequest);
    }

    #[test]
    fn natural_language_knowledge_work_closes_lexeme_plan_analysis_and_text_output() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-FINANCE-1".to_string(),
                command: "이 재무제표를 분석하고 회계 등식도 확인해".to_string(),
                source: Some(KnowledgeSourceIR::Text {
                    text: "항목,2025,2026\n총자산,100,120\n총부채,40,50\n총자본,60,70".to_string(),
                    format: Some(SourceTextFormatIR::Csv),
                }),
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Markdown,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["finance".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            response.product.document.kind(),
            DocumentKindIR::FinancialStatement
        );
        assert!(response
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "financial_statement"));
        assert!(response
            .product
            .findings
            .iter()
            .any(|finding| finding.statement.contains("자산 = 부채 + 자본")));
        assert!(response
            .product
            .text_output
            .as_deref()
            .is_some_and(|text| text.contains("분석 결과")));
        assert!(response.plan.structurally_validated);
    }

    #[test]
    fn command_api_persists_verified_sense_weight_separately_from_encounters() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "LEX-1".to_string(),
                text: "표를 분석해".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: vec!["data".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        let activation = first
            .lexical_activations
            .iter()
            .find(|activation| activation.lexeme_id == "KO.TABLE")
            .unwrap();
        api.record_lexical_outcome(&LexicalOutcomeIR {
            activation_keys: vec![format!("{}/{}", activation.lexeme_id, activation.sense_id)],
            verified_success: true,
            evidence: vec!["human-confirmed table interpretation".to_string()],
        })
        .unwrap();
        assert_eq!(api.lexical_memory_statistics().verified_successes, 1);
        let snapshot = api.export_lexeme_snapshot();
        assert!(snapshot.entries.iter().any(|entry| {
            entry
                .usage
                .sense_usage
                .values()
                .any(|usage| usage.verified_success_count == 1)
        }));
    }

    #[test]
    fn injected_lexeme_can_drive_a_new_natural_language_revision_command() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.inject_lexeme(LexemeIR {
            schema: LEXEME_SCHEMA.to_string(),
            lexeme_id: "KO.CUSTOM.REFINE".to_string(),
            language: LanguageCodeIR::Korean,
            lemma: "정련해".to_string(),
            inflected_forms: vec!["정련".to_string()],
            part_of_speech: PartOfSpeechIR::Verb,
            grammatical_roles: Vec::new(),
            senses: vec![SenseIR {
                sense_id: "KO.CUSTOM.REFINE.S1".to_string(),
                canonical_concept: "revise".to_string(),
                gloss: "지정된 문서 구조를 다듬다".to_string(),
                semantic_tags: vec!["revision".to_string()],
                context_selectors: Vec::new(),
                relations: Vec::new(),
                intent_hint: Some(dockable_semantic_core::PlanIntentIR::Create),
                confidence_millis: 1_000,
            }],
            collocations: Vec::new(),
            domains: vec!["document".to_string()],
            source: "operator supplied terminology".to_string(),
            confidence_millis: 1_000,
            frequency_prior: 1,
        })
        .unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-CUSTOM-1".to_string(),
                command: "정련해\n제목: 검증 가능한 실행안".to_string(),
                source: Some(KnowledgeSourceIR::Structured {
                    document: Box::new(KnowledgeDocumentIR::PlanProposal(PlanProposalIR {
                        schema: PLAN_PROPOSAL_SCHEMA.to_string(),
                        document_id: "PLAN-1".to_string(),
                        title: "이전 계획".to_string(),
                        objective: "목표".to_string(),
                        tasks: Vec::new(),
                        risks: Vec::new(),
                        assumptions: Vec::new(),
                    })),
                }),
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Markdown,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["document".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(response.product.operation, KnowledgeWorkOperationIR::Revise);
        let KnowledgeDocumentIR::PlanProposal(plan) = response.product.document else {
            panic!("plan proposal")
        };
        assert_eq!(plan.title, "검증 가능한 실행안");
    }

    #[test]
    fn chart_command_writes_a_real_svg_file_and_returns_a_receipt() {
        let root =
            std::env::temp_dir().join(format!("b-core-cognitive-chart-{}", std::process::id()));
        let path = root.join("trend.svg");
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-CHART-1".to_string(),
                command: "이 데이터로 선형 차트를 작성해".to_string(),
                source: Some(KnowledgeSourceIR::Text {
                    text: "period,value\nQ1,10\nQ2,15\nQ3,25".to_string(),
                    format: Some(SourceTextFormatIR::Csv),
                }),
                document_kind: Some(DocumentKindIR::Chart),
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::File,
                    format: OutputFormatIR::Svg,
                    path: Some(path.to_string_lossy().to_string()),
                    overwrite: true,
                },
                context_tags: vec!["data".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert!(response.product.text_output.is_none());
        assert!(response.product.file_output.is_some());
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("polyline"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bilingual_business_genres_drive_the_cognitive_document_pipeline() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let korean = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-BUSINESS-KO".to_string(),
                command: "시장 표와 성장 차트를 포함한 사업계획서를 작성해".to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["business".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(korean.product.document.kind(), DocumentKindIR::BusinessPlan);
        assert_eq!(
            korean.product.design.theme,
            crate::knowledge_work::DocumentThemeIR::ExecutiveNavy
        );
        assert!(korean
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "business_plan"));
        assert!(korean
            .product
            .text_output
            .as_deref()
            .is_some_and(|html| html.contains("BUSINESS PLAN")));

        let english = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-BUSINESS-EN".to_string(),
                command: "Create a client business proposal with an executive chart".to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::English),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["proposal".to_string()],
                max_plan_steps: 12,
            })
            .unwrap();
        assert_eq!(
            english.product.document.kind(),
            DocumentKindIR::BusinessProposal
        );
        assert!(english
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "business_proposal"));
    }

    #[test]
    fn natural_language_manual_activates_guide_without_false_table_activation() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-GUIDE-KO".to_string(),
                command: "GPT 사용 설명서를 작성해. 모르는 기능은 확인 필요라고 표시해."
                    .to_string(),
                source: None,
                document_kind: None,
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["manual".to_string()],
                max_plan_steps: 16,
            })
            .unwrap();
        assert_eq!(response.product.document.kind(), DocumentKindIR::UserGuide);
        assert_eq!(
            response.product.deliberation.swarm.parent_reasoning_sha256,
            response.plan.plan_sha256
        );
        assert!(response.product.deliberation.causally_gated);
        assert!(response.product.deliberation.render_authorized);
        assert_eq!(response.product.deliberation.swarm.external_model_calls, 0);
        assert!(response
            .lexical_activations
            .iter()
            .any(|activation| activation.canonical_concept == "user_guide"));
        assert!(!response
            .lexical_activations
            .iter()
            .any(|activation| activation.lexeme_id == "KO.TABLE"));
    }

    #[test]
    fn professional_a4_manual_remains_an_authored_guide_and_binds_swarm_to_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let command =
            "GPT 사용 설명서를 전문 A4 문서로 작성해. 확인되지 않은 기능은 확인 필요라고 표시해.";
        let activations = api.lexical_memory.activate(
            command,
            &["manual".to_string(), "professional_document".to_string()],
        );
        assert_eq!(
            lexical_knowledge_operation(infer_operation(command), &activations),
            KnowledgeWorkOperationIR::Write
        );
        let response = api
            .process_knowledge_work(&KnowledgeWorkRequestIR {
                schema: KNOWLEDGE_WORK_REQUEST_SCHEMA.to_string(),
                request_id: "KW-GUIDE-A4".to_string(),
                command: command.to_string(),
                source: None,
                document_kind: Some(DocumentKindIR::UserGuide),
                output_language: Some(LanguageCodeIR::Korean),
                design: None,
                output: OutputDirectiveIR {
                    mode: OutputModeIR::Text,
                    format: OutputFormatIR::Html,
                    path: None,
                    overwrite: false,
                },
                context_tags: vec!["manual".to_string(), "professional_document".to_string()],
                max_plan_steps: 16,
            })
            .expect("reasoned professional guide");
        assert_eq!(
            response.product.deliberation.swarm.parent_reasoning_sha256,
            response.plan.plan_sha256
        );
    }

    fn conversation_request(
        conversation_id: &str,
        turn_index: u64,
        text: &str,
    ) -> crate::conversation::ConversationTurnRequestIR {
        crate::conversation::ConversationTurnRequestIR {
            schema: crate::conversation::CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: conversation_id.to_string(),
            turn_index,
            request_id: format!("{conversation_id}-{turn_index}"),
            modality: crate::conversation::ConversationInputModalityIR::Text,
            raw_text: text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        }
    }

    #[test]
    fn feedback_correction_preserves_contextual_explanation_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let setup = api
            .process_conversation_turn(&conversation_request(
                "CHAT-FEEDBACK-CORRECTION",
                1,
                "The Helix parser has kept failing since deployment. We cannot leave it like this.",
            ))
            .expect("context turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-FEEDBACK-CORRECTION",
                2,
                "아니, 고치지는 말고 왜 실패하는지만 설명해.",
            ))
            .expect("correction turn");
        assert!(
            response.grounded_response.as_ref().is_some_and(|grounded| {
                grounded.plan.intent == PlanIntentIR::Explain
                    && grounded
                        .understanding
                        .subject
                        .to_lowercase()
                        .contains("helix")
            }),
            "correction lost before GoalIR projection: setup_entities={:#?}, native={:#?}, center={:#?}",
            setup.conversation_state.active_typed_entities,
            response.native_language_circuit,
            response.pragmatic_interpretation.language_center,
        );
        let grounded = response
            .grounded_response
            .as_deref()
            .expect("grounded correction plan");
        let prohibited_repair = grounded
            .semantic_goal
            .events
            .iter()
            .find(|event| {
                event.intent == PlanIntentIR::Repair
                    && event.projection
                        == dockable_semantic_core::SemanticPlanProjectionIR::Prohibited
            })
            .expect("prohibited repair event remains auditable");
        let prohibited_targets = prohibited_repair
            .goal_subject_argument_ids
            .iter()
            .filter_map(|argument_id| {
                grounded
                    .semantic_goal
                    .arguments
                    .iter()
                    .find(|argument| &argument.argument_id == argument_id)
            })
            .map(|argument| argument.grounded_label.to_lowercase())
            .collect::<Vec<_>>();
        assert_eq!(prohibited_targets, vec!["helix"]);
        assert!(!response.output.text.contains("아니를"));
        assert!(response.output.text.contains("Helix에 대한 금지된 요청"));
    }

    #[test]
    fn resolved_reference_surface_is_the_single_downstream_parse_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (turn, text) in [
            "Aurora 서버 캐시를 조사해",
            "Beryl 백업 큐를 조사해",
            "Aurora 서버 이야기로 돌아가자",
        ]
        .into_iter()
        .enumerate()
        {
            api.process_conversation_turn(&conversation_request(
                "CHAT-RESOLVED-NATIVE-AUTHORITY",
                turn as u64 + 1,
                text,
            ))
            .expect("context turn");
        }
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESOLVED-NATIVE-AUTHORITY",
                4,
                "그걸 수리해",
            ))
            .expect("resolved repair turn");
        let grounded = response
            .grounded_response
            .as_deref()
            .expect("grounded plan");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "서버를 수리해"
        );
        assert_eq!(grounded.understanding.subject, "서버");
        assert!(response
            .native_language_circuit
            .selected_live_goals
            .iter()
            .all(|goal| !goal.subject.to_lowercase().contains("backup")
                && !goal.subject.contains("백업")));
        assert!(response.validate_against(&conversation_request(
            "CHAT-RESOLVED-NATIVE-AUTHORITY",
            4,
            "그걸 수리해",
        )));
    }

    #[test]
    fn required_reference_clarification_outranks_acknowledgement_candidates() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (turn, text) in [
            "Quinn says that the build failed",
            "Rowan says that the cache failed",
        ]
        .into_iter()
        .enumerate()
        {
            api.process_conversation_turn(&conversation_request(
                "CHAT-CLARIFICATION-OWNS-TURN",
                turn as u64 + 1,
                text,
            ))
            .expect("attribution turn");
        }
        let mut request = conversation_request(
            "CHAT-CLARIFICATION-OWNS-TURN",
            3,
            "She should revise the report",
        );
        request.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&request)
            .expect("ambiguous reference turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ClarificationRequest
        );
        assert!(response.output.text.to_lowercase().contains("refer"));
        assert!(response.validate_against(&request));
    }

    #[test]
    fn equivalent_module_goals_materialize_as_one_semantic_event() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut request = conversation_request(
            "CHAT-SINGLE-SEMANTIC-MATERIALIZATION",
            1,
            "Investigate the cache and repair the queue, but do not delete the log",
        );
        request.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&request)
            .expect("compound plan");
        let grounded = response
            .grounded_response
            .as_deref()
            .expect("grounded plan");
        assert_eq!(grounded.semantic_goal.selected_live_event_ids.len(), 2);
        assert_eq!(grounded.semantic_plan_bundle.plans.len(), 2);
        assert_eq!(
            response
                .natural_realization
                .coverage
                .obligations
                .iter()
                .filter(|obligation| {
                    obligation.kind
                        == crate::natural_realization::NaturalRealizationObligationKindIR::SelectedPlanEvent
                })
                .count(),
            2
        );
        assert!(response.validate_against(&request));
    }

    #[test]
    fn nested_possibility_does_not_become_an_implicit_delete_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-MODAL-NONGOAL",
                1,
                "We might need to delete the cache.",
            ))
            .expect("modal turn");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .root_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records[0]
                .signature
                .modal_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
    }

    #[test]
    fn active_action_failure_report_remains_a_report_not_a_diagnostic_request() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-ACTION-FAILURE-REPORT",
            1,
            "파서를 수리해",
        ))
        .expect("action request");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ACTION-FAILURE-REPORT",
                2,
                "수리는 실패했어",
            ))
            .expect("failure report");

        assert!(response.action_state_analysis.has_language_reports());
        assert!(response.grounded_response.is_none());
        assert!(
            response.output.text.contains("보고"),
            "unexpected action report realization: {}",
            response.output.text
        );
        let record = response
            .conversation_state
            .action_state_ledger
            .current_record()
            .expect("active action record");
        assert_eq!(
            record.reported_status,
            Some(ActionReportedStatusIR::FailureClaimed)
        );
        assert_eq!(
            record.execution_status,
            ActionExecutionStatusIR::NotObserved
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ActionState
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn english_success_report_query_preserves_the_original_action_lifecycle() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PLAN-RESULT-REPORT",
            1,
            "Run the compression.",
        ))
        .expect("action request");
        api.process_conversation_turn(&conversation_request(
            "CHAT-PLAN-RESULT-REPORT",
            2,
            "Compression succeeded.",
        ))
        .expect("language outcome report");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-RESULT-REPORT",
                3,
                "Is that only a success report, or is there a verified result too?",
            ))
            .expect("lifecycle query");

        assert_eq!(
            response.plan_result_boundary.query_focus,
            PlanResultQueryFocusIR::ReportedVersusResult
        );
        assert_eq!(response.plan_result_boundary.snapshots.len(), 1);
        assert_eq!(
            response.plan_result_boundary.snapshots[0].reported_status,
            Some(ActionReportedStatusIR::SuccessClaimed)
        );
        assert_eq!(
            response.plan_result_boundary.snapshots[0].execution_status,
            ActionExecutionStatusIR::NotObserved
        );
        assert_eq!(response.conversation_state.active_goals.len(), 1);
        assert_eq!(
            response.conversation_state.active_goals[0].canonical_predicate,
            "EXECUTE"
        );
        assert_eq!(
            response
                .conversation_state
                .action_state_ledger
                .records
                .len(),
            1
        );
        assert!(response.grounded_response.is_none());
    }

    #[test]
    fn explicit_result_target_outranks_a_rejected_report_reference() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-REJECTED-REPORT-RESULT",
            1,
            "Execute the Ocher migration",
        ))
        .expect("plan turn");
        api.process_conversation_turn(&conversation_request(
            "CHAT-REJECTED-REPORT-RESULT",
            2,
            "A teammate reported that it completed",
        ))
        .expect("report turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-REJECTED-REPORT-RESULT",
                3,
                "Ignore that report. Was the real Ocher result actually verified?",
            ))
            .expect("result query");
        assert!(
            response
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty(),
            "unexpected ambiguity: {:?}",
            response.reference_resolution.ambiguous_reference_surfaces
        );
        assert_ne!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(
            response.output.text.contains("No execution result")
                || response
                    .output
                    .text
                    .contains("검증된 실행 결과는 아직 없어"),
            "unexpected output: {}",
            response.output.text
        );
    }

    #[test]
    fn response_axis_corrections_do_not_replace_the_action_with_a_telling_plan() {
        for (conversation_id, query) in [
            (
                "CHAT-PLAN-RESULT-AXIS-KO",
                "앞으로 뭘 할지 말고 지금까지 실제로 일어난 것만 말해줘.",
            ),
            (
                "CHAT-PLAN-RESULT-AXIS-EN",
                "Do not tell me what you plan to do; tell me only what actually happened.",
            ),
            (
                "CHAT-PLAN-RESULT-AXIS-EN-RESULT",
                "Tell me only the actual execution result, not the plan",
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&conversation_request(
                conversation_id,
                1,
                "Run the tests.",
            ))
            .expect("action request");
            let response = api
                .process_conversation_turn(&conversation_request(conversation_id, 2, query))
                .expect("response-axis correction");

            assert_eq!(
                response.plan_result_boundary.query_focus,
                PlanResultQueryFocusIR::ExecutionVersusPlan
            );
            assert_eq!(response.conversation_state.active_goals.len(), 1);
            assert_eq!(
                response.conversation_state.active_goals[0].canonical_predicate,
                "EXECUTE"
            );
            assert_eq!(
                response
                    .conversation_state
                    .action_state_ledger
                    .records
                    .len(),
                1
            );
            assert_eq!(
                response.conversation_state.action_state_ledger.records[0].plan_status,
                crate::action_state::ActionPlanStatusIR::Active
            );
            assert!(response.grounded_response.is_none());
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanResultStatus,
                "response-axis query must realize the lifecycle boundary: {response:#?}"
            );
        }
    }

    #[test]
    fn explanatory_goal_correction_is_not_consumed_as_a_lifecycle_status_query() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PLAN-RESULT-EXPLANATION",
            1,
            "Inspect the log.",
        ))
        .expect("initial investigation request");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-RESULT-EXPLANATION",
                2,
                "No, I am not asking you to inspect it; explain why it failed.",
            ))
            .expect("explanatory goal correction");

        assert_eq!(response.conversation_state.active_goals.len(), 1);
        assert_eq!(
            response.conversation_state.active_goals[0].canonical_predicate, "EXPLAIN",
            "{response:#?}"
        );
        assert!(response.grounded_response.is_some());
        assert!(response.output.grounded_plan_sha256.is_some());

        let mut korean_api = CognitiveApi::new_embedded().unwrap();
        korean_api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-RESULT-EXPLANATION-KO",
                1,
                "로그를 검사해.",
            ))
            .expect("initial Korean investigation request");
        let korean = korean_api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-RESULT-EXPLANATION-KO",
                2,
                "아니, 로그를 검사하라는 게 아니라 왜 실패했는지 설명해달라는 거야.",
            ))
            .expect("Korean explanatory goal correction");
        assert_eq!(
            korean.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{korean:#?}"
        );
        assert!(!korean
            .pragmatic_interpretation
            .language_center_goal_projection
            .as_ref()
            .expect("goal projection")
            .decisions
            .iter()
            .any(|decision| {
                decision.source
                    == crate::language_center::LanguageCenterGoalDecisionSourceIR::UtteranceIntentGraph
                    && decision.effect
                        == crate::language_center::LanguageCenterGoalEffectIR::PreserveConstraint
            }));
        assert!(korean.validate_against(&conversation_request(
            "CHAT-PLAN-RESULT-EXPLANATION-KO",
            2,
            "아니, 로그를 검사하라는 게 아니라 왜 실패했는지 설명해달라는 거야.",
        )));
    }

    #[test]
    fn punctuation_only_fragments_are_not_epistemic_propositions() {
        assert!(!has_semantic_proposition_content("’"));
        assert!(!has_semantic_proposition_content("...?!"));
        assert!(has_semantic_proposition_content("verified result"));

        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PLAN-RESULT-WITHDRAWAL",
            1,
            "Run the deployment.",
        ))
        .expect("action request");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-RESULT-WITHDRAWAL",
                2,
                "Do not do that task.",
            ))
            .expect("goal withdrawal must not create an empty epistemic fingerprint");
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(
            response.conversation_state.action_state_ledger.records[0].plan_status,
            crate::action_state::ActionPlanStatusIR::Withdrawn
        );
    }

    #[test]
    fn rejected_definition_does_not_make_unknown_korean_predicate_executable() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let rejected = api
            .process_conversation_turn(&conversation_request(
                "CHAT-REJECTED-DEFINITION",
                1,
                "\"소라\"는 삭제하라는 뜻이야?",
            ))
            .expect("questioned definition");
        assert_eq!(
            rejected.definition_grounding.disposition,
            DefinitionGroundingDispositionIR::NonAssertedRejected
        );

        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-REJECTED-DEFINITION",
                2,
                "캐시를 소라해줘.",
            ))
            .expect("unknown opaque predicate");
        assert!(
            response.grounded_response.is_none(),
            "unknown predicate must not fall back to a generic plan: speech_act={:?}, frames={:#?}, intent_graph={:#?}",
            response.pragmatic_interpretation.speech_act,
            response
                .pragmatic_interpretation
                .compositional_analysis
                .frames,
            response.pragmatic_interpretation.pragmatic_intent_graph
        );
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn definition_grounding_uses_typed_generation_without_internal_operator_surface() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let added = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DEFINITION-TYPED-GENERATION",
                1,
                "\"파온\"은 수리하라는 뜻이야.",
            ))
            .expect("new definition");
        assert_eq!(
            added.natural_realization.response_act,
            NaturalResponseActIR::DefinitionGrounding
        );
        assert_eq!(
            added.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(added.natural_realization.generation_traces.len(), 1);
        let trace = &added.natural_realization.generation_traces[0];
        assert!(trace.validate());
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_DEFINITION_BIND_ADDED"));
        assert!(added.output.text.contains("수리"));
        assert!(!added.output.text.contains("REPAIR"));
        assert!(!trace.semantic_authority);
        assert!(!trace.language_can_execute);

        let confirmed = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DEFINITION-TYPED-GENERATION",
                2,
                "\"파온\"은 수리하라는 뜻이야.",
            ))
            .expect("confirmed definition");
        assert!(!confirmed.definition_grounding.lexical_store_changed);
        assert!(confirmed.natural_realization.generation_traces[0]
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_DEFINITION_BIND_CONFIRMED"));
    }

    #[test]
    fn polite_modal_question_projects_the_action_but_not_modal_truth() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-POLITE-REQUEST",
                1,
                "Could you delete the cache?",
            ))
            .expect("polite request");
        let goal = response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .expect("indirect request becomes a typed goal");
        assert_eq!(goal.intent, dockable_semantic_core::PlanIntentIR::Execute);
        assert!(goal.external_execution_authorized);
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .illocution,
            crate::modality::ModalIllocutionIR::PoliteRequest
        );
        assert!(
            !response
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .dialogue_truth_established
        );
    }

    #[test]
    fn conditional_directive_is_not_a_current_action_or_satisfied_condition() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CONDITIONAL-AUTHORITY",
                1,
                "If the tests pass, deploy the service.",
            ))
            .expect("conditional directive");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        let conditional = &response
            .pragmatic_interpretation
            .compositional_analysis
            .modal_scope_graph
            .conditionals[0];
        assert!(!conditional.condition_satisfied);
        assert!(!conditional.external_execution_authorized);
        assert!(!conditional.reverse_inference_authorized);
        assert_eq!(response.conditional_guard_evaluations.len(), 1);
        assert_eq!(
            response.conditional_guard_evaluations[0].status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ConditionalGuard
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0]
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_GUARD_UNRESOLVED"));
        assert!(!response.natural_realization.semantic_authority);
        assert!(!response.natural_realization.language_can_execute);
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn later_actual_evidence_supports_guard_for_deliberation_only() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-SUPPORT",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-SUPPORT",
                2,
                "The tests passed.",
            ))
            .expect("guard evidence");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::SupportedByDialogueEvidence
        );
        assert!(evaluation.deliberation_eligible);
        assert!(!evaluation.dialogue_truth_established);
        assert!(!evaluation.external_execution_authorized);
        assert!(!evaluation.reverse_inference_authorized);
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert!(response.natural_realization.generation_traces[0]
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_GUARD_SUPPORTED"));
        assert!(response.output.text.contains("자동으로 실행되지는 않아"));
        assert!(!response.output.text.contains("C_GUARD_"));
        assert!(response.conversation_state.active_goals.is_empty());
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn possible_evidence_and_observed_consequent_do_not_activate_guard() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-NO-REVERSE",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let possible = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-NO-REVERSE",
                2,
                "The tests might pass.",
            ))
            .expect("possible evidence");
        assert!(possible.conditional_guard_evaluations.is_empty());
        assert_eq!(
            possible.conversation_state.conditional_guard_store.guards[0].status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        let consequent = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-NO-REVERSE",
                3,
                "The service deployed.",
            ))
            .expect("consequent observation");
        let guard = &consequent.conversation_state.conditional_guard_store.guards[0];
        assert_eq!(
            guard.status,
            crate::conditional_guard::GuardStatusIR::Unresolved
        );
        assert!(!guard.reverse_inference_authorized);
        assert!(!guard.deliberation_eligible);
    }

    #[test]
    fn conflicting_sources_keep_guard_contested() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-CONFLICT",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-CONFLICT",
            2,
            "Alice says the tests passed.",
        ))
        .expect("supporting source");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-CONFLICT",
                3,
                "Bob says the tests failed.",
            ))
            .expect("contradicting source");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::Contested
        );
        assert_eq!(evaluation.evidence.len(), 2);
        assert!(!evaluation.deliberation_eligible);
        assert!(!evaluation.external_execution_authorized);
    }

    #[test]
    fn korean_unless_guard_uses_negated_antecedent_without_auto_action() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-KOREAN",
            1,
            "테스트가 통과하지 않으면 배포를 멈춰.",
        ))
        .expect("Korean unless guard");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-KOREAN",
                2,
                "테스트가 실패했다.",
            ))
            .expect("Korean guard evidence");
        let evaluation = &response.conditional_guard_evaluations[0];
        assert_eq!(
            evaluation.status,
            crate::conditional_guard::GuardStatusIR::SupportedByDialogueEvidence
        );
        assert!(evaluation.realized_text.contains("자동 실행 권한은 없어"));
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn recognized_question_does_not_mutate_existing_guard_state() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARD-QUESTION",
            1,
            "If the tests pass, deploy the service.",
        ))
        .expect("guard declaration");
        let supported = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-QUESTION",
                2,
                "The tests passed.",
            ))
            .expect("guard support");
        let before = supported.conversation_state.conditional_guard_store.clone();
        let question = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARD-QUESTION",
                3,
                "Is it true that the tests passed?",
            ))
            .expect("recognized discourse question");
        assert!(question.discourse_answer.is_some());
        assert!(question.conditional_guard_evaluations.is_empty());
        assert_eq!(question.conversation_state.conditional_guard_store, before);
    }

    #[test]
    fn discourse_question_answers_from_prior_attribution_without_making_a_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-SOURCE",
            1,
            "Alice says that the server is down.",
        ))
        .expect("attributed statement");
        let mut question = conversation_request("CHAT-QA-SOURCE", 2, "What did Alice say?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("discourse answer");
        let answer = response
            .discourse_answer
            .as_ref()
            .expect("typed discourse answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        );
        assert_eq!(answer.evidence[0].source_actor, "alice");
        assert!(response.grounded_response.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(response.pragmatic_state.task_frames.is_empty());
        assert_eq!(response.output.unsupported_freeform_claims, 0);
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseAnswer
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        let trace = &response.natural_realization.generation_traces[0];
        assert!(trace.validate());
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_DIALOGUE_ANSWER_RECORD"));
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_DIALOGUE_ANSWER_NOT_FACT"));
        assert!(!trace.semantic_authority);
        assert!(!trace.language_can_execute);
    }

    #[test]
    fn modal_status_question_preserves_possible_world_and_record_count() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-MODAL",
            1,
            "Alice believes that the server might be down.",
        ))
        .expect("modal belief");
        let mut question = conversation_request(
            "CHAT-QA-MODAL",
            2,
            "Is the server merely possible or actual?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("modal answer");
        let answer = response.discourse_answer.expect("typed modal answer");
        assert_eq!(answer.evidence.len(), 1);
        assert_eq!(
            answer.evidence[0].modal_world,
            crate::modality::ModalWorldIR::EpistemicPossible
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(!answer.dialogue_truth_established);
    }

    #[test]
    fn conflicting_sources_are_answered_without_selecting_a_truth_winner() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CONFLICT",
            1,
            "Alice says that the server is up.",
        ))
        .expect("Alice record");
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CONFLICT",
            2,
            "Bob says that the server is down.",
        ))
        .expect("Bob record");
        let mut question = conversation_request(
            "CHAT-QA-CONFLICT",
            3,
            "Are Alice and Bob in conflict about the server?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("conflict answer");
        let answer = response.discourse_answer.expect("typed conflict answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::ConflictingDialogueRecords
        );
        assert_eq!(answer.evidence.len(), 2);
        assert!(answer.realized_text.contains("No source has been selected"));
    }

    #[test]
    fn casual_conflict_question_uses_all_current_opposing_source_records() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CASUAL-CONFLICT",
            1,
            "Mina says the Lotus cache is stale.",
        ))
        .expect("Mina record");
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-CASUAL-CONFLICT",
            2,
            "Joon says the Lotus cache is not stale.",
        ))
        .expect("Joon record");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-CASUAL-CONFLICT",
                3,
                "둘이 반대로 말하는데, 어느 쪽이 사실이야?",
            ))
            .expect("conflict answer");
        let answer = response.discourse_answer.expect("typed conflict answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::ConflictingDialogueRecords
        );
        assert_eq!(answer.evidence.len(), 2);
        assert!(answer.realized_text.contains("Mina") || answer.realized_text.contains("mina"));
        assert!(answer.realized_text.contains("Joon") || answer.realized_text.contains("joon"));
    }

    #[test]
    fn comparison_keeps_parallel_focus_and_generic_demonstrative_ambiguous() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PARALLEL-FOCUS",
            1,
            "Compare the Aster cache and the Birch queue.",
        ))
        .expect("comparison turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PARALLEL-FOCUS",
                2,
                "그거 왜 느려?",
            ))
            .expect("clarification turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert_eq!(
            response.reference_resolution.ambiguous_reference_surfaces,
            vec!["그거".to_string()]
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ClarificationRequest
        );
        assert!(response.output.text.contains("어느 대상"));
    }

    #[test]
    fn concise_feedback_reuses_the_prior_explanation_subject() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-FEEDBACK-SUBJECT",
            1,
            "Explain how the Cedar scheduler decides priority.",
        ))
        .expect("explanation plan");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-FEEDBACK-SUBJECT",
                2,
                "너무 길어. 핵심만 다시 설명해.",
            ))
            .expect("concise explanation plan");
        let goal = response
            .conversation_state
            .active_goals
            .first()
            .expect("context-bound explanation goal");
        assert_eq!(goal.intent, PlanIntentIR::Explain);
        assert!(
            goal.subject.contains("cedar scheduler"),
            "goal={goal:#?} native={:#?}",
            response.native_language_circuit
        );
        assert!(!goal.external_execution_authorized);
        assert!(response.output.text.to_lowercase().contains("cedar"));
    }

    #[test]
    fn factive_attribution_answer_remains_presented_as_known_not_fact() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-FACTIVE",
            1,
            "Alice knows that the server is down.",
        ))
        .expect("factive attribution");
        let mut question = conversation_request("CHAT-QA-FACTIVE", 2, "What does Alice know?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("factive answer");
        let answer = response.discourse_answer.expect("typed factive answer");
        assert_eq!(
            answer.evidence[0].epistemic_status,
            crate::attribution::EpistemicStatusIR::PresentedAsKnown
        );
        assert!(!answer.evidence[0].dialogue_truth_established);
        assert!(answer.realized_text.contains("not established facts"));
    }

    #[test]
    fn presuppositional_why_question_abstains_and_does_not_add_a_belief() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-PRESUPPOSITION",
            1,
            "Alice believes that the server might fail.",
        ))
        .expect("possible failure belief");
        let mut question =
            conversation_request("CHAT-QA-PRESUPPOSITION", 2, "Why did the server fail?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("presupposition response");
        let answer = response
            .discourse_answer
            .expect("typed presupposition response");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::PresuppositionUnverified
        );
        assert_eq!(
            response.conversation_state.epistemic_ledger.records.len(),
            1
        );
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn korean_source_question_uses_the_same_typed_answer_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-KOREAN",
            1,
            "민수는 서버가 다운됐다고 말했다.",
        ))
        .expect("Korean attribution");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-KOREAN",
                2,
                "민수는 뭐라고 말했어?",
            ))
            .expect("Korean source answer");
        let answer = response.discourse_answer.expect("typed Korean answer");
        assert_eq!(answer.language, LanguageCodeIR::Korean);
        assert_eq!(answer.evidence.len(), 1);
        assert!(answer.realized_text.contains("사실 확정은 아니야"));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn actuality_question_treats_expletive_it_and_complementizer_that_as_syntax() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QA-SYNTACTIC-REFERENCE",
            1,
            "Alice says that the server is down.",
        ))
        .expect("attributed statement");
        let mut question = conversation_request(
            "CHAT-QA-SYNTACTIC-REFERENCE",
            2,
            "Is it true that the server is down?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("actuality answer");
        let answer = response.discourse_answer.expect("typed actuality answer");
        assert_eq!(
            answer.disposition,
            crate::discourse_qa::DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
        );
        assert_eq!(answer.evidence.len(), 1);
        assert_eq!(response.conversation_state.unresolved_reference_count, 0);
        assert!(response.conversation_state.active_goals.is_empty());
    }

    #[test]
    fn proxy_observation_cannot_turn_a_pending_gate_into_a_continuation_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let gate = api
            .process_conversation_turn(&conversation_request(
                "CHAT-R20-GATE",
                1,
                "통합으로 실제 커버리지가 늘어야 계속할 수 있다.",
            ))
            .expect("gate");
        assert!(gate.pragmatic_state.pending_continuation_gate.is_some());

        let proxy = api
            .process_conversation_turn(&conversation_request("CHAT-R20-GATE", 2, "점수만 올랐어"))
            .expect("proxy");
        assert!(proxy.grounded_response.is_none());
        assert!(proxy.output.grounded_plan_sha256.is_none());
        assert!(proxy.output.text.contains("대리 지표 변화는 기록했지만"));
        assert_eq!(
            proxy.natural_realization.response_act,
            NaturalResponseActIR::ContinuationGate
        );
        assert_eq!(
            proxy.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(proxy.natural_realization.generation_traces.len(), 1);
        assert!(proxy.natural_realization.generation_traces[0].validate());
        assert_eq!(proxy.natural_realization.stage_overwrite_count, 0);

        let decision = api
            .process_conversation_turn(&conversation_request(
                "CHAT-R20-GATE",
                3,
                "그러면 계속해도 돼?",
            ))
            .expect("decision");
        assert!(decision.grounded_response.is_none());
        assert!(decision.output.grounded_plan_sha256.is_none());
        assert!(decision.output.text.contains("커버리지"));
        assert!(decision.output.text.contains("대리 지표만으로"));
        assert_eq!(
            decision.natural_realization.response_act,
            NaturalResponseActIR::ContinuationGate
        );
        assert_eq!(
            decision.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(decision.natural_realization.generation_traces.len(), 1);
        assert!(decision.natural_realization.generation_traces[0].validate());
        assert_eq!(decision.natural_realization.stage_overwrite_count, 0);
        assert_eq!(
            decision
                .pragmatic_state
                .pending_continuation_gate
                .as_ref()
                .map(|gate| gate.status),
            Some(crate::pragmatic_memory::PendingGateStatusIR::AwaitingEvidence)
        );
    }

    #[test]
    fn attribution_attitude_projects_desired_and_predicted_worlds() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let wanted = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-ATTITUDE-WORLD",
                1,
                "Leo wants the team to retry.",
            ))
            .expect("desired attribution");
        assert_eq!(
            wanted.conversation_state.epistemic_ledger.records[0]
                .signature
                .modal_world,
            crate::modality::ModalWorldIR::Desired
        );
        let expected = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QA-ATTITUDE-WORLD",
                2,
                "Uma expects that the rollout will finish.",
            ))
            .expect("expected attribution");
        assert!(
            expected
                .conversation_state
                .epistemic_ledger
                .records
                .iter()
                .any(|record| record.signature.modal_world
                    == crate::modality::ModalWorldIR::Predicted)
        );
    }

    #[test]
    fn temporal_question_uses_event_time_not_report_turn() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let statement = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-TIME",
                1,
                "The backup completed yesterday.",
            ))
            .expect("temporal statement");
        assert_eq!(statement.conversation_state.temporal_graph.events.len(), 1);
        assert_eq!(
            statement.conversation_state.temporal_graph.events[0]
                .event_time
                .as_ref()
                .and_then(|time| time.relative_day_offset),
            Some(-1)
        );
        let mut question =
            conversation_request("CHAT-TEMPORAL-TIME", 2, "When did the backup complete?");
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("temporal answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(
            answer.disposition,
            crate::temporal::TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
        );
        assert!(answer.realized_text.contains("DAY_OFFSET:-1"));
        assert!(response.grounded_response.is_none());
        assert!(response.conversation_state.active_goals.is_empty());
        assert_eq!(response.conversation_state.temporal_graph.events.len(), 1);
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::TemporalAnswer
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        let trace = &response.natural_realization.generation_traces[0];
        assert!(trace.validate());
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_TEMPORAL_ANSWER_TIME"));
        assert!(trace
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY"));
        assert!(!trace.semantic_authority);
        assert!(!trace.language_can_execute);
        assert_eq!(trace.verification.unsupported_claims, 0);
        assert!(response.output.text.contains("DAY_OFFSET:-1"));
    }

    #[test]
    fn temporal_before_question_cites_relation_without_asserting_truth() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-BEFORE",
            1,
            "The backup completed before the deploy started.",
        ))
        .expect("temporal relation");
        let mut question = conversation_request(
            "CHAT-TEMPORAL-BEFORE",
            2,
            "What happened before the deploy started?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("temporal relation answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(answer.relation_evidence.len(), 1);
        assert!(answer
            .event_evidence
            .iter()
            .any(|event| event.surface.contains("backup")));
        assert!(!answer.dialogue_truth_established);
        assert!(!answer.external_execution_authorized);
    }

    #[test]
    fn temporal_deictic_chain_supports_transitive_multi_turn_reasoning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-CHAIN",
            1,
            "The backup completed.",
        ))
        .expect("first event");
        let second = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-CHAIN",
                2,
                "After that, the deploy started.",
            ))
            .expect("second event");
        assert_eq!(second.conversation_state.unresolved_reference_count, 0);
        let third = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-CHAIN",
                3,
                "After that, the monitor failed.",
            ))
            .expect("third event");
        assert_eq!(third.conversation_state.temporal_graph.events.len(), 3);
        assert_eq!(third.conversation_state.temporal_graph.relations.len(), 2);
        let mut question = conversation_request(
            "CHAT-TEMPORAL-CHAIN",
            4,
            "Did the backup complete before the monitor failed?",
        );
        question.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&question)
            .expect("transitive answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(
            answer.disposition,
            crate::temporal::TemporalAnswerDispositionIR::AnsweredByTransitivePath
        );
        assert_eq!(answer.relation_evidence.len(), 2);
    }

    #[test]
    fn korean_temporal_relation_uses_same_graph_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TEMPORAL-KOREAN",
            1,
            "배포가 시작되기 전에 백업이 완료됐다.",
        ))
        .expect("Korean temporal relation");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TEMPORAL-KOREAN",
                2,
                "배포가 시작되기 전에 무슨 일이 있었어?",
            ))
            .expect("Korean temporal answer");
        let answer = response.temporal_answer.expect("typed temporal answer");
        assert_eq!(answer.language, LanguageCodeIR::Korean);
        assert_eq!(answer.relation_evidence.len(), 1);
        assert!(answer
            .event_evidence
            .iter()
            .any(|event| event.surface.contains("백업")));
    }

    #[test]
    fn conversational_frontend_repairs_surface_noise_before_semantic_planning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NOISE",
                1,
                "음... 파일 오류를 고처줘",
            ))
            .expect("conversation turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert_eq!(response.normalization.semantic_text, "파일 오류를 고쳐줘");
        let grounded = response.grounded_response.expect("grounded response");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
        assert_eq!(
            grounded.understanding.original_text,
            "음... 파일 오류를 고처줘"
        );
        assert_eq!(
            response.output.grounded_plan_sha256,
            Some(grounded.plan.plan_sha256.clone())
        );
        assert!(response.output.text.starts_with("알겠어."));
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn conversational_reference_uses_prior_turn_without_rewriting_the_world_model() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request("CHAT-REF", 1, "파일을 열어"))
            .expect("first turn");
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-REF", 2, "음... 그걸 확인해"))
            .expect("second turn");
        assert_eq!(response.reference_resolution.resolved_reference_count, 1);
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "파일을 확인해"
        );
        assert_eq!(response.conversation_state.completed_turns, 2);
        assert_eq!(response.conversation_state.active_referents.len(), 1);
        assert_eq!(
            response.conversation_state.active_referents[0].canonical_concept,
            "C_OBJECT_FILE"
        );
    }

    #[test]
    fn result_explanation_keeps_prior_goal_identity() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-IDENTITY",
                1,
                "CCTV 오류를 진단해",
            ))
            .expect("first turn");
        api.process_conversation_turn(&conversation_request("CHAT-RESULT-IDENTITY", 2, "고마워"))
            .expect("social turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-IDENTITY",
                3,
                "그 결과를 설명해",
            ))
            .expect("result query");
        let before = first
            .conversation_state
            .active_goals
            .iter()
            .map(|goal| goal.goal_id.clone())
            .collect::<Vec<_>>();
        let after = response
            .conversation_state
            .active_goals
            .iter()
            .map(|goal| goal.goal_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(after, before, "response={response:#?}");
        assert!(response.grounded_response.is_none());
    }

    #[test]
    fn filler_only_turn_is_acknowledged_without_a_fake_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-HOLD", 1, "음..."))
            .expect("hold turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::HoldFloor
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
        assert!(response.output.text.contains("천천히"));
        assert!(response.output.text.contains("듣고 있어"));
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::HoldFloor
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn greeting_receives_a_social_response_without_planning() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-HELLO", 1, "안녕"))
            .expect("greeting");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::BackchannelOnly
        );
        assert!(response.grounded_response.is_none());
        assert_eq!(response.output.text, "안녕! 무엇을 도와줄까?");
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::SocialBackchannel
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn common_action_language_selects_execute_intent() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-OPEN", 1, "파일을 열어"))
            .expect("execute request");
        assert_eq!(
            response
                .grounded_response
                .expect("grounded response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Execute
        );
    }

    #[test]
    fn cross_turn_parallel_ellipsis_reuses_the_typed_action_with_a_new_subject() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PARALLEL-ELLIPSIS",
            1,
            "파일을 확인해",
        ))
        .expect("first turn");
        let response = api
            .process_conversation_turn(&conversation_request("CHAT-PARALLEL-ELLIPSIS", 2, "문서도"))
            .expect("elliptical second turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "문서를 확인해"
        );
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("inherited action is grounded again");
        assert_eq!(selected.subject, "문서");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::EllipticalAction
        );
    }

    #[test]
    fn cross_turn_discourse_program_replays_all_ordered_steps_for_an_explicit_new_subject() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DISCOURSE-PROGRAM",
                1,
                "캐시를 확인하고 수리해",
            ))
            .expect("source program");
        assert_eq!(first.conversation_state.active_discourse_programs.len(), 1);
        assert!(first.conversation_state.active_discourse_programs[0].replayable);
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DISCOURSE-PROGRAM",
                2,
                "인덱스도 똑같이 해",
            ))
            .expect("program instantiation");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidates();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].intent, PlanIntentIR::Investigate);
        assert_eq!(selected[1].intent, PlanIntentIR::Repair);
        assert!(selected
            .iter()
            .all(|candidate| candidate.subject == "인덱스"));
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            DiscourseBindingKindIR::DiscourseProgramInstantiation
        );
    }

    #[test]
    fn explicit_topic_restoration_selects_the_matching_prior_discourse_program() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TOPICAL-PROGRAM",
            1,
            "캐시를 확인하고 수리해",
        ))
        .expect("cache program");
        api.process_conversation_turn(&conversation_request(
            "CHAT-TOPICAL-PROGRAM",
            2,
            "파일을 읽고 저장해",
        ))
        .expect("file program");
        api.process_conversation_turn(&conversation_request(
            "CHAT-TOPICAL-PROGRAM",
            3,
            "캐시로 돌아가자",
        ))
        .expect("restore cache topic");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TOPICAL-PROGRAM",
                4,
                "인덱스도 똑같이 해",
            ))
            .expect("topical program instantiation");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidates();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].intent, PlanIntentIR::Investigate);
        assert_eq!(selected[1].intent, PlanIntentIR::Repair);
    }

    #[test]
    fn discourse_program_hash_tampering_is_rejected() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROGRAM-TAMPER",
                1,
                "파일을 읽고 저장해",
            ))
            .expect("source program");
        let mut state = response.conversation_state;
        assert!(crate::conversation::validate_conversation_state(&state).is_ok());
        state.active_discourse_programs[0].shared_subject = "서버".to_string();
        assert!(crate::conversation::validate_conversation_state(&state).is_err());
    }

    #[test]
    fn withdrawn_discourse_program_cannot_be_reactivated_by_parallel_ellipsis() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PROGRAM-WITHDRAWAL",
            1,
            "캐시를 확인하고 수리해",
        ))
        .expect("source program");
        let withdrawn = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROGRAM-WITHDRAWAL",
                2,
                "그 작업은 취소해",
            ))
            .expect("withdraw program");
        assert!(withdrawn
            .conversation_state
            .active_discourse_programs
            .is_empty());
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROGRAM-WITHDRAWAL",
                3,
                "인덱스도 똑같이 해",
            ))
            .expect("blocked replay");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired,
            "native_goal={:?} native_selected={:?} native_unresolved={:?} compositional={:?} active_goals={:?} active_programs={:?} grounded={}",
            response.native_language_circuit.response_goal,
            response.native_language_circuit.selected_live_goals,
            response.native_language_circuit.unresolved,
            response
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidates(),
            response.conversation_state.active_goals,
            response.conversation_state.active_discourse_programs,
            response.grounded_response.is_some(),
        );
        assert!(response.grounded_response.is_none());
    }

    #[test]
    fn same_subject_negated_program_is_recorded_as_nonreplayable() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROGRAM-NEGATED-SAME-SUBJECT",
                1,
                "Inspect and do not delete the backup.",
            ))
            .expect("negated source program");
        assert_eq!(
            response.conversation_state.active_discourse_programs.len(),
            1,
            "analysis={:#?}",
            response.pragmatic_interpretation.compositional_analysis
        );
        assert!(!response.conversation_state.active_discourse_programs[0].replayable);
    }

    #[test]
    fn mixed_immediate_and_conditional_request_creates_a_guarded_discourse_program() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-PROGRAM",
                1,
                "캐시를 검사하고 캐시에 문제가 있으면 수리해",
            ))
            .expect("guarded source");
        assert_eq!(source.conversation_state.active_goals.len(), 1);
        assert_eq!(
            source.conversation_state.active_goals[0].intent,
            PlanIntentIR::Investigate
        );
        assert_eq!(
            source.conversation_state.deferred_action_commitments.len(),
            1
        );
        assert_eq!(
            source.conversation_state.deferred_action_commitments[0].status,
            DeferredCommitmentStatusIR::ConditionPending
        );
        let program = &source.conversation_state.active_discourse_programs[0];
        assert!(program.replayable);
        assert_eq!(program.guarded_step_count, 1);
        let guard = program.steps[1].guard.as_ref().expect("typed guard");
        assert_eq!(guard.canonical_condition_predicate, "PROBLEM_PRESENT");
        assert_eq!(
            guard.deferred_commitment_id,
            source.conversation_state.deferred_action_commitments[0].commitment_id
        );
        assert_eq!(
            guard.condition_sha256,
            source.conversation_state.deferred_action_commitments[0].condition_sha256
        );
        assert!(guard.requires_verified_evidence);
        assert!(!guard.semantic_authority);
        assert!(!guard.external_execution_authorized);

        let rebound = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-PROGRAM",
                2,
                "인덱스도 같은 절차로 해",
            ))
            .expect("guarded rebound");
        let selected = rebound
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidates();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].intent, PlanIntentIR::Investigate);
        assert_eq!(selected[0].subject, "인덱스");
        let pending = rebound
            .conversation_state
            .deferred_action_commitments
            .iter()
            .find(|commitment| commitment.introduced_turn == 2)
            .expect("rebound pending action");
        assert_eq!(pending.action.intent, PlanIntentIR::Repair);
        assert_eq!(pending.action.subject, "인덱스");
        assert_eq!(pending.status, DeferredCommitmentStatusIR::ConditionPending);
        assert!(pending.activated_goal_id.is_none());
        assert!(rebound
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| {
                binding
                    .evidence
                    .iter()
                    .any(|item| item == "GUARDED_DISCOURSE_PROGRAM_INSTANTIATION:true")
            }));
    }

    #[test]
    fn verified_evidence_activates_only_the_linked_guarded_step() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARDED-EVIDENCE-LINK",
            1,
            "캐시를 검사하고 캐시에 문제가 있으면 수리해",
        ))
        .expect("guarded source");
        let rebound = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-EVIDENCE-LINK",
                2,
                "인덱스도 같은 절차로 해",
            ))
            .expect("guarded rebound");
        let pending = rebound
            .conversation_state
            .deferred_action_commitments
            .iter()
            .find(|commitment| commitment.introduced_turn == 2)
            .expect("rebound commitment")
            .clone();
        let rebound_program = rebound
            .conversation_state
            .active_discourse_programs
            .iter()
            .find(|program| program.introduced_turn == 2)
            .expect("rebound program");
        assert_eq!(
            rebound_program.steps[1]
                .guard
                .as_ref()
                .expect("rebound guard")
                .deferred_commitment_id,
            pending.commitment_id
        );
        let mut request = ConditionEvidenceRequestIR {
            schema: CONDITION_EVIDENCE_REQUEST_SCHEMA.to_string(),
            evidence_id: "EVIDENCE-R48-LINK".to_string(),
            conversation_id: "CHAT-GUARDED-EVIDENCE-LINK".to_string(),
            commitment_id: pending.commitment_id.clone(),
            condition_sha256: pending.condition_sha256.clone(),
            disposition: ConditionEvidenceDispositionIR::VerifiedSatisfied,
            source: ConditionEvidenceSourceIR::TrustedVerifier,
            verifier_receipt_sha256: String::new(),
        };
        request.verifier_receipt_sha256 = condition_evidence_receipt_sha256(&request);
        let receipt = api
            .submit_condition_evidence(&request)
            .expect("trusted evidence");
        assert_eq!(
            receipt.prior_status,
            DeferredCommitmentStatusIR::ConditionPending
        );
        assert_eq!(
            receipt.resulting_status,
            DeferredCommitmentStatusIR::Activated
        );
        assert!(!receipt.external_action_executed);
        let state = api
            .conversation_state("CHAT-GUARDED-EVIDENCE-LINK")
            .expect("post-evidence state");
        let activated = state
            .active_goals
            .iter()
            .find(|goal| Some(goal.goal_id.as_str()) == receipt.activated_goal_id.as_deref())
            .expect("activated linked goal");
        assert_eq!(activated.intent, PlanIntentIR::Repair);
        assert_eq!(activated.subject, "인덱스");
        assert!(crate::conversation::guarded_program_lifecycle_links_valid(
            state
        ));
    }

    #[test]
    fn rehashed_foreign_commitment_link_fails_cross_state_validation() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GUARDED-LINK-TAMPER",
            1,
            "캐시를 검사하고 캐시에 문제가 있으면 수리해",
        ))
        .expect("guarded source");
        let rebound = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-LINK-TAMPER",
                2,
                "인덱스도 같은 절차로 해",
            ))
            .expect("guarded rebound");
        let mut tampered = rebound.conversation_state.clone();
        let source_commitment_id = tampered
            .deferred_action_commitments
            .iter()
            .find(|commitment| commitment.introduced_turn == 1)
            .expect("source commitment")
            .commitment_id
            .clone();
        let rebound_program = tampered
            .active_discourse_programs
            .iter_mut()
            .find(|program| program.introduced_turn == 2)
            .expect("rebound program");
        rebound_program.steps[1]
            .guard
            .as_mut()
            .expect("rebound guard")
            .deferred_commitment_id = source_commitment_id;
        rebound_program.program_sha256 = discourse_program_sha256(rebound_program);
        assert!(!crate::conversation::guarded_program_lifecycle_links_valid(
            &tampered
        ));
    }

    #[test]
    fn guarded_program_preserves_negation_and_rejects_rehashed_authority_tampering() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-NEGATION",
                1,
                "캐시를 검사하고 캐시가 유효하지 않으면 수리해",
            ))
            .expect("negated guarded source");
        let program = &source.conversation_state.active_discourse_programs[0];
        let guard = program.steps[1].guard.as_ref().expect("negated guard");
        assert_eq!(guard.canonical_condition_predicate, "INVALID");
        assert!(guard.antecedent_negated);
        assert!(source.conversation_state.deferred_action_commitments[0]
            .condition_surface
            .starts_with("NOT ("));

        let mut tampered = program.clone();
        tampered.steps[1]
            .guard
            .as_mut()
            .expect("guard")
            .requires_verified_evidence = false;
        tampered.program_sha256 = discourse_program_sha256(&tampered);
        assert!(!tampered.validate(source.conversation_state.completed_turns));
    }

    #[test]
    fn compound_guard_preserves_typed_scope_and_rejects_expression_tampering() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOUND-GUARD-TAMPER",
                1,
                "Inspect the cache and if the cache is stale or damaged and invalid, repair the cache.",
            ))
            .expect("compound guarded source");
        let program = &source.conversation_state.active_discourse_programs[0];
        let guard = program.steps[1].guard.as_ref().expect("compound guard");
        assert_eq!(guard.canonical_condition_predicate, "COMPOUND");
        assert_eq!(
            guard.condition_expression.operator,
            GuardConditionOperatorIR::Any
        );
        assert_eq!(guard.condition_expression.children.len(), 2);
        assert_eq!(
            guard.condition_expression.children[1].operator,
            GuardConditionOperatorIR::All
        );
        assert_eq!(
            guard.condition_expression_sha256,
            guard_condition_expression_sha256(&guard.condition_expression)
        );

        let mut stale_hash = program.clone();
        stale_hash.steps[1]
            .guard
            .as_mut()
            .expect("guard")
            .condition_expression
            .children[0]
            .canonical_predicate = Some("EMPTY".to_string());
        stale_hash.program_sha256 = discourse_program_sha256(&stale_hash);
        assert!(!stale_hash.validate(source.conversation_state.completed_turns));

        let mut authority_tamper = program.clone();
        let tampered_guard = authority_tamper.steps[1].guard.as_mut().expect("guard");
        tampered_guard.condition_expression.children[0].semantic_authority = true;
        tampered_guard.condition_expression_sha256 =
            guard_condition_expression_sha256(&tampered_guard.condition_expression);
        authority_tamper.program_sha256 = discourse_program_sha256(&authority_tamper);
        assert!(!authority_tamper.validate(source.conversation_state.completed_turns));
    }

    #[test]
    fn conditional_predicate_alternative_is_not_an_action_alternative() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CONDITION-NOT-ACTION-ALTERNATIVE",
                1,
                "Inspect the cache and if the cache is stale or damaged, repair the cache.",
            ))
            .expect("compound guarded source");
        assert_eq!(source.disposition, ConversationTurnDispositionIR::Grounded);
        assert!(source
            .pragmatic_interpretation
            .compositional_analysis
            .unresolved_competitions
            .iter()
            .all(|competition| competition != "PRAGMATIC_ACTION_ALTERNATIVE"));
        assert_eq!(
            source.conversation_state.deferred_action_commitments.len(),
            1
        );
        assert_eq!(source.conversation_state.active_discourse_programs.len(), 1);
        assert!(source.conversation_state.active_discourse_programs[0].replayable);
    }

    #[test]
    fn mixed_target_conditional_workflow_is_recorded_but_never_replayed() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-MIXED-TARGET",
                1,
                "캐시를 검사하고 큐가 오래됐으면 큐를 수리해",
            ))
            .expect("mixed target source");
        assert_eq!(source.conversation_state.active_discourse_programs.len(), 1);
        let program = &source.conversation_state.active_discourse_programs[0];
        assert_eq!(program.guarded_step_count, 0);
        assert_eq!(program.blocked_frame_count, 1);
        assert!(!program.replayable);
        let rebound = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GUARDED-MIXED-TARGET",
                2,
                "인덱스도 같은 절차로 해",
            ))
            .expect("blocked rebound");
        assert_eq!(
            rebound.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(rebound
            .reference_resolution
            .ambiguous_reference_surfaces
            .iter()
            .any(|surface| surface == "ELLIPTICAL_ACTION"));
    }

    #[test]
    fn cross_turn_argument_correction_revises_the_prior_goal_not_the_world_model() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ARGUMENT-CORRECTION",
                1,
                "파일을 열어",
            ))
            .expect("first turn");
        assert_eq!(
            first
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidates()
                .len(),
            1
        );
        assert_eq!(first.conversation_state.active_goals.len(), 1);
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ARGUMENT-CORRECTION",
                2,
                "그거 말고 폴더로",
            ))
            .expect("correction turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "폴더를 열어"
        );
        assert_eq!(
            response
                .reference_resolution
                .discourse_bindings
                .last()
                .unwrap()
                .kind,
            crate::conversation::DiscourseBindingKindIR::CorrectedArgument
        );
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidate()
                .expect("corrected goal")
                .subject,
            "폴더"
        );
    }

    #[test]
    fn cross_language_pronoun_resolution_feeds_the_pragmatic_parser() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-CROSS-LANGUAGE-REFERENCE",
            1,
            "파일을 열어",
        ))
        .expect("Korean first turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CROSS-LANGUAGE-REFERENCE",
                2,
                "fix it",
            ))
            .expect("English reference turn");
        assert_eq!(
            response.reference_resolution.resolved_semantic_text,
            "fix file"
        );
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("resolved English action");
        assert_eq!(selected.subject, "file");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
    }

    #[test]
    fn ambiguous_repeat_of_a_multi_goal_program_requires_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-AMBIGUOUS-REPEAT",
            1,
            "파일을 읽고 저장해",
        ))
        .expect("multi-goal first turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-AMBIGUOUS-REPEAT",
                2,
                "그대로 해",
            ))
            .expect("ambiguous repeat turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response.reference_resolution.ambiguous_reference_surfaces,
            vec!["ELLIPTICAL_ACTION"]
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
    }

    #[test]
    fn event_reference_is_reintroduced_as_quoted_non_authoritative_content() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-EVENT-REFERENCE",
            1,
            "보고서를 저장해",
        ))
        .expect("event source turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-EVENT-REFERENCE",
                2,
                "그 작업을 설명해",
            ))
            .expect("event reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::EventReference
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘보고서를 저장해’"));
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert_eq!(
            analysis
                .selected_candidate()
                .expect("outer explanation")
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == dockable_semantic_core::PlanIntentIR::Execute
                && candidate.disposition
                    == crate::compositional_semantics::CandidateDispositionIR::NonAuthoritativeMention
        }));
    }

    #[test]
    fn result_reference_keeps_provenance_across_an_english_turn() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-RESULT-REFERENCE",
            1,
            "save report",
        ))
        .expect("result source turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-REFERENCE",
                2,
                "explain that result",
            ))
            .expect("result reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::ResultReference
        );
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidate()
                .expect("explanation")
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
    }

    #[test]
    fn proposition_reference_uses_prior_assertion_without_inventing_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-REFERENCE",
                1,
                "빌드가 실패했다",
            ))
            .expect("assertion turn");
        assert!(first
            .conversation_state
            .active_discourse_referents
            .iter()
            .any(|referent| {
                referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                    && !referent.external_execution_authorized
            }));
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-REFERENCE",
                2,
                "그 사실을 설명해",
            ))
            .expect("proposition reference turn");
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::PropositionReference
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘빌드가 실패했다’"));
    }

    #[test]
    fn attributed_command_is_remembered_as_a_claim_not_reissued_as_a_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ATTRIBUTED-COMMAND",
                1,
                "Alice said delete the file",
            ))
            .expect("attributed command turn");
        assert!(response.pragmatic_interpretation.inferred_goal.is_none());
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert!(analysis.attribution_graph.validate());
        assert!(analysis.candidates.iter().all(|candidate| {
            !analysis
                .attribution_graph
                .attributes_frame(&candidate.source_frame_id)
                || !candidate.external_execution_authorized
        }));
        let proposition = response
            .conversation_state
            .active_discourse_referents
            .iter()
            .find(|referent| {
                referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
            })
            .expect("attributed proposition memory");
        assert_eq!(proposition.attributed_source.as_deref(), Some("alice"));
        assert_eq!(
            proposition.attribution_attitude,
            Some(crate::attribution::AttributionAttitudeIR::Say)
        );
        assert!(!proposition.external_execution_authorized);
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("사실로 확인"));
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn nested_attribution_can_be_recalled_by_source_without_collapsing_beliefs() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NESTED-ATTRIBUTION",
                1,
                "Alice says Bob believes that the server is down",
            ))
            .expect("nested attribution turn");
        assert_eq!(first.disposition, ConversationTurnDispositionIR::Grounded);
        assert_eq!(
            first
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                })
                .count(),
            2
        );
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-NESTED-ATTRIBUTION",
                2,
                "explain Bob's belief",
            ))
            .expect("source-specific reference");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].kind,
            crate::conversation::DiscourseBindingKindIR::PropositionReference
        );
        assert_eq!(
            response.reference_resolution.discourse_bindings[0].referent_ids,
            vec!["DREF-P-000001-02"]
        );
        assert!(response
            .reference_resolution
            .resolved_semantic_text
            .contains("‘the server is down’"));
    }

    #[test]
    fn same_source_explicit_update_supersedes_prior_belief_without_becoming_fact() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-UPDATE",
            1,
            "Alice says that the server is down",
        ))
        .expect("initial attributed state");
        let updated = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-UPDATE",
                2,
                "Alice now says that the server is up",
            ))
            .expect("updated attributed state");
        let ledger = &updated.conversation_state.epistemic_ledger;
        assert_eq!(ledger.records.len(), 2);
        assert_eq!(
            ledger.records[0].status,
            crate::epistemic::BeliefRecordStatusIR::Superseded
        );
        assert_eq!(
            ledger.records[1].status,
            crate::epistemic::BeliefRecordStatusIR::Active
        );
        assert!(ledger.revisions.iter().any(|revision| {
            revision.kind == crate::epistemic::BeliefRevisionKindIR::Supersedes
        }));
        assert!(ledger.records.iter().all(|record| {
            !record.dialogue_truth_established && !record.external_execution_authorized
        }));
    }

    #[test]
    fn different_sources_with_opposite_states_remain_contested() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-CONFLICT",
            1,
            "Alice says that the server is down",
        ))
        .expect("Alice claim");
        let conflict = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-CONFLICT",
                2,
                "Bob says that the server is up",
            ))
            .expect("Bob claim");
        let ledger = &conflict.conversation_state.epistemic_ledger;
        assert_eq!(ledger.records.len(), 2);
        assert!(ledger
            .records
            .iter()
            .all(|record| { record.status == crate::epistemic::BeliefRecordStatusIR::Contested }));
        assert_eq!(
            conflict
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == crate::conversation::DiscourseReferentKindIR::Proposition
                })
                .count(),
            2
        );
    }

    #[test]
    fn explicit_claim_retraction_removes_it_from_active_reference_state() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-BELIEF-RETRACT",
            1,
            "Alice claims that the cache is corrupt",
        ))
        .expect("claim");
        let retracted = api
            .process_conversation_turn(&conversation_request(
                "CHAT-BELIEF-RETRACT",
                2,
                "Alice retracts that claim",
            ))
            .expect("retraction");
        assert_eq!(
            retracted.conversation_state.epistemic_ledger.records[0].status,
            crate::epistemic::BeliefRecordStatusIR::Retracted
        );
        assert!(retracted
            .conversation_state
            .active_discourse_referents
            .iter()
            .all(|referent| {
                referent.kind != crate::conversation::DiscourseReferentKindIR::Proposition
            }));
        assert!(retracted
            .conversation_state
            .epistemic_ledger
            .revisions
            .iter()
            .any(|revision| { revision.kind == crate::epistemic::BeliefRevisionKindIR::Retracts }));
    }

    #[test]
    fn competing_prior_propositions_require_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-PROPOSITION-AMBIGUITY",
            1,
            "빌드가 실패했다. 로그가 비었다.",
        ))
        .expect("two proposition turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-AMBIGUITY",
                2,
                "그 사실을 설명해",
            ))
            .expect("ambiguous reference turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response
            .reference_resolution
            .ambiguous_reference_surfaces
            .contains(&"Proposition_REFERENCE".to_string()));
    }

    #[test]
    fn clarification_answer_selects_one_pending_proposition_without_new_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let source = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-QUD",
                1,
                "빌드가 실패했다. 로그가 비었다.",
            ))
            .expect("two propositions");
        assert!(
            source
                .conversation_state
                .active_discourse_referents
                .iter()
                .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
                .count()
                >= 2,
            "{source:#?}"
        );
        let clarification = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-QUD",
                2,
                "그 사실을 설명해",
            ))
            .expect("clarification");
        assert!(
            clarification.conversation_state.pending_question.is_some(),
            "{:#?}",
            clarification.conversation_state
        );
        let selected = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PROPOSITION-QUD",
                3,
                "로그가 비었다는 쪽",
            ))
            .expect("selection");
        assert_eq!(
            selected.disposition,
            ConversationTurnDispositionIR::Grounded,
            "{selected:#?}"
        );
        assert!(selected
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer));
        assert!(selected.conversation_state.pending_question.is_none());
    }

    #[test]
    fn hold_floor_preserves_pending_question_until_direct_answer() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let clarification = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QUD-HOLD",
                1,
                "API를 조사해; 캐시를 삭제해",
            ))
            .expect("competing goals");
        let question_id = clarification
            .conversation_state
            .pending_question
            .as_ref()
            .expect("pending question")
            .question_id
            .clone();

        let held = api
            .process_conversation_turn(&conversation_request("CHAT-QUD-HOLD", 2, "잠깐"))
            .expect("hold floor");
        assert_eq!(held.disposition, ConversationTurnDispositionIR::HoldFloor);
        assert_eq!(
            held.conversation_state
                .pending_question
                .as_ref()
                .expect("question survives")
                .question_id,
            question_id
        );

        let selected = api
            .process_conversation_turn(&conversation_request("CHAT-QUD-HOLD", 3, "API 조사 쪽"))
            .expect("direct answer");
        assert_eq!(
            selected.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert!(selected
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer));
        assert!(selected.conversation_state.pending_question.is_none());
    }

    #[test]
    fn hold_floor_continuation_binds_the_active_task_in_both_languages() {
        for (conversation_id, first, hold, continuation) in [
            (
                "CHAT-HOLD-CONTINUATION-KO",
                "Quartz 캐시를 수리해",
                "음...",
                "그 작업 계속해",
            ),
            (
                "CHAT-HOLD-CONTINUATION-EN",
                "Repair the Quartz cache",
                "uh...",
                "Keep doing that work",
            ),
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            api.process_conversation_turn(&conversation_request(conversation_id, 1, first))
                .expect("initial task");
            let held = api
                .process_conversation_turn(&conversation_request(conversation_id, 2, hold))
                .expect("hold floor");
            assert_eq!(held.disposition, ConversationTurnDispositionIR::HoldFloor);
            let response = api
                .process_conversation_turn(&conversation_request(conversation_id, 3, continuation))
                .expect("task continuation");
            let understanding = &response
                .grounded_response
                .as_ref()
                .unwrap_or_else(|| panic!("missing continuation plan: {response:#?}"))
                .understanding;
            assert_eq!(understanding.intent, PlanIntentIR::Execute, "{response:#?}");
            assert!(
                understanding.subject.to_lowercase().contains("quartz"),
                "{response:#?}"
            );
        }
    }

    #[test]
    fn hold_floor_preserves_the_task_for_a_result_axis_correction() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-HOLD-RESULT-AXIS",
            1,
            "Cedar 인덱스를 갱신해",
        ))
        .expect("initial task");
        let held = api
            .process_conversation_turn(&conversation_request(
                "CHAT-HOLD-RESULT-AXIS",
                2,
                "어... 잠깐",
            ))
            .expect("hold floor");
        assert_eq!(held.disposition, ConversationTurnDispositionIR::HoldFloor);
        assert!(
            !held
                .conversation_state
                .action_state_ledger
                .records
                .is_empty(),
            "{held:#?}"
        );

        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-HOLD-RESULT-AXIS",
                3,
                "계획이 아니라 그 실제 결과를 말해",
            ))
            .expect("result-axis correction");
        assert_eq!(
            response.plan_result_boundary.query_focus,
            PlanResultQueryFocusIR::VerifiedResult,
            "{response:#?}"
        );
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ResultAbsence,
            "{response:#?}"
        );
        assert!(response.output.text.contains("Cedar"), "{response:#?}");
    }

    #[test]
    fn explicit_new_request_clears_pending_question_without_qud_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let clarification = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QUD-REPLACE",
                1,
                "파일을 분석해; 코드를 수정해",
            ))
            .expect("competing goals");
        assert!(clarification.conversation_state.pending_question.is_some());

        let replacement = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QUD-REPLACE",
                2,
                "새로 백업을 검사해",
            ))
            .expect("replacement request");
        assert_eq!(
            replacement.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert!(replacement.conversation_state.pending_question.is_none());
        assert!(replacement
            .reference_resolution
            .discourse_bindings
            .iter()
            .all(|binding| binding.kind != DiscourseBindingKindIR::ClarificationAnswer));
        assert!(replacement
            .reference_resolution
            .resolved_semantic_text
            .contains("백업"));
    }

    #[test]
    fn uncertain_or_attributed_choice_fails_closed_and_keeps_qud() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-QUD-UNCERTAIN",
            1,
            "API를 조사해; 캐시를 삭제해",
        ))
        .expect("competing goals");

        let uncertain = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QUD-UNCERTAIN",
                2,
                "민수는 첫 번째라고 말했다",
            ))
            .expect("reported choice");
        assert_eq!(
            uncertain.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(uncertain.grounded_response.is_none());
        assert!(uncertain.conversation_state.pending_question.is_some());
        assert!(uncertain
            .reference_resolution
            .discourse_bindings
            .iter()
            .all(|binding| binding.kind != DiscourseBindingKindIR::ClarificationAnswer));
    }

    #[test]
    fn cross_language_qud_answer_localizes_surface_without_changing_selection() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut voice = conversation_request("CHAT-QUD-XLANG", 1, "파일을 열어");
        voice.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
        voice.input_confidence_millis = 820;
        voice.alternatives = vec![crate::conversation::UtteranceAlternativeIR {
            text: "폴더를 열어".to_string(),
            confidence_millis: 790,
        }];
        api.process_conversation_turn(&voice)
            .expect("voice clarification");

        let mut answer = conversation_request("CHAT-QUD-XLANG", 2, "the second one");
        answer.output_language = Some(LanguageCodeIR::English);
        let selected = api
            .process_conversation_turn(&answer)
            .expect("cross-language answer");
        assert_eq!(
            selected.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert!(selected
            .reference_resolution
            .resolved_semantic_text
            .contains("folder"));
        assert!(!selected
            .reference_resolution
            .resolved_semantic_text
            .contains("file"));
        assert!(selected
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer));
    }

    #[test]
    fn semantic_roles_and_quantifier_scope_reach_goal_ir_constraints() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ROLE-GOAL-IR",
                1,
                "서버에서 모든 파일을 읽어",
            ))
            .expect("role-grounded turn");
        let grounded = response.grounded_response.expect("grounded response");
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"semantic_role_graph".to_string()));
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"quantifier_scope_explicit".to_string()));
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("Source=서버")
                && constraint.contains("Theme=모든 파일")));
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("All(파일)")));
    }

    #[test]
    fn recursive_grammatical_scope_reaches_goal_ir_without_execution_claims() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GRAMMATICAL-SCOPE-GOAL-IR",
                1,
                "Repair every cache that is stale and not locked.",
            ))
            .expect("scope-grounded turn");
        let grounded = response.grounded_response.expect("grounded response");
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"grammatical_scope_graph".to_string()));
        for operator in ["Quantifier", "Conjunction", "Negation", "Restriction"] {
            assert!(grounded
                .understanding
                .constraints
                .iter()
                .any(|constraint| constraint.contains(operator)));
        }
        assert_eq!(grounded.output.unsupported_freeform_claims, 0);
        assert!(!response.action_state_analysis.external_action_executed);
    }

    #[test]
    fn indirect_cost_benefit_utterance_becomes_a_verified_continuation_gate() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PRAGMATIC-GATE",
                1,
                "유일하게 고통을 참고 진행하려면 기존에는 점수만 높았지 실제 코딩능력은 한참 낮았다. 왜냐? capability와 routing이 결합되어 나온 거품점수라서 실제 커버리지는 낮았다. 그래서 통합을 하면 커버리지를 확장하는 효과가 있다. 이러면 할만하지",
            ))
            .expect("pragmatic turn");
        assert!(response
            .normalization
            .operations
            .iter()
            .all(|operation| operation.before != "실제"));
        assert!(response.pragmatic_interpretation.clauses.len() >= 5);
        assert_eq!(
            response.pragmatic_interpretation.speech_act,
            crate::pragmatics::SpeechActIR::ConditionalContinuation
        );
        let gate = response
            .pragmatic_interpretation
            .continuation_gate
            .as_ref()
            .expect("continuation gate");
        assert_eq!(gate.current_task, "통합");
        assert_eq!(gate.required_benefit, "커버리지를 확장하는 효과가 있다");
        assert_eq!(
            gate.negative_action,
            crate::pragmatics::DecisionBranchActionIR::ReportNegativeAndAskWhetherToStop
        );
        let grounded = response.grounded_response.expect("grounded response");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Investigate
        );
        assert!(grounded
            .understanding
            .subject
            .starts_with("continuation_gate(task=통합"));
        assert!(response.output.text.contains("이득이 확인되면 계속"));
        assert!(response.output.text.contains("멈출지 물을게"));
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ContinuationGate
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn continuation_gate_outranks_surface_when_question_answering() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-CONTINUATION-REFERENCE",
            1,
            "We are migrating the storage adapter.",
        ))
        .expect("task turn");
        api.process_conversation_turn(&conversation_request(
            "CHAT-CONTINUATION-REFERENCE",
            2,
            "The latest number may reflect reused samples.",
        ))
        .expect("neutral turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-CONTINUATION-REFERENCE",
                3,
                "Continue it only when fresh trials expand production coverage; otherwise ask before stopping.",
            ))
            .expect("gate turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert!(
            response
                .output
                .text
                .contains("migrating the storage adapter"),
            "output: {}; gate: {:#?}",
            response.output.text,
            response.pragmatic_interpretation.continuation_gate
        );
        assert!(response.output.text.contains("production coverage"));
        assert!(!response.output.text.contains("presupposes"));
        assert!(response
            .conversation_state
            .active_goals
            .iter()
            .all(|goal| goal.canonical_predicate != "CONTINUE"));
    }

    #[test]
    fn same_turn_result_anaphor_keeps_assessment_grounded() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LOCAL-RESULT",
                1,
                "게시가 감사 추적을 보존하는지 평가하고 그 결과만 보고해.",
            ))
            .expect("local result request");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::Grounded
        );
        assert!(response
            .reference_resolution
            .ambiguous_reference_surfaces
            .is_empty());
        assert!(response.conversation_state.active_goals.iter().any(|goal| {
            goal.intent == dockable_semantic_core::PlanIntentIR::Investigate
                && goal.subject.contains("감사")
        }));
        assert!(response
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(|binding| binding.evidence.iter().any(|evidence| {
                evidence == "SYNTACTIC_PRIORITY:SAME_TURN_RESULT_OF_PRECEDING_EVENT"
            })));
    }

    #[test]
    fn standalone_language_api_uses_the_same_pragmatic_reasoning_circuit() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process(&NaturalLanguageRequestIR {
                schema: NATURAL_LANGUAGE_REQUEST_SCHEMA.to_string(),
                request_id: "PRAGMATIC-STANDALONE".to_string(),
                text: "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어드는 효과가 있다. 그 정도 이득이면 계속 진행할 만하다.".to_string(),
                output_language: Some(LanguageCodeIR::Korean),
                context_tags: Vec::new(),
                max_plan_steps: 12,
            })
            .expect("standalone pragmatic reasoning");
        assert!(response.pragmatic_interpretation.has_continuation_gate());
        assert_eq!(
            response.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Investigate
        );
        assert_ne!(
            response.understanding.subject,
            response.understanding.original_text
        );
    }

    #[test]
    fn indirect_problem_statement_projects_a_repair_goal_without_broad_authority() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-IMPLICIT-REPAIR",
                1,
                "배포 후 오류가 늘었네. 이 상태로 둘 수는 없지.",
            ))
            .expect("implicit repair");
        let goal = response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .expect("inferred goal");
        assert_eq!(goal.intent, dockable_semantic_core::PlanIntentIR::Repair);
        assert!(!goal.external_execution_authorized);
        assert!(response.output.text.contains("수리가 필요"));
        assert_eq!(
            response
                .grounded_response
                .expect("grounded response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Repair
        );
    }

    #[test]
    fn quoted_publish_cannot_capture_a_later_recovery_assessment() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-QUOTED-ASSESSMENT",
                1,
                "The release lead wrote, 'publish the bundle tonight.' I am asking only for an assessment of recovery cost; do not publish it.",
            ))
            .expect("assessment request");
        assert!(
            response.conversation_state.active_goals.iter().any(|goal| {
                goal.intent == dockable_semantic_core::PlanIntentIR::Investigate
                    && goal.subject == "recovery cost"
            }),
            "assessment response: {response:#?}"
        );
        assert!(!response
            .conversation_state
            .active_goals
            .iter()
            .any(|goal| goal.canonical_predicate == "DEPLOY"));
    }

    #[test]
    fn capability_correction_keeps_the_full_korean_embedded_question() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-KO-CAPABILITY-ASSESSMENT",
            1,
            "시스템이 산출물을 게시할 수 있어?",
        ))
        .expect("capability question");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-KO-CAPABILITY-ASSESSMENT",
                2,
                "가능 여부를 묻는 게 아니야. 게시가 감사 추적을 보존하는지 평가하고 그 결과만 보고해.",
            ))
            .expect("corrected assessment");
        let goal = response
            .pragmatic_interpretation
            .inferred_goal
            .as_ref()
            .expect("investigation goal");
        assert_eq!(
            goal.intent,
            dockable_semantic_core::PlanIntentIR::Investigate
        );
        assert!(
            goal.subject.contains("감사"),
            "assessment response: {response:#?}"
        );
        assert!(!response
            .conversation_state
            .active_goals
            .iter()
            .any(|goal| goal.canonical_predicate == "DEPLOY"));
    }

    #[test]
    fn pragmatic_memory_restores_an_elliptical_continuation_gate_across_turns() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LONG-PRAGMATIC",
                1,
                "마이그레이션은 힘들다. 마이그레이션을 하면 장애 빈도가 감소한다. 그 정도 이득이면 계속 진행할 만하다.",
            ))
            .expect("first gate");
        assert!(first.pragmatic_state.pending_continuation_gate.is_some());
        let second = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LONG-PRAGMATIC",
                2,
                "그 정도면 계속할 만하지",
            ))
            .expect("elliptical gate");
        let gate = second
            .pragmatic_interpretation
            .continuation_gate
            .as_ref()
            .expect("restored gate");
        assert_eq!(gate.current_task, "마이그레이션");
        assert!(gate.required_benefit.contains("장애 빈도"));
        assert_eq!(second.pragmatic_state.completed_turns, 2);
    }

    #[test]
    fn user_rejection_suspends_a_pending_gate_until_explicitly_reopened() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-SUSPEND-GATE",
            1,
            "리팩터링은 힘들다. 리팩터링을 하면 장애가 줄어든다. 이러면 계속할 만하다.",
        ))
        .expect("initial gate");
        let rejection = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SUSPEND-GATE",
                2,
                "그래도 계속하지 마",
            ))
            .expect("rejection");
        assert_eq!(
            rejection
                .pragmatic_state
                .pending_continuation_gate
                .as_ref()
                .expect("remembered gate")
                .status,
            crate::pragmatic_memory::PendingGateStatusIR::SuspendedByUser
        );
        let elliptical = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SUSPEND-GATE",
                3,
                "그 정도면 계속할 만하지",
            ))
            .expect("later ellipsis");
        assert!(elliptical
            .pragmatic_interpretation
            .continuation_gate
            .is_none());
    }

    #[test]
    fn sarcastic_praise_after_failure_is_not_treated_as_approval() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SARCASM",
                1,
                "테스트가 전부 깨졌네. 아주 잘했어.",
            ))
            .expect("sarcastic turn");
        assert_eq!(
            response.pragmatic_interpretation.speech_act,
            crate::pragmatics::SpeechActIR::NegativeEvaluation
        );
        assert!(
            response
                .pragmatic_interpretation
                .nonliteral_analysis
                .literal_execution_blocked
        );
        assert!(response.output.text.contains("긍정 승인이 아니라"));
    }

    #[test]
    fn context_free_literal_or_metaphorical_fire_fails_closed() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-FIRE-AMBIGUITY",
                1,
                "여기 불이 났어",
            ))
            .expect("ambiguous fire");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("문자 그대로의 상황"));
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
    }

    #[test]
    fn software_metaphor_selects_problem_state_without_literal_execution() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-SOFTWARE-METAPHOR",
                1,
                "배포 뒤 프로젝트에 불이 났어",
            ))
            .expect("software metaphor");
        assert_eq!(
            response
                .pragmatic_interpretation
                .nonliteral_analysis
                .expressions[0]
                .selected_reading,
            crate::nonliteral::ReadingSelectionIR::Figurative
        );
        assert!(response.output.text.contains("비유적 상태"));
    }

    #[test]
    fn compositional_scope_selects_explanation_and_blocks_negated_repair() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-NEGATION",
                1,
                "서비스를 수정하지 말고 장애 원인만 설명해줘",
            ))
            .expect("scope-aware turn");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("outer explanation");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(response
            .pragmatic_interpretation
            .compositional_analysis
            .candidates
            .iter()
            .any(|candidate| {
                candidate.intent == dockable_semantic_core::PlanIntentIR::Repair
                    && candidate.disposition
                        == crate::compositional_semantics::CandidateDispositionIR::BlockedByNegation
            }));
        assert_eq!(
            response
                .grounded_response
                .expect("grounded explanation")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
    }

    #[test]
    fn quoted_destructive_command_remains_non_authoritative_in_cognitive_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-QUOTE",
                1,
                "‘데이터를 삭제해’라는 문장을 설명해",
            ))
            .expect("quoted command turn");
        let analysis = &response.pragmatic_interpretation.compositional_analysis;
        assert_eq!(
            analysis.selected_candidate().expect("outer request").intent,
            dockable_semantic_core::PlanIntentIR::Explain
        );
        assert!(analysis.candidates.iter().any(|candidate| {
            candidate.intent == dockable_semantic_core::PlanIntentIR::Execute
                && candidate.disposition
                    == crate::compositional_semantics::CandidateDispositionIR::NonAuthoritativeMention
                && !candidate.external_execution_authorized
        }));
        assert!(response
            .conversation_state
            .action_state_ledger
            .records
            .is_empty());
    }

    #[test]
    fn equally_supported_conflicting_requests_require_clarification() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-COMPOSITIONAL-COMPETITION",
                1,
                "파일을 분석해; 코드를 수정해",
            ))
            .expect("competing request turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .clarification_required
        );
        assert!(response.output.text.contains("실제 요청"));
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
    }

    #[test]
    fn injected_predicate_enters_the_same_compositional_reasoning_path() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        assert!(api
            .inject_compositional_predicate(crate::compositional_semantics::PredicateLexemeIR {
                schema: crate::compositional_semantics::PREDICATE_LEXEME_SCHEMA.to_string(),
                predicate_id: "P-REFINE-DOCUMENT-COGNITIVE".to_string(),
                language: LanguageCodeIR::Korean,
                surface_forms: vec!["다듬".to_string()],
                canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
                intent_hint: dockable_semantic_core::PlanIntentIR::Create,
                definition: "revise a document into a clearer finished form".to_string(),
                confidence_millis: 920,
            })
            .expect("inject predicate"));
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-LEARNED-PREDICATE",
                1,
                "문서를 다듬어줘",
            ))
            .expect("learned predicate turn");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("selected learned predicate");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
        assert_eq!(selected.subject, "문서");
        assert_eq!(
            response
                .grounded_response
                .expect("grounded create response")
                .understanding
                .intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
    }

    #[test]
    fn learned_predicate_snapshot_restores_language_capability() {
        let predicate = crate::compositional_semantics::PredicateLexemeIR {
            schema: crate::compositional_semantics::PREDICATE_LEXEME_SCHEMA.to_string(),
            predicate_id: "P-PERSIST-REFINE-COGNITIVE".to_string(),
            language: LanguageCodeIR::Korean,
            surface_forms: vec!["다듬".to_string()],
            canonical_predicate: "C_REFINE_DOCUMENT".to_string(),
            intent_hint: dockable_semantic_core::PlanIntentIR::Create,
            definition: "revise a document into a clearer finished form".to_string(),
            confidence_millis: 920,
        };
        let mut source = CognitiveApi::new_embedded().unwrap();
        source
            .inject_compositional_predicate(predicate)
            .expect("inject predicate");
        let snapshot = source.export_compositional_predicates();

        let mut restored = CognitiveApi::new_embedded().unwrap();
        restored
            .import_compositional_predicates(&snapshot)
            .expect("restore predicates");
        let response = restored
            .process_conversation_turn(&conversation_request(
                "CHAT-RESTORED-PREDICATE",
                1,
                "문서를 다듬어줘",
            ))
            .expect("restored language capability");
        let selected = response
            .pragmatic_interpretation
            .compositional_analysis
            .selected_candidate()
            .expect("restored predicate selected");
        assert_eq!(
            selected.intent,
            dockable_semantic_core::PlanIntentIR::Create
        );
        assert_eq!(selected.subject, "문서");
    }

    #[test]
    fn ordered_multi_goal_language_projects_a_plan_without_losing_prohibition() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-ORDERED-GOALS",
                1,
                "파일을 읽고 각 줄을 변환한 뒤 저장해. 원본은 지우지 마",
            ))
            .expect("ordered multi-goal turn");
        let graph = response
            .pragmatic_interpretation
            .compositional_analysis
            .goal_graph
            .as_ref()
            .expect("goal graph");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.prohibitions.len(), 1);
        let grounded = response.grounded_response.expect("grounded plan");
        assert_eq!(
            grounded.understanding.intent,
            dockable_semantic_core::PlanIntentIR::Plan
        );
        assert_eq!(grounded.understanding.desired_outcomes.len(), 3);
        assert!(grounded
            .understanding
            .constraints
            .iter()
            .any(|constraint| constraint.contains("preserve explicit prohibition")));
        assert!(grounded
            .understanding
            .semantic_tags
            .contains(&"ordered_multi_goal_request".to_string()));
    }

    #[test]
    fn ambiguous_voice_turn_asks_instead_of_guessing() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut request = conversation_request("CHAT-VOICE", 1, "파일을 열어");
        request.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
        request.input_confidence_millis = 800;
        request.alternatives = vec![crate::conversation::UtteranceAlternativeIR {
            text: "파일을 얼어".to_string(),
            confidence_millis: 770,
        }];
        let response = api.process_conversation_turn(&request).expect("voice turn");
        assert_eq!(
            response.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("어느 쪽인지"));
        assert_eq!(response.conversation_state.unresolved_reference_count, 1);
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
    }

    #[test]
    fn acronym_restoration_is_bounded_to_user_provenance() {
        assert_eq!(
            user_grounded_acronyms("CCTV와 API 오류를 확인해"),
            vec!["CCTV", "API"]
        );
        assert_eq!(
            restore_user_grounded_acronyms("cctv와 api 오류", "CCTV와 API 오류를 확인해"),
            "CCTV와 API 오류"
        );
        assert_eq!(
            restore_user_grounded_acronyms("gpu 오류", "캐시 오류를 확인해"),
            "gpu 오류"
        );
    }

    #[test]
    fn quoted_affect_is_not_attributed_to_the_user() {
        assert_eq!(detect_user_affect("민수가 ‘답답해’라고 말했다"), None);
        assert_eq!(detect_user_affect("The log says ‘I am worried’"), None);
        assert_eq!(
            detect_user_affect("배포가 또 실패해서 화나"),
            Some(UserAffectIR::Angry)
        );
    }

    #[test]
    fn conversational_output_surfaces_validated_plan_operations() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-PLAN-PREVIEW",
                1,
                "CCTV 오류를 진단해",
            ))
            .expect("grounded plan turn");
        assert!(response.output.text.contains("CCTV"));
        assert!(response.output.text.contains("현재 상태"));
        assert!(response.output.text.contains("원인을 좁"));
        assert!(response.output.text.contains("검증"));
        assert!(response.output.text.contains("아직 실행한 것은 아니"));
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
        assert!(response.natural_realization.validate());
        assert_eq!(response.output.unsupported_freeform_claims, 0);
    }

    #[test]
    fn explaining_a_bound_result_preserves_the_underlying_active_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let first = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-FOCUS",
                1,
                "CCTV 오류를 진단해",
            ))
            .expect("first turn");
        let goal_ids = first
            .conversation_state
            .active_goals
            .iter()
            .map(|goal| goal.goal_id.clone())
            .collect::<Vec<_>>();
        api.process_conversation_turn(&conversation_request("CHAT-RESULT-FOCUS", 2, "고마워"))
            .expect("social turn");
        let explanation = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-FOCUS",
                3,
                "그 결과를 설명해",
            ))
            .expect("result explanation");
        assert_eq!(
            explanation
                .conversation_state
                .active_goals
                .iter()
                .map(|goal| goal.goal_id.clone())
                .collect::<Vec<_>>(),
            goal_ids
        );
        assert!(explanation.output.text.contains("CCTV"));
        assert!(explanation.output.text.contains("실행 결과는 아직"));
        assert!(explanation.grounded_response.is_none());
        assert!(explanation.output.grounded_plan_sha256.is_none());
        assert!(!explanation.output.text.contains("‘‘"));
    }

    #[test]
    fn bound_result_absence_precedes_generic_presupposition_qa() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-RESULT-PRECEDENCE",
            1,
            "DNS 문제를 수리해",
        ))
        .expect("plan turn");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-RESULT-PRECEDENCE",
                2,
                "그 결과가 어떻게 됐어?",
            ))
            .expect("result question");
        assert!(response.discourse_answer.is_none());
        assert!(
            response.output.text.contains("DNS"),
            "unexpected result-reference realization: {}",
            response.output.text
        );
        assert!(response.output.text.contains("실행 결과는 아직"));
        assert!(response.grounded_response.is_none());
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn explicit_topic_return_scopes_long_horizon_result_reference() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        for (turn, text) in [
            "Switch to cache.",
            "Diagnose the cache.",
            "Switch to queue.",
            "Inspect the queue.",
            "Return to cache.",
        ]
        .into_iter()
        .enumerate()
        {
            api.process_conversation_turn(&conversation_request(
                "CHAT-TOPIC-RESULT",
                u64::try_from(turn + 1).unwrap(),
                text,
            ))
            .unwrap();
        }
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TOPIC-RESULT",
                6,
                "And the result?",
            ))
            .unwrap();
        let binding = response
            .reference_resolution
            .discourse_bindings
            .iter()
            .find(|binding| binding.kind == DiscourseBindingKindIR::ResultReference)
            .expect("topic-scoped result binding");
        assert!(binding.resolved_surface.contains("cache"), "{binding:#?}");
        assert!(!binding.resolved_surface.contains("queue"), "{binding:#?}");
        assert_eq!(binding.referent_ids, ["DREF-R-000002-01"]);
        assert!(binding
            .evidence
            .iter()
            .any(|evidence| evidence.starts_with("REFERENCE_SCOPE:EXPLICIT_TOPIC:")));
        assert!(response.grounded_response.is_none());
        assert!(response.output.grounded_plan_sha256.is_none());
    }

    #[test]
    fn topic_return_restores_each_suspended_qud_independently() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TOPIC-QUD",
            1,
            "Switch to cache.",
        ))
        .unwrap();
        let mut cache_question = conversation_request("CHAT-TOPIC-QUD", 2, "Inspect the cache.");
        cache_question.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
        cache_question.input_confidence_millis = 820;
        cache_question.alternatives = vec![crate::conversation::UtteranceAlternativeIR {
            text: "Repair the cache.".to_string(),
            confidence_millis: 790,
        }];
        api.process_conversation_turn(&cache_question).unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TOPIC-QUD",
            3,
            "Switch to queue.",
        ))
        .unwrap();
        let mut queue_question = conversation_request("CHAT-TOPIC-QUD", 4, "Inspect the queue.");
        queue_question.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
        queue_question.input_confidence_millis = 820;
        queue_question.alternatives = vec![crate::conversation::UtteranceAlternativeIR {
            text: "Delete the queue.".to_string(),
            confidence_millis: 790,
        }];
        let queued = api.process_conversation_turn(&queue_question).unwrap();
        assert_eq!(queued.conversation_state.topic_pending_questions.len(), 2);

        let restored = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TOPIC-QUD",
                5,
                "Return to cache.",
            ))
            .unwrap();
        let question = restored
            .conversation_state
            .pending_question
            .as_ref()
            .expect("cache question restored");
        assert_eq!(question.source_turn, 2);
        assert_eq!(
            question.topic_id.as_deref(),
            restored
                .conversation_state
                .active_topics
                .first()
                .map(|topic| topic.topic_id.as_str())
        );

        let selected = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TOPIC-QUD",
                6,
                "The second one.",
            ))
            .unwrap();
        assert!(selected
            .reference_resolution
            .discourse_bindings
            .iter()
            .any(
                |binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer
                    && binding.resolved_surface.contains("repair")
                    && !binding.resolved_surface.contains("queue")
            ));
        assert!(selected.conversation_state.pending_question.is_none());
        assert_eq!(selected.conversation_state.topic_pending_questions.len(), 1);

        let queue_restored = api
            .process_conversation_turn(&conversation_request(
                "CHAT-TOPIC-QUD",
                7,
                "Return to queue.",
            ))
            .unwrap();
        assert_eq!(
            queue_restored
                .conversation_state
                .pending_question
                .as_ref()
                .map(|question| question.source_turn),
            Some(4)
        );
    }

    #[test]
    fn unseen_topic_result_ellipsis_does_not_fall_back_to_global_recency() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-MISSING-TOPIC-RESULT",
            1,
            "Switch to queue.",
        ))
        .unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-MISSING-TOPIC-RESULT",
            2,
            "Repair the queue.",
        ))
        .unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-MISSING-TOPIC-RESULT",
            3,
            "Switch to report.",
        ))
        .unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-MISSING-TOPIC-RESULT",
                4,
                "And the output?",
            ))
            .unwrap();
        assert!(response
            .reference_resolution
            .discourse_bindings
            .iter()
            .all(|binding| binding.kind != DiscourseBindingKindIR::ResultReference));
        assert!(response
            .reference_resolution
            .ambiguous_reference_surfaces
            .iter()
            .any(|surface| surface == "Result_REFERENCE"));
        assert!(!response
            .reference_resolution
            .resolved_semantic_text
            .contains("queue"));
        assert!(response.grounded_response.is_none());
    }

    #[test]
    fn direct_feedback_is_realized_without_a_fake_work_plan() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DIRECT-FEEDBACK",
                1,
                "이 답변은 별로야",
            ))
            .expect("feedback turn");
        assert_eq!(
            response.pragmatic_interpretation.speech_act,
            SpeechActIR::NegativeEvaluation
        );
        assert!(response.grounded_response.is_none());
        assert!(response.output.text.contains("도움이 되지 않았"));
        assert!(response.output.text.contains("어긋난 부분"));
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::UserFeedback
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
    }

    #[test]
    fn response_constraint_survives_cross_language_turn_through_one_ledger() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let feedback = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DIRECTIVE-MEMORY",
                1,
                "이 답변은 너무 길어",
            ))
            .expect("Korean response-length feedback");
        let directive = feedback
            .conversation_state
            .dialogue_directive_ledger
            .active()
            .next()
            .expect("active response-length directive");
        assert_eq!(directive.kind, DialogueDirectiveKindIR::ResponseLength);
        assert_eq!(directive.target_key, "ASSISTANT_RESPONSE");
        assert_eq!(directive.value_key, "CONCISE");
        assert!(!directive.semantic_authority);
        assert!(!directive.external_execution_authorized);

        let task = api
            .process_conversation_turn(&conversation_request(
                "CHAT-DIRECTIVE-MEMORY",
                2,
                "Inspect the cache.",
            ))
            .expect("English follow-up task");
        let tag = "DIALOGUE_DIRECTIVE:RESPONSELENGTH:ASSISTANT_RESPONSE:CONCISE";
        assert!(task
            .grounded_response
            .as_deref()
            .is_some_and(|response| response
                .understanding
                .semantic_tags
                .iter()
                .any(|semantic_tag| semantic_tag == tag)));
        assert!(task
            .natural_realization
            .sentences
            .iter()
            .flat_map(|sentence| sentence.source_refs.iter())
            .any(|source| source == tag));
        assert!(task
            .natural_realization
            .generation_traces
            .iter()
            .any(|trace| {
                trace.meaning.nodes.iter().any(|node| {
                    node.grounding_refs
                        .iter()
                        .any(|evidence| evidence.starts_with("DIALOGUE_DIRECTIVE:"))
                })
            }));
        let mut baseline_api = CognitiveApi::new_embedded().unwrap();
        let baseline = baseline_api
            .process_conversation_turn(&conversation_request(
                "CHAT-DIRECTIVE-BASELINE",
                1,
                "Inspect the cache.",
            ))
            .expect("unconstrained baseline");
        assert!(
            task.output.text.chars().count() < baseline.output.text.chars().count(),
            "concise={} baseline={}",
            task.output.text,
            baseline.output.text
        );
        assert_eq!(
            task.conversation_state
                .dialogue_directive_ledger
                .active()
                .count(),
            1
        );
        assert!(task.validate_against(&conversation_request(
            "CHAT-DIRECTIVE-MEMORY",
            2,
            "Inspect the cache.",
        )));

        let compound_text = "계속 실패해서 너무 답답해. Cedar 큐 원인을 좁히는 걸 도와줘.";
        let mut concise_compound_api = CognitiveApi::new_embedded().unwrap();
        concise_compound_api
            .process_conversation_turn(&conversation_request(
                "CHAT-DIRECTIVE-COMPOUND",
                1,
                "이 답변은 너무 길어",
            ))
            .expect("establish concise policy");
        let concise_compound_request =
            conversation_request("CHAT-DIRECTIVE-COMPOUND", 2, compound_text);
        let concise_compound = concise_compound_api
            .process_conversation_turn(&concise_compound_request)
            .expect("concise affect-plus-task turn");
        let mut compound_baseline_api = CognitiveApi::new_embedded().unwrap();
        let compound_baseline_request =
            conversation_request("CHAT-DIRECTIVE-COMPOUND-BASELINE", 1, compound_text);
        let compound_baseline = compound_baseline_api
            .process_conversation_turn(&compound_baseline_request)
            .expect("unconstrained affect-plus-task turn");
        assert_eq!(
            concise_compound
                .natural_realization
                .response_plan
                .moves
                .len(),
            1,
            "{concise_compound:#?}"
        );
        assert_eq!(
            concise_compound.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview
        );
        assert!(concise_compound.natural_realization.response_plan.moves[0]
            .evidence
            .iter()
            .any(|evidence| evidence.starts_with("DIALOGUE_DIRECTIVE:")));
        assert!(
            compound_baseline
                .natural_realization
                .response_plan
                .moves
                .iter()
                .any(|response_move| response_move.response_act
                    == NaturalResponseActIR::AffectSupport)
        );
        assert!(
            concise_compound.output.text.chars().count()
                < compound_baseline.output.text.chars().count(),
            "concise={} baseline={}",
            concise_compound.output.text,
            compound_baseline.output.text
        );
        assert!(concise_compound.validate_against(&concise_compound_request));
    }

    #[test]
    fn explicit_lexical_directives_update_policy_without_becoming_fake_tasks() {
        for (index, (directive_text, language, expected_value)) in [
            (
                "응답은 핵심만 간결하게 해줘.",
                LanguageCodeIR::Korean,
                "CONCISE",
            ),
            (
                "Please make the response detailed.",
                LanguageCodeIR::English,
                "DETAILED",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-EXPLICIT-DIRECTIVE-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request = conversation_request(&conversation_id, 1, directive_text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .expect("explicit response directive");
            assert!(response.validate_against(&request), "{response:#?}");
            assert_eq!(
                response.disposition,
                ConversationTurnDispositionIR::Grounded,
                "{response:#?}"
            );
            assert!(response.grounded_response.is_none(), "{response:#?}");
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::InformAcknowledgement,
                "{response:#?}"
            );
            let directive = response
                .conversation_state
                .dialogue_directive_ledger
                .active()
                .next()
                .expect("committed explicit directive");
            assert_eq!(directive.kind, DialogueDirectiveKindIR::ResponseLength);
            assert_eq!(directive.target_key, "ASSISTANT_RESPONSE");
            assert_eq!(directive.value_key, expected_value);
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
            assert!(response.conversation_state.active_goals.is_empty());
        }

        for text in [
            "그 응답은 간결했다.",
            "The response was concise.",
            "‘응답은 짧게 해줘’라는 문장을 설명해.",
        ] {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let response = api
                .process_conversation_turn(&conversation_request(
                    "CHAT-NON-DIRECTIVE-DESCRIPTION",
                    1,
                    text,
                ))
                .expect("non-directive description");
            assert_eq!(
                response
                    .conversation_state
                    .dialogue_directive_ledger
                    .active()
                    .count(),
                0,
                "text={text} response={response:#?}"
            );
        }

        let mut compound_api = CognitiveApi::new_embedded().unwrap();
        let compound_request = conversation_request(
            "CHAT-DIRECTIVE-PLUS-TASK",
            1,
            "Please keep the response concise and inspect the Cedar queue.",
        );
        let compound = compound_api
            .process_conversation_turn(&compound_request)
            .expect("directive plus independent task");
        assert!(
            compound.validate_against(&compound_request),
            "{compound:#?}"
        );
        assert!(compound.grounded_response.is_some(), "{compound:#?}");
        assert_eq!(
            compound.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{compound:#?}"
        );
        assert!(
            compound
                .native_language_circuit
                .selected_live_goals
                .iter()
                .any(|goal| goal.intent == PlanIntentIR::Investigate
                    && goal.subject.contains("Cedar"))
        );
        assert_eq!(
            compound
                .conversation_state
                .dialogue_directive_ledger
                .active()
                .next()
                .map(|directive| directive.value_key.as_str()),
            Some("CONCISE")
        );
        assert_eq!(compound.natural_realization.response_plan.moves.len(), 1);
        assert!(
            !compound
                .language_cortex_integration
                .external_action_executed
        );

        let mut format_api = CognitiveApi::new_embedded().unwrap();
        let table_directive_request =
            conversation_request("CHAT-FORMAT-DIRECTIVE", 1, "답변은 표로 해줘.");
        let table_directive = format_api
            .process_conversation_turn(&table_directive_request)
            .expect("table response directive");
        assert!(table_directive.validate_against(&table_directive_request));
        assert_eq!(
            table_directive
                .natural_realization
                .response_plan
                .response_format,
            NaturalResponseFormatIR::Table
        );
        assert!(table_directive.output.text.starts_with("| 번호 | 내용 |"));
        assert!(table_directive.conversation_state.active_goals.is_empty());

        let table_task_request =
            conversation_request("CHAT-FORMAT-DIRECTIVE", 2, "Cedar 큐를 점검해.");
        let table_task = format_api
            .process_conversation_turn(&table_task_request)
            .expect("cross-turn table response task");
        assert!(table_task.validate_against(&table_task_request));
        assert_eq!(
            table_task.natural_realization.response_plan.response_format,
            NaturalResponseFormatIR::Table
        );
        assert!(table_task.output.text.starts_with("| 번호 | 내용 |"));
        assert!(table_task
            .conversation_state
            .active_goals
            .iter()
            .any(|goal| goal.subject.to_lowercase().contains("cedar")));

        let bullet_task_request = conversation_request(
            "CHAT-FORMAT-COMPOUND",
            1,
            "Answer with bullet points and inspect the Juniper cache.",
        );
        let mut bullet_api = CognitiveApi::new_embedded().unwrap();
        let bullet_task = bullet_api
            .process_conversation_turn(&bullet_task_request)
            .expect("compound bullet response task");
        assert!(bullet_task.validate_against(&bullet_task_request));
        assert_eq!(
            bullet_task
                .natural_realization
                .response_plan
                .response_format,
            NaturalResponseFormatIR::Bullets
        );
        assert!(bullet_task
            .output
            .text
            .lines()
            .all(|line| line.starts_with("- ")));
        assert!(bullet_task
            .native_language_circuit
            .selected_live_goals
            .iter()
            .any(|goal| goal.subject.contains("Juniper")));
        assert!(
            bullet_task
                .conversation_state
                .active_goals
                .iter()
                .any(|goal| goal.subject.to_lowercase().contains("juniper")),
            "the accepted semantic plan must be the memory source: {bullet_task:#?}"
        );

        let plain_request = conversation_request(
            "CHAT-FORMAT-DIRECTIVE",
            3,
            "이제 답변은 일반 문장으로 해줘.",
        );
        let plain = format_api
            .process_conversation_turn(&plain_request)
            .expect("plain format supersession");
        assert!(plain.validate_against(&plain_request));
        assert_eq!(
            plain.natural_realization.response_plan.response_format,
            NaturalResponseFormatIR::Plain
        );
        assert!(!plain.output.text.starts_with('|'));

        let mut conflict_api = CognitiveApi::new_embedded().unwrap();
        let conflict_request =
            conversation_request("CHAT-DIRECTIVE-CONFLICT", 1, "답변은 짧고 자세히 해줘.");
        let conflict = conflict_api
            .process_conversation_turn(&conflict_request)
            .expect("conflicting response directive");
        assert_eq!(
            conflict.disposition,
            ConversationTurnDispositionIR::ClarificationRequired,
            "{conflict:#?}"
        );
        assert_eq!(
            conflict
                .conversation_state
                .dialogue_directive_ledger
                .active()
                .count(),
            0
        );
        assert!(
            !conflict
                .language_cortex_integration
                .external_action_executed
        );
        let format_conflict_request =
            conversation_request("CHAT-FORMAT-CONFLICT", 1, "응답은 표로 목록으로 해줘.");
        let mut format_conflict_api = CognitiveApi::new_embedded().unwrap();
        let format_conflict = format_conflict_api
            .process_conversation_turn(&format_conflict_request)
            .expect("conflicting response formats");
        assert_eq!(
            format_conflict.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
        assert_eq!(
            format_conflict
                .conversation_state
                .dialogue_directive_ledger
                .active()
                .count(),
            0
        );
    }

    #[test]
    fn applied_discourse_group_update_uses_typed_generation() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-GROUP-GENERATION",
            1,
            "캐시를 확인하고 큐를 수리해",
        ))
        .expect("establish action group");
        api.process_conversation_turn(&conversation_request(
            "CHAT-GROUP-GENERATION",
            2,
            "워커를 분석해",
        ))
        .expect("establish additional action");
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-GROUP-GENERATION",
                3,
                "그 작업 묶음에 워커 작업을 추가해",
            ))
            .expect("update discourse group");
        assert!(response
            .discourse_group_update
            .as_ref()
            .is_some_and(|update| update.applied));
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::DiscourseGroupUpdate
        );
        assert_eq!(
            response.natural_realization.realization_path,
            crate::natural_realization::NaturalRealizationPathIR::Generative
        );
        assert_eq!(response.natural_realization.generation_traces.len(), 1);
        assert!(response.natural_realization.generation_traces[0].validate());
        assert_eq!(response.natural_realization.stage_overwrite_count, 0);
        assert!(response.output.text.contains("3개 대상"));
    }

    #[test]
    fn korean_topic_particle_uses_the_realized_subject_ending() {
        assert_eq!(korean_topic_particle("DNS 설정"), "은");
        assert_eq!(korean_topic_particle("CCTV 오류"), "는");
    }

    #[test]
    fn structured_realization_uses_typed_goals_without_internal_ir_or_result_claims() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-STRUCTURED-REALIZATION",
                1,
                "파일을 분석하고 폴더를 수리해",
            ))
            .expect("structured plan");
        assert!(response.output.text.contains("파일"), "{response:#?}");
        assert!(response.output.text.contains("폴더"), "{response:#?}");
        assert!(response.output.text.contains("계획"));
        assert!(response.output.text.contains("아직 실행 결과는 없"));
        assert!(!response.output.text.contains("compositional_goal_graph"));
        assert!(!response.output.text.contains("Investigate:"));
    }

    #[test]
    fn structured_realization_names_the_prohibited_goal_as_excluded() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let response = api
            .process_conversation_turn(&conversation_request(
                "CHAT-STRUCTURED-PROHIBITION",
                1,
                "캐시를 분석하되 큐는 삭제하지 마",
            ))
            .expect("bounded prohibition");
        assert!(response.output.text.contains("캐시"));
        assert!(response.output.text.contains("큐"));
        assert!(response.output.text.contains("금지"));
        assert!(response.output.text.contains("제외"));
        assert!(!response.output.text.contains("‘‘"));
        assert_eq!(
            response
                .pragmatic_interpretation
                .compositional_analysis
                .blocked_execution_count(),
            1
        );
    }

    #[test]
    fn native_circuit_prioritizes_live_goal_over_non_live_clause_events() {
        for (index, (text, intent, subject)) in [
            (
                "Unless the Dune service is healthy, repair the Ember worker; inspect the Flint report now",
                PlanIntentIR::Investigate,
                "Flint",
            ),
            (
                "Even if the Garnet cache failed, do not delete it; explain why it failed",
                PlanIntentIR::Explain,
                "Garnet",
            ),
            (
                "Garnet 캐시가 실패했더라도 그걸 삭제하지 말고 왜 실패했는지 설명해",
                PlanIntentIR::Explain,
                "Garnet",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let request = conversation_request(&format!("CHAT-NATIVE-LIVE-{index}"), 1, text);
            let response = api
                .process_conversation_turn(&request)
                .expect("native live goal");
            let goal = response
                .native_language_circuit
                .authoritative_single_live_goal()
                .expect("native selection");
            assert_eq!(goal.intent, intent, "{response:#?}");
            assert!(goal.subject.contains(subject), "{response:#?}");
            let grounded = response
                .grounded_response
                .as_deref()
                .unwrap_or_else(|| panic!("native goal must reach planner: {response:#?}"));
            assert_eq!(grounded.understanding.intent, intent, "{response:#?}");
            assert!(grounded.understanding.subject.contains(subject), "{response:#?}");
            assert!(response.conversation_state.active_goals.iter().any(|goal| {
                goal.intent == intent && goal.subject.to_lowercase().contains(&subject.to_lowercase())
            }), "native planner selection must also own the stored conversation goal: {response:#?}");
        }
    }

    #[test]
    fn native_projection_owns_goal_state_for_prohibition_and_goal_correction() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let scoped = conversation_request(
            "CHAT-NATIVE-GOAL-STATE-SCOPE",
            1,
            "the server is slow and the worker is healthy. inspect the former but never delete the latter",
        );
        let response = api
            .process_conversation_turn(&scoped)
            .expect("scoped goal state");
        assert_eq!(response.conversation_state.active_goals.len(), 1);
        assert_eq!(
            response.conversation_state.active_goals[0].intent,
            PlanIntentIR::Investigate
        );
        assert!(
            response.conversation_state.active_goals[0]
                .subject
                .to_lowercase()
                .contains("server"),
            "{response:#?}"
        );
        assert!(!response.conversation_state.active_goals[0]
            .subject
            .to_lowercase()
            .contains("worker"));

        let conversation_id = "CHAT-NATIVE-GOAL-STATE-CORRECTION";
        api.process_conversation_turn(&conversation_request(
            conversation_id,
            1,
            "Remove the cache",
        ))
        .expect("seed goal");
        let corrected = api
            .process_conversation_turn(&conversation_request(
                conversation_id,
                2,
                "No, review it rather than remove it",
            ))
            .expect("corrected goal state");
        let latest = corrected
            .conversation_state
            .active_goals
            .iter()
            .max_by_key(|goal| goal.introduced_turn)
            .expect("corrected active goal");
        assert_eq!(latest.intent, PlanIntentIR::Investigate);
        assert!(
            latest.subject.to_lowercase().contains("cache"),
            "{corrected:#?}"
        );
        assert!(!latest.subject.to_lowercase().contains("rather than"));
    }

    #[test]
    fn native_contrastive_retarget_preserves_full_response_contract() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut request = conversation_request(
            "CHAT-NATIVE-CONTRAST",
            1,
            "Not the Ivory index—the Juniper queue. Repair that one",
        );
        request.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&request)
            .expect("contrastive retarget");
        assert!(response.validate_against(&request), "response binding");
        assert!(
            response.natural_realization.validate(),
            "natural realization"
        );
        assert!(
            response.grounded_realization.validate(),
            "grounded realization"
        );
        assert_eq!(
            response.natural_realization.realized_text,
            response.output.text
        );
        assert_eq!(
            response.grounded_realization.realized_text,
            response.output.text
        );
        assert_eq!(response.output.unsupported_freeform_claims, 0);
        assert!(
            response.six_axis_integration.complete,
            "ambiguities={:#?} axes={:#?} links={:#?} invariants={:#?} violations={:#?}",
            response.reference_resolution.ambiguous_reference_surfaces,
            response.six_axis_integration.axes,
            response.six_axis_integration.cross_axis_links,
            response.six_axis_integration.cross_axis_invariants,
            response.six_axis_integration.violations
        );
        assert!(!response.six_axis_integration.semantic_authority);
        assert!(!response.six_axis_integration.language_can_execute);
        assert_eq!(response.six_axis_integration.cross_axis_links.len(), 8);
        assert!(
            response
                .six_axis_integration
                .cross_axis_links
                .iter()
                .all(|link| link.satisfied && !link.evidence_refs.is_empty()),
            "{:#?}",
            response.six_axis_integration.cross_axis_links
        );
    }

    #[test]
    fn native_operation_ellipsis_reuses_prior_goal_for_new_theme() {
        for (index, (seed, source)) in [
            (
                "Inspect the Kestrel worker",
                "Do the same to the Linen queue",
            ),
            (
                "The Kestrel worker—inspect it",
                "For the Linen queue, do the same",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let conversation_id = format!("CHAT-NATIVE-OP-ELLIPSIS-{index}");
            let seed_response = api
                .process_conversation_turn(&conversation_request(&conversation_id, 1, seed))
                .expect("seed operation");
            let seed_goal = seed_response
                .native_language_circuit
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("native seed operation: {seed_response:#?}"));
            assert_eq!(seed_goal.intent, PlanIntentIR::Investigate);
            assert!(seed_goal.subject.contains("Kestrel"));
            let seed_grounded = seed_response
                .grounded_response
                .as_deref()
                .unwrap_or_else(|| {
                    panic!("native seed operation must reach planner: {seed_response:#?}")
                });
            assert_eq!(
                seed_grounded.understanding.intent,
                PlanIntentIR::Investigate
            );
            assert!(seed_grounded
                .understanding
                .subject
                .to_lowercase()
                .contains("kestrel"));
            assert_eq!(seed_response.natural_realization.stage_overwrite_count, 0);
            if index == 1 {
                assert!(seed_response
                    .native_language_circuit
                    .reference_bindings
                    .iter()
                    .any(|binding| binding.kind == NativeReferenceKindIR::ExplicitPriorTheme));
            }
            let request = conversation_request(&conversation_id, 2, source);
            let response = api
                .process_conversation_turn(&request)
                .expect("operation ellipsis");
            let goal = response
                .native_language_circuit
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("native operation inheritance: {response:#?}"));
            assert_eq!(goal.intent, PlanIntentIR::Investigate, "{response:#?}");
            assert!(goal.subject.contains("Linen"), "{response:#?}");
            let grounded = response
                .grounded_response
                .as_deref()
                .unwrap_or_else(|| panic!("inherited operation must reach planner: {response:#?}"));
            assert_eq!(grounded.understanding.intent, PlanIntentIR::Investigate);
            assert!(grounded.understanding.subject.contains("Linen"));
            assert!(response
                .native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| binding.kind == NativeReferenceKindIR::OperationEllipsis));
            assert_eq!(response.natural_realization.stage_overwrite_count, 0);
            assert!(response.validate_against(&request));
        }
    }

    #[test]
    fn native_structural_variants_preserve_full_response_contract() {
        for (index, text) in [
            "Inspect the Flint report now; unless Dune is healthy, repair the Ember worker",
            "Context note: this concerns the current matter. Unless the Dune service is healthy, repair the Ember worker; inspect the Flint report now",
            "Navy 서비스에서 시간 초과가 반복돼. 왜 그런지 조사해",
            "Repair only the latter, but first inspect the Rose cache and the Sienna queue",
            "Context note: this concerns the current matter. Inspect the Rose cache and the Sienna queue, but repair only the latter",
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request =
                conversation_request(&format!("CHAT-NATIVE-CONTRACT-{index}"), 1, text);
            request.output_language = Some(if text
                .chars()
                .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
            {
                LanguageCodeIR::Korean
            } else {
                LanguageCodeIR::English
            });
            request.max_plan_steps = 16;
            let response = api
                .process_conversation_turn(&request)
                .expect("native structural variant");
            assert!(
                response.validate_against(&request)
                    && response.output.unsupported_freeform_claims == 0
                    && response.six_axis_integration.complete,
                "text={text}\nintegration={:#?}\nsix_axis={:#?}\nreference_ambiguities={:#?}\ncomposition_clarification={:?}\nunresolved_bindings={:#?}\nintent_ambiguities={:#?}",
                response.language_cortex_integration.violations,
                response.six_axis_integration.violations,
                response.reference_resolution.ambiguous_reference_surfaces,
                response.pragmatic_interpretation.compositional_analysis.clarification_required,
                response.pragmatic_interpretation.unresolved_bindings,
                response.pragmatic_interpretation.pragmatic_intent_graph.unresolved_ambiguities,
            );
            if !response
                .native_language_circuit
                .selected_live_goals
                .is_empty()
            {
                let native_contributions = response
                    .pragmatic_interpretation
                    .language_center
                    .contributions
                    .iter()
                    .filter(|contribution| {
                        contribution.source
                            == crate::language_center::LanguageCenterSourceIR::NativeCircuit
                    })
                    .collect::<Vec<_>>();
                assert!(
                    !native_contributions.is_empty(),
                    "native selection disappeared before LanguageCenter: {response:#?}"
                );
                assert_eq!(
                    native_contributions
                        .iter()
                        .map(|contribution| contribution.event_id.as_str())
                        .collect::<BTreeSet<_>>()
                        .len(),
                    native_contributions.len(),
                    "native contributions must not collapse repeated predicates: {response:#?}"
                );
                assert!(response
                    .pragmatic_interpretation
                    .language_center_goal_projection
                    .as_ref()
                    .is_some_and(|projection| projection.validate_against(
                        &response.pragmatic_interpretation.language_center,
                        &response.pragmatic_interpretation.compositional_analysis,
                    )));
            }
        }
    }

    #[test]
    fn noisy_colloquial_indirect_requests_link_composition_to_pragmatic_intent() {
        for (index, surface) in [
            "Um, could ya chek the Knoll service for me?",
            "Uh, can ya inspect the Lyric log?",
            "Would ya repair the Meadow queue?",
            "Did ya review the Nucleus report?",
            "Will ya insepct the Quartz cache?",
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request =
                conversation_request(&format!("CHAT-NOISY-INDIRECT-{index}"), 1, surface);
            request.modality = crate::conversation::ConversationInputModalityIR::VoiceTranscript;
            request.output_language = Some(LanguageCodeIR::English);
            let response = api
                .process_conversation_turn(&request)
                .expect("noisy indirect request");
            assert!(response.validate_against(&request), "{surface}");
            assert!(
                response
                    .pragmatic_interpretation
                    .compositional_analysis
                    .selected_candidate()
                    .is_some(),
                "composition must ground an action: {surface}"
            );
            assert!(
                response.pragmatic_interpretation.inferred_goal.is_some()
                    || response
                        .pragmatic_interpretation
                        .pragmatic_intent_graph
                        .primary
                        .is_some(),
                "pragmatic intent must consume the composition: {surface}"
            );
            assert!(
                response.six_axis_integration.complete,
                "surface={surface}\nnormalization={:#?}\nviolations={:#?}",
                response.normalization, response.six_axis_integration.violations
            );
            assert!(response
                .six_axis_integration
                .cross_axis_links
                .iter()
                .all(|link| !link.active || link.satisfied));
            assert_eq!(response.output.unsupported_freeform_claims, 0);
        }
    }

    #[test]
    fn native_result_query_keeps_language_report_separate_from_verified_result() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let conversation_id = "CHAT-NATIVE-RESULT-BOUNDARY";
        api.process_conversation_turn(&conversation_request(
            conversation_id,
            1,
            "Ocher 마이그레이션을 수행해",
        ))
        .expect("native execution plan");
        let report = api
            .process_conversation_turn(&conversation_request(
                conversation_id,
                2,
                "동료가 완료됐다고 보고했어",
            ))
            .expect("language report");
        assert_eq!(
            report
                .conversation_state
                .action_state_ledger
                .language_report_history
                .len(),
            1,
            "{report:#?}"
        );
        let request = conversation_request(
            conversation_id,
            3,
            "보고는 빼고 실제 Ocher 결과가 검증됐는지 말해줘",
        );
        let result = api
            .process_conversation_turn(&request)
            .expect("verified result query");
        assert!(result.validate_against(&request), "{result:#?}");
        assert!(
            result
                .plan_result_boundary
                .snapshots
                .iter()
                .all(|snapshot| !snapshot.verified_result),
            "{result:#?}"
        );
        assert!(result.output.text.contains("Ocher"), "{result:#?}");
        assert!(result.output.text.contains("검증"), "{result:#?}");
    }

    #[test]
    fn compact_completion_questions_route_to_verified_result_boundary_bilingually() {
        for (index, (plan_text, query_text, language)) in [
            ("Aster 캐시를 수리해", "수리했어?", LanguageCodeIR::Korean),
            (
                "Repair the Alder cache.",
                "Did you repair it?",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-COMPACT-RESULT-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut plan = conversation_request(&conversation_id, 1, plan_text);
            plan.output_language = Some(language);
            api.process_conversation_turn(&plan).expect("repair plan");
            let mut query = conversation_request(&conversation_id, 2, query_text);
            query.output_language = Some(language);
            let response = api
                .process_conversation_turn(&query)
                .expect("compact result query");
            assert!(response.validate_against(&query), "{response:#?}");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::AnswerVerifiedResult,
                "{response:#?}"
            );
            assert!(response
                .native_language_circuit
                .reference_bindings
                .iter()
                .filter(|binding| {
                    binding.kind
                        == crate::native_language_circuit::NativeReferenceKindIR::VerifiedResultTarget
                })
                .all(|binding| binding
                    .inherited_goal_id
                    .as_deref()
                    .is_some_and(|goal_id| goal_id.starts_with("GOAL-"))),
                "{response:#?}");
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::ResultAbsence,
                "{response:#?}"
            );
            assert_eq!(
                response.grounded_realization.claims.len(),
                1,
                "{response:#?}"
            );
            assert_eq!(
                response.grounded_realization.claims[0].kind,
                crate::grounded_realization::GroundedClaimKindIR::EvidenceAbsence,
                "{response:#?}"
            );
            assert_eq!(
                response.grounded_realization.claims[0].epistemic_status,
                crate::grounded_realization::ClaimEpistemicStatusIR::Unknown,
                "{response:#?}"
            );
        }
    }

    #[test]
    fn reported_completion_persists_into_later_verification_answer() {
        for (index, (plan_text, report_text, query_text, language)) in [
            (
                "Aster 캐시를 수리해",
                "내가 방금 수리했어.",
                "그럼 검증까지 된 거지?",
                LanguageCodeIR::Korean,
            ),
            (
                "Repair the Alder cache.",
                "I just repaired it myself.",
                "So it is verified now, right?",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-PERSISTED-REPORT-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            for (turn_index, text) in [(1, plan_text), (2, report_text)] {
                let mut request = conversation_request(&conversation_id, turn_index, text);
                request.output_language = Some(language);
                api.process_conversation_turn(&request)
                    .expect("preceding lifecycle turn");
            }
            let mut query = conversation_request(&conversation_id, 3, query_text);
            query.output_language = Some(language);
            let response = api
                .process_conversation_turn(&query)
                .expect("verification answer");
            assert!(response.validate_against(&query), "{response:#?}");
            assert_eq!(
                response.plan_result_boundary.query_focus,
                PlanResultQueryFocusIR::VerifiedResult,
                "{response:#?}"
            );
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanResultStatus,
                "{response:#?}"
            );
            assert!(response.grounded_realization.claims.iter().any(|claim| {
                claim.kind == crate::grounded_realization::GroundedClaimKindIR::LanguageReport
                    && claim.epistemic_status
                        == crate::grounded_realization::ClaimEpistemicStatusIR::Reported
            }));
            assert!(response.grounded_realization.claims.iter().any(|claim| {
                claim.kind == crate::grounded_realization::GroundedClaimKindIR::EvidenceAbsence
            }));
            assert!(response.grounded_realization.claims.iter().all(|claim| {
                claim.kind != crate::grounded_realization::GroundedClaimKindIR::PlanStatus
            }));
        }
    }

    #[test]
    fn ambiguous_action_qud_restores_operation_after_entity_clarification() {
        for (index, (initial_text, ambiguous_text, answer_text, language)) in [
            (
                "Aster 캐시와 Dune 큐를 확인해.",
                "그거 고쳐 줘.",
                "Dune 큐 말한 거야.",
                LanguageCodeIR::Korean,
            ),
            (
                "Check the Alder cache and the Birch queue.",
                "Fix that.",
                "I meant the Birch queue.",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-NATIVE-ACTION-QUD-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut initial = conversation_request(&conversation_id, 1, initial_text);
            initial.output_language = Some(language);
            api.process_conversation_turn(&initial)
                .expect("initial investigation");

            let mut ambiguous = conversation_request(&conversation_id, 2, ambiguous_text);
            ambiguous.output_language = Some(language);
            let clarification = api
                .process_conversation_turn(&ambiguous)
                .expect("clarification request");
            assert_eq!(
                clarification.natural_realization.response_act,
                NaturalResponseActIR::ClarificationRequest,
                "{clarification:#?}"
            );
            let question = clarification
                .conversation_state
                .pending_question
                .as_ref()
                .unwrap_or_else(|| panic!("missing action QUD: {clarification:#?}"));
            assert_eq!(question.options.len(), 2, "{clarification:#?}");
            assert!(question
                .options
                .iter()
                .all(|option| option.intent == Some(PlanIntentIR::Repair)));

            let mut answer = conversation_request(&conversation_id, 3, answer_text);
            answer.output_language = Some(language);
            let preview = api
                .conversation_memory
                .resolve_pending_question(&conversation_id, answer_text);
            assert_eq!(
                preview.disposition,
                QuestionAnswerDispositionIR::Resolved,
                "question={question:#?}; preview={preview:#?}"
            );
            let resolved = api
                .process_conversation_turn(&answer)
                .expect("resolved clarification");
            assert!(resolved.validate_against(&answer), "{resolved:#?}");
            assert!(
                resolved
                    .reference_resolution
                    .discourse_bindings
                    .iter()
                    .any(|binding| binding.kind == DiscourseBindingKindIR::ClarificationAnswer),
                "{resolved:#?}"
            );
            let goal = resolved
                .native_language_circuit
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("missing rebound native goal: {resolved:#?}"));
            assert_eq!(goal.intent, PlanIntentIR::Repair, "{resolved:#?}");
            assert!(
                goal.subject
                    .to_lowercase()
                    .contains(if index == 0 { "dune" } else { "birch" }),
                "{resolved:#?}"
            );
            assert!(resolved
                .native_language_circuit
                .reference_bindings
                .iter()
                .any(|binding| {
                    binding.kind
                        == crate::native_language_circuit::NativeReferenceKindIR::ClarificationAnswer
                }));
        }
    }

    #[test]
    fn contrastive_ordinal_retarget_resolves_prohibited_plural_without_guessing() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-CONTRASTIVE-ORDINAL",
            1,
            "Repair the Alder cache and the Birch queue.",
        ))
        .expect("initial coordinated repair");
        let request = conversation_request(
            "CHAT-CONTRASTIVE-ORDINAL",
            2,
            "No—do not repair them. Explain only the cause of the first one.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("contrastive retarget");
        assert!(response.validate_against(&request), "{response:#?}");
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{response:#?}"
        );
        assert_eq!(
            response
                .natural_realization
                .response_arbitration
                .selected_source,
            NaturalResponseSourceIR::NativePlan,
            "{response:#?}"
        );
        assert!(response
            .natural_realization
            .response_arbitration
            .candidates
            .iter()
            .any(|candidate| candidate.source == NaturalResponseSourceIR::Fallback));
        assert!(
            response
                .reference_resolution
                .ambiguous_reference_surfaces
                .is_empty(),
            "{response:#?}"
        );
        assert!(response
            .native_language_circuit
            .reference_bindings
            .iter()
            .any(|binding| binding.kind == NativeReferenceKindIR::ContrastiveRetarget));
    }

    #[test]
    fn native_projection_prevents_legacy_discourse_nouns_from_overwriting_typed_themes() {
        for (index, (text, expected, rejected)) in [
            ("Aster 캐시 말인데, 좀 확인해 줄래?", "Aster", "말인데"),
            (
                "Birch 로그는 지금 조사하되 캐시가 오래됐을 때만 Cedar 큐를 수리해",
                "Birch",
                "오래됐",
            ),
            (
                "Navy 서비스에서 시간 초과가 반복돼. 왜 그런지 조사해",
                "Navy",
                "원인",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let request = conversation_request(&format!("CHAT-NATIVE-PROJECTION-{index}"), 1, text);
            let response = api
                .process_conversation_turn(&request)
                .expect("typed native projection");
            assert!(response.validate_against(&request), "{response:#?}");
            let understanding = &response
                .grounded_response
                .as_ref()
                .unwrap_or_else(|| panic!("missing grounded response: {response:#?}"))
                .understanding;
            assert!(understanding.subject.contains(expected), "{response:#?}");
            assert!(!understanding.subject.contains(rejected), "{response:#?}");
        }
    }

    #[test]
    fn ordinal_topic_return_prefers_ordered_goal_history_over_latest_entity_focus() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-ORDINAL-TOPIC-RETURN",
            1,
            "Inspect the Parchment cache.",
        ))
        .expect("first issue");
        api.process_conversation_turn(&conversation_request(
            "CHAT-ORDINAL-TOPIC-RETURN",
            2,
            "Inspect the Quartz queue.",
        ))
        .expect("second issue");
        let request = conversation_request(
            "CHAT-ORDINAL-TOPIC-RETURN",
            3,
            "Go back to the first issue and explain why it failed.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("ordinal topic return");
        assert!(response.validate_against(&request), "{response:#?}");
        let goal = response
            .native_language_circuit
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("missing ordinal goal: {response:#?}"));
        assert_eq!(goal.intent, PlanIntentIR::Explain);
        assert!(
            goal.subject.to_lowercase().contains("parchment"),
            "{response:#?}"
        );
        assert!(
            !goal.subject.to_lowercase().contains("quartz"),
            "{response:#?}"
        );
    }

    #[test]
    fn first_person_completion_report_remains_reported_not_verified() {
        for (index, (plan_text, report_text, language)) in [
            (
                "Aster 캐시를 수리해",
                "내가 방금 수리했어.",
                LanguageCodeIR::Korean,
            ),
            (
                "Repair the Alder cache.",
                "I just repaired it myself.",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-FIRST-PERSON-REPORT-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut plan = conversation_request(&conversation_id, 1, plan_text);
            plan.output_language = Some(language);
            api.process_conversation_turn(&plan).expect("repair plan");
            let mut report = conversation_request(&conversation_id, 2, report_text);
            report.output_language = Some(language);
            let response = api
                .process_conversation_turn(&report)
                .expect("completion report");
            assert!(response.validate_against(&report), "{response:#?}");
            assert!(response.action_state_analysis.has_language_reports());
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::ActionState
            );
            assert!(response
                .conversation_state
                .action_state_ledger
                .records
                .iter()
                .all(|record| record.execution_status == ActionExecutionStatusIR::NotObserved));
        }
    }

    #[test]
    fn gratitude_with_social_adjunct_stays_a_social_response() {
        for (index, (text, language)) in [
            ("말이라도 고맙다.", LanguageCodeIR::Korean),
            ("Thanks for saying that.", LanguageCodeIR::English),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request = conversation_request(&format!("CHAT-GRATITUDE-{index}"), 1, text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .expect("social gratitude response");
            assert!(response.validate_against(&request), "{response:#?}");
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::SocialBackchannel,
                "{response:#?}"
            );
        }
    }

    #[test]
    fn exhaustion_and_causal_ellipsis_keep_distinct_affect_and_plan_paths() {
        let mut affect_api = CognitiveApi::new_embedded().unwrap();
        let affect_request = conversation_request(
            "CHAT-EXHAUSTION",
            1,
            "Aster 캐시가 계속 깨져서 진짜 지친다.",
        );
        let affect = affect_api
            .process_conversation_turn(&affect_request)
            .expect("affect support");
        assert_eq!(
            affect.natural_realization.response_act,
            NaturalResponseActIR::AffectSupport,
            "{affect:#?}"
        );

        let mut plan_api = CognitiveApi::new_embedded().unwrap();
        plan_api
            .process_conversation_turn(&conversation_request(
                "CHAT-CAUSAL-ELLIPSIS",
                1,
                "Aster 캐시를 확인해.",
            ))
            .expect("initial investigation");
        let followup_request =
            conversation_request("CHAT-CAUSAL-ELLIPSIS", 2, "응, 왜 그런지 알아봐 줘.");
        let followup = plan_api
            .process_conversation_turn(&followup_request)
            .expect("causal ellipsis investigation");
        assert!(
            followup.validate_against(&followup_request),
            "{followup:#?}"
        );
        assert_eq!(
            followup.native_language_circuit.response_goal,
            NativeResponseGoalIR::PlanActions,
            "{followup:#?}"
        );
        assert_eq!(
            followup.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{followup:#?}"
        );
    }

    #[test]
    fn affect_and_request_are_composed_without_losing_the_primary_goal() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let request = conversation_request(
            "CHAT-AFFECT-PLUS-REQUEST",
            1,
            "계속 실패해서 너무 답답해. Cedar 큐 원인을 좁히는 걸 도와줘.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("composed affect and task response");
        assert!(response.validate_against(&request), "{response:#?}");
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{response:#?}"
        );
        assert_eq!(
            response
                .natural_realization
                .response_plan
                .moves
                .iter()
                .map(|response_move| response_move.response_act)
                .collect::<Vec<_>>(),
            vec![
                NaturalResponseActIR::AffectSupport,
                NaturalResponseActIR::PlanPreview,
            ],
            "{response:#?}"
        );
        assert!(response.output.text.contains("답답"));
        assert!(response
            .native_language_circuit
            .selected_live_goals
            .iter()
            .any(|goal| goal.intent == PlanIntentIR::Investigate));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }

    #[test]
    fn feedback_and_corrective_request_remain_two_ordered_response_moves() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let request = conversation_request(
            "CHAT-FEEDBACK-PLUS-REQUEST",
            1,
            "그 답변은 핵심을 놓쳤어. Cedar 큐 원인을 설명해 줘.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("composed feedback and task response");
        assert!(response.validate_against(&request), "{response:#?}");
        assert_eq!(
            response.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview,
            "{response:#?}"
        );
        assert_eq!(
            response
                .natural_realization
                .response_plan
                .moves
                .iter()
                .map(|response_move| response_move.response_act)
                .collect::<Vec<_>>(),
            vec![
                NaturalResponseActIR::UserFeedback,
                NaturalResponseActIR::PlanPreview,
            ],
            "{response:#?}"
        );
        assert!(response
            .native_language_circuit
            .selected_live_goals
            .iter()
            .any(|goal| goal.intent == PlanIntentIR::Explain));
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }

    #[test]
    fn context_restored_goal_displaces_same_intent_surface_placeholders_once() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-CONTEXT-GOAL-SINGLE-OWNER",
            1,
            "Explain how the Cedar scheduler decides priority.",
        ))
        .expect("context goal");
        let request = conversation_request(
            "CHAT-CONTEXT-GOAL-SINGLE-OWNER",
            2,
            "너무 길어. 핵심만 다시 설명해.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("context-restored explanation");
        let grounded = response
            .grounded_response
            .as_deref()
            .expect("grounded contextual plan");
        assert_eq!(grounded.semantic_goal.selected_live_event_ids.len(), 1);
        assert_eq!(grounded.semantic_plan_bundle.plans.len(), 1);
        assert!(grounded
            .understanding
            .subject
            .to_lowercase()
            .contains("cedar"));
        assert_eq!(
            response
                .natural_realization
                .coverage
                .obligations
                .iter()
                .filter(|obligation| {
                    obligation.kind
                        == crate::natural_realization::NaturalRealizationObligationKindIR::SelectedPlanEvent
                })
                .count(),
            1
        );
        assert!(response.validate_against(&request), "{response:#?}");
    }

    #[test]
    fn action_nominal_and_light_verb_constructions_reach_goal_ir_without_overwrite() {
        for (index, (text, language, expected_intent, expected_subject)) in [
            (
                "Prepare restoration steps for the Saffron queue.",
                LanguageCodeIR::English,
                PlanIntentIR::Repair,
                "Saffron",
            ),
            (
                "Give the Topaz worker inspection first priority.",
                LanguageCodeIR::English,
                PlanIntentIR::Investigate,
                "Topaz",
            ),
            (
                "Walk us through what you would examine in the Umber cache.",
                LanguageCodeIR::English,
                PlanIntentIR::Explain,
                "Umber",
            ),
            (
                "Violet 서비스 상태를 파악하는 계획을 잡아 줘.",
                LanguageCodeIR::Korean,
                PlanIntentIR::Investigate,
                "Violet",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request =
                conversation_request(&format!("CHAT-ACTION-NOMINAL-{index}"), 1, text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .unwrap_or_else(|error| panic!("surface={text}; error={error:?}"));
            assert!(response.validate_against(&request), "{response:#?}");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::PlanActions,
                "surface={text}; response={response:#?}"
            );
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanPreview,
                "surface={text}; response={response:#?}"
            );
            let goal = response
                .native_language_circuit
                .authoritative_single_live_goal()
                .unwrap_or_else(|| panic!("missing live goal: {response:#?}"));
            assert_eq!(goal.intent, expected_intent, "surface={text}");
            assert!(
                goal.subject.contains(expected_subject),
                "surface={text}; goal={goal:#?}"
            );
            assert!(!goal.semantic_authority);
            assert!(!goal.external_execution_authorized);
        }
    }

    #[test]
    fn plan_only_and_evidence_status_turns_answer_the_prior_lifecycle_without_new_goals() {
        for (index, (query, language)) in [
            (
                "We only have a Wisteria cache plan, not an outcome, correct?",
                LanguageCodeIR::English,
            ),
            (
                "If evidence is absent, state that no fact is established for the Wisteria cache.",
                LanguageCodeIR::English,
            ),
            (
                "검증 근거가 없으면 Wisteria 캐시에 확립된 사실이 없다고 답해.",
                LanguageCodeIR::Korean,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-RESULT-STATUS-CONSTRUCTION-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut plan = conversation_request(
                &conversation_id,
                1,
                if language == LanguageCodeIR::Korean {
                    "Wisteria 캐시를 점검해."
                } else {
                    "Inspect the Wisteria cache."
                },
            );
            plan.output_language = Some(language);
            api.process_conversation_turn(&plan).expect("initial plan");

            let mut request = conversation_request(&conversation_id, 2, query);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .unwrap_or_else(|error| panic!("surface={query}; error={error:?}"));
            assert!(response.validate_against(&request), "{response:#?}");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::AnswerVerifiedResult,
                "surface={query}; response={response:#?}"
            );
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::ResultAbsence,
                "surface={query}; response={response:#?}"
            );
            assert!(response
                .native_language_circuit
                .selected_live_goals
                .is_empty());
            assert!(response
                .plan_result_boundary
                .snapshots
                .iter()
                .all(|snapshot| !snapshot.verified_result));
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
        }
    }

    #[test]
    fn correction_and_ordinal_explanation_preserve_the_requested_act_and_rebind_the_theme() {
        let mut correction_api = CognitiveApi::new_embedded().unwrap();
        correction_api
            .process_conversation_turn(&conversation_request(
                "CHAT-NOMINAL-CORRECTION",
                1,
                "Inspect the Amber cache and explain the Cobalt worker.",
            ))
            .expect("initial paired request");
        let correction_request = conversation_request(
            "CHAT-NOMINAL-CORRECTION",
            2,
            "Let me correct that: the explanation should cover the Amber cache.",
        );
        let correction = correction_api
            .process_conversation_turn(&correction_request)
            .expect("corrected explanation");
        assert!(
            correction.validate_against(&correction_request),
            "{correction:#?}"
        );
        assert_eq!(
            correction
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidates()
                .len(),
            1,
            "analysis={:#?}",
            correction.pragmatic_interpretation.compositional_analysis
        );
        assert_eq!(
            correction
                .pragmatic_interpretation
                .compositional_analysis
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.disposition
                        == crate::compositional_semantics::CandidateDispositionIR::Viable
                })
                .count(),
            1,
            "analysis={:#?}",
            correction.pragmatic_interpretation.compositional_analysis
        );
        assert!(
            correction
                .pragmatic_interpretation
                .compositional_analysis
                .selected_candidates()[0]
                .external_execution_authorized,
            "analysis={:#?}",
            correction.pragmatic_interpretation.compositional_analysis
        );
        assert!(
            correction
                .pragmatic_interpretation
                .compositional_analysis
                .modal_scope_graph
                .conditionals
                .is_empty(),
            "analysis={:#?}",
            correction.pragmatic_interpretation.compositional_analysis
        );
        assert!(
            correction
                .pragmatic_interpretation
                .compositional_analysis
                .goal_graph
                .as_ref()
                .is_none_or(|graph| graph.conditions.is_empty() && graph.prohibitions.is_empty()),
            "analysis={:#?}",
            correction.pragmatic_interpretation.compositional_analysis
        );
        let corrected_goal = correction
            .native_language_circuit
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("missing corrected goal: {correction:#?}"));
        assert_eq!(corrected_goal.intent, PlanIntentIR::Explain);
        assert!(corrected_goal.subject.contains("Amber"), "{correction:#?}");
        assert!(!corrected_goal.subject.contains("Cobalt"));

        let mut ordinal_api = CognitiveApi::new_embedded().unwrap();
        ordinal_api
            .process_conversation_turn(&conversation_request(
                "CHAT-ORDINAL-EXPLANATION",
                1,
                "Inspect the Dune queue and the Elm relay.",
            ))
            .expect("ordered targets");
        let ordinal_request = conversation_request(
            "CHAT-ORDINAL-EXPLANATION",
            2,
            "Explain the reason for examining the second target.",
        );
        let ordinal = ordinal_api
            .process_conversation_turn(&ordinal_request)
            .expect("ordinal explanation");
        assert!(ordinal.validate_against(&ordinal_request), "{ordinal:#?}");
        let ordinal_goal = ordinal
            .native_language_circuit
            .authoritative_single_live_goal()
            .unwrap_or_else(|| panic!("missing ordinal goal: {ordinal:#?}"));
        assert_eq!(ordinal_goal.intent, PlanIntentIR::Explain);
        assert!(ordinal_goal.subject.contains("Elm"), "{ordinal:#?}");
        assert!(!ordinal_goal.subject.contains("Dune"));
    }

    #[test]
    fn coordinated_conflicting_reports_remain_epistemic_and_answer_certainty_questions() {
        for (index, (report, certainty, outcome, language)) in [
            (
                "민수는 Aster 캐시가 성공했다고 했는데 지수는 실패했다고 했어.",
                "그럼 지금 확실한 건 뭐야?",
                "그래서 성공한 거야, 실패한 거야?",
                LanguageCodeIR::Korean,
            ),
            (
                "Mina says the Alder cache succeeded, but Jisoo says it failed.",
                "Then what is certain right now?",
                "So did it succeed or fail?",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-COORDINATED-CONFLICT-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut initial = conversation_request(&conversation_id, 1, report);
            initial.output_language = Some(language);
            let initial = api
                .process_conversation_turn(&initial)
                .expect("coordinated report");
            assert!(
                initial.grounded_response.is_none(),
                "index={index} speech={:?} native={:?} act={:?} selected={:?}",
                initial.pragmatic_interpretation.speech_act,
                initial.native_language_circuit.response_goal,
                initial.natural_realization.response_act,
                initial
                    .pragmatic_interpretation
                    .compositional_analysis
                    .selected_candidate_ids
            );
            assert_eq!(initial.conversation_state.epistemic_ledger.records.len(), 2);
            assert!(
                initial
                    .conversation_state
                    .epistemic_ledger
                    .records
                    .iter()
                    .all(
                        |record| record.status == crate::epistemic::BeliefRecordStatusIR::Contested
                    ),
                "index={index} records={:?}",
                initial
                    .conversation_state
                    .epistemic_ledger
                    .records
                    .iter()
                    .map(|record| (
                        record.source_actor.as_str(),
                        record.proposition_surface.as_str(),
                        record.status,
                        record.signature.subject_key.as_str(),
                        record.signature.state_axis.as_deref(),
                        &record.signature.state_value,
                    ))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                initial.natural_realization.response_act,
                NaturalResponseActIR::ActionState,
                "{initial:#?}"
            );
            assert!(initial.grounded_realization.claims.iter().any(|claim| {
                claim.kind == crate::grounded_realization::GroundedClaimKindIR::LanguageReport
                    && claim.epistemic_status
                        == crate::grounded_realization::ClaimEpistemicStatusIR::Reported
            }));
            assert!(initial.grounded_realization.claims.iter().any(|claim| {
                claim.kind == crate::grounded_realization::GroundedClaimKindIR::EvidenceAbsence
            }));
            assert!(
                initial.output.text.to_lowercase().contains(
                    if language == LanguageCodeIR::Korean {
                        "충돌"
                    } else {
                        "conflict"
                    }
                ),
                "{initial:#?}"
            );

            for (turn_index, text, expected_act) in [
                (2, certainty, NaturalResponseActIR::DiscourseAnswer),
                (3, outcome, NaturalResponseActIR::ResultAbsence),
            ] {
                let mut query = conversation_request(&conversation_id, turn_index, text);
                query.output_language = Some(language);
                let answer = api
                    .process_conversation_turn(&query)
                    .expect("certainty answer");
                assert_eq!(
                    answer.native_language_circuit.response_goal,
                    NativeResponseGoalIR::AnswerVerifiedResult
                );
                assert_eq!(
                    answer.natural_realization.response_act,
                    expected_act,
                    "index={index} turn={turn_index} disposition={:?} native={:?} refs={:?} output={}",
                    answer.disposition,
                    answer.native_language_circuit.response_goal,
                    answer.reference_resolution.ambiguous_reference_surfaces,
                    answer.output.text
                );
                assert!(answer.discourse_answer.is_some());
                assert!(!answer
                    .grounded_realization
                    .claims
                    .iter()
                    .any(|claim| claim.verified));
            }
        }
    }

    #[test]
    fn epistemic_record_updates_are_stored_as_reports_not_action_plans() {
        for (index, (text, language)) in [
            (
                "Dune 큐에는 실패 기록이 있다고 추가해 둠.",
                LanguageCodeIR::Korean,
            ),
            (
                "Add that the Birch queue contains a failure record.",
                LanguageCodeIR::English,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request = conversation_request(&format!("CHAT-RECORD-UPDATE-{index}"), 1, text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .expect("epistemic record update");
            assert!(response.grounded_response.is_none(), "index={index}");
            assert!(response.conversation_state.active_goals.is_empty());
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::InformAcknowledgement,
                "index={index} output={}",
                response.output.text
            );
            assert!(response
                .conversation_state
                .epistemic_ledger
                .records
                .iter()
                .all(|record| !record.dialogue_truth_established));
        }
    }

    #[test]
    fn atomic_predicate_requests_materialize_one_native_goal_without_templates() {
        for (index, (text, language, expected_intent)) in [
            (
                "Arrange the Birch queue before the Cedar cache.",
                LanguageCodeIR::English,
                PlanIntentIR::Plan,
            ),
            (
                "Design a recovery procedure for the Harbor worker.",
                LanguageCodeIR::English,
                PlanIntentIR::Plan,
            ),
            (
                "Walk through the Amber service layout.",
                LanguageCodeIR::English,
                PlanIntentIR::Explain,
            ),
            (
                "Delta 로그를 관찰만 해.",
                LanguageCodeIR::Korean,
                PlanIntentIR::Investigate,
            ),
            (
                "Harbor 워커 복구 절차를 설계해.",
                LanguageCodeIR::Korean,
                PlanIntentIR::Plan,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request =
                conversation_request(&format!("CHAT-ATOMIC-PREDICATE-{index}"), 1, text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .expect("atomic predicate request");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::PlanActions,
                "text={text} response={response:#?}"
            );
            assert!(response
                .native_language_circuit
                .selected_live_goals
                .iter()
                .any(|goal| goal.intent == expected_intent));
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanPreview,
                "text={text} output={}",
                response.output.text
            );
            assert!(response.six_axis_integration.complete);
            assert_eq!(response.language_cortex_integration.external_llm_calls, 0);
            assert_eq!(
                response
                    .language_cortex_integration
                    .recursive_source_mutations,
                0
            );
        }
    }

    #[test]
    fn atomic_predicate_descriptions_remain_non_executable() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let mut request = conversation_request(
            "CHAT-ATOMIC-PREDICATE-DESCRIPTION",
            1,
            "Mina described how Rowan could organize the Birch queue.",
        );
        request.output_language = Some(LanguageCodeIR::English);
        let response = api
            .process_conversation_turn(&request)
            .expect("descriptive predicate");
        assert!(response
            .native_language_circuit
            .selected_live_goals
            .is_empty());
        assert_ne!(
            response.natural_realization.response_act,
            NaturalResponseActIR::PlanPreview
        );
        assert!(response.conversation_state.active_goals.is_empty());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }

    #[test]
    fn benefactive_and_lead_in_requests_reach_goal_ir_compositionally() {
        for (index, (text, language)) in [
            (
                "Please help us narrow down the Harbor worker failures.",
                LanguageCodeIR::English,
            ),
            (
                "Okay—focus on separating the Birch cache causes.",
                LanguageCodeIR::English,
            ),
            ("오류 원인을 좁히는 걸 도와줘.", LanguageCodeIR::Korean),
            ("음, Cedar 큐 원인에 집중해.", LanguageCodeIR::Korean),
        ]
        .into_iter()
        .enumerate()
        {
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut request =
                conversation_request(&format!("CHAT-BENEFACTIVE-REQUEST-{index}"), 1, text);
            request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&request)
                .expect("benefactive request");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::PlanActions,
                "text={text} response={response:#?}"
            );
            assert!(response
                .native_language_circuit
                .selected_live_goals
                .iter()
                .any(|goal| goal.intent == PlanIntentIR::Investigate));
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::PlanPreview,
                "text={text} output={}",
                response.output.text
            );
        }
    }

    #[test]
    fn typed_lifecycle_queries_refine_the_native_response_goal_without_claiming_results() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let initial_request =
            conversation_request("CHAT-TYPED-BOUNDARY-QUERY", 1, "Inspect the Quartz relay.");
        let initial = api
            .process_conversation_turn(&initial_request)
            .expect("initial Quartz plan");
        assert_eq!(
            initial.native_language_circuit.response_goal,
            NativeResponseGoalIR::PlanActions
        );

        let query_request = conversation_request(
            "CHAT-TYPED-BOUNDARY-QUERY",
            2,
            "Is an outcome established for the Quartz relay?",
        );
        let query = api
            .process_conversation_turn(&query_request)
            .expect("typed lifecycle query");
        assert_eq!(
            query.native_language_circuit.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult,
            "{query:#?}"
        );
        assert_eq!(
            query.native_language_circuit.response_mode,
            NativeResponseModeIR::EvidenceResultQuery
        );
        assert_eq!(
            query.natural_realization.response_act,
            NaturalResponseActIR::ResultAbsence,
            "{}",
            query.output.text
        );
        assert!(query
            .conversation_state
            .action_state_ledger
            .records
            .iter()
            .all(|record| !record.verified_outcome));
        assert!(query
            .grounded_realization
            .claims
            .iter()
            .all(|claim| !claim.verified));
        assert!(!query.language_cortex_integration.external_action_executed);
    }

    #[test]
    fn typed_user_reports_refine_response_boundary_but_not_execution_state() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        api.process_conversation_turn(&conversation_request(
            "CHAT-TYPED-BOUNDARY-REPORT",
            1,
            "Review the Quartz relay.",
        ))
        .expect("initial Quartz review");

        let report_request = conversation_request(
            "CHAT-TYPED-BOUNDARY-REPORT",
            2,
            "I wrapped up the Quartz relay task.",
        );
        let report = api
            .process_conversation_turn(&report_request)
            .expect("typed user report");
        assert!(
            report.action_state_analysis.has_language_reports(),
            "{report:#?}"
        );
        assert_eq!(
            report.native_language_circuit.response_goal,
            NativeResponseGoalIR::AnswerVerifiedResult
        );
        assert_eq!(
            report.native_language_circuit.response_mode,
            NativeResponseModeIR::ReportedOutcome
        );
        assert_eq!(
            report.natural_realization.response_act,
            NaturalResponseActIR::ActionState
        );
        let record = report
            .conversation_state
            .action_state_ledger
            .records
            .last()
            .expect("Quartz action record");
        assert_eq!(
            record.reported_status,
            Some(ActionReportedStatusIR::SuccessClaimed)
        );
        assert_eq!(
            record.execution_status,
            ActionExecutionStatusIR::NotObserved
        );
        assert!(!record.verified_outcome);
        assert!(!report.language_cortex_integration.external_action_executed);
    }

    #[test]
    fn response_boundary_refinement_does_not_reclassify_plain_information() {
        let mut api = CognitiveApi::new_embedded().unwrap();
        let request = conversation_request(
            "CHAT-TYPED-BOUNDARY-NEGATIVE",
            1,
            "The Quartz relay documentation mentions a possible outcome.",
        );
        let response = api
            .process_conversation_turn(&request)
            .expect("plain informational turn");
        assert_eq!(
            response.native_language_circuit.response_goal,
            NativeResponseGoalIR::Acknowledge,
            "{response:#?}"
        );
        assert_ne!(
            response.natural_realization.response_act,
            NaturalResponseActIR::ResultAbsence
        );
        assert!(response.conversation_state.active_goals.is_empty());
        assert!(
            !response
                .language_cortex_integration
                .external_action_executed
        );
    }

    #[test]
    fn future_evidence_notifications_acknowledge_policy_instead_of_answering_current_state() {
        for (index, (plan, notification, language)) in [
            (
                "Inspect the Harbor worker.",
                "Let me know when the Harbor outcome is confirmed.",
                LanguageCodeIR::English,
            ),
            (
                "Harbor 워커를 확인해.",
                "확인된 근거가 생기면 Harbor 워커 건을 알려 줘.",
                LanguageCodeIR::Korean,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let conversation_id = format!("CHAT-FUTURE-EVIDENCE-NOTIFICATION-{index}");
            let mut api = CognitiveApi::new_embedded().unwrap();
            let mut plan_request = conversation_request(&conversation_id, 1, plan);
            plan_request.output_language = Some(language);
            api.process_conversation_turn(&plan_request)
                .expect("initial plan");

            let mut notification_request = conversation_request(&conversation_id, 2, notification);
            notification_request.output_language = Some(language);
            let response = api
                .process_conversation_turn(&notification_request)
                .expect("future evidence notification");
            assert_eq!(
                response.native_language_circuit.response_goal,
                NativeResponseGoalIR::Acknowledge,
                "text={notification} response={response:#?}"
            );
            assert_eq!(
                response.natural_realization.response_act,
                NaturalResponseActIR::InformAcknowledgement,
                "text={notification} output={}",
                response.output.text
            );
            assert!(!response
                .grounded_realization
                .claims
                .iter()
                .any(|claim| claim.verified));
            assert!(
                !response
                    .language_cortex_integration
                    .external_action_executed
            );
        }
    }
}
