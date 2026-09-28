//! Typed workplace-channel framing for approved B_Core language output.
//!
//! The channel layer may arrange an already inverse-verified response, but it
//! cannot add facts.  Headings, greetings and closings are pragmatic scaffolds
//! only; the embedded `DocumentResponseOutputIR` remains the complete semantic
//! authority for the rendered message body.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::approved_response::ApprovedCompositionalResponseIR;
use crate::approved_response::{
    ApprovedDiscourseRelationIR, ApprovedRelationTypeIR, ApprovedSpeechActIR,
};
use crate::discourse_focus::{DiscourseFocusStateIR, MAX_DISCOURSE_FOCUS_TURN_DISTANCE};
use crate::document_response::{realize_document_response, DocumentResponseOutputIR};
use crate::language_knowledge::{LanguageCodeIR, LanguageRegisterIR};

pub const WORKPLACE_LANGUAGE_OUTPUT_SCHEMA: &str = "B_CORE_WORKPLACE_LANGUAGE_OUTPUT_IR_1";
pub const WORKPLACE_DIALOGUE_LANGUAGE_OUTPUT_SCHEMA: &str =
    "B_CORE_WORKPLACE_DIALOGUE_LANGUAGE_OUTPUT_IR_1";
const MAX_WORKPLACE_SURFACE_CHARS: usize = 48 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkplaceCommunicationFormatIR {
    InstantMessage,
    Email,
    StatusUpdate,
    IncidentAlert,
    MeetingSummary,
    HandoffNote,
    ApprovalRequest,
    CorrectionNotice,
    ExecutiveBrief,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkplaceAudienceIR {
    Peer,
    Manager,
    Team,
    Executive,
    ExternalPartner,
}

/// Reports whether every asserted Action relation has the predicate sense and
/// event frame required for a natural verbal realization.  This is not a
/// naturalness verdict: it only prevents a safe nominal fallback from being
/// mistaken for a fully specified event sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkplaceActionRealizationCompletenessIR {
    TypedEventFramesOnly,
    ContainsOpaqueActionFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceCommunicationContextIR {
    pub format: WorkplaceCommunicationFormatIR,
    pub audience: WorkplaceAudienceIR,
    /// Must be the lexical label of an approved subject.  It is used only in
    /// non-factual headings and subject lines.
    pub topic_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceLanguageOutputIR {
    pub schema: String,
    pub source_response_sha256: String,
    pub context: WorkplaceCommunicationContextIR,
    pub body: DocumentResponseOutputIR,
    pub rendered: String,
    pub action_realization_completeness: WorkplaceActionRealizationCompletenessIR,
    pub semantic_authority: bool,
    pub output_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkplaceDialogueDispositionIR {
    ContextualSubjectOmission,
    ExplicitSubjectFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceDialogueContextIR {
    pub workplace: WorkplaceCommunicationContextIR,
    pub discourse_focus: DiscourseFocusStateIR,
    pub completed_turns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceDialogueLanguageOutputIR {
    pub schema: String,
    pub source_response_sha256: String,
    pub context: WorkplaceDialogueContextIR,
    pub full_form: WorkplaceLanguageOutputIR,
    pub rendered: String,
    pub disposition: WorkplaceDialogueDispositionIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_subject_node_id: Option<String>,
    pub contextual_inverse_pass: bool,
    pub semantic_authority: bool,
    pub output_sha256: String,
}

impl WorkplaceLanguageOutputIR {
    pub fn validate(&self, response: &ApprovedCompositionalResponseIR) -> bool {
        self.schema == WORKPLACE_LANGUAGE_OUTPUT_SCHEMA
            && response.validate()
            && self.source_response_sha256 == response.semantic_sha256
            && context_is_valid(&self.context, response)
            && self.body.validate(response)
            && self.body.plan.output_language == LanguageCodeIR::Korean
            && self.action_realization_completeness
                == workplace_action_realization_completeness(response)
            && !self.rendered.trim().is_empty()
            && self.rendered.chars().count() <= MAX_WORKPLACE_SURFACE_CHARS
            && self.rendered
                == render_workplace_surface(
                    &self.context,
                    &self.body.markdown,
                    response.style.register,
                    response.claims.len(),
                    response.speech_act,
                    response.discourse_relation,
                )
            && self
                .body
                .semantic_interpretation
                .claims
                .iter()
                .all(|claim| self.rendered.contains(&claim.source_surface))
            && !self.semantic_authority
            && self.output_sha256 == workplace_language_output_sha256(self)
    }
}

pub fn realize_workplace_response(
    response: &ApprovedCompositionalResponseIR,
    context: WorkplaceCommunicationContextIR,
) -> Result<WorkplaceLanguageOutputIR, String> {
    if !response.validate() || !context_is_valid(&context, response) {
        return Err("INVALID_WORKPLACE_LANGUAGE_CONTEXT".into());
    }
    let body = realize_document_response(response, LanguageCodeIR::Korean)?;
    let rendered = render_workplace_surface(
        &context,
        &body.markdown,
        response.style.register,
        response.claims.len(),
        response.speech_act,
        response.discourse_relation,
    );
    let mut output = WorkplaceLanguageOutputIR {
        schema: WORKPLACE_LANGUAGE_OUTPUT_SCHEMA.into(),
        source_response_sha256: response.semantic_sha256.clone(),
        context,
        body,
        rendered,
        action_realization_completeness: workplace_action_realization_completeness(response),
        semantic_authority: false,
        output_sha256: String::new(),
    };
    output.output_sha256 = workplace_language_output_sha256(&output);
    output
        .validate(response)
        .then_some(output)
        .ok_or_else(|| "WORKPLACE_LANGUAGE_OUTPUT_VALIDATION_FAILED".into())
}

/// An Action claim is only a nominal action description until the approved
/// response binds its event subject to a typed predicate sense.  The check is
/// semantic and structural: it does not inspect Korean surface words or infer
/// a verb from an opaque lexical value.
pub fn workplace_action_realization_completeness(
    response: &ApprovedCompositionalResponseIR,
) -> WorkplaceActionRealizationCompletenessIR {
    if opaque_action_claim_count(response) > 0 {
        WorkplaceActionRealizationCompletenessIR::ContainsOpaqueActionFallback
    } else {
        WorkplaceActionRealizationCompletenessIR::TypedEventFramesOnly
    }
}

/// Counts only Action relations that lack an approved typed event binding.
/// This is intentionally semantic: free lexical values remain opaque even if
/// their Korean labels look verbal, while an Event-valued action is complete
/// only when the response carries its registered predicate sense.
pub fn opaque_action_claim_count(response: &ApprovedCompositionalResponseIR) -> usize {
    response
        .claims
        .iter()
        .filter(|claim| claim.relation == ApprovedRelationTypeIR::Action && claim.polarity)
        .filter(|claim| !typed_action_event_binding(claim, response))
        .count()
}

fn typed_action_event_binding(
    claim: &crate::approved_response::ApprovedCompositionalClaimIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    match &claim.value {
        crate::approved_response::ApprovedOpenValueIR::Lexical(node)
            if node.semantic_type == crate::approved_response::ApprovedSemanticTypeIR::Event =>
        {
            response.event_realizations.iter().any(|event| {
                event.subject_node_id == node.node_id && event.predicate_sense.is_some()
            })
        }
        _ => false,
    }
}

pub fn workplace_language_output_sha256(output: &WorkplaceLanguageOutputIR) -> String {
    let mut unsigned = output.clone();
    unsigned.output_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&unsigned).expect("workplace output serializes"))
    )
}

pub fn realize_workplace_dialogue_response(
    response: &ApprovedCompositionalResponseIR,
    context: WorkplaceDialogueContextIR,
) -> Result<WorkplaceDialogueLanguageOutputIR, String> {
    if context.workplace.format != WorkplaceCommunicationFormatIR::InstantMessage
        || !context.discourse_focus.validate(context.completed_turns)
    {
        return Err("INVALID_WORKPLACE_DIALOGUE_CONTEXT".into());
    }
    let full_form = realize_workplace_response(response, context.workplace.clone())?;
    let subjects = response
        .claims
        .iter()
        .map(|claim| &claim.subject)
        .collect::<Vec<_>>();
    let one_subject = subjects
        .first()
        .filter(|subject| subjects.iter().all(|candidate| *candidate == **subject));
    let eligible = one_subject.and_then(|subject| {
        let focus = context.discourse_focus.current()?;
        (focus.concept_id_hint.as_deref() == Some(subject.node_id.as_str())
            && focus.surface.trim() == subject.canonical_lexical_label.trim()
            && context
                .completed_turns
                .saturating_sub(focus.last_focused_turn)
                <= MAX_DISCOURSE_FOCUS_TURN_DISTANCE)
            .then_some((subject, focus.focus_id.as_str()))
    });
    let (rendered, disposition, recovered_subject_node_id) = eligible
        .and_then(|(subject, _)| {
            omit_contextually_bound_subject(
                &full_form.rendered,
                subject.canonical_lexical_label.trim(),
                response,
            )
            .map(|surface| {
                (
                    surface,
                    WorkplaceDialogueDispositionIR::ContextualSubjectOmission,
                    Some(subject.node_id.clone()),
                )
            })
        })
        .unwrap_or_else(|| {
            (
                full_form.rendered.clone(),
                WorkplaceDialogueDispositionIR::ExplicitSubjectFallback,
                None,
            )
        });
    let mut output = WorkplaceDialogueLanguageOutputIR {
        schema: WORKPLACE_DIALOGUE_LANGUAGE_OUTPUT_SCHEMA.into(),
        source_response_sha256: response.semantic_sha256.clone(),
        context,
        full_form,
        rendered,
        disposition,
        recovered_subject_node_id,
        contextual_inverse_pass: true,
        semantic_authority: false,
        output_sha256: String::new(),
    };
    output.output_sha256 = workplace_dialogue_output_sha256(&output);
    output
        .validate(response)
        .then_some(output)
        .ok_or_else(|| "WORKPLACE_DIALOGUE_OUTPUT_VALIDATION_FAILED".into())
}

impl WorkplaceDialogueLanguageOutputIR {
    pub fn validate(&self, response: &ApprovedCompositionalResponseIR) -> bool {
        if self.schema != WORKPLACE_DIALOGUE_LANGUAGE_OUTPUT_SCHEMA
            || self.source_response_sha256 != response.semantic_sha256
            || self.context.workplace.format != WorkplaceCommunicationFormatIR::InstantMessage
            || !self
                .context
                .discourse_focus
                .validate(self.context.completed_turns)
            || !self.full_form.validate(response)
            || !self.contextual_inverse_pass
            || self.semantic_authority
            || self.output_sha256 != workplace_dialogue_output_sha256(self)
        {
            return false;
        }
        let subjects = response
            .claims
            .iter()
            .map(|claim| &claim.subject)
            .collect::<Vec<_>>();
        let one_subject = subjects
            .first()
            .filter(|subject| subjects.iter().all(|candidate| *candidate == **subject));
        let eligible = one_subject.and_then(|subject| {
            let focus = self.context.discourse_focus.current()?;
            (focus.concept_id_hint.as_deref() == Some(subject.node_id.as_str())
                && focus.surface.trim() == subject.canonical_lexical_label.trim()
                && self
                    .context
                    .completed_turns
                    .saturating_sub(focus.last_focused_turn)
                    <= MAX_DISCOURSE_FOCUS_TURN_DISTANCE)
                .then_some(subject)
        });
        match (self.disposition, eligible) {
            (WorkplaceDialogueDispositionIR::ContextualSubjectOmission, Some(subject)) => {
                self.recovered_subject_node_id.as_deref() == Some(subject.node_id.as_str())
                    && omit_contextually_bound_subject(
                        &self.full_form.rendered,
                        subject.canonical_lexical_label.trim(),
                        response,
                    )
                    .as_deref()
                        == Some(self.rendered.as_str())
            }
            (WorkplaceDialogueDispositionIR::ExplicitSubjectFallback, _) => {
                self.recovered_subject_node_id.is_none() && self.rendered == self.full_form.rendered
            }
            _ => false,
        }
    }
}

pub fn workplace_dialogue_output_sha256(output: &WorkplaceDialogueLanguageOutputIR) -> String {
    let mut unsigned = output.clone();
    unsigned.output_sha256.clear();
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&unsigned).expect("workplace dialogue output serializes")
        )
    )
}

