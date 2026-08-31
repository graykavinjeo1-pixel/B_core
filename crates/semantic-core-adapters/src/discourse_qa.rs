//! Evidence-bounded question answering over conversation discourse state.
//!
//! This module does not answer from surface plausibility or general knowledge.
//! It compiles a bounded family of dialogue questions into typed queries and
//! answers only from the attribution, modality, and revision records already
//! present in `ConversationStateIR`.  A report that somebody knows, believes,
//! or observed a proposition remains a report; it is never promoted to truth.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::attribution::{AttributionAttitudeIR, EpistemicStatusIR};
use crate::conversation::ConversationStateIR;
use crate::epistemic::{BeliefRecordIR, BeliefRecordStatusIR};
use crate::language_knowledge::LanguageCodeIR;
use crate::modality::ModalWorldIR;

pub const DISCOURSE_QUERY_SCHEMA: &str = "B_CORE_DISCOURSE_QUERY_IR_1";
pub const DISCOURSE_ANSWER_SCHEMA: &str = "B_CORE_DISCOURSE_ANSWER_IR_1";
const MAX_ANSWER_EVIDENCE: usize = 16;
const MAX_ANSWER_CLAIMS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscourseQueryKindIR {
    SourceContent,
    PropositionSources,
    ActualityStatus,
    ModalStatus,
    ConflictStatus,
    PresuppositionCheck,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscourseAnswerIR {
    pub schema: String,
    pub query: DiscourseQueryIR,
    pub disposition: DiscourseAnswerDispositionIR,
    pub evidence: Vec<DiscourseAnswerEvidenceIR>,
    pub claims: Vec<DiscourseAnswerClaimIR>,
    pub language: LanguageCodeIR,
    pub realized_text: String,
    pub dialogue_truth_established: bool,
    pub external_execution_authorized: bool,
    pub unsupported_claims: usize,
}

impl DiscourseAnswerIR {
    pub fn validate(&self) -> bool {
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

#[derive(Debug, Clone, Copy, Default)]
pub struct DiscourseQaEngine;

impl DiscourseQaEngine {
    pub fn parse(
        &self,
        text: &str,
        state: Option<&ConversationStateIR>,
    ) -> Option<DiscourseQueryIR> {
        let normalized = normalize_space(&text.to_lowercase());
        if normalized.is_empty() || !looks_like_question(&normalized) {
            return None;
        }
        let known_sources = state.map_or_else(Vec::new, known_sources);
        let mut source = source_in_query(&normalized, &known_sources)
            .or_else(|| extract_unknown_source(&normalized));
        let mut requested_attitudes = requested_attitudes(&normalized);
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

        let kind = if presuppositional_question(&normalized) {
            DiscourseQueryKindIR::PresuppositionCheck
        } else if conflict_question(&normalized) {
            DiscourseQueryKindIR::ConflictStatus
        } else if modal_status_question(&normalized) {
            DiscourseQueryKindIR::ModalStatus
        } else if actuality_question(&normalized) {
            DiscourseQueryKindIR::ActualityStatus
        } else if proposition_source_question(&normalized) {
            DiscourseQueryKindIR::PropositionSources
        } else if source_content_question(&normalized, source.as_deref()) {
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
        let mut topic_terms = query_topic_terms(&normalized, source.as_deref());
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
        let query = self.parse(text, state)?;
        let mut matching = state.map_or_else(Vec::new, |state| matching_records(&query, state));
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
        let (disposition, mut claims) = answer_claims(&query, &evidence);
        claims.truncate(MAX_ANSWER_CLAIMS);
        let realized_text = realize_answer(language, &query, disposition, &evidence);
        let answer = DiscourseAnswerIR {
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
        attitudes.extend([
            AttributionAttitudeIR::Say,
            AttributionAttitudeIR::Report,
            AttributionAttitudeIR::Claim,
            AttributionAttitudeIR::Correct,
        ]);
    }
    attitudes.sort();
    attitudes.dedup();
    attitudes
}

fn query_topic_terms(text: &str, source: Option<&str>) -> Vec<String> {
    let source_terms = source.map_or_else(BTreeSet::new, normalized_terms);
    normalized_terms(text)
        .into_iter()
        .filter(|term| !source_terms.contains(term))
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

fn normalize_actor(actor: &str) -> String {
    normalize_space(&actor.to_lowercase())
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_string()
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_question(text: &str) -> bool {
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
    source.is_some()
        && contains_any(text, &["what", "뭐", "무엇", "어떤", "내용"])
        && contains_any(
            text,
            &[
                "say", "said", "claim", "report", "believe", "think", "know", "want", "expect",
                "말", "주장", "보고", "믿", "생각", "알", "원", "예상", "기대",
            ],
        )
}

fn proposition_source_question(text: &str) -> bool {
    contains_any(text, &["who ", "who's ", "누가", "누구"])
        && contains_any(
            text,
            &[
                "say", "said", "claim", "report", "believe", "think", "know", "말", "주장", "보고",
                "믿", "생각", "알",
            ],
        )
}

fn actuality_question(text: &str) -> bool {
    contains_any(
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
    )
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
        ],
    )
}

fn presuppositional_question(text: &str) -> bool {
    (contains_any(text, &["why ", "when ", "how ", "왜 ", "언제 ", "어떻게 "])
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

    fn state_with(
        records: &[(&str, &str, ModalWorldIR, AttributionAttitudeIR)],
    ) -> ConversationStateIR {
        let mut state = ConversationStateIR {
            schema: crate::conversation::CONVERSATION_STATE_SCHEMA.to_string(),
            conversation_id: "QA-TEST".to_string(),
            completed_turns: 0,
            active_subject: None,
            active_referents: Vec::new(),
            active_goals: Vec::new(),
            active_discourse_referents: Vec::new(),
            epistemic_ledger: EpistemicLedgerIR::default(),
            temporal_graph: crate::temporal::TemporalGraphIR::default(),
            conditional_guard_store: Default::default(),
            last_guard_evaluations: Vec::new(),
            preferred_language: Some(LanguageCodeIR::English),
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
}
