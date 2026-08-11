//! Autonomous installation of core-generated source improvements.
//!
//! This module grants the core write authority over one explicitly configured
//! source root. A candidate is bound to the exact predecessor bytes, installed
//! atomically, compiled and regression-tested locally, and rolled back on any
//! failure. Successful builds are staged for the persistent launcher to swap
//! after the running supervisor exits.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::compiler_guided_repair::{discover_compiler_guided_repairs, CompilerGuidedRepairPolicy};
use crate::generalized_self_application::{
    derive_dynamic_weakness, feedback_priority, synthesize_generalized_change,
    validate_change_binding, validation_counterexample, GeneralizedChangeIR,
    ValidationCounterexampleIR, ValidationPhase, WeaknessEvidenceKind,
};
use crate::grammar_repair_synthesis::discover_grammar_repairs_for_generation;
use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{
    execute_structural_repair, synthesize_structural_repair, SourceEditAtom,
    StructuralRepairProgram,
};

pub const AUTONOMOUS_SOURCE_MUTATION_SCHEMA: &str = "B_CORE_AUTONOMOUS_SOURCE_MUTATION_1";
pub const SELF_UPDATE_HANDOFF_FILE: &str = "SELF_UPDATE_READY.json";
pub const SOURCE_REPAIR_LEARNING_SCHEMA: &str = "B_CORE_SOURCE_REPAIR_LEARNING_1";
pub const SOURCE_REPAIR_ENGINE_REVISION: u64 = 4;
const KNOWN_REMAINDER_PREDICTED_VALUE: u16 = 35;
const KNOWN_REMAINDER_STRATEGIES: [&str; 4] = [
    "TYPED_IS_MULTIPLE_OF",
    "PARENTHESIZED_IS_MULTIPLE_OF",
    "CHECKED_REMAINDER_MATCH",
    "EUCLIDEAN_REMAINDER_COMPARISON",
];

fn default_source_repair_attempts() -> u8 {
    4
}

fn is_default_source_repair_attempts(value: &u8) -> bool {
    *value == default_source_repair_attempts()
}

fn default_minimum_predicted_value() -> u16 {
    60
}

fn is_default_minimum_predicted_value(value: &u16) -> bool {
    *value == default_minimum_predicted_value()
}

fn default_compiler_repair_discovery() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourceMutationPolicy {
    pub enabled: bool,
    pub source_root: PathBuf,
    pub cargo_executable: PathBuf,
    pub build_target_dir: PathBuf,
    pub runtime_bin_dir: PathBuf,
    pub auto_discover_known_transformations: bool,
    #[serde(
        default = "default_compiler_repair_discovery",
        skip_serializing_if = "is_true"
    )]
    pub auto_discover_compiler_repairs: bool,
    #[serde(
        default = "default_compiler_repair_discovery",
        skip_serializing_if = "is_true"
    )]
    pub auto_synthesize_grammar_repairs: bool,
    pub max_candidate_bytes: u64,
    pub max_installations: u64,
    pub validation_timeout_ms: u64,
    #[serde(
        default = "default_source_repair_attempts",
        skip_serializing_if = "is_default_source_repair_attempts"
    )]
    pub max_attempts_per_problem: u8,
    #[serde(
        default = "default_minimum_predicted_value",
        skip_serializing_if = "is_default_minimum_predicted_value"
    )]
    pub minimum_predicted_value: u16,
}

impl Default for AutonomousSourceMutationPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            source_root: PathBuf::new(),
            cargo_executable: PathBuf::new(),
            build_target_dir: PathBuf::new(),
            runtime_bin_dir: PathBuf::new(),
            auto_discover_known_transformations: false,
            auto_discover_compiler_repairs: default_compiler_repair_discovery(),
            auto_synthesize_grammar_repairs: default_compiler_repair_discovery(),
            max_candidate_bytes: 2 * 1024 * 1024,
            max_installations: 64,
            validation_timeout_ms: 15 * 60 * 1_000,
            max_attempts_per_problem: default_source_repair_attempts(),
            minimum_predicted_value: default_minimum_predicted_value(),
        }
    }
}

