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

use crate::self_repair_contract::sha256;

pub const AUTONOMOUS_SOURCE_MUTATION_SCHEMA: &str = "B_CORE_AUTONOMOUS_SOURCE_MUTATION_1";
pub const SELF_UPDATE_HANDOFF_FILE: &str = "SELF_UPDATE_READY.json";
pub const SOURCE_REPAIR_LEARNING_SCHEMA: &str = "B_CORE_SOURCE_REPAIR_LEARNING_1";
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousSourceMutationPolicy {
    pub enabled: bool,
    pub source_root: PathBuf,
    pub cargo_executable: PathBuf,
    pub build_target_dir: PathBuf,
    pub runtime_bin_dir: PathBuf,
    pub auto_discover_known_transformations: bool,
    pub max_candidate_bytes: u64,
    pub max_installations: u64,
    pub validation_timeout_ms: u64,
    #[serde(
        default = "default_source_repair_attempts",
        skip_serializing_if = "is_default_source_repair_attempts"
    )]
    pub max_attempts_per_problem: u8,
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
            max_candidate_bytes: 2 * 1024 * 1024,
            max_installations: 64,
            validation_timeout_ms: 15 * 60 * 1_000,
            max_attempts_per_problem: default_source_repair_attempts(),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedSuccessfulRepair {
    pub learned_at_generation: u64,
    pub solution_strategy: String,
    pub candidate_sha256: String,
    pub validation_output_sha256: String,
    pub release_build_output_sha256: String,
    pub attempts_required: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRepairLearningRecord {
    pub schema: String,
    pub problem_id: String,
    pub relative_path: PathBuf,
    pub transformation: String,
    pub status: String,
    pub cycle_started_generation: u64,
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
    pub format_check: Option<LocalCommandReceipt>,
    pub validation: LocalCommandReceipt,
    pub release_build: Option<LocalCommandReceipt>,
    pub runtime_update_staged: bool,
    pub rollback_source: PathBuf,
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

fn active_cycle_attempts<'a>(
    record: &'a SourceRepairLearningRecord,
    source_generation: u64,
) -> &'a [SourceRepairAttempt] {
    if record.status == "ADMITTED_FAILURE"
        && record
            .eligible_after_generation
            .is_some_and(|eligible| source_generation >= eligible)
    {
        &[]
    } else {
        &record.attempts
    }
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
            eligible_after_generation: None,
            attempts: Vec::new(),
            learned_success: None,
        }
    });
    if record.status == "ADMITTED_FAILURE"
        && record
            .eligible_after_generation
            .is_some_and(|eligible| request.source_generation >= eligible)
    {
        record.status = "RETRYING".to_string();
        record.cycle_started_generation = request.source_generation;
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
    record.attempts.push(SourceRepairAttempt {
        attempt_number,
        source_generation: request.source_generation,
        solution_strategy: solution_strategy.clone(),
        candidate_sha256: request.candidate_sha256.clone(),
        succeeded: receipt.installed,
        receipt_sha256: receipt.receipt_sha256.clone(),
        diagnostic_sha256: receipt.validation.output_sha256.clone(),
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
            format_check: Some(format_check.clone()),
            validation: format_check,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
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
            format_check: Some(format_check),
            validation,
            release_build: None,
            runtime_update_staged: false,
            rollback_source,
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
            format_check: Some(format_check),
            validation,
            release_build: Some(release_build),
            runtime_update_staged: false,
            rollback_source,
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
        format_check: Some(format_check),
        validation,
        release_build: Some(release_build),
        runtime_update_staged: true,
        rollback_source,
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
            b'=' | b';' | b',' | b'{' | b'[' | b'!' | b'&' | b'|' if depth == 0 => {
                return Some(index + 1)
            }
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

pub fn discover_known_source_improvement(
    policy: &AutonomousSourceMutationPolicy,
    state_dir: &Path,
    source_generation: u64,
) -> Result<Option<AutonomousSourcePatchRequest>, String> {
    validate_policy(policy)?;
    if !policy.enabled || !policy.auto_discover_known_transformations {
        return Ok(None);
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
        let problem_id = sha256(format!("{}:{transformation}", relative_path.display()).as_bytes());
        let record = load_repair_learning(state_dir, &problem_id)?;
        if record.as_ref().is_some_and(|knowledge| {
            knowledge.status == "LEARNED_SUCCESS"
                || (knowledge.status == "ADMITTED_FAILURE"
                    && knowledge
                        .eligible_after_generation
                        .is_some_and(|eligible| source_generation < eligible))
        }) {
            continue;
        }
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
            {
                continue;
            }
            let Some(candidate_source) = rewrite_first_known_improvement(source, strategy_index)
            else {
                continue;
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
            return Ok(Some(AutonomousSourcePatchRequest {
                schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
                patch_id,
                relative_path: relative_path.clone(),
                predecessor_sha256: predecessor_sha256.clone(),
                candidate_source,
                candidate_sha256,
                transformation: transformation.clone(),
                consequence_predictions: vec![
                    "preserve parity/divisibility semantics".to_string(),
                    "replace a manual predicate using a distinct bounded repair strategy"
                        .to_string(),
                    "retain only a method that passes format, regression, and release build gates"
                        .to_string(),
                ],
                predicted_value: 78,
                source_generation,
                core_generated: true,
                core_self_approved: true,
                solution_strategy: (*solution_strategy).to_string(),
            }));
        }
    }
    Ok(None)
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
            format_check: Some(command.clone()),
            validation: command.clone(),
            release_build: installed.then_some(command),
            runtime_update_staged: installed,
            rollback_source: PathBuf::from("predecessor.source"),
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
            max_candidate_bytes: 1024 * 1024,
            max_installations: 4,
            validation_timeout_ms: 120_000,
            max_attempts_per_problem: 4,
        };
        (root, policy)
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
        let restored: AutonomousSourceMutationPolicy = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.max_attempts_per_problem, 4);
    }

    #[test]
    fn core_can_install_validate_and_stage_its_own_source_patch() {
        let (root, policy) = fixture("install");
        let state = root.join("state");
        let request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        assert!(request.core_self_approved);
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
        assert_eq!(
            knowledge.learned_success.unwrap().solution_strategy,
            "TYPED_IS_MULTIPLE_OF"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_self_patch_is_rolled_back_to_exact_predecessor() {
        let (root, policy) = fixture("rollback");
        let state = root.join("state");
        let predecessor = fs::read(root.join("src/lib.rs")).unwrap();
        let mut request = discover_known_source_improvement(&policy, &state, 3)
            .unwrap()
            .expect("discovered improvement");
        request.patch_id.push_str("-invalid");
        request.candidate_source = "pub fn broken( {\n".to_string();
        request.candidate_sha256 = sha256(request.candidate_source.as_bytes());
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
    }

    #[test]
    fn four_failed_solutions_are_admitted_then_reopened_after_growth() {
        let (root, policy) = fixture("bounded-retry");
        let state = root.join("state");
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
    }
}
