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
use crate::bounded_parallel::map_ordered as parallel_map_ordered;
use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{
    apply_edit_atom, synthesize_structural_repair, ByteRange, SourceEditAtom,
    StructuralRepairProgram,
};

const CACHE_SCHEMA: &str = "B_CORE_COMPILER_DIAGNOSTIC_CACHE_3";
const MAX_DIAGNOSTIC_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CACHED_SOURCE_STATES: usize = 2;
const MAX_SUGGESTIONS: usize = 128;
const MAX_EAGER_FAMILY_FALLBACKS: usize = 2;

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
    let files = source_inputs(&canonical_root)?;
    let records = parallel_map_ordered(&files, "COMPILER_REPAIR_FINGERPRINT", |path| {
        let relative = path
            .strip_prefix(&canonical_root)
            .map_err(|_| "COMPILER_REPAIR_SOURCE_OUTSIDE_ROOT".to_string())?;
        let bytes = fs::read(path)
            .map_err(|error| format!("COMPILER_REPAIR_SOURCE_READ:{}:{error}", path.display()))?;
        Ok(format!(
            "{}:{}:{}",
            relative.to_string_lossy().replace('\\', "/"),
            bytes.len(),
            sha256(&bytes)
        ))
    })?;
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
    clippy_args.extend([
        "--message-format=json",
        "--",
        "-W",
        "clippy::all",
        // These performance-oriented lints are deliberately selected instead
        // of enabling the broad pedantic/nursery surfaces. They expose
        // executable allocation/numeric/collection improvements without
        // turning documentation or naming preferences into growth events.
        "-W",
        "clippy::redundant_clone",
        "-W",
        "clippy::suboptimal_flops",
        "-W",
        "clippy::needless_collect",
    ]);
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

