//! Evidence-bound experience for API migrations, environment failures, and
//! nondeterministic repository defects.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::cross_language_synthesis::{code_identifiers, locate_function, CrossLanguage};
use crate::self_repair_contract::sha256;

pub const REPOSITORY_CHANGE_EXPERIENCE_SCHEMA: &str = "B_REPOSITORY_CHANGE_EXPERIENCE_1";
pub const MAX_MIGRATION_FILES: usize = 256;
pub const MAX_MIGRATION_BYTES: usize = 16 * 1024 * 1024;
static API_VALIDATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySourceFileIR {
    pub relative_path: PathBuf,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMigrationRequestIR {
    pub language: CrossLanguage,
    pub files: Vec<RepositorySourceFileIR>,
    pub old_symbol: String,
    pub new_symbol: String,
    pub preserve_public_api: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedSourceFileIR {
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub code_identifier_replacements: usize,
    pub candidate_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMigrationReceiptIR {
    pub schema: String,
    pub language: CrossLanguage,
    pub old_symbol: String,
    pub new_symbol: String,
    pub definition_owner: PathBuf,
    pub migrated_files: Vec<MigratedSourceFileIR>,
    pub definition_count: usize,
    pub callsite_and_import_replacements: usize,
    pub compatibility_shims: usize,
    pub comments_or_strings_rewritten: usize,
    pub candidate_manifest_sha256: String,
    pub repository_identity_routing_events: u64,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMigrationNativeValidationRequestIR {
    pub tool_path: PathBuf,
    /// Required for TypeScript. The runtime remains Node; this path must be a
    /// real `tsc` installation used before any emitted JavaScript executes.
    #[serde(default)]
    pub typescript_compiler_path: Option<PathBuf>,
    pub harness_files: Vec<RepositorySourceFileIR>,
    pub expected_output_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMigrationNativeValidationReceiptIR {
    pub language: CrossLanguage,
    pub tool_path: PathBuf,
    pub command_status: Option<i32>,
    #[serde(default)]
    pub typecheck_tool_path: Option<PathBuf>,
    #[serde(default)]
    pub typecheck_status: Option<i32>,
    #[serde(default)]
    pub typecheck_pass: bool,
    #[serde(default)]
    pub typecheck_stdout_sha256: String,
    #[serde(default)]
    pub typecheck_stderr_sha256: String,
    pub expected_output_observed: bool,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub diagnostic_excerpt: String,
    pub sandbox_cleaned: bool,
    pub pass: bool,
    pub network_reads: u64,
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn safe_relative_path(path: &std::path::Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn definition_positions(source: &str, language: CrossLanguage, symbol: &str) -> Vec<usize> {
    let keyword = match language {
        CrossLanguage::JavaScript | CrossLanguage::TypeScript => "function",
        CrossLanguage::Go => "func",
    };
    code_identifiers(source)
        .windows(2)
        .filter(|window| window[0].0 == keyword && window[1].0 == symbol)
        .map(|window| window[1].1)
        .collect()
}

fn replace_code_identifier(source: &str, old: &str, new: &str) -> (String, usize) {
    let replacements = code_identifiers(source)
        .into_iter()
        .filter(|(identifier, _, _)| identifier == old)
        .map(|(_, start, end)| (start, end))
        .collect::<Vec<_>>();
    let mut candidate = source.to_string();
    for (start, end) in replacements.iter().rev() {
        candidate.replace_range(*start..*end, new);
    }
    (candidate, replacements.len())
}

fn compatibility_shim(
    source: &str,
    language: CrossLanguage,
    old: &str,
    new: &str,
) -> Result<(usize, String), String> {
    let boundary = locate_function(source, language, new)?;
    let shim = match language {
        CrossLanguage::JavaScript | CrossLanguage::TypeScript => {
            format!("\nexport const {old} = {new};\n")
        }
        CrossLanguage::Go => {
            let signature = source[boundary.parameter_start..boundary.body_start].trim_end();
            let return_type = source[boundary.parameter_end + 1..boundary.body_start].trim();
            let arguments = boundary.parameter_names.join(", ");
            if return_type.is_empty() {
                format!("\n\nfunc {old}{signature} {{\n\t{new}({arguments})\n}}\n")
            } else {
                format!("\n\nfunc {old}{signature} {{\n\treturn {new}({arguments})\n}}\n")
            }
        }
    };
    let insertion = boundary.body_end + 1;
    let mut candidate = String::with_capacity(source.len() + shim.len());
    candidate.push_str(&source[..insertion]);
    candidate.push_str(&shim);
    candidate.push_str(&source[insertion..]);
    Ok((1, candidate))
}

/// Atomically plan a bounded multi-file API rename with an optional public
/// compatibility shim. The returned files are candidates; this function does
/// not write repository state.
pub fn migrate_repository_api(
    request: &ApiMigrationRequestIR,
) -> Result<ApiMigrationReceiptIR, String> {
    if !valid_identifier(&request.old_symbol)
        || !valid_identifier(&request.new_symbol)
        || request.old_symbol == request.new_symbol
    {
        return Err("API_MIGRATION_INVALID_SYMBOLS".to_string());
    }
    if request.files.is_empty() || request.files.len() > MAX_MIGRATION_FILES {
        return Err("API_MIGRATION_FILE_BOUND".to_string());
    }
    let total_bytes = request
        .files
        .iter()
        .try_fold(0usize, |total, file| total.checked_add(file.source.len()))
        .ok_or_else(|| "API_MIGRATION_BYTE_OVERFLOW".to_string())?;
    if total_bytes > MAX_MIGRATION_BYTES
        || request
            .files
            .iter()
            .any(|file| !safe_relative_path(&file.relative_path))
    {
        return Err("API_MIGRATION_SCOPE_BOUND".to_string());
    }
    if request.files.iter().any(|file| {
        code_identifiers(&file.source)
            .iter()
            .any(|(identifier, _, _)| identifier == &request.new_symbol)
    }) {
        return Err("API_MIGRATION_NEW_SYMBOL_COLLISION".to_string());
    }
    let definitions = request
        .files
        .iter()
        .enumerate()
        .flat_map(|(file_index, file)| {
            definition_positions(&file.source, request.language, &request.old_symbol)
                .into_iter()
                .map(move |position| (file_index, position))
        })
        .collect::<Vec<_>>();
    if definitions.len() != 1 {
        return Err(format!(
            "API_MIGRATION_DEFINITION_CARDINALITY:{}",
            definitions.len()
        ));
    }
    let owner_index = definitions[0].0;
    let total_occurrences = request
        .files
        .iter()
        .map(|file| {
            code_identifiers(&file.source)
                .iter()
                .filter(|(identifier, _, _)| identifier == &request.old_symbol)
                .count()
        })
        .sum::<usize>();
    if total_occurrences < 2 {
        return Err("API_MIGRATION_NO_CALLSITE_EVIDENCE".to_string());
    }

    let mut candidates = request
        .files
        .iter()
        .map(|file| {
            let (candidate_source, replacements) =
                replace_code_identifier(&file.source, &request.old_symbol, &request.new_symbol);
            (candidate_source, replacements)
        })
        .collect::<Vec<_>>();
    let compatibility_shims = if request.preserve_public_api {
        let (count, source) = compatibility_shim(
            &candidates[owner_index].0,
            request.language,
            &request.old_symbol,
            &request.new_symbol,
        )?;
        candidates[owner_index].0 = source;
        count
    } else {
        0
    };
    let migrated_files = request
        .files
        .iter()
        .zip(candidates)
        .map(
            |(file, (candidate_source, replacements))| MigratedSourceFileIR {
                relative_path: file.relative_path.clone(),
                predecessor_sha256: sha256(file.source.as_bytes()),
                candidate_sha256: sha256(candidate_source.as_bytes()),
                code_identifier_replacements: replacements,
                candidate_source,
            },
        )
        .collect::<Vec<_>>();
    let manifest_bytes = serde_json::to_vec(
        &migrated_files
            .iter()
            .map(|file| (&file.relative_path, &file.candidate_sha256))
            .collect::<Vec<_>>(),
    )
    .map_err(|error| format!("API_MIGRATION_MANIFEST:{error}"))?;
    Ok(ApiMigrationReceiptIR {
        schema: REPOSITORY_CHANGE_EXPERIENCE_SCHEMA.to_string(),
        language: request.language,
        old_symbol: request.old_symbol.clone(),
        new_symbol: request.new_symbol.clone(),
        definition_owner: request.files[owner_index].relative_path.clone(),
        migrated_files,
        definition_count: 1,
        callsite_and_import_replacements: total_occurrences - 1,
        compatibility_shims,
        comments_or_strings_rewritten: 0,
        candidate_manifest_sha256: sha256(&manifest_bytes),
        repository_identity_routing_events: 0,
        external_llm_calls: 0,
        network_reads: 0,
    })
}

/// Materialize an in-memory migration and caller-supplied regression harness
/// in an isolated temporary directory, then validate it with a local tool.
pub fn validate_api_migration_candidate(
    receipt: &ApiMigrationReceiptIR,
    request: &ApiMigrationNativeValidationRequestIR,
) -> Result<ApiMigrationNativeValidationReceiptIR, String> {
    let typescript_compiler = if receipt.language == CrossLanguage::TypeScript {
        Some(
            request
                .typescript_compiler_path
                .as_deref()
                .filter(|path| path.is_file())
                .ok_or_else(|| "API_MIGRATION_TYPESCRIPT_COMPILER_REQUIRED".to_string())?,
        )
    } else {
        None
    };
    if !request.tool_path.is_file()
        || request.harness_files.is_empty()
        || request.harness_files.len() > 8
        || request.expected_output_token.is_empty()
        || request.expected_output_token.len() > 128
        || request
            .harness_files
            .iter()
            .any(|file| !safe_relative_path(&file.relative_path))
    {
        return Err("API_MIGRATION_NATIVE_VALIDATION_BOUND".to_string());
    }
    let workspace = std::env::temp_dir().join(format!(
        "b-core-api-migration-{}-{}-{}",
        std::process::id(),
        &receipt.candidate_manifest_sha256[..16],
        API_VALIDATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    if workspace.exists() {
        fs::remove_dir_all(&workspace)
            .map_err(|error| format!("API_MIGRATION_NATIVE_CLEAN:{error}"))?;
    }
    fs::create_dir_all(&workspace)
        .map_err(|error| format!("API_MIGRATION_NATIVE_CREATE:{error}"))?;
    for file in &receipt.migrated_files {
        let path = workspace.join(&file.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("API_MIGRATION_NATIVE_CREATE:{error}"))?;
        }
        fs::write(path, &file.candidate_source)
            .map_err(|error| format!("API_MIGRATION_NATIVE_WRITE:{error}"))?;
    }
    for file in &request.harness_files {
        let path = workspace.join(&file.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("API_MIGRATION_NATIVE_CREATE:{error}"))?;
        }
        fs::write(path, &file.source)
            .map_err(|error| format!("API_MIGRATION_NATIVE_WRITE:{error}"))?;
    }
    let mut typecheck_status = None;
    let mut typecheck_pass = true;
    let mut typecheck_stdout = Vec::new();
    let mut typecheck_stderr = Vec::new();
    let mut runtime_harness = workspace.join(&request.harness_files[0].relative_path);
    if let Some(compiler) = typescript_compiler {
        fs::write(workspace.join("package.json"), "{\"type\":\"module\"}\n")
            .map_err(|error| format!("API_MIGRATION_NATIVE_WRITE:{error}"))?;
        let emitted = workspace.join("emitted");
        let mut typecheck = Command::new(compiler);
        typecheck.args([
            "--strict",
            "--noEmitOnError",
            "--target",
            "ES2022",
            "--module",
            "ES2022",
            "--moduleResolution",
            "bundler",
            "--rootDir",
            ".",
            "--outDir",
            "emitted",
        ]);
        for file in &receipt.migrated_files {
            typecheck.arg(&file.relative_path);
        }
        for file in &request.harness_files {
            typecheck.arg(&file.relative_path);
        }
        let output = typecheck
            .current_dir(&workspace)
            .output()
            .map_err(|error| format!("API_MIGRATION_TYPESCRIPT_EXECUTE:{error}"))?;
        typecheck_status = output.status.code();
        typecheck_pass = output.status.success();
        typecheck_stdout = output.stdout;
        typecheck_stderr = output.stderr;
        if !typecheck_pass {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&typecheck_stdout),
                String::from_utf8_lossy(&typecheck_stderr)
            );
            let result = ApiMigrationNativeValidationReceiptIR {
                language: receipt.language,
                tool_path: request.tool_path.clone(),
                command_status: None,
                typecheck_tool_path: Some(compiler.to_path_buf()),
                typecheck_status,
                typecheck_pass: false,
                typecheck_stdout_sha256: sha256(&typecheck_stdout),
                typecheck_stderr_sha256: sha256(&typecheck_stderr),
                expected_output_observed: false,
                stdout_sha256: sha256(&[]),
                stderr_sha256: sha256(&[]),
                diagnostic_excerpt: combined.chars().take(2_048).collect(),
                sandbox_cleaned: true,
                pass: false,
                network_reads: 0,
            };
            fs::remove_dir_all(&workspace)
                .map_err(|error| format!("API_MIGRATION_NATIVE_CLEAN:{error}"))?;
            return Ok(result);
        }
        runtime_harness = emitted.join(&request.harness_files[0].relative_path);
        runtime_harness.set_extension("js");
    }
    let mut command = Command::new(&request.tool_path);
    match receipt.language {
        CrossLanguage::JavaScript => {
            command.arg(&runtime_harness);
        }
        CrossLanguage::TypeScript => {
            command.arg(&runtime_harness);
        }
        CrossLanguage::Go => {
            command.arg("run");
            for file in &receipt.migrated_files {
                command.arg(&file.relative_path);
            }
            for file in &request.harness_files {
                command.arg(&file.relative_path);
            }
        }
    }
    let output = command
        .current_dir(&workspace)
        .output()
        .map_err(|error| format!("API_MIGRATION_NATIVE_EXECUTE:{error}"))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let expected_output_observed = combined.contains(&request.expected_output_token);
    let pass = output.status.success() && expected_output_observed;
    let result = ApiMigrationNativeValidationReceiptIR {
        language: receipt.language,
        tool_path: request.tool_path.clone(),
        command_status: output.status.code(),
        typecheck_tool_path: typescript_compiler.map(PathBuf::from),
        typecheck_status,
        typecheck_pass,
        typecheck_stdout_sha256: sha256(&typecheck_stdout),
        typecheck_stderr_sha256: sha256(&typecheck_stderr),
        expected_output_observed,
        stdout_sha256: sha256(&output.stdout),
        stderr_sha256: sha256(&output.stderr),
        diagnostic_excerpt: combined.chars().take(2_048).collect(),
        sandbox_cleaned: true,
        pass,
        network_reads: 0,
    };
    fs::remove_dir_all(&workspace)
        .map_err(|error| format!("API_MIGRATION_NATIVE_CLEAN:{error}"))?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvironmentFailureKind {
    ToolchainUnavailable,
    ToolchainVersionDrift,
    DependencyLockMismatch,
    MissingEnvironmentVariable,
    FeatureConfiguration,
    ModuleOrPathResolution,
    NativeLibraryUnavailable,
    PermissionBoundary,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvironmentProbeKind {
    ResolveExecutable,
    ReadToolVersion,
    CheckLockfileConsistency,
    CheckVariablePresence,
    EnumerateEnabledFeatures,
    ResolveModulePath,
    ResolveNativeLibrary,
    CheckWriteBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EnvironmentDiagnosisDisposition {
    Classified,
    NeedsProbe,
    InvalidEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentFailureEvidenceIR {
    pub command_label: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub repeated_attempts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentDiagnosisIR {
    pub schema: String,
    pub disposition: EnvironmentDiagnosisDisposition,
    pub kind: EnvironmentFailureKind,
    pub evidence_sha256: String,
    pub probes: Vec<EnvironmentProbeKind>,
    pub mutation_authorized: bool,
    pub automatic_install_authorized: bool,
    pub network_reads: u64,
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

/// Classify an executed environment failure and prescribe only read-only
/// probes. It never installs a tool or mutates configuration.
pub fn diagnose_environment_failure(
    evidence: &EnvironmentFailureEvidenceIR,
) -> EnvironmentDiagnosisIR {
    let combined = format!(
        "{}\n{}\n{}",
        evidence.command_label, evidence.stdout, evidence.stderr
    )
    .to_ascii_lowercase();
    let kind = if contains_any(
        &combined,
        &[
            "command not found",
            "is not recognized as an internal",
            "executable file not found",
            "cannot find the executable",
        ],
    ) {
        EnvironmentFailureKind::ToolchainUnavailable
    } else if contains_any(
        &combined,
        &[
            "requires rustc",
            "requires go ",
            "unsupported class file version",
            "node version",
            "toolchain version",
        ],
    ) {
        EnvironmentFailureKind::ToolchainVersionDrift
    } else if contains_any(
        &combined,
        &[
            "lock file needs to be updated",
            "lockfile is out of date",
            "frozen lockfile",
            "go.sum is missing",
        ],
    ) {
        EnvironmentFailureKind::DependencyLockMismatch
    } else if contains_any(
        &combined,
        &[
            "environment variable",
            "env var",
            "not set in the environment",
        ],
    ) {
        EnvironmentFailureKind::MissingEnvironmentVariable
    } else if contains_any(
        &combined,
        &[
            "feature is not enabled",
            "unknown feature",
            "build constraints exclude",
            "build tag",
        ],
    ) {
        EnvironmentFailureKind::FeatureConfiguration
    } else if contains_any(
        &combined,
        &[
            "cannot find module",
            "module not found",
            "no module provides package",
            "failed to resolve import",
        ],
    ) {
        EnvironmentFailureKind::ModuleOrPathResolution
    } else if contains_any(
        &combined,
        &[
            "shared object file",
            "dll was not found",
            "native library",
            "cannot open shared library",
        ],
    ) {
        EnvironmentFailureKind::NativeLibraryUnavailable
    } else if contains_any(
        &combined,
        &[
            "permission denied",
            "access is denied",
            "operation not permitted",
        ],
    ) {
        EnvironmentFailureKind::PermissionBoundary
    } else {
        EnvironmentFailureKind::Unknown
    };
    let probes = match kind {
        EnvironmentFailureKind::ToolchainUnavailable => {
            vec![EnvironmentProbeKind::ResolveExecutable]
        }
        EnvironmentFailureKind::ToolchainVersionDrift => vec![
            EnvironmentProbeKind::ResolveExecutable,
            EnvironmentProbeKind::ReadToolVersion,
        ],
        EnvironmentFailureKind::DependencyLockMismatch => {
            vec![EnvironmentProbeKind::CheckLockfileConsistency]
        }
        EnvironmentFailureKind::MissingEnvironmentVariable => {
            vec![EnvironmentProbeKind::CheckVariablePresence]
        }
        EnvironmentFailureKind::FeatureConfiguration => {
            vec![EnvironmentProbeKind::EnumerateEnabledFeatures]
        }
        EnvironmentFailureKind::ModuleOrPathResolution => {
            vec![EnvironmentProbeKind::ResolveModulePath]
        }
        EnvironmentFailureKind::NativeLibraryUnavailable => {
            vec![EnvironmentProbeKind::ResolveNativeLibrary]
        }
        EnvironmentFailureKind::PermissionBoundary => {
            vec![EnvironmentProbeKind::CheckWriteBoundary]
        }
        EnvironmentFailureKind::Unknown => Vec::new(),
    };
    let invalid = evidence.command_label.trim().is_empty()
        || (evidence.exit_code == Some(0) && evidence.stderr.trim().is_empty());
    EnvironmentDiagnosisIR {
        schema: REPOSITORY_CHANGE_EXPERIENCE_SCHEMA.to_string(),
        disposition: if invalid {
            EnvironmentDiagnosisDisposition::InvalidEvidence
        } else if kind == EnvironmentFailureKind::Unknown {
            EnvironmentDiagnosisDisposition::NeedsProbe
        } else {
            EnvironmentDiagnosisDisposition::Classified
        },
        kind,
        evidence_sha256: sha256(combined.as_bytes()),
        probes,
        mutation_authorized: false,
        automatic_install_authorized: false,
        network_reads: 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionPerturbation {
    HashSeed,
    RandomSeed,
    TestOrder,
    ThreadCount,
    TimeOffset,
    CpuLoad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NondeterminismCause {
    Ordering,
    Concurrency,
    ClockOrTiming,
    Randomness,
    SharedState,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NondeterminismDisposition {
    Confirmed,
    Deterministic,
    InsufficientEvidence,
    UnresolvedCause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NondeterminismRepairConstraint {
    CanonicalizeOrdering,
    SynchronizeSharedAccess,
    InjectControllableClock,
    SeedRandomGenerator,
    ResetSharedStatePerTest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatedRunObservationIR {
    pub attempt: usize,
    pub outcome_sha256: String,
    pub perturbations: BTreeSet<ExecutionPerturbation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NondeterminismAnalysisIR {
    pub schema: String,
    pub disposition: NondeterminismDisposition,
    pub cause: NondeterminismCause,
    pub attempts: usize,
    pub distinct_outcomes: usize,
    pub exercised_perturbations: BTreeSet<ExecutionPerturbation>,
    pub repair_constraints: Vec<NondeterminismRepairConstraint>,
    pub source_mutation_authorized: bool,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

/// Require repeated, perturbed observations before classifying a defect as
/// nondeterministic. A diagnostic hint selects constraints but cannot by
/// itself establish nondeterminism.
pub fn analyze_nondeterministic_failure(
    observations: &[RepeatedRunObservationIR],
    diagnostic_hint: &str,
) -> NondeterminismAnalysisIR {
    let outcomes = observations
        .iter()
        .map(|observation| observation.outcome_sha256.clone())
        .collect::<BTreeSet<_>>();
    let perturbations = observations
        .iter()
        .flat_map(|observation| observation.perturbations.iter().copied())
        .collect::<BTreeSet<_>>();
    let lower = diagnostic_hint.to_ascii_lowercase();
    let cause = if contains_any(
        &lower,
        &[
            "unordered",
            "map iteration",
            "unstable order",
            "정렬",
            "순서",
        ],
    ) && perturbations.iter().any(|value| {
        matches!(
            value,
            ExecutionPerturbation::HashSeed | ExecutionPerturbation::TestOrder
        )
    }) {
        NondeterminismCause::Ordering
    } else if contains_any(&lower, &["race", "thread", "concurrent", "경쟁", "동시성"])
        && perturbations.iter().any(|value| {
            matches!(
                value,
                ExecutionPerturbation::ThreadCount | ExecutionPerturbation::CpuLoad
            )
        })
    {
        NondeterminismCause::Concurrency
    } else if contains_any(&lower, &["clock", "timing", "timeout", "시간", "시계"])
        && perturbations.iter().any(|value| {
            matches!(
                value,
                ExecutionPerturbation::TimeOffset | ExecutionPerturbation::CpuLoad
            )
        })
    {
        NondeterminismCause::ClockOrTiming
    } else if contains_any(&lower, &["random", "rng", "seed", "무작위", "난수"])
        && perturbations.contains(&ExecutionPerturbation::RandomSeed)
    {
        NondeterminismCause::Randomness
    } else if contains_any(
        &lower,
        &[
            "shared state",
            "global state",
            "cache leak",
            "공유 상태",
            "전역 상태",
        ],
    ) && perturbations.contains(&ExecutionPerturbation::TestOrder)
    {
        NondeterminismCause::SharedState
    } else {
        NondeterminismCause::Unknown
    };
    let disposition = if observations.len() < 5 || perturbations.is_empty() {
        NondeterminismDisposition::InsufficientEvidence
    } else if outcomes.len() <= 1 {
        NondeterminismDisposition::Deterministic
    } else if cause == NondeterminismCause::Unknown {
        NondeterminismDisposition::UnresolvedCause
    } else {
        NondeterminismDisposition::Confirmed
    };
    let repair_constraints = if disposition == NondeterminismDisposition::Confirmed {
        match cause {
            NondeterminismCause::Ordering => {
                vec![NondeterminismRepairConstraint::CanonicalizeOrdering]
            }
            NondeterminismCause::Concurrency => {
                vec![NondeterminismRepairConstraint::SynchronizeSharedAccess]
            }
            NondeterminismCause::ClockOrTiming => {
                vec![NondeterminismRepairConstraint::InjectControllableClock]
            }
            NondeterminismCause::Randomness => {
                vec![NondeterminismRepairConstraint::SeedRandomGenerator]
            }
            NondeterminismCause::SharedState => {
                vec![NondeterminismRepairConstraint::ResetSharedStatePerTest]
            }
            NondeterminismCause::Unknown => Vec::new(),
        }
    } else {
        Vec::new()
    };
    NondeterminismAnalysisIR {
        schema: REPOSITORY_CHANGE_EXPERIENCE_SCHEMA.to_string(),
        disposition,
        cause,
        attempts: observations.len(),
        distinct_outcomes: outcomes.len(),
        exercised_perturbations: perturbations,
        repair_constraints,
        source_mutation_authorized: false,
        external_llm_calls: 0,
        network_reads: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn native_workspace(label: &str, hash: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "b-core-migration-{label}-{}-{}",
            std::process::id(),
            &hash[..16]
        ))
    }

    #[test]
    fn javascript_migration_updates_three_files_and_preserves_legacy_export() {
        let receipt = migrate_repository_api(&ApiMigrationRequestIR {
            language: CrossLanguage::JavaScript,
            files: vec![
                RepositorySourceFileIR {
                    relative_path: PathBuf::from("src/api.mjs"),
                    source: "// compute must remain documented\nexport function compute(left, right) { return left + right; }\nconst label = 'compute';\n".to_string(),
                },
                RepositorySourceFileIR {
                    relative_path: PathBuf::from("src/service.mjs"),
                    source: "import { compute } from './api.mjs';\nexport function service(a, b) { return compute(a, b); }\n".to_string(),
                },
                RepositorySourceFileIR {
                    relative_path: PathBuf::from("test/service.test.mjs"),
                    source: "import { service } from '../src/service.mjs';\nexport function regression() { return service(2, 3); }\n".to_string(),
                },
            ],
            old_symbol: "compute".to_string(),
            new_symbol: "combine".to_string(),
            preserve_public_api: true,
        })
        .unwrap();
        assert_eq!(receipt.definition_count, 1);
        assert_eq!(receipt.compatibility_shims, 1);
        assert!(receipt.callsite_and_import_replacements >= 2);
        let owner = &receipt.migrated_files[0].candidate_source;
        assert!(owner.contains("export function combine"));
        assert!(owner.contains("export const compute = combine;"));
        assert!(owner.contains("// compute must remain documented"));
        assert!(owner.contains("const label = 'compute'"));
        assert_eq!(receipt.comments_or_strings_rewritten, 0);

        let node = PathBuf::from(r"C:\Program Files\nodejs\node.exe");
        if node.is_file() {
            let workspace = native_workspace("javascript", &receipt.candidate_manifest_sha256);
            if workspace.exists() {
                fs::remove_dir_all(&workspace).unwrap();
            }
            for file in &receipt.migrated_files {
                let path = workspace.join(&file.relative_path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, &file.candidate_source).unwrap();
            }
            let harness = workspace.join("legacy.mjs");
            fs::write(
                &harness,
                "import { compute, combine } from './src/api.mjs';\nimport { service } from './src/service.mjs';\nimport { regression } from './test/service.test.mjs';\nif (compute(2,3) !== 5 || combine(3,4) !== 7 || service(4,5) !== 9 || regression() !== 5) throw new Error('migration mismatch');\nconsole.log('PASS:API_MIGRATION');\n",
            )
            .unwrap();
            let output = Command::new(node).arg(harness).output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("PASS:API_MIGRATION"));
            fs::remove_dir_all(workspace).unwrap();
        }
    }

    #[test]
    fn go_migration_emits_typed_forwarding_wrapper() {
        let receipt = migrate_repository_api(&ApiMigrationRequestIR {
            language: CrossLanguage::Go,
            files: vec![
                RepositorySourceFileIR {
                    relative_path: PathBuf::from("api.go"),
                    source: "package main\n\nfunc compute(left int64, right int64) int64 { return left + right }\n".to_string(),
                },
                RepositorySourceFileIR {
                    relative_path: PathBuf::from("service.go"),
                    source: "package main\n\nfunc service() int64 { return compute(2, 3) }\n".to_string(),
                },
            ],
            old_symbol: "compute".to_string(),
            new_symbol: "combine".to_string(),
            preserve_public_api: true,
        })
        .unwrap();
        let owner = &receipt.migrated_files[0].candidate_source;
        assert!(owner.contains("func combine(left int64, right int64) int64"));
        assert!(owner.contains("func compute(left int64, right int64) int64"));
        assert!(owner.contains("return combine(left, right)"));

        let go = PathBuf::from(r"C:\Program Files\Go\bin\go.exe");
        if go.is_file() {
            let workspace = native_workspace("go", &receipt.candidate_manifest_sha256);
            if workspace.exists() {
                fs::remove_dir_all(&workspace).unwrap();
            }
            fs::create_dir_all(&workspace).unwrap();
            for file in &receipt.migrated_files {
                fs::write(workspace.join(&file.relative_path), &file.candidate_source).unwrap();
            }
            fs::write(
                workspace.join("main.go"),
                "package main\n\nfunc main() { if compute(2,3) != 5 || combine(3,4) != 7 || service() != 5 { panic(\"migration mismatch\") }; println(\"PASS:API_MIGRATION\") }\n",
            )
            .unwrap();
            let output = Command::new(go)
                .args(["run", "api.go", "service.go", "main.go"])
                .current_dir(&workspace)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(combined.contains("PASS:API_MIGRATION"));
            fs::remove_dir_all(workspace).unwrap();
        }
    }

    #[test]
    fn environment_failures_map_to_read_only_minimal_probes() {
        let cases = [
            (
                "go test",
                "go: go.mod requires go 1.26",
                EnvironmentFailureKind::ToolchainVersionDrift,
                EnvironmentProbeKind::ReadToolVersion,
            ),
            (
                "npm ci",
                "npm error frozen lockfile is out of date",
                EnvironmentFailureKind::DependencyLockMismatch,
                EnvironmentProbeKind::CheckLockfileConsistency,
            ),
            (
                "node test.mjs",
                "Error: Cannot find module './adapter.js'",
                EnvironmentFailureKind::ModuleOrPathResolution,
                EnvironmentProbeKind::ResolveModulePath,
            ),
            (
                "cargo test",
                "environment variable DATABASE_URL not set in the environment",
                EnvironmentFailureKind::MissingEnvironmentVariable,
                EnvironmentProbeKind::CheckVariablePresence,
            ),
        ];
        for (command, stderr, expected, probe) in cases {
            let diagnosis = diagnose_environment_failure(&EnvironmentFailureEvidenceIR {
                command_label: command.to_string(),
                exit_code: Some(1),
                stdout: String::new(),
                stderr: stderr.to_string(),
                repeated_attempts: 1,
            });
            assert_eq!(diagnosis.kind, expected);
            assert!(diagnosis.probes.contains(&probe));
            assert!(!diagnosis.mutation_authorized);
            assert!(!diagnosis.automatic_install_authorized);
        }
    }

    fn observations(
        outcomes: &[&str],
        perturbation: ExecutionPerturbation,
    ) -> Vec<RepeatedRunObservationIR> {
        outcomes
            .iter()
            .enumerate()
            .map(|(attempt, outcome)| RepeatedRunObservationIR {
                attempt,
                outcome_sha256: sha256(outcome.as_bytes()),
                perturbations: BTreeSet::from([perturbation]),
            })
            .collect()
    }

    #[test]
    fn nondeterminism_requires_repetition_variation_and_relevant_perturbation() {
        let ordering = analyze_nondeterministic_failure(
            &observations(
                &["a,b", "b,a", "a,b", "b,a", "a,b", "b,a"],
                ExecutionPerturbation::HashSeed,
            ),
            "unordered map iteration changes output order",
        );
        assert_eq!(ordering.disposition, NondeterminismDisposition::Confirmed);
        assert_eq!(ordering.cause, NondeterminismCause::Ordering);
        assert_eq!(
            ordering.repair_constraints,
            vec![NondeterminismRepairConstraint::CanonicalizeOrdering]
        );

        let deterministic = analyze_nondeterministic_failure(
            &observations(
                &["same", "same", "same", "same", "same"],
                ExecutionPerturbation::ThreadCount,
            ),
            "possible race",
        );
        assert_eq!(
            deterministic.disposition,
            NondeterminismDisposition::Deterministic
        );

        let insufficient = analyze_nondeterministic_failure(
            &observations(&["a", "b"], ExecutionPerturbation::RandomSeed),
            "random seed",
        );
        assert_eq!(
            insufficient.disposition,
            NondeterminismDisposition::InsufficientEvidence
        );
    }
}
