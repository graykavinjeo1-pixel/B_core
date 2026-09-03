use gpt_reference_evaluator::{
    candidate_batch_from_b_core, BCoreResponseBatchIR, BenchmarkInputSuiteIR, ReferenceSuiteIR,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let reference_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-candidate-dump <reference.json> <input.json> <b-core.json> <output.json>"
            .to_string()
    })?;
    let input_path = arguments
        .next()
        .ok_or_else(|| "INPUT_PATH_REQUIRED".to_string())?;
    let candidate_path = arguments
        .next()
        .ok_or_else(|| "B_CORE_RESPONSE_PATH_REQUIRED".to_string())?;
    let output_path = arguments
        .next()
        .ok_or_else(|| "OUTPUT_PATH_REQUIRED".to_string())?;
    if arguments.next().is_some() {
        return Err("TOO_MANY_ARGUMENTS".to_string());
    }

    let references: ReferenceSuiteIR = serde_json::from_slice(
        &fs::read(&reference_path).map_err(|error| format!("REFERENCE_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("REFERENCE_JSON_INVALID:{error}"))?;
    let input: BenchmarkInputSuiteIR = serde_json::from_slice(
        &fs::read(&input_path).map_err(|error| format!("INPUT_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("INPUT_JSON_INVALID:{error}"))?;
    let responses: BCoreResponseBatchIR = serde_json::from_slice(
        &fs::read(&candidate_path).map_err(|error| format!("B_CORE_READ_FAILED:{error}"))?,
    )
    .map_err(|error| format!("B_CORE_JSON_INVALID:{error}"))?;
    let candidates = candidate_batch_from_b_core(&references, &input, &responses)?;
    let payload = serde_json::to_string_pretty(&candidates)
        .map_err(|error| format!("CANDIDATE_SERIALIZATION_FAILED:{error}"))?;
    fs::write(output_path, format!("{payload}\n"))
        .map_err(|error| format!("OUTPUT_WRITE_FAILED:{error}"))
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
