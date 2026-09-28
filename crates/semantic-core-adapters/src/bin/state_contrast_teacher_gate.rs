//! Gate teacher-generated state contrasts at the Approved Response IR boundary.
//! The candidate surface is never a runtime template or semantic authority.

use semantic_core_adapters::{
    interpret_document_semantics, ApprovedCompositionalResponseIR, LanguageCodeIR,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, fs};

#[derive(Debug, Deserialize)]
struct Candidate {
    meaning_family_id: String,
    split: String,
    factor: String,
    value: String,
    baseline_value: String,
    approved_response_ir: ApprovedCompositionalResponseIR,
    surface: String,
    teacher_model: String,
    prompt_sha256: String,
    #[serde(default)]
    paired_observation_id: Option<String>,
    #[serde(default)]
    paired_side: Option<String>,
    #[serde(default)]
    acquisition_protocol: Option<String>,
    #[serde(default)]
    conditions: BTreeMap<String, String>,
    #[serde(default)]
    contrast_kind: Option<String>,
    #[serde(default)]
    interaction_holdout: bool,
    #[serde(default)]
    observation_id: Option<String>,
    #[serde(default)]
    acquisition_mode: Option<String>,
    #[serde(default)]
    fresh_context: bool,
    #[serde(default)]
    repeat_index: Option<usize>,
}

#[derive(Debug, Serialize)]
struct GatedCandidate {
    meaning_family_id: String,
    split: String,
    factor: String,
    value: String,
    baseline_value: String,
    speech_act: String,
    surface: String,
    teacher_model: String,
    prompt_sha256: String,
    paired_observation_id: Option<String>,
    paired_side: Option<String>,
    acquisition_protocol: Option<String>,
    conditions: BTreeMap<String, String>,
    contrast_kind: Option<String>,
    interaction_holdout: bool,
    observation_id: Option<String>,
    acquisition_mode: Option<String>,
    fresh_context: bool,
    repeat_index: Option<usize>,
    canonical_inverse: bool,
    claim_same: bool,
    polarity_same: bool,
    modality_same: bool,
    event_phase_same: bool,
    unsupported_fact_count: usize,
    approval: String,
    reject_reason: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = env::args().nth(1).ok_or("INPUT_JSON_REQUIRED")?;
    let output = env::args().nth(2).ok_or("OUTPUT_JSON_REQUIRED")?;
    let payload: serde_json::Value = serde_json::from_slice(&fs::read(input)?)?;
    let candidates: Vec<Candidate> =
        serde_json::from_value(payload.get("rows").cloned().ok_or("ROWS_MISSING")?)?;
    let mut accepted = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let inverse = interpret_document_semantics(
            &candidate.surface,
            &candidate.approved_response_ir,
            LanguageCodeIR::Korean,
        )
        .ok()
        .is_some_and(|parsed| parsed.validate(&candidate.approved_response_ir));
        let unsupported_fact_count = if inverse { 0 } else { 1 };
        let approved = inverse;
        accepted.push(GatedCandidate {
            meaning_family_id: candidate.meaning_family_id,
            split: candidate.split,
            factor: candidate.factor,
            value: candidate.value,
            baseline_value: candidate.baseline_value,
            speech_act: format!("{:?}", candidate.approved_response_ir.speech_act)
                .to_ascii_uppercase(),
            surface: candidate.surface,
            teacher_model: candidate.teacher_model,
            prompt_sha256: candidate.prompt_sha256,
            paired_observation_id: candidate.paired_observation_id,
            paired_side: candidate.paired_side,
            acquisition_protocol: candidate.acquisition_protocol,
            conditions: candidate.conditions,
            contrast_kind: candidate.contrast_kind,
            interaction_holdout: candidate.interaction_holdout,
            observation_id: candidate.observation_id,
            acquisition_mode: candidate.acquisition_mode,
            fresh_context: candidate.fresh_context,
            repeat_index: candidate.repeat_index,
            canonical_inverse: inverse,
            claim_same: inverse,
            polarity_same: inverse,
            modality_same: inverse,
            event_phase_same: inverse,
            unsupported_fact_count,
            approval: if approved {
                "APPROVED_STATE_CONTRAST_GOLD"
            } else {
                "REJECTED"
            }
            .into(),
            reject_reason: (!approved).then(|| "CANONICAL_INVERSE_GATE_FAILED".into()),
        });
    }
    fs::write(output, serde_json::to_vec_pretty(&accepted)?)?;
    println!(
        "{}",
        serde_json::json!({
            "candidates": accepted.len(),
            "approved": accepted.iter().filter(|row| row.approval == "APPROVED_STATE_CONTRAST_GOLD").count(),
            "rejected": accepted.iter().filter(|row| row.approval == "REJECTED").count(),
        })
    );
    Ok(())
}
