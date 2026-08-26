//! Evidence-bound TypeScript compiler suggestion repair.
//!
//! `tsc` diagnostics are observations, never source-mutation authority. Only
//! exact identifier suggestions from a small, explicit diagnostic family are
//! lowered to predecessor-bound candidates. A caller must still type-check,
//! execute public regressions, and authorize installation separately.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cross_language_synthesis::code_identifiers;
use crate::repository_change_experience::RepositorySourceFileIR;
use crate::self_repair_contract::sha256;

pub const TYPESCRIPT_COMPILER_REPAIR_SCHEMA: &str = "B_TYPESCRIPT_COMPILER_REPAIR_1";
pub const MAX_TYPESCRIPT_DIAGNOSTICS: usize = 128;
pub const MAX_TYPESCRIPT_DIAGNOSTIC_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypeScriptSuggestionFamilyIR {
    PropertyName,
    ObjectLiteralProperty,
    ExportedMember,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeScriptCompilerSuggestionIR {
    pub schema: String,
    pub diagnostic_code: String,
    pub family: TypeScriptSuggestionFamilyIR,
    pub relative_path: PathBuf,
    pub line: usize,
    pub utf16_column: usize,
    pub observed_identifier: String,
    pub suggested_identifier: String,
    pub message: String,
    pub observation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeScriptCompilerRepairCandidateIR {
    pub schema: String,
    pub diagnostic_code: String,
    pub family: TypeScriptSuggestionFamilyIR,
    pub relative_path: PathBuf,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub edit_start: usize,
    pub edit_end: usize,
    pub expected_identifier_sha256: String,
    pub replacement: String,
    pub candidate_source: String,
    pub observation_sha256: String,
    pub changed_identifiers: usize,
    pub compiler_suggestion_required: bool,
    pub source_mutation_authorized: bool,
    pub external_llm_calls: u64,
    pub network_reads: u64,
}

fn valid_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
}

fn capture_quoted_after<'a>(message: &'a str, marker: &str) -> Option<&'a str> {
    let remainder = message.split_once(marker)?.1;
    let end = remainder.find('\'')?;
    Some(&remainder[..end])
}

fn suggested_identifier(message: &str) -> Option<&str> {
    capture_quoted_after(message, "Did you mean to write '")
        .or_else(|| capture_quoted_after(message, "Did you mean '"))
}

fn suggestion_family(
    diagnostic_code: &str,
    message: &str,
) -> Option<(TypeScriptSuggestionFamilyIR, String, String)> {
    let (family, observed) = match diagnostic_code {
        "TS2551" => (
            TypeScriptSuggestionFamilyIR::PropertyName,
            capture_quoted_after(message, "Property '")?,
        ),
        "TS2561" => (
            TypeScriptSuggestionFamilyIR::ObjectLiteralProperty,
            capture_quoted_after(message, "but '")?,
        ),
        "TS2724" => (
            TypeScriptSuggestionFamilyIR::ExportedMember,
            capture_quoted_after(message, "has no exported member named '")
                .or_else(|| capture_quoted_after(message, "has no exported member '"))?,
        ),
        _ => return None,
    };
    let suggested = suggested_identifier(message)?;
    if !valid_identifier(observed) || !valid_identifier(suggested) || observed == suggested {
        return None;
    }
    Some((family, observed.to_string(), suggested.to_string()))
}

fn safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn diagnostic_relative_path(repository_root: &Path, raw: &str) -> Option<PathBuf> {
    let path = PathBuf::from(raw.trim());
    let relative = if path.is_absolute() {
        path.strip_prefix(repository_root).ok()?.to_path_buf()
    } else {
        path
    };
    safe_relative_path(&relative).then_some(relative)
}