fn omit_contextually_bound_subject(
    surface: &str,
    subject: &str,
    response: &ApprovedCompositionalResponseIR,
) -> Option<String> {
    // A COUNT clause would collapse to a bare scalar (for example,
    // `23건입니다.`).  That is acceptable only under a typed question/answer
    // contract, which this workplace focus context does not yet prove.  Keep
    // the explicit subject rather than adding an unapproved temporal or total
    // marker to make the fragment sound fuller.
    if response
        .claims
        .iter()
        .all(|claim| claim.relation == ApprovedRelationTypeIR::Count)
    {
        return None;
    }
    let mut changed = false;
    let mut realized = Vec::new();
    for segment in surface.split_inclusive(['.', '?', '!']) {
        let sentence = segment.trim();
        if sentence.is_empty() {
            continue;
        }
        let transformed = if let Some(rest) = sentence.strip_prefix(subject) {
            if let Some(rest) = rest.strip_prefix(' ') {
                changed = true;
                rest.to_string()
            } else {
                let mut omitted = None;
                for particle in ["은 ", "는 ", "이 ", "가 ", "을 ", "를 ", "에 "] {
                    if let Some(rest) = rest.strip_prefix(particle) {
                        changed = true;
                        omitted = Some(rest.to_string());
                        break;
                    }
                }
                omitted.unwrap_or_else(|| sentence.to_string())
            }
        } else {
            sentence.to_string()
        };
        realized.push(transformed);
    }
    changed.then(|| realized.join(" "))
}

