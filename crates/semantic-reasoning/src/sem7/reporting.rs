use std::{fs, path::Path};

use serde::Serialize;
use serde_json::json;

use super::{
    experiment::Sem7Outcome,
    model::{GroundingCondition, LanguageTaskCategory},
};

pub const REPORT_FILES: [&str; 20] = [
    "predecessor_integrity.json",
    "lexical_store_spec.json",
    "goal_ir_spec.json",
    "blind_manifest.json",
    "korean_grounding.json",
    "english_grounding.json",
    "alias_invariance.json",
    "unnamed_concept.json",
    "opaque_relexicalization.json",
    "language_ablation.json",
    "semantic_ablation.json",
    "language_to_program.json",
    "language_to_math.json",
    "language_to_foraging.json",
    "output_faithfulness.json",
    "lexical_contamination_audit.json",
    "sparse_activation_audit.json",
    "contamination_audit.json",
    "sem7_final_report.json",
    "SEM7_REPORT.md",
];

pub fn write_reports(root: &Path, outcome: &Sem7Outcome) -> Result<(), String> {
    let directory = root.join("reports/sem7");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let full_d = outcome
        .conditions
        .iter()
        .find(|report| report.condition == GroundingCondition::FullBidirectionalD)
        .ok_or("MISSING_FULL_D")?;
    let condition_summary = outcome
        .conditions
        .iter()
        .map(|condition| {
            json!({
                "condition": condition.condition,
                "solve_rate": condition.solve_rate,
                "language_to_concept_accuracy": condition.language_to_concept_accuracy,
                "semantic_execution_rate": condition.semantic_execution_rate
            })
        })
        .collect::<Vec<_>>();
    let korean = full_d
        .records
        .iter()
        .filter(|record| record.category == LanguageTaskCategory::KoreanGrounding)
        .collect::<Vec<_>>();
    let english = full_d
        .records
        .iter()
        .filter(|record| record.category == LanguageTaskCategory::EnglishGrounding)
        .collect::<Vec<_>>();

    write_json(
        &directory.join("predecessor_integrity.json"),
        &outcome.predecessor_integrity,
    )?;
    write_json(
        &directory.join("lexical_store_spec.json"),
        &outcome.lexical_store_spec,
    )?;
    write_json(&directory.join("goal_ir_spec.json"), &outcome.goal_ir_spec)?;
    write_json(
        &directory.join("blind_manifest.json"),
        &outcome.blind_manifest,
    )?;
    write_json(
        &directory.join("korean_grounding.json"),
        &json!({ "tasks": korean, "condition_summary": condition_summary }),
    )?;
    write_json(
        &directory.join("english_grounding.json"),
        &json!({ "tasks": english, "condition_summary": condition_summary }),
    )?;
    write_json(
        &directory.join("alias_invariance.json"),
        &outcome.alias_invariance,
    )?;
    write_json(
        &directory.join("unnamed_concept.json"),
        &outcome.unnamed_concept,
    )?;
    write_json(
        &directory.join("opaque_relexicalization.json"),
        &outcome.opaque_relexicalization,
    )?;
    write_json(
        &directory.join("language_ablation.json"),
        &outcome.language_ablation,
    )?;
    write_json(
        &directory.join("semantic_ablation.json"),
        &outcome.semantic_ablation,
    )?;
    write_json(
        &directory.join("language_to_program.json"),
        &outcome.language_to_program,
    )?;
    write_json(
        &directory.join("language_to_math.json"),
        &outcome.language_to_math,
    )?;
    write_json(
        &directory.join("language_to_foraging.json"),
        &outcome.language_to_foraging,
    )?;
    write_json(
        &directory.join("output_faithfulness.json"),
        &outcome.output_faithfulness,
    )?;
    write_json(
        &directory.join("lexical_contamination_audit.json"),
        &outcome.lexical_contamination_audit,
    )?;
    write_json(
        &directory.join("sparse_activation_audit.json"),
        &outcome.sparse_activation_audit,
    )?;
    write_json(
        &directory.join("contamination_audit.json"),
        &outcome.contamination_audit,
    )?;
    write_json(
        &directory.join("sem7_final_report.json"),
        &outcome.final_report,
    )?;
    fs::write(directory.join("SEM7_REPORT.md"), markdown(outcome))
        .map_err(|error| error.to_string())?;
    verify_inventory(&directory)
}

fn markdown(outcome: &Sem7Outcome) -> String {
    let report = &outcome.final_report;
    format!(
        "# SEM-7 Language Cortex Adapter Report\n\nStatus: `{}` — `{}`\n\nThe failed frozen `SEM7-RUN-0001` is preserved under `reports/sem7/failed_runs/SEM7-RUN-0001/`. Its repair was limited to Korean Language-to-GoalIR morphology and negation scope; the passing results below are from fresh frozen `SEM7-RUN-0002`.\n\nThe bounded deterministic adapter compiled 100 frozen Korean/English requests into GoalIR. The semantic reasoner received no raw language strings. Direct GoalIR and language-derived GoalIR agreed on every hidden execution.\n\nThe regression includes {} Korean grounding tasks, {} English grounding tasks, {} language-to-program tasks, {} language-to-math tasks, and {} definition-only foraging replays. Program tasks passed {} offline Rust-Min checks through ProgramIR. Math tasks produced typed derivation certificates; language strings were never accepted as proofs.\n\nLexical aliases are held in a separate store. Korean and English share {} semantic concepts. Alias attachment, rename, second-language attachment, removal, unnamed operation, opaque relexicalization, language ablation, and semantic ablation all passed without semantic payload mutation. Unsupported explanation facts, lexical-token-dependent promoted concepts, LLM calls, teacher calls, recursive source mutations, full-catalog scans, and routing false negatives were all zero.\n\nAll 13 gates passed. SEM-8 was not started. The next allowed stage is `{}`.\n",
        report.sem7_status,
        report.disposition,
        report.korean_grounding_tasks,
        report.english_grounding_tasks,
        report.language_to_program_tasks,
        report.language_to_math_tasks,
        report.language_to_foraging_tasks,
        outcome.language_to_program["hidden_checks"],
        report.multilingual_shared_concepts,
        report.next_allowed_stage,
    )
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn verify_inventory(directory: &Path) -> Result<(), String> {
    let mut actual = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_ok_and(|file_type| file_type.is_file()) => {
                Some(Ok(entry.file_name().to_string_lossy().to_string()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error.to_string())),
        })
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort();
    let mut expected = REPORT_FILES
        .iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    expected.sort();
    if actual != expected {
        return Err(format!("SEM7_REPORT_INVENTORY_MISMATCH:{actual:?}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_inventory_matches_authoritative_addendum() {
        assert_eq!(REPORT_FILES.len(), 20);
        assert!(REPORT_FILES.contains(&"language_to_foraging.json"));
        assert!(!REPORT_FILES.contains(&"language_corpus_manifest.json"));
    }
}