fn matching_closing_parenthesis(fragment: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in fragment[open..].char_indices() {
        match character {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Clippy marks `manual_clamp` suggestions as `MaybeIncorrect` because `clamp`
/// panics when its bounds are reversed while the original method chain does
/// not.  We promote only the typed literal-bound subset whose ordering can be
/// proved before execution.  Identifiers and literal values may vary; no
/// repository-specific source template is selected.
fn is_typed_manual_clamp_lowering(
    suggestion: &CompilerSuggestion,
    replaced_source: &str,
    source: &str,
) -> bool {
    if suggestion.diagnostic_code != "clippy::manual_clamp"
        || suggestion.applicability != "MaybeIncorrect"
        || !replaced_source.starts_with("max(")
    {
        return false;
    }
    let Some(close) = matching_closing_parenthesis(replaced_source, 3) else {
        return false;
    };
    if &replaced_source[close + 1..] != ".min(" {
        return false;
    }
    let lower_bound = &replaced_source[4..close];
    let Some(lower) = syn::parse_str::<syn::LitInt>(lower_bound.trim())
        .ok()
        .and_then(|literal| literal.base10_parse::<u128>().ok())
    else {
        return false;
    };
    let suffix = &source[suggestion.byte_end..];
    let Some(upper_close) = matching_closing_parenthesis(&format!("({suffix}"), 0) else {
        return false;
    };
    let upper_bound = &suffix[..upper_close.saturating_sub(1)];
    let Some(upper) = syn::parse_str::<syn::LitInt>(upper_bound.trim())
        .ok()
        .and_then(|literal| literal.base10_parse::<u128>().ok())
    else {
        return false;
    };
    lower <= upper && suggestion.replacement == format!("clamp({lower_bound}, ")
}

fn has_unresolved_placeholder(replacement: &str) -> bool {
    replacement.contains("/*")
        || replacement.contains("*/")
        || replacement.contains("<placeholder>")
        || replacement.contains("...")
}

fn suggestion_is_executable(
    suggestion: &CompilerSuggestion,
    replaced_source: &str,
    source: &str,
) -> bool {
    if has_unresolved_placeholder(&suggestion.replacement) {
        return false;
    }
    match suggestion.applicability.as_str() {
        "MachineApplicable" => {
            // An empty replacement is a valid deletion only when the
            // diagnostic selected an actual source range.
            !suggestion.replacement.is_empty() || suggestion.byte_start < suggestion.byte_end
        }
        "MaybeIncorrect" => is_typed_manual_clamp_lowering(suggestion, replaced_source, source),
        _ => false,
    }
}

fn trim_touched_line_trailing_whitespace(
    candidate: &str,
    edit_start: usize,
    replacement_len: usize,
) -> String {
    let bounded_start = edit_start.min(candidate.len());
    let line_start = candidate[..bounded_start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let touched_end = bounded_start
        .saturating_add(replacement_len)
        .min(candidate.len());
    let line_end = candidate[touched_end..]
        .find('\n')
        .map_or(candidate.len(), |offset| touched_end + offset);
    let body = &candidate[line_start..line_end];
    let trimmed = body.trim_end_matches([' ', '\t']);
    if trimmed.len() == body.len() {
        return candidate.to_string();
    }
    let mut normalized = String::with_capacity(candidate.len() - (body.len() - trimmed.len()));
    normalized.push_str(&candidate[..line_start]);
    normalized.push_str(trimmed);
    normalized.push_str(&candidate[line_end..]);
    normalized
}

fn rustfmt_candidate_source(
    policy: &CompilerGuidedRepairPolicy<'_>,
    candidate: &str,
) -> Result<String, String> {
    let format_root = policy.state_dir.join("compiler_candidate_format");
    fs::create_dir_all(&format_root)
        .map_err(|error| format!("COMPILER_REPAIR_FORMAT_DIR:{error}"))?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| format!("COMPILER_REPAIR_FORMAT_CLOCK:{error}"))?
        .as_nanos();
    let candidate_path = format_root.join(format!(
        "{}-{}-{nonce}.rs",
        &sha256(candidate.as_bytes())[..16],
        std::process::id()
    ));
    let mut candidate_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&candidate_path)
        .map_err(|error| format!("COMPILER_REPAIR_FORMAT_CREATE:{error}"))?;
    candidate_file
        .write_all(candidate.as_bytes())
        .map_err(|error| format!("COMPILER_REPAIR_FORMAT_WRITE:{error}"))?;
    candidate_file
        .sync_all()
        .map_err(|error| format!("COMPILER_REPAIR_FORMAT_SYNC:{error}"))?;
    drop(candidate_file);

    let sibling_name = if cfg!(windows) {
        "rustfmt.exe"
    } else {
        "rustfmt"
    };
    let sibling = policy
        .cargo_executable
        .parent()
        .map(|parent| parent.join(sibling_name));
    let rustfmt = sibling
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(sibling_name));
    let mut child = match Command::new(&rustfmt)
        .args(["--edition", "2021"])
        .arg(&candidate_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_file(&candidate_path);
            let _ = fs::remove_dir(&format_root);
            return Err(format!("COMPILER_REPAIR_RUSTFMT_SPAWN:{error}"));
        }
    };
    let timeout = Duration::from_millis(policy.timeout_ms);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("COMPILER_REPAIR_RUSTFMT_WAIT:{error}"))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&candidate_path);
            let _ = fs::remove_dir(&format_root);
            return Err("COMPILER_REPAIR_RUSTFMT_TIMEOUT".to_string());
        }
        thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        let _ = fs::remove_file(&candidate_path);
        let _ = fs::remove_dir(&format_root);
        return Err(format!(
            "COMPILER_REPAIR_RUSTFMT_FAILED:{}",
            status.code().unwrap_or(-1)
        ));
    }
    let formatted = fs::read_to_string(&candidate_path);
    let _ = fs::remove_file(&candidate_path);
    let _ = fs::remove_dir(&format_root);
    formatted.map_err(|error| format!("COMPILER_REPAIR_FORMAT_READ:{error}"))
}

fn canonicalize_rust_candidate(
    policy: &CompilerGuidedRepairPolicy<'_>,
    predecessor: &str,
    candidate: &str,
) -> Result<String, String> {
    // Do not smuggle unrelated formatting cleanup into a compiler repair when
    // the predecessor was already noncanonical. Canonical production sources
    // receive the formatted postimage; synthetic or externally edited sources
    // keep only the compiler-selected semantic delta.
    if rustfmt_candidate_source(policy, predecessor)? != predecessor {
        return Ok(candidate.to_string());
    }
    rustfmt_candidate_source(policy, candidate)
}

