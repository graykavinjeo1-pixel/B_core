//! Execute only existing register realization on sealed response IRs.
//! This binary never installs a discovered construction.
use semantic_core_adapters::{
    compositional_response_sha256, interpret_document_semantics, realize_document_response,
    ApprovedCompositionalResponseIR, LanguageCodeIR, LanguageRegisterIR,
};
use serde::{Deserialize, Serialize};
use std::{env, fs};

#[derive(Deserialize)]
struct Input {
    rows: Vec<Row>,
}
#[derive(Deserialize)]
struct Row {
    id: String,
    approved_response_ir: ApprovedCompositionalResponseIR,
}
#[derive(Serialize)]
struct Output {
    id: String,
    variant: String,
    surface: String,
    canonical_inverse: bool,
    unsupported_fact_count: usize,
    runtime_installation: &'static str,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::args().nth(1).ok_or("INPUT_JSON_REQUIRED")?;
    let output = env::args().nth(2).ok_or("OUTPUT_JSON_REQUIRED")?;
    let parsed: Input = serde_json::from_slice(&fs::read(input)?)?;
    let mut rows = Vec::new();
    for item in parsed.rows {
        for (variant, register) in [
            ("NEUTRAL", LanguageRegisterIR::Neutral),
            ("FORMAL", LanguageRegisterIR::Formal),
        ] {
            let mut response = item.approved_response_ir.clone();
            response.style.register = register;
            response.semantic_sha256 = compositional_response_sha256(&response);
            let surface = realize_document_response(&response, LanguageCodeIR::Korean)?.markdown;
            let inverse = interpret_document_semantics(&surface, &response, LanguageCodeIR::Korean)
                .ok()
                .is_some_and(|parsed| parsed.validate(&response));
            rows.push(Output {
                id: item.id.clone(),
                variant: variant.into(),
                surface,
                canonical_inverse: inverse,
                unsupported_fact_count: usize::from(!inverse),
                runtime_installation: "FORBIDDEN",
            });
        }
    }
    fs::write(output, serde_json::to_vec_pretty(&rows)?)?;
    Ok(())
}
