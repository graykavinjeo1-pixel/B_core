//! Compiles grounded persona observations as 17-way contrast sets.

use std::{error::Error, fs, path::PathBuf};

use semantic_core_adapters::PersonaContrastiveConstructionModelIR;
use serde::Serialize;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::var_os("BCORE_PERSONA_GROUNDED_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\canonical_persona_parallel_ground_v1\canonical_persona_gold_grounded_ko_v1.jsonl"));
    let output_root = std::env::var_os("BCORE_PERSONA_CONTRASTIVE_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\persona_contrastive_training_v1"));
    fs::create_dir_all(&output_root)?;
    let model = PersonaContrastiveConstructionModelIR::train_from_jsonl(&input)
        .map_err(std::io::Error::other)?;
    let model_path = output_root.join("persona_contrastive_construction_model_ko_v1.json");
    write_json(&model_path, &model)?;
    let receipt = serde_json::json!({
        "schema":"BCORE.PERSONA_CONTRASTIVE_TRAINING_RECEIPT.V1",
        "source_grounded_file_sha256":model.source_grounded_file_sha256,
        "model_sha256":model.model_sha256,
        "train_family_count":model.train_family_count,
        "validation_family_count":model.validation_family_count,
        "blind_family_count":model.blind_family_count,
        "train_profile_count":model.train_families.iter().map(|family| family.profiles.len()).sum::<usize>(),
        "training_unit":"17_WAY_MEANING_FAMILY_CONTRAST_SET",
        "surface_storage":"NONE; STRUCTURAL_KEYS_AND_TYPED_PERSONA_AXES_ONLY",
        "blind_usage":"COUNTED_AND_HELD_OUT; NOT_INCLUDED_IN_MODEL",
        "model_path":model_path,
        "model_file_sha256":file_sha256(&model_path)?,
    });
    let receipt_path = output_root.join("persona_contrastive_training_receipt_ko_v1.json");
    write_json(&receipt_path, &receipt)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn file_sha256(path: &PathBuf) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