fn compiler_suggestion_edit_atom(
    source: &str,
    suggestion: &CompilerSuggestion,
) -> Option<SourceEditAtom> {
    if suggestion.byte_start > suggestion.byte_end
        || suggestion.byte_end > source.len()
        || !source.is_char_boundary(suggestion.byte_start)
        || !source.is_char_boundary(suggestion.byte_end)
    {
        return None;
    }

    // Removing an ownership-only clone from a struct initializer can expose a
    // second, purely grammatical lint:
    //
    //     field: field.clone()  ->  field: field  ->  field
    //
    // Treat that sequence as one typed lowering rule.  The rule is independent
    // of the field name and fires only when both identifier roles are exactly
    // equal and the compiler-selected span is precisely `.clone()`.
    if suggestion.diagnostic_code == "clippy::redundant_clone"
        && suggestion.replacement.is_empty()
        && &source[suggestion.byte_start..suggestion.byte_end] == ".clone()"
    {
        let line_start = source[..suggestion.byte_start]
            .rfind('\n')
            .map_or(0, |offset| offset + 1);
        let line_end = source[suggestion.byte_end..]
            .find('\n')
            .map_or(source.len(), |offset| suggestion.byte_end + offset);
        let prefix = &source[line_start..suggestion.byte_start];
        let suffix = &source[suggestion.byte_end..line_end];
        let indentation_bytes = prefix.len() - prefix.trim_start().len();
        let initializer = prefix.trim();
        if let Some((field, value)) = initializer.split_once(':') {
            let field = field.trim();
            let value = value.trim();
            let identifier_is_typed = syn::parse_str::<syn::Ident>(field).is_ok();
            let suffix_is_field_terminator = suffix.trim_start().starts_with(',');
            if identifier_is_typed
                && value == field
                && !initializer[field.len() + 1..].contains(':')
                && suffix_is_field_terminator
            {
                let range = ByteRange {
                    start: line_start + indentation_bytes,
                    end: suggestion.byte_end,
                };
                return Some(SourceEditAtom::Replace {
                    range,
                    expected_sha256: sha256(&source.as_bytes()[range.start..range.end]),
                    replacement: field.to_string(),
                });
            }
        }
    }

    let range = ByteRange {
        start: suggestion.byte_start,
        end: suggestion.byte_end,
    };
    match (
        suggestion.byte_start == suggestion.byte_end,
        suggestion.replacement.is_empty(),
    ) {
        (true, false) => Some(SourceEditAtom::Insert {
            offset: suggestion.byte_start,
            content: suggestion.replacement.clone(),
        }),
        (false, true) => Some(SourceEditAtom::Delete {
            range,
            expected_sha256: sha256(&source.as_bytes()[range.start..range.end]),
        }),
        (false, false) => Some(SourceEditAtom::Replace {
            range,
            expected_sha256: sha256(&source.as_bytes()[range.start..range.end]),
            replacement: suggestion.replacement.clone(),
        }),
        (true, true) => None,
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
    let replaced_source = &source[suggestion.byte_start..suggestion.byte_end];
    if !suggestion_is_executable(suggestion, replaced_source, source) {
        return Ok(None);
    }
    let Some(edit) = compiler_suggestion_edit_atom(source, suggestion) else {
        return Ok(None);
    };
    let Ok(mut candidate_source) = apply_edit_atom(source, &edit) else {
        return Ok(None);
    };
    // Deletion suggestions such as `needless_else` can leave one space before
    // a newline.  Sending that mechanically correct candidate through an
    // expensive compile/test cycle only for `cargo fmt --check` to reject the
    // whitespace is avoidable static work.
    candidate_source = trim_touched_line_trailing_whitespace(
        &candidate_source,
        suggestion.byte_start,
        suggestion.replacement.len(),
    );
    candidate_source = canonicalize_rust_candidate(policy, source, &candidate_source)?;
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

fn family_candidate_from_suggestions(
    policy: &CompilerGuidedRepairPolicy<'_>,
    suggestions: &[&CompilerSuggestion],
) -> Result<Option<CompilerGuidedRepairCandidate>, String> {
    if suggestions.len() < 2 {
        return Ok(None);
    }
    if !matches!(
        suggestions[0].diagnostic_code.as_str(),
        "clippy::redundant_clone" | "clippy::suboptimal_flops" | "clippy::needless_collect"
    ) {
        return Ok(None);
    }
    // Validate the family directly from compiler spans. Calling
    // `candidate_from_suggestion` here used to parse the entire Rust file once
    // per member, and discovery parsed every member a second time when it made
    // fallback candidates. A 62-member family therefore paid for more than a
    // hundred identical predecessor AST analyses before selecting one patch.
    let Some(first_relative_path) =
        relative_source_path(policy.source_root, &suggestions[0].file_name)
    else {
        return Ok(None);
    };
    if suggestions.iter().any(|suggestion| {
        relative_source_path(policy.source_root, &suggestion.file_name).as_ref()
            != Some(&first_relative_path)
    }) {
        return Ok(None);
    }
    let path = policy.source_root.join(&first_relative_path);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("COMPILER_REPAIR_FAMILY_READ:{}:{error}", path.display()))?;
    if source.len() as u64 > policy.max_candidate_bytes
        || suggestions.iter().any(|suggestion| {
            suggestion.byte_start > suggestion.byte_end
                || suggestion.byte_end > source.len()
                || !source.is_char_boundary(suggestion.byte_start)
                || !source.is_char_boundary(suggestion.byte_end)
                || !suggestion_is_executable(
                    suggestion,
                    &source[suggestion.byte_start..suggestion.byte_end],
                    &source,
                )
        })
    {
        return Ok(None);
    }
    // Use compiler spans as the independent member preconditions. Individual
    // structural programs are line-oriented and two valid edits on one line
    // can therefore overlap when naively combined. Exact diagnostic spans keep
    // the family algebra minimal and still derive one AST/data-flow target for
    // the combined postimage below.
    let Some(edits) = suggestions
        .iter()
        .map(|suggestion| compiler_suggestion_edit_atom(&source, suggestion))
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(None);
    };
    let atomic_edit = SourceEditAtom::AtomicMultiEdit { edits };
    let Ok(raw_candidate_source) = apply_edit_atom(&source, &atomic_edit) else {
        return Ok(None);
    };
    let candidate_source = canonicalize_rust_candidate(policy, &source, &raw_candidate_source)?;
    if candidate_source == source || candidate_source.len() as u64 > policy.max_candidate_bytes {
        return Ok(None);
    }
    let file_id = first_relative_path.to_string_lossy().replace('\\', "/");
    let Ok(mut structural_repair_program) =
        synthesize_structural_repair(&file_id, &source, &candidate_source)
    else {
        return Ok(None);
    };
    if candidate_source == raw_candidate_source {
        structural_repair_program.edit = atomic_edit;
    }
    let family_sha256 = sha256(
        suggestions
            .iter()
            .map(|suggestion| suggestion.observation_sha256.as_str())
            .collect::<Vec<_>>()
            .join(":")
            .as_bytes(),
    );
    let diagnostic_code = &suggestions[0].diagnostic_code;
    let applicability = &suggestions[0].applicability;
    let family_size = suggestions.len();
    Ok(Some(CompilerGuidedRepairCandidate {
        relative_path: first_relative_path,
        predecessor_sha256: sha256(source.as_bytes()),
        candidate_sha256: sha256(candidate_source.as_bytes()),
        candidate_source,
        transformation: format!(
            "COMPILER_OBSERVATION_FAMILY:{diagnostic_code}:{}",
            &family_sha256[..16]
        ),
        solution_strategy: format!(
            "COMPILER_SUGGESTION_FAMILY:{applicability}:{family_size}:{}",
            &family_sha256[..12]
        ),
        consequence_predictions: vec![
            format!(
                "all {family_size} independent {diagnostic_code} observations must disappear atomically"
            ),
            "each member edit must retain its exact predecessor precondition".to_string(),
            "source compile and public regression observations must pass once for the family"
                .to_string(),
        ],
        predicted_value: predicted_value(suggestions[0])
            .saturating_add(family_size.min(25) as u16)
            .min(100),
        structural_repair_program,
        public_observation_sha256: family_sha256,
    }))
}

