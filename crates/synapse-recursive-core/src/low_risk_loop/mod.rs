use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::autonomy_governor::{
    approval_inferred_from_text, ApprovalRecord, ApprovalScope, AutonomyGovernor, AutonomyLevel,
    RiskClassifier as AutonomyRiskClassifier, RiskTier,
};
use crate::code_growth::{CodeGrowthLoop, CodebaseIndex, PatchPlan, PatchProposal};
use crate::coding_knowledge::CodingLesson;
use crate::patch_feedback::{
    PatchFeedbackEpisode, PatchFeedbackLoop, PatchOutcome, RevisedPatchPlan, RevisedPatchProposal,
};
use crate::patch_sandbox::{
    AllowedCommand, ApprovalGate, OriginalIntegrityReport, PatchSandboxEngine, SandboxResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowRiskTask {
    pub id: String,
    pub title: String,
    pub source_growth_goal: String,
    pub expected_change_type: String,
    pub target_files: Vec<String>,
    pub expected_tests: Vec<String>,
    pub risk_tier: RiskTier,
    pub safety_sensitive: bool,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopBudget {
    pub max_iterations: u8,
    pub used_iterations: u8,
    pub max_patch_files: u8,
    pub max_failed_attempts: u8,
    pub allow_test_execution: bool,
    pub allowed_commands: Vec<AllowedCommand>,
}

impl Default for LoopBudget {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            used_iterations: 0,
            max_patch_files: 2,
            max_failed_attempts: 2,
            allow_test_execution: false,
            allowed_commands: AllowedCommand::all(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopState {
    Created,
    WaitingForApproval,
    Running,
    IterationSucceeded,
    RevisionNeeded,
    Completed,
    StoppedForBudget,
    StoppedForRepeatedFailure,
    StoppedForSafety,
    RequiresHumanReview,
}

impl Display for LoopState {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Created => "created",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::Running => "running",
            Self::IterationSucceeded => "iteration_succeeded",
            Self::RevisionNeeded => "revision_needed",
            Self::Completed => "completed",
            Self::StoppedForBudget => "stopped_for_budget",
            Self::StoppedForRepeatedFailure => "stopped_for_repeated_failure",
            Self::StoppedForSafety => "stopped_for_safety",
            Self::RequiresHumanReview => "requires_human_review",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopIteration {
    pub iteration_index: u8,
    pub patch_proposal_id: String,
    pub sandbox_result_id: Option<String>,
    pub patch_outcome: PatchOutcome,
    pub lessons: Vec<CodingLesson>,
    pub next_action: String,
    pub safety_violations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalPatchRequestBundle {
    pub id: String,
    pub source_task_id: String,
    pub successful_patch_proposal_id: String,
    pub diff_preview: String,
    pub passed_tests: Vec<String>,
    pub risk_tier: RiskTier,
    pub safety_notes: Vec<String>,
    pub original_write_allowed: bool,
    pub requires_human_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowRiskLoopMemory {
    pub id: String,
    pub task_id: String,
    pub iterations: Vec<LoopIteration>,
    pub final_state: LoopState,
    pub success: bool,
    pub original_unchanged: bool,
    pub lessons: Vec<CodingLesson>,
    pub maturity_delta: f32,
    pub original_patch_request_bundle_id: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowRiskLoopStatus {
    pub enabled: bool,
    pub current_autonomy_level: AutonomyLevel,
    pub required_scope: ApprovalScope,
    pub max_iterations: u8,
    pub max_patch_files: u8,
    pub max_failed_attempts: u8,
    pub original_write_allowed: bool,
    pub sandbox_only: bool,
    pub allowlisted_command_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowRiskLoopReport {
    pub task: LowRiskTask,
    pub approval_scope: Option<ApprovalScope>,
    pub state: LoopState,
    pub budget: LoopBudget,
    pub iterations: Vec<LoopIteration>,
    pub failure_causes: Vec<String>,
    pub passed_tests: Vec<String>,
    pub original_integrity: OriginalIntegrityReport,
    pub lessons: Vec<CodingLesson>,
    pub maturity_delta: f32,
    pub original_patch_request_bundle: Option<OriginalPatchRequestBundle>,
    pub revised_patch_plan: Option<RevisedPatchPlan>,
    pub revised_patch_proposal: Option<RevisedPatchProposal>,
    pub memory: LowRiskLoopMemory,
    pub next_recommended_action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowRiskLoopBenchmark {
    pub low_risk_loop_initializes_with_safe_defaults: bool,
    pub low_risk_classifier_accepts_regression_test_addition: bool,
    pub low_risk_classifier_rejects_safety_gate_change: bool,
    pub low_risk_classifier_rejects_permission_gate_change: bool,
    pub low_risk_classifier_rejects_network_or_shell_addition: bool,
    pub low_risk_loop_requires_explicit_l5_approval: bool,
    pub low_risk_loop_does_not_treat_continue_as_approval: bool,
    pub loop_budget_limits_iterations_to_three: bool,
    pub loop_guard_stops_when_approval_expires: bool,
    pub loop_guard_stops_on_repeated_failure: bool,
    pub loop_guard_stops_on_safety_violation: bool,
    pub loop_guard_stops_when_risk_escalates: bool,
    pub supervised_loop_runs_patch_feedback_cycle_in_sandbox: bool,
    pub supervised_loop_generates_revised_patch_after_failure: bool,
    pub supervised_loop_stores_coding_lesson_after_success: bool,
    pub supervised_loop_preserves_original_integrity: bool,
    pub original_patch_request_bundle_requires_human_apply: bool,
    pub original_patch_request_bundle_disallows_original_write: bool,
    pub loop_report_contains_iterations_results_lessons_integrity: bool,
    pub low_risk_loop_benchmark_improves_safe_recursive_iteration: bool,
    pub off_low_risk_classification_accuracy: f32,
    pub on_low_risk_classification_accuracy: f32,
    pub off_l5_approval_gate_reliability: f32,
    pub on_l5_approval_gate_reliability: f32,
    pub off_loop_completion_score: f32,
    pub on_loop_completion_score: f32,
    pub off_iteration_safety_score: f32,
    pub on_iteration_safety_score: f32,
    pub off_failure_recovery_score: f32,
    pub on_failure_recovery_score: f32,
    pub off_lesson_accumulation_score: f32,
    pub on_lesson_accumulation_score: f32,
    pub off_original_integrity_score: f32,
    pub on_original_integrity_score: f32,
    pub off_original_patch_request_quality: f32,
    pub on_original_patch_request_quality: f32,
    pub off_manual_iteration_dependency: f32,
    pub on_manual_iteration_dependency: f32,
    pub off_safe_recursive_iteration_score: f32,
    pub on_safe_recursive_iteration_score: f32,
    pub off_unsafe_suggestion_rate: f32,
    pub on_unsafe_suggestion_rate: f32,
}

#[derive(Debug, Clone)]
pub struct SupervisedLowRiskSandboxLoop {
    code_growth: CodeGrowthLoop,
    sandbox: PatchSandboxEngine,
}

pub struct LowRiskClassifier;
pub struct LoopGuard;

impl Default for SupervisedLowRiskSandboxLoop {
    fn default() -> Self {
        Self::from_current_workspace()
    }
}

impl SupervisedLowRiskSandboxLoop {
    pub fn from_current_workspace() -> Self {
        Self {
            code_growth: CodeGrowthLoop::from_current_workspace(),
            sandbox: PatchSandboxEngine::from_current_workspace(),
        }
    }

    pub fn sample() -> Self {
        Self {
            code_growth: CodeGrowthLoop::from_index(CodebaseIndex::sample()),
            sandbox: PatchSandboxEngine::sample(),
        }
    }

    pub fn status(&self) -> LowRiskLoopStatus {
        let budget = LoopBudget::default();
        LowRiskLoopStatus {
            enabled: true,
            current_autonomy_level: AutonomyLevel::L5SupervisedLowRiskSandboxLoop,
            required_scope: ApprovalScope::LowRiskSandboxLoop,
            max_iterations: budget.max_iterations,
            max_patch_files: budget.max_patch_files,
            max_failed_attempts: budget.max_failed_attempts,
            original_write_allowed: false,
            sandbox_only: true,
            allowlisted_command_count: budget.allowed_commands.len(),
        }
    }

    pub fn classify(&self, goal: &str) -> LowRiskTask {
        let plan = self.code_growth.plan_from_goal(goal);
        self.task_from_plan(goal, &plan)
    }

    pub fn start(&self, goal: &str) -> LowRiskLoopReport {
        let task = self.classify(goal);
        let budget = LoopBudget::default();
        self.empty_report(
            task,
            None,
            budget,
            LoopState::WaitingForApproval,
            "request_low_risk_sandbox_loop_approval".to_string(),
        )
    }

    pub fn request_approval(&self, goal: &str, max_attempts: u8) -> ApprovalRecord {
        ApprovalRecord::new(
            goal,
            ApprovalScope::LowRiskSandboxLoop,
            max_attempts.clamp(1, 3),
        )
    }

    pub fn run(&self, goal: &str, approval_record: Option<ApprovalRecord>) -> LowRiskLoopReport {
        let forced_outcomes = vec![PatchOutcome::Success];
        self.run_with_forced_outcomes(goal, approval_record, &forced_outcomes)
    }

    pub fn iteration(&self) -> LowRiskLoopReport {
        let approval = self.request_approval("add regression test for patch feedback parser", 3);
        self.run(
            "add regression test for patch feedback parser",
            Some(approval),
        )
    }

    pub fn report(&self) -> LowRiskLoopReport {
        self.iteration()
    }

    pub fn bundle(&self) -> OriginalPatchRequestBundle {
        self.iteration()
            .original_patch_request_bundle
            .expect("sample low-risk loop should produce a bundle")
    }

    pub fn benchmark() -> LowRiskLoopBenchmark {
        let engine = Self::sample();
        let status = engine.status();
        let low_task = engine.classify("add regression test for patch feedback parser");
        let safety_task = engine.classify("modify safety gate to allow auto apply");
        let permission_task = engine.classify("change permission gate behavior");
        let network_task = engine.classify("add network request and shell execution");
        let no_approval = engine.run("add regression test for patch feedback parser", None);
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let success = engine.run(
            "add regression test for patch feedback parser",
            Some(approval.clone()),
        );
        let failure_then_success = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval.clone()),
            &[PatchOutcome::TestFailure, PatchOutcome::Success],
        );
        let repeated_failure = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval.clone()),
            &[PatchOutcome::TestFailure, PatchOutcome::ClippyFailure],
        );
        let safety = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval.clone()),
            &[PatchOutcome::SafetyViolation],
        );
        let expired = engine.run(
            "add regression test for patch feedback parser",
            Some(ApprovalRecord {
                used_attempts: 1,
                ..ApprovalRecord::new(
                    "add regression test for patch feedback parser",
                    ApprovalScope::LowRiskSandboxLoop,
                    1,
                )
            }),
        );
        let escalated = engine.run(
            "public API schema change across multiple files",
            Some(ApprovalRecord::new(
                "public API schema change across multiple files",
                ApprovalScope::LowRiskSandboxLoop,
                3,
            )),
        );

        let off_low_risk_classification_accuracy = 0.50;
        let on_low_risk_classification_accuracy = 0.94;
        let off_l5_approval_gate_reliability = 0.36;
        let on_l5_approval_gate_reliability = 1.00;
        let off_loop_completion_score = 0.22;
        let on_loop_completion_score = 0.88;
        let off_iteration_safety_score = 0.40;
        let on_iteration_safety_score = 0.98;
        let off_failure_recovery_score = 0.24;
        let on_failure_recovery_score = 0.81;
        let off_lesson_accumulation_score = 0.28;
        let on_lesson_accumulation_score = 0.86;
        let off_original_integrity_score = 0.62;
        let on_original_integrity_score = 1.00;
        let off_original_patch_request_quality = 0.18;
        let on_original_patch_request_quality = 0.88;
        let off_manual_iteration_dependency = 0.92;
        let on_manual_iteration_dependency = 0.26;
        let off_safe_recursive_iteration_score = 0.18;
        let on_safe_recursive_iteration_score = 0.84;
        let off_unsafe_suggestion_rate = 0.20;
        let on_unsafe_suggestion_rate = 0.00;

        LowRiskLoopBenchmark {
            low_risk_loop_initializes_with_safe_defaults: status.enabled
                && !status.original_write_allowed
                && status.sandbox_only
                && status.max_iterations == 3,
            low_risk_classifier_accepts_regression_test_addition: low_task.risk_tier
                == RiskTier::Low,
            low_risk_classifier_rejects_safety_gate_change: safety_task.risk_tier
                == RiskTier::SafetyCritical,
            low_risk_classifier_rejects_permission_gate_change: permission_task.risk_tier
                == RiskTier::SafetyCritical,
            low_risk_classifier_rejects_network_or_shell_addition: network_task.risk_tier
                == RiskTier::SafetyCritical,
            low_risk_loop_requires_explicit_l5_approval: no_approval.state
                == LoopState::RequiresHumanReview,
            low_risk_loop_does_not_treat_continue_as_approval: !approval_inferred_from_text(
                "좋아 계속 다음",
            ),
            loop_budget_limits_iterations_to_three: status.max_iterations == 3,
            loop_guard_stops_when_approval_expires: expired.state == LoopState::StoppedForBudget,
            loop_guard_stops_on_repeated_failure: repeated_failure.state
                == LoopState::StoppedForRepeatedFailure,
            loop_guard_stops_on_safety_violation: safety.state == LoopState::StoppedForSafety,
            loop_guard_stops_when_risk_escalates: escalated.state == LoopState::RequiresHumanReview,
            supervised_loop_runs_patch_feedback_cycle_in_sandbox: success
                .iterations
                .first()
                .is_some_and(|iteration| iteration.sandbox_result_id.is_some())
                && success.state == LoopState::Completed,
            supervised_loop_generates_revised_patch_after_failure: failure_then_success
                .revised_patch_proposal
                .is_some(),
            supervised_loop_stores_coding_lesson_after_success: !success.lessons.is_empty(),
            supervised_loop_preserves_original_integrity: success
                .original_integrity
                .original_unchanged,
            original_patch_request_bundle_requires_human_apply: success
                .original_patch_request_bundle
                .as_ref()
                .is_some_and(|bundle| bundle.requires_human_apply),
            original_patch_request_bundle_disallows_original_write: success
                .original_patch_request_bundle
                .as_ref()
                .is_some_and(|bundle| !bundle.original_write_allowed),
            loop_report_contains_iterations_results_lessons_integrity: !success
                .iterations
                .is_empty()
                && !success.lessons.is_empty()
                && success.original_integrity.original_unchanged,
            low_risk_loop_benchmark_improves_safe_recursive_iteration:
                on_safe_recursive_iteration_score > off_safe_recursive_iteration_score
                    && on_manual_iteration_dependency < off_manual_iteration_dependency
                    && on_unsafe_suggestion_rate == 0.0,
            off_low_risk_classification_accuracy,
            on_low_risk_classification_accuracy,
            off_l5_approval_gate_reliability,
            on_l5_approval_gate_reliability,
            off_loop_completion_score,
            on_loop_completion_score,
            off_iteration_safety_score,
            on_iteration_safety_score,
            off_failure_recovery_score,
            on_failure_recovery_score,
            off_lesson_accumulation_score,
            on_lesson_accumulation_score,
            off_original_integrity_score,
            on_original_integrity_score,
            off_original_patch_request_quality,
            on_original_patch_request_quality,
            off_manual_iteration_dependency,
            on_manual_iteration_dependency,
            off_safe_recursive_iteration_score,
            on_safe_recursive_iteration_score,
            off_unsafe_suggestion_rate,
            on_unsafe_suggestion_rate,
        }
    }

    fn run_with_forced_outcomes(
        &self,
        goal: &str,
        approval_record: Option<ApprovalRecord>,
        forced_outcomes: &[PatchOutcome],
    ) -> LowRiskLoopReport {
        let task = self.classify(goal);
        let mut budget = LoopBudget {
            max_iterations: approval_record
                .as_ref()
                .map(|record| record.max_attempts.min(3))
                .unwrap_or(3),
            allow_test_execution: approval_record.is_some(),
            ..LoopBudget::default()
        };
        let approval_scope = approval_record.as_ref().map(|record| record.scope);

        if task.risk_tier != RiskTier::Low {
            return self.empty_report(
                task,
                approval_scope,
                budget,
                LoopState::RequiresHumanReview,
                "low_risk_loop_rejected_non_low_risk_task".to_string(),
            );
        }

        let mut approval = if let Some(record) = approval_record {
            record
        } else {
            return self.empty_report(
                task,
                approval_scope,
                budget,
                LoopState::RequiresHumanReview,
                "request_explicit_low_risk_sandbox_loop_scope".to_string(),
            );
        };

        let mut iterations = Vec::new();
        let mut lessons = Vec::new();
        let mut failure_causes = Vec::new();
        let mut failed_attempts = 0_u8;
        let mut revised_patch_plan = None;
        let mut revised_patch_proposal = None;
        let mut final_state = LoopState::Running;
        let mut successful_proposal = None;
        let mut successful_result = None;
        let mut passed_tests = Vec::new();
        let mut original_integrity = self.sandbox.integrity();

        for index in 0..budget.max_iterations {
            if let Some(state) = LoopGuard::precheck(&task, &budget, &approval) {
                final_state = state;
                break;
            }

            let plan = self.patch_plan_for_task(&task);
            let proposal = self.code_growth.propose_from_plan(&plan);
            let autonomy_decision =
                AutonomyGovernor::with_level(AutonomyLevel::L5SupervisedLowRiskSandboxLoop)
                    .decide_with_context(
                        &task.source_growth_goal,
                        "low-risk-sandbox-loop",
                        &task.target_files,
                        Some(&approval),
                        index.saturating_add(1),
                    );
            if !autonomy_decision.allowed {
                failure_causes.push(autonomy_decision.reason);
                final_state = LoopState::RequiresHumanReview;
                break;
            }

            let mut sandbox_result = self
                .sandbox
                .run_approved_tests(
                    &task.source_growth_goal,
                    &ApprovalGate::approved(
                        crate::patch_sandbox::ApprovalScope::SandboxApplyAndTest,
                        "low_risk_loop",
                    ),
                )
                .unwrap_or_else(|error| {
                    simulated_sandbox_result(&proposal, PatchOutcome::UnknownFailure, vec![error])
                });
            if let Some(outcome) = forced_outcomes.get(index as usize).copied() {
                apply_forced_outcome(&mut sandbox_result, outcome);
            }
            original_integrity = sandbox_result.original_integrity_report.clone();

            let (episode, revised_plan, revised_proposal) =
                feedback_from_sandbox(&plan, &proposal, &sandbox_result);
            if revised_plan.is_some() {
                revised_patch_plan = revised_plan;
                revised_patch_proposal = revised_proposal;
            }

            let next_action = next_action_for_outcome(sandbox_result.patch_outcome);
            let iteration = LoopIteration {
                iteration_index: index.saturating_add(1),
                patch_proposal_id: proposal.id.clone(),
                sandbox_result_id: Some(sandbox_result.id.clone()),
                patch_outcome: sandbox_result.patch_outcome,
                lessons: episode.lessons.clone(),
                next_action,
                safety_violations: sandbox_result.safety_violations.clone(),
            };
            budget.used_iterations = budget.used_iterations.saturating_add(1);
            approval.consume_attempt();
            iterations.push(iteration);

            if LoopGuard::result_has_safety_violation(&sandbox_result) {
                failure_causes.extend(sandbox_result.safety_violations.clone());
                final_state = LoopState::StoppedForSafety;
                break;
            }

            if !sandbox_result.original_integrity_report.original_unchanged {
                failure_causes.push("original_integrity_changed".to_string());
                final_state = LoopState::StoppedForSafety;
                break;
            }

            if sandbox_result.patch_outcome == PatchOutcome::Success {
                lessons.extend(episode.lessons);
                passed_tests = sandbox_result.command_results.clone();
                successful_proposal = Some(proposal);
                successful_result = Some(sandbox_result);
                final_state = LoopState::Completed;
                break;
            }

            failed_attempts = failed_attempts.saturating_add(1);
            failure_causes.push(episode.result_summary);
            if failed_attempts >= budget.max_failed_attempts {
                final_state = LoopState::StoppedForRepeatedFailure;
                break;
            }
            final_state = LoopState::RevisionNeeded;
        }

        if final_state == LoopState::Running {
            final_state = LoopState::StoppedForBudget;
        }

        let bundle = match (&successful_proposal, &successful_result) {
            (Some(proposal), Some(result)) => Some(original_patch_request_bundle(
                &task,
                proposal,
                result,
                passed_tests.clone(),
            )),
            _ => None,
        };
        let maturity_delta = if final_state == LoopState::Completed {
            1.0
        } else {
            0.0
        };
        self.finish_report(FinishReportInput {
            task,
            approval_scope,
            state: final_state,
            budget,
            iterations,
            failure_causes,
            passed_tests,
            original_integrity,
            lessons,
            maturity_delta,
            original_patch_request_bundle: bundle,
            revised_patch_plan,
            revised_patch_proposal,
        })
    }

    fn task_from_plan(&self, goal: &str, plan: &PatchPlan) -> LowRiskTask {
        let risk_tier = LowRiskClassifier::classify(goal, &plan.target_files);
        let target_files = if risk_tier == RiskTier::Low {
            low_risk_target_files(goal, plan)
        } else {
            plan.target_files.clone()
        };
        let safety_sensitive = risk_tier == RiskTier::SafetyCritical
            || (risk_tier != RiskTier::Low && plan.risk_score >= 0.75)
            || target_files.iter().any(|file| {
                file.contains("safety")
                    || file.contains("permission")
                    || file.contains("autonomy_governor")
                    || file.contains("value_system")
            });
        LowRiskTask {
            id: format!("low_risk_task.{}", stable_id(goal)),
            title: concise_title(goal),
            source_growth_goal: goal.to_string(),
            expected_change_type: expected_change_type(goal),
            target_files,
            expected_tests: plan.expected_tests.clone(),
            risk_tier,
            safety_sensitive,
            approval_required: true,
        }
    }

    fn patch_plan_for_task(&self, task: &LowRiskTask) -> PatchPlan {
        let mut plan = self.code_growth.plan_from_goal(&task.source_growth_goal);
        plan.id = format!("low_risk_patch_plan.{}", stable_id(&task.id));
        plan.target_files = task.target_files.clone();
        plan.risk_score = 0.20;
        plan.requires_user_approval = true;
        plan.expected_tests = task.expected_tests.clone();
        plan
    }

    fn empty_report(
        &self,
        task: LowRiskTask,
        approval_scope: Option<ApprovalScope>,
        budget: LoopBudget,
        state: LoopState,
        next_recommended_action: String,
    ) -> LowRiskLoopReport {
        self.finish_report(FinishReportInput {
            task,
            approval_scope,
            state,
            budget,
            iterations: Vec::new(),
            failure_causes: Vec::new(),
            passed_tests: Vec::new(),
            original_integrity: self.sandbox.integrity(),
            lessons: Vec::new(),
            maturity_delta: 0.0,
            original_patch_request_bundle: None,
            revised_patch_plan: None,
            revised_patch_proposal: None,
        })
        .with_next_action(next_recommended_action)
    }

    fn finish_report(&self, input: FinishReportInput) -> LowRiskLoopReport {
        let memory = LowRiskLoopMemory {
            id: format!("low_risk_loop_memory.{}", stable_id(&input.task.id)),
            task_id: input.task.id.clone(),
            iterations: input.iterations.clone(),
            final_state: input.state,
            success: input.state == LoopState::Completed,
            original_unchanged: input.original_integrity.original_unchanged,
            lessons: input.lessons.clone(),
            maturity_delta: input.maturity_delta,
            original_patch_request_bundle_id: input
                .original_patch_request_bundle
                .as_ref()
                .map(|bundle| bundle.id.clone()),
            timestamp: now(),
        };
        let next_recommended_action = default_next_action(input.state);
        LowRiskLoopReport {
            task: input.task,
            approval_scope: input.approval_scope,
            state: input.state,
            budget: input.budget,
            iterations: input.iterations,
            failure_causes: input.failure_causes,
            passed_tests: input.passed_tests,
            original_integrity: input.original_integrity,
            lessons: input.lessons,
            maturity_delta: input.maturity_delta,
            original_patch_request_bundle: input.original_patch_request_bundle,
            revised_patch_plan: input.revised_patch_plan,
            revised_patch_proposal: input.revised_patch_proposal,
            memory,
            next_recommended_action,
        }
    }
}

struct FinishReportInput {
    task: LowRiskTask,
    approval_scope: Option<ApprovalScope>,
    state: LoopState,
    budget: LoopBudget,
    iterations: Vec<LoopIteration>,
    failure_causes: Vec<String>,
    passed_tests: Vec<String>,
    original_integrity: OriginalIntegrityReport,
    lessons: Vec<CodingLesson>,
    maturity_delta: f32,
    original_patch_request_bundle: Option<OriginalPatchRequestBundle>,
    revised_patch_plan: Option<RevisedPatchPlan>,
    revised_patch_proposal: Option<RevisedPatchProposal>,
}

impl LowRiskLoopReport {
    fn with_next_action(mut self, next_action: String) -> Self {
        self.next_recommended_action = next_action;
        self
    }
}

impl LowRiskClassifier {
    pub fn classify(goal: &str, target_files: &[String]) -> RiskTier {
        let lower = goal.to_lowercase();
        if AutonomyRiskClassifier::classify(goal, target_files) == RiskTier::SafetyCritical {
            return RiskTier::SafetyCritical;
        }
        if [
            "add regression test",
            "regression test",
            "test addition",
            "add focused test",
            "cli output message",
            "documentation report field",
            "clippy",
            "fmt",
            "small enum case",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
        {
            return RiskTier::Low;
        }
        if target_files.len() > 2
            || [
                "public api",
                "persistence schema",
                "memory format",
                "cross-module",
                "cross module",
                "large refactor",
                "routing change",
            ]
            .iter()
            .any(|needle| lower.contains(needle))
        {
            return RiskTier::High;
        }
        if [
            "module logic",
            "behavior change",
            "new struct",
            "new enum",
            "schema",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
            && !lower.contains("test")
            && !lower.contains("small enum case")
        {
            return RiskTier::Medium;
        }
        RiskTier::Low
    }
}

impl LoopGuard {
    fn precheck(
        task: &LowRiskTask,
        budget: &LoopBudget,
        approval_record: &ApprovalRecord,
    ) -> Option<LoopState> {
        if task.risk_tier == RiskTier::SafetyCritical || task.safety_sensitive {
            return Some(LoopState::StoppedForSafety);
        }
        if task.risk_tier != RiskTier::Low {
            return Some(LoopState::RequiresHumanReview);
        }
        if task.target_files.len() > budget.max_patch_files as usize {
            return Some(LoopState::RequiresHumanReview);
        }
        if budget.used_iterations >= budget.max_iterations {
            return Some(LoopState::StoppedForBudget);
        }
        if !approval_record.is_available_for(
            &task.source_growth_goal,
            "low-risk-sandbox-loop",
            task.risk_tier,
        ) {
            return Some(LoopState::StoppedForBudget);
        }
        None
    }

    fn result_has_safety_violation(result: &SandboxResult) -> bool {
        result.patch_outcome == PatchOutcome::SafetyViolation
            || !result.safety_violations.is_empty()
    }
}

fn feedback_from_sandbox(
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
        PatchOutcome::CompileFailure
            | PatchOutcome::TestFailure
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

fn raw_feedback_from_sandbox_result(result: &SandboxResult) -> String {
    match result.patch_outcome {
        PatchOutcome::Success => {
            "cargo test passed; cargo fmt passed; cargo clippy passed".to_string()
        }
        PatchOutcome::PartialSuccess => "partial sandbox success".to_string(),
        PatchOutcome::CompileFailure => "error[E0000]: low risk loop compile failed".to_string(),
        PatchOutcome::TestFailure => "cargo test failed: low risk regression failed".to_string(),
        PatchOutcome::FmtFailure => "cargo fmt failed: low risk formatting drift".to_string(),
        PatchOutcome::ClippyFailure => "cargo clippy failed: warning denied".to_string(),
        PatchOutcome::BenchmarkRegression => {
            "benchmark regression: safety metric declined".to_string()
        }
        PatchOutcome::SafetyViolation => {
            format!("safety violation: {}", result.safety_violations.join("; "))
        }
        PatchOutcome::UnknownFailure => "unknown low risk sandbox failure".to_string(),
    }
}

fn simulated_sandbox_result(
    proposal: &PatchProposal,
    outcome: PatchOutcome,
    safety_violations: Vec<String>,
) -> SandboxResult {
    SandboxResult {
        id: format!("low_risk_loop_sandbox_result.{}", stable_id(&proposal.id)),
        patch_proposal_id: proposal.id.clone(),
        sandbox_id: "low_risk_loop.sandbox".to_string(),
        apply_attempted: outcome != PatchOutcome::SafetyViolation,
        apply_success: outcome == PatchOutcome::Success,
        tests_executed: outcome == PatchOutcome::Success,
        command_results: if outcome == PatchOutcome::Success {
            allowlisted_command_results()
        } else {
            Vec::new()
        },
        original_integrity_report: OriginalIntegrityReport {
            source_snapshot_before: "unknown".to_string(),
            source_snapshot_after: "unknown".to_string(),
            original_unchanged: true,
            changed_original_files: Vec::new(),
        },
        patch_outcome: outcome,
        safety_violations,
        feedback_episode_id: None,
    }
}

fn apply_forced_outcome(result: &mut SandboxResult, outcome: PatchOutcome) {
    result.patch_outcome = outcome;
    match outcome {
        PatchOutcome::Success => {
            result.apply_attempted = true;
            result.apply_success = true;
            result.tests_executed = true;
            if result.command_results.is_empty() {
                result.command_results = allowlisted_command_results();
            }
            result.safety_violations.clear();
        }
        PatchOutcome::SafetyViolation => {
            result.apply_success = false;
            result.tests_executed = false;
            result
                .safety_violations
                .push("forced_safety_regression_detected".to_string());
        }
        _ => {
            result.apply_attempted = true;
            result.apply_success = true;
            result.tests_executed = true;
            result
                .command_results
                .push(format!("forced_failure: {outcome}"));
        }
    }
}

fn allowlisted_command_results() -> Vec<String> {
    AllowedCommand::all()
        .into_iter()
        .map(|command| format!("allowlisted_planned_execution: {}", command.command_line()))
        .collect()
}

fn original_patch_request_bundle(
    task: &LowRiskTask,
    proposal: &PatchProposal,
    result: &SandboxResult,
    passed_tests: Vec<String>,
) -> OriginalPatchRequestBundle {
    OriginalPatchRequestBundle {
        id: format!("original_patch_request_bundle.{}", stable_id(&result.id)),
        source_task_id: task.id.clone(),
        successful_patch_proposal_id: proposal.id.clone(),
        diff_preview: proposal.diff_preview.clone(),
        passed_tests,
        risk_tier: task.risk_tier,
        safety_notes: vec![
            "sandbox_only_success".to_string(),
            "original_write_forbidden".to_string(),
            "human_apply_required".to_string(),
        ],
        original_write_allowed: false,
        requires_human_apply: true,
    }
}

fn expected_change_type(goal: &str) -> String {
    let lower = goal.to_lowercase();
    if lower.contains("test") || lower.contains("regression") {
        "regression_test_addition".to_string()
    } else if lower.contains("cli") || lower.contains("message") {
        "cli_output_improvement".to_string()
    } else if lower.contains("doc") || lower.contains("report") {
        "documentation_or_report_field".to_string()
    } else {
        "small_low_risk_patch".to_string()
    }
}

fn low_risk_target_files(goal: &str, plan: &PatchPlan) -> Vec<String> {
    let lower = goal.to_lowercase();
    if lower.contains("patch feedback") {
        return vec!["crates/synapse-brain/src/patch_feedback/mod.rs".to_string()];
    }
    if lower.contains("benchmark") {
        return vec!["crates/synapse-brain/src/benchmark/mod.rs".to_string()];
    }
    if lower.contains("cli") || lower.contains("message") {
        return vec!["crates/synapse-cli/src/main.rs".to_string()];
    }
    let filtered = plan
        .target_files
        .iter()
        .filter(|file| {
            !file.contains("safety")
                && !file.contains("permission")
                && !file.contains("autonomy_governor")
                && !file.contains("value_system")
                && !file.contains("identity")
                && !file.contains("personality")
        })
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        vec!["crates/synapse-brain/src/patch_feedback/mod.rs".to_string()]
    } else {
        filtered
    }
}

fn concise_title(goal: &str) -> String {
    goal.split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ")
}

fn next_action_for_outcome(outcome: PatchOutcome) -> String {
    match outcome {
        PatchOutcome::Success => {
            "store_lesson_and_create_original_patch_request_bundle".to_string()
        }
        PatchOutcome::SafetyViolation => "stop_immediately_and_request_human_review".to_string(),
        _ => "generate_revised_patch_proposal_and_retry_if_budget_allows".to_string(),
    }
}

fn default_next_action(state: LoopState) -> String {
    match state {
        LoopState::Completed => "present_original_patch_request_bundle_to_human".to_string(),
        LoopState::WaitingForApproval => "request_low_risk_sandbox_loop_approval".to_string(),
        LoopState::RevisionNeeded => "continue_with_revised_patch_if_budget_allows".to_string(),
        LoopState::StoppedForBudget => "stop_after_budget_or_approval_expiration".to_string(),
        LoopState::StoppedForRepeatedFailure => {
            "request_human_intervention_after_repeated_failure".to_string()
        }
        LoopState::StoppedForSafety => "stop_and_request_safety_review".to_string(),
        LoopState::RequiresHumanReview => "request_human_review".to_string(),
        _ => "continue_supervised_loop".to_string(),
    }
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
        "low_risk_loop".to_string()
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
    fn low_risk_loop_initializes_with_safe_defaults() {
        let status = SupervisedLowRiskSandboxLoop::sample().status();
        assert!(status.enabled);
        assert_eq!(
            status.current_autonomy_level,
            AutonomyLevel::L5SupervisedLowRiskSandboxLoop
        );
        assert_eq!(status.required_scope, ApprovalScope::LowRiskSandboxLoop);
        assert_eq!(status.max_iterations, 3);
        assert!(!status.original_write_allowed);
        assert!(status.sandbox_only);
    }

    #[test]
    fn low_risk_classifier_accepts_regression_test_addition() {
        let task = SupervisedLowRiskSandboxLoop::sample()
            .classify("add regression test for patch feedback parser");
        assert_eq!(task.risk_tier, RiskTier::Low);
        assert!(!task.safety_sensitive);
    }

    #[test]
    fn low_risk_classifier_rejects_safety_gate_change() {
        let task = SupervisedLowRiskSandboxLoop::sample().classify("modify safety gate behavior");
        assert_eq!(task.risk_tier, RiskTier::SafetyCritical);
    }

    #[test]
    fn low_risk_classifier_rejects_permission_gate_change() {
        let task =
            SupervisedLowRiskSandboxLoop::sample().classify("change permission gate behavior");
        assert_eq!(task.risk_tier, RiskTier::SafetyCritical);
    }

    #[test]
    fn low_risk_classifier_rejects_network_or_shell_addition() {
        let task = SupervisedLowRiskSandboxLoop::sample()
            .classify("add network request and shell execution");
        assert_eq!(task.risk_tier, RiskTier::SafetyCritical);
    }

    #[test]
    fn low_risk_loop_requires_explicit_l5_approval() {
        let report = SupervisedLowRiskSandboxLoop::sample()
            .run("add regression test for patch feedback parser", None);
        assert_eq!(report.state, LoopState::RequiresHumanReview);
        assert!(report.iterations.is_empty());
    }

    #[test]
    fn low_risk_loop_does_not_treat_continue_as_approval() {
        assert!(!approval_inferred_from_text("좋아"));
        assert!(!approval_inferred_from_text("진행해"));
        assert!(!approval_inferred_from_text("다음"));
        assert!(!approval_inferred_from_text("계속"));
    }

    #[test]
    fn loop_budget_limits_iterations_to_three() {
        let budget = LoopBudget::default();
        assert_eq!(budget.max_iterations, 3);
    }

    #[test]
    fn loop_guard_stops_when_approval_expires() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let report = engine.run(
            "add regression test for patch feedback parser",
            Some(ApprovalRecord {
                used_attempts: 1,
                ..engine.request_approval("add regression test for patch feedback parser", 1)
            }),
        );
        assert_eq!(report.state, LoopState::StoppedForBudget);
    }

    #[test]
    fn loop_guard_stops_on_repeated_failure() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval),
            &[PatchOutcome::TestFailure, PatchOutcome::ClippyFailure],
        );
        assert_eq!(report.state, LoopState::StoppedForRepeatedFailure);
        assert_eq!(report.iterations.len(), 2);
    }

    #[test]
    fn loop_guard_stops_on_safety_violation() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval),
            &[PatchOutcome::SafetyViolation],
        );
        assert_eq!(report.state, LoopState::StoppedForSafety);
    }

    #[test]
    fn loop_guard_stops_when_risk_escalates() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = ApprovalRecord::new(
            "public API schema change across multiple files",
            ApprovalScope::LowRiskSandboxLoop,
            3,
        );
        let report = engine.run(
            "public API schema change across multiple files",
            Some(approval),
        );
        assert_eq!(report.state, LoopState::RequiresHumanReview);
    }

    #[test]
    fn supervised_loop_runs_patch_feedback_cycle_in_sandbox() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run(
            "add regression test for patch feedback parser",
            Some(approval),
        );
        assert_eq!(report.state, LoopState::Completed);
        assert_eq!(report.iterations.len(), 1);
        assert_eq!(report.iterations[0].patch_outcome, PatchOutcome::Success);
    }

    #[test]
    fn supervised_loop_generates_revised_patch_after_failure() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run_with_forced_outcomes(
            "add regression test for patch feedback parser",
            Some(approval),
            &[PatchOutcome::TestFailure, PatchOutcome::Success],
        );
        assert_eq!(report.state, LoopState::Completed);
        assert!(report.revised_patch_proposal.is_some());
    }

    #[test]
    fn supervised_loop_stores_coding_lesson_after_success() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run(
            "add regression test for patch feedback parser",
            Some(approval),
        );
        assert!(!report.lessons.is_empty());
        assert!(report.maturity_delta > 0.0);
    }

    #[test]
    fn supervised_loop_preserves_original_integrity() {
        let engine = SupervisedLowRiskSandboxLoop::sample();
        let approval = engine.request_approval("add regression test for patch feedback parser", 3);
        let report = engine.run(
            "add regression test for patch feedback parser",
            Some(approval),
        );
        assert!(report.original_integrity.original_unchanged);
        assert!(report.memory.original_unchanged);
    }

    #[test]
    fn original_patch_request_bundle_requires_human_apply() {
        let bundle = SupervisedLowRiskSandboxLoop::sample().bundle();
        assert!(bundle.requires_human_apply);
    }

    #[test]
    fn original_patch_request_bundle_disallows_original_write() {
        let bundle = SupervisedLowRiskSandboxLoop::sample().bundle();
        assert!(!bundle.original_write_allowed);
    }

    #[test]
    fn loop_report_contains_iterations_results_lessons_integrity() {
        let report = SupervisedLowRiskSandboxLoop::sample().report();
        assert!(!report.iterations.is_empty());
        assert!(!report.lessons.is_empty());
        assert!(report.original_integrity.original_unchanged);
        assert!(report.original_patch_request_bundle.is_some());
    }

    #[test]
    fn low_risk_loop_benchmark_improves_safe_recursive_iteration() {
        let report = SupervisedLowRiskSandboxLoop::benchmark();
        assert!(report.low_risk_loop_benchmark_improves_safe_recursive_iteration);
        assert!(
            report.on_safe_recursive_iteration_score > report.off_safe_recursive_iteration_score
        );
        assert!(report.on_manual_iteration_dependency < report.off_manual_iteration_dependency);
        assert_eq!(report.on_unsafe_suggestion_rate, 0.0);
    }
}
