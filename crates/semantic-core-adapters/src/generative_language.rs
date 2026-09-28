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

#[path = "event_summary.rs"]
mod event_summary;
#[path = "world_realization.rs"]
mod world_realization;
pub(crate) use event_summary::generate_event_summary;
pub use event_summary::EventSummaryIR;
pub(crate) use world_realization::generate_decision_inquiry;
pub(crate) use world_realization::generate_world_clarification;
pub(crate) use world_realization::generate_world_decision;
pub(crate) use world_realization::generate_world_memory_update;
pub(crate) use world_realization::{
    world_decision_language_available, world_update_language_available,
};

pub const GENERATION_MEANING_SCHEMA: &str = "B_CORE_GENERATION_MEANING_IR_2";
pub const GENERATIVE_LANGUAGE_SCHEMA: &str = "B_CORE_GENERATIVE_LANGUAGE_IR_31";

const KOREAN_ACKNOWLEDGEMENT_STEM: &str = "알겠";

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
    /// A mentioned action, not an assertion or instruction to perform it.
    EventReference,
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
    /// Describe an action inside a proposed plan; not a speaker execution promise.
    DescribePlan,
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
    /// The source-attributed event supplies its finite verb form; generation
    /// must not reinterpret it as present or future.
    SourcePreserved,
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
    ResponsePreference,
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
            Self::ResponsePreference => "C_NAME_TARGET",
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
    ExternalExecutionUnavailable,
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
            Self::ExternalExecutionUnavailable => "C_CONVERSATION_EXTERNAL_EXECUTION_UNAVAILABLE",
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
    KoreanDigeutIrregular,
    KoreanReuIrregular,
    KoreanInvariable,
    EnglishRegular,
    EnglishCopula,
    EnglishInvariable,
}