impl AutonomousSourceMutationPolicy {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourcePatchRequest {
    pub schema: String,
    pub patch_id: String,
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub transformation: String,
    pub consequence_predictions: Vec<String>,
    pub predicted_value: u16,
    pub source_generation: u64,
    pub core_generated: bool,
    pub core_self_approved: bool,
    #[serde(default)]
    pub solution_strategy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structural_repair_program: Option<StructuralRepairProgram>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change: Option<GeneralizedChangeIR>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceDiscoveryDisposition {
    Candidate,
    Disabled,
    BelowValueThreshold,
    NoApplicableTransformation,
}

impl SourceDiscoveryDisposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::Candidate => "CANDIDATE",
            Self::Disabled => "DISABLED",
            Self::BelowValueThreshold => "BELOW_VALUE_THRESHOLD",
            Self::NoApplicableTransformation => "NO_APPLICABLE_TRANSFORMATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDiscoveryResult {
    pub disposition: SourceDiscoveryDisposition,
    pub candidate: Option<AutonomousSourcePatchRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepairAttempt {
    pub attempt_number: u8,
    pub source_generation: u64,
    pub solution_strategy: String,
    pub candidate_sha256: String,
    pub succeeded: bool,
    pub receipt_sha256: String,
    pub diagnostic_sha256: String,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub structural_repair_program_sha256: Option<String>,
    #[serde(default)]
    pub edit_atom_kinds: Vec<String>,
    #[serde(default)]
    pub structural_postcondition_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_counterexample: Option<ValidationCounterexampleIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change_sha256: Option<String>,
    #[serde(default)]
    pub derived_from_counterexample_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedSuccessfulRepair {
    pub learned_at_generation: u64,
    pub solution_strategy: String,
    pub candidate_sha256: String,
    pub validation_output_sha256: String,
    pub release_build_output_sha256: String,
    pub attempts_required: u8,
    #[serde(default)]
    pub structural_repair_program_sha256: Option<String>,
    #[serde(default)]
    pub edit_atom_kinds: Vec<String>,
    #[serde(default)]
    pub structural_postcondition_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generalized_change_sha256: Option<String>,
    #[serde(default)]
    pub derived_from_counterexample_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepairLearningRecord {
    pub schema: String,
    pub problem_id: String,
    pub relative_path: PathBuf,
    pub transformation: String,
    pub status: String,
    pub cycle_started_generation: u64,
    #[serde(default)]
    pub cycle_started_engine_revision: u64,
    pub eligible_after_generation: Option<u64>,
    pub attempts: Vec<SourceRepairAttempt>,
    pub learned_success: Option<LearnedSuccessfulRepair>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalCommandReceipt {
    pub program: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    #[serde(default)]
    pub output_sha256: String,
    #[serde(default)]
    pub diagnostic_tail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeUpdateHandoff {
    pub schema: String,
    pub patch_id: String,
    pub staged_supervisor: PathBuf,
    pub staged_verifier: PathBuf,
    pub runtime_supervisor: PathBuf,
    pub runtime_verifier: PathBuf,
    pub source_receipt: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourcePatchReceipt {
    pub schema: String,
    pub patch_id: String,
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub core_generated: bool,
    pub core_self_approved: bool,
    pub installed: bool,
    pub rolled_back: bool,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub format_check: Option<LocalCommandReceipt>,
    #[serde(default)]
    pub compile_check: Option<LocalCommandReceipt>,
    pub validation: LocalCommandReceipt,
    pub release_build: Option<LocalCommandReceipt>,
    pub runtime_update_staged: bool,
    pub rollback_source: PathBuf,
    #[serde(default)]
    pub workspace_fingerprint_before: String,
    #[serde(default)]
    pub workspace_fingerprint_after: String,
    #[serde(default)]
    pub workspace_stable_during_validation: bool,
    pub receipt_sha256: String,
}

pub fn validate_policy(policy: &AutonomousSourceMutationPolicy) -> Result<(), String> {
    if !policy.enabled {
        return Ok(());
    }
    if !policy.source_root.is_absolute() || !policy.source_root.is_dir() {
        return Err("SOURCE_MUTATION_ROOT_INVALID".to_string());
    }
    if !policy.cargo_executable.is_absolute() || !policy.cargo_executable.is_file() {
        return Err("SOURCE_MUTATION_CARGO_INVALID".to_string());
    }
    if !policy.build_target_dir.is_absolute() || !policy.runtime_bin_dir.is_absolute() {
        return Err("SOURCE_MUTATION_BUILD_OR_RUNTIME_ROOT_INVALID".to_string());
    }
    if policy.max_candidate_bytes == 0
        || policy.max_installations == 0
        || policy.validation_timeout_ms < 1_000
        || !(3..=4).contains(&policy.max_attempts_per_problem)
        || policy.minimum_predicted_value > 100
    {
        return Err("SOURCE_MUTATION_BOUND_INVALID".to_string());
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("SOURCE_MUTATION_PARENT_MISSING:{}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("SOURCE_MUTATION_MKDIR:{}:{error}", parent.display()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("SOURCE_MUTATION_CREATE:{}:{error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("SOURCE_MUTATION_WRITE:{}:{error}", path.display()))
}

fn write_immutable_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SOURCE_MUTATION_JSON:{error}"))?;
    write_new_file(path, &bytes)
}

fn write_mutable_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_JSON:{error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "SOURCE_REPAIR_LEARNING_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("SOURCE_REPAIR_LEARNING_MKDIR:{error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_OPEN:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_WRITE:{error}"))
}

fn repair_problem_id(request: &AutonomousSourcePatchRequest) -> String {
    sha256(
        format!(
            "{}:{}",
            request.relative_path.display(),
            request.transformation
        )
        .as_bytes(),
    )
}

fn repair_learning_path(state_dir: &Path, problem_id: &str) -> PathBuf {
    state_dir
        .join("source_repair_knowledge")
        .join(format!("{problem_id}.json"))
}

fn load_repair_learning(
    state_dir: &Path,
    problem_id: &str,
) -> Result<Option<SourceRepairLearningRecord>, String> {
    let path = repair_learning_path(state_dir, problem_id);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|error| format!("SOURCE_REPAIR_LEARNING_READ:{error}"))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("SOURCE_REPAIR_LEARNING_PARSE:{error}"))
}

fn active_cycle_attempts(
    record: &SourceRepairLearningRecord,
    source_generation: u64,
) -> &[SourceRepairAttempt] {
    if (record.status == "ADMITTED_FAILURE"
        && record
            .eligible_after_generation
            .is_some_and(|eligible| source_generation >= eligible))
        || (record.status != "LEARNED_SUCCESS"
            && record.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION)
    {
        &[]
    } else {
        &record.attempts
    }
}

fn collect_edit_atom_kinds(edit: &SourceEditAtom, kinds: &mut Vec<String>) {
    let kind = match edit {
        SourceEditAtom::Replace { .. } => "REPLACE",
        SourceEditAtom::Insert { .. } => "INSERT",
        SourceEditAtom::Delete { .. } => "DELETE",
        SourceEditAtom::Move { .. } => "MOVE",
        SourceEditAtom::AtomicMultiEdit { edits } => {
            kinds.push("ATOMIC_MULTI_EDIT".to_string());
            for nested in edits {
                collect_edit_atom_kinds(nested, kinds);
            }
            return;
        }
    };
    kinds.push(kind.to_string());
}

fn structural_program_learning_features(
    request: &AutonomousSourcePatchRequest,
) -> Result<(Option<String>, Vec<String>, usize), String> {
    let Some(program) = &request.structural_repair_program else {
        return Ok((None, Vec::new(), 0));
    };
    let encoded = serde_json::to_vec(program)
        .map_err(|error| format!("STRUCTURAL_REPAIR_PROGRAM_SERIALIZE:{error}"))?;
    let mut edit_atom_kinds = Vec::new();
    collect_edit_atom_kinds(&program.edit, &mut edit_atom_kinds);
    Ok((
        Some(sha256(&encoded)),
        edit_atom_kinds,
        program.postconditions.len(),
    ))
}

fn generalized_change_learning_features(
    request: &AutonomousSourcePatchRequest,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(change) = &request.generalized_change else {
        return Ok((None, Vec::new()));
    };
    let encoded = serde_json::to_vec(change)
        .map_err(|error| format!("GENERALIZED_CHANGE_SERIALIZE:{error}"))?;
    Ok((
        Some(sha256(&encoded)),
        change.derived_from_counterexample_ids.clone(),
    ))
}

fn counterexample_from_receipt(
    request: &AutonomousSourcePatchRequest,
    receipt: &AutonomousSourcePatchReceipt,
) -> Option<ValidationCounterexampleIR> {
    if receipt.installed {
        return None;
    }
    let reason = receipt
        .failure_reason
        .as_deref()
        .unwrap_or("UNKNOWN_FAILURE");
    let (phase, command) = if reason == "FORMAT_CHECK_FAILED" {
        (ValidationPhase::Format, receipt.format_check.as_ref())
    } else if reason == "COMPILE_CHECK_FAILED" {
        (ValidationPhase::Compile, receipt.compile_check.as_ref())
    } else if reason == "REGRESSION_VALIDATION_FAILED" {
        (
            ValidationPhase::PublicObservation,
            Some(&receipt.validation),
        )
    } else if reason == "RELEASE_BUILD_FAILED" {
        (
            ValidationPhase::ReleaseBuild,
            receipt.release_build.as_ref(),
        )
    } else if reason.contains("WORKSPACE") || reason.contains("TARGET_CHANGED") {
        (
            ValidationPhase::WorkspaceIntegrity,
            Some(&receipt.validation),
        )
    } else {
        (ValidationPhase::Infrastructure, Some(&receipt.validation))
    };
    let diagnostic_sha256 = command
        .map(|value| value.output_sha256.as_str())
        .unwrap_or("");
    let diagnostic_tail = command
        .map(|value| value.diagnostic_tail.as_str())
        .unwrap_or("");
    Some(validation_counterexample(
        request.source_generation,
        phase,
        reason,
        diagnostic_sha256,
        diagnostic_tail,
        if request.solution_strategy.is_empty() {
            &request.transformation
        } else {
            &request.solution_strategy
        },
        &request.candidate_sha256,
    ))
}

fn prior_counterexamples(
    state_dir: &Path,
    relative_path: &Path,
    transformation: &str,
) -> Result<Vec<ValidationCounterexampleIR>, String> {
    let problem_id = repair_problem_id_for(relative_path, transformation);
    Ok(load_repair_learning(state_dir, &problem_id)?
        .map(|record| {
            record
                .attempts
                .into_iter()
                .filter_map(|attempt| attempt.validation_counterexample)
                .collect()
        })
        .unwrap_or_default())
}

#[allow(clippy::too_many_arguments)]
fn generalized_change_for_candidate(
    state_dir: &Path,
    source_generation: u64,
    relative_path: &Path,
    transformation: &str,
    solution_strategy: &str,
    predecessor_sha256: &str,
    candidate_sha256: &str,
    evidence_kind: WeaknessEvidenceKind,
    evidence_sha256: &str,
    observed_mechanism: &str,
    consequence_predictions: &[String],
    program: &StructuralRepairProgram,
) -> Result<GeneralizedChangeIR, String> {
    let prior = prior_counterexamples(state_dir, relative_path, transformation)?;
    let weakness = derive_dynamic_weakness(
        source_generation,
        relative_path,
        transformation,
        evidence_kind,
        evidence_sha256,
        observed_mechanism,
        consequence_predictions.to_vec(),
        prior
            .iter()
            .map(|counterexample| counterexample.counterexample_id.clone())
            .collect(),
    );
    synthesize_generalized_change(
        &weakness,
        solution_strategy,
        predecessor_sha256,
        candidate_sha256,
        program,
    )
}

fn record_source_repair_outcome(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
    receipt: &AutonomousSourcePatchReceipt,
) -> Result<SourceRepairLearningRecord, String> {
    let problem_id = repair_problem_id(request);
    let mut record = load_repair_learning(state_dir, &problem_id)?.unwrap_or_else(|| {
        SourceRepairLearningRecord {
            schema: SOURCE_REPAIR_LEARNING_SCHEMA.to_string(),
            problem_id: problem_id.clone(),
            relative_path: request.relative_path.clone(),
            transformation: request.transformation.clone(),
            status: "RETRYING".to_string(),
            cycle_started_generation: request.source_generation,
            cycle_started_engine_revision: SOURCE_REPAIR_ENGINE_REVISION,
            eligible_after_generation: None,
            attempts: Vec::new(),
            learned_success: None,
        }
    });
    if record.status != "LEARNED_SUCCESS"
        && ((record.status == "ADMITTED_FAILURE"
            && record
                .eligible_after_generation
                .is_some_and(|eligible| request.source_generation >= eligible))
            || record.cycle_started_engine_revision < SOURCE_REPAIR_ENGINE_REVISION)
    {
        record.status = "RETRYING".to_string();
        record.cycle_started_generation = request.source_generation;
        record.cycle_started_engine_revision = SOURCE_REPAIR_ENGINE_REVISION;
        record.eligible_after_generation = None;
        record.attempts.clear();
    }
    let attempt_number = record
        .attempts
        .len()
        .saturating_add(1)
        .min(u8::MAX as usize) as u8;
    let solution_strategy = if request.solution_strategy.is_empty() {
        request.transformation.clone()
    } else {
        request.solution_strategy.clone()
    };
    let (structural_repair_program_sha256, edit_atom_kinds, structural_postcondition_count) =
        structural_program_learning_features(request)?;
    let (generalized_change_sha256, derived_from_counterexample_ids) =
        generalized_change_learning_features(request)?;
    let validation_counterexample = counterexample_from_receipt(request, receipt);
    record.attempts.push(SourceRepairAttempt {
        attempt_number,
        source_generation: request.source_generation,
        solution_strategy: solution_strategy.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        succeeded: receipt.installed,
        receipt_sha256: receipt.receipt_sha256.clone(),
        diagnostic_sha256: receipt.validation.output_sha256.clone(),
        failure_reason: receipt.failure_reason.clone(),
        structural_repair_program_sha256: structural_repair_program_sha256.clone(),
        edit_atom_kinds: edit_atom_kinds.clone(),
        structural_postcondition_count,
        validation_counterexample,
        generalized_change_sha256: generalized_change_sha256.clone(),
        derived_from_counterexample_ids: derived_from_counterexample_ids.clone(),
    });
    if receipt.installed {
        record.status = "LEARNED_SUCCESS".to_string();
        record.eligible_after_generation = None;
        record.learned_success = Some(LearnedSuccessfulRepair {
            learned_at_generation: request.source_generation,
            solution_strategy,
            candidate_sha256: request.candidate_sha256.clone(),
            validation_output_sha256: receipt.validation.output_sha256.clone(),
            release_build_output_sha256: receipt
                .release_build
                .as_ref()
                .map(|build| build.output_sha256.clone())
                .unwrap_or_default(),
            attempts_required: attempt_number,
            structural_repair_program_sha256,
            edit_atom_kinds,
            structural_postcondition_count,
            generalized_change_sha256,
            derived_from_counterexample_ids,
        });
    } else if attempt_number >= policy.max_attempts_per_problem {
        record.status = "ADMITTED_FAILURE".to_string();
        record.eligible_after_generation = Some(request.source_generation.saturating_add(1));
        record.learned_success = None;
    } else {
        record.status = "RETRYING".to_string();
    }
    write_mutable_json(&repair_learning_path(state_dir, &problem_id), &record)?;
    Ok(record)
}

fn normalized_target(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.is_absolute()
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
        return Err("SOURCE_MUTATION_RELATIVE_PATH_INVALID".to_string());
    }
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("SOURCE_MUTATION_ROOT_CANONICALIZE:{error}"))?;
    let target = root.join(relative);
    let canonical_target = fs::canonicalize(&target)
        .map_err(|error| format!("SOURCE_MUTATION_TARGET_CANONICALIZE:{error}"))?;
    if !canonical_target.starts_with(&canonical_root) || !canonical_target.is_file() {
        return Err("SOURCE_MUTATION_TARGET_OUTSIDE_ROOT".to_string());
    }
    if fs::symlink_metadata(&canonical_target)
        .map_err(|error| format!("SOURCE_MUTATION_TARGET_METADATA:{error}"))?
        .file_type()
        .is_symlink()
    {
        return Err("SOURCE_MUTATION_SYMLINK_FORBIDDEN".to_string());
    }
    Ok(canonical_target)
}

fn structural_file_id(relative: &Path) -> String {
    relative.to_string_lossy().replace('\\', "/")
}

fn workspace_semantic_fingerprint(root: &Path, excluded_target: &Path) -> Result<String, String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_ROOT:{error}"))?;
    let canonical_target = fs::canonicalize(excluded_target)
        .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_TARGET:{error}"))?;
    let mut pending = vec![canonical_root.clone()];
    let mut entries = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)
            .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_READ_DIR:{error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_ENTRY:{error}"))?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let file_type = child
                .file_type()
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_TYPE:{error}"))?;
            let path = child.path();
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if !excluded_directory(&path) {
                    pending.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&path)
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_CANONICAL:{error}"))?;
            let file_name = canonical.file_name().and_then(OsStr::to_str).unwrap_or("");
            if canonical == canonical_target
                || file_name.contains(".bcore-rollback")
                || file_name.contains(".bcore-candidate")
            {
                continue;
            }
            let relative = canonical
                .strip_prefix(&canonical_root)
                .map_err(|_| "SOURCE_MUTATION_FINGERPRINT_OUTSIDE_ROOT".to_string())?;
            let bytes = fs::read(&canonical)
                .map_err(|error| format!("SOURCE_MUTATION_FINGERPRINT_READ:{error}"))?;
            entries.push(format!(
                "{}:{}:{}",
                relative.display(),
                bytes.len(),
                sha256(&bytes)
            ));
        }
    }
    entries.sort();
    Ok(sha256(entries.join("\n").as_bytes()))
}

fn command_receipt(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    target_dir: &Path,
    timeout_ms: u64,
    diagnostic_path: &Path,
) -> Result<LocalCommandReceipt, String> {
    let started = Instant::now();
    let diagnostic = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(diagnostic_path)
        .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_CREATE:{error}"))?;
    let diagnostic_error = diagnostic
        .try_clone()
        .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_CLONE:{error}"))?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_NET_OFFLINE", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::from(diagnostic))
        .stderr(Stdio::from(diagnostic_error))
        .spawn()
        .map_err(|error| format!("SOURCE_MUTATION_COMMAND_SPAWN:{error}"))?;
    let timeout = Duration::from_millis(timeout_ms);
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("SOURCE_MUTATION_COMMAND_WAIT:{error}"))?
        {
            let output = fs::read(diagnostic_path)
                .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_READ:{error}"))?;
            let tail_start = output.len().saturating_sub(4_096);
            return Ok(LocalCommandReceipt {
                program: program.display().to_string(),
                args: args.iter().map(|value| (*value).to_string()).collect(),
                exit_code: status.code(),
                success: status.success(),
                timed_out: false,
                duration_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                output_sha256: sha256(&output),
                diagnostic_tail: String::from_utf8_lossy(&output[tail_start..]).to_string(),
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let status = child.wait().ok();
            let output = fs::read(diagnostic_path)
                .map_err(|error| format!("SOURCE_MUTATION_DIAGNOSTIC_READ:{error}"))?;
            let tail_start = output.len().saturating_sub(4_096);
            return Ok(LocalCommandReceipt {
                program: program.display().to_string(),
                args: args.iter().map(|value| (*value).to_string()).collect(),
                exit_code: status.and_then(|value| value.code()),
                success: false,
                timed_out: true,
                duration_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                output_sha256: sha256(&output),
                diagnostic_tail: String::from_utf8_lossy(&output[tail_start..]).to_string(),
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn restore_target(target: &Path, rollback_sibling: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_file(target)
            .map_err(|error| format!("SOURCE_MUTATION_REMOVE_FAILED_TARGET:{error}"))?;
    }
    fs::rename(rollback_sibling, target)
        .map_err(|error| format!("SOURCE_MUTATION_ROLLBACK_RENAME:{error}"))
}

fn receipt_hash(receipt: &AutonomousSourcePatchReceipt) -> Result<String, String> {
    let mut clone = receipt.clone();
    clone.receipt_sha256.clear();
    serde_json::to_vec(&clone)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| format!("SOURCE_MUTATION_RECEIPT_JSON:{error}"))
}

pub fn install_and_stage_source_patch(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    request: &AutonomousSourcePatchRequest,
) -> Result<AutonomousSourcePatchReceipt, String> {
    validate_policy(policy)?;
    if !policy.enabled
        || request.schema != AUTONOMOUS_SOURCE_MUTATION_SCHEMA
        || !request.core_generated
        || !request.core_self_approved
        || request.patch_id.is_empty()
        || request.predicted_value < policy.minimum_predicted_value
        || request.predicted_value > 100
        || request.candidate_source.len() as u64 > policy.max_candidate_bytes
        || sha256(request.candidate_source.as_bytes()) != request.candidate_sha256
    {
        return Err("SOURCE_MUTATION_REQUEST_INVALID".to_string());
    }
    let handoff_path = state_dir.join("control").join(SELF_UPDATE_HANDOFF_FILE);
    if handoff_path.exists() {
        return Err("SOURCE_UPDATE_ALREADY_STAGED".to_string());
    }
    let target = normalized_target(&policy.source_root, &request.relative_path)?;
    let predecessor =
        fs::read(&target).map_err(|error| format!("SOURCE_MUTATION_PREDECESSOR_READ:{error}"))?;
    if sha256(&predecessor) != request.predecessor_sha256 {
        return Err("SOURCE_MUTATION_PREDECESSOR_MISMATCH".to_string());
    }
    if let Some(program) = &request.structural_repair_program {
        let predecessor_source = std::str::from_utf8(&predecessor)
            .map_err(|_| "STRUCTURAL_REPAIR_PREDECESSOR_NOT_UTF8".to_string())?;
        if program.file_id != structural_file_id(&request.relative_path) {
            return Err("STRUCTURAL_REPAIR_FILE_ID_MISMATCH".to_string());
        }
        let execution = execute_structural_repair(program, predecessor_source)
            .map_err(|error| format!("STRUCTURAL_REPAIR_REPLAY:{error}"))?;
        if !execution.structurally_verified
            || execution.candidate_source != request.candidate_source
            || execution.candidate_snapshot.source_sha256 != request.candidate_sha256
        {
            return Err("STRUCTURAL_REPAIR_REPLAY_MISMATCH".to_string());
        }
    }
    if let Some(change) = &request.generalized_change {
        let program = request
            .structural_repair_program
            .as_ref()
            .ok_or_else(|| "GENERALIZED_CHANGE_STRUCTURAL_PROGRAM_MISSING".to_string())?;
        validate_change_binding(
            change,
            &request.relative_path,
            &request.transformation,
            if request.solution_strategy.is_empty() {
                &request.transformation
            } else {
                &request.solution_strategy
            },
            &request.predecessor_sha256,
            &request.candidate_sha256,
            program,
        )?;
    }
    let workspace_fingerprint_before =
        workspace_semantic_fingerprint(&policy.source_root, &target)?;

    let mutation_root = state_dir.join("source_mutations").join(&request.patch_id);
    fs::create_dir_all(&mutation_root)
        .map_err(|error| format!("SOURCE_MUTATION_RECEIPT_DIR:{error}"))?;
    let request_path = mutation_root.join("request.json");
    if !request_path.exists() {
        write_immutable_json(&request_path, request)?;
    }
    let rollback_source = mutation_root.join("predecessor.source");
    if !rollback_source.exists() {
        write_new_file(&rollback_source, &predecessor)?;
    }
    let rollback_sibling = target.with_file_name(format!(
        ".{}.{}.bcore-rollback",
        target
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("source"),
        request.patch_id
    ));
    let candidate_sibling = target.with_file_name(format!(
        ".{}.{}.bcore-candidate",
        target
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("source"),
        request.patch_id
    ));
    if rollback_sibling.exists() || candidate_sibling.exists() {
        return Err("SOURCE_MUTATION_SIBLING_COLLISION".to_string());
    }
    write_new_file(&candidate_sibling, request.candidate_source.as_bytes())?;
    fs::rename(&target, &rollback_sibling)
        .map_err(|error| format!("SOURCE_MUTATION_PREDECESSOR_RENAME:{error}"))?;
    if let Err(error) = fs::rename(&candidate_sibling, &target) {
        let _ = fs::rename(&rollback_sibling, &target);
        return Err(format!("SOURCE_MUTATION_CANDIDATE_RENAME:{error}"));
    }

    let format_check = match command_receipt(
        &policy.cargo_executable,
        &["fmt", "-p", "semantic-reasoning", "--", "--check"],
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("format-check.log"),
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !format_check.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed: false,
            rolled_back: true,
            failure_reason: Some("FORMAT_CHECK_FAILED".to_string()),
            format_check: Some(format_check.clone()),
            compile_check: None,
            validation: format_check,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before: workspace_fingerprint_before.clone(),
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let compile_check = match command_receipt(
        &policy.cargo_executable,
        &["check", "-p", "semantic-reasoning", "--lib", "--quiet"],
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("compile-check.log"),
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !compile_check.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed: false,
            rolled_back: true,
            failure_reason: Some("COMPILE_CHECK_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check.clone()),
            validation: compile_check,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before: workspace_fingerprint_before.clone(),
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let validation = match command_receipt(
        &policy.cargo_executable,
        &["test", "-p", "semantic-reasoning", "--lib", "--quiet"],
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("test.log"),
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !validation.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed: false,
            rolled_back: true,
            failure_reason: Some("REGRESSION_VALIDATION_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before: workspace_fingerprint_before.clone(),
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let release_build = match command_receipt(
        &policy.cargo_executable,
        &[
            "build",
            "-p",
            "semantic-reasoning",
            "--release",
            "--bin",
            "b-core-growth-supervisor",
            "--bin",
            "b-core-growth-verifier",
        ],
        &policy.source_root,
        &policy.build_target_dir,
        policy.validation_timeout_ms,
        &mutation_root.join("release-build.log"),
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            restore_target(&target, &rollback_sibling)?;
            return Err(error);
        }
    };
    if !release_build.success {
        restore_target(&target, &rollback_sibling)?;
        let workspace_fingerprint_after =
            workspace_semantic_fingerprint(&policy.source_root, &target)?;
        let workspace_stable_during_validation =
            workspace_fingerprint_before == workspace_fingerprint_after;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed: false,
            rolled_back: true,
            failure_reason: Some("RELEASE_BUILD_FAILED".to_string()),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build: Some(release_build),
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before: workspace_fingerprint_before.clone(),
            workspace_fingerprint_after,
            workspace_stable_during_validation,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let workspace_fingerprint_after = workspace_semantic_fingerprint(&policy.source_root, &target)?;
    let target_still_exact_candidate = fs::read(&target)
        .map(|bytes| sha256(&bytes) == request.candidate_sha256)
        .unwrap_or(false);
    let workspace_stable_during_validation =
        workspace_fingerprint_before == workspace_fingerprint_after;
    if !workspace_stable_during_validation || !target_still_exact_candidate {
        restore_target(&target, &rollback_sibling)?;
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed: false,
            rolled_back: true,
            failure_reason: Some(if target_still_exact_candidate {
                "CONCURRENT_WORKSPACE_CHANGE_DURING_VALIDATION".to_string()
            } else {
                "TARGET_CHANGED_DURING_VALIDATION".to_string()
            }),
            format_check: Some(format_check),
            compile_check: Some(compile_check),
            validation,
            release_build: Some(release_build),
            runtime_update_staged: false,
            rollback_source,
            workspace_fingerprint_before,
            workspace_fingerprint_after,
            workspace_stable_during_validation: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt)?;
        write_immutable_json(&mutation_root.join("receipt.json"), &receipt)?;
        record_source_repair_outcome(policy, state_dir, request, &receipt)?;
        return Ok(receipt);
    }

    let built_supervisor = policy
        .build_target_dir
        .join("release")
        .join("b-core-growth-supervisor.exe");
    let built_verifier = policy
        .build_target_dir
        .join("release")
        .join("b-core-growth-verifier.exe");
    if !built_supervisor.is_file() || !built_verifier.is_file() {
        restore_target(&target, &rollback_sibling)?;
        return Err("SOURCE_MUTATION_RELEASE_ARTIFACT_MISSING".to_string());
    }
    let staging = mutation_root.join("staging");
    fs::create_dir_all(&staging).map_err(|error| format!("SOURCE_MUTATION_STAGING_DIR:{error}"))?;
    let staged_supervisor = staging.join("b-core-growth-supervisor.exe");
    let staged_verifier = staging.join("b-core-growth-verifier.exe");
    fs::copy(&built_supervisor, &staged_supervisor)
        .map_err(|error| format!("SOURCE_MUTATION_STAGE_SUPERVISOR:{error}"))?;
    fs::copy(&built_verifier, &staged_verifier)
        .map_err(|error| format!("SOURCE_MUTATION_STAGE_VERIFIER:{error}"))?;

    let receipt_path = mutation_root.join("receipt.json");
    let mut receipt = AutonomousSourcePatchReceipt {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id: request.patch_id.clone(),
        relative_path: request.relative_path.clone(),
        predecessor_sha256: request.predecessor_sha256.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        core_generated: true,
        core_self_approved: true,
        installed: true,
        rolled_back: false,
        failure_reason: None,
        format_check: Some(format_check),
        compile_check: Some(compile_check),
        validation,
        release_build: Some(release_build),
        runtime_update_staged: true,
        rollback_source,
        workspace_fingerprint_before,
        workspace_fingerprint_after,
        workspace_stable_during_validation,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_hash(&receipt)?;
    write_immutable_json(&receipt_path, &receipt)?;
    record_source_repair_outcome(policy, state_dir, request, &receipt)?;
    fs::remove_file(&rollback_sibling)
        .map_err(|error| format!("SOURCE_MUTATION_ROLLBACK_SIBLING_CLEANUP:{error}"))?;

    fs::create_dir_all(&policy.runtime_bin_dir)
        .map_err(|error| format!("SOURCE_MUTATION_RUNTIME_DIR:{error}"))?;
    let handoff = RuntimeUpdateHandoff {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id: request.patch_id.clone(),
        staged_supervisor,
        staged_verifier,
        runtime_supervisor: policy.runtime_bin_dir.join("b-core-growth-supervisor.exe"),
        runtime_verifier: policy.runtime_bin_dir.join("b-core-growth-verifier.exe"),
        source_receipt: receipt_path,
    };
    write_immutable_json(&handoff_path, &handoff)?;
    Ok(receipt)
}

fn excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            [".git", "target", "vendor", "reports", "artifacts"]
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
        })
}

fn rust_source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("SOURCE_DISCOVERY_READ_DIR:{}:{error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("SOURCE_DISCOVERY_ENTRY:{error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("SOURCE_DISCOVERY_FILE_TYPE:{error}"))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() && !excluded_directory(&path) {
                pending.push(path);
            } else if file_type.is_file() && path.extension().and_then(OsStr::to_str) == Some("rs")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn expression_start(prefix: &str) -> Option<usize> {
    let bytes = prefix.as_bytes();
    let mut index = bytes.len();
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    let end = index;
    let mut depth = 0_i32;
    while index > 0 {
        index -= 1;
        match bytes[index] {
            b')' => depth += 1,
            b'(' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let previous = index.checked_sub(1).map(|value| bytes[value]);
                    if !previous.is_some_and(|value| {
                        value.is_ascii_alphanumeric() || matches!(value, b'_' | b'.')
                    }) {
                        return Some(index);
                    }
                }
            }
            b'(' if depth == 0 => return Some(index + 1),
            b'=' if depth == 0 => {
                return Some(if bytes.get(index + 1) == Some(&b'>') {
                    index + 2
                } else {
                    index + 1
                })
            }
            b';' | b',' | b'{' | b'[' | b'!' | b'&' | b'|' if depth == 0 => return Some(index + 1),
            _ => {}
        }
    }
    (end > 0).then_some(0)
}

fn rewrite_remainder_predicate(line: &str, strategy_index: usize) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with("#")
        || line.contains('"')
        || line.contains("assert")
    {
        return None;
    }
    let modulo = line.find(" % ")?;
    let right_start = modulo + 3;
    let tail = &line[right_start..];
    let (divisor, negated, comparison_len) = if let Some(position) = tail.find(" == 0") {
        (&tail[..position], false, position + 5)
    } else if let Some(position) = tail.find(" != 0") {
        (&tail[..position], true, position + 5)
    } else {
        return None;
    };
    if divisor.is_empty()
        || divisor.chars().any(char::is_whitespace)
        || !divisor
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    let left_boundary = expression_start(&line[..modulo])?;
    let leading_whitespace = line[left_boundary..modulo]
        .bytes()
        .take_while(u8::is_ascii_whitespace)
        .count();
    let left_start = left_boundary + leading_whitespace;
    let expression = line[left_start..modulo].trim();
    if expression.is_empty() {
        return None;
    }
    let positive = match strategy_index {
        0 => format!("{expression}.is_multiple_of({divisor})"),
        1 => format!("({expression}).is_multiple_of({divisor})"),
        2 => format!("matches!(({expression}).checked_rem({divisor}), Some(0))"),
        3 => format!("({expression}).rem_euclid({divisor}) == 0"),
        _ => return None,
    };
    let replacement = if negated {
        format!("!({positive})")
    } else {
        positive
    };
    let mut result = String::with_capacity(line.len() + 16);
    result.push_str(&line[..left_start]);
    result.push_str(&replacement);
    result.push_str(&line[right_start + comparison_len..]);
    Some(result)
}

fn rewrite_first_known_improvement(source: &str, strategy_index: usize) -> Option<String> {
    let mut output = String::with_capacity(source.len() + 32);
    let mut changed = false;
    let mut test_module_reached = false;
    for line in source.split_inclusive('\n') {
        if line.trim() == "#[cfg(test)]" {
            test_module_reached = true;
        }
        if !changed && !test_module_reached {
            if let Some(rewritten) = rewrite_remainder_predicate(line, strategy_index) {
                output.push_str(&rewritten);
                changed = true;
                continue;
            }
        }
        output.push_str(line);
    }
    changed.then_some(output)
}

fn repair_problem_id_for(relative_path: &Path, transformation: &str) -> String {
    sha256(format!("{}:{transformation}", relative_path.display()).as_bytes())
}

fn repair_strategy_is_available(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    relative_path: &Path,
    transformation: &str,
    solution_strategy: &str,
    source_generation: u64,
) -> Result<bool, String> {
    let problem_id = repair_problem_id_for(relative_path, transformation);
    let record = load_repair_learning(state_dir, &problem_id)?;
    if record.as_ref().is_some_and(|knowledge| {
        knowledge.status == "LEARNED_SUCCESS"
            || (knowledge.status == "ADMITTED_FAILURE"
                && knowledge
                    .eligible_after_generation
                    .is_some_and(|eligible| source_generation < eligible)
                && knowledge.cycle_started_engine_revision >= SOURCE_REPAIR_ENGINE_REVISION)
    }) {
        return Ok(false);
    }
    let attempted = record
        .as_ref()
        .map(|knowledge| active_cycle_attempts(knowledge, source_generation))
        .unwrap_or(&[]);
    Ok(
        attempted.len() < usize::from(policy.max_attempts_per_problem)
            && !attempted
                .iter()
                .any(|attempt| attempt.solution_strategy == solution_strategy),
    )
}

fn compiler_guided_request(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    if !policy.auto_discover_compiler_repairs {
        return Ok(None);
    }
    let diagnostic_policy = CompilerGuidedRepairPolicy {
        source_root: &policy.source_root,
        cargo_executable: &policy.cargo_executable,
        build_target_dir: &policy.build_target_dir,
        state_dir,
        timeout_ms: policy.validation_timeout_ms,
        max_candidate_bytes: policy.max_candidate_bytes,
    };
    for candidate in discover_compiler_guided_repairs(&diagnostic_policy)? {
        if candidate.predicted_value < policy.minimum_predicted_value
            || !repair_strategy_is_available(
                policy,
                state_dir,
                &candidate.relative_path,
                &candidate.transformation,
                &candidate.solution_strategy,
                source_generation,
            )?
        {
            continue;
        }
        let problem_id = repair_problem_id_for(&candidate.relative_path, &candidate.transformation);
        let patch_id = format!(
            "SELF-{}",
            &sha256(
                format!(
                    "{}:{}:{}:{}",
                    problem_id,
                    source_generation,
                    candidate.solution_strategy,
                    candidate.candidate_sha256
                )
                .as_bytes()
            )[..24]
        );
        if state_dir
            .join("source_mutations")
            .join(&patch_id)
            .join("receipt.json")
            .exists()
        {
            continue;
        }
        let generalized_change = generalized_change_for_candidate(
            state_dir,
            source_generation,
            &candidate.relative_path,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.predecessor_sha256,
            &candidate.candidate_sha256,
            WeaknessEvidenceKind::CompilerDiagnostic,
            &candidate.public_observation_sha256,
            "current compiler or clippy observation exposes a source-level weakness",
            &candidate.consequence_predictions,
            &candidate.structural_repair_program,
        )?;
        return Ok(Some(AutonomousSourcePatchRequest {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id,
            relative_path: candidate.relative_path,
            predecessor_sha256: candidate.predecessor_sha256,
            candidate_source: candidate.candidate_source,
            candidate_sha256: candidate.candidate_sha256,
            transformation: candidate.transformation,
            consequence_predictions: candidate.consequence_predictions,
            predicted_value: candidate.predicted_value,
            source_generation,
            core_generated: true,
            core_self_approved: true,
            solution_strategy: candidate.solution_strategy,
            structural_repair_program: Some(candidate.structural_repair_program),
            generalized_change: Some(generalized_change),
        }));
    }
    Ok(None)
}

fn grammar_synthesized_request(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    if !policy.auto_synthesize_grammar_repairs {
        return Ok(None);
    }
    let mut ranked = Vec::new();
    for candidate in discover_grammar_repairs_for_generation(
        &policy.source_root,
        policy.max_candidate_bytes,
        source_generation,
    )? {
        let counterexamples = prior_counterexamples(
            state_dir,
            &candidate.relative_path,
            &candidate.transformation,
        )?;
        let priority = i32::from(candidate.predicted_value)
            + feedback_priority(&candidate.solution_strategy, &counterexamples);
        ranked.push((priority, candidate));
    }
    ranked.sort_by_key(|(priority, candidate)| {
        (
            std::cmp::Reverse(*priority),
            candidate.relative_path.clone(),
            candidate.transformation.clone(),
            candidate.solution_strategy.clone(),
        )
    });
    for (_, candidate) in ranked {
        if candidate.predicted_value < policy.minimum_predicted_value
            || !repair_strategy_is_available(
                policy,
                state_dir,
                &candidate.relative_path,
                &candidate.transformation,
                &candidate.solution_strategy,
                source_generation,
            )?
        {
            continue;
        }
        let problem_id = repair_problem_id_for(&candidate.relative_path, &candidate.transformation);
        let patch_id = format!(
            "SELF-{}",
            &sha256(
                format!(
                    "{}:{}:{}:{}",
                    problem_id,
                    source_generation,
                    candidate.solution_strategy,
                    candidate.candidate_sha256
                )
                .as_bytes()
            )[..24]
        );
        if state_dir
            .join("source_mutations")
            .join(&patch_id)
            .join("receipt.json")
            .exists()
        {
            continue;
        }
        let evidence_sha256 = sha256(
            format!(
                "{}:{}:{}",
                candidate.relative_path.display(),
                candidate.transformation,
                candidate.predecessor_sha256
            )
            .as_bytes(),
        );
        let generalized_change = generalized_change_for_candidate(
            state_dir,
            source_generation,
            &candidate.relative_path,
            &candidate.transformation,
            &candidate.solution_strategy,
            &candidate.predecessor_sha256,
            &candidate.candidate_sha256,
            WeaknessEvidenceKind::ExplicitCodeHole,
            &evidence_sha256,
            "current Rust AST contains an executable todo or unimplemented hole",
            &candidate.consequence_predictions,
            &candidate.structural_repair_program,
        )?;
        return Ok(Some(AutonomousSourcePatchRequest {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id,
            relative_path: candidate.relative_path,
            predecessor_sha256: candidate.predecessor_sha256,
            candidate_source: candidate.candidate_source,
            candidate_sha256: candidate.candidate_sha256,
            transformation: candidate.transformation,
            consequence_predictions: candidate.consequence_predictions,
            predicted_value: candidate.predicted_value,
            source_generation,
            core_generated: true,
            core_self_approved: true,
            solution_strategy: candidate.solution_strategy,
            structural_repair_program: Some(candidate.structural_repair_program),
            generalized_change: Some(generalized_change),
        }));
    }
    Ok(None)
}

pub fn discover_known_source_improvement_detailed(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<SourceDiscoveryResult, String> {
    validate_policy(policy)?;
    if !policy.enabled
        || (!policy.auto_discover_known_transformations
            && !policy.auto_discover_compiler_repairs
            && !policy.auto_synthesize_grammar_repairs)
    {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Disabled,
            candidate: None,
        });
    }
    if let Some(candidate) = compiler_guided_request(policy, state_dir, source_generation)? {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Candidate,
            candidate: Some(candidate),
        });
    }
    if let Some(candidate) = grammar_synthesized_request(policy, state_dir, source_generation)? {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::Candidate,
            candidate: Some(candidate),
        });
    }
    if !policy.auto_discover_known_transformations {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::NoApplicableTransformation,
            candidate: None,
        });
    }
    if KNOWN_REMAINDER_PREDICTED_VALUE < policy.minimum_predicted_value {
        return Ok(SourceDiscoveryResult {
            disposition: SourceDiscoveryDisposition::BelowValueThreshold,
            candidate: None,
        });
    }
    for path in rust_source_files(&policy.source_root)? {
        let bytes = fs::read(&path)
            .map_err(|error| format!("SOURCE_DISCOVERY_READ:{}:{error}", path.display()))?;
        if bytes.len() as u64 > policy.max_candidate_bytes {
            continue;
        }
        let Ok(source) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let relative_path = path
            .strip_prefix(&policy.source_root)
            .map_err(|_| "SOURCE_DISCOVERY_PATH_OUTSIDE_ROOT".to_string())?
            .to_path_buf();
        let predecessor_sha256 = sha256(&bytes);
        let transformation =
            "MANUAL_REMAINDER_PREDICATE_TO_TYPED_DIVISIBILITY_PREDICATE".to_string();
        let problem_id = repair_problem_id_for(&relative_path, &transformation);
        let record = load_repair_learning(state_dir, &problem_id)?;
        let attempted = record
            .as_ref()
            .map(|knowledge| active_cycle_attempts(knowledge, source_generation))
            .unwrap_or(&[]);
        if attempted.len() >= usize::from(policy.max_attempts_per_problem) {
            continue;
        }
        for (strategy_index, solution_strategy) in KNOWN_REMAINDER_STRATEGIES
            .iter()
            .enumerate()
            .take(usize::from(policy.max_attempts_per_problem))
        {
            if attempted
                .iter()
                .any(|attempt| attempt.solution_strategy == *solution_strategy)
                || !repair_strategy_is_available(
                    policy,
                    state_dir,
                    &relative_path,
                    &transformation,
                    solution_strategy,
                    source_generation,
                )?
            {
                continue;
            }
            let Some(candidate_source) = rewrite_first_known_improvement(source, strategy_index)
            else {
                continue;
            };
            let structural_repair_program = match synthesize_structural_repair(
                &structural_file_id(&relative_path),
                source,
                &candidate_source,
            ) {
                Ok(program) => program,
                Err(_) => continue,
            };
            let candidate_sha256 = sha256(candidate_source.as_bytes());
            let patch_id = format!(
                "SELF-{}",
                &sha256(
                    format!(
                        "{}:{}:{}:{}",
                        problem_id, source_generation, solution_strategy, candidate_sha256
                    )
                    .as_bytes()
                )[..24]
            );
            if state_dir
                .join("source_mutations")
                .join(&patch_id)
                .join("receipt.json")
                .exists()
            {
                continue;
            }
            let consequence_predictions = vec![
                "preserve parity/divisibility semantics".to_string(),
                "replace a manual predicate using a distinct bounded repair strategy".to_string(),
                "retain only a method that passes format, regression, and release build gates"
                    .to_string(),
            ];
            let evidence_sha256 = sha256(
                format!(
                    "{}:{}:{}",
                    relative_path.display(),
                    transformation,
                    predecessor_sha256
                )
                .as_bytes(),
            );
            let generalized_change = generalized_change_for_candidate(
                state_dir,
                source_generation,
                &relative_path,
                &transformation,
                solution_strategy,
                &predecessor_sha256,
                &candidate_sha256,
                WeaknessEvidenceKind::StructuralSourceSmell,
                &evidence_sha256,
                "current source contains a mechanically recognized redundant predicate form",
                &consequence_predictions,
                &structural_repair_program,
            )?;
            return Ok(SourceDiscoveryResult {
                disposition: SourceDiscoveryDisposition::Candidate,
                candidate: Some(AutonomousSourcePatchRequest {
                    schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
                    patch_id,
                    relative_path: relative_path.clone(),
                    predecessor_sha256: predecessor_sha256.clone(),
                    candidate_source,
                    candidate_sha256,
                    transformation: transformation.clone(),
                    consequence_predictions,
                    predicted_value: KNOWN_REMAINDER_PREDICTED_VALUE,
                    source_generation,
                    core_generated: true,
                    core_self_approved: true,
                    solution_strategy: (*solution_strategy).to_string(),
                    structural_repair_program: Some(structural_repair_program),
                    generalized_change: Some(generalized_change),
                }),
            });
        }
    }
    Ok(SourceDiscoveryResult {
        disposition: SourceDiscoveryDisposition::NoApplicableTransformation,
        candidate: None,
    })
}