/// Parse bounded, non-pretty `tsc` output. Unsupported diagnostics remain
/// observations outside this repair lane and are intentionally omitted.
pub fn parse_typescript_compiler_suggestions(
    output: &str,
    repository_root: &Path,
) -> Result<Vec<TypeScriptCompilerSuggestionIR>, String> {
    if output.len() > MAX_TYPESCRIPT_DIAGNOSTIC_BYTES {
        return Err("TYPESCRIPT_DIAGNOSTIC_BYTE_BOUND".to_string());
    }
    let mut suggestions = Vec::new();
    for line in output.lines() {
        let Some((location, diagnostic)) = line.rsplit_once("): error ") else {
            continue;
        };
        let Some(open) = location.rfind('(') else {
            continue;
        };
        let relative_path = match diagnostic_relative_path(repository_root, &location[..open]) {
            Some(path) => path,
            None => continue,
        };
        let Some((line_number, column)) = location[open + 1..].split_once(',') else {
            continue;
        };
        let Ok(line_number) = line_number.parse::<usize>() else {
            continue;
        };
        let Ok(column) = column.parse::<usize>() else {
            continue;
        };
        if line_number == 0 || column == 0 {
            continue;
        }
        let Some((diagnostic_code, message)) = diagnostic.split_once(": ") else {
            continue;
        };
        let Some((family, observed_identifier, suggested_identifier)) =
            suggestion_family(diagnostic_code, message)
        else {
            continue;
        };
        suggestions.push(TypeScriptCompilerSuggestionIR {
            schema: TYPESCRIPT_COMPILER_REPAIR_SCHEMA.to_string(),
            diagnostic_code: diagnostic_code.to_string(),
            family,
            relative_path,
            line: line_number,
            utf16_column: column,
            observed_identifier,
            suggested_identifier,
            message: message.to_string(),
            observation_sha256: sha256(line.as_bytes()),
        });
        if suggestions.len() > MAX_TYPESCRIPT_DIAGNOSTICS {
            return Err("TYPESCRIPT_DIAGNOSTIC_COUNT_BOUND".to_string());
        }
    }
    Ok(suggestions)
}

fn utf16_location_to_byte(source: &str, line: usize, column: usize) -> Option<usize> {
    let line_start = source
        .split_inclusive('\n')
        .take(line.checked_sub(1)?)
        .map(str::len)
        .sum::<usize>();
    let line_source = source.get(line_start..)?.split('\n').next()?;
    let target = column.checked_sub(1)?;
    let mut utf16_units = 0usize;
    for (byte_offset, character) in line_source.char_indices() {
        if utf16_units == target {
            return Some(line_start + byte_offset);
        }
        utf16_units = utf16_units.checked_add(character.len_utf16())?;
        if utf16_units > target {
            return None;
        }
    }
    (utf16_units == target).then_some(line_start + line_source.len())
}

/// Lower one exact compiler suggestion to a predecessor-bound candidate.
/// No repository file is written by this function.
pub fn synthesize_typescript_compiler_repair(
    files: &[RepositorySourceFileIR],
    suggestion: &TypeScriptCompilerSuggestionIR,
) -> Result<TypeScriptCompilerRepairCandidateIR, String> {
    if suggestion.schema != TYPESCRIPT_COMPILER_REPAIR_SCHEMA
        || !safe_relative_path(&suggestion.relative_path)
        || !valid_identifier(&suggestion.observed_identifier)
        || !valid_identifier(&suggestion.suggested_identifier)
        || suggestion.observed_identifier == suggestion.suggested_identifier
    {
        return Err("TYPESCRIPT_SUGGESTION_CONTRACT".to_string());
    }
    let matching_files = files
        .iter()
        .filter(|file| file.relative_path == suggestion.relative_path)
        .collect::<Vec<_>>();
    if matching_files.len() != 1 {
        return Err("TYPESCRIPT_SUGGESTION_FILE_CARDINALITY".to_string());
    }
    let file = matching_files[0];
    let start = utf16_location_to_byte(&file.source, suggestion.line, suggestion.utf16_column)
        .ok_or_else(|| "TYPESCRIPT_SUGGESTION_LOCATION".to_string())?;
    let end = start
        .checked_add(suggestion.observed_identifier.len())
        .ok_or_else(|| "TYPESCRIPT_SUGGESTION_RANGE_OVERFLOW".to_string())?;
    if file.source.get(start..end) != Some(suggestion.observed_identifier.as_str())
        || !code_identifiers(&file.source)
            .iter()
            .any(|(identifier, token_start, token_end)| {
                identifier == &suggestion.observed_identifier
                    && *token_start == start
                    && *token_end == end
            })
    {
        return Err("TYPESCRIPT_SUGGESTION_SOURCE_BINDING".to_string());
    }
    let mut candidate_source = file.source.clone();
    candidate_source.replace_range(start..end, &suggestion.suggested_identifier);
    Ok(TypeScriptCompilerRepairCandidateIR {
        schema: TYPESCRIPT_COMPILER_REPAIR_SCHEMA.to_string(),
        diagnostic_code: suggestion.diagnostic_code.clone(),
        family: suggestion.family,
        relative_path: suggestion.relative_path.clone(),
        predecessor_sha256: sha256(file.source.as_bytes()),
        candidate_sha256: sha256(candidate_source.as_bytes()),
        edit_start: start,
        edit_end: end,
        expected_identifier_sha256: sha256(suggestion.observed_identifier.as_bytes()),
        replacement: suggestion.suggested_identifier.clone(),
        candidate_source,
        observation_sha256: suggestion.observation_sha256.clone(),
        changed_identifiers: 1,
        compiler_suggestion_required: true,
        source_mutation_authorized: false,
        external_llm_calls: 0,
        network_reads: 0,
    })
}

