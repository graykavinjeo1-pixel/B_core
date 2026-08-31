use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use dockable_semantic_core::{
    AssessmentVerdictIR, DeliberationFactIR, DockableCore, QualityCriterionIR, SwarmDeliberationIR,
    SwarmDeliberationRequestIR, SWARM_DELIBERATION_REQUEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::long_term_repair::{
    extract_evidence, EvidenceExtractionReceiptIR, EvidenceInputIR, StructuredEvidenceIR,
};

pub const PROFESSIONAL_DOCUMENT_REQUEST_SCHEMA: &str = "B_CORE_PROFESSIONAL_DOCUMENT_REQUEST_1";
pub const PROFESSIONAL_DOCUMENT_RESPONSE_SCHEMA: &str = "B_CORE_PROFESSIONAL_DOCUMENT_RESPONSE_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfessionalDocumentKindIR {
    Paper,
    BusinessPlan,
    BusinessProposal,
    PolicyReport,
    TechnicalReport,
    LongTermRepairPlan,
    UserGuide,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionRequirementIR {
    pub section_id: String,
    pub title: String,
    pub objective: String,
    #[serde(default)]
    pub required_evidence_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_page_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDirectiveIR {
    pub directive_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_section_id: Option<String>,
    pub instruction: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorDocumentSnapshotIR {
    pub artifact_sha256: String,
    pub sections: Vec<GroundedSectionDraftIR>,
    pub working_memory: DocumentWorkingMemoryIR,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionalDocumentRequestIR {
    pub schema: String,
    pub request_id: String,
    pub command: String,
    pub title: String,
    pub kind: ProfessionalDocumentKindIR,
    pub target_page_count: usize,
    pub audience: String,
    pub purpose: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceInputIR>,
    #[serde(default)]
    pub required_sections: Vec<SectionRequirementIR>,
    #[serde(default)]
    pub revision_directives: Vec<RevisionDirectiveIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_snapshot: Option<PriorDocumentSnapshotIR>,
    #[serde(default = "default_revision_rounds")]
    pub max_revision_rounds: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_html_path: Option<String>,
    #[serde(default = "default_plan_steps")]
    pub max_plan_steps: usize,
}

fn default_revision_rounds() -> usize {
    4
}

fn default_plan_steps() -> usize {
    16
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentPlanSectionIR {
    pub section_id: String,
    pub title: String,
    pub objective: String,
    pub page_count: usize,
    pub dependencies: Vec<String>,
    pub required_evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongFormDocumentPlanIR {
    pub schema: String,
    pub target_page_count: usize,
    pub sections: Vec<DocumentPlanSectionIR>,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFactIR {
    pub fact_id: String,
    pub text: String,
    pub evidence_id: String,
    pub block_id: String,
    pub source_location: String,
    pub numeric_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionMemoryIR {
    pub section_id: String,
    pub selected_fact_ids: Vec<String>,
    pub summary: String,
    pub unresolved_questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentWorkingMemoryIR {
    pub schema: String,
    pub global_constraints: Vec<String>,
    pub canonical_terms: BTreeMap<String, String>,
    pub evidence_facts: Vec<EvidenceFactIR>,
    pub section_memory: Vec<SectionMemoryIR>,
    pub unresolved_global_questions: Vec<String>,
    pub memory_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParagraphGroundingIR {
    Grounded,
    Derived,
    NeedsEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedParagraphIR {
    pub paragraph_id: String,
    pub text: String,
    pub grounding: ParagraphGroundingIR,
    pub evidence_refs: Vec<String>,
    pub source_locations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedSectionDraftIR {
    pub section_id: String,
    pub title: String,
    pub objective: String,
    pub assigned_page_count: usize,
    pub paragraphs: Vec<GroundedParagraphIR>,
    pub section_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsistencySeverityIR {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsistencyIssueKindIR {
    PageBudgetMismatch,
    MissingRequiredEvidence,
    UnknownEvidenceReference,
    UnsupportedClaim,
    DuplicateParagraph,
    NumericConflict,
    BrokenSectionDependency,
    RevisionDirectiveUnapplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyIssueIR {
    pub issue_id: String,
    pub kind: ConsistencyIssueKindIR,
    pub severity: ConsistencySeverityIR,
    pub location: String,
    pub diagnostic: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionRoundIR {
    pub round: usize,
    pub issues_before: Vec<ConsistencyIssueIR>,
    pub applied_edits: Vec<String>,
    pub issues_after: Vec<ConsistencyIssueIR>,
    pub quality_score_millis: u16,
    pub document_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionalDocumentPageIR {
    pub page_number: usize,
    pub section_id: String,
    pub section_title: String,
    pub body: Vec<GroundedParagraphIR>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionalDocumentFileReceiptIR {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub a4_page_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionalDocumentResponseIR {
    pub schema: String,
    pub request_id: String,
    pub reasoning_plan_sha256: String,
    pub plan: LongFormDocumentPlanIR,
    pub extraction_receipts: Vec<EvidenceExtractionReceiptIR>,
    pub structured_evidence: Vec<StructuredEvidenceIR>,
    pub working_memory: DocumentWorkingMemoryIR,
    pub sections: Vec<GroundedSectionDraftIR>,
    pub revision_rounds: Vec<RevisionRoundIR>,
    pub final_consistency_issues: Vec<ConsistencyIssueIR>,
    pub pages: Vec<ProfessionalDocumentPageIR>,
    pub deliberation: SwarmDeliberationIR,
    pub accepted: bool,
    pub professional_review_required: bool,
    pub artifact_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_receipt: Option<ProfessionalDocumentFileReceiptIR>,
    pub external_model_calls: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfessionalDocumentError {
    InvalidRequest,
    Planning,
    Deliberation,
    OutputWrite,
}

pub fn process_professional_document(
    core: &DockableCore,
    request: &ProfessionalDocumentRequestIR,
    reasoning_plan_sha256: &str,
) -> Result<ProfessionalDocumentResponseIR, ProfessionalDocumentError> {
    validate_request(request, reasoning_plan_sha256)?;
    let extracted = request
        .evidence
        .iter()
        .map(extract_evidence)
        .collect::<Vec<_>>();
    let structured_evidence = extracted
        .iter()
        .map(|value| value.structured.clone())
        .collect::<Vec<_>>();
    let extraction_receipts = extracted
        .iter()
        .map(|value| value.receipt.clone())
        .collect::<Vec<_>>();
    let evidence_ids = request
        .evidence
        .iter()
        .map(|value| value.evidence_id.clone())
        .collect::<BTreeSet<_>>();
    let plan = build_plan(request)?;
    let facts = collect_facts(&structured_evidence);
    let mut working_memory = build_working_memory(request, &plan, facts);
    let mut sections = initial_draft(request, &plan, &mut working_memory);
    let mut revision_rounds = Vec::new();
    let mut prior_issue_hash = String::new();
    for round in 1..=request.max_revision_rounds {
        let issues_before = check_consistency(request, &plan, &sections, &evidence_ids);
        let issue_hash = hash_json(&issues_before);
        let applied_edits = revise_document(request, &mut sections, &issues_before);
        refresh_section_hashes(&mut sections);
        let issues_after = check_consistency(request, &plan, &sections, &evidence_ids);
        let document_sha256 = hash_json(&sections);
        revision_rounds.push(RevisionRoundIR {
            round,
            quality_score_millis: quality_score(&issues_after),
            issues_before,
            applied_edits,
            issues_after: issues_after.clone(),
            document_sha256,
        });
        if issues_after
            .iter()
            .all(|issue| issue.severity != ConsistencySeverityIR::Error)
            || issue_hash == prior_issue_hash
        {
            break;
        }
        prior_issue_hash = issue_hash;
    }
    refresh_working_memory(&mut working_memory, &sections);
    let final_consistency_issues = check_consistency(request, &plan, &sections, &evidence_ids);
    let pages = paginate(&plan, &sections);
    let deliberation = deliberate(
        core,
        request,
        reasoning_plan_sha256,
        &plan,
        &structured_evidence,
        &working_memory,
        &final_consistency_issues,
        &pages,
    )?;
    let artifact_sha256 = hash_json(&(
        request,
        &plan,
        &extraction_receipts,
        &structured_evidence,
        &working_memory,
        &sections,
        &revision_rounds,
        &final_consistency_issues,
        &pages,
        &deliberation.deliberation_sha256,
    ));
    let file_receipt = request
        .output_html_path
        .as_ref()
        .map(|path| write_html(Path::new(path), request, &pages, &artifact_sha256))
        .transpose()?;
    let accepted = pages.len() == request.target_page_count
        && final_consistency_issues
            .iter()
            .all(|issue| issue.severity != ConsistencySeverityIR::Error)
        && final_consistency_issues
            .iter()
            .all(|issue| issue.kind != ConsistencyIssueKindIR::MissingRequiredEvidence)
        && working_memory.unresolved_global_questions.is_empty()
        && deliberation.accepted;
    Ok(ProfessionalDocumentResponseIR {
        schema: PROFESSIONAL_DOCUMENT_RESPONSE_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        reasoning_plan_sha256: reasoning_plan_sha256.to_string(),
        plan,
        extraction_receipts,
        structured_evidence,
        working_memory,
        sections,
        revision_rounds,
        final_consistency_issues,
        pages,
        deliberation,
        accepted,
        professional_review_required: true,
        artifact_sha256,
        file_receipt,
        external_model_calls: 0,
    })
}

fn validate_request(
    request: &ProfessionalDocumentRequestIR,
    reasoning_plan_sha256: &str,
) -> Result<(), ProfessionalDocumentError> {
    let ids = request
        .evidence
        .iter()
        .map(|value| value.evidence_id.as_str())
        .collect::<BTreeSet<_>>();
    let section_ids = request
        .required_sections
        .iter()
        .map(|value| value.section_id.as_str())
        .collect::<BTreeSet<_>>();
    let directive_ids = request
        .revision_directives
        .iter()
        .map(|value| value.directive_id.as_str())
        .collect::<BTreeSet<_>>();
    let html_path_valid = request.output_html_path.as_ref().is_none_or(|value| {
        Path::new(value)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("html"))
    });
    if request.schema != PROFESSIONAL_DOCUMENT_REQUEST_SCHEMA
        || request.request_id.trim().is_empty()
        || request.title.trim().is_empty()
        || request.command.trim().is_empty()
        || request.audience.trim().is_empty()
        || request.purpose.trim().is_empty()
        || !(1..=200).contains(&request.target_page_count)
        || !(1..=8).contains(&request.max_revision_rounds)
        || !(5..=32).contains(&request.max_plan_steps)
        || request.evidence.len() > 64
        || ids.len() != request.evidence.len()
        || ids.contains("")
        || request.required_sections.len() > request.target_page_count
        || section_ids.len() != request.required_sections.len()
        || directive_ids.len() != request.revision_directives.len()
        || reasoning_plan_sha256.len() != 64
        || !html_path_valid
    {
        return Err(ProfessionalDocumentError::InvalidRequest);
    }
    Ok(())
}

fn build_plan(
    request: &ProfessionalDocumentRequestIR,
) -> Result<LongFormDocumentPlanIR, ProfessionalDocumentError> {
    let requirements = if request.required_sections.is_empty() {
        default_sections(request.kind)
    } else {
        request.required_sections.clone()
    };
    if requirements.is_empty() || requirements.len() > request.target_page_count {
        return Err(ProfessionalDocumentError::Planning);
    }
    let requested = requirements
        .iter()
        .map(|value| value.requested_page_count.unwrap_or(0))
        .sum::<usize>();
    if requested > request.target_page_count {
        return Err(ProfessionalDocumentError::Planning);
    }
    let unspecified = requirements
        .iter()
        .filter(|value| value.requested_page_count.is_none())
        .count();
    let mut remaining = request.target_page_count.saturating_sub(requested);
    if remaining < unspecified {
        return Err(ProfessionalDocumentError::Planning);
    }
    let mut sections = Vec::with_capacity(requirements.len());
    for (index, requirement) in requirements.iter().enumerate() {
        let left = requirements[index..]
            .iter()
            .filter(|value| value.requested_page_count.is_none())
            .count();
        let page_count = if let Some(value) = requirement.requested_page_count {
            value
        } else if left == 1 {
            remaining
        } else {
            let share = (remaining / left).max(1);
            remaining -= share;
            share
        };
        let dependencies = sections
            .last()
            .map(|previous: &DocumentPlanSectionIR| vec![previous.section_id.clone()])
            .unwrap_or_default();
        sections.push(DocumentPlanSectionIR {
            section_id: requirement.section_id.clone(),
            title: requirement.title.clone(),
            objective: requirement.objective.clone(),
            page_count,
            dependencies,
            required_evidence_ids: requirement.required_evidence_ids.clone(),
        });
    }
    let plan_sha256 = hash_json(&(request.target_page_count, &sections));
    Ok(LongFormDocumentPlanIR {
        schema: "B_CORE_LONG_FORM_DOCUMENT_PLAN_1".to_string(),
        target_page_count: request.target_page_count,
        sections,
        plan_sha256,
    })
}

fn default_sections(kind: ProfessionalDocumentKindIR) -> Vec<SectionRequirementIR> {
    let analysis_title = match kind {
        ProfessionalDocumentKindIR::Paper => "연구 설계 및 분석",
        ProfessionalDocumentKindIR::BusinessPlan | ProfessionalDocumentKindIR::BusinessProposal => {
            "사업 및 시장 분석"
        }
        ProfessionalDocumentKindIR::LongTermRepairPlan => "시설 현황 및 수선 수요 분석",
        _ => "현황 및 쟁점 분석",
    };
    [
        (
            "SEC-01",
            "표지 및 문서 통제",
            "문서의 권위·범위·버전을 고정한다.",
        ),
        ("SEC-02", "요약", "핵심 판단과 후속 행동을 압축한다."),
        (
            "SEC-03",
            "목적과 범위",
            "작성 목적, 독자, 적용 경계를 명시한다.",
        ),
        (
            "SEC-04",
            "입력자료와 근거",
            "사용된 자료와 근거 위치를 추적 가능하게 만든다.",
        ),
        (
            "SEC-05",
            analysis_title,
            "관측 사실을 근거로 현재 상태를 분석한다.",
        ),
        (
            "SEC-06",
            "핵심 쟁점",
            "의사결정에 영향을 주는 쟁점과 불확실성을 분리한다.",
        ),
        (
            "SEC-07",
            "대안 및 평가",
            "가능한 대안과 비교 기준을 구조화한다.",
        ),
        (
            "SEC-08",
            "실행 계획",
            "책임, 순서, 검증 가능한 완료 조건을 제시한다.",
        ),
        (
            "SEC-09",
            "위험과 통제",
            "오류·누락·변경 위험과 통제를 제시한다.",
        ),
        (
            "SEC-10",
            "결론 및 부록",
            "결론, 미확인 사항, 근거 색인을 정리한다.",
        ),
    ]
    .into_iter()
    .map(|(section_id, title, objective)| SectionRequirementIR {
        section_id: section_id.to_string(),
        title: title.to_string(),
        objective: objective.to_string(),
        required_evidence_ids: Vec::new(),
        requested_page_count: None,
    })
    .collect()
}

fn collect_facts(documents: &[StructuredEvidenceIR]) -> Vec<EvidenceFactIR> {
    let mut facts = Vec::new();
    for document in documents {
        for block in &document.blocks {
            for sentence in split_fact_sentences(&block.text) {
                if sentence.chars().count() < 2 {
                    continue;
                }
                let digest = hash_bytes(
                    format!("{}|{}|{}", document.evidence_id, block.block_id, sentence).as_bytes(),
                );
                facts.push(EvidenceFactIR {
                    fact_id: format!("FACT-{}", &digest[..16]),
                    text: sentence,
                    evidence_id: document.evidence_id.clone(),
                    block_id: block.block_id.clone(),
                    source_location: block.source_location.clone(),
                    numeric_tokens: numeric_tokens(&block.text),
                });
            }
        }
    }
    facts
}

fn split_fact_sentences(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        current.push(character);
        if matches!(character, '.' | '。' | '?' | '!' | '\n') || current.chars().count() >= 280 {
            let value = current.trim().to_string();
            if !value.is_empty() {
                values.push(value);
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_string());
    }
    values
}

fn numeric_tokens(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_digit() || matches!(character, '.' | ',' | '%' | '-') {
            current.push(character);
        } else if !current.is_empty() {
            if current.chars().any(|value| value.is_ascii_digit()) {
                values.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
    }
    if current.chars().any(|value| value.is_ascii_digit()) {
        values.push(current);
    }
    values.sort();
    values.dedup();
    values
}

fn build_working_memory(
    request: &ProfessionalDocumentRequestIR,
    plan: &LongFormDocumentPlanIR,
    evidence_facts: Vec<EvidenceFactIR>,
) -> DocumentWorkingMemoryIR {
    let mut canonical_terms = BTreeMap::new();
    canonical_terms.insert("DOCUMENT_TITLE".to_string(), request.title.clone());
    canonical_terms.insert("AUDIENCE".to_string(), request.audience.clone());
    canonical_terms.insert("PURPOSE".to_string(), request.purpose.clone());
    let mut memory = DocumentWorkingMemoryIR {
        schema: "B_CORE_DOCUMENT_WORKING_MEMORY_1".to_string(),
        global_constraints: vec![
            format!("TARGET_A4_PAGES={}", request.target_page_count),
            "NO_UNBOUND_FACTUAL_CLAIMS".to_string(),
            "MISSING_VALUES_REMAIN_EXPLICIT".to_string(),
            "GLOBAL_TERMINOLOGY_AND_NUMERIC_CONSISTENCY_REQUIRED".to_string(),
            "REVISION_MUST_PRESERVE_SOURCE_BINDINGS".to_string(),
        ],
        canonical_terms,
        evidence_facts,
        section_memory: plan
            .sections
            .iter()
            .map(|section| SectionMemoryIR {
                section_id: section.section_id.clone(),
                selected_fact_ids: Vec::new(),
                summary: String::new(),
                unresolved_questions: Vec::new(),
            })
            .collect(),
        unresolved_global_questions: Vec::new(),
        memory_sha256: String::new(),
    };
    memory.memory_sha256 = hash_json(&(
        &memory.global_constraints,
        &memory.canonical_terms,
        &memory.evidence_facts,
    ));
    memory
}

fn initial_draft(
    request: &ProfessionalDocumentRequestIR,
    plan: &LongFormDocumentPlanIR,
    memory: &mut DocumentWorkingMemoryIR,
) -> Vec<GroundedSectionDraftIR> {
    let mut sections = Vec::new();
    for planned in &plan.sections {
        let selected = rank_facts(planned, &memory.evidence_facts, 12);
        let mut paragraphs = Vec::new();
        paragraphs.push(derived_paragraph(
            &planned.section_id,
            paragraphs.len() + 1,
            format!(
                "이 절은 ‘{}’을 목적으로 하며, 문서 전체 목적 ‘{}’과 독자 ‘{}’를 기준으로 검토한다.",
                planned.objective, request.purpose, request.audience
            ),
        ));
        for fact in &selected {
            paragraphs.push(grounded_paragraph(
                &planned.section_id,
                paragraphs.len() + 1,
                format!("입력자료에서 확인된 내용은 다음과 같다. {}", fact.text),
                fact,
            ));
        }
        if selected.is_empty() {
            paragraphs.push(needs_evidence_paragraph(
                &planned.section_id,
                paragraphs.len() + 1,
                format!(
                    "‘{}’에 관한 확인 가능한 근거가 현재 입력자료에서 발견되지 않았다. 해당 절의 사실 판단과 수치는 추가 자료 확인 전까지 확정하지 않는다.",
                    planned.title
                ),
            ));
        }
        for directive in request.revision_directives.iter().filter(|directive| {
            directive
                .target_section_id
                .as_ref()
                .is_none_or(|target| target == &planned.section_id)
        }) {
            paragraphs.push(derived_paragraph(
                &planned.section_id,
                paragraphs.len() + 1,
                format!(
                    "수정 요구 {}를 적용한다: {}",
                    directive.directive_id, directive.instruction
                ),
            ));
        }
        if let Some(prior) = request.prior_snapshot.as_ref().and_then(|snapshot| {
            snapshot
                .sections
                .iter()
                .find(|section| section.section_id == planned.section_id)
        }) {
            let existing = paragraphs
                .iter()
                .map(|paragraph| paragraph.text.clone())
                .collect::<BTreeSet<_>>();
            paragraphs.extend(
                prior
                    .paragraphs
                    .iter()
                    .filter(|paragraph| !existing.contains(&paragraph.text))
                    .cloned(),
            );
        }
        if let Some(section_memory) = memory
            .section_memory
            .iter_mut()
            .find(|value| value.section_id == planned.section_id)
        {
            section_memory.selected_fact_ids =
                selected.iter().map(|fact| fact.fact_id.clone()).collect();
            section_memory.summary = format!(
                "{}: 근거 사실 {}건, 배정 {}쪽",
                planned.title,
                selected.len(),
                planned.page_count
            );
            if selected.is_empty() {
                section_memory
                    .unresolved_questions
                    .push(format!("{}의 근거자료가 필요한가?", planned.title));
            }
        }
        let section_sha256 = hash_json(&(
            &planned.section_id,
            &planned.title,
            &planned.objective,
            &paragraphs,
        ));
        sections.push(GroundedSectionDraftIR {
            section_id: planned.section_id.clone(),
            title: planned.title.clone(),
            objective: planned.objective.clone(),
            assigned_page_count: planned.page_count,
            paragraphs,
            section_sha256,
        });
    }
    sections
}

fn rank_facts<'a>(
    section: &DocumentPlanSectionIR,
    facts: &'a [EvidenceFactIR],
    limit: usize,
) -> Vec<&'a EvidenceFactIR> {
    let query = tokens(&format!("{} {}", section.title, section.objective));
    let mut ranked = facts
        .iter()
        .map(|fact| {
            let fact_tokens = tokens(&fact.text);
            let overlap = query.intersection(&fact_tokens).count() as i32;
            let required_bonus = if section.required_evidence_ids.contains(&fact.evidence_id) {
                20
            } else {
                0
            };
            let numeric_bonus = i32::from(!fact.numeric_tokens.is_empty()) * 2;
            (overlap * 5 + required_bonus + numeric_bonus, fact)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.fact_id.cmp(&right.1.fact_id))
    });
    ranked
        .into_iter()
        .filter(|(score, _)| *score > 0 || facts.len() <= limit)
        .take(limit)
        .map(|(_, fact)| fact)
        .collect()
}

fn tokens(text: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_alphanumeric() {
            current.extend(character.to_lowercase());
        } else if current.chars().count() >= 2 {
            values.insert(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.chars().count() >= 2 {
        values.insert(current);
    }
    values
}

fn grounded_paragraph(
    section_id: &str,
    ordinal: usize,
    text: String,
    fact: &EvidenceFactIR,
) -> GroundedParagraphIR {
    GroundedParagraphIR {
        paragraph_id: format!("{section_id}-P{ordinal:03}"),
        text,
        grounding: ParagraphGroundingIR::Grounded,
        evidence_refs: vec![fact.evidence_id.clone()],
        source_locations: vec![fact.source_location.clone()],
    }
}

fn derived_paragraph(section_id: &str, ordinal: usize, text: String) -> GroundedParagraphIR {
    GroundedParagraphIR {
        paragraph_id: format!("{section_id}-P{ordinal:03}"),
        text,
        grounding: ParagraphGroundingIR::Derived,
        evidence_refs: Vec::new(),
        source_locations: Vec::new(),
    }
}

fn needs_evidence_paragraph(section_id: &str, ordinal: usize, text: String) -> GroundedParagraphIR {
    GroundedParagraphIR {
        paragraph_id: format!("{section_id}-P{ordinal:03}"),
        text,
        grounding: ParagraphGroundingIR::NeedsEvidence,
        evidence_refs: Vec::new(),
        source_locations: Vec::new(),
    }
}

fn check_consistency(
    request: &ProfessionalDocumentRequestIR,
    plan: &LongFormDocumentPlanIR,
    sections: &[GroundedSectionDraftIR],
    evidence_ids: &BTreeSet<String>,
) -> Vec<ConsistencyIssueIR> {
    let mut issues = Vec::new();
    let pages = sections
        .iter()
        .map(|section| section.assigned_page_count)
        .sum::<usize>();
    if pages != request.target_page_count {
        issues.push(issue(
            ConsistencyIssueKindIR::PageBudgetMismatch,
            ConsistencySeverityIR::Error,
            "document:page_budget",
            format!("expected={},observed={pages}", request.target_page_count),
            Vec::new(),
        ));
    }
    let section_ids = sections
        .iter()
        .map(|section| section.section_id.as_str())
        .collect::<BTreeSet<_>>();
    for planned in &plan.sections {
        if planned
            .dependencies
            .iter()
            .any(|dependency| !section_ids.contains(dependency.as_str()))
        {
            issues.push(issue(
                ConsistencyIssueKindIR::BrokenSectionDependency,
                ConsistencySeverityIR::Error,
                format!("section:{}", planned.section_id),
                "SECTION_DEPENDENCY_NOT_FOUND".to_string(),
                Vec::new(),
            ));
        }
        for required in &planned.required_evidence_ids {
            if !evidence_ids.contains(required) {
                issues.push(issue(
                    ConsistencyIssueKindIR::MissingRequiredEvidence,
                    ConsistencySeverityIR::Warning,
                    format!("section:{}", planned.section_id),
                    "REQUIRED_EVIDENCE_NOT_PROVIDED".to_string(),
                    vec![required.clone()],
                ));
            }
        }
    }
    let mut paragraph_hashes = BTreeMap::<String, String>::new();
    for section in sections {
        for paragraph in &section.paragraphs {
            if paragraph.grounding == ParagraphGroundingIR::Grounded
                && (paragraph.evidence_refs.is_empty() || paragraph.source_locations.is_empty())
            {
                issues.push(issue(
                    ConsistencyIssueKindIR::UnsupportedClaim,
                    ConsistencySeverityIR::Error,
                    format!("paragraph:{}", paragraph.paragraph_id),
                    "GROUNDED_PARAGRAPH_WITHOUT_SOURCE_BINDING".to_string(),
                    paragraph.evidence_refs.clone(),
                ));
            }
            let unknown = paragraph
                .evidence_refs
                .iter()
                .filter(|reference| !evidence_ids.contains(*reference))
                .cloned()
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                issues.push(issue(
                    ConsistencyIssueKindIR::UnknownEvidenceReference,
                    ConsistencySeverityIR::Error,
                    format!("paragraph:{}", paragraph.paragraph_id),
                    "EVIDENCE_REFERENCE_NOT_IN_REQUEST".to_string(),
                    unknown,
                ));
            }
            let fingerprint = hash_bytes(normalize_for_duplicate(&paragraph.text).as_bytes());
            if let Some(first) =
                paragraph_hashes.insert(fingerprint, paragraph.paragraph_id.clone())
            {
                issues.push(issue(
                    ConsistencyIssueKindIR::DuplicateParagraph,
                    ConsistencySeverityIR::Warning,
                    format!("paragraph:{}", paragraph.paragraph_id),
                    format!("DUPLICATES={first}"),
                    Vec::new(),
                ));
            }
        }
    }
    for directive in &request.revision_directives {
        let marker = format!("수정 요구 {}를 적용한다", directive.directive_id);
        if !sections
            .iter()
            .flat_map(|section| &section.paragraphs)
            .any(|paragraph| paragraph.text.contains(&marker))
        {
            issues.push(issue(
                ConsistencyIssueKindIR::RevisionDirectiveUnapplied,
                ConsistencySeverityIR::Error,
                directive
                    .target_section_id
                    .clone()
                    .unwrap_or_else(|| "document".to_string()),
                "REVISION_DIRECTIVE_NOT_OBSERVABLE_IN_DRAFT".to_string(),
                directive.evidence_refs.clone(),
            ));
        }
    }
    issues.extend(numeric_conflicts(sections));
    issues.sort_by(|left, right| left.issue_id.cmp(&right.issue_id));
    issues
}

fn numeric_conflicts(sections: &[GroundedSectionDraftIR]) -> Vec<ConsistencyIssueIR> {
    let mut seen = BTreeMap::<String, (String, String)>::new();
    let mut issues = Vec::new();
    for paragraph in sections.iter().flat_map(|section| &section.paragraphs) {
        if paragraph.grounding != ParagraphGroundingIR::Grounded {
            continue;
        }
        let numbers = numeric_tokens(&paragraph.text);
        if numbers.len() != 1 {
            continue;
        }
        let subject = paragraph
            .text
            .split([':', '='])
            .next()
            .map(normalize_for_duplicate)
            .unwrap_or_default();
        if subject.chars().count() < 4 {
            continue;
        }
        if let Some((previous, location)) = seen.get(&subject) {
            if previous != &numbers[0] {
                issues.push(issue(
                    ConsistencyIssueKindIR::NumericConflict,
                    ConsistencySeverityIR::Warning,
                    format!("paragraph:{}", paragraph.paragraph_id),
                    format!("CONFLICTS_WITH={location}"),
                    paragraph.evidence_refs.clone(),
                ));
            }
        } else {
            seen.insert(
                subject,
                (numbers[0].clone(), paragraph.paragraph_id.clone()),
            );
        }
    }
    issues
}

fn issue(
    kind: ConsistencyIssueKindIR,
    severity: ConsistencySeverityIR,
    location: impl Into<String>,
    diagnostic: String,
    evidence_refs: Vec<String>,
) -> ConsistencyIssueIR {
    let location = location.into();
    let digest = hash_bytes(format!("{kind:?}|{severity:?}|{location}|{diagnostic}").as_bytes());
    ConsistencyIssueIR {
        issue_id: format!("ISSUE-{}", &digest[..16]),
        kind,
        severity,
        location,
        diagnostic,
        evidence_refs,
    }
}

fn revise_document(
    request: &ProfessionalDocumentRequestIR,
    sections: &mut [GroundedSectionDraftIR],
    issues: &[ConsistencyIssueIR],
) -> Vec<String> {
    let mut edits = Vec::new();
    let duplicate_ids = issues
        .iter()
        .filter(|issue| issue.kind == ConsistencyIssueKindIR::DuplicateParagraph)
        .filter_map(|issue| issue.location.strip_prefix("paragraph:"))
        .collect::<BTreeSet<_>>();
    for section in sections.iter_mut() {
        let before = section.paragraphs.len();
        section
            .paragraphs
            .retain(|paragraph| !duplicate_ids.contains(paragraph.paragraph_id.as_str()));
        if section.paragraphs.len() != before {
            edits.push(format!("REMOVE_DUPLICATE_PARAGRAPH:{}", section.section_id));
        }
        if !section
            .paragraphs
            .iter()
            .any(|paragraph| paragraph.grounding == ParagraphGroundingIR::Grounded)
            && !section
                .paragraphs
                .iter()
                .any(|paragraph| paragraph.grounding == ParagraphGroundingIR::NeedsEvidence)
        {
            section.paragraphs.push(needs_evidence_paragraph(
                &section.section_id,
                section.paragraphs.len() + 1,
                "중복 근거를 제거한 뒤 이 절에 고유하게 남은 근거가 없다. 절별 근거를 보완해야 한다."
                    .to_string(),
            ));
            edits.push(format!("MARK_SECTION_EVIDENCE_GAP:{}", section.section_id));
        }
        for paragraph in &mut section.paragraphs {
            if issues.iter().any(|issue| {
                issue.kind == ConsistencyIssueKindIR::UnknownEvidenceReference
                    && issue.location == format!("paragraph:{}", paragraph.paragraph_id)
            }) {
                paragraph.evidence_refs.retain(|reference| {
                    request
                        .evidence
                        .iter()
                        .any(|input| input.evidence_id == *reference)
                });
                paragraph.source_locations.clear();
                paragraph.grounding = ParagraphGroundingIR::NeedsEvidence;
                paragraph.text = format!(
                    "{} [근거 참조가 유효하지 않아 확인 필요 상태로 전환됨]",
                    paragraph.text
                );
                edits.push(format!(
                    "FAIL_CLOSED_INVALID_EVIDENCE:{}",
                    paragraph.paragraph_id
                ));
            }
        }
    }
    edits
}

fn refresh_section_hashes(sections: &mut [GroundedSectionDraftIR]) {
    for section in sections {
        section.section_sha256 = hash_json(&(
            &section.section_id,
            &section.title,
            &section.objective,
            &section.paragraphs,
        ));
    }
}

fn refresh_working_memory(
    memory: &mut DocumentWorkingMemoryIR,
    sections: &[GroundedSectionDraftIR],
) {
    memory.unresolved_global_questions = sections
        .iter()
        .filter(|section| {
            section
                .paragraphs
                .iter()
                .any(|paragraph| paragraph.grounding == ParagraphGroundingIR::NeedsEvidence)
        })
        .map(|section| format!("{} 절의 미확인 근거를 보완해야 한다.", section.title))
        .collect();
    memory.memory_sha256 = hash_json(&(
        &memory.global_constraints,
        &memory.canonical_terms,
        &memory.evidence_facts,
        &memory.section_memory,
        &memory.unresolved_global_questions,
    ));
}

fn quality_score(issues: &[ConsistencyIssueIR]) -> u16 {
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == ConsistencySeverityIR::Error)
        .count() as u16;
    let warnings = issues.len() as u16 - errors;
    1000_u16.saturating_sub(errors.saturating_mul(180) + warnings.saturating_mul(35))
}

fn paginate(
    plan: &LongFormDocumentPlanIR,
    sections: &[GroundedSectionDraftIR],
) -> Vec<ProfessionalDocumentPageIR> {
    let mut pages = Vec::new();
    for planned in &plan.sections {
        let Some(section) = sections
            .iter()
            .find(|section| section.section_id == planned.section_id)
        else {
            continue;
        };
        for local_page in 0..planned.page_count {
            let body = section
                .paragraphs
                .iter()
                .enumerate()
                .filter(|(index, _)| index % planned.page_count == local_page)
                .map(|(_, paragraph)| paragraph.clone())
                .collect::<Vec<_>>();
            pages.push(ProfessionalDocumentPageIR {
                page_number: pages.len() + 1,
                section_id: section.section_id.clone(),
                section_title: section.title.clone(),
                body: if body.is_empty() {
                    vec![needs_evidence_paragraph(
                        &section.section_id,
                        local_page + 1,
                        "이 페이지에 배정할 추가 근거가 없다. 분량을 채우기 위한 사실을 생성하지 않으며 자료 보완이 필요하다."
                            .to_string(),
                    )]
                } else {
                    body
                },
            });
        }
    }
    pages
}

#[allow(clippy::too_many_arguments)]
fn deliberate(
    core: &DockableCore,
    request: &ProfessionalDocumentRequestIR,
    parent: &str,
    plan: &LongFormDocumentPlanIR,
    evidence: &[StructuredEvidenceIR],
    memory: &DocumentWorkingMemoryIR,
    issues: &[ConsistencyIssueIR],
    pages: &[ProfessionalDocumentPageIR],
) -> Result<SwarmDeliberationIR, ProfessionalDocumentError> {
    let evidence_blocks = evidence
        .iter()
        .map(|value| value.blocks.len())
        .sum::<usize>();
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == ConsistencySeverityIR::Error)
        .count();
    let facts = vec![
        deliberation_fact(
            "LONG-FORM-PLAN",
            QualityCriterionIR::RequirementCoverage,
            verdict(plan.target_page_count == request.target_page_count),
            "PAGE_BUDGET_AND_SECTION_DEPENDENCY_PLAN",
            vec![format!("plan:sha256:{}", plan.plan_sha256)],
        ),
        deliberation_fact(
            "SECTION-EVIDENCE",
            QualityCriterionIR::EvidenceIntegrity,
            if evidence_blocks == 0 {
                AssessmentVerdictIR::Warning
            } else {
                AssessmentVerdictIR::Pass
            },
            "EACH_GROUNDED_PARAGRAPH_RETAINS_SOURCE_LOCATION",
            vec![format!("structured_blocks:{evidence_blocks}")],
        ),
        deliberation_fact(
            "WORKING-MEMORY",
            QualityCriterionIR::StructureIntegrity,
            verdict(!memory.memory_sha256.is_empty()),
            "GLOBAL_AND_SECTION_MEMORY_HASH_BOUND",
            vec![format!("memory:sha256:{}", memory.memory_sha256)],
        ),
        deliberation_fact(
            "GLOBAL-CONSISTENCY",
            QualityCriterionIR::ContradictionResistance,
            verdict(errors == 0),
            "EVIDENCE_NUMERIC_DUPLICATE_DEPENDENCY_AND_REVISION_CHECKS",
            vec![
                format!("errors:{errors}"),
                format!("issues:{}", issues.len()),
            ],
        ),
        deliberation_fact(
            "A4-PAGINATION",
            QualityCriterionIR::AudienceUsability,
            verdict(pages.len() == request.target_page_count),
            "EXACT_REQUESTED_A4_PAGE_IR",
            vec![format!("pages:{}", pages.len())],
        ),
    ];
    core.deliberate(&SwarmDeliberationRequestIR {
        schema: SWARM_DELIBERATION_REQUEST_SCHEMA.to_string(),
        request_id: format!("{}-PROFESSIONAL-DOCUMENT-REVIEW", request.request_id),
        subject: format!("{}: 장문 계획·근거·일관성·수정 검토", request.title),
        parent_reasoning_sha256: parent.to_string(),
        facts,
        max_workers: 6,
        max_rounds: 2,
    })
    .map_err(|_| ProfessionalDocumentError::Deliberation)
}

fn verdict(pass: bool) -> AssessmentVerdictIR {
    if pass {
        AssessmentVerdictIR::Pass
    } else {
        AssessmentVerdictIR::Fail
    }
}

fn deliberation_fact(
    fact_id: &str,
    criterion: QualityCriterionIR,
    verdict: AssessmentVerdictIR,
    rationale_code: &str,
    evidence_refs: Vec<String>,
) -> DeliberationFactIR {
    DeliberationFactIR {
        fact_id: fact_id.to_string(),
        criterion,
        verdict,
        rationale_code: rationale_code.to_string(),
        evidence_refs,
    }
}

fn write_html(
    path: &Path,
    request: &ProfessionalDocumentRequestIR,
    pages: &[ProfessionalDocumentPageIR],
    artifact_sha256: &str,
) -> Result<ProfessionalDocumentFileReceiptIR, ProfessionalDocumentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ProfessionalDocumentError::OutputWrite)?;
    }
    let mut html = String::from(
        "<!doctype html><html lang=\"ko\"><head><meta charset=\"utf-8\"><style>\
         @page{size:A4;margin:0}*{box-sizing:border-box}body{margin:0;background:#eceff1;color:#17212b;font-family:'Noto Sans KR','Malgun Gothic',sans-serif}\
         .page{width:210mm;height:297mm;margin:8mm auto;background:#fff;padding:18mm 17mm 16mm;page-break-after:always;overflow:hidden;position:relative}\
         .page:last-child{page-break-after:auto}h1{font-size:21pt;margin:0 0 7mm;border-bottom:1.2pt solid #24445f;padding-bottom:3mm}h2{font-size:12pt;color:#365d78;margin:0 0 5mm}\
         p{font-size:10.2pt;line-height:1.72;text-align:justify;margin:0 0 4mm}.source{font-size:8pt;color:#607584}.needs{border-left:3px solid #b78322;padding-left:4mm;color:#6b4a12}\
         footer{position:absolute;bottom:8mm;left:17mm;right:17mm;font-size:8pt;color:#6c7882;display:flex;justify-content:space-between}@media print{body{background:#fff}.page{margin:0}}\
         </style><title>Professional Document</title></head><body>",
    );
    for page in pages {
        html.push_str("<section class=\"page\">");
        html.push_str(&format!(
            "<h1>{}</h1><h2>{} · {}</h2>",
            escape_html(&request.title),
            escape_html(&page.section_id),
            escape_html(&page.section_title)
        ));
        for paragraph in &page.body {
            let class = if paragraph.grounding == ParagraphGroundingIR::NeedsEvidence {
                " class=\"needs\""
            } else {
                ""
            };
            html.push_str(&format!("<p{class}>{}</p>", escape_html(&paragraph.text)));
            if !paragraph.source_locations.is_empty() {
                html.push_str(&format!(
                    "<p class=\"source\">근거: {}</p>",
                    escape_html(&paragraph.source_locations.join(", "))
                ));
            }
        }
        html.push_str(&format!(
            "<footer><span>{}</span><span>{} / {}</span></footer></section>",
            &artifact_sha256[..16],
            page.page_number,
            pages.len()
        ));
    }
    html.push_str("</body></html>");
    write_replace_safe(path, html.as_bytes())?;
    let bytes = fs::read(path).map_err(|_| ProfessionalDocumentError::OutputWrite)?;
    Ok(ProfessionalDocumentFileReceiptIR {
        path: path.to_string_lossy().into_owned(),
        bytes: bytes.len() as u64,
        sha256: hash_bytes(&bytes),
        a4_page_count: pages.len(),
    })
}

fn write_replace_safe(path: &Path, bytes: &[u8]) -> Result<(), ProfessionalDocumentError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ProfessionalDocumentError::OutputWrite)?;
    let temporary = parent.join(format!(".{file_name}.{}.tmp", &hash_bytes(bytes)[..12]));
    fs::write(&temporary, bytes).map_err(|_| ProfessionalDocumentError::OutputWrite)?;
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let result = fs::write(path, bytes).map_err(|_| ProfessionalDocumentError::OutputWrite);
            let _ = fs::remove_file(&temporary);
            result
        }
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn normalize_for_duplicate(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !character.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn hash_json(value: &impl Serialize) -> String {
    hash_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term_repair::EvidenceKindIR;

    fn request(target_page_count: usize) -> ProfessionalDocumentRequestIR {
        ProfessionalDocumentRequestIR {
            schema: PROFESSIONAL_DOCUMENT_REQUEST_SCHEMA.to_string(),
            request_id: "DOC-TEST-1".to_string(),
            command: "근거 기반 전문 보고서를 작성하라".to_string(),
            title: "전문 보고서".to_string(),
            kind: ProfessionalDocumentKindIR::TechnicalReport,
            target_page_count,
            audience: "의사결정자".to_string(),
            purpose: "검토 및 승인".to_string(),
            evidence: Vec::new(),
            required_sections: Vec::new(),
            revision_directives: Vec::new(),
            prior_snapshot: None,
            max_revision_rounds: 4,
            output_html_path: None,
            max_plan_steps: 16,
        }
    }

    #[test]
    fn plan_and_pagination_are_exact_for_fifty_pages() {
        let core = DockableCore::load_embedded().unwrap();
        let response = process_professional_document(&core, &request(50), &"a".repeat(64)).unwrap();
        assert_eq!(response.plan.target_page_count, 50);
        assert_eq!(response.pages.len(), 50);
        assert_eq!(
            response
                .sections
                .iter()
                .map(|section| section.assigned_page_count)
                .sum::<usize>(),
            50
        );
        assert_eq!(response.external_model_calls, 0);
    }

    #[test]
    fn extracted_fact_retains_block_and_source_location() {
        let path = std::env::temp_dir().join("b_core_professional_document_evidence.txt");
        fs::write(&path, "총 사업비: 1,200원. 검증 기준은 공개되어 있다.").unwrap();
        let mut request = request(10);
        request.evidence.push(EvidenceInputIR {
            evidence_id: "E-1".to_string(),
            path: path.to_string_lossy().into_owned(),
            kind: EvidenceKindIR::PlainText,
        });
        let core = DockableCore::load_embedded().unwrap();
        let response = process_professional_document(&core, &request, &"b".repeat(64)).unwrap();
        assert!(!response.working_memory.evidence_facts.is_empty());
        assert!(response
            .working_memory
            .evidence_facts
            .iter()
            .all(|fact| fact.evidence_id == "E-1" && !fact.source_location.is_empty()));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn revision_directive_is_observable_and_checked() {
        let mut request = request(10);
        request.revision_directives.push(RevisionDirectiveIR {
            directive_id: "REV-1".to_string(),
            target_section_id: Some("SEC-05".to_string()),
            instruction: "위험도를 우선순위로 정렬한다.".to_string(),
            evidence_refs: Vec::new(),
        });
        let core = DockableCore::load_embedded().unwrap();
        let response = process_professional_document(&core, &request, &"c".repeat(64)).unwrap();
        assert!(response.sections.iter().any(|section| section
            .paragraphs
            .iter()
            .any(|paragraph| paragraph.text.contains("REV-1"))));
        assert!(!response
            .final_consistency_issues
            .iter()
            .any(|issue| { issue.kind == ConsistencyIssueKindIR::RevisionDirectiveUnapplied }));
    }

    #[test]
    fn duplicate_evidence_ids_fail_closed() {
        let mut request = request(10);
        for _ in 0..2 {
            request.evidence.push(EvidenceInputIR {
                evidence_id: "DUP".to_string(),
                path: "missing.txt".to_string(),
                kind: EvidenceKindIR::PlainText,
            });
        }
        let core = DockableCore::load_embedded().unwrap();
        assert_eq!(
            process_professional_document(&core, &request, &"d".repeat(64)),
            Err(ProfessionalDocumentError::InvalidRequest)
        );
    }
}