pub fn discover_known_source_improvement(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    Ok(discover_known_source_improvement_detailed(policy, state_dir, source_generation)?.candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_receipt(
        request: &AutonomousSourcePatchRequest,
        installed: bool,
    ) -> AutonomousSourcePatchReceipt {
        let output: &[u8] = if installed { b"pass" } else { b"failure" };
        let command = LocalCommandReceipt {
            program: "cargo".to_string(),
            args: vec!["test".to_string()],
            exit_code: Some(if installed { 0 } else { 101 }),
            success: installed,
            timed_out: false,
            duration_ms: 1,
            output_sha256: sha256(output),
            diagnostic_tail: String::from_utf8_lossy(output).to_string(),
        };
        let mut receipt = AutonomousSourcePatchReceipt {
            schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
            patch_id: request.patch_id.clone(),
            relative_path: request.relative_path.clone(),
            predecessor_sha256: request.predecessor_sha256.clone(),
            candidate_sha256: request.candidate_sha256.clone(),
            core_generated: true,
            core_self_approved: true,
            installed,
            rolled_back: !installed,
            failure_reason: (!installed).then(|| "SYNTHETIC_FAILURE".to_string()),
            format_check: Some(command.clone()),
            compile_check: Some(command.clone()),
            validation: command.clone(),
            release_build: installed.then_some(command),
            runtime_update_staged: installed,
            rollback_source: PathBuf::from("predecessor.source"),
            workspace_fingerprint_before: "a".repeat(64),
            workspace_fingerprint_after: "a".repeat(64),
            workspace_stable_during_validation: true,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_hash(&receipt).unwrap();
        receipt
    }

    fn cargo_path() -> PathBuf {
        let candidate = std::env::var_os("CARGO")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("cargo.exe"));
        if candidate.is_absolute() {
            candidate
        } else {
            std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .map(|home| home.join("bin").join(&candidate))
                .filter(|path| path.is_file())
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
                        .join(".cargo")
                        .join("bin")
                        .join(candidate)
                })
        }
    }

    fn fixture(label: &str) -> (PathBuf, AutonomousSourceMutationPolicy) {
        let root = std::env::temp_dir().join(format!(
            "b-core-source-mutation-{label}-{}-{}",
            std::process::id(),
            crate::self_repair_contract::sha256(label.as_bytes())
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='semantic-reasoning'\nversion='0.1.0'\nedition='2021'\n\n[lib]\npath='src/lib.rs'\n\n[[bin]]\nname='b-core-growth-supervisor'\npath='src/growth_supervisor_main.rs'\n\n[[bin]]\nname='b-core-growth-verifier'\npath='src/growth_verifier_main.rs'\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"semantic-reasoning\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "pub fn even(value: u32) -> bool {\n    value % 2 == 0\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn even_works() {\n        assert!(super::even(2));\n    }\n}\n",
        )
        .unwrap();
        fs::write(root.join("src/growth_supervisor_main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("src/growth_verifier_main.rs"), "fn main() {}\n").unwrap();
        let policy = AutonomousSourceMutationPolicy {
            enabled: true,
            source_root: root.clone(),
            cargo_executable: cargo_path(),
            build_target_dir: root.join("target"),
            runtime_bin_dir: root.join("runtime"),
            auto_discover_known_transformations: true,
            auto_discover_compiler_repairs: false,
            auto_synthesize_grammar_repairs: false,
            max_candidate_bytes: 1024 * 1024,
            max_installations: 4,
            validation_timeout_ms: 120_000,
            max_attempts_per_problem: 4,
            minimum_predicted_value: 0,
        };
        (root, policy)
    }

    fn external_state(root: &Path) -> PathBuf {
        let state = root.with_file_name(format!(
            "{}-state",
            root.file_name().and_then(OsStr::to_str).unwrap_or("b-core")
        ));
        if state.exists() {
            fs::remove_dir_all(&state).unwrap();
        }
        state
    }

    #[test]
    fn known_improvement_is_predicted_without_touching_tests_or_strings() {
        let source = "pub fn even(value: u32) -> bool { value % 2 == 0 }\n#[cfg(test)]\nmod tests { const TEXT: &str = \"x % 2 == 0\"; }\n";
        let rewritten = rewrite_first_known_improvement(source, 0).expect("candidate");
        assert!(rewritten.contains("value.is_multiple_of(2)"));
        assert!(rewritten.contains("\"x % 2 == 0\""));

        let conditional = "let scope = if ordinal % 5 == 0 { 1 } else { 2 };\n";
        let rewritten = rewrite_first_known_improvement(conditional, 0).expect("conditional");
        assert!(rewritten.contains("= if ordinal.is_multiple_of(5)"));

        let match_arm = "            Self::Even => value % 2 == 0,\n";
        for strategy in 0..KNOWN_REMAINDER_STRATEGIES.len() {
            let rewritten =
                rewrite_first_known_improvement(match_arm, strategy).expect("match arm");
            assert!(rewritten.contains("Self::Even =>"));
            assert!(!rewritten.contains("=(>"));
            assert!(!rewritten.contains("=matches"));
        }
    }

    #[test]
    fn traversal_and_absolute_targets_are_rejected() {
        let root = std::env::temp_dir();
        assert!(normalized_target(&root, Path::new("..\\escape.rs")).is_err());
        assert!(normalized_target(&root, Path::new("C:\\escape.rs")).is_err());
    }

    #[test]
    fn default_retry_bound_is_backward_compatible_with_frozen_configs() {
        let policy = AutonomousSourceMutationPolicy::default();
        let serialized = serde_json::to_value(&policy).unwrap();
        assert!(serialized.get("max_attempts_per_problem").is_none());
        assert!(serialized.get("minimum_predicted_value").is_none());
        assert!(serialized.get("auto_discover_compiler_repairs").is_none());
        assert!(serialized.get("auto_synthesize_grammar_repairs").is_none());
        let restored: AutonomousSourceMutationPolicy = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.max_attempts_per_problem, 4);
        assert_eq!(restored.minimum_predicted_value, 60);
        assert!(restored.auto_discover_compiler_repairs);
        assert!(restored.auto_synthesize_grammar_repairs);
    }

    #[test]
    fn low_value_cosmetic_discovery_is_skipped_before_validation() {
        let (root, mut policy) = fixture("utility-gate");
        policy.minimum_predicted_value = 60;
        let state = external_state(&root);
        let discovery = discover_known_source_improvement_detailed(&policy, &state, 1).unwrap();
        assert_eq!(
            discovery.disposition,
            SourceDiscoveryDisposition::BelowValueThreshold
        );
        assert!(discovery.candidate.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_fingerprint_detects_non_target_changes() {
        let (root, _) = fixture("workspace-fingerprint");
        let target = root.join("src/lib.rs");
        let before = workspace_semantic_fingerprint(&root, &target).unwrap();
        fs::write(root.join("src/concurrent.rs"), "pub fn changed() {}\n").unwrap();
        let after = workspace_semantic_fingerprint(&root, &target).unwrap();
        assert_ne!(before, after);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compiler_observation_autonomously_finds_and_repairs_a_fresh_defect() {
        let (root, mut policy) = fixture("compiler-guided-defect");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    1;\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 4)
            .unwrap()
            .expect("compiler-guided repair candidate");

        assert!(request.transformation.starts_with("COMPILER_OBSERVATION:"));
        assert!(request
            .solution_strategy
            .starts_with("COMPILER_SUGGESTION:"));
        assert!(request.structural_repair_program.is_some());
        assert_eq!(
            request.candidate_source,
            "pub fn value() -> i32 {\n    1\n}\n"
        );
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).unwrap(),
            request.candidate_source
        );
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned compiler repair");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| !success.edit_atom_kinds.is_empty()));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn grammar_atoms_compose_and_validate_new_code_without_a_gold_patch() {
        let (root, mut policy) = fixture("grammar-composition-defect");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn add(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn add_works() {\n        assert_eq!(super::add(2, 3), 5);\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);

        let request = discover_known_source_improvement(&policy, &state, 5)
            .unwrap()
            .expect("grammar-composed repair candidate");

        assert!(request.transformation.starts_with("AST_GRAMMAR_HOLE:TODO:"));
        assert!(request
            .solution_strategy
            .starts_with("GRAMMAR_COMPOSITION:BINARY_ADD"));
        assert!(request.candidate_source.contains("    left + right\n"));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(receipt.validation.success);
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("learned grammar composition");
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert!(learned
            .learned_success
            .as_ref()
            .is_some_and(|success| success.solution_strategy.contains("BINARY_ADD")));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn public_counterexamples_drive_bounded_grammar_revision_until_success() {
        let (root, mut policy) = fixture("grammar-counterexample-revision");
        policy.auto_discover_known_transformations = false;
        policy.auto_discover_compiler_repairs = false;
        policy.auto_synthesize_grammar_repairs = true;
        policy.minimum_predicted_value = 60;
        policy.max_attempts_per_problem = 4;
        fs::write(
            root.join("src/lib.rs"),
            "pub fn combine(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combine_works() {\n        let observed = super::combine(3, 4);\n        if observed != 12 {\n            panic!(\"assertion `left == right` failed\\n  left: {observed}\\n right: 12\");\n        }\n    }\n}\n",
        )
        .unwrap();
        let state = external_state(&root);
        let mut strategies = Vec::new();
        let mut final_receipt = None;

        for _ in 0..4 {
            let request = discover_known_source_improvement(&policy, &state, 6)
                .unwrap()
                .expect("next grammar hypothesis");
            strategies.push(request.solution_strategy.clone());
            let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
            if receipt.installed {
                final_receipt = Some((request, receipt));
                break;
            }
            assert!(receipt.rolled_back);
            assert_eq!(
                fs::read_to_string(root.join("src/lib.rs")).unwrap(),
                "pub fn combine(left: i32, right: i32) -> i32 {\n    todo!()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn combine_works() {\n        let observed = super::combine(3, 4);\n        if observed != 12 {\n            panic!(\"assertion `left == right` failed\\n  left: {observed}\\n right: 12\");\n        }\n    }\n}\n"
            );
        }

        let (request, receipt) =
            final_receipt.expect("feedback-ranked grammar composition succeeds");
        assert!(receipt.validation.success);
        assert_eq!(strategies.len(), 2);
        assert!(strategies[0].contains("BINARY_ADD"));
        assert!(strategies[1].contains("BINARY_MULTIPLY"));
        let learned = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("counterexample-guided learning record");
        let success = learned
            .learned_success
            .expect("learned successful composition");
        assert_eq!(success.attempts_required, 2);
        assert!(success.solution_strategy.contains("BINARY_MULTIPLY"));
        assert_eq!(learned.attempts.len(), 2);
        let first_counterexample = learned.attempts[0]
            .validation_counterexample
            .as_ref()
            .expect("public failure becomes a structured counterexample");
        assert_eq!(
            first_counterexample.numeric_relation,
            Some(crate::generalized_self_application::NumericRelation::ExpectedGreaterThanObserved)
        );
        assert!(learned.attempts[1]
            .derived_from_counterexample_ids
            .contains(&first_counterexample.counterexample_id));
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn core_can_install_validate_and_stage_its_own_source_patch() {
        let (root, policy) = fixture("install");
        let state = external_state(&root);
        let request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        assert!(request.core_self_approved);
        assert!(request
            .structural_repair_program
            .as_ref()
            .is_some_and(|program| program.file_id == "src/lib.rs"));
        assert!(request.generalized_change.as_ref().is_some_and(|change| {
            !change.fixed_toggle_patch
                && !change.one_generation_only
                && change.source_generation == 3
        }));
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(receipt.installed);
        assert!(!receipt.rolled_back);
        assert!(receipt.validation.success);
        assert!(receipt
            .release_build
            .as_ref()
            .is_some_and(|value| value.success));
        assert!(receipt.runtime_update_staged);
        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("value.is_multiple_of(2)"));
        assert!(state
            .join("control")
            .join(SELF_UPDATE_HANDOFF_FILE)
            .is_file());
        let knowledge = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("success knowledge");
        assert_eq!(knowledge.status, "LEARNED_SUCCESS");
        assert_eq!(knowledge.attempts.len(), 1);
        let learned = knowledge.learned_success.unwrap();
        assert_eq!(learned.solution_strategy, "TYPED_IS_MULTIPLE_OF");
        assert!(learned.structural_repair_program_sha256.is_some());
        assert!(!learned.edit_atom_kinds.is_empty());
        assert!(learned.structural_postcondition_count > 0);
        assert!(learned.generalized_change_sha256.is_some());
        let incremental = root.join("target/debug/incremental");
        assert!(!incremental.exists() || fs::read_dir(&incremental).unwrap().next().is_none());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn failed_self_patch_is_rolled_back_to_exact_predecessor() {
        let (root, policy) = fixture("rollback");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        request.patch_id.push_str("-invalid");
        request.candidate_source = "pub fn broken( {\n".to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());
        request.structural_repair_program = None;
        request.generalized_change = None;
        let receipt = install_and_stage_source_patch(&policy, &state, &request).unwrap();
        assert!(!receipt.installed);
        assert!(receipt.rolled_back);
        assert!(!receipt.validation.success);
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state
            .join("control")
            .join(SELF_UPDATE_HANDOFF_FILE)
            .exists());
        let knowledge = load_repair_learning(&state, &repair_problem_id(&request))
            .unwrap()
            .expect("retry knowledge");
        assert_eq!(knowledge.status, "RETRYING");
        assert_eq!(knowledge.attempts.len(), 1);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }

    #[test]
    fn tampered_candidate_cannot_bypass_structural_program_replay() {
        let (root, policy) = fixture("structural-replay");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        request.candidate_source = "pub fn even(_: u32) -> bool { false }\n".to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());

        let error = install_and_stage_source_patch(&policy, &state, &request).unwrap_err();

        assert!(error.contains("STRUCTURAL_REPAIR_REPLAY_MISMATCH"));
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state.join("source_mutations").exists());
        fs::remove_dir_all(root).unwrap();
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
    }

    #[test]
    fn tampered_generalized_change_cannot_bypass_source_binding() {
        let (root, policy) = fixture("generalized-change-tamper");
        let state = external_state(&root);
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 11)
            .unwrap()
            .expect("generalized change");
        request
            .generalized_change
            .as_mut()
            .expect("change")
            .solution_strategy = "FIXED_SEM9_TOGGLE_REPLAY".to_string();

        let error = install_and_stage_source_patch(&policy, &state, &request).unwrap_err();
        assert_eq!(error, "GENERALIZED_CHANGE_REQUEST_BINDING_FAILURE");
        assert_eq!(fs::read(root.join("src/lib.rs")).unwrap(), predecessor);
        assert!(!state.join("source_mutations").exists());
        fs::remove_dir_all(root).unwrap();
        if state.exists() {
            fs::remove_dir_all(state).unwrap();
        }
    }

    #[test]
    fn four_failed_solutions_are_admitted_then_reopened_after_growth() {
        let (root, policy) = fixture("bounded-retry");
        let state = external_state(&root);
        let mut problem_id = String::new();
        for expected_attempts in 1..=4 {
            let request = discover_known_source_improvement(&policy, &state, 7)
                .unwrap()
                .expect("bounded solution");
            problem_id = repair_problem_id(&request);
            let receipt = synthetic_receipt(&request, false);
            let knowledge =
                record_source_repair_outcome(&policy, &state, &request, &receipt).unwrap();
            assert_eq!(knowledge.attempts.len(), expected_attempts);
        }
        let admitted = load_repair_learning(&state, &problem_id)
            .unwrap()
            .expect("admitted failure");
        assert_eq!(admitted.status, "ADMITTED_FAILURE");
        assert_eq!(admitted.eligible_after_generation, Some(8));
        assert!(discover_known_source_improvement(&policy, &state, 7)
            .unwrap()
            .is_none());

        let retry = discover_known_source_improvement(&policy, &state, 8)
            .unwrap()
            .expect("reopened after growth");
        let success = synthetic_receipt(&retry, true);
        let learned = record_source_repair_outcome(&policy, &state, &retry, &success).unwrap();
        assert_eq!(learned.status, "LEARNED_SUCCESS");
        assert_eq!(learned.attempts.len(), 1);
        assert_eq!(learned.learned_success.unwrap().attempts_required, 1);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(state).unwrap();
    }
}
