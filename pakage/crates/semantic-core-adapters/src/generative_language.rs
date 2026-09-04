//! Explainable language generation from language-independent meaning graphs.
//!
//! The pipeline stores knowledge for constructing utterances, not completed
//! sentences.  Every stage appends a typed IR derived from the preceding IR;
//! no stage mutates or reparses an earlier stage's output.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::attribution::EpistemicStatusIR;
use crate::conditional_guard::{
    ConditionalGuardEvaluationIR, GuardEvidencePolarityIR, GuardStatusIR,
    CONDITIONAL_GUARD_EVALUATION_SCHEMA,
};
use crate::conversation::{DiscourseTopicAnchorKindIR, TopicTransitionIR, TopicTransitionKindIR};
use crate::definition_grounding::{DefinitionGroundingDispositionIR, DefinitionGroundingIR};
use crate::discourse_qa::{
    DiscourseAnswerDispositionIR, DiscourseAnswerEvidenceIR, DiscourseAnswerIR,
    DiscourseQueryKindIR,
};
use crate::discourse_relations::{
    DialogueRelationAnswerDispositionIR, DialogueRelationAnswerIR, DialogueRelationKindIR,
    DialogueRelationQueryKindIR,
};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};
use crate::modality::ModalWorldIR;
use crate::pragmatics::{
    CommitmentActivationIR, GoalWithdrawalScopeIR, IllocutionaryCommitmentGraphIR,
    IllocutionaryForceIR,
};
use crate::temporal::{
    TemporalAnswerDispositionIR, TemporalAnswerIR, TemporalQueryKindIR, TemporalRelationKindIR,
};

#[path = "world_realization.rs"]
mod world_realization;
pub(crate) use world_realization::generate_world_clarification;
pub(crate) use world_realization::generate_world_decision;
pub(crate) use world_realization::generate_world_memory_update;

pub const GENERATION_MEANING_SCHEMA: &str = "B_CORE_GENERATION_MEANING_IR_1";
pub const GENERATIVE_LANGUAGE_SCHEMA: &str = "B_CORE_GENERATIVE_LANGUAGE_IR_2";

