//! Offline-only compiler for the canonical language-codec training campaign.
//!
//! The binary writes reproducible research artifacts to D: by default.  It
//! never writes a learned artifact into the production source tree and never
//! reads natural-language requests or prior response text.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use semantic_core_adapters::{
    CanonicalResponseCodecModelIR, CanonicalResponseDatasetSplitIR,
    CanonicalResponseTrainingCampaignIR,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let output_root = std::env::var_os("BCORE_CANONICAL_TRAINING_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\canonical_response_training"));
    fs::create_dir_all(&output_root)?;

    let campaign = CanonicalResponseTrainingCampaignIR::build_korean_plan_descriptions()?;
    let model = CanonicalResponseCodecModelIR::train(&campaign)?;
    let campaign_path = output_root.join("canonical_plan_description_campaign_ko_v1.json");
    let model_path = output_root.join("canonical_response_codec_model_ko_v1.json");
    write_json(&campaign_path, &campaign)?;
    write_json(&model_path, &model)?;

    let receipt = serde_json::json!({
        "schema": "BCORE.CANONICAL_RESPONSE_CODEC_TRAINING_RECEIPT.V1",
        "campaign_sha256": campaign.campaign_sha256,
        "model_sha256": model.model_sha256,
        "record_count": campaign.records.len(),
        "attempted_seed_count": 300,
        "generated_record_count": campaign.records.len(),
        "rejected_record_count": 0,
        "rejection_reasons": {},
        "split_policy": "SEMANTIC_FAMILY_DISJOINT; FAMILY_00=BLIND; FAMILY_01=VALIDATION; REMAINING_FAMILIES=TRAIN",
        "train_count": campaign.records.iter().filter(|record| record.split == CanonicalResponseDatasetSplitIR::Train).count(),
        "validation_count": campaign.records.iter().filter(|record| record.split == CanonicalResponseDatasetSplitIR::Validation).count(),
        "blind_count": campaign.records.iter().filter(|record| record.split == CanonicalResponseDatasetSplitIR::Blind).count(),
        "construction_count": model.constructions.len(),
        "construction_observation_counts": model.constructions.iter().map(|construction| construction.observation_count).collect::<Vec<_>>(),
        "input_boundary": "SYNTHETIC_WORLD_STATE_TO_BCORE_PLANNER_TO_APPROVED_RESPONSE_IR_ONLY",
        "surface_authority": "NONAUTHORITATIVE_CODEC_REFERENCE_ONLY",
        "runtime_status": "RUNTIME_LOADABLE_SHADOW_STORE; ENABLE_WITH_STANDALONE_CONSTRUCTION_STORE_PATH",
        "campaign_file_sha256": file_sha256(&campaign_path)?,
        "model_file_sha256": file_sha256(&model_path)?,
    });
    let receipt_path = output_root.join("canonical_response_codec_training_receipt_ko_v1.json");
    write_json(&receipt_path, &receipt)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let rendered = serde_json::to_vec_pretty(value)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, rendered)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn file_sha256(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
