use gpt_reference_evaluator::{
    BCoreResponseBatchIR, BCoreResponseTurnIR, BenchmarkInputSuiteIR, ReferenceSuiteIR,
    SuiteSplitIR, B_CORE_RESPONSE_BATCH_SCHEMA,
};
use semantic_core_adapters::CognitiveApi;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn run() -> Result<bool, String> {
    let mut arguments = env::args().skip(1);
    let input_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-runner <frozen-input-suite.json> <reports-output-batch.json> [sealed-reference-suite.json]"
            .to_string()
    })?;
    let output_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-runner <frozen-input-suite.json> <reports-output-batch.json> [sealed-reference-suite.json]"
            .to_string()
    })?;
    let reference_path = arguments.next();
    if arguments.next().is_some() {
        return Err("TOO_MANY_ARGUMENTS".to_string());
    }

    let workspace = find_workspace_root(
        &env::current_dir().map_err(|error| format!("CURRENT_DIRECTORY_UNAVAILABLE:{error}"))?,
    )?;
    require_reports_output(&workspace, Path::new(&output_path))?;
    let input_bytes = fs::read(&input_path)
        .map_err(|error| format!("INPUT_SUITE_READ_FAILED:{input_path}:{error}"))?;
    let input: BenchmarkInputSuiteIR = serde_json::from_slice(&input_bytes)
        .map_err(|error| format!("INPUT_SUITE_JSON_INVALID:{error}"))?;
    input.validate()?;
    if input.split == SuiteSplitIR::Final && reference_path.is_none() {
        return Err("FINAL_RUN_REQUIRES_SEALED_REFERENCE".to_string());
    }
    if let Some(reference_path) = reference_path {
        let reference_bytes = fs::read(&reference_path)
            .map_err(|error| format!("REFERENCE_READ_FAILED:{reference_path}:{error}"))?;
        let references: ReferenceSuiteIR = serde_json::from_slice(&reference_bytes)
            .map_err(|error| format!("REFERENCE_JSON_INVALID:{error}"))?;
        input.validate_against_references(&references)?;
    }

    let source_tree_sha256_before = source_tree_sha256(&workspace)?;
    let mut api = CognitiveApi::new_embedded()
        .map_err(|error| format!("COGNITIVE_API_INITIALIZATION_FAILED:{error:?}"))?;
    let mut input_turns = input.turns.iter().collect::<Vec<_>>();
    input_turns.sort_by(|left, right| {
        left.dialogue_id
            .cmp(&right.dialogue_id)
            .then_with(|| left.turn_index.cmp(&right.turn_index))
    });
    let mut responses = Vec::with_capacity(input_turns.len());
    for input_turn in input_turns {
        let request = input_turn.to_request();
        let response = api
            .process_conversation_turn(&request)
            .map_err(|error| format!("B_CORE_TURN_FAILED:{}:{error:?}", input_turn.response_id))?;
        if !response.validate_against(&request) {
            return Err(format!(
                "B_CORE_TURN_RETURNED_INVALID_IR:{}",
                input_turn.response_id
            ));
        }
        responses.push(BCoreResponseTurnIR {
            response_id: input_turn.response_id.clone(),
            request,
            response,
        });
    }
    let source_tree_sha256_after = source_tree_sha256(&workspace)?;
    let executable = env::current_exe()
        .map_err(|error| format!("RUNNER_EXECUTABLE_PATH_UNAVAILABLE:{error}"))?;
    let runner_executable_sha256 = file_sha256(&executable)?;
    let recursive_source_mutations =
        u64::from(source_tree_sha256_before != source_tree_sha256_after);
    let mut batch = BCoreResponseBatchIR {
        schema: B_CORE_RESPONSE_BATCH_SCHEMA.to_string(),
        suite_id: input.suite_id.clone(),
        input_suite_sha256: input.suite_payload_sha256.clone(),
        responses,
        source_tree_sha256_before,
        source_tree_sha256_after,
        runner_executable_sha256,
        recursive_source_mutations,
        batch_payload_sha256: String::new(),
    };
    batch.seal()?;
    batch.validate_against_input(&input)?;
    let payload = serde_json::to_string_pretty(&batch)
        .map_err(|error| format!("B_CORE_RESPONSE_BATCH_SERIALIZATION_FAILED:{error}"))?;
    fs::write(&output_path, format!("{payload}\n"))
        .map_err(|error| format!("B_CORE_RESPONSE_BATCH_WRITE_FAILED:{output_path}:{error}"))?;
    Ok(recursive_source_mutations == 0)
}

fn find_workspace_root(start: &Path) -> Result<PathBuf, String> {
    for candidate in start.ancestors() {
        let manifest = candidate.join("Cargo.toml");
        if manifest.is_file() {
            let text = fs::read_to_string(&manifest)
                .map_err(|error| format!("WORKSPACE_MANIFEST_READ_FAILED:{error}"))?;
            if text.contains("[workspace]") {
                return candidate
                    .canonicalize()
                    .map_err(|error| format!("WORKSPACE_CANONICALIZATION_FAILED:{error}"));
            }
        }
    }
    Err("WORKSPACE_ROOT_NOT_FOUND".to_string())
}

fn require_reports_output(workspace: &Path, output: &Path) -> Result<(), String> {
    let reports = workspace
        .join("reports")
        .canonicalize()
        .map_err(|error| format!("REPORTS_DIRECTORY_UNAVAILABLE:{error}"))?;
    let parent = output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .map_err(|error| format!("OUTPUT_PARENT_UNAVAILABLE:{error}"))?;
    if !parent.starts_with(reports) {
        return Err("OUTPUT_MUST_BE_INSIDE_WORKSPACE_REPORTS".to_string());
    }
    Ok(())
}

fn source_tree_sha256(workspace: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        let path = workspace.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    collect_files(&workspace.join("crates"), &mut files)?;
    collect_files(&workspace.join("scripts"), &mut files)?;
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        let relative = path
            .strip_prefix(workspace)
            .map_err(|error| format!("SOURCE_PATH_OUTSIDE_WORKSPACE:{error}"))?;
        digest.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        digest.update([0]);
        digest.update(
            fs::read(&path)
                .map_err(|error| format!("SOURCE_FILE_READ_FAILED:{}:{error}", path.display()))?,
        );
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)
        .map_err(|error| {
            format!(
                "SOURCE_DIRECTORY_READ_FAILED:{}:{error}",
                directory.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("SOURCE_DIRECTORY_ENTRY_FAILED:{error}"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry
            .file_type()
            .map_err(|error| format!("SOURCE_FILE_TYPE_FAILED:{error}"))?;
        if file_type.is_symlink() {
            return Err(format!(
                "SOURCE_TREE_SYMLINK_NOT_ALLOWED:{}",
                entry.path().display()
            ));
        }
        if file_type.is_dir() {
            collect_files(&entry.path(), files)?;
        } else if file_type.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("FILE_HASH_READ_FAILED:{}:{error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("SOURCE_TREE_MUTATED_DURING_BENCHMARK");
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
