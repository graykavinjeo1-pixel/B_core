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

use crate::attribution::{
    AttributedPropositionPolarityIR, AttributionAttitudeIR, EpistemicStatusIR,
};
use crate::conditional_guard::{ConditionalGuardEvaluationIR, ConditionalGuardStoreIR};
use crate::epistemic::{proposition_signature, EpistemicLedgerIR, EpistemicObservationIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::{ConditionalRelationIR, ModalSemanticAnalyzer, ModalWorldIR};
use crate::temporal::{TemporalGraphIR, TemporalTurnAnalysisIR};

pub const CONVERSATION_TURN_REQUEST_SCHEMA: &str = "B_CORE_CONVERSATION_TURN_REQUEST_1";
pub const CONVERSATION_FRONTEND_SCHEMA: &str = "B_CORE_CONVERSATION_FRONTEND_2";
pub const CONVERSATION_STATE_SCHEMA: &str = "B_CORE_CONVERSATION_STATE_8";
pub const CONVERSATIONAL_CONCEPT_SCHEMA: &str = "B_CORE_CONVERSATIONAL_CONCEPT_1";
const MAX_ALTERNATIVES: usize = 8;
const MAX_ACTIVE_REFERENTS: usize = 8;
const MAX_ACTIVE_GOALS: usize = 8;
const MAX_DISCOURSE_REFERENTS: usize = 12;
const MAX_REFERENCE_TURN_DISTANCE: u64 = 4;
const MAX_GOAL_ELLIPSIS_TURN_DISTANCE: u64 = 3;

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
    EllipticalAction,
    RepeatedGoal,
    CorrectedArgument,
    EventReference,
    ResultReference,
    PropositionReference,
    LocalAntecedentReference,
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

pub struct ConversationCommitContext<'a> {
    pub semantic_subject: Option<&'a str>,
    pub used_referent_ids: &'a [String],
    pub unresolved_reference_count: usize,
    pub language: Option<LanguageCodeIR>,
    pub grounded_goals: &'a [ConversationGoalFrameIR],
    pub proposition_referents: &'a [DynamicDiscourseReferentIR],
    pub temporal_analysis: Option<&'a TemporalTurnAnalysisIR>,
    pub guard_conditionals: Option<&'a [ConditionalRelationIR]>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationStateIR {
    pub schema: String,
    pub conversation_id: String,
    pub completed_turns: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_subject: Option<String>,
    pub active_referents: Vec<DynamicReferentIR>,
    #[serde(default)]
    pub active_goals: Vec<ConversationGoalFrameIR>,
    #[serde(default)]
    pub active_discourse_referents: Vec<DynamicDiscourseReferentIR>,
    #[serde(default)]
    pub epistemic_ledger: EpistemicLedgerIR,
    #[serde(default)]
    pub temporal_graph: TemporalGraphIR,
    #[serde(default)]
    pub conditional_guard_store: ConditionalGuardStoreIR,
    #[serde(default)]
    pub last_guard_evaluations: Vec<ConditionalGuardEvaluationIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_language: Option<LanguageCodeIR>,
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
    #[serde(default)]
    pub discourse_bindings: Vec<DiscourseBindingIR>,
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
                discourse_events.push(event(function, &token, "C_DIALOGUE_SOCIAL_ACT", 950));
                semantic_replacements.push(None);
                continue;
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
            if let Some((canonical, confidence, kind)) = repair_token(&lower) {
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
    if matches!(token, "저기" | "excuse" | "well") && index == 0 && token_count > 1 {
        return Some(DiscourseFunctionIR::AttentionCall);
    }
    None
}

fn backchannel_function(token: &str) -> Option<DiscourseFunctionIR> {
    match token {
        "응" | "네" | "넵" | "yeah" | "yep" | "okay" | "ok" => {
            Some(DiscourseFunctionIR::Acknowledge)
        }
        "맞아" | "맞습니다" | "ㅇㅋ" | "right" | "correct" => {
            Some(DiscourseFunctionIR::Approve)
        }
        "아니" | "아니야" | "ㄴㄴ" | "no" | "nope" => Some(DiscourseFunctionIR::Reject),
        _ => None,
    }
}

fn social_function(token: &str) -> Option<DiscourseFunctionIR> {
    match token {
        "안녕" | "안녕하세요" | "반가워" | "hello" | "hi" | "hey" => {
            Some(DiscourseFunctionIR::Greeting)
        }
        "고마워" | "감사" | "감사해" | "감사합니다" | "thanks" | "thankyou" => {
            Some(DiscourseFunctionIR::Gratitude)
        }
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
    if !token.is_ascii()
        && KOREAN_PARTICLES.iter().any(|particle| {
            token
                .strip_suffix(particle)
                .is_some_and(|stem| CANONICAL_CONTROL_FORMS.contains(&stem))
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
    // Two-syllable Korean words are dense in valid vocabulary (for example,
    // `실제` versus the control verb `실행`). A one-edit fuzzy rewrite at that
    // length destroys meaning more often than it repairs a typo. Known short
    // typos remain covered by the explicit table above.
    let minimum = if token.is_ascii() { 4 } else { 3 };
    if token.chars().count() < minimum {
        return None;
    }
    let matches = CANONICAL_CONTROL_FORMS
        .iter()
        .filter(|candidate| damerau_levenshtein(token, candidate) == 1)
        .copied()
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| {
        (
            matches[0].to_string(),
            820,
            NormalizationOperationKindIR::UniqueFuzzyMatch,
        )
    })
}

fn damerau_levenshtein(left: &str, right: &str) -> usize {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    let mut distance = vec![vec![0; right.len() + 1]; left.len() + 1];
    for (index, row) in distance.iter_mut().enumerate() {
        row[0] = index;
    }
    for (index, cell) in distance[0].iter_mut().enumerate() {
        *cell = index;
    }
    for i in 1..=left.len() {
        for j in 1..=right.len() {
            let substitution = usize::from(left[i - 1] != right[j - 1]);
            distance[i][j] = distance[i - 1][j]
                .saturating_add(1)
                .min(distance[i][j - 1].saturating_add(1))
                .min(distance[i - 1][j - 1].saturating_add(substitution));
            if i > 1 && j > 1 && left[i - 1] == right[j - 2] && left[i - 2] == right[j - 1] {
                distance[i][j] = distance[i][j].min(distance[i - 2][j - 2].saturating_add(1));
            }
        }
    }
    distance[left.len()][right.len()]
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
    pub fn state(&self, conversation_id: &str) -> Option<&ConversationStateIR> {
        self.states.get(conversation_id)
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
        if let Some(resolution) = resolve_local_conditional_reference(semantic_text) {
            return resolution;
        }
        let Some(state) = self.states.get(conversation_id) else {
            let mut ambiguous = reference_surfaces(semantic_text);
            if is_goal_ellipsis_surface(semantic_text) {
                ambiguous.push("ELLIPTICAL_GOAL".to_string());
            }
            if let Some(kind) = discourse_reference_kind(semantic_text) {
                ambiguous.push(format!("{kind:?}_REFERENCE"));
            }
            return ReferenceResolutionIR {
                original_semantic_text: semantic_text.to_string(),
                resolved_semantic_text: semantic_text.to_string(),
                resolved_reference_count: 0,
                used_referent_ids: Vec::new(),
                ambiguous_reference_surfaces: ambiguous,
                discourse_bindings: Vec::new(),
            };
        };
        let typed_discourse = resolve_typed_discourse_reference(state, semantic_text);
        let working_text = typed_discourse.resolved_text;
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
        let mut used = typed_discourse
            .binding
            .as_ref()
            .into_iter()
            .flat_map(|binding| binding.referent_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let mut ambiguous = typed_discourse.ambiguous_surfaces;
        let mut bindings = typed_discourse.binding.into_iter().collect::<Vec<_>>();
        let working_tokens = working_text.split_whitespace().collect::<Vec<_>>();
        let resolved = working_tokens
            .iter()
            .enumerate()
            .map(|(token_index, token)| {
                let (prefix, core, suffix) = token_parts(token);
                if english_that_is_complementizer(&working_tokens, token_index) {
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
                    });
                    return format!("{prefix}{replacement}{suffix}");
                }
                if !is_reference_surface(core) {
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
                });
                format!("{prefix}{replacement}{suffix}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        let discourse = resolve_goal_ellipsis(state, &resolved);
        if let Some(binding) = discourse.binding {
            bindings.push(binding);
        }
        ambiguous.extend(discourse.ambiguous_surfaces);
        ReferenceResolutionIR {
            original_semantic_text: semantic_text.to_string(),
            resolved_semantic_text: discourse.resolved_text,
            resolved_reference_count: resolved_count,
            used_referent_ids: used.into_iter().collect(),
            ambiguous_reference_surfaces: ambiguous,
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
            state.active_goals = grounded_goals.to_vec();
            state.active_goals.sort_by(|left, right| {
                left.goal_id
                    .cmp(&right.goal_id)
                    .then_with(|| left.subject.cmp(&right.subject))
            });
            state.active_goals.truncate(MAX_ACTIVE_GOALS);
            state.active_discourse_referents.retain(|referent| {
                !matches!(
                    referent.kind,
                    DiscourseReferentKindIR::Event | DiscourseReferentKindIR::Result
                )
            });
            for (index, goal) in grounded_goals.iter().enumerate() {
                state
                    .active_discourse_referents
                    .extend(event_and_result_referents(goal, request.turn_index, index));
            }
        }
        let mut current_propositions = proposition_referents.to_vec();
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
        state.completed_turns = request.turn_index;
        state.preferred_language = language.or(state.preferred_language);
        state.unresolved_reference_count = unresolved_reference_count;
        state.state_sha256 = state_hash(state)?;
        validate_conversation_state(state)?;
        Ok(state.clone())
    }
}

fn resolve_local_conditional_reference(text: &str) -> Option<ReferenceResolutionIR> {
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
        discourse_bindings: vec![DiscourseBindingIR {
            kind: DiscourseBindingKindIR::LocalAntecedentReference,
            source_surface: "it".to_string(),
            resolved_surface: subject,
            referent_ids: Vec::new(),
            inherited_goal_id: None,
            confidence_millis: 930,
        }],
    })
}

fn event_and_result_referents(
    goal: &ConversationGoalFrameIR,
    turn_index: u64,
    index: usize,
) -> [DynamicDiscourseReferentIR; 2] {
    let suffix = index + 1;
    [
        DynamicDiscourseReferentIR {
            referent_id: format!("DREF-E-{turn_index:06}-{suffix:02}"),
            kind: DiscourseReferentKindIR::Event,
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

fn empty_state(conversation_id: &str) -> ConversationStateIR {
    let mut state = ConversationStateIR {
        schema: CONVERSATION_STATE_SCHEMA.to_string(),
        conversation_id: conversation_id.to_string(),
        completed_turns: 0,
        active_subject: None,
        active_referents: Vec::new(),
        active_goals: Vec::new(),
        active_discourse_referents: Vec::new(),
        epistemic_ledger: EpistemicLedgerIR::default(),
        temporal_graph: TemporalGraphIR::default(),
        conditional_guard_store: ConditionalGuardStoreIR::default(),
        last_guard_evaluations: Vec::new(),
        preferred_language: None,
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
        &state.active_referents,
        &state.active_goals,
        &state.active_discourse_referents,
        &state.epistemic_ledger,
        &state.temporal_graph,
        &state.conditional_guard_store,
        &state.last_guard_evaluations,
        state.preferred_language,
        state.unresolved_reference_count,
    ))
    .map_err(|_| ConversationFrontendError::InvalidState)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
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
    if state.schema != CONVERSATION_STATE_SCHEMA
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
        || state.active_discourse_referents.len() > MAX_DISCOURSE_REFERENTS
        || unique_discourse_referents.len() != state.active_discourse_referents.len()
        || state.active_discourse_referents.iter().any(|referent| {
            referent.referent_id.trim().is_empty()
                || referent.semantic_summary.trim().is_empty()
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
        || state.last_guard_evaluations.len() > MAX_ACTIVE_GOALS * 4
        || state.last_guard_evaluations.iter().any(|evaluation| {
            evaluation.evaluation_turn > state.completed_turns
                || !evaluation.validate(&state.conditional_guard_store, &state.epistemic_ledger)
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

fn resolve_typed_discourse_reference(
    state: &ConversationStateIR,
    text: &str,
) -> TypedDiscourseResolution {
    let Some(kind) = discourse_reference_kind(text) else {
        return TypedDiscourseResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: Vec::new(),
        };
    };
    let mut eligible = state
        .active_discourse_referents
        .iter()
        .filter(|referent| {
            referent.kind == kind
                && state
                    .completed_turns
                    .saturating_sub(referent.last_referenced_turn)
                    <= MAX_REFERENCE_TURN_DISTANCE
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
    let latest_turn = eligible
        .iter()
        .map(|referent| referent.last_referenced_turn)
        .max();
    let latest = eligible
        .into_iter()
        .filter(|referent| Some(referent.last_referenced_turn) == latest_turn)
        .collect::<Vec<_>>();
    let Some(referent) = (latest.len() == 1).then(|| latest[0]) else {
        return TypedDiscourseResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: vec![format!("{kind:?}_REFERENCE")],
        };
    };
    let marker = discourse_reference_markers(kind)
        .iter()
        .find(|marker| text.to_lowercase().contains(**marker))
        .map(|marker| (*marker).to_string())
        .or_else(|| {
            (kind == DiscourseReferentKindIR::Proposition)
                .then(|| attributed_reference_marker(text, referent.attributed_source.as_deref()))
                .flatten()
        })
        .unwrap_or_default();
    let summary = referent
        .semantic_summary
        .trim_matches(|character| matches!(character, '‘' | '’' | '“' | '”' | '"' | '\''));
    let attribution_prefix = referent
        .attributed_source
        .as_deref()
        .zip(referent.attribution_attitude)
        .map(|(source, attitude)| {
            if text_is_english(text) {
                format!("{source}'s {attitude:?} attribution")
            } else {
                format!("{source}의 {attitude:?} 귀속")
            }
        });
    let replacement = if text_is_english(text) {
        match kind {
            DiscourseReferentKindIR::Event => format!("the action ‘{summary}’"),
            DiscourseReferentKindIR::Result => format!("the result of ‘{summary}’"),
            DiscourseReferentKindIR::Proposition => attribution_prefix.map_or_else(
                || format!("the attributed proposition ‘{summary}’"),
                |prefix| format!("the {prefix} ‘{summary}’"),
            ),
        }
    } else {
        match kind {
            DiscourseReferentKindIR::Event => format!("‘{summary}’라는 작업"),
            DiscourseReferentKindIR::Result => format!("‘{summary}’의 결과"),
            DiscourseReferentKindIR::Proposition => attribution_prefix.map_or_else(
                || format!("‘{summary}’라는 귀속 명제"),
                |prefix| format!("{prefix} ‘{summary}’라는 명제"),
            ),
        }
    };
    let resolved_text = replace_first_case_insensitive(text, &marker, &replacement);
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
        }),
        ambiguous_surfaces: Vec::new(),
    }
}

fn discourse_reference_kind(text: &str) -> Option<DiscourseReferentKindIR> {
    let normalized = text.to_lowercase();
    if proposition_reference_surface(&normalized) {
        return Some(DiscourseReferentKindIR::Proposition);
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
            .any(|marker| normalized.contains(marker))
    })
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
    .any(|marker| text.contains(marker))
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
            "그 결과",
            "그 출력",
            "그 산출물",
            "that result",
            "that output",
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

fn resolve_goal_ellipsis(state: &ConversationStateIR, text: &str) -> GoalEllipsisResolution {
    let Some((kind, replacement_subject)) = classify_goal_ellipsis(text) else {
        return GoalEllipsisResolution {
            resolved_text: text.to_string(),
            binding: None,
            ambiguous_surfaces: Vec::new(),
        };
    };
    let eligible_goals = state
        .active_goals
        .iter()
        .filter(|goal| {
            state
                .completed_turns
                .saturating_sub(goal.last_referenced_turn)
                <= MAX_GOAL_ELLIPSIS_TURN_DISTANCE
        })
        .collect::<Vec<_>>();
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
        (_, Some((_, concept_id))) => render_goal_for_subject(goal, concept_id, text),
        (_, None) => text.to_string(),
    };
    let referent_ids = replacement_subject
        .as_ref()
        .map(|(_, concept_id)| {
            state
                .active_referents
                .iter()
                .filter(|referent| &referent.canonical_concept == concept_id)
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
        }),
        ambiguous_surfaces: Vec::new(),
    }
}

fn classify_goal_ellipsis(text: &str) -> Option<(GoalEllipsisKind, Option<(String, String)>)> {
    let normalized = text
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_lowercase();
    if [
        "그대로 해",
        "그대로 해줘",
        "똑같이 해",
        "똑같이 해줘",
        "do the same",
        "do the same again",
        "same again",
    ]
    .contains(&normalized.as_str())
    {
        return Some((GoalEllipsisKind::Repeat, None));
    }
    if !contains_explicit_action_surface(&normalized) {
        if normalized.contains("말고")
            || (normalized.contains("instead")
                && (normalized.contains("not ") || normalized.contains("rather ")))
        {
            if let Some(subject) = known_subject_in_fragment(&normalized) {
                return Some((GoalEllipsisKind::CorrectedArgument, Some(subject)));
            }
        }
        let parallel_marker = normalized.contains("같은 방식")
            || normalized.contains("똑같이")
            || normalized.ends_with('도')
            || normalized.contains("same for ")
            || normalized.contains("same operation for ")
            || normalized.ends_with(" too");
        if parallel_marker {
            if let Some(subject) = known_subject_in_fragment(&normalized) {
                return Some((GoalEllipsisKind::ParallelArgument, Some(subject)));
            }
        }
    }
    None
}

fn is_goal_ellipsis_surface(text: &str) -> bool {
    classify_goal_ellipsis(text).is_some()
}

fn contains_explicit_action_surface(text: &str) -> bool {
    [
        "열어",
        "읽어",
        "변환",
        "저장",
        "삭제",
        "배포",
        "실행",
        "확인",
        "조사",
        "분석",
        "고쳐",
        "수정",
        "수리",
        "복구",
        "만들",
        "작성",
        "생성",
        "설명",
        "해설",
        "기록",
        "전달",
        "말해",
        "학습",
        "배워",
        "익혀",
        "open ",
        "read ",
        "transform ",
        "convert ",
        "save ",
        "delete ",
        "deploy ",
        "run ",
        "check ",
        "inspect ",
        "analyze ",
        "fix ",
        "repair ",
        "restore ",
        "create ",
        "write ",
        "explain ",
        "record ",
        "report ",
        "tell ",
        "learn ",
    ]
    .iter()
    .any(|surface| text.contains(surface))
}

fn known_subject_in_fragment(text: &str) -> Option<(String, String)> {
    let aliases = [
        ("source code", "C_OBJECT_SOURCE_CODE"),
        ("repository", "C_OBJECT_REPOSITORY"),
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

fn repeat_goal_in_current_language(goal: &ConversationGoalFrameIR, current_text: &str) -> String {
    let current_is_english = text_is_english(current_text);
    let source_is_english = text_is_english(&goal.source_semantic_text);
    if current_is_english == source_is_english {
        return goal.source_semantic_text.clone();
    }
    known_subject_in_fragment(&goal.subject)
        .map(|(_, concept)| render_goal_for_subject(goal, &concept, current_text))
        .unwrap_or_else(|| goal.source_semantic_text.clone())
}

fn render_goal_for_subject(
    goal: &ConversationGoalFrameIR,
    concept_id: &str,
    current_text: &str,
) -> String {
    let english = text_is_english(current_text);
    let subject = concept_surface(concept_id, english).unwrap_or(goal.subject.as_str());
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
            ("조사", "inspect"),
            ("분석", "analyze"),
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
        ("C_OBJECT_FILE", true) => Some("file"),
        ("C_OBJECT_FOLDER", true) => Some("folder"),
        ("C_OBJECT_SOURCE_CODE", true) => Some("code"),
        ("C_OBJECT_DOCUMENT", true) => Some("document"),
        ("C_OBJECT_REPORT", true) => Some("report"),
        ("C_OBJECT_DEFECT", true) => Some("error"),
        ("C_OBJECT_PROJECT", true) => Some("project"),
        ("C_OBJECT_REPOSITORY", true) => Some("repository"),
        ("C_OBJECT_PLAN", true) => Some("plan"),
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
        .filter(|(index, _)| !english_that_is_complementizer(&tokens, *index))
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
                    | '"'
                    | '\''
            )
        })
        .collect::<String>();
    if output.is_empty() {
        let openings = punctuation
            .chars()
            .filter(|character| matches!(character, '‘' | '“' | '「' | '『' | '"' | '\''))
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
                    guard_conditionals: None,
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
}
