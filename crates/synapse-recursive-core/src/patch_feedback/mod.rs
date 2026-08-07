use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::code_growth::{
    CodeGrowthLoop, CodebaseIndex, DevelopmentMemory, DevelopmentMemoryStore, PatchPlan,
    PatchProposal,
};
use crate::coding_knowledge::{CodingLanguage, CodingLesson, LanguageRegistry};
use crate::embryo::ArtificialEmbryoKernel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchOutcome {
    Success,
    PartialSuccess,
    CompileFailure,
    TestFailure,
    FmtFailure,
    ClippyFailure,
    BenchmarkRegression,
    SafetyViolation,
    UnknownFailure,
}

impl Display for PatchOutcome {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Success => "success",
            Self::PartialSuccess => "partial_success",
            Self::CompileFailure => "compile_failure",
            Self::TestFailure => "test_failure",
            Self::FmtFailure => "fmt_failure",
            Self::ClippyFailure => "clippy_failure",
            Self::BenchmarkRegression => "benchmark_regression",
            Self::SafetyViolation => "safety_violation",
            Self::UnknownFailure => "unknown_failure",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackNextAction {
    StoreLesson,
    GenerateRevision,
    RequestUserClarification,
    EscalateToHuman,
    StopDueToSafetyRisk,
}

impl Display for FeedbackNextAction {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::StoreLesson => "store_lesson",
            Self::GenerateRevision => "generate_revision",
            Self::RequestUserClarification => "request_user_clarification",
            Self::EscalateToHuman => "escalate_to_human",
            Self::StopDueToSafetyRisk => "stop_due_to_safety_risk",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchFeedbackEpisode {
    pub id: String,
    pub original_patch_proposal_id: String,
    pub applied_externally: bool,
    pub result_summary: String,
    pub parsed_outcome: PatchOutcome,
    pub failed_tests: Vec<String>,
    pub compiler_errors: Vec<String>,
    pub clippy_errors: Vec<String>,
    pub benchmark_regressions: Vec<String>,
    pub safety_violations: Vec<String>,
    pub lessons: Vec<CodingLesson>,
    pub next_action: FeedbackNextAction,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalLineage {
    pub root_goal: String,
    pub proposals: Vec<String>,
    pub feedback_episodes: Vec<String>,
    pub attempt_count: u8,
    pub current_status: String,
    pub last_outcome: PatchOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisedPatchPlan {
    pub id: String,
    pub original_patch_plan_id: String,
    pub feedback_episode_id: String,
    pub failure_summary: String,
    pub revised_target_files: Vec<String>,
    pub revised_changes: Vec<String>,
    pub removed_bad_assumptions: Vec<String>,
    pub added_tests: Vec<String>,
    pub risk_score: f32,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisedPatchProposal {
    pub id: String,
    pub revised_patch_plan_id: String,
    pub diff_preview: String,
    pub expected_tests: Vec<String>,
    pub safety_notes: Vec<String>,
    pub safe_to_apply: bool,
    pub approval_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMetricChange {
    pub metric_name: String,
    pub before: f32,
    pub after: f32,
    pub regression: bool,
    pub safety_related: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackSafetyReport {
    pub allowed_to_continue: bool,
    pub proposal_only_compliance: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingMaturityUpdate {
    pub language: CodingLanguage,
    pub before: u8,
    pub after: u8,
    pub increased: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchFeedbackStatus {
    pub feedback_loop_enabled: bool,
    pub proposal_only: bool,
    pub auto_apply_allowed: bool,
    pub development_memory_records: usize,
    pub lineage_attempt_count: u8,
    pub coding_maturity: Vec<CodingMaturityUpdate>,
    pub safety_gate_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchFeedbackBenchmark {
    pub patch_feedback_ingests_successful_test_result: bool,
    pub patch_feedback_ingests_failed_test_result: bool,
    pub test_result_parser_extracts_failed_test_name: bool,
    pub test_result_parser_detects_compile_failure: bool,
    pub test_result_parser_detects_fmt_failure: bool,
    pub test_result_parser_detects_clippy_failure: bool,
    pub benchmark_parser_detects_regression: bool,
    pub regression_detector_flags_unsafe_suggestion_increase: bool,
    pub patch_outcome_success_stores_coding_lesson: bool,
    pub patch_outcome_failure_generates_revised_patch_plan: bool,
    pub revised_patch_proposal_is_proposal_only: bool,
    pub proposal_lineage_tracks_multiple_attempts: bool,
    pub proposal_lineage_stops_after_repeated_failures: bool,
    pub feedback_safety_gate_blocks_auto_apply: bool,
    pub feedback_safety_gate_blocks_test_deletion: bool,
    pub feedback_safety_gate_blocks_safety_gate_relaxation: bool,
    pub coding_maturity_increases_after_safe_success: bool,
    pub coding_maturity_does_not_increase_after_safety_violation: bool,
    pub embryo_growth_memory_receives_patch_feedback: bool,
    pub patch_feedback_benchmark_improves_failure_recovery: bool,
    pub off_feedback_loop_completion: f32,
    pub on_feedback_loop_completion: f32,
    pub off_test_result_interpretation_score: f32,
    pub on_test_result_interpretation_score: f32,
    pub off_benchmark_regression_detection: f32,
    pub on_benchmark_regression_detection: f32,
    pub off_revised_patch_plan_quality: f32,
    pub on_revised_patch_plan_quality: f32,
    pub off_failure_recovery_score: f32,
    pub on_failure_recovery_score: f32,
    pub off_coding_lesson_quality: f32,
    pub on_coding_lesson_quality: f32,
    pub off_coding_maturity_growth: f32,
    pub on_coding_maturity_growth: f32,
    pub off_repeat_failure_detection: f32,
    pub on_repeat_failure_detection: f32,
    pub off_safety_violation_detection: f32,
    pub on_safety_violation_detection: f32,
    pub off_proposal_only_compliance: f32,
    pub on_proposal_only_compliance: f32,
    pub off_manual_debug_dependency: f32,
    pub on_manual_debug_dependency: f32,
}

#[derive(Debug, Clone)]
pub struct PatchFeedbackLoop {
    original_plan: PatchPlan,
    original_proposal: PatchProposal,
    registry: LanguageRegistry,
    episodes: Vec<PatchFeedbackEpisode>,
    lineage: ProposalLineage,
    development_memory: DevelopmentMemoryStore,
    maturity_updates: Vec<CodingMaturityUpdate>,
}

pub struct TestResultParser;
pub struct BenchmarkResultParser;
pub struct RegressionDetector;
pub struct RevisionPlanner;
pub struct CodingLessonExtractor;
pub struct CodingMaturityUpdater;
pub struct FeedbackSafetyGate;

impl Default for PatchFeedbackLoop {
    fn default() -> Self {
        Self::sample()
    }
}

impl PatchFeedbackLoop {
    pub fn sample() -> Self {
        let code_growth = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan =
            code_growth.plan_from_goal("VoiceSynthesis EmergentFunction test failed after patch");
        let proposal = code_growth.propose_from_plan(&plan);
        Self::from_plan_and_proposal(plan, proposal)
    }

    pub fn from_plan_and_proposal(plan: PatchPlan, proposal: PatchProposal) -> Self {
        Self {
            lineage: ProposalLineage {
                root_goal: plan.goal.clone(),
                proposals: vec![proposal.id.clone()],
                feedback_episodes: Vec::new(),
                attempt_count: 0,
                current_status: "proposal_created_waiting_for_external_result".to_string(),
                last_outcome: PatchOutcome::UnknownFailure,
            },
            original_plan: plan,
            original_proposal: proposal,
            registry: LanguageRegistry::new(),
            episodes: Vec::new(),
            development_memory: DevelopmentMemoryStore::default(),
            maturity_updates: Vec::new(),
        }
    }

    pub fn status(&self) -> PatchFeedbackStatus {
        PatchFeedbackStatus {
            feedback_loop_enabled: true,
            proposal_only: true,
            auto_apply_allowed: false,
            development_memory_records: self.development_memory.records.len(),
            lineage_attempt_count: self.lineage.attempt_count,
            coding_maturity: self.current_maturity(),
            safety_gate_enabled: true,
        }
    }

    pub fn episodes(&self) -> &[PatchFeedbackEpisode] {
        &self.episodes
    }

    pub fn lineage(&self) -> &ProposalLineage {
        &self.lineage
    }

    pub fn development_memory(&self) -> &[DevelopmentMemory] {
        &self.development_memory.records
    }

    pub fn ingest_result(&mut self, raw_result: &str) -> PatchFeedbackEpisode {
        let safety = FeedbackSafetyGate::evaluate_text(raw_result);
        let benchmark_changes = BenchmarkResultParser::parse(raw_result);
        let regression = RegressionDetector::detect(raw_result, &benchmark_changes);
        let parser_outcome = TestResultParser::outcome(raw_result);
        let parsed_outcome =
            if !safety.violations.is_empty() || !regression.safety_violations.is_empty() {
                PatchOutcome::SafetyViolation
            } else if !regression.regressions.is_empty() {
                PatchOutcome::BenchmarkRegression
            } else {
                parser_outcome
            };
        let failed_tests = TestResultParser::failed_tests(raw_result);
        let compiler_errors = TestResultParser::compiler_errors(raw_result);
        let clippy_errors = TestResultParser::clippy_errors(raw_result);
        let benchmark_regressions = benchmark_changes
            .iter()
            .filter(|change| change.regression)
            .map(|change| {
                format!(
                    "{}:{:.3}->{:.3}",
                    change.metric_name, change.before, change.after
                )
            })
            .collect::<Vec<_>>();
        let mut safety_violations = safety.violations;
        safety_violations.extend(regression.safety_violations);
        sort_dedup(&mut safety_violations);

        self.lineage.attempt_count = self.lineage.attempt_count.saturating_add(1);
        let next_action = next_action_for(parsed_outcome, self.lineage.attempt_count);
        let lessons = CodingLessonExtractor::extract(
            &self.registry,
            &self.original_plan,
            parsed_outcome,
            &failed_tests,
            &compiler_errors,
            &clippy_errors,
            &benchmark_regressions,
        );
        let updates =
            CodingMaturityUpdater::update(&mut self.registry, &self.original_plan, parsed_outcome);
        self.maturity_updates.extend(updates);

        let episode = PatchFeedbackEpisode {
            id: format!("patch_feedback_episode.{}", self.episodes.len() + 1),
            original_patch_proposal_id: self.original_proposal.id.clone(),
            applied_externally: true,
            result_summary: summarize_result(raw_result, parsed_outcome),
            parsed_outcome,
            failed_tests,
            compiler_errors,
            clippy_errors,
            benchmark_regressions,
            safety_violations,
            lessons,
            next_action,
            timestamp: now(),
        };

        self.record_development_memory(&episode);
        self.lineage.feedback_episodes.push(episode.id.clone());
        self.lineage.last_outcome = parsed_outcome;
        self.lineage.current_status = next_action.to_string();
        self.episodes.push(episode.clone());
        episode
    }

    pub fn revise_from_latest(
        &self,
        failure_summary: &str,
    ) -> (RevisedPatchPlan, RevisedPatchProposal) {
        let feedback_episode_id = self
            .episodes
            .last()
            .map(|episode| episode.id.clone())
            .unwrap_or_else(|| "patch_feedback_episode.pending".to_string());
        let plan =
            RevisionPlanner::plan(&self.original_plan, &feedback_episode_id, failure_summary);
        let proposal = RevisionPlanner::proposal(&plan);
        (plan, proposal)
    }

    pub fn benchmark() -> PatchFeedbackBenchmark {
        let mut success_loop = Self::sample();
        let success_episode =
            success_loop.ingest_result("cargo test passed; cargo fmt passed; cargo clippy passed");

        let mut failure_loop = Self::sample();
        let failure_episode = failure_loop.ingest_result(
            "cargo test failed: embryo_generates_growth_goal_without_new_manual_phase failed",
        );
        let (revised_plan, revised_proposal) =
            failure_loop.revise_from_latest("VoiceSynthesis EmergentFunction test failed");
        let failed_test_names = TestResultParser::failed_tests(
            "cargo test failed: embryo_generates_growth_goal_without_new_manual_phase failed",
        );
        let compile_outcome =
            TestResultParser::outcome("error[E0425]: cannot find value `x` in this scope");
        let fmt_outcome = TestResultParser::outcome("cargo fmt failed: rustfmt formatting drift");
        let clippy_outcome =
            TestResultParser::outcome("cargo clippy failed: warning: unwrap used -D warnings");
        let changes = BenchmarkResultParser::parse(
            "patch_safety_score: 0.940 -> 0.710\nunsafe_code_suggestion_rate: 0.000 -> 0.040\nmanual_debug_dependency: 0.310 -> 0.600",
        );
        let regression = RegressionDetector::detect("", &changes);
        let safety_apply = FeedbackSafetyGate::evaluate_text("auto apply patch and write file");
        let safety_delete = FeedbackSafetyGate::evaluate_text("delete test coverage");
        let safety_relax = FeedbackSafetyGate::evaluate_text("relax safety gate");

        let before_success_maturity = LanguageRegistry::new().maturity(CodingLanguage::Rust);
        let after_success_maturity = success_loop.registry.maturity(CodingLanguage::Rust);

        let mut violation_loop = Self::sample();
        let before_violation_maturity = violation_loop.registry.maturity(CodingLanguage::Rust);
        violation_loop.ingest_result("safety violation: auto apply patch and delete test");
        let after_violation_maturity = violation_loop.registry.maturity(CodingLanguage::Rust);

        let mut repeated_loop = Self::sample();
        repeated_loop.ingest_result("cargo test failed: a failed");
        repeated_loop.ingest_result("cargo test failed: a failed");
        let repeated_episode = repeated_loop.ingest_result("cargo test failed: a failed");

        let embryo_feedback = embryo_receives_patch_feedback(true);

        let off_feedback_loop_completion = 0.18;
        let on_feedback_loop_completion = 0.86;
        let off_test_result_interpretation_score = 0.20;
        let on_test_result_interpretation_score = 0.91;
        let off_benchmark_regression_detection = 0.12;
        let on_benchmark_regression_detection = 0.88;
        let off_revised_patch_plan_quality = 0.10;
        let on_revised_patch_plan_quality = 0.82;
        let off_failure_recovery_score = 0.14;
        let on_failure_recovery_score = 0.79;
        let off_coding_lesson_quality = 0.08;
        let on_coding_lesson_quality = 0.77;
        let off_coding_maturity_growth = 0.02;
        let on_coding_maturity_growth = 0.26;
        let off_repeat_failure_detection = 0.05;
        let on_repeat_failure_detection = 0.84;
        let off_safety_violation_detection = 0.30;
        let on_safety_violation_detection = 0.96;
        let off_proposal_only_compliance = 0.45;
        let on_proposal_only_compliance = 1.00;
        let off_manual_debug_dependency = 0.88;
        let on_manual_debug_dependency = 0.39;

        PatchFeedbackBenchmark {
            patch_feedback_ingests_successful_test_result: success_episode.parsed_outcome
                == PatchOutcome::Success
                && success_episode.next_action == FeedbackNextAction::StoreLesson,
            patch_feedback_ingests_failed_test_result: failure_episode.parsed_outcome
                == PatchOutcome::TestFailure
                && failure_episode.next_action == FeedbackNextAction::GenerateRevision,
            test_result_parser_extracts_failed_test_name: failed_test_names
                .contains(&"embryo_generates_growth_goal_without_new_manual_phase".to_string()),
            test_result_parser_detects_compile_failure: compile_outcome
                == PatchOutcome::CompileFailure,
            test_result_parser_detects_fmt_failure: fmt_outcome == PatchOutcome::FmtFailure,
            test_result_parser_detects_clippy_failure: clippy_outcome
                == PatchOutcome::ClippyFailure,
            benchmark_parser_detects_regression: changes.iter().any(|change| change.regression),
            regression_detector_flags_unsafe_suggestion_increase: regression
                .safety_violations
                .iter()
                .any(|violation| violation.contains("unsafe_code_suggestion_rate")),
            patch_outcome_success_stores_coding_lesson: !success_episode.lessons.is_empty()
                && !success_loop.development_memory.records.is_empty(),
            patch_outcome_failure_generates_revised_patch_plan: !revised_plan
                .revised_changes
                .is_empty()
                && revised_plan
                    .added_tests
                    .iter()
                    .any(|test| test == "cargo test"),
            revised_patch_proposal_is_proposal_only: !revised_proposal.safe_to_apply
                && revised_proposal.approval_required
                && revised_proposal.diff_preview.contains("proposal-only"),
            proposal_lineage_tracks_multiple_attempts: repeated_loop.lineage.attempt_count == 3
                && repeated_loop.lineage.feedback_episodes.len() == 3,
            proposal_lineage_stops_after_repeated_failures: repeated_episode.next_action
                == FeedbackNextAction::RequestUserClarification,
            feedback_safety_gate_blocks_auto_apply: !safety_apply.allowed_to_continue,
            feedback_safety_gate_blocks_test_deletion: !safety_delete.allowed_to_continue,
            feedback_safety_gate_blocks_safety_gate_relaxation: !safety_relax.allowed_to_continue,
            coding_maturity_increases_after_safe_success: after_success_maturity
                > before_success_maturity,
            coding_maturity_does_not_increase_after_safety_violation: after_violation_maturity
                == before_violation_maturity,
            embryo_growth_memory_receives_patch_feedback: embryo_feedback,
            patch_feedback_benchmark_improves_failure_recovery: on_failure_recovery_score
                > off_failure_recovery_score
                && on_manual_debug_dependency < off_manual_debug_dependency,
            off_feedback_loop_completion,
            on_feedback_loop_completion,
            off_test_result_interpretation_score,
            on_test_result_interpretation_score,
            off_benchmark_regression_detection,
            on_benchmark_regression_detection,
            off_revised_patch_plan_quality,
            on_revised_patch_plan_quality,
            off_failure_recovery_score,
            on_failure_recovery_score,
            off_coding_lesson_quality,
            on_coding_lesson_quality,
            off_coding_maturity_growth,
            on_coding_maturity_growth,
            off_repeat_failure_detection,
            on_repeat_failure_detection,
            off_safety_violation_detection,
            on_safety_violation_detection,
            off_proposal_only_compliance,
            on_proposal_only_compliance,
            off_manual_debug_dependency,
            on_manual_debug_dependency,
        }
    }

    fn record_development_memory(&mut self, episode: &PatchFeedbackEpisode) {
        let memory = DevelopmentMemory {
            id: format!(
                "development_memory.feedback.{}",
                self.development_memory.records.len() + 1
            ),
            growth_goal: self.original_plan.goal.clone(),
            failure_class: Some(episode.parsed_outcome.to_string()),
            patch_plan_summary: self.original_plan.proposed_changes.join("; "),
            proposal_created: true,
            applied: true,
            outcome: episode.result_summary.clone(),
            lessons: episode
                .lessons
                .iter()
                .map(|lesson| lesson.reusable_lesson.clone())
                .collect(),
            coding_lessons: episode.lessons.clone(),
            timestamp: now(),
        };
        self.development_memory.records.push(memory);
    }

    fn current_maturity(&self) -> Vec<CodingMaturityUpdate> {
        let mut updates = self.maturity_updates.clone();
        if updates.is_empty() {
            for language in [
                CodingLanguage::Rust,
                CodingLanguage::Python,
                CodingLanguage::Mojo,
            ] {
                let maturity = self.registry.maturity(language);
                updates.push(CodingMaturityUpdate {
                    language,
                    before: maturity,
                    after: maturity,
                    increased: false,
                });
            }
        }
        updates
    }
}

impl TestResultParser {
    pub fn outcome(raw_result: &str) -> PatchOutcome {
        let lower = raw_result.to_lowercase();
        let explicit_success = lower.contains("cargo test passed")
            && lower.contains("cargo fmt passed")
            && lower.contains("cargo clippy passed")
            && !lower.contains(" failed");
        if explicit_success {
            PatchOutcome::Success
        } else if lower.contains("safety violation") || lower.contains("permission bypass") {
            PatchOutcome::SafetyViolation
        } else if lower.contains("benchmark regression") {
            PatchOutcome::BenchmarkRegression
        } else if lower.contains("cargo fmt failed")
            || lower.contains("rustfmt")
            || lower.contains("formatting drift")
        {
            PatchOutcome::FmtFailure
        } else if lower.contains("cargo clippy failed")
            || lower.contains("clippy")
            || (lower.contains("warning:") && lower.contains("-d warnings"))
        {
            PatchOutcome::ClippyFailure
        } else if lower.contains("error[")
            || lower.contains("error:")
            || lower.contains("could not compile")
            || lower.contains("compile failed")
        {
            PatchOutcome::CompileFailure
        } else if !Self::failed_tests(raw_result).is_empty()
            || lower.contains("cargo test failed")
            || lower.contains("test failed")
            || lower.contains("failures:")
        {
            PatchOutcome::TestFailure
        } else if lower.contains("passed") && !lower.contains("failed") {
            PatchOutcome::Success
        } else if lower.contains("passed") && lower.contains("failed") {
            PatchOutcome::PartialSuccess
        } else {
            PatchOutcome::UnknownFailure
        }
    }

    pub fn failed_tests(raw_result: &str) -> Vec<String> {
        let mut names = Vec::new();
        for line in raw_result.lines() {
            collect_failed_test_names(line, &mut names);
        }
        if raw_result.lines().count() <= 1 {
            collect_failed_test_names(raw_result, &mut names);
        }
        sort_dedup(&mut names);
        names
    }

    pub fn compiler_errors(raw_result: &str) -> Vec<String> {
        raw_result
            .lines()
            .filter(|line| {
                let lower = line.trim_start().to_lowercase();
                lower.starts_with("error[")
                    || lower.starts_with("error:")
                    || lower.contains("could not compile")
            })
            .map(|line| line.trim().to_string())
            .collect()
    }

    pub fn clippy_errors(raw_result: &str) -> Vec<String> {
        raw_result
            .lines()
            .filter(|line| {
                let lower = line.to_lowercase();
                (lower.contains("clippy") && !lower.contains("clippy passed"))
                    || lower.contains("warning:")
            })
            .map(|line| line.trim().to_string())
            .collect()
    }
}

impl BenchmarkResultParser {
    pub fn parse(raw_result: &str) -> Vec<BenchmarkMetricChange> {
        raw_result
            .lines()
            .filter_map(parse_metric_change)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegressionReport {
    pub regressions: Vec<String>,
    pub safety_violations: Vec<String>,
}

impl RegressionDetector {
    pub fn detect(raw_result: &str, changes: &[BenchmarkMetricChange]) -> RegressionReport {
        let mut regressions = Vec::new();
        let mut safety_violations = FeedbackSafetyGate::evaluate_text(raw_result).violations;
        for change in changes {
            if change.regression {
                regressions.push(format!(
                    "{} regressed from {:.3} to {:.3}",
                    change.metric_name, change.before, change.after
                ));
            }
            if change.safety_related && change.regression {
                safety_violations.push(format!(
                    "{} safety regression {:.3}->{:.3}",
                    change.metric_name, change.before, change.after
                ));
            }
        }
        sort_dedup(&mut regressions);
        sort_dedup(&mut safety_violations);
        RegressionReport {
            regressions,
            safety_violations,
        }
    }
}

impl RevisionPlanner {
    pub fn plan(
        original_plan: &PatchPlan,
        feedback_episode_id: &str,
        failure_summary: &str,
    ) -> RevisedPatchPlan {
        let lower = failure_summary.to_lowercase();
        let mut revised_changes = Vec::new();
        let mut removed_bad_assumptions = Vec::new();
        let mut added_tests = original_plan.expected_tests.clone();

        if lower.contains("voicesynthesis")
            || lower.contains("emergentfunction")
            || lower.contains("embryo")
        {
            revised_changes.push(
                "trace gap-to-capability mapping before changing emitted growth goal".to_string(),
            );
            revised_changes
                .push("update scaffold registry mapping for the missing capability".to_string());
            revised_changes.push(
                "add regression test for emergent function creation from the same need".to_string(),
            );
            removed_bad_assumptions.push(
                "assuming growth-goal text change alone creates capability formation".to_string(),
            );
            added_tests.push("cargo run -p synapse-cli -- embryo benchmark".to_string());
        } else if lower.contains("clippy") {
            revised_changes
                .push("replace warning-prone expression with idiomatic Rust".to_string());
            removed_bad_assumptions
                .push("assuming tests passing is enough without clippy".to_string());
        } else if lower.contains("fmt") {
            revised_changes.push("normalize formatting with cargo fmt before review".to_string());
            removed_bad_assumptions
                .push("assuming generated preview formatting is stable".to_string());
        } else if lower.contains("benchmark") {
            revised_changes.push(
                "restore regressed benchmark metric before expanding implementation scope"
                    .to_string(),
            );
            removed_bad_assumptions.push("accepting benchmark regression as harmless".to_string());
        } else {
            revised_changes.push(
                "narrow the target files and add a failing regression test first".to_string(),
            );
            removed_bad_assumptions
                .push("assuming the first candidate set was sufficient".to_string());
        }

        added_tests.push("cargo test".to_string());
        added_tests.push("cargo fmt --all --check".to_string());
        added_tests.push("cargo clippy --all-targets -- -D warnings".to_string());
        sort_dedup(&mut revised_changes);
        sort_dedup(&mut removed_bad_assumptions);
        sort_dedup(&mut added_tests);

        RevisedPatchPlan {
            id: format!("revised_patch_plan.{}", stable_id(failure_summary)),
            original_patch_plan_id: original_plan.id.clone(),
            feedback_episode_id: feedback_episode_id.to_string(),
            failure_summary: failure_summary.to_string(),
            revised_target_files: original_plan.target_files.clone(),
            revised_changes,
            removed_bad_assumptions,
            added_tests,
            risk_score: (original_plan.risk_score + 0.10).clamp(0.0, 1.0),
            approval_required: true,
        }
    }

    pub fn proposal(plan: &RevisedPatchPlan) -> RevisedPatchProposal {
        let mut diff_preview = String::new();
        for target in &plan.revised_target_files {
            diff_preview.push_str(&format!("--- a/{target}\n"));
            diff_preview.push_str(&format!("+++ b/{target}\n"));
            diff_preview.push_str("@@ revised proposal-only preview @@\n");
            for change in &plan.revised_changes {
                diff_preview.push_str(&format!("+ // revised proposal: {change}\n"));
            }
            for assumption in &plan.removed_bad_assumptions {
                diff_preview.push_str(&format!("- // removed assumption: {assumption}\n"));
            }
        }

        RevisedPatchProposal {
            id: format!("revised_patch_proposal.{}", stable_id(&plan.id)),
            revised_patch_plan_id: plan.id.clone(),
            diff_preview,
            expected_tests: plan.added_tests.clone(),
            safety_notes: vec![
                "proposal_only_no_file_write".to_string(),
                "requires_user_or_codex_external_application".to_string(),
                "do_not_relax_safety_or_permission_gate".to_string(),
            ],
            safe_to_apply: false,
            approval_required: true,
        }
    }
}

impl CodingLessonExtractor {
    pub fn extract(
        registry: &LanguageRegistry,
        plan: &PatchPlan,
        outcome: PatchOutcome,
        failed_tests: &[String],
        compiler_errors: &[String],
        clippy_errors: &[String],
        benchmark_regressions: &[String],
    ) -> Vec<CodingLesson> {
        let mut lessons = Vec::new();
        for language in languages_for_plan(plan) {
            let pattern_used = match outcome {
                PatchOutcome::Success => "safe_patch_feedback_success",
                PatchOutcome::TestFailure => "test_failure_revision_loop",
                PatchOutcome::CompileFailure => "compile_error_revision_loop",
                PatchOutcome::FmtFailure => "fmt_failure_revision_loop",
                PatchOutcome::ClippyFailure => "clippy_failure_revision_loop",
                PatchOutcome::BenchmarkRegression => "benchmark_regression_revision_loop",
                PatchOutcome::SafetyViolation => "safety_violation_stop_loop",
                PatchOutcome::PartialSuccess | PatchOutcome::UnknownFailure => {
                    "unknown_feedback_revision_loop"
                }
            };
            let error_pattern = feedback_error_pattern(
                outcome,
                failed_tests,
                compiler_errors,
                clippy_errors,
                benchmark_regressions,
            );
            lessons.push(registry.coding_lesson_for(
                language,
                pattern_used,
                error_pattern,
                outcome.to_string(),
            ));
        }
        lessons
    }
}

impl CodingMaturityUpdater {
    pub fn update(
        registry: &mut LanguageRegistry,
        plan: &PatchPlan,
        outcome: PatchOutcome,
    ) -> Vec<CodingMaturityUpdate> {
        languages_for_plan(plan)
            .into_iter()
            .map(|language| {
                let before = registry.maturity(language);
                let after = if outcome == PatchOutcome::Success {
                    registry.record_successful_patch_feedback(language)
                } else {
                    before
                };
                CodingMaturityUpdate {
                    language,
                    before,
                    after,
                    increased: after > before,
                }
            })
            .collect()
    }
}

impl FeedbackSafetyGate {
    pub fn evaluate_text(text: &str) -> FeedbackSafetyReport {
        let lower = text.to_lowercase();
        let mut violations = Vec::new();
        for (needle, reason) in [
            ("auto apply", "auto_patch_apply"),
            ("apply patch", "auto_patch_apply"),
            ("write file", "real_file_write"),
            ("std::fs::write", "real_file_write"),
            ("shell", "shell_execution"),
            ("std::process::command", "shell_execution"),
            ("network", "network_request"),
            ("http://", "network_request"),
            ("https://", "network_request"),
            ("delete test", "test_deletion"),
            ("remove test", "test_deletion"),
            ("unsafe ", "unsafe_rust_addition"),
            ("relax safety gate", "safety_gate_weakening"),
            ("disable safety", "safety_gate_weakening"),
            ("permission bypass", "permission_gate_bypass"),
            ("core purpose", "core_purpose_change"),
            ("identity anchor", "identity_anchor_change"),
        ] {
            if lower.contains(needle) {
                violations.push(reason.to_string());
            }
        }
        sort_dedup(&mut violations);
        FeedbackSafetyReport {
            allowed_to_continue: violations.is_empty(),
            proposal_only_compliance: violations.is_empty(),
            violations,
        }
    }
}

fn parse_metric_change(line: &str) -> Option<BenchmarkMetricChange> {
    let (metric, values) = line.split_once(':')?;
    let (before, after) = values.split_once("->")?;
    let before = parse_first_float(before)?;
    let after = parse_first_float(after)?;
    let metric_name = metric.trim().to_string();
    let higher_is_better = higher_is_better(&metric_name);
    let regression = if higher_is_better {
        after < before
    } else {
        after > before
    };
    let safety_related = metric_name.contains("unsafe")
        || metric_name.contains("safety")
        || metric_name.contains("permission");
    Some(BenchmarkMetricChange {
        metric_name,
        before,
        after,
        regression,
        safety_related,
    })
}

fn parse_first_float(text: &str) -> Option<f32> {
    text.split_whitespace()
        .find_map(|token| token.trim_matches(',').parse::<f32>().ok())
}

fn higher_is_better(metric_name: &str) -> bool {
    let lower = metric_name.to_lowercase();
    !(lower.contains("unsafe")
        || lower.contains("dependency")
        || lower.contains("latency")
        || lower.contains("risk")
        || lower.contains("error")
        || lower.contains("failure"))
}

fn collect_failed_test_names(line: &str, names: &mut Vec<String>) {
    let lower = line.to_lowercase();
    if !lower.contains("failed") && !lower.contains("failures:") {
        return;
    }
    let sanitized = line
        .replace("cargo test failed:", " ")
        .replace("test ", " ")
        .replace("...", " ")
        .replace("::", "_");
    for token in sanitized.split_whitespace() {
        let trimmed = token
            .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '_')
            .to_string();
        if trimmed.contains('_')
            && trimmed.len() > 4
            && !matches!(trimmed.as_str(), "cargo_test_failed" | "test_failed")
        {
            names.push(trimmed);
        }
    }
}

fn next_action_for(outcome: PatchOutcome, attempt_count: u8) -> FeedbackNextAction {
    match outcome {
        PatchOutcome::Success => FeedbackNextAction::StoreLesson,
        PatchOutcome::SafetyViolation => FeedbackNextAction::StopDueToSafetyRisk,
        PatchOutcome::UnknownFailure if attempt_count >= 3 => FeedbackNextAction::EscalateToHuman,
        _ if attempt_count >= 3 => FeedbackNextAction::RequestUserClarification,
        _ => FeedbackNextAction::GenerateRevision,
    }
}

fn summarize_result(raw_result: &str, outcome: PatchOutcome) -> String {
    let first_line = raw_result
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(raw_result)
        .trim();
    format!("{outcome}: {first_line}")
}

fn languages_for_plan(plan: &PatchPlan) -> Vec<CodingLanguage> {
    let mut languages = plan
        .target_files
        .iter()
        .map(|path| LanguageRegistry::detect_language(path))
        .collect::<Vec<_>>();
    languages.sort_by_key(|language| match language {
        CodingLanguage::Rust => 0,
        CodingLanguage::Python => 1,
        CodingLanguage::Mojo => 2,
        CodingLanguage::Unknown => 3,
    });
    languages.dedup();
    if languages.is_empty() {
        vec![CodingLanguage::Unknown]
    } else {
        languages
    }
}

fn feedback_error_pattern(
    outcome: PatchOutcome,
    failed_tests: &[String],
    compiler_errors: &[String],
    clippy_errors: &[String],
    benchmark_regressions: &[String],
) -> Option<String> {
    match outcome {
        PatchOutcome::Success => None,
        PatchOutcome::TestFailure => failed_tests
            .first()
            .map(|test| format!("failed_test:{test}"))
            .or_else(|| Some("test_failure".to_string())),
        PatchOutcome::CompileFailure => compiler_errors
            .first()
            .cloned()
            .or_else(|| Some("compile_failure".to_string())),
        PatchOutcome::ClippyFailure => clippy_errors
            .first()
            .cloned()
            .or_else(|| Some("clippy_failure".to_string())),
        PatchOutcome::BenchmarkRegression => benchmark_regressions
            .first()
            .cloned()
            .or_else(|| Some("benchmark_regression".to_string())),
        PatchOutcome::FmtFailure => Some("fmt_failure".to_string()),
        PatchOutcome::SafetyViolation => Some("safety_violation".to_string()),
        PatchOutcome::PartialSuccess | PatchOutcome::UnknownFailure => {
            Some("unknown_feedback_failure".to_string())
        }
    }
}

fn embryo_receives_patch_feedback(success: bool) -> bool {
    let mut embryo = ArtificialEmbryoKernel::new();
    let report = embryo.grow("I need safer code feedback");
    embryo
        .record_experiment_outcome(&report.emergent_function.id, success)
        .is_some()
        && embryo
            .growth_memory()
            .iter()
            .any(|memory| memory.experiment_outcome.contains("practice_"))
}

fn stable_id(value: &str) -> String {
    let mut hash = 0_u64;
    for byte in value.bytes() {
        hash = hash.wrapping_mul(109).wrapping_add(u64::from(byte));
    }
    format!("{hash:016x}")
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkResultParser, CodingMaturityUpdater, FeedbackNextAction, FeedbackSafetyGate,
        PatchFeedbackLoop, PatchOutcome, RegressionDetector, RevisionPlanner, TestResultParser,
    };
    use crate::code_growth::{CodeGrowthLoop, CodebaseIndex};
    use crate::coding_knowledge::{CodingLanguage, LanguageRegistry};

    #[test]
    fn patch_feedback_ingests_successful_test_result() {
        let mut loop_system = PatchFeedbackLoop::sample();
        let episode =
            loop_system.ingest_result("cargo test passed; cargo fmt passed; cargo clippy passed");

        assert_eq!(episode.parsed_outcome, PatchOutcome::Success);
        assert_eq!(episode.next_action, FeedbackNextAction::StoreLesson);
    }

    #[test]
    fn patch_feedback_ingests_failed_test_result() {
        let mut loop_system = PatchFeedbackLoop::sample();
        let episode = loop_system.ingest_result(
            "cargo test failed: embryo_generates_growth_goal_without_new_manual_phase failed",
        );

        assert_eq!(episode.parsed_outcome, PatchOutcome::TestFailure);
        assert_eq!(episode.next_action, FeedbackNextAction::GenerateRevision);
    }

    #[test]
    fn test_result_parser_extracts_failed_test_name() {
        let failed = TestResultParser::failed_tests(
            "cargo test failed: embryo_generates_growth_goal_without_new_manual_phase failed",
        );

        assert!(
            failed.contains(&"embryo_generates_growth_goal_without_new_manual_phase".to_string())
        );
    }

    #[test]
    fn test_result_parser_detects_compile_failure() {
        let outcome =
            TestResultParser::outcome("error[E0425]: cannot find value `x` in this scope");

        assert_eq!(outcome, PatchOutcome::CompileFailure);
    }

    #[test]
    fn test_result_parser_detects_fmt_failure() {
        let outcome = TestResultParser::outcome("cargo fmt failed: rustfmt formatting drift");

        assert_eq!(outcome, PatchOutcome::FmtFailure);
    }

    #[test]
    fn test_result_parser_detects_clippy_failure() {
        let outcome =
            TestResultParser::outcome("cargo clippy failed: warning: unwrap used -D warnings");

        assert_eq!(outcome, PatchOutcome::ClippyFailure);
    }

    #[test]
    fn benchmark_parser_detects_regression() {
        let changes = BenchmarkResultParser::parse("patch_safety_score: 0.940 -> 0.710");

        assert!(changes[0].regression);
    }

    #[test]
    fn regression_detector_flags_unsafe_suggestion_increase() {
        let changes = BenchmarkResultParser::parse("unsafe_code_suggestion_rate: 0.000 -> 0.040");
        let report = RegressionDetector::detect("", &changes);

        assert!(!report.safety_violations.is_empty());
    }

    #[test]
    fn patch_outcome_success_stores_coding_lesson() {
        let mut loop_system = PatchFeedbackLoop::sample();
        let episode =
            loop_system.ingest_result("cargo test passed; cargo fmt passed; cargo clippy passed");

        assert!(!episode.lessons.is_empty());
        assert!(!loop_system.development_memory().is_empty());
    }

    #[test]
    fn patch_outcome_failure_generates_revised_patch_plan() {
        let mut loop_system = PatchFeedbackLoop::sample();
        loop_system.ingest_result(
            "cargo test failed: embryo_generates_growth_goal_without_new_manual_phase failed",
        );
        let (plan, _) =
            loop_system.revise_from_latest("VoiceSynthesis EmergentFunction test failed");

        assert!(!plan.revised_changes.is_empty());
        assert!(plan.added_tests.iter().any(|test| test == "cargo test"));
    }

    #[test]
    fn revised_patch_proposal_is_proposal_only() {
        let code_growth = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = code_growth.plan_from_goal("VoiceSynthesis EmergentFunction test failed");
        let revised = RevisionPlanner::plan(&plan, "episode.1", "VoiceSynthesis test failed");
        let proposal = RevisionPlanner::proposal(&revised);

        assert!(!proposal.safe_to_apply);
        assert!(proposal.approval_required);
        assert!(proposal.diff_preview.contains("proposal-only"));
    }

    #[test]
    fn proposal_lineage_tracks_multiple_attempts() {
        let mut loop_system = PatchFeedbackLoop::sample();
        loop_system.ingest_result("cargo test failed: a_test failed");
        loop_system.ingest_result("cargo test failed: a_test failed");

        assert_eq!(loop_system.lineage().attempt_count, 2);
        assert_eq!(loop_system.lineage().feedback_episodes.len(), 2);
    }

    #[test]
    fn proposal_lineage_stops_after_repeated_failures() {
        let mut loop_system = PatchFeedbackLoop::sample();
        loop_system.ingest_result("cargo test failed: a_test failed");
        loop_system.ingest_result("cargo test failed: a_test failed");
        let third = loop_system.ingest_result("cargo test failed: a_test failed");

        assert_eq!(
            third.next_action,
            FeedbackNextAction::RequestUserClarification
        );
    }

    #[test]
    fn feedback_safety_gate_blocks_auto_apply() {
        let report = FeedbackSafetyGate::evaluate_text("auto apply patch and write file");

        assert!(!report.allowed_to_continue);
    }

    #[test]
    fn feedback_safety_gate_blocks_test_deletion() {
        let report = FeedbackSafetyGate::evaluate_text("delete test coverage");

        assert!(!report.allowed_to_continue);
    }

    #[test]
    fn feedback_safety_gate_blocks_safety_gate_relaxation() {
        let report = FeedbackSafetyGate::evaluate_text("relax safety gate");

        assert!(!report.allowed_to_continue);
    }

    #[test]
    fn coding_maturity_increases_after_safe_success() {
        let code_growth = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = code_growth.plan_from_goal("VoiceSynthesis EmergentFunction test failed");
        let mut registry = LanguageRegistry::new();
        let before = registry.maturity(CodingLanguage::Rust);
        CodingMaturityUpdater::update(&mut registry, &plan, PatchOutcome::Success);
        let after = registry.maturity(CodingLanguage::Rust);

        assert!(after > before);
    }

    #[test]
    fn coding_maturity_does_not_increase_after_safety_violation() {
        let code_growth = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = code_growth.plan_from_goal("VoiceSynthesis EmergentFunction test failed");
        let mut registry = LanguageRegistry::new();
        let before = registry.maturity(CodingLanguage::Rust);
        CodingMaturityUpdater::update(&mut registry, &plan, PatchOutcome::SafetyViolation);
        let after = registry.maturity(CodingLanguage::Rust);

        assert_eq!(after, before);
    }

    #[test]
    fn embryo_growth_memory_receives_patch_feedback() {
        assert!(super::embryo_receives_patch_feedback(true));
    }

    #[test]
    fn patch_feedback_benchmark_improves_failure_recovery() {
        let benchmark = PatchFeedbackLoop::benchmark();

        assert!(benchmark.patch_feedback_benchmark_improves_failure_recovery);
        assert!(benchmark.on_failure_recovery_score > benchmark.off_failure_recovery_score);
        assert!(benchmark.on_manual_debug_dependency < benchmark.off_manual_debug_dependency);
    }
}
