//! Deterministic conversational language front-end.
//!
//! Surface noise, speech disfluency, and per-conversation references stay in
//! this adapter. They never become authority to mutate canonical semantic
//! concepts. The raw utterance is preserved, every selected normalization is
//! inspectable, and ambiguous ASR/reference bindings fail closed.

use std::collections::{BTreeMap, BTreeSet};

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::{
    ActionEvidenceReceiptIR, ActionEvidenceRequestIR, ActionPlanSeedIR, ActionPlanStatusIR,
    ActionStateAnalysisIR, ActionStateLedgerIR, ActionStateRecordIR,
};
use crate::attribution::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, AttributionGraphIR, EpistemicStatusIR,
};
use crate::clause_graph::ClauseRelationKindIR;
use crate::conditional_guard::{ConditionalGuardEvaluationIR, ConditionalGuardStoreIR};
use crate::deferred_commitment::{
    condition_sha256, normalize_condition, ConditionEvidenceDispositionIR,
    ConditionEvidenceReceiptIR, ConditionEvidenceRequestIR, DeferredActionCommitmentIR,
    DeferredCommitmentStatusIR, CONDITION_EVIDENCE_RECEIPT_SCHEMA,
};
use crate::deixis_ellipsis::{
    resolve_typed_deixis_or_ellipsis, unresolved_typed_deixis_kind, TypedDeixisEllipsisKindIR,
};
use crate::discourse_focus::{
    DiscourseFocusCandidateIR, DiscourseFocusSourceIR, DiscourseFocusStateIR,
    MAX_DISCOURSE_FOCUS_TURN_DISTANCE,
};
use crate::discourse_ontology::{
    merge_ontology_mentions, resolve_ontology_entity_reference, resolve_ontology_event_reference,
    OntologyBindingKind,
};
use crate::discourse_relations::{
    relation_connector_contains_anaphoric_that, resolve_relation_antecedent,
    DialogueRelationGraphIR,
};
use crate::epistemic::{proposition_signature, EpistemicLedgerIR, EpistemicObservationIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{
    ConditionalKindIR, ConditionalRelationIR, ModalSemanticAnalyzer, ModalWorldIR,
};
use crate::reference_resolution_graph::{
    build_reference_resolution_graph, scan_reference_mentions, ReferenceAntecedentCandidateIR,
    ReferenceMentionKindIR, ReferenceMentionNodeIR, ReferenceResolutionGraphIR,
    ReferenceSelectionHint,
};
use crate::semantic_roles::SemanticRoleGraphIR;
use crate::temporal::{TemporalGraphIR, TemporalTurnAnalysisIR};
use crate::topic_context::TopicContextGraphIR;
use crate::typed_coreference::{
    merge_typed_mentions, resolve_typed_coreference, TypedCoreferenceBindingKind,
    TypedEntityKindIR, TypedEntityReferentIR, MAX_TYPED_ENTITY_REFERENTS,
    MAX_TYPED_REFERENCE_TURN_DISTANCE,
};

pub const CONVERSATION_TURN_REQUEST_SCHEMA: &str = "B_CORE_CONVERSATION_TURN_REQUEST_1";
pub const CONVERSATION_FRONTEND_SCHEMA: &str = "B_CORE_CONVERSATION_FRONTEND_3";
pub const CONVERSATION_STATE_SCHEMA: &str = "B_CORE_CONVERSATION_STATE_32";
pub const DIALOGUE_DIRECTIVE_LEDGER_SCHEMA: &str = "B_CORE_DIALOGUE_DIRECTIVE_LEDGER_IR_1";
pub const DISCOURSE_PROGRAM_SCHEMA: &str = "B_CORE_DISCOURSE_PROGRAM_IR_4";
pub const DISCOURSE_PROGRAM_GUARD_SCHEMA: &str = "B_CORE_DISCOURSE_PROGRAM_GUARD_IR_3";
pub const GUARD_CONDITION_EXPRESSION_SCHEMA: &str = "B_CORE_GUARD_CONDITION_EXPRESSION_IR_1";
pub const TOPIC_TRANSITION_SCHEMA: &str = "B_CORE_TOPIC_TRANSITION_IR_1";
pub const TOPIC_ANCHORED_REFERENCE_SCHEMA: &str = "B_CORE_TOPIC_ANCHORED_REFERENCE_IR_1";
pub const CONVERSATIONAL_CONCEPT_SCHEMA: &str = "B_CORE_CONVERSATIONAL_CONCEPT_1";
pub const DISCOURSE_GROUP_UPDATE_SCHEMA: &str = "B_CORE_DISCOURSE_GROUP_UPDATE_IR_1";
const MAX_ALTERNATIVES: usize = 8;
const MAX_ACTIVE_REFERENTS: usize = 8;
const MAX_ACTIVE_GOALS: usize = 8;
const MAX_ACTIVE_DISCOURSE_PROGRAMS: usize = 8;
const MAX_DEFERRED_COMMITMENTS: usize = 16;
const MAX_ACTIVE_TOPICS: usize = 8;
const MAX_TOPIC_PENDING_QUESTIONS: usize = 8;
const MAX_DISCOURSE_REFERENTS: usize = 48;
const MAX_DISCOURSE_GROUPS: usize = 8;
const MAX_DIALOGUE_DIRECTIVES: usize = 32;
const MAX_REFERENCE_TURN_DISTANCE: u64 = 4;
const MAX_GOAL_ELLIPSIS_TURN_DISTANCE: u64 = 3;
const MAX_DISCOURSE_GROUP_TURN_DISTANCE: u64 = 16;
const MAX_GUARD_CONDITION_EXPRESSION_DEPTH: usize = 4;
const MAX_GUARD_CONDITION_EXPRESSION_NODES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationInputModalityIR {
    Text,
    VoiceTranscript,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceAlternativeIR {
    pub text: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationTurnRequestIR {
    pub schema: String,
    pub conversation_id: String,
    pub turn_index: u64,
    pub request_id: String,
    pub modality: ConversationInputModalityIR,
    pub raw_text: String,
    pub input_confidence_millis: u16,
    #[serde(default)]
    pub alternatives: Vec<UtteranceAlternativeIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_language: Option<LanguageCodeIR>,
    #[serde(default)]
    pub context_tags: Vec<String>,
    pub max_plan_steps: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizationOperationKindIR {
    UnicodeWidth,
    Whitespace,
    AsrCandidateSelection,
    KnownTypo,
    UniqueFuzzyMatch,
    SelfRepair,
    FillerRemoval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizationOperationIR {
    pub kind: NormalizationOperationKindIR,
    pub before: String,
    pub after: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseFunctionIR {
    Hesitation,
    HoldFloor,
    AttentionCall,
    Backchannel,
    Acknowledge,
    Approve,
    Reject,
    SelfRepair,
    Laughter,
    AffectDisplay,
    OnomatopoeicEvent,
    Greeting,
    Gratitude,
    Farewell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseEventIR {
    pub function: DiscourseFunctionIR,
    pub surface: String,
    pub semantic_concept_id: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizationCandidateIR {
    pub source_text: String,
    pub normalized_text: String,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationTurnDispositionIR {
    Grounded,
    HoldFloor,
    BackchannelOnly,
    ClarificationRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedUtteranceIR {
    pub schema: String,
    pub raw_text: String,
    pub selected_source_text: String,
    pub normalized_text: String,
    pub semantic_text: String,
    #[serde(default)]
    pub semantic_surface_text: String,
    pub candidates: Vec<NormalizationCandidateIR>,
    pub operations: Vec<NormalizationOperationIR>,
    pub discourse_events: Vec<DiscourseEventIR>,
    pub semantic_tags: Vec<String>,
    pub disposition: ConversationTurnDispositionIR,
    pub ambiguous_input: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationalConceptKindIR {
    AgentRole,
    InteractionUnit,
    DiscourseState,
    ReferenceState,
    EpistemicState,
    EventProperty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationalConceptIR {
    pub schema: String,
    pub concept_id: String,
    pub kind: ConversationalConceptKindIR,
    pub relation_targets: Vec<String>,
}

/// Small language-independent ontology. Surface forms are deliberately absent.
pub fn conversational_concept_catalog() -> Vec<ConversationalConceptIR> {
    use ConversationalConceptKindIR as Kind;
    let concept = |id: &str, kind, targets: &[&str]| ConversationalConceptIR {
        schema: CONVERSATIONAL_CONCEPT_SCHEMA.to_string(),
        concept_id: id.to_string(),
        kind,
        relation_targets: targets.iter().map(|target| (*target).to_string()).collect(),
    };
    vec![
        concept("C_DIALOGUE_SPEAKER", Kind::AgentRole, &["C_DIALOGUE_TURN"]),
        concept("C_DIALOGUE_LISTENER", Kind::AgentRole, &["C_DIALOGUE_TURN"]),
        concept(
            "C_DIALOGUE_TURN",
            Kind::InteractionUnit,
            &["C_DIALOGUE_TOPIC"],
        ),
        concept("C_DIALOGUE_TOPIC", Kind::DiscourseState, &[]),
        concept(
            "C_DIALOGUE_REFERENT",
            Kind::ReferenceState,
            &["C_DIALOGUE_TOPIC"],
        ),
        concept("C_DIALOGUE_UNCERTAINTY", Kind::EpistemicState, &[]),
        concept(
            "C_DIALOGUE_HESITATION",
            Kind::DiscourseState,
            &["C_DIALOGUE_UNCERTAINTY"],
        ),
        concept(
            "C_DIALOGUE_HOLD_FLOOR",
            Kind::DiscourseState,
            &["C_DIALOGUE_TURN"],
        ),
        concept(
            "C_DIALOGUE_ACKNOWLEDGE",
            Kind::DiscourseState,
            &["C_DIALOGUE_TURN"],
        ),
        concept(
            "C_DIALOGUE_SELF_REPAIR",
            Kind::DiscourseState,
            &["C_DIALOGUE_TURN"],
        ),
        concept(
            "C_DIALOGUE_AFFECT",
            Kind::DiscourseState,
            &["C_DIALOGUE_TURN"],
        ),
        concept(
            "C_DIALOGUE_SOCIAL_ACT",
            Kind::DiscourseState,
            &["C_DIALOGUE_TURN"],
        ),
        concept("C_WORLD_ACOUSTIC_EVENT", Kind::EventProperty, &[]),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicReferentIR {
    pub referent_id: String,
    pub surface: String,
    pub canonical_concept: String,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseBindingKindIR {
    PronominalReference,
    PluralReference,
    OrderedReference,
    LocalOrderedReference,
    LocalOrdinalReference,
    EllipticalAction,
    DiscourseProgramInstantiation,
    RepeatedGoal,
    CorrectedArgument,
    EventReference,
    EventOrdinalReference,
    PluralEventReference,
    PluralEventMemberReference,
    ResultReference,
    PropositionReference,
    PluralPropositionReference,
    LocalAntecedentReference,
    TopicReference,
    DiscourseFocusReference,
    PossessiveFocusReference,
    DemonstrativeFocusReference,
    ZeroArgumentEllipsis,
    TypedEntityReference,
    BeliefHolderReference,
    OntologyEntityReference,
    OntologyEventReference,
    DialogueRelationAntecedent,
    ClarificationAnswer,
    TopicAnchoredActionGroupReference,
    TopicAnchoredActionMemberReference,
    TopicAnchoredPropositionGroupReference,
    TopicAnchoredPropositionMemberReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TopicTransitionKindIR {
    ActivateNamed,
    ActivateGroup,
    ReturnPrevious,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseTopicAnchorKindIR {
    #[default]
    Surface,
    Concept,
    ActionGroup,
    AttributedPropositionGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseTopicIR {
    pub topic_id: String,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id_hint: Option<String>,
    #[serde(default)]
    pub explicitly_activated: bool,
    #[serde(default)]
    pub anchor_kind: DiscourseTopicAnchorKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_group_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_membership_sha256: Option<String>,
    pub introduced_turn: u64,
    pub last_activated_turn: u64,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub topic_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicTransitionIR {
    pub schema: String,
    pub kind: TopicTransitionKindIR,
    pub applied: bool,
    pub history_offset: usize,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_id_hint: Option<String>,
    #[serde(default)]
    pub anchor_kind: DiscourseTopicAnchorKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_group_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_membership_sha256: Option<String>,
    #[serde(default)]
    pub unresolved_terms: Vec<String>,
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub transition_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TopicAnchoredReferentKindIR {
    ActionGroup,
    ActionMember,
    PropositionGroup,
    PropositionMember,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TopicAnchoredSelectorKindIR {
    Ordinal,
    PredicateRole,
    Plural,
    GenericSingular,
    ZeroArgument,
    TypeMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicAnchoredReferenceIR {
    pub schema: String,
    pub applied: bool,
    pub kind: TopicAnchoredReferentKindIR,
    pub selector: TopicAnchoredSelectorKindIR,
    pub original_text: String,
    pub resolved_text: String,
    pub source_surface: String,
    pub topic_id: String,
    pub topic_sha256: String,
    pub anchor_kind: DiscourseTopicAnchorKindIR,
    pub group_id: String,
    pub group_revision: u64,
    pub membership_sha256: String,
    pub member_keys: Vec<String>,
    pub selected_member_keys: Vec<String>,
    #[serde(default)]
    pub unresolved_terms: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub resolution_sha256: String,
}

impl TopicAnchoredReferenceIR {
    pub fn validate(&self) -> bool {
        let members = self.member_keys.iter().collect::<BTreeSet<_>>();
        let selected = self.selected_member_keys.iter().collect::<BTreeSet<_>>();
        let sorted = |values: &[String]| values.windows(2).all(|window| window[0] < window[1]);
        let group_kind_matches = matches!(
            (self.anchor_kind, self.kind),
            (
                DiscourseTopicAnchorKindIR::ActionGroup,
                TopicAnchoredReferentKindIR::ActionGroup
                    | TopicAnchoredReferentKindIR::ActionMember
                    | TopicAnchoredReferentKindIR::Unresolved
            ) | (
                DiscourseTopicAnchorKindIR::AttributedPropositionGroup,
                TopicAnchoredReferentKindIR::PropositionGroup
                    | TopicAnchoredReferentKindIR::PropositionMember
                    | TopicAnchoredReferentKindIR::Unresolved
            )
        );
        self.schema == TOPIC_ANCHORED_REFERENCE_SCHEMA
            && !self.original_text.trim().is_empty()
            && !self.source_surface.trim().is_empty()
            && !self.topic_id.trim().is_empty()
            && self.topic_sha256.len() == 64
            && self
                .topic_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && !self.group_id.trim().is_empty()
            && self.group_revision > 0
            && self.membership_sha256.len() == 64
            && self
                .membership_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && !self.member_keys.is_empty()
            && members.len() == self.member_keys.len()
            && selected.len() == self.selected_member_keys.len()
            && sorted(&self.member_keys)
            && sorted(&self.selected_member_keys)
            && selected.is_subset(&members)
            && group_kind_matches
            && !self.semantic_authority
            && !self.external_execution_authorized
            && if self.applied {
                self.kind != TopicAnchoredReferentKindIR::Unresolved
                    && self.resolved_text != self.original_text
                    && !self.selected_member_keys.is_empty()
                    && self.unresolved_terms.is_empty()
            } else {
                self.kind == TopicAnchoredReferentKindIR::Unresolved
                    && self.resolved_text == self.original_text
                    && self.selected_member_keys.is_empty()
                    && !self.unresolved_terms.is_empty()
            }
            && self.resolution_sha256 == topic_anchored_reference_sha256(self)
    }
}

pub fn topic_anchored_reference_sha256(reference: &TopicAnchoredReferenceIR) -> String {
    let mut canonical = reference.clone();
    canonical.resolution_sha256.clear();
    let bytes =
        serde_json::to_vec(&canonical).expect("bounded topic-anchored reference serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn seal_topic_anchored_reference(
    mut reference: TopicAnchoredReferenceIR,
) -> TopicAnchoredReferenceIR {
    reference.member_keys.sort();
    reference.member_keys.dedup();
    reference.selected_member_keys.sort();
    reference.selected_member_keys.dedup();
    reference.unresolved_terms.sort();
    reference.unresolved_terms.dedup();
    reference.resolution_sha256 = topic_anchored_reference_sha256(&reference);
    debug_assert!(reference.validate());
    reference
}

impl TopicTransitionIR {
    pub fn validate(&self) -> bool {
        let group_fields = (
            self.anchor_group_id.as_deref(),
            self.anchor_group_revision,
            self.anchor_membership_sha256.as_deref(),
        );
        let group_anchor = matches!(
            self.anchor_kind,
            DiscourseTopicAnchorKindIR::ActionGroup
                | DiscourseTopicAnchorKindIR::AttributedPropositionGroup
        );
        self.schema == TOPIC_TRANSITION_SCHEMA
            && !self.surface.trim().is_empty()
            && !self.semantic_authority
            && !self.external_action_executed
            && self.transition_sha256 == topic_transition_sha256(self)
            && if self.applied {
                self.kind != TopicTransitionKindIR::Unresolved
                    && self.unresolved_terms.is_empty()
                    && if group_anchor {
                        matches!(
                            self.kind,
                            TopicTransitionKindIR::ActivateGroup
                                | TopicTransitionKindIR::ReturnPrevious
                        ) && matches!(group_fields, (Some(id), Some(revision), Some(hash))
                            if !id.is_empty()
                                && revision > 0
                                && hash.len() == 64
                                && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
                    } else {
                        matches!(
                            self.anchor_kind,
                            DiscourseTopicAnchorKindIR::Surface
                                | DiscourseTopicAnchorKindIR::Concept
                        ) && matches!(group_fields, (None, None, None))
                    }
            } else {
                self.kind == TopicTransitionKindIR::Unresolved
                    && !self.unresolved_terms.is_empty()
                    && matches!(group_fields, (None, None, None))
            }
    }
}

fn topic_transition_sha256(transition: &TopicTransitionIR) -> String {
    let bytes = serde_json::to_vec(&(
        TOPIC_TRANSITION_SCHEMA,
        transition.kind,
        transition.applied,
        transition.history_offset,
        &transition.surface,
        &transition.concept_id_hint,
        transition.anchor_kind,
        &transition.anchor_group_id,
        transition.anchor_group_revision,
        &transition.anchor_membership_sha256,
        &transition.unresolved_terms,
        &transition.evidence,
        transition.semantic_authority,
        transition.external_action_executed,
    ))
    .expect("bounded topic transition serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn seal_topic_transition(mut transition: TopicTransitionIR) -> TopicTransitionIR {
    transition.transition_sha256 = topic_transition_sha256(&transition);
    debug_assert!(transition.validate());
    transition
}

fn unresolved_topic_transition(surface: &str, terms: Vec<String>) -> TopicTransitionIR {
    seal_topic_transition(TopicTransitionIR {
        schema: TOPIC_TRANSITION_SCHEMA.to_string(),
        kind: TopicTransitionKindIR::Unresolved,
        applied: false,
        history_offset: 0,
        surface: surface.trim().to_string(),
        concept_id_hint: None,
        anchor_kind: DiscourseTopicAnchorKindIR::Surface,
        anchor_group_id: None,
        anchor_group_revision: None,
        anchor_membership_sha256: None,
        unresolved_terms: terms,
        evidence: vec![
            "DISCOURSE_MANAGEMENT:TOPIC_TARGET_UNRESOLVED".to_string(),
            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            "SEMANTIC_PAYLOAD_MUTATED:false".to_string(),
        ],
        semantic_authority: false,
        external_action_executed: false,
        transition_sha256: String::new(),
    })
}

fn discourse_topic_sha256(topic: &DiscourseTopicIR) -> String {
    let bytes = serde_json::to_vec(&(
        "B_CORE_DISCOURSE_TOPIC_IR_1",
        &topic.topic_id,
        &topic.surface,
        &topic.concept_id_hint,
        topic.explicitly_activated,
        topic.anchor_kind,
        &topic.anchor_group_id,
        topic.anchor_group_revision,
        &topic.anchor_membership_sha256,
        topic.introduced_turn,
        topic.last_activated_turn,
        topic.semantic_authority,
        topic.external_execution_authorized,
    ))
    .expect("bounded discourse topic serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn synchronize_active_topic_context(state: &mut ConversationStateIR, turn_index: u64) {
    let live_referent_ids = state
        .active_discourse_referents
        .iter()
        .map(|referent| referent.referent_id.clone())
        .collect::<BTreeSet<_>>();
    state
        .topic_context_graph
        .retain_live_discourse_referents(&live_referent_ids, turn_index);
    let Some(topic) = state.active_topics.first().cloned() else {
        return;
    };
    let current_focus_id = state.discourse_focus.current_focus_id.clone();
    let pending_question_id = state
        .topic_pending_questions
        .iter()
        .find(|question| question.topic_id.as_deref() == Some(topic.topic_id.as_str()))
        .map(|question| question.question_id.clone());
    let discourse_referent_ids = state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.topic_id.as_deref() == Some(topic.topic_id.as_str()))
        .map(|referent| referent.referent_id.clone())
        .collect::<Vec<_>>();
    let live_topic_ids = state
        .active_topics
        .iter()
        .map(|topic| topic.topic_id.clone())
        .collect::<Vec<_>>();
    state.topic_context_graph.refresh_active(
        &topic.topic_id,
        &topic.topic_sha256,
        current_focus_id.as_deref(),
        pending_question_id.as_deref(),
        &discourse_referent_ids,
        turn_index,
        &live_topic_ids,
    );
}

pub fn detect_topic_transition(text: &str) -> Option<TopicTransitionIR> {
    let normalized = text.trim().to_lowercase();
    if let Some(history_offset) = indexed_topic_history_offset(&normalized) {
        return Some(seal_topic_transition(TopicTransitionIR {
            schema: TOPIC_TRANSITION_SCHEMA.to_string(),
            kind: TopicTransitionKindIR::ReturnPrevious,
            applied: true,
            history_offset,
            surface: format!("TOPIC_HISTORY_OFFSET_{history_offset}"),
            concept_id_hint: None,
            anchor_kind: DiscourseTopicAnchorKindIR::Surface,
            anchor_group_id: None,
            anchor_group_revision: None,
            anchor_membership_sha256: None,
            unresolved_terms: Vec::new(),
            evidence: vec![
                "DISCOURSE_MANAGEMENT:INDEXED_TOPIC_HISTORY".to_string(),
                format!("TOPIC_HISTORY_OFFSET:{history_offset}"),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                "SEMANTIC_PAYLOAD_MUTATED:false".to_string(),
            ],
            semantic_authority: false,
            external_action_executed: false,
            transition_sha256: String::new(),
        }));
    }
    let target = if normalized
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
    {
        korean_implicit_topic_switch_target(&normalized).or_else(|| {
            [
                " 얘기로 돌아가",
                " 이야기로 돌아가",
                " 주제로 돌아가",
                " 얘기로 복귀",
                " 이야기로 복귀",
                " 주제로 복귀",
                " 얘기로 전환",
                " 이야기로 전환",
                " 주제로 전환",
                "으로 돌아가",
                "로 돌아가",
                " 얘기로 다시",
                " 이야기로 다시",
            ]
            .iter()
            .filter_map(|marker| {
                normalized.find(marker).map(|position| {
                    ["이제 ", "다시 ", "그럼 ", "그러면 "]
                        .iter()
                        .find_map(|prefix| normalized[..position].trim().strip_prefix(prefix))
                        .unwrap_or_else(|| normalized[..position].trim())
                        .trim_matches(|character: char| !character.is_alphanumeric())
                        .to_string()
                })
            })
            .find(|surface| !surface.is_empty())
        })
    } else {
        [
            "return to ",
            "go back to ",
            "back to ",
            "resume ",
            "switch to ",
        ]
        .iter()
        .filter_map(|marker| {
            normalized.find(marker).map(|position| {
                let tail = normalized[position + marker.len()..]
                    .split(['.', '?', '!', ';', ','])
                    .next()
                    .unwrap_or_default()
                    .trim();
                tail.strip_prefix("the ")
                    .unwrap_or(tail)
                    .strip_suffix(" topic")
                    .unwrap_or_else(|| tail.strip_prefix("the ").unwrap_or(tail))
                    .trim()
                    .to_string()
            })
        })
        .find(|surface| !surface.is_empty())
    }?;
    let target = restore_topic_surface_case(&target, text);
    let mut transition = if is_previous_topic_pointer(&target) {
        seal_topic_transition(TopicTransitionIR {
            schema: TOPIC_TRANSITION_SCHEMA.to_string(),
            kind: TopicTransitionKindIR::ReturnPrevious,
            applied: true,
            history_offset: 1,
            surface: clean_topic_surface(&target),
            concept_id_hint: None,
            anchor_kind: DiscourseTopicAnchorKindIR::Surface,
            anchor_group_id: None,
            anchor_group_revision: None,
            anchor_membership_sha256: None,
            unresolved_terms: Vec::new(),
            evidence: Vec::new(),
            semantic_authority: false,
            external_action_executed: false,
            transition_sha256: String::new(),
        })
    } else {
        topic_transition_from_surface(&target)
    };
    let explicit_named_switch = contains_any(
        &normalized,
        &[
            "switch to ",
            "전환",
            "현재 화제로",
            "현재 주제로",
            "주제로 두",
            "화제로 두",
        ],
    );
    transition.evidence = vec![
        match transition.kind {
            TopicTransitionKindIR::ActivateNamed if explicit_named_switch => {
                "DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_SWITCH".to_string()
            }
            TopicTransitionKindIR::ActivateNamed => {
                "DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_RETURN".to_string()
            }
            TopicTransitionKindIR::ReturnPrevious => {
                "DISCOURSE_MANAGEMENT:PREVIOUS_TOPIC_STACK".to_string()
            }
            TopicTransitionKindIR::ActivateGroup | TopicTransitionKindIR::Unresolved => {
                unreachable!("surface detector only constructs named/history transitions")
            }
        },
        "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
        "SEMANTIC_PAYLOAD_MUTATED:false".to_string(),
    ];
    transition.transition_sha256 = topic_transition_sha256(&transition);
    debug_assert!(transition.validate());
    Some(transition)
}

fn korean_implicit_topic_switch_target(text: &str) -> Option<String> {
    let (position, _marker) = ["으로 전환해", "로 전환해", "으로 전환하", "로 전환하"]
        .iter()
        .filter_map(|marker| text.find(marker).map(|position| (position, *marker)))
        .min_by_key(|(position, _)| *position)?;
    let target = ["이제 ", "다시 ", "그럼 ", "그러면 "]
        .iter()
        .find_map(|prefix| text[..position].trim().strip_prefix(prefix))
        .unwrap_or_else(|| text[..position].trim())
        .trim_matches(|character: char| !character.is_alphanumeric())
        .trim();
    let topic_head = target.split_whitespace().next_back()?;
    let discourse_noun = [
        "보고서",
        "문서",
        "로그",
        "서버",
        "서비스",
        "큐",
        "캐시",
        "인덱스",
        "배포",
        "빌드",
        "작업",
        "문제",
        "이슈",
        "프로젝트",
        "기능",
        "설정",
        "파일",
        "폴더",
        "백업",
        "워커",
        "주제",
        "이야기",
        "얘기",
    ]
    .iter()
    .any(|noun| topic_head.ends_with(noun));
    (discourse_noun && target.split_whitespace().count() <= 6).then(|| target.to_string())
}

fn topic_transition_from_surface(surface: &str) -> TopicTransitionIR {
    let cleaned = clean_topic_surface(surface);
    let concept_id_hint = topic_alias(&cleaned).map(|(concept, _, _)| concept.to_string());
    let anchor_kind = if concept_id_hint.is_some() {
        DiscourseTopicAnchorKindIR::Concept
    } else {
        DiscourseTopicAnchorKindIR::Surface
    };
    seal_topic_transition(TopicTransitionIR {
        schema: TOPIC_TRANSITION_SCHEMA.to_string(),
        kind: TopicTransitionKindIR::ActivateNamed,
        applied: true,
        history_offset: 0,
        surface: cleaned,
        concept_id_hint,
        anchor_kind,
        anchor_group_id: None,
        anchor_group_revision: None,
        anchor_membership_sha256: None,
        unresolved_terms: Vec::new(),
        evidence: vec!["DISCOURSE_TOPIC:GOAL_SUBJECT".to_string()],
        semantic_authority: false,
        external_action_executed: false,
        transition_sha256: String::new(),
    })
}

pub(crate) fn discourse_topic_id_for_transition(transition: &TopicTransitionIR) -> String {
    let identity = transition
        .anchor_group_id
        .clone()
        .or_else(|| transition.concept_id_hint.clone())
        .unwrap_or_else(|| transition.surface.to_lowercase());
    let digest = Sha256::digest(identity.as_bytes());
    format!(
        "TOPIC-{:02X}{:02X}{:02X}{:02X}",
        digest[0], digest[1], digest[2], digest[3]
    )
}

fn indexed_topic_history_offset(text: &str) -> Option<usize> {
    let return_requested = text.contains("돌아가")
        || text.contains("복귀")
        || text.contains("화제로")
        || text.contains("return")
        || text.contains("go back")
        || text.contains("resume");
    if !return_requested {
        return None;
    }
    [
        (
            5,
            [
                "다섯 주제 전",
                "다섯 단계 전",
                "5 주제 전",
                "5단계 전",
                "five topics",
                "five turns ago",
                "5 topics",
            ]
            .as_slice(),
        ),
        (
            4,
            [
                "네 주제 전",
                "네 단계 전",
                "4 주제 전",
                "4단계 전",
                "four topics",
                "four turns ago",
                "4 topics",
            ]
            .as_slice(),
        ),
        (
            3,
            [
                "세 주제 전",
                "세 단계 전",
                "3 주제 전",
                "3단계 전",
                "three topics",
                "three turns ago",
                "3 topics",
            ]
            .as_slice(),
        ),
        (
            2,
            [
                "두 주제 전",
                "두 단계 전",
                "2 주제 전",
                "2단계 전",
                "two topics",
                "two turns ago",
                "2 topics",
            ]
            .as_slice(),
        ),
    ]
    .into_iter()
    .find_map(|(offset, markers)| {
        markers
            .iter()
            .any(|marker| text.contains(marker))
            .then_some(offset)
    })
}

fn is_previous_topic_pointer(surface: &str) -> bool {
    matches!(
        clean_topic_surface(surface).to_lowercase().as_str(),
        "이전" | "아까" | "직전" | "previous" | "prior" | "earlier"
    )
}

fn clean_topic_surface(surface: &str) -> String {
    let mut cleaned = surface
        .trim()
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        .to_string();
    for suffix in ["으로", "로", "은", "는", "이", "가", "을", "를"] {
        if cleaned.chars().count() > suffix.chars().count() + 1 && cleaned.ends_with(suffix) {
            cleaned.truncate(cleaned.len() - suffix.len());
            break;
        }
    }
    cleaned
}

fn restore_topic_surface_case(surface: &str, source: &str) -> String {
    let source_tokens = source
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| {
            token.len() >= 2
                && token
                    .chars()
                    .any(|character| character.is_ascii_uppercase())
        })
        .collect::<Vec<_>>();
    if source_tokens.is_empty() {
        return surface.to_string();
    }
    let mut restored = String::with_capacity(surface.len());
    let mut token = String::new();
    let flush = |token: &mut String, restored: &mut String| {
        if let Some(original) = source_tokens
            .iter()
            .find(|original| original.eq_ignore_ascii_case(token))
        {
            restored.push_str(original);
        } else {
            restored.push_str(token);
        }
        token.clear();
    };
    for character in surface.chars() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
        } else {
            flush(&mut token, &mut restored);
            restored.push(character);
        }
    }
    flush(&mut token, &mut restored);
    restored
}

fn topic_alias(surface: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let lower = surface.to_lowercase();
    let english_tokens = lower
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .collect::<BTreeSet<_>>();
    let aliases = [
        ("TOPIC_CACHE", "캐시", "cache"),
        ("TOPIC_QUEUE", "큐", "queue"),
        ("TOPIC_BACKUP", "백업", "backup"),
        ("TOPIC_LOG", "로그", "log"),
        ("TOPIC_SERVER", "서버", "server"),
        ("TOPIC_WORKER", "워커", "worker"),
        ("C_OBJECT_FILE", "파일", "file"),
        ("C_OBJECT_FOLDER", "폴더", "folder"),
        ("C_OBJECT_SOURCE_CODE", "코드", "code"),
        ("C_OBJECT_DOCUMENT", "문서", "document"),
        ("C_OBJECT_REPORT", "보고서", "report"),
        ("C_OBJECT_PROJECT", "프로젝트", "project"),
        ("C_OBJECT_REPOSITORY", "저장소", "repository"),
    ];
    let mut matches = aliases
        .into_iter()
        .filter(|(_, korean, english)| lower.contains(korean) || english_tokens.contains(english));
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

pub(crate) fn discourse_topic_concept_id(surface: &str) -> Option<String> {
    topic_alias(surface).map(|(concept, _, _)| concept.to_string())
}

fn activate_topic(
    state: &mut ConversationStateIR,
    transition: &TopicTransitionIR,
    turn_index: u64,
) {
    debug_assert!(transition.validate() && transition.applied);
    synchronize_active_topic_context(state, turn_index);
    let outgoing_focus_id = state.discourse_focus.current_focus_id.clone();
    let introduced_turn = state
        .active_topics
        .iter()
        .find(|topic| same_topic_identity(topic, transition))
        .map_or(turn_index, |topic| topic.introduced_turn);
    state
        .active_topics
        .retain(|topic| !same_topic_identity(topic, transition));
    let explicitly_activated = matches!(
        transition.kind,
        TopicTransitionKindIR::ActivateNamed
            | TopicTransitionKindIR::ActivateGroup
            | TopicTransitionKindIR::ReturnPrevious
    ) && transition.evidence.iter().any(|evidence| {
        matches!(
            evidence.as_str(),
            "DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_RETURN"
                | "DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_SWITCH"
                | "DISCOURSE_MANAGEMENT:PREVIOUS_TOPIC_STACK"
                | "DISCOURSE_MANAGEMENT:EXPLICIT_GROUP_TOPIC"
        )
    });
    let mut topic = DiscourseTopicIR {
        topic_id: discourse_topic_id_for_transition(transition),
        surface: transition.surface.clone(),
        concept_id_hint: transition.concept_id_hint.clone(),
        explicitly_activated,
        anchor_kind: transition.anchor_kind,
        anchor_group_id: transition.anchor_group_id.clone(),
        anchor_group_revision: transition.anchor_group_revision,
        anchor_membership_sha256: transition.anchor_membership_sha256.clone(),
        introduced_turn,
        last_activated_turn: turn_index,
        semantic_authority: false,
        external_execution_authorized: false,
        topic_sha256: String::new(),
    };
    topic.topic_sha256 = discourse_topic_sha256(&topic);
    state.active_topics.insert(0, topic);
    state.active_topics.truncate(MAX_ACTIVE_TOPICS);
    let active_topic_ids = state
        .active_topics
        .iter()
        .map(|topic| topic.topic_id.as_str())
        .collect::<BTreeSet<_>>();
    state.topic_pending_questions.retain(|question| {
        question
            .topic_id
            .as_deref()
            .is_some_and(|topic_id| active_topic_ids.contains(topic_id))
    });
    let active_topic = state
        .active_topics
        .first()
        .expect("activated topic is retained")
        .clone();
    let live_topic_ids = state
        .active_topics
        .iter()
        .map(|topic| topic.topic_id.clone())
        .collect::<Vec<_>>();
    let restored_focus_id = state.topic_context_graph.activate(
        &active_topic.topic_id,
        &active_topic.topic_sha256,
        turn_index,
        outgoing_focus_id.as_deref(),
        &live_topic_ids,
        &transition.evidence,
    );
    if explicitly_activated {
        let restored = restored_focus_id.as_deref().is_some_and(|focus_id| {
            state.discourse_focus.restore_topic_focus(
                turn_index,
                focus_id,
                &["TOPIC_CONTEXT_RESUME".to_string()],
            )
        });
        if !restored {
            state.discourse_focus.apply_turn(
                turn_index,
                &[DiscourseFocusCandidateIR::explicit_topic(
                    &transition.surface,
                    transition.concept_id_hint.as_deref(),
                )],
            );
        }
    }
    synchronize_active_topic_context(state, turn_index);
}

fn same_topic_identity(topic: &DiscourseTopicIR, transition: &TopicTransitionIR) -> bool {
    if let (Some(left), Some(right)) = (
        topic.anchor_group_id.as_deref(),
        transition.anchor_group_id.as_deref(),
    ) {
        return left == right;
    }
    match (
        topic.concept_id_hint.as_deref(),
        transition.concept_id_hint.as_deref(),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => topic.surface.eq_ignore_ascii_case(&transition.surface),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseBindingIR {
    pub kind: DiscourseBindingKindIR,
    pub source_surface: String,
    pub resolved_surface: String,
    pub referent_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_goal_id: Option<String>,
    pub confidence_millis: u16,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuestionUnderDiscussionKindIR {
    VoiceAlternative,
    CompetingGoal,
    RepeatedGoal,
    PropositionReference,
    NonliteralReading,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionOptionIR {
    pub option_id: String,
    pub display_surface: String,
    pub resolved_semantic_text: String,
    #[serde(default)]
    pub referent_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<PlanIntentIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionUnderDiscussionIR {
    pub question_id: String,
    pub kind: QuestionUnderDiscussionKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
    pub source_turn: u64,
    pub source_request: String,
    pub options: Vec<QuestionOptionIR>,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionAnswerDispositionIR {
    NotApplicable,
    Resolved,
    InvalidOrNonAuthoritative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionAnswerResolutionIR {
    pub disposition: QuestionAnswerDispositionIR,
    pub resolved_semantic_text: String,
    pub binding: Option<DiscourseBindingIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationGoalFrameIR {
    pub goal_id: String,
    pub intent: PlanIntentIR,
    pub canonical_predicate: String,
    pub predicate_surface: String,
    pub subject: String,
    pub source_semantic_text: String,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GuardConditionOperatorIR {
    Atom,
    All,
    Any,
    Not,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardConditionExpressionIR {
    pub schema: String,
    pub operator: GuardConditionOperatorIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_predicate: Option<String>,
    #[serde(default)]
    pub children: Vec<GuardConditionExpressionIR>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl GuardConditionExpressionIR {
    pub fn atom(canonical_predicate: impl Into<String>) -> Self {
        Self {
            schema: GUARD_CONDITION_EXPRESSION_SCHEMA.to_string(),
            operator: GuardConditionOperatorIR::Atom,
            canonical_predicate: Some(canonical_predicate.into()),
            children: Vec::new(),
            semantic_authority: false,
            external_execution_authorized: false,
        }
    }

    pub fn composite(
        operator: GuardConditionOperatorIR,
        children: Vec<GuardConditionExpressionIR>,
    ) -> Self {
        Self {
            schema: GUARD_CONDITION_EXPRESSION_SCHEMA.to_string(),
            operator,
            canonical_predicate: None,
            children,
            semantic_authority: false,
            external_execution_authorized: false,
        }
    }

    pub fn validate(&self) -> bool {
        let mut node_count = 0;
        self.validate_bounded(1, &mut node_count)
            && node_count <= MAX_GUARD_CONDITION_EXPRESSION_NODES
    }

    fn validate_bounded(&self, depth: usize, node_count: &mut usize) -> bool {
        *node_count += 1;
        if depth > MAX_GUARD_CONDITION_EXPRESSION_DEPTH
            || *node_count > MAX_GUARD_CONDITION_EXPRESSION_NODES
            || self.schema != GUARD_CONDITION_EXPRESSION_SCHEMA
            || self.semantic_authority
            || self.external_execution_authorized
        {
            return false;
        }
        match self.operator {
            GuardConditionOperatorIR::Atom => {
                self.canonical_predicate
                    .as_deref()
                    .is_some_and(|predicate| {
                        !predicate.trim().is_empty()
                            && predicate != "UNRESOLVED"
                            && predicate.len() <= 64
                            && predicate.bytes().all(|byte| {
                                byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
                            })
                    })
                    && self.children.is_empty()
            }
            GuardConditionOperatorIR::All | GuardConditionOperatorIR::Any => {
                self.canonical_predicate.is_none()
                    && (2..=8).contains(&self.children.len())
                    && self
                        .children
                        .iter()
                        .all(|child| child.validate_bounded(depth + 1, node_count))
            }
            GuardConditionOperatorIR::Not => {
                self.canonical_predicate.is_none()
                    && self.children.len() == 1
                    && self.children[0].operator == GuardConditionOperatorIR::Atom
                    && self.children[0].validate_bounded(depth + 1, node_count)
            }
        }
    }

    pub fn legacy_predicate(&self) -> &str {
        match self.operator {
            GuardConditionOperatorIR::Atom => {
                self.canonical_predicate.as_deref().unwrap_or("UNRESOLVED")
            }
            GuardConditionOperatorIR::Not => self
                .children
                .first()
                .and_then(|child| child.canonical_predicate.as_deref())
                .map(|predicate| {
                    if predicate == "VALID" {
                        "INVALID"
                    } else {
                        predicate
                    }
                })
                .unwrap_or("COMPOUND"),
            GuardConditionOperatorIR::All | GuardConditionOperatorIR::Any => "COMPOUND",
        }
    }
}

pub fn guard_condition_expression_sha256(expression: &GuardConditionExpressionIR) -> String {
    let bytes = serde_json::to_vec(expression).expect("guard condition expression serializes");
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseProgramGuardIR {
    pub schema: String,
    pub kind: ConditionalKindIR,
    pub antecedent_surface: String,
    pub normalized_antecedent: String,
    pub condition_sha256: String,
    pub deferred_commitment_id: String,
    pub canonical_condition_predicate: String,
    pub condition_expression: GuardConditionExpressionIR,
    pub condition_expression_sha256: String,
    pub source_subject: String,
    pub antecedent_negated: bool,
    pub requires_verified_evidence: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl DiscourseProgramGuardIR {
    pub fn validate(&self, shared_subject: &str) -> bool {
        self.schema == DISCOURSE_PROGRAM_GUARD_SCHEMA
            && self.kind != ConditionalKindIR::Counterfactual
            && !self.antecedent_surface.trim().is_empty()
            && self.normalized_antecedent == normalize_condition(&self.antecedent_surface)
            && self.condition_sha256 == condition_sha256(&self.antecedent_surface)
            && self.condition_sha256.len() == 64
            && self.deferred_commitment_id.starts_with("DEFERRED-")
            && self.deferred_commitment_id.len() <= 160
            && self
                .deferred_commitment_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && !self.canonical_condition_predicate.trim().is_empty()
            && self.canonical_condition_predicate != "UNRESOLVED"
            && self.canonical_condition_predicate == self.condition_expression.legacy_predicate()
            && self.condition_expression.validate()
            && self.condition_expression_sha256
                == guard_condition_expression_sha256(&self.condition_expression)
            && self.condition_expression_sha256.len() == 64
            && self.source_subject.eq_ignore_ascii_case(shared_subject)
            && self.requires_verified_evidence
            && !self.semantic_authority
            && !self.external_execution_authorized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseProgramStepIR {
    pub position: u16,
    pub goal: ConversationGoalFrameIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation_from_previous: Option<ClauseRelationKindIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<DiscourseProgramGuardIR>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseProgramIR {
    pub schema: String,
    pub program_id: String,
    pub source_frame_count: usize,
    pub blocked_frame_count: usize,
    pub guarded_step_count: usize,
    pub shared_subject: String,
    pub steps: Vec<DiscourseProgramStepIR>,
    pub replayable: bool,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub program_sha256: String,
}

impl DiscourseProgramIR {
    pub fn validate(&self, completed_turns: u64) -> bool {
        let subjects = self
            .steps
            .iter()
            .map(|step| step.goal.subject.trim().to_lowercase())
            .collect::<BTreeSet<_>>();
        let guarded_step_count = self
            .steps
            .iter()
            .filter(|step| step.guard.is_some())
            .count();
        let replayable = self.source_frame_count >= 2
            && self.blocked_frame_count == 0
            && self.steps.len() == self.source_frame_count
            && self.steps.len() >= 2
            && self.steps.iter().any(|step| step.guard.is_none())
            && subjects.len() == 1
            && subjects.first().is_some_and(|subject| !subject.is_empty())
            && self.steps.iter().enumerate().all(|(index, step)| {
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
        self.schema == DISCOURSE_PROGRAM_SCHEMA
            && !self.program_id.trim().is_empty()
            && self.source_frame_count >= self.steps.len()
            && self.source_frame_count <= MAX_ACTIVE_GOALS
            && self.blocked_frame_count == self.source_frame_count - self.steps.len()
            && self.guarded_step_count == guarded_step_count
            && !self.shared_subject.trim().is_empty()
            && self.steps.len() <= MAX_ACTIVE_GOALS
            && self.steps.iter().enumerate().all(|(index, step)| {
                step.position == u16::try_from(index + 1).unwrap_or(u16::MAX)
                    && !step.goal.goal_id.trim().is_empty()
                    && !step.goal.canonical_predicate.trim().is_empty()
                    && step.goal.introduced_turn == self.introduced_turn
                    && step.goal.last_referenced_turn >= step.goal.introduced_turn
                    && step.goal.last_referenced_turn <= completed_turns
                    && step
                        .guard
                        .as_ref()
                        .is_none_or(|guard| guard.validate(&self.shared_subject))
                    && !step.semantic_authority
                    && !step.external_execution_authorized
            })
            && self
                .steps
                .iter()
                .map(|step| &step.goal.goal_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.steps.len()
            && self.replayable == replayable
            && self.shared_subject.eq_ignore_ascii_case(
                self.steps
                    .first()
                    .map_or("", |step| step.goal.subject.trim()),
            )
            && self.introduced_turn > 0
            && self.last_referenced_turn >= self.introduced_turn
            && self.last_referenced_turn <= completed_turns
            && !self.semantic_authority
            && !self.external_execution_authorized
            && self.program_sha256 == discourse_program_sha256(self)
    }
}

pub fn discourse_program_sha256(program: &DiscourseProgramIR) -> String {
    let bytes = serde_json::to_vec(&(
        &program.schema,
        &program.program_id,
        program.source_frame_count,
        program.blocked_frame_count,
        program.guarded_step_count,
        &program.shared_subject,
        &program.steps,
        program.replayable,
        program.introduced_turn,
        program.last_referenced_turn,
        program.semantic_authority,
        program.external_execution_authorized,
    ))
    .expect("discourse program serializes");
    format!("{:x}", Sha256::digest(bytes))
}

pub struct ConversationCommitContext<'a> {
    pub semantic_subject: Option<&'a str>,
    pub used_referent_ids: &'a [String],
    pub unresolved_reference_count: usize,
    pub language: Option<LanguageCodeIR>,
    pub grounded_goals: &'a [ConversationGoalFrameIR],
    pub proposition_referents: &'a [DynamicDiscourseReferentIR],
    pub temporal_analysis: Option<&'a TemporalTurnAnalysisIR>,
    pub guard_conditionals: Option<&'a [ConditionalRelationIR]>,
    pub semantic_role_graph: Option<&'a SemanticRoleGraphIR>,
    pub attribution_graph: Option<&'a AttributionGraphIR>,
    pub discourse_focus_candidates: &'a [DiscourseFocusCandidateIR],
}

/// A language module may propose a conversational constraint, but only this
/// typed value crosses the conversation-memory boundary. Surface forms are
/// retained as a hash for provenance; they never become the directive value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueDirectiveKindIR {
    ResponseLength,
    ResponseFormat,
    InteractionPolicy,
    GeneralConstraint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DialogueDirectiveStatusIR {
    Active,
    Superseded,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueDirectiveCandidateIR {
    pub kind: DialogueDirectiveKindIR,
    pub target_key: String,
    pub value_key: String,
    pub evidence_sha256: String,
    pub confidence_millis: u16,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl DialogueDirectiveCandidateIR {
    pub fn from_surface(
        kind: DialogueDirectiveKindIR,
        target_key: impl Into<String>,
        value_key: impl Into<String>,
        surface: &str,
        confidence_millis: u16,
    ) -> Self {
        Self {
            kind,
            target_key: target_key.into(),
            value_key: value_key.into(),
            evidence_sha256: format!("{:x}", Sha256::digest(surface.trim().as_bytes())),
            confidence_millis,
            semantic_authority: false,
            external_execution_authorized: false,
        }
    }

    pub fn validate(&self) -> bool {
        !self.target_key.trim().is_empty()
            && !self.value_key.trim().is_empty()
            && self.evidence_sha256.len() == 64
            && self
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && self.confidence_millis <= 1_000
            && !self.semantic_authority
            && !self.external_execution_authorized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueDirectiveIR {
    pub directive_id: String,
    pub kind: DialogueDirectiveKindIR,
    pub target_key: String,
    pub value_key: String,
    pub evidence_sha256: String,
    pub confidence_millis: u16,
    pub introduced_turn: u64,
    pub last_reaffirmed_turn: u64,
    pub status: DialogueDirectiveStatusIR,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl DialogueDirectiveIR {
    pub fn is_active(&self) -> bool {
        self.status == DialogueDirectiveStatusIR::Active
    }

    fn validate(&self, completed_turns: u64) -> bool {
        !self.directive_id.trim().is_empty()
            && !self.target_key.trim().is_empty()
            && !self.value_key.trim().is_empty()
            && self.evidence_sha256.len() == 64
            && self
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            && self.confidence_millis <= 1_000
            && self.introduced_turn > 0
            && self.last_reaffirmed_turn >= self.introduced_turn
            && self.last_reaffirmed_turn <= completed_turns
            && !self.semantic_authority
            && !self.external_execution_authorized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueDirectiveLedgerIR {
    pub schema: String,
    pub directives: Vec<DialogueDirectiveIR>,
    pub ledger_sha256: String,
}

impl Default for DialogueDirectiveLedgerIR {
    fn default() -> Self {
        let mut ledger = Self {
            schema: DIALOGUE_DIRECTIVE_LEDGER_SCHEMA.to_string(),
            directives: Vec::new(),
            ledger_sha256: String::new(),
        };
        ledger.rehash();
        ledger
    }
}

impl DialogueDirectiveLedgerIR {
    pub fn active(&self) -> impl Iterator<Item = &DialogueDirectiveIR> {
        self.directives
            .iter()
            .filter(|directive| directive.is_active())
    }

    fn apply_turn(
        &mut self,
        turn_index: u64,
        candidates: &[DialogueDirectiveCandidateIR],
    ) -> Result<(), ConversationFrontendError> {
        if turn_index == 0 || candidates.iter().any(|candidate| !candidate.validate()) {
            return Err(ConversationFrontendError::InvalidState);
        }
        for (index, candidate) in candidates.iter().enumerate() {
            let active_index = self.directives.iter().position(|directive| {
                directive.is_active()
                    && directive.kind == candidate.kind
                    && directive.target_key == candidate.target_key
            });
            if let Some(active_index) = active_index {
                if self.directives[active_index].value_key == candidate.value_key {
                    let existing = &mut self.directives[active_index];
                    existing.last_reaffirmed_turn = turn_index;
                    existing.confidence_millis =
                        existing.confidence_millis.max(candidate.confidence_millis);
                    existing.evidence_sha256 = candidate.evidence_sha256.clone();
                    continue;
                }
                self.directives[active_index].status = DialogueDirectiveStatusIR::Superseded;
            }
            self.directives.push(DialogueDirectiveIR {
                directive_id: format!("DIALOGUE-DIRECTIVE-{turn_index:06}-{:02}", index + 1),
                kind: candidate.kind,
                target_key: candidate.target_key.clone(),
                value_key: candidate.value_key.clone(),
                evidence_sha256: candidate.evidence_sha256.clone(),
                confidence_millis: candidate.confidence_millis,
                introduced_turn: turn_index,
                last_reaffirmed_turn: turn_index,
                status: DialogueDirectiveStatusIR::Active,
                semantic_authority: false,
                external_execution_authorized: false,
            });
        }
        self.directives.sort_by(|left, right| {
            right
                .is_active()
                .cmp(&left.is_active())
                .then_with(|| right.last_reaffirmed_turn.cmp(&left.last_reaffirmed_turn))
                .then_with(|| left.directive_id.cmp(&right.directive_id))
        });
        self.directives.truncate(MAX_DIALOGUE_DIRECTIVES);
        self.rehash();
        Ok(())
    }

    fn rehash(&mut self) {
        let bytes = serde_json::to_vec(&(&self.schema, &self.directives))
            .expect("dialogue directive ledger serializes");
        self.ledger_sha256 = format!("{:x}", Sha256::digest(bytes));
    }

    fn validate(&self, completed_turns: u64) -> bool {
        let ids = self
            .directives
            .iter()
            .map(|directive| directive.directive_id.as_str())
            .collect::<BTreeSet<_>>();
        let active_axes = self
            .active()
            .map(|directive| (directive.kind, directive.target_key.as_str()))
            .collect::<BTreeSet<_>>();
        let active_count = self.active().count();
        let mut canonical = self.clone();
        canonical.rehash();
        self.schema == DIALOGUE_DIRECTIVE_LEDGER_SCHEMA
            && self.directives.len() <= MAX_DIALOGUE_DIRECTIVES
            && ids.len() == self.directives.len()
            && active_axes.len() == active_count
            && self
                .directives
                .iter()
                .all(|directive| directive.validate(completed_turns))
            && self.ledger_sha256 == canonical.ledger_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseReferentKindIR {
    Event,
    Result,
    Proposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicDiscourseReferentIR {
    pub referent_id: String,
    pub kind: DiscourseReferentKindIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
    pub semantic_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributed_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution_attitude: Option<AttributionAttitudeIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epistemic_status: Option<EpistemicStatusIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposition_polarity: Option<AttributedPropositionPolarityIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modal_world: Option<ModalWorldIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub belief_record_id: Option<String>,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseGroupKindIR {
    Action,
    AttributedProposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseGroupIR {
    pub group_id: String,
    pub kind: DiscourseGroupKindIR,
    pub member_keys: Vec<String>,
    #[serde(default)]
    pub topic_keys: Vec<String>,
    pub revision: u64,
    #[serde(default)]
    pub component_group_ids: Vec<String>,
    pub membership_sha256: String,
    pub introduced_turn: u64,
    pub last_referenced_turn: u64,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseGroupUpdateOperationIR {
    AddMember,
    RemoveMember,
    MergeGroups,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseGroupUpdateIR {
    pub schema: String,
    pub operation: DiscourseGroupUpdateOperationIR,
    pub applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_group_id: Option<String>,
    #[serde(default)]
    pub source_group_ids: Vec<String>,
    #[serde(default)]
    pub before_member_keys: Vec<String>,
    #[serde(default)]
    pub after_member_keys: Vec<String>,
    #[serde(default)]
    pub added_member_keys: Vec<String>,
    #[serde(default)]
    pub removed_member_keys: Vec<String>,
    pub revision: u64,
    #[serde(default)]
    pub unresolved_terms: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub update_sha256: String,
}

impl DiscourseGroupUpdateIR {
    pub fn validate(&self) -> bool {
        let source_ids = self.source_group_ids.iter().collect::<BTreeSet<_>>();
        let before = self.before_member_keys.iter().collect::<BTreeSet<_>>();
        let after = self.after_member_keys.iter().collect::<BTreeSet<_>>();
        let added = self.added_member_keys.iter().collect::<BTreeSet<_>>();
        let removed = self.removed_member_keys.iter().collect::<BTreeSet<_>>();
        let strictly_sorted =
            |values: &[String]| values.windows(2).all(|window| window[0] < window[1]);
        self.schema == DISCOURSE_GROUP_UPDATE_SCHEMA
            && source_ids.len() == self.source_group_ids.len()
            && before.len() == self.before_member_keys.len()
            && after.len() == self.after_member_keys.len()
            && added.len() == self.added_member_keys.len()
            && removed.len() == self.removed_member_keys.len()
            && strictly_sorted(&self.source_group_ids)
            && strictly_sorted(&self.before_member_keys)
            && strictly_sorted(&self.after_member_keys)
            && strictly_sorted(&self.added_member_keys)
            && strictly_sorted(&self.removed_member_keys)
            && self.before_member_keys.len() <= MAX_ACTIVE_GOALS
            && self.after_member_keys.len() <= MAX_ACTIVE_GOALS
            && !self.semantic_authority
            && !self.external_action_executed
            && if self.applied {
                self.operation != DiscourseGroupUpdateOperationIR::Unresolved
                    && self
                        .target_group_id
                        .as_deref()
                        .is_some_and(|id| !id.is_empty())
                    && !self.source_group_ids.is_empty()
                    && self.after_member_keys.len() >= 2
                    && self.unresolved_terms.is_empty()
                    && self.revision > 0
                    && match self.operation {
                        DiscourseGroupUpdateOperationIR::AddMember => {
                            self.source_group_ids.len() == 1
                                && self.target_group_id.as_ref() == self.source_group_ids.first()
                                && self.added_member_keys.len() == 1
                                && self.removed_member_keys.is_empty()
                                && !before.contains(&self.added_member_keys[0])
                                && after.contains(&self.added_member_keys[0])
                                && self
                                    .before_member_keys
                                    .iter()
                                    .all(|member| after.contains(member))
                                && self.after_member_keys.len() == self.before_member_keys.len() + 1
                        }
                        DiscourseGroupUpdateOperationIR::RemoveMember => {
                            self.source_group_ids.len() == 1
                                && self.target_group_id.as_ref() == self.source_group_ids.first()
                                && self.removed_member_keys.len() == 1
                                && self.added_member_keys.is_empty()
                                && before.contains(&self.removed_member_keys[0])
                                && !after.contains(&self.removed_member_keys[0])
                                && self
                                    .after_member_keys
                                    .iter()
                                    .all(|member| before.contains(member))
                                && self.before_member_keys.len() == self.after_member_keys.len() + 1
                        }
                        DiscourseGroupUpdateOperationIR::MergeGroups => {
                            self.source_group_ids.len() == 2
                                && self.before_member_keys.is_empty()
                                && self.added_member_keys.is_empty()
                                && self.removed_member_keys.is_empty()
                                && self.revision == 1
                        }
                        DiscourseGroupUpdateOperationIR::Unresolved => false,
                    }
            } else {
                self.operation == DiscourseGroupUpdateOperationIR::Unresolved
                    && !self.unresolved_terms.is_empty()
                    && self.revision == 0
            }
            && self.update_sha256 == discourse_group_update_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationStateIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer_focus: Option<crate::discourse_qa::AnswerFocusIR>,
    #[serde(default)]
    pub dialogue_world: crate::world_dialogue::DialogueWorldIR,
    pub conversation_id: String,
    pub completed_turns: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_subject: Option<String>,
    pub active_referents: Vec<DynamicReferentIR>,
    #[serde(default)]
    pub active_topics: Vec<DiscourseTopicIR>,
    #[serde(default)]
    pub discourse_focus: DiscourseFocusStateIR,
    #[serde(default)]
    pub topic_context_graph: TopicContextGraphIR,
    #[serde(default)]
    pub active_goals: Vec<ConversationGoalFrameIR>,
    #[serde(default)]
    pub active_discourse_programs: Vec<DiscourseProgramIR>,
    #[serde(default)]
    pub action_state_ledger: ActionStateLedgerIR,
    #[serde(default)]
    pub deferred_action_commitments: Vec<DeferredActionCommitmentIR>,
    #[serde(default)]
    pub active_discourse_referents: Vec<DynamicDiscourseReferentIR>,
    #[serde(default)]
    pub active_discourse_groups: Vec<DiscourseGroupIR>,
    #[serde(default)]
    pub active_typed_entities: Vec<TypedEntityReferentIR>,
    #[serde(default)]
    pub epistemic_ledger: EpistemicLedgerIR,
    #[serde(default)]
    pub temporal_graph: TemporalGraphIR,
    #[serde(default)]
    pub conditional_guard_store: ConditionalGuardStoreIR,
    #[serde(default)]
    pub dialogue_relation_graph: DialogueRelationGraphIR,
    #[serde(default)]
    pub dialogue_directive_ledger: DialogueDirectiveLedgerIR,
    #[serde(default)]
    pub last_guard_evaluations: Vec<ConditionalGuardEvaluationIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_language: Option<LanguageCodeIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_question: Option<QuestionUnderDiscussionIR>,
    #[serde(default)]
    pub topic_pending_questions: Vec<QuestionUnderDiscussionIR>,
    pub unresolved_reference_count: usize,
    pub state_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceResolutionIR {
    pub original_semantic_text: String,
    pub resolved_semantic_text: String,
    pub resolved_reference_count: usize,
    pub used_referent_ids: Vec<String>,
    pub ambiguous_reference_surfaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic_anchored_resolution: Option<TopicAnchoredReferenceIR>,
    #[serde(default)]
    pub discourse_bindings: Vec<DiscourseBindingIR>,
    #[serde(default)]
    pub resolution_graph: ReferenceResolutionGraphIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversationFrontendError {
    InvalidRequest,
    TurnOrder,
    InvalidState,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UtteranceNormalizer;

impl UtteranceNormalizer {
    pub fn normalize(
        &self,
        request: &ConversationTurnRequestIR,
    ) -> Result<NormalizedUtteranceIR, ConversationFrontendError> {
        validate_turn_request(request)?;
        let mut sources = vec![UtteranceAlternativeIR {
            text: request.raw_text.clone(),
            confidence_millis: request.input_confidence_millis,
        }];
        sources.extend(request.alternatives.clone());
        sources.sort_by(|left, right| {
            right
                .confidence_millis
                .cmp(&left.confidence_millis)
                .then_with(|| left.text.cmp(&right.text))
        });
        let mut candidates = sources
            .iter()
            .map(|source| NormalizationCandidateIR {
                source_text: source.text.clone(),
                normalized_text: normalize_surface(&source.text).0,
                confidence_millis: source.confidence_millis,
            })
            .collect::<Vec<_>>();
        candidates.dedup_by(|left, right| left.normalized_text == right.normalized_text);
        let selected = candidates
            .first()
            .cloned()
            .ok_or(ConversationFrontendError::InvalidRequest)?;
        let ambiguous_input = request.modality == ConversationInputModalityIR::VoiceTranscript
            && candidates.get(1).is_some_and(|second| {
                selected.normalized_text != second.normalized_text
                    && selected
                        .confidence_millis
                        .saturating_sub(second.confidence_millis)
                        <= 50
            });

        let (mut normalized_text, surface_changed) = normalize_surface(&selected.source_text);
        let mut operations = Vec::new();
        if request.modality == ConversationInputModalityIR::VoiceTranscript
            && (!request.alternatives.is_empty()
                || selected.source_text.trim() != request.raw_text.trim())
        {
            operations.push(NormalizationOperationIR {
                kind: NormalizationOperationKindIR::AsrCandidateSelection,
                before: request.raw_text.clone(),
                after: selected.source_text.clone(),
                confidence_millis: selected.confidence_millis,
            });
        }
        if surface_changed {
            operations.push(NormalizationOperationIR {
                kind: NormalizationOperationKindIR::Whitespace,
                before: selected.source_text.clone(),
                after: normalized_text.clone(),
                confidence_millis: 1_000,
            });
        }

        let mut discourse_events = Vec::new();
        if let Some((before, repaired)) = apply_self_repair(&normalized_text) {
            operations.push(NormalizationOperationIR {
                kind: NormalizationOperationKindIR::SelfRepair,
                before,
                after: repaired.clone(),
                confidence_millis: 930,
            });
            discourse_events.push(event(
                DiscourseFunctionIR::SelfRepair,
                "self-repair",
                "C_DIALOGUE_SELF_REPAIR",
                930,
            ));
            normalized_text = repaired;
        }

        if let Some(function) = standalone_discourse_phrase(&normalized_text) {
            let surface = normalized_text.clone();
            let concept_id = match function {
                DiscourseFunctionIR::HoldFloor | DiscourseFunctionIR::Hesitation => {
                    "C_DIALOGUE_HOLD_FLOOR"
                }
                _ => "C_DIALOGUE_ACKNOWLEDGE",
            };
            discourse_events.push(event(function, &surface, concept_id, 940));
            operations.push(NormalizationOperationIR {
                kind: NormalizationOperationKindIR::FillerRemoval,
                before: surface,
                after: String::new(),
                confidence_millis: 940,
            });
            normalized_text.clear();
        }

        let mut semantic_tokens = Vec::new();
        let tokens = tokenize(&normalized_text);
        let token_count = tokens.len();
        let mut semantic_replacements = Vec::with_capacity(token_count);
        for (index, token) in tokens.iter().cloned().enumerate() {
            let lower = token.to_lowercase();
            if let Some(function) = filler_function(&lower, index, token_count) {
                let concept_id = match function {
                    DiscourseFunctionIR::AttentionCall => "C_DIALOGUE_TURN",
                    DiscourseFunctionIR::HoldFloor => "C_DIALOGUE_HOLD_FLOOR",
                    _ => "C_DIALOGUE_HESITATION",
                };
                discourse_events.push(event(function, &token, concept_id, 900));
                operations.push(NormalizationOperationIR {
                    kind: NormalizationOperationKindIR::FillerRemoval,
                    before: token,
                    after: String::new(),
                    confidence_millis: 900,
                });
                semantic_replacements.push(None);
                continue;
            }
            if let Some(function) = backchannel_function(&lower) {
                discourse_events.push(event(function, &token, "C_DIALOGUE_ACKNOWLEDGE", 920));
                if token_count == 1 || semantic_tokens.is_empty() && index + 1 == token_count {
                    semantic_replacements.push(None);
                    continue;
                }
            }
            if let Some(function) = social_function(&lower) {
                let ambiguous_korean_nominal = lower == "감사" && token_count > 1;
                if !ambiguous_korean_nominal {
                    discourse_events.push(event(function, &token, "C_DIALOGUE_SOCIAL_ACT", 950));
                    semantic_replacements.push(None);
                    continue;
                }
            }
            if is_laughter(&lower) {
                discourse_events.push(event(
                    DiscourseFunctionIR::Laughter,
                    &token,
                    "C_DIALOGUE_AFFECT",
                    970,
                ));
                semantic_replacements.push(None);
                continue;
            }
            if let Some((canonical, confidence, kind)) =
                repair_token_in_context(&lower, index, &tokens)
            {
                operations.push(NormalizationOperationIR {
                    kind,
                    before: token,
                    after: canonical.clone(),
                    confidence_millis: confidence,
                });
                semantic_replacements.push(Some(canonical.clone()));
                semantic_tokens.push(canonical);
            } else {
                semantic_replacements.push(Some(token.clone()));
                semantic_tokens.push(token);
            }
        }
        let semantic_text = semantic_tokens.join(" ");
        let semantic_surface_text =
            reconstruct_semantic_surface(&normalized_text, &tokens, &semantic_replacements);
        let mut semantic_tags = BTreeSet::new();
        for token in &semantic_tokens {
            if let Some(tag) = onomatopoeia_tag(&token.to_lowercase()) {
                semantic_tags.insert(tag.to_string());
                semantic_tags.insert("acoustic_event".to_string());
                discourse_events.push(event(
                    DiscourseFunctionIR::OnomatopoeicEvent,
                    token,
                    "C_WORLD_ACOUSTIC_EVENT",
                    900,
                ));
            }
        }
        for event in &discourse_events {
            semantic_tags.insert(event.semantic_concept_id.clone());
        }
        let has_backchannel = discourse_events.iter().any(|item| {
            matches!(
                item.function,
                DiscourseFunctionIR::Backchannel
                    | DiscourseFunctionIR::Acknowledge
                    | DiscourseFunctionIR::Approve
                    | DiscourseFunctionIR::Reject
                    | DiscourseFunctionIR::Greeting
                    | DiscourseFunctionIR::Gratitude
                    | DiscourseFunctionIR::Farewell
            )
        });
        let has_hesitation = discourse_events.iter().any(|item| {
            matches!(
                item.function,
                DiscourseFunctionIR::Hesitation | DiscourseFunctionIR::HoldFloor
            )
        });
        let disposition = if ambiguous_input {
            ConversationTurnDispositionIR::ClarificationRequired
        } else if semantic_text.is_empty() && has_backchannel {
            ConversationTurnDispositionIR::BackchannelOnly
        } else if semantic_text.is_empty() && has_hesitation {
            ConversationTurnDispositionIR::HoldFloor
        } else if semantic_text.is_empty() {
            ConversationTurnDispositionIR::ClarificationRequired
        } else {
            ConversationTurnDispositionIR::Grounded
        };
        Ok(NormalizedUtteranceIR {
            schema: CONVERSATION_FRONTEND_SCHEMA.to_string(),
            raw_text: request.raw_text.clone(),
            selected_source_text: selected.source_text,
            normalized_text,
            semantic_text,
            semantic_surface_text,
            candidates,
            operations,
            discourse_events,
            semantic_tags: semantic_tags.into_iter().collect(),
            disposition,
            ambiguous_input,
        })
    }
}

fn validate_turn_request(
    request: &ConversationTurnRequestIR,
) -> Result<(), ConversationFrontendError> {
    let valid_id = |value: &str| {
        !value.trim().is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
    };
    if request.schema != CONVERSATION_TURN_REQUEST_SCHEMA
        || !valid_id(&request.conversation_id)
        || !valid_id(&request.request_id)
        || request.turn_index == 0
        || request.raw_text.trim().is_empty()
        || request.raw_text.len() > 64 * 1024
        || request.input_confidence_millis > 1_000
        || request.alternatives.len() > MAX_ALTERNATIVES
        || request.alternatives.iter().any(|alternative| {
            alternative.text.trim().is_empty()
                || alternative.text.len() > 64 * 1024
                || alternative.confidence_millis > 1_000
        })
        || request.context_tags.len() > 64
        || request
            .context_tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 128)
        || !(5..=32).contains(&request.max_plan_steps)
    {
        return Err(ConversationFrontendError::InvalidRequest);
    }
    if request.modality == ConversationInputModalityIR::Text
        && (!request.alternatives.is_empty() || request.input_confidence_millis != 1_000)
    {
        return Err(ConversationFrontendError::InvalidRequest);
    }
    Ok(())
}

fn normalize_surface(text: &str) -> (String, bool) {
    let width_normalized = text
        .chars()
        .map(|character| match character {
            '\u{3000}' => ' ',
            '\u{ff01}'..='\u{ff5e}' => char::from_u32(u32::from(character) - 0xfee0)
                .expect("full-width ASCII maps to ASCII"),
            _ => character,
        })
        .collect::<String>();
    let normalized = width_normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let changed = normalized != text.trim().to_lowercase() || width_normalized != text;
    (normalized, changed)
}

fn apply_self_repair(text: &str) -> Option<(String, String)> {
    const MARKERS: [&str; 8] = [
        ", 아니 ",
        " 아니, ",
        " 아니 ",
        ", no, ",
        " no, ",
        " i mean ",
        ", rather ",
        " rather, ",
    ];
    MARKERS
        .iter()
        .filter_map(|marker| text.rfind(marker).map(|index| (index, *marker)))
        .filter(|(index, _)| *index > 0)
        .max_by_key(|(index, _)| *index)
        .and_then(|(index, marker)| {
            let repaired = text[index + marker.len()..].trim();
            (!repaired.is_empty()).then(|| (text.to_string(), repaired.to_string()))
        })
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                ',' | '.'
                    | '!'
                    | '?'
                    | ';'
                    | ':'
                    | '…'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '"'
                    | '\''
                    | '‘'
                    | '’'
                    | '“'
                    | '”'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
            )
    })
    .filter(|token| !token.is_empty())
    .map(ToString::to_string)
    .collect()
}

fn filler_function(token: &str, index: usize, token_count: usize) -> Option<DiscourseFunctionIR> {
    if matches!(token, "음" | "음음" | "um" | "umm" | "hmm" | "흠") {
        return Some(if token_count == 1 {
            DiscourseFunctionIR::HoldFloor
        } else {
            DiscourseFunctionIR::Hesitation
        });
    }
    if matches!(token, "어" | "어어" | "uh" | "uhh" | "er") && index == 0 {
        return Some(if token_count == 1 {
            DiscourseFunctionIR::HoldFloor
        } else {
            DiscourseFunctionIR::Hesitation
        });
    }
    if matches!(token, "잠깐" | "잠깐만" | "잠시" | "wait" | "hold") && token_count <= 2 {
        return Some(DiscourseFunctionIR::HoldFloor);
    }
    if matches!(token, "저기" | "excuse" | "well") && index == 0 && token_count > 1 {
        return Some(DiscourseFunctionIR::AttentionCall);
    }
    None
}

fn backchannel_function(token: &str) -> Option<DiscourseFunctionIR> {
    match token {
        "응" | "네" | "넵" | "그래" | "알겠어" | "알겠습니다" | "yeah" | "yep" | "yes" | "okay"
        | "ok" | "noted" => Some(DiscourseFunctionIR::Acknowledge),
        "좋아" | "맞아" | "맞습니다" | "ㅇㅋ" | "good" | "right" | "correct" => {
            Some(DiscourseFunctionIR::Approve)
        }
        "아니" | "아니야" | "ㄴㄴ" | "no" | "nope" => Some(DiscourseFunctionIR::Reject),
        _ => None,
    }
}

fn standalone_discourse_phrase(text: &str) -> Option<DiscourseFunctionIR> {
    let normalized = text
        .trim()
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, '.' | ',' | '!' | '?' | '…' | '~')
        })
        .to_lowercase()
        .replace([',', '…'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    match normalized.as_str() {
        "one moment"
        | "just a moment"
        | "hold on a moment"
        | "let me think"
        | "um let me think"
        | "uh let me think"
        | "잠깐 생각할게"
        | "잠깐 생각해볼게"
        | "잠시 생각할게"
        | "음 잠깐만"
        | "음 그러니까"
        | "어 그러니까"
        | "음 그러면"
        | "어 그러면"
        | "저기" => Some(DiscourseFunctionIR::HoldFloor),
        "you're welcome" | "you are welcome" | "천만에" | "별말씀을" => {
            Some(DiscourseFunctionIR::Acknowledge)
        }
        _ => None,
    }
}

fn social_function(token: &str) -> Option<DiscourseFunctionIR> {
    match token {
        "안녕" | "안녕하세요" | "반가워" | "hello" | "hi" | "hey" => {
            Some(DiscourseFunctionIR::Greeting)
        }
        "고마워" | "고맙다" | "고마워요" | "감사" | "감사해" | "감사합니다" | "thanks"
        | "thankyou" | "thank" | "thx" => Some(DiscourseFunctionIR::Gratitude),
        "잘가" | "안녕히" | "바이" | "bye" | "goodbye" => {
            Some(DiscourseFunctionIR::Farewell)
        }
        _ => None,
    }
}

fn is_laughter(token: &str) -> bool {
    matches!(
        token,
        "ㅋㅋ" | "ㅋㅋㅋ" | "ㅎㅎ" | "ㅎㅎㅎ" | "lol" | "haha" | "hehe"
    )
}

fn event(
    function: DiscourseFunctionIR,
    surface: &str,
    semantic_concept_id: &str,
    confidence_millis: u16,
) -> DiscourseEventIR {
    DiscourseEventIR {
        function,
        surface: surface.to_string(),
        semantic_concept_id: semantic_concept_id.to_string(),
        confidence_millis,
    }
}

fn repair_token(token: &str) -> Option<(String, u16, NormalizationOperationKindIR)> {
    let known = match token {
        "고처" => Some("고쳐"),
        "고처줘" => Some("고쳐줘"),
        "고처주세요" => Some("고쳐주세요"),
        "확잏" => Some("확인"),
        "해결헤" => Some("해결해"),
        "만드러" => Some("만들어"),
        "되요" => Some("돼요"),
        "됬어" => Some("됐어"),
        "됬습니다" => Some("됐습니다"),
        "plese" => Some("please"),
        "teh" => Some("the"),
        "chek" => Some("check"),
        "finsh" => Some("finish"),
        "udpate" => Some("update"),
        "isntall" => Some("install"),
        _ => None,
    };
    if let Some(canonical) = known {
        return Some((
            canonical.to_string(),
            990,
            NormalizationOperationKindIR::KnownTypo,
        ));
    }
    const CANONICAL_CONTROL_FORMS: [&str; 24] = [
        "계획",
        "확인",
        "수리",
        "고쳐",
        "구현",
        "추가",
        "설명",
        "실행",
        "파일",
        "폴더",
        "코드",
        "문서",
        "보고서",
        "project",
        "please",
        "check",
        "repair",
        "explain",
        "create",
        "execute",
        "file",
        "folder",
        "code",
        "document",
    ];
    const KOREAN_PARTICLES: [&str; 14] = [
        "은", "는", "이", "가", "을", "를", "와", "과", "에", "에서", "로", "도", "만", "의",
    ];
    const KOREAN_GRAMMAR_STEMS: [&str; 1] = ["보고"];
    const DISCOURSE_GRAMMAR_FORMS: [&str; 13] = [
        "older", "oldest", "newer", "newest", "later", "latest", "earlier", "earliest", "former",
        "latter", "first", "second", "recent",
    ];
    if DISCOURSE_GRAMMAR_FORMS.contains(&token) {
        return None;
    }
    if !token.is_ascii()
        && KOREAN_GRAMMAR_STEMS
            .iter()
            .any(|stem| token.starts_with(stem))
    {
        return None;
    }
    if !token.is_ascii()
        && KOREAN_PARTICLES.iter().any(|particle| {
            token.strip_suffix(particle).is_some_and(|stem| {
                CANONICAL_CONTROL_FORMS.contains(&stem) || KOREAN_GRAMMAR_STEMS.contains(&stem)
            })
        })
    {
        return None;
    }
    if CANONICAL_CONTROL_FORMS
        .iter()
        .any(|candidate| token != *candidate && token.starts_with(candidate))
    {
        return None;
    }
    // Open-vocabulary text is not a closed command lexicon. A valid unseen
    // word such as `fire` must never be rewritten to a one-edit control word
    // such as `file`. Only the auditable high-confidence table above repairs
    // spelling; uncertain forms remain intact for semantic disambiguation.
    None
}

fn repair_token_in_context(
    token: &str,
    index: usize,
    tokens: &[String],
) -> Option<(String, u16, NormalizationOperationKindIR)> {
    if token == "ya" && index > 0 && index + 1 < tokens.len() {
        let auxiliary = tokens[index - 1].to_lowercase();
        if matches!(
            auxiliary.as_str(),
            "can" | "could" | "would" | "will" | "do" | "did" | "are" | "were" | "have"
        ) {
            return Some((
                "you".to_string(),
                980,
                NormalizationOperationKindIR::KnownTypo,
            ));
        }
    }
    repair_token(token).or_else(|| unique_fuzzy_control_form(token, index, tokens))
}

fn unique_fuzzy_control_form(
    token: &str,
    index: usize,
    tokens: &[String],
) -> Option<(String, u16, NormalizationOperationKindIR)> {
    const CONTROL_FORMS: [&str; 11] = [
        "inspect", "review", "repair", "explain", "create", "execute", "update", "install",
        "finish", "remove", "delete",
    ];
    const REQUEST_CONTEXT: [&str; 16] = [
        "can", "could", "would", "will", "do", "did", "please", "to", "just", "then", "and",
        "kindly", "lets", "let's", "you", "ya",
    ];
    if !token.is_ascii() || token.len() < 5 || index + 1 >= tokens.len() {
        return None;
    }
    let context_allows_repair =
        index == 0 || REQUEST_CONTEXT.contains(&tokens[index - 1].to_lowercase().as_str());
    if !context_allows_repair {
        return None;
    }
    let mut matches = CONTROL_FORMS
        .iter()
        .copied()
        .filter(|candidate| *candidate != token && edit_distance_at_most_one(token, candidate))
        .collect::<Vec<_>>();
    matches.sort_unstable();
    matches.dedup();
    (matches.len() == 1).then(|| {
        (
            matches[0].to_string(),
            960,
            NormalizationOperationKindIR::UniqueFuzzyMatch,
        )
    })
}

fn edit_distance_at_most_one(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len().abs_diff(right.len()) > 1 {
        return false;
    }
    if left.len() == right.len() {
        let mismatches = left
            .iter()
            .zip(right)
            .enumerate()
            .filter_map(|(index, (left, right))| (left != right).then_some(index))
            .collect::<Vec<_>>();
        return mismatches.len() == 1
            || mismatches.len() == 2
                && mismatches[1] == mismatches[0] + 1
                && left[mismatches[0]] == right[mismatches[1]]
                && left[mismatches[1]] == right[mismatches[0]];
    }
    let (shorter, longer) = if left.len() < right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let mut shorter_index = 0;
    let mut longer_index = 0;
    let mut skipped = false;
    while shorter_index < shorter.len() && longer_index < longer.len() {
        if shorter[shorter_index] == longer[longer_index] {
            shorter_index += 1;
            longer_index += 1;
        } else if skipped {
            return false;
        } else {
            skipped = true;
            longer_index += 1;
        }
    }
    true
}

fn onomatopoeia_tag(token: &str) -> Option<&'static str> {
    match token {
        "쿵" | "쾅" | "bang" | "boom" => Some("impact_sound"),
        "딸깍" | "철컥" | "click" | "clack" => Some("mechanical_switch_sound"),
        "삐" | "beep" => Some("electronic_alert_sound"),
        "웅웅" | "buzz" | "humming" => Some("continuous_vibration_sound"),
        "슥" | "swish" => Some("light_motion_sound"),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConversationMemory {
    states: BTreeMap<String, ConversationStateIR>,
}

impl ConversationMemory {
    /// Explicit host registration, separate from natural-language dialogue.
    /// Validate a clone first: failure cannot create a session or advance a turn.
    pub fn update_world_vocabulary(
        &mut self,
        conversation_id: &str,
        update: &crate::world_vocabulary::WorldVocabularyUpdateIR,
    ) -> Result<ConversationStateIR, String> {
        if conversation_id.trim().is_empty() || conversation_id.len() > 128 {
            return Err("INVALID_CONVERSATION_ID".into());
        }
        let mut state = self
            .states
            .get(conversation_id)
            .cloned()
            .unwrap_or_else(|| empty_state(conversation_id));
        state.dialogue_world.vocabulary = state.dialogue_world.vocabulary.updated(update)?;
        state.state_sha256 = state_hash(&state).map_err(|_| "INVALID_STATE_HASH")?;
        validate_conversation_state(&state).map_err(|_| "INVALID_VOCABULARY_STATE")?;
        self.states.insert(conversation_id.into(), state.clone());
        Ok(state)
    }

    pub(crate) fn restore_turn_state(
        &mut self,
        conversation_id: &str,
        state: Option<ConversationStateIR>,
    ) {
        if let Some(state) = state {
            self.states.insert(conversation_id.to_string(), state);
        } else {
            self.states.remove(conversation_id);
        }
    }

    pub fn state(&self, conversation_id: &str) -> Option<&ConversationStateIR> {
        self.states.get(conversation_id)
    }

    pub(crate) fn commit_answer_focus(
        &mut self,
        conversation_id: &str,
        focus: Option<crate::discourse_qa::AnswerFocusIR>,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if focus
            .as_ref()
            .is_some_and(|focus| !focus.validate(state.completed_turns))
        {
            return Err(ConversationFrontendError::InvalidState);
        }
        state.answer_focus = focus;
        state.state_sha256 = state_hash(state)?;
        Ok(state.clone())
    }

    pub(crate) fn commit_dialogue_world(
        &mut self,
        conversation_id: &str,
        world: crate::world_dialogue::DialogueWorldIR,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if !world.validate(state.completed_turns) {
            return Err(ConversationFrontendError::InvalidState);
        }
        state.dialogue_world = world;
        state.state_sha256 = state_hash(state)?;
        Ok(state.clone())
    }

    /// Commits already-grounded dialogue constraints through one bounded
    /// memory owner. An analyzer cannot edit conversation state directly.
    pub fn apply_dialogue_directives(
        &mut self,
        conversation_id: &str,
        turn_index: u64,
        candidates: &[DialogueDirectiveCandidateIR],
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::TurnOrder)?;
        if state.completed_turns != turn_index {
            return Err(ConversationFrontendError::TurnOrder);
        }
        state
            .dialogue_directive_ledger
            .apply_turn(turn_index, candidates)?;
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn analyze_discourse_group_update(
        &self,
        conversation_id: &str,
        text: &str,
        turn_index: u64,
    ) -> Option<DiscourseGroupUpdateIR> {
        self.states
            .get(conversation_id)
            .and_then(|state| analyze_discourse_group_update(state, text, turn_index))
    }

    pub fn analyze_topic_transition(
        &self,
        conversation_id: &str,
        text: &str,
        quoted_metalinguistic_request: bool,
    ) -> Option<TopicTransitionIR> {
        self.analyze_topic_transition_with_surface(
            conversation_id,
            text,
            text,
            quoted_metalinguistic_request,
        )
    }

    pub fn analyze_topic_transition_with_surface(
        &self,
        conversation_id: &str,
        semantic_text: &str,
        surface_source: &str,
        quoted_metalinguistic_request: bool,
    ) -> Option<TopicTransitionIR> {
        if quoted_metalinguistic_request {
            return None;
        }
        if let Some(kind) = group_topic_activation_kind(semantic_text) {
            let Some(state) = self.states.get(conversation_id) else {
                return Some(unresolved_topic_transition(
                    semantic_text,
                    vec!["DISCOURSE_GROUP_TARGET_UNRESOLVED".to_string()],
                ));
            };
            return Some(analyze_group_topic_activation(state, semantic_text, kind));
        }
        let mut detected = detect_topic_transition(semantic_text)?;
        if detected.kind == TopicTransitionKindIR::ActivateNamed {
            detected.surface = restore_topic_surface_case(&detected.surface, surface_source);
            detected.transition_sha256 = topic_transition_sha256(&detected);
            debug_assert!(detected.validate());
        }
        self.bind_topic_transition(conversation_id, &detected)
            .or_else(|| {
                Some(unresolved_topic_transition(
                    semantic_text,
                    vec![format!(
                        "TOPIC_HISTORY_OFFSET_UNRESOLVED:{}",
                        detected.history_offset.max(1)
                    )],
                ))
            })
    }

    pub fn apply_discourse_group_update(
        &mut self,
        conversation_id: &str,
        update: &DiscourseGroupUpdateIR,
        turn_index: u64,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        if !update.validate() || !update.applied {
            return Err(ConversationFrontendError::InvalidState);
        }
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if turn_index != state.completed_turns {
            return Err(ConversationFrontendError::InvalidState);
        }
        match update.operation {
            DiscourseGroupUpdateOperationIR::AddMember
            | DiscourseGroupUpdateOperationIR::RemoveMember => {
                let target_id = update
                    .target_group_id
                    .as_deref()
                    .ok_or(ConversationFrontendError::InvalidState)?;
                let index = state
                    .active_discourse_groups
                    .iter()
                    .position(|group| group.group_id == target_id)
                    .ok_or(ConversationFrontendError::InvalidState)?;
                if state.active_discourse_groups[index].member_keys != update.before_member_keys
                    || state.active_discourse_groups[index].revision + 1 != update.revision
                {
                    return Err(ConversationFrontendError::InvalidState);
                }
                let kind = state.active_discourse_groups[index].kind;
                let topics = discourse_group_topics(state, kind, &update.after_member_keys);
                let group = &mut state.active_discourse_groups[index];
                group.member_keys = update.after_member_keys.clone();
                group.topic_keys = topics;
                group.revision = update.revision;
                group.last_referenced_turn = turn_index;
                group.membership_sha256 = discourse_group_membership_sha256(group);
            }
            DiscourseGroupUpdateOperationIR::MergeGroups => {
                if state.active_discourse_groups.len() >= MAX_DISCOURSE_GROUPS {
                    return Err(ConversationFrontendError::InvalidState);
                }
                if state
                    .active_discourse_groups
                    .iter()
                    .any(|group| update.target_group_id.as_deref() == Some(group.group_id.as_str()))
                {
                    return Err(ConversationFrontendError::InvalidState);
                }
                let sources = update
                    .source_group_ids
                    .iter()
                    .filter_map(|group_id| {
                        state
                            .active_discourse_groups
                            .iter()
                            .find(|group| &group.group_id == group_id)
                    })
                    .collect::<Vec<_>>();
                if sources.len() != 2 || sources[0].kind != sources[1].kind {
                    return Err(ConversationFrontendError::InvalidState);
                }
                let expected_members = sources
                    .iter()
                    .flat_map(|group| group.member_keys.iter().cloned())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                if update.revision != 1 || expected_members != update.after_member_keys {
                    return Err(ConversationFrontendError::InvalidState);
                }
                let kind = sources[0].kind;
                let topics = discourse_group_topics(state, kind, &update.after_member_keys);
                let mut group = DiscourseGroupIR {
                    group_id: update
                        .target_group_id
                        .clone()
                        .ok_or(ConversationFrontendError::InvalidState)?,
                    kind,
                    member_keys: update.after_member_keys.clone(),
                    topic_keys: topics,
                    revision: 1,
                    component_group_ids: update.source_group_ids.clone(),
                    membership_sha256: String::new(),
                    introduced_turn: turn_index,
                    last_referenced_turn: turn_index,
                    semantic_authority: false,
                    external_execution_authorized: false,
                };
                group.membership_sha256 = discourse_group_membership_sha256(&group);
                state.active_discourse_groups.push(group);
            }
            DiscourseGroupUpdateOperationIR::Unresolved => {
                return Err(ConversationFrontendError::InvalidState)
            }
        }
        refresh_group_topic_anchors(state);
        synchronize_active_topic_context(state, turn_index);
        state.active_discourse_groups.sort_by(|left, right| {
            right
                .last_referenced_turn
                .cmp(&left.last_referenced_turn)
                .then_with(|| right.introduced_turn.cmp(&left.introduced_turn))
                .then_with(|| left.group_id.cmp(&right.group_id))
        });
        state.active_discourse_groups.truncate(MAX_DISCOURSE_GROUPS);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn resolve_pending_question(
        &self,
        conversation_id: &str,
        answer_text: &str,
    ) -> QuestionAnswerResolutionIR {
        self.resolve_pending_question_in_topic(conversation_id, answer_text, None)
    }

    pub fn resolve_pending_question_in_topic(
        &self,
        conversation_id: &str,
        answer_text: &str,
        topic_id: Option<&str>,
    ) -> QuestionAnswerResolutionIR {
        let Some(question) = self.states.get(conversation_id).and_then(|state| {
            topic_id.map_or_else(
                || state.pending_question.as_ref(),
                |topic_id| {
                    state
                        .topic_pending_questions
                        .iter()
                        .find(|question| question.topic_id.as_deref() == Some(topic_id))
                },
            )
        }) else {
            return QuestionAnswerResolutionIR {
                disposition: QuestionAnswerDispositionIR::NotApplicable,
                resolved_semantic_text: answer_text.to_string(),
                binding: None,
            };
        };
        resolve_question_answer(question, answer_text)
    }

    pub fn update_pending_question(
        &mut self,
        conversation_id: &str,
        pending_question: Option<QuestionUnderDiscussionIR>,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let topic_id = pending_question
            .as_ref()
            .and_then(|question| question.topic_id.clone())
            .or_else(|| {
                self.states
                    .get(conversation_id)
                    .and_then(|state| state.active_topics.first())
                    .map(|topic| topic.topic_id.clone())
            });
        self.update_pending_question_in_topic(
            conversation_id,
            pending_question,
            topic_id.as_deref(),
        )
    }

    pub fn update_pending_question_in_topic(
        &mut self,
        conversation_id: &str,
        mut pending_question: Option<QuestionUnderDiscussionIR>,
        topic_id: Option<&str>,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if let Some(topic_id) = topic_id {
            if !state
                .active_topics
                .iter()
                .any(|topic| topic.topic_id == topic_id)
            {
                return Err(ConversationFrontendError::InvalidState);
            }
            if let Some(question) = &mut pending_question {
                match question.topic_id.as_deref() {
                    Some(existing) if existing != topic_id => {
                        return Err(ConversationFrontendError::InvalidRequest)
                    }
                    Some(_) => {}
                    None => question.topic_id = Some(topic_id.to_string()),
                }
            }
            state
                .topic_pending_questions
                .retain(|question| question.topic_id.as_deref() != Some(topic_id));
            if let Some(question) = pending_question.as_ref() {
                state.topic_pending_questions.push(question.clone());
                state.topic_pending_questions.sort_by(|left, right| {
                    right
                        .source_turn
                        .cmp(&left.source_turn)
                        .then_with(|| left.question_id.cmp(&right.question_id))
                });
                state
                    .topic_pending_questions
                    .truncate(MAX_TOPIC_PENDING_QUESTIONS);
            }
        } else if pending_question
            .as_ref()
            .is_some_and(|question| question.topic_id.is_some())
        {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        state.pending_question = pending_question;
        if let Some((surface, concept_id_hint)) = state
            .pending_question
            .as_ref()
            .and_then(shared_question_focus)
        {
            state.discourse_focus.apply_turn(
                state.completed_turns,
                &[DiscourseFocusCandidateIR {
                    surface,
                    concept_id_hint: Some(concept_id_hint),
                    source: DiscourseFocusSourceIR::Proposition,
                    source_frame_id: None,
                    source_clause_id: None,
                    clause_function: None,
                    governing_relation: None,
                    salience_millis: 970,
                    source_order: 0,
                    evidence: vec![
                        "QUD_SHARED_REFERENT_CENTERING".to_string(),
                        "QUESTION_DOES_NOT_ESTABLISH_TRUTH:true".to_string(),
                    ],
                }],
            );
        }
        synchronize_active_topic_context(state, state.completed_turns);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn remember_discourse_program(
        &mut self,
        conversation_id: &str,
        program: &DiscourseProgramIR,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if program.introduced_turn != state.completed_turns
            || !program.validate(state.completed_turns)
        {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        state
            .active_discourse_programs
            .retain(|existing| existing.program_id != program.program_id);
        state.active_discourse_programs.push(program.clone());
        state.active_discourse_programs.sort_by(|left, right| {
            right
                .last_referenced_turn
                .cmp(&left.last_referenced_turn)
                .then_with(|| right.introduced_turn.cmp(&left.introduced_turn))
                .then_with(|| left.program_id.cmp(&right.program_id))
        });
        state
            .active_discourse_programs
            .truncate(MAX_ACTIVE_DISCOURSE_PROGRAMS);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn retire_active_goals(
        &mut self,
        conversation_id: &str,
        goal_ids: &[String],
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        let retired = goal_ids.iter().cloned().collect::<BTreeSet<_>>();
        state
            .action_state_ledger
            .withdraw(goal_ids, state.completed_turns);
        let retired_referents = retired
            .iter()
            .flat_map(|goal_id| {
                let suffix = goal_id.strip_prefix("GOAL-").unwrap_or(goal_id);
                [format!("DREF-E-{suffix}"), format!("DREF-R-{suffix}")]
            })
            .collect::<BTreeSet<_>>();
        state
            .active_goals
            .retain(|goal| !retired.contains(&goal.goal_id));
        state.active_discourse_programs.retain(|program| {
            !program
                .steps
                .iter()
                .any(|step| retired.contains(&step.goal.goal_id))
        });
        state
            .active_discourse_referents
            .retain(|referent| !retired_referents.contains(&referent.referent_id));
        state.active_subject = state.active_goals.last().map(|goal| goal.subject.clone());
        synchronize_active_topic_context(state, state.completed_turns);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn apply_action_state_analysis(
        &mut self,
        conversation_id: &str,
        analysis: &ActionStateAnalysisIR,
        turn_index: u64,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if state.completed_turns != turn_index
            || !analysis.unresolved_ambiguities.is_empty()
            || analysis.semantic_authority
            || analysis.external_action_executed
        {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        for report in analysis.language_reports() {
            if !state
                .action_state_ledger
                .apply_language_report(report, turn_index)
            {
                return Err(ConversationFrontendError::InvalidRequest);
            }
        }
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn apply_action_evidence(
        &mut self,
        request: &ActionEvidenceRequestIR,
    ) -> Result<(ActionEvidenceReceiptIR, ConversationStateIR), ConversationFrontendError> {
        let state = self
            .states
            .get_mut(&request.conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        let receipt = state
            .action_state_ledger
            .apply_evidence(request, state.completed_turns)
            .ok_or(ConversationFrontendError::InvalidRequest)?;
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok((receipt, state.clone()))
    }

    pub fn add_deferred_action_commitments(
        &mut self,
        conversation_id: &str,
        commitments: &[DeferredActionCommitmentIR],
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if commitments.iter().any(|commitment| {
            commitment.introduced_turn != state.completed_turns
                || !commitment.validate(state.completed_turns)
        }) {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        for commitment in commitments {
            if state
                .deferred_action_commitments
                .iter()
                .any(|existing| existing.commitment_id == commitment.commitment_id)
            {
                return Err(ConversationFrontendError::InvalidRequest);
            }
            state.deferred_action_commitments.push(commitment.clone());
        }
        if state.deferred_action_commitments.len() > MAX_DEFERRED_COMMITMENTS {
            let excess = state.deferred_action_commitments.len() - MAX_DEFERRED_COMMITMENTS;
            if state
                .deferred_action_commitments
                .iter()
                .take(excess)
                .any(DeferredActionCommitmentIR::is_pending)
            {
                return Err(ConversationFrontendError::InvalidState);
            }
            let removed_commitment_ids = state
                .deferred_action_commitments
                .iter()
                .take(excess)
                .map(|commitment| commitment.commitment_id.clone())
                .collect::<BTreeSet<_>>();
            state.deferred_action_commitments.drain(..excess);
            state.active_discourse_programs.retain(|program| {
                !program.steps.iter().any(|step| {
                    step.guard.as_ref().is_some_and(|guard| {
                        removed_commitment_ids.contains(&guard.deferred_commitment_id)
                    })
                })
            });
        }
        let focus_candidates = commitments
            .iter()
            .enumerate()
            .map(|(index, commitment)| DiscourseFocusCandidateIR {
                surface: commitment.action.subject.clone(),
                concept_id_hint: topic_alias(&commitment.action.subject)
                    .map(|(concept, _, _)| concept.to_string()),
                source: DiscourseFocusSourceIR::DeferredGoal,
                source_frame_id: None,
                source_clause_id: None,
                clause_function: None,
                governing_relation: None,
                salience_millis: 880,
                source_order: index,
                evidence: vec![
                    "DEFERRED_GOAL_CENTER".to_string(),
                    format!("COMMITMENT_ID:{}", commitment.commitment_id),
                ],
            })
            .collect::<Vec<_>>();
        state
            .discourse_focus
            .apply_turn(state.completed_turns, &focus_candidates);
        synchronize_active_topic_context(state, state.completed_turns);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn apply_condition_evidence(
        &mut self,
        request: &ConditionEvidenceRequestIR,
    ) -> Result<(ConditionEvidenceReceiptIR, ConversationStateIR), ConversationFrontendError> {
        if !request.validate() {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        let state = self
            .states
            .get_mut(&request.conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if state
            .deferred_action_commitments
            .iter()
            .flat_map(|commitment| commitment.evidence_ids.iter())
            .any(|evidence_id| evidence_id == &request.evidence_id)
        {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        let commitment_index = state
            .deferred_action_commitments
            .iter()
            .position(|commitment| commitment.commitment_id == request.commitment_id)
            .ok_or(ConversationFrontendError::InvalidRequest)?;
        let commitment = &state.deferred_action_commitments[commitment_index];
        if !commitment.is_pending() || commitment.condition_sha256 != request.condition_sha256 {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        let prior_status = commitment.status;
        let mut activated_goal = None;
        if request.disposition == ConditionEvidenceDispositionIR::VerifiedSatisfied {
            let action = commitment.action.clone();
            let goal_id = format!(
                "GOAL-D-{}",
                commitment
                    .commitment_id
                    .strip_prefix("DEFERRED-")
                    .unwrap_or(&commitment.commitment_id)
            );
            activated_goal = Some(ConversationGoalFrameIR {
                goal_id: goal_id.clone(),
                intent: action.intent,
                canonical_predicate: action.canonical_predicate,
                predicate_surface: action.predicate_surface,
                subject: action.subject,
                source_semantic_text: action.source_semantic_text,
                introduced_turn: state.completed_turns,
                last_referenced_turn: state.completed_turns,
                external_execution_authorized: true,
            });
            let commitment = &mut state.deferred_action_commitments[commitment_index];
            commitment.status = DeferredCommitmentStatusIR::Activated;
            commitment.last_transition_turn = state.completed_turns;
            commitment.evidence_ids.push(request.evidence_id.clone());
            commitment.activated_goal_id = Some(goal_id);
        } else {
            let commitment = &mut state.deferred_action_commitments[commitment_index];
            commitment.status = DeferredCommitmentStatusIR::Contradicted;
            commitment.last_transition_turn = state.completed_turns;
            commitment.evidence_ids.push(request.evidence_id.clone());
        }
        if let Some(goal) = activated_goal {
            state
                .action_state_ledger
                .add_plans(&[action_plan_seed(&goal)]);
            state
                .active_goals
                .retain(|existing| existing.goal_id != goal.goal_id);
            state.active_goals.push(goal.clone());
            state
                .active_goals
                .sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
            state.active_goals.truncate(MAX_ACTIVE_GOALS);
            let referent_index = state.active_discourse_referents.len() % MAX_ACTIVE_GOALS;
            let transition = topic_transition_from_surface(&goal.subject);
            let topic_id = discourse_topic_id_for_transition(&transition);
            state
                .active_discourse_referents
                .extend(event_and_result_referents(
                    &goal,
                    state.completed_turns,
                    referent_index,
                    Some(&topic_id),
                ));
            state.active_subject = Some(goal.subject.clone());
            activate_topic(state, &transition, state.completed_turns);
            state.discourse_focus.apply_turn(
                state.completed_turns,
                &[DiscourseFocusCandidateIR {
                    surface: goal.subject.clone(),
                    concept_id_hint: topic_alias(&goal.subject)
                        .map(|(concept, _, _)| concept.to_string()),
                    source: DiscourseFocusSourceIR::ActivatedGoal,
                    source_frame_id: None,
                    source_clause_id: None,
                    clause_function: None,
                    governing_relation: None,
                    salience_millis: 940,
                    source_order: 0,
                    evidence: vec!["VERIFIED_ACTIVATED_GOAL_CENTER".to_string()],
                }],
            );
            synchronize_active_topic_context(state, state.completed_turns);
        }
        state.active_discourse_referents.sort_by(|left, right| {
            right
                .last_referenced_turn
                .cmp(&left.last_referenced_turn)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.referent_id.cmp(&right.referent_id))
        });
        state
            .active_discourse_referents
            .truncate(MAX_DISCOURSE_REFERENTS);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        let commitment = &state.deferred_action_commitments[commitment_index];
        let receipt = ConditionEvidenceReceiptIR {
            schema: CONDITION_EVIDENCE_RECEIPT_SCHEMA.to_string(),
            evidence_id: request.evidence_id.clone(),
            conversation_id: request.conversation_id.clone(),
            commitment_id: request.commitment_id.clone(),
            accepted: true,
            prior_status,
            resulting_status: commitment.status,
            activated_goal_id: commitment.activated_goal_id.clone(),
            state_sha256: state.state_sha256.clone(),
            external_action_executed: false,
            unsupported_claims: 0,
        };
        debug_assert!(receipt.validate());
        Ok((receipt, state.clone()))
    }

    pub fn withdraw_deferred_action_commitments(
        &mut self,
        conversation_id: &str,
        commitment_ids: &[String],
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        let selected = commitment_ids.iter().collect::<BTreeSet<_>>();
        let mut changed = false;
        for commitment in &mut state.deferred_action_commitments {
            if commitment.is_pending() && selected.contains(&commitment.commitment_id) {
                commitment.status = DeferredCommitmentStatusIR::Withdrawn;
                commitment.last_transition_turn = state.completed_turns;
                changed = true;
            }
        }
        if !changed {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn apply_topic_transition(
        &mut self,
        conversation_id: &str,
        transition: &TopicTransitionIR,
        turn_index: u64,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        if !transition.validate() || !transition.applied {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if turn_index != state.completed_turns {
            return Err(ConversationFrontendError::InvalidState);
        }
        if let Some(group_id) = transition.anchor_group_id.as_deref() {
            let group = state
                .active_discourse_groups
                .iter_mut()
                .find(|group| group.group_id == group_id)
                .ok_or(ConversationFrontendError::InvalidState)?;
            if transition.anchor_group_revision != Some(group.revision)
                || transition.anchor_membership_sha256.as_deref()
                    != Some(group.membership_sha256.as_str())
            {
                return Err(ConversationFrontendError::InvalidState);
            }
            group.last_referenced_turn = turn_index;
        }
        activate_topic(state, transition, turn_index);
        let active_topic_id = state
            .active_topics
            .first()
            .map(|topic| topic.topic_id.as_str());
        state.pending_question = active_topic_id.and_then(|topic_id| {
            state
                .topic_pending_questions
                .iter()
                .find(|question| question.topic_id.as_deref() == Some(topic_id))
                .cloned()
        });
        synchronize_active_topic_context(state, turn_index);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn reassert_topic_anchor(
        &mut self,
        conversation_id: &str,
        reference: &TopicAnchoredReferenceIR,
        turn_index: u64,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        if !reference.validate() || !reference.applied {
            return Err(ConversationFrontendError::InvalidRequest);
        }
        let state = self
            .states
            .get_mut(conversation_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if turn_index != state.completed_turns {
            return Err(ConversationFrontendError::InvalidState);
        }
        let group = state
            .active_discourse_groups
            .iter_mut()
            .find(|group| group.group_id == reference.group_id)
            .ok_or(ConversationFrontendError::InvalidState)?;
        if group.revision != reference.group_revision
            || group.membership_sha256 != reference.membership_sha256
            || group.member_keys != reference.member_keys
        {
            return Err(ConversationFrontendError::InvalidState);
        }
        group.last_referenced_turn = turn_index;
        let topic_index = state
            .active_topics
            .iter()
            .position(|topic| {
                topic.topic_id == reference.topic_id
                    && topic.topic_sha256 == reference.topic_sha256
                    && topic.anchor_group_id.as_deref() == Some(reference.group_id.as_str())
            })
            .ok_or(ConversationFrontendError::InvalidState)?;
        if topic_index != 0 {
            synchronize_active_topic_context(state, turn_index);
            let outgoing_focus_id = state.discourse_focus.current_focus_id.clone();
            let topic = state.active_topics.remove(topic_index);
            state.active_topics.insert(0, topic);
            let active_topic = state.active_topics[0].clone();
            let live_topic_ids = state
                .active_topics
                .iter()
                .map(|topic| topic.topic_id.clone())
                .collect::<Vec<_>>();
            let restored_focus_id = state.topic_context_graph.activate(
                &active_topic.topic_id,
                &active_topic.topic_sha256,
                turn_index,
                outgoing_focus_id.as_deref(),
                &live_topic_ids,
                &["TOPIC_ANCHORED_REFERENCE_REASSERTION".to_string()],
            );
            if let Some(focus_id) = restored_focus_id {
                state.discourse_focus.restore_topic_focus(
                    turn_index,
                    &focus_id,
                    &["TOPIC_ANCHORED_CONTEXT_RESUME".to_string()],
                );
            }
        }
        synchronize_active_topic_context(state, turn_index);
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }

    pub fn bind_topic_transition(
        &self,
        conversation_id: &str,
        transition: &TopicTransitionIR,
    ) -> Option<TopicTransitionIR> {
        if !transition.validate() {
            return None;
        }
        if matches!(
            transition.kind,
            TopicTransitionKindIR::ActivateNamed | TopicTransitionKindIR::ActivateGroup
        ) {
            return Some(transition.clone());
        }
        if transition.kind == TopicTransitionKindIR::Unresolved {
            return None;
        }
        let state = self.states.get(conversation_id)?;
        let previous = state.active_topics.get(transition.history_offset.max(1))?;
        let group = previous.anchor_group_id.as_deref().and_then(|group_id| {
            state
                .active_discourse_groups
                .iter()
                .find(|group| group.group_id == group_id)
        });
        if previous.anchor_group_id.is_some() && group.is_none() {
            return None;
        }
        Some(seal_topic_transition(TopicTransitionIR {
            schema: TOPIC_TRANSITION_SCHEMA.to_string(),
            kind: TopicTransitionKindIR::ReturnPrevious,
            applied: true,
            history_offset: transition.history_offset,
            surface: previous.surface.clone(),
            concept_id_hint: previous.concept_id_hint.clone(),
            anchor_kind: previous.anchor_kind,
            anchor_group_id: group.map(|group| group.group_id.clone()),
            anchor_group_revision: group.map(|group| group.revision),
            anchor_membership_sha256: group.map(|group| group.membership_sha256.clone()),
            unresolved_terms: Vec::new(),
            evidence: vec![
                "DISCOURSE_MANAGEMENT:PREVIOUS_TOPIC_STACK".to_string(),
                format!("TOPIC_HISTORY_OFFSET:{}", transition.history_offset),
                format!("PREVIOUS_TOPIC_ID:{}", previous.topic_id),
                "GLOBAL_RECENCY_OVERRIDDEN:true".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                "SEMANTIC_PAYLOAD_MUTATED:false".to_string(),
            ],
            semantic_authority: false,
            external_action_executed: false,
            transition_sha256: String::new(),
        }))
    }

    pub fn validate_turn_order(
        &self,
        request: &ConversationTurnRequestIR,
    ) -> Result<(), ConversationFrontendError> {
        validate_turn_request(request)?;
        let expected_turn = self
            .states
            .get(&request.conversation_id)
            .map_or(1, |state| state.completed_turns.saturating_add(1));
        if request.turn_index != expected_turn {
            return Err(ConversationFrontendError::TurnOrder);
        }
        Ok(())
    }

    pub fn resolve_references(
        &self,
        conversation_id: &str,
        semantic_text: &str,
    ) -> ReferenceResolutionIR {
        let state = self.states.get(conversation_id);
        let mentions = scan_reference_mentions(semantic_text);
        let candidates = state
            .map(|state| reference_graph_candidates(state, semantic_text))
            .unwrap_or_default();
        let composition = state
            .filter(|_| should_compose_reference_mentions(&mentions))
            .map(|state| compose_reference_mentions(state, semantic_text, &mentions))
            .unwrap_or_else(|| ReferenceComposition::unchanged(semantic_text));
        let mut resolution =
            self.resolve_references_single_pass(conversation_id, &composition.resolved_text);
        resolution.original_semantic_text = semantic_text.to_string();
        if !composition.bindings.is_empty() {
            resolution.resolved_reference_count = resolution
                .resolved_reference_count
                .saturating_add(composition.bindings.len());
            let mut used = composition.used_referent_ids.clone();
            used.extend(resolution.used_referent_ids);
            used.sort();
            used.dedup();
            resolution.used_referent_ids = used;
            let mut bindings = composition.bindings.clone();
            bindings.extend(resolution.discourse_bindings);
            resolution.discourse_bindings = bindings;
        }
        let mut selection_hints = composition.selection_hints;
        selection_hints.extend(infer_selection_hints(
            &mentions,
            &resolution.discourse_bindings,
            &candidates,
            &selection_hints,
        ));
        resolution.resolution_graph = build_reference_resolution_graph(
            semantic_text,
            &resolution.resolved_semantic_text,
            &resolution.discourse_bindings,
            &candidates,
            &selection_hints,
        );
        debug_assert!(resolution.resolution_graph.validate());
        resolution
    }

    fn resolve_references_single_pass(
        &self,
        conversation_id: &str,
        semantic_text: &str,
    ) -> ReferenceResolutionIR {
        let state = self.states.get(conversation_id);
        if let Some(state) = state {
            if result_reference_requires_action_clarification(state, semantic_text) {
                return ReferenceResolutionIR {
                    original_semantic_text: semantic_text.to_string(),
                    resolved_semantic_text: semantic_text.to_string(),
                    resolved_reference_count: 0,
                    used_referent_ids: Vec::new(),
                    ambiguous_reference_surfaces: vec!["Result_REFERENCE".to_string()],
                    topic_anchored_resolution: None,
                    resolution_graph: ReferenceResolutionGraphIR::default(),
                    discourse_bindings: Vec::new(),
                };
            }
            if let Some(resolution) = resolve_topic_anchored_reference(state, semantic_text) {
                return resolution;
            }
            if let Some(resolution) = resolve_event_group_reference(state, semantic_text) {
                return resolution;
            }
            if let Some(resolution) = resolve_event_sequence_reference(state, semantic_text) {
                return resolution;
            }
        }
        if let Some(resolution) = resolve_same_turn_ordinal_reference(semantic_text) {
            return resolution;
        }
        if let Some(resolution) = resolve_same_turn_ordered_reference(semantic_text) {
            return resolution;
        }
        if let Some(resolution) = resolve_same_turn_actor_reference(semantic_text) {
            return resolution;
        }
        if let Some(resolution) = resolve_same_turn_result_reference(semantic_text) {
            return resolution;
        }
        if let Some(resolution) = resolve_local_conditional_reference(semantic_text) {
            return resolution;
        }
        if let Some(resolution) = resolve_same_turn_entity_reference(semantic_text) {
            return resolution;
        }
        let Some(state) = state else {
            let mut ambiguous = reference_surfaces(semantic_text);
            if let Some(reference) = event_group_reference(semantic_text) {
                ambiguous.push(format!("ACTION_GROUP_REFERENCE:{}", reference.marker));
            }
            if let Some(reference) = proposition_group_reference(semantic_text) {
                ambiguous.push(format!("PROPOSITION_GROUP_REFERENCE:{}", reference.marker));
            }
            ambiguous.extend(resolve_relation_antecedent(&[], 0, semantic_text).ambiguous_surfaces);
            ambiguous.extend(resolve_typed_coreference(&[], 0, semantic_text).ambiguous_surfaces);
            ambiguous.extend(
                resolve_ontology_entity_reference(&[], 0, semantic_text).ambiguous_surfaces,
            );
            ambiguous
                .extend(resolve_ontology_event_reference(&[], 0, semantic_text).ambiguous_surfaces);
            ambiguous.sort();
            ambiguous.dedup();
            if is_goal_ellipsis_surface(semantic_text) {
                ambiguous.push("ELLIPTICAL_GOAL".to_string());
            }
            if let Some(kind) = discourse_reference_kind(semantic_text) {
                ambiguous.push(format!("{kind:?}_REFERENCE"));
            }
            if let Some(kind) = unresolved_typed_deixis_kind(semantic_text) {
                ambiguous.push(typed_deixis_ambiguity_surface(kind).to_string());
            }
            ambiguous.sort();
            ambiguous.dedup();
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: semantic_text.to_string(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: ambiguous,
                topic_anchored_resolution: None,
                resolution_graph: ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            };
        };
        if let Some(surface) =
            independently_introduced_plural_person_ambiguity(state, semantic_text)
        {
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: semantic_text.to_string(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: vec![surface],
                topic_anchored_resolution: None,
                resolution_graph: ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            };
        }
        if result_reference_requires_action_clarification(state, semantic_text) {
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: semantic_text.to_string(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: vec!["Result_REFERENCE".to_string()],
                topic_anchored_resolution: None,
                resolution_graph: ReferenceResolutionGraphIR::default(),
                discourse_bindings: Vec::new(),
            };
        }
        if is_goal_ellipsis_surface(semantic_text) {
            let discourse = resolve_goal_ellipsis(state, semantic_text);
            let mut used_referent_ids = Vec::new();
            let mut discourse_bindings = Vec::new();
            if let Some(binding) = discourse.binding {
                used_referent_ids.extend(binding.referent_ids.iter().cloned());
                discourse_bindings.push(binding);
            }
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: discourse.resolved_text,
                resolved_reference_count: discourse_bindings.len(),
                used_referent_ids,
                ambiguous_reference_surfaces: discourse.ambiguous_surfaces,
                topic_anchored_resolution: None,
                resolution_graph: ReferenceResolutionGraphIR::default(),
                discourse_bindings,
            };
        }
        if let Some(resolution) = resolve_plural_proposition_reference(state, semantic_text) {
            return resolution;
        }
        let continuation_task_anaphor = is_continuation_task_anaphor(semantic_text);
        if topic_context_has_distinct_local_focus(state) && !continuation_task_anaphor {
            if let Some(resolution) = resolve_active_discourse_focus_reference(state, semantic_text)
            {
                return resolution;
            }
        }
        if let Some(resolution) = resolve_active_topic_reference(state, semantic_text) {
            return resolution;
        }
        if continuation_task_anaphor && state.active_subject.is_some() {
            let active_subject = state
                .active_subject
                .as_deref()
                .expect("continuation task reference requires an active subject");
            let inherited_goal_id =
                (state.active_goals.len() == 1).then(|| state.active_goals[0].goal_id.clone());
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: semantic_text.to_string(),
                resolved_reference_count: 1,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: Vec::new(),
                topic_anchored_resolution: None,
                resolution_graph: ReferenceResolutionGraphIR::default(),
                discourse_bindings: vec![DiscourseBindingIR {
                    kind: DiscourseBindingKindIR::EllipticalAction,
                    source_surface: semantic_text.to_string(),
                    resolved_surface: active_subject.to_string(),
                    referent_ids: Vec::new(),
                    inherited_goal_id,
                    confidence_millis: 930,
                    evidence: vec![
                        "TASK_ANAPHORA:ACTIVE_SUBJECT".to_string(),
                        "PREDICATE_CONTINUATION_TYPED:true".to_string(),
                        "SEMANTIC_AUTHORITY:false".to_string(),
                    ],
                }],
            };
        }
        if !continuation_task_anaphor {
            if let Some(kind) = unresolved_typed_deixis_kind(semantic_text) {
                if let Some(resolution) = resolve_active_typed_deixis(state, semantic_text) {
                    return resolution;
                }
                return ReferenceResolutionIR {
                    original_semantic_text: semantic_text.to_string(),
                    resolved_semantic_text: semantic_text.to_string(),
                    resolved_reference_count: 0,
                    used_referent_ids: Vec::new(),
                    ambiguous_reference_surfaces: vec![
                        typed_deixis_ambiguity_surface(kind).to_string()
                    ],
                    topic_anchored_resolution: None,
                    resolution_graph: ReferenceResolutionGraphIR::default(),
                    discourse_bindings: Vec::new(),
                };
            }
        }
        if discourse_focus_should_override_flat_recency(state) && !continuation_task_anaphor {
            if let Some(resolution) = resolve_active_discourse_focus_reference(state, semantic_text)
            {
                return resolution;
            }
        }
        let relation_antecedent = resolve_relation_antecedent(
            &state.active_discourse_referents,
            state.completed_turns,
            semantic_text,
        );
        let typed_entity = resolve_typed_coreference(
            &state.active_typed_entities,
            state.completed_turns,
            semantic_text,
        );
        let ontology_entity = resolve_ontology_entity_reference(
            &state.active_typed_entities,
            state.completed_turns,
            &typed_entity.resolved_text,
        );
        let ontology_event = resolve_ontology_event_reference(
            &state.active_discourse_referents,
            state.completed_turns,
            &ontology_entity.resolved_text,
        );
        let typed_discourse = if relation_antecedent.detected || continuation_task_anaphor {
            TypedDiscourseResolution {
                resolved_text: ontology_event.resolved_text.clone(),
                binding: None,
                ambiguous_surfaces: Vec::new(),
            }
        } else {
            resolve_typed_discourse_reference(state, &ontology_event.resolved_text)
        };
        let working_text = typed_discourse.resolved_text.clone();
        let eligible_referents = state
            .active_referents
            .iter()
            .filter(|referent| {
                state
                    .completed_turns
                    .saturating_sub(referent.last_referenced_turn)
                    <= MAX_REFERENCE_TURN_DISTANCE
            })
            .collect::<Vec<_>>();
        let latest_turn = eligible_referents
            .iter()
            .map(|referent| referent.last_referenced_turn)
            .max();
        let latest = eligible_referents
            .into_iter()
            .filter(|referent| Some(referent.last_referenced_turn) == latest_turn)
            .collect::<Vec<_>>();
        let mut resolved_count = 0;
        let mut used = BTreeSet::new();
        let mut ambiguous = typed_entity.ambiguous_surfaces.clone();
        let mut bindings = Vec::new();
        ambiguous.extend(relation_antecedent.ambiguous_surfaces.iter().cloned());
        if relation_antecedent.referent_ids.len() == 1 {
            resolved_count += 1;
            used.extend(relation_antecedent.referent_ids.iter().cloned());
            bindings.push(DiscourseBindingIR {
                kind: DiscourseBindingKindIR::DialogueRelationAntecedent,
                source_surface: relation_antecedent.connector_surface.clone(),
                resolved_surface: relation_antecedent.connector_surface.clone(),
                referent_ids: relation_antecedent.referent_ids.clone(),
                inherited_goal_id: None,
                confidence_millis: relation_antecedent.confidence_millis,
                evidence: vec![
                    "DIALOGUE_RELATION_PATH:LATEST_UNIQUE_PROPOSITION".to_string(),
                    "CAUSAL_TRUTH_ESTABLISHED:false".to_string(),
                    "SEMANTIC_AUTHORITY:false".to_string(),
                    "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                ],
            });
        }
        if let (Some(kind), Some(source_surface), Some(resolved_surface)) = (
            typed_entity.binding_kind,
            typed_entity.source_surface.clone(),
            typed_entity.resolved_surface.clone(),
        ) {
            resolved_count += 1;
            used.extend(typed_entity.entity_ids.iter().cloned());
            bindings.push(DiscourseBindingIR {
                kind: match kind {
                    TypedCoreferenceBindingKind::Entity => {
                        DiscourseBindingKindIR::TypedEntityReference
                    }
                    TypedCoreferenceBindingKind::BeliefHolder => {
                        DiscourseBindingKindIR::BeliefHolderReference
                    }
                },
                source_surface,
                resolved_surface,
                referent_ids: typed_entity.entity_ids.clone(),
                inherited_goal_id: None,
                confidence_millis: typed_entity.confidence_millis,
                evidence: vec![
                    "TYPE_CONSTRAINT:UNIQUE_COMPATIBLE_ENTITY".to_string(),
                    "SEMANTIC_AUTHORITY:false".to_string(),
                ],
            });
        }
        for ontology in [&ontology_entity, &ontology_event] {
            ambiguous.extend(ontology.ambiguous_surfaces.iter().cloned());
            if let (Some(kind), Some(source_surface), Some(resolved_surface)) = (
                ontology.binding_kind,
                ontology.source_surface.clone(),
                ontology.resolved_surface.clone(),
            ) {
                resolved_count += 1;
                used.extend(ontology.referent_ids.iter().cloned());
                bindings.push(DiscourseBindingIR {
                    kind: match kind {
                        OntologyBindingKind::Entity => {
                            DiscourseBindingKindIR::OntologyEntityReference
                        }
                        OntologyBindingKind::Event => {
                            DiscourseBindingKindIR::OntologyEventReference
                        }
                    },
                    source_surface,
                    resolved_surface,
                    referent_ids: ontology.referent_ids.clone(),
                    inherited_goal_id: None,
                    confidence_millis: ontology.confidence_millis,
                    evidence: ontology.evidence.clone(),
                });
            }
        }
        ambiguous.extend(typed_discourse.ambiguous_surfaces.iter().cloned());
        if let Some(binding) = typed_discourse.binding {
            resolved_count += 1;
            used.extend(binding.referent_ids.iter().cloned());
            bindings.push(binding);
        }
        let working_tokens = working_text.split_whitespace().collect::<Vec<_>>();
        let resolved = working_tokens
            .iter()
            .enumerate()
            .map(|(token_index, token)| {
                let (prefix, core, suffix) = token_parts(token);
                if english_that_is_complementizer(&working_tokens, token_index) {
                    return (*token).to_string();
                }
                if core.eq_ignore_ascii_case("that")
                    && relation_connector_contains_anaphoric_that(semantic_text)
                {
                    return (*token).to_string();
                }
                if continuation_task_anaphor
                    && core.eq_ignore_ascii_case("it")
                    && continuation_task_anaphor_at(&working_tokens, token_index)
                {
                    return (*token).to_string();
                }
                if english_local_pronoun(&working_tokens, token_index) {
                    return (*token).to_string();
                }
                if is_plural_reference_surface(core) {
                    if latest.len() < 2 {
                        ambiguous.push(core.to_string());
                        return token.to_string();
                    }
                    let surfaces = latest
                        .iter()
                        .map(|referent| localized_referent_surface(referent, &working_text))
                        .collect::<Vec<_>>();
                    let replacement = realize_plural_reference(core, &surfaces);
                    let referent_ids = latest
                        .iter()
                        .map(|referent| referent.referent_id.clone())
                        .collect::<Vec<_>>();
                    resolved_count += 1;
                    used.extend(referent_ids.iter().cloned());
                    bindings.push(DiscourseBindingIR {
                        kind: DiscourseBindingKindIR::PluralReference,
                        source_surface: core.to_string(),
                        resolved_surface: replacement.clone(),
                        referent_ids,
                        inherited_goal_id: None,
                        confidence_millis: 920,
                        evidence: vec!["RECENCY_PATH:LATEST_PLURAL_REFERENTS".to_string()],
                    });
                    return format!("{prefix}{replacement}{suffix}");
                }
                if let Some(referent) = ordered_referent(core, &latest) {
                    let surface = localized_referent_surface(referent, &working_text);
                    let replacement = realize_reference(core, &surface);
                    resolved_count += 1;
                    used.insert(referent.referent_id.clone());
                    bindings.push(DiscourseBindingIR {
                        kind: DiscourseBindingKindIR::OrderedReference,
                        source_surface: core.to_string(),
                        resolved_surface: replacement.clone(),
                        referent_ids: vec![referent.referent_id.clone()],
                        inherited_goal_id: None,
                        confidence_millis: 940,
                        evidence: vec!["ORDER_PATH:LATEST_ORDERED_REFERENT".to_string()],
                    });
                    return format!("{prefix}{replacement}{suffix}");
                }
                if !is_reference_surface(core) {
                    return token.to_string();
                }
                if ambiguous
                    .iter()
                    .any(|surface| ambiguity_blocks_surface(surface, core))
                {
                    return token.to_string();
                }
                if latest.len() != 1 {
                    ambiguous.push(core.to_string());
                    return token.to_string();
                }
                let referent = latest[0];
                let surface = localized_referent_surface(referent, &working_text);
                let replacement = realize_reference(core, &surface);
                resolved_count += 1;
                used.insert(referent.referent_id.clone());
                bindings.push(DiscourseBindingIR {
                    kind: DiscourseBindingKindIR::PronominalReference,
                    source_surface: core.to_string(),
                    resolved_surface: replacement.clone(),
                    referent_ids: vec![referent.referent_id.clone()],
                    inherited_goal_id: None,
                    confidence_millis: 900,
                    evidence: vec!["RECENCY_PATH:LATEST_UNIQUE_REFERENT".to_string()],
                });
                format!("{prefix}{replacement}{suffix}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let discourse = resolve_goal_ellipsis(state, &resolved);
        if let Some(binding) = discourse.binding {
            resolved_count += 1;
            used.extend(binding.referent_ids.iter().cloned());
            bindings.push(binding);
        }
        ambiguous.extend(discourse.ambiguous_surfaces);
        ReferenceResolutionIR {
            original_semantic_text: semantic_text.to_string(),
            resolved_semantic_text: discourse.resolved_text,
            resolved_reference_count: resolved_count,
            used_referent_ids: used.into_iter().collect(),
            ambiguous_reference_surfaces: ambiguous,
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: bindings,
        }
    }

    pub fn commit_turn(
        &mut self,
        request: &ConversationTurnRequestIR,
        semantic_subject: Option<&str>,
        used_referent_ids: &[String],
        unresolved_reference_count: usize,
        language: Option<LanguageCodeIR>,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        self.commit_turn_with_goals(
            request,
            semantic_subject,
            used_referent_ids,
            unresolved_reference_count,
            language,
            &[],
        )
    }

    pub fn commit_turn_with_goals(
        &mut self,
        request: &ConversationTurnRequestIR,
        semantic_subject: Option<&str>,
        used_referent_ids: &[String],
        unresolved_reference_count: usize,
        language: Option<LanguageCodeIR>,
        grounded_goals: &[ConversationGoalFrameIR],
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        self.commit_turn_with_discourse(
            request,
            ConversationCommitContext {
                semantic_subject,
                used_referent_ids,
                unresolved_reference_count,
                language,
                grounded_goals,
                proposition_referents: &[],
                temporal_analysis: None,
                guard_conditionals: None,
                semantic_role_graph: None,
                attribution_graph: None,
                discourse_focus_candidates: &[],
            },
        )
    }

    pub fn commit_turn_with_discourse(
        &mut self,
        request: &ConversationTurnRequestIR,
        context: ConversationCommitContext<'_>,
    ) -> Result<ConversationStateIR, ConversationFrontendError> {
        let ConversationCommitContext {
            semantic_subject,
            used_referent_ids,
            unresolved_reference_count,
            language,
            grounded_goals,
            proposition_referents,
            temporal_analysis,
            guard_conditionals,
            semantic_role_graph,
            attribution_graph,
            discourse_focus_candidates,
        } = context;
        self.validate_turn_order(request)?;
        let state = self
            .states
            .entry(request.conversation_id.clone())
            .or_insert_with(|| empty_state(&request.conversation_id));
        for referent in &mut state.active_referents {
            if used_referent_ids.contains(&referent.referent_id) {
                referent.last_referenced_turn = request.turn_index;
            }
        }
        for referent in &mut state.active_discourse_referents {
            if used_referent_ids.contains(&referent.referent_id) {
                referent.last_referenced_turn = request.turn_index;
            }
        }
        for goal in &mut state.active_goals {
            let referenced = goal
                .goal_id
                .strip_prefix("GOAL-")
                .is_some_and(|suffix| used_referent_ids.contains(&format!("DREF-E-{suffix}")));
            if referenced {
                goal.last_referenced_turn = request.turn_index;
            }
        }
        update_discourse_groups_from_references(state, used_referent_ids, request.turn_index);
        for referent in &mut state.active_typed_entities {
            if used_referent_ids.contains(&referent.entity_id) {
                referent.last_mentioned_turn = request.turn_index;
            }
        }
        let semantic_subject = semantic_subject.filter(|subject| !subject.trim().is_empty());
        if let Some(subject) = semantic_subject {
            state.active_subject = Some(subject.to_string());
        }
        let mut referent_context = semantic_subject.unwrap_or_default().to_string();
        for goal in grounded_goals {
            if !referent_context.is_empty() {
                referent_context.push(' ');
            }
            referent_context.push_str(&goal.source_semantic_text);
        }
        if !referent_context.is_empty() {
            let extracted = extract_referents(&referent_context, request.turn_index);
            for referent in extracted {
                state
                    .active_referents
                    .retain(|existing| existing.canonical_concept != referent.canonical_concept);
                state.active_referents.push(referent);
            }
            state.active_referents.sort_by(|left, right| {
                right
                    .last_referenced_turn
                    .cmp(&left.last_referenced_turn)
                    .then_with(|| left.referent_id.cmp(&right.referent_id))
            });
            state.active_referents.truncate(MAX_ACTIVE_REFERENTS);
        }
        if !grounded_goals.is_empty() {
            let scoped_explicit_topic_id = state
                .active_topics
                .first()
                .filter(|topic| topic.explicitly_activated)
                .map(|topic| topic.topic_id.clone());
            let action_seeds = grounded_goals
                .iter()
                .map(action_plan_seed)
                .collect::<Vec<_>>();
            state
                .action_state_ledger
                .replace_active_plans(&action_seeds, request.turn_index);
            state.active_goals = grounded_goals.to_vec();
            state
                .active_goals
                .sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
            remember_action_group(state, grounded_goals, request.turn_index);
            for (index, goal) in grounded_goals.iter().enumerate() {
                let transition = topic_transition_from_surface(&goal.subject);
                let topic_id = scoped_explicit_topic_id
                    .clone()
                    .unwrap_or_else(|| discourse_topic_id_for_transition(&transition));
                state
                    .active_discourse_referents
                    .extend(event_and_result_referents(
                        goal,
                        request.turn_index,
                        index,
                        Some(&topic_id),
                    ));
                if scoped_explicit_topic_id.is_none() {
                    activate_topic(state, &transition, request.turn_index);
                }
            }
        }
        state
            .discourse_focus
            .apply_turn(request.turn_index, discourse_focus_candidates);
        synchronize_active_topic_context(state, request.turn_index);
        let mut current_propositions = proposition_referents.to_vec();
        if let Some(topic_id) = state
            .active_topics
            .first()
            .filter(|topic| topic.explicitly_activated)
            .map(|topic| topic.topic_id.clone())
        {
            for proposition in &mut current_propositions {
                if proposition.topic_id.is_none() {
                    proposition.topic_id = Some(topic_id.clone());
                }
            }
        }
        let observations = current_propositions
            .iter()
            .map(|referent| EpistemicObservationIR {
                origin_referent_id: referent.referent_id.clone(),
                source_actor: referent
                    .attributed_source
                    .clone()
                    .unwrap_or_else(|| "DIALOGUE_USER".to_string()),
                proposition_surface: referent.semantic_summary.clone(),
                proposition_polarity: referent
                    .proposition_polarity
                    .unwrap_or(AttributedPropositionPolarityIR::Positive),
                modal_world: referent.modal_world.unwrap_or(ModalWorldIR::Actual),
                attribution_attitude: referent
                    .attribution_attitude
                    .unwrap_or(AttributionAttitudeIR::Say),
                epistemic_status: referent
                    .epistemic_status
                    .unwrap_or(EpistemicStatusIR::Reported),
            })
            .collect::<Vec<_>>();
        let belief_bindings = state.epistemic_ledger.apply_turn(
            request.turn_index,
            &request.raw_text,
            used_referent_ids,
            &observations,
        );
        for referent in &mut current_propositions {
            referent.belief_record_id = belief_bindings
                .iter()
                .find(|(referent_id, _)| referent_id == &referent.referent_id)
                .map(|(_, belief_id)| belief_id.clone());
        }
        if let Some(analysis) = temporal_analysis {
            state.temporal_graph.apply_turn(analysis);
        }
        if let Some(conditionals) = guard_conditionals {
            state.last_guard_evaluations = state.conditional_guard_store.apply_turn(
                request.turn_index,
                conditionals,
                &state.epistemic_ledger,
                language.unwrap_or(LanguageCodeIR::English),
            );
        }
        state.dialogue_relation_graph.apply_turn(
            request.turn_index,
            &request.raw_text,
            used_referent_ids,
            &state.active_discourse_referents,
            &current_propositions,
        );
        merge_typed_mentions(
            &mut state.active_typed_entities,
            request.turn_index,
            semantic_role_graph,
            attribution_graph,
        );
        merge_ontology_mentions(
            &mut state.active_typed_entities,
            request.turn_index,
            &request.raw_text,
        );
        state
            .active_discourse_referents
            .extend(current_propositions);
        state.active_discourse_referents.retain(|referent| {
            referent.kind != DiscourseReferentKindIR::Proposition
                || referent
                    .belief_record_id
                    .as_deref()
                    .is_some_and(|belief_id| {
                        state
                            .epistemic_ledger
                            .record(belief_id)
                            .is_some_and(|record| record.status.is_reference_active())
                    })
        });
        state.active_discourse_referents.sort_by(|left, right| {
            right
                .last_referenced_turn
                .cmp(&left.last_referenced_turn)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.referent_id.cmp(&right.referent_id))
        });
        state
            .active_discourse_referents
            .truncate(MAX_DISCOURSE_REFERENTS);
        synchronize_active_topic_context(state, request.turn_index);
        state
            .dialogue_relation_graph
            .synchronize_with_ledger(request.turn_index, &state.epistemic_ledger);
        state.completed_turns = request.turn_index;
        state.preferred_language = language.or(state.preferred_language);
        state.unresolved_reference_count = unresolved_reference_count;
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }
}

#[derive(Debug)]
struct ReferenceComposition {
    resolved_text: String,
    bindings: Vec<DiscourseBindingIR>,
    used_referent_ids: Vec<String>,
    selection_hints: Vec<ReferenceSelectionHint>,
}

impl ReferenceComposition {
    fn unchanged(text: &str) -> Self {
        Self {
            resolved_text: text.to_string(),
            bindings: Vec::new(),
            used_referent_ids: Vec::new(),
            selection_hints: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct CompositionalAntecedent {
    id: String,
    surface: String,
    source: String,
    confidence_millis: u16,
}

#[derive(Debug)]
struct CompositionalReplacement {
    start: usize,
    end: usize,
    replacement: String,
    binding: DiscourseBindingIR,
    selection_hint: ReferenceSelectionHint,
    used_referent_id: Option<String>,
}

fn should_compose_reference_mentions(mentions: &[ReferenceMentionNodeIR]) -> bool {
    mentions.len() >= 2 && mentions.iter().any(|mention| !mention.quote_inert)
}

fn reference_graph_candidates(
    state: &ConversationStateIR,
    text: &str,
) -> Vec<ReferenceAntecedentCandidateIR> {
    let mut candidates = Vec::new();
    if let Some(focus) = current_focus_antecedent(state, text) {
        candidates.push(ReferenceAntecedentCandidateIR {
            antecedent_id: focus.id,
            antecedent_surface: focus.surface,
            semantic_type: "ENTITY".to_string(),
            source: focus.source,
            salience_millis: focus.confidence_millis,
        });
    }
    candidates.extend(
        state
            .active_referents
            .iter()
            .filter(|referent| {
                state
                    .completed_turns
                    .saturating_sub(referent.last_referenced_turn)
                    <= MAX_REFERENCE_TURN_DISTANCE
            })
            .map(|referent| ReferenceAntecedentCandidateIR {
                antecedent_id: referent.referent_id.clone(),
                antecedent_surface: localized_referent_surface(referent, text),
                semantic_type: "ENTITY".to_string(),
                source: "BOUNDED_DYNAMIC_REFERENT".to_string(),
                salience_millis: 900,
            }),
    );
    candidates.extend(
        state
            .active_goals
            .iter()
            .map(|goal| ReferenceAntecedentCandidateIR {
                antecedent_id: goal.goal_id.clone(),
                antecedent_surface: goal.subject.clone(),
                semantic_type: "ENTITY".to_string(),
                source: "ACTIVE_GOAL_SEQUENCE".to_string(),
                salience_millis: 940,
            }),
    );
    candidates.extend(
        state
            .active_typed_entities
            .iter()
            .filter(|referent| {
                state
                    .completed_turns
                    .saturating_sub(referent.last_mentioned_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
            })
            .map(|referent| ReferenceAntecedentCandidateIR {
                antecedent_id: referent.entity_id.clone(),
                antecedent_surface: referent.canonical_surface.clone(),
                semantic_type: format!("{:?}", referent.kind).to_uppercase(),
                source: "TYPED_ENTITY_MEMORY".to_string(),
                salience_millis: 910,
            }),
    );
    candidates
}

fn compose_reference_mentions(
    state: &ConversationStateIR,
    text: &str,
    mentions: &[ReferenceMentionNodeIR],
) -> ReferenceComposition {
    let focus = current_focus_antecedent(state, text);
    let person = unique_person_antecedent(state);
    let ordered = latest_ordered_referents(state, text);
    let mut local_anchor: Option<CompositionalAntecedent> = None;
    let mut replacements = Vec::new();
    for mention in mentions.iter().filter(|mention| !mention.quote_inert) {
        let antecedent = match mention.kind {
            ReferenceMentionKindIR::Ordered => {
                let selected = ordered_antecedent(mention, &ordered);
                if let Some(selected) = &selected {
                    local_anchor = Some(selected.clone());
                }
                selected
            }
            ReferenceMentionKindIR::PersonPronoun => person.clone(),
            ReferenceMentionKindIR::Possessive => local_anchor.clone().or_else(|| focus.clone()),
            ReferenceMentionKindIR::Demonstrative
            | ReferenceMentionKindIR::GenericPronoun
            | ReferenceMentionKindIR::ZeroArgumentEllipsis => focus.clone(),
        };
        let Some(antecedent) = antecedent else {
            continue;
        };
        let replacement = realize_compositional_reference(text, mention, &antecedent.surface);
        let kind = match mention.kind {
            ReferenceMentionKindIR::Possessive => DiscourseBindingKindIR::PossessiveFocusReference,
            ReferenceMentionKindIR::Demonstrative => {
                DiscourseBindingKindIR::DemonstrativeFocusReference
            }
            ReferenceMentionKindIR::Ordered => DiscourseBindingKindIR::OrderedReference,
            ReferenceMentionKindIR::PersonPronoun => DiscourseBindingKindIR::TypedEntityReference,
            ReferenceMentionKindIR::GenericPronoun => DiscourseBindingKindIR::PronominalReference,
            ReferenceMentionKindIR::ZeroArgumentEllipsis => {
                DiscourseBindingKindIR::ZeroArgumentEllipsis
            }
        };
        let used_referent_id = (antecedent.id.starts_with("REF-")
            || antecedent.id.starts_with("DREF-")
            || antecedent.id.starts_with("TREF-"))
        .then(|| antecedent.id.clone());
        replacements.push(CompositionalReplacement {
            start: mention.byte_start,
            end: mention.byte_end,
            replacement: replacement.clone(),
            binding: DiscourseBindingIR {
                kind,
                source_surface: mention.source_surface.clone(),
                resolved_surface: replacement,
                referent_ids: used_referent_id.iter().cloned().collect(),
                inherited_goal_id: None,
                confidence_millis: antecedent.confidence_millis,
                evidence: vec![
                    format!("REFERENCE_COMPOSITION_SOURCE:{}", antecedent.source),
                    format!("REFERENCE_MENTION_ID:{}", mention.mention_id),
                    "MULTI_MENTION_COMPETITION:true".to_string(),
                    "SEMANTIC_AUTHORITY:false".to_string(),
                    "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                ],
            },
            selection_hint: ReferenceSelectionHint {
                mention_byte_start: mention.byte_start,
                antecedent_id: antecedent.id,
                antecedent_surface: antecedent.surface,
            },
            used_referent_id,
        });
    }
    let mut resolved = text.to_string();
    for replacement in replacements.iter().rev() {
        resolved.replace_range(replacement.start..replacement.end, &replacement.replacement);
    }
    ReferenceComposition {
        resolved_text: resolved,
        bindings: replacements
            .iter()
            .map(|replacement| replacement.binding.clone())
            .collect(),
        used_referent_ids: replacements
            .iter()
            .filter_map(|replacement| replacement.used_referent_id.clone())
            .collect(),
        selection_hints: replacements
            .into_iter()
            .map(|replacement| replacement.selection_hint)
            .collect(),
    }
}

fn current_focus_antecedent(
    state: &ConversationStateIR,
    text: &str,
) -> Option<CompositionalAntecedent> {
    let focus = state.discourse_focus.current().filter(|focus| {
        state
            .completed_turns
            .saturating_sub(focus.last_focused_turn)
            <= MAX_DISCOURSE_FOCUS_TURN_DISTANCE
    })?;
    let english = text_is_english(text);
    let surface = focus
        .concept_id_hint
        .as_deref()
        .and_then(|concept| topic_surface(concept, english))
        .unwrap_or(focus.surface.as_str());
    Some(CompositionalAntecedent {
        id: format!("FOCUS-{}", focus.focus_id),
        surface: surface.to_string(),
        source: "ACTIVE_DISCOURSE_FOCUS".to_string(),
        confidence_millis: focus.salience_millis.min(950),
    })
}

fn unique_person_antecedent(state: &ConversationStateIR) -> Option<CompositionalAntecedent> {
    let people = state
        .active_typed_entities
        .iter()
        .filter(|referent| {
            referent.kind == TypedEntityKindIR::Person
                && state
                    .completed_turns
                    .saturating_sub(referent.last_mentioned_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    let [person] = people.as_slice() else {
        return None;
    };
    Some(CompositionalAntecedent {
        id: person.entity_id.clone(),
        surface: person.canonical_surface.clone(),
        source: "UNIQUE_TYPED_PERSON".to_string(),
        confidence_millis: 910,
    })
}

fn independently_introduced_plural_person_ambiguity(
    state: &ConversationStateIR,
    text: &str,
) -> Option<String> {
    let surface = unquoted_marker_surface(text, "they")?;
    let people = state
        .active_typed_entities
        .iter()
        .filter(|referent| {
            referent.kind == TypedEntityKindIR::Person
                && state
                    .completed_turns
                    .saturating_sub(referent.last_mentioned_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    if people.len() < 2 {
        return None;
    }
    let introduction_turns = people
        .iter()
        .map(|person| person.introduced_turn)
        .collect::<BTreeSet<_>>();
    (introduction_turns.len() > 1).then_some(surface)
}

fn result_reference_requires_action_clarification(state: &ConversationStateIR, text: &str) -> bool {
    if discourse_reference_kind(text) != Some(DiscourseReferentKindIR::Result) {
        return false;
    }
    let lower = text.to_lowercase();
    if ["results", "outputs", "outcomes", "결과들", "출력들"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return false;
    }
    let result_referents = state
        .active_discourse_referents
        .iter()
        .filter(|referent| {
            referent.kind == DiscourseReferentKindIR::Result
                && state
                    .completed_turns
                    .saturating_sub(referent.last_referenced_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    let latest_reference_turn = result_referents
        .iter()
        .map(|referent| referent.last_referenced_turn)
        .max();
    let equally_recent_results = result_referents
        .iter()
        .filter(|referent| Some(referent.last_referenced_turn) == latest_reference_turn)
        .count();
    if equally_recent_results < 2
        || result_referents
            .iter()
            .any(|referent| reference_mentions_source(text, &referent.semantic_summary))
    {
        return false;
    }
    !state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)
        .is_some_and(|topic| {
            result_referents
                .iter()
                .any(|referent| referent.topic_id.as_deref() == Some(topic.topic_id.as_str()))
        })
}

fn latest_ordered_referents(
    state: &ConversationStateIR,
    text: &str,
) -> Vec<CompositionalAntecedent> {
    let latest_goal_turn = state
        .active_goals
        .iter()
        .map(|goal| goal.introduced_turn)
        .max();
    let mut goals = state
        .active_goals
        .iter()
        .filter(|goal| Some(goal.introduced_turn) == latest_goal_turn)
        .map(|goal| CompositionalAntecedent {
            id: goal.goal_id.clone(),
            surface: goal.subject.clone(),
            source: "ORDERED_ACTIVE_GOAL".to_string(),
            confidence_millis: 950,
        })
        .collect::<Vec<_>>();
    goals.sort_by(|left, right| left.id.cmp(&right.id));
    if goals.len() >= 2 {
        return goals;
    }
    let eligible = state
        .active_referents
        .iter()
        .filter(|referent| {
            state
                .completed_turns
                .saturating_sub(referent.last_referenced_turn)
                <= MAX_REFERENCE_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    let latest_turn = eligible
        .iter()
        .map(|referent| referent.last_referenced_turn)
        .max();
    let mut latest = eligible
        .into_iter()
        .filter(|referent| Some(referent.last_referenced_turn) == latest_turn)
        .map(|referent| CompositionalAntecedent {
            id: referent.referent_id.clone(),
            surface: localized_referent_surface(referent, text),
            source: "ORDERED_DYNAMIC_REFERENT".to_string(),
            confidence_millis: 940,
        })
        .collect::<Vec<_>>();
    latest.sort_by(|left, right| left.id.cmp(&right.id));
    latest
}

fn ordered_antecedent(
    mention: &ReferenceMentionNodeIR,
    ordered: &[CompositionalAntecedent],
) -> Option<CompositionalAntecedent> {
    if ordered.len() < 2 {
        return None;
    }
    match mention.normalized_surface.as_str() {
        "former" | "전자" | "전자를" => ordered.first().cloned(),
        "latter" | "후자" | "후자를" => ordered.last().cloned(),
        _ => None,
    }
}

fn realize_compositional_reference(
    text: &str,
    mention: &ReferenceMentionNodeIR,
    surface: &str,
) -> String {
    match mention.kind {
        ReferenceMentionKindIR::Possessive => {
            if mention.source_surface.is_ascii() {
                format!("{surface}'s")
            } else {
                format!("{surface}의")
            }
        }
        ReferenceMentionKindIR::Demonstrative => {
            if mention.source_surface.ends_with('을') || mention.source_surface.ends_with('를') {
                format!("{surface}{}", object_particle(surface))
            } else if mention.source_surface.ends_with('이')
                || mention.source_surface.ends_with('가')
            {
                format!("{surface}{}", subject_particle(surface))
            } else {
                surface.to_string()
            }
        }
        ReferenceMentionKindIR::Ordered => realize_reference(&mention.source_surface, surface),
        ReferenceMentionKindIR::PersonPronoun => {
            if english_person_pronoun_is_determiner(text, mention) {
                format!("{surface}'s")
            } else {
                surface.to_string()
            }
        }
        ReferenceMentionKindIR::GenericPronoun | ReferenceMentionKindIR::ZeroArgumentEllipsis => {
            surface.to_string()
        }
    }
}

fn english_person_pronoun_is_determiner(text: &str, mention: &ReferenceMentionNodeIR) -> bool {
    if mention.normalized_surface == "his" {
        return true;
    }
    let next = text[mention.byte_end..]
        .split_whitespace()
        .next()
        .map(|token| {
            token
                .trim_matches(|character: char| !character.is_ascii_alphanumeric())
                .to_lowercase()
        });
    next.is_some_and(|word| {
        matches!(
            word.as_str(),
            "report"
                | "claim"
                | "belief"
                | "statement"
                | "status"
                | "configuration"
                | "result"
                | "output"
        )
    })
}

fn infer_selection_hints(
    mentions: &[ReferenceMentionNodeIR],
    bindings: &[DiscourseBindingIR],
    candidates: &[ReferenceAntecedentCandidateIR],
    existing: &[ReferenceSelectionHint],
) -> Vec<ReferenceSelectionHint> {
    let mut used_mentions = existing
        .iter()
        .map(|hint| hint.mention_byte_start)
        .collect::<BTreeSet<_>>();
    let mut inferred = Vec::new();
    for binding in bindings {
        let source = binding.source_surface.to_lowercase();
        let mention = mentions.iter().find(|mention| {
            !mention.quote_inert
                && !used_mentions.contains(&mention.byte_start)
                && (mention.normalized_surface == source
                    || mention.normalized_surface.contains(&source)
                    || source.contains(&mention.normalized_surface))
        });
        let Some(mention) = mention else {
            continue;
        };
        let resolved = binding.resolved_surface.to_lowercase();
        let candidate = candidates.iter().find(|candidate| {
            let surface = candidate.antecedent_surface.to_lowercase();
            resolved.contains(&surface) || surface.contains(resolved.trim_matches(['\'', 's']))
        });
        let Some(candidate) = candidate else {
            continue;
        };
        used_mentions.insert(mention.byte_start);
        inferred.push(ReferenceSelectionHint {
            mention_byte_start: mention.byte_start,
            antecedent_id: candidate.antecedent_id.clone(),
            antecedent_surface: candidate.antecedent_surface.clone(),
        });
    }
    inferred
}

#[derive(Debug)]
struct LocalOrdinalOccurrence {
    start_index: usize,
    end_index: usize,
    slot: usize,
    source_surface: String,
}

fn resolve_same_turn_ordinal_reference(text: &str) -> Option<ReferenceResolutionIR> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let occurrences = local_ordinal_occurrences(&tokens);
    let first_ordinal_index = occurrences
        .iter()
        .map(|occurrence| occurrence.start_index)
        .min()?;
    let antecedents =
        local_ordered_antecedents(&tokens[..first_ordinal_index], text_is_english(text));
    if antecedents.len() < 2 {
        return None;
    }
    let required = occurrences
        .iter()
        .map(|occurrence| occurrence.slot + 1)
        .max()
        .unwrap_or_default();
    if antecedents.len() < required {
        return Some(ReferenceResolutionIR {
            original_semantic_text: text.to_string(),
            resolved_semantic_text: text.to_string(),
            resolved_reference_count: 0,
            used_referent_ids: Vec::new(),
            ambiguous_reference_surfaces: vec!["LOCAL_ORDINAL_ANTECEDENT_SET".to_string()],
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: Vec::new(),
        });
    }

    let mut resolved_tokens = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    let mut bindings = Vec::new();
    for occurrence in occurrences {
        let antecedent = &antecedents[occurrence.slot];
        let (prefix, core, suffix) = token_parts(tokens[occurrence.end_index]);
        let replacement = realize_local_ordinal_reference(core, antecedent);
        for token in resolved_tokens
            .iter_mut()
            .take(occurrence.end_index)
            .skip(occurrence.start_index)
        {
            token.clear();
        }
        resolved_tokens[occurrence.end_index] = format!("{prefix}{replacement}{suffix}");
        bindings.push(DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalOrdinalReference,
            source_surface: occurrence.source_surface,
            resolved_surface: replacement,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 955,
            evidence: vec![
                "SYNTACTIC_PRIORITY:LOCAL_ORDINAL_ANTECEDENTS".to_string(),
                format!("LOCAL_ANTECEDENT_POSITION:{}", occurrence.slot + 1),
                "GLOBAL_RECENCY_OVERRIDDEN:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
            ],
        });
    }
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_tokens
            .into_iter()
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        resolved_reference_count: bindings.len(),
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: bindings,
    })
}

fn local_ordinal_occurrences(tokens: &[&str]) -> Vec<LocalOrdinalOccurrence> {
    let mut occurrences = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let core = token_parts(tokens[index]).1.to_lowercase();
        if matches!(core.as_str(), "첫" | "두" | "세")
            && tokens
                .get(index + 1)
                .is_some_and(|next| token_parts(next).1.to_lowercase().starts_with("번째"))
        {
            let slot = match core.as_str() {
                "첫" => 0,
                "두" => 1,
                _ => 2,
            };
            occurrences.push(LocalOrdinalOccurrence {
                start_index: index,
                end_index: index + 1,
                slot,
                source_surface: format!("{} {}", tokens[index], tokens[index + 1]),
            });
            index += 2;
            continue;
        }
        if let Some(slot) = local_ordinal_slot(&core) {
            let english = matches!(core.as_str(), "first" | "second" | "third");
            let has_determiner = index.checked_sub(1).is_some_and(|previous| {
                token_parts(tokens[previous]).1.eq_ignore_ascii_case("the")
            });
            if !english || has_determiner {
                occurrences.push(LocalOrdinalOccurrence {
                    start_index: index,
                    end_index: index,
                    slot,
                    source_surface: token_parts(tokens[index]).1.to_string(),
                });
            }
        }
        index += 1;
    }
    occurrences
}

fn local_ordinal_slot(surface: &str) -> Option<usize> {
    let compact = surface.replace([' ', '-', '_'], "");
    [
        (
            0,
            [
                "첫째",
                "첫째를",
                "첫째는",
                "첫째가",
                "첫번째",
                "첫번째를",
                "첫번째는",
                "첫번째가",
                "first",
            ]
            .as_slice(),
        ),
        (
            1,
            [
                "둘째",
                "둘째를",
                "둘째는",
                "둘째가",
                "두번째",
                "두번째를",
                "두번째는",
                "두번째가",
                "second",
            ]
            .as_slice(),
        ),
        (
            2,
            [
                "셋째",
                "셋째를",
                "셋째는",
                "셋째가",
                "세번째",
                "세번째를",
                "세번째는",
                "세번째가",
                "third",
            ]
            .as_slice(),
        ),
    ]
    .into_iter()
    .find_map(|(slot, surfaces)| surfaces.contains(&compact.as_str()).then_some(slot))
}

fn realize_local_ordinal_reference(reference: &str, surface: &str) -> String {
    if reference.ends_with('를') || reference.ends_with('을') {
        format!("{surface}{}", object_particle(surface))
    } else if reference.ends_with('는') || reference.ends_with('은') {
        format!("{surface}{}", topic_particle(surface))
    } else if reference.ends_with('가') || reference.ends_with('이') {
        format!("{surface}{}", subject_particle(surface))
    } else {
        surface.to_string()
    }
}

fn resolve_same_turn_ordered_reference(text: &str) -> Option<ReferenceResolutionIR> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let ordered_tokens = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let core = token_parts(token).1;
            local_order_slot(core).map(|slot| (index, core.to_string(), slot))
        })
        .collect::<Vec<_>>();
    let first_ordered_index = ordered_tokens.first().map(|(index, _, _)| *index)?;
    let english = text_is_english(text);
    let broad_antecedents = local_ordered_antecedents(&tokens[..first_ordered_index], english);
    let scoped_antecedents = if english {
        tokens[..first_ordered_index]
            .iter()
            .position(|token| {
                matches!(
                    token_parts(token).1.to_lowercase().as_str(),
                    "analyze"
                        | "analyse"
                        | "inspect"
                        | "check"
                        | "review"
                        | "repair"
                        | "fix"
                        | "explain"
                        | "summarize"
                )
            })
            .map(|action_index| {
                local_ordered_antecedents(&tokens[action_index + 1..first_ordered_index], true)
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let antecedents = if scoped_antecedents.len() >= 2 {
        scoped_antecedents
    } else {
        broad_antecedents
    };
    if antecedents.len() < 2 {
        return None;
    }
    if antecedents.len() != 2 {
        return Some(ReferenceResolutionIR {
            original_semantic_text: text.to_string(),
            resolved_semantic_text: text.to_string(),
            resolved_reference_count: 0,
            used_referent_ids: Vec::new(),
            ambiguous_reference_surfaces: vec!["LOCAL_ORDERED_ANTECEDENT_SET".to_string()],
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: Vec::new(),
        });
    }

    let mut resolved_tokens = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    let mut bindings = Vec::new();
    for (index, source_surface, slot) in ordered_tokens {
        let antecedent = &antecedents[slot];
        let (prefix, core, suffix) = token_parts(tokens[index]);
        let replacement = realize_local_order_reference(core, antecedent);
        resolved_tokens[index] = format!("{prefix}{replacement}{suffix}");
        bindings.push(DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalOrderedReference,
            source_surface,
            resolved_surface: replacement,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 960,
            evidence: vec![
                "SYNTACTIC_PRIORITY:LOCAL_ORDERED_ANTECEDENTS".to_string(),
                format!("LOCAL_ANTECEDENT_POSITION:{}", slot + 1),
                "GLOBAL_RECENCY_OVERRIDDEN:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
            ],
        });
    }
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_tokens.join(" "),
        resolved_reference_count: bindings.len(),
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: bindings,
    })
}

fn local_order_slot(surface: &str) -> Option<usize> {
    match surface.to_lowercase().as_str() {
        "전자" | "전자를" | "전자는" | "전자가" | "former" => Some(0),
        "후자" | "후자를" | "후자는" | "후자가" | "latter" => Some(1),
        _ => None,
    }
}

fn local_ordered_antecedents(tokens: &[&str], english: bool) -> Vec<String> {
    let words = tokens
        .iter()
        .map(|token| token_parts(token).1)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    if english {
        for pair in words.windows(2) {
            let first = pair[0].to_lowercase();
            let second = pair[1].to_lowercase();
            if matches!(first.as_str(), "the" | "a" | "an") && valid_local_nominal(&second) {
                push_unique_surface(&mut candidates, second.clone());
            }
            if matches!(second.as_str(), "is" | "was" | "seems" | "looks")
                && valid_local_nominal(&first)
            {
                push_unique_surface(&mut candidates, first);
            }
        }
    } else {
        for word in words {
            for suffix in ["은", "는", "이", "가"] {
                if let Some(stem) = word.strip_suffix(suffix) {
                    if !stem.is_empty() && valid_local_nominal(stem) {
                        push_unique_surface(&mut candidates, clean_topic_surface(stem));
                    }
                    break;
                }
            }
        }
    }
    candidates
}

fn push_unique_surface(candidates: &mut Vec<String>, surface: String) {
    if !candidates
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&surface))
    {
        candidates.push(surface);
    }
}

fn realize_local_order_reference(reference: &str, surface: &str) -> String {
    match reference {
        "전자를" | "후자를" => format!("{surface}{}", object_particle(surface)),
        "전자는" | "후자는" => format!("{surface}{}", topic_particle(surface)),
        "전자가" | "후자가" => format!("{surface}{}", subject_particle(surface)),
        _ => surface.to_string(),
    }
}

fn resolve_same_turn_actor_reference(text: &str) -> Option<ReferenceResolutionIR> {
    if text.contains(['"', '\'', '‘', '’', '“', '”']) {
        return None;
    }
    let lower = text.to_lowercase();
    let attribution_forms = [
        " believes ",
        " says ",
        " reports ",
        " thinks ",
        " knows ",
        " expects ",
        " wants ",
    ];
    let first_mention = attribution_forms
        .iter()
        .filter_map(|form| lower.find(form).map(|position| (position, *form)))
        .min_by_key(|(position, _)| *position)?;
    let actor = lower[..first_mention.0]
        .split_whitespace()
        .next_back()?
        .trim_matches(|character: char| !character.is_alphanumeric());
    if actor.is_empty() || matches!(actor, "he" | "she" | "they" | "it" | "that" | "this") {
        return None;
    }
    let pronouns = [" she ", " he ", " they "];
    let (pronoun_position, pronoun) = pronouns
        .iter()
        .filter_map(|pronoun| {
            lower[first_mention.0 + first_mention.1.len()..]
                .find(pronoun)
                .map(|offset| (first_mention.0 + first_mention.1.len() + offset, *pronoun))
        })
        .min_by_key(|(position, _)| *position)?;
    let after_pronoun = &lower[pronoun_position + pronoun.len()..];
    if !attribution_forms
        .iter()
        .any(|form| after_pronoun.starts_with(form.trim_start()))
    {
        return None;
    }
    let resolved = format!(
        "{} {} {}",
        &text[..pronoun_position],
        actor,
        &text[pronoun_position + pronoun.len()..]
    );
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved,
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalAntecedentReference,
            source_surface: pronoun.trim().to_string(),
            resolved_surface: actor.to_string(),
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 930,
            evidence: vec![
                "SYNTACTIC_PRIORITY:SAME_TURN_ATTRIBUTION_ACTOR".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }],
    })
}

fn resolve_same_turn_result_reference(text: &str) -> Option<ReferenceResolutionIR> {
    let markers = [
        "that result",
        "that output",
        "that outcome",
        "그 결과",
        "그 출력",
        "그 산출물",
    ];
    let (marker_start, marker) = markers
        .into_iter()
        .filter_map(|marker| unquoted_marker_start(text, marker).map(|start| (start, marker)))
        .min_by_key(|(start, _)| *start)?;
    let prefix = &text[..marker_start];
    let event = preceding_result_event(prefix)?;
    let marker_end = marker_start + marker.len();
    let source_surface = text.get(marker_start..marker_end)?.to_string();
    let resolved_surface = if text_is_english(text) {
        format!("the result of ‘{event}’")
    } else {
        format!("‘{event}’의 결과")
    };
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: text.to_string(),
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalAntecedentReference,
            source_surface,
            resolved_surface,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 960,
            evidence: vec![
                "SYNTACTIC_PRIORITY:SAME_TURN_RESULT_OF_PRECEDING_EVENT".to_string(),
                "CROSS_TURN_RECENCY_OVERRIDDEN:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }],
    })
}

fn preceding_result_event(prefix: &str) -> Option<String> {
    let predicates = [
        "investigate",
        "assessment",
        "evaluate",
        "validate",
        "calculate",
        "summarize",
        "compare",
        "analyze",
        "inspect",
        "verify",
        "assess",
        "compute",
        "compile",
        "execute",
        "transform",
        "check",
        "test",
        "점검",
        "평가",
        "검증",
        "검사",
        "조사",
        "분석",
        "확인",
        "비교",
        "계산",
        "실행",
        "변환",
        "시험",
        "테스트",
    ];
    let (predicate_start, _) = predicates
        .into_iter()
        .filter_map(|predicate| {
            unquoted_marker_positions(prefix, predicate)
                .into_iter()
                .last()
                .map(|start| (start, predicate))
        })
        .max_by_key(|(start, _)| *start)?;
    let boundary = prefix[..predicate_start]
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '.' | '?' | '!' | ';'))
        .map_or(0, |(index, character)| index + character.len_utf8());
    let event = prefix[boundary..]
        .trim()
        .trim_start_matches([',', ':', '-'])
        .trim()
        .trim_end_matches(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | ':' | '-')
        });
    let lower = event.to_lowercase();
    let coordinated = lower.contains(" then ")
        || lower.contains(" and ")
        || lower.ends_with("하고")
        || lower.ends_with("한 뒤")
        || lower.ends_with("한 다음")
        || lower.ends_with("하고 나서")
        || lower.ends_with("해서")
        || lower.ends_with("후");
    if event.is_empty() || !coordinated {
        return None;
    }
    let event = event
        .trim_end_matches("then")
        .trim_end_matches("and")
        .trim_end_matches("하고")
        .trim_end_matches("한 뒤")
        .trim_end_matches("한 다음")
        .trim_end_matches("하고 나서")
        .trim_end_matches("해서")
        .trim_end_matches("후")
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | ':' | '-')
        });
    (!event.is_empty()).then(|| event.to_string())
}

fn resolve_same_turn_entity_reference(text: &str) -> Option<ReferenceResolutionIR> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let pronoun_index = tokens.iter().enumerate().rev().find_map(|(index, token)| {
        let core = token_parts(token).1.to_lowercase();
        matches!(
            core.as_str(),
            "그거" | "그것" | "그걸" | "그것을" | "그게" | "그것이" | "it"
        )
        .then_some(index)
    })?;
    if tokens[pronoun_index]
        .trim_matches(|character: char| !character.is_alphanumeric())
        .eq_ignore_ascii_case("it")
        && (english_it_is_expletive(&tokens, pronoun_index)
            || english_local_pronoun(&tokens, pronoun_index))
    {
        return None;
    }
    let antecedent = local_antecedent_surface(&tokens[..pronoun_index])?;
    let (prefix, core, suffix) = token_parts(tokens[pronoun_index]);
    let replacement = realize_reference(core, &antecedent);
    let mut resolved_tokens = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    resolved_tokens[pronoun_index] = format!("{prefix}{replacement}{suffix}");
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_tokens.join(" "),
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalAntecedentReference,
            source_surface: core.to_string(),
            resolved_surface: replacement,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 940,
            evidence: vec![
                "SYNTACTIC_PRIORITY:NEAREST_SAME_TURN_NOMINAL".to_string(),
                "GLOBAL_RECENCY_OVERRIDDEN:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
            ],
        }],
    })
}

fn local_antecedent_surface(tokens: &[&str]) -> Option<String> {
    let words = tokens
        .iter()
        .map(|token| {
            token_parts(token)
                .1
                .trim_matches(|character: char| !character.is_alphanumeric())
        })
        .collect::<Vec<_>>();
    if words.iter().any(|word| {
        word.chars()
            .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
    }) {
        return words.iter().rev().find_map(|word| {
            ["은", "는", "이", "가", "을", "를"]
                .iter()
                .find_map(|suffix| {
                    word.strip_suffix(suffix)
                        .filter(|stem| stem.chars().count() >= 1)
                        .map(clean_topic_surface)
                })
        });
    }
    let mut article_candidate = None;
    for pair in words.windows(2) {
        if matches!(pair[0].to_lowercase().as_str(), "the" | "a" | "an")
            && valid_local_nominal(pair[1])
        {
            article_candidate = Some(pair[1].to_lowercase());
        }
    }
    if article_candidate.is_some() {
        return article_candidate;
    }
    words.windows(2).rev().find_map(|pair| {
        matches!(
            pair[1].to_lowercase().as_str(),
            "is" | "was" | "seems" | "looks"
        )
        .then(|| pair[0].to_lowercase())
        .filter(|candidate| valid_local_nominal(candidate))
    })
}

fn valid_local_nominal(surface: &str) -> bool {
    !surface.is_empty()
        && ![
            "it", "that", "this", "and", "but", "although", "analyze", "repair", "inspect",
            "check", "the", "a", "an", "why", "who", "whom", "whose", "what", "which", "where",
            "when", "how", "is", "are", "was", "were", "has", "have", "did", "does", "do", "can",
            "could", "would", "should",
        ]
        .contains(&surface.to_lowercase().as_str())
}

fn english_it_is_expletive(tokens: &[&str], index: usize) -> bool {
    let next = tokens
        .get(index + 1)
        .map(|token| token_parts(token).1.to_lowercase());
    let complement = tokens
        .get(index + 2)
        .map(|token| token_parts(token).1.to_lowercase());
    next.is_some_and(|word| matches!(word.as_str(), "is" | "was" | "seems" | "feels"))
        && complement.is_some_and(|word| {
            matches!(
                word.as_str(),
                "worth"
                    | "costly"
                    | "painful"
                    | "hard"
                    | "difficult"
                    | "possible"
                    | "necessary"
                    | "important"
                    | "likely"
                    | "unlikely"
            )
        })
}

fn resolve_active_topic_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    if discourse_reference_kind(text).is_some()
        || topic_scoped_result_ellipsis_marker(state, text).is_some()
    {
        return None;
    }
    let topic = state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)?;
    let english = text_is_english(text);
    let surface = topic
        .concept_id_hint
        .as_deref()
        .and_then(|concept| topic_surface(concept, english))
        .unwrap_or(topic.surface.as_str());
    let inherited_goal_id = topic_goal_id(state, topic);
    if text.to_lowercase().contains("그 작업") {
        let replacement = format!("{surface} 작업");
        return Some(ReferenceResolutionIR {
            original_semantic_text: text.to_string(),
            resolved_semantic_text: replace_first_case_insensitive(text, "그 작업", &replacement),
            resolved_reference_count: 1,
            used_referent_ids: Vec::new(),
            ambiguous_reference_surfaces: Vec::new(),
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: vec![DiscourseBindingIR {
                kind: DiscourseBindingKindIR::TopicReference,
                source_surface: "그 작업".to_string(),
                resolved_surface: replacement,
                referent_ids: Vec::new(),
                inherited_goal_id,
                confidence_millis: 950,
                evidence: vec![
                    format!("ACTIVE_TOPIC:{}", topic.topic_id),
                    "EXPLICIT_TOPIC_OVERRIDES_EVENT_RECENCY:true".to_string(),
                    "SEMANTIC_AUTHORITY:false".to_string(),
                ],
            }],
        });
    }
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let pronoun_index = tokens.iter().enumerate().rev().find_map(|(index, token)| {
        let core = token_parts(token).1.to_lowercase();
        matches!(
            core.as_str(),
            "그거" | "그것" | "그걸" | "그것을" | "그게" | "그것이" | "그건" | "it" | "that"
        )
        .then_some(index)
    })?;
    if english_it_is_expletive(&tokens, pronoun_index) {
        return None;
    }
    let (prefix, core, suffix) = token_parts(tokens[pronoun_index]);
    let replacement = realize_reference(core, surface);
    let mut resolved_tokens = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    resolved_tokens[pronoun_index] = format!("{prefix}{replacement}{suffix}");
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_tokens.join(" "),
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::TopicReference,
            source_surface: core.to_string(),
            resolved_surface: replacement,
            referent_ids: Vec::new(),
            inherited_goal_id,
            confidence_millis: 930,
            evidence: vec![
                format!("ACTIVE_TOPIC:{}", topic.topic_id),
                "DISCOURSE_FOCUS_OVERRIDES_GLOBAL_RECENCY:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
            ],
        }],
    })
}

fn topic_goal_id(state: &ConversationStateIR, topic: &DiscourseTopicIR) -> Option<String> {
    let matches = state
        .active_goals
        .iter()
        .filter(|goal| topic_matches_subject(topic, &goal.subject))
        .collect::<Vec<_>>();
    let [goal] = matches.as_slice() else {
        return None;
    };
    Some(goal.goal_id.clone())
}

pub(crate) fn topic_matches_subject(topic: &DiscourseTopicIR, subject: &str) -> bool {
    let topic_concept = topic
        .concept_id_hint
        .clone()
        .or_else(|| discourse_topic_concept_id(&topic.surface));
    subject.eq_ignore_ascii_case(&topic.surface)
        || topic_concept
            .as_deref()
            .is_some_and(|concept| discourse_topic_concept_id(subject).as_deref() == Some(concept))
}

fn topic_context_has_distinct_local_focus(state: &ConversationStateIR) -> bool {
    state
        .topic_context_graph
        .active()
        .and_then(|context| context.current_focus_id.as_deref())
        .and_then(|focus_id| {
            state
                .discourse_focus
                .nodes
                .iter()
                .find(|node| node.focus_id == focus_id)
        })
        .is_some_and(|focus| focus.source != DiscourseFocusSourceIR::ExplicitTopic)
}

fn resolve_active_discourse_focus_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    let focus = state.discourse_focus.current().filter(|focus| {
        state
            .completed_turns
            .saturating_sub(focus.last_focused_turn)
            <= MAX_DISCOURSE_FOCUS_TURN_DISTANCE
    })?;
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    let pronoun_count = tokens
        .iter()
        .filter(|token| {
            matches!(
                token_parts(token).1.to_lowercase().as_str(),
                "그거" | "그것" | "그걸" | "그것을" | "그게" | "그것이" | "it"
            )
        })
        .count();
    if pronoun_count != 1 {
        return None;
    }
    let pronoun_index = tokens.iter().enumerate().rev().find_map(|(index, token)| {
        let core = token_parts(token).1.to_lowercase();
        matches!(
            core.as_str(),
            "그거" | "그것" | "그걸" | "그것을" | "그게" | "그것이" | "it"
        )
        .then_some(index)
    })?;
    if english_it_is_expletive(&tokens, pronoun_index) {
        return None;
    }
    let parallel_comparison_focus = focus.source_frame_id.as_deref().is_some_and(|frame_id| {
        let members = state
            .discourse_focus
            .nodes
            .iter()
            .filter(|node| {
                node.source_frame_id.as_deref() == Some(frame_id)
                    && node.introduced_turn == focus.introduced_turn
            })
            .collect::<Vec<_>>();
        members.len() >= 2
    });
    if parallel_comparison_focus {
        let core = token_parts(tokens[pronoun_index]).1;
        return Some(ReferenceResolutionIR {
            original_semantic_text: text.to_string(),
            resolved_semantic_text: text.to_string(),
            resolved_reference_count: 0,
            used_referent_ids: Vec::new(),
            ambiguous_reference_surfaces: vec![core.to_string()],
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: Vec::new(),
        });
    }
    let english = text_is_english(text);
    let surface = focus
        .concept_id_hint
        .as_deref()
        .and_then(|concept| topic_surface(concept, english))
        .unwrap_or(focus.surface.as_str());
    let (prefix, core, suffix) = token_parts(tokens[pronoun_index]);
    let replacement = realize_reference(core, surface);
    let mut resolved_tokens = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    resolved_tokens[pronoun_index] = format!("{prefix}{replacement}{suffix}");
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_tokens.join(" "),
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::DiscourseFocusReference,
            source_surface: core.to_string(),
            resolved_surface: replacement,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: focus.salience_millis.min(950),
            evidence: vec![
                format!("DISCOURSE_CENTER:{}", focus.focus_id),
                "CLAUSE_AWARE_FOCUS_OVERRIDES_FLAT_RECENCY:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }],
    })
}

fn resolve_active_typed_deixis(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    let focus = state.discourse_focus.current().filter(|focus| {
        state
            .completed_turns
            .saturating_sub(focus.last_focused_turn)
            <= MAX_DISCOURSE_FOCUS_TURN_DISTANCE
    })?;
    let english = text_is_english(text);
    let surface = focus
        .concept_id_hint
        .as_deref()
        .and_then(|concept| topic_surface(concept, english))
        .unwrap_or(focus.surface.as_str());
    let typed = resolve_typed_deixis_or_ellipsis(text, surface)?;
    let kind = match typed.kind {
        TypedDeixisEllipsisKindIR::PossessiveFocusReference => {
            DiscourseBindingKindIR::PossessiveFocusReference
        }
        TypedDeixisEllipsisKindIR::DemonstrativeFocusReference => {
            DiscourseBindingKindIR::DemonstrativeFocusReference
        }
        TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis => {
            DiscourseBindingKindIR::ZeroArgumentEllipsis
        }
    };
    let mut evidence = typed.evidence;
    evidence.push(format!("DISCOURSE_CENTER:{}", focus.focus_id));
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: typed.resolved_text.clone(),
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind,
            source_surface: typed.source_surface,
            resolved_surface: typed.resolved_text,
            referent_ids: Vec::new(),
            inherited_goal_id: focus.source_frame_id.clone(),
            confidence_millis: typed.confidence_millis.min(focus.salience_millis),
            evidence,
        }],
    })
}

fn typed_deixis_ambiguity_surface(kind: TypedDeixisEllipsisKindIR) -> &'static str {
    match kind {
        TypedDeixisEllipsisKindIR::PossessiveFocusReference => "POSSESSIVE_FOCUS_REFERENCE",
        TypedDeixisEllipsisKindIR::DemonstrativeFocusReference => "DEMONSTRATIVE_FOCUS_REFERENCE",
        TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis => "ZERO_ARGUMENT_ELLIPSIS",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopicAnchorReferenceDomain {
    Action,
    Proposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopicAnchorOrdinal {
    Position(usize),
    Last,
}

#[derive(Debug)]
struct TopicAnchorReferenceRequest {
    domain: TopicAnchorReferenceDomain,
    selector: TopicAnchoredSelectorKindIR,
    marker: String,
    ordinal: Option<TopicAnchorOrdinal>,
    predicate_role: Option<&'static str>,
}

fn resolve_topic_anchored_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    let topic = state.active_topics.first().filter(|topic| {
        topic.explicitly_activated
            && matches!(
                topic.anchor_kind,
                DiscourseTopicAnchorKindIR::ActionGroup
                    | DiscourseTopicAnchorKindIR::AttributedPropositionGroup
            )
    })?;
    let group_id = topic.anchor_group_id.as_deref()?;
    let group = state
        .active_discourse_groups
        .iter()
        .find(|group| group.group_id == group_id)?;
    let request = topic_anchor_reference_request(text)?;
    let expected_group_kind = match request.domain {
        TopicAnchorReferenceDomain::Action => DiscourseGroupKindIR::Action,
        TopicAnchorReferenceDomain::Proposition => DiscourseGroupKindIR::AttributedProposition,
    };
    let anchor_live = group.kind == expected_group_kind
        && topic.anchor_group_revision == Some(group.revision)
        && topic.anchor_membership_sha256.as_deref() == Some(&group.membership_sha256)
        && group.membership_sha256 == discourse_group_membership_sha256(group);
    if group.kind != expected_group_kind {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            TopicAnchoredSelectorKindIR::TypeMismatch,
            &request.marker,
            "ANCHOR_KIND_MISMATCH",
        ));
    }
    if !anchor_live {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "STALE_TOPIC_ANCHOR",
        ));
    }
    match request.domain {
        TopicAnchorReferenceDomain::Action => {
            resolve_topic_anchored_action(state, text, topic, group, &request)
        }
        TopicAnchorReferenceDomain::Proposition => {
            resolve_topic_anchored_proposition(state, text, topic, group, &request)
        }
    }
}

fn resolve_topic_anchored_action(
    state: &ConversationStateIR,
    text: &str,
    topic: &DiscourseTopicIR,
    group: &DiscourseGroupIR,
    request: &TopicAnchorReferenceRequest,
) -> Option<ReferenceResolutionIR> {
    let records = group
        .member_keys
        .iter()
        .filter_map(|goal_id| {
            state
                .action_state_ledger
                .records
                .iter()
                .find(|record| record.goal_id == *goal_id)
        })
        .collect::<Vec<_>>();
    if records.len() != group.member_keys.len() {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "ANCHOR_MEMBER_UNAVAILABLE",
        ));
    }
    let selected = match request.selector {
        TopicAnchoredSelectorKindIR::Plural => records.clone(),
        TopicAnchoredSelectorKindIR::Ordinal => {
            let index = match request.ordinal {
                Some(TopicAnchorOrdinal::Position(index)) => Some(index),
                Some(TopicAnchorOrdinal::Last) => records.len().checked_sub(1),
                None => None,
            };
            let Some(record) = index.and_then(|index| records.get(index)).copied() else {
                return Some(unresolved_topic_anchor_reference(
                    text,
                    topic,
                    group,
                    request.selector,
                    &request.marker,
                    "ORDINAL_OUT_OF_RANGE",
                ));
            };
            vec![record]
        }
        TopicAnchoredSelectorKindIR::PredicateRole => {
            let role = request.predicate_role?;
            let matches = records
                .iter()
                .copied()
                .filter(|record| topic_anchor_predicate_role_matches(record, role))
                .collect::<Vec<_>>();
            let [record] = matches.as_slice() else {
                let term = if matches.is_empty() {
                    "PREDICATE_ROLE_NOT_FOUND"
                } else {
                    "AMBIGUOUS_PREDICATE_ROLE"
                };
                return Some(unresolved_topic_anchor_reference(
                    text,
                    topic,
                    group,
                    request.selector,
                    &request.marker,
                    term,
                ));
            };
            vec![*record]
        }
        TopicAnchoredSelectorKindIR::GenericSingular
        | TopicAnchoredSelectorKindIR::ZeroArgument => {
            let [record] = records.as_slice() else {
                return Some(unresolved_topic_anchor_reference(
                    text,
                    topic,
                    group,
                    request.selector,
                    &request.marker,
                    "AMBIGUOUS_GROUP_MEMBER",
                ));
            };
            vec![*record]
        }
        TopicAnchoredSelectorKindIR::TypeMismatch => return None,
    };
    let surfaces = selected
        .iter()
        .map(|record| record.subject.clone())
        .collect::<Vec<_>>();
    let replacement = if selected.len() == 1 {
        realize_topic_anchor_marker(&request.marker, &surfaces[0])
    } else {
        realize_plural_reference(&request.marker, &surfaces)
    };
    let resolved_text = if request.selector == TopicAnchoredSelectorKindIR::ZeroArgument {
        resolve_typed_deixis_or_ellipsis(text, &surfaces[0])?.resolved_text
    } else {
        replace_first_case_insensitive(text, &request.marker, &replacement)
    };
    if resolved_text == text {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "RESOLUTION_REWRITE_FAILED",
        ));
    }
    let selected_keys = selected
        .iter()
        .map(|record| record.goal_id.clone())
        .collect::<Vec<_>>();
    let kind = if selected.len() == 1 {
        TopicAnchoredReferentKindIR::ActionMember
    } else {
        TopicAnchoredReferentKindIR::ActionGroup
    };
    let anchored = seal_topic_anchored_reference(TopicAnchoredReferenceIR {
        schema: TOPIC_ANCHORED_REFERENCE_SCHEMA.to_string(),
        applied: true,
        kind,
        selector: request.selector,
        original_text: text.to_string(),
        resolved_text: resolved_text.clone(),
        source_surface: request.marker.clone(),
        topic_id: topic.topic_id.clone(),
        topic_sha256: topic.topic_sha256.clone(),
        anchor_kind: topic.anchor_kind,
        group_id: group.group_id.clone(),
        group_revision: group.revision,
        membership_sha256: group.membership_sha256.clone(),
        member_keys: group.member_keys.clone(),
        selected_member_keys: selected_keys.clone(),
        unresolved_terms: Vec::new(),
        semantic_authority: false,
        external_execution_authorized: false,
        resolution_sha256: String::new(),
    });
    let referent_ids = selected_keys
        .iter()
        .filter_map(|goal_id| goal_id.strip_prefix("GOAL-"))
        .map(|suffix| format!("DREF-E-{suffix}"))
        .filter(|referent_id| {
            state
                .active_discourse_referents
                .iter()
                .any(|referent| referent.referent_id == *referent_id)
        })
        .collect::<Vec<_>>();
    let binding_kind = if selected.len() == 1 {
        DiscourseBindingKindIR::TopicAnchoredActionMemberReference
    } else {
        DiscourseBindingKindIR::TopicAnchoredActionGroupReference
    };
    let evidence = topic_anchor_evidence(topic, group, &anchored);
    let mut bindings = vec![DiscourseBindingIR {
        kind: binding_kind,
        source_surface: request.marker.clone(),
        resolved_surface: replacement,
        referent_ids: referent_ids.clone(),
        inherited_goal_id: (selected.len() == 1).then(|| selected[0].goal_id.clone()),
        confidence_millis: if selected.len() == 1 { 970 } else { 960 },
        evidence: evidence.clone(),
    }];
    if selected.len() > 1 {
        bindings.extend(selected.iter().filter_map(|record| {
            let suffix = record.goal_id.strip_prefix("GOAL-")?;
            Some(DiscourseBindingIR {
                kind: DiscourseBindingKindIR::PluralEventMemberReference,
                source_surface: request.marker.clone(),
                resolved_surface: record.subject.clone(),
                referent_ids: vec![format!("DREF-E-{suffix}")],
                inherited_goal_id: Some(record.goal_id.clone()),
                confidence_millis: 960,
                evidence: evidence.clone(),
            })
        }));
    }
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_text,
        resolved_reference_count: 1,
        used_referent_ids: referent_ids,
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: Some(anchored),
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: bindings,
    })
}

fn resolve_topic_anchored_proposition(
    state: &ConversationStateIR,
    text: &str,
    topic: &DiscourseTopicIR,
    group: &DiscourseGroupIR,
    request: &TopicAnchorReferenceRequest,
) -> Option<ReferenceResolutionIR> {
    let PropositionGroupLookup::Selected(mut members) =
        persistent_proposition_group_members(state, DiscourseGroupSelection::ActiveTopic, text)
    else {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "ANCHOR_MEMBER_UNAVAILABLE",
        ));
    };
    members.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.referent_id.cmp(&right.referent_id))
    });
    let selected = match request.selector {
        TopicAnchoredSelectorKindIR::Plural => members.clone(),
        TopicAnchoredSelectorKindIR::Ordinal => {
            let index = match request.ordinal {
                Some(TopicAnchorOrdinal::Position(index)) => Some(index),
                Some(TopicAnchorOrdinal::Last) => members.len().checked_sub(1),
                None => None,
            };
            let Some(referent) = index.and_then(|index| members.get(index)).copied() else {
                return Some(unresolved_topic_anchor_reference(
                    text,
                    topic,
                    group,
                    request.selector,
                    &request.marker,
                    "ORDINAL_OUT_OF_RANGE",
                ));
            };
            vec![referent]
        }
        TopicAnchoredSelectorKindIR::GenericSingular => {
            let [referent] = members.as_slice() else {
                return Some(unresolved_topic_anchor_reference(
                    text,
                    topic,
                    group,
                    request.selector,
                    &request.marker,
                    "AMBIGUOUS_GROUP_MEMBER",
                ));
            };
            vec![*referent]
        }
        TopicAnchoredSelectorKindIR::PredicateRole
        | TopicAnchoredSelectorKindIR::ZeroArgument
        | TopicAnchoredSelectorKindIR::TypeMismatch => return None,
    };
    let sources = selected
        .iter()
        .filter_map(|referent| referent.attributed_source.clone())
        .collect::<Vec<_>>();
    if sources.len() != selected.len() {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "ATTRIBUTED_SOURCE_UNAVAILABLE",
        ));
    }
    let replacement = if selected.len() == 1 {
        realize_topic_anchor_marker(&request.marker, &sources[0])
    } else {
        realize_proposition_group_marker(&request.marker, &sources)
    };
    let resolved_text = replace_first_case_insensitive(text, &request.marker, &replacement);
    if resolved_text == text {
        return Some(unresolved_topic_anchor_reference(
            text,
            topic,
            group,
            request.selector,
            &request.marker,
            "RESOLUTION_REWRITE_FAILED",
        ));
    }
    let selected_keys = sources
        .iter()
        .map(|source| normalize_group_member_key(source))
        .collect::<Vec<_>>();
    let kind = if selected.len() == 1 {
        TopicAnchoredReferentKindIR::PropositionMember
    } else {
        TopicAnchoredReferentKindIR::PropositionGroup
    };
    let anchored = seal_topic_anchored_reference(TopicAnchoredReferenceIR {
        schema: TOPIC_ANCHORED_REFERENCE_SCHEMA.to_string(),
        applied: true,
        kind,
        selector: request.selector,
        original_text: text.to_string(),
        resolved_text: resolved_text.clone(),
        source_surface: request.marker.clone(),
        topic_id: topic.topic_id.clone(),
        topic_sha256: topic.topic_sha256.clone(),
        anchor_kind: topic.anchor_kind,
        group_id: group.group_id.clone(),
        group_revision: group.revision,
        membership_sha256: group.membership_sha256.clone(),
        member_keys: group.member_keys.clone(),
        selected_member_keys: selected_keys,
        unresolved_terms: Vec::new(),
        semantic_authority: false,
        external_execution_authorized: false,
        resolution_sha256: String::new(),
    });
    let referent_ids = selected
        .iter()
        .map(|referent| referent.referent_id.clone())
        .collect::<Vec<_>>();
    let binding_kind = if selected.len() == 1 {
        DiscourseBindingKindIR::TopicAnchoredPropositionMemberReference
    } else {
        DiscourseBindingKindIR::TopicAnchoredPropositionGroupReference
    };
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_text,
        resolved_reference_count: 1,
        used_referent_ids: referent_ids.clone(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: Some(anchored.clone()),
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: binding_kind,
            source_surface: request.marker.clone(),
            resolved_surface: replacement,
            referent_ids,
            inherited_goal_id: None,
            confidence_millis: if selected.len() == 1 { 970 } else { 960 },
            evidence: topic_anchor_evidence(topic, group, &anchored),
        }],
    })
}

fn unresolved_topic_anchor_reference(
    text: &str,
    topic: &DiscourseTopicIR,
    group: &DiscourseGroupIR,
    selector: TopicAnchoredSelectorKindIR,
    source_surface: &str,
    term: &str,
) -> ReferenceResolutionIR {
    let anchored = seal_topic_anchored_reference(TopicAnchoredReferenceIR {
        schema: TOPIC_ANCHORED_REFERENCE_SCHEMA.to_string(),
        applied: false,
        kind: TopicAnchoredReferentKindIR::Unresolved,
        selector,
        original_text: text.to_string(),
        resolved_text: text.to_string(),
        source_surface: source_surface.to_string(),
        topic_id: topic.topic_id.clone(),
        topic_sha256: topic.topic_sha256.clone(),
        anchor_kind: topic.anchor_kind,
        group_id: group.group_id.clone(),
        group_revision: group.revision,
        membership_sha256: group.membership_sha256.clone(),
        member_keys: group.member_keys.clone(),
        selected_member_keys: Vec::new(),
        unresolved_terms: vec![term.to_string()],
        semantic_authority: false,
        external_execution_authorized: false,
        resolution_sha256: String::new(),
    });
    ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: text.to_string(),
        resolved_reference_count: 0,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: vec![format!("TOPIC_ANCHORED_REFERENCE:{term}")],
        topic_anchored_resolution: Some(anchored),
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: Vec::new(),
    }
}

fn topic_anchor_evidence(
    topic: &DiscourseTopicIR,
    group: &DiscourseGroupIR,
    anchored: &TopicAnchoredReferenceIR,
) -> Vec<String> {
    vec![
        format!("TOPIC_ID:{}", topic.topic_id),
        format!("TOPIC_SHA256:{}", topic.topic_sha256),
        format!("ANCHOR_GROUP_ID:{}", group.group_id),
        format!("ANCHOR_GROUP_REVISION:{}", group.revision),
        format!("ANCHOR_MEMBERSHIP_SHA256:{}", group.membership_sha256),
        format!(
            "TOPIC_ANCHORED_RESOLUTION_SHA256:{}",
            anchored.resolution_sha256
        ),
        "LIVE_TOPIC_GROUP_ANCHOR:true".to_string(),
        "SEMANTIC_AUTHORITY:false".to_string(),
        "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
    ]
}

fn topic_anchor_reference_request(text: &str) -> Option<TopicAnchorReferenceRequest> {
    let lower = text.to_lowercase();
    if has_balanced_quotation(text)
        && [
            "explain", "describe", "meaning", "sentence", "설명", "문장", "뜻", "표현",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return None;
    }
    let proposition_context = proposition_reference_context(text);
    let action_context = action_reference_context(text);
    if let Some((marker, ordinal)) = topic_anchor_ordinal_marker(text) {
        let domain = if proposition_context && !action_context {
            TopicAnchorReferenceDomain::Proposition
        } else if action_context && !proposition_context {
            TopicAnchorReferenceDomain::Action
        } else if speaker_ordinal_marker(&marker) {
            TopicAnchorReferenceDomain::Proposition
        } else {
            TopicAnchorReferenceDomain::Action
        };
        return Some(TopicAnchorReferenceRequest {
            domain,
            selector: TopicAnchoredSelectorKindIR::Ordinal,
            marker,
            ordinal: Some(ordinal),
            predicate_role: None,
        });
    }
    if action_context {
        if let Some((marker, role)) = topic_anchor_predicate_marker(text) {
            return Some(TopicAnchorReferenceRequest {
                domain: TopicAnchorReferenceDomain::Action,
                selector: TopicAnchoredSelectorKindIR::PredicateRole,
                marker,
                ordinal: None,
                predicate_role: Some(role),
            });
        }
    }
    if proposition_context {
        if let Some(marker) = find_unquoted_topic_anchor_marker(
            text,
            &[
                "their reports",
                "their report",
                "their claims",
                "they",
                "그들의 보고를",
                "그들의 보고",
                "그들의 주장",
                "그들은",
                "그들이",
            ],
        ) {
            return Some(TopicAnchorReferenceRequest {
                domain: TopicAnchorReferenceDomain::Proposition,
                selector: TopicAnchoredSelectorKindIR::Plural,
                marker,
                ordinal: None,
                predicate_role: None,
            });
        }
    }
    if action_context {
        if let Some(marker) = find_unquoted_topic_anchor_marker(
            text,
            &[
                "all of them",
                "both of them",
                "them",
                "both",
                "그것들 전부를",
                "그것들 전부",
                "그것들을",
                "그것들",
                "둘 다",
            ],
        ) {
            return Some(TopicAnchorReferenceRequest {
                domain: TopicAnchorReferenceDomain::Action,
                selector: TopicAnchoredSelectorKindIR::Plural,
                marker,
                ordinal: None,
                predicate_role: None,
            });
        }
        if let Some(marker) = find_unquoted_topic_anchor_marker(
            text,
            &[
                "that one",
                "that task",
                "그 대상을",
                "그 항목을",
                "그것을",
                "그거를",
                "그걸",
                "그것",
            ],
        ) {
            return Some(TopicAnchorReferenceRequest {
                domain: TopicAnchorReferenceDomain::Action,
                selector: TopicAnchoredSelectorKindIR::GenericSingular,
                marker,
                ordinal: None,
                predicate_role: None,
            });
        }
        if unresolved_typed_deixis_kind(text)
            == Some(TypedDeixisEllipsisKindIR::ZeroArgumentEllipsis)
        {
            return Some(TopicAnchorReferenceRequest {
                domain: TopicAnchorReferenceDomain::Action,
                selector: TopicAnchoredSelectorKindIR::ZeroArgument,
                marker: text.trim().to_string(),
                ordinal: None,
                predicate_role: None,
            });
        }
    }
    None
}

fn has_balanced_quotation(text: &str) -> bool {
    let paired = [('“', '”'), ('‘', '’'), ('「', '」')];
    paired
        .iter()
        .any(|(open, close)| text.contains(*open) && text.contains(*close))
        || ['"', '\'']
            .into_iter()
            .any(|quote| text.chars().filter(|character| *character == quote).count() >= 2)
}

fn topic_anchor_ordinal_marker(text: &str) -> Option<(String, TopicAnchorOrdinal)> {
    [
        (
            TopicAnchorOrdinal::Last,
            &[
                "the latter speaker",
                "latter speaker",
                "the last one",
                "the last task",
                "last one",
                "last task",
                "마지막 사람은",
                "마지막 사람이",
                "마지막 것을",
                "마지막 것",
                "마지막 작업을",
                "마지막 작업",
                "뒤 사람이",
                "뒤 사람은",
                "뒤 사람",
            ][..],
        ),
        (
            TopicAnchorOrdinal::Position(3),
            &[
                "the fourth one",
                "the fourth task",
                "fourth one",
                "fourth task",
                "네 번째 것을",
                "네 번째 것",
                "네 번째 작업을",
                "네 번째 작업",
            ][..],
        ),
        (
            TopicAnchorOrdinal::Position(2),
            &[
                "the third one",
                "the third task",
                "third one",
                "third task",
                "세 번째 것을",
                "세 번째 것",
                "세 번째 작업을",
                "세 번째 작업",
            ][..],
        ),
        (
            TopicAnchorOrdinal::Position(1),
            &[
                "the second speaker",
                "the second one",
                "the second task",
                "second speaker",
                "second one",
                "second task",
                "두 번째 사람은",
                "두 번째 사람이",
                "두 번째 사람",
                "두 번째 것을",
                "두 번째 것",
                "두 번째 작업을",
                "두 번째 작업",
            ][..],
        ),
        (
            TopicAnchorOrdinal::Position(0),
            &[
                "the first speaker",
                "the first one",
                "the first task",
                "first speaker",
                "first one",
                "first task",
                "첫 번째 화자가",
                "첫 번째 화자는",
                "첫 번째 화자",
                "첫 번째 사람은",
                "첫 번째 사람이",
                "첫 번째 사람",
                "첫 번째 것을",
                "첫 번째 것",
                "첫 번째 작업을",
                "첫 번째 작업",
            ][..],
        ),
    ]
    .into_iter()
    .find_map(|(ordinal, markers)| {
        find_unquoted_topic_anchor_marker(text, markers).map(|marker| (marker, ordinal))
    })
}

fn topic_anchor_predicate_marker(text: &str) -> Option<(String, &'static str)> {
    [
        (
            "REPAIR",
            &[
                "whichever task was for repair",
                "the one being repaired",
                "the repair task",
                "repair task",
                "수리하는 것을",
                "수리하는 것",
                "수리 작업을",
                "수리 작업",
                "수리하던 쪽을",
                "수리하던 쪽",
            ][..],
        ),
        (
            "ANALYZE_SURFACE",
            &[
                "the one being analyzed",
                "the analysis task",
                "analysis task",
                "분석하는 것을",
                "분석하는 것",
                "분석 작업을",
                "분석 작업",
            ][..],
        ),
        (
            "INVESTIGATE",
            &[
                "the inspection task",
                "inspection task",
                "확인 작업을",
                "확인 작업",
                "검사 작업을",
                "검사 작업",
            ][..],
        ),
    ]
    .into_iter()
    .find_map(|(role, markers)| {
        find_unquoted_topic_anchor_marker(text, markers).map(|marker| (marker, role))
    })
}

fn find_unquoted_topic_anchor_marker(text: &str, markers: &[&str]) -> Option<String> {
    let lower = text.to_lowercase();
    markers.iter().find_map(|marker| {
        lower.match_indices(marker).find_map(|(start, _)| {
            let end = start + marker.len();
            let ascii_boundary = !marker.is_ascii()
                || ((start == 0
                    || lower[..start]
                        .chars()
                        .next_back()
                        .is_none_or(|character| !character.is_ascii_alphanumeric()))
                    && (end == lower.len()
                        || lower[end..]
                            .chars()
                            .next()
                            .is_none_or(|character| !character.is_ascii_alphanumeric())));
            (ascii_boundary && !marker_is_quoted(text, start)).then(|| text[start..end].to_string())
        })
    })
}

fn action_reference_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "inspect",
        "check",
        "analyze",
        "repair",
        "run",
        "execute",
        "recheck",
        "retry",
        "검사",
        "확인",
        "분석",
        "수리",
        "실행",
        "재시도",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn proposition_reference_context(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "say",
        "said",
        "report",
        "claim",
        "statement",
        "summarize",
        "what did",
        "말",
        "보고",
        "주장",
        "내용",
        "요약",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn speaker_ordinal_marker(marker: &str) -> bool {
    let lower = marker.to_lowercase();
    ["speaker", "사람", "화자"]
        .iter()
        .any(|term| lower.contains(term))
}

fn topic_anchor_predicate_role_matches(
    record: &crate::action_state::ActionStateRecordIR,
    role: &str,
) -> bool {
    match role {
        "ANALYZE_SURFACE" => matches!(
            record.predicate_surface.to_lowercase().as_str(),
            "analyze" | "analyse" | "분석"
        ),
        _ => record.canonical_predicate == role,
    }
}

fn realize_topic_anchor_marker(marker: &str, surface: &str) -> String {
    if marker.is_ascii() {
        return surface.to_string();
    }
    if marker.ends_with('을') || marker.ends_with('를') {
        format!("{surface}{}", object_particle(surface))
    } else if marker.ends_with('은') || marker.ends_with('는') {
        format!("{surface}{}", topic_particle(surface))
    } else if marker.ends_with('이') || marker.ends_with('가') {
        format!("{surface}{}", subject_particle(surface))
    } else if marker.ends_with('의') {
        format!("{surface}의")
    } else {
        surface.to_string()
    }
}

fn realize_proposition_group_marker(marker: &str, sources: &[String]) -> String {
    let phrase = realize_plural_reference(marker, sources);
    let lower = marker.to_lowercase();
    if lower.starts_with("their report") {
        format!("{phrase}'s reports")
    } else if lower.starts_with("their claim") {
        format!("{phrase}'s claims")
    } else if marker.contains("보고") {
        format!("{phrase}의 보고")
    } else if marker.contains("주장") {
        format!("{phrase}의 주장")
    } else if marker.ends_with('은') || marker.ends_with('는') {
        format!("{phrase}{}", topic_particle(&phrase))
    } else if marker.ends_with('이') || marker.ends_with('가') {
        format!("{phrase}{}", subject_particle(&phrase))
    } else {
        phrase
    }
}

fn discourse_focus_should_override_flat_recency(state: &ConversationStateIR) -> bool {
    if state.discourse_focus.nodes.len() > 1 {
        return true;
    }
    let latest_referent_turn = state
        .active_referents
        .iter()
        .map(|referent| referent.last_referenced_turn)
        .max();
    let latest_referent_count = state
        .active_referents
        .iter()
        .filter(|referent| Some(referent.last_referenced_turn) == latest_referent_turn)
        .take(2)
        .count();
    if latest_referent_count != 1 {
        return true;
    }
    let latest_turn = state
        .active_typed_entities
        .iter()
        .map(|entity| entity.last_mentioned_turn)
        .max();
    state
        .active_typed_entities
        .iter()
        .filter(|entity| Some(entity.last_mentioned_turn) == latest_turn)
        .take(2)
        .count()
        > 1
}

fn topic_surface(concept: &str, english: bool) -> Option<&'static str> {
    [
        ("TOPIC_CACHE", "캐시", "cache"),
        ("TOPIC_QUEUE", "큐", "queue"),
        ("TOPIC_BACKUP", "백업", "backup"),
        ("TOPIC_LOG", "로그", "log"),
        ("TOPIC_SERVER", "서버", "server"),
        ("TOPIC_WORKER", "워커", "worker"),
        ("C_OBJECT_FILE", "파일", "file"),
        ("C_OBJECT_FOLDER", "폴더", "folder"),
        ("C_OBJECT_SOURCE_CODE", "코드", "code"),
        ("C_OBJECT_DOCUMENT", "문서", "document"),
        ("C_OBJECT_REPORT", "보고서", "report"),
        ("C_OBJECT_PROJECT", "프로젝트", "project"),
        ("C_OBJECT_REPOSITORY", "저장소", "repository"),
    ]
    .into_iter()
    .find(|(candidate, _, _)| *candidate == concept)
    .map(|(_, korean, english_surface)| if english { english_surface } else { korean })
}

fn resolve_local_conditional_reference(text: &str) -> Option<ReferenceResolutionIR> {
    if is_continuation_task_anaphor(text) {
        return None;
    }
    let graph = ModalSemanticAnalyzer.analyze(text);
    let conditional = (graph.conditionals.len() == 1).then(|| &graph.conditionals[0])?;
    let consequent_tokens = conditional
        .consequent
        .split_whitespace()
        .collect::<Vec<_>>();
    let pronoun_count = consequent_tokens
        .iter()
        .filter(|token| token_parts(token).1.eq_ignore_ascii_case("it"))
        .count();
    if pronoun_count != 1 {
        return None;
    }
    let subject = proposition_signature(
        &conditional.antecedent,
        AttributedPropositionPolarityIR::Positive,
    )
    .subject_key;
    if subject == "unknown_subject" || subject.trim().is_empty() {
        return None;
    }
    let resolved_consequent = consequent_tokens
        .iter()
        .map(|token| {
            let (prefix, core, suffix) = token_parts(token);
            if core.eq_ignore_ascii_case("it") {
                format!("{prefix}{subject}{suffix}")
            } else {
                (*token).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let resolved = text.replacen(&conditional.consequent, &resolved_consequent, 1);
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved,
        resolved_reference_count: 1,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalAntecedentReference,
            source_surface: "it".to_string(),
            resolved_surface: subject,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 930,
            evidence: vec!["LOCAL_CONDITIONAL_ANTECEDENT".to_string()],
        }],
    })
}

pub(crate) fn is_continuation_task_anaphor(text: &str) -> bool {
    let normalized = text.to_lowercase();
    if [
        "그 일을 계속",
        "그 일을 이어",
        "그 작업을 계속",
        "그 작업을 이어",
        "그 작업 계속",
        "그 작업 이어",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
    {
        return true;
    }
    let words = text
        .split_whitespace()
        .map(|token| token_parts(token).1.to_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    words.windows(2).any(|pair| {
        matches!(
            (pair[0].as_str(), pair[1].as_str()),
            ("continue" | "continuing" | "proceed", "it" | "that")
        )
    }) || words.windows(3).any(|triple| {
        matches!(
            (triple[0].as_str(), triple[1].as_str(), triple[2].as_str()),
            ("keep", "at", "it" | "that")
                | ("proceed", "with", "it" | "that")
                | ("keep", "doing", "it" | "that")
        )
    })
}

fn continuation_task_anaphor_at(tokens: &[&str], index: usize) -> bool {
    let previous = index
        .checked_sub(1)
        .and_then(|position| tokens.get(position))
        .map(|token| token_parts(token).1.to_lowercase());
    let two_back = index
        .checked_sub(2)
        .and_then(|position| tokens.get(position))
        .map(|token| token_parts(token).1.to_lowercase());
    previous
        .as_deref()
        .is_some_and(|word| matches!(word, "continue" | "continuing" | "proceed" | "doing"))
        || matches!(
            (two_back.as_deref(), previous.as_deref()),
            (Some("keep"), Some("at"))
                | (Some("proceed"), Some("with"))
                | (Some("keep"), Some("doing"))
        )
}

fn ambiguity_blocks_surface(ambiguous_surface: &str, core: &str) -> bool {
    if ambiguous_surface.eq_ignore_ascii_case(core) {
        return true;
    }
    ambiguous_surface
        .rsplit_once(':')
        .map(|(_, marker)| marker)
        .unwrap_or(ambiguous_surface)
        .split_whitespace()
        .next()
        .is_some_and(|marker| marker.eq_ignore_ascii_case(core))
}

fn event_and_result_referents(
    goal: &ConversationGoalFrameIR,
    turn_index: u64,
    index: usize,
    topic_id: Option<&str>,
) -> [DynamicDiscourseReferentIR; 2] {
    let suffix = index + 1;
    [
        DynamicDiscourseReferentIR {
            referent_id: format!("DREF-E-{turn_index:06}-{suffix:02}"),
            kind: DiscourseReferentKindIR::Event,
            topic_id: topic_id.map(ToString::to_string),
            semantic_summary: goal.source_semantic_text.clone(),
            attributed_source: None,
            attribution_attitude: None,
            epistemic_status: None,
            proposition_polarity: None,
            modal_world: None,
            belief_record_id: None,
            introduced_turn: turn_index,
            last_referenced_turn: turn_index,
            external_execution_authorized: goal.external_execution_authorized,
        },
        DynamicDiscourseReferentIR {
            referent_id: format!("DREF-R-{turn_index:06}-{suffix:02}"),
            kind: DiscourseReferentKindIR::Result,
            topic_id: topic_id.map(ToString::to_string),
            semantic_summary: goal.source_semantic_text.clone(),
            attributed_source: None,
            attribution_attitude: None,
            epistemic_status: None,
            proposition_polarity: None,
            modal_world: None,
            belief_record_id: None,
            introduced_turn: turn_index,
            last_referenced_turn: turn_index,
            external_execution_authorized: false,
        },
    ]
}

fn action_plan_seed(goal: &ConversationGoalFrameIR) -> ActionPlanSeedIR {
    ActionPlanSeedIR {
        action_id: goal.goal_id.clone(),
        goal_id: goal.goal_id.clone(),
        canonical_predicate: goal.canonical_predicate.clone(),
        predicate_surface: goal.predicate_surface.clone(),
        subject: goal.subject.clone(),
        source_semantic_text: goal.source_semantic_text.clone(),
        introduced_turn: goal.introduced_turn,
        external_execution_authorized: goal.external_execution_authorized,
    }
}

fn remember_action_group(
    state: &mut ConversationStateIR,
    goals: &[ConversationGoalFrameIR],
    turn_index: u64,
) {
    if goals.len() < 2 {
        return;
    }
    let member_keys = goals
        .iter()
        .map(|goal| goal.goal_id.clone())
        .collect::<Vec<_>>();
    let topic_keys = goals
        .iter()
        .flat_map(|goal| topic_keys_from_text(&goal.subject))
        .collect::<Vec<_>>();
    remember_discourse_group(
        state,
        DiscourseGroupKindIR::Action,
        member_keys,
        topic_keys,
        turn_index,
    );
}

fn update_discourse_groups_from_references(
    state: &mut ConversationStateIR,
    used_referent_ids: &[String],
    turn_index: u64,
) {
    if used_referent_ids.is_empty() {
        return;
    }
    let used = used_referent_ids.iter().collect::<BTreeSet<_>>();
    let referenced_sources = state
        .active_discourse_referents
        .iter()
        .filter(|referent| {
            referent.kind == DiscourseReferentKindIR::Proposition
                && used.contains(&referent.referent_id)
        })
        .filter_map(|referent| referent.attributed_source.as_deref())
        .map(normalize_group_member_key)
        .collect::<Vec<_>>();
    let unique_sources = referenced_sources.iter().collect::<BTreeSet<_>>();
    if referenced_sources.len() >= 2 && unique_sources.len() == referenced_sources.len() {
        let topic_keys = state
            .active_discourse_referents
            .iter()
            .filter(|referent| {
                referent.kind == DiscourseReferentKindIR::Proposition
                    && used.contains(&referent.referent_id)
            })
            .flat_map(|referent| topic_keys_from_text(&referent.semantic_summary))
            .collect::<Vec<_>>();
        remember_discourse_group(
            state,
            DiscourseGroupKindIR::AttributedProposition,
            referenced_sources,
            topic_keys,
            turn_index,
        );
    }

    for group in &mut state.active_discourse_groups {
        let fully_referenced = match group.kind {
            DiscourseGroupKindIR::Action => group.member_keys.iter().all(|goal_id| {
                goal_id.strip_prefix("GOAL-").is_some_and(|suffix| {
                    let referent_id = format!("DREF-E-{suffix}");
                    used_referent_ids.contains(&referent_id)
                })
            }),
            DiscourseGroupKindIR::AttributedProposition => {
                group.member_keys.iter().all(|source| {
                    state.active_discourse_referents.iter().any(|referent| {
                        referent.kind == DiscourseReferentKindIR::Proposition
                            && used.contains(&referent.referent_id)
                            && referent
                                .attributed_source
                                .as_deref()
                                .is_some_and(|candidate| {
                                    normalize_group_member_key(candidate) == *source
                                })
                    })
                }) && (group.topic_keys.is_empty()
                    || group.topic_keys.iter().all(|topic| {
                        state.active_discourse_referents.iter().any(|referent| {
                            referent.kind == DiscourseReferentKindIR::Proposition
                                && used.contains(&referent.referent_id)
                                && topic_keys_from_text(&referent.semantic_summary).contains(topic)
                        })
                    }))
            }
        };
        if fully_referenced {
            group.last_referenced_turn = turn_index;
        }
    }
}

fn remember_discourse_group(
    state: &mut ConversationStateIR,
    kind: DiscourseGroupKindIR,
    mut member_keys: Vec<String>,
    mut topic_keys: Vec<String>,
    turn_index: u64,
) {
    member_keys.retain(|member| !member.trim().is_empty());
    let mut seen = BTreeSet::new();
    member_keys.retain(|member| seen.insert(member.clone()));
    member_keys.sort();
    topic_keys.retain(|topic| !topic.trim().is_empty());
    topic_keys.sort();
    topic_keys.dedup();
    if member_keys.len() < 2 || member_keys.len() > MAX_ACTIVE_GOALS {
        return;
    }
    if let Some(existing) = state.active_discourse_groups.iter_mut().find(|group| {
        group.kind == kind && group.member_keys == member_keys && group.topic_keys == topic_keys
    }) {
        existing.last_referenced_turn = turn_index;
        return;
    }
    let kind_label = match kind {
        DiscourseGroupKindIR::Action => "ACTION",
        DiscourseGroupKindIR::AttributedProposition => "ATTRIBUTED_PROPOSITION",
    };
    let digest = Sha256::digest(
        serde_json::to_vec(&(kind_label, &member_keys, &topic_keys, turn_index))
            .expect("bounded discourse group identity serializes"),
    );
    let mut group = DiscourseGroupIR {
        group_id: format!(
            "DG-{:02X}{:02X}{:02X}{:02X}",
            digest[0], digest[1], digest[2], digest[3]
        ),
        kind,
        member_keys,
        topic_keys,
        revision: 1,
        component_group_ids: Vec::new(),
        membership_sha256: String::new(),
        introduced_turn: turn_index,
        last_referenced_turn: turn_index,
        semantic_authority: false,
        external_execution_authorized: false,
    };
    group.membership_sha256 = discourse_group_membership_sha256(&group);
    state.active_discourse_groups.push(group);
    state.active_discourse_groups.sort_by(|left, right| {
        right
            .last_referenced_turn
            .cmp(&left.last_referenced_turn)
            .then_with(|| right.introduced_turn.cmp(&left.introduced_turn))
            .then_with(|| left.group_id.cmp(&right.group_id))
    });
    state.active_discourse_groups.truncate(MAX_DISCOURSE_GROUPS);
}

fn discourse_group_membership_sha256(group: &DiscourseGroupIR) -> String {
    let bytes = serde_json::to_vec(&(
        "B_CORE_DISCOURSE_GROUP_MEMBERSHIP_1",
        &group.group_id,
        group.kind,
        &group.member_keys,
        &group.topic_keys,
        group.revision,
        &group.component_group_ids,
        group.introduced_turn,
    ))
    .expect("bounded discourse group membership serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn discourse_group_update_sha256(update: &DiscourseGroupUpdateIR) -> String {
    let bytes = serde_json::to_vec(&(
        DISCOURSE_GROUP_UPDATE_SCHEMA,
        update.operation,
        update.applied,
        &update.target_group_id,
        &update.source_group_ids,
        &update.before_member_keys,
        &update.after_member_keys,
        &update.added_member_keys,
        &update.removed_member_keys,
        update.revision,
        &update.unresolved_terms,
        update.semantic_authority,
        update.external_action_executed,
    ))
    .expect("bounded discourse group update serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn normalize_group_member_key(surface: &str) -> String {
    surface.trim().to_lowercase()
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn discourse_topic_anchor_kind(kind: DiscourseGroupKindIR) -> DiscourseTopicAnchorKindIR {
    match kind {
        DiscourseGroupKindIR::Action => DiscourseTopicAnchorKindIR::ActionGroup,
        DiscourseGroupKindIR::AttributedProposition => {
            DiscourseTopicAnchorKindIR::AttributedPropositionGroup
        }
    }
}

fn group_topic_activation_kind(text: &str) -> Option<DiscourseGroupKindIR> {
    let lower = text.to_lowercase();
    let topic = contains_any(&lower, &["topic", "주제", "화제"]);
    let activation = contains_any(
        &lower,
        &[
            "pin",
            "make",
            "keep",
            "set",
            "remember",
            "기억",
            "두자",
            "두어",
            "둘게",
            "삼자",
            "삼아",
            "정하자",
        ],
    );
    (topic && activation)
        .then(|| discourse_group_update_kind(&lower))
        .flatten()
}

fn analyze_group_topic_activation(
    state: &ConversationStateIR,
    text: &str,
    kind: DiscourseGroupKindIR,
) -> TopicTransitionIR {
    let lower = text.to_lowercase();
    let mut groups = state
        .active_discourse_groups
        .iter()
        .filter(|group| group.kind == kind)
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.group_id.cmp(&right.group_id))
    });
    let selected = if contains_any(
        &lower,
        &[
            "combined",
            "merged",
            "결합된",
            "결합한",
            "합친",
            "병합한",
            "병합된",
        ],
    ) {
        let composite = groups
            .iter()
            .copied()
            .filter(|group| !group.component_group_ids.is_empty())
            .collect::<Vec<_>>();
        (composite.len() == 1)
            .then(|| composite.first().copied())
            .flatten()
    } else if contains_any(
        &lower,
        &[
            "first task group",
            "first speaker group",
            "earlier task group",
            "earlier speaker group",
            "첫 작업 묶음",
            "첫 번째 작업 묶음",
            "첫 화자 묶음",
            "첫 번째 화자 묶음",
            "앞 작업 묶음",
            "앞 화자 묶음",
        ],
    ) {
        groups.first().copied()
    } else if contains_any(
        &lower,
        &[
            "second task group",
            "second speaker group",
            "later task group",
            "later speaker group",
            "둘째 작업 묶음",
            "두 번째 작업 묶음",
            "둘째 화자 묶음",
            "두 번째 화자 묶음",
            "뒤 작업 묶음",
            "뒤 화자 묶음",
        ],
    ) {
        groups.get(1).copied()
    } else {
        (groups.len() == 1)
            .then(|| groups.first().copied())
            .flatten()
    };
    let Some(group) = selected else {
        let issue = if groups.is_empty() {
            "DISCOURSE_GROUP_TARGET_UNRESOLVED"
        } else {
            "DISCOURSE_GROUP_TARGET_AMBIGUOUS"
        };
        return unresolved_topic_transition(text, vec![issue.to_string()]);
    };
    let surface = match kind {
        DiscourseGroupKindIR::Action if text_is_english(text) => "task group",
        DiscourseGroupKindIR::Action => "작업 묶음",
        DiscourseGroupKindIR::AttributedProposition if text_is_english(text) => "speaker group",
        DiscourseGroupKindIR::AttributedProposition => "화자 묶음",
    };
    seal_topic_transition(TopicTransitionIR {
        schema: TOPIC_TRANSITION_SCHEMA.to_string(),
        kind: TopicTransitionKindIR::ActivateGroup,
        applied: true,
        history_offset: 0,
        surface: surface.to_string(),
        concept_id_hint: None,
        anchor_kind: discourse_topic_anchor_kind(kind),
        anchor_group_id: Some(group.group_id.clone()),
        anchor_group_revision: Some(group.revision),
        anchor_membership_sha256: Some(group.membership_sha256.clone()),
        unresolved_terms: Vec::new(),
        evidence: vec![
            "DISCOURSE_MANAGEMENT:EXPLICIT_GROUP_TOPIC".to_string(),
            format!("DISCOURSE_GROUP_ID:{}", group.group_id),
            format!("DISCOURSE_GROUP_REVISION:{}", group.revision),
            group.membership_sha256.clone(),
            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            "SEMANTIC_PAYLOAD_MUTATED:false".to_string(),
        ],
        semantic_authority: false,
        external_action_executed: false,
        transition_sha256: String::new(),
    })
}

fn refresh_group_topic_anchors(state: &mut ConversationStateIR) {
    let anchors = state
        .active_discourse_groups
        .iter()
        .map(|group| {
            (
                group.group_id.clone(),
                (
                    discourse_topic_anchor_kind(group.kind),
                    group.revision,
                    group.membership_sha256.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for topic in &mut state.active_topics {
        let Some((kind, revision, membership_sha256)) = topic
            .anchor_group_id
            .as_ref()
            .and_then(|group_id| anchors.get(group_id))
        else {
            continue;
        };
        topic.anchor_kind = *kind;
        topic.anchor_group_revision = Some(*revision);
        topic.anchor_membership_sha256 = Some(membership_sha256.clone());
        topic.topic_sha256 = discourse_topic_sha256(topic);
    }
}

fn analyze_discourse_group_update(
    state: &ConversationStateIR,
    text: &str,
    turn_index: u64,
) -> Option<DiscourseGroupUpdateIR> {
    let kind = discourse_group_update_kind(text)?;
    let operation = discourse_group_update_operation(text)?;
    let mut groups = state
        .active_discourse_groups
        .iter()
        .filter(|group| {
            group.kind == kind
                && state
                    .completed_turns
                    .saturating_sub(group.last_referenced_turn)
                    <= MAX_DISCOURSE_GROUP_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.group_id.cmp(&right.group_id))
    });
    if operation == DiscourseGroupUpdateOperationIR::MergeGroups {
        return Some(analyze_discourse_group_merge(
            kind, text, turn_index, &groups,
        ));
    }
    let target = select_group_update_target(text, &groups);
    let Some(target) = target else {
        return Some(unresolved_discourse_group_update(vec![
            "DISCOURSE_GROUP_TARGET_UNRESOLVED".to_string(),
        ]));
    };
    let member = match kind {
        DiscourseGroupKindIR::Action => action_group_update_member(state, text),
        DiscourseGroupKindIR::AttributedProposition => proposition_group_update_member(state, text),
    };
    let Some(member) = member else {
        return Some(unresolved_discourse_group_update(vec![
            "DISCOURSE_GROUP_MEMBER_UNRESOLVED".to_string(),
        ]));
    };
    let mut after = target.member_keys.clone();
    let (added, removed) = match operation {
        DiscourseGroupUpdateOperationIR::AddMember => {
            if after.contains(&member) {
                return Some(unresolved_discourse_group_update(vec![
                    "DISCOURSE_GROUP_MEMBER_ALREADY_PRESENT".to_string(),
                ]));
            }
            after.push(member.clone());
            (vec![member], Vec::new())
        }
        DiscourseGroupUpdateOperationIR::RemoveMember => {
            if !after.contains(&member) {
                return Some(unresolved_discourse_group_update(vec![
                    "DISCOURSE_GROUP_MEMBER_NOT_PRESENT".to_string(),
                ]));
            }
            after.retain(|candidate| candidate != &member);
            if after.len() < 2 {
                return Some(unresolved_discourse_group_update(vec![
                    "DISCOURSE_GROUP_MINIMUM_CARDINALITY".to_string(),
                ]));
            }
            (Vec::new(), vec![member])
        }
        DiscourseGroupUpdateOperationIR::MergeGroups
        | DiscourseGroupUpdateOperationIR::Unresolved => unreachable!(),
    };
    after.sort();
    after.dedup();
    let mut update = DiscourseGroupUpdateIR {
        schema: DISCOURSE_GROUP_UPDATE_SCHEMA.to_string(),
        operation,
        applied: true,
        target_group_id: Some(target.group_id.clone()),
        source_group_ids: vec![target.group_id.clone()],
        before_member_keys: target.member_keys.clone(),
        after_member_keys: after,
        added_member_keys: added,
        removed_member_keys: removed,
        revision: target.revision + 1,
        unresolved_terms: Vec::new(),
        semantic_authority: false,
        external_action_executed: false,
        update_sha256: String::new(),
    };
    update.update_sha256 = discourse_group_update_sha256(&update);
    debug_assert!(update.validate());
    Some(update)
}

fn discourse_group_update_kind(text: &str) -> Option<DiscourseGroupKindIR> {
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &["speaker group", "speaker groups", "화자 묶음", "화자 그룹"],
    ) {
        Some(DiscourseGroupKindIR::AttributedProposition)
    } else if contains_any(
        &lower,
        &[
            "task group",
            "task groups",
            "action group",
            "action groups",
            "task pair",
            "작업 묶음",
            "작업 그룹",
        ],
    ) {
        Some(DiscourseGroupKindIR::Action)
    } else {
        None
    }
}

fn discourse_group_update_operation(text: &str) -> Option<DiscourseGroupUpdateOperationIR> {
    let lower = text.to_lowercase();
    let candidates = [
        (
            DiscourseGroupUpdateOperationIR::MergeGroups,
            &[
                "combine",
                "merge",
                "결합",
                "합쳐",
                "합치",
                "병합해",
                "병합하",
                "병합시켜",
            ][..],
        ),
        (
            DiscourseGroupUpdateOperationIR::RemoveMember,
            &[
                "remove", "detach", "leave", "take", "drop", " 빼", "제외", "제거",
            ][..],
        ),
        (
            DiscourseGroupUpdateOperationIR::AddMember,
            &[
                "add", "attach", "include", "put", "bring", "추가", "포함", "넣어", "넣으",
            ][..],
        ),
    ];
    candidates.into_iter().find_map(|(operation, markers)| {
        markers.iter().find_map(|marker| {
            if marker.is_ascii() && !contains_ascii_term(&lower, marker) {
                return None;
            }
            lower
                .match_indices(marker)
                .find(|(start, _)| !marker_is_quoted(text, *start))
                .map(|_| operation)
        })
    })
}

fn select_group_update_target<'a>(
    text: &str,
    groups: &[&'a DiscourseGroupIR],
) -> Option<&'a DiscourseGroupIR> {
    if groups.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &[
            "first group",
            "first task group",
            "first speaker group",
            "earlier group",
            "earlier task group",
            "earlier speaker group",
            "아까 그 작업 묶음",
            "첫 번째 묶음",
            "첫 작업 묶음",
            "첫 화자 묶음",
            "첫 번째 화자 묶음",
            "앞 작업 묶음",
            "앞 화자 묶음",
        ],
    ) {
        groups.first().copied()
    } else {
        groups.last().copied()
    }
}

fn action_group_update_member(state: &ConversationStateIR, text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut matches = state
        .action_state_ledger
        .records
        .iter()
        .filter(|record| phrase_mentioned(&lower, &record.subject.to_lowercase()))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .introduced_turn
            .cmp(&left.introduced_turn)
            .then_with(|| left.goal_id.cmp(&right.goal_id))
    });
    let subjects = matches
        .iter()
        .map(|record| record.subject.to_lowercase())
        .collect::<BTreeSet<_>>();
    (subjects.len() == 1).then(|| matches[0].goal_id.clone())
}

fn proposition_group_update_member(state: &ConversationStateIR, text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut matches = state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
        .filter_map(|referent| referent.attributed_source.as_deref())
        .map(normalize_group_member_key)
        .filter(|source| phrase_mentioned(&lower, source))
        .collect::<Vec<_>>();
    matches.sort();
    matches.dedup();
    (matches.len() == 1).then(|| matches[0].clone())
}

fn analyze_discourse_group_merge(
    kind: DiscourseGroupKindIR,
    text: &str,
    turn_index: u64,
    groups: &[&DiscourseGroupIR],
) -> DiscourseGroupUpdateIR {
    if groups.len() < 2 {
        return unresolved_discourse_group_update(vec![
            "DISCOURSE_GROUP_MERGE_SOURCES_UNRESOLVED".to_string()
        ]);
    }
    let lower = text.to_lowercase();
    let selected = if contains_any(
        &lower,
        &[
            "first task pair",
            "first speaker group",
            "first and second",
            "first task group",
            "첫 번째 작업 묶음",
            "첫 번째 화자 묶음",
            "첫 번째와 두 번째",
        ],
    ) && contains_any(&lower, &["second", "두 번째"])
    {
        groups.first().zip(groups.get(1))
    } else if (contains_any(
        &lower,
        &[
            "earlier task group",
            "earlier speaker group",
            "earlier and later",
            "앞 작업 묶음",
            "앞 화자 묶음",
            "앞과 뒤",
        ],
    ) && contains_any(&lower, &["later", "뒤"]))
        || groups.len() == 2
    {
        groups.first().zip(groups.last())
    } else {
        None
    };
    let Some((left, right)) = selected.filter(|(left, right)| left.group_id != right.group_id)
    else {
        return unresolved_discourse_group_update(vec![
            "DISCOURSE_GROUP_MERGE_AMBIGUOUS".to_string()
        ]);
    };
    if left.kind != kind || right.kind != kind {
        return unresolved_discourse_group_update(
            vec!["DISCOURSE_GROUP_KIND_MISMATCH".to_string()],
        );
    }
    let mut after = left
        .member_keys
        .iter()
        .chain(&right.member_keys)
        .cloned()
        .collect::<Vec<_>>();
    after.sort();
    after.dedup();
    if after.len() < 2 || after.len() > MAX_ACTIVE_GOALS {
        return unresolved_discourse_group_update(vec![
            "DISCOURSE_GROUP_MERGE_CARDINALITY".to_string()
        ]);
    }
    let mut source_group_ids = vec![left.group_id.clone(), right.group_id.clone()];
    source_group_ids.sort();
    let digest = Sha256::digest(
        serde_json::to_vec(&(
            "B_CORE_COMPOSITE_DISCOURSE_GROUP_1",
            kind,
            &source_group_ids,
            turn_index,
        ))
        .expect("bounded composite group identity serializes"),
    );
    let target_group_id = format!(
        "DG-{:02X}{:02X}{:02X}{:02X}",
        digest[0], digest[1], digest[2], digest[3]
    );
    let mut update = DiscourseGroupUpdateIR {
        schema: DISCOURSE_GROUP_UPDATE_SCHEMA.to_string(),
        operation: DiscourseGroupUpdateOperationIR::MergeGroups,
        applied: true,
        target_group_id: Some(target_group_id),
        source_group_ids,
        before_member_keys: Vec::new(),
        after_member_keys: after,
        added_member_keys: Vec::new(),
        removed_member_keys: Vec::new(),
        revision: 1,
        unresolved_terms: Vec::new(),
        semantic_authority: false,
        external_action_executed: false,
        update_sha256: String::new(),
    };
    update.update_sha256 = discourse_group_update_sha256(&update);
    debug_assert!(update.validate());
    update
}

fn unresolved_discourse_group_update(mut unresolved_terms: Vec<String>) -> DiscourseGroupUpdateIR {
    unresolved_terms.sort();
    unresolved_terms.dedup();
    let mut update = DiscourseGroupUpdateIR {
        schema: DISCOURSE_GROUP_UPDATE_SCHEMA.to_string(),
        operation: DiscourseGroupUpdateOperationIR::Unresolved,
        applied: false,
        target_group_id: None,
        source_group_ids: Vec::new(),
        before_member_keys: Vec::new(),
        after_member_keys: Vec::new(),
        added_member_keys: Vec::new(),
        removed_member_keys: Vec::new(),
        revision: 0,
        unresolved_terms,
        semantic_authority: false,
        external_action_executed: false,
        update_sha256: String::new(),
    };
    update.update_sha256 = discourse_group_update_sha256(&update);
    debug_assert!(update.validate());
    update
}

fn discourse_group_topics(
    state: &ConversationStateIR,
    kind: DiscourseGroupKindIR,
    members: &[String],
) -> Vec<String> {
    let mut topics = match kind {
        DiscourseGroupKindIR::Action => state
            .action_state_ledger
            .records
            .iter()
            .filter(|record| members.contains(&record.goal_id))
            .flat_map(|record| topic_keys_from_text(&record.subject))
            .collect::<Vec<_>>(),
        DiscourseGroupKindIR::AttributedProposition => state
            .active_discourse_referents
            .iter()
            .filter(|referent| {
                referent.kind == DiscourseReferentKindIR::Proposition
                    && referent
                        .attributed_source
                        .as_deref()
                        .is_some_and(|source| members.contains(&normalize_group_member_key(source)))
            })
            .flat_map(|referent| topic_keys_from_text(&referent.semantic_summary))
            .collect::<Vec<_>>(),
    };
    topics.sort();
    topics.dedup();
    topics
}

fn topic_keys_from_text(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut keys = [
        ("TOPIC_CACHE", "캐시", "cache"),
        ("TOPIC_QUEUE", "큐", "queue"),
        ("TOPIC_BACKUP", "백업", "backup"),
        ("TOPIC_LOG", "로그", "log"),
        ("TOPIC_SERVER", "서버", "server"),
        ("TOPIC_WORKER", "워커", "worker"),
        ("C_OBJECT_FILE", "파일", "file"),
        ("C_OBJECT_FOLDER", "폴더", "folder"),
        ("C_OBJECT_SOURCE_CODE", "코드", "code"),
        ("C_OBJECT_DOCUMENT", "문서", "document"),
        ("C_OBJECT_REPORT", "보고서", "report"),
        ("C_OBJECT_PROJECT", "프로젝트", "project"),
        ("C_OBJECT_REPOSITORY", "저장소", "repository"),
    ]
    .into_iter()
    .filter_map(|(concept, korean, english)| {
        (lower.contains(korean) || contains_ascii_term(&lower, english))
            .then_some(concept.to_string())
    })
    .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn shared_question_focus(question: &QuestionUnderDiscussionIR) -> Option<(String, String)> {
    let mut option_keys = question
        .options
        .iter()
        .map(|option| {
            topic_keys_from_text(&option.resolved_semantic_text)
                .into_iter()
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let first = option_keys.pop()?;
    let shared = option_keys.into_iter().fold(first, |shared, keys| {
        shared.intersection(&keys).cloned().collect()
    });
    let mut shared = shared.into_iter();
    let concept_id = shared.next()?;
    if shared.next().is_some() {
        return None;
    }
    let english = !question
        .source_request
        .chars()
        .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character));
    let surface = topic_surface(&concept_id, english)?.to_string();
    Some((surface, concept_id))
}

fn empty_state(conversation_id: &str) -> ConversationStateIR {
    let mut state = ConversationStateIR {
        schema: CONVERSATION_STATE_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        completed_turns: 0,
        active_subject: None,
        answer_focus: None,
        dialogue_world: Default::default(),
        active_referents: Vec::new(),
        active_topics: Vec::new(),
        discourse_focus: DiscourseFocusStateIR::default(),
        topic_context_graph: TopicContextGraphIR::default(),
        active_goals: Vec::new(),
        active_discourse_programs: Vec::new(),
        action_state_ledger: ActionStateLedgerIR::default(),
        deferred_action_commitments: Vec::new(),
        active_discourse_referents: Vec::new(),
        active_discourse_groups: Vec::new(),
        active_typed_entities: Vec::new(),
        epistemic_ledger: EpistemicLedgerIR::default(),
        temporal_graph: TemporalGraphIR::default(),
        conditional_guard_store: ConditionalGuardStoreIR::default(),
        dialogue_relation_graph: DialogueRelationGraphIR::default(),
        dialogue_directive_ledger: DialogueDirectiveLedgerIR::default(),
        last_guard_evaluations: Vec::new(),
        preferred_language: None,
        pending_question: None,
        topic_pending_questions: Vec::new(),
        unresolved_reference_count: 0,
        state_sha256: String::new(),
    };
    state.state_sha256 = state_hash(&state).expect("empty state serializes");
    state
}

fn state_hash(state: &ConversationStateIR) -> Result<String, ConversationFrontendError> {
    let bytes = serde_json::to_vec(&(
        &state.schema,
        &state.conversation_id,
        state.completed_turns,
        &state.active_subject,
        (
            &state.active_referents,
            &state.active_topics,
            &state.discourse_focus,
            &state.topic_context_graph,
        ),
        (
            &state.active_goals,
            &state.active_discourse_programs,
            &state.action_state_ledger,
            &state.deferred_action_commitments,
        ),
        (
            &state.active_discourse_referents,
            &state.active_discourse_groups,
            &state.active_typed_entities,
        ),
        &state.epistemic_ledger,
        &state.temporal_graph,
        &state.conditional_guard_store,
        &state.dialogue_relation_graph,
        &state.dialogue_directive_ledger,
        &state.last_guard_evaluations,
        state.preferred_language,
        (
            &state.pending_question,
            &state.topic_pending_questions,
            &state.answer_focus,
            &state.dialogue_world,
        ),
        state.unresolved_reference_count,
    ))
    .map_err(|_| ConversationFrontendError::InvalidState)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn guarded_program_lifecycle_links_valid(state: &ConversationStateIR) -> bool {
    let commitments = state
        .deferred_action_commitments
        .iter()
        .map(|commitment| (commitment.commitment_id.as_str(), commitment))
        .collect::<BTreeMap<_, _>>();
    let mut linked_commitment_ids = BTreeSet::new();
    for program in &state.active_discourse_programs {
        for step in &program.steps {
            let Some(guard) = step.guard.as_ref() else {
                continue;
            };
            if !linked_commitment_ids.insert(guard.deferred_commitment_id.as_str()) {
                return false;
            }
            let Some(commitment) = commitments.get(guard.deferred_commitment_id.as_str()) else {
                return false;
            };
            if guard.condition_sha256 != commitment.condition_sha256
                || guard.normalized_antecedent != commitment.normalized_condition
                || !guard
                    .source_subject
                    .eq_ignore_ascii_case(&commitment.action.subject)
                || step.goal.intent != commitment.action.intent
                || step.goal.canonical_predicate != commitment.action.canonical_predicate
                || !step
                    .goal
                    .subject
                    .eq_ignore_ascii_case(&commitment.action.subject)
                || step.goal.source_semantic_text != commitment.action.source_semantic_text
                || commitment.introduced_turn != program.introduced_turn
                || commitment.introduced_turn != step.goal.introduced_turn
            {
                return false;
            }
        }
    }
    state
        .active_goals
        .iter()
        .filter(|goal| goal.goal_id.starts_with("GOAL-D-"))
        .all(|goal| {
            state
                .deferred_action_commitments
                .iter()
                .find(|commitment| {
                    commitment.status == DeferredCommitmentStatusIR::Activated
                        && commitment.activated_goal_id.as_deref() == Some(goal.goal_id.as_str())
                })
                .is_some_and(|commitment| {
                    goal.intent == commitment.action.intent
                        && goal.canonical_predicate == commitment.action.canonical_predicate
                        && goal
                            .subject
                            .eq_ignore_ascii_case(&commitment.action.subject)
                        && goal.source_semantic_text == commitment.action.source_semantic_text
                        && goal.external_execution_authorized
                })
        })
}

fn question_under_discussion_valid(
    question: &QuestionUnderDiscussionIR,
    completed_turns: u64,
) -> bool {
    !question.question_id.trim().is_empty()
        && question
            .topic_id
            .as_deref()
            .is_none_or(|topic_id| !topic_id.trim().is_empty())
        && question.source_turn > 0
        && question.source_turn <= completed_turns
        && !question.source_request.trim().is_empty()
        && (2..=MAX_ALTERNATIVES).contains(&question.options.len())
        && question
            .options
            .iter()
            .map(|option| &option.option_id)
            .collect::<BTreeSet<_>>()
            .len()
            == question.options.len()
        && question.options.iter().all(|option| {
            !option.option_id.trim().is_empty()
                && !option.display_surface.trim().is_empty()
                && !option.resolved_semantic_text.trim().is_empty()
        })
}

pub fn validate_conversation_state(
    state: &ConversationStateIR,
) -> Result<(), ConversationFrontendError> {
    let unique_referents = state
        .active_referents
        .iter()
        .map(|referent| &referent.referent_id)
        .collect::<BTreeSet<_>>();
    let unique_discourse_referents = state
        .active_discourse_referents
        .iter()
        .map(|referent| &referent.referent_id)
        .collect::<BTreeSet<_>>();
    let unique_topics = state
        .active_topics
        .iter()
        .map(|topic| &topic.topic_id)
        .collect::<BTreeSet<_>>();
    let topic_context_ids = state
        .topic_context_graph
        .contexts
        .iter()
        .map(|context| context.topic_id.as_str())
        .collect::<BTreeSet<_>>();
    let focus_ids = state
        .discourse_focus
        .nodes
        .iter()
        .map(|node| node.focus_id.as_str())
        .collect::<BTreeSet<_>>();
    let unique_typed_entities = state
        .active_typed_entities
        .iter()
        .map(|referent| &referent.entity_id)
        .collect::<BTreeSet<_>>();
    let unique_discourse_groups = state
        .active_discourse_groups
        .iter()
        .map(|group| &group.group_id)
        .collect::<BTreeSet<_>>();
    let unique_deferred_commitments = state
        .deferred_action_commitments
        .iter()
        .map(|commitment| &commitment.commitment_id)
        .collect::<BTreeSet<_>>();
    let unique_deferred_evidence = state
        .deferred_action_commitments
        .iter()
        .flat_map(|commitment| commitment.evidence_ids.iter())
        .collect::<BTreeSet<_>>();
    let unique_topic_question_ids = state
        .topic_pending_questions
        .iter()
        .map(|question| &question.question_id)
        .collect::<BTreeSet<_>>();
    let unique_topic_question_topics = state
        .topic_pending_questions
        .iter()
        .filter_map(|question| question.topic_id.as_ref())
        .collect::<BTreeSet<_>>();
    let deferred_evidence_count = state
        .deferred_action_commitments
        .iter()
        .map(|commitment| commitment.evidence_ids.len())
        .sum::<usize>();
    if state.schema != CONVERSATION_STATE_SCHEMA
        || !state.dialogue_world.validate(state.completed_turns)
        || state
            .answer_focus
            .as_ref()
            .is_some_and(|focus| !focus.validate(state.completed_turns))
        || state.conversation_id.trim().is_empty()
        || state.active_referents.len() > MAX_ACTIVE_REFERENTS
        || unique_referents.len() != state.active_referents.len()
        || state.active_referents.iter().any(|referent| {
            referent.referent_id.trim().is_empty()
                || referent.surface.trim().is_empty()
                || referent.canonical_concept.trim().is_empty()
                || referent.introduced_turn == 0
                || referent.last_referenced_turn < referent.introduced_turn
                || referent.last_referenced_turn > state.completed_turns
        })
        || state.active_topics.len() > MAX_ACTIVE_TOPICS
        || unique_topics.len() != state.active_topics.len()
        || state.active_topics.iter().any(|topic| {
            let anchor_valid = match topic.anchor_kind {
                DiscourseTopicAnchorKindIR::Surface => {
                    topic.anchor_group_id.is_none()
                        && topic.anchor_group_revision.is_none()
                        && topic.anchor_membership_sha256.is_none()
                }
                DiscourseTopicAnchorKindIR::Concept => {
                    topic.concept_id_hint.is_some()
                        && topic.anchor_group_id.is_none()
                        && topic.anchor_group_revision.is_none()
                        && topic.anchor_membership_sha256.is_none()
                }
                DiscourseTopicAnchorKindIR::ActionGroup
                | DiscourseTopicAnchorKindIR::AttributedPropositionGroup => topic
                    .anchor_group_id
                    .as_deref()
                    .and_then(|group_id| {
                        state
                            .active_discourse_groups
                            .iter()
                            .find(|group| group.group_id == group_id)
                    })
                    .is_some_and(|group| {
                        discourse_topic_anchor_kind(group.kind) == topic.anchor_kind
                            && topic.anchor_group_revision == Some(group.revision)
                            && topic.anchor_membership_sha256.as_deref()
                                == Some(group.membership_sha256.as_str())
                    }),
            };
            topic.topic_id.trim().is_empty()
                || topic.surface.trim().is_empty()
                || topic.introduced_turn == 0
                || topic.last_activated_turn < topic.introduced_turn
                || topic.last_activated_turn > state.completed_turns
                || topic
                    .concept_id_hint
                    .as_deref()
                    .is_some_and(|concept| concept.trim().is_empty())
                || !anchor_valid
                || topic.semantic_authority
                || topic.external_execution_authorized
                || topic.topic_sha256.len() != 64
                || !topic
                    .topic_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || topic.topic_sha256 != discourse_topic_sha256(topic)
        })
        || !state.discourse_focus.validate(state.completed_turns)
        || !state.topic_context_graph.validate(state.completed_turns)
        || state.topic_context_graph.active_topic_id.as_deref()
            != state
                .active_topics
                .first()
                .map(|topic| topic.topic_id.as_str())
        || topic_context_ids
            != state
                .active_topics
                .iter()
                .map(|topic| topic.topic_id.as_str())
                .collect::<BTreeSet<_>>()
        || state.topic_context_graph.contexts.iter().any(|context| {
            context
                .current_focus_id
                .as_deref()
                .is_some_and(|focus_id| !focus_ids.contains(focus_id))
                || context
                    .pending_question_id
                    .as_deref()
                    .is_some_and(|question_id| {
                        !state
                            .topic_pending_questions
                            .iter()
                            .any(|question| question.question_id == question_id)
                    })
                || context.discourse_referent_ids.iter().any(|referent_id| {
                    !state
                        .active_discourse_referents
                        .iter()
                        .any(|referent| referent.referent_id == *referent_id)
                })
        })
        || state.topic_context_graph.active().is_some_and(|context| {
            context.current_focus_id.as_deref() != state.discourse_focus.current_focus_id.as_deref()
        })
        || state.active_goals.len() > MAX_ACTIVE_GOALS
        || state.active_goals.iter().any(|goal| {
            goal.goal_id.trim().is_empty()
                || goal.canonical_predicate.trim().is_empty()
                || goal.source_semantic_text.trim().is_empty()
                || goal.introduced_turn == 0
                || goal.last_referenced_turn < goal.introduced_turn
                || goal.last_referenced_turn > state.completed_turns
        })
        || state
            .active_goals
            .iter()
            .map(|goal| &goal.goal_id)
            .collect::<BTreeSet<_>>()
            .len()
            != state.active_goals.len()
        || state.active_discourse_programs.len() > MAX_ACTIVE_DISCOURSE_PROGRAMS
        || state
            .active_discourse_programs
            .iter()
            .any(|program| !program.validate(state.completed_turns))
        || state
            .active_discourse_programs
            .iter()
            .map(|program| &program.program_id)
            .collect::<BTreeSet<_>>()
            .len()
            != state.active_discourse_programs.len()
        || !guarded_program_lifecycle_links_valid(state)
        || !state.action_state_ledger.validate(state.completed_turns)
        || state.deferred_action_commitments.len() > MAX_DEFERRED_COMMITMENTS
        || unique_deferred_commitments.len() != state.deferred_action_commitments.len()
        || unique_deferred_evidence.len() != deferred_evidence_count
        || state
            .deferred_action_commitments
            .iter()
            .any(|commitment| !commitment.validate(state.completed_turns))
        || state.active_discourse_referents.len() > MAX_DISCOURSE_REFERENTS
        || unique_discourse_referents.len() != state.active_discourse_referents.len()
        || state.active_discourse_groups.len() > MAX_DISCOURSE_GROUPS
        || unique_discourse_groups.len() != state.active_discourse_groups.len()
        || state.active_discourse_groups.iter().any(|group| {
            group.group_id.trim().is_empty()
                || group.member_keys.len() < 2
                || group.member_keys.len() > MAX_ACTIVE_GOALS
                || group
                    .member_keys
                    .iter()
                    .any(|member| member.trim().is_empty())
                || group.member_keys.iter().collect::<BTreeSet<_>>().len()
                    != group.member_keys.len()
                || group.revision == 0
                || group.component_group_ids.len() > MAX_DISCOURSE_GROUPS
                || group
                    .component_group_ids
                    .iter()
                    .any(|component| component.trim().is_empty() || component == &group.group_id)
                || group.component_group_ids.iter().any(|component| {
                    !state
                        .active_discourse_groups
                        .iter()
                        .any(|candidate| &candidate.group_id == component)
                })
                || group
                    .component_group_ids
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != group.component_group_ids.len()
                || (!group.component_group_ids.is_empty() && group.component_group_ids.len() < 2)
                || group.membership_sha256.len() != 64
                || !group
                    .membership_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || group.membership_sha256 != discourse_group_membership_sha256(group)
                || group.topic_keys.len() > MAX_ACTIVE_TOPICS
                || group.topic_keys.iter().any(|topic| topic.trim().is_empty())
                || group.topic_keys.iter().collect::<BTreeSet<_>>().len() != group.topic_keys.len()
                || group
                    .topic_keys
                    .windows(2)
                    .any(|window| window[0] >= window[1])
                || group.introduced_turn == 0
                || group.last_referenced_turn < group.introduced_turn
                || group.last_referenced_turn > state.completed_turns
                || group.semantic_authority
                || group.external_execution_authorized
        })
        || state.active_typed_entities.len() > MAX_TYPED_ENTITY_REFERENTS
        || unique_typed_entities.len() != state.active_typed_entities.len()
        || state
            .active_typed_entities
            .iter()
            .any(|referent| !referent.validate(state.completed_turns))
        || state.active_discourse_referents.iter().any(|referent| {
            referent.referent_id.trim().is_empty()
                || referent.semantic_summary.trim().is_empty()
                || referent
                    .topic_id
                    .as_deref()
                    .is_some_and(|topic_id| topic_id.trim().is_empty())
                || referent
                    .attributed_source
                    .as_deref()
                    .is_some_and(|source| source.trim().is_empty())
                || referent.introduced_turn == 0
                || referent.last_referenced_turn < referent.introduced_turn
                || referent.last_referenced_turn > state.completed_turns
                || (referent.kind != DiscourseReferentKindIR::Event
                    && referent.external_execution_authorized)
                || (referent.kind != DiscourseReferentKindIR::Proposition
                    && (referent.attributed_source.is_some()
                        || referent.attribution_attitude.is_some()
                        || referent.epistemic_status.is_some()
                        || referent.proposition_polarity.is_some()
                        || referent.modal_world.is_some()
                        || referent.belief_record_id.is_some()))
                || (referent.attribution_attitude.is_some() != referent.epistemic_status.is_some())
                || (referent.kind == DiscourseReferentKindIR::Proposition
                    && (referent.proposition_polarity.is_none()
                        || referent.modal_world.is_none()
                        || referent
                            .belief_record_id
                            .as_deref()
                            .is_none_or(|belief_id| {
                                state
                                    .epistemic_ledger
                                    .record(belief_id)
                                    .is_none_or(|record| {
                                        record.origin_referent_id != referent.referent_id
                                            || record.source_actor
                                                != referent
                                                    .attributed_source
                                                    .as_deref()
                                                    .unwrap_or("DIALOGUE_USER")
                                            || record.proposition_surface
                                                != referent.semantic_summary
                                            || Some(record.proposition_polarity)
                                                != referent.proposition_polarity
                                            || Some(record.signature.modal_world)
                                                != referent.modal_world
                                            || record.attribution_attitude
                                                != referent
                                                    .attribution_attitude
                                                    .unwrap_or(AttributionAttitudeIR::Say)
                                            || record.epistemic_status
                                                != referent
                                                    .epistemic_status
                                                    .unwrap_or(EpistemicStatusIR::Reported)
                                            || !record.status.is_reference_active()
                                    })
                            })))
        })
        || !state.epistemic_ledger.validate(state.completed_turns)
        || !state.temporal_graph.validate(state.completed_turns)
        || !state
            .conditional_guard_store
            .validate(state.completed_turns, &state.epistemic_ledger)
        || !state
            .dialogue_relation_graph
            .validate_with_ledger(state.completed_turns, &state.epistemic_ledger)
        || !state
            .dialogue_directive_ledger
            .validate(state.completed_turns)
        || state.last_guard_evaluations.len() > MAX_ACTIVE_GOALS * 4
        || state.last_guard_evaluations.iter().any(|evaluation| {
            evaluation.evaluation_turn > state.completed_turns
                || !evaluation.validate(&state.conditional_guard_store, &state.epistemic_ledger)
        })
        || state.pending_question.as_ref().is_some_and(|question| {
            !question_under_discussion_valid(question, state.completed_turns)
        })
        || state.topic_pending_questions.len() > MAX_TOPIC_PENDING_QUESTIONS
        || unique_topic_question_ids.len() != state.topic_pending_questions.len()
        || unique_topic_question_topics.len() != state.topic_pending_questions.len()
        || state.topic_pending_questions.iter().any(|question| {
            !question_under_discussion_valid(question, state.completed_turns)
                || question.topic_id.as_deref().is_none_or(|topic_id| {
                    !state
                        .active_topics
                        .iter()
                        .any(|topic| topic.topic_id == topic_id)
                })
        })
        || state.pending_question.as_ref().is_some_and(|question| {
            question.topic_id.as_deref().is_some_and(|topic_id| {
                state
                    .active_topics
                    .first()
                    .map(|topic| topic.topic_id.as_str())
                    != Some(topic_id)
                    || !state
                        .topic_pending_questions
                        .iter()
                        .any(|candidate| candidate == question)
            })
        })
        || state.state_sha256.len() != 64
        || state.state_sha256 != state_hash(state)?
    {
        return Err(ConversationFrontendError::InvalidState);
    }
    Ok(())
}

#[derive(Debug)]
struct TypedDiscourseResolution {
    resolved_text: String,
    binding: Option<DiscourseBindingIR>,
    ambiguous_surfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum EventSequenceSelector {
    Position(usize),
    Last,
    AfterLast,
}

#[derive(Debug)]
struct EventSequenceReference {
    marker: String,
    selector: EventSequenceSelector,
}

fn resolve_event_sequence_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    let reference = event_sequence_reference(text)?;
    let latest_turn = state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Event)
        .map(|referent| referent.introduced_turn)
        .max();
    let mut events = state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Event)
        .filter(|referent| Some(referent.introduced_turn) == latest_turn)
        .collect::<Vec<_>>();
    events.sort_by(|left, right| left.referent_id.cmp(&right.referent_id));
    let selected_index = match reference.selector {
        EventSequenceSelector::Position(index) => Some(index),
        EventSequenceSelector::Last => events.len().checked_sub(1),
        EventSequenceSelector::AfterLast => None,
    };
    let Some(index) = selected_index.filter(|index| *index < events.len()) else {
        return Some(ReferenceResolutionIR {
            original_semantic_text: text.to_string(),
            resolved_semantic_text: text.to_string(),
            resolved_reference_count: 0,
            used_referent_ids: Vec::new(),
            ambiguous_reference_surfaces: vec!["EVENT_SEQUENCE_ORDINAL".to_string()],
            topic_anchored_resolution: None,
            resolution_graph: ReferenceResolutionGraphIR::default(),
            discourse_bindings: Vec::new(),
        });
    };
    let referent = events[index];
    let mut goals = state
        .active_goals
        .iter()
        .filter(|goal| goal.introduced_turn == referent.introduced_turn)
        .collect::<Vec<_>>();
    goals.sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
    let goal = goals.get(index).copied();
    let summary = goal.map_or_else(
        || referent.semantic_summary.clone(),
        |goal| repeat_goal_in_current_language(goal, text),
    );
    let replacement = if text_is_english(text) {
        format!("the action ‘{summary}’")
    } else {
        format!("‘{summary}’라는 작업")
    };
    let resolved_text = replace_first_case_insensitive(text, &reference.marker, &replacement);
    Some(ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: resolved_text,
        resolved_reference_count: 1,
        used_referent_ids: vec![referent.referent_id.clone()],
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::EventOrdinalReference,
            source_surface: reference.marker,
            resolved_surface: replacement,
            referent_ids: vec![referent.referent_id.clone()],
            inherited_goal_id: goal.map(|goal| goal.goal_id.clone()),
            confidence_millis: 970,
            evidence: vec![
                format!("EVENT_SEQUENCE_POSITION:{}", index + 1),
                format!("EVENT_SEQUENCE_SIZE:{}", events.len()),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventGroupSelector {
    ExactlyTwo,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscourseGroupSelection {
    Newest,
    Oldest,
    Ordinal(usize),
    ActiveTopic,
    SurfaceTopics,
    ExplicitlyAmbiguous,
}

enum DiscourseGroupLookup<'a> {
    Selected(&'a DiscourseGroupIR),
    Ambiguous,
    None,
}

#[derive(Debug)]
struct EventGroupReference {
    marker: String,
    selector: EventGroupSelector,
    group_selection: DiscourseGroupSelection,
}

fn resolve_event_group_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    let reference = event_group_reference(text)?;
    match persistent_action_group_members(
        state,
        reference.selector,
        reference.group_selection,
        text,
    ) {
        ActionGroupLookup::Selected(members) => {
            return Some(build_event_group_resolution(
                text,
                &reference.marker,
                &members,
                "PERSISTENT_DISCOURSE_GROUP",
            ));
        }
        ActionGroupLookup::Ambiguous => {
            return Some(ambiguous_group_resolution(
                text,
                "ACTION_GROUP_REFERENCE",
                &reference.marker,
            ));
        }
        ActionGroupLookup::None => {}
    }
    let goals = state
        .active_goals
        .iter()
        .filter(|goal| {
            state
                .completed_turns
                .saturating_sub(goal.last_referenced_turn)
                <= MAX_GOAL_ELLIPSIS_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    let required_size = match reference.selector {
        EventGroupSelector::ExactlyTwo => 2,
        EventGroupSelector::All => goals.len(),
    };
    let valid_cardinality = match reference.selector {
        EventGroupSelector::ExactlyTwo => goals.len() == 2,
        EventGroupSelector::All => goals.len() >= 2,
    };
    if !valid_cardinality {
        return Some(ambiguous_group_resolution(
            text,
            "ACTION_GROUP_REFERENCE",
            &reference.marker,
        ));
    }
    let members = goals
        .iter()
        .filter_map(|goal| {
            let suffix = goal.goal_id.strip_prefix("GOAL-")?;
            let referent_id = format!("DREF-E-{suffix}");
            state
                .active_discourse_referents
                .iter()
                .find(|referent| {
                    referent.kind == DiscourseReferentKindIR::Event
                        && referent.referent_id == referent_id
                })
                .map(|referent| (goal.goal_id.as_str(), referent))
        })
        .collect::<Vec<_>>();
    if members.len() != required_size {
        return Some(ambiguous_group_resolution(
            text,
            "ACTION_GROUP_REFERENCE",
            &reference.marker,
        ));
    }
    Some(build_event_group_resolution(
        text,
        &reference.marker,
        &members,
        "ACTIVE_GOAL_SET",
    ))
}

enum ActionGroupLookup<'a> {
    Selected(Vec<(&'a str, &'a DynamicDiscourseReferentIR)>),
    Ambiguous,
    None,
}

fn persistent_action_group_members<'a>(
    state: &'a ConversationStateIR,
    selector: EventGroupSelector,
    mut selection: DiscourseGroupSelection,
    text: &str,
) -> ActionGroupLookup<'a> {
    if selection == DiscourseGroupSelection::Newest
        && state.active_topics.first().is_some_and(|topic| {
            topic.anchor_kind == DiscourseTopicAnchorKindIR::ActionGroup
                && topic.anchor_group_id.is_some()
        })
    {
        selection = DiscourseGroupSelection::ActiveTopic;
    }
    let lookup = select_discourse_group(
        state,
        DiscourseGroupKindIR::Action,
        selection,
        text,
        |group| match selector {
            EventGroupSelector::ExactlyTwo => group.member_keys.len() == 2,
            EventGroupSelector::All => group.member_keys.len() >= 2,
        },
    );
    let DiscourseGroupLookup::Selected(group) = lookup else {
        return match lookup {
            DiscourseGroupLookup::Ambiguous => ActionGroupLookup::Ambiguous,
            DiscourseGroupLookup::None => ActionGroupLookup::None,
            DiscourseGroupLookup::Selected(_) => unreachable!(),
        };
    };
    let members = group
        .member_keys
        .iter()
        .filter_map(|goal_id| {
            let action = state
                .action_state_ledger
                .records
                .iter()
                .find(|record| record.goal_id == *goal_id)?;
            let suffix = action.goal_id.strip_prefix("GOAL-")?;
            let referent_id = format!("DREF-E-{suffix}");
            let referent = state.active_discourse_referents.iter().find(|referent| {
                referent.kind == DiscourseReferentKindIR::Event
                    && referent.referent_id == referent_id
            })?;
            Some((action.goal_id.as_str(), referent))
        })
        .collect::<Vec<_>>();
    if members.len() == group.member_keys.len() {
        ActionGroupLookup::Selected(members)
    } else {
        ActionGroupLookup::None
    }
}

fn select_discourse_group<'a>(
    state: &'a ConversationStateIR,
    kind: DiscourseGroupKindIR,
    selection: DiscourseGroupSelection,
    text: &str,
    cardinality: impl Fn(&DiscourseGroupIR) -> bool,
) -> DiscourseGroupLookup<'a> {
    if selection == DiscourseGroupSelection::ActiveTopic {
        if let Some(group_id) = state
            .active_topics
            .first()
            .and_then(|topic| topic.anchor_group_id.as_deref())
        {
            return state
                .active_discourse_groups
                .iter()
                .find(|group| {
                    group.group_id == group_id && group.kind == kind && cardinality(group)
                })
                .map_or(DiscourseGroupLookup::None, DiscourseGroupLookup::Selected);
        }
    }
    let mut groups = state
        .active_discourse_groups
        .iter()
        .filter(|group| {
            group.kind == kind
                && state
                    .completed_turns
                    .saturating_sub(group.last_referenced_turn)
                    <= MAX_DISCOURSE_GROUP_TURN_DISTANCE
                && cardinality(group)
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.group_id.cmp(&right.group_id))
    });
    if groups.is_empty() {
        return DiscourseGroupLookup::None;
    }
    let narrowed = match selection {
        DiscourseGroupSelection::Newest => {
            return DiscourseGroupLookup::Selected(groups[groups.len() - 1])
        }
        DiscourseGroupSelection::Oldest => return DiscourseGroupLookup::Selected(groups[0]),
        DiscourseGroupSelection::Ordinal(index) => {
            return groups.get(index).copied().map_or(
                DiscourseGroupLookup::Ambiguous,
                DiscourseGroupLookup::Selected,
            );
        }
        DiscourseGroupSelection::ActiveTopic => {
            let topics = state
                .active_topics
                .first()
                .map(|topic| {
                    topic
                        .concept_id_hint
                        .clone()
                        .into_iter()
                        .chain(topic_keys_from_text(&topic.surface))
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            groups
                .into_iter()
                .filter(|group| group.topic_keys.iter().any(|topic| topics.contains(topic)))
                .collect::<Vec<_>>()
        }
        DiscourseGroupSelection::SurfaceTopics => {
            let topics = topic_keys_from_text(text);
            groups
                .into_iter()
                .filter(|group| {
                    !topics.is_empty()
                        && topics.iter().all(|topic| group.topic_keys.contains(topic))
                })
                .collect::<Vec<_>>()
        }
        DiscourseGroupSelection::ExplicitlyAmbiguous => groups,
    };
    match narrowed.as_slice() {
        [group] => DiscourseGroupLookup::Selected(group),
        [] => DiscourseGroupLookup::None,
        _ => DiscourseGroupLookup::Ambiguous,
    }
}

fn ambiguous_group_resolution(text: &str, kind: &str, marker: &str) -> ReferenceResolutionIR {
    ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: text.to_string(),
        resolved_reference_count: 0,
        used_referent_ids: Vec::new(),
        ambiguous_reference_surfaces: vec![format!("{kind}:{marker}")],
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: Vec::new(),
    }
}

fn build_event_group_resolution(
    text: &str,
    marker: &str,
    members: &[(&str, &DynamicDiscourseReferentIR)],
    group_source: &str,
) -> ReferenceResolutionIR {
    let english = text_is_english(text);
    let summaries = members
        .iter()
        .map(|(_, referent)| {
            let summary = referent
                .semantic_summary
                .trim_matches(|character| matches!(character, '‘' | '’' | '“' | '”' | '"'));
            format!("‘{summary}’")
        })
        .collect::<Vec<_>>();
    let replacement = if english {
        format!("the actions {}", summaries.join(" and "))
    } else {
        format!("{} 작업들", summaries.join(", "))
    };
    let referent_ids = members
        .iter()
        .map(|(_, referent)| referent.referent_id.clone())
        .collect::<Vec<_>>();
    let mut bindings = vec![DiscourseBindingIR {
        kind: DiscourseBindingKindIR::PluralEventReference,
        source_surface: marker.to_string(),
        resolved_surface: replacement.clone(),
        referent_ids: referent_ids.clone(),
        inherited_goal_id: None,
        confidence_millis: 960,
        evidence: vec![
            format!("ACTION_GROUP_CARDINALITY:{}", members.len()),
            format!("GROUP_SOURCE:{group_source}"),
            "SEMANTIC_AUTHORITY:false".to_string(),
            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
        ],
    }];
    bindings.extend(
        members
            .iter()
            .map(|(goal_id, referent)| DiscourseBindingIR {
                kind: DiscourseBindingKindIR::PluralEventMemberReference,
                source_surface: marker.to_string(),
                resolved_surface: referent.semantic_summary.clone(),
                referent_ids: vec![referent.referent_id.clone()],
                inherited_goal_id: Some((*goal_id).to_string()),
                confidence_millis: 960,
                evidence: vec![
                    "GROUP_MEMBERSHIP:TYPED_ACTION_LEDGER".to_string(),
                    "SEMANTIC_AUTHORITY:false".to_string(),
                    "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                ],
            }),
    );
    ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: replace_first_case_insensitive(text, marker, &replacement),
        resolved_reference_count: 1,
        used_referent_ids: referent_ids,
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: bindings,
    }
}

fn event_group_reference(text: &str) -> Option<EventGroupReference> {
    let lower = text.to_lowercase();
    let markers = [
        (
            "they both",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "that combined task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the combined task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the merged task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "that task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the current task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "current task group",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "합친 작업 묶음",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "병합한 작업 묶음",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "그 작업 묶음",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "현재 작업 묶음",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the first pair of tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "the first task pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "the second pair of tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(1),
        ),
        (
            "the second task pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(1),
        ),
        (
            "the earliest task pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the most recent task pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "this topic's task pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "the task pair for this topic",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "jobs linked to this topic",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "the earlier pair of tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the earlier pair of actions",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "that earlier pair",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "those two actions",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "those two tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "those two jobs",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "those results",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "those outputs",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both results",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both outcomes",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "that pair of tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the pair of tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the two actions",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "the two tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both of those actions",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both of those tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both actions",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both tasks",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "both jobs",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "all of the actions",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "all of the tasks",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "all actions",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "all tasks",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "every action",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "every task",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "첫 번째 작업 묶음",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "두 번째 작업 묶음",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Ordinal(1),
        ),
        (
            "먼저 묶은 작업 둘",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "나중 작업 둘",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "이 주제의 작업 묶음",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "현재 주제에 속한 작업 둘",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "이 주제와 연결된 작업 두 건",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "아까 그 두 작업",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "앞서 말한 두 건",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Oldest,
        ),
        (
            "그 두 작업",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "그 작업 둘",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 작업 모두",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 작업 다",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 작업의",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 작업",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "양쪽 작업",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 건",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "그 결과들",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "두 결과",
            EventGroupSelector::ExactlyTwo,
            DiscourseGroupSelection::Newest,
        ),
        (
            "모든 작업",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "작업 전부",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "전체 작업",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
        (
            "작업 전체",
            EventGroupSelector::All,
            DiscourseGroupSelection::Newest,
        ),
    ];
    if let Some((marker, selector, group_selection)) = markers.into_iter().find(|(marker, _, _)| {
        lower
            .find(marker)
            .is_some_and(|start| !marker_is_quoted(text, start))
    }) {
        return Some(EventGroupReference {
            marker: marker.to_string(),
            selector,
            group_selection,
        });
    }
    for (marker, group_selection) in [
        ("the earlier pair", DiscourseGroupSelection::Oldest),
        ("the pair", DiscourseGroupSelection::Newest),
        ("those two", DiscourseGroupSelection::Newest),
        ("양쪽", DiscourseGroupSelection::Newest),
        ("둘 다", DiscourseGroupSelection::Newest),
    ] {
        if lower
            .find(marker)
            .is_some_and(|start| !marker_is_quoted(text, start) && action_group_context(&lower))
        {
            return Some(EventGroupReference {
                marker: marker.to_string(),
                selector: EventGroupSelector::ExactlyTwo,
                group_selection,
            });
        }
    }
    for marker in ["task pair", "작업 묶음"] {
        if lower.find(marker).is_some_and(|start| {
            !marker_is_quoted(text, start)
                && action_group_context(&lower)
                && !topic_keys_from_text(&lower).is_empty()
        }) {
            return Some(EventGroupReference {
                marker: marker.to_string(),
                selector: EventGroupSelector::ExactlyTwo,
                group_selection: DiscourseGroupSelection::SurfaceTopics,
            });
        }
    }
    None
}

fn action_group_context(text: &str) -> bool {
    let korean_context = [
        "상태",
        "결과",
        "실행",
        "완료",
        "성공",
        "실패",
        "끝",
        "검증",
        "현황",
        "진척",
        "진행",
        "어디까지",
        "되어가",
        "마무리",
        "처리",
    ]
    .iter()
    .any(|marker| text.contains(marker));
    let english_context = [
        "status",
        "result",
        "outcome",
        "execution",
        "complete",
        "completed",
        "finish",
        "finished",
        "done",
        "success",
        "succeed",
        "failed",
        "verified",
        "state",
        "states",
        "stand",
        "progress",
        "progressing",
        "coming along",
        "up to speed",
        "catch me up",
        "wrapped up",
        "taken care of",
        "took care of",
        "through with",
    ]
    .iter()
    .any(|marker| contains_ascii_term(text, marker));
    korean_context || english_context
}

fn contains_ascii_term(text: &str, marker: &str) -> bool {
    text.match_indices(marker).any(|(start, matched)| {
        let end = start + matched.len();
        let left_boundary = start == 0
            || text[..start]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        let right_boundary = end == text.len()
            || text[end..]
                .chars()
                .next()
                .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_');
        left_boundary && right_boundary
    })
}

fn resolve_plural_proposition_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    if let Some(resolution) = resolve_explicit_source_group_reference(state, text) {
        return Some(resolution);
    }
    let reference = proposition_group_reference(text)?;
    match persistent_proposition_group_members(state, reference.group_selection, text) {
        PropositionGroupLookup::Selected(eligible) => {
            return Some(build_proposition_group_resolution(
                text,
                &reference.marker,
                &eligible,
                "PERSISTENT_DISCOURSE_GROUP",
            ));
        }
        PropositionGroupLookup::Ambiguous => {
            return Some(ambiguous_group_resolution(
                text,
                "PROPOSITION_GROUP_REFERENCE",
                &reference.marker,
            ));
        }
        PropositionGroupLookup::None => {}
    }
    let mut eligible = state
        .active_discourse_referents
        .iter()
        .filter(|referent| {
            referent.kind == DiscourseReferentKindIR::Proposition
                && referent.attributed_source.is_some()
                && state
                    .completed_turns
                    .saturating_sub(referent.last_referenced_turn)
                    <= MAX_TYPED_REFERENCE_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        left.introduced_turn
            .cmp(&right.introduced_turn)
            .then_with(|| left.referent_id.cmp(&right.referent_id))
    });
    let sources = eligible
        .iter()
        .filter_map(|referent| referent.attributed_source.as_deref())
        .map(str::to_lowercase)
        .collect::<BTreeSet<_>>();
    if eligible.len() != 2 || sources.len() != 2 {
        return Some(ambiguous_group_resolution(
            text,
            "PROPOSITION_GROUP_REFERENCE",
            &reference.marker,
        ));
    }
    Some(build_proposition_group_resolution(
        text,
        &reference.marker,
        &eligible,
        "ACTIVE_PROPOSITION_SET",
    ))
}

enum PropositionGroupLookup<'a> {
    Selected(Vec<&'a DynamicDiscourseReferentIR>),
    Ambiguous,
    None,
}

fn persistent_proposition_group_members<'a>(
    state: &'a ConversationStateIR,
    mut selection: DiscourseGroupSelection,
    text: &str,
) -> PropositionGroupLookup<'a> {
    if selection == DiscourseGroupSelection::Newest
        && state.active_topics.first().is_some_and(|topic| {
            topic.anchor_kind == DiscourseTopicAnchorKindIR::AttributedPropositionGroup
                && topic.anchor_group_id.is_some()
        })
    {
        selection = DiscourseGroupSelection::ActiveTopic;
    }
    let lookup = select_discourse_group(
        state,
        DiscourseGroupKindIR::AttributedProposition,
        selection,
        text,
        |group| group.member_keys.len() >= 2,
    );
    let DiscourseGroupLookup::Selected(group) = lookup else {
        return match lookup {
            DiscourseGroupLookup::Ambiguous => PropositionGroupLookup::Ambiguous,
            DiscourseGroupLookup::None => PropositionGroupLookup::None,
            DiscourseGroupLookup::Selected(_) => unreachable!(),
        };
    };
    let members = group
        .member_keys
        .iter()
        .filter_map(|source| {
            state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == DiscourseReferentKindIR::Proposition
                        && referent
                            .attributed_source
                            .as_deref()
                            .is_some_and(|candidate| {
                                normalize_group_member_key(candidate) == *source
                            })
                        && (group.topic_keys.is_empty()
                            || topic_keys_from_text(&referent.semantic_summary)
                                .iter()
                                .any(|topic| group.topic_keys.contains(topic)))
                })
                .max_by(|left, right| {
                    left.introduced_turn
                        .cmp(&right.introduced_turn)
                        .then_with(|| left.referent_id.cmp(&right.referent_id))
                })
        })
        .collect::<Vec<_>>();
    if members.len() == group.member_keys.len() {
        PropositionGroupLookup::Selected(members)
    } else {
        PropositionGroupLookup::None
    }
}

fn resolve_explicit_source_group_reference(
    state: &ConversationStateIR,
    text: &str,
) -> Option<ReferenceResolutionIR> {
    if !proposition_group_context(&text.to_lowercase()) {
        return None;
    }
    let lower = text.to_lowercase();
    let mut sources = state
        .active_discourse_referents
        .iter()
        .filter(|referent| referent.kind == DiscourseReferentKindIR::Proposition)
        .filter_map(|referent| referent.attributed_source.as_deref())
        .filter(|source| {
            let normalized = normalize_group_member_key(source);
            if normalized.is_ascii() {
                contains_ascii_term(&lower, &normalized)
            } else {
                lower.contains(&normalized)
            }
        })
        .map(normalize_group_member_key)
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    if sources.len() < 2 {
        return None;
    }
    if sources.len() > 2 {
        return Some(ambiguous_group_resolution(
            text,
            "PROPOSITION_GROUP_REFERENCE",
            text,
        ));
    }
    let members = sources
        .iter()
        .filter_map(|source| {
            state
                .active_discourse_referents
                .iter()
                .filter(|referent| {
                    referent.kind == DiscourseReferentKindIR::Proposition
                        && referent
                            .attributed_source
                            .as_deref()
                            .is_some_and(|candidate| {
                                normalize_group_member_key(candidate) == *source
                            })
                })
                .max_by(|left, right| {
                    left.introduced_turn
                        .cmp(&right.introduced_turn)
                        .then_with(|| left.referent_id.cmp(&right.referent_id))
                })
        })
        .collect::<Vec<_>>();
    (members.len() == 2)
        .then(|| build_proposition_group_resolution(text, text, &members, "EXPLICIT_SOURCE_SET"))
}

fn build_proposition_group_resolution(
    text: &str,
    marker: &str,
    eligible: &[&DynamicDiscourseReferentIR],
    group_source: &str,
) -> ReferenceResolutionIR {
    let english = text_is_english(text);
    let descriptions = eligible
        .iter()
        .map(|referent| {
            let source = referent
                .attributed_source
                .as_deref()
                .unwrap_or("DIALOGUE_SOURCE");
            let attitude = referent
                .attribution_attitude
                .unwrap_or(AttributionAttitudeIR::Say);
            let summary = referent
                .semantic_summary
                .trim_matches(|character| matches!(character, '‘' | '’' | '“' | '”' | '"'));
            if english {
                format!(
                    "{source}'s {} ‘{summary}’",
                    attribution_noun(attitude, true)
                )
            } else {
                format!(
                    "{source}의 {} ‘{summary}’",
                    attribution_noun(attitude, false)
                )
            }
        })
        .collect::<Vec<_>>();
    let replacement = if english {
        descriptions.join(" and ")
    } else {
        descriptions.join("과 ")
    };
    let referent_ids = eligible
        .iter()
        .map(|referent| referent.referent_id.clone())
        .collect::<Vec<_>>();
    ReferenceResolutionIR {
        original_semantic_text: text.to_string(),
        resolved_semantic_text: replace_first_case_insensitive(text, marker, &replacement),
        resolved_reference_count: 1,
        used_referent_ids: referent_ids.clone(),
        ambiguous_reference_surfaces: Vec::new(),
        topic_anchored_resolution: None,
        resolution_graph: ReferenceResolutionGraphIR::default(),
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::PluralPropositionReference,
            source_surface: marker.to_string(),
            resolved_surface: replacement,
            referent_ids,
            inherited_goal_id: None,
            confidence_millis: 940,
            evidence: vec![
                "PROPOSITION_GROUP:EXACTLY_TWO_ACTIVE_DISTINCT_SOURCES".to_string(),
                format!("GROUP_SOURCE:{group_source}"),
                "INACTIVE_REVISIONS_EXCLUDED:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }],
    }
}

#[derive(Debug)]
struct PropositionGroupReference {
    marker: String,
    group_selection: DiscourseGroupSelection,
}

fn proposition_group_reference(text: &str) -> Option<PropositionGroupReference> {
    let lower = text.to_lowercase();
    let markers = [
        (
            "that combined speaker group",
            DiscourseGroupSelection::Newest,
        ),
        (
            "the combined speaker group",
            DiscourseGroupSelection::Newest,
        ),
        ("the merged speaker group", DiscourseGroupSelection::Newest),
        ("that speaker group", DiscourseGroupSelection::Newest),
        ("the current speaker group", DiscourseGroupSelection::Newest),
        ("current speaker group", DiscourseGroupSelection::Newest),
        ("합친 화자 묶음", DiscourseGroupSelection::Newest),
        ("병합한 화자 묶음", DiscourseGroupSelection::Newest),
        ("그 화자 묶음", DiscourseGroupSelection::Newest),
        ("현재 화자 묶음", DiscourseGroupSelection::Newest),
        (
            "the first pair's reports",
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "the first pair's claims",
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "the first speaker pair",
            DiscourseGroupSelection::Ordinal(0),
        ),
        (
            "the second pair's reports",
            DiscourseGroupSelection::Ordinal(1),
        ),
        (
            "the second pair's claims",
            DiscourseGroupSelection::Ordinal(1),
        ),
        (
            "the second speaker pair",
            DiscourseGroupSelection::Ordinal(1),
        ),
        ("the earliest speaker pair", DiscourseGroupSelection::Oldest),
        ("the newer speaker pair", DiscourseGroupSelection::Newest),
        (
            "the most recent speaker pair",
            DiscourseGroupSelection::Newest,
        ),
        ("the older pair's reports", DiscourseGroupSelection::Oldest),
        ("the later pair's reports", DiscourseGroupSelection::Newest),
        (
            "this topic's pair of reports",
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "the pair associated with this topic",
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "this topic's speaker pair",
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "the original pair's reports",
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the original pair's claims",
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the earlier pair of statements",
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the earlier pair of reports",
            DiscourseGroupSelection::Oldest,
        ),
        (
            "the earlier pair of claims",
            DiscourseGroupSelection::Oldest,
        ),
        ("those two statements", DiscourseGroupSelection::Newest),
        ("those two reports", DiscourseGroupSelection::Newest),
        ("those two claims", DiscourseGroupSelection::Newest),
        ("what that pair said", DiscourseGroupSelection::Newest),
        ("what those two said", DiscourseGroupSelection::Newest),
        ("their claims", DiscourseGroupSelection::Newest),
        ("their reports", DiscourseGroupSelection::Newest),
        ("their statements", DiscourseGroupSelection::Newest),
        ("their beliefs", DiscourseGroupSelection::Newest),
        ("both claims", DiscourseGroupSelection::Newest),
        ("both reports", DiscourseGroupSelection::Newest),
        ("both statements", DiscourseGroupSelection::Newest),
        (
            "one of the pairs' reports",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "either speaker pair",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "choose one speaker pair",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "either of the two speaker groups",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        ("첫 번째 묶음의 보고", DiscourseGroupSelection::Ordinal(0)),
        ("첫 번째 묶음의 말", DiscourseGroupSelection::Ordinal(0)),
        ("첫 번째 화자 묶음", DiscourseGroupSelection::Ordinal(0)),
        ("두 번째 묶음의 보고", DiscourseGroupSelection::Ordinal(1)),
        ("두 번째 묶음의 말", DiscourseGroupSelection::Ordinal(1)),
        ("두 번째 화자 묶음", DiscourseGroupSelection::Ordinal(1)),
        ("먼저 만든 화자 묶음", DiscourseGroupSelection::Oldest),
        ("최근 화자 묶음", DiscourseGroupSelection::Newest),
        ("앞 화자 묶음", DiscourseGroupSelection::Oldest),
        ("뒤 화자 묶음", DiscourseGroupSelection::Newest),
        (
            "이 주제에 연결된 두 사람의 보고",
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "현재 주제의 화자 묶음",
            DiscourseGroupSelection::ActiveTopic,
        ),
        (
            "현재 주제의 두 사람 보고",
            DiscourseGroupSelection::ActiveTopic,
        ),
        ("처음 묶은 두 사람의 보고", DiscourseGroupSelection::Oldest),
        ("처음 묶은 두 사람의 말", DiscourseGroupSelection::Oldest),
        ("앞서 묶은 두 사람의 말", DiscourseGroupSelection::Oldest),
        ("앞서 묶은 두 사람의 보고", DiscourseGroupSelection::Oldest),
        ("아까 그 두 사람의 보고", DiscourseGroupSelection::Oldest),
        ("그 두 사람의 주장", DiscourseGroupSelection::Newest),
        ("그 두 사람의 보고", DiscourseGroupSelection::Newest),
        ("그 두 사람의 말", DiscourseGroupSelection::Newest),
        ("아까 그 둘이 한 말", DiscourseGroupSelection::Oldest),
        ("아까 그 둘의 보고", DiscourseGroupSelection::Oldest),
        ("그 둘이 말한 내용", DiscourseGroupSelection::Newest),
        ("그 둘의 보고", DiscourseGroupSelection::Newest),
        ("그들의 주장", DiscourseGroupSelection::Newest),
        ("그들의 보고", DiscourseGroupSelection::Newest),
        ("그들의 말", DiscourseGroupSelection::Newest),
        ("그들의 믿음", DiscourseGroupSelection::Newest),
        ("두 사람의 주장", DiscourseGroupSelection::Newest),
        ("두 사람의 보고", DiscourseGroupSelection::Newest),
        ("두 사람의 말", DiscourseGroupSelection::Newest),
        (
            "두 묶음 중 하나의 보고",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "어느 화자 묶음이든",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "두 화자 조 중 아무거나",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
        (
            "어느 한 화자 그룹",
            DiscourseGroupSelection::ExplicitlyAmbiguous,
        ),
    ];
    if let Some((marker, group_selection)) = markers.into_iter().find(|(marker, _)| {
        lower
            .find(marker)
            .is_some_and(|start| !marker_is_quoted(text, start))
    }) {
        return Some(PropositionGroupReference {
            marker: marker.to_string(),
            group_selection,
        });
    }
    for marker in ["those two", "that pair"] {
        if lower.find(marker).is_some_and(|start| {
            !marker_is_quoted(text, start) && proposition_group_context(&lower)
        }) {
            return Some(PropositionGroupReference {
                marker: marker.to_string(),
                group_selection: DiscourseGroupSelection::Newest,
            });
        }
    }
    None
}

fn proposition_group_context(text: &str) -> bool {
    [
        "say",
        "said",
        "claim",
        "report",
        "statement",
        "belie",
        "말",
        "주장",
        "보고",
        "믿",
    ]
    .iter()
    .any(|marker| text.contains(marker))
}

fn marker_is_quoted(text: &str, marker_start: usize) -> bool {
    let prefix = &text[..marker_start];
    let double_open = prefix.chars().filter(|character| *character == '"').count() % 2 == 1;
    let straight_single_open = unmatched_straight_single_quotes(prefix) % 2 == 1;
    let curly_double_open = prefix.chars().filter(|character| *character == '“').count()
        > prefix.chars().filter(|character| *character == '”').count();
    let curly_single_open = prefix.chars().filter(|character| *character == '‘').count()
        > prefix.chars().filter(|character| *character == '’').count();
    let corner_open = prefix
        .chars()
        .filter(|character| *character == '「')
        .count()
        > prefix
            .chars()
            .filter(|character| *character == '」')
            .count();
    double_open || straight_single_open || curly_double_open || curly_single_open || corner_open
}

fn unmatched_straight_single_quotes(text: &str) -> usize {
    let characters = text.char_indices().collect::<Vec<_>>();
    characters
        .iter()
        .enumerate()
        .filter(|(_, (_, character))| *character == '\'')
        .filter(|(index, _)| {
            let previous = index.checked_sub(1).map(|prior| characters[prior].1);
            let next = characters.get(index + 1).map(|(_, character)| *character);
            !(previous.is_some_and(char::is_alphanumeric)
                && next.is_some_and(char::is_alphanumeric))
        })
        .count()
}

fn unquoted_marker_positions(text: &str, marker: &str) -> Vec<usize> {
    let lower = text.to_lowercase();
    let marker = marker.to_lowercase();
    lower
        .match_indices(&marker)
        .map(|(start, _)| start)
        .filter(|start| text.is_char_boundary(*start) && !marker_is_quoted(text, *start))
        .collect()
}

fn unquoted_marker_start(text: &str, marker: &str) -> Option<usize> {
    unquoted_marker_positions(text, marker).into_iter().next()
}

fn unquoted_marker_surface(text: &str, marker: &str) -> Option<String> {
    let start = unquoted_marker_start(text, marker)?;
    let end = start + marker.len();
    text.get(start..end).map(ToString::to_string)
}

fn event_sequence_reference(text: &str) -> Option<EventSequenceReference> {
    let lower = text.to_lowercase();
    let selectors = [
        (
            "the action after the last",
            EventSequenceSelector::AfterLast,
        ),
        ("the task after the last", EventSequenceSelector::AfterLast),
        ("action after the last", EventSequenceSelector::AfterLast),
        ("task after the last", EventSequenceSelector::AfterLast),
        ("마지막 다음 작업", EventSequenceSelector::AfterLast),
        ("마지막 작업 다음", EventSequenceSelector::AfterLast),
        ("the fourth action", EventSequenceSelector::Position(3)),
        ("the fourth task", EventSequenceSelector::Position(3)),
        ("fourth action", EventSequenceSelector::Position(3)),
        ("fourth task", EventSequenceSelector::Position(3)),
        ("네 번째 작업", EventSequenceSelector::Position(3)),
        ("네번째 작업", EventSequenceSelector::Position(3)),
        ("넷째 작업", EventSequenceSelector::Position(3)),
        ("the third action", EventSequenceSelector::Position(2)),
        ("the third task", EventSequenceSelector::Position(2)),
        ("third action", EventSequenceSelector::Position(2)),
        ("third task", EventSequenceSelector::Position(2)),
        ("세 번째 작업", EventSequenceSelector::Position(2)),
        ("세번째 작업", EventSequenceSelector::Position(2)),
        ("셋째 작업", EventSequenceSelector::Position(2)),
        ("the second action", EventSequenceSelector::Position(1)),
        ("the second task", EventSequenceSelector::Position(1)),
        ("second action", EventSequenceSelector::Position(1)),
        ("second task", EventSequenceSelector::Position(1)),
        ("두 번째 작업", EventSequenceSelector::Position(1)),
        ("두번째 작업", EventSequenceSelector::Position(1)),
        ("둘째 작업", EventSequenceSelector::Position(1)),
        ("the first action", EventSequenceSelector::Position(0)),
        ("the first task", EventSequenceSelector::Position(0)),
        ("first action", EventSequenceSelector::Position(0)),
        ("first task", EventSequenceSelector::Position(0)),
        ("첫 번째 작업", EventSequenceSelector::Position(0)),
        ("첫번째 작업", EventSequenceSelector::Position(0)),
        ("첫째 작업", EventSequenceSelector::Position(0)),
        ("the last action", EventSequenceSelector::Last),
        ("the last task", EventSequenceSelector::Last),
        ("last action", EventSequenceSelector::Last),
        ("last task", EventSequenceSelector::Last),
        ("마지막 작업", EventSequenceSelector::Last),
    ];
    selectors.into_iter().find_map(|(marker, selector)| {
        let start = lower.find(marker)?;
        let end = start + marker.len();
        text.get(start..end).map(|surface| EventSequenceReference {
            marker: surface.to_string(),
            selector,
        })
    })
}

fn resolve_typed_discourse_reference(
    state: &ConversationStateIR,
    text: &str,
) -> TypedDiscourseResolution {
    let topic_result_ellipsis = topic_scoped_result_ellipsis_marker(state, text);
    let Some(kind) = discourse_reference_kind(text).or_else(|| {
        topic_result_ellipsis
            .is_some()
            .then_some(DiscourseReferentKindIR::Result)
    }) else {
        return TypedDiscourseResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: Vec::new(),
        };
    };
    let topic_scope = state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)
        .map(|topic| topic.topic_id.as_str());
    let topic_scoped_kind = matches!(
        kind,
        DiscourseReferentKindIR::Event | DiscourseReferentKindIR::Result
    );
    let mut eligible = state
        .active_discourse_referents
        .iter()
        .filter(|referent| {
            referent.kind == kind
                && topic_scope.is_none_or(|topic_id| {
                    !topic_scoped_kind || referent.topic_id.as_deref() == Some(topic_id)
                })
                && (topic_scope.is_some() && topic_scoped_kind
                    || state
                        .completed_turns
                        .saturating_sub(referent.last_referenced_turn)
                        <= MAX_TYPED_REFERENCE_TURN_DISTANCE)
        })
        .collect::<Vec<_>>();
    if kind == DiscourseReferentKindIR::Proposition {
        let explicitly_named = eligible
            .iter()
            .copied()
            .filter(|referent| {
                referent
                    .attributed_source
                    .as_deref()
                    .is_some_and(|source| reference_mentions_source(text, source))
            })
            .collect::<Vec<_>>();
        if !explicitly_named.is_empty() {
            eligible = explicitly_named;
        }
    }
    if let Some(descriptor) = descriptive_discourse_descriptor(text, kind) {
        let described = eligible
            .iter()
            .copied()
            .filter(|referent| descriptor_matches(&referent.semantic_summary, &descriptor))
            .collect::<Vec<_>>();
        if described.is_empty() {
            return TypedDiscourseResolution {
                resolved_text: text.to_string(),
                binding: None,
                ambiguous_surfaces: vec![format!("{kind:?}_REFERENCE:{descriptor}")],
            };
        }
        eligible = described;
    }
    if kind == DiscourseReferentKindIR::Result && topic_scope.is_none() {
        let active_actions = state
            .action_state_ledger
            .records
            .iter()
            .filter(|record| record.plan_status == ActionPlanStatusIR::Active)
            .collect::<Vec<_>>();
        let latest_action_turn = active_actions
            .iter()
            .map(|record| record.introduced_turn)
            .max();
        let equally_recent_actions = active_actions
            .iter()
            .filter(|record| Some(record.introduced_turn) == latest_action_turn)
            .count();
        let explicitly_anchored = active_actions
            .iter()
            .any(|record| reference_mentions_source(text, &record.subject));
        if equally_recent_actions > 1 && !explicitly_anchored {
            return TypedDiscourseResolution {
                resolved_text: text.to_string(),
                binding: None,
                ambiguous_surfaces: vec!["Result_REFERENCE".to_string()],
            };
        }
    }
    let latest_turn = eligible
        .iter()
        .map(|referent| {
            if kind == DiscourseReferentKindIR::Proposition {
                referent.introduced_turn
            } else {
                referent.last_referenced_turn
            }
        })
        .max();
    let latest = eligible
        .into_iter()
        .filter(|referent| {
            Some(if kind == DiscourseReferentKindIR::Proposition {
                referent.introduced_turn
            } else {
                referent.last_referenced_turn
            }) == latest_turn
        })
        .collect::<Vec<_>>();
    if kind == DiscourseReferentKindIR::Result && latest.is_empty() && topic_scope.is_none() {
        let active_actions = state
            .action_state_ledger
            .records
            .iter()
            .filter(|record| record.plan_status == ActionPlanStatusIR::Active)
            .collect::<Vec<_>>();
        if let [record] = active_actions.as_slice() {
            let marker = discourse_reference_markers(kind)
                .iter()
                .find_map(|marker| unquoted_marker_surface(text, marker))
                .or_else(|| topic_result_ellipsis.clone())
                .unwrap_or_default();
            if !marker.is_empty() {
                let replacement = if text_is_english(text) {
                    format!("the result slot for ‘{}’", record.subject)
                } else {
                    format!("‘{}’의 결과 슬롯", record.subject)
                };
                return TypedDiscourseResolution {
                    resolved_text: replace_first_unquoted_case_insensitive(
                        text,
                        &marker,
                        &replacement,
                    ),
                    binding: Some(DiscourseBindingIR {
                        kind: DiscourseBindingKindIR::ResultReference,
                        source_surface: marker,
                        resolved_surface: replacement,
                        referent_ids: Vec::new(),
                        inherited_goal_id: Some(record.goal_id.clone()),
                        confidence_millis: 940,
                        evidence: vec![
                            "RESULT_SLOT:ACTIVE_ACTION_LEDGER".to_string(),
                            "VERIFIED_RESULT_ESTABLISHED:false".to_string(),
                            "SEMANTIC_AUTHORITY:false".to_string(),
                            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                        ],
                    }),
                    ambiguous_surfaces: Vec::new(),
                };
            }
        }
    }
    let Some(referent) = (latest.len() == 1).then(|| latest[0]) else {
        return TypedDiscourseResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: vec![format!("{kind:?}_REFERENCE")],
        };
    };
    let marker = discourse_reference_markers(kind)
        .iter()
        .find_map(|marker| unquoted_marker_surface(text, marker))
        .or_else(|| topic_result_ellipsis.clone())
        .or_else(|| {
            (kind == DiscourseReferentKindIR::Proposition)
                .then(|| attributed_reference_marker(text, referent.attributed_source.as_deref()))
                .flatten()
        })
        .or_else(|| descriptive_discourse_marker(text, kind))
        .unwrap_or_default();
    let summary = referent
        .semantic_summary
        .trim_matches(|character| matches!(character, '‘' | '’' | '“' | '”' | '"' | '\''));
    let attributed_source = referent
        .attributed_source
        .as_deref()
        .zip(referent.attribution_attitude);
    let replacement = if text_is_english(text) {
        match kind {
            DiscourseReferentKindIR::Event => format!("the action ‘{summary}’"),
            DiscourseReferentKindIR::Result => format!("the result of ‘{summary}’"),
            DiscourseReferentKindIR::Proposition => attributed_source.map_or_else(
                || format!("the attributed proposition ‘{summary}’"),
                |(source, attitude)| {
                    format!(
                        "{source}'s {} ‘{summary}’",
                        attribution_noun(attitude, true)
                    )
                },
            ),
        }
    } else {
        match kind {
            DiscourseReferentKindIR::Event => format!("‘{summary}’라는 작업"),
            DiscourseReferentKindIR::Result => format!("‘{summary}’의 결과"),
            DiscourseReferentKindIR::Proposition => attributed_source.map_or_else(
                || format!("‘{summary}’라는 귀속 명제"),
                |(source, attitude)| {
                    format!(
                        "{source}의 {} ‘{summary}’",
                        attribution_noun(attitude, false)
                    )
                },
            ),
        }
    };
    let resolved_text = replace_first_unquoted_case_insensitive(text, &marker, &replacement);
    TypedDiscourseResolution {
        resolved_text: resolved_text.clone(),
        binding: Some(DiscourseBindingIR {
            kind: match kind {
                DiscourseReferentKindIR::Event => DiscourseBindingKindIR::EventReference,
                DiscourseReferentKindIR::Result => DiscourseBindingKindIR::ResultReference,
                DiscourseReferentKindIR::Proposition => {
                    DiscourseBindingKindIR::PropositionReference
                }
            },
            source_surface: marker,
            resolved_surface: replacement,
            referent_ids: vec![referent.referent_id.clone()],
            inherited_goal_id: None,
            confidence_millis: 930,
            evidence: vec![
                format!("DISCOURSE_KIND:{kind:?}"),
                if topic_result_ellipsis.is_some() {
                    "REFERENCE_FORM:TOPIC_SCOPED_RESULT_ELLIPSIS".to_string()
                } else {
                    "REFERENCE_FORM:EXPLICIT_NOMINAL".to_string()
                },
                topic_scope.map_or_else(
                    || "REFERENCE_SCOPE:BOUNDED_RECENCY".to_string(),
                    |topic_id| format!("REFERENCE_SCOPE:EXPLICIT_TOPIC:{topic_id}"),
                ),
                "SEMANTIC_AUTHORITY:false".to_string(),
            ],
        }),
        ambiguous_surfaces: Vec::new(),
    }
}

fn attribution_noun(attitude: AttributionAttitudeIR, english: bool) -> &'static str {
    if english {
        match attitude {
            AttributionAttitudeIR::Believe | AttributionAttitudeIR::Think => "belief",
            AttributionAttitudeIR::Know => "knowledge report",
            AttributionAttitudeIR::Doubt => "doubt",
            AttributionAttitudeIR::Deny => "denial",
            AttributionAttitudeIR::Want => "desire",
            AttributionAttitudeIR::Expect => "expectation",
            AttributionAttitudeIR::Correct => "correction",
            AttributionAttitudeIR::Report => "report",
            _ => "claim",
        }
    } else {
        match attitude {
            AttributionAttitudeIR::Believe | AttributionAttitudeIR::Think => "믿음",
            AttributionAttitudeIR::Know => "앎의 보고",
            AttributionAttitudeIR::Doubt => "의심",
            AttributionAttitudeIR::Deny => "부인",
            AttributionAttitudeIR::Want => "바람",
            AttributionAttitudeIR::Expect => "예상",
            AttributionAttitudeIR::Correct => "정정",
            AttributionAttitudeIR::Report => "보고",
            _ => "주장",
        }
    }
}

fn discourse_reference_kind(text: &str) -> Option<DiscourseReferentKindIR> {
    if proposition_reference_surface(text) {
        return Some(DiscourseReferentKindIR::Proposition);
    }
    if descriptive_discourse_marker(text, DiscourseReferentKindIR::Event).is_some() {
        return Some(DiscourseReferentKindIR::Event);
    }
    [
        DiscourseReferentKindIR::Result,
        DiscourseReferentKindIR::Event,
        DiscourseReferentKindIR::Proposition,
    ]
    .into_iter()
    .find(|kind| {
        discourse_reference_markers(*kind)
            .iter()
            .any(|marker| unquoted_marker_start(text, marker).is_some())
    })
}

fn topic_scoped_result_ellipsis_marker(state: &ConversationStateIR, text: &str) -> Option<String> {
    state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)?;
    let markers: &[&str] = if text_is_english(text) {
        &[
            "the result",
            "the output",
            "the outcome",
            "its result",
            "its output",
            "its outcome",
        ]
    } else {
        &[
            "결과는",
            "결과가",
            "결과를",
            "결과만",
            "출력은",
            "출력이",
            "출력을",
            "출력만",
            "산출물은",
            "산출물이",
            "산출물을",
            "산출물만",
        ]
    };
    markers
        .iter()
        .find_map(|marker| unquoted_marker_surface(text, marker))
}

fn descriptive_discourse_marker(text: &str, kind: DiscourseReferentKindIR) -> Option<String> {
    if kind != DiscourseReferentKindIR::Event {
        return None;
    }
    if text_is_english(text) {
        let lower = text.to_lowercase();
        for start in unquoted_marker_positions(text, "that ") {
            let tail = &lower[start..];
            if let Some(noun) = [" operation", " action", " task", " event"]
                .into_iter()
                .filter_map(|marker| tail.find(marker).map(|index| (index, marker.len())))
                .min_by_key(|(index, _)| *index)
            {
                let end = start + noun.0 + noun.1;
                if let Some(surface) = text.get(start..end) {
                    return Some(surface.to_string());
                }
            }
        }
        return None;
    }
    for start in unquoted_marker_positions(text, "그 ") {
        let tail = &text[start..];
        if let Some(noun) = [" 작업", " 동작", " 과정", " 사건"]
            .into_iter()
            .filter_map(|marker| tail.find(marker).map(|index| (index, marker.len())))
            .min_by_key(|(index, _)| *index)
        {
            let end = start + noun.0 + noun.1;
            if let Some(surface) = text.get(start..end) {
                return Some(surface.to_string());
            }
        }
    }
    None
}

fn descriptive_discourse_descriptor(text: &str, kind: DiscourseReferentKindIR) -> Option<String> {
    let marker = descriptive_discourse_marker(text, kind)?;
    let noise = [
        "that",
        "operation",
        "action",
        "task",
        "event",
        "그",
        "작업",
        "동작",
        "과정",
        "사건",
    ];
    let descriptor = marker
        .to_lowercase()
        .split_whitespace()
        .filter(|token| !noise.contains(token))
        .collect::<Vec<_>>()
        .join(" ");
    (!descriptor.is_empty()).then_some(descriptor)
}

fn descriptor_matches(summary: &str, descriptor: &str) -> bool {
    let summary = summary.to_lowercase();
    descriptor
        .split_whitespace()
        .all(|token| summary.contains(token))
}

fn proposition_reference_surface(text: &str) -> bool {
    [
        "그 주장",
        "그 사실",
        "의 주장",
        "의 믿음",
        "의 말",
        "that claim",
        "that belief",
        "that statement",
        "'s claim",
        "'s belief",
        "'s statement",
    ]
    .iter()
    .any(|marker| unquoted_marker_start(text, marker).is_some())
}

fn reference_mentions_source(text: &str, source: &str) -> bool {
    let text = text.to_lowercase();
    let source = source.to_lowercase();
    text.contains(&source)
}

fn attributed_reference_marker(text: &str, source: Option<&str>) -> Option<String> {
    let source = source?.to_lowercase();
    let lower = text.to_lowercase();
    let start = lower.find(&source)?;
    let tail = &lower[start + source.len()..];
    for noun in ["claim", "belief", "statement", "주장", "믿음", "말"] {
        if let Some(noun_start) = tail.find(noun) {
            let end = start + source.len() + noun_start + noun.len();
            return text.get(start..end).map(ToString::to_string);
        }
    }
    None
}

fn discourse_reference_markers(kind: DiscourseReferentKindIR) -> &'static [&'static str] {
    match kind {
        DiscourseReferentKindIR::Event => &[
            "그 작업",
            "그 동작",
            "그 과정",
            "that task",
            "that action",
            "that operation",
        ],
        DiscourseReferentKindIR::Result => &[
            "그 실제 결과",
            "그 결과",
            "그 출력",
            "그 산출물",
            "that actual result",
            "that result",
            "that output",
            "that outcome",
        ],
        DiscourseReferentKindIR::Proposition => &[
            "그 사실",
            "그 주장",
            "그 믿음",
            "그 말",
            "that fact",
            "that claim",
            "that belief",
            "that statement",
            "that proposition",
        ],
    }
}

fn replace_first_case_insensitive(text: &str, marker: &str, replacement: &str) -> String {
    if marker.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let Some(start) = lower.find(marker) else {
        return text.to_string();
    };
    let end = start + marker.len();
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.to_string();
    }
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

fn replace_first_unquoted_case_insensitive(text: &str, marker: &str, replacement: &str) -> String {
    if marker.is_empty() {
        return text.to_string();
    }
    let Some(start) = unquoted_marker_start(text, marker) else {
        return text.to_string();
    };
    let end = start + marker.len();
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.to_string();
    }
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

#[derive(Debug)]
struct GoalEllipsisResolution {
    resolved_text: String,
    binding: Option<DiscourseBindingIR>,
    ambiguous_surfaces: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum GoalEllipsisKind {
    Repeat,
    ParallelArgument,
    CorrectedArgument,
}

#[derive(Debug, Clone)]
struct GoalEllipsisSubject {
    surface: String,
    concept_id: Option<String>,
}

fn resolve_goal_ellipsis(state: &ConversationStateIR, text: &str) -> GoalEllipsisResolution {
    let Some((kind, replacement_subject)) = classify_goal_ellipsis(text) else {
        return GoalEllipsisResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: Vec::new(),
        };
    };
    if goal_ellipsis_marker_is_quoted(text)
        || (matches!(kind, GoalEllipsisKind::ParallelArgument) && replacement_subject.is_none())
    {
        return GoalEllipsisResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: vec!["ELLIPTICAL_ACTION".to_string()],
        };
    }
    if matches!(kind, GoalEllipsisKind::Repeat) {
        match explicit_topic_action_record(state) {
            Ok(Some(record)) => {
                let resolved_text = repeat_action_record_in_current_language(record, text);
                return GoalEllipsisResolution {
                    resolved_text: resolved_text.clone(),
                    binding: Some(DiscourseBindingIR {
                        kind: DiscourseBindingKindIR::RepeatedGoal,
                        source_surface: text.to_string(),
                        resolved_surface: resolved_text,
                        referent_ids: Vec::new(),
                        inherited_goal_id: Some(record.goal_id.clone()),
                        confidence_millis: 930,
                        evidence: vec![
                            "GOAL_INHERITANCE:EXPLICIT_TOPIC_ACTION_LEDGER".to_string(),
                            "WITHDRAWN_ACTION_EXCLUDED:true".to_string(),
                            "PREDICATE_ELLIPSIS_TYPED:true".to_string(),
                            "SEMANTIC_AUTHORITY:false".to_string(),
                            "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                        ],
                    }),
                    ambiguous_surfaces: Vec::new(),
                };
            }
            Err(()) => {
                return GoalEllipsisResolution {
                    resolved_text: text.to_string(),
                    binding: None,
                    ambiguous_surfaces: vec!["ELLIPTICAL_ACTION".to_string()],
                };
            }
            Ok(None) => {}
        }
    }
    if matches!(kind, GoalEllipsisKind::ParallelArgument) {
        if let Some(subject) = replacement_subject.as_ref() {
            match discourse_program_for_parallel_ellipsis(state) {
                Ok(Some(program)) if program.replayable => {
                    let resolved_text = render_discourse_program_for_subject(
                        program,
                        &subject.surface,
                        subject.concept_id.as_deref(),
                        text,
                    );
                    let referent_ids = subject
                        .concept_id
                        .as_deref()
                        .map(|concept_id| {
                            state
                                .active_referents
                                .iter()
                                .filter(|referent| referent.canonical_concept == concept_id)
                                .map(|referent| referent.referent_id.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let inherited_goal_ids = program
                        .steps
                        .iter()
                        .map(|step| step.goal.goal_id.clone())
                        .collect::<Vec<_>>();
                    let guarded_step_count = program.guarded_step_count;
                    let mut evidence = vec![
                        "DISCOURSE_PROGRAM_INSTANTIATION:true".to_string(),
                        format!("PROGRAM_ID:{}", program.program_id),
                        format!("ORDERED_STEP_COUNT:{}", program.steps.len()),
                        format!("INHERITED_GOAL_IDS:{}", inherited_goal_ids.join(",")),
                        "EXPLICIT_REPLACEMENT_SUBJECT:true".to_string(),
                        "SEMANTIC_AUTHORITY:false".to_string(),
                        "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
                    ];
                    if guarded_step_count > 0 {
                        evidence.extend([
                            "GUARDED_DISCOURSE_PROGRAM_INSTANTIATION:true".to_string(),
                            format!("GUARDED_STEP_COUNT:{guarded_step_count}"),
                            "TRUSTED_EVIDENCE_REQUIRED:true".to_string(),
                        ]);
                    }
                    return GoalEllipsisResolution {
                        resolved_text: resolved_text.clone(),
                        binding: Some(DiscourseBindingIR {
                            kind: DiscourseBindingKindIR::DiscourseProgramInstantiation,
                            source_surface: text.to_string(),
                            resolved_surface: resolved_text,
                            referent_ids,
                            inherited_goal_id: inherited_goal_ids.first().cloned(),
                            confidence_millis: 950,
                            evidence,
                        }),
                        ambiguous_surfaces: Vec::new(),
                    };
                }
                Ok(Some(_)) | Err(()) => {
                    return GoalEllipsisResolution {
                        resolved_text: text.to_string(),
                        binding: None,
                        ambiguous_surfaces: vec!["ELLIPTICAL_ACTION".to_string()],
                    };
                }
                Ok(None) => {}
            }
        }
    }
    let mut eligible_goals = state
        .active_goals
        .iter()
        .filter(|goal| {
            state
                .completed_turns
                .saturating_sub(goal.last_referenced_turn)
                <= MAX_GOAL_ELLIPSIS_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    if eligible_goals.len() > 1 {
        if let Some(topic_goal_id) = state
            .active_topics
            .first()
            .filter(|topic| topic.explicitly_activated)
            .and_then(|topic| topic_goal_id(state, topic))
        {
            eligible_goals.retain(|goal| goal.goal_id == topic_goal_id);
        }
    }
    if eligible_goals.len() != 1 {
        return GoalEllipsisResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: vec!["ELLIPTICAL_ACTION".to_string()],
        };
    }
    let goal = eligible_goals[0];
    let resolved_text = match (kind, replacement_subject.as_ref()) {
        (GoalEllipsisKind::Repeat, _) => repeat_goal_in_current_language(goal, text),
        (_, Some(subject)) => {
            render_goal_for_subject(goal, &subject.surface, subject.concept_id.as_deref(), text)
        }
        (_, None) => text.to_string(),
    };
    let referent_ids = replacement_subject
        .as_ref()
        .and_then(|subject| subject.concept_id.as_deref())
        .map(|concept_id| {
            state
                .active_referents
                .iter()
                .filter(|referent| referent.canonical_concept == concept_id)
                .map(|referent| referent.referent_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    GoalEllipsisResolution {
        resolved_text: resolved_text.clone(),
        binding: Some(DiscourseBindingIR {
            kind: match kind {
                GoalEllipsisKind::Repeat => DiscourseBindingKindIR::RepeatedGoal,
                GoalEllipsisKind::ParallelArgument => DiscourseBindingKindIR::EllipticalAction,
                GoalEllipsisKind::CorrectedArgument => DiscourseBindingKindIR::CorrectedArgument,
            },
            source_surface: text.to_string(),
            resolved_surface: resolved_text,
            referent_ids,
            inherited_goal_id: Some(goal.goal_id.clone()),
            confidence_millis: 920,
            evidence: vec![
                "GOAL_INHERITANCE:ACTIVE_GOAL_FRAME".to_string(),
                "PREDICATE_ELLIPSIS_TYPED:true".to_string(),
                "SEMANTIC_AUTHORITY:false".to_string(),
                "EXTERNAL_EXECUTION_AUTHORIZED:false".to_string(),
            ],
        }),
        ambiguous_surfaces: Vec::new(),
    }
}

fn discourse_program_for_parallel_ellipsis(
    state: &ConversationStateIR,
) -> Result<Option<&DiscourseProgramIR>, ()> {
    let eligible = state
        .active_discourse_programs
        .iter()
        .filter(|program| {
            state
                .completed_turns
                .saturating_sub(program.last_referenced_turn)
                <= MAX_GOAL_ELLIPSIS_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Ok(None);
    }
    if let Some(topic) = state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)
    {
        let topical = eligible
            .iter()
            .copied()
            .filter(|program| topic_matches_subject(topic, &program.shared_subject))
            .collect::<Vec<_>>();
        match topical.as_slice() {
            [program] => return Ok(Some(*program)),
            [_, ..] => return Err(()),
            [] => {}
        }
    }
    let active_goal_ids = state
        .active_goals
        .iter()
        .map(|goal| goal.goal_id.as_str())
        .collect::<BTreeSet<_>>();
    let current = eligible
        .iter()
        .copied()
        .filter(|program| {
            program
                .steps
                .iter()
                .map(|step| step.goal.goal_id.as_str())
                .collect::<BTreeSet<_>>()
                == active_goal_ids
        })
        .collect::<Vec<_>>();
    match current.as_slice() {
        [program] => return Ok(Some(*program)),
        [_, ..] => return Err(()),
        [] => {}
    }
    match eligible.as_slice() {
        [program] => Ok(Some(*program)),
        _ => Err(()),
    }
}

fn render_discourse_program_for_subject(
    program: &DiscourseProgramIR,
    replacement_surface: &str,
    concept_id: Option<&str>,
    current_text: &str,
) -> String {
    let english = text_is_english(current_text);
    let subject = concept_id
        .and_then(|concept| concept_surface(concept, english))
        .unwrap_or(replacement_surface);
    let clauses = program
        .steps
        .iter()
        .map(|step| {
            let action = localized_action_surface(&step.goal, english);
            if let Some(guard) = step.guard.as_ref() {
                let antecedent = render_discourse_guard_antecedent(guard, subject, english);
                if english {
                    format!("if {antecedent}, {action} {subject}")
                } else {
                    format!(
                        "{antecedent} {subject}{} {action}",
                        object_particle(subject)
                    )
                }
            } else if english {
                format!("{action} {subject}")
            } else {
                format!("{subject}{} {action}", object_particle(subject))
            }
        })
        .collect::<Vec<_>>();
    if english {
        clauses.join(", then ")
    } else {
        clauses
            .iter()
            .enumerate()
            .map(|(index, clause)| {
                if index + 1 == clauses.len() {
                    clause.clone()
                } else {
                    korean_clause_as_connective(clause)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn render_discourse_guard_antecedent(
    guard: &DiscourseProgramGuardIR,
    subject: &str,
    english: bool,
) -> String {
    if english {
        render_guard_condition_expression_english(&guard.condition_expression, subject, 0)
    } else {
        render_guard_condition_expression_korean(
            &guard.condition_expression,
            subject,
            KoreanGuardConnector::Conditional,
            0,
        )
    }
}

fn guard_condition_precedence(operator: GuardConditionOperatorIR) -> u8 {
    match operator {
        GuardConditionOperatorIR::Any => 1,
        GuardConditionOperatorIR::All => 2,
        GuardConditionOperatorIR::Not => 3,
        GuardConditionOperatorIR::Atom => 4,
    }
}

fn render_guard_condition_expression_english(
    expression: &GuardConditionExpressionIR,
    subject: &str,
    parent_precedence: u8,
) -> String {
    let precedence = guard_condition_precedence(expression.operator);
    let rendered = match expression.operator {
        GuardConditionOperatorIR::Atom => render_guard_atom_english(
            expression
                .canonical_predicate
                .as_deref()
                .unwrap_or("UNRESOLVED"),
            subject,
            false,
        ),
        GuardConditionOperatorIR::Not => render_guard_atom_english(
            expression
                .children
                .first()
                .and_then(|child| child.canonical_predicate.as_deref())
                .unwrap_or("UNRESOLVED"),
            subject,
            true,
        ),
        GuardConditionOperatorIR::All | GuardConditionOperatorIR::Any => {
            let connector = if expression.operator == GuardConditionOperatorIR::All {
                " and "
            } else {
                " or "
            };
            expression
                .children
                .iter()
                .map(|child| render_guard_condition_expression_english(child, subject, precedence))
                .collect::<Vec<_>>()
                .join(connector)
        }
    };
    if precedence < parent_precedence {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn render_guard_atom_english(predicate: &str, subject: &str, negated: bool) -> String {
    match (predicate, negated) {
        ("PROBLEM_PRESENT", false) => format!("{subject} has a problem"),
        ("PROBLEM_PRESENT", true) => format!("{subject} has no problem"),
        ("ERROR_PRESENT", false) => format!("{subject} has an error"),
        ("ERROR_PRESENT", true) => format!("{subject} has no error"),
        ("STALE", false) => format!("{subject} is stale"),
        ("STALE", true) => format!("{subject} is not stale"),
        ("INVALID", false) => format!("{subject} is invalid"),
        ("INVALID", true) => format!("{subject} is not invalid"),
        ("VALID", false) => format!("{subject} is valid"),
        ("VALID", true) => format!("{subject} is not valid"),
        ("UNHEALTHY", false) => format!("{subject} is unhealthy"),
        ("UNHEALTHY", true) => format!("{subject} is not unhealthy"),
        ("HEALTHY", false) => format!("{subject} is healthy"),
        ("HEALTHY", true) => format!("{subject} is not healthy"),
        ("DAMAGED", false) => format!("{subject} is damaged"),
        ("DAMAGED", true) => format!("{subject} is not damaged"),
        ("EMPTY", false) => format!("{subject} is empty"),
        ("EMPTY", true) => format!("{subject} is not empty"),
        (_, false) => format!("{subject} is {predicate}"),
        (_, true) => format!("{subject} is not {predicate}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KoreanGuardConnector {
    Conditional,
    And,
    Or,
}

fn render_guard_condition_expression_korean(
    expression: &GuardConditionExpressionIR,
    subject: &str,
    terminal: KoreanGuardConnector,
    parent_precedence: u8,
) -> String {
    let precedence = guard_condition_precedence(expression.operator);
    let rendered = match expression.operator {
        GuardConditionOperatorIR::Atom => render_guard_atom_korean(
            expression
                .canonical_predicate
                .as_deref()
                .unwrap_or("UNRESOLVED"),
            subject,
            false,
            terminal,
        ),
        GuardConditionOperatorIR::Not => render_guard_atom_korean(
            expression
                .children
                .first()
                .and_then(|child| child.canonical_predicate.as_deref())
                .unwrap_or("UNRESOLVED"),
            subject,
            true,
            terminal,
        ),
        GuardConditionOperatorIR::All | GuardConditionOperatorIR::Any => expression
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                let child_terminal = if index + 1 == expression.children.len() {
                    terminal
                } else if expression.operator == GuardConditionOperatorIR::All {
                    KoreanGuardConnector::And
                } else {
                    KoreanGuardConnector::Or
                };
                render_guard_condition_expression_korean(child, subject, child_terminal, precedence)
            })
            .collect::<Vec<_>>()
            .join(" "),
    };
    if precedence < parent_precedence {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn render_guard_atom_korean(
    predicate: &str,
    subject: &str,
    negated: bool,
    terminal: KoreanGuardConnector,
) -> String {
    let particle = subject_particle(subject);
    let stem = match (predicate, negated) {
        ("PROBLEM_PRESENT", false) => format!("{subject}에 문제가 있"),
        ("PROBLEM_PRESENT", true) => format!("{subject}에 문제가 없"),
        ("ERROR_PRESENT", false) => format!("{subject}에 오류가 있"),
        ("ERROR_PRESENT", true) => format!("{subject}에 오류가 없"),
        ("STALE", false) => format!("{subject}{particle} 오래됐"),
        ("STALE", true) => format!("{subject}{particle} 오래되지 않았"),
        ("INVALID", false) => format!("{subject}{particle} 무효이"),
        ("INVALID", true) => format!("{subject}{particle} 무효가 아니"),
        ("VALID", false) => format!("{subject}{particle} 유효하"),
        ("VALID", true) => format!("{subject}{particle} 유효하지 않"),
        ("UNHEALTHY", false) => format!("{subject}{particle} 건강하지 않"),
        ("UNHEALTHY", true) => format!("{subject}{particle} 건강하"),
        ("HEALTHY", false) => format!("{subject}{particle} 건강하"),
        ("HEALTHY", true) => format!("{subject}{particle} 건강하지 않"),
        ("DAMAGED", false) => format!("{subject}{particle} 손상됐"),
        ("DAMAGED", true) => format!("{subject}{particle} 손상되지 않았"),
        ("EMPTY", false) => format!("{subject}{particle} 비었"),
        ("EMPTY", true) => format!("{subject}{particle} 비어 있지 않"),
        (_, false) => format!("{subject}{particle} {predicate}"),
        (_, true) => format!("{subject}{particle} {predicate}이 아니"),
    };
    match terminal {
        KoreanGuardConnector::Conditional => {
            if stem.ends_with('하') {
                format!("{stem}면")
            } else {
                format!("{stem}으면")
            }
        }
        KoreanGuardConnector::And => format!("{stem}고"),
        KoreanGuardConnector::Or => format!("{stem}거나"),
    }
}

fn korean_clause_as_connective(clause: &str) -> String {
    if let Some(stem) = clause.strip_suffix("고쳐") {
        return format!("{stem}고치고");
    }
    if let Some(stem) = clause.strip_suffix("해") {
        return format!("{stem}하고");
    }
    if let Some(stem) = clause.strip_suffix("어") {
        return format!("{stem}고");
    }
    format!("{clause} 그리고")
}

fn explicit_topic_action_record(
    state: &ConversationStateIR,
) -> Result<Option<&ActionStateRecordIR>, ()> {
    let Some(topic) = state
        .active_topics
        .first()
        .filter(|topic| topic.explicitly_activated)
    else {
        return Ok(None);
    };
    let matches = state
        .action_state_ledger
        .records
        .iter()
        .filter(|record| {
            record.plan_status != ActionPlanStatusIR::Withdrawn
                && topic_matches_subject(topic, &record.subject)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [record] => Ok(Some(*record)),
        _ => Err(()),
    }
}

fn repeat_action_record_in_current_language(
    record: &ActionStateRecordIR,
    current_text: &str,
) -> String {
    let goal = ConversationGoalFrameIR {
        goal_id: record.goal_id.clone(),
        intent: intent_for_canonical_predicate(&record.canonical_predicate),
        canonical_predicate: record.canonical_predicate.clone(),
        predicate_surface: record.predicate_surface.clone(),
        subject: record.subject.clone(),
        source_semantic_text: record.source_semantic_text.clone(),
        introduced_turn: record.introduced_turn,
        last_referenced_turn: record.last_update_turn,
        external_execution_authorized: record.external_execution_authorized,
    };
    repeat_goal_in_current_language(&goal, current_text)
}

fn intent_for_canonical_predicate(canonical_predicate: &str) -> PlanIntentIR {
    match canonical_predicate {
        "PLAN" => PlanIntentIR::Plan,
        "INVESTIGATE" => PlanIntentIR::Investigate,
        "REPAIR" => PlanIntentIR::Repair,
        "CREATE" => PlanIntentIR::Create,
        "LEARN" => PlanIntentIR::Learn,
        "EXPLAIN" => PlanIntentIR::Explain,
        "COMMUNICATE" => PlanIntentIR::Communicate,
        _ => PlanIntentIR::Execute,
    }
}

fn classify_goal_ellipsis(text: &str) -> Option<(GoalEllipsisKind, Option<GoalEllipsisSubject>)> {
    let normalized = text
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_lowercase();
    if [
        "그대로 해",
        "그대로 해줘",
        "똑같이 해",
        "똑같이 해줘",
        "그거 다시 해",
        "그거 다시 해줘",
        "그것을 다시 해",
        "그 작업 다시 해",
        "do the same",
        "do the same again",
        "do it again",
        "do that again",
        "repeat that",
        "same again",
    ]
    .contains(&normalized.as_str())
    {
        return Some((GoalEllipsisKind::Repeat, None));
    }
    let interrogative_surface = text.trim_end().ends_with('?');
    let parallel_marker = normalized.contains("같은 방식")
        || normalized.contains("같은 절차")
        || normalized.contains("똑같이")
        || normalized.contains("그렇게")
        || normalized.ends_with('도')
        || normalized.contains("same for ")
        || normalized.contains("same operation for ")
        || normalized.contains("apply the same")
        || normalized.contains("same workflow")
        || normalized.contains("same procedure")
        || normalized.contains("same guarded procedure")
        || normalized.contains("that procedure")
        || normalized.contains("that workflow")
        || normalized.contains("the workflow for")
        || normalized.contains("repeat that workflow")
        || (normalized.ends_with(" too") && !interrogative_surface);
    if parallel_marker {
        if let Some(surface) = open_predicate_ellipsis_subject(&normalized) {
            return Some((
                GoalEllipsisKind::ParallelArgument,
                Some(GoalEllipsisSubject {
                    surface,
                    concept_id: None,
                }),
            ));
        }
        if let Some(subject) = known_subject_in_fragment(&normalized) {
            return Some((
                GoalEllipsisKind::ParallelArgument,
                Some(GoalEllipsisSubject {
                    surface: subject.0,
                    concept_id: Some(subject.1),
                }),
            ));
        }
        return Some((GoalEllipsisKind::ParallelArgument, None));
    }
    if !contains_explicit_action_surface(&normalized)
        && (normalized.contains("말고")
            || (normalized.contains("instead")
                && (normalized.contains("not ") || normalized.contains("rather "))))
    {
        if let Some(subject) = known_subject_in_fragment(&normalized) {
            return Some((
                GoalEllipsisKind::CorrectedArgument,
                Some(GoalEllipsisSubject {
                    surface: subject.0,
                    concept_id: Some(subject.1),
                }),
            ));
        }
    }
    None
}

fn goal_ellipsis_marker_is_quoted(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "같은 방식",
        "같은 절차",
        "똑같이",
        "그렇게",
        "do the same",
        "apply the same",
        "same workflow",
        "same procedure",
        "same guarded procedure",
        "that procedure",
        "that workflow",
        "repeat that workflow",
        "repeat the workflow",
    ]
    .iter()
    .filter_map(|marker| lower.find(marker))
    .any(|start| marker_is_quoted(text, start))
}

fn is_goal_ellipsis_surface(text: &str) -> bool {
    classify_goal_ellipsis(text).is_some()
}

pub(crate) fn contains_explicit_action_surface(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    let korean_action = [
        "열어", "읽어", "변환", "저장", "삭제", "지워", "배포", "실행", "확인", "검사", "조사",
        "분석", "고쳐", "수정", "수리", "복구", "만들", "작성", "생성", "설명", "해설", "기록",
        "전달", "말해", "학습", "배워", "익혀",
    ]
    .iter()
    .any(|surface| normalized.contains(surface));
    if korean_action {
        return true;
    }
    let english_verbs = [
        "open",
        "read",
        "transform",
        "convert",
        "save",
        "delete",
        "deploy",
        "run",
        "check",
        "inspect",
        "analyze",
        "verify",
        "validate",
        "fix",
        "repair",
        "restore",
        "create",
        "write",
        "explain",
        "record",
        "report",
        "tell",
        "learn",
    ];
    let tokens = normalized
        .split_whitespace()
        .map(|token| token.trim_matches(|character: char| !character.is_alphanumeric()))
        .collect::<Vec<_>>();
    if tokens.iter().enumerate().any(|(index, token)| {
        english_verbs.contains(token)
            && (index == 0
                || tokens.get(index.wrapping_sub(1)).is_some_and(|previous| {
                    matches!(
                        *previous,
                        "please"
                            | "to"
                            | "and"
                            | "then"
                            | "now"
                            | "do"
                            | "don't"
                            | "not"
                            | "never"
                            | "must"
                            | "should"
                            | "can"
                            | "could"
                    )
                }))
    }) {
        return true;
    }
    english_verbs.iter().any(|verb| {
        unquoted_marker_positions(&normalized, verb)
            .into_iter()
            .any(|start| {
                let before = normalized[..start].chars().next_back();
                let after = normalized[start + verb.len()..].chars().next();
                let word_bounded = before.is_none_or(|character| !character.is_alphanumeric())
                    && after.is_none_or(|character| !character.is_alphanumeric());
                let clause_prefix = normalized[..start]
                    .rsplit(['.', ';', '!', '?', ':'])
                    .next()
                    .unwrap_or_default()
                    .trim_matches(|character: char| {
                        character.is_whitespace()
                            || matches!(character, '‘' | '’' | '“' | '”' | '"' | '\'')
                    });
                word_bounded && clause_prefix.is_empty()
            })
    })
}

fn resolve_question_answer(
    question: &QuestionUnderDiscussionIR,
    answer_text: &str,
) -> QuestionAnswerResolutionIR {
    let normalized = answer_text
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_lowercase();
    let not_applicable = || QuestionAnswerResolutionIR {
        disposition: QuestionAnswerDispositionIR::NotApplicable,
        resolved_semantic_text: answer_text.to_string(),
        binding: None,
    };
    if normalized.is_empty() {
        return not_applicable();
    }

    let selection_marker = contains_any_surface(
        &normalized,
        &[
            "쪽", "선택", "번째", "첫째", "둘째", "option", "action", "fact", "one",
        ],
    );
    let ordinal = ordinal_answer_index(&normalized);
    let selection_like = selection_marker
        || ordinal.is_some()
        || question
            .options
            .iter()
            .any(|option| question_option_score(&normalized, option) > 0);
    let non_authoritative = contains_any_surface(
        &normalized,
        &[
            "라고 말했다",
            "라고 말했",
            "보고했다",
            "according to",
            " said ",
            "reported",
            "maybe",
            "perhaps",
            "possibly",
            "아마",
            "어쩌면",
            "일 수도",
        ],
    );
    if non_authoritative && selection_like {
        return invalid_question_answer(answer_text);
    }
    if contains_explicit_action_surface(&normalized) && !selection_marker {
        return not_applicable();
    }
    if let Some(index) = ordinal {
        return question
            .options
            .get(index)
            .map(|option| resolved_question_answer(question, answer_text, option))
            .unwrap_or_else(|| invalid_question_answer(answer_text));
    }

    let mut ranked = question
        .options
        .iter()
        .map(|option| (question_option_score(&normalized, option), option))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.option_id.cmp(&right.1.option_id))
    });
    let strongest = ranked.first().map_or(0, |(score, _)| *score);
    let second = ranked.get(1).map_or(0, |(score, _)| *score);
    if strongest > 0 && strongest > second {
        return resolved_question_answer(question, answer_text, ranked[0].1);
    }
    if selection_like || normalized.split_whitespace().count() <= 5 {
        return invalid_question_answer(answer_text);
    }
    not_applicable()
}

fn resolved_question_answer(
    question: &QuestionUnderDiscussionIR,
    answer_text: &str,
    option: &QuestionOptionIR,
) -> QuestionAnswerResolutionIR {
    let resolved_semantic_text = localize_question_option_for_answer(option, answer_text);
    QuestionAnswerResolutionIR {
        disposition: QuestionAnswerDispositionIR::Resolved,
        resolved_semantic_text: resolved_semantic_text.clone(),
        binding: Some(DiscourseBindingIR {
            kind: DiscourseBindingKindIR::ClarificationAnswer,
            source_surface: answer_text.to_string(),
            resolved_surface: resolved_semantic_text,
            referent_ids: option.referent_ids.clone(),
            inherited_goal_id: None,
            confidence_millis: 960,
            evidence: vec![
                format!("PENDING_QUD:{}", question.question_id),
                format!("SELECTED_OPTION:{}", option.option_id),
                "ANSWER_ONLY_SELECTS_PENDING_READING:true".to_string(),
                "NEW_EXECUTION_AUTHORITY:false".to_string(),
            ],
        }),
    }
}

fn localize_question_option_for_answer(option: &QuestionOptionIR, answer_text: &str) -> String {
    if !text_is_english(answer_text) || text_is_english(&option.resolved_semantic_text) {
        return option.resolved_semantic_text.clone();
    }
    let lower = format!(
        "{} {}",
        option.display_surface.to_lowercase(),
        option.resolved_semantic_text.to_lowercase()
    );
    let subject = [
        (["폴더", "folder"].as_slice(), "folder"),
        (["파일", "file"].as_slice(), "file"),
        (["백업", "backup"].as_slice(), "backup"),
        (["캐시", "cache"].as_slice(), "cache"),
        (["가시", "cash"].as_slice(), "cash"),
        (["로그", "log"].as_slice(), "log"),
        (["서버", "server"].as_slice(), "server"),
        (["워커", "worker"].as_slice(), "worker"),
        (["코드", "code"].as_slice(), "code"),
        (["문서", "document"].as_slice(), "document"),
        (["빌드", "build"].as_slice(), "build"),
        (["테스트", "test"].as_slice(), "test"),
        (["배포", "rollout"].as_slice(), "rollout"),
        (["큐", "queue"].as_slice(), "queue"),
        (["api"].as_slice(), "API"),
    ]
    .iter()
    .find(|(aliases, _)| aliases.iter().any(|alias| lower.contains(alias)))
    .map_or(option.display_surface.as_str(), |(_, surface)| *surface);
    let action = [
        (["열", "open"].as_slice(), "open"),
        (["읽", "read"].as_slice(), "read"),
        (["저장", "save"].as_slice(), "save"),
        (["삭제", "지우", "delete", "clear"].as_slice(), "delete"),
        (["확인", "검사", "inspect", "check"].as_slice(), "inspect"),
        (["분석", "analyze"].as_slice(), "analyze"),
        (
            ["수정", "수리", "고치", "repair", "fix"].as_slice(),
            "repair",
        ),
        (["작성", "생성", "create", "write"].as_slice(), "create"),
        (["설명", "explain"].as_slice(), "explain"),
        (["요약", "summarize"].as_slice(), "summarize"),
    ]
    .iter()
    .find(|(aliases, _)| aliases.iter().any(|alias| lower.contains(alias)))
    .map(|(_, surface)| *surface)
    .unwrap_or_else(|| match option.intent {
        Some(PlanIntentIR::Explain) => "explain",
        Some(PlanIntentIR::Investigate) => "inspect",
        Some(PlanIntentIR::Repair) => "repair",
        Some(PlanIntentIR::Create) => "create",
        _ => "execute",
    });
    format!("{action} {subject}")
}

fn invalid_question_answer(answer_text: &str) -> QuestionAnswerResolutionIR {
    QuestionAnswerResolutionIR {
        disposition: QuestionAnswerDispositionIR::InvalidOrNonAuthoritative,
        resolved_semantic_text: answer_text.to_string(),
        binding: None,
    }
}

fn ordinal_answer_index(text: &str) -> Option<usize> {
    let compact = text.replace([' ', '-', '_'], "");
    for (index, surfaces) in [
        ["첫번째", "첫째", "first", "thefirstone", "optionone"].as_slice(),
        ["두번째", "둘째", "second", "thesecondone", "optiontwo"].as_slice(),
        ["세번째", "셋째", "third", "thethirdone", "optionthree"].as_slice(),
    ]
    .iter()
    .enumerate()
    {
        if surfaces.iter().any(|surface| compact == *surface) {
            return Some(index);
        }
    }
    None
}

fn question_option_score(answer: &str, option: &QuestionOptionIR) -> usize {
    let option_text = format!(
        "{} {}",
        option.display_surface.to_lowercase(),
        option.resolved_semantic_text.to_lowercase()
    );
    let answer_concepts = selection_concepts(answer);
    let option_concepts = selection_concepts(&option_text);
    let concept_score = answer_concepts.intersection(&option_concepts).count() * 8;
    let lexical_score = answer
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.chars().count() >= 2)
        .filter(|term| {
            ![
                "the", "one", "option", "action", "fact", "쪽", "선택", "사실", "하는", "한다",
                "대한", "that",
            ]
            .contains(term)
        })
        .filter(|term| option_text.contains(term))
        .count();
    concept_score + lexical_score
}

fn selection_concepts(text: &str) -> BTreeSet<&'static str> {
    let lower = text.to_lowercase();
    [
        ("FILE", ["파일", "file"].as_slice()),
        ("FOLDER", ["폴더", "folder"].as_slice()),
        ("CACHE", ["캐시", "cache"].as_slice()),
        ("CASH", ["가시", "cash"].as_slice()),
        ("QUEUE", ["큐", "queue"].as_slice()),
        ("BACKUP", ["백업", "backup"].as_slice()),
        ("LOG", ["로그", "log"].as_slice()),
        ("SERVER", ["서버", "server"].as_slice()),
        ("WORKER", ["워커", "worker"].as_slice()),
        ("CODE", ["코드", "code"].as_slice()),
        ("DOCUMENT", ["문서", "document"].as_slice()),
        ("BUILD", ["빌드", "build"].as_slice()),
        ("TEST", ["테스트", "test"].as_slice()),
        ("ROLLOUT", ["배포", "rollout"].as_slice()),
        ("API", ["api"].as_slice()),
        ("READ", ["읽", "read"].as_slice()),
        ("SAVE", ["저장", "save"].as_slice()),
        ("INSPECT", ["확인", "검사", "inspect", "check"].as_slice()),
        ("ANALYZE", ["분석", "analyze"].as_slice()),
        ("DELETE", ["삭제", "지우", "delete", "clear"].as_slice()),
        (
            "REPAIR",
            ["수정", "수리", "고치", "repair", "fix"].as_slice(),
        ),
        ("CREATE", ["작성", "생성", "create", "write"].as_slice()),
        ("SUMMARY", ["요약", "summarize"].as_slice()),
    ]
    .into_iter()
    .filter_map(|(concept, aliases)| {
        aliases
            .iter()
            .any(|alias| lower.contains(alias))
            .then_some(concept)
    })
    .collect()
}

fn contains_any_surface(text: &str, surfaces: &[&str]) -> bool {
    surfaces.iter().any(|surface| text.contains(surface))
}

fn known_subject_in_fragment(text: &str) -> Option<(String, String)> {
    let aliases = [
        ("source code", "C_OBJECT_SOURCE_CODE"),
        ("repository", "C_OBJECT_REPOSITORY"),
        ("캐시", "TOPIC_CACHE"),
        ("큐", "TOPIC_QUEUE"),
        ("백업", "TOPIC_BACKUP"),
        ("로그", "TOPIC_LOG"),
        ("서버", "TOPIC_SERVER"),
        ("워커", "TOPIC_WORKER"),
        ("프로젝트", "C_OBJECT_PROJECT"),
        ("저장소", "C_OBJECT_REPOSITORY"),
        ("보고서", "C_OBJECT_REPORT"),
        ("폴더", "C_OBJECT_FOLDER"),
        ("파일", "C_OBJECT_FILE"),
        ("문서", "C_OBJECT_DOCUMENT"),
        ("코드", "C_OBJECT_SOURCE_CODE"),
        ("오류", "C_OBJECT_DEFECT"),
        ("계획", "C_OBJECT_PLAN"),
        ("project", "C_OBJECT_PROJECT"),
        ("report", "C_OBJECT_REPORT"),
        ("folder", "C_OBJECT_FOLDER"),
        ("file", "C_OBJECT_FILE"),
        ("document", "C_OBJECT_DOCUMENT"),
        ("code", "C_OBJECT_SOURCE_CODE"),
        ("error", "C_OBJECT_DEFECT"),
        ("plan", "C_OBJECT_PLAN"),
        ("cache", "TOPIC_CACHE"),
        ("queue", "TOPIC_QUEUE"),
        ("backup", "TOPIC_BACKUP"),
        ("log", "TOPIC_LOG"),
        ("server", "TOPIC_SERVER"),
        ("worker", "TOPIC_WORKER"),
    ];
    aliases
        .iter()
        .filter_map(|(surface, concept)| {
            text.rfind(surface)
                .map(|position| (position, (*surface).to_string(), (*concept).to_string()))
        })
        .max_by_key(|(position, _, _)| *position)
        .map(|(_, surface, concept)| (surface, concept))
}

fn open_predicate_ellipsis_subject(text: &str) -> Option<String> {
    let normalized = text
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim();
    let candidate = if let Some(rest) = normalized.strip_prefix("do the same operation for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("do the same for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("apply the same procedure to ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("apply that procedure to ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("apply that workflow to ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("apply the same workflow to ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("apply the same to ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("use the same procedure for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("use that procedure for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("use the same guarded procedure for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("repeat that workflow for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("repeat the workflow for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("same operation for ") {
        rest
    } else if let Some(rest) = normalized.strip_prefix("same for ") {
        rest
    } else if let Some(rest) = normalized.strip_suffix(" too") {
        rest
    } else if let Some((subject, remainder)) = normalized.split_once("도 ") {
        if remainder.contains("똑같이")
            || remainder.contains("같은 방식")
            || remainder.contains("같은 절차")
            || remainder.contains("그렇게")
        {
            subject
        } else {
            return None;
        }
    } else {
        normalized.strip_suffix('도')?
    };
    let candidate = candidate.trim();
    let candidate = candidate
        .strip_suffix(" as well")
        .unwrap_or(candidate)
        .trim_start_matches("the ")
        .trim();
    let token_count = candidate.split_whitespace().count();
    let safe = !candidate.is_empty()
        && candidate.chars().count() <= 64
        && token_count <= 6
        && candidate
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, ' ' | '-' | '_'))
        && ![
            "same",
            "operation",
            "too",
            "같은 방식",
            "같은 절차",
            "똑같이",
            "그렇게",
            "고마워",
            "thanks",
        ]
        .contains(&candidate);
    safe.then(|| candidate.to_string())
}

fn repeat_goal_in_current_language(goal: &ConversationGoalFrameIR, current_text: &str) -> String {
    let current_is_english = text_is_english(current_text);
    let source_is_english = text_is_english(&goal.source_semantic_text);
    if current_is_english == source_is_english {
        return goal.source_semantic_text.clone();
    }
    known_subject_in_fragment(&goal.subject)
        .map(|(surface, concept)| {
            render_goal_for_subject(goal, &surface, Some(&concept), current_text)
        })
        .unwrap_or_else(|| goal.source_semantic_text.clone())
}

fn render_goal_for_subject(
    goal: &ConversationGoalFrameIR,
    replacement_surface: &str,
    concept_id: Option<&str>,
    current_text: &str,
) -> String {
    let english = text_is_english(current_text);
    let subject = concept_id
        .and_then(|concept| concept_surface(concept, english))
        .unwrap_or(replacement_surface);
    let action = localized_action_surface(goal, english);
    if english {
        format!("{action} {subject}")
    } else {
        format!("{subject}{} {action}", object_particle(subject))
    }
}

fn localized_action_surface(goal: &ConversationGoalFrameIR, english: bool) -> String {
    let form = goal.predicate_surface.to_lowercase();
    if english {
        for (needle, realization) in [
            ("열", "open"),
            ("읽", "read"),
            ("변환", "transform"),
            ("저장", "save"),
            ("삭제", "delete"),
            ("지우", "delete"),
            ("배포", "deploy"),
            ("실행", "run"),
            ("확인", "check"),
            ("검사", "inspect"),
            ("조사", "inspect"),
            ("분석", "analyze"),
            ("검증", "verify"),
            ("고치", "fix"),
            ("고쳐", "fix"),
            ("수정", "fix"),
            ("수리", "repair"),
            ("복구", "restore"),
            ("만들", "create"),
            ("작성", "write"),
            ("생성", "create"),
            ("설명", "explain"),
            ("해설", "explain"),
            ("기록", "record"),
            ("전달", "tell"),
            ("말해", "tell"),
            ("학습", "learn"),
        ] {
            if form.contains(needle) {
                return realization.to_string();
            }
        }
        if form
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        {
            return form;
        }
        return goal.canonical_predicate.to_lowercase();
    }
    for (needle, realization) in [
        ("open", "열어"),
        ("read", "읽어"),
        ("transform", "변환해"),
        ("convert", "변환해"),
        ("save", "저장해"),
        ("delete", "삭제해"),
        ("clear", "지워"),
        ("deploy", "배포해"),
        ("run", "실행해"),
        ("check", "확인해"),
        ("inspect", "확인해"),
        ("analyze", "분석해"),
        ("verify", "검증해"),
        ("validate", "검증해"),
        ("fix", "고쳐"),
        ("repair", "수리해"),
        ("restore", "복구해"),
        ("create", "만들어"),
        ("write", "작성해"),
        ("explain", "설명해"),
        ("record", "기록해"),
        ("report", "보고해"),
        ("tell", "말해"),
        ("learn", "학습해"),
    ] {
        if form.contains(needle) {
            return realization.to_string();
        }
    }
    if !form.is_empty()
        && !form
            .chars()
            .any(|character| character.is_ascii_alphabetic())
    {
        if form.ends_with(['어', '아', '해', '줘'])
            || [
                "열어",
                "알려",
                "찾아",
                "고쳐",
                "만들어",
                "배워",
                "옮겨",
                "이어가",
                "말해",
            ]
            .contains(&form.as_str())
        {
            return form;
        }
        return format!("{form}해");
    }
    match goal.intent {
        PlanIntentIR::Investigate => "확인해",
        PlanIntentIR::Repair => "수정해",
        PlanIntentIR::Create => "만들어",
        PlanIntentIR::Explain => "설명해",
        PlanIntentIR::Communicate => "전달해",
        PlanIntentIR::Learn => "학습해",
        _ => "실행해",
    }
    .to_string()
}

fn text_is_english(text: &str) -> bool {
    let ascii_letters = text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    let hangul = text
        .chars()
        .filter(|character| ('\u{ac00}'..='\u{d7a3}').contains(character))
        .count();
    ascii_letters > hangul
}

fn token_parts(token: &str) -> (&str, &str, &str) {
    let start = token
        .char_indices()
        .find(|(_, character)| !is_token_delimiter(*character))
        .map_or(token.len(), |(index, _)| index);
    let end = token
        .char_indices()
        .rev()
        .find(|(_, character)| !is_token_delimiter(*character))
        .map_or(start, |(index, character)| index + character.len_utf8());
    (&token[..start], &token[start..end], &token[end..])
}

fn is_token_delimiter(character: char) -> bool {
    character.is_ascii_punctuation()
        || matches!(character, '‘' | '’' | '“' | '”' | '「' | '」' | '『' | '』')
}

fn is_plural_reference_surface(token: &str) -> bool {
    matches!(
        token.to_lowercase().as_str(),
        "그것들" | "그것들을" | "그것들이" | "그거들" | "them" | "those"
    )
}

fn ordered_referent<'a>(
    token: &str,
    latest: &[&'a DynamicReferentIR],
) -> Option<&'a DynamicReferentIR> {
    if latest.len() < 2 {
        return None;
    }
    let mut ordered = latest.to_vec();
    ordered.sort_by(|left, right| left.referent_id.cmp(&right.referent_id));
    match token.to_lowercase().as_str() {
        "전자" | "전자를" | "former" => ordered.first().copied(),
        "후자" | "후자를" | "latter" => ordered.last().copied(),
        _ => None,
    }
}

fn localized_referent_surface(referent: &DynamicReferentIR, text: &str) -> String {
    concept_surface(&referent.canonical_concept, text_is_english(text))
        .unwrap_or(&referent.surface)
        .to_string()
}

fn concept_surface(concept_id: &str, english: bool) -> Option<&'static str> {
    match (concept_id, english) {
        ("C_OBJECT_FILE", false) => Some("파일"),
        ("C_OBJECT_FOLDER", false) => Some("폴더"),
        ("C_OBJECT_SOURCE_CODE", false) => Some("코드"),
        ("C_OBJECT_DOCUMENT", false) => Some("문서"),
        ("C_OBJECT_REPORT", false) => Some("보고서"),
        ("C_OBJECT_DEFECT", false) => Some("오류"),
        ("C_OBJECT_PROJECT", false) => Some("프로젝트"),
        ("C_OBJECT_REPOSITORY", false) => Some("저장소"),
        ("C_OBJECT_PLAN", false) => Some("계획"),
        ("TOPIC_CACHE", false) => Some("캐시"),
        ("TOPIC_QUEUE", false) => Some("큐"),
        ("TOPIC_BACKUP", false) => Some("백업"),
        ("TOPIC_LOG", false) => Some("로그"),
        ("TOPIC_SERVER", false) => Some("서버"),
        ("TOPIC_WORKER", false) => Some("워커"),
        ("C_OBJECT_FILE", true) => Some("file"),
        ("C_OBJECT_FOLDER", true) => Some("folder"),
        ("C_OBJECT_SOURCE_CODE", true) => Some("code"),
        ("C_OBJECT_DOCUMENT", true) => Some("document"),
        ("C_OBJECT_REPORT", true) => Some("report"),
        ("C_OBJECT_DEFECT", true) => Some("error"),
        ("C_OBJECT_PROJECT", true) => Some("project"),
        ("C_OBJECT_REPOSITORY", true) => Some("repository"),
        ("C_OBJECT_PLAN", true) => Some("plan"),
        ("TOPIC_CACHE", true) => Some("cache"),
        ("TOPIC_QUEUE", true) => Some("queue"),
        ("TOPIC_BACKUP", true) => Some("backup"),
        ("TOPIC_LOG", true) => Some("log"),
        ("TOPIC_SERVER", true) => Some("server"),
        ("TOPIC_WORKER", true) => Some("worker"),
        _ => None,
    }
}

fn realize_plural_reference(reference: &str, surfaces: &[String]) -> String {
    if text_is_english(reference) {
        return surfaces.join(" and ");
    }
    let mut phrase = surfaces.first().cloned().unwrap_or_default();
    for surface in surfaces.iter().skip(1) {
        phrase.push_str(if has_final_consonant(&phrase) {
            "과 "
        } else {
            "와 "
        });
        phrase.push_str(surface);
    }
    if matches!(reference, "그것들을") {
        phrase.push_str(object_particle(&phrase));
    } else if matches!(reference, "그것들이") {
        phrase.push_str(subject_particle(&phrase));
    }
    phrase
}

fn reference_surfaces(text: &str) -> Vec<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    tokens
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !english_that_is_complementizer(&tokens, *index)
                && !english_local_pronoun(&tokens, *index)
        })
        .map(|(_, token)| token_parts(token).1)
        .filter(|token| {
            is_reference_surface(token)
                || is_plural_reference_surface(token)
                || matches!(
                    token.to_lowercase().as_str(),
                    "전자" | "전자를" | "후자" | "후자를" | "former" | "latter"
                )
        })
        .map(ToString::to_string)
        .collect()
}

fn english_local_pronoun(tokens: &[&str], index: usize) -> bool {
    let Some(surface) = tokens
        .get(index)
        .map(|token| token_parts(token).1.to_lowercase())
    else {
        return false;
    };
    let explanation_recipient = matches!(
        surface.as_str(),
        "me" | "us" | "you" | "them" | "him" | "her"
    ) && index > 0
        && matches!(
            token_parts(tokens[index - 1]).1.to_lowercase().as_str(),
            "walk" | "talk"
        );
    if explanation_recipient {
        let next = tokens
            .get(index + 1)
            .map(|token| token_parts(token).1.to_lowercase());
        if next.as_deref() == Some("through") {
            return true;
        }
    }
    let local_process_object = surface == "them"
        && index > 0
        && matches!(
            token_parts(tokens[index - 1]).1.to_lowercase().as_str(),
            "apply" | "applies" | "applying" | "execute" | "executing" | "perform" | "performing"
        )
        && tokens[..index].iter().any(|token| {
            matches!(
                token_parts(token).1.to_lowercase().as_str(),
                "step" | "steps" | "check" | "checks" | "action" | "actions"
            )
        });
    if local_process_object {
        return true;
    }
    if matches!(surface.as_str(), "it" | "that") && continuation_task_anaphor_at(tokens, index) {
        return true;
    }
    if matches!(surface.as_str(), "this" | "that") {
        let next = tokens
            .get(index + 1)
            .map(|token| token_parts(token).1.to_lowercase());
        return next.is_some_and(|head| {
            !matches!(
                head.as_str(),
                "again" | "now" | "then" | "please" | "too" | "instead"
            ) && !matches!(
                head.as_str(),
                "result"
                    | "outcome"
                    | "action"
                    | "claim"
                    | "statement"
                    | "proposal"
                    | "plan"
                    | "idea"
                    | "event"
                    | "change"
                    | "decision"
            )
        });
    }
    if surface != "it" {
        return false;
    }
    if index > 0
        && token_parts(tokens[index - 1]).1.eq_ignore_ascii_case("if")
        && tokens[..index].windows(2).any(|pair| {
            token_parts(pair[0]).1.eq_ignore_ascii_case("keep") && {
                let action = token_parts(pair[1]).1.to_lowercase();
                action.ends_with("ing")
                    && action.len() > "ing".len()
                    && !matches!(
                        action.as_str(),
                        "thing" | "something" | "anything" | "nothing"
                    )
            }
        })
    {
        return true;
    }
    if continuation_task_anaphor_at(tokens, index) {
        return true;
    }
    if english_it_is_expletive(tokens, index) {
        return true;
    }
    let previous = index
        .checked_sub(1)
        .and_then(|prior| tokens.get(prior))
        .map(|token| token_parts(token).1.to_lowercase());
    let next = tokens
        .get(index + 1)
        .map(|token| token_parts(token).1.to_lowercase());
    let two_back = index
        .checked_sub(2)
        .and_then(|prior| tokens.get(prior))
        .map(|token| token_parts(token).1.to_lowercase());
    let prohibited_local_object = two_back.as_deref() == Some("not")
        && previous.as_deref().is_some_and(|word| {
            matches!(
                word,
                "publish"
                    | "delete"
                    | "remove"
                    | "deploy"
                    | "run"
                    | "execute"
                    | "modify"
                    | "change"
                    | "save"
                    | "send"
            )
        });
    let coordinated_local_predicate = previous.as_deref() == Some("and")
        && next.is_some_and(|word| matches!(word.as_str(), "is" | "was" | "feels" | "seems"));
    let local_response_antecedent = tokens[..index].iter().any(|token| {
        matches!(
            token_parts(token).1.to_lowercase().as_str(),
            "answer" | "response" | "explanation"
        )
    });
    let local_response_action = previous.is_some_and(|word| {
        matches!(
            word.as_str(),
            "explain" | "rewrite" | "summarize" | "shorten" | "revise" | "clarify"
        )
    });
    prohibited_local_object
        || coordinated_local_predicate
        || (local_response_antecedent && local_response_action)
}

fn english_that_is_complementizer(tokens: &[&str], index: usize) -> bool {
    let Some(token) = tokens.get(index) else {
        return false;
    };
    if !token_parts(token).1.eq_ignore_ascii_case("that") {
        return false;
    }
    let prior = index
        .checked_sub(1)
        .and_then(|prior| tokens.get(prior))
        .map(|token| token_parts(token).1.to_lowercase());
    prior.is_some_and(|word| {
        [
            "say",
            "says",
            "said",
            "state",
            "states",
            "stated",
            "report",
            "reports",
            "reported",
            "claim",
            "claims",
            "claimed",
            "believe",
            "believes",
            "believed",
            "think",
            "thinks",
            "thought",
            "know",
            "knows",
            "knew",
            "doubt",
            "doubts",
            "doubted",
            "deny",
            "denies",
            "denied",
            "hear",
            "hears",
            "heard",
            "observe",
            "observes",
            "observed",
            "infer",
            "infers",
            "inferred",
            "want",
            "wants",
            "wanted",
            "expect",
            "expects",
            "expected",
            "correct",
            "corrects",
            "corrected",
        ]
        .contains(&word.as_str())
    })
}

fn reconstruct_semantic_surface(
    source: &str,
    tokens: &[String],
    replacements: &[Option<String>],
) -> String {
    debug_assert_eq!(tokens.len(), replacements.len());
    let mut output = String::new();
    let mut cursor = 0;
    for (token, replacement) in tokens.iter().zip(replacements) {
        let Some(offset) = source[cursor..].find(token) else {
            continue;
        };
        let start = cursor + offset;
        if let Some(replacement) = replacement {
            append_semantic_separator(&mut output, &source[cursor..start]);
            output.push_str(replacement);
        }
        cursor = start + token.len();
    }
    append_semantic_separator(&mut output, &source[cursor..]);
    output.trim().to_string()
}

fn append_semantic_separator(output: &mut String, separator: &str) {
    let punctuation = separator
        .chars()
        .filter(|character| {
            matches!(
                character,
                ',' | '.'
                    | '!'
                    | '?'
                    | ';'
                    | ':'
                    | '…'
                    | '‘'
                    | '’'
                    | '“'
                    | '”'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '('
                    | ')'
                    | '"'
                    | '\''
            )
        })
        .collect::<String>();
    if output.is_empty() {
        let openings = punctuation
            .chars()
            .filter(|character| matches!(character, '‘' | '“' | '「' | '『' | '(' | '"' | '\''))
            .collect::<String>();
        output.push_str(&openings);
        return;
    }
    if separator.chars().next().is_some_and(char::is_whitespace)
        && !output.chars().next_back().is_some_and(char::is_whitespace)
    {
        output.push(' ');
    }
    output.push_str(&punctuation);
    if separator
        .chars()
        .next_back()
        .is_some_and(char::is_whitespace)
        && !output.chars().next_back().is_some_and(char::is_whitespace)
    {
        output.push(' ');
    }
}

fn is_reference_surface(token: &str) -> bool {
    matches!(
        token.to_lowercase().as_str(),
        "그거"
            | "그것"
            | "그걸"
            | "그것을"
            | "그게"
            | "그것이"
            | "그거에"
            | "그것에"
            | "it"
            | "that"
            | "this"
    )
}

fn realize_reference(reference: &str, surface: &str) -> String {
    match reference {
        "그걸" | "그것을" | "전자를" | "후자를" => {
            format!("{surface}{}", object_particle(surface))
        }
        "그게" | "그것이" => format!("{surface}{}", subject_particle(surface)),
        "그거에" | "그것에" => format!("{surface}에"),
        _ => surface.to_string(),
    }
}

fn has_final_consonant(value: &str) -> bool {
    value.chars().next_back().is_some_and(|character| {
        let code = u32::from(character);
        (0xac00..=0xd7a3).contains(&code) && (code - 0xac00) % 28 != 0
    })
}

fn object_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "을"
    } else {
        "를"
    }
}

fn topic_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "은"
    } else {
        "는"
    }
}

fn subject_particle(value: &str) -> &'static str {
    if has_final_consonant(value) {
        "이"
    } else {
        "가"
    }
}

fn extract_referents(subject: &str, turn: u64) -> Vec<DynamicReferentIR> {
    let known = [
        ("파일", "C_OBJECT_FILE"),
        ("폴더", "C_OBJECT_FOLDER"),
        ("코드", "C_OBJECT_SOURCE_CODE"),
        ("문서", "C_OBJECT_DOCUMENT"),
        ("보고서", "C_OBJECT_REPORT"),
        ("오류", "C_OBJECT_DEFECT"),
        ("프로젝트", "C_OBJECT_PROJECT"),
        ("저장소", "C_OBJECT_REPOSITORY"),
        ("계획", "C_OBJECT_PLAN"),
        ("file", "C_OBJECT_FILE"),
        ("folder", "C_OBJECT_FOLDER"),
        ("code", "C_OBJECT_SOURCE_CODE"),
        ("document", "C_OBJECT_DOCUMENT"),
        ("report", "C_OBJECT_REPORT"),
        ("error", "C_OBJECT_DEFECT"),
        ("project", "C_OBJECT_PROJECT"),
        ("repository", "C_OBJECT_REPOSITORY"),
        ("plan", "C_OBJECT_PLAN"),
    ];
    let lower = subject.to_lowercase();
    let mut seen = BTreeSet::new();
    known
        .iter()
        .filter(|(surface, _)| phrase_mentioned(&lower, surface))
        .filter(|(surface, _)| seen.insert(*surface))
        .enumerate()
        .map(|(index, (surface, concept))| DynamicReferentIR {
            referent_id: format!("REF-{turn:06}-{:02}", index + 1),
            surface: (*surface).to_string(),
            canonical_concept: (*concept).to_string(),
            introduced_turn: turn,
            last_referenced_turn: turn,
        })
        .collect()
}

fn phrase_mentioned(text: &str, phrase: &str) -> bool {
    if phrase.is_ascii() {
        text.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|token| token == phrase)
    } else {
        text.contains(phrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str) -> ConversationTurnRequestIR {
        ConversationTurnRequestIR {
            schema: CONVERSATION_TURN_REQUEST_SCHEMA.to_string(),
            conversation_id: "CONV-1".to_string(),
            turn_index: 1,
            request_id: "REQ-1".to_string(),
            modality: ConversationInputModalityIR::Text,
            raw_text: text.to_string(),
            input_confidence_millis: 1_000,
            alternatives: Vec::new(),
            output_language: Some(LanguageCodeIR::Korean),
            context_tags: Vec::new(),
            max_plan_steps: 12,
        }
    }

    fn goal_frame(index: usize, predicate_surface: &str, subject: &str) -> ConversationGoalFrameIR {
        ConversationGoalFrameIR {
            goal_id: format!("GOAL-000001-{index:02}"),
            intent: PlanIntentIR::Investigate,
            canonical_predicate: "INVESTIGATE".to_string(),
            predicate_surface: predicate_surface.to_string(),
            subject: subject.to_string(),
            source_semantic_text: format!("{subject}를 {predicate_surface}해"),
            introduced_turn: 1,
            last_referenced_turn: 1,
            external_execution_authorized: true,
        }
    }

    fn attributed_proposition(
        index: usize,
        source: &str,
        summary: &str,
    ) -> DynamicDiscourseReferentIR {
        DynamicDiscourseReferentIR {
            referent_id: format!("DREF-P-000001-{index:02}"),
            kind: DiscourseReferentKindIR::Proposition,
            topic_id: None,
            semantic_summary: summary.to_string(),
            attributed_source: Some(source.to_string()),
            attribution_attitude: Some(AttributionAttitudeIR::Say),
            epistemic_status: Some(EpistemicStatusIR::Reported),
            proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
            modal_world: Some(ModalWorldIR::Actual),
            belief_record_id: None,
            introduced_turn: 1,
            last_referenced_turn: 1,
            external_execution_authorized: false,
        }
    }

    fn discourse_group_fixture(
        group_id: &str,
        kind: DiscourseGroupKindIR,
        members: &[&str],
        revision: u64,
        component_group_ids: &[&str],
        introduced_turn: u64,
        last_referenced_turn: u64,
    ) -> DiscourseGroupIR {
        let mut group = DiscourseGroupIR {
            group_id: group_id.to_string(),
            kind,
            member_keys: members.iter().map(|member| (*member).to_string()).collect(),
            topic_keys: Vec::new(),
            revision,
            component_group_ids: component_group_ids
                .iter()
                .map(|group_id| (*group_id).to_string())
                .collect(),
            membership_sha256: String::new(),
            introduced_turn,
            last_referenced_turn,
            semantic_authority: false,
            external_execution_authorized: false,
        };
        group.member_keys.sort();
        group.component_group_ids.sort();
        group.membership_sha256 = discourse_group_membership_sha256(&group);
        group
    }

    #[allow(clippy::too_many_arguments)]
    fn discourse_group_update_fixture(
        operation: DiscourseGroupUpdateOperationIR,
        target_group_id: &str,
        source_group_ids: &[&str],
        before_member_keys: &[&str],
        after_member_keys: &[&str],
        added_member_keys: &[&str],
        removed_member_keys: &[&str],
        revision: u64,
    ) -> DiscourseGroupUpdateIR {
        let strings = |values: &[&str]| {
            let mut values = values
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>();
            values.sort();
            values
        };
        let mut update = DiscourseGroupUpdateIR {
            schema: DISCOURSE_GROUP_UPDATE_SCHEMA.to_string(),
            operation,
            applied: true,
            target_group_id: Some(target_group_id.to_string()),
            source_group_ids: strings(source_group_ids),
            before_member_keys: strings(before_member_keys),
            after_member_keys: strings(after_member_keys),
            added_member_keys: strings(added_member_keys),
            removed_member_keys: strings(removed_member_keys),
            revision,
            unresolved_terms: Vec::new(),
            semantic_authority: false,
            external_action_executed: false,
            update_sha256: String::new(),
        };
        update.update_sha256 = discourse_group_update_sha256(&update);
        update
    }

    #[test]
    fn typo_and_hesitation_normalize_without_changing_semantic_authority() {
        let normalized = UtteranceNormalizer
            .normalize(&request("음... 파일 오류를 고처줘"))
            .expect("normalization");
        assert_eq!(normalized.semantic_text, "파일 오류를 고쳐줘");
        assert!(normalized.operations.iter().any(|operation| {
            operation.kind == NormalizationOperationKindIR::KnownTypo
                && operation.before == "고처줘"
        }));
        assert!(normalized
            .discourse_events
            .iter()
            .any(|event| event.function == DiscourseFunctionIR::Hesitation));
        assert_eq!(normalized.semantic_surface_text, "파일 오류를 고쳐줘");
    }

    #[test]
    fn colloquial_second_person_after_auxiliary_normalizes_compositionally() {
        for (surface, expected) in [
            (
                "Um, could ya chek the Knoll service for me?",
                "could you check the knoll service for me?",
            ),
            ("Can ya inspect the cache?", "can you inspect the cache?"),
            ("Would ya repair the queue?", "would you repair the queue?"),
            ("Did ya review the report?", "did you review the report?"),
        ] {
            let normalized = UtteranceNormalizer
                .normalize(&request(surface))
                .expect("colloquial normalization");
            assert_eq!(normalized.semantic_surface_text, expected, "{surface}");
            assert!(normalized.operations.iter().any(|operation| {
                operation.kind == NormalizationOperationKindIR::KnownTypo
                    && operation.before.eq_ignore_ascii_case("ya")
                    && operation.after == "you"
            }));
        }

        let lexical_use = UtteranceNormalizer
            .normalize(&request("Ya is a colloquial spelling under discussion."))
            .expect("metalinguistic lexical use");
        assert_eq!(
            lexical_use.semantic_surface_text,
            "ya is a colloquial spelling under discussion."
        );
        assert!(!lexical_use
            .operations
            .iter()
            .any(|operation| operation.before.eq_ignore_ascii_case("ya")));
    }

    #[test]
    fn unseen_control_typos_require_unique_request_context() {
        for (surface, expected, typo, repaired) in [
            (
                "Will ya insepct the Quartz cache?",
                "will you inspect the quartz cache?",
                "insepct",
                "inspect",
            ),
            (
                "Please revieew the report.",
                "please review the report.",
                "revieew",
                "review",
            ),
            (
                "Reapir the Meadow queue.",
                "repair the meadow queue.",
                "reapir",
                "repair",
            ),
        ] {
            let normalized = UtteranceNormalizer
                .normalize(&request(surface))
                .expect("unique fuzzy request repair");
            assert_eq!(normalized.semantic_surface_text, expected, "{surface}");
            assert!(normalized.operations.iter().any(|operation| {
                operation.kind == NormalizationOperationKindIR::UniqueFuzzyMatch
                    && operation.before.eq_ignore_ascii_case(typo)
                    && operation.after == repaired
            }));
        }

        for surface in [
            "Please fire the worker.",
            "The installer is ready.",
            "We saw a revieew yesterday.",
        ] {
            let normalized = UtteranceNormalizer
                .normalize(&request(surface))
                .expect("open vocabulary control");
            assert!(
                !normalized
                    .operations
                    .iter()
                    .any(|operation| operation.kind
                        == NormalizationOperationKindIR::UniqueFuzzyMatch),
                "{surface}: {:#?}",
                normalized.operations
            );
        }
    }

    #[test]
    fn semantic_surface_keeps_clause_and_quote_scope_after_noise_removal() {
        let normalized = UtteranceNormalizer
            .normalize(&request(
                "음... ‘코드를 고처줘’라는 표현을 설명해. 왜 그런지도 확인해?",
            ))
            .expect("normalization");
        assert_eq!(
            normalized.semantic_surface_text,
            "‘코드를 고쳐줘’라는 표현을 설명해. 왜 그런지도 확인해?"
        );
    }

    #[test]
    fn semantic_surface_preserves_parentheses_and_report_particle_forms() {
        let normalized = UtteranceNormalizer
            .normalize(&request(
                "모든 작업에서 (캐시 또는 큐) 중 완료 보고가 있는 작업을 보여줘",
            ))
            .expect("normalization");
        assert_eq!(
            normalized.semantic_surface_text,
            "모든 작업에서 (캐시 또는 큐) 중 완료 보고가 있는 작업을 보여줘"
        );
        assert!(!normalized.operations.iter().any(|operation| {
            operation.kind == NormalizationOperationKindIR::UniqueFuzzyMatch
                && operation.before == "보고가"
        }));

        let object_particle = UtteranceNormalizer
            .normalize(&request("완료 보고를 가진 작업을 보여줘"))
            .expect("normalization");
        assert_eq!(
            object_particle.semantic_surface_text,
            "완료 보고를 가진 작업을 보여줘"
        );

        let conjugated = UtteranceNormalizer
            .normalize(&request("완료됐다고 보고된 작업을 보여줘"))
            .expect("normalization");
        assert_eq!(
            conjugated.semantic_surface_text,
            "완료됐다고 보고된 작업을 보여줘"
        );
    }

    #[test]
    fn short_valid_korean_word_is_not_rewritten_as_a_control_verb() {
        let normalized = UtteranceNormalizer
            .normalize(&request("실제 코딩 능력과 실제 커버리지를 확인해"))
            .expect("normalization");
        assert_eq!(
            normalized.semantic_text,
            "실제 코딩 능력과 실제 커버리지를 확인해"
        );
        assert!(!normalized
            .operations
            .iter()
            .any(|operation| operation.before == "실제"));
    }

    #[test]
    fn discourse_selector_is_not_fuzzily_rewritten_as_an_object_noun() {
        let normalized = UtteranceNormalizer
            .normalize(&request("Revisit the older pair's reports."))
            .expect("normalization");
        assert_eq!(
            normalized.semantic_surface_text,
            "revisit the older pair's reports."
        );
        assert!(!normalized.operations.iter().any(|operation| {
            operation.before == "older"
                && operation.kind == NormalizationOperationKindIR::UniqueFuzzyMatch
        }));
    }

    #[test]
    fn open_vocabulary_minimal_pair_is_not_rewritten_as_a_control_word() {
        let normalized = UtteranceNormalizer
            .normalize(&request("The opal room is on fire"))
            .expect("normalization");
        assert_eq!(normalized.semantic_surface_text, "the opal room is on fire");
        assert!(!normalized.operations.iter().any(|operation| {
            operation.before == "fire"
                && operation.kind == NormalizationOperationKindIR::UniqueFuzzyMatch
        }));
    }

    #[test]
    fn explicit_self_repair_keeps_only_the_corrected_content() {
        let normalized = UtteranceNormalizer
            .normalize(&request("파일을, 아니 폴더를 열어"))
            .expect("normalization");
        assert_eq!(normalized.semantic_text, "폴더를 열어");
        assert!(normalized
            .discourse_events
            .iter()
            .any(|event| event.function == DiscourseFunctionIR::SelfRepair));
    }

    #[test]
    fn fillers_and_backchannels_do_not_create_fake_goals() {
        let hold = UtteranceNormalizer
            .normalize(&request("음..."))
            .expect("hold");
        assert_eq!(hold.disposition, ConversationTurnDispositionIR::HoldFloor);
        assert!(hold.semantic_text.is_empty());
        let acknowledge = UtteranceNormalizer
            .normalize(&request("응"))
            .expect("acknowledge");
        assert_eq!(
            acknowledge.disposition,
            ConversationTurnDispositionIR::BackchannelOnly
        );

        for surface in [
            "그래",
            "좋아",
            "yes",
            "good",
            "one moment",
            "잠깐 생각할게",
            "you're welcome",
            "별말씀을",
        ] {
            let normalized = UtteranceNormalizer
                .normalize(&request(surface))
                .expect("standalone discourse phrase");
            assert!(
                matches!(
                    normalized.disposition,
                    ConversationTurnDispositionIR::HoldFloor
                        | ConversationTurnDispositionIR::BackchannelOnly
                ),
                "{surface} must remain a non-content discourse turn"
            );
            assert!(
                normalized.semantic_text.is_empty(),
                "{surface} must not replace the active discourse focus"
            );
        }
    }

    #[test]
    fn social_phrases_are_dialogue_acts_not_world_model_goals() {
        for (surface, function) in [
            ("안녕", DiscourseFunctionIR::Greeting),
            ("고마워", DiscourseFunctionIR::Gratitude),
            ("bye", DiscourseFunctionIR::Farewell),
        ] {
            let normalized = UtteranceNormalizer
                .normalize(&request(surface))
                .expect("social act");
            assert_eq!(
                normalized.disposition,
                ConversationTurnDispositionIR::BackchannelOnly
            );
            assert!(normalized.semantic_text.is_empty());
            assert!(normalized
                .discourse_events
                .iter()
                .any(|event| event.function == function));
        }
    }

    #[test]
    fn korean_audit_nominal_is_not_removed_as_gratitude() {
        let normalized = UtteranceNormalizer
            .normalize(&request("감사 추적을 확인해"))
            .expect("audit phrase");
        assert!(normalized.semantic_text.contains("감사 추적"));
        assert!(!normalized
            .discourse_events
            .iter()
            .any(|event| event.function == DiscourseFunctionIR::Gratitude));
    }

    #[test]
    fn onomatopoeia_maps_to_event_properties_not_new_surface_concepts() {
        let normalized = UtteranceNormalizer
            .normalize(&request("쿵 소리가 났어"))
            .expect("onomatopoeia");
        assert!(normalized
            .semantic_tags
            .contains(&"impact_sound".to_string()));
        assert!(normalized.semantic_text.contains("쿵"));
    }

    #[test]
    fn close_voice_candidates_require_clarification() {
        let mut voice = request("파일을 열어");
        voice.modality = ConversationInputModalityIR::VoiceTranscript;
        voice.input_confidence_millis = 780;
        voice.alternatives = vec![UtteranceAlternativeIR {
            text: "파일을 얼어".to_string(),
            confidence_millis: 750,
        }];
        let normalized = UtteranceNormalizer.normalize(&voice).expect("voice");
        assert!(normalized.ambiguous_input);
        assert_eq!(
            normalized.disposition,
            ConversationTurnDispositionIR::ClarificationRequired
        );
    }

    #[test]
    fn conversation_reference_resolves_from_dynamic_state() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 열어");
        memory
            .commit_turn(
                &first,
                Some("파일을 열어"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
            )
            .expect("first turn");
        let resolved = memory.resolve_references("CONV-1", "그걸 수정해");
        assert_eq!(resolved.resolved_semantic_text, "파일을 수정해");
        assert_eq!(resolved.resolved_reference_count, 1);
    }

    #[test]
    fn cross_language_reference_uses_concept_alias_not_prior_surface() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 열어");
        memory
            .commit_turn(
                &first,
                Some("파일을 열어"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
            )
            .expect("first turn");
        let resolved = memory.resolve_references("CONV-1", "fix it.");
        assert_eq!(resolved.resolved_semantic_text, "fix file.");
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::PronominalReference
        );
    }

    #[test]
    fn plural_and_ordered_references_use_the_introduced_entity_set() {
        let mut memory = ConversationMemory::default();
        let first = request("파일과 폴더를 확인해");
        memory
            .commit_turn(
                &first,
                Some("파일과 폴더를 확인해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
            )
            .expect("first turn");
        let plural = memory.resolve_references("CONV-1", "그것들을 저장해");
        assert_eq!(plural.resolved_semantic_text, "파일과 폴더를 저장해");
        assert_eq!(plural.used_referent_ids.len(), 2);
        let latter = memory.resolve_references("CONV-1", "후자를 저장해");
        assert_eq!(latter.resolved_semantic_text, "폴더를 저장해");
    }

    #[test]
    fn explicit_parallel_ellipsis_inherits_one_typed_goal() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 확인해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("파일을 확인해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "파일")],
            )
            .expect("first turn");
        let resolved = memory.resolve_references("CONV-1", "문서도");
        assert_eq!(resolved.resolved_semantic_text, "문서를 확인해");
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::EllipticalAction
        );
    }

    #[test]
    fn cross_turn_event_ordinal_selects_one_goal_and_fails_closed_out_of_range() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("파일을 분석하고 폴더를 수리한 뒤 보고서를 저장해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[
                    goal_frame(1, "분석", "파일"),
                    goal_frame(2, "수리", "폴더"),
                    goal_frame(3, "저장", "보고서"),
                ],
            )
            .expect("event sequence turn");

        let second = memory.resolve_references("CONV-1", "두 번째 작업을 설명해");
        assert_eq!(second.resolved_reference_count, 1);
        assert_eq!(
            second.discourse_bindings[0].kind,
            DiscourseBindingKindIR::EventOrdinalReference
        );
        assert!(second.resolved_semantic_text.contains("폴더"));
        assert!(!second.resolved_semantic_text.contains("파일"));
        assert!(!second.resolved_semantic_text.contains("보고서"));

        let fourth = memory.resolve_references("CONV-1", "네 번째 작업을 설명해");
        assert_eq!(fourth.resolved_reference_count, 0);
        assert_eq!(
            fourth.ambiguous_reference_surfaces,
            vec!["EVENT_SEQUENCE_ORDINAL"]
        );
    }

    #[test]
    fn repeating_multiple_active_goals_fails_closed() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 읽고 저장해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("파일을 읽고 저장해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "읽", "파일"), goal_frame(2, "저장", "파일")],
            )
            .expect("first turn");
        let resolved = memory.resolve_references("CONV-1", "그대로 해");
        assert_eq!(resolved.resolved_semantic_text, "그대로 해");
        assert_eq!(
            resolved.ambiguous_reference_surfaces,
            vec!["ELLIPTICAL_ACTION"]
        );
    }

    #[test]
    fn explicit_topic_return_repeats_one_superseded_topic_action() {
        let mut memory = ConversationMemory::default();
        let mut first = request("nexel the cache.");
        first.output_language = Some(LanguageCodeIR::English);
        let mut cache_goal = goal_frame(1, "nexel", "cache");
        cache_goal.source_semantic_text = "nexel the cache.".to_string();
        memory
            .commit_turn_with_goals(
                &first,
                Some("cache"),
                &[],
                0,
                Some(LanguageCodeIR::English),
                &[cache_goal],
            )
            .expect("cache action");

        let mut second = request("inspect the server.");
        second.turn_index = 2;
        second.request_id = "REQ-2".to_string();
        second.output_language = Some(LanguageCodeIR::English);
        let mut server_goal = goal_frame(1, "inspect", "server");
        server_goal.goal_id = "GOAL-000002-01".to_string();
        server_goal.subject = "server".to_string();
        server_goal.source_semantic_text = "inspect the server.".to_string();
        server_goal.introduced_turn = 2;
        server_goal.last_referenced_turn = 2;
        memory
            .commit_turn_with_goals(
                &second,
                Some("server"),
                &[],
                0,
                Some(LanguageCodeIR::English),
                &[server_goal],
            )
            .expect("server action");

        let mut third = request("Back to the cache topic.");
        third.turn_index = 3;
        third.request_id = "REQ-3".to_string();
        third.output_language = Some(LanguageCodeIR::English);
        memory
            .commit_turn(&third, None, &[], 0, Some(LanguageCodeIR::English))
            .expect("topic-return turn");
        let transition = detect_topic_transition(&third.raw_text).expect("topic return");
        memory
            .apply_topic_transition("CONV-1", &transition, 3)
            .expect("explicit cache topic");

        let resolved = memory.resolve_references("CONV-1", "Do it again.");
        assert_eq!(resolved.resolved_semantic_text, "nexel the cache.");
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::RepeatedGoal
        );
        assert_eq!(
            resolved.discourse_bindings[0].inherited_goal_id.as_deref(),
            Some("GOAL-000001-01")
        );
        assert!(resolved.discourse_bindings[0]
            .evidence
            .contains(&"GOAL_INHERITANCE:EXPLICIT_TOPIC_ACTION_LEDGER".to_string()));
    }

    #[test]
    fn explicit_topic_repeat_with_two_action_records_fails_closed() {
        let mut memory = ConversationMemory::default();
        let first = request("캐시를 확인해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("캐시"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "캐시")],
            )
            .expect("first cache action");

        let mut second = request("캐시를 수리해");
        second.turn_index = 2;
        second.request_id = "REQ-2".to_string();
        let mut second_goal = goal_frame(1, "수리", "캐시");
        second_goal.goal_id = "GOAL-000002-01".to_string();
        second_goal.intent = PlanIntentIR::Repair;
        second_goal.canonical_predicate = "REPAIR".to_string();
        second_goal.source_semantic_text = "캐시를 수리해".to_string();
        second_goal.introduced_turn = 2;
        second_goal.last_referenced_turn = 2;
        memory
            .commit_turn_with_goals(
                &second,
                Some("캐시"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[second_goal],
            )
            .expect("second cache action");

        let mut third = request("캐시로 돌아가자");
        third.turn_index = 3;
        third.request_id = "REQ-3".to_string();
        memory
            .commit_turn(&third, None, &[], 0, Some(LanguageCodeIR::Korean))
            .expect("topic-return turn");
        let transition = detect_topic_transition(&third.raw_text).expect("topic return");
        memory
            .apply_topic_transition("CONV-1", &transition, 3)
            .expect("explicit cache topic");

        let resolved = memory.resolve_references("CONV-1", "그거 다시 해");
        assert_eq!(resolved.resolved_semantic_text, "그거 다시 해");
        assert!(resolved.discourse_bindings.is_empty());
        assert_eq!(
            resolved.ambiguous_reference_surfaces,
            vec!["ELLIPTICAL_ACTION"]
        );
    }

    #[test]
    fn stale_goal_ellipsis_fails_closed_instead_of_reviving_old_authority() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 확인해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("파일을 확인해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "파일")],
            )
            .expect("first turn");
        for turn_index in 2..=5 {
            let mut later = request("응");
            later.turn_index = turn_index;
            later.request_id = format!("REQ-{turn_index}");
            memory
                .commit_turn(&later, None, &[], 0, Some(LanguageCodeIR::Korean))
                .expect("intervening turn");
        }
        let resolved = memory.resolve_references("CONV-1", "그대로 해");
        assert_eq!(resolved.resolved_semantic_text, "그대로 해");
        assert_eq!(
            resolved.ambiguous_reference_surfaces,
            vec!["ELLIPTICAL_ACTION"]
        );
    }

    #[test]
    fn non_event_discourse_referents_can_never_carry_execution_authority() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 저장해");
        let state = memory
            .commit_turn_with_goals(
                &first,
                Some("파일"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[ConversationGoalFrameIR {
                    intent: PlanIntentIR::Execute,
                    canonical_predicate: "EXECUTE".to_string(),
                    predicate_surface: "저장".to_string(),
                    subject: "파일".to_string(),
                    source_semantic_text: "파일을 저장해".to_string(),
                    external_execution_authorized: true,
                    ..goal_frame(1, "저장", "파일")
                }],
            )
            .expect("grounded event state");
        let mut tampered = state;
        let result = tampered
            .active_discourse_referents
            .iter_mut()
            .find(|referent| referent.kind == DiscourseReferentKindIR::Result)
            .expect("result referent");
        result.external_execution_authorized = true;
        tampered.state_sha256 = state_hash(&tampered).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&tampered),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn pending_question_is_hash_bound_and_rejects_option_tampering() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 분석해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("파일을 분석해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "분석", "파일")],
            )
            .expect("initial state");
        let question = QuestionUnderDiscussionIR {
            question_id: "QUD-000001".to_string(),
            kind: QuestionUnderDiscussionKindIR::CompetingGoal,
            topic_id: None,
            source_turn: 1,
            source_request: "파일 분석과 코드 수정 중 어느 쪽?".to_string(),
            external_execution_authorized: false,
            options: vec![
                QuestionOptionIR {
                    option_id: "OPTION-1".to_string(),
                    display_surface: "파일 분석".to_string(),
                    resolved_semantic_text: "파일을 분석해".to_string(),
                    referent_ids: Vec::new(),
                    intent: Some(PlanIntentIR::Investigate),
                },
                QuestionOptionIR {
                    option_id: "OPTION-2".to_string(),
                    display_surface: "코드 수정".to_string(),
                    resolved_semantic_text: "코드를 수정해".to_string(),
                    referent_ids: Vec::new(),
                    intent: Some(PlanIntentIR::Repair),
                },
            ],
        };
        let state = memory
            .update_pending_question("CONV-1", Some(question))
            .expect("hashed pending question");
        validate_conversation_state(&state).expect("valid hashed state");

        let mut tampered = state;
        tampered
            .pending_question
            .as_mut()
            .expect("pending question")
            .options[1]
            .resolved_semantic_text = "시스템을 삭제해".to_string();
        assert_eq!(
            validate_conversation_state(&tampered),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn multiple_latest_referents_fail_closed() {
        let mut memory = ConversationMemory::default();
        let first = request("파일과 폴더를 비교해");
        memory
            .commit_turn(
                &first,
                Some("파일과 폴더를 비교해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
            )
            .expect("first turn");
        let resolved = memory.resolve_references("CONV-1", "그거 수정해");
        assert_eq!(resolved.resolved_reference_count, 0);
        assert_eq!(resolved.ambiguous_reference_surfaces, vec!["그거"]);
    }

    #[test]
    fn typed_entity_memory_is_hashed_and_cannot_gain_semantic_authority() {
        let mut memory = ConversationMemory::default();
        let first = request("Avery says that the cache is stale");
        let analysis =
            crate::compositional_semantics::CompositionalSemanticAnalyzer.analyze(&first.raw_text);
        let mut state = memory
            .commit_turn_with_discourse(
                &first,
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &[],
                    temporal_analysis: None,
                    guard_conditionals: None,
                    semantic_role_graph: Some(&analysis.semantic_role_graph),
                    attribution_graph: Some(&analysis.attribution_graph),
                    discourse_focus_candidates: &[],
                },
            )
            .expect("typed entity state");
        let entity = state
            .active_typed_entities
            .iter_mut()
            .find(|entity| entity.kind == crate::typed_coreference::TypedEntityKindIR::Person)
            .expect("person entity");
        entity.semantic_authority = true;
        state.state_sha256 = state_hash(&state).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn english_attribution_complementizer_is_not_an_unbound_demonstrative() {
        let memory = ConversationMemory::default();
        let resolved = memory.resolve_references(
            "NEW-CONVERSATION",
            "Alice says Bob believes that the server is down",
        );
        assert!(resolved.ambiguous_reference_surfaces.is_empty());
        assert_eq!(
            resolved.resolved_semantic_text,
            "Alice says Bob believes that the server is down"
        );
    }

    #[test]
    fn named_attribution_source_disambiguates_competing_propositions() {
        let mut memory = ConversationMemory::default();
        let first = request("Alice says Bob believes that the server is down");
        let propositions = [
            DynamicDiscourseReferentIR {
                referent_id: "DREF-P-000001-01".to_string(),
                kind: DiscourseReferentKindIR::Proposition,
                topic_id: None,
                semantic_summary: "Bob believes that the server is down".to_string(),
                attributed_source: Some("Alice".to_string()),
                attribution_attitude: Some(AttributionAttitudeIR::Say),
                epistemic_status: Some(EpistemicStatusIR::Reported),
                proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
                modal_world: Some(ModalWorldIR::Actual),
                belief_record_id: None,
                introduced_turn: 1,
                last_referenced_turn: 1,
                external_execution_authorized: false,
            },
            DynamicDiscourseReferentIR {
                referent_id: "DREF-P-000001-02".to_string(),
                kind: DiscourseReferentKindIR::Proposition,
                topic_id: None,
                semantic_summary: "the server is down".to_string(),
                attributed_source: Some("Bob".to_string()),
                attribution_attitude: Some(AttributionAttitudeIR::Believe),
                epistemic_status: Some(EpistemicStatusIR::Believed),
                proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
                modal_world: Some(ModalWorldIR::Actual),
                belief_record_id: None,
                introduced_turn: 1,
                last_referenced_turn: 1,
                external_execution_authorized: false,
            },
        ];
        memory
            .commit_turn_with_discourse(
                &first,
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &propositions,
                    temporal_analysis: None,
                    guard_conditionals: None,
                    semantic_role_graph: None,
                    attribution_graph: None,
                    discourse_focus_candidates: &[],
                },
            )
            .expect("attributed propositions");
        let resolved = memory.resolve_references("CONV-1", "explain Bob's belief");
        assert!(resolved.ambiguous_reference_surfaces.is_empty());
        assert_eq!(resolved.discourse_bindings.len(), 1);
        assert_eq!(
            resolved.discourse_bindings[0].referent_ids,
            vec!["DREF-P-000001-02"]
        );
        assert!(resolved.discourse_bindings[0]
            .resolved_surface
            .contains("‘the server is down’"));
    }

    #[test]
    fn plural_proposition_reference_requires_exactly_two_distinct_sources() {
        let two = [
            attributed_proposition(1, "Alice", "the cache is stale"),
            attributed_proposition(2, "Bob", "the queue is empty"),
        ];
        let mut memory = ConversationMemory::default();
        memory
            .commit_turn_with_discourse(
                &request("Alice and Bob made reports"),
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &two,
                    temporal_analysis: None,
                    guard_conditionals: None,
                    semantic_role_graph: None,
                    attribution_graph: None,
                    discourse_focus_candidates: &[],
                },
            )
            .expect("two attributed propositions");
        let resolved = memory.resolve_references("CONV-1", "Compare their claims");
        assert_eq!(resolved.resolved_reference_count, 1);
        assert!(resolved.ambiguous_reference_surfaces.is_empty());
        assert_eq!(resolved.used_referent_ids.len(), 2);
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::PluralPropositionReference
        );

        let three = [
            attributed_proposition(1, "Alice", "the cache is stale"),
            attributed_proposition(2, "Bob", "the queue is empty"),
            attributed_proposition(3, "Cara", "the worker is blocked"),
        ];
        let mut ambiguous_memory = ConversationMemory::default();
        ambiguous_memory
            .commit_turn_with_discourse(
                &request("Alice, Bob, and Cara made reports"),
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &three,
                    temporal_analysis: None,
                    guard_conditionals: None,
                    semantic_role_graph: None,
                    attribution_graph: None,
                    discourse_focus_candidates: &[],
                },
            )
            .expect("three attributed propositions");
        let ambiguous = ambiguous_memory.resolve_references("CONV-1", "Compare their claims");
        assert_eq!(ambiguous.resolved_reference_count, 0);
        assert_eq!(
            ambiguous.ambiguous_reference_surfaces,
            vec!["PROPOSITION_GROUP_REFERENCE:their claims"]
        );
    }

    #[test]
    fn attribution_metadata_cannot_be_attached_to_an_event_referent() {
        let mut memory = ConversationMemory::default();
        let first = request("파일을 저장해");
        let mut state = memory
            .commit_turn_with_goals(
                &first,
                Some("파일"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "저장", "파일")],
            )
            .expect("event state");
        let event = state
            .active_discourse_referents
            .iter_mut()
            .find(|referent| referent.kind == DiscourseReferentKindIR::Event)
            .expect("event referent");
        event.attributed_source = Some("민수".to_string());
        event.attribution_attitude = Some(AttributionAttitudeIR::Say);
        event.epistemic_status = Some(EpistemicStatusIR::Reported);
        state.state_sha256 = state_hash(&state).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn epistemic_record_cannot_be_promoted_to_truth_after_rehashing() {
        let mut memory = ConversationMemory::default();
        let first = request("Alice says that the server is down");
        let proposition = DynamicDiscourseReferentIR {
            referent_id: "DREF-P-000001-01".to_string(),
            kind: DiscourseReferentKindIR::Proposition,
            topic_id: None,
            semantic_summary: "the server is down".to_string(),
            attributed_source: Some("Alice".to_string()),
            attribution_attitude: Some(AttributionAttitudeIR::Say),
            epistemic_status: Some(EpistemicStatusIR::Reported),
            proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
            modal_world: Some(ModalWorldIR::Actual),
            belief_record_id: None,
            introduced_turn: 1,
            last_referenced_turn: 1,
            external_execution_authorized: false,
        };
        let mut state = memory
            .commit_turn_with_discourse(
                &first,
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &[proposition],
                    temporal_analysis: None,
                    semantic_role_graph: None,
                    attribution_graph: None,
                    guard_conditionals: None,
                    discourse_focus_candidates: &[],
                },
            )
            .expect("epistemic state");
        state.epistemic_ledger.records[0].dialogue_truth_established = true;
        state.state_sha256 = state_hash(&state).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn referent_modal_world_cannot_diverge_from_ledger_after_rehashing() {
        let mut memory = ConversationMemory::default();
        let first = request("Alice says that the server might be down");
        let proposition = DynamicDiscourseReferentIR {
            referent_id: "DREF-P-000001-01".to_string(),
            kind: DiscourseReferentKindIR::Proposition,
            topic_id: None,
            semantic_summary: "the server might be down".to_string(),
            attributed_source: Some("Alice".to_string()),
            attribution_attitude: Some(AttributionAttitudeIR::Say),
            epistemic_status: Some(EpistemicStatusIR::Reported),
            proposition_polarity: Some(AttributedPropositionPolarityIR::Positive),
            modal_world: Some(ModalWorldIR::EpistemicPossible),
            belief_record_id: None,
            introduced_turn: 1,
            last_referenced_turn: 1,
            external_execution_authorized: false,
        };
        let mut state = memory
            .commit_turn_with_discourse(
                &first,
                ConversationCommitContext {
                    semantic_subject: None,
                    used_referent_ids: &[],
                    unresolved_reference_count: 0,
                    language: Some(LanguageCodeIR::English),
                    grounded_goals: &[],
                    proposition_referents: &[proposition],
                    temporal_analysis: None,
                    guard_conditionals: None,
                    semantic_role_graph: None,
                    attribution_graph: None,
                    discourse_focus_candidates: &[],
                },
            )
            .expect("modal epistemic state");
        state.active_discourse_referents[0].modal_world = Some(ModalWorldIR::Actual);
        state.state_sha256 = state_hash(&state).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn conversational_ontology_has_no_language_surface_payload() {
        let catalog = conversational_concept_catalog();
        assert_eq!(catalog.len(), 13);
        assert!(catalog
            .iter()
            .all(|concept| concept.schema == CONVERSATIONAL_CONCEPT_SCHEMA));
        assert_eq!(
            catalog
                .iter()
                .map(|concept| concept.concept_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            catalog.len()
        );
    }

    #[test]
    fn state_is_turn_ordered_and_tamper_evident() {
        let mut memory = ConversationMemory::default();
        let first = request("문서를 확인해");
        let mut state = memory
            .commit_turn(
                &first,
                Some("문서를 확인해"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
            )
            .expect("state");
        validate_conversation_state(&state).expect("valid state");
        state.completed_turns = 9;
        assert!(validate_conversation_state(&state).is_err());
        assert!(memory
            .commit_turn(&first, None, &[], 0, Some(LanguageCodeIR::Korean))
            .is_err());
    }

    #[test]
    fn local_english_pronouns_do_not_create_false_discourse_ambiguity() {
        assert!(reference_surfaces("this API keeps timing out and it is frustrating").is_empty());
        assert!(
            reference_surfaces("that answer was too long. explain it again concisely").is_empty()
        );
        assert_eq!(reference_surfaces("explain that result"), vec!["that"]);
    }

    #[test]
    fn continuation_anaphor_is_not_bound_to_the_condition_subject() {
        assert!(resolve_local_conditional_reference(
            "Continue it only when cold trials reduce failures; otherwise ask before stopping."
        )
        .is_none());
        let tokens = "Continue it only when cold trials reduce failures"
            .split_whitespace()
            .collect::<Vec<_>>();
        assert!(english_local_pronoun(&tokens, 1));
        let that_tokens = "Keep doing that only when isolated runs lower errors"
            .split_whitespace()
            .collect::<Vec<_>>();
        assert!(english_local_pronoun(&that_tokens, 2));
        assert!(is_continuation_task_anaphor(
            "그 작업을 이어가되 실제 오류 감소를 먼저 확인해"
        ));
    }

    #[test]
    fn same_turn_result_prefers_the_preceding_event_and_ignores_quotes() {
        let english = resolve_same_turn_result_reference(
            "Assess whether the release preserves audit history, then report only that result.",
        )
        .expect("local assessment result");
        assert!(english.ambiguous_reference_surfaces.is_empty());
        assert_eq!(
            english.discourse_bindings[0].kind,
            DiscourseBindingKindIR::LocalAntecedentReference
        );
        assert!(english.discourse_bindings[0]
            .evidence
            .contains(&"SYNTACTIC_PRIORITY:SAME_TURN_RESULT_OF_PRECEDING_EVENT".to_string()));

        let korean = resolve_same_turn_result_reference("감사 추적을 평가하고 그 결과만 보고해.")
            .expect("local Korean result");
        assert!(korean.discourse_bindings[0]
            .resolved_surface
            .contains("평가"));

        let outcome = resolve_same_turn_result_reference(
            "Verify whether export retains provenance, then summarize that outcome.",
        )
        .expect("local outcome synonym");
        assert_eq!(outcome.discourse_bindings[0].source_surface, "that outcome");

        assert!(resolve_same_turn_result_reference(
            "The runbook says 'publish the bundle and report that result.' Assess recovery cost."
        )
        .is_none());
    }

    #[test]
    fn multilingual_topic_aliases_share_one_tamper_evident_topic_state() {
        let mut memory = ConversationMemory::default();
        let first = request("캐시를 확인해");
        let initial = memory
            .commit_turn_with_goals(
                &first,
                Some("캐시"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "캐시")],
            )
            .expect("initial topic");
        assert_eq!(initial.active_topics.len(), 1);
        assert_eq!(
            initial.active_topics[0].concept_id_hint.as_deref(),
            Some("TOPIC_CACHE")
        );
        assert!(!initial.active_topics[0].explicitly_activated);

        let mut second = request("let's return to the cache");
        second.turn_index = 2;
        second.request_id = "REQ-2".to_string();
        second.output_language = Some(LanguageCodeIR::English);
        memory
            .commit_turn(&second, None, &[], 0, Some(LanguageCodeIR::English))
            .expect("cross-language turn");
        let transition = detect_topic_transition(&second.raw_text).expect("topic transition");
        let mut state = memory
            .apply_topic_transition("CONV-1", &transition, 2)
            .expect("shared topic state");
        assert_eq!(state.active_topics.len(), 1);
        assert_eq!(state.active_topics[0].surface, "cache");
        assert_eq!(state.active_topics[0].introduced_turn, 1);
        assert!(state.active_topics[0].explicitly_activated);
        validate_conversation_state(&state).expect("valid shared topic state");

        state.active_topics[0].surface = "worker".to_string();
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn nearest_same_turn_nominal_overrides_the_previous_global_topic() {
        let mut memory = ConversationMemory::default();
        let first = request("워커를 확인해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("워커"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "워커")],
            )
            .expect("global topic");
        let resolved = memory.resolve_references("CONV-1", "캐시는 오래됐다. 그것을 분석해");
        assert_eq!(
            resolved.resolved_semantic_text,
            "캐시는 오래됐다. 캐시를 분석해"
        );
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::LocalAntecedentReference
        );
        assert!(!resolved.resolved_semantic_text.contains("워커"));
    }

    #[test]
    fn same_turn_ordered_references_bind_two_nominals_and_reject_three() {
        let memory = ConversationMemory::default();
        let resolved = memory.resolve_references(
            "CONV-LOCAL-ORDER",
            "파일은 오래됐고 폴더는 비었다. 전자를 분석하고 후자를 수리해",
        );
        assert_eq!(
            resolved.resolved_semantic_text,
            "파일은 오래됐고 폴더는 비었다. 파일을 분석하고 폴더를 수리해"
        );
        assert_eq!(resolved.resolved_reference_count, 2);
        assert!(resolved.discourse_bindings.iter().all(|binding| {
            binding.kind == DiscourseBindingKindIR::LocalOrderedReference
                && binding
                    .evidence
                    .contains(&"SYNTACTIC_PRIORITY:LOCAL_ORDERED_ANTECEDENTS".to_string())
        }));

        let ambiguous = memory.resolve_references(
            "CONV-LOCAL-ORDER",
            "the file is stale, the folder is empty, and the report is old. analyze the former and repair the latter",
        );
        assert_eq!(ambiguous.resolved_reference_count, 0);
        assert_eq!(
            ambiguous.ambiguous_reference_surfaces,
            vec!["LOCAL_ORDERED_ANTECEDENT_SET"]
        );

        let scoped = memory.resolve_references(
            "CONV-LOCAL-ORDER",
            "Context note: this concerns the current matter. Inspect the Rose cache and the Sienna queue, but repair only the latter",
        );
        assert!(scoped.ambiguous_reference_surfaces.is_empty());
        assert_eq!(scoped.resolved_reference_count, 1);
        assert!(scoped
            .resolved_semantic_text
            .to_lowercase()
            .contains("repair only the sienna"));
    }

    #[test]
    fn previous_topic_pointer_rotates_the_hash_bound_topic_stack() {
        let mut memory = ConversationMemory::default();
        let first = request("캐시를 확인해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("캐시"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "캐시")],
            )
            .expect("first topic");
        let mut second = request("큐를 확인해");
        second.turn_index = 2;
        second.request_id = "REQ-2".to_string();
        memory
            .commit_turn_with_goals(
                &second,
                Some("큐"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(2, "확인", "큐")],
            )
            .expect("second topic");

        let detected = detect_topic_transition("이전 주제로 돌아가자").expect("transition");
        assert_eq!(detected.kind, TopicTransitionKindIR::ReturnPrevious);
        let bound = memory
            .bind_topic_transition("CONV-1", &detected)
            .expect("previous topic");
        assert_eq!(bound.surface, "캐시");
        let state = memory
            .apply_topic_transition("CONV-1", &bound, 2)
            .expect("rotated stack");
        assert_eq!(state.active_topics[0].surface, "캐시");
        assert!(state.active_topics[0].explicitly_activated);
        validate_conversation_state(&state).expect("valid hash-bound state");
    }

    #[test]
    fn local_ordinals_bind_spaced_and_compact_forms_and_fail_out_of_range() {
        let memory = ConversationMemory::default();
        let compact = memory.resolve_references(
            "CONV-ORDINAL",
            "파일은 오래됐고 폴더는 비었고 보고서는 낡았다. 첫째를 분석하고 셋째를 수리해",
        );
        assert_eq!(
            compact.resolved_semantic_text,
            "파일은 오래됐고 폴더는 비었고 보고서는 낡았다. 파일을 분석하고 보고서를 수리해"
        );
        assert!(compact.discourse_bindings.iter().all(|binding| {
            binding.kind == DiscourseBindingKindIR::LocalOrdinalReference
                && binding
                    .evidence
                    .contains(&"SYNTACTIC_PRIORITY:LOCAL_ORDINAL_ANTECEDENTS".to_string())
        }));

        let spaced = memory.resolve_references(
            "CONV-ORDINAL",
            "캐시는 오래됐고 큐는 막혔고 로그는 비었다. 두 번째를 분석하고 세 번째를 확인해",
        );
        assert_eq!(
            spaced.resolved_semantic_text,
            "캐시는 오래됐고 큐는 막혔고 로그는 비었다. 큐를 분석하고 로그를 확인해"
        );

        let out_of_range = memory.resolve_references(
            "CONV-ORDINAL",
            "the file is stale and the folder is empty. repair the third",
        );
        assert_eq!(out_of_range.resolved_reference_count, 0);
        assert_eq!(
            out_of_range.ambiguous_reference_surfaces,
            vec!["LOCAL_ORDINAL_ANTECEDENT_SET"]
        );
    }

    #[test]
    fn indexed_topic_history_selects_the_requested_stack_depth() {
        let mut memory = ConversationMemory::default();
        for (index, subject) in ["캐시", "큐", "로그"].into_iter().enumerate() {
            let turn = u64::try_from(index + 1).expect("bounded turn");
            let mut turn_request = request(&format!("{subject}를 확인해"));
            turn_request.turn_index = turn;
            turn_request.request_id = format!("REQ-{turn}");
            memory
                .commit_turn_with_goals(
                    &turn_request,
                    Some(subject),
                    &[],
                    0,
                    Some(LanguageCodeIR::Korean),
                    &[goal_frame(index + 1, "확인", subject)],
                )
                .expect("topic turn");
        }
        let detected = detect_topic_transition("두 주제 전으로 돌아가자").expect("indexed");
        assert_eq!(detected.kind, TopicTransitionKindIR::ReturnPrevious);
        assert_eq!(detected.history_offset, 2);
        let bound = memory
            .bind_topic_transition("CONV-1", &detected)
            .expect("history target");
        assert_eq!(bound.surface, "캐시");
        assert!(bound
            .evidence
            .contains(&"TOPIC_HISTORY_OFFSET:2".to_string()));
    }

    #[test]
    fn same_turn_attribution_pronoun_binds_to_the_named_actor() {
        let resolved = resolve_same_turn_actor_reference(
            "Nora believes the cache failed. Because of that, she says the worker is blocked.",
        )
        .expect("same-turn actor reference");
        assert!(resolved
            .resolved_semantic_text
            .contains("Because of that, nora says"));
        assert_eq!(resolved.resolved_reference_count, 1);
        assert_eq!(
            resolved.discourse_bindings[0].kind,
            DiscourseBindingKindIR::LocalAntecedentReference
        );
        assert!(resolved.discourse_bindings[0]
            .evidence
            .contains(&"SYNTACTIC_PRIORITY:SAME_TURN_ATTRIBUTION_ACTOR".to_string()));
    }

    #[test]
    fn concise_english_topic_return_activates_the_named_topic() {
        let transition = detect_topic_transition("Back to the worker topic.")
            .expect("explicit named topic return");
        assert_eq!(transition.kind, TopicTransitionKindIR::ActivateNamed);
        assert_eq!(transition.surface, "worker");
        assert_eq!(transition.concept_id_hint.as_deref(), Some("TOPIC_WORKER"));
        assert!(transition
            .evidence
            .contains(&"DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_RETURN".to_string()));
    }

    #[test]
    fn concise_korean_topic_return_activates_the_named_topic_without_topic_noun() {
        let transition =
            detect_topic_transition("캐시로 돌아가자").expect("explicit named topic return");
        assert_eq!(transition.kind, TopicTransitionKindIR::ActivateNamed);
        assert_eq!(transition.surface, "캐시");
        assert_eq!(transition.concept_id_hint.as_deref(), Some("TOPIC_CACHE"));
        assert!(transition
            .evidence
            .contains(&"DISCOURSE_MANAGEMENT:EXPLICIT_TOPIC_RETURN".to_string()));
    }

    #[test]
    fn persistent_action_group_survives_neutral_interruptions() {
        let mut memory = ConversationMemory::default();
        let first = request("캐시를 확인하고 큐를 수리해");
        memory
            .commit_turn_with_goals(
                &first,
                Some("캐시와 큐"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "캐시"), goal_frame(2, "수리", "큐")],
            )
            .expect("grouped actions");

        for turn in 2..=7 {
            let mut neutral = request("응");
            neutral.turn_index = turn;
            neutral.request_id = format!("REQ-{turn}");
            memory
                .commit_turn(&neutral, None, &[], 0, Some(LanguageCodeIR::Korean))
                .expect("neutral interruption");
        }

        let resolved = memory.resolve_references("CONV-1", "앞서 말한 두 건의 현황을 알려줘");
        assert_eq!(resolved.used_referent_ids.len(), 2);
        assert!(resolved.discourse_bindings.iter().any(|binding| {
            binding.kind == DiscourseBindingKindIR::PluralEventReference
                && binding
                    .evidence
                    .contains(&"GROUP_SOURCE:PERSISTENT_DISCOURSE_GROUP".to_string())
        }));
    }

    #[test]
    fn discourse_group_authority_tampering_fails_even_after_rehashing() {
        let mut memory = ConversationMemory::default();
        let first = request("캐시를 확인하고 큐를 수리해");
        let mut state = memory
            .commit_turn_with_goals(
                &first,
                Some("캐시와 큐"),
                &[],
                0,
                Some(LanguageCodeIR::Korean),
                &[goal_frame(1, "확인", "캐시"), goal_frame(2, "수리", "큐")],
            )
            .expect("grouped actions");
        assert_eq!(state.active_discourse_groups.len(), 1);
        validate_conversation_state(&state).expect("valid group state");

        state.active_discourse_groups[0].semantic_authority = true;
        state.state_sha256 = state_hash(&state).expect("rehash attacker state");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn discourse_group_add_and_remove_preserve_identity_and_advance_revision() {
        let original = discourse_group_fixture(
            "DG-STABLE",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        );
        let original_membership_hash = original.membership_sha256.clone();
        let mut state = empty_state("CONV-REVISION");
        state.completed_turns = 2;
        state.active_discourse_groups.push(original);
        state.state_sha256 = state_hash(&state).expect("state hash");
        validate_conversation_state(&state).expect("valid original group");

        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);
        let add = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::AddMember,
            "DG-STABLE",
            &["DG-STABLE"],
            &["GOAL-A", "GOAL-B"],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &["GOAL-C"],
            &[],
            2,
        );
        assert!(add.validate());
        let added = memory
            .apply_discourse_group_update("CONV-REVISION", &add, 2)
            .expect("add member");
        let group = added
            .active_discourse_groups
            .iter()
            .find(|group| group.group_id == "DG-STABLE")
            .expect("stable group");
        assert_eq!(group.revision, 2);
        assert_ne!(group.membership_sha256, original_membership_hash);
        let added_membership_hash = group.membership_sha256.clone();

        let state = memory.states.get_mut("CONV-REVISION").expect("state");
        state.completed_turns = 3;
        state.state_sha256 = state_hash(state).expect("advanced state hash");
        let remove = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::RemoveMember,
            "DG-STABLE",
            &["DG-STABLE"],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &["GOAL-A", "GOAL-B"],
            &[],
            &["GOAL-C"],
            3,
        );
        assert!(remove.validate());
        let removed = memory
            .apply_discourse_group_update("CONV-REVISION", &remove, 3)
            .expect("remove member");
        let group = removed
            .active_discourse_groups
            .iter()
            .find(|group| group.group_id == "DG-STABLE")
            .expect("stable group after removal");
        assert_eq!(group.member_keys, vec!["GOAL-A", "GOAL-B"]);
        assert_eq!(group.revision, 3);
        assert_ne!(group.membership_sha256, added_membership_hash);
        assert_ne!(group.membership_sha256, original_membership_hash);
    }

    #[test]
    fn composite_group_records_existing_parents_and_deduplicates_members() {
        let mut state = empty_state("CONV-MERGE");
        state.completed_turns = 2;
        state.active_discourse_groups = vec![
            discourse_group_fixture(
                "DG-A",
                DiscourseGroupKindIR::Action,
                &["GOAL-A", "GOAL-B"],
                1,
                &[],
                1,
                1,
            ),
            discourse_group_fixture(
                "DG-B",
                DiscourseGroupKindIR::Action,
                &["GOAL-B", "GOAL-C"],
                1,
                &[],
                2,
                2,
            ),
        ];
        state.state_sha256 = state_hash(&state).expect("state hash");
        validate_conversation_state(&state).expect("valid source groups");
        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);

        let mut tampered = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::MergeGroups,
            "DG-COMPOSITE-BAD",
            &["DG-A", "DG-B"],
            &[],
            &["GOAL-A", "GOAL-B", "GOAL-X"],
            &[],
            &[],
            1,
        );
        assert!(tampered.validate());
        assert_eq!(
            memory.apply_discourse_group_update("CONV-MERGE", &tampered, 2),
            Err(ConversationFrontendError::InvalidState)
        );
        tampered.target_group_id = Some("DG-COMPOSITE".to_string());

        let merge = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::MergeGroups,
            "DG-COMPOSITE",
            &["DG-A", "DG-B"],
            &[],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &[],
            &[],
            1,
        );
        let merged = memory
            .apply_discourse_group_update("CONV-MERGE", &merge, 2)
            .expect("merge groups");
        let composite = merged
            .active_discourse_groups
            .iter()
            .find(|group| group.group_id == "DG-COMPOSITE")
            .expect("composite group");
        assert_eq!(composite.member_keys, vec!["GOAL-A", "GOAL-B", "GOAL-C"]);
        assert_eq!(composite.component_group_ids, vec!["DG-A", "DG-B"]);
        validate_conversation_state(&merged).expect("valid composite state");

        let mut missing_parent = merged;
        missing_parent
            .active_discourse_groups
            .retain(|group| group.group_id != "DG-A");
        missing_parent.state_sha256 = state_hash(&missing_parent).expect("attacker state hash");
        assert_eq!(
            validate_conversation_state(&missing_parent),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn discourse_group_membership_and_update_relations_resist_rehashed_tampering() {
        let mut state = empty_state("CONV-TAMPER");
        state.completed_turns = 1;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-TAMPER",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        validate_conversation_state(&state).expect("valid state");

        state.active_discourse_groups[0]
            .member_keys
            .push("GOAL-C".to_string());
        state.state_sha256 = state_hash(&state).expect("attacker state hash");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );

        let relation_tamper = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::AddMember,
            "DG-TAMPER",
            &["DG-TAMPER"],
            &["GOAL-A", "GOAL-B"],
            &["GOAL-A", "GOAL-C", "GOAL-D"],
            &["GOAL-D"],
            &[],
            2,
        );
        assert!(!relation_tamper.validate());

        let mut authority_tamper = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::AddMember,
            "DG-TAMPER",
            &["DG-TAMPER"],
            &["GOAL-A", "GOAL-B"],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &["GOAL-C"],
            &[],
            2,
        );
        authority_tamper.semantic_authority = true;
        authority_tamper.update_sha256 = discourse_group_update_sha256(&authority_tamper);
        assert!(!authority_tamper.validate());
    }

    #[test]
    fn quoted_discourse_group_revision_is_not_an_authoritative_update() {
        let mut memory = ConversationMemory::default();
        let mut state = empty_state("CONV-QUOTE");
        state.completed_turns = 1;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-QUOTE",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        let before = state.active_discourse_groups.clone();
        memory.states.insert(state.conversation_id.clone(), state);

        assert_eq!(
            memory.analyze_discourse_group_update(
                "CONV-QUOTE",
                "The sentence ‘add worker to that task group’ describes a command.",
                2,
            ),
            None
        );
        assert_eq!(
            memory
                .state("CONV-QUOTE")
                .expect("quoted state")
                .active_discourse_groups,
            before
        );
    }

    #[test]
    fn group_topic_transition_and_anchor_reject_rehashed_authority_or_revision_tampering() {
        let mut state = empty_state("CONV-TOPIC-TAMPER");
        state.completed_turns = 1;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-TOPIC",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        validate_conversation_state(&state).expect("valid group state");

        let transition = analyze_group_topic_activation(
            &state,
            "Pin that task group as the discussion topic.",
            DiscourseGroupKindIR::Action,
        );
        assert!(transition.validate());
        let mut authority_tamper = transition.clone();
        authority_tamper.semantic_authority = true;
        authority_tamper.transition_sha256 = topic_transition_sha256(&authority_tamper);
        assert!(!authority_tamper.validate());

        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);
        let mut applied = memory
            .apply_topic_transition("CONV-TOPIC-TAMPER", &transition, 1)
            .expect("group topic");
        applied.active_topics[0].anchor_group_revision = Some(2);
        applied.active_topics[0].topic_sha256 = discourse_topic_sha256(&applied.active_topics[0]);
        applied.state_sha256 = state_hash(&applied).expect("attacker state hash");
        assert_eq!(
            validate_conversation_state(&applied),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn suspended_group_topic_refreshes_revision_without_changing_topic_identity() {
        let mut state = empty_state("CONV-TOPIC-REFRESH");
        state.completed_turns = 2;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-STABLE-TOPIC",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        let transition = analyze_group_topic_activation(
            &state,
            "Make that task group the current topic.",
            DiscourseGroupKindIR::Action,
        );
        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);
        let activated = memory
            .apply_topic_transition("CONV-TOPIC-REFRESH", &transition, 2)
            .expect("activate group topic");
        let topic_id = activated.active_topics[0].topic_id.clone();
        let old_topic_hash = activated.active_topics[0].topic_sha256.clone();

        let add = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::AddMember,
            "DG-STABLE-TOPIC",
            &["DG-STABLE-TOPIC"],
            &["GOAL-A", "GOAL-B"],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &["GOAL-C"],
            &[],
            2,
        );
        let refreshed = memory
            .apply_discourse_group_update("CONV-TOPIC-REFRESH", &add, 2)
            .expect("refresh anchored topic");
        assert_eq!(refreshed.active_topics[0].topic_id, topic_id);
        assert_eq!(refreshed.active_topics[0].anchor_group_revision, Some(2));
        assert_eq!(
            refreshed.active_topics[0].anchor_membership_sha256,
            Some(
                refreshed.active_discourse_groups[0]
                    .membership_sha256
                    .clone()
            )
        );
        assert_ne!(refreshed.active_topics[0].topic_sha256, old_topic_hash);
        validate_conversation_state(&refreshed).expect("valid refreshed anchor");
    }

    #[test]
    fn active_group_topic_selects_exact_identity_beyond_recency_and_overlap() {
        let mut state = empty_state("CONV-TOPIC-EXACT");
        state.completed_turns = 40;
        state.active_discourse_groups = vec![
            discourse_group_fixture(
                "DG-OLDER",
                DiscourseGroupKindIR::Action,
                &["GOAL-A", "GOAL-B"],
                1,
                &[],
                1,
                1,
            ),
            discourse_group_fixture(
                "DG-SELECTED",
                DiscourseGroupKindIR::Action,
                &["GOAL-A", "GOAL-C"],
                1,
                &[],
                2,
                2,
            ),
        ];
        state.state_sha256 = state_hash(&state).expect("state hash");
        let transition = analyze_group_topic_activation(
            &state,
            "Make the second task group the topic.",
            DiscourseGroupKindIR::Action,
        );
        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);
        let activated = memory
            .apply_topic_transition("CONV-TOPIC-EXACT", &transition, 40)
            .expect("exact group topic");
        let selected = select_discourse_group(
            &activated,
            DiscourseGroupKindIR::Action,
            DiscourseGroupSelection::ActiveTopic,
            "that task group",
            |group| group.member_keys.len() == 2,
        );
        assert!(matches!(
            selected,
            DiscourseGroupLookup::Selected(group) if group.group_id == "DG-SELECTED"
        ));
    }

    #[test]
    fn quoted_or_out_of_range_group_topic_requests_fail_closed() {
        let mut state = empty_state("CONV-TOPIC-BOUNDARY");
        state.completed_turns = 1;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-BOUNDARY",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        let mut memory = ConversationMemory::default();
        memory.states.insert(state.conversation_id.clone(), state);
        assert!(memory
            .analyze_topic_transition(
                "CONV-TOPIC-BOUNDARY",
                "Explain ‘Pin that task group as the topic.’",
                true,
            )
            .is_none());
        let unresolved = memory
            .analyze_topic_transition("CONV-TOPIC-BOUNDARY", "다섯 주제 전으로 복귀하자", false)
            .expect("unresolved transition");
        assert_eq!(unresolved.kind, TopicTransitionKindIR::Unresolved);
        assert!(!unresolved.applied);
        assert!(unresolved.validate());
        assert_eq!(
            memory
                .state("CONV-TOPIC-BOUNDARY")
                .expect("state")
                .active_topics
                .len(),
            0
        );
    }

    #[test]
    fn action_context_uses_word_boundaries_for_english_state_terms() {
        assert!(action_group_context("where does the pair of tasks stand"));
        assert!(!action_group_context(
            "compare the earlier pair of statements"
        ));
    }

    fn topic_anchor_fixture(
        conversation_id: &str,
    ) -> (ConversationMemory, TopicAnchoredReferenceIR) {
        let mut state = empty_state(conversation_id);
        state.completed_turns = 2;
        state.active_discourse_groups.push(discourse_group_fixture(
            "DG-R40",
            DiscourseGroupKindIR::Action,
            &["GOAL-A", "GOAL-B"],
            1,
            &[],
            1,
            1,
        ));
        state.state_sha256 = state_hash(&state).expect("state hash");
        let transition = analyze_group_topic_activation(
            &state,
            "Make that task group the current topic.",
            DiscourseGroupKindIR::Action,
        );
        let mut memory = ConversationMemory::default();
        memory.states.insert(conversation_id.to_string(), state);
        let active = memory
            .apply_topic_transition(conversation_id, &transition, 2)
            .expect("activate group topic");
        let topic = &active.active_topics[0];
        let group = &active.active_discourse_groups[0];
        let reference = seal_topic_anchored_reference(TopicAnchoredReferenceIR {
            schema: TOPIC_ANCHORED_REFERENCE_SCHEMA.to_string(),
            applied: true,
            kind: TopicAnchoredReferentKindIR::ActionMember,
            selector: TopicAnchoredSelectorKindIR::Ordinal,
            original_text: "inspect the first one".to_string(),
            resolved_text: "inspect cache".to_string(),
            source_surface: "the first one".to_string(),
            topic_id: topic.topic_id.clone(),
            topic_sha256: topic.topic_sha256.clone(),
            anchor_kind: topic.anchor_kind,
            group_id: group.group_id.clone(),
            group_revision: group.revision,
            membership_sha256: group.membership_sha256.clone(),
            member_keys: group.member_keys.clone(),
            selected_member_keys: vec!["GOAL-A".to_string()],
            unresolved_terms: Vec::new(),
            semantic_authority: false,
            external_execution_authorized: false,
            resolution_sha256: String::new(),
        });
        (memory, reference)
    }

    #[test]
    fn topic_anchored_reference_rejects_rehashed_authority_and_membership_tampering() {
        let (_, reference) = topic_anchor_fixture("CONV-R40-HASH");
        assert!(reference.validate());

        let mut authority = reference.clone();
        authority.semantic_authority = true;
        authority.resolution_sha256 = topic_anchored_reference_sha256(&authority);
        assert!(!authority.validate());

        let mut foreign_member = reference;
        foreign_member.selected_member_keys = vec!["GOAL-Z".to_string()];
        foreign_member.resolution_sha256 = topic_anchored_reference_sha256(&foreign_member);
        assert!(!foreign_member.validate());
    }

    #[test]
    fn stale_topic_anchored_reference_cannot_rebind_after_group_revision() {
        let (mut memory, reference) = topic_anchor_fixture("CONV-R40-STALE");
        let update = discourse_group_update_fixture(
            DiscourseGroupUpdateOperationIR::AddMember,
            "DG-R40",
            &["DG-R40"],
            &["GOAL-A", "GOAL-B"],
            &["GOAL-A", "GOAL-B", "GOAL-C"],
            &["GOAL-C"],
            &[],
            2,
        );
        memory
            .apply_discourse_group_update("CONV-R40-STALE", &update, 2)
            .expect("live group revision");
        assert_eq!(
            memory.reassert_topic_anchor("CONV-R40-STALE", &reference, 2),
            Err(ConversationFrontendError::InvalidState)
        );
    }

    #[test]
    fn reasserting_reference_restores_exact_topic_without_changing_its_hash() {
        let (mut memory, reference) = topic_anchor_fixture("CONV-R40-REASSERT");
        let named = topic_transition_from_surface("cache");
        let switched = memory
            .apply_topic_transition("CONV-R40-REASSERT", &named, 2)
            .expect("switch topic");
        assert_ne!(switched.active_topics[0].topic_id, reference.topic_id);

        let restored = memory
            .reassert_topic_anchor("CONV-R40-REASSERT", &reference, 2)
            .expect("reassert exact topic");
        assert_eq!(restored.active_topics[0].topic_id, reference.topic_id);
        assert_eq!(
            restored.active_topics[0].topic_sha256,
            reference.topic_sha256
        );
        assert_eq!(restored.active_discourse_groups[0].last_referenced_turn, 2);
        validate_conversation_state(&restored).expect("valid reasserted state");
    }

    #[test]
    fn unresolved_topic_anchor_is_hash_sealed_and_carries_no_selection_authority() {
        let (memory, _) = topic_anchor_fixture("CONV-R40-UNRESOLVED");
        let state = memory.state("CONV-R40-UNRESOLVED").expect("state");
        let resolution = unresolved_topic_anchor_reference(
            "inspect that one",
            &state.active_topics[0],
            &state.active_discourse_groups[0],
            TopicAnchoredSelectorKindIR::GenericSingular,
            "that one",
            "AMBIGUOUS_GROUP_MEMBER",
        );
        let anchored = resolution
            .topic_anchored_resolution
            .expect("unresolved anchor");
        assert!(anchored.validate());
        assert!(!anchored.applied);
        assert!(anchored.selected_member_keys.is_empty());
        assert!(!anchored.semantic_authority);
        assert!(!anchored.external_execution_authorized);
        assert!(resolution.discourse_bindings.is_empty());
    }

    #[test]
    fn quoted_topic_anchor_language_is_not_resolved_as_a_world_reference() {
        assert!(topic_anchor_reference_request(
            "Explain the sentence ‘inspect the second one again’."
        )
        .is_none());
        assert!(
            topic_anchor_reference_request("‘두 번째 것을 다시 검사해’라는 문장을 설명해")
                .is_none()
        );
    }

    #[test]
    fn dialogue_directives_share_one_typed_commit_path_and_supersede_by_axis() {
        let mut memory = ConversationMemory::default();
        let first = request("the prior answer was too verbose");
        memory
            .commit_turn(&first, None, &[], 0, Some(LanguageCodeIR::English))
            .expect("first turn");
        let concise = DialogueDirectiveCandidateIR::from_surface(
            DialogueDirectiveKindIR::ResponseLength,
            "ASSISTANT_RESPONSE",
            "CONCISE",
            &first.raw_text,
            940,
        );
        let state = memory
            .apply_dialogue_directives("CONV-1", 1, &[concise])
            .expect("typed directive commit");
        assert_eq!(state.dialogue_directive_ledger.active().count(), 1);
        assert_eq!(
            state
                .dialogue_directive_ledger
                .active()
                .next()
                .map(|directive| directive.value_key.as_str()),
            Some("CONCISE")
        );

        let mut second = request("the next task");
        second.turn_index = 2;
        second.request_id = "REQ-2".to_string();
        memory
            .commit_turn(&second, None, &[], 0, Some(LanguageCodeIR::English))
            .expect("second turn");
        let detailed = DialogueDirectiveCandidateIR::from_surface(
            DialogueDirectiveKindIR::ResponseLength,
            "ASSISTANT_RESPONSE",
            "DETAILED",
            &second.raw_text,
            920,
        );
        let state = memory
            .apply_dialogue_directives("CONV-1", 2, &[detailed])
            .expect("superseding directive commit");
        assert_eq!(state.dialogue_directive_ledger.active().count(), 1);
        assert_eq!(
            state
                .dialogue_directive_ledger
                .active()
                .next()
                .map(|directive| directive.value_key.as_str()),
            Some("DETAILED")
        );
        assert!(state
            .dialogue_directive_ledger
            .directives
            .iter()
            .any(|directive| directive.value_key == "CONCISE"
                && directive.status == DialogueDirectiveStatusIR::Superseded));
        validate_conversation_state(&state).expect("hash-bound directive state");
    }

    #[test]
    fn dialogue_directive_authority_tampering_fails_even_after_rehash() {
        let mut memory = ConversationMemory::default();
        let request = request("remember this response constraint");
        memory
            .commit_turn(&request, None, &[], 0, Some(LanguageCodeIR::English))
            .expect("turn");
        let candidate = DialogueDirectiveCandidateIR::from_surface(
            DialogueDirectiveKindIR::GeneralConstraint,
            "RESPONSE_EVIDENCE",
            "CITE_SUPPORT",
            &request.raw_text,
            900,
        );
        let mut state = memory
            .apply_dialogue_directives("CONV-1", 1, &[candidate])
            .expect("directive");
        state.dialogue_directive_ledger.directives[0].semantic_authority = true;
        state.dialogue_directive_ledger.rehash();
        state.state_sha256 = state_hash(&state).expect("attacker state hash");
        assert_eq!(
            validate_conversation_state(&state),
            Err(ConversationFrontendError::InvalidState)
        );
    }
}