fn context_is_valid(
    context: &WorkplaceCommunicationContextIR,
    response: &ApprovedCompositionalResponseIR,
) -> bool {
    let topic = context.topic_label.trim();
    !topic.is_empty()
        && topic.chars().count() <= 80
        && !topic.contains(['\n', '\r', '|', '<', '>'])
        && response
            .claims
            .iter()
            .any(|claim| claim.subject.canonical_lexical_label == topic)
}

fn render_workplace_surface(
    context: &WorkplaceCommunicationContextIR,
    body: &str,
    register: LanguageRegisterIR,
    claim_count: usize,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
) -> String {
    let topic = context.topic_label.trim();
    // Short workplace messages read better without document scaffolding.  In
    // a genuinely long mail or meeting record, however, removing every
    // section heading turns separate numbered blocks into an unlabeled list
    // whose numbering repeatedly restarts.  Preserve only inner section
    // headings for long structured channels; the channel still owns its top
    // level title, salutation, and closing.
    let channel_body = |body: &str| {
        if claim_count >= 8 {
            structured_operational_body(body)
        } else {
            operational_body(body)
        }
    };
    match context.format {
        WorkplaceCommunicationFormatIR::InstantMessage => operational_body(body)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        WorkplaceCommunicationFormatIR::Email => format!(
            "제목: {}\n\n{}\n\n{}\n\n{}",
            email_subject(topic, speech_act, discourse_relation),
            email_salutation(context.audience, register),
            channel_body(body),
            email_closing(register),
        ),
        WorkplaceCommunicationFormatIR::StatusUpdate => {
            format!("## {topic} 현황\n\n{}", channel_body(body))
        }
        WorkplaceCommunicationFormatIR::IncidentAlert => {
            format!("[긴급] {topic}\n\n{}", channel_body(body))
        }
        WorkplaceCommunicationFormatIR::MeetingSummary => format!(
            "## {}\n\n{}",
            if ["회의", "미팅", "면접", "리허설", "합주"]
                .iter()
                .any(|suffix| topic.ends_with(suffix))
            {
                format!("{topic} 요약")
            } else {
                format!("{topic} 회의 요약")
            },
            channel_body(body),
        ),
        WorkplaceCommunicationFormatIR::HandoffNote => {
            format!("## {topic} 인수인계\n\n{}", channel_body(body))
        }
        WorkplaceCommunicationFormatIR::ApprovalRequest => format!(
            "## {}\n\n{}",
            if topic.ends_with("요청") {
                format!("{topic} 검토")
            } else {
                format!("{topic} 검토 요청")
            },
            channel_body(body),
        ),
        WorkplaceCommunicationFormatIR::CorrectionNotice => {
            format!("## {topic} 정정 안내\n\n{}", channel_body(body))
        }
        WorkplaceCommunicationFormatIR::ExecutiveBrief => {
            // A claim count is an internal planning measure, not a reader-facing
            // topic.  Keeping it out of the heading avoids titles such as
            // `… 외 50개 항목 핵심 보고` while preserving the approved body.
            let heading = format!("## {topic} 핵심 보고");
            if let Some(start) = body.find("## ") {
                let end = body[start..]
                    .find('\n')
                    .map(|offset| start + offset)
                    .unwrap_or(body.len());
                compact_blank_lines(&format!("{heading}\n\n{}{}", &body[..start], &body[end..]))
            } else {
                format!("{heading}\n\n{body}")
            }
        }
    }
}

