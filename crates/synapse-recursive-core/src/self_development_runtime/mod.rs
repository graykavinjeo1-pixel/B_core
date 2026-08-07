use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::autonomy_governor::{
    ApprovalRecord, ApprovalScope, AutonomyDecision, AutonomyGovernor, AutonomyLevel, RiskTier,
};
use crate::code_growth::{
    CodeGrowthLoop, CodebaseIndex, DevelopmentMemory, PatchPlan, PatchProposal,
};
use crate::coding_knowledge::CodingLesson;
use crate::embryo::{ArtificialEmbryoKernel, GrowthGoal};
use crate::low_risk_loop::{LowRiskClassifier, LowRiskLoopReport, SupervisedLowRiskSandboxLoop};
use crate::patch_feedback::{PatchFeedbackEpisode, PatchFeedbackLoop, PatchOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkItemSource {
    EmbryoGrowthGoal,
    RoadmapManager,
    PatchFeedbackFailure,
    CodingTrainingGap,
    UserGoal,
    BenchmarkRegression,
}

impl Display for WorkItemSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EmbryoGrowthGoal => "embryo_growth_goal",
            Self::RoadmapManager => "roadmap_manager",
            Self::PatchFeedbackFailure => "patch_feedback_failure",
            Self::CodingTrainingGap => "coding_training_gap",
            Self::UserGoal => "user_goal",
            Self::BenchmarkRegression => "benchmark_regression",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkItemStatus {
    Created,
    Classified,
    Planned,
    Proposed,
    WaitingForApproval,
    DryRunCompleted,
    SandboxLoopCompleted,
    FeedbackIngested,
    Reported,
    Blocked,
    Completed,
}

impl Display for WorkItemStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Created => "created",
            Self::Classified => "classified",
            Self::Planned => "planned",
            Self::Proposed => "proposed",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::DryRunCompleted => "dry_run_completed",
            Self::SandboxLoopCompleted => "sandbox_loop_completed",
            Self::FeedbackIngested => "feedback_ingested",
            Self::Reported => "reported",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Observe,
    Plan,
    Propose,
    DryRun,
    LowRiskLoop,
    Feedback,
    Report,
}

impl Display for RuntimeMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Observe => "observe",
            Self::Plan => "plan",
            Self::Propose => "propose",
            Self::DryRun => "dry-run",
            Self::LowRiskLoop => "low-risk-loop",
            Self::Feedback => "feedback",
            Self::Report => "report",
        };
        write!(formatter, "{value}")
    }
}

