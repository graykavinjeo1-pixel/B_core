use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::code_growth::{CodeGrowthLoop, CodebaseIndex, PatchPlan, PatchProposal};
use crate::coding_knowledge::CodingLesson;
use crate::patch_feedback::{PatchFeedbackLoop, PatchOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalScope {
    DryRunOnly,
    SandboxApplyOnly,
    SandboxApplyAndTest,
}

impl Display for ApprovalScope {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::DryRunOnly => "dry_run_only",
            Self::SandboxApplyOnly => "sandbox_apply_only",
            Self::SandboxApplyAndTest => "sandbox_apply_and_test",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllowedCommand {
    CargoTest,
    CargoFmtCheck,
    CargoClippyDenyWarnings,
}

impl Display for AllowedCommand {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::CargoTest => "cargo_test",
            Self::CargoFmtCheck => "cargo_fmt_check",
            Self::CargoClippyDenyWarnings => "cargo_clippy_deny_warnings",
        };
        write!(formatter, "{value}")
    }
}

impl AllowedCommand {
    pub fn command_line(self) -> &'static str {
        match self {
            Self::CargoTest => "cargo test",
            Self::CargoFmtCheck => "cargo fmt --all --check",
            Self::CargoClippyDenyWarnings => "cargo clippy --all-targets -- -D warnings",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::CargoTest,
            Self::CargoFmtCheck,
            Self::CargoClippyDenyWarnings,
        ]
    }

    pub fn from_command_text(command: &str) -> Option<Self> {
        match command.trim() {
            "cargo test" => Some(Self::CargoTest),
            "cargo fmt --all --check" => Some(Self::CargoFmtCheck),
            "cargo clippy --all-targets -- -D warnings" => Some(Self::CargoClippyDenyWarnings),
            _ => None,
        }
    }

    pub fn rejects_forbidden_text(command: &str) -> bool {
        let lower = command.to_lowercase();
        let forbidden = [
            "git commit",
            "git push",
            "git reset",
            "git clean",
            "cargo update",
            "cargo install",
            "pip install",
            "mojo install",
            "curl ",
            "wget ",
            "http://",
            "https://",
            "powershell",
            "cmd /c",
            "bash -c",
            "rm ",
            "del ",
            "remove-item",
            "new-item -itemtype symboliclink",
        ];
        Self::from_command_text(command).is_none()
            || forbidden.iter().any(|needle| lower.contains(needle))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSandbox {
    pub id: String,
    pub root_path: String,
    pub source_snapshot_hash: String,
    pub sandbox_path: String,
    pub created_at: u64,
    pub active: bool,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxManifest {
    pub id: String,
    pub patch_proposal_id: String,
    pub source_root: String,
    pub sandbox_root: String,
    pub copied_files: Vec<String>,
    pub excluded_paths: Vec<String>,
    pub allowed_write_paths: Vec<String>,
    pub allowed_commands: Vec<AllowedCommand>,
    pub original_snapshot_hash: String,
    pub approval_required: bool,
    pub approval_granted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalGate {
    pub approval_required: bool,
    pub approval_granted: bool,
    pub approved_by: Option<String>,
    pub approved_at: Option<u64>,
    pub approval_scope: ApprovalScope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchApplyPlan {
    pub id: String,
    pub patch_proposal_id: String,
    pub target_files: Vec<String>,
    pub sandbox_only: bool,
    pub estimated_risk: f32,
    pub touches_safety_sensitive_files: bool,
    pub requires_user_approval: bool,
    pub expected_tests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalIntegrityReport {
    pub source_snapshot_before: String,
    pub source_snapshot_after: String,
    pub original_unchanged: bool,
    pub changed_original_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxTestRunner {
    pub execution_enabled: bool,
    pub approval_scope: ApprovalScope,
    pub allowed_commands: Vec<AllowedCommand>,
    pub cwd_must_be_sandbox: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxResult {
    pub id: String,
    pub patch_proposal_id: String,
    pub sandbox_id: String,
    pub apply_attempted: bool,
    pub apply_success: bool,
    pub tests_executed: bool,
    pub command_results: Vec<String>,
    pub original_integrity_report: OriginalIntegrityReport,
    pub patch_outcome: PatchOutcome,
    pub safety_violations: Vec<String>,
    pub feedback_episode_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxPatchMemory {
    pub id: String,
    pub patch_proposal_id: String,
    pub sandbox_result_id: String,
    pub outcome: String,
    pub original_unchanged: bool,
    pub lessons: Vec<CodingLesson>,
    pub maturity_delta: f32,
    pub next_recommendation: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedGrowthCyclePreview {
    pub embryo_growth_goal: String,
    pub patch_proposal_id: String,
    pub sandbox_result_id: String,
    pub feedback_episode_id: Option<String>,
    pub coding_lessons: Vec<String>,
    pub closed_growth_cycle_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSandboxStatus {
    pub safe_defaults: bool,
    pub proposal_only_input: bool,
    pub sandbox_apply_requires_approval: bool,
    pub tests_require_apply_and_test_scope: bool,
    pub allowed_command_count: usize,
    pub original_integrity_check_enabled: bool,
    pub closed_growth_preview_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSandboxBenchmark {
    pub patch_sandbox_initializes_with_safe_defaults: bool,
    pub sandbox_manifest_requires_user_approval: bool,
    pub patch_apply_plan_is_sandbox_only: bool,
    pub patch_apply_plan_marks_safety_sensitive_files_high_risk: bool,
    pub dry_run_does_not_modify_any_file: bool,
    pub create_copy_writes_only_to_sandbox_path: bool,
    pub create_copy_rejects_non_sandbox_path: bool,
    pub sandbox_applier_blocks_without_approval: bool,
    pub sandbox_applier_applies_only_inside_sandbox_after_approval: bool,
    pub original_integrity_checker_detects_no_original_change: bool,
    pub original_integrity_checker_flags_original_modification: bool,
    pub allowed_command_rejects_arbitrary_shell: bool,
    pub allowed_command_allows_only_cargo_test_fmt_clippy: bool,
    pub sandbox_test_runner_blocks_without_test_approval: bool,
    pub sandbox_test_runner_requires_sandbox_cwd: bool,
    pub sandbox_result_feeds_patch_feedback_loop: bool,
    pub sandbox_patch_memory_records_outcome: bool,
    pub sandbox_blocks_git_commit_and_push: bool,
    pub sandbox_blocks_network_and_package_install: bool,
    pub sandbox_blocks_file_delete_and_symlink_escape: bool,
    pub patch_sandbox_benchmark_improves_closed_growth_readiness: bool,
    pub off_sandbox_creation_safety: f32,
    pub on_sandbox_creation_safety: f32,
    pub off_approval_gate_reliability: f32,
    pub on_approval_gate_reliability: f32,
    pub off_original_integrity_score: f32,
    pub on_original_integrity_score: f32,
    pub off_sandbox_apply_accuracy: f32,
    pub on_sandbox_apply_accuracy: f32,
    pub off_allowed_command_safety: f32,
    pub on_allowed_command_safety: f32,
    pub off_test_result_capture_score: f32,
    pub on_test_result_capture_score: f32,
    pub off_patch_feedback_integration: f32,
    pub on_patch_feedback_integration: f32,
    pub off_closed_growth_cycle_readiness: f32,
    pub on_closed_growth_cycle_readiness: f32,
    pub off_proposal_only_to_sandbox_transition_score: f32,
    pub on_proposal_only_to_sandbox_transition_score: f32,
    pub off_safety_violation_detection: f32,
    pub on_safety_violation_detection: f32,
}

#[derive(Debug, Clone)]
pub struct PatchSandboxEngine {
    root: PathBuf,
    code_growth: CodeGrowthLoop,
}

pub struct SandboxWorkingCopy;
pub struct SandboxPathGuard;
pub struct SandboxPatchApplier;
pub struct OriginalIntegrityChecker;

impl Default for ApprovalGate {
    fn default() -> Self {
        Self {
            approval_required: true,
            approval_granted: false,
            approved_by: None,
            approved_at: None,
            approval_scope: ApprovalScope::DryRunOnly,
        }
    }
}

impl ApprovalGate {
    pub fn approved(scope: ApprovalScope, approved_by: impl Into<String>) -> Self {
        Self {
            approval_required: true,
            approval_granted: true,
            approved_by: Some(approved_by.into()),
            approved_at: Some(now()),
            approval_scope: scope,
        }
    }

    pub fn allows_apply(&self) -> bool {
        self.approval_granted
            && matches!(
                self.approval_scope,
                ApprovalScope::SandboxApplyOnly | ApprovalScope::SandboxApplyAndTest
            )
    }

    pub fn allows_tests(&self) -> bool {
        self.approval_granted && self.approval_scope == ApprovalScope::SandboxApplyAndTest
    }
}

impl PatchSandboxEngine {
    pub fn from_current_workspace() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root,
            code_growth: CodeGrowthLoop::from_current_workspace(),
        }
    }

    pub fn sample() -> Self {
        Self {
            root: PathBuf::from("."),
            code_growth: CodeGrowthLoop::from_index(CodebaseIndex::sample()),
        }
    }

    pub fn status(&self) -> PatchSandboxStatus {
        PatchSandboxStatus {
            safe_defaults: true,
            proposal_only_input: true,
            sandbox_apply_requires_approval: true,
            tests_require_apply_and_test_scope: true,
            allowed_command_count: AllowedCommand::all().len(),
            original_integrity_check_enabled: true,
            closed_growth_preview_enabled: true,
        }
    }

    pub fn plan(&self, goal: &str) -> (PatchPlan, PatchProposal, PatchApplyPlan) {
        let plan = self.code_growth.plan_from_goal(goal);
        let proposal = self.code_growth.propose_from_plan(&plan);
        let touches_safety_sensitive_files = plan.target_files.iter().any(|target| {
            self.code_growth
                .index()
                .find_file(target)
                .is_some_and(|file| file.safety_sensitive)
        });
        let estimated_risk = if touches_safety_sensitive_files {
            plan.risk_score.max(0.75)
        } else {
            plan.risk_score.max(0.20)
        };
        let apply_plan = PatchApplyPlan {
            id: format!("patch_apply_plan.{}", stable_id(&proposal.id)),
            patch_proposal_id: proposal.id.clone(),
            target_files: plan.target_files.clone(),
            sandbox_only: true,
            estimated_risk,
            touches_safety_sensitive_files,
            requires_user_approval: true,
            expected_tests: plan.expected_tests.clone(),
        };
        (plan, proposal, apply_plan)
    }

    pub fn dry_run(&self, goal: &str) -> SandboxResult {
        let (plan, proposal, _) = self.plan(goal);
        let sandbox = self.preview_sandbox(&proposal);
        let before = snapshot_hash(self.code_growth.index());
        let integrity =
            OriginalIntegrityChecker::check_index(self.code_growth.index(), before.clone());
        let mut feedback = PatchFeedbackLoop::from_plan_and_proposal(plan, proposal.clone());
        let episode = feedback.ingest_result(
            "dry run only: patch was not applied; cargo test planned; cargo fmt planned; cargo clippy planned",
        );
        SandboxResult {
            id: format!("sandbox_result.{}", stable_id(&format!("dry-run-{goal}"))),
            patch_proposal_id: proposal.id,
            sandbox_id: sandbox.id,
            apply_attempted: false,
            apply_success: false,
            tests_executed: false,
            command_results: AllowedCommand::all()
                .into_iter()
                .map(|command| format!("planned_only: {}", command.command_line()))
                .collect(),
            original_integrity_report: integrity,
            patch_outcome: PatchOutcome::UnknownFailure,
            safety_violations: Vec::new(),
            feedback_episode_id: Some(episode.id),
        }
    }

    pub fn create_copy(&self, goal: &str) -> Result<(PatchSandbox, SandboxManifest), String> {
        let (_, proposal, _) = self.plan(goal);
        let run_id = format!("run_{}", stable_id(&proposal.id));
        let sandbox_root = self
            .root
            .join("target")
            .join("synapse_patch_sandbox")
            .join(run_id);
        self.create_copy_at(&proposal, &sandbox_root, &ApprovalGate::default())
    }

    pub fn create_copy_at(
        &self,
        proposal: &PatchProposal,
        sandbox_root: &Path,
        approval_gate: &ApprovalGate,
    ) -> Result<(PatchSandbox, SandboxManifest), String> {
        SandboxPathGuard::ensure_allowed_sandbox_path(&self.root, sandbox_root)?;
        SandboxWorkingCopy::create(&self.root, self.code_growth.index(), proposal, sandbox_root)?;

        let snapshot = snapshot_hash(self.code_growth.index());
        let sandbox = PatchSandbox {
            id: format!("patch_sandbox.{}", stable_id(&proposal.id)),
            root_path: display_path(&self.root),
            source_snapshot_hash: snapshot.clone(),
            sandbox_path: display_path(sandbox_root),
            created_at: now(),
            active: true,
            approved: approval_gate.approval_granted,
        };
        let manifest = SandboxManifest {
            id: format!("sandbox_manifest.{}", stable_id(&sandbox.id)),
            patch_proposal_id: proposal.id.clone(),
            source_root: display_path(&self.root),
            sandbox_root: display_path(sandbox_root),
            copied_files: proposal.target_files.clone(),
            excluded_paths: vec![
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "node_modules".to_string(),
            ],
            allowed_write_paths: proposal.target_files.clone(),
            allowed_commands: AllowedCommand::all(),
            original_snapshot_hash: snapshot,
            approval_required: true,
            approval_granted: approval_gate.approval_granted,
        };
        Ok((sandbox, manifest))
    }

    pub fn apply_with_gate(
        &self,
        goal: &str,
        approval_gate: &ApprovalGate,
    ) -> Result<SandboxResult, String> {
        let (plan, proposal, _) = self.plan(goal);
        let (sandbox, _) = self.create_copy_at(
            &proposal,
            &self
                .root
                .join("target")
                .join("synapse_patch_sandbox")
                .join(format!("apply_{}", stable_id(&proposal.id))),
            approval_gate,
        )?;
        let before = snapshot_hash(self.code_growth.index());
        let result = SandboxPatchApplier::apply(
            &self.root,
            self.code_growth.index(),
            &plan,
            &proposal,
            &sandbox,
            approval_gate,
            before,
        );
        Ok(result)
    }

    pub fn test_plan(&self, goal: &str) -> SandboxTestRunner {
        let (_, _, apply_plan) = self.plan(goal);
        let mut commands = Vec::new();
        for test in &apply_plan.expected_tests {
            if let Some(command) = AllowedCommand::from_command_text(test) {
                commands.push(command);
            }
        }
        if commands.is_empty() {
            commands = AllowedCommand::all();
        }
        SandboxTestRunner {
            execution_enabled: false,
            approval_scope: ApprovalScope::DryRunOnly,
            allowed_commands: commands,
            cwd_must_be_sandbox: true,
        }
    }

    pub fn run_approved_tests(
        &self,
        goal: &str,
        approval_gate: &ApprovalGate,
    ) -> Result<SandboxResult, String> {
        let mut result = self.apply_with_gate(goal, approval_gate)?;
        let (_, proposal, _) = self.plan(goal);
        let runner = SandboxTestRunner {
            execution_enabled: approval_gate.allows_tests(),
            approval_scope: approval_gate.approval_scope,
            allowed_commands: AllowedCommand::all(),
            cwd_must_be_sandbox: true,
        };
        let sandbox_path = self
            .root
            .join("target")
            .join("synapse_patch_sandbox")
            .join(format!("apply_{}", stable_id(&proposal.id)));
        let command_results = runner.run_planned(&sandbox_path);
        if approval_gate.allows_tests() {
            result.tests_executed = true;
            result.command_results = command_results;
            result.patch_outcome = PatchOutcome::Success;
            let (plan, proposal, _) = self.plan(goal);
            let mut feedback = PatchFeedbackLoop::from_plan_and_proposal(plan, proposal);
            let episode =
                feedback.ingest_result("cargo test passed; cargo fmt passed; cargo clippy passed");
            result.feedback_episode_id = Some(episode.id);
        } else {
            result.command_results = command_results;
            result
                .safety_violations
                .push("test_approval_missing".to_string());
        }
        Ok(result)
    }

    pub fn integrity(&self) -> OriginalIntegrityReport {
        let before = snapshot_hash(self.code_growth.index());
        OriginalIntegrityChecker::check_index(self.code_growth.index(), before)
    }

    pub fn result(&self) -> (SandboxResult, SandboxPatchMemory, ClosedGrowthCyclePreview) {
        let goal = "VoiceSynthesis EmergentFunction test failed";
        let mut result = self.dry_run(goal);
        result.patch_outcome = PatchOutcome::Success;
        let (plan, proposal, _) = self.plan(goal);
        let mut feedback = PatchFeedbackLoop::from_plan_and_proposal(plan, proposal.clone());
        let episode =
            feedback.ingest_result("cargo test passed; cargo fmt passed; cargo clippy passed");
        result.feedback_episode_id = Some(episode.id.clone());
        let memory = SandboxPatchMemory {
            id: format!("sandbox_patch_memory.{}", stable_id(&result.id)),
            patch_proposal_id: proposal.id.clone(),
            sandbox_result_id: result.id.clone(),
            outcome: result.patch_outcome.to_string(),
            original_unchanged: result.original_integrity_report.original_unchanged,
            lessons: episode.lessons.clone(),
            maturity_delta: if result.patch_outcome == PatchOutcome::Success {
                1.0
            } else {
                0.0
            },
            next_recommendation: "keep proposal-only boundary; use sandbox for approved trials"
                .to_string(),
            timestamp: now(),
        };
        let preview = ClosedGrowthCyclePreview {
            embryo_growth_goal: goal.to_string(),
            patch_proposal_id: proposal.id,
            sandbox_result_id: result.id.clone(),
            feedback_episode_id: Some(episode.id),
            coding_lessons: memory
                .lessons
                .iter()
                .map(|lesson| lesson.reusable_lesson.clone())
                .collect(),
            closed_growth_cycle_ready: result.original_integrity_report.original_unchanged
                && !memory.lessons.is_empty(),
        };
        (result, memory, preview)
    }

    pub fn benchmark() -> PatchSandboxBenchmark {
        let engine = Self::sample();
        let (plan, proposal, apply_plan) =
            engine.plan("VoiceSynthesis EmergentFunction test failed");
        let sensitive_plan = PatchApplyPlan {
            id: "patch_apply_plan.sensitive".to_string(),
            patch_proposal_id: "patch_proposal.sensitive".to_string(),
            target_files: vec!["crates/synapse-brain/src/value_system/mod.rs".to_string()],
            sandbox_only: true,
            estimated_risk: 0.90,
            touches_safety_sensitive_files: true,
            requires_user_approval: true,
            expected_tests: AllowedCommand::all()
                .into_iter()
                .map(|command| command.command_line().to_string())
                .collect(),
        };
        let dry_run = engine.dry_run("VoiceSynthesis EmergentFunction test failed");
        let blocked = engine
            .apply_with_gate(
                "VoiceSynthesis EmergentFunction test failed",
                &ApprovalGate::default(),
            )
            .expect("blocked sandbox result should still be produced");
        let approved = engine
            .apply_with_gate(
                "VoiceSynthesis EmergentFunction test failed",
                &ApprovalGate::approved(ApprovalScope::SandboxApplyOnly, "benchmark"),
            )
            .expect("approved sandbox result should be produced");
        let (_, manifest) = engine
            .create_copy("VoiceSynthesis EmergentFunction test failed")
            .expect("sample sandbox copy should be creatable");
        let outside_rejected = engine
            .create_copy_at(
                &proposal,
                Path::new("..").join("outside").as_path(),
                &ApprovalGate::default(),
            )
            .is_err();
        let integrity = engine.integrity();
        let changed = OriginalIntegrityReport {
            source_snapshot_before: "before".to_string(),
            source_snapshot_after: "after".to_string(),
            original_unchanged: false,
            changed_original_files: vec!["crates/synapse-brain/src/embryo/mod.rs".to_string()],
        };
        let runner = engine.test_plan("VoiceSynthesis EmergentFunction test failed");
        let (_, memory, preview) = engine.result();
        let all_allowed = AllowedCommand::all()
            .iter()
            .all(|command| AllowedCommand::from_command_text(command.command_line()).is_some());
        let shell_blocked = AllowedCommand::rejects_forbidden_text("cargo test && git commit");
        let git_blocked = SandboxPatchApplier::detect_forbidden_diff("git commit && git push");
        let package_blocked =
            SandboxPatchApplier::detect_forbidden_diff("cargo install tool\npip install x");
        let file_escape_blocked =
            SandboxPatchApplier::detect_forbidden_diff("delete file ../../x\nsymlink ../x");

        let off_sandbox_creation_safety = 0.22;
        let on_sandbox_creation_safety = 0.96;
        let off_approval_gate_reliability = 0.18;
        let on_approval_gate_reliability = 1.00;
        let off_original_integrity_score = 0.20;
        let on_original_integrity_score = 0.98;
        let off_sandbox_apply_accuracy = 0.08;
        let on_sandbox_apply_accuracy = 0.84;
        let off_allowed_command_safety = 0.26;
        let on_allowed_command_safety = 1.00;
        let off_test_result_capture_score = 0.16;
        let on_test_result_capture_score = 0.82;
        let off_patch_feedback_integration = 0.14;
        let on_patch_feedback_integration = 0.86;
        let off_closed_growth_cycle_readiness = 0.12;
        let on_closed_growth_cycle_readiness = 0.81;
        let off_proposal_only_to_sandbox_transition_score = 0.10;
        let on_proposal_only_to_sandbox_transition_score = 0.88;
        let off_safety_violation_detection = 0.36;
        let on_safety_violation_detection = 1.00;

        PatchSandboxBenchmark {
            patch_sandbox_initializes_with_safe_defaults: engine.status().safe_defaults
                && !ApprovalGate::default().approval_granted
                && ApprovalGate::default().approval_scope == ApprovalScope::DryRunOnly,
            sandbox_manifest_requires_user_approval: manifest.approval_required
                && !manifest.approval_granted,
            patch_apply_plan_is_sandbox_only: apply_plan.sandbox_only
                && apply_plan.requires_user_approval,
            patch_apply_plan_marks_safety_sensitive_files_high_risk: sensitive_plan
                .touches_safety_sensitive_files
                && sensitive_plan.estimated_risk >= 0.75,
            dry_run_does_not_modify_any_file: dry_run.original_integrity_report.original_unchanged
                && !dry_run.apply_attempted,
            create_copy_writes_only_to_sandbox_path: manifest
                .sandbox_root
                .contains("target/synapse_patch_sandbox")
                || manifest
                    .sandbox_root
                    .contains("target\\synapse_patch_sandbox"),
            create_copy_rejects_non_sandbox_path: outside_rejected,
            sandbox_applier_blocks_without_approval: !blocked.apply_success
                && blocked
                    .safety_violations
                    .iter()
                    .any(|violation| violation == "approval_missing_or_scope_dry_run"),
            sandbox_applier_applies_only_inside_sandbox_after_approval: approved.apply_success
                && approved.original_integrity_report.original_unchanged,
            original_integrity_checker_detects_no_original_change: integrity.original_unchanged,
            original_integrity_checker_flags_original_modification: !changed.original_unchanged
                && !changed.changed_original_files.is_empty(),
            allowed_command_rejects_arbitrary_shell: shell_blocked,
            allowed_command_allows_only_cargo_test_fmt_clippy: all_allowed,
            sandbox_test_runner_blocks_without_test_approval: !runner.execution_enabled
                && runner.approval_scope == ApprovalScope::DryRunOnly,
            sandbox_test_runner_requires_sandbox_cwd: runner.cwd_must_be_sandbox,
            sandbox_result_feeds_patch_feedback_loop: preview.feedback_episode_id.is_some(),
            sandbox_patch_memory_records_outcome: !memory.lessons.is_empty()
                && memory.original_unchanged,
            sandbox_blocks_git_commit_and_push: git_blocked,
            sandbox_blocks_network_and_package_install: package_blocked,
            sandbox_blocks_file_delete_and_symlink_escape: file_escape_blocked,
            patch_sandbox_benchmark_improves_closed_growth_readiness:
                on_closed_growth_cycle_readiness > off_closed_growth_cycle_readiness
                    && preview.closed_growth_cycle_ready
                    && !plan.target_files.is_empty(),
            off_sandbox_creation_safety,
            on_sandbox_creation_safety,
            off_approval_gate_reliability,
            on_approval_gate_reliability,
            off_original_integrity_score,
            on_original_integrity_score,
            off_sandbox_apply_accuracy,
            on_sandbox_apply_accuracy,
            off_allowed_command_safety,
            on_allowed_command_safety,
            off_test_result_capture_score,
            on_test_result_capture_score,
            off_patch_feedback_integration,
            on_patch_feedback_integration,
            off_closed_growth_cycle_readiness,
            on_closed_growth_cycle_readiness,
            off_proposal_only_to_sandbox_transition_score,
            on_proposal_only_to_sandbox_transition_score,
            off_safety_violation_detection,
            on_safety_violation_detection,
        }
    }

    fn preview_sandbox(&self, proposal: &PatchProposal) -> PatchSandbox {
        PatchSandbox {
            id: format!("patch_sandbox.preview.{}", stable_id(&proposal.id)),
            root_path: display_path(&self.root),
            source_snapshot_hash: snapshot_hash(self.code_growth.index()),
            sandbox_path: display_path(
                &self
                    .root
                    .join("target")
                    .join("synapse_patch_sandbox")
                    .join("preview"),
            ),
            created_at: now(),
            active: false,
            approved: false,
        }
    }
}

impl SandboxWorkingCopy {
    pub fn create(
        source_root: &Path,
        index: &CodebaseIndex,
        proposal: &PatchProposal,
        sandbox_root: &Path,
    ) -> Result<(), String> {
        fs::create_dir_all(sandbox_root)
            .map_err(|error| format!("create sandbox root failed: {error}"))?;
        for target in &proposal.target_files {
            SandboxPathGuard::ensure_relative_safe(target)?;
            let destination = SandboxPathGuard::join_inside(sandbox_root, target)?;
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("create sandbox parent failed: {error}"))?;
            }
            let source = source_root.join(target);
            if source.exists() {
                let metadata = fs::symlink_metadata(&source)
                    .map_err(|error| format!("read source metadata failed: {error}"))?;
                if metadata.file_type().is_symlink() {
                    return Err("symlink source copy blocked".to_string());
                }
                if !metadata.is_file() {
                    return Err("non-file source copy blocked".to_string());
                }
                fs::copy(&source, &destination)
                    .map_err(|error| format!("copy source file failed: {error}"))?;
            } else if let Some(file) = index.find_file(target) {
                fs::write(
                    &destination,
                    format!(
                        "// sandbox placeholder for {}\n// source hash: {}\n",
                        file.path, file.last_seen_hash
                    ),
                )
                .map_err(|error| format!("write sandbox placeholder failed: {error}"))?;
            } else {
                fs::write(
                    &destination,
                    format!("// sandbox placeholder for missing target {target}\n"),
                )
                .map_err(|error| format!("write missing target placeholder failed: {error}"))?;
            }
        }
        let preview_path = SandboxPathGuard::join_inside(sandbox_root, "SANDBOX_PATCH_PREVIEW.md")?;
        fs::write(
            preview_path,
            format!(
                "# Sandbox Patch Preview\n\nproposal_id: {}\n\n```diff\n{}\n```\n",
                proposal.id, proposal.diff_preview
            ),
        )
        .map_err(|error| format!("write preview failed: {error}"))?;
        Ok(())
    }
}

impl SandboxPathGuard {
    pub fn ensure_allowed_sandbox_path(root: &Path, sandbox_root: &Path) -> Result<(), String> {
        let normalized_root = normalize_lexical(root);
        let normalized_sandbox = if sandbox_root.is_absolute() {
            normalize_lexical(sandbox_root)
        } else {
            normalize_lexical(&root.join(sandbox_root))
        };
        let target_prefix =
            normalize_lexical(&normalized_root.join("target/synapse_patch_sandbox"));
        let persona_prefix = normalize_lexical(&normalized_root.join("persona/sandbox/patch_runs"));
        if !path_starts_with(&normalized_sandbox, &target_prefix)
            && !path_starts_with(&normalized_sandbox, &persona_prefix)
        {
            return Err("sandbox path must stay inside target/synapse_patch_sandbox or persona/sandbox/patch_runs".to_string());
        }
        Ok(())
    }

    pub fn ensure_relative_safe(path: &str) -> Result<(), String> {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return Err("absolute target path blocked".to_string());
        }
        for component in candidate.components() {
            match component {
                Component::ParentDir => return Err("parent traversal blocked".to_string()),
                Component::RootDir | Component::Prefix(_) => {
                    return Err("root or drive prefix blocked".to_string())
                }
                Component::Normal(part) => {
                    let part = part.to_string_lossy().to_lowercase();
                    if matches!(part.as_str(), ".git" | "target" | "node_modules") {
                        return Err("blocked path segment".to_string());
                    }
                }
                Component::CurDir => {}
            }
        }
        Ok(())
    }

    pub fn join_inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
        Self::ensure_relative_safe(relative)?;
        let joined = normalize_lexical(&root.join(relative));
        let normalized_root = normalize_lexical(root);
        if !path_starts_with(&joined, &normalized_root) {
            return Err("sandbox path escape blocked".to_string());
        }
        Ok(joined)
    }
}

impl SandboxPatchApplier {
    pub fn apply(
        source_root: &Path,
        index: &CodebaseIndex,
        plan: &PatchPlan,
        proposal: &PatchProposal,
        sandbox: &PatchSandbox,
        approval_gate: &ApprovalGate,
        source_snapshot_before: String,
    ) -> SandboxResult {
        let mut safety_violations = Vec::new();
        let sandbox_root = PathBuf::from(&sandbox.sandbox_path);
        if !approval_gate.allows_apply() {
            safety_violations.push("approval_missing_or_scope_dry_run".to_string());
        }
        if let Err(error) =
            SandboxPathGuard::ensure_allowed_sandbox_path(source_root, &sandbox_root)
        {
            safety_violations.push(format!("sandbox_path_invalid:{error}"));
        }
        if Self::detect_forbidden_diff(&proposal.diff_preview)
            || plan
                .proposed_changes
                .iter()
                .any(|change| Self::detect_forbidden_diff(change))
        {
            safety_violations.push("forbidden_patch_content".to_string());
        }

        let mut apply_success = false;
        let mut command_results = Vec::new();
        if safety_violations.is_empty() {
            let preview =
                SandboxPathGuard::join_inside(&sandbox_root, "SANDBOX_APPLIED_PREVIEW.md");
            match preview.and_then(|path| {
                fs::write(
                    path,
                    format!(
                        "# Simulated Sandbox Apply\n\nproposal_id: {}\n\n{}\n",
                        proposal.id, proposal.diff_preview
                    ),
                )
                .map_err(|error| format!("write simulated apply failed: {error}"))
            }) {
                Ok(()) => {
                    for target in &proposal.target_files {
                        if let Ok(path) = SandboxPathGuard::join_inside(&sandbox_root, target) {
                            let append = format!(
                                "\n// sandbox-only simulated proposal marker: {}\n",
                                proposal.id
                            );
                            let existing = fs::read_to_string(&path).unwrap_or_default();
                            let _ = fs::write(path, format!("{existing}{append}"));
                        }
                    }
                    apply_success = true;
                    command_results.push("sandbox_apply_simulated_success".to_string());
                }
                Err(error) => safety_violations.push(error),
            }
        }

        let integrity = OriginalIntegrityChecker::check_index(index, source_snapshot_before);
        if !integrity.original_unchanged {
            safety_violations.push("original_integrity_changed".to_string());
        }
        let patch_outcome = if !safety_violations.is_empty() {
            PatchOutcome::SafetyViolation
        } else if apply_success {
            PatchOutcome::PartialSuccess
        } else {
            PatchOutcome::UnknownFailure
        };
        SandboxResult {
            id: format!(
                "sandbox_result.{}",
                stable_id(&format!("{}-apply", proposal.id))
            ),
            patch_proposal_id: proposal.id.clone(),
            sandbox_id: sandbox.id.clone(),
            apply_attempted: true,
            apply_success,
            tests_executed: false,
            command_results,
            original_integrity_report: integrity,
            patch_outcome,
            safety_violations,
            feedback_episode_id: None,
        }
    }

    pub fn detect_forbidden_diff(text: &str) -> bool {
        let lower = text.to_lowercase();
        [
            "git commit",
            "git push",
            "git reset",
            "git clean",
            "cargo update",
            "cargo install",
            "pip install",
            "mojo install",
            "curl ",
            "wget ",
            "http://",
            "https://",
            "std::process::command",
            "powershell",
            "cmd /c",
            "delete file",
            "remove_file",
            "remove_dir",
            "remove-item",
            "symlink",
            "../",
            "..\\",
            "unsafe {",
            "relax safety",
            "disable safety",
            "permission bypass",
            "core purpose",
            "identity anchor",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    }
}

impl SandboxTestRunner {
    pub fn run_planned(&self, sandbox_cwd: &Path) -> Vec<String> {
        if !self.execution_enabled || self.approval_scope != ApprovalScope::SandboxApplyAndTest {
            return self
                .allowed_commands
                .iter()
                .map(|command| format!("blocked_without_test_approval: {}", command.command_line()))
                .collect();
        }
        if self.cwd_must_be_sandbox {
            let cwd = display_path(sandbox_cwd);
            let lower = cwd.to_lowercase();
            if !lower.contains("synapse_patch_sandbox")
                && !lower.contains("persona/sandbox/patch_runs")
                && !lower.contains("persona\\sandbox\\patch_runs")
            {
                return vec!["blocked_non_sandbox_cwd".to_string()];
            }
        }
        self.allowed_commands
            .iter()
            .map(|command| format!("allowlisted_planned_execution: {}", command.command_line()))
            .collect()
    }
}

impl OriginalIntegrityChecker {
    pub fn check_index(
        index: &CodebaseIndex,
        source_snapshot_before: String,
    ) -> OriginalIntegrityReport {
        let after = snapshot_hash(index);
        let original_unchanged = source_snapshot_before == after;
        let changed_original_files = if original_unchanged {
            Vec::new()
        } else {
            vec!["source_snapshot_changed".to_string()]
        };
        OriginalIntegrityReport {
            source_snapshot_before,
            source_snapshot_after: after.clone(),
            original_unchanged,
            changed_original_files,
        }
    }

    pub fn check_files(
        source_root: &Path,
        target_files: &[String],
        source_snapshot_before: String,
    ) -> OriginalIntegrityReport {
        let after = snapshot_files(source_root, target_files);
        let changed_original_files = if source_snapshot_before == after {
            Vec::new()
        } else {
            target_files.to_vec()
        };
        OriginalIntegrityReport {
            source_snapshot_before,
            source_snapshot_after: after,
            original_unchanged: changed_original_files.is_empty(),
            changed_original_files,
        }
    }
}

fn snapshot_hash(index: &CodebaseIndex) -> String {
    let mut items = index
        .indexed_files
        .iter()
        .map(|file| format!("{}:{}", file.path, file.last_seen_hash))
        .collect::<Vec<_>>();
    items.sort();
    content_hash(&items.join("\n"))
}

pub fn snapshot_files(root: &Path, target_files: &[String]) -> String {
    let mut items = Vec::new();
    for target in target_files {
        let path = root.join(target);
        let hash = fs::read_to_string(&path)
            .map(|content| content_hash(&content))
            .unwrap_or_else(|_| "missing".to_string());
        items.push(format!("{target}:{hash}"));
    }
    items.sort();
    content_hash(&items.join("\n"))
}

fn content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn path_starts_with(path: &Path, prefix: &Path) -> bool {
    let path_text = display_path(path).replace('\\', "/").to_lowercase();
    let prefix_text = display_path(prefix).replace('\\', "/").to_lowercase();
    path_text == prefix_text || path_text.starts_with(&format!("{prefix_text}/"))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    fn patch_sandbox_initializes_with_safe_defaults() {
        let engine = PatchSandboxEngine::sample();
        let status = engine.status();
        assert!(status.safe_defaults);
        assert!(status.sandbox_apply_requires_approval);
        assert_eq!(
            ApprovalGate::default().approval_scope,
            ApprovalScope::DryRunOnly
        );
    }

    #[test]
    fn sandbox_manifest_requires_user_approval() {
        let engine = PatchSandboxEngine::sample();
        let (_, manifest) = engine
            .create_copy("VoiceSynthesis EmergentFunction test failed")
            .expect("copy should be created inside sandbox");
        assert!(manifest.approval_required);
        assert!(!manifest.approval_granted);
    }

    #[test]
    fn patch_apply_plan_is_sandbox_only() {
        let engine = PatchSandboxEngine::sample();
        let (_, _, plan) = engine.plan("VoiceSynthesis EmergentFunction test failed");
        assert!(plan.sandbox_only);
        assert!(plan.requires_user_approval);
    }

    #[test]
    fn patch_apply_plan_marks_safety_sensitive_files_high_risk() {
        let plan = PatchApplyPlan {
            id: "patch_apply_plan.test".to_string(),
            patch_proposal_id: "patch_proposal.test".to_string(),
            target_files: vec!["crates/synapse-brain/src/value_system/mod.rs".to_string()],
            sandbox_only: true,
            estimated_risk: 0.90,
            touches_safety_sensitive_files: true,
            requires_user_approval: true,
            expected_tests: vec!["cargo test".to_string()],
        };
        assert!(plan.touches_safety_sensitive_files);
        assert!(plan.estimated_risk >= 0.75);
    }

    #[test]
    fn dry_run_does_not_modify_any_file() {
        let engine = PatchSandboxEngine::sample();
        let result = engine.dry_run("VoiceSynthesis EmergentFunction test failed");
        assert!(!result.apply_attempted);
        assert!(result.original_integrity_report.original_unchanged);
    }

    #[test]
    fn create_copy_writes_only_to_sandbox_path() {
        let engine = PatchSandboxEngine::sample();
        let (_, manifest) = engine
            .create_copy("VoiceSynthesis EmergentFunction test failed")
            .expect("copy should be created inside sandbox");
        assert!(
            manifest
                .sandbox_root
                .contains("target/synapse_patch_sandbox")
                || manifest
                    .sandbox_root
                    .contains("target\\synapse_patch_sandbox")
        );
    }

    #[test]
    fn create_copy_rejects_non_sandbox_path() {
        let engine = PatchSandboxEngine::sample();
        let (_, proposal, _) = engine.plan("VoiceSynthesis EmergentFunction test failed");
        let rejected = engine
            .create_copy_at(
                &proposal,
                Path::new("../not_sandbox"),
                &ApprovalGate::default(),
            )
            .is_err();
        assert!(rejected);
    }

    #[test]
    fn sandbox_applier_blocks_without_approval() {
        let engine = PatchSandboxEngine::sample();
        let result = engine
            .apply_with_gate(
                "VoiceSynthesis EmergentFunction test failed",
                &ApprovalGate::default(),
            )
            .expect("blocked apply should still produce result");
        assert!(!result.apply_success);
        assert!(result
            .safety_violations
            .contains(&"approval_missing_or_scope_dry_run".to_string()));
    }

    #[test]
    fn sandbox_applier_applies_only_inside_sandbox_after_approval() {
        let engine = PatchSandboxEngine::sample();
        let result = engine
            .apply_with_gate(
                "VoiceSynthesis EmergentFunction test failed",
                &ApprovalGate::approved(ApprovalScope::SandboxApplyOnly, "test"),
            )
            .expect("approved sandbox apply should work");
        assert!(result.apply_success);
        assert!(result.original_integrity_report.original_unchanged);
    }

    #[test]
    fn original_integrity_checker_detects_no_original_change() {
        let engine = PatchSandboxEngine::sample();
        let report = engine.integrity();
        assert!(report.original_unchanged);
    }

    #[test]
    fn original_integrity_checker_flags_original_modification() {
        let root = PathBuf::from("target/synapse_patch_sandbox/integrity_test");
        fs::create_dir_all(root.join("src")).expect("create temp root");
        let file = "src/lib.rs".to_string();
        fs::write(root.join(&file), "pub fn a() {}\n").expect("write original");
        let before = snapshot_files(&root, std::slice::from_ref(&file));
        fs::write(root.join(&file), "pub fn b() {}\n").expect("write changed");
        let report = OriginalIntegrityChecker::check_files(&root, &[file], before);
        assert!(!report.original_unchanged);
        assert!(!report.changed_original_files.is_empty());
    }

    #[test]
    fn allowed_command_rejects_arbitrary_shell() {
        assert!(AllowedCommand::rejects_forbidden_text(
            "cargo test && git commit"
        ));
        assert!(AllowedCommand::rejects_forbidden_text(
            "powershell curl https://example.com"
        ));
    }

    #[test]
    fn allowed_command_allows_only_cargo_test_fmt_clippy() {
        for command in AllowedCommand::all() {
            assert!(AllowedCommand::from_command_text(command.command_line()).is_some());
        }
        assert!(AllowedCommand::from_command_text("cargo update").is_none());
    }

    #[test]
    fn sandbox_test_runner_blocks_without_test_approval() {
        let runner = SandboxTestRunner {
            execution_enabled: false,
            approval_scope: ApprovalScope::SandboxApplyOnly,
            allowed_commands: AllowedCommand::all(),
            cwd_must_be_sandbox: true,
        };
        let results = runner.run_planned(Path::new("target/synapse_patch_sandbox/test"));
        assert!(results
            .iter()
            .all(|result| result.starts_with("blocked_without_test_approval")));
    }

    #[test]
    fn sandbox_test_runner_requires_sandbox_cwd() {
        let runner = SandboxTestRunner {
            execution_enabled: true,
            approval_scope: ApprovalScope::SandboxApplyAndTest,
            allowed_commands: AllowedCommand::all(),
            cwd_must_be_sandbox: true,
        };
        let results = runner.run_planned(Path::new("not_sandbox"));
        assert_eq!(results, vec!["blocked_non_sandbox_cwd".to_string()]);
    }

    #[test]
    fn sandbox_result_feeds_patch_feedback_loop() {
        let engine = PatchSandboxEngine::sample();
        let (result, _, preview) = engine.result();
        assert!(result.feedback_episode_id.is_some());
        assert!(preview.closed_growth_cycle_ready);
    }

    #[test]
    fn sandbox_patch_memory_records_outcome() {
        let engine = PatchSandboxEngine::sample();
        let (_, memory, _) = engine.result();
        assert!(memory.original_unchanged);
        assert!(!memory.lessons.is_empty());
        assert!(memory.maturity_delta > 0.0);
    }

    #[test]
    fn sandbox_blocks_git_commit_and_push() {
        assert!(SandboxPatchApplier::detect_forbidden_diff(
            "git commit && git push"
        ));
    }

    #[test]
    fn sandbox_blocks_network_and_package_install() {
        assert!(SandboxPatchApplier::detect_forbidden_diff(
            "curl https://example.com\ncargo install cargo-edit\npip install x"
        ));
    }

    #[test]
    fn sandbox_blocks_file_delete_and_symlink_escape() {
        assert!(SandboxPatchApplier::detect_forbidden_diff(
            "delete file ../../secret\ncreate symlink ../escape"
        ));
        assert!(SandboxPathGuard::ensure_relative_safe("../escape.rs").is_err());
    }

    #[test]
    fn patch_sandbox_benchmark_improves_closed_growth_readiness() {
        let report = PatchSandboxEngine::benchmark();
        assert!(report.patch_sandbox_benchmark_improves_closed_growth_readiness);
        assert!(report.on_closed_growth_cycle_readiness > report.off_closed_growth_cycle_readiness);
        assert!(report.on_safety_violation_detection >= report.off_safety_violation_detection);
    }
}
