use std::io::{Read, Write};

use semantic_reasoning::source_bound_causal_frontend::{
    analyze_and_synthesize_source_bound, SourceBoundCausalRequestIR,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{}:{}", error.kind.as_code(), error.detail);
        std::process::exit(1);
    }
}

fn run() -> Result<(), semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).map_err(|error| {
        semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(format!(
            "SOURCE_BOUND_HOST_STDIN:{error}"
        ))
    })?;
    let request: SourceBoundCausalRequestIR = serde_json::from_slice(&input).map_err(|error| {
        semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(format!(
            "SOURCE_BOUND_HOST_REQUEST:{error}"
        ))
    })?;
    let receipt = analyze_and_synthesize_source_bound(&request)?;
    let output = serde_json::to_vec_pretty(&receipt).map_err(|error| {
        semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(format!(
            "SOURCE_BOUND_HOST_RECEIPT:{error}"
        ))
    })?;
    std::io::stdout().write_all(&output).map_err(|error| {
        semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(format!(
            "SOURCE_BOUND_HOST_STDOUT:{error}"
        ))
    })
}
