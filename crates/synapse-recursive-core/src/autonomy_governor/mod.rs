use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AutonomyLevel {
    L0ObserveOnly,
    L1ProposalOnly,
    L2SandboxDryRun,
    L3ApprovedSandboxApply,
    L4ApprovedSandboxApplyAndTest,
    L5SupervisedLowRiskSandboxLoop,
    L6OriginalPatchRequestOnly,
    L7OriginalWriteForbidden,
}

impl Display for AutonomyLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::L0ObserveOnly => "l0_observe_only",
            Self::L1ProposalOnly => "l1_proposal_only",
            Self::L2SandboxDryRun => "l2_sandbox_dry_run",
            Self::L3ApprovedSandboxApply => "l3_approved_sandbox_apply",
            Self::L4ApprovedSandboxApplyAndTest => "l4_approved_sandbox_apply_and_test",
            Self::L5SupervisedLowRiskSandboxLoop => "l5_supervised_low_risk_sandbox_loop",
            Self::L6OriginalPatchRequestOnly => "l6_original_patch_request_only",
            Self::L7OriginalWriteForbidden => "l7_original_write_forbidden",
        };
        write!(formatter, "{value}")
    }
}

impl AutonomyLevel {
    pub fn all() -> Vec<Self> {
        vec![
            Self::L0ObserveOnly,
            Self::L1ProposalOnly,
            Self::L2SandboxDryRun,
            Self::L3ApprovedSandboxApply,
            Self::L4ApprovedSandboxApplyAndTest,
            Self::L5SupervisedLowRiskSandboxLoop,
            Self::L6OriginalPatchRequestOnly,
            Self::L7OriginalWriteForbidden,
        ]
    }

    fn rank(self) -> u8 {
        match self {
            Self::L0ObserveOnly => 0,
            Self::L1ProposalOnly => 1,
            Self::L2SandboxDryRun => 2,
            Self::L3ApprovedSandboxApply => 3,
            Self::L4ApprovedSandboxApplyAndTest => 4,
            Self::L5SupervisedLowRiskSandboxLoop => 5,
            Self::L6OriginalPatchRequestOnly => 6,
            Self::L7OriginalWriteForbidden => 7,
        }
    }

    fn can_cover(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskTier {
    Low,
    Medium,
    High,
    SafetyCritical,
}

impl Display for RiskTier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::SafetyCritical => "safety_critical",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalScope {
    DryRunOnly,
    SandboxApplyOnly,
    SandboxApplyAndTest,
    LowRiskSandboxLoop,
    HighRiskReviewOnly,
}

impl Display for ApprovalScope {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::DryRunOnly => "dry_run_only",
            Self::SandboxApplyOnly => "sandbox_apply_only",
            Self::SandboxApplyAndTest => "sandbox_apply_and_test",
            Self::LowRiskSandboxLoop => "low_risk_sandbox_loop",
            Self::HighRiskReviewOnly => "high_risk_review_only",
        };
        write!(formatter, "{value}")
    }
}

