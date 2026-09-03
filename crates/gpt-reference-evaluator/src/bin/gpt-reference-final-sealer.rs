use gpt_reference_evaluator::{
    seal_final_reference_suite, BenchmarkInputSuiteIR, ReferenceSuiteIR, ReferenceSurfaceRunIR,
};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "USAGE: gpt-reference-final-sealer <final-input.json> <annotation-draft.json> <gpt-run-1.json> <gpt-run-2.json> <gpt-run-3.json> <sealed-reference.json>";

fn read_json<T: serde::de::DeserializeOwned>(path: &str, kind: &str) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("{kind}_READ_FAILED:{path}:{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{kind}_JSON_INVALID:{path}:{error}"))
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 6 {
        return Err(USAGE.to_string());
    }
    let workspace = find_workspace_root(
        &env::current_dir().map_err(|error| format!("CURRENT_DIRECTORY_UNAVAILABLE:{error}"))?,
    )?;
    require_reports_output(&workspace, Path::new(&arguments[5]))?;
    let input: BenchmarkInputSuiteIR = read_json(&arguments[0], "FINAL_INPUT")?;
    let draft: ReferenceSuiteIR = read_json(&arguments[1], "ANNOTATION_DRAFT")?;
    let runs = [
        read_json::<ReferenceSurfaceRunIR>(&arguments[2], "GPT_SURFACE_RUN")?,
        read_json::<ReferenceSurfaceRunIR>(&arguments[3], "GPT_SURFACE_RUN")?,
        read_json::<ReferenceSurfaceRunIR>(&arguments[4], "GPT_SURFACE_RUN")?,
    ];
    let sealed = seal_final_reference_suite(&input, &draft, &runs)?;
    let payload = serde_json::to_string_pretty(&sealed)
        .map_err(|error| format!("SEALED_REFERENCE_SERIALIZATION_FAILED:{error}"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&arguments[5])
        .map_err(|error| format!("SEALED_REFERENCE_CREATE_FAILED:{}:{error}", arguments[5]))?;
    output
        .write_all(format!("{payload}\n").as_bytes())
        .map_err(|error| format!("SEALED_REFERENCE_WRITE_FAILED:{}:{error}", arguments[5]))?;
    Ok(())
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
    if output.exists() {
        return Err("SEALED_REFERENCE_OUTPUT_ALREADY_EXISTS".to_string());
    }
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