fn email_subject(
    topic: &str,
    speech_act: ApprovedSpeechActIR,
    discourse_relation: ApprovedDiscourseRelationIR,
) -> String {
    let suffix = if discourse_relation == ApprovedDiscourseRelationIR::Correction {
        "정정 안내"
    } else {
        match speech_act {
            ApprovedSpeechActIR::Query => "확인 요청",
            ApprovedSpeechActIR::Request => "검토 요청",
            ApprovedSpeechActIR::Promise => "처리 안내",
            ApprovedSpeechActIR::Reassure => "안내",
            _ => "관련 공유",
        }
    };
    if topic.ends_with("요청") && suffix.ends_with("요청") {
        format!("{topic} 검토")
    } else if topic.ends_with("안내") && suffix.ends_with("안내") {
        topic.to_string()
    } else {
        format!("{topic} {suffix}")
    }
}

fn compact_blank_lines(surface: &str) -> String {
    let mut compact = Vec::new();
    let mut prior_blank = false;
    let has_chart = surface.contains("```mermaid");
    for line in surface.lines() {
        // A numeric chart and its preceding prose already carry the same
        // approved values.  In an executive brief, omit the mechanically
        // duplicated Markdown table while retaining the chart.  This is a
        // presentation projection only; semantic recovery remains anchored
        // in the unchanged prose clauses.
        if has_chart && line.trim_start().starts_with('|') {
            continue;
        }
        let blank = line.trim().is_empty();
        if blank && prior_blank {
            continue;
        }
        compact.push(line);
        prior_blank = blank;
    }
    compact.join("\n").trim().to_string()
}

