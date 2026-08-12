//! Autonomous Rust compiler/clippy observation and repair-target construction.
//!
//! Compiler diagnostics are treated as public observations, not as authority.
//! A suggested replacement becomes only a bounded candidate target. The
//! structural repair engine must replay it and the normal source mutation gate
//! must still compile it and run public regression observations.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::autonomous_source_mutation::runtime_core_feature_available;
use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{synthesize_structural_repair, StructuralRepairProgram};

const CACHE_SCHEMA: &str = "B_CORE_COMPILER_DIAGNOSTIC_CACHE_2";
const MAX_DIAGNOSTIC_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CACHED_SOURCE_STATES: usize = 2;
const MAX_SUGGESTIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerGuidedRepairPolicy<'a> {
    pub source_root: &'a Path,
    pub cargo_executable: &'a Path,
    pub build_target_dir: &'a Path,
    pub state_dir: &'a Path,
    pub timeout_ms: u64,
    pub max_candidate_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CompilerSuggestion {
    pub level: String,
    pub diagnostic_code: String,
    pub message: String,
    pub file_name: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub replacement: String,
    pub applicability: String,
    pub primary: bool,
    pub observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerGuidedRepairCandidate {
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_source: String,
    pub candidate_sha256: String,
    pub transformation: String,
    pub solution_strategy: String,
    pub consequence_predictions: Vec<String>,
    pub predicted_value: u16,
    pub structural_repair_program: StructuralRepairProgram,
    pub public_observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DiagnosticCache {
    schema: String,
    source_fingerprint: String,
    check_success: bool,
    suggestions: Vec<CompilerSuggestion>,
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

fn source_inputs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("COMPILER_REPAIR_SCAN_DIR:{}:{error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("COMPILER_REPAIR_SCAN_ENTRY:{error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry
                .file_type()
                .map_err(|error| format!("COMPILER_REPAIR_SCAN_TYPE:{error}"))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() && !excluded_directory(&path) {
                pending.push(path);
            } else if file_type.is_file()
                && (path.extension().and_then(OsStr::to_str) == Some("rs")
                    || path.file_name().and_then(OsStr::to_str) == Some("Cargo.toml")
                    || path.file_name().and_then(OsStr::to_str) == Some("Cargo.lock"))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn source_fingerprint(root: &Path) -> Result<String, String> {
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("COMPILER_REPAIR_ROOT_CANONICALIZE:{error}"))?;
    let mut records = Vec::new();
    for path in source_inputs(&canonical_root)? {
        let relative = path
            .strip_prefix(&canonical_root)
            .map_err(|_| "COMPILER_REPAIR_SOURCE_OUTSIDE_ROOT".to_string())?;
        let bytes = fs::read(&path)
            .map_err(|error| format!("COMPILER_REPAIR_SOURCE_READ:{}:{error}", path.display()))?;
        records.push(format!(
            "{}:{}:{}",
            relative.to_string_lossy().replace('\\', "/"),
            bytes.len(),
            sha256(&bytes)
        ));
    }
    Ok(sha256(records.join("\n").as_bytes()))
}

fn cache_root(state_dir: &Path) -> PathBuf {
    state_dir.join("compiler_diagnostic_cache")
}

fn write_new_cache(path: &Path, cache: &DiagnosticCache) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(cache)
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_SERIALIZE:{error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_CREATE:{error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_WRITE:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_SYNC:{error}"))
}

fn cleanup_old_caches(root: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(root)
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_LIST:{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("COMPILER_REPAIR_CACHE_ENTRY:{error}"))?
        .into_iter()
        .filter(|entry| entry.path().extension().and_then(OsStr::to_str) == Some("json"))
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, entry.path())
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|(modified, path)| (*modified, path.clone()));
    let remove_count = entries.len().saturating_sub(MAX_CACHED_SOURCE_STATES);
    for (_, path) in entries.into_iter().take(remove_count) {
        fs::remove_file(&path)
            .map_err(|error| format!("COMPILER_REPAIR_CACHE_CLEANUP:{}:{error}", path.display()))?;
        for log in [
            path.with_extension("check.log"),
            path.with_extension("clippy.log"),
        ] {
            if log.exists() {
                fs::remove_file(&log).map_err(|error| {
                    format!("COMPILER_REPAIR_LOG_CLEANUP:{}:{error}", log.display())
                })?;
            }
        }
    }
    Ok(())
}

fn run_cargo_observation(
    policy: &CompilerGuidedRepairPolicy<'_>,
    args: &[&str],
    log_path: &Path,
) -> Result<(bool, Vec<u8>), String> {
    let output = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)
        .map_err(|error| format!("COMPILER_REPAIR_LOG_CREATE:{error}"))?;
    let errors = output
        .try_clone()
        .map_err(|error| format!("COMPILER_REPAIR_LOG_CLONE:{error}"))?;
    let mut child = Command::new(policy.cargo_executable)
        .args(args)
        .current_dir(policy.source_root)
        .env("CARGO_TARGET_DIR", policy.build_target_dir)
        .env("CARGO_INCREMENTAL", "1")
        .env("CARGO_NET_OFFLINE", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::from(output))
        .stderr(Stdio::from(errors))
        .spawn()
        .map_err(|error| format!("COMPILER_REPAIR_COMMAND_SPAWN:{error}"))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("COMPILER_REPAIR_COMMAND_WAIT:{error}"))?
        {
            let bytes =
                fs::read(log_path).map_err(|error| format!("COMPILER_REPAIR_LOG_READ:{error}"))?;
            return Ok((status.success(), bytes));
        }
        let log_size = fs::metadata(log_path).map(|value| value.len()).unwrap_or(0);
        if started.elapsed() >= Duration::from_millis(policy.timeout_ms)
            || log_size > MAX_DIAGNOSTIC_BYTES
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if log_size > MAX_DIAGNOSTIC_BYTES {
                "COMPILER_REPAIR_DIAGNOSTIC_BOUND_REACHED".to_string()
            } else {
                "COMPILER_REPAIR_OBSERVATION_TIMEOUT".to_string()
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn diagnostic_code(message: &JsonValue) -> String {
    message
        .get("code")
        .and_then(|value| value.get("code"))
        .and_then(JsonValue::as_str)
        .unwrap_or("RUSTC_UNCODED")
        .to_string()
}

fn collect_message_suggestions(
    message: &JsonValue,
    inherited_level: &str,
    inherited_code: &str,
    inherited_message: &str,
    output: &mut BTreeSet<CompilerSuggestion>,
) {
    let level = message
        .get("level")
        .and_then(JsonValue::as_str)
        .unwrap_or(inherited_level);
    let code = {
        let own = diagnostic_code(message);
        if own == "RUSTC_UNCODED" {
            inherited_code
        } else {
            &own
        }
        .to_string()
    };
    let text = message
        .get("message")
        .and_then(JsonValue::as_str)
        .unwrap_or(inherited_message);
    if let Some(spans) = message.get("spans").and_then(JsonValue::as_array) {
        for span in spans {
            let Some(replacement) = span
                .get("suggested_replacement")
                .and_then(JsonValue::as_str)
            else {
                continue;
            };
            let Some(file_name) = span.get("file_name").and_then(JsonValue::as_str) else {
                continue;
            };
            let Some(byte_start) = span.get("byte_start").and_then(JsonValue::as_u64) else {
                continue;
            };
            let Some(byte_end) = span.get("byte_end").and_then(JsonValue::as_u64) else {
                continue;
            };
            let applicability = span
                .get("suggestion_applicability")
                .and_then(JsonValue::as_str)
                .unwrap_or("Unspecified")
                .to_string();
            let primary = span
                .get("is_primary")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            let observation_sha256 = sha256(
                format!(
                    "{level}:{code}:{text}:{file_name}:{byte_start}:{byte_end}:{replacement}:{applicability}"
                )
                .as_bytes(),
            );
            output.insert(CompilerSuggestion {
                level: level.to_string(),
                diagnostic_code: code.clone(),
                message: text.to_string(),
                file_name: file_name.to_string(),
                byte_start: byte_start.min(usize::MAX as u64) as usize,
                byte_end: byte_end.min(usize::MAX as u64) as usize,
                replacement: replacement.to_string(),
                applicability,
                primary,
                observation_sha256,
            });
        }
    }
    if let Some(children) = message.get("children").and_then(JsonValue::as_array) {
        for child in children {
            collect_message_suggestions(child, level, &code, text, output);
        }
    }
}

fn parse_suggestions(output: &[u8]) -> Vec<CompilerSuggestion> {
    let mut suggestions = BTreeSet::new();
    for line in output.split(|byte| *byte == b'\n') {
        let Ok(value) = serde_json::from_slice::<JsonValue>(line) else {
            continue;
        };
        if value.get("reason").and_then(JsonValue::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let level = message
            .get("level")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let code = diagnostic_code(message);
        let text = message
            .get("message")
            .and_then(JsonValue::as_str)
            .unwrap_or("compiler observation");
        collect_message_suggestions(message, level, &code, text, &mut suggestions);
    }
    let mut suggestions = suggestions.into_iter().collect::<Vec<_>>();
    suggestions.sort_by_key(|suggestion| {
        (
            if suggestion.level == "error" { 0 } else { 1 },
            if suggestion.applicability == "MachineApplicable" {
                0
            } else {
                1
            },
            !suggestion.primary,
            suggestion.file_name.clone(),
            suggestion.byte_start,
        )
    });
    suggestions.truncate(MAX_SUGGESTIONS);
    suggestions
}

fn load_or_observe(
    policy: &CompilerGuidedRepairPolicy<'_>,
    fingerprint: &str,
) -> Result<DiagnosticCache, String> {
    let root = cache_root(policy.state_dir);
    fs::create_dir_all(&root).map_err(|error| format!("COMPILER_REPAIR_CACHE_DIR:{error}"))?;
    // Include the observation contract in the immutable cache key. Otherwise
    // an old-schema file at the same source fingerprint cannot be replaced by
    // `write_new_cache` and every lookup repeats the expensive observation.
    let cache_path = root.join(format!("{CACHE_SCHEMA}-{fingerprint}.json"));
    if cache_path.exists() {
        let bytes =
            fs::read(&cache_path).map_err(|error| format!("COMPILER_REPAIR_CACHE_READ:{error}"))?;
        let cache: DiagnosticCache = serde_json::from_slice(&bytes)
            .map_err(|error| format!("COMPILER_REPAIR_CACHE_PARSE:{error}"))?;
        if cache.schema == CACHE_SCHEMA && cache.source_fingerprint == fingerprint {
            return Ok(cache);
        }
    }

    // `cargo clippy` performs the same type-checking pass as `cargo check` and
    // emits rustc errors as compiler-message records before it emits lint
    // suggestions. Running both commands made every fresh source fingerprint
    // pay for two nearly identical workspace observations. One clippy pass
    // preserves both classes of evidence while keeping the cache and the
    // downstream compile/test/install gate authoritative.
    let clippy_log = cache_path.with_extension("clippy.log");
    let mut clippy_args = vec!["clippy", "-p", "semantic-reasoning", "--lib"];
    if runtime_core_feature_available(policy.source_root) {
        clippy_args.extend(["--no-default-features", "--features", "runtime-core"]);
    }
    clippy_args.extend(["--message-format=json", "--", "-W", "clippy::all"]);
    let (check_success, clippy_output) = run_cargo_observation(policy, &clippy_args, &clippy_log)?;
    let mut suggestions = parse_suggestions(&clippy_output);
    suggestions.sort();
    suggestions.dedup();
    suggestions.truncate(MAX_SUGGESTIONS);
    let cache = DiagnosticCache {
        schema: CACHE_SCHEMA.to_string(),
        source_fingerprint: fingerprint.to_string(),
        check_success,
        suggestions,
    };
    write_new_cache(&cache_path, &cache)?;
    cleanup_old_caches(&root)?;
    Ok(cache)
}

fn relative_source_path(root: &Path, diagnostic_path: &str) -> Option<PathBuf> {
    let candidate = PathBuf::from(diagnostic_path);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let canonical_root = fs::canonicalize(root).ok()?;
    let canonical = fs::canonicalize(joined).ok()?;
    if !canonical.starts_with(&canonical_root)
        || canonical.extension().and_then(OsStr::to_str) != Some("rs")
        || fs::symlink_metadata(&canonical)
            .ok()?
            .file_type()
            .is_symlink()
    {
        return None;
    }
    let relative = canonical.strip_prefix(canonical_root).ok()?.to_path_buf();
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(relative)
}

fn predicted_value(suggestion: &CompilerSuggestion) -> u16 {
    if suggestion.level == "error" {
        100
    } else if suggestion.applicability == "MachineApplicable" {
        75
    } else {
        60
    }
}

fn candidate_from_suggestion(
    policy: &CompilerGuidedRepairPolicy<'_>,
    suggestion: &CompilerSuggestion,
) -> Result<Option<CompilerGuidedRepairCandidate>, String> {
    let Some(relative_path) = relative_source_path(policy.source_root, &suggestion.file_name)
    else {
        return Ok(None);
    };
    let path = policy.source_root.join(&relative_path);
    let bytes = fs::read(&path)
        .map_err(|error| format!("COMPILER_REPAIR_CANDIDATE_READ:{}:{error}", path.display()))?;
    if bytes.len() as u64 > policy.max_candidate_bytes {
        return Ok(None);
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|_| "COMPILER_REPAIR_CANDIDATE_NOT_UTF8".to_string())?;
    if suggestion.byte_start > suggestion.byte_end
        || suggestion.byte_end > source.len()
        || !source.is_char_boundary(suggestion.byte_start)
        || !source.is_char_boundary(suggestion.byte_end)
    {
        return Ok(None);
    }
    let mut candidate_source = String::with_capacity(
        source.len() - (suggestion.byte_end - suggestion.byte_start) + suggestion.replacement.len(),
    );
    candidate_source.push_str(&source[..suggestion.byte_start]);
    candidate_source.push_str(&suggestion.replacement);
    candidate_source.push_str(&source[suggestion.byte_end..]);
    if candidate_source == source || candidate_source.len() as u64 > policy.max_candidate_bytes {
        return Ok(None);
    }
    let file_id = relative_path.to_string_lossy().replace('\\', "/");
    let Ok(structural_repair_program) =
        synthesize_structural_repair(&file_id, source, &candidate_source)
    else {
        return Ok(None);
    };
    let diagnostic_key = &suggestion.observation_sha256[..16];
    let transformation = format!(
        "COMPILER_OBSERVATION:{}:{diagnostic_key}",
        suggestion.diagnostic_code
    );
    let solution_strategy = format!(
        "COMPILER_SUGGESTION:{}:{}",
        suggestion.applicability,
        &sha256(suggestion.replacement.as_bytes())[..12]
    );
    Ok(Some(CompilerGuidedRepairCandidate {
        relative_path,
        predecessor_sha256: sha256(&bytes),
        candidate_sha256: sha256(candidate_source.as_bytes()),
        candidate_source,
        transformation,
        solution_strategy,
        consequence_predictions: vec![
            format!(
                "public compiler observation {} must disappear",
                suggestion.observation_sha256
            ),
            "AST/call/data-flow postconditions must replay exactly".to_string(),
            "source compile and public regression observations must pass".to_string(),
        ],
        predicted_value: predicted_value(suggestion),
        structural_repair_program,
        public_observation_sha256: suggestion.observation_sha256.clone(),
    }))
}

pub fn discover_compiler_guided_repairs(
    policy: &CompilerGuidedRepairPolicy<'_>,
) -> Result<Vec<CompilerGuidedRepairCandidate>, String> {
    let fingerprint = source_fingerprint(policy.source_root)?;
    let cache = load_or_observe(policy, &fingerprint)?;
    let mut candidates = Vec::new();
    for suggestion in &cache.suggestions {
        if let Some(candidate) = candidate_from_suggestion(policy, suggestion)? {
            candidates.push(candidate);
        }
    }
    candidates.sort_by_key(|candidate| {
        (
            std::cmp::Reverse(candidate.predicted_value),
            candidate.relative_path.clone(),
            candidate.transformation.clone(),
            candidate.solution_strategy.clone(),
        )
    });
    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_machine_applicable_compiler_observation() {
        let line = serde_json::json!({
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "message": "mismatched return type",
                "code": {"code": "E0308"},
                "spans": [{
                    "file_name": "src/lib.rs",
                    "byte_start": 20,
                    "byte_end": 21,
                    "is_primary": true,
                    "suggested_replacement": "",
                    "suggestion_applicability": "MachineApplicable"
                }],
                "children": []
            }
        });
        let suggestions = parse_suggestions(serde_json::to_string(&line).unwrap().as_bytes());
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].diagnostic_code, "E0308");
        assert_eq!(suggestions[0].replacement, "");
        assert_eq!(suggestions[0].applicability, "MachineApplicable");
    }

    #[test]
    fn compiler_span_becomes_structurally_replayable_target() {
        let root = std::env::temp_dir().join(format!(
            "b-core-compiler-guided-unit-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        let source = "pub fn value() -> i32 { 1; }\n";
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let semicolon = source.find(';').unwrap();
        let suggestion = CompilerSuggestion {
            level: "error".to_string(),
            diagnostic_code: "E0308".to_string(),
            message: "remove this semicolon".to_string(),
            file_name: "src/lib.rs".to_string(),
            byte_start: semicolon,
            byte_end: semicolon + 1,
            replacement: String::new(),
            applicability: "MachineApplicable".to_string(),
            primary: true,
            observation_sha256: sha256(b"observation"),
        };
        let cargo = PathBuf::from("cargo");
        let target = root.join("target");
        let state = root.join("state");
        let policy = CompilerGuidedRepairPolicy {
            source_root: &root,
            cargo_executable: &cargo,
            build_target_dir: &target,
            state_dir: &state,
            timeout_ms: 1_000,
            max_candidate_bytes: 1_024,
        };
        let candidate = candidate_from_suggestion(&policy, &suggestion)
            .unwrap()
            .expect("candidate");
        assert_eq!(candidate.candidate_source, "pub fn value() -> i32 { 1 }\n");
        assert_eq!(candidate.predicted_value, 100);
        assert_eq!(candidate.structural_repair_program.file_id, "src/lib.rs");
        fs::remove_dir_all(root).unwrap();
    }
}