/// A language phenotype attached to a semantic concept.  This is lexical and
/// grammatical knowledge, never a semantic concept payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpressionNodeIR {
    /// Optional expression-level affinity; never part of semantic identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_emotion: Option<GenerationEmotionIR>,
    /// Optional regional expression affinity. The standard expression remains
    /// available, and dialect selection cannot cross a semantic concept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_korean_dialect: Option<crate::affective_field::KoreanDialectIR>,
    /// Optional social-distance affinity. This selects a lexical phenotype
    /// only after the meaning node is fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_roleplay_relationship: Option<crate::affective_field::RoleplayRelationshipIR>,
    /// Optional character-voice affinity. It is lexical knowledge rather than
    /// a replacement for the approved semantic plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_roleplay_voice: Option<crate::affective_field::RoleplayVoiceIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub korean_nominal_forms: Vec<crate::korean_nominal::KoreanNominalFormIR>,
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
        if !crate::korean_nominal::validate_forms(&entry.korean_nominal_forms, &entry.lexical_root)
            || entry.expression_id.trim().is_empty()
            || entry.concept_id.trim().is_empty()
            || entry.lexical_root.trim().is_empty()
            || entry.confidence_millis > 1_000
            || entry.provenance.trim().is_empty()
            || (entry
                .lexical_root
                .contains(['.', '!', '?', '。', '！', '？'])
                && !(entry.part_of_speech == ExpressionPartOfSpeechIR::Noun
                    && entry.provenance.starts_with("RUNTIME_REFERENT_SURFACE:"))
                // A source-bound report is the one non-nominal dynamic
                // expression allowed to retain sentence punctuation.  Its
                // lexical content is immutable user evidence, its concept
                // and provenance are fixed, and it is replayed as one
                // source-preserved trace node rather than parsed as a new
                // predicate.
                && !(entry.concept_id == "C_SOURCE_BOUND_REPORT"
                    && entry.provenance == "RUNTIME_SOURCE_BOUND_REPORT:VERBATIM_USER_EVIDENCE"))
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
            preferred_emotion: None,
            preferred_korean_dialect: None,
            preferred_roleplay_relationship: None,
            preferred_roleplay_voice: None,
            korean_nominal_forms: Vec::new(),
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

    fn attach_nominal_forms(
        &mut self,
        id: &str,
        forms: &[crate::korean_nominal::KoreanNominalFormIR],
    ) -> Result<(), String> {
        let entry = self.entries.get_mut(id).ok_or("UNKNOWN_EXPRESSION")?;
        let selected = forms
            .iter()
            .filter(|f| f.matches(&entry.lexical_root))
            .cloned()
            .collect::<Vec<_>>();
        if !crate::korean_nominal::validate_forms(&selected, &entry.lexical_root) {
            return Err("INVALID_NOMINAL_FORMS".into());
        }
        crate::korean_nominal::merge_forms(&mut entry.korean_nominal_forms, &selected);
        Ok(())
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
    NominalTheme,
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
    #[serde(default)]
    pub korean_dialect: crate::affective_field::KoreanDialectIR,
    /// Effective persona selector for this language-only realization. It is
    /// not semantic authority and must replay to the same surface.
    #[serde(default)]
    pub roleplay_persona: crate::affective_field::RoleplayPersonaIR,
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

/// Explicit surface policy carried with a response obligation. It is applied
/// before expression selection, syntax and morphology; no completed text is
/// accepted or rewritten here. Language-only callers retain their defaults.
#[derive(Clone, Copy)]
pub(crate) struct GenerationSettings {
    language: LanguageCodeIR,
    policy: Option<crate::affective_field::AffectiveRealizationPolicyIR>,
}

impl From<LanguageCodeIR> for GenerationSettings {
    fn from(language: LanguageCodeIR) -> Self {
        Self {
            language,
            policy: None,
        }
    }
}

impl GenerationSettings {
    pub(crate) fn language(self) -> LanguageCodeIR {
        self.language
    }

    pub(crate) fn with_policy(
        language: LanguageCodeIR,
        policy: &crate::affective_field::AffectiveRealizationPolicyIR,
    ) -> Self {
        Self {
            language,
            policy: Some(*policy),
        }
    }

    fn generate(
        self,
        mut request: GenerativeLanguageRequestIR<'_>,
    ) -> Result<GenerativeLanguageIR, String> {
        let mut korean_dialect = crate::affective_field::KoreanDialectIR::Standard;
        let mut roleplay_persona = crate::affective_field::RoleplayPersonaIR::default();
        if let Some(policy) = self.policy {
            roleplay_persona = policy.persona();
            if policy.formal
                || matches!(
                    policy.relationship,
                    crate::affective_field::RoleplayRelationshipIR::Professional
                        | crate::affective_field::RoleplayRelationshipIR::Respectful
                )
                || policy.scene == crate::affective_field::RoleplaySceneIR::Service
            {
                request.context.register = LanguageRegisterIR::Formal;
            } else if matches!(
                policy.relationship,
                crate::affective_field::RoleplayRelationshipIR::Peer
                    | crate::affective_field::RoleplayRelationshipIR::Close
                    | crate::affective_field::RoleplayRelationshipIR::Caregiving
            ) || policy.scene == crate::affective_field::RoleplaySceneIR::Companion
            {
                request.context.register = LanguageRegisterIR::Informal;
            }
            request.context.urgency_millis = policy.urgency_millis;
            if policy.warmth_millis > 150
                || matches!(
                    policy.voice,
                    crate::affective_field::RoleplayVoiceIR::Gentle
                )
                || matches!(
                    policy.relationship,
                    crate::affective_field::RoleplayRelationshipIR::Caregiving
                )
            {
                request.context.emotion = GenerationEmotionIR::Warm;
            }
            // Social coloring cannot change facts, scope or task intent.
            if (policy.playfulness_millis > 150
                || matches!(
                    policy.voice,
                    crate::affective_field::RoleplayVoiceIR::Lively
                ))
                && policy.urgency_millis <= 150
                && policy.brevity_millis <= 150
                && policy.scene != crate::affective_field::RoleplaySceneIR::Tense
                && request.context.register != LanguageRegisterIR::Formal
                && playful_social_anchor(&request.meaning).is_some()
            {
                request.context.emotion = GenerationEmotionIR::Playful;
            } else if request.context.emotion == GenerationEmotionIR::Playful {
                request.context.emotion = GenerationEmotionIR::Neutral;
            }
            if request.context.language == LanguageCodeIR::Korean {
                korean_dialect = policy.korean_dialect;
            }
        }
        GenerativeLanguageCortex.generate_with_korean_dialect(
            request,
            korean_dialect,
            roleplay_persona,
        )
    }
}

impl GenerativeLanguageIR {
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
                    self.korean_dialect,
                    self.roleplay_persona,
                )
            && (self.context.language == LanguageCodeIR::Korean
                || self.korean_dialect == crate::affective_field::KoreanDialectIR::Standard)
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

#[cfg(test)]
thread_local! {
    pub(crate) static GENERATION_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl GenerativeLanguageCortex {
    pub fn generate(
        &self,
        request: GenerativeLanguageRequestIR<'_>,
    ) -> Result<GenerativeLanguageIR, String> {
        self.generate_with_korean_dialect(
            request,
            crate::affective_field::KoreanDialectIR::Standard,
            crate::affective_field::RoleplayPersonaIR::default(),
        )
    }

    fn generate_with_korean_dialect(
        &self,
        request: GenerativeLanguageRequestIR<'_>,
        korean_dialect: crate::affective_field::KoreanDialectIR,
        roleplay_persona: crate::affective_field::RoleplayPersonaIR,
    ) -> Result<GenerativeLanguageIR, String> {
        #[cfg(test)]
        GENERATION_INVOCATIONS.with(|count| count.set(count.get() + 1));
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
        let expression_selection = select_expressions(
            &request.meaning,
            &request.context,
            korean_dialect,
            roleplay_persona,
            request.expressions,
        );
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
            korean_dialect,
            roleplay_persona,
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
            korean_dialect,
            roleplay_persona,
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
        // This builder owns every intermediate value and has already checked
        // the input meaning and the completed surface. Replaying morphology
        // here constructs the same sentence a second time. Untrusted or edited
        // IR still uses validate(), including replay, at its consumer boundary.
        if !generated.verification.faithful
            || generated.verification.unsupported_surface_tokens != 0
            || generated.verification.unsupported_claims != 0
            || generated.verification.semantic_roundtrip_sha256 != generated.meaning.semantic_sha256
            || generated.morphology.realized_text.chars().count() > MAX_REALIZED_CHARS
        {
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
                concept
                    if context.default_speech_intent == GenerationSpeechIntentIR::DescribePlan
                        && !matches!(concept, "C_ACKNOWLEDGE" | "C_COPULA") =>
                {
                    GenerationSpeechIntentIR::DescribePlan
                }
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
                "C_DIALOGUE_ANSWER_AMBIGUOUS" | "C_DIALOGUE_ANSWER_REQUEST_CONFLICT" => {
                    GenerationSpeechIntentIR::Ask
                }
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
    korean_dialect: crate::affective_field::KoreanDialectIR,
    roleplay_persona: crate::affective_field::RoleplayPersonaIR,
    store: &ExpressionNodeStore,
) -> ExpressionSelectionGraphIR {
    let mut selections = Vec::new();
    let mut unresolved = Vec::new();
    for node in &meaning.nodes {
        let mut candidates = store.candidates(&node.concept_id, context.language);
        candidates.sort_by(|left, right| {
            expression_score(right, context, korean_dialect, roleplay_persona)
                .cmp(&expression_score(
                    left,
                    context,
                    korean_dialect,
                    roleplay_persona,
                ))
                .then_with(|| left.expression_id.cmp(&right.expression_id))
        });
        let Some(selected) = candidates.first() else {
            unresolved.push(node.node_id.clone());
            continue;
        };
        let context_fit = context_fit(selected, context, korean_dialect, roleplay_persona);
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
                    format!(
                        "EXPRESSION_AFFECT:{:?};CONTEXT:{:?}",
                        selected.preferred_emotion, context.emotion
                    ),
                    format!(
                        "EXPRESSION_DIALECT:{:?};CONTEXT:{korean_dialect:?}",
                        selected.preferred_korean_dialect
                    ),
                    format!(
                        "EXPRESSION_RELATIONSHIP:{:?};CONTEXT:{:?}",
                        selected.preferred_roleplay_relationship, roleplay_persona.relationship
                    ),
                    format!(
                        "EXPRESSION_VOICE:{:?};CONTEXT:{:?}",
                        selected.preferred_roleplay_voice, roleplay_persona.voice
                    ),
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

fn expression_score(
    entry: &ExpressionNodeIR,
    context: &GenerationContextIR,
    korean_dialect: crate::affective_field::KoreanDialectIR,
    roleplay_persona: crate::affective_field::RoleplayPersonaIR,
) -> u32 {
    u32::from(entry.confidence_millis)
        + u32::from(context_fit(
            entry,
            context,
            korean_dialect,
            roleplay_persona,
        ))
}

fn context_fit(
    entry: &ExpressionNodeIR,
    context: &GenerationContextIR,
    korean_dialect: crate::affective_field::KoreanDialectIR,
    roleplay_persona: crate::affective_field::RoleplayPersonaIR,
) -> u16 {
    let register_fit = if entry.register == context.register {
        700
    } else if entry.register == LanguageRegisterIR::Neutral {
        400
    } else {
        100
    };
    // Style never routes to another concept. An unmarked expression remains
    // eligible in every context; a marked alias wins only for matching affect.
    // Register compatibility outweighs this optional preference.
    let affect_fit = match entry.preferred_emotion {
        Some(emotion) if emotion == context.emotion => 120,
        None => 60,
        Some(_) => 0,
    };
    let dialect_fit = match entry.preferred_korean_dialect {
        Some(dialect)
            if context.language == LanguageCodeIR::Korean && dialect == korean_dialect =>
        {
            160
        }
        None => 60,
        Some(_) => 0,
    };
    let relationship_fit = match entry.preferred_roleplay_relationship {
        Some(relationship) if relationship == roleplay_persona.relationship => 90,
        None => 50,
        Some(_) => 0,
    };
    let voice_fit = match entry.preferred_roleplay_voice {
        Some(voice) if voice == roleplay_persona.voice => 90,
        None => 50,
        Some(_) => 0,
    };
    (register_fit + affect_fit + dialect_fit + relationship_fit + voice_fit).min(1_000)
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
                        if role == SyntaxConstituentRoleIR::Agent
                            && meaning.nodes.iter().any(|node| {
                                node.node_id == edge.target_node_id
                                    && node.kind == GenerationMeaningNodeKindIR::EventReference
                            })
                        {
                            for argument in meaning.edges.iter().filter(|argument| {
                                argument.source_node_id == edge.target_node_id
                                    && argument.relation == GenerationMeaningRelationIR::Theme
                            }) {
                                if let Some(selected_argument) = expressions
                                    .selections
                                    .iter()
                                    .find(|item| item.meaning_node_id == argument.target_node_id)
                                {
                                    constituents.push(SyntaxConstituentIR {
                                        meaning_node_id: argument.target_node_id.clone(),
                                        expression_id: selected_argument
                                            .expression
                                            .expression_id
                                            .clone(),
                                        role: SyntaxConstituentRoleIR::NominalTheme,
                                    });
                                    source_edge_ids.push(argument.edge_id.clone());
                                }
                            }
                        }
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
        (_, SyntaxConstituentRoleIR::NominalTheme) => 0,
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

#[cfg(test)]
thread_local! {
    pub(crate) static MORPHOLOGY_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn realize_morphology(
    meaning: &GenerationMeaningGraphIR,
    context: &GenerationContextIR,
    syntax: &SyntaxPlanIR,
    expressions: &ExpressionSelectionGraphIR,
    korean_dialect: crate::affective_field::KoreanDialectIR,
    _roleplay_persona: crate::affective_field::RoleplayPersonaIR,
) -> MorphologicalRealizationIR {
    #[cfg(test)]
    MORPHOLOGY_PASSES.with(|passes| passes.set(passes.get() + 1));
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
        // Coordinate only compatible predicates joined by an explicit semantic
        // sequence edge. One plan scope owns the whole phrase; no output repair.
        let chainable = |c: &SyntaxClauseIR| {
            c.speech_intent == GenerationSpeechIntentIR::DescribePlan
                && c.constituents.iter().all(|item| {
                    matches!(
                        item.role,
                        SyntaxConstituentRoleIR::Predicate
                            | SyntaxConstituentRoleIR::Theme
                            | SyntaxConstituentRoleIR::Possessor
                    )
                })
                && constituent_selection(c, SyntaxConstituentRoleIR::Predicate, &selected)
                    .is_some_and(|p| {
                        matches!(
                            p.expression.morphology,
                            ExpressionMorphologyClassIR::KoreanHada
                                | ExpressionMorphologyClassIR::EnglishRegular
                        )
                    })
        };
        let joined = |left: &SyntaxClauseIR, right: &SyntaxClauseIR| {
            chainable(left)
                && chainable(right)
                && meaning.edges.iter().any(|edge| {
                    edge.relation == GenerationMeaningRelationIR::Sequence
                        && edge.source_node_id == left.event_node_id
                        && edge.target_node_id == right.event_node_id
                })
        };
        let chain = (
            clause_index
                .checked_sub(1)
                .and_then(|i| syntax.clauses.get(i))
                .is_some_and(|prior| joined(prior, clause)),
            syntax
                .clauses
                .get(clause_index + 1)
                .is_some_and(|next| joined(clause, next)),
        );
        let content_joined = |left: &SyntaxClauseIR, right: &SyntaxClauseIR| {
            content_projection_joined(left, right, meaning, &selected)
        };
        let content_chain = (
            clause_index
                .checked_sub(1)
                .and_then(|i| syntax.clauses.get(i))
                .is_some_and(|prior| content_joined(prior, clause)),
            syntax
                .clauses
                .get(clause_index + 1)
                .is_some_and(|next| content_joined(clause, next)),
        );
        let clause_tokens = if content_chain.0 || content_chain.1 {
            realize_content_projection_chain(
                clause,
                context,
                &selected,
                constituent_selection(clause, SyntaxConstituentRoleIR::Predicate, &selected)
                    .unwrap(),
                content_chain,
            )
        } else {
            match context.language {
                LanguageCodeIR::Korean => realize_korean_clause(clause, context, &selected, chain),
                _ => realize_english_clause(clause, context, &selected, chain),
            }
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
                let observed_nominal = first.expression_id.as_deref().is_some_and(|id| {
                    selected.values().any(|s| {
                        s.expression.expression_id == id
                            && s.expression.part_of_speech == ExpressionPartOfSpeechIR::Noun
                            && first.source_meaning_node_ids.contains(&s.meaning_node_id)
                            && first.surface == s.expression.lexical_root
                    })
                });
                first.surface = if observed_nominal {
                    english_positioned_nominal(&first.surface, true)
                } else {
                    uppercase_first(&first.surface)
                };
            }
        }
        for mut token in clause_tokens {
            token.token_index = tokens.len();
            tokens.push(token);
        }
    }
    if context.language == LanguageCodeIR::Korean {
        apply_korean_dialect(&mut tokens, korean_dialect);
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
    plan_chain: (bool, bool),
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
    if predicate
        .expression
        .concept_id
        .starts_with("C_EVENT_RECAP_")
    {
        return event_summary::realize_event_summary(clause, context, selected, predicate);
    }
    // A source-bound report is intentionally a literal realization of
    // user-supplied evidence.  It is not a paraphrase, a learned template, or
    // an assertion with new semantic authority.  Keeping this branch in the
    // morphology executor lets the completed report retain the ordinary
    // generation trace and replay contract instead of bypassing verification.
    if predicate.expression.concept_id == "C_SOURCE_BOUND_REPORT" {
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_EVENT_REFERENCE_CHOICE" | "C_EVENT_PERSON_REFERENCE_CHOICE"
    ) {
        return realize_event_reference_question(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_INTERACTION_PREFERENCE_ANSWER" {
        return realize_interaction_preference_answer(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_CONTENT_PROJECTION"
        || predicate
            .expression
            .concept_id
            .starts_with("C_CONTENT_RECALL_")
        || predicate
            .expression
            .concept_id
            .starts_with("C_CONTENT_FOCUS_")
    {
        return realize_content_projection(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_CONDITIONAL_ACK" {
        return realize_conditional_ack(clause, context, selected, predicate);
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
                    let particle = korean_particle(&theme.expression.lexical_root, "이", "가");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("{}{particle}", theme.expression.lexical_root),
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
                    let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!(
                            "‘{}’{particle} 문자 그대로의 행동이 아니라",
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
        let ending = if context.register == LanguageRegisterIR::Formal {
            "까요?"
        } else {
            "까?"
        };
        push_expression_token(&mut output, predicate, format!("{future_stem}{ending}"));
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
                "{}.",
                korean_request(&predicate.expression, context.register)
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
                "{}.",
                if context.register == LanguageRegisterIR::Formal {
                    korean_request(&predicate.expression, context.register)
                } else {
                    korean_conjugate(&predicate.expression, "아")
                }
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
            let particle = korean_particle(&benefit.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                benefit,
                format!("{}{particle}", benefit.expression.lexical_root),
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
            let particle = korean_particle(&benefit.expression.lexical_root, "을", "를");
            push_expression_token(
                &mut output,
                benefit,
                format!("{}{particle} 확인하거나,", benefit.expression.lexical_root),
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
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_ASK_EXPLANATION_TARGET" | "C_ASK_COMPARISON_TARGET"
    ) {
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
            format!("{}줄까?", korean_conjugate(&predicate.expression, "아")),
        );
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
                    let surface =
                        if crate::korean_nominal::surface_coda(&detail.expression.lexical_root)
                            .is_some()
                        {
                            let particle =
                                korean_direction_particle(&detail.expression.lexical_root);
                            format!("{}{particle}", detail.expression.lexical_root)
                        } else {
                            format!("‘{}’라고", detail.expression.lexical_root)
                        };
                    push_expression_token(&mut output, detail, surface);
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
            let surface = if crate::korean_nominal::surface_coda(particle_basis).is_some() {
                let particle = korean_particle(particle_basis, "이", "가");
                format!("{}{particle}", theme.expression.lexical_root)
            } else {
                format!("‘{}’ 표기가", theme.expression.lexical_root)
            };
            push_expression_token(&mut output, theme, surface);
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
            let surface =
                if crate::korean_nominal::surface_coda(&theme.expression.lexical_root).is_some() {
                    let particle = korean_particle(&theme.expression.lexical_root, "을", "를");
                    format!("{}{particle}", theme.expression.lexical_root)
                } else {
                    format!("‘{}’ 대상을", theme.expression.lexical_root)
                };
            push_expression_token(&mut output, theme, surface);
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
                "{}.",
                korean_request(&predicate.expression, context.register)
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
                    "대화 기록:",
                    "KO.DIALOGUE_ANSWER.RECORD.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("내용", &theme.expression.lexical_root),
                    );
                }
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        korean_labeled_quote("출처", &property.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "이렇게 남아 있어.".to_string());
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
                    "질문의 전제:",
                    "KO.DIALOGUE_ANSWER.PRESUPPOSITION.QUESTION",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’.", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "하지만 대화에서는 참으로 검증되지 않았어. 그 전제를 받아들여 답을 만들지 않을게."
                        .to_string(),
                );
            }
            "C_DIALOGUE_ANSWER_UNKNOWN_PROPERTY" => {
                if let Some(property) = property {
                    if let Some(owner) = theme {
                    let surface = if owner.expression.concept_id == "C_QUERY_CONTENT_ARGUMENT" {
                        format!("‘{}’에 대한", owner.expression.lexical_root)
                    } else { format!("{}의", owner.expression.lexical_root) };
                    push_expression_token(&mut output, owner, surface);
                    }
                    push_expression_token(&mut output, property, format!("{}{}", property.expression.lexical_root, korean_particle(&property.expression.lexical_root, "은", "는")));
                    push_grammar_token(&mut output, "아직", "KO.EPISTEMIC.CURRENT_GAP", &clause.event_node_id);
                    let ending = if context.register == LanguageRegisterIR::Formal { "겠습니다." } else { "겠어." };
                    push_expression_token(&mut output, predicate, format!("{}{ending}", predicate.expression.lexical_root));
                }
            },
            "C_DIALOGUE_ANSWER_NO_MATCH" => {
                if let Some(theme) = theme {
                    push_expression_token(&mut output, theme, format!("‘{}’에 관해서는", theme.expression.lexical_root));
                } else {
                    push_grammar_token(
                        &mut output,
                        "그 질문에 대해서는",
                        "KO.ANSWER_GAP.GENERIC_QUESTION",
                        &clause.event_node_id,
                    );
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
            "C_DIALOGUE_ANSWER_REQUEST_CONFLICT" => push_expression_token(
                &mut output, predicate,
                "요청한 항목과 금지한 항목이 겹쳐. 어느 요청을 따를지 알려줘.".to_string(),
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
                        korean_labeled_quote("인용 내용:", &theme.expression.lexical_root),
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
                        korean_labeled_quote("인용 내용:", &theme.expression.lexical_root),
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
                        korean_labeled_quote("인용 내용:", &theme.expression.lexical_root),
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
                        korean_labeled_quote("인용 내용:", &theme.expression.lexical_root),
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
                    let particle = korean_particle(&goal.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’{particle} 활성화되지 않았어.",
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
                    let particle = korean_particle(&goal.expression.lexical_root, "을", "를");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’{particle} 검토할 수 있지만 자동으로 실행되지는 않아.",
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
                    let particle = korean_particle(&goal.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’{particle} 활성화되지 않았어.",
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
                    let particle = korean_particle(&goal.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’{particle} 활성화되지 않았어.",
                            goal.expression.lexical_root
                        ),
                    );
                }
            }
            "C_GUARD_COUNTERFACTUAL" => {
                if let Some(theme) = theme {
                    let particle = korean_particle(&theme.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        theme,
                        format!("‘{}’{particle}", theme.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "반사실 조건이어서 현재 조건으로 취급하지 않아.".to_string(),
                );
                if let Some(goal) = goal {
                    let particle = korean_particle(&goal.expression.lexical_root, "은", "는");
                    push_expression_token(
                        &mut output,
                        goal,
                        format!(
                            "따라서 ‘{}’{particle} 활성화되지 않았어.",
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
                    "대화 기록의 원인 관계:",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("원인", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote("결과", &goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "이렇게 연결돼 있어.".to_string());
            }
            "C_DIALOGUE_RELATION_RESULT_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록의 결과 관계:",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("선행 내용", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote("결과", &goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "이렇게 이어진 것으로 남아 있어.".to_string());
            }
            "C_DIALOGUE_RELATION_CONCESSION_EDGE" => {
                push_grammar_token(
                    &mut output,
                    "대화 기록의 양보 관계:",
                    "KO.DIALOGUE_RELATION.EDGE.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("전제", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote(
                            "그럼에도 성립한 내용",
                            &goal.expression.lexical_root,
                        ),
                    );
                }
                push_expression_token(&mut output, predicate, "이렇게 연결돼 있어.".to_string());
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
                    "대화 사건 기록:",
                    "KO.TEMPORAL_ANSWER.TIME.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("사건", &theme.expression.lexical_root),
                    );
                }
                if let Some(property) = property {
                    push_expression_token(
                        &mut output,
                        property,
                        korean_labeled_quote("기록 시각", &property.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "이렇게 남아 있어.".to_string());
            }
            "C_TEMPORAL_ANSWER_EVENT" => {
                push_grammar_token(
                    &mut output,
                    "대화 사건 기록:",
                    "KO.TEMPORAL_ANSWER.EVENT.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("사건", &theme.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "기록이 남아 있어.".to_string());
            }
            "C_TEMPORAL_ANSWER_BEFORE" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록:",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("앞선 사건", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote("뒤의 사건", &goal.expression.lexical_root),
                    );
                }
                push_expression_token(&mut output, predicate, "앞선 사건이 먼저야.".to_string());
            }
            "C_TEMPORAL_ANSWER_DURING" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록:",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("포함된 사건", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote("기준 구간", &goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "포함된 사건은 이 구간 동안 일어난 것으로 연결돼 있어.".to_string(),
                );
            }
            "C_TEMPORAL_ANSWER_SIMULTANEOUS" => {
                push_grammar_token(
                    &mut output,
                    "대화의 시간 기록:",
                    "KO.TEMPORAL_ANSWER.RELATION.CONTEXT",
                    &clause.event_node_id,
                );
                if let Some(theme) = theme {
                    push_expression_token(
                        &mut output,
                        theme,
                        korean_labeled_quote("첫 사건", &theme.expression.lexical_root),
                    );
                }
                if let Some(goal) = goal {
                    push_expression_token(
                        &mut output,
                        goal,
                        korean_labeled_quote("둘째 사건", &goal.expression.lexical_root),
                    );
                }
                push_expression_token(
                    &mut output,
                    predicate,
                    "두 사건은 같은 시점으로 연결돼 있어.".to_string(),
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
        if !push_action_reference(&mut output, clause, context.language, selected, "에는") {
            if let Some(agent) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
            {
                push_expression_token(
                    &mut output,
                    agent,
                    format!("{}에는", agent.expression.lexical_root),
                );
            }
        }
        let absence = if context.register == LanguageRegisterIR::Formal {
            korean_formal_statement("없")
        } else {
            "없어".to_string()
        };
        push_expression_token(
            &mut output,
            property,
            format!("아직 실행 결과는 {absence}."),
        );
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
        if !push_action_reference(&mut output, clause, context.language, selected, "에 관해") {
            if let Some(agent) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
            {
                push_expression_token(
                    &mut output,
                    agent,
                    format!("{}에 관해", agent.expression.lexical_root),
                );
            }
        }
        push_expression_token(
            &mut output,
            property,
            format!(
                "검증된 실행 결과는 아직 {}.",
                if context.register == LanguageRegisterIR::Formal {
                    korean_formal_statement("없")
                } else {
                    "없어".to_string()
                }
            ),
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
            || constituent.role == SyntaxConstituentRoleIR::NominalTheme
            || (adjective_property.is_some()
                && constituent.role == SyntaxConstituentRoleIR::Negation)
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
        if constituent.role == SyntaxConstituentRoleIR::Agent
            && push_action_reference(&mut output, clause, context.language, selected, "는")
        {
            continue;
        }
        let particle = match constituent.role {
            SyntaxConstituentRoleIR::NominalTheme => "",
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
        let finite_negative_copula = constituent.role == SyntaxConstituentRoleIR::Negation
            && predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula
            && adjective_property.is_none()
            && matches!(
                context.register,
                LanguageRegisterIR::Formal | LanguageRegisterIR::Neutral
            );
        let surface = if finite_negative_copula {
            if context.register == LanguageRegisterIR::Formal {
                korean_formal_statement(&expression.expression.lexical_root)
            } else {
                "아니에요".to_string()
            }
        } else if crate::korean_nominal::surface_coda(&expression.expression.lexical_root).is_none()
            && matches!(
                constituent.role,
                SyntaxConstituentRoleIR::Agent
                    | SyntaxConstituentRoleIR::Theme
                    | SyntaxConstituentRoleIR::Goal
                    | SyntaxConstituentRoleIR::Property
            )
        {
            let root = &expression.expression.lexical_root;
            match constituent.role {
                SyntaxConstituentRoleIR::Agent => format!("‘{root}’ 항목은"),
                SyntaxConstituentRoleIR::Theme => format!("‘{root}’ 대상을"),
                SyntaxConstituentRoleIR::Goal => format!("‘{root}’ 대상으로"),
                SyntaxConstituentRoleIR::Property if has_negation => {
                    format!("‘{root}’ 상태는")
                }
                SyntaxConstituentRoleIR::Property => format!("‘{root}’ 상태"),
                _ => unreachable!(),
            }
        } else {
            format!("{}{}", expression.expression.lexical_root, particle)
        };
        push_expression_token(&mut output, expression, surface);
    }
    if clause.speech_intent == GenerationSpeechIntentIR::DescribePlan {
        // A plan description embeds the lexical predicate; it does not reuse
        // the promissive ending (-ㄹ게/-겠습니다). Keep grammar and root sources
        // separate and construct the surface only once.
        if !plan_chain.0 {
            output.insert(
                0,
                MorphologicalTokenIR {
                    token_index: 0,
                    surface: "계획은".to_string(),
                    attach_left: false,
                    expression_id: None,
                    grammar_rule_id: Some("KO.PLAN.TOPIC".to_string()),
                    source_meaning_node_ids: vec![clause.event_node_id.clone()],
                },
            );
        }
        if plan_chain.1 {
            push_expression_token(
                &mut output,
                predicate,
                format!("{}고,", predicate.expression.lexical_root),
            );
            return output;
        }
        push_expression_token(
            &mut output,
            predicate,
            korean_present_adnominal(&predicate.expression.lexical_root),
        );
        push_grammar_token(
            &mut output,
            match context.register {
                LanguageRegisterIR::Formal => "것입니다.",
                LanguageRegisterIR::Neutral => "거예요.",
                LanguageRegisterIR::Informal | LanguageRegisterIR::Internet => "거야.",
            },
            "KO.PLAN.NOMINAL_COMPLEMENT_COPULA",
            &clause.event_node_id,
        );
        return output;
    }
    let ending = korean_speech_ending(clause.speech_intent, context.register);
    let punctuation = if clause.speech_intent == GenerationSpeechIntentIR::Ask {
        "?"
    } else {
        "."
    };
    if let Some(property) = adjective_property {
        if let Some(negation) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Negation, selected)
        {
            // Adjectival negation uses the connective -지 and auxiliary 않-,
            // not the nominal negative copula 아니-. Keep both meaning sources.
            push_expression_token(
                &mut output,
                property,
                format!("{}지", property.expression.lexical_root),
            );
            let mut auxiliary = negation.expression.clone();
            auxiliary.lexical_root = "않".to_string();
            auxiliary.morphology = ExpressionMorphologyClassIR::KoreanInvariable;
            let surface = if ending == "아" {
                "않아".to_string()
            } else {
                korean_conjugate(&auxiliary, ending)
            };
            push_expression_token(&mut output, negation, format!("{surface}{punctuation}"));
        } else {
            let surface = korean_conjugate(&property.expression, ending);
            push_expression_token(&mut output, property, format!("{surface}{punctuation}"));
        }
        if let Some(token) = output.last_mut() {
            token
                .source_meaning_node_ids
                .push(clause.event_node_id.clone());
        }
        return output;
    }
    let mut surface = korean_conjugate(&predicate.expression, ending);
    if predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula
        && has_negation
        && matches!(
            context.register,
            LanguageRegisterIR::Formal | LanguageRegisterIR::Neutral
        )
    {
        // The negative predicate already bears the formal ending; adding the
        // positive copula again would produce an invalid double predication.
        surface.clear();
    }
    if predicate.expression.morphology == ExpressionMorphologyClassIR::KoreanCopula
        && !has_negation
        && matches!(ending, "아" | "아요")
    {
        if let Some(property) =
            constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
        {
            surface = if ending == "아요" {
                crate::korean_copula::positive_suffix(
                    crate::korean_copula::KoreanCopulaFormIR::PoliteStatement,
                    has_korean_final_consonant(&property.expression.lexical_root),
                )
                .to_string()
            } else if has_korean_final_consonant(&property.expression.lexical_root) {
                "이야".to_string()
            } else {
                "야".to_string()
            };
        }
    }
    push_expression_token(&mut output, predicate, format!("{surface}{punctuation}"));
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
    plan_chain: (bool, bool),
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
    if predicate
        .expression
        .concept_id
        .starts_with("C_EVENT_RECAP_")
    {
        return event_summary::realize_event_summary(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_SOURCE_BOUND_REPORT" {
        push_expression_token(
            &mut output,
            predicate,
            predicate.expression.lexical_root.clone(),
        );
        return output;
    }
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_EVENT_REFERENCE_CHOICE" | "C_EVENT_PERSON_REFERENCE_CHOICE"
    ) {
        return realize_event_reference_question(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_INTERACTION_PREFERENCE_ANSWER" {
        return realize_interaction_preference_answer(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_CONTENT_PROJECTION"
        || predicate
            .expression
            .concept_id
            .starts_with("C_CONTENT_RECALL_")
        || predicate
            .expression
            .concept_id
            .starts_with("C_CONTENT_FOCUS_")
    {
        return realize_content_projection(clause, context, selected, predicate);
    }
    if predicate.expression.concept_id == "C_CONDITIONAL_ACK" {
        return realize_conditional_ack(clause, context, selected, predicate);
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
    if matches!(
        predicate.expression.concept_id.as_str(),
        "C_ASK_EXPLANATION_TARGET" | "C_ASK_COMPARISON_TARGET"
    ) {
        if let Some(theme) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
        {
            push_expression_token(&mut output, theme, theme.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "should I",
            "EN.WH_TARGET.SELF_MODAL",
            &clause.event_node_id,
        );
        push_expression_token(
            &mut output,
            predicate,
            format!("{}?", predicate.expression.lexical_root),
        );
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
            "C_DIALOGUE_ANSWER_UNKNOWN_PROPERTY" => {
                if let Some(property) = property {
                    push_grammar_token(&mut output, "I don't yet", "EN.EPISTEMIC.CURRENT_GAP", &clause.event_node_id);
                    push_expression_token(&mut output, predicate, predicate.expression.lexical_root.clone());
                    push_grammar_token(&mut output, "the", "EN.PROPERTY.DEFINITE", &clause.event_node_id);
                    push_expression_token(&mut output, property, property.expression.lexical_root.clone());
                    if let Some(owner) = theme {
                        let query_argument = owner.expression.concept_id == "C_QUERY_CONTENT_ARGUMENT";
                        push_grammar_token(&mut output, if query_argument { "to" } else { "of" }, "EN.PROPERTY.OWNER", &clause.event_node_id);
                        let surface = if query_argument { format!("‘{}’.", owner.expression.lexical_root) }
                            else { format!("{}.", english_embedded_nominal(&owner.expression.lexical_root)) };
                        push_expression_token(&mut output, owner, surface);
                    } else if let Some(last) = output.last_mut() { last.surface.push('.'); }
                }
            },
            "C_DIALOGUE_ANSWER_NO_MATCH" => {
                if let Some(theme) = theme {
                    push_grammar_token(&mut output, "Regarding", "EN.ANSWER_GAP.TOPIC", &clause.event_node_id);
                    push_expression_token(&mut output, theme, format!("‘{}’,", theme.expression.lexical_root));
                } else {
                    push_grammar_token(
                        &mut output,
                        "For that question,",
                        "EN.ANSWER_GAP.GENERIC_QUESTION",
                        &clause.event_node_id,
                    );
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
            "C_DIALOGUE_ANSWER_REQUEST_CONFLICT" => push_expression_token(
                &mut output, predicate,
                "A requested item also appears in a prohibition. Which instruction should I follow?".to_string(),
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
        if !push_action_reference(&mut output, clause, context.language, selected, ".") {
            if let Some(agent) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
            {
                push_expression_token(
                    &mut output,
                    agent,
                    format!("{}.", english_nominal(&agent.expression.lexical_root)),
                );
            }
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
        if !push_action_reference(
            &mut output,
            clause,
            context.language,
            selected,
            ", so it has not been verified as executed.",
        ) {
            if let Some(agent) =
                constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
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
    if clause.speech_intent == GenerationSpeechIntentIR::DescribePlan && !plan_chain.0 {
        push_grammar_token(
            &mut output,
            if agent.is_some() {
                "the plan is for"
            } else {
                "the plan is"
            },
            "EN.PLAN.MATRIX_CLAUSE",
            &clause.event_node_id,
        );
    }
    if plan_chain.0 {
        push_grammar_token(
            &mut output,
            "then",
            "EN.PLAN.COORDINATED_INFINITIVE",
            &clause.event_node_id,
        );
    }
    if push_action_reference(&mut output, clause, context.language, selected, "") {
        // The referential predicate is nominal, never this clause's assertion.
    } else if let Some(agent) = agent {
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
        GenerationSpeechIntentIR::DescribePlan if plan_chain.0 => None,
        GenerationSpeechIntentIR::DescribePlan => Some(("to", "EN.PLAN.INFINITIVE")),
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
        last.surface.push(if plan_chain.1 { ',' } else { '.' });
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

/// Realize a noun phrase from the reference's predicate and its argument.
/// Tokens keep their separate concept sources; no compound sentence is stored
/// as an alias and no rendered output is parsed back into meaning.
fn push_action_reference(
    output: &mut Vec<MorphologicalTokenIR>,
    clause: &SyntaxClauseIR,
    language: LanguageCodeIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    suffix: &str,
) -> bool {
    let Some(theme) =
        constituent_selection(clause, SyntaxConstituentRoleIR::NominalTheme, selected)
    else {
        return false;
    };
    let Some(action) = constituent_selection(clause, SyntaxConstituentRoleIR::Agent, selected)
    else {
        return false;
    };
    let verbal = action.expression.part_of_speech == ExpressionPartOfSpeechIR::Verb;
    if language == LanguageCodeIR::Korean {
        push_expression_token(output, theme, theme.expression.lexical_root.clone());
        if verbal {
            let nominal = if action.expression.morphology == ExpressionMorphologyClassIR::KoreanHada
            {
                action
                    .expression
                    .lexical_root
                    .trim_end_matches('하')
                    .to_string()
            } else {
                format!("{}기", action.expression.lexical_root)
            };
            let suffix = if suffix == "는" {
                korean_particle(&nominal, "은", "는")
            } else {
                suffix
            };
            push_expression_token(output, action, format!("{nominal}{suffix}"));
        } else {
            push_grammar_token(
                output,
                "관련",
                "KO.REFERENCE.TARGET",
                &action.meaning_node_id,
            );
            let suffix = if suffix == "는" {
                korean_particle(&action.expression.lexical_root, "은", "는")
            } else {
                suffix
            };
            push_expression_token(
                output,
                action,
                format!("{}{suffix}", action.expression.lexical_root),
            );
        }
    } else if verbal {
        push_grammar_token(
            output,
            "the task to",
            "EN.REFERENCE.INFINITIVE",
            &action.meaning_node_id,
        );
        push_expression_token(output, action, action.expression.lexical_root.clone());
        push_expression_token(
            output,
            theme,
            format!(
                "{}{suffix}",
                english_nominal(&theme.expression.lexical_root)
            ),
        );
    } else {
        push_expression_token(
            output,
            action,
            english_nominal(&action.expression.lexical_root),
        );
        push_grammar_token(
            output,
            "concerning",
            "EN.REFERENCE.TARGET",
            &action.meaning_node_id,
        );
        push_expression_token(
            output,
            theme,
            format!(
                "{}{suffix}",
                english_nominal(&theme.expression.lexical_root)
            ),
        );
    }
    true
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

/// Apply regional finite morphology to already typed Korean tokens before the
/// sentence is joined. This is a suffix inventory, not a sentence rewrite:
/// meaning-node ownership, token count and expression identity stay intact.
fn apply_korean_dialect(
    tokens: &mut [MorphologicalTokenIR],
    dialect: crate::affective_field::KoreanDialectIR,
) {
    use crate::affective_field::KoreanDialectIR as D;
    if dialect == D::Standard {
        return;
    }
    for token in tokens {
        if !token.surface.ends_with(['.', '?', '!', '。', '？', '！']) {
            continue;
        }
        token.surface = korean_dialect_surface(&token.surface, dialect);
    }
}

fn korean_dialect_surface(
    surface: &str,
    dialect: crate::affective_field::KoreanDialectIR,
) -> String {
    use crate::affective_field::KoreanDialectIR as D;
    let (body, punctuation) = surface
        .char_indices()
        .next_back()
        .filter(|(_, c)| matches!(c, '.' | '?' | '!' | '。' | '？' | '！'))
        .map_or((surface, ""), |(index, _)| {
            (&surface[..index], &surface[index..])
        });
    let replace = |pairs: &[(&str, &str)]| {
        pairs.iter().find_map(|(standard, regional)| {
            body.strip_suffix(standard)
                .map(|prefix| format!("{prefix}{regional}{punctuation}"))
        })
    };
    match dialect {
        D::Standard => surface.to_string(),
        D::Gyeongsang => replace(&[
            ("하지 않습니까", "하지 않습니꺼"),
            ("하지 않습니다", "하지 않습니더"),
            ("하지 않나요", "하지 않습니꺼"),
            ("하지 않아요", "하지 않습니더"),
            ("아닌가요", "아입니꺼"),
            ("아니에요", "아입니더"),
            ("아닙니까", "아입니꺼"),
            ("아닙니다", "아입니더"),
            ("합니까", "합니꺼"),
            ("합니다", "합니더"),
            ("습니까", "습니꺼"),
            ("습니다", "습니더"),
            ("하나요", "합니꺼"),
            ("해요", "합니더"),
            ("안녕하세요", "안녕하이소"),
            ("까요", "까예"),
            ("인가요", "입니꺼"),
            ("이에요", "입니더"),
            ("예요", "입니더"),
            ("요", "예"),
            ("까", "까예"),
        ])
        .unwrap_or_else(|| surface.to_string()),
        D::Chungcheong => {
            if let Some(prefix) = body.strip_suffix('요') {
                return format!("{prefix}유{punctuation}");
            }
            replace(&[
                ("아니야", "아니에유"),
                ("이야", "이에유"),
                ("야", "예유"),
                ("하지 않아", "하지 않아유"),
                ("모르겠어", "모르겠어유"),
                ("할게", "할게유"),
                ("해", "해유"),
                ("까", "까유"),
            ])
            .unwrap_or_else(|| surface.to_string())
        }
    }
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

fn korean_labeled_quote(label: &str, content: &str) -> String {
    let content = content.trim();
    let punctuation_already_inside = content
        .chars()
        .last()
        .is_some_and(|character| matches!(character, '.' | '?' | '!' | '。'));
    format!(
        "{label} ‘{content}’{}",
        if punctuation_already_inside { "" } else { "." }
    )
}

fn nominal_suffix<'a>(
    expression: &ExpressionNodeIR,
    consonant: &'a str,
    vowel: &'a str,
) -> Option<&'a str> {
    crate::korean_nominal::final_coda(&expression.lexical_root, &expression.korean_nominal_forms)
        .map(|coda| if coda { consonant } else { vowel })
}

fn korean_particle<'a>(surface: &str, consonant: &'a str, vowel: &'a str) -> &'a str {
    crate::korean_nominal::select_particle(surface, consonant, vowel).unwrap_or_else(|| {
        // A Latin spelling or opaque identifier does not provide Korean
        // pronunciation evidence.  Case particles may be omitted in Korean,
        // so omission is safer than inventing a final sound.  Callers that
        // require an overt role marker use a Korean head noun instead.
        match (consonant, vowel) {
            ("의", "의") => "의",
            ("과", "와") => "하고",
            _ => "",
        }
    })
}

fn korean_direction_particle(surface: &str) -> &'static str {
    crate::korean_nominal::select_directional_particle(surface).unwrap_or("")
}

fn has_korean_final_consonant(surface: &str) -> bool {
    crate::korean_nominal::surface_coda(surface).unwrap_or(false)
}

fn korean_formal_statement(root: &str) -> String {
    let Some(last) = root.chars().last().filter(|c| ('가'..='힣').contains(c)) else {
        return format!("{root}습니다");
    };
    let jong = (u32::from(last) - u32::from('가')) % 28;
    if jong == 0 || jong == 8 {
        let syllable =
            char::from_u32(u32::from(last) - jong + 17).expect("Hangul final substitution");
        format!("{}{syllable}니다", &root[..root.len() - last.len_utf8()])
    } else {
        format!("{root}습니다")
    }
}

/// The acknowledgement lexeme and inflection are shared by standalone and
/// clause-initial receipts. A world clause must not freeze its own informal
/// acknowledgement while the rest of the clause follows the chosen register.
fn korean_acknowledgement(register: LanguageRegisterIR) -> String {
    let lexeme = expression(
        "EXPR.KO.ACKNOWLEDGE",
        LanguageCodeIR::Korean,
        "C_ACKNOWLEDGE",
        KOREAN_ACKNOWLEDGEMENT_STEM,
        ExpressionPartOfSpeechIR::Verb,
        ExpressionMorphologyClassIR::KoreanInvariable,
        LanguageRegisterIR::Informal,
    );
    korean_conjugate(
        &lexeme,
        korean_speech_ending(GenerationSpeechIntentIR::Acknowledge, register),
    )
}

// Grammatical act and register select an ending before surface construction.
// No completed sentence is rewritten or generated again to adjust politeness.
fn korean_speech_ending(
    intent: GenerationSpeechIntentIR,
    register: LanguageRegisterIR,
) -> &'static str {
    let formal = register == LanguageRegisterIR::Formal;
    match intent {
        GenerationSpeechIntentIR::DescribePlan => "는",
        GenerationSpeechIntentIR::CommitFutureAction => {
            if formal {
                "겠습니다"
            } else if register == LanguageRegisterIR::Neutral {
                "ㄹ게요"
            } else {
                "ㄹ게"
            }
        }
        GenerationSpeechIntentIR::Advise => "아야 해요",
        GenerationSpeechIntentIR::Invite => {
            if formal {
                "아 봅시다"
            } else if register == LanguageRegisterIR::Neutral {
                "아 봐요"
            } else {
                "아 보자"
            }
        }
        GenerationSpeechIntentIR::Ask => {
            if formal {
                "ㅂ니까"
            } else {
                "나요"
            }
        }
        GenerationSpeechIntentIR::Acknowledge | GenerationSpeechIntentIR::Inform => {
            if formal {
                "ㅂ니다"
            } else if register == LanguageRegisterIR::Neutral {
                "아요"
            } else {
                "아"
            }
        }
    }
}

/// Present adnominal verb ending: ㄹ drops before 는; other regular roots
/// retain their stem. This construction is shared by any supplied verb root.
fn korean_present_adnominal(root: &str) -> String {
    let Some(last) = root.chars().last() else {
        return "는".to_string();
    };
    if ('가'..='힣').contains(&last) && (u32::from(last) - u32::from('가')) % 28 == 8 {
        let stem = &root[..root.len() - last.len_utf8()];
        let without_rieul = char::from_u32(u32::from(last) - 8).expect("Hangul final removal");
        format!("{stem}{without_rieul}는")
    } else {
        format!("{root}는")
    }
}

/// Connect a lexical predicate stem to 아/어 without treating every stem as
/// `stem + 어`.  This covers the productive regular contractions used by the
/// runtime expression inventory.  Lexically irregular ㄷ/ㅂ/ㅅ predicates need
/// explicit morphology evidence rather than being guessed from spelling.
fn korean_aeo_connective(root: &str) -> String {
    let Some(last) = root.chars().last().filter(|c| ('가'..='힣').contains(c)) else {
        return format!("{root}어");
    };
    let code = u32::from(last) - u32::from('가');
    let initial = code / (21 * 28);
    let medial = (code / 28) % 21;
    let final_consonant = code % 28;
    if final_consonant != 0 {
        let ending = if matches!(medial, 0 | 8) {
            '아'
        } else {
            '어'
        };
        return format!("{root}{ending}");
    }

    let contracted_medial = match medial {
        // ㅏ/ㅐ/ㅓ/ㅔ/ㅕ/ㅘ/ㅙ/ㅝ/ㅞ already contain the connective vowel.
        0 | 1 | 4 | 5 | 6 | 9 | 10 | 14 | 15 => None,
        // ㅗ + ㅏ -> ㅘ
        8 => Some(9),
        // ㅜ + ㅓ -> ㅝ
        13 => Some(14),
        // ㅣ + ㅓ -> ㅕ
        20 => Some(6),
        // ㅚ + ㅓ -> ㅙ
        11 => Some(10),
        // ㅡ drops before 아/어. The preceding syllable controls vowel
        // harmony when it exists: 바쁘 -> 바빠, 잠그 -> 잠가, 쓰 -> 써.
        18 => {
            let prefix = &root[..root.len() - last.len_utf8()];
            let connective_medial = prefix
                .chars()
                .next_back()
                .filter(|c| ('가'..='힣').contains(c))
                .map(|c| (u32::from(c) - u32::from('가')) / 28 % 21)
                .filter(|medial| matches!(medial, 0 | 8))
                .map_or(4, |_| 0);
            Some(connective_medial)
        }
        _ => return format!("{root}어"),
    };
    if let Some(medial) = contracted_medial {
        let contracted = char::from_u32(u32::from('가') + (initial * 21 + medial) * 28)
            .expect("valid Hangul contraction");
        format!("{}{contracted}", &root[..root.len() - last.len_utf8()])
    } else {
        root.to_string()
    }
}

fn korean_digeut_irregular_aeo(root: &str) -> String {
    let Some(last) = root.chars().last().filter(|c| ('가'..='힣').contains(c)) else {
        return format!("{root}어");
    };
    let code = u32::from(last) - u32::from('가');
    if code % 28 != 7 {
        return korean_aeo_connective(root);
    }
    let changed = char::from_u32(u32::from(last) + 1).expect("valid Hangul ㄷ-to-ㄹ change");
    let prefix = &root[..root.len() - last.len_utf8()];
    let medial = code / 28 % 21;
    let ending = if matches!(medial, 0 | 8) {
        '아'
    } else {
        '어'
    };
    format!("{prefix}{changed}{ending}")
}

fn korean_reu_irregular_aeo(root: &str) -> String {
    let Some(last) = root.chars().last().filter(|c| ('가'..='힣').contains(c)) else {
        return format!("{root}어");
    };
    let last_code = u32::from(last) - u32::from('가');
    if last_code / 28 % 21 != 18 || last_code % 28 != 0 {
        return korean_aeo_connective(root);
    }
    let prefix = &root[..root.len() - last.len_utf8()];
    let Some(previous) = prefix
        .chars()
        .next_back()
        .filter(|c| ('가'..='힣').contains(c))
    else {
        return korean_aeo_connective(root);
    };
    let previous_code = u32::from(previous) - u32::from('가');
    if previous_code % 28 != 0 {
        return korean_aeo_connective(root);
    }
    let previous_with_rieul =
        char::from_u32(u32::from(previous) + 8).expect("valid Hangul ㄹ insertion");
    let final_medial = if matches!(previous_code / 28 % 21, 0 | 8) {
        0
    } else {
        4
    };
    let final_syllable =
        char::from_u32(u32::from('가') + ((last_code / (21 * 28)) * 21 + final_medial) * 28)
            .expect("valid Hangul 르 contraction");
    let prefix_without_previous = &prefix[..prefix.len() - previous.len_utf8()];
    format!("{prefix_without_previous}{previous_with_rieul}{final_syllable}")
}

fn korean_request(expression: &ExpressionNodeIR, register: LanguageRegisterIR) -> String {
    let connective = korean_conjugate(expression, "아");
    if matches!(
        register,
        LanguageRegisterIR::Formal | LanguageRegisterIR::Neutral
    ) {
        format!("{connective} 주세요")
    } else {
        format!("{connective}줘")
    }
}

fn korean_conjugate(expression: &ExpressionNodeIR, ending: &str) -> String {
    if ending == "겠습니다" {
        if expression.morphology == ExpressionMorphologyClassIR::KoreanHada {
            return crate::korean_hada::realize(
                expression.lexical_root.trim_end_matches('하'),
                crate::korean_hada::KoreanHadaFormIR::FutureFormal,
                true,
            )
            .expect("non-empty Korean HADA root");
        }
        return format!("{}겠습니다", expression.lexical_root);
    }
    match expression.morphology {
        ExpressionMorphologyClassIR::KoreanHada => {
            use crate::korean_hada::KoreanHadaFormIR as F;
            let form = match ending {
                "ㄹ게" => F::FutureInformal,
                "ㄹ게요" => {
                    return format!(
                        "{}요",
                        crate::korean_hada::realize(
                            expression.lexical_root.trim_end_matches('하'),
                            F::FutureInformal,
                            true,
                        )
                        .expect("non-empty Korean HADA root")
                    )
                }
                "아야 해요" => F::NecessityPolite,
                "나요" => F::PoliteQuestion,
                "ㅂ니까" => F::FormalQuestion,
                "ㅂ니다" => F::FormalStatement,
                "아요" => F::PoliteStatement,
                "아 보자" => F::InviteInformal,
                "아 봐요" => {
                    return format!("{}해 봐요", expression.lexical_root.trim_end_matches('하'))
                }
                "아 봅시다" => F::InviteFormal,
                _ => F::InformalStatement,
            };
            crate::korean_hada::realize(expression.lexical_root.trim_end_matches('하'), form, true)
                .expect("non-empty Korean HADA root")
        }
        ExpressionMorphologyClassIR::KoreanCopula => {
            use crate::korean_copula::KoreanCopulaFormIR as F;
            crate::korean_copula::positive_suffix(
                match ending {
                    "ㅂ니다" => F::FormalStatement,
                    "ㅂ니까" => F::FormalQuestion,
                    "나요" => F::PoliteQuestion,
                    "아요" => F::PoliteStatement,
                    _ => F::InformalStatement,
                },
                false,
            )
            .to_string()
        }
        ExpressionMorphologyClassIR::KoreanDigeutIrregular => match ending {
            "ㄹ게요" => format!("{}요", korean_conjugate(expression, "ㄹ게")),
            "ㄹ게" => {
                let root = &expression.lexical_root;
                let Some(last) = root.chars().last().filter(|c| ('가'..='힣').contains(c)) else {
                    return format!("{root}을게");
                };
                let code = u32::from(last) - u32::from('가');
                if code % 28 == 7 {
                    let changed =
                        char::from_u32(u32::from(last) + 1).expect("valid Hangul ㄷ-to-ㄹ change");
                    format!("{}{changed}을게", &root[..root.len() - last.len_utf8()])
                } else {
                    korean_future_commitment(root)
                }
            }
            "아야 해요" => format!(
                "{}야 해요",
                korean_digeut_irregular_aeo(&expression.lexical_root)
            ),
            "나요" => format!("{}나요", expression.lexical_root),
            "ㅂ니까" => korean_formal_question(&expression.lexical_root),
            "ㅂ니다" => korean_formal_statement(&expression.lexical_root),
            "아 보자" => format!(
                "{} 보자",
                korean_digeut_irregular_aeo(&expression.lexical_root)
            ),
            "아 봅시다" => format!(
                "{} 봅시다",
                korean_digeut_irregular_aeo(&expression.lexical_root)
            ),
            "아 봐요" => format!(
                "{} 봐요",
                korean_digeut_irregular_aeo(&expression.lexical_root)
            ),
            "아요" => format!(
                "{}요",
                korean_digeut_irregular_aeo(&expression.lexical_root)
            ),
            _ => korean_digeut_irregular_aeo(&expression.lexical_root),
        },
        ExpressionMorphologyClassIR::KoreanReuIrregular => match ending {
            "ㄹ게" => korean_future_commitment(&expression.lexical_root),
            "ㄹ게요" => format!("{}요", korean_future_commitment(&expression.lexical_root)),
            "아야 해요" => format!(
                "{}야 해요",
                korean_reu_irregular_aeo(&expression.lexical_root)
            ),
            "나요" => format!("{}나요", expression.lexical_root),
            "ㅂ니까" => korean_formal_question(&expression.lexical_root),
            "ㅂ니다" => korean_formal_statement(&expression.lexical_root),
            "아 보자" => format!(
                "{} 보자",
                korean_reu_irregular_aeo(&expression.lexical_root)
            ),
            "아 봅시다" => format!(
                "{} 봅시다",
                korean_reu_irregular_aeo(&expression.lexical_root)
            ),
            "아 봐요" => format!(
                "{} 봐요",
                korean_reu_irregular_aeo(&expression.lexical_root)
            ),
            "아요" => format!("{}요", korean_reu_irregular_aeo(&expression.lexical_root)),
            _ => korean_reu_irregular_aeo(&expression.lexical_root),
        },
        ExpressionMorphologyClassIR::KoreanInvariable => match ending {
            "ㄹ게" => korean_future_commitment(&expression.lexical_root),
            "ㄹ게요" => format!("{}요", korean_future_commitment(&expression.lexical_root)),
            "아야 해요" => {
                format!("{}야 해요", korean_aeo_connective(&expression.lexical_root))
            }
            "나요" => korean_present_adnominal(&expression.lexical_root)
                .strip_suffix('는')
                .map_or_else(
                    || format!("{}나요", expression.lexical_root),
                    |stem| format!("{stem}나요"),
                ),
            "ㅂ니까" => korean_formal_question(&expression.lexical_root),
            "ㅂ니다" => korean_formal_statement(&expression.lexical_root),
            "아 보자" => format!("{} 보자", korean_aeo_connective(&expression.lexical_root)),
            "아 봅시다" => {
                format!("{} 봅시다", korean_aeo_connective(&expression.lexical_root))
            }
            "아 봐요" => format!("{} 봐요", korean_aeo_connective(&expression.lexical_root)),
            "아요" => format!("{}요", korean_aeo_connective(&expression.lexical_root)),
            _ => korean_aeo_connective(&expression.lexical_root),
        },
        _ => format!("{}{}", expression.lexical_root, ending),
    }
}

fn korean_formal_question(root: &str) -> String {
    let statement = korean_formal_statement(root);
    statement
        .strip_suffix("니다")
        .map_or_else(|| format!("{root}습니까"), |stem| format!("{stem}니까"))
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

fn english_embedded_nominal(root: &str) -> String {
    english_positioned_nominal(root, false)
}

fn english_positioned_nominal(root: &str, initial: bool) -> String {
    if let Some((determiner, noun)) = root.split_once(' ') {
        if matches!(determiner, "The" | "A" | "An" | "the" | "a" | "an") {
            let determiner = determiner.to_ascii_lowercase();
            return format!(
                "{} {noun}",
                if initial {
                    uppercase_first(&determiner)
                } else {
                    determiner
                }
            );
        }
    }
    if initial
        && root
            .split_whitespace()
            .next()
            .is_none_or(|first| !first.chars().any(char::is_uppercase))
    {
        uppercase_first(root)
    } else {
        root.to_string()
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
        preferred_emotion: None,
        preferred_korean_dialect: None,
        preferred_roleplay_relationship: None,
        preferred_roleplay_voice: None,
        korean_nominal_forms: Vec::new(),
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
        EnglishCopula, EnglishInvariable, EnglishRegular, KoreanCopula, KoreanDigeutIrregular,
        KoreanHada, KoreanInvariable, KoreanReuIrregular,
    };
    use ExpressionPartOfSpeechIR::{Adjective, Interjection, Noun, Verb};
    use LanguageCodeIR::{English, Korean};
    use LanguageRegisterIR::{Informal, Neutral};
    let concepts = [
        (
            "C_ACKNOWLEDGE",
            KOREAN_ACKNOWLEDGEMENT_STEM,
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
            KoreanDigeutIrregular,
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
            KoreanDigeutIrregular,
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
            "C_SAVE",
            "저장하",
            "save",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_READ",
            "읽",
            "read",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_OPEN",
            "열",
            "open",
            Verb,
            KoreanInvariable,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_TRANSFORM",
            "변환하",
            "transform",
            Verb,
            KoreanHada,
            EnglishRegular,
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
            "C_DELETE",
            "삭제하",
            "delete",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_DEPLOY",
            "배포하",
            "deploy",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_UPDATE",
            "갱신하",
            "update",
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
            "C_CURRENT_SITUATION",
            "그 상황",
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
            "C_ASK_EXPLANATION_TARGET",
            "설명하",
            "explain",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_ASK_COMPARISON_TARGET",
            "비교하",
            "compare",
            Verb,
            KoreanHada,
            EnglishRegular,
            Neutral,
        ),
        (
            "C_UNBOUND_TARGET",
            "무엇",
            "What",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
            Neutral,
        ),
        (
            "C_UNBOUND_PAIR",
            "무엇과 무엇",
            "Which two things",
            Noun,
            KoreanInvariable,
            EnglishInvariable,
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
            "C_DIALOGUE_ANSWER_UNKNOWN_PROPERTY",
            "모르",
            "know",
            Verb,
            KoreanReuIrregular,
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
            "C_DIALOGUE_ANSWER_REQUEST_CONFLICT",
            "확인하",
            "clarify",
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
            "C_CONVERSATION_EXTERNAL_EXECUTION_UNAVAILABLE",
            "이 대화 경로에서 직접 실행을 지원하지 않는 작업",
            "not directly executable through this conversation path",
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
    let mut entries: Vec<_> = concepts
        .into_iter()
        .flat_map(|(concept, ko, en, pos, ko_morph, en_morph, register)| {
            let suffix = concept.trim_start_matches("C_");
            [
                expression(
                    &format!("EXPR.KO.{suffix}"),
                    Korean,
                    concept,
                    ko,
                    if concept == "C_ACKNOWLEDGE" {
                        Verb
                    } else {
                        pos
                    },
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
        .collect();
    // Lexical greeting variants, not complete response alternatives. Their
    // surrounding discourse and morphology are still assembled from meaning.
    for (language, root, morphology) in [
        (Korean, "반가워", KoreanInvariable),
        (English, "Hey", EnglishInvariable),
    ] {
        let mut variant = expression(
            &format!("EXPR.{language:?}.GREETING.WARM"),
            language,
            "C_DIALOGUE_GREETING_REPLY",
            root,
            Interjection,
            morphology,
            Informal,
        );
        variant.preferred_emotion = Some(GenerationEmotionIR::Warm);
        variant.preferred_roleplay_voice = Some(crate::affective_field::RoleplayVoiceIR::Gentle);
        entries.push(variant);
    }
    for (concept, root) in [
        ("C_DIALOGUE_HOLD_ACK", "그래"),
        ("C_DIALOGUE_GRATITUDE_REPLY", "별말을"),
        ("C_DIALOGUE_FAREWELL_REPLY", "또 보자"),
    ] {
        let mut variant = expression(
            &format!("EXPR.Korean.{concept}.WARM"),
            Korean,
            concept,
            root,
            Interjection,
            KoreanInvariable,
            Informal,
        );
        variant.preferred_emotion = Some(GenerationEmotionIR::Warm);
        entries.push(variant);
    }
    // Persona-specific entries remain lexical/interjection phenotypes. They
    // never encode a complete answer or bypass the shared semantic plan.
    for (concept, root, relationship, voice) in [
        (
            "C_DIALOGUE_GREETING_REPLY",
            "어, 안녕",
            crate::affective_field::RoleplayRelationshipIR::Close,
            crate::affective_field::RoleplayVoiceIR::Balanced,
        ),
        (
            "C_DIALOGUE_GREETING_REPLY",
            "반가워",
            crate::affective_field::RoleplayRelationshipIR::Close,
            crate::affective_field::RoleplayVoiceIR::Gentle,
        ),
        (
            "C_DIALOGUE_GREETING_REPLY",
            "반가워",
            crate::affective_field::RoleplayRelationshipIR::Peer,
            crate::affective_field::RoleplayVoiceIR::Lively,
        ),
        (
            "C_DIALOGUE_GRATITUDE_REPLY",
            "별말을",
            crate::affective_field::RoleplayRelationshipIR::Close,
            crate::affective_field::RoleplayVoiceIR::Balanced,
        ),
        (
            "C_DIALOGUE_FAREWELL_REPLY",
            "잘 가",
            crate::affective_field::RoleplayRelationshipIR::Close,
            crate::affective_field::RoleplayVoiceIR::Balanced,
        ),
    ] {
        let mut variant = expression(
            &format!("EXPR.KO.{concept}.ROLEPLAY.{relationship:?}.{voice:?}"),
            Korean,
            concept,
            root,
            ExpressionPartOfSpeechIR::Interjection,
            KoreanInvariable,
            Informal,
        );
        variant.preferred_roleplay_relationship = Some(relationship);
        variant.preferred_roleplay_voice = Some(voice);
        entries.push(variant);
    }
    // Register-specific lexical forms of social acts. These are words and
    // conventional interjections, not input-sentence/answer pairs. The same
    // concept still owns the discourse move and its following clauses.
    for (concept, korean, english) in [
        ("C_DIALOGUE_GREETING_REPLY", "안녕하세요", "Hello"),
        ("C_DIALOGUE_HOLD_ACK", "네", "Okay"),
        ("C_DIALOGUE_GRATITUDE_REPLY", "천만에요", "You're welcome"),
        ("C_DIALOGUE_FAREWELL_REPLY", "좋습니다", "Sounds good"),
    ] {
        for (language, root, morphology) in [
            (Korean, korean, KoreanInvariable),
            (English, english, EnglishInvariable),
        ] {
            entries.push(expression(
                &format!("EXPR.{language:?}.{concept}.FORMAL"),
                language,
                concept,
                root,
                Interjection,
                morphology,
                LanguageRegisterIR::Formal,
            ));
        }
    }
    // Honorific predicate lexeme. Register selection chooses the lexical
    // honorific before the shared future/question morphology is composed, so
    // the following clause cannot fall back to an informal `도와주` stem.
    entries.push(expression(
        "EXPR.Korean.C_DIALOGUE_OFFER_HELP.FORMAL",
        Korean,
        "C_DIALOGUE_OFFER_HELP",
        "도와드리",
        Verb,
        KoreanInvariable,
        LanguageRegisterIR::Formal,
    ));
    for (dialect, label, variants) in [
        (
            crate::affective_field::KoreanDialectIR::Gyeongsang,
            "GYEONGSANG",
            [
                ("C_DIALOGUE_GREETING_REPLY", "반갑데이"),
                ("C_DIALOGUE_HOLD_ACK", "알겠데이"),
                ("C_DIALOGUE_GRATITUDE_REPLY", "아이다"),
                ("C_DIALOGUE_FAREWELL_REPLY", "또 보자데이"),
            ],
        ),
        (
            crate::affective_field::KoreanDialectIR::Chungcheong,
            "CHUNGCHEONG",
            [
                ("C_DIALOGUE_GREETING_REPLY", "반가워유"),
                ("C_DIALOGUE_HOLD_ACK", "알겠어유"),
                ("C_DIALOGUE_GRATITUDE_REPLY", "괜찮아유"),
                ("C_DIALOGUE_FAREWELL_REPLY", "또 봐유"),
            ],
        ),
    ] {
        for (concept, root) in variants {
            let mut variant = expression(
                &format!("EXPR.Korean.{concept}.{label}"),
                Korean,
                concept,
                root,
                Interjection,
                KoreanInvariable,
                Informal,
            );
            variant.preferred_korean_dialect = Some(dialect);
            entries.push(variant);
        }
    }
    entries
}

#[cfg(test)]
pub(crate) fn generate_plan_preview_from_knowledge(
    settings: impl Into<GenerationSettings>,
    subject: &str,
    intent: PlanIntentIR,
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    generate_plan_preview_from_knowledge_with_directive(
        settings,
        subject,
        intent,
        grounding_ref,
        None,
        false,
    )
}

#[cfg(test)]
pub(crate) fn generate_plan_preview_from_knowledge_with_directive(
    settings: impl Into<GenerationSettings>,
    subject: &str,
    intent: PlanIntentIR,
    grounding_ref: &str,
    directive_ref: Option<&str>,
    concise: bool,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    generate_plan_preview_with_predicate(
        settings,
        subject,
        intent,
        grounding_ref,
        directive_ref,
        if concise {
            PlanPreviewContentIR::Compact
        } else {
            PlanPreviewContentIR::Detailed
        },
        None,
    )
}

fn resolve_plan_action_concept(
    expressions: &ExpressionNodeStore,
    language: LanguageCodeIR,
    intent: PlanIntentIR,
    predicate: Option<&str>,
) -> String {
    let fallback = match intent {
        PlanIntentIR::Repair => "C_REPAIR",
        PlanIntentIR::Execute => "C_PERFORM",
        PlanIntentIR::Investigate => "C_NARROW",
        PlanIntentIR::Create => "C_CREATE",
        PlanIntentIR::Learn => "C_LEARN",
        PlanIntentIR::Explain | PlanIntentIR::Communicate => "C_EXPLAIN",
        PlanIntentIR::Plan => "C_PLAN",
    };
    predicate
        .map(|id| format!("C_{}", id.strip_prefix("C_").unwrap_or(id)))
        .filter(|id| {
            expressions
                .candidates(id, language)
                .iter()
                .any(|entry| entry.part_of_speech == ExpressionPartOfSpeechIR::Verb)
        })
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn plan_action_concept(
    language: LanguageCodeIR,
    intent: PlanIntentIR,
    predicate: &str,
) -> String {
    resolve_plan_action_concept(
        &ExpressionNodeStore::bilingual_builtin(),
        language,
        intent,
        Some(predicate),
    )
}

/// Realize a report whose factual body is bound to verbatim user-provided
/// source clauses.  This is deliberately narrower than paraphrase: the
/// caller has already decided that the request is a deictic transformation of
/// the source, and this function changes only presentation scaffolding.  The
/// source clauses remain an indivisible expression node so the generation
/// trace, morphology replay and source hashes all agree on the exact text.
pub(crate) fn generate_source_bound_report_from_knowledge(
    settings: impl Into<GenerationSettings>,
    report_surface: &str,
    source_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    if report_surface.trim().is_empty()
        || report_surface.chars().count() > MAX_REALIZED_CHARS
        || source_refs.is_empty()
        // Source-bound reports retain one receipt per source sentence plus
        // the transformation request. The caller's bounded report contract
        // permits 128 source sentences, the full-document hash, and the
        // transformation request, so 130 provenance references are valid.
        || source_refs.len() > 130
        || source_refs.iter().any(|reference| reference.trim().is_empty())
    {
        return Err("INVALID_SOURCE_BOUND_REPORT".to_string());
    }
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    expressions.attach_alias(
        match language {
            LanguageCodeIR::Korean => "EXPR.KO.SOURCE_BOUND_REPORT",
            _ => "EXPR.EN.SOURCE_BOUND_REPORT",
        },
        language,
        "C_SOURCE_BOUND_REPORT",
        report_surface,
        ExpressionPartOfSpeechIR::Verb,
        "RUNTIME_SOURCE_BOUND_REPORT:VERBATIM_USER_EVIDENCE",
    )?;
    let mut grounding_refs = source_refs.to_vec();
    grounding_refs.sort();
    grounding_refs.dedup();
    let meaning = GenerationMeaningGraphIR::new(
        vec![GenerationMeaningNodeIR {
            node_id: "E_SOURCE_BOUND_REPORT".to_string(),
            concept_id: "C_SOURCE_BOUND_REPORT".to_string(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs,
        }],
        Vec::new(),
    );
    settings.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Neutral,
            tense: GenerationTenseIR::SourcePreserved,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        },
        expressions: &expressions,
    })
}

/// Content selection happens before surface generation. Action traces need not
/// each repeat the lifecycle boundary owned by their enclosing response.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanPreviewContentIR {
    Detailed,
    DetailedAction,
    Compact,
    ActionOnly,
}

pub(crate) fn generate_plan_preview_with_predicate(
    settings: impl Into<GenerationSettings>,
    subject: &str,
    intent: PlanIntentIR,
    grounding_ref: &str,
    directive_ref: Option<&str>,
    content: PlanPreviewContentIR,
    predicate: Option<&str>,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    if subject.trim().is_empty() || grounding_ref.trim().is_empty() {
        return Err("INVALID_PLAN_PREVIEW_REQUEST".to_string());
    }
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let action_concept = resolve_plan_action_concept(&expressions, language, intent, predicate);
    // An investigation plan must retain the question it is meant to resolve.
    // Predicate selection can choose a more idiomatic investigation verb, but
    // it must not erase the causal target carried by the plan intent.
    let narrow_cause = intent == PlanIntentIR::Investigate || action_concept == "C_NARROW";
    let node = |node_id: &str, concept_id: &str, kind| {
        let mut grounding_refs = vec![format!("PLAN_INTENT:{intent:?}"), grounding_ref.to_string()];
        if let Some(predicate) = predicate {
            grounding_refs.push(format!("PLAN_PREDICATE:{predicate}"));
        }
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
            &action_concept,
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
    if narrow_cause {
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
    if narrow_cause {
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
    if content != PlanPreviewContentIR::Detailed {
        let mut retained = ["E_ACTION", "R_SUBJECT", "R_CAUSE"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        match content {
            PlanPreviewContentIR::Compact => {
                retained.extend(["E_EXECUTED", "R_WORK", "Q_EXECUTED", "N_NOT"]);
                edges.push(meaning_edge(
                    "M.COMPACT.BOUNDARY",
                    "E_ACTION",
                    "E_EXECUTED",
                    GenerationMeaningRelationIR::Sequence,
                ));
            }
            PlanPreviewContentIR::DetailedAction => {
                retained.extend(["E_OBSERVE", "E_VERIFY", "R_CURRENT_STATE", "R_RESULT"]);
            }
            PlanPreviewContentIR::ActionOnly | PlanPreviewContentIR::Detailed => {}
        }
        nodes.retain(|node| retained.contains(node.node_id.as_str()));
        edges.retain(|edge| {
            retained.contains(edge.source_node_id.as_str())
                && retained.contains(edge.target_node_id.as_str())
        });
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
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
    settings.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Future,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::DescribePlan,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_plan_exclusion_from_knowledge(
    settings: impl Into<GenerationSettings>,
    subject: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    kind: GenerationPlanInterpretationKindIR,
    primary_surface: &str,
    secondary_surface: Option<&str>,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    subject: &str,
    claims: &[GenerationLifecycleClaimIR],
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    generate_lifecycle_status_with_action(settings, subject, None, claims, grounding_ref)
}

pub(crate) fn generate_lifecycle_status_with_action(
    settings: impl Into<GenerationSettings>,
    subject: &str,
    action_predicate: Option<&str>,
    claims: &[GenerationLifecycleClaimIR],
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    let language = if language == LanguageCodeIR::Korean {
        LanguageCodeIR::Korean
    } else {
        LanguageCodeIR::English
    };
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    if let Some(predicate) = action_predicate.filter(|predicate| !predicate.trim().is_empty()) {
        let concept = format!("C_{}", predicate.strip_prefix("C_").unwrap_or(predicate));
        nodes[0].concept_id = concept.clone();
        nodes[0].kind = GenerationMeaningNodeKindIR::EventReference;
        nodes.push(GenerationMeaningNodeIR {
            node_id: "R_ACTION_TARGET".into(),
            concept_id: "C_RUNTIME_LIFECYCLE_SUBJECT".into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: vec![grounding_ref.to_string()],
        });
        edges.push(meaning_edge(
            "ACTION_REFERENCE.THEME",
            "R_ACTION",
            "R_ACTION_TARGET",
            GenerationMeaningRelationIR::Theme,
        ));
        if !expressions
            .candidates(&concept, language)
            .iter()
            .any(|entry| entry.part_of_speech == ExpressionPartOfSpeechIR::Verb)
        {
            // Preserve the unknown operation's identity. A generic reference is
            // not a guessed translation or a substitute executable capability.
            expressions.attach_alias(
                "EXPR.RUNTIME.OPAQUE_ACTION_REFERENCE",
                language,
                &concept,
                if language == LanguageCodeIR::Korean {
                    "작업"
                } else {
                    "task"
                },
                ExpressionPartOfSpeechIR::Noun,
                "OPAQUE_ACTION_REFERENCE_NO_LEXICALIZATION",
            )?;
        }
    }
    let meaning = GenerationMeaningGraphIR::new(nodes, edges);
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    selected_count: usize,
    quantifier: GenerationActionSetQuantifierIR,
    predicate: GenerationActionSetPredicateIR,
    truth: GenerationActionSetTruthIR,
    grounding_ref: &str,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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

pub(crate) fn generate_acknowledgement_from_knowledge(
    settings: impl Into<GenerationSettings>,
    grounding_refs: Vec<String>,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            vec![GenerationMeaningNodeIR {
                node_id: "ACK_EVENT".into(),
                concept_id: "C_ACKNOWLEDGE".into(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs,
            }],
            vec![],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Acknowledge,
        },
        expressions: &ExpressionNodeStore::bilingual_builtin(),
    })
}

/// Content selected by the dialogue layer before lexical realization. A receipt
/// does not certify the statement, promise persistence, or request evidence.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AcknowledgementContentIR<'a> {
    Receipt,
    OpenConversation,
    Conditional(&'a crate::modality::ConditionalRelationIR),
    State(&'a EventSummaryIR),
}

pub(crate) fn generate_inform_acknowledgement_from_knowledge(
    settings: impl Into<GenerationSettings>,
    content: AcknowledgementContentIR<'_>,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    match content {
        AcknowledgementContentIR::State(state) => {
            if state.source_actor == "DIALOGUE_USER" {
                event_summary::generate_state_acknowledgement(settings, state)
            } else {
                // A third-party report retains its source, not the direct
                // user's receipt ending or a statement of established truth.
                event_summary::generate_event_summary(settings, state)
            }
        }
        AcknowledgementContentIR::Receipt => {
            generate_acknowledgement_from_knowledge(settings, grounding_refs.to_vec())
        }
        AcknowledgementContentIR::OpenConversation => generate_dialogue_response_from_knowledge(
            settings,
            GenerationDialogueResponseKindIR::HoldFloor,
        ),
        AcknowledgementContentIR::Conditional(conditional) => {
            generate_conditional_ack(settings, conditional, grounding_refs)
        }
    }
}

pub(crate) fn generate_affect_support_from_knowledge(
    settings: impl Into<GenerationSettings>,
    affect: GenerationAffectKindIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
                "R_SITUATION",
                "C_CURRENT_SITUATION",
                GenerationMeaningNodeKindIR::Entity,
            ),
            node(
                "Q_AFFECT",
                quality_concept,
                GenerationMeaningNodeKindIR::Quality,
            ),
        ],
        vec![
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
        ],
    );
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    response: GenerationDialogueResponseKindIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
            default_speech_intent: GenerationSpeechIntentIR::Acknowledge,
        },
        expressions: &ExpressionNodeStore::bilingual_builtin(),
    })
}

pub(crate) fn generate_continuation_gate_from_knowledge(
    settings: impl Into<GenerationSettings>,
    task_surface: &str,
    benefit_surface: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    task_surface: &str,
    benefit_surface: &str,
    grounding_refs: &[String],
    followup: GenerationContinuationGateFollowupIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    feedback: GenerationUserFeedbackKindIR,
    target_surface: &str,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    operation: GenerationDiscourseGroupUpdateKindIR,
    member_count: usize,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    kind: GenerationClarificationKindIR,
    detail_surface: Option<&str>,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
            | GenerationClarificationKindIR::ResponsePreference
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    graph: &IllocutionaryCommitmentGraphIR,
    withdrawn_goal_ids: &[String],
    withdrawn_deferred_ids: &[String],
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    evaluation: &ConditionalGuardEvaluationIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    grounding: &DefinitionGroundingIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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

    settings.generate(GenerativeLanguageRequestIR {
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

fn generate_conditional_ack(
    settings: impl Into<GenerationSettings>,
    conditional: &crate::modality::ConditionalRelationIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    if conditional.consequent_is_directive
        || conditional.antecedent.trim().is_empty()
        || conditional.consequent.trim().is_empty()
    {
        return Err("INVALID_CONDITIONAL_ACKNOWLEDGEMENT".into());
    }
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    for (id, surface, kind, pos) in [
        (
            "C_CONDITIONAL_ACK",
            if language == LanguageCodeIR::Korean {
                "이해하"
            } else {
                "understand"
            },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "R_ANTECEDENT",
            conditional.antecedent.as_str(),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
        (
            "R_CONSEQUENT",
            conditional.consequent.as_str(),
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        expressions.attach_alias(
            &format!("EXPR.{id}"),
            language,
            id,
            surface,
            pos,
            "RUNTIME_REFERENT_SURFACE:CONDITIONAL_REPORT",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.into(),
            concept_id: id.into(),
            kind,
            grounding_refs: grounding_refs
                .iter()
                .cloned()
                .chain(std::iter::once(format!(
                    "CONDITIONAL_RELATION:{}:{}",
                    conditional.conditional_id,
                    content_sha256(conditional)
                )))
                .collect(),
        });
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(
            nodes,
            vec![
                meaning_edge(
                    "ANTECEDENT",
                    "C_CONDITIONAL_ACK",
                    "R_ANTECEDENT",
                    GenerationMeaningRelationIR::Theme,
                ),
                meaning_edge(
                    "CONSEQUENT",
                    "C_CONDITIONAL_ACK",
                    "R_CONSEQUENT",
                    GenerationMeaningRelationIR::Goal,
                ),
            ],
        ),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Acknowledge,
        },
        expressions: &expressions,
    })
}

fn realize_conditional_ack(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let Some(antecedent) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected)
    else {
        return output;
    };
    let Some(consequent) = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected)
    else {
        return output;
    };
    if context.language == LanguageCodeIR::Korean {
        push_expression_token(
            &mut output,
            antecedent,
            format!("‘{}", antecedent.expression.lexical_root),
        );
        push_expression_token(
            &mut output,
            consequent,
            format!(
                "{}’라는 뜻으로",
                consequent
                    .expression
                    .lexical_root
                    .trim_end_matches(['.', '!'])
            ),
        );
        push_expression_token(&mut output, predicate, "이해했어.".into());
    } else {
        push_expression_token(&mut output, predicate, "I understand: if".into());
        push_expression_token(
            &mut output,
            antecedent,
            format!("{},", antecedent.expression.lexical_root),
        );
        push_expression_token(
            &mut output,
            consequent,
            format!(
                "{}.",
                consequent
                    .expression
                    .lexical_root
                    .trim_end_matches(['.', '!'])
            ),
        );
    }
    output
}

pub(crate) fn generate_information_target_question(
    settings: impl Into<GenerationSettings>,
    kind: crate::discourse_qa::DiscourseQueryKindIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    use crate::discourse_qa::DiscourseQueryKindIR;
    let (operation, target) = match kind {
        DiscourseQueryKindIR::MissingExplanationTarget => {
            ("C_ASK_EXPLANATION_TARGET", "C_UNBOUND_TARGET")
        }
        DiscourseQueryKindIR::MissingComparisonOperands => {
            ("C_ASK_COMPARISON_TARGET", "C_UNBOUND_PAIR")
        }
        _ => return Err("NOT_AN_UNBOUND_INFORMATION_TARGET".into()),
    };
    let refs = vec![format!("UNBOUND_INFORMATION_ARGUMENT:{kind:?}")];
    let meaning = GenerationMeaningGraphIR::new(
        vec![
            GenerationMeaningNodeIR {
                node_id: "OPERATION".into(),
                concept_id: operation.into(),
                kind: GenerationMeaningNodeKindIR::Event,
                grounding_refs: refs.clone(),
            },
            GenerationMeaningNodeIR {
                node_id: "TARGET".into(),
                concept_id: target.into(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: refs,
            },
        ],
        vec![meaning_edge(
            "TARGET_ROLE",
            "OPERATION",
            "TARGET",
            GenerationMeaningRelationIR::Theme,
        )],
    );
    settings.generate(GenerativeLanguageRequestIR {
        meaning,
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Ask,
        },
        expressions: &ExpressionNodeStore::bilingual_builtin(),
    })
}

/// Restore only source-attested determiner/case morphology around the bound
/// value. A bare role value is not automatically a grammatical English phrase.
/// Multiple matching phrases cannot be silently disambiguated by realization.
fn focused_english_phrase(
    projection: &crate::proposition_content::ContentProjectionIR,
) -> Option<String> {
    use crate::proposition_content::{reported_event_surface, ContentSlotIR};
    let source = reported_event_surface(&projection.source_proposition)
        .trim()
        .trim_end_matches(['.', '?', '!']);
    let words = source.split_whitespace().collect::<Vec<_>>();
    let value = projection
        .binding
        .value
        .split_whitespace()
        .collect::<Vec<_>>();
    if value.is_empty() {
        return None;
    }
    let mut candidates = BTreeSet::new();
    for (index, span) in words.windows(value.len()).enumerate() {
        if !span
            .iter()
            .zip(&value)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
        {
            continue;
        }
        let mut start = index;
        if start > 0 && matches!(words[start - 1].to_lowercase().as_str(), "a" | "an" | "the") {
            start -= 1;
        }
        // A validated participant perspective licenses the answer's target
        // case. The source nominal keeps its determiner, but a source Agent
        // need not already carry the target Source/Recipient preposition.
        if projection.event_perspective.is_some() {
            let target_case = match projection.binding.slot {
                ContentSlotIR::Source => Some("from"),
                ContentSlotIR::Recipient => Some("to"),
                _ => None,
            };
            if let Some(case) = target_case {
                candidates.insert(format!(
                    "{case} {}",
                    words[start..index + value.len()].join(" ")
                ));
                continue;
            }
        }
        let prepositions: &[&str] = match projection.binding.slot {
            ContentSlotIR::Location => &["in", "at"],
            ContentSlotIR::Recipient => &["to"],
            ContentSlotIR::Source => &["from"],
            ContentSlotIR::Duration => &["for"],
            _ => &[],
        };
        if !prepositions.is_empty() {
            if start == 0 || !prepositions.contains(&words[start - 1].to_lowercase().as_str()) {
                continue;
            }
            start -= 1;
        }
        candidates.insert(words[start..index + value.len()].join(" "));
    }
    (candidates.len() == 1).then(|| candidates.pop_first().unwrap())
}

fn generate_event_reference_question(
    settings: GenerationSettings,
    gap: &crate::proposition_content::EventReferenceGapIR,
) -> Result<GenerativeLanguageIR, String> {
    let choices = gap.choices().ok_or("INVALID_REFERENCE_QUESTION")?;
    let predicate_concept = if gap.refers_to_person() {
        "C_EVENT_PERSON_REFERENCE_CHOICE"
    } else {
        "C_EVENT_REFERENCE_CHOICE"
    };
    let language = settings.language;
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let grounding = gap
        .contexts
        .iter()
        .map(|c| format!("DIALOGUE_BELIEF_ID:{}", c.belief_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    store.attach_alias(
        "EXPR.EVENT_REFERENCE_CHOICE",
        language,
        predicate_concept,
        if language == LanguageCodeIR::Korean {
            "선택하다"
        } else {
            "mean"
        },
        ExpressionPartOfSpeechIR::Verb,
        "GRAMMAR:REFERENCE_SELECTION_QUESTION",
    )?;
    let mut nodes = vec![GenerationMeaningNodeIR {
        node_id: "REFERENCE_CHOICE_EVENT".into(),
        concept_id: predicate_concept.into(),
        kind: GenerationMeaningNodeKindIR::Event,
        grounding_refs: grounding.clone(),
    }];
    let mut edges = Vec::new();
    for (index, (value, _)) in choices.iter().enumerate() {
        let id = format!("REFERENCE_CHOICE_VALUE_{index}");
        store.attach_alias(
            &format!("EXPR.{id}"),
            language,
            &id,
            value,
            ExpressionPartOfSpeechIR::Noun,
            "SOURCE_BOUND_REFERENCE_CANDIDATE",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.clone(),
            concept_id: id.clone(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: grounding.clone(),
        });
        edges.push(meaning_edge(
            &format!("CHOICE_{index}"),
            "REFERENCE_CHOICE_EVENT",
            &id,
            GenerationMeaningRelationIR::Property,
        ));
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Ask,
        },
        expressions: &store,
    })
}

fn realize_event_reference_question(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let values = clause
        .constituents
        .iter()
        .filter(|c| c.role == SyntaxConstituentRoleIR::Property)
        .filter_map(|c| {
            selected
                .get(&(c.expression_id.as_str(), c.meaning_node_id.as_str()))
                .copied()
        })
        .collect::<Vec<_>>();
    let person = predicate.expression.concept_id == "C_EVENT_PERSON_REFERENCE_CHOICE";
    if context.language == LanguageCodeIR::Korean {
        push_coordinated_content_values(&mut output, &values, context.language, false);
        push_expression_token(
            &mut output,
            predicate,
            if person && context.register == LanguageRegisterIR::Formal {
                "중 누구를 말씀하시는 건가요?".into()
            } else if person {
                "중 누구를 말하는 거야?".into()
            } else if context.register == LanguageRegisterIR::Formal {
                "중 어느 것을 말씀하시는 건가요?".into()
            } else {
                "중 어느 걸 말하는 거야?".into()
            },
        );
    } else {
        push_expression_token(
            &mut output,
            predicate,
            if person {
                "Who do you mean:"
            } else {
                "Which do you mean:"
            }
            .into(),
        );
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                let connector = if index + 1 == values.len() { "or" } else { "," };
                push_grammar_token(
                    &mut output,
                    connector,
                    "EN.COORDINATION.ALTERNATIVE",
                    &value.meaning_node_id,
                );
                if connector == "," {
                    output.last_mut().unwrap().attach_left = true;
                }
            }
            push_expression_token(&mut output, value, value.expression.lexical_root.clone());
        }
        push_grammar_token(
            &mut output,
            "?",
            "EN.INTERROGATIVE.PUNCTUATION",
            &predicate.meaning_node_id,
        );
        output.last_mut().unwrap().attach_left = true;
    }
    output
}

fn generate_content_answer_set(
    settings: GenerationSettings,
    projection: &crate::proposition_content::ContentProjectionIR,
    contextual: bool,
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    let korean = language == LanguageCodeIR::Korean;
    let mut values: BTreeMap<String, (String, BTreeSet<String>)> = BTreeMap::new();
    for p in projection.all_projections() {
        let surface = if korean {
            p.binding.value.clone()
        } else {
            focused_english_phrase(p).unwrap_or_else(|| p.binding.value.clone())
        };
        let entry = values
            .entry(p.binding.value.trim().to_lowercase())
            .or_insert_with(|| (surface, BTreeSet::new()));
        entry
            .1
            .insert(format!("DIALOGUE_BELIEF_ID:{}", p.belief_id));
        entry.1.insert(p.binding.grammar_evidence.clone());
    }
    generate_answer_value_set(
        settings,
        projection.binding.slot,
        values,
        (!contextual).then_some(projection.source_actor.as_str()),
        &[],
    )
}

// Shared noun-phrase coordination and copular realization. Source identity and
// event-role answers differ in selected meaning, not in handcrafted sentences.
fn generate_answer_value_set(
    settings: GenerationSettings,
    slot: crate::proposition_content::ContentSlotIR,
    values: BTreeMap<String, (String, BTreeSet<String>)>,
    source: Option<&str>,
    nominal_forms: &[crate::korean_nominal::KoreanNominalFormIR],
) -> Result<GenerativeLanguageIR, String> {
    let language = settings.language;
    let korean = language == LanguageCodeIR::Korean;
    let contextual = source.is_none();
    let source_actor = source.unwrap_or_default();
    let grounding = values
        .values()
        .flat_map(|(_, refs)| refs.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let event_concept = format!(
        "{}{:?}",
        if contextual {
            "C_CONTENT_RECALL_"
        } else {
            "C_CONTENT_FOCUS_"
        },
        slot
    );
    let source_concept = if source_actor == "DIALOGUE_USER" {
        "C_CONTENT_USER_SOURCE"
    } else {
        "CONTENT_SET_SOURCE"
    };
    for (id, concept, surface, kind, pos) in [
        (
            "CONTENT_SET_EVENT",
            event_concept.as_str(),
            if korean { "이다" } else { "is" },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "CONTENT_SET_SOURCE",
            source_concept,
            source_actor,
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        if contextual && id == "CONTENT_SET_SOURCE" {
            continue;
        }
        store.attach_alias(
            &format!("EXPR.{id}"),
            language,
            concept,
            surface,
            pos,
            "RUNTIME_REFERENT_SURFACE:CONTENT_PROJECTION",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.into(),
            concept_id: concept.into(),
            kind,
            grounding_refs: grounding.clone(),
        });
    }
    if !contextual {
        edges.push(meaning_edge(
            "SET_SOURCE",
            "CONTENT_SET_EVENT",
            "CONTENT_SET_SOURCE",
            GenerationMeaningRelationIR::Goal,
        ));
    }
    for (index, (_, (surface, refs))) in values.into_iter().enumerate() {
        let id = format!("CONTENT_SET_VALUE_{index}");
        store.attach_alias(
            &format!("EXPR.{id}"),
            language,
            &id,
            &surface,
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:CONTENT_PROJECTION",
        )?;
        store.attach_nominal_forms(&format!("EXPR.{id}"), nominal_forms)?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: id.clone(),
            concept_id: id.clone(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: refs.into_iter().collect(),
        });
        edges.push(meaning_edge(
            &format!("SET_VALUE_{index}"),
            "CONTENT_SET_EVENT",
            &id,
            GenerationMeaningRelationIR::Property,
        ));
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
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

fn interaction_mode_concept(mode: crate::proposition_content::InteractionModeIR) -> &'static str {
    use crate::proposition_content::InteractionModeIR;
    match mode {
        InteractionModeIR::Acknowledgement => "C_INTERACTION_MODE_ACKNOWLEDGEMENT",
        InteractionModeIR::Explanation => "C_INTERACTION_MODE_EXPLANATION",
        InteractionModeIR::Listening => "C_INTERACTION_MODE_LISTENING",
        InteractionModeIR::Conversation => "C_INTERACTION_MODE_CONVERSATION",
        InteractionModeIR::Concise => "C_INTERACTION_MODE_CONCISE",
        InteractionModeIR::Advice => "C_INTERACTION_MODE_ADVICE",
        InteractionModeIR::Summary => "C_INTERACTION_MODE_SUMMARY",
        InteractionModeIR::Execution => "C_INTERACTION_MODE_EXECUTION",
    }
}

fn interaction_mode_expression_root(
    mode: crate::proposition_content::InteractionModeIR,
    language: LanguageCodeIR,
) -> &'static str {
    use crate::proposition_content::InteractionModeIR;
    match (language, mode) {
        (LanguageCodeIR::Korean, InteractionModeIR::Acknowledgement) => "확인",
        (LanguageCodeIR::Korean, InteractionModeIR::Explanation) => "설명",
        (LanguageCodeIR::Korean, InteractionModeIR::Listening) => "경청",
        (LanguageCodeIR::Korean, InteractionModeIR::Conversation) => "대화",
        (LanguageCodeIR::Korean, InteractionModeIR::Concise) => "짧은 응답",
        (LanguageCodeIR::Korean, InteractionModeIR::Advice) => "해결책",
        (LanguageCodeIR::Korean, InteractionModeIR::Summary) => "요약",
        (LanguageCodeIR::Korean, InteractionModeIR::Execution) => "직접 처리",
        (_, InteractionModeIR::Acknowledgement) => "acknowledgement",
        (_, InteractionModeIR::Explanation) => "explanation",
        (_, InteractionModeIR::Listening) => "listening",
        (_, InteractionModeIR::Conversation) => "conversation",
        (_, InteractionModeIR::Concise) => "short response",
        (_, InteractionModeIR::Advice) => "solution",
        (_, InteractionModeIR::Summary) => "summary",
        (_, InteractionModeIR::Execution) => "direct action",
    }
}

fn generate_interaction_preference_answer(
    settings: GenerationSettings,
    projection: &crate::proposition_content::ContentProjectionIR,
    focus: crate::proposition_content::InteractionPreferenceAnswerFocusIR,
) -> Result<GenerativeLanguageIR, String> {
    use crate::proposition_content::InteractionPreferenceAnswerFocusIR;
    let language = settings.language;
    let mut refs = vec![
        format!("DIALOGUE_BELIEF_ID:{}", projection.belief_id),
        format!("SOURCE_ACTOR:{}", projection.source_actor),
        projection.binding.grammar_evidence.clone(),
    ];
    let (answer_concept, answer_root, rejected) = match focus {
        InteractionPreferenceAnswerFocusIR::ModeChoice { desired, rejected } => {
            refs.push(format!("INTERACTION_PREFERENCE_DESIRED:{desired:?}"));
            refs.push(format!("INTERACTION_PREFERENCE_EXCLUDED:{rejected:?}"));
            (
                interaction_mode_concept(desired),
                interaction_mode_expression_root(desired, language),
                Some(rejected),
            )
        }
        InteractionPreferenceAnswerFocusIR::ResponseLength => {
            refs.push("INTERACTION_PREFERENCE_RESPONSE_MANNER:CONCISE".into());
            (
                "C_RESPONSE_DIMENSION_LENGTH",
                if language == LanguageCodeIR::Korean {
                    "답변 길이"
                } else {
                    "response length"
                },
                None,
            )
        }
    };
    refs.sort();
    refs.dedup();
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (node_id, concept, root, kind, pos) in [
        (
            "INTERACTION_PREFERENCE_EVENT",
            "C_INTERACTION_PREFERENCE_ANSWER",
            if language == LanguageCodeIR::Korean {
                "이다"
            } else {
                "be"
            },
            GenerationMeaningNodeKindIR::Event,
            ExpressionPartOfSpeechIR::Verb,
        ),
        (
            "INTERACTION_PREFERENCE_VALUE",
            answer_concept,
            answer_root,
            GenerationMeaningNodeKindIR::Entity,
            ExpressionPartOfSpeechIR::Noun,
        ),
    ] {
        store.attach_alias(
            &format!("EXPR.{node_id}"),
            language,
            concept,
            root,
            pos,
            "SOURCE_BOUND_INTERACTION_PREFERENCE_FOCUS",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: node_id.into(),
            concept_id: concept.into(),
            kind,
            grounding_refs: refs.clone(),
        });
    }
    edges.push(meaning_edge(
        "INTERACTION_PREFERENCE_VALUE_EDGE",
        "INTERACTION_PREFERENCE_EVENT",
        "INTERACTION_PREFERENCE_VALUE",
        GenerationMeaningRelationIR::Property,
    ));
    if let Some(rejected) = rejected {
        let concept = interaction_mode_concept(rejected);
        store.attach_alias(
            "EXPR.INTERACTION_PREFERENCE_REJECTED",
            language,
            concept,
            interaction_mode_expression_root(rejected, language),
            ExpressionPartOfSpeechIR::Noun,
            "SOURCE_BOUND_INTERACTION_PREFERENCE_CONTRAST",
        )?;
        nodes.push(GenerationMeaningNodeIR {
            node_id: "INTERACTION_PREFERENCE_REJECTED".into(),
            concept_id: concept.into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: refs.clone(),
        });
        edges.push(meaning_edge(
            "INTERACTION_PREFERENCE_REJECTED_EDGE",
            "INTERACTION_PREFERENCE_EVENT",
            "INTERACTION_PREFERENCE_REJECTED",
            GenerationMeaningRelationIR::Theme,
        ));
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
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

fn generate_content_projection(
    settings: impl Into<GenerationSettings>,
    projection: &crate::proposition_content::ContentProjectionIR,
    focused: bool,
    framing: crate::discourse_qa::AnswerSourceFramingIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    use crate::proposition_content::ContentSlotIR;
    if !projection.validate() {
        return Err("INVALID_CONTENT_PROJECTION".into());
    }
    if projection.binding.slot == ContentSlotIR::Property {
        let summaries = projection
            .all_projections()
            .map(|p| {
                let content = crate::proposition_content::PropositionContentIR::compile_contextual(
                    &p.source_proposition,
                    &p.context_sources,
                )
                .ok_or("INVALID_STATE_SOURCE")?;
                let event = content
                    .events
                    .into_iter()
                    .find(|e| {
                        Some(&e.event_id) == p.binding.event_id.as_ref()
                            && e.kind == crate::proposition_content::DescriptionKindIR::State
                    })
                    .ok_or("MISSING_STATE_PROPERTY_SOURCE")?;
                Ok(event_summary::EventSummaryIR {
                    omitted_roles: vec![],
                    belief_id: p.belief_id.clone(),
                    source_actor: p.source_actor.clone(),
                    source_proposition: p.source_proposition.clone(),
                    context_sources: p.context_sources.clone(),
                    event,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        return event_summary::generate_event_summaries(settings, &summaries);
    }
    if !projection.co_answers.is_empty() {
        return generate_content_answer_set(
            settings,
            projection,
            focused && framing == crate::discourse_qa::AnswerSourceFramingIR::SharedDialogueRecall,
        );
    }
    let korean = language == LanguageCodeIR::Korean;
    let english_phrase = (!korean && focused)
        .then(|| focused_english_phrase(projection))
        .flatten();
    let focused = focused && (korean || english_phrase.is_some());
    let contextual =
        focused && framing == crate::discourse_qa::AnswerSourceFramingIR::SharedDialogueRecall;
    let actor = if projection.source_actor == "DIALOGUE_USER" {
        if korean {
            "네 말"
        } else {
            "your account"
        }
    } else {
        &projection.source_actor
    };
    let mut store = ExpressionNodeStore::bilingual_builtin();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (index, binding) in projection.all_bindings().enumerate() {
        let slot = content_slot_label(binding.slot, korean, true);
        let mut grounding = vec![
            format!("DIALOGUE_BELIEF_ID:{}", projection.belief_id),
            binding.grammar_evidence.clone(),
        ];
        if let Some(event_id) = &binding.event_id {
            grounding.push(format!("SOURCE_EVENT:{event_id}"));
        }
        if let Some(proof) = &projection.event_perspective {
            grounding.push(format!(
                "EVENT_ROLE_PERSPECTIVE:{}:{}:{}",
                proof.relation_id, proof.source_perspective, proof.target_perspective
            ));
        }
        let event = format!("CONTENT_EVENT_{index}");
        let focused_concept = format!(
            "{}{:?}",
            if contextual {
                "C_CONTENT_RECALL_"
            } else {
                "C_CONTENT_FOCUS_"
            },
            binding.slot
        );
        let slot_concept = format!("C_CONTENT_SLOT_{:?}", binding.slot);
        for (role, surface, kind, pos) in [
            (
                "EVENT",
                if korean { "이다" } else { "is" },
                GenerationMeaningNodeKindIR::Event,
                ExpressionPartOfSpeechIR::Verb,
            ),
            (
                "VALUE",
                english_phrase.as_deref().unwrap_or(binding.value.as_str()),
                GenerationMeaningNodeKindIR::Entity,
                ExpressionPartOfSpeechIR::Noun,
            ),
            (
                "SLOT",
                slot,
                GenerationMeaningNodeKindIR::Entity,
                ExpressionPartOfSpeechIR::Noun,
            ),
            (
                "SOURCE",
                actor,
                GenerationMeaningNodeKindIR::Entity,
                ExpressionPartOfSpeechIR::Noun,
            ),
        ] {
            // The query supplies this role as given information. Its identity
            // remains in the focused predicate; no silent, uncovered node is
            // added merely to suppress its surface later.
            if focused && role == "SLOT" || contextual && role == "SOURCE" {
                continue;
            }
            let id = format!("CONTENT_{role}_{index}");
            let concept = if role == "EVENT" {
                if focused {
                    &focused_concept
                } else {
                    "C_CONTENT_PROJECTION"
                }
            } else if role == "VALUE" && binding.grammar_evidence == "DESIDERATIVE_COMPLEMENT" {
                // The source parser licensed a proposition-sized desiderative
                // complement. Preserve that category through generation so a
                // finite Korean clause is quoted instead of treated as a noun.
                "C_CONTENT_EMBEDDED_CLAUSE"
            } else if role == "SLOT" {
                &slot_concept
            } else if role == "SOURCE" && projection.source_actor == "DIALOGUE_USER" {
                "C_CONTENT_USER_SOURCE"
            } else {
                &id
            };
            store.attach_alias(
                &format!("EXPR.CONTENT.{id}"),
                language,
                concept,
                surface,
                pos,
                "RUNTIME_REFERENT_SURFACE:CONTENT_PROJECTION",
            )?;
            nodes.push(GenerationMeaningNodeIR {
                node_id: id.clone(),
                concept_id: concept.to_string(),
                kind,
                grounding_refs: grounding.clone(),
            });
        }
        for (role, relation) in [
            ("VALUE", GenerationMeaningRelationIR::Property),
            ("SLOT", GenerationMeaningRelationIR::Theme),
            ("SOURCE", GenerationMeaningRelationIR::Goal),
        ] {
            if focused && role == "SLOT" || contextual && role == "SOURCE" {
                continue;
            }
            edges.push(meaning_edge(
                &format!("{role}_{index}"),
                &event,
                &format!("CONTENT_{role}_{index}"),
                relation,
            ));
        }
        if index > 0 {
            edges.push(meaning_edge(
                &format!("ORDER_{index}"),
                &format!("CONTENT_EVENT_{}", index - 1),
                &event,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
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

/// Coordinate grounded constituents, never preassembled answer sentences.
/// Every lexical token retains its own node; conjunctions are grammar tokens.
fn push_coordinated_content_values(
    output: &mut Vec<MorphologicalTokenIR>,
    values: &[&ExpressionSelectionIR],
    language: LanguageCodeIR,
    capitalize_first: bool,
) {
    for (index, value) in values.iter().enumerate() {
        let mut surface = if language == LanguageCodeIR::English {
            english_positioned_nominal(
                &value.expression.lexical_root,
                index == 0 && capitalize_first,
            )
        } else {
            value.expression.lexical_root.clone()
        };
        if language != LanguageCodeIR::English && index == 0 && capitalize_first {
            let mut chars = surface.chars();
            surface = chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default();
        }
        push_expression_token(output, value, surface);
        if index + 1 == values.len() {
            continue;
        }
        if language == LanguageCodeIR::Korean && index + 2 == values.len() {
            push_grammar_token(
                output,
                nominal_suffix(&value.expression, "과", "와").unwrap_or("하고"),
                "KO.COORDINATION.NOMINAL.AND",
                &value.meaning_node_id,
            );
            output.last_mut().unwrap().attach_left = true;
        } else if index + 2 < values.len() {
            push_grammar_token(
                output,
                ",",
                "COORDINATION.LIST.COMMA",
                &value.meaning_node_id,
            );
            output.last_mut().unwrap().attach_left = true;
        } else {
            push_grammar_token(
                output,
                "and",
                "EN.COORDINATION.NOMINAL.AND",
                &value.meaning_node_id,
            );
        }
    }
}

fn korean_interaction_mode_answer_surface(
    concept: &str,
    register: LanguageRegisterIR,
    rejected: bool,
) -> Option<&'static str> {
    if rejected {
        return match concept {
            "C_INTERACTION_MODE_ACKNOWLEDGEMENT" => Some("확인"),
            "C_INTERACTION_MODE_EXPLANATION" => Some("설명"),
            "C_INTERACTION_MODE_LISTENING" => Some("들어주는 것"),
            "C_INTERACTION_MODE_CONVERSATION" => Some("대화"),
            "C_INTERACTION_MODE_CONCISE" => Some("짧은 대답"),
            "C_INTERACTION_MODE_ADVICE" => Some("해결책"),
            "C_INTERACTION_MODE_SUMMARY" => Some("요약"),
            "C_INTERACTION_MODE_EXECUTION" => Some("직접 처리"),
            _ => None,
        };
    }
    match (concept, register) {
        ("C_INTERACTION_MODE_ACKNOWLEDGEMENT", _) => Some("확인해 주는 쪽"),
        ("C_INTERACTION_MODE_EXPLANATION", _) => Some("설명하는 쪽"),
        ("C_INTERACTION_MODE_LISTENING", LanguageRegisterIR::Formal) => {
            Some("말씀을 들어드리는 쪽")
        }
        ("C_INTERACTION_MODE_LISTENING", _) => Some("네 이야기를 들어주는 쪽"),
        ("C_INTERACTION_MODE_CONVERSATION", _) => Some("같이 이야기하는 쪽"),
        ("C_INTERACTION_MODE_CONCISE", _) => Some("짧게 답하는 쪽"),
        ("C_INTERACTION_MODE_ADVICE", LanguageRegisterIR::Formal) => Some("해결책을 드리는 쪽"),
        ("C_INTERACTION_MODE_ADVICE", _) => Some("해결책을 주는 쪽"),
        ("C_INTERACTION_MODE_SUMMARY", _) => Some("요약하는 쪽"),
        ("C_INTERACTION_MODE_EXECUTION", _) => Some("직접 처리하는 쪽"),
        _ => None,
    }
}

fn english_interaction_mode_answer_surface(concept: &str, rejected: bool) -> Option<&'static str> {
    if rejected {
        return match concept {
            "C_INTERACTION_MODE_ACKNOWLEDGEMENT" => Some("acknowledgement"),
            "C_INTERACTION_MODE_EXPLANATION" => Some("an explanation"),
            "C_INTERACTION_MODE_LISTENING" => Some("being heard"),
            "C_INTERACTION_MODE_CONVERSATION" => Some("a conversation"),
            "C_INTERACTION_MODE_CONCISE" => Some("a short answer"),
            "C_INTERACTION_MODE_ADVICE" => Some("a solution"),
            "C_INTERACTION_MODE_SUMMARY" => Some("a summary"),
            "C_INTERACTION_MODE_EXECUTION" => Some("direct action"),
            _ => None,
        };
    }
    match concept {
        "C_INTERACTION_MODE_ACKNOWLEDGEMENT" => Some("to be acknowledged"),
        "C_INTERACTION_MODE_EXPLANATION" => Some("an explanation"),
        "C_INTERACTION_MODE_LISTENING" => Some("someone to listen to you"),
        "C_INTERACTION_MODE_CONVERSATION" => Some("to talk together"),
        "C_INTERACTION_MODE_CONCISE" => Some("a short answer"),
        "C_INTERACTION_MODE_ADVICE" => Some("a solution"),
        "C_INTERACTION_MODE_SUMMARY" => Some("a summary"),
        "C_INTERACTION_MODE_EXECUTION" => Some("direct action"),
        _ => None,
    }
}

fn realize_interaction_preference_answer(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let rejected = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected);
    let Some(answer) = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
    else {
        return output;
    };
    if context.language == LanguageCodeIR::Korean {
        if let Some(rejected) = rejected {
            let Some(surface) = korean_interaction_mode_answer_surface(
                &rejected.expression.concept_id,
                context.register,
                true,
            ) else {
                return output;
            };
            push_expression_token(&mut output, rejected, format!("{surface}보다는"));
        }
        let surface = if answer.expression.concept_id == "C_RESPONSE_DIMENSION_LENGTH" {
            "답변 길이"
        } else {
            let Some(surface) = korean_interaction_mode_answer_surface(
                &answer.expression.concept_id,
                context.register,
                false,
            ) else {
                return Vec::new();
            };
            surface
        };
        push_expression_token(&mut output, answer, surface.to_string());
        push_expression_token(
            &mut output,
            predicate,
            if context.register == LanguageRegisterIR::Formal {
                "입니다.".into()
            } else {
                korean_particle(surface, "이야.", "야.").into()
            },
        );
        output.last_mut().unwrap().attach_left = true;
    } else {
        if answer.expression.concept_id == "C_RESPONSE_DIMENSION_LENGTH" {
            push_grammar_token(
                &mut output,
                "The thing to reduce is",
                "EN.INTERACTION_PREFERENCE.REDUCTION_TARGET",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, answer, "the response length".into());
        } else {
            let Some(surface) =
                english_interaction_mode_answer_surface(&answer.expression.concept_id, false)
            else {
                return output;
            };
            push_grammar_token(
                &mut output,
                "You want",
                "EN.INTERACTION_PREFERENCE.DESIRED",
                &clause.event_node_id,
            );
            push_expression_token(&mut output, answer, surface.into());
            if let Some(rejected) = rejected {
                let Some(surface) =
                    english_interaction_mode_answer_surface(&rejected.expression.concept_id, true)
                else {
                    return Vec::new();
                };
                push_grammar_token(
                    &mut output,
                    "rather than",
                    "EN.INTERACTION_PREFERENCE.CONTRAST",
                    &rejected.meaning_node_id,
                );
                push_expression_token(&mut output, rejected, surface.into());
            }
        }
        push_expression_token(&mut output, predicate, ".".into());
        output.last_mut().unwrap().attach_left = true;
    }
    output
}

fn realize_content_projection(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
) -> Vec<MorphologicalTokenIR> {
    realize_content_projection_chain(clause, context, selected, predicate, (false, false))
}

/// Parallel answer clauses may share attribution only when their semantic
/// sequence, source belief, event and speaker all agree. Equal wording alone
/// is not evidence that two reports have the same scope.
fn content_projection_joined(
    left: &SyntaxClauseIR,
    right: &SyntaxClauseIR,
    meaning: &GenerationMeaningGraphIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
) -> bool {
    let eligible = |c: &SyntaxClauseIR| {
        c.speech_intent == GenerationSpeechIntentIR::Inform
            && constituent_selection(c, SyntaxConstituentRoleIR::Predicate, selected)
                .is_some_and(|p| p.expression.concept_id == "C_CONTENT_PROJECTION")
    };
    let anchors = |c: &SyntaxClauseIR| {
        meaning
            .nodes
            .iter()
            .find(|n| n.node_id == c.event_node_id)
            .map(|n| {
                n.grounding_refs
                    .iter()
                    .filter(|r| {
                        r.starts_with("DIALOGUE_BELIEF_ID:") || r.starts_with("SOURCE_EVENT:")
                    })
                    .collect::<BTreeSet<_>>()
            })
    };
    let sources = (
        constituent_selection(left, SyntaxConstituentRoleIR::Goal, selected),
        constituent_selection(right, SyntaxConstituentRoleIR::Goal, selected),
    );
    eligible(left)
        && eligible(right)
        && matches!(sources, (Some(a), Some(b)) if a.expression.concept_id == b.expression.concept_id
            && a.expression.lexical_root == b.expression.lexical_root)
        && anchors(left).is_some_and(|a| a.len() == 2 && anchors(right).as_ref() == Some(&a))
        && meaning.edges.iter().any(|e| {
            e.relation == GenerationMeaningRelationIR::Sequence
                && e.source_node_id == left.event_node_id
                && e.target_node_id == right.event_node_id
        })
}

fn realize_content_projection_chain(
    clause: &SyntaxClauseIR,
    context: &GenerationContextIR,
    selected: &BTreeMap<(&str, &str), &ExpressionSelectionIR>,
    predicate: &ExpressionSelectionIR,
    chain: (bool, bool),
) -> Vec<MorphologicalTokenIR> {
    let mut output = Vec::new();
    let source = constituent_selection(clause, SyntaxConstituentRoleIR::Goal, selected);
    let contextual = predicate
        .expression
        .concept_id
        .starts_with("C_CONTENT_RECALL_");
    if !contextual && source.is_none() {
        return output;
    }
    let Some(value) = constituent_selection(clause, SyntaxConstituentRoleIR::Property, selected)
    else {
        return output;
    };
    if predicate
        .expression
        .concept_id
        .starts_with("C_CONTENT_FOCUS_")
        || contextual
    {
        let values = clause
            .constituents
            .iter()
            .filter(|c| c.role == SyntaxConstituentRoleIR::Property)
            .filter_map(|c| {
                selected
                    .get(&(c.expression_id.as_str(), c.meaning_node_id.as_str()))
                    .copied()
            })
            .collect::<Vec<_>>();
        let last_value = values.last().copied().unwrap_or(value);
        if context.language == LanguageCodeIR::Korean {
            if let Some(source) = source {
                push_expression_token(
                    &mut output,
                    source,
                    if source.expression.concept_id == "C_CONTENT_USER_SOURCE" {
                        if context.register == LanguageRegisterIR::Formal {
                            "말씀하신 내용으로는".into()
                        } else {
                            "네 말로는".into()
                        }
                    } else {
                        format!("{}에 따르면,", source.expression.lexical_root)
                    },
                );
            }
            push_coordinated_content_values(&mut output, &values, context.language, false);
            let ending = if context.register == LanguageRegisterIR::Formal {
                "입니다."
            } else {
                nominal_suffix(&last_value.expression, "이야.", "야.").unwrap_or(".")
            };
            push_expression_token(&mut output, predicate, ending.into());
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
        } else {
            push_coordinated_content_values(&mut output, &values, context.language, true);
            if let Some(source) = source {
                push_expression_token(
                    &mut output,
                    source,
                    if source.expression.concept_id == "C_CONTENT_USER_SOURCE" {
                        ", from what you told me".into()
                    } else {
                        format!(
                            ", according to {}",
                            english_embedded_nominal(&source.expression.lexical_root)
                        )
                    },
                );
                if let Some(token) = output.last_mut() {
                    token.attach_left = true;
                }
            }
            push_expression_token(&mut output, predicate, ".".into());
            if let Some(token) = output.last_mut() {
                token.attach_left = true;
            }
        }
        return output;
    }
    let Some(source) = source else {
        return output;
    };
    let Some(slot) = constituent_selection(clause, SyntaxConstituentRoleIR::Theme, selected) else {
        return output;
    };
    if context.language == LanguageCodeIR::Korean {
        if chain.0 {
            push_grammar_token(
                &mut output,
                "",
                "ATTRIBUTION.SHARED_SOURCE_SCOPE",
                &source.meaning_node_id,
            );
        } else {
            push_expression_token(
                &mut output,
                source,
                if source.expression.concept_id == "C_CONTENT_USER_SOURCE" {
                    if context.register == LanguageRegisterIR::Formal {
                        "말씀하신 내용에 따르면,".into()
                    } else {
                        "네 말에 따르면,".into()
                    }
                } else {
                    format!("{}에 따르면,", source.expression.lexical_root)
                },
            );
        }
        let slot_surface = if slot.expression.concept_id == "C_CONTENT_SLOT_Intention"
            && source.expression.concept_id == "C_CONTENT_USER_SOURCE"
        {
            if context.register == LanguageRegisterIR::Formal {
                "원하시는 것은".into()
            } else {
                "네가 원하는 건".into()
            }
        } else {
            format!(
                "{}{}",
                slot.expression.lexical_root,
                korean_particle(&slot.expression.lexical_root, "은", "는")
            )
        };
        push_expression_token(&mut output, slot, slot_surface);
        push_expression_token(
            &mut output,
            value,
            format!("‘{}’", value.expression.lexical_root),
        );
        let embedded_clause = value.expression.concept_id == "C_CONTENT_EMBEDDED_CLAUSE";
        let unknown_korean_coda =
            crate::korean_nominal::surface_coda(&value.expression.lexical_root).is_none();
        let ending = if chain.1 && embedded_clause {
            "라는 것이고,"
        } else if chain.1 && unknown_korean_coda {
            "라고 되어 있고,"
        } else if chain.1 {
            "이고,"
        } else if embedded_clause && context.register == LanguageRegisterIR::Formal {
            "라는 것입니다."
        } else if embedded_clause {
            "라는 거야."
        } else if unknown_korean_coda && context.register == LanguageRegisterIR::Formal {
            "라고 되어 있습니다."
        } else if unknown_korean_coda {
            "라고 되어 있어."
        } else if context.register == LanguageRegisterIR::Formal {
            "입니다."
        } else {
            korean_particle(&value.expression.lexical_root, "이야.", "야.")
        };
        push_expression_token(&mut output, predicate, ending.to_string());
        if let Some(token) = output.last_mut() {
            token.attach_left = true;
        }
    } else {
        if chain.0 {
            push_grammar_token(
                &mut output,
                "and",
                "EN.COORDINATION.CLAUSE.AND",
                &clause.event_node_id,
            );
            push_grammar_token(
                &mut output,
                "",
                "ATTRIBUTION.SHARED_SOURCE_SCOPE",
                &source.meaning_node_id,
            );
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
                format!(
                    "{},",
                    english_embedded_nominal(&source.expression.lexical_root)
                ),
            );
        }
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
            format!(
                "‘{}’{}",
                value.expression.lexical_root,
                if chain.1 { "," } else { "." }
            ),
        );
    }
    output
}

/// Shared lexical knowledge for both known content and an unfilled content slot.
/// Reporting a value and not knowing it use the same relation identity.
fn content_slot_label(
    slot: crate::proposition_content::ContentSlotIR,
    korean: bool,
    reported: bool,
) -> &'static str {
    use crate::proposition_content::ContentSlotIR;
    match (korean, slot) {
        (true, ContentSlotIR::Agent) => "사람",
        (false, ContentSlotIR::Agent) => "actor",
        (true, ContentSlotIR::Theme) => "대상",
        (false, ContentSlotIR::Theme) => "object",
        (true, ContentSlotIR::Property) => "상태",
        (false, ContentSlotIR::Property) => "state",
        (true, ContentSlotIR::Recipient) => "받는 사람",
        (false, ContentSlotIR::Recipient) => "recipient",
        (true, ContentSlotIR::Source) => "출처",
        (false, ContentSlotIR::Source) => "source",
        (true, ContentSlotIR::Location) => "장소",
        (false, ContentSlotIR::Location) => "location",
        (true, ContentSlotIR::Time) => "시점",
        (false, ContentSlotIR::Time) => "time",
        (true, ContentSlotIR::Duration) => "기간",
        (false, ContentSlotIR::Duration) => "duration",
        (true, ContentSlotIR::Cause) => {
            if reported {
                "말한 이유"
            } else {
                "이유"
            }
        }
        (false, ContentSlotIR::Cause) => {
            if reported {
                "stated reason"
            } else {
                "reason"
            }
        }
        (true, ContentSlotIR::Definition) => "정의",
        (false, ContentSlotIR::Definition) => "definition",
        (true, ContentSlotIR::Summary) => "요점",
        (false, ContentSlotIR::Summary) => "summary",
        (true, ContentSlotIR::Manner) => {
            if reported {
                "방식"
            } else {
                "방법"
            }
        }
        (false, ContentSlotIR::Manner) => "method",
        (true, ContentSlotIR::Intention) => {
            if reported {
                "원하는 것"
            } else {
                "의도"
            }
        }
        (false, ContentSlotIR::Intention) => {
            if reported {
                "stated wish"
            } else {
                "intention"
            }
        }
        (true, ContentSlotIR::Condition) => "조건",
        (false, ContentSlotIR::Condition) => "condition",
    }
}

/// Summarize only steps actually present in the retained PlanIR. The selection
/// is a content projection (not a new plan); every verb remains under DescribePlan.
fn generate_recorded_plan_method(
    settings: GenerationSettings,
    method: &crate::discourse_qa::PlanMethodAnswerIR,
) -> Result<GenerativeLanguageIR, String> {
    use dockable_semantic_core::PlanOperationIR;
    let plan = method
        .recorded
        .bundle
        .plans
        .get(method.selected_index)
        .ok_or("INVALID_RECORDED_PLAN_INDEX")?;
    let subject = &method.recorded.discourse_goals[method.selected_index].subject;
    let mut expressions = ExpressionNodeStore::bilingual_builtin();
    let language = settings.language;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut previous: Option<String> = None;
    for step in &plan.steps {
        let (verb, ko_verb, en_verb, object, ko_object, en_object, possessive) =
            match step.operation {
                PlanOperationIR::ObserveCurrentState => (
                    "C_METHOD_OBSERVE",
                    "확인하",
                    "check",
                    "C_METHOD_STATE",
                    "현재 상태",
                    "current state",
                    true,
                ),
                PlanOperationIR::GenerateCompetingHypotheses => (
                    "C_METHOD_HYPOTHESES",
                    "구성하",
                    "formulate",
                    "C_METHOD_ALTERNATIVES",
                    "여러 가설",
                    "alternative hypotheses",
                    false,
                ),
                PlanOperationIR::RunDiagnostic => (
                    "C_METHOD_DIAGNOSE",
                    "검사하",
                    "test",
                    "C_METHOD_TARGET",
                    subject.as_str(),
                    subject.as_str(),
                    false,
                ),
                PlanOperationIR::ValidateCandidates => (
                    "C_METHOD_VALIDATE",
                    "검증하",
                    "validate",
                    "C_METHOD_CANDIDATES",
                    "후보",
                    "candidates",
                    false,
                ),
                PlanOperationIR::ApplySelectedAction => (
                    "C_METHOD_APPLY",
                    "적용하",
                    "apply",
                    "C_METHOD_SELECTED_ACTION",
                    "선택한 조치",
                    "selected action",
                    false,
                ),
                PlanOperationIR::SynthesizeExplanation => (
                    "C_METHOD_EXPLAIN",
                    "구성하",
                    "construct",
                    "C_METHOD_EXPLANATION",
                    "설명",
                    "explanation",
                    false,
                ),
                PlanOperationIR::VerifyOutcome => (
                    "C_METHOD_VERIFY",
                    "검증하",
                    "verify",
                    "C_METHOD_OUTCOME",
                    "결과",
                    "outcome",
                    false,
                ),
                _ => continue,
            };
        let event_id = format!("METHOD.{}", step.step_id);
        let object_id = format!("{event_id}.OBJECT");
        let refs = vec![format!(
            "RECORDED_PLAN_STEP:{}:{}:{:?}",
            plan.plan_sha256, step.step_id, step.operation
        )];
        nodes.push(GenerationMeaningNodeIR {
            node_id: event_id.clone(),
            concept_id: verb.into(),
            kind: GenerationMeaningNodeKindIR::Event,
            grounding_refs: refs.clone(),
        });
        nodes.push(GenerationMeaningNodeIR {
            node_id: object_id.clone(),
            concept_id: object.into(),
            kind: GenerationMeaningNodeKindIR::Entity,
            grounding_refs: refs.clone(),
        });
        expressions.inject(ExpressionNodeIR {
            expression_id: format!("EXPR.{language:?}.{verb}"),
            language,
            concept_id: verb.into(),
            lexical_root: if language == LanguageCodeIR::Korean {
                ko_verb
            } else {
                en_verb
            }
            .into(),
            part_of_speech: ExpressionPartOfSpeechIR::Verb,
            morphology: if language == LanguageCodeIR::Korean {
                ExpressionMorphologyClassIR::KoreanHada
            } else {
                ExpressionMorphologyClassIR::EnglishRegular
            },
            preferred_emotion: None,
            preferred_korean_dialect: None,
            preferred_roleplay_relationship: None,
            preferred_roleplay_voice: None,
            korean_nominal_forms: Vec::new(),
            register: LanguageRegisterIR::Neutral,
            confidence_millis: 1000,
            provenance: "SUPPLIED_PLAN_OPERATION_LEXICAL_AND_MORPHOLOGICAL_KNOWLEDGE".into(),
        })?;
        expressions.attach_alias(
            &format!("EXPR.{language:?}.{object}"),
            language,
            object,
            if language == LanguageCodeIR::Korean {
                ko_object
            } else {
                en_object
            },
            ExpressionPartOfSpeechIR::Noun,
            "RUNTIME_REFERENT_SURFACE:RECORDED_PLAN_OBJECT",
        )?;
        edges.push(meaning_edge(
            &format!("{event_id}.THEME"),
            &event_id,
            &object_id,
            GenerationMeaningRelationIR::Theme,
        ));
        if possessive {
            let owner_id = format!("{event_id}.OWNER");
            nodes.push(GenerationMeaningNodeIR {
                node_id: owner_id.clone(),
                concept_id: "C_METHOD_OWNER".into(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: refs,
            });
            expressions.attach_alias(
                &format!("EXPR.{language:?}.METHOD_OWNER"),
                language,
                "C_METHOD_OWNER",
                subject,
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:RECORDED_PLAN_TARGET",
            )?;
            edges.push(meaning_edge(
                &format!("{event_id}.POSSESSOR"),
                &object_id,
                &owner_id,
                GenerationMeaningRelationIR::Possessor,
            ));
        }
        if let Some(prior) = previous {
            edges.push(meaning_edge(
                &format!("{event_id}.ORDER"),
                &prior,
                &event_id,
                GenerationMeaningRelationIR::Sequence,
            ));
        }
        previous = Some(event_id);
    }
    settings.generate(GenerativeLanguageRequestIR {
        meaning: GenerationMeaningGraphIR::new(nodes, edges),
        context: GenerationContextIR {
            language,
            register: LanguageRegisterIR::Informal,
            tense: GenerationTenseIR::Future,
            emotion: GenerationEmotionIR::Neutral,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::DescribePlan,
        },
        expressions: &expressions,
    })
}

pub(crate) fn generate_discourse_answer_from_knowledge(
    settings: impl Into<GenerationSettings>,
    answer: &DiscourseAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
    if !answer.validate() {
        return Err("INVALID_DISCOURSE_ANSWER_GENERATION_SOURCE".to_string());
    }
    if let Some(method) = &answer.plan_method {
        return generate_recorded_plan_method(settings, method);
    }
    if let Some(gap) = &answer.reference_gap {
        return generate_event_reference_question(settings, gap);
    }
    if let Some(world) = &answer.world_reasoning {
        return generate_world_decision(settings, world);
    }
    if let Some(inquiry) = &answer.decision_inquiry {
        return generate_decision_inquiry(settings, inquiry);
    }
    if let Some(c) = &answer.world_clarification {
        return generate_world_clarification(settings, c);
    }
    if let Some(update) = &answer.world_memory_update {
        return generate_world_memory_update(settings, update);
    }
    if matches!(
        answer.query.kind,
        DiscourseQueryKindIR::MissingExplanationTarget
            | DiscourseQueryKindIR::MissingComparisonOperands
    ) {
        return generate_information_target_question(settings, answer.query.kind);
    }
    if let Some(projection) = &answer.content_projection {
        if let Some(focus) =
            crate::proposition_content::interaction_preference(&projection.source_proposition)
                .as_ref()
                .and_then(|preference| {
                    crate::proposition_content::interaction_preference_answer_focus(
                        &answer.query.original_text,
                        preference,
                    )
                })
        {
            return generate_interaction_preference_answer(settings, projection, focus);
        }
        if projection.elaboration_event.is_some() {
            let summaries = projection
                .all_projections()
                .map(|p| {
                    Ok(EventSummaryIR {
                        omitted_roles: p.elaboration_omitted_roles.clone(),
                        belief_id: p.belief_id.clone(),
                        source_actor: p.source_actor.clone(),
                        source_proposition: p.source_proposition.clone(),
                        context_sources: p.context_sources.clone(),
                        event: p
                            .elaboration_event
                            .clone()
                            .ok_or("INCOMPLETE_ANSWER_ELABORATION")?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            return event_summary::generate_event_summaries(settings, &summaries);
        }
        // Ellipsis is licensed by a single validated role gap, not by a
        // particular input sentence or a preferred expected answer.
        if let Some(clause) =
            event_summary::role_answer_clause(projection, &answer.query.original_text, language)
        {
            return generate_event_summary(settings, &clause);
        }
        let focused = (projection.binding.event_id.is_some()
            || answer.source_framing()
                == crate::discourse_qa::AnswerSourceFramingIR::SharedDialogueRecall)
            && projection.additional_bindings.is_empty()
            && crate::proposition_content::requested_content_slots(&answer.query.original_text)
                == [projection.binding.slot]
            && answer
                .question_request
                .as_ref()
                .and_then(|q| q.response_manner)
                != Some(crate::proposition_content::ResponseMannerIR::Detailed);
        return generate_content_projection(settings, projection, focused, answer.source_framing());
    }
    if let Some(summary) = &answer.event_summary {
        return generate_event_summary(settings, summary);
    }
    if let Some(contents) = answer
        .spoken_content()
        .filter(|contents| contents.iter().all(|s| s.can_realize(language)))
    {
        return event_summary::generate_event_summaries(settings, &contents);
    }
    if let Some(sources) = answer.focused_source_values() {
        let values = sources
            .into_iter()
            .map(|(actor, belief_ids)| {
                let surface = if actor == "DIALOGUE_USER" {
                    if language == LanguageCodeIR::Korean {
                        "너".into()
                    } else {
                        "you".into()
                    }
                } else {
                    actor.clone()
                };
                let mut refs = grounding_refs.iter().cloned().collect::<BTreeSet<_>>();
                refs.extend(
                    belief_ids
                        .into_iter()
                        .map(|id| format!("DIALOGUE_BELIEF_ID:{id}")),
                );
                refs.insert("QUERY_FOCUS:PROPOSITION_SOURCE_IDENTITY".into());
                if actor == "DIALOGUE_USER" {
                    refs.insert("DIALOGUE_DEIXIS:SOURCE_USER_AS_CURRENT_ADDRESSEE".into());
                }
                (actor, (surface, refs))
            })
            .collect();
        return generate_answer_value_set(
            settings,
            crate::proposition_content::ContentSlotIR::Source,
            values,
            None,
            &answer.korean_nominal_forms,
        );
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
    let property_owner = answer.missing_property_owner();
    let unknown_slot = answer.unknown_content_slot();
    let terminal_concept = match answer.disposition {
        DiscourseAnswerDispositionIR::AnsweredFromDialogueRecords
        | DiscourseAnswerDispositionIR::MultipleDialogueRecords
        | DiscourseAnswerDispositionIR::DialogueTruthNotEstablished => "C_DIALOGUE_ANSWER_NOT_FACT",
        DiscourseAnswerDispositionIR::ConflictingDialogueRecords => "C_DIALOGUE_ANSWER_CONFLICT",
        DiscourseAnswerDispositionIR::NoConflictRecorded => "C_DIALOGUE_ANSWER_NO_CONFLICT",
        DiscourseAnswerDispositionIR::PresuppositionUnverified => {
            "C_DIALOGUE_ANSWER_PRESUPPOSITION"
        }
        DiscourseAnswerDispositionIR::NoMatchingRecord
            if property_owner.is_some() || unknown_slot.is_some() =>
        {
            "C_DIALOGUE_ANSWER_UNKNOWN_PROPERTY"
        }
        DiscourseAnswerDispositionIR::NoMatchingRecord => "C_DIALOGUE_ANSWER_NO_MATCH",
        DiscourseAnswerDispositionIR::AmbiguousQuery
            if answer.response_constraint_conflict.is_some() =>
        {
            "C_DIALOGUE_ANSWER_REQUEST_CONFLICT"
        }
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
    if let Some(owner) = property_owner {
        for (id, concept, root, relation) in [
            (
                "R_GAP_OWNER",
                "C_RUNTIME_GAP_OWNER",
                owner,
                GenerationMeaningRelationIR::Theme,
            ),
            (
                "R_GAP_PROPERTY",
                "C_RUNTIME_GAP_PROPERTY",
                if language == LanguageCodeIR::Korean {
                    "상태"
                } else {
                    "state"
                },
                GenerationMeaningRelationIR::Property,
            ),
        ] {
            nodes.push(GenerationMeaningNodeIR {
                node_id: id.into(),
                concept_id: concept.into(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: base_refs.clone(),
            });
            edges.push(meaning_edge(
                &format!("GAP_{id}"),
                &terminal_id,
                id,
                relation,
            ));
            expressions.attach_alias(
                &format!("EXPR.{id}"),
                language,
                concept,
                root,
                ExpressionPartOfSpeechIR::Noun,
                "RUNTIME_REFERENT_SURFACE:UNDERSTOOD_QUERY",
            )?;
        }
    }
    if property_owner.is_none() {
        if let Some(slot) = unknown_slot {
            // An explicitly owned inner question/new nominal target cannot
            // disappear merely because lookup failed. Lookup hints are not
            // allowed to replace this source-bound argument.
            let explicit_argument = answer
                .question_request
                .as_ref()
                .map(|request| request.question_text.trim_end_matches('?'))
                .or_else(|| {
                    answer
                        .content_request
                        .as_ref()
                        .filter(|request| request.target_surface.is_some())
                        .map(|request| request.argument_surface.as_str())
                });
            let concept = format!("C_QUERY_SLOT_{slot:?}");
            nodes.push(GenerationMeaningNodeIR {
                node_id: "R_UNKNOWN_SLOT".into(),
                concept_id: concept.clone(),
                kind: GenerationMeaningNodeKindIR::Entity,
                grounding_refs: base_refs
                    .iter()
                    .cloned()
                    .chain([format!("REQUESTED_CONTENT_SLOT:{slot:?}")])
                    .collect(),
            });
            edges.push(meaning_edge(
                "UNKNOWN_SLOT",
                &terminal_id,
                "R_UNKNOWN_SLOT",
                GenerationMeaningRelationIR::Property,
            ));
            expressions.attach_alias(
                "EXPR.UNKNOWN_SLOT",
                language,
                &concept,
                if explicit_argument.is_some() {
                    if language == LanguageCodeIR::Korean {
                        "답"
                    } else {
                        "answer"
                    }
                } else {
                    content_slot_label(slot, language == LanguageCodeIR::Korean, false)
                },
                ExpressionPartOfSpeechIR::Noun,
                "SUPPLIED_CONTENT_SLOT_LEXICAL_KNOWLEDGE",
            )?;
            if let Some(argument) = explicit_argument {
                nodes.push(GenerationMeaningNodeIR {
                    node_id: "R_QUERY_ARGUMENT".into(),
                    concept_id: "C_QUERY_CONTENT_ARGUMENT".into(),
                    kind: GenerationMeaningNodeKindIR::Entity,
                    grounding_refs: base_refs.clone(),
                });
                edges.push(meaning_edge(
                    "QUERY_ARGUMENT",
                    &terminal_id,
                    "R_QUERY_ARGUMENT",
                    GenerationMeaningRelationIR::Theme,
                ));
                expressions.attach_alias(
                    "EXPR.QUERY_ARGUMENT",
                    language,
                    "C_QUERY_CONTENT_ARGUMENT",
                    argument,
                    ExpressionPartOfSpeechIR::Noun,
                    "RUNTIME_REFERENT_SURFACE:SOURCE_BOUND_QUERY_ARGUMENT",
                )?;
            }
        }
    }
    let gap_topic = if property_owner.is_some() || unknown_slot.is_some() {
        None
    } else if let Some(request) = &answer.question_request {
        // Source-bound query owns missing-information wording as well as
        // successful retrieval. Generic subject hints cannot replace it.
        Some(request.question_text.trim_end_matches('?').to_string())
    } else {
        answer
            .content_request
            .as_ref()
            .map(|request| {
                request
                    .target_surface
                    .as_ref()
                    .map(|_| request.argument_surface.clone())
            })
            .unwrap_or_else(|| {
                // Topic terms are normalized retrieval keys, not necessarily
                // a realizable noun phrase. Only a single source-owned term is
                // safe to surface; multiple stems retain the generic question
                // reference instead of exposing an artificial concatenation.
                match answer.query.topic_terms.as_slice() {
                    [term] => Some(term.clone()),
                    _ => None,
                }
            })
    };
    if answer.disposition == DiscourseAnswerDispositionIR::NoMatchingRecord && gap_topic.is_some() {
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
            gap_topic.as_deref().unwrap_or_default(),
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

    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    answer: &DialogueRelationAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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

    settings.generate(GenerativeLanguageRequestIR {
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
    settings: impl Into<GenerationSettings>,
    answer: &TemporalAnswerIR,
    grounding_refs: &[String],
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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

    settings.generate(GenerativeLanguageRequestIR {
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

pub(crate) fn dialogue_attribution_surface(
    language: LanguageCodeIR,
    query_kind: DiscourseQueryKindIR,
    evidence: &DiscourseAnswerEvidenceIR,
) -> String {
    // Dialogue participant identity is not a display name. Resolve its
    // possessive expression before attaching the attribution constituent.
    let possessive = if evidence.source_actor == "DIALOGUE_USER" {
        if language == LanguageCodeIR::Korean {
            "네".into()
        } else {
            "your".into()
        }
    } else if language == LanguageCodeIR::Korean {
        format!("{}의", evidence.source_actor)
    } else {
        format!("{}'s", evidence.source_actor)
    };
    if query_kind == DiscourseQueryKindIR::ModalStatus {
        return match language {
            LanguageCodeIR::Korean => format!(
                "{} {}",
                possessive,
                korean_modal_classification(evidence.modal_world)
            ),
            _ => format!(
                "{} {}",
                possessive,
                english_modal_classification(evidence.modal_world)
            ),
        };
    }
    match language {
        LanguageCodeIR::Korean => format!(
            "{} {}",
            possessive,
            korean_epistemic_record(evidence.epistemic_status)
        ),
        _ => format!(
            "{} {}",
            possessive,
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
    settings: impl Into<GenerationSettings>,
    transition: &TopicTransitionIR,
) -> Result<GenerativeLanguageIR, String> {
    let settings = settings.into();
    let language = settings.language;
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
    settings.generate(GenerativeLanguageRequestIR {
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

    #[test]
    fn parallel_content_scope_requires_same_belief_event_speaker_and_sequence() {
        use crate::proposition_content::{
            ContentProjectionIR, ContentSlotIR, PropositionContentIR,
        };
        for (source, language, marker) in [
            (
                "Mira read a letter at the station.",
                LanguageCodeIR::English,
                "According to",
            ),
            (
                "Mira did not read a letter at the station.",
                LanguageCodeIR::English,
                "According to",
            ),
            (
                "예린이 공원에서 잡지를 읽었어.",
                LanguageCodeIR::Korean,
                "따르면",
            ),
        ] {
            let event = PropositionContentIR::compile(source).events[0].clone();
            let bindings = event.bindings();
            let projection = ContentProjectionIR {
                elaboration_omitted_roles: vec![],
                elaboration_event: None,
                event_perspective: None,
                co_answers: vec![],
                belief_id: "SCOPE-BELIEF".into(),
                source_actor: "DIALOGUE_USER".into(),
                source_proposition: source.into(),
                binding: bindings
                    .iter()
                    .find(|b| b.slot == ContentSlotIR::Agent)
                    .unwrap()
                    .clone(),
                additional_bindings: vec![bindings
                    .iter()
                    .find(|b| b.slot == ContentSlotIR::Location)
                    .unwrap()
                    .clone()],
                context_sources: vec![],
                reference_context: None,
                reference_bindings: vec![],
            };
            let g = generate_content_projection(
                language,
                &projection,
                false,
                crate::discourse_qa::AnswerSourceFramingIR::Explicit,
            )
            .unwrap();
            assert!(g.validate());
            assert_eq!(g.morphology.realized_text.matches(marker).count(), 1);
            for boundary in ["belief", "event", "speaker", "sequence", "speech_act"] {
                let mut meaning = g.meaning.clone();
                let mut syntax = g.syntax_plan.clone();
                let mut expressions = g.expression_selection.clone();
                let second = &mut syntax.clauses[1];
                match boundary {
                    "belief" | "event" => {
                        let prefix = if boundary == "belief" {
                            "DIALOGUE_BELIEF_ID:"
                        } else {
                            "SOURCE_EVENT:"
                        };
                        let node = meaning
                            .nodes
                            .iter_mut()
                            .find(|n| n.node_id == second.event_node_id)
                            .unwrap();
                        for anchor in &mut node.grounding_refs {
                            if anchor.starts_with(prefix) {
                                *anchor = format!("{prefix}DIFFERENT");
                            }
                        }
                    }
                    "speaker" => {
                        let node = second
                            .constituents
                            .iter()
                            .find(|c| c.role == SyntaxConstituentRoleIR::Goal)
                            .unwrap();
                        expressions
                            .selections
                            .iter_mut()
                            .find(|s| s.meaning_node_id == node.meaning_node_id)
                            .unwrap()
                            .expression
                            .lexical_root = "someone else".into();
                    }
                    "sequence" => meaning
                        .edges
                        .retain(|e| e.relation != GenerationMeaningRelationIR::Sequence),
                    _ => second.speech_intent = GenerationSpeechIntentIR::DescribePlan,
                }
                let selected = expressions
                    .selections
                    .iter()
                    .map(|s| {
                        (
                            (
                                s.expression.expression_id.as_str(),
                                s.meaning_node_id.as_str(),
                            ),
                            s,
                        )
                    })
                    .collect();
                assert!(
                    !content_projection_joined(
                        &syntax.clauses[0],
                        &syntax.clauses[1],
                        &meaning,
                        &selected
                    ),
                    "{boundary}"
                );
            }
            let mut forged = g.clone();
            forged.morphology.realized_text.push_str(" invented");
            assert!(!forged.validate());
        }
    }

    #[test]
    fn nominal_sound_evidence_changes_expression_not_meaning() {
        let values = BTreeMap::from([(
            "Qx".into(),
            ("Qx".into(), BTreeSet::from(["TEST_OBSERVATION:1".into()])),
        )]);
        let witness = crate::korean_nominal::KoreanNominalFormIR::observe("Qx", "Qx이").unwrap();
        let generate = |forms: &[crate::korean_nominal::KoreanNominalFormIR]| {
            generate_answer_value_set(
                LanguageCodeIR::Korean.into(),
                crate::proposition_content::ContentSlotIR::Source,
                values.clone(),
                None,
                forms,
            )
            .unwrap()
        };
        let known = generate(&[witness]);
        let unknown = generate(&[]);
        assert_eq!(known.morphology.realized_text, "Qx이야.");
        assert_eq!(unknown.morphology.realized_text, "Qx.");
        assert_eq!(
            known.meaning.semantic_sha256,
            unknown.meaning.semantic_sha256
        );
        assert_eq!(known.verification.unsupported_claims, 0);
        assert_eq!(unknown.verification.unsupported_claims, 0);
    }

    #[test]
    fn nominal_position_changes_determiners_not_observed_name_spelling() {
        for (root, medial, initial) in [
            ("The gardener", "the gardener", "The gardener"),
            ("an engineer", "an engineer", "An engineer"),
            ("McKay", "McKay", "McKay"),
            ("eBay", "eBay", "eBay"),
            ("UNESCO", "UNESCO", "UNESCO"),
            ("to Lena", "to Lena", "To Lena"),
            ("from McKay", "from McKay", "From McKay"),
        ] {
            assert_eq!(english_positioned_nominal(root, false), medial);
            assert_eq!(english_positioned_nominal(root, true), initial);
        }
    }

    #[test]
    fn lifecycle_action_reference_preserves_predicate_target_and_nonassertion() {
        for (predicate, ko, en) in [
            ("READ", "읽기", "read"),
            ("OPEN", "열기", "open"),
            ("SAVE", "저장", "save"),
            ("REPAIR", "수리", "repair"),
        ] {
            let mut meanings = Vec::new();
            for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
                MORPHOLOGY_PASSES.with(|passes| passes.set(0));
                let generated = generate_lifecycle_status_with_action(
                    language,
                    "Aster",
                    Some(predicate),
                    &[
                        GenerationLifecycleClaimIR::ActivePlan,
                        GenerationLifecycleClaimIR::NoVerifiedExecutionOrResult,
                    ],
                    "TEST:ACTION:1",
                )
                .unwrap();
                assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
                assert_eq!(
                    generated
                        .meaning
                        .nodes
                        .iter()
                        .find(|n| n.node_id == "R_ACTION")
                        .unwrap()
                        .kind,
                    GenerationMeaningNodeKindIR::EventReference
                );
                assert!(generated
                    .speech_intent
                    .intents
                    .iter()
                    .all(|i| i.event_node_id != "R_ACTION"));
                assert!(
                    generated.morphology.realized_text.contains(
                        if language == LanguageCodeIR::Korean {
                            ko
                        } else {
                            en
                        }
                    ),
                    "{}",
                    generated.morphology.realized_text
                );
                assert!(generated.morphology.realized_text.contains("Aster"));
                assert!(generated.validate());
                assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 2);
                let mut forged = generated.clone();
                forged
                    .meaning
                    .edges
                    .retain(|edge| edge.edge_id != "ACTION_REFERENCE.THEME");
                forged.meaning.semantic_sha256 = generation_meaning_sha256(&forged.meaning);
                forged.generation_sha256 = generative_language_sha256(&forged);
                assert!(!forged.validate());
                meanings.push(generated.meaning);
            }
            assert_eq!(meanings[0], meanings[1]);
        }
    }

    #[test]
    fn opaque_action_reference_does_not_guess_a_known_operation() {
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let generated = generate_lifecycle_status_with_action(
                language,
                "Aster",
                Some("C_UNLEXICALIZED_TEST"),
                &[GenerationLifecycleClaimIR::ActivePlan],
                "TEST:OPAQUE:1",
            )
            .unwrap();
            assert!(generated
                .meaning
                .nodes
                .iter()
                .any(|n| n.concept_id == "C_UNLEXICALIZED_TEST"
                    && n.kind == GenerationMeaningNodeKindIR::EventReference));
            assert!(generated
                .expression_selection
                .selections
                .iter()
                .all(|s| s.expression.concept_id != "C_PERFORM"));
            assert!(generated.validate());
        }
    }

    #[test]
    fn action_reference_uses_supplied_lexical_stems_without_changing_its_meaning() {
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let baseline = generate_lifecycle_status_with_action(
                language,
                "Aster",
                Some("C_TEST_REFERENCE"),
                &[GenerationLifecycleClaimIR::ActivePlan],
                "TEST:REFERENCE:1",
            )
            .unwrap();
            let roots = if language == LanguageCodeIR::Korean {
                ["도루하", "누마하"]
            } else {
                ["florp", "zindle"]
            };
            for root in roots {
                let mut expressions = ExpressionNodeStore::bilingual_builtin();
                expressions
                    .inject(expression(
                        "TEST.REFERENCE.VERB",
                        language,
                        "C_TEST_REFERENCE",
                        root,
                        ExpressionPartOfSpeechIR::Verb,
                        if language == LanguageCodeIR::Korean {
                            ExpressionMorphologyClassIR::KoreanHada
                        } else {
                            ExpressionMorphologyClassIR::EnglishRegular
                        },
                        LanguageRegisterIR::Neutral,
                    ))
                    .unwrap();
                expressions
                    .attach_alias(
                        "TEST.REFERENCE.TARGET",
                        language,
                        "C_RUNTIME_LIFECYCLE_SUBJECT",
                        "Aster",
                        ExpressionPartOfSpeechIR::Noun,
                        "TEST:REFERENT",
                    )
                    .unwrap();
                MORPHOLOGY_PASSES.with(|passes| passes.set(0));
                let generated = GenerativeLanguageCortex
                    .generate(GenerativeLanguageRequestIR {
                        meaning: baseline.meaning.clone(),
                        context: baseline.context.clone(),
                        expressions: &expressions,
                    })
                    .unwrap();
                assert_eq!(generated.meaning, baseline.meaning);
                assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
                assert!(generated.morphology.realized_text.contains(
                    if language == LanguageCodeIR::Korean {
                        root.trim_end_matches('하')
                    } else {
                        root
                    }
                ));
                assert!(generated.morphology.realized_text.contains("Aster"));
                assert!(generated.validate());
            }
        }
    }

    #[test]
    fn korean_grammatical_act_owns_register_for_supplied_roots() {
        use GenerationSpeechIntentIR::*;
        for root in ["점검하", "도루하", "느마하"] {
            let mut store = ExpressionNodeStore::bilingual_builtin();
            store
                .inject(expression(
                    "TEST.KO.PREDICATE",
                    LanguageCodeIR::Korean,
                    "C_TEST_PREDICATE",
                    root,
                    ExpressionPartOfSpeechIR::Verb,
                    ExpressionMorphologyClassIR::KoreanHada,
                    LanguageRegisterIR::Neutral,
                ))
                .unwrap();
            let meaning = GenerationMeaningGraphIR::new(
                vec![GenerationMeaningNodeIR {
                    node_id: "E".into(),
                    concept_id: "C_TEST_PREDICATE".into(),
                    kind: GenerationMeaningNodeKindIR::Event,
                    grounding_refs: vec!["TEST:GRAMMAR".into()],
                }],
                vec![],
            );
            for (intent, informal, neutral, formal) in [
                (CommitFutureAction, "할게.", "할게요.", "하겠습니다."),
                (DescribePlan, "하는 거야.", "하는 거예요.", "하는 것입니다."),
                (Invite, "해 보자.", "해 봐요.", "해 봅시다."),
                (Ask, "하나요?", "하나요?", "합니까?"),
                (Inform, "해.", "해요.", "합니다."),
                (Acknowledge, "해.", "해요.", "합니다."),
                (Advise, "해야 해요.", "해야 해요.", "해야 해요."),
            ] {
                for (register, suffix) in [
                    (LanguageRegisterIR::Informal, informal),
                    (LanguageRegisterIR::Neutral, neutral),
                    (LanguageRegisterIR::Formal, formal),
                ] {
                    MORPHOLOGY_PASSES.with(|passes| passes.set(0));
                    let generated = GenerativeLanguageCortex
                        .generate(GenerativeLanguageRequestIR {
                            meaning: meaning.clone(),
                            expressions: &store,
                            context: GenerationContextIR {
                                language: LanguageCodeIR::Korean,
                                register,
                                tense: GenerationTenseIR::Present,
                                emotion: GenerationEmotionIR::Neutral,
                                urgency_millis: 0,
                                default_speech_intent: intent,
                            },
                        })
                        .unwrap();
                    assert_eq!(
                        generated.morphology.realized_text,
                        format!(
                            "{}{}{suffix}",
                            if intent == DescribePlan {
                                "계획은 "
                            } else {
                                ""
                            },
                            root.trim_end_matches('하')
                        )
                    );
                    assert_eq!(generated.meaning, meaning);
                    assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
                    assert!(generated.validate());
                    assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 2);
                }
            }
        }
    }

    #[test]
    fn korean_property_polarity_and_register_compose_without_changing_meaning() {
        for root in ["답답하", "도루하"] {
            for negated in [false, true] {
                let node = |id: &str, concept: &str, kind| GenerationMeaningNodeIR {
                    node_id: id.into(),
                    concept_id: concept.into(),
                    kind,
                    grounding_refs: vec!["TEST:PROPERTY".into()],
                };
                let mut nodes = vec![
                    node("E", "C_COPULA", GenerationMeaningNodeKindIR::Event),
                    node("P", "C_TEST_PROPERTY", GenerationMeaningNodeKindIR::State),
                ];
                let mut edges = vec![meaning_edge(
                    "PROPERTY",
                    "E",
                    "P",
                    GenerationMeaningRelationIR::Property,
                )];
                if negated {
                    nodes.push(node("N", "C_NEGATION", GenerationMeaningNodeKindIR::State));
                    edges.push(meaning_edge(
                        "NEGATION",
                        "E",
                        "N",
                        GenerationMeaningRelationIR::Negates,
                    ));
                }
                let meaning = GenerationMeaningGraphIR::new(nodes, edges);
                let mut store = ExpressionNodeStore::bilingual_builtin();
                store
                    .inject(expression(
                        "TEST.KO.PROPERTY",
                        LanguageCodeIR::Korean,
                        "C_TEST_PROPERTY",
                        root,
                        ExpressionPartOfSpeechIR::Adjective,
                        ExpressionMorphologyClassIR::KoreanHada,
                        LanguageRegisterIR::Neutral,
                    ))
                    .unwrap();
                for register in [
                    LanguageRegisterIR::Informal,
                    LanguageRegisterIR::Neutral,
                    LanguageRegisterIR::Formal,
                ] {
                    let generated = GenerativeLanguageCortex
                        .generate(GenerativeLanguageRequestIR {
                            meaning: meaning.clone(),
                            expressions: &store,
                            context: GenerationContextIR {
                                language: LanguageCodeIR::Korean,
                                register,
                                tense: GenerationTenseIR::Present,
                                emotion: GenerationEmotionIR::Neutral,
                                urgency_millis: 0,
                                default_speech_intent: GenerationSpeechIntentIR::Inform,
                            },
                        })
                        .unwrap();
                    let suffix = match (negated, register) {
                        (true, LanguageRegisterIR::Formal) => "하지 않습니다.",
                        (true, LanguageRegisterIR::Neutral) => "하지 않아요.",
                        (true, _) => "하지 않아.",
                        (false, LanguageRegisterIR::Formal) => "합니다.",
                        (false, LanguageRegisterIR::Neutral) => "해요.",
                        (false, _) => "해.",
                    };
                    assert_eq!(
                        generated.morphology.realized_text,
                        format!("{}{suffix}", root.trim_end_matches('하'))
                    );
                    assert_eq!(generated.meaning, meaning);
                    assert!(generated.validate());
                }
            }
        }
    }

    #[test]
    fn korean_neutral_copula_respects_the_property_final_sound() {
        for (property, expected) in [("사람", "사람이에요."), ("나무", "나무예요.")] {
            let meaning = GenerationMeaningGraphIR::new(
                vec![
                    GenerationMeaningNodeIR {
                        node_id: "E".into(),
                        concept_id: "C_COPULA".into(),
                        kind: GenerationMeaningNodeKindIR::Event,
                        grounding_refs: vec!["TEST:COPULA".into()],
                    },
                    GenerationMeaningNodeIR {
                        node_id: "P".into(),
                        concept_id: "C_TEST_NOMINAL_PROPERTY".into(),
                        kind: GenerationMeaningNodeKindIR::State,
                        grounding_refs: vec!["TEST:COPULA".into()],
                    },
                ],
                vec![meaning_edge(
                    "PROPERTY",
                    "E",
                    "P",
                    GenerationMeaningRelationIR::Property,
                )],
            );
            let mut store = ExpressionNodeStore::bilingual_builtin();
            store
                .inject(expression(
                    "TEST.KO.NOMINAL_PROPERTY",
                    LanguageCodeIR::Korean,
                    "C_TEST_NOMINAL_PROPERTY",
                    property,
                    ExpressionPartOfSpeechIR::Noun,
                    ExpressionMorphologyClassIR::KoreanInvariable,
                    LanguageRegisterIR::Neutral,
                ))
                .unwrap();
            let generated = GenerativeLanguageCortex
                .generate(GenerativeLanguageRequestIR {
                    meaning: meaning.clone(),
                    expressions: &store,
                    context: GenerationContextIR {
                        language: LanguageCodeIR::Korean,
                        register: LanguageRegisterIR::Neutral,
                        tense: GenerationTenseIR::Present,
                        emotion: GenerationEmotionIR::Neutral,
                        urgency_millis: 0,
                        default_speech_intent: GenerationSpeechIntentIR::Inform,
                    },
                })
                .unwrap();
            assert_eq!(generated.morphology.realized_text, expected);
            assert_eq!(generated.meaning, meaning);
            assert!(generated.validate());
        }
    }

    #[test]
    fn formal_acknowledgement_and_detailed_plan_share_construction_register() {
        let policy = crate::affective_field::AffectiveRealizationPolicyIR {
            formal: true,
            ..Default::default()
        };
        let settings = GenerationSettings::with_policy(LanguageCodeIR::Korean, &policy);
        let ack =
            generate_acknowledgement_from_knowledge(settings, vec!["TEST:FORMAL".into()]).unwrap();
        assert_eq!(ack.morphology.realized_text, "알겠습니다.");
        for (predicate, expected) in [
            ("READ", "읽는 것입니다"),
            // A regular predicate can share its final formal ending with the
            // following plan clause; the connective itself has no register.
            ("SAVE", "자료를 저장하고,"),
            ("OPEN", "여는 것입니다"),
        ] {
            MORPHOLOGY_PASSES.with(|passes| passes.set(0));
            let generated = generate_plan_preview_with_predicate(
                settings,
                "자료",
                PlanIntentIR::Execute,
                "TEST:FORMAL",
                None,
                PlanPreviewContentIR::Detailed,
                Some(predicate),
            )
            .unwrap();
            assert!(
                generated.morphology.realized_text.contains(expected),
                "{}",
                generated.morphology.realized_text
            );
            assert!(!generated.morphology.realized_text.contains("할게"));
            assert!(generated
                .morphology
                .realized_text
                .contains("검증하는 것입니다"));
            assert!(!generated.morphology.realized_text.contains("거야"));
            assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
            assert!(generated.validate());
        }
    }

    #[test]
    fn policy_matrix_preserves_meaning_and_constructs_each_surface_once() {
        use crate::affective_field::AffectiveRealizationPolicyIR as Policy;
        let policies = [
            Policy::default(),
            Policy {
                formal: true,
                ..Default::default()
            },
            Policy {
                warmth_millis: 800,
                ..Default::default()
            },
            Policy {
                playfulness_millis: 800,
                ..Default::default()
            },
            Policy {
                urgency_millis: 800,
                playfulness_millis: 800,
                ..Default::default()
            },
            Policy {
                formal: true,
                warmth_millis: 800,
                brevity_millis: 800,
                urgency_millis: 800,
                playfulness_millis: 800,
                korean_dialect: crate::affective_field::KoreanDialectIR::Standard,
                ..Default::default()
            },
        ];
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            for category in 0..6 {
                let build = |settings: GenerationSettings| match category {
                    0 => generate_acknowledgement_from_knowledge(
                        settings,
                        vec!["TEST:POLICY".into()],
                    ),
                    1 => generate_dialogue_response_from_knowledge(
                        settings,
                        GenerationDialogueResponseKindIR::Greeting,
                    ),
                    2 => generate_affect_support_from_knowledge(
                        settings,
                        GenerationAffectKindIR::Worried,
                    ),
                    3 => generate_plan_preview_with_predicate(
                        settings,
                        "Aster",
                        PlanIntentIR::Execute,
                        "TEST:POLICY",
                        None,
                        PlanPreviewContentIR::Compact,
                        Some("READ"),
                    ),
                    4 => generate_lifecycle_status_from_knowledge(
                        settings,
                        "Aster",
                        &[GenerationLifecycleClaimIR::NoVerifiedExecutionOrResult],
                        "TEST:POLICY",
                    ),
                    _ => generate_clarification_from_knowledge(
                        settings,
                        GenerationClarificationKindIR::MissingDetails,
                        None,
                        &["TEST:POLICY".into()],
                    ),
                };
                let baseline = build(language.into()).unwrap();
                for policy in &policies {
                    MORPHOLOGY_PASSES.with(|passes| passes.set(0));
                    let generated =
                        build(GenerationSettings::with_policy(language, policy)).unwrap();
                    assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
                    assert_eq!(generated.meaning, baseline.meaning);
                    assert_eq!(generated.speech_intent, baseline.speech_intent);
                    assert_eq!(generated.context.urgency_millis, policy.urgency_millis);
                    if policy.formal {
                        assert_eq!(generated.context.register, LanguageRegisterIR::Formal);
                    }
                    assert!(generated.validate());
                    let mut tampered = generated.clone();
                    tampered.morphology.realized_text.push_str(" unsupported");
                    tampered.generation_sha256 = generative_language_sha256(&tampered);
                    assert!(!tampered.validate());
                }
            }
        }
    }

    #[test]
    fn surface_policy_precedes_generation_and_formal_negation_composes() {
        MORPHOLOGY_PASSES.with(|passes| passes.set(0));
        let ack = generate_acknowledgement_from_knowledge(
            LanguageCodeIR::Korean,
            vec!["TEST:DIRECTIVE".into()],
        )
        .unwrap();
        assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
        assert!(ack.validate());
        for (stem, expected) in [("아니", "아닙니다"), ("읽", "읽습니다"), ("열", "엽니다")]
        {
            assert_eq!(korean_formal_statement(stem), expected);
        }
        for subject in ["문서", "캐시"] {
            MORPHOLOGY_PASSES.with(|passes| passes.set(0));
            let generated = generate_plan_preview_with_predicate(
                GenerationSettings::with_policy(
                    LanguageCodeIR::Korean,
                    &crate::affective_field::AffectiveRealizationPolicyIR {
                        formal: true,
                        ..Default::default()
                    },
                ),
                subject,
                PlanIntentIR::Execute,
                "TEST:PLAN",
                None,
                PlanPreviewContentIR::Compact,
                Some("SAVE"),
            )
            .unwrap();
            assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
            assert!(generated.morphology.realized_text.contains("아닙니다"));
            assert!(!generated.morphology.realized_text.contains("아니입니다"));
            assert!(generated.validate());
        }
    }

    #[test]
    fn generation_constructs_surface_once_and_boundary_rejects_edited_output() {
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            MORPHOLOGY_PASSES.with(|passes| passes.set(0));
            let generated = generate_safety(language);
            assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
            assert!(generated.validate());
            let mut forged = generated.clone();
            forged.morphology.realized_text.push_str(" invented");
            forged.generation_sha256 = generative_language_sha256(&forged);
            assert!(!forged.validate());
        }
    }

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
    fn expression_affinity_composes_unseen_aliases_without_answer_dispatch() {
        for language in [LanguageCodeIR::Korean, LanguageCodeIR::English] {
            let baseline = generate_dialogue_response_from_knowledge(
                language,
                GenerationDialogueResponseKindIR::Greeting,
            )
            .unwrap();
            // An opaque concept and new lexical roots use the same selector;
            // neither the concept ID nor these forms has a runtime branch.
            let meaning = GenerationMeaningGraphIR::new(
                vec![GenerationMeaningNodeIR {
                    node_id: "E_OPAQUE".into(),
                    concept_id: "OPAQUE-STYLE-TEST".into(),
                    kind: GenerationMeaningNodeKindIR::Event,
                    grounding_refs: vec!["SUPPLIED_LEXICAL_EQUIVALENCE".into()],
                }],
                vec![],
            );
            let mut store = ExpressionNodeStore::default();
            for (id, root, emotion) in [
                ("N", "Navo", None),
                ("W", "Muri", Some(GenerationEmotionIR::Warm)),
                ("P", "Selo", Some(GenerationEmotionIR::Playful)),
            ] {
                let mut alias = expression(
                    id,
                    language,
                    "OPAQUE-STYLE-TEST",
                    root,
                    ExpressionPartOfSpeechIR::Interjection,
                    if language == LanguageCodeIR::Korean {
                        ExpressionMorphologyClassIR::KoreanInvariable
                    } else {
                        ExpressionMorphologyClassIR::EnglishInvariable
                    },
                    LanguageRegisterIR::Neutral,
                );
                alias.preferred_emotion = emotion;
                store.inject(alias).unwrap();
            }
            let mut surfaces = BTreeSet::new();
            for (emotion, selected) in [
                (GenerationEmotionIR::Neutral, "N"),
                (GenerationEmotionIR::Warm, "W"),
                (GenerationEmotionIR::Playful, "P"),
            ] {
                MORPHOLOGY_PASSES.with(|c| c.set(0));
                let generated = GenerativeLanguageCortex
                    .generate(GenerativeLanguageRequestIR {
                        meaning: meaning.clone(),
                        context: GenerationContextIR {
                            emotion,
                            ..baseline.context.clone()
                        },
                        expressions: &store,
                    })
                    .unwrap();
                assert_eq!(MORPHOLOGY_PASSES.with(|c| c.get()), 1);
                assert_eq!(generated.meaning, meaning);
                assert_eq!(
                    generated.expression_selection.selections[0]
                        .expression
                        .expression_id,
                    selected
                );
                assert!(generated.validate());
                surfaces.insert(generated.morphology.realized_text);
            }
            assert_eq!(surfaces.len(), 3);
            // Lexical selection cannot supply a concept after semantic removal.
            assert!(GenerativeLanguageCortex
                .generate(GenerativeLanguageRequestIR {
                    meaning: GenerationMeaningGraphIR::new(vec![], vec![]),
                    context: baseline.context,
                    expressions: &store,
                })
                .is_err());
        }
    }

    #[test]
    fn regional_style_changes_lexicon_and_morphology_without_changing_meaning() {
        use crate::affective_field::{
            AffectiveRealizationPolicyIR as Policy, KoreanDialectIR as D,
        };
        let baseline = generate_dialogue_response_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        for (dialect, expected) in [
            (D::Gyeongsang, "반갑데이! 무엇을 도와줄까예?"),
            (D::Chungcheong, "반가워유! 무엇을 도와줄까유?"),
        ] {
            MORPHOLOGY_PASSES.with(|passes| passes.set(0));
            let generated = generate_dialogue_response_from_knowledge(
                GenerationSettings::with_policy(
                    LanguageCodeIR::Korean,
                    &Policy {
                        korean_dialect: dialect,
                        ..Default::default()
                    },
                ),
                GenerationDialogueResponseKindIR::Greeting,
            )
            .unwrap();
            assert_eq!(MORPHOLOGY_PASSES.with(|passes| passes.get()), 1);
            assert_eq!(generated.korean_dialect, dialect);
            assert_eq!(generated.meaning, baseline.meaning);
            assert_eq!(generated.speech_intent, baseline.speech_intent);
            assert_eq!(generated.morphology.realized_text, expected);
            assert_eq!(
                generated.expression_selection.selections[0]
                    .expression
                    .preferred_korean_dialect,
                Some(dialect)
            );
            assert!(generated.validate());
        }

        // Warmth and dialect are independent expression dimensions. A warm
        // policy must not pull a regional response back to the standard-only
        // warm lexeme, while the approved meaning remains unchanged.
        let warm_gyeongsang = generate_dialogue_response_from_knowledge(
            GenerationSettings::with_policy(
                LanguageCodeIR::Korean,
                &Policy {
                    warmth_millis: 800,
                    korean_dialect: D::Gyeongsang,
                    ..Default::default()
                },
            ),
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        assert_eq!(
            warm_gyeongsang.morphology.realized_text,
            "반갑데이! 무엇을 도와줄까예?"
        );
        assert_eq!(warm_gyeongsang.meaning, baseline.meaning);
        assert_eq!(warm_gyeongsang.speech_intent, baseline.speech_intent);
        assert_eq!(
            warm_gyeongsang.expression_selection.selections[0]
                .expression
                .preferred_korean_dialect,
            Some(D::Gyeongsang)
        );
        assert!(warm_gyeongsang.validate());

        for (dialect, expected) in [
            (D::Gyeongsang, "안녕하이소! 무엇을 도와드릴까예?"),
            (D::Chungcheong, "안녕하세유! 무엇을 도와드릴까유?"),
        ] {
            let formal_regional = generate_dialogue_response_from_knowledge(
                GenerationSettings::with_policy(
                    LanguageCodeIR::Korean,
                    &Policy {
                        formal: true,
                        korean_dialect: dialect,
                        ..Default::default()
                    },
                ),
                GenerationDialogueResponseKindIR::Greeting,
            )
            .unwrap();
            assert_eq!(formal_regional.morphology.realized_text, expected);
            assert_eq!(formal_regional.context.register, LanguageRegisterIR::Formal);
            assert_eq!(formal_regional.meaning, baseline.meaning);
            assert_eq!(formal_regional.speech_intent, baseline.speech_intent);
            assert!(formal_regional.validate());
        }

        for (dialect, standard, regional) in [
            (D::Gyeongsang, "점검합니다.", "점검합니더."),
            (D::Gyeongsang, "점검합니까?", "점검합니꺼?"),
            (D::Gyeongsang, "사람이 아닙니다.", "사람이 아입니더."),
            (D::Chungcheong, "점검해요.", "점검해유."),
            (D::Chungcheong, "사람이에요.", "사람이에유."),
            (D::Chungcheong, "아직 모르겠어.", "아직 모르겠어유."),
        ] {
            assert_eq!(korean_dialect_surface(standard, dialect), regional);
        }
    }

    #[test]
    fn roleplay_relationship_and_voice_change_social_realization_not_meaning() {
        use crate::affective_field::{
            AffectiveRealizationPolicyIR as Policy, RoleplayAgeBandIR as Age,
            RoleplayBackgroundIR as Background, RoleplayRelationshipIR as Relationship,
            RoleplayVoiceIR as Voice,
        };
        let baseline = generate_dialogue_response_from_knowledge(
            LanguageCodeIR::Korean,
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        let professional = generate_dialogue_response_from_knowledge(
            GenerationSettings::with_policy(
                LanguageCodeIR::Korean,
                &Policy {
                    relationship: Relationship::Professional,
                    ..Default::default()
                },
            ),
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        let gentle_close = generate_dialogue_response_from_knowledge(
            GenerationSettings::with_policy(
                LanguageCodeIR::Korean,
                &Policy {
                    relationship: Relationship::Close,
                    voice: Voice::Gentle,
                    ..Default::default()
                },
            ),
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        let lively_peer = generate_dialogue_response_from_knowledge(
            GenerationSettings::with_policy(
                LanguageCodeIR::Korean,
                &Policy {
                    relationship: Relationship::Peer,
                    voice: Voice::Lively,
                    ..Default::default()
                },
            ),
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();
        let demographic_only = generate_dialogue_response_from_knowledge(
            GenerationSettings::with_policy(
                LanguageCodeIR::Korean,
                &Policy {
                    age_band: Age::Elder,
                    background: Background::Academic,
                    ..Default::default()
                },
            ),
            GenerationDialogueResponseKindIR::Greeting,
        )
        .unwrap();

        for generated in [
            &professional,
            &gentle_close,
            &lively_peer,
            &demographic_only,
        ] {
            assert_eq!(generated.meaning, baseline.meaning);
            assert_eq!(generated.speech_intent, baseline.speech_intent);
            assert_eq!(
                generated
                    .syntax_plan
                    .clauses
                    .iter()
                    .map(|clause| (&clause.event_node_id, &clause.source_edge_ids))
                    .collect::<Vec<_>>(),
                baseline
                    .syntax_plan
                    .clauses
                    .iter()
                    .map(|clause| (&clause.event_node_id, &clause.source_edge_ids))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                generated.verification.semantic_roundtrip_sha256,
                baseline.verification.semantic_roundtrip_sha256
            );
            assert!(generated.validate());
        }
        assert_eq!(professional.context.register, LanguageRegisterIR::Formal);
        assert!(professional
            .morphology
            .realized_text
            .starts_with("안녕하세요!"));
        assert_eq!(gentle_close.context.register, LanguageRegisterIR::Informal);
        assert_eq!(gentle_close.context.emotion, GenerationEmotionIR::Warm);
        assert!(gentle_close.morphology.realized_text.starts_with("반가워!"));
        assert_eq!(
            gentle_close.expression_selection.selections[0]
                .expression
                .preferred_roleplay_relationship,
            Some(Relationship::Close)
        );
        assert_eq!(
            gentle_close.expression_selection.selections[0]
                .expression
                .preferred_roleplay_voice,
            Some(Voice::Gentle)
        );
        assert_eq!(lively_peer.context.emotion, GenerationEmotionIR::Playful);
        assert!(lively_peer.morphology.realized_text.starts_with("ㅎㅎ "));
        assert_eq!(
            lively_peer.expression_selection.selections[0]
                .expression
                .preferred_roleplay_voice,
            Some(Voice::Lively)
        );
        // Character metadata is preserved in policy, but no age or background
        // automatically selects a manner of speech or a regional dialect.
        assert_eq!(demographic_only.context, baseline.context);
        assert_eq!(demographic_only.morphology, baseline.morphology);
    }

    #[test]
    fn exact_register_outweighs_affect_without_changing_concept_selection() {
        let mut neutral = expression(
            "OPAQUE.N",
            LanguageCodeIR::Korean,
            "OPAQUE.C",
            "Neri",
            ExpressionPartOfSpeechIR::Noun,
            ExpressionMorphologyClassIR::KoreanInvariable,
            LanguageRegisterIR::Neutral,
        );
        let mut warm = neutral.clone();
        warm.expression_id = "OPAQUE.W".into();
        warm.register = LanguageRegisterIR::Informal;
        warm.preferred_emotion = Some(GenerationEmotionIR::Warm);
        let mut formal = neutral.clone();
        formal.expression_id = "OPAQUE.F".into();
        formal.register = LanguageRegisterIR::Formal;
        let mut context = GenerationContextIR {
            language: LanguageCodeIR::Korean,
            register: LanguageRegisterIR::Formal,
            tense: GenerationTenseIR::Present,
            emotion: GenerationEmotionIR::Warm,
            urgency_millis: 0,
            default_speech_intent: GenerationSpeechIntentIR::Inform,
        };
        let standard = crate::affective_field::KoreanDialectIR::Standard;
        let persona = crate::affective_field::RoleplayPersonaIR::default();
        assert!(
            expression_score(&formal, &context, standard, persona)
                > expression_score(&neutral, &context, standard, persona)
        );
        assert!(
            expression_score(&neutral, &context, standard, persona)
                > expression_score(&warm, &context, standard, persona)
        );
        context.register = LanguageRegisterIR::Informal;
        assert!(
            expression_score(&warm, &context, standard, persona)
                > expression_score(&neutral, &context, standard, persona)
        );
        context.register = LanguageRegisterIR::Neutral;
        neutral.preferred_emotion = Some(GenerationEmotionIR::Concerned);
        assert!(
            expression_score(&neutral, &context, standard, persona)
                > expression_score(&warm, &context, standard, persona)
        );
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
    fn source_bound_report_is_a_replayable_verbatim_generation_trace() {
        let source = "첫 번째 근거입니다. 두 번째 근거입니다.";
        let generated = generate_source_bound_report_from_knowledge(
            LanguageCodeIR::Korean,
            source,
            &["SOURCE_SENTENCE_SHA256:test-one".to_string()],
        )
        .expect("source-bound report generation");
        assert!(generated.validate());
        assert_eq!(generated.context.tense, GenerationTenseIR::SourcePreserved);
        assert_eq!(generated.verification.unsupported_claims, 0);
        assert_eq!(generated.external_llm_calls, 0);
        assert_eq!(generated.local_teacher_calls, 0);
        assert_eq!(generated.morphology.realized_text, source);
        assert!(generated
            .meaning
            .nodes
            .iter()
            .all(|node| node.concept_id == "C_SOURCE_BOUND_REPORT"));
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
    fn receipt_acknowledgement_does_not_certify_or_echo_reported_content() {
        let refs = vec!["ACCEPTED_INFORMATION_ACT:TEST".into()];
        let korean = generate_inform_acknowledgement_from_knowledge(
            LanguageCodeIR::Korean,
            AcknowledgementContentIR::Receipt,
            &refs,
        )
        .unwrap();
        let english = generate_inform_acknowledgement_from_knowledge(
            LanguageCodeIR::English,
            AcknowledgementContentIR::Receipt,
            &refs,
        )
        .unwrap();
        assert!(korean.validate());
        assert!(english.validate());
        assert_eq!(
            korean.meaning.semantic_sha256,
            english.meaning.semantic_sha256
        );
        for generated in [&korean, &english] {
            assert_eq!(generated.meaning.nodes.len(), 1);
            assert_eq!(generated.meaning.nodes[0].concept_id, "C_ACKNOWLEDGE");
            assert_eq!(generated.meaning.nodes[0].grounding_refs, refs);
            assert!(generated.meaning.edges.is_empty());
            assert!(generated
                .speech_intent
                .intents
                .iter()
                .all(|speech| speech.intent == GenerationSpeechIntentIR::Acknowledge));
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
    fn affect_support_does_not_invent_recurrence_failure_or_action() {
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
        assert_eq!(korean.morphology.realized_text, "그 상황은 답답할 만해.");
        assert_eq!(english.morphology.realized_text, "That is frustrating.");
        for kind in [
            GenerationAffectKindIR::Frustrated,
            GenerationAffectKindIR::Angry,
            GenerationAffectKindIR::Worried,
            GenerationAffectKindIR::Hurt,
            GenerationAffectKindIR::Annoyed,
        ] {
            let generated =
                generate_affect_support_from_knowledge(LanguageCodeIR::Korean, kind).unwrap();
            assert!(generated.validate());
            assert!(!generated.meaning.nodes.iter().any(|n| matches!(
                n.concept_id.as_str(),
                "C_REPEATED_SITUATION" | "C_RECENT_FAILURE" | "C_INVITE_CHECK"
            )));
        }
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
        assert!(korean
            .morphology
            .realized_text
            .contains("원인 ‘the gateway might fail’."));
        assert!(korean
            .morphology
            .realized_text
            .contains("결과 ‘the queue grew’."));
        assert!(!korean.morphology.realized_text.contains("fail’가"));
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

    #[test]
    fn korean_dynamic_particles_cover_coda_vowel_rieul_and_numeric_surfaces() {
        for (surface, topic, subject, object, directional) in [
            ("점검", "은", "이", "을", "으로"),
            ("배포", "는", "가", "를", "로"),
            ("파일", "은", "이", "을", "로"),
            ("1", "은", "이", "을", "로"),
            ("6", "은", "이", "을", "으로"),
            ("9", "는", "가", "를", "로"),
        ] {
            assert_eq!(korean_particle(surface, "은", "는"), topic, "{surface}");
            assert_eq!(korean_particle(surface, "이", "가"), subject, "{surface}");
            assert_eq!(korean_particle(surface, "을", "를"), object, "{surface}");
            assert_eq!(korean_direction_particle(surface), directional, "{surface}");
        }
    }

    #[test]
    fn korean_aeo_connective_covers_regular_vowel_harmony_and_contractions() {
        for (stem, expected) in [
            ("가", "가"),
            ("오", "와"),
            ("보", "봐"),
            ("주", "줘"),
            ("되", "돼"),
            ("합치", "합쳐"),
            ("가리키", "가리켜"),
            ("좁히", "좁혀"),
            ("읽", "읽어"),
            ("열", "열어"),
            ("만들", "만들어"),
            ("바로잡", "바로잡아"),
            ("쓰", "써"),
            ("크", "커"),
            ("바쁘", "바빠"),
            ("알겠", "알겠어"),
        ] {
            assert_eq!(korean_aeo_connective(stem), expected, "{stem}");
        }
    }

    #[test]
    fn korean_invariable_runtime_endings_use_the_shared_regular_conjugator() {
        let verb = |stem: &str| {
            expression(
                "TEST.KO.VERB",
                LanguageCodeIR::Korean,
                "TEST_VERB",
                stem,
                ExpressionPartOfSpeechIR::Verb,
                ExpressionMorphologyClassIR::KoreanInvariable,
                LanguageRegisterIR::Neutral,
            )
        };

        assert_eq!(
            korean_conjugate(&verb("도와주"), "아야 해요"),
            "도와줘야 해요"
        );
        assert_eq!(korean_conjugate(&verb("합치"), "아"), "합쳐");
        assert_eq!(korean_conjugate(&verb("좁히"), "아"), "좁혀");
        assert_eq!(korean_conjugate(&verb("바로잡"), "아"), "바로잡아");
        assert_eq!(korean_conjugate(&verb("열"), "나요"), "여나요");
        assert_eq!(korean_conjugate(&verb("만들"), "나요"), "만드나요");
        assert_eq!(korean_conjugate(&verb("읽"), "나요"), "읽나요");

        let irregular = |stem: &str, morphology| {
            expression(
                "TEST.KO.IRREGULAR",
                LanguageCodeIR::Korean,
                "TEST_IRREGULAR_VERB",
                stem,
                ExpressionPartOfSpeechIR::Verb,
                morphology,
                LanguageRegisterIR::Neutral,
            )
        };
        let ask = irregular(
            "중단 여부를 묻",
            ExpressionMorphologyClassIR::KoreanDigeutIrregular,
        );
        assert_eq!(korean_conjugate(&ask, "아"), "중단 여부를 물어");
        assert_eq!(korean_conjugate(&ask, "ㄹ게"), "중단 여부를 물을게");
        assert_eq!(korean_conjugate(&ask, "나요"), "중단 여부를 묻나요");
        assert_eq!(korean_conjugate(&ask, "ㅂ니다"), "중단 여부를 묻습니다");

        let know = irregular("모르", ExpressionMorphologyClassIR::KoreanReuIrregular);
        assert_eq!(korean_conjugate(&know, "아"), "몰라");
        assert_eq!(korean_conjugate(&know, "ㄹ게"), "모를게");
        assert_eq!(korean_conjugate(&know, "나요"), "모르나요");
        assert_eq!(korean_conjugate(&know, "ㅂ니다"), "모릅니다");
    }
}
