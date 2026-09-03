//! Bounded inference from utterance form to the response goal the user is
//! pragmatically requesting.
//!
//! This graph is adapter evidence, not semantic truth.  It can ask the
//! planner for an assessment, explanation, recommendation, or diagnostic,
//! but it never authorizes an external action and never mutates a concept.

use dockable_semantic_core::PlanIntentIR;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const UTTERANCE_INTENT_GRAPH_SCHEMA: &str = "B_CORE_UTTERANCE_INTENT_GRAPH_IR_1";
pub const MAX_UTTERANCE_INTENT_SIGNALS: usize = 32;
pub const MAX_UTTERANCE_INTENT_CANDIDATES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UtteranceSurfaceFormIR {
    Declarative,
    Interrogative,
    Imperative,
    Fragment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UtteranceSignalKindIR {
    ProblemState,
    ReadinessOrSafety,
    EvidenceDemand,
    AlternativeComparison,
    ResponseGoalCorrection,
    ExplanationDemand,
    SummaryDemand,
    ConditionalPremise,
    ContinuationDecision,
    BenefitCriterion,
    PreservationConstraint,
    InterrogativeForm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommunicativeIntentIR {
    ProblemDisclosure,
    AssessmentRequest,
    EvidenceRequest,
    RecommendationRequest,
    ResponseGoalCorrection,
    ExplanationRequest,
    SummaryRequest,
    ConditionalDecisionRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpectedResponseKindIR {
    DiagnosisOrNextStep,
    Assessment,
    Evidence,
    Recommendation,
    Explanation,
    Summary,
    VerifyThenDecide,
    Clarification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentSignalIR {
    pub signal_id: String,
    pub kind: UtteranceSignalKindIR,
    pub evidence_surface: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub confidence_millis: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentCandidateIR {
    pub candidate_id: String,
    pub communicative_intent: CommunicativeIntentIR,
    pub expected_response: ExpectedResponseKindIR,
    pub target: String,
    pub constraints: Vec<String>,
    pub evidence_signal_ids: Vec<String>,
    pub score_millis: u16,
    pub requires_prior_context: bool,
    pub prior_context_bound: bool,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
}

impl UtteranceIntentCandidateIR {
    pub fn plan_intent(&self) -> Option<PlanIntentIR> {
        match self.expected_response {
            ExpectedResponseKindIR::DiagnosisOrNextStep
            | ExpectedResponseKindIR::Assessment
            | ExpectedResponseKindIR::VerifyThenDecide => Some(PlanIntentIR::Investigate),
            ExpectedResponseKindIR::Evidence
            | ExpectedResponseKindIR::Explanation
            | ExpectedResponseKindIR::Summary => Some(PlanIntentIR::Explain),
            ExpectedResponseKindIR::Recommendation => Some(PlanIntentIR::Plan),
            ExpectedResponseKindIR::Clarification => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtteranceIntentGraphIR {
    pub schema: String,
    pub source_text_sha256: String,
    pub context_sha256: String,
    pub surface_form: UtteranceSurfaceFormIR,
    pub signals: Vec<UtteranceIntentSignalIR>,
    pub candidates: Vec<UtteranceIntentCandidateIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    pub unresolved_ambiguities: Vec<String>,
    pub semantic_authority: bool,
    pub external_execution_authorized: bool,
    pub graph_sha256: String,
}

impl Default for UtteranceIntentGraphIR {
    fn default() -> Self {
        analyze_utterance_intent("", None, &[])
    }
}

impl UtteranceIntentGraphIR {
    pub fn selected(&self) -> Option<&UtteranceIntentCandidateIR> {
        self.selected_candidate_id.as_ref().and_then(|selected| {
            self.candidates
                .iter()
                .find(|candidate| &candidate.candidate_id == selected)
        })
    }

    pub fn supporting(&self) -> impl Iterator<Item = &UtteranceIntentCandidateIR> {
        let selected = self.selected_candidate_id.as_deref();
        self.candidates
            .iter()
            .filter(move |candidate| Some(candidate.candidate_id.as_str()) != selected)
    }

    /// Returns the primary intent first and every compatible supporting
    /// contribution after it. All entries are evidence-only at this layer.
    pub fn active(&self) -> impl Iterator<Item = &UtteranceIntentCandidateIR> {
        self.selected().into_iter().chain(self.supporting())
    }

    pub fn requires_clarification(&self) -> bool {
        self.selected().is_some_and(|candidate| {
            candidate.expected_response == ExpectedResponseKindIR::Clarification
        }) || !self.unresolved_ambiguities.is_empty()
    }

    pub fn validate(&self) -> bool {
        if self.schema != UTTERANCE_INTENT_GRAPH_SCHEMA
            || self.source_text_sha256.len() != 64
            || self.context_sha256.len() != 64
            || self.graph_sha256.len() != 64
            || self.signals.len() > MAX_UTTERANCE_INTENT_SIGNALS
            || self.candidates.len() > MAX_UTTERANCE_INTENT_CANDIDATES
            || self.semantic_authority
            || self.external_execution_authorized
            || self.signals.iter().any(|signal| {
                signal.signal_id.is_empty()
                    || signal.byte_start > signal.byte_end
                    || signal.confidence_millis > 1_000
            })
            || self.candidates.iter().any(|candidate| {
                candidate.candidate_id.is_empty()
                    || candidate.target.trim().is_empty()
                    || candidate.score_millis > 1_000
                    || candidate.semantic_authority
                    || candidate.external_execution_authorized
            })
        {
            return false;
        }
        let signal_ids = self
            .signals
            .iter()
            .map(|signal| signal.signal_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let candidate_ids = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if signal_ids.len() != self.signals.len()
            || candidate_ids.len() != self.candidates.len()
            || self.candidates.iter().any(|candidate| {
                candidate
                    .evidence_signal_ids
                    .iter()
                    .any(|signal_id| !signal_ids.contains(signal_id.as_str()))
            })
            || self
                .selected_candidate_id
                .as_ref()
                .is_some_and(|selected| !candidate_ids.contains(selected.as_str()))
            || self.selected_candidate_id.as_ref().is_some_and(|selected| {
                self.candidates
                    .first()
                    .map(|candidate| &candidate.candidate_id)
                    != Some(selected)
            })
            || self.candidates.is_empty() != self.selected_candidate_id.is_none()
        {
            return false;
        }
        let mut canonical = self.clone();
        canonical.graph_sha256.clear();
        self.graph_sha256 == hash_json(&canonical)
    }

    pub fn validate_against(
        &self,
        text: &str,
        active_subject: Option<&str>,
        active_predicates: &[String],
    ) -> bool {
        self.validate_source(text)
            && self.context_sha256 == context_hash(active_subject, active_predicates)
    }

    pub fn validate_source(&self, text: &str) -> bool {
        self.validate() && self.source_text_sha256 == hash_bytes(text.trim().as_bytes())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UtteranceIntentAnalyzer;

impl UtteranceIntentAnalyzer {
    pub fn analyze(
        &self,
        text: &str,
        active_subject: Option<&str>,
        active_predicates: &[String],
    ) -> UtteranceIntentGraphIR {
        analyze_utterance_intent(text, active_subject, active_predicates)
    }
}

fn analyze_utterance_intent(
    text: &str,
    active_subject: Option<&str>,
    active_predicates: &[String],
) -> UtteranceIntentGraphIR {
    let trimmed = text.trim();
    let normalized = trimmed.to_lowercase();
    let surface_form = surface_form(&normalized);
    let mut signals = collect_signals(&normalized);
    signals.truncate(MAX_UTTERANCE_INTENT_SIGNALS);
    let has_context = active_subject.is_some_and(|value| !value.trim().is_empty())
        || !active_predicates.is_empty();
    let mut candidates = infer_candidates(&normalized, active_subject, has_context, &signals);
    candidates.sort_by(|left, right| {
        right
            .score_millis
            .cmp(&left.score_millis)
            .then_with(|| {
                intent_precedence(right.communicative_intent)
                    .cmp(&intent_precedence(left.communicative_intent))
            })
            .then_with(|| left.target.cmp(&right.target))
    });
    candidates.dedup_by(|left, right| {
        left.communicative_intent == right.communicative_intent
            && left.expected_response == right.expected_response
            && left.target == right.target
    });
    candidates.truncate(MAX_UTTERANCE_INTENT_CANDIDATES);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_id = format!("UTTERANCE-INTENT-CANDIDATE-{:02}", index + 1);
    }
    let mut unresolved_ambiguities = Vec::new();
    if candidates
        .iter()
        .any(|candidate| candidate.expected_response == ExpectedResponseKindIR::Clarification)
    {
        unresolved_ambiguities.push("PRIOR_DISCOURSE_CONTEXT".to_string());
    }
    let selected_candidate_id = candidates
        .first()
        .map(|candidate| candidate.candidate_id.clone());
    let mut graph = UtteranceIntentGraphIR {
        schema: UTTERANCE_INTENT_GRAPH_SCHEMA.to_string(),
        source_text_sha256: hash_bytes(trimmed.as_bytes()),
        context_sha256: context_hash(active_subject, active_predicates),
        surface_form,
        signals,
        candidates,
        selected_candidate_id,
        unresolved_ambiguities,
        semantic_authority: false,
        external_execution_authorized: false,
        graph_sha256: String::new(),
    };
    graph.graph_sha256 = hash_json(&graph);
    graph
}

fn infer_candidates(
    text: &str,
    active_subject: Option<&str>,
    has_context: bool,
    signals: &[UtteranceIntentSignalIR],
) -> Vec<UtteranceIntentCandidateIR> {
    let has = |kind| signals.iter().any(|signal| signal.kind == kind);
    let signal_ids = |kinds: &[UtteranceSignalKindIR]| {
        signals
            .iter()
            .filter(|signal| kinds.contains(&signal.kind))
            .map(|signal| signal.signal_id.clone())
            .collect::<Vec<_>>()
    };
    let make = |communicative_intent,
                expected_response,
                target: String,
                constraints: Vec<String>,
                evidence_signal_ids,
                score_millis,
                requires_prior_context,
                prior_context_bound| UtteranceIntentCandidateIR {
        candidate_id: String::new(),
        communicative_intent,
        expected_response,
        target,
        constraints,
        evidence_signal_ids,
        score_millis,
        requires_prior_context,
        prior_context_bound,
        semantic_authority: false,
        external_execution_authorized: false,
    };

    let mut candidates = Vec::new();

    if has(UtteranceSignalKindIR::ResponseGoalCorrection)
        && has(UtteranceSignalKindIR::ExplanationDemand)
    {
        let target = active_subject
            .filter(|subject| {
                !subject.trim().is_empty() && explanation_target_is_context_bound(text)
            })
            .map(str::to_string)
            .unwrap_or_else(|| explanation_target(text));
        let self_contained = has_explicit_explanation_target(text);
        candidates.push(make(
            CommunicativeIntentIR::ResponseGoalCorrection,
            if has_context || self_contained {
                ExpectedResponseKindIR::Explanation
            } else {
                ExpectedResponseKindIR::Clarification
            },
            target,
            vec![
                "replace the prior response goal; do not retain the superseded action request"
                    .to_string(),
            ],
            signal_ids(&[
                UtteranceSignalKindIR::ResponseGoalCorrection,
                UtteranceSignalKindIR::ExplanationDemand,
            ]),
            980,
            !self_contained,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ConditionalPremise)
        && has(UtteranceSignalKindIR::ContinuationDecision)
        && has(UtteranceSignalKindIR::BenefitCriterion)
    {
        candidates.push(make(
            CommunicativeIntentIR::ConditionalDecisionRequest,
            ExpectedResponseKindIR::VerifyThenDecide,
            conditional_target(text),
            vec![format!(
                "verify the stated benefit criterion before continuing: {}",
                benefit_constraint(text)
            )],
            signal_ids(&[
                UtteranceSignalKindIR::ConditionalPremise,
                UtteranceSignalKindIR::ContinuationDecision,
                UtteranceSignalKindIR::BenefitCriterion,
            ]),
            970,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::SummaryDemand) {
        let self_contained =
            contains_any(text, &["답변", "설명", "answer", "explanation", "response"]);
        let target = active_subject
            .filter(|subject| !subject.trim().is_empty())
            .map(str::to_string)
            .or_else(|| self_contained.then(|| summary_response_target(text)))
            .unwrap_or_else(|| "prior discourse result".to_string());
        candidates.push(make(
            CommunicativeIntentIR::SummaryRequest,
            if has_context || self_contained {
                ExpectedResponseKindIR::Summary
            } else {
                ExpectedResponseKindIR::Clarification
            },
            target,
            if has_context || self_contained {
                vec!["summarize only claims supported by the prior discourse".to_string()]
            } else {
                vec!["context is required to identify the prior discourse result".to_string()]
            },
            signal_ids(&[UtteranceSignalKindIR::SummaryDemand]),
            if has_context || self_contained {
                950
            } else {
                700
            },
            !self_contained,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::EvidenceDemand) {
        candidates.push(make(
            CommunicativeIntentIR::EvidenceRequest,
            ExpectedResponseKindIR::Evidence,
            evidence_target(text),
            vec!["cite or expose supporting evidence; do not invent support".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::EvidenceDemand,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            960,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ReadinessOrSafety)
        && has(UtteranceSignalKindIR::InterrogativeForm)
    {
        candidates.push(make(
            CommunicativeIntentIR::AssessmentRequest,
            ExpectedResponseKindIR::Assessment,
            assessment_target(text),
            vec!["assess readiness or safety; do not execute deployment".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::ReadinessOrSafety,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            950,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::AlternativeComparison) {
        let mut constraints = vec!["prefer the safer or better supported alternative".to_string()];
        if has(UtteranceSignalKindIR::PreservationConstraint) {
            constraints.push("preserve the original data and avoid destructive action".to_string());
        }
        candidates.push(make(
            CommunicativeIntentIR::RecommendationRequest,
            ExpectedResponseKindIR::Recommendation,
            recommendation_target(text),
            constraints,
            signal_ids(&[
                UtteranceSignalKindIR::AlternativeComparison,
                UtteranceSignalKindIR::PreservationConstraint,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            940,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ExplanationDemand)
        && has(UtteranceSignalKindIR::InterrogativeForm)
        && !has(UtteranceSignalKindIR::ResponseGoalCorrection)
        && !contains_any(text, &["why don't we", "why dont we", "why not"])
    {
        candidates.push(make(
            CommunicativeIntentIR::ExplanationRequest,
            ExpectedResponseKindIR::Explanation,
            explanation_target(text),
            vec!["explain only from available causal evidence".to_string()],
            signal_ids(&[
                UtteranceSignalKindIR::ExplanationDemand,
                UtteranceSignalKindIR::InterrogativeForm,
            ]),
            920,
            false,
            has_context,
        ));
    }

    if has(UtteranceSignalKindIR::ProblemState)
        && !has(UtteranceSignalKindIR::InterrogativeForm)
        && !has(UtteranceSignalKindIR::ResponseGoalCorrection)
    {
        candidates.push(make(
            CommunicativeIntentIR::ProblemDisclosure,
            ExpectedResponseKindIR::DiagnosisOrNextStep,
            problem_target(text),
            vec![
                "treat the disclosed failure as a request for bounded diagnosis or a next step, not mutation authority"
                    .to_string(),
            ],
            signal_ids(&[UtteranceSignalKindIR::ProblemState]),
            900,
            false,
            has_context,
        ));
    }
    candidates
}

fn intent_precedence(intent: CommunicativeIntentIR) -> u8 {
    match intent {
        CommunicativeIntentIR::ResponseGoalCorrection => 8,
        CommunicativeIntentIR::ConditionalDecisionRequest => 7,
        CommunicativeIntentIR::EvidenceRequest => 6,
        CommunicativeIntentIR::SummaryRequest => 5,
        CommunicativeIntentIR::AssessmentRequest => 4,
        CommunicativeIntentIR::RecommendationRequest => 3,
        CommunicativeIntentIR::ExplanationRequest => 2,
        CommunicativeIntentIR::ProblemDisclosure => 1,
    }
}

fn collect_signals(text: &str) -> Vec<UtteranceIntentSignalIR> {
    let mut signals = Vec::new();
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ProblemState,
        &[
            "멈추",
            "멎",
            "깨지",
            "실패",
            "안 돼",
            "안돼",
            "오류",
            "stopping",
            "stops",
            "dies",
            "failed",
            "failure",
            "broken",
            "keeps crashing",
        ],
        900,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ReadinessOrSafety,
        &[
            "배포해도",
            "올려도",
            "되는 상태",
            "괜찮은 거야",
            "ready to",
            "safe to",
            "go live",
            "ship",
            "release safely",
        ],
        940,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::EvidenceDemand,
        &[
            "근거",
            "증거",
            "뒷받침",
            "evidence",
            "supports that",
            "backs up",
            "basis for",
        ],
        960,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::AlternativeComparison,
        &[
            "중 뭐",
            "뭐가 더",
            "어떤 ",
            "제일 나아",
            "which is",
            "which recovery",
            "which approach",
            "best",
            "safer",
            "better option",
        ],
        930,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ResponseGoalCorrection,
        &[
            "아니,",
            "게 아니라",
            "말이 아니야",
            "not asking",
            "wasn't asking",
            "was not asking",
            "i meant",
        ],
        970,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ExplanationDemand,
        &[
            "왜 ",
            "왜 실패",
            "원인",
            "설명",
            "why ",
            "explain",
            "cause",
            "caused",
        ],
        930,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::SummaryDemand,
        &[
            "그래서 결론",
            "핵심만",
            "요지는",
            "bottom line",
            "takeaway",
            "in short",
        ],
        950,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ConditionalPremise,
        &["다면", "라면", "하면", "if ", "provided that"],
        900,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::ContinuationDecision,
        &[
            "계속할",
            "진행할",
            "할 만",
            "할만",
            "worth continuing",
            "continue",
            "proceed",
        ],
        920,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::BenefitCriterion,
        &[
            "커버리지",
            "이득",
            "효과",
            "benefit",
            "coverage",
            "payoff",
            "worth",
        ],
        920,
    );
    add_signal(
        &mut signals,
        text,
        UtteranceSignalKindIR::PreservationConstraint,
        &[
            "잃으면 안",
            "원본 데이터",
            "건드리면 안",
            "cannot lose",
            "must not lose",
            "preserve the original",
            "do not touch the original",
        ],
        970,
    );
    if text.contains('?') {
        signals.push(UtteranceIntentSignalIR {
            signal_id: format!("UTTERANCE-SIGNAL-{:02}", signals.len() + 1),
            kind: UtteranceSignalKindIR::InterrogativeForm,
            evidence_surface: "?".to_string(),
            byte_start: text.rfind('?').unwrap_or_default(),
            byte_end: text.rfind('?').unwrap_or_default() + 1,
            confidence_millis: 1_000,
        });
    }
    signals
}

fn add_signal(
    signals: &mut Vec<UtteranceIntentSignalIR>,
    text: &str,
    kind: UtteranceSignalKindIR,
    markers: &[&str],
    confidence_millis: u16,
) {
    let Some((start, marker)) = markers
        .iter()
        .filter_map(|marker| text.find(marker).map(|start| (start, *marker)))
        .min_by_key(|(start, _)| *start)
    else {
        return;
    };
    signals.push(UtteranceIntentSignalIR {
        signal_id: format!("UTTERANCE-SIGNAL-{:02}", signals.len() + 1),
        kind,
        evidence_surface: marker.to_string(),
        byte_start: start,
        byte_end: start + marker.len(),
        confidence_millis,
    });
}

fn surface_form(text: &str) -> UtteranceSurfaceFormIR {
    if text.contains('?') {
        if text.split_whitespace().count() <= 5 {
            UtteranceSurfaceFormIR::Fragment
        } else {
            UtteranceSurfaceFormIR::Interrogative
        }
    } else if contains_any(
        text,
        &["해줘", "해 줘", "해달라", "please ", "explain ", "inspect "],
    ) {
        UtteranceSurfaceFormIR::Imperative
    } else if text.split_whitespace().count() <= 3 {
        UtteranceSurfaceFormIR::Fragment
    } else {
        UtteranceSurfaceFormIR::Declarative
    }
}

fn problem_target(text: &str) -> String {
    if let Some((prefix, _)) = split_first(text, &["가 계속", "이 계속", "가 또", "이 또"])
    {
        return clean_target(prefix);
    }
    if let Some((prefix, _)) = split_first(
        text,
        &[
            " keeps ",
            " is stopping",
            " stops",
            " dies",
            " failed",
            " keeps crashing",
        ],
    ) {
        return clean_target(prefix);
    }
    clean_target(text)
}

fn assessment_target(text: &str) -> String {
    let without_frame = text
        .strip_prefix("do you think the ")
        .or_else(|| text.strip_prefix("do you think "))
        .unwrap_or(text);
    for marker in [" is actually ready", " is ready", " safe to", " ready to"] {
        if let Some(position) = without_frame.find(marker) {
            let target = clean_target(&without_frame[..position]);
            if !target.is_empty() {
                return target;
            }
        }
    }
    for marker in ["빌드", "운영", "build", "release"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    clean_target(text)
}

fn evidence_target(text: &str) -> String {
    for marker in ["결론", "판단", "recommendation", "conclusion", "claim"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "stated proposition".to_string()
}

fn recommendation_target(text: &str) -> String {
    for marker in ["복구", "recovery"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    if text.starts_with("which is") || text.starts_with("which option") {
        if let Some((_, tail)) = text.split_once(',') {
            return clean_target(tail);
        }
    }
    if let Some((prefix, _)) = split_first(text, &[" 중 ", " 중", " which", " is safer"]) {
        return clean_target(prefix);
    }
    clean_target(text)
}

fn explanation_target(text: &str) -> String {
    for marker in ["실패", "failed", "failure", "원인", "cause"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    clean_target(text)
}

fn explanation_target_is_context_bound(text: &str) -> bool {
    contains_any(
        text,
        &[
            "why it failed",
            "why that failed",
            "why this failed",
            "why they failed",
            "cause of it",
            "왜 실패했는지",
            "왜 실패한지",
            "왜 실패하는지",
            "왜 실패하는지만",
            "왜 안 됐는지",
            "그게 왜 실패",
            "그것이 왜 실패",
        ],
    )
}

fn summary_response_target(text: &str) -> String {
    for marker in ["답변", "설명", "answer", "explanation", "response"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "response".to_string()
}

fn has_explicit_explanation_target(text: &str) -> bool {
    contains_any(
        text,
        &[
            "실패 원인",
            "원인만",
            "왜 실패",
            "failure",
            "cause of",
            "why it failed",
            "why it fails",
        ],
    )
}

fn conditional_target(text: &str) -> String {
    for marker in ["통합", "integration"] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    split_first(text, &["다면", "라면", "하면", " if "])
        .map(|(prefix, _)| clean_target(prefix))
        .unwrap_or_else(|| clean_target(text))
}

fn benefit_constraint(text: &str) -> String {
    for marker in [
        "커버리지를 넓힌",
        "커버리지",
        "expands coverage",
        "coverage",
        "benefit",
    ] {
        if text.contains(marker) {
            return marker.to_string();
        }
    }
    "stated benefit".to_string()
}

fn clean_target(value: &str) -> String {
    let cleaned = value
        .trim()
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .trim_start_matches("the ")
        .trim_start_matches("this ")
        .trim_start_matches("이 ")
        .trim();
    if cleaned.is_empty() {
        "utterance subject".to_string()
    } else {
        cleaned.to_string()
    }
}

fn split_first<'a>(text: &'a str, markers: &[&str]) -> Option<(&'a str, &'a str)> {
    markers
        .iter()
        .filter_map(|marker| text.find(marker).map(|start| (start, *marker)))
        .min_by_key(|(start, _)| *start)
        .map(|(start, marker)| (&text[..start], &text[start..start + marker.len()]))
}

fn context_hash(active_subject: Option<&str>, active_predicates: &[String]) -> String {
    let mut predicates = active_predicates.to_vec();
    predicates.sort();
    hash_json(&(
        active_subject.unwrap_or_default().trim().to_lowercase(),
        predicates,
    ))
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn hash_json<T: Serialize>(value: &T) -> String {
    hash_bytes(&serde_json::to_vec(value).expect("utterance intent serialization"))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(text: &str) -> UtteranceIntentCandidateIR {
        UtteranceIntentAnalyzer
            .analyze(text, None, &[])
            .selected()
            .expect("selected utterance intent")
            .clone()
    }

    #[test]
    fn declarative_problem_disclosure_requests_diagnosis_without_execution_authority() {
        let candidate = selected("The upload keeps stopping halfway.");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ProblemDisclosure
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::DiagnosisOrNextStep
        );
        assert!(!candidate.external_execution_authorized);
    }

    #[test]
    fn deployment_readiness_question_is_assessment_not_deployment() {
        let candidate = selected("Is this build actually ready to deploy?");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::AssessmentRequest
        );
        assert_eq!(candidate.plan_intent(), Some(PlanIntentIR::Investigate));

        let service = selected(
            "Do you think the Quartz service is actually ready to deploy, or should we check it first?",
        );
        assert_eq!(
            service.communicative_intent,
            CommunicativeIntentIR::AssessmentRequest
        );
        assert_eq!(service.target, "quartz service");
        assert!(!service.external_execution_authorized);
    }

    #[test]
    fn response_goal_correction_replaces_prior_action_with_explanation() {
        let graph = UtteranceIntentAnalyzer.analyze(
            "No, I am not asking you to inspect it; explain why it failed.",
            Some("log"),
            &["INVESTIGATE".to_string()],
        );
        let candidate = graph.selected().expect("correction");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ResponseGoalCorrection
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::Explanation
        );
        assert!(graph.validate_against(
            "No, I am not asking you to inspect it; explain why it failed.",
            Some("log"),
            &["INVESTIGATE".to_string()]
        ));
    }

    #[test]
    fn self_contained_response_goal_correction_does_not_require_prior_state() {
        let graph = UtteranceIntentAnalyzer.analyze(
            "I was not asking for another run; just explain the cause of the failure.",
            None,
            &[],
        );
        let candidate = graph.selected().expect("self-contained correction");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::ResponseGoalCorrection
        );
        assert_eq!(
            candidate.expected_response,
            ExpectedResponseKindIR::Explanation
        );
        assert!(!candidate.requires_prior_context);
        assert!(!candidate.prior_context_bound);
        assert!(graph.validate_against(
            "I was not asking for another run; just explain the cause of the failure.",
            None,
            &[]
        ));
    }

    #[test]
    fn advisory_why_dont_we_is_not_reclassified_as_explanation() {
        let graph =
            UtteranceIntentAnalyzer.analyze("Why don't we review the policy first?", None, &[]);
        assert!(graph.selected().is_none());
        assert!(graph.validate_against("Why don't we review the policy first?", None, &[]));
    }

    #[test]
    fn explicit_response_artifact_makes_summary_goal_self_contained() {
        let graph =
            UtteranceIntentAnalyzer.analyze("답변이 너무 길어. 핵심만 다시 설명해", None, &[]);
        let candidate = graph.selected().expect("summary response goal");
        assert_eq!(
            candidate.communicative_intent,
            CommunicativeIntentIR::SummaryRequest
        );
        assert_eq!(candidate.expected_response, ExpectedResponseKindIR::Summary);
        assert!(!candidate.requires_prior_context);
    }

    #[test]
    fn contextless_summary_fails_closed() {
        let graph = UtteranceIntentAnalyzer.analyze("And the takeaway?", None, &[]);
        assert!(graph.requires_clarification());
        assert_eq!(
            graph
                .selected()
                .map(|candidate| candidate.expected_response),
            Some(ExpectedResponseKindIR::Clarification)
        );
    }

    #[test]
    fn compound_response_intents_retain_primary_and_supporting_contributions() {
        let text =
            "If integration expands coverage, should we continue, and what evidence supports that?";
        let graph = UtteranceIntentAnalyzer.analyze(text, None, &[]);
        let active = graph.active().collect::<Vec<_>>();

        assert_eq!(active.len(), 2);
        assert_eq!(
            active[0].communicative_intent,
            CommunicativeIntentIR::ConditionalDecisionRequest
        );
        assert_eq!(
            active[1].communicative_intent,
            CommunicativeIntentIR::EvidenceRequest
        );
        assert_eq!(graph.supporting().count(), 1);
        assert!(graph.validate_against(text, None, &[]));

        let mut tampered = graph;
        tampered.selected_candidate_id = Some("UTTERANCE-INTENT-CANDIDATE-02".to_string());
        assert!(!tampered.validate());
    }

    #[test]
    fn response_goal_correction_subsumes_duplicate_explanation_candidate() {
        let text = "No, I was not asking you to inspect it; explain why it failed.";
        let graph = UtteranceIntentAnalyzer.analyze(text, Some("log"), &[]);

        assert_eq!(graph.active().count(), 1);
        assert_eq!(
            graph
                .selected()
                .map(|candidate| candidate.communicative_intent),
            Some(CommunicativeIntentIR::ResponseGoalCorrection)
        );
        assert!(graph.validate_against(text, Some("log"), &[]));
    }

    #[test]
    fn graph_hash_and_context_binding_reject_tampering() {
        let predicates = vec!["INVESTIGATE".to_string()];
        let graph = UtteranceIntentAnalyzer.analyze(
            "So what is the bottom line?",
            Some("cache state"),
            &predicates,
        );
        assert!(graph.validate_against(
            "So what is the bottom line?",
            Some("cache state"),
            &predicates
        ));
        assert!(!graph.validate_against(
            "So what is the takeaway?",
            Some("cache state"),
            &predicates
        ));
        let mut tampered = graph;
        tampered.candidates[0].target.push_str(" forged");
        assert!(!tampered.validate());
    }
}
