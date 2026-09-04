//! Typed separation of plan state, language reports, verified execution, and results.
//!
//! This module derives an inspectable lifecycle view from the action ledger.
//! It never advances the ledger: language may choose which axis to discuss,
//! while only the existing typed host-evidence API may change execution state.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::action_state::{
    ActionExecutionStatusIR, ActionPlanStatusIR, ActionReportedStatusIR, ActionStateAnalysisIR,
    ActionStateLedgerIR, ActionStateRecordIR,
};
use crate::language_knowledge::LanguageCodeIR;

pub const PLAN_RESULT_BOUNDARY_SCHEMA: &str = "B_CORE_PLAN_RESULT_BOUNDARY_IR_2";
pub const ACTION_LIFECYCLE_SNAPSHOT_SCHEMA: &str = "B_CORE_ACTION_LIFECYCLE_SNAPSHOT_IR_1";
const MAX_LIFECYCLE_SNAPSHOTS: usize = 32;
const MAX_SELECTED_ACTIONS: usize = 32;
const MAX_BOUNDARY_AMBIGUITIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlanResultQueryFocusIR {
    None,
    PlanState,
    ReportedState,
    VerifiedExecution,
    VerifiedResult,
    PlanVersusResult,
    ReportedVersusResult,
    ExecutionVersusPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResultAvailabilityIR {
    Unavailable,
    Pending,
    VerifiedSuccess,
    VerifiedFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionLifecycleSnapshotIR {
    pub schema: String,
    pub action_id: String,
    pub subject: String,
    pub canonical_predicate: String,
    pub plan_status: ActionPlanStatusIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_status: Option<ActionReportedStatusIR>,
    pub execution_status: ActionExecutionStatusIR,
    pub result_availability: ResultAvailabilityIR,
    pub plan_only: bool,
    pub report_only: bool,
    pub verified_result: bool,
    pub execution_evidence_ids: Vec<String>,
    pub language_report_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_evidence_turn: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_language_report_turn: Option<u64>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub snapshot_sha256: String,
}

impl ActionLifecycleSnapshotIR {
    fn from_record(record: &ActionStateRecordIR, ledger: &ActionStateLedgerIR) -> Self {
        let language_reports = ledger
            .language_report_history
            .iter()
            .filter(|report| report.action_id == record.action_id)
            .collect::<Vec<_>>();
        let evidence = ledger
            .evidence_audit_history
            .iter()
            .filter(|audit| audit.action_id == record.action_id)
            .collect::<Vec<_>>();
        let result_availability = match record.execution_status {
            ActionExecutionStatusIR::NotObserved => ResultAvailabilityIR::Unavailable,
            ActionExecutionStatusIR::InProgress => ResultAvailabilityIR::Pending,
            ActionExecutionStatusIR::Succeeded => ResultAvailabilityIR::VerifiedSuccess,
            ActionExecutionStatusIR::Failed => ResultAvailabilityIR::VerifiedFailure,
        };
        let mut snapshot = Self {
            schema: ACTION_LIFECYCLE_SNAPSHOT_SCHEMA.to_string(),
            action_id: record.action_id.clone(),
            subject: record.subject.clone(),
            canonical_predicate: record.canonical_predicate.clone(),
            plan_status: record.plan_status,
            reported_status: record.reported_status,
            execution_status: record.execution_status,
            result_availability,
            plan_only: record.execution_status == ActionExecutionStatusIR::NotObserved
                && record.reported_status.is_none(),
            report_only: record.execution_status == ActionExecutionStatusIR::NotObserved
                && record.reported_status.is_some(),
            verified_result: matches!(
                record.execution_status,
                ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed
            ),
            execution_evidence_ids: record.execution_evidence_ids.clone(),
            language_report_ids: language_reports
                .iter()
                .map(|report| report.report_id.clone())
                .collect(),
            latest_evidence_turn: evidence.last().map(|audit| audit.accepted_turn),
            latest_language_report_turn: language_reports.last().map(|report| report.turn_index),
            semantic_authority: false,
            external_action_executed: false,
            snapshot_sha256: String::new(),
        };
        snapshot.snapshot_sha256 = snapshot_sha256(&snapshot);
        snapshot
    }

    pub fn validate(&self) -> bool {
        self.schema == ACTION_LIFECYCLE_SNAPSHOT_SCHEMA
            && !self.action_id.trim().is_empty()
            && !self.subject.trim().is_empty()
            && !self.canonical_predicate.trim().is_empty()
            && self.execution_evidence_ids.len() <= 8
            && self.language_report_ids.len() <= 8
            && self
                .execution_evidence_ids
                .iter()
                .all(|id| !id.trim().is_empty())
            && self
                .language_report_ids
                .iter()
                .all(|id| !id.trim().is_empty())
            && self.plan_only
                == (self.execution_status == ActionExecutionStatusIR::NotObserved
                    && self.reported_status.is_none())
            && self.report_only
                == (self.execution_status == ActionExecutionStatusIR::NotObserved
                    && self.reported_status.is_some())
            && self.verified_result
                == matches!(
                    self.execution_status,
                    ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed
                )
            && self.result_availability
                == match self.execution_status {
                    ActionExecutionStatusIR::NotObserved => ResultAvailabilityIR::Unavailable,
                    ActionExecutionStatusIR::InProgress => ResultAvailabilityIR::Pending,
                    ActionExecutionStatusIR::Succeeded => ResultAvailabilityIR::VerifiedSuccess,
                    ActionExecutionStatusIR::Failed => ResultAvailabilityIR::VerifiedFailure,
                }
            && !self.semantic_authority
            && !self.external_action_executed
            && valid_digest(&self.snapshot_sha256)
            && self.snapshot_sha256 == snapshot_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanResultBoundaryIR {
    pub schema: String,
    pub query_focus: PlanResultQueryFocusIR,
    pub source_text_sha256: String,
    pub ledger_sha256: String,
    pub selected_action_ids: Vec<String>,
    pub snapshots: Vec<ActionLifecycleSnapshotIR>,
    /// The query is well-formed, but the ledger contains no action record to
    /// inspect.  This is a known absence of evidence, not a reference
    /// ambiguity.
    #[serde(default)]
    pub no_action_lifecycle_record: bool,
    pub unresolved_ambiguities: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub boundary_sha256: String,
}

impl Default for PlanResultBoundaryIR {
    fn default() -> Self {
        let mut boundary = Self {
            schema: PLAN_RESULT_BOUNDARY_SCHEMA.to_string(),
            query_focus: PlanResultQueryFocusIR::None,
            source_text_sha256: text_sha256(""),
            ledger_sha256: content_sha256(&ActionStateLedgerIR::default()),
            selected_action_ids: Vec::new(),
            snapshots: Vec::new(),
            no_action_lifecycle_record: false,
            unresolved_ambiguities: Vec::new(),
            semantic_authority: false,
            external_action_executed: false,
            boundary_sha256: String::new(),
        };
        boundary.boundary_sha256 = boundary_sha256(&boundary);
        boundary
    }
}

impl PlanResultBoundaryIR {
    pub fn validate(&self) -> bool {
        let snapshot_ids = self
            .snapshots
            .iter()
            .map(|snapshot| snapshot.action_id.as_str())
            .collect::<BTreeSet<_>>();
        let selected_ids = self
            .selected_action_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.schema == PLAN_RESULT_BOUNDARY_SCHEMA
            && valid_digest(&self.source_text_sha256)
            && valid_digest(&self.ledger_sha256)
            && self.snapshots.len() <= MAX_LIFECYCLE_SNAPSHOTS
            && snapshot_ids.len() == self.snapshots.len()
            && self
                .snapshots
                .iter()
                .all(ActionLifecycleSnapshotIR::validate)
            && self.selected_action_ids.len() <= MAX_SELECTED_ACTIONS
            && selected_ids.len() == self.selected_action_ids.len()
            && selected_ids.is_subset(&snapshot_ids)
            && (!self.no_action_lifecycle_record
                || (self.query_focus != PlanResultQueryFocusIR::None
                    && self.selected_action_ids.is_empty()
                    && self.snapshots.is_empty()))
            && self.unresolved_ambiguities.len() <= MAX_BOUNDARY_AMBIGUITIES
            && self
                .unresolved_ambiguities
                .iter()
                .all(|ambiguity| !ambiguity.trim().is_empty())
            && (self.query_focus == PlanResultQueryFocusIR::None
                || !self.selected_action_ids.is_empty()
                || self.no_action_lifecycle_record
                || !self.unresolved_ambiguities.is_empty())
            && !self.semantic_authority
            && !self.external_action_executed
            && valid_digest(&self.boundary_sha256)
            && self.boundary_sha256 == boundary_sha256(self)
    }

    pub fn validate_against(
        &self,
        source_text: &str,
        analysis: &ActionStateAnalysisIR,
        ledger: &ActionStateLedgerIR,
    ) -> bool {
        self.validate() && self == &build_plan_result_boundary(source_text, analysis, ledger)
    }

    pub fn has_lifecycle_query(&self) -> bool {
        self.query_focus != PlanResultQueryFocusIR::None
    }

    pub fn realize(&self, language: LanguageCodeIR) -> Option<String> {
        self.has_lifecycle_query().then(|| {
            if self.no_action_lifecycle_record {
                return match language {
                    LanguageCodeIR::Korean => {
                        "연결된 실행 기록이 없어 성공이나 실패를 판정할 근거가 없어.".to_string()
                    }
                    _ => "There is no linked execution record, so there is no basis for deciding success or failure.".to_string(),
                };
            }
            self.selected_action_ids
                .iter()
                .filter_map(|action_id| {
                    self.snapshots
                        .iter()
                        .find(|snapshot| &snapshot.action_id == action_id)
                })
                .map(|snapshot| realize_snapshot(snapshot, self.query_focus, language))
                .collect::<Vec<_>>()
                .join(" ")
        })
    }
}

pub fn build_plan_result_boundary(
    source_text: &str,
    analysis: &ActionStateAnalysisIR,
    ledger: &ActionStateLedgerIR,
) -> PlanResultBoundaryIR {
    let query_focus = classify_plan_result_query_focus(source_text);
    let snapshots = ledger
        .records
        .iter()
        .take(MAX_LIFECYCLE_SNAPSHOTS)
        .map(|record| ActionLifecycleSnapshotIR::from_record(record, ledger))
        .collect::<Vec<_>>();
    let snapshot_ids = snapshots
        .iter()
        .map(|snapshot| snapshot.action_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut selected_action_ids = if query_focus == PlanResultQueryFocusIR::None {
        Vec::new()
    } else if selects_action_set(source_text) {
        snapshots
            .iter()
            .map(|snapshot| snapshot.action_id.clone())
            .collect()
    } else {
        analysis
            .target_action_ids
            .iter()
            .filter(|id| snapshot_ids.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>()
    };
    if query_focus != PlanResultQueryFocusIR::None && selected_action_ids.is_empty() {
        if let Some(record) = ledger.current_record() {
            selected_action_ids.push(record.action_id.clone());
        }
    }
    selected_action_ids.sort();
    selected_action_ids.dedup();
    selected_action_ids.truncate(MAX_SELECTED_ACTIONS);

    let no_action_lifecycle_record = query_focus != PlanResultQueryFocusIR::None
        && selected_action_ids.is_empty()
        && snapshots.is_empty();
    let mut unresolved_ambiguities = analysis.unresolved_ambiguities.clone();
    unresolved_ambiguities.sort();
    unresolved_ambiguities.dedup();
    unresolved_ambiguities.truncate(MAX_BOUNDARY_AMBIGUITIES);

    let mut boundary = PlanResultBoundaryIR {
        schema: PLAN_RESULT_BOUNDARY_SCHEMA.to_string(),
        query_focus,
        source_text_sha256: text_sha256(source_text),
        ledger_sha256: content_sha256(ledger),
        selected_action_ids,
        snapshots,
        no_action_lifecycle_record,
        unresolved_ambiguities,
        semantic_authority: false,
        external_action_executed: false,
        boundary_sha256: String::new(),
    };
    boundary.boundary_sha256 = boundary_sha256(&boundary);
    debug_assert!(boundary.validate());
    boundary
}

pub fn boundary_sha256(boundary: &PlanResultBoundaryIR) -> String {
    let mut canonical = boundary.clone();
    canonical.boundary_sha256.clear();
    content_sha256(&canonical)
}

fn snapshot_sha256(snapshot: &ActionLifecycleSnapshotIR) -> String {
    let mut canonical = snapshot.clone();
    canonical.snapshot_sha256.clear();
    content_sha256(&canonical)
}

pub fn classify_plan_result_query_focus(text: &str) -> PlanResultQueryFocusIR {
    let lower = text.to_lowercase();
    let trimmed = lower.trim_start();
    let response_directive = lower
        .split(|character: char| !character.is_ascii_alphabetic())
        .any(|token| {
            matches!(
                token,
                "say" | "state" | "separate" | "distinguish" | "answer"
            )
        })
        && contains_any(
            &lower,
            &[
                "evidence",
                "verified",
                "fact",
                "established",
                "result",
                "outcome",
            ],
        );
    let lifecycle_query_surface = lower.trim_end().ends_with('?')
        || [
            "what ",
            "which ",
            "did ",
            "does ",
            "do ",
            "is ",
            "are ",
            "was ",
            "were ",
            "has ",
            "have ",
            "tell me ",
            "explain ",
            "say ",
            "state ",
            "separate ",
            "distinguish ",
        ]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
        || response_directive
        || contains_any(
            &lower,
            &[
                "알려줘",
                "알려 줘",
                "말해줘",
                "말해 줘",
                "말해",
                "설명해",
                "답해",
                "구분해",
                "분리해",
                "뭐야",
                "뭐지",
                "건가",
                "거야",
            ],
        );
    if !lifecycle_query_surface {
        return PlanResultQueryFocusIR::None;
    }
    let report = contains_any(
        &lower,
        &[
            "보고", "주장", "얘기", "말과", "말뿐", "claim", "report", "said", "says",
        ],
    );
    let plan = contains_any(
        &lower,
        &[
            "계획",
            "예정",
            "앞으로",
            "하겠",
            "말만",
            "plan",
            "planned",
            "roadmap",
            "intention",
            "intend",
            "going to",
        ],
    );
    let verified = contains_any(
        &lower,
        &["검증", "확인", "verified", "receipt", "evidence", "fact"],
    );
    let result = contains_any(
        &lower,
        &[
            "결과",
            "끝난",
            "끝났",
            "성공",
            "실패",
            "완료",
            "수리했",
            "수리된",
            "고쳤",
            "result",
            "finished",
            "finish",
            "done",
            "completed",
            "complete",
            "repaired",
            "fixed",
            "succeeded",
            "failed",
            "completion",
            "outcome",
            "produced",
        ],
    );
    let execution = contains_any(
        &lower,
        &[
            "실행",
            "돌아간",
            "일어난",
            "진행 상태",
            "execution",
            "executed",
            "actually run",
            "ran",
            "happened",
            "actual state",
        ],
    );
    let correction_prefaced_actual = (trimmed.starts_with("actually,")
        || trimmed.starts_with("actually ")
        || trimmed.starts_with("실제로는")
        || trimmed.starts_with("실은"))
        && contains_any(&lower, &["instead", "rather than", "말고"]);
    let actual = contains_any(&lower, &["실제", "실제로", "actual", "actually"])
        && !correction_prefaced_actual;
    let completed_action_question = lower.trim_start().starts_with("did you ")
        && contains_any(
            &lower,
            &[
                "inspect",
                "investigate",
                "check",
                "review",
                "repair",
                "fix",
                "delete",
                "remove",
                "run",
                "execute",
                "apply",
                "find",
            ],
        );
    let contrast = contains_any(
        &lower,
        &[
            "말고",
            "아니라",
            "rather than",
            "not the",
            "do not tell",
            "don't tell",
        ],
    );
    let plan_explicitly_excluded = contains_any(
        &lower,
        &[
            "계획이 아니라",
            "계획 말고 검증",
            "rather than the plan",
            "not the plan;",
            "not the plan,",
            "not the plan",
        ],
    );

    if completed_action_question {
        PlanResultQueryFocusIR::VerifiedResult
    } else if report && (verified || result || actual) {
        PlanResultQueryFocusIR::ReportedVersusResult
    } else if plan_explicitly_excluded && result && execution {
        PlanResultQueryFocusIR::ExecutionVersusPlan
    } else if plan_explicitly_excluded && result {
        PlanResultQueryFocusIR::VerifiedResult
    } else if plan && (result || verified || ((execution || actual) && !contrast)) {
        PlanResultQueryFocusIR::PlanVersusResult
    } else if (plan || contrast) && (execution || actual) {
        PlanResultQueryFocusIR::ExecutionVersusPlan
    } else if result || verified {
        PlanResultQueryFocusIR::VerifiedResult
    } else if execution || actual {
        PlanResultQueryFocusIR::VerifiedExecution
    } else if plan {
        PlanResultQueryFocusIR::PlanState
    } else if report {
        PlanResultQueryFocusIR::ReportedState
    } else {
        PlanResultQueryFocusIR::None
    }
}

fn selects_action_set(text: &str) -> bool {
    let lower = text.to_lowercase();
    contains_any(
        &lower,
        &[
            "두 계획",
            "둘 중",
            "두 작업",
            "two plans",
            "two tasks",
            "those two",
            "which of",
        ],
    )
}

fn realize_snapshot(
    snapshot: &ActionLifecycleSnapshotIR,
    focus: PlanResultQueryFocusIR,
    language: LanguageCodeIR,
) -> String {
    let plan = match (language, snapshot.plan_status) {
        (LanguageCodeIR::Korean, ActionPlanStatusIR::Active) => "활성 계획",
        (LanguageCodeIR::Korean, ActionPlanStatusIR::Superseded) => "대체된 계획",
        (LanguageCodeIR::Korean, ActionPlanStatusIR::Withdrawn) => "철회된 계획",
        (_, ActionPlanStatusIR::Active) => "an active plan",
        (_, ActionPlanStatusIR::Superseded) => "a superseded plan",
        (_, ActionPlanStatusIR::Withdrawn) => "a withdrawn plan",
    };
    let execution = match (language, snapshot.execution_status) {
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::NotObserved) => "검증된 실행 관측 없음",
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::InProgress) => "호스트 검증 기준 실행 중",
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::Succeeded) => "호스트 검증 기준 성공",
        (LanguageCodeIR::Korean, ActionExecutionStatusIR::Failed) => "호스트 검증 기준 실패",
        (_, ActionExecutionStatusIR::NotObserved) => "no verified execution observation",
        (_, ActionExecutionStatusIR::InProgress) => "host-verified execution in progress",
        (_, ActionExecutionStatusIR::Succeeded) => "host-verified success",
        (_, ActionExecutionStatusIR::Failed) => "host-verified failure",
    };
    let result = match (language, snapshot.result_availability) {
        (LanguageCodeIR::Korean, ResultAvailabilityIR::Unavailable) => "검증된 실행 결과 없음",
        (LanguageCodeIR::Korean, ResultAvailabilityIR::Pending) => {
            "실행 중이며 종결 결과는 아직 없음"
        }
        (LanguageCodeIR::Korean, ResultAvailabilityIR::VerifiedSuccess) => "검증된 성공 결과",
        (LanguageCodeIR::Korean, ResultAvailabilityIR::VerifiedFailure) => "검증된 실패 결과",
        (_, ResultAvailabilityIR::Unavailable) => "no verified execution result",
        (_, ResultAvailabilityIR::Pending) => {
            "execution is in progress with no terminal result yet"
        }
        (_, ResultAvailabilityIR::VerifiedSuccess) => "a verified successful result",
        (_, ResultAvailabilityIR::VerifiedFailure) => "a verified failed result",
    };
    let report = match (language, snapshot.reported_status) {
        (LanguageCodeIR::Korean, None) => "사용자 결과 보고 없음".to_string(),
        (LanguageCodeIR::Korean, Some(status)) => format!("사용자 보고 {status:?}"),
        (_, None) => "no user outcome report".to_string(),
        (_, Some(status)) => format!("user-reported {status:?}"),
    };
    match (language, focus) {
        (LanguageCodeIR::Korean, PlanResultQueryFocusIR::ReportedVersusResult) => format!(
            "‘{}’은 {report} 상태지만, 별개로 {result} 상태야.",
            snapshot.subject
        ),
        (LanguageCodeIR::Korean, PlanResultQueryFocusIR::VerifiedResult) => {
            format!("‘{}’의 실제 결과 축은 {result}이야.", snapshot.subject)
        }
        (LanguageCodeIR::Korean, PlanResultQueryFocusIR::ExecutionVersusPlan) => format!(
            "‘{}’은 {plan}이고, 계획과 분리된 실제 실행 축은 {execution}이야.",
            snapshot.subject
        ),
        (LanguageCodeIR::Korean, _) => format!(
            "‘{}’은 {plan}이고, 실제 실행은 {execution}, 결과는 {result}이야.",
            snapshot.subject
        ),
        (_, PlanResultQueryFocusIR::ReportedVersusResult) => format!(
            "For {}, the state is {report}; separately, there is {result}.",
            snapshot.subject
        ),
        (_, PlanResultQueryFocusIR::VerifiedResult) => {
            format!(
                "The actual result axis for {} has {result}.",
                snapshot.subject
            )
        }
        (_, PlanResultQueryFocusIR::ExecutionVersusPlan) => format!(
            "For {}, the plan is {plan}; the separate execution axis has {execution}.",
            snapshot.subject
        ),
        (_, _) => format!(
            "For {}, the plan is {plan}, execution has {execution}, and there is {result}.",
            snapshot.subject
        ),
    }
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn content_sha256<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable plan/result boundary");
    format!("{:x}", Sha256::digest(bytes))
}

fn text_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_state::ActionPlanSeedIR;

    fn ledger() -> ActionStateLedgerIR {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[ActionPlanSeedIR {
            action_id: "GOAL-000001-01".to_string(),
            goal_id: "GOAL-000001-01".to_string(),
            canonical_predicate: "EXECUTE".to_string(),
            predicate_surface: "실행".to_string(),
            subject: "배포".to_string(),
            source_semantic_text: "배포를 실행해".to_string(),
            introduced_turn: 1,
            external_execution_authorized: true,
        }]);
        ledger
    }

    #[test]
    fn bilingual_contrast_queries_select_distinct_axes() {
        assert_eq!(
            classify_plan_result_query_focus("계획 말고 실제 실행 상태는 뭐야?"),
            PlanResultQueryFocusIR::ExecutionVersusPlan
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Is that only a success report, or is there a verified result?",
            ),
            PlanResultQueryFocusIR::ReportedVersusResult
        );
        assert_eq!(
            classify_plan_result_query_focus("Did that finish, or is it only planned?"),
            PlanResultQueryFocusIR::PlanVersusResult
        );
        assert_eq!(
            classify_plan_result_query_focus("What was the verified result rather than the plan?"),
            PlanResultQueryFocusIR::VerifiedResult
        );
        assert_eq!(
            classify_plan_result_query_focus("말만 잡혀 있는 거야, 실제로 돌아간 거야?"),
            PlanResultQueryFocusIR::PlanVersusResult
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Is that merely on the roadmap, or did anything actually run?",
            ),
            PlanResultQueryFocusIR::PlanVersusResult
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Beyond the intention to do it, what fact was verified?",
            ),
            PlanResultQueryFocusIR::PlanVersusResult
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Do not tell me what you plan to do; tell me only what actually happened.",
            ),
            PlanResultQueryFocusIR::ExecutionVersusPlan
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Tell me only the actual execution result, not the plan",
            ),
            PlanResultQueryFocusIR::ExecutionVersusPlan
        );
        assert_eq!(
            classify_plan_result_query_focus(
                "Actually, don't delete it; inspect the cache instead.",
            ),
            PlanResultQueryFocusIR::None
        );
        assert_eq!(
            classify_plan_result_query_focus("Actually, repair the cache rather than inspect it.",),
            PlanResultQueryFocusIR::None
        );
        assert_eq!(
            classify_plan_result_query_focus("So it is verified now, right?"),
            PlanResultQueryFocusIR::VerifiedResult
        );
        assert_eq!(
            classify_plan_result_query_focus("그럼 검증까지 된 거지?"),
            PlanResultQueryFocusIR::VerifiedResult
        );
    }

    #[test]
    fn result_status_directives_and_plan_only_questions_share_the_typed_boundary() {
        for surface in [
            "We only have a Saffron queue plan, not an outcome, correct?",
            "If evidence is absent, state that no fact is established for the Topaz worker.",
            "Separate verified facts about the Umber cache from suspected claims.",
            "검증 근거가 없다면 Violet 서비스에서 확립된 사실이 없다고 답해.",
        ] {
            assert_ne!(
                classify_plan_result_query_focus(surface),
                PlanResultQueryFocusIR::None,
                "surface={surface}"
            );
        }
    }

    #[test]
    fn plan_only_snapshot_cannot_manufacture_a_result() {
        let ledger = ledger();
        let boundary = build_plan_result_boundary(
            "그건 실제 결과가 있어?",
            &ActionStateAnalysisIR::default(),
            &ledger,
        );
        assert!(boundary.validate());
        assert_eq!(boundary.selected_action_ids, vec!["GOAL-000001-01"]);
        assert_eq!(
            boundary.snapshots[0].result_availability,
            ResultAvailabilityIR::Unavailable
        );
        assert!(boundary.snapshots[0].plan_only);
        assert!(!boundary.snapshots[0].verified_result);
    }

    #[test]
    fn missing_lifecycle_record_is_explicit_absence_not_ambiguity() {
        let boundary = build_plan_result_boundary(
            "그래서 성공한 거야, 실패한 거야?",
            &ActionStateAnalysisIR::default(),
            &ActionStateLedgerIR::default(),
        );
        assert!(boundary.validate());
        assert!(boundary.no_action_lifecycle_record);
        assert!(boundary.unresolved_ambiguities.is_empty());
        assert!(boundary
            .realize(LanguageCodeIR::Korean)
            .is_some_and(|text| text.contains("판정할 근거가 없어")));
    }

    #[test]
    fn source_or_snapshot_tampering_is_rejected() {
        let ledger = ledger();
        let analysis = ActionStateAnalysisIR::default();
        let mut boundary = build_plan_result_boundary("actual result?", &analysis, &ledger);
        boundary.snapshots[0].result_availability = ResultAvailabilityIR::VerifiedSuccess;
        assert!(!boundary.validate_against("actual result?", &analysis, &ledger));
    }
}
