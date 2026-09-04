//! Typed plan, report, and verified execution-state boundary.
//!
//! Language can report an attempt or outcome, but only the typed host receipt
//! channel can change the observed execution state.  The two axes are stored
//! independently so fluent wording cannot manufacture a completed action.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ACTION_STATE_ANALYSIS_SCHEMA: &str = "B_CORE_ACTION_STATE_ANALYSIS_IR_1";
pub const ACTION_SET_QUERY_SCHEMA: &str = "B_CORE_ACTION_SET_QUERY_IR_1";
pub const ACTION_STATE_LEDGER_SCHEMA: &str = "B_CORE_ACTION_STATE_LEDGER_IR_1";
pub const ACTION_EVIDENCE_REQUEST_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_REQUEST_1";
pub const ACTION_EVIDENCE_RECEIPT_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_RECEIPT_1";
pub const ACTION_LANGUAGE_REPORT_RECORD_SCHEMA: &str = "B_CORE_ACTION_LANGUAGE_REPORT_RECORD_IR_1";
pub const ACTION_EVIDENCE_AUDIT_SCHEMA: &str = "B_CORE_ACTION_EVIDENCE_AUDIT_IR_1";

const MAX_ACTION_RECORDS: usize = 32;
const MAX_EXECUTION_EVIDENCE: usize = 8;
const MAX_LANGUAGE_REPORT_HISTORY: usize = 256;
const MAX_LANGUAGE_REPORTS_PER_ACTION: usize = 8;
const MAX_LANGUAGE_REPORT_EVIDENCE: usize = 16;
const MAX_EVIDENCE_AUDIT_HISTORY: usize = MAX_ACTION_RECORDS * MAX_EXECUTION_EVIDENCE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionPlanStatusIR {
    Active,
    Superseded,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionExecutionStatusIR {
    NotObserved,
    InProgress,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionReportedStatusIR {
    Attempted,
    InProgressClaimed,
    SuccessClaimed,
    FailureClaimed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionEvidenceStatusIR {
    ExecutionStarted,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPlanSeedIR {
    pub action_id: String,
    pub goal_id: String,
    pub canonical_predicate: String,
    pub predicate_surface: String,
    pub subject: String,
    pub source_semantic_text: String,
    pub introduced_turn: u64,
    pub external_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionStateRecordIR {
    pub action_id: String,
    pub goal_id: String,
    pub canonical_predicate: String,
    pub predicate_surface: String,
    pub subject: String,
    pub source_semantic_text: String,
    pub plan_status: ActionPlanStatusIR,
    pub execution_status: ActionExecutionStatusIR,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_status: Option<ActionReportedStatusIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub execution_evidence_ids: Vec<String>,
    pub introduced_turn: u64,
    pub last_update_turn: u64,
    pub external_execution_authorized: bool,
    pub external_action_execution_observed: bool,
    pub verified_outcome: bool,
    pub semantic_authority: bool,
}

impl ActionStateRecordIR {
    fn from_seed(seed: &ActionPlanSeedIR) -> Self {
        Self {
            action_id: seed.action_id.clone(),
            goal_id: seed.goal_id.clone(),
            canonical_predicate: seed.canonical_predicate.clone(),
            predicate_surface: seed.predicate_surface.clone(),
            subject: seed.subject.clone(),
            source_semantic_text: seed.source_semantic_text.clone(),
            plan_status: ActionPlanStatusIR::Active,
            execution_status: ActionExecutionStatusIR::NotObserved,
            reported_status: None,
            execution_id: None,
            execution_evidence_ids: Vec::new(),
            introduced_turn: seed.introduced_turn,
            last_update_turn: seed.introduced_turn,
            external_execution_authorized: seed.external_execution_authorized,
            external_action_execution_observed: false,
            verified_outcome: false,
            semantic_authority: false,
        }
    }

    fn validate(&self, completed_turns: u64) -> bool {
        valid_id(&self.action_id)
            && valid_id(&self.goal_id)
            && !self.canonical_predicate.trim().is_empty()
            && !self.predicate_surface.trim().is_empty()
            && !self.subject.trim().is_empty()
            && !self.source_semantic_text.trim().is_empty()
            && self.introduced_turn > 0
            && self.last_update_turn >= self.introduced_turn
            && self.last_update_turn <= completed_turns
            && self.execution_evidence_ids.len() <= MAX_EXECUTION_EVIDENCE
            && self.execution_evidence_ids.iter().all(|id| valid_id(id))
            && self.execution_id.as_deref().is_none_or(valid_id)
            && !self.semantic_authority
            && match self.execution_status {
                ActionExecutionStatusIR::NotObserved => {
                    self.execution_id.is_none()
                        && self.execution_evidence_ids.is_empty()
                        && !self.external_action_execution_observed
                        && !self.verified_outcome
                }
                ActionExecutionStatusIR::InProgress => {
                    self.execution_id.is_some()
                        && self.execution_evidence_ids.len() == 1
                        && self.external_action_execution_observed
                        && !self.verified_outcome
                }
                ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed => {
                    self.execution_id.is_some()
                        && self.execution_evidence_ids.len() >= 2
                        && self.external_action_execution_observed
                        && self.verified_outcome
                }
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionStateLedgerIR {
    pub schema: String,
    #[serde(default)]
    pub records: Vec<ActionStateRecordIR>,
    #[serde(default)]
    pub language_report_history: Vec<ActionLanguageReportRecordIR>,
    #[serde(default)]
    pub evidence_audit_history: Vec<ActionEvidenceAuditIR>,
}

impl Default for ActionStateLedgerIR {
    fn default() -> Self {
        Self {
            schema: ACTION_STATE_LEDGER_SCHEMA.to_string(),
            records: Vec::new(),
            language_report_history: Vec::new(),
            evidence_audit_history: Vec::new(),
        }
    }
}

impl ActionStateLedgerIR {
    pub fn record(&self, action_id: &str) -> Option<&ActionStateRecordIR> {
        self.records
            .iter()
            .find(|record| record.action_id == action_id)
    }

    pub fn current_record(&self) -> Option<&ActionStateRecordIR> {
        self.records
            .iter()
            .rev()
            .find(|record| record.plan_status == ActionPlanStatusIR::Active)
            .or_else(|| self.records.last())
    }

    pub fn replace_active_plans(&mut self, seeds: &[ActionPlanSeedIR], turn_index: u64) {
        if seeds.is_empty() {
            return;
        }
        let incoming = seeds
            .iter()
            .map(|seed| seed.action_id.as_str())
            .collect::<BTreeSet<_>>();
        for record in &mut self.records {
            if record.plan_status == ActionPlanStatusIR::Active
                && !incoming.contains(record.action_id.as_str())
            {
                record.plan_status = ActionPlanStatusIR::Superseded;
                record.last_update_turn = turn_index;
            }
        }
        self.add_plans(seeds);
        self.prune();
    }

    pub fn add_plans(&mut self, seeds: &[ActionPlanSeedIR]) {
        for seed in seeds {
            if let Some(record) = self
                .records
                .iter_mut()
                .find(|record| record.action_id == seed.action_id)
            {
                record.plan_status = ActionPlanStatusIR::Active;
                record.external_execution_authorized = seed.external_execution_authorized;
                record.last_update_turn = seed.introduced_turn;
            } else {
                self.records.push(ActionStateRecordIR::from_seed(seed));
            }
        }
        self.prune();
    }

    pub fn withdraw(&mut self, action_ids: &[String], turn_index: u64) {
        for record in &mut self.records {
            if action_ids.contains(&record.action_id) {
                record.plan_status = ActionPlanStatusIR::Withdrawn;
                record.last_update_turn = turn_index;
            }
        }
    }

    pub fn apply_language_report(
        &mut self,
        report: &ActionLanguageReportIR,
        turn_index: u64,
    ) -> bool {
        if report.source_surface.trim().is_empty()
            || report.confidence_millis > 1_000
            || report.evidence.len() > MAX_LANGUAGE_REPORT_EVIDENCE
            || report.semantic_authority
            || report.external_action_executed
            || turn_index == 0
        {
            return false;
        }
        let audit = ActionLanguageReportRecordIR::from_report(report, turn_index);
        if self
            .language_report_history
            .iter()
            .any(|existing| existing.report_id == audit.report_id)
        {
            return false;
        }
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.action_id == report.action_id)
        else {
            return false;
        };
        record.reported_status = Some(report.reported_status);
        record.last_update_turn = turn_index;
        self.language_report_history.push(audit);
        let reports_for_action = self
            .language_report_history
            .iter()
            .filter(|entry| entry.action_id == report.action_id)
            .count();
        if reports_for_action > MAX_LANGUAGE_REPORTS_PER_ACTION {
            if let Some(oldest) = self
                .language_report_history
                .iter()
                .position(|entry| entry.action_id == report.action_id)
            {
                self.language_report_history.remove(oldest);
            }
        }
        true
    }

    pub fn apply_evidence(
        &mut self,
        request: &ActionEvidenceRequestIR,
        turn_index: u64,
    ) -> Option<ActionEvidenceReceiptIR> {
        if !request.validate()
            || self
                .records
                .iter()
                .flat_map(|record| record.execution_evidence_ids.iter())
                .any(|id| id == &request.receipt_id)
        {
            return None;
        }
        let record = self
            .records
            .iter_mut()
            .find(|record| record.action_id == request.action_id)?;
        let prior = record.execution_status;
        match request.status {
            ActionEvidenceStatusIR::ExecutionStarted
                if record.execution_status == ActionExecutionStatusIR::NotObserved =>
            {
                record.execution_status = ActionExecutionStatusIR::InProgress;
                record.execution_id = Some(request.execution_id.clone());
            }
            ActionEvidenceStatusIR::Succeeded
                if record.execution_status == ActionExecutionStatusIR::InProgress
                    && record.execution_id.as_deref() == Some(request.execution_id.as_str()) =>
            {
                record.execution_status = ActionExecutionStatusIR::Succeeded;
            }
            ActionEvidenceStatusIR::Failed
                if record.execution_status == ActionExecutionStatusIR::InProgress
                    && record.execution_id.as_deref() == Some(request.execution_id.as_str()) =>
            {
                record.execution_status = ActionExecutionStatusIR::Failed;
            }
            _ => return None,
        }
        record
            .execution_evidence_ids
            .push(request.receipt_id.clone());
        record.last_update_turn = turn_index;
        record.external_action_execution_observed = true;
        record.verified_outcome = matches!(
            record.execution_status,
            ActionExecutionStatusIR::Succeeded | ActionExecutionStatusIR::Failed
        );
        let receipt = ActionEvidenceReceiptIR {
            schema: ACTION_EVIDENCE_RECEIPT_SCHEMA.to_string(),
            receipt_id: request.receipt_id.clone(),
            conversation_id: request.conversation_id.clone(),
            action_id: request.action_id.clone(),
            accepted: true,
            prior_execution_status: prior,
            resulting_execution_status: record.execution_status,
            verified_outcome: record.verified_outcome,
            external_action_execution_observed: true,
            unsupported_claims: 0,
        };
        self.evidence_audit_history.push(ActionEvidenceAuditIR::new(
            request,
            turn_index,
            prior,
            record.execution_status,
            record.verified_outcome,
        ));
        Some(receipt)
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        self.schema == ACTION_STATE_LEDGER_SCHEMA
            && self.records.len() <= MAX_ACTION_RECORDS
            && self
                .records
                .iter()
                .map(|record| &record.action_id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.records.len()
            && self
                .records
                .iter()
                .flat_map(|record| record.execution_evidence_ids.iter())
                .collect::<BTreeSet<_>>()
                .len()
                == self
                    .records
                    .iter()
                    .map(|record| record.execution_evidence_ids.len())
                    .sum::<usize>()
            && self
                .records
                .iter()
                .all(|record| record.validate(completed_turns))
            && self.validate_report_history(completed_turns)
            && self.validate_evidence_history(completed_turns)
    }

    fn validate_report_history(&self, completed_turns: u64) -> bool {
        let ids = self
            .language_report_history
            .iter()
            .map(|report| report.report_id.as_str())
            .collect::<BTreeSet<_>>();
        self.language_report_history.len() <= MAX_LANGUAGE_REPORT_HISTORY
            && ids.len() == self.language_report_history.len()
            && self.language_report_history.iter().all(|report| {
                report.validate(completed_turns)
                    && self
                        .record(&report.action_id)
                        .is_some_and(|record| report.turn_index >= record.introduced_turn)
            })
            && self.records.iter().all(|record| {
                let latest = self
                    .language_report_history
                    .iter()
                    .rev()
                    .find(|report| report.action_id == record.action_id);
                latest.map(|report| report.reported_status) == record.reported_status
            })
    }

    fn validate_evidence_history(&self, completed_turns: u64) -> bool {
        let ids = self
            .evidence_audit_history
            .iter()
            .map(|audit| audit.receipt_id.as_str())
            .collect::<BTreeSet<_>>();
        let stored_count = self
            .records
            .iter()
            .map(|record| record.execution_evidence_ids.len())
            .sum::<usize>();
        self.evidence_audit_history.len() <= MAX_EVIDENCE_AUDIT_HISTORY
            && ids.len() == self.evidence_audit_history.len()
            && self.evidence_audit_history.len() == stored_count
            && self.evidence_audit_history.iter().all(|audit| {
                audit.validate(completed_turns)
                    && self.record(&audit.action_id).is_some_and(|record| {
                        record.execution_evidence_ids.contains(&audit.receipt_id)
                    })
            })
            && self.records.iter().all(|record| {
                let audits = self
                    .evidence_audit_history
                    .iter()
                    .filter(|audit| audit.action_id == record.action_id)
                    .collect::<Vec<_>>();
                audits.len() == record.execution_evidence_ids.len()
                    && audits
                        .iter()
                        .map(|audit| &audit.receipt_id)
                        .eq(record.execution_evidence_ids.iter())
                    && match audits.as_slice() {
                        [] => record.execution_status == ActionExecutionStatusIR::NotObserved,
                        [start] => {
                            start.status == ActionEvidenceStatusIR::ExecutionStarted
                                && record.execution_status == ActionExecutionStatusIR::InProgress
                        }
                        [start, terminal] => {
                            start.status == ActionEvidenceStatusIR::ExecutionStarted
                                && matches!(
                                    terminal.status,
                                    ActionEvidenceStatusIR::Succeeded
                                        | ActionEvidenceStatusIR::Failed
                                )
                                && start.execution_id == terminal.execution_id
                                && terminal.resulting_execution_status == record.execution_status
                        }
                        _ => false,
                    }
            })
    }

    fn prune(&mut self) {
        if self.records.len() <= MAX_ACTION_RECORDS {
            return;
        }
        let excess = self.records.len() - MAX_ACTION_RECORDS;
        let removable = self
            .records
            .iter()
            .take(excess)
            .all(|record| record.plan_status != ActionPlanStatusIR::Active);
        if removable {
            self.records.drain(..excess);
            let retained = self
                .records
                .iter()
                .map(|record| record.action_id.as_str())
                .collect::<BTreeSet<_>>();
            self.language_report_history
                .retain(|report| retained.contains(report.action_id.as_str()));
            self.evidence_audit_history
                .retain(|audit| retained.contains(audit.action_id.as_str()));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionLanguageReportIR {
    pub action_id: String,
    pub reported_status: ActionReportedStatusIR,
    pub source_surface: String,
    pub confidence_millis: u16,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionLanguageReportRecordIR {
    pub schema: String,
    pub report_id: String,
    pub action_id: String,
    pub reported_status: ActionReportedStatusIR,
    pub source_surface: String,
    pub confidence_millis: u16,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub turn_index: u64,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub report_sha256: String,
}

impl ActionLanguageReportRecordIR {
    fn from_report(report: &ActionLanguageReportIR, turn_index: u64) -> Self {
        let mut record = Self {
            schema: ACTION_LANGUAGE_REPORT_RECORD_SCHEMA.to_string(),
            report_id: String::new(),
            action_id: report.action_id.clone(),
            reported_status: report.reported_status,
            source_surface: report.source_surface.clone(),
            confidence_millis: report.confidence_millis,
            evidence: report.evidence.clone(),
            turn_index,
            semantic_authority: false,
            external_action_executed: false,
            report_sha256: String::new(),
        };
        record.report_sha256 = action_language_report_record_sha256(&record);
        record.report_id = format!("REPORT-{turn_index}-{}", &record.report_sha256[..16]);
        record
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        self.schema == ACTION_LANGUAGE_REPORT_RECORD_SCHEMA
            && valid_id(&self.report_id)
            && valid_id(&self.action_id)
            && !self.source_surface.trim().is_empty()
            && self.confidence_millis <= 1_000
            && self.evidence.len() <= MAX_LANGUAGE_REPORT_EVIDENCE
            && self.turn_index > 0
            && self.turn_index <= completed_turns
            && !self.semantic_authority
            && !self.external_action_executed
            && valid_digest(&self.report_sha256)
            && self.report_sha256 == action_language_report_record_sha256(self)
            && self.report_id == format!("REPORT-{}-{}", self.turn_index, &self.report_sha256[..16])
    }
}

pub fn action_language_report_record_sha256(record: &ActionLanguageReportRecordIR) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_LANGUAGE_REPORT_RECORD_SCHEMA,
        record.action_id.as_str(),
        record.reported_status,
        record.source_surface.as_str(),
        record.confidence_millis,
        &record.evidence,
        record.turn_index,
        record.semantic_authority,
        record.external_action_executed,
    ))
    .expect("bounded action language report serializes");
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionSetOperatorIR {
    Identity,
    Intersection,
    Union,
    Difference,
    Complement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionSetQuantifierIR {
    All,
    Any,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionStatePredicateIR {
    ActivePlan,
    ReportedCompletion,
    ReportedFailure,
    UnverifiedExecution,
    VerifiedExecution,
    VerifiedSuccess,
    VerifiedFailure,
    VerifiedInProgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionSetTruthIR {
    True,
    False,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSetTermIR {
    pub source_surface: String,
    #[serde(default)]
    pub matched_action_ids: Vec<String>,
    pub excluded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionSetExpressionIR {
    SourceSet {
        action_ids: Vec<String>,
    },
    SubjectTerm {
        surface: String,
        action_ids: Vec<String>,
    },
    StatePredicate {
        predicate: ActionStatePredicateIR,
        negated: bool,
        action_ids: Vec<String>,
    },
    Union {
        left: Box<ActionSetExpressionIR>,
        right: Box<ActionSetExpressionIR>,
        action_ids: Vec<String>,
    },
    Intersection {
        left: Box<ActionSetExpressionIR>,
        right: Box<ActionSetExpressionIR>,
        action_ids: Vec<String>,
    },
    Difference {
        left: Box<ActionSetExpressionIR>,
        right: Box<ActionSetExpressionIR>,
        action_ids: Vec<String>,
    },
    Complement {
        source: Box<ActionSetExpressionIR>,
        excluded: Box<ActionSetExpressionIR>,
        action_ids: Vec<String>,
    },
}

impl ActionSetExpressionIR {
    pub fn action_ids(&self) -> &[String] {
        match self {
            Self::SourceSet { action_ids }
            | Self::SubjectTerm { action_ids, .. }
            | Self::StatePredicate { action_ids, .. }
            | Self::Union { action_ids, .. }
            | Self::Intersection { action_ids, .. }
            | Self::Difference { action_ids, .. }
            | Self::Complement { action_ids, .. } => action_ids,
        }
    }

    pub fn depth(&self) -> usize {
        1 + match self {
            Self::Union { left, right, .. }
            | Self::Intersection { left, right, .. }
            | Self::Difference { left, right, .. } => left.depth().max(right.depth()),
            Self::Complement {
                source, excluded, ..
            } => source.depth().max(excluded.depth()),
            Self::SourceSet { .. } | Self::SubjectTerm { .. } | Self::StatePredicate { .. } => 0,
        }
    }

    pub fn node_count(&self) -> usize {
        1 + match self {
            Self::Union { left, right, .. }
            | Self::Intersection { left, right, .. }
            | Self::Difference { left, right, .. } => left.node_count() + right.node_count(),
            Self::Complement {
                source, excluded, ..
            } => source.node_count() + excluded.node_count(),
            Self::SourceSet { .. } | Self::SubjectTerm { .. } | Self::StatePredicate { .. } => 0,
        }
    }

    fn validate_against(&self, source_action_ids: &[String]) -> bool {
        if self.depth() > 8 || self.node_count() > 32 {
            return false;
        }
        let source = source_action_ids.iter().collect::<BTreeSet<_>>();
        let ids_are_valid = |ids: &[String]| {
            ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
                && ids.iter().all(|action_id| source.contains(action_id))
        };
        if !ids_are_valid(self.action_ids()) {
            return false;
        }
        match self {
            Self::SourceSet { action_ids } => action_ids == source_action_ids,
            Self::SubjectTerm {
                surface,
                action_ids,
            } => !surface.trim().is_empty() && !action_ids.is_empty(),
            Self::StatePredicate { .. } => true,
            Self::Union {
                left,
                right,
                action_ids,
            } => {
                left.validate_against(source_action_ids)
                    && right.validate_against(source_action_ids)
                    && *action_ids == union_action_ids(left.action_ids(), right.action_ids())
            }
            Self::Intersection {
                left,
                right,
                action_ids,
            } => {
                left.validate_against(source_action_ids)
                    && right.validate_against(source_action_ids)
                    && *action_ids == intersection_action_ids(left.action_ids(), right.action_ids())
            }
            Self::Difference {
                left,
                right,
                action_ids,
            } => {
                left.validate_against(source_action_ids)
                    && right.validate_against(source_action_ids)
                    && *action_ids == difference_action_ids(left.action_ids(), right.action_ids())
            }
            Self::Complement {
                source,
                excluded,
                action_ids,
            } => {
                source.validate_against(source_action_ids)
                    && excluded.validate_against(source_action_ids)
                    && *action_ids
                        == difference_action_ids(source.action_ids(), excluded.action_ids())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSetQueryIR {
    pub schema: String,
    #[serde(default)]
    pub source_action_ids: Vec<String>,
    #[serde(default)]
    pub selected_action_ids: Vec<String>,
    #[serde(default)]
    pub excluded_action_ids: Vec<String>,
    #[serde(default)]
    pub terms: Vec<ActionSetTermIR>,
    #[serde(default)]
    pub operator_trace: Vec<ActionSetOperatorIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<ActionSetExpressionIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantifier: Option<ActionSetQuantifierIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<ActionStatePredicateIR>,
    pub truth: ActionSetTruthIR,
    #[serde(default)]
    pub unresolved_terms: Vec<String>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
    pub query_sha256: String,
}

impl ActionSetQueryIR {
    pub fn validate(&self) -> bool {
        let source = self.source_action_ids.iter().collect::<BTreeSet<_>>();
        let selected = self.selected_action_ids.iter().collect::<BTreeSet<_>>();
        let excluded = self.excluded_action_ids.iter().collect::<BTreeSet<_>>();
        self.schema == ACTION_SET_QUERY_SCHEMA
            && !self.source_action_ids.is_empty()
            && source.len() == self.source_action_ids.len()
            && selected.len() == self.selected_action_ids.len()
            && excluded.len() == self.excluded_action_ids.len()
            && self
                .selected_action_ids
                .iter()
                .all(|action_id| source.contains(action_id))
            && self
                .excluded_action_ids
                .iter()
                .all(|action_id| source.contains(action_id))
            && self.terms.iter().all(|term| {
                !term.source_surface.trim().is_empty()
                    && term
                        .matched_action_ids
                        .iter()
                        .all(|action_id| source.contains(action_id))
            })
            && !self.operator_trace.is_empty()
            && self.expression.as_ref().is_none_or(|expression| {
                expression.validate_against(&self.source_action_ids)
                    && expression.action_ids() == self.selected_action_ids
            })
            && !self.semantic_authority
            && !self.external_action_executed
            && self.query_sha256 == action_set_query_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionStateAnalysisIR {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_report: Option<ActionLanguageReportIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_reports: Vec<ActionLanguageReportIR>,
    pub query_requested: bool,
    pub untrusted_evidence_claim: bool,
    #[serde(default)]
    pub target_action_ids: Vec<String>,
    #[serde(default)]
    pub unresolved_ambiguities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_query: Option<ActionSetQueryIR>,
    pub semantic_authority: bool,
    pub external_action_executed: bool,
}

impl Default for ActionStateAnalysisIR {
    fn default() -> Self {
        Self {
            schema: ACTION_STATE_ANALYSIS_SCHEMA.to_string(),
            detected_report: None,
            detected_reports: Vec::new(),
            query_requested: false,
            untrusted_evidence_claim: false,
            target_action_ids: Vec::new(),
            unresolved_ambiguities: Vec::new(),
            set_query: None,
            semantic_authority: false,
            external_action_executed: false,
        }
    }
}

impl ActionStateAnalysisIR {
    pub fn has_language_reports(&self) -> bool {
        self.detected_report.is_some() || !self.detected_reports.is_empty()
    }

    pub fn language_reports(&self) -> Vec<&ActionLanguageReportIR> {
        if self.detected_reports.is_empty() {
            self.detected_report.iter().collect()
        } else {
            self.detected_reports.iter().collect()
        }
    }

    pub fn consumes_turn(&self) -> bool {
        self.has_language_reports()
            || self.query_requested
            || self.untrusted_evidence_claim
            || !self.unresolved_ambiguities.is_empty()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActionStateAnalyzer;

impl ActionStateAnalyzer {
    pub fn analyze(&self, text: &str, ledger: &ActionStateLedgerIR) -> ActionStateAnalysisIR {
        self.analyze_with_goal_hint(text, ledger, None)
    }

    pub fn analyze_with_goal_hint(
        &self,
        text: &str,
        ledger: &ActionStateLedgerIR,
        inherited_goal_id: Option<&str>,
    ) -> ActionStateAnalysisIR {
        let hints = inherited_goal_id.into_iter().collect::<Vec<_>>();
        self.analyze_with_goal_hints(text, ledger, &hints)
    }

    pub fn analyze_with_goal_hints(
        &self,
        text: &str,
        ledger: &ActionStateLedgerIR,
        inherited_goal_ids: &[&str],
    ) -> ActionStateAnalysisIR {
        self.analyze_with_goal_hints_and_query_surface(text, text, ledger, inherited_goal_ids)
    }

    pub fn analyze_with_goal_hints_and_query_surface(
        &self,
        text: &str,
        query_surface: &str,
        ledger: &ActionStateLedgerIR,
        inherited_goal_ids: &[&str],
    ) -> ActionStateAnalysisIR {
        let normalized = text.trim().to_lowercase();
        if normalized.is_empty() || ledger.records.is_empty() {
            return ActionStateAnalysisIR::default();
        }
        let query_requested = is_action_state_query(&normalized)
            || is_action_set_selection_query(&query_surface.trim().to_lowercase());
        let untrusted_evidence_claim = is_untrusted_evidence_claim(&normalized);
        let reported_status = (!query_requested
            && !untrusted_evidence_claim
            && !crate::conversation_contract::is_interrogative(query_surface))
        .then(|| reported_status(&normalized))
        .flatten();
        if !query_requested && !untrusted_evidence_claim && reported_status.is_none() {
            return ActionStateAnalysisIR::default();
        }
        let mut candidates = ledger
            .records
            .iter()
            .filter(|record| record.plan_status == ActionPlanStatusIR::Active)
            .collect::<Vec<_>>();
        if candidates.is_empty() && query_requested {
            candidates.extend(ledger.records.last());
        }
        let unique_hints = inherited_goal_ids.iter().copied().collect::<BTreeSet<_>>();
        let hinted = unique_hints
            .iter()
            .filter_map(|goal_id| {
                ledger
                    .records
                    .iter()
                    .find(|record| record.goal_id == **goal_id)
            })
            .collect::<Vec<_>>();
        let explicit = candidates
            .iter()
            .copied()
            .filter(|record| {
                normalized.contains(&record.subject.to_lowercase())
                    || normalized.contains(&record.predicate_surface.to_lowercase())
                    || normalized.contains(&record.canonical_predicate.to_lowercase())
            })
            .collect::<Vec<_>>();
        let mut targets = if !unique_hints.is_empty() && hinted.len() == unique_hints.len() {
            hinted
        } else if !unique_hints.is_empty() {
            Vec::new()
        } else if explicit.len() == 1 {
            explicit
        } else if explicit.is_empty() && candidates.len() == 1 {
            vec![candidates[0]]
        } else {
            Vec::new()
        };
        let mut unresolved_ambiguities = Vec::new();
        if targets.is_empty() {
            unresolved_ambiguities.push(if candidates.is_empty() {
                "NO_ACTION_STATE_TARGET".to_string()
            } else {
                "MULTIPLE_ACTION_STATE_TARGETS".to_string()
            });
        }
        let set_query = (query_requested && targets.len() >= 2)
            .then(|| compose_action_set_query(query_surface, &targets));
        if let Some(query) = &set_query {
            if query.unresolved_terms.is_empty() {
                targets.retain(|record| query.selected_action_ids.contains(&record.action_id));
            } else {
                targets.clear();
                unresolved_ambiguities.extend(
                    query
                        .unresolved_terms
                        .iter()
                        .map(|term| format!("ACTION_SET_QUERY:{term}")),
                );
            }
        }
        let detected_reports = reported_status.map_or_else(Vec::new, |status| {
            targets
                .iter()
                .map(|record| ActionLanguageReportIR {
                    action_id: record.action_id.clone(),
                    reported_status: status,
                    source_surface: text.trim().to_string(),
                    confidence_millis: report_confidence(status),
                    evidence: vec![
                        format!("LANGUAGE_REPORT={status:?}").to_uppercase(),
                        "SEMANTIC_AUTHORITY=FALSE".to_string(),
                        "EXTERNAL_ACTION_EXECUTED=FALSE".to_string(),
                    ],
                    semantic_authority: false,
                    external_action_executed: false,
                })
                .collect()
        });
        let detected_report = (detected_reports.len() == 1).then(|| detected_reports[0].clone());
        ActionStateAnalysisIR {
            schema: ACTION_STATE_ANALYSIS_SCHEMA.to_string(),
            detected_report,
            detected_reports,
            query_requested,
            untrusted_evidence_claim,
            target_action_ids: targets
                .iter()
                .map(|record| record.action_id.clone())
                .collect(),
            unresolved_ambiguities,
            set_query,
            semantic_authority: false,
            external_action_executed: false,
        }
    }
}

fn compose_action_set_query(
    query_surface: &str,
    source_records: &[&ActionStateRecordIR],
) -> ActionSetQueryIR {
    let normalized = query_surface.trim().to_lowercase();
    let source_action_ids = source_records
        .iter()
        .map(|record| record.action_id.clone())
        .collect::<Vec<_>>();
    let korean_negative_scope = contains_any(
        &normalized,
        &["빼고", "제외", "말고", "아닌", "아니고", "도 아닌"],
    );
    let english_negative_start = [
        " except ",
        "except ",
        " excluding ",
        "excluding ",
        " apart from ",
        " leave out ",
        "leave out ",
        " exclude ",
        "exclude ",
        " neither ",
        "neither ",
    ]
    .iter()
    .filter_map(|marker| normalized.find(marker))
    .min();
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    let mut terms = Vec::new();
    for record in source_records {
        let subject = record.subject.trim().to_lowercase();
        let Some(position) = (!subject.is_empty())
            .then(|| normalized.find(&subject))
            .flatten()
        else {
            continue;
        };
        let excluded =
            korean_negative_scope || english_negative_start.is_some_and(|start| position > start);
        if excluded {
            negative.push(record.action_id.clone());
        } else {
            positive.push(record.action_id.clone());
        }
        terms.push(ActionSetTermIR {
            source_surface: record.subject.clone(),
            matched_action_ids: vec![record.action_id.clone()],
            excluded,
        });
    }
    positive.sort();
    positive.dedup();
    negative.sort();
    negative.dedup();
    let explicit_positive_selector = contains_any(
        &normalized,
        &[
            " only ",
            "only ",
            " just ",
            "just ",
            "만 ",
            "만,",
            "만 상태",
            "만 현황",
            "만 골라",
        ],
    );
    let explicit_union = contains_any(
        &normalized,
        &[
            " or ",
            "either ",
            " 또는 ",
            " 혹은 ",
            " 아니면 ",
            "나 ",
            "이나 ",
        ],
    );
    let explicit_negative = korean_negative_scope || english_negative_start.is_some();
    let mut operator_trace = Vec::new();
    let mut selected_action_ids = if explicit_negative {
        if negative.len() > 1 {
            operator_trace.push(ActionSetOperatorIR::Complement);
        } else {
            operator_trace.push(ActionSetOperatorIR::Difference);
        }
        source_action_ids
            .iter()
            .filter(|action_id| !negative.contains(action_id))
            .cloned()
            .collect::<Vec<_>>()
    } else if explicit_positive_selector || explicit_union {
        operator_trace.push(if explicit_union && positive.len() > 1 {
            ActionSetOperatorIR::Union
        } else {
            ActionSetOperatorIR::Intersection
        });
        positive.clone()
    } else {
        operator_trace.push(ActionSetOperatorIR::Identity);
        source_action_ids.clone()
    };
    selected_action_ids.sort();
    selected_action_ids.dedup();
    let mut predicate = detect_action_state_predicate(&normalized);
    let mut quantifier = predicate.and_then(|_| detect_action_set_quantifier(&normalized));
    let mut truth = match (quantifier, predicate) {
        (Some(quantifier), Some(predicate)) if !selected_action_ids.is_empty() => {
            let evaluations = source_records
                .iter()
                .filter(|record| selected_action_ids.contains(&record.action_id))
                .map(|record| action_record_matches_predicate(record, predicate))
                .collect::<Vec<_>>();
            let value = match quantifier {
                ActionSetQuantifierIR::All => evaluations.iter().all(|value| *value),
                ActionSetQuantifierIR::Any => evaluations.iter().any(|value| *value),
                ActionSetQuantifierIR::None => !evaluations.iter().any(|value| *value),
            };
            if value {
                ActionSetTruthIR::True
            } else {
                ActionSetTruthIR::False
            }
        }
        (Some(_), Some(_)) => ActionSetTruthIR::Unknown,
        _ => ActionSetTruthIR::NotApplicable,
    };
    let mut unresolved_terms = Vec::new();
    if !explicit_negative && (explicit_positive_selector || explicit_union) && positive.is_empty() {
        unresolved_terms.push("UNBOUND_POSITIVE_ACTION_TERM".to_string());
    }
    if explicit_negative && negative.is_empty() {
        unresolved_terms.push("UNBOUND_EXCLUDED_ACTION_TERM".to_string());
    }
    if selected_action_ids.is_empty() {
        unresolved_terms.push("EMPTY_ACTION_SET".to_string());
    }
    let mut expression = None;
    match recursive_action_expression(&normalized, source_records) {
        RecursiveExpressionOutcome::NotApplicable => {}
        RecursiveExpressionOutcome::Parsed(parsed) => {
            selected_action_ids = parsed.action_ids().to_vec();
            negative = difference_action_ids(&source_action_ids, &selected_action_ids);
            operator_trace = expression_operator_trace(&parsed);
            if operator_trace.is_empty() {
                operator_trace.push(ActionSetOperatorIR::Identity);
            }
            predicate = expression_state_predicate(&parsed).or(predicate);
            quantifier = None;
            truth = ActionSetTruthIR::NotApplicable;
            unresolved_terms.clear();
            if selected_action_ids.is_empty() {
                unresolved_terms.push("EMPTY_ACTION_SET".to_string());
            }
            expression = Some(parsed);
        }
        RecursiveExpressionOutcome::Invalid(mut errors) => {
            selected_action_ids.clear();
            negative = source_action_ids.clone();
            expression = None;
            unresolved_terms.append(&mut errors);
        }
    }
    unresolved_terms.sort();
    unresolved_terms.dedup();
    let mut query = ActionSetQueryIR {
        schema: ACTION_SET_QUERY_SCHEMA.to_string(),
        source_action_ids,
        selected_action_ids,
        excluded_action_ids: negative,
        terms,
        operator_trace,
        expression,
        quantifier,
        predicate,
        truth,
        unresolved_terms,
        semantic_authority: false,
        external_action_executed: false,
        query_sha256: String::new(),
    };
    query.query_sha256 = action_set_query_sha256(&query);
    debug_assert!(query.validate());
    query
}

enum RecursiveExpressionOutcome {
    NotApplicable,
    Parsed(ActionSetExpressionIR),
    Invalid(Vec<String>),
}

fn recursive_action_expression(
    text: &str,
    source_records: &[&ActionStateRecordIR],
) -> RecursiveExpressionOutcome {
    let relative_filter = detect_relative_state_predicate(text).is_some()
        && contains_any(
            text,
            &["show", "list", "give", "골라", "보여", "알려", "말해"],
        );
    if !text.contains(['(', ')']) && !relative_filter {
        return RecursiveExpressionOutcome::NotApplicable;
    }
    if !balanced_parentheses(text) {
        return RecursiveExpressionOutcome::Invalid(vec![
            "MALFORMED_ACTION_SET_PARENTHESES".to_string()
        ]);
    }
    let source_action_ids = source_records
        .iter()
        .map(|record| record.action_id.clone())
        .collect::<Vec<_>>();
    match parse_action_set_expression(text, source_records, &source_action_ids, 1) {
        Ok(expression) if expression.validate_against(&source_action_ids) => {
            RecursiveExpressionOutcome::Parsed(expression)
        }
        Ok(_) => RecursiveExpressionOutcome::Invalid(vec![
            "INVALID_RECURSIVE_ACTION_SET_EXPRESSION".to_string(),
        ]),
        Err(error) => RecursiveExpressionOutcome::Invalid(vec![error]),
    }
}

fn parse_action_set_expression(
    text: &str,
    source_records: &[&ActionStateRecordIR],
    source_action_ids: &[String],
    depth: usize,
) -> Result<ActionSetExpressionIR, String> {
    if depth > 8 {
        return Err("ACTION_SET_EXPRESSION_DEPTH_EXCEEDED".to_string());
    }
    let text = text.trim_matches(|character: char| {
        character.is_whitespace() || matches!(character, '.' | '?' | '!' | ',' | ':' | ';')
    });
    if text.is_empty() {
        return Err("EMPTY_ACTION_SET_EXPRESSION".to_string());
    }
    if wrapping_parentheses(text).is_some() {
        let (start, end) = wrapping_parentheses(text).expect("checked wrapping parentheses");
        return parse_action_set_expression(
            &text[start..end],
            source_records,
            source_action_ids,
            depth + 1,
        );
    }
    if let Some((index, marker)) =
        find_top_level_marker(text, &[" or ", " 또는 ", " 혹은 ", " 아니면 "])
    {
        let left = parse_action_set_expression(
            &text[..index],
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        let right = parse_action_set_expression(
            &text[index + marker.len()..],
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        return Ok(union_expression(left, right));
    }
    if let Some((index, marker)) =
        find_top_level_marker(text, &[" except ", " excluding ", " apart from "])
    {
        let left = parse_action_set_expression(
            &text[..index],
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        let right = parse_action_set_expression(
            &text[index + marker.len()..],
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        return Ok(difference_expression(left, right));
    }
    if let Some((left_surface, excluded_surface, complement)) = korean_difference_parts(text) {
        let left = parse_action_set_expression(
            left_surface,
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        let excluded = parse_action_set_expression(
            excluded_surface,
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        return Ok(if complement {
            complement_expression(left, excluded)
        } else {
            difference_expression(left, excluded)
        });
    }
    let parenthesized = first_parenthesized_span(text);
    if let Some((predicate, negated)) = detect_relative_state_predicate(text) {
        let base = if let Some((start, end)) = parenthesized {
            parse_action_set_expression(
                &text[start..end],
                source_records,
                source_action_ids,
                depth + 1,
            )?
        } else if let Some(subjects) = subject_expression(text, source_records) {
            subjects
        } else {
            source_expression(source_action_ids)
        };
        let predicate = state_predicate_expression(predicate, negated, source_records);
        return Ok(intersection_expression(base, predicate));
    }
    if let Some(index) = find_top_level_prefix_not(text) {
        let excluded = parse_action_set_expression(
            &text[index..],
            source_records,
            source_action_ids,
            depth + 1,
        )?;
        return Ok(complement_expression(
            source_expression(source_action_ids),
            excluded,
        ));
    }
    if let Some((start, end)) = parenthesized {
        let outside = format!("{} {}", &text[..start - 1], &text[end + 1..]);
        if subject_expression(&outside, source_records).is_none() {
            return parse_action_set_expression(
                &text[start..end],
                source_records,
                source_action_ids,
                depth + 1,
            );
        }
    }
    if let Some(term) = korean_negated_subject_expression(text, source_records) {
        return Ok(complement_expression(
            source_expression(source_action_ids),
            term,
        ));
    }
    if let Some(subjects) = subject_expression(text, source_records) {
        return Ok(subjects);
    }
    if contains_source_set_surface(text) {
        return Ok(source_expression(source_action_ids));
    }
    Err("UNBOUND_RECURSIVE_ACTION_TERM".to_string())
}

fn source_expression(source_action_ids: &[String]) -> ActionSetExpressionIR {
    ActionSetExpressionIR::SourceSet {
        action_ids: source_action_ids.to_vec(),
    }
}

fn subject_expression(
    text: &str,
    source_records: &[&ActionStateRecordIR],
) -> Option<ActionSetExpressionIR> {
    let normalized = text.to_lowercase();
    let terms = source_records
        .iter()
        .filter_map(|record| {
            let surface = record.subject.trim().to_lowercase();
            (!surface.is_empty() && normalized.contains(&surface)).then(|| {
                ActionSetExpressionIR::SubjectTerm {
                    surface: record.subject.clone(),
                    action_ids: vec![record.action_id.clone()],
                }
            })
        })
        .collect::<Vec<_>>();
    let mut terms = terms.into_iter();
    let first = terms.next()?;
    Some(terms.fold(first, union_expression))
}

fn korean_negated_subject_expression(
    text: &str,
    source_records: &[&ActionStateRecordIR],
) -> Option<ActionSetExpressionIR> {
    let normalized = text.to_lowercase();
    source_records.iter().find_map(|record| {
        let subject = record.subject.trim().to_lowercase();
        [
            format!("{subject}가 아닌"),
            format!("{subject}이 아닌"),
            format!("{subject}는 아닌"),
        ]
        .iter()
        .any(|pattern| normalized.contains(pattern))
        .then(|| ActionSetExpressionIR::SubjectTerm {
            surface: record.subject.clone(),
            action_ids: vec![record.action_id.clone()],
        })
    })
}

fn state_predicate_expression(
    predicate: ActionStatePredicateIR,
    negated: bool,
    source_records: &[&ActionStateRecordIR],
) -> ActionSetExpressionIR {
    let action_ids = source_records
        .iter()
        .filter(|record| action_record_matches_predicate(record, predicate) != negated)
        .map(|record| record.action_id.clone())
        .collect::<Vec<_>>();
    ActionSetExpressionIR::StatePredicate {
        predicate,
        negated,
        action_ids,
    }
}

fn union_expression(
    left: ActionSetExpressionIR,
    right: ActionSetExpressionIR,
) -> ActionSetExpressionIR {
    let action_ids = union_action_ids(left.action_ids(), right.action_ids());
    ActionSetExpressionIR::Union {
        left: Box::new(left),
        right: Box::new(right),
        action_ids,
    }
}

fn intersection_expression(
    left: ActionSetExpressionIR,
    right: ActionSetExpressionIR,
) -> ActionSetExpressionIR {
    let action_ids = intersection_action_ids(left.action_ids(), right.action_ids());
    ActionSetExpressionIR::Intersection {
        left: Box::new(left),
        right: Box::new(right),
        action_ids,
    }
}

fn difference_expression(
    left: ActionSetExpressionIR,
    right: ActionSetExpressionIR,
) -> ActionSetExpressionIR {
    let action_ids = difference_action_ids(left.action_ids(), right.action_ids());
    ActionSetExpressionIR::Difference {
        left: Box::new(left),
        right: Box::new(right),
        action_ids,
    }
}

fn complement_expression(
    source: ActionSetExpressionIR,
    excluded: ActionSetExpressionIR,
) -> ActionSetExpressionIR {
    let action_ids = difference_action_ids(source.action_ids(), excluded.action_ids());
    ActionSetExpressionIR::Complement {
        source: Box::new(source),
        excluded: Box::new(excluded),
        action_ids,
    }
}

fn union_action_ids(left: &[String], right: &[String]) -> Vec<String> {
    let mut values = left.iter().chain(right).cloned().collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn intersection_action_ids(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().collect::<BTreeSet<_>>();
    let mut values = left
        .iter()
        .filter(|action_id| right.contains(action_id))
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn difference_action_ids(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().collect::<BTreeSet<_>>();
    let mut values = left
        .iter()
        .filter(|action_id| !right.contains(action_id))
        .cloned()
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn expression_operator_trace(expression: &ActionSetExpressionIR) -> Vec<ActionSetOperatorIR> {
    let mut operators = Vec::new();
    fn visit(expression: &ActionSetExpressionIR, operators: &mut Vec<ActionSetOperatorIR>) {
        match expression {
            ActionSetExpressionIR::Union { left, right, .. } => {
                operators.push(ActionSetOperatorIR::Union);
                visit(left, operators);
                visit(right, operators);
            }
            ActionSetExpressionIR::Intersection { left, right, .. } => {
                operators.push(ActionSetOperatorIR::Intersection);
                visit(left, operators);
                visit(right, operators);
            }
            ActionSetExpressionIR::Difference { left, right, .. } => {
                operators.push(ActionSetOperatorIR::Difference);
                visit(left, operators);
                visit(right, operators);
            }
            ActionSetExpressionIR::Complement {
                source, excluded, ..
            } => {
                operators.push(ActionSetOperatorIR::Complement);
                visit(source, operators);
                visit(excluded, operators);
            }
            ActionSetExpressionIR::SourceSet { .. }
            | ActionSetExpressionIR::SubjectTerm { .. }
            | ActionSetExpressionIR::StatePredicate { .. } => {}
        }
    }
    visit(expression, &mut operators);
    operators
}

fn expression_state_predicate(
    expression: &ActionSetExpressionIR,
) -> Option<ActionStatePredicateIR> {
    match expression {
        ActionSetExpressionIR::StatePredicate { predicate, .. } => Some(*predicate),
        ActionSetExpressionIR::Union { left, right, .. }
        | ActionSetExpressionIR::Intersection { left, right, .. }
        | ActionSetExpressionIR::Difference { left, right, .. } => {
            expression_state_predicate(left).or_else(|| expression_state_predicate(right))
        }
        ActionSetExpressionIR::Complement {
            source, excluded, ..
        } => expression_state_predicate(source).or_else(|| expression_state_predicate(excluded)),
        ActionSetExpressionIR::SourceSet { .. } | ActionSetExpressionIR::SubjectTerm { .. } => None,
    }
}

fn detect_relative_state_predicate(text: &str) -> Option<(ActionStatePredicateIR, bool)> {
    let normalized = text.to_lowercase();
    let negated = contains_any(
        &normalized,
        &[
            "without a completion report",
            "without completion report",
            "not reported complete",
            "lacking a completion report",
            "lack a completion report",
            "완료 보고가 없는",
            "완료 보고 없이",
            "보고되지 않은",
            "완료 보고가 빠진",
        ],
    );
    if negated {
        return Some((ActionStatePredicateIR::ReportedCompletion, true));
    }
    let predicate = detect_action_state_predicate(&normalized)?;
    let relative = contains_any(
        &normalized,
        &[
            " with ",
            " that ",
            " which ",
            " having ",
            " reported complete",
            "완료 보고가 있는",
            "완료됐다고 보고된",
            "완료 보고를 가진",
            "보고된 작업",
        ],
    );
    relative.then_some((predicate, false))
}

fn is_action_set_selection_query(text: &str) -> bool {
    let group = contains_any(
        text,
        &[
            "all tasks",
            "every task",
            "the actions",
            "모든 작업",
            "작업 전부",
            "작업들",
        ],
    );
    let selection = contains_any(
        text,
        &["show", "list", "give", "골라", "보여", "알려", "말해"],
    );
    let structure = text.contains(['(', ')'])
        || contains_any(
            text,
            &[
                " or ",
                " except ",
                " apart from ",
                " 또는 ",
                " 혹은 ",
                " 빼고",
                " 제외",
                " 말고",
                "완료 보고",
                "완료됐다고 보고",
                "보고된",
                "보고되지",
            ],
        );
    group && selection && structure
}

fn contains_source_set_surface(text: &str) -> bool {
    contains_any(
        &text.to_lowercase(),
        &[
            "all tasks",
            "every task",
            "tasks",
            "actions",
            "모든 작업",
            "작업 전부",
            "작업들",
            "작업",
        ],
    )
}

fn balanced_parentheses(text: &str) -> bool {
    let mut depth = 0_i32;
    for character in text.chars() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn wrapping_parentheses(text: &str) -> Option<(usize, usize)> {
    let start = text.find('(')?;
    if !text[..start].trim().is_empty() {
        return None;
    }
    let end = matching_parenthesis(text, start)?;
    text[end + 1..]
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, '.' | '?' | '!' | ',' | ':' | ';')
        })
        .is_empty()
        .then_some((start + 1, end))
}

fn first_parenthesized_span(text: &str) -> Option<(usize, usize)> {
    let start = text.find('(')?;
    let end = matching_parenthesis(text, start)?;
    Some((start + 1, end))
}

fn matching_parenthesis(text: &str, start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (offset, character) in text[start..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_top_level_marker(text: &str, markers: &[&'static str]) -> Option<(usize, &'static str)> {
    let mut depth = 0_i32;
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => {
                if let Some(marker) = markers
                    .iter()
                    .copied()
                    .find(|marker| text[index..].starts_with(marker))
                {
                    return Some((index, marker));
                }
            }
            _ => {}
        }
    }
    None
}

fn korean_difference_parts(text: &str) -> Option<(&str, &str, bool)> {
    let separators = ["에서 ", " 중 "];
    let endings = [
        "를 제외한",
        "을 제외한",
        "를 제외하고",
        "을 제외하고",
        "를 빼고",
        "을 빼고",
        "를 뺀",
        "을 뺀",
        " 말고",
    ];
    let mut depth = 0_i32;
    let mut separator_matches = Vec::new();
    let mut ending_match = None;
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => {
                if let Some(marker) = separators
                    .iter()
                    .copied()
                    .find(|marker| text[index..].starts_with(marker))
                {
                    separator_matches.push((index, marker));
                }
                if ending_match.is_none() {
                    ending_match = endings
                        .iter()
                        .copied()
                        .find(|marker| text[index..].starts_with(marker))
                        .map(|marker| (index, marker));
                }
            }
            _ => {}
        }
    }
    let (ending, end_marker) = ending_match?;
    let (separator, separator_marker) = separator_matches
        .into_iter()
        .rfind(|(position, _)| *position < ending)?;
    let rest_start = separator + separator_marker.len();
    let relative_end = ending.saturating_sub(rest_start);
    let rest = &text[rest_start..];
    let left = text[..separator].trim();
    let excluded = rest[..relative_end].trim();
    if left.is_empty() || excluded.is_empty() {
        return None;
    }
    let tail = &rest[relative_end + end_marker.len()..];
    let complement = contains_source_set_surface(left) && tail.contains("나머지");
    Some((left, excluded, complement))
}

fn find_top_level_prefix_not(text: &str) -> Option<usize> {
    let (position, marker) = find_top_level_marker(text, &["not ", " not "])?;
    Some(position + marker.len())
}

fn detect_action_set_quantifier(text: &str) -> Option<ActionSetQuantifierIR> {
    if contains_any(
        text,
        &["none of", "no task", "not any", "하나도", "아무 작업도"],
    ) {
        Some(ActionSetQuantifierIR::None)
    } else if contains_any(
        text,
        &[
            " any ",
            "any of",
            "at least one",
            "is there any",
            "하나라도",
            "적어도 하나",
            "최소 하나",
            "있는 게 하나",
        ],
    ) {
        Some(ActionSetQuantifierIR::Any)
    } else if contains_any(
        text,
        &[
            "all tasks",
            "every task",
            "every one",
            "모든 작업",
            "작업 전부",
            "전부가",
            "모두가",
        ],
    ) {
        Some(ActionSetQuantifierIR::All)
    } else {
        None
    }
}

fn detect_action_state_predicate(text: &str) -> Option<ActionStatePredicateIR> {
    if contains_any(
        text,
        &[
            "reported completion",
            "reported complete",
            "reported as finished",
            "reported as complete",
            "completion report",
            "완료 보고",
            "완료됐다고 보고",
            "끝났다고 보고",
            "끝났다고",
        ],
    ) {
        Some(ActionStatePredicateIR::ReportedCompletion)
    } else if contains_any(text, &["reported failure", "실패 보고", "실패했다고 보고"]) {
        Some(ActionStatePredicateIR::ReportedFailure)
    } else if contains_any(
        text,
        &[
            "lack verified",
            "without verified",
            "without a verified",
            "no verified",
            "unverified",
            "검증된 실행 결과가 없",
            "검증 결과 없는",
            "검증 증거가 없는",
            "검증 증거가 없",
            "미검증",
        ],
    ) {
        Some(ActionStatePredicateIR::UnverifiedExecution)
    } else if contains_any(text, &["verified success", "성공 검증"]) {
        Some(ActionStatePredicateIR::VerifiedSuccess)
    } else if contains_any(text, &["verified failure", "실패 검증"]) {
        Some(ActionStatePredicateIR::VerifiedFailure)
    } else if contains_any(text, &["verified in progress", "검증된 실행 중"]) {
        Some(ActionStatePredicateIR::VerifiedInProgress)
    } else if contains_any(text, &["verified execution", "검증된 실행", "실행 검증"]) {
        Some(ActionStatePredicateIR::VerifiedExecution)
    } else if contains_any(text, &["active plan", "활성 계획"]) {
        Some(ActionStatePredicateIR::ActivePlan)
    } else {
        None
    }
}

fn action_record_matches_predicate(
    record: &ActionStateRecordIR,
    predicate: ActionStatePredicateIR,
) -> bool {
    match predicate {
        ActionStatePredicateIR::ActivePlan => record.plan_status == ActionPlanStatusIR::Active,
        ActionStatePredicateIR::ReportedCompletion => {
            record.reported_status == Some(ActionReportedStatusIR::SuccessClaimed)
        }
        ActionStatePredicateIR::ReportedFailure => {
            record.reported_status == Some(ActionReportedStatusIR::FailureClaimed)
        }
        ActionStatePredicateIR::UnverifiedExecution => {
            record.execution_status == ActionExecutionStatusIR::NotObserved
        }
        ActionStatePredicateIR::VerifiedExecution => {
            record.execution_status != ActionExecutionStatusIR::NotObserved
        }
        ActionStatePredicateIR::VerifiedSuccess => {
            record.execution_status == ActionExecutionStatusIR::Succeeded
        }
        ActionStatePredicateIR::VerifiedFailure => {
            record.execution_status == ActionExecutionStatusIR::Failed
        }
        ActionStatePredicateIR::VerifiedInProgress => {
            record.execution_status == ActionExecutionStatusIR::InProgress
        }
    }
}

fn action_set_query_sha256(query: &ActionSetQueryIR) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_SET_QUERY_SCHEMA,
        &query.source_action_ids,
        &query.selected_action_ids,
        &query.excluded_action_ids,
        &query.terms,
        &query.operator_trace,
        &query.expression,
        query.quantifier,
        query.predicate,
        query.truth,
        &query.unresolved_terms,
        query.semantic_authority,
        query.external_action_executed,
    ))
    .expect("bounded action-set query serializes");
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEvidenceRequestIR {
    pub schema: String,
    pub receipt_id: String,
    pub conversation_id: String,
    pub action_id: String,
    pub execution_id: String,
    pub status: ActionEvidenceStatusIR,
    pub evidence_digest: String,
    pub verifier_receipt_sha256: String,
}

impl ActionEvidenceRequestIR {
    pub fn validate(&self) -> bool {
        self.schema == ACTION_EVIDENCE_REQUEST_SCHEMA
            && valid_id(&self.receipt_id)
            && valid_id(&self.conversation_id)
            && valid_id(&self.action_id)
            && valid_id(&self.execution_id)
            && valid_digest(&self.evidence_digest)
            && valid_digest(&self.verifier_receipt_sha256)
            && self.verifier_receipt_sha256 == action_evidence_receipt_sha256(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEvidenceReceiptIR {
    pub schema: String,
    pub receipt_id: String,
    pub conversation_id: String,
    pub action_id: String,
    pub accepted: bool,
    pub prior_execution_status: ActionExecutionStatusIR,
    pub resulting_execution_status: ActionExecutionStatusIR,
    pub verified_outcome: bool,
    pub external_action_execution_observed: bool,
    pub unsupported_claims: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEvidenceAuditIR {
    pub schema: String,
    pub receipt_id: String,
    pub conversation_id: String,
    pub action_id: String,
    pub execution_id: String,
    pub status: ActionEvidenceStatusIR,
    pub evidence_digest: String,
    pub verifier_receipt_sha256: String,
    pub accepted_turn: u64,
    pub prior_execution_status: ActionExecutionStatusIR,
    pub resulting_execution_status: ActionExecutionStatusIR,
    pub verified_outcome: bool,
    pub semantic_authority: bool,
    pub language_originated: bool,
    pub audit_sha256: String,
}

impl ActionEvidenceAuditIR {
    fn new(
        request: &ActionEvidenceRequestIR,
        accepted_turn: u64,
        prior_execution_status: ActionExecutionStatusIR,
        resulting_execution_status: ActionExecutionStatusIR,
        verified_outcome: bool,
    ) -> Self {
        let mut audit = Self {
            schema: ACTION_EVIDENCE_AUDIT_SCHEMA.to_string(),
            receipt_id: request.receipt_id.clone(),
            conversation_id: request.conversation_id.clone(),
            action_id: request.action_id.clone(),
            execution_id: request.execution_id.clone(),
            status: request.status,
            evidence_digest: request.evidence_digest.clone(),
            verifier_receipt_sha256: request.verifier_receipt_sha256.clone(),
            accepted_turn,
            prior_execution_status,
            resulting_execution_status,
            verified_outcome,
            semantic_authority: false,
            language_originated: false,
            audit_sha256: String::new(),
        };
        audit.audit_sha256 = action_evidence_audit_sha256(&audit);
        audit
    }

    pub fn validate(&self, completed_turns: u64) -> bool {
        let transition_valid = match self.status {
            ActionEvidenceStatusIR::ExecutionStarted => {
                self.prior_execution_status == ActionExecutionStatusIR::NotObserved
                    && self.resulting_execution_status == ActionExecutionStatusIR::InProgress
                    && !self.verified_outcome
            }
            ActionEvidenceStatusIR::Succeeded => {
                self.prior_execution_status == ActionExecutionStatusIR::InProgress
                    && self.resulting_execution_status == ActionExecutionStatusIR::Succeeded
                    && self.verified_outcome
            }
            ActionEvidenceStatusIR::Failed => {
                self.prior_execution_status == ActionExecutionStatusIR::InProgress
                    && self.resulting_execution_status == ActionExecutionStatusIR::Failed
                    && self.verified_outcome
            }
        };
        self.schema == ACTION_EVIDENCE_AUDIT_SCHEMA
            && valid_id(&self.receipt_id)
            && valid_id(&self.conversation_id)
            && valid_id(&self.action_id)
            && valid_id(&self.execution_id)
            && valid_digest(&self.evidence_digest)
            && valid_digest(&self.verifier_receipt_sha256)
            && self.verifier_receipt_sha256
                == action_evidence_receipt_sha256_fields(
                    &self.receipt_id,
                    &self.conversation_id,
                    &self.action_id,
                    &self.execution_id,
                    self.status,
                    &self.evidence_digest,
                )
            && self.accepted_turn > 0
            && self.accepted_turn <= completed_turns
            && transition_valid
            && !self.semantic_authority
            && !self.language_originated
            && valid_digest(&self.audit_sha256)
            && self.audit_sha256 == action_evidence_audit_sha256(self)
    }
}

pub fn action_evidence_audit_sha256(audit: &ActionEvidenceAuditIR) -> String {
    let bytes = serde_json::to_vec(&(
        ACTION_EVIDENCE_AUDIT_SCHEMA,
        audit.receipt_id.as_str(),
        audit.conversation_id.as_str(),
        audit.action_id.as_str(),
        audit.execution_id.as_str(),
        audit.status,
        audit.evidence_digest.as_str(),
        audit.verifier_receipt_sha256.as_str(),
        audit.accepted_turn,
        audit.prior_execution_status,
        audit.resulting_execution_status,
        audit.verified_outcome,
        audit.semantic_authority,
        audit.language_originated,
    ))
    .expect("bounded action evidence audit serializes");
    format!("{:x}", Sha256::digest(bytes))
}

pub fn action_evidence_receipt_sha256(request: &ActionEvidenceRequestIR) -> String {
    action_evidence_receipt_sha256_fields(
        &request.receipt_id,
        &request.conversation_id,
        &request.action_id,
        &request.execution_id,
        request.status,
        &request.evidence_digest,
    )
}

fn action_evidence_receipt_sha256_fields(
    receipt_id: &str,
    conversation_id: &str,
    action_id: &str,
    execution_id: &str,
    evidence_status: ActionEvidenceStatusIR,
    evidence_digest: &str,
) -> String {
    let status = match evidence_status {
        ActionEvidenceStatusIR::ExecutionStarted => "EXECUTION_STARTED",
        ActionEvidenceStatusIR::Succeeded => "SUCCEEDED",
        ActionEvidenceStatusIR::Failed => "FAILED",
    };
    let bytes = serde_json::to_vec(&(
        ACTION_EVIDENCE_REQUEST_SCHEMA,
        receipt_id,
        conversation_id,
        action_id,
        execution_id,
        status,
        evidence_digest,
    ))
    .expect("bounded action evidence serializes");
    format!("{:x}", Sha256::digest(bytes))
}

fn reported_status(text: &str) -> Option<ActionReportedStatusIR> {
    if contains_any(
        text,
        &[
            "실패",
            "못했",
            "끝내지 못",
            "막혔",
            "failed",
            "failure",
            "could not",
            "couldn't",
            "did not complete",
            "not complete",
        ],
    ) {
        return Some(ActionReportedStatusIR::FailureClaimed);
    }
    if contains_any(
        text,
        &[
            "실행 중",
            "처리하고 있",
            "돌리는 중",
            "진행 중",
            "working on",
            "running now",
            "underway",
            "still working",
        ],
    ) {
        return Some(ActionReportedStatusIR::InProgressClaimed);
    }
    if contains_any(
        text,
        &[
            "시도",
            "해 보긴",
            "해봤",
            "손은 대봤",
            "try",
            "tried",
            "attempt",
            "gave it a try",
        ],
    ) {
        return Some(ActionReportedStatusIR::Attempted);
    }
    contains_any(
        text,
        &[
            "끝났",
            "끝냈",
            "완료",
            "성공",
            "completed",
            "succeeded",
            "success",
            "all done",
            "are done",
            "is done",
            "finished",
            "wrapped up",
            "taken care of",
            "took care of",
            "through with",
            "마무리",
            "수리했",
            "수리 끝",
            "고쳤",
            "처리했",
            "처리 끝냈",
            "처리 끝났",
            "repaired",
            "fixed",
        ],
    )
    .then_some(ActionReportedStatusIR::SuccessClaimed)
}

fn is_action_state_query(text: &str) -> bool {
    if matches!(
        crate::proposition_content::requested_content_slot(text),
        Some(
            crate::proposition_content::ContentSlotIR::Cause
                | crate::proposition_content::ContentSlotIR::Agent
        )
    ) {
        return false;
    }
    let state_noun = contains_any(
        text,
        &[
            "결과",
            "상태",
            "실행",
            "완료",
            "성공",
            "검증",
            "result",
            "results",
            "status",
            "statuses",
            "state",
            "states",
            "execution",
            "outcome",
            "outcomes",
            "complete",
            "succeed",
            "verified",
            "report",
            "reported",
            "보고",
            "progress",
            "update",
            "진척",
            "진행",
            "현황",
            "상황",
        ],
    );
    let completion_question = text.ends_with('?')
        && contains_any(
            text,
            &[
                "끝났",
                "끝난",
                "완료됐",
                "수리했",
                "수리된",
                "고쳤",
                "됐어",
                "finished",
                "finish",
                "done",
                "completed",
                "complete",
                "repaired",
                "fixed",
                "succeed",
                "failed",
            ],
        );
    (state_noun
        && (text.ends_with('?')
            || contains_any(
                text,
                &[
                    "알려",
                    "보여",
                    "말해",
                    "정리해",
                    "show",
                    "give",
                    "tell",
                    "list",
                    "summarize",
                ],
            )))
        || contains_any(
            text,
            &[
                "what happened",
                "what has happened",
                "어떻게 됐",
                "상태가 뭐",
                "결과는?",
                "구분해서 말",
                "separate the reported",
                "coming along",
                "up to speed",
                "catch me up",
                "what's the progress",
                "what is the progress",
                "what remains",
                "remaining status",
                "remaining states",
                "어디까지",
                "어떻게 되어가",
                "진행은 어때",
                "남은 상태",
                "나머지 상태",
                "나머지 현황",
            ],
        )
        || (contains_any(text, &["where do", "where does"]) && text.contains("stand"))
        || completion_question
        || (text.ends_with('?')
            && contains_any(text, &["how is ", "how are "])
            && contains_any(text, &[" doing", " going"]))
}

fn is_untrusted_evidence_claim(text: &str) -> bool {
    contains_any(
        text,
        &["영수증", "터미널", "콘솔", "receipt", "terminal", "console"],
    ) && contains_any(
        text,
        &[
            "검증",
            "완료",
            "성공",
            "verified",
            "complete",
            "success",
            "succeeded",
        ],
    ) && !text.ends_with('?')
}

fn report_confidence(status: ActionReportedStatusIR) -> u16 {
    match status {
        ActionReportedStatusIR::Attempted => 900,
        ActionReportedStatusIR::InProgressClaimed => 920,
        ActionReportedStatusIR::SuccessClaimed | ActionReportedStatusIR::FailureClaimed => 940,
    }
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> ActionPlanSeedIR {
        ActionPlanSeedIR {
            action_id: "GOAL-1".to_string(),
            goal_id: "GOAL-1".to_string(),
            canonical_predicate: "REPAIR".to_string(),
            predicate_surface: "repair".to_string(),
            subject: "worker".to_string(),
            source_semantic_text: "repair the worker".to_string(),
            introduced_turn: 1,
            external_execution_authorized: true,
        }
    }

    fn request(status: ActionEvidenceStatusIR, suffix: &str) -> ActionEvidenceRequestIR {
        let mut request = ActionEvidenceRequestIR {
            schema: ACTION_EVIDENCE_REQUEST_SCHEMA.to_string(),
            receipt_id: format!("R-{suffix}"),
            conversation_id: "C-1".to_string(),
            action_id: "GOAL-1".to_string(),
            execution_id: "EXEC-1".to_string(),
            status,
            evidence_digest: format!("{:064x}", suffix.len()),
            verifier_receipt_sha256: String::new(),
        };
        request.verifier_receipt_sha256 = action_evidence_receipt_sha256(&request);
        request
    }

    #[test]
    fn language_success_claim_does_not_change_verified_execution() {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[seed()]);
        let analysis = ActionStateAnalyzer.analyze("I completed it", &ledger);
        let report = analysis.detected_report.expect("language report");
        assert!(ledger.apply_language_report(&report, 2));
        let record = ledger.record("GOAL-1").expect("record");
        assert_eq!(
            record.reported_status,
            Some(ActionReportedStatusIR::SuccessClaimed)
        );
        assert_eq!(
            record.execution_status,
            ActionExecutionStatusIR::NotObserved
        );
        assert!(!record.verified_outcome);
    }

    #[test]
    fn terminal_receipt_requires_a_matching_start_receipt() {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[seed()]);
        assert!(ledger
            .apply_evidence(&request(ActionEvidenceStatusIR::Succeeded, "END"), 1)
            .is_none());
        assert!(ledger
            .apply_evidence(
                &request(ActionEvidenceStatusIR::ExecutionStarted, "START"),
                1,
            )
            .is_some());
        assert!(ledger
            .apply_evidence(&request(ActionEvidenceStatusIR::Succeeded, "END"), 1)
            .is_some());
        assert!(ledger.record("GOAL-1").expect("record").verified_outcome);
    }

    #[test]
    fn language_report_audit_detects_payload_tampering() {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[seed()]);
        let analysis = ActionStateAnalyzer.analyze("I completed it", &ledger);
        assert!(ledger.apply_language_report(
            analysis.detected_report.as_ref().expect("language report"),
            2,
        ));
        assert!(ledger.validate(2));
        ledger.language_report_history[0]
            .source_surface
            .push_str(" and verified");
        assert!(!ledger.validate(2));
    }

    #[test]
    fn evidence_audit_binds_the_accepted_receipt_payload() {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[seed()]);
        assert!(ledger
            .apply_evidence(
                &request(ActionEvidenceStatusIR::ExecutionStarted, "START"),
                1,
            )
            .is_some());
        assert!(ledger.validate(1));
        ledger.evidence_audit_history[0].evidence_digest = "f".repeat(64);
        assert!(!ledger.validate(1));
    }

    #[test]
    fn punctuation_question_does_not_become_a_success_report() {
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[seed()]);
        let analysis = ActionStateAnalyzer.analyze("Did it succeed?", &ledger);
        assert!(analysis.query_requested);
        assert!(analysis.detected_report.is_none());
        assert!(!analysis.semantic_authority);
    }

    #[test]
    fn inherited_goal_disambiguates_same_predicate_action_reports() {
        let first = seed();
        let mut second = seed();
        second.action_id = "GOAL-2".to_string();
        second.goal_id = "GOAL-2".to_string();
        second.subject = "queue".to_string();
        second.source_semantic_text = "repair the queue".to_string();

        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[first, second]);
        let analysis =
            ActionStateAnalyzer.analyze_with_goal_hint("I completed it", &ledger, Some("GOAL-2"));
        assert_eq!(analysis.target_action_ids, vec!["GOAL-2"]);
        assert!(analysis.unresolved_ambiguities.is_empty());
        let report = analysis.detected_report.expect("goal-bound report");
        assert_eq!(report.action_id, "GOAL-2");
        assert!(ledger.apply_language_report(&report, 2));
        assert_eq!(
            ledger
                .record("GOAL-1")
                .expect("first action")
                .reported_status,
            None
        );
        assert_eq!(
            ledger
                .record("GOAL-2")
                .expect("second action")
                .reported_status,
            Some(ActionReportedStatusIR::SuccessClaimed)
        );
    }

    #[test]
    fn plural_goal_hints_create_separate_unverified_language_reports() {
        let first = seed();
        let mut second = seed();
        second.action_id = "GOAL-2".to_string();
        second.goal_id = "GOAL-2".to_string();
        second.subject = "queue".to_string();
        second.source_semantic_text = "repair the queue".to_string();

        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[first, second]);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints(
            "both actions are done",
            &ledger,
            &["GOAL-1", "GOAL-2"],
        );

        assert_eq!(analysis.target_action_ids, vec!["GOAL-1", "GOAL-2"]);
        assert_eq!(analysis.language_reports().len(), 2);
        assert!(analysis.detected_report.is_none());
        assert!(!analysis.external_action_executed);
        for report in analysis.language_reports() {
            assert!(ledger.apply_language_report(report, 2));
        }
        assert!(ledger.records.iter().all(|record| {
            record.reported_status == Some(ActionReportedStatusIR::SuccessClaimed)
                && record.execution_status == ActionExecutionStatusIR::NotObserved
                && !record.verified_outcome
        }));
    }

    #[test]
    fn state_noun_in_an_information_request_selects_all_bound_actions() {
        let first = seed();
        let mut second = seed();
        second.action_id = "GOAL-2".to_string();
        second.goal_id = "GOAL-2".to_string();
        second.subject = "queue".to_string();
        second.source_semantic_text = "repair the queue".to_string();

        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[first, second]);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints(
            "show the state of both tasks",
            &ledger,
            &["GOAL-1", "GOAL-2"],
        );

        assert!(analysis.query_requested);
        assert_eq!(analysis.target_action_ids, vec!["GOAL-1", "GOAL-2"]);
        assert!(analysis.unresolved_ambiguities.is_empty());
        assert!(analysis.language_reports().is_empty());
        assert!(!analysis.external_action_executed);
    }

    #[test]
    fn typed_historical_goal_hints_can_query_superseded_plans_without_execution_authority() {
        let first = seed();
        let mut second = seed();
        second.action_id = "GOAL-2".to_string();
        second.goal_id = "GOAL-2".to_string();
        second.subject = "queue".to_string();
        second.source_semantic_text = "repair the queue".to_string();

        let mut replacement = seed();
        replacement.action_id = "GOAL-3".to_string();
        replacement.goal_id = "GOAL-3".to_string();
        replacement.subject = "server".to_string();
        replacement.source_semantic_text = "repair the server".to_string();
        replacement.introduced_turn = 2;

        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[first, second]);
        ledger.replace_active_plans(&[replacement], 2);

        let analysis = ActionStateAnalyzer.analyze_with_goal_hints(
            "show the state of the first task pair",
            &ledger,
            &["GOAL-1", "GOAL-2"],
        );

        assert!(analysis.query_requested);
        assert_eq!(analysis.target_action_ids, vec!["GOAL-1", "GOAL-2"]);
        assert!(analysis.unresolved_ambiguities.is_empty());
        assert!(analysis.language_reports().is_empty());
        assert!(!analysis.semantic_authority);
        assert!(!analysis.external_action_executed);
        assert!(ledger.records[..2].iter().all(|record| {
            record.plan_status == ActionPlanStatusIR::Superseded
                && record.execution_status == ActionExecutionStatusIR::NotObserved
                && !record.verified_outcome
        }));
    }

    #[test]
    fn completion_and_progress_paraphrases_remain_structurally_distinct() {
        let first = seed();
        let mut second = seed();
        second.action_id = "GOAL-2".to_string();
        second.goal_id = "GOAL-2".to_string();
        second.subject = "queue".to_string();

        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[first, second]);
        let ids = ["GOAL-1", "GOAL-2"];

        let completion =
            ActionStateAnalyzer.analyze_with_goal_hints("I took care of both tasks", &ledger, &ids);
        assert!(!completion.query_requested);
        assert_eq!(completion.language_reports().len(), 2);
        assert!(completion.language_reports().iter().all(|report| {
            report.reported_status == ActionReportedStatusIR::SuccessClaimed
                && !report.semantic_authority
                && !report.external_action_executed
        }));

        let query = ActionStateAnalyzer.analyze_with_goal_hints(
            "Where do both tasks stand?",
            &ledger,
            &ids,
        );
        assert!(query.query_requested);
        assert!(query.language_reports().is_empty());
        assert!(!query.external_action_executed);
    }

    fn three_action_ledger() -> ActionStateLedgerIR {
        let mut worker = seed();
        worker.subject = "worker".to_string();
        let mut queue = seed();
        queue.action_id = "GOAL-2".to_string();
        queue.goal_id = "GOAL-2".to_string();
        queue.subject = "queue".to_string();
        queue.source_semantic_text = "repair the queue".to_string();
        let mut cache = seed();
        cache.action_id = "GOAL-3".to_string();
        cache.goal_id = "GOAL-3".to_string();
        cache.subject = "cache".to_string();
        cache.source_semantic_text = "inspect the cache".to_string();
        let mut ledger = ActionStateLedgerIR::default();
        ledger.add_plans(&[worker, queue, cache]);
        ledger
    }

    #[test]
    fn query_surface_filters_a_resolved_group_without_treating_inserted_members_as_selectors() {
        let ledger = three_action_ledger();
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show the state of worker and queue and cache",
            "among all tasks show only the cache action's status",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let query = analysis.set_query.as_ref().expect("typed set query");
        assert_eq!(
            query.operator_trace,
            vec![ActionSetOperatorIR::Intersection]
        );
        assert_eq!(analysis.target_action_ids, vec!["GOAL-3"]);
        assert!(query.validate());
    }

    #[test]
    fn complement_scope_can_remove_two_members_from_a_three_action_group() {
        let ledger = three_action_ledger();
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "exclude both worker and queue from all tasks and show what remains",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let query = analysis.set_query.as_ref().expect("typed complement");
        assert_eq!(query.operator_trace, vec![ActionSetOperatorIR::Complement]);
        assert_eq!(query.excluded_action_ids, vec!["GOAL-1", "GOAL-2"]);
        assert_eq!(analysis.target_action_ids, vec!["GOAL-3"]);
        assert!(analysis.unresolved_ambiguities.is_empty());
    }

    #[test]
    fn reported_completion_quantifier_does_not_become_verified_execution() {
        let mut ledger = three_action_ledger();
        ledger.records[1].reported_status = Some(ActionReportedStatusIR::SuccessClaimed);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "do worker, queue, and cache have a reported completion status?",
            "do any of all tasks have a reported completion status?",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let query = analysis.set_query.as_ref().expect("quantified query");
        assert_eq!(query.quantifier, Some(ActionSetQuantifierIR::Any));
        assert_eq!(
            query.predicate,
            Some(ActionStatePredicateIR::ReportedCompletion)
        );
        assert_eq!(query.truth, ActionSetTruthIR::True);
        assert!(ledger.records.iter().all(|record| {
            record.execution_status == ActionExecutionStatusIR::NotObserved
                && !record.verified_outcome
        }));
    }

    #[test]
    fn empty_composed_action_set_fails_closed_and_cannot_gain_authority() {
        let ledger = three_action_ledger();
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "show all tasks except worker, queue, and cache",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let mut query = analysis.set_query.expect("empty typed set");
        assert!(analysis.target_action_ids.is_empty());
        assert!(analysis
            .unresolved_ambiguities
            .iter()
            .any(|item| item.contains("EMPTY_ACTION_SET")));
        query.semantic_authority = true;
        query.query_sha256 = action_set_query_sha256(&query);
        assert!(!query.validate());
    }

    #[test]
    fn quantified_report_question_cannot_create_new_language_reports() {
        let mut ledger = three_action_ledger();
        ledger.records[1].reported_status = Some(ActionReportedStatusIR::SuccessClaimed);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "were worker, queue, or cache reported as finished?",
            "was at least one of all tasks reported as finished?",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let query = analysis.set_query.as_ref().expect("report-state query");
        assert!(analysis.query_requested);
        assert!(analysis.language_reports().is_empty());
        assert_eq!(query.quantifier, Some(ActionSetQuantifierIR::Any));
        assert_eq!(
            query.predicate,
            Some(ActionStatePredicateIR::ReportedCompletion)
        );
        assert_eq!(query.truth, ActionSetTruthIR::True);
        assert_eq!(
            ledger
                .records
                .iter()
                .filter(|record| record.reported_status.is_some())
                .count(),
            1
        );
    }

    #[test]
    fn recursive_action_expression_preserves_parenthesized_precedence() {
        let ledger = three_action_ledger();
        let grouped = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show (cache or queue) except cache",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert_eq!(grouped.target_action_ids, vec!["GOAL-2"]);
        assert!(matches!(
            grouped.set_query.as_ref().and_then(|query| query.expression.as_ref()),
            Some(ActionSetExpressionIR::Difference { left, .. })
                if matches!(left.as_ref(), ActionSetExpressionIR::Union { .. })
        ));

        let nested = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show cache or (queue except queue)",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert_eq!(nested.target_action_ids, vec!["GOAL-3"]);
        assert!(matches!(
            nested.set_query.as_ref().and_then(|query| query.expression.as_ref()),
            Some(ActionSetExpressionIR::Union { right, .. })
                if matches!(right.as_ref(), ActionSetExpressionIR::Difference { .. })
        ));
    }

    #[test]
    fn recursive_complement_scope_excludes_the_group_inside_parentheses() {
        let ledger = three_action_ledger();
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show not (cache or queue)",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert_eq!(analysis.target_action_ids, vec!["GOAL-1"]);
        assert!(matches!(
            analysis.set_query.as_ref().and_then(|query| query.expression.as_ref()),
            Some(ActionSetExpressionIR::Complement { excluded, .. })
                if matches!(excluded.as_ref(), ActionSetExpressionIR::Union { .. })
        ));
    }

    #[test]
    fn recursive_relative_filter_uses_report_state_without_verifying_execution() {
        let mut ledger = three_action_ledger();
        ledger.records[1].reported_status = Some(ActionReportedStatusIR::SuccessClaimed);
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show (cache or worker) that were not reported complete",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert_eq!(analysis.target_action_ids, vec!["GOAL-1", "GOAL-3"]);
        assert!(matches!(
            analysis.set_query.as_ref().and_then(|query| query.expression.as_ref()),
            Some(ActionSetExpressionIR::Intersection { left, right, .. })
                if matches!(left.as_ref(), ActionSetExpressionIR::Union { .. })
                    && matches!(right.as_ref(), ActionSetExpressionIR::StatePredicate {
                        predicate: ActionStatePredicateIR::ReportedCompletion,
                        negated: true,
                        ..
                    })
        ));
        assert!(ledger.records.iter().all(|record| {
            record.execution_status == ActionExecutionStatusIR::NotObserved
                && !record.verified_outcome
        }));
    }

    #[test]
    fn malformed_or_tampered_recursive_expression_fails_closed() {
        let ledger = three_action_ledger();
        let malformed = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show (cache or queue",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert!(malformed.target_action_ids.is_empty());
        assert!(malformed
            .unresolved_ambiguities
            .iter()
            .any(|item| item.contains("MALFORMED_ACTION_SET_PARENTHESES")));

        let valid = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "show worker, queue, and cache state",
            "from all tasks show (cache or queue) except cache",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        let mut query = valid.set_query.expect("recursive query");
        let expression = query.expression.as_mut().expect("recursive expression");
        if let ActionSetExpressionIR::Difference { action_ids, .. } = expression {
            action_ids.push("GOAL-3".to_string());
        } else {
            panic!("expected difference expression");
        }
        query.query_sha256 = action_set_query_sha256(&query);
        assert!(!query.validate());
    }

    #[test]
    fn idiomatic_how_is_group_doing_is_an_action_state_query() {
        let ledger = three_action_ledger();
        let analysis = ActionStateAnalyzer.analyze_with_goal_hints_and_query_surface(
            "how is the actions inspect cache and repair queue and analyze worker doing?",
            "How is that task group doing?",
            &ledger,
            &["GOAL-1", "GOAL-2", "GOAL-3"],
        );
        assert!(analysis.query_requested);
        assert_eq!(
            analysis.target_action_ids,
            vec!["GOAL-1", "GOAL-2", "GOAL-3"]
        );
        assert!(analysis.unresolved_ambiguities.is_empty());
    }
}
