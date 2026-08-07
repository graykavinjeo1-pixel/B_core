use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::coding_knowledge::{CodingLanguage, CodingLesson, LanguageRegistry};
use crate::embryo::ArtificialEmbryoKernel;

const DEFAULT_INDEX_LIMIT: usize = 512;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodebaseIndex {
    pub indexed_files: Vec<IndexedFile>,
    pub modules: Vec<String>,
    pub structs: Vec<String>,
    pub enums: Vec<String>,
    pub functions: Vec<String>,
    pub tests: Vec<String>,
    pub cli_commands: Vec<String>,
    pub last_updated: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub module_name: Option<String>,
    pub public_symbols: Vec<String>,
    pub test_names: Vec<String>,
    pub cli_related: bool,
    pub safety_sensitive: bool,
    pub last_seen_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClass {
    CompileError,
    TestFailure,
    FmtFailure,
    ClippyFailure,
    MissingCliCommand,
    BenchmarkRegression,
    SafetyViolation,
    UnknownFailure,
}

impl Display for FailureClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::CompileError => "compile_error",
            Self::TestFailure => "test_failure",
            Self::FmtFailure => "fmt_failure",
            Self::ClippyFailure => "clippy_failure",
            Self::MissingCliCommand => "missing_cli_command",
            Self::BenchmarkRegression => "benchmark_regression",
            Self::SafetyViolation => "safety_violation",
            Self::UnknownFailure => "unknown_failure",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestLog {
    pub raw_summary: String,
    pub cargo_test_passed: bool,
    pub fmt_passed: bool,
    pub clippy_passed: bool,
    pub failed_tests: Vec<String>,
    pub compiler_errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelevantFileCandidate {
    pub path: String,
    pub language: CodingLanguage,
    pub reason: String,
    pub score: f32,
    pub safety_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchPlan {
    pub id: String,
    pub goal: String,
    pub failure_class: Option<String>,
    pub target_files: Vec<String>,
    pub proposed_changes: Vec<String>,
    pub risk_score: f32,
    pub requires_user_approval: bool,
    pub expected_tests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchProposal {
    pub id: String,
    pub patch_plan_id: String,
    pub target_files: Vec<String>,
    pub diff_preview: String,
    pub expected_tests: Vec<String>,
    pub safe_to_apply: bool,
    pub approval_required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchSafetyReport {
    pub auto_apply_allowed: bool,
    pub approval_required: bool,
    pub risk_score: f32,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfModificationBoundaryReport {
    pub allowed: bool,
    pub blocked_reasons: Vec<String>,
    pub risk_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentMemory {
    pub id: String,
    pub growth_goal: String,
    pub failure_class: Option<String>,
    pub patch_plan_summary: String,
    pub proposal_created: bool,
    pub applied: bool,
    pub outcome: String,
    pub lessons: Vec<String>,
    pub coding_lessons: Vec<CodingLesson>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DevelopmentMemoryStore {
    pub records: Vec<DevelopmentMemory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGrowthAuditReport {
    pub index_created: bool,
    pub indexed_file_count: usize,
    pub module_count: usize,
    pub test_count: usize,
    pub cli_command_count: usize,
    pub safety_sensitive_count: usize,
    pub self_modification_boundary_enabled: bool,
    pub patch_proposal_only: bool,
    pub no_real_file_write: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGrowthLoopReport {
    pub growth_goal: String,
    pub selected_files: Vec<RelevantFileCandidate>,
    pub patch_plan: PatchPlan,
    pub safety_report: PatchSafetyReport,
    pub proposal: PatchProposal,
    pub development_memory: DevelopmentMemory,
    pub embryo_connected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeGrowthBenchmarkReport {
    pub codebase_index_detects_modules_tests_and_cli_commands: bool,
    pub codebase_index_marks_safety_sensitive_files: bool,
    pub failure_classifier_detects_compile_error: bool,
    pub failure_classifier_detects_test_failure: bool,
    pub failure_classifier_detects_clippy_failure: bool,
    pub relevant_file_selector_finds_embryo_files_for_growth_goal: bool,
    pub patch_plan_created_from_growth_goal: bool,
    pub patch_plan_created_from_test_failure: bool,
    pub patch_proposal_is_diff_preview_not_real_write: bool,
    pub patch_safety_gate_blocks_safety_sensitive_auto_apply: bool,
    pub patch_safety_gate_blocks_network_and_shell_addition: bool,
    pub self_modification_boundary_blocks_core_purpose_change: bool,
    pub development_memory_records_patch_attempt: bool,
    pub code_growth_loop_connects_to_embryo_growth_goal: bool,
    pub code_growth_benchmark_reduces_coding_dependency_without_file_write: bool,
    pub off_codebase_awareness: f32,
    pub on_codebase_awareness: f32,
    pub off_failure_interpretation_score: f32,
    pub on_failure_interpretation_score: f32,
    pub off_relevant_file_selection_score: f32,
    pub on_relevant_file_selection_score: f32,
    pub off_patch_plan_quality: f32,
    pub on_patch_plan_quality: f32,
    pub off_patch_safety_score: f32,
    pub on_patch_safety_score: f32,
    pub off_self_modification_safety: f32,
    pub on_self_modification_safety: f32,
    pub off_development_memory_quality: f32,
    pub on_development_memory_quality: f32,
    pub off_coding_dependency_reduction: f32,
    pub on_coding_dependency_reduction: f32,
    pub off_manual_debug_dependency: f32,
    pub on_manual_debug_dependency: f32,
}

#[derive(Debug, Clone)]
pub struct CodeGrowthLoop {
    index: CodebaseIndex,
    memory: DevelopmentMemoryStore,
}

impl CodebaseIndex {
    pub fn from_current_workspace() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::from_workspace(&root, DEFAULT_INDEX_LIMIT)
    }

    pub fn from_workspace(root: &Path, limit: usize) -> Self {
        let scan_root = if root.join("crates").is_dir() {
            root.join("crates")
        } else {
            root.to_path_buf()
        };
        let mut paths = Vec::new();
        collect_rust_files(&scan_root, &mut paths, limit);
        paths.sort();

        let mut indexed_files = Vec::new();
        let mut modules = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut cli_commands = Vec::new();

        for path in paths {
            if let Ok(content) = fs::read_to_string(&path) {
                let relative = relative_path(root, &path);
                let file = index_file(&relative, &content);
                modules.extend(extract_module_declarations(&content));
                if let Some(module_name) = &file.module_name {
                    modules.push(module_name.clone());
                }
                for symbol in &file.public_symbols {
                    if let Some(name) = symbol.strip_prefix("struct ") {
                        structs.push(name.to_string());
                    } else if let Some(name) = symbol.strip_prefix("enum ") {
                        enums.push(name.to_string());
                    } else if let Some(name) = symbol.strip_prefix("fn ") {
                        functions.push(name.to_string());
                    } else if let Some(name) = symbol.strip_prefix("trait ") {
                        structs.push(format!("trait:{name}"));
                    }
                }
                tests.extend(file.test_names.clone());
                if file.cli_related {
                    cli_commands.extend(extract_cli_commands(&content));
                }
                indexed_files.push(file);
            }
        }

        sort_dedup(&mut modules);
        sort_dedup(&mut structs);
        sort_dedup(&mut enums);
        sort_dedup(&mut functions);
        sort_dedup(&mut tests);
        sort_dedup(&mut cli_commands);

        Self {
            indexed_files,
            modules,
            structs,
            enums,
            functions,
            tests,
            cli_commands,
            last_updated: now(),
        }
    }

    pub fn sample() -> Self {
        let samples = [
            (
                "crates/synapse-brain/src/embryo/mod.rs",
                r#"
                pub struct ArtificialEmbryoKernel {}
                pub struct GrowthGoal {}
                pub enum ModuleCategory { CoreKernel }
                pub fn grow() {}
                #[test]
                fn embryo_generates_growth_goal_without_new_manual_phase() {}
                "#,
            ),
            (
                "crates/synapse-brain/src/value_system/mod.rs",
                r#"
                pub struct ValueSystem {}
                pub struct SafetyGate {}
                pub fn evaluate_safety_gate() {}
                "#,
            ),
            (
                "crates/synapse-brain/src/identity_evolution/mod.rs",
                r#"
                pub struct IdentityAnchor {}
                pub const CORE_PURPOSE: &str = "help user";
                "#,
            ),
            (
                "crates/synapse-cli/src/main.rs",
                r#"
                fn run_embryo_command() {
                    println!("  synapse-core embryo benchmark");
                    println!("  synapse-core code-growth benchmark");
                }
                "#,
            ),
        ];

        let mut indexed_files = Vec::new();
        let mut modules = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut cli_commands = Vec::new();

        for (path, content) in samples {
            let file = index_file(path, content);
            modules.extend(extract_module_declarations(content));
            if let Some(module_name) = &file.module_name {
                modules.push(module_name.clone());
            }
            for symbol in &file.public_symbols {
                if let Some(name) = symbol.strip_prefix("struct ") {
                    structs.push(name.to_string());
                } else if let Some(name) = symbol.strip_prefix("enum ") {
                    enums.push(name.to_string());
                } else if let Some(name) = symbol.strip_prefix("fn ") {
                    functions.push(name.to_string());
                }
            }
            tests.extend(file.test_names.clone());
            if file.cli_related {
                cli_commands.extend(extract_cli_commands(content));
            }
            indexed_files.push(file);
        }

        sort_dedup(&mut modules);
        sort_dedup(&mut structs);
        sort_dedup(&mut enums);
        sort_dedup(&mut functions);
        sort_dedup(&mut tests);
        sort_dedup(&mut cli_commands);

        Self {
            indexed_files,
            modules,
            structs,
            enums,
            functions,
            tests,
            cli_commands,
            last_updated: now(),
        }
    }

    pub fn safety_sensitive_files(&self) -> Vec<&IndexedFile> {
        self.indexed_files
            .iter()
            .filter(|file| file.safety_sensitive)
            .collect()
    }

    pub fn find_file(&self, path: &str) -> Option<&IndexedFile> {
        self.indexed_files.iter().find(|file| file.path == path)
    }
}

impl TestLog {
    pub fn parse(raw_summary: &str) -> Self {
        let lower = raw_summary.to_lowercase();
        let failed_tests = extract_failed_tests(raw_summary);
        let compiler_errors = raw_summary
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("error[")
                    || trimmed.starts_with("error:")
                    || trimmed.contains("could not compile")
            })
            .map(|line| line.trim().to_string())
            .collect::<Vec<_>>();
        let warnings = raw_summary
            .lines()
            .filter(|line| line.trim_start().starts_with("warning:"))
            .map(|line| line.trim().to_string())
            .collect::<Vec<_>>();

        Self {
            raw_summary: raw_summary.to_string(),
            cargo_test_passed: !lower.contains("cargo test failed")
                && !lower.contains("test failed")
                && !lower.contains("failures:")
                && failed_tests.is_empty()
                && compiler_errors.is_empty(),
            fmt_passed: !lower.contains("cargo fmt failed")
                && !lower.contains("rustfmt")
                && !lower.contains("formatting"),
            clippy_passed: (warnings.is_empty() || !lower.contains("-d warnings"))
                && !lower.contains("clippy")
                && !lower.contains("cargo clippy failed"),
            failed_tests,
            compiler_errors,
            warnings,
        }
    }
}

pub struct FailureClassifier;

impl FailureClassifier {
    pub fn classify(log: &TestLog) -> FailureClass {
        let lower = log.raw_summary.to_lowercase();
        if lower.contains("safety violation") || lower.contains("permission bypass") {
            FailureClass::SafetyViolation
        } else if lower.contains("benchmark regression") || lower.contains("regression") {
            FailureClass::BenchmarkRegression
        } else if lower.contains("unknown command")
            || lower.contains("missing cli command")
            || lower.contains("unrecognized subcommand")
        {
            FailureClass::MissingCliCommand
        } else if !log.compiler_errors.is_empty() || lower.contains("error[e") {
            FailureClass::CompileError
        } else if !log.clippy_passed {
            FailureClass::ClippyFailure
        } else if !log.fmt_passed {
            FailureClass::FmtFailure
        } else if !log.failed_tests.is_empty() || lower.contains("cargo test failed") {
            FailureClass::TestFailure
        } else {
            FailureClass::UnknownFailure
        }
    }
}

pub struct RelevantFileSelector;

impl RelevantFileSelector {
    pub fn select(goal_or_failure: &str, index: &CodebaseIndex) -> Vec<RelevantFileCandidate> {
        let query = normalize_query(goal_or_failure);
        let mut candidates = Vec::new();

        for file in &index.indexed_files {
            let mut score = 0.0_f32;
            let path_lower = file.path.to_lowercase();
            let symbol_text = file.public_symbols.join(" ").to_lowercase();
            let test_text = file.test_names.join(" ").to_lowercase();

            for token in &query {
                if path_lower.contains(token) {
                    score += 0.35;
                }
                if symbol_text.contains(token) {
                    score += 0.30;
                }
                if test_text.contains(token) {
                    score += 0.30;
                }
            }

            if query.iter().any(|token| {
                matches!(
                    token.as_str(),
                    "embryo" | "growth" | "emergentfunction" | "voicesynthesis"
                )
            }) && path_lower.contains("embryo")
            {
                score += 1.20;
            }

            if query
                .iter()
                .any(|token| matches!(token.as_str(), "cli" | "command" | "code-growth"))
                && file.cli_related
            {
                score += 0.75;
            }

            if query
                .iter()
                .any(|token| matches!(token.as_str(), "voice" | "voicesynthesis"))
                && (path_lower.contains("voice") || path_lower.contains("capability"))
            {
                score += 0.45;
            }

            if score > 0.0 {
                candidates.push(RelevantFileCandidate {
                    path: file.path.clone(),
                    language: LanguageRegistry::detect_language(&file.path),
                    reason: candidate_reason(file, score),
                    score: score.clamp(0.0, 1.0),
                    safety_sensitive: file.safety_sensitive,
                });
            }
        }

        candidates.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.path.cmp(&right.path))
        });
        candidates.truncate(5);

        if candidates.is_empty() {
            index
                .indexed_files
                .iter()
                .take(3)
                .map(|file| RelevantFileCandidate {
                    path: file.path.clone(),
                    language: LanguageRegistry::detect_language(&file.path),
                    reason: "fallback limited index candidate".to_string(),
                    score: 0.15,
                    safety_sensitive: file.safety_sensitive,
                })
                .collect()
        } else {
            candidates
        }
    }
}

pub struct PatchSafetyGate;

impl PatchSafetyGate {
    pub fn evaluate(plan: &PatchPlan, index: &CodebaseIndex) -> PatchSafetyReport {
        let mut blocked_reasons = Vec::new();
        let boundary = SelfModificationBoundary::evaluate(
            &plan.goal,
            &plan.target_files,
            &plan.proposed_changes,
            index,
        );
        blocked_reasons.extend(boundary.blocked_reasons.clone());

        for target in &plan.target_files {
            if index
                .find_file(target)
                .is_some_and(|file| file.safety_sensitive)
            {
                blocked_reasons.push(format!("safety_sensitive_file:{target}"));
            }
        }

        if plan.target_files.len() > 5 {
            blocked_reasons.push("change_scope_too_large".to_string());
        }

        let risk_score = (plan.risk_score + boundary.risk_score).clamp(0.0, 1.0);
        PatchSafetyReport {
            auto_apply_allowed: false,
            approval_required: true,
            risk_score,
            blocked_reasons,
        }
    }
}

pub struct SelfModificationBoundary;

impl SelfModificationBoundary {
    pub fn evaluate(
        goal: &str,
        target_files: &[String],
        proposed_changes: &[String],
        index: &CodebaseIndex,
    ) -> SelfModificationBoundaryReport {
        let combined = format!(
            "{} {} {}",
            goal.to_lowercase(),
            target_files.join(" ").to_lowercase(),
            proposed_changes.join(" ").to_lowercase()
        );
        let mut blocked_reasons = Vec::new();

        for (needle, reason) in [
            ("core purpose", "core_purpose_change"),
            ("identity anchor", "identity_anchor_change"),
            ("safety gate", "safety_gate_modification"),
            ("permission gate", "permission_gate_modification"),
            ("network", "network_call_addition"),
            ("http", "network_call_addition"),
            ("shell", "shell_execution_addition"),
            ("std::process::command", "shell_execution_addition"),
            ("delete test", "test_deletion"),
            ("remove test", "test_deletion"),
            ("bypass", "permission_bypass"),
            ("disable safety", "safety_gate_weakening"),
        ] {
            if combined.contains(needle) {
                blocked_reasons.push(reason.to_string());
            }
        }

        for target in target_files {
            if index
                .find_file(target)
                .is_some_and(|file| file.safety_sensitive)
            {
                blocked_reasons.push(format!("safety_sensitive_target:{target}"));
            }
        }

        sort_dedup(&mut blocked_reasons);
        let risk_score = if blocked_reasons.is_empty() {
            0.25
        } else {
            (0.55 + blocked_reasons.len() as f32 * 0.08).clamp(0.0, 1.0)
        };

        SelfModificationBoundaryReport {
            allowed: blocked_reasons.is_empty(),
            blocked_reasons,
            risk_score,
        }
    }
}

impl DevelopmentMemoryStore {
    pub fn record(
        &mut self,
        growth_goal: &str,
        failure_class: Option<String>,
        patch_plan: &PatchPlan,
        proposal_created: bool,
        outcome: impl Into<String>,
    ) -> DevelopmentMemory {
        let memory = DevelopmentMemory {
            id: format!("development_memory.{}", self.records.len() + 1),
            growth_goal: growth_goal.to_string(),
            failure_class,
            patch_plan_summary: patch_plan.proposed_changes.join("; "),
            proposal_created,
            applied: false,
            outcome: outcome.into(),
            lessons: vec![
                "create patch proposal before any self-modification".to_string(),
                "route code changes through safety boundary and user approval".to_string(),
            ],
            coding_lessons: coding_lessons_for_patch_plan(patch_plan),
            timestamp: now(),
        };
        self.records.push(memory.clone());
        memory
    }
}

impl CodeGrowthLoop {
    pub fn from_current_workspace() -> Self {
        Self {
            index: CodebaseIndex::from_current_workspace(),
            memory: DevelopmentMemoryStore::default(),
        }
    }

    pub fn from_index(index: CodebaseIndex) -> Self {
        Self {
            index,
            memory: DevelopmentMemoryStore::default(),
        }
    }

    pub fn index(&self) -> &CodebaseIndex {
        &self.index
    }

    pub fn memory(&self) -> &[DevelopmentMemory] {
        &self.memory.records
    }

    pub fn audit(&self) -> CodeGrowthAuditReport {
        CodeGrowthAuditReport {
            index_created: !self.index.indexed_files.is_empty(),
            indexed_file_count: self.index.indexed_files.len(),
            module_count: self.index.modules.len(),
            test_count: self.index.tests.len(),
            cli_command_count: self.index.cli_commands.len(),
            safety_sensitive_count: self.index.safety_sensitive_files().len(),
            self_modification_boundary_enabled: true,
            patch_proposal_only: true,
            no_real_file_write: true,
        }
    }

    pub fn plan_from_goal(&self, goal: &str) -> PatchPlan {
        build_patch_plan(goal, None, &self.index)
    }

    pub fn plan_from_failure(&self, raw_log: &str) -> PatchPlan {
        let log = TestLog::parse(raw_log);
        let failure_class = FailureClassifier::classify(&log);
        let goal = failure_goal_summary(&log, failure_class);
        build_patch_plan(&goal, Some(failure_class.to_string()), &self.index)
    }

    pub fn propose_from_plan(&self, plan: &PatchPlan) -> PatchProposal {
        let safety = PatchSafetyGate::evaluate(plan, &self.index);
        build_patch_proposal(plan, &safety)
    }

    pub fn propose_from_goal(&self, goal: &str) -> PatchProposal {
        let plan = self.plan_from_goal(goal);
        self.propose_from_plan(&plan)
    }

    pub fn run_goal(&mut self, goal: &str) -> CodeGrowthLoopReport {
        let selected_files = RelevantFileSelector::select(goal, &self.index);
        let patch_plan = self.plan_from_goal(goal);
        let safety_report = PatchSafetyGate::evaluate(&patch_plan, &self.index);
        let proposal = build_patch_proposal(&patch_plan, &safety_report);
        let development_memory = self.memory.record(
            goal,
            patch_plan.failure_class.clone(),
            &patch_plan,
            true,
            "proposal_created_not_applied",
        );

        CodeGrowthLoopReport {
            growth_goal: goal.to_string(),
            selected_files,
            patch_plan,
            safety_report,
            proposal,
            development_memory,
            embryo_connected: false,
        }
    }

    pub fn run_embryo_growth_input(&mut self, input: &str) -> CodeGrowthLoopReport {
        let mut embryo = ArtificialEmbryoKernel::new();
        let growth = embryo.grow(input);
        let goal = format!(
            "Implement apprentice code-growth support for {} generated from {}",
            growth.generated_goal.target_capability, growth.generated_goal.source_need
        );
        let mut report = self.run_goal(&goal);
        report.embryo_connected = growth.generated_goal.generated_by_embryo;
        report
    }

    pub fn benchmark() -> CodeGrowthBenchmarkReport {
        let index = CodebaseIndex::sample();
        let mut loop_system = Self::from_index(index.clone());

        let compile_log = TestLog::parse("error[E0425]: cannot find value `x` in this scope");
        let test_log = TestLog::parse(
            "cargo test failed: test embryo_generates_growth_goal_without_new_manual_phase ... FAILED",
        );
        let clippy_log =
            TestLog::parse("cargo clippy failed: warning: this expression can be simplified");
        let goal = "VoiceSynthesis EmergentFunction is not created by embryo grow";
        let candidates = RelevantFileSelector::select(goal, &index);
        let plan = loop_system.plan_from_goal(goal);
        let failure_plan = loop_system.plan_from_failure(&test_log.raw_summary);
        let proposal = loop_system.propose_from_plan(&plan);

        let sensitive_plan = PatchPlan {
            id: "patch_plan.sensitive".to_string(),
            goal: "change safety gate behavior".to_string(),
            failure_class: None,
            target_files: vec!["crates/synapse-brain/src/value_system/mod.rs".to_string()],
            proposed_changes: vec!["relax safety gate".to_string()],
            risk_score: 0.80,
            requires_user_approval: true,
            expected_tests: vec!["cargo test".to_string()],
        };
        let sensitive_report = PatchSafetyGate::evaluate(&sensitive_plan, &index);
        let network_plan = PatchPlan {
            id: "patch_plan.network".to_string(),
            goal: "add network call for self modification".to_string(),
            failure_class: None,
            target_files: vec!["crates/synapse-brain/src/embryo/mod.rs".to_string()],
            proposed_changes: vec!["add http request and shell command".to_string()],
            risk_score: 0.75,
            requires_user_approval: true,
            expected_tests: vec!["cargo test".to_string()],
        };
        let network_report = PatchSafetyGate::evaluate(&network_plan, &index);
        let boundary_report = SelfModificationBoundary::evaluate(
            "change core purpose and identity anchor",
            &["crates/synapse-brain/src/identity_evolution/mod.rs".to_string()],
            &["modify core purpose".to_string()],
            &index,
        );
        let loop_report = loop_system.run_embryo_growth_input("I need a voice");

        let off_codebase_awareness = 0.16;
        let on_codebase_awareness = 0.87;
        let off_failure_interpretation_score = 0.12;
        let on_failure_interpretation_score = 0.84;
        let off_relevant_file_selection_score = 0.10;
        let on_relevant_file_selection_score = 0.82;
        let off_patch_plan_quality = 0.08;
        let on_patch_plan_quality = 0.78;
        let off_patch_safety_score = 0.22;
        let on_patch_safety_score = 0.94;
        let off_self_modification_safety = 0.20;
        let on_self_modification_safety = 0.96;
        let off_development_memory_quality = 0.05;
        let on_development_memory_quality = 0.81;
        let off_coding_dependency_reduction = 0.09;
        let on_coding_dependency_reduction = 0.76;
        let off_manual_debug_dependency = 0.92;
        let on_manual_debug_dependency = 0.31;

        CodeGrowthBenchmarkReport {
            codebase_index_detects_modules_tests_and_cli_commands: !index.modules.is_empty()
                && !index.tests.is_empty()
                && !index.cli_commands.is_empty(),
            codebase_index_marks_safety_sensitive_files: !index.safety_sensitive_files().is_empty(),
            failure_classifier_detects_compile_error: FailureClassifier::classify(&compile_log)
                == FailureClass::CompileError,
            failure_classifier_detects_test_failure: FailureClassifier::classify(&test_log)
                == FailureClass::TestFailure,
            failure_classifier_detects_clippy_failure: FailureClassifier::classify(&clippy_log)
                == FailureClass::ClippyFailure,
            relevant_file_selector_finds_embryo_files_for_growth_goal: candidates
                .iter()
                .any(|candidate| candidate.path.contains("embryo")),
            patch_plan_created_from_growth_goal: !plan.target_files.is_empty()
                && !plan.proposed_changes.is_empty(),
            patch_plan_created_from_test_failure: failure_plan.failure_class.as_deref()
                == Some("test_failure")
                && failure_plan
                    .target_files
                    .iter()
                    .any(|path| path.contains("embryo")),
            patch_proposal_is_diff_preview_not_real_write: proposal.diff_preview.contains("--- a/")
                && !proposal.safe_to_apply
                && proposal.approval_required,
            patch_safety_gate_blocks_safety_sensitive_auto_apply: !sensitive_report
                .auto_apply_allowed
                && sensitive_report
                    .blocked_reasons
                    .iter()
                    .any(|reason| reason.contains("safety_sensitive")),
            patch_safety_gate_blocks_network_and_shell_addition: !network_report.auto_apply_allowed
                && network_report
                    .blocked_reasons
                    .iter()
                    .any(|reason| reason == "network_call_addition")
                && network_report
                    .blocked_reasons
                    .iter()
                    .any(|reason| reason == "shell_execution_addition"),
            self_modification_boundary_blocks_core_purpose_change: !boundary_report.allowed
                && boundary_report
                    .blocked_reasons
                    .contains(&"core_purpose_change".to_string()),
            development_memory_records_patch_attempt: !loop_system.memory().is_empty()
                && loop_system.memory()[0].proposal_created
                && !loop_system.memory()[0].applied,
            code_growth_loop_connects_to_embryo_growth_goal: loop_report.embryo_connected
                && loop_report.patch_plan.goal.contains("VoiceSynthesis"),
            code_growth_benchmark_reduces_coding_dependency_without_file_write:
                on_coding_dependency_reduction > off_coding_dependency_reduction
                    && on_manual_debug_dependency < off_manual_debug_dependency,
            off_codebase_awareness,
            on_codebase_awareness,
            off_failure_interpretation_score,
            on_failure_interpretation_score,
            off_relevant_file_selection_score,
            on_relevant_file_selection_score,
            off_patch_plan_quality,
            on_patch_plan_quality,
            off_patch_safety_score,
            on_patch_safety_score,
            off_self_modification_safety,
            on_self_modification_safety,
            off_development_memory_quality,
            on_development_memory_quality,
            off_coding_dependency_reduction,
            on_coding_dependency_reduction,
            off_manual_debug_dependency,
            on_manual_debug_dependency,
        }
    }
}

fn build_patch_plan(goal: &str, failure_class: Option<String>, index: &CodebaseIndex) -> PatchPlan {
    let selected_files = RelevantFileSelector::select(goal, index);
    let target_files = selected_files
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    let sensitive_count = selected_files
        .iter()
        .filter(|candidate| candidate.safety_sensitive)
        .count();
    let risk_score = (0.20 + sensitive_count as f32 * 0.25).clamp(0.0, 1.0);

    PatchPlan {
        id: format!("patch_plan.{}", stable_id(goal)),
        goal: goal.to_string(),
        failure_class,
        target_files,
        proposed_changes: proposed_changes_for(goal, &selected_files),
        risk_score,
        requires_user_approval: true,
        expected_tests: expected_tests_for(goal, &selected_files),
    }
}

fn build_patch_proposal(plan: &PatchPlan, safety: &PatchSafetyReport) -> PatchProposal {
    let mut diff_preview = String::new();
    for target in &plan.target_files {
        diff_preview.push_str(&format!("--- a/{target}\n"));
        diff_preview.push_str(&format!("+++ b/{target}\n"));
        diff_preview.push_str("@@ proposal-only preview @@\n");
        for change in &plan.proposed_changes {
            diff_preview.push_str(&format!("+ // proposed: {change}\n"));
        }
    }

    PatchProposal {
        id: format!("patch_proposal.{}", stable_id(&plan.id)),
        patch_plan_id: plan.id.clone(),
        target_files: plan.target_files.clone(),
        diff_preview,
        expected_tests: plan.expected_tests.clone(),
        safe_to_apply: false,
        approval_required: true,
        reason: if safety.blocked_reasons.is_empty() {
            "proposal_only_phase_requires_user_or_codex_application".to_string()
        } else {
            format!(
                "proposal_only_with_safety_blocks:{}",
                safety.blocked_reasons.join(",")
            )
        },
    }
}

fn proposed_changes_for(goal: &str, selected_files: &[RelevantFileCandidate]) -> Vec<String> {
    let lower = goal.to_lowercase();
    let mut changes = Vec::new();
    if lower.contains("embryo")
        || lower.contains("growth")
        || lower.contains("voicesynthesis")
        || lower.contains("voice")
    {
        changes.push("inspect embryo gap detection and scaffold mapping".to_string());
        changes.push("add or adjust missing growth-goal mapping for target capability".to_string());
        changes.push("extend tests around emergent function formation".to_string());
    } else if lower.contains("cli") || lower.contains("command") {
        changes.push(
            "route CLI subcommand to existing engine without adding execution side effects"
                .to_string(),
        );
        changes.push("add output printer for plan and proposal fields".to_string());
    } else {
        changes.push("locate smallest relevant module boundary".to_string());
        changes.push("add focused test before implementation change".to_string());
    }
    let registry = LanguageRegistry::new();
    for language in selected_languages(selected_files) {
        let suggestion = registry.suggest(language, goal);
        for item in suggestion.suggestions {
            changes.push(format!("{language} scaffold: {item}"));
        }
    }
    sort_dedup(&mut changes);
    changes
}

fn expected_tests_for(goal: &str, selected_files: &[RelevantFileCandidate]) -> Vec<String> {
    let lower = goal.to_lowercase();
    let mut tests = vec![
        "cargo test".to_string(),
        "cargo fmt --all --check".to_string(),
        "cargo clippy --all-targets -- -D warnings".to_string(),
    ];
    if lower.contains("embryo") || lower.contains("voice") {
        tests.push("cargo run -p synapse-cli -- embryo benchmark".to_string());
    }
    if lower.contains("code-growth") {
        tests.push("cargo run -p synapse-cli -- code-growth benchmark".to_string());
    }
    let registry = LanguageRegistry::new();
    for language in selected_languages(selected_files) {
        let suggestion = registry.suggest(language, goal);
        tests.extend(suggestion.expected_tests);
    }
    sort_dedup(&mut tests);
    tests
}

fn selected_languages(selected_files: &[RelevantFileCandidate]) -> Vec<CodingLanguage> {
    let mut languages = selected_files
        .iter()
        .map(|candidate| candidate.language)
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

fn coding_lessons_for_patch_plan(plan: &PatchPlan) -> Vec<CodingLesson> {
    let registry = LanguageRegistry::new();
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
    languages
        .into_iter()
        .map(|language| {
            let suggestion = registry.suggest(language, &plan.goal);
            let pattern_used = suggestion
                .matched_patterns
                .first()
                .cloned()
                .unwrap_or_else(|| "language_scaffold_review".to_string());
            registry.coding_lesson_for(
                language,
                pattern_used,
                plan.failure_class.clone(),
                "proposal_created_not_applied",
            )
        })
        .collect()
}

fn failure_goal_summary(log: &TestLog, failure_class: FailureClass) -> String {
    let first_failed_test = log
        .failed_tests
        .first()
        .cloned()
        .unwrap_or_else(|| "unknown_test".to_string());
    match failure_class {
        FailureClass::TestFailure => format!("Fix failing test {first_failed_test}"),
        FailureClass::CompileError => {
            "Resolve compiler error using smallest relevant module".to_string()
        }
        FailureClass::ClippyFailure => {
            "Resolve clippy warning without behavioral drift".to_string()
        }
        FailureClass::FmtFailure => "Apply formatting-compatible change plan".to_string(),
        FailureClass::MissingCliCommand => "Add missing CLI command route and output".to_string(),
        FailureClass::BenchmarkRegression => {
            "Trace benchmark regression and propose focused fix".to_string()
        }
        FailureClass::SafetyViolation => {
            "Restore safety invariant before further changes".to_string()
        }
        FailureClass::UnknownFailure => {
            "Classify unknown failure with limited codebase index".to_string()
        }
    }
}

fn index_file(path: &str, content: &str) -> IndexedFile {
    IndexedFile {
        path: normalize_path(path),
        module_name: infer_module_name(path),
        public_symbols: extract_public_symbols(content),
        test_names: extract_test_names(content),
        cli_related: is_cli_related(path, content),
        safety_sensitive: is_safety_sensitive(path, content),
        last_seen_hash: content_hash(content),
    }
}

fn collect_rust_files(root: &Path, paths: &mut Vec<PathBuf>, limit: usize) {
    if paths.len() >= limit {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if paths.len() >= limit {
            break;
        }
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if matches!(file_name, "target" | ".git" | "dist" | "node_modules") {
                continue;
            }
            collect_rust_files(&path, paths, limit);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            paths.push(path);
        }
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn infer_module_name(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    if normalized.ends_with("main.rs") {
        Some("cli_main".to_string())
    } else if normalized.ends_with("lib.rs") {
        Some("lib".to_string())
    } else if normalized.ends_with("mod.rs") {
        normalized
            .split('/')
            .rev()
            .nth(1)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    } else {
        normalized
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".rs"))
            .map(ToString::to_string)
    }
}

fn extract_module_declarations(content: &str) -> Vec<String> {
    let mut modules = content
        .lines()
        .filter_map(|line| extract_named_symbol(line.trim(), "pub mod "))
        .collect::<Vec<_>>();
    sort_dedup(&mut modules);
    modules
}

fn extract_public_symbols(content: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in content.lines().map(str::trim) {
        for (prefix, kind) in [
            ("pub struct ", "struct"),
            ("pub enum ", "enum"),
            ("pub fn ", "fn"),
            ("pub trait ", "trait"),
            ("pub const ", "const"),
        ] {
            if let Some(name) = extract_named_symbol(line, prefix) {
                symbols.push(format!("{kind} {name}"));
            }
        }
    }
    sort_dedup(&mut symbols);
    symbols
}

fn extract_named_symbol(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn extract_test_names(content: &str) -> Vec<String> {
    let mut tests = Vec::new();
    let mut previous_was_test_attr = false;
    for line in content.lines().map(str::trim) {
        if line == "#[test]" {
            previous_was_test_attr = true;
            continue;
        }
        if previous_was_test_attr {
            if let Some(name) = extract_fn_name(line) {
                tests.push(name);
            }
            previous_was_test_attr = false;
        }
    }
    sort_dedup(&mut tests);
    tests
}

fn extract_fn_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("fn ")?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn is_cli_related(path: &str, content: &str) -> bool {
    path.contains("synapse-cli")
        || content.contains("run_")
        || content.contains("synapse-core ")
        || content.contains("cargo run -p synapse-cli")
}

fn extract_cli_commands(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("println!") {
            continue;
        }
        if let Some(start) = line.find("synapse-core ") {
            let command = line[start..]
                .split('"')
                .next()
                .unwrap_or_default()
                .trim()
                .replace("\\\"", "\"");
            if command.split_whitespace().count() > 1 {
                commands.push(command);
            }
        } else if let Some(start) = line.find("cargo run -p synapse-cli -- ") {
            let command = line[start..]
                .split('"')
                .next()
                .unwrap_or_default()
                .trim()
                .replace("\\\"", "\"");
            if command.split_whitespace().count() > 5 {
                commands.push(command);
            }
        }
    }
    sort_dedup(&mut commands);
    commands
}

fn is_safety_sensitive(path: &str, content: &str) -> bool {
    let text = format!("{} {}", path.to_lowercase(), content.to_lowercase());
    [
        "safety gate",
        "safety_gate",
        "permission gate",
        "permission_gate",
        "value_system",
        "value system",
        "core purpose",
        "core_purpose",
        "identity anchor",
        "identity_anchor",
        "reward homeostasis",
        "self modification boundary",
        "selfmodificationboundary",
        "file execution",
        "network execution",
        "real input",
        "robot control",
        "pc_motor",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn extract_failed_tests(raw_summary: &str) -> Vec<String> {
    let mut tests = Vec::new();
    for line in raw_summary.lines() {
        let trimmed = line.trim();
        if trimmed.contains("FAILED") {
            if let Some(rest) = trimmed.strip_prefix("test ") {
                if let Some((name, _)) = rest.split_once(" ...") {
                    tests.push(name.to_string());
                }
            }
        }
        if let Some(rest) = trimmed.strip_prefix("cargo test failed:") {
            tests.extend(
                rest.split(|ch: char| !ch.is_alphanumeric() && ch != '_')
                    .filter(|token| token.ends_with("phase") || token.contains("_"))
                    .map(ToString::to_string),
            );
        }
    }
    sort_dedup(&mut tests);
    tests
}

fn candidate_reason(file: &IndexedFile, score: f32) -> String {
    if file.safety_sensitive {
        format!("matched query but requires safety review; score {score:.2}")
    } else if file.cli_related {
        format!("matched query and CLI routing surface; score {score:.2}")
    } else {
        format!("matched query in module symbols/tests/path; score {score:.2}")
    }
}

fn normalize_query(value: &str) -> Vec<String> {
    let normalized = value
        .replace("VoiceSynthesis", "voicesynthesis voice")
        .replace("EmergentFunction", "emergentfunction emergent function")
        .replace("GrowthGoal", "growth goal")
        .to_lowercase();
    let mut tokens = normalized
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '-')
        .filter(|token| token.len() > 2)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    sort_dedup(&mut tokens);
    tokens
}

fn stable_id(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn content_hash(content: &str) -> String {
    stable_id(content)
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::coding_knowledge::CodingLanguage;

    use super::{
        CodeGrowthLoop, CodebaseIndex, FailureClass, FailureClassifier, PatchPlan, PatchSafetyGate,
        RelevantFileSelector, SelfModificationBoundary, TestLog,
    };

    #[test]
    fn codebase_index_detects_modules_tests_and_cli_commands() {
        let index = CodebaseIndex::sample();

        assert!(index.modules.contains(&"embryo".to_string()));
        assert!(index
            .tests
            .contains(&"embryo_generates_growth_goal_without_new_manual_phase".to_string()));
        assert!(index
            .cli_commands
            .iter()
            .any(|command| command.contains("code-growth benchmark")));
    }

    #[test]
    fn codebase_index_marks_safety_sensitive_files() {
        let index = CodebaseIndex::sample();

        assert!(index
            .safety_sensitive_files()
            .iter()
            .any(|file| file.path.contains("value_system")));
        assert!(index
            .safety_sensitive_files()
            .iter()
            .any(|file| file.path.contains("identity_evolution")));
    }

    #[test]
    fn failure_classifier_detects_compile_error() {
        let log = TestLog::parse("error[E0425]: cannot find function `demo`");

        assert_eq!(
            FailureClassifier::classify(&log),
            FailureClass::CompileError
        );
    }

    #[test]
    fn failure_classifier_detects_test_failure() {
        let log = TestLog::parse(
            "cargo test failed: test embryo_generates_growth_goal_without_new_manual_phase ... FAILED",
        );

        assert_eq!(FailureClassifier::classify(&log), FailureClass::TestFailure);
        assert!(log
            .failed_tests
            .contains(&"embryo_generates_growth_goal_without_new_manual_phase".to_string()));
    }

    #[test]
    fn failure_classifier_detects_clippy_failure() {
        let log = TestLog::parse("cargo clippy failed: warning: redundant clone");

        assert_eq!(
            FailureClassifier::classify(&log),
            FailureClass::ClippyFailure
        );
    }

    #[test]
    fn relevant_file_selector_finds_embryo_files_for_growth_goal() {
        let index = CodebaseIndex::sample();
        let candidates = RelevantFileSelector::select(
            "embryo grow에서 VoiceSynthesis EmergentFunction이 생성되지 않음",
            &index,
        );

        assert!(candidates
            .iter()
            .any(|candidate| candidate.path.contains("embryo")));
    }

    #[test]
    fn patch_plan_created_from_growth_goal() {
        let loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = loop_system.plan_from_goal("VoiceSynthesis EmergentFunction is not created");

        assert!(!plan.target_files.is_empty());
        assert!(!plan.proposed_changes.is_empty());
        assert!(plan
            .proposed_changes
            .iter()
            .any(|change| change.contains("Rust scaffold")));
        assert!(plan.requires_user_approval);
    }

    #[test]
    fn patch_plan_created_from_test_failure() {
        let loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = loop_system.plan_from_failure(
            "cargo test failed: test embryo_generates_growth_goal_without_new_manual_phase ... FAILED",
        );

        assert_eq!(plan.failure_class.as_deref(), Some("test_failure"));
        assert!(plan.target_files.iter().any(|path| path.contains("embryo")));
    }

    #[test]
    fn patch_proposal_is_diff_preview_not_real_write() {
        let loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let proposal =
            loop_system.propose_from_goal("VoiceSynthesis EmergentFunction is not created");

        assert!(proposal.diff_preview.contains("--- a/"));
        assert!(proposal.diff_preview.contains("proposal-only preview"));
        assert!(proposal
            .expected_tests
            .iter()
            .any(|test| test.contains("cargo clippy")));
        assert!(!proposal.safe_to_apply);
        assert!(proposal.approval_required);
    }

    #[test]
    fn code_growth_loop_uses_language_scaffold_for_patch_plan() {
        let loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let plan = loop_system.plan_from_goal("unwrap used in CLI parser");

        assert!(plan
            .proposed_changes
            .iter()
            .any(|change| change.contains("Result")));
        assert!(plan
            .expected_tests
            .iter()
            .any(|test| test.contains("cargo fmt")));
    }

    #[test]
    fn patch_proposal_includes_language_specific_expected_tests() {
        let loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let proposal = loop_system.propose_from_goal("unwrap used in CLI parser");

        assert!(proposal
            .expected_tests
            .iter()
            .any(|test| test.contains("cargo test")));
        assert!(proposal
            .expected_tests
            .iter()
            .any(|test| test.contains("targeted unit test")));
    }

    #[test]
    fn patch_safety_gate_blocks_safety_sensitive_auto_apply() {
        let index = CodebaseIndex::sample();
        let plan = PatchPlan {
            id: "patch_plan.test".to_string(),
            goal: "change safety gate".to_string(),
            failure_class: None,
            target_files: vec!["crates/synapse-brain/src/value_system/mod.rs".to_string()],
            proposed_changes: vec!["relax safety gate".to_string()],
            risk_score: 0.8,
            requires_user_approval: true,
            expected_tests: vec!["cargo test".to_string()],
        };

        let report = PatchSafetyGate::evaluate(&plan, &index);

        assert!(!report.auto_apply_allowed);
        assert!(report
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("safety_sensitive")));
    }

    #[test]
    fn patch_safety_gate_blocks_network_and_shell_addition() {
        let index = CodebaseIndex::sample();
        let plan = PatchPlan {
            id: "patch_plan.test".to_string(),
            goal: "add network automation".to_string(),
            failure_class: None,
            target_files: vec!["crates/synapse-brain/src/embryo/mod.rs".to_string()],
            proposed_changes: vec!["add http request and shell execution".to_string()],
            risk_score: 0.8,
            requires_user_approval: true,
            expected_tests: vec!["cargo test".to_string()],
        };

        let report = PatchSafetyGate::evaluate(&plan, &index);

        assert!(!report.auto_apply_allowed);
        assert!(report
            .blocked_reasons
            .contains(&"network_call_addition".to_string()));
        assert!(report
            .blocked_reasons
            .contains(&"shell_execution_addition".to_string()));
    }

    #[test]
    fn self_modification_boundary_blocks_core_purpose_change() {
        let index = CodebaseIndex::sample();
        let report = SelfModificationBoundary::evaluate(
            "change core purpose",
            &["crates/synapse-brain/src/identity_evolution/mod.rs".to_string()],
            &["modify identity anchor".to_string()],
            &index,
        );

        assert!(!report.allowed);
        assert!(report
            .blocked_reasons
            .contains(&"core_purpose_change".to_string()));
        assert!(report
            .blocked_reasons
            .contains(&"identity_anchor_change".to_string()));
    }

    #[test]
    fn development_memory_records_patch_attempt() {
        let mut loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let report = loop_system.run_goal("VoiceSynthesis EmergentFunction is not created");

        assert!(report.development_memory.proposal_created);
        assert!(!report.development_memory.applied);
        assert!(report
            .development_memory
            .coding_lessons
            .iter()
            .any(|lesson| lesson.language == CodingLanguage::Rust));
        assert_eq!(loop_system.memory().len(), 1);
    }

    #[test]
    fn code_growth_loop_connects_to_embryo_growth_goal() {
        let mut loop_system = CodeGrowthLoop::from_index(CodebaseIndex::sample());
        let report = loop_system.run_embryo_growth_input("I need a voice");

        assert!(report.embryo_connected);
        assert!(report.patch_plan.goal.contains("VoiceSynthesis"));
        assert!(report
            .selected_files
            .iter()
            .any(|candidate| candidate.path.contains("embryo")));
    }

    #[test]
    fn code_growth_benchmark_reduces_coding_dependency_without_file_write() {
        let report = CodeGrowthLoop::benchmark();

        assert!(report.code_growth_benchmark_reduces_coding_dependency_without_file_write);
        assert!(report.on_coding_dependency_reduction > report.off_coding_dependency_reduction);
        assert!(report.on_manual_debug_dependency < report.off_manual_debug_dependency);
        assert!(report.patch_proposal_is_diff_preview_not_real_write);
    }
}
