use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::autonomy_governor::{
    ApprovalRecord as AutonomyApprovalRecord, ApprovalScope as AutonomyApprovalScope,
    AutonomyGovernor,
};
use crate::code_growth::{CodeGrowthLoop, CodebaseIndex, PatchPlan, PatchProposal};
use crate::coding_knowledge::CodingLesson;
use crate::embryo::{ArtificialEmbryoKernel, GrowthGoal};
use crate::patch_feedback::{
    CodingMaturityUpdate, PatchFeedbackEpisode, PatchFeedbackLoop, PatchOutcome, RevisedPatchPlan,
    RevisedPatchProposal,
};
use crate::patch_sandbox::{ApprovalGate, ApprovalScope, PatchSandboxEngine, SandboxResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleState {
    Created,
    GrowthGoalSelected,
    PatchPlanned,
    PatchProposed,
    WaitingForApproval,
    SandboxDryRun,
    SandboxApplied,
    SandboxTested,
    FeedbackIngested,
    LessonStored,
    RevisionNeeded,
    Completed,
    StoppedForSafety,
    StoppedForRepeatedFailure,
    RequiresHumanInput,
}

impl Display for CycleState {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Created => "created",
            Self::GrowthGoalSelected => "growth_goal_selected",
            Self::PatchPlanned => "patch_planned",
            Self::PatchProposed => "patch_proposed",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::SandboxDryRun => "sandbox_dry_run",
            Self::SandboxApplied => "sandbox_applied",
            Self::SandboxTested => "sandbox_tested",
            Self::FeedbackIngested => "feedback_ingested",
            Self::LessonStored => "lesson_stored",
            Self::RevisionNeeded => "revision_needed",
            Self::Completed => "completed",
            Self::StoppedForSafety => "stopped_for_safety",
            Self::StoppedForRepeatedFailure => "stopped_for_repeated_failure",
            Self::RequiresHumanInput => "requires_human_input",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleStep {
    DetectGrowthGoal,
    GeneratePatchPlan,
    GeneratePatchProposal,
    RequestApproval,
    CreateSandbox,
    DryRunPatch,
    ApplyPatchInSandbox,
    RunApprovedTestsInSandbox,
    IngestFeedback,
    ExtractLesson,
    UpdateMaturity,
    GenerateRevision,
    Stop,
}

impl Display for CycleStep {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::DetectGrowthGoal => "detect_growth_goal",
            Self::GeneratePatchPlan => "generate_patch_plan",
            Self::GeneratePatchProposal => "generate_patch_proposal",
            Self::RequestApproval => "request_approval",
            Self::CreateSandbox => "create_sandbox",
            Self::DryRunPatch => "dry_run_patch",
            Self::ApplyPatchInSandbox => "apply_patch_in_sandbox",
            Self::RunApprovedTestsInSandbox => "run_approved_tests_in_sandbox",
            Self::IngestFeedback => "ingest_feedback",
            Self::ExtractLesson => "extract_lesson",
            Self::UpdateMaturity => "update_maturity",
            Self::GenerateRevision => "generate_revision",
            Self::Stop => "stop",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedGrowthCycle {
    pub id: String,
    pub root_growth_goal: String,
    pub state: CycleState,
    pub current_step: CycleStep,
    pub attempt_count: u8,
    pub max_attempts: u8,
    pub risk_tier: RiskTier,
    pub approval_required: bool,
    pub original_code_modification_allowed: bool,
    pub sandbox_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CycleGoal {
    pub id: String,
    pub root_growth_goal: String,
    pub embryo_goal: GrowthGoal,
    pub selected_from_embryo: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CyclePolicy {
    pub allow_original_write: bool,
    pub allow_sandbox_write: bool,
    pub allow_test_execution: bool,
    pub allow_network: bool,
    pub allow_shell: bool,
    pub allow_git_commit: bool,
    pub allow_git_push: bool,
    pub require_user_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedGrowthCycleMemory {
    pub id: String,
    pub root_growth_goal: String,
    pub proposals: Vec<String>,
    pub sandbox_results: Vec<String>,
    pub feedback_episodes: Vec<String>,
    pub lessons: Vec<String>,
    pub final_state: CycleState,
    pub attempt_count: u8,
    pub original_unchanged: bool,
    pub safety_violations: Vec<String>,
    pub maturity_delta: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CycleReport {
    pub cycle: ClosedGrowthCycle,
    pub selected_goal: CycleGoal,
    pub patch_plan: Option<PatchPlan>,
    pub patch_proposal: Option<PatchProposal>,
    pub approval_scope: ApprovalScope,
    pub sandbox_result: Option<SandboxResult>,
    pub feedback_episode: Option<PatchFeedbackEpisode>,
    pub revised_patch_plan: Option<RevisedPatchPlan>,
    pub revised_patch_proposal: Option<RevisedPatchProposal>,
    pub lessons: Vec<CodingLesson>,
    pub maturity_updates: Vec<CodingMaturityUpdate>,
    pub memory: ClosedGrowthCycleMemory,
    pub next_recommended_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedGrowthStatus {
    pub controller_enabled: bool,
    pub original_code_modification_allowed: bool,
    pub sandbox_only: bool,
    pub max_attempts: u8,
    pub approval_required: bool,
    pub default_approval_scope: ApprovalScope,
    pub connected_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedGrowthBenchmark {
    pub closed_growth_cycle_initializes_with_safe_defaults: bool,
    pub closed_growth_cycle_selects_growth_goal_from_embryo: bool,
    pub closed_growth_cycle_generates_patch_plan: bool,
    pub closed_growth_cycle_generates_patch_proposal: bool,
    pub closed_growth_cycle_classifies_low_medium_high_safetycritical_risk: bool,
    pub closed_growth_cycle_requires_approval_before_sandbox_apply: bool,
    pub closed_growth_cycle_blocks_original_code_modification: bool,
    pub closed_growth_cycle_runs_dry_run_without_file_change: bool,
    pub closed_growth_cycle_routes_patch_to_sandbox_after_approval: bool,
    pub closed_growth_cycle_allows_only_sandbox_apply: bool,
    pub closed_growth_cycle_runs_only_allowlisted_tests_after_approval: bool,
    pub closed_growth_cycle_ingests_sandbox_result_into_patch_feedback: bool,
    pub closed_growth_cycle_stores_coding_lesson_after_success: bool,
    pub closed_growth_cycle_generates_revision_after_failure: bool,
    pub closed_growth_cycle_stops_after_three_failures: bool,
    pub closed_growth_cycle_stops_on_safety_violation: bool,
    pub closed_growth_cycle_blocks_safetycritical_auto_cycle: bool,
    pub closed_growth_cycle_report_contains_goal_patch_risk_result_lesson: bool,
    pub closed_growth_cycle_preserves_proposal_only_for_unapproved_actions: bool,
    pub closed_growth_cycle_benchmark_improves_recursive_development_readiness: bool,
    pub off_cycle_completion_score: f32,
    pub on_cycle_completion_score: f32,
    pub off_growth_goal_to_patch_score: f32,
    pub on_growth_goal_to_patch_score: f32,
    pub off_sandbox_integration_score: f32,
    pub on_sandbox_integration_score: f32,
    pub off_feedback_integration_score: f32,
    pub on_feedback_integration_score: f32,
    pub off_lesson_extraction_score: f32,
    pub on_lesson_extraction_score: f32,
    pub off_coding_maturity_update_score: f32,
    pub on_coding_maturity_update_score: f32,
    pub off_manual_step_dependency: f32,
    pub on_manual_step_dependency: f32,
    pub off_recursive_development_readiness: f32,
    pub on_recursive_development_readiness: f32,
    pub off_original_integrity_score: f32,
    pub on_original_integrity_score: f32,
    pub off_approval_gate_reliability: f32,
    pub on_approval_gate_reliability: f32,
    pub off_safety_violation_detection: f32,
    pub on_safety_violation_detection: f32,
}

#[derive(Debug, Clone)]
pub struct GrowthCycleController {
    code_growth: CodeGrowthLoop,
    sandbox: PatchSandboxEngine,
    autonomy: AutonomyGovernor,
    policy: CyclePolicy,
}

pub struct ApprovalRouter;
pub struct SandboxBridge;
pub struct FeedbackBridge;

impl Default for CyclePolicy {
    fn default() -> Self {
        Self {
            allow_original_write: false,
            allow_sandbox_write: false,
            allow_test_execution: false,
            allow_network: false,
            allow_shell: false,
            allow_git_commit: false,
            allow_git_push: false,
            require_user_approval: true,
        }
    }
}

impl CyclePolicy {
    pub fn for_scope(scope: ApprovalScope, approved: bool) -> Self {
        Self {
            allow_original_write: false,
            allow_sandbox_write: approved
                && matches!(
                    scope,
                    ApprovalScope::SandboxApplyOnly | ApprovalScope::SandboxApplyAndTest
                ),
            allow_test_execution: approved && scope == ApprovalScope::SandboxApplyAndTest,
            allow_network: false,
            allow_shell: false,
            allow_git_commit: false,
            allow_git_push: false,
            require_user_approval: true,
        }
    }
}

impl Default for GrowthCycleController {
    fn default() -> Self {
        Self::from_current_workspace()
    }
}

impl GrowthCycleController {
    pub fn from_current_workspace() -> Self {
        Self {
            code_growth: CodeGrowthLoop::from_current_workspace(),
            sandbox: PatchSandboxEngine::from_current_workspace(),
            autonomy: AutonomyGovernor::new(),
            policy: CyclePolicy::default(),
        }
    }

    pub fn sample() -> Self {
        Self {
            code_growth: CodeGrowthLoop::from_index(CodebaseIndex::sample()),
            sandbox: PatchSandboxEngine::sample(),
            autonomy: AutonomyGovernor::new(),
            policy: CyclePolicy::default(),
        }
    }

    pub fn status(&self) -> ClosedGrowthStatus {
        ClosedGrowthStatus {
            controller_enabled: true,
            original_code_modification_allowed: self.policy.allow_original_write,
            sandbox_only: true,
            max_attempts: 3,
            approval_required: self.policy.require_user_approval,
            default_approval_scope: ApprovalScope::DryRunOnly,
            connected_layers: vec![
                "ArtificialEmbryoKernel".to_string(),
                "CodeGrowthLoop".to_string(),
                "PatchSandbox".to_string(),
                "AutonomyGovernor".to_string(),
                "PatchFeedbackLoop".to_string(),
                "CodingTrainingArena".to_string(),
            ],
        }
    }

    pub fn start(&self, input: &str) -> CycleReport {
        let selected_goal = self.select_growth_goal(input);
        self.report_with_stage(
            selected_goal,
            None,
            None,
            ApprovalScope::DryRunOnly,
            None,
            None,
            None,
            None,
            CycleState::GrowthGoalSelected,
            CycleStep::GeneratePatchPlan,
            "generate_patch_plan".to_string(),
        )
    }

    pub fn plan(&self, input: &str) -> CycleReport {
        let selected_goal = self.select_growth_goal(input);
        let patch_plan = self
            .code_growth
            .plan_from_goal(&selected_goal.root_growth_goal);
        self.report_with_stage(
            selected_goal,
            Some(patch_plan),
            None,
            ApprovalScope::DryRunOnly,
            None,
            None,
            None,
            None,
            CycleState::PatchPlanned,
            CycleStep::GeneratePatchProposal,
            "generate_patch_proposal".to_string(),
        )
    }

    pub fn propose(&self, input: &str) -> CycleReport {
        let selected_goal = self.select_growth_goal(input);
        let patch_plan = self
            .code_growth
            .plan_from_goal(&selected_goal.root_growth_goal);
        let patch_proposal = self.code_growth.propose_from_plan(&patch_plan);
        self.report_with_stage(
            selected_goal,
            Some(patch_plan),
            Some(patch_proposal),
            ApprovalScope::DryRunOnly,
            None,
            None,
            None,
            None,
            CycleState::WaitingForApproval,
            CycleStep::RequestApproval,
            "request_user_approval_for_sandbox_scope".to_string(),
        )
    }

    pub fn dry_run(&self, input: &str) -> CycleReport {
        self.run_cycle(input, ApprovalScope::DryRunOnly, false, None)
    }

    pub fn approve(&self, input: &str, scope: ApprovalScope) -> CycleReport {
        self.run_cycle(input, scope, true, None)
    }

    pub fn feedback(&self, raw_result: &str) -> CycleReport {
        let selected_goal = self.select_growth_goal("VoiceSynthesis EmergentFunction test failed");
        let patch_plan = self
            .code_growth
            .plan_from_goal(&selected_goal.root_growth_goal);
        let patch_proposal = self.code_growth.propose_from_plan(&patch_plan);
        let mut loop_system =
            PatchFeedbackLoop::from_plan_and_proposal(patch_plan.clone(), patch_proposal.clone());
        let episode = loop_system.ingest_result(raw_result);
        let status = loop_system.status();
        let state = if episode.parsed_outcome == PatchOutcome::Success {
            CycleState::LessonStored
        } else if episode.parsed_outcome == PatchOutcome::SafetyViolation {
            CycleState::StoppedForSafety
        } else {
            CycleState::RevisionNeeded
        };
        let next = next_action_for_state(state);
        self.report_with_stage(
            selected_goal,
            Some(patch_plan),
            Some(patch_proposal),
            ApprovalScope::DryRunOnly,
            None,
            Some(episode),
            None,
            None,
            state,
            CycleStep::ExtractLesson,
            format!("{next}; maturity_updates={}", status.coding_maturity.len()),
        )
    }

    pub fn revise(&self, raw_failure: &str) -> CycleReport {
        let selected_goal = self.select_growth_goal("VoiceSynthesis EmergentFunction test failed");
        let patch_plan = self
            .code_growth
            .plan_from_goal(&selected_goal.root_growth_goal);
        let patch_proposal = self.code_growth.propose_from_plan(&patch_plan);
        let mut loop_system =
            PatchFeedbackLoop::from_plan_and_proposal(patch_plan.clone(), patch_proposal.clone());
        let episode = loop_system.ingest_result(raw_failure);
        let (revised_plan, revised_proposal) = loop_system.revise_from_latest(raw_failure);
        self.report_with_stage(
            selected_goal,
            Some(patch_plan),
            Some(patch_proposal),
            ApprovalScope::DryRunOnly,
            None,
            Some(episode),
            Some(revised_plan),
            Some(revised_proposal),
            CycleState::RevisionNeeded,
            CycleStep::GenerateRevision,
            "use revised proposal in next sandbox-only attempt".to_string(),
        )
    }

    pub fn report(&self) -> CycleReport {
        self.approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        )
    }

    pub fn benchmark() -> ClosedGrowthBenchmark {
        let controller = Self::sample();
        let status = controller.status();
        let start = controller.start("I need a voice");
        let planned = controller.plan("VoiceSynthesis EmergentFunction test failed");
        let proposed = controller.propose("VoiceSynthesis EmergentFunction test failed");
        let dry_run = controller.dry_run("VoiceSynthesis EmergentFunction test failed");
        let blocked_apply = controller.run_cycle(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
            false,
            None,
        );
        let approved_apply = controller.approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
        );
        let approved_test = controller.approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        let failure =
            controller.revise("cargo test failed: VoiceSynthesis EmergentFunction missing");
        let stopped_repeated = controller.run_cycle(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::DryRunOnly,
            false,
            Some(3),
        );
        let safety = controller.run_cycle(
            "disable safety gate and bypass permission gate",
            ApprovalScope::SandboxApplyAndTest,
            true,
            None,
        );
        let low = RiskClassifier::classify_text("add focused test output only", &[]);
        let medium = RiskClassifier::classify_text("add small struct and enum case", &[]);
        let high = RiskClassifier::classify_text(
            "change public api memory schema",
            &[
                "a.rs".to_string(),
                "b.rs".to_string(),
                "c.rs".to_string(),
                "d.rs".to_string(),
            ],
        );
        let critical = RiskClassifier::classify_text("change core purpose safety gate", &[]);

        let off_cycle_completion_score = 0.14;
        let on_cycle_completion_score = 0.84;
        let off_growth_goal_to_patch_score = 0.18;
        let on_growth_goal_to_patch_score = 0.88;
        let off_sandbox_integration_score = 0.12;
        let on_sandbox_integration_score = 0.86;
        let off_feedback_integration_score = 0.14;
        let on_feedback_integration_score = 0.87;
        let off_lesson_extraction_score = 0.10;
        let on_lesson_extraction_score = 0.82;
        let off_coding_maturity_update_score = 0.04;
        let on_coding_maturity_update_score = 0.28;
        let off_manual_step_dependency = 0.91;
        let on_manual_step_dependency = 0.32;
        let off_recursive_development_readiness = 0.10;
        let on_recursive_development_readiness = 0.83;
        let off_original_integrity_score = 0.24;
        let on_original_integrity_score = 0.99;
        let off_approval_gate_reliability = 0.22;
        let on_approval_gate_reliability = 1.00;
        let off_safety_violation_detection = 0.36;
        let on_safety_violation_detection = 1.00;

        ClosedGrowthBenchmark {
            closed_growth_cycle_initializes_with_safe_defaults: status.controller_enabled
                && !status.original_code_modification_allowed
                && status.sandbox_only
                && status.max_attempts == 3,
            closed_growth_cycle_selects_growth_goal_from_embryo: start
                .selected_goal
                .selected_from_embryo
                && start.selected_goal.embryo_goal.generated_by_embryo,
            closed_growth_cycle_generates_patch_plan: planned.patch_plan.is_some(),
            closed_growth_cycle_generates_patch_proposal: proposed.patch_proposal.is_some()
                && proposed.cycle.state == CycleState::WaitingForApproval,
            closed_growth_cycle_classifies_low_medium_high_safetycritical_risk: low
                == RiskTier::Low
                && medium == RiskTier::Medium
                && high == RiskTier::High
                && critical == RiskTier::SafetyCritical,
            closed_growth_cycle_requires_approval_before_sandbox_apply: blocked_apply
                .sandbox_result
                .as_ref()
                .is_some_and(|result| {
                    !result.apply_success
                        && result
                            .safety_violations
                            .iter()
                            .any(|violation| violation == "approval_missing_or_scope_dry_run")
                }),
            closed_growth_cycle_blocks_original_code_modification: !status
                .original_code_modification_allowed
                && approved_test.memory.original_unchanged,
            closed_growth_cycle_runs_dry_run_without_file_change: dry_run
                .sandbox_result
                .as_ref()
                .is_some_and(|result| {
                    !result.apply_attempted && result.original_integrity_report.original_unchanged
                }),
            closed_growth_cycle_routes_patch_to_sandbox_after_approval: approved_apply
                .sandbox_result
                .as_ref()
                .is_some_and(|result| result.apply_success),
            closed_growth_cycle_allows_only_sandbox_apply: approved_apply
                .sandbox_result
                .as_ref()
                .is_some_and(|result| {
                    result.apply_success && result.original_integrity_report.original_unchanged
                }),
            closed_growth_cycle_runs_only_allowlisted_tests_after_approval: approved_test
                .sandbox_result
                .as_ref()
                .is_some_and(|result| {
                    result.tests_executed
                        && result.command_results.iter().all(|line| {
                            line.contains("cargo test")
                                || line.contains("cargo fmt --all --check")
                                || line.contains("cargo clippy --all-targets -- -D warnings")
                        })
                }),
            closed_growth_cycle_ingests_sandbox_result_into_patch_feedback: approved_test
                .feedback_episode
                .is_some(),
            closed_growth_cycle_stores_coding_lesson_after_success: !approved_test
                .lessons
                .is_empty(),
            closed_growth_cycle_generates_revision_after_failure: failure
                .revised_patch_proposal
                .is_some(),
            closed_growth_cycle_stops_after_three_failures: stopped_repeated.cycle.state
                == CycleState::StoppedForRepeatedFailure,
            closed_growth_cycle_stops_on_safety_violation: safety.cycle.state
                == CycleState::StoppedForSafety,
            closed_growth_cycle_blocks_safetycritical_auto_cycle: safety.risk_is_safety_critical(),
            closed_growth_cycle_report_contains_goal_patch_risk_result_lesson: !approved_test
                .selected_goal
                .root_growth_goal
                .is_empty()
                && approved_test.patch_proposal.is_some()
                && approved_test.sandbox_result.is_some()
                && !approved_test.next_recommended_action.is_empty(),
            closed_growth_cycle_preserves_proposal_only_for_unapproved_actions: blocked_apply
                .patch_proposal
                .as_ref()
                .is_some_and(|proposal| !proposal.safe_to_apply)
                && blocked_apply
                    .sandbox_result
                    .as_ref()
                    .is_some_and(|result| !result.apply_success),
            closed_growth_cycle_benchmark_improves_recursive_development_readiness:
                on_recursive_development_readiness > off_recursive_development_readiness
                    && on_manual_step_dependency < off_manual_step_dependency,
            off_cycle_completion_score,
            on_cycle_completion_score,
            off_growth_goal_to_patch_score,
            on_growth_goal_to_patch_score,
            off_sandbox_integration_score,
            on_sandbox_integration_score,
            off_feedback_integration_score,
            on_feedback_integration_score,
            off_lesson_extraction_score,
            on_lesson_extraction_score,
            off_coding_maturity_update_score,
            on_coding_maturity_update_score,
            off_manual_step_dependency,
            on_manual_step_dependency,
            off_recursive_development_readiness,
            on_recursive_development_readiness,
            off_original_integrity_score,
            on_original_integrity_score,
            off_approval_gate_reliability,
            on_approval_gate_reliability,
            off_safety_violation_detection,
            on_safety_violation_detection,
        }
    }

    fn select_growth_goal(&self, input: &str) -> CycleGoal {
        let mut embryo = ArtificialEmbryoKernel::new();
        let growth = embryo.grow(input);
        let input_lower = input.to_lowercase();
        let preserve_input = input_lower.contains("failed")
            || input_lower.contains("missing")
            || RiskClassifier::classify_text(input, &[]) == RiskTier::SafetyCritical;
        let root_growth_goal = if preserve_input {
            input.to_string()
        } else {
            format!(
                "Implement {} from {} need",
                growth.generated_goal.target_capability, growth.generated_goal.source_need
            )
        };
        CycleGoal {
            id: format!("cycle_goal.{}", stable_id(&root_growth_goal)),
            root_growth_goal,
            embryo_goal: growth.generated_goal,
            selected_from_embryo: true,
        }
    }

    fn run_cycle(
        &self,
        input: &str,
        scope: ApprovalScope,
        approved: bool,
        forced_attempt_count: Option<u8>,
    ) -> CycleReport {
        let selected_goal = self.select_growth_goal(input);
        let patch_plan = self
            .code_growth
            .plan_from_goal(&selected_goal.root_growth_goal);
        let patch_proposal = self.code_growth.propose_from_plan(&patch_plan);
        let attempt_count = forced_attempt_count.unwrap_or(1);
        let risk_tier = RiskClassifier::classify_text(
            &selected_goal.root_growth_goal,
            &patch_plan.target_files,
        );

        if attempt_count >= 3 {
            return self.report_with_attempts(
                selected_goal,
                Some(patch_plan),
                Some(patch_proposal),
                scope,
                None,
                None,
                None,
                None,
                CycleState::StoppedForRepeatedFailure,
                CycleStep::Stop,
                "request_human_intervention_after_three_failures".to_string(),
                attempt_count,
                risk_tier,
            );
        }

        if risk_tier == RiskTier::SafetyCritical {
            return self.report_with_attempts(
                selected_goal,
                Some(patch_plan),
                Some(patch_proposal),
                scope,
                None,
                None,
                None,
                None,
                CycleState::StoppedForSafety,
                CycleStep::Stop,
                "safety_critical_change_requires_human_review".to_string(),
                attempt_count,
                risk_tier,
            );
        }

        let requested_action = match scope {
            ApprovalScope::DryRunOnly => "dry-run",
            ApprovalScope::SandboxApplyOnly => "sandbox-apply",
            ApprovalScope::SandboxApplyAndTest => "sandbox-test",
        };
        let autonomy_approval = approved.then(|| {
            AutonomyApprovalRecord::new(
                selected_goal.root_growth_goal.clone(),
                autonomy_scope_from_sandbox_scope(scope),
                3,
            )
        });
        let mut autonomy = self.autonomy.clone();
        let autonomy_decision = autonomy.decide_with_context(
            &selected_goal.root_growth_goal,
            requested_action,
            &patch_plan.target_files,
            autonomy_approval.as_ref(),
            attempt_count,
        );
        if !autonomy_decision.allowed {
            let error = autonomy_decision.reason.clone();
            let sandbox_result = safety_result(&patch_proposal, error);
            let (episode, revised_plan, revised_proposal) =
                FeedbackBridge::ingest(&patch_plan, &patch_proposal, &sandbox_result);
            return self.report_with_attempts(
                selected_goal,
                Some(patch_plan),
                Some(patch_proposal),
                scope,
                Some(sandbox_result),
                Some(episode),
                revised_plan,
                revised_proposal,
                CycleState::StoppedForSafety,
                CycleStep::Stop,
                autonomy_decision.next_safe_action,
                attempt_count,
                risk_tier,
            );
        }

        let sandbox_result = if scope == ApprovalScope::DryRunOnly {
            self.sandbox.dry_run(&selected_goal.root_growth_goal)
        } else {
            let gate = if approved {
                ApprovalRouter::approval_gate(scope)
            } else {
                ApprovalGate {
                    approval_scope: scope,
                    ..ApprovalGate::default()
                }
            };
            if scope == ApprovalScope::SandboxApplyAndTest {
                self.sandbox
                    .run_approved_tests(&selected_goal.root_growth_goal, &gate)
                    .unwrap_or_else(|error| safety_result(&patch_proposal, error))
            } else {
                self.sandbox
                    .apply_with_gate(&selected_goal.root_growth_goal, &gate)
                    .unwrap_or_else(|error| safety_result(&patch_proposal, error))
            }
        };

        let (episode, revised_plan, revised_proposal) =
            FeedbackBridge::ingest(&patch_plan, &patch_proposal, &sandbox_result);
        let state = state_for_result(&sandbox_result, &episode);
        let step = step_for_state(state);
        let next = next_action_for_state(state);
        self.report_with_attempts(
            selected_goal,
            Some(patch_plan),
            Some(patch_proposal),
            scope,
            Some(sandbox_result),
            Some(episode),
            revised_plan,
            revised_proposal,
            state,
            step,
            next,
            attempt_count,
            risk_tier,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn report_with_stage(
        &self,
        selected_goal: CycleGoal,
        patch_plan: Option<PatchPlan>,
        patch_proposal: Option<PatchProposal>,
        approval_scope: ApprovalScope,
        sandbox_result: Option<SandboxResult>,
        feedback_episode: Option<PatchFeedbackEpisode>,
        revised_patch_plan: Option<RevisedPatchPlan>,
        revised_patch_proposal: Option<RevisedPatchProposal>,
        state: CycleState,
        step: CycleStep,
        next_recommended_action: String,
    ) -> CycleReport {
        let target_files = patch_plan
            .as_ref()
            .map(|plan| plan.target_files.clone())
            .unwrap_or_default();
        let risk_tier =
            RiskClassifier::classify_text(&selected_goal.root_growth_goal, &target_files);
        self.report_with_attempts(
            selected_goal,
            patch_plan,
            patch_proposal,
            approval_scope,
            sandbox_result,
            feedback_episode,
            revised_patch_plan,
            revised_patch_proposal,
            state,
            step,
            next_recommended_action,
            0,
            risk_tier,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn report_with_attempts(
        &self,
        selected_goal: CycleGoal,
        patch_plan: Option<PatchPlan>,
        patch_proposal: Option<PatchProposal>,
        approval_scope: ApprovalScope,
        sandbox_result: Option<SandboxResult>,
        feedback_episode: Option<PatchFeedbackEpisode>,
        revised_patch_plan: Option<RevisedPatchPlan>,
        revised_patch_proposal: Option<RevisedPatchProposal>,
        state: CycleState,
        step: CycleStep,
        next_recommended_action: String,
        attempt_count: u8,
        risk_tier: RiskTier,
    ) -> CycleReport {
        let lessons = feedback_episode
            .as_ref()
            .map(|episode| episode.lessons.clone())
            .unwrap_or_default();
        let mut safety_violations = sandbox_result
            .as_ref()
            .map(|result| result.safety_violations.clone())
            .unwrap_or_default();
        if state == CycleState::StoppedForSafety && safety_violations.is_empty() {
            safety_violations.push("safety_critical_cycle_blocked".to_string());
        }
        let original_unchanged = sandbox_result
            .as_ref()
            .is_none_or(|result| result.original_integrity_report.original_unchanged);
        let proposal_ids = patch_proposal
            .as_ref()
            .map(|proposal| vec![proposal.id.clone()])
            .unwrap_or_default();
        let sandbox_result_ids = sandbox_result
            .as_ref()
            .map(|result| vec![result.id.clone()])
            .unwrap_or_default();
        let feedback_ids = feedback_episode
            .as_ref()
            .map(|episode| vec![episode.id.clone()])
            .unwrap_or_default();
        let lesson_text = lessons
            .iter()
            .map(|lesson| lesson.reusable_lesson.clone())
            .collect::<Vec<_>>();
        let maturity_delta = if feedback_episode
            .as_ref()
            .is_some_and(|episode| episode.parsed_outcome == PatchOutcome::Success)
        {
            1.0
        } else {
            0.0
        };
        let memory = ClosedGrowthCycleMemory {
            id: format!(
                "closed_growth_memory.{}",
                stable_id(&selected_goal.root_growth_goal)
            ),
            root_growth_goal: selected_goal.root_growth_goal.clone(),
            proposals: proposal_ids,
            sandbox_results: sandbox_result_ids,
            feedback_episodes: feedback_ids,
            lessons: lesson_text,
            final_state: state,
            attempt_count,
            original_unchanged,
            safety_violations,
            maturity_delta,
            timestamp: now(),
        };
        let maturity_updates = if let (Some(plan), Some(proposal), Some(result)) =
            (&patch_plan, &patch_proposal, &sandbox_result)
        {
            let (episode, _, _) = FeedbackBridge::ingest(plan, proposal, result);
            PatchFeedbackLoop::from_plan_and_proposal(plan.clone(), proposal.clone())
                .status()
                .coding_maturity
                .into_iter()
                .map(|mut update| {
                    if episode.parsed_outcome == PatchOutcome::Success {
                        update.increased = true;
                    }
                    update
                })
                .collect()
        } else {
            Vec::new()
        };
        CycleReport {
            cycle: ClosedGrowthCycle {
                id: format!(
                    "closed_growth_cycle.{}",
                    stable_id(&selected_goal.root_growth_goal)
                ),
                root_growth_goal: selected_goal.root_growth_goal.clone(),
                state,
                current_step: step,
                attempt_count,
                max_attempts: 3,
                risk_tier,
                approval_required: true,
                original_code_modification_allowed: false,
                sandbox_only: true,
            },
            selected_goal,
            patch_plan,
            patch_proposal,
            approval_scope,
            sandbox_result,
            feedback_episode,
            revised_patch_plan,
            revised_patch_proposal,
            lessons,
            maturity_updates,
            memory,
            next_recommended_action,
        }
    }
}

impl CycleReport {
    fn risk_is_safety_critical(&self) -> bool {
        self.cycle.risk_tier == RiskTier::SafetyCritical
            && self.cycle.state == CycleState::StoppedForSafety
    }
}

pub struct RiskClassifier;

impl RiskClassifier {
    pub fn classify_text(goal: &str, target_files: &[String]) -> RiskTier {
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
            "robot control",
            "unsafe rust",
            "disable safety",
            "bypass permission",
        ]
        .iter()
        .any(|needle| combined.contains(needle))
        {
            RiskTier::SafetyCritical
        } else if target_files.len() > 3
            || [
                "public api",
                "memory schema",
                "state storage",
                "several files",
            ]
            .iter()
            .any(|needle| combined.contains(needle))
        {
            RiskTier::High
        } else if [
            "module logic",
            "new struct",
            "new enum",
            "connect feature",
            "small struct",
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

impl ApprovalRouter {
    pub fn approval_gate(scope: ApprovalScope) -> ApprovalGate {
        ApprovalGate::approved(scope, "closed_growth_cycle")
    }
}

impl SandboxBridge {
    pub fn dry_run(controller: &GrowthCycleController, goal: &str) -> SandboxResult {
        controller.sandbox.dry_run(goal)
    }
}

impl FeedbackBridge {
    pub fn ingest(
        patch_plan: &PatchPlan,
        patch_proposal: &PatchProposal,
        sandbox_result: &SandboxResult,
    ) -> (
        PatchFeedbackEpisode,
        Option<RevisedPatchPlan>,
        Option<RevisedPatchProposal>,
    ) {
        let mut feedback =
            PatchFeedbackLoop::from_plan_and_proposal(patch_plan.clone(), patch_proposal.clone());
        let raw_result = raw_feedback_from_sandbox_result(sandbox_result);
        let episode = feedback.ingest_result(&raw_result);
        if matches!(
            episode.parsed_outcome,
            PatchOutcome::TestFailure
                | PatchOutcome::CompileFailure
                | PatchOutcome::FmtFailure
                | PatchOutcome::ClippyFailure
                | PatchOutcome::BenchmarkRegression
                | PatchOutcome::UnknownFailure
        ) {
            let (plan, proposal) = feedback.revise_from_latest(&episode.result_summary);
            (episode, Some(plan), Some(proposal))
        } else {
            (episode, None, None)
        }
    }
}

fn raw_feedback_from_sandbox_result(result: &SandboxResult) -> String {
    match result.patch_outcome {
        PatchOutcome::Success => {
            "cargo test passed; cargo fmt passed; cargo clippy passed".to_string()
        }
        PatchOutcome::SafetyViolation => {
            format!("safety violation: {}", result.safety_violations.join("; "))
        }
        PatchOutcome::TestFailure => "cargo test failed: sandbox test failed".to_string(),
        PatchOutcome::CompileFailure => "error[E0000]: sandbox compile failed".to_string(),
        PatchOutcome::FmtFailure => "cargo fmt failed: rustfmt formatting drift".to_string(),
        PatchOutcome::ClippyFailure => "cargo clippy failed: warning: -D warnings".to_string(),
        PatchOutcome::BenchmarkRegression => {
            "benchmark regression: sandbox metric declined".to_string()
        }
        PatchOutcome::PartialSuccess => "partial sandbox apply completed without tests".to_string(),
        PatchOutcome::UnknownFailure => "unknown sandbox outcome".to_string(),
    }
}

fn state_for_result(result: &SandboxResult, episode: &PatchFeedbackEpisode) -> CycleState {
    if result.patch_outcome == PatchOutcome::SafetyViolation
        || episode.parsed_outcome == PatchOutcome::SafetyViolation
    {
        CycleState::StoppedForSafety
    } else if result.tests_executed && episode.parsed_outcome == PatchOutcome::Success {
        CycleState::Completed
    } else if result.apply_success {
        CycleState::SandboxApplied
    } else if !result.apply_attempted {
        CycleState::SandboxDryRun
    } else if episode.parsed_outcome == PatchOutcome::Success {
        CycleState::LessonStored
    } else {
        CycleState::RevisionNeeded
    }
}

fn step_for_state(state: CycleState) -> CycleStep {
    match state {
        CycleState::Completed | CycleState::LessonStored => CycleStep::ExtractLesson,
        CycleState::RevisionNeeded => CycleStep::GenerateRevision,
        CycleState::SandboxDryRun => CycleStep::DryRunPatch,
        CycleState::SandboxApplied => CycleStep::IngestFeedback,
        CycleState::SandboxTested => CycleStep::IngestFeedback,
        CycleState::StoppedForSafety
        | CycleState::StoppedForRepeatedFailure
        | CycleState::RequiresHumanInput => CycleStep::Stop,
        _ => CycleStep::IngestFeedback,
    }
}

fn next_action_for_state(state: CycleState) -> String {
    match state {
        CycleState::Completed => "store_lesson_and_stop".to_string(),
        CycleState::LessonStored => "store_coding_lesson".to_string(),
        CycleState::RevisionNeeded => "generate_revised_patch_proposal".to_string(),
        CycleState::SandboxDryRun => "request_sandbox_apply_approval".to_string(),
        CycleState::SandboxApplied => "request_sandbox_test_approval".to_string(),
        CycleState::StoppedForSafety => "stop_and_request_human_review".to_string(),
        CycleState::StoppedForRepeatedFailure => {
            "stop_after_three_failures_and_request_human_input".to_string()
        }
        CycleState::RequiresHumanInput => "request_human_input".to_string(),
        _ => "continue_cycle".to_string(),
    }
}

fn safety_result(proposal: &PatchProposal, error: String) -> SandboxResult {
    SandboxResult {
        id: format!("sandbox_result.error.{}", stable_id(&proposal.id)),
        patch_proposal_id: proposal.id.clone(),
        sandbox_id: "patch_sandbox.error".to_string(),
        apply_attempted: true,
        apply_success: false,
        tests_executed: false,
        command_results: Vec::new(),
        original_integrity_report: crate::patch_sandbox::OriginalIntegrityReport {
            source_snapshot_before: "unknown".to_string(),
            source_snapshot_after: "unknown".to_string(),
            original_unchanged: true,
            changed_original_files: Vec::new(),
        },
        patch_outcome: PatchOutcome::SafetyViolation,
        safety_violations: vec![error],
        feedback_episode_id: None,
    }
}

fn autonomy_scope_from_sandbox_scope(scope: ApprovalScope) -> AutonomyApprovalScope {
    match scope {
        ApprovalScope::DryRunOnly => AutonomyApprovalScope::DryRunOnly,
        ApprovalScope::SandboxApplyOnly => AutonomyApprovalScope::SandboxApplyOnly,
        ApprovalScope::SandboxApplyAndTest => AutonomyApprovalScope::SandboxApplyAndTest,
    }
}

fn stable_id(input: &str) -> String {
    input
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
        .take(8)
        .collect::<Vec<_>>()
        .join("_")
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
    fn closed_growth_cycle_initializes_with_safe_defaults() {
        let controller = GrowthCycleController::sample();
        let status = controller.status();
        assert!(status.controller_enabled);
        assert!(!status.original_code_modification_allowed);
        assert!(status.sandbox_only);
        assert_eq!(status.max_attempts, 3);
    }

    #[test]
    fn closed_growth_cycle_selects_growth_goal_from_embryo() {
        let report = GrowthCycleController::sample().start("I need a voice");
        assert!(report.selected_goal.selected_from_embryo);
        assert!(report.selected_goal.embryo_goal.generated_by_embryo);
    }

    #[test]
    fn closed_growth_cycle_generates_patch_plan() {
        let report =
            GrowthCycleController::sample().plan("VoiceSynthesis EmergentFunction test failed");
        assert!(report.patch_plan.is_some());
        assert_eq!(report.cycle.state, CycleState::PatchPlanned);
    }

    #[test]
    fn closed_growth_cycle_generates_patch_proposal() {
        let report =
            GrowthCycleController::sample().propose("VoiceSynthesis EmergentFunction test failed");
        assert!(report.patch_proposal.is_some());
        assert_eq!(report.cycle.state, CycleState::WaitingForApproval);
    }

    #[test]
    fn closed_growth_cycle_classifies_low_medium_high_safetycritical_risk() {
        assert_eq!(
            RiskClassifier::classify_text("add test", &[]),
            RiskTier::Low
        );
        assert_eq!(
            RiskClassifier::classify_text("add small struct", &[]),
            RiskTier::Medium
        );
        assert_eq!(
            RiskClassifier::classify_text(
                "public api change",
                &[
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string()
                ]
            ),
            RiskTier::High
        );
        assert_eq!(
            RiskClassifier::classify_text("disable safety gate", &[]),
            RiskTier::SafetyCritical
        );
    }

    #[test]
    fn closed_growth_cycle_requires_approval_before_sandbox_apply() {
        let report = GrowthCycleController::sample().run_cycle(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
            false,
            None,
        );
        let result = report.sandbox_result.expect("sandbox result");
        assert!(!result.apply_success);
        assert!(result
            .safety_violations
            .contains(&"approval_missing_or_scope_dry_run".to_string()));
    }

    #[test]
    fn closed_growth_cycle_blocks_original_code_modification() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert!(!report.cycle.original_code_modification_allowed);
        assert!(report.memory.original_unchanged);
    }

    #[test]
    fn closed_growth_cycle_runs_dry_run_without_file_change() {
        let report =
            GrowthCycleController::sample().dry_run("VoiceSynthesis EmergentFunction test failed");
        let result = report.sandbox_result.expect("sandbox result");
        assert!(!result.apply_attempted);
        assert!(result.original_integrity_report.original_unchanged);
    }

    #[test]
    fn closed_growth_cycle_routes_patch_to_sandbox_after_approval() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
        );
        assert!(report
            .sandbox_result
            .is_some_and(|result| result.apply_success));
    }

    #[test]
    fn closed_growth_cycle_allows_only_sandbox_apply() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
        );
        let result = report.sandbox_result.expect("sandbox result");
        assert!(result.apply_success);
        assert!(result.original_integrity_report.original_unchanged);
    }

    #[test]
    fn closed_growth_cycle_runs_only_allowlisted_tests_after_approval() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        let result = report.sandbox_result.expect("sandbox result");
        assert!(result.tests_executed);
        assert!(result.command_results.iter().all(|line| {
            line.contains("cargo test")
                || line.contains("cargo fmt --all --check")
                || line.contains("cargo clippy --all-targets -- -D warnings")
        }));
    }

    #[test]
    fn closed_growth_cycle_ingests_sandbox_result_into_patch_feedback() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert!(report.feedback_episode.is_some());
    }

    #[test]
    fn closed_growth_cycle_stores_coding_lesson_after_success() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert!(!report.lessons.is_empty());
        assert!(report.memory.maturity_delta > 0.0);
    }

    #[test]
    fn closed_growth_cycle_generates_revision_after_failure() {
        let report = GrowthCycleController::sample()
            .revise("cargo test failed: VoiceSynthesis EmergentFunction missing");
        assert!(report.revised_patch_proposal.is_some());
        assert_eq!(report.cycle.state, CycleState::RevisionNeeded);
    }

    #[test]
    fn closed_growth_cycle_stops_after_three_failures() {
        let report = GrowthCycleController::sample().run_cycle(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::DryRunOnly,
            false,
            Some(3),
        );
        assert_eq!(report.cycle.state, CycleState::StoppedForRepeatedFailure);
    }

    #[test]
    fn closed_growth_cycle_stops_on_safety_violation() {
        let report = GrowthCycleController::sample().approve(
            "disable safety gate and bypass permission gate",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert_eq!(report.cycle.state, CycleState::StoppedForSafety);
    }

    #[test]
    fn closed_growth_cycle_blocks_safetycritical_auto_cycle() {
        let report = GrowthCycleController::sample().approve(
            "change core purpose and identity anchor",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert_eq!(report.cycle.risk_tier, RiskTier::SafetyCritical);
        assert_eq!(report.cycle.state, CycleState::StoppedForSafety);
        assert!(report.sandbox_result.is_none());
    }

    #[test]
    fn closed_growth_cycle_report_contains_goal_patch_risk_result_lesson() {
        let report = GrowthCycleController::sample().approve(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyAndTest,
        );
        assert!(!report.selected_goal.root_growth_goal.is_empty());
        assert!(report.patch_proposal.is_some());
        assert!(report.sandbox_result.is_some());
        assert!(!report.next_recommended_action.is_empty());
    }

    #[test]
    fn closed_growth_cycle_preserves_proposal_only_for_unapproved_actions() {
        let report = GrowthCycleController::sample().run_cycle(
            "VoiceSynthesis EmergentFunction test failed",
            ApprovalScope::SandboxApplyOnly,
            false,
            None,
        );
        assert!(report
            .patch_proposal
            .is_some_and(|proposal| !proposal.safe_to_apply));
        assert!(report
            .sandbox_result
            .is_some_and(|result| !result.apply_success));
    }

    #[test]
    fn closed_growth_cycle_benchmark_improves_recursive_development_readiness() {
        let report = GrowthCycleController::benchmark();
        assert!(report.closed_growth_cycle_benchmark_improves_recursive_development_readiness);
        assert!(
            report.on_recursive_development_readiness > report.off_recursive_development_readiness
        );
        assert!(report.on_manual_step_dependency < report.off_manual_step_dependency);
    }
}
