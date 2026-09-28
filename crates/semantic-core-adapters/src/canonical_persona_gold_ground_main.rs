//! Streams the 10,000-row synthetic persona corpus through B_Core's actual
//! planner and approval path.  Output lives on D: so the USB source volume is
//! never used as a build or artifact cache.

use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
};

use dockable_semantic_core::DockableCore;
use semantic_core_adapters::{ground_synthetic_persona_gold_row, CanonicalPersonaGoldRecordIR};
use serde::Serialize;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::var_os("BCORE_PERSONA_GOLD_SOURCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\corpora\persona_synthetic_ko_v1\persona_synthetic_ko_v1.jsonl"));
    let output_root = std::env::var_os("BCORE_PERSONA_GOLD_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\B_Core_validation\canonical_persona_gold_v1"));
    fs::create_dir_all(&output_root)?;
    let output = output_root.join("canonical_persona_gold_grounded_ko_v1.jsonl");
    let temporary = output.with_extension("tmp");
    let mut writer = BufWriter::new(fs::File::create(&temporary)?);
    let core = DockableCore::load_embedded()
        .map_err(|error| std::io::Error::other(format!("CORE_LOAD:{error:?}")))?;
    let mut accepted = 0_u64;
    let mut inverse_verified = 0_u64;
    let mut rejected = BTreeMap::<String, u64>::new();
    let mut split_counts = BTreeMap::<String, u64>::new();
    let source = BufReader::new(fs::File::open(&input)?);
    for (line_number, line) in source.lines().enumerate() {
        let line = line?;
        let row = match serde_json::from_str(&line) {
            Ok(row) => row,
            Err(_) => {
                *rejected.entry("SOURCE_JSON_INVALID".into()).or_default() += 1;
                continue;
            }
        };
        match ground_synthetic_persona_gold_row(&core, &row) {
            Ok(record) => {
                record
                    .validate()
                    .then_some(())
                    .ok_or("GROUNDED_RECORD_INVALID")?;
                inverse_verified += u64::from(record.source_surface_inverse_verified);
                *split_counts
                    .entry(format!("{:?}", record.canonical_record.split))
                    .or_default() += 1;
                serde_json::to_writer(&mut writer, &record)?;
                writer.write_all(b"\n")?;
                accepted += 1;
            }
            Err(reason) => *rejected.entry(reason).or_default() += 1,
        }
        if line_number % 1_000 == 999 {
            writer.flush()?;
        }
    }
    writer.flush()?;
    drop(writer);
    fs::rename(&temporary, &output)?;
    let report = serde_json::json!({
        "schema":"BCORE.CANONICAL_PERSONA_GOLD_GROUNDING_RECEIPT.V1",
        "source_path": input,
        "source_file_sha256": file_sha256(&input)?,
        "input_boundary":"SYNTHETIC_APPROVED_MEANING_TO_ACTUAL_BCORE_PLANNER_TO_REPLAY_VERIFIED_APPROVED_IR",
        "surface_authority":"NONAUTHORITATIVE_CONSTRUCTION_CANDIDATE_ONLY; RUNTIME_USE_REQUIRES_CODEC_INVERSE",
        "accepted_count":accepted,
        "source_surface_inverse_verified_count":inverse_verified,
        "rejected_count":rejected.values().sum::<u64>(),
        "rejection_reasons":rejected,
        "split_counts":split_counts,
        "grounded_file":output,
        "grounded_file_sha256":file_sha256(&output)?,
    });
    let receipt = output_root.join("canonical_persona_gold_grounding_receipt_ko_v1.json");
    write_json(&receipt, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
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

#[allow(dead_code)]
fn decode_record(bytes: &[u8]) -> Result<CanonicalPersonaGoldRecordIR, Box<dyn Error>> {
    Ok(serde_json::from_slice(bytes)?)
}