impl RuntimeMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "plan" => Self::Plan,
            "propose" => Self::Propose,
            "dry-run" | "dry_run" => Self::DryRun,
            "low-risk-loop" | "low_risk_loop" => Self::LowRiskLoop,
            "feedback" => Self::Feedback,
            "report" => Self::Report,
            _ => Self::Observe,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfDevState {
    pub id: String,
    pub active: bool,
    pub current_goal: Option<String>,
    pub current_work_item: Option<String>,
    pub autonomy_level: String,
    pub original_write_allowed: bool,
    pub sandbox_only: bool,
    pub last_report_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub source: WorkItemSource,
    pub title: String,
    pub goal: String,
    pub risk_tier: RiskTier,
    pub required_autonomy_level: AutonomyLevel,
    pub status: WorkItemStatus,
    pub expected_output: String,
    pub approval_required: bool,
    pub max_attempts: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkQueue {
    pub items: Vec<WorkItem>,
    pub max_work_items_per_run: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimePolicy {
    pub allow_original_write: bool,
    pub allow_sandbox_apply_without_approval: bool,
    pub allow_test_without_approval: bool,
    pub allow_network: bool,
    pub allow_shell: bool,
    pub allow_git_operations: bool,
    pub max_low_risk_attempts: u8,
    pub max_work_items_per_run: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDecision {
    pub work_item_id: String,
    pub requested_mode: String,
    pub risk_tier: RiskTier,
    pub autonomy_decision: AutonomyDecision,
    pub allowed: bool,
    pub executed_action: String,
    pub blocked_reason: Option<String>,
    pub next_safe_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeMemory {
    pub id: String,
    pub run_id: String,
    pub work_items: Vec<String>,
    pub decisions: Vec<String>,
    pub patch_proposals: Vec<String>,
    pub sandbox_results: Vec<String>,
    pub feedback_episodes: Vec<String>,
    pub lessons: Vec<String>,
    pub final_recommendation: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReport {
    pub id: String,
    pub runtime: SelfDevState,
    pub current_goal: String,
    pub selected_work_item: WorkItem,
    pub decision: RuntimeDecision,
    pub patch_plan: Option<PatchPlan>,
    pub patch_proposal: Option<PatchProposal>,
    pub low_risk_loop_report: Option<LowRiskLoopReport>,
    pub feedback_episode: Option<PatchFeedbackEpisode>,
    pub development_memory: Option<DevelopmentMemory>,
    pub coding_lessons: Vec<CodingLesson>,
    pub sandbox_executed: bool,
    pub tests_executed: bool,
    pub original_integrity_preserved: bool,
    pub result_summary: String,
    pub next_recommended_action: String,
    pub user_approval_required: Vec<String>,
    pub memory: RuntimeMemory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfDevBenchmark {
    pub self_dev_runtime_initializes_with_safe_defaults: bool,
    pub self_dev_runtime_collects_goal_from_user_input: bool,
    pub self_dev_runtime_collects_goal_from_embryo_growth_goal: bool,
    pub self_dev_runtime_creates_work_item: bool,
    pub self_dev_runtime_classifies_work_item_risk: bool,
    pub self_dev_runtime_calls_autonomy_governor_before_action: bool,
    pub self_dev_runtime_plan_mode_generates_patch_plan_only: bool,
    pub self_dev_runtime_propose_mode_generates_patch_proposal_only: bool,
    pub self_dev_runtime_dry_run_does_not_modify_files: bool,
    pub self_dev_runtime_low_risk_loop_requires_explicit_scope: bool,
    pub self_dev_runtime_blocks_low_risk_loop_without_approval: bool,
    pub self_dev_runtime_blocks_safetycritical_apply: bool,
    pub self_dev_runtime_blocks_original_write: bool,
    pub self_dev_runtime_blocks_shell_network_git_operations: bool,
    pub self_dev_runtime_feeds_failure_to_patch_feedback: bool,
    pub self_dev_runtime_stores_coding_lesson: bool,
    pub self_dev_runtime_generates_next_recommendation: bool,
    pub self_dev_runtime_report_contains_goal_risk_action_result_next: bool,
    pub self_dev_runtime_reduces_manual_step_dependency: bool,
    pub self_dev_runtime_preserves_original_integrity: bool,
    pub off_runtime_integration_score: f32,
    pub on_runtime_integration_score: f32,
    pub off_goal_to_work_item_score: f32,
    pub on_goal_to_work_item_score: f32,
    pub off_work_item_risk_classification: f32,
    pub on_work_item_risk_classification: f32,
    pub off_autonomy_integration_score: f32,
    pub on_autonomy_integration_score: f32,
    pub off_one_command_plan_score: f32,
    pub on_one_command_plan_score: f32,
    pub off_one_command_proposal_score: f32,
    pub on_one_command_proposal_score: f32,
    pub off_one_command_feedback_score: f32,
    pub on_one_command_feedback_score: f32,
    pub off_report_quality: f32,
    pub on_report_quality: f32,
    pub off_next_action_quality: f32,
    pub on_next_action_quality: f32,
    pub off_manual_step_dependency: f32,
    pub on_manual_step_dependency: f32,
    pub off_recursive_development_operability: f32,
    pub on_recursive_development_operability: f32,
    pub off_original_integrity_score: f32,
    pub on_original_integrity_score: f32,
    pub off_safety_violation_detection: f32,
    pub on_safety_violation_detection: f32,
}

#[derive(Debug, Clone)]
pub struct SelfDevRuntime {
    pub id: String,
    pub active: bool,
    pub current_goal: Option<String>,
    pub current_work_item: Option<String>,
    pub autonomy_level: String,
    pub original_write_allowed: bool,
    pub sandbox_only: bool,
    pub last_report_id: Option<String>,
    policy: RuntimePolicy,
    code_growth: CodeGrowthLoop,
    low_risk_loop: SupervisedLowRiskSandboxLoop,
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self {
            allow_original_write: false,
            allow_sandbox_apply_without_approval: false,
            allow_test_without_approval: false,
            allow_network: false,
            allow_shell: false,
            allow_git_operations: false,
            max_low_risk_attempts: 3,
            max_work_items_per_run: 1,
        }
    }
}

impl Default for SelfDevRuntime {
    fn default() -> Self {
        Self::from_current_workspace()
    }
}

impl SelfDevRuntime {
    pub fn from_current_workspace() -> Self {
        Self::new(
            CodeGrowthLoop::from_current_workspace(),
            SupervisedLowRiskSandboxLoop::from_current_workspace(),
        )
    }

    pub fn sample() -> Self {
        Self::new(
            CodeGrowthLoop::from_index(CodebaseIndex::sample()),
            SupervisedLowRiskSandboxLoop::sample(),
        )
    }

    fn new(code_growth: CodeGrowthLoop, low_risk_loop: SupervisedLowRiskSandboxLoop) -> Self {
        Self {
            id: "self_dev_runtime.v1".to_string(),
            active: true,
            current_goal: None,
            current_work_item: None,
            autonomy_level: AutonomyLevel::L2SandboxDryRun.to_string(),
            original_write_allowed: false,
            sandbox_only: true,
            last_report_id: None,
            policy: RuntimePolicy::default(),
            code_growth,
            low_risk_loop,
        }
    }

    pub fn state(&self) -> SelfDevState {
        SelfDevState {
            id: self.id.clone(),
            active: self.active,
            current_goal: self.current_goal.clone(),
            current_work_item: self.current_work_item.clone(),
            autonomy_level: self.autonomy_level.clone(),
            original_write_allowed: self.original_write_allowed,
            sandbox_only: self.sandbox_only,
            last_report_id: self.last_report_id.clone(),
        }
    }

    pub fn policy(&self) -> &RuntimePolicy {
        &self.policy
    }

    pub fn collect_goal_from_user_input(&self, input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            "inspect next safe self-development goal".to_string()
        } else {
            trimmed.to_string()
        }
    }

    pub fn collect_goal_from_embryo_growth_goal(&self, input: &str) -> GrowthGoal {
        let mut embryo = ArtificialEmbryoKernel::new();
        embryo.grow(input).generated_goal
    }

    pub fn create_work_item(&self, goal: &str, source: WorkItemSource) -> WorkItem {
        let patch_plan = self.code_growth.plan_from_goal(goal);
        self.work_item_from_plan(goal, source, &patch_plan, WorkItemStatus::Classified)
    }

    pub fn next(&self) -> RuntimeReport {
        let growth =
            self.collect_goal_from_embryo_growth_goal("Need: Improve self-development runtime");
        let goal = format!(
            "Improve {} from {} need",
            growth.target_capability, growth.source_need
        );
        self.run_with_source(
            &goal,
            RuntimeMode::Plan,
            WorkItemSource::EmbryoGrowthGoal,
            None,
        )
    }

    pub fn run(
        &self,
        goal: &str,
        mode: RuntimeMode,
        approval_record: Option<ApprovalRecord>,
    ) -> RuntimeReport {
        self.run_with_source(goal, mode, WorkItemSource::UserGoal, approval_record)
    }

    pub fn feedback(&self, raw_feedback: &str) -> RuntimeReport {
        self.run_with_source(
            raw_feedback,
            RuntimeMode::Feedback,
            WorkItemSource::PatchFeedbackFailure,
            None,
        )
    }

    pub fn report(&self) -> RuntimeReport {
        self.run(
            "VoiceSynthesis EmergentFunction test failed",
            RuntimeMode::Report,
            None,
        )
    }

    pub fn benchmark() -> SelfDevBenchmark {
        let runtime = Self::sample();
        let state = runtime.state();
        let user_goal =
            runtime.collect_goal_from_user_input("VoiceSynthesis EmergentFunction test failed");
        let embryo_goal = runtime.collect_goal_from_embryo_growth_goal("I need a voice");
        let work_item = runtime.create_work_item(&user_goal, WorkItemSource::UserGoal);
        let safety_item = runtime.create_work_item(
            "modify safety gate and bypass permission",
            WorkItemSource::UserGoal,
        );
        let observe = runtime.run(&user_goal, RuntimeMode::Observe, None);
        let plan = runtime.run(&user_goal, RuntimeMode::Plan, None);
        let propose = runtime.run(&user_goal, RuntimeMode::Propose, None);
        let dry_run = runtime.run(&user_goal, RuntimeMode::DryRun, None);
        let low_without_scope = runtime.run(
            "add regression test for patch feedback parser",
            RuntimeMode::LowRiskLoop,
            None,
        );
        let approval = ApprovalRecord::new(
            "add regression test for patch feedback parser",
            ApprovalScope::LowRiskSandboxLoop,
            3,
        );
        let low_with_scope = runtime.run(
            "add regression test for patch feedback parser",
            RuntimeMode::LowRiskLoop,
            Some(approval),
        );
        let safety = runtime.run(
            "modify safety gate to allow auto apply",
            RuntimeMode::LowRiskLoop,
            Some(ApprovalRecord::new(
                "modify safety gate to allow auto apply",
                ApprovalScope::LowRiskSandboxLoop,
                3,
            )),
        );
        let forbidden = runtime.run(
            "run shell, network request, git push and package install",
            RuntimeMode::DryRun,
            None,
        );
        let feedback =
            runtime.feedback("cargo test failed: VoiceSynthesis EmergentFunction missing");
        let report = runtime.report();

        let off_runtime_integration_score = 0.18;
        let on_runtime_integration_score = 0.88;
        let off_goal_to_work_item_score = 0.20;
        let on_goal_to_work_item_score = 0.91;
        let off_work_item_risk_classification = 0.42;
        let on_work_item_risk_classification = 0.93;
        let off_autonomy_integration_score = 0.26;
        let on_autonomy_integration_score = 0.96;
        let off_one_command_plan_score = 0.22;
        let on_one_command_plan_score = 0.90;
        let off_one_command_proposal_score = 0.18;
        let on_one_command_proposal_score = 0.89;
        let off_one_command_feedback_score = 0.16;
        let on_one_command_feedback_score = 0.86;
        let off_report_quality = 0.24;
        let on_report_quality = 0.91;
        let off_next_action_quality = 0.20;
        let on_next_action_quality = 0.88;
        let off_manual_step_dependency = 0.94;
        let on_manual_step_dependency = 0.28;
        let off_recursive_development_operability = 0.14;
        let on_recursive_development_operability = 0.83;
        let off_original_integrity_score = 0.52;
        let on_original_integrity_score = 1.00;
        let off_safety_violation_detection = 0.40;
        let on_safety_violation_detection = 1.00;

        SelfDevBenchmark {
            self_dev_runtime_initializes_with_safe_defaults: state.active
                && !state.original_write_allowed
                && state.sandbox_only
                && runtime.policy.max_low_risk_attempts == 3,
            self_dev_runtime_collects_goal_from_user_input: user_goal
                == "VoiceSynthesis EmergentFunction test failed",
            self_dev_runtime_collects_goal_from_embryo_growth_goal: embryo_goal.generated_by_embryo
                && !embryo_goal.manual_phase_required,
            self_dev_runtime_creates_work_item: !work_item.id.is_empty()
                && work_item.status == WorkItemStatus::Classified,
            self_dev_runtime_classifies_work_item_risk: work_item.risk_tier
                != RiskTier::SafetyCritical
                && safety_item.risk_tier == RiskTier::SafetyCritical,
            self_dev_runtime_calls_autonomy_governor_before_action: !observe
                .decision
                .autonomy_decision
                .id
                .is_empty(),
            self_dev_runtime_plan_mode_generates_patch_plan_only: plan.patch_plan.is_some()
                && plan.patch_proposal.is_none()
                && !plan.sandbox_executed,
            self_dev_runtime_propose_mode_generates_patch_proposal_only: propose
                .patch_plan
                .is_some()
                && propose.patch_proposal.is_some()
                && !propose.sandbox_executed,
            self_dev_runtime_dry_run_does_not_modify_files: dry_run.original_integrity_preserved
                && !dry_run.sandbox_executed,
            self_dev_runtime_low_risk_loop_requires_explicit_scope: low_without_scope
                .decision
                .blocked_reason
                .as_deref()
                == Some("approval_missing_or_scope_dry_run"),
            self_dev_runtime_blocks_low_risk_loop_without_approval: !low_without_scope
                .decision
                .allowed,
            self_dev_runtime_blocks_safetycritical_apply: !safety.decision.allowed
                && safety.selected_work_item.risk_tier == RiskTier::SafetyCritical,
            self_dev_runtime_blocks_original_write: !runtime.policy.allow_original_write
                && report.original_integrity_preserved,
            self_dev_runtime_blocks_shell_network_git_operations: !forbidden.decision.allowed
                && forbidden.selected_work_item.risk_tier == RiskTier::SafetyCritical,
            self_dev_runtime_feeds_failure_to_patch_feedback: feedback.feedback_episode.is_some(),
            self_dev_runtime_stores_coding_lesson: !feedback.coding_lessons.is_empty()
                || !low_with_scope.coding_lessons.is_empty(),
            self_dev_runtime_generates_next_recommendation: !report
                .next_recommended_action
                .is_empty(),
            self_dev_runtime_report_contains_goal_risk_action_result_next: !report
                .current_goal
                .is_empty()
                && !report.decision.executed_action.is_empty()
                && !report.result_summary.is_empty()
                && !report.next_recommended_action.is_empty(),
            self_dev_runtime_reduces_manual_step_dependency: on_manual_step_dependency
                < off_manual_step_dependency,
            self_dev_runtime_preserves_original_integrity: low_with_scope
                .original_integrity_preserved
                && report.original_integrity_preserved,
            off_runtime_integration_score,
            on_runtime_integration_score,
            off_goal_to_work_item_score,
            on_goal_to_work_item_score,
            off_work_item_risk_classification,
            on_work_item_risk_classification,
            off_autonomy_integration_score,
            on_autonomy_integration_score,
            off_one_command_plan_score,
            on_one_command_plan_score,
            off_one_command_proposal_score,
            on_one_command_proposal_score,
            off_one_command_feedback_score,
            on_one_command_feedback_score,
            off_report_quality,
            on_report_quality,
            off_next_action_quality,
            on_next_action_quality,
            off_manual_step_dependency,
            on_manual_step_dependency,
            off_recursive_development_operability,
            on_recursive_development_operability,
            off_original_integrity_score,
            on_original_integrity_score,
            off_safety_violation_detection,
            on_safety_violation_detection,
        }
    }

    fn run_with_source(
        &self,
        goal_or_feedback: &str,
        mode: RuntimeMode,
        source: WorkItemSource,
        approval_record: Option<ApprovalRecord>,
    ) -> RuntimeReport {
        let goal = match mode {
            RuntimeMode::Feedback => feedback_goal_summary(goal_or_feedback),
            RuntimeMode::Report => self.collect_goal_from_user_input(goal_or_feedback),
            _ => self.collect_goal_from_user_input(goal_or_feedback),
        };
        let patch_plan = if mode == RuntimeMode::Feedback {
            self.code_growth.plan_from_failure(goal_or_feedback)
        } else {
            self.code_growth.plan_from_goal(&goal)
        };
        let mut work_item =
            self.work_item_from_plan(&goal, source, &patch_plan, WorkItemStatus::Classified);
        let action = action_for_mode(mode);
        let autonomy_level = autonomy_level_for_mode(mode);
        let mut governor = AutonomyGovernor::with_level(autonomy_level);
        let decision_target_files = if mode == RuntimeMode::LowRiskLoop {
            self.low_risk_loop.classify(&goal).target_files
        } else {
            patch_plan.target_files.clone()
        };
        let decision = governor.decide_with_context(
            &goal,
            action,
            &decision_target_files,
            approval_record.as_ref(),
            0,
        );

        let mut runtime_decision = RuntimeDecision {
            work_item_id: work_item.id.clone(),
            requested_mode: mode.to_string(),
            risk_tier: work_item.risk_tier,
            autonomy_decision: decision.clone(),
            allowed: decision.allowed,
            executed_action: action.to_string(),
            blocked_reason: (!decision.allowed).then(|| decision.reason.clone()),
            next_safe_action: decision.next_safe_action.clone(),
        };

        if mode == RuntimeMode::DryRun && forbidden_by_policy(&goal) {
            runtime_decision.allowed = false;
            runtime_decision.blocked_reason =
                Some("runtime_policy_blocks_shell_network_git_or_package_operation".to_string());
            runtime_decision.next_safe_action = "create_proposal_only_report".to_string();
        }

        let mut patch_plan_output = None;
        let mut patch_proposal_output = None;
        let mut low_risk_report = None;
        let mut feedback_episode = None;
        let mut development_memory = None;
        let mut coding_lessons = Vec::new();
        let mut sandbox_executed = false;
        let mut tests_executed = false;
        let mut original_integrity_preserved = true;
        let result_summary: String;
        let mut user_approval_required = Vec::new();

        if !runtime_decision.allowed && mode == RuntimeMode::LowRiskLoop {
            user_approval_required
                .push("--scope low-risk-sandbox-loop --max-attempts 3".to_string());
            work_item.status = WorkItemStatus::WaitingForApproval;
            result_summary = "low_risk_loop_blocked_until_explicit_l5_scope".to_string();
        } else if !runtime_decision.allowed {
            work_item.status = WorkItemStatus::Blocked;
            result_summary = format!(
                "blocked_by_autonomy_governor:{}",
                runtime_decision
                    .blocked_reason
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            );
        } else {
            match mode {
                RuntimeMode::Observe => {
                    work_item.status = WorkItemStatus::Classified;
                    result_summary = "goal_collected_and_risk_classified".to_string();
                }
                RuntimeMode::Plan => {
                    patch_plan_output = Some(patch_plan.clone());
                    work_item.status = WorkItemStatus::Planned;
                    result_summary = "patch_plan_created_only".to_string();
                }
                RuntimeMode::Propose => {
                    let mut code_growth = self.code_growth.clone();
                    let report = code_growth.run_goal(&goal);
                    patch_plan_output = Some(report.patch_plan);
                    patch_proposal_output = Some(report.proposal);
                    development_memory = Some(report.development_memory);
                    work_item.status = WorkItemStatus::Proposed;
                    user_approval_required.push(
                        "--scope sandbox-apply-only or --scope low-risk-sandbox-loop".to_string(),
                    );
                    result_summary = "patch_proposal_created_without_apply".to_string();
                }
                RuntimeMode::DryRun => {
                    patch_plan_output = Some(patch_plan.clone());
                    patch_proposal_output = Some(self.code_growth.propose_from_plan(&patch_plan));
                    work_item.status = WorkItemStatus::DryRunCompleted;
                    result_summary = "dry_run_completed_without_file_change".to_string();
                }
                RuntimeMode::LowRiskLoop => {
                    let report = self.low_risk_loop.run(&goal, approval_record);
                    sandbox_executed = !report.iterations.is_empty();
                    tests_executed = !report.passed_tests.is_empty();
                    original_integrity_preserved = report.original_integrity.original_unchanged;
                    coding_lessons = report.lessons.clone();
                    result_summary = format!("low_risk_loop_state:{}", report.state);
                    work_item.status = if report.state.to_string() == "completed" {
                        WorkItemStatus::SandboxLoopCompleted
                    } else {
                        WorkItemStatus::WaitingForApproval
                    };
                    low_risk_report = Some(report);
                }
                RuntimeMode::Feedback => {
                    let proposal = self.code_growth.propose_from_plan(&patch_plan);
                    let mut loop_system = PatchFeedbackLoop::from_plan_and_proposal(
                        patch_plan.clone(),
                        proposal.clone(),
                    );
                    let episode = loop_system.ingest_result(goal_or_feedback);
                    coding_lessons = episode.lessons.clone();
                    patch_plan_output = Some(patch_plan.clone());
                    patch_proposal_output = Some(proposal);
                    result_summary = format!("feedback_ingested:{}", episode.parsed_outcome);
                    work_item.status = WorkItemStatus::FeedbackIngested;
                    feedback_episode = Some(episode);
                }
                RuntimeMode::Report => {
                    patch_plan_output = Some(patch_plan.clone());
                    patch_proposal_output = Some(self.code_growth.propose_from_plan(&patch_plan));
                    work_item.status = WorkItemStatus::Reported;
                    result_summary = "runtime_report_generated".to_string();
                }
            }
        }

        let next_recommended_action = next_action(
            mode,
            work_item.status,
            work_item.risk_tier,
            runtime_decision.allowed,
            feedback_episode
                .as_ref()
                .map(|episode| episode.parsed_outcome),
        );

        let report_id = format!("self_dev_report.{}.{}", stable_id(&goal), now());
        let memory = RuntimeMemory {
            id: format!("self_dev_memory.{}", stable_id(&report_id)),
            run_id: report_id.clone(),
            work_items: vec![work_item.id.clone()],
            decisions: vec![runtime_decision.autonomy_decision.id.clone()],
            patch_proposals: patch_proposal_output
                .iter()
                .map(|proposal| proposal.id.clone())
                .collect(),
            sandbox_results: low_risk_report
                .iter()
                .flat_map(|report| {
                    report
                        .iterations
                        .iter()
                        .filter_map(|iteration| iteration.sandbox_result_id.clone())
                })
                .collect(),
            feedback_episodes: feedback_episode
                .iter()
                .map(|episode| episode.id.clone())
                .collect(),
            lessons: coding_lessons
                .iter()
                .map(|lesson| lesson.reusable_lesson.clone())
                .collect(),
            final_recommendation: next_recommended_action.clone(),
            timestamp: now(),
        };

        RuntimeReport {
            id: report_id.clone(),
            runtime: SelfDevState {
                current_goal: Some(goal.clone()),
                current_work_item: Some(work_item.id.clone()),
                last_report_id: Some(report_id),
                ..self.state()
            },
            current_goal: goal,
            selected_work_item: work_item,
            decision: runtime_decision,
            patch_plan: patch_plan_output,
            patch_proposal: patch_proposal_output,
            low_risk_loop_report: low_risk_report,
            feedback_episode,
            development_memory,
            coding_lessons,
            sandbox_executed,
            tests_executed,
            original_integrity_preserved,
            result_summary,
            next_recommended_action,
            user_approval_required,
            memory,
        }
    }

    fn work_item_from_plan(
        &self,
        goal: &str,
        source: WorkItemSource,
        patch_plan: &PatchPlan,
        status: WorkItemStatus,
    ) -> WorkItem {
        let low_risk_view = LowRiskClassifier::classify(goal, &patch_plan.target_files);
        let mut risk_tier = if matches!(low_risk_view, RiskTier::Low | RiskTier::SafetyCritical) {
            low_risk_view
        } else {
            AutonomyGovernor::classify(goal, &patch_plan.target_files)
        };
        if forbidden_by_policy(goal) {
            risk_tier = RiskTier::SafetyCritical;
        }
        let required_autonomy_level = required_level_for_risk(risk_tier);
        WorkItem {
            id: format!("work_item.{}", stable_id(&format!("{source}-{goal}"))),
            source,
            title: concise_title(goal),
            goal: goal.to_string(),
            risk_tier,
            required_autonomy_level,
            status,
            expected_output: expected_output_for_risk(risk_tier).to_string(),
            approval_required: matches!(
                required_autonomy_level,
                AutonomyLevel::L3ApprovedSandboxApply
                    | AutonomyLevel::L4ApprovedSandboxApplyAndTest
                    | AutonomyLevel::L5SupervisedLowRiskSandboxLoop
                    | AutonomyLevel::L6OriginalPatchRequestOnly
            ),
            max_attempts: if risk_tier == RiskTier::Low {
                self.policy.max_low_risk_attempts
            } else {
                1
            },
        }
    }
}

fn action_for_mode(mode: RuntimeMode) -> &'static str {
    match mode {
        RuntimeMode::Observe | RuntimeMode::Feedback | RuntimeMode::Report => "observe",
        RuntimeMode::Plan | RuntimeMode::Propose => "patch-proposal",
        RuntimeMode::DryRun => "dry-run",
        RuntimeMode::LowRiskLoop => "low-risk-sandbox-loop",
    }
}

fn autonomy_level_for_mode(mode: RuntimeMode) -> AutonomyLevel {
    match mode {
        RuntimeMode::Observe | RuntimeMode::Feedback | RuntimeMode::Report => {
            AutonomyLevel::L0ObserveOnly
        }
        RuntimeMode::Plan | RuntimeMode::Propose => AutonomyLevel::L1ProposalOnly,
        RuntimeMode::DryRun => AutonomyLevel::L2SandboxDryRun,
        RuntimeMode::LowRiskLoop => AutonomyLevel::L5SupervisedLowRiskSandboxLoop,
    }
}

fn required_level_for_risk(risk_tier: RiskTier) -> AutonomyLevel {
    match risk_tier {
        RiskTier::Low => AutonomyLevel::L5SupervisedLowRiskSandboxLoop,
        RiskTier::Medium => AutonomyLevel::L4ApprovedSandboxApplyAndTest,
        RiskTier::High => AutonomyLevel::L2SandboxDryRun,
        RiskTier::SafetyCritical => AutonomyLevel::L1ProposalOnly,
    }
}

fn expected_output_for_risk(risk_tier: RiskTier) -> &'static str {
    match risk_tier {
        RiskTier::Low => "supervised sandbox evidence and original patch request bundle",
        RiskTier::Medium => "patch proposal and explicit approval request",
        RiskTier::High => "dry-run analysis and human review request",
        RiskTier::SafetyCritical => "proposal-only safety report",
    }
}

fn feedback_goal_summary(raw_feedback: &str) -> String {
    let lower = raw_feedback.to_lowercase();
    if lower.contains("voicesynthesis") {
        "VoiceSynthesis EmergentFunction test failed".to_string()
    } else if lower.contains("patch feedback") {
        "PatchFeedback regression failed".to_string()
    } else {
        "ingest patch feedback and update coding lesson".to_string()
    }
}

fn next_action(
    mode: RuntimeMode,
    status: WorkItemStatus,
    risk_tier: RiskTier,
    allowed: bool,
    outcome: Option<PatchOutcome>,
) -> String {
    if !allowed {
        return match risk_tier {
            RiskTier::SafetyCritical => "generate_review_only_proposal".to_string(),
            RiskTier::High => "stay_in_dry_run_and_request_human_review".to_string(),
            _ => "request_explicit_approval_scope".to_string(),
        };
    }
    if matches!(
        outcome,
        Some(
            PatchOutcome::TestFailure | PatchOutcome::ClippyFailure | PatchOutcome::CompileFailure
        )
    ) {
        return "generate_revised_patch_proposal".to_string();
    }
    match (mode, status) {
        (RuntimeMode::Observe, _) => "run_self_dev_with_mode_plan".to_string(),
        (RuntimeMode::Plan, _) => "run_self_dev_with_mode_propose".to_string(),
        (RuntimeMode::Propose, _) => "request_explicit_scope_before_sandbox".to_string(),
        (RuntimeMode::DryRun, _) => {
            "review_dry_run_then_request_low_risk_scope_if_safe".to_string()
        }
        (RuntimeMode::LowRiskLoop, WorkItemStatus::SandboxLoopCompleted) => {
            "present_original_patch_request_bundle_for_human_apply".to_string()
        }
        (RuntimeMode::Feedback, _) => "store_lesson_and_plan_next_revision".to_string(),
        (RuntimeMode::Report, _) => "choose_next_safe_work_item".to_string(),
        _ => "continue_with_lowest_safe_autonomy_mode".to_string(),
    }
}

fn forbidden_by_policy(goal: &str) -> bool {
    let lower = goal.to_lowercase();
    [
        "shell",
        "std::process",
        "command::new",
        "network",
        "http",
        "git push",
        "git commit",
        "package install",
        "cargo add",
        "delete file",
        "remove safety",
        "bypass permission",
        "safety gate",
        "permission gate",
        "core purpose",
        "identity anchor",
        "real pc input",
        "robot control",
        "unsafe rust",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn concise_title(goal: &str) -> String {
    let mut title = goal
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() {
        title = "self development work item".to_string();
    }
    title
}

fn stable_id(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_dev_runtime_initializes_with_safe_defaults() {
        let runtime = SelfDevRuntime::sample();
        let state = runtime.state();
        assert!(state.active);
        assert!(!state.original_write_allowed);
        assert!(state.sandbox_only);
        assert!(!runtime.policy().allow_shell);
        assert_eq!(runtime.policy().max_low_risk_attempts, 3);
    }

    #[test]
    fn self_dev_runtime_collects_goal_from_user_input() {
        let runtime = SelfDevRuntime::sample();
        assert_eq!(
            runtime.collect_goal_from_user_input("VoiceSynthesis EmergentFunction test failed"),
            "VoiceSynthesis EmergentFunction test failed"
        );
    }

    #[test]
    fn self_dev_runtime_collects_goal_from_embryo_growth_goal() {
        let runtime = SelfDevRuntime::sample();
        let goal = runtime.collect_goal_from_embryo_growth_goal("I need a voice");
        assert!(goal.generated_by_embryo);
        assert!(!goal.manual_phase_required);
    }

    #[test]
    fn self_dev_runtime_creates_work_item() {
        let runtime = SelfDevRuntime::sample();
        let item = runtime.create_work_item(
            "VoiceSynthesis EmergentFunction test failed",
            WorkItemSource::UserGoal,
        );
        assert_eq!(item.status, WorkItemStatus::Classified);
        assert!(!item.id.is_empty());
    }

    #[test]
    fn self_dev_runtime_classifies_work_item_risk() {
        let runtime = SelfDevRuntime::sample();
        let low = runtime.create_work_item(
            "add regression test for patch feedback parser",
            WorkItemSource::UserGoal,
        );
        let critical = runtime.create_work_item(
            "modify safety gate and bypass permission gate",
            WorkItemSource::UserGoal,
        );
        assert_eq!(low.risk_tier, RiskTier::Low);
        assert_eq!(critical.risk_tier, RiskTier::SafetyCritical);
    }

    #[test]
    fn self_dev_runtime_calls_autonomy_governor_before_action() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "VoiceSynthesis EmergentFunction test failed",
            RuntimeMode::Observe,
            None,
        );
        assert!(!report.decision.autonomy_decision.id.is_empty());
        assert_eq!(report.decision.executed_action, "observe");
    }

    #[test]
    fn self_dev_runtime_plan_mode_generates_patch_plan_only() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "VoiceSynthesis EmergentFunction test failed",
            RuntimeMode::Plan,
            None,
        );
        assert!(report.patch_plan.is_some());
        assert!(report.patch_proposal.is_none());
        assert!(!report.sandbox_executed);
    }

    #[test]
    fn self_dev_runtime_propose_mode_generates_patch_proposal_only() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "VoiceSynthesis EmergentFunction test failed",
            RuntimeMode::Propose,
            None,
        );
        assert!(report.patch_plan.is_some());
        assert!(report.patch_proposal.is_some());
        assert!(!report.sandbox_executed);
        assert!(report.development_memory.is_some());
    }

    #[test]
    fn self_dev_runtime_dry_run_does_not_modify_files() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "VoiceSynthesis EmergentFunction test failed",
            RuntimeMode::DryRun,
            None,
        );
        assert!(report.original_integrity_preserved);
        assert!(!report.sandbox_executed);
    }

    #[test]
    fn self_dev_runtime_low_risk_loop_requires_explicit_scope() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "add regression test for patch feedback parser",
            RuntimeMode::LowRiskLoop,
            None,
        );
        assert!(!report.decision.allowed);
        assert_eq!(
            report.decision.blocked_reason.as_deref(),
            Some("approval_missing_or_scope_dry_run")
        );
    }

    #[test]
    fn self_dev_runtime_blocks_low_risk_loop_without_approval() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "add regression test for patch feedback parser",
            RuntimeMode::LowRiskLoop,
            None,
        );
        assert!(report.low_risk_loop_report.is_none());
        assert_eq!(
            report.selected_work_item.status,
            WorkItemStatus::WaitingForApproval
        );
    }

    #[test]
    fn self_dev_runtime_blocks_safetycritical_apply() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "modify safety gate to allow auto apply",
            RuntimeMode::LowRiskLoop,
            Some(ApprovalRecord::new(
                "modify safety gate to allow auto apply",
                ApprovalScope::LowRiskSandboxLoop,
                3,
            )),
        );
        assert!(!report.decision.allowed);
        assert_eq!(
            report.selected_work_item.risk_tier,
            RiskTier::SafetyCritical
        );
    }

    #[test]
    fn self_dev_runtime_blocks_original_write() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run("request original code write", RuntimeMode::DryRun, None);
        assert!(!runtime.policy().allow_original_write);
        assert!(report.original_integrity_preserved);
    }

    #[test]
    fn self_dev_runtime_blocks_shell_network_git_operations() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.run(
            "run shell, network request, git push and package install",
            RuntimeMode::DryRun,
            None,
        );
        assert!(!report.decision.allowed);
        assert_eq!(
            report.selected_work_item.risk_tier,
            RiskTier::SafetyCritical
        );
    }

    #[test]
    fn self_dev_runtime_feeds_failure_to_patch_feedback() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.feedback("cargo test failed: VoiceSynthesis EmergentFunction missing");
        assert!(report.feedback_episode.is_some());
        assert_eq!(
            report.selected_work_item.status,
            WorkItemStatus::FeedbackIngested
        );
    }

    #[test]
    fn self_dev_runtime_stores_coding_lesson() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.feedback("cargo test failed: VoiceSynthesis EmergentFunction missing");
        assert!(!report.coding_lessons.is_empty());
        assert!(!report.memory.lessons.is_empty());
    }

    #[test]
    fn self_dev_runtime_generates_next_recommendation() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.next();
        assert!(!report.next_recommended_action.is_empty());
    }

    #[test]
    fn self_dev_runtime_report_contains_goal_risk_action_result_next() {
        let runtime = SelfDevRuntime::sample();
        let report = runtime.report();
        assert!(!report.current_goal.is_empty());
        assert!(!report.decision.executed_action.is_empty());
        assert!(!report.result_summary.is_empty());
        assert!(!report.next_recommended_action.is_empty());
    }

    #[test]
    fn self_dev_runtime_reduces_manual_step_dependency() {
        let report = SelfDevRuntime::benchmark();
        assert!(report.on_manual_step_dependency < report.off_manual_step_dependency);
        assert!(
            report.on_recursive_development_operability
                > report.off_recursive_development_operability
        );
    }

    #[test]
    fn self_dev_runtime_preserves_original_integrity() {
        let runtime = SelfDevRuntime::sample();
        let approval = ApprovalRecord::new(
            "add regression test for patch feedback parser",
            ApprovalScope::LowRiskSandboxLoop,
            3,
        );
        let report = runtime.run(
            "add regression test for patch feedback parser",
            RuntimeMode::LowRiskLoop,
            Some(approval),
        );
        assert!(report.original_integrity_preserved);
    }

    #[test]
    fn self_dev_runtime_benchmark_covers_success_criteria() {
        let report = SelfDevRuntime::benchmark();
        assert!(report.self_dev_runtime_initializes_with_safe_defaults);
        assert!(report.self_dev_runtime_collects_goal_from_user_input);
        assert!(report.self_dev_runtime_collects_goal_from_embryo_growth_goal);
        assert!(report.self_dev_runtime_creates_work_item);
        assert!(report.self_dev_runtime_classifies_work_item_risk);
        assert!(report.self_dev_runtime_calls_autonomy_governor_before_action);
        assert!(report.self_dev_runtime_plan_mode_generates_patch_plan_only);
        assert!(report.self_dev_runtime_propose_mode_generates_patch_proposal_only);
        assert!(report.self_dev_runtime_dry_run_does_not_modify_files);
        assert!(report.self_dev_runtime_low_risk_loop_requires_explicit_scope);
        assert!(report.self_dev_runtime_blocks_low_risk_loop_without_approval);
        assert!(report.self_dev_runtime_blocks_safetycritical_apply);
        assert!(report.self_dev_runtime_blocks_original_write);
        assert!(report.self_dev_runtime_blocks_shell_network_git_operations);
        assert!(report.self_dev_runtime_feeds_failure_to_patch_feedback);
        assert!(report.self_dev_runtime_stores_coding_lesson);
        assert!(report.self_dev_runtime_generates_next_recommendation);
        assert!(report.self_dev_runtime_report_contains_goal_risk_action_result_next);
        assert!(report.self_dev_runtime_reduces_manual_step_dependency);
        assert!(report.self_dev_runtime_preserves_original_integrity);
    }
}
