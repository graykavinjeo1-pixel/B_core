use std::io::{Read, Write};

use semantic_reasoning::source_bound_causal_frontend::{
    analyze_and_synthesize_source_bound, discover_and_synthesize_python_repository,
    SourceBoundCausalRequestIR, SourceBoundRepositoryDiscoveryRequestIR,
    SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA, SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA,
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
    let envelope: serde_json::Value = serde_json::from_slice(&input).map_err(|error| {
        semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(format!(
            "SOURCE_BOUND_HOST_REQUEST:{error}"
        ))
    })?;
    let schema = envelope
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let receipt = match schema {
        SOURCE_BOUND_CAUSAL_REQUEST_SCHEMA => {
            let request: SourceBoundCausalRequestIR =
                serde_json::from_value(envelope).map_err(|error| {
                    semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(
                        format!("SOURCE_BOUND_HOST_EXPLICIT_REQUEST:{error}"),
                    )
                })?;
            analyze_and_synthesize_source_bound(&request)?
        }
        SOURCE_BOUND_REPOSITORY_DISCOVERY_SCHEMA => {
            let request: SourceBoundRepositoryDiscoveryRequestIR = serde_json::from_value(envelope)
                .map_err(|error| {
                    semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(
                        format!("SOURCE_BOUND_HOST_DISCOVERY_REQUEST:{error}"),
                    )
                })?;
            discover_and_synthesize_python_repository(&request)?
        }
        _ => {
            return Err(
                semantic_reasoning::source_bound_causal_frontend::CausalFrontendFailure::public(
                    format!("SOURCE_BOUND_HOST_SCHEMA:{schema}"),
                ),
            )
        }
    };
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