// The bounded dialogue-relation engine can return up to 48 typed evidence
// edges (8 paths × 6 hops). Each edge is preserved as an event plus two
// endpoint nodes, with room for bounded path and safety-boundary nodes.
const MAX_GENERATION_NODES: usize = 160;
const MAX_GENERATION_EDGES: usize = 320;
const MAX_REALIZED_CHARS: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationMeaningNodeKindIR {
    Event,
    Entity,
    State,
    Quality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationMeaningRelationIR {
    Agent,
    Theme,
    Goal,
    Property,
    Possessor,
    Sequence,
    Contrast,
    Negates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMeaningNodeIR {
    pub node_id: String,
    pub concept_id: String,
    pub kind: GenerationMeaningNodeKindIR,
    pub grounding_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMeaningEdgeIR {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation: GenerationMeaningRelationIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMeaningGraphIR {
    pub schema: String,
    pub nodes: Vec<GenerationMeaningNodeIR>,
    pub edges: Vec<GenerationMeaningEdgeIR>,
    pub semantic_sha256: String,
}

impl GenerationMeaningGraphIR {
    pub fn new(nodes: Vec<GenerationMeaningNodeIR>, edges: Vec<GenerationMeaningEdgeIR>) -> Self {
        let mut graph = Self {
            schema: GENERATION_MEANING_SCHEMA.to_string(),
            nodes,
            edges,
            semantic_sha256: String::new(),
        };
        graph.semantic_sha256 = generation_meaning_sha256(&graph);
        graph
    }

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
        self.schema == GENERATION_MEANING_SCHEMA
            && !self.nodes.is_empty()
            && self.nodes.len() <= MAX_GENERATION_NODES
            && self.edges.len() <= MAX_GENERATION_EDGES
            && node_ids.len() == self.nodes.len()
            && edge_ids.len() == self.edges.len()
            && self.nodes.iter().all(|node| {
                !node.node_id.trim().is_empty()
                    && !node.concept_id.trim().is_empty()
                    && !node.grounding_refs.is_empty()
                    && node
                        .grounding_refs
                        .iter()
                        .all(|evidence| !evidence.trim().is_empty())
            })
            && self.edges.iter().all(|edge| {
                !edge.edge_id.trim().is_empty()
                    && edge.source_node_id != edge.target_node_id
                    && node_ids.contains(edge.source_node_id.as_str())
                    && node_ids.contains(edge.target_node_id.as_str())
            })
            && self.semantic_sha256 == generation_meaning_sha256(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationSpeechIntentIR {
    Acknowledge,
    CommitFutureAction,
    Advise,
    Invite,
    Inform,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationTenseIR {
    Present,
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationEmotionIR {
    Neutral,
    Warm,
    Concerned,
    Playful,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GenerationAffectKindIR {
    Frustrated,
    Angry,
    Worried,
    Hurt,
    Annoyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationPlanInterpretationKindIR {
    Suggestion,
    ImplicitInvestigation,
    ImplicitRepair,
    ImplicitExplanation,
    ImplicitPlanning,
    SarcasmBoundary,
    FigurativeBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationDialogueResponseKindIR {
    HoldFloor,
    Greeting,
    Gratitude,
    Farewell,
    Backchannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationClarificationKindIR {
    PendingChoice,
    OrderedPair,
    LocalOrdinal,
    EventOrdinal,
    PreviousTopic,
    CompetingRequest,
    NonliteralReading,
    VoiceAlternative,
    Reference,
    MissingDetails,
}

impl GenerationClarificationKindIR {
    fn concept_id(self) -> &'static str {
        match self {
            Self::PendingChoice => "C_CLARIFY_PENDING_CHOICE",
            Self::OrderedPair => "C_CLARIFY_ORDERED_PAIR",
            Self::LocalOrdinal => "C_CLARIFY_LOCAL_ORDINAL",
            Self::EventOrdinal => "C_CLARIFY_EVENT_ORDINAL",
            Self::PreviousTopic => "C_CLARIFY_PREVIOUS_TOPIC",
            Self::CompetingRequest => "C_CLARIFY_COMPETING_REQUEST",
            Self::NonliteralReading => "C_CLARIFY_NONLITERAL_READING",
            Self::VoiceAlternative => "C_CLARIFY_VOICE_ALTERNATIVE",
            Self::Reference => "C_RESOLVE_REFERENCE",
            Self::MissingDetails => "C_CLARIFY_MISSING_DETAILS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationContinuationGateFollowupIR {
    PendingDecision,
    ProxyEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationUserFeedbackKindIR {
    Unhelpful,
    Misunderstood,
    MissedPoint,
    TooVerbose,
    TooBrief,
    Incorrect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationDiscourseGroupUpdateKindIR {
    AddMember,
    RemoveMember,
    MergeGroups,
}

impl GenerationDiscourseGroupUpdateKindIR {
    fn operation_concept_id(self) -> &'static str {
        match self {
            Self::AddMember => "C_GROUP_ADD_MEMBER",
            Self::RemoveMember => "C_GROUP_REMOVE_MEMBER",
            Self::MergeGroups => "C_GROUP_MERGE",
        }
    }

    fn target_concept_id(self) -> &'static str {
        match self {
            Self::AddMember | Self::RemoveMember => "C_REFERENCED_MEMBER",
            Self::MergeGroups => "C_TWO_DISCOURSE_GROUPS",
        }
    }

    fn group_concept_id(self) -> &'static str {
        match self {
            Self::AddMember | Self::RemoveMember => "C_DISCOURSE_GROUP",
            Self::MergeGroups => "C_NEW_DISCOURSE_GROUP",
        }
    }
}

impl GenerationUserFeedbackKindIR {
    fn quality_concept_id(self) -> &'static str {
        match self {
            Self::Unhelpful => "C_FEEDBACK_UNHELPFUL",
            Self::Misunderstood => "C_FEEDBACK_MISUNDERSTOOD",
            Self::MissedPoint => "C_FEEDBACK_MISSED_POINT",
            Self::TooVerbose => "C_FEEDBACK_TOO_VERBOSE",
            Self::TooBrief => "C_FEEDBACK_TOO_BRIEF",
            Self::Incorrect => "C_FEEDBACK_INCORRECT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationLifecycleClaimIR {
    ActivePlan,
    SupersededPlan,
    WithdrawnPlan,
    ReportedAttempt,
    ReportedInProgress,
    ReportedSuccess,
    ReportedFailure,
    NoUserReport,
    NoVerifiedExecutionOrResult,
    ExecutionInProgress,
    FinalResultUnavailable,
    VerifiedSuccess,
    VerifiedFailure,
    ResultUnavailable,
    UntrustedEvidenceMention,
    ExecutionStateUnchanged,
    ConflictingReports,
    ReportsNotVerified,
}

impl GenerationLifecycleClaimIR {
    fn concept_id(self) -> &'static str {
        match self {
            Self::ActivePlan => "C_LIFECYCLE_ACTIVE_PLAN",
            Self::SupersededPlan => "C_LIFECYCLE_SUPERSEDED_PLAN",
            Self::WithdrawnPlan => "C_LIFECYCLE_WITHDRAWN_PLAN",
            Self::ReportedAttempt => "C_LIFECYCLE_REPORTED_ATTEMPT",
            Self::ReportedInProgress => "C_LIFECYCLE_REPORTED_IN_PROGRESS",
            Self::ReportedSuccess => "C_LIFECYCLE_REPORTED_SUCCESS",
            Self::ReportedFailure => "C_LIFECYCLE_REPORTED_FAILURE",
            Self::NoUserReport => "C_LIFECYCLE_NO_USER_REPORT",
            Self::NoVerifiedExecutionOrResult => "C_LIFECYCLE_NO_EXECUTION_OR_RESULT",
            Self::ExecutionInProgress => "C_LIFECYCLE_EXECUTION_IN_PROGRESS",
            Self::FinalResultUnavailable => "C_LIFECYCLE_FINAL_RESULT_UNAVAILABLE",
            Self::VerifiedSuccess => "C_LIFECYCLE_VERIFIED_SUCCESS",
            Self::VerifiedFailure => "C_LIFECYCLE_VERIFIED_FAILURE",
            Self::ResultUnavailable => "C_LIFECYCLE_RESULT_UNAVAILABLE",
            Self::UntrustedEvidenceMention => "C_LIFECYCLE_UNTRUSTED_EVIDENCE_MENTION",
            Self::ExecutionStateUnchanged => "C_LIFECYCLE_EXECUTION_STATE_UNCHANGED",
            Self::ConflictingReports => "C_LIFECYCLE_CONFLICTING_REPORTS",
            Self::ReportsNotVerified => "C_LIFECYCLE_REPORTS_NOT_VERIFIED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationActionSetQuantifierIR {
    All,
    Any,
    None,
}

impl GenerationActionSetQuantifierIR {
    fn concept_id(self) -> &'static str {
        match self {
            Self::All => "C_ACTION_SET_ALL",
            Self::Any => "C_ACTION_SET_ANY",
            Self::None => "C_ACTION_SET_NONE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationActionSetPredicateIR {
    ActivePlan,
    ReportedCompletion,
    ReportedFailure,
    UnverifiedExecution,
    VerifiedExecution,
    VerifiedSuccess,
    VerifiedFailure,
    VerifiedInProgress,
}

impl GenerationActionSetPredicateIR {
    fn concept_id(self) -> &'static str {
        match self {
            Self::ActivePlan => "C_ACTION_SET_ACTIVE_PLAN",
            Self::ReportedCompletion => "C_ACTION_SET_REPORTED_COMPLETION",
            Self::ReportedFailure => "C_ACTION_SET_REPORTED_FAILURE",
            Self::UnverifiedExecution => "C_ACTION_SET_UNVERIFIED_EXECUTION",
            Self::VerifiedExecution => "C_ACTION_SET_VERIFIED_EXECUTION",
            Self::VerifiedSuccess => "C_ACTION_SET_VERIFIED_SUCCESS",
            Self::VerifiedFailure => "C_ACTION_SET_VERIFIED_FAILURE",
            Self::VerifiedInProgress => "C_ACTION_SET_VERIFIED_IN_PROGRESS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum GenerationActionSetTruthIR {
    True,
    False,
    Unknown,
}

impl GenerationActionSetTruthIR {
    fn concept_id(self) -> &'static str {
        match self {
            Self::True => "C_ACTION_SET_TRUE",
            Self::False => "C_ACTION_SET_FALSE",
            Self::Unknown => "C_ACTION_SET_UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationContextIR {
    pub language: LanguageCodeIR,
    pub register: LanguageRegisterIR,
    pub tense: GenerationTenseIR,
    pub emotion: GenerationEmotionIR,
    pub urgency_millis: u16,
    pub default_speech_intent: GenerationSpeechIntentIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechIntentNodeIR {
    pub event_node_id: String,
    pub intent: GenerationSpeechIntentIR,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechIntentGraphIR {
    pub intents: Vec<SpeechIntentNodeIR>,
    pub source_semantic_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseMoveKindIR {
    Acknowledgement,
    Action,
    EvidenceBoundary,
    Question,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseMoveIR {
    pub move_id: String,
    pub event_node_id: String,
    pub kind: DiscourseMoveKindIR,
    pub predecessor_move_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationDiscoursePlanIR {
    pub moves: Vec<DiscourseMoveIR>,
    pub source_semantic_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpressionPartOfSpeechIR {
    Verb,
    Noun,
    Adjective,
    Interjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpressionMorphologyClassIR {
    KoreanHadaLocative,
    KoreanHadaAccusative,
    EnglishRegularRelation,
    KoreanHada,
    KoreanCopula,
    KoreanInvariable,
    EnglishRegular,
    EnglishCopula,
    EnglishInvariable,
}

/// A language phenotype attached to a semantic concept.  This is lexical and
/// grammatical knowledge, never a semantic concept payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionNodeIR {
    pub expression_id: String,
    pub language: LanguageCodeIR,
    pub concept_id: String,
    pub lexical_root: String,
    pub part_of_speech: ExpressionPartOfSpeechIR,
    pub morphology: ExpressionMorphologyClassIR,
    pub register: LanguageRegisterIR,
    pub confidence_millis: u16,
    pub provenance: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExpressionNodeStore {
    entries: BTreeMap<String, ExpressionNodeIR>,
}

impl ExpressionNodeStore {
    pub fn bilingual_builtin() -> Self {
        let mut store = Self::default();
        for entry in builtin_expression_nodes() {
            store
                .inject(entry)
                .expect("built-in expression node must be valid");
        }
        store
    }

    pub fn inject(&mut self, entry: ExpressionNodeIR) -> Result<bool, String> {
        if entry.expression_id.trim().is_empty()
            || entry.concept_id.trim().is_empty()
            || entry.lexical_root.trim().is_empty()
            || entry.confidence_millis > 1_000
            || entry.provenance.trim().is_empty()
            || (entry
                .lexical_root
                .contains(['.', '!', '?', '。', '！', '？'])
                && !(entry.part_of_speech == ExpressionPartOfSpeechIR::Noun
                    && entry.provenance.starts_with("RUNTIME_REFERENT_SURFACE:")))
            || matches!(
                entry.language,
                LanguageCodeIR::Mixed | LanguageCodeIR::Unknown
            )
        {
            return Err("INVALID_EXPRESSION_NODE".to_string());
        }
        if let Some(existing) = self.entries.get(&entry.expression_id) {
            return if existing == &entry {
                Ok(false)
            } else {
                Err("EXPRESSION_IDENTITY_CONFLICT".to_string())
            };
        }
        self.entries.insert(entry.expression_id.clone(), entry);
        Ok(true)
    }

    pub fn attach_alias(
        &mut self,
        expression_id: &str,
        language: LanguageCodeIR,
        concept_id: &str,
        surface: &str,
        part_of_speech: ExpressionPartOfSpeechIR,
        provenance: &str,
    ) -> Result<bool, String> {
        self.inject(ExpressionNodeIR {
            expression_id: expression_id.to_string(),
            language,
            concept_id: concept_id.to_string(),
            lexical_root: surface.to_string(),
            part_of_speech,
            morphology: match language {
                LanguageCodeIR::Korean => ExpressionMorphologyClassIR::KoreanInvariable,
                _ => ExpressionMorphologyClassIR::EnglishInvariable,
            },
            register: LanguageRegisterIR::Neutral,
            confidence_millis: 1_000,
            provenance: provenance.to_string(),
        })
    }

    fn candidates(&self, concept_id: &str, language: LanguageCodeIR) -> Vec<&ExpressionNodeIR> {
        self.entries
            .values()
            .filter(|entry| entry.concept_id == concept_id && entry.language == language)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainableActivationIR {
    pub activation_millis: u16,
    pub confidence_millis: u16,
    pub context_fit_millis: u16,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionSelectionIR {
    pub meaning_node_id: String,
    pub expression: ExpressionNodeIR,
    pub score: ExplainableActivationIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionSelectionGraphIR {
    pub selections: Vec<ExpressionSelectionIR>,
    pub unresolved_meaning_node_ids: Vec<String>,
    pub source_semantic_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyntaxConstituentRoleIR {
    Agent,
    Theme,
    Goal,
    Property,
    Possessor,
    Negation,
    Predicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxConstituentIR {
    pub meaning_node_id: String,
    pub expression_id: String,
    pub role: SyntaxConstituentRoleIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxClauseIR {
    pub clause_id: String,
    pub move_id: String,
    pub event_node_id: String,
    pub speech_intent: GenerationSpeechIntentIR,
    pub constituents: Vec<SyntaxConstituentIR>,
    pub source_edge_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxPlanIR {
    pub language: LanguageCodeIR,
    pub clauses: Vec<SyntaxClauseIR>,
    pub source_semantic_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologicalTokenIR {
    pub token_index: usize,
    pub surface: String,
    pub attach_left: bool,
    pub expression_id: Option<String>,
    pub grammar_rule_id: Option<String>,
    pub source_meaning_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologicalRealizationIR {
    pub language: LanguageCodeIR,
    pub tokens: Vec<MorphologicalTokenIR>,
    pub realized_text: String,
    pub source_semantic_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationVerificationIR {
    pub covered_meaning_node_ids: Vec<String>,
    pub covered_meaning_edge_ids: Vec<String>,
    pub unresolved_meaning_node_ids: Vec<String>,
    pub unsupported_surface_tokens: usize,
    pub unsupported_claims: usize,
    pub semantic_roundtrip_sha256: String,
    pub faithful: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerativeLanguageIR {
    pub schema: String,
    pub context: GenerationContextIR,
    pub meaning: GenerationMeaningGraphIR,
    pub speech_intent: SpeechIntentGraphIR,
    pub discourse_plan: GenerationDiscoursePlanIR,
    pub expression_selection: ExpressionSelectionGraphIR,
    pub syntax_plan: SyntaxPlanIR,
    pub morphology: MorphologicalRealizationIR,
    pub verification: GenerationVerificationIR,
    pub semantic_authority: bool,
    pub language_can_execute: bool,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub generation_sha256: String,
}

impl GenerativeLanguageIR {
    /// Re-realize the same semantic/syntactic graph under an affective policy.
    /// No fact, speech intent, polarity, scope or source reference is writable
    /// through this interface. Called before the response is committed.
    pub(crate) fn condition_realization(
        &mut self,
        policy: &crate::affective_field::AffectiveRealizationPolicyIR,
    ) {
        if policy.formal {
            self.context.register = LanguageRegisterIR::Formal;
        }
        self.context.urgency_millis = policy.urgency_millis;
        if policy.warmth_millis > 150 {
            self.context.emotion = GenerationEmotionIR::Warm;
        }
        // A light social marker is a grammar choice, not a claim about the
        // user's feelings. Never attach it to answers, refusals or task plans.
        if policy.playfulness_millis > 150
            && policy.urgency_millis <= 150
            && policy.brevity_millis <= 150
            && self.context.register != LanguageRegisterIR::Formal
            && playful_social_anchor(&self.meaning).is_some()
        {
            self.context.emotion = GenerationEmotionIR::Playful;
        } else if self.context.emotion == GenerationEmotionIR::Playful {
            self.context.emotion = GenerationEmotionIR::Neutral;
        }
        self.morphology = realize_morphology(
            &self.meaning,
            &self.context,
            &self.syntax_plan,
            &self.expression_selection,
        );
        self.verification = verify_generation(
            &self.meaning,
            &self.expression_selection,
            &self.syntax_plan,
            &self.morphology,
        );
        self.generation_sha256 = generative_language_sha256(self);
    }

    pub fn validate(&self) -> bool {
        self.schema == GENERATIVE_LANGUAGE_SCHEMA
            && self.meaning.validate()
            && self.speech_intent.source_semantic_sha256 == self.meaning.semantic_sha256
            && self.discourse_plan.source_semantic_sha256 == self.meaning.semantic_sha256
            && self.expression_selection.source_semantic_sha256 == self.meaning.semantic_sha256
            && self.syntax_plan.source_semantic_sha256 == self.meaning.semantic_sha256
            && self.morphology.source_semantic_sha256 == self.meaning.semantic_sha256
            && self.morphology
                == realize_morphology(
                    &self.meaning,
                    &self.context,
                    &self.syntax_plan,
                    &self.expression_selection,
                )
            && self.morphology.realized_text.chars().count() <= MAX_REALIZED_CHARS
            && self.verification.faithful
            && self.verification.unsupported_surface_tokens == 0
            && self.verification.unsupported_claims == 0
            && self.verification.semantic_roundtrip_sha256 == self.meaning.semantic_sha256
            && !self.semantic_authority
            && !self.language_can_execute
            && self.external_llm_calls == 0
            && self.local_teacher_calls == 0
            && self.generation_sha256 == generative_language_sha256(self)
    }
}

pub struct GenerativeLanguageRequestIR<'a> {
    pub meaning: GenerationMeaningGraphIR,
    pub context: GenerationContextIR,
    pub expressions: &'a ExpressionNodeStore,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GenerativeLanguageCortex;

impl GenerativeLanguageCortex {
    pub fn generate(
        &self,
        request: GenerativeLanguageRequestIR<'_>,
    ) -> Result<GenerativeLanguageIR, String> {
        if !request.meaning.validate()
            || matches!(
                request.context.language,
                LanguageCodeIR::Mixed | LanguageCodeIR::Unknown
            )
            || request.context.urgency_millis > 1_000
        {
            return Err("INVALID_GENERATION_REQUEST".to_string());
        }
        let speech_intent = derive_speech_intent(&request.meaning, &request.context);
        let discourse_plan = build_discourse_plan(&request.meaning, &speech_intent);
        let expression_selection =
            select_expressions(&request.meaning, &request.context, request.expressions);
        if !expression_selection.unresolved_meaning_node_ids.is_empty() {
            return Err(format!(
                "UNRESOLVED_EXPRESSION_NODES:{}",
                expression_selection.unresolved_meaning_node_ids.join(",")
            ));
        }
        let syntax_plan = assemble_syntax(
            &request.meaning,
            &speech_intent,
            &discourse_plan,
            &expression_selection,
            request.context.language,
        );
        let morphology = realize_morphology(
            &request.meaning,
            &request.context,
            &syntax_plan,
            &expression_selection,
        );
        let verification = verify_generation(
            &request.meaning,
            &expression_selection,
            &syntax_plan,
            &morphology,
        );
        let mut generated = GenerativeLanguageIR {
            schema: GENERATIVE_LANGUAGE_SCHEMA.to_string(),
            context: request.context,
            meaning: request.meaning,
            speech_intent,
            discourse_plan,
            expression_selection,
            syntax_plan,
            morphology,
            verification,
            semantic_authority: false,
            language_can_execute: false,
            external_llm_calls: 0,
            local_teacher_calls: 0,
            generation_sha256: String::new(),
        };
        generated.generation_sha256 = generative_language_sha256(&generated);
        if !generated.validate() {
            return Err(format!(
                "GENERATION_VALIDATION_FAILED:{}",
                serde_json::to_string(&generated.verification).unwrap_or_default()
            ));
        }
        Ok(generated)
    }
}

pub fn generation_meaning_sha256(graph: &GenerationMeaningGraphIR) -> String {
    let mut canonical = graph.clone();
    canonical.semantic_sha256.clear();
    content_sha256(&canonical)
}

pub fn generative_language_sha256(generated: &GenerativeLanguageIR) -> String {
    let mut canonical = generated.clone();
    canonical.generation_sha256.clear();
    content_sha256(&canonical)
}

fn derive_speech_intent(
    meaning: &GenerationMeaningGraphIR,
    context: &GenerationContextIR,
) -> SpeechIntentGraphIR {
    let intents = meaning
        .nodes
        .iter()
        .filter(|node| node.kind == GenerationMeaningNodeKindIR::Event)
        .map(|node| {
            let intent = match node.concept_id.as_str() {
                "C_ACKNOWLEDGE" => GenerationSpeechIntentIR::Acknowledge,
                "C_REMEMBER" => GenerationSpeechIntentIR::CommitFutureAction,
                "C_INVITE_CHECK" => GenerationSpeechIntentIR::Invite,
                "C_RETURN_TOPIC" => GenerationSpeechIntentIR::Invite,
                "C_ACTIVATE_TOPIC" | "C_ACTIVATE_TOPIC_GROUP" => GenerationSpeechIntentIR::Inform,
                "C_DIALOGUE_OFFER_HELP" => GenerationSpeechIntentIR::Ask,
                "C_DIALOGUE_INVITE_NEED" | "C_DIALOGUE_CONTINUE" => {
                    GenerationSpeechIntentIR::Invite
                }
                "C_DIALOGUE_LISTEN" => GenerationSpeechIntentIR::Inform,
                "C_GATE_VERIFY" => GenerationSpeechIntentIR::Advise,
                "C_GATE_CONTINUE" => GenerationSpeechIntentIR::CommitFutureAction,
                "C_GATE_REPORT_ASK_STOP"
                | "C_GATE_ASK_UNRESOLVED"
                | "C_GATE_VERIFY_OR_ASK_STOP" => GenerationSpeechIntentIR::Ask,
                "C_FEEDBACK_REQUEST_DETAIL" => GenerationSpeechIntentIR::Ask,
                "C_FEEDBACK_CORRECT" | "C_FEEDBACK_ADJUST" => {
                    GenerationSpeechIntentIR::CommitFutureAction
                }
                concept if concept.starts_with("C_CLARIFY_") => GenerationSpeechIntentIR::Ask,
                "C_RESOLVE_REFERENCE" | "C_NAME_TARGET" => GenerationSpeechIntentIR::Ask,
                "C_DIALOGUE_ANSWER_AMBIGUOUS" => GenerationSpeechIntentIR::Ask,
                "C_TEMPORAL_ANSWER_AMBIGUOUS" => GenerationSpeechIntentIR::Ask,
                "C_COPULA" => GenerationSpeechIntentIR::Inform,
                "C_WORLD_CLAUSE_ASK" | "C_WORLD_CLAUSE_REFERENCE" => GenerationSpeechIntentIR::Ask,
                "C_WORLD_CLAUSE_REMEMBER" => GenerationSpeechIntentIR::Acknowledge,
                _ => context.default_speech_intent,
            };
            SpeechIntentNodeIR {
                event_node_id: node.node_id.clone(),
                intent,
                evidence_refs: vec![
                    format!("MEANING_NODE:{}", node.node_id),
                    format!("CONTEXT_INTENT:{intent:?}"),
                ],
            }
        })
        .collect();
    SpeechIntentGraphIR {
        intents,
        source_semantic_sha256: meaning.semantic_sha256.clone(),
    }
}

fn build_discourse_plan(
    meaning: &GenerationMeaningGraphIR,
    speech: &SpeechIntentGraphIR,
) -> GenerationDiscoursePlanIR {
    let sequence_predecessors = meaning
        .edges
        .iter()
        .filter(|edge| edge.relation == GenerationMeaningRelationIR::Sequence)
        .map(|edge| (edge.target_node_id.clone(), edge.source_node_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let event_order = topological_event_order(meaning, &sequence_predecessors);
    let move_by_event = event_order
        .iter()
        .enumerate()
        .map(|(index, event)| (event.clone(), format!("DISCOURSE-MOVE-{:03}", index + 1)))
        .collect::<BTreeMap<_, _>>();
    let moves = event_order
        .iter()
        .enumerate()
        .map(|(index, event_node_id)| {
            let intent = speech
                .intents
                .iter()
                .find(|item| item.event_node_id == *event_node_id)
                .map(|item| item.intent)
                .unwrap_or(GenerationSpeechIntentIR::Inform);
            let kind = match intent {
                GenerationSpeechIntentIR::Acknowledge => DiscourseMoveKindIR::Acknowledgement,
                GenerationSpeechIntentIR::Ask => DiscourseMoveKindIR::Question,
                GenerationSpeechIntentIR::Inform
                    if meaning.nodes.iter().any(|node| {
                        node.node_id == *event_node_id
                            && matches!(
                                node.concept_id.as_str(),
                                "C_COPULA" | "C_TOPIC_CHANGE_BOUNDARY"
                            )
                    }) =>
                {
                    DiscourseMoveKindIR::EvidenceBoundary
                }
                _ => DiscourseMoveKindIR::Action,
            };
            let predecessor_move_ids = sequence_predecessors
                .get(event_node_id)
                .and_then(|event| move_by_event.get(event))
                .cloned()
                .into_iter()
                .collect();
            DiscourseMoveIR {
                move_id: format!("DISCOURSE-MOVE-{:03}", index + 1),
                event_node_id: event_node_id.clone(),
                kind,
                predecessor_move_ids,
                evidence_refs: vec![format!("MEANING_NODE:{event_node_id}")],
            }
        })
        .collect();
    GenerationDiscoursePlanIR {
        moves,
        source_semantic_sha256: meaning.semantic_sha256.clone(),
    }
}

fn topological_event_order(
    meaning: &GenerationMeaningGraphIR,
    predecessors: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut remaining = meaning
        .nodes
        .iter()
        .filter(|node| node.kind == GenerationMeaningNodeKindIR::Event)
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let position = remaining
            .iter()
            .position(|node| {
                predecessors
                    .get(node)
                    .is_none_or(|predecessor| ordered.contains(predecessor))
            })
            .unwrap_or(0);
        ordered.push(remaining.remove(position));
    }
    ordered
}

fn select_expressions(
    meaning: &GenerationMeaningGraphIR,
    context: &GenerationContextIR,
    store: &ExpressionNodeStore,
) -> ExpressionSelectionGraphIR {
    let mut selections = Vec::new();
    let mut unresolved = Vec::new();
    for node in &meaning.nodes {
        let mut candidates = store.candidates(&node.concept_id, context.language);
        candidates.sort_by(|left, right| {
            expression_score(right, context)
                .cmp(&expression_score(left, context))
                .then_with(|| left.expression_id.cmp(&right.expression_id))
        });
        let Some(selected) = candidates.first() else {
            unresolved.push(node.node_id.clone());
            continue;
        };
        let context_fit = context_fit(selected, context);
        selections.push(ExpressionSelectionIR {
            meaning_node_id: node.node_id.clone(),
            expression: (*selected).clone(),
            score: ExplainableActivationIR {
                activation_millis: 1_000,
                confidence_millis: selected.confidence_millis,
                context_fit_millis: context_fit,
                reasons: vec![
                    format!("EXACT_CONCEPT_MATCH:{}", node.concept_id),
                    format!("LANGUAGE_MATCH:{:?}", context.language),
                    format!("REGISTER_MATCH:{:?}", selected.register),
                    format!("PROVENANCE:{}", selected.provenance),
                ],
            },
        });
    }
    ExpressionSelectionGraphIR {
        selections,
        unresolved_meaning_node_ids: unresolved,
        source_semantic_sha256: meaning.semantic_sha256.clone(),
    }
}

fn expression_score(entry: &ExpressionNodeIR, context: &GenerationContextIR) -> u32 {
    u32::from(entry.confidence_millis) + u32::from(context_fit(entry, context))
}

fn context_fit(entry: &ExpressionNodeIR, context: &GenerationContextIR) -> u16 {
    if entry.register == context.register || entry.register == LanguageRegisterIR::Neutral {
        1_000
    } else {
        700
    }
}

fn assemble_syntax(
    meaning: &GenerationMeaningGraphIR,
    speech: &SpeechIntentGraphIR,
    discourse: &GenerationDiscoursePlanIR,
    expressions: &ExpressionSelectionGraphIR,
    language: LanguageCodeIR,
) -> SyntaxPlanIR {
    let clauses = discourse
        .moves
        .iter()
        .enumerate()
        .map(|(index, discourse_move)| {
            let mut constituents = Vec::new();
            if let Some(selection) = expressions
                .selections
                .iter()
                .find(|item| item.meaning_node_id == discourse_move.event_node_id)
            {
                constituents.push(SyntaxConstituentIR {
                    meaning_node_id: discourse_move.event_node_id.clone(),
                    expression_id: selection.expression.expression_id.clone(),
                    role: SyntaxConstituentRoleIR::Predicate,
                });
            }
            let mut source_edge_ids = Vec::new();
            for edge in meaning
                .edges
                .iter()
                .filter(|edge| edge.source_node_id == discourse_move.event_node_id)
            {
                let role = match edge.relation {
                    GenerationMeaningRelationIR::Agent => Some(SyntaxConstituentRoleIR::Agent),
                    GenerationMeaningRelationIR::Theme => Some(SyntaxConstituentRoleIR::Theme),
                    GenerationMeaningRelationIR::Goal => Some(SyntaxConstituentRoleIR::Goal),
                    GenerationMeaningRelationIR::Property => {
                        Some(SyntaxConstituentRoleIR::Property)
                    }
                    GenerationMeaningRelationIR::Negates => Some(SyntaxConstituentRoleIR::Negation),
                    _ => None,
                };
                if let Some(role) = role {
                    if let Some(selection) = expressions
                        .selections
                        .iter()
                        .find(|item| item.meaning_node_id == edge.target_node_id)
                    {
                        constituents.push(SyntaxConstituentIR {
                            meaning_node_id: edge.target_node_id.clone(),
                            expression_id: selection.expression.expression_id.clone(),
                            role,
                        });
                        source_edge_ids.push(edge.edge_id.clone());
                        for modifier in meaning.edges.iter().filter(|modifier| {
                            modifier.source_node_id == edge.target_node_id
                                && modifier.relation == GenerationMeaningRelationIR::Possessor
                        }) {
                            if let Some(modifier_selection) = expressions
                                .selections
                                .iter()
                                .find(|item| item.meaning_node_id == modifier.target_node_id)
                            {
                                constituents.push(SyntaxConstituentIR {
                                    meaning_node_id: modifier.target_node_id.clone(),
                                    expression_id: modifier_selection
                                        .expression
                                        .expression_id
                                        .clone(),
                                    role: SyntaxConstituentRoleIR::Possessor,
                                });
                                source_edge_ids.push(modifier.edge_id.clone());
                            }
                        }
                    }
                }
            }
            constituents.sort_by_key(|item| syntax_order(language, item.role));
            SyntaxClauseIR {
                clause_id: format!("SYNTAX-CLAUSE-{:03}", index + 1),
                move_id: discourse_move.move_id.clone(),
                event_node_id: discourse_move.event_node_id.clone(),
                speech_intent: speech
                    .intents
                    .iter()
                    .find(|item| item.event_node_id == discourse_move.event_node_id)
                    .map(|item| item.intent)
                    .unwrap_or(GenerationSpeechIntentIR::Inform),
                constituents,
                source_edge_ids,
            }
        })
        .collect();
    SyntaxPlanIR {
        language,
        clauses,
        source_semantic_sha256: meaning.semantic_sha256.clone(),
    }
}

fn syntax_order(language: LanguageCodeIR, role: SyntaxConstituentRoleIR) -> usize {
    match (language, role) {
        (_, SyntaxConstituentRoleIR::Agent) => 0,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Possessor) => 1,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Theme) => 2,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Goal) => 3,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Property) => 4,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Negation) => 5,
        (LanguageCodeIR::Korean, SyntaxConstituentRoleIR::Predicate) => 6,
        (_, SyntaxConstituentRoleIR::Predicate) => 1,
        (_, SyntaxConstituentRoleIR::Negation) => 2,
        (_, SyntaxConstituentRoleIR::Theme) => 2,
        (_, SyntaxConstituentRoleIR::Possessor) => 3,
        (_, SyntaxConstituentRoleIR::Goal) => 4,
        (_, SyntaxConstituentRoleIR::Property) => 5,
    }
}

fn playful_social_anchor(meaning: &GenerationMeaningGraphIR) -> Option<&GenerationMeaningNodeIR> {
    // A social clause inside an otherwise serious message is not sufficient.
    if meaning.nodes.iter().any(|node| {
        node.kind == GenerationMeaningNodeKindIR::Event
            && !matches!(
                node.concept_id.as_str(),
                "C_DIALOGUE_GREETING_REPLY"
                    | "C_DIALOGUE_GRATITUDE_REPLY"
                    | "C_DIALOGUE_OFFER_HELP"
                    | "C_DIALOGUE_INVITE_NEED"
            )
    }) {
        return None;
    }
    meaning.nodes.iter().find(|node| {
        matches!(
            node.concept_id.as_str(),
            "C_DIALOGUE_GREETING_REPLY" | "C_DIALOGUE_GRATITUDE_REPLY"
        )
    })
}

fn realize_morphology(
    meaning: &GenerationMeaningGraphIR,
    context: &GenerationContextIR,
    syntax: &SyntaxPlanIR,
    expressions: &ExpressionSelectionGraphIR,
) -> MorphologicalRealizationIR {
    let selected = expressions
        .selections
        .iter()
        .map(|item| {
            (
                (
                    item.expression.expression_id.as_str(),
                    item.meaning_node_id.as_str(),
                ),
                item,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut tokens = Vec::new();
    if context.emotion == GenerationEmotionIR::Playful
        && context.register != LanguageRegisterIR::Formal
        && context.urgency_millis <= 150
    {
        if let Some(node) = playful_social_anchor(meaning) {
            push_grammar_token(
                &mut tokens,
                if context.language == LanguageCodeIR::Korean {
                    "ㅎㅎ"
                } else {
                    "Heh,"
                },
                "GRAMMAR_SOCIAL_PLAYFUL_MARKER",
                &node.node_id,
            );
        }
    }
    for (clause_index, clause) in syntax.clauses.iter().enumerate() {
        let clause_tokens = match context.language {
            LanguageCodeIR::Korean => realize_korean_clause(clause, context, &selected),
            _ => realize_english_clause(clause, context, &selected),
        };
        let mut clause_tokens = clause_tokens;
        // Korean zero subjects are licensed by a unique discourse referent,
        // not by deleting answer text. Keep an auditable zero-width grammar
        // token so the semantic subject remains in the generation trace.
        if context.language == LanguageCodeIR::Korean {
            if let (Some(predicate), Some(subject)) = (
                constituent_selection(clause, SyntaxConstituentRoleIR::Predicate, &selected),
                constituent_selection(clause, SyntaxConstituentRoleIR::Theme, &selected),
            ) {
                let mode = &predicate.expression.concept_id;
                let self_report = mode == "C_WORLD_CLAUSE_REMEMBER"
                    && subject.expression.concept_id == "C_ENTITY___user__";
                let shared = matches!(
                    mode.as_str(),
                    "C_WORLD_CLAUSE_DERIVED" | "C_WORLD_CLAUSE_CONCLUSION"
                ) && clause_index > 0
                    && syntax.clauses.get(clause_index - 1).is_some_and(|prior| {
                        constituent_selection(prior, SyntaxConstituentRoleIR::Goal, &selected)
                            .is_none()
                            && constituent_selection(
                                prior,
                                SyntaxConstituentRoleIR::Predicate,
                                &selected,
                            )
                            .is_some_and(|p| p.expression.concept_id.starts_with("C_WORLD_CLAUSE_"))
                            && constituent_selection(
                                prior,
                                SyntaxConstituentRoleIR::Theme,
                                &selected,
                            )
                            .is_some_and(|p| {
                                p.expression.concept_id == subject.expression.concept_id
                            })
                    });
                if self_report || shared {
                    for token in &mut clause_tokens {
                        if token.source_meaning_node_ids == [subject.meaning_node_id.clone()]
                            && token.expression_id.as_deref()
                                == Some(subject.expression.expression_id.as_str())
                        {
                            token.surface.clear();
                            token.expression_id = None;
                            token.grammar_rule_id = Some(
                                if self_report {
                                    "KO.ZERO_SUBJECT.SPEAKER_REPORT"
                                } else {
                                    "KO.ZERO_SUBJECT.SHARED_REFERENT"
                                }
                                .into(),
                            );
                        }
                    }
                }
            }
        }
        if context.language == LanguageCodeIR::English
            && !tokens
                .last()
                .is_some_and(|token| token.surface.ends_with(','))
        {
            if let Some(first) = clause_tokens.first_mut() {
                first.surface = uppercase_first(&first.surface);
            }
        }
        for mut token in clause_tokens {
            token.token_index = tokens.len();
            tokens.push(token);
        }
    }
    let realized_text = join_morphological_tokens(&tokens, context.language);
    MorphologicalRealizationIR {
        language: context.language,
        tokens,
        realized_text,
        source_semantic_sha256: meaning.semantic_sha256.clone(),
    }
}

fn realize_korean_clause(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let predicate = clause
        .constituents
        .iter()
        .find(|item| item.role == SyntaxConstituentRoleIR::Predicate)
        .and_then(|item| {
            selected
                .get(&(item.expression_id.as_str(), item.meaning_node_id.as_str()))
                .copied()
        });
    let Some(predicate) = predicate else {
        return output;
    };
    if predicate.expression.concept_id == "C_CONTENT_PROJECTION" {
        return realize_content_projection(clause, context, selected, predicate);
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_WORLD_CLAUSE_")
    {
        return world_realization::realize_world_clause(clause, context, selected, predicate);
    }
    if predicate.expression.part_of_speech == ExpressionPartOfSpeechIR::Interjection {
        let punctuation = if predicate.expression.concept_id == "C_DIALOGUE_GREETING_REPLY" {
            "!"
        } else {
            "."
        };
        push_expression_token(
            &mut output,
            predicate,
            format!("{}{punctuation}", predicate.expression.lexical_root),
        );
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_PLAN_SUGGESTION_BOUNDARY"
            | "C_PLAN_IMPLICIT_INVESTIGATION"
            | "C_PLAN_IMPLICIT_REPAIR"
            | "C_PLAN_IMPLICIT_EXPLANATION"
            | "C_PLAN_IMPLICIT_PLANNING"
            | "C_SARCASM_INTERPRETATION_BOUNDARY"
            | "C_FIGURATIVE_INTERPRETATION_BOUNDARY"
    ) {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_PLAN_SUGGESTION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’라는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "개선 제안으로 이해했어. 구현 명령으로 단정하지 않고 기대 효과와 요구사항부터 확인할게."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_INVESTIGATION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’의", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "원인이나 이유를 알고 싶다는 뜻으로 이해했어. 관찰 가능한 증거부터 확인할게."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_REPAIR" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’ 상태는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "그대로 둘 수 없어 수리가 필요하다는 뜻으로 이해했어. 원인과 수정 범위를 먼저 확인하되, 이 암묵적 표현만으로 외부 변경 권한을 넓히지는 않을게."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_EXPLANATION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’에 대해", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "근거가 있는 설명이나 요약을 원한다는 뜻으로 이해했어. 확인된 내용과 아직 모르는 부분을 나눠서 답할게."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_PLANNING" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’에 맞는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "선택지를 비교해 추천해달라는 뜻으로 이해했어. 제약과 근거를 먼저 확인하고 실행 권한은 별도로 둘게."
                        .to_string(),
                );
            }
            "C_SARCASM_INTERPRETATION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("{}가", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "충돌하므로 긍정 승인이 아니라 부정적 평가나 불만으로 이해했어. 이 표현만으로 새 작업 권한을 만들지는 않을게."
                        .to_string(),
                );
            }
            "C_FIGURATIVE_INTERPRETATION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "‘{}’를 문자 그대로의 행동이 아니라",
                            theme.expression.lexical_root
                        ),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "‘{}’에 해당하는 비유적 상태로",
                            goal.expression.lexical_root
                        ),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "이해했어. 물리적 의미로 실행하지 않고 실제 막힘이나 문제를 확인할게."
                        .to_string(),
                );
            }
            _ => unreachable!(),
        }
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_OFFER_HELP" {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                theme,
                format!("{}{particle}", theme.expression.lexical_root),
            );
        }
        let future_stem = korean_future_commitment(&predicate.expression.lexical_root)
            .trim_end_matches('게')
            .to_string();
        push_expression_token(&mut output, predicate, format!("{future_stem}까?"));
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_INVITE_NEED" {
        if let Some(goal) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected) {
            match goal.expression.concept_id.as_str() {
                "C_ADDITIONAL_NEED" => {
                    push_expression_token(&mut output, goal, "더 필요한 게".to_string());
                    push_grammar_token(
                        &mut output,
                        "있으면",
                        "KO.DIALOGUE.CONDITION.ADDITIONAL_NEED",
                        &clause.event_node_id,
                    );
                }
                "C_FUTURE_NEED" => {
                    push_expression_token(&mut output, goal, goal.expression.lexical_root.clone());
                    push_grammar_token(
                        &mut output,
                        "다시",
                        "KO.DIALOGUE.RETURN",
                        &clause.event_node_id,
                    );
                }
                _ => push_expression_token(&mut output, goal, goal.expression.lexical_root.clone()),
            }
        }
        push_expression_token(
            &mut output,
            predicate,
            format!(
                "{}해줘.",
                predicate.expression.lexical_root.trim_end_matches('하')
            ),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_CONTINUE" {
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                property,
                property.expression.lexical_root.clone(),
            );
        }
        if let Some(goal) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected) {
            push_expression_token(&mut output, goal, goal.expression.lexical_root.clone());
        }
        push_expression_token(
            &mut output,
            predicate,
            format!(
                "{}해.",
                predicate.expression.lexical_root.trim_end_matches('하')
            ),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_LISTEN" {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                theme,
                format!("{}{particle}", theme.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            format!("{}어.", predicate.expression.lexical_root),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_INTERPRET" {
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                task,
                format!("{} 작업의 계속 여부는", task.expression.lexical_root),
            );
        }
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}라는 실제 이득에", benefit.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "달린 조건으로 이해했어.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_VERIFY" {
        push_grammar_token(
            &mut output,
            "먼저",
            "KO.GATE.VERIFY.ORDER",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}을", benefit.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "검증해야 해.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_CONTINUE" {
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(&mut output, benefit, "그 이득이 확인되면".to_string());
        }
        push_expression_token(&mut output, predicate, "계속할게.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_REPORT_ASK_STOP" {
        push_grammar_token(
            &mut output,
            "아니면 그 결과를 보고한 뒤",
            "KO.GATE.NEGATIVE.REPORT",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                task,
                format!("{} 작업을", task.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "멈출지 물을게.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_ASK_UNRESOLVED" {
        push_grammar_token(
            &mut output,
            "증거가 부족하면",
            "KO.GATE.UNKNOWN.CONDITION",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{} 확인을", benefit.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "요청하고 추측하지 않을게.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_NOT_VERIFIED" {
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}의 달성 여부를", benefit.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "아직 직접 확인하지 못했어.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_PROXY_INSUFFICIENT" {
        push_grammar_token(
            &mut output,
            "점수나 대리 지표만으로",
            "KO.GATE.PENDING.PROXY_INSUFFICIENT",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                task,
                format!("{} 작업을", task.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "계속해도 된다고 판단하지 않을게.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_VERIFY_OR_ASK_STOP" {
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}을 확인하거나,", benefit.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "확인할 수 없다면",
            "KO.GATE.PENDING.UNRESOLVED",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected) {
            push_expression_token(
                &mut output,
                task,
                format!("{} 작업의", task.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "중단 여부를 다시 물어야 해.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_RECORD_PROXY" {
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                task,
                format!("{} 작업의", task.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "대리 지표 변화는 기록했지만,".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_PROXY_NOT_BENEFIT" {
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("필요한 실제 이득 {}의", benefit.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "확인으로 간주하지 않을게.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_ASSESS" {
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let surface = if property
                .is_some_and(|item| item.expression.concept_id == "C_FEEDBACK_MISUNDERSTOOD")
            {
                format!("{}에서", target.expression.lexical_root)
            } else {
                let particle = korean_particle(&target.expression.lexical_root, "이", "가");
                format!("{}{particle}", target.expression.lexical_root)
            };
            push_expression_token(&mut output, target, surface);
        }
        if let Some(property) = property {
            push_expression_token(
                &mut output,
                property,
                property.expression.lexical_root.clone(),
            );
        }
        push_grammar_token(
            &mut output,
            "네.",
            "KO.FEEDBACK.RETROSPECTIVE",
            &clause.event_node_id,
        );
        if let Some(token) = output.last_mut() {
            token.attach_left = true;
        }
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_REQUEST_DETAIL" {
        if let Some(detail) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&detail.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                detail,
                format!("{}{particle}", detail.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "짚어줘.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_CORRECT" {
        push_grammar_token(
            &mut output,
            "그 기준으로",
            "KO.FEEDBACK.CORRECTION.BASIS",
            &clause.event_node_id,
        );
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&target.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                target,
                format!("{}{particle}", target.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            format!(
                "{}.",
                korean_future_commitment(&predicate.expression.lexical_root)
            ),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_ADJUST" {
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&target.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                target,
                format!("{}{particle}", target.expression.lexical_root),
            );
        }
        if let Some(strategy) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                strategy,
                strategy.expression.lexical_root.clone(),
            );
        }
        push_expression_token(&mut output, predicate, "조정할게.".to_string());
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_GROUP_ADD_MEMBER" | "C_GROUP_REMOVE_MEMBER" | "C_GROUP_MERGE"
    ) {
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&target.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                target,
                format!("{}{particle}", target.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            match predicate.expression.concept_id.as_str() {
                "C_GROUP_ADD_MEMBER" => "추가했어.".to_string(),
                "C_GROUP_REMOVE_MEMBER" => "제외했어.".to_string(),
                _ => "합쳤어.".to_string(),
            },
        );
        if let Some(group) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(&mut output, group, group.expression.lexical_root.clone());
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GROUP_COUNT_STATE" {
        if let Some(group) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&group.expression.lexical_root, "은", "는");
            push_expression_token(
                &mut output,
                group,
                format!("{}{particle}", group.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "이제",
            "KO.DISCOURSE_GROUP.CURRENT_STATE",
            &clause.event_node_id,
        );
        if let Some(count) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                count,
                format!("{}개 대상을", count.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "가리켜.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_ASSESS_ACTION_SET" {
        let truth = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected);
        let set = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let quantifier = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let claim = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        let count = constituent_selection(clause, SyntaxConstituentRoleIR::Possessor, selected);
        if let Some(truth) = truth {
            push_expression_token(
                &mut output,
                truth,
                format!("{}.", truth.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "현재 행위 원장 기준으로",
            "KO.ACTION_SET.LEDGER_BASIS",
            &clause.event_node_id,
        );
        if let Some(set) = set {
            push_expression_token(&mut output, set, "선택된".to_string());
        }
        if let Some(count) = count {
            push_expression_token(&mut output, count, count.expression.lexical_root.clone());
        }
        if let Some(set) = set {
            push_expression_token(&mut output, set, set.expression.lexical_root.clone());
        }
        if let Some(quantifier) = quantifier {
            if matches!(
                quantifier.expression.concept_id.as_str(),
                "C_ACTION_SET_ANY" | "C_ACTION_SET_NONE"
            ) {
                push_grammar_token(
                    &mut output,
                    "중",
                    "KO.ACTION_SET.PARTITIVE",
                    &clause.event_node_id,
                );
            }
            push_expression_token(
                &mut output,
                quantifier,
                quantifier.expression.lexical_root.clone(),
            );
        }
        if let Some(claim) = claim {
            push_expression_token(
                &mut output,
                claim,
                format!("{}.", claim.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_CLARIFY_") {
        let detail = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        match predicate.expression.concept_id.as_str() {
            "C_CLARIFY_PENDING_CHOICE" => push_expression_token(
                &mut output,
                predicate,
                "앞서 물은 선택지 중 어느 쪽인지 직접 지정해줘.".to_string(),
            ),
            "C_CLARIFY_ORDERED_PAIR" => push_expression_token(
                &mut output,
                predicate,
                "전자와 후자의 기준이 되는 두 대상을 직접 지정해줘.".to_string(),
            ),
            "C_CLARIFY_LOCAL_ORDINAL" => push_expression_token(
                &mut output,
                predicate,
                "몇 번째 대상을 뜻하는지 다시 확인해줘.".to_string(),
            ),
            "C_CLARIFY_EVENT_ORDINAL" => push_expression_token(
                &mut output,
                predicate,
                "이전 계획의 몇 번째 작업인지 다시 확인해줘.".to_string(),
            ),
            "C_CLARIFY_PREVIOUS_TOPIC" => push_expression_token(
                &mut output,
                predicate,
                "돌아갈 주제의 이름을 말해줘.".to_string(),
            ),
            "C_CLARIFY_COMPETING_REQUEST" => {
                push_grammar_token(
                    &mut output,
                    "문장에서",
                    "KO.CLARIFY.COMPETITION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(detail) = detail {
                    push_expression_token(
                        &mut output,
                        detail,
                        detail.expression.lexical_root.clone(),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "중 어느 쪽이 실제 요청인지 지정해줘.".to_string(),
                );
            }
            "C_CLARIFY_NONLITERAL_READING" => {
                if let Some(detail) = detail {
                    let particle = korean_particle(&detail.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        detail,
                        format!("‘{}’{particle}", detail.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "문자 그대로의 상황인지 비유적인 문제 상황인지 알려줘.".to_string(),
                );
            }
            "C_CLARIFY_VOICE_ALTERNATIVE" => {
                push_grammar_token(
                    &mut output,
                    "음성 입력이",
                    "KO.CLARIFY.VOICE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(detail) = detail {
                    push_expression_token(
                        &mut output,
                        detail,
                        format!("{}로", detail.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "들릴 수 있어. 어느 쪽인지 한 번만 확인해줘.".to_string(),
                );
            }
            _ => push_expression_token(
                &mut output,
                predicate,
                "무엇을 원하는지 조금만 더 구체적으로 말해줘.".to_string(),
            ),
        }
        return output;
    }
    if predicate.expression.concept_id == "C_RESOLVE_REFERENCE" {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        if let Some(theme) = theme {
            let particle_basis = theme
                .expression
                .lexical_root
                .trim_matches(['‘', '’', '“', '”', '\'', '"']);
            let particle = korean_particle(particle_basis, "이", "가");
            push_expression_token(
                &mut output,
                theme,
                format!("{}{particle}", theme.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "어느 대상을",
            "KO.WH.REFERENCE",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            format!("{}는지", predicate.expression.lexical_root),
        );
        push_grammar_token(
            &mut output,
            "알려줘.",
            "KO.REQUEST.REFERENCE_EXPLANATION",
            &clause.event_node_id,
        );
        return output;
    }
    if predicate.expression.concept_id == "C_NAME_TARGET" {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                theme,
                format!("{}{particle}", theme.expression.lexical_root),
            );
        }
        if let Some(single) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                single,
                format!("{}만", single.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            format!(
                "{}해줘.",
                predicate.expression.lexical_root.trim_end_matches('하')
            ),
        );
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_DIALOGUE_ANSWER_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DIALOGUE_ANSWER_RECORD" | "C_DIALOGUE_ANSWER_MODAL" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록에는",
                    "KO.DIALOGUE_ANSWER.RECORD.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                if let Some(property) = property {
                    let particle = korean_direction_particle(&property.expression.lexical_root);
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}{particle}", property.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "남아 있어.".to_string());
            }
            "C_DIALOGUE_ANSWER_NOT_FACT" => push_expression_token(
                &mut output,
                predicate,
                "이건 출처가 있는 대화 기록이지, 사실로 검증된 내용은 아니야.".to_string(),
            ),
            "C_DIALOGUE_ANSWER_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "관련 기록이 서로 충돌해. 어느 출처도 사실의 승자로 고르지 않았어."
                    .to_string(),
            ),
            "C_DIALOGUE_ANSWER_NO_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "현재 일치하는 대화 기록에서는 출처 간 충돌이 확인되지 않아. 그렇다고 명제가 참으로 검증된 것은 아니야."
                    .to_string(),
            ),
            "C_DIALOGUE_ANSWER_PRESUPPOSITION" => {
                push_grammar_token(
                    &mut output,
                    "질문은",
                    "KO.DIALOGUE_ANSWER.PRESUPPOSITION.QUESTION",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle} 전제로 하지만,", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "대화에서는 참으로 검증되지 않았어. 그 전제를 받아들여 답을 만들지 않을게."
                        .to_string(),
                );
            }
            "C_DIALOGUE_ANSWER_NO_MATCH" => {
                if let Some(theme) = theme {
                    push_expression_token(&mut output, theme, format!("‘{}’에 관해서는", theme.expression.lexical_root));
                }
                push_expression_token(
                &mut output,
                predicate,
                "조건에 맞는 대화 기록을 찾지 못했어. 없는 출처나 내용을 추측해서 채우지 않을게."
                    .to_string(),
            ) },
            "C_DIALOGUE_ANSWER_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "어느 출처나 주장을 묻는지 하나로 정해지지 않아. 대상 출처나 내용을 지정해줘."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_INTERACTION_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_INTERACTION_SELF_COMMITMENT" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "네가 직접 하겠다는 약속으로 이해했어.".to_string(),
                );
            }
            "C_INTERACTION_REPORTED_COMMITMENT" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "제3자의 향후 약속을 전한 말로 이해했어. 실제 완료 사실은 아직 아니야."
                        .to_string(),
                );
            }
            "C_INTERACTION_CAPABILITY_QUESTION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "기능 지원 여부를 묻는 질문으로 이해했어. 지원 여부는 확인 가능한 기능 근거로 판단해야 해."
                        .to_string(),
                );
            }
            "C_INTERACTION_DEFERRED_REQUEST" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "조건 충족 뒤에만 가능한 요청으로 기록했어. 지금은 조건 대기 상태라 실행 목표를 활성화하지 않았어."
                        .to_string(),
                );
            }
            "C_INTERACTION_GOAL_WITHDRAWAL" => {
                if let Some(goal) = goal {
                    let particle = korean_particle(&goal.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("{}{particle}", goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "철회한 것으로 반영했어. 철회된 작업은 더 이상 활성 목표가 아니야."
                        .to_string(),
                );
            }
            "C_INTERACTION_WITHDRAWAL_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "철회 요청은 이해했지만 일치하는 활성 작업이 없어 목표 상태를 바꾸지 않았어."
                    .to_string(),
            ),
            "C_INTERACTION_OUTCOME_POLICY" => push_expression_token(
                &mut output,
                predicate,
                "완료·성공·실행은 직접 검증이나 기록된 근거가 있을 때만 말할게. 근거가 없으면 완료로 표현하지 않아."
                    .to_string(),
            ),
            "C_INTERACTION_NO_AUTHORITY" => push_expression_token(
                &mut output,
                predicate,
                "이 해석 자체는 새 실행을 허용하거나 결과를 사실로 확정하지 않아."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_GUARD_") {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_GUARD_UNRESOLVED" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’ 조건은", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "아직 대화 증거로 확인되지 않았어.".to_string(),
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’는 활성화되지 않았어.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_SUPPORTED" => {
                push_grammar_token(
                    &mut output,
                    "대화 증거가",
                    "KO.GUARD.EVIDENCE.SUBJECT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’ 조건을", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "뒷받침해.".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’를 검토할 수 있지만 자동으로 실행되지는 않아.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_CONTRADICTED" => {
                push_grammar_token(
                    &mut output,
                    "대화 증거가",
                    "KO.GUARD.EVIDENCE.SUBJECT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’ 조건과", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "어긋나.".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’는 활성화되지 않았어.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_CONTESTED" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’ 조건을 두고", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "대화 증거가 엇갈려.".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’는 활성화되지 않았어.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_COUNTERFACTUAL" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’는", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "반사실 조건이어서 현재 조건으로 취급하지 않아.".to_string(),
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’는 활성화되지 않았어.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_NO_REVERSE_INFERENCE" => push_expression_token(
                &mut output,
                predicate,
                "결과만 보고 조건이 성립했다고 역추론하거나 실행을 허용하지 않아.".to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_DEFINITION_") {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DEFINITION_BIND_ADDED" | "C_DEFINITION_BIND_CONFIRMED" => {
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "기존 동작 의미",
                    "KO.DEFINITION.BIND.TARGET",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’에", goal.expression.lexical_root),
                    );
                }
                let ending = if predicate.expression.concept_id == "C_DEFINITION_BIND_ADDED" {
                    "연결하고 새 어휘 연결을 추가했어."
                } else {
                    "연결된 같은 어휘 관계로 확인했어."
                };
                push_expression_token(&mut output, predicate, ending.to_string());
            }
            "C_DEFINITION_PAYLOAD_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "이건 이름의 연결만 다룬 것이고, 동작의 뜻이나 실행 권한은 바꾸지 않았어."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "그 표현은 이미 다른 의미에 연결돼 있어 재정의를 거부했어. 기존 의미와 실행 권한은 그대로야."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_NONASSERTED" => push_expression_token(
                &mut output,
                predicate,
                "질문·가정·인용·전언 속 정의는 사용자가 확정한 정의로 받아들이지 않았어."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "정의가 여러 의미 연산자를 가리켜 연결을 보류했어. 한 가지 뜻으로 명확히 정의해줘."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_UNRESOLVED" => push_expression_token(
                &mut output,
                predicate,
                "정의에서 이미 알려진 의미 연산자를 찾지 못해 어휘 연결을 만들지 않았어."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_INVALID_ALIAS" => push_expression_token(
                &mut output,
                predicate,
                "별칭 형식이 유효하지 않아 연결하지 않았어.".to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_DIALOGUE_RELATION_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DIALOGUE_RELATION_CAUSE_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록에서는",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’의 이유로", goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "연결돼 있어.".to_string());
            }
            "C_DIALOGUE_RELATION_RESULT_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록에서는",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’에서", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    let particle = korean_particle(&goal.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’{particle} 결과로", goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "이어진 것으로 남아 있어.".to_string());
            }
            "C_DIALOGUE_RELATION_CONCESSION_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록에서는",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’에도", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    let particle = korean_particle(&goal.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’{particle}", goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "성립한 것으로 연결돼 있어.".to_string());
            }
            "C_DIALOGUE_RELATION_CAUSE_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "이건 대화에서 제시된 이유 연결이지, 실제 인과가 검증됐다는 뜻은 아니야."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_RESULT_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "이건 대화에 기록된 결과 연결이지, 실제 인과를 독립 검증한 것은 아니야."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "이 연결은 어려움과 그럼에도 성립한 결과를 함께 보존할 뿐, 어느 명제도 새 사실로 만들지 않아."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY" => {
                push_grammar_token(
                    &mut output,
                    "이 답은 대화에 기록된",
                    "KO.DIALOGUE_RELATION.PATH.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}개 관계를", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "잇는 경로에서 나왔어. 실제 인과가 검증됐다는 뜻은 아니야.".to_string(),
                );
            }
            "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY" => {
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("대화에는 {}개 관계 경로가 맞아.", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "하나를 유일한 설명으로 고르지 않았고, 어느 경로도 검증된 실제 인과로 취급하지 않아."
                        .to_string(),
                );
            }
            "C_DIALOGUE_RELATION_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "대화 기록에서 질문과 맞는 관계를 찾지 못했어. 없는 원인이나 결과를 추측해서 만들지 않을게."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_NONACTUAL_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "이 경로에는 가능성·가정 같은 비현실 세계의 명제가 포함돼 있어 실제 사건 경로로 볼 수 없어."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_CONTESTED_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "이 경로에는 대화 안에서 다투어지는 명제가 포함돼 있어.".to_string(),
            ),
            "C_DIALOGUE_RELATION_TRUNCATED_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "관계 경로가 안전 홉 한도에서 잘려 더 먼 연결은 포함하지 않았어.".to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_TEMPORAL_ANSWER_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_TEMPORAL_ANSWER_TIME" => {
                push_grammar_token(
                    &mut output,
                    "대화 사건 기록에는",
                    "KO.TEMPORAL_ANSWER.TIME.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "의", "의");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle} 시간은", theme.expression.lexical_root),
                    );
                }
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}로", property.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "남아 있어.".to_string());
            }
            "C_TEMPORAL_ANSWER_EVENT" => {
                push_grammar_token(
                    &mut output,
                    "대화 사건 기록에는",
                    "KO.TEMPORAL_ANSWER.EVENT.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "남아 있어.".to_string());
            }
            "C_TEMPORAL_ANSWER_BEFORE" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록상",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’보다", goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "먼저야.".to_string());
            }
            "C_TEMPORAL_ANSWER_DURING" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록상",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’ 동안", goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "일어난 것으로 연결돼 있어.".to_string(),
                );
            }
            "C_TEMPORAL_ANSWER_SIMULTANEOUS" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록상",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’와", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’는", goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "같은 시점으로 연결돼 있어.".to_string(),
                );
            }
            "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "이 시간 답변은 대화 기록에 근거한 것이고, 실제 세계에서 독립 검증된 사실은 아니야."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY" => {
                push_grammar_token(
                    &mut output,
                    "이 답은 대화의",
                    "KO.TEMPORAL_ANSWER.TRANSITIVE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}개 시간 관계를", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "잇는 경로에서 나왔어. 대화 근거이지 독립 검증된 세계 사실은 아니야."
                        .to_string(),
                );
            }
            "C_TEMPORAL_ANSWER_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "질문의 대상과 일치하는 사건 기록이 없어. 사건을 추측해서 만들지 않을게."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_NO_RELATION" => push_expression_token(
                &mut output,
                predicate,
                "일치하는 사건 기록은 있지만 요청한 시간 관계는 기록되지 않았어. 순서를 추측하지 않을게."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "같은 대상으로 해석될 수 있는 사건 기록이 여러 개야. 어느 사건인지 더 구체적으로 말해줘."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "서로 양립하지 않는 시간 관계 기록이 있어. 어느 순서도 임의로 사실로 고르지 않을게."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_TIME_MISSING" => push_expression_token(
                &mut output,
                predicate,
                "사건 기록은 있지만 사건 시점은 기록되지 않았어. 보고된 대화 차례를 사건 시점으로 바꾸지 않을게."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_ACTIVATE_TOPIC" | "C_ACTIVATE_TOPIC_GROUP"
    ) {
        let return_style =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
                .filter(|property| property.expression.concept_id == "C_TOPIC_RETURN_STYLE");
        if let Some(style) = return_style {
            if let Some(topic) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
            {
                push_expression_token(
                    &mut output,
                    topic,
                    format!("{} 이야기로", topic.expression.lexical_root),
                );
            }
            push_expression_token(&mut output, predicate, "돌아가자.".to_string());
            push_expression_token(
                &mut output,
                style,
                "이제 그 이야기가 현재 화제야.".to_string(),
            );
            return output;
        }
        if let Some(topic) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            let particle_source = topic
                .expression
                .lexical_root
                .trim_matches(['‘', '’', '\'', '"']);
            let particle = korean_particle(particle_source, "을", "를");
            push_expression_token(
                &mut output,
                topic,
                format!(
                    "이제 {}{} 현재 화제로",
                    topic.expression.lexical_root, particle
                ),
            );
        }
        push_expression_token(&mut output, predicate, "둘게.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_TOPIC_CHANGE_BOUNDARY" {
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(&mut output, property, "이건 대화 초점만".to_string());
        }
        push_expression_token(
            &mut output,
            predicate,
            "바꾸는 거야. 작업을 실행한 것은 아니야.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_RETURN_TOPIC" {
        if let Some(topic) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                topic,
                format!("{} 이야기로", topic.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            format!("{}자.", predicate.expression.lexical_root),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_REQUIRE" {
        if let Some(agent) = clause
            .constituents
            .iter()
            .find(|item| item.role == SyntaxConstituentRoleIR::Agent)
        {
            push_grammar_token(
                &mut output,
                "사실로 확인하려면",
                "KO.CONDITION.ESTABLISH_FACT",
                &agent.meaning_node_id,
            );
        }
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
            push_expression_token(
                &mut output,
                theme,
                format!("{}{particle}", theme.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "필요해.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_EXCLUDE_FROM_PLAN" {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                theme,
                format!("{}에 대한", theme.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "금지된 요청은 계획에서 제외했어.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_OBSERVE_CURRENT_STATE" {
        push_grammar_token(
            &mut output,
            "먼저",
            "KO.PLAN.ORDER.FIRST",
            &clause.event_node_id,
        );
    }
    let no_execution_or_result =
        constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected).filter(
            |property| property.expression.concept_id == "C_LIFECYCLE_NO_EXECUTION_OR_RESULT",
        );
    if let Some(property) = no_execution_or_result {
        if let Some(agent) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        {
            push_expression_token(
                &mut output,
                agent,
                format!("{}에는", agent.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, property, "아직 실행 결과는 없어.".to_string());
        if let Some(token) = output.last_mut() {
            token
                .source_meaning_node_ids
                .push(clause.event_node_id.clone());
        }
        return output;
    }
    let result_unavailable =
        constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
            .filter(|property| property.expression.concept_id == "C_LIFECYCLE_RESULT_UNAVAILABLE");
    if let Some(property) = result_unavailable {
        if let Some(agent) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        {
            push_expression_token(
                &mut output,
                agent,
                format!("{}에 관해", agent.expression.lexical_root),
            );
        }
        push_expression_token(
            &mut output,
            property,
            "검증된 실행 결과는 아직 없어.".to_string(),
        );
        if let Some(token) = output.last_mut() {
            token
                .source_meaning_node_ids
                .push(clause.event_node_id.clone());
        }
        return output;
    }
    let adjective_property = clause
        .constituents
        .iter()
        .find(|item| item.role == SyntaxConstituentRoleIR::Property)
        .and_then(|item| {
            selected
                .get(&(item.expression_id.as_str(), item.meaning_node_id.as_str()))
                .copied()
        })
        .filter(|selection| {
            predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula
                && selection.expression.part_of_speech == ExpressionPartOfSpeechIR::Adjective
                && selection.expression.morphology == ExpressionMorphologyClassIR::KoreanHada
        });
    let has_negation = clause
        .constituents
        .iter()
        .any(|item| item.role == SyntaxConstituentRoleIR::Negation);
    for constituent in &clause.constituents {
        if constituent.role == SyntaxConstituentRoleIR::Predicate
            || adjective_property
                .is_some_and(|property| constituent.meaning_node_id == property.meaning_node_id)
        {
            continue;
        }
        let Some(expression) = selected
            .get(&(
                constituent.expression_id.as_str(),
                constituent.meaning_node_id.as_str(),
            ))
            .copied()
        else {
            continue;
        };
        let particle = match constituent.role {
            SyntaxConstituentRoleIR::Agent => {
                korean_particle(&expression.expression.lexical_root, "은", "는")
            }
            SyntaxConstituentRoleIR::Theme => {
                korean_particle(&expression.expression.lexical_root, "을", "를")
            }
            SyntaxConstituentRoleIR::Goal => {
                korean_direction_particle(&expression.expression.lexical_root)
            }
            SyntaxConstituentRoleIR::Property if has_negation => {
                korean_particle(&expression.expression.lexical_root, "은", "는")
            }
            SyntaxConstituentRoleIR::Property => "",
            SyntaxConstituentRoleIR::Possessor => "의",
            SyntaxConstituentRoleIR::Negation => "",
            SyntaxConstituentRoleIR::Predicate => "",
        };
        push_expression_token(
            &mut output,
            expression,
            format!("{}{}", expression.expression.lexical_root, particle),
        );
    }
    if let Some(property) = adjective_property {
        let surface = korean_conjugate(&property.expression, "아");
        push_expression_token(&mut output, property, format!("{surface}."));
        if let Some(token) = output.last_mut() {
            token
                .source_meaning_node_ids
                .push(clause.event_node_id.clone());
        }
        return output;
    }
    let ending = match clause.speech_intent {
        GenerationSpeechIntentIR::CommitFutureAction => "ㄹ게",
        GenerationSpeechIntentIR::Advise => "아야 해요",
        GenerationSpeechIntentIR::Invite => "아 보자",
        GenerationSpeechIntentIR::Ask => "나요",
        GenerationSpeechIntentIR::Acknowledge | GenerationSpeechIntentIR::Inform => {
            match context.register {
                LanguageRegisterIR::Formal => "ㅂ니다",
                _ => "아",
            }
        }
    };
    let mut surface = korean_conjugate(&predicate.expression, ending);
    if predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula
        && !has_negation
        && ending == "아"
    {
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            surface = if has_korean_final_consonant(&property.expression.lexical_root) {
                "이야".to_string()
            } else {
                "야".to_string()
            };
        }
    }
    push_expression_token(&mut output, predicate, format!("{surface}."));
    if predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula {
        if let Some(token) = output.last_mut() {
            token.attach_left = true;
        }
    }
    output
}

fn realize_english_clause(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let predicate = clause
        .constituents
        .iter()
        .find(|item| item.role == SyntaxConstituentRoleIR::Predicate)
        .and_then(|item| {
            selected
                .get(&(item.expression_id.as_str(), item.meaning_node_id.as_str()))
                .copied()
        });
    let Some(predicate) = predicate else {
        return output;
    };
    if predicate.expression.concept_id == "C_CONTENT_PROJECTION" {
        return realize_content_projection(clause, context, selected, predicate);
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_WORLD_CLAUSE_")
    {
        return world_realization::realize_world_clause(clause, context, selected, predicate);
    }
    if predicate.expression.part_of_speech == ExpressionPartOfSpeechIR::Interjection {
        let punctuation = if predicate.expression.concept_id == "C_DIALOGUE_GREETING_REPLY" {
            "!"
        } else {
            "."
        };
        push_expression_token(
            &mut output,
            predicate,
            format!("{}{punctuation}", predicate.expression.lexical_root),
        );
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_PLAN_SUGGESTION_BOUNDARY"
            | "C_PLAN_IMPLICIT_INVESTIGATION"
            | "C_PLAN_IMPLICIT_REPAIR"
            | "C_PLAN_IMPLICIT_EXPLANATION"
            | "C_PLAN_IMPLICIT_PLANNING"
            | "C_SARCASM_INTERPRETATION_BOUNDARY"
            | "C_FIGURATIVE_INTERPRETATION_BOUNDARY"
    ) {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_PLAN_SUGGESTION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("I understood ‘{}’ as", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "an improvement suggestion, not automatic authorization to implement it. I will first clarify its expected benefit and requirements."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_INVESTIGATION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "I understood that you want to know the cause or explanation for ‘{}’.",
                            theme.expression.lexical_root
                        ),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I will start from observable evidence.".to_string(),
                );
            }
            "C_PLAN_IMPLICIT_REPAIR" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "I understood that ‘{}’ needs repair rather than being left as it is.",
                            theme.expression.lexical_root
                        ),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I will first inspect the cause and repair scope, without treating this implicit wording as broader external-mutation authority."
                        .to_string(),
                );
            }
            "C_PLAN_IMPLICIT_EXPLANATION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "I understood that you want an evidence-based explanation or summary of ‘{}’.",
                            theme.expression.lexical_root
                        ),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I will separate confirmed information from what remains unknown.".to_string(),
                );
            }
            "C_PLAN_IMPLICIT_PLANNING" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "I understood that you want options compared and recommended for ‘{}’.",
                            theme.expression.lexical_root
                        ),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I will check the constraints and evidence first, while keeping execution authority separate."
                        .to_string(),
                );
            }
            "C_SARCASM_INTERPRETATION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("{} conflict,", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "so I read this as a negative evaluation or complaint rather than positive approval. I will not derive new action authority from it."
                        .to_string(),
                );
            }
            "C_FIGURATIVE_INTERPRETATION_BOUNDARY" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "I understood ‘{}’ not as a literal action but as",
                            theme.expression.lexical_root
                        ),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("a figurative state of ‘{}’.", goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I will inspect the actual blockage or problem instead of executing the physical reading."
                        .to_string(),
                );
            }
            _ => unreachable!(),
        }
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_OFFER_HELP" {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                theme,
                uppercase_first(&theme.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "can I",
            "EN.DIALOGUE.OFFER_HELP.AUXILIARY",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        push_grammar_token(
            &mut output,
            "you with?",
            "EN.DIALOGUE.OFFER_HELP.COMPLEMENT",
            &clause.event_node_id,
        );
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_INVITE_NEED" {
        push_expression_token(&mut output, predicate, "Tell".to_string());
        push_grammar_token(
            &mut output,
            "me",
            "EN.DIALOGUE.INVITE_NEED.RECIPIENT",
            &clause.event_node_id,
        );
        if let Some(goal) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected) {
            match goal.expression.concept_id.as_str() {
                "C_ADDITIONAL_NEED" => {
                    push_grammar_token(
                        &mut output,
                        "if you need",
                        "EN.DIALOGUE.CONDITION.ADDITIONAL_NEED",
                        &clause.event_node_id,
                    );
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("{}.", goal.expression.lexical_root),
                    );
                }
                "C_FUTURE_NEED" => {
                    push_grammar_token(
                        &mut output,
                        "again",
                        "EN.DIALOGUE.RETURN",
                        &clause.event_node_id,
                    );
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("{}.", goal.expression.lexical_root),
                    );
                }
                _ => push_expression_token(
                    &mut output,
                    goal,
                    format!("{}.", goal.expression.lexical_root),
                ),
            }
        }
        return output;
    }
    if predicate.expression.concept_id == "C_EXCLUDE_FROM_PLAN" {
        push_grammar_token(
            &mut output,
            "I excluded the prohibited request concerning",
            "EN.PLAN.EXCLUSION.SUBJECT_AND_TENSE",
            &clause.event_node_id,
        );
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                theme,
                english_nominal(&theme.expression.lexical_root),
            );
        }
        push_expression_token(&mut output, predicate, "from the plan.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_CONTINUE" {
        push_expression_token(&mut output, predicate, "Continue".to_string());
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                property,
                format!("{}.", property.expression.lexical_root),
            );
        } else if let Some(goal) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                goal,
                format!("{}.", goal.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_DIALOGUE_LISTEN" {
        push_grammar_token(
            &mut output,
            "I'm",
            "EN.DIALOGUE.LISTEN.SPEAKER_PROGRESSIVE",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "listening".to_string());
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_grammar_token(
                &mut output,
                "to",
                "EN.DIALOGUE.LISTEN.THEME",
                &clause.event_node_id,
            );
            push_expression_token(
                &mut output,
                theme,
                format!("{}.", theme.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_INTERPRET" {
        push_grammar_token(
            &mut output,
            "I",
            "EN.GATE.INTERPRET.SPEAKER",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "understand".to_string());
        push_grammar_token(
            &mut output,
            "continuation of",
            "EN.GATE.INTERPRET.CONTINUATION",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, task, task.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "as conditional on the real benefit",
            "EN.GATE.INTERPRET.CONDITION",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}.", benefit.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_VERIFY" {
        push_grammar_token(
            &mut output,
            "First, the real benefit",
            "EN.GATE.VERIFY.ORDER",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                benefit.expression.lexical_root.clone(),
            );
        }
        push_grammar_token(
            &mut output,
            "must be",
            "EN.GATE.VERIFY.MODAL",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "verified.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_CONTINUE" {
        push_grammar_token(
            &mut output,
            "If",
            "EN.GATE.POSITIVE.CONDITION",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                benefit.expression.lexical_root.clone(),
            );
        }
        push_grammar_token(
            &mut output,
            "is supported, I will",
            "EN.GATE.POSITIVE.COMMITMENT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "continue.".to_string());
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_REPORT_ASK_STOP" {
        push_grammar_token(
            &mut output,
            "Otherwise, I will report that and",
            "EN.GATE.NEGATIVE.REPORT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "ask whether to stop".to_string());
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                task,
                format!("{}.", task.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_ASK_UNRESOLVED" {
        push_grammar_token(
            &mut output,
            "If evidence remains unresolved for",
            "EN.GATE.UNKNOWN.CONDITION",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                benefit.expression.lexical_root.clone(),
            );
        }
        push_grammar_token(
            &mut output,
            "I will",
            "EN.GATE.UNKNOWN.SPEAKER",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            "ask instead of guessing.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_NOT_VERIFIED" {
        push_grammar_token(
            &mut output,
            "The required benefit",
            "EN.GATE.PENDING.REQUIRED_BENEFIT",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                benefit.expression.lexical_root.clone(),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "is not directly verified yet.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_PROXY_INSUFFICIENT" {
        push_grammar_token(
            &mut output,
            "I will not authorize a decision to continue",
            "EN.GATE.PENDING.PROXY_INSUFFICIENT",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, task, task.expression.lexical_root.clone());
        }
        push_expression_token(
            &mut output,
            predicate,
            "from a score or proxy alone.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_VERIFY_OR_ASK_STOP" {
        push_grammar_token(
            &mut output,
            "Verify the real outcome",
            "EN.GATE.PENDING.VERIFY",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("for {} first,", benefit.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "or",
            "EN.GATE.PENDING.ALTERNATIVE",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "ask whether to stop".to_string());
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected) {
            push_expression_token(
                &mut output,
                task,
                format!("{} if it remains unresolved.", task.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_RECORD_PROXY" {
        push_grammar_token(
            &mut output,
            "For",
            "EN.GATE.PROXY.TASK",
            &clause.event_node_id,
        );
        if let Some(task) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, task, task.expression.lexical_root.clone());
        }
        push_expression_token(
            &mut output,
            predicate,
            "I recorded the proxy change,".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_GATE_PROXY_NOT_BENEFIT" {
        push_grammar_token(
            &mut output,
            "but it",
            "EN.GATE.PROXY.NOT_BENEFIT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "does not verify".to_string());
        push_grammar_token(
            &mut output,
            "the required real benefit",
            "EN.GATE.PROXY.REAL_BENEFIT",
            &clause.event_node_id,
        );
        if let Some(benefit) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                benefit,
                format!("{}.", benefit.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_ASSESS" {
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        if property.is_some_and(|item| item.expression.concept_id == "C_FEEDBACK_MISUNDERSTOOD") {
            if let Some(property) = property {
                push_expression_token(
                    &mut output,
                    property,
                    uppercase_first(&property.expression.lexical_root),
                );
            }
            push_grammar_token(
                &mut output,
                "in",
                "EN.FEEDBACK.MISUNDERSTANDING.TARGET",
                &clause.event_node_id,
            );
            if let Some(target) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
            {
                push_expression_token(
                    &mut output,
                    target,
                    format!("{}.", target.expression.lexical_root),
                );
            }
            return output;
        }
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                target,
                uppercase_first(&target.expression.lexical_root),
            );
        }
        if let Some(property) = property {
            let missed_point = property.expression.concept_id == "C_FEEDBACK_MISSED_POINT";
            let unhelpful = property.expression.concept_id == "C_FEEDBACK_UNHELPFUL";
            if unhelpful {
                push_grammar_token(
                    &mut output,
                    "wasn't",
                    "EN.FEEDBACK.RETROSPECTIVE.COPULA_NEGATED",
                    &clause.event_node_id,
                );
            } else if !missed_point {
                push_grammar_token(
                    &mut output,
                    "was",
                    "EN.FEEDBACK.RETROSPECTIVE.COPULA",
                    &clause.event_node_id,
                );
            }
            push_expression_token(
                &mut output,
                property,
                if missed_point {
                    property.expression.lexical_root.clone()
                } else if unhelpful {
                    "useful enough.".to_string()
                } else {
                    format!("{}.", property.expression.lexical_root)
                },
            );
            if missed_point {
                push_grammar_token(
                    &mut output,
                    ".",
                    "EN.FEEDBACK.RETROSPECTIVE.CLAUSE_CLOSE",
                    &clause.event_node_id,
                );
                if let Some(token) = output.last_mut() {
                    token.attach_left = true;
                }
            }
        }
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_REQUEST_DETAIL" {
        push_expression_token(&mut output, predicate, "Tell me".to_string());
        if let Some(detail) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                detail,
                format!("{}.", detail.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_CORRECT" {
        push_grammar_token(
            &mut output,
            "I will",
            "EN.FEEDBACK.CORRECTION.COMMITMENT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "correct".to_string());
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, target, target.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "against that.",
            "EN.FEEDBACK.CORRECTION.BASIS",
            &clause.event_node_id,
        );
        return output;
    }
    if predicate.expression.concept_id == "C_FEEDBACK_ADJUST" {
        push_grammar_token(
            &mut output,
            "I will",
            "EN.FEEDBACK.ADJUST.COMMITMENT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "adjust".to_string());
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, target, target.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "to be",
            "EN.FEEDBACK.ADJUST.RESULT",
            &clause.event_node_id,
        );
        if let Some(strategy) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                strategy,
                format!("{}.", strategy.expression.lexical_root),
            );
        }
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_GROUP_ADD_MEMBER" | "C_GROUP_REMOVE_MEMBER" | "C_GROUP_MERGE"
    ) {
        push_grammar_token(
            &mut output,
            "I",
            "EN.DISCOURSE_GROUP.SPEAKER",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            match predicate.expression.concept_id.as_str() {
                "C_GROUP_ADD_MEMBER" => "added".to_string(),
                "C_GROUP_REMOVE_MEMBER" => "removed".to_string(),
                _ => "combined".to_string(),
            },
        );
        if let Some(target) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                target,
                format!("{}.", target.expression.lexical_root),
            );
        }
        if let Some(group) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(&mut output, group, group.expression.lexical_root.clone());
        }
        return output;
    }
    if predicate.expression.concept_id == "C_GROUP_COUNT_STATE" {
        if let Some(group) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                group,
                uppercase_first(&group.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "now",
            "EN.DISCOURSE_GROUP.CURRENT_STATE",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, predicate, "contains".to_string());
        if let Some(count) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                count,
                format!("{} members.", count.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_ASSESS_ACTION_SET" {
        let truth = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected);
        let set = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let quantifier = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let claim = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        let count = constituent_selection(clause, SyntaxConstituentRoleIR::Possessor, selected);
        if let Some(truth) = truth {
            push_expression_token(
                &mut output,
                truth,
                format!("{}.", truth.expression.lexical_root),
            );
        }
        push_grammar_token(
            &mut output,
            "According to the action ledger,",
            "EN.ACTION_SET.LEDGER_BASIS",
            &clause.event_node_id,
        );
        if let Some(quantifier) = quantifier {
            push_expression_token(
                &mut output,
                quantifier,
                quantifier.expression.lexical_root.clone(),
            );
            if matches!(
                quantifier.expression.concept_id.as_str(),
                "C_ACTION_SET_ANY" | "C_ACTION_SET_NONE"
            ) {
                push_grammar_token(
                    &mut output,
                    "the",
                    "EN.ACTION_SET.PARTITIVE",
                    &clause.event_node_id,
                );
            }
        }
        if let Some(count) = count {
            push_expression_token(&mut output, count, count.expression.lexical_root.clone());
        }
        if let Some(set) = set {
            push_expression_token(&mut output, set, set.expression.lexical_root.clone());
        }
        if let Some(claim) = claim {
            let singular =
                quantifier.is_some_and(|item| item.expression.concept_id == "C_ACTION_SET_ANY");
            let surface = if singular {
                claim
                    .expression
                    .lexical_root
                    .strip_prefix("are ")
                    .map(|rest| format!("is {rest}"))
                    .or_else(|| {
                        claim
                            .expression
                            .lexical_root
                            .strip_prefix("have ")
                            .map(|rest| format!("has {rest}"))
                    })
                    .unwrap_or_else(|| claim.expression.lexical_root.clone())
            } else {
                claim.expression.lexical_root.clone()
            };
            push_expression_token(&mut output, claim, format!("{surface}."));
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_CLARIFY_") {
        let detail = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        match predicate.expression.concept_id.as_str() {
            "C_CLARIFY_PENDING_CHOICE" => push_expression_token(
                &mut output,
                predicate,
                "Please select one of the options from my previous question directly.".to_string(),
            ),
            "C_CLARIFY_ORDERED_PAIR" => push_expression_token(
                &mut output,
                predicate,
                "Please name the two items that ‘former’ and ‘latter’ should denote.".to_string(),
            ),
            "C_CLARIFY_LOCAL_ORDINAL" => push_expression_token(
                &mut output,
                predicate,
                "Please confirm which numbered item you mean.".to_string(),
            ),
            "C_CLARIFY_EVENT_ORDINAL" => push_expression_token(
                &mut output,
                predicate,
                "Please confirm which step of the earlier plan you mean.".to_string(),
            ),
            "C_CLARIFY_PREVIOUS_TOPIC" => push_expression_token(
                &mut output,
                predicate,
                "Please name the earlier topic you want to return to.".to_string(),
            ),
            "C_CLARIFY_COMPETING_REQUEST" => {
                push_grammar_token(
                    &mut output,
                    "The sentence supports",
                    "EN.CLARIFY.COMPETITION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(detail) = detail {
                    push_expression_token(
                        &mut output,
                        detail,
                        detail.expression.lexical_root.clone(),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "with similar strength. Which one is the actual request?".to_string(),
                );
            }
            "C_CLARIFY_NONLITERAL_READING" => {
                push_grammar_token(
                    &mut output,
                    "Did you mean",
                    "EN.CLARIFY.NONLITERAL.QUESTION",
                    &clause.event_node_id,
                );
                if let Some(detail) = detail {
                    push_expression_token(
                        &mut output,
                        detail,
                        format!("‘{}’", detail.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "literally, or as a figurative description of a problem?".to_string(),
                );
            }
            "C_CLARIFY_VOICE_ALTERNATIVE" => {
                push_grammar_token(
                    &mut output,
                    "The voice input could be",
                    "EN.CLARIFY.VOICE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(detail) = detail {
                    push_expression_token(
                        &mut output,
                        detail,
                        format!("{}.", detail.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "Which one did you mean?".to_string(),
                );
            }
            _ => push_expression_token(
                &mut output,
                predicate,
                "Could you add a little more detail about what you want?".to_string(),
            ),
        }
        return output;
    }
    if predicate.expression.concept_id == "C_RESOLVE_REFERENCE" {
        push_grammar_token(
            &mut output,
            "Which target",
            "EN.WH.REFERENCE",
            &clause.event_node_id,
        );
        push_grammar_token(
            &mut output,
            "does",
            "EN.AUX.QUESTION",
            &clause.event_node_id,
        );
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, theme, theme.expression.lexical_root.clone());
        }
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        push_grammar_token(
            &mut output,
            "to?",
            "EN.PREP.REFERENCE",
            &clause.event_node_id,
        );
        return output;
    }
    if predicate.expression.concept_id == "C_NAME_TARGET" {
        push_grammar_token(
            &mut output,
            "Please",
            "EN.POLITE.REQUEST",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        if let Some(single) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(&mut output, single, single.expression.lexical_root.clone());
        }
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(
                &mut output,
                theme,
                format!("{}.", theme.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_INTERACTION_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_INTERACTION_SELF_COMMITMENT" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("I understood ‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "as your own commitment.".to_string(),
                );
            }
            "C_INTERACTION_REPORTED_COMMITMENT" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("I understood ‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "as a report of a third party's future commitment. It does not establish completion."
                        .to_string(),
                );
            }
            "C_INTERACTION_CAPABILITY_QUESTION" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("I understood ‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "as a capability question. Support must be determined from inspectable capability evidence."
                        .to_string(),
                );
            }
            "C_INTERACTION_DEFERRED_REQUEST" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("I recorded ‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "as a condition-pending request. The action is not active until the antecedent is verified."
                        .to_string(),
                );
            }
            "C_INTERACTION_GOAL_WITHDRAWAL" => {
                push_grammar_token(
                    &mut output,
                    "I applied the withdrawal to",
                    "EN.INTERACTION.WITHDRAWAL.OPENING",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("{}.", goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "The retired work is no longer an active goal.".to_string(),
                );
            }
            "C_INTERACTION_WITHDRAWAL_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "I understood the withdrawal request, but no active work matched, so I left the goal state unchanged."
                    .to_string(),
            ),
            "C_INTERACTION_OUTCOME_POLICY" => push_expression_token(
                &mut output,
                predicate,
                "I will claim completion, success, or execution only from direct verification or recorded evidence. Without that evidence, I will not describe the result as complete."
                    .to_string(),
            ),
            "C_INTERACTION_NO_AUTHORITY" => push_expression_token(
                &mut output,
                predicate,
                "This interpretation does not authorize a new execution or establish an outcome as fact."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_GUARD_") {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_GUARD_UNRESOLVED" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("The condition ‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "is not yet established by dialogue evidence.".to_string(),
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "Therefore, ‘{}’ is not active.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_SUPPORTED" => {
                push_grammar_token(
                    &mut output,
                    "Dialogue evidence",
                    "EN.GUARD.EVIDENCE.SUBJECT",
                    &clause.event_node_id,
                );
                push_expression_token(&mut output, predicate, "supports".to_string());
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("the condition ‘{}’.", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "Therefore, ‘{}’ may be considered, but it does not run automatically.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_CONTRADICTED" => {
                push_grammar_token(
                    &mut output,
                    "Dialogue evidence",
                    "EN.GUARD.EVIDENCE.SUBJECT",
                    &clause.event_node_id,
                );
                push_expression_token(&mut output, predicate, "contradicts".to_string());
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("the condition ‘{}’.", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "Therefore, ‘{}’ is not active.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_CONTESTED" => {
                push_grammar_token(
                    &mut output,
                    "Dialogue evidence conflicts over",
                    "EN.GUARD.EVIDENCE.CONFLICT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("the condition ‘{}’.", theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "Therefore, ‘{}’ is not active.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_COUNTERFACTUAL" => {
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "is counterfactual, so it is not treated as a current condition.".to_string(),
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "Therefore, ‘{}’ is not active.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_NO_REVERSE_INFERENCE" => push_expression_token(
                &mut output,
                predicate,
                "Observing the result alone cannot establish the condition or authorize execution."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate.expression.concept_id.starts_with("C_DEFINITION_") {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DEFINITION_BIND_ADDED" | "C_DEFINITION_BIND_CONFIRMED" => {
                let opening = if predicate.expression.concept_id == "C_DEFINITION_BIND_ADDED" {
                    "I linked"
                } else {
                    "I confirmed the lexical link from"
                };
                push_grammar_token(
                    &mut output,
                    opening,
                    "EN.DEFINITION.BIND.OPENING",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "to the known action meaning",
                    "EN.DEFINITION.BIND.TARGET",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_DEFINITION_PAYLOAD_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "This only concerns the label link; the action's meaning and permission to execute remain unchanged."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "I rejected the redefinition because the label already has a different binding. Its existing meaning and execution authority remain unchanged."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_NONASSERTED" => push_expression_token(
                &mut output,
                predicate,
                "I did not treat a questioned, hypothetical, quoted, or reported definition as the user's asserted definition."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "The definition points to multiple semantic operators, so I left it unbound. Please define one meaning explicitly."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_UNRESOLVED" => push_expression_token(
                &mut output,
                predicate,
                "I could not ground the definition to an existing semantic operator, so I created no lexical binding."
                    .to_string(),
            ),
            "C_DEFINITION_REJECT_INVALID_ALIAS" => push_expression_token(
                &mut output,
                predicate,
                "I rejected the binding because the alias form is invalid.".to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_DIALOGUE_RELATION_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DIALOGUE_RELATION_CAUSE_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue record links",
                    "EN.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "as a reason for",
                    "EN.DIALOGUE_RELATION.CAUSE.LINK",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_DIALOGUE_RELATION_RESULT_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue record links",
                    "EN.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "to the result",
                    "EN.DIALOGUE_RELATION.RESULT.LINK",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_DIALOGUE_RELATION_CONCESSION_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue record links",
                    "EN.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "to the outcome that still held:",
                    "EN.DIALOGUE_RELATION.CONCESSION.LINK",
                    &clause.event_node_id,
                );
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_DIALOGUE_RELATION_CAUSE_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "This is a reason link asserted in the dialogue; it does not establish actual causation."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_RESULT_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "This is a result link recorded in the dialogue, not independently verified causation."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "This link preserves the difficulty and the outcome that still held; it does not establish either proposition as a new fact."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY" => {
                push_grammar_token(
                    &mut output,
                    "This answer follows a",
                    "EN.DIALOGUE_RELATION.PATH.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}-link path", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "recorded in the dialogue; it does not establish actual causation."
                        .to_string(),
                );
            }
            "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY" => {
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{} dialogue-relation paths match.", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "I did not select one as the unique explanation, and none is treated as verified actual causation."
                        .to_string(),
                );
            }
            "C_DIALOGUE_RELATION_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "I found no matching relation in the dialogue record. I will not invent a cause or result."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_NONACTUAL_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "The path contains a possible or hypothetical proposition and is not an actual-event path."
                    .to_string(),
            ),
            "C_DIALOGUE_RELATION_CONTESTED_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "The path also contains a proposition contested in the dialogue.".to_string(),
            ),
            "C_DIALOGUE_RELATION_TRUNCATED_WARNING" => push_expression_token(
                &mut output,
                predicate,
                "The relation path reached the safe hop limit, so more distant links are omitted."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_DIALOGUE_ANSWER_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_DIALOGUE_ANSWER_RECORD" | "C_DIALOGUE_ANSWER_MODAL" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue record stores",
                    "EN.DIALOGUE_ANSWER.RECORD.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "as",
                    "EN.DIALOGUE_ANSWER.RECORD.AS",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}.", property.expression.lexical_root),
                    );
                } else {
                    push_expression_token(
                        &mut output,
                        predicate,
                        "a sourced statement.".to_string(),
                    );
                }
            }
            "C_DIALOGUE_ANSWER_NOT_FACT" => push_expression_token(
                &mut output,
                predicate,
                "This is a source-attributed dialogue record, not an established fact."
                    .to_string(),
            ),
            "C_DIALOGUE_ANSWER_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "The matching records conflict. No source has been selected as the truth winner."
                    .to_string(),
            ),
            "C_DIALOGUE_ANSWER_NO_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "No conflict appears in the matching dialogue records. That does not establish the proposition itself as true."
                    .to_string(),
            ),
            "C_DIALOGUE_ANSWER_PRESUPPOSITION" => {
                push_grammar_token(
                    &mut output,
                    "The question presupposes",
                    "EN.DIALOGUE_ANSWER.PRESUPPOSITION.QUESTION",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’,", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "but the dialogue does not establish that premise as true. I will not build an answer by silently accepting it."
                        .to_string(),
                );
            }
            "C_DIALOGUE_ANSWER_NO_MATCH" => {
                if let Some(theme) = theme {
                    push_grammar_token(&mut output, "Regarding", "EN.ANSWER_GAP.TOPIC", &clause.event_node_id);
                    push_expression_token(&mut output, theme, format!("‘{}’,", theme.expression.lexical_root));
                }
                push_expression_token(&mut output, predicate,
                    "I found no matching dialogue record. I will not invent a source or proposition to fill the gap.".to_string());
            },
            "C_DIALOGUE_ANSWER_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "The question does not identify one source or proposition. Please specify the source or content."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if predicate
        .expression
        .concept_id
        .starts_with("C_TEMPORAL_ANSWER_")
    {
        let theme = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
        let goal = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
        let property = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected);
        match predicate.expression.concept_id.as_str() {
            "C_TEMPORAL_ANSWER_TIME" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue event record gives the time of",
                    "EN.TEMPORAL_ANSWER.TIME.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_grammar_token(
                    &mut output,
                    "as",
                    "EN.TEMPORAL_ANSWER.TIME.AS",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}.", property.expression.lexical_root),
                    );
                }
            }
            "C_TEMPORAL_ANSWER_EVENT" => {
                push_grammar_token(
                    &mut output,
                    "The dialogue contains the event record",
                    "EN.TEMPORAL_ANSWER.EVENT.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’.", theme.expression.lexical_root),
                    );
                }
            }
            "C_TEMPORAL_ANSWER_BEFORE" => {
                push_grammar_token(
                    &mut output,
                    "In the dialogue temporal record,",
                    "EN.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "comes before".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_TEMPORAL_ANSWER_DURING" => {
                push_grammar_token(
                    &mut output,
                    "In the dialogue temporal record,",
                    "EN.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "occurs during".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_TEMPORAL_ANSWER_SIMULTANEOUS" => {
                push_grammar_token(
                    &mut output,
                    "In the dialogue temporal record,",
                    "EN.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’", theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "is simultaneous with".to_string());
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        format!("‘{}’.", goal.expression.lexical_root),
                    );
                }
            }
            "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY" => push_expression_token(
                &mut output,
                predicate,
                "This temporal answer is grounded in dialogue records, not independently verified world truth."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY" => {
                push_grammar_token(
                    &mut output,
                    "This answer follows a",
                    "EN.TEMPORAL_ANSWER.TRANSITIVE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        format!("{}-edge temporal path", property.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "in the dialogue record; it is not independently verified world truth."
                        .to_string(),
                );
            }
            "C_TEMPORAL_ANSWER_NO_MATCH" => push_expression_token(
                &mut output,
                predicate,
                "There is no matching event record. I will not invent an event.".to_string(),
            ),
            "C_TEMPORAL_ANSWER_NO_RELATION" => push_expression_token(
                &mut output,
                predicate,
                "Matching event records exist, but the requested temporal relation is not recorded. I will not infer the order."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_AMBIGUOUS" => push_expression_token(
                &mut output,
                predicate,
                "Several event records match the target. Please specify which event you mean."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_CONFLICT" => push_expression_token(
                &mut output,
                predicate,
                "The dialogue contains incompatible temporal relation records. I will not silently choose either order as fact."
                    .to_string(),
            ),
            "C_TEMPORAL_ANSWER_TIME_MISSING" => push_expression_token(
                &mut output,
                predicate,
                "The event is recorded, but its event time is not. I will not substitute dialogue turn order for event time."
                    .to_string(),
            ),
            _ => {}
        }
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_ACTIVATE_TOPIC" | "C_ACTIVATE_TOPIC_GROUP"
    ) {
        let return_style =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
                .filter(|property| property.expression.concept_id == "C_TOPIC_RETURN_STYLE");
        if let Some(style) = return_style {
            push_grammar_token(
                &mut output,
                "Let's",
                "EN.HORTATIVE.LETS",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, predicate, "return".to_string());
            push_grammar_token(&mut output, "to", "EN.PREP.GOAL", &clause.event_node_id);
            if let Some(topic) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
            {
                push_expression_token(
                    &mut output,
                    topic,
                    format!("the {} topic.", topic.expression.lexical_root),
                );
            }
            push_expression_token(
                &mut output,
                style,
                "It is now the active topic.".to_string(),
            );
            return output;
        }
        if let Some(topic) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_grammar_token(
                &mut output,
                "The",
                "EN.DETERMINER.DEFINITE",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, topic, topic.expression.lexical_root.clone());
        }
        push_expression_token(
            &mut output,
            predicate,
            "is now the active topic.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_TOPIC_CHANGE_BOUNDARY" {
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            push_expression_token(
                &mut output,
                property,
                "This only changes the conversation focus;".to_string(),
            );
        }
        push_expression_token(
            &mut output,
            predicate,
            "it does not execute any work.".to_string(),
        );
        return output;
    }
    if predicate.expression.concept_id == "C_RETURN_TOPIC" {
        push_grammar_token(
            &mut output,
            "Let's",
            "EN.HORTATIVE.LETS",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        push_grammar_token(&mut output, "to", "EN.PREP.GOAL", &clause.event_node_id);
        if let Some(topic) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
        {
            push_expression_token(
                &mut output,
                topic,
                format!("the {} topic.", topic.expression.lexical_root),
            );
        }
        return output;
    }
    if predicate.expression.concept_id == "C_REQUIRE" {
        push_expression_token(&mut output, predicate, "We would need".to_string());
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, theme, theme.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "before it became",
            "EN.CONDITION.ESTABLISH_FACT",
            &clause.event_node_id,
        );
        if let Some(agent) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        {
            push_expression_token(&mut output, agent, "an established fact.".to_string());
        }
        return output;
    }
    let result_unavailable =
        constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
            .filter(|property| property.expression.concept_id == "C_LIFECYCLE_RESULT_UNAVAILABLE");
    if let Some(property) = result_unavailable {
        push_expression_token(&mut output, property, "No execution result".to_string());
        push_grammar_token(
            &mut output,
            "is recorded yet for",
            "EN.LIFECYCLE.RESULT_UNAVAILABLE",
            &clause.event_node_id,
        );
        if let Some(agent) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        {
            push_expression_token(
                &mut output,
                agent,
                format!("{}.", english_nominal(&agent.expression.lexical_root)),
            );
        }
        return output;
    }
    let no_execution_or_result =
        constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected).filter(
            |property| property.expression.concept_id == "C_LIFECYCLE_NO_EXECUTION_OR_RESULT",
        );
    if let Some(property) = no_execution_or_result {
        push_expression_token(
            &mut output,
            property,
            "No execution result is recorded".to_string(),
        );
        push_grammar_token(
            &mut output,
            "for",
            "EN.LIFECYCLE.NO_EXECUTION_OR_RESULT",
            &clause.event_node_id,
        );
        if let Some(agent) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
        {
            push_expression_token(
                &mut output,
                agent,
                format!(
                    "{}, so it has not been verified as executed.",
                    english_nominal(&agent.expression.lexical_root)
                ),
            );
        }
        return output;
    }
    let agent = clause
        .constituents
        .iter()
        .find(|item| item.role == SyntaxConstituentRoleIR::Agent)
        .and_then(|item| {
            selected
                .get(&(item.expression_id.as_str(), item.meaning_node_id.as_str()))
                .copied()
        });
    let implicit_agent = match clause.speech_intent {
        GenerationSpeechIntentIR::CommitFutureAction => Some("I"),
        GenerationSpeechIntentIR::Advise => Some("you"),
        GenerationSpeechIntentIR::Invite => Some("we"),
        _ => None,
    };
    if predicate.expression.concept_id == "C_OBSERVE_CURRENT_STATE" {
        push_grammar_token(
            &mut output,
            "First,",
            "EN.PLAN.ORDER.FIRST",
            &clause.event_node_id,
        );
    }
    if let Some(agent) = agent {
        push_expression_token(
            &mut output,
            agent,
            english_nominal(&agent.expression.lexical_root),
        );
    } else if let Some(agent) = implicit_agent {
        push_grammar_token(
            &mut output,
            agent,
            "EN.IMPLICIT_SPEAKER",
            &clause.event_node_id,
        );
    }
    let modal = match clause.speech_intent {
        GenerationSpeechIntentIR::CommitFutureAction => Some(("will", "EN.MODAL.FUTURE")),
        GenerationSpeechIntentIR::Advise => Some(("should", "EN.MODAL.ADVICE")),
        GenerationSpeechIntentIR::Invite => Some(("can", "EN.MODAL.INVITATION")),
        _ => None,
    };
    if let Some((surface, rule)) = modal {
        push_grammar_token(&mut output, surface, rule, &clause.event_node_id);
    }
    let predicate_surface = match predicate.expression.morphology {
        ExpressionMorphologyClassIR::EnglishCopula => "is".to_string(),
        _ => predicate.expression.lexical_root.clone(),
    };
    push_expression_token(&mut output, predicate, predicate_surface);
    for role in [
        SyntaxConstituentRoleIR::Theme,
        SyntaxConstituentRoleIR::Negation,
        SyntaxConstituentRoleIR::Possessor,
        SyntaxConstituentRoleIR::Goal,
        SyntaxConstituentRoleIR::Property,
    ] {
        for constituent in clause.constituents.iter().filter(|item| item.role == role) {
            let Some(expression) = selected
                .get(&(
                    constituent.expression_id.as_str(),
                    constituent.meaning_node_id.as_str(),
                ))
                .copied()
            else {
                continue;
            };
            if role == SyntaxConstituentRoleIR::Goal {
                push_grammar_token(
                    &mut output,
                    "to",
                    "EN.PREP.GOAL",
                    &constituent.meaning_node_id,
                );
            }
            if role == SyntaxConstituentRoleIR::Possessor {
                push_grammar_token(
                    &mut output,
                    "of",
                    "EN.PREP.POSSESSOR",
                    &constituent.meaning_node_id,
                );
            }
            let surface = if role == SyntaxConstituentRoleIR::Property
                && expression.expression.concept_id == "C_CONFIRMED_FACT"
            {
                format!("a {}", expression.expression.lexical_root)
            } else if matches!(
                role,
                SyntaxConstituentRoleIR::Property | SyntaxConstituentRoleIR::Negation
            ) {
                expression.expression.lexical_root.clone()
            } else {
                english_nominal(&expression.expression.lexical_root)
            };
            push_expression_token(&mut output, expression, surface);
        }
    }
    if let Some(last) = output.last_mut() {
        last.surface.push('.');
    }
    output
}

fn push_expression_token(
    output: &mut Vec<MorphologicalTokenIR>,
    selection: &ExpressionSelectionIR,
    surface: String,
) {
    output.push(MorphologicalTokenIR {
        token_index: 0,
        surface,
        attach_left: false,
        expression_id: Some(selection.expression.expression_id.clone()),
        grammar_rule_id: None,
        source_meaning_node_ids: vec![selection.meaning_node_id.clone()],
    });
}

fn constituent_selection<'a>(
    clause: &SyntaxClauseIR,
    role: SyntaxConstituentRoleIR,
    selected: &'a BTreeMap<(&str, &str), &ExpressionSelectionIR>,
) -> Option<&'a ExpressionSelectionIR> {
    clause
        .constituents
        .iter()
        .find(|item| item.role == role)
        .and_then(|item| {
            selected
                .get(&(item.expression_id.as_str(), item.meaning_node_id.as_str()))
                .copied()
        })
}

fn push_grammar_token(
    output: &mut Vec<MorphologicalTokenIR>,
    surface: &str,
    rule: &str,
    source_node_id: &str,
) {
    output.push(MorphologicalTokenIR {
        token_index: 0,
        surface: surface.to_string(),
        attach_left: false,
        expression_id: None,
        grammar_rule_id: Some(rule.to_string()),
        source_meaning_node_ids: vec![source_node_id.to_string()],
    });
}

fn join_morphological_tokens(tokens: &[MorphologicalTokenIR], language: LanguageCodeIR) -> String {
    let mut text = String::new();
    for token in tokens {
        if token.surface.is_empty() {
            continue;
        }
        if !text.is_empty() && !token.attach_left {
            text.push(' ');
        }
        text.push_str(&token.surface);
    }
    if language == LanguageCodeIR::Korean {
        text = text.replace(" .", ".");
    }
    text
}

fn korean_particle<'a>(surface: &str, consonant: &'a str, vowel: &'a str) -> &'a str {
    if has_korean_final_consonant(surface) {
        consonant
    } else {
        vowel
    }
}

fn korean_direction_particle(surface: &str) -> &'static str {
    let Some(last) = surface.chars().last() else {
        return "로";
    };
    if ('가'..='힣').contains(&last) {
        let jong = (u32::from(last) - u32::from('가')) % 28;
        if jong == 0 || jong == 8 {
            "로"
        } else {
            "으로"
        }
    } else {
        "로"
    }
}

fn has_korean_final_consonant(surface: &str) -> bool {
    surface
        .chars()
        .last()
        .filter(|last| ('가'..='힣').contains(last))
        .is_some_and(|last| (u32::from(last) - u32::from('가')) % 28 != 0)
}

fn korean_conjugate(expression: &ExpressionNodeIR, ending: &str) -> String {
    match expression.morphology {
        ExpressionMorphologyClassIR::KoreanHada => match ending {
            "ㄹ게" => format!("{}할게", expression.lexical_root.trim_end_matches('하')),
            "아야 해요" => format!(
                "{}해야 해요",
                expression.lexical_root.trim_end_matches('하')
            ),
            "나요" => format!("{}하나요", expression.lexical_root.trim_end_matches('하')),
            "ㅂ니다" => format!("{}합니다", expression.lexical_root.trim_end_matches('하')),
            "아 보자" => format!("{}해 보자", expression.lexical_root.trim_end_matches('하')),
            _ => format!("{}해", expression.lexical_root.trim_end_matches('하')),
        },
        ExpressionMorphologyClassIR::KoreanCopula => match ending {
            "ㅂ니다" => "입니다".to_string(),
            "나요" => "인가요".to_string(),
            _ => "야".to_string(),
        },
        ExpressionMorphologyClassIR::KoreanInvariable => match ending {
            "ㄹ게" => korean_future_commitment(&expression.lexical_root),
            "아야 해요" => format!("{}어야 해요", expression.lexical_root),
            "나요" => format!("{}나요", expression.lexical_root),
            "ㅂ니다" => format!("{}습니다", expression.lexical_root),
            "아 보자" => format!("{}어 보자", expression.lexical_root),
            _ => format!("{}어", expression.lexical_root),
        },
        _ => format!("{}{}", expression.lexical_root, ending),
    }
}

fn korean_future_commitment(root: &str) -> String {
    let Some(last) = root.chars().last() else {
        return "게".to_string();
    };
    if !('가'..='힣').contains(&last) {
        return format!("{root}할게");
    }
    let jong = (u32::from(last) - u32::from('가')) % 28;
    if jong == 8 {
        format!("{root}게")
    } else if jong == 0 {
        let stem = &root[..root.len() - last.len_utf8()];
        let with_rieul = char::from_u32(u32::from(last) + 8).unwrap_or(last);
        format!("{stem}{with_rieul}게")
    } else {
        format!("{root}을게")
    }
}

fn english_nominal(root: &str) -> String {
    if matches!(root, "I" | "you" | "this" | "that" | "it")
        || root.starts_with('“')
        || root.starts_with('"')
        || root.starts_with("the ")
        || root.starts_with("this ")
        || root.starts_with("a ")
        || root.starts_with("an ")
    {
        root.to_string()
    } else {
        format!("the {root}")
    }
}

fn uppercase_first(text: &str) -> String {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(characters).collect()
}

fn verify_generation(
    meaning: &GenerationMeaningGraphIR,
    expressions: &ExpressionSelectionGraphIR,
    syntax: &SyntaxPlanIR,
    morphology: &MorphologicalRealizationIR,
) -> GenerationVerificationIR {
    let covered_meaning_node_ids = morphology
        .tokens
        .iter()
        .flat_map(|token| token.source_meaning_node_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let covered_set = covered_meaning_node_ids.iter().collect::<BTreeSet<_>>();
    let unresolved_meaning_node_ids = meaning
        .nodes
        .iter()
        .filter(|node| !covered_set.contains(&node.node_id))
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let covered_meaning_edge_ids = syntax
        .clauses
        .iter()
        .flat_map(|clause| clause.source_edge_ids.iter().cloned())
        .chain(
            meaning
                .edges
                .iter()
                .filter(|edge| edge.relation == GenerationMeaningRelationIR::Sequence)
                .map(|edge| edge.edge_id.clone()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let covered_edges = covered_meaning_edge_ids.iter().collect::<BTreeSet<_>>();
    let unsupported_claims = meaning
        .edges
        .iter()
        .filter(|edge| !covered_edges.contains(&edge.edge_id))
        .count();
    let unsupported_surface_tokens = morphology
        .tokens
        .iter()
        .filter(|token| token.expression_id.is_none() && token.grammar_rule_id.is_none())
        .count();
    let faithful = unresolved_meaning_node_ids.is_empty()
        && expressions.unresolved_meaning_node_ids.is_empty()
        && unsupported_claims == 0
        && unsupported_surface_tokens == 0
        && !morphology.realized_text.trim().is_empty();
    GenerationVerificationIR {
        covered_meaning_node_ids,
        covered_meaning_edge_ids,
        unresolved_meaning_node_ids,
        unsupported_surface_tokens,
        unsupported_claims,
        semantic_roundtrip_sha256: if faithful {
            meaning.semantic_sha256.clone()
        } else {
            String::new()
        },
        faithful,
    }
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).unwrap_or_default())
    )
}

fn expression(
    id: &str,
    language: LanguageCodeIR,
    concept: &str,
    root: &str,
    part_of_speech: ExpressionPartOfSpeechIR,
    morphology: ExpressionMorphologyClassIR,
    register: LanguageRegisterIR,
) -> ExpressionNodeIR {
    ExpressionNodeIR {
        expression_id: id.to_string(),
        language,
        concept_id: concept.to_string(),
        lexical_root: root.to_string(),
        part_of_speech,
        morphology,
        register,
        confidence_millis: 1_000,
        provenance: "B_CORE_BUILTIN_EXPRESSION_KNOWLEDGE_V1".to_string(),
    }
}

fn builtin_expression_nodes() -> Vec<ExpressionNodeIR> {
    use ExpressionMorphologyClassIR::{
        EnglishCopula, EnglishInvariable, EnglishRegular, KoreanCopula, KoreanHada,
        KoreanInvariable,
    };
    use ExpressionPartOfSpeechIR::{Adjective, Interjection, Noun, Verb};
    use LanguageCodeIR::{English, Korean};
    use LanguageRegisterIR::{Informal, Neutral};
    let concepts = [
        (
            "C_ACKNOWLEDGE",
            "알겠어",
            "Got it",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_DIALOGUE_HOLD_ACK",
            "응",
            "Okay",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_DIALOGUE_GREETING_REPLY",
            "안녕",
            "Hi",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_DIALOGUE_GRATITUDE_REPLY",
            "천만에",
            "You're welcome",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_DIALOGUE_FAREWELL_REPLY",
            "좋아",
            "Sounds good",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_DIALOGUE_OFFER_HELP",
            "도와주",
            "help",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Informal,
        ),
        (
            "C_DIALOGUE_INVITE_NEED",
            "말하",
            "tell",
            Verb,
            KoreanHada,
            EnglishRegular,
            Informal,
        ),
        (
            "C_DIALOGUE_CONTINUE",
            "이어 말하",
            "continue",
            Verb,
            KoreanHada,
            EnglishRegular,
            Informal,
        ),
        (
            "C_DIALOGUE_LISTEN",
            "듣고 있",
            "listen",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Informal,
        ),
        (
            "C_OPEN_NEED",
            "무엇",
            "what",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ADDITIONAL_NEED",
            "더 필요한 것",
            "anything else",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FUTURE_NEED",
            "필요하면",
            "when you need help",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_WHEN_READY",
            "준비되면",
            "when you're ready",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_UNHURRIED_PACE",
            "천천히",
            "at your own pace",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_USER_TURN",
            "네 말",
            "you",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_INTERPRET",
            "이해하",
            "understand",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GATE_VERIFY",
            "검증하",
            "verify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GATE_CONTINUE",
            "계속하",
            "continue",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GATE_REPORT_ASK_STOP",
            "중단 여부를 묻",
            "ask whether to stop",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_ASK_UNRESOLVED",
            "확인을 요청하",
            "ask instead of guessing",
            Verb,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_NOT_VERIFIED",
            "직접 확인되지 않",
            "is not directly verified",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_PROXY_INSUFFICIENT",
            "대리 지표만으로 충분하지 않",
            "is insufficient from a proxy alone",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_VERIFY_OR_ASK_STOP",
            "검증하거나 중단 여부를 묻",
            "verify or ask whether to stop",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GATE_RECORD_PROXY",
            "대리 지표 변화를 기록하",
            "record the proxy change",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GATE_PROXY_NOT_BENEFIT",
            "실제 이득을 확인하지 못하",
            "does not verify the real benefit",
            Verb,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_ASSESS",
            "평가하",
            "assess",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_FEEDBACK_UNHELPFUL",
            "도움이 되지 않았",
            "not useful enough",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_MISUNDERSTOOD",
            "네 말을 잘못 이해했",
            "I misunderstood you",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_MISSED_POINT",
            "핵심을 놓쳤",
            "missed your point",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_TOO_VERBOSE",
            "너무 길었",
            "too long",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_TOO_BRIEF",
            "너무 짧았",
            "too brief",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_INCORRECT",
            "정확하지 않았",
            "incorrect",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_REQUEST_DETAIL",
            "짚",
            "tell",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Informal,
        ),
        (
            "C_FEEDBACK_MISSING_DETAIL",
            "어긋난 부분",
            "what missed the mark",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_CORRECT",
            "바로잡",
            "correct",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_FEEDBACK_ADJUST",
            "조정하",
            "adjust",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_FEEDBACK_CONCISE",
            "핵심만 짧게",
            "concise and focused",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_DETAIL_CONTEXT",
            "필요한 근거와 맥락을 더 자세히",
            "the needed detail and context",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FEEDBACK_VERIFY_CORRECT",
            "틀린 부분을 확인해서",
            "after checking what was wrong",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_GROUP_ADD_MEMBER",
            "추가하",
            "add",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GROUP_REMOVE_MEMBER",
            "제외하",
            "remove",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GROUP_MERGE",
            "합치",
            "combine",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GROUP_COUNT_STATE",
            "가리키",
            "contain",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_REFERENCED_MEMBER",
            "지정한 대상",
            "the referenced member",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_TWO_DISCOURSE_GROUPS",
            "두 담화 묶음",
            "the two discourse groups",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_DISCOURSE_GROUP",
            "그 담화 묶음",
            "that discourse group",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_NEW_DISCOURSE_GROUP",
            "새 담화 묶음",
            "the new discourse group",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_MOVE",
            "이동하",
            "move",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_ASSAULT_VICTIM",
            "폭행 피해자",
            "assault victim",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SAFE_PLACE",
            "안전한 곳",
            "safe place",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_OBSERVE_CURRENT_STATE",
            "확인하",
            "check",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CURRENT_STATE",
            "현재 상태",
            "current state",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_RELEVANT_EVIDENCE",
            "관련 근거",
            "relevant evidence",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SELECTED_ACTION",
            "선택 행동",
            "selected action",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_DIAGNOSTIC_EXECUTION",
            "진단 실행",
            "diagnostic",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_COMPLETION_CONDITIONS",
            "완료 조건",
            "completion conditions",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_KNOWLEDGE_GAP",
            "지식 공백",
            "knowledge gap",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LESSON",
            "교훈",
            "lesson",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_EXPLANATION_SYNTHESIS",
            "설명 합성",
            "explanation synthesis",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_RESULT_DELIVERY",
            "결과 전달",
            "result delivery",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_RESULT_VERIFICATION",
            "결과 검증",
            "result verification",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_REPAIR",
            "수리하",
            "repair",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PERFORM",
            "수행하",
            "perform",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INVESTIGATE",
            "조사하",
            "investigate",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_NARROW",
            "좁히",
            "narrow",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CAUSE",
            "원인",
            "cause",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_CREATE",
            "만들",
            "create",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_LEARN",
            "학습하",
            "learn",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_EXPLAIN",
            "설명하",
            "explain",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN",
            "계획하",
            "plan",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_EXCLUDE_FROM_PLAN",
            "제외하",
            "exclude",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN_SUGGESTION_BOUNDARY",
            "제안으로 해석하",
            "interpret as a suggestion",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN_IMPLICIT_INVESTIGATION",
            "조사 의도로 해석하",
            "interpret as an investigation need",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN_IMPLICIT_REPAIR",
            "수리 필요로 해석하",
            "interpret as a repair need",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN_IMPLICIT_EXPLANATION",
            "설명 의도로 해석하",
            "interpret as an explanation need",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_PLAN_IMPLICIT_PLANNING",
            "계획 요청으로 해석하",
            "interpret as a planning need",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_SARCASM_INTERPRETATION_BOUNDARY",
            "풍자로 해석하",
            "interpret as sarcasm",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_FIGURATIVE_INTERPRETATION_BOUNDARY",
            "비유로 해석하",
            "interpret figuratively",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_VERIFY_RESULT",
            "검증하",
            "verify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_RESULT",
            "결과",
            "result",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_COPULA",
            "이",
            "be",
            Verb,
            KoreanCopula,
            EnglishCopula,
            Neutral,
        ),
        (
            "C_CURRENT_WORK",
            "이 작업",
            "this work",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_PLANNED_STATE",
            "아직 계획 상태",
            "still planned",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_EXECUTED_OCCURRENCE",
            "아직 실행한 것",
            "executed",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_NEGATION",
            "아니",
            "not",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_REPORTED_SAY",
            "말했",
            "said",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_DIALOGUE_USER",
            "너",
            "you",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SPOKEN_CONTENT",
            "말한 내용",
            "statement",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_REMEMBER",
            "기억하",
            "remember",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CONFIRMED_FACT",
            "확인된 사실",
            "confirmed fact",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SEPARATE_EVIDENCE",
            "별도 증거",
            "separate evidence",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_REQUIRE",
            "필요로 하",
            "requires",
            Verb,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_REPEATED_SITUATION",
            "계속 반복되는 일",
            "that",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_FRUSTRATING",
            "답답할 만하",
            "frustrating",
            Adjective,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ANGERING",
            "화날 만하",
            "infuriating",
            Adjective,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_WORRYING",
            "걱정할 만하",
            "worrying",
            Adjective,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_HURTFUL",
            "속상할 만하",
            "hurtful",
            Adjective,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ANNOYING",
            "짜증날 만하",
            "annoying",
            Adjective,
            KoreanHada,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_INVITE_CHECK",
            "확인하",
            "check",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_RECENT_FAILURE",
            "가장 최근 실패",
            "most recent failure",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_RESOLVE_REFERENCE",
            "가리키",
            "refer",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_PENDING_CHOICE",
            "선택하",
            "select",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_ORDERED_PAIR",
            "지정하",
            "name",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_LOCAL_ORDINAL",
            "확인하",
            "confirm",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_EVENT_ORDINAL",
            "확인하",
            "confirm",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_PREVIOUS_TOPIC",
            "말하",
            "name",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_COMPETING_REQUEST",
            "지정하",
            "identify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_NONLITERAL_READING",
            "구분하",
            "clarify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_VOICE_ALTERNATIVE",
            "확인하",
            "confirm",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CLARIFY_MISSING_DETAILS",
            "구체화하",
            "clarify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_CHANGE_TARGET",
            "대상",
            "target",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SINGLE",
            "하나",
            "one",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_NAME_TARGET",
            "지정하",
            "name",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_RECORD",
            "기록하",
            "record",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_MODAL",
            "분류하",
            "classify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_NOT_FACT",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_CONFLICT",
            "충돌하",
            "conflict",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_NO_CONFLICT",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_PRESUPPOSITION",
            "검증하",
            "verify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_NO_MATCH",
            "찾",
            "find",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_ANSWER_AMBIGUOUS",
            "지정하",
            "specify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_SELF_COMMITMENT",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_REPORTED_COMMITMENT",
            "기록하",
            "record",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_CAPABILITY_QUESTION",
            "분류하",
            "classify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_DEFERRED_REQUEST",
            "보류하",
            "defer",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_GOAL_WITHDRAWAL",
            "철회하",
            "withdraw",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_WITHDRAWAL_NO_MATCH",
            "보존하",
            "preserve",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_OUTCOME_POLICY",
            "제한하",
            "constrain",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_INTERACTION_NO_AUTHORITY",
            "경계하",
            "bound",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_UNRESOLVED",
            "보류하",
            "defer",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_SUPPORTED",
            "지지하",
            "support",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_CONTRADICTED",
            "반박하",
            "contradict",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_CONTESTED",
            "충돌하",
            "conflict",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_COUNTERFACTUAL",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_GUARD_NO_REVERSE_INFERENCE",
            "제한하",
            "bound",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_BIND_ADDED",
            "연결하",
            "link",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_BIND_CONFIRMED",
            "확인하",
            "confirm",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_PAYLOAD_BOUNDARY",
            "보존하",
            "preserve",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_REJECT_CONFLICT",
            "거부하",
            "reject",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_REJECT_NONASSERTED",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_REJECT_AMBIGUOUS",
            "보류하",
            "defer",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_REJECT_UNRESOLVED",
            "보류하",
            "defer",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEFINITION_REJECT_INVALID_ALIAS",
            "거부하",
            "reject",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_CAUSE_EDGE",
            "연결하",
            "link",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_RESULT_EDGE",
            "이어지",
            "lead",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_CONCESSION_EDGE",
            "성립하",
            "hold",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_RESULT_BOUNDARY",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY",
            "보존하",
            "preserve",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY",
            "도출하",
            "derive",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_NO_MATCH",
            "찾",
            "find",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_NONACTUAL_WARNING",
            "제한하",
            "bound",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_CONTESTED_WARNING",
            "보존하",
            "preserve",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DIALOGUE_RELATION_TRUNCATED_WARNING",
            "제한하",
            "limit",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_TIME",
            "기록하",
            "record",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_EVENT",
            "남",
            "remain",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_BEFORE",
            "앞서",
            "precede",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_DURING",
            "이어지",
            "overlap",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_SIMULTANEOUS",
            "동시에 일어나",
            "coincide",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY",
            "구분하",
            "distinguish",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY",
            "도출하",
            "derive",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_NO_MATCH",
            "찾",
            "find",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_NO_RELATION",
            "기록하",
            "record",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_AMBIGUOUS",
            "지정하",
            "specify",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_CONFLICT",
            "충돌하",
            "conflict",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TEMPORAL_ANSWER_TIME_MISSING",
            "누락하",
            "omit",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_ACTIVATE_TOPIC",
            "현재 화제로 두",
            "activate as topic",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTIVATE_TOPIC_GROUP",
            "묶음을 현재 화제로 두",
            "activate group as topic",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_RETURN_TOPIC",
            "돌아가",
            "return",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TOPIC_CHANGE_BOUNDARY",
            "대화 초점만 바꾸",
            "change only the conversation focus",
            Verb,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_TOPIC_RETURN_STYLE",
            "이야기로 돌아가",
            "return to the topic",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_CURRENT_CHANGE",
            "지금 바꾼 것",
            "this change",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_TOPIC_ONLY",
            "화제뿐",
            "topic-only",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_ACTIVE_PLAN",
            "아직 계획만 있는 상태",
            "still only a plan",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_SUPERSEDED_PLAN",
            "대체된 계획으로 남은 상태",
            "a superseded plan",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_WITHDRAWN_PLAN",
            "철회된 계획",
            "a withdrawn plan",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_REPORTED_ATTEMPT",
            "시도했다는 사용자 보고가 있는 상태",
            "reported as attempted",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_REPORTED_IN_PROGRESS",
            "진행 중이라는 사용자 보고가 있는 상태",
            "reported as in progress",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_REPORTED_SUCCESS",
            "끝났다는 사용자 보고가 있는 상태",
            "reported as complete",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_REPORTED_FAILURE",
            "실패했다는 사용자 보고가 있는 상태",
            "reported as failed",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_NO_USER_REPORT",
            "사용자 결과 보고가 없는 상태",
            "without a user-reported result",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_NO_EXECUTION_OR_RESULT",
            "검증된 실행 기록이나 실행 결과가 없는 상태",
            "without a verified execution record or result",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_EXECUTION_IN_PROGRESS",
            "검증 기록상 실행 중인 상태",
            "running according to a verified receipt",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_FINAL_RESULT_UNAVAILABLE",
            "최종 성공이나 실패 결과가 아직 없는 상태",
            "without a final success or failure result yet",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_VERIFIED_SUCCESS",
            "검증된 실행 결과가 성공인 상태",
            "verified as successfully completed",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_VERIFIED_FAILURE",
            "검증된 실행 결과가 실패인 상태",
            "verified as failed",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_RESULT_UNAVAILABLE",
            "검증된 실행 결과가 아직 없는 상태",
            "without a verified execution result yet",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_UNTRUSTED_EVIDENCE_MENTION",
            "호스트 검증 영수증이 아닌 상태",
            "not a host-verified execution receipt",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_EXECUTION_STATE_UNCHANGED",
            "실행 상태가 승격되지 않은 상태",
            "unchanged in verified execution state",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_CONFLICTING_REPORTS",
            "서로 충돌하는 상태",
            "in conflict",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LIFECYCLE_REPORTS_NOT_VERIFIED",
            "보고일 뿐 검증된 실행 결과는 아닌 상태",
            "reports only, not verified execution results",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ASSESS_ACTION_SET",
            "판단하",
            "assess",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_ACTION_SET",
            "작업",
            "selected actions",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_ALL",
            "모두",
            "all",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_ANY",
            "적어도 하나",
            "at least one of",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_NONE",
            "어느 것도",
            "none of",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_TRUE",
            "맞아",
            "Yes",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_ACTION_SET_FALSE",
            "아니야",
            "No",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_ACTION_SET_UNKNOWN",
            "현재 기록만으로 판단할 수 없어",
            "The current records do not determine that",
            Interjection,
            KoreanInvariable,
            EnglishInvariable,
            Informal,
        ),
        (
            "C_ACTION_SET_ACTIVE_PLAN",
            "활성 계획이야",
            "are active plans",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_REPORTED_COMPLETION",
            "완료됐다는 사용자 보고가 있어",
            "have a user-reported completion",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_REPORTED_FAILURE",
            "실패했다는 사용자 보고가 있어",
            "have a user-reported failure",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_UNVERIFIED_EXECUTION",
            "검증된 실행 관찰이 없어",
            "have no verified execution observation",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_VERIFIED_EXECUTION",
            "검증된 실행 관찰이 있어",
            "have a verified execution observation",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_VERIFIED_SUCCESS",
            "검증된 성공 결과가 있어",
            "have a verified successful result",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_VERIFIED_FAILURE",
            "검증된 실패 결과가 있어",
            "have a verified failed result",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_ACTION_SET_VERIFIED_IN_PROGRESS",
            "검증된 실행 중 상태야",
            "have a verified in-progress state",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_LANGUAGE_REPORT",
            "사용자 언어 보고",
            "a user language report",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_SEPARATE_FROM_VERIFIED_RESULT",
            "호스트 검증 결과와 분리된 상태",
            "separate from host-verified execution results",
            Adjective,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
    ];
    concepts
        .into_iter()
        .flat_map(|(concept, ko, en, pos, ko_morph, en_morph, register)| {
            let suffix = concept.trim_start_matches("C_");
            [
                expression(
                    &format!("EXPR.KO.{suffix}"),
                    Korean,
                    concept,
                    ko,
                    pos,
                    ko_morph,
                    register,
                ),
                expression(
                    &format!("EXPR.EN.{suffix}"),
                    English,
                    concept,
                    en,
                    pos,
                    en_morph,
                    register,
                ),
            ]
        })
        .collect()
}

pub(crate) fn generate_plan_preview_from_knowledge(
    language: LanguageCodeIR,
    subject: &str,
    intent: PlanIntentIR,
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    generate_plan_preview_from_knowledge_with_directive(
        language,
        subject,
        intent,
        grounding_ref,
        None,
        false,
    )
}

pub(crate) fn generate_plan_preview_from_knowledge_with_directive(
    language: LanguageCodeIR,
    subject: &str,
    intent: PlanIntentIR,
    grounding_ref: &str,
    directive_ref: Option<&str>,
    concise: bool,
) -> Result<GenerativeLanguageIR, String> {
    if subject.trim().is_empty() || grounding_ref.trim().is_empty() {
        return Err("INVALID_PLAN_PREVIEW_REQUEST".to_string());
    }
    let action_concept = match intent {
        PlanIntentIR::Repair => "C_REPAIR",
        PlanIntentIR::Execute => "C_PERFORM",
        PlanIntentIR::Investigate => "C_NARROW",
        PlanIntentIR::Create => "C_CREATE",
        PlanIntentIR::Learn => "C_LEARN",
        PlanIntentIR::Explain | PlanIntentIR::Communicate => "C_EXPLAIN",
        PlanIntentIR::Plan => "C_PLAN",
    };
    let node = |node_id: &str, concept_id: &str, kind| {
        let mut grounding_refs = vec![format!("PLAN_INTENT:{intent:?}"), grounding_ref.to_string()];
        if let Some(directive_ref) = directive_ref {
            grounding_refs.push(directive_ref.to_string());
        }
        GenerationMeaningNodeIR {
            node_id: node_id.to_string(),
            concept_id: concept_id.to_string(),
            kind,
            grounding_refs,
        }
    };
    let mut nodes = vec![
        node("E_ACK", "C_ACKNOWLEDGE", GenerationMeaningNodeKindIR::Event),
        node(
            "E_OBSERVE",
            "C_OBSERVE_CURRENT_STATE",
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "E_ACTION",
            action_concept,
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "E_VERIFY",
            "C_VERIFY_RESULT",
            GenerationMeaningNodeKindIR::Event,
        ),
        node("E_BOUNDARY", "C_COPULA", GenerationMeaningNodeKindIR::Event),
        node("E_EXECUTED", "C_COPULA", GenerationMeaningNodeKindIR::Event),
        node(
            "R_SUBJECT",
            "C_CURRENT_PLAN_SUBJECT",
            GenerationMeaningNodeKindIR::Entity,
        ),
        node(
            "R_CURRENT_STATE",
            match intent {
                PlanIntentIR::Create | PlanIntentIR::Plan => "C_COMPLETION_CONDITIONS",
                PlanIntentIR::Learn => "C_KNOWLEDGE_GAP",
                PlanIntentIR::Explain | PlanIntentIR::Communicate => "C_RELEVANT_EVIDENCE",
                _ => "C_CURRENT_STATE",
            },
            GenerationMeaningNodeKindIR::State,
        ),
        node("R_RESULT", "C_RESULT", GenerationMeaningNodeKindIR::Entity),
        node(
            "R_WORK",
            "C_CURRENT_WORK",
            GenerationMeaningNodeKindIR::Entity,
        ),
        node(
            "Q_PLANNED",
            "C_PLANNED_STATE",
            GenerationMeaningNodeKindIR::Quality,
        ),
        node(
            "Q_EXECUTED",
            "C_EXECUTED_OCCURRENCE",
            GenerationMeaningNodeKindIR::Quality,
        ),
        node("N_NOT", "C_NEGATION", GenerationMeaningNodeKindIR::State),
    ];
    if intent == PlanIntentIR::Investigate {
        nodes.push(node(
            "R_CAUSE",
            "C_CAUSE",
            GenerationMeaningNodeKindIR::Entity,
        ));
    }
    // Keep scheduler concepts inspectable without surfacing their internal labels.
    // The action event and its subject already express the selected action, while
    // the observation object above carries the intent-specific preparation.
    let detail_concepts: &[&str] = &[];
    for (index, concept_id) in detail_concepts.iter().enumerate() {
        nodes.push(node(
            &format!("E_DETAIL_{index:02}"),
            "C_PERFORM",
            GenerationMeaningNodeKindIR::Event,
        ));
        nodes.push(node(
            &format!("R_DETAIL_{index:02}"),
            concept_id,
            GenerationMeaningNodeKindIR::Entity,
        ));
    }
    let mut edges = vec![
        meaning_edge(
            "M1",
            "E_ACK",
            "E_OBSERVE",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "M3",
            "E_ACTION",
            "E_VERIFY",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "M4",
            "E_VERIFY",
            "E_BOUNDARY",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "M5",
            "E_BOUNDARY",
            "E_EXECUTED",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "M6",
            "E_OBSERVE",
            "R_CURRENT_STATE",
            GenerationMeaningRelationIR::Theme,
        ),
        meaning_edge(
            "M7",
            "E_VERIFY",
            "R_RESULT",
            GenerationMeaningRelationIR::Theme,
        ),
        meaning_edge(
            "M8",
            "E_BOUNDARY",
            "R_WORK",
            GenerationMeaningRelationIR::Agent,
        ),
        meaning_edge(
            "M9",
            "E_BOUNDARY",
            "Q_PLANNED",
            GenerationMeaningRelationIR::Property,
        ),
        meaning_edge(
            "M10",
            "R_CURRENT_STATE",
            "R_SUBJECT",
            GenerationMeaningRelationIR::Possessor,
        ),
        meaning_edge(
            "M11",
            "E_EXECUTED",
            "R_WORK",
            GenerationMeaningRelationIR::Agent,
        ),
        meaning_edge(
            "M12",
            "E_EXECUTED",
            "Q_EXECUTED",
            GenerationMeaningRelationIR::Property,
        ),
        meaning_edge(
            "M13",
            "E_EXECUTED",
            "N_NOT",
            GenerationMeaningRelationIR::Negates,
        ),
    ];
    let mut previous_event = "E_OBSERVE".to_string();
    for index in 0..detail_concepts.len() {
        let event_id = format!("E_DETAIL_{index:02}");
        let detail_id = format!("R_DETAIL_{index:02}");
        edges.push(meaning_edge(
            &format!("MD{index:02}.SEQUENCE"),
            &previous_event,
            &event_id,
            GenerationMeaningRelationIR::Sequence,
        ));
        edges.push(meaning_edge(
            &format!("MD{index:02}.THEME"),
            &event_id,
            &detail_id,
            GenerationMeaningRelationIR::Theme,
        ));
        previous_event = event_id;
    }
    edges.push(meaning_edge(
        "M2",
        &previous_event,
        "E_ACTION",
        GenerationMeaningRelationIR::Sequence,
    ));
    if intent == PlanIntentIR::Investigate {
        edges.push(meaning_edge(
            "M14",
            "E_ACTION",
            "R_CAUSE",
            GenerationMeaningRelationIR::Theme,
        ));
        edges.push(meaning_edge(
            "M15",
            "R_CAUSE",
            "R_SUBJECT",
            GenerationMeaningRelationIR::Possessor,
        ));
    } else {
        edges.push(meaning_edge(
            "M14",
            "E_ACTION",
            "R_SUBJECT",
            GenerationMeaningRelationIR::Theme,
        ));
    }
    if concise {
        let retained = [
            "E_ACTION",
            "E_BOUNDARY",
            "E_EXECUTED",
            "R_SUBJECT",
            "R_WORK",
            "Q_PLANNED",
            "Q_EXECUTED",
            "N_NOT",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        nodes.retain(|node| retained.contains(node.node_id.as_str()));
        edges.retain(|edge| {
            retained.contains(edge.source_node_id.as_str())
                && retained.contains(edge.target_node_id.as_str())
        });
        edges.push(meaning_edge(
            "M.CONCISE.ACTION_BOUNDARY",
            "E_ACTION",
            "E_BOUNDARY",
            GenerationMeaningRelationIR::Sequence,
        ));
        if !edges.iter().any(|edge| {
            edge.source_node_id == "E_ACTION"
                && edge.target_node_id == "R_SUBJECT"
                && edge.relation == GenerationMeaningRelationIR::Theme
        }) {
            edges.push(meaning_edge(
                "M.CONCISE.ACTION_THEME",
                "E_ACTION",
                "R_SUBJECT",
                GenerationMeaningRelationIR::Theme,
            ));
        }
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_PLAN_SUBJECT",
            _ => "EXPR.EN.RUNTIME_PLAN_SUBJECT",
        },
        language,
        "C_CURRENT_PLAN_SUBJECT",
        subject,
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:CURRENT_PLAN_SUBJECT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Future,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::CommitFutureAction,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_plan_exclusion_from_knowledge(
    language: LanguageCodeIR,
    subject: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if subject.trim().is_empty() || grounding_refs.is_empty() {
        return Err("INVALID_PLAN_EXCLUSION_REQUEST".to_string());
    }
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.to_vec(),
    };
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node(
                "E_EXCLUDE",
                "C_EXCLUDE_FROM_PLAN",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "R_PROHIBITED_REQUEST",
                "C_RUNTIME_PROHIBITED_PLAN_SUBJECT",
                GenerationMeaningNodeKindIR::Entity,
            ),
        ],
        vec![meaning_edge(
            "PLAN_EXCLUSION.THEME",
            "E_EXCLUDE",
            "R_PROHIBITED_REQUEST",
            GenerationMeaningRelationIR::Theme,
        )],
    );
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_PROHIBITED_PLAN_SUBJECT",
            _ => "EXPR.EN.RUNTIME_PROHIBITED_PLAN_SUBJECT",
        },
        language,
        "C_RUNTIME_PROHIBITED_PLAN_SUBJECT",
        subject.trim(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:PROHIBITED_PLAN_SUBJECT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_plan_interpretation_from_knowledge(
    language: LanguageCodeIR,
    kind: GenerationPlanInterpretationKindIR,
    primary_surface: &str,
    secondary_surface: Option<&str>,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if primary_surface.trim().is_empty() || grounding_refs.is_empty() {
        return Err("INVALID_PLAN_INTERPRETATION_REQUEST".to_string());
    }
    if (kind == GenerationPlanInterpretationKindIR::FigurativeBoundary)
        != secondary_surface.is_some_and(|surface| !surface.trim().is_empty())
    {
        return Err("INVALID_PLAN_INTERPRETATION_SHAPE".to_string());
    }
    let concept_id = match kind {
        GenerationPlanInterpretationKindIR::Suggestion => "C_PLAN_SUGGESTION_BOUNDARY",
        GenerationPlanInterpretationKindIR::ImplicitInvestigation => {
            "C_PLAN_IMPLICIT_INVESTIGATION"
        }
        GenerationPlanInterpretationKindIR::ImplicitRepair => "C_PLAN_IMPLICIT_REPAIR",
        GenerationPlanInterpretationKindIR::ImplicitExplanation => "C_PLAN_IMPLICIT_EXPLANATION",
        GenerationPlanInterpretationKindIR::ImplicitPlanning => "C_PLAN_IMPLICIT_PLANNING",
        GenerationPlanInterpretationKindIR::SarcasmBoundary => "C_SARCASM_INTERPRETATION_BOUNDARY",
        GenerationPlanInterpretationKindIR::FigurativeBoundary => {
            "C_FIGURATIVE_INTERPRETATION_BOUNDARY"
        }
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.to_vec(),
    };
    let mut nodes = vec![
        node(
            "E_INTERPRET",
            concept_id,
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "R_INTERPRETATION_SUBJECT",
            "C_RUNTIME_INTERPRETATION_SUBJECT",
            GenerationMeaningNodeKindIR::Entity,
        ),
    ];
    let mut edges = vec![meaning_edge(
        "INTERPRETATION.THEME",
        "E_INTERPRET",
        "R_INTERPRETATION_SUBJECT",
        GenerationMeaningRelationIR::Theme,
    )];
    if secondary_surface.is_some() {
        nodes.push(node(
            "R_INTERPRETATION_TARGET",
            "C_RUNTIME_INTERPRETATION_TARGET",
            GenerationMeaningNodeKindIR::State,
        ));
        edges.push(meaning_edge(
            "INTERPRETATION.GOAL",
            "E_INTERPRET",
            "R_INTERPRETATION_TARGET",
            GenerationMeaningRelationIR::Goal,
        ));
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_INTERPRETATION_SUBJECT",
            _ => "EXPR.EN.RUNTIME_INTERPRETATION_SUBJECT",
        },
        language,
        "C_RUNTIME_INTERPRETATION_SUBJECT",
        primary_surface.trim(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:INTERPRETATION_SUBJECT",
    )?;
    if let Some(secondary_surface) = secondary_surface {
        expressions.attach_alias(
            match language {
                LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_INTERPRETATION_TARGET",
                _ => "EXPR.EN.RUNTIME_INTERPRETATION_TARGET",
            },
            language,
            "C_RUNTIME_INTERPRETATION_TARGET",
            secondary_surface.trim(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:INTERPRETATION_TARGET",
        )?;
    }
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_lifecycle_status_from_knowledge(
    language: LanguageCodeIR,
    subject: &str,
    claims: &[GenerationLifecycleClaimIR],
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    if subject.trim().is_empty() || claims.is_empty() || grounding_ref.trim().is_empty() {
        return Err("INVALID_LIFECYCLE_GENERATION_REQUEST".to_string());
    }
    let mut nodes = vec![GenerationMeaningNodeIR {
        node_id: "R_ACTION".to_string(),
        concept_id: "C_RUNTIME_LIFECYCLE_SUBJECT".to_string(),
        kind: GenerationMeaningNodeKindIR::Entity,
        grounding_refs: vec![grounding_ref.to_string()],
    }];
    let mut edges = Vec::new();
    for (index, claim) in claims.iter().copied().enumerate() {
        let event_id = format!("E_STATUS_{index:02}");
        let quality_id = format!("Q_STATUS_{index:02}");
        nodes.push(GenerationMeaningNodeIR {
            node_id: event_id.clone(),
            concept_id: "C_COPULA".to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: vec![
                grounding_ref.to_string(),
                format!("LIFECYCLE_CLAIM:{claim:?}"),
            ],
        });
        nodes.push(GenerationMeaningNodeIR {
            node_id: quality_id.clone(),
            concept_id: claim.concept_id().to_string(),
            kind: GenerationMeaningNodeKindIR::Quality,
            grounding_refs: vec![
                grounding_ref.to_string(),
                format!("LIFECYCLE_CLAIM:{claim:?}"),
            ],
        });
        edges.push(meaning_edge(
            &format!("LC{index:02}.AGENT"),
            &event_id,
            "R_ACTION",
            GenerationMeaningRelationIR::Agent,
        ));
        edges.push(meaning_edge(
            &format!("LC{index:02}.PROPERTY"),
            &event_id,
            &quality_id,
            GenerationMeaningRelationIR::Property,
        ));
        if index > 0 {
            edges.push(meaning_edge(
                &format!("LC{index:02}.SEQUENCE"),
                &format!("E_STATUS_{:02}", index - 1),
                &event_id,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_LIFECYCLE_SUBJECT",
            _ => "EXPR.EN.RUNTIME_LIFECYCLE_SUBJECT",
        },
        language,
        "C_RUNTIME_LIFECYCLE_SUBJECT",
        subject.trim(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:LIFECYCLE_SUBJECT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_action_set_answer_from_knowledge(
    language: LanguageCodeIR,
    selected_count: usize,
    quantifier: GenerationActionSetQuantifierIR,
    predicate: GenerationActionSetPredicateIR,
    truth: GenerationActionSetTruthIR,
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    if selected_count > 32 || grounding_ref.trim().is_empty() {
        return Err("INVALID_ACTION_SET_GENERATION_REQUEST".to_string());
    }
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: vec![grounding_ref.to_string()],
    };
    let cardinality_concept = format!("C_CARDINALITY_{selected_count}");
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node(
                "E_ASSESS",
                "C_ASSESS_ACTION_SET",
                GenerationMeaningNodeKindIR::Event,
            ),
            node("E_BOUNDARY", "C_COPULA", GenerationMeaningNodeKindIR::Event),
            node("R_SET", "C_ACTION_SET", GenerationMeaningNodeKindIR::Entity),
            node(
                "R_COUNT",
                &cardinality_concept,
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "Q_QUANTIFIER",
                quantifier.concept_id(),
                GenerationMeaningNodeKindIR::Quality,
            ),
            node(
                "Q_PREDICATE",
                predicate.concept_id(),
                GenerationMeaningNodeKindIR::Quality,
            ),
            node(
                "Q_TRUTH",
                truth.concept_id(),
                GenerationMeaningNodeKindIR::Quality,
            ),
            node(
                "R_LANGUAGE_REPORT",
                "C_LANGUAGE_REPORT",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "Q_SEPARATE",
                "C_SEPARATE_FROM_VERIFIED_RESULT",
                GenerationMeaningNodeKindIR::Quality,
            ),
        ],
        vec![
            meaning_edge(
                "ASQ1",
                "E_ASSESS",
                "E_BOUNDARY",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "ASQ2",
                "E_ASSESS",
                "Q_TRUTH",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "ASQ3",
                "E_ASSESS",
                "R_SET",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "ASQ4",
                "R_SET",
                "R_COUNT",
                GenerationMeaningRelationIR::Possessor,
            ),
            meaning_edge(
                "ASQ5",
                "E_ASSESS",
                "Q_QUANTIFIER",
                GenerationMeaningRelationIR::Goal,
            ),
            meaning_edge(
                "ASQ6",
                "E_ASSESS",
                "Q_PREDICATE",
                GenerationMeaningRelationIR::Property,
            ),
            meaning_edge(
                "ASQ7",
                "E_BOUNDARY",
                "R_LANGUAGE_REPORT",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "ASQ8",
                "E_BOUNDARY",
                "Q_SEPARATE",
                GenerationMeaningRelationIR::Property,
            ),
        ],
    );
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let count_surface = match language {
        LanguageCodeIR::Korean => format!("{selected_count}개"),
        _ => selected_count.to_string(),
    };
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_ACTION_COUNT",
            _ => "EXPR.EN.RUNTIME_ACTION_COUNT",
        },
        language,
        &cardinality_concept,
        &count_surface,
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:ACTION_SET_CARDINALITY",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_inform_acknowledgement_from_knowledge(
    language: LanguageCodeIR,
    reported_surface: &str,
) -> Result<GenerativeLanguageIR, String> {
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: vec!["LANGUAGE_REPORT:CURRENT_TURN".to_string()],
    };
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node("E_ACK", "C_ACKNOWLEDGE", GenerationMeaningNodeKindIR::Event),
            node(
                "E_REPORT",
                "C_REPORTED_SAY",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_REMEMBER",
                "C_REMEMBER",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_FACT_BOUNDARY",
                "C_COPULA",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_EVIDENCE_REQUIREMENT",
                "C_REQUIRE",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "R_USER",
                "C_DIALOGUE_USER",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "R_REPORTED_PROPOSITION",
                "C_REPORTED_PROPOSITION",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "R_SPOKEN_CONTENT",
                "C_SPOKEN_CONTENT",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "R_CONFIRMED_FACT",
                "C_CONFIRMED_FACT",
                GenerationMeaningNodeKindIR::State,
            ),
            node(
                "R_SEPARATE_EVIDENCE",
                "C_SEPARATE_EVIDENCE",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node("N_NOT", "C_NEGATION", GenerationMeaningNodeKindIR::State),
        ],
        vec![
            meaning_edge(
                "IA1",
                "E_ACK",
                "E_REPORT",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "IA2",
                "E_REPORT",
                "E_REMEMBER",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "IA3",
                "E_REMEMBER",
                "E_FACT_BOUNDARY",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "IA4",
                "E_FACT_BOUNDARY",
                "E_EVIDENCE_REQUIREMENT",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "IA5",
                "E_REPORT",
                "R_USER",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "IA6",
                "E_REPORT",
                "R_REPORTED_PROPOSITION",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "IA7",
                "E_REMEMBER",
                "R_SPOKEN_CONTENT",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "IA8",
                "E_FACT_BOUNDARY",
                "R_SPOKEN_CONTENT",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "IA9",
                "E_FACT_BOUNDARY",
                "R_CONFIRMED_FACT",
                GenerationMeaningRelationIR::Property,
            ),
            meaning_edge(
                "IA10",
                "E_FACT_BOUNDARY",
                "N_NOT",
                GenerationMeaningRelationIR::Negates,
            ),
            meaning_edge(
                "IA11",
                "E_EVIDENCE_REQUIREMENT",
                "R_CONFIRMED_FACT",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "IA12",
                "E_EVIDENCE_REQUIREMENT",
                "R_SEPARATE_EVIDENCE",
                GenerationMeaningRelationIR::Theme,
            ),
        ],
    );
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_REPORTED_PROPOSITION",
            _ => "EXPR.EN.RUNTIME_REPORTED_PROPOSITION",
        },
        language,
        "C_REPORTED_PROPOSITION",
        &format!("“{}”", reported_surface.trim()),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:REPORTED_PROPOSITION",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language: if language == LanguageCodeIR::Korean {
                LanguageCodeIR::Korean
            } else {
                LanguageCodeIR::English
            },
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_affect_support_from_knowledge(
    language: LanguageCodeIR,
    affect: GenerationAffectKindIR,
) -> Result<GenerativeLanguageIR, String> {
    let quality_concept = match affect {
        GenerationAffectKindIR::Frustrated => "C_FRUSTRATING",
        GenerationAffectKindIR::Angry => "C_ANGERING",
        GenerationAffectKindIR::Worried => "C_WORRYING",
        GenerationAffectKindIR::Hurt => "C_HURTFUL",
        GenerationAffectKindIR::Annoyed => "C_ANNOYING",
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: vec![format!("USER_AFFECT:{affect:?}")],
    };
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node("E_EMPATHY", "C_COPULA", GenerationMeaningNodeKindIR::Event),
            node(
                "E_INVITE_CHECK",
                "C_INVITE_CHECK",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "R_SITUATION",
                "C_REPEATED_SITUATION",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "Q_AFFECT",
                quality_concept,
                GenerationMeaningNodeKindIR::Quality,
            ),
            node(
                "R_RECENT_FAILURE",
                "C_RECENT_FAILURE",
                GenerationMeaningNodeKindIR::Entity,
            ),
        ],
        vec![
            meaning_edge(
                "AS1",
                "E_EMPATHY",
                "E_INVITE_CHECK",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "AS2",
                "E_EMPATHY",
                "R_SITUATION",
                GenerationMeaningRelationIR::Agent,
            ),
            meaning_edge(
                "AS3",
                "E_EMPATHY",
                "Q_AFFECT",
                GenerationMeaningRelationIR::Property,
            ),
            meaning_edge(
                "AS4",
                "E_INVITE_CHECK",
                "R_RECENT_FAILURE",
                GenerationMeaningRelationIR::Theme,
            ),
        ],
    );
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language: if language == LanguageCodeIR::Korean {
                LanguageCodeIR::Korean
            } else {
                LanguageCodeIR::English
            },
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Warm,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &ExpressionNodeStore::bilingual_builtin(),
    })
}

pub(crate) fn generate_dialogue_response_from_knowledge(
    language: LanguageCodeIR,
    response: GenerationDialogueResponseKindIR,
) -> Result<GenerativeLanguageIR, String> {
    let grounding = format!("DISCOURSE_EVENT:{response:?}");
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: vec![grounding.clone()],
    };
    let (nodes, edges) = match response {
        GenerationDialogueResponseKindIR::HoldFloor => (
            vec![
                node(
                    "E_ACK",
                    "C_DIALOGUE_HOLD_ACK",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_CONTINUE",
                    "C_DIALOGUE_CONTINUE",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_LISTEN",
                    "C_DIALOGUE_LISTEN",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "Q_PACE",
                    "C_UNHURRIED_PACE",
                    GenerationMeaningNodeKindIR::Quality,
                ),
                node("R_TURN", "C_USER_TURN", GenerationMeaningNodeKindIR::Entity),
            ],
            vec![
                meaning_edge(
                    "DR1",
                    "E_ACK",
                    "E_CONTINUE",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR2",
                    "E_CONTINUE",
                    "E_LISTEN",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR3",
                    "E_CONTINUE",
                    "Q_PACE",
                    GenerationMeaningRelationIR::Property,
                ),
                meaning_edge(
                    "DR4",
                    "E_LISTEN",
                    "R_TURN",
                    GenerationMeaningRelationIR::Theme,
                ),
            ],
        ),
        GenerationDialogueResponseKindIR::Greeting => (
            vec![
                node(
                    "E_SOCIAL_REPLY",
                    "C_DIALOGUE_GREETING_REPLY",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_FOLLOW_UP",
                    "C_DIALOGUE_OFFER_HELP",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_OPEN_NEED",
                    "C_OPEN_NEED",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![
                meaning_edge(
                    "DR1",
                    "E_SOCIAL_REPLY",
                    "E_FOLLOW_UP",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR2",
                    "E_FOLLOW_UP",
                    "R_OPEN_NEED",
                    GenerationMeaningRelationIR::Theme,
                ),
            ],
        ),
        GenerationDialogueResponseKindIR::Gratitude => (
            vec![
                node(
                    "E_SOCIAL_REPLY",
                    "C_DIALOGUE_GRATITUDE_REPLY",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_FOLLOW_UP",
                    "C_DIALOGUE_INVITE_NEED",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_ADDITIONAL_NEED",
                    "C_ADDITIONAL_NEED",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![
                meaning_edge(
                    "DR1",
                    "E_SOCIAL_REPLY",
                    "E_FOLLOW_UP",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR2",
                    "E_FOLLOW_UP",
                    "R_ADDITIONAL_NEED",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        ),
        GenerationDialogueResponseKindIR::Farewell => (
            vec![
                node(
                    "E_SOCIAL_REPLY",
                    "C_DIALOGUE_FAREWELL_REPLY",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_FOLLOW_UP",
                    "C_DIALOGUE_INVITE_NEED",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_FUTURE_NEED",
                    "C_FUTURE_NEED",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![
                meaning_edge(
                    "DR1",
                    "E_SOCIAL_REPLY",
                    "E_FOLLOW_UP",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR2",
                    "E_FOLLOW_UP",
                    "R_FUTURE_NEED",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        ),
        GenerationDialogueResponseKindIR::Backchannel => (
            vec![
                node("E_ACK", "C_ACKNOWLEDGE", GenerationMeaningNodeKindIR::Event),
                node(
                    "E_CONTINUE",
                    "C_DIALOGUE_CONTINUE",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_READY",
                    "C_WHEN_READY",
                    GenerationMeaningNodeKindIR::State,
                ),
            ],
            vec![
                meaning_edge(
                    "DR1",
                    "E_ACK",
                    "E_CONTINUE",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "DR2",
                    "E_CONTINUE",
                    "R_READY",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        ),
    };
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language: if language == LanguageCodeIR::Korean {
                LanguageCodeIR::Korean
            } else {
                LanguageCodeIR::English
            },
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Warm,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Acknowledge,
        },
        expressions: &ExpressionNodeStore::bilingual_builtin(),
    })
}

pub(crate) fn generate_continuation_gate_from_knowledge(
    language: LanguageCodeIR,
    task_surface: &str,
    benefit_surface: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if task_surface.trim().is_empty() || benefit_surface.trim().is_empty() {
        return Err("CONTINUATION_GATE_REQUIRES_TASK_AND_BENEFIT".to_string());
    }
    let grounding_refs = if grounding_refs.is_empty() {
        vec!["CONTINUATION_GATE:TYPED".to_string()]
    } else {
        grounding_refs.to_vec()
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node(
                "E_INTERPRET",
                "C_GATE_INTERPRET",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_VERIFY",
                "C_GATE_VERIFY",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_POSITIVE",
                "C_GATE_CONTINUE",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_NEGATIVE",
                "C_GATE_REPORT_ASK_STOP",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_UNKNOWN",
                "C_GATE_ASK_UNRESOLVED",
                GenerationMeaningNodeKindIR::Event,
            ),
            node("R_TASK", "C_GATE_TASK", GenerationMeaningNodeKindIR::Entity),
            node(
                "R_BENEFIT",
                "C_GATE_BENEFIT",
                GenerationMeaningNodeKindIR::Entity,
            ),
        ],
        vec![
            meaning_edge(
                "CG1",
                "E_INTERPRET",
                "E_VERIFY",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "CG2",
                "E_VERIFY",
                "E_POSITIVE",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "CG3",
                "E_POSITIVE",
                "E_NEGATIVE",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "CG4",
                "E_NEGATIVE",
                "E_UNKNOWN",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "CG5",
                "E_INTERPRET",
                "R_TASK",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "CG6",
                "E_INTERPRET",
                "R_BENEFIT",
                GenerationMeaningRelationIR::Goal,
            ),
            meaning_edge(
                "CG7",
                "E_VERIFY",
                "R_BENEFIT",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "CG8",
                "E_POSITIVE",
                "R_BENEFIT",
                GenerationMeaningRelationIR::Goal,
            ),
            meaning_edge(
                "CG9",
                "E_NEGATIVE",
                "R_TASK",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "CG10",
                "E_UNKNOWN",
                "R_BENEFIT",
                GenerationMeaningRelationIR::Theme,
            ),
        ],
    );
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GATE_TASK",
            _ => "EXPR.EN.RUNTIME_GATE_TASK",
        },
        language,
        "C_GATE_TASK",
        &format!("‘{}’", task_surface.trim()),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:CONTINUATION_TASK",
    )?;
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GATE_BENEFIT",
            _ => "EXPR.EN.RUNTIME_GATE_BENEFIT",
        },
        language,
        "C_GATE_BENEFIT",
        &format!("‘{}’", benefit_surface.trim()),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:REQUIRED_BENEFIT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Future,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_continuation_gate_followup_from_knowledge(
    language: LanguageCodeIR,
    task_surface: &str,
    benefit_surface: &str,
    grounding_refs: &[String],
    followup: GenerationContinuationGateFollowupIR,
) -> Result<GenerativeLanguageIR, String> {
    if task_surface.trim().is_empty() || benefit_surface.trim().is_empty() {
        return Err("CONTINUATION_GATE_REQUIRES_TASK_AND_BENEFIT".to_string());
    }
    let grounding_refs = if grounding_refs.is_empty() {
        vec![format!("CONTINUATION_GATE_FOLLOWUP:{followup:?}")]
    } else {
        grounding_refs.to_vec()
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };
    let (nodes, edges) = match followup {
        GenerationContinuationGateFollowupIR::PendingDecision => (
            vec![
                node(
                    "E_BOUNDARY",
                    "C_GATE_NOT_VERIFIED",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_PROXY_BOUNDARY",
                    "C_GATE_PROXY_INSUFFICIENT",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_NEXT",
                    "C_GATE_VERIFY_OR_ASK_STOP",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node("R_TASK", "C_GATE_TASK", GenerationMeaningNodeKindIR::Entity),
                node(
                    "R_BENEFIT",
                    "C_GATE_BENEFIT",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![
                meaning_edge(
                    "CF1",
                    "E_BOUNDARY",
                    "E_PROXY_BOUNDARY",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "CF2",
                    "E_PROXY_BOUNDARY",
                    "E_NEXT",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "CF3",
                    "E_BOUNDARY",
                    "R_BENEFIT",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CF4",
                    "E_PROXY_BOUNDARY",
                    "R_TASK",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CF5",
                    "E_NEXT",
                    "R_BENEFIT",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge("CF6", "E_NEXT", "R_TASK", GenerationMeaningRelationIR::Goal),
            ],
        ),
        GenerationContinuationGateFollowupIR::ProxyEvidence => (
            vec![
                node(
                    "E_RECORD",
                    "C_GATE_RECORD_PROXY",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_BOUNDARY",
                    "C_GATE_PROXY_NOT_BENEFIT",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node("R_TASK", "C_GATE_TASK", GenerationMeaningNodeKindIR::Entity),
                node(
                    "R_BENEFIT",
                    "C_GATE_BENEFIT",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![
                meaning_edge(
                    "CF1",
                    "E_RECORD",
                    "E_BOUNDARY",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "CF2",
                    "E_RECORD",
                    "R_TASK",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CF3",
                    "E_BOUNDARY",
                    "R_BENEFIT",
                    GenerationMeaningRelationIR::Theme,
                ),
            ],
        ),
    };
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GATE_FOLLOWUP_TASK",
            _ => "EXPR.EN.RUNTIME_GATE_FOLLOWUP_TASK",
        },
        language,
        "C_GATE_TASK",
        &format!("‘{}’", task_surface.trim()),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:CONTINUATION_TASK",
    )?;
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GATE_FOLLOWUP_BENEFIT",
            _ => "EXPR.EN.RUNTIME_GATE_FOLLOWUP_BENEFIT",
        },
        language,
        "C_GATE_BENEFIT",
        &format!("‘{}’", benefit_surface.trim()),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:REQUIRED_BENEFIT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_user_feedback_from_knowledge(
    language: LanguageCodeIR,
    feedback: GenerationUserFeedbackKindIR,
    target_surface: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if target_surface.trim().is_empty() {
        return Err("USER_FEEDBACK_REQUIRES_TARGET".to_string());
    }
    let grounding_refs = if grounding_refs.is_empty() {
        vec![format!("USER_FEEDBACK:{feedback:?}")]
    } else {
        grounding_refs.to_vec()
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };
    let mut nodes = vec![
        node(
            "E_ASSESS",
            "C_FEEDBACK_ASSESS",
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "R_TARGET",
            "C_FEEDBACK_TARGET",
            GenerationMeaningNodeKindIR::Entity,
        ),
        node(
            "Q_FEEDBACK",
            feedback.quality_concept_id(),
            GenerationMeaningNodeKindIR::Quality,
        ),
    ];
    let mut edges = vec![
        meaning_edge(
            "UF1",
            "E_ASSESS",
            "R_TARGET",
            GenerationMeaningRelationIR::Theme,
        ),
        meaning_edge(
            "UF2",
            "E_ASSESS",
            "Q_FEEDBACK",
            GenerationMeaningRelationIR::Property,
        ),
    ];
    if matches!(
        feedback,
        GenerationUserFeedbackKindIR::Unhelpful
            | GenerationUserFeedbackKindIR::Misunderstood
            | GenerationUserFeedbackKindIR::MissedPoint
    ) {
        nodes.extend([
            node(
                "E_REQUEST_DETAIL",
                "C_FEEDBACK_REQUEST_DETAIL",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_CORRECT",
                "C_FEEDBACK_CORRECT",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "R_MISSING_DETAIL",
                "C_FEEDBACK_MISSING_DETAIL",
                GenerationMeaningNodeKindIR::Entity,
            ),
        ]);
        edges.extend([
            meaning_edge(
                "UF3",
                "E_ASSESS",
                "E_REQUEST_DETAIL",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "UF4",
                "E_REQUEST_DETAIL",
                "E_CORRECT",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "UF5",
                "E_REQUEST_DETAIL",
                "R_MISSING_DETAIL",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "UF6",
                "E_CORRECT",
                "R_TARGET",
                GenerationMeaningRelationIR::Theme,
            ),
        ]);
    } else {
        let strategy = match feedback {
            GenerationUserFeedbackKindIR::TooVerbose => "C_FEEDBACK_CONCISE",
            GenerationUserFeedbackKindIR::TooBrief => "C_FEEDBACK_DETAIL_CONTEXT",
            GenerationUserFeedbackKindIR::Incorrect => "C_FEEDBACK_VERIFY_CORRECT",
            _ => unreachable!("request-detail feedback handled above"),
        };
        nodes.extend([
            node(
                "E_ADJUST",
                "C_FEEDBACK_ADJUST",
                GenerationMeaningNodeKindIR::Event,
            ),
            node("Q_STRATEGY", strategy, GenerationMeaningNodeKindIR::Quality),
        ]);
        edges.extend([
            meaning_edge(
                "UF3",
                "E_ASSESS",
                "E_ADJUST",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "UF4",
                "E_ADJUST",
                "R_TARGET",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "UF5",
                "E_ADJUST",
                "Q_STRATEGY",
                GenerationMeaningRelationIR::Property,
            ),
        ]);
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let target = match language {
        LanguageCodeIR::Korean => match target_surface.trim().to_lowercase().as_str() {
            "answer" | "response" => "답변".to_string(),
            "explanation" => "설명".to_string(),
            "interpretation" => "해석".to_string(),
            _ => target_surface.trim().to_string(),
        },
        _ => match target_surface.trim().to_lowercase().as_str() {
            "answer" | "response" => "the answer".to_string(),
            "explanation" => "the explanation".to_string(),
            "interpretation" => "the interpretation".to_string(),
            _ => format!("the {}", target_surface.trim()),
        },
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_FEEDBACK_TARGET",
            _ => "EXPR.EN.RUNTIME_FEEDBACK_TARGET",
        },
        language,
        "C_FEEDBACK_TARGET",
        &target,
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:USER_FEEDBACK_TARGET",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Future,
            emotion: GenerationEmotionIR::Concerned,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_discourse_group_update_from_knowledge(
    language: LanguageCodeIR,
    operation: GenerationDiscourseGroupUpdateKindIR,
    member_count: usize,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if member_count == 0 {
        return Err("DISCOURSE_GROUP_UPDATE_REQUIRES_MEMBERS".to_string());
    }
    let grounding_refs = if grounding_refs.is_empty() {
        vec![format!("DISCOURSE_GROUP_UPDATE:{operation:?}")]
    } else {
        grounding_refs.to_vec()
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            node(
                "E_UPDATE",
                operation.operation_concept_id(),
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "E_STATE",
                "C_GROUP_COUNT_STATE",
                GenerationMeaningNodeKindIR::Event,
            ),
            node(
                "R_TARGET",
                operation.target_concept_id(),
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "R_GROUP",
                operation.group_concept_id(),
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "Q_COUNT",
                "C_GROUP_MEMBER_COUNT",
                GenerationMeaningNodeKindIR::Quality,
            ),
        ],
        vec![
            meaning_edge(
                "GU1",
                "E_UPDATE",
                "R_TARGET",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "GU2",
                "E_UPDATE",
                "E_STATE",
                GenerationMeaningRelationIR::Sequence,
            ),
            meaning_edge(
                "GU3",
                "E_STATE",
                "R_GROUP",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "GU4",
                "E_STATE",
                "Q_COUNT",
                GenerationMeaningRelationIR::Property,
            ),
        ],
    );
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GROUP_MEMBER_COUNT",
            _ => "EXPR.EN.RUNTIME_GROUP_MEMBER_COUNT",
        },
        language,
        "C_GROUP_MEMBER_COUNT",
        &member_count.to_string(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_DISCOURSE_GROUP_MEMBER_COUNT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_clarification_from_knowledge(
    language: LanguageCodeIR,
    kind: GenerationClarificationKindIR,
    detail_surface: Option<&str>,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let default_grounding = format!("CLARIFICATION:{kind:?}").to_ascii_uppercase();
    let grounding_refs = if grounding_refs.is_empty() {
        vec![default_grounding]
    } else {
        grounding_refs.to_vec()
    };
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };

    let supplied_detail = detail_surface
        .map(str::trim)
        .filter(|detail| !detail.is_empty());
    let detail_is_semantically_relevant = matches!(
        kind,
        GenerationClarificationKindIR::CompetingRequest
            | GenerationClarificationKindIR::NonliteralReading
            | GenerationClarificationKindIR::VoiceAlternative
            | GenerationClarificationKindIR::Reference
    );
    let detail_surface = if kind == GenerationClarificationKindIR::Reference {
        match (language, supplied_detail) {
            (LanguageCodeIR::English, Some(detail))
                if detail
                    .chars()
                    .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character)) =>
            {
                "“that”"
            }
            (LanguageCodeIR::Korean, Some(detail))
                if matches!(
                    detail
                        .trim_matches(['‘', '’', '“', '”', '\'', '"'])
                        .to_lowercase()
                        .as_str(),
                    "it" | "that" | "this"
                ) =>
            {
                "‘그거’"
            }
            (_, Some(detail)) => detail,
            (LanguageCodeIR::Korean, None) => "‘그거’",
            (_, None) => "“that”",
        }
    } else {
        supplied_detail.unwrap_or("")
    };

    let meaning = if kind == GenerationClarificationKindIR::Reference {
        GenerationMeaningGraphIR::new(
            vec![
                node(
                    "E_REFERENCE_QUESTION",
                    kind.concept_id(),
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "E_NAME_TARGET",
                    "C_NAME_TARGET",
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_CLARIFICATION_DETAIL",
                    "C_CLARIFICATION_DETAIL",
                    GenerationMeaningNodeKindIR::Entity,
                ),
                node(
                    "R_CHANGE_TARGET",
                    "C_CHANGE_TARGET",
                    GenerationMeaningNodeKindIR::Entity,
                ),
                node("Q_SINGLE", "C_SINGLE", GenerationMeaningNodeKindIR::Quality),
            ],
            vec![
                meaning_edge(
                    "CR1",
                    "E_REFERENCE_QUESTION",
                    "E_NAME_TARGET",
                    GenerationMeaningRelationIR::Sequence,
                ),
                meaning_edge(
                    "CR2",
                    "E_REFERENCE_QUESTION",
                    "R_CLARIFICATION_DETAIL",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CR3",
                    "E_NAME_TARGET",
                    "R_CHANGE_TARGET",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CR4",
                    "E_NAME_TARGET",
                    "Q_SINGLE",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        )
    } else if detail_is_semantically_relevant && !detail_surface.is_empty() {
        GenerationMeaningGraphIR::new(
            vec![
                node(
                    "E_CLARIFICATION",
                    kind.concept_id(),
                    GenerationMeaningNodeKindIR::Event,
                ),
                node(
                    "R_CLARIFICATION_DETAIL",
                    "C_CLARIFICATION_DETAIL",
                    GenerationMeaningNodeKindIR::Entity,
                ),
            ],
            vec![meaning_edge(
                "CR1",
                "E_CLARIFICATION",
                "R_CLARIFICATION_DETAIL",
                GenerationMeaningRelationIR::Theme,
            )],
        )
    } else {
        GenerationMeaningGraphIR::new(
            vec![node(
                "E_CLARIFICATION",
                kind.concept_id(),
                GenerationMeaningNodeKindIR::Event,
            )],
            Vec::new(),
        )
    };

    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    if detail_is_semantically_relevant && !detail_surface.is_empty() {
        expressions.attach_alias(
            match language {
                LanguageCodeIR::Korean => "EXPR.KO.CLARIFICATION_DETAIL",
                _ => "EXPR.EN.CLARIFICATION_DETAIL",
            },
            language,
            "C_CLARIFICATION_DETAIL",
            detail_surface,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:CLARIFICATION_DETAIL",
        )?;
    }
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Ask,
        },
        expressions: &expressions,
    })
}

pub(crate) fn validate_interaction_boundary_generation_source(
    graph: &IllocutionaryCommitmentGraphIR,
) -> bool {
    let Some(primary) = graph.commitments.first() else {
        return false;
    };
    let consuming_force = matches!(
        primary.force,
        IllocutionaryForceIR::SelfCommitment
            | IllocutionaryForceIR::ReportedCommitment
            | IllocutionaryForceIR::CapabilityQuestion
            | IllocutionaryForceIR::DeferredConditionalRequest
            | IllocutionaryForceIR::GoalWithdrawal
            | IllocutionaryForceIR::OutcomeClaimConstraint
    );
    let activation_matches = matches!(
        (primary.force, primary.activation),
        (
            IllocutionaryForceIR::SelfCommitment
                | IllocutionaryForceIR::ReportedCommitment
                | IllocutionaryForceIR::CapabilityQuestion,
            CommitmentActivationIR::Inactive
        ) | (
            IllocutionaryForceIR::DeferredConditionalRequest,
            CommitmentActivationIR::ConditionPending
        ) | (
            IllocutionaryForceIR::GoalWithdrawal | IllocutionaryForceIR::OutcomeClaimConstraint,
            CommitmentActivationIR::Immediate
        )
    );
    let ids = graph
        .commitments
        .iter()
        .map(|commitment| commitment.commitment_id.as_str())
        .collect::<BTreeSet<_>>();
    let force_payload_matches = match primary.force {
        IllocutionaryForceIR::GoalWithdrawal => {
            graph.goal_withdrawal.as_ref().is_some_and(|withdrawal| {
                !withdrawal.evidence_surface.trim().is_empty()
                    && match withdrawal.scope {
                        GoalWithdrawalScopeIR::AllActiveGoals => withdrawal.event_ordinal.is_none(),
                        GoalWithdrawalScopeIR::EventOrdinal => {
                            withdrawal.event_ordinal.is_some_and(|ordinal| ordinal > 0)
                        }
                    }
            })
        }
        IllocutionaryForceIR::OutcomeClaimConstraint => {
            graph.outcome_claim_policy.as_ref().is_some_and(|policy| {
                policy.verified_outcome_only
                    && !policy.policy.trim().is_empty()
                    && !policy.evidence_surface.trim().is_empty()
                    && !policy.required_evidence.is_empty()
            })
        }
        _ => true,
    };
    consuming_force
        && activation_matches
        && graph.commitments.len() <= 8
        && ids.len() == graph.commitments.len()
        && force_payload_matches
        && graph.commitments.iter().all(|commitment| {
            !commitment.commitment_id.trim().is_empty()
                && !commitment.proposition_surface.trim().is_empty()
                && !commitment.external_execution_authorized
                && !commitment.evidence.is_empty()
                && commitment
                    .evidence
                    .iter()
                    .all(|evidence| !evidence.trim().is_empty())
        })
}

pub(crate) fn generate_interaction_boundary_from_knowledge(
    language: LanguageCodeIR,
    graph: &IllocutionaryCommitmentGraphIR,
    withdrawn_goal_ids: &[String],
    withdrawn_deferred_ids: &[String],
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !validate_interaction_boundary_generation_source(graph) {
        return Err("INVALID_INTERACTION_BOUNDARY_GENERATION_SOURCE".to_string());
    }
    let primary = &graph.commitments[0];
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let withdrawn_count = withdrawn_goal_ids.len() + withdrawn_deferred_ids.len();
    let event_concept = match primary.force {
        IllocutionaryForceIR::SelfCommitment => "C_INTERACTION_SELF_COMMITMENT",
        IllocutionaryForceIR::ReportedCommitment => "C_INTERACTION_REPORTED_COMMITMENT",
        IllocutionaryForceIR::CapabilityQuestion => "C_INTERACTION_CAPABILITY_QUESTION",
        IllocutionaryForceIR::DeferredConditionalRequest => "C_INTERACTION_DEFERRED_REQUEST",
        IllocutionaryForceIR::GoalWithdrawal if withdrawn_count == 0 => {
            "C_INTERACTION_WITHDRAWAL_NO_MATCH"
        }
        IllocutionaryForceIR::GoalWithdrawal => "C_INTERACTION_GOAL_WITHDRAWAL",
        IllocutionaryForceIR::OutcomeClaimConstraint => "C_INTERACTION_OUTCOME_POLICY",
        IllocutionaryForceIR::AnswerOnlyInformationRequest
        | IllocutionaryForceIR::IndirectActionRequest => {
            return Err("NON_BOUNDARY_ILLOCUTIONARY_FORCE".to_string());
        }
    };
    let mut refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    for commitment in &graph.commitments {
        refs.extend([
            format!("ILLOCUTIONARY_COMMITMENT:{}", commitment.commitment_id),
            format!("ILLOCUTIONARY_ACTOR:{:?}", commitment.actor),
            format!("ILLOCUTIONARY_ADDRESSEE:{:?}", commitment.addressee),
            format!("ILLOCUTIONARY_FORCE:{:?}", commitment.force),
            format!("ILLOCUTIONARY_ACTIVATION:{:?}", commitment.activation),
        ]);
        refs.extend(
            commitment
                .evidence
                .iter()
                .map(|evidence| format!("ILLOCUTIONARY_EVIDENCE:{evidence}")),
        );
    }
    refs.extend(
        withdrawn_goal_ids
            .iter()
            .map(|goal_id| format!("WITHDRAWN_GOAL:{goal_id}")),
    );
    refs.extend(
        withdrawn_deferred_ids
            .iter()
            .map(|commitment_id| format!("WITHDRAWN_DEFERRED:{commitment_id}")),
    );
    if let Some(policy) = &graph.outcome_claim_policy {
        refs.push(format!("OUTCOME_POLICY:{}", policy.policy));
        refs.extend(
            policy
                .required_evidence
                .iter()
                .map(|evidence| format!("OUTCOME_REQUIRED_EVIDENCE:{evidence:?}")),
        );
    }
    refs.sort();
    refs.dedup();

    let mut nodes = vec![
        GenerationMeaningNodeIR {
            node_id: "E_INTERACTION_BOUNDARY".to_string(),
            concept_id: event_concept.to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: refs.clone(),
        },
        GenerationMeaningNodeIR {
            node_id: "E_INTERACTION_AUTHORITY_BOUNDARY".to_string(),
            concept_id: "C_INTERACTION_NO_AUTHORITY".to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: refs.clone(),
        },
    ];
    let mut edges = vec![meaning_edge(
        "INTERACTION_BOUNDARY_SEQUENCE",
        "E_INTERACTION_BOUNDARY",
        "E_INTERACTION_AUTHORITY_BOUNDARY",
        GenerationMeaningRelationIR::Sequence,
    )];
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    if matches!(
        primary.force,
        IllocutionaryForceIR::SelfCommitment
            | IllocutionaryForceIR::ReportedCommitment
            | IllocutionaryForceIR::CapabilityQuestion
            | IllocutionaryForceIR::DeferredConditionalRequest
    ) {
        nodes.push(GenerationMeaningNodeIR {
            node_id: "R_INTERACTION_PROPOSITION".to_string(),
            concept_id: "C_RUNTIME_INTERACTION_PROPOSITION".to_string(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: refs.clone(),
        });
        edges.push(meaning_edge(
            "INTERACTION_PROPOSITION_THEME",
            "E_INTERACTION_BOUNDARY",
            "R_INTERACTION_PROPOSITION",
            GenerationMeaningRelationIR::Theme,
        ));
        expressions.attach_alias(
            match language {
                LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_INTERACTION_PROPOSITION",
                _ => "EXPR.EN.RUNTIME_INTERACTION_PROPOSITION",
            },
            language,
            "C_RUNTIME_INTERACTION_PROPOSITION",
            primary.proposition_surface.trim(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:INTERACTION_PROPOSITION",
        )?;
    }
    if primary.force == IllocutionaryForceIR::GoalWithdrawal && withdrawn_count > 0 {
        let withdrawal = graph
            .goal_withdrawal
            .as_ref()
            .ok_or_else(|| "WITHDRAWAL_FORCE_REQUIRES_SCOPE".to_string())?;
        let target_surface = match (language, withdrawal.scope, withdrawal.event_ordinal) {
            (LanguageCodeIR::Korean, GoalWithdrawalScopeIR::EventOrdinal, Some(ordinal)) => {
                format!("{ordinal}번째 활성 작업")
            }
            (_, GoalWithdrawalScopeIR::EventOrdinal, Some(ordinal)) => {
                format!("active action {ordinal}")
            }
            (LanguageCodeIR::Korean, GoalWithdrawalScopeIR::AllActiveGoals, _) => {
                format!("활성 작업 {withdrawn_count}개")
            }
            (_, GoalWithdrawalScopeIR::AllActiveGoals, _) => {
                format!("{withdrawn_count} active task(s)")
            }
            (_, GoalWithdrawalScopeIR::EventOrdinal, None) => {
                return Err("EVENT_ORDINAL_WITHDRAWAL_REQUIRES_ORDINAL".to_string());
            }
        };
        nodes.push(GenerationMeaningNodeIR {
            node_id: "R_INTERACTION_WITHDRAWAL_TARGET".to_string(),
            concept_id: "C_RUNTIME_INTERACTION_WITHDRAWAL_TARGET".to_string(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: refs.clone(),
        });
        edges.push(meaning_edge(
            "INTERACTION_WITHDRAWAL_GOAL",
            "E_INTERACTION_BOUNDARY",
            "R_INTERACTION_WITHDRAWAL_TARGET",
            GenerationMeaningRelationIR::Goal,
        ));
        expressions.attach_alias(
            match language {
                LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_INTERACTION_WITHDRAWAL_TARGET",
                _ => "EXPR.EN.RUNTIME_INTERACTION_WITHDRAWAL_TARGET",
            },
            language,
            "C_RUNTIME_INTERACTION_WITHDRAWAL_TARGET",
            &target_surface,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:INTERACTION_WITHDRAWAL_TARGET",
        )?;
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

fn validate_conditional_guard_generation_source(evaluation: &ConditionalGuardEvaluationIR) -> bool {
    let evidence_ids = evaluation
        .evidence
        .iter()
        .map(|evidence| evidence.belief_id.as_str())
        .collect::<BTreeSet<_>>();
    let has_support = evaluation
        .evidence
        .iter()
        .any(|evidence| evidence.polarity == GuardEvidencePolarityIR::Supports);
    let has_contradiction = evaluation
        .evidence
        .iter()
        .any(|evidence| evidence.polarity == GuardEvidencePolarityIR::Contradicts);
    let evidence_shape_matches = match evaluation.status {
        GuardStatusIR::Unresolved | GuardStatusIR::IneligibleCounterfactual => {
            evaluation.evidence.is_empty()
        }
        GuardStatusIR::SupportedByDialogueEvidence => has_support && !has_contradiction,
        GuardStatusIR::ContradictedByDialogueEvidence => has_contradiction && !has_support,
        GuardStatusIR::Contested => !evaluation.evidence.is_empty(),
    };
    evaluation.schema == CONDITIONAL_GUARD_EVALUATION_SCHEMA
        && !evaluation.guard_id.trim().is_empty()
        && !evaluation.antecedent_surface.trim().is_empty()
        && !evaluation.consequent_surface.trim().is_empty()
        && !evaluation.realized_text.trim().is_empty()
        && evaluation.evaluation_turn > 0
        && evaluation.evidence.len() <= 16
        && evidence_ids.len() == evaluation.evidence.len()
        && evaluation.unsupported_claims == 0
        && !evaluation.dialogue_truth_established
        && !evaluation.reverse_inference_authorized
        && !evaluation.external_execution_authorized
        && evaluation.deliberation_eligible
            == (evaluation.status == GuardStatusIR::SupportedByDialogueEvidence)
        && evidence_shape_matches
        && evaluation.evidence.iter().all(|evidence| {
            !evidence.belief_id.trim().is_empty()
                && !evidence.proposition_surface.trim().is_empty()
                && !evidence.source_actor.trim().is_empty()
                && evidence.introduced_turn > 0
                && evidence.introduced_turn <= evaluation.evaluation_turn
                && evidence.modal_world == ModalWorldIR::Actual
                && !evidence.dialogue_truth_established
                && !evidence.external_execution_authorized
        })
}

pub(crate) fn generate_conditional_guard_from_knowledge(
    language: LanguageCodeIR,
    evaluation: &ConditionalGuardEvaluationIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !validate_conditional_guard_generation_source(evaluation) {
        return Err("INVALID_CONDITIONAL_GUARD_GENERATION_SOURCE".to_string());
    }
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let status_concept = match evaluation.status {
        GuardStatusIR::Unresolved => "C_GUARD_UNRESOLVED",
        GuardStatusIR::SupportedByDialogueEvidence => "C_GUARD_SUPPORTED",
        GuardStatusIR::ContradictedByDialogueEvidence => "C_GUARD_CONTRADICTED",
        GuardStatusIR::Contested => "C_GUARD_CONTESTED",
        GuardStatusIR::IneligibleCounterfactual => "C_GUARD_COUNTERFACTUAL",
    };
    let mut refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    refs.extend([
        format!("CONDITIONAL_GUARD_ID:{}", evaluation.guard_id),
        format!("CONDITIONAL_GUARD_STATUS:{:?}", evaluation.status),
        format!("CONDITIONAL_GUARD_TURN:{}", evaluation.evaluation_turn),
        format!(
            "CONDITIONAL_GUARD_STATUS_CHANGED:{}",
            evaluation.status_changed
        ),
        format!(
            "CONDITIONAL_GUARD_DELIBERATION_ELIGIBLE:{}",
            evaluation.deliberation_eligible
        ),
    ]);
    for evidence in &evaluation.evidence {
        refs.extend([
            format!("GUARD_EVIDENCE_ID:{}", evidence.belief_id),
            format!("GUARD_EVIDENCE_SOURCE:{}", evidence.source_actor),
            format!("GUARD_EVIDENCE_POLARITY:{:?}", evidence.polarity),
            format!("GUARD_EVIDENCE_TURN:{}", evidence.introduced_turn),
            format!("GUARD_EVIDENCE_WORLD:{:?}", evidence.modal_world),
        ]);
    }
    refs.sort();
    refs.dedup();

    let meaning = GenerationMeaningGraphIR::new(
        vec![
            GenerationMeaningNodeIR {
                node_id: "E_GUARD_STATUS".to_string(),
                concept_id: status_concept.to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: refs.clone(),
            },
            GenerationMeaningNodeIR {
                node_id: "R_GUARD_ANTECEDENT".to_string(),
                concept_id: "C_RUNTIME_GUARD_ANTECEDENT".to_string(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: refs.clone(),
            },
            GenerationMeaningNodeIR {
                node_id: "R_GUARD_CONSEQUENT".to_string(),
                concept_id: "C_RUNTIME_GUARD_CONSEQUENT".to_string(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: refs.clone(),
            },
            GenerationMeaningNodeIR {
                node_id: "E_GUARD_BOUNDARY".to_string(),
                concept_id: "C_GUARD_NO_REVERSE_INFERENCE".to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: refs,
            },
        ],
        vec![
            meaning_edge(
                "GUARD_THEME",
                "E_GUARD_STATUS",
                "R_GUARD_ANTECEDENT",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "GUARD_GOAL",
                "E_GUARD_STATUS",
                "R_GUARD_CONSEQUENT",
                GenerationMeaningRelationIR::Goal,
            ),
            meaning_edge(
                "GUARD_BOUNDARY_SEQUENCE",
                "E_GUARD_STATUS",
                "E_GUARD_BOUNDARY",
                GenerationMeaningRelationIR::Sequence,
            ),
        ],
    );
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GUARD_ANTECEDENT",
            _ => "EXPR.EN.RUNTIME_GUARD_ANTECEDENT",
        },
        language,
        "C_RUNTIME_GUARD_ANTECEDENT",
        evaluation.antecedent_surface.trim(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:GUARD_ANTECEDENT",
    )?;
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_GUARD_CONSEQUENT",
            _ => "EXPR.EN.RUNTIME_GUARD_CONSEQUENT",
        },
        language,
        "C_RUNTIME_GUARD_CONSEQUENT",
        evaluation.consequent_surface.trim(),
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:GUARD_CONSEQUENT",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_definition_grounding_from_knowledge(
    language: LanguageCodeIR,
    grounding: &DefinitionGroundingIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !grounding.validate() {
        return Err("INVALID_DEFINITION_GROUNDING_GENERATION_SOURCE".to_string());
    }
    if grounding.disposition == DefinitionGroundingDispositionIR::NoDefinition {
        return Err("NO_DEFINITION_HAS_NO_REALIZATION".to_string());
    }
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut base_refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    base_refs.push(format!(
        "DEFINITION_GROUNDING_DISPOSITION:{:?}",
        grounding.disposition
    ));
    base_refs.push(format!(
        "DEFINITION_GROUNDING_LEXICAL_STORE_CHANGED:{}",
        grounding.lexical_store_changed
    ));
    for reason in &grounding.rejection_reasons {
        base_refs.push(format!("DEFINITION_GROUNDING_REJECTION:{reason}"));
    }
    base_refs.sort();
    base_refs.dedup();

    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let meaning = if grounding.disposition == DefinitionGroundingDispositionIR::Bound {
        let binding = grounding
            .binding
            .as_ref()
            .ok_or_else(|| "BOUND_DEFINITION_REQUIRES_BINDING".to_string())?;
        let mut binding_refs = base_refs.clone();
        binding_refs.extend([
            format!("DEFINITION_ALIAS_ID:{}", binding.alias_id),
            format!("DEFINITION_ALIAS_LANGUAGE:{:?}", binding.alias_language),
            format!("DEFINITION_INTENT_HINT:{:?}", binding.intent_hint),
            format!(
                "DEFINITION_SEMANTIC_PAYLOAD:{}",
                binding.semantic_payload_sha256
            ),
            format!("DEFINITION_PROVENANCE:{}", binding.provenance_sha256),
        ]);
        binding_refs.sort();
        binding_refs.dedup();
        let bind_concept = if grounding.lexical_store_changed {
            "C_DEFINITION_BIND_ADDED"
        } else {
            "C_DEFINITION_BIND_CONFIRMED"
        };
        let canonical_surface = match (language, binding.canonical_predicate.as_str()) {
            (LanguageCodeIR::Korean, "INVESTIGATE") => "검사".to_string(),
            (LanguageCodeIR::Korean, "REPAIR") => "수리".to_string(),
            (LanguageCodeIR::Korean, "CREATE") => "생성".to_string(),
            (LanguageCodeIR::Korean, "DELETE") => "삭제".to_string(),
            (LanguageCodeIR::Korean, "EXPLAIN") => "설명".to_string(),
            (LanguageCodeIR::Korean, "SUMMARIZE") => "요약".to_string(),
            (LanguageCodeIR::Korean, "EXECUTE") => "실행".to_string(),
            (_, "INVESTIGATE") => "inspect".to_string(),
            (_, "REPAIR") => "repair".to_string(),
            (_, "CREATE") => "create".to_string(),
            (_, "DELETE") => "delete".to_string(),
            (_, "EXPLAIN") => "explain".to_string(),
            (_, "SUMMARIZE") => "summarize".to_string(),
            (_, "EXECUTE") => "execute".to_string(),
            (LanguageCodeIR::Korean, other) => other.replace('_', " "),
            (_, other) => other.replace('_', " ").to_lowercase(),
        };
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DEFINITION_ALIAS"),
            language,
            "C_RUNTIME_DEFINITION_ALIAS",
            binding.alias_surface.trim(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DEFINITION_ALIAS",
        )?;
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DEFINITION_CANONICAL"),
            language,
            "C_RUNTIME_DEFINITION_CANONICAL",
            &canonical_surface,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DEFINITION_CANONICAL",
        )?;
        GenerationMeaningGraphIR::new(
            vec![
                GenerationMeaningNodeIR {
                    node_id: "E_DEFINITION_BIND".to_string(),
                    concept_id: bind_concept.to_string(),
                    kind: GenerationMeaningNodeKindIR::Event,
                    grounding_refs: binding_refs.clone(),
                },
                GenerationMeaningNodeIR {
                    node_id: "R_DEFINITION_ALIAS".to_string(),
                    concept_id: "C_RUNTIME_DEFINITION_ALIAS".to_string(),
                    kind: GenerationMeaningNodeKindIR::Entity,
                    grounding_refs: binding_refs.clone(),
                },
                GenerationMeaningNodeIR {
                    node_id: "R_DEFINITION_CANONICAL".to_string(),
                    concept_id: "C_RUNTIME_DEFINITION_CANONICAL".to_string(),
                    kind: GenerationMeaningNodeKindIR::Entity,
                    grounding_refs: binding_refs,
                },
                GenerationMeaningNodeIR {
                    node_id: "E_DEFINITION_BOUNDARY".to_string(),
                    concept_id: "C_DEFINITION_PAYLOAD_BOUNDARY".to_string(),
                    kind: GenerationMeaningNodeKindIR::Event,
                    grounding_refs: base_refs,
                },
            ],
            vec![
                meaning_edge(
                    "DEF_THEME",
                    "E_DEFINITION_BIND",
                    "R_DEFINITION_ALIAS",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "DEF_GOAL",
                    "E_DEFINITION_BIND",
                    "R_DEFINITION_CANONICAL",
                    GenerationMeaningRelationIR::Goal,
                ),
                meaning_edge(
                    "DEF_SEQUENCE",
                    "E_DEFINITION_BIND",
                    "E_DEFINITION_BOUNDARY",
                    GenerationMeaningRelationIR::Sequence,
                ),
            ],
        )
    } else {
        let concept_id = match grounding.disposition {
            DefinitionGroundingDispositionIR::ConflictRejected => "C_DEFINITION_REJECT_CONFLICT",
            DefinitionGroundingDispositionIR::NonAssertedRejected => {
                "C_DEFINITION_REJECT_NONASSERTED"
            }
            DefinitionGroundingDispositionIR::AmbiguousRejected => "C_DEFINITION_REJECT_AMBIGUOUS",
            DefinitionGroundingDispositionIR::UnresolvedRejected => {
                "C_DEFINITION_REJECT_UNRESOLVED"
            }
            DefinitionGroundingDispositionIR::InvalidAliasRejected => {
                "C_DEFINITION_REJECT_INVALID_ALIAS"
            }
            DefinitionGroundingDispositionIR::NoDefinition
            | DefinitionGroundingDispositionIR::Bound => {
                return Err("INVALID_DEFINITION_DISPOSITION_BRANCH".to_string());
            }
        };
        GenerationMeaningGraphIR::new(
            vec![GenerationMeaningNodeIR {
                node_id: "E_DEFINITION_REJECTION".to_string(),
                concept_id: concept_id.to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: base_refs,
            }],
            Vec::new(),
        )
    };

    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

fn generate_content_projection(
    language: LanguageCodeIR,
    projection: &crate::proposition_content::ContentProjectionIR,
) -> Result<GenerativeLanguageIR, String> {
    use crate::proposition_content::ContentSlotIR;
    if !projection.validate() {
        return Err("INVALID_CONTENT_PROJECTION".into());
    }
    let korean = language == LanguageCodeIR::Korean;
    let slot = match (korean, projection.binding.slot) {
        (true, ContentSlotIR::Cause) => "말한 이유",
        (false, ContentSlotIR::Cause) => "stated reason",
        (true, ContentSlotIR::Agent) => "행위자",
        (false, ContentSlotIR::Agent) => "actor",
        (true, ContentSlotIR::Theme) => "대상",
        (false, ContentSlotIR::Theme) => "object",
        (true, ContentSlotIR::Definition) => "정의",
        (false, ContentSlotIR::Definition) => "definition",
        (true, ContentSlotIR::Summary) => "요점",
        (false, ContentSlotIR::Summary) => "summary",
        (true, ContentSlotIR::Manner) => "방식",
        (false, ContentSlotIR::Manner) => "method",
    };
    let actor = if projection.source_actor == "DIALOGUE_USER" {
        if korean {
            "네 말"
        } else {
            "your account"
        }
    } else {
        &projection.source_actor
    };
    let grounding = vec![
        format!("DIALOGUE_BELIEF_ID:{}", projection.belief_id),
        projection.binding.grammar_evidence.clone(),
    ];
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    for (id, surface, kind, pos) in [
        (
            "C_CONTENT_PROJECTION",
            if korean { "이다" } else { "is" },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "C_CONTENT_VALUE",
            projection.binding.value.as_str(),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "C_CONTENT_SLOT",
            slot,
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "C_CONTENT_SOURCE",
            actor,
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.CONTENT.{id}"),
            language,
            id,
            surface,
            pos,
            "RUNTIME_REFERENT_SURFACE:CONTENT_PROJECTION",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.to_string(),
            concept_id: id.to_string(),
            kind,
            grounding_refs: grounding.clone(),
        });
    }
    let meaning = GenerationMeaningGraphIR::new(
        nodes,
        vec![
            meaning_edge(
                "VALUE",
                "C_CONTENT_PROJECTION",
                "C_CONTENT_VALUE",
                GenerationMeaningRelationIR::Property,
            ),
            meaning_edge(
                "SLOT",
                "C_CONTENT_PROJECTION",
                "C_CONTENT_SLOT",
                GenerationMeaningRelationIR::Theme,
            ),
            meaning_edge(
                "SOURCE",
                "C_CONTENT_PROJECTION",
                "C_CONTENT_SOURCE",
                GenerationMeaningRelationIR::Goal,
            ),
        ],
    );
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &store,
    })
}

fn realize_content_projection(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let Some(source) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
    else {
        return output;
    };
    let Some(slot) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected) else {
        return output;
    };
    let Some(value) = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
    else {
        return output;
    };
    if context.language == LanguageCodeIR::Korean {
        push_expression_token(
            &mut output,
            source,
            format!("{}에 따르면,", source.expression.lexical_root),
        );
        push_expression_token(
            &mut output,
            slot,
            format!(
                "{}{}",
                slot.expression.lexical_root,
                korean_particle(&slot.expression.lexical_root, "은", "는")
            ),
        );
        push_expression_token(
            &mut output,
            value,
            format!("‘{}’", value.expression.lexical_root),
        );
        let ending = if context.register == LanguageRegisterIR::Formal {
            "입니다."
        } else {
            "이야."
        };
        push_expression_token(&mut output, predicate, ending.to_string());
        if let Some(token) = output.last_mut() {
            token.attach_left = true;
        }
    } else {
        push_grammar_token(
            &mut output,
            "According to",
            "EN.ATTRIBUTED_CONTENT",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            source,
            format!("{},", source.expression.lexical_root),
        );
        push_grammar_token(
            &mut output,
            "the",
            "EN.DEFINITE_SLOT",
            &clause.event_node_id,
        );
        push_expression_token(&mut output, slot, slot.expression.lexical_root.clone());
        push_expression_token(&mut output, predicate, "is".into());
        push_expression_token(
            &mut output,
            value,
            format!("‘{}’.", value.expression.lexical_root),
        );
    }
    output
}

pub(crate) fn generate_discourse_answer_from_knowledge(
    language: LanguageCodeIR,
    answer: &DiscourseAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !answer.validate() {
        return Err("INVALID_DISCOURSE_ANSWER_GENERATION_SOURCE".to_string());
    }
    if let Some(world) = &answer.world_reasoning {
        return generate_world_decision(language, world);
    }
    if let Some(c) = &answer.world_clarification {
        return generate_world_clarification(language, c);
    }
    if let Some(update) = &answer.world_memory_update {
        return generate_world_memory_update(language, update);
    }
    if let Some(projection) = &answer.content_projection {
        return generate_content_projection(language, projection);
    }
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut base_refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    base_refs.push(format!(
        "DISCOURSE_ANSWER_DISPOSITION:{:?}",
        answer.disposition
    ));
    base_refs.sort();
    base_refs.dedup();

    let include_records = matches!(
        answer.disposition,
        DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
            | DiscourseAnswerDispositionIR::MultipleDialogueRecords
            | DiscourseAnswerDispositionIR::ConflictingDialogueRecords
            | DiscourseAnswerDispositionIR::NoConflictRecorded
            | DiscourseAnswerDispositionIR::DialogueTruthNotEstablished
    );
    let record_concept = if answer.query.kind == DiscourseQueryKindIR::ModalStatus {
        "C_DIALOGUE_ANSWER_MODAL"
    } else {
        "C_DIALOGUE_ANSWER_RECORD"
    };
    let terminal_concept = match answer.disposition {
        DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        | DiscourseAnswerDispositionIR::MultipleDialogueRecords
        | DiscourseAnswerDispositionIR::DialogueTruthNotEstablished => "C_DIALOGUE_ANSWER_NOT_FACT",
        DiscourseAnswerDispositionIR::ConflictingDialogueRecords => "C_DIALOGUE_ANSWER_CONFLICT",
        DiscourseAnswerDispositionIR::NoConflictRecorded => "C_DIALOGUE_ANSWER_NO_CONFLICT",
        DiscourseAnswerDispositionIR::PresuppositionUnverified => {
            "C_DIALOGUE_ANSWER_PRESUPPOSITION"
        }
        DiscourseAnswerDispositionIR::NoMatchingRecord => "C_DIALOGUE_ANSWER_NO_MATCH",
        DiscourseAnswerDispositionIR::AmbiguousQuery => "C_DIALOGUE_ANSWER_AMBIGUOUS",
    };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let mut previous_event_id: Option<String> = None;
    if include_records {
        for (index, evidence) in answer.evidence.iter().enumerate() {
            let ordinal = index + 1;
            let event_id = format!("E_DIALOGUE_RECORD_{ordinal:03}");
            let proposition_id = format!("R_DIALOGUE_PROPOSITION_{ordinal:03}");
            let proposition_concept = format!("C_RUNTIME_DIALOGUE_PROPOSITION_{ordinal:03}");
            let attribution_id = format!("Q_DIALOGUE_ATTRIBUTION_{ordinal:03}");
            let attribution_concept = format!("C_RUNTIME_DIALOGUE_ATTRIBUTION_{ordinal:03}");
            let mut evidence_refs = base_refs.clone();
            evidence_refs.push(format!("DIALOGUE_BELIEF_ID:{}", evidence.belief_id));
            evidence_refs.sort();
            evidence_refs.dedup();
            nodes.push(GenerationMeaningNodeIR {
                node_id: event_id.clone(),
                concept_id: record_concept.to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: proposition_id.clone(),
                concept_id: proposition_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: attribution_id.clone(),
                concept_id: attribution_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Quality,
                grounding_refs: evidence_refs,
            });
            edges.push(meaning_edge(
                &format!("DA_THEME_{ordinal:03}"),
                &event_id,
                &proposition_id,
                GenerationMeaningRelationIR::Theme,
            ));
            edges.push(meaning_edge(
                &format!("DA_PROPERTY_{ordinal:03}"),
                &event_id,
                &attribution_id,
                GenerationMeaningRelationIR::Property,
            ));
            if let Some(previous) = previous_event_id.as_deref() {
                edges.push(meaning_edge(
                    &format!("DA_SEQUENCE_{ordinal:03}"),
                    previous,
                    &event_id,
                    GenerationMeaningRelationIR::Sequence,
                ));
            }
            expressions.attach_alias(
                &format!("EXPR.{language:?}.DIALOGUE_PROPOSITION.{ordinal:03}"),
                language,
                &proposition_concept,
                evidence.proposition_surface.trim(),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:DIALOGUE_PROPOSITION",
            )?;
            expressions.attach_alias(
                &format!("EXPR.{language:?}.DIALOGUE_ATTRIBUTION.{ordinal:03}"),
                language,
                &attribution_concept,
                &dialogue_attribution_surface(language, answer.query.kind, evidence),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:DIALOGUE_ATTRIBUTION",
            )?;
            previous_event_id = Some(event_id);
        }
    }

    let terminal_id = "E_DIALOGUE_ANSWER_TERMINAL".to_string();
    nodes.push(GenerationMeaningNodeIR {
        node_id: terminal_id.clone(),
        concept_id: terminal_concept.to_string(),
        kind: GenerationMeaningNodeKindIR::Event,
        grounding_refs: base_refs.clone(),
    });
    if let Some(previous) = previous_event_id.as_deref() {
        edges.push(meaning_edge(
            "DA_SEQUENCE_TERMINAL",
            previous,
            &terminal_id,
            GenerationMeaningRelationIR::Sequence,
        ));
    }
    if answer.disposition == DiscourseAnswerDispositionIR::NoMatchingRecord
        && !answer.query.topic_terms.is_empty()
    {
        nodes.push(GenerationMeaningNodeIR {
            node_id: "R_GAP_TOPIC".into(),
            concept_id: "C_RUNTIME_GAP_TOPIC".into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: base_refs.clone(),
        });
        edges.push(meaning_edge(
            "GAP_TOPIC",
            &terminal_id,
            "R_GAP_TOPIC",
            GenerationMeaningRelationIR::Theme,
        ));
        expressions.attach_alias(
            "EXPR.GAP.TOPIC",
            language,
            "C_RUNTIME_GAP_TOPIC",
            &answer.query.topic_terms.join(" "),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:REQUEST_TOPIC",
        )?;
    }
    if answer.disposition == DiscourseAnswerDispositionIR::PresuppositionUnverified {
        let premise = answer
            .query
            .presuppositions
            .first()
            .map(|item| item.surface_text.trim())
            .filter(|item| !item.is_empty())
            .unwrap_or_else(|| {
                if language == LanguageCodeIR::Korean {
                    "질문의 전제"
                } else {
                    "the question premise"
                }
            });
        nodes.push(GenerationMeaningNodeIR {
            node_id: "R_DIALOGUE_PREMISE".to_string(),
            concept_id: "C_RUNTIME_DIALOGUE_PREMISE".to_string(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: base_refs,
        });
        edges.push(meaning_edge(
            "DA_PREMISE_THEME",
            &terminal_id,
            "R_DIALOGUE_PREMISE",
            GenerationMeaningRelationIR::Theme,
        ));
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DIALOGUE_PREMISE"),
            language,
            "C_RUNTIME_DIALOGUE_PREMISE",
            premise,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DIALOGUE_PREMISE",
        )?;
    }

    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_dialogue_relation_answer_from_knowledge(
    language: LanguageCodeIR,
    answer: &DialogueRelationAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !answer.validate() {
        return Err("INVALID_DIALOGUE_RELATION_ANSWER_GENERATION_SOURCE".to_string());
    }
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut base_refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    base_refs.push(format!(
        "DIALOGUE_RELATION_DISPOSITION:{:?}",
        answer.disposition
    ));
    base_refs.push(format!(
        "DIALOGUE_RELATION_QUERY_KIND:{:?}",
        answer.query.kind
    ));
    for path in &answer.paths {
        base_refs.push(format!(
            "DIALOGUE_RELATION_PATH:{}:HOPS:{}",
            path.path_id, path.hop_count
        ));
    }
    base_refs.sort();
    base_refs.dedup();

    let terminal_concept = match answer.disposition {
        DialogueRelationAnswerDispositionIR::NoMatchingDialogueRelation => {
            "C_DIALOGUE_RELATION_NO_MATCH"
        }
        DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath => {
            "C_DIALOGUE_RELATION_TRANSITIVE_BOUNDARY"
        }
        DialogueRelationAnswerDispositionIR::MultipleDialogueRelations => {
            "C_DIALOGUE_RELATION_MULTIPLE_BOUNDARY"
        }
        DialogueRelationAnswerDispositionIR::AnsweredFromDialogueRelation => {
            match answer.query.kind {
                DialogueRelationQueryKindIR::CauseOf => "C_DIALOGUE_RELATION_CAUSE_BOUNDARY",
                DialogueRelationQueryKindIR::ConsequenceOf => "C_DIALOGUE_RELATION_RESULT_BOUNDARY",
                DialogueRelationQueryKindIR::ConcessionOutcome => {
                    "C_DIALOGUE_RELATION_CONCESSION_BOUNDARY"
                }
            }
        }
    };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let mut previous_event_id: Option<String> = None;
    for (index, evidence) in answer.evidence.iter().enumerate() {
        let ordinal = index + 1;
        let relation_concept = match evidence.kind {
            DialogueRelationKindIR::Cause => "C_DIALOGUE_RELATION_CAUSE_EDGE",
            DialogueRelationKindIR::Consequence => "C_DIALOGUE_RELATION_RESULT_EDGE",
            DialogueRelationKindIR::Concession => "C_DIALOGUE_RELATION_CONCESSION_EDGE",
        };
        let relation_event_id = format!("E_DIALOGUE_RELATION_{ordinal:03}");
        let source_node_id = format!("R_DIALOGUE_RELATION_SOURCE_{ordinal:03}");
        let source_concept = format!("C_RUNTIME_DIALOGUE_RELATION_SOURCE_{ordinal:03}");
        let target_node_id = format!("R_DIALOGUE_RELATION_TARGET_{ordinal:03}");
        let target_concept = format!("C_RUNTIME_DIALOGUE_RELATION_TARGET_{ordinal:03}");
        let mut evidence_refs = base_refs.clone();
        evidence_refs.push(format!("DIALOGUE_RELATION_ID:{}", evidence.relation_id));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_SOURCE_STATUS:{:?}",
            evidence.source_belief_status
        ));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_TARGET_STATUS:{:?}",
            evidence.target_belief_status
        ));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_SOURCE_WORLD:{:?}",
            evidence.source_modal_world
        ));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_TARGET_WORLD:{:?}",
            evidence.target_modal_world
        ));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_SOURCE_POLARITY:{:?}",
            evidence.source_polarity
        ));
        evidence_refs.push(format!(
            "DIALOGUE_RELATION_TARGET_POLARITY:{:?}",
            evidence.target_polarity
        ));
        evidence_refs.sort();
        evidence_refs.dedup();
        nodes.push(GenerationMeaningNodeIR {
            node_id: relation_event_id.clone(),
            concept_id: relation_concept.to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: evidence_refs.clone(),
        });
        nodes.push(GenerationMeaningNodeIR {
            node_id: source_node_id.clone(),
            concept_id: source_concept.clone(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: evidence_refs.clone(),
        });
        nodes.push(GenerationMeaningNodeIR {
            node_id: target_node_id.clone(),
            concept_id: target_concept.clone(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: evidence_refs,
        });
        edges.push(meaning_edge(
            &format!("DR_THEME_{ordinal:03}"),
            &relation_event_id,
            &source_node_id,
            GenerationMeaningRelationIR::Theme,
        ));
        edges.push(meaning_edge(
            &format!("DR_GOAL_{ordinal:03}"),
            &relation_event_id,
            &target_node_id,
            GenerationMeaningRelationIR::Goal,
        ));
        if let Some(previous) = previous_event_id.as_deref() {
            edges.push(meaning_edge(
                &format!("DR_SEQUENCE_{ordinal:03}"),
                previous,
                &relation_event_id,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DIALOGUE_RELATION_SOURCE.{ordinal:03}"),
            language,
            &source_concept,
            evidence.source_summary.trim(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DIALOGUE_RELATION_SOURCE",
        )?;
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DIALOGUE_RELATION_TARGET.{ordinal:03}"),
            language,
            &target_concept,
            evidence.target_summary.trim(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DIALOGUE_RELATION_TARGET",
        )?;
        previous_event_id = Some(relation_event_id);
    }

    let terminal_id = "E_DIALOGUE_RELATION_TERMINAL".to_string();
    nodes.push(GenerationMeaningNodeIR {
        node_id: terminal_id.clone(),
        concept_id: terminal_concept.to_string(),
        kind: GenerationMeaningNodeKindIR::Event,
        grounding_refs: base_refs.clone(),
    });
    if let Some(previous) = previous_event_id.as_deref() {
        edges.push(meaning_edge(
            "DR_SEQUENCE_TERMINAL",
            previous,
            &terminal_id,
            GenerationMeaningRelationIR::Sequence,
        ));
    }
    previous_event_id = Some(terminal_id.clone());

    let path_measure = match answer.disposition {
        DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath => {
            answer.paths.iter().map(|path| path.hop_count).max()
        }
        DialogueRelationAnswerDispositionIR::MultipleDialogueRelations => Some(answer.paths.len()),
        _ => None,
    };
    if let Some(value) = path_measure {
        nodes.push(GenerationMeaningNodeIR {
            node_id: "Q_DIALOGUE_RELATION_PATH_MEASURE".to_string(),
            concept_id: "C_RUNTIME_DIALOGUE_RELATION_PATH_MEASURE".to_string(),
            kind: GenerationMeaningNodeKindIR::Quality,
            grounding_refs: base_refs.clone(),
        });
        edges.push(meaning_edge(
            "DR_PATH_MEASURE",
            &terminal_id,
            "Q_DIALOGUE_RELATION_PATH_MEASURE",
            GenerationMeaningRelationIR::Property,
        ));
        expressions.attach_alias(
            &format!("EXPR.{language:?}.DIALOGUE_RELATION_PATH_MEASURE"),
            language,
            "C_RUNTIME_DIALOGUE_RELATION_PATH_MEASURE",
            &value.to_string(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:DIALOGUE_RELATION_PATH_MEASURE",
        )?;
    }

    let warnings = [
        (
            answer
                .paths
                .iter()
                .any(|path| path.contains_nonactual_world),
            "C_DIALOGUE_RELATION_NONACTUAL_WARNING",
            "NONACTUAL",
        ),
        (
            answer
                .paths
                .iter()
                .any(|path| path.contains_contested_endpoint),
            "C_DIALOGUE_RELATION_CONTESTED_WARNING",
            "CONTESTED",
        ),
        (
            answer.paths.iter().any(|path| path.truncated_by_hop_limit),
            "C_DIALOGUE_RELATION_TRUNCATED_WARNING",
            "TRUNCATED",
        ),
    ];
    for (enabled, concept_id, suffix) in warnings {
        if !enabled {
            continue;
        }
        let warning_id = format!("E_DIALOGUE_RELATION_WARNING_{suffix}");
        nodes.push(GenerationMeaningNodeIR {
            node_id: warning_id.clone(),
            concept_id: concept_id.to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: base_refs.clone(),
        });
        if let Some(previous) = previous_event_id.as_deref() {
            edges.push(meaning_edge(
                &format!("DR_SEQUENCE_WARNING_{suffix}"),
                previous,
                &warning_id,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
        previous_event_id = Some(warning_id);
    }

    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_temporal_answer_from_knowledge(
    language: LanguageCodeIR,
    answer: &TemporalAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    if !answer.validate() {
        return Err("INVALID_TEMPORAL_ANSWER_GENERATION_SOURCE".to_string());
    }
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut base_refs = grounding_refs
        .iter()
        .filter(|item| !item.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    base_refs.push(format!(
        "TEMPORAL_ANSWER_DISPOSITION:{:?}",
        answer.disposition
    ));
    base_refs.push(format!("TEMPORAL_QUERY_KIND:{:?}", answer.query.kind));
    base_refs.sort();
    base_refs.dedup();

    let terminal_concept = match answer.disposition {
        TemporalAnswerDispositionIR::AnsweredFromTemporalGraph => {
            "C_TEMPORAL_ANSWER_EVIDENCE_BOUNDARY"
        }
        TemporalAnswerDispositionIR::AnsweredByTransitivePath => {
            "C_TEMPORAL_ANSWER_TRANSITIVE_BOUNDARY"
        }
        TemporalAnswerDispositionIR::NoMatchingEvent => "C_TEMPORAL_ANSWER_NO_MATCH",
        TemporalAnswerDispositionIR::NoRecordedRelation => "C_TEMPORAL_ANSWER_NO_RELATION",
        TemporalAnswerDispositionIR::AmbiguousEvent => "C_TEMPORAL_ANSWER_AMBIGUOUS",
        TemporalAnswerDispositionIR::ConflictingRelations => "C_TEMPORAL_ANSWER_CONFLICT",
        TemporalAnswerDispositionIR::EventTimeNotRecorded => "C_TEMPORAL_ANSWER_TIME_MISSING",
    };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let mut previous_event_id: Option<String> = None;

    if answer.query.kind == TemporalQueryKindIR::EventTime
        && matches!(
            answer.disposition,
            TemporalAnswerDispositionIR::AnsweredFromTemporalGraph
                | TemporalAnswerDispositionIR::AnsweredByTransitivePath
                | TemporalAnswerDispositionIR::AmbiguousEvent
        )
    {
        for (index, event) in answer.event_evidence.iter().enumerate() {
            let Some(time) = event.event_time.as_ref() else {
                continue;
            };
            let ordinal = index + 1;
            let answer_event_id = format!("E_TEMPORAL_TIME_{ordinal:03}");
            let event_node_id = format!("R_TEMPORAL_EVENT_{ordinal:03}");
            let event_concept = format!("C_RUNTIME_TEMPORAL_EVENT_{ordinal:03}");
            let time_node_id = format!("Q_TEMPORAL_TIME_{ordinal:03}");
            let time_concept = format!("C_RUNTIME_TEMPORAL_TIME_{ordinal:03}");
            let mut evidence_refs = base_refs.clone();
            evidence_refs.push(format!("TEMPORAL_EVENT_ID:{}", event.event_id));
            evidence_refs.push(format!("TEMPORAL_EVENT_TIME:{}", time.normalized_value));
            evidence_refs.push(format!("TEMPORAL_EVENT_WORLD:{:?}", event.modal_world));
            evidence_refs.sort();
            evidence_refs.dedup();
            nodes.push(GenerationMeaningNodeIR {
                node_id: answer_event_id.clone(),
                concept_id: "C_TEMPORAL_ANSWER_TIME".to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: event_node_id.clone(),
                concept_id: event_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: time_node_id.clone(),
                concept_id: time_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Quality,
                grounding_refs: evidence_refs,
            });
            edges.push(meaning_edge(
                &format!("TA_TIME_THEME_{ordinal:03}"),
                &answer_event_id,
                &event_node_id,
                GenerationMeaningRelationIR::Theme,
            ));
            edges.push(meaning_edge(
                &format!("TA_TIME_PROPERTY_{ordinal:03}"),
                &answer_event_id,
                &time_node_id,
                GenerationMeaningRelationIR::Property,
            ));
            if let Some(previous) = previous_event_id.as_deref() {
                edges.push(meaning_edge(
                    &format!("TA_TIME_SEQUENCE_{ordinal:03}"),
                    previous,
                    &answer_event_id,
                    GenerationMeaningRelationIR::Sequence,
                ));
            }
            expressions.attach_alias(
                &format!("EXPR.{language:?}.TEMPORAL_EVENT.{ordinal:03}"),
                language,
                &event_concept,
                event.surface.trim(),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:TEMPORAL_EVENT",
            )?;
            expressions.attach_alias(
                &format!("EXPR.{language:?}.TEMPORAL_TIME.{ordinal:03}"),
                language,
                &time_concept,
                &format!("{} ({})", time.surface.trim(), time.normalized_value),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:TEMPORAL_TIME",
            )?;
            previous_event_id = Some(answer_event_id);
        }
    } else if !answer.relation_evidence.is_empty() {
        for (index, relation) in answer.relation_evidence.iter().enumerate() {
            let ordinal = index + 1;
            let left = answer
                .event_evidence
                .iter()
                .find(|event| event.event_id == relation.left_event_id)
                .ok_or_else(|| "TEMPORAL_RELATION_LEFT_EVENT_MISSING".to_string())?;
            let right = answer
                .event_evidence
                .iter()
                .find(|event| event.event_id == relation.right_event_id)
                .ok_or_else(|| "TEMPORAL_RELATION_RIGHT_EVENT_MISSING".to_string())?;
            let relation_concept = match relation.kind {
                TemporalRelationKindIR::Before => "C_TEMPORAL_ANSWER_BEFORE",
                TemporalRelationKindIR::During => "C_TEMPORAL_ANSWER_DURING",
                TemporalRelationKindIR::Simultaneous => "C_TEMPORAL_ANSWER_SIMULTANEOUS",
            };
            let relation_event_id = format!("E_TEMPORAL_RELATION_{ordinal:03}");
            let left_node_id = format!("R_TEMPORAL_LEFT_EVENT_{ordinal:03}");
            let left_concept = format!("C_RUNTIME_TEMPORAL_LEFT_EVENT_{ordinal:03}");
            let right_node_id = format!("R_TEMPORAL_RIGHT_EVENT_{ordinal:03}");
            let right_concept = format!("C_RUNTIME_TEMPORAL_RIGHT_EVENT_{ordinal:03}");
            let mut evidence_refs = base_refs.clone();
            evidence_refs.push(format!("TEMPORAL_RELATION_ID:{}", relation.relation_id));
            evidence_refs.push(format!("TEMPORAL_RELATION_KIND:{:?}", relation.kind));
            evidence_refs.push(format!("TEMPORAL_RELATION_STATUS:{:?}", relation.status));
            evidence_refs.sort();
            evidence_refs.dedup();
            nodes.push(GenerationMeaningNodeIR {
                node_id: relation_event_id.clone(),
                concept_id: relation_concept.to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: left_node_id.clone(),
                concept_id: left_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: right_node_id.clone(),
                concept_id: right_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: evidence_refs,
            });
            edges.push(meaning_edge(
                &format!("TA_RELATION_THEME_{ordinal:03}"),
                &relation_event_id,
                &left_node_id,
                GenerationMeaningRelationIR::Theme,
            ));
            edges.push(meaning_edge(
                &format!("TA_RELATION_GOAL_{ordinal:03}"),
                &relation_event_id,
                &right_node_id,
                GenerationMeaningRelationIR::Goal,
            ));
            if let Some(previous) = previous_event_id.as_deref() {
                edges.push(meaning_edge(
                    &format!("TA_RELATION_SEQUENCE_{ordinal:03}"),
                    previous,
                    &relation_event_id,
                    GenerationMeaningRelationIR::Sequence,
                ));
            }
            expressions.attach_alias(
                &format!("EXPR.{language:?}.TEMPORAL_LEFT_EVENT.{ordinal:03}"),
                language,
                &left_concept,
                left.surface.trim(),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:TEMPORAL_LEFT_EVENT",
            )?;
            expressions.attach_alias(
                &format!("EXPR.{language:?}.TEMPORAL_RIGHT_EVENT.{ordinal:03}"),
                language,
                &right_concept,
                right.surface.trim(),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:TEMPORAL_RIGHT_EVENT",
            )?;
            previous_event_id = Some(relation_event_id);
        }
    } else if !answer.event_evidence.is_empty() {
        for (index, event) in answer.event_evidence.iter().enumerate() {
            let ordinal = index + 1;
            let answer_event_id = format!("E_TEMPORAL_EVENT_RECORD_{ordinal:03}");
            let event_node_id = format!("R_TEMPORAL_EVENT_RECORD_{ordinal:03}");
            let event_concept = format!("C_RUNTIME_TEMPORAL_EVENT_RECORD_{ordinal:03}");
            let mut evidence_refs = base_refs.clone();
            evidence_refs.push(format!("TEMPORAL_EVENT_ID:{}", event.event_id));
            evidence_refs.push(format!("TEMPORAL_EVENT_WORLD:{:?}", event.modal_world));
            evidence_refs.sort();
            evidence_refs.dedup();
            nodes.push(GenerationMeaningNodeIR {
                node_id: answer_event_id.clone(),
                concept_id: "C_TEMPORAL_ANSWER_EVENT".to_string(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: evidence_refs.clone(),
            });
            nodes.push(GenerationMeaningNodeIR {
                node_id: event_node_id.clone(),
                concept_id: event_concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: evidence_refs,
            });
            edges.push(meaning_edge(
                &format!("TA_EVENT_THEME_{ordinal:03}"),
                &answer_event_id,
                &event_node_id,
                GenerationMeaningRelationIR::Theme,
            ));
            if let Some(previous) = previous_event_id.as_deref() {
                edges.push(meaning_edge(
                    &format!("TA_EVENT_SEQUENCE_{ordinal:03}"),
                    previous,
                    &answer_event_id,
                    GenerationMeaningRelationIR::Sequence,
                ));
            }
            expressions.attach_alias(
                &format!("EXPR.{language:?}.TEMPORAL_EVENT_RECORD.{ordinal:03}"),
                language,
                &event_concept,
                event.surface.trim(),
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:TEMPORAL_EVENT_RECORD",
            )?;
            previous_event_id = Some(answer_event_id);
        }
    }

    let terminal_id = "E_TEMPORAL_ANSWER_TERMINAL".to_string();
    nodes.push(GenerationMeaningNodeIR {
        node_id: terminal_id.clone(),
        concept_id: terminal_concept.to_string(),
        kind: GenerationMeaningNodeKindIR::Event,
        grounding_refs: base_refs.clone(),
    });
    if let Some(previous) = previous_event_id.as_deref() {
        edges.push(meaning_edge(
            "TA_SEQUENCE_TERMINAL",
            previous,
            &terminal_id,
            GenerationMeaningRelationIR::Sequence,
        ));
    }
    if answer.disposition == TemporalAnswerDispositionIR::AnsweredByTransitivePath {
        nodes.push(GenerationMeaningNodeIR {
            node_id: "Q_TEMPORAL_PATH_LENGTH".to_string(),
            concept_id: "C_RUNTIME_TEMPORAL_PATH_LENGTH".to_string(),
            kind: GenerationMeaningNodeKindIR::Quality,
            grounding_refs: base_refs,
        });
        edges.push(meaning_edge(
            "TA_TRANSITIVE_PATH_LENGTH",
            &terminal_id,
            "Q_TEMPORAL_PATH_LENGTH",
            GenerationMeaningRelationIR::Property,
        ));
        expressions.attach_alias(
            &format!("EXPR.{language:?}.TEMPORAL_PATH_LENGTH"),
            language,
            "C_RUNTIME_TEMPORAL_PATH_LENGTH",
            &answer.relation_evidence.len().to_string(),
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:TEMPORAL_PATH_LENGTH",
        )?;
    }

    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

fn dialogue_attribution_surface(
    language: LanguageCodeIR,
    query_kind: DiscourseQueryKindIR,
    evidence: &DiscourseAnswerEvidenceIR,
) -> String {
    if query_kind == DiscourseQueryKindIR::ModalStatus {
        return match language {
            LanguageCodeIR::Korean => format!(
                "{}의 {}",
                evidence.source_actor,
                korean_modal_classification(evidence.modal_world)
            ),
            _ => format!(
                "{}'s {}",
                evidence.source_actor,
                english_modal_classification(evidence.modal_world)
            ),
        };
    }
    match language {
        LanguageCodeIR::Korean => format!(
            "{}의 {}",
            evidence.source_actor,
            korean_epistemic_record(evidence.epistemic_status)
        ),
        _ => format!(
            "{}'s {}",
            evidence.source_actor,
            english_epistemic_record(evidence.epistemic_status)
        ),
    }
}

fn korean_epistemic_record(status: EpistemicStatusIR) -> &'static str {
    match status {
        EpistemicStatusIR::Reported | EpistemicStatusIR::Hearsay => "발화 기록",
        EpistemicStatusIR::Claimed => "주장",
        EpistemicStatusIR::Believed => "믿음",
        EpistemicStatusIR::PresentedAsKnown => "앎에 대한 진술",
        EpistemicStatusIR::Doubted => "의심",
        EpistemicStatusIR::Denied => "부인",
        EpistemicStatusIR::Observed => "관찰 보고",
        EpistemicStatusIR::Inferred => "추론",
        EpistemicStatusIR::Desired => "바람",
        EpistemicStatusIR::Expected => "예상",
        EpistemicStatusIR::Corrected => "정정",
    }
}

fn english_epistemic_record(status: EpistemicStatusIR) -> &'static str {
    match status {
        EpistemicStatusIR::Reported | EpistemicStatusIR::Hearsay => "statement",
        EpistemicStatusIR::Claimed => "claim",
        EpistemicStatusIR::Believed => "belief",
        EpistemicStatusIR::PresentedAsKnown => "knowledge claim",
        EpistemicStatusIR::Doubted => "doubt",
        EpistemicStatusIR::Denied => "denial",
        EpistemicStatusIR::Observed => "observation report",
        EpistemicStatusIR::Inferred => "inference",
        EpistemicStatusIR::Desired => "desire",
        EpistemicStatusIR::Expected => "prediction",
        EpistemicStatusIR::Corrected => "correction",
    }
}

fn korean_modal_classification(world: ModalWorldIR) -> &'static str {
    match world {
        ModalWorldIR::Actual => "실제 세계 진술",
        ModalWorldIR::EpistemicPossible => "가능성 진술",
        ModalWorldIR::EpistemicProbable => "개연성 진술",
        ModalWorldIR::EpistemicCertain => "확실성 진술",
        ModalWorldIR::Normative => "규범 진술",
        ModalWorldIR::Desired => "희망 진술",
        ModalWorldIR::Intended => "의도 진술",
        ModalWorldIR::Ability => "능력 진술",
        ModalWorldIR::Predicted => "예측 진술",
        ModalWorldIR::Hypothetical => "가정 진술",
        ModalWorldIR::Counterfactual => "반사실 진술",
        ModalWorldIR::Questioned => "의문 진술",
    }
}

fn english_modal_classification(world: ModalWorldIR) -> &'static str {
    match world {
        ModalWorldIR::Actual => "actual-world statement",
        ModalWorldIR::EpistemicPossible => "possibility statement",
        ModalWorldIR::EpistemicProbable => "probability statement",
        ModalWorldIR::EpistemicCertain => "certainty statement",
        ModalWorldIR::Normative => "normative statement",
        ModalWorldIR::Desired => "desire statement",
        ModalWorldIR::Intended => "intention statement",
        ModalWorldIR::Ability => "ability statement",
        ModalWorldIR::Predicted => "prediction",
        ModalWorldIR::Hypothetical => "hypothetical statement",
        ModalWorldIR::Counterfactual => "counterfactual statement",
        ModalWorldIR::Questioned => "questioned statement",
    }
}

pub(crate) fn generate_topic_transition_from_knowledge(
    language: LanguageCodeIR,
    transition: &TopicTransitionIR,
) -> Result<GenerativeLanguageIR, String> {
    if !transition.validate()
        || !transition.applied
        || transition.kind == TopicTransitionKindIR::Unresolved
    {
        return Err("topic transition must be valid, resolved, and applied".to_string());
    }
    let mut grounding_refs = transition.evidence.clone();
    grounding_refs.push(format!("TOPIC_TRANSITION:{}", transition.transition_sha256));
    grounding_refs.sort();
    grounding_refs.dedup();
    grounding_refs.truncate(32);
    let node = |node_id: &str, concept_id: &str, kind| GenerationMeaningNodeIR {
        node_id: node_id.to_string(),
        concept_id: concept_id.to_string(),
        kind,
        grounding_refs: grounding_refs.clone(),
    };
    let movement_concept = match transition.kind {
        TopicTransitionKindIR::ActivateNamed => "C_ACTIVATE_TOPIC",
        TopicTransitionKindIR::ActivateGroup => "C_ACTIVATE_TOPIC_GROUP",
        TopicTransitionKindIR::ReturnPrevious => "C_RETURN_TOPIC",
        TopicTransitionKindIR::Unresolved => unreachable!("rejected above"),
    };
    let return_style = transition.kind == TopicTransitionKindIR::ActivateNamed
        && transition
            .evidence
            .iter()
            .any(|evidence| evidence == "DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_RETURN");
    let mut nodes = vec![
        node("E_ACK", "C_ACKNOWLEDGE", GenerationMeaningNodeKindIR::Event),
        node(
            "E_MOVE",
            movement_concept,
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "E_BOUNDARY",
            "C_TOPIC_CHANGE_BOUNDARY",
            GenerationMeaningNodeKindIR::Event,
        ),
        node(
            "R_TOPIC",
            "C_RUNTIME_TOPIC",
            GenerationMeaningNodeKindIR::Entity,
        ),
        node(
            "Q_TOPIC_ONLY",
            "C_TOPIC_ONLY",
            GenerationMeaningNodeKindIR::Quality,
        ),
    ];
    let mut edges = vec![
        meaning_edge(
            "TT1",
            "E_ACK",
            "E_MOVE",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "TT2",
            "E_MOVE",
            "E_BOUNDARY",
            GenerationMeaningRelationIR::Sequence,
        ),
        meaning_edge(
            "TT3",
            "E_MOVE",
            "R_TOPIC",
            GenerationMeaningRelationIR::Goal,
        ),
        meaning_edge(
            "TT4",
            "E_BOUNDARY",
            "Q_TOPIC_ONLY",
            GenerationMeaningRelationIR::Property,
        ),
    ];
    if return_style {
        nodes.push(node(
            "Q_RETURN_STYLE",
            "C_TOPIC_RETURN_STYLE",
            GenerationMeaningNodeKindIR::Quality,
        ));
        edges.push(meaning_edge(
            "TT5",
            "E_MOVE",
            "Q_RETURN_STYLE",
            GenerationMeaningRelationIR::Property,
        ));
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let topic_surface = match transition.anchor_kind {
        DiscourseTopicAnchorKindIR::ActionGroup => match language {
            LanguageCodeIR::Korean => "작업 묶음".to_string(),
            _ => "task group".to_string(),
        },
        DiscourseTopicAnchorKindIR::AttributedPropositionGroup => match language {
            LanguageCodeIR::Korean => "화자 묶음".to_string(),
            _ => "speaker group".to_string(),
        },
        DiscourseTopicAnchorKindIR::Surface | DiscourseTopicAnchorKindIR::Concept => {
            localize_topic_surface(&transition.surface, language)
        }
    };
    let topic_expression = match language {
        LanguageCodeIR::Korean => format!("‘{}’", topic_surface.trim()),
        _ => topic_surface.trim().to_string(),
    };
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.RUNTIME_TOPIC",
            _ => "EXPR.EN.RUNTIME_TOPIC",
        },
        language,
        "C_RUNTIME_TOPIC",
        &topic_expression,
        ExpressionPartOfSpeechIR::Noun,
        "RUNTIME_REFERENT_SURFACE:TOPIC",
    )?;
    GenerativeLanguageCortex.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

fn localize_topic_surface(surface: &str, language: LanguageCodeIR) -> String {
    surface
        .split_whitespace()
        .map(|token| {
            let lower = token.to_lowercase();
            match language {
                LanguageCodeIR::Korean => match lower.as_str() {
                    "cache" => "캐시",
                    "queue" => "큐",
                    "backup" => "백업",
                    "log" => "로그",
                    "server" => "서버",
                    "worker" => "워커",
                    "index" => "인덱스",
                    "build" => "빌드",
                    "deployment" => "배포",
                    "project" => "프로젝트",
                    "report" => "보고서",
                    "file" => "파일",
                    "folder" => "폴더",
                    "repository" => "저장소",
                    _ => token,
                },
                _ => match token {
                    "캐시" => "cache",
                    "큐" => "queue",
                    "백업" => "backup",
                    "로그" => "log",
                    "서버" => "server",
                    "워커" => "worker",
                    "인덱스" => "index",
                    "빌드" => "build",
                    "배포" => "deployment",
                    "프로젝트" => "project",
                    "보고서" => "report",
                    "파일" => "file",
                    "폴더" => "folder",
                    "저장소" => "repository",
                    _ => token,
                },
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn meaning_edge(
    id: &str,
    source: &str,
    target: &str,
    relation: GenerationMeaningRelationIR,
) -> GenerationMeaningEdgeIR {
    GenerationMeaningEdgeIR {
        edge_id: id.to_string(),
        source_node_id: source.to_string(),
        target_node_id: target.to_string(),
        relation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safety_graph() -> GenerationMeaningGraphIR {
        GenerationMeaningGraphIR::new(
            vec![
                GenerationMeaningNodeIR {
                    node_id: "E_MOVE".to_string(),
                    concept_id: "C_MOVE".to_string(),
                    kind: GenerationMeaningNodeKindIR::Event,
                    grounding_refs: vec!["SAFETY_ASSESSMENT:1".to_string()],
                },
                GenerationMeaningNodeIR {
                    node_id: "R_VICTIM".to_string(),
                    concept_id: "C_ASSAULT_VICTIM".to_string(),
                    kind: GenerationMeaningNodeKindIR::Entity,
                    grounding_refs: vec!["PERSON_ROLE:VICTIM".to_string()],
                },
                GenerationMeaningNodeIR {
                    node_id: "R_SAFE_PLACE".to_string(),
                    concept_id: "C_SAFE_PLACE".to_string(),
                    kind: GenerationMeaningNodeKindIR::Entity,
                    grounding_refs: vec!["SAFETY_TARGET:PLACE".to_string()],
                },
            ],
            vec![
                meaning_edge(
                    "EDGE_AGENT",
                    "E_MOVE",
                    "R_VICTIM",
                    GenerationMeaningRelationIR::Agent,
                ),
                meaning_edge(
                    "EDGE_GOAL",
                    "E_MOVE",
                    "R_SAFE_PLACE",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        )
    }

    fn generate_safety(language: LanguageCodeIR) -> GenerativeLanguageIR {
        GenerativeLanguageCortex
            .generate(GenerativeLanguageRequestIR {
                meaning: safety_graph(),
                context: GenerationContextIR {
                    language,
                    register: LanguageRegisterIR::Neutral,
                    tense: GenerationTenseIR::Present,
                    emotion: GenerationEmotionIR::Concerned,
                    urgency_millis: 900,
                    default_speech_intent: GenerationSpeechIntentIR::Advise,
                },
                expressions: &ExpressionNodeStore::bilingual_builtin(),
            })
            .unwrap()
    }

    #[test]
    fn one_meaning_graph_selects_distinct_korean_and_english_phenotypes() {
        let korean = generate_safety(LanguageCodeIR::Korean);
        let english = generate_safety(LanguageCodeIR::English);
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert_ne!(
            korean.morphology.realized_text,
            english.morphology.realized_text
        );
        assert_eq!(
            korean.morphology.realized_text,
            "폭행 피해자는 안전한 곳으로 이동해야 해요."
        );
        assert_eq!(
            english.morphology.realized_text,
            "The assault victim should move to the safe place."
        );
        assert_eq!(korean.verification.unsupported_claims, 0);
        assert_eq!(english.verification.unsupported_claims, 0);
    }

    #[test]
    fn expression_aliases_do_not_change_semantic_hash_and_scores_are_explainable() {
        let graph = safety_graph();
        let semantic_hash = graph.semantic_sha256.clone();
        let mut expressions = ExpressionNodeStore::bilingual_builtin();
        expressions
            .attach_alias(
                "EXPR.KO.SAFE_PLACE.ALT",
                LanguageCodeIR::Korean,
                "C_SAFE_PLACE",
                "피신할 장소",
                ExpressionPartOfSpeechIR::Noun,
                "USER_ALIAS:1",
            )
            .unwrap();
        assert_eq!(semantic_hash, graph.semantic_sha256);
        let generated = GenerativeLanguageCortex
            .generate(GenerativeLanguageRequestIR {
                meaning: graph,
                context: GenerationContextIR {
                    language: LanguageCodeIR::Korean,
                    register: LanguageRegisterIR::Neutral,
                    tense: GenerationTenseIR::Present,
                    emotion: GenerationEmotionIR::Concerned,
                    urgency_millis: 900,
                    default_speech_intent: GenerationSpeechIntentIR::Advise,
                },
                expressions: &expressions,
            })
            .unwrap();
        assert!(generated
            .expression_selection
            .selections
            .iter()
            .all(|selection| {
                selection.score.activation_millis <= 1_000
                    && selection.score.confidence_millis <= 1_000
                    && selection.score.context_fit_millis <= 1_000
                    && !selection.score.reasons.is_empty()
            }));
    }

    #[test]
    fn completed_sentences_are_not_valid_expression_nodes() {
        let mut expressions = ExpressionNodeStore::bilingual_builtin();
        let result = expressions.attach_alias(
            "EXPR.EN.INVALID_SENTENCE",
            LanguageCodeIR::English,
            "C_SAFE_PLACE",
            "Move to a safe place.",
            ExpressionPartOfSpeechIR::Verb,
            "INVALID_TEST_FIXTURE",
        );
        assert_eq!(result, Err("INVALID_EXPRESSION_NODE".to_string()));
    }

    #[test]
    fn plan_preview_is_built_from_nodes_not_a_sentence_record() {
        let generated = generate_plan_preview_from_knowledge(
            LanguageCodeIR::English,
            "Aster cache",
            PlanIntentIR::Repair,
            "PLAN_TEST:ASTER",
        )
        .unwrap();
        assert!(generated.validate());
        assert!(generated.morphology.realized_text.contains("Aster cache"));
        assert!(generated.morphology.realized_text.contains("First"));
        assert!(generated.morphology.realized_text.contains("repair"));
        assert!(generated.morphology.realized_text.contains("verify"));
        assert!(!generated
            .morphology
            .realized_text
            .contains("selected action"));
        assert!(!generated
            .morphology
            .realized_text
            .contains("result verification"));
        assert_eq!(generated.verification.unsupported_claims, 0);
    }

    #[test]
    fn plan_boundaries_compose_from_typed_interpretations_and_prohibitions() {
        let refs = vec!["PLAN_BOUNDARY_TEST:1".to_string()];
        for kind in [
            GenerationPlanInterpretationKindIR::Suggestion,
            GenerationPlanInterpretationKindIR::ImplicitInvestigation,
            GenerationPlanInterpretationKindIR::ImplicitRepair,
            GenerationPlanInterpretationKindIR::ImplicitExplanation,
            GenerationPlanInterpretationKindIR::ImplicitPlanning,
            GenerationPlanInterpretationKindIR::SarcasmBoundary,
        ] {
            let korean = generate_plan_interpretation_from_knowledge(
                LanguageCodeIR::Korean,
                kind,
                "Aster 상태",
                None,
                &refs,
            )
            .unwrap();
            let english = generate_plan_interpretation_from_knowledge(
                LanguageCodeIR::English,
                kind,
                "Aster 상태",
                None,
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "kind={kind:?}");
            assert!(english.validate(), "kind={kind:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "kind={kind:?}"
            );
        }
        let korean_figurative = generate_plan_interpretation_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationPlanInterpretationKindIR::FigurativeBoundary,
            "불이 났어",
            Some("심각한 문제 상태"),
            &refs,
        )
        .unwrap();
        let english_figurative = generate_plan_interpretation_from_knowledge(
            LanguageCodeIR::English,
            GenerationPlanInterpretationKindIR::FigurativeBoundary,
            "불이 났어",
            Some("심각한 문제 상태"),
            &refs,
        )
        .unwrap();
        assert!(korean_figurative.validate());
        assert!(english_figurative.validate());
        assert_eq!(
            korean_figurative.meaning.semantic_sha256,
            english_figurative.meaning.semantic_sha256
        );
        assert!(korean_figurative
            .morphology
            .realized_text
            .contains("비유적 상태"));
        assert!(english_figurative
            .morphology
            .realized_text
            .contains("figurative state"));

        let korean_exclusion =
            generate_plan_exclusion_from_knowledge(LanguageCodeIR::Korean, "Nova 큐 삭제", &refs)
                .unwrap();
        let english_exclusion =
            generate_plan_exclusion_from_knowledge(LanguageCodeIR::English, "Nova 큐 삭제", &refs)
                .unwrap();
        assert!(korean_exclusion.validate());
        assert!(english_exclusion.validate());
        assert_eq!(
            korean_exclusion.meaning.semantic_sha256,
            english_exclusion.meaning.semantic_sha256
        );
        assert!(korean_exclusion
            .morphology
            .realized_text
            .contains("계획에서 제외"));
        assert!(english_exclusion
            .morphology
            .realized_text
            .contains("excluded"));
    }

    #[test]
    fn reported_content_is_remembered_without_becoming_a_fact_in_both_phenotypes() {
        let korean = generate_inform_acknowledgement_from_knowledge(
            LanguageCodeIR::Korean,
            "현재 CCTV 상태는 오프라인이야",
        )
        .unwrap();
        let english = generate_inform_acknowledgement_from_knowledge(
            LanguageCodeIR::English,
            "The current CCTV status is offline",
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        for fragment in ["CCTV", "말한 내용", "확인된 사실", "별도 증거"] {
            assert!(
                korean.morphology.realized_text.contains(fragment),
                "{fragment}: {}",
                korean.morphology.realized_text
            );
        }
        for fragment in ["CCTV", "You said", "confirmed fact", "separate evidence"] {
            assert!(
                english.morphology.realized_text.contains(fragment),
                "{fragment}: {}",
                english.morphology.realized_text
            );
        }
    }

    #[test]
    fn lifecycle_status_uses_one_semantic_claim_graph_for_both_phenotypes() {
        let claims = [
            GenerationLifecycleClaimIR::ActivePlan,
            GenerationLifecycleClaimIR::NoVerifiedExecutionOrResult,
        ];
        let korean = generate_lifecycle_status_from_knowledge(
            LanguageCodeIR::Korean,
            "백업",
            &claims,
            "ACTION_LIFECYCLE_SNAPSHOT:TEST",
        )
        .unwrap();
        let english = generate_lifecycle_status_from_knowledge(
            LanguageCodeIR::English,
            "backup",
            &claims,
            "ACTION_LIFECYCLE_SNAPSHOT:TEST",
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert!(korean.morphology.realized_text.contains("계획"));
        assert!(korean.morphology.realized_text.contains("실행 결과"));
        assert!(english.morphology.realized_text.contains("plan"));
        assert!(english
            .morphology
            .realized_text
            .contains("execution result"));
        assert!(english.morphology.realized_text.contains("not"));
        assert_ne!(
            korean.morphology.realized_text,
            english.morphology.realized_text
        );
    }

    #[test]
    fn action_set_answer_composes_quantifier_truth_and_evidence_boundary() {
        let korean = generate_action_set_answer_from_knowledge(
            LanguageCodeIR::Korean,
            3,
            GenerationActionSetQuantifierIR::All,
            GenerationActionSetPredicateIR::ActivePlan,
            GenerationActionSetTruthIR::True,
            "ACTION_SET_QUERY:TEST",
        )
        .unwrap();
        let english = generate_action_set_answer_from_knowledge(
            LanguageCodeIR::English,
            3,
            GenerationActionSetQuantifierIR::All,
            GenerationActionSetPredicateIR::ActivePlan,
            GenerationActionSetTruthIR::True,
            "ACTION_SET_QUERY:TEST",
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert!(korean.morphology.realized_text.contains("3개 작업 모두"));
        assert!(korean.morphology.realized_text.contains("분리"));
        assert!(english
            .morphology
            .realized_text
            .contains("all 3 selected actions"));
        assert!(english.morphology.realized_text.contains("separate"));
    }

    #[test]
    fn affect_support_composes_state_and_invitation_without_a_sentence_record() {
        let korean = generate_affect_support_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationAffectKindIR::Frustrated,
        )
        .unwrap();
        let english = generate_affect_support_from_knowledge(
            LanguageCodeIR::English,
            GenerationAffectKindIR::Frustrated,
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert_eq!(
            korean.morphology.realized_text,
            "계속 반복되는 일은 답답할 만해. 가장 최근 실패를 확인해 보자."
        );
        assert_eq!(
            english.morphology.realized_text,
            "That is frustrating. We can check the most recent failure."
        );
    }

    #[test]
    fn dialogue_management_composes_bilingual_social_and_floor_responses() {
        for response in [
            GenerationDialogueResponseKindIR::HoldFloor,
            GenerationDialogueResponseKindIR::Greeting,
            GenerationDialogueResponseKindIR::Gratitude,
            GenerationDialogueResponseKindIR::Farewell,
            GenerationDialogueResponseKindIR::Backchannel,
        ] {
            let korean =
                generate_dialogue_response_from_knowledge(LanguageCodeIR::Korean, response)
                    .unwrap();
            let english =
                generate_dialogue_response_from_knowledge(LanguageCodeIR::English, response)
                    .unwrap();
            assert!(korean.validate(), "response={response:?}");
            assert!(english.validate(), "response={response:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "response={response:?}"
            );
            assert_ne!(
                korean.morphology.realized_text, english.morphology.realized_text,
                "response={response:?}"
            );
        }
        let hold = generate_dialogue_response_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationDialogueResponseKindIR::HoldFloor,
        )
        .unwrap();
        assert!(hold.morphology.realized_text.contains("천천히"));
        assert!(hold.morphology.realized_text.contains("듣고 있어"));
        let greeting = generate_dialogue_response_from_knowledge(
            LanguageCodeIR::English,
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        assert!(greeting
            .morphology
            .realized_text
            .contains("What can I help"));
    }

    #[test]
    fn continuation_gate_composes_task_benefit_and_three_typed_branches() {
        let refs = vec!["CLAUSE:GATE-1".to_string()];
        let korean = generate_continuation_gate_from_knowledge(
            LanguageCodeIR::Korean,
            "통합",
            "커버리지 확장",
            &refs,
        )
        .unwrap();
        let english = generate_continuation_gate_from_knowledge(
            LanguageCodeIR::English,
            "통합",
            "커버리지 확장",
            &refs,
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert!(korean.morphology.realized_text.contains("확인되면 계속"));
        assert!(korean.morphology.realized_text.contains("멈출지 물을게"));
        assert!(korean.morphology.realized_text.contains("추측하지 않을게"));
        assert!(english.morphology.realized_text.contains("If"));
        assert!(english
            .morphology
            .realized_text
            .contains("ask whether to stop"));
        assert!(english
            .morphology
            .realized_text
            .contains("instead of guessing"));
    }

    #[test]
    fn continuation_gate_followups_preserve_pending_and_proxy_boundaries() {
        let refs = vec!["PENDING_GATE:TURN-1".to_string()];
        for followup in [
            GenerationContinuationGateFollowupIR::PendingDecision,
            GenerationContinuationGateFollowupIR::ProxyEvidence,
        ] {
            let korean = generate_continuation_gate_followup_from_knowledge(
                LanguageCodeIR::Korean,
                "통합",
                "커버리지 확장",
                &refs,
                followup,
            )
            .unwrap();
            let english = generate_continuation_gate_followup_from_knowledge(
                LanguageCodeIR::English,
                "통합",
                "커버리지 확장",
                &refs,
                followup,
            )
            .unwrap();
            assert!(korean.validate(), "followup={followup:?}");
            assert!(english.validate(), "followup={followup:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "followup={followup:?}"
            );
        }
        let pending = generate_continuation_gate_followup_from_knowledge(
            LanguageCodeIR::Korean,
            "통합",
            "커버리지 확장",
            &refs,
            GenerationContinuationGateFollowupIR::PendingDecision,
        )
        .unwrap();
        assert!(pending
            .morphology
            .realized_text
            .contains("직접 확인하지 못했"));
        assert!(pending.morphology.realized_text.contains("대리 지표만으로"));
        assert!(pending.morphology.realized_text.contains("중단 여부"));
        let proxy = generate_continuation_gate_followup_from_knowledge(
            LanguageCodeIR::English,
            "integration",
            "coverage expansion",
            &refs,
            GenerationContinuationGateFollowupIR::ProxyEvidence,
        )
        .unwrap();
        assert!(proxy
            .morphology
            .realized_text
            .contains("recorded the proxy change"));
        assert!(proxy.morphology.realized_text.contains("does not verify"));
    }

    #[test]
    fn user_feedback_composes_six_kinds_from_shared_meaning() {
        let refs = vec!["CLAUSE:FEEDBACK-1".to_string()];
        for feedback in [
            GenerationUserFeedbackKindIR::Unhelpful,
            GenerationUserFeedbackKindIR::Misunderstood,
            GenerationUserFeedbackKindIR::MissedPoint,
            GenerationUserFeedbackKindIR::TooVerbose,
            GenerationUserFeedbackKindIR::TooBrief,
            GenerationUserFeedbackKindIR::Incorrect,
        ] {
            let korean = generate_user_feedback_from_knowledge(
                LanguageCodeIR::Korean,
                feedback,
                "answer",
                &refs,
            )
            .unwrap();
            let english = generate_user_feedback_from_knowledge(
                LanguageCodeIR::English,
                feedback,
                "answer",
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "feedback={feedback:?}");
            assert!(english.validate(), "feedback={feedback:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "feedback={feedback:?}"
            );
        }
        let unhelpful = generate_user_feedback_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationUserFeedbackKindIR::Unhelpful,
            "answer",
            &refs,
        )
        .unwrap();
        assert!(unhelpful
            .morphology
            .realized_text
            .contains("도움이 되지 않았"));
        assert!(unhelpful.morphology.realized_text.contains("어긋난 부분"));
        let missed = generate_user_feedback_from_knowledge(
            LanguageCodeIR::English,
            GenerationUserFeedbackKindIR::MissedPoint,
            "answer",
            &refs,
        )
        .unwrap();
        assert!(missed
            .morphology
            .realized_text
            .contains("missed your point"));
        assert!(missed.morphology.realized_text.contains("correct"));
    }

    #[test]
    fn discourse_group_updates_compose_operation_and_member_state() {
        let refs = vec!["DISCOURSE_GROUP_UPDATE:REVISION-2".to_string()];
        for operation in [
            GenerationDiscourseGroupUpdateKindIR::AddMember,
            GenerationDiscourseGroupUpdateKindIR::RemoveMember,
            GenerationDiscourseGroupUpdateKindIR::MergeGroups,
        ] {
            let korean = generate_discourse_group_update_from_knowledge(
                LanguageCodeIR::Korean,
                operation,
                3,
                &refs,
            )
            .unwrap();
            let english = generate_discourse_group_update_from_knowledge(
                LanguageCodeIR::English,
                operation,
                3,
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "operation={operation:?}");
            assert!(english.validate(), "operation={operation:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "operation={operation:?}"
            );
            assert!(korean.morphology.realized_text.contains("3개 대상"));
            assert!(english.morphology.realized_text.contains("3 members"));
        }
    }

    #[test]
    fn clarification_kinds_share_typed_meaning_and_never_gain_language_authority() {
        let refs = vec!["CLARIFICATION_TEST:BLIND_SHAPE".to_string()];
        for kind in [
            GenerationClarificationKindIR::PendingChoice,
            GenerationClarificationKindIR::OrderedPair,
            GenerationClarificationKindIR::LocalOrdinal,
            GenerationClarificationKindIR::EventOrdinal,
            GenerationClarificationKindIR::PreviousTopic,
            GenerationClarificationKindIR::CompetingRequest,
            GenerationClarificationKindIR::NonliteralReading,
            GenerationClarificationKindIR::VoiceAlternative,
            GenerationClarificationKindIR::Reference,
            GenerationClarificationKindIR::MissingDetails,
        ] {
            let detail = matches!(
                kind,
                GenerationClarificationKindIR::CompetingRequest
                    | GenerationClarificationKindIR::NonliteralReading
                    | GenerationClarificationKindIR::VoiceAlternative
                    | GenerationClarificationKindIR::Reference
            )
            .then_some("unseen referent surface");
            let korean =
                generate_clarification_from_knowledge(LanguageCodeIR::Korean, kind, detail, &refs)
                    .unwrap();
            let english =
                generate_clarification_from_knowledge(LanguageCodeIR::English, kind, detail, &refs)
                    .unwrap();
            assert!(korean.validate(), "kind={kind:?}");
            assert!(english.validate(), "kind={kind:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "kind={kind:?}"
            );
            assert!(
                korean
                    .speech_intent
                    .intents
                    .iter()
                    .all(|intent| intent.intent == GenerationSpeechIntentIR::Ask),
                "kind={kind:?}"
            );
            assert_eq!(korean.verification.unsupported_claims, 0);
            assert_eq!(english.verification.unsupported_claims, 0);
            assert!(!korean.semantic_authority);
            assert!(!english.language_can_execute);
        }
    }

    #[test]
    fn unbound_reference_clarification_is_composed_from_question_roles() {
        let refs = vec!["REFERENCE_RESOLUTION:UNBOUND_DEMONSTRATIVE".to_string()];
        let korean = generate_clarification_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationClarificationKindIR::Reference,
            None,
            &refs,
        )
        .unwrap();
        let english = generate_clarification_from_knowledge(
            LanguageCodeIR::English,
            GenerationClarificationKindIR::Reference,
            None,
            &refs,
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert_eq!(
            korean.morphology.realized_text,
            "‘그거’가 어느 대상을 가리키는지 알려줘. 대상을 하나만 지정해줘."
        );
        assert_eq!(
            english.morphology.realized_text,
            "Which target does “that” refer to? Please name one target."
        );
    }

    #[test]
    fn interaction_boundary_generation_covers_every_consuming_illocutionary_force() {
        let graph = |force| {
            let (actor, addressee, activation) = match force {
                IllocutionaryForceIR::ReportedCommitment => (
                    crate::pragmatics::DialogueParticipantIR::ThirdParty,
                    crate::pragmatics::DialogueParticipantIR::Unknown,
                    CommitmentActivationIR::Inactive,
                ),
                IllocutionaryForceIR::CapabilityQuestion => (
                    crate::pragmatics::DialogueParticipantIR::User,
                    crate::pragmatics::DialogueParticipantIR::System,
                    CommitmentActivationIR::Inactive,
                ),
                IllocutionaryForceIR::DeferredConditionalRequest => (
                    crate::pragmatics::DialogueParticipantIR::User,
                    crate::pragmatics::DialogueParticipantIR::Assistant,
                    CommitmentActivationIR::ConditionPending,
                ),
                IllocutionaryForceIR::GoalWithdrawal
                | IllocutionaryForceIR::OutcomeClaimConstraint => (
                    crate::pragmatics::DialogueParticipantIR::User,
                    crate::pragmatics::DialogueParticipantIR::Assistant,
                    CommitmentActivationIR::Immediate,
                ),
                _ => (
                    crate::pragmatics::DialogueParticipantIR::User,
                    crate::pragmatics::DialogueParticipantIR::Assistant,
                    CommitmentActivationIR::Inactive,
                ),
            };
            IllocutionaryCommitmentGraphIR {
                commitments: vec![crate::pragmatics::IllocutionaryCommitmentIR {
                    commitment_id: "ILLOCUTION-TEST-01".to_string(),
                    actor,
                    addressee,
                    force,
                    activation,
                    proposition_surface: "I will repair the parser myself.".to_string(),
                    external_execution_authorized: false,
                    evidence: vec!["TYPED_TEST_EVIDENCE".to_string()],
                }],
                goal_withdrawal: (force == IllocutionaryForceIR::GoalWithdrawal).then(|| {
                    crate::pragmatics::GoalWithdrawalIR {
                        scope: GoalWithdrawalScopeIR::AllActiveGoals,
                        event_ordinal: None,
                        evidence_surface: "Cancel that work.".to_string(),
                    }
                }),
                outcome_claim_policy: (force == IllocutionaryForceIR::OutcomeClaimConstraint).then(
                    || crate::pragmatics::OutcomeClaimPolicyIR {
                        policy: "VERIFIED_OUTCOME_ONLY".to_string(),
                        verified_outcome_only: true,
                        required_evidence: vec![
                            crate::pragmatics::RequiredOutcomeEvidenceIR::DirectVerification,
                        ],
                        evidence_surface: "Do not claim completion without proof.".to_string(),
                    },
                ),
            }
        };
        let fixtures = [
            (
                IllocutionaryForceIR::SelfCommitment,
                "C_INTERACTION_SELF_COMMITMENT",
                "직접 하겠다는 약속",
                "your own commitment",
            ),
            (
                IllocutionaryForceIR::ReportedCommitment,
                "C_INTERACTION_REPORTED_COMMITMENT",
                "제3자의 향후 약속",
                "third party's future commitment",
            ),
            (
                IllocutionaryForceIR::CapabilityQuestion,
                "C_INTERACTION_CAPABILITY_QUESTION",
                "기능 지원 여부",
                "capability question",
            ),
            (
                IllocutionaryForceIR::DeferredConditionalRequest,
                "C_INTERACTION_DEFERRED_REQUEST",
                "조건 대기 상태",
                "condition-pending request",
            ),
            (
                IllocutionaryForceIR::GoalWithdrawal,
                "C_INTERACTION_GOAL_WITHDRAWAL",
                "활성 작업 1개",
                "1 active task(s)",
            ),
            (
                IllocutionaryForceIR::OutcomeClaimConstraint,
                "C_INTERACTION_OUTCOME_POLICY",
                "직접 검증",
                "direct verification",
            ),
        ];
        for (force, concept_id, korean_fragment, english_fragment) in fixtures {
            let graph = graph(force);
            let withdrawn = if force == IllocutionaryForceIR::GoalWithdrawal {
                vec!["GOAL-TEST-01".to_string()]
            } else {
                Vec::new()
            };
            let refs = vec![format!("INTERACTION_TEST:{force:?}")];
            let korean = generate_interaction_boundary_from_knowledge(
                LanguageCodeIR::Korean,
                &graph,
                &withdrawn,
                &[],
                &refs,
            )
            .unwrap();
            let english = generate_interaction_boundary_from_knowledge(
                LanguageCodeIR::English,
                &graph,
                &withdrawn,
                &[],
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "force={force:?}");
            assert!(english.validate(), "force={force:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "force={force:?}"
            );
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept_id));
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| { node.concept_id == "C_INTERACTION_NO_AUTHORITY" }));
            assert!(korean.morphology.realized_text.contains(korean_fragment));
            assert!(english.morphology.realized_text.contains(english_fragment));
            assert!(!korean.semantic_authority);
            assert!(!english.language_can_execute);
            assert_eq!(korean.verification.unsupported_claims, 0);
            assert_eq!(english.verification.unsupported_claims, 0);
        }
    }

    #[test]
    fn conditional_guard_generation_covers_every_status_without_reverse_inference() {
        let evidence = |polarity| crate::conditional_guard::GuardEvidenceIR {
            belief_id: format!("BELIEF-{polarity:?}"),
            proposition_surface: "The tests passed.".to_string(),
            source_actor: "USER".to_string(),
            polarity,
            introduced_turn: 2,
            modal_world: ModalWorldIR::Actual,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        };
        let evaluation = |status, evidence: Vec<crate::conditional_guard::GuardEvidenceIR>| {
            ConditionalGuardEvaluationIR {
                schema: CONDITIONAL_GUARD_EVALUATION_SCHEMA.to_string(),
                guard_id: format!("GUARD-{status:?}"),
                status,
                antecedent_surface: "the tests pass".to_string(),
                consequent_surface: "deploy the service".to_string(),
                evidence,
                evaluation_turn: 3,
                deliberation_eligible: status == GuardStatusIR::SupportedByDialogueEvidence,
                status_changed: true,
                realized_text: "legacy text must not be reused".to_string(),
                unsupported_claims: 0,
                dialogue_truth_established: false,
                reverse_inference_authorized: false,
                external_execution_authorized: false,
            }
        };
        let fixtures = [
            (
                evaluation(GuardStatusIR::Unresolved, Vec::new()),
                "C_GUARD_UNRESOLVED",
                "확인되지 않았어",
                "not yet established",
            ),
            (
                evaluation(
                    GuardStatusIR::SupportedByDialogueEvidence,
                    vec![evidence(GuardEvidencePolarityIR::Supports)],
                ),
                "C_GUARD_SUPPORTED",
                "뒷받침해",
                "supports",
            ),
            (
                evaluation(
                    GuardStatusIR::ContradictedByDialogueEvidence,
                    vec![evidence(GuardEvidencePolarityIR::Contradicts)],
                ),
                "C_GUARD_CONTRADICTED",
                "어긋나",
                "contradicts",
            ),
            (
                evaluation(
                    GuardStatusIR::Contested,
                    vec![
                        evidence(GuardEvidencePolarityIR::Supports),
                        evidence(GuardEvidencePolarityIR::Contradicts),
                    ],
                ),
                "C_GUARD_CONTESTED",
                "엇갈려",
                "conflicts over",
            ),
            (
                evaluation(GuardStatusIR::IneligibleCounterfactual, Vec::new()),
                "C_GUARD_COUNTERFACTUAL",
                "반사실 조건",
                "counterfactual",
            ),
        ];
        for (evaluation, concept_id, korean_fragment, english_fragment) in fixtures {
            let refs = vec![format!("GUARD_TEST:{concept_id}")];
            let korean = generate_conditional_guard_from_knowledge(
                LanguageCodeIR::Korean,
                &evaluation,
                &refs,
            )
            .unwrap();
            let english = generate_conditional_guard_from_knowledge(
                LanguageCodeIR::English,
                &evaluation,
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "concept={concept_id}");
            assert!(english.validate(), "concept={concept_id}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "concept={concept_id}"
            );
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept_id));
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| { node.concept_id == "C_GUARD_NO_REVERSE_INFERENCE" }));
            assert!(korean.morphology.realized_text.contains(korean_fragment));
            assert!(english.morphology.realized_text.contains(english_fragment));
            assert!(!korean
                .morphology
                .realized_text
                .contains("legacy text must not be reused"));
            assert!(!korean.semantic_authority);
            assert!(!english.language_can_execute);
            assert_eq!(korean.verification.unsupported_claims, 0);
            assert_eq!(english.verification.unsupported_claims, 0);
        }
    }

    #[test]
    fn definition_grounding_generation_covers_every_typed_disposition_bilingually() {
        let grounder = crate::definition_grounding::DefinitionGrounder;
        let added = grounder.ground("\"quorin\" means inspect.", 1, &[]);
        let lexeme = added
            .binding
            .as_ref()
            .expect("new binding")
            .predicate_lexeme();
        let fixtures = vec![
            (added, "C_DEFINITION_BIND_ADDED", "검사", "inspect"),
            (
                grounder.ground(
                    "\"quorin\" means inspect.",
                    2,
                    std::slice::from_ref(&lexeme),
                ),
                "C_DEFINITION_BIND_CONFIRMED",
                "같은 어휘 관계",
                "confirmed the lexical link",
            ),
            (
                grounder.ground("\"quorin\" means delete.", 2, std::slice::from_ref(&lexeme)),
                "C_DEFINITION_REJECT_CONFLICT",
                "재정의를 거부",
                "rejected the redefinition",
            ),
            (
                grounder.ground("\"sovel\" means delete?", 1, &[]),
                "C_DEFINITION_REJECT_NONASSERTED",
                "확정한 정의",
                "asserted definition",
            ),
            (
                grounder.ground("\"brika\" means inspect or repair.", 1, &[]),
                "C_DEFINITION_REJECT_AMBIGUOUS",
                "여러 의미",
                "multiple semantic operators",
            ),
            (
                grounder.ground("\"tremi\" means frobnicate.", 1, &[]),
                "C_DEFINITION_REJECT_UNRESOLVED",
                "찾지 못해",
                "could not ground",
            ),
            (
                grounder.ground("\"bad alias!\" means inspect.", 1, &[]),
                "C_DEFINITION_REJECT_INVALID_ALIAS",
                "유효하지 않아",
                "alias form is invalid",
            ),
        ];
        for (grounding, concept_id, korean_fragment, english_fragment) in fixtures {
            assert!(grounding.validate(), "concept={concept_id}");
            let refs = vec![format!("DEFINITION_TEST:{concept_id}")];
            let korean = generate_definition_grounding_from_knowledge(
                LanguageCodeIR::Korean,
                &grounding,
                &refs,
            )
            .unwrap();
            let english = generate_definition_grounding_from_knowledge(
                LanguageCodeIR::English,
                &grounding,
                &refs,
            )
            .unwrap();
            assert!(korean.validate(), "concept={concept_id}");
            assert!(english.validate(), "concept={concept_id}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "concept={concept_id}"
            );
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept_id));
            assert!(korean.morphology.realized_text.contains(korean_fragment));
            assert!(english.morphology.realized_text.contains(english_fragment));
            assert!(!korean.semantic_authority);
            assert!(!english.language_can_execute);
            assert_eq!(korean.verification.unsupported_claims, 0);
            assert_eq!(english.verification.unsupported_claims, 0);
        }
        assert!(generate_definition_grounding_from_knowledge(
            LanguageCodeIR::English,
            &DefinitionGroundingIR::no_definition(),
            &[],
        )
        .is_err());
    }

    fn dialogue_relation_warning_fixture() -> DialogueRelationAnswerIR {
        DialogueRelationAnswerIR {
            schema: crate::discourse_relations::DIALOGUE_RELATION_ANSWER_SCHEMA.to_string(),
            query: crate::discourse_relations::DialogueRelationQueryIR {
                original_text: "Why did the queue grow?".to_string(),
                kind: DialogueRelationQueryKindIR::CauseOf,
                topic_terms: vec!["queue".to_string(), "grow".to_string()],
            },
            disposition: DialogueRelationAnswerDispositionIR::AnsweredFromDialoguePath,
            evidence: vec![crate::discourse_relations::DialogueRelationEvidenceIR {
                relation_id: "DREL-WARNING-01".to_string(),
                kind: DialogueRelationKindIR::Cause,
                source_belief_id: "BELIEF-SOURCE".to_string(),
                target_belief_id: "BELIEF-TARGET".to_string(),
                source_belief_status: crate::epistemic::BeliefRecordStatusIR::Contested,
                target_belief_status: crate::epistemic::BeliefRecordStatusIR::Active,
                source_modal_world: ModalWorldIR::EpistemicPossible,
                target_modal_world: ModalWorldIR::Actual,
                source_polarity: crate::attribution::AttributedPropositionPolarityIR::Positive,
                target_polarity: crate::attribution::AttributedPropositionPolarityIR::Positive,
                source_summary: "the gateway might fail".to_string(),
                target_summary: "the queue grew".to_string(),
                source_turn: 1,
                target_turn: 2,
                dialogue_claim_only: true,
                causal_truth_established: false,
                semantic_authority: false,
                external_execution_authorized: false,
            }],
            paths: vec![crate::discourse_relations::DialogueRelationPathIR {
                path_id: "DREL-PATH-01".to_string(),
                relation_ids: vec!["DREL-WARNING-01".to_string()],
                root_referent_id: "REF-SOURCE".to_string(),
                terminal_referent_id: "REF-TARGET".to_string(),
                root_summary: "the gateway might fail".to_string(),
                terminal_summary: "the queue grew".to_string(),
                hop_count: 1,
                confidence_millis: 700,
                contains_nonactual_world: true,
                contains_contested_endpoint: true,
                truncated_by_hop_limit: true,
                dialogue_claim_only: true,
                causal_truth_established: false,
                semantic_authority: false,
                external_execution_authorized: false,
            }],
            language: LanguageCodeIR::English,
            realized_text: "legacy fixture must not be copied".to_string(),
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        }
    }

    #[test]
    fn dialogue_relation_generation_preserves_typed_safety_warnings_bilingually() {
        let answer = dialogue_relation_warning_fixture();
        assert!(answer.validate());
        let refs = vec!["DIALOGUE_RELATION_TEST:TYPED_WARNINGS".to_string()];
        let korean = generate_dialogue_relation_answer_from_knowledge(
            LanguageCodeIR::Korean,
            &answer,
            &refs,
        )
        .unwrap();
        let english = generate_dialogue_relation_answer_from_knowledge(
            LanguageCodeIR::English,
            &answer,
            &refs,
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        for concept_id in [
            "C_DIALOGUE_RELATION_NONACTUAL_WARNING",
            "C_DIALOGUE_RELATION_CONTESTED_WARNING",
            "C_DIALOGUE_RELATION_TRUNCATED_WARNING",
        ] {
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept_id));
        }
        assert!(korean.morphology.realized_text.contains("실제 사건 경로"));
        assert!(english
            .morphology
            .realized_text
            .contains("not an actual-event path"));
        assert!(!korean.semantic_authority);
        assert!(!english.language_can_execute);
        assert_eq!(korean.verification.unsupported_claims, 0);
        assert_eq!(english.verification.unsupported_claims, 0);
    }

    fn temporal_relation_fixture(kind: TemporalRelationKindIR) -> TemporalAnswerIR {
        let left = crate::temporal::TemporalEventIR {
            event_id: "TEMP-EVENT-LEFT".to_string(),
            surface: "the lapis scan ran".to_string(),
            normalized_key: "lapis scan run".to_string(),
            event_time: None,
            report_turn: 1,
            modal_world: ModalWorldIR::Actual,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        };
        let right = crate::temporal::TemporalEventIR {
            event_id: "TEMP-EVENT-RIGHT".to_string(),
            surface: "the pearl deploy ran".to_string(),
            normalized_key: "pearl deploy run".to_string(),
            event_time: None,
            report_turn: 1,
            modal_world: ModalWorldIR::Actual,
            dialogue_truth_established: false,
            external_execution_authorized: false,
        };
        TemporalAnswerIR {
            schema: crate::temporal::TEMPORAL_ANSWER_SCHEMA.to_string(),
            query: crate::temporal::TemporalQueryIR {
                schema: crate::temporal::TEMPORAL_QUERY_SCHEMA.to_string(),
                original_text: "typed relation fixture".to_string(),
                kind: TemporalQueryKindIR::RelationCheck,
                target_terms: vec!["lapis".to_string()],
                second_target_terms: vec!["pearl".to_string()],
                expected_relation: Some(kind),
                confidence_millis: 1_000,
            },
            disposition: TemporalAnswerDispositionIR::AnsweredFromTemporalGraph,
            event_evidence: vec![left, right],
            relation_evidence: vec![crate::temporal::TemporalRelationIR {
                relation_id: "TEMP-REL-FIXTURE".to_string(),
                left_event_id: "TEMP-EVENT-LEFT".to_string(),
                right_event_id: "TEMP-EVENT-RIGHT".to_string(),
                kind,
                status: crate::temporal::TemporalRelationStatusIR::Active,
                evidence_surface: "typed relation fixture".to_string(),
                introduced_turn: 1,
                dialogue_truth_established: false,
                external_execution_authorized: false,
            }],
            language: LanguageCodeIR::English,
            realized_text: "typed temporal fixture".to_string(),
            dialogue_truth_established: false,
            external_execution_authorized: false,
            unsupported_claims: 0,
        }
    }

    #[test]
    fn temporal_generation_supports_during_and_simultaneous_relations_bilingually() {
        for (kind, concept, korean_fragment, english_fragment) in [
            (
                TemporalRelationKindIR::During,
                "C_TEMPORAL_ANSWER_DURING",
                "동안",
                "occurs during",
            ),
            (
                TemporalRelationKindIR::Simultaneous,
                "C_TEMPORAL_ANSWER_SIMULTANEOUS",
                "같은 시점",
                "is simultaneous with",
            ),
        ] {
            let answer = temporal_relation_fixture(kind);
            let refs = vec!["TEMPORAL_TEST:TYPED_RELATION".to_string()];
            let korean =
                generate_temporal_answer_from_knowledge(LanguageCodeIR::Korean, &answer, &refs)
                    .unwrap();
            let english =
                generate_temporal_answer_from_knowledge(LanguageCodeIR::English, &answer, &refs)
                    .unwrap();
            assert!(korean.validate(), "kind={kind:?}");
            assert!(english.validate(), "kind={kind:?}");
            assert_eq!(
                korean.meaning.semantic_sha256, english.meaning.semantic_sha256,
                "kind={kind:?}"
            );
            assert!(korean
                .meaning
                .nodes
                .iter()
                .any(|node| node.concept_id == concept));
            assert!(korean.morphology.realized_text.contains(korean_fragment));
            assert!(english.morphology.realized_text.contains(english_fragment));
            assert_eq!(korean.verification.unsupported_claims, 0);
            assert_eq!(english.verification.unsupported_claims, 0);
        }
    }

    #[test]
    fn missing_event_time_never_substitutes_dialogue_turn_order() {
        let mut answer = temporal_relation_fixture(TemporalRelationKindIR::Before);
        answer.query.kind = TemporalQueryKindIR::EventTime;
        answer.query.expected_relation = None;
        answer.query.second_target_terms.clear();
        answer.disposition = TemporalAnswerDispositionIR::EventTimeNotRecorded;
        answer.event_evidence.truncate(1);
        answer.relation_evidence.clear();
        assert!(answer.validate());
        let refs = vec!["TEMPORAL_TEST:MISSING_EVENT_TIME".to_string()];
        let korean =
            generate_temporal_answer_from_knowledge(LanguageCodeIR::Korean, &answer, &refs)
                .unwrap();
        let english =
            generate_temporal_answer_from_knowledge(LanguageCodeIR::English, &answer, &refs)
                .unwrap();
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert!(korean
            .meaning
            .nodes
            .iter()
            .any(|node| node.concept_id == "C_TEMPORAL_ANSWER_TIME_MISSING"));
        assert!(korean.morphology.realized_text.contains("대화 차례"));
        assert!(english
            .morphology
            .realized_text
            .contains("dialogue turn order"));
        assert!(!korean.semantic_authority);
        assert!(!english.language_can_execute);
    }

    #[test]
    fn topic_transition_composes_topic_motion_and_non_execution_boundary() {
        let transition = crate::conversation::detect_topic_transition("서버 이야기로 돌아가자")
            .expect("typed topic transition");
        let korean =
            generate_topic_transition_from_knowledge(LanguageCodeIR::Korean, &transition).unwrap();
        let english =
            generate_topic_transition_from_knowledge(LanguageCodeIR::English, &transition).unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        assert_eq!(
            korean.morphology.realized_text,
            "알겠어. ‘서버’ 이야기로 돌아가자. 이제 그 이야기가 현재 화제야. 이건 대화 초점만 바꾸는 거야. 작업을 실행한 것은 아니야."
        );
        assert_eq!(
            english.morphology.realized_text,
            "Got it. Let's return to the server topic. It is now the active topic. This only changes the conversation focus; it does not execute any work."
        );
    }
}
