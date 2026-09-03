use gpt_reference_evaluator::{
    sha256_text, BenchmarkInputSuiteIR, ReferenceSurfaceResponseIR, ReferenceSurfaceRunIR,
    REFERENCE_SURFACE_RUN_SCHEMA,
};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "USAGE: gpt-reference-surface-run-sealer <final-input.json> <raw-gpt-responses.json> <generation-run-id> <model-id> <generation-date> <system-prompt.txt> <generation-configuration.txt> <sealed-run.json>";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReferenceSurfaceBatchIR {
    responses: Vec<RawReferenceSurfaceIR>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReferenceSurfaceIR {
    response_id: String,
    surface: String,
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
        .map_err(|error| format!("REPORTS_DIRECTORY_INVALID:{error}"))?;
    let parent = output
        .parent()
        .ok_or_else(|| "SEALED_RUN_OUTPUT_PARENT_MISSING".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("SEALED_RUN_OUTPUT_PARENT_CREATE_FAILED:{error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("SEALED_RUN_OUTPUT_PARENT_INVALID:{error}"))?;
    if !parent.starts_with(&reports) {
        return Err("SEALED_RUN_OUTPUT_MUST_BE_UNDER_REPORTS".to_string());
    }
    if output.exists() {
        return Err("SEALED_RUN_OUTPUT_ALREADY_EXISTS".to_string());
    }
    Ok(())
}

fn execute(args: &[String]) -> Result<(), String> {
    if args.len() != 9 {
        return Err(USAGE.to_string());
    }
    let input_path = Path::new(&args[1]);
    let raw_path = Path::new(&args[2]);
    let generation_run_id = args[3].trim();
    let model_id = args[4].trim();
    let generation_date = args[5].trim();
    let system_prompt_path = Path::new(&args[6]);
    let generation_configuration_path = Path::new(&args[7]);
    let output_path = Path::new(&args[8]);
    if generation_run_id.is_empty() || model_id.is_empty() || generation_date.is_empty() {
        return Err("REFERENCE_SURFACE_RUN_PROVENANCE_EMPTY".to_string());
    }

    let input: BenchmarkInputSuiteIR = serde_json::from_str(
        &fs::read_to_string(input_path)
            .map_err(|error| format!("FINAL_INPUT_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("FINAL_INPUT_PARSE_FAILED:{error}"))?;
    input.validate()?;
    let raw: RawReferenceSurfaceBatchIR = serde_json::from_str(
        &fs::read_to_string(raw_path)
            .map_err(|error| format!("RAW_GPT_RESPONSES_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("RAW_GPT_RESPONSES_PARSE_FAILED:{error}"))?;
    let system_prompt = fs::read_to_string(system_prompt_path)
        .map_err(|error| format!("SYSTEM_PROMPT_READ_FAILED:{error}"))?;
    let generation_configuration = fs::read_to_string(generation_configuration_path)
        .map_err(|error| format!("GENERATION_CONFIGURATION_READ_FAILED:{error}"))?;

    let mut run = ReferenceSurfaceRunIR {
        schema: REFERENCE_SURFACE_RUN_SCHEMA.to_string(),
        suite_id: input.suite_id.clone(),
        input_suite_sha256: input.suite_payload_sha256.clone(),
        generation_run_id: generation_run_id.to_string(),
        model_id: model_id.to_string(),
        generation_date: generation_date.to_string(),
        system_prompt_sha256: sha256_text(&system_prompt),
        generation_configuration_sha256: sha256_text(&generation_configuration),
        b_core_output_consulted: false,
        responses: raw
            .responses
            .into_iter()
            .map(|response| ReferenceSurfaceResponseIR::new(response.response_id, response.surface))
            .collect(),
        run_payload_sha256: String::new(),
    };
    run.seal()?;
    run.validate_against_input(&input)?;

    let workspace = find_workspace_root(&env::current_dir().map_err(|error| error.to_string())?)?;
    require_reports_output(&workspace, output_path)?;
    fs::write(
        output_path,
        serde_json::to_string_pretty(&run)
            .map_err(|error| format!("SEALED_RUN_SERIALIZATION_FAILED:{error}"))?,
    )
    .map_err(|error| format!("SEALED_RUN_WRITE_FAILED:{error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    match execute(&env::args().collect::<Vec<_>>()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
