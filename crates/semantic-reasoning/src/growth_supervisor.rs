//! Always-on, bounded growth coordination.
//!
//! The supervisor learns generalized structural lessons from explicitly scoped
//! local work. It never records raw source fragments, reads outside configured
//! roots, calls a network/LLM, approves its own candidate, or raises research
//! difficulty to escape a plateau. A separate deterministic verifier process
//! must accept every candidate before a new memory generation is promoted.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::autonomous_self_inspection::{
    inspect as inspect_self, AutonomousSelfInspectionReceipt, InternalBottleneckClass,
    RepairDisposition, SelfInspectionInput,
};
use crate::generative_growth::{
    promote_generative_cycle, run_generative_cycle, GenerativeCycleResult, GenerativeGrowthMemory,
    GenerativeInput,
};
use crate::self_repair_contract::sha256;

pub const SUPERVISOR_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_SUPERVISOR_1";
pub const CONFIG_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_CONFIG_1";
pub const VERIFIER_SCHEMA: &str = "B_CORE_BOUNDED_GROWTH_VERIFIER_1";
const MAX_SUMMARY_BYTES: usize = 512;
const SCAN_WATCHDOG_TICK_MS: u64 = 1_000;
const MAX_SCAN_RUNTIME_MS: u64 = 60_000;
const FULL_HASH_CANARY_INTERVAL: u64 = 64;
const BASELINE_MAX_HASHED_FILES_PER_SCAN: usize = 1_024;
const BASELINE_MAX_BYTES_PER_SCAN: u64 = 64 * 1024 * 1024;

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
    pub runtime_self_repairs_activated: u64,
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
    DefectRepair,
    RegressionTest,
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
    pub occurred_at_ms: u64,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorMemory {
    pub schema: String,
    pub generation: u64,
    pub predecessor_sha256: Option<String>,
    pub challenge_suite: Vec<EvaluatorMutationKind>,
    pub source_lesson_ids: Vec<String>,
    pub accepted_expansions: u64,
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
    pub runtime_self_repairs_activated: u64,
    pub self_repair_capability_gaps: u64,
    pub last_internal_bottleneck: Option<String>,
    pub evaluator_generation: u64,
    pub evaluator_challenge_cases: u64,
    pub mutual_revalidation_events: u64,
    pub generative_predictions: u64,
    pub valuable_combinations_learned: u64,
    pub generative_memory_reuse_events: u64,
    pub generative_self_application_events: u64,
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
    pub promoted_lessons_drive_executable_repairs: bool,
    pub mutual_recursive_growth_observed: bool,
}

pub fn self_check() -> SelfCheck {
    SelfCheck {
        schema: SUPERVISOR_SCHEMA.to_string(),
        pass: true,
        proposer_cannot_self_approve: true,
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
        promoted_lessons_drive_executable_repairs: false,
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

fn structural_features(text: &str) -> StructuralFeatures {
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
    let features = structural_features(text);
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
        extension: path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase(),
        features,
    }))
}

fn classify_work_kind(path: &Path, features: &StructuralFeatures) -> WorkKind {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if lower.contains("test") || features.test_tokens > 0 {
        WorkKind::RegressionTest
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
    } else if features.error_handling_tokens > 0 && features.validation_tokens > 0 {
        WorkKind::DefectRepair
    } else {
        WorkKind::CodeChange
    }
}

fn event_for_path<'a>(path: &Path, events: &'a [WorkEvent]) -> Option<&'a WorkEvent> {
    let canonical_path = fs::canonicalize(path).ok()?;
    events.iter().rev().find(|event| {
        event.paths.iter().any(|candidate| {
            fs::canonicalize(candidate)
                .map(|canonical| canonical == canonical_path)
                .unwrap_or(false)
        })
    })
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
    let kind = event
        .map(|value| value.kind)
        .unwrap_or_else(|| classify_work_kind(Path::new(&logical_path), &current.features));
    let actor = event
        .map(|value| value.actor)
        .unwrap_or(WorkActor::UnknownLocalWriter);
    let outcome = event
        .map(|value| value.outcome)
        .unwrap_or(WorkOutcome::Unknown);

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
    let observation_id = sha256(
        format!(
            "{}:{}:{}:{}",
            logical_path,
            current.content_sha256,
            previous
                .map(|value| value.content_sha256.as_str())
                .unwrap_or("NEW"),
            event
                .map(|value| value.event_id.as_str())
                .unwrap_or("PASSIVE")
        )
        .as_bytes(),
    );
    LearningObservation {
        observation_id,
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
        exact_source_fragments_stored: 0,
        raw_source_bytes_stored: 0,
        observed_at_ms: now_ms(),
    }
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