/// Status and incident channels already provide their own title and evidence
/// boundary.  Remove document-only scaffolding while leaving every semantic
/// clause byte-for-byte intact for inverse validation.
fn operational_body(body: &str) -> String {
    filtered_operational_body(body, false)
}

fn structured_operational_body(body: &str) -> String {
    filtered_operational_body(body, true)
}

fn filtered_operational_body(body: &str, preserve_inner_headings: bool) -> String {
    const DOCUMENT_SCAFFOLDS: &[&str] = &[
        "핵심 내용을 항목별로 정리했습니다.",
        "핵심 내용을 항목별로 정리했어요.",
        "핵심 내용을 항목별로 정리했어.",
        "확인된 내용을 흐름에 따라 정리했습니다.",
        "확인된 내용을 흐름에 따라 정리했어요.",
        "확인된 내용을 흐름에 따라 정리했어.",
        "위 내용은 확인된 정보만 반영했습니다.",
        "위 내용은 확인된 정보만 반영했어요.",
        "위 내용은 확인된 정보만 반영했어.",
        "The confirmed points are organized by topic below.",
        "The confirmed information is organized in sequence below.",
        "This response stays within the confirmed evidence.",
    ];
    let filtered = body
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            let is_preserved_inner_heading = preserve_inner_headings && trimmed.starts_with("### ");
            (!trimmed.starts_with('#') || is_preserved_inner_heading)
                && !DOCUMENT_SCAFFOLDS.contains(&trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n");
    compact_blank_lines(&filtered)
}

fn email_salutation(audience: WorkplaceAudienceIR, register: LanguageRegisterIR) -> &'static str {
    match (audience, register) {
        (WorkplaceAudienceIR::Manager, LanguageRegisterIR::Formal) => "안녕하세요, 팀장님.",
        (WorkplaceAudienceIR::Executive, LanguageRegisterIR::Formal) => "안녕하세요.",
        (WorkplaceAudienceIR::ExternalPartner, _) => "안녕하세요.",
        (_, LanguageRegisterIR::Informal | LanguageRegisterIR::Internet) => "안녕하세요.",
        _ => "안녕하세요.",
    }
}