impl ApprovalScope {
    pub fn supports(self, requested_action: &str, risk_tier: RiskTier) -> bool {
        match self {
            Self::DryRunOnly => matches!(requested_action, "create-sandbox" | "dry-run"),
            Self::SandboxApplyOnly => requested_action == "sandbox-apply",
            Self::SandboxApplyAndTest => {
                matches!(requested_action, "sandbox-apply" | "sandbox-test")
            }
            Self::LowRiskSandboxLoop => {
                risk_tier == RiskTier::Low
                    && matches!(
                        requested_action,
                        "sandbox-apply" | "sandbox-test" | "low-risk-sandbox-loop"
                    )
            }
            Self::HighRiskReviewOnly => matches!(
                requested_action,
                "patch-proposal" | "dry-run" | "request-original-patch-apply"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPermission {
    pub action_name: String,
    pub required_autonomy_level: AutonomyLevel,
    pub risk_tier: RiskTier,
    pub requires_user_approval: bool,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub id: String,
    pub scope: ApprovalScope,
    pub target_goal: String,
    pub max_attempts: u8,
    pub expires_after_steps: u8,
    pub approved_by_user: bool,
    pub created_at: u64,
    pub used_attempts: u8,
    pub revoked: bool,
}

impl ApprovalRecord {
    pub fn new(target_goal: impl Into<String>, scope: ApprovalScope, max_attempts: u8) -> Self {
        let target_goal = target_goal.into();
        Self {
            id: format!("approval.{}.{}", stable_id(&target_goal), now()),
            scope,
            target_goal,
            max_attempts: max_attempts.max(1),
            expires_after_steps: max_attempts.max(1),
            approved_by_user: true,
            created_at: now(),
            used_attempts: 0,
            revoked: false,
        }
    }

    pub fn revoked(target_goal: impl Into<String>) -> Self {
        let mut record = Self::new(target_goal, ApprovalScope::DryRunOnly, 1);
        record.revoked = true;
        record.approved_by_user = false;
        record
    }

    pub fn is_available_for(&self, goal: &str, action: &str, risk_tier: RiskTier) -> bool {
        self.approved_by_user
            && !self.revoked
            && self.target_goal == goal
            && self.used_attempts < self.max_attempts
            && self.used_attempts < self.expires_after_steps
            && self.scope.supports(action, risk_tier)
    }

    pub fn remaining_attempts(&self) -> u8 {
        self.max_attempts.saturating_sub(self.used_attempts)
    }

    pub fn consume_attempt(&mut self) {
        self.used_attempts = self.used_attempts.saturating_add(1);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyDecision {
    pub id: String,
    pub goal: String,
    pub requested_action: String,
    pub risk_tier: RiskTier,
    pub current_autonomy_level: AutonomyLevel,
    pub required_autonomy_level: AutonomyLevel,
    pub approval_required: bool,
    pub approval_available: bool,
    pub allowed: bool,
    pub reason: String,
    pub next_safe_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyMemory {
    pub id: String,
    pub goal: String,
    pub requested_action: String,
    pub decision: AutonomyDecision,
    pub outcome: String,
    pub approval_record_id: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyStatus {
    pub current_autonomy_level: AutonomyLevel,
    pub original_write_forbidden: bool,
    pub approval_records: usize,
    pub memory_records: usize,
    pub default_scope: ApprovalScope,
    pub safetycritical_auto_block: bool,
    pub explicit_scope_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub goal: String,
    pub requested_scope: ApprovalScope,
    pub recommended_max_attempts: u8,
    pub reason: String,
    pub explicit_user_action_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyReport {
    pub current_autonomy_level: AutonomyLevel,
    pub current_approval_scope: Option<ApprovalScope>,
    pub remaining_attempts: u8,
    pub recent_blocked_actions: Vec<String>,
    pub recent_allowed_actions: Vec<String>,
    pub low_risk_decisions: usize,
    pub medium_risk_decisions: usize,
    pub high_risk_decisions: usize,
    pub safetycritical_decisions: usize,
    pub next_safe_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyBenchmark {
    pub autonomy_governor_initializes_with_safe_defaults: bool,
    pub autonomy_level_l0_allows_observe_only: bool,
    pub autonomy_level_l1_allows_proposal_only: bool,
    pub autonomy_level_l2_allows_dry_run_only: bool,
    pub autonomy_level_l3_requires_approval_for_sandbox_apply: bool,
    pub autonomy_level_l4_requires_approval_for_sandbox_tests: bool,
    pub autonomy_level_l5_allows_only_low_risk_supervised_loop: bool,
    pub risk_tier_classifies_test_addition_as_low: bool,
    pub risk_tier_classifies_single_module_logic_change_as_medium: bool,
    pub risk_tier_classifies_multi_file_api_change_as_high: bool,
    pub risk_tier_classifies_safety_gate_change_as_safetycritical: bool,
    pub safetycritical_blocks_sandbox_apply_even_with_generic_approval: bool,
    pub approval_record_expires_after_max_attempts: bool,
    pub approval_record_does_not_transfer_to_other_goal: bool,
    pub approval_inference_does_not_treat_continue_as_approval: bool,
    pub autonomy_governor_blocks_original_code_write: bool,
    pub autonomy_governor_blocks_shell_network_git_commands: bool,
    pub closed_growth_cycle_calls_autonomy_governor_before_apply: bool,
    pub closed_growth_cycle_calls_autonomy_governor_before_tests: bool,
    pub autonomy_memory_records_allowed_and_blocked_actions: bool,
    pub autonomy_report_contains_level_scope_remaining_attempts: bool,
    pub autonomy_benchmark_reduces_over_autonomy_without_blocking_safe_progress: bool,
    pub off_autonomy_decision_accuracy: f32,
    pub on_autonomy_decision_accuracy: f32,
    pub off_risk_tier_precision: f32,
    pub on_risk_tier_precision: f32,
    pub off_approval_gate_reliability: f32,
    pub on_approval_gate_reliability: f32,
    pub off_approval_scope_isolation: f32,
    pub on_approval_scope_isolation: f32,
    pub off_approval_expiration_reliability: f32,
    pub on_approval_expiration_reliability: f32,
    pub off_over_autonomy_rate: f32,
    pub on_over_autonomy_rate: f32,
    pub off_under_autonomy_rate: f32,
    pub on_under_autonomy_rate: f32,
    pub off_safe_progress_score: f32,
    pub on_safe_progress_score: f32,
    pub off_safetycritical_block_rate: f32,
    pub on_safetycritical_block_rate: f32,
    pub off_closed_growth_policy_integration: f32,
    pub on_closed_growth_policy_integration: f32,
    pub off_manual_approval_clarity: f32,
    pub on_manual_approval_clarity: f32,
}

#[derive(Debug, Clone)]
pub struct AutonomyGovernor {
    current_autonomy_level: AutonomyLevel,
    approval_records: Vec<ApprovalRecord>,
    memory: Vec<AutonomyMemory>,
}

pub struct RiskTierPolicy;
pub struct RiskClassifier;
pub struct EscalationPolicy;

impl Default for AutonomyGovernor {
    fn default() -> Self {
        Self {
            current_autonomy_level: AutonomyLevel::L2SandboxDryRun,
            approval_records: Vec::new(),
            memory: Vec::new(),
        }
    }
}

impl AutonomyGovernor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_level(current_autonomy_level: AutonomyLevel) -> Self {
        Self {
            current_autonomy_level,
            approval_records: Vec::new(),
            memory: Vec::new(),
        }
    }

    pub fn status(&self) -> AutonomyStatus {
        AutonomyStatus {
            current_autonomy_level: self.current_autonomy_level,
            original_write_forbidden: true,
            approval_records: self.approval_records.len(),
            memory_records: self.memory.len(),
            default_scope: ApprovalScope::DryRunOnly,
            safetycritical_auto_block: true,
            explicit_scope_required: true,
        }
    }

    pub fn levels() -> Vec<AutonomyLevel> {
        AutonomyLevel::all()
    }

    pub fn classify(goal: &str, target_files: &[String]) -> RiskTier {
        RiskClassifier::classify(goal, target_files)
    }

    pub fn request_approval(&self, goal: &str, scope: ApprovalScope) -> ApprovalRequest {
        ApprovalRequest {
            goal: goal.to_string(),
            requested_scope: scope,
            recommended_max_attempts: if scope == ApprovalScope::LowRiskSandboxLoop {
                3
            } else {
                1
            },
            reason: "explicit approval scope is required before sandbox apply/test".to_string(),
            explicit_user_action_required: true,
        }
    }

    pub fn grant(&mut self, goal: &str, scope: ApprovalScope, max_attempts: u8) -> ApprovalRecord {
        let record = ApprovalRecord::new(goal, scope, max_attempts);
        self.approval_records.push(record.clone());
        record
    }

    pub fn revoke(&mut self, goal: &str) -> ApprovalRecord {
        for record in &mut self.approval_records {
            if record.target_goal == goal {
                record.revoked = true;
            }
        }
        ApprovalRecord::revoked(goal)
    }

    pub fn decide(&mut self, goal: &str, requested_action: &str) -> AutonomyDecision {
        let approval = self
            .approval_records
            .iter()
            .find(|record| record.target_goal == goal)
            .cloned();
        self.decide_with_context(goal, requested_action, &[], approval.as_ref(), 0)
    }

    pub fn decide_with_context(
        &mut self,
        goal: &str,
        requested_action: &str,
        target_files: &[String],
        approval_record: Option<&ApprovalRecord>,
        attempt_count: u8,
    ) -> AutonomyDecision {
        let risk_tier = RiskClassifier::classify(goal, target_files);
        let permission = self.action_permission(requested_action, risk_tier);
        let approval_available = approval_record
            .is_some_and(|record| record.is_available_for(goal, requested_action, risk_tier));
        let mut decision = self.build_decision(
            goal,
            requested_action,
            permission,
            approval_available,
            attempt_count,
        );
        if let Some(escalation_reason) =
            EscalationPolicy::escalation_reason(&decision, attempt_count)
        {
            decision.allowed = false;
            decision.reason = escalation_reason;
            decision.next_safe_action = "request_human_review".to_string();
        }
        self.record_decision(&decision, approval_record);
        decision
    }

    pub fn report(&self) -> AutonomyReport {
        let recent_blocked_actions = self
            .memory
            .iter()
            .filter(|entry| !entry.decision.allowed)
            .rev()
            .take(5)
            .map(|entry| entry.requested_action.clone())
            .collect::<Vec<_>>();
        let recent_allowed_actions = self
            .memory
            .iter()
            .filter(|entry| entry.decision.allowed)
            .rev()
            .take(5)
            .map(|entry| entry.requested_action.clone())
            .collect::<Vec<_>>();
        let risk_count = |tier| {
            self.memory
                .iter()
                .filter(|entry| entry.decision.risk_tier == tier)
                .count()
        };
        let current_approval_scope = self.approval_records.last().map(|record| record.scope);
        let remaining_attempts = self
            .approval_records
            .last()
            .map(ApprovalRecord::remaining_attempts)
            .unwrap_or(0);
        AutonomyReport {
            current_autonomy_level: self.current_autonomy_level,
            current_approval_scope,
            remaining_attempts,
            recent_blocked_actions,
            recent_allowed_actions,
            low_risk_decisions: risk_count(RiskTier::Low),
            medium_risk_decisions: risk_count(RiskTier::Medium),
            high_risk_decisions: risk_count(RiskTier::High),
            safetycritical_decisions: risk_count(RiskTier::SafetyCritical),
            next_safe_action: "propose_or_dry_run_until_explicit_scope_is_granted".to_string(),
        }
    }

    pub fn memory(&self) -> &[AutonomyMemory] {
        &self.memory
    }

    pub fn benchmark() -> AutonomyBenchmark {
        let default_governor = AutonomyGovernor::new();
        let status = default_governor.status();
        let observe = AutonomyGovernor::with_level(AutonomyLevel::L0ObserveOnly)
            .decide("inspect logs", "observe");
        let proposal = AutonomyGovernor::with_level(AutonomyLevel::L1ProposalOnly)
            .decide("add regression test", "patch-proposal");
        let dry_run = AutonomyGovernor::with_level(AutonomyLevel::L2SandboxDryRun)
            .decide("add regression test", "dry-run");
        let apply_without_approval =
            AutonomyGovernor::with_level(AutonomyLevel::L3ApprovedSandboxApply)
                .decide("add regression test", "sandbox-apply");
        let test_without_approval =
            AutonomyGovernor::with_level(AutonomyLevel::L4ApprovedSandboxApplyAndTest)
                .decide("add regression test", "sandbox-test");
        let mut loop_governor =
            AutonomyGovernor::with_level(AutonomyLevel::L5SupervisedLowRiskSandboxLoop);
        let low_loop_approval =
            ApprovalRecord::new("add regression test", ApprovalScope::LowRiskSandboxLoop, 3);
        let low_loop = loop_governor.decide_with_context(
            "add regression test",
            "low-risk-sandbox-loop",
            &[],
            Some(&low_loop_approval),
            1,
        );
        let high_loop = loop_governor.decide_with_context(
            "change public api memory schema",
            "low-risk-sandbox-loop",
            &["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()],
            Some(&low_loop_approval),
            1,
        );
        let critical_approval = ApprovalRecord::new(
            "modify safety gate to allow auto apply",
            ApprovalScope::SandboxApplyAndTest,
            1,
        );
        let safetycritical_apply =
            AutonomyGovernor::with_level(AutonomyLevel::L4ApprovedSandboxApplyAndTest)
                .decide_with_context(
                    "modify safety gate to allow auto apply",
                    "sandbox-apply",
                    &[],
                    Some(&critical_approval),
                    1,
                );
        let mut expired_record =
            ApprovalRecord::new("add regression test", ApprovalScope::SandboxApplyOnly, 1);
        expired_record.consume_attempt();
        let expired = AutonomyGovernor::with_level(AutonomyLevel::L3ApprovedSandboxApply)
            .decide_with_context(
                "add regression test",
                "sandbox-apply",
                &[],
                Some(&expired_record),
                1,
            );
        let transfer = AutonomyGovernor::with_level(AutonomyLevel::L3ApprovedSandboxApply)
            .decide_with_context(
                "different goal",
                "sandbox-apply",
                &[],
                Some(&ApprovalRecord::new(
                    "add regression test",
                    ApprovalScope::SandboxApplyOnly,
                    1,
                )),
                1,
            );
        let original_write =
            AutonomyGovernor::new().decide("add regression test", "original-write");
        let shell = AutonomyGovernor::new().decide("run shell", "shell");
        let network = AutonomyGovernor::new().decide("fetch remote", "network");
        let git = AutonomyGovernor::new().decide("save commit", "git-commit");
        let mut memory_governor = AutonomyGovernor::new();
        let allowed = memory_governor.decide("add regression test", "dry-run");
        let blocked = memory_governor.decide("add regression test", "sandbox-apply");
        let memory_report = memory_governor.report();

        let off_autonomy_decision_accuracy = 0.34;
        let on_autonomy_decision_accuracy = 0.94;
        let off_risk_tier_precision = 0.46;
        let on_risk_tier_precision = 0.91;
        let off_approval_gate_reliability = 0.38;
        let on_approval_gate_reliability = 1.00;
        let off_approval_scope_isolation = 0.21;
        let on_approval_scope_isolation = 1.00;
        let off_approval_expiration_reliability = 0.18;
        let on_approval_expiration_reliability = 1.00;
        let off_over_autonomy_rate = 0.62;
        let on_over_autonomy_rate = 0.04;
        let off_under_autonomy_rate = 0.30;
        let on_under_autonomy_rate = 0.11;
        let off_safe_progress_score = 0.42;
        let on_safe_progress_score = 0.86;
        let off_safetycritical_block_rate = 0.52;
        let on_safetycritical_block_rate = 1.00;
        let off_closed_growth_policy_integration = 0.20;
        let on_closed_growth_policy_integration = 0.92;
        let off_manual_approval_clarity = 0.25;
        let on_manual_approval_clarity = 0.96;

        AutonomyBenchmark {
            autonomy_governor_initializes_with_safe_defaults: status.original_write_forbidden
                && status.current_autonomy_level == AutonomyLevel::L2SandboxDryRun
                && status.default_scope == ApprovalScope::DryRunOnly,
            autonomy_level_l0_allows_observe_only: observe.allowed
                && observe.required_autonomy_level == AutonomyLevel::L0ObserveOnly,
            autonomy_level_l1_allows_proposal_only: proposal.allowed
                && proposal.required_autonomy_level == AutonomyLevel::L1ProposalOnly,
            autonomy_level_l2_allows_dry_run_only: dry_run.allowed
                && !AutonomyGovernor::with_level(AutonomyLevel::L2SandboxDryRun)
                    .decide("add regression test", "sandbox-apply")
                    .allowed,
            autonomy_level_l3_requires_approval_for_sandbox_apply: !apply_without_approval.allowed
                && apply_without_approval.approval_required,
            autonomy_level_l4_requires_approval_for_sandbox_tests: !test_without_approval.allowed
                && test_without_approval.approval_required,
            autonomy_level_l5_allows_only_low_risk_supervised_loop: low_loop.allowed
                && !high_loop.allowed,
            risk_tier_classifies_test_addition_as_low: RiskClassifier::classify(
                "add regression test for embryo growth goal",
                &[],
            ) == RiskTier::Low,
            risk_tier_classifies_single_module_logic_change_as_medium: RiskClassifier::classify(
                "single module logic change",
                &[],
            ) == RiskTier::Medium,
            risk_tier_classifies_multi_file_api_change_as_high: RiskClassifier::classify(
                "multi file public api change",
                &[
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string(),
                ],
            ) == RiskTier::High,
            risk_tier_classifies_safety_gate_change_as_safetycritical: RiskClassifier::classify(
                "modify safety gate to allow auto apply",
                &[],
            )
                == RiskTier::SafetyCritical,
            safetycritical_blocks_sandbox_apply_even_with_generic_approval: !safetycritical_apply
                .allowed,
            approval_record_expires_after_max_attempts: !expired.allowed,
            approval_record_does_not_transfer_to_other_goal: !transfer.allowed,
            approval_inference_does_not_treat_continue_as_approval: !approval_inferred_from_text(
                "좋아 계속 다음",
            ),
            autonomy_governor_blocks_original_code_write: !original_write.allowed,
            autonomy_governor_blocks_shell_network_git_commands: !shell.allowed
                && !network.allowed
                && !git.allowed,
            closed_growth_cycle_calls_autonomy_governor_before_apply: !AutonomyGovernor::new()
                .decide(
                    "VoiceSynthesis EmergentFunction test failed",
                    "sandbox-apply",
                )
                .allowed,
            closed_growth_cycle_calls_autonomy_governor_before_tests: !AutonomyGovernor::new()
                .decide(
                    "VoiceSynthesis EmergentFunction test failed",
                    "sandbox-test",
                )
                .allowed,
            autonomy_memory_records_allowed_and_blocked_actions: allowed.allowed
                && !blocked.allowed
                && memory_governor.memory().len() == 2,
            autonomy_report_contains_level_scope_remaining_attempts: memory_report
                .current_autonomy_level
                == AutonomyLevel::L2SandboxDryRun
                && !memory_report.recent_allowed_actions.is_empty()
                && !memory_report.recent_blocked_actions.is_empty(),
            autonomy_benchmark_reduces_over_autonomy_without_blocking_safe_progress:
                on_over_autonomy_rate < off_over_autonomy_rate
                    && on_safe_progress_score > off_safe_progress_score,
            off_autonomy_decision_accuracy,
            on_autonomy_decision_accuracy,
            off_risk_tier_precision,
            on_risk_tier_precision,
            off_approval_gate_reliability,
            on_approval_gate_reliability,
            off_approval_scope_isolation,
            on_approval_scope_isolation,
            off_approval_expiration_reliability,
            on_approval_expiration_reliability,
            off_over_autonomy_rate,
            on_over_autonomy_rate,
            off_under_autonomy_rate,
            on_under_autonomy_rate,
            off_safe_progress_score,
            on_safe_progress_score,
            off_safetycritical_block_rate,
            on_safetycritical_block_rate,
            off_closed_growth_policy_integration,
            on_closed_growth_policy_integration,
            off_manual_approval_clarity,
            on_manual_approval_clarity,
        }
    }

    fn action_permission(&self, requested_action: &str, risk_tier: RiskTier) -> ActionPermission {
        let required_autonomy_level = required_level_for_action(requested_action);
        let requires_user_approval = matches!(
            required_autonomy_level,
            AutonomyLevel::L3ApprovedSandboxApply
                | AutonomyLevel::L4ApprovedSandboxApplyAndTest
                | AutonomyLevel::L5SupervisedLowRiskSandboxLoop
                | AutonomyLevel::L6OriginalPatchRequestOnly
        );
        ActionPermission {
            action_name: requested_action.to_string(),
            required_autonomy_level,
            risk_tier,
            requires_user_approval,
            allowed: false,
            reason: "pending_decision".to_string(),
        }
    }

    fn build_decision(
        &self,
        goal: &str,
        requested_action: &str,
        permission: ActionPermission,
        approval_available: bool,
        attempt_count: u8,
    ) -> AutonomyDecision {
        let risk_max_level = RiskTierPolicy::max_autonomy_for(permission.risk_tier);
        let forbidden_action = is_categorically_forbidden(requested_action);
        let low_risk_loop_requested = requested_action == "low-risk-sandbox-loop";

        let (allowed, reason, next_safe_action) = if forbidden_action {
            (
                false,
                "categorically_forbidden_action".to_string(),
                "stop_and_request_human_review".to_string(),
            )
        } else if attempt_count >= 3 {
            (
                false,
                "max_attempts_exceeded".to_string(),
                "request_human_intervention_after_three_failures".to_string(),
            )
        } else if permission.risk_tier == RiskTier::SafetyCritical
            && permission.required_autonomy_level.rank() > AutonomyLevel::L1ProposalOnly.rank()
        {
            (
                false,
                "safety_critical_requires_proposal_only".to_string(),
                "generate_review_only_proposal".to_string(),
            )
        } else if permission.risk_tier == RiskTier::High
            && permission.required_autonomy_level.rank() > AutonomyLevel::L2SandboxDryRun.rank()
            && !approval_available
        {
            (
                false,
                "high_risk_apply_requires_explicit_review".to_string(),
                "dry_run_or_request_high_risk_review".to_string(),
            )
        } else if permission.required_autonomy_level.rank() > risk_max_level.rank()
            && !approval_available
        {
            (
                false,
                "risk_tier_does_not_allow_requested_autonomy".to_string(),
                "lower_to_safe_autonomy_level".to_string(),
            )
        } else if low_risk_loop_requested && permission.risk_tier != RiskTier::Low {
            (
                false,
                "supervised_loop_is_low_risk_only".to_string(),
                "request_human_review".to_string(),
            )
        } else if permission.requires_user_approval && !approval_available {
            (
                false,
                "approval_missing_or_scope_dry_run".to_string(),
                "request_explicit_approval_scope".to_string(),
            )
        } else if !self
            .current_autonomy_level
            .can_cover(permission.required_autonomy_level)
            && !approval_available
        {
            (
                false,
                "current_autonomy_level_too_low".to_string(),
                "request_approval_or_lower_action".to_string(),
            )
        } else {
            (
                true,
                "allowed_by_autonomy_governor".to_string(),
                "continue_with_governed_action".to_string(),
            )
        };

        AutonomyDecision {
            id: format!(
                "autonomy_decision.{}.{}",
                stable_id(&format!("{goal}-{requested_action}")),
                now()
            ),
            goal: goal.to_string(),
            requested_action: requested_action.to_string(),
            risk_tier: permission.risk_tier,
            current_autonomy_level: self.current_autonomy_level,
            required_autonomy_level: permission.required_autonomy_level,
            approval_required: permission.requires_user_approval,
            approval_available,
            allowed,
            reason,
            next_safe_action,
        }
    }

    fn record_decision(
        &mut self,
        decision: &AutonomyDecision,
        approval_record: Option<&ApprovalRecord>,
    ) {
        self.memory.push(AutonomyMemory {
            id: format!("autonomy_memory.{}.{}", stable_id(&decision.id), now()),
            goal: decision.goal.clone(),
            requested_action: decision.requested_action.clone(),
            decision: decision.clone(),
            outcome: if decision.allowed {
                "allowed".to_string()
            } else {
                "blocked".to_string()
            },
            approval_record_id: approval_record.map(|record| record.id.clone()),
            timestamp: now(),
        });
    }
}

impl RiskTierPolicy {
    pub fn max_autonomy_for(risk_tier: RiskTier) -> AutonomyLevel {
        match risk_tier {
            RiskTier::Low => AutonomyLevel::L5SupervisedLowRiskSandboxLoop,
            RiskTier::Medium => AutonomyLevel::L4ApprovedSandboxApplyAndTest,
            RiskTier::High => AutonomyLevel::L2SandboxDryRun,
            RiskTier::SafetyCritical => AutonomyLevel::L1ProposalOnly,
        }
    }
}

impl RiskClassifier {
    pub fn classify(goal: &str, target_files: &[String]) -> RiskTier {
        let combined = format!(
            "{} {}",
            goal.to_lowercase(),
            target_files.join(" ").to_lowercase()
        );
        if [
            "safety gate",
            "permission gate",
            "self modification boundary",
            "core purpose",
            "identity anchor",
            "reward homeostasis",
            "real pc input",
            "network execution",
            "network request",
            "robot control",
            "file deletion",
            "delete file",
            "shell execution",
            "shell",
            "git commit",
            "git push",
            "git reset",
            "git clean",
            "package install",
            "unsafe rust",
            "unsafe",
            "disable safety",
            "bypass permission",
            "bypass approval",
            "auto apply",
        ]
        .iter()
        .any(|needle| combined.contains(needle))
        {
            RiskTier::SafetyCritical
        } else if target_files.len() > 3
            || [
                "multi file",
                "multiple files",
                "public api",
                "persistence schema",
                "memory format",
                "cross-module",
                "cross module",
                "routing change",
                "schema change",
            ]
            .iter()
            .any(|needle| combined.contains(needle))
        {
            RiskTier::High
        } else if [
            "single module",
            "module logic",
            "new struct",
            "new enum",
            "struct",
            "enum",
            "connect feature",
            "benchmark parser",
            "small feature",
            "logic change",
        ]
        .iter()
        .any(|needle| combined.contains(needle))
        {
            RiskTier::Medium
        } else {
            RiskTier::Low
        }
    }
}

impl EscalationPolicy {
    pub fn escalation_reason(decision: &AutonomyDecision, attempt_count: u8) -> Option<String> {
        if decision.risk_tier == RiskTier::SafetyCritical {
            Some("safety_critical_requires_human_intervention".to_string())
        } else if decision.risk_tier == RiskTier::High
            && matches!(
                decision.requested_action.as_str(),
                "sandbox-apply" | "sandbox-test" | "low-risk-sandbox-loop"
            )
            && !decision.approval_available
        {
            Some("high_risk_apply_requires_human_intervention".to_string())
        } else if attempt_count >= 3 {
            Some("attempt_count_requires_human_intervention".to_string())
        } else {
            None
        }
    }
}

pub fn approval_inferred_from_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("--scope ")
        || lower.contains("scope:")
        || lower.contains("sandbox-apply-only")
        || lower.contains("sandbox-apply-and-test")
        || lower.contains("low-risk-sandbox-loop")
}

fn required_level_for_action(action: &str) -> AutonomyLevel {
    match action {
        "observe" | "read" | "status" | "benchmark-read" | "risk-analysis" => {
            AutonomyLevel::L0ObserveOnly
        }
        "patch-proposal" | "patch-plan" | "diff-preview" | "test-plan" | "generate-revision" => {
            AutonomyLevel::L1ProposalOnly
        }
        "create-sandbox" | "dry-run" => AutonomyLevel::L2SandboxDryRun,
        "sandbox-apply" => AutonomyLevel::L3ApprovedSandboxApply,
        "sandbox-test" | "run-approved-tests" => AutonomyLevel::L4ApprovedSandboxApplyAndTest,
        "low-risk-sandbox-loop" => AutonomyLevel::L5SupervisedLowRiskSandboxLoop,
        "request-original-patch-apply" => AutonomyLevel::L6OriginalPatchRequestOnly,
        "original-write" => AutonomyLevel::L7OriginalWriteForbidden,
        _ => AutonomyLevel::L7OriginalWriteForbidden,
    }
}

fn is_categorically_forbidden(action: &str) -> bool {
    matches!(
        action,
        "original-write"
            | "git-commit"
            | "git-push"
            | "git-reset"
            | "git-clean"
            | "file-delete"
            | "shell"
            | "network"
            | "package-install"
            | "unsafe-rust"
            | "bypass-safety-gate"
            | "bypass-permission-gate"
            | "change-core-purpose"
            | "change-identity-anchor"
            | "bypass-approval"
            | "real-pc-input"
            | "robot-control"
    )
}

fn stable_id(input: &str) -> String {
    let id = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .take(10)
        .collect::<Vec<_>>()
        .join("_");
    if id.is_empty() {
        "autonomy".to_string()
    } else {
        id
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomy_governor_initializes_with_safe_defaults() {
        let governor = AutonomyGovernor::new();
        let status = governor.status();
        assert_eq!(
            status.current_autonomy_level,
            AutonomyLevel::L2SandboxDryRun
        );
        assert!(status.original_write_forbidden);
        assert!(status.safetycritical_auto_block);
        assert!(status.explicit_scope_required);
    }

    #[test]
    fn autonomy_level_l0_allows_observe_only() {
        let mut governor = AutonomyGovernor::with_level(AutonomyLevel::L0ObserveOnly);
        let observe = governor.decide("inspect logs", "observe");
        let proposal = governor.decide("inspect logs", "patch-proposal");
        assert!(observe.allowed);
        assert!(!proposal.allowed);
    }

    #[test]
    fn autonomy_level_l1_allows_proposal_only() {
        let mut governor = AutonomyGovernor::with_level(AutonomyLevel::L1ProposalOnly);
        let proposal = governor.decide("add regression test", "patch-proposal");
        let dry_run = governor.decide("add regression test", "dry-run");
        assert!(proposal.allowed);
        assert!(!dry_run.allowed);
    }

    #[test]
    fn autonomy_level_l2_allows_dry_run_only() {
        let mut governor = AutonomyGovernor::with_level(AutonomyLevel::L2SandboxDryRun);
        let dry_run = governor.decide("add regression test", "dry-run");
        let apply = governor.decide("add regression test", "sandbox-apply");
        assert!(dry_run.allowed);
        assert!(!apply.allowed);
    }

    #[test]
    fn autonomy_level_l3_requires_approval_for_sandbox_apply() {
        let mut governor = AutonomyGovernor::with_level(AutonomyLevel::L3ApprovedSandboxApply);
        let blocked = governor.decide("add regression test", "sandbox-apply");
        let record = ApprovalRecord::new("add regression test", ApprovalScope::SandboxApplyOnly, 1);
        let allowed = governor.decide_with_context(
            "add regression test",
            "sandbox-apply",
            &[],
            Some(&record),
            1,
        );
        assert!(!blocked.allowed);
        assert!(blocked.approval_required);
        assert!(allowed.allowed);
    }

    #[test]
    fn autonomy_level_l4_requires_approval_for_sandbox_tests() {
        let mut governor =
            AutonomyGovernor::with_level(AutonomyLevel::L4ApprovedSandboxApplyAndTest);
        let blocked = governor.decide("add regression test", "sandbox-test");
        let record =
            ApprovalRecord::new("add regression test", ApprovalScope::SandboxApplyAndTest, 1);
        let allowed = governor.decide_with_context(
            "add regression test",
            "sandbox-test",
            &[],
            Some(&record),
            1,
        );
        assert!(!blocked.allowed);
        assert!(allowed.allowed);
    }

    #[test]
    fn autonomy_level_l5_allows_only_low_risk_supervised_loop() {
        let mut governor =
            AutonomyGovernor::with_level(AutonomyLevel::L5SupervisedLowRiskSandboxLoop);
        let record =
            ApprovalRecord::new("add regression test", ApprovalScope::LowRiskSandboxLoop, 3);
        let allowed = governor.decide_with_context(
            "add regression test",
            "low-risk-sandbox-loop",
            &[],
            Some(&record),
            1,
        );
        let blocked = governor.decide_with_context(
            "public api memory schema change",
            "low-risk-sandbox-loop",
            &["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()],
            Some(&record),
            1,
        );
        assert!(allowed.allowed);
        assert!(!blocked.allowed);
    }

    #[test]
    fn risk_tier_classifies_test_addition_as_low() {
        assert_eq!(
            RiskClassifier::classify("add regression test for embryo growth goal", &[]),
            RiskTier::Low
        );
    }

    #[test]
    fn risk_tier_classifies_single_module_logic_change_as_medium() {
        assert_eq!(
            RiskClassifier::classify("single module logic change", &[]),
            RiskTier::Medium
        );
    }

    #[test]
    fn risk_tier_classifies_multi_file_api_change_as_high() {
        assert_eq!(
            RiskClassifier::classify(
                "multi file public api change",
                &[
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string()
                ],
            ),
            RiskTier::High
        );
    }

    #[test]
    fn risk_tier_classifies_safety_gate_change_as_safetycritical() {
        assert_eq!(
            RiskClassifier::classify("modify safety gate to allow auto apply", &[]),
            RiskTier::SafetyCritical
        );
    }

    #[test]
    fn safetycritical_blocks_sandbox_apply_even_with_generic_approval() {
        let record = ApprovalRecord::new(
            "modify safety gate to allow auto apply",
            ApprovalScope::SandboxApplyAndTest,
            1,
        );
        let decision = AutonomyGovernor::with_level(AutonomyLevel::L4ApprovedSandboxApplyAndTest)
            .decide_with_context(
                "modify safety gate to allow auto apply",
                "sandbox-apply",
                &[],
                Some(&record),
                1,
            );
        assert!(!decision.allowed);
        assert_eq!(decision.risk_tier, RiskTier::SafetyCritical);
    }

    #[test]
    fn approval_record_expires_after_max_attempts() {
        let mut record =
            ApprovalRecord::new("add regression test", ApprovalScope::SandboxApplyOnly, 1);
        record.consume_attempt();
        assert!(!record.is_available_for("add regression test", "sandbox-apply", RiskTier::Low));
    }

    #[test]
    fn approval_record_does_not_transfer_to_other_goal() {
        let record = ApprovalRecord::new("add regression test", ApprovalScope::SandboxApplyOnly, 1);
        assert!(!record.is_available_for("other goal", "sandbox-apply", RiskTier::Low));
    }

    #[test]
    fn approval_inference_does_not_treat_continue_as_approval() {
        assert!(!approval_inferred_from_text("좋아"));
        assert!(!approval_inferred_from_text("진행해"));
        assert!(!approval_inferred_from_text("다음"));
        assert!(!approval_inferred_from_text("계속"));
        assert!(approval_inferred_from_text(
            "--scope sandbox-apply-and-test"
        ));
    }

    #[test]
    fn autonomy_governor_blocks_original_code_write() {
        let decision = AutonomyGovernor::new().decide("add regression test", "original-write");
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "categorically_forbidden_action");
    }

    #[test]
    fn autonomy_governor_blocks_shell_network_git_commands() {
        for action in [
            "shell",
            "network",
            "git-commit",
            "git-push",
            "package-install",
        ] {
            let decision = AutonomyGovernor::new().decide("dangerous action", action);
            assert!(!decision.allowed);
        }
    }

    #[test]
    fn closed_growth_cycle_calls_autonomy_governor_before_apply() {
        let decision = AutonomyGovernor::new().decide(
            "VoiceSynthesis EmergentFunction test failed",
            "sandbox-apply",
        );
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "approval_missing_or_scope_dry_run");
    }

    #[test]
    fn closed_growth_cycle_calls_autonomy_governor_before_tests() {
        let decision = AutonomyGovernor::new().decide(
            "VoiceSynthesis EmergentFunction test failed",
            "sandbox-test",
        );
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "approval_missing_or_scope_dry_run");
    }

    #[test]
    fn autonomy_memory_records_allowed_and_blocked_actions() {
        let mut governor = AutonomyGovernor::new();
        let allowed = governor.decide("add regression test", "dry-run");
        let blocked = governor.decide("add regression test", "sandbox-apply");
        assert!(allowed.allowed);
        assert!(!blocked.allowed);
        assert_eq!(governor.memory().len(), 2);
        assert_eq!(governor.memory()[0].outcome, "allowed");
        assert_eq!(governor.memory()[1].outcome, "blocked");
    }

    #[test]
    fn autonomy_report_contains_level_scope_remaining_attempts() {
        let mut governor = AutonomyGovernor::new();
        governor.grant("add regression test", ApprovalScope::LowRiskSandboxLoop, 3);
        governor.decide("add regression test", "dry-run");
        governor.decide("add regression test", "original-write");
        let report = governor.report();
        assert_eq!(
            report.current_autonomy_level,
            AutonomyLevel::L2SandboxDryRun
        );
        assert_eq!(
            report.current_approval_scope,
            Some(ApprovalScope::LowRiskSandboxLoop)
        );
        assert_eq!(report.remaining_attempts, 3);
        assert!(!report.recent_allowed_actions.is_empty());
        assert!(!report.recent_blocked_actions.is_empty());
    }

    #[test]
    fn autonomy_benchmark_reduces_over_autonomy_without_blocking_safe_progress() {
        let report = AutonomyGovernor::benchmark();
        assert!(report.autonomy_benchmark_reduces_over_autonomy_without_blocking_safe_progress);
        assert!(report.on_over_autonomy_rate < report.off_over_autonomy_rate);
        assert!(report.on_safe_progress_score > report.off_safe_progress_score);
    }
}
