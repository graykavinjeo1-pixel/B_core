//! Generalized, multi-generation self-application substrate.
//!
//! SEM-9 demonstrated that a mechanism can be mapped onto an observed weakness,
//! lowered into a minimal change, falsified in isolation, and retained only
//! after a gain.  Its concrete weakness inventory and final toggle were frozen
//! campaign fixtures.  This module keeps the causal shape while deriving every
//! weakness and change identity from current source observations.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::self_repair_contract::sha256;
use crate::structural_source_repair::{SourceEditAtom, StructuralRepairProgram};

pub const GENERALIZED_SELF_APPLICATION_SCHEMA: &str = "B_CORE_GENERALIZED_SELF_APPLICATION_1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WeaknessEvidenceKind {
    CompilerDiagnostic,
    ExplicitCodeHole,
    StructuralSourceSmell,
    ValidationCounterexample,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicSelfWeaknessIR {
    pub schema: String,
    pub weakness_id: String,
    pub source_generation: u64,
    pub relative_path: PathBuf,
    pub transformation: String,
    pub evidence_kind: WeaknessEvidenceKind,
    pub evidence_sha256: String,
    pub observed_mechanism: String,
    pub required_postconditions: Vec<String>,
    pub prior_counterexample_ids: Vec<String>,
    pub fixed_campaign_fixture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeneralizedChangeOperation {
    Replace,
    Insert,
    Delete,
    Move,
    AtomicMultiEdit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneralizedChangeIR {
    pub schema: String,
    pub change_id: String,
    pub weakness_id: String,
    pub weakness_evidence_kind: WeaknessEvidenceKind,
    pub weakness_evidence_sha256: String,
    pub observed_weakness_mechanism: String,
    pub source_generation: u64,
    pub relative_path: PathBuf,
    pub transformation: String,
    pub solution_strategy: String,
    pub predecessor_sha256: String,
    pub candidate_sha256: String,
    pub structural_program_sha256: String,
    pub operations: Vec<GeneralizedChangeOperation>,
    pub structural_postcondition_count: usize,
    pub derived_from_counterexample_ids: Vec<String>,
    pub fixed_toggle_patch: bool,
    pub one_generation_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationPhase {
    Format,
    Compile,
    PublicObservation,
    ReleaseBuild,
    WorkspaceIntegrity,
    Infrastructure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CounterexampleClass {
    Formatting,
    CompilerType,
    CompilerBorrow,
    CompilerOther,
    AssertionMismatch,
    PublicTestFailure,
    Timeout,
    ConcurrentWorkspaceChange,
    ReleaseBuildFailure,
    InfrastructureFailure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumericRelation {
    ExpectedGreaterThanObserved,
    ExpectedLessThanObserved,
    ExpectedEqualsObserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCounterexampleIR {
    pub schema: String,
    pub counterexample_id: String,
    pub source_generation: u64,
    pub phase: ValidationPhase,
    pub class: CounterexampleClass,
    pub diagnostic_sha256: String,
    pub compiler_error_codes: Vec<String>,
    pub numeric_relation: Option<NumericRelation>,
    pub observed_numeric_value: Option<i128>,
    pub expected_numeric_value: Option<i128>,
    pub failed_strategy: String,
    pub failed_candidate_sha256: String,
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn collect_operations(atom: &SourceEditAtom, output: &mut Vec<GeneralizedChangeOperation>) {
    match atom {
        SourceEditAtom::Replace { .. } => output.push(GeneralizedChangeOperation::Replace),
        SourceEditAtom::Insert { .. } => output.push(GeneralizedChangeOperation::Insert),
        SourceEditAtom::Delete { .. } => output.push(GeneralizedChangeOperation::Delete),
        SourceEditAtom::Move { .. } => output.push(GeneralizedChangeOperation::Move),
        SourceEditAtom::AtomicMultiEdit { edits } => {
            output.push(GeneralizedChangeOperation::AtomicMultiEdit);
            for edit in edits {
                collect_operations(edit, output);
            }
        }
    }
}

fn structural_program_sha256(program: &StructuralRepairProgram) -> Result<String, String> {
    serde_json::to_vec(program)
        .map(|encoded| sha256(&encoded))
        .map_err(|error| format!("GENERALIZED_CHANGE_PROGRAM_SERIALIZE:{error}"))
}

#[allow(clippy::too_many_arguments)]
pub fn derive_dynamic_weakness(
    source_generation: u64,
    relative_path: &Path,
    transformation: &str,
    evidence_kind: WeaknessEvidenceKind,
    evidence_sha256: &str,
    observed_mechanism: &str,
    required_postconditions: Vec<String>,
    prior_counterexample_ids: Vec<String>,
) -> DynamicSelfWeaknessIR {
    let path = normalized_path(relative_path);
    let weakness_id =
        sha256(format!("{path}:{transformation}:{evidence_kind:?}:{evidence_sha256}").as_bytes());
    DynamicSelfWeaknessIR {
        schema: GENERALIZED_SELF_APPLICATION_SCHEMA.to_string(),
        weakness_id,
        source_generation,
        relative_path: relative_path.to_path_buf(),
        transformation: transformation.to_string(),
        evidence_kind,
        evidence_sha256: evidence_sha256.to_string(),
        observed_mechanism: observed_mechanism.to_string(),
        required_postconditions,
        prior_counterexample_ids,
        fixed_campaign_fixture: false,
    }
}

pub fn synthesize_generalized_change(
    weakness: &DynamicSelfWeaknessIR,
    solution_strategy: &str,
    predecessor_sha256: &str,
    candidate_sha256: &str,
    program: &StructuralRepairProgram,
) -> Result<GeneralizedChangeIR, String> {
    if weakness.schema != GENERALIZED_SELF_APPLICATION_SCHEMA
        || weakness.fixed_campaign_fixture
        || weakness.relative_path.as_os_str().is_empty()
        || solution_strategy.is_empty()
        || predecessor_sha256 != program.predecessor_source_sha256
        || candidate_sha256 != program.target_source_sha256
    {
        return Err("GENERALIZED_CHANGE_INPUT_BINDING_FAILURE".to_string());
    }
    let structural_program_sha256 = structural_program_sha256(program)?;
    let mut operations = Vec::new();
    collect_operations(&program.edit, &mut operations);
    if operations.is_empty() {
        return Err("GENERALIZED_CHANGE_HAS_NO_EDIT_OPERATION".to_string());
    }
    let change_id = sha256(
        format!(
            "{}:{}:{}:{}:{}",
            weakness.weakness_id,
            weakness.source_generation,
            solution_strategy,
            candidate_sha256,
            structural_program_sha256
        )
        .as_bytes(),
    );
    Ok(GeneralizedChangeIR {
        schema: GENERALIZED_SELF_APPLICATION_SCHEMA.to_string(),
        change_id,
        weakness_id: weakness.weakness_id.clone(),
        weakness_evidence_kind: weakness.evidence_kind,
        weakness_evidence_sha256: weakness.evidence_sha256.clone(),
        observed_weakness_mechanism: weakness.observed_mechanism.clone(),
        source_generation: weakness.source_generation,
        relative_path: weakness.relative_path.clone(),
        transformation: weakness.transformation.clone(),
        solution_strategy: solution_strategy.to_string(),
        predecessor_sha256: predecessor_sha256.to_string(),
        candidate_sha256: candidate_sha256.to_string(),
        structural_program_sha256,
        operations,
        structural_postcondition_count: program.postconditions.len(),
        derived_from_counterexample_ids: weakness.prior_counterexample_ids.clone(),
        fixed_toggle_patch: false,
        one_generation_only: false,
    })
}

pub fn validate_change_binding(
    change: &GeneralizedChangeIR,
    relative_path: &Path,
    transformation: &str,
    solution_strategy: &str,
    predecessor_sha256: &str,
    candidate_sha256: &str,
    program: &StructuralRepairProgram,
) -> Result<(), String> {
    let program_sha256 = structural_program_sha256(program)?;
    if change.schema != GENERALIZED_SELF_APPLICATION_SCHEMA
        || change.relative_path != relative_path
        || change.transformation != transformation
        || change.solution_strategy != solution_strategy
        || change.predecessor_sha256 != predecessor_sha256
        || change.candidate_sha256 != candidate_sha256
        || change.structural_program_sha256 != program_sha256
        || change.structural_postcondition_count != program.postconditions.len()
        || change.fixed_toggle_patch
        || change.one_generation_only
    {
        return Err("GENERALIZED_CHANGE_REQUEST_BINDING_FAILURE".to_string());
    }
    let mut operations = Vec::new();
    collect_operations(&program.edit, &mut operations);
    if change.operations != operations {
        return Err("GENERALIZED_CHANGE_OPERATION_BINDING_FAILURE".to_string());
    }
    Ok(())
}

fn compiler_error_codes(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut output = BTreeSet::new();
    for index in 0..bytes.len().saturating_sub(6) {
        if bytes[index..].starts_with(b"error[E") {
            let rest = &text[index + 6..];
            if let Some(end) = rest.find(']') {
                let digits = &rest[..end];
                if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    output.insert(format!("E{digits}"));
                }
            }
        }
    }
    output.into_iter().collect()
}

fn numeric_value_after(text: &str, marker: &str) -> Option<i128> {
    text.lines().rev().find_map(|line| {
        let trimmed = line.trim();
        let value = trimmed.strip_prefix(marker)?.trim();
        value
            .trim_matches(|character: char| !character.is_ascii_digit() && character != '-')
            .parse::<i128>()
            .ok()
    })
}

fn counterexample_class(
    phase: ValidationPhase,
    failure_reason: &str,
    diagnostic_tail: &str,
) -> CounterexampleClass {
    let lower = diagnostic_tail.to_ascii_lowercase();
    if lower.contains("timed out") || failure_reason.contains("TIMEOUT") {
        CounterexampleClass::Timeout
    } else if failure_reason.contains("CONCURRENT_WORKSPACE")
        || failure_reason.contains("TARGET_CHANGED")
    {
        CounterexampleClass::ConcurrentWorkspaceChange
    } else if phase == ValidationPhase::Format {
        CounterexampleClass::Formatting
    } else if lower.contains("assertion `left == right` failed")
        || (lower.contains("left:") && lower.contains("right:"))
    {
        CounterexampleClass::AssertionMismatch
    } else if phase == ValidationPhase::Compile && lower.contains("mismatched types") {
        CounterexampleClass::CompilerType
    } else if phase == ValidationPhase::Compile
        && (lower.contains("borrow") || lower.contains("moved value"))
    {
        CounterexampleClass::CompilerBorrow
    } else if phase == ValidationPhase::Compile {
        CounterexampleClass::CompilerOther
    } else if phase == ValidationPhase::PublicObservation {
        CounterexampleClass::PublicTestFailure
    } else if phase == ValidationPhase::ReleaseBuild {
        CounterexampleClass::ReleaseBuildFailure
    } else if phase == ValidationPhase::Infrastructure {
        CounterexampleClass::InfrastructureFailure
    } else {
        CounterexampleClass::Unknown
    }
}

pub fn validation_counterexample(
    source_generation: u64,
    phase: ValidationPhase,
    failure_reason: &str,
    diagnostic_sha256: &str,
    diagnostic_tail: &str,
    failed_strategy: &str,
    failed_candidate_sha256: &str,
) -> ValidationCounterexampleIR {
    let observed_numeric_value = numeric_value_after(diagnostic_tail, "left:");
    let expected_numeric_value = numeric_value_after(diagnostic_tail, "right:");
    let numeric_relation =
        observed_numeric_value
            .zip(expected_numeric_value)
            .map(|(observed, expected)| {
                if expected > observed {
                    NumericRelation::ExpectedGreaterThanObserved
                } else if expected < observed {
                    NumericRelation::ExpectedLessThanObserved
                } else {
                    NumericRelation::ExpectedEqualsObserved
                }
            });
    let class = counterexample_class(phase, failure_reason, diagnostic_tail);
    let codes = compiler_error_codes(diagnostic_tail);
    let counterexample_id = sha256(
        format!(
            "{source_generation}:{phase:?}:{class:?}:{diagnostic_sha256}:{failed_strategy}:{failed_candidate_sha256}:{numeric_relation:?}"
        )
        .as_bytes(),
    );
    ValidationCounterexampleIR {
        schema: GENERALIZED_SELF_APPLICATION_SCHEMA.to_string(),
        counterexample_id,
        source_generation,
        phase,
        class,
        diagnostic_sha256: diagnostic_sha256.to_string(),
        compiler_error_codes: codes,
        numeric_relation,
        observed_numeric_value,
        expected_numeric_value,
        failed_strategy: failed_strategy.to_string(),
        failed_candidate_sha256: failed_candidate_sha256.to_string(),
    }
}

/// Scores a newly synthesized strategy against actual prior counterexamples.
/// The score is deliberately advisory: compile and public observations remain
/// the authority.  It changes search order without manufacturing acceptance.
pub fn feedback_priority(
    solution_strategy: &str,
    counterexamples: &[ValidationCounterexampleIR],
) -> i32 {
    let strategy = solution_strategy.to_ascii_uppercase();
    let mut score = 0_i32;
    for counterexample in counterexamples {
        match counterexample.numeric_relation {
            Some(NumericRelation::ExpectedGreaterThanObserved) => {
                if strategy.contains("MULTIPLY") {
                    score += 60;
                } else if strategy.contains("ADD") || strategy.contains("MAX") {
                    score += 25;
                } else if strategy.contains("SUBTRACT")
                    || strategy.contains("MIN")
                    || strategy.contains("ZERO")
                {
                    score -= 30;
                }
            }
            Some(NumericRelation::ExpectedLessThanObserved) => {
                if strategy.contains("SUBTRACT") || strategy.contains("MIN") {
                    score += 45;
                } else if strategy.contains("MULTIPLY") || strategy.contains("ADD") {
                    score -= 25;
                }
            }
            Some(NumericRelation::ExpectedEqualsObserved) => score -= 5,
            None => {}
        }
        match counterexample.class {
            CounterexampleClass::CompilerType => {
                if strategy.contains("BOUND_VALUE") || strategy.contains("EXISTING_CALL") {
                    score += 15;
                }
            }
            CounterexampleClass::CompilerBorrow if strategy.contains("CLONE") => score += 30,
            _ => {}
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structural_source_repair::synthesize_structural_repair;

    #[test]
    fn weakness_is_observation_derived_and_stable_across_generations() {
        let first = derive_dynamic_weakness(
            1,
            Path::new("src/lib.rs"),
            "EXPLICIT_HOLE",
            WeaknessEvidenceKind::ExplicitCodeHole,
            &sha256(b"hole-observation"),
            "typed callable contains an executable hole",
            vec!["public observation passes".to_string()],
            Vec::new(),
        );
        let second = derive_dynamic_weakness(
            2,
            Path::new("src/lib.rs"),
            "EXPLICIT_HOLE",
            WeaknessEvidenceKind::ExplicitCodeHole,
            &sha256(b"hole-observation"),
            "typed callable contains an executable hole",
            vec!["public observation passes".to_string()],
            Vec::new(),
        );
        assert_eq!(first.weakness_id, second.weakness_id);
        assert_ne!(first.source_generation, second.source_generation);
        assert!(!first.fixed_campaign_fixture);
    }

    #[test]
    fn change_is_bound_to_real_edit_program_and_is_multi_generation() {
        let before = "pub fn value() -> i32 { todo!() }\n";
        let after = "pub fn value() -> i32 { 7 }\n";
        let program = synthesize_structural_repair("src/lib.rs", before, after).unwrap();
        let weakness = derive_dynamic_weakness(
            7,
            Path::new("src/lib.rs"),
            "EXPLICIT_HOLE",
            WeaknessEvidenceKind::ExplicitCodeHole,
            &sha256(before.as_bytes()),
            "hole",
            vec!["AST target".to_string()],
            Vec::new(),
        );
        let change = synthesize_generalized_change(
            &weakness,
            "INTEGER_LITERAL",
            &sha256(before.as_bytes()),
            &sha256(after.as_bytes()),
            &program,
        )
        .unwrap();
        assert!(!change.fixed_toggle_patch);
        assert!(!change.one_generation_only);
        assert!(!change.operations.is_empty());
        validate_change_binding(
            &change,
            Path::new("src/lib.rs"),
            "EXPLICIT_HOLE",
            "INTEGER_LITERAL",
            &sha256(before.as_bytes()),
            &sha256(after.as_bytes()),
            &program,
        )
        .unwrap();
    }

    #[test]
    fn public_counterexample_changes_next_strategy_order() {
        let tail = "assertion `left == right` failed\n  left: 7\n right: 12\n";
        let counterexample = validation_counterexample(
            3,
            ValidationPhase::PublicObservation,
            "REGRESSION_VALIDATION_FAILED",
            &sha256(tail.as_bytes()),
            tail,
            "GRAMMAR_COMPOSITION:BINARY_ADD",
            &sha256(b"candidate"),
        );
        assert_eq!(counterexample.class, CounterexampleClass::AssertionMismatch);
        assert_eq!(
            counterexample.numeric_relation,
            Some(NumericRelation::ExpectedGreaterThanObserved)
        );
        assert!(
            feedback_priority(
                "GRAMMAR_COMPOSITION:BINARY_MULTIPLY",
                std::slice::from_ref(&counterexample)
            ) > feedback_priority("GRAMMAR_COMPOSITION:BINARY_SUBTRACT", &[counterexample])
        );
    }
}