fn maximal_non_overlapping_family<'a>(
    suggestions: &[&'a CompilerSuggestion],
) -> Vec<&'a CompilerSuggestion> {
    let mut ordered = suggestions.to_vec();
    ordered.sort_by_key(|suggestion| {
        (
            suggestion.byte_start,
            suggestion.byte_end,
            suggestion.observation_sha256.clone(),
        )
    });
    let mut selected = Vec::new();
    let mut component = Vec::new();
    let mut component_end = 0usize;
    for suggestion in ordered {
        if component.is_empty() || suggestion.byte_start < component_end {
            component_end = component_end.max(suggestion.byte_end);
            component.push(suggestion);
            continue;
        }
        component.sort_by_key(|member| {
            (
                std::cmp::Reverse(member.byte_end.saturating_sub(member.byte_start)),
                member.byte_start,
                member.observation_sha256.clone(),
            )
        });
        selected.push(component[0]);
        component.clear();
        component_end = suggestion.byte_end;
        component.push(suggestion);
    }
    if !component.is_empty() {
        component.sort_by_key(|member| {
            (
                std::cmp::Reverse(member.byte_end.saturating_sub(member.byte_start)),
                member.byte_start,
                member.observation_sha256.clone(),
            )
        });
        selected.push(component[0]);
    }
    selected.sort_by_key(|suggestion| suggestion.byte_start);
    selected
}

