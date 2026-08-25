//! Always-on, bounded growth coordination.
//!
//! The supervisor learns generalized structural lessons from explicitly scoped
//! local work. It never records raw source fragments, reads outside configured
//! roots, calls a network/LLM, approves its own candidate, or raises research
//! difficulty to escape a plateau. A separate deterministic verifier process
//! must accept every candidate before a new memory generation is promoted.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::autonomous_self_inspection::{
    inspect as inspect_self, AutonomousSelfInspectionReceipt, DiagnosticPolicyMemory,
    RepairDisposition, RuntimeRepairActionReceipt, RuntimeRepairMechanism, SelfInspectionInput,
};
use crate::autonomous_source_mutation::{
    cleanup_consumed_source_mutation_staging, command_receipt_with_incremental,
    counterexample_from_receipt, derive_improvement_operator_memory,
    discover_executable_performance_improvement, discover_repository_improvement,
    discover_repository_improvement_detailed, full_workspace_semantic_fingerprint,
    install_and_stage_source_patch, runtime_core_feature_available, runtime_core_source_files,
    source_opportunity_family_id, source_patch_failure_is_transient,
    source_patch_validation_critical_path_ms, validate_improvement_operator, validate_policy,
    AutonomousSourceMutationPolicy, AutonomousSourcePatchReceipt, AutonomousSourcePatchRequest,
    ChangeOpportunityKind, ImprovementOperatorGeneratorKind, ImprovementOperatorIR,
    LocalCommandReceipt, SourceDiscoveryResult, SourceMutationStagingCleanup,
    SOURCE_REPAIR_ENGINE_REVISION,
};
use crate::compound_growth::{
    run_compound_growth_input, CompoundGrowthCycleIR, CompoundGrowthInputIR,
    CompoundOperatorRepositoryIR, COMPOUND_GROWTH_INPUT_SCHEMA,
};
use crate::compound_typed_goal::derive_compound_typed_behavior_goals;
use crate::generative_growth::{
    executable_generative_substrate_available, promote_generative_cycle, run_generative_cycle,
    validate_behavioral_execution_receipt, GenerativeComposerIR, GenerativeCycleResult,
    GenerativeGrowthMemory, GenerativeInput,
};
use crate::integrated_development::{
    compose_typed_behavior_goal_candidate, execute_behavioral_composition_canary,
    install_composite_candidate_family, MAX_INSTALLED_TYPED_CAPABILITIES,
};
use crate::intrinsic_drive::{IntrinsicCuriosityHypothesis, IntrinsicDriveMemory};
use crate::same_attempt_revision::{
    CandidateAdmission, SameAttemptCounterexample, SameAttemptRevisionTracker,
    MAX_SAME_ATTEMPT_EXECUTIONS,
};
use crate::self_repair_contract::sha256;
#[cfg(test)]
use crate::sem5::typed_mechanism::select_bounded_typed_mechanism_operator_ids;
use crate::sem5::typed_mechanism::{
    load_authorized_typed_mechanism_operators, persist_authorized_typed_mechanism_operator,
    typed_mechanism_improvement_operator_from_receipt,
    typed_mechanism_operator_authority_directory, typed_mechanism_operator_directory,
    validate_typed_mechanism_improvement_operator, validate_typed_mechanism_operator_authority,
    validate_typed_mechanism_synthesis_goal, TypedMechanismImprovementOperatorIR,
    TypedMechanismOperatorAuthorityReceiptIR, TypedMechanismOperatorPromotionEvidenceIR,
    TypedMechanismSynthesisGoalIR, TypedMechanismSynthesisReceiptIR,
    INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA, MAX_ACTIVE_TYPED_MECHANISM_OPERATORS,
};
use crate::source_bound_causal_frontend::{
    discover_and_synthesize_python_repository_paths_with_operators, replay_source_bound_patch,
    SourceBoundRepositoryPathDiscoveryRequestIR, SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA,
};
use crate::structural_source_repair::SourceEditAtom;

pub const SUPERVISOR_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_SUPERVISOR_1";
pub const CONFIG_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_CONFIG_1";
pub const VERIFIER_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_VERIFIER_1";
pub const COMPOUND_GROWTH_INTEGRATION_SCHEMA: &str = "B_CORE_COMPOUND_GROWTH_INTEGRATION_1";
pub const LINEAGE_CONTINUATION_SCHEMA: &str = "B_CORE_LINEAGE_CONTINUATION_1";
const MAX_COMPOUND_INPUTS_PER_STEP: usize = 8;
const MAX_PENDING_COMPOUND_INPUTS: usize = 256;
const MAX_COMPOUND_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPOUND_RECEIPT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SUMMARY_BYTES: usize = 512;
const SOURCE_PATCH_VALIDATION_CONTRACT_REVISION: u64 = 2;
const PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION: u64 = 6;
const MAX_INTRINSIC_CURIOSITY_HYPOTHESES: usize = 48;
const MAX_RETAINED_INTRINSIC_CURIOSITY_RECEIPTS: usize = 96;
const SCAN_WATCHDOG_TICK_MS: u64 = 1_000;
const MAX_SCAN_RUNTIME_MS: u64 = 60_000;
const FULL_HASH_CANARY_INTERVAL: u64 = 64;
const MAX_QUIET_IDLE_POLL_INTERVAL_MS: u64 = 60_000;
const BASELINE_MAX_HASHED_FILES_PER_SCAN: usize = 1_024;
const BASELINE_MAX_BYTES_PER_SCAN: u64 = 64 * 1024 * 1024;
const MAX_CLASSIFIER_REFINEMENT_EVENTS: usize = 64;
const MAX_RECENT_SOURCE_PATCH_OUTCOMES: usize = 16;
const RUNTIME_REPAIR_COUNTER_CONTRACT_REVISION: u64 = 2;
const INSTALLED_EXECUTION_COUNTER_CONTRACT_REVISION: u64 = 2;
const MAX_CORE_COHORT_VALIDATION_MS: u64 = 3 * 60 * 1_000;
const FULL_CORE_REGRESSION_CANARY_INTERVAL: u64 = 8;
const MAX_REPOSITORY_TEST_PATHS: usize = 8;
const MAX_REPOSITORY_REPAIR_SOURCE_PATHS: usize = 8;
const MAX_REPOSITORY_REPAIR_TARGET_SYMBOLS: usize = 64;
const MAX_REPOSITORY_REPAIR_SANDBOX_FILES: usize = 4_096;
const MAX_REPOSITORY_REPAIR_SANDBOX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ACTIVE_SOURCE_BOUND_IMPROVEMENT_OPERATORS: usize = MAX_ACTIVE_TYPED_MECHANISM_OPERATORS;
const MAX_COMPOSITE_INSTALL_FAMILY: usize = 32;
const REPOSITORY_INSTALL_TRANSACTION_SCHEMA: &str = "B_REPOSITORY_INSTALL_TRANSACTION_1";
const REPOSITORY_INSTALL_COMMIT_SCHEMA: &str = "B_REPOSITORY_INSTALL_COMMIT_1";
const REPOSITORY_REPAIR_SYNTHESIS_SCHEMA: &str = "B_REPOSITORY_REPAIR_SYNTHESIS_3";

fn u64_is_zero(value: &u64) -> bool {
    *value == 0
}

fn logical_path_canary_bucket(logical_path: &str) -> u64 {
    let digest = sha256(logical_path.as_bytes());
    u64::from_str_radix(&digest[..16], 16).unwrap_or(0) % FULL_HASH_CANARY_INTERVAL
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_lifetime_campaigns: u64,
    pub max_generations: u64,
    pub max_active_runtime_ms: u64,
    pub max_state_bytes: u64,
    pub max_observed_bytes: u64,
    pub max_bytes_per_scan: u64,
    pub max_files_per_scan: usize,
    pub max_file_bytes: u64,
    pub max_observations_per_campaign: usize,
    pub max_pending_observations: usize,
    pub max_lessons: usize,
    pub max_consecutive_failures: u32,
    pub plateau_scans_before_wait: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_lifetime_campaigns: 256,
            max_generations: 128,
            max_active_runtime_ms: 7 * 24 * 60 * 60 * 1_000,
            max_state_bytes: 2 * 1024 * 1024 * 1024,
            max_observed_bytes: 64 * 1024 * 1024 * 1024,
            max_bytes_per_scan: 512 * 1024 * 1024,
            max_files_per_scan: 20_000,
            max_file_bytes: 2 * 1024 * 1024,
            max_observations_per_campaign: 32,
            max_pending_observations: 2_048,
            max_lessons: 512,
            max_consecutive_failures: 5,
            plateau_scans_before_wait: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationPolicy {
    pub allowed_extensions: Vec<String>,
    pub excluded_directory_names: Vec<String>,
    pub excluded_file_names: Vec<String>,
    pub minimum_learning_score: u16,
    pub retain_raw_source: bool,
    pub follow_symlinks: bool,
}

impl Default for ObservationPolicy {
    fn default() -> Self {
        Self {
            allowed_extensions: [
                "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "kt", "cs", "cpp", "cc", "c",
                "h", "hpp", "swift", "toml", "yaml", "yml", "json", "ps1", "sh", "sql", "html",
                "css", "scss", "md",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            excluded_directory_names: [
                ".git",
                ".svn",
                ".hg",
                "target",
                "node_modules",
                "vendor",
                "dist",
                "build",
                "coverage",
                ".next",
                ".cache",
                "__pycache__",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            excluded_file_names: [
                ".env",
                ".env.local",
                "id_rsa",
                "id_ed25519",
                "credentials.json",
                "secrets.json",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            minimum_learning_score: 45,
            retain_raw_source: false,
            follow_symlinks: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthSupervisorConfig {
    pub schema: String,
    pub state_dir: PathBuf,
    pub watched_roots: Vec<PathBuf>,
    pub verifier_executable: PathBuf,
    pub poll_interval_ms: u64,
    pub lease_stale_ms: u64,
    pub autonomous_campaigns: bool,
    pub resources: ResourceLimits,
    pub observation: ObservationPolicy,
    #[serde(
        default,
        skip_serializing_if = "AutonomousSourceMutationPolicy::is_default"
    )]
    pub source_mutation: AutonomousSourceMutationPolicy,
    #[serde(default, skip_serializing_if = "RepositoryMutationPolicy::is_default")]
    pub repository_mutation: RepositoryMutationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageStoreReceipt {
    pub relative_store: String,
    pub files: u64,
    pub bytes: u64,
    pub tree_sha256: String,
}

/// Machine-readable proof that a new bounded state line inherited executable
/// production memory from an exact sealed predecessor without carrying its
/// build products, control files, or mutable campaign worktrees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageContinuationReceipt {
    pub schema: String,
    pub predecessor_config_sha256: String,
    pub predecessor_state_sha256: String,
    pub predecessor_state_dir: PathBuf,
    pub predecessor_generation: u64,
    pub predecessor_memory_sha256: String,
    pub successor_config_sha256: String,
    pub successor_state_dir: PathBuf,
    pub successor_initial_state_sha256: String,
    pub carried_memory_sha256: Vec<String>,
    pub carried_stores: Vec<LineageStoreReceipt>,
    pub prestart_autonomous_research_events: u64,
    pub prestart_future_instance_exposure_events: u64,
    pub created_at_ms: u64,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryMutationPolicy {
    pub enabled: bool,
    pub max_installations_per_step: usize,
}

impl RepositoryMutationPolicy {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for RepositoryMutationPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_installations_per_step: 1,
        }
    }
}

impl GrowthSupervisorConfig {
    pub fn bounded_default(
        state_dir: PathBuf,
        watched_root: PathBuf,
        verifier_executable: PathBuf,
    ) -> Self {
        Self {
            schema: CONFIG_SCHEMA.to_string(),
            state_dir,
            watched_roots: vec![watched_root],
            verifier_executable,
            poll_interval_ms: 10_000,
            lease_stale_ms: 15 * 60 * 1_000,
            autonomous_campaigns: true,
            resources: ResourceLimits::default(),
            observation: ObservationPolicy::default(),
            source_mutation: AutonomousSourceMutationPolicy::default(),
            repository_mutation: RepositoryMutationPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupervisorPhase {
    InfraReady,
    Scanning,
    CampaignFrozen,
    CampaignRunning,
    Verifying,
    Promoting,
    WaitingPlateau,
    SafeStopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePatchOutcomeSample {
    pub engine_revision: u64,
    pub installed: bool,
    pub rolled_back: bool,
    pub validation_ms: u64,
    #[serde(default)]
    pub opportunity_kind: ChangeOpportunityKind,
    #[serde(default)]
    pub opportunity_family_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorState {
    pub schema: String,
    pub sequence: u64,
    pub phase: SupervisorPhase,
    pub config_sha256: String,
    pub generation: u64,
    pub current_memory_sha256: String,
    pub predecessor_memory_sha256: Option<String>,
    pub campaigns_started: u64,
    pub campaigns_accepted: u64,
    pub campaigns_failed: u64,
    pub consecutive_failures: u32,
    pub plateau_scans: u32,
    pub active_runtime_ms: u64,
    pub observed_bytes: u64,
    pub pending_campaign_id: Option<String>,
    pub stop_reason: Option<String>,
    pub last_transition_ms: u64,
    pub difficulty_escalation_events: u64,
    pub human_difficulty_level_selection_events: u64,
    pub codex_calls: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub prestart_autonomous_research_events: u64,
    pub prestart_future_instance_exposure_events: u64,
    #[serde(default)]
    pub last_scan_duration_ms: u64,
    #[serde(default)]
    pub last_scan_files_reused: u64,
    #[serde(default)]
    pub last_scan_files_hashed: u64,
    #[serde(default)]
    pub scan_timeout_events: u64,
    #[serde(default)]
    pub self_inspection_events: u64,
    #[serde(default)]
    pub diagnostic_experiment_events: u64,
    #[serde(default)]
    pub diagnostic_policy: DiagnosticPolicyMemory,
    #[serde(default)]
    pub runtime_self_repairs_activated: u64,
    #[serde(default)]
    pub runtime_self_repair_counter_contract_revision: u64,
    #[serde(default)]
    pub legacy_unbound_runtime_self_repair_activations: u64,
    #[serde(default)]
    pub self_repair_capability_gaps: u64,
    #[serde(default)]
    pub last_internal_bottleneck: Option<String>,
    #[serde(default)]
    pub last_self_inspection_sha256: Option<String>,
    #[serde(default)]
    pub evaluator_generation: u64,
    #[serde(default)]
    pub current_evaluator_memory_sha256: String,
    #[serde(default)]
    pub evaluator_challenge_cases: u64,
    #[serde(default)]
    pub mutual_revalidation_events: u64,
    #[serde(default)]
    pub generative_predictions: u64,
    #[serde(default)]
    pub valuable_combinations_learned: u64,
    #[serde(default)]
    pub generative_memory_reuse_events: u64,
    #[serde(default)]
    pub generative_self_application_events: u64,
    #[serde(default)]
    pub generative_exploration_events: u64,
    #[serde(default)]
    pub productive_generative_reuse_events: u64,
    #[serde(default)]
    pub generative_frontier_advance_events: u64,
    #[serde(default)]
    pub generative_frontier_capability_units: u64,
    #[serde(default)]
    pub unverified_generative_frontier_candidate_events: u64,
    #[serde(default)]
    pub legacy_unverified_generative_frontier_advance_events: u64,
    #[serde(default)]
    pub legacy_wrapper_generative_frontier_advance_events: u64,
    #[serde(default)]
    pub generative_behavioral_verification_events: u64,
    #[serde(default)]
    pub redundant_generative_selection_events: u64,
    #[serde(default)]
    pub generative_prediction_absolute_error_total: u64,
    #[serde(default)]
    pub generative_calibrated_prediction_records: u64,
    #[serde(default)]
    pub generative_legacy_uncalibrated_prediction_error_total: u64,
    #[serde(default)]
    pub autonomous_source_patch_attempts: u64,
    #[serde(default)]
    pub autonomous_source_patches_installed: u64,
    #[serde(default)]
    pub autonomous_source_patch_rollbacks: u64,
    #[serde(default)]
    pub autonomous_source_patch_validation_ms: u64,
    #[serde(default)]
    pub source_patch_recent_outcomes: Vec<SourcePatchOutcomeSample>,
    #[serde(default)]
    pub source_patch_telemetry_engine_revision: u64,
    #[serde(default)]
    pub source_patch_validation_contract_revision: u64,
    #[serde(default)]
    pub source_discovery_no_candidate_streak: u32,
    #[serde(default)]
    pub last_source_discovery_reason: Option<String>,
    #[serde(default)]
    pub last_source_discovery_state_sha256: Option<String>,
    #[serde(default)]
    pub source_discovery_duplicate_states_suppressed: u64,
    #[serde(default)]
    pub source_patch_consecutive_failures: u32,
    #[serde(default)]
    pub last_source_patch_receipt_sha256: Option<String>,
    #[serde(default)]
    pub composite_capability_install_attempts: u64,
    #[serde(default)]
    pub composite_capabilities_installed: u64,
    #[serde(default)]
    pub composite_capability_install_rollbacks: u64,
    #[serde(default)]
    pub composite_capability_consecutive_failures: u32,
    #[serde(default)]
    pub last_composite_candidate_sha256: Option<String>,
    #[serde(default)]
    pub installed_composite_capability_execution_events: u64,
    #[serde(default)]
    pub installed_composite_capability_execution_failures: u64,
    #[serde(default)]
    pub last_installed_composite_execution_sha256: Option<String>,
    #[serde(default)]
    pub installed_context_bound_capabilities_validated: u64,
    #[serde(default)]
    pub last_installed_capability_inventory_sha256: Option<String>,
    #[serde(default)]
    pub installed_capability_continuation_observations: u64,
    #[serde(default)]
    pub installed_execution_counter_contract_revision: u64,
    #[serde(default)]
    pub legacy_unbound_installed_composite_execution_events: u64,
    #[serde(default)]
    pub legacy_unbound_installed_composite_execution_failures: u64,
    #[serde(default)]
    pub distinct_semantic_lessons: u64,
    #[serde(default)]
    pub semantic_duplicate_lessons: u64,
    #[serde(default)]
    pub semantic_revalidation_events: u64,
    #[serde(default)]
    pub redundant_observations_consumed: u64,
    #[serde(default)]
    pub measured_performance_promotions: u64,
    #[serde(default)]
    pub classifier_outcome_bound_refinements: u64,
    #[serde(default)]
    pub classifier_unsupported_refinements_suppressed: u64,
    #[serde(default)]
    pub intrinsic_drive: IntrinsicDriveMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkActor {
    User,
    Codex,
    LocalTool,
    UnknownLocalWriter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkKind {
    CodeChange,
    CapabilitySynthesis,
    DefectRepair,
    RegressionTest,
    PerformanceOptimization,
    Refactor,
    FrontendChange,
    BackendChange,
    OperationsChange,
    Documentation,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkOutcome {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkEvent {
    pub event_id: String,
    pub actor: WorkActor,
    pub kind: WorkKind,
    pub paths: Vec<PathBuf>,
    pub outcome: WorkOutcome,
    pub summary: String,
    pub evidence_sha256: Vec<String>,
    #[serde(default)]
    pub evidence_artifacts: Vec<PathBuf>,
    #[serde(default)]
    pub performance_metrics: Vec<PerformanceMetricEvidence>,
    /// Explicit semantic transport for requirement-bearing work. Free-form
    /// summary text is forensic context only and never substitutes for this
    /// observed→expected plus typed-goal contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_contract_deltas: Vec<PublicContractDeltaIR>,
    pub occurred_at_ms: u64,
}

pub const PUBLIC_CONTRACT_DELTA_SCHEMA: &str = "B_CORE_PUBLIC_CONTRACT_DELTA_1";
const MAX_PUBLIC_CONTRACT_DELTAS_PER_EVENT: usize = 8;
const MAX_TYPED_BEHAVIOR_GOALS_PER_DELTA: usize = 8;
const MAX_TYPED_BEHAVIOR_GOALS_PER_GENERATIVE_INPUT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicContractDeltaIR {
    pub schema: String,
    pub delta_id: String,
    pub observed_behavior: String,
    pub expected_behavior: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_symbols: Vec<String>,
    pub typed_behavior_goals: Vec<TypedMechanismSynthesisGoalIR>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformanceMetricEvidence {
    pub metric: String,
    pub before: u64,
    pub after: u64,
    pub lower_is_better: bool,
    pub evidence_sha256: String,
    /// Optional executable optimization knowledge. The metric remains useful
    /// evidence without this payload, but it cannot seed another source patch.
    /// Both hashes bind the operator to the exact observed source transition;
    /// prose and a benchmark delta alone never acquire mutation authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_knowledge: Option<ExecutablePerformanceKnowledgeIR>,
}

pub const EXECUTABLE_PERFORMANCE_KNOWLEDGE_SCHEMA: &str =
    "B_CORE_EXECUTABLE_PERFORMANCE_KNOWLEDGE_1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutablePerformanceKnowledgeIR {
    pub schema: String,
    pub predecessor_content_sha256: String,
    pub candidate_content_sha256: String,
    pub improvement_operator: ImprovementOperatorIR,
}

impl PerformanceMetricEvidence {
    fn improved(&self) -> bool {
        if self.lower_is_better {
            self.after < self.before
        } else {
            self.after > self.before
        }
    }

    fn executable_for_transition(&self, predecessor: Option<&str>, candidate: &str) -> bool {
        let Some(knowledge) = &self.executable_knowledge else {
            return false;
        };
        self.improved()
            && knowledge.schema == EXECUTABLE_PERFORMANCE_KNOWLEDGE_SCHEMA
            && predecessor == Some(knowledge.predecessor_content_sha256.as_str())
            && candidate == knowledge.candidate_content_sha256
            && validate_improvement_operator(&knowledge.improvement_operator).is_ok()
    }

    fn has_executable_knowledge(&self) -> bool {
        self.improved()
            && self.executable_knowledge.as_ref().is_some_and(|knowledge| {
                knowledge.schema == EXECUTABLE_PERFORMANCE_KNOWLEDGE_SCHEMA
                    && validate_improvement_operator(&knowledge.improvement_operator).is_ok()
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralFeatures {
    pub lines: u32,
    pub non_empty_lines: u32,
    pub public_symbols: u32,
    pub branch_tokens: u32,
    pub assertion_tokens: u32,
    pub test_tokens: u32,
    pub validation_tokens: u32,
    pub error_handling_tokens: u32,
    pub documentation_tokens: u32,
    pub todo_tokens: u32,
    #[serde(default)]
    pub benchmark_tokens: u32,
    #[serde(default)]
    pub performance_tokens: u32,
    #[serde(default)]
    pub algebraic_constructor_tokens: u32,
    #[serde(default)]
    pub data_composition_tokens: u32,
    pub max_line_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub content_sha256: String,
    pub bytes: u64,
    pub modified_ms: u64,
    pub extension: String,
    pub features: StructuralFeatures,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LearningValue {
    High,
    Medium,
    Low,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningObservation {
    pub observation_id: String,
    pub work_event_id: Option<String>,
    pub logical_path: String,
    pub content_sha256: String,
    pub predecessor_content_sha256: Option<String>,
    pub actor: WorkActor,
    pub work_kind: WorkKind,
    pub work_outcome: WorkOutcome,
    pub features_before: Option<StructuralFeatures>,
    pub features_after: StructuralFeatures,
    pub signals: Vec<String>,
    pub composition_roles: Vec<String>,
    pub learning_score: u16,
    pub learning_value: LearningValue,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub verification_evidence_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub performance_metrics: Vec<PerformanceMetricEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_contract_deltas: Vec<PublicContractDeltaIR>,
    pub exact_source_fragments_stored: usize,
    pub raw_source_bytes_stored: usize,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIndex {
    pub schema: String,
    pub sequence: u64,
    #[serde(default)]
    pub baseline_complete: bool,
    pub files: BTreeMap<String, FileFingerprint>,
    pub consumed_observation_ids: BTreeSet<String>,
    pub consumed_work_event_ids: BTreeSet<String>,
}

impl Default for FileIndex {
    fn default() -> Self {
        Self {
            schema: SUPERVISOR_SCHEMA.to_string(),
            sequence: 0,
            baseline_complete: false,
            files: BTreeMap::new(),
            consumed_observation_ids: BTreeSet::new(),
            consumed_work_event_ids: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifierMemory {
    pub signal_weights: BTreeMap<String, i16>,
    pub accepted_campaigns: u64,
    pub rejected_campaigns: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refinement_events: Vec<ClassifierRefinementEvent>,
    #[serde(default, skip_serializing_if = "u64_is_zero")]
    pub outcome_bound_refinements: u64,
    #[serde(default, skip_serializing_if = "u64_is_zero")]
    pub unsupported_refinements_suppressed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifierWeightDelta {
    pub signal: String,
    pub before: i16,
    pub after: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifierRefinementEvent {
    pub refinement_id: String,
    pub generation: u64,
    pub source_lesson_id: String,
    pub evidence_observation_sha256: Vec<String>,
    pub considered_signals: Vec<String>,
    pub weight_deltas: Vec<ClassifierWeightDelta>,
    pub behavioral_frontier_advance: bool,
    pub measured_performance_gain: bool,
    pub behavioral_verification_sha256: Option<String>,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedCompositionLesson {
    pub lesson_id: String,
    pub evidence_observation_sha256: Vec<String>,
    pub work_kinds: Vec<WorkKind>,
    pub diagnostic_signals: Vec<String>,
    pub composition_recipe: Vec<String>,
    pub applicability: Vec<String>,
    pub verification_obligations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub performance_metrics: Vec<PerformanceMetricEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_contract_deltas: Vec<PublicContractDeltaIR>,
    pub learning_score: u16,
    pub exact_patch_data_present: bool,
    pub exact_source_fragment_present: bool,
    pub raw_source_bytes_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthMemory {
    pub schema: String,
    pub generation: u64,
    pub predecessor_sha256: Option<String>,
    pub lessons: Vec<LearnedCompositionLesson>,
    pub classifier: ClassifierMemory,
    #[serde(default, skip_serializing_if = "EvaluatorMemory::is_default")]
    pub evaluator: EvaluatorMemory,
    #[serde(default, skip_serializing_if = "GenerativeGrowthMemory::is_default")]
    pub generative: GenerativeGrowthMemory,
}

/// Immutable bridge receipt proving that one typed compound-growth input was
/// evaluated inside the ordinary Supervisor loop against the exact preceding
/// operator repository. The complete input is retained for deterministic
/// replay; prose never substitutes for its typed evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundGrowthIntegrationReceipt {
    pub schema: String,
    pub sequence: u64,
    pub generation: u64,
    pub input: CompoundGrowthInputIR,
    pub input_sha256: String,
    pub predecessor_repository_sha256: String,
    pub predecessor_receipt_sha256: Option<String>,
    pub cycle: CompoundGrowthCycleIR,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundGrowthIntegrationStatus {
    pub schema: String,
    pub cycles_committed: u64,
    pub pending_inputs: usize,
    pub repository_profiles: usize,
    pub productive_composite_graphs: usize,
    pub latest_cycle_sha256: Option<String>,
    pub latest_receipt_sha256: Option<String>,
    pub external_model_calls: usize,
    pub text_only_growth_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorMemory {
    pub schema: String,
    pub generation: u64,
    pub predecessor_sha256: Option<String>,
    pub challenge_suite: Vec<EvaluatorMutationKind>,
    pub source_lesson_ids: Vec<String>,
    pub accepted_expansions: u64,
    #[serde(default, skip_serializing_if = "u64_is_zero")]
    pub capability_expansion_contract_revision: u64,
    #[serde(default, skip_serializing_if = "u64_is_zero")]
    pub legacy_unbound_accepted_expansions: u64,
}

impl Default for EvaluatorMemory {
    fn default() -> Self {
        Self {
            schema: "B_CORE_GROWTH_EVALUATOR_MEMORY_1".to_string(),
            generation: 0,
            predecessor_sha256: None,
            challenge_suite: vec![
                EvaluatorMutationKind::EvidenceDigestSubstitution,
                EvaluatorMutationKind::AggregateScoreInflation,
                EvaluatorMutationKind::LessonScoreInflation,
                EvaluatorMutationKind::DiagnosticSignalInjection,
                EvaluatorMutationKind::CompositionRecipeMutation,
                EvaluatorMutationKind::WorkKindMutation,
            ],
            source_lesson_ids: Vec::new(),
            accepted_expansions: 0,
            capability_expansion_contract_revision: 0,
            legacy_unbound_accepted_expansions: 0,
        }
    }
}

impl EvaluatorMemory {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignFreeze {
    pub schema: String,
    pub campaign_id: String,
    pub generation: u64,
    pub predecessor_memory_sha256: String,
    pub config_sha256: String,
    pub observation_ids: Vec<String>,
    pub observation_sha256: Vec<String>,
    pub proposer_executable_sha256: String,
    pub verifier_executable_sha256: String,
    pub seed: u64,
    pub budget_observations: usize,
    pub frozen_before_candidate: bool,
    pub operator_selected_difficulty: bool,
    pub human_difficulty_escalation_events: u64,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningCandidate {
    pub schema: String,
    pub campaign_id: String,
    pub freeze_sha256: String,
    pub generation: u64,
    pub predecessor_memory_sha256: String,
    pub lesson: LearnedCompositionLesson,
    pub observation_ids: Vec<String>,
    pub total_learning_score: u32,
    pub generative_cycle: GenerativeCycleResult,
    pub raw_source_bytes: usize,
    pub exact_source_fragments: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
    pub self_approval_events: usize,
    pub difficulty_escalation_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierRequest {
    pub schema: String,
    pub freeze_path: PathBuf,
    pub candidate_path: PathBuf,
    pub expected_freeze_sha256: String,
    pub expected_candidate_sha256: String,
    pub expected_verifier_sha256: String,
    pub minimum_learning_score: u16,
    pub max_observations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrowthDecision {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrowthVerificationReceipt {
    pub schema: String,
    pub campaign_id: String,
    pub freeze_sha256: String,
    pub candidate_sha256: String,
    pub verifier_executable_sha256: String,
    pub decision: GrowthDecision,
    pub reasons: Vec<String>,
    pub verifier_is_proposer: bool,
    pub deterministic_checks_only: bool,
    pub local_process: bool,
    pub raw_source_bytes_observed: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
    pub human_verification_decisions: usize,
    pub evaluator_self_audit: EvaluatorSelfAudit,
    pub receipt_sha256: String,
    pub authority_seal: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvaluatorMutationKind {
    EvidenceDigestSubstitution,
    AggregateScoreInflation,
    LessonScoreInflation,
    DiagnosticSignalInjection,
    CompositionRecipeMutation,
    WorkKindMutation,
    ApplicabilityMutation,
    VerificationObligationMutation,
    RawSourceFlagInjection,
    EvidenceRemoval,
}

impl EvaluatorMutationKind {
    const ALL: [Self; 10] = [
        Self::EvidenceDigestSubstitution,
        Self::AggregateScoreInflation,
        Self::LessonScoreInflation,
        Self::DiagnosticSignalInjection,
        Self::CompositionRecipeMutation,
        Self::WorkKindMutation,
        Self::ApplicabilityMutation,
        Self::VerificationObligationMutation,
        Self::RawSourceFlagInjection,
        Self::EvidenceRemoval,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorMutationResult {
    pub mutation: EvaluatorMutationKind,
    pub expected_reject: bool,
    pub rejected: bool,
    pub survived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorSelfAudit {
    pub schema: String,
    pub challenger_identity: String,
    pub evaluator_identity: String,
    pub baseline_candidate_reconstructed: bool,
    pub mutation_results: Vec<EvaluatorMutationResult>,
    pub mutation_cases: usize,
    pub mutation_survivors: usize,
    pub pass: bool,
    pub active_evaluator_generation: u64,
    pub proposed_evaluator_generation: u64,
    pub proposed_evaluator_memory_sha256: String,
    pub knowledge_challenge_cases: usize,
    pub challenge_suite_expanded: bool,
    pub post_challenge_core_revalidated: bool,
    pub evaluator_self_approval_events: usize,
    pub codex_calls: usize,
    pub external_llm_calls: usize,
    pub network_reads: usize,
    pub network_writes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignHistory {
    pub campaign_id: String,
    pub generation_attempted: u64,
    pub accepted: bool,
    pub predecessor_memory_sha256: String,
    pub resulting_memory_sha256: Option<String>,
    pub freeze_sha256: String,
    pub candidate_sha256: String,
    pub verification_receipt_sha256: String,
    pub rollback_reference: String,
    pub failed_candidate_deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InvalidGenerationRecoveryReceipt {
    schema: String,
    invalid_generation: u64,
    invalid_memory_sha256: String,
    invalid_lesson_id: String,
    restored_generation: u64,
    restored_memory_sha256: String,
    reason: String,
    recovered_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReport {
    pub schema: String,
    pub phase: SupervisorPhase,
    pub generation: u64,
    pub baseline_created: bool,
    pub files_scanned: usize,
    pub observations_created: usize,
    pub high_value_observations: usize,
    pub campaign_id: Option<String>,
    pub campaign_accepted: Option<bool>,
    pub waiting_on_plateau: bool,
    pub stop_reason: Option<String>,
    pub current_memory_sha256: String,
    pub difficulty_escalation_events: u64,
    pub codex_calls: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
    pub network_writes: u64,
    pub last_scan_duration_ms: u64,
    pub last_scan_files_reused: u64,
    pub last_scan_files_hashed: u64,
    pub scan_timeout_events: u64,
    pub self_inspection_events: u64,
    pub diagnostic_experiment_events: u64,
    pub diagnostic_policy_selections: u64,
    pub diagnostic_policy_explorations: u64,
    pub diagnostic_policy_causal_support_events: u64,
    pub diagnostic_policy_outcome_bound_selections: u64,
    pub diagnostic_policy_productive_outcomes: u64,
    pub diagnostic_policy_failed_outcomes: u64,
    pub diagnostic_policy_duplicate_selections_suppressed: u64,
    pub runtime_self_repairs_activated: u64,
    pub runtime_self_repair_counter_contract_revision: u64,
    pub legacy_unbound_runtime_self_repair_activations: u64,
    pub self_repair_capability_gaps: u64,
    pub last_internal_bottleneck: Option<String>,
    pub evaluator_generation: u64,
    pub evaluator_challenge_cases: u64,
    pub mutual_revalidation_events: u64,
    pub generative_predictions: u64,
    pub valuable_combinations_learned: u64,
    pub generative_memory_reuse_events: u64,
    pub generative_self_application_events: u64,
    pub generative_exploration_events: u64,
    pub productive_generative_reuse_events: u64,
    pub generative_frontier_advance_events: u64,
    pub generative_frontier_capability_units: u64,
    pub unverified_generative_frontier_candidate_events: u64,
    pub legacy_unverified_generative_frontier_advance_events: u64,
    pub legacy_wrapper_generative_frontier_advance_events: u64,
    pub generative_behavioral_verification_events: u64,
    pub redundant_generative_selection_events: u64,
    pub generative_mean_prediction_error_millis: u64,
    pub generative_calibrated_prediction_records: u64,
    pub generative_legacy_uncalibrated_prediction_error_total: u64,
    pub autonomous_source_patch_attempts: u64,
    pub autonomous_source_patches_installed: u64,
    pub autonomous_source_patch_rollbacks: u64,
    pub autonomous_source_patch_validation_ms: u64,
    pub source_patch_recent_attempts: u64,
    pub source_patch_recent_installations: u64,
    pub source_patch_recent_rollbacks: u64,
    pub source_patch_recent_validation_ms: u64,
    pub source_patch_recent_distinct_opportunity_families: u64,
    pub source_patch_recent_defect_families: u64,
    pub source_patch_recent_capability_gap_families: u64,
    pub source_patch_recent_efficiency_opportunity_families: u64,
    pub source_patch_recent_robustness_opportunity_families: u64,
    pub source_patch_recent_research_hypothesis_families: u64,
    pub source_patch_recent_verified_improvements: u64,
    pub source_discovery_no_candidate_streak: u32,
    pub last_source_discovery_reason: Option<String>,
    pub source_discovery_duplicate_states_suppressed: u64,
    pub source_patch_consecutive_failures: u32,
    pub last_source_patch_receipt_sha256: Option<String>,
    pub composite_capability_install_attempts: u64,
    pub composite_capabilities_installed: u64,
    pub composite_capability_install_rollbacks: u64,
    pub composite_capability_consecutive_failures: u32,
    pub last_composite_candidate_sha256: Option<String>,
    pub installed_composite_capability_execution_events: u64,
    pub installed_composite_capability_execution_failures: u64,
    pub last_installed_composite_execution_sha256: Option<String>,
    pub installed_context_bound_capabilities_validated: u64,
    pub last_installed_capability_inventory_sha256: Option<String>,
    pub installed_capability_continuation_observations: u64,
    pub installed_execution_counter_contract_revision: u64,
    pub legacy_unbound_installed_composite_execution_events: u64,
    pub legacy_unbound_installed_composite_execution_failures: u64,
    pub distinct_semantic_lessons: u64,
    pub semantic_duplicate_lessons: u64,
    pub semantic_revalidation_events: u64,
    pub redundant_observations_consumed: u64,
    pub measured_performance_promotions: u64,
    pub classifier_outcome_bound_refinements: u64,
    pub classifier_unsupported_refinements_suppressed: u64,
    pub intrinsic_curiosity_hypotheses_attempted: u64,
    pub intrinsic_curiosity_hypotheses_succeeded: u64,
    pub intrinsic_curiosity_hypotheses_failed: u64,
    pub intrinsic_curiosity_hypotheses_pending: usize,
    pub intrinsic_reward_events: u64,
    pub intrinsic_reward_total: u64,
    pub intrinsic_reward_contract_revision: u64,
    pub legacy_precommit_intrinsic_reward_events: u64,
    pub legacy_precommit_intrinsic_reward_total: u64,
    pub current_curiosity: u16,
    pub verified_satisfaction: u16,
    pub last_intrinsic_hypothesis_id: Option<String>,
    pub last_intrinsic_reward: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfCheck {
    pub schema: String,
    pub pass: bool,
    pub proposer_cannot_self_approve: bool,
    pub raw_source_retention_forbidden: bool,
    pub network_and_llm_disabled: bool,
    pub plateau_difficulty_escalation_disabled: bool,
    pub current_and_predecessor_memory_only: bool,
    pub frozen_observation_reconstruction_enabled: bool,
    pub bound_pass_evidence_required: bool,
    pub evaluator_mutation_self_audit_enabled: bool,
    pub evaluator_generation_evolution_enabled: bool,
    pub prediction_before_composition_enabled: bool,
    pub valuable_combination_memory_enabled: bool,
    pub generative_memory_self_application_enabled: bool,
    pub core_self_approval_enabled: bool,
    pub autonomous_source_patch_install_enabled: bool,
    pub source_patch_rollback_enabled: bool,
    pub promoted_lessons_drive_executable_repairs: bool,
    /// Free-form diagnostic labels and prose are never executable knowledge
    /// authority. A lesson must carry a validated typed behavior goal before
    /// it can enter growth memory.
    pub text_only_knowledge_is_capability_authority: bool,
    pub executable_knowledge_gate_enabled: bool,
    pub static_canary_replay_is_knowledge_growth: bool,
    pub bounded_failure_retry_enabled: bool,
    pub successful_solution_learning_enabled: bool,
    pub admitted_failure_revisit_after_growth_enabled: bool,
    pub source_repair_engine_revision: u64,
    pub operator_stop_survives_self_update: bool,
    pub workspace_freeze_during_patch_validation: bool,
    pub performance_aware_self_inspection: bool,
    pub predicted_utility_source_gate: bool,
    pub staged_source_validation: bool,
    pub runtime_core_static_validation_surface_enabled: bool,
    pub historical_regression_canary_separated: bool,
    pub warm_incremental_validation_cache_enabled: bool,
    pub adaptive_idle_polling: bool,
    pub mixed_production_file_role_detection: bool,
    pub semantic_duplicate_promotion_blocked: bool,
    pub measured_performance_evidence_supported: bool,
    pub metric_only_performance_is_not_growth_authority: bool,
    pub executable_performance_operator_reuse_enabled: bool,
    pub contextual_generative_exploration_enabled: bool,
    pub redundant_reuse_excluded_from_growth: bool,
    pub adaptive_diagnostic_policy_enabled: bool,
    pub exploration_bonus_excluded_from_prediction: bool,
    pub composition_scoped_policy_application: bool,
    pub diagnostic_reward_requires_later_frontier_outcome: bool,
    pub same_generation_diagnostic_reward_deduplicated: bool,
    pub heuristic_composition_value_excluded_from_frontier: bool,
    pub behavioral_evidence_required_for_generative_self_application: bool,
    pub behavioral_composition_execution_enabled: bool,
    pub redundant_generative_verifier_search_disabled: bool,
    pub classifier_refinement_requires_capability_evidence: bool,
    pub classifier_refinement_delta_ledger_enabled: bool,
    pub source_patch_diagnostics_use_recent_engine_window: bool,
    pub source_synthesis_exhaustion_is_capability_gap: bool,
    pub rust_source_ast_modeling_enabled: bool,
    pub syntactic_call_and_data_flow_modeling_enabled: bool,
    pub structural_postcondition_derivation_enabled: bool,
    pub universal_source_edit_atoms_enabled: bool,
    pub structural_repair_replay_gate_enabled: bool,
    pub autonomous_compiler_diagnostic_discovery_enabled: bool,
    pub typed_grammar_composition_enabled: bool,
    pub public_counterexample_guided_revision_enabled: bool,
    pub same_attempt_counterexample_revision_enabled: bool,
    pub same_attempt_revision_requires_exact_rollback: bool,
    pub validation_process_tree_termination_enabled: bool,
    pub validation_output_is_bounded: bool,
    pub successful_edit_composition_learning_enabled: bool,
    pub bounded_compiler_diagnostic_cache_enabled: bool,
    pub dynamic_self_weakness_discovery_enabled: bool,
    pub generalized_change_ir_bound_to_source_edits: bool,
    pub validation_counterexamples_drive_candidate_ranking: bool,
    pub multi_generation_self_application_lineage_enabled: bool,
    pub fixed_sem9_toggle_replay_forbidden: bool,
    pub runtime_repair_counter_requires_executed_action: bool,
    pub diagnostic_outcome_requires_action_output_consumption: bool,
    pub diagnostic_productivity_requires_current_executable_intervention: bool,
    pub unbound_capability_gap_state_deduplicated: bool,
    pub test_only_evaluator_cohort_validation_enabled: bool,
    pub validation_receipt_identity_excludes_generation: bool,
    pub verification_only_generation_promotion_forbidden: bool,
    pub verification_only_false_tip_auto_recovery: bool,
    pub source_discovery_applicability_precedes_value_gate: bool,
    pub identical_source_discovery_state_deduplicated: bool,
    pub diagnostic_opportunity_kind_separated_from_executability: bool,
    pub self_healing_candidates_route_to_atomic_installer: bool,
    pub repository_candidate_requires_authoritative_install_authority: bool,
    pub repository_install_transaction_recovery_enabled: bool,
    pub authoritative_repository_validation_before_learning_enabled: bool,
    pub integrated_program_ir_lowers_to_compiled_rust: bool,
    pub installed_compositions_are_runtime_callable: bool,
    pub typed_lowering_preserves_installed_capability_registry: bool,
    pub generated_capabilities_dispatch_by_program_hash: bool,
    pub contextual_typed_task_generation_enabled: bool,
    pub verified_program_artifact_frontier_tracked: bool,
    pub wrapper_composition_count_excluded_from_capability_count: bool,
    pub canonical_grammar_role_operations_enabled: bool,
    pub same_type_call_role_permutations_bounded: bool,
    pub symmetric_state_transform_compilation_enabled: bool,
    pub accepted_sem5_compositions_route_to_installer: bool,
    pub sem5_installer_requires_typed_plan_and_exact_goal: bool,
    pub active_binaries_forbid_proposal_only_exit: bool,
    pub executable_improvement_operator_repository_enabled: bool,
    pub improvement_operator_repository_requires_source_synthesis_payload: bool,
    pub program_execution_profile_is_not_synthesis_knowledge: bool,
    pub source_proposal_composition_and_ranking_owned_by_rust_kernel: bool,
    pub source_proposal_competitors_bounded_to_three: bool,
    pub all_language_source_proposals_share_one_rust_kernel: bool,
    pub required_composition_groups_owned_by_rust_kernel: bool,
    pub language_frontends_cannot_rank_or_merge_source_proposals: bool,
    pub source_generators_submit_bounded_proposal_batches: bool,
    pub source_kernel_is_first_candidate_selection_authority: bool,
    pub compiler_applicability_authority_is_typed: bool,
    pub raw_compiler_applicability_is_metadata_only: bool,
    pub generative_execution_dispatch_uses_typed_rust_ir: bool,
    pub generative_stage_strings_are_metadata_only: bool,
    pub python_host_failure_ontology_owned_by_rust_kernel: bool,
    pub fullstack_knowledge_uses_typed_executable_transitions: bool,
    pub fullstack_text_contracts_are_metadata_only: bool,
    pub successful_operators_are_content_addressed: bool,
    pub cross_family_operator_transfer_changes_candidate_priority: bool,
    pub repository_guided_outcomes_are_causally_tracked: bool,
    pub typed_improvement_operator_execution_required: bool,
    pub operator_repository_requires_executed_receipt: bool,
    pub generative_substrate_capacity_isolated: bool,
    pub saturated_substrate_routes_without_difficulty_escalation: bool,
    pub compound_growth_runs_inside_supervisor_loop: bool,
    pub compound_repository_authority_is_supervisor_owned: bool,
    pub compound_growth_requires_typed_hashed_evidence: bool,
    pub compound_typed_goal_functional_composition_enabled: bool,
    pub compound_typed_goal_requires_public_causal_join: bool,
    pub compound_typed_goal_effects_fail_closed: bool,
    pub verified_compound_programs_are_promoted_to_memory: bool,
    pub generative_prediction_is_selection_only: bool,
    pub evaluator_expansion_requires_new_challenge_capability: bool,
    pub intrinsic_curiosity_requires_executable_hypotheses: bool,
    pub intrinsic_reward_requires_verified_frontier: bool,
    pub intrinsic_reward_requires_independent_promotion: bool,
    pub intrinsic_exploration_is_bounded: bool,
    pub mutual_recursive_growth_observed: bool,
}

pub fn self_check() -> SelfCheck {
    SelfCheck {
        schema: SUPERVISOR_SCHEMA.to_string(),
        pass: true,
        proposer_cannot_self_approve: false,
        raw_source_retention_forbidden: true,
        network_and_llm_disabled: true,
        plateau_difficulty_escalation_disabled: true,
        current_and_predecessor_memory_only: true,
        frozen_observation_reconstruction_enabled: true,
        bound_pass_evidence_required: true,
        evaluator_mutation_self_audit_enabled: true,
        evaluator_generation_evolution_enabled: true,
        prediction_before_composition_enabled: true,
        valuable_combination_memory_enabled: true,
        generative_memory_self_application_enabled: true,
        core_self_approval_enabled: true,
        autonomous_source_patch_install_enabled: true,
        source_patch_rollback_enabled: true,
        promoted_lessons_drive_executable_repairs: true,
        text_only_knowledge_is_capability_authority: false,
        executable_knowledge_gate_enabled: true,
        static_canary_replay_is_knowledge_growth: false,
        bounded_failure_retry_enabled: true,
        successful_solution_learning_enabled: true,
        admitted_failure_revisit_after_growth_enabled: true,
        source_repair_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
        operator_stop_survives_self_update: true,
        workspace_freeze_during_patch_validation: true,
        performance_aware_self_inspection: true,
        predicted_utility_source_gate: true,
        staged_source_validation: true,
        runtime_core_static_validation_surface_enabled: true,
        historical_regression_canary_separated: true,
        warm_incremental_validation_cache_enabled: true,
        adaptive_idle_polling: true,
        mixed_production_file_role_detection: true,
        semantic_duplicate_promotion_blocked: true,
        measured_performance_evidence_supported: true,
        metric_only_performance_is_not_growth_authority: true,
        executable_performance_operator_reuse_enabled: true,
        contextual_generative_exploration_enabled: true,
        redundant_reuse_excluded_from_growth: true,
        adaptive_diagnostic_policy_enabled: true,
        exploration_bonus_excluded_from_prediction: true,
        composition_scoped_policy_application: true,
        diagnostic_reward_requires_later_frontier_outcome: true,
        same_generation_diagnostic_reward_deduplicated: true,
        heuristic_composition_value_excluded_from_frontier: true,
        behavioral_evidence_required_for_generative_self_application: true,
        behavioral_composition_execution_enabled: true,
        redundant_generative_verifier_search_disabled: true,
        classifier_refinement_requires_capability_evidence: true,
        classifier_refinement_delta_ledger_enabled: true,
        source_patch_diagnostics_use_recent_engine_window: true,
        source_synthesis_exhaustion_is_capability_gap: true,
        rust_source_ast_modeling_enabled: true,
        syntactic_call_and_data_flow_modeling_enabled: true,
        structural_postcondition_derivation_enabled: true,
        universal_source_edit_atoms_enabled: true,
        structural_repair_replay_gate_enabled: true,
        autonomous_compiler_diagnostic_discovery_enabled: true,
        typed_grammar_composition_enabled: true,
        public_counterexample_guided_revision_enabled: true,
        same_attempt_counterexample_revision_enabled: true,
        same_attempt_revision_requires_exact_rollback: true,
        validation_process_tree_termination_enabled: true,
        validation_output_is_bounded: true,
        successful_edit_composition_learning_enabled: true,
        bounded_compiler_diagnostic_cache_enabled: true,
        dynamic_self_weakness_discovery_enabled: true,
        generalized_change_ir_bound_to_source_edits: true,
        validation_counterexamples_drive_candidate_ranking: true,
        multi_generation_self_application_lineage_enabled: true,
        fixed_sem9_toggle_replay_forbidden: true,
        runtime_repair_counter_requires_executed_action: true,
        diagnostic_outcome_requires_action_output_consumption: true,
        diagnostic_productivity_requires_current_executable_intervention: true,
        unbound_capability_gap_state_deduplicated: true,
        test_only_evaluator_cohort_validation_enabled: true,
        validation_receipt_identity_excludes_generation: true,
        verification_only_generation_promotion_forbidden: true,
        verification_only_false_tip_auto_recovery: true,
        source_discovery_applicability_precedes_value_gate: true,
        identical_source_discovery_state_deduplicated: true,
        diagnostic_opportunity_kind_separated_from_executability: true,
        self_healing_candidates_route_to_atomic_installer: true,
        repository_candidate_requires_authoritative_install_authority: true,
        repository_install_transaction_recovery_enabled: true,
        authoritative_repository_validation_before_learning_enabled: true,
        integrated_program_ir_lowers_to_compiled_rust: true,
        installed_compositions_are_runtime_callable: true,
        typed_lowering_preserves_installed_capability_registry: true,
        generated_capabilities_dispatch_by_program_hash: true,
        contextual_typed_task_generation_enabled: true,
        verified_program_artifact_frontier_tracked: true,
        wrapper_composition_count_excluded_from_capability_count: true,
        canonical_grammar_role_operations_enabled: true,
        same_type_call_role_permutations_bounded: true,
        symmetric_state_transform_compilation_enabled: true,
        accepted_sem5_compositions_route_to_installer: true,
        sem5_installer_requires_typed_plan_and_exact_goal: true,
        active_binaries_forbid_proposal_only_exit: true,
        executable_improvement_operator_repository_enabled: true,
        improvement_operator_repository_requires_source_synthesis_payload: true,
        program_execution_profile_is_not_synthesis_knowledge: true,
        source_proposal_composition_and_ranking_owned_by_rust_kernel: true,
        source_proposal_competitors_bounded_to_three: true,
        all_language_source_proposals_share_one_rust_kernel: true,
        required_composition_groups_owned_by_rust_kernel: true,
        language_frontends_cannot_rank_or_merge_source_proposals: true,
        source_generators_submit_bounded_proposal_batches: true,
        source_kernel_is_first_candidate_selection_authority: true,
        compiler_applicability_authority_is_typed: true,
        raw_compiler_applicability_is_metadata_only: true,
        generative_execution_dispatch_uses_typed_rust_ir: true,
        generative_stage_strings_are_metadata_only: true,
        python_host_failure_ontology_owned_by_rust_kernel: true,
        fullstack_knowledge_uses_typed_executable_transitions: true,
        fullstack_text_contracts_are_metadata_only: true,
        successful_operators_are_content_addressed: true,
        cross_family_operator_transfer_changes_candidate_priority: true,
        repository_guided_outcomes_are_causally_tracked: true,
        typed_improvement_operator_execution_required: true,
        operator_repository_requires_executed_receipt: true,
        generative_substrate_capacity_isolated: true,
        saturated_substrate_routes_without_difficulty_escalation: true,
        compound_growth_runs_inside_supervisor_loop: true,
        compound_repository_authority_is_supervisor_owned: true,
        compound_growth_requires_typed_hashed_evidence: true,
        compound_typed_goal_functional_composition_enabled: true,
        compound_typed_goal_requires_public_causal_join: true,
        compound_typed_goal_effects_fail_closed: true,
        verified_compound_programs_are_promoted_to_memory: true,
        generative_prediction_is_selection_only: true,
        evaluator_expansion_requires_new_challenge_capability: true,
        intrinsic_curiosity_requires_executable_hypotheses: true,
        intrinsic_reward_requires_verified_frontier: true,
        intrinsic_reward_requires_independent_promotion: true,
        intrinsic_exploration_is_bounded: true,
        mutual_recursive_growth_observed: false,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn json_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("JSON_SERIALIZE:{error}"))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("READ:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("JSON:{}:{error}", path.display()))
}

fn write_immutable_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if path.exists() {
        return Err(format!("IMMUTABLE_PATH_EXISTS:{}", path.display()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("PARENT_MISSING:{}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("MKDIR:{}:{error}", parent.display()))?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(OsStr::to_str).unwrap_or("growth"),
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("CREATE:{}:{error}", temporary.display()))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("WRITE:{}:{error}", temporary.display()))?;
    drop(file);
    if path.exists() {
        let _ = fs::remove_file(&temporary);
        return Err(format!("IMMUTABLE_PATH_RACE:{}", path.display()));
    }
    fs::rename(&temporary, path).map_err(|error| format!("RENAME:{}:{error}", path.display()))
}

fn file_sha256(path: &Path, max_bytes: u64) -> Result<String, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("METADATA:{}:{error}", path.display()))?;
    if metadata.len() > max_bytes {
        return Err(format!("FILE_TOO_LARGE:{}", path.display()));
    }
    fs::read(path)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("READ:{}:{error}", path.display()))
}

fn compound_growth_root(config: &GrowthSupervisorConfig) -> PathBuf {
    config.state_dir.join("compound_growth")
}

fn compound_growth_queue(config: &GrowthSupervisorConfig) -> PathBuf {
    compound_growth_root(config).join("queue")
}

fn compound_growth_receipts(config: &GrowthSupervisorConfig) -> PathBuf {
    compound_growth_root(config).join("receipts")
}

fn compound_receipt_hash(receipt: &CompoundGrowthIntegrationReceipt) -> Result<String, String> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    json_sha256(&unsigned)
}

fn read_bounded_compound_json<T: DeserializeOwned>(
    path: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("{label}_METADATA:{}:{error}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(format!("{label}_NOT_REGULAR_FILE:{}", path.display()));
    }
    if metadata.len() > max_bytes {
        return Err(format!("{label}_TOO_LARGE:{}", path.display()));
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(max_bytes) as usize);
    File::open(path)
        .map_err(|error| format!("{label}_OPEN:{}:{error}", path.display()))?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{label}_READ:{}:{error}", path.display()))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("{label}_TOO_LARGE:{}", path.display()));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{label}_JSON:{}:{error}", path.display()))
}

fn compound_receipt_files(config: &GrowthSupervisorConfig) -> Result<Vec<PathBuf>, String> {
    let directory = compound_growth_receipts(config);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("COMPOUND_RECEIPT_DIR_READ:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("COMPOUND_RECEIPT_ENTRY:{error}"))?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file() && !kind.is_symlink())
                .unwrap_or(false)
                && entry.path().extension().and_then(OsStr::to_str) == Some("json")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn load_compound_growth_receipts(
    config: &GrowthSupervisorConfig,
) -> Result<Vec<CompoundGrowthIntegrationReceipt>, String> {
    let mut receipts = Vec::new();
    let mut repository = CompoundOperatorRepositoryIR::default();
    let mut predecessor_receipt_sha256 = None;
    for (index, path) in compound_receipt_files(config)?.into_iter().enumerate() {
        let receipt: CompoundGrowthIntegrationReceipt =
            read_bounded_compound_json(&path, MAX_COMPOUND_RECEIPT_BYTES, "COMPOUND_RECEIPT")?;
        let expected_sequence = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
        let input_sha256 = json_sha256(&receipt.input)?;
        let expected_cycle =
            run_compound_growth_input(receipt.generation, &repository, &receipt.input)?;
        if receipt.schema != COMPOUND_GROWTH_INTEGRATION_SCHEMA
            || receipt.sequence != expected_sequence
            || receipt.input.schema != COMPOUND_GROWTH_INPUT_SCHEMA
            || receipt.input_sha256 != input_sha256
            || receipt.predecessor_repository_sha256 != repository.repository_sha256
            || receipt.predecessor_receipt_sha256 != predecessor_receipt_sha256
            || receipt.cycle != expected_cycle
            || receipt.receipt_sha256 != compound_receipt_hash(&receipt)?
        {
            return Err(format!(
                "COMPOUND_GROWTH_RECEIPT_CHAIN_INVALID:{}",
                path.display()
            ));
        }
        repository = receipt.cycle.repository.clone();
        predecessor_receipt_sha256 = Some(receipt.receipt_sha256.clone());
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn pending_compound_input_files(config: &GrowthSupervisorConfig) -> Result<Vec<PathBuf>, String> {
    let directory = compound_growth_queue(config);
    fs::create_dir_all(&directory).map_err(|error| format!("COMPOUND_QUEUE_CREATE:{error}"))?;
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("COMPOUND_QUEUE_READ:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("COMPOUND_QUEUE_ENTRY:{error}"))?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file() && !kind.is_symlink())
                .unwrap_or(false)
                && entry.path().extension().and_then(OsStr::to_str) == Some("json")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn compound_growth_status_for_config(
    config: &GrowthSupervisorConfig,
) -> Result<CompoundGrowthIntegrationStatus, String> {
    let receipts = load_compound_growth_receipts(config)?;
    let latest = receipts.last();
    Ok(CompoundGrowthIntegrationStatus {
        schema: COMPOUND_GROWTH_INTEGRATION_SCHEMA.to_string(),
        cycles_committed: receipts.len().min(u64::MAX as usize) as u64,
        pending_inputs: pending_compound_input_files(config)?.len(),
        repository_profiles: latest
            .map(|receipt| receipt.cycle.repository.profiles.len())
            .unwrap_or(0),
        productive_composite_graphs: latest
            .map(|receipt| receipt.cycle.productive_composite_graphs.len())
            .unwrap_or(0),
        latest_cycle_sha256: latest.map(|receipt| receipt.cycle.cycle_sha256.clone()),
        latest_receipt_sha256: latest.map(|receipt| receipt.receipt_sha256.clone()),
        external_model_calls: latest
            .map(|receipt| receipt.cycle.external_model_calls)
            .unwrap_or(0),
        text_only_growth_events: latest
            .map(|receipt| receipt.cycle.text_only_growth_events)
            .unwrap_or(0),
    })
}

pub fn compound_growth_status(
    config_path: &Path,
) -> Result<CompoundGrowthIntegrationStatus, String> {
    let config = load_config(config_path)?;
    let _ = initialize(config_path)?;
    compound_growth_status_for_config(&config)
}

pub fn record_compound_growth_input(
    config_path: &Path,
    input: CompoundGrowthInputIR,
) -> Result<serde_json::Value, String> {
    let config = load_config(config_path)?;
    let state = initialize(config_path)?;
    let receipts = load_compound_growth_receipts(&config)?;
    let repository = receipts
        .last()
        .map(|receipt| receipt.cycle.repository.clone())
        .unwrap_or_default();
    let input_sha256 = json_sha256(&input)?;
    if let Some(existing) = receipts
        .iter()
        .find(|receipt| receipt.input.input_id == input.input_id)
    {
        if existing.input_sha256 != input_sha256 {
            return Err("COMPOUND_INPUT_ID_COLLISION".to_string());
        }
        return Ok(serde_json::json!({
            "queued": false,
            "already_committed": true,
            "input_id": input.input_id,
            "input_sha256": input_sha256,
            "receipt_sha256": existing.receipt_sha256,
        }));
    }
    // Validation is read-only. The normal Supervisor step remains the only
    // authority that can commit the resulting repository transition.
    let _ = run_compound_growth_input(state.generation, &repository, &input)?;
    let queue = pending_compound_input_files(&config)?;
    if queue.len() >= MAX_PENDING_COMPOUND_INPUTS {
        return Err("COMPOUND_INPUT_QUEUE_BOUND_REACHED".to_string());
    }
    let path = compound_growth_queue(&config).join(format!("input_{input_sha256}.json"));
    if path.exists() {
        let existing: CompoundGrowthInputIR =
            read_bounded_compound_json(&path, MAX_COMPOUND_INPUT_BYTES, "COMPOUND_INPUT")?;
        if existing != input {
            return Err("COMPOUND_INPUT_QUEUE_HASH_COLLISION".to_string());
        }
    } else {
        write_immutable_json(&path, &input)?;
    }
    Ok(serde_json::json!({
        "queued": true,
        "already_committed": false,
        "input_id": input.input_id,
        "input_sha256": input_sha256,
        "path": path,
    }))
}

fn process_pending_compound_growth(
    config: &GrowthSupervisorConfig,
    generation: u64,
) -> Result<usize, String> {
    let mut receipts = load_compound_growth_receipts(config)?;
    let mut processed = 0_usize;
    for path in pending_compound_input_files(config)?
        .into_iter()
        .take(MAX_COMPOUND_INPUTS_PER_STEP)
    {
        let input: CompoundGrowthInputIR =
            read_bounded_compound_json(&path, MAX_COMPOUND_INPUT_BYTES, "COMPOUND_INPUT")?;
        let input_sha256 = json_sha256(&input)?;
        if let Some(existing) = receipts
            .iter()
            .find(|receipt| receipt.input.input_id == input.input_id)
        {
            if existing.input_sha256 != input_sha256 {
                return Err("COMPOUND_INPUT_ID_COLLISION".to_string());
            }
            fs::remove_file(&path).map_err(|error| format!("COMPOUND_QUEUE_CONSUME:{error}"))?;
            continue;
        }
        let repository = receipts
            .last()
            .map(|receipt| receipt.cycle.repository.clone())
            .unwrap_or_default();
        let cycle = run_compound_growth_input(generation, &repository, &input)?;
        let sequence = (receipts.len().min(u64::MAX as usize) as u64).saturating_add(1);
        let mut receipt = CompoundGrowthIntegrationReceipt {
            schema: COMPOUND_GROWTH_INTEGRATION_SCHEMA.to_string(),
            sequence,
            generation,
            input,
            input_sha256,
            predecessor_repository_sha256: repository.repository_sha256.clone(),
            predecessor_receipt_sha256: receipts
                .last()
                .map(|receipt| receipt.receipt_sha256.clone()),
            cycle,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = compound_receipt_hash(&receipt)?;
        let receipt_path =
            compound_growth_receipts(config).join(format!("receipt_{sequence:020}.json"));
        write_immutable_json(&receipt_path, &receipt)?;
        receipts.push(receipt);
        fs::remove_file(&path).map_err(|error| format!("COMPOUND_QUEUE_CONSUME:{error}"))?;
        processed = processed.saturating_add(1);
    }
    Ok(processed)
}

fn validate_config(config: &GrowthSupervisorConfig) -> Result<(), String> {
    if config.schema != CONFIG_SCHEMA {
        return Err("CONFIG_SCHEMA_INVALID".to_string());
    }
    if config.watched_roots.is_empty() {
        return Err("WATCHED_ROOTS_EMPTY".to_string());
    }
    if config.poll_interval_ms < 1_000 || config.lease_stale_ms < config.poll_interval_ms * 3 {
        return Err("POLL_OR_LEASE_BOUND_INVALID".to_string());
    }
    if config.observation.retain_raw_source || config.observation.follow_symlinks {
        return Err("RAW_SOURCE_OR_SYMLINK_OBSERVATION_FORBIDDEN".to_string());
    }
    let limits = &config.resources;
    if limits.max_lifetime_campaigns == 0
        || limits.max_generations == 0
        || limits.max_active_runtime_ms == 0
        || limits.max_state_bytes < 1024 * 1024
        || limits.max_bytes_per_scan == 0
        || limits.max_files_per_scan == 0
        || limits.max_file_bytes == 0
        || limits.max_observations_per_campaign == 0
        || limits.max_pending_observations < limits.max_observations_per_campaign
        || limits.max_lessons == 0
        || limits.max_consecutive_failures == 0
        || limits.plateau_scans_before_wait == 0
    {
        return Err("RESOURCE_BOUND_INVALID".to_string());
    }
    for root in &config.watched_roots {
        if !root.is_absolute()
            || !root.is_dir()
            || root
                .components()
                .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        {
            return Err(format!("WATCH_ROOT_INVALID:{}", root.display()));
        }
    }
    for (index, root) in config.watched_roots.iter().enumerate() {
        if config
            .watched_roots
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                index != other_index && (root.starts_with(other) || other.starts_with(root))
            })
        {
            return Err("OVERLAPPING_WATCH_ROOTS_FORBIDDEN".to_string());
        }
    }
    if !config.state_dir.is_absolute()
        || config
            .state_dir
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err("STATE_DIR_MUST_BE_ABSOLUTE".to_string());
    }
    if !config.verifier_executable.is_absolute() || !config.verifier_executable.is_file() {
        return Err(format!(
            "VERIFIER_EXECUTABLE_INVALID:{}",
            config.verifier_executable.display()
        ));
    }
    validate_policy(&config.source_mutation)?;
    if config.repository_mutation.max_installations_per_step != 1 {
        return Err("REPOSITORY_MUTATION_STEP_BOUND_INVALID".to_string());
    }
    if config.source_mutation.enabled {
        let source_root = fs::canonicalize(&config.source_mutation.source_root)
            .map_err(|error| format!("SOURCE_MUTATION_ROOT_CANONICALIZE:{error}"))?;
        let inside_watch_root = config.watched_roots.iter().any(|root| {
            fs::canonicalize(root)
                .map(|watch_root| source_root.starts_with(watch_root))
                .unwrap_or(false)
        });
        if !inside_watch_root {
            return Err("SOURCE_MUTATION_ROOT_NOT_WATCHED".to_string());
        }
    }
    Ok(())
}

fn load_config(path: &Path) -> Result<GrowthSupervisorConfig, String> {
    let config: GrowthSupervisorConfig = read_json(path)?;
    validate_config(&config)?;
    Ok(config)
}

fn is_path_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn normalized_logical_path(path: &Path, roots: &[PathBuf]) -> Result<String, String> {
    for (index, root) in roots.iter().enumerate() {
        if let Ok(relative) = path.strip_prefix(root) {
            let text = relative
                .components()
                .filter_map(|component| match component {
                    Component::Normal(value) => Some(value.to_string_lossy().replace('\\', "/")),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/");
            return Ok(format!("ROOT_{index}/{text}"));
        }
    }
    Err(format!("PATH_OUTSIDE_WATCH_ROOTS:{}", path.display()))
}

fn path_is_secret(path: &Path, policy: &ObservationPolicy) -> bool {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if policy
        .excluded_file_names
        .iter()
        .any(|excluded| name == excluded.to_ascii_lowercase())
    {
        return true;
    }
    [".pem", ".pfx", ".p12", ".key", ".keystore"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        || name.contains("secret")
        || name.contains("credential")
        || name.contains("private_key")
        || name.contains("access_token")
}

fn directory_is_excluded(path: &Path, policy: &ObservationPolicy) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    policy
        .excluded_directory_names
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
}

fn extension_allowed(path: &Path, policy: &ObservationPolicy) -> bool {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    policy
        .allowed_extensions
        .iter()
        .any(|allowed| extension == allowed.to_ascii_lowercase())
}

fn collect_files(
    roots: &[PathBuf],
    policy: &ObservationPolicy,
    max_files: usize,
) -> Result<Vec<PathBuf>, String> {
    let mut pending = roots.to_vec();
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("READ_DIR:{}:{error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("READ_DIR_ENTRY:{}:{error}", directory.display()))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("FILE_TYPE:{}:{error}", entry.path().display()))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                if !directory_is_excluded(&path, policy) {
                    pending.push(path);
                }
            } else if file_type.is_file()
                && extension_allowed(&path, policy)
                && !path_is_secret(&path, policy)
            {
                files.push(path);
                if files.len() > max_files {
                    return Err("MAX_FILES_PER_SCAN_EXCEEDED".to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn count_any(text: &str, needles: &[&str]) -> u32 {
    needles
        .iter()
        .map(|needle| text.matches(needle).count() as u32)
        .sum()
}

fn collect_rust_identifiers(stream: proc_macro2::TokenStream, identifiers: &mut Vec<String>) {
    for token in stream {
        match token {
            proc_macro2::TokenTree::Ident(ident) => {
                identifiers.push(
                    ident
                        .to_string()
                        .trim_start_matches("r#")
                        .to_ascii_lowercase(),
                );
            }
            proc_macro2::TokenTree::Group(group) => {
                collect_rust_identifiers(group.stream(), identifiers);
            }
            // Literal contents and comments are deliberately absent from the
            // semantic surface. Test fixtures that quote source code must not
            // masquerade as executable branches, repairs, or assertions.
            proc_macro2::TokenTree::Literal(_) | proc_macro2::TokenTree::Punct(_) => {}
        }
    }
}

fn rust_semantic_identifiers(text: &str) -> Option<Vec<String>> {
    let stream = text.parse::<proc_macro2::TokenStream>().ok()?;
    let mut identifiers = Vec::new();
    collect_rust_identifiers(stream, &mut identifiers);
    Some(identifiers)
}

fn count_identifiers(identifiers: &[String], predicate: impl Fn(&str) -> bool) -> u32 {
    identifiers
        .iter()
        .filter(|identifier| predicate(identifier))
        .count()
        .min(u32::MAX as usize) as u32
}

fn rust_structural_features(text: &str, identifiers: &[String]) -> StructuralFeatures {
    let lines = text.lines().collect::<Vec<_>>();
    let public_symbols = identifiers
        .windows(2)
        .filter(|pair| {
            pair[0] == "pub"
                && matches!(
                    pair[1].as_str(),
                    "fn" | "struct" | "enum" | "trait" | "type" | "const" | "static" | "mod"
                )
        })
        .count()
        .min(u32::MAX as usize) as u32;
    StructuralFeatures {
        lines: lines.len().min(u32::MAX as usize) as u32,
        non_empty_lines: lines.iter().filter(|line| !line.trim().is_empty()).count() as u32,
        public_symbols,
        branch_tokens: count_identifiers(identifiers, |value| {
            matches!(value, "if" | "match" | "else")
        }),
        assertion_tokens: count_identifiers(identifiers, |value| {
            value.starts_with("assert")
                || matches!(
                    value,
                    "expect" | "should" | "toequal" | "pytest" | "unittest"
                )
        }),
        test_tokens: count_identifiers(identifiers, |value| matches!(value, "test" | "tests")),
        validation_tokens: count_identifiers(identifiers, |value| {
            [
                "validate",
                "invalid",
                "guard",
                "sanitize",
                "constraint",
                "schema",
            ]
            .iter()
            .any(|marker| value.contains(marker))
        }),
        error_handling_tokens: count_identifiers(identifiers, |value| {
            matches!(
                value,
                "result" | "error" | "exception" | "catch" | "map_err" | "rollback" | "retry"
            ) || value.ends_with("error")
        }),
        documentation_tokens: count_identifiers(identifiers, |value| {
            matches!(value, "doc" | "readme" | "docstring")
        }),
        todo_tokens: count_identifiers(identifiers, |value| {
            matches!(value, "todo" | "fixme" | "hack" | "unimplemented")
        }),
        benchmark_tokens: count_identifiers(identifiers, |value| {
            ["benchmark", "criterion", "bench", "latency", "throughput"]
                .iter()
                .any(|marker| value.contains(marker))
        }),
        performance_tokens: count_identifiers(identifiers, |value| {
            ["allocation", "elapsed", "duration", "memoiz", "cache"]
                .iter()
                .any(|marker| value.contains(marker))
        }),
        algebraic_constructor_tokens: count_identifiers(identifiers, |value| {
            matches!(value, "some" | "none" | "ok" | "err" | "option" | "result")
        }),
        data_composition_tokens: count_identifiers(identifiers, |value| {
            matches!(
                value,
                "format" | "to_string" | "collect" | "map" | "and_then" | "chain"
            )
        }),
        max_line_bytes: lines
            .iter()
            .map(|line| line.len().min(u32::MAX as usize) as u32)
            .max()
            .unwrap_or(0),
    }
}

fn structural_features(text: &str, extension: &str) -> StructuralFeatures {
    if extension.eq_ignore_ascii_case("rs") {
        if let Some(identifiers) = rust_semantic_identifiers(text) {
            return rust_structural_features(text, &identifiers);
        }
    }
    let lower = text.to_ascii_lowercase();
    let lines = text.lines().collect::<Vec<_>>();
    StructuralFeatures {
        lines: lines.len().min(u32::MAX as usize) as u32,
        non_empty_lines: lines.iter().filter(|line| !line.trim().is_empty()).count() as u32,
        public_symbols: count_any(
            &lower,
            &[
                "pub fn ",
                "public ",
                "export function",
                "export const",
                "def ",
                "func ",
            ],
        ),
        branch_tokens: count_any(&lower, &["if ", "match ", "switch ", "else ", "case "]),
        assertion_tokens: count_any(
            &lower,
            &[
                "assert", "expect(", "should(", "toequal", "pytest", "unittest",
            ],
        ),
        test_tokens: count_any(
            &lower,
            &[
                "#[test]",
                "mod tests",
                "test(",
                "it(",
                "describe(",
                "@test",
                "_test",
            ],
        ),
        validation_tokens: count_any(
            &lower,
            &[
                "validate",
                "invalid",
                "guard",
                "sanitize",
                "constraint",
                "schema",
            ],
        ),
        error_handling_tokens: count_any(
            &lower,
            &[
                "result<",
                "error",
                "exception",
                "catch ",
                "map_err",
                "rollback",
                "retry",
            ],
        ),
        documentation_tokens: count_any(&lower, &["///", "//!", "# ", "readme", "docstring"]),
        todo_tokens: count_any(&lower, &["todo", "fixme", "hack", "unimplemented"]),
        benchmark_tokens: count_any(
            &lower,
            &[
                "benchmark",
                "criterion",
                "bench_function",
                "latency",
                "throughput",
            ],
        ),
        performance_tokens: count_any(
            &lower,
            &[
                "allocation",
                "elapsed",
                "duration",
                "memoiz",
                "cache hit",
                "cache miss",
            ],
        ),
        algebraic_constructor_tokens: count_any(
            &lower,
            &["some(", "none", "ok(", "err(", "option<", "result<"],
        ),
        data_composition_tokens: count_any(
            &lower,
            &[
                "format!(",
                ".to_string()",
                ".collect(",
                ".map(",
                ".and_then(",
                ".chain(",
            ],
        ),
        max_line_bytes: lines
            .iter()
            .map(|line| line.len().min(u32::MAX as usize) as u32)
            .max()
            .unwrap_or(0),
    }
}

fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn fingerprint_file_with_metadata(
    path: &Path,
    metadata: &fs::Metadata,
    max_bytes: u64,
) -> Result<Option<FileFingerprint>, String> {
    if metadata.len() > max_bytes {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| format!("READ:{}:{error}", path.display()))?;
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok(None);
    };
    let lower_head = text
        .chars()
        .take(512)
        .collect::<String>()
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let features = structural_features(text, &extension);
    if lower_head.contains("do not edit")
        || lower_head.contains("auto-generated")
        || lower_head.contains("automatically generated")
        || features.max_line_bytes > 20_000
    {
        return Ok(None);
    }
    Ok(Some(FileFingerprint {
        content_sha256: sha256(&bytes),
        bytes: metadata.len(),
        modified_ms: modified_ms(metadata),
        extension,
        features,
    }))
}

fn path_is_dedicated_test(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let normalized = lower.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or("");
    normalized.contains("/tests/")
        || normalized.contains("/test/")
        || file_name.starts_with("test_")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with("_test.py")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".spec.js")
        || file_name.ends_with(".spec.ts")
}

fn classify_work_kind(
    path: &Path,
    features: &StructuralFeatures,
    previous: Option<&StructuralFeatures>,
) -> WorkKind {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let prior = previous.cloned().unwrap_or_default();
    let test_delta = features.test_tokens.saturating_sub(prior.test_tokens);
    let performance_delta = features
        .benchmark_tokens
        .saturating_sub(prior.benchmark_tokens)
        .saturating_add(
            features
                .performance_tokens
                .saturating_sub(prior.performance_tokens),
        );
    let implementation_delta = features
        .public_symbols
        .saturating_sub(prior.public_symbols)
        .saturating_add(features.branch_tokens.saturating_sub(prior.branch_tokens))
        .saturating_add(
            features
                .validation_tokens
                .saturating_sub(prior.validation_tokens),
        )
        .saturating_add(
            features
                .error_handling_tokens
                .saturating_sub(prior.error_handling_tokens),
        );
    if path_is_dedicated_test(path) || (test_delta > 0 && implementation_delta == 0) {
        WorkKind::RegressionTest
    } else if performance_delta > 0 {
        WorkKind::PerformanceOptimization
    } else if [".tsx", ".jsx", ".css", ".scss", ".html"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
    {
        WorkKind::FrontendChange
    } else if lower.contains("deploy")
        || lower.contains("docker")
        || lower.contains("infra")
        || lower.ends_with(".ps1")
        || lower.ends_with(".sh")
    {
        WorkKind::OperationsChange
    } else if lower.ends_with(".md") {
        WorkKind::Documentation
    } else if features
        .error_handling_tokens
        .saturating_sub(prior.error_handling_tokens)
        > 0
        || features
            .validation_tokens
            .saturating_sub(prior.validation_tokens)
            > 0
    {
        WorkKind::DefectRepair
    } else {
        WorkKind::CodeChange
    }
}

fn event_path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\");
    let ordinary = text.strip_prefix("\\\\?\\").unwrap_or(&text);
    if cfg!(windows) {
        ordinary.to_ascii_lowercase()
    } else {
        ordinary.to_string()
    }
}

fn work_event_path_index(events: &[WorkEvent]) -> BTreeMap<String, usize> {
    // Canonicalize only the small pending-event side once. Watched paths are
    // already absolute, non-symlink directory entries and use their normalized
    // in-memory keys during every scan.
    let mut index = BTreeMap::new();
    for (event_index, event) in events.iter().enumerate() {
        for path in &event.paths {
            index.insert(event_path_key(path), event_index);
            if let Ok(canonical) = fs::canonicalize(path) {
                index.insert(event_path_key(&canonical), event_index);
            }
        }
    }
    index
}

fn classify_observation(
    logical_path: String,
    current: &FileFingerprint,
    previous: Option<&FileFingerprint>,
    event: Option<&WorkEvent>,
    classifier: &ClassifierMemory,
    minimum_score: u16,
) -> LearningObservation {
    let mut score: i32 = 0;
    let mut signals = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut reasons = Vec::new();
    let kind = event.map(|value| value.kind).unwrap_or_else(|| {
        classify_work_kind(
            Path::new(&logical_path),
            &current.features,
            previous.map(|value| &value.features),
        )
    });
    let actor = event
        .map(|value| value.actor)
        .unwrap_or(WorkActor::UnknownLocalWriter);
    let outcome = event
        .map(|value| value.outcome)
        .unwrap_or(WorkOutcome::Unknown);
    let mut performance_metrics = event
        .map(|value| value.performance_metrics.clone())
        .unwrap_or_default();
    for metric in &mut performance_metrics {
        if !metric.executable_for_transition(
            previous.map(|value| value.content_sha256.as_str()),
            &current.content_sha256,
        ) {
            metric.executable_knowledge = None;
        }
    }

    match kind {
        WorkKind::RegressionTest | WorkKind::Verification => {
            score += 22;
            signals.insert("REGRESSION_EVIDENCE".to_string());
            roles.insert("REGRESSION_TEST".to_string());
        }
        WorkKind::DefectRepair => {
            score += 20;
            signals.insert("DEFECT_REPAIR".to_string());
            roles.insert("IMPLEMENTATION_REPAIR".to_string());
        }
        WorkKind::PerformanceOptimization => {
            score += 20;
            signals.insert("PERFORMANCE_OPTIMIZATION".to_string());
            roles.insert("PERFORMANCE_IMPLEMENTATION".to_string());
        }
        WorkKind::FrontendChange => {
            score += 13;
            signals.insert("FRONTEND_CONTRACT".to_string());
            roles.insert("FRONTEND_CONSUMER".to_string());
        }
        WorkKind::BackendChange => {
            score += 13;
            signals.insert("BACKEND_CONTRACT".to_string());
            roles.insert("BACKEND_PROVIDER".to_string());
        }
        WorkKind::OperationsChange => {
            score += 13;
            signals.insert("OPERATIONS_CHANGE".to_string());
            roles.insert("OPERATIONS_GUARD".to_string());
        }
        WorkKind::Refactor => {
            score += 10;
            signals.insert("REFACTOR".to_string());
            roles.insert("IMPLEMENTATION".to_string());
        }
        WorkKind::CodeChange => {
            score += 10;
            signals.insert("CODE_CHANGE".to_string());
            roles.insert("IMPLEMENTATION".to_string());
        }
        WorkKind::CapabilitySynthesis => {
            score += 22;
            signals.insert("BEHAVIORAL_FRONTIER_ADVANCE".to_string());
            roles.insert("PROGRAM_COMPOSITION".to_string());
        }
        WorkKind::Documentation => {
            score += 2;
            signals.insert("DOCUMENTATION_ONLY".to_string());
            roles.insert("DOCUMENTATION".to_string());
        }
    }

    let pass_evidence_bound = event.is_some_and(|value| !value.evidence_sha256.is_empty());
    if outcome == WorkOutcome::Pass && pass_evidence_bound {
        score += 25;
        signals.insert("VERIFIED_PASS".to_string());
        reasons.push("explicit local PASS is bound to hashed evidence artifacts".to_string());
    } else if outcome == WorkOutcome::Pass {
        score -= 35;
        signals.insert("UNBOUND_PASS_REJECTED".to_string());
        reasons.push("PASS without a bound evidence artifact is not verification".to_string());
    } else if outcome == WorkOutcome::Fail {
        score -= 35;
        signals.insert("OBSERVED_FAILURE".to_string());
        reasons.push("failed work is retained only as negative evidence".to_string());
    }

    if matches!(actor, WorkActor::User | WorkActor::Codex) {
        score += 2;
        signals.insert("ATTRIBUTED_WORK_EVENT".to_string());
    }

    let before = previous.map(|value| &value.features);
    let delta = |after: u32, before_value: u32| after.saturating_sub(before_value);
    let prior = before.cloned().unwrap_or_default();
    if delta(current.features.test_tokens, prior.test_tokens) > 0 {
        score += 16;
        signals.insert("TEST_ADDED".to_string());
        roles.insert("REGRESSION_TEST".to_string());
    }
    if delta(current.features.assertion_tokens, prior.assertion_tokens) > 0 {
        score += 10;
        signals.insert("ASSERTION_STRENGTHENED".to_string());
        roles.insert("INVARIANT_CHECK".to_string());
    }
    if delta(current.features.validation_tokens, prior.validation_tokens) > 0 {
        score += 8;
        signals.insert("VALIDATION_ADDED".to_string());
        roles.insert("INPUT_VALIDATION".to_string());
    }
    if delta(
        current.features.error_handling_tokens,
        prior.error_handling_tokens,
    ) > 0
    {
        score += 8;
        signals.insert("ERROR_HANDLING_ADDED".to_string());
        roles.insert("ERROR_PROPAGATION".to_string());
    }
    if delta(current.features.public_symbols, prior.public_symbols) > 0 {
        score += 5;
        signals.insert("CAPABILITY_SURFACE_ADDED".to_string());
    }
    if delta(current.features.benchmark_tokens, prior.benchmark_tokens) > 0 {
        score += 12;
        signals.insert("BENCHMARK_EVIDENCE".to_string());
        roles.insert("PERFORMANCE_BENCHMARK".to_string());
    }
    if delta(
        current.features.performance_tokens,
        prior.performance_tokens,
    ) > 0
    {
        score += 8;
        signals.insert("EFFICIENCY_MECHANISM".to_string());
        roles.insert("PERFORMANCE_IMPLEMENTATION".to_string());
    }
    if delta(
        current.features.algebraic_constructor_tokens,
        prior.algebraic_constructor_tokens,
    ) > 0
    {
        score += 6;
        signals.insert("ALGEBRAIC_CONSTRUCTOR_MECHANISM".to_string());
        roles.insert("IMPLEMENTATION".to_string());
    }
    if delta(
        current.features.data_composition_tokens,
        prior.data_composition_tokens,
    ) > 0
    {
        score += 6;
        signals.insert("DATA_COMPOSITION_MECHANISM".to_string());
        roles.insert("IMPLEMENTATION".to_string());
    }
    if event.is_some() {
        for metric in performance_metrics
            .iter()
            .filter(|metric| metric.improved())
        {
            score += 25;
            signals.insert("MEASURED_PERFORMANCE_GAIN".to_string());
            signals.insert(format!(
                "PERFORMANCE_METRIC:{}:{}",
                metric.metric.to_ascii_uppercase(),
                if metric.lower_is_better {
                    "LOWER_IS_BETTER"
                } else {
                    "HIGHER_IS_BETTER"
                }
            ));
            roles.insert("PERFORMANCE_BENCHMARK".to_string());
            reasons.push(format!(
                "bound metric {} improved from {} to {}",
                metric.metric, metric.before, metric.after
            ));
        }
        if !performance_metrics.is_empty()
            && !performance_metrics.iter().any(|metric| metric.improved())
        {
            score -= 30;
            signals.insert("PERFORMANCE_GAIN_NOT_OBSERVED".to_string());
            reasons.push("bound before/after metrics did not improve".to_string());
        }
    }
    if current.features.todo_tokens > prior.todo_tokens {
        score -= 10;
        signals.insert("UNRESOLVED_MARKER_ADDED".to_string());
    }
    let line_delta = current.features.lines.abs_diff(prior.lines);
    if line_delta > 1_000 {
        score -= 12;
        signals.insert("OVERSIZED_CHANGE".to_string());
    }
    for signal in &signals {
        score += i32::from(*classifier.signal_weights.get(signal).unwrap_or(&0));
    }
    score = score.clamp(0, 100);
    let learning_score = score as u16;
    let learning_value = if outcome == WorkOutcome::Fail {
        LearningValue::Rejected
    } else if learning_score >= minimum_score {
        LearningValue::High
    } else if learning_score >= 25 {
        LearningValue::Medium
    } else if learning_score > 0 {
        LearningValue::Low
    } else {
        LearningValue::Rejected
    };
    reasons.push(format!("bounded structural score={learning_score}"));
    // Observation identity must bind the classified semantics, not merely the
    // file transition and optional event name. Otherwise a retry after a
    // classifier/evidence change reuses the old id for different content and
    // an immutable observation store correctly reports a collision. Bind a
    // deterministic timestamp (event time or file mtime), then hash the full
    // semantic postimage with its id field empty.
    let mut observation = LearningObservation {
        observation_id: String::new(),
        work_event_id: event.map(|value| value.event_id.clone()),
        logical_path,
        content_sha256: current.content_sha256.clone(),
        predecessor_content_sha256: previous.map(|value| value.content_sha256.clone()),
        actor,
        work_kind: kind,
        work_outcome: outcome,
        features_before: before.cloned(),
        features_after: current.features.clone(),
        signals: signals.into_iter().collect(),
        composition_roles: roles.into_iter().collect(),
        learning_score,
        learning_value,
        reasons,
        verification_evidence_sha256: event
            .map(|value| value.evidence_sha256.clone())
            .unwrap_or_default(),
        performance_metrics,
        public_contract_deltas: event
            .map(|value| value.public_contract_deltas.clone())
            .unwrap_or_default(),
        exact_source_fragments_stored: 0,
        raw_source_bytes_stored: 0,
        observed_at_ms: event
            .map(|value| value.occurred_at_ms)
            .filter(|value| *value != 0)
            .unwrap_or(current.modified_ms),
    };
    let identity_bytes = serde_json::to_vec(&observation).unwrap_or_else(|_| {
        format!(
            "{}:{}:{}:{}",
            observation.logical_path,
            observation.content_sha256,
            observation
                .predecessor_content_sha256
                .as_deref()
                .unwrap_or("NEW"),
            observation.work_event_id.as_deref().unwrap_or("PASSIVE")
        )
        .into_bytes()
    });
    observation.observation_id = sha256(
        [
            b"B_CORE_LEARNING_OBSERVATION_2:".as_slice(),
            identity_bytes.as_slice(),
        ]
        .concat()
        .as_slice(),
    );
    observation
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TransitionRecord {
    schema: String,
    sequence: u64,
    from: SupervisorPhase,
    to: SupervisorPhase,
    reason: String,
    state_sha256: String,
    occurred_at_ms: u64,
}

#[derive(Debug)]
struct ScanResult {
    index: FileIndex,
    baseline_created: bool,
    files_scanned: usize,
    bytes_observed: u64,
    observations: Vec<LearningObservation>,
    files_reused: usize,
    files_hashed: usize,
    pending_work_events: usize,
    replayed_unchanged_work_events: usize,
}

struct SupervisorLease {
    path: PathBuf,
}

impl SupervisorLease {
    fn acquire(config: &GrowthSupervisorConfig) -> Result<Self, String> {
        let control = config.state_dir.join("control");
        fs::create_dir_all(&control).map_err(|error| format!("CONTROL_DIR:{error}"))?;
        let path = control.join("supervisor.lease");
        if path.exists() {
            let age = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
                .unwrap_or(0);
            if age <= config.lease_stale_ms {
                return Err("SUPERVISOR_ALREADY_RUNNING_OR_LEASE_RECENT".to_string());
            }
            fs::remove_file(&path).map_err(|error| format!("STALE_LEASE_REMOVE:{error}"))?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("LEASE_CREATE:{error}"))?;
        writeln!(
            file,
            "pid={}
heartbeat_ms={}",
            std::process::id(),
            now_ms()
        )
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("LEASE_WRITE:{error}"))?;
        Ok(Self { path })
    }

    fn heartbeat(&self) -> Result<(), String> {
        let bytes = format!("pid={}\nheartbeat_ms={}\n", std::process::id(), now_ms());
        fs::write(&self.path, bytes).map_err(|error| format!("LEASE_HEARTBEAT:{error}"))
    }
}

impl Drop for SupervisorLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn make_config(
    config_path: &Path,
    watch_root: &Path,
    state_dir: &Path,
) -> Result<GrowthSupervisorConfig, String> {
    let watch_root = fs::canonicalize(watch_root)
        .map_err(|error| format!("WATCH_ROOT_CANONICALIZE:{}:{error}", watch_root.display()))?;
    let state_dir = if state_dir.is_absolute() {
        state_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(state_dir)
    };
    fs::create_dir_all(&state_dir).map_err(|error| format!("STATE_DIR_CREATE:{error}"))?;
    let state_dir =
        fs::canonicalize(&state_dir).map_err(|error| format!("STATE_DIR_CANONICALIZE:{error}"))?;
    let current = std::env::current_exe().map_err(|error| format!("CURRENT_EXE:{error}"))?;
    let verifier_name = if cfg!(windows) {
        "b-core-growth-verifier.exe"
    } else {
        "b-core-growth-verifier"
    };
    let verifier = current
        .parent()
        .ok_or_else(|| "CURRENT_EXE_PARENT_MISSING".to_string())?
        .join(verifier_name);
    if !verifier.is_file() {
        return Err(format!("SIBLING_VERIFIER_MISSING:{}", verifier.display()));
    }
    let config = GrowthSupervisorConfig::bounded_default(state_dir, watch_root, verifier);
    validate_config(&config)?;
    write_immutable_json(config_path, &config)?;
    Ok(config)
}

fn config_hash(config: &GrowthSupervisorConfig) -> Result<String, String> {
    json_sha256(config)
}

fn latest_numbered_file(directory: &Path, prefix: &str) -> Result<Option<PathBuf>, String> {
    if !directory.exists() {
        return Ok(None);
    }
    let mut candidates = fs::read_dir(directory)
        .map_err(|error| format!("READ_DIR:{}:{error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().ok()?.is_file()
                && name.starts_with(prefix)
                && name.ends_with(".json")
            {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    candidates.sort();
    Ok(candidates.pop())
}

fn cleanup_numbered_files(directory: &Path, prefix: &str, keep: usize) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    let mut candidates = fs::read_dir(directory)
        .map_err(|error| format!("READ_DIR:{}:{error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            (entry.file_type().ok()?.is_file()
                && name.starts_with(prefix)
                && name.ends_with(".json"))
            .then_some(entry.path())
        })
        .collect::<Vec<_>>();
    candidates.sort();
    let remove_count = candidates.len().saturating_sub(keep);
    for path in candidates.into_iter().take(remove_count) {
        fs::remove_file(&path).map_err(|error| format!("CLEANUP:{}:{error}", path.display()))?;
    }
    Ok(())
}

fn cleanup_recent_files(directory: &Path, prefix: &str, keep: usize) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    let mut candidates = fs::read_dir(directory)
        .map_err(|error| format!("READ_DIR:{}:{error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().ok()?.is_file()
                || !name.starts_with(prefix)
                || !name.ends_with(".json")
            {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let remove_count = candidates.len().saturating_sub(keep);
    for (_, path) in candidates.into_iter().take(remove_count) {
        fs::remove_file(&path).map_err(|error| format!("CLEANUP:{}:{error}", path.display()))?;
    }
    Ok(())
}

fn load_state(config: &GrowthSupervisorConfig) -> Result<SupervisorState, String> {
    let path = latest_numbered_file(&config.state_dir.join("state"), "state_")?
        .ok_or_else(|| "SUPERVISOR_NOT_INITIALIZED".to_string())?;
    let mut state: SupervisorState = read_json(&path)?;
    state
        .intrinsic_drive
        .ensure_post_promotion_reward_contract();
    if state.schema != SUPERVISOR_SCHEMA
        || state.config_sha256 != config_hash(config)?
        || !state.intrinsic_drive.is_valid()
    {
        return Err("STATE_OR_CONFIG_INTEGRITY_FAILURE".to_string());
    }
    Ok(state)
}

fn save_transition(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    to: SupervisorPhase,
    reason: &str,
) -> Result<(), String> {
    let from = state.phase;
    state.sequence = state.sequence.saturating_add(1);
    state.phase = to;
    state.last_transition_ms = now_ms();
    let state_hash = json_sha256(state)?;
    let state_path = config
        .state_dir
        .join("state")
        .join(format!("state_{:020}.json", state.sequence));
    write_immutable_json(&state_path, state)?;
    let transition = TransitionRecord {
        schema: SUPERVISOR_SCHEMA.to_string(),
        sequence: state.sequence,
        from,
        to,
        reason: reason.to_string(),
        state_sha256: state_hash,
        occurred_at_ms: state.last_transition_ms,
    };
    write_immutable_json(
        &config
            .state_dir
            .join("journal")
            .join(format!("transition_{:020}.json", state.sequence)),
        &transition,
    )?;
    cleanup_numbered_files(&config.state_dir.join("state"), "state_", 2)
}

fn load_index(config: &GrowthSupervisorConfig) -> Result<FileIndex, String> {
    latest_numbered_file(&config.state_dir.join("index"), "index_")?
        .map(|path| read_json(&path))
        .transpose()
        .map(|value| value.unwrap_or_default())
}

fn save_index(config: &GrowthSupervisorConfig, index: &mut FileIndex) -> Result<(), String> {
    index.sequence = index.sequence.saturating_add(1);
    write_immutable_json(
        &config
            .state_dir
            .join("index")
            .join(format!("index_{:020}.json", index.sequence)),
        index,
    )?;
    cleanup_numbered_files(&config.state_dir.join("index"), "index_", 2)
}

fn memory_path(config: &GrowthSupervisorConfig, generation: u64) -> PathBuf {
    config
        .state_dir
        .join("memory")
        .join(format!("generation_{generation:020}.json"))
}

fn load_memory(config: &GrowthSupervisorConfig, generation: u64) -> Result<GrowthMemory, String> {
    let memory: GrowthMemory = read_json(&memory_path(config, generation))?;
    if memory.schema != SUPERVISOR_SCHEMA || memory.generation != generation {
        return Err("MEMORY_INTEGRITY_FAILURE".to_string());
    }
    Ok(memory)
}

fn cleanup_memory_generations(config: &GrowthSupervisorConfig) -> Result<(), String> {
    cleanup_numbered_files(&config.state_dir.join("memory"), "generation_", 2)
}

fn invalidated_generation_dir(config: &GrowthSupervisorConfig, generation: u64) -> PathBuf {
    config
        .state_dir
        .join("invalidated_generations")
        .join(format!("generation_{generation:020}"))
}

fn restore_memory_projection(
    state: &mut SupervisorState,
    memory: &GrowthMemory,
) -> Result<(), String> {
    state.generation = memory.generation;
    state.current_memory_sha256 = json_sha256(memory)?;
    state.predecessor_memory_sha256 = memory.predecessor_sha256.clone();
    state.evaluator_generation = memory.evaluator.generation;
    state.current_evaluator_memory_sha256 = json_sha256(&memory.evaluator)?;
    state.evaluator_challenge_cases = memory
        .evaluator
        .challenge_suite
        .len()
        .min(u64::MAX as usize) as u64;
    state.generative_predictions = memory.generative.prediction_records;
    state.valuable_combinations_learned = memory.generative.distinct_verified_artifact_count();
    state.generative_memory_reuse_events = memory.generative.reuse_events;
    state.generative_self_application_events = memory.generative.self_application_events;
    state.generative_exploration_events = memory.generative.exploration_events;
    state.productive_generative_reuse_events = memory.generative.productive_reuse_events;
    state.generative_frontier_advance_events = memory.generative.frontier_advance_events;
    state.generative_frontier_capability_units = memory.generative.frontier_capability_units;
    state.unverified_generative_frontier_candidate_events =
        memory.generative.unverified_frontier_candidate_events;
    state.legacy_unverified_generative_frontier_advance_events =
        memory.generative.legacy_unverified_frontier_advance_events;
    state.legacy_wrapper_generative_frontier_advance_events =
        memory.generative.legacy_wrapper_frontier_advance_events;
    state.generative_behavioral_verification_events =
        memory.generative.behavioral_verification_events;
    state.redundant_generative_selection_events = memory.generative.redundant_selection_events;
    state.generative_prediction_absolute_error_total =
        memory.generative.prediction_absolute_error_total;
    state.generative_calibrated_prediction_records =
        memory.generative.calibrated_prediction_records;
    state.generative_legacy_uncalibrated_prediction_error_total =
        memory.generative.legacy_uncalibrated_prediction_error_total;
    let (distinct_semantic_lessons, semantic_duplicate_lessons) = semantic_lesson_counts(memory)?;
    state.distinct_semantic_lessons = distinct_semantic_lessons;
    state.semantic_duplicate_lessons = semantic_duplicate_lessons;
    state.measured_performance_promotions = executable_performance_promotion_count(memory);
    state.classifier_outcome_bound_refinements = memory.classifier.outcome_bound_refinements;
    state.classifier_unsupported_refinements_suppressed =
        memory.classifier.unsupported_refinements_suppressed;
    Ok(())
}

fn cleanup_recovered_invalid_successor(
    config: &GrowthSupervisorConfig,
    state: &SupervisorState,
) -> Result<(), String> {
    let invalid_generation = state.generation.saturating_add(1);
    let directory = invalidated_generation_dir(config, invalid_generation);
    let receipt_path = directory.join("recovery_receipt.json");
    let canonical_path = memory_path(config, invalid_generation);
    if !receipt_path.exists() || !canonical_path.exists() {
        return Ok(());
    }
    let receipt: InvalidGenerationRecoveryReceipt = read_json(&receipt_path)?;
    if receipt.invalid_generation != invalid_generation
        || receipt.restored_generation != state.generation
        || receipt.restored_memory_sha256 != state.current_memory_sha256
        || file_sha256(&canonical_path, 64 * 1024 * 1024)?
            != file_sha256(&directory.join("memory.json"), 64 * 1024 * 1024)?
    {
        return Err("INVALIDATED_SUCCESSOR_CLEANUP_BINDING_FAILURE".to_string());
    }
    fs::remove_file(&canonical_path).map_err(|error| {
        format!(
            "INVALIDATED_MEMORY_DELETE:{}:{error}",
            canonical_path.display()
        )
    })
}

fn recover_verification_only_generation_tip(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
) -> Result<bool, String> {
    cleanup_recovered_invalid_successor(config, state)?;
    if state.generation == 0 || state.pending_campaign_id.is_some() {
        return Ok(false);
    }
    let invalid_memory = load_memory(config, state.generation)?;
    let invalid_memory_sha256 = json_sha256(&invalid_memory)?;
    if invalid_memory_sha256 != state.current_memory_sha256 {
        return Err("CURRENT_MEMORY_HASH_MISMATCH_DURING_TIP_RECOVERY".to_string());
    }
    let Some(invalid_lesson) = invalid_memory.lessons.last() else {
        return Ok(false);
    };
    if lesson_has_growth_subject(invalid_lesson) {
        return Ok(false);
    }
    let restored_memory = load_memory(config, state.generation.saturating_sub(1))?;
    let restored_memory_sha256 = json_sha256(&restored_memory)?;
    if invalid_memory.predecessor_sha256.as_deref() != Some(restored_memory_sha256.as_str())
        || invalid_memory.generation != restored_memory.generation.saturating_add(1)
    {
        return Err("VERIFICATION_ONLY_TIP_PREDECESSOR_BINDING_FAILURE".to_string());
    }

    let directory = invalidated_generation_dir(config, invalid_memory.generation);
    let quarantined_memory_path = directory.join("memory.json");
    if quarantined_memory_path.exists() {
        let existing: GrowthMemory = read_json(&quarantined_memory_path)?;
        if json_sha256(&existing)? != invalid_memory_sha256 {
            return Err("INVALIDATED_MEMORY_QUARANTINE_DIVERGENCE".to_string());
        }
    } else {
        write_immutable_json(&quarantined_memory_path, &invalid_memory)?;
    }
    let receipt_path = directory.join("recovery_receipt.json");
    let receipt = InvalidGenerationRecoveryReceipt {
        schema: SUPERVISOR_SCHEMA.to_string(),
        invalid_generation: invalid_memory.generation,
        invalid_memory_sha256,
        invalid_lesson_id: invalid_lesson.lesson_id.clone(),
        restored_generation: restored_memory.generation,
        restored_memory_sha256,
        reason: "VERIFICATION_RECEIPT_WITHOUT_GROWTH_SUBJECT_WAS_FALSELY_PROMOTED".to_string(),
        recovered_at_ms: now_ms(),
    };
    if receipt_path.exists() {
        let existing: InvalidGenerationRecoveryReceipt = read_json(&receipt_path)?;
        if existing.invalid_generation != receipt.invalid_generation
            || existing.invalid_memory_sha256 != receipt.invalid_memory_sha256
            || existing.restored_generation != receipt.restored_generation
            || existing.restored_memory_sha256 != receipt.restored_memory_sha256
        {
            return Err("INVALIDATED_GENERATION_RECEIPT_DIVERGENCE".to_string());
        }
    } else {
        write_immutable_json(&receipt_path, &receipt)?;
    }

    restore_memory_projection(state, &restored_memory)?;
    state.campaigns_accepted = state.campaigns_accepted.saturating_sub(1);
    state.campaigns_failed = state.campaigns_failed.saturating_add(1);
    state.mutual_revalidation_events = state.mutual_revalidation_events.saturating_sub(1);
    state.consecutive_failures = 0;
    state.plateau_scans = 0;
    state.pending_campaign_id = None;
    if state.diagnostic_policy.active_generation == Some(invalid_memory.generation) {
        state.diagnostic_policy.active_experiment_id = None;
        state.diagnostic_policy.active_generation = None;
        state.diagnostic_policy.active_observations = 0;
        state.diagnostic_policy.active_causal_support = false;
        state.diagnostic_policy.active_action_id = None;
        state.diagnostic_policy.active_action_receipt_sha256 = None;
        state
            .diagnostic_policy
            .active_output_observation_ids
            .clear();
    }
    let preserved_phase = state.phase;
    save_transition(
        config,
        state,
        preserved_phase,
        "VERIFICATION_ONLY_FALSE_GENERATION_ROLLED_BACK",
    )?;
    cleanup_recovered_invalid_successor(config, state)?;
    Ok(true)
}

fn next_queued_source_patch(
    config: &GrowthSupervisorConfig,
) -> Result<Option<(PathBuf, AutonomousSourcePatchRequest)>, String> {
    if !config.source_mutation.enabled {
        return Ok(None);
    }
    let queue = config.state_dir.join("control").join("source_patch_queue");
    if !queue.exists() {
        return Ok(None);
    }
    let mut requests = fs::read_dir(&queue)
        .map_err(|error| format!("SOURCE_PATCH_QUEUE_READ:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_PATCH_QUEUE_ENTRY:{error}"))?
        .into_iter()
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file() && !kind.is_symlink())
                .unwrap_or(false)
                && entry.path().extension().and_then(OsStr::to_str) == Some("json")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    requests.sort();
    requests
        .into_iter()
        .next()
        .map(|path| read_json(&path).map(|request| (path, request)))
        .transpose()
}

fn directory_bytes(root: &Path) -> Result<u64, String> {
    if !root.exists() {
        return Ok(0);
    }
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0_u64;
    while let Some(path) = pending.pop() {
        for entry in
            fs::read_dir(&path).map_err(|error| format!("READ_DIR:{}:{error}", path.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                total = total
                    .saturating_add(entry.metadata().map_err(|error| error.to_string())?.len());
            }
        }
    }
    Ok(total)
}

fn lineage_receipt_hash(receipt: &LineageContinuationReceipt) -> Result<String, String> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    json_sha256(&unsigned)
}

fn directory_has_entries(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    fs::read_dir(path)
        .map_err(|error| format!("LINEAGE_DIRECTORY_READ:{}:{error}", path.display()))?
        .next()
        .transpose()
        .map(|entry| entry.is_some())
        .map_err(|error| format!("LINEAGE_DIRECTORY_ENTRY:{}:{error}", path.display()))
}

fn copy_lineage_store(
    predecessor_root: &Path,
    successor_root: &Path,
    relative_store: &str,
    json_only: bool,
    max_bytes: u64,
) -> Result<LineageStoreReceipt, String> {
    let source = predecessor_root.join(relative_store);
    let destination = successor_root.join(relative_store);
    if !source.exists() {
        return Ok(LineageStoreReceipt {
            relative_store: relative_store.to_string(),
            files: 0,
            bytes: 0,
            tree_sha256: sha256(&[]),
        });
    }
    let root_metadata = fs::symlink_metadata(&source)
        .map_err(|error| format!("LINEAGE_STORE_METADATA:{}:{error}", source.display()))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(format!(
            "LINEAGE_STORE_NOT_PLAIN_DIRECTORY:{}",
            source.display()
        ));
    }

    let mut pending = vec![source.clone()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("LINEAGE_STORE_READ_DIR:{}:{error}", directory.display()))?
        {
            let entry = entry.map_err(|error| format!("LINEAGE_STORE_ENTRY:{error}"))?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
                format!(
                    "LINEAGE_STORE_ENTRY_METADATA:{}:{error}",
                    entry.path().display()
                )
            })?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "LINEAGE_STORE_SYMLINK_FORBIDDEN:{}",
                    entry.path().display()
                ));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                if !json_only || entry.path().extension().and_then(OsStr::to_str) == Some("json") {
                    files.push(entry.path());
                }
            } else {
                return Err(format!(
                    "LINEAGE_STORE_SPECIAL_FILE_FORBIDDEN:{}",
                    entry.path().display()
                ));
            }
        }
    }
    files.sort();

    let mut total_bytes = 0_u64;
    let mut manifest = Vec::with_capacity(files.len());
    for source_path in files {
        let relative = source_path
            .strip_prefix(&source)
            .map_err(|error| format!("LINEAGE_STORE_RELATIVE_PATH:{error}"))?;
        let bytes = fs::read(&source_path)
            .map_err(|error| format!("LINEAGE_STORE_READ:{}:{error}", source_path.display()))?;
        total_bytes = total_bytes.saturating_add(bytes.len().min(u64::MAX as usize) as u64);
        if total_bytes > max_bytes {
            return Err(format!("LINEAGE_STORE_BOUND_REACHED:{relative_store}"));
        }
        let destination_path = destination.join(relative);
        let parent = destination_path.parent().ok_or_else(|| {
            format!(
                "LINEAGE_STORE_PARENT_MISSING:{}",
                destination_path.display()
            )
        })?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("LINEAGE_STORE_MKDIR:{}:{error}", parent.display()))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination_path)
            .map_err(|error| {
                format!(
                    "LINEAGE_STORE_CREATE:{}:{error}",
                    destination_path.display()
                )
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                format!("LINEAGE_STORE_WRITE:{}:{error}", destination_path.display())
            })?;
        let normalized = relative.to_string_lossy().replace('\\', "/");
        manifest.push(format!("{normalized}:{}:{}", bytes.len(), sha256(&bytes)));
    }
    Ok(LineageStoreReceipt {
        relative_store: relative_store.to_string(),
        files: manifest.len().min(u64::MAX as usize) as u64,
        bytes: total_bytes,
        tree_sha256: sha256(manifest.join("\n").as_bytes()),
    })
}

fn validate_lineage_successor_config(
    predecessor: &GrowthSupervisorConfig,
    successor: &GrowthSupervisorConfig,
    state: &SupervisorState,
) -> Result<(), String> {
    if predecessor.state_dir == successor.state_dir {
        return Err("LINEAGE_SUCCESSOR_STATE_DIR_NOT_NEW".to_string());
    }
    if predecessor.schema != successor.schema
        || predecessor.watched_roots != successor.watched_roots
        || predecessor.verifier_executable != successor.verifier_executable
        || predecessor.poll_interval_ms != successor.poll_interval_ms
        || predecessor.lease_stale_ms != successor.lease_stale_ms
        || predecessor.autonomous_campaigns != successor.autonomous_campaigns
        || predecessor.observation != successor.observation
        || predecessor.repository_mutation != successor.repository_mutation
    {
        return Err("LINEAGE_SUCCESSOR_POLICY_DRIFT".to_string());
    }
    let mut predecessor_source = predecessor.source_mutation.clone();
    let mut successor_source = successor.source_mutation.clone();
    predecessor_source.max_installations = 0;
    successor_source.max_installations = 0;
    if predecessor_source != successor_source
        || successor.source_mutation.max_installations
            < predecessor.source_mutation.max_installations
        || successor.source_mutation.max_installations < state.autonomous_source_patches_installed
    {
        return Err("LINEAGE_SUCCESSOR_SOURCE_POLICY_DRIFT".to_string());
    }

    let before = &predecessor.resources;
    let after = &successor.resources;
    if before.max_bytes_per_scan != after.max_bytes_per_scan
        || before.max_files_per_scan != after.max_files_per_scan
        || before.max_file_bytes != after.max_file_bytes
        || before.max_observations_per_campaign != after.max_observations_per_campaign
        || before.max_pending_observations != after.max_pending_observations
        || before.max_lessons != after.max_lessons
        || before.max_consecutive_failures != after.max_consecutive_failures
        || before.plateau_scans_before_wait != after.plateau_scans_before_wait
        || after.max_lifetime_campaigns < before.max_lifetime_campaigns
        || after.max_generations < before.max_generations
        || after.max_active_runtime_ms < before.max_active_runtime_ms
        || after.max_state_bytes < before.max_state_bytes
        || after.max_observed_bytes < before.max_observed_bytes
        || after.max_lifetime_campaigns <= state.campaigns_started
        || after.max_generations <= state.generation
        || after.max_active_runtime_ms <= state.active_runtime_ms
        || after.max_observed_bytes <= state.observed_bytes
    {
        return Err("LINEAGE_SUCCESSOR_RESOURCE_BOUND_INVALID".to_string());
    }
    let triggering_bound_expanded = match state.stop_reason.as_deref() {
        Some("MAX_LIFETIME_CAMPAIGNS_REACHED") => {
            after.max_lifetime_campaigns > before.max_lifetime_campaigns
        }
        Some("MAX_GENERATIONS_REACHED") => after.max_generations > before.max_generations,
        Some("MAX_ACTIVE_RUNTIME_REACHED") => {
            after.max_active_runtime_ms > before.max_active_runtime_ms
        }
        Some("MAX_OBSERVED_BYTES_REACHED") => after.max_observed_bytes > before.max_observed_bytes,
        Some("MAX_STATE_BYTES_REACHED") => after.max_state_bytes > before.max_state_bytes,
        _ => false,
    };
    if !triggering_bound_expanded {
        return Err("LINEAGE_PREDECESSOR_NOT_EXPANDABLE_RESOURCE_STOP".to_string());
    }
    Ok(())
}

/// Creates a fresh bounded supervisor state line from a sealed hard-stopped
/// predecessor. The operation is infrastructure-only: it preserves learned
/// state and pending typed inputs, but executes no scan, campaign, repair,
/// verifier, or difficulty-selection action.
pub fn continue_lineage(
    predecessor_config_path: &Path,
    successor_config_path: &Path,
) -> Result<LineageContinuationReceipt, String> {
    let predecessor = load_config(predecessor_config_path)?;
    let successor = load_config(successor_config_path)?;
    let frozen: GrowthSupervisorConfig =
        read_json(&predecessor.state_dir.join("config.freeze.json"))?;
    if config_hash(&frozen)? != config_hash(&predecessor)? {
        return Err("LINEAGE_PREDECESSOR_CONFIG_NOT_SEALED".to_string());
    }
    let predecessor_state = load_state(&predecessor)?;
    if predecessor_state.phase != SupervisorPhase::SafeStopped
        || predecessor_state.pending_campaign_id.is_some()
        || predecessor_state.prestart_autonomous_research_events != 0
        || predecessor_state.prestart_future_instance_exposure_events != 0
    {
        return Err("LINEAGE_PREDECESSOR_NOT_CLEANLY_SEALED".to_string());
    }
    if predecessor
        .state_dir
        .join("control")
        .join("supervisor.lease")
        .exists()
        || predecessor
            .state_dir
            .join("control")
            .join(crate::autonomous_source_mutation::SELF_UPDATE_HANDOFF_FILE)
            .exists()
        || directory_has_entries(
            &predecessor
                .state_dir
                .join("repository_install_transactions"),
        )?
    {
        return Err("LINEAGE_PREDECESSOR_HAS_ACTIVE_TRANSACTION".to_string());
    }
    validate_lineage_successor_config(&predecessor, &successor, &predecessor_state)?;
    if successor.state_dir.exists() {
        return Err("LINEAGE_SUCCESSOR_STATE_DIR_EXISTS".to_string());
    }

    let current_memory = load_memory(&predecessor, predecessor_state.generation)?;
    if json_sha256(&current_memory)? != predecessor_state.current_memory_sha256 {
        return Err("LINEAGE_CURRENT_MEMORY_HASH_MISMATCH".to_string());
    }
    let mut carried_memories = vec![current_memory];
    if let Some(expected_predecessor_hash) = &predecessor_state.predecessor_memory_sha256 {
        let generation = predecessor_state
            .generation
            .checked_sub(1)
            .ok_or_else(|| "LINEAGE_PREDECESSOR_MEMORY_GENERATION_INVALID".to_string())?;
        let memory = load_memory(&predecessor, generation)?;
        if json_sha256(&memory)? != *expected_predecessor_hash {
            return Err("LINEAGE_PREDECESSOR_MEMORY_HASH_MISMATCH".to_string());
        }
        carried_memories.push(memory);
    }
    carried_memories.sort_by_key(|memory| memory.generation);

    let parent = successor
        .state_dir
        .parent()
        .ok_or_else(|| "LINEAGE_SUCCESSOR_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("LINEAGE_SUCCESSOR_PARENT_CREATE:{error}"))?;
    let state_name = successor
        .state_dir
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "LINEAGE_SUCCESSOR_NAME_INVALID".to_string())?;
    let staging = parent.join(format!(
        ".{state_name}.lineage-{}-{}.tmp",
        std::process::id(),
        now_ms()
    ));
    fs::create_dir(&staging).map_err(|error| format!("LINEAGE_STAGING_CREATE:{error}"))?;

    let result = (|| {
        let canonical_staging = fs::canonicalize(&staging)
            .map_err(|error| format!("LINEAGE_STAGING_CANONICALIZE:{error}"))?;
        if successor.watched_roots.iter().any(|root| {
            fs::canonicalize(root)
                .map(|canonical_root| is_path_within(&canonical_staging, &canonical_root))
                .unwrap_or(true)
        }) {
            return Err("LINEAGE_SUCCESSOR_STATE_INSIDE_WATCH_ROOT_FORBIDDEN".to_string());
        }
        for directory in [
            "state",
            "journal",
            "index",
            "observations",
            "events",
            "campaigns",
            "history",
            "memory",
            "invalidated_generations",
            "control",
            "source_mutations",
            "compound_growth/queue",
            "compound_growth/receipts",
        ] {
            fs::create_dir_all(staging.join(directory))
                .map_err(|error| format!("LINEAGE_STATE_DIR_CREATE:{directory}:{error}"))?;
        }
        write_immutable_json(&staging.join("config.freeze.json"), &successor)?;
        let mut carried_memory_sha256 = Vec::new();
        for memory in &carried_memories {
            let hash = json_sha256(memory)?;
            write_immutable_json(
                &staging
                    .join("memory")
                    .join(format!("generation_{:020}.json", memory.generation)),
                memory,
            )?;
            carried_memory_sha256.push(hash);
        }

        let stores = [
            ("index", false),
            ("observations", true),
            ("events", true),
            ("source_repair_knowledge", true),
            ("improvement_operator_repository", true),
            ("compiler_diagnostic_cache", true),
            ("source_mutations", true),
            ("compound_growth", true),
            ("control/source_patch_queue", true),
        ];
        let mut carried_stores = Vec::with_capacity(stores.len());
        for (relative, json_only) in stores {
            carried_stores.push(copy_lineage_store(
                &predecessor.state_dir,
                &staging,
                relative,
                json_only,
                successor.resources.max_state_bytes,
            )?);
        }

        let mut successor_state = predecessor_state.clone();
        successor_state.sequence = 1;
        successor_state.phase = SupervisorPhase::InfraReady;
        successor_state.config_sha256 = config_hash(&successor)?;
        successor_state.pending_campaign_id = None;
        successor_state.stop_reason = None;
        successor_state.last_transition_ms = now_ms();
        // Discovery suppression is an engine-local cache, not learned
        // semantic authority. Clearing it lets a newly deployed engine inspect
        // the exact inherited source once without replaying old observations.
        successor_state.last_source_discovery_state_sha256 = None;
        let successor_state_sha256 = json_sha256(&successor_state)?;
        write_immutable_json(
            &staging
                .join("state")
                .join("state_00000000000000000001.json"),
            &successor_state,
        )?;
        write_immutable_json(
            &staging
                .join("journal")
                .join("transition_00000000000000000001.json"),
            &TransitionRecord {
                schema: SUPERVISOR_SCHEMA.to_string(),
                sequence: 1,
                from: SupervisorPhase::SafeStopped,
                to: SupervisorPhase::InfraReady,
                reason: "CONTINUED_FROM_SEALED_HARD_RESOURCE_STOP".to_string(),
                state_sha256: successor_state_sha256.clone(),
                occurred_at_ms: successor_state.last_transition_ms,
            },
        )?;

        let mut receipt = LineageContinuationReceipt {
            schema: LINEAGE_CONTINUATION_SCHEMA.to_string(),
            predecessor_config_sha256: config_hash(&predecessor)?,
            predecessor_state_sha256: json_sha256(&predecessor_state)?,
            predecessor_state_dir: predecessor.state_dir.clone(),
            predecessor_generation: predecessor_state.generation,
            predecessor_memory_sha256: predecessor_state.current_memory_sha256.clone(),
            successor_config_sha256: config_hash(&successor)?,
            successor_state_dir: successor.state_dir.clone(),
            successor_initial_state_sha256: successor_state_sha256,
            carried_memory_sha256,
            carried_stores,
            prestart_autonomous_research_events: 0,
            prestart_future_instance_exposure_events: 0,
            created_at_ms: now_ms(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = lineage_receipt_hash(&receipt)?;
        write_immutable_json(&staging.join("lineage_predecessor.json"), &receipt)?;
        if directory_bytes(&staging)? > successor.resources.max_state_bytes {
            return Err("LINEAGE_SUCCESSOR_STATE_BOUND_REACHED".to_string());
        }
        fs::rename(&staging, &successor.state_dir)
            .map_err(|error| format!("LINEAGE_SUCCESSOR_COMMIT:{error}"))?;
        Ok(receipt)
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub fn initialize(config_path: &Path) -> Result<SupervisorState, String> {
    let config = load_config(config_path)?;
    fs::create_dir_all(&config.state_dir).map_err(|error| format!("STATE_DIR_CREATE:{error}"))?;
    let canonical_state = fs::canonicalize(&config.state_dir)
        .map_err(|error| format!("STATE_DIR_CANONICALIZE:{error}"))?;
    if config.watched_roots.iter().any(|root| {
        fs::canonicalize(root)
            .map(|canonical_root| is_path_within(&canonical_state, &canonical_root))
            .unwrap_or(true)
    }) {
        return Err("STATE_DIR_INSIDE_WATCH_ROOT_FORBIDDEN".to_string());
    }
    let freeze_path = config.state_dir.join("config.freeze.json");
    if freeze_path.exists() {
        let frozen: GrowthSupervisorConfig = read_json(&freeze_path)?;
        if config_hash(&frozen)? != config_hash(&config)? {
            return Err("CONFIG_CHANGED_AFTER_FREEZE".to_string());
        }
        return load_state(&config);
    }
    for directory in [
        "state",
        "journal",
        "index",
        "observations",
        "events",
        "campaigns",
        "history",
        "memory",
        "invalidated_generations",
        "control",
        "source_mutations",
        "compound_growth/queue",
        "compound_growth/receipts",
    ] {
        fs::create_dir_all(config.state_dir.join(directory))
            .map_err(|error| format!("STATE_DIR_CREATE:{directory}:{error}"))?;
    }
    write_immutable_json(&freeze_path, &config)?;
    let memory = GrowthMemory {
        schema: SUPERVISOR_SCHEMA.to_string(),
        generation: 0,
        predecessor_sha256: None,
        lessons: Vec::new(),
        classifier: ClassifierMemory::default(),
        evaluator: EvaluatorMemory::default(),
        generative: GenerativeGrowthMemory::default(),
    };
    let memory_hash = json_sha256(&memory)?;
    write_immutable_json(&memory_path(&config, 0), &memory)?;
    let mut state = SupervisorState {
        schema: SUPERVISOR_SCHEMA.to_string(),
        sequence: 0,
        phase: SupervisorPhase::InfraReady,
        config_sha256: config_hash(&config)?,
        generation: 0,
        current_memory_sha256: memory_hash,
        predecessor_memory_sha256: None,
        campaigns_started: 0,
        campaigns_accepted: 0,
        campaigns_failed: 0,
        consecutive_failures: 0,
        plateau_scans: 0,
        active_runtime_ms: 0,
        observed_bytes: 0,
        pending_campaign_id: None,
        stop_reason: None,
        last_transition_ms: now_ms(),
        difficulty_escalation_events: 0,
        human_difficulty_level_selection_events: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
        prestart_autonomous_research_events: 0,
        prestart_future_instance_exposure_events: 0,
        last_scan_duration_ms: 0,
        last_scan_files_reused: 0,
        last_scan_files_hashed: 0,
        scan_timeout_events: 0,
        self_inspection_events: 0,
        diagnostic_experiment_events: 0,
        diagnostic_policy: DiagnosticPolicyMemory::default(),
        runtime_self_repairs_activated: 0,
        runtime_self_repair_counter_contract_revision: RUNTIME_REPAIR_COUNTER_CONTRACT_REVISION,
        legacy_unbound_runtime_self_repair_activations: 0,
        self_repair_capability_gaps: 0,
        last_internal_bottleneck: None,
        last_self_inspection_sha256: None,
        evaluator_generation: 0,
        current_evaluator_memory_sha256: json_sha256(&EvaluatorMemory::default())?,
        evaluator_challenge_cases: EvaluatorMemory::default().challenge_suite.len() as u64,
        mutual_revalidation_events: 0,
        generative_predictions: 0,
        valuable_combinations_learned: 0,
        generative_memory_reuse_events: 0,
        generative_self_application_events: 0,
        generative_exploration_events: 0,
        productive_generative_reuse_events: 0,
        generative_frontier_advance_events: 0,
        generative_frontier_capability_units: 0,
        unverified_generative_frontier_candidate_events: 0,
        legacy_unverified_generative_frontier_advance_events: 0,
        legacy_wrapper_generative_frontier_advance_events: 0,
        generative_behavioral_verification_events: 0,
        redundant_generative_selection_events: 0,
        generative_prediction_absolute_error_total: 0,
        generative_calibrated_prediction_records: 0,
        generative_legacy_uncalibrated_prediction_error_total: 0,
        autonomous_source_patch_attempts: 0,
        autonomous_source_patches_installed: 0,
        autonomous_source_patch_rollbacks: 0,
        autonomous_source_patch_validation_ms: 0,
        source_patch_recent_outcomes: Vec::new(),
        source_patch_telemetry_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
        source_patch_validation_contract_revision: SOURCE_PATCH_VALIDATION_CONTRACT_REVISION,
        source_discovery_no_candidate_streak: 0,
        last_source_discovery_reason: None,
        last_source_discovery_state_sha256: None,
        source_discovery_duplicate_states_suppressed: 0,
        source_patch_consecutive_failures: 0,
        last_source_patch_receipt_sha256: None,
        composite_capability_install_attempts: 0,
        composite_capabilities_installed: 0,
        composite_capability_install_rollbacks: 0,
        composite_capability_consecutive_failures: 0,
        last_composite_candidate_sha256: None,
        installed_composite_capability_execution_events: 0,
        installed_composite_capability_execution_failures: 0,
        last_installed_composite_execution_sha256: None,
        installed_context_bound_capabilities_validated: 0,
        last_installed_capability_inventory_sha256: None,
        installed_capability_continuation_observations: 0,
        installed_execution_counter_contract_revision:
            INSTALLED_EXECUTION_COUNTER_CONTRACT_REVISION,
        legacy_unbound_installed_composite_execution_events: 0,
        legacy_unbound_installed_composite_execution_failures: 0,
        distinct_semantic_lessons: 0,
        semantic_duplicate_lessons: 0,
        semantic_revalidation_events: 0,
        redundant_observations_consumed: 0,
        measured_performance_promotions: 0,
        classifier_outcome_bound_refinements: 0,
        classifier_unsupported_refinements_suppressed: 0,
        intrinsic_drive: IntrinsicDriveMemory::default(),
    };
    save_transition(
        &config,
        &mut state,
        SupervisorPhase::InfraReady,
        "INITIALIZED_WITH_FROZEN_CONFIG",
    )?;
    Ok(state)
}

pub fn status(config_path: &Path) -> Result<SupervisorState, String> {
    let config = load_config(config_path)?;
    let mut state = load_state(&config)?;
    let memory = load_memory(&config, state.generation)?;
    if json_sha256(&memory)? != state.current_memory_sha256 {
        return Err("CURRENT_MEMORY_HASH_MISMATCH".to_string());
    }
    // Status is read-only, but all memory-derived projections must reflect
    // the current executable-knowledge contract immediately. Otherwise a
    // stopped supervisor can display legacy metric-only lessons as promoted
    // performance knowledge until another campaign mutates state.
    restore_memory_projection(&mut state, &memory)?;
    Ok(state)
}

pub fn preview_source_repair(config_path: &Path) -> Result<serde_json::Value, String> {
    let config = load_config(config_path)?;
    let state = load_state(&config)?;
    let candidate = discover_repository_improvement(
        &config.source_mutation,
        &config.state_dir,
        state.generation,
    )?;
    let operator_memory = derive_improvement_operator_memory(&config.state_dir)?;
    let direct_source_operator_profiles = operator_memory
        .profiles
        .iter()
        .filter(|profile| {
            profile.successful_uses > 0
                && profile.operator.generator_kind
                    == ImprovementOperatorGeneratorKind::KnownStructuralRewrite
        })
        .count();
    let specialized_typed_operator_profiles = operator_memory
        .profiles
        .iter()
        .filter(|profile| {
            profile.successful_uses > 0
                && profile.operator.generator_kind
                    != ImprovementOperatorGeneratorKind::KnownStructuralRewrite
        })
        .count();
    let operator_metrics = serde_json::json!({
        "profiles": operator_memory.profiles.len(),
        "successful_uses": operator_memory.total_successful_uses,
        "direct_source_operator_profiles": direct_source_operator_profiles,
        "specialized_typed_operator_profiles": specialized_typed_operator_profiles,
        "productive_cross_family_transfers": operator_memory.productive_cross_family_transfers,
        "repository_guided_attempts": operator_memory.repository_guided_attempts,
        "repository_guided_successful_uses": operator_memory.repository_guided_successful_uses,
    });
    Ok(match candidate {
        Some(request) => serde_json::json!({
            "candidate_available": true,
            "patch_id": request.patch_id,
            "relative_path": request.relative_path,
            "transformation": request.transformation,
            "solution_strategy": request.solution_strategy,
            "opportunity_kind": request.opportunity_kind,
            "opportunity_family_id": request.opportunity_family_id,
            "source_generation": request.source_generation,
            "candidate_sha256": request.candidate_sha256,
            "improvement_operator_invocation": request.improvement_operator_invocation,
            "operator_metrics": operator_metrics,
        }),
        None => serde_json::json!({
            "candidate_available": false,
            "source_generation": state.generation,
            "operator_metrics": operator_metrics,
        }),
    })
}

fn validate_public_contract_deltas(deltas: &[PublicContractDeltaIR]) -> Result<(), String> {
    if deltas.len() > MAX_PUBLIC_CONTRACT_DELTAS_PER_EVENT {
        return Err("EVENT_PUBLIC_CONTRACT_DELTA_BOUND".to_string());
    }
    let mut delta_ids = BTreeSet::new();
    let mut goal_ids = BTreeSet::new();
    for delta in deltas {
        let observed = delta.observed_behavior.trim();
        let expected = delta.expected_behavior.trim();
        if delta.schema != PUBLIC_CONTRACT_DELTA_SCHEMA
            || delta.delta_id.is_empty()
            || delta.delta_id.len() > 80
            || !delta.delta_id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
            })
            || observed.is_empty()
            || expected.is_empty()
            || observed == expected
            || delta.observed_behavior.len() > MAX_SUMMARY_BYTES
            || delta.expected_behavior.len() > MAX_SUMMARY_BYTES
            || delta.target_symbols.len() > 32
            || delta
                .target_symbols
                .iter()
                .any(|symbol| symbol.trim().is_empty() || symbol.len() > 256)
            || delta.provenance.len() > 32
            || delta
                .provenance
                .iter()
                .any(|item| item.trim().is_empty() || item.len() > 512)
            || delta.typed_behavior_goals.is_empty()
            || delta.typed_behavior_goals.len() > MAX_TYPED_BEHAVIOR_GOALS_PER_DELTA
            || !delta_ids.insert(delta.delta_id.clone())
        {
            return Err("EVENT_PUBLIC_CONTRACT_DELTA_INVALID".to_string());
        }
        let contract_binding = format!(
            "PUBLIC_CONTRACT_DELTA_SHA256:{}",
            public_contract_delta_binding_sha256(delta)?
        );
        let delta_id_binding = format!("PUBLIC_CONTRACT_DELTA_ID:{}", delta.delta_id);
        for goal in &delta.typed_behavior_goals {
            validate_typed_mechanism_synthesis_goal(goal)
                .map_err(|error| format!("EVENT_TYPED_BEHAVIOR_GOAL_INVALID:{error}"))?;
            if !goal.provenance.contains(&contract_binding)
                || !goal.provenance.contains(&delta_id_binding)
            {
                return Err("EVENT_TYPED_BEHAVIOR_GOAL_CONTRACT_BINDING_MISSING".to_string());
            }
            if !goal_ids.insert(goal.goal_id.clone()) {
                return Err("EVENT_TYPED_BEHAVIOR_GOAL_DUPLICATE".to_string());
            }
        }
    }
    Ok(())
}

fn public_contract_delta_binding_sha256(delta: &PublicContractDeltaIR) -> Result<String, String> {
    let mut target_symbols = delta
        .target_symbols
        .iter()
        .map(|symbol| symbol.trim().to_string())
        .collect::<Vec<_>>();
    target_symbols.sort();
    target_symbols.dedup();
    json_sha256(&(
        PUBLIC_CONTRACT_DELTA_SCHEMA,
        delta.delta_id.as_str(),
        delta.observed_behavior.trim(),
        delta.expected_behavior.trim(),
        target_symbols,
    ))
}

fn validate_event(config: &GrowthSupervisorConfig, event: &mut WorkEvent) -> Result<(), String> {
    if event.event_id.is_empty() {
        event.event_id = json_sha256(event)?[..32].to_string();
    }
    if event.event_id.len() > 80
        || !event
            .event_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("EVENT_ID_INVALID".to_string());
    }
    if event.paths.is_empty() || event.paths.len() > config.resources.max_observations_per_campaign
    {
        return Err("EVENT_PATH_COUNT_INVALID".to_string());
    }
    if event.summary.len() > MAX_SUMMARY_BYTES {
        return Err("EVENT_SUMMARY_TOO_LARGE".to_string());
    }
    if event.evidence_sha256.len() > 32
        || event.evidence_artifacts.len() > 32
        || event.performance_metrics.len() > 16
        || event
            .evidence_sha256
            .iter()
            .any(|hash| hash.len() != 64 || !hash.chars().all(|value| value.is_ascii_hexdigit()))
    {
        return Err("EVENT_EVIDENCE_HASH_INVALID".to_string());
    }
    validate_public_contract_deltas(&event.public_contract_deltas)?;
    let mut bound_evidence = Vec::with_capacity(event.evidence_artifacts.len());
    for path in &mut event.evidence_artifacts {
        let canonical = fs::canonicalize(&*path)
            .map_err(|error| format!("EVIDENCE_PATH_CANONICALIZE:{}:{error}", path.display()))?;
        if !canonical.is_file() || path_is_secret(&canonical, &config.observation) {
            return Err(format!("EVIDENCE_PATH_FORBIDDEN:{}", canonical.display()));
        }
        bound_evidence.push(file_sha256(&canonical, 16 * 1024 * 1024)?);
        *path = canonical;
    }
    if !event.evidence_sha256.is_empty() && event.evidence_sha256 != bound_evidence {
        return Err("EVENT_EVIDENCE_ARTIFACT_HASH_MISMATCH".to_string());
    }
    event.evidence_sha256 = bound_evidence;
    if event.performance_metrics.iter().any(|metric| {
        metric.metric.is_empty()
            || metric.metric.len() > 64
            || !metric
                .metric
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | '.'))
            || metric.evidence_sha256.len() != 64
            || !event.evidence_sha256.contains(&metric.evidence_sha256)
    }) {
        return Err("PERFORMANCE_METRIC_EVIDENCE_INVALID_OR_UNBOUND".to_string());
    }
    for metric in &event.performance_metrics {
        let Some(knowledge) = &metric.executable_knowledge else {
            continue;
        };
        if !metric.improved()
            || knowledge.schema != EXECUTABLE_PERFORMANCE_KNOWLEDGE_SCHEMA
            || knowledge.predecessor_content_sha256.len() != 64
            || knowledge.candidate_content_sha256.len() != 64
            || !knowledge
                .predecessor_content_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !knowledge
                .candidate_content_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("EXECUTABLE_PERFORMANCE_KNOWLEDGE_BINDING_INVALID".to_string());
        }
        validate_improvement_operator(&knowledge.improvement_operator)?;
    }
    if event.outcome == WorkOutcome::Pass && event.evidence_sha256.is_empty() {
        return Err("PASS_EVENT_REQUIRES_BOUND_EVIDENCE_ARTIFACT".to_string());
    }
    for path in &mut event.paths {
        let canonical = fs::canonicalize(&*path)
            .map_err(|error| format!("EVENT_PATH_CANONICALIZE:{}:{error}", path.display()))?;
        if !config.watched_roots.iter().any(|root| {
            fs::canonicalize(root)
                .map(|canonical_root| is_path_within(&canonical, &canonical_root))
                .unwrap_or(false)
        }) || path_is_secret(&canonical, &config.observation)
        {
            return Err(format!("EVENT_PATH_FORBIDDEN:{}", canonical.display()));
        }
        *path = canonical;
    }
    if event.occurred_at_ms == 0 {
        event.occurred_at_ms = now_ms();
    }
    Ok(())
}

pub fn record_work_event(config_path: &Path, mut event: WorkEvent) -> Result<WorkEvent, String> {
    let config = load_config(config_path)?;
    let _ = load_state(&config)?;
    validate_event(&config, &mut event)?;
    let path = config
        .state_dir
        .join("events")
        .join(format!("{}.json", event.event_id));
    write_immutable_json(&path, &event)?;
    Ok(event)
}

pub fn request_stop(config_path: &Path) -> Result<serde_json::Value, String> {
    let config = load_config(config_path)?;
    let _ = load_state(&config)?;
    let path = config.state_dir.join("control").join("STOP");
    if !path.exists() {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("STOP_CREATE:{error}"))?;
        writeln!(file, "requested_at_ms={}", now_ms())
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("STOP_WRITE:{error}"))?;
    }
    Ok(serde_json::json!({"stop_requested": true, "path": path}))
}

fn latest_failed_campaign_freeze(
    config: &GrowthSupervisorConfig,
) -> Result<Option<CampaignFreeze>, String> {
    let mut latest: Option<(u64, String)> = None;
    for entry in fs::read_dir(config.state_dir.join("history"))
        .map_err(|error| format!("FAILURE_HISTORY_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("FAILURE_HISTORY_ENTRY:{error}"))?;
        if !entry
            .file_name()
            .to_string_lossy()
            .ends_with(".failure.json")
        {
            continue;
        }
        let failure: CampaignFailure = read_json(&entry.path())?;
        let candidate = (failure.occurred_at_ms, failure.campaign_id);
        if latest.as_ref().is_none_or(|current| candidate > *current) {
            latest = Some(candidate);
        }
    }
    latest
        .map(|(_, campaign_id)| read_json(&campaign_dir(config, &campaign_id).join("freeze.json")))
        .transpose()
}

fn failed_campaign_engine_changed(
    config: &GrowthSupervisorConfig,
    current_proposer_sha256: &str,
) -> Result<bool, String> {
    Ok(latest_failed_campaign_freeze(config)?
        .is_some_and(|freeze| freeze.proposer_executable_sha256 != current_proposer_sha256))
}

pub fn request_resume(config_path: &Path) -> Result<serde_json::Value, String> {
    let config = load_config(config_path)?;
    let mut state = load_state(&config)?;
    let invalid_tip_recovered = recover_verification_only_generation_tip(&config, &mut state)?;
    let pending_self_update = config
        .state_dir
        .join("control")
        .join(crate::autonomous_source_mutation::SELF_UPDATE_HANDOFF_FILE)
        .is_file();
    let staged_stop = matches!(
        state.stop_reason.as_deref(),
        Some("AUTONOMOUS_SOURCE_UPDATE_STAGED" | "AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED")
    );
    if state.phase == SupervisorPhase::SafeStopped && staged_stop && pending_self_update {
        return Ok(serde_json::json!({
            "resume_requested": false,
            "phase": state.phase,
            "invalid_verification_only_tip_recovered": invalid_tip_recovered,
            "hard_resource_stop_preserved": false,
            "pending_self_update": true
        }));
    }
    let path = config.state_dir.join("control").join("STOP");
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("STOP_REMOVE:{error}"))?;
    }
    let proposer_executable_sha256 = std::env::current_exe()
        .map_err(|error| format!("CURRENT_EXE:{error}"))
        .and_then(|path| file_sha256(&path, 512 * 1024 * 1024))?;
    let repaired_failure_stop = state.stop_reason.as_deref()
        == Some("MAX_CONSECUTIVE_FAILURES_REACHED")
        && failed_campaign_engine_changed(&config, &proposer_executable_sha256)?;
    let resumable_reason = matches!(
        state.stop_reason.as_deref(),
        Some(
            "OPERATOR_STOP_REQUESTED"
                | "SCAN_RUNTIME_BOUND_REACHED"
                | "AUTONOMOUS_SOURCE_UPDATE_STAGED"
                | "AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED"
        )
    ) || repaired_failure_stop;
    if state.phase == SupervisorPhase::SafeStopped && resumable_reason {
        let previous_reason = state.stop_reason.clone();
        if repaired_failure_stop {
            state.consecutive_failures = 0;
        }
        state.stop_reason = None;
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::InfraReady,
            match previous_reason.as_deref() {
                Some("SCAN_RUNTIME_BOUND_REACHED") => {
                    "OPERATOR_RESUME_AFTER_TRANSIENT_SCAN_TIMEOUT"
                }
                Some("AUTONOMOUS_SOURCE_UPDATE_STAGED") => {
                    "AUTONOMOUS_SOURCE_UPDATE_APPLIED_AND_RESUMED"
                }
                Some("AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED") => {
                    "AUTONOMOUS_COMPOSITE_CAPABILITY_APPLIED_AND_RESUMED"
                }
                Some("MAX_CONSECUTIVE_FAILURES_REACHED") => {
                    "OPERATOR_RESUME_AFTER_CAMPAIGN_ENGINE_REPAIR"
                }
                _ => "OPERATOR_RESUME_REQUESTED",
            },
        )?;
    }
    Ok(serde_json::json!({
        "resume_requested": true,
        "phase": state.phase,
        "invalid_verification_only_tip_recovered": invalid_tip_recovered,
        "hard_resource_stop_preserved": state.stop_reason.is_some(),
        "campaign_engine_repair_detected": repaired_failure_stop,
        "pending_self_update": false
    }))
}

fn load_pending_events(
    config: &GrowthSupervisorConfig,
    index: &FileIndex,
) -> Result<Vec<WorkEvent>, String> {
    let mut events = Vec::new();
    for entry in fs::read_dir(config.state_dir.join("events"))
        .map_err(|error| format!("EVENT_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }
        let event: WorkEvent = read_json(&entry.path())?;
        if !index.consumed_work_event_ids.contains(&event.event_id) {
            events.push(event);
        }
    }
    events.sort_by_key(|event| (event.occurred_at_ms, event.event_id.clone()));
    Ok(events)
}

struct ScanFingerprintTask {
    ordinal: usize,
    path: PathBuf,
    metadata: fs::Metadata,
    logical_path: String,
    previous: Option<FileFingerprint>,
    work_event_index: Option<usize>,
}

fn fingerprint_scan_batch(
    tasks: &[ScanFingerprintTask],
    max_file_bytes: u64,
) -> Result<Vec<Option<FileFingerprint>>, String> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(tasks.len())
        .max(1);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::with_capacity(tasks.len()));
    thread::scope(|scope| -> Result<(), String> {
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            handles.push(scope.spawn(|| -> Result<(), String> {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };
                    let result =
                        fingerprint_file_with_metadata(&task.path, &task.metadata, max_file_bytes);
                    results
                        .lock()
                        .map_err(|_| "SCAN_FINGERPRINT_RESULT_LOCK_POISONED".to_string())?
                        .push((index, result));
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| "SCAN_FINGERPRINT_WORKER_PANICKED".to_string())??;
        }
        Ok(())
    })?;
    let mut ordered = results
        .into_inner()
        .map_err(|_| "SCAN_FINGERPRINT_RESULT_LOCK_POISONED".to_string())?;
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, result)| result).collect()
}

fn scan_watched_roots(
    config: &GrowthSupervisorConfig,
    memory: &GrowthMemory,
) -> Result<ScanResult, String> {
    let old_index = load_index(config)?;
    let baseline_created = !old_index.baseline_complete;
    let events = load_pending_events(config, &old_index)?;
    let event_paths = work_event_path_index(&events);
    let paths = collect_files(
        &config.watched_roots,
        &config.observation,
        config.resources.max_files_per_scan,
    )?;
    let mut new_index = FileIndex {
        schema: SUPERVISOR_SCHEMA.to_string(),
        sequence: old_index.sequence,
        baseline_complete: old_index.baseline_complete,
        files: old_index.files.clone(),
        consumed_observation_ids: old_index.consumed_observation_ids.clone(),
        consumed_work_event_ids: old_index.consumed_work_event_ids.clone(),
    };
    let current_logical_paths = paths
        .iter()
        .map(|path| normalized_logical_path(path, &config.watched_roots))
        .collect::<Result<BTreeSet<_>, _>>()?;
    new_index
        .files
        .retain(|logical_path, _| current_logical_paths.contains(logical_path));
    let mut ordered_observations = Vec::new();
    let mut bytes_observed = 0_u64;
    let mut files_reused = 0_usize;
    let mut files_hashed = 0_usize;
    let mut replayed_unchanged_work_events = 0_usize;
    let mut baseline_pending_files = false;
    let mut eligible_logical_paths = BTreeSet::new();
    let canary_bucket = old_index.sequence % FULL_HASH_CANARY_INTERVAL;
    let mut fingerprint_tasks = Vec::new();
    for (ordinal, path) in paths.iter().enumerate() {
        let metadata =
            fs::metadata(path).map_err(|error| format!("METADATA:{}:{error}", path.display()))?;
        let logical = normalized_logical_path(path, &config.watched_roots)?;
        if metadata.len() > config.resources.max_file_bytes {
            new_index.files.remove(&logical);
            continue;
        }
        eligible_logical_paths.insert(logical.clone());
        let previous = old_index.files.get(&logical);
        let matching_event_index = event_paths.get(&event_path_key(path)).copied();
        let matching_event = matching_event_index.and_then(|index| events.get(index));
        let canary_rehash = !baseline_created
            && previous.is_some()
            && logical_path_canary_bucket(&logical) == canary_bucket;
        if (baseline_created || !canary_rehash)
            && previous.is_some_and(|value| {
                value.bytes == metadata.len() && value.modified_ms == modified_ms(&metadata)
            })
        {
            let indexed = previous.expect("checked above");
            if !baseline_created {
                if let Some(event) = matching_event {
                    ordered_observations.push((
                        ordinal,
                        classify_observation(
                            logical.clone(),
                            indexed,
                            Some(indexed),
                            Some(event),
                            &memory.classifier,
                            config.observation.minimum_learning_score,
                        ),
                    ));
                    replayed_unchanged_work_events =
                        replayed_unchanged_work_events.saturating_add(1);
                }
            }
            new_index.files.insert(logical, indexed.clone());
            files_reused = files_reused.saturating_add(1);
            continue;
        }
        fingerprint_tasks.push(ScanFingerprintTask {
            ordinal,
            path: path.clone(),
            metadata,
            logical_path: logical,
            previous: previous.cloned(),
            work_event_index: matching_event_index,
        });
    }

    let scan_byte_limit = if baseline_created {
        config
            .resources
            .max_bytes_per_scan
            .min(BASELINE_MAX_BYTES_PER_SCAN)
    } else {
        config.resources.max_bytes_per_scan
    };
    let parallel_width = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .saturating_mul(2)
        .max(1);
    let mut cursor = 0_usize;
    'scan_batches: while cursor < fingerprint_tasks.len() {
        if baseline_created && files_hashed >= BASELINE_MAX_HASHED_FILES_PER_SCAN {
            baseline_pending_files = true;
            break;
        }
        let remaining_file_budget = if baseline_created {
            BASELINE_MAX_HASHED_FILES_PER_SCAN.saturating_sub(files_hashed)
        } else {
            usize::MAX
        };
        let batch_limit = parallel_width.min(remaining_file_budget).max(1);
        let batch_start = cursor;
        let mut reserved_bytes = 0_u64;
        while cursor < fingerprint_tasks.len() && cursor - batch_start < batch_limit {
            let task = &fingerprint_tasks[cursor];
            if bytes_observed
                .saturating_add(reserved_bytes)
                .saturating_add(task.metadata.len())
                > scan_byte_limit
            {
                if baseline_created {
                    baseline_pending_files = true;
                    if cursor == batch_start {
                        cursor = cursor.saturating_add(1);
                        continue 'scan_batches;
                    }
                    break;
                }
                break 'scan_batches;
            }
            reserved_bytes = reserved_bytes.saturating_add(task.metadata.len());
            cursor = cursor.saturating_add(1);
        }
        if batch_start == cursor {
            continue;
        }
        let batch = &fingerprint_tasks[batch_start..cursor];
        let fingerprints = fingerprint_scan_batch(batch, config.resources.max_file_bytes)?;
        for (task, fingerprint) in batch.iter().zip(fingerprints) {
            let Some(fingerprint) = fingerprint else {
                eligible_logical_paths.remove(&task.logical_path);
                new_index.files.remove(&task.logical_path);
                continue;
            };
            bytes_observed = bytes_observed.saturating_add(fingerprint.bytes);
            files_hashed = files_hashed.saturating_add(1);
            let matching_event = task.work_event_index.and_then(|index| events.get(index));
            let content_changed = task
                .previous
                .as_ref()
                .map(|value| value.content_sha256.as_str())
                != Some(fingerprint.content_sha256.as_str());
            if !baseline_created && (content_changed || matching_event.is_some()) {
                ordered_observations.push((
                    task.ordinal,
                    classify_observation(
                        task.logical_path.clone(),
                        &fingerprint,
                        task.previous.as_ref(),
                        matching_event,
                        &memory.classifier,
                        config.observation.minimum_learning_score,
                    ),
                ));
                if !content_changed && matching_event.is_some() {
                    replayed_unchanged_work_events =
                        replayed_unchanged_work_events.saturating_add(1);
                }
            }
            new_index
                .files
                .insert(task.logical_path.clone(), fingerprint);
        }
    }
    ordered_observations.sort_by_key(|(ordinal, _)| *ordinal);
    let observations = ordered_observations
        .into_iter()
        .map(|(_, observation)| observation)
        .collect::<Vec<_>>();
    if baseline_created {
        new_index.baseline_complete = !baseline_pending_files
            && eligible_logical_paths
                .iter()
                .all(|logical_path| new_index.files.contains_key(logical_path));
    }
    Ok(ScanResult {
        index: new_index,
        baseline_created,
        files_scanned: paths.len(),
        bytes_observed,
        observations,
        files_reused,
        files_hashed,
        pending_work_events: events.len(),
        replayed_unchanged_work_events,
    })
}

fn persist_scan_observations(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Result<(), String> {
    for observation in observations {
        let path = config
            .state_dir
            .join("observations")
            .join(format!("{}.json", observation.observation_id));
        if !path.exists() {
            write_immutable_json(&path, observation)?;
        } else {
            let existing: LearningObservation = read_json(&path)?;
            if existing != *observation {
                return Err(format!(
                    "OBSERVATION_ID_COLLISION:{}:{}:{}",
                    observation.observation_id,
                    json_sha256(&existing)?,
                    json_sha256(observation)?
                ));
            }
        }
    }
    Ok(())
}

fn scan_watched_roots_bounded(
    config: &GrowthSupervisorConfig,
    memory: &GrowthMemory,
    lease: &SupervisorLease,
) -> Result<ScanResult, String> {
    if config.state_dir.join("control").join("STOP").exists() {
        return Err("OPERATOR_STOP_REQUESTED_DURING_SCAN".to_string());
    }
    let worker_config = config.clone();
    let worker_memory = memory.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("b-core-bounded-scanner".to_string())
        .spawn(move || {
            let _ = sender.send(scan_watched_roots(&worker_config, &worker_memory));
        })
        .map_err(|error| format!("SCAN_WORKER_SPAWN:{error}"))?;
    let started = Instant::now();
    loop {
        match receiver.recv_timeout(Duration::from_millis(SCAN_WATCHDOG_TICK_MS)) {
            Ok(result) => return result,
            Err(RecvTimeoutError::Disconnected) => {
                return Err("SCAN_WORKER_DISCONNECTED".to_string())
            }
            Err(RecvTimeoutError::Timeout) => {
                lease.heartbeat()?;
                if config.state_dir.join("control").join("STOP").exists() {
                    return Err("OPERATOR_STOP_REQUESTED_DURING_SCAN".to_string());
                }
                if started.elapsed() >= Duration::from_millis(MAX_SCAN_RUNTIME_MS) {
                    return Err("SCAN_RUNTIME_BOUND_REACHED".to_string());
                }
            }
        }
    }
}

fn load_unconsumed_high_observations(
    config: &GrowthSupervisorConfig,
    index: &FileIndex,
) -> Result<Vec<LearningObservation>, String> {
    let mut observations = Vec::new();
    for entry in fs::read_dir(config.state_dir.join("observations"))
        .map_err(|error| format!("OBSERVATION_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }
        let observation: LearningObservation = read_json(&entry.path())?;
        if observation.learning_value == LearningValue::High
            && !index
                .consumed_observation_ids
                .contains(&observation.observation_id)
            && !observation
                .work_event_id
                .as_ref()
                .is_some_and(|event_id| index.consumed_work_event_ids.contains(event_id))
        {
            observations.push(observation);
        }
    }
    observations.sort_by_key(|observation| {
        (
            std::cmp::Reverse(observation.learning_score),
            observation.observation_id.clone(),
        )
    });
    if observations.len() > config.resources.max_pending_observations {
        observations.truncate(config.resources.max_pending_observations);
    }
    Ok(observations)
}

fn consume_superseded_high_observations(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
    observations: &mut Vec<LearningObservation>,
) -> Result<usize, String> {
    let mut retained = Vec::with_capacity(observations.len());
    let mut consumed = 0usize;
    for observation in observations.drain(..) {
        let superseded = index
            .files
            .get(&observation.logical_path)
            .is_some_and(|current| current.content_sha256 != observation.content_sha256);
        if superseded {
            index
                .consumed_observation_ids
                .insert(observation.observation_id.clone());
            if let Some(event_id) = observation.work_event_id {
                index.consumed_work_event_ids.insert(event_id);
            }
            consumed = consumed.saturating_add(1);
        } else {
            retained.push(observation);
        }
    }
    *observations = retained;
    if consumed > 0 {
        save_index(config, index)?;
        state.redundant_observations_consumed = state
            .redundant_observations_consumed
            .saturating_add(consumed.min(u64::MAX as usize) as u64);
    }
    Ok(consumed)
}

fn derive_composition_recipe(observations: &[LearningObservation]) -> Vec<String> {
    let mut roles = observations
        .iter()
        .flat_map(|observation| observation.composition_roles.iter().cloned())
        .collect::<BTreeSet<_>>();
    // A production source file can contain both implementation and its local
    // tests.  Treating the entire file as test-only starves the campaign of an
    // implementation role even when capability, validation, or error-handling
    // structure changed in the same observation.
    if observations.iter().any(|observation| {
        !path_is_dedicated_test(Path::new(&observation.logical_path))
            && observation.signals.iter().any(|signal| {
                matches!(
                    signal.as_str(),
                    "CAPABILITY_SURFACE_ADDED"
                        | "VALIDATION_ADDED"
                        | "ERROR_HANDLING_ADDED"
                        | "DEFECT_REPAIR"
                        | "REFACTOR"
                        | "CODE_CHANGE"
                        | "PERFORMANCE_OPTIMIZATION"
                        | "EFFICIENCY_MECHANISM"
                )
            })
    }) {
        roles.insert("IMPLEMENTATION".to_string());
    }
    let mut recipe = Vec::new();
    for role in [
        "BACKEND_PROVIDER",
        "INPUT_VALIDATION",
        "IMPLEMENTATION",
        "IMPLEMENTATION_REPAIR",
        "ERROR_PROPAGATION",
        "FRONTEND_CONSUMER",
        "OPERATIONS_GUARD",
        "PERFORMANCE_IMPLEMENTATION",
        "PERFORMANCE_BENCHMARK",
        "PROGRAM_COMPOSITION",
        "INVARIANT_CHECK",
        "REGRESSION_TEST",
    ] {
        if roles.contains(role) {
            recipe.push(role.to_string());
        }
    }
    if recipe.len() < 2 {
        recipe.push("INDEPENDENT_VERIFICATION".to_string());
    }
    recipe.dedup();
    recipe
}

fn structural_delta_bucket(magnitude: u32) -> &'static str {
    match magnitude {
        0 => "NONE",
        1 => "ONE",
        2..=4 => "SMALL",
        5..=16 => "MEDIUM",
        _ => "LARGE",
    }
}

fn append_structural_delta_signals(
    signals: &mut BTreeSet<String>,
    observation: &LearningObservation,
) {
    let (Some(before), after) = (
        observation.features_before.as_ref(),
        &observation.features_after,
    ) else {
        return;
    };
    for (field, prior, current) in [
        ("PUBLIC_SYMBOL", before.public_symbols, after.public_symbols),
        ("BRANCH", before.branch_tokens, after.branch_tokens),
        ("ASSERTION", before.assertion_tokens, after.assertion_tokens),
        ("TEST", before.test_tokens, after.test_tokens),
        (
            "VALIDATION",
            before.validation_tokens,
            after.validation_tokens,
        ),
        (
            "ERROR_HANDLING",
            before.error_handling_tokens,
            after.error_handling_tokens,
        ),
        ("TODO", before.todo_tokens, after.todo_tokens),
        ("BENCHMARK", before.benchmark_tokens, after.benchmark_tokens),
        (
            "PERFORMANCE",
            before.performance_tokens,
            after.performance_tokens,
        ),
        (
            "ALGEBRAIC_CONSTRUCTOR",
            before.algebraic_constructor_tokens,
            after.algebraic_constructor_tokens,
        ),
        (
            "DATA_COMPOSITION",
            before.data_composition_tokens,
            after.data_composition_tokens,
        ),
    ] {
        if prior == current {
            continue;
        }
        signals.insert(format!(
            "STRUCTURAL_DELTA:{field}:{}:{}",
            if current > prior {
                "INCREASE"
            } else {
                "DECREASE"
            },
            structural_delta_bucket(current.abs_diff(prior))
        ));
    }
}

fn build_lesson(observations: &[LearningObservation]) -> Result<LearnedCompositionLesson, String> {
    let mut signals = observations
        .iter()
        .flat_map(|observation| observation.signals.iter().cloned())
        .collect::<BTreeSet<_>>();
    for observation in observations {
        append_structural_delta_signals(&mut signals, observation);
        if !observation.public_contract_deltas.is_empty() {
            signals.insert("PUBLIC_CONTRACT_DELTA_BOUND".to_string());
            signals.insert("TYPED_BEHAVIOR_GOAL_AVAILABLE".to_string());
        }
    }
    let kinds = observations
        .iter()
        .map(|observation| observation.work_kind)
        .collect::<BTreeSet<_>>();
    let evidence = observations
        .iter()
        .map(json_sha256)
        .collect::<Result<Vec<_>, _>>()?;
    let mut performance_metric_index = BTreeMap::new();
    for metric in observations
        .iter()
        .flat_map(|observation| &observation.performance_metrics)
    {
        performance_metric_index
            .entry(json_sha256(metric)?)
            .or_insert_with(|| metric.clone());
    }
    let performance_metrics = performance_metric_index.into_values().collect::<Vec<_>>();
    let mut public_contract_delta_index = BTreeMap::new();
    for delta in observations
        .iter()
        .flat_map(|observation| &observation.public_contract_deltas)
    {
        public_contract_delta_index
            .entry(json_sha256(delta)?)
            .or_insert_with(|| delta.clone());
    }
    let public_contract_deltas = public_contract_delta_index
        .into_values()
        .collect::<Vec<_>>();
    let learning_score = observations
        .iter()
        .map(|observation| u32::from(observation.learning_score))
        .sum::<u32>()
        .checked_div(observations.len().max(1) as u32)
        .unwrap_or(0)
        .min(100) as u16;
    let recipe = derive_composition_recipe(observations);
    let lesson_id = sha256(
        format!(
            "{}:{}:{}:{}",
            evidence.join(":"),
            signals.iter().cloned().collect::<Vec<_>>().join(":"),
            recipe.join(":"),
            public_contract_deltas
                .iter()
                .map(json_sha256)
                .collect::<Result<Vec<_>, _>>()?
                .join(":")
        )
        .as_bytes(),
    );
    Ok(LearnedCompositionLesson {
        lesson_id,
        evidence_observation_sha256: evidence,
        work_kinds: kinds.into_iter().collect(),
        diagnostic_signals: signals.into_iter().collect(),
        composition_recipe: recipe,
        applicability: vec![
            "apply only when the same structural signals recur".to_string(),
            "do not route by repository, task, or exact patch identity".to_string(),
        ],
        verification_obligations: vec![
            "local deterministic verifier must accept the frozen candidate".to_string(),
            "regression evidence must accompany implementation learning".to_string(),
            "raw source and secret material must remain absent".to_string(),
        ],
        performance_metrics,
        public_contract_deltas,
        learning_score,
        exact_patch_data_present: false,
        exact_source_fragment_present: false,
        raw_source_bytes_present: false,
    })
}

fn lesson_semantic_sha256(lesson: &LearnedCompositionLesson) -> Result<String, String> {
    let mut executable_goal_hashes = lesson
        .public_contract_deltas
        .iter()
        .flat_map(|delta| &delta.typed_behavior_goals)
        .map(|goal| {
            let mut executable_identity = goal.clone();
            // These fields are useful explanation/provenance, but neither is
            // consumed by the typed grammar evaluator. Changing prose or an
            // identifier cannot create another learned capability.
            executable_identity.goal_id.clear();
            executable_identity.preconditions.clear();
            executable_identity.postconditions.clear();
            executable_identity.invariants.clear();
            executable_identity.provenance.clear();
            json_sha256(&executable_identity)
        })
        .collect::<Result<Vec<_>, _>>()?;
    executable_goal_hashes.sort();
    executable_goal_hashes.dedup();
    let mut executable_performance_operator_ids = lesson
        .performance_metrics
        .iter()
        .filter(|metric| metric.has_executable_knowledge())
        .filter_map(|metric| metric.executable_knowledge.as_ref())
        .map(|knowledge| knowledge.improvement_operator.operator_id.clone())
        .collect::<Vec<_>>();
    executable_performance_operator_ids.sort();
    executable_performance_operator_ids.dedup();
    json_sha256(&(executable_goal_hashes, executable_performance_operator_ids))
}

fn memory_contains_semantic_lesson(
    memory: &GrowthMemory,
    candidate: &LearnedCompositionLesson,
) -> Result<bool, String> {
    let candidate_sha256 = lesson_semantic_sha256(candidate)?;
    for lesson in &memory.lessons {
        if lesson_semantic_sha256(lesson)? == candidate_sha256 {
            return Ok(true);
        }
    }
    Ok(false)
}

fn semantic_lesson_counts(memory: &GrowthMemory) -> Result<(u64, u64), String> {
    let identities = memory
        .lessons
        .iter()
        .filter(|lesson| lesson_has_executable_knowledge(lesson))
        .map(lesson_semantic_sha256)
        .collect::<Result<BTreeSet<_>, _>>()?;
    let distinct = identities.len().min(u64::MAX as usize) as u64;
    let total = memory
        .lessons
        .iter()
        .filter(|lesson| lesson_has_executable_knowledge(lesson))
        .count()
        .min(u64::MAX as usize) as u64;
    Ok((distinct, total.saturating_sub(distinct)))
}

fn derive_next_evaluator_memory(
    current: &EvaluatorMemory,
    prior_lessons: &[LearnedCompositionLesson],
    lesson: &LearnedCompositionLesson,
) -> Result<EvaluatorMemory, String> {
    if current.schema != "B_CORE_GROWTH_EVALUATOR_MEMORY_1" {
        return Err("EVALUATOR_MEMORY_SCHEMA_INVALID".to_string());
    }
    let mut challenge_suite = current
        .challenge_suite
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let predecessor_challenge_count = challenge_suite.len();
    if !lesson.applicability.is_empty() {
        challenge_suite.insert(EvaluatorMutationKind::ApplicabilityMutation);
    }
    if !lesson.verification_obligations.is_empty() {
        challenge_suite.insert(EvaluatorMutationKind::VerificationObligationMutation);
    }
    if !lesson.evidence_observation_sha256.is_empty() {
        challenge_suite.insert(EvaluatorMutationKind::EvidenceRemoval);
    }
    challenge_suite.insert(EvaluatorMutationKind::RawSourceFlagInjection);

    let mut source_lesson_ids = current.source_lesson_ids.clone();
    for prior_lesson in prior_lessons {
        if !source_lesson_ids.contains(&prior_lesson.lesson_id) {
            source_lesson_ids.push(prior_lesson.lesson_id.clone());
        }
    }
    if !source_lesson_ids.contains(&lesson.lesson_id) {
        source_lesson_ids.push(lesson.lesson_id.clone());
    }
    let challenge_suite_expanded = challenge_suite.len() > predecessor_challenge_count;
    let (accepted_expansions, legacy_unbound_accepted_expansions) =
        if current.capability_expansion_contract_revision < 2 {
            let evidence_bound_historical_expansions = u64::from(
                current.challenge_suite.len() > EvaluatorMemory::default().challenge_suite.len(),
            );
            (
                evidence_bound_historical_expansions,
                current.legacy_unbound_accepted_expansions.saturating_add(
                    current
                        .accepted_expansions
                        .saturating_sub(evidence_bound_historical_expansions),
                ),
            )
        } else {
            (
                current.accepted_expansions,
                current.legacy_unbound_accepted_expansions,
            )
        };
    Ok(EvaluatorMemory {
        schema: current.schema.clone(),
        generation: current.generation.saturating_add(1),
        predecessor_sha256: Some(json_sha256(current)?),
        challenge_suite: challenge_suite.into_iter().collect(),
        source_lesson_ids,
        accepted_expansions: accepted_expansions
            .saturating_add(u64::from(challenge_suite_expanded)),
        capability_expansion_contract_revision: 2,
        legacy_unbound_accepted_expansions,
    })
}

fn lesson_has_verification_evidence(lesson: &LearnedCompositionLesson) -> bool {
    let signals = lesson
        .diagnostic_signals
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    signals.contains("VERIFIED_PASS")
}

fn lesson_has_growth_subject(lesson: &LearnedCompositionLesson) -> bool {
    lesson
        .work_kinds
        .iter()
        .any(|kind| *kind != WorkKind::Verification)
        || lesson
            .diagnostic_signals
            .iter()
            .any(|signal| signal == "MUTUAL_REVALIDATION_GAP")
}

fn executable_performance_operators(
    lesson: &LearnedCompositionLesson,
) -> Vec<ImprovementOperatorIR> {
    let mut operators = BTreeMap::new();
    for metric in &lesson.performance_metrics {
        if !metric.has_executable_knowledge() {
            continue;
        }
        let Some(knowledge) = &metric.executable_knowledge else {
            continue;
        };
        operators
            .entry(knowledge.improvement_operator.operator_id.clone())
            .or_insert_with(|| knowledge.improvement_operator.clone());
    }
    operators.into_values().collect()
}

fn executable_performance_promotion_count(memory: &GrowthMemory) -> u64 {
    memory
        .lessons
        .iter()
        .filter(|lesson| !executable_performance_operators(lesson).is_empty())
        .count()
        .min(u64::MAX as usize) as u64
}

/// Returns true only when a lesson contains a machine-consumable payload.
/// Structural labels, natural-language summaries, applicability prose and a
/// PASS token remain useful forensic evidence, but cannot synthesize a patch
/// and therefore cannot authorize a learned generation.
fn lesson_has_executable_knowledge(lesson: &LearnedCompositionLesson) -> bool {
    let typed_goal_bound = !lesson.public_contract_deltas.is_empty()
        && validate_public_contract_deltas(&lesson.public_contract_deltas).is_ok()
        && lesson
            .public_contract_deltas
            .iter()
            .flat_map(|delta| &delta.typed_behavior_goals)
            .next()
            .is_some();
    typed_goal_bound || !executable_performance_operators(lesson).is_empty()
}

fn cohort_has_promotable_growth_subject(
    observations: &[LearningObservation],
    lesson: &LearnedCompositionLesson,
) -> bool {
    lesson_has_verification_evidence(lesson)
        && lesson_has_growth_subject(lesson)
        && lesson_has_executable_knowledge(lesson)
        && !observations.is_empty()
}

fn selected_campaign_observations(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Vec<LearningObservation> {
    const COHERENT_COHORT_WINDOW_MS: u64 = 30 * 60 * 1_000;
    for anchor in observations {
        let anchor_root = anchor.logical_path.split('/').next().unwrap_or("");
        let mut seen_paths = BTreeSet::new();
        let coherent = observations
            .iter()
            .filter(|observation| {
                observation.logical_path.split('/').next().unwrap_or("") == anchor_root
                    && observation.observed_at_ms.abs_diff(anchor.observed_at_ms)
                        <= COHERENT_COHORT_WINDOW_MS
                    && seen_paths.insert(observation.logical_path.clone())
            })
            .take(config.resources.max_observations_per_campaign)
            .cloned()
            .collect::<Vec<_>>();
        if !coherent.is_empty()
            && build_lesson(&coherent)
                .map(|lesson| cohort_has_promotable_growth_subject(&coherent, &lesson))
                .unwrap_or(false)
        {
            return coherent;
        }
    }

    // A score-only prefix can be saturated by implementation-shaped files and
    // hide the lower-scored verification or typed contract that gives those
    // files meaning. Seed the bounded fallback with one observation for each
    // authoritative role, then fill the remaining capacity by score order.
    // This is selection only: it does not invent evidence or relax promotion.
    let maximum = config.resources.max_observations_per_campaign;
    let mut selected = Vec::new();
    let role_candidates = [
        observations.iter().find(|observation| {
            (!observation.public_contract_deltas.is_empty()
                && validate_public_contract_deltas(&observation.public_contract_deltas).is_ok())
                || observation
                    .performance_metrics
                    .iter()
                    .any(PerformanceMetricEvidence::has_executable_knowledge)
        }),
        observations.iter().find(|observation| {
            observation
                .signals
                .iter()
                .any(|signal| signal == "VERIFIED_PASS")
        }),
        observations.iter().find(|observation| {
            observation.work_kind != WorkKind::Verification
                || observation
                    .signals
                    .iter()
                    .any(|signal| signal == "MUTUAL_REVALIDATION_GAP")
        }),
    ];
    for observation in role_candidates.into_iter().flatten() {
        if selected.len() >= maximum {
            break;
        }
        if !selected.iter().any(|existing: &LearningObservation| {
            existing.observation_id == observation.observation_id
        }) {
            selected.push(observation.clone());
        }
    }
    for observation in observations {
        if selected.len() >= maximum {
            break;
        }
        if !selected
            .iter()
            .any(|existing| existing.observation_id == observation.observation_id)
        {
            selected.push(observation.clone());
        }
    }
    if selected.is_empty()
        || build_lesson(&selected)
            .map(|lesson| cohort_has_promotable_growth_subject(&selected, &lesson))
            .unwrap_or(false)
    {
        return selected;
    }

    // Try one bounded substitution and retain the first evidence-complete
    // cohort. This changes no score or acceptance rule.
    for evidence in observations.iter().skip(selected.len()) {
        for replace_index in (0..selected.len()).rev() {
            let mut trial = selected.clone();
            trial[replace_index] = evidence.clone();
            if build_lesson(&trial)
                .map(|lesson| cohort_has_promotable_growth_subject(&trial, &lesson))
                .unwrap_or(false)
            {
                selected = trial;
                return selected;
            }
        }
    }
    selected
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CampaignPreflightDiagnostic {
    schema: String,
    cohort_sha256: String,
    observation_ids: Vec<String>,
    reason: String,
    campaign_started: bool,
    failure_budget_consumed: bool,
    created_at_ms: u64,
}

fn campaign_preflight_ready(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Result<bool, String> {
    let chosen = selected_campaign_observations(config, observations);
    if chosen.is_empty() {
        return Ok(false);
    }
    let lesson = build_lesson(&chosen)?;
    if cohort_has_promotable_growth_subject(&chosen, &lesson) {
        return Ok(true);
    }
    let observation_ids = chosen
        .iter()
        .map(|observation| observation.observation_id.clone())
        .collect::<Vec<_>>();
    let cohort_sha256 = json_sha256(&observation_ids)?;
    let reason = if !lesson_has_verification_evidence(&lesson) {
        "NO_VERIFICATION_EVIDENCE"
    } else if !lesson_has_growth_subject(&lesson) {
        "NO_GROWTH_SUBJECT"
    } else if !lesson_has_executable_knowledge(&lesson) {
        "NO_EXECUTABLE_TYPED_KNOWLEDGE"
    } else {
        "COHORT_NOT_PROMOTABLE"
    };
    let diagnostic = CampaignPreflightDiagnostic {
        schema: SUPERVISOR_SCHEMA.to_string(),
        cohort_sha256: cohort_sha256.clone(),
        observation_ids,
        reason: reason.to_string(),
        campaign_started: false,
        failure_budget_consumed: false,
        created_at_ms: now_ms(),
    };
    let path = config
        .state_dir
        .join("diagnostics")
        .join(format!("preflight_{cohort_sha256}.json"));
    if !path.exists() {
        write_immutable_json(&path, &diagnostic)?;
    }
    Ok(false)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DeferredNonExecutableCohortReceipt {
    schema: String,
    generation: u64,
    cohort_sha256: String,
    observation_ids: Vec<String>,
    verification_evidence_sha256: Vec<String>,
    reason: String,
    receipt_sha256: String,
}

/// Removes a validated but non-executable cohort from the active campaign
/// queue while preserving every immutable observation and a causal receipt.
/// A later typed work event creates a distinct observation identity and remains
/// eligible; prose or structural scores never acquire synthesis authority.
fn defer_verified_non_executable_cohort(
    config: &GrowthSupervisorConfig,
    generation: u64,
    index: &mut FileIndex,
    observations: &mut Vec<LearningObservation>,
) -> Result<usize, String> {
    let chosen = selected_campaign_observations(config, observations);
    if chosen.is_empty() {
        return Ok(0);
    }
    let lesson = build_lesson(&chosen)?;
    if !lesson_has_verification_evidence(&lesson)
        || !lesson_has_growth_subject(&lesson)
        || lesson_has_executable_knowledge(&lesson)
    {
        return Ok(0);
    }
    let observation_ids = chosen
        .iter()
        .map(|observation| observation.observation_id.clone())
        .collect::<Vec<_>>();
    let mut verification_evidence_sha256 = chosen
        .iter()
        .flat_map(|observation| observation.verification_evidence_sha256.iter().cloned())
        .collect::<Vec<_>>();
    verification_evidence_sha256.sort();
    verification_evidence_sha256.dedup();
    let cohort_sha256 = json_sha256(&observation_ids)?;
    let mut receipt = DeferredNonExecutableCohortReceipt {
        schema: "B_CORE_DEFERRED_NON_EXECUTABLE_COHORT_1".to_string(),
        generation,
        cohort_sha256: cohort_sha256.clone(),
        observation_ids: observation_ids.clone(),
        verification_evidence_sha256,
        reason: "VERIFIED_BUT_NO_EXECUTABLE_TYPED_KNOWLEDGE".to_string(),
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = json_sha256(&receipt)?;
    let diagnostics = config.state_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics)
        .map_err(|error| format!("DEFERRED_COHORT_DIAGNOSTICS_CREATE:{error}"))?;
    let receipt_path = diagnostics.join(format!("deferred_cohort_{cohort_sha256}.json"));
    if !receipt_path.exists() {
        write_immutable_json(&receipt_path, &receipt)?;
    }
    let chosen_ids = observation_ids.iter().cloned().collect::<BTreeSet<_>>();
    index
        .consumed_observation_ids
        .extend(chosen_ids.iter().cloned());
    observations.retain(|observation| !chosen_ids.contains(&observation.observation_id));
    save_index(config, index)?;
    Ok(chosen_ids.len())
}

fn consume_semantic_revalidation(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
    memory: &GrowthMemory,
    observations: &[LearningObservation],
) -> Result<Option<usize>, String> {
    let chosen = selected_campaign_observations(config, observations);
    if chosen.is_empty() {
        return Ok(None);
    }
    let lesson = build_lesson(&chosen)?;
    if !cohort_has_promotable_growth_subject(&chosen, &lesson)
        || !memory_contains_semantic_lesson(memory, &lesson)?
    {
        return Ok(None);
    }
    let consumed_observation_ids = chosen
        .iter()
        .map(|observation| observation.observation_id.clone())
        .collect::<Vec<_>>();
    resolve_intrinsic_observation_outcomes(config, state, &consumed_observation_ids, false)?;
    for observation in &chosen {
        index
            .consumed_observation_ids
            .insert(observation.observation_id.clone());
        if let Some(event_id) = &observation.work_event_id {
            index.consumed_work_event_ids.insert(event_id.clone());
        }
    }
    save_index(config, index)?;
    state.semantic_revalidation_events = state.semantic_revalidation_events.saturating_add(1);
    state.redundant_observations_consumed = state
        .redundant_observations_consumed
        .saturating_add(chosen.len().min(u64::MAX as usize) as u64);
    state.diagnostic_policy.resolve_consumed_action_outcome(
        state.generation,
        true,
        &consumed_observation_ids,
    );
    state.plateau_scans = state.plateau_scans.saturating_add(1);
    Ok(Some(chosen.len()))
}

fn campaign_dir(config: &GrowthSupervisorConfig, campaign_id: &str) -> PathBuf {
    config.state_dir.join("campaigns").join(campaign_id)
}

fn freeze_new_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    observations: &[LearningObservation],
) -> Result<CampaignFreeze, String> {
    let chosen = selected_campaign_observations(config, observations);
    if chosen.is_empty() {
        return Err("NO_OBSERVATIONS_TO_FREEZE".to_string());
    }
    let observation_sha256 = chosen
        .iter()
        .map(json_sha256)
        .collect::<Result<Vec<_>, _>>()?;
    let generation = state.generation.saturating_add(1);
    let proposer = std::env::current_exe().map_err(|error| format!("CURRENT_EXE:{error}"))?;
    let proposer_executable_sha256 = file_sha256(&proposer, 512 * 1024 * 1024)?;
    let verifier_executable_sha256 = file_sha256(&config.verifier_executable, 512 * 1024 * 1024)?;
    if proposer_executable_sha256 == verifier_executable_sha256 {
        return Err("PROPOSER_VERIFIER_BINARY_COLLISION".to_string());
    }
    let campaign_id = format!(
        "G{:020}-{}",
        generation,
        &sha256(
            format!(
                "{}:{}:{}",
                observation_sha256.join(":"),
                proposer_executable_sha256,
                state.sequence
            )
            .as_bytes()
        )[..16]
    );
    let seed_hash = sha256(
        format!(
            "{}:{}:{}",
            state.current_memory_sha256,
            observation_sha256.join(":"),
            generation
        )
        .as_bytes(),
    );
    let seed =
        u64::from_str_radix(&seed_hash[..16], 16).map_err(|error| format!("SEED_PARSE:{error}"))?;
    let freeze = CampaignFreeze {
        schema: SUPERVISOR_SCHEMA.to_string(),
        campaign_id: campaign_id.clone(),
        generation,
        predecessor_memory_sha256: state.current_memory_sha256.clone(),
        config_sha256: state.config_sha256.clone(),
        observation_ids: chosen
            .iter()
            .map(|observation| observation.observation_id.clone())
            .collect(),
        observation_sha256,
        proposer_executable_sha256,
        verifier_executable_sha256,
        seed,
        budget_observations: chosen.len(),
        frozen_before_candidate: true,
        operator_selected_difficulty: false,
        human_difficulty_escalation_events: 0,
        created_at_ms: now_ms(),
    };
    let directory = campaign_dir(config, &campaign_id);
    fs::create_dir_all(&directory).map_err(|error| format!("CAMPAIGN_DIR:{error}"))?;
    let predecessor_memory = load_memory(config, state.generation)?;
    if json_sha256(&predecessor_memory)? != state.current_memory_sha256 {
        return Err("PREDECESSOR_MEMORY_CHANGED_DURING_FREEZE".to_string());
    }
    write_immutable_json(
        &directory.join("predecessor_memory.json"),
        &predecessor_memory,
    )?;
    write_immutable_json(&directory.join("freeze.json"), &freeze)?;
    for observation in &chosen {
        write_immutable_json(
            &directory.join(format!("observation_{}.json", observation.observation_id)),
            observation,
        )?;
    }
    state.campaigns_started = state.campaigns_started.saturating_add(1);
    state.pending_campaign_id = Some(campaign_id);
    save_transition(
        config,
        state,
        SupervisorPhase::CampaignFrozen,
        "CAMPAIGN_INPUT_SEED_BUDGET_AND_VERIFIER_FROZEN",
    )?;
    Ok(freeze)
}

fn load_campaign_observations(
    config: &GrowthSupervisorConfig,
    freeze: &CampaignFreeze,
) -> Result<Vec<LearningObservation>, String> {
    let directory = campaign_dir(config, &freeze.campaign_id);
    let mut observations = Vec::new();
    for (index, observation_id) in freeze.observation_ids.iter().enumerate() {
        let observation: LearningObservation =
            read_json(&directory.join(format!("observation_{observation_id}.json")))?;
        if observation.observation_id != *observation_id
            || json_sha256(&observation)? != freeze.observation_sha256[index]
            || observation.exact_source_fragments_stored != 0
            || observation.raw_source_bytes_stored != 0
        {
            return Err("FROZEN_OBSERVATION_INTEGRITY_FAILURE".to_string());
        }
        observations.push(observation);
    }
    Ok(observations)
}

fn build_candidate(
    config: &GrowthSupervisorConfig,
    freeze: &CampaignFreeze,
) -> Result<LearningCandidate, String> {
    let observations = load_campaign_observations(config, freeze)?;
    let lesson = build_lesson(&observations)?;
    let predecessor: GrowthMemory =
        read_json(&campaign_dir(config, &freeze.campaign_id).join("predecessor_memory.json"))?;
    if json_sha256(&predecessor)? != freeze.predecessor_memory_sha256 {
        return Err("CANDIDATE_PREDECESSOR_MEMORY_BINDING_FAILURE".to_string());
    }
    let generative_cycle = run_generative_cycle(
        &predecessor.generative,
        &generative_input(&lesson),
        freeze.seed,
    )?;
    Ok(LearningCandidate {
        schema: SUPERVISOR_SCHEMA.to_string(),
        campaign_id: freeze.campaign_id.clone(),
        freeze_sha256: json_sha256(freeze)?,
        generation: freeze.generation,
        predecessor_memory_sha256: freeze.predecessor_memory_sha256.clone(),
        total_learning_score: observations
            .iter()
            .map(|observation| u32::from(observation.learning_score))
            .sum(),
        observation_ids: freeze.observation_ids.clone(),
        lesson,
        generative_cycle,
        raw_source_bytes: 0,
        exact_source_fragments: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
        self_approval_events: 0,
        difficulty_escalation_events: 0,
    })
}

fn generative_input(lesson: &LearnedCompositionLesson) -> GenerativeInput {
    GenerativeInput {
        source_lesson_id: lesson.lesson_id.clone(),
        diagnostic_signals: lesson.diagnostic_signals.clone(),
        observed_composition_roles: lesson.composition_recipe.clone(),
        learning_score: lesson.learning_score,
        verification_evidence_count: lesson.evidence_observation_sha256.len(),
        measured_performance_gain: lesson
            .performance_metrics
            .iter()
            .any(PerformanceMetricEvidence::has_executable_knowledge),
        typed_behavior_goals: lesson
            .public_contract_deltas
            .iter()
            .flat_map(|delta| delta.typed_behavior_goals.iter().cloned())
            .take(MAX_TYPED_BEHAVIOR_GOALS_PER_GENERATIVE_INPUT)
            .collect(),
        executable_performance_operators: executable_performance_operators(lesson),
    }
}

fn plateau_generative_input_from_lessons(
    lessons: &[&LearnedCompositionLesson],
) -> Result<Option<(GenerativeInput, Vec<PublicContractDeltaIR>)>, String> {
    if lessons.len() < 2
        || lessons
            .iter()
            .map(|lesson| lesson.lesson_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != lessons.len()
        || lessons
            .iter()
            .any(|lesson| !lesson_has_executable_knowledge(lesson))
    {
        return Ok(None);
    }
    let mut diagnostic_signals = lessons
        .iter()
        .flat_map(|lesson| lesson.diagnostic_signals.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(12)
        .collect::<Vec<_>>();
    diagnostic_signals.push("AUTONOMOUS_INTRINSIC_CURIOSITY_PROBE".to_string());
    let mut observed_composition_roles = lessons
        .iter()
        .flat_map(|lesson| lesson.composition_recipe.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(12)
        .collect::<Vec<_>>();
    let lesson_identity = lessons
        .iter()
        .map(|lesson| lesson.lesson_id.as_str())
        .collect::<Vec<_>>()
        .join(":");
    let mut delta_index = BTreeMap::new();
    for delta in lessons
        .iter()
        .flat_map(|lesson| lesson.public_contract_deltas.iter())
    {
        delta_index
            .entry(delta.delta_id.clone())
            .or_insert_with(|| delta.clone());
    }
    let public_contract_deltas = delta_index
        .into_values()
        .take(MAX_PUBLIC_CONTRACT_DELTAS_PER_EVENT)
        .collect::<Vec<_>>();
    let base_typed_behavior_goals = public_contract_deltas
        .iter()
        .flat_map(|delta| delta.typed_behavior_goals.iter().cloned())
        .take(MAX_TYPED_BEHAVIOR_GOALS_PER_GENERATIVE_INPUT)
        .collect::<Vec<_>>();
    let compound_typed_behavior_goals =
        derive_compound_typed_behavior_goals(&base_typed_behavior_goals)?;
    if !compound_typed_behavior_goals.is_empty() {
        diagnostic_signals.push("COMPOUND_TYPED_GOAL_DERIVED".to_string());
        observed_composition_roles.push("TYPED_GOAL_FUNCTIONAL_COMPOSITION".to_string());
    }
    let mut seen_goal_ids = BTreeSet::new();
    let mut typed_behavior_goals = Vec::new();
    for goal in compound_typed_behavior_goals
        .into_iter()
        .chain(base_typed_behavior_goals)
    {
        if seen_goal_ids.insert(goal.goal_id.clone()) {
            typed_behavior_goals.push(goal);
        }
        if typed_behavior_goals.len() >= MAX_TYPED_BEHAVIOR_GOALS_PER_GENERATIVE_INPUT {
            break;
        }
    }
    Ok(Some((
        GenerativeInput {
            source_lesson_id: format!(
                "INTRINSIC-CROSS-LESSON-{}",
                &sha256(lesson_identity.as_bytes())[..24]
            ),
            diagnostic_signals,
            observed_composition_roles,
            learning_score: lessons
                .iter()
                .map(|lesson| lesson.learning_score)
                .max()
                .unwrap_or(0),
            verification_evidence_count: lessons
                .iter()
                .map(|lesson| lesson.evidence_observation_sha256.len())
                .sum::<usize>()
                .min(16),
            measured_performance_gain: lessons.iter().any(|lesson| {
                lesson
                    .performance_metrics
                    .iter()
                    .any(PerformanceMetricEvidence::has_executable_knowledge)
            }),
            typed_behavior_goals,
            executable_performance_operators: {
                let mut operators = BTreeMap::new();
                for operator in lessons
                    .iter()
                    .flat_map(|lesson| executable_performance_operators(lesson))
                {
                    operators
                        .entry(operator.operator_id.clone())
                        .or_insert(operator);
                }
                operators.into_values().collect()
            },
        },
        public_contract_deltas,
    )))
}

#[cfg(test)]
fn plateau_generative_input(
    memory: &GrowthMemory,
) -> Result<Option<(GenerativeInput, Vec<PublicContractDeltaIR>)>, String> {
    let mut lessons = memory
        .lessons
        .iter()
        .rev()
        .filter(|lesson| lesson_has_executable_knowledge(lesson))
        .take(2)
        .collect::<Vec<_>>();
    lessons.reverse();
    plateau_generative_input_from_lessons(&lessons)
}

fn plateau_promotion_contract_deltas(
    base_deltas: &[PublicContractDeltaIR],
    cycle: &GenerativeCycleResult,
) -> Result<Vec<PublicContractDeltaIR>, String> {
    let mut compound_goals = BTreeMap::new();
    let mut artifact_sha256s = Vec::new();
    if cycle.frontier_advance {
        if let Some(receipt) = cycle
            .behavioral_execution_receipt
            .as_ref()
            .filter(|receipt| receipt.executed && validate_behavioral_execution_receipt(cycle))
        {
            for artifact in &receipt.verified_artifacts {
                let Some(goal) = artifact
                    .typed_behavior_goal
                    .as_ref()
                    .filter(|goal| goal.goal_id.starts_with("compound_"))
                else {
                    continue;
                };
                if artifact.cases_executed == 0
                    || artifact.cases_passed != artifact.cases_executed
                    || compound_goals
                        .insert(goal.goal_id.clone(), goal.clone())
                        .is_some()
                {
                    return Err("PLATEAU_COMPOUND_PROGRAM_EVIDENCE_INVALID".to_string());
                }
                artifact_sha256s.push(artifact.artifact_sha256.clone());
            }
        }
    }
    if compound_goals.is_empty() {
        return Ok(base_deltas.to_vec());
    }
    if compound_goals.len() > MAX_TYPED_BEHAVIOR_GOALS_PER_DELTA {
        return Err("PLATEAU_COMPOUND_PROGRAM_GOAL_BOUND".to_string());
    }
    artifact_sha256s.sort();
    artifact_sha256s.dedup();
    let behavioral_sha256 = cycle
        .behavioral_verification_sha256
        .as_deref()
        .ok_or_else(|| "PLATEAU_COMPOUND_PROGRAM_RECEIPT_MISSING".to_string())?;
    let identity = json_sha256(&(
        "B_CORE_PLATEAU_COMPOUND_PROGRAM_DELTA_1",
        behavioral_sha256,
        &artifact_sha256s,
        compound_goals.keys().collect::<Vec<_>>(),
    ))?;
    let mut compound_delta = PublicContractDeltaIR {
        schema: PUBLIC_CONTRACT_DELTA_SCHEMA.to_string(),
        delta_id: format!("compound-program-{}", &identity[..32]),
        observed_behavior: "verified component goals existed only as separate executable programs"
            .to_string(),
        expected_behavior:
            "the evidence-bound functional join is retained as one reusable compound ProgramIR"
                .to_string(),
        target_symbols: vec![
            "compound_typed_goal::derive_compound_typed_behavior_goals".to_string(),
            "generative_growth::execute_composer".to_string(),
        ],
        typed_behavior_goals: compound_goals.into_values().collect(),
        provenance: std::iter::once(format!(
            "BEHAVIORAL_COMPOSITION_RECEIPT:{behavioral_sha256}"
        ))
        .chain(
            artifact_sha256s
                .iter()
                .map(|artifact| format!("COMPOUND_PROGRAM_ARTIFACT:{artifact}")),
        )
        .collect(),
    };
    let contract_binding = format!(
        "PUBLIC_CONTRACT_DELTA_SHA256:{}",
        public_contract_delta_binding_sha256(&compound_delta)?
    );
    let delta_id_binding = format!("PUBLIC_CONTRACT_DELTA_ID:{}", compound_delta.delta_id);
    for goal in &mut compound_delta.typed_behavior_goals {
        goal.provenance.retain(|item| {
            !item.starts_with("PUBLIC_CONTRACT_DELTA_SHA256:")
                && !item.starts_with("PUBLIC_CONTRACT_DELTA_ID:")
        });
        goal.provenance.push(delta_id_binding.clone());
        goal.provenance.push(contract_binding.clone());
        goal.provenance.sort();
        goal.provenance.dedup();
    }

    let mut deltas = Vec::with_capacity(MAX_PUBLIC_CONTRACT_DELTAS_PER_EVENT);
    deltas.push(compound_delta);
    deltas.extend(
        base_deltas
            .iter()
            .take(MAX_PUBLIC_CONTRACT_DELTAS_PER_EVENT.saturating_sub(1))
            .cloned(),
    );
    validate_public_contract_deltas(&deltas)?;
    Ok(deltas)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlateauCuriosityCandidate {
    hypothesis: IntrinsicCuriosityHypothesis,
    input: GenerativeInput,
    public_contract_deltas: Vec<PublicContractDeltaIR>,
}

fn plateau_curiosity_candidates(
    state: &SupervisorState,
    memory: &GrowthMemory,
) -> Result<Vec<PlateauCuriosityCandidate>, String> {
    let mut recent_lessons = memory
        .lessons
        .iter()
        .rev()
        .filter(|lesson| lesson_has_executable_knowledge(lesson))
        .take(8)
        .collect::<Vec<_>>();
    recent_lessons.reverse();
    if recent_lessons.len() < 2 {
        return Ok(Vec::new());
    }

    let prediction_uncertainty = state
        .generative_prediction_absolute_error_total
        .checked_div(state.generative_calibrated_prediction_records.max(1))
        .unwrap_or(0)
        .min(100) as u16;
    let mut lesson_groups = Vec::new();
    for first in 0..recent_lessons.len() {
        for second in first + 1..recent_lessons.len() {
            lesson_groups.push(vec![recent_lessons[first], recent_lessons[second]]);
            for third in second + 1..recent_lessons.len() {
                lesson_groups.push(vec![
                    recent_lessons[first],
                    recent_lessons[second],
                    recent_lessons[third],
                ]);
            }
        }
    }
    let mut candidates = Vec::new();
    for lessons in lesson_groups {
        let Some((input, public_contract_deltas)) =
            plateau_generative_input_from_lessons(&lessons)?
        else {
            continue;
        };
        let lesson_ids = lessons
            .iter()
            .map(|lesson| lesson.lesson_id.clone())
            .collect::<Vec<_>>();
        let signal_diversity = input
            .diagnostic_signals
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            .min(100) as u16;
        let executable_goal_count = input.typed_behavior_goals.len().min(100) as u16;
        let structural_novelty = input
            .observed_composition_roles
            .len()
            .saturating_add(public_contract_deltas.len())
            .min(100) as u16;
        let expected_information_gain = executable_goal_count
            .saturating_mul(12)
            .saturating_add(signal_diversity.saturating_mul(3))
            .saturating_add(structural_novelty.saturating_mul(4))
            .saturating_add(prediction_uncertainty / 2)
            .min(100);
        let predicted_cost_units = (lessons.len() as u16)
            .saturating_mul(8)
            .saturating_add(executable_goal_count.saturating_mul(4))
            .saturating_add(input.executable_performance_operators.len().min(20) as u16)
            .min(100);
        let hypothesis_id = sha256(
            format!(
                "INTRINSIC_CURIOSITY_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}:{}:{}",
                state.current_memory_sha256,
                lesson_ids.join(":")
            )
            .as_bytes(),
        );
        candidates.push(PlateauCuriosityCandidate {
            hypothesis: IntrinsicCuriosityHypothesis {
                hypothesis_id,
                lesson_ids,
                signal_diversity,
                executable_goal_count,
                structural_novelty,
                prediction_uncertainty,
                expected_information_gain,
                predicted_cost_units,
            },
            input,
            public_contract_deltas,
        });
    }
    candidates.sort_by(|left, right| {
        state
            .intrinsic_drive
            .score(&right.hypothesis)
            .cmp(&state.intrinsic_drive.score(&left.hypothesis))
            .then_with(|| {
                left.hypothesis
                    .hypothesis_id
                    .cmp(&right.hypothesis.hypothesis_id)
            })
    });
    candidates.truncate(MAX_INTRINSIC_CURIOSITY_HYPOTHESES);
    Ok(candidates)
}

fn refine_classifier_from_capability_outcome(
    classifier: &mut ClassifierMemory,
    generation: u64,
    lesson: &LearnedCompositionLesson,
    applied_policy_signals: &[String],
    behavioral_frontier_advance: bool,
    behavioral_verification_sha256: Option<&String>,
) {
    let measured_performance_gain = lesson
        .performance_metrics
        .iter()
        .any(PerformanceMetricEvidence::has_executable_knowledge);
    let verified_behavioral_frontier =
        behavioral_frontier_advance && behavioral_verification_sha256.is_some();
    let supported = verified_behavioral_frontier || measured_performance_gain;
    let mut considered_signals = lesson
        .diagnostic_signals
        .iter()
        .chain(applied_policy_signals)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    considered_signals.sort();
    let mut weight_deltas = Vec::with_capacity(considered_signals.len());
    for signal in &considered_signals {
        let before = classifier.signal_weights.get(signal).copied().unwrap_or(0);
        let after = if supported {
            before.saturating_add(1).min(5)
        } else {
            before
        };
        if supported {
            classifier.signal_weights.insert(signal.clone(), after);
        }
        weight_deltas.push(ClassifierWeightDelta {
            signal: signal.clone(),
            before,
            after,
        });
    }
    if supported {
        classifier.outcome_bound_refinements =
            classifier.outcome_bound_refinements.saturating_add(1);
    } else {
        classifier.unsupported_refinements_suppressed = classifier
            .unsupported_refinements_suppressed
            .saturating_add(1);
    }
    let refinement_identity = format!(
        "classifier:{generation}:{}:{}:{}:{}",
        lesson.lesson_id,
        considered_signals.join("|"),
        verified_behavioral_frontier,
        measured_performance_gain
    );
    let refinement_id = sha256(refinement_identity.as_bytes());
    classifier
        .refinement_events
        .push(ClassifierRefinementEvent {
            refinement_id,
            generation,
            source_lesson_id: lesson.lesson_id.clone(),
            evidence_observation_sha256: lesson.evidence_observation_sha256.clone(),
            considered_signals,
            weight_deltas,
            behavioral_frontier_advance: verified_behavioral_frontier,
            measured_performance_gain,
            behavioral_verification_sha256: behavioral_verification_sha256.cloned(),
            applied: supported,
        });
    while classifier.refinement_events.len() > MAX_CLASSIFIER_REFINEMENT_EVENTS {
        classifier.refinement_events.remove(0);
    }
}

fn receipt_hash_input(receipt: &GrowthVerificationReceipt) -> Result<String, String> {
    let mut clone = receipt.clone();
    clone.receipt_sha256.clear();
    json_sha256(&clone)
}

fn authority_seal_input(receipt: &GrowthVerificationReceipt) -> Result<String, String> {
    let mut clone = receipt.clone();
    clone.receipt_sha256.clear();
    clone.authority_seal.clear();
    json_sha256(&clone)
}

fn load_verifier_observations(
    freeze_path: &Path,
    freeze: &CampaignFreeze,
) -> Result<Vec<LearningObservation>, String> {
    let directory = freeze_path
        .parent()
        .ok_or_else(|| "VERIFIER_FREEZE_PARENT_MISSING".to_string())?;
    let mut observations = Vec::with_capacity(freeze.observation_ids.len());
    for (index, observation_id) in freeze.observation_ids.iter().enumerate() {
        let path = directory.join(format!("observation_{observation_id}.json"));
        let observation: LearningObservation = read_json(&path)?;
        if observation.observation_id != *observation_id
            || freeze.observation_sha256.get(index) != Some(&json_sha256(&observation)?)
            || observation.learning_value != LearningValue::High
            || (observation.work_outcome == WorkOutcome::Pass
                && observation.verification_evidence_sha256.is_empty())
            || observation.exact_source_fragments_stored != 0
            || observation.raw_source_bytes_stored != 0
        {
            return Err("VERIFIER_FROZEN_OBSERVATION_INTEGRITY_FAILURE".to_string());
        }
        observations.push(observation);
    }
    Ok(observations)
}

fn load_verifier_predecessor_memory(
    freeze_path: &Path,
    freeze: &CampaignFreeze,
) -> Result<GrowthMemory, String> {
    let directory = freeze_path
        .parent()
        .ok_or_else(|| "VERIFIER_FREEZE_PARENT_MISSING".to_string())?;
    let memory: GrowthMemory = read_json(&directory.join("predecessor_memory.json"))?;
    if memory.schema != SUPERVISOR_SCHEMA
        || memory.generation.saturating_add(1) != freeze.generation
        || json_sha256(&memory)? != freeze.predecessor_memory_sha256
        || memory.evaluator.schema != "B_CORE_GROWTH_EVALUATOR_MEMORY_1"
    {
        return Err("VERIFIER_PREDECESSOR_MEMORY_INTEGRITY_FAILURE".to_string());
    }
    Ok(memory)
}

fn candidate_matches_frozen_derivation(
    candidate: &LearningCandidate,
    expected_lesson: &LearnedCompositionLesson,
    expected_total: u32,
    expected_generative_cycle: &GenerativeCycleResult,
) -> bool {
    candidate.lesson == *expected_lesson
        && candidate.total_learning_score == expected_total
        && candidate.generative_cycle == *expected_generative_cycle
}

fn evaluator_self_audit(
    candidate: &LearningCandidate,
    expected_lesson: &LearnedCompositionLesson,
    expected_total: u32,
    predecessor: &GrowthMemory,
    proposed_evaluator: &EvaluatorMemory,
    seed: u64,
) -> EvaluatorSelfAudit {
    let baseline_generative_cycle = run_generative_cycle(
        &predecessor.generative,
        &generative_input(expected_lesson),
        seed,
    );
    let baseline_candidate_reconstructed = baseline_generative_cycle
        .as_ref()
        .map(|expected_cycle| {
            candidate_matches_frozen_derivation(
                candidate,
                expected_lesson,
                expected_total,
                expected_cycle,
            )
        })
        .unwrap_or(false);
    // Every predecessor lesson was already challenged before it entered the hash-chained,
    // immutable predecessor memory. Replaying the same field mutations for the entire history
    // made verification O(generation) without adding a new counterexample. Challenge the newly
    // proposed lesson against the complete current suite and preserve prior assurance through the
    // predecessor/evaluator hashes instead.
    let mutation_results = proposed_evaluator
        .challenge_suite
        .iter()
        .copied()
        .map(|mutation| {
            let mut mutant = candidate.clone();
            if let Ok(expected_cycle) = &baseline_generative_cycle {
                mutant.generative_cycle = expected_cycle.clone();
            }
            match mutation {
                EvaluatorMutationKind::EvidenceDigestSubstitution => {
                    mutant.lesson.evidence_observation_sha256 = vec!["0".repeat(64)];
                }
                EvaluatorMutationKind::AggregateScoreInflation => {
                    mutant.total_learning_score = mutant.total_learning_score.saturating_add(1);
                }
                EvaluatorMutationKind::LessonScoreInflation => {
                    mutant.lesson.learning_score = mutant.lesson.learning_score.saturating_add(1);
                }
                EvaluatorMutationKind::DiagnosticSignalInjection => {
                    mutant
                        .lesson
                        .diagnostic_signals
                        .push("UNSUPPORTED_AUTHORITY_SIGNAL".to_string());
                }
                EvaluatorMutationKind::CompositionRecipeMutation => {
                    mutant
                        .lesson
                        .composition_recipe
                        .push("UNSUPPORTED_ROLE".to_string());
                }
                EvaluatorMutationKind::WorkKindMutation => {
                    mutant.lesson.work_kinds.clear();
                }
                EvaluatorMutationKind::ApplicabilityMutation => {
                    mutant
                        .lesson
                        .applicability
                        .push("unfrozen scope".to_string());
                }
                EvaluatorMutationKind::VerificationObligationMutation => {
                    mutant.lesson.verification_obligations.clear();
                }
                EvaluatorMutationKind::RawSourceFlagInjection => {
                    mutant.lesson.raw_source_bytes_present = true;
                }
                EvaluatorMutationKind::EvidenceRemoval => {
                    mutant.lesson.evidence_observation_sha256.clear();
                }
            }
            let survived = candidate_matches_frozen_derivation(
                &mutant,
                expected_lesson,
                expected_total,
                baseline_generative_cycle
                    .as_ref()
                    .unwrap_or(&candidate.generative_cycle),
            );
            EvaluatorMutationResult {
                mutation,
                expected_reject: true,
                rejected: !survived,
                survived,
            }
        })
        .collect::<Vec<_>>();
    let mutation_survivors = mutation_results
        .iter()
        .filter(|result| result.survived)
        .count();
    let post_challenge_core_revalidated = baseline_candidate_reconstructed
        && mutation_survivors == 0
        && proposed_evaluator.generation == predecessor.evaluator.generation.saturating_add(1)
        && proposed_evaluator.predecessor_sha256 == json_sha256(&predecessor.evaluator).ok();
    EvaluatorSelfAudit {
        schema: "B_CORE_GROWTH_EVALUATOR_SELF_AUDIT_1".to_string(),
        challenger_identity: "CORE_SCHEMA_DERIVED_MUTATION_CHALLENGER".to_string(),
        evaluator_identity: "INDEPENDENT_LOCAL_GROWTH_VERIFIER".to_string(),
        baseline_candidate_reconstructed,
        mutation_cases: mutation_results.len(),
        mutation_survivors,
        pass: post_challenge_core_revalidated,
        active_evaluator_generation: predecessor.evaluator.generation,
        proposed_evaluator_generation: proposed_evaluator.generation,
        proposed_evaluator_memory_sha256: json_sha256(proposed_evaluator).unwrap_or_default(),
        knowledge_challenge_cases: mutation_results.len(),
        challenge_suite_expanded: proposed_evaluator.challenge_suite.len()
            > predecessor.evaluator.challenge_suite.len(),
        post_challenge_core_revalidated,
        mutation_results,
        evaluator_self_approval_events: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    }
}

fn failed_evaluator_self_audit() -> EvaluatorSelfAudit {
    EvaluatorSelfAudit {
        schema: "B_CORE_GROWTH_EVALUATOR_SELF_AUDIT_1".to_string(),
        challenger_identity: "CORE_SCHEMA_DERIVED_MUTATION_CHALLENGER".to_string(),
        evaluator_identity: "INDEPENDENT_LOCAL_GROWTH_VERIFIER".to_string(),
        baseline_candidate_reconstructed: false,
        mutation_results: Vec::new(),
        mutation_cases: 0,
        mutation_survivors: 0,
        pass: false,
        active_evaluator_generation: 0,
        proposed_evaluator_generation: 0,
        proposed_evaluator_memory_sha256: String::new(),
        knowledge_challenge_cases: 0,
        challenge_suite_expanded: false,
        post_challenge_core_revalidated: false,
        evaluator_self_approval_events: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    }
}

pub fn run_verifier_request(
    request: &VerifierRequest,
) -> Result<GrowthVerificationReceipt, String> {
    if request.schema != VERIFIER_SCHEMA {
        return Err("VERIFIER_REQUEST_SCHEMA_INVALID".to_string());
    }
    let verifier = std::env::current_exe().map_err(|error| format!("CURRENT_EXE:{error}"))?;
    let verifier_sha256 = file_sha256(&verifier, 512 * 1024 * 1024)?;
    if verifier_sha256 != request.expected_verifier_sha256 {
        return Err("VERIFIER_BINARY_IDENTITY_MISMATCH".to_string());
    }
    let freeze: CampaignFreeze = read_json(&request.freeze_path)?;
    let candidate: LearningCandidate = read_json(&request.candidate_path)?;
    let freeze_sha256 = json_sha256(&freeze)?;
    let candidate_sha256 = json_sha256(&candidate)?;
    let mut reasons = Vec::new();
    if freeze_sha256 != request.expected_freeze_sha256 {
        reasons.push("FREEZE_HASH_MISMATCH".to_string());
    }
    if candidate_sha256 != request.expected_candidate_sha256 {
        reasons.push("CANDIDATE_HASH_MISMATCH".to_string());
    }
    if freeze.schema != SUPERVISOR_SCHEMA
        || candidate.schema != SUPERVISOR_SCHEMA
        || freeze.campaign_id != candidate.campaign_id
        || freeze.generation != candidate.generation
        || freeze.predecessor_memory_sha256 != candidate.predecessor_memory_sha256
        || candidate.freeze_sha256 != freeze_sha256
    {
        reasons.push("CAMPAIGN_BINDING_FAILURE".to_string());
    }
    if !freeze.frozen_before_candidate
        || freeze.operator_selected_difficulty
        || freeze.human_difficulty_escalation_events != 0
        || freeze.budget_observations == 0
        || freeze.budget_observations > request.max_observations
        || freeze.observation_ids.len() != freeze.budget_observations
        || freeze.observation_sha256.len() != freeze.budget_observations
        || candidate.observation_ids != freeze.observation_ids
    {
        reasons.push("FREEZE_OR_BUDGET_INVALID".to_string());
    }
    if freeze.proposer_executable_sha256 == verifier_sha256
        || freeze.verifier_executable_sha256 != verifier_sha256
    {
        reasons.push("PROPOSER_VERIFIER_INDEPENDENCE_FAILURE".to_string());
    }
    if candidate.raw_source_bytes != 0
        || candidate.exact_source_fragments != 0
        || candidate.lesson.raw_source_bytes_present
        || candidate.lesson.exact_source_fragment_present
        || candidate.lesson.exact_patch_data_present
    {
        reasons.push("RAW_OR_EXACT_SOLUTION_DATA_PRESENT".to_string());
    }
    if candidate.codex_calls != 0
        || candidate.external_llm_calls != 0
        || candidate.network_reads != 0
        || candidate.network_writes != 0
        || candidate.self_approval_events != 0
        || candidate.difficulty_escalation_events != 0
    {
        reasons.push("FORBIDDEN_DEPENDENCY_OR_SELF_APPROVAL".to_string());
    }
    if candidate.generative_cycle.schema != crate::generative_growth::GENERATIVE_GROWTH_SCHEMA
        || !candidate
            .generative_cycle
            .prediction_recorded_before_composition
        || !candidate
            .generative_cycle
            .selected_from_precomposition_prediction
        || !candidate.generative_cycle.isolated_composition_executed
        || !candidate.generative_cycle.composition_typecheck_pass
        || candidate.generative_cycle.observed_value_is_heuristic_proxy
        || !validate_behavioral_execution_receipt(&candidate.generative_cycle)
        || (!candidate.generative_cycle.behavioral_composition_executed
            && (candidate
                .generative_cycle
                .behavioral_verification_sha256
                .is_some()
                || candidate.generative_cycle.frontier_advance
                || candidate.generative_cycle.productive_reuse
                || candidate.generative_cycle.applied_to_self_improvement
                || !candidate.generative_cycle.applied_policy_signals.is_empty()))
        || (candidate.generative_cycle.behavioral_composition_executed
            && candidate
                .generative_cycle
                .behavioral_verification_sha256
                .is_none())
        || candidate.generative_cycle.exact_source_fragments != 0
        || candidate.generative_cycle.codex_calls != 0
        || candidate.generative_cycle.external_llm_calls != 0
        || candidate.generative_cycle.network_reads != 0
        || candidate.generative_cycle.network_writes != 0
    {
        reasons.push("GENERATIVE_COMPOSITION_BOUNDARY_FAILURE".to_string());
    }
    if candidate.lesson.learning_score < request.minimum_learning_score
        || candidate.lesson.composition_recipe.len() < 2
        || candidate.lesson.evidence_observation_sha256.is_empty()
    {
        reasons.push("INSUFFICIENT_LEARNING_VALUE_OR_COMPOSITION".to_string());
    }
    if !lesson_has_verification_evidence(&candidate.lesson) {
        reasons.push("NO_PASS_OR_CODE_TEST_COHORT_EVIDENCE".to_string());
    }
    if !lesson_has_growth_subject(&candidate.lesson) {
        reasons.push("VERIFICATION_ONLY_COHORT_HAS_NO_GROWTH_SUBJECT".to_string());
    }
    if !lesson_has_executable_knowledge(&candidate.lesson) {
        reasons.push("TEXT_ONLY_LESSON_HAS_NO_EXECUTABLE_KNOWLEDGE".to_string());
    }
    let evaluator_self_audit = match (
        load_verifier_observations(&request.freeze_path, &freeze),
        load_verifier_predecessor_memory(&request.freeze_path, &freeze),
    ) {
        (Ok(observations), Ok(predecessor)) => {
            let expected_total = observations
                .iter()
                .map(|observation| u32::from(observation.learning_score))
                .sum::<u32>();
            match build_lesson(&observations) {
                Ok(expected_lesson) => {
                    if memory_contains_semantic_lesson(&predecessor, &expected_lesson)? {
                        reasons.push("DUPLICATE_SEMANTIC_LESSON".to_string());
                    }
                    match derive_next_evaluator_memory(
                        &predecessor.evaluator,
                        &predecessor.lessons,
                        &expected_lesson,
                    ) {
                        Ok(proposed_evaluator) => {
                            let audit = evaluator_self_audit(
                                &candidate,
                                &expected_lesson,
                                expected_total,
                                &predecessor,
                                &proposed_evaluator,
                                freeze.seed,
                            );
                            if !audit.baseline_candidate_reconstructed {
                                reasons.push(
                                    "CANDIDATE_NOT_DERIVED_FROM_FROZEN_OBSERVATIONS".to_string(),
                                );
                            }
                            if !audit.pass {
                                reasons.push("EVALUATOR_SELF_AUDIT_FAILED".to_string());
                            }
                            audit
                        }
                        Err(_) => {
                            reasons.push("EVALUATOR_MEMORY_EVOLUTION_FAILED".to_string());
                            failed_evaluator_self_audit()
                        }
                    }
                }
                Err(_) => {
                    reasons.push("FROZEN_OBSERVATION_LESSON_REBUILD_FAILED".to_string());
                    failed_evaluator_self_audit()
                }
            }
        }
        _ => {
            reasons.push("FROZEN_OBSERVATION_OR_MEMORY_BINDING_FAILURE".to_string());
            failed_evaluator_self_audit()
        }
    };
    let decision = if reasons.is_empty() {
        GrowthDecision::Accept
    } else {
        GrowthDecision::Reject
    };
    let mut receipt = GrowthVerificationReceipt {
        schema: VERIFIER_SCHEMA.to_string(),
        campaign_id: freeze.campaign_id,
        freeze_sha256,
        candidate_sha256,
        verifier_executable_sha256: verifier_sha256,
        decision,
        reasons,
        verifier_is_proposer: false,
        deterministic_checks_only: true,
        local_process: true,
        raw_source_bytes_observed: 0,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
        human_verification_decisions: 0,
        evaluator_self_audit,
        receipt_sha256: String::new(),
        authority_seal: String::new(),
    };
    receipt.authority_seal = authority_seal_input(&receipt)?;
    receipt.receipt_sha256 = receipt_hash_input(&receipt)?;
    Ok(receipt)
}

fn validate_receipt(
    freeze: &CampaignFreeze,
    candidate: &LearningCandidate,
    receipt: &GrowthVerificationReceipt,
) -> Result<(), String> {
    if receipt.schema != VERIFIER_SCHEMA
        || receipt.campaign_id != freeze.campaign_id
        || receipt.freeze_sha256 != json_sha256(freeze)?
        || receipt.candidate_sha256 != json_sha256(candidate)?
        || receipt.verifier_executable_sha256 != freeze.verifier_executable_sha256
        || receipt.verifier_is_proposer
        || !receipt.deterministic_checks_only
        || !receipt.local_process
        || receipt.raw_source_bytes_observed != 0
        || receipt.codex_calls != 0
        || receipt.external_llm_calls != 0
        || receipt.network_reads != 0
        || receipt.network_writes != 0
        || receipt.human_verification_decisions != 0
        || receipt.authority_seal != authority_seal_input(receipt)?
        || receipt.receipt_sha256 != receipt_hash_input(receipt)?
    {
        return Err("VERIFICATION_RECEIPT_INTEGRITY_FAILURE".to_string());
    }
    Ok(())
}

fn run_independent_verifier(
    config: &GrowthSupervisorConfig,
    freeze: &CampaignFreeze,
    candidate: &LearningCandidate,
) -> Result<GrowthVerificationReceipt, String> {
    let directory = campaign_dir(config, &freeze.campaign_id);
    let freeze_path = directory.join("freeze.json");
    let candidate_path = directory.join("candidate.json");
    let request_path = directory.join("verification_request.json");
    let receipt_path = directory.join("verification_receipt.json");
    if receipt_path.exists() {
        let receipt = read_json(&receipt_path)?;
        validate_receipt(freeze, candidate, &receipt)?;
        return Ok(receipt);
    }
    let request = VerifierRequest {
        schema: VERIFIER_SCHEMA.to_string(),
        freeze_path,
        candidate_path,
        expected_freeze_sha256: json_sha256(freeze)?,
        expected_candidate_sha256: json_sha256(candidate)?,
        expected_verifier_sha256: freeze.verifier_executable_sha256.clone(),
        minimum_learning_score: config.observation.minimum_learning_score,
        max_observations: config.resources.max_observations_per_campaign,
    };
    if !request_path.exists() {
        write_immutable_json(&request_path, &request)?;
    }
    let output = Command::new(&config.verifier_executable)
        .arg(&request_path)
        .arg(&receipt_path)
        .output()
        .map_err(|error| format!("VERIFIER_SPAWN:{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "VERIFIER_FAILED:{}:{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(1024)
                .collect::<String>()
        ));
    }
    let receipt: GrowthVerificationReceipt = read_json(&receipt_path)?;
    validate_receipt(freeze, candidate, &receipt)?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PlateauGenerativeProbeReceipt {
    schema: String,
    probe_id: String,
    predecessor_memory_sha256: String,
    intrinsic_attempt_sequence: u64,
    hypothesis: IntrinsicCuriosityHypothesis,
    input: GenerativeInput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    public_contract_deltas: Vec<PublicContractDeltaIR>,
    seed: u64,
    cycle: GenerativeCycleResult,
    intrinsic_attempt_pending: bool,
    observation: Option<LearningObservation>,
    receipt_sha256: String,
}

fn plateau_probe_receipt_hash(receipt: &PlateauGenerativeProbeReceipt) -> Result<String, String> {
    let mut clone = receipt.clone();
    clone.receipt_sha256.clear();
    json_sha256(&clone)
}

fn validate_plateau_probe_receipt(receipt: &PlateauGenerativeProbeReceipt) -> Result<(), String> {
    let behaviorally_verified = receipt.cycle.behavioral_composition_executed
        && validate_behavioral_execution_receipt(&receipt.cycle);
    let mut attempt_contract = IntrinsicDriveMemory::default();
    let expected_pending = attempt_contract.begin_attempt(
        &receipt.hypothesis,
        behaviorally_verified,
        receipt.cycle.frontier_advance_units,
        receipt.cycle.novel_verified_artifact_count,
    );
    if receipt.schema
        != format!("B_CORE_PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}")
        || receipt.probe_id.len() != 64
        || receipt.predecessor_memory_sha256.len() != 64
        || receipt.intrinsic_attempt_sequence == 0
        || receipt.hypothesis.hypothesis_id.len() != 64
        || receipt.input.source_lesson_id != receipt.cycle.source_lesson_id
        || receipt.hypothesis.lesson_ids.len() < 2
        || receipt.intrinsic_attempt_pending != expected_pending
        || receipt.receipt_sha256 != plateau_probe_receipt_hash(receipt)?
        || (receipt.observation.is_some() != expected_pending)
    {
        return Err("PLATEAU_GENERATIVE_PROBE_RECEIPT_DIVERGED".to_string());
    }
    Ok(())
}

fn reconcile_intrinsic_drive_receipts(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
) -> Result<(), String> {
    let probe_root = config.state_dir.join("generative_plateau_probes");
    if !probe_root.is_dir() {
        return Ok(());
    }
    let expected_schema =
        format!("B_CORE_PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}");
    let mut receipts = Vec::new();
    for entry in fs::read_dir(&probe_root)
        .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_ENTRY:{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_TYPE:{error}"))?
            .is_file()
        {
            continue;
        }
        let value: serde_json::Value = read_json(&entry.path())?;
        if value.get("schema").and_then(serde_json::Value::as_str) != Some(expected_schema.as_str())
        {
            continue;
        }
        let receipt: PlateauGenerativeProbeReceipt = serde_json::from_value(value)
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_PARSE:{error}"))?;
        if receipt.predecessor_memory_sha256 == state.current_memory_sha256 {
            validate_plateau_probe_receipt(&receipt)?;
            receipts.push(receipt);
        }
    }
    receipts.sort_by(|left, right| {
        left.intrinsic_attempt_sequence
            .cmp(&right.intrinsic_attempt_sequence)
            .then_with(|| left.probe_id.cmp(&right.probe_id))
    });
    for receipt in receipts {
        let pending = state.intrinsic_drive.begin_attempt(
            &receipt.hypothesis,
            receipt.cycle.behavioral_composition_executed
                && validate_behavioral_execution_receipt(&receipt.cycle),
            receipt.cycle.frontier_advance_units,
            receipt.cycle.novel_verified_artifact_count,
        );
        let already_resolved = state
            .intrinsic_drive
            .recent_outcomes
            .iter()
            .any(|outcome| outcome.hypothesis_id == receipt.hypothesis.hypothesis_id);
        if receipt.intrinsic_attempt_pending != pending && !already_resolved {
            return Err("INTRINSIC_ATTEMPT_RECEIPT_DIVERGED".to_string());
        }
    }
    Ok(())
}

fn resolve_intrinsic_observation_outcomes(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    observation_ids: &[String],
    campaign_accepted: bool,
) -> Result<usize, String> {
    if observation_ids.is_empty() {
        return Ok(0);
    }
    let requested = observation_ids.iter().collect::<BTreeSet<_>>();
    let probe_root = config.state_dir.join("generative_plateau_probes");
    if !probe_root.is_dir() {
        return Ok(0);
    }
    let expected_schema =
        format!("B_CORE_PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}");
    let mut matches = Vec::new();
    for entry in fs::read_dir(&probe_root)
        .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_ENTRY:{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_TYPE:{error}"))?
            .is_file()
        {
            continue;
        }
        let value: serde_json::Value = read_json(&entry.path())?;
        if value.get("schema").and_then(serde_json::Value::as_str) != Some(expected_schema.as_str())
        {
            continue;
        }
        let receipt: PlateauGenerativeProbeReceipt = serde_json::from_value(value)
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_PARSE:{error}"))?;
        if receipt
            .observation
            .as_ref()
            .is_some_and(|observation| requested.contains(&observation.observation_id))
        {
            validate_plateau_probe_receipt(&receipt)?;
            matches.push(receipt);
        }
    }
    matches.sort_by(|left, right| {
        left.intrinsic_attempt_sequence
            .cmp(&right.intrinsic_attempt_sequence)
            .then_with(|| left.probe_id.cmp(&right.probe_id))
    });
    let mut resolved = 0;
    for receipt in matches {
        if state
            .intrinsic_drive
            .resolve_attempt(&receipt.hypothesis.hypothesis_id, campaign_accepted)
            .is_some()
        {
            resolved += 1;
        }
    }
    Ok(resolved)
}

fn plateau_generative_probe_observation(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    memory: &GrowthMemory,
) -> Result<Option<LearningObservation>, String> {
    if !executable_generative_substrate_available(&memory.generative) {
        return Ok(None);
    }
    let candidates = plateau_curiosity_candidates(state, memory)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let probe_root = config.state_dir.join("generative_plateau_probes");
    for candidate in candidates {
        let probe_id = sha256(
            format!(
                "PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}:{}:{}:{}",
                state.current_memory_sha256, state.generation, candidate.hypothesis.hypothesis_id,
            )
            .as_bytes(),
        );
        let receipt_path = probe_root.join(format!("{probe_id}.json"));
        if receipt_path.is_file() {
            let existing: PlateauGenerativeProbeReceipt = read_json(&receipt_path)?;
            validate_plateau_probe_receipt(&existing)?;
            if existing.probe_id != probe_id
                || existing.predecessor_memory_sha256 != state.current_memory_sha256
                || existing.hypothesis != candidate.hypothesis
            {
                return Err("PLATEAU_GENERATIVE_PROBE_RECEIPT_DIVERGED".to_string());
            }
            let pending = state.intrinsic_drive.begin_attempt(
                &existing.hypothesis,
                existing.cycle.behavioral_composition_executed
                    && validate_behavioral_execution_receipt(&existing.cycle),
                existing.cycle.frontier_advance_units,
                existing.cycle.novel_verified_artifact_count,
            );
            let already_resolved = state
                .intrinsic_drive
                .recent_outcomes
                .iter()
                .any(|outcome| outcome.hypothesis_id == existing.hypothesis.hypothesis_id);
            if existing.intrinsic_attempt_pending != pending && !already_resolved {
                return Err("INTRINSIC_ATTEMPT_RECEIPT_DIVERGED".to_string());
            }
            if let Some(observation) = existing.observation {
                let observation_path = config
                    .state_dir
                    .join("observations")
                    .join(format!("{}.json", observation.observation_id));
                if !observation_path.exists() {
                    return Ok(Some(observation));
                }
            }
            continue;
        }
        let seed = u64::from_str_radix(&probe_id[..16], 16)
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_SEED:{error}"))?;
        let cycle = run_generative_cycle(&memory.generative, &candidate.input, seed)?;
        let behaviorally_verified =
            cycle.behavioral_composition_executed && validate_behavioral_execution_receipt(&cycle);
        let intrinsic_attempt_pending = state.intrinsic_drive.begin_attempt(
            &candidate.hypothesis,
            behaviorally_verified,
            cycle.frontier_advance_units,
            cycle.novel_verified_artifact_count,
        );
        let observation = if intrinsic_attempt_pending {
            let behavioral_receipt = cycle
                .behavioral_execution_receipt
                .as_ref()
                .filter(|receipt| receipt.executed)
                .ok_or_else(|| "PLATEAU_GENERATIVE_BEHAVIORAL_RECEIPT_MISSING".to_string())?;
            let behavioral_sha256 = cycle
                .behavioral_verification_sha256
                .as_ref()
                .filter(|hash| **hash == behavioral_receipt.receipt_sha256)
                .ok_or_else(|| "PLATEAU_GENERATIVE_BEHAVIORAL_BINDING_FAILURE".to_string())?;
            let frontier_before = memory.generative.distinct_verified_artifact_count();
            let frontier_after = frontier_before.saturating_add(cycle.frontier_advance_units);
            let artifact_sha256s = behavioral_receipt
                .verified_artifacts
                .iter()
                .map(|artifact| artifact.artifact_sha256.clone())
                .collect::<Vec<_>>();
            let content_sha256 = json_sha256(&(
                format!(
                    "B_CORE_PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}"
                ),
                &probe_id,
                &state.current_memory_sha256,
                &candidate.hypothesis,
                &candidate.input,
                seed,
                behavioral_sha256,
                &artifact_sha256s,
                intrinsic_attempt_pending,
            ))?;
            let observation_id =
                sha256(format!("INTRINSIC_CURIOSITY_OBSERVATION:{content_sha256}").as_bytes());
            let mut evidence = vec![behavioral_sha256.clone()];
            evidence.extend(artifact_sha256s);
            evidence.sort();
            evidence.dedup();
            let promotion_deltas =
                plateau_promotion_contract_deltas(&candidate.public_contract_deltas, &cycle)?;
            Some(LearningObservation {
                observation_id: observation_id.clone(),
                work_event_id: None,
                logical_path: format!("INTERNAL/.b_intrinsic_curiosity/{observation_id}"),
                content_sha256: content_sha256.clone(),
                predecessor_content_sha256: Some(state.current_memory_sha256.clone()),
                actor: WorkActor::LocalTool,
                work_kind: WorkKind::CapabilitySynthesis,
                work_outcome: WorkOutcome::Pass,
                features_before: None,
                features_after: StructuralFeatures::default(),
                signals: vec![
                    "AUTONOMOUS_INTRINSIC_CURIOSITY".to_string(),
                    "BEHAVIORALLY_VERIFIED_NOVEL_ARTIFACT".to_string(),
                    "INTRINSIC_REWARD_PENDING_PROMOTION".to_string(),
                    "VERIFIED_PASS".to_string(),
                ],
                composition_roles: vec![
                    "HYPOTHESIS_SELECTION".to_string(),
                    "CROSS_LESSON_PREDICTION".to_string(),
                    "PROGRAM_COMPOSITION".to_string(),
                    "BEHAVIORAL_FALSIFICATION".to_string(),
                    "REWARD_ELIGIBILITY_GATE".to_string(),
                ],
                learning_score: 95,
                learning_value: LearningValue::High,
                reasons: vec![
                    "bounded intrinsic curiosity selected an executable cross-lesson hypothesis without external work input"
                        .to_string(),
                    "reward remains pending until the independent campaign verifier accepts a non-duplicate generation"
                        .to_string(),
                ],
                verification_evidence_sha256: evidence,
                performance_metrics: vec![PerformanceMetricEvidence {
                    metric: "GENERATIVE_VERIFIED_ARTIFACT_COUNT".to_string(),
                    before: frontier_before,
                    after: frontier_after,
                    lower_is_better: false,
                    evidence_sha256: content_sha256,
                    executable_knowledge: None,
                }],
                public_contract_deltas: promotion_deltas,
                exact_source_fragments_stored: 0,
                raw_source_bytes_stored: 0,
                observed_at_ms: state.generation,
            })
        } else {
            None
        };
        let mut receipt = PlateauGenerativeProbeReceipt {
            schema: format!(
                "B_CORE_PLATEAU_GENERATIVE_PROBE_{PLATEAU_GENERATIVE_PROBE_CONTRACT_REVISION}"
            ),
            probe_id,
            predecessor_memory_sha256: state.current_memory_sha256.clone(),
            intrinsic_attempt_sequence: state.intrinsic_drive.hypotheses_attempted,
            hypothesis: candidate.hypothesis,
            input: candidate.input,
            public_contract_deltas: candidate.public_contract_deltas,
            seed,
            cycle,
            intrinsic_attempt_pending,
            observation: observation.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = plateau_probe_receipt_hash(&receipt)?;
        fs::create_dir_all(&probe_root)
            .map_err(|error| format!("PLATEAU_GENERATIVE_PROBE_DIR:{error}"))?;
        write_immutable_json(&receipt_path, &receipt)?;
        cleanup_recent_files(&probe_root, "", MAX_RETAINED_INTRINSIC_CURIOSITY_RECEIPTS)?;
        return Ok(observation);
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn generative_frontier_continuation_observation(
    campaign_id: &str,
    generation: u64,
    predecessor_memory_sha256: &str,
    resulting_memory_sha256: &str,
    frontier_before: u64,
    frontier_after: u64,
    generative_cycle: &GenerativeCycleResult,
    verifier_receipt_sha256: &str,
    successor_substrate_available: bool,
) -> Result<Option<LearningObservation>, String> {
    if !generative_cycle.frontier_advance
        || frontier_after <= frontier_before
        || !successor_substrate_available
    {
        return Ok(None);
    }
    let behavioral_receipt = generative_cycle
        .behavioral_execution_receipt
        .as_ref()
        .filter(|receipt| receipt.executed)
        .ok_or_else(|| "GENERATIVE_CONTINUATION_BEHAVIORAL_RECEIPT_MISSING".to_string())?;
    let behavioral_verification_sha256 = generative_cycle
        .behavioral_verification_sha256
        .as_ref()
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "GENERATIVE_CONTINUATION_VERIFICATION_HASH_MISSING".to_string())?;
    if behavioral_receipt.receipt_sha256 != *behavioral_verification_sha256
        || generative_cycle.frontier_advance_units != frontier_after - frontier_before
        || verifier_receipt_sha256.len() != 64
        || resulting_memory_sha256.len() != 64
    {
        return Err("GENERATIVE_CONTINUATION_BINDING_FAILURE".to_string());
    }
    let artifact_sha256 = behavioral_receipt
        .verified_artifacts
        .iter()
        .map(|artifact| artifact.artifact_sha256.clone())
        .collect::<Vec<_>>();
    let content_sha256 = json_sha256(&(
        "B_CORE_GENERATIVE_FRONTIER_CONTINUATION_1",
        campaign_id,
        generation,
        predecessor_memory_sha256,
        resulting_memory_sha256,
        frontier_before,
        frontier_after,
        behavioral_verification_sha256,
        verifier_receipt_sha256,
        &artifact_sha256,
    ))?;
    let observation_id =
        sha256(format!("GENERATIVE_FRONTIER_CONTINUATION:{content_sha256}").as_bytes());
    let mut evidence = vec![
        behavioral_verification_sha256.clone(),
        verifier_receipt_sha256.to_string(),
        resulting_memory_sha256.to_string(),
    ];
    evidence.extend(artifact_sha256);
    evidence.sort();
    evidence.dedup();
    Ok(Some(LearningObservation {
        observation_id: observation_id.clone(),
        work_event_id: None,
        logical_path: format!("INTERNAL/.b_generative_frontier/{observation_id}"),
        content_sha256: content_sha256.clone(),
        predecessor_content_sha256: Some(predecessor_memory_sha256.to_string()),
        actor: WorkActor::LocalTool,
        work_kind: WorkKind::CapabilitySynthesis,
        work_outcome: WorkOutcome::Pass,
        features_before: None,
        features_after: StructuralFeatures::default(),
        signals: vec![
            "BEHAVIORAL_FRONTIER_ADVANCE".to_string(),
            "GENERATIVE_FRONTIER_CONTINUATION".to_string(),
            format!("GENERATIVE_FRONTIER_RANGE:{frontier_before}:{frontier_after}"),
            "VERIFIED_PASS".to_string(),
        ],
        composition_roles: vec![
            "PROGRAM_COMPOSITION".to_string(),
            "INVARIANT_CHECK".to_string(),
            "REGRESSION_TEST".to_string(),
        ],
        learning_score: 95,
        learning_value: LearningValue::High,
        reasons: vec![
            "the independent verifier accepted a larger behaviorally executed artifact frontier"
                .to_string(),
            "only a strict frontier increase may seed one bounded successor composition cycle"
                .to_string(),
        ],
        verification_evidence_sha256: evidence,
        performance_metrics: vec![PerformanceMetricEvidence {
            metric: "GENERATIVE_VERIFIED_ARTIFACT_COUNT".to_string(),
            before: frontier_before,
            after: frontier_after,
            lower_is_better: false,
            evidence_sha256: content_sha256,
            executable_knowledge: None,
        }],
        public_contract_deltas: Vec::new(),
        exact_source_fragments_stored: 0,
        raw_source_bytes_stored: 0,
        // Generation is part of the immutable campaign freeze and therefore
        // remains identical when promotion is replayed after a crash.
        observed_at_ms: generation,
    }))
}

fn promote_candidate(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
    freeze: &CampaignFreeze,
    candidate: &LearningCandidate,
    receipt: &GrowthVerificationReceipt,
) -> Result<String, String> {
    if receipt.decision != GrowthDecision::Accept {
        return Err("REJECTED_CANDIDATE_CANNOT_PROMOTE".to_string());
    }
    validate_receipt(freeze, candidate, receipt)?;
    if !lesson_has_growth_subject(&candidate.lesson) {
        return Err("VERIFICATION_ONLY_CANDIDATE_CANNOT_PROMOTE".to_string());
    }
    if !lesson_has_executable_knowledge(&candidate.lesson) {
        return Err("TEXT_ONLY_CANDIDATE_CANNOT_PROMOTE".to_string());
    }
    let mut memory = load_memory(config, state.generation)?;
    if json_sha256(&memory)? != freeze.predecessor_memory_sha256
        || candidate.predecessor_memory_sha256 != freeze.predecessor_memory_sha256
    {
        return Err("PREDECESSOR_MEMORY_MISMATCH".to_string());
    }
    if memory_contains_semantic_lesson(&memory, &candidate.lesson)? {
        return Err("DUPLICATE_SEMANTIC_LESSON_CANNOT_PROMOTE".to_string());
    }
    let generative_frontier_before = memory.generative.distinct_verified_artifact_count();
    let generative_input = generative_input(&candidate.lesson);
    let expected_generative_cycle =
        run_generative_cycle(&memory.generative, &generative_input, freeze.seed)?;
    if candidate.generative_cycle != expected_generative_cycle {
        return Err("GENERATIVE_CYCLE_PROMOTION_BINDING_FAILURE".to_string());
    }
    let next_evaluator =
        derive_next_evaluator_memory(&memory.evaluator, &memory.lessons, &candidate.lesson)?;
    if !receipt.evaluator_self_audit.pass
        || !receipt.evaluator_self_audit.post_challenge_core_revalidated
        || receipt.evaluator_self_audit.active_evaluator_generation != memory.evaluator.generation
        || receipt.evaluator_self_audit.proposed_evaluator_generation != next_evaluator.generation
        || receipt
            .evaluator_self_audit
            .proposed_evaluator_memory_sha256
            != json_sha256(&next_evaluator)?
    {
        return Err("EVALUATOR_PROMOTION_BINDING_FAILURE".to_string());
    }
    if !memory
        .lessons
        .iter()
        .any(|lesson| lesson.lesson_id == candidate.lesson.lesson_id)
    {
        memory.lessons.push(candidate.lesson.clone());
    }
    while memory.lessons.len() > config.resources.max_lessons {
        memory.lessons.remove(0);
    }
    let applied_policy_signals = if candidate.generative_cycle.applied_to_self_improvement {
        candidate.generative_cycle.applied_policy_signals.as_slice()
    } else {
        &[]
    };
    refine_classifier_from_capability_outcome(
        &mut memory.classifier,
        freeze.generation,
        &candidate.lesson,
        applied_policy_signals,
        candidate.generative_cycle.frontier_advance,
        candidate
            .generative_cycle
            .behavioral_verification_sha256
            .as_ref(),
    );
    memory.classifier.accepted_campaigns = memory.classifier.accepted_campaigns.saturating_add(1);
    memory.evaluator = next_evaluator;
    memory.generative = promote_generative_cycle(
        &memory.generative,
        &generative_input,
        &candidate.generative_cycle,
    )?;
    memory.predecessor_sha256 = Some(freeze.predecessor_memory_sha256.clone());
    memory.generation = freeze.generation;
    let memory_hash = json_sha256(&memory)?;
    let generative_continuation = generative_frontier_continuation_observation(
        &freeze.campaign_id,
        freeze.generation,
        &freeze.predecessor_memory_sha256,
        &memory_hash,
        generative_frontier_before,
        memory.generative.distinct_verified_artifact_count(),
        &candidate.generative_cycle,
        &receipt.receipt_sha256,
        executable_generative_substrate_available(&memory.generative),
    )?;
    let next_memory_path = memory_path(config, memory.generation);
    if next_memory_path.exists() {
        let existing: GrowthMemory = read_json(&next_memory_path)?;
        if json_sha256(&existing)? != memory_hash {
            return Err("EXISTING_GENERATION_DIVERGES_FROM_RECOVERY".to_string());
        }
    } else {
        write_immutable_json(&next_memory_path, &memory)?;
    }
    // A behaviorally verified typed ProgramIR is executable knowledge now,
    // not merely a candidate for another static Rust registry rebuild. Make
    // its name-independent recipe available to the next repair cycle before
    // committing the campaign transition. Replay is idempotent after a crash.
    reconcile_verified_generative_typed_operators(config, &memory)?;
    cleanup_memory_generations(config)?;
    if let Some(observation) = generative_continuation {
        persist_scan_observations(config, std::slice::from_ref(&observation))?;
    }
    for observation_id in &candidate.observation_ids {
        index
            .consumed_observation_ids
            .insert(observation_id.clone());
        let observation: LearningObservation = read_json(
            &campaign_dir(config, &freeze.campaign_id)
                .join(format!("observation_{observation_id}.json")),
        )?;
        if let Some(event_id) = observation.work_event_id {
            index.consumed_work_event_ids.insert(event_id);
        }
    }
    save_index(config, index)?;
    state.predecessor_memory_sha256 = Some(state.current_memory_sha256.clone());
    state.current_memory_sha256 = memory_hash.clone();
    state.generation = memory.generation;
    state.evaluator_generation = memory.evaluator.generation;
    state.current_evaluator_memory_sha256 = json_sha256(&memory.evaluator)?;
    state.evaluator_challenge_cases = receipt
        .evaluator_self_audit
        .knowledge_challenge_cases
        .min(u64::MAX as usize) as u64;
    state.mutual_revalidation_events = state.mutual_revalidation_events.saturating_add(1);
    state.generative_predictions = memory.generative.prediction_records;
    state.valuable_combinations_learned = memory.generative.distinct_verified_artifact_count();
    state.generative_memory_reuse_events = memory.generative.reuse_events;
    state.generative_self_application_events = memory.generative.self_application_events;
    state.generative_exploration_events = memory.generative.exploration_events;
    state.productive_generative_reuse_events = memory.generative.productive_reuse_events;
    state.generative_frontier_advance_events = memory.generative.frontier_advance_events;
    state.generative_frontier_capability_units = memory.generative.frontier_capability_units;
    state.unverified_generative_frontier_candidate_events =
        memory.generative.unverified_frontier_candidate_events;
    state.legacy_unverified_generative_frontier_advance_events =
        memory.generative.legacy_unverified_frontier_advance_events;
    state.legacy_wrapper_generative_frontier_advance_events =
        memory.generative.legacy_wrapper_frontier_advance_events;
    state.generative_behavioral_verification_events =
        memory.generative.behavioral_verification_events;
    state.redundant_generative_selection_events = memory.generative.redundant_selection_events;
    state.generative_prediction_absolute_error_total =
        memory.generative.prediction_absolute_error_total;
    state.generative_calibrated_prediction_records =
        memory.generative.calibrated_prediction_records;
    state.generative_legacy_uncalibrated_prediction_error_total =
        memory.generative.legacy_uncalibrated_prediction_error_total;
    let (distinct_semantic_lessons, semantic_duplicate_lessons) = semantic_lesson_counts(&memory)?;
    state.distinct_semantic_lessons = distinct_semantic_lessons;
    state.semantic_duplicate_lessons = semantic_duplicate_lessons;
    state.measured_performance_promotions = executable_performance_promotion_count(&memory);
    state.classifier_outcome_bound_refinements = memory.classifier.outcome_bound_refinements;
    state.classifier_unsupported_refinements_suppressed =
        memory.classifier.unsupported_refinements_suppressed;
    resolve_intrinsic_observation_outcomes(config, state, &candidate.observation_ids, true)?;
    state.diagnostic_policy.resolve_consumed_action_outcome(
        freeze.generation.saturating_sub(1),
        true,
        &candidate.observation_ids,
    );
    state.campaigns_accepted = state.campaigns_accepted.saturating_add(1);
    state.consecutive_failures = 0;
    state.plateau_scans = 0;
    state.pending_campaign_id = None;
    Ok(memory_hash)
}

fn consume_failed_observations(
    config: &GrowthSupervisorConfig,
    index: &mut FileIndex,
    freeze: &CampaignFreeze,
    candidate: &LearningCandidate,
) -> Result<(), String> {
    for observation_id in &candidate.observation_ids {
        index
            .consumed_observation_ids
            .insert(observation_id.clone());
        let observation: LearningObservation = read_json(
            &campaign_dir(config, &freeze.campaign_id)
                .join(format!("observation_{observation_id}.json")),
        )?;
        if let Some(event_id) = observation.work_event_id {
            index.consumed_work_event_ids.insert(event_id);
        }
    }
    save_index(config, index)
}

fn complete_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
    freeze: &CampaignFreeze,
    candidate: &LearningCandidate,
    receipt: &GrowthVerificationReceipt,
) -> Result<bool, String> {
    let directory = campaign_dir(config, &freeze.campaign_id);
    let accepted = receipt.decision == GrowthDecision::Accept;
    let resulting_memory_sha256 = if accepted {
        save_transition(
            config,
            state,
            SupervisorPhase::Promoting,
            "INDEPENDENT_VERIFIER_ACCEPTED_CANDIDATE",
        )?;
        Some(promote_candidate(
            config, state, index, freeze, candidate, receipt,
        )?)
    } else {
        consume_failed_observations(config, index, freeze, candidate)?;
        resolve_intrinsic_observation_outcomes(config, state, &candidate.observation_ids, false)?;
        state.diagnostic_policy.resolve_consumed_action_outcome(
            freeze.generation.saturating_sub(1),
            false,
            &candidate.observation_ids,
        );
        state.campaigns_failed = state.campaigns_failed.saturating_add(1);
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        state.pending_campaign_id = None;
        None
    };
    let candidate_path = directory.join("candidate.json");
    let failed_candidate_deleted = !accepted && candidate_path.exists();
    if failed_candidate_deleted {
        fs::remove_file(&candidate_path)
            .map_err(|error| format!("FAILED_CANDIDATE_DELETE:{error}"))?;
    }
    let history = CampaignHistory {
        campaign_id: freeze.campaign_id.clone(),
        generation_attempted: freeze.generation,
        accepted,
        predecessor_memory_sha256: freeze.predecessor_memory_sha256.clone(),
        resulting_memory_sha256,
        freeze_sha256: json_sha256(freeze)?,
        candidate_sha256: json_sha256(candidate)?,
        verification_receipt_sha256: receipt.receipt_sha256.clone(),
        rollback_reference: freeze.predecessor_memory_sha256.clone(),
        failed_candidate_deleted,
    };
    let history_path = config
        .state_dir
        .join("history")
        .join(format!("{}.json", freeze.campaign_id));
    if !history_path.exists() {
        write_immutable_json(&history_path, &history)?;
    }
    let next_phase = if state.consecutive_failures >= config.resources.max_consecutive_failures {
        state.stop_reason = Some("MAX_CONSECUTIVE_FAILURES_REACHED".to_string());
        SupervisorPhase::SafeStopped
    } else {
        SupervisorPhase::InfraReady
    };
    save_transition(
        config,
        state,
        next_phase,
        if accepted {
            "VERIFIED_GENERATION_PROMOTED"
        } else {
            "FAILED_GENERATION_DISCARDED_PREDECESSOR_PRESERVED"
        },
    )?;
    Ok(accepted)
}

fn execute_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
    freeze: CampaignFreeze,
) -> Result<bool, String> {
    let directory = campaign_dir(config, &freeze.campaign_id);
    let candidate_path = directory.join("candidate.json");
    let candidate = if candidate_path.exists() {
        read_json(&candidate_path)?
    } else {
        save_transition(
            config,
            state,
            SupervisorPhase::CampaignRunning,
            "DETERMINISTIC_LEARNING_CANDIDATE_CONSTRUCTION",
        )?;
        let candidate = build_candidate(config, &freeze)?;
        write_immutable_json(&candidate_path, &candidate)?;
        candidate
    };
    save_transition(
        config,
        state,
        SupervisorPhase::Verifying,
        "SEPARATE_LOCAL_VERIFIER_PROCESS_REQUIRED",
    )?;
    let receipt = run_independent_verifier(config, &freeze, &candidate)?;
    complete_campaign(config, state, index, &freeze, &candidate, &receipt)
}

fn recover_pending_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: &mut FileIndex,
) -> Result<Option<(String, bool)>, String> {
    let Some(campaign_id) = state.pending_campaign_id.clone() else {
        return Ok(None);
    };
    let history_path = config
        .state_dir
        .join("history")
        .join(format!("{campaign_id}.json"));
    if history_path.exists() {
        let history: CampaignHistory = read_json(&history_path)?;
        let recovered_freeze: CampaignFreeze =
            read_json(&campaign_dir(config, &campaign_id).join("freeze.json"))?;
        resolve_intrinsic_observation_outcomes(
            config,
            state,
            &recovered_freeze.observation_ids,
            history.accepted,
        )?;
        state.pending_campaign_id = None;
        if history.accepted {
            let expected_hash = history
                .resulting_memory_sha256
                .ok_or_else(|| "ACCEPTED_HISTORY_MEMORY_HASH_MISSING".to_string())?;
            let recovered_memory = load_memory(config, history.generation_attempted)?;
            if json_sha256(&recovered_memory)? != expected_hash {
                return Err("RECOVERED_MEMORY_HISTORY_MISMATCH".to_string());
            }
            state.predecessor_memory_sha256 = Some(history.predecessor_memory_sha256.clone());
            state.generation = history.generation_attempted;
            state.current_memory_sha256 = expected_hash;
            state.evaluator_generation = recovered_memory.evaluator.generation;
            state.current_evaluator_memory_sha256 = json_sha256(&recovered_memory.evaluator)?;
        }
        save_transition(
            config,
            state,
            SupervisorPhase::InfraReady,
            "RECOVERED_COMPLETED_CAMPAIGN_FROM_IMMUTABLE_HISTORY",
        )?;
        return Ok(Some((campaign_id, history.accepted)));
    }
    let freeze: CampaignFreeze =
        read_json(&campaign_dir(config, &campaign_id).join("freeze.json"))?;
    let accepted = execute_campaign(config, state, index, freeze)?;
    Ok(Some((campaign_id, accepted)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CampaignFailure {
    schema: String,
    campaign_id: String,
    #[serde(default)]
    error_class: String,
    error_sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    observation_ids: Vec<String>,
    #[serde(default)]
    observations_quarantined: bool,
    predecessor_preserved: bool,
    failed_candidate_deleted: bool,
    occurred_at_ms: u64,
}

fn campaign_error_class(error: &str) -> String {
    let class = error.split(':').next().unwrap_or_default().trim();
    if !class.is_empty()
        && class.len() <= 96
        && class
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        class.to_string()
    } else {
        "UNCLASSIFIED_CAMPAIGN_ERROR".to_string()
    }
}

fn quarantine_aborted_campaign_observations(
    config: &GrowthSupervisorConfig,
    index: &mut FileIndex,
    campaign_id: &str,
) -> Result<Vec<String>, String> {
    let freeze: CampaignFreeze = read_json(&campaign_dir(config, campaign_id).join("freeze.json"))?;
    let mut changed = false;
    for observation_id in &freeze.observation_ids {
        changed |= index
            .consumed_observation_ids
            .insert(observation_id.clone());
        let observation: LearningObservation = read_json(
            &campaign_dir(config, campaign_id).join(format!("observation_{observation_id}.json")),
        )?;
        if let Some(event_id) = observation.work_event_id {
            changed |= index.consumed_work_event_ids.insert(event_id);
        }
    }
    if changed {
        save_index(config, index)?;
    }
    Ok(freeze.observation_ids)
}

fn abort_pending_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    index: Option<&mut FileIndex>,
    error: &str,
    quarantine_observations: bool,
) -> Result<Option<String>, String> {
    let Some(campaign_id) = state.pending_campaign_id.clone() else {
        return Ok(None);
    };
    let candidate_path = campaign_dir(config, &campaign_id).join("candidate.json");
    let failed_candidate_deleted = candidate_path.exists();
    if failed_candidate_deleted {
        fs::remove_file(&candidate_path)
            .map_err(|remove_error| format!("FAILED_CANDIDATE_DELETE:{remove_error}"))?;
    }
    let observation_ids = if quarantine_observations {
        quarantine_aborted_campaign_observations(
            config,
            index.ok_or_else(|| "FAILED_CAMPAIGN_INDEX_REQUIRED".to_string())?,
            &campaign_id,
        )?
    } else {
        read_json::<CampaignFreeze>(&campaign_dir(config, &campaign_id).join("freeze.json"))
            .map(|freeze| freeze.observation_ids)
            .unwrap_or_default()
    };
    resolve_intrinsic_observation_outcomes(config, state, &observation_ids, false)?;
    let failure = CampaignFailure {
        schema: SUPERVISOR_SCHEMA.to_string(),
        campaign_id: campaign_id.clone(),
        error_class: campaign_error_class(error),
        error_sha256: sha256(error.as_bytes()),
        observation_ids,
        observations_quarantined: quarantine_observations,
        predecessor_preserved: true,
        failed_candidate_deleted,
        occurred_at_ms: now_ms(),
    };
    let path = config
        .state_dir
        .join("history")
        .join(format!("{campaign_id}.failure.json"));
    if !path.exists() {
        write_immutable_json(&path, &failure)?;
    }
    state.campaigns_failed = state.campaigns_failed.saturating_add(1);
    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    state.pending_campaign_id = None;
    let phase = if state.consecutive_failures >= config.resources.max_consecutive_failures {
        state.stop_reason = Some("MAX_CONSECUTIVE_FAILURES_REACHED".to_string());
        SupervisorPhase::SafeStopped
    } else {
        SupervisorPhase::InfraReady
    };
    save_transition(
        config,
        state,
        phase,
        "CAMPAIGN_EXECUTION_FAILED_CANDIDATE_DISCARDED_PREDECESSOR_PRESERVED",
    )?;
    Ok(Some(campaign_id))
}

fn resource_stop_reason(
    config: &GrowthSupervisorConfig,
    state: &SupervisorState,
) -> Result<Option<String>, String> {
    let limits = &config.resources;
    let reason = if state.campaigns_started >= limits.max_lifetime_campaigns {
        Some("MAX_LIFETIME_CAMPAIGNS_REACHED")
    } else if state.generation >= limits.max_generations {
        Some("MAX_GENERATIONS_REACHED")
    } else if state.active_runtime_ms >= limits.max_active_runtime_ms {
        Some("MAX_ACTIVE_RUNTIME_REACHED")
    } else if state.observed_bytes >= limits.max_observed_bytes {
        Some("MAX_OBSERVED_BYTES_REACHED")
    } else if directory_bytes(&config.state_dir)? >= limits.max_state_bytes {
        Some("MAX_STATE_BYTES_REACHED")
    } else if state.consecutive_failures >= limits.max_consecutive_failures {
        Some("MAX_CONSECUTIVE_FAILURES_REACHED")
    } else {
        None
    };
    Ok(reason.map(str::to_string))
}

fn cohort_has_verification_evidence(observations: &[LearningObservation]) -> bool {
    !observations.is_empty()
        && build_lesson(observations)
            .map(|lesson| lesson_has_verification_evidence(&lesson))
            .unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CoreCohortValidationReceipt {
    schema: String,
    validation_id: String,
    originating_diagnostic_id: String,
    generation: u64,
    source_root_sha256: String,
    input_observation_ids: Vec<String>,
    source_fingerprint_before: String,
    source_fingerprint_after: String,
    workspace_stable_during_validation: bool,
    #[serde(default)]
    validation_scope: String,
    #[serde(default)]
    targeted_test_filter: Option<String>,
    #[serde(default)]
    full_regression_canary: bool,
    #[serde(default)]
    reused_validation_receipt_sha256: Option<String>,
    command: LocalCommandReceipt,
    success: bool,
    authoritative_source_write_events: u64,
    operator_selected: bool,
    codex_calls: u64,
    external_llm_calls: u64,
    network_reads: u64,
    network_writes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CoreValidationPlan {
    args: Vec<String>,
    validation_scope: String,
    targeted_test_filter: Option<String>,
    full_regression_canary: bool,
}

fn reusable_core_validation_receipt(
    config: &GrowthSupervisorConfig,
    source_fingerprint: &str,
    plan: &CoreValidationPlan,
) -> Result<Option<(CoreCohortValidationReceipt, String)>, String> {
    // Periodic and historically triggered full canaries must execute afresh.
    // Ordinary module-scoped validation may reuse an immutable successful
    // receipt when source state and the exact validation command are equal.
    if plan.full_regression_canary {
        return Ok(None);
    }
    let diagnostics = config.state_dir.join("diagnostics");
    let Ok(entries) = fs::read_dir(&diagnostics) else {
        return Ok(None);
    };
    let mut receipts = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("core_cohort_validation_")
                && entry.path().extension().and_then(OsStr::to_str) == Some("json")
        })
        .filter_map(|entry| {
            let modified = entry
                .metadata()
                .ok()?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    receipts.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in receipts {
        let Ok(receipt) = read_json::<CoreCohortValidationReceipt>(&path) else {
            continue;
        };
        if receipt.success
            && receipt.workspace_stable_during_validation
            && receipt.source_fingerprint_before == source_fingerprint
            && receipt.source_fingerprint_after == source_fingerprint
            && receipt.validation_scope == plan.validation_scope
            && receipt.targeted_test_filter == plan.targeted_test_filter
            && !receipt.full_regression_canary
            && receipt.reused_validation_receipt_sha256.is_none()
            && receipt.command.args == plan.args
        {
            let receipt_sha256 = json_sha256(&receipt)?;
            return Ok(Some((receipt, receipt_sha256)));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepositoryValidationPlan {
    validator_kind: RepositoryValidatorKind,
    test_selection_source: String,
    reused_validation_receipt_sha256: Option<String>,
    root_index: usize,
    root: PathBuf,
    input_observation_ids: Vec<String>,
    public_contract_target_symbols: Vec<String>,
    scope_paths: Vec<PathBuf>,
    test_paths: Vec<PathBuf>,
    program: PathBuf,
    args: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum RepositoryValidatorKind {
    #[default]
    PythonPytest,
    RustCargo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepositoryCohortValidationReceipt {
    schema: String,
    validation_id: String,
    originating_diagnostic_id: String,
    generation: u64,
    root_index: usize,
    root_sha256: String,
    #[serde(default)]
    validator_kind: RepositoryValidatorKind,
    #[serde(default)]
    test_selection_source: String,
    #[serde(default)]
    reused_validation_receipt_sha256: Option<String>,
    input_observation_ids: Vec<String>,
    test_paths: Vec<PathBuf>,
    scope_fingerprint_before: String,
    scope_fingerprint_after: String,
    scope_stable_during_validation: bool,
    program_sha256: String,
    command: LocalCommandReceipt,
    success: bool,
    authoritative_source_write_events: u64,
    operator_selected: bool,
    codex_calls: u64,
    external_llm_calls: u64,
    network_reads: u64,
    network_writes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepositoryRepairSynthesisReceipt {
    schema: String,
    repair_id: String,
    #[serde(default)]
    repair_problem_id: String,
    #[serde(default)]
    synthesis_capability_sha256: String,
    #[serde(default)]
    source_repair_engine_revision: u64,
    originating_validation_id: String,
    originating_diagnostic_id: String,
    generation: u64,
    root_index: usize,
    root_sha256: String,
    source_relative_path: PathBuf,
    predecessor_sha256: String,
    candidate_sha256: Option<String>,
    source_bound_receipt_sha256: Option<String>,
    source_bound_alternative_sha256: Vec<String>,
    #[serde(default)]
    source_bound_patch_variant_ids_attempted: Vec<String>,
    #[serde(default)]
    source_bound_patch_variant_sha256s_attempted: Vec<String>,
    #[serde(default)]
    prior_counterexample_candidate_sha256s: Vec<String>,
    #[serde(default)]
    selected_source_bound_patch_variant_id: Option<String>,
    #[serde(default)]
    selected_source_bound_template_symbols: Vec<String>,
    operator_family: String,
    edit_atom_kinds: Vec<String>,
    #[serde(default)]
    available_improvement_operator_ids: Vec<String>,
    #[serde(default)]
    attempted_improvement_operator_ids: Vec<String>,
    #[serde(default)]
    rejected_improvement_operator_ids: Vec<String>,
    #[serde(default)]
    selected_improvement_operator_ids: Vec<String>,
    #[serde(default)]
    promoted_improvement_operator_ids: Vec<String>,
    #[serde(default)]
    improvement_operators: Vec<TypedMechanismImprovementOperatorIR>,
    #[serde(default)]
    typed_candidates_enumerated: usize,
    candidate_materialization_is_one_to_one: bool,
    failure_code: Option<String>,
    scope_fingerprint_before: String,
    scope_fingerprint_after: String,
    authoritative_scope_stable: bool,
    sandbox_command: Option<LocalCommandReceipt>,
    #[serde(default)]
    authoritative_command: Option<LocalCommandReceipt>,
    sandbox_verified: bool,
    sandbox_cleaned: bool,
    #[serde(default)]
    rolled_back: bool,
    candidate_installed: bool,
    authoritative_source_write_events: u64,
    operator_selected: bool,
    codex_calls: u64,
    external_llm_calls: u64,
    network_reads: u64,
    network_writes: u64,
    exact_source_fragments_stored: u64,
    raw_source_bytes_stored: u64,
}

fn repository_repair_problem_id(
    validation_id: &str,
    relative: &Path,
    predecessor_sha256: &str,
) -> String {
    sha256(
        format!(
            "REPOSITORY_SOURCE_BOUND_REPAIR_PROBLEM_1:{validation_id}:{}:{predecessor_sha256}",
            relative.to_string_lossy().replace('\\', "/")
        )
        .as_bytes(),
    )
}

fn repository_repair_synthesis_capability_sha256(
    operators: &[TypedMechanismImprovementOperatorIR],
) -> Result<String, String> {
    let mut operator_bindings = operators
        .iter()
        .map(|operator| Ok((operator.operator_id.clone(), json_sha256(operator)?)))
        .collect::<Result<Vec<_>, String>>()?;
    operator_bindings.sort();
    json_sha256(&(SOURCE_REPAIR_ENGINE_REVISION, operator_bindings))
}

fn repository_repair_attempt_id(problem_id: &str, capability_sha256: &str) -> String {
    sha256(
        format!(
            "REPOSITORY_SOURCE_BOUND_REPAIR_ATTEMPT_3:{problem_id}:{}:{capability_sha256}",
            SOURCE_REPAIR_ENGINE_REVISION
        )
        .as_bytes(),
    )
}

fn repository_repair_succeeded(
    receipt: &RepositoryRepairSynthesisReceipt,
    mutation_enabled: bool,
) -> bool {
    receipt.sandbox_verified
        && receipt.sandbox_cleaned
        && receipt.authoritative_scope_stable
        && !receipt.rolled_back
        && (receipt.candidate_installed || !mutation_enabled)
}

fn repository_repair_verifier_falsified(receipt: &RepositoryRepairSynthesisReceipt) -> bool {
    receipt.sandbox_cleaned
        && receipt.authoritative_scope_stable
        && (receipt
            .sandbox_command
            .as_ref()
            .is_some_and(|command| !command.success)
            || receipt
                .authoritative_command
                .as_ref()
                .is_some_and(|command| !command.success))
}

fn repository_repair_counterexample_candidate_sha256s(
    history: &[RepositoryRepairSynthesisReceipt],
) -> BTreeSet<String> {
    history
        .iter()
        .filter(|receipt| repository_repair_verifier_falsified(receipt))
        .flat_map(|receipt| {
            receipt
                .source_bound_patch_variant_sha256s_attempted
                .iter()
                .cloned()
                .chain(receipt.candidate_sha256.iter().cloned())
        })
        .collect()
}

fn repository_repair_history(
    diagnostics: &Path,
    validation_id: &str,
    relative: &Path,
    predecessor_sha256: &str,
) -> Result<Vec<RepositoryRepairSynthesisReceipt>, String> {
    let Ok(entries) = fs::read_dir(diagnostics) else {
        return Ok(Vec::new());
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                && path.extension().and_then(OsStr::to_str) == Some("json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut history = Vec::new();
    for path in paths {
        let Ok(receipt) = read_json::<RepositoryRepairSynthesisReceipt>(&path) else {
            continue;
        };
        if matches!(
            receipt.schema.as_str(),
            "B_REPOSITORY_REPAIR_SYNTHESIS_2" | REPOSITORY_REPAIR_SYNTHESIS_SCHEMA
        ) && receipt.originating_validation_id == validation_id
            && receipt.source_relative_path == relative
            && receipt.predecessor_sha256 == predecessor_sha256
        {
            history.push(receipt);
        }
    }
    history.sort_by(|left, right| {
        left.generation
            .cmp(&right.generation)
            .then_with(|| left.repair_id.cmp(&right.repair_id))
    });
    Ok(history)
}

type SourceBoundImprovementOperatorAuthorityReceipt = TypedMechanismOperatorAuthorityReceiptIR;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepositoryCohortValidationOutcome {
    executed: bool,
    sandbox_repair_verified: bool,
    repository_repair_installed: bool,
    evidence_sha256: Vec<String>,
    output_observation_ids: Vec<String>,
}

fn logical_root_relative(logical_path: &str) -> Option<(usize, PathBuf)> {
    let (root, relative) = logical_path.split_once('/')?;
    let root_index = root.strip_prefix("ROOT_")?.parse::<usize>().ok()?;
    let relative = PathBuf::from(relative);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some((root_index, relative))
}

fn resolve_local_program(name: &str) -> Result<PathBuf, String> {
    let requested = Path::new(name);
    if requested.is_absolute() && requested.is_file() {
        return fs::canonicalize(requested)
            .map_err(|error| format!("REPOSITORY_VALIDATOR_PROGRAM_CANONICALIZE:{error}"));
    }
    let path =
        env::var_os("PATH").ok_or_else(|| "REPOSITORY_VALIDATOR_PATH_MISSING".to_string())?;
    let suffixes: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    for directory in env::split_paths(&path) {
        for suffix in suffixes {
            let candidate = directory.join(format!("{name}{suffix}"));
            if candidate.is_file() {
                return fs::canonicalize(&candidate)
                    .map_err(|error| format!("REPOSITORY_VALIDATOR_PROGRAM_CANONICALIZE:{error}"));
            }
        }
    }
    Err(format!("REPOSITORY_VALIDATOR_PROGRAM_NOT_FOUND:{name}"))
}

fn validated_repository_file(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("REPOSITORY_VALIDATOR_ROOT_CANONICALIZE:{error}"))?;
    let joined = root.join(relative);
    if fs::symlink_metadata(&joined)
        .map_err(|error| format!("REPOSITORY_VALIDATOR_FILE_METADATA:{error}"))?
        .file_type()
        .is_symlink()
    {
        return Err("REPOSITORY_VALIDATOR_SYMLINK_FORBIDDEN".to_string());
    }
    let canonical = fs::canonicalize(&joined)
        .map_err(|error| format!("REPOSITORY_VALIDATOR_FILE_CANONICALIZE:{error}"))?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        return Err("REPOSITORY_VALIDATOR_FILE_OUTSIDE_ROOT".to_string());
    }
    Ok(canonical)
}

fn repository_validation_scope_fingerprint(
    root: &Path,
    relative_paths: &[PathBuf],
    max_file_bytes: u64,
) -> Result<String, String> {
    let mut entries = Vec::with_capacity(relative_paths.len());
    for relative in relative_paths {
        let path = validated_repository_file(root, relative)?;
        let metadata = fs::metadata(&path)
            .map_err(|error| format!("REPOSITORY_VALIDATOR_FILE_METADATA:{error}"))?;
        if metadata.len() > max_file_bytes {
            return Err("REPOSITORY_VALIDATOR_SCOPE_FILE_TOO_LARGE".to_string());
        }
        let bytes =
            fs::read(&path).map_err(|error| format!("REPOSITORY_VALIDATOR_SCOPE_READ:{error}"))?;
        entries.push(format!(
            "{}:{}:{}",
            relative.to_string_lossy().replace('\\', "/"),
            bytes.len(),
            sha256(&bytes)
        ));
    }
    entries.sort();
    Ok(sha256(entries.join("\n").as_bytes()))
}

fn source_edit_atom_kinds(edit: &SourceEditAtom, kinds: &mut BTreeSet<String>) {
    match edit {
        SourceEditAtom::Replace { .. } => {
            kinds.insert("REPLACE".to_string());
        }
        SourceEditAtom::Insert { .. } => {
            kinds.insert("INSERT".to_string());
        }
        SourceEditAtom::Delete { .. } => {
            kinds.insert("DELETE".to_string());
        }
        SourceEditAtom::Move { .. } => {
            kinds.insert("MOVE".to_string());
        }
        SourceEditAtom::AtomicMultiEdit { edits } => {
            kinds.insert("ATOMIC_MULTI_EDIT".to_string());
            for nested in edits {
                source_edit_atom_kinds(nested, kinds);
            }
        }
    }
}

#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)]
fn ensure_repository_repair_file_writable(path: &Path) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("REPOSITORY_REPAIR_FILE_METADATA:{error}"))?
        .permissions();
    if permissions.readonly() {
        // On Windows this clears the FILE_ATTRIBUTE_READONLY bit; it does not
        // broaden an ACL or make the file world-writable.
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)
            .map_err(|error| format!("REPOSITORY_REPAIR_FILE_PERMISSIONS:{error}"))?;
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_repository_repair_file_writable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("REPOSITORY_REPAIR_FILE_METADATA:{error}"))?
        .permissions();
    let owner_writable = permissions.mode() | 0o200;
    permissions.set_mode(owner_writable);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("REPOSITORY_REPAIR_FILE_PERMISSIONS:{error}"))
}

fn copy_repository_to_repair_sandbox(
    config: &GrowthSupervisorConfig,
    root: &Path,
    sandbox: &Path,
) -> Result<(), String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("REPOSITORY_REPAIR_ROOT_CANONICALIZE:{error}"))?;
    let files = collect_files(
        std::slice::from_ref(&canonical_root),
        &config.observation,
        config
            .resources
            .max_files_per_scan
            .min(MAX_REPOSITORY_REPAIR_SANDBOX_FILES),
    )?;
    let byte_budget = config
        .resources
        .max_bytes_per_scan
        .min(MAX_REPOSITORY_REPAIR_SANDBOX_BYTES);
    let mut copied_bytes = 0_u64;
    fs::create_dir_all(sandbox)
        .map_err(|error| format!("REPOSITORY_REPAIR_SANDBOX_CREATE:{error}"))?;
    for source in files {
        let metadata = fs::metadata(&source)
            .map_err(|error| format!("REPOSITORY_REPAIR_SOURCE_METADATA:{error}"))?;
        if metadata.len() > config.resources.max_file_bytes {
            return Err("PUBLIC_INFORMATION_INSUFFICIENT:SANDBOX_FILE_TOO_LARGE".to_string());
        }
        copied_bytes = copied_bytes.saturating_add(metadata.len());
        if copied_bytes > byte_budget {
            return Err("PUBLIC_INFORMATION_INSUFFICIENT:SANDBOX_BYTE_BUDGET".to_string());
        }
        let relative = source
            .strip_prefix(&canonical_root)
            .map_err(|_| "REPOSITORY_REPAIR_SANDBOX_PREFIX".to_string())?;
        let destination = sandbox.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("REPOSITORY_REPAIR_SANDBOX_PARENT:{error}"))?;
        }
        fs::copy(&source, &destination)
            .map_err(|error| format!("REPOSITORY_REPAIR_SANDBOX_COPY:{error}"))?;
        ensure_repository_repair_file_writable(&destination)?;
    }
    Ok(())
}

fn remove_repository_repair_sandbox(
    config: &GrowthSupervisorConfig,
    sandbox: &Path,
) -> Result<(), String> {
    let parent = config.state_dir.join("repository_repair_sandboxes");
    if sandbox.parent() != Some(parent.as_path()) || sandbox == parent {
        return Err("REPOSITORY_REPAIR_SANDBOX_DELETE_SCOPE".to_string());
    }
    if sandbox.exists() {
        fs::remove_dir_all(sandbox)
            .map_err(|error| format!("REPOSITORY_REPAIR_SANDBOX_DELETE:{error}"))?;
    }
    Ok(())
}

#[derive(Debug)]
struct RepositoryInstallOutcome {
    installed: bool,
    rolled_back: bool,
    authoritative_source_write_events: u64,
    command: Option<LocalCommandReceipt>,
    scope_fingerprint_after: String,
    authoritative_scope_stable: bool,
    failure_code: Option<String>,
}

struct RepositoryInstallRequest<'a> {
    plan: &'a RepositoryValidationPlan,
    validation: &'a RepositoryCohortValidationReceipt,
    repair_id: &'a str,
    relative: &'a Path,
    predecessor_sha256: &'a str,
    candidate_source: &'a str,
    generation: u64,
    pending_improvement_operators: &'a [TypedMechanismImprovementOperatorIR],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepositoryInstallTransaction {
    schema: String,
    repair_id: String,
    generation: u64,
    root_index: usize,
    source_relative_path: PathBuf,
    predecessor_sha256: String,
    candidate_sha256: String,
    scope_fingerprint_before: String,
    scope_paths: Vec<PathBuf>,
    candidate_file_name: String,
    rollback_file_name: String,
    predecessor_readonly: bool,
    predecessor_unix_mode: Option<u32>,
    pending_improvement_operators: Vec<TypedMechanismImprovementOperatorIR>,
    operator_selected: bool,
    codex_calls: u64,
    external_llm_calls: u64,
    network_reads: u64,
    network_writes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepositoryInstallCommitReceipt {
    schema: String,
    repair_id: String,
    transaction_sha256: String,
    root_index: usize,
    source_relative_path: PathBuf,
    predecessor_sha256: String,
    candidate_sha256: String,
    scope_fingerprint_after: String,
    authoritative_command_sha256: String,
    authoritative_output_sha256: String,
    authoritative_source_write_events: u64,
    operator_selected: bool,
    codex_calls: u64,
    external_llm_calls: u64,
    network_reads: u64,
    network_writes: u64,
}

fn repository_install_transaction_directory(config: &GrowthSupervisorConfig) -> PathBuf {
    config
        .state_dir
        .join("control")
        .join("repository_install_transactions")
}

fn repository_install_transaction_path(
    config: &GrowthSupervisorConfig,
    repair_id: &str,
) -> PathBuf {
    repository_install_transaction_directory(config).join(format!("{repair_id}.json"))
}

fn repository_install_commit_path(config: &GrowthSupervisorConfig, repair_id: &str) -> PathBuf {
    config
        .state_dir
        .join("diagnostics")
        .join(format!("repository_install_commit_{repair_id}.json"))
}

fn validated_repository_target_slot(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err("REPOSITORY_INSTALL_TARGET_RELATIVE_PATH".to_string());
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("REPOSITORY_INSTALL_ROOT_CANONICALIZE:{error}"))?;
    let joined = canonical_root.join(relative);
    let parent = joined
        .parent()
        .ok_or_else(|| "REPOSITORY_INSTALL_TARGET_PARENT".to_string())?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("REPOSITORY_INSTALL_TARGET_PARENT_CANONICALIZE:{error}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("REPOSITORY_INSTALL_TARGET_OUTSIDE_ROOT".to_string());
    }
    let file_name = joined
        .file_name()
        .ok_or_else(|| "REPOSITORY_INSTALL_TARGET_NAME".to_string())?;
    let target = canonical_parent.join(file_name);
    if target.exists() {
        if fs::symlink_metadata(&target)
            .map_err(|error| format!("REPOSITORY_INSTALL_TARGET_METADATA:{error}"))?
            .file_type()
            .is_symlink()
        {
            return Err("REPOSITORY_INSTALL_TARGET_SYMLINK_FORBIDDEN".to_string());
        }
        let canonical_target = fs::canonicalize(&target)
            .map_err(|error| format!("REPOSITORY_INSTALL_TARGET_CANONICALIZE:{error}"))?;
        if !canonical_target.starts_with(&canonical_root) || !canonical_target.is_file() {
            return Err("REPOSITORY_INSTALL_TARGET_OUTSIDE_ROOT".to_string());
        }
    }
    Ok(target)
}

fn repository_install_sibling_paths(
    target: &Path,
    repair_id: &str,
) -> Result<(PathBuf, PathBuf), String> {
    if repair_id.len() != 64 || !repair_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("REPOSITORY_INSTALL_REPAIR_ID".to_string());
    }
    let file_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "REPOSITORY_INSTALL_TARGET_NAME".to_string())?;
    Ok((
        target.with_file_name(format!(
            ".{file_name}.b-core-{}.candidate",
            &repair_id[..16]
        )),
        target.with_file_name(format!(".{file_name}.b-core-{}.rollback", &repair_id[..16])),
    ))
}

#[cfg(unix)]
fn repository_permission_snapshot(permissions: &fs::Permissions) -> (bool, Option<u32>) {
    use std::os::unix::fs::PermissionsExt;
    (permissions.readonly(), Some(permissions.mode()))
}

#[cfg(not(unix))]
fn repository_permission_snapshot(permissions: &fs::Permissions) -> (bool, Option<u32>) {
    (permissions.readonly(), None)
}

#[cfg(unix)]
fn restore_repository_permissions(
    path: &Path,
    readonly: bool,
    unix_mode: Option<u32>,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = unix_mode.ok_or_else(|| "REPOSITORY_INSTALL_UNIX_MODE_MISSING".to_string())?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|error| format!("REPOSITORY_INSTALL_PERMISSIONS_RESTORE:{error}"))?;
    if fs::metadata(path)
        .map_err(|error| format!("REPOSITORY_INSTALL_PERMISSIONS_METADATA:{error}"))?
        .permissions()
        .readonly()
        != readonly
    {
        return Err("REPOSITORY_INSTALL_PERMISSIONS_DIVERGED".to_string());
    }
    Ok(())
}

#[cfg(not(unix))]
#[allow(clippy::permissions_set_readonly_false)]
fn restore_repository_permissions(
    path: &Path,
    readonly: bool,
    _unix_mode: Option<u32>,
) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("REPOSITORY_INSTALL_PERMISSIONS_METADATA:{error}"))?
        .permissions();
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("REPOSITORY_INSTALL_PERMISSIONS_RESTORE:{error}"))
}

fn remove_repository_install_artifact(
    path: &Path,
    expected_sha256: &str,
    max_file_bytes: u64,
    mismatch_code: &str,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("REPOSITORY_INSTALL_ARTIFACT_METADATA:{error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("REPOSITORY_INSTALL_ARTIFACT_TYPE_FORBIDDEN".to_string());
    }
    if file_sha256(path, max_file_bytes)? != expected_sha256 {
        return Err(mismatch_code.to_string());
    }
    fs::remove_file(path).map_err(|error| format!("REPOSITORY_INSTALL_ARTIFACT_REMOVE:{error}"))
}

fn validate_repository_install_transaction(
    config: &GrowthSupervisorConfig,
    transaction: &RepositoryInstallTransaction,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    if transaction.schema != REPOSITORY_INSTALL_TRANSACTION_SCHEMA
        || transaction.repair_id.len() != 64
        || !transaction
            .repair_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || transaction.predecessor_sha256.len() != 64
        || !transaction
            .predecessor_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || transaction.candidate_sha256.len() != 64
        || !transaction
            .candidate_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || transaction.scope_fingerprint_before.len() != 64
        || !transaction
            .scope_fingerprint_before
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || transaction.scope_paths.is_empty()
        || !transaction
            .scope_paths
            .contains(&transaction.source_relative_path)
        || transaction.operator_selected
        || transaction.codex_calls != 0
        || transaction.external_llm_calls != 0
        || transaction.network_reads != 0
        || transaction.network_writes != 0
        || cfg!(unix) != transaction.predecessor_unix_mode.is_some()
        || transaction.pending_improvement_operators.len()
            > MAX_ACTIVE_SOURCE_BOUND_IMPROVEMENT_OPERATORS
    {
        return Err("REPOSITORY_INSTALL_TRANSACTION_INVALID".to_string());
    }
    let mut operator_ids = BTreeSet::new();
    for operator in &transaction.pending_improvement_operators {
        validate_typed_mechanism_improvement_operator(operator)?;
        if !operator_ids.insert(operator.operator_id.clone()) {
            return Err("REPOSITORY_INSTALL_TRANSACTION_DUPLICATE_OPERATOR".to_string());
        }
    }
    let root = config
        .watched_roots
        .get(transaction.root_index)
        .ok_or_else(|| "REPOSITORY_INSTALL_TRANSACTION_ROOT".to_string())?;
    let target = validated_repository_target_slot(root, &transaction.source_relative_path)?;
    let (candidate, rollback) = repository_install_sibling_paths(&target, &transaction.repair_id)?;
    if candidate.file_name().and_then(OsStr::to_str)
        != Some(transaction.candidate_file_name.as_str())
        || rollback.file_name().and_then(OsStr::to_str)
            != Some(transaction.rollback_file_name.as_str())
    {
        return Err("REPOSITORY_INSTALL_TRANSACTION_ARTIFACT_MISMATCH".to_string());
    }
    Ok((target, candidate, rollback))
}

fn persist_committed_repository_operators(
    config: &GrowthSupervisorConfig,
    transaction: &RepositoryInstallTransaction,
    commit: &RepositoryInstallCommitReceipt,
) -> Result<(), String> {
    let commit_sha256 = json_sha256(commit)?;
    let operator_directory = typed_mechanism_operator_directory(&config.state_dir);
    let authority_directory = typed_mechanism_operator_authority_directory(&config.state_dir);
    fs::create_dir_all(&operator_directory)
        .map_err(|error| format!("COMMITTED_OPERATOR_REPOSITORY_CREATE:{error}"))?;
    fs::create_dir_all(&authority_directory)
        .map_err(|error| format!("COMMITTED_OPERATOR_AUTHORITY_CREATE:{error}"))?;
    for pending in &transaction.pending_improvement_operators {
        let mut operator = pending.clone();
        operator.evidence_sha256 = commit.authoritative_output_sha256.clone();
        validate_typed_mechanism_improvement_operator(&operator)?;
        let operator_path = operator_directory.join(format!("{}.json", operator.operator_id));
        if operator_path.exists() {
            let stored: TypedMechanismImprovementOperatorIR = read_json(&operator_path)?;
            validate_typed_mechanism_improvement_operator(&stored)?;
            let mut stored_identity = stored.clone();
            stored_identity.evidence_sha256.clear();
            let mut requested_identity = operator.clone();
            requested_identity.evidence_sha256.clear();
            if stored_identity != requested_identity {
                return Err("COMMITTED_OPERATOR_REPOSITORY_COLLISION".to_string());
            }
            if source_bound_operator_has_exact_authority(config, &stored)? {
                continue;
            }
            if stored.evidence_sha256 != operator.evidence_sha256 {
                return Err("COMMITTED_OPERATOR_UNAUTHORIZED_FIRST_EVIDENCE".to_string());
            }
            operator = stored;
        } else {
            write_immutable_json(&operator_path, &operator)?;
        }
        let operator_sha256 = json_sha256(&operator)?;
        let authority_id = sha256(
            format!(
                "INSTALLED_TYPED_OPERATOR_AUTHORITY_1:{}:{}:{}:{}",
                operator.operator_id,
                transaction.repair_id,
                commit_sha256,
                operator.evidence_sha256
            )
            .as_bytes(),
        );
        let mut authority = TypedMechanismOperatorAuthorityReceiptIR {
            schema: INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA.to_string(),
            authority_id: authority_id.clone(),
            operator_id: operator.operator_id.clone(),
            operator_sha256,
            repair_id: transaction.repair_id.clone(),
            repair_receipt_sha256: commit_sha256.clone(),
            sandbox_output_sha256: operator.evidence_sha256.clone(),
            candidate_sha256: transaction.candidate_sha256.clone(),
            sandbox_verified: true,
            sandbox_cleaned: true,
            authoritative_scope_stable: true,
            candidate_installed: true,
            authoritative_source_write_events: 1,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
            promotion_generation: transaction.generation,
            receipt_sha256: String::new(),
        };
        authority.receipt_sha256 = json_sha256(&authority)?;
        validate_typed_mechanism_operator_authority(&authority)?;
        let authority_path = authority_directory.join(format!("{authority_id}.json"));
        if authority_path.exists() {
            let stored: TypedMechanismOperatorAuthorityReceiptIR = read_json(&authority_path)?;
            if stored != authority {
                return Err("COMMITTED_OPERATOR_AUTHORITY_COLLISION".to_string());
            }
        } else {
            write_immutable_json(&authority_path, &authority)?;
        }
    }
    Ok(())
}

fn recover_repository_install_transaction(
    config: &GrowthSupervisorConfig,
    transaction_path: &Path,
    transaction: &RepositoryInstallTransaction,
) -> Result<(), String> {
    let (target, candidate, rollback) =
        validate_repository_install_transaction(config, transaction)?;
    let commit_path = repository_install_commit_path(config, &transaction.repair_id);
    if commit_path.exists() {
        let commit: RepositoryInstallCommitReceipt = read_json(&commit_path)?;
        if commit.schema != REPOSITORY_INSTALL_COMMIT_SCHEMA
            || commit.repair_id != transaction.repair_id
            || commit.transaction_sha256 != json_sha256(transaction)?
            || commit.root_index != transaction.root_index
            || commit.source_relative_path != transaction.source_relative_path
            || commit.predecessor_sha256 != transaction.predecessor_sha256
            || commit.candidate_sha256 != transaction.candidate_sha256
            || commit.scope_fingerprint_after.len() != 64
            || commit.authoritative_command_sha256.len() != 64
            || commit.authoritative_output_sha256.len() != 64
            || !commit
                .authoritative_output_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || commit.authoritative_source_write_events != 1
            || commit.operator_selected
            || commit.codex_calls != 0
            || commit.external_llm_calls != 0
            || commit.network_reads != 0
            || commit.network_writes != 0
        {
            return Err("REPOSITORY_INSTALL_COMMIT_INVALID".to_string());
        }
        if !target.exists()
            || file_sha256(&target, config.resources.max_file_bytes)?
                != transaction.candidate_sha256
        {
            return Err("REPOSITORY_INSTALL_COMMITTED_TARGET_DIVERGED".to_string());
        }
        let root = config
            .watched_roots
            .get(transaction.root_index)
            .ok_or_else(|| "REPOSITORY_INSTALL_TRANSACTION_ROOT".to_string())?;
        let committed_scope = repository_validation_scope_fingerprint(
            root,
            &transaction.scope_paths,
            config.resources.max_file_bytes,
        )?;
        if committed_scope != commit.scope_fingerprint_after {
            return Err("REPOSITORY_INSTALL_COMMITTED_SCOPE_DIVERGED".to_string());
        }
        persist_committed_repository_operators(config, transaction, &commit)?;
        remove_repository_install_artifact(
            &candidate,
            &transaction.candidate_sha256,
            config.resources.max_file_bytes,
            "REPOSITORY_INSTALL_COMMITTED_CANDIDATE_ARTIFACT_DIVERGED",
        )?;
        remove_repository_install_artifact(
            &rollback,
            &transaction.predecessor_sha256,
            config.resources.max_file_bytes,
            "REPOSITORY_INSTALL_COMMITTED_ROLLBACK_ARTIFACT_DIVERGED",
        )?;
        fs::remove_file(transaction_path)
            .map_err(|error| format!("REPOSITORY_INSTALL_TRANSACTION_FINALIZE:{error}"))?;
        fs::remove_file(&commit_path)
            .map_err(|error| format!("REPOSITORY_INSTALL_COMMIT_FINALIZE:{error}"))?;
        return Ok(());
    }

    if rollback.exists() {
        let rollback_metadata = fs::symlink_metadata(&rollback)
            .map_err(|error| format!("REPOSITORY_INSTALL_ROLLBACK_METADATA:{error}"))?;
        if rollback_metadata.file_type().is_symlink() || !rollback_metadata.is_file() {
            return Err("REPOSITORY_INSTALL_ROLLBACK_TYPE_FORBIDDEN".to_string());
        }
        if file_sha256(&rollback, config.resources.max_file_bytes)?
            != transaction.predecessor_sha256
        {
            return Err("REPOSITORY_INSTALL_RECOVERY_ROLLBACK_DIVERGED".to_string());
        }
        if target.exists() {
            let target_sha256 = file_sha256(&target, config.resources.max_file_bytes)?;
            if target_sha256 == transaction.candidate_sha256 {
                fs::remove_file(&target)
                    .map_err(|error| format!("REPOSITORY_INSTALL_RECOVERY_REMOVE:{error}"))?;
                fs::rename(&rollback, &target)
                    .map_err(|error| format!("REPOSITORY_INSTALL_RECOVERY_RESTORE:{error}"))?;
            } else if target_sha256 == transaction.predecessor_sha256 {
                fs::remove_file(&rollback).map_err(|error| {
                    format!("REPOSITORY_INSTALL_RECOVERY_DUPLICATE_ROLLBACK:{error}")
                })?;
            } else {
                return Err("REPOSITORY_INSTALL_RECOVERY_TARGET_DIVERGED".to_string());
            }
        } else {
            fs::rename(&rollback, &target)
                .map_err(|error| format!("REPOSITORY_INSTALL_RECOVERY_RESTORE:{error}"))?;
        }
    } else if !target.exists()
        || file_sha256(&target, config.resources.max_file_bytes)? != transaction.predecessor_sha256
    {
        return Err("REPOSITORY_INSTALL_RECOVERY_PREDECESSOR_UNAVAILABLE".to_string());
    }

    remove_repository_install_artifact(
        &candidate,
        &transaction.candidate_sha256,
        config.resources.max_file_bytes,
        "REPOSITORY_INSTALL_RECOVERY_CANDIDATE_DIVERGED",
    )?;
    if file_sha256(&target, config.resources.max_file_bytes)? != transaction.predecessor_sha256 {
        return Err("REPOSITORY_INSTALL_RECOVERY_PREDECESSOR_DIVERGED".to_string());
    }
    restore_repository_permissions(
        &target,
        transaction.predecessor_readonly,
        transaction.predecessor_unix_mode,
    )?;
    let root = config
        .watched_roots
        .get(transaction.root_index)
        .ok_or_else(|| "REPOSITORY_INSTALL_TRANSACTION_ROOT".to_string())?;
    let restored_scope = repository_validation_scope_fingerprint(
        root,
        &transaction.scope_paths,
        config.resources.max_file_bytes,
    )?;
    if restored_scope != transaction.scope_fingerprint_before {
        return Err("REPOSITORY_INSTALL_RECOVERY_SCOPE_DIVERGED".to_string());
    }
    fs::remove_file(transaction_path)
        .map_err(|error| format!("REPOSITORY_INSTALL_TRANSACTION_ROLLBACK_FINALIZE:{error}"))
}

fn recover_repository_install_transactions(
    config: &GrowthSupervisorConfig,
) -> Result<usize, String> {
    let directory = repository_install_transaction_directory(config);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Ok(0);
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(OsStr::to_str) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut recovered = 0_usize;
    for path in paths {
        let transaction: RepositoryInstallTransaction = read_json(&path)?;
        if path.file_stem().and_then(OsStr::to_str) != Some(transaction.repair_id.as_str()) {
            return Err("REPOSITORY_INSTALL_TRANSACTION_FILE_ID_MISMATCH".to_string());
        }
        recover_repository_install_transaction(config, &path, &transaction)?;
        recovered = recovered.saturating_add(1);
    }
    Ok(recovered)
}

fn write_repository_candidate_sibling(
    path: &Path,
    source: &str,
    permissions: fs::Permissions,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("REPOSITORY_INSTALL_CANDIDATE_CREATE:{error}"))?;
    let write_result = file
        .write_all(source.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("REPOSITORY_INSTALL_CANDIDATE_WRITE:{error}"));
    drop(file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    if let Err(error) = fs::set_permissions(path, permissions) {
        let _ = fs::remove_file(path);
        return Err(format!("REPOSITORY_INSTALL_CANDIDATE_PERMISSIONS:{error}"));
    }
    Ok(())
}

fn install_verified_repository_candidate(
    config: &GrowthSupervisorConfig,
    request: RepositoryInstallRequest<'_>,
) -> Result<RepositoryInstallOutcome, String> {
    let RepositoryInstallRequest {
        plan,
        validation,
        repair_id,
        relative,
        predecessor_sha256,
        candidate_source,
        generation,
        pending_improvement_operators,
    } = request;
    if !config.repository_mutation.enabled {
        return Ok(RepositoryInstallOutcome {
            installed: false,
            rolled_back: false,
            authoritative_source_write_events: 0,
            command: None,
            scope_fingerprint_after: validation.scope_fingerprint_after.clone(),
            authoritative_scope_stable: validation.scope_stable_during_validation,
            failure_code: None,
        });
    }
    if plan.validator_kind != RepositoryValidatorKind::PythonPytest
        || repair_id.len() != 64
        || sha256(candidate_source.as_bytes()) == predecessor_sha256
    {
        return Err("REPOSITORY_INSTALL_ENVELOPE".to_string());
    }
    let configured_root = config
        .watched_roots
        .get(plan.root_index)
        .ok_or_else(|| "REPOSITORY_INSTALL_ROOT_INDEX".to_string())?;
    if fs::canonicalize(configured_root)
        .map_err(|error| format!("REPOSITORY_INSTALL_CONFIG_ROOT:{error}"))?
        != fs::canonicalize(&plan.root)
            .map_err(|error| format!("REPOSITORY_INSTALL_PLAN_ROOT:{error}"))?
    {
        return Err("REPOSITORY_INSTALL_ROOT_AUTHORITY_MISMATCH".to_string());
    }
    let target = validated_repository_file(&plan.root, relative)?;
    let current = fs::read(&target)
        .map_err(|error| format!("REPOSITORY_INSTALL_PREDECESSOR_READ:{error}"))?;
    if sha256(&current) != predecessor_sha256 {
        return Err("REPOSITORY_INSTALL_PREDECESSOR_DIVERGED".to_string());
    }
    let scope_before = repository_validation_scope_fingerprint(
        &plan.root,
        &plan.scope_paths,
        config.resources.max_file_bytes,
    )?;
    if scope_before != validation.scope_fingerprint_before
        || validation.scope_fingerprint_before != validation.scope_fingerprint_after
        || !validation.scope_stable_during_validation
    {
        return Err("REPOSITORY_INSTALL_SCOPE_DIVERGED_BEFORE_WRITE".to_string());
    }
    let (candidate_sibling, rollback_sibling) =
        repository_install_sibling_paths(&target, repair_id)?;
    let transaction_path = repository_install_transaction_path(config, repair_id);
    if candidate_sibling.exists() || rollback_sibling.exists() {
        return Err("REPOSITORY_INSTALL_STALE_TRANSACTION".to_string());
    }
    if transaction_path.exists() {
        return Err("REPOSITORY_INSTALL_STALE_TRANSACTION_JOURNAL".to_string());
    }
    let permissions = fs::metadata(&target)
        .map_err(|error| format!("REPOSITORY_INSTALL_TARGET_METADATA:{error}"))?
        .permissions();
    let (predecessor_readonly, predecessor_unix_mode) =
        repository_permission_snapshot(&permissions);
    write_repository_candidate_sibling(&candidate_sibling, candidate_source, permissions)?;
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    if file_sha256(&candidate_sibling, config.resources.max_file_bytes)? != candidate_sha256 {
        let _ = fs::remove_file(&candidate_sibling);
        return Err("REPOSITORY_INSTALL_CANDIDATE_HASH_MISMATCH".to_string());
    }
    let transaction = RepositoryInstallTransaction {
        schema: REPOSITORY_INSTALL_TRANSACTION_SCHEMA.to_string(),
        repair_id: repair_id.to_string(),
        generation,
        root_index: plan.root_index,
        source_relative_path: relative.to_path_buf(),
        predecessor_sha256: predecessor_sha256.to_string(),
        candidate_sha256: candidate_sha256.clone(),
        scope_fingerprint_before: scope_before.clone(),
        scope_paths: plan.scope_paths.clone(),
        candidate_file_name: candidate_sibling
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| "REPOSITORY_INSTALL_CANDIDATE_NAME".to_string())?
            .to_string(),
        rollback_file_name: rollback_sibling
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| "REPOSITORY_INSTALL_ROLLBACK_NAME".to_string())?
            .to_string(),
        predecessor_readonly,
        predecessor_unix_mode,
        pending_improvement_operators: pending_improvement_operators.to_vec(),
        operator_selected: false,
        codex_calls: 0,
        external_llm_calls: 0,
        network_reads: 0,
        network_writes: 0,
    };
    if let Err(error) = write_immutable_json(&transaction_path, &transaction) {
        let _ = remove_repository_install_artifact(
            &candidate_sibling,
            &candidate_sha256,
            config.resources.max_file_bytes,
            "REPOSITORY_INSTALL_CANDIDATE_CLEANUP_DIVERGED",
        );
        return Err(error);
    }
    if let Err(error) = ensure_repository_repair_file_writable(&target) {
        let _ = remove_repository_install_artifact(
            &candidate_sibling,
            &candidate_sha256,
            config.resources.max_file_bytes,
            "REPOSITORY_INSTALL_CANDIDATE_CLEANUP_DIVERGED",
        );
        let _ = fs::remove_file(&transaction_path);
        return Err(error);
    }
    if let Err(error) = fs::rename(&target, &rollback_sibling) {
        let _ =
            restore_repository_permissions(&target, predecessor_readonly, predecessor_unix_mode);
        let _ = remove_repository_install_artifact(
            &candidate_sibling,
            &candidate_sha256,
            config.resources.max_file_bytes,
            "REPOSITORY_INSTALL_CANDIDATE_CLEANUP_DIVERGED",
        );
        let _ = fs::remove_file(&transaction_path);
        return Err(format!("REPOSITORY_INSTALL_ACTIVATE_ROLLBACK:{error}"));
    }
    let activate = fs::rename(&candidate_sibling, &target);
    if let Err(error) = activate {
        if fs::rename(&rollback_sibling, &target).is_ok() {
            let _ = restore_repository_permissions(
                &target,
                predecessor_readonly,
                predecessor_unix_mode,
            );
            let _ = fs::remove_file(&transaction_path);
        }
        return Err(format!("REPOSITORY_INSTALL_ACTIVATE_CANDIDATE:{error}"));
    }
    let log_path = config
        .state_dir
        .join("diagnostics")
        .join(format!("repository_install_{repair_id}.log"));
    let mut attempt_error = None;
    let installed_scope = match repository_validation_scope_fingerprint(
        &plan.root,
        &plan.scope_paths,
        config.resources.max_file_bytes,
    ) {
        Ok(scope) => Some(scope),
        Err(error) => {
            attempt_error = Some(error);
            None
        }
    };
    let command = if attempt_error.is_none() {
        let arg_refs = plan.args.iter().map(String::as_str).collect::<Vec<_>>();
        match command_receipt_with_incremental(
            &plan.program,
            &arg_refs,
            &plan.root,
            &runtime_validation_target_dir(config),
            MAX_CORE_COHORT_VALIDATION_MS,
            &log_path,
            true,
        ) {
            Ok(command) => Some(command),
            Err(error) => {
                attempt_error = Some(error);
                None
            }
        }
    } else {
        None
    };
    let _ = fs::remove_file(&log_path);
    let scope_after_attempt = match repository_validation_scope_fingerprint(
        &plan.root,
        &plan.scope_paths,
        config.resources.max_file_bytes,
    ) {
        Ok(scope) => Some(scope),
        Err(error) => {
            if attempt_error.is_none() {
                attempt_error = Some(error);
            }
            None
        }
    };
    let target_is_candidate = match file_sha256(&target, config.resources.max_file_bytes) {
        Ok(actual) => actual == candidate_sha256,
        Err(error) => {
            if attempt_error.is_none() {
                attempt_error = Some(error);
            }
            false
        }
    };
    let authoritative_scope_stable =
        installed_scope.is_some() && installed_scope == scope_after_attempt;
    let installed = command.as_ref().is_some_and(|receipt| receipt.success)
        && target_is_candidate
        && authoritative_scope_stable
        && attempt_error.is_none();
    if installed {
        let command_ref = command
            .as_ref()
            .ok_or_else(|| "REPOSITORY_INSTALL_COMMAND_MISSING".to_string())?;
        let scope_after =
            scope_after_attempt.ok_or_else(|| "REPOSITORY_INSTALL_SCOPE_MISSING".to_string())?;
        let commit = RepositoryInstallCommitReceipt {
            schema: REPOSITORY_INSTALL_COMMIT_SCHEMA.to_string(),
            repair_id: repair_id.to_string(),
            transaction_sha256: json_sha256(&transaction)?,
            root_index: plan.root_index,
            source_relative_path: relative.to_path_buf(),
            predecessor_sha256: predecessor_sha256.to_string(),
            candidate_sha256,
            scope_fingerprint_after: scope_after.clone(),
            authoritative_command_sha256: json_sha256(command_ref)?,
            authoritative_output_sha256: command_ref.output_sha256.clone(),
            authoritative_source_write_events: 1,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        if let Err(error) =
            write_immutable_json(&repository_install_commit_path(config, repair_id), &commit)
        {
            recover_repository_install_transaction(config, &transaction_path, &transaction)?;
            return Err(error);
        }
        recover_repository_install_transaction(config, &transaction_path, &transaction)?;
        return Ok(RepositoryInstallOutcome {
            installed: true,
            rolled_back: false,
            authoritative_source_write_events: 1,
            command,
            scope_fingerprint_after: scope_after,
            authoritative_scope_stable: true,
            failure_code: None,
        });
    }
    recover_repository_install_transaction(config, &transaction_path, &transaction)?;
    Ok(RepositoryInstallOutcome {
        installed: false,
        rolled_back: true,
        authoritative_source_write_events: 2,
        command,
        scope_fingerprint_after: scope_before,
        authoritative_scope_stable: true,
        failure_code: Some(if let Some(error) = attempt_error {
            format!(
                "PUBLIC_INFORMATION_INSUFFICIENT:AUTHORITATIVE_VALIDATION_ERROR:{}",
                sha256(error.as_bytes())
            )
        } else if !target_is_candidate || !authoritative_scope_stable {
            "CONFLICTING_SOURCE_BOUND_EDITS:AUTHORITATIVE_SCOPE_CHANGED".to_string()
        } else {
            "PUBLIC_INFORMATION_INSUFFICIENT:AUTHORITATIVE_VALIDATION_FAILED".to_string()
        }),
    })
}

fn repository_repair_observation_id(
    receipt: &RepositoryRepairSynthesisReceipt,
    receipt_sha256: &str,
) -> String {
    sha256(
        format!(
            "REPOSITORY_SANDBOX_REPAIR_OBSERVATION:{}:{}",
            receipt.repair_id, receipt_sha256
        )
        .as_bytes(),
    )
}

#[cfg(test)]
fn source_bound_improvement_operator_directory(config: &GrowthSupervisorConfig) -> PathBuf {
    typed_mechanism_operator_directory(&config.state_dir)
}

fn source_bound_improvement_operator_authority_directory(
    config: &GrowthSupervisorConfig,
) -> PathBuf {
    typed_mechanism_operator_authority_directory(&config.state_dir)
}

fn source_bound_operator_has_exact_authority(
    config: &GrowthSupervisorConfig,
    operator: &TypedMechanismImprovementOperatorIR,
) -> Result<bool, String> {
    validate_typed_mechanism_improvement_operator(operator)?;
    let operator_sha256 = json_sha256(operator)?;
    let directory = source_bound_improvement_operator_authority_directory(config);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Ok(false);
    };
    for entry in entries {
        let path = entry
            .map_err(|error| format!("SOURCE_BOUND_OPERATOR_AUTHORITY_ENTRY:{error}"))?
            .path();
        if path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }
        let authority: SourceBoundImprovementOperatorAuthorityReceipt = read_json(&path)?;
        validate_source_bound_operator_authority(&authority)?;
        if path.file_stem().and_then(OsStr::to_str) != Some(authority.authority_id.as_str()) {
            return Err("SOURCE_BOUND_OPERATOR_AUTHORITY_PATH_ID_MISMATCH".to_string());
        }
        if authority.operator_id == operator.operator_id
            && authority.operator_sha256 == operator_sha256
            && authority.sandbox_output_sha256 == operator.evidence_sha256
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_source_bound_operator_authority(
    authority: &SourceBoundImprovementOperatorAuthorityReceipt,
) -> Result<(), String> {
    validate_typed_mechanism_operator_authority(authority)
}

fn load_source_bound_improvement_operators(
    config: &GrowthSupervisorConfig,
) -> Result<Vec<TypedMechanismImprovementOperatorIR>, String> {
    load_authorized_typed_mechanism_operators(
        &config.state_dir,
        MAX_ACTIVE_SOURCE_BOUND_IMPROVEMENT_OPERATORS,
    )
}

fn persist_source_bound_improvement_operator(
    config: &GrowthSupervisorConfig,
    operator: &TypedMechanismImprovementOperatorIR,
    repair: &RepositoryRepairSynthesisReceipt,
    repair_receipt_sha256: &str,
) -> Result<(), String> {
    validate_typed_mechanism_improvement_operator(operator)?;
    let sandbox_only_authority = !repair.candidate_installed
        && repair.authoritative_source_write_events == 0
        && repair.authoritative_command.is_none();
    let installed_authority = repair.candidate_installed
        && repair.authoritative_source_write_events == 1
        && repair
            .authoritative_command
            .as_ref()
            .is_some_and(|command| command.success);
    if !repair.sandbox_verified
        || !repair.sandbox_cleaned
        || !repair.authoritative_scope_stable
        || repair.rolled_back
        || (!sandbox_only_authority && !installed_authority)
        || repair_receipt_sha256.len() != 64
    {
        return Err("SOURCE_BOUND_OPERATOR_PROMOTION_WITHOUT_AUTHORITY".to_string());
    }
    let execution_authority_output_sha256 = if installed_authority {
        repair
            .authoritative_command
            .as_ref()
            .filter(|command| command.success)
            .map(|command| command.output_sha256.clone())
            .ok_or_else(|| "SOURCE_BOUND_OPERATOR_AUTHORITATIVE_EVIDENCE_MISSING".to_string())?
    } else {
        repair
            .sandbox_command
            .as_ref()
            .filter(|command| command.success)
            .map(|command| command.output_sha256.clone())
            .ok_or_else(|| "SOURCE_BOUND_OPERATOR_SANDBOX_EVIDENCE_MISSING".to_string())?
    };
    if operator.evidence_sha256 != execution_authority_output_sha256 {
        return Err("SOURCE_BOUND_OPERATOR_EVIDENCE_MISMATCH".to_string());
    }
    persist_authorized_typed_mechanism_operator(
        &config.state_dir,
        operator,
        &TypedMechanismOperatorPromotionEvidenceIR {
            repair_id: repair.repair_id.clone(),
            repair_receipt_sha256: repair_receipt_sha256.to_string(),
            execution_output_sha256: execution_authority_output_sha256,
            candidate_sha256: repair
                .candidate_sha256
                .clone()
                .ok_or_else(|| "SOURCE_BOUND_OPERATOR_CANDIDATE_EVIDENCE_MISSING".to_string())?,
            sandbox_verified: repair.sandbox_verified,
            sandbox_cleaned: repair.sandbox_cleaned,
            authoritative_scope_stable: repair.authoritative_scope_stable,
            candidate_installed: repair.candidate_installed,
            authoritative_source_write_events: repair.authoritative_source_write_events,
            codex_calls: repair.codex_calls,
            external_llm_calls: repair.external_llm_calls,
            network_reads: repair.network_reads,
            network_writes: repair.network_writes,
            promotion_generation: repair.generation,
        },
    )?;
    Ok(())
}

fn python_pytest_target_symbols(diagnostic_tail: &str) -> Vec<String> {
    fn is_symbol_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
    }

    let mut symbols = BTreeSet::new();
    for line in diagnostic_tail.lines() {
        let assertion_line = line.contains("assert ") || line.contains("<function ");
        if !assertion_line {
            continue;
        }
        let bytes = line.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            if *byte != b'(' {
                continue;
            }
            let mut start = index;
            while start > 0 && is_symbol_byte(bytes[start - 1]) {
                start -= 1;
            }
            let symbol = &line[start..index];
            if !symbol.is_empty()
                && !symbol.starts_with("test_")
                && symbol
                    .split('.')
                    .all(|part| !part.is_empty() && !part.chars().next().unwrap().is_ascii_digit())
            {
                symbols.insert(symbol.to_string());
            }
        }
        if let Some(rest) = line.split_once("<function ").map(|(_, rest)| rest) {
            let symbol = rest
                .split(|character: char| character.is_whitespace() || character == '>')
                .next()
                .unwrap_or("");
            if !symbol.is_empty() && !symbol.starts_with("test_") {
                symbols.insert(symbol.to_string());
            }
        }
    }
    symbols.into_iter().collect()
}

fn repository_repair_target_symbols(
    plan: &RepositoryValidationPlan,
    diagnostic_tail: &str,
) -> Vec<String> {
    let mut targets = plan.public_contract_target_symbols.clone();
    for diagnostic_symbol in python_pytest_target_symbols(diagnostic_tail) {
        if !targets.contains(&diagnostic_symbol) {
            targets.push(diagnostic_symbol);
        }
    }
    targets.truncate(MAX_REPOSITORY_REPAIR_TARGET_SYMBOLS);
    targets
}

fn try_synthesize_failed_python_cohort(
    config: &GrowthSupervisorConfig,
    diagnostic: &AutonomousSelfInspectionReceipt,
    plan: &RepositoryValidationPlan,
    validation: &RepositoryCohortValidationReceipt,
) -> Result<Option<(String, String, bool)>, String> {
    if plan.validator_kind != RepositoryValidatorKind::PythonPytest || validation.success {
        return Ok(None);
    }
    let mut implementation_paths = plan
        .scope_paths
        .iter()
        .filter(|relative| {
            relative.extension().and_then(OsStr::to_str) == Some("py")
                && !path_is_dedicated_test(relative)
        })
        .cloned()
        .collect::<Vec<_>>();
    implementation_paths.sort();
    implementation_paths.dedup();
    implementation_paths.truncate(MAX_REPOSITORY_REPAIR_SOURCE_PATHS);

    let diagnostics = config.state_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics)
        .map_err(|error| format!("REPOSITORY_REPAIR_DIAGNOSTICS_CREATE:{error}"))?;
    let available_operators = load_source_bound_improvement_operators(config)?;
    let available_operator_ids = available_operators
        .iter()
        .map(|operator| operator.operator_id.clone())
        .collect::<BTreeSet<_>>();
    let synthesis_capability_sha256 =
        repository_repair_synthesis_capability_sha256(&available_operators)?;
    let mut authoritative_installation_attempts = 0_usize;
    for relative in implementation_paths {
        let source_path = validated_repository_file(&plan.root, &relative)?;
        let source = fs::read_to_string(&source_path)
            .map_err(|error| format!("REPOSITORY_REPAIR_SOURCE_READ:{error}"))?;
        let predecessor_sha256 = sha256(source.as_bytes());
        let repair_problem_id =
            repository_repair_problem_id(&validation.validation_id, &relative, &predecessor_sha256);
        let history = repository_repair_history(
            &diagnostics,
            &validation.validation_id,
            &relative,
            &predecessor_sha256,
        )?;
        if let Some(existing) = history.iter().rev().find(|receipt| {
            repository_repair_succeeded(receipt, config.repository_mutation.enabled)
        }) {
            let receipt_sha256 = json_sha256(existing)?;
            for operator in &existing.improvement_operators {
                persist_source_bound_improvement_operator(
                    config,
                    operator,
                    existing,
                    &receipt_sha256,
                )?;
            }
            let observation_id = repository_repair_observation_id(existing, &receipt_sha256);
            return Ok(Some((
                receipt_sha256,
                observation_id,
                existing.candidate_installed,
            )));
        }
        let prior_counterexample_candidate_sha256s =
            repository_repair_counterexample_candidate_sha256s(&history);
        let repair_id =
            repository_repair_attempt_id(&repair_problem_id, &synthesis_capability_sha256);
        let receipt_path =
            diagnostics.join(format!("repository_repair_synthesis_{repair_id}.json"));
        if receipt_path.exists() {
            let existing: RepositoryRepairSynthesisReceipt = read_json(&receipt_path)?;
            if existing.schema != REPOSITORY_REPAIR_SYNTHESIS_SCHEMA
                || existing.repair_id != repair_id
                || existing.repair_problem_id != repair_problem_id
                || existing.synthesis_capability_sha256 != synthesis_capability_sha256
                || existing.source_repair_engine_revision != SOURCE_REPAIR_ENGINE_REVISION
                || existing.originating_validation_id != validation.validation_id
                || existing.source_relative_path != relative
                || existing.predecessor_sha256 != predecessor_sha256
            {
                return Err("REPOSITORY_REPAIR_SYNTHESIS_RECEIPT_MISMATCH".to_string());
            }
            continue;
        }

        let mut candidate_sha256 = None;
        let mut source_bound_receipt_sha256 = None;
        let mut source_bound_alternative_sha256 = Vec::new();
        let mut source_bound_patch_variant_ids_attempted = Vec::new();
        let mut source_bound_patch_variant_sha256s_attempted = Vec::new();
        let mut selected_source_bound_patch_variant_id = None;
        let mut selected_source_bound_template_symbols = Vec::new();
        let mut edit_atom_kinds = BTreeSet::new();
        let mut materialization_is_one_to_one = false;
        let mut failure_code = None;
        let mut sandbox_command = None;
        let mut authoritative_command = None;
        let mut sandbox_verified = false;
        let mut sandbox_cleaned = true;
        let mut candidate_installed = false;
        let mut rolled_back = false;
        let mut authoritative_source_write_events = 0_u64;
        let mut selected_candidate_source = None;
        let mut candidate_variants = Vec::new();
        let mut selected_improvement_operator_ids = Vec::new();
        let mut attempted_improvement_operator_ids = Vec::new();
        let mut rejected_improvement_operator_ids = Vec::new();
        let mut typed_candidates_enumerated = 0_usize;
        let mut successful_syntheses = Vec::new();

        let request = SourceBoundRepositoryPathDiscoveryRequestIR {
            schema: SOURCE_BOUND_REPOSITORY_PATH_DISCOVERY_SCHEMA.to_string(),
            repository_root: plan.root.clone(),
            source_relative_path: relative.clone(),
            test_relative_paths: plan.test_paths.clone(),
            python_executable: plan.program.clone(),
            target_symbols: repository_repair_target_symbols(
                plan,
                &validation.command.diagnostic_tail,
            ),
            allowed_effects: Vec::new(),
            max_expression_depth: 3,
            max_candidates: 2_048,
        };
        match discover_and_synthesize_python_repository_paths_with_operators(
            &request,
            &available_operators,
        ) {
            Ok(source_bound) => {
                // Discovery returns only after exact Python AST/template
                // re-derivation succeeds. `source_bound` and `source` remain
                // immutable in this frame, so spawning the same frontend a
                // second time here adds latency without another authority
                // boundary or additional evidence.
                source_bound_receipt_sha256 = Some(json_sha256(&source_bound)?);
                for alternative in &source_bound.alternatives {
                    source_bound_alternative_sha256.push(json_sha256(alternative)?);
                    for synthesis in std::iter::once(&alternative.synthesis).chain(
                        alternative
                            .closure_candidates
                            .iter()
                            .map(|candidate| &candidate.synthesis),
                    ) {
                        typed_candidates_enumerated = typed_candidates_enumerated
                            .saturating_add(synthesis.candidates_enumerated);
                        attempted_improvement_operator_ids
                            .extend(synthesis.attempted_operator_ids.iter().cloned());
                        rejected_improvement_operator_ids
                            .extend(synthesis.rejected_operator_ids.iter().cloned());
                    }
                }
                for variant in &source_bound.patch_variants {
                    if variant.selected_candidate_indices.len() != source_bound.alternatives.len() {
                        return Err("SOURCE_BOUND_PATCH_VARIANT_CARDINALITY".to_string());
                    }
                    let syntheses = source_bound
                        .alternatives
                        .iter()
                        .zip(&variant.selected_candidate_indices)
                        .map(|(alternative, selected)| {
                            if *selected == 0 {
                                Ok(alternative.synthesis.clone())
                            } else {
                                alternative
                                    .closure_candidates
                                    .get(selected - 1)
                                    .map(|candidate| candidate.synthesis.clone())
                                    .ok_or_else(|| {
                                        "SOURCE_BOUND_PATCH_VARIANT_CANDIDATE_INDEX".to_string()
                                    })
                            }
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    if prior_counterexample_candidate_sha256s
                        .contains(&variant.replayable_patch.candidate_sha256)
                    {
                        continue;
                    }
                    candidate_variants.push((
                        variant.variant_id.clone(),
                        variant.selected_template_symbols.clone(),
                        variant.replayable_patch.clone(),
                        syntheses,
                    ));
                }
                if candidate_variants.is_empty()
                    && !prior_counterexample_candidate_sha256s.is_empty()
                {
                    failure_code = Some(
                        "PUBLIC_INFORMATION_INSUFFICIENT:ALL_CANDIDATES_PREVIOUSLY_FALSIFIED"
                            .to_string(),
                    );
                }
            }
            Err(error) => {
                failure_code = Some(format!(
                    "{}:{}",
                    error.kind.as_code(),
                    sha256(error.detail.as_bytes())
                ));
            }
        }

        for (variant_id, template_symbols, patch, variant_syntheses) in candidate_variants {
            source_bound_patch_variant_ids_attempted.push(variant_id.clone());
            source_bound_patch_variant_sha256s_attempted.push(patch.candidate_sha256.clone());
            candidate_sha256 = Some(patch.candidate_sha256.clone());
            materialization_is_one_to_one = patch.candidate_materialization_is_one_to_one;
            edit_atom_kinds.clear();
            source_edit_atom_kinds(&patch.edit, &mut edit_atom_kinds);
            let candidate_source = replay_source_bound_patch(&source, &patch)
                .map_err(|error| format!("{}:{}", error.kind.as_code(), error.detail))?;
            let sandbox_parent = config.state_dir.join("repository_repair_sandboxes");
            fs::create_dir_all(&sandbox_parent)
                .map_err(|error| format!("REPOSITORY_REPAIR_SANDBOX_PARENT:{error}"))?;
            let sandbox = sandbox_parent.join(&repair_id);
            if sandbox.exists() {
                remove_repository_repair_sandbox(config, &sandbox)?;
            }
            let log_path = diagnostics.join(format!("repository_repair_synthesis_{repair_id}.log"));
            let verification = (|| -> Result<LocalCommandReceipt, String> {
                copy_repository_to_repair_sandbox(config, &plan.root, &sandbox)?;
                let destination = sandbox.join(&relative);
                ensure_repository_repair_file_writable(&destination)?;
                fs::write(&destination, &candidate_source)
                    .map_err(|error| format!("REPOSITORY_REPAIR_CANDIDATE_WRITE:{error}"))?;
                let arg_refs = plan.args.iter().map(String::as_str).collect::<Vec<_>>();
                command_receipt_with_incremental(
                    &plan.program,
                    &arg_refs,
                    &sandbox,
                    &runtime_validation_target_dir(config),
                    MAX_CORE_COHORT_VALIDATION_MS,
                    &log_path,
                    true,
                )
            })();
            let cleanup = remove_repository_repair_sandbox(config, &sandbox);
            sandbox_cleaned = cleanup.is_ok() && !sandbox.exists();
            let _ = fs::remove_file(&log_path);
            match verification {
                Ok(mut command) => {
                    sandbox_verified = command.success && sandbox_cleaned;
                    command.diagnostic_tail =
                        format!("SANDBOX_VALIDATION_OUTPUT_SHA256:{}", command.output_sha256);
                    sandbox_command = Some(command);
                    if sandbox_verified {
                        selected_source_bound_patch_variant_id = Some(variant_id);
                        selected_source_bound_template_symbols = template_symbols;
                        successful_syntheses = variant_syntheses;
                        selected_candidate_source = Some(candidate_source);
                        failure_code = None;
                        break;
                    } else {
                        failure_code = Some(if sandbox_cleaned {
                            "PUBLIC_INFORMATION_INSUFFICIENT:CANDIDATE_FAILED_PUBLIC_TESTS"
                                .to_string()
                        } else {
                            "PUBLIC_INFORMATION_INSUFFICIENT:SANDBOX_CLEANUP_FAILED".to_string()
                        });
                    }
                }
                Err(error) => {
                    failure_code = Some(format!(
                        "PUBLIC_INFORMATION_INSUFFICIENT:{}",
                        sha256(error.as_bytes())
                    ));
                }
            }
        }

        let mut scope_fingerprint_after = repository_validation_scope_fingerprint(
            &plan.root,
            &plan.scope_paths,
            config.resources.max_file_bytes,
        )?;
        let mut authoritative_scope_stable =
            validation.scope_fingerprint_before == scope_fingerprint_after;
        sandbox_verified &= authoritative_scope_stable;
        if !authoritative_scope_stable {
            failure_code =
                Some("CONFLICTING_SOURCE_BOUND_EDITS:AUTHORITATIVE_SCOPE_CHANGED".to_string());
        }
        if sandbox_verified
            && (!config.repository_mutation.enabled
                || authoritative_installation_attempts
                    < config.repository_mutation.max_installations_per_step)
        {
            let candidate = selected_candidate_source
                .as_deref()
                .ok_or_else(|| "REPOSITORY_INSTALL_SELECTED_CANDIDATE_MISSING".to_string())?;
            let sandbox_evidence_sha256 = sandbox_command
                .as_ref()
                .filter(|command| command.success)
                .map(|command| command.output_sha256.clone())
                .ok_or_else(|| "REPOSITORY_INSTALL_SANDBOX_EVIDENCE_MISSING".to_string())?;
            let pending_improvement_operators = successful_syntheses
                .iter()
                .map(|synthesis| {
                    typed_mechanism_improvement_operator_from_receipt(
                        synthesis,
                        sandbox_evidence_sha256.clone(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            authoritative_installation_attempts = authoritative_installation_attempts
                .saturating_add(usize::from(config.repository_mutation.enabled));
            let install = install_verified_repository_candidate(
                config,
                RepositoryInstallRequest {
                    plan,
                    validation,
                    repair_id: &repair_id,
                    relative: &relative,
                    predecessor_sha256: &predecessor_sha256,
                    candidate_source: candidate,
                    generation: diagnostic.generation,
                    pending_improvement_operators: &pending_improvement_operators,
                },
            )?;
            candidate_installed = install.installed;
            rolled_back = install.rolled_back;
            authoritative_source_write_events = install.authoritative_source_write_events;
            authoritative_command = install.command;
            scope_fingerprint_after = install.scope_fingerprint_after;
            authoritative_scope_stable = install.authoritative_scope_stable;
            if install.failure_code.is_some() {
                failure_code = install.failure_code;
            }
        }
        selected_improvement_operator_ids.extend(
            successful_syntheses
                .iter()
                .filter_map(|synthesis| synthesis.selected_operator_id.clone()),
        );
        selected_improvement_operator_ids.sort();
        selected_improvement_operator_ids.dedup();
        attempted_improvement_operator_ids.sort();
        attempted_improvement_operator_ids.dedup();
        rejected_improvement_operator_ids.sort();
        rejected_improvement_operator_ids.dedup();
        let mut improvement_operators = Vec::new();
        let mut promoted_improvement_operator_ids = Vec::new();
        let repair_has_learning_authority = sandbox_verified
            && (!config.repository_mutation.enabled || candidate_installed)
            && !rolled_back;
        if repair_has_learning_authority {
            let evidence_sha256 = if candidate_installed {
                authoritative_command
                    .as_ref()
                    .filter(|command| command.success)
                    .map(|command| command.output_sha256.clone())
                    .ok_or_else(|| "REPOSITORY_REPAIR_AUTHORITATIVE_EVIDENCE_MISSING".to_string())?
            } else {
                sandbox_command
                    .as_ref()
                    .filter(|command| command.success)
                    .map(|command| command.output_sha256.clone())
                    .ok_or_else(|| "REPOSITORY_REPAIR_SANDBOX_EVIDENCE_MISSING".to_string())?
            };
            for synthesis in &successful_syntheses {
                let operator = typed_mechanism_improvement_operator_from_receipt(
                    synthesis,
                    evidence_sha256.clone(),
                )?;
                if !available_operator_ids.contains(&operator.operator_id) {
                    promoted_improvement_operator_ids.push(operator.operator_id.clone());
                }
                improvement_operators.push(operator);
            }
            improvement_operators.sort_by(|left, right| left.operator_id.cmp(&right.operator_id));
            improvement_operators.dedup_by(|left, right| left.operator_id == right.operator_id);
            promoted_improvement_operator_ids.sort();
            promoted_improvement_operator_ids.dedup();
        }
        let receipt = RepositoryRepairSynthesisReceipt {
            schema: REPOSITORY_REPAIR_SYNTHESIS_SCHEMA.to_string(),
            repair_id,
            repair_problem_id,
            synthesis_capability_sha256: synthesis_capability_sha256.clone(),
            source_repair_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
            originating_validation_id: validation.validation_id.clone(),
            originating_diagnostic_id: diagnostic.diagnostic_id.clone(),
            generation: diagnostic.generation,
            root_index: plan.root_index,
            root_sha256: sha256(plan.root.to_string_lossy().as_bytes()),
            source_relative_path: relative,
            predecessor_sha256,
            candidate_sha256,
            source_bound_receipt_sha256,
            source_bound_alternative_sha256,
            source_bound_patch_variant_ids_attempted,
            source_bound_patch_variant_sha256s_attempted,
            prior_counterexample_candidate_sha256s: prior_counterexample_candidate_sha256s
                .into_iter()
                .collect(),
            selected_source_bound_patch_variant_id,
            selected_source_bound_template_symbols,
            operator_family: "PUBLIC_SYMBOL_EXECUTION_CLOSURE_TO_TYPED_ATOMIC_SOURCE_PATCH"
                .to_string(),
            edit_atom_kinds: edit_atom_kinds.into_iter().collect(),
            available_improvement_operator_ids: available_operator_ids.iter().cloned().collect(),
            attempted_improvement_operator_ids,
            rejected_improvement_operator_ids,
            selected_improvement_operator_ids,
            promoted_improvement_operator_ids,
            improvement_operators,
            typed_candidates_enumerated,
            candidate_materialization_is_one_to_one: materialization_is_one_to_one,
            failure_code,
            scope_fingerprint_before: validation.scope_fingerprint_before.clone(),
            scope_fingerprint_after,
            authoritative_scope_stable,
            sandbox_command,
            authoritative_command,
            sandbox_verified,
            sandbox_cleaned,
            rolled_back,
            candidate_installed,
            authoritative_source_write_events,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
        };
        write_immutable_json(&receipt_path, &receipt)?;
        let receipt_sha256 = json_sha256(&receipt)?;
        for operator in &receipt.improvement_operators {
            persist_source_bound_improvement_operator(config, operator, &receipt, &receipt_sha256)?;
        }
        cleanup_recent_files(&diagnostics, "repository_repair_synthesis_", 64)?;
        if receipt.sandbox_verified
            && receipt.sandbox_cleaned
            && (!config.repository_mutation.enabled || receipt.candidate_installed)
            && !receipt.rolled_back
        {
            let observation_id = repository_repair_observation_id(&receipt, &receipt_sha256);
            return Ok(Some((
                receipt_sha256,
                observation_id,
                receipt.candidate_installed,
            )));
        }
        if config.repository_mutation.enabled
            && authoritative_installation_attempts
                >= config.repository_mutation.max_installations_per_step
        {
            return Ok(None);
        }
    }
    Ok(None)
}

fn reusable_python_test_paths(
    config: &GrowthSupervisorConfig,
    root_index: usize,
    root: &Path,
) -> Result<Option<(Vec<PathBuf>, String)>, String> {
    let diagnostics = config.state_dir.join("diagnostics");
    let Ok(entries) = fs::read_dir(&diagnostics) else {
        return Ok(None);
    };
    let mut receipts = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("repository_cohort_validation_")
                && entry.path().extension().and_then(OsStr::to_str) == Some("json")
        })
        .filter_map(|entry| {
            let modified = entry
                .metadata()
                .ok()?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    receipts.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in receipts {
        let Ok(receipt) = read_json::<RepositoryCohortValidationReceipt>(&path) else {
            continue;
        };
        if !receipt.success
            || receipt.root_index != root_index
            || receipt.validator_kind != RepositoryValidatorKind::PythonPytest
            || (!receipt.test_selection_source.is_empty()
                && receipt.test_selection_source != "OBSERVED_TEST_COHORT")
            || receipt.test_paths.is_empty()
            || receipt
                .test_paths
                .iter()
                .any(|test| test.extension().and_then(OsStr::to_str) != Some("py"))
            || receipt
                .test_paths
                .iter()
                .any(|test| validated_repository_file(root, test).is_err())
        {
            continue;
        }
        let receipt_sha256 = json_sha256(&receipt)?;
        let mut tests = receipt.test_paths;
        tests.sort();
        tests.dedup();
        tests.truncate(MAX_REPOSITORY_TEST_PATHS);
        return Ok(Some((tests, receipt_sha256)));
    }
    Ok(None)
}

fn nearest_cargo_manifest(root: &Path, relative: &Path) -> Option<PathBuf> {
    let mut directory = relative.parent()?.to_path_buf();
    loop {
        let candidate = directory.join("Cargo.toml");
        if root.join(&candidate).is_file() && validated_repository_file(root, &candidate).is_ok() {
            return Some(candidate);
        }
        if !directory.pop() {
            break;
        }
    }
    None
}

fn repository_validation_plan(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Result<Option<RepositoryValidationPlan>, String> {
    let mut by_root: BTreeMap<usize, Vec<(&LearningObservation, PathBuf)>> = BTreeMap::new();
    for observation in observations {
        if let Some((root_index, relative)) = logical_root_relative(&observation.logical_path) {
            if root_index < config.watched_roots.len() {
                by_root
                    .entry(root_index)
                    .or_default()
                    .push((observation, relative));
            }
        }
    }
    for (root_index, entries) in by_root {
        let root = config.watched_roots[root_index].clone();
        let python_entries = entries
            .iter()
            .filter(|(_, relative)| relative.extension().and_then(OsStr::to_str) == Some("py"))
            .cloned()
            .collect::<Vec<_>>();
        let python_implementation_present = python_entries
            .iter()
            .any(|(_, relative)| !path_is_dedicated_test(relative));
        'python_plan: {
            if root.join("pyproject.toml").is_file() {
                let mut test_paths = python_entries
                    .iter()
                    .filter(|(_, relative)| {
                        path_is_dedicated_test(relative)
                            && relative
                                .file_name()
                                .and_then(OsStr::to_str)
                                .is_some_and(|name| name.starts_with("test_"))
                    })
                    .map(|(_, relative)| relative.clone())
                    .collect::<Vec<_>>();
                test_paths.sort();
                test_paths.dedup();
                test_paths.truncate(MAX_REPOSITORY_TEST_PATHS);
                if !python_implementation_present && test_paths.is_empty() {
                    break 'python_plan;
                }
                let (test_selection_source, reused_validation_receipt_sha256) =
                    if test_paths.is_empty() {
                        let Some((reused, receipt_sha256)) =
                            reusable_python_test_paths(config, root_index, &root)?
                        else {
                            break 'python_plan;
                        };
                        test_paths = reused;
                        ("VERIFIED_RECEIPT_REUSE".to_string(), Some(receipt_sha256))
                    } else {
                        ("OBSERVED_TEST_COHORT".to_string(), None)
                    };
                for relative in &test_paths {
                    validated_repository_file(&root, relative)?;
                }
                let mut scope_paths = python_entries
                    .iter()
                    .map(|(_, relative)| relative.clone())
                    .collect::<Vec<_>>();
                scope_paths.extend(test_paths.iter().cloned());
                scope_paths.push(PathBuf::from("pyproject.toml"));
                scope_paths.sort();
                scope_paths.dedup();
                let mut input_observation_ids = python_entries
                    .iter()
                    .map(|(observation, _)| observation.observation_id.clone())
                    .collect::<Vec<_>>();
                input_observation_ids.sort();
                input_observation_ids.dedup();
                // Structured requirement targets have exact-owner priority.
                // Diagnostic text is useful fallback evidence, but cannot
                // displace a product symbol already bound by a validated
                // observed-to-expected contract.
                let mut public_contract_target_symbols = python_entries
                    .iter()
                    .flat_map(|(observation, _)| &observation.public_contract_deltas)
                    .flat_map(|delta| delta.target_symbols.iter().cloned())
                    .collect::<Vec<_>>();
                public_contract_target_symbols.sort();
                public_contract_target_symbols.dedup();
                public_contract_target_symbols.truncate(MAX_REPOSITORY_REPAIR_TARGET_SYMBOLS);
                let Ok(program) = resolve_local_program("python") else {
                    break 'python_plan;
                };
                let mut args = vec![
                    "-m".to_string(),
                    "pytest".to_string(),
                    "-q".to_string(),
                    "--disable-warnings".to_string(),
                    "--maxfail=1".to_string(),
                    "-p".to_string(),
                    "no:cacheprovider".to_string(),
                ];
                args.extend(
                    test_paths
                        .iter()
                        .map(|path| path.to_string_lossy().replace('\\', "/")),
                );
                return Ok(Some(RepositoryValidationPlan {
                    validator_kind: RepositoryValidatorKind::PythonPytest,
                    test_selection_source,
                    reused_validation_receipt_sha256,
                    root_index,
                    root,
                    input_observation_ids,
                    public_contract_target_symbols,
                    scope_paths,
                    test_paths,
                    program,
                    args,
                }));
            }
        }

        let mut rust_by_manifest: BTreeMap<PathBuf, Vec<(&LearningObservation, PathBuf)>> =
            BTreeMap::new();
        for (observation, relative) in &entries {
            if relative.extension().and_then(OsStr::to_str) == Some("rs") {
                if let Some(manifest) = nearest_cargo_manifest(&root, relative) {
                    rust_by_manifest
                        .entry(manifest)
                        .or_default()
                        .push((observation, relative.clone()));
                }
            }
        }
        if let Some((manifest, rust_entries)) = rust_by_manifest.into_iter().next() {
            let cargo_name = config.source_mutation.cargo_executable.to_string_lossy();
            let Ok(program) = resolve_local_program(&cargo_name) else {
                continue;
            };
            let mut input_observation_ids = rust_entries
                .iter()
                .map(|(observation, _)| observation.observation_id.clone())
                .collect::<Vec<_>>();
            input_observation_ids.sort();
            input_observation_ids.dedup();
            let mut scope_paths = rust_entries
                .iter()
                .map(|(_, relative)| relative.clone())
                .collect::<Vec<_>>();
            scope_paths.push(manifest.clone());
            for workspace_file in ["Cargo.toml", "Cargo.lock"] {
                let relative = PathBuf::from(workspace_file);
                if root.join(&relative).is_file() {
                    scope_paths.push(relative);
                }
            }
            scope_paths.sort();
            scope_paths.dedup();
            let manifest_argument = manifest.to_string_lossy().replace('\\', "/");
            return Ok(Some(RepositoryValidationPlan {
                validator_kind: RepositoryValidatorKind::RustCargo,
                test_selection_source: "CRATE_LOCAL_LIB_TESTS".to_string(),
                reused_validation_receipt_sha256: None,
                root_index,
                root,
                input_observation_ids,
                public_contract_target_symbols: Vec::new(),
                scope_paths,
                test_paths: vec![manifest],
                program,
                args: vec![
                    "test".to_string(),
                    "--manifest-path".to_string(),
                    manifest_argument,
                    "--lib".to_string(),
                    "--quiet".to_string(),
                    "--locked".to_string(),
                ],
            }));
        }
    }
    Ok(None)
}

fn source_mutation_watch_prefix(config: &GrowthSupervisorConfig) -> Result<Option<String>, String> {
    if !config.source_mutation.enabled {
        return Ok(None);
    }
    let source_root = fs::canonicalize(&config.source_mutation.source_root)
        .map_err(|error| format!("CORE_COHORT_SOURCE_ROOT_CANONICALIZE:{error}"))?;
    for (index, watched_root) in config.watched_roots.iter().enumerate() {
        let watched_root = fs::canonicalize(watched_root)
            .map_err(|error| format!("CORE_COHORT_WATCH_ROOT_CANONICALIZE:{error}"))?;
        if source_root.starts_with(&watched_root) {
            let relative = source_root
                .strip_prefix(&watched_root)
                .map_err(|_| "CORE_COHORT_SOURCE_PREFIX_STRIP".to_string())?;
            let suffix = relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            return Ok(Some(if suffix.is_empty() {
                format!("ROOT_{index}/")
            } else {
                format!("ROOT_{index}/{suffix}/")
            }));
        }
    }
    Ok(None)
}

// Runtime validation executes while developers or the autonomous mutator may compile the same
// workspace. Keep one persistent warm cache beside the deployed binaries so Windows never has to
// relink a test executable that another process is currently holding open.
fn runtime_validation_target_dir(config: &GrowthSupervisorConfig) -> PathBuf {
    config
        .source_mutation
        .runtime_bin_dir
        .parent()
        .unwrap_or(&config.source_mutation.runtime_bin_dir)
        .join("validation-target")
}

fn core_cohort_observation_ids(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Result<Vec<String>, String> {
    let Some(source_prefix) = source_mutation_watch_prefix(config)? else {
        return Ok(Vec::new());
    };
    let mut observation_ids = observations
        .iter()
        .filter(|observation| observation.logical_path.starts_with(&source_prefix))
        .map(|observation| observation.observation_id.clone())
        .collect::<Vec<_>>();
    observation_ids.sort();
    observation_ids.dedup();
    Ok(observation_ids)
}

fn syn_items_contain_test(items: &[syn::Item]) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Fn(function) => function
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("test")),
        syn::Item::Mod(module) => module
            .content
            .as_ref()
            .is_some_and(|(_, nested)| syn_items_contain_test(nested)),
        _ => false,
    })
}

fn module_has_targetable_test_filter(source_root: &Path, module: &str) -> bool {
    let source_dir = source_root.join("crates/semantic-reasoning/src");
    let module_path = source_dir.join(format!("{module}.rs"));
    let Ok(source) = fs::read_to_string(&module_path) else {
        return false;
    };
    let Ok(file) = syn::parse_file(&source) else {
        return false;
    };
    file.items.iter().any(|item| {
        let syn::Item::Mod(test_module) = item else {
            return false;
        };
        if test_module.ident != "tests" {
            return false;
        }
        if let Some((_, items)) = &test_module.content {
            return syn_items_contain_test(items);
        }
        [
            source_dir.join(module).join("tests.rs"),
            source_dir.join(module).join("tests/mod.rs"),
        ]
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter_map(|source| syn::parse_file(&source).ok())
        .any(|file| syn_items_contain_test(&file.items))
    })
}

fn core_validation_plan(
    config: &GrowthSupervisorConfig,
    generation: u64,
    observations: &[LearningObservation],
) -> Result<CoreValidationPlan, String> {
    // A periodic canary protects the complete deployed runtime surface.  It
    // must not implicitly enable every sealed historical campaign: at a
    // plateau the generation remains divisible by the interval, so each new
    // runtime-core source fingerprint otherwise pays the multi-minute archive
    // suite again. Historical modules still trigger their own exact regression
    // surface when one of them actually changes.
    let full_regression_canary = generation.is_multiple_of(FULL_CORE_REGRESSION_CANARY_INTERVAL);
    let source_prefix = source_mutation_watch_prefix(config)?;
    let runtime_files = runtime_core_source_files(&config.source_mutation.source_root).ok();
    let historical_path_observed = source_prefix.as_ref().is_some_and(|prefix| {
        observations
            .iter()
            .filter_map(|observation| observation.logical_path.strip_prefix(prefix))
            .map(|relative| config.source_mutation.source_root.join(relative))
            .any(|path| {
                fs::canonicalize(path).ok().is_none_or(|canonical| {
                    runtime_files
                        .as_ref()
                        .is_none_or(|runtime_files| !runtime_files.contains(&canonical))
                })
            })
    });
    let historical_regression_required = historical_path_observed;
    let validation_args = || {
        let mut args = vec![
            "test".to_string(),
            "-p".to_string(),
            "semantic-reasoning".to_string(),
            "--lib".to_string(),
        ];
        if !historical_regression_required
            && runtime_core_feature_available(&config.source_mutation.source_root)
        {
            args.extend([
                "--no-default-features".to_string(),
                "--features".to_string(),
                "runtime-core".to_string(),
            ]);
        }
        args.extend(["--quiet".to_string(), "--locked".to_string()]);
        args
    };
    let Some(source_prefix) = source_prefix else {
        return Ok(CoreValidationPlan {
            args: validation_args(),
            validation_scope: if historical_regression_required {
                "FULL_HISTORICAL_REGRESSION_CANARY".to_string()
            } else if full_regression_canary {
                "FULL_RUNTIME_CORE_REGRESSION_CANARY".to_string()
            } else {
                "RUNTIME_CORE_REGRESSION".to_string()
            },
            targeted_test_filter: None,
            full_regression_canary,
        });
    };
    let mut modules = BTreeSet::new();
    let mut core_paths = 0_usize;
    for observation in observations
        .iter()
        .filter(|observation| observation.logical_path.starts_with(&source_prefix))
    {
        core_paths = core_paths.saturating_add(1);
        let relative = observation
            .logical_path
            .strip_prefix(&source_prefix)
            .unwrap_or_default();
        let parts = relative.split('/').collect::<Vec<_>>();
        let module = if parts.len() == 4
            && parts[0] == "crates"
            && parts[1] == "semantic-reasoning"
            && parts[2] == "src"
            && parts[3].ends_with(".rs")
        {
            parts[3].strip_suffix(".rs").unwrap_or_default()
        } else {
            ""
        };
        if module.is_empty() || module == "lib" || module == "main" || module.ends_with("_main") {
            modules.insert(String::new());
        } else {
            modules.insert(module.to_string());
        }
    }
    let mut targetable_module_missing_tests = false;
    if !historical_regression_required
        && !full_regression_canary
        && core_paths > 0
        && modules.len() == 1
    {
        let module = modules.into_iter().next().unwrap_or_default();
        if !module.is_empty()
            && module_has_targetable_test_filter(&config.source_mutation.source_root, &module)
        {
            let filter = format!("{module}::tests::");
            let mut args = validation_args();
            args.push(filter.clone());
            return Ok(CoreValidationPlan {
                args,
                validation_scope: "CHANGED_RUST_MODULE".to_string(),
                targeted_test_filter: Some(filter),
                full_regression_canary: false,
            });
        } else if !module.is_empty() {
            targetable_module_missing_tests = true;
        }
    }
    Ok(CoreValidationPlan {
        args: validation_args(),
        validation_scope: if historical_regression_required {
            "FULL_HISTORICAL_REGRESSION_CANARY".to_string()
        } else if full_regression_canary {
            "FULL_RUNTIME_CORE_REGRESSION_CANARY".to_string()
        } else if targetable_module_missing_tests {
            "RUNTIME_CORE_REGRESSION_NO_TARGETABLE_MODULE_TESTS".to_string()
        } else {
            "RUNTIME_CORE_REGRESSION".to_string()
        },
        targeted_test_filter: None,
        full_regression_canary,
    })
}

fn targeted_test_filter_executed(diagnostic_tail: &str) -> bool {
    !diagnostic_tail
        .lines()
        .any(|line| line.trim().starts_with("running 0 tests"))
}

fn validate_blocked_core_cohort(
    config: &GrowthSupervisorConfig,
    diagnostic: &AutonomousSelfInspectionReceipt,
    observations: &[LearningObservation],
) -> Result<(bool, Vec<String>, Vec<String>), String> {
    let input_observation_ids = core_cohort_observation_ids(config, observations)?;
    if input_observation_ids.is_empty() {
        return Ok((false, Vec::new(), Vec::new()));
    }
    let validation_plan = core_validation_plan(config, diagnostic.generation, observations)?;

    let source_fingerprint_before =
        full_workspace_semantic_fingerprint(&config.source_mutation.source_root)?;
    let validation_id = sha256(
        format!(
            "CORE_COHORT_VALIDATION_2:{}:{}:{}",
            source_fingerprint_before,
            input_observation_ids.join(":"),
            validation_plan.args.join("\u{1f}")
        )
        .as_bytes(),
    );
    let diagnostics = config.state_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics)
        .map_err(|error| format!("CORE_COHORT_DIAGNOSTICS_CREATE:{error}"))?;
    let receipt_path = diagnostics.join(format!("core_cohort_validation_{validation_id}.json"));
    let receipt = if receipt_path.exists() {
        let existing: CoreCohortValidationReceipt = read_json(&receipt_path)?;
        if existing.schema != "B_CORE_COHORT_VALIDATION_1"
            || existing.validation_id != validation_id
            || existing.input_observation_ids != input_observation_ids
            || existing.source_fingerprint_before != source_fingerprint_before
            || existing.validation_scope != validation_plan.validation_scope
            || existing.targeted_test_filter != validation_plan.targeted_test_filter
            || existing.full_regression_canary != validation_plan.full_regression_canary
        {
            return Err("CORE_COHORT_VALIDATION_RECEIPT_MISMATCH".to_string());
        }
        existing
    } else if let Some((reused, reused_receipt_sha256)) =
        reusable_core_validation_receipt(config, &source_fingerprint_before, &validation_plan)?
    {
        let command = LocalCommandReceipt {
            program: reused.command.program,
            args: validation_plan.args.clone(),
            cargo_incremental: reused.command.cargo_incremental,
            exit_code: Some(0),
            success: true,
            timed_out: false,
            duration_ms: 0,
            output_sha256: reused.command.output_sha256,
            diagnostic_tail: format!(
                "REUSED_SOURCE_IDENTICAL_CORE_VALIDATION_RECEIPT:{reused_receipt_sha256}"
            ),
            ..Default::default()
        };
        let receipt = CoreCohortValidationReceipt {
            schema: "B_CORE_COHORT_VALIDATION_1".to_string(),
            validation_id,
            originating_diagnostic_id: diagnostic.diagnostic_id.clone(),
            generation: diagnostic.generation,
            source_root_sha256: sha256(
                config
                    .source_mutation
                    .source_root
                    .to_string_lossy()
                    .as_bytes(),
            ),
            input_observation_ids,
            source_fingerprint_before: source_fingerprint_before.clone(),
            source_fingerprint_after: source_fingerprint_before,
            workspace_stable_during_validation: true,
            validation_scope: validation_plan.validation_scope.clone(),
            targeted_test_filter: validation_plan.targeted_test_filter.clone(),
            full_regression_canary: false,
            reused_validation_receipt_sha256: Some(reused_receipt_sha256),
            command,
            success: true,
            authoritative_source_write_events: 0,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        write_immutable_json(&receipt_path, &receipt)?;
        cleanup_recent_files(&diagnostics, "core_cohort_validation_", 64)?;
        receipt
    } else {
        let log_path = diagnostics.join(format!("core_cohort_validation_{validation_id}.log"));
        let arg_refs = validation_plan
            .args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let command = command_receipt_with_incremental(
            &config.source_mutation.cargo_executable,
            &arg_refs,
            &config.source_mutation.source_root,
            &runtime_validation_target_dir(config),
            config
                .source_mutation
                .validation_timeout_ms
                .clamp(1, MAX_CORE_COHORT_VALIDATION_MS),
            &log_path,
            true,
        );
        let _ = fs::remove_file(&log_path);
        let command = command?;
        let targeted_tests_executed = validation_plan.targeted_test_filter.is_none()
            || targeted_test_filter_executed(&command.diagnostic_tail);
        let source_fingerprint_after =
            full_workspace_semantic_fingerprint(&config.source_mutation.source_root)?;
        let workspace_stable_during_validation =
            source_fingerprint_before == source_fingerprint_after;
        let receipt = CoreCohortValidationReceipt {
            schema: "B_CORE_COHORT_VALIDATION_1".to_string(),
            validation_id,
            originating_diagnostic_id: diagnostic.diagnostic_id.clone(),
            generation: diagnostic.generation,
            source_root_sha256: sha256(
                config
                    .source_mutation
                    .source_root
                    .to_string_lossy()
                    .as_bytes(),
            ),
            input_observation_ids,
            source_fingerprint_before,
            source_fingerprint_after,
            workspace_stable_during_validation,
            validation_scope: validation_plan.validation_scope.clone(),
            targeted_test_filter: validation_plan.targeted_test_filter.clone(),
            full_regression_canary: validation_plan.full_regression_canary,
            reused_validation_receipt_sha256: None,
            success: command.success
                && workspace_stable_during_validation
                && targeted_tests_executed,
            command,
            authoritative_source_write_events: 0,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        write_immutable_json(&receipt_path, &receipt)?;
        cleanup_recent_files(&diagnostics, "core_cohort_validation_", 64)?;
        receipt
    };
    let receipt_sha256 = json_sha256(&receipt)?;
    let output_observation_ids = if receipt.success {
        vec![sha256(
            format!(
                "CORE_COHORT_VALIDATION_OBSERVATION:{}:{}",
                receipt.validation_id, receipt_sha256
            )
            .as_bytes(),
        )]
    } else {
        Vec::new()
    };
    Ok((
        receipt.success,
        vec![receipt_sha256],
        output_observation_ids,
    ))
}

fn validate_blocked_repository_cohort(
    config: &GrowthSupervisorConfig,
    diagnostic: &AutonomousSelfInspectionReceipt,
    observations: &[LearningObservation],
) -> Result<RepositoryCohortValidationOutcome, String> {
    let Some(plan) = repository_validation_plan(config, observations)? else {
        return Ok(RepositoryCohortValidationOutcome {
            executed: false,
            sandbox_repair_verified: false,
            repository_repair_installed: false,
            evidence_sha256: Vec::new(),
            output_observation_ids: Vec::new(),
        });
    };
    let scope_fingerprint_before = repository_validation_scope_fingerprint(
        &plan.root,
        &plan.scope_paths,
        config.resources.max_file_bytes,
    )?;
    let program_sha256 = file_sha256(&plan.program, 512 * 1024 * 1024)?;
    let validation_id = sha256(
        format!(
            "REPOSITORY_COHORT_VALIDATION_2:{}:{:?}:{}:{}:{}:{}:{}:{}",
            plan.root_index,
            plan.validator_kind,
            plan.test_selection_source,
            plan.reused_validation_receipt_sha256
                .as_deref()
                .unwrap_or("OBSERVED"),
            scope_fingerprint_before,
            program_sha256,
            plan.input_observation_ids.join(":"),
            plan.args.join("\u{1f}")
        )
        .as_bytes(),
    );
    let diagnostics = config.state_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics)
        .map_err(|error| format!("REPOSITORY_COHORT_DIAGNOSTICS_CREATE:{error}"))?;
    let receipt_path =
        diagnostics.join(format!("repository_cohort_validation_{validation_id}.json"));
    let receipt = if receipt_path.exists() {
        let existing: RepositoryCohortValidationReceipt = read_json(&receipt_path)?;
        if existing.schema != "B_REPOSITORY_COHORT_VALIDATION_1"
            || existing.validation_id != validation_id
            || existing.root_index != plan.root_index
            || existing.validator_kind != plan.validator_kind
            || existing.test_selection_source != plan.test_selection_source
            || existing.reused_validation_receipt_sha256.as_ref()
                != plan.reused_validation_receipt_sha256.as_ref()
            || existing.input_observation_ids != plan.input_observation_ids
            || existing.test_paths != plan.test_paths
            || existing.scope_fingerprint_before != scope_fingerprint_before
            || existing.program_sha256 != program_sha256
        {
            return Err("REPOSITORY_COHORT_VALIDATION_RECEIPT_MISMATCH".to_string());
        }
        existing
    } else {
        let log_path =
            diagnostics.join(format!("repository_cohort_validation_{validation_id}.log"));
        let arg_refs = plan.args.iter().map(String::as_str).collect::<Vec<_>>();
        let command = command_receipt_with_incremental(
            &plan.program,
            &arg_refs,
            &plan.root,
            &runtime_validation_target_dir(config),
            MAX_CORE_COHORT_VALIDATION_MS,
            &log_path,
            true,
        );
        let _ = fs::remove_file(&log_path);
        let command = command?;
        let scope_fingerprint_after = repository_validation_scope_fingerprint(
            &plan.root,
            &plan.scope_paths,
            config.resources.max_file_bytes,
        )?;
        let scope_stable_during_validation = scope_fingerprint_before == scope_fingerprint_after;
        let receipt = RepositoryCohortValidationReceipt {
            schema: "B_REPOSITORY_COHORT_VALIDATION_1".to_string(),
            validation_id,
            originating_diagnostic_id: diagnostic.diagnostic_id.clone(),
            generation: diagnostic.generation,
            root_index: plan.root_index,
            root_sha256: sha256(plan.root.to_string_lossy().as_bytes()),
            validator_kind: plan.validator_kind,
            test_selection_source: plan.test_selection_source.clone(),
            reused_validation_receipt_sha256: plan.reused_validation_receipt_sha256.clone(),
            input_observation_ids: plan.input_observation_ids.clone(),
            test_paths: plan.test_paths.clone(),
            scope_fingerprint_before,
            scope_fingerprint_after,
            scope_stable_during_validation,
            success: command.success && scope_stable_during_validation,
            program_sha256,
            command,
            authoritative_source_write_events: 0,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        write_immutable_json(&receipt_path, &receipt)?;
        cleanup_recent_files(&diagnostics, "repository_cohort_validation_", 64)?;
        receipt
    };
    let receipt_sha256 = json_sha256(&receipt)?;
    if receipt.success {
        let output_observation_ids = vec![sha256(
            format!(
                "REPOSITORY_COHORT_VALIDATION_OBSERVATION:{}:{}",
                receipt.validation_id, receipt_sha256
            )
            .as_bytes(),
        )];
        return Ok(RepositoryCohortValidationOutcome {
            executed: true,
            sandbox_repair_verified: false,
            repository_repair_installed: false,
            evidence_sha256: vec![receipt_sha256],
            output_observation_ids,
        });
    }
    if let Some((repair_receipt_sha256, observation_id, repository_repair_installed)) =
        try_synthesize_failed_python_cohort(config, diagnostic, &plan, &receipt)?
    {
        return Ok(RepositoryCohortValidationOutcome {
            executed: true,
            sandbox_repair_verified: true,
            repository_repair_installed,
            evidence_sha256: vec![receipt_sha256, repair_receipt_sha256],
            output_observation_ids: vec![observation_id],
        });
    }
    Ok(RepositoryCohortValidationOutcome {
        executed: false,
        sandbox_repair_verified: false,
        repository_repair_installed: false,
        evidence_sha256: vec![receipt_sha256],
        output_observation_ids: Vec::new(),
    })
}

fn runtime_repair_action(
    config: &GrowthSupervisorConfig,
    receipt: &AutonomousSelfInspectionReceipt,
    scan_observations: &[LearningObservation],
    naive_cohort: &[LearningObservation],
    evidence_aware_cohort: &[LearningObservation],
) -> Result<Option<(RuntimeRepairActionReceipt, Option<LearningObservation>)>, String> {
    let Some(mechanism) = receipt.repair_mechanism else {
        return Ok(None);
    };
    if receipt.repair_disposition != RepairDisposition::RuntimeRepairActive
        || !receipt
            .experiments
            .iter()
            .all(|experiment| experiment.causal_support)
    {
        return Ok(None);
    }
    let (
        executed,
        changed_runtime_decision,
        execution_evidence_sha256,
        output_observation_ids,
        repository_sandbox_repair_verified,
        repository_repair_installed,
    ) = match mechanism {
        RuntimeRepairMechanism::ReplayVerifiedEventAgainstIndexedContent => {
            let outputs = scan_observations
                .iter()
                .filter(|observation| observation.work_event_id.is_some())
                .map(|observation| observation.observation_id.clone())
                .collect::<Vec<_>>();
            let evidence = scan_observations
                .iter()
                .filter(|observation| observation.work_event_id.is_some())
                .map(json_sha256)
                .collect::<Result<Vec<_>, _>>()?;
            (
                !outputs.is_empty(),
                !outputs.is_empty(),
                evidence,
                outputs,
                false,
                false,
            )
        }
        RuntimeRepairMechanism::EvidenceAwareBoundedCohortRouting => {
            let outputs = evidence_aware_cohort
                .iter()
                .map(|observation| observation.observation_id.clone())
                .collect::<Vec<_>>();
            let evidence = evidence_aware_cohort
                .iter()
                .map(json_sha256)
                .collect::<Result<Vec<_>, _>>()?;
            let changed = outputs
                != naive_cohort
                    .iter()
                    .map(|observation| observation.observation_id.clone())
                    .collect::<Vec<_>>();
            (
                changed && cohort_has_verification_evidence(evidence_aware_cohort),
                changed,
                evidence,
                outputs,
                false,
                false,
            )
        }
        RuntimeRepairMechanism::BootstrapFrozenCoreEvaluatorCanary => {
            let inspection_sha256 = json_sha256(receipt)?;
            let observation_id = sha256(
                format!(
                    "MUTUAL_RECURSIVE_BOOTSTRAP:{}:{}",
                    receipt.generation, inspection_sha256
                )
                .as_bytes(),
            );
            (
                true,
                true,
                vec![inspection_sha256],
                vec![observation_id],
                false,
                false,
            )
        }
        RuntimeRepairMechanism::ValidateBlockedCoreCohort => {
            let (success, evidence, outputs) =
                validate_blocked_core_cohort(config, receipt, evidence_aware_cohort)?;
            (success, success, evidence, outputs, false, false)
        }
        RuntimeRepairMechanism::ValidateBlockedRepositoryCohort => {
            let outcome =
                validate_blocked_repository_cohort(config, receipt, evidence_aware_cohort)?;
            (
                outcome.executed,
                outcome.executed,
                outcome.evidence_sha256,
                outcome.output_observation_ids,
                outcome.sandbox_repair_verified,
                outcome.repository_repair_installed,
            )
        }
    };
    let action_id = sha256(
        format!(
            "{}:{:?}:{}:{}:{}",
            receipt.diagnostic_id,
            mechanism,
            executed,
            changed_runtime_decision,
            output_observation_ids.join(":")
        )
        .as_bytes(),
    );
    let action = RuntimeRepairActionReceipt {
        schema: "B_CORE_RUNTIME_REPAIR_ACTION_1".to_string(),
        action_id,
        diagnostic_id: receipt.diagnostic_id.clone(),
        generation: receipt.generation,
        mechanism,
        executed,
        changed_runtime_decision,
        execution_evidence_sha256,
        output_observation_ids,
        authoritative_source_write_events: usize::from(repository_repair_installed),
        operator_selected: false,
        codex_calls: 0,
        external_llm_calls: 0,
    };
    // The same validated output may be selected by more than one diagnostic
    // action. Its immutable observation must therefore be a projection of the
    // executed validation evidence, not of the selecting action receipt or
    // wall clock. The action separately binds diagnostic -> output id.
    let observation_content_sha256 = sha256(
        format!(
            "B_CORE_RUNTIME_REPAIR_OBSERVATION_1:{:?}:{}",
            action.mechanism,
            action.execution_evidence_sha256.join(":")
        )
        .as_bytes(),
    );
    let observation = if mechanism == RuntimeRepairMechanism::BootstrapFrozenCoreEvaluatorCanary
        && action.executed
    {
        Some(LearningObservation {
            observation_id: action.output_observation_ids[0].clone(),
            work_event_id: None,
            logical_path: "INTERNAL/MUTUAL_CORE_EVALUATOR_BOOTSTRAP".to_string(),
            content_sha256: observation_content_sha256,
            predecessor_content_sha256: None,
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::Verification,
            work_outcome: WorkOutcome::Pass,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec![
                "AUTONOMOUS_SELF_INSPECTION".to_string(),
                "MUTUAL_REVALIDATION_GAP".to_string(),
                "REGRESSION_EVIDENCE".to_string(),
                "VERIFIED_PASS".to_string(),
            ],
            composition_roles: vec![
                "INVARIANT_CHECK".to_string(),
                "REGRESSION_TEST".to_string(),
            ],
            learning_score: 80,
            learning_value: LearningValue::High,
            reasons: vec![
                "generation zero has no observed core/evaluator mutual revalidation".to_string(),
                "the frozen independent verifier must reconstruct the candidate and reject the complete evaluator mutation suite"
                    .to_string(),
            ],
            verification_evidence_sha256: action.execution_evidence_sha256.clone(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: action.generation,
        })
    } else if mechanism == RuntimeRepairMechanism::ValidateBlockedCoreCohort && action.executed {
        let source_prefix = source_mutation_watch_prefix(config)?
            .ok_or_else(|| "CORE_COHORT_SOURCE_ROOT_NOT_WATCHED".to_string())?;
        Some(LearningObservation {
            observation_id: action.output_observation_ids[0].clone(),
            work_event_id: None,
            logical_path: format!(
                "{source_prefix}.b_core_validation/{}",
                action.output_observation_ids[0]
            ),
            content_sha256: observation_content_sha256,
            predecessor_content_sha256: None,
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::Verification,
            work_outcome: WorkOutcome::Pass,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec![
                "AUTONOMOUS_RUNTIME_REPAIR".to_string(),
                "CORE_COHORT_VALIDATION".to_string(),
                "REGRESSION_EVIDENCE".to_string(),
                "VERIFIED_PASS".to_string(),
            ],
            composition_roles: vec!["INVARIANT_CHECK".to_string(), "REGRESSION_TEST".to_string()],
            learning_score: 85,
            learning_value: LearningValue::High,
            reasons: vec![
                "high-value core implementation observations lacked executed regression evidence"
                    .to_string(),
                "bounded local core regression passed without source mutation or network access"
                    .to_string(),
            ],
            verification_evidence_sha256: action.execution_evidence_sha256.clone(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: action.generation,
        })
    } else if mechanism == RuntimeRepairMechanism::ValidateBlockedRepositoryCohort
        && action.executed
        && !repository_sandbox_repair_verified
    {
        let plan = repository_validation_plan(config, evidence_aware_cohort)?
            .ok_or_else(|| "REPOSITORY_COHORT_VALIDATION_PLAN_LOST".to_string())?;
        let validator_signal = match plan.validator_kind {
            RepositoryValidatorKind::PythonPytest => "PYTHON_PYTEST_VALIDATION",
            RepositoryValidatorKind::RustCargo => "RUST_CARGO_VALIDATION",
        };
        Some(LearningObservation {
            observation_id: action.output_observation_ids[0].clone(),
            work_event_id: None,
            logical_path: format!(
                "ROOT_{}/.b_repository_validation/{}",
                plan.root_index, action.output_observation_ids[0]
            ),
            content_sha256: observation_content_sha256,
            predecessor_content_sha256: None,
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::Verification,
            work_outcome: WorkOutcome::Pass,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec![
                "AUTONOMOUS_RUNTIME_REPAIR".to_string(),
                "REPOSITORY_COHORT_VALIDATION".to_string(),
                validator_signal.to_string(),
                "REGRESSION_EVIDENCE".to_string(),
                "VERIFIED_PASS".to_string(),
            ],
            composition_roles: vec!["INVARIANT_CHECK".to_string(), "REGRESSION_TEST".to_string()],
            learning_score: 85,
            learning_value: LearningValue::High,
            reasons: vec![
                "repository implementation lacked applicable executed pass evidence".to_string(),
                "bounded repository-native tests passed with a stable validation scope".to_string(),
                format!("test selection source={}", plan.test_selection_source),
            ],
            verification_evidence_sha256: action.execution_evidence_sha256.clone(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: action.generation,
        })
    } else if mechanism == RuntimeRepairMechanism::ValidateBlockedRepositoryCohort
        && action.executed
        && repository_sandbox_repair_verified
    {
        let plan = repository_validation_plan(config, evidence_aware_cohort)?
            .ok_or_else(|| "REPOSITORY_COHORT_VALIDATION_PLAN_LOST".to_string())?;
        let mut signals = vec![
            "AUTONOMOUS_RUNTIME_REPAIR".to_string(),
            "SOURCE_BOUND_TYPED_SYNTHESIS".to_string(),
            "PUBLIC_SYMBOL_OWNER_PRESERVED".to_string(),
            "EXECUTION_DEPENDENCY_CLOSURE_PRESERVED".to_string(),
            "SANDBOX_VERIFIED_REPAIR_CANDIDATE".to_string(),
        ];
        let mut reasons = vec![
            "repository-native tests rejected the authoritative implementation".to_string(),
            "a source-bound typed repair passed in a disposable local sandbox".to_string(),
        ];
        if repository_repair_installed {
            signals.extend([
                "AUTHORITATIVE_REPOSITORY_PATCH_INSTALLED".to_string(),
                "POST_INSTALL_PUBLIC_REGRESSION_PASS".to_string(),
            ]);
            reasons.push(
                "the exact predecessor-bound candidate was atomically installed and revalidated in the authoritative repository"
                    .to_string(),
            );
        } else {
            signals.push("CANDIDATE_NOT_INSTALLED".to_string());
            reasons.push(
                "repository mutation policy retained the candidate as generalized repair evidence without an authoritative write"
                    .to_string(),
            );
        }
        Some(LearningObservation {
            observation_id: action.output_observation_ids[0].clone(),
            work_event_id: None,
            logical_path: format!(
                "ROOT_{}/.b_repository_repair_candidate/{}",
                plan.root_index, action.output_observation_ids[0]
            ),
            content_sha256: observation_content_sha256,
            predecessor_content_sha256: None,
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: if repository_repair_installed {
                WorkOutcome::Pass
            } else {
                WorkOutcome::Unknown
            },
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals,
            composition_roles: vec![
                "PUBLIC_OBSERVATION".to_string(),
                "TYPED_COMPOSITION".to_string(),
                "ATOMIC_SOURCE_PATCH".to_string(),
                "SANDBOX_FALSIFICATION".to_string(),
            ],
            learning_score: 82,
            learning_value: LearningValue::High,
            reasons,
            verification_evidence_sha256: action.execution_evidence_sha256.clone(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: action.generation,
        })
    } else {
        None
    };
    Ok(Some((action, observation)))
}

fn persist_self_inspection(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    receipt: &AutonomousSelfInspectionReceipt,
) -> Result<(), String> {
    let receipt_sha256 = json_sha256(receipt)?;
    let path = config
        .state_dir
        .join("diagnostics")
        .join(format!("self_inspection_{}.json", receipt.diagnostic_id));
    let is_new = !path.exists();
    let mut persisted = false;
    if is_new {
        let new_policy_selection = state.diagnostic_policy.record(receipt);
        let should_persist = new_policy_selection
            || receipt.repair_disposition == RepairDisposition::RuntimeRepairActive;
        if should_persist {
            write_immutable_json(&path, receipt)?;
            persisted = true;
            state.self_inspection_events = state.self_inspection_events.saturating_add(1);
            state.diagnostic_experiment_events = state
                .diagnostic_experiment_events
                .saturating_add(receipt.experiments.len() as u64);
        }
        if new_policy_selection {
            match receipt.repair_disposition {
                RepairDisposition::RuntimeRepairActive => {}
                RepairDisposition::CapabilityGap => {
                    state.self_repair_capability_gaps =
                        state.self_repair_capability_gaps.saturating_add(1);
                }
                RepairDisposition::ProposalRequired | RepairDisposition::SafeWait => {}
            }
        }
        if should_persist {
            cleanup_recent_files(
                &config.state_dir.join("diagnostics"),
                "self_inspection_",
                64,
            )?;
        }
    }
    state.last_internal_bottleneck = Some(receipt.selected_bottleneck.label().to_string());
    if persisted || !is_new {
        state.last_self_inspection_sha256 = Some(receipt_sha256);
    }
    Ok(())
}

fn persist_runtime_repair_action(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    diagnostic: &AutonomousSelfInspectionReceipt,
    action: &RuntimeRepairActionReceipt,
) -> Result<bool, String> {
    let action_sha256 = json_sha256(action)?;
    let path = config
        .state_dir
        .join("diagnostics")
        .join(format!("runtime_repair_action_{}.json", action.action_id));
    if path.exists() {
        return Ok(false);
    }
    write_immutable_json(&path, action)?;
    if action.executed
        && state
            .diagnostic_policy
            .bind_executed_action(diagnostic, action, action_sha256)
    {
        state.runtime_self_repairs_activated =
            state.runtime_self_repairs_activated.saturating_add(1);
        cleanup_recent_files(
            &config.state_dir.join("diagnostics"),
            "runtime_repair_action_",
            64,
        )?;
        return Ok(true);
    }
    state.self_repair_capability_gaps = state.self_repair_capability_gaps.saturating_add(1);
    Ok(false)
}

fn ensure_runtime_repair_counter_contract(state: &mut SupervisorState) {
    state.diagnostic_policy.ensure_action_causal_contract();
    if state.runtime_self_repair_counter_contract_revision
        < RUNTIME_REPAIR_COUNTER_CONTRACT_REVISION
    {
        state.legacy_unbound_runtime_self_repair_activations = state
            .legacy_unbound_runtime_self_repair_activations
            .saturating_add(state.runtime_self_repairs_activated);
        state.runtime_self_repairs_activated = 0;
        state.runtime_self_repair_counter_contract_revision =
            RUNTIME_REPAIR_COUNTER_CONTRACT_REVISION;
    }
}

fn ensure_installed_execution_counter_contract(state: &mut SupervisorState) {
    if state.installed_execution_counter_contract_revision
        < INSTALLED_EXECUTION_COUNTER_CONTRACT_REVISION
    {
        state.legacy_unbound_installed_composite_execution_events = state
            .legacy_unbound_installed_composite_execution_events
            .saturating_add(state.installed_composite_capability_execution_events);
        state.legacy_unbound_installed_composite_execution_failures = state
            .legacy_unbound_installed_composite_execution_failures
            .saturating_add(state.installed_composite_capability_execution_failures);
        state.installed_composite_capability_execution_events = 0;
        state.installed_composite_capability_execution_failures = 0;
        state.last_installed_composite_execution_sha256 = None;
        state.installed_execution_counter_contract_revision =
            INSTALLED_EXECUTION_COUNTER_CONTRACT_REVISION;
    }
}

fn report_from_state(
    state: &SupervisorState,
    baseline_created: bool,
    files_scanned: usize,
    observations_created: usize,
    high_value_observations: usize,
    campaign_id: Option<String>,
    campaign_accepted: Option<bool>,
) -> StepReport {
    let (
        source_patch_recent_attempts,
        source_patch_recent_installations,
        source_patch_recent_rollbacks,
        source_patch_recent_validation_ms,
    ) = recent_source_patch_stats(state);
    let opportunity_stats = recent_source_opportunity_stats(state);
    StepReport {
        schema: SUPERVISOR_SCHEMA.to_string(),
        phase: state.phase,
        generation: state.generation,
        baseline_created,
        files_scanned,
        observations_created,
        high_value_observations,
        campaign_id,
        campaign_accepted,
        waiting_on_plateau: state.phase == SupervisorPhase::WaitingPlateau,
        stop_reason: state.stop_reason.clone(),
        current_memory_sha256: state.current_memory_sha256.clone(),
        difficulty_escalation_events: state.difficulty_escalation_events,
        codex_calls: state.codex_calls,
        external_llm_calls: state.external_llm_calls,
        network_reads: state.network_reads,
        network_writes: state.network_writes,
        last_scan_duration_ms: state.last_scan_duration_ms,
        last_scan_files_reused: state.last_scan_files_reused,
        last_scan_files_hashed: state.last_scan_files_hashed,
        scan_timeout_events: state.scan_timeout_events,
        self_inspection_events: state.self_inspection_events,
        diagnostic_experiment_events: state.diagnostic_experiment_events,
        diagnostic_policy_selections: state.diagnostic_policy.selections,
        diagnostic_policy_explorations: state.diagnostic_policy.exploration_selections,
        diagnostic_policy_causal_support_events: state.diagnostic_policy.causal_support_events,
        diagnostic_policy_outcome_bound_selections: state
            .diagnostic_policy
            .outcome_bound_selections,
        diagnostic_policy_productive_outcomes: state.diagnostic_policy.productive_outcome_events,
        diagnostic_policy_failed_outcomes: state.diagnostic_policy.failed_outcome_events,
        diagnostic_policy_duplicate_selections_suppressed: state
            .diagnostic_policy
            .duplicate_selection_suppressed,
        runtime_self_repairs_activated: state.runtime_self_repairs_activated,
        runtime_self_repair_counter_contract_revision: state
            .runtime_self_repair_counter_contract_revision,
        legacy_unbound_runtime_self_repair_activations: state
            .legacy_unbound_runtime_self_repair_activations,
        self_repair_capability_gaps: state.self_repair_capability_gaps,
        last_internal_bottleneck: state.last_internal_bottleneck.clone(),
        evaluator_generation: state.evaluator_generation,
        evaluator_challenge_cases: state.evaluator_challenge_cases,
        mutual_revalidation_events: state.mutual_revalidation_events,
        generative_predictions: state.generative_predictions,
        valuable_combinations_learned: state.valuable_combinations_learned,
        generative_memory_reuse_events: state.generative_memory_reuse_events,
        generative_self_application_events: state.generative_self_application_events,
        generative_exploration_events: state.generative_exploration_events,
        productive_generative_reuse_events: state.productive_generative_reuse_events,
        generative_frontier_advance_events: state.generative_frontier_advance_events,
        generative_frontier_capability_units: state.generative_frontier_capability_units,
        unverified_generative_frontier_candidate_events: state
            .unverified_generative_frontier_candidate_events,
        legacy_unverified_generative_frontier_advance_events: state
            .legacy_unverified_generative_frontier_advance_events,
        legacy_wrapper_generative_frontier_advance_events: state
            .legacy_wrapper_generative_frontier_advance_events,
        generative_behavioral_verification_events: state.generative_behavioral_verification_events,
        redundant_generative_selection_events: state.redundant_generative_selection_events,
        generative_mean_prediction_error_millis: state
            .generative_prediction_absolute_error_total
            .saturating_mul(1_000)
            .checked_div(state.generative_calibrated_prediction_records.max(1))
            .unwrap_or(0),
        generative_calibrated_prediction_records: state.generative_calibrated_prediction_records,
        generative_legacy_uncalibrated_prediction_error_total: state
            .generative_legacy_uncalibrated_prediction_error_total,
        autonomous_source_patch_attempts: state.autonomous_source_patch_attempts,
        autonomous_source_patches_installed: state.autonomous_source_patches_installed,
        autonomous_source_patch_rollbacks: state.autonomous_source_patch_rollbacks,
        autonomous_source_patch_validation_ms: state.autonomous_source_patch_validation_ms,
        source_patch_recent_attempts,
        source_patch_recent_installations,
        source_patch_recent_rollbacks,
        source_patch_recent_validation_ms,
        source_patch_recent_distinct_opportunity_families: opportunity_stats.distinct_total,
        source_patch_recent_defect_families: opportunity_stats.defects,
        source_patch_recent_capability_gap_families: opportunity_stats.capability_gaps,
        source_patch_recent_efficiency_opportunity_families: opportunity_stats
            .efficiency_opportunities,
        source_patch_recent_robustness_opportunity_families: opportunity_stats
            .robustness_opportunities,
        source_patch_recent_research_hypothesis_families: opportunity_stats.research_hypotheses,
        source_patch_recent_verified_improvements: opportunity_stats.verified_improvements,
        source_discovery_no_candidate_streak: state.source_discovery_no_candidate_streak,
        last_source_discovery_reason: state.last_source_discovery_reason.clone(),
        source_discovery_duplicate_states_suppressed: state
            .source_discovery_duplicate_states_suppressed,
        source_patch_consecutive_failures: state.source_patch_consecutive_failures,
        last_source_patch_receipt_sha256: state.last_source_patch_receipt_sha256.clone(),
        composite_capability_install_attempts: state.composite_capability_install_attempts,
        composite_capabilities_installed: state.composite_capabilities_installed,
        composite_capability_install_rollbacks: state.composite_capability_install_rollbacks,
        composite_capability_consecutive_failures: state.composite_capability_consecutive_failures,
        last_composite_candidate_sha256: state.last_composite_candidate_sha256.clone(),
        installed_composite_capability_execution_events: state
            .installed_composite_capability_execution_events,
        installed_composite_capability_execution_failures: state
            .installed_composite_capability_execution_failures,
        last_installed_composite_execution_sha256: state
            .last_installed_composite_execution_sha256
            .clone(),
        installed_context_bound_capabilities_validated: state
            .installed_context_bound_capabilities_validated,
        last_installed_capability_inventory_sha256: state
            .last_installed_capability_inventory_sha256
            .clone(),
        installed_capability_continuation_observations: state
            .installed_capability_continuation_observations,
        installed_execution_counter_contract_revision: state
            .installed_execution_counter_contract_revision,
        legacy_unbound_installed_composite_execution_events: state
            .legacy_unbound_installed_composite_execution_events,
        legacy_unbound_installed_composite_execution_failures: state
            .legacy_unbound_installed_composite_execution_failures,
        distinct_semantic_lessons: state.distinct_semantic_lessons,
        semantic_duplicate_lessons: state.semantic_duplicate_lessons,
        semantic_revalidation_events: state.semantic_revalidation_events,
        redundant_observations_consumed: state.redundant_observations_consumed,
        measured_performance_promotions: state.measured_performance_promotions,
        classifier_outcome_bound_refinements: state.classifier_outcome_bound_refinements,
        classifier_unsupported_refinements_suppressed: state
            .classifier_unsupported_refinements_suppressed,
        intrinsic_curiosity_hypotheses_attempted: state.intrinsic_drive.hypotheses_attempted,
        intrinsic_curiosity_hypotheses_succeeded: state.intrinsic_drive.hypotheses_succeeded,
        intrinsic_curiosity_hypotheses_failed: state.intrinsic_drive.hypotheses_failed,
        intrinsic_curiosity_hypotheses_pending: state.intrinsic_drive.pending_attempts.len(),
        intrinsic_reward_events: state.intrinsic_drive.intrinsic_reward_events,
        intrinsic_reward_total: state.intrinsic_drive.intrinsic_reward_total,
        intrinsic_reward_contract_revision: state.intrinsic_drive.reward_contract_revision,
        legacy_precommit_intrinsic_reward_events: state
            .intrinsic_drive
            .legacy_precommit_reward_events,
        legacy_precommit_intrinsic_reward_total: state
            .intrinsic_drive
            .legacy_precommit_reward_total,
        current_curiosity: state.intrinsic_drive.current_curiosity,
        verified_satisfaction: state.intrinsic_drive.verified_satisfaction,
        last_intrinsic_hypothesis_id: state.intrinsic_drive.last_hypothesis_id.clone(),
        last_intrinsic_reward: state.intrinsic_drive.last_reward,
    }
}

fn reconcile_source_patch_validation_cost(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
) -> Result<(), String> {
    if state.autonomous_source_patch_validation_ms > 0
        || state.autonomous_source_patch_attempts == 0
    {
        return Ok(());
    }
    let root = config.state_dir.join("source_mutations");
    let mut total = 0_u64;
    for entry in
        fs::read_dir(&root).map_err(|error| format!("SOURCE_MUTATION_METRICS_DIR:{error}"))?
    {
        let entry = entry.map_err(|error| format!("SOURCE_MUTATION_METRICS_ENTRY:{error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("SOURCE_MUTATION_METRICS_TYPE:{error}"))?
            .is_dir()
        {
            continue;
        }
        let path = entry.path().join("receipt.json");
        if path.is_file() {
            let receipt: AutonomousSourcePatchReceipt = read_json(&path)?;
            total = total.saturating_add(source_patch_validation_critical_path_ms(&receipt));
        }
    }
    state.autonomous_source_patch_validation_ms = total;
    Ok(())
}

fn ensure_source_patch_telemetry_epoch(state: &mut SupervisorState) {
    if state.source_patch_telemetry_engine_revision != SOURCE_REPAIR_ENGINE_REVISION
        || state.source_patch_validation_contract_revision
            != SOURCE_PATCH_VALIDATION_CONTRACT_REVISION
    {
        state.source_patch_recent_outcomes.clear();
        state.source_patch_telemetry_engine_revision = SOURCE_REPAIR_ENGINE_REVISION;
        state.source_patch_validation_contract_revision = SOURCE_PATCH_VALIDATION_CONTRACT_REVISION;
        state.source_patch_consecutive_failures = 0;
        state.source_discovery_no_candidate_streak = 0;
        state.last_source_discovery_reason = None;
    }
}

fn push_source_patch_outcome(state: &mut SupervisorState, sample: SourcePatchOutcomeSample) {
    ensure_source_patch_telemetry_epoch(state);
    state.source_patch_recent_outcomes.push(sample);
    while state.source_patch_recent_outcomes.len() > MAX_RECENT_SOURCE_PATCH_OUTCOMES {
        state.source_patch_recent_outcomes.remove(0);
    }
}

fn recent_source_patch_stats(state: &SupervisorState) -> (u64, u64, u64, u64) {
    let current = state
        .source_patch_recent_outcomes
        .iter()
        .filter(|sample| sample.engine_revision == SOURCE_REPAIR_ENGINE_REVISION)
        .collect::<Vec<_>>();
    let attempts = current.len().min(u64::MAX as usize) as u64;
    let installations = current.iter().filter(|sample| sample.installed).count() as u64;
    let rollbacks = current.iter().filter(|sample| sample.rolled_back).count() as u64;
    let validation_ms = current.iter().fold(0_u64, |total, sample| {
        total.saturating_add(sample.validation_ms)
    });
    (attempts, installations, rollbacks, validation_ms)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct RecentSourceOpportunityStats {
    distinct_total: u64,
    defects: u64,
    capability_gaps: u64,
    efficiency_opportunities: u64,
    robustness_opportunities: u64,
    research_hypotheses: u64,
    verified_improvements: u64,
}

fn recent_source_opportunity_stats(state: &SupervisorState) -> RecentSourceOpportunityStats {
    let mut all = BTreeSet::new();
    let mut by_kind: BTreeMap<ChangeOpportunityKind, BTreeSet<String>> = BTreeMap::new();
    let mut verified = BTreeSet::new();
    for sample in state
        .source_patch_recent_outcomes
        .iter()
        .filter(|sample| sample.engine_revision == SOURCE_REPAIR_ENGINE_REVISION)
    {
        if sample.opportunity_family_id.is_empty() {
            continue;
        }
        all.insert(sample.opportunity_family_id.clone());
        by_kind
            .entry(sample.opportunity_kind)
            .or_default()
            .insert(sample.opportunity_family_id.clone());
        if sample.installed {
            verified.insert(sample.opportunity_family_id.clone());
        }
    }
    let count = |kind| {
        by_kind
            .get(&kind)
            .map_or(0, |families| families.len().min(u64::MAX as usize) as u64)
    };
    RecentSourceOpportunityStats {
        distinct_total: all.len().min(u64::MAX as usize) as u64,
        defects: count(ChangeOpportunityKind::Defect),
        capability_gaps: count(ChangeOpportunityKind::CapabilityGap),
        efficiency_opportunities: count(ChangeOpportunityKind::EfficiencyOpportunity),
        robustness_opportunities: count(ChangeOpportunityKind::RobustnessOpportunity),
        research_hypotheses: count(ChangeOpportunityKind::ResearchHypothesis),
        verified_improvements: verified.len().min(u64::MAX as usize) as u64,
    }
}

fn account_source_patch_receipt(
    state: &mut SupervisorState,
    receipt: &AutonomousSourcePatchReceipt,
) {
    state.last_source_patch_receipt_sha256 = Some(receipt.receipt_sha256.clone());
    let validation_ms = source_patch_validation_critical_path_ms(receipt);
    state.autonomous_source_patch_validation_ms = state
        .autonomous_source_patch_validation_ms
        .saturating_add(validation_ms);
    if source_patch_failure_is_transient(receipt.failure_reason.as_deref()) {
        state.last_source_discovery_reason =
            Some("TRANSIENT_WORKSPACE_CONTENTION_DEFERRED".to_string());
        return;
    }
    push_source_patch_outcome(
        state,
        SourcePatchOutcomeSample {
            engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
            installed: receipt.installed,
            rolled_back: receipt.rolled_back,
            validation_ms,
            opportunity_kind: receipt.opportunity_kind,
            opportunity_family_id: receipt.opportunity_family_id.clone(),
        },
    );
    if receipt.installed {
        state.autonomous_source_patches_installed =
            state.autonomous_source_patches_installed.saturating_add(1);
        state.source_patch_consecutive_failures = 0;
    } else if receipt.rolled_back {
        state.autonomous_source_patch_rollbacks =
            state.autonomous_source_patch_rollbacks.saturating_add(1);
        state.source_patch_consecutive_failures =
            state.source_patch_consecutive_failures.saturating_add(1);
    }
}

fn account_source_patch_error(
    state: &mut SupervisorState,
    opportunity_kind: ChangeOpportunityKind,
    opportunity_family_id: String,
) {
    push_source_patch_outcome(
        state,
        SourcePatchOutcomeSample {
            engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
            installed: false,
            rolled_back: true,
            validation_ms: 0,
            opportunity_kind,
            opportunity_family_id,
        },
    );
    state.autonomous_source_patch_rollbacks =
        state.autonomous_source_patch_rollbacks.saturating_add(1);
    state.source_patch_consecutive_failures =
        state.source_patch_consecutive_failures.saturating_add(1);
}

fn is_sem5_composition(accepted: &crate::generative_growth::ReusableCompositionMemory) -> bool {
    accepted.successful_uses > 0
        && accepted.has_executable_composer(GenerativeComposerIR::Sem5Program)
}

fn accepted_sem5_executable_artifacts(
    memory: &GrowthMemory,
) -> Vec<(
    String,
    String,
    TypedMechanismSynthesisGoalIR,
    Option<TypedMechanismSynthesisReceiptIR>,
)> {
    let mut artifacts =
        memory
            .generative
            .accepted_compositions
            .iter()
            .rev()
            .filter(|accepted| is_sem5_composition(accepted))
            .flat_map(|accepted| {
                accepted.verified_artifact_contexts.iter().rev().filter_map(
                    |(artifact, context)| {
                        if !accepted.has_executable_artifact(artifact) {
                            return None;
                        }
                        let goal = accepted.verified_typed_behavior_goals.get(artifact)?;
                        let synthesis_receipt = accepted
                            .verified_typed_mechanism_receipts
                            .get(artifact)
                            .cloned();
                        Some((
                            artifact.clone(),
                            context.clone(),
                            goal.clone(),
                            synthesis_receipt,
                        ))
                    },
                )
            })
            .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    artifacts.retain(|(artifact, _, _, _)| seen.insert(artifact.clone()));
    artifacts
}

fn accepted_sem5_artifact_contexts(memory: &GrowthMemory) -> Vec<(String, String)> {
    accepted_sem5_executable_artifacts(memory)
        .into_iter()
        .map(|(artifact, context, _, _)| (artifact, context))
        .collect()
}

fn reconcile_verified_generative_typed_operators(
    config: &GrowthSupervisorConfig,
    memory: &GrowthMemory,
) -> Result<usize, String> {
    let memory_sha256 = json_sha256(memory)?;
    let mut authorized_ids = load_source_bound_improvement_operators(config)?
        .into_iter()
        .map(|operator| operator.operator_id)
        .collect::<BTreeSet<_>>();
    let mut promoted = 0_usize;
    let mut seen = BTreeSet::new();
    for (artifact_sha256, _, goal, synthesis_receipt) in accepted_sem5_executable_artifacts(memory)
        .into_iter()
        .take(MAX_ACTIVE_SOURCE_BOUND_IMPROVEMENT_OPERATORS)
    {
        let Some(synthesis_receipt) = synthesis_receipt else {
            // Revision-6 goal-only memories remain reconstructable through
            // the legacy callable cache, but cannot mint new authority.
            continue;
        };
        if synthesis_receipt.synthesis_request.as_ref() != Some(&goal)
            || synthesis_receipt.receipt_sha256.len() != 64
            || artifact_sha256.len() != 64
        {
            return Err("GENERATIVE_TYPED_OPERATOR_PROMOTION_BINDING_FAILURE".to_string());
        }
        let execution_output_sha256 = sha256(
            format!(
                "VERIFIED_GENERATIVE_TYPED_OPERATOR_OUTPUT_1:{memory_sha256}:{artifact_sha256}:{}",
                synthesis_receipt.receipt_sha256
            )
            .as_bytes(),
        );
        let operator = typed_mechanism_improvement_operator_from_receipt(
            &synthesis_receipt,
            execution_output_sha256.clone(),
        )?;
        if !seen.insert(operator.operator_id.clone())
            || authorized_ids.contains(&operator.operator_id)
        {
            continue;
        }
        let repair_id = sha256(
            format!(
                "VERIFIED_GENERATIVE_TYPED_OPERATOR_1:{}:{artifact_sha256}:{memory_sha256}",
                memory.generation
            )
            .as_bytes(),
        );
        persist_authorized_typed_mechanism_operator(
            &config.state_dir,
            &operator,
            &TypedMechanismOperatorPromotionEvidenceIR {
                repair_id,
                repair_receipt_sha256: memory_sha256.clone(),
                execution_output_sha256,
                candidate_sha256: artifact_sha256,
                sandbox_verified: true,
                sandbox_cleaned: true,
                authoritative_scope_stable: true,
                candidate_installed: false,
                authoritative_source_write_events: 0,
                codex_calls: 0,
                external_llm_calls: 0,
                network_reads: 0,
                network_writes: 0,
                promotion_generation: memory.generation,
            },
        )?;
        authorized_ids.insert(operator.operator_id);
        promoted = promoted.saturating_add(1);
    }
    Ok(promoted)
}

fn pending_sem5_composition_candidates(
    config: &GrowthSupervisorConfig,
    memory: &GrowthMemory,
) -> Result<Vec<crate::integrated_development::CompositeProgramCandidateIR>, String> {
    let installed = crate::generated_sem5_capability::generated_capability_hashes()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let authorized_operator_ids = load_source_bound_improvement_operators(config)?
        .into_iter()
        .map(|operator| operator.operator_id)
        .collect::<BTreeSet<_>>();
    let family_limit = MAX_COMPOSITE_INSTALL_FAMILY;
    let mut candidates = Vec::new();
    for (expected_artifact, context, goal, expected_synthesis) in
        accepted_sem5_executable_artifacts(memory)
    {
        if installed.contains(expected_artifact.as_str()) {
            continue;
        }
        if let Some(synthesis) = expected_synthesis.as_ref() {
            let operator = typed_mechanism_improvement_operator_from_receipt(
                synthesis,
                synthesis.receipt_sha256.clone(),
            )?;
            if authorized_operator_ids.contains(&operator.operator_id) {
                continue;
            }
        } else if installed.len().saturating_add(candidates.len())
            >= MAX_INSTALLED_TYPED_CAPABILITIES
        {
            // Only legacy goal-only memories still require the bounded static
            // callable cache. Exact typed recipes use the dynamic repository.
            continue;
        }
        let (candidate, _) = compose_typed_behavior_goal_candidate(&context, &goal)?;
        if candidate.program_ir_sha256 != expected_artifact {
            return Err("VERIFIED_ARTIFACT_CONTEXT_BINDING_FAILURE".to_string());
        }
        if expected_synthesis.as_ref().is_some_and(|expected| {
            candidate.typed_mechanism_synthesis_receipt.as_ref() != Some(expected)
        }) {
            return Err("VERIFIED_ARTIFACT_SYNTHESIS_RECEIPT_BINDING_FAILURE".to_string());
        }
        candidates.push(candidate);
        if candidates.len() >= family_limit {
            break;
        }
    }
    Ok(candidates)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InstalledCapabilityInventoryExecutionReceipt {
    schema: String,
    installed_registry_capability_count: usize,
    context_bound_capability_count: usize,
    inventory_sha256: String,
    artifact_sha256: Vec<String>,
    canary_receipt_sha256: Vec<String>,
    failure_classes: Vec<String>,
    pass: bool,
    receipt_sha256: String,
}

fn revalidate_installed_composite_capability(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    memory: &GrowthMemory,
) -> Result<Option<LearningObservation>, String> {
    if !crate::generated_sem5_capability::GENERATED_CAPABILITY_ACTIVE {
        return Ok(None);
    }
    let installed = crate::generated_sem5_capability::generated_capability_hashes()
        .iter()
        .map(|hash| (*hash).to_string())
        .collect::<BTreeSet<_>>();
    let mut context_bound = accepted_sem5_artifact_contexts(memory)
        .into_iter()
        .filter(|(artifact, _)| installed.contains(artifact))
        .collect::<Vec<_>>();
    context_bound.sort();
    if context_bound.is_empty() {
        return Ok(None);
    }

    let installed_hashes = installed.iter().cloned().collect::<Vec<_>>();
    let inventory_sha256 = json_sha256(&(
        "INSTALLED_CONTEXT_BOUND_CAPABILITY_INVENTORY_1",
        crate::generated_sem5_capability::GENERATED_SOURCE_SCHEMA_REVISION,
        &installed_hashes,
        &context_bound,
    ))?;
    let mut canary_receipt_sha256 = Vec::new();
    let mut failure_classes = Vec::new();
    for (artifact, context) in &context_bound {
        match execute_behavioral_composition_canary(context) {
            Ok(receipt) => {
                let pass = receipt.context_sha256 == *context
                    && receipt.program_ir_sha256 == *artifact
                    && receipt.installed_capability_present
                    && receipt.installed_program_match
                    && receipt.installed_cases_executed == receipt.cases_executed
                    && receipt.installed_cases_passed == receipt.cases_passed
                    && receipt.installed_output_sha256.is_some();
                canary_receipt_sha256.push(receipt.receipt_sha256);
                if !pass {
                    failure_classes.push(format!("INSTALLED_CANARY_MISMATCH:{artifact}"));
                }
            }
            Err(error) => {
                failure_classes.push(format!(
                    "INSTALLED_CANARY_ERROR:{artifact}:{}",
                    error.split(':').next().unwrap_or("UNKNOWN")
                ));
            }
        }
    }
    let pass = failure_classes.is_empty() && canary_receipt_sha256.len() == context_bound.len();
    let receipt_sha256 = json_sha256(&(
        "B_CORE_INSTALLED_CAPABILITY_INVENTORY_EXECUTION_1",
        crate::generated_sem5_capability::GENERATED_CAPABILITY_COUNT,
        context_bound.len(),
        &inventory_sha256,
        &installed_hashes,
        &canary_receipt_sha256,
        &failure_classes,
        pass,
    ))?;
    let receipt = InstalledCapabilityInventoryExecutionReceipt {
        schema: "B_CORE_INSTALLED_CAPABILITY_INVENTORY_EXECUTION_1".to_string(),
        installed_registry_capability_count:
            crate::generated_sem5_capability::GENERATED_CAPABILITY_COUNT,
        context_bound_capability_count: context_bound.len(),
        inventory_sha256: inventory_sha256.clone(),
        artifact_sha256: context_bound
            .iter()
            .map(|(artifact, _)| artifact.clone())
            .collect(),
        canary_receipt_sha256,
        failure_classes,
        pass,
        receipt_sha256: receipt_sha256.clone(),
    };
    if state.last_installed_composite_execution_sha256.as_deref() == Some(receipt_sha256.as_str()) {
        return Ok(None);
    }
    let diagnostics = config.state_dir.join("diagnostics");
    fs::create_dir_all(&diagnostics)
        .map_err(|error| format!("INSTALLED_CAPABILITY_DIAGNOSTICS_CREATE:{error}"))?;
    write_immutable_json(
        &diagnostics.join(format!(
            "installed_capability_inventory_{receipt_sha256}.json"
        )),
        &receipt,
    )?;
    cleanup_recent_files(&diagnostics, "installed_capability_inventory_", 8)?;

    let previous_validated = state.installed_context_bound_capabilities_validated;
    state.last_installed_capability_inventory_sha256 = Some(inventory_sha256.clone());
    state.last_installed_composite_execution_sha256 = Some(receipt_sha256.clone());
    state.installed_composite_capability_execution_events = state
        .installed_composite_capability_execution_events
        .saturating_add(1);
    if !pass {
        state.installed_composite_capability_execution_failures = state
            .installed_composite_capability_execution_failures
            .saturating_add(1);
        state.self_repair_capability_gaps = state.self_repair_capability_gaps.saturating_add(1);
        return Ok(None);
    }
    let validated = context_bound.len().min(u64::MAX as usize) as u64;
    state.installed_context_bound_capabilities_validated = validated;
    if validated <= previous_validated {
        return Ok(None);
    }
    state.installed_capability_continuation_observations = state
        .installed_capability_continuation_observations
        .saturating_add(1);
    let observation_id = sha256(
        format!(
            "INSTALLED_CAPABILITY_CONTINUATION:{}:{}:{}",
            previous_validated, validated, receipt_sha256
        )
        .as_bytes(),
    );
    let source_prefix = source_mutation_watch_prefix(config)?
        .ok_or_else(|| "INSTALLED_CAPABILITY_SOURCE_ROOT_NOT_WATCHED".to_string())?;
    Ok(Some(LearningObservation {
        observation_id: observation_id.clone(),
        work_event_id: None,
        logical_path: format!(
            "{source_prefix}.b_installed_capability_inventory/{observation_id}"
        ),
        content_sha256: receipt_sha256.clone(),
        predecessor_content_sha256: None,
        actor: WorkActor::LocalTool,
        work_kind: WorkKind::CodeChange,
        work_outcome: WorkOutcome::Pass,
        features_before: None,
        features_after: StructuralFeatures::default(),
        signals: vec![
            "BEHAVIORAL_FRONTIER_ADVANCE".to_string(),
            "INSTALLED_COMPOSITE_CAPABILITY_INVENTORY_EXECUTION".to_string(),
            "VERIFIED_PASS".to_string(),
        ],
        composition_roles: vec![
            "IMPLEMENTATION".to_string(),
            "INVARIANT_CHECK".to_string(),
            "PROGRAM_COMPOSITION".to_string(),
            "REGRESSION_TEST".to_string(),
        ],
        learning_score: 95,
        learning_value: LearningValue::High,
        reasons: vec![
            "every context-bound installed callable passed fresh deterministic property cases"
                .to_string(),
            "a larger executable capability inventory is a measured growth subject, not a repeated source-shape lesson"
                .to_string(),
        ],
        verification_evidence_sha256: vec![receipt_sha256.clone(), inventory_sha256],
        performance_metrics: vec![PerformanceMetricEvidence {
            metric: "INSTALLED_CONTEXT_BOUND_CAPABILITY_COUNT".to_string(),
            before: previous_validated,
            after: validated,
            lower_is_better: false,
            evidence_sha256: receipt_sha256,
            executable_knowledge: None,
        }],
        public_contract_deltas: Vec::new(),
        exact_source_fragments_stored: 0,
        raw_source_bytes_stored: 0,
        observed_at_ms: state.last_transition_ms.saturating_add(1),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompositeInstallAttemptOutcome {
    attempted: bool,
    staged: bool,
}

fn attempt_pending_composite_capability_install(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    memory: &GrowthMemory,
) -> Result<CompositeInstallAttemptOutcome, String> {
    let candidates = pending_sem5_composition_candidates(config, memory)?;
    if candidates.is_empty() {
        return Ok(CompositeInstallAttemptOutcome {
            attempted: false,
            staged: false,
        });
    }
    let family_identity = sha256(
        format!(
            "COMPOSITE_INSTALL_FAMILY_2:{}:{}",
            SOURCE_REPAIR_ENGINE_REVISION,
            candidates
                .iter()
                .map(|candidate| candidate.generated_rust_sha256.as_str())
                .collect::<Vec<_>>()
                .join(":")
        )
        .as_bytes(),
    );
    if state.last_composite_candidate_sha256.as_deref() != Some(family_identity.as_str()) {
        state.last_composite_candidate_sha256 = Some(family_identity);
        state.composite_capability_consecutive_failures = 0;
    }
    if state.composite_capability_consecutive_failures
        >= u32::from(config.source_mutation.max_attempts_per_problem)
    {
        return Ok(CompositeInstallAttemptOutcome {
            attempted: false,
            staged: false,
        });
    }
    state.composite_capability_install_attempts = state
        .composite_capability_install_attempts
        .saturating_add(1);
    state.last_source_discovery_reason = Some(format!(
        "VERIFIED_COMPOSITE_CAPABILITY_FAMILY_PENDING_INSTALL:{}",
        candidates.len()
    ));
    state.autonomous_source_patch_attempts =
        state.autonomous_source_patch_attempts.saturating_add(1);
    let receipt = match install_composite_candidate_family(
        &candidates,
        &config.source_mutation,
        &config.state_dir,
        state.generation,
        state.composite_capability_install_attempts,
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            let error_class = error.split(':').next().unwrap_or("UNKNOWN");
            state.last_source_discovery_reason =
                Some(format!("COMPOSITE_CAPABILITY_INSTALL_ERROR:{error_class}"));
            if source_patch_failure_is_transient(Some(error_class)) {
                return Ok(CompositeInstallAttemptOutcome {
                    attempted: true,
                    staged: false,
                });
            }
            account_source_patch_error(
                state,
                ChangeOpportunityKind::CapabilityGap,
                source_opportunity_family_id(
                    ChangeOpportunityKind::CapabilityGap,
                    "SEM5_PROGRAM_IR_TO_ACTIVE_RUNTIME_CALLABLE",
                ),
            );
            state.composite_capability_install_rollbacks = state
                .composite_capability_install_rollbacks
                .saturating_add(1);
            state.composite_capability_consecutive_failures = state
                .composite_capability_consecutive_failures
                .saturating_add(1);
            return Ok(CompositeInstallAttemptOutcome {
                attempted: true,
                staged: false,
            });
        }
    };
    let transient = source_patch_failure_is_transient(receipt.failure_reason.as_deref());
    account_source_patch_receipt(state, &receipt);
    if transient {
        return Ok(CompositeInstallAttemptOutcome {
            attempted: true,
            staged: false,
        });
    }
    if receipt.installed {
        state.last_source_discovery_reason = Some(format!(
            "COMPOSITE_CAPABILITY_FAMILY_INSTALLED_AND_STAGED:{}",
            candidates.len()
        ));
        state.composite_capabilities_installed = state
            .composite_capabilities_installed
            .saturating_add(candidates.len() as u64);
        state.composite_capability_consecutive_failures = 0;
    } else if receipt.rolled_back {
        state.last_source_discovery_reason = Some(format!(
            "COMPOSITE_CAPABILITY_ROLLED_BACK:{}",
            receipt.failure_reason.as_deref().unwrap_or("UNKNOWN")
        ));
        state.composite_capability_install_rollbacks = state
            .composite_capability_install_rollbacks
            .saturating_add(1);
        state.composite_capability_consecutive_failures = state
            .composite_capability_consecutive_failures
            .saturating_add(1);
    }
    Ok(CompositeInstallAttemptOutcome {
        attempted: true,
        staged: receipt.installed && receipt.runtime_update_staged,
    })
}

fn source_discovery_state_sha256(
    config: &GrowthSupervisorConfig,
    state: &SupervisorState,
    index: &FileIndex,
) -> Result<String, String> {
    let prefix = source_mutation_watch_prefix(config)?
        .ok_or_else(|| "SOURCE_DISCOVERY_ROOT_NOT_DIRECTLY_WATCHED".to_string())?;
    let content_identity = index
        .files
        .iter()
        .filter(|(logical_path, _)| logical_path.starts_with(&prefix))
        .map(|(logical_path, fingerprint)| {
            (logical_path.clone(), fingerprint.content_sha256.clone())
        })
        .collect::<BTreeMap<_, _>>();
    let mut repository_policy = config.source_mutation.clone();
    repository_policy.auto_discover_known_transformations = false;
    json_sha256(&(
        SOURCE_REPAIR_ENGINE_REVISION,
        state.generation,
        repository_policy,
        content_identity,
    ))
}

fn discover_next_source_repair(
    config: &GrowthSupervisorConfig,
    state: &SupervisorState,
    memory: &GrowthMemory,
) -> Result<SourceDiscoveryResult, String> {
    let executable_performance_knowledge = memory
        .lessons
        .iter()
        .flat_map(executable_performance_operators)
        .fold(BTreeMap::new(), |mut operators, operator| {
            operators
                .entry(operator.operator_id.clone())
                .or_insert(operator);
            operators
        })
        .into_values()
        .collect::<Vec<_>>();
    if executable_performance_knowledge.is_empty() {
        discover_repository_improvement_detailed(
            &config.source_mutation,
            &config.state_dir,
            state.generation,
        )
    } else {
        match discover_executable_performance_improvement(
            &config.source_mutation,
            &config.state_dir,
            state.generation,
            &executable_performance_knowledge,
        ) {
            Ok(learned) if learned.candidate.is_some() => Ok(learned),
            Ok(_) => discover_repository_improvement_detailed(
                &config.source_mutation,
                &config.state_dir,
                state.generation,
            ),
            Err(error) => Err(error),
        }
    }
}

fn source_patch_revision_dimensions(request: &AutonomousSourcePatchRequest) -> BTreeSet<String> {
    let mut dimensions = BTreeSet::from([
        format!("strategy:{}", request.solution_strategy),
        format!("candidate:{}", request.candidate_sha256),
    ]);
    if let Some(change) = &request.generalized_change {
        dimensions.extend(
            change
                .derived_from_counterexample_ids
                .iter()
                .map(|id| format!("counterexample:{id}")),
        );
        dimensions.extend(
            change
                .operations
                .iter()
                .map(|operation| format!("operation:{operation:?}")),
        );
    }
    dimensions
}

fn same_attempt_failure_requirement(counterexample_id: &str) -> SameAttemptCounterexample {
    SameAttemptCounterexample::new(
        counterexample_id,
        [format!("counterexample:{counterexample_id}")],
    )
}

fn attempt_discovered_source_repair(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    memory: &GrowthMemory,
    index: &FileIndex,
) -> Result<bool, String> {
    if !config.source_mutation.enabled
        || state.autonomous_source_patches_installed >= config.source_mutation.max_installations
    {
        return Ok(false);
    }
    // A verified composition family is already a concrete repair action, not a
    // new discovery hypothesis.  Requiring another full plateau window here
    // made every successful self-application idle for N scans before it could
    // become executable source.  Install the already verified work at the next
    // safe cycle; retain the plateau gate only for fresh source discovery.
    reconcile_verified_generative_typed_operators(config, memory)?;
    let composite = attempt_pending_composite_capability_install(config, state, memory)?;
    if composite.attempted {
        if composite.staged {
            state.stop_reason = Some("AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED".to_string());
        }
        return Ok(composite.staged);
    }
    if state.plateau_scans < config.resources.plateau_scans_before_wait {
        return Ok(false);
    }
    if !config.source_mutation.auto_discover_compiler_repairs
        && !config.source_mutation.auto_synthesize_grammar_repairs
    {
        return Ok(false);
    }
    let discovery_state_sha256 = source_discovery_state_sha256(config, state, index)?;
    if state.last_source_discovery_state_sha256.as_deref() == Some(discovery_state_sha256.as_str())
    {
        state.source_discovery_duplicate_states_suppressed = state
            .source_discovery_duplicate_states_suppressed
            .saturating_add(1);
        return Ok(false);
    }
    let configured_limit = usize::from(config.source_mutation.max_attempts_per_problem);
    let mut revision =
        SameAttemptRevisionTracker::new(configured_limit.min(MAX_SAME_ATTEMPT_EXECUTIONS));
    loop {
        let discovery = discover_next_source_repair(config, state, memory);
        match discovery {
            Ok(discovery) if discovery.candidate.is_some() => {
                let Some(request) = discovery.candidate else {
                    break;
                };
                let opportunity_kind = request.opportunity_kind;
                let opportunity_family_id = request.opportunity_family_id.clone();
                let revision_dimensions = source_patch_revision_dimensions(&request);
                match revision.admit_candidate(&request.candidate_sha256, &revision_dimensions) {
                    CandidateAdmission::Execute => {}
                    disposition => {
                        state.last_source_discovery_state_sha256 = Some(discovery_state_sha256);
                        state.last_source_discovery_reason = Some(format!(
                            "SAME_ATTEMPT_REVISION_STOP:{disposition:?}:EXECUTED={}",
                            revision.metrics().candidates_admitted
                        ));
                        break;
                    }
                }
                state.last_source_discovery_state_sha256 = None;
                state.source_discovery_no_candidate_streak = 0;
                state.last_source_discovery_reason = Some(format!(
                    "{}:SAME_ATTEMPT_EXECUTION={}",
                    discovery.disposition.label(),
                    revision.metrics().candidates_admitted
                ));
                state.autonomous_source_patch_attempts =
                    state.autonomous_source_patch_attempts.saturating_add(1);
                match install_and_stage_source_patch(
                    &config.source_mutation,
                    &config.state_dir,
                    &request,
                ) {
                    Ok(receipt) => {
                        account_source_patch_receipt(state, &receipt);
                        if receipt.installed && receipt.runtime_update_staged {
                            state.stop_reason = Some("AUTONOMOUS_SOURCE_UPDATE_STAGED".to_string());
                            return Ok(true);
                        }
                        if receipt.installed
                            || source_patch_failure_is_transient(receipt.failure_reason.as_deref())
                        {
                            break;
                        }
                        let exact_rollback =
                            receipt.rolled_back && receipt.workspace_stable_during_validation;
                        let Some(counterexample) = counterexample_from_receipt(&request, &receipt)
                        else {
                            break;
                        };
                        let requirement =
                            same_attempt_failure_requirement(&counterexample.counterexample_id);
                        if !revision.observe_failure(requirement, exact_rollback) {
                            break;
                        }
                        state.last_source_discovery_reason = Some(format!(
                            "SAME_ATTEMPT_COUNTEREXAMPLE_REVISION:{}:{}",
                            revision.metrics().candidates_admitted,
                            counterexample.counterexample_id
                        ));
                    }
                    Err(_) => {
                        account_source_patch_error(state, opportunity_kind, opportunity_family_id);
                        break;
                    }
                }
            }
            Ok(discovery) => {
                state.last_source_discovery_state_sha256 = Some(discovery_state_sha256);
                state.source_discovery_no_candidate_streak =
                    state.source_discovery_no_candidate_streak.saturating_add(1);
                state.last_source_discovery_reason =
                    Some(discovery.disposition.label().to_string());
                break;
            }
            Err(_) => {
                state.last_source_discovery_state_sha256 = None;
                state.last_source_discovery_reason = Some("DISCOVERY_ERROR".to_string());
                state.self_repair_capability_gaps =
                    state.self_repair_capability_gaps.saturating_add(1);
                break;
            }
        }
    }
    Ok(false)
}

fn stop_if_requested(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
) -> Result<bool, String> {
    if config.state_dir.join("control").join("STOP").exists() {
        state.stop_reason = Some("OPERATOR_STOP_REQUESTED".to_string());
        save_transition(
            config,
            state,
            SupervisorPhase::SafeStopped,
            "OPERATOR_STOP_REQUEST_OBSERVED_AT_SAFE_BOUNDARY",
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn step_without_lease(
    config: &GrowthSupervisorConfig,
    lease: &SupervisorLease,
) -> Result<StepReport, String> {
    let started = Instant::now();
    let mut state = load_state(config)?;
    ensure_runtime_repair_counter_contract(&mut state);
    ensure_installed_execution_counter_contract(&mut state);
    ensure_source_patch_telemetry_epoch(&mut state);
    recover_verification_only_generation_tip(config, &mut state)?;
    if stop_if_requested(config, &mut state)? {
        return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
    }
    if state.phase == SupervisorPhase::SafeStopped {
        return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
    }
    reconcile_source_patch_validation_cost(config, &mut state)?;
    if let Some(reason) = resource_stop_reason(config, &state)? {
        state.stop_reason = Some(reason);
        save_transition(
            config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "HARD_RESOURCE_BOUND_REACHED",
        )?;
        return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
    }

    let compound_inputs_processed = process_pending_compound_growth(config, state.generation)?;
    if compound_inputs_processed > 0 {
        let phase = state.phase;
        save_transition(
            config,
            &mut state,
            phase,
            "TYPED_COMPOUND_GROWTH_EVIDENCE_COMMITTED_IN_SUPERVISOR_LOOP",
        )?;
    }

    let mut index = load_index(config)?;
    if state.pending_campaign_id.is_some() {
        let pending_id = state.pending_campaign_id.clone();
        let recovered = match recover_pending_campaign(config, &mut state, &mut index) {
            Ok(value) => value,
            Err(error) => {
                let campaign_id =
                    abort_pending_campaign(config, &mut state, Some(&mut index), &error, false)?;
                campaign_id.map(|id| (id, false))
            }
        };
        state.active_runtime_ms = state
            .active_runtime_ms
            .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        let (campaign_id, accepted) = recovered
            .map(|(id, accepted)| (Some(id), Some(accepted)))
            .unwrap_or((pending_id, None));
        let recovery_phase = state.phase;
        save_transition(
            config,
            &mut state,
            recovery_phase,
            "RECOVERY_ACTIVE_RUNTIME_ACCOUNTED",
        )?;
        return Ok(report_from_state(
            &state,
            false,
            0,
            0,
            0,
            campaign_id,
            accepted,
        ));
    }

    let memory = load_memory(config, state.generation)?;
    if json_sha256(&memory)? != state.current_memory_sha256 {
        return Err("CURRENT_MEMORY_HASH_MISMATCH".to_string());
    }
    reconcile_intrinsic_drive_receipts(config, &mut state)?;
    let installed_capability_observation =
        revalidate_installed_composite_capability(config, &mut state, &memory)?;
    let (distinct_semantic_lessons, semantic_duplicate_lessons) = semantic_lesson_counts(&memory)?;
    state.distinct_semantic_lessons = distinct_semantic_lessons;
    state.semantic_duplicate_lessons = semantic_duplicate_lessons;
    state.measured_performance_promotions = executable_performance_promotion_count(&memory);
    state.classifier_outcome_bound_refinements = memory.classifier.outcome_bound_refinements;
    state.classifier_unsupported_refinements_suppressed =
        memory.classifier.unsupported_refinements_suppressed;
    state.generative_predictions = memory.generative.prediction_records;
    state.valuable_combinations_learned = memory.generative.distinct_verified_artifact_count();
    state.generative_memory_reuse_events = memory.generative.reuse_events;
    state.generative_self_application_events = memory.generative.self_application_events;
    state.generative_exploration_events = memory.generative.exploration_events;
    state.productive_generative_reuse_events = memory.generative.productive_reuse_events;
    state.generative_frontier_advance_events = memory.generative.frontier_advance_events;
    state.generative_frontier_capability_units = memory.generative.frontier_capability_units;
    state.unverified_generative_frontier_candidate_events =
        memory.generative.unverified_frontier_candidate_events;
    state.legacy_unverified_generative_frontier_advance_events =
        memory.generative.legacy_unverified_frontier_advance_events;
    state.legacy_wrapper_generative_frontier_advance_events =
        memory.generative.legacy_wrapper_frontier_advance_events;
    state.generative_behavioral_verification_events =
        memory.generative.behavioral_verification_events;
    state.redundant_generative_selection_events = memory.generative.redundant_selection_events;
    state.generative_prediction_absolute_error_total =
        memory.generative.prediction_absolute_error_total;
    state.generative_calibrated_prediction_records =
        memory.generative.calibrated_prediction_records;
    state.generative_legacy_uncalibrated_prediction_error_total =
        memory.generative.legacy_uncalibrated_prediction_error_total;
    let evaluator_memory_sha256 = json_sha256(&memory.evaluator)?;
    if state.current_evaluator_memory_sha256.is_empty() {
        state.current_evaluator_memory_sha256 = evaluator_memory_sha256;
        state.evaluator_generation = memory.evaluator.generation;
        state.evaluator_challenge_cases = memory.evaluator.challenge_suite.len() as u64;
    } else if state.current_evaluator_memory_sha256 != evaluator_memory_sha256
        || state.evaluator_generation != memory.evaluator.generation
    {
        return Err("CURRENT_EVALUATOR_MEMORY_HASH_MISMATCH".to_string());
    }
    if let Some((queued_path, request)) = next_queued_source_patch(config)? {
        let opportunity_kind = request.opportunity_kind;
        let opportunity_family_id = request.opportunity_family_id.clone();
        state.autonomous_source_patch_attempts =
            state.autonomous_source_patch_attempts.saturating_add(1);
        match install_and_stage_source_patch(&config.source_mutation, &config.state_dir, &request) {
            Ok(receipt) => {
                account_source_patch_receipt(&mut state, &receipt);
                let transient =
                    source_patch_failure_is_transient(receipt.failure_reason.as_deref());
                if !transient {
                    fs::remove_file(&queued_path)
                        .map_err(|error| format!("SOURCE_PATCH_QUEUE_CONSUME:{error}"))?;
                }
                if receipt.installed && receipt.runtime_update_staged {
                    state.stop_reason = Some("AUTONOMOUS_SOURCE_UPDATE_STAGED".to_string());
                    state.active_runtime_ms = state.active_runtime_ms.saturating_add(
                        started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    );
                    save_transition(
                        config,
                        &mut state,
                        SupervisorPhase::SafeStopped,
                        "QUEUED_CORE_SOURCE_PATCH_VALIDATED_AND_STAGED_FOR_RESTART",
                    )?;
                    return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
                }
            }
            Err(_) => {
                account_source_patch_error(&mut state, opportunity_kind, opportunity_family_id);
                let rejected = config
                    .state_dir
                    .join("control")
                    .join("source_patch_rejected");
                fs::create_dir_all(&rejected)
                    .map_err(|error| format!("SOURCE_PATCH_REJECTED_DIR:{error}"))?;
                let file_name = queued_path
                    .file_name()
                    .ok_or_else(|| "SOURCE_PATCH_QUEUE_FILE_NAME_MISSING".to_string())?;
                fs::rename(&queued_path, rejected.join(file_name))
                    .map_err(|error| format!("SOURCE_PATCH_QUEUE_QUARANTINE:{error}"))?;
            }
        }
    }
    save_transition(
        config,
        &mut state,
        SupervisorPhase::Scanning,
        "BOUNDED_SCOPED_WORKSPACE_SCAN_STARTED",
    )?;
    let scan_started = Instant::now();
    let mut scan = match scan_watched_roots_bounded(config, &memory, lease) {
        Ok(scan) => scan,
        Err(error)
            if error == "OPERATOR_STOP_REQUESTED_DURING_SCAN"
                || error == "SCAN_RUNTIME_BOUND_REACHED" =>
        {
            let elapsed = scan_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            state.last_scan_duration_ms = elapsed;
            state.last_scan_files_reused = 0;
            state.last_scan_files_hashed = 0;
            state.active_runtime_ms = state.active_runtime_ms.saturating_add(elapsed);
            if error == "SCAN_RUNTIME_BOUND_REACHED" {
                state.scan_timeout_events = state.scan_timeout_events.saturating_add(1);
                state.stop_reason = Some("SCAN_RUNTIME_BOUND_REACHED".to_string());
                save_transition(
                    config,
                    &mut state,
                    SupervisorPhase::SafeStopped,
                    "SCAN_WATCHDOG_STOPPED_UNRESPONSIVE_SCAN",
                )?;
            } else {
                state.stop_reason = Some("OPERATOR_STOP_REQUESTED".to_string());
                save_transition(
                    config,
                    &mut state,
                    SupervisorPhase::SafeStopped,
                    "OPERATOR_STOP_OBSERVED_DURING_SCAN",
                )?;
            }
            return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
        }
        Err(error) => return Err(error),
    };
    state.last_scan_duration_ms =
        scan_started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    state.last_scan_files_reused = scan.files_reused.min(u64::MAX as usize) as u64;
    state.last_scan_files_hashed = scan.files_hashed.min(u64::MAX as usize) as u64;
    state.observed_bytes = state.observed_bytes.saturating_add(scan.bytes_observed);
    persist_scan_observations(config, &scan.observations)?;
    if let Some(observation) = installed_capability_observation.as_ref() {
        persist_scan_observations(config, std::slice::from_ref(observation))?;
    }
    save_index(config, &mut scan.index)?;
    let mut high = load_unconsumed_high_observations(config, &scan.index)?;
    consume_superseded_high_observations(config, &mut state, &mut scan.index, &mut high)?;
    let naive = high
        .iter()
        .take(config.resources.max_observations_per_campaign)
        .cloned()
        .collect::<Vec<_>>();
    let evidence_aware = selected_campaign_observations(config, &high);
    let (
        recent_source_patch_attempts,
        recent_source_patch_installations,
        recent_source_patch_rollbacks,
        recent_source_patch_validation_ms,
    ) = recent_source_patch_stats(&state);
    let inspection = inspect_self(SelfInspectionInput {
        generation: state.generation,
        supervisor_sequence: state.sequence,
        files_scanned: scan.files_scanned,
        files_reused: scan.files_reused,
        files_hashed: scan.files_hashed,
        scan_duration_ms: state.last_scan_duration_ms,
        pending_work_events: scan.pending_work_events,
        replayed_unchanged_work_events: scan.replayed_unchanged_work_events,
        naive_cohort_has_verification: cohort_has_verification_evidence(&naive),
        evidence_aware_cohort_has_verification: cohort_has_verification_evidence(&evidence_aware),
        autonomous_campaigns_enabled: config.autonomous_campaigns,
        campaigns_started: state.campaigns_started,
        mutual_revalidation_events: state.mutual_revalidation_events,
        evaluator_challenge_cases: state.evaluator_challenge_cases,
        evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
        consecutive_failures: state.consecutive_failures,
        plateau_scans: state.plateau_scans,
        unconsumed_high_observations: high.len(),
        cohort_preflight_ready: cohort_has_verification_evidence(&evidence_aware),
        core_cohort_validation_applicable: !core_cohort_observation_ids(config, &evidence_aware)?
            .is_empty(),
        repository_cohort_validation_applicable: repository_validation_plan(
            config,
            &evidence_aware,
        )?
        .is_some(),
        source_patch_attempts: recent_source_patch_attempts,
        source_patch_installations: recent_source_patch_installations,
        source_patch_rollbacks: recent_source_patch_rollbacks,
        source_patch_consecutive_failures: state.source_patch_consecutive_failures,
        source_patch_validation_ms: recent_source_patch_validation_ms,
        source_discovery_no_candidate_streak: state.source_discovery_no_candidate_streak,
        last_source_discovery_reason: state.last_source_discovery_reason.clone(),
        active_runtime_ms: state.active_runtime_ms,
        diagnostic_policy: state.diagnostic_policy.clone(),
    })?;
    persist_self_inspection(config, &mut state, &inspection)?;
    if let Some((action, generated_observation)) = runtime_repair_action(
        config,
        &inspection,
        &scan.observations,
        &naive,
        &evidence_aware,
    )? {
        let action_executed =
            persist_runtime_repair_action(config, &mut state, &inspection, &action)?;
        if config.autonomous_campaigns && action_executed {
            if let Some(observation) = generated_observation {
                persist_scan_observations(config, std::slice::from_ref(&observation))?;
                if !scan
                    .index
                    .consumed_observation_ids
                    .contains(&observation.observation_id)
                    && !high
                        .iter()
                        .any(|existing| existing.observation_id == observation.observation_id)
                {
                    high.push(observation);
                    high.sort_by_key(|observation| {
                        (
                            std::cmp::Reverse(observation.learning_score),
                            observation.observation_id.clone(),
                        )
                    });
                }
            }
        }
    }
    // A successful validator can prove that a structural change is safe, but
    // it cannot manufacture the typed postcondition needed to reuse that
    // change. Keep the evidence immutable and remove only that exact cohort
    // from the active queue so the same diagnostic is not executed forever.
    if config.autonomous_campaigns {
        let _ = defer_verified_non_executable_cohort(
            config,
            state.generation,
            &mut scan.index,
            &mut high,
        )?;
    }
    if high.is_empty()
        && config.autonomous_campaigns
        && state.plateau_scans >= config.resources.plateau_scans_before_wait
    {
        if let Some(observation) =
            plateau_generative_probe_observation(config, &mut state, &memory)?
        {
            persist_scan_observations(config, std::slice::from_ref(&observation))?;
            if !scan
                .index
                .consumed_observation_ids
                .contains(&observation.observation_id)
            {
                high.push(observation);
            }
        }
    }
    let high_count = high.len();
    let mut campaign_id = None;
    let mut campaign_accepted = None;

    if scan.baseline_created {
        state.plateau_scans = 0;
        save_transition(
            config,
            &mut state,
            SupervisorPhase::InfraReady,
            "BASELINE_CREATED_NO_PREEXISTING_WORK_TREATED_AS_NEW_LEARNING",
        )?;
    } else if high.is_empty() {
        state.plateau_scans = state.plateau_scans.saturating_add(1);
        if attempt_discovered_source_repair(config, &mut state, &memory, &scan.index)? {
            state.active_runtime_ms = state
                .active_runtime_ms
                .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
            save_transition(
                config,
                &mut state,
                SupervisorPhase::SafeStopped,
                "CORE_AUTHORED_SOURCE_PATCH_VALIDATED_AND_STAGED_FOR_RESTART",
            )?;
            return Ok(report_from_state(
                &state,
                scan.baseline_created,
                scan.files_scanned,
                scan.observations.len(),
                high_count,
                None,
                None,
            ));
        }
        let phase = if state.plateau_scans >= config.resources.plateau_scans_before_wait {
            SupervisorPhase::WaitingPlateau
        } else {
            SupervisorPhase::InfraReady
        };
        save_transition(
            config,
            &mut state,
            phase,
            if phase == SupervisorPhase::WaitingPlateau {
                "PLATEAU_WAIT_NO_DIFFICULTY_ESCALATION"
            } else {
                "NO_HIGH_VALUE_OBSERVATION"
            },
        )?;
    } else if !config.autonomous_campaigns {
        save_transition(
            config,
            &mut state,
            SupervisorPhase::InfraReady,
            "HIGH_VALUE_WORK_AVAILABLE_AUTONOMOUS_CAMPAIGNS_DISABLED",
        )?;
    } else if resource_stop_reason(config, &state)?.is_some() {
        state.stop_reason = resource_stop_reason(config, &state)?;
        save_transition(
            config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "RESOURCE_BOUND_BLOCKED_NEW_CAMPAIGN",
        )?;
    } else if !campaign_preflight_ready(config, &high)? {
        state.plateau_scans = state.plateau_scans.saturating_add(1);
        if attempt_discovered_source_repair(config, &mut state, &memory, &scan.index)? {
            state.active_runtime_ms = state
                .active_runtime_ms
                .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
            save_transition(
                config,
                &mut state,
                SupervisorPhase::SafeStopped,
                "DEFERRED_COHORT_TRIGGERED_SOURCE_REPAIR_STAGED_FOR_RESTART",
            )?;
            return Ok(report_from_state(
                &state,
                scan.baseline_created,
                scan.files_scanned,
                scan.observations.len(),
                high_count,
                None,
                None,
            ));
        }
        save_transition(
            config,
            &mut state,
            SupervisorPhase::WaitingPlateau,
            "CAMPAIGN_DEFERRED_WAITING_FOR_PASS_OR_TEST_COHORT",
        )?;
    } else if consume_semantic_revalidation(config, &mut state, &mut scan.index, &memory, &high)?
        .is_some()
    {
        let phase = if state.plateau_scans >= config.resources.plateau_scans_before_wait {
            SupervisorPhase::WaitingPlateau
        } else {
            SupervisorPhase::InfraReady
        };
        save_transition(
            config,
            &mut state,
            phase,
            "SEMANTIC_REVALIDATION_CONSUMED_WITHOUT_GENERATION_PROMOTION",
        )?;
    } else if !executable_generative_substrate_available(&memory.generative) {
        state.plateau_scans = state.plateau_scans.saturating_add(1);
        // Closing the finite generative campaign catalog is not the same as
        // exhausting source-level improvement. High-value observations can
        // remain permanently unconsumed at this boundary; route them into the
        // typed source synthesizer before entering passive plateau wait.
        if attempt_discovered_source_repair(config, &mut state, &memory, &scan.index)? {
            state.active_runtime_ms = state
                .active_runtime_ms
                .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
            save_transition(
                config,
                &mut state,
                SupervisorPhase::SafeStopped,
                "GENERATIVE_SUBSTRATE_CLOSURE_TRIGGERED_SOURCE_REPAIR_STAGED",
            )?;
            return Ok(report_from_state(
                &state,
                scan.baseline_created,
                scan.files_scanned,
                scan.observations.len(),
                high_count,
                None,
                None,
            ));
        }
        save_transition(
            config,
            &mut state,
            SupervisorPhase::WaitingPlateau,
            "CAMPAIGN_DEFERRED_NO_EXECUTABLE_GENERATIVE_SUBSTRATE",
        )?;
    } else {
        let freeze = freeze_new_campaign(config, &mut state, &high)?;
        campaign_id = Some(freeze.campaign_id.clone());
        match execute_campaign(config, &mut state, &mut scan.index, freeze) {
            Ok(accepted) => campaign_accepted = Some(accepted),
            Err(error) => {
                let _ = abort_pending_campaign(
                    config,
                    &mut state,
                    Some(&mut scan.index),
                    &error,
                    true,
                )?;
                campaign_accepted = Some(false);
            }
        }
    }

    state.active_runtime_ms = state
        .active_runtime_ms
        .saturating_add(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
    let final_phase = state.phase;
    save_transition(
        config,
        &mut state,
        final_phase,
        "ACTIVE_RUNTIME_AND_RESOURCE_ACCOUNTING_COMMITTED",
    )?;
    Ok(report_from_state(
        &state,
        scan.baseline_created,
        scan.files_scanned,
        scan.observations.len(),
        high_count,
        campaign_id,
        campaign_accepted,
    ))
}

pub fn supervisor_step(config_path: &Path) -> Result<StepReport, String> {
    let config = load_config(config_path)?;
    let _ = initialize(config_path)?;
    let lease = SupervisorLease::acquire(&config)?;
    lease.heartbeat()?;
    let _ = recover_repository_install_transactions(&config)?;
    // Runtime staging is a disposable transfer cache, not research authority.
    // Cleanup is deliberately outside the semantic step transaction and may
    // never turn a valid sealed state into a failed campaign.
    let _ = cleanup_consumed_source_mutation_staging(&config.state_dir);
    step_without_lease(&config, &lease)
}

pub fn cleanup_source_staging(config_path: &Path) -> Result<SourceMutationStagingCleanup, String> {
    let config = load_config(config_path)?;
    let lease = SupervisorLease::acquire(&config)?;
    lease.heartbeat()?;
    cleanup_consumed_source_mutation_staging(&config.state_dir)
}

pub fn run_daemon(config_path: &Path) -> Result<StepReport, String> {
    let config = load_config(config_path)?;
    let _ = initialize(config_path)?;
    let lease = SupervisorLease::acquire(&config)?;
    let _ = recover_repository_install_transactions(&config)?;
    let _ = cleanup_consumed_source_mutation_staging(&config.state_dir);
    loop {
        lease.heartbeat()?;
        let report = step_without_lease(&config, &lease)?;
        if report.phase == SupervisorPhase::SafeStopped {
            return Ok(report);
        }
        let poll_interval_ms = if report.waiting_on_plateau && report.high_value_observations == 0 {
            config
                .poll_interval_ms
                .saturating_mul(6)
                .min(MAX_QUIET_IDLE_POLL_INTERVAL_MS)
        } else {
            config.poll_interval_ms
        };
        thread::sleep(Duration::from_millis(poll_interval_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "b-core-growth-{label}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_config(root: &Path) -> (PathBuf, GrowthSupervisorConfig) {
        let watched = root.join("watched");
        let state = root.join("state");
        fs::create_dir_all(&watched).unwrap();
        let verifier = root.join(if cfg!(windows) {
            "verifier.exe"
        } else {
            "verifier"
        });
        fs::write(&verifier, b"independent verifier placeholder").unwrap();
        let mut config = GrowthSupervisorConfig::bounded_default(state, watched, verifier);
        config.poll_interval_ms = 1_000;
        config.lease_stale_ms = 3_000;
        config.resources.plateau_scans_before_wait = 2;
        config.autonomous_campaigns = false;
        config.repository_mutation.enabled = false;
        let config_path = root.join("config.json");
        write_immutable_json(&config_path, &config).unwrap();
        (config_path, config)
    }

    #[test]
    fn sealed_resource_stop_continues_with_memory_and_executable_knowledge_only() {
        let root = temp_root("lineage-continuation");
        let watched = root.join("watched");
        fs::create_dir_all(&watched).unwrap();
        let verifier = root.join(if cfg!(windows) {
            "verifier.exe"
        } else {
            "verifier"
        });
        fs::write(&verifier, b"independent verifier placeholder").unwrap();

        let predecessor_config_path = root.join("predecessor.json");
        let mut predecessor =
            GrowthSupervisorConfig::bounded_default(root.join("state-r4"), watched, verifier);
        predecessor.poll_interval_ms = 1_000;
        predecessor.lease_stale_ms = 3_000;
        predecessor.resources.max_generations = 1;
        predecessor.autonomous_campaigns = false;
        predecessor.repository_mutation.enabled = false;
        write_immutable_json(&predecessor_config_path, &predecessor).unwrap();
        let mut predecessor_state = initialize(&predecessor_config_path).unwrap();
        let memory_zero = load_memory(&predecessor, 0).unwrap();
        let memory_zero_sha256 = json_sha256(&memory_zero).unwrap();
        let mut memory_one = memory_zero.clone();
        memory_one.generation = 1;
        memory_one.predecessor_sha256 = Some(memory_zero_sha256.clone());
        memory_one.lessons.push(LearnedCompositionLesson {
            lesson_id: "EXECUTABLE-LINEAGE-LESSON".to_string(),
            evidence_observation_sha256: vec!["a".repeat(64)],
            work_kinds: vec![WorkKind::DefectRepair],
            diagnostic_signals: vec!["VALIDATED_REPAIR".to_string()],
            composition_recipe: vec!["REPLACE".to_string()],
            applicability: vec!["SAME_STRUCTURAL_FAMILY".to_string()],
            verification_obligations: vec!["PUBLIC_REGRESSION".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        });
        let memory_one_sha256 = json_sha256(&memory_one).unwrap();
        write_immutable_json(&memory_path(&predecessor, 1), &memory_one).unwrap();
        predecessor_state.generation = 1;
        predecessor_state.current_memory_sha256 = memory_one_sha256.clone();
        predecessor_state.predecessor_memory_sha256 = Some(memory_zero_sha256.clone());
        predecessor_state.stop_reason = Some("MAX_GENERATIONS_REACHED".to_string());
        save_transition(
            &predecessor,
            &mut predecessor_state,
            SupervisorPhase::SafeStopped,
            "TEST_RESOURCE_STOP",
        )
        .unwrap();

        let mut index = FileIndex {
            baseline_complete: true,
            ..FileIndex::default()
        };
        index
            .consumed_work_event_ids
            .insert("already-consumed".to_string());
        save_index(&predecessor, &mut index).unwrap();
        write_immutable_json(
            &predecessor
                .state_dir
                .join("source_repair_knowledge")
                .join("operator.json"),
            &serde_json::json!({"executable": true}),
        )
        .unwrap();
        write_immutable_json(
            &predecessor
                .state_dir
                .join("source_mutations")
                .join("PATCH-1")
                .join("receipt.json"),
            &serde_json::json!({"installed": true}),
        )
        .unwrap();
        let staged_binary = predecessor
            .state_dir
            .join("source_mutations")
            .join("PATCH-1")
            .join("staging")
            .join("growth-supervisor.exe");
        fs::create_dir_all(staged_binary.parent().unwrap()).unwrap();
        fs::write(&staged_binary, b"disposable build product").unwrap();

        let drifted_config_path = root.join("drifted-successor.json");
        let mut drifted = predecessor.clone();
        drifted.state_dir = root.join("state-policy-drift");
        drifted.resources.max_generations = 2;
        drifted.observation.minimum_learning_score += 1;
        write_immutable_json(&drifted_config_path, &drifted).unwrap();
        assert_eq!(
            continue_lineage(&predecessor_config_path, &drifted_config_path).unwrap_err(),
            "LINEAGE_SUCCESSOR_POLICY_DRIFT"
        );
        assert!(!drifted.state_dir.exists());

        let successor_config_path = root.join("successor.json");
        let mut successor = predecessor.clone();
        successor.state_dir = root.join("state-r5");
        successor.resources.max_generations = 2;
        write_immutable_json(&successor_config_path, &successor).unwrap();

        let receipt = continue_lineage(&predecessor_config_path, &successor_config_path).unwrap();
        assert_eq!(receipt.schema, LINEAGE_CONTINUATION_SCHEMA);
        assert_eq!(receipt.predecessor_generation, 1);
        assert_eq!(receipt.predecessor_memory_sha256, memory_one_sha256);
        assert_eq!(receipt.carried_memory_sha256.len(), 2);
        assert_eq!(
            receipt.receipt_sha256,
            lineage_receipt_hash(&receipt).unwrap()
        );
        let continued = status(&successor_config_path).unwrap();
        assert_eq!(continued.generation, 1);
        assert_eq!(continued.phase, SupervisorPhase::InfraReady);
        assert_eq!(continued.stop_reason, None);
        assert_eq!(continued.current_memory_sha256, memory_one_sha256);
        assert_eq!(
            continued.predecessor_memory_sha256,
            Some(memory_zero_sha256)
        );
        assert!(successor
            .state_dir
            .join("source_repair_knowledge/operator.json")
            .is_file());
        assert!(successor
            .state_dir
            .join("source_mutations/PATCH-1/receipt.json")
            .is_file());
        assert!(!successor
            .state_dir
            .join("source_mutations/PATCH-1/staging/growth-supervisor.exe")
            .exists());
        assert_eq!(load_index(&successor).unwrap(), index);
        fs::remove_dir_all(root).unwrap();
    }

    fn typed_behavior_goal_fixture(goal_id: &str) -> TypedMechanismSynthesisGoalIR {
        use crate::sem5::{
            model::{DataSplit, Effect, ProgramType, Value},
            typed_mechanism::{
                SourceOperandIR, TypedMechanismObservationIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
            },
        };

        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                SourceOperandIR {
                    role: "base".to_string(),
                    source: "node.baseline".to_string(),
                    value_type: ProgramType::Int,
                },
                SourceOperandIR {
                    role: "gain".to_string(),
                    source: "observation.gain".to_string(),
                    value_type: ProgramType::Int,
                },
            ],
            output_type: ProgramType::Int,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["postimage matches every public observation".to_string()],
            invariants: Vec::new(),
            public_observations: [(4, 3, 7), (-2, 8, 6), (10, -3, 7)]
                .into_iter()
                .map(|(base, gain, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("base".to_string(), Value::Int(base)),
                        ("gain".to_string(), Value::Int(gain)),
                    ]),
                    expected_postimage: Value::Int(expected),
                })
                .collect(),
            require_conditional: false,
            max_expression_depth: 2,
            max_candidates: 1_024,
            provenance: vec!["PUBLIC_CONTRACT_DELTA".to_string()],
        }
    }

    fn boolean_gate_goal_fixture(goal_id: &str) -> TypedMechanismSynthesisGoalIR {
        use crate::sem5::{
            model::{DataSplit, Effect, ProgramType, Value},
            typed_mechanism::{
                SourceOperandIR, TypedMechanismObservationIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
            },
        };

        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                SourceOperandIR {
                    role: "verified".to_string(),
                    source: "state.verified".to_string(),
                    value_type: ProgramType::Bool,
                },
                SourceOperandIR {
                    role: "executable".to_string(),
                    source: "state.executable".to_string(),
                    value_type: ProgramType::Bool,
                },
            ],
            output_type: ProgramType::Bool,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["retain unverified or executable cohorts".to_string()],
            invariants: vec!["verification cannot create knowledge".to_string()],
            public_observations: [
                (false, false, true),
                (true, false, false),
                (true, true, true),
            ]
            .into_iter()
            .map(
                |(verified, executable, expected)| TypedMechanismObservationIR {
                    operands: BTreeMap::from([
                        ("verified".to_string(), Value::Bool(verified)),
                        ("executable".to_string(), Value::Bool(executable)),
                    ]),
                    expected_postimage: Value::Bool(expected),
                },
            )
            .collect(),
            require_conditional: false,
            max_expression_depth: 3,
            max_candidates: 128,
            provenance: vec!["PUBLIC_CONTRACT_DELTA".to_string()],
        }
    }

    fn conditional_string_transport_goal_fixture(goal_id: &str) -> TypedMechanismSynthesisGoalIR {
        use crate::sem5::{
            model::{DataSplit, Effect, ProgramType, Value},
            typed_mechanism::{
                SourceOperandIR, TypedMechanismObservationIR, TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA,
            },
        };

        TypedMechanismSynthesisGoalIR {
            schema: TYPED_MECHANISM_SYNTHESIS_GOAL_SCHEMA.to_string(),
            goal_id: goal_id.to_string(),
            split: DataSplit::FreshBlind,
            operands: vec![
                SourceOperandIR {
                    role: "condition".to_string(),
                    source: "state.condition".to_string(),
                    value_type: ProgramType::Bool,
                },
                SourceOperandIR {
                    role: "typed_value".to_string(),
                    source: "observation.typed_value".to_string(),
                    value_type: ProgramType::String,
                },
            ],
            output_type: ProgramType::String,
            definitions: Vec::new(),
            allowed_effects: vec![Effect::Pure],
            preconditions: Vec::new(),
            postconditions: vec!["transport the value only when allowed".to_string()],
            invariants: vec!["false uses the universal empty value".to_string()],
            public_observations: [
                (true, "alpha", "alpha"),
                (true, "beta", "beta"),
                (false, "alpha", ""),
            ]
            .into_iter()
            .map(|(condition, value, expected)| TypedMechanismObservationIR {
                operands: BTreeMap::from([
                    ("condition".to_string(), Value::Bool(condition)),
                    ("typed_value".to_string(), Value::String(value.to_string())),
                ]),
                expected_postimage: Value::String(expected.to_string()),
            })
            .collect(),
            require_conditional: true,
            max_expression_depth: 3,
            max_candidates: 128,
            provenance: vec!["PUBLIC_CONTRACT_DELTA".to_string()],
        }
    }

    fn compound_input_fixture(input_id: &str) -> CompoundGrowthInputIR {
        use crate::compound_growth::{
            ActiveExperimentCandidateIR, ExperimentPredictionIR, HypothesisIR,
        };

        CompoundGrowthInputIR {
            schema: COMPOUND_GROWTH_INPUT_SCHEMA.to_string(),
            input_id: input_id.to_string(),
            evidence_sha256: vec![sha256(format!("evidence:{input_id}").as_bytes())],
            mechanisms: Vec::new(),
            execution_traces: Vec::new(),
            promotion_evidence: Vec::new(),
            source_bindings: Vec::new(),
            hypotheses: vec![
                HypothesisIR {
                    hypothesis_id: "BOTTLENECK-IO".to_string(),
                },
                HypothesisIR {
                    hypothesis_id: "BOTTLENECK-COMPUTE".to_string(),
                },
            ],
            experiment_candidates: vec![ActiveExperimentCandidateIR {
                experiment_id: "READ-ONLY-PROFILE".to_string(),
                predictions: vec![
                    ExperimentPredictionIR {
                        hypothesis_id: "BOTTLENECK-IO".to_string(),
                        observation_signature: "IO-WAIT-HIGH".to_string(),
                    },
                    ExperimentPredictionIR {
                        hypothesis_id: "BOTTLENECK-COMPUTE".to_string(),
                        observation_signature: "CPU-TIME-HIGH".to_string(),
                    },
                ],
                reliability_millis: 900,
                cost_millis: 10,
                risk_millis: 0,
                read_only: true,
            }],
            counterexamples: Vec::new(),
            revision_candidates: Vec::new(),
            operator_outcomes: Vec::new(),
        }
    }

    #[test]
    fn ordinary_supervisor_step_commits_typed_compound_growth_input_once() {
        let root = temp_root("compound-supervisor-loop");
        let (config_path, _) = test_config(&root);
        let input = compound_input_fixture("INPUT-READ-ONLY-BOTTLENECK");
        let queued = record_compound_growth_input(&config_path, input.clone()).unwrap();
        assert_eq!(queued["queued"], true);
        assert_eq!(
            compound_growth_status(&config_path).unwrap().pending_inputs,
            1
        );

        supervisor_step(&config_path).unwrap();
        let status = compound_growth_status(&config_path).unwrap();
        assert_eq!(status.cycles_committed, 1);
        assert_eq!(status.pending_inputs, 0);
        assert_eq!(status.external_model_calls, 0);
        assert_eq!(status.text_only_growth_events, 0);
        let receipts = load_compound_growth_receipts(&load_config(&config_path).unwrap()).unwrap();
        assert_eq!(
            receipts[0]
                .cycle
                .selected_experiment
                .as_ref()
                .map(|selection| selection.experiment_id.as_str()),
            Some("READ-ONLY-PROFILE")
        );

        let duplicate = record_compound_growth_input(&config_path, input).unwrap();
        assert_eq!(duplicate["already_committed"], true);
        supervisor_step(&config_path).unwrap();
        assert_eq!(
            compound_growth_status(&config_path)
                .unwrap()
                .cycles_committed,
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn public_contract_delta_fixture() -> PublicContractDeltaIR {
        let mut delta = PublicContractDeltaIR {
            schema: PUBLIC_CONTRACT_DELTA_SCHEMA.to_string(),
            delta_id: "observed-return-to-expected-sum".to_string(),
            observed_behavior: "call returns the base operand".to_string(),
            expected_behavior: "call returns base plus gain".to_string(),
            target_symbols: vec!["crate::engine::apply_gain".to_string()],
            typed_behavior_goals: vec![typed_behavior_goal_fixture("apply_gain_contract")],
            provenance: vec!["PUBLIC_OBSERVATION".to_string()],
        };
        bind_public_contract_delta_fixture(&mut delta);
        delta
    }

    fn bind_public_contract_delta_fixture(delta: &mut PublicContractDeltaIR) {
        let binding = format!(
            "PUBLIC_CONTRACT_DELTA_SHA256:{}",
            public_contract_delta_binding_sha256(delta).unwrap()
        );
        let id_binding = format!("PUBLIC_CONTRACT_DELTA_ID:{}", delta.delta_id);
        for goal in &mut delta.typed_behavior_goals {
            goal.provenance.retain(|item| {
                !item.starts_with("PUBLIC_CONTRACT_DELTA_SHA256:")
                    && !item.starts_with("PUBLIC_CONTRACT_DELTA_ID:")
            });
            goal.provenance.push(id_binding.clone());
            goal.provenance.push(binding.clone());
        }
    }

    #[test]
    fn opportunity_metrics_count_unique_families_not_repeated_attempts() {
        let root = temp_root("opportunity-family-metrics");
        let (config_path, _) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let defect_family =
            source_opportunity_family_id(ChangeOpportunityKind::Defect, "GENERAL_PARSE_REPAIR");
        let efficiency_family = source_opportunity_family_id(
            ChangeOpportunityKind::EfficiencyOpportunity,
            "GENERAL_SCAN_CACHE_REUSE",
        );
        for installed in [false, false, true] {
            state
                .source_patch_recent_outcomes
                .push(SourcePatchOutcomeSample {
                    engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
                    installed,
                    rolled_back: !installed,
                    validation_ms: 11,
                    opportunity_kind: ChangeOpportunityKind::Defect,
                    opportunity_family_id: defect_family.clone(),
                });
        }
        state
            .source_patch_recent_outcomes
            .push(SourcePatchOutcomeSample {
                engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
                installed: true,
                rolled_back: false,
                validation_ms: 7,
                opportunity_kind: ChangeOpportunityKind::EfficiencyOpportunity,
                opportunity_family_id: efficiency_family,
            });

        let stats = recent_source_opportunity_stats(&state);
        assert_eq!(stats.distinct_total, 2);
        assert_eq!(stats.defects, 1);
        assert_eq!(stats.efficiency_opportunities, 1);
        assert_eq!(stats.verified_improvements, 2);
        assert_eq!(recent_source_patch_stats(&state), (4, 2, 2, 40));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verified_generative_frontier_increase_seeds_exactly_one_successor_observation() {
        let input = GenerativeInput {
            source_lesson_id: "frontier-continuation-source".to_string(),
            diagnostic_signals: vec!["CAPABILITY_SURFACE_ADDED".to_string()],
            observed_composition_roles: vec![
                "IMPLEMENTATION".to_string(),
                "REGRESSION_TEST".to_string(),
            ],
            learning_score: 90,
            verification_evidence_count: 1,
            measured_performance_gain: false,
            typed_behavior_goals: vec![typed_behavior_goal_fixture("frontier-continuation-goal")],
            executable_performance_operators: Vec::new(),
        };
        let result = run_generative_cycle(&GenerativeGrowthMemory::default(), &input, 17).unwrap();
        assert!(result.frontier_advance);
        let observation = generative_frontier_continuation_observation(
            "G-CONTINUATION",
            1,
            &"a".repeat(64),
            &"b".repeat(64),
            0,
            result.frontier_advance_units,
            &result,
            &"c".repeat(64),
            true,
        )
        .unwrap()
        .expect("strict frontier increase creates one continuation");
        assert_eq!(observation.work_kind, WorkKind::CapabilitySynthesis);
        assert_eq!(observation.work_outcome, WorkOutcome::Pass);
        assert!(observation
            .signals
            .contains(&"GENERATIVE_FRONTIER_CONTINUATION".to_string()));
        assert_eq!(observation.performance_metrics[0].before, 0);
        assert_eq!(
            observation.performance_metrics[0].after,
            result.frontier_advance_units
        );
        assert_eq!(observation.exact_source_fragments_stored, 0);
        assert_eq!(observation.raw_source_bytes_stored, 0);
        assert!(generative_frontier_continuation_observation(
            "G-CONTINUATION",
            2,
            &"b".repeat(64),
            &"d".repeat(64),
            result.frontier_advance_units,
            result.frontier_advance_units,
            &result,
            &"e".repeat(64),
            true,
        )
        .unwrap()
        .is_none());
        assert!(generative_frontier_continuation_observation(
            "G-CONTINUATION",
            2,
            &"b".repeat(64),
            &"d".repeat(64),
            result.frontier_advance_units,
            result.frontier_advance_units * 2,
            &result,
            &"e".repeat(64),
            false,
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn plateau_cross_lesson_probe_rejects_text_only_lessons() {
        let root = temp_root("plateau-cross-lesson-probe");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let mut memory = load_memory(&config, 0).unwrap();
        let lesson = |id: &str, signal: &str, role: &str| LearnedCompositionLesson {
            lesson_id: id.to_string(),
            evidence_observation_sha256: vec![sha256(format!("evidence-{id}").as_bytes())],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec![signal.to_string(), "VERIFIED_PASS".to_string()],
            composition_recipe: vec![role.to_string(), "REGRESSION_TEST".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        memory.lessons = vec![
            lesson("LESSON-A", "AST_ROLE_BINDING", "PREDICT"),
            lesson("LESSON-B", "COUNTEREXAMPLE_REVISION", "COMPOSE"),
        ];

        assert!(
            plateau_generative_probe_observation(&config, &mut state, &memory)
                .unwrap()
                .is_none()
        );
        assert!(!config.state_dir.join("generative_plateau_probes").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_contention_does_not_poison_source_repair_telemetry() {
        let root = temp_root("workspace-contention-telemetry");
        let (config_path, _) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let command = LocalCommandReceipt {
            program: "cargo".to_string(),
            args: vec!["test".to_string()],
            cargo_incremental: true,
            exit_code: Some(0),
            success: true,
            timed_out: false,
            duration_ms: 17,
            output_sha256: "a".repeat(64),
            diagnostic_tail: String::new(),
            ..Default::default()
        };
        let receipt = AutonomousSourcePatchReceipt {
            schema: "B_CORE_AUTONOMOUS_SOURCE_MUTATION_1".to_string(),
            patch_id: "contention".to_string(),
            relative_path: PathBuf::from("src/lib.rs"),
            predecessor_sha256: "b".repeat(64),
            candidate_sha256: "c".repeat(64),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: ChangeOpportunityKind::Defect,
            opportunity_family_id: "family".to_string(),
            installed: false,
            rolled_back: true,
            failure_reason: Some("CONCURRENT_WORKSPACE_CHANGE_DURING_VALIDATION".to_string()),
            format_check: None,
            compile_check: None,
            validation: command,
            release_build: None,
            runtime_update_staged: false,
            rollback_source: PathBuf::from("predecessor.source"),
            workspace_fingerprint_before: "d".repeat(64),
            workspace_fingerprint_after: "e".repeat(64),
            workspace_stable_during_validation: false,
            receipt_sha256: "f".repeat(64),
        };

        account_source_patch_receipt(&mut state, &receipt);

        assert_eq!(state.autonomous_source_patch_validation_ms, 17);
        assert_eq!(state.autonomous_source_patch_rollbacks, 0);
        assert_eq!(state.source_patch_consecutive_failures, 0);
        assert!(state.source_patch_recent_outcomes.is_empty());
        assert_eq!(
            state.last_source_discovery_reason.as_deref(),
            Some("TRANSIENT_WORKSPACE_CONTENTION_DEFERRED")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn evaluator_accounts_parallel_source_validation_by_critical_path() {
        let root = temp_root("parallel-source-validation-telemetry");
        let (config_path, _) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let command = |program: &str, duration_ms: u64| LocalCommandReceipt {
            program: program.to_string(),
            args: Vec::new(),
            cargo_incremental: true,
            exit_code: Some(0),
            success: true,
            timed_out: false,
            duration_ms,
            output_sha256: program.to_string(),
            diagnostic_tail: String::new(),
            ..Default::default()
        };
        let receipt = AutonomousSourcePatchReceipt {
            schema: "B_CORE_AUTONOMOUS_SOURCE_MUTATION_1".to_string(),
            patch_id: "parallel-evaluator".to_string(),
            relative_path: PathBuf::from("src/lib.rs"),
            predecessor_sha256: "a".repeat(64),
            candidate_sha256: "b".repeat(64),
            core_generated: true,
            core_self_approved: true,
            opportunity_kind: ChangeOpportunityKind::EfficiencyOpportunity,
            opportunity_family_id: "parallel-family".to_string(),
            installed: true,
            rolled_back: false,
            failure_reason: None,
            format_check: Some(command("fmt", 10)),
            compile_check: Some(command("clippy", 100)),
            validation: command("test", 200),
            release_build: Some(command("release", 500)),
            runtime_update_staged: true,
            rollback_source: PathBuf::from("predecessor.source"),
            workspace_fingerprint_before: "c".repeat(64),
            workspace_fingerprint_after: "c".repeat(64),
            workspace_stable_during_validation: true,
            receipt_sha256: "d".repeat(64),
        };

        account_source_patch_receipt(&mut state, &receipt);

        assert_eq!(state.autonomous_source_patch_validation_ms, 510);
        assert_eq!(state.source_patch_recent_outcomes.len(), 1);
        assert_eq!(state.source_patch_recent_outcomes[0].validation_ms, 510);
        assert_eq!(recent_source_patch_stats(&state), (1, 1, 0, 510));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_discovery_identity_changes_only_with_relevant_source_or_generation() {
        let root = temp_root("source-discovery-identity");
        let (config_path, mut config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        config.source_mutation.enabled = true;
        config.source_mutation.source_root = config.watched_roots[0].clone();
        let mut index = FileIndex::default();
        index.files.insert(
            "ROOT_0/src/lib.rs".to_string(),
            FileFingerprint {
                content_sha256: "a".repeat(64),
                bytes: 10,
                modified_ms: 1,
                extension: "rs".to_string(),
                features: StructuralFeatures::default(),
            },
        );
        let first = source_discovery_state_sha256(&config, &state, &index).unwrap();
        index
            .files
            .get_mut("ROOT_0/src/lib.rs")
            .unwrap()
            .modified_ms = 2;
        assert_eq!(
            source_discovery_state_sha256(&config, &state, &index).unwrap(),
            first
        );

        index
            .files
            .get_mut("ROOT_0/src/lib.rs")
            .unwrap()
            .content_sha256 = "b".repeat(64);
        let changed_source = source_discovery_state_sha256(&config, &state, &index).unwrap();
        assert_ne!(changed_source, first);

        state.generation = state.generation.saturating_add(1);
        assert_ne!(
            source_discovery_state_sha256(&config, &state, &index).unwrap(),
            changed_source
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_source_discovery_state_is_suppressed_before_rediscovery() {
        let root = temp_root("source-discovery-dedup");
        let (config_path, mut config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let memory = load_memory(&config, 0).unwrap();
        config.source_mutation.enabled = true;
        config.source_mutation.source_root = config.watched_roots[0].clone();
        config.source_mutation.auto_discover_compiler_repairs = true;
        let mut index = FileIndex::default();
        index.files.insert(
            "ROOT_0/src/lib.rs".to_string(),
            FileFingerprint {
                content_sha256: "c".repeat(64),
                bytes: 10,
                modified_ms: 1,
                extension: "rs".to_string(),
                features: StructuralFeatures::default(),
            },
        );
        state.plateau_scans = config.resources.plateau_scans_before_wait;
        state.source_discovery_no_candidate_streak = 1;
        state.last_source_discovery_reason = Some("NO_APPLICABLE_TRANSFORMATION".to_string());
        state.last_source_discovery_state_sha256 =
            Some(source_discovery_state_sha256(&config, &state, &index).unwrap());

        assert!(!attempt_discovered_source_repair(&config, &mut state, &memory, &index).unwrap());
        assert_eq!(state.source_discovery_no_candidate_streak, 1);
        assert_eq!(state.source_discovery_duplicate_states_suppressed, 1);
        assert_eq!(
            state.last_source_discovery_reason.as_deref(),
            Some("NO_APPLICABLE_TRANSFORMATION")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repeated_unbound_gap_does_not_emit_diagnostic_report_spam() {
        let root = temp_root("unbound-gap-report-dedup");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let generation = state.generation;
        let inspection_input = |sequence, streak, policy| SelfInspectionInput {
            generation,
            supervisor_sequence: sequence,
            files_scanned: 10,
            files_reused: 10,
            files_hashed: 0,
            scan_duration_ms: 10,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 1,
            mutual_revalidation_events: 1,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 12,
            unconsumed_high_observations: 0,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: false,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: streak,
            last_source_discovery_reason: Some("BELOW_VALUE_THRESHOLD".to_string()),
            active_runtime_ms: 100,
            diagnostic_policy: policy,
        };
        let first = inspect_self(inspection_input(1, 4, state.diagnostic_policy.clone())).unwrap();
        persist_self_inspection(&config, &mut state, &first).unwrap();
        let first_sha = state.last_self_inspection_sha256.clone();

        let repeated =
            inspect_self(inspection_input(2, 5, state.diagnostic_policy.clone())).unwrap();
        persist_self_inspection(&config, &mut state, &repeated).unwrap();

        assert_eq!(state.self_inspection_events, 1);
        assert_eq!(state.diagnostic_experiment_events, 1);
        assert_eq!(state.last_self_inspection_sha256, first_sha);
        assert_eq!(state.diagnostic_policy.duplicate_selection_suppressed, 1);
        assert_eq!(
            fs::read_dir(config.state_dir.join("diagnostics"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("self_inspection_"))
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hashed_diagnostic_cleanup_keeps_the_newest_receipt_not_largest_name() {
        let root = temp_root("diagnostic-retention");
        let old = root.join("self_inspection_ffff.json");
        let newest = root.join("self_inspection_0000.json");
        fs::write(&old, b"old").unwrap();
        thread::sleep(Duration::from_millis(20));
        fs::write(&newest, b"newest").unwrap();

        cleanup_recent_files(&root, "self_inspection_", 1).unwrap();

        assert!(!old.exists());
        assert!(newest.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_validation_cache_isolated_from_source_state_and_developer_target() {
        let root = temp_root("runtime-validation-target-isolation");
        let (_, mut config) = test_config(&root);
        config.source_mutation.source_root = config.watched_roots[0].clone();
        config.source_mutation.build_target_dir = root.join("developer-target");
        config.source_mutation.runtime_bin_dir = root.join("runtime/bin");

        let validation_target = runtime_validation_target_dir(&config);

        assert_eq!(validation_target, root.join("runtime/validation-target"));
        assert!(!validation_target.starts_with(&config.source_mutation.source_root));
        assert!(!validation_target.starts_with(&config.state_dir));
        assert_ne!(validation_target, config.source_mutation.build_target_dir);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core_validation_targets_changed_top_level_rust_module_between_canaries() {
        let root = temp_root("targeted-core-validation");
        let (_, mut config) = test_config(&root);
        config.source_mutation.enabled = true;
        config.source_mutation.source_root = config.watched_roots[0].clone();
        fs::write(
            config.source_mutation.source_root.join("Cargo.toml"),
            "[features]\nruntime-core = []\n",
        )
        .unwrap();
        let source_dir = config
            .source_mutation
            .source_root
            .join("crates/semantic-reasoning/src");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("lib.rs"),
            "pub mod growth_supervisor;\npub mod generated_sem5_capability;\n",
        )
        .unwrap();
        fs::write(
            source_dir.join("growth_supervisor.rs"),
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn exercises_module() {}\n}\n",
        )
        .unwrap();
        fs::write(
            source_dir.join("generated_sem5_capability.rs"),
            "pub fn generated_capability() {}\n",
        )
        .unwrap();
        let observation = LearningObservation {
            observation_id: "growth-supervisor-change".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/crates/semantic-reasoning/src/growth_supervisor.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: Some("b".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 80,
            learning_value: LearningValue::High,
            reasons: vec!["fixture".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };

        let targeted =
            core_validation_plan(&config, 2, std::slice::from_ref(&observation)).unwrap();
        assert_eq!(targeted.validation_scope, "CHANGED_RUST_MODULE");
        assert_eq!(
            targeted.targeted_test_filter.as_deref(),
            Some("growth_supervisor::tests::")
        );
        assert_eq!(
            targeted.args.last().map(String::as_str),
            Some("growth_supervisor::tests::")
        );
        assert!(!targeted.full_regression_canary);

        let canary = core_validation_plan(
            &config,
            FULL_CORE_REGRESSION_CANARY_INTERVAL,
            std::slice::from_ref(&observation),
        )
        .unwrap();
        assert_eq!(
            canary.validation_scope,
            "FULL_RUNTIME_CORE_REGRESSION_CANARY"
        );
        assert!(canary.targeted_test_filter.is_none());
        assert!(canary.full_regression_canary);
        assert!(canary.args.contains(&"runtime-core".to_string()));
        assert!(targeted.args.contains(&"runtime-core".to_string()));

        let mut generated_observation = observation.clone();
        generated_observation.logical_path =
            "ROOT_0/crates/semantic-reasoning/src/generated_sem5_capability.rs".to_string();
        fs::write(
            source_dir.join("generated_sem5_capability.rs"),
            "pub fn generated_capability() {}\n",
        )
        .unwrap();
        let generated = core_validation_plan(&config, 2, &[generated_observation]).unwrap();
        assert_eq!(
            generated.validation_scope,
            "RUNTIME_CORE_REGRESSION_NO_TARGETABLE_MODULE_TESTS"
        );
        assert!(generated.targeted_test_filter.is_none());
        assert_eq!(generated.args.last().map(String::as_str), Some("--locked"));

        let mut historical_observation = observation.clone();
        historical_observation.logical_path =
            "ROOT_0/crates/semantic-reasoning/src/sem12/mod.rs".to_string();
        let historical = core_validation_plan(&config, 2, &[historical_observation]).unwrap();
        assert_eq!(
            historical.validation_scope,
            "FULL_HISTORICAL_REGRESSION_CANARY"
        );
        assert!(!historical.full_regression_canary);
        assert!(!historical.args.contains(&"runtime-core".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn targeted_validation_does_not_misread_fifty_passes_as_zero_tests() {
        assert!(targeted_test_filter_executed(
            "\nrunning 50 tests\n..................................................\ntest result: ok. 50 passed; 0 failed; 0 ignored; 0 measured\n"
        ));
        assert!(!targeted_test_filter_executed(
            "\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured\n"
        ));
    }

    #[test]
    fn superseded_high_observations_are_consumed_before_repository_validation() {
        let root = temp_root("superseded-high-observation");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let mut index = load_index(&config).unwrap();
        let logical_path = "ROOT_0/src/module.py".to_string();
        index.files.insert(
            logical_path.clone(),
            FileFingerprint {
                content_sha256: "b".repeat(64),
                bytes: 1,
                modified_ms: 2,
                extension: "py".to_string(),
                features: StructuralFeatures::default(),
            },
        );
        let current = LearningObservation {
            observation_id: "current".to_string(),
            work_event_id: Some("current-event".to_string()),
            logical_path: logical_path.clone(),
            content_sha256: "b".repeat(64),
            predecessor_content_sha256: Some("a".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 80,
            learning_value: LearningValue::High,
            reasons: vec!["current fixture".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 2,
        };
        let mut superseded = current.clone();
        superseded.observation_id = "superseded".to_string();
        superseded.work_event_id = Some("superseded-event".to_string());
        superseded.content_sha256 = "a".repeat(64);
        superseded.observed_at_ms = 1;
        let mut synthetic = current.clone();
        synthetic.observation_id = "synthetic".to_string();
        synthetic.work_event_id = None;
        synthetic.logical_path = "ROOT_0/.b_repository_validation/pass".to_string();
        synthetic.content_sha256 = "c".repeat(64);
        let mut observations = vec![superseded, current, synthetic];

        let consumed = consume_superseded_high_observations(
            &config,
            &mut state,
            &mut index,
            &mut observations,
        )
        .unwrap();

        assert_eq!(consumed, 1);
        assert_eq!(observations.len(), 2);
        assert!(observations
            .iter()
            .all(|observation| observation.observation_id != "superseded"));
        assert!(index.consumed_observation_ids.contains("superseded"));
        assert!(index.consumed_work_event_ids.contains("superseded-event"));
        assert!(!index.consumed_observation_ids.contains("current"));
        assert_eq!(state.redundant_observations_consumed, 1);
        fs::remove_dir_all(root).unwrap();
    }

    fn accepted_candidate(root: &Path) -> (CampaignFreeze, LearningCandidate, VerifierRequest) {
        let verifier = std::env::current_exe().unwrap();
        let verifier_sha256 = file_sha256(&verifier, 512 * 1024 * 1024).unwrap();
        let observation = LearningObservation {
            observation_id: "observation".to_string(),
            work_event_id: Some("event".to_string()),
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "f".repeat(64),
            predecessor_content_sha256: Some("0".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Pass,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures {
                assertion_tokens: 1,
                validation_tokens: 1,
                error_handling_tokens: 1,
                ..StructuralFeatures::default()
            },
            signals: vec![
                "DEFECT_REPAIR".to_string(),
                "VERIFIED_PASS".to_string(),
                "REGRESSION_EVIDENCE".to_string(),
            ],
            composition_roles: vec![
                "IMPLEMENTATION_REPAIR".to_string(),
                "REGRESSION_TEST".to_string(),
            ],
            learning_score: 70,
            learning_value: LearningValue::High,
            reasons: vec!["test fixture".to_string()],
            verification_evidence_sha256: vec!["a".repeat(64)],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![public_contract_delta_fixture()],
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let observation_sha256 = json_sha256(&observation).unwrap();
        let predecessor_memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 0,
            predecessor_sha256: None,
            lessons: Vec::new(),
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };
        let freeze = CampaignFreeze {
            schema: SUPERVISOR_SCHEMA.to_string(),
            campaign_id: "G0001-test".to_string(),
            generation: 1,
            predecessor_memory_sha256: json_sha256(&predecessor_memory).unwrap(),
            config_sha256: "b".repeat(64),
            observation_ids: vec!["observation".to_string()],
            observation_sha256: vec![observation_sha256],
            proposer_executable_sha256: "d".repeat(64),
            verifier_executable_sha256: verifier_sha256.clone(),
            seed: 7,
            budget_observations: 1,
            frozen_before_candidate: true,
            operator_selected_difficulty: false,
            human_difficulty_escalation_events: 0,
            created_at_ms: 1,
        };
        let freeze_sha256 = json_sha256(&freeze).unwrap();
        let lesson = build_lesson(std::slice::from_ref(&observation)).unwrap();
        let generative_cycle = run_generative_cycle(
            &predecessor_memory.generative,
            &generative_input(&lesson),
            freeze.seed,
        )
        .unwrap();
        let candidate = LearningCandidate {
            schema: SUPERVISOR_SCHEMA.to_string(),
            campaign_id: freeze.campaign_id.clone(),
            freeze_sha256: freeze_sha256.clone(),
            generation: 1,
            predecessor_memory_sha256: freeze.predecessor_memory_sha256.clone(),
            lesson,
            observation_ids: freeze.observation_ids.clone(),
            total_learning_score: 70,
            generative_cycle,
            raw_source_bytes: 0,
            exact_source_fragments: 0,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
            self_approval_events: 0,
            difficulty_escalation_events: 0,
        };
        let freeze_path = root.join("freeze.json");
        let candidate_path = root.join("candidate.json");
        write_immutable_json(&freeze_path, &freeze).unwrap();
        write_immutable_json(&root.join("predecessor_memory.json"), &predecessor_memory).unwrap();
        write_immutable_json(&root.join("observation_observation.json"), &observation).unwrap();
        write_immutable_json(&candidate_path, &candidate).unwrap();
        let request = VerifierRequest {
            schema: VERIFIER_SCHEMA.to_string(),
            freeze_path,
            candidate_path,
            expected_freeze_sha256: freeze_sha256,
            expected_candidate_sha256: json_sha256(&candidate).unwrap(),
            expected_verifier_sha256: verifier_sha256,
            minimum_learning_score: 45,
            max_observations: 32,
        };
        (freeze, candidate, request)
    }

    #[test]
    fn self_check_locks_constitutional_boundaries() {
        let check = self_check();
        assert!(check.pass);
        assert!(!check.proposer_cannot_self_approve);
        assert!(check.raw_source_retention_forbidden);
        assert!(check.network_and_llm_disabled);
        assert!(check.plateau_difficulty_escalation_disabled);
        assert!(check.frozen_observation_reconstruction_enabled);
        assert!(check.bound_pass_evidence_required);
        assert!(check.evaluator_mutation_self_audit_enabled);
        assert!(check.classifier_refinement_requires_capability_evidence);
        assert!(check.classifier_refinement_delta_ledger_enabled);
        assert!(check.source_patch_diagnostics_use_recent_engine_window);
        assert!(check.source_synthesis_exhaustion_is_capability_gap);
        assert!(check.rust_source_ast_modeling_enabled);
        assert!(check.syntactic_call_and_data_flow_modeling_enabled);
        assert!(check.structural_postcondition_derivation_enabled);
        assert!(check.universal_source_edit_atoms_enabled);
        assert!(check.structural_repair_replay_gate_enabled);
        assert!(check.autonomous_compiler_diagnostic_discovery_enabled);
        assert!(check.typed_grammar_composition_enabled);
        assert!(check.public_counterexample_guided_revision_enabled);
        assert!(check.same_attempt_counterexample_revision_enabled);
        assert!(check.same_attempt_revision_requires_exact_rollback);
        assert!(check.validation_process_tree_termination_enabled);
        assert!(check.validation_output_is_bounded);
        assert!(check.successful_edit_composition_learning_enabled);
        assert!(check.bounded_compiler_diagnostic_cache_enabled);
        assert!(check.evaluator_generation_evolution_enabled);
        assert!(check.prediction_before_composition_enabled);
        assert!(check.valuable_combination_memory_enabled);
        assert!(check.generative_memory_self_application_enabled);
        assert!(check.core_self_approval_enabled);
        assert!(check.autonomous_source_patch_install_enabled);
        assert!(check.source_patch_rollback_enabled);
        assert!(check.promoted_lessons_drive_executable_repairs);
        assert!(!check.text_only_knowledge_is_capability_authority);
        assert!(check.executable_knowledge_gate_enabled);
        assert!(!check.static_canary_replay_is_knowledge_growth);
        assert!(check.bounded_failure_retry_enabled);
        assert!(check.successful_solution_learning_enabled);
        assert!(check.admitted_failure_revisit_after_growth_enabled);
        assert_eq!(
            check.source_repair_engine_revision,
            SOURCE_REPAIR_ENGINE_REVISION
        );
        assert!(check.dynamic_self_weakness_discovery_enabled);
        assert!(check.generalized_change_ir_bound_to_source_edits);
        assert!(check.validation_counterexamples_drive_candidate_ranking);
        assert!(check.multi_generation_self_application_lineage_enabled);
        assert!(check.fixed_sem9_toggle_replay_forbidden);
        assert!(check.runtime_repair_counter_requires_executed_action);
        assert!(check.diagnostic_outcome_requires_action_output_consumption);
        assert!(check.diagnostic_productivity_requires_current_executable_intervention);
        assert!(check.unbound_capability_gap_state_deduplicated);
        assert!(check.test_only_evaluator_cohort_validation_enabled);
        assert!(check.validation_receipt_identity_excludes_generation);
        assert!(check.verification_only_generation_promotion_forbidden);
        assert!(check.verification_only_false_tip_auto_recovery);
        assert!(check.source_discovery_applicability_precedes_value_gate);
        assert!(check.identical_source_discovery_state_deduplicated);
        assert!(check.diagnostic_opportunity_kind_separated_from_executability);
        assert!(check.self_healing_candidates_route_to_atomic_installer);
        assert!(check.repository_candidate_requires_authoritative_install_authority);
        assert!(check.repository_install_transaction_recovery_enabled);
        assert!(check.authoritative_repository_validation_before_learning_enabled);
        assert!(check.integrated_program_ir_lowers_to_compiled_rust);
        assert!(check.installed_compositions_are_runtime_callable);
        assert!(check.typed_lowering_preserves_installed_capability_registry);
        assert!(check.generated_capabilities_dispatch_by_program_hash);
        assert!(check.contextual_typed_task_generation_enabled);
        assert!(check.verified_program_artifact_frontier_tracked);
        assert!(check.wrapper_composition_count_excluded_from_capability_count);
        assert!(check.canonical_grammar_role_operations_enabled);
        assert!(check.same_type_call_role_permutations_bounded);
        assert!(check.symmetric_state_transform_compilation_enabled);
        assert!(check.accepted_sem5_compositions_route_to_installer);
        assert!(check.sem5_installer_requires_typed_plan_and_exact_goal);
        assert!(check.active_binaries_forbid_proposal_only_exit);
        assert!(check.executable_improvement_operator_repository_enabled);
        assert!(check.improvement_operator_repository_requires_source_synthesis_payload);
        assert!(check.program_execution_profile_is_not_synthesis_knowledge);
        assert!(check.source_proposal_composition_and_ranking_owned_by_rust_kernel);
        assert!(check.source_proposal_competitors_bounded_to_three);
        assert!(check.all_language_source_proposals_share_one_rust_kernel);
        assert!(check.required_composition_groups_owned_by_rust_kernel);
        assert!(check.language_frontends_cannot_rank_or_merge_source_proposals);
        assert!(check.source_generators_submit_bounded_proposal_batches);
        assert!(check.source_kernel_is_first_candidate_selection_authority);
        assert!(check.compiler_applicability_authority_is_typed);
        assert!(check.raw_compiler_applicability_is_metadata_only);
        assert!(check.generative_execution_dispatch_uses_typed_rust_ir);
        assert!(check.generative_stage_strings_are_metadata_only);
        assert!(check.python_host_failure_ontology_owned_by_rust_kernel);
        assert!(check.fullstack_knowledge_uses_typed_executable_transitions);
        assert!(check.fullstack_text_contracts_are_metadata_only);
        assert!(check.successful_operators_are_content_addressed);
        assert!(check.typed_improvement_operator_execution_required);
        assert!(check.operator_repository_requires_executed_receipt);
        assert!(check.generative_substrate_capacity_isolated);
        assert!(check.saturated_substrate_routes_without_difficulty_escalation);
        assert!(check.compound_growth_runs_inside_supervisor_loop);
        assert!(check.compound_repository_authority_is_supervisor_owned);
        assert!(check.compound_growth_requires_typed_hashed_evidence);
        assert!(check.compound_typed_goal_functional_composition_enabled);
        assert!(check.compound_typed_goal_requires_public_causal_join);
        assert!(check.compound_typed_goal_effects_fail_closed);
        assert!(check.verified_compound_programs_are_promoted_to_memory);
        assert!(check.generative_prediction_is_selection_only);
        assert!(check.cross_family_operator_transfer_changes_candidate_priority);
        assert!(check.repository_guided_outcomes_are_causally_tracked);
        assert!(check.evaluator_expansion_requires_new_challenge_capability);
        assert!(check.operator_stop_survives_self_update);
        assert!(check.workspace_freeze_during_patch_validation);
        assert!(check.performance_aware_self_inspection);
        assert!(check.predicted_utility_source_gate);
        assert!(check.staged_source_validation);
        assert!(check.runtime_core_static_validation_surface_enabled);
        assert!(check.historical_regression_canary_separated);
        assert!(check.warm_incremental_validation_cache_enabled);
        assert!(check.adaptive_idle_polling);
        assert!(check.mixed_production_file_role_detection);
        assert!(check.semantic_duplicate_promotion_blocked);
        assert!(check.measured_performance_evidence_supported);
        assert!(check.contextual_generative_exploration_enabled);
        assert!(check.redundant_reuse_excluded_from_growth);
        assert!(check.heuristic_composition_value_excluded_from_frontier);
        assert!(check.behavioral_evidence_required_for_generative_self_application);
        assert!(check.behavioral_composition_execution_enabled);
        assert!(check.redundant_generative_verifier_search_disabled);
        assert!(check.intrinsic_curiosity_requires_executable_hypotheses);
        assert!(check.intrinsic_reward_requires_verified_frontier);
        assert!(check.intrinsic_reward_requires_independent_promotion);
        assert!(check.intrinsic_exploration_is_bounded);
        assert!(!check.mutual_recursive_growth_observed);
    }

    #[test]
    fn legacy_frozen_config_defaults_to_bounded_repository_install_without_hash_churn() {
        let config = GrowthSupervisorConfig::bounded_default(
            PathBuf::from("state"),
            PathBuf::from("watched"),
            PathBuf::from("verifier"),
        );
        assert!(config.repository_mutation.enabled);
        assert_eq!(config.repository_mutation.max_installations_per_step, 1);
        let serialized = serde_json::to_value(&config).unwrap();
        assert!(serialized.get("repository_mutation").is_none());
        let legacy: GrowthSupervisorConfig = serde_json::from_value(serialized).unwrap();
        assert!(legacy.repository_mutation.enabled);
        assert_eq!(legacy.repository_mutation.max_installations_per_step, 1);

        let mut disabled = legacy;
        disabled.repository_mutation.enabled = false;
        assert!(serde_json::to_value(disabled)
            .unwrap()
            .get("repository_mutation")
            .is_some());
    }

    fn classifier_refinement_lesson(
        performance_metrics: Vec<PerformanceMetricEvidence>,
    ) -> LearnedCompositionLesson {
        LearnedCompositionLesson {
            lesson_id: "classifier-refinement-lesson".to_string(),
            evidence_observation_sha256: vec!["a".repeat(64)],
            work_kinds: vec![WorkKind::DefectRepair],
            diagnostic_signals: vec!["VERIFIED_PASS".to_string(), "DEFECT_REPAIR".to_string()],
            composition_recipe: vec!["IMPLEMENTATION_REPAIR".to_string()],
            applicability: vec!["test".to_string()],
            verification_obligations: vec!["regression".to_string()],
            performance_metrics,
            public_contract_deltas: Vec::new(),
            learning_score: 80,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        }
    }

    #[test]
    fn classifier_suppresses_accepted_but_capability_neutral_reinforcement() {
        let lesson = classifier_refinement_lesson(Vec::new());
        let mut classifier = ClassifierMemory::default();

        refine_classifier_from_capability_outcome(
            &mut classifier,
            7,
            &lesson,
            &["POLICY_SIGNAL".to_string()],
            false,
            None,
        );

        assert!(classifier.signal_weights.is_empty());
        assert_eq!(classifier.outcome_bound_refinements, 0);
        assert_eq!(classifier.unsupported_refinements_suppressed, 1);
        let event = classifier.refinement_events.last().unwrap();
        assert!(!event.applied);
        assert!(!event.behavioral_frontier_advance);
        assert!(!event.measured_performance_gain);
        assert!(event
            .weight_deltas
            .iter()
            .all(|delta| delta.before == delta.after));
    }

    #[test]
    fn classifier_records_evidence_bound_behavioral_weight_deltas() {
        let lesson = classifier_refinement_lesson(Vec::new());
        let mut classifier = ClassifierMemory::default();
        classifier
            .signal_weights
            .insert("DEFECT_REPAIR".to_string(), 2);
        let receipt_sha256 = "b".repeat(64);

        refine_classifier_from_capability_outcome(
            &mut classifier,
            8,
            &lesson,
            &["POLICY_SIGNAL".to_string()],
            true,
            Some(&receipt_sha256),
        );

        assert_eq!(classifier.signal_weights["DEFECT_REPAIR"], 3);
        assert_eq!(classifier.signal_weights["POLICY_SIGNAL"], 1);
        assert_eq!(classifier.outcome_bound_refinements, 1);
        assert_eq!(classifier.unsupported_refinements_suppressed, 0);
        let event = classifier.refinement_events.last().unwrap();
        assert!(event.applied);
        assert!(event.behavioral_frontier_advance);
        assert_eq!(
            event.behavioral_verification_sha256.as_deref(),
            Some(receipt_sha256.as_str())
        );
        assert!(event.weight_deltas.iter().any(|delta| {
            delta.signal == "DEFECT_REPAIR" && delta.before == 2 && delta.after == 3
        }));
    }

    #[test]
    fn classifier_rejects_metric_only_performance_as_executable_growth() {
        let lesson = classifier_refinement_lesson(vec![PerformanceMetricEvidence {
            metric: "latency_ns".to_string(),
            before: 100,
            after: 80,
            lower_is_better: true,
            evidence_sha256: "c".repeat(64),
            executable_knowledge: None,
        }]);
        let mut classifier = ClassifierMemory::default();

        refine_classifier_from_capability_outcome(&mut classifier, 9, &lesson, &[], false, None);

        assert_eq!(classifier.outcome_bound_refinements, 0);
        assert!(!classifier.refinement_events[0].measured_performance_gain);
        assert!(!classifier.signal_weights.contains_key("VERIFIED_PASS"));
    }

    fn executable_performance_operator_fixture() -> ImprovementOperatorIR {
        let mut operator = ImprovementOperatorIR {
            schema: crate::autonomous_source_mutation::IMPROVEMENT_OPERATOR_MEMORY_SCHEMA
                .to_string(),
            operator_id: String::new(),
            weakness_evidence_kind:
                crate::generalized_self_application::WeaknessEvidenceKind::StructuralSourceSmell,
            generator_kind: ImprovementOperatorGeneratorKind::KnownStructuralRewrite,
            executable_payload: Some(
                crate::autonomous_source_mutation::ExecutableImprovementOperatorPayloadIR::KnownStructuralRewrite {
                    rewrite: crate::autonomous_source_mutation::KnownStructuralRewriteIR::TypedIsMultipleOf,
                },
            ),
            solution_strategy_family: "TYPED_IS_MULTIPLE_OF".to_string(),
            edit_atom_kinds: vec!["REPLACE".to_string()],
            structural_postcondition_class: "FEW".to_string(),
            validation_contract: vec!["STRUCTURAL_REPLAY".to_string()],
        };
        operator.operator_id = sha256(&serde_json::to_vec(&operator).unwrap());
        operator
    }

    #[test]
    fn executable_performance_operator_drives_growth_and_generative_input() {
        let operator = executable_performance_operator_fixture();
        validate_improvement_operator(&operator).unwrap();
        let lesson = classifier_refinement_lesson(vec![PerformanceMetricEvidence {
            metric: "latency_ns".to_string(),
            before: 100,
            after: 80,
            lower_is_better: true,
            evidence_sha256: "d".repeat(64),
            executable_knowledge: Some(ExecutablePerformanceKnowledgeIR {
                schema: EXECUTABLE_PERFORMANCE_KNOWLEDGE_SCHEMA.to_string(),
                predecessor_content_sha256: "a".repeat(64),
                candidate_content_sha256: "b".repeat(64),
                improvement_operator: operator.clone(),
            }),
        }]);
        assert!(lesson_has_executable_knowledge(&lesson));
        let input = generative_input(&lesson);
        assert!(input.measured_performance_gain);
        assert_eq!(input.executable_performance_operators, vec![operator]);

        let mut classifier = ClassifierMemory::default();
        refine_classifier_from_capability_outcome(&mut classifier, 10, &lesson, &[], false, None);
        assert_eq!(classifier.outcome_bound_refinements, 1);
        assert!(classifier.refinement_events[0].measured_performance_gain);
    }

    #[test]
    fn default_evaluator_memory_preserves_legacy_growth_memory_hash() {
        #[derive(Serialize)]
        struct LegacyGrowthMemory<'a> {
            schema: &'a str,
            generation: u64,
            predecessor_sha256: Option<String>,
            lessons: Vec<LearnedCompositionLesson>,
            classifier: ClassifierMemory,
        }
        let current = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 0,
            predecessor_sha256: None,
            lessons: Vec::new(),
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };
        let legacy = LegacyGrowthMemory {
            schema: SUPERVISOR_SCHEMA,
            generation: 0,
            predecessor_sha256: None,
            lessons: Vec::new(),
            classifier: ClassifierMemory::default(),
        };
        assert_eq!(
            json_sha256(&current).unwrap(),
            json_sha256(&legacy).unwrap()
        );
    }

    #[test]
    fn raw_source_or_symlink_policy_is_rejected() {
        let root = temp_root("invalid-policy");
        let (_, mut config) = test_config(&root);
        config.observation.retain_raw_source = true;
        assert_eq!(
            validate_config(&config),
            Err("RAW_SOURCE_OR_SYMLINK_OBSERVATION_FORBIDDEN".to_string())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn first_scan_is_baseline_not_learning() {
        let root = temp_root("baseline");
        let (config_path, config) = test_config(&root);
        fs::write(
            config.watched_roots[0].join("lib.rs"),
            "pub fn existing() {}\n",
        )
        .unwrap();
        initialize(&config_path).unwrap();
        let report = supervisor_step(&config_path).unwrap();
        assert!(report.baseline_created);
        assert_eq!(report.observations_created, 0);
        assert_eq!(report.generation, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_policy_selection_counts_are_quarantined_from_executed_repairs() {
        let root = temp_root("runtime-repair-counter-migration");
        let (config_path, _) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.runtime_self_repair_counter_contract_revision = 0;
        state.runtime_self_repairs_activated = 340;
        state.diagnostic_policy.outcome_causal_contract_revision = 0;
        state.diagnostic_policy.outcome_bound_selections = 87;
        state.diagnostic_policy.productive_outcome_events = 13;
        state.diagnostic_policy.failed_outcome_events = 73;

        ensure_runtime_repair_counter_contract(&mut state);

        assert_eq!(state.runtime_self_repairs_activated, 0);
        assert_eq!(state.legacy_unbound_runtime_self_repair_activations, 340);
        assert_eq!(state.diagnostic_policy.outcome_bound_selections, 0);
        assert_eq!(
            state
                .diagnostic_policy
                .legacy_unbound_outcome_bound_selections,
            87
        );
        assert_eq!(
            state
                .diagnostic_policy
                .legacy_unbound_productive_outcome_events,
            13
        );
        assert_eq!(
            state.diagnostic_policy.legacy_unbound_failed_outcome_events,
            73
        );
        assert_eq!(
            state.runtime_self_repair_counter_contract_revision,
            RUNTIME_REPAIR_COUNTER_CONTRACT_REVISION
        );
        assert_eq!(state.diagnostic_policy.outcome_causal_contract_revision, 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_preinstallation_events_are_quarantined_from_execution_failures() {
        let root = temp_root("installed-execution-counter-migration");
        let (config_path, _) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.installed_execution_counter_contract_revision = 0;
        state.installed_composite_capability_execution_events = 11;
        state.installed_composite_capability_execution_failures = 2;
        state.last_installed_composite_execution_sha256 = Some("a".repeat(64));

        ensure_installed_execution_counter_contract(&mut state);

        assert_eq!(state.installed_composite_capability_execution_events, 0);
        assert_eq!(state.installed_composite_capability_execution_failures, 0);
        assert_eq!(
            state.legacy_unbound_installed_composite_execution_events,
            11
        );
        assert_eq!(
            state.legacy_unbound_installed_composite_execution_failures,
            2
        );
        assert!(state.last_installed_composite_execution_sha256.is_none());
        assert_eq!(
            state.installed_execution_counter_contract_revision,
            INSTALLED_EXECUTION_COUNTER_CONTRACT_REVISION
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepted_but_not_yet_installed_capability_is_not_an_execution_failure() {
        let root = temp_root("pending-capability-not-execution-failure");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let mut memory = load_memory(&config, 0).unwrap();
        let installed = crate::generated_sem5_capability::generated_capability_hashes();
        let mut selected = None;
        for ordinal in 0..128_u64 {
            let input = GenerativeInput {
                source_lesson_id: format!("pending-install-{ordinal}"),
                diagnostic_signals: vec![format!("PENDING_INSTALL_CONTEXT_{ordinal}")],
                observed_composition_roles: vec![
                    "INVARIANT_CHECK".to_string(),
                    "REGRESSION_TEST".to_string(),
                ],
                learning_score: 80,
                verification_evidence_count: 1,
                measured_performance_gain: false,
                typed_behavior_goals: vec![typed_behavior_goal_fixture(&format!(
                    "pending-install-goal-{ordinal}"
                ))],
                executable_performance_operators: Vec::new(),
            };
            let result = run_generative_cycle(&GenerativeGrowthMemory::default(), &input, ordinal)
                .expect("behavioral composition");
            let artifact = result
                .behavioral_execution_receipt
                .as_ref()
                .and_then(|receipt| receipt.composite_artifact_sha256.as_deref())
                .expect("verified artifact");
            if !installed.contains(&artifact) {
                selected = Some((input, result));
                break;
            }
        }
        let (input, result) = selected.expect("context outside installed registry");
        memory.generative =
            promote_generative_cycle(&GenerativeGrowthMemory::default(), &input, &result)
                .expect("promote pending capability");
        assert!(memory
            .generative
            .accepted_compositions
            .iter()
            .any(|accepted| !accepted.verified_typed_behavior_goals.is_empty()));

        let mut legacy_text_only = memory.clone();
        for accepted in &mut legacy_text_only.generative.accepted_compositions {
            accepted.execution_plan = None;
            accepted.verified_typed_behavior_goals.clear();
        }
        assert!(accepted_sem5_artifact_contexts(&legacy_text_only).is_empty());
        assert!(
            pending_sem5_composition_candidates(&config, &legacy_text_only)
                .expect("legacy text-only lookup")
                .is_empty()
        );

        let pending_candidate = pending_sem5_composition_candidates(&config, &memory)
            .expect("pending lookup")
            .into_iter()
            .next()
            .expect("pending artifact candidate");
        let pending_context = accepted_sem5_artifact_contexts(&memory)
            .into_iter()
            .find_map(|(artifact, context)| {
                (artifact == pending_candidate.program_ir_sha256).then_some(context)
            })
            .expect("pending artifact context");
        assert!(!installed.contains(&pending_candidate.program_ir_sha256.as_str()));
        assert!(memory
            .generative
            .accepted_compositions
            .iter()
            .any(|accepted| {
                accepted
                    .verified_artifact_contexts
                    .get(&pending_candidate.program_ir_sha256)
                    == Some(&pending_context)
            }));

        let predecessor_source = fs::read_to_string(
            config
                .source_mutation
                .source_root
                .join("crates/semantic-reasoning/src/generated_sem5_capability.rs"),
        )
        .ok();
        assert_eq!(
            reconcile_verified_generative_typed_operators(&config, &memory)
                .expect("promote verified operator"),
            1
        );
        assert_eq!(
            load_source_bound_improvement_operators(&config)
                .expect("authorized dynamic operator")
                .len(),
            1
        );
        assert!(pending_sem5_composition_candidates(&config, &memory)
            .expect("dynamic operator suppresses static rebuild")
            .is_empty());
        let successor_source = fs::read_to_string(
            config
                .source_mutation
                .source_root
                .join("crates/semantic-reasoning/src/generated_sem5_capability.rs"),
        )
        .ok();
        assert_eq!(predecessor_source, successor_source);

        let observation =
            revalidate_installed_composite_capability(&config, &mut state, &memory).unwrap();

        assert!(observation.is_none());
        assert_eq!(state.installed_composite_capability_execution_events, 0);
        assert_eq!(state.installed_composite_capability_execution_failures, 0);
        assert_eq!(state.self_repair_capability_gaps, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pass_event_recorded_after_scan_replays_against_unchanged_indexed_content() {
        let root = temp_root("event-replay");
        let (config_path, config) = test_config(&root);
        let source = config.watched_roots[0].join("lib.rs");
        fs::write(&source, "pub fn value() -> u8 { 1 }\n").unwrap();
        supervisor_step(&config_path).expect("baseline");
        let evidence = root.join("verification.log");
        fs::write(&evidence, "test result: ok\n").unwrap();

        record_work_event(
            &config_path,
            WorkEvent {
                event_id: "verified-after-scan".to_string(),
                actor: WorkActor::LocalTool,
                kind: WorkKind::DefectRepair,
                paths: vec![source],
                outcome: WorkOutcome::Pass,
                summary: "deterministic regression passed".to_string(),
                evidence_sha256: vec![],
                evidence_artifacts: vec![evidence],
                performance_metrics: Vec::new(),
                public_contract_deltas: Vec::new(),
                occurred_at_ms: 1,
            },
        )
        .expect("event");
        let report = supervisor_step(&config_path).expect("replay");
        assert_eq!(report.observations_created, 1);
        assert_eq!(report.high_value_observations, 1);
        assert_eq!(
            report.last_internal_bottleneck.as_deref(),
            Some("WORK_EVENT_ATTRIBUTION_GAP")
        );
        assert_eq!(report.runtime_self_repairs_activated, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pass_event_without_a_real_evidence_artifact_is_rejected() {
        let root = temp_root("unbound-pass");
        let (config_path, config) = test_config(&root);
        let source = config.watched_roots[0].join("lib.rs");
        fs::write(&source, "pub fn value() -> u8 { 1 }\n").unwrap();
        initialize(&config_path).unwrap();
        let error = record_work_event(
            &config_path,
            WorkEvent {
                event_id: "unbound-pass".to_string(),
                actor: WorkActor::LocalTool,
                kind: WorkKind::Verification,
                paths: vec![source],
                outcome: WorkOutcome::Pass,
                summary: "claimed pass".to_string(),
                evidence_sha256: vec![],
                evidence_artifacts: vec![],
                performance_metrics: Vec::new(),
                public_contract_deltas: Vec::new(),
                occurred_at_ms: 1,
            },
        )
        .unwrap_err();
        assert_eq!(error, "PASS_EVENT_REQUIRES_BOUND_EVIDENCE_ARTIFACT");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn evidence_aware_selection_prevents_score_prefix_starvation() {
        let root = temp_root("evidence-aware-selection");
        let (_config_path, mut config) = test_config(&root);
        config.resources.max_observations_per_campaign = 1;
        let mut implementation = accepted_candidate(&root).1.lesson;
        implementation.diagnostic_signals = vec!["DEFECT_REPAIR".to_string()];
        let high_without_evidence = LearningObservation {
            observation_id: "high".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/high.rs".to_string(),
            content_sha256: "1".repeat(64),
            predecessor_content_sha256: None,
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: implementation.diagnostic_signals,
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 90,
            learning_value: LearningValue::High,
            reasons: Vec::new(),
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let mut verified = high_without_evidence.clone();
        verified.observation_id = "verified".to_string();
        verified.learning_score = 80;
        verified.work_outcome = WorkOutcome::Pass;
        verified.signals.push("VERIFIED_PASS".to_string());
        verified.verification_evidence_sha256 = vec!["a".repeat(64)];
        verified.public_contract_deltas = vec![public_contract_delta_fixture()];
        let selected =
            selected_campaign_observations(&config, &[high_without_evidence, verified.clone()]);
        assert_eq!(selected, vec![verified]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validated_text_only_cohort_is_preserved_but_removed_from_active_queue() {
        let root = temp_root("defer-validated-text-only-cohort");
        let (_, config) = test_config(&root);
        let mut index = FileIndex::default();
        let implementation = LearningObservation {
            observation_id: "implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "1".repeat(64),
            predecessor_content_sha256: None,
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 100,
            learning_value: LearningValue::High,
            reasons: Vec::new(),
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let mut verification = implementation.clone();
        verification.observation_id = "verification".to_string();
        verification.logical_path = "ROOT_0/.b_validation/pass".to_string();
        verification.work_kind = WorkKind::Verification;
        verification.work_outcome = WorkOutcome::Pass;
        verification.signals = vec!["VERIFIED_PASS".to_string()];
        verification.verification_evidence_sha256 = vec!["a".repeat(64)];
        verification.learning_score = 80;
        let mut observations = vec![implementation, verification];

        let deferred =
            defer_verified_non_executable_cohort(&config, 7, &mut index, &mut observations)
                .unwrap();

        assert_eq!(deferred, 2);
        assert!(observations.is_empty());
        assert_eq!(index.consumed_observation_ids.len(), 2);
        assert_eq!(
            fs::read_dir(config.state_dir.join("diagnostics"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("deferred_cohort_"))
                .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plateau_input_preserves_executable_contract_transport() {
        let delta = public_contract_delta_fixture();
        let lesson = |id: &str| LearnedCompositionLesson {
            lesson_id: id.to_string(),
            evidence_observation_sha256: vec![sha256(id.as_bytes())],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec!["VERIFIED_PASS".to_string()],
            composition_recipe: vec!["PROGRAM_COMPOSITION".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta.clone()],
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        let memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 2,
            predecessor_sha256: None,
            lessons: vec![lesson("LESSON-A"), lesson("LESSON-B")],
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };

        let (input, deltas) = plateau_generative_input(&memory)
            .unwrap()
            .expect("executable plateau input");
        assert_eq!(deltas, vec![delta.clone()]);
        assert_eq!(input.typed_behavior_goals, delta.typed_behavior_goals);
    }

    #[test]
    fn plateau_input_places_a_new_executable_compound_program_before_components() {
        let mut producer_delta = PublicContractDeltaIR {
            schema: PUBLIC_CONTRACT_DELTA_SCHEMA.to_string(),
            delta_id: "verified-executable-to-gate".to_string(),
            observed_behavior: "verification is treated as execution".to_string(),
            expected_behavior: "unverified or executable cohorts remain eligible".to_string(),
            target_symbols: vec!["crate::engine::verified_queue_gate".to_string()],
            typed_behavior_goals: vec![boolean_gate_goal_fixture("verified_queue_gate")],
            provenance: vec!["PUBLIC_OBSERVATION".to_string()],
        };
        let mut consumer_delta = PublicContractDeltaIR {
            schema: PUBLIC_CONTRACT_DELTA_SCHEMA.to_string(),
            delta_id: "gate-to-typed-transport".to_string(),
            observed_behavior: "the typed value ignores its condition".to_string(),
            expected_behavior: "the typed value is transported only when allowed".to_string(),
            target_symbols: vec!["crate::engine::conditional_transport".to_string()],
            typed_behavior_goals: vec![conditional_string_transport_goal_fixture(
                "conditional_string_transport",
            )],
            provenance: vec!["PUBLIC_OBSERVATION".to_string()],
        };
        bind_public_contract_delta_fixture(&mut producer_delta);
        bind_public_contract_delta_fixture(&mut consumer_delta);
        let lesson = |id: &str, delta: PublicContractDeltaIR| LearnedCompositionLesson {
            lesson_id: id.to_string(),
            evidence_observation_sha256: vec![sha256(id.as_bytes())],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec!["VERIFIED_PASS".to_string()],
            composition_recipe: vec!["PROGRAM_COMPOSITION".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta],
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        let memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 2,
            predecessor_sha256: None,
            lessons: vec![
                lesson("PRODUCER", producer_delta.clone()),
                lesson("CONSUMER", consumer_delta.clone()),
            ],
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };

        let (input, base_deltas) = plateau_generative_input(&memory)
            .unwrap()
            .expect("compound plateau input");
        let compound = input
            .typed_behavior_goals
            .first()
            .expect("compound goal is prioritized");
        assert!(compound.goal_id.starts_with("compound_"));
        assert!(input
            .diagnostic_signals
            .contains(&"COMPOUND_TYPED_GOAL_DERIVED".to_string()));
        assert!(input
            .observed_composition_roles
            .contains(&"TYPED_GOAL_FUNCTIONAL_COMPOSITION".to_string()));

        let compound_receipt = crate::integrated_development::execute_typed_behavior_goal_canary(
            &"a".repeat(64),
            compound,
        )
        .unwrap();
        assert_eq!(
            compound_receipt.cases_passed,
            compound_receipt.cases_executed
        );
        for component in [
            &producer_delta.typed_behavior_goals[0],
            &consumer_delta.typed_behavior_goals[0],
        ] {
            let component_receipt =
                crate::integrated_development::execute_typed_behavior_goal_canary(
                    &"b".repeat(64),
                    component,
                )
                .unwrap();
            assert_ne!(
                compound_receipt.program_ir_sha256,
                component_receipt.program_ir_sha256
            );
        }

        let probe_cycle =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &input, 7).unwrap();
        assert!(probe_cycle.frontier_advance);
        assert!(probe_cycle
            .behavioral_execution_receipt
            .as_ref()
            .is_some_and(|receipt| receipt.verified_artifacts.iter().any(|artifact| {
                artifact
                    .typed_behavior_goal
                    .as_ref()
                    .is_some_and(|goal| goal.goal_id.starts_with("compound_"))
            })));
        let promotion_deltas =
            plateau_promotion_contract_deltas(&base_deltas, &probe_cycle).unwrap();
        assert!(promotion_deltas[0]
            .delta_id
            .starts_with("compound-program-"));
        assert!(promotion_deltas[0]
            .typed_behavior_goals
            .iter()
            .all(|goal| goal.goal_id.starts_with("compound_")));

        let promoted_lesson = LearnedCompositionLesson {
            lesson_id: "PROMOTED-COMPOUND-PROGRAM".to_string(),
            evidence_observation_sha256: vec![probe_cycle
                .behavioral_verification_sha256
                .clone()
                .unwrap()],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec!["BEHAVIORALLY_VERIFIED_NOVEL_ARTIFACT".to_string()],
            composition_recipe: vec!["PROGRAM_COMPOSITION".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: promotion_deltas,
            learning_score: 95,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        let campaign_input = generative_input(&promoted_lesson);
        assert!(campaign_input.typed_behavior_goals[0]
            .goal_id
            .starts_with("compound_"));
        let campaign_cycle =
            run_generative_cycle(&GenerativeGrowthMemory::default(), &campaign_input, 7).unwrap();
        let promoted = promote_generative_cycle(
            &GenerativeGrowthMemory::default(),
            &campaign_input,
            &campaign_cycle,
        )
        .unwrap();
        assert!(promoted.accepted_compositions.iter().any(|composition| {
            composition
                .verified_typed_behavior_goals
                .values()
                .any(|goal| goal.goal_id.starts_with("compound_"))
        }));
    }

    #[test]
    fn intrinsic_curiosity_enumerates_multiple_bounded_executable_hypotheses() {
        let root = temp_root("intrinsic-curiosity-hypotheses");
        let (config_path, _) = test_config(&root);
        let state = initialize(&config_path).unwrap();
        let delta = public_contract_delta_fixture();
        let lesson = |id: &str, role: &str| LearnedCompositionLesson {
            lesson_id: id.to_string(),
            evidence_observation_sha256: vec![sha256(id.as_bytes())],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec![format!("SIGNAL_{id}"), "VERIFIED_PASS".to_string()],
            composition_recipe: vec![role.to_string(), "REGRESSION_TEST".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta.clone()],
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        let memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 4,
            predecessor_sha256: None,
            lessons: vec![
                lesson("A", "PREDICT"),
                lesson("B", "COMPOSE"),
                lesson("C", "VERIFY"),
                lesson("D", "REVISE"),
            ],
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };

        let candidates = plateau_curiosity_candidates(&state, &memory).unwrap();
        assert_eq!(candidates.len(), 10);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.hypothesis.hypothesis_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            candidates.len()
        );
        assert!(candidates.iter().all(|candidate| {
            (2..=3).contains(&candidate.hypothesis.lesson_ids.len())
                && candidate.hypothesis.executable_goal_count > 0
                && !candidate.input.typed_behavior_goals.is_empty()
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn intrinsic_curiosity_rotates_hypotheses_without_external_work() {
        let root = temp_root("intrinsic-curiosity-rotation");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let delta = public_contract_delta_fixture();
        let lesson = |id: &str, role: &str| LearnedCompositionLesson {
            lesson_id: id.to_string(),
            evidence_observation_sha256: vec![sha256(id.as_bytes())],
            work_kinds: vec![WorkKind::CapabilitySynthesis],
            diagnostic_signals: vec![format!("SIGNAL_{id}"), "VERIFIED_PASS".to_string()],
            composition_recipe: vec![role.to_string(), "REGRESSION_TEST".to_string()],
            applicability: vec!["BOUND_CONTEXT".to_string()],
            verification_obligations: vec!["BEHAVIORAL_CANARY".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta.clone()],
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        };
        let memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 3,
            predecessor_sha256: None,
            lessons: vec![
                lesson("A", "PREDICT"),
                lesson("B", "COMPOSE"),
                lesson("C", "VERIFY"),
            ],
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };

        let mut observation_ids = Vec::new();
        for _ in 0..2 {
            if let Some(observation) =
                plateau_generative_probe_observation(&config, &mut state, &memory).unwrap()
            {
                persist_scan_observations(&config, std::slice::from_ref(&observation)).unwrap();
                observation_ids.push(observation.observation_id.clone());
                assert!(observation
                    .signals
                    .contains(&"AUTONOMOUS_INTRINSIC_CURIOSITY".to_string()));
            }
        }

        assert_eq!(state.intrinsic_drive.hypotheses_attempted, 2);
        assert_eq!(state.intrinsic_drive.pending_attempts.len(), 2);
        assert_eq!(state.intrinsic_drive.hypotheses_succeeded, 0);
        assert_eq!(state.intrinsic_drive.intrinsic_reward_events, 0);
        assert_eq!(observation_ids.len(), 2);
        assert_eq!(
            resolve_intrinsic_observation_outcomes(
                &config,
                &mut state,
                std::slice::from_ref(&observation_ids[0]),
                true,
            )
            .unwrap(),
            1
        );
        assert_eq!(state.intrinsic_drive.hypotheses_succeeded, 1);
        assert_eq!(state.intrinsic_drive.intrinsic_reward_events, 1);
        assert_eq!(
            resolve_intrinsic_observation_outcomes(
                &config,
                &mut state,
                std::slice::from_ref(&observation_ids[1]),
                false,
            )
            .unwrap(),
            1
        );
        assert_eq!(state.intrinsic_drive.hypotheses_failed, 1);
        assert_eq!(state.intrinsic_drive.pending_attempts.len(), 0);
        assert_eq!(
            fs::read_dir(config.state_dir.join("generative_plateau_probes"))
                .unwrap()
                .filter_map(Result::ok)
                .count(),
            2
        );
        assert!(state.intrinsic_drive.is_valid());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unchanged_files_reuse_index_without_rehashing() {
        let root = temp_root("incremental-scan");
        let (config_path, config) = test_config(&root);
        fs::write(
            config.watched_roots[0].join("stable.rs"),
            "pub fn stable() -> bool { true }\n",
        )
        .unwrap();
        initialize(&config_path).unwrap();
        let baseline = supervisor_step(&config_path).unwrap();
        assert_eq!(baseline.last_scan_files_hashed, 1);
        assert_eq!(baseline.last_scan_files_reused, 0);
        let incremental = supervisor_step(&config_path).unwrap();
        assert_eq!(incremental.last_scan_files_hashed, 0);
        assert_eq!(incremental.last_scan_files_reused, 1);
        assert_eq!(incremental.observations_created, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn scan_observes_operator_stop_before_work_starts() {
        let root = temp_root("scan-stop");
        let (config_path, config) = test_config(&root);
        let state = initialize(&config_path).unwrap();
        request_stop(&config_path).unwrap();
        let lease = SupervisorLease::acquire(&config).unwrap();
        let memory = load_memory(&config, state.generation).unwrap();
        assert_eq!(
            scan_watched_roots_bounded(&config, &memory, &lease).unwrap_err(),
            "OPERATOR_STOP_REQUESTED_DURING_SCAN"
        );
        drop(lease);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_baseline_does_not_hide_later_new_work() {
        let root = temp_root("empty-baseline");
        let (config_path, config) = test_config(&root);
        initialize(&config_path).unwrap();
        let baseline = supervisor_step(&config_path).unwrap();
        assert!(baseline.baseline_created);
        let code = config.watched_roots[0].join("new_repair.rs");
        fs::write(
            &code,
            "pub fn checked(value: i32) -> Result<i32, String> {\n    if value < 0 { return Err(\"negative\".to_string()); }\n    Ok(value)\n}\n",
        )
        .unwrap();
        let evidence = root.join("verification.log");
        fs::write(&evidence, "test result: ok\n").unwrap();
        record_work_event(
            &config_path,
            WorkEvent {
                event_id: "new-work-pass".to_string(),
                actor: WorkActor::User,
                kind: WorkKind::DefectRepair,
                paths: vec![code],
                outcome: WorkOutcome::Pass,
                summary: "verified bounded repair".to_string(),
                evidence_sha256: vec![],
                evidence_artifacts: vec![evidence],
                performance_metrics: Vec::new(),
                public_contract_deltas: Vec::new(),
                occurred_at_ms: 1,
            },
        )
        .unwrap();
        let report = supervisor_step(&config_path).unwrap();
        assert!(!report.baseline_created);
        assert_eq!(report.observations_created, 1);
        assert_eq!(report.high_value_observations, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plateau_wait_does_not_raise_difficulty() {
        let root = temp_root("plateau");
        let (config_path, config) = test_config(&root);
        fs::write(
            config.watched_roots[0].join("lib.rs"),
            "pub fn stable() {}\n",
        )
        .unwrap();
        initialize(&config_path).unwrap();
        supervisor_step(&config_path).unwrap();
        supervisor_step(&config_path).unwrap();
        let report = supervisor_step(&config_path).unwrap();
        assert_eq!(report.phase, SupervisorPhase::WaitingPlateau);
        assert_eq!(report.difficulty_escalation_events, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_pass_and_repair_signals_are_high_value() {
        let current = FileFingerprint {
            content_sha256: "a".repeat(64),
            bytes: 100,
            modified_ms: 1,
            extension: "rs".to_string(),
            features: StructuralFeatures {
                lines: 10,
                non_empty_lines: 9,
                validation_tokens: 2,
                error_handling_tokens: 2,
                ..StructuralFeatures::default()
            },
        };
        let event = WorkEvent {
            event_id: "event".to_string(),
            actor: WorkActor::Codex,
            kind: WorkKind::DefectRepair,
            paths: vec![],
            outcome: WorkOutcome::Pass,
            summary: String::new(),
            evidence_sha256: vec!["a".repeat(64)],
            evidence_artifacts: vec![],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            occurred_at_ms: 1,
        };
        let observation = classify_observation(
            "ROOT_0/src/lib.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            45,
        );
        assert_eq!(observation.learning_value, LearningValue::High);
        assert!(observation.signals.contains(&"VERIFIED_PASS".to_string()));
        assert_eq!(observation.raw_source_bytes_stored, 0);
    }

    #[test]
    fn failed_work_is_negative_evidence() {
        let current = FileFingerprint {
            content_sha256: "a".repeat(64),
            bytes: 100,
            modified_ms: 1,
            extension: "rs".to_string(),
            features: StructuralFeatures::default(),
        };
        let event = WorkEvent {
            event_id: "event".to_string(),
            actor: WorkActor::User,
            kind: WorkKind::DefectRepair,
            paths: vec![],
            outcome: WorkOutcome::Fail,
            summary: String::new(),
            evidence_sha256: vec![],
            evidence_artifacts: vec![],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            occurred_at_ms: 1,
        };
        let observation = classify_observation(
            "ROOT_0/src/lib.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            45,
        );
        assert_eq!(observation.learning_value, LearningValue::Rejected);
    }

    #[test]
    fn observation_identity_is_retry_stable_and_binds_classified_semantics() {
        let current = FileFingerprint {
            content_sha256: "a".repeat(64),
            bytes: 100,
            modified_ms: 17,
            extension: "rs".to_string(),
            features: StructuralFeatures::default(),
        };
        let event = WorkEvent {
            event_id: "stable-event".to_string(),
            actor: WorkActor::Codex,
            kind: WorkKind::DefectRepair,
            paths: Vec::new(),
            outcome: WorkOutcome::Pass,
            summary: String::new(),
            evidence_sha256: vec!["b".repeat(64)],
            evidence_artifacts: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            occurred_at_ms: 23,
        };
        let first = classify_observation(
            "ROOT_0/src/lib.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            45,
        );
        let replay = classify_observation(
            "ROOT_0/src/lib.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            45,
        );
        assert_eq!(first, replay);
        assert_eq!(first.observed_at_ms, event.occurred_at_ms);

        let mut semantically_distinct = event;
        semantically_distinct.kind = WorkKind::CapabilitySynthesis;
        let reclassified = classify_observation(
            "ROOT_0/src/lib.rs".to_string(),
            &current,
            None,
            Some(&semantically_distinct),
            &ClassifierMemory::default(),
            45,
        );
        assert_ne!(first.observation_id, reclassified.observation_id);
    }

    #[test]
    fn unverifiable_cohort_is_deferred_without_failure_budget() {
        let root = temp_root("preflight-defer");
        let (_, config) = test_config(&root);
        let observation = LearningObservation {
            observation_id: "repair-without-test".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: None,
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 60,
            learning_value: LearningValue::High,
            reasons: vec!["repair observed".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        assert!(!campaign_preflight_ready(&config, &[observation]).unwrap());
        let diagnostics = config.state_dir.join("diagnostics");
        assert_eq!(fs::read_dir(diagnostics).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verification_receipt_alone_cannot_be_promoted_as_growth() {
        let root = temp_root("verification-only-not-growth");
        let (_, config) = test_config(&root);
        let verification = LearningObservation {
            observation_id: "validation-receipt".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/.b_repository_validation/receipt".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: None,
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::Verification,
            work_outcome: WorkOutcome::Pass,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec![
                "REPOSITORY_COHORT_VALIDATION".to_string(),
                "VERIFIED_PASS".to_string(),
            ],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            learning_score: 85,
            learning_value: LearningValue::High,
            reasons: vec!["same test cohort passed again".to_string()],
            verification_evidence_sha256: vec!["b".repeat(64)],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 40 * 60 * 1_000,
        };
        assert!(!campaign_preflight_ready(&config, std::slice::from_ref(&verification)).unwrap());

        let mut evaluator_change = verification.clone();
        evaluator_change.observation_id = "evaluator-change".to_string();
        evaluator_change.logical_path = "ROOT_0/tests/test_policy.py".to_string();
        evaluator_change.work_kind = WorkKind::RegressionTest;
        evaluator_change.work_outcome = WorkOutcome::Unknown;
        evaluator_change.signals = vec!["TEST_ADDED".to_string()];
        evaluator_change.verification_evidence_sha256.clear();
        evaluator_change.learning_score = 60;
        evaluator_change.observed_at_ms = 1;
        let selected = selected_campaign_observations(
            &config,
            &[verification.clone(), evaluator_change.clone()],
        );
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&verification));
        assert!(selected.contains(&evaluator_change));
        assert!(!campaign_preflight_ready(&config, &selected).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deterministic_campaign_error_is_classified_and_exact_cohort_is_quarantined() {
        let root = temp_root("deterministic-campaign-error-quarantine");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let observation = LearningObservation {
            observation_id: "typed-cohort-that-failed-synthesis".to_string(),
            work_event_id: Some("typed-event-that-failed-synthesis".to_string()),
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: Some("b".repeat(64)),
            actor: WorkActor::Codex,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Pass,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["VERIFIED_PASS".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 90,
            learning_value: LearningValue::High,
            reasons: vec!["typed synthesis failed deterministically".to_string()],
            verification_evidence_sha256: vec!["c".repeat(64)],
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![public_contract_delta_fixture()],
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let freeze =
            freeze_new_campaign(&config, &mut state, std::slice::from_ref(&observation)).unwrap();
        let mut index = load_index(&config).unwrap();

        let campaign_id = abort_pending_campaign(
            &config,
            &mut state,
            Some(&mut index),
            "TYPED_MECHANISM_SYNTHESIS_EXHAUSTED:bounded-detail",
            true,
        )
        .unwrap()
        .unwrap();

        assert_eq!(campaign_id, freeze.campaign_id);
        let failure: CampaignFailure = read_json(
            &config
                .state_dir
                .join("history")
                .join(format!("{campaign_id}.failure.json")),
        )
        .unwrap();
        assert_eq!(failure.error_class, "TYPED_MECHANISM_SYNTHESIS_EXHAUSTED");
        assert_eq!(
            failure.observation_ids,
            vec![observation.observation_id.clone()]
        );
        assert!(failure.observations_quarantined);
        let reloaded_index = load_index(&config).unwrap();
        assert!(reloaded_index
            .consumed_observation_ids
            .contains(&observation.observation_id));
        assert!(reloaded_index
            .consumed_work_event_ids
            .contains(observation.work_event_id.as_deref().unwrap()));
        assert_eq!(state.pending_campaign_id, None);
        assert_eq!(state.campaigns_failed, 1);
        assert_eq!(state.generation, 0);
        assert!(
            !failed_campaign_engine_changed(&config, &freeze.proposer_executable_sha256).unwrap()
        );
        assert!(failed_campaign_engine_changed(&config, &"d".repeat(64)).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verification_only_false_tip_is_quarantined_and_predecessor_restored() {
        let root = temp_root("recover-verification-only-tip");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let restored = load_memory(&config, 0).unwrap();
        let restored_hash = json_sha256(&restored).unwrap();
        let mut invalid = restored.clone();
        invalid.generation = 1;
        invalid.predecessor_sha256 = Some(restored_hash.clone());
        invalid.lessons.push(LearnedCompositionLesson {
            lesson_id: "verification-receipt-only".to_string(),
            evidence_observation_sha256: vec!["a".repeat(64)],
            work_kinds: vec![WorkKind::Verification],
            diagnostic_signals: vec!["VERIFIED_PASS".to_string()],
            composition_recipe: vec!["REUSE_REGRESSION_EVIDENCE".to_string()],
            applicability: vec!["ROOT_0".to_string()],
            verification_obligations: vec!["REPOSITORY_VALIDATION_PASS".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        });
        let invalid_hash = json_sha256(&invalid).unwrap();
        write_immutable_json(&memory_path(&config, 1), &invalid).unwrap();
        state.generation = 1;
        state.current_memory_sha256 = invalid_hash.clone();
        state.predecessor_memory_sha256 = Some(restored_hash.clone());
        state.campaigns_started = 1;
        state.campaigns_accepted = 1;
        state.mutual_revalidation_events = 1;

        assert!(recover_verification_only_generation_tip(&config, &mut state).unwrap());
        assert_eq!(state.generation, 0);
        assert_eq!(state.current_memory_sha256, restored_hash);
        assert_eq!(state.campaigns_accepted, 0);
        assert_eq!(state.campaigns_failed, 1);
        assert_eq!(state.mutual_revalidation_events, 0);
        assert!(!memory_path(&config, 1).exists());
        let directory = invalidated_generation_dir(&config, 1);
        let quarantined: GrowthMemory = read_json(&directory.join("memory.json")).unwrap();
        assert_eq!(json_sha256(&quarantined).unwrap(), invalid_hash);
        let receipt: InvalidGenerationRecoveryReceipt =
            read_json(&directory.join("recovery_receipt.json")).unwrap();
        assert_eq!(receipt.invalid_generation, 1);
        assert_eq!(receipt.restored_generation, 0);
        assert!(!recover_verification_only_generation_tip(&config, &mut state).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn real_growth_tip_is_never_recovered_as_verification_only() {
        let root = temp_root("preserve-real-growth-tip");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let mut current = load_memory(&config, 0).unwrap();
        current.generation = 1;
        current.predecessor_sha256 = Some(state.current_memory_sha256.clone());
        current.lessons.push(LearnedCompositionLesson {
            lesson_id: "source-change-with-verification".to_string(),
            evidence_observation_sha256: vec!["b".repeat(64)],
            work_kinds: vec![WorkKind::CodeChange, WorkKind::Verification],
            diagnostic_signals: vec!["VERIFIED_PASS".to_string()],
            composition_recipe: vec!["SOURCE_CHANGE_THEN_VERIFY".to_string()],
            applicability: vec!["ROOT_0".to_string()],
            verification_obligations: vec!["REGRESSION_PASS".to_string()],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            learning_score: 90,
            exact_patch_data_present: false,
            exact_source_fragment_present: false,
            raw_source_bytes_present: false,
        });
        state.generation = 1;
        state.predecessor_memory_sha256 = current.predecessor_sha256.clone();
        state.current_memory_sha256 = json_sha256(&current).unwrap();
        write_immutable_json(&memory_path(&config, 1), &current).unwrap();

        assert!(!recover_verification_only_generation_tip(&config, &mut state).unwrap());
        assert_eq!(state.generation, 1);
        assert!(memory_path(&config, 1).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn production_file_with_tests_requires_executed_pass_before_verification_credit() {
        let observation = LearningObservation {
            observation_id: "mixed-production-change".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/engine.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: Some("b".repeat(64)),
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::RegressionTest,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec![
                "REGRESSION_EVIDENCE".to_string(),
                "TEST_ADDED".to_string(),
                "VALIDATION_ADDED".to_string(),
            ],
            composition_roles: vec![
                "REGRESSION_TEST".to_string(),
                "INPUT_VALIDATION".to_string(),
            ],
            learning_score: 55,
            learning_value: LearningValue::High,
            reasons: vec!["mixed production change".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let lesson = build_lesson(std::slice::from_ref(&observation)).unwrap();
        assert!(lesson
            .composition_recipe
            .contains(&"IMPLEMENTATION".to_string()));
        assert!(!lesson_has_verification_evidence(&lesson));

        let mut verified = observation.clone();
        verified.work_outcome = WorkOutcome::Pass;
        verified.signals.push("VERIFIED_PASS".to_string());
        verified.verification_evidence_sha256 = vec!["a".repeat(64)];
        let verified_lesson = build_lesson(&[verified]).unwrap();
        assert!(lesson_has_verification_evidence(&verified_lesson));

        let mut dedicated_test = observation;
        dedicated_test.logical_path = "ROOT_0/tests/engine_test.rs".to_string();
        let test_only_lesson = build_lesson(&[dedicated_test]).unwrap();
        assert!(!test_only_lesson
            .composition_recipe
            .contains(&"IMPLEMENTATION".to_string()));
        assert!(!lesson_has_verification_evidence(&test_only_lesson));
    }

    #[test]
    fn embedded_tests_do_not_hide_an_implementation_delta() {
        let prior = FileFingerprint {
            content_sha256: "a".repeat(64),
            bytes: 10,
            modified_ms: 1,
            extension: "rs".to_string(),
            features: StructuralFeatures {
                test_tokens: 4,
                validation_tokens: 2,
                ..StructuralFeatures::default()
            },
        };
        let current = FileFingerprint {
            content_sha256: "b".repeat(64),
            bytes: 11,
            modified_ms: 2,
            extension: "rs".to_string(),
            features: StructuralFeatures {
                test_tokens: 4,
                validation_tokens: 3,
                ..StructuralFeatures::default()
            },
        };
        let observation = classify_observation(
            "ROOT_0/src/engine.rs".to_string(),
            &current,
            Some(&prior),
            None,
            &ClassifierMemory::default(),
            45,
        );
        assert_eq!(observation.work_kind, WorkKind::DefectRepair);
        assert!(observation
            .signals
            .contains(&"VALIDATION_ADDED".to_string()));
    }

    #[test]
    fn blocked_core_cohort_runs_bounded_regression_and_emits_reusable_pass_evidence() {
        let root = temp_root("blocked-core-cohort-validation");
        let (_, mut config) = test_config(&root);
        let source_root = config.watched_roots[0].clone();
        fs::create_dir_all(source_root.join("src")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            source_root.join("Cargo.toml"),
            "[package]\nname = \"semantic-reasoning\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            source_root.join("Cargo.lock"),
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"semantic-reasoning\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            source_root.join("src/lib.rs"),
            "pub fn repaired() -> bool { true }\n#[cfg(test)]\nmod tests { #[test] fn repaired_passes() { assert!(super::repaired()); } }\n",
        )
        .unwrap();
        config.source_mutation = AutonomousSourceMutationPolicy {
            enabled: true,
            source_root: source_root.clone(),
            cargo_executable: std::env::var_os("CARGO")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("cargo")),
            build_target_dir: root.join("target"),
            runtime_bin_dir: root.join("runtime"),
            validation_timeout_ms: 120_000,
            ..AutonomousSourceMutationPolicy::default()
        };
        let implementation = LearningObservation {
            observation_id: "core-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: Some("b".repeat(64)),
            actor: WorkActor::Codex,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 79,
            learning_value: LearningValue::High,
            reasons: vec!["unverified core repair".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let diagnostic = inspect_self(SelfInspectionInput {
            generation: 2,
            supervisor_sequence: 9,
            files_scanned: 1,
            files_reused: 0,
            files_hashed: 1,
            scan_duration_ms: 1,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 1,
            mutual_revalidation_events: 1,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 0,
            unconsumed_high_observations: 1,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: true,
            repository_cohort_validation_applicable: false,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 1,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap();

        let (first_action, first_observation) = runtime_repair_action(
            &config,
            &diagnostic,
            &[],
            std::slice::from_ref(&implementation),
            std::slice::from_ref(&implementation),
        )
        .unwrap()
        .expect("bounded validation action");
        let first_observation = first_observation.expect("verified pass observation");
        assert!(first_action.executed);
        assert!(first_action.changed_runtime_decision);
        assert_eq!(first_observation.work_outcome, WorkOutcome::Pass);
        assert!(first_observation
            .signals
            .contains(&"VERIFIED_PASS".to_string()));
        assert!(!first_observation.verification_evidence_sha256.is_empty());
        let lesson = build_lesson(&[implementation.clone(), first_observation.clone()]).unwrap();
        assert!(lesson_has_verification_evidence(&lesson));
        assert!(lesson
            .composition_recipe
            .contains(&"IMPLEMENTATION_REPAIR".to_string()));

        let (reused_action, reused_observation) = runtime_repair_action(
            &config,
            &diagnostic,
            &[],
            std::slice::from_ref(&implementation),
            std::slice::from_ref(&implementation),
        )
        .unwrap()
        .expect("reused validation action");
        assert_eq!(
            reused_action.output_observation_ids,
            first_action.output_observation_ids
        );
        let reused_observation = reused_observation.expect("reused pass observation");
        assert_eq!(reused_observation, first_observation);
        let receipt_count = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("core_cohort_validation_")
                    && entry.path().extension().and_then(OsStr::to_str) == Some("json")
            })
            .count();
        assert_eq!(receipt_count, 1);
        let receipt_path = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("core_cohort_validation_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .expect("core validation receipt");
        let validation: CoreCohortValidationReceipt = read_json(&receipt_path).unwrap();
        assert!(validation.command.cargo_incremental);

        let mut rebound_implementation = implementation.clone();
        rebound_implementation.observation_id = "core-implementation-rebound".to_string();
        let (rebound_action, rebound_observation) = runtime_repair_action(
            &config,
            &diagnostic,
            &[],
            std::slice::from_ref(&rebound_implementation),
            std::slice::from_ref(&rebound_implementation),
        )
        .unwrap()
        .expect("source-identical validation reuse action");
        assert!(rebound_action.executed);
        assert!(rebound_observation.is_some());
        let receipts = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("core_cohort_validation_")
                    && entry.path().extension().and_then(OsStr::to_str) == Some("json")
            })
            .map(|entry| read_json::<CoreCohortValidationReceipt>(&entry.path()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(receipts.len(), 2);
        let rebound_receipt = receipts
            .iter()
            .find(|receipt| receipt.input_observation_ids == ["core-implementation-rebound"])
            .expect("rebound receipt");
        assert!(rebound_receipt.reused_validation_receipt_sha256.is_some());
        assert_eq!(rebound_receipt.command.duration_ms, 0);
        assert!(rebound_receipt
            .command
            .diagnostic_tail
            .starts_with("REUSED_SOURCE_IDENTICAL_CORE_VALIDATION_RECEIPT:"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn blocked_python_repository_cohort_executes_observed_native_tests() {
        let Ok(python) = resolve_local_program("python") else {
            return;
        };
        if !Command::new(&python)
            .args(["-c", "import pytest"])
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        let root = temp_root("blocked-python-repository-validation");
        let (_, config) = test_config(&root);
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("tests")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        fs::write(
            repository.join("core_module.py"),
            "def repaired():\n    return True\n",
        )
        .unwrap();
        fs::write(
            repository.join("tests/test_core_module.py"),
            "from core_module import repaired\n\ndef test_repaired():\n    assert repaired()\n",
        )
        .unwrap();
        let implementation = LearningObservation {
            observation_id: "python-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/core_module.py".to_string(),
            content_sha256: "c".repeat(64),
            predecessor_content_sha256: Some("d".repeat(64)),
            actor: WorkActor::Codex,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 70,
            learning_value: LearningValue::High,
            reasons: vec!["python repair".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let test_observation = LearningObservation {
            observation_id: "python-test".to_string(),
            logical_path: "ROOT_0/tests/test_core_module.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            learning_score: 60,
            reasons: vec!["python regression".to_string()],
            ..implementation.clone()
        };
        let test_only_plan =
            repository_validation_plan(&config, std::slice::from_ref(&test_observation))
                .unwrap()
                .expect("test-only Python evaluator change is independently verifiable");
        assert_eq!(
            test_only_plan.validator_kind,
            RepositoryValidatorKind::PythonPytest
        );
        assert_eq!(
            test_only_plan.test_paths,
            vec![PathBuf::from("tests/test_core_module.py")]
        );
        assert_eq!(
            test_only_plan.input_observation_ids,
            vec!["python-test".to_string()]
        );
        let mut exact_owner_observation = implementation.clone();
        let mut exact_owner_delta = public_contract_delta_fixture();
        exact_owner_delta.target_symbols = vec!["RenamedPolicy.marker".to_string()];
        exact_owner_observation.public_contract_deltas = vec![exact_owner_delta];
        let exact_owner_plan = repository_validation_plan(
            &config,
            &[exact_owner_observation, test_observation.clone()],
        )
        .unwrap()
        .expect("structured public owner reaches repository plan");
        assert_eq!(
            exact_owner_plan.public_contract_target_symbols,
            ["RenamedPolicy.marker"]
        );
        assert_eq!(
            repository_repair_target_symbols(
                &exact_owner_plan,
                "> assert Fixture.marker == 'ready'\nE where <function Fixture.marker at 0x1>"
            ),
            ["RenamedPolicy.marker", "Fixture.marker"]
        );
        let cohort = vec![implementation.clone(), test_observation];
        let diagnostic = inspect_self(SelfInspectionInput {
            generation: 3,
            supervisor_sequence: 10,
            files_scanned: 2,
            files_reused: 0,
            files_hashed: 2,
            scan_duration_ms: 1,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 2,
            mutual_revalidation_events: 2,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 0,
            unconsumed_high_observations: 2,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: true,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 1,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap();

        let (action, verification) =
            runtime_repair_action(&config, &diagnostic, &[], &cohort, &cohort)
                .unwrap()
                .expect("repository validation action");
        let verification = verification.expect("repository pass observation");

        assert!(action.executed);
        assert!(action.changed_runtime_decision);
        assert_eq!(verification.work_outcome, WorkOutcome::Pass);
        assert!(verification
            .logical_path
            .starts_with("ROOT_0/.b_repository_validation/"));
        assert!(verification
            .signals
            .contains(&"REPOSITORY_COHORT_VALIDATION".to_string()));
        let lesson = build_lesson(&[implementation.clone(), verification]).unwrap();
        assert!(lesson_has_verification_evidence(&lesson));
        assert!(lesson
            .composition_recipe
            .contains(&"IMPLEMENTATION_REPAIR".to_string()));
        let receipt = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("repository_cohort_validation_")
                    && entry.path().extension().and_then(OsStr::to_str) == Some("json")
            })
            .expect("repository validation receipt");
        let receipt: RepositoryCohortValidationReceipt = read_json(&receipt.path()).unwrap();
        assert!(receipt.success);
        assert_eq!(
            receipt.validator_kind,
            RepositoryValidatorKind::PythonPytest
        );
        assert_eq!(receipt.test_selection_source, "OBSERVED_TEST_COHORT");
        assert_eq!(
            receipt.test_paths,
            vec![PathBuf::from("tests/test_core_module.py")]
        );
        assert!(receipt.scope_stable_during_validation);
        let mut next_generation_diagnostic = diagnostic.clone();
        next_generation_diagnostic.generation = diagnostic.generation.saturating_add(1);
        next_generation_diagnostic.diagnostic_id = "next-generation-diagnostic".to_string();
        let reused =
            validate_blocked_repository_cohort(&config, &next_generation_diagnostic, &cohort)
                .unwrap();
        assert!(reused.executed);
        assert!(!reused.sandbox_repair_verified);
        assert_eq!(reused.output_observation_ids, action.output_observation_ids);
        assert_eq!(reused.evidence_sha256, action.execution_evidence_sha256);
        assert_eq!(
            fs::read_dir(config.state_dir.join("diagnostics"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("repository_cohort_validation_"))
                .count(),
            1
        );
        let reused = repository_validation_plan(&config, &[implementation])
            .unwrap()
            .expect("reuse prior verified tests");
        assert_eq!(reused.validator_kind, RepositoryValidatorKind::PythonPytest);
        assert_eq!(reused.test_selection_source, "VERIFIED_RECEIPT_REUSE");
        assert!(reused.reused_validation_receipt_sha256.is_some());
        assert_eq!(
            reused.test_paths,
            vec![PathBuf::from("tests/test_core_module.py")]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_python_cohort_synthesizes_and_falsifies_candidate_without_false_installation() {
        let Ok(python) = resolve_local_program("python") else {
            return;
        };
        if !Command::new(&python)
            .args(["-c", "import pytest"])
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        let root = temp_root("failed-python-source-bound-repair");
        let (_, mut config) = test_config(&root);
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("tests")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        let predecessor = "def add(left, right):\n    return add_impl(left, right)\n\ndef stable_zero(left, right):\n    return add_impl(left, right)\n\ndef add_impl(first, second):\n    return 0\n";
        fs::write(repository.join("core_module.py"), predecessor).unwrap();
        fs::write(
            repository.join("tests/test_core_module.py"),
            "from core_module import add, stable_zero\n\ndef test_add():\n    assert add(2, 3) == 5\n    assert add(4, 7) == 11\n\n\ndef test_stable_zero():\n    assert stable_zero(2, 3) == 0\n    assert stable_zero(4, 7) == 0\n",
        )
        .unwrap();
        let implementation = LearningObservation {
            observation_id: "failed-python-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/core_module.py".to_string(),
            content_sha256: sha256(predecessor.as_bytes()),
            predecessor_content_sha256: Some("e".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 75,
            learning_value: LearningValue::High,
            reasons: vec!["failing public behavior".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let test_observation = LearningObservation {
            observation_id: "failed-python-test".to_string(),
            logical_path: "ROOT_0/tests/test_core_module.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            learning_score: 65,
            reasons: vec!["public arithmetic observations".to_string()],
            ..implementation.clone()
        };
        let cohort = vec![implementation.clone(), test_observation];
        let diagnostic = inspect_self(SelfInspectionInput {
            generation: 4,
            supervisor_sequence: 11,
            files_scanned: 2,
            files_reused: 0,
            files_hashed: 2,
            scan_duration_ms: 1,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 2,
            mutual_revalidation_events: 2,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 0,
            unconsumed_high_observations: 2,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: true,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 1,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap();

        let (action, candidate_observation) =
            runtime_repair_action(&config, &diagnostic, &[], &cohort, &cohort)
                .unwrap()
                .expect("source-bound repair action");
        let candidate_observation = candidate_observation.expect("sandbox repair observation");
        assert!(action.executed);
        assert_eq!(action.execution_evidence_sha256.len(), 2);
        assert!(candidate_observation
            .signals
            .contains(&"SANDBOX_VERIFIED_REPAIR_CANDIDATE".to_string()));
        assert!(candidate_observation
            .signals
            .contains(&"CANDIDATE_NOT_INSTALLED".to_string()));
        assert!(!candidate_observation
            .signals
            .contains(&"VERIFIED_PASS".to_string()));
        assert_eq!(candidate_observation.work_outcome, WorkOutcome::Unknown);
        assert_eq!(
            fs::read_to_string(repository.join("core_module.py")).unwrap(),
            predecessor
        );
        let sandbox_parent = config.state_dir.join("repository_repair_sandboxes");
        assert!(
            !sandbox_parent.exists() || fs::read_dir(&sandbox_parent).unwrap().next().is_none()
        );
        let repair_path = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .expect("repair synthesis receipt");
        let repair: RepositoryRepairSynthesisReceipt = read_json(&repair_path).unwrap();
        assert_eq!(repair.schema, REPOSITORY_REPAIR_SYNTHESIS_SCHEMA);
        assert_eq!(
            repair.source_repair_engine_revision,
            SOURCE_REPAIR_ENGINE_REVISION
        );
        assert_eq!(repair.repair_problem_id.len(), 64);
        assert_eq!(repair.synthesis_capability_sha256.len(), 64);
        assert_eq!(
            repair.source_bound_patch_variant_sha256s_attempted.len(),
            repair.source_bound_patch_variant_ids_attempted.len()
        );
        assert!(repair.sandbox_verified);
        assert!(repair.sandbox_cleaned);
        assert!(repair.authoritative_scope_stable);
        assert!(!repair.candidate_installed);
        assert_eq!(repair.authoritative_source_write_events, 0);
        assert_eq!(repair.raw_source_bytes_stored, 0);
        assert!(repair.candidate_materialization_is_one_to_one);
        assert!(repair.failure_code.is_none());
        assert_eq!(repair.source_bound_patch_variant_ids_attempted.len(), 1);
        assert_eq!(
            repair.selected_source_bound_patch_variant_id,
            repair
                .source_bound_patch_variant_ids_attempted
                .first()
                .cloned()
        );
        assert_eq!(repair.selected_source_bound_template_symbols, ["add"]);
        assert_eq!(repair.promoted_improvement_operator_ids.len(), 1);
        assert!(repair.selected_improvement_operator_ids.is_empty());
        assert_eq!(repair.improvement_operators.len(), 1);
        assert!(repair.typed_candidates_enumerated > 1);
        assert!(repair.sandbox_command.as_ref().is_some_and(|command| {
            command.success
                && command
                    .diagnostic_tail
                    .starts_with("SANDBOX_VALIDATION_OUTPUT_SHA256:")
        }));
        let promoted_operator_id = repair.promoted_improvement_operator_ids[0].clone();
        assert!(source_bound_improvement_operator_directory(&config)
            .join(format!("{promoted_operator_id}.json"))
            .is_file());
        let authority_path = fs::read_dir(source_bound_improvement_operator_authority_directory(
            &config,
        ))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .next()
        .expect("operator authority receipt");
        let authority_bytes = fs::read(&authority_path).unwrap();
        let authority: SourceBoundImprovementOperatorAuthorityReceipt =
            serde_json::from_slice(&authority_bytes).unwrap();
        validate_source_bound_operator_authority(&authority).unwrap();
        assert_eq!(authority.promotion_generation, 4);
        let mut tampered_authority = authority.clone();
        tampered_authority.candidate_sha256 = "0".repeat(64);
        assert_eq!(
            validate_source_bound_operator_authority(&tampered_authority),
            Err("SOURCE_BOUND_OPERATOR_AUTHORITY_HASH_MISMATCH".to_string())
        );
        let mut tampered_generation = authority;
        tampered_generation.promotion_generation = 400;
        assert_eq!(
            validate_source_bound_operator_authority(&tampered_generation),
            Err("SOURCE_BOUND_OPERATOR_AUTHORITY_HASH_MISMATCH".to_string())
        );
        fs::remove_file(&authority_path).unwrap();
        assert!(load_source_bound_improvement_operators(&config)
            .unwrap()
            .is_empty());
        fs::write(&authority_path, authority_bytes).unwrap();
        assert_eq!(
            load_source_bound_improvement_operators(&config)
                .unwrap()
                .len(),
            1
        );
        let lesson = build_lesson(&[implementation, candidate_observation]).unwrap();
        assert!(!lesson_has_verification_evidence(&lesson));

        let renamed_repository = root.join("renamed-repository");
        fs::create_dir_all(renamed_repository.join("tests")).unwrap();
        fs::write(
            renamed_repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        let renamed_predecessor = "def combine(alpha, beta):\n    return 0\n";
        fs::write(renamed_repository.join("solver.py"), renamed_predecessor).unwrap();
        fs::write(
            renamed_repository.join("tests/test_solver.py"),
            "from solver import combine\n\ndef test_combine():\n    assert combine(8, 5) == 13\n    assert combine(-4, 9) == 5\n",
        )
        .unwrap();
        config.watched_roots.push(renamed_repository.clone());
        let renamed_implementation = LearningObservation {
            observation_id: "renamed-python-implementation".to_string(),
            logical_path: "ROOT_1/solver.py".to_string(),
            content_sha256: sha256(renamed_predecessor.as_bytes()),
            predecessor_content_sha256: Some("f".repeat(64)),
            reasons: vec!["renamed public behavior contradiction".to_string()],
            ..cohort[0].clone()
        };
        let renamed_test = LearningObservation {
            observation_id: "renamed-python-test".to_string(),
            logical_path: "ROOT_1/tests/test_solver.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            ..renamed_implementation.clone()
        };
        let renamed_cohort = vec![renamed_implementation, renamed_test];
        let mut renamed_diagnostic = diagnostic;
        renamed_diagnostic.generation = renamed_diagnostic.generation.saturating_add(1);
        renamed_diagnostic.diagnostic_id = "renamed-repository-diagnostic".to_string();
        let (renamed_action, renamed_candidate) = runtime_repair_action(
            &config,
            &renamed_diagnostic,
            &[],
            &renamed_cohort,
            &renamed_cohort,
        )
        .unwrap()
        .expect("renamed repository repair action");
        assert!(renamed_action.executed);
        assert!(renamed_candidate.is_some());
        assert_eq!(
            fs::read_to_string(renamed_repository.join("solver.py")).unwrap(),
            renamed_predecessor
        );
        let renamed_repair = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .map(|path| read_json::<RepositoryRepairSynthesisReceipt>(&path).unwrap())
            .find(|receipt| receipt.source_relative_path == Path::new("solver.py"))
            .expect("renamed repair receipt");
        assert_eq!(
            renamed_repair.selected_improvement_operator_ids,
            [promoted_operator_id]
        );
        assert!(renamed_repair.promoted_improvement_operator_ids.is_empty());
        assert_eq!(renamed_repair.typed_candidates_enumerated, 1);
        fs::remove_dir_all(root).unwrap();
    }

    fn class_declaration_repair_cohort(predecessor: &str) -> Vec<LearningObservation> {
        let mut delta = public_contract_delta_fixture();
        delta.target_symbols = vec!["ProductPolicy.marker".to_string()];
        bind_public_contract_delta_fixture(&mut delta);
        let implementation = LearningObservation {
            observation_id: "class-declaration-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/policy.py".to_string(),
            content_sha256: sha256(predecessor.as_bytes()),
            predecessor_content_sha256: Some("e".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 75,
            learning_value: LearningValue::High,
            reasons: vec!["missing public class declaration".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta],
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let test_observation = LearningObservation {
            observation_id: "class-declaration-test".to_string(),
            logical_path: "ROOT_0/tests/test_policy.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            learning_score: 65,
            reasons: vec!["public declaration observation".to_string()],
            ..implementation.clone()
        };
        vec![implementation, test_observation]
    }

    fn repository_repair_diagnostic() -> AutonomousSelfInspectionReceipt {
        inspect_self(SelfInspectionInput {
            generation: 5,
            supervisor_sequence: 12,
            files_scanned: 2,
            files_reused: 0,
            files_hashed: 2,
            scan_duration_ms: 1,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 2,
            mutual_revalidation_events: 2,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 0,
            unconsumed_high_observations: 2,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: true,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 1,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap()
    }

    #[test]
    fn failed_python_requirement_without_contract_delta_reaches_product_install() {
        let Ok(python) = resolve_local_program("python") else {
            return;
        };
        if !Command::new(&python)
            .args(["-c", "import pytest"])
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        let root = temp_root("failed-python-class-declaration-repair");
        let (_, mut config) = test_config(&root);
        config.repository_mutation.enabled = true;
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("tests")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        let predecessor = "class ProductPolicy:\n    pass\n";
        fs::write(repository.join("policy.py"), predecessor).unwrap();
        fs::write(
            repository.join("tests/test_policy.py"),
            "from policy import ProductPolicy\n\ndef test_policy():\n    assert ProductPolicy.marker == 'ready'\n",
        )
        .unwrap();
        let implementation = LearningObservation {
            observation_id: "class-declaration-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/policy.py".to_string(),
            content_sha256: sha256(predecessor.as_bytes()),
            predecessor_content_sha256: Some("e".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 75,
            learning_value: LearningValue::High,
            reasons: vec!["missing public class declaration".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let test_observation = LearningObservation {
            observation_id: "class-declaration-test".to_string(),
            logical_path: "ROOT_0/tests/test_policy.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            learning_score: 65,
            reasons: vec!["public declaration observation".to_string()],
            ..implementation.clone()
        };
        let cohort = vec![implementation, test_observation];
        let diagnostic = inspect_self(SelfInspectionInput {
            generation: 5,
            supervisor_sequence: 12,
            files_scanned: 2,
            files_reused: 0,
            files_hashed: 2,
            scan_duration_ms: 1,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 2,
            mutual_revalidation_events: 2,
            evaluator_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 0,
            unconsumed_high_observations: 2,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: true,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 1,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap();

        let outcome = validate_blocked_repository_cohort(&config, &diagnostic, &cohort).unwrap();
        assert!(outcome.executed);
        assert!(outcome.sandbox_repair_verified);
        assert!(outcome.repository_repair_installed);
        assert_eq!(outcome.evidence_sha256.len(), 2);
        assert!(fs::read_to_string(repository.join("policy.py"))
            .unwrap()
            .contains("marker = 'ready'"));
        let repair = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .map(|path| read_json::<RepositoryRepairSynthesisReceipt>(&path).unwrap())
            .find(|receipt| receipt.source_relative_path == Path::new("policy.py"))
            .expect("class declaration repair receipt");
        assert!(repair.sandbox_verified);
        assert!(repair.sandbox_cleaned);
        assert!(repair.candidate_installed);
        assert!(!repair.rolled_back);
        assert_eq!(repair.authoritative_source_write_events, 1);
        assert!(repair
            .authoritative_command
            .as_ref()
            .is_some_and(|command| command.success));
        assert_eq!(
            repair.selected_source_bound_template_symbols,
            ["ProductPolicy.marker"]
        );
        assert_eq!(repair.edit_atom_kinds, ["ATOMIC_MULTI_EDIT", "INSERT"]);
        assert_eq!(repair.typed_candidates_enumerated, 0);
        assert!(repair.improvement_operators.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_repository_operator_learns_only_from_post_install_verifier() {
        let Ok(python) = resolve_local_program("python") else {
            return;
        };
        if !Command::new(&python)
            .args(["-c", "import pytest"])
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        let root = temp_root("installed-repository-operator-authority");
        let (_, mut config) = test_config(&root);
        config.repository_mutation.enabled = true;
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("tests")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        let predecessor = "def add(left, right):\n    return 0\n";
        fs::write(repository.join("calculator.py"), predecessor).unwrap();
        fs::write(
            repository.join("tests/test_calculator.py"),
            "from calculator import add\n\ndef test_add():\n    assert add(2, 3) == 5\n    assert add(4, 7) == 11\n",
        )
        .unwrap();
        let implementation = LearningObservation {
            observation_id: "installed-operator-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/calculator.py".to_string(),
            content_sha256: sha256(predecessor.as_bytes()),
            predecessor_content_sha256: Some("e".repeat(64)),
            actor: WorkActor::LocalTool,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 75,
            learning_value: LearningValue::High,
            reasons: vec!["contradicted public arithmetic".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let test_observation = LearningObservation {
            observation_id: "installed-operator-test".to_string(),
            logical_path: "ROOT_0/tests/test_calculator.py".to_string(),
            work_kind: WorkKind::RegressionTest,
            signals: vec!["REGRESSION_EVIDENCE".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["REGRESSION_TEST".to_string()],
            ..implementation.clone()
        };
        let outcome = validate_blocked_repository_cohort(
            &config,
            &repository_repair_diagnostic(),
            &[implementation, test_observation],
        )
        .unwrap();
        assert!(outcome.executed);
        assert!(outcome.repository_repair_installed);
        assert!(fs::read_to_string(repository.join("calculator.py"))
            .unwrap()
            .contains("_b_core_left + _b_core_right"));
        let repair = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .map(|path| read_json::<RepositoryRepairSynthesisReceipt>(&path).unwrap())
            .find(|receipt| receipt.source_relative_path == Path::new("calculator.py"))
            .expect("installed arithmetic repair receipt");
        let authoritative_output_sha256 = repair
            .authoritative_command
            .as_ref()
            .filter(|command| command.success)
            .map(|command| command.output_sha256.clone())
            .expect("authoritative verifier output");
        assert_eq!(repair.improvement_operators.len(), 1);
        assert_eq!(
            repair.improvement_operators[0].evidence_sha256,
            authoritative_output_sha256
        );
        let authority_path = fs::read_dir(source_bound_improvement_operator_authority_directory(
            &config,
        ))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .next()
        .expect("installed operator authority");
        let authority: SourceBoundImprovementOperatorAuthorityReceipt =
            read_json(&authority_path).unwrap();
        validate_source_bound_operator_authority(&authority).unwrap();
        assert_eq!(authority.schema, INSTALLED_TYPED_OPERATOR_AUTHORITY_SCHEMA);
        assert!(authority.candidate_installed);
        assert_eq!(authority.authoritative_source_write_events, 1);
        assert_eq!(authority.sandbox_output_sha256, authoritative_output_sha256);
        assert!(source_bound_operator_has_exact_authority(
            &config,
            &repair.improvement_operators[0]
        )
        .unwrap());
        let mut unbound_evidence = repair.improvement_operators[0].clone();
        unbound_evidence.evidence_sha256 = "f".repeat(64);
        assert!(!source_bound_operator_has_exact_authority(&config, &unbound_evidence).unwrap());
        assert_eq!(
            fs::read_dir(source_bound_improvement_operator_authority_directory(
                &config
            ))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(OsStr::to_str) == Some("json"))
            .filter_map(|entry| {
                read_json::<SourceBoundImprovementOperatorAuthorityReceipt>(&entry.path()).ok()
            })
            .filter(|authority| {
                authority.operator_id == repair.improvement_operators[0].operator_id
            })
            .count(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn authoritative_repository_failure_rolls_back_verified_sandbox_candidate() {
        let Ok(python) = resolve_local_program("python") else {
            return;
        };
        if !Command::new(&python)
            .args(["-c", "import pytest"])
            .status()
            .is_ok_and(|status| status.success())
        {
            return;
        }
        let root = temp_root("authoritative-repository-rollback");
        let (_, mut config) = test_config(&root);
        config.repository_mutation.enabled = true;
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("tests")).unwrap();
        fs::create_dir_all(config.state_dir.join("diagnostics")).unwrap();
        fs::write(
            repository.join("pyproject.toml"),
            "[build-system]\nrequires = []\nbuild-backend = \"\"\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
        )
        .unwrap();
        let predecessor = "class ProductPolicy:\n    pass\n";
        fs::write(repository.join("policy.py"), predecessor).unwrap();
        fs::write(
            repository.join("tests/test_policy.py"),
            "from pathlib import Path\nfrom policy import ProductPolicy\n\ndef test_policy():\n    assert ProductPolicy.marker == 'ready'\n    assert 'repository_repair_sandboxes' in str(Path.cwd())\n",
        )
        .unwrap();

        let outcome = validate_blocked_repository_cohort(
            &config,
            &repository_repair_diagnostic(),
            &class_declaration_repair_cohort(predecessor),
        )
        .unwrap();
        assert!(!outcome.executed);
        assert!(!outcome.sandbox_repair_verified);
        assert!(!outcome.repository_repair_installed);
        assert_eq!(
            fs::read_to_string(repository.join("policy.py")).unwrap(),
            predecessor
        );
        let repair = fs::read_dir(config.state_dir.join("diagnostics"))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("repository_repair_synthesis_"))
                    && path.extension().and_then(OsStr::to_str) == Some("json")
            })
            .map(|path| read_json::<RepositoryRepairSynthesisReceipt>(&path).unwrap())
            .find(|receipt| receipt.source_relative_path == Path::new("policy.py"))
            .expect("rollback repair receipt");
        assert!(repair.sandbox_verified);
        assert!(repair.sandbox_cleaned);
        assert!(!repair.candidate_installed);
        assert!(repair.rolled_back);
        assert_eq!(repair.authoritative_source_write_events, 2);
        assert!(repair
            .authoritative_command
            .as_ref()
            .is_some_and(|command| !command.success));
        assert_eq!(
            repair.failure_code.as_deref(),
            Some("PUBLIC_INFORMATION_INSUFFICIENT:AUTHORITATIVE_VALIDATION_FAILED")
        );
        assert!(repair.improvement_operators.is_empty());
        assert!(repository_repair_verifier_falsified(&repair));
        let history = repository_repair_history(
            &config.state_dir.join("diagnostics"),
            &repair.originating_validation_id,
            &repair.source_relative_path,
            &repair.predecessor_sha256,
        )
        .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].candidate_sha256, repair.candidate_sha256);
        let counterexamples = repository_repair_counterexample_candidate_sha256s(&history);
        assert!(repair
            .candidate_sha256
            .as_ref()
            .is_some_and(|candidate| counterexamples.contains(candidate)));
        assert_ne!(
            repository_repair_attempt_id(&repair.repair_problem_id, &"a".repeat(64)),
            repository_repair_attempt_id(&repair.repair_problem_id, &"b".repeat(64))
        );
        let _ = validate_blocked_repository_cohort(
            &config,
            &repository_repair_diagnostic(),
            &class_declaration_repair_cohort(predecessor),
        )
        .unwrap();
        assert_eq!(
            fs::read_dir(config.state_dir.join("diagnostics"))
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("repository_repair_synthesis_")
                })
                .count(),
            1
        );
        assert_eq!(recover_repository_install_transactions(&config).unwrap(), 0);
        assert!(!repository_install_transaction_directory(&config)
            .read_dir()
            .is_ok_and(|mut entries| entries.next().is_some()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repository_install_journal_recovers_uncommitted_and_finalizes_committed_patch() {
        let root = temp_root("repository-install-journal-recovery");
        let (_, mut config) = test_config(&root);
        config.repository_mutation.enabled = true;
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(&repository).unwrap();
        let relative = PathBuf::from("policy.py");
        let target = repository.join(&relative);
        let predecessor = b"marker = 'old'\n";
        let candidate_source = "marker = 'candidate'\n";
        fs::write(&target, predecessor).unwrap();
        let predecessor_sha256 = sha256(predecessor);
        let candidate_sha256 = sha256(candidate_source.as_bytes());
        let scope_paths = vec![relative.clone()];
        let scope_before = repository_validation_scope_fingerprint(
            &repository,
            &scope_paths,
            config.resources.max_file_bytes,
        )
        .unwrap();
        let repair_id = sha256(b"uncommitted repository install recovery");
        let (candidate, rollback) = repository_install_sibling_paths(&target, &repair_id).unwrap();
        let permissions = fs::metadata(&target).unwrap().permissions();
        let (predecessor_readonly, predecessor_unix_mode) =
            repository_permission_snapshot(&permissions);
        write_repository_candidate_sibling(&candidate, candidate_source, permissions).unwrap();
        let transaction = RepositoryInstallTransaction {
            schema: REPOSITORY_INSTALL_TRANSACTION_SCHEMA.to_string(),
            repair_id: repair_id.clone(),
            generation: 1,
            root_index: 0,
            source_relative_path: relative.clone(),
            predecessor_sha256: predecessor_sha256.clone(),
            candidate_sha256: candidate_sha256.clone(),
            scope_fingerprint_before: scope_before,
            scope_paths: scope_paths.clone(),
            candidate_file_name: candidate
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap()
                .to_string(),
            rollback_file_name: rollback
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap()
                .to_string(),
            predecessor_readonly,
            predecessor_unix_mode,
            pending_improvement_operators: Vec::new(),
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        write_immutable_json(
            &repository_install_transaction_path(&config, &repair_id),
            &transaction,
        )
        .unwrap();
        fs::rename(&target, &rollback).unwrap();
        fs::rename(&candidate, &target).unwrap();

        assert_eq!(recover_repository_install_transactions(&config).unwrap(), 1);
        assert_eq!(fs::read(&target).unwrap(), predecessor);
        assert!(!candidate.exists());
        assert!(!rollback.exists());

        let committed_candidate_source = "marker = 'committed'\n";
        let committed_candidate_sha256 = sha256(committed_candidate_source.as_bytes());
        let committed_repair_id = sha256(b"committed repository install recovery");
        let (committed_candidate, committed_rollback) =
            repository_install_sibling_paths(&target, &committed_repair_id).unwrap();
        let permissions = fs::metadata(&target).unwrap().permissions();
        let (predecessor_readonly, predecessor_unix_mode) =
            repository_permission_snapshot(&permissions);
        write_repository_candidate_sibling(
            &committed_candidate,
            committed_candidate_source,
            permissions,
        )
        .unwrap();
        let committed_scope_before = repository_validation_scope_fingerprint(
            &repository,
            &scope_paths,
            config.resources.max_file_bytes,
        )
        .unwrap();
        let committed_transaction = RepositoryInstallTransaction {
            schema: REPOSITORY_INSTALL_TRANSACTION_SCHEMA.to_string(),
            repair_id: committed_repair_id.clone(),
            generation: 2,
            root_index: 0,
            source_relative_path: relative,
            predecessor_sha256,
            candidate_sha256: committed_candidate_sha256.clone(),
            scope_fingerprint_before: committed_scope_before,
            scope_paths: scope_paths.clone(),
            candidate_file_name: committed_candidate
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap()
                .to_string(),
            rollback_file_name: committed_rollback
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap()
                .to_string(),
            predecessor_readonly,
            predecessor_unix_mode,
            pending_improvement_operators: Vec::new(),
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        let transaction_path = repository_install_transaction_path(&config, &committed_repair_id);
        write_immutable_json(&transaction_path, &committed_transaction).unwrap();
        fs::rename(&target, &committed_rollback).unwrap();
        fs::rename(&committed_candidate, &target).unwrap();
        let committed_scope_after = repository_validation_scope_fingerprint(
            &repository,
            &scope_paths,
            config.resources.max_file_bytes,
        )
        .unwrap();
        let commit = RepositoryInstallCommitReceipt {
            schema: REPOSITORY_INSTALL_COMMIT_SCHEMA.to_string(),
            repair_id: committed_repair_id.clone(),
            transaction_sha256: json_sha256(&committed_transaction).unwrap(),
            root_index: 0,
            source_relative_path: PathBuf::from("policy.py"),
            predecessor_sha256: sha256(predecessor),
            candidate_sha256: committed_candidate_sha256,
            scope_fingerprint_after: committed_scope_after,
            authoritative_command_sha256: "a".repeat(64),
            authoritative_output_sha256: "b".repeat(64),
            authoritative_source_write_events: 1,
            operator_selected: false,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
        };
        write_immutable_json(
            &repository_install_commit_path(&config, &committed_repair_id),
            &commit,
        )
        .unwrap();

        assert_eq!(recover_repository_install_transactions(&config).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            committed_candidate_source
        );
        assert!(!committed_candidate.exists());
        assert!(!committed_rollback.exists());
        assert!(!transaction_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pytest_failure_targets_preserve_qualified_public_owner() {
        let targets = python_pytest_target_symbols(
            ">       assert Rational.distance(9, 4) == 5\n\
             E       assert 13 == 5\n\
             E        +  where 13 = <function Rational.distance at 0x1>(9, 4)\n\
             FAILED tests/test_engine.py::test_distance",
        );
        assert_eq!(targets, ["Rational.distance"]);
    }

    #[test]
    fn source_bound_operator_capacity_prefers_latest_verified_not_hash_prefix() {
        let latest_verified_generation = BTreeMap::from([
            ("000-old-hash".to_string(), 2),
            ("fff-latest-hash".to_string(), 9),
            ("aaa-same-generation".to_string(), 9),
        ]);
        let selected = select_bounded_typed_mechanism_operator_ids(
            [
                "000-old-hash".to_string(),
                "fff-latest-hash".to_string(),
                "aaa-same-generation".to_string(),
                "fff-latest-hash".to_string(),
            ],
            &latest_verified_generation,
            2,
        );
        assert_eq!(selected, ["aaa-same-generation", "fff-latest-hash"]);
        assert!(!selected.contains(&"000-old-hash".to_string()));
    }

    #[test]
    fn blocked_rust_repository_cohort_selects_crate_local_lib_tests() {
        let Ok(cargo) = resolve_local_program("cargo") else {
            return;
        };
        let root = temp_root("blocked-rust-repository-validation");
        let (_, mut config) = test_config(&root);
        config.source_mutation.cargo_executable = cargo;
        let repository = config.watched_roots[0].clone();
        fs::create_dir_all(repository.join("crates/example/src")).unwrap();
        fs::write(
            repository.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            repository.join("Cargo.lock"),
            "# This file is automatically @generated by Cargo.\nversion = 4\n",
        )
        .unwrap();
        fs::write(
            repository.join("crates/example/Cargo.toml"),
            "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        fs::write(
            repository.join("crates/example/src/lib.rs"),
            "pub fn repaired() -> bool { true }\n#[cfg(test)] mod tests { #[test] fn pass() { assert!(super::repaired()); } }\n",
        )
        .unwrap();
        let implementation = LearningObservation {
            observation_id: "rust-implementation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/crates/example/src/lib.rs".to_string(),
            content_sha256: "e".repeat(64),
            predecessor_content_sha256: Some("d".repeat(64)),
            actor: WorkActor::Codex,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Unknown,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures::default(),
            signals: vec!["DEFECT_REPAIR".to_string(), "TEST_ADDED".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 75,
            learning_value: LearningValue::High,
            reasons: vec!["rust repair".to_string()],
            verification_evidence_sha256: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };

        let plan = repository_validation_plan(&config, &[implementation])
            .unwrap()
            .expect("rust cargo validation plan");

        assert_eq!(plan.validator_kind, RepositoryValidatorKind::RustCargo);
        assert_eq!(plan.test_selection_source, "CRATE_LOCAL_LIB_TESTS");
        assert_eq!(
            plan.test_paths,
            vec![PathBuf::from("crates/example/Cargo.toml")]
        );
        assert_eq!(plan.input_observation_ids, vec!["rust-implementation"]);
        assert!(plan.args.windows(2).any(|pair| {
            pair == [
                "--manifest-path".to_string(),
                "crates/example/Cargo.toml".to_string(),
            ]
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn autonomous_bootstrap_receipt_does_not_become_text_only_growth() {
        let receipt = inspect_self(SelfInspectionInput {
            generation: 0,
            supervisor_sequence: 12,
            files_scanned: 100,
            files_reused: 100,
            files_hashed: 0,
            scan_duration_ms: 100,
            pending_work_events: 0,
            replayed_unchanged_work_events: 0,
            naive_cohort_has_verification: false,
            evidence_aware_cohort_has_verification: false,
            autonomous_campaigns_enabled: true,
            campaigns_started: 0,
            mutual_revalidation_events: 0,
            evaluator_challenge_cases: 6,
            evaluator_required_challenge_cases: EvaluatorMutationKind::ALL.len() as u64,
            consecutive_failures: 0,
            plateau_scans: 12,
            unconsumed_high_observations: 0,
            cohort_preflight_ready: false,
            core_cohort_validation_applicable: false,
            repository_cohort_validation_applicable: false,
            source_patch_attempts: 0,
            source_patch_installations: 0,
            source_patch_rollbacks: 0,
            source_patch_consecutive_failures: 0,
            source_patch_validation_ms: 0,
            source_discovery_no_candidate_streak: 0,
            last_source_discovery_reason: None,
            active_runtime_ms: 0,
            diagnostic_policy: DiagnosticPolicyMemory::default(),
        })
        .unwrap();
        let root = temp_root("mutual-bootstrap-cohort");
        let (_, config) = test_config(&root);
        let (_, observation) = runtime_repair_action(&config, &receipt, &[], &[], &[])
            .unwrap()
            .expect("actionable bootstrap action");
        let observation = observation.expect("actionable bootstrap observation");
        assert!(!campaign_preflight_ready(&config, std::slice::from_ref(&observation)).unwrap());
        let lesson = build_lesson(&[observation]).unwrap();
        let next = derive_next_evaluator_memory(&EvaluatorMemory::default(), &[], &lesson).unwrap();
        assert_eq!(next.generation, 1);
        assert_eq!(next.challenge_suite.len(), EvaluatorMutationKind::ALL.len());
        assert_eq!(next.accepted_expansions, 1);
        assert_eq!(next.capability_expansion_contract_revision, 2);
        let saturated = derive_next_evaluator_memory(&next, &[], &lesson).unwrap();
        assert_eq!(saturated.challenge_suite.len(), next.challenge_suite.len());
        assert_eq!(saturated.accepted_expansions, 1);
        let mut legacy = next.clone();
        legacy.capability_expansion_contract_revision = 0;
        legacy.accepted_expansions = 75;
        let migrated = derive_next_evaluator_memory(&legacy, &[], &lesson).unwrap();
        assert_eq!(migrated.accepted_expansions, 1);
        assert_eq!(migrated.legacy_unbound_accepted_expansions, 74);
        assert!(lesson_has_verification_evidence(&lesson));
        assert!(!lesson.raw_source_bytes_present);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_accepts_only_bound_independent_candidate() {
        let root = temp_root("verify-accept");
        let (freeze, candidate, request) = accepted_candidate(&root);
        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Accept);
        validate_receipt(&freeze, &candidate, &receipt).unwrap();
        assert!(!receipt.verifier_is_proposer);
        assert_eq!(receipt.network_reads, 0);
        assert!(receipt.evaluator_self_audit.pass);
        assert_eq!(receipt.evaluator_self_audit.mutation_cases, 10);
        assert_eq!(receipt.evaluator_self_audit.mutation_survivors, 0);
        assert_eq!(receipt.evaluator_self_audit.active_evaluator_generation, 0);
        assert_eq!(
            receipt.evaluator_self_audit.proposed_evaluator_generation,
            1
        );
        assert!(receipt.evaluator_self_audit.post_challenge_core_revalidated);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn evaluator_audit_challenges_only_new_lesson_after_sealed_predecessor() {
        let root = temp_root("incremental-evaluator-audit");
        let (_, candidate, _) = accepted_candidate(&root);
        let expected_lesson = candidate.lesson.clone();
        let mut prior_lesson = expected_lesson.clone();
        prior_lesson.lesson_id = "sealed-prior-lesson".to_string();
        prior_lesson.evidence_observation_sha256 = vec!["b".repeat(64)];
        let prior_evaluator =
            derive_next_evaluator_memory(&EvaluatorMemory::default(), &[], &prior_lesson).unwrap();
        let predecessor = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 1,
            predecessor_sha256: None,
            lessons: vec![prior_lesson],
            classifier: ClassifierMemory::default(),
            evaluator: prior_evaluator,
            generative: GenerativeGrowthMemory::default(),
        };
        let proposed_evaluator = derive_next_evaluator_memory(
            &predecessor.evaluator,
            &predecessor.lessons,
            &expected_lesson,
        )
        .unwrap();

        let audit = evaluator_self_audit(
            &candidate,
            &expected_lesson,
            candidate.total_learning_score,
            &predecessor,
            &proposed_evaluator,
            7,
        );

        assert!(audit.pass);
        assert_eq!(audit.mutation_cases, EvaluatorMutationKind::ALL.len());
        assert_eq!(audit.knowledge_challenge_cases, audit.mutation_cases);
        assert_eq!(audit.mutation_survivors, 0);
        assert_eq!(audit.active_evaluator_generation, 1);
        assert_eq!(audit.proposed_evaluator_generation, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_raw_source_or_self_approval() {
        let root = temp_root("verify-reject");
        let (freeze, mut candidate, mut request) = accepted_candidate(&root);
        candidate.raw_source_bytes = 1;
        candidate.self_approval_events = 1;
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();
        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Reject);
        validate_receipt(&freeze, &candidate, &receipt).unwrap();
        assert!(receipt
            .reasons
            .contains(&"RAW_OR_EXACT_SOLUTION_DATA_PRESENT".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_candidate_evidence_not_derived_from_frozen_observations() {
        let root = temp_root("verify-evidence-binding");
        let (_freeze, mut candidate, mut request) = accepted_candidate(&root);
        candidate.lesson.evidence_observation_sha256 = vec!["9".repeat(64)];
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();
        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"CANDIDATE_NOT_DERIVED_FROM_FROZEN_OBSERVATIONS".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_verification_only_generation_claim() {
        let root = temp_root("verify-verification-only");
        let (_freeze, mut candidate, mut request) = accepted_candidate(&root);
        candidate.lesson.work_kinds = vec![WorkKind::Verification];
        candidate
            .lesson
            .diagnostic_signals
            .retain(|signal| signal != "MUTUAL_REVALIDATION_GAP");
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();

        let receipt = run_verifier_request(&request).unwrap();

        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"VERIFICATION_ONLY_COHORT_HAS_NO_GROWTH_SUBJECT".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_generative_prediction_not_derived_before_composition() {
        let root = temp_root("verify-generative-binding");
        let (_freeze, mut candidate, mut request) = accepted_candidate(&root);
        candidate.generative_cycle.predicted_value =
            candidate.generative_cycle.predicted_value.saturating_add(1);
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();
        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"CANDIDATE_NOT_DERIVED_FROM_FROZEN_OBSERVATIONS".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_unearned_frontier_claim() {
        let root = temp_root("verify-unearned-frontier");
        let (_freeze, mut candidate, mut request) = accepted_candidate(&root);
        assert!(candidate.generative_cycle.behavioral_composition_executed);
        assert!(candidate.generative_cycle.accepted_for_memory);
        candidate.generative_cycle.accepted_for_memory = false;
        candidate.generative_cycle.frontier_advance = true;
        candidate.generative_cycle.applied_to_self_improvement = true;
        candidate
            .generative_cycle
            .applied_policy_signals
            .push("DEFECT_REPAIR".to_string());
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();

        let receipt = run_verifier_request(&request).unwrap();

        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"CANDIDATE_NOT_DERIVED_FROM_FROZEN_OBSERVATIONS".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_tampered_behavioral_execution_receipt() {
        let root = temp_root("verify-behavioral-receipt");
        let (_freeze, mut candidate, mut request) = accepted_candidate(&root);
        candidate
            .generative_cycle
            .behavioral_execution_receipt
            .as_mut()
            .expect("execution receipt")
            .receipt_sha256 = "0".repeat(64);
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();

        let receipt = run_verifier_request(&request).unwrap();

        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"GENERATIVE_COMPOSITION_BOUNDARY_FAILURE".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn verifier_rejects_unbound_predecessor_evaluator_memory() {
        let root = temp_root("verify-evaluator-memory-binding");
        let (_freeze, _candidate, request) = accepted_candidate(&root);
        let path = root.join("predecessor_memory.json");
        fs::remove_file(&path).unwrap();
        let mut memory = GrowthMemory {
            schema: SUPERVISOR_SCHEMA.to_string(),
            generation: 0,
            predecessor_sha256: None,
            lessons: Vec::new(),
            classifier: ClassifierMemory::default(),
            evaluator: EvaluatorMemory::default(),
            generative: GenerativeGrowthMemory::default(),
        };
        memory.evaluator.generation = 9;
        write_immutable_json(&path, &memory).unwrap();
        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"FROZEN_OBSERVATION_OR_MEMORY_BINDING_FAILURE".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repeated_promotion_after_crash_is_idempotent() {
        let root = temp_root("idempotent-promotion");
        let (config_path, config) = test_config(&root);
        let state = initialize(&config_path).unwrap();
        let (mut freeze, mut candidate, mut request) = accepted_candidate(&root);
        freeze.predecessor_memory_sha256 = state.current_memory_sha256.clone();
        freeze.config_sha256 = state.config_sha256.clone();
        candidate.predecessor_memory_sha256 = state.current_memory_sha256.clone();
        candidate.freeze_sha256 = json_sha256(&freeze).unwrap();
        fs::remove_file(&request.freeze_path).unwrap();
        fs::remove_file(&request.candidate_path).unwrap();
        write_immutable_json(&request.freeze_path, &freeze).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        request.expected_freeze_sha256 = json_sha256(&freeze).unwrap();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();
        let receipt = run_verifier_request(&request).unwrap();
        let campaign = campaign_dir(&config, &freeze.campaign_id);
        fs::create_dir_all(&campaign).unwrap();
        let observation = LearningObservation {
            observation_id: "observation".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/lib.rs".to_string(),
            content_sha256: "f".repeat(64),
            predecessor_content_sha256: None,
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::DefectRepair,
            work_outcome: WorkOutcome::Pass,
            features_before: None,
            features_after: StructuralFeatures::default(),
            signals: vec!["VERIFIED_PASS".to_string()],
            composition_roles: vec!["IMPLEMENTATION_REPAIR".to_string()],
            learning_score: 70,
            learning_value: LearningValue::High,
            reasons: vec!["test".to_string()],
            verification_evidence_sha256: vec!["a".repeat(64)],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        write_immutable_json(&campaign.join("observation_observation.json"), &observation).unwrap();

        let mut first_state = state.clone();
        let mut first_index = FileIndex::default();
        let first_hash = promote_candidate(
            &config,
            &mut first_state,
            &mut first_index,
            &freeze,
            &candidate,
            &receipt,
        )
        .unwrap();

        let mut recovered_state = state;
        let mut recovered_index = load_index(&config).unwrap();
        let recovered_hash = promote_candidate(
            &config,
            &mut recovered_state,
            &mut recovered_index,
            &freeze,
            &candidate,
            &receipt,
        )
        .unwrap();
        assert_eq!(first_hash, recovered_hash);
        assert_eq!(recovered_state.generation, 1);
        assert_eq!(first_state.evaluator_generation, 1);
        assert_eq!(recovered_state.evaluator_generation, 1);
        assert_eq!(first_state.mutual_revalidation_events, 1);
        assert_eq!(recovered_state.mutual_revalidation_events, 1);
        assert_eq!(first_state.evaluator_challenge_cases, 10);
        assert_eq!(first_state.generative_predictions, 1);
        assert_eq!(first_state.valuable_combinations_learned, 1);
        assert_eq!(first_state.generative_self_application_events, 0);
        assert_eq!(first_state.generative_frontier_advance_events, 1);
        assert_eq!(
            first_state.unverified_generative_frontier_candidate_events,
            0
        );
        assert_eq!(recovered_state.generative_predictions, 1);
        let promoted = load_memory(&config, 1).unwrap();
        assert_eq!(promoted.generative.accepted_compositions.len(), 1);
        assert_eq!(promoted.generative.distinct_verified_artifact_count(), 1);
        assert_eq!(promoted.generative.self_application_events, 0);
        assert_eq!(promoted.generative.frontier_advance_events, 1);
        assert_eq!(promoted.generative.unverified_frontier_candidate_events, 0);
        assert_eq!(
            first_state.current_evaluator_memory_sha256,
            recovered_state.current_evaluator_memory_sha256
        );
        assert_eq!(
            fs::read_dir(config.state_dir.join("memory"))
                .unwrap()
                .count(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn secret_files_are_never_collected() {
        let root = temp_root("secrets");
        let (_, config) = test_config(&root);
        fs::write(config.watched_roots[0].join(".env"), "TOKEN=do-not-read").unwrap();
        fs::write(
            config.watched_roots[0].join("safe.rs"),
            "pub fn safe() {}\n",
        )
        .unwrap();
        let files = collect_files(
            &config.watched_roots,
            &config.observation,
            config.resources.max_files_per_scan,
        )
        .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "safe.rs");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn operator_stop_is_observed_at_step_boundary() {
        let root = temp_root("stop");
        let (config_path, _) = test_config(&root);
        initialize(&config_path).unwrap();
        request_stop(&config_path).unwrap();
        let report = supervisor_step(&config_path).unwrap();
        assert_eq!(report.phase, SupervisorPhase::SafeStopped);
        assert_eq!(
            report.stop_reason.as_deref(),
            Some("OPERATOR_STOP_REQUESTED")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn transient_scan_timeout_can_be_resumed_after_repair() {
        let root = temp_root("resume-scan-timeout");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.stop_reason = Some("SCAN_RUNTIME_BOUND_REACHED".to_string());
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "TEST_TRANSIENT_TIMEOUT",
        )
        .unwrap();
        let response = request_resume(&config_path).unwrap();
        assert_eq!(response["phase"], "INFRA_READY");
        assert_eq!(response["hard_resource_stop_preserved"], false);
        let resumed = status(&config_path).unwrap();
        assert_eq!(resumed.phase, SupervisorPhase::InfraReady);
        assert_eq!(resumed.stop_reason, None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_autonomous_source_update_can_resume_after_binary_swap() {
        let root = temp_root("resume-source-update");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.stop_reason = Some("AUTONOMOUS_SOURCE_UPDATE_STAGED".to_string());
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "TEST_SOURCE_UPDATE_STAGED",
        )
        .unwrap();
        let response = request_resume(&config_path).unwrap();
        assert_eq!(response["phase"], "INFRA_READY");
        assert_eq!(response["hard_resource_stop_preserved"], false);
        let resumed = status(&config_path).unwrap();
        assert_eq!(resumed.phase, SupervisorPhase::InfraReady);
        assert_eq!(resumed.stop_reason, None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_composite_capability_can_resume_after_binary_swap() {
        let root = temp_root("resume-composite-capability");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.stop_reason = Some("AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED".to_string());
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "TEST_COMPOSITE_CAPABILITY_STAGED",
        )
        .unwrap();
        let response = request_resume(&config_path).unwrap();
        assert_eq!(response["phase"], "INFRA_READY");
        assert_eq!(response["hard_resource_stop_preserved"], false);
        let resumed = status(&config_path).unwrap();
        assert_eq!(resumed.phase, SupervisorPhase::InfraReady);
        assert_eq!(resumed.stop_reason, None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_update_cannot_resume_before_wrapper_consumes_handoff() {
        let root = temp_root("resume-before-handoff-application");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        state.stop_reason = Some("AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED".to_string());
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::SafeStopped,
            "TEST_COMPOSITE_CAPABILITY_STAGED",
        )
        .unwrap();
        fs::write(
            config
                .state_dir
                .join("control")
                .join(crate::autonomous_source_mutation::SELF_UPDATE_HANDOFF_FILE),
            b"pending wrapper application",
        )
        .unwrap();

        let response = request_resume(&config_path).unwrap();

        assert_eq!(response["resume_requested"], false);
        assert_eq!(response["pending_self_update"], true);
        assert_eq!(response["phase"], "SAFE_STOPPED");
        let preserved = status(&config_path).unwrap();
        assert_eq!(preserved.phase, SupervisorPhase::SafeStopped);
        assert_eq!(
            preserved.stop_reason.as_deref(),
            Some("AUTONOMOUS_COMPOSITE_CAPABILITY_STAGED")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn full_hash_canary_is_distributed_instead_of_rehashing_every_file() {
        let root = temp_root("distributed-canary");
        let (config_path, config) = test_config(&root);
        for index in 0..128 {
            fs::write(
                config.watched_roots[0].join(format!("file_{index:03}.rs")),
                format!("pub fn value_{index}() -> usize {{ {index} }}\n"),
            )
            .unwrap();
        }
        let state = initialize(&config_path).unwrap();
        let baseline = supervisor_step(&config_path).unwrap();
        assert_eq!(baseline.last_scan_files_hashed, 128);
        let mut index = load_index(&config).unwrap();
        index.sequence = FULL_HASH_CANARY_INTERVAL - 1;
        save_index(&config, &mut index).unwrap();
        let memory = load_memory(&config, state.generation).unwrap();
        let scan = scan_watched_roots(&config, &memory).unwrap();
        assert!(scan.files_hashed < 128);
        assert_eq!(scan.files_hashed + scan.files_reused, 128);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fresh_baseline_is_accumulated_in_bounded_batches() {
        let root = temp_root("batched-baseline");
        let (config_path, config) = test_config(&root);
        for index in 0..(BASELINE_MAX_HASHED_FILES_PER_SCAN + 17) {
            fs::write(
                config.watched_roots[0].join(format!("file_{index:04}.rs")),
                format!("pub fn value_{index}() -> usize {{ {index} }}\n"),
            )
            .unwrap();
        }
        initialize(&config_path).unwrap();
        let first = supervisor_step(&config_path).unwrap();
        assert!(first.baseline_created);
        assert_eq!(
            first.last_scan_files_hashed as usize,
            BASELINE_MAX_HASHED_FILES_PER_SCAN
        );
        assert!(!load_index(&config).unwrap().baseline_complete);

        let second = supervisor_step(&config_path).unwrap();
        assert!(second.baseline_created);
        assert_eq!(second.last_scan_files_hashed, 17);
        assert!(load_index(&config).unwrap().baseline_complete);
        assert_eq!(second.observations_created, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn excluded_files_do_not_block_baseline_completion() {
        let root = temp_root("baseline-file-exclusions");
        let (config_path, config) = test_config(&root);
        fs::write(
            config.watched_roots[0].join("eligible.rs"),
            "pub fn eligible() {}\n",
        )
        .unwrap();
        let oversized = vec![b'x'; config.resources.max_file_bytes as usize + 1];
        fs::write(config.watched_roots[0].join("oversized.rs"), oversized).unwrap();
        fs::write(
            config.watched_roots[0].join("generated.rs"),
            "// automatically generated\npub fn generated() {}\n",
        )
        .unwrap();
        fs::write(
            config.watched_roots[0].join("non_utf8.rs"),
            [0xff, 0xfe, 0xfd],
        )
        .unwrap();
        initialize(&config_path).unwrap();
        let report = supervisor_step(&config_path).unwrap();
        assert!(report.baseline_created);
        let index = load_index(&config).unwrap();
        assert!(index.baseline_complete);
        assert_eq!(index.files.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn semantic_revalidation_does_not_create_a_generation() {
        let root = temp_root("semantic-revalidation");
        let (config_path, config) = test_config(&root);
        let mut state = initialize(&config_path).unwrap();
        let (_, candidate, _) = accepted_candidate(&root);
        let observation: LearningObservation =
            read_json(&root.join("observation_observation.json")).unwrap();
        let mut memory = load_memory(&config, 0).unwrap();
        memory.lessons.push(candidate.lesson);
        let mut index = FileIndex::default();
        state.diagnostic_policy.experiment_records.insert(
            "MIXED_ROLE_COHORT_RECONSTRUCTION".to_string(),
            crate::autonomous_self_inspection::DiagnosticExperimentMemory {
                trials: 1,
                causal_support_events: 1,
                ..crate::autonomous_self_inspection::DiagnosticExperimentMemory::default()
            },
        );
        state.diagnostic_policy.active_experiment_id =
            Some("MIXED_ROLE_COHORT_RECONSTRUCTION".to_string());
        state.diagnostic_policy.active_generation = Some(0);
        state.diagnostic_policy.active_causal_support = true;
        state.diagnostic_policy.active_action_id = Some("validation-action".to_string());
        state.diagnostic_policy.active_action_receipt_sha256 = Some("receipt".to_string());
        state.diagnostic_policy.active_output_observation_ids =
            vec![observation.observation_id.clone()];
        state.diagnostic_policy.outcome_bound_selections = 1;

        let consumed =
            consume_semantic_revalidation(&config, &mut state, &mut index, &memory, &[observation])
                .unwrap();

        assert_eq!(consumed, Some(1));
        assert_eq!(state.generation, 0);
        assert_eq!(state.campaigns_started, 0);
        assert_eq!(state.semantic_revalidation_events, 1);
        assert_eq!(state.redundant_observations_consumed, 1);
        assert_eq!(index.consumed_observation_ids.len(), 1);
        assert_eq!(index.consumed_work_event_ids.len(), 1);
        assert_eq!(state.diagnostic_policy.productive_outcome_events, 1);
        assert_eq!(state.diagnostic_policy.failed_outcome_events, 0);
        assert!(state.diagnostic_policy.active_action_id.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn independent_verifier_rejects_a_semantically_duplicate_lesson() {
        let root = temp_root("verifier-semantic-duplicate");
        let (mut freeze, mut candidate, mut request) = accepted_candidate(&root);
        let mut predecessor: GrowthMemory =
            read_json(&root.join("predecessor_memory.json")).unwrap();
        predecessor.lessons.push(candidate.lesson.clone());
        freeze.predecessor_memory_sha256 = json_sha256(&predecessor).unwrap();
        candidate.predecessor_memory_sha256 = freeze.predecessor_memory_sha256.clone();
        candidate.freeze_sha256 = json_sha256(&freeze).unwrap();
        request.expected_freeze_sha256 = candidate.freeze_sha256.clone();
        request.expected_candidate_sha256 = json_sha256(&candidate).unwrap();

        fs::remove_file(&request.freeze_path).unwrap();
        fs::remove_file(&request.candidate_path).unwrap();
        fs::remove_file(root.join("predecessor_memory.json")).unwrap();
        write_immutable_json(&request.freeze_path, &freeze).unwrap();
        write_immutable_json(&request.candidate_path, &candidate).unwrap();
        write_immutable_json(&root.join("predecessor_memory.json"), &predecessor).unwrap();

        let receipt = run_verifier_request(&request).unwrap();
        assert_eq!(receipt.decision, GrowthDecision::Reject);
        assert!(receipt
            .reasons
            .contains(&"DUPLICATE_SEMANTIC_LESSON".to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn observed_to_expected_contract_cannot_enter_without_a_typed_behavior_goal() {
        let mut delta = public_contract_delta_fixture();
        delta.typed_behavior_goals.clear();

        assert_eq!(
            validate_public_contract_deltas(&[delta]),
            Err("EVENT_PUBLIC_CONTRACT_DELTA_INVALID".to_string())
        );
    }

    #[test]
    fn unrelated_typed_goal_cannot_satisfy_an_observed_to_expected_contract() {
        let mut delta = public_contract_delta_fixture();
        delta.expected_behavior = "call returns base multiplied by gain".to_string();

        assert_eq!(
            validate_public_contract_deltas(&[delta]),
            Err("EVENT_TYPED_BEHAVIOR_GOAL_CONTRACT_BINDING_MISSING".to_string())
        );
    }

    #[test]
    fn typed_behavior_goal_survives_event_observation_lesson_and_generative_input() {
        let delta = public_contract_delta_fixture();
        validate_public_contract_deltas(std::slice::from_ref(&delta)).unwrap();
        let current = FileFingerprint {
            content_sha256: "b".repeat(64),
            bytes: 100,
            modified_ms: 1,
            extension: "rs".to_string(),
            features: StructuralFeatures {
                public_symbols: 1,
                ..StructuralFeatures::default()
            },
        };
        let event = WorkEvent {
            event_id: "typed-public-contract".to_string(),
            actor: WorkActor::LocalTool,
            kind: WorkKind::DefectRepair,
            paths: Vec::new(),
            outcome: WorkOutcome::Unknown,
            summary: "forensic context must not replace the typed contract".to_string(),
            evidence_sha256: Vec::new(),
            evidence_artifacts: Vec::new(),
            performance_metrics: Vec::new(),
            public_contract_deltas: vec![delta.clone()],
            occurred_at_ms: 1,
        };

        let observation = classify_observation(
            "ROOT_0/src/engine.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            10,
        );
        assert_eq!(observation.public_contract_deltas, vec![delta.clone()]);
        let lesson = build_lesson(std::slice::from_ref(&observation)).unwrap();
        assert_eq!(lesson.public_contract_deltas, vec![delta.clone()]);
        assert!(lesson
            .diagnostic_signals
            .contains(&"TYPED_BEHAVIOR_GOAL_AVAILABLE".to_string()));
        let input = generative_input(&lesson);
        assert_eq!(input.typed_behavior_goals, delta.typed_behavior_goals);
    }

    #[test]
    fn measured_performance_gain_is_bound_and_value_sensitive() {
        let metric = PerformanceMetricEvidence {
            metric: "scan_latency_ns".to_string(),
            before: 100,
            after: 80,
            lower_is_better: true,
            evidence_sha256: "a".repeat(64),
            executable_knowledge: None,
        };
        let current = FileFingerprint {
            content_sha256: "b".repeat(64),
            bytes: 100,
            modified_ms: 1,
            extension: "rs".to_string(),
            features: StructuralFeatures {
                benchmark_tokens: 1,
                performance_tokens: 1,
                ..StructuralFeatures::default()
            },
        };
        let event = WorkEvent {
            event_id: "performance-gain".to_string(),
            actor: WorkActor::LocalTool,
            kind: WorkKind::PerformanceOptimization,
            paths: Vec::new(),
            outcome: WorkOutcome::Pass,
            summary: "bounded benchmark improved".to_string(),
            evidence_sha256: vec!["a".repeat(64)],
            evidence_artifacts: Vec::new(),
            performance_metrics: vec![metric],
            public_contract_deltas: Vec::new(),
            occurred_at_ms: 1,
        };
        let observation = classify_observation(
            "ROOT_0/src/scanner.rs".to_string(),
            &current,
            None,
            Some(&event),
            &ClassifierMemory::default(),
            45,
        );
        assert_eq!(observation.learning_value, LearningValue::High);
        assert!(observation
            .signals
            .contains(&"MEASURED_PERFORMANCE_GAIN".to_string()));
        let lesson = build_lesson(std::slice::from_ref(&observation)).unwrap();

        let mut independently_remeasured = observation.clone();
        independently_remeasured.performance_metrics[0].evidence_sha256 = "c".repeat(64);
        let repeated = build_lesson(&[independently_remeasured]).unwrap();
        assert_eq!(
            lesson_semantic_sha256(&lesson).unwrap(),
            lesson_semantic_sha256(&repeated).unwrap()
        );

        let mut better = observation;
        better.performance_metrics[0].after = 60;
        let better_lesson = build_lesson(&[better]).unwrap();
        assert_eq!(
            lesson_semantic_sha256(&lesson).unwrap(),
            lesson_semantic_sha256(&better_lesson).unwrap()
        );
        assert!(!lesson_has_executable_knowledge(&lesson));
    }

    #[test]
    fn structural_delta_profile_distinguishes_capabilities_without_exact_source_identity() {
        let base = LearningObservation {
            observation_id: "first-shape".to_string(),
            work_event_id: None,
            logical_path: "ROOT_0/src/first.rs".to_string(),
            content_sha256: "a".repeat(64),
            predecessor_content_sha256: Some("b".repeat(64)),
            actor: WorkActor::UnknownLocalWriter,
            work_kind: WorkKind::CodeChange,
            work_outcome: WorkOutcome::Pass,
            features_before: Some(StructuralFeatures::default()),
            features_after: StructuralFeatures {
                public_symbols: 1,
                branch_tokens: 3,
                ..StructuralFeatures::default()
            },
            signals: vec!["CODE_CHANGE".to_string(), "VERIFIED_PASS".to_string()],
            composition_roles: vec!["IMPLEMENTATION".to_string(), "REGRESSION_TEST".to_string()],
            learning_score: 80,
            learning_value: LearningValue::High,
            reasons: vec!["bounded verified change".to_string()],
            verification_evidence_sha256: vec!["c".repeat(64)],
            performance_metrics: Vec::new(),
            public_contract_deltas: Vec::new(),
            exact_source_fragments_stored: 0,
            raw_source_bytes_stored: 0,
            observed_at_ms: 1,
        };
        let first = build_lesson(std::slice::from_ref(&base)).unwrap();
        assert!(first
            .diagnostic_signals
            .contains(&"STRUCTURAL_DELTA:PUBLIC_SYMBOL:INCREASE:ONE".to_string()));
        assert!(first
            .diagnostic_signals
            .contains(&"STRUCTURAL_DELTA:BRANCH:INCREASE:SMALL".to_string()));

        let mut same_shape = base.clone();
        same_shape.observation_id = "renamed-shape".to_string();
        same_shape.logical_path = "ROOT_0/src/renamed.rs".to_string();
        same_shape.content_sha256 = "d".repeat(64);
        same_shape.predecessor_content_sha256 = Some("e".repeat(64));
        let repeated = build_lesson(&[same_shape]).unwrap();
        assert_eq!(
            lesson_semantic_sha256(&first).unwrap(),
            lesson_semantic_sha256(&repeated).unwrap()
        );

        let mut different_shape = base;
        different_shape.features_after.branch_tokens = 0;
        different_shape.features_after.assertion_tokens = 3;
        let distinct = build_lesson(&[different_shape]).unwrap();
        assert_eq!(
            lesson_semantic_sha256(&first).unwrap(),
            lesson_semantic_sha256(&distinct).unwrap()
        );
        assert!(!lesson_has_executable_knowledge(&first));
    }

    #[test]
    fn semantic_feature_vector_observes_constructors_and_data_composition() {
        let features = structural_features(
            "fn compose(value: i32) -> Option<String> { Some(format!(\"{}\", value)) }",
            "rs",
        );

        assert!(features.algebraic_constructor_tokens >= 2);
        assert!(features.data_composition_tokens >= 1);

        let observation = classify_observation(
            "ROOT_0/src/composition.rs".to_string(),
            &FileFingerprint {
                content_sha256: "a".repeat(64),
                bytes: 1,
                modified_ms: 1,
                extension: "rs".to_string(),
                features,
            },
            Some(&FileFingerprint {
                content_sha256: "b".repeat(64),
                bytes: 1,
                modified_ms: 0,
                extension: "rs".to_string(),
                features: StructuralFeatures::default(),
            }),
            None,
            &ClassifierMemory::default(),
            45,
        );
        assert!(observation
            .signals
            .contains(&"ALGEBRAIC_CONSTRUCTOR_MECHANISM".to_string()));
        assert!(observation
            .signals
            .contains(&"DATA_COMPOSITION_MECHANISM".to_string()));
        let lesson = build_lesson(&[observation]).unwrap();
        assert!(lesson
            .diagnostic_signals
            .iter()
            .any(|signal| signal.starts_with("STRUCTURAL_DELTA:ALGEBRAIC_CONSTRUCTOR:")));
        assert!(lesson
            .diagnostic_signals
            .iter()
            .any(|signal| signal.starts_with("STRUCTURAL_DELTA:DATA_COMPOSITION:")));
    }

    #[test]
    fn rust_feature_family_ignores_quoted_source_fixtures() {
        let features = structural_features(
            r###"
const QUOTED_SOURCE: &str = r#"
pub fn fake() -> Option<i32> {
    todo!();
    assert!(true);
    Some(1)
}
"#;

pub fn real(value: i32) -> Result<i32, String> {
    if value > 0 { Ok(value) } else { Err("invalid".to_string()) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_positive_value() {
        assert!(real(1).is_ok());
    }
}
"###,
            "rs",
        );

        assert_eq!(features.public_symbols, 1);
        assert_eq!(features.todo_tokens, 0);
        assert_eq!(features.assertion_tokens, 1);
        assert!(features.branch_tokens >= 2);
        assert!(features.algebraic_constructor_tokens >= 3);
        assert!(features.data_composition_tokens >= 1);
    }
}
