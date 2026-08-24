//! Deterministic natural-language understanding for repository repair issues.
//!
//! This module extracts evidence-collection goals from English and Korean issue
//! prose. It deliberately does not generate source, select a patch, or treat an
//! issue claim as an executed observation. The downstream repository planner
//! still requires a reproduced failure and inspected source before it can
//! become ready.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::repository_coding_knowledge::{
    classify_public_diagnostic, plan_repository_repair, DiagnosticFamily, EvidenceKind,
    RepairPlanDisposition, RepositoryLanguage, RepositoryObservationIR, RepositoryTaskIR,
    REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA,
};

pub const REPOSITORY_ISSUE_UNDERSTANDING_SCHEMA: &str = "B_REPOSITORY_ISSUE_UNDERSTANDING_1";
pub const MAX_ISSUE_BYTES: usize = 64 * 1024;
pub const MAX_ISSUE_CLAIMS: usize = 256;
pub const MAX_ISSUE_TARGET_SYMBOLS: usize = 32;
pub const MAX_ISSUE_AMBIGUITIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueLanguage {
    English,
    Korean,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueClaimKind {
    ActualBehavior,
    ExpectedBehavior,
    ReproductionStep,
    CompatibilityConstraint,
    ProhibitedChange,
    VerificationRequirement,
    EnvironmentCondition,
    ScopeHint,
    Uncertainty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequirementStrength {
    Observed,
    Requested,
    Required,
    Forbidden,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClauseRelation {
    Standalone,
    Conditional,
    Contrastive,
    Temporal,
    Alternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueUnderstandingDisposition {
    ReadyForEvidenceCollection,
    NeedsClarification,
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueClaimIR {
    pub kind: IssueClaimKind,
    pub strength: RequirementStrength,
    pub relation: ClauseRelation,
    pub diagnostic_family: DiagnosticFamily,
    pub negated: bool,
    /// Content address of the source clause. Raw prose is intentionally not
    /// copied into the semantic repair substrate.
    pub clause_sha256: String,
    pub target_symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssueUnderstandingIR {
    pub schema: String,
    pub disposition: IssueUnderstandingDisposition,
    pub detected_language: IssueLanguage,
    pub source_sha256: String,
    pub claims: Vec<IssueClaimIR>,
    pub diagnostic_families: Vec<DiagnosticFamily>,
    pub target_symbols: Vec<String>,
    pub preserve_public_api: bool,
    pub preserve_data_compatibility: bool,
    /// `None` means the issue did not grant authority either way. The bridge
    /// to repair planning resolves that absence conservatively to `false`.
    pub allow_dependency_changes: Option<bool>,
    pub has_actual_behavior: bool,
    pub has_expected_behavior: bool,
    pub has_reproduction_steps: bool,
    pub missing_semantics: Vec<String>,
    pub ambiguity_reasons: Vec<String>,
    pub issue_text_to_patch_shortcut_events: u64,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIssueCanaryReceipt {
    pub schema: String,
    pub pass: bool,
    pub fresh_synthetic_issues: usize,
    pub structured_issues: usize,
    pub bilingual_equivalence_checks: usize,
    pub bilingual_equivalence_passes: usize,
    pub negation_scope_checks: usize,
    pub negation_scope_passes: usize,
    pub ambiguity_checks: usize,
    pub ambiguity_passes: usize,
    pub planner_fail_closed_checks: usize,
    pub planner_fail_closed_passes: usize,
    pub failed_case_ids: Vec<String>,
    pub issue_text_to_patch_shortcut_events: u64,
    pub task_identity_routing_events: u64,
    pub repository_identity_routing_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub official_benchmark_score_claimed: bool,
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn detect_language(text: &str) -> IssueLanguage {
    let korean = text
        .chars()
        .filter(|character| matches!(*character as u32, 0xAC00..=0xD7A3))
        .count();
    let english = text
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .count();
    match (korean, english) {
        (0, 0) => IssueLanguage::Unknown,
        (0, _) => IssueLanguage::English,
        (_, 0..=7) => IssueLanguage::Korean,
        _ => IssueLanguage::Mixed,
    }
}

fn heading_kind(heading: &str) -> Option<IssueClaimKind> {
    let lower = heading.trim().trim_matches('#').trim().to_ascii_lowercase();
    if contains_any(
        &lower,
        &[
            "actual behavior",
            "current behavior",
            "observed behavior",
            "현재 동작",
            "실제 동작",
            "현상",
        ],
    ) {
        Some(IssueClaimKind::ActualBehavior)
    } else if contains_any(
        &lower,
        &[
            "expected behavior",
            "desired behavior",
            "acceptance criteria",
            "기대 동작",
            "예상 동작",
            "완료 조건",
        ],
    ) {
        Some(IssueClaimKind::ExpectedBehavior)
    } else if contains_any(
        &lower,
        &[
            "steps to reproduce",
            "reproduction",
            "reproducer",
            "재현 단계",
            "재현 방법",
        ],
    ) {
        Some(IssueClaimKind::ReproductionStep)
    } else if contains_any(
        &lower,
        &[
            "compatibility",
            "backward compatibility",
            "constraints",
            "호환성",
            "제약 조건",
        ],
    ) {
        Some(IssueClaimKind::CompatibilityConstraint)
    } else if contains_any(
        &lower,
        &[
            "tests",
            "verification",
            "validation",
            "테스트",
            "검증",
            "회귀",
        ],
    ) {
        Some(IssueClaimKind::VerificationRequirement)
    } else if contains_any(&lower, &["environment", "versions", "환경", "버전"]) {
        Some(IssueClaimKind::EnvironmentCondition)
    } else {
        None
    }
}

fn trim_list_prefix(line: &str) -> &str {
    let trimmed = line.trim();
    let without_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
        .unwrap_or(trimmed);
    let digit_count = without_bullet
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digit_count > 0 {
        let remainder = &without_bullet[digit_count..];
        if let Some(remainder) = remainder
            .strip_prefix(". ")
            .or_else(|| remainder.strip_prefix(") "))
        {
            return remainder.trim();
        }
    }
    without_bullet.trim()
}

fn split_sentences(line: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut in_inline_code = false;
    let chars = line.char_indices().collect::<Vec<_>>();
    for (position, (index, character)) in chars.iter().enumerate() {
        if *character == '`' {
            in_inline_code = !in_inline_code;
            continue;
        }
        if in_inline_code || !matches!(character, '.' | '?' | '!' | ';') {
            continue;
        }
        let boundary = chars
            .get(position + 1)
            .is_none_or(|(_, next)| next.is_whitespace());
        if boundary {
            let end = index + character.len_utf8();
            let sentence = line[start..end].trim();
            if !sentence.is_empty() {
                output.push(sentence.to_string());
            }
            start = end;
        }
    }
    let tail = line[start..].trim();
    if !tail.is_empty() {
        output.push(tail.to_string());
    }
    output
}

pub(crate) fn issue_segments(text: &str) -> Vec<(Option<IssueClaimKind>, String)> {
    let mut output = Vec::new();
    let mut section = None;
    let mut in_code_fence = false;
    for raw_line in text.lines() {
        let raw_trimmed = raw_line.trim();
        if raw_trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        let line = trim_list_prefix(raw_line);
        if line.is_empty() {
            continue;
        }
        if in_code_fence {
            output.push((Some(IssueClaimKind::ActualBehavior), line.to_string()));
            continue;
        }
        let stripped_heading = line.trim_matches('#').trim();
        if (line.starts_with('#') || line.ends_with(':'))
            && heading_kind(stripped_heading.trim_end_matches(':')).is_some()
        {
            section = heading_kind(stripped_heading.trim_end_matches(':'));
            continue;
        }
        for sentence in split_sentences(line) {
            if let Some((heading, content)) = sentence.split_once(':') {
                if let Some(kind) = heading_kind(heading) {
                    section = Some(kind);
                    if !content.trim().is_empty() {
                        output.push((section, content.trim().to_string()));
                    }
                    continue;
                }
            }
            output.push((section, sentence));
        }
        if output.len() >= MAX_ISSUE_CLAIMS {
            break;
        }
    }
    output.truncate(MAX_ISSUE_CLAIMS);
    output
}

fn classify_korean_diagnostic(text: &str) -> DiagnosticFamily {
    if contains_any(text, &["데이터 경합", "경쟁 상태", "교착", "데드락"]) {
        DiagnosticFamily::RaceOrDeadlock
    } else if contains_any(
        text,
        &["시간 초과", "타임아웃", "응답하지", "무한 대기", "멈춥니다"],
    ) {
        DiagnosticFamily::TimeoutOrLiveness
    } else if contains_any(
        text,
        &["모듈을 찾", "모듈 오류", "임포트 오류", "가져오기 오류"],
    ) {
        DiagnosticFamily::ImportOrModule
    } else if contains_any(
        text,
        &["리소스 누수", "자원 누수", "닫힌 핸들", "파일 디스크립터"],
    ) {
        DiagnosticFamily::ResourceLifecycle
    } else if contains_any(
        text,
        &["스키마", "직렬화", "역직렬화", "프로토콜", "잘못된 json"],
    ) {
        DiagnosticFamily::ProtocolOrSchema
    } else if contains_any(
        text,
        &["비결정", "순서가 달라", "정렬되지", "순서가 불안정"],
    ) {
        DiagnosticFamily::OrderingOrDeterminism
    } else if contains_any(
        text,
        &["하위 호환", "api 호환", "시그니처 불일치", "속성 오류"],
    ) {
        DiagnosticFamily::ApiCompatibility
    } else if contains_any(text, &["성능 저하", "느려졌", "할당 증가", "처리량 감소"])
    {
        DiagnosticFamily::PerformanceRegression
    } else if contains_any(
        text,
        &["타입 불일치", "컴파일 오류", "빌드 오류", "빌드에 실패"],
    ) {
        DiagnosticFamily::CompilationOrType
    } else if contains_any(
        text,
        &["단언 실패", "검증값 불일치", "테스트가 실패", "예상값과 다"],
    ) {
        DiagnosticFamily::AssertionContract
    } else if contains_any(text, &["예외", "패닉", "치명적 오류"]) {
        DiagnosticFamily::ExceptionOrPanic
    } else {
        DiagnosticFamily::Unknown
    }
}

fn classify_issue_diagnostic(clause: &str) -> DiagnosticFamily {
    let lower = clause.to_ascii_lowercase();
    let family = if contains_any(&lower, &["times out", "timing out"]) {
        DiagnosticFamily::TimeoutOrLiveness
    } else {
        classify_public_diagnostic(clause)
    };
    if family == DiagnosticFamily::Unknown {
        classify_korean_diagnostic(&clause.to_ascii_lowercase())
    } else {
        family
    }
}

fn relation(lower: &str) -> ClauseRelation {
    if contains_any(
        lower,
        &[
            " if ",
            "when ",
            "unless ",
            "only when",
            " whenever ",
            " 경우",
            " 때",
            "라면",
            "이면",
        ],
    ) {
        ClauseRelation::Conditional
    } else if contains_any(
        lower,
        &[
            " instead",
            "however",
            " but ",
            "rather than",
            "대신",
            "하지만",
            "반면",
        ],
    ) {
        ClauseRelation::Contrastive
    } else if contains_any(
        lower,
        &[
            " after ",
            " before ",
            " then ",
            " afterwards",
            " 뒤",
            " 이후",
            " 전에",
            " 다음",
        ],
    ) {
        ClauseRelation::Temporal
    } else if contains_any(
        lower,
        &[" either ", " or ", "one of", " 또는 ", "이거나", "혹은"],
    ) {
        ClauseRelation::Alternative
    } else {
        ClauseRelation::Standalone
    }
}

fn change_is_forbidden(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "do not change",
            "don't change",
            "must not change",
            "should not change",
            "without changing",
            "do not modify",
            "must not modify",
            "without modifying",
            "no new dependencies",
            "do not add dependencies",
            "must not add dependencies",
            "변경하지 마",
            "변경하면 안",
            "수정하지 마",
            "수정하면 안",
            "새 의존성을 추가하지",
            "의존성 추가 금지",
        ],
    )
}

fn negative_behavior(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "should not",
            "must not",
            "should never",
            "must never",
            "does not",
            "do not fail",
            "without panicking",
            "without failing",
            "하지 않아야",
            "발생하지 않아야",
            "실패하면 안",
            "패닉 없이",
        ],
    )
}

fn classify_claim(context: Option<IssueClaimKind>, clause: &str) -> IssueClaimKind {
    let lower = clause.to_ascii_lowercase();
    if change_is_forbidden(&lower) {
        return IssueClaimKind::ProhibitedChange;
    }
    if contains_any(
        &lower,
        &[
            "steps to reproduce",
            "reproduce by",
            "run the following",
            "재현하려면",
            "다음 명령을 실행",
        ],
    ) {
        IssueClaimKind::ReproductionStep
    } else if contains_any(
        &lower,
        &[
            "currently",
            "actual behavior",
            "observed",
            "fails",
            "failure",
            "timed out",
            "timeout",
            "panics",
            "throws",
            "raises",
            "returns instead",
            "instead returns",
            "현재",
            "실패합니다",
            "실패한다",
            "오류가 발생",
            "예외가 발생",
            "패닉이 발생",
            "대신 반환",
        ],
    ) {
        IssueClaimKind::ActualBehavior
    } else if contains_any(
        &lower,
        &[
            "backward compatible",
            "backwards compatible",
            "preserve the public api",
            "keep the public api",
            "existing callers",
            "existing data format",
            "하위 호환",
            "공개 api를 유지",
            "기존 호출자",
            "기존 데이터 형식",
        ],
    ) {
        IssueClaimKind::CompatibilityConstraint
    } else if contains_any(
        &lower,
        &[
            "add a regression test",
            "add regression tests",
            "tests must",
            "test must",
            "verify that",
            "verification must",
            "회귀 테스트",
            "테스트를 추가",
            "검증해야",
        ],
    ) {
        IssueClaimKind::VerificationRequirement
    } else if contains_any(
        &lower,
        &[
            "should",
            "must",
            "expected",
            "needs to",
            "need to",
            "ensure",
            "해야 합니다",
            "해야 한다",
            "어야 합니다",
            "아야 합니다",
            "어야 한다",
            "아야 한다",
            "되어야",
            "기대 동작",
            "예상 동작",
        ],
    ) {
        IssueClaimKind::ExpectedBehavior
    } else if contains_any(
        &lower,
        &[
            "environment",
            "version ",
            "rustc ",
            "operating system",
            "platform",
            "환경",
            "버전",
        ],
    ) {
        IssueClaimKind::EnvironmentCondition
    } else if contains_any(
        &lower,
        &[
            "maybe",
            "might",
            "possibly",
            "unclear",
            "not sure",
            "suspect",
            "아마",
            "가능성",
            "불확실",
            "모르겠",
        ],
    ) || lower.ends_with('?')
    {
        IssueClaimKind::Uncertainty
    } else if let Some(context) = context {
        context
    } else {
        IssueClaimKind::ScopeHint
    }
}

fn strength(kind: IssueClaimKind, lower: &str) -> RequirementStrength {
    match kind {
        IssueClaimKind::ActualBehavior
        | IssueClaimKind::ReproductionStep
        | IssueClaimKind::EnvironmentCondition
        | IssueClaimKind::ScopeHint => RequirementStrength::Observed,
        IssueClaimKind::ProhibitedChange => RequirementStrength::Forbidden,
        IssueClaimKind::Uncertainty => RequirementStrength::Uncertain,
        IssueClaimKind::ExpectedBehavior
        | IssueClaimKind::CompatibilityConstraint
        | IssueClaimKind::VerificationRequirement => {
            if contains_any(
                lower,
                &[
                    "must",
                    "required",
                    "해야",
                    "어야 합니다",
                    "아야 합니다",
                    "어야 한다",
                    "아야 한다",
                    "필수",
                    "반드시",
                ],
            ) {
                RequirementStrength::Required
            } else {
                RequirementStrength::Requested
            }
        }
    }
}

fn clean_target(token: &str) -> Option<String> {
    let cleaned = token
        .trim()
        .trim_matches(|character: char| {
            matches!(
                character,
                '`' | '\'' | '"' | ',' | ';' | ':' | '[' | ']' | '{' | '}'
            )
        })
        .trim_end_matches(['.', '?', '!'])
        .trim();
    if cleaned.is_empty()
        || cleaned.len() > 256
        || cleaned.chars().any(char::is_whitespace)
        || !cleaned
            .chars()
            .any(|character| character.is_alphabetic() || character == '_')
        || cleaned.chars().any(|character| {
            !(character.is_alphanumeric()
                || matches!(
                    character,
                    '_' | '-' | '.' | ':' | '/' | '\\' | '(' | ')' | '<' | '>'
                ))
        })
    {
        None
    } else {
        Some(cleaned.to_string())
    }
}

pub(crate) fn extract_targets(clause: &str) -> Vec<String> {
    let mut targets = BTreeSet::new();
    let mut remainder = clause;
    while let Some(start) = remainder.find('`') {
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        if let Some(target) = clean_target(&after_start[..end]) {
            targets.insert(target);
        }
        remainder = &after_start[end + 1..];
    }
    for token in clause.split_whitespace() {
        let cleaned = token.trim_matches(|character: char| {
            matches!(
                character,
                ',' | ';' | ':' | '[' | ']' | '{' | '}' | '\'' | '"'
            )
        });
        let lower = cleaned.to_ascii_lowercase();
        let path_like = cleaned.contains('/')
            || cleaned.contains('\\')
            || [
                ".rs", ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".toml", ".json",
            ]
            .iter()
            .any(|extension| lower.ends_with(extension));
        let function_like = cleaned.ends_with("()") && cleaned.len() > 2;
        if (path_like || function_like) && clean_target(cleaned).is_some() {
            targets.insert(clean_target(cleaned).expect("target was checked"));
        }
    }
    targets.into_iter().take(MAX_ISSUE_TARGET_SYMBOLS).collect()
}

fn claim_from_clause(context: Option<IssueClaimKind>, clause: &str) -> IssueClaimIR {
    let lower = clause.to_ascii_lowercase();
    let kind = classify_claim(context, clause);
    IssueClaimIR {
        kind,
        strength: strength(kind, &lower),
        relation: relation(&format!(" {lower} ")),
        diagnostic_family: classify_issue_diagnostic(clause),
        negated: change_is_forbidden(&lower) || negative_behavior(&lower),
        clause_sha256: sha256(clause.trim().as_bytes()),
        target_symbols: extract_targets(clause),
    }
}

fn has_unbound_reference(text: &str, targets: &[String]) -> bool {
    if !targets.is_empty() {
        return false;
    }
    let lower = format!(" {} ", text.to_ascii_lowercase());
    contains_any(
        &lower,
        &[
            " it ",
            " this ",
            " that ",
            " they ",
            " them ",
            " 그것",
            " 이것",
            " 해당 것",
            " 이를 ",
        ],
    )
}

/// Compile issue prose into a bounded semantic understanding record.
///
/// The record contains only typed claims, content hashes, and bounded target
/// hints. No source edit or solution template can be emitted by this stage.
pub fn understand_repository_issue(text: &str) -> RepositoryIssueUnderstandingIR {
    let trimmed = text.trim();
    let source_sha256 = sha256(trimmed.as_bytes());
    if trimmed.is_empty() || text.len() > MAX_ISSUE_BYTES {
        return RepositoryIssueUnderstandingIR {
            schema: REPOSITORY_ISSUE_UNDERSTANDING_SCHEMA.to_string(),
            disposition: IssueUnderstandingDisposition::InvalidInput,
            detected_language: detect_language(trimmed),
            source_sha256,
            claims: Vec::new(),
            diagnostic_families: Vec::new(),
            target_symbols: Vec::new(),
            preserve_public_api: false,
            preserve_data_compatibility: false,
            allow_dependency_changes: None,
            has_actual_behavior: false,
            has_expected_behavior: false,
            has_reproduction_steps: false,
            missing_semantics: vec![if trimmed.is_empty() {
                "NON_EMPTY_ISSUE_TEXT".to_string()
            } else {
                "BOUNDED_ISSUE_TEXT".to_string()
            }],
            ambiguity_reasons: Vec::new(),
            issue_text_to_patch_shortcut_events: 0,
            task_identity_routing_events: 0,
            repository_identity_routing_events: 0,
            external_llm_calls: 0,
            network_reads: 0,
        };
    }

    let claims = issue_segments(trimmed)
        .into_iter()
        .map(|(context, clause)| claim_from_clause(context, &clause))
        .collect::<Vec<_>>();
    let has_actual_behavior = claims
        .iter()
        .any(|claim| claim.kind == IssueClaimKind::ActualBehavior);
    let has_expected_behavior = claims.iter().any(|claim| {
        matches!(
            claim.kind,
            IssueClaimKind::ExpectedBehavior
                | IssueClaimKind::CompatibilityConstraint
                | IssueClaimKind::ProhibitedChange
        )
    });
    let has_reproduction_steps = claims
        .iter()
        .any(|claim| claim.kind == IssueClaimKind::ReproductionStep);
    let target_symbols = claims
        .iter()
        .flat_map(|claim| claim.target_symbols.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_ISSUE_TARGET_SYMBOLS)
        .collect::<Vec<_>>();
    let diagnostic_families = claims
        .iter()
        .map(|claim| claim.diagnostic_family)
        .filter(|family| *family != DiagnosticFamily::Unknown)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let lower = trimmed.to_ascii_lowercase();
    let preserve_public_api = contains_any(
        &lower,
        &[
            "public api",
            "backward compatible",
            "backwards compatible",
            "existing callers",
            "공개 api",
            "하위 호환",
            "기존 호출자",
        ],
    );
    let preserve_data_compatibility = contains_any(
        &lower,
        &[
            "data compatibility",
            "existing data format",
            "wire format",
            "serialized format",
            "데이터 호환",
            "기존 데이터 형식",
            "직렬화 형식",
        ],
    );
    let dependency_forbidden = contains_any(
        &lower,
        &[
            "no new dependencies",
            "do not add dependencies",
            "must not add dependencies",
            "without adding dependencies",
            "새 의존성을 추가하지",
            "의존성 추가 금지",
        ],
    );
    let dependency_allowed = contains_any(
        &lower,
        &[
            "dependency changes are allowed",
            "may add a dependency",
            "adding a dependency is acceptable",
            "의존성을 추가해도",
            "의존성 변경 허용",
        ],
    );
    let allow_dependency_changes = match (dependency_forbidden, dependency_allowed) {
        (true, false) => Some(false),
        (false, true) => Some(true),
        _ => None,
    };

    let mut missing_semantics = Vec::new();
    if !has_actual_behavior && !has_reproduction_steps {
        missing_semantics.push("ACTUAL_BEHAVIOR_OR_REPRODUCTION".to_string());
    }
    if !has_expected_behavior {
        missing_semantics.push("EXPECTED_BEHAVIOR_OR_CONSTRAINT".to_string());
    }
    let mut ambiguity_reasons = BTreeSet::new();
    if claims
        .iter()
        .any(|claim| claim.kind == IssueClaimKind::Uncertainty)
        || contains_any(
            &lower,
            &[
                "maybe",
                "might",
                "possibly",
                "unclear",
                "not sure",
                "suspect",
                "아마",
                "가능성",
                "불확실",
                "모르겠",
            ],
        )
    {
        ambiguity_reasons.insert("EXPLICIT_UNCERTAINTY".to_string());
    }
    if claims
        .iter()
        .any(|claim| claim.relation == ClauseRelation::Alternative)
    {
        ambiguity_reasons.insert("UNRESOLVED_ALTERNATIVE".to_string());
    }
    if has_unbound_reference(trimmed, &target_symbols) {
        ambiguity_reasons.insert("UNBOUND_REFERENCE".to_string());
    }
    if dependency_forbidden && dependency_allowed {
        ambiguity_reasons.insert("CONFLICTING_DEPENDENCY_AUTHORITY".to_string());
    }
    let ambiguity_reasons = ambiguity_reasons
        .into_iter()
        .take(MAX_ISSUE_AMBIGUITIES)
        .collect::<Vec<_>>();
    let disposition = if claims.is_empty() {
        IssueUnderstandingDisposition::InvalidInput
    } else if !missing_semantics.is_empty() || !ambiguity_reasons.is_empty() {
        IssueUnderstandingDisposition::NeedsClarification
    } else {
        IssueUnderstandingDisposition::ReadyForEvidenceCollection
    };

    RepositoryIssueUnderstandingIR {
        schema: REPOSITORY_ISSUE_UNDERSTANDING_SCHEMA.to_string(),
        disposition,
        detected_language: detect_language(trimmed),
        source_sha256,
        claims,
        diagnostic_families,
        target_symbols,
        preserve_public_api,
        preserve_data_compatibility,
        allow_dependency_changes,
        has_actual_behavior,
        has_expected_behavior,
        has_reproduction_steps,
        missing_semantics,
        ambiguity_reasons,
        issue_text_to_patch_shortcut_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
    }
}

/// Bridge typed issue claims into the evidence-bound repository task IR.
///
/// All emitted observations remain non-reproducible. Consequently issue text
/// alone can never satisfy the downstream repair planner's evidence quorum.
pub fn issue_to_repository_task(
    issue: &RepositoryIssueUnderstandingIR,
    language: RepositoryLanguage,
) -> RepositoryTaskIR {
    let observations = issue
        .claims
        .iter()
        .map(|claim| RepositoryObservationIR {
            kind: if matches!(
                claim.kind,
                IssueClaimKind::ExpectedBehavior
                    | IssueClaimKind::CompatibilityConstraint
                    | IssueClaimKind::ProhibitedChange
                    | IssueClaimKind::VerificationRequirement
            ) {
                EvidenceKind::PublicContract
            } else {
                EvidenceKind::IssueStatement
            },
            diagnostic_family: claim.diagnostic_family,
            evidence_sha256: claim.clause_sha256.clone(),
            target_symbols: claim.target_symbols.clone(),
            reproducible: false,
        })
        .collect();
    RepositoryTaskIR {
        schema: REPOSITORY_REPAIR_KNOWLEDGE_SCHEMA.to_string(),
        language,
        observations,
        preserve_public_api: issue.preserve_public_api,
        preserve_data_compatibility: issue.preserve_data_compatibility,
        allow_dependency_changes: issue.allow_dependency_changes == Some(true),
    }
}

fn structural_signature(
    issue: &RepositoryIssueUnderstandingIR,
) -> (
    BTreeSet<IssueClaimKind>,
    BTreeSet<DiagnosticFamily>,
    bool,
    bool,
    Option<bool>,
) {
    (
        issue.claims.iter().map(|claim| claim.kind).collect(),
        issue.diagnostic_families.iter().copied().collect(),
        issue.preserve_public_api,
        issue.preserve_data_compatibility,
        issue.allow_dependency_changes,
    )
}

/// Deterministic synthetic regression suite for issue understanding. It is not
/// an official SWE benchmark score and contains no benchmark task identities.
pub fn run_repository_issue_canary() -> RepositoryIssueCanaryReceipt {
    let bilingual_pairs = [
        (
            "Actual behavior: `poll()` times out when the queue is empty. Expected behavior: `poll()` must return an empty result. Preserve the public API.",
            "실제 동작: 큐가 비어 있을 때 `poll()`이 시간 초과됩니다. 기대 동작: `poll()`은 빈 결과를 반환해야 합니다. 공개 API를 유지해야 합니다.",
        ),
        (
            "Currently `encode()` emits an unstable order. It should produce deterministic output. Do not add dependencies.",
            "현재 `encode()`의 순서가 불안정합니다. `encode()`은 결정적인 출력을 만들어야 합니다. 새 의존성을 추가하지 마십시오.",
        ),
        (
            "Actual behavior: `parse()` panics on invalid JSON. Expected behavior: it must reject the input without panicking. Add a regression test.",
            "실제 동작: `parse()`은 잘못된 JSON에서 패닉이 발생합니다. 기대 동작: 패닉 없이 입력을 거부해야 합니다. 회귀 테스트를 추가해야 합니다.",
        ),
        (
            "Observed behavior: `cancel()` causes a resource leak after shutdown. It should close the handle and preserve the existing data format.",
            "실제 동작: 종료 이후 `cancel()`에서 리소스 누수가 발생합니다. 기대 동작: 핸들을 닫고 기존 데이터 형식을 유지해야 합니다.",
        ),
    ];
    let ambiguity_cases = [
        "It might use either cache or storage. It should be fixed.",
        "Actual behavior: `read()` fails. Expected behavior: maybe it should retry?",
        "현재 이것이 실패합니다. 아마 다른 방식이거나 기존 방식이어야 합니다.",
        "Actual behavior: `load()` fails. Dependency changes are allowed, but no new dependencies.",
    ];
    let composition_cases = [
        "Actual behavior: `a()` fails. Expected behavior: `a()` must succeed without changing the public API.",
        "Actual behavior: `b()` times out when empty. Expected behavior: `b()` should return before the deadline.",
        "Actual behavior: `c()` returns unstable order. Expected behavior: `c()` must remain deterministic.",
        "Actual behavior: `d()` leaks a resource after cancellation. Expected behavior: `d()` must close it.",
        "실제 동작: `e()`에서 컴파일 오류가 발생합니다. 기대 동작: `e()`은 빌드되어야 합니다.",
        "실제 동작: `f()`에서 스키마 역직렬화 오류가 발생합니다. 기대 동작: `f()`은 입력을 거부해야 합니다.",
        "실제 동작: `g()`의 성능 저하가 발생합니다. 기대 동작: `g()`은 기존 처리량을 유지해야 합니다.",
        "실제 동작: `h()`에서 데이터 경합이 발생합니다. 기대 동작: `h()`은 결정적으로 완료되어야 합니다.",
    ];

    let mut structured_issues = 0;
    let mut bilingual_equivalence_passes = 0;
    let mut negation_scope_checks = 0;
    let mut negation_scope_passes = 0;
    let mut planner_fail_closed_checks = 0;
    let mut planner_fail_closed_passes = 0;
    let mut failed_case_ids = Vec::new();

    for (pair_index, (english, korean)) in bilingual_pairs.iter().enumerate() {
        let english = understand_repository_issue(english);
        let korean = understand_repository_issue(korean);
        let english_ready =
            english.disposition == IssueUnderstandingDisposition::ReadyForEvidenceCollection;
        let korean_ready =
            korean.disposition == IssueUnderstandingDisposition::ReadyForEvidenceCollection;
        structured_issues += usize::from(english_ready);
        structured_issues += usize::from(korean_ready);
        if !english_ready {
            failed_case_ids.push(format!("BILINGUAL_{pair_index}_EN_STRUCTURE"));
        }
        if !korean_ready {
            failed_case_ids.push(format!("BILINGUAL_{pair_index}_KO_STRUCTURE"));
        }
        let equivalent = structural_signature(&english) == structural_signature(&korean);
        bilingual_equivalence_passes += usize::from(equivalent);
        if !equivalent {
            failed_case_ids.push(format!("BILINGUAL_{pair_index}_EQUIVALENCE"));
        }
        for issue in [&english, &korean] {
            negation_scope_checks += 1;
            let negation_valid = issue
                .claims
                .iter()
                .filter(|claim| claim.negated)
                .all(|claim| {
                    matches!(
                        claim.kind,
                        IssueClaimKind::ExpectedBehavior | IssueClaimKind::ProhibitedChange
                    )
                });
            negation_scope_passes += usize::from(negation_valid);
            let task = issue_to_repository_task(issue, RepositoryLanguage::Rust);
            let plan = plan_repository_repair(&task);
            planner_fail_closed_checks += 1;
            planner_fail_closed_passes += usize::from(
                plan.disposition == RepairPlanDisposition::CapabilityGap
                    && plan
                        .missing_evidence
                        .contains(&"REPRODUCIBLE_FAILURE".to_string())
                    && plan
                        .missing_evidence
                        .contains(&"SOURCE_OBSERVATION".to_string()),
            );
        }
    }
    for (case_index, issue) in composition_cases.iter().enumerate() {
        let issue = understand_repository_issue(issue);
        let ready = issue.disposition == IssueUnderstandingDisposition::ReadyForEvidenceCollection;
        structured_issues += usize::from(ready);
        if !ready {
            failed_case_ids.push(format!("COMPOSITION_{case_index}_STRUCTURE"));
        }
        let task = issue_to_repository_task(&issue, RepositoryLanguage::Rust);
        let plan = plan_repository_repair(&task);
        planner_fail_closed_checks += 1;
        planner_fail_closed_passes += usize::from(
            plan.disposition == RepairPlanDisposition::CapabilityGap
                && plan
                    .missing_evidence
                    .contains(&"REPRODUCIBLE_FAILURE".to_string())
                && plan
                    .missing_evidence
                    .contains(&"SOURCE_OBSERVATION".to_string()),
        );
    }
    let mut ambiguity_passes = 0;
    for issue in ambiguity_cases {
        let issue = understand_repository_issue(issue);
        ambiguity_passes += usize::from(
            issue.disposition == IssueUnderstandingDisposition::NeedsClarification
                && !issue.ambiguity_reasons.is_empty(),
        );
    }

    let fresh_synthetic_issues =
        bilingual_pairs.len() * 2 + ambiguity_cases.len() + composition_cases.len();
    let pass = structured_issues == bilingual_pairs.len() * 2 + composition_cases.len()
        && bilingual_equivalence_passes == bilingual_pairs.len()
        && negation_scope_passes == negation_scope_checks
        && ambiguity_passes == ambiguity_cases.len()
        && planner_fail_closed_passes == planner_fail_closed_checks;
    RepositoryIssueCanaryReceipt {
        schema: REPOSITORY_ISSUE_UNDERSTANDING_SCHEMA.to_string(),
        pass,
        fresh_synthetic_issues,
        structured_issues,
        bilingual_equivalence_checks: bilingual_pairs.len(),
        bilingual_equivalence_passes,
        negation_scope_checks,
        negation_scope_passes,
        ambiguity_checks: ambiguity_cases.len(),
        ambiguity_passes,
        planner_fail_closed_checks,
        planner_fail_closed_passes,
        failed_case_ids,
        issue_text_to_patch_shortcut_events: 0,
        task_identity_routing_events: 0,
        repository_identity_routing_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
        official_benchmark_score_claimed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_english_issue_extracts_contract_conditions_and_authority() {
        let issue = understand_repository_issue(
            "## Actual behavior\n`poll()` times out when the queue is empty.\n\
             ## Expected behavior\n`poll()` must return an empty result without changing the public API.\n\
             ## Tests\nAdd a regression test. No new dependencies.",
        );
        assert_eq!(
            issue.disposition,
            IssueUnderstandingDisposition::ReadyForEvidenceCollection
        );
        assert!(issue.has_actual_behavior);
        assert!(issue.has_expected_behavior);
        assert!(issue.preserve_public_api);
        assert_eq!(issue.allow_dependency_changes, Some(false));
        assert!(issue
            .diagnostic_families
            .contains(&DiagnosticFamily::TimeoutOrLiveness));
        assert!(issue.target_symbols.contains(&"poll()".to_string()));
        assert!(issue.claims.iter().any(|claim| {
            claim.relation == ClauseRelation::Conditional
                && claim.kind == IssueClaimKind::ActualBehavior
        }));
    }

    #[test]
    fn korean_issue_compiles_to_the_same_semantic_dimensions() {
        let english = understand_repository_issue(
            "Actual behavior: `encode()` emits an unstable order. Expected behavior: `encode()` must produce deterministic output. Preserve the public API.",
        );
        let korean = understand_repository_issue(
            "실제 동작: `encode()`의 순서가 불안정합니다. 기대 동작: `encode()`은 결정적인 출력을 만들어야 합니다. 공개 API를 유지해야 합니다.",
        );
        assert_eq!(
            structural_signature(&english),
            structural_signature(&korean)
        );
    }

    #[test]
    fn bilingual_dependency_constraint_preserves_the_same_structure() {
        let english = understand_repository_issue(
            "Currently `encode()` emits an unstable order. It should produce deterministic output. Do not add dependencies.",
        );
        let korean = understand_repository_issue(
            "현재 `encode()`의 순서가 불안정합니다. `encode()`은 결정적인 출력을 만들어야 합니다. 새 의존성을 추가하지 마십시오.",
        );
        assert_eq!(
            structural_signature(&english),
            structural_signature(&korean)
        );
    }

    #[test]
    fn behavior_negation_and_change_prohibition_remain_distinct() {
        let issue = understand_repository_issue(
            "Actual behavior: `parse()` panics. Expected behavior: `parse()` must not panic. Do not change the public API.",
        );
        assert!(issue
            .claims
            .iter()
            .any(|claim| { claim.kind == IssueClaimKind::ExpectedBehavior && claim.negated }));
        assert!(issue.claims.iter().any(|claim| {
            claim.kind == IssueClaimKind::ProhibitedChange
                && claim.strength == RequirementStrength::Forbidden
                && claim.negated
        }));
    }

    #[test]
    fn alternatives_uncertainty_and_unbound_references_fail_closed() {
        let issue = understand_repository_issue(
            "It might use either cache or storage. It should be fixed.",
        );
        assert_eq!(
            issue.disposition,
            IssueUnderstandingDisposition::NeedsClarification
        );
        assert!(issue
            .ambiguity_reasons
            .contains(&"EXPLICIT_UNCERTAINTY".to_string()));
        assert!(issue
            .ambiguity_reasons
            .contains(&"UNRESOLVED_ALTERNATIVE".to_string()));
        assert!(issue
            .ambiguity_reasons
            .contains(&"UNBOUND_REFERENCE".to_string()));
    }

    #[test]
    fn issue_claims_never_satisfy_executable_evidence_quorum() {
        let issue = understand_repository_issue(
            "Actual behavior: `load()` fails with a type mismatch. Expected behavior: `load()` must accept the documented value.",
        );
        let task = issue_to_repository_task(&issue, RepositoryLanguage::Rust);
        assert!(task
            .observations
            .iter()
            .all(|observation| !observation.reproducible));
        let plan = plan_repository_repair(&task);
        assert_eq!(plan.disposition, RepairPlanDisposition::CapabilityGap);
        assert!(plan
            .missing_evidence
            .contains(&"REPRODUCIBLE_FAILURE".to_string()));
        assert!(plan
            .missing_evidence
            .contains(&"SOURCE_OBSERVATION".to_string()));
    }

    #[test]
    fn oversized_and_empty_inputs_are_invalid_without_side_effects() {
        for text in [String::new(), "x".repeat(MAX_ISSUE_BYTES + 1)] {
            let issue = understand_repository_issue(&text);
            assert_eq!(
                issue.disposition,
                IssueUnderstandingDisposition::InvalidInput
            );
            assert_eq!(issue.external_llm_calls, 0);
            assert_eq!(issue.network_reads, 0);
            assert_eq!(issue.issue_text_to_patch_shortcut_events, 0);
        }
    }

    #[test]
    fn synthetic_issue_canary_preserves_bilingual_structure_and_safety() {
        let receipt = run_repository_issue_canary();
        assert!(receipt.pass, "{receipt:#?}");
        assert_eq!(receipt.fresh_synthetic_issues, 20);
        assert_eq!(receipt.bilingual_equivalence_checks, 4);
        assert_eq!(receipt.bilingual_equivalence_passes, 4);
        assert_eq!(receipt.ambiguity_passes, receipt.ambiguity_checks);
        assert_eq!(
            receipt.planner_fail_closed_passes,
            receipt.planner_fail_closed_checks
        );
        assert!(!receipt.official_benchmark_score_claimed);
    }
}
