//! Language-free, replay-verified response meaning approved for realization.
//!
//! This boundary intentionally excludes source utterances, lexical selections,
//! surface strings, hidden states and completed responses. A candidate is
//! accepted only when the dockable semantic core reproduces its deliberation.

use std::collections::BTreeSet;

use dockable_semantic_core::{
    DeliberationDispositionIR, DeliberationEngine, DeliberationIR, DeliberationRequestIR,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::discourse_focus::DiscourseFocusStateIR;
use crate::language_knowledge::LanguageRegisterIR;

pub const APPROVED_RESPONSE_SCHEMA: &str = "B_CORE_APPROVED_RESPONSE_IR_1";
pub const COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA: &str = "B_CORE_APPROVED_RESPONSE_IR_6";
pub const APPROVED_EVENT_DISCOURSE_STATE_SCHEMA: &str =
    "B_CORE_APPROVED_EVENT_DISCOURSE_STATE_IR_1";
pub const APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA: &str =
    "B_CORE_APPROVED_EVENT_PRAGMATIC_CONTEXT_IR_1";
pub const MAX_APPROVED_RESPONSE_CLAIMS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedSpeechActIR {
    Inform,
    Acknowledge,
    Explain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedDiscourseRelationIR {
    Statement,
    Correction,
    Negation,
    Comparison,
    Cause,
    Condition,
    Temporal,
    Recall,
    Confirmation,
    Explanation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedVerbosityIR {
    Short,
    Explanatory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedValueIR {
    Symbol(String),
    Text(String),
    Boolean(bool),
    Integer(i64),
    Clock { hour: u8, minute: u8 },
    Date { year: i32, month: u8, day: u8 },
    Quantity { amount: i64, unit: String },
}

impl ApprovedValueIR {
    fn validate(&self) -> bool {
        match self {
            Self::Symbol(value) => valid_symbol(value),
            Self::Text(value) => valid_lexical_label(value),
            Self::Boolean(_) | Self::Integer(_) => true,
            Self::Clock { hour, minute } => *hour < 24 && *minute < 60,
            Self::Date { year, month, day } => {
                *year >= 1
                    && *year <= 9999
                    && *month >= 1
                    && *month <= 12
                    && *day >= 1
                    && *day <= 31
            }
            Self::Quantity { unit, .. } => valid_symbol(unit),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedSemanticTypeIR {
    Event,
    Location,
    Person,
    Organization,
    Concept,
    State,
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedRelationTypeIR {
    Status,
    Time,
    Cancelled,
    EarlierThan,
    RoomAvailable,
    Location,
    Registration,
    Entry,
    Confirmed,
    Capacity,
    Duration,
    Name,
    Count,
    Date,
    Quantity,
    Agent,
    Patient,
    Theme,
    Source,
    Destination,
    Instrument,
    Manner,
    InitialState,
    ResultState,
}

impl ApprovedRelationTypeIR {
    fn symbol(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Time => "time",
            Self::Cancelled => "cancelled",
            Self::EarlierThan => "earlier_than",
            Self::RoomAvailable => "room_available",
            Self::Location => "location",
            Self::Registration => "registration",
            Self::Entry => "entry",
            Self::Confirmed => "confirmed",
            Self::Capacity => "capacity",
            Self::Duration => "duration",
            Self::Name => "name",
            Self::Count => "count",
            Self::Date => "date",
            Self::Quantity => "quantity",
            Self::Agent => "agent",
            Self::Patient => "patient",
            Self::Theme => "theme",
            Self::Source => "source",
            Self::Destination => "destination",
            Self::Instrument => "instrument",
            Self::Manner => "manner",
            Self::InitialState => "initial_state",
            Self::ResultState => "result_state",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedModalityIR {
    Asserted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedOperationIR {
    Assert,
    Revise,
    Negate,
    Compare,
    Explain,
    Condition,
    Recall,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedLexicalNodeIR {
    pub node_id: String,
    pub semantic_type: ApprovedSemanticTypeIR,
    pub canonical_lexical_label: String,
}

impl ApprovedLexicalNodeIR {
    fn validate(&self) -> bool {
        valid_symbol(&self.node_id) && valid_lexical_label(&self.canonical_lexical_label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedOpenValueIR {
    Lexical(ApprovedLexicalNodeIR),
    Text(String),
    Boolean(bool),
    Integer(i64),
    Clock {
        hour: u8,
        minute: u8,
    },
    Date {
        year: i32,
        month: u8,
        day: u8,
    },
    Quantity {
        amount: i64,
        unit: ApprovedLexicalNodeIR,
    },
}

impl ApprovedOpenValueIR {
    fn validate(&self) -> bool {
        match self {
            Self::Lexical(node) => node.validate(),
            Self::Text(value) => valid_lexical_label(value),
            Self::Boolean(_) | Self::Integer(_) => true,
            Self::Clock { hour, minute } => *hour < 24 && *minute < 60,
            Self::Date { year, month, day } => {
                *year >= 1
                    && *year <= 9999
                    && *month >= 1
                    && *month <= 12
                    && *day >= 1
                    && *day <= 31
            }
            Self::Quantity { unit, .. } => unit.validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedCompositionalClaimIR {
    pub proposition_id: String,
    pub subject: ApprovedLexicalNodeIR,
    pub relation: ApprovedRelationTypeIR,
    pub value: ApprovedOpenValueIR,
    pub polarity: bool,
    pub modality: ApprovedModalityIR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventRealizationClassIR {
    HostedEvent,
    ScheduledProcess,
    Departure,
    Arrival,
    Motion,
    Transfer,
    Creation,
    StateChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventPredicateSenseIR {
    Move,
    Transfer,
    Deliver,
    Send,
    Give,
    Create,
    Change,
    Start,
    Complete,
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventPhaseIR {
    Planned,
    Scheduled,
    Ongoing,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ApprovedEventPredicateSpec {
    pub event_class: ApprovedEventRealizationClassIR,
    pub predicate_sense: ApprovedEventPredicateSenseIR,
    pub required_roles: &'static [ApprovedRelationTypeIR],
    pub optional_roles: &'static [ApprovedRelationTypeIR],
    pub allowed_phases: &'static [ApprovedEventPhaseIR],
}

impl ApprovedEventPredicateSpec {
    pub(crate) fn allows_role(self, role: ApprovedRelationTypeIR) -> bool {
        self.required_roles.contains(&role) || self.optional_roles.contains(&role)
    }

    pub(crate) fn allows_phase(self, phase: ApprovedEventPhaseIR) -> bool {
        self.allowed_phases.contains(&phase)
    }
}

const ALL_SEMANTIC_EVENT_PHASES: &[ApprovedEventPhaseIR] = &[
    ApprovedEventPhaseIR::Planned,
    ApprovedEventPhaseIR::Scheduled,
    ApprovedEventPhaseIR::Ongoing,
    ApprovedEventPhaseIR::Completed,
    ApprovedEventPhaseIR::Cancelled,
];

const COMMON_SEMANTIC_EVENT_ROLES: &[ApprovedRelationTypeIR] = &[
    ApprovedRelationTypeIR::Agent,
    ApprovedRelationTypeIR::Date,
    ApprovedRelationTypeIR::Time,
    ApprovedRelationTypeIR::Location,
    ApprovedRelationTypeIR::Instrument,
    ApprovedRelationTypeIR::Manner,
];

pub(crate) const APPROVED_EVENT_PREDICATE_SPECS: &[ApprovedEventPredicateSpec] = &[
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Motion,
        predicate_sense: ApprovedEventPredicateSenseIR::Move,
        required_roles: &[
            ApprovedRelationTypeIR::Theme,
            ApprovedRelationTypeIR::Destination,
        ],
        optional_roles: &[
            ApprovedRelationTypeIR::Agent,
            ApprovedRelationTypeIR::Source,
            ApprovedRelationTypeIR::Date,
            ApprovedRelationTypeIR::Time,
            ApprovedRelationTypeIR::Location,
            ApprovedRelationTypeIR::Instrument,
            ApprovedRelationTypeIR::Manner,
        ],
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Transfer,
        predicate_sense: ApprovedEventPredicateSenseIR::Transfer,
        required_roles: &[
            ApprovedRelationTypeIR::Theme,
            ApprovedRelationTypeIR::Destination,
        ],
        optional_roles: &[
            ApprovedRelationTypeIR::Agent,
            ApprovedRelationTypeIR::Patient,
            ApprovedRelationTypeIR::Source,
            ApprovedRelationTypeIR::Date,
            ApprovedRelationTypeIR::Time,
            ApprovedRelationTypeIR::Location,
            ApprovedRelationTypeIR::Instrument,
            ApprovedRelationTypeIR::Manner,
        ],
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Transfer,
        predicate_sense: ApprovedEventPredicateSenseIR::Deliver,
        required_roles: &[
            ApprovedRelationTypeIR::Patient,
            ApprovedRelationTypeIR::Theme,
        ],
        optional_roles: &[
            ApprovedRelationTypeIR::Agent,
            ApprovedRelationTypeIR::Source,
            ApprovedRelationTypeIR::Destination,
            ApprovedRelationTypeIR::Date,
            ApprovedRelationTypeIR::Time,
            ApprovedRelationTypeIR::Location,
            ApprovedRelationTypeIR::Instrument,
            ApprovedRelationTypeIR::Manner,
        ],
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Transfer,
        predicate_sense: ApprovedEventPredicateSenseIR::Send,
        required_roles: &[
            ApprovedRelationTypeIR::Theme,
            ApprovedRelationTypeIR::Destination,
        ],
        optional_roles: &[
            ApprovedRelationTypeIR::Agent,
            ApprovedRelationTypeIR::Patient,
            ApprovedRelationTypeIR::Source,
            ApprovedRelationTypeIR::Date,
            ApprovedRelationTypeIR::Time,
            ApprovedRelationTypeIR::Location,
            ApprovedRelationTypeIR::Instrument,
            ApprovedRelationTypeIR::Manner,
        ],
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Transfer,
        predicate_sense: ApprovedEventPredicateSenseIR::Give,
        required_roles: &[
            ApprovedRelationTypeIR::Patient,
            ApprovedRelationTypeIR::Theme,
        ],
        optional_roles: &[
            ApprovedRelationTypeIR::Agent,
            ApprovedRelationTypeIR::Source,
            ApprovedRelationTypeIR::Destination,
            ApprovedRelationTypeIR::Date,
            ApprovedRelationTypeIR::Time,
            ApprovedRelationTypeIR::Location,
            ApprovedRelationTypeIR::Instrument,
            ApprovedRelationTypeIR::Manner,
        ],
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::Creation,
        predicate_sense: ApprovedEventPredicateSenseIR::Create,
        required_roles: &[ApprovedRelationTypeIR::Theme],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::StateChange,
        predicate_sense: ApprovedEventPredicateSenseIR::Change,
        required_roles: &[
            ApprovedRelationTypeIR::Theme,
            ApprovedRelationTypeIR::InitialState,
            ApprovedRelationTypeIR::ResultState,
        ],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::StateChange,
        predicate_sense: ApprovedEventPredicateSenseIR::Start,
        required_roles: &[ApprovedRelationTypeIR::Theme],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::StateChange,
        predicate_sense: ApprovedEventPredicateSenseIR::Complete,
        required_roles: &[ApprovedRelationTypeIR::Theme],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::StateChange,
        predicate_sense: ApprovedEventPredicateSenseIR::Open,
        required_roles: &[ApprovedRelationTypeIR::Theme],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
    ApprovedEventPredicateSpec {
        event_class: ApprovedEventRealizationClassIR::StateChange,
        predicate_sense: ApprovedEventPredicateSenseIR::Close,
        required_roles: &[ApprovedRelationTypeIR::Theme],
        optional_roles: COMMON_SEMANTIC_EVENT_ROLES,
        allowed_phases: ALL_SEMANTIC_EVENT_PHASES,
    },
];

pub(crate) fn approved_event_predicate_spec(
    event_class: ApprovedEventRealizationClassIR,
    predicate_sense: ApprovedEventPredicateSenseIR,
) -> Option<ApprovedEventPredicateSpec> {
    APPROVED_EVENT_PREDICATE_SPECS
        .iter()
        .copied()
        .find(|spec| spec.event_class == event_class && spec.predicate_sense == predicate_sense)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventVoiceIR {
    Active,
    Passive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventFocusIR {
    Event,
    Agent,
    Theme,
    Destination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventInformationRoleIR {
    Event,
    Agent,
    Patient,
    Theme,
    Source,
    Destination,
}

/// Typed discourse input for selecting an event's language-only information
/// structure. Node identity and semantic roles are authoritative inputs here;
/// lexical labels and surface words are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedEventDiscourseStateIR {
    pub schema: String,
    pub subject_node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_topic: Option<ApprovedEventInformationRoleIR>,
    pub requested_focus: ApprovedEventInformationRoleIR,
    #[serde(default)]
    pub backgrounded_roles: Vec<ApprovedRelationTypeIR>,
    #[serde(default)]
    pub recoverable_roles: Vec<ApprovedRelationTypeIR>,
}

impl ApprovedEventDiscourseStateIR {
    fn validate_shape(&self) -> bool {
        self.schema == APPROVED_EVENT_DISCOURSE_STATE_SCHEMA
            && valid_symbol(&self.subject_node_id)
            && !has_duplicates(&self.backgrounded_roles)
            && !has_duplicates(&self.recoverable_roles)
            && self
                .recoverable_roles
                .iter()
                .all(|role| self.backgrounded_roles.contains(role))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovedEventExpressionPreferenceIR {
    #[default]
    Auto,
    Concise,
    Explicit,
}

/// Binds an actual dialogue focus to an approved event by stable node identity.
/// Surface strings are carried by `DiscourseFocusStateIR` for display only and
/// are never consulted by this bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedEventPragmaticContextIR {
    pub schema: String,
    pub subject_node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_focus_node_id: Option<String>,
    #[serde(default)]
    pub backgrounded_node_ids: Vec<String>,
    #[serde(default)]
    pub recoverable_node_ids: Vec<String>,
    #[serde(default)]
    pub expression_preference: ApprovedEventExpressionPreferenceIR,
}

impl ApprovedEventPragmaticContextIR {
    fn validate_shape(&self) -> bool {
        let distinct =
            |values: &[String]| values.iter().collect::<BTreeSet<_>>().len() == values.len();
        self.schema == APPROVED_EVENT_PRAGMATIC_CONTEXT_SCHEMA
            && valid_symbol(&self.subject_node_id)
            && self
                .requested_focus_node_id
                .as_deref()
                .is_none_or(valid_symbol)
            && self.backgrounded_node_ids.iter().all(|id| valid_symbol(id))
            && self.recoverable_node_ids.iter().all(|id| valid_symbol(id))
            && distinct(&self.backgrounded_node_ids)
            && distinct(&self.recoverable_node_ids)
            && self
                .recoverable_node_ids
                .iter()
                .all(|id| self.backgrounded_node_ids.contains(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedEventInformationStructureIR {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<ApprovedEventInformationRoleIR>,
    pub focus: ApprovedEventInformationRoleIR,
    #[serde(default)]
    pub omitted_roles: Vec<ApprovedRelationTypeIR>,
    #[serde(default)]
    pub recoverable_roles: Vec<ApprovedRelationTypeIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedEventPerspectiveIR {
    pub voice: ApprovedEventVoiceIR,
    pub focus: ApprovedEventFocusIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedEventRealizationIR {
    pub subject_node_id: String,
    pub class: ApprovedEventRealizationClassIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate_sense: Option<ApprovedEventPredicateSenseIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<ApprovedEventPhaseIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<ApprovedEventPerspectiveIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<ApprovedEventVoiceIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub information_structure: Option<ApprovedEventInformationStructureIR>,
}

impl ApprovedCompositionalClaimIR {
    fn validate(&self) -> bool {
        valid_symbol(&self.proposition_id) && self.subject.validate() && self.value.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedCompositionalResponseIR {
    pub schema: String,
    pub speech_act: ApprovedSpeechActIR,
    pub operation: ApprovedOperationIR,
    pub claims: Vec<ApprovedCompositionalClaimIR>,
    #[serde(default)]
    pub event_realizations: Vec<ApprovedEventRealizationIR>,
    pub discourse_relation: ApprovedDiscourseRelationIR,
    pub clause_plan: Vec<ApprovedClauseUnitIR>,
    pub style: ApprovedResponseStyleIR,
    pub source_world_state_sha256: String,
    pub source_deliberation_sha256: String,
    pub approval_replay_verified: bool,
    pub unsupported_claims: usize,
    pub semantic_sha256: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApprovedCompositionalResponseBuilder;

impl ApprovedCompositionalResponseBuilder {
    pub fn enrich(
        &self,
        approved: &ApprovedResponseIR,
        claims: Vec<ApprovedCompositionalClaimIR>,
    ) -> Result<ApprovedCompositionalResponseIR, String> {
        self.enrich_with_event_realizations(approved, claims, Vec::new())
    }

    pub fn enrich_with_event_realizations(
        &self,
        approved: &ApprovedResponseIR,
        claims: Vec<ApprovedCompositionalClaimIR>,
        event_realizations: Vec<ApprovedEventRealizationIR>,
    ) -> Result<ApprovedCompositionalResponseIR, String> {
        if !approved.validate() || claims.len() != approved.claims.len() {
            return Err("INVALID_APPROVED_RESPONSE_SOURCE".into());
        }
        for source in &approved.claims {
            let Some(enriched) = claims
                .iter()
                .find(|claim| claim.proposition_id == source.proposition_id)
            else {
                return Err("MISSING_COMPOSITIONAL_CLAIM".into());
            };
            if !enriched.validate()
                || enriched.subject.node_id != source.subject
                || enriched.relation.symbol() != source.property
                || enriched.polarity != source.polarity
                || !open_value_matches_source(&enriched.value, &source.value)
            {
                return Err("COMPOSITIONAL_CLAIM_DOES_NOT_MATCH_APPROVED_MEANING".into());
            }
        }
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: approved.speech_act,
            operation: operation_for(approved.discourse_relation),
            claims,
            event_realizations,
            discourse_relation: approved.discourse_relation,
            clause_plan: approved.clause_plan.clone(),
            style: approved.style.clone(),
            source_world_state_sha256: approved.source_world_state_sha256.clone(),
            source_deliberation_sha256: approved.source_deliberation_sha256.clone(),
            approval_replay_verified: approved.approval_replay_verified,
            unsupported_claims: approved.unsupported_claims,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        response
            .validate()
            .then_some(response)
            .ok_or_else(|| "COMPOSITIONAL_APPROVED_RESPONSE_VALIDATION_FAILED".into())
    }
}

impl ApprovedCompositionalResponseIR {
    pub fn validate(&self) -> bool {
        self.schema == COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA
            && !self.claims.is_empty()
            && self.claims.len() <= MAX_APPROVED_RESPONSE_CLAIMS
            && self
                .claims
                .iter()
                .all(ApprovedCompositionalClaimIR::validate)
            && self.event_realizations.len() <= self.claims.len()
            && self
                .event_realizations
                .iter()
                .map(|realization| realization.subject_node_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                == self.event_realizations.len()
            && self.event_realizations.iter().all(|realization| {
                valid_symbol(&realization.subject_node_id)
                    && self.claims.iter().any(|claim| {
                        claim.subject.node_id == realization.subject_node_id
                            && claim.subject.semantic_type == ApprovedSemanticTypeIR::Event
                    })
                    && realization.phase.is_none_or(|phase| {
                        event_class_allows_phase(
                            realization.class,
                            realization.predicate_sense,
                            phase,
                        )
                    })
                    && event_class_allows_predicate_sense(
                        realization.class,
                        realization.predicate_sense,
                    )
                    && event_class_allows_perspective(
                        realization.class,
                        realization.predicate_sense,
                        realization.perspective,
                    )
                    && event_information_structure_is_valid(self, realization)
            })
            && self.operation == operation_for(self.discourse_relation)
            && self.clause_plan.iter().all(|unit| {
                unit.proposition_ids
                    .iter()
                    .all(|id| self.claims.iter().any(|claim| claim.proposition_id == *id))
            })
            && sha256_shape(&self.source_world_state_sha256)
            && sha256_shape(&self.source_deliberation_sha256)
            && self.approval_replay_verified
            && self.unsupported_claims == 0
            && self.semantic_sha256 == compositional_response_sha256(self)
    }

    /// Projects typed discourse state into language-only topic/focus/voice.
    ///
    /// Explicit information structure (including the legacy perspective) is
    /// preserved. A missing, ambiguous, contradictory, or unsupported state
    /// leaves the semantic event realization unprojected, which makes the
    /// document codec use its ordinary claim fallback instead of guessing.
    pub fn apply_event_discourse_states(
        &self,
        states: &[ApprovedEventDiscourseStateIR],
    ) -> Result<Self, String> {
        if !self.validate()
            || states.iter().any(|state| {
                !state.validate_shape()
                    || !self
                        .event_realizations
                        .iter()
                        .any(|realization| realization.subject_node_id == state.subject_node_id)
            })
        {
            return Err("INVALID_APPROVED_EVENT_DISCOURSE_STATE".into());
        }

        let mut projected = self.clone();
        for realization in &mut projected.event_realizations {
            if realization.perspective.is_some()
                || realization.voice.is_some()
                || realization.information_structure.is_some()
            {
                continue;
            }
            let matching = states
                .iter()
                .filter(|state| state.subject_node_id == realization.subject_node_id)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                continue;
            }
            if let Some((voice, information)) =
                select_event_information_structure(&projected.claims, realization, matching[0])
            {
                realization.voice = Some(voice);
                realization.information_structure = Some(information);
            }
        }
        projected.semantic_sha256 = compositional_response_sha256(&projected);
        projected
            .validate()
            .then_some(projected)
            .ok_or_else(|| "PROJECTED_APPROVED_RESPONSE_VALIDATION_FAILED".into())
    }

    /// Derives language-only event discourse states from the live dialogue
    /// focus and typed event node bindings. Only stable node identity crosses
    /// this boundary; focus surface text never controls semantic role choice.
    pub fn derive_event_discourse_states(
        &self,
        discourse_focus: &DiscourseFocusStateIR,
        completed_turns: u64,
        contexts: &[ApprovedEventPragmaticContextIR],
    ) -> Result<Vec<ApprovedEventDiscourseStateIR>, String> {
        if !self.validate()
            || !discourse_focus.validate(completed_turns)
            || contexts.iter().any(|context| {
                !context.validate_shape()
                    || !self
                        .event_realizations
                        .iter()
                        .any(|realization| realization.subject_node_id == context.subject_node_id)
            })
            || contexts
                .iter()
                .map(|context| context.subject_node_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != contexts.len()
        {
            return Err("INVALID_APPROVED_EVENT_PRAGMATIC_CONTEXT".into());
        }

        let mut states = Vec::new();
        for context in contexts {
            let event_claims = self
                .claims
                .iter()
                .filter(|claim| claim.subject.node_id == context.subject_node_id)
                .collect::<Vec<_>>();
            let requested_focus = match context.requested_focus_node_id.as_deref() {
                None => Some(ApprovedEventInformationRoleIR::Event),
                Some(node_id) if node_id == context.subject_node_id => {
                    Some(ApprovedEventInformationRoleIR::Event)
                }
                Some(node_id) => event_role_for_node(&event_claims, node_id)
                    .and_then(information_role_for_relation),
            };
            let Some(requested_focus) = requested_focus else {
                continue;
            };

            let current_topic = discourse_focus
                .current()
                .and_then(|node| node.concept_id_hint.as_deref())
                .filter(|node_id| *node_id != context.subject_node_id)
                .and_then(|node_id| event_role_for_node(&event_claims, node_id))
                .and_then(information_role_for_relation);

            let Some(backgrounded_roles) =
                map_event_node_ids_to_roles(&event_claims, &context.backgrounded_node_ids)
            else {
                continue;
            };
            let Some(mut recoverable_roles) =
                map_event_node_ids_to_roles(&event_claims, &context.recoverable_node_ids)
            else {
                continue;
            };
            let explicit = match context.expression_preference {
                ApprovedEventExpressionPreferenceIR::Auto => {
                    self.style.verbosity == ApprovedVerbosityIR::Explanatory
                }
                ApprovedEventExpressionPreferenceIR::Concise => false,
                ApprovedEventExpressionPreferenceIR::Explicit => true,
            };
            if explicit {
                recoverable_roles.clear();
            }

            states.push(ApprovedEventDiscourseStateIR {
                schema: APPROVED_EVENT_DISCOURSE_STATE_SCHEMA.into(),
                subject_node_id: context.subject_node_id.clone(),
                current_topic,
                requested_focus,
                backgrounded_roles,
                recoverable_roles,
            });
        }
        Ok(states)
    }

    pub fn apply_event_pragmatic_contexts(
        &self,
        discourse_focus: &DiscourseFocusStateIR,
        completed_turns: u64,
        contexts: &[ApprovedEventPragmaticContextIR],
    ) -> Result<Self, String> {
        let states =
            self.derive_event_discourse_states(discourse_focus, completed_turns, contexts)?;
        self.apply_event_discourse_states(&states)
    }
}

fn event_role_for_node(
    event_claims: &[&ApprovedCompositionalClaimIR],
    node_id: &str,
) -> Option<ApprovedRelationTypeIR> {
    let roles = event_claims
        .iter()
        .filter_map(|claim| match &claim.value {
            ApprovedOpenValueIR::Lexical(node) if node.node_id == node_id => Some(claim.relation),
            _ => None,
        })
        .collect::<Vec<_>>();
    let first = *roles.first()?;
    roles.iter().all(|role| *role == first).then_some(first)
}

fn information_role_for_relation(
    role: ApprovedRelationTypeIR,
) -> Option<ApprovedEventInformationRoleIR> {
    match role {
        ApprovedRelationTypeIR::Agent => Some(ApprovedEventInformationRoleIR::Agent),
        ApprovedRelationTypeIR::Patient => Some(ApprovedEventInformationRoleIR::Patient),
        ApprovedRelationTypeIR::Theme => Some(ApprovedEventInformationRoleIR::Theme),
        ApprovedRelationTypeIR::Source => Some(ApprovedEventInformationRoleIR::Source),
        ApprovedRelationTypeIR::Destination => Some(ApprovedEventInformationRoleIR::Destination),
        _ => None,
    }
}

fn map_event_node_ids_to_roles(
    event_claims: &[&ApprovedCompositionalClaimIR],
    node_ids: &[String],
) -> Option<Vec<ApprovedRelationTypeIR>> {
    let roles = node_ids
        .iter()
        .map(|node_id| event_role_for_node(event_claims, node_id))
        .collect::<Option<Vec<_>>>()?;
    (!has_duplicates(&roles)).then_some(roles)
}

fn select_event_information_structure(
    claims: &[ApprovedCompositionalClaimIR],
    realization: &ApprovedEventRealizationIR,
    state: &ApprovedEventDiscourseStateIR,
) -> Option<(ApprovedEventVoiceIR, ApprovedEventInformationStructureIR)> {
    let predicate_sense = realization.predicate_sense?;
    let spec = approved_event_predicate_spec(realization.class, predicate_sense)?;
    let event_claims = claims
        .iter()
        .filter(|claim| claim.subject.node_id == realization.subject_node_id)
        .collect::<Vec<_>>();
    let has_role = |role| event_claims.iter().any(|claim| claim.relation == role);
    let role_is_present = |role| information_role_relation(role).is_none_or(has_role);

    if !role_is_present(state.requested_focus)
        || state
            .current_topic
            .is_some_and(|topic| !role_is_present(topic))
        || state
            .backgrounded_roles
            .iter()
            .chain(&state.recoverable_roles)
            .any(|role| !spec.allows_role(*role) || !has_role(*role))
        || state
            .recoverable_roles
            .iter()
            .any(|role| *role != ApprovedRelationTypeIR::Agent)
    {
        return None;
    }

    let agent_present = has_role(ApprovedRelationTypeIR::Agent);
    let agent_backgrounded = state
        .backgrounded_roles
        .contains(&ApprovedRelationTypeIR::Agent);
    let omit_agent = agent_backgrounded
        && state
            .recoverable_roles
            .contains(&ApprovedRelationTypeIR::Agent);
    let passive_information = |topic| ApprovedEventInformationStructureIR {
        topic,
        focus: ApprovedEventInformationRoleIR::Theme,
        omitted_roles: omit_agent
            .then_some(ApprovedRelationTypeIR::Agent)
            .into_iter()
            .collect(),
        recoverable_roles: omit_agent
            .then_some(ApprovedRelationTypeIR::Agent)
            .into_iter()
            .collect(),
    };
    let active_information = |topic, focus| ApprovedEventInformationStructureIR {
        topic,
        focus,
        omitted_roles: Vec::new(),
        recoverable_roles: Vec::new(),
    };

    match state.requested_focus {
        ApprovedEventInformationRoleIR::Event => {
            if !agent_present || agent_backgrounded {
                None
            } else {
                Some((
                    ApprovedEventVoiceIR::Active,
                    active_information(state.current_topic, ApprovedEventInformationRoleIR::Event),
                ))
            }
        }
        ApprovedEventInformationRoleIR::Agent => {
            if !agent_present || agent_backgrounded {
                None
            } else {
                Some((
                    ApprovedEventVoiceIR::Active,
                    active_information(state.current_topic, ApprovedEventInformationRoleIR::Agent),
                ))
            }
        }
        ApprovedEventInformationRoleIR::Theme => {
            if !agent_present || omit_agent {
                Some((
                    ApprovedEventVoiceIR::Passive,
                    passive_information(state.current_topic),
                ))
            } else {
                Some((
                    ApprovedEventVoiceIR::Active,
                    active_information(
                        state
                            .current_topic
                            .or(Some(ApprovedEventInformationRoleIR::Theme)),
                        ApprovedEventInformationRoleIR::Agent,
                    ),
                ))
            }
        }
        requested @ (ApprovedEventInformationRoleIR::Patient
        | ApprovedEventInformationRoleIR::Source
        | ApprovedEventInformationRoleIR::Destination) => {
            if state.current_topic.is_some_and(|topic| topic != requested) {
                return None;
            }
            let topic = Some(requested);
            if !agent_present || omit_agent {
                Some((ApprovedEventVoiceIR::Passive, passive_information(topic)))
            } else {
                Some((
                    ApprovedEventVoiceIR::Active,
                    active_information(topic, ApprovedEventInformationRoleIR::Agent),
                ))
            }
        }
    }
}

fn information_role_relation(
    role: ApprovedEventInformationRoleIR,
) -> Option<ApprovedRelationTypeIR> {
    match role {
        ApprovedEventInformationRoleIR::Event => None,
        ApprovedEventInformationRoleIR::Agent => Some(ApprovedRelationTypeIR::Agent),
        ApprovedEventInformationRoleIR::Patient => Some(ApprovedRelationTypeIR::Patient),
        ApprovedEventInformationRoleIR::Theme => Some(ApprovedRelationTypeIR::Theme),
        ApprovedEventInformationRoleIR::Source => Some(ApprovedRelationTypeIR::Source),
        ApprovedEventInformationRoleIR::Destination => Some(ApprovedRelationTypeIR::Destination),
    }
}

fn event_information_structure_is_valid(
    response: &ApprovedCompositionalResponseIR,
    realization: &ApprovedEventRealizationIR,
) -> bool {
    let typed_present = realization.voice.is_some() || realization.information_structure.is_some();
    if realization.perspective.is_some() {
        return !typed_present;
    }
    if !typed_present {
        return true;
    }
    let (Some(voice), Some(information), Some(predicate_sense)) = (
        realization.voice,
        realization.information_structure.as_ref(),
        realization.predicate_sense,
    ) else {
        return false;
    };
    let Some(spec) = approved_event_predicate_spec(realization.class, predicate_sense) else {
        return false;
    };
    let claims = response
        .claims
        .iter()
        .filter(|claim| claim.subject.node_id == realization.subject_node_id)
        .collect::<Vec<_>>();
    let has_role = |role| claims.iter().any(|claim| claim.relation == role);
    let role_is_present = |role| information_role_relation(role).is_none_or(has_role);
    if !role_is_present(information.focus)
        || information
            .topic
            .is_some_and(|topic| !role_is_present(topic))
        || information
            .omitted_roles
            .iter()
            .chain(&information.recoverable_roles)
            .any(|role| !spec.allows_role(*role) || !has_role(*role))
        || has_duplicates(&information.omitted_roles)
        || has_duplicates(&information.recoverable_roles)
        || information
            .omitted_roles
            .iter()
            .any(|role| !information.recoverable_roles.contains(role))
        || information
            .omitted_roles
            .iter()
            .any(|role| *role != ApprovedRelationTypeIR::Agent)
    {
        return false;
    }
    let topic_relation = information.topic.and_then(information_role_relation);
    let focus_relation = information_role_relation(information.focus);
    if information
        .omitted_roles
        .iter()
        .any(|role| Some(*role) == topic_relation || Some(*role) == focus_relation)
    {
        return false;
    }
    match voice {
        ApprovedEventVoiceIR::Active => {
            has_role(ApprovedRelationTypeIR::Agent)
                && information.omitted_roles.is_empty()
                && matches!(
                    information.focus,
                    ApprovedEventInformationRoleIR::Agent | ApprovedEventInformationRoleIR::Event
                )
                && matches!(
                    information.topic,
                    None | Some(ApprovedEventInformationRoleIR::Agent)
                        | Some(ApprovedEventInformationRoleIR::Patient)
                        | Some(ApprovedEventInformationRoleIR::Theme)
                        | Some(ApprovedEventInformationRoleIR::Source)
                        | Some(ApprovedEventInformationRoleIR::Destination)
                )
        }
        ApprovedEventVoiceIR::Passive => {
            information.focus == ApprovedEventInformationRoleIR::Theme
                && matches!(
                    information.topic,
                    None | Some(ApprovedEventInformationRoleIR::Theme)
                        | Some(ApprovedEventInformationRoleIR::Patient)
                        | Some(ApprovedEventInformationRoleIR::Source)
                        | Some(ApprovedEventInformationRoleIR::Destination)
                )
        }
    }
}

fn has_duplicates(values: &[ApprovedRelationTypeIR]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
}

fn event_class_allows_phase(
    class: ApprovedEventRealizationClassIR,
    predicate_sense: Option<ApprovedEventPredicateSenseIR>,
    phase: ApprovedEventPhaseIR,
) -> bool {
    if let Some(predicate_sense) = predicate_sense {
        return approved_event_predicate_spec(class, predicate_sense)
            .is_some_and(|spec| spec.allows_phase(phase));
    }
    match class {
        ApprovedEventRealizationClassIR::HostedEvent
        | ApprovedEventRealizationClassIR::ScheduledProcess
        | ApprovedEventRealizationClassIR::Motion
        | ApprovedEventRealizationClassIR::Transfer
        | ApprovedEventRealizationClassIR::Creation
        | ApprovedEventRealizationClassIR::StateChange => true,
        ApprovedEventRealizationClassIR::Departure => {
            !matches!(phase, ApprovedEventPhaseIR::Ongoing)
        }
        ApprovedEventRealizationClassIR::Arrival => matches!(
            phase,
            ApprovedEventPhaseIR::Planned
                | ApprovedEventPhaseIR::Scheduled
                | ApprovedEventPhaseIR::Completed
        ),
    }
}

fn event_class_allows_predicate_sense(
    class: ApprovedEventRealizationClassIR,
    predicate_sense: Option<ApprovedEventPredicateSenseIR>,
) -> bool {
    match predicate_sense {
        Some(predicate_sense) => approved_event_predicate_spec(class, predicate_sense).is_some(),
        None => matches!(
            class,
            ApprovedEventRealizationClassIR::HostedEvent
                | ApprovedEventRealizationClassIR::ScheduledProcess
                | ApprovedEventRealizationClassIR::Departure
                | ApprovedEventRealizationClassIR::Arrival
                | ApprovedEventRealizationClassIR::Motion
                | ApprovedEventRealizationClassIR::Transfer
                | ApprovedEventRealizationClassIR::Creation
                | ApprovedEventRealizationClassIR::StateChange
        ),
    }
}

fn event_class_allows_perspective(
    class: ApprovedEventRealizationClassIR,
    predicate_sense: Option<ApprovedEventPredicateSenseIR>,
    perspective: Option<ApprovedEventPerspectiveIR>,
) -> bool {
    let Some(perspective) = perspective else {
        return true;
    };
    let Some(predicate_sense) = predicate_sense else {
        return matches!(
            class,
            ApprovedEventRealizationClassIR::Motion
                | ApprovedEventRealizationClassIR::Transfer
                | ApprovedEventRealizationClassIR::Creation
                | ApprovedEventRealizationClassIR::StateChange
        );
    };
    let Some(spec) = approved_event_predicate_spec(class, predicate_sense) else {
        return false;
    };
    let focus_role = match perspective.focus {
        ApprovedEventFocusIR::Event => return false,
        ApprovedEventFocusIR::Agent => ApprovedRelationTypeIR::Agent,
        ApprovedEventFocusIR::Theme => ApprovedRelationTypeIR::Theme,
        ApprovedEventFocusIR::Destination => ApprovedRelationTypeIR::Destination,
    };
    spec.allows_role(focus_role)
        && match perspective.voice {
            ApprovedEventVoiceIR::Active => spec.allows_role(ApprovedRelationTypeIR::Agent),
            ApprovedEventVoiceIR::Passive => {
                perspective.focus == ApprovedEventFocusIR::Theme
                    && spec.allows_role(ApprovedRelationTypeIR::Theme)
            }
        }
}

pub fn compositional_response_sha256(response: &ApprovedCompositionalResponseIR) -> String {
    let mut canonical = response.clone();
    canonical.semantic_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&canonical).expect("Compositional Approved Response IR serializes")
        )
    )
}

fn operation_for(relation: ApprovedDiscourseRelationIR) -> ApprovedOperationIR {
    match relation {
        ApprovedDiscourseRelationIR::Statement | ApprovedDiscourseRelationIR::Temporal => {
            ApprovedOperationIR::Assert
        }
        ApprovedDiscourseRelationIR::Correction => ApprovedOperationIR::Revise,
        ApprovedDiscourseRelationIR::Negation => ApprovedOperationIR::Negate,
        ApprovedDiscourseRelationIR::Comparison => ApprovedOperationIR::Compare,
        ApprovedDiscourseRelationIR::Cause | ApprovedDiscourseRelationIR::Explanation => {
            ApprovedOperationIR::Explain
        }
        ApprovedDiscourseRelationIR::Condition => ApprovedOperationIR::Condition,
        ApprovedDiscourseRelationIR::Recall => ApprovedOperationIR::Recall,
        ApprovedDiscourseRelationIR::Confirmation => ApprovedOperationIR::Confirm,
    }
}

fn open_value_matches_source(open: &ApprovedOpenValueIR, source: &ApprovedValueIR) -> bool {
    match (open, source) {
        (ApprovedOpenValueIR::Lexical(node), ApprovedValueIR::Symbol(symbol)) => {
            node.node_id == *symbol
        }
        (ApprovedOpenValueIR::Text(left), ApprovedValueIR::Text(right)) => left == right,
        (ApprovedOpenValueIR::Boolean(left), ApprovedValueIR::Boolean(right)) => left == right,
        (ApprovedOpenValueIR::Integer(left), ApprovedValueIR::Integer(right)) => left == right,
        (
            ApprovedOpenValueIR::Clock {
                hour: lh,
                minute: lm,
            },
            ApprovedValueIR::Clock {
                hour: rh,
                minute: rm,
            },
        ) => lh == rh && lm == rm,
        (
            ApprovedOpenValueIR::Date {
                year: ly,
                month: lm,
                day: ld,
            },
            ApprovedValueIR::Date {
                year: ry,
                month: rm,
                day: rd,
            },
        ) => ly == ry && lm == rm && ld == rd,
        (
            ApprovedOpenValueIR::Quantity { amount: left, unit },
            ApprovedValueIR::Quantity {
                amount: right,
                unit: source_unit,
            },
        ) => left == right && unit.node_id == *source_unit,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedClaimBindingIR {
    pub proposition_id: String,
    pub subject: String,
    pub property: String,
    pub value: ApprovedValueIR,
    pub polarity: bool,
}

impl ApprovedClaimBindingIR {
    fn validate(&self) -> bool {
        valid_symbol(&self.proposition_id)
            && valid_symbol(&self.subject)
            && valid_symbol(&self.property)
            && self.value.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedClauseUnitIR {
    pub unit_index: usize,
    pub role: String,
    pub proposition_ids: Vec<String>,
    pub predecessor_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedResponseStyleIR {
    pub register: LanguageRegisterIR,
    pub verbosity: ApprovedVerbosityIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedResponseIR {
    pub schema: String,
    pub speech_act: ApprovedSpeechActIR,
    pub claims: Vec<ApprovedClaimBindingIR>,
    pub discourse_relation: ApprovedDiscourseRelationIR,
    pub clause_plan: Vec<ApprovedClauseUnitIR>,
    pub style: ApprovedResponseStyleIR,
    pub source_world_state_sha256: String,
    pub source_deliberation_sha256: String,
    pub approval_replay_verified: bool,
    pub unsupported_claims: usize,
    pub semantic_sha256: String,
}

#[derive(Debug, Clone)]
pub struct ApprovedResponseCandidateIR {
    pub world_state_sha256: String,
    pub request: DeliberationRequestIR,
    pub deliberation: DeliberationIR,
    pub speech_act: ApprovedSpeechActIR,
    pub claims: Vec<ApprovedClaimBindingIR>,
    pub discourse_relation: ApprovedDiscourseRelationIR,
    pub style: ApprovedResponseStyleIR,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApprovedResponseBuilder;

impl ApprovedResponseBuilder {
    pub fn approve(
        &self,
        candidate: ApprovedResponseCandidateIR,
    ) -> Result<ApprovedResponseIR, String> {
        if !sha256_shape(&candidate.world_state_sha256)
            || candidate.claims.is_empty()
            || candidate.claims.len() > MAX_APPROVED_RESPONSE_CLAIMS
            || candidate.claims.iter().any(|claim| !claim.validate())
        {
            return Err("INVALID_APPROVED_RESPONSE_CANDIDATE".into());
        }
        let replay = DeliberationEngine
            .deliberate(&candidate.request)
            .map_err(|_| "DELIBERATION_REPLAY_FAILED")?;
        if replay != candidate.deliberation
            || !matches!(
                replay.disposition,
                DeliberationDispositionIR::GoalAlreadySatisfied
                    | DeliberationDispositionIR::GoalReachable
            )
            || !replay
                .selected_plan
                .as_ref()
                .is_some_and(|plan| plan.reaches_goal)
        {
            return Err("UNAPPROVED_DELIBERATION".into());
        }
        let known = candidate
            .request
            .evidence
            .iter()
            .map(|item| item.literal.proposition_id.as_str())
            .chain(candidate.request.mechanisms.iter().flat_map(|mechanism| {
                mechanism
                    .prerequisites
                    .iter()
                    .chain(&mechanism.effects)
                    .map(|literal| literal.proposition_id.as_str())
            }))
            .chain(
                candidate
                    .request
                    .goals
                    .iter()
                    .map(|goal| goal.proposition_id.as_str()),
            )
            .collect::<BTreeSet<_>>();
        let ids = candidate
            .claims
            .iter()
            .map(|claim| claim.proposition_id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != candidate.claims.len()
            || candidate
                .claims
                .iter()
                .any(|claim| !known.contains(claim.proposition_id.as_str()))
            || candidate.request.goals.iter().any(|goal| {
                !candidate.claims.iter().any(|claim| {
                    claim.proposition_id == goal.proposition_id && claim.polarity == goal.value
                })
            })
        {
            return Err("UNBOUND_OR_UNSUPPORTED_CLAIM".into());
        }
        let clause_plan = clause_plan(candidate.discourse_relation, &candidate.claims);
        let mut approved = ApprovedResponseIR {
            schema: APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: candidate.speech_act,
            claims: candidate.claims,
            discourse_relation: candidate.discourse_relation,
            clause_plan,
            style: candidate.style,
            source_world_state_sha256: candidate.world_state_sha256,
            source_deliberation_sha256: replay.deliberation_sha256,
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        approved.semantic_sha256 = approved_response_sha256(&approved);
        approved
            .validate()
            .then_some(approved)
            .ok_or_else(|| "APPROVED_RESPONSE_VALIDATION_FAILED".into())
    }
}

impl ApprovedResponseIR {
    pub fn validate(&self) -> bool {
        self.schema == APPROVED_RESPONSE_SCHEMA
            && !self.claims.is_empty()
            && self.claims.len() <= MAX_APPROVED_RESPONSE_CLAIMS
            && self.claims.iter().all(ApprovedClaimBindingIR::validate)
            && self.clause_plan == clause_plan(self.discourse_relation, &self.claims)
            && sha256_shape(&self.source_world_state_sha256)
            && sha256_shape(&self.source_deliberation_sha256)
            && self.approval_replay_verified
            && self.unsupported_claims == 0
            && self.semantic_sha256 == approved_response_sha256(self)
    }
}

pub fn approved_response_sha256(response: &ApprovedResponseIR) -> String {
    let mut canonical = response.clone();
    canonical.semantic_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).expect("Approved Response IR serializes"))
    )
}

fn clause_plan(
    relation: ApprovedDiscourseRelationIR,
    claims: &[ApprovedClaimBindingIR],
) -> Vec<ApprovedClauseUnitIR> {
    let mut units = Vec::new();
    if relation == ApprovedDiscourseRelationIR::Correction {
        units.push(ApprovedClauseUnitIR {
            unit_index: 0,
            role: "ACKNOWLEDGE_CORRECTION".into(),
            proposition_ids: Vec::new(),
            predecessor_indices: Vec::new(),
        });
    }
    for claim in claims {
        let index = units.len();
        units.push(ApprovedClauseUnitIR {
            unit_index: index,
            role: match relation {
                ApprovedDiscourseRelationIR::Cause if index + 1 < claims.len() => "CAUSE",
                ApprovedDiscourseRelationIR::Condition if index + 1 < claims.len() => "CONDITION",
                ApprovedDiscourseRelationIR::Explanation if index + 1 < claims.len() => "SUPPORT",
                _ => "ASSERT",
            }
            .into(),
            proposition_ids: vec![claim.proposition_id.clone()],
            predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
        });
    }
    units
}

fn valid_symbol(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 128
        && value.chars().all(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | ':' | '.')
        })
}

fn valid_lexical_label(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= 256
        && value.chars().all(|character| !character.is_control())
}

fn sha256_shape(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