/// Replay the complete lowering from the bound diagnostic and predecessor.
/// Any changed span, replacement, hash, or authority flag invalidates the
/// candidate before native validation.
pub fn validate_typescript_compiler_repair_candidate(
    files: &[RepositorySourceFileIR],
    suggestion: &TypeScriptCompilerSuggestionIR,
    candidate: &TypeScriptCompilerRepairCandidateIR,
) -> Result<(), String> {
    let expected = synthesize_typescript_compiler_repair(files, suggestion)?;
    if &expected != candidate {
        return Err("TYPESCRIPT_REPAIR_CANDIDATE_REPLAY_MISMATCH".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn tsc_path() -> Option<PathBuf> {
        let path = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\npm\tsc.cmd");
        path.is_file().then_some(path)
    }

    #[test]
    fn real_tsc_property_suggestion_becomes_one_bound_candidate() {
        let Some(tsc) = tsc_path() else {
            return;
        };
        let root = std::env::temp_dir().join(format!(
            "b-core-ts-repair-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let source = "interface User { displayName: string }\nexport function format(user: User): string { const 접두사 = '>'; return 접두사 + user.displayNmae; }\n";
        fs::write(root.join("format.ts"), source).unwrap();
        let output = Command::new(&tsc)
            .args([
                "--strict",
                "--noEmit",
                "--pretty",
                "false",
                "--target",
                "ES2022",
                "format.ts",
            ])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let suggestions = parse_typescript_compiler_suggestions(&diagnostics, &root).unwrap();
        assert_eq!(suggestions.len(), 1, "{diagnostics}");
        assert_eq!(suggestions[0].diagnostic_code, "TS2551");
        let candidate = synthesize_typescript_compiler_repair(
            &[RepositorySourceFileIR {
                relative_path: PathBuf::from("format.ts"),
                source: source.to_string(),
            }],
            &suggestions[0],
        )
        .unwrap();
        assert!(candidate.candidate_source.contains("user.displayName"));
        assert_eq!(candidate.changed_identifiers, 1);
        assert!(!candidate.source_mutation_authorized);
        validate_typescript_compiler_repair_candidate(
            &[RepositorySourceFileIR {
                relative_path: PathBuf::from("format.ts"),
                source: source.to_string(),
            }],
            &suggestions[0],
            &candidate,
        )
        .unwrap();
        let mut tampered = candidate.clone();
        tampered.replacement = "other".to_string();
        assert_eq!(
            validate_typescript_compiler_repair_candidate(
                &[RepositorySourceFileIR {
                    relative_path: PathBuf::from("format.ts"),
                    source: source.to_string(),
                }],
                &suggestions[0],
                &tampered,
            ),
            Err("TYPESCRIPT_REPAIR_CANDIDATE_REPLAY_MISMATCH".to_string())
        );
        fs::write(root.join("format.ts"), &candidate.candidate_source).unwrap();
        let repaired = Command::new(&tsc)
            .args([
                "--strict",
                "--noEmit",
                "--pretty",
                "false",
                "--target",
                "ES2022",
                "format.ts",
            ])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            repaired.status.success(),
            "{}{}",
            String::from_utf8_lossy(&repaired.stdout),
            String::from_utf8_lossy(&repaired.stderr)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unsupported_or_unbound_diagnostics_cannot_create_candidates() {
        let unsupported =
            "src/main.ts(1,8): error TS2339: Property 'missing' does not exist on type 'Value'.\n";
        assert!(
            parse_typescript_compiler_suggestions(unsupported, Path::new("."))
                .unwrap()
                .is_empty()
        );

        let supported = "src/main.ts(1,8): error TS2551: Property 'missng' does not exist on type 'Value'. Did you mean 'missing'?\n";
        let suggestion = parse_typescript_compiler_suggestions(supported, Path::new("."))
            .unwrap()
            .pop()
            .unwrap();
        let error = synthesize_typescript_compiler_repair(
            &[RepositorySourceFileIR {
                relative_path: PathBuf::from("src/main.ts"),
                source: "value.other".to_string(),
            }],
            &suggestion,
        )
        .unwrap_err();
        assert_eq!(error, "TYPESCRIPT_SUGGESTION_SOURCE_BINDING");
    }
}