fn email_closing(register: LanguageRegisterIR) -> &'static str {
    match register {
        LanguageRegisterIR::Formal => "감사합니다.",
        LanguageRegisterIR::Neutral => "확인 부탁드려요.",
        LanguageRegisterIR::Informal | LanguageRegisterIR::Internet => "확인 부탁해요.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approved_response::{
        compositional_response_sha256, ApprovedClauseUnitIR, ApprovedCompositionalClaimIR,
        ApprovedDiscourseRelationIR, ApprovedLexicalNodeIR, ApprovedModalityIR,
        ApprovedOpenValueIR, ApprovedOperationIR, ApprovedRelationTypeIR, ApprovedResponseStyleIR,
        ApprovedSemanticTypeIR, ApprovedSpeechActIR, ApprovedVerbosityIR,
        COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA,
    };
    use crate::discourse_focus::DiscourseFocusCandidateIR;

    fn approved_status() -> ApprovedCompositionalResponseIR {
        let claim = ApprovedCompositionalClaimIR {
            proposition_id: "P_STATUS".into(),
            subject: ApprovedLexicalNodeIR {
                node_id: "release".into(),
                semantic_type: ApprovedSemanticTypeIR::Event,
                canonical_lexical_label: "배포".into(),
            },
            relation: ApprovedRelationTypeIR::Status,
            value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                node_id: "ready".into(),
                semantic_type: ApprovedSemanticTypeIR::State,
                canonical_lexical_label: "준비".into(),
            }),
            polarity: true,
            modality: ApprovedModalityIR::Asserted,
    status_frame: None,
};
        let mut response = ApprovedCompositionalResponseIR {
            schema: COMPOSITIONAL_APPROVED_RESPONSE_SCHEMA.into(),
            speech_act: ApprovedSpeechActIR::Inform,
            operation: ApprovedOperationIR::Assert,
            claims: vec![claim],
            event_realizations: Vec::new(),
            discourse_relation: ApprovedDiscourseRelationIR::Statement,
            clause_plan: vec![ApprovedClauseUnitIR {
                unit_index: 0,
                role: "ASSERT".into(),
                proposition_ids: vec!["P_STATUS".into()],
                predecessor_indices: Vec::new(),
            }],
            style: ApprovedResponseStyleIR {
                register: LanguageRegisterIR::Formal,
                verbosity: ApprovedVerbosityIR::Short,
            },
            source_world_state_sha256: "a".repeat(64),
            source_deliberation_sha256: "b".repeat(64),
            approval_replay_verified: true,
            unsupported_claims: 0,
            semantic_sha256: String::new(),
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());
        response
    }

    #[test]
    fn workplace_scaffolding_does_not_change_the_approved_body() {
        let response = approved_status();
        for format in [
            WorkplaceCommunicationFormatIR::InstantMessage,
            WorkplaceCommunicationFormatIR::Email,
            WorkplaceCommunicationFormatIR::StatusUpdate,
            WorkplaceCommunicationFormatIR::IncidentAlert,
            WorkplaceCommunicationFormatIR::MeetingSummary,
            WorkplaceCommunicationFormatIR::HandoffNote,
            WorkplaceCommunicationFormatIR::ApprovalRequest,
            WorkplaceCommunicationFormatIR::CorrectionNotice,
            WorkplaceCommunicationFormatIR::ExecutiveBrief,
        ] {
            let output = realize_workplace_response(
                &response,
                WorkplaceCommunicationContextIR {
                    format,
                    audience: WorkplaceAudienceIR::Team,
                    topic_label: "배포".into(),
                },
            )
            .unwrap();
            assert!(output.validate(&response));
            assert!(output.rendered.contains("배포는 준비됐습니다."));
            assert_eq!(
                output.action_realization_completeness,
                WorkplaceActionRealizationCompletenessIR::TypedEventFramesOnly
            );
            assert_eq!(
                output.body.semantic_interpretation.recovered_claim_ids,
                ["P_STATUS"]
            );
        }
    }

    #[test]
    fn typed_action_event_is_not_reported_as_an_opaque_workplace_action() {
        let mut response = approved_status();
        response.claims = vec![
            ApprovedCompositionalClaimIR {
                proposition_id: "P_ACTION".into(),
                subject: ApprovedLexicalNodeIR {
                    node_id: "payment_incident".into(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: "결제 장애".into(),
                },
                relation: ApprovedRelationTypeIR::Action,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "coupon_deactivation".into(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: "할인 쿠폰 배포 비활성화".into(),
                }),
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
},
            ApprovedCompositionalClaimIR {
                proposition_id: "P_AGENT".into(),
                subject: ApprovedLexicalNodeIR {
                    node_id: "coupon_deactivation".into(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: "할인 쿠폰 배포 비활성화".into(),
                },
                relation: ApprovedRelationTypeIR::Agent,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "operations_team".into(),
                    semantic_type: ApprovedSemanticTypeIR::Organization,
                    canonical_lexical_label: "운영팀".into(),
                }),
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
},
            ApprovedCompositionalClaimIR {
                proposition_id: "P_THEME".into(),
                subject: ApprovedLexicalNodeIR {
                    node_id: "coupon_deactivation".into(),
                    semantic_type: ApprovedSemanticTypeIR::Event,
                    canonical_lexical_label: "할인 쿠폰 배포 비활성화".into(),
                },
                relation: ApprovedRelationTypeIR::Theme,
                value: ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
                    node_id: "coupon_distribution".into(),
                    semantic_type: ApprovedSemanticTypeIR::Concept,
                    canonical_lexical_label: "할인 쿠폰 배포".into(),
                }),
                polarity: true,
                modality: ApprovedModalityIR::Asserted,
    status_frame: None,
},
        ];
        response.clause_plan = response
            .claims
            .iter()
            .enumerate()
            .map(|(index, claim)| ApprovedClauseUnitIR {
                unit_index: index,
                role: "ASSERT".into(),
                proposition_ids: vec![claim.proposition_id.clone()],
                predecessor_indices: (index > 0).then(|| index - 1).into_iter().collect(),
            })
            .collect();
        response.event_realizations = vec![crate::approved_response::ApprovedEventRealizationIR {
            subject_node_id: "coupon_deactivation".into(),
            class: crate::approved_response::ApprovedEventRealizationClassIR::StateChange,
            predicate_sense: Some(crate::approved_response::ApprovedEventPredicateSenseIR::Deactivate),
            phase: Some(crate::approved_response::ApprovedEventPhaseIR::Planned),
            perspective: Some(crate::approved_response::ApprovedEventPerspectiveIR {
                voice: crate::approved_response::ApprovedEventVoiceIR::Active,
                focus: crate::approved_response::ApprovedEventFocusIR::Agent,
            }),
            voice: None,
            information_structure: None,
        }];
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());

        let output = realize_workplace_response(
            &response,
            WorkplaceCommunicationContextIR {
                format: WorkplaceCommunicationFormatIR::IncidentAlert,
                audience: WorkplaceAudienceIR::Team,
                topic_label: "결제 장애".into(),
            },
        )
        .expect("typed action event should be workplace renderable");
        assert!(output.validate(&response));
        assert_eq!(
            output.action_realization_completeness,
            WorkplaceActionRealizationCompletenessIR::TypedEventFramesOnly
        );
        assert_eq!(opaque_action_claim_count(&response), 0);
        assert!(output
            .rendered
            .contains("운영팀이 할인 쿠폰 배포를 비활성화할 예정입니다."));
    }

    #[test]
    fn opaque_action_fallback_is_explicit_in_the_workplace_output_contract() {
        let mut response = approved_status();
        response.claims[0].relation = ApprovedRelationTypeIR::Action;
        response.claims[0].value = ApprovedOpenValueIR::Lexical(ApprovedLexicalNodeIR {
            node_id: "coupon_rollback".into(),
            semantic_type: ApprovedSemanticTypeIR::Concept,
            canonical_lexical_label: "할인 쿠폰 배포 롤백".into(),
        });
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());

        let output = realize_workplace_response(
            &response,
            WorkplaceCommunicationContextIR {
                format: WorkplaceCommunicationFormatIR::IncidentAlert,
                audience: WorkplaceAudienceIR::Team,
                topic_label: "배포".into(),
            },
        )
        .expect("opaque action remains semantically renderable");
        assert_eq!(
            output.action_realization_completeness,
            WorkplaceActionRealizationCompletenessIR::ContainsOpaqueActionFallback
        );
        assert_eq!(
            workplace_action_realization_completeness(&response),
            WorkplaceActionRealizationCompletenessIR::ContainsOpaqueActionFallback
        );
        assert_eq!(opaque_action_claim_count(&response), 1);
        assert!(output.validate(&response));
    }

    #[test]
    fn executive_brief_heading_does_not_expose_internal_claim_counts() {
        let rendered = render_workplace_surface(
            &WorkplaceCommunicationContextIR {
                format: WorkplaceCommunicationFormatIR::ExecutiveBrief,
                audience: WorkplaceAudienceIR::Executive,
                topic_label: "동계 관측 준비".into(),
            },
            "확인된 내용을 정리했습니다.",
            LanguageRegisterIR::Formal,
            51,
            ApprovedSpeechActIR::Inform,
            ApprovedDiscourseRelationIR::Statement,
        );
        assert!(rendered.starts_with("## 동계 관측 준비 핵심 보고"));
        assert!(!rendered.contains("외 50개 항목"));
    }

    #[test]
    fn topic_heading_cannot_introduce_an_unapproved_subject() {
        let response = approved_status();
        let error = realize_workplace_response(
            &response,
            WorkplaceCommunicationContextIR {
                format: WorkplaceCommunicationFormatIR::Email,
                audience: WorkplaceAudienceIR::ExternalPartner,
                topic_label: "계약 해지".into(),
            },
        )
        .unwrap_err();
        assert_eq!(error, "INVALID_WORKPLACE_LANGUAGE_CONTEXT");
    }

    #[test]
    fn operational_channel_removes_document_scaffolding_but_keeps_claim_sentences() {
        let body = "핵심 내용을 항목별로 정리했습니다.\n\n## 핵심 내용\n\n### 1. 이유와 설명\n\n결제 장애 원인은 교착 상태입니다.\n\n### 2. 상태와 결정\n\n결제 장애 조치는 롤백입니다.\n\n위 내용은 확인된 정보만 반영했습니다.";
        assert_eq!(
            operational_body(body),
            "결제 장애 원인은 교착 상태입니다.\n\n결제 장애 조치는 롤백입니다."
        );
    }

    #[test]
    fn structured_operational_body_preserves_only_inner_section_headings() {
        let body = "핵심 내용을 항목별로 정리했습니다.\n\n## 내부 문서 제목\n\n### 1. 원인\n\n장애 원인은 교착 상태입니다.\n\n### 2. 조치\n\n장애 조치는 롤백입니다.\n\n위 내용은 확인된 정보만 반영했습니다.";
        assert_eq!(
            structured_operational_body(body),
            "### 1. 원인\n\n장애 원인은 교착 상태입니다.\n\n### 2. 조치\n\n장애 조치는 롤백입니다."
        );
    }

    fn dialogue_context(
        surface: &str,
        concept_id: &str,
        completed_turns: u64,
    ) -> WorkplaceDialogueContextIR {
        let mut discourse_focus = DiscourseFocusStateIR::default();
        discourse_focus.apply_turn(
            1,
            &[DiscourseFocusCandidateIR::explicit_topic(
                surface,
                Some(concept_id),
            )],
        );
        WorkplaceDialogueContextIR {
            workplace: WorkplaceCommunicationContextIR {
                format: WorkplaceCommunicationFormatIR::InstantMessage,
                audience: WorkplaceAudienceIR::Peer,
                topic_label: "배포".into(),
            },
            discourse_focus,
            completed_turns,
        }
    }

    #[test]
    fn dialogue_subject_ellipsis_requires_an_exact_live_focus_binding() {
        let response = approved_status();
        let output =
            realize_workplace_dialogue_response(&response, dialogue_context("배포", "release", 2))
                .expect("exact live focus should license subject omission");
        assert_eq!(
            output.disposition,
            WorkplaceDialogueDispositionIR::ContextualSubjectOmission
        );
        assert_eq!(output.rendered, "준비됐습니다.");
        assert_eq!(output.recovered_subject_node_id.as_deref(), Some("release"));
        assert!(output.validate(&response));

        let mut tampered_surface = output.clone();
        tampered_surface.rendered = "완료됐습니다.".into();
        tampered_surface.output_sha256 = workplace_dialogue_output_sha256(&tampered_surface);
        assert!(!tampered_surface.validate(&response));

        let mut tampered_binding = output.clone();
        tampered_binding.context.discourse_focus.nodes[0].concept_id_hint =
            Some("different_release".into());
        tampered_binding.output_sha256 = workplace_dialogue_output_sha256(&tampered_binding);
        assert!(!tampered_binding.validate(&response));

        let mismatch =
            realize_workplace_dialogue_response(&response, dialogue_context("계약", "contract", 2))
                .expect("mismatched focus should safely keep the explicit subject");
        assert_eq!(
            mismatch.disposition,
            WorkplaceDialogueDispositionIR::ExplicitSubjectFallback
        );
        assert_eq!(mismatch.rendered, "배포는 준비됐습니다.");

        let stale =
            realize_workplace_dialogue_response(&response, dialogue_context("배포", "release", 18))
                .expect("stale focus should safely keep the explicit subject");
        assert_eq!(
            stale.disposition,
            WorkplaceDialogueDispositionIR::ExplicitSubjectFallback
        );
        assert_eq!(stale.rendered, "배포는 준비됐습니다.");
    }

    #[test]
    fn dialogue_count_without_a_typed_question_keeps_the_explicit_subject() {
        let mut response = approved_status();
        response.claims[0].subject = ApprovedLexicalNodeIR {
            node_id: "unassigned_ticket".into(),
            semantic_type: ApprovedSemanticTypeIR::Concept,
            canonical_lexical_label: "미배정 문의".into(),
        };
        response.claims[0].relation = ApprovedRelationTypeIR::Count;
        response.claims[0].value = ApprovedOpenValueIR::Quantity {
            amount: 23,
            unit: ApprovedLexicalNodeIR {
                node_id: "case_unit".into(),
                semantic_type: ApprovedSemanticTypeIR::Unit,
                canonical_lexical_label: "건".into(),
            },
        };
        response.semantic_sha256 = compositional_response_sha256(&response);
        assert!(response.validate());

        let mut discourse_focus = DiscourseFocusStateIR::default();
        discourse_focus.apply_turn(
            1,
            &[DiscourseFocusCandidateIR::explicit_topic(
                "미배정 문의",
                Some("unassigned_ticket"),
            )],
        );
        let output = realize_workplace_dialogue_response(
            &response,
            WorkplaceDialogueContextIR {
                workplace: WorkplaceCommunicationContextIR {
                    format: WorkplaceCommunicationFormatIR::InstantMessage,
                    audience: WorkplaceAudienceIR::Peer,
                    topic_label: "미배정 문의".into(),
                },
                discourse_focus,
                completed_turns: 2,
            },
        )
        .expect("a bare count remnant should safely keep the explicit subject");

        assert_eq!(output.full_form.rendered, "미배정 문의는 23건입니다.");
        assert_eq!(output.rendered, "미배정 문의는 23건입니다.");
        assert_eq!(
            output.disposition,
            WorkplaceDialogueDispositionIR::ExplicitSubjectFallback
        );
        assert!(output.validate(&response));
    }
}
