use gpt_reference_evaluator::{
    evaluate_b_core, BCoreResponseBatchIR, BenchmarkInputSuiteIR, ReferenceSuiteIR,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<bool, String> {
    let mut arguments = env::args().skip(1);
    let reference_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-evaluator <reference-suite.json> <input-suite.json> <b-core-response-batch.json> [report.json]"
            .to_string()
    })?;
    let input_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-evaluator <reference-suite.json> <input-suite.json> <b-core-response-batch.json> [report.json]"
            .to_string()
    })?;
    let candidate_path = arguments.next().ok_or_else(|| {
        "USAGE: gpt-reference-evaluator <reference-suite.json> <input-suite.json> <b-core-response-batch.json> [report.json]"
            .to_string()
    })?;
    let report_path = arguments.next();
    if arguments.next().is_some() {
        return Err("TOO_MANY_ARGUMENTS".to_string());
    }

    let reference_bytes = fs::read(&reference_path)
        .map_err(|error| format!("REFERENCE_READ_FAILED:{reference_path}:{error}"))?;
    let input_bytes = fs::read(&input_path)
        .map_err(|error| format!("INPUT_SUITE_READ_FAILED:{input_path}:{error}"))?;
    let candidate_bytes = fs::read(&candidate_path)
        .map_err(|error| format!("CANDIDATE_READ_FAILED:{candidate_path}:{error}"))?;
    let references: ReferenceSuiteIR = serde_json::from_slice(&reference_bytes)
        .map_err(|error| format!("REFERENCE_JSON_INVALID:{error}"))?;
    let input: BenchmarkInputSuiteIR = serde_json::from_slice(&input_bytes)
        .map_err(|error| format!("INPUT_SUITE_JSON_INVALID:{error}"))?;
    let candidates: BCoreResponseBatchIR = serde_json::from_slice(&candidate_bytes)
        .map_err(|error| format!("B_CORE_RESPONSE_BATCH_JSON_INVALID:{error}"))?;
    let report = evaluate_b_core(&references, &input, &candidates)?;
    let payload = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("REPORT_SERIALIZATION_FAILED:{error}"))?;
    if let Some(path) = report_path {
        fs::write(&path, format!("{payload}\n"))
            .map_err(|error| format!("REPORT_WRITE_FAILED:{path}:{error}"))?;
    } else {
        println!("{payload}");
    }
    Ok(report.pass)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