pub fn discover_compiler_guided_repairs(
    policy: &CompilerGuidedRepairPolicy<'_>,
) -> Result<Vec<CompilerGuidedRepairCandidate>, String> {
    let fingerprint = source_fingerprint(policy.source_root)?;
    let cache = load_or_observe(policy, &fingerprint)?;
    let mut candidates = Vec::new();
    let mut families =
        std::collections::BTreeMap::<(String, String, String), Vec<&CompilerSuggestion>>::new();
    for suggestion in &cache.suggestions {
        if let Some(relative_path) = relative_source_path(policy.source_root, &suggestion.file_name)
        {
            families
                .entry((
                    relative_path.to_string_lossy().replace('\\', "/"),
                    suggestion.diagnostic_code.clone(),
                    suggestion.applicability.clone(),
                ))
                .or_default()
                .push(suggestion);
        }
    }
    let family_tasks = families.values().cloned().collect::<Vec<_>>();
    let family_results = parallel_map_ordered(
        &family_tasks,
        "COMPILER_REPAIR_FAMILY_SYNTHESIS",
        |suggestions| {
            let independent = maximal_non_overlapping_family(suggestions);
            let candidate = family_candidate_from_suggestions(policy, &independent)?;
            let member_ids = independent
                .iter()
                .map(|suggestion| suggestion.observation_sha256.clone())
                .collect::<Vec<_>>();
            let eager_ids = independent
                .iter()
                .take(MAX_EAGER_FAMILY_FALLBACKS)
                .map(|suggestion| suggestion.observation_sha256.clone())
                .collect::<Vec<_>>();
            Ok((candidate, member_ids, eager_ids))
        },
    )?;
    let mut eager_fallback_ids = BTreeSet::new();
    let mut family_member_ids = BTreeSet::new();
    for (candidate, member_ids, eager_ids) in family_results {
        if let Some(candidate) = candidate {
            candidates.push(candidate);
            family_member_ids.extend(member_ids);
            // Preserve a bounded counterexample-isolation path without eagerly
            // compiling an individual structural program for every family
            // member. After a fallback changes the source, the next compiler
            // observation reconstructs the remaining family on the new state.
            eager_fallback_ids.extend(eager_ids);
        }
    }
    let individual_tasks = cache
        .suggestions
        .iter()
        .filter(|suggestion| {
            !family_member_ids.contains(&suggestion.observation_sha256)
                || eager_fallback_ids.contains(&suggestion.observation_sha256)
        })
        .collect::<Vec<_>>();
    for candidate in parallel_map_ordered(
        &individual_tasks,
        "COMPILER_REPAIR_INDIVIDUAL_SYNTHESIS",
        |suggestion| candidate_from_suggestion(policy, suggestion),
    )?
    .into_iter()
    .flatten()
    {
        candidates.push(candidate);
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

    fn candidate_fixture(
        name: &str,
        source: &str,
        suggestion: CompilerSuggestion,
    ) -> Option<CompilerGuidedRepairCandidate> {
        let root = std::env::temp_dir().join(format!(
            "b-core-compiler-guided-{name}-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let cargo = PathBuf::from("cargo");
        let target = root.join("target");
        let state = root.join("state");
        let policy = CompilerGuidedRepairPolicy {
            source_root: &root,
            cargo_executable: &cargo,
            build_target_dir: &target,
            state_dir: &state,
            timeout_ms: 1_000,
            max_candidate_bytes: 4_096,
        };
        let result = candidate_from_suggestion(&policy, &suggestion).unwrap();
        fs::remove_dir_all(root).unwrap();
        result
    }

    #[test]
    fn typed_manual_clamp_lowering_accepts_fresh_identifiers_and_ordered_literal_bounds() {
        let source = "pub fn bound(reading: i64) -> i64 { reading.max(7).min(41) }\n";
        let start = source.find("max(7)").unwrap();
        let old = "max(7).min(";
        let suggestion = CompilerSuggestion {
            level: "warning".to_string(),
            diagnostic_code: "clippy::manual_clamp".to_string(),
            message: "clamp-like pattern without panicking".to_string(),
            file_name: "src/lib.rs".to_string(),
            byte_start: start,
            byte_end: start + old.len(),
            replacement: "clamp(7, ".to_string(),
            applicability: "MaybeIncorrect".to_string(),
            primary: true,
            observation_sha256: sha256(b"typed-clamp"),
        };
        let candidate = candidate_fixture("typed-clamp", source, suggestion).expect("candidate");
        assert_eq!(
            candidate.candidate_source,
            "pub fn bound(reading: i64) -> i64 { reading.clamp(7, 41) }\n"
        );
        assert!(candidate
            .solution_strategy
            .starts_with("COMPILER_SUGGESTION:MaybeIncorrect"));
    }

    #[test]
    fn typed_manual_clamp_lowering_rejects_unproved_or_reversed_bounds() {
        for (name, source, old, replacement) in [
            (
                "dynamic-bound",
                "pub fn bound(v: i64, floor: i64) -> i64 { v.max(floor).min(41) }\n",
                "max(floor).min(",
                "clamp(floor, ",
            ),
            (
                "reversed-bound",
                "pub fn bound(v: i64) -> i64 { v.max(41).min(7) }\n",
                "max(41).min(",
                "clamp(41, ",
            ),
        ] {
            let start = source.find(old).unwrap();
            let suggestion = CompilerSuggestion {
                level: "warning".to_string(),
                diagnostic_code: "clippy::manual_clamp".to_string(),
                message: "clamp-like pattern".to_string(),
                file_name: "src/lib.rs".to_string(),
                byte_start: start,
                byte_end: start + old.len(),
                replacement: replacement.to_string(),
                applicability: "MaybeIncorrect".to_string(),
                primary: true,
                observation_sha256: sha256(name.as_bytes()),
            };
            assert!(candidate_fixture(name, source, suggestion).is_none());
        }
    }

    #[test]
    fn untyped_maybe_incorrect_and_placeholder_suggestions_are_observations_only() {
        let source = "pub fn invoke() { target(); }\n";
        let target = source.find("target()").unwrap();
        for (name, applicability, replacement) in [
            ("untyped", "MaybeIncorrect", "other()"),
            ("placeholder", "HasPlaceholders", "target(/* usize */)"),
        ] {
            let suggestion = CompilerSuggestion {
                level: "error".to_string(),
                diagnostic_code: "E0061".to_string(),
                message: "missing argument".to_string(),
                file_name: "src/lib.rs".to_string(),
                byte_start: target,
                byte_end: target + "target()".len(),
                replacement: replacement.to_string(),
                applicability: applicability.to_string(),
                primary: true,
                observation_sha256: sha256(name.as_bytes()),
            };
            assert!(candidate_fixture(name, source, suggestion).is_none());
        }
    }

    #[test]
    fn machine_applicable_deletion_canonicalizes_touched_line_only() {
        let source =
            "pub fn choose(flag: bool) {\n    if flag {\n        return;\n    } else {\n    }\n}\n";
        let start = source.find("else {").unwrap();
        let deleted = "else {\n    }";
        let suggestion = CompilerSuggestion {
            level: "warning".to_string(),
            diagnostic_code: "clippy::needless_else".to_string(),
            message: "unneeded else block".to_string(),
            file_name: "src/lib.rs".to_string(),
            byte_start: start,
            byte_end: start + deleted.len(),
            replacement: String::new(),
            applicability: "MachineApplicable".to_string(),
            primary: true,
            observation_sha256: sha256(b"deletion"),
        };
        let candidate = candidate_fixture("deletion", source, suggestion).expect("candidate");
        assert_eq!(
            candidate.candidate_source,
            "pub fn choose(flag: bool) {\n    if flag {\n        return;\n    }\n}\n"
        );
    }

    #[test]
    fn same_compiler_family_becomes_one_atomic_multi_edit() {
        let root = std::env::temp_dir().join(format!(
            "b-core-compiler-guided-family-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        let source = "pub fn merge(left: String, right: String) -> String {\n    let first = left.clone();\n    let second = right.clone();\n    first + &second\n}\n";
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let mut suggestions = Vec::new();
        for (index, needle) in ["left.clone()", "right.clone()"].iter().enumerate() {
            let expression_start = source.find(needle).unwrap();
            let clone_start = expression_start + needle.find(".clone()").unwrap();
            suggestions.push(CompilerSuggestion {
                level: "warning".to_string(),
                diagnostic_code: "clippy::redundant_clone".to_string(),
                message: "redundant clone".to_string(),
                file_name: "src/lib.rs".to_string(),
                byte_start: clone_start,
                byte_end: clone_start + ".clone()".len(),
                replacement: String::new(),
                applicability: "MachineApplicable".to_string(),
                primary: true,
                observation_sha256: sha256(format!("clone-{index}").as_bytes()),
            });
        }
        let cargo = PathBuf::from("cargo");
        let target = root.join("target");
        let state = root.join("state");
        let policy = CompilerGuidedRepairPolicy {
            source_root: &root,
            cargo_executable: &cargo,
            build_target_dir: &target,
            state_dir: &state,
            timeout_ms: 1_000,
            max_candidate_bytes: 4_096,
        };
        let refs = suggestions.iter().collect::<Vec<_>>();
        let candidate = family_candidate_from_suggestions(&policy, &refs)
            .unwrap()
            .expect("family candidate");
        assert_eq!(
            candidate.candidate_source,
            "pub fn merge(left: String, right: String) -> String {\n    let first = left;\n    let second = right;\n    first + &second\n}\n"
        );
        assert!(candidate
            .transformation
            .starts_with("COMPILER_OBSERVATION_FAMILY:clippy::redundant_clone:"));
        assert!(matches!(
            candidate.structural_repair_program.edit,
            SourceEditAtom::AtomicMultiEdit { ref edits } if edits.len() == 2
        ));
        assert_eq!(candidate.predicted_value, 77);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn numeric_family_is_canonicalized_before_structural_lowering() {
        let root = std::env::temp_dir().join(format!(
            "b-core-compiler-guided-formatted-family-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        let cargo = PathBuf::from("cargo");
        let target = root.join("target");
        let state = root.join("state");
        let policy = CompilerGuidedRepairPolicy {
            source_root: &root,
            cargo_executable: &cargo,
            build_target_dir: &target,
            state_dir: &state,
            timeout_ms: 1_000,
            max_candidate_bytes: 16_384,
        };
        let raw_source = "pub fn retention(plasticity: f32, access: f32, importance: f32, average_activation: f32) -> f32 {\n    0.05 * plasticity + 0.20 * access + 0.45 * importance + 0.30 * average_activation\n}\n\npub fn score(reflex_bonus: f32, abstraction_level: f32, cue_overlap: f32, definition_match: f32) -> f32 {\n    0.10 * reflex_bonus + 0.10 * abstraction_level + 0.45 * cue_overlap + 0.20 * definition_match\n}\n";
        let source = rustfmt_candidate_source(&policy, raw_source).unwrap();
        fs::write(root.join("src/lib.rs"), &source).unwrap();
        let replacements = [
            (
                "0.05 * plasticity + 0.20 * access + 0.45 * importance + 0.30 * average_activation",
                "0.05f32.mul_add(plasticity, 0.20f32.mul_add(access, 0.45 * importance + 0.30 * average_activation))",
            ),
            (
                "0.10 * reflex_bonus + 0.10 * abstraction_level + 0.45 * cue_overlap + 0.20 * definition_match",
                "0.10f32.mul_add(reflex_bonus, 0.10f32.mul_add(abstraction_level, 0.45 * cue_overlap + 0.20 * definition_match))",
            ),
        ];
        let suggestions = replacements
            .iter()
            .enumerate()
            .map(|(index, (needle, replacement))| {
                let byte_start = source.find(needle).unwrap();
                CompilerSuggestion {
                    level: "warning".to_string(),
                    diagnostic_code: "clippy::suboptimal_flops".to_string(),
                    message: "consider using fused multiply-add".to_string(),
                    file_name: "src/lib.rs".to_string(),
                    byte_start,
                    byte_end: byte_start + needle.len(),
                    replacement: (*replacement).to_string(),
                    applicability: "MachineApplicable".to_string(),
                    primary: true,
                    observation_sha256: sha256(format!("numeric-{index}").as_bytes()),
                }
            })
            .collect::<Vec<_>>();
        let refs = suggestions.iter().collect::<Vec<_>>();
        let raw_edits = refs
            .iter()
            .map(|suggestion| compiler_suggestion_edit_atom(&source, suggestion).unwrap())
            .collect();
        let raw_candidate = apply_edit_atom(
            &source,
            &SourceEditAtom::AtomicMultiEdit { edits: raw_edits },
        )
        .unwrap();
        let candidate = family_candidate_from_suggestions(&policy, &refs)
            .unwrap()
            .expect("formatted family candidate");

        assert_ne!(candidate.candidate_source, raw_candidate);
        assert_eq!(
            rustfmt_candidate_source(&policy, &candidate.candidate_source).unwrap(),
            candidate.candidate_source
        );
        assert_eq!(
            apply_edit_atom(&source, &candidate.structural_repair_program.edit).unwrap(),
            candidate.candidate_source
        );
        assert!(!state.join("compiler_candidate_format").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn redundant_clone_family_lowers_matching_struct_field_roles_atomically() {
        let root = std::env::temp_dir().join(format!(
            "b-core-compiler-guided-field-lowering-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src")).unwrap();
        let source = "pub struct Pair { pub left: String, pub right: String }\n\
pub fn pair(left: String, right: String) -> Pair {\n\
    Pair {\n\
        left: left.clone(),\n\
        right: right.clone(),\n\
    }\n\
}\n";
        fs::write(root.join("src/lib.rs"), source).unwrap();
        let mut suggestions = Vec::new();
        for (index, needle) in ["left.clone()", "right.clone()"].iter().enumerate() {
            let expression_start = source.find(needle).unwrap();
            let clone_start = expression_start + needle.find(".clone()").unwrap();
            suggestions.push(CompilerSuggestion {
                level: "warning".to_string(),
                diagnostic_code: "clippy::redundant_clone".to_string(),
                message: "redundant clone".to_string(),
                file_name: "src/lib.rs".to_string(),
                byte_start: clone_start,
                byte_end: clone_start + ".clone()".len(),
                replacement: String::new(),
                applicability: "MachineApplicable".to_string(),
                primary: true,
                observation_sha256: sha256(format!("field-clone-{index}").as_bytes()),
            });
        }
        let cargo = PathBuf::from("cargo");
        let target = root.join("target");
        let state = root.join("state");
        let policy = CompilerGuidedRepairPolicy {
            source_root: &root,
            cargo_executable: &cargo,
            build_target_dir: &target,
            state_dir: &state,
            timeout_ms: 1_000,
            max_candidate_bytes: 4_096,
        };
        let refs = suggestions.iter().collect::<Vec<_>>();

        let candidate = family_candidate_from_suggestions(&policy, &refs)
            .unwrap()
            .expect("field-role family candidate");

        assert_eq!(
            candidate.candidate_source,
            "pub struct Pair { pub left: String, pub right: String }\n\
pub fn pair(left: String, right: String) -> Pair {\n\
    Pair {\n\
        left,\n\
        right,\n\
    }\n\
}\n"
        );
        assert!(matches!(
            candidate.structural_repair_program.edit,
            SourceEditAtom::AtomicMultiEdit { ref edits } if edits.len() == 2
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nested_numeric_alternatives_lower_to_one_outer_edit_per_independent_expression() {
        let make = |start: usize, end: usize, id: &str| CompilerSuggestion {
            level: "warning".to_string(),
            diagnostic_code: "clippy::suboptimal_flops".to_string(),
            message: "consider using fused multiply-add".to_string(),
            file_name: "src/lib.rs".to_string(),
            byte_start: start,
            byte_end: end,
            replacement: format!("fused_{id}()"),
            applicability: "MachineApplicable".to_string(),
            primary: true,
            observation_sha256: sha256(id.as_bytes()),
        };
        let suggestions = [
            make(10, 62, "outer-a"),
            make(11, 31, "inner-a1"),
            make(11, 47, "inner-a2"),
            make(90, 173, "inner-b"),
            make(90, 258, "outer-b"),
            make(300, 351, "independent-c"),
        ];
        let refs = suggestions.iter().collect::<Vec<_>>();

        let selected = maximal_non_overlapping_family(&refs);

        assert_eq!(selected.len(), 3);
        assert_eq!(
            selected
                .iter()
                .map(|suggestion| (suggestion.byte_start, suggestion.byte_end))
                .collect::<Vec<_>>(),
            vec![(10, 62), (90, 258), (300, 351)]
        );
    }
}