fn load_state(config: &GrowthSupervisorConfig) -> Result<SupervisorState, String> {
    let path = latest_numbered_file(&config.state_dir.join("state"), "state_")?
        .ok_or_else(|| "SUPERVISOR_NOT_INITIALIZED".to_string())?;
    let state: SupervisorState = read_json(&path)?;
    if state.schema != SUPERVISOR_SCHEMA || state.config_sha256 != config_hash(config)? {
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
        "control",
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
        runtime_self_repairs_activated: 0,
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
    load_state(&config)
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
        || event
            .evidence_sha256
            .iter()
            .any(|hash| hash.len() != 64 || !hash.chars().all(|value| value.is_ascii_hexdigit()))
    {
        return Err("EVENT_EVIDENCE_HASH_INVALID".to_string());
    }
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

pub fn request_resume(config_path: &Path) -> Result<serde_json::Value, String> {
    let config = load_config(config_path)?;
    let mut state = load_state(&config)?;
    let path = config.state_dir.join("control").join("STOP");
    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("STOP_REMOVE:{error}"))?;
    }
    let resumable_reason = matches!(
        state.stop_reason.as_deref(),
        Some("OPERATOR_STOP_REQUESTED" | "SCAN_RUNTIME_BOUND_REACHED")
    );
    if state.phase == SupervisorPhase::SafeStopped && resumable_reason {
        let recovered_scan_timeout =
            state.stop_reason.as_deref() == Some("SCAN_RUNTIME_BOUND_REACHED");
        state.stop_reason = None;
        save_transition(
            &config,
            &mut state,
            SupervisorPhase::InfraReady,
            if recovered_scan_timeout {
                "OPERATOR_RESUME_AFTER_TRANSIENT_SCAN_TIMEOUT"
            } else {
                "OPERATOR_RESUME_REQUESTED"
            },
        )?;
    }
    Ok(serde_json::json!({
        "resume_requested": true,
        "phase": state.phase,
        "hard_resource_stop_preserved": state.stop_reason.is_some()
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

fn scan_watched_roots(
    config: &GrowthSupervisorConfig,
    memory: &GrowthMemory,
) -> Result<ScanResult, String> {
    let old_index = load_index(config)?;
    let baseline_created = !old_index.baseline_complete;
    let events = load_pending_events(config, &old_index)?;
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
    let mut observations = Vec::new();
    let mut bytes_observed = 0_u64;
    let mut files_reused = 0_usize;
    let mut files_hashed = 0_usize;
    let mut replayed_unchanged_work_events = 0_usize;
    let mut baseline_pending_files = false;
    let mut eligible_logical_paths = BTreeSet::new();
    let canary_bucket = old_index.sequence % FULL_HASH_CANARY_INTERVAL;
    for path in &paths {
        let metadata =
            fs::metadata(path).map_err(|error| format!("METADATA:{}:{error}", path.display()))?;
        let logical = normalized_logical_path(path, &config.watched_roots)?;
        if metadata.len() > config.resources.max_file_bytes {
            new_index.files.remove(&logical);
            continue;
        }
        eligible_logical_paths.insert(logical.clone());
        let previous = old_index.files.get(&logical);
        let matching_event = event_for_path(path, &events);
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
                    observations.push(classify_observation(
                        logical.clone(),
                        indexed,
                        Some(indexed),
                        Some(event),
                        &memory.classifier,
                        config.observation.minimum_learning_score,
                    ));
                    replayed_unchanged_work_events =
                        replayed_unchanged_work_events.saturating_add(1);
                }
            }
            new_index.files.insert(logical, indexed.clone());
            files_reused = files_reused.saturating_add(1);
            continue;
        }
        if baseline_created
            && (files_hashed >= BASELINE_MAX_HASHED_FILES_PER_SCAN
                || bytes_observed.saturating_add(metadata.len())
                    > config
                        .resources
                        .max_bytes_per_scan
                        .min(BASELINE_MAX_BYTES_PER_SCAN))
        {
            baseline_pending_files = true;
            continue;
        }
        if bytes_observed.saturating_add(metadata.len()) > config.resources.max_bytes_per_scan {
            break;
        }
        let Some(fingerprint) =
            fingerprint_file_with_metadata(path, &metadata, config.resources.max_file_bytes)?
        else {
            eligible_logical_paths.remove(&logical);
            new_index.files.remove(&logical);
            continue;
        };
        bytes_observed = bytes_observed.saturating_add(fingerprint.bytes);
        files_hashed = files_hashed.saturating_add(1);
        let content_changed = previous.map(|value| value.content_sha256.as_str())
            != Some(fingerprint.content_sha256.as_str());
        if !baseline_created && (content_changed || matching_event.is_some()) {
            let observation = classify_observation(
                logical.clone(),
                &fingerprint,
                previous,
                matching_event,
                &memory.classifier,
                config.observation.minimum_learning_score,
            );
            observations.push(observation);
            if !content_changed && matching_event.is_some() {
                replayed_unchanged_work_events = replayed_unchanged_work_events.saturating_add(1);
            }
        }
        new_index.files.insert(logical, fingerprint);
    }
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

fn derive_composition_recipe(observations: &[LearningObservation]) -> Vec<String> {
    let roles = observations
        .iter()
        .flat_map(|observation| observation.composition_roles.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut recipe = Vec::new();
    for role in [
        "BACKEND_PROVIDER",
        "INPUT_VALIDATION",
        "IMPLEMENTATION",
        "IMPLEMENTATION_REPAIR",
        "ERROR_PROPAGATION",
        "FRONTEND_CONSUMER",
        "OPERATIONS_GUARD",
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

fn build_lesson(observations: &[LearningObservation]) -> Result<LearnedCompositionLesson, String> {
    let signals = observations
        .iter()
        .flat_map(|observation| observation.signals.iter().cloned())
        .collect::<BTreeSet<_>>();
    let kinds = observations
        .iter()
        .map(|observation| observation.work_kind)
        .collect::<BTreeSet<_>>();
    let evidence = observations
        .iter()
        .map(json_sha256)
        .collect::<Result<Vec<_>, _>>()?;
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
            "{}:{}:{}",
            evidence.join(":"),
            signals.iter().cloned().collect::<Vec<_>>().join(":"),
            recipe.join(":")
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
        learning_score,
        exact_patch_data_present: false,
        exact_source_fragment_present: false,
        raw_source_bytes_present: false,
    })
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
    Ok(EvaluatorMemory {
        schema: current.schema.clone(),
        generation: current.generation.saturating_add(1),
        predecessor_sha256: Some(json_sha256(current)?),
        challenge_suite: challenge_suite.into_iter().collect(),
        source_lesson_ids,
        accepted_expansions: current.accepted_expansions.saturating_add(1),
    })
}

fn lesson_has_verification_evidence(lesson: &LearnedCompositionLesson) -> bool {
    let signals = lesson
        .diagnostic_signals
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let recipe = lesson
        .composition_recipe
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    signals.contains("VERIFIED_PASS")
        || ((signals.contains("TEST_ADDED") || signals.contains("REGRESSION_EVIDENCE"))
            && (recipe.contains("IMPLEMENTATION")
                || recipe.contains("IMPLEMENTATION_REPAIR")
                || recipe.contains("BACKEND_PROVIDER")
                || recipe.contains("FRONTEND_CONSUMER")
                || recipe.contains("OPERATIONS_GUARD")))
}

fn selected_campaign_observations(
    config: &GrowthSupervisorConfig,
    observations: &[LearningObservation],
) -> Vec<LearningObservation> {
    let mut selected = observations
        .iter()
        .take(config.resources.max_observations_per_campaign)
        .cloned()
        .collect::<Vec<_>>();
    if selected.is_empty()
        || build_lesson(&selected)
            .map(|lesson| lesson_has_verification_evidence(&lesson))
            .unwrap_or(false)
    {
        return selected;
    }

    // A score-only prefix can indefinitely hide a slightly lower-scored PASS
    // or regression observation. Try one bounded substitution and retain the
    // first evidence-complete cohort. This changes no score or acceptance rule.
    for evidence in observations.iter().skip(selected.len()) {
        for replace_index in (0..selected.len()).rev() {
            let mut trial = selected.clone();
            trial[replace_index] = evidence.clone();
            if build_lesson(&trial)
                .map(|lesson| lesson_has_verification_evidence(&lesson))
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
    if lesson_has_verification_evidence(&lesson) {
        return Ok(true);
    }
    let observation_ids = chosen
        .iter()
        .map(|observation| observation.observation_id.clone())
        .collect::<Vec<_>>();
    let cohort_sha256 = json_sha256(&observation_ids)?;
    let diagnostic = CampaignPreflightDiagnostic {
        schema: SUPERVISOR_SCHEMA.to_string(),
        cohort_sha256: cohort_sha256.clone(),
        observation_ids,
        reason: "NO_PASS_OR_CODE_TEST_COHORT_EVIDENCE".to_string(),
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
    let campaign_id = format!(
        "G{:020}-{}",
        generation,
        &sha256(observation_sha256.join(":").as_bytes())[..16]
    );
    let proposer = std::env::current_exe().map_err(|error| format!("CURRENT_EXE:{error}"))?;
    let proposer_executable_sha256 = file_sha256(&proposer, 512 * 1024 * 1024)?;
    let verifier_executable_sha256 = file_sha256(&config.verifier_executable, 512 * 1024 * 1024)?;
    if proposer_executable_sha256 == verifier_executable_sha256 {
        return Err("PROPOSER_VERIFIER_BINARY_COLLISION".to_string());
    }
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
    let mut challenge_lessons = predecessor.lessons.clone();
    if !challenge_lessons
        .iter()
        .any(|lesson| lesson.lesson_id == expected_lesson.lesson_id)
    {
        challenge_lessons.push(expected_lesson.clone());
    }
    let mutation_results = challenge_lessons
        .iter()
        .flat_map(|challenge_lesson| {
            proposed_evaluator
                .challenge_suite
                .iter()
                .copied()
                .map(move |mutation| {
                    let challenge_total = if challenge_lesson.lesson_id == expected_lesson.lesson_id
                    {
                        expected_total
                    } else {
                        u32::from(challenge_lesson.learning_score)
                    };
                    let mut mutant = candidate.clone();
                    mutant.lesson = challenge_lesson.clone();
                    mutant.total_learning_score = challenge_total;
                    let expected_generative_cycle = run_generative_cycle(
                        &predecessor.generative,
                        &generative_input(challenge_lesson),
                        seed,
                    );
                    if let Ok(expected_cycle) = &expected_generative_cycle {
                        mutant.generative_cycle = expected_cycle.clone();
                    }
                    match mutation {
                        EvaluatorMutationKind::EvidenceDigestSubstitution => {
                            mutant.lesson.evidence_observation_sha256 = vec!["0".repeat(64)];
                        }
                        EvaluatorMutationKind::AggregateScoreInflation => {
                            mutant.total_learning_score =
                                mutant.total_learning_score.saturating_add(1);
                        }
                        EvaluatorMutationKind::LessonScoreInflation => {
                            mutant.lesson.learning_score =
                                mutant.lesson.learning_score.saturating_add(1);
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
                        challenge_lesson,
                        challenge_total,
                        expected_generative_cycle
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
    let mut memory = load_memory(config, state.generation)?;
    if json_sha256(&memory)? != freeze.predecessor_memory_sha256
        || candidate.predecessor_memory_sha256 != freeze.predecessor_memory_sha256
    {
        return Err("PREDECESSOR_MEMORY_MISMATCH".to_string());
    }
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
    for signal in &candidate.lesson.diagnostic_signals {
        let weight = memory
            .classifier
            .signal_weights
            .entry(signal.clone())
            .or_insert(0);
        *weight = weight.saturating_add(1).min(5);
    }
    if candidate.generative_cycle.applied_to_self_improvement {
        for signal in &candidate.lesson.diagnostic_signals {
            let weight = memory
                .classifier
                .signal_weights
                .entry(signal.clone())
                .or_insert(0);
            *weight = weight.saturating_add(1).min(5);
        }
    }
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
    let next_memory_path = memory_path(config, memory.generation);
    if next_memory_path.exists() {
        let existing: GrowthMemory = read_json(&next_memory_path)?;
        if json_sha256(&existing)? != memory_hash {
            return Err("EXISTING_GENERATION_DIVERGES_FROM_RECOVERY".to_string());
        }
    } else {
        write_immutable_json(&next_memory_path, &memory)?;
    }
    cleanup_memory_generations(config)?;
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
    state.valuable_combinations_learned = memory
        .generative
        .accepted_compositions
        .len()
        .min(u64::MAX as usize) as u64;
    state.generative_memory_reuse_events = memory.generative.reuse_events;
    state.generative_self_application_events = memory.generative.self_application_events;
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
    error_sha256: String,
    predecessor_preserved: bool,
    failed_candidate_deleted: bool,
    occurred_at_ms: u64,
}

fn abort_pending_campaign(
    config: &GrowthSupervisorConfig,
    state: &mut SupervisorState,
    error: &str,
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
    let failure = CampaignFailure {
        schema: SUPERVISOR_SCHEMA.to_string(),
        campaign_id: campaign_id.clone(),
        error_sha256: sha256(error.as_bytes()),
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

fn mutual_bootstrap_observation(
    receipt: &AutonomousSelfInspectionReceipt,
) -> Result<Option<LearningObservation>, String> {
    if receipt.selected_bottleneck != InternalBottleneckClass::MutualRecursiveBootstrapGap
        || receipt.repair_disposition != RepairDisposition::RuntimeRepairActive
        || !receipt.actionable_defect
        || !receipt
            .experiments
            .iter()
            .all(|experiment| experiment.causal_support)
    {
        return Ok(None);
    }
    let receipt_sha256 = json_sha256(receipt)?;
    let observation_id = sha256(
        format!(
            "MUTUAL_RECURSIVE_BOOTSTRAP:{}:{}",
            receipt.generation, receipt_sha256
        )
        .as_bytes(),
    );
    Ok(Some(LearningObservation {
        observation_id,
        work_event_id: None,
        logical_path: "INTERNAL/MUTUAL_CORE_EVALUATOR_BOOTSTRAP".to_string(),
        content_sha256: receipt_sha256.clone(),
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
        verification_evidence_sha256: vec![receipt_sha256],
        exact_source_fragments_stored: 0,
        raw_source_bytes_stored: 0,
        observed_at_ms: now_ms(),
    }))
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
    if is_new {
        write_immutable_json(&path, receipt)?;
        state.self_inspection_events = state.self_inspection_events.saturating_add(1);
        state.diagnostic_experiment_events = state
            .diagnostic_experiment_events
            .saturating_add(receipt.experiments.len() as u64);
        match receipt.repair_disposition {
            RepairDisposition::RuntimeRepairActive => {
                state.runtime_self_repairs_activated =
                    state.runtime_self_repairs_activated.saturating_add(1);
            }
            RepairDisposition::CapabilityGap => {
                state.self_repair_capability_gaps =
                    state.self_repair_capability_gaps.saturating_add(1);
            }
            RepairDisposition::ProposalRequired | RepairDisposition::SafeWait => {}
        }
        cleanup_numbered_files(
            &config.state_dir.join("diagnostics"),
            "self_inspection_",
            64,
        )?;
    }
    state.last_internal_bottleneck = Some(receipt.selected_bottleneck.label().to_string());
    state.last_self_inspection_sha256 = Some(receipt_sha256);
    Ok(())
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
        runtime_self_repairs_activated: state.runtime_self_repairs_activated,
        self_repair_capability_gaps: state.self_repair_capability_gaps,
        last_internal_bottleneck: state.last_internal_bottleneck.clone(),
        evaluator_generation: state.evaluator_generation,
        evaluator_challenge_cases: state.evaluator_challenge_cases,
        mutual_revalidation_events: state.mutual_revalidation_events,
        generative_predictions: state.generative_predictions,
        valuable_combinations_learned: state.valuable_combinations_learned,
        generative_memory_reuse_events: state.generative_memory_reuse_events,
        generative_self_application_events: state.generative_self_application_events,
    }
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
    if stop_if_requested(config, &mut state)? {
        return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
    }
    if state.phase == SupervisorPhase::SafeStopped {
        return Ok(report_from_state(&state, false, 0, 0, 0, None, None));
    }
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

    let mut index = load_index(config)?;
    if state.pending_campaign_id.is_some() {
        let pending_id = state.pending_campaign_id.clone();
        let recovered = match recover_pending_campaign(config, &mut state, &mut index) {
            Ok(value) => value,
            Err(error) => {
                let campaign_id = abort_pending_campaign(config, &mut state, &error)?;
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
    let evaluator_memory_sha256 = json_sha256(&memory.evaluator)?;
    if state.current_evaluator_memory_sha256.is_empty() {
        state.current_evaluator_memory_sha256 = evaluator_memory_sha256.clone();
        state.evaluator_generation = memory.evaluator.generation;
        state.evaluator_challenge_cases = memory.evaluator.challenge_suite.len() as u64;
    } else if state.current_evaluator_memory_sha256 != evaluator_memory_sha256
        || state.evaluator_generation != memory.evaluator.generation
    {
        return Err("CURRENT_EVALUATOR_MEMORY_HASH_MISMATCH".to_string());
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
    save_index(config, &mut scan.index)?;
    let mut high = load_unconsumed_high_observations(config, &scan.index)?;
    let naive = high
        .iter()
        .take(config.resources.max_observations_per_campaign)
        .cloned()
        .collect::<Vec<_>>();
    let evidence_aware = selected_campaign_observations(config, &high);
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
    })?;
    persist_self_inspection(config, &mut state, &inspection)?;
    if config.autonomous_campaigns {
        if let Some(observation) = mutual_bootstrap_observation(&inspection)? {
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
        save_transition(
            config,
            &mut state,
            SupervisorPhase::WaitingPlateau,
            "CAMPAIGN_DEFERRED_WAITING_FOR_PASS_OR_TEST_COHORT",
        )?;
    } else {
        let freeze = freeze_new_campaign(config, &mut state, &high)?;
        campaign_id = Some(freeze.campaign_id.clone());
        match execute_campaign(config, &mut state, &mut scan.index, freeze) {
            Ok(accepted) => campaign_accepted = Some(accepted),
            Err(error) => {
                let _ = abort_pending_campaign(config, &mut state, &error)?;
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
    step_without_lease(&config, &lease)
}

pub fn run_daemon(config_path: &Path) -> Result<StepReport, String> {
    let config = load_config(config_path)?;
    let _ = initialize(config_path)?;
    let lease = SupervisorLease::acquire(&config)?;
    loop {
        lease.heartbeat()?;
        let report = step_without_lease(&config, &lease)?;
        if report.phase == SupervisorPhase::SafeStopped {
            return Ok(report);
        }
        thread::sleep(Duration::from_millis(config.poll_interval_ms));
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
        let config_path = root.join("config.json");
        write_immutable_json(&config_path, &config).unwrap();
        (config_path, config)
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
        assert!(check.proposer_cannot_self_approve);
        assert!(check.raw_source_retention_forbidden);
        assert!(check.network_and_llm_disabled);
        assert!(check.plateau_difficulty_escalation_disabled);
        assert!(check.frozen_observation_reconstruction_enabled);
        assert!(check.bound_pass_evidence_required);
        assert!(check.evaluator_mutation_self_audit_enabled);
        assert!(check.evaluator_generation_evolution_enabled);
        assert!(check.prediction_before_composition_enabled);
        assert!(check.valuable_combination_memory_enabled);
        assert!(check.generative_memory_self_application_enabled);
        assert!(!check.promoted_lessons_drive_executable_repairs);
        assert!(!check.mutual_recursive_growth_observed);
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
        let selected =
            selected_campaign_observations(&config, &[high_without_evidence, verified.clone()]);
        assert_eq!(selected, vec![verified]);
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
    fn autonomous_bootstrap_receipt_forms_a_verified_mutual_growth_cohort() {
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
        })
        .unwrap();
        let observation = mutual_bootstrap_observation(&receipt)
            .unwrap()
            .expect("actionable bootstrap observation");
        let root = temp_root("mutual-bootstrap-cohort");
        let (_, config) = test_config(&root);
        assert!(campaign_preflight_ready(&config, std::slice::from_ref(&observation)).unwrap());
        let lesson = build_lesson(&[observation]).unwrap();
        let next = derive_next_evaluator_memory(&EvaluatorMemory::default(), &[], &lesson).unwrap();
        assert_eq!(next.generation, 1);
        assert_eq!(next.challenge_suite.len(), EvaluatorMutationKind::ALL.len());
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
        assert_eq!(first_state.generative_self_application_events, 1);
        assert_eq!(recovered_state.generative_predictions, 1);
        let promoted = load_memory(&config, 1).unwrap();
        assert_eq!(promoted.generative.accepted_compositions.len(), 1);
        assert_eq!(promoted.generative.self_application_events, 1);
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
}
