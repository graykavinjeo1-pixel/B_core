use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sem35::{
    acceptance::{evaluate_primary, Sem35Evaluation},
    config::{DEVELOPMENT_SEED, DEVELOPMENT_TASK_COUNT, FINAL_HOLDOUT_SEED, FINAL_TASK_COUNT},
    engine::{
        generate_tasks, run_arm, run_autonomous_research, task_fingerprint, ProcessFamily,
        TemporalArmMode, TemporalProgram, TemporalSet, TemporalTask,
    },
};

use super::{
    acceptance::{evaluate_secondary_independent, SecondaryAcceptance},
    config::{CONTRACT_VERSION, FRESH_HOLDOUT_SEED, FRESH_HOLDOUT_TASK_COUNT},
    numeric::{validate_matrix, NumericTransportMatrix},
    transport::CanonicalTemporalArm,
};

const FRESH_TASK_ID_SALT: u64 = 0x35A1_E7C0_92D4_B611;
const FRESH_ENTITY_OFFSET: u32 = 1_000_000;
const FRESH_TOPOLOGY_OFFSET: u8 = 64;
const FRESH_DURATION_OFFSET: u16 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshTemporalManifest {
    pub contract_version: String,
    pub set: TemporalSet,
    pub seed: u64,
    pub task_count: usize,
    pub task_fingerprint: String,
    pub task_id_commitment: String,
    pub historical_sem35_final_fingerprint: String,
    pub historical_sem35_final_holdout_reuse: u64,
    pub old_new_holdout_overlap: u64,
    pub fresh_temporal_holdout: bool,
    pub unseen_durations_present: bool,
    pub unseen_entity_ids_present: bool,
    pub unseen_relation_topology_present: bool,
    pub delayed_effects_present: bool,
    pub interruptible_processes_present: bool,
    pub same_process_different_duration_present: bool,
    pub novel_process_composition_present: bool,
    pub long_primitive_horizon_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem35R1VerificationRequest {
    NumericTransportMatrix {
        contract_version: String,
        payload: Box<NumericTransportMatrix>,
    },
    FreezeFreshManifest {
        contract_version: String,
        seed: u64,
        task_count: usize,
    },
    RunArm {
        contract_version: String,
        manifest: Box<FreshTemporalManifest>,
        program: TemporalProgram,
    },
    Evaluate {
        contract_version: String,
        manifest: Box<FreshTemporalManifest>,
        arms: Box<Vec<CanonicalTemporalArm>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem35R1VerificationResponse {
    NumericTransportMatrixVerified {
        payload: Box<NumericTransportMatrix>,
    },
    FreshManifestFrozen {
        manifest: Box<FreshTemporalManifest>,
    },
    ArmCompleted {
        arm: Box<CanonicalTemporalArm>,
    },
    EvaluationCompleted {
        primary: Box<Sem35Evaluation>,
        secondary: Box<SecondaryAcceptance>,
        verifier_runner_numeric_transport_equivalence: bool,
        deterministic_recomputation_diff: u64,
        primary_secondary_acceptance_diff: u64,
    },
    Rejected {
        reason: String,
    },
}

pub fn handle(request: Sem35R1VerificationRequest) -> Sem35R1VerificationResponse {
    match handle_checked(request) {
        Ok(response) => response,
        Err(reason) => Sem35R1VerificationResponse::Rejected { reason },
    }
}

fn handle_checked(
    request: Sem35R1VerificationRequest,
) -> Result<Sem35R1VerificationResponse, String> {
    match request {
        Sem35R1VerificationRequest::NumericTransportMatrix {
            contract_version,
            payload,
        } => {
            require_contract(&contract_version)?;
            validate_matrix(&payload)?;
            Ok(Sem35R1VerificationResponse::NumericTransportMatrixVerified { payload })
        }
        Sem35R1VerificationRequest::FreezeFreshManifest {
            contract_version,
            seed,
            task_count,
        } => {
            require_contract(&contract_version)?;
            if seed != FRESH_HOLDOUT_SEED || task_count != FRESH_HOLDOUT_TASK_COUNT {
                return Err("SEM35_R1_FROZEN_HOLDOUT_CONFIGURATION_MISMATCH".to_string());
            }
            Ok(Sem35R1VerificationResponse::FreshManifestFrozen {
                manifest: Box::new(build_fresh_manifest(seed, task_count)?),
            })
        }
        Sem35R1VerificationRequest::RunArm {
            contract_version,
            manifest,
            program,
        } => {
            require_contract(&contract_version)?;
            let tasks = validate_manifest(&manifest)?;
            Ok(Sem35R1VerificationResponse::ArmCompleted {
                arm: Box::new(CanonicalTemporalArm::try_from(run_arm(&tasks, program))?),
            })
        }
        Sem35R1VerificationRequest::Evaluate {
            contract_version,
            manifest,
            arms,
        } => {
            require_contract(&contract_version)?;
            let tasks = validate_manifest(&manifest)?;
            if arms.len() != 7 {
                return Err("SEM35_R1_REQUIRED_ARM_COUNT_MISMATCH".to_string());
            }
            let mut transported = Vec::with_capacity(arms.len());
            for arm in arms.iter() {
                let recomputed =
                    CanonicalTemporalArm::try_from(run_arm(&tasks, arm.program.clone()))?;
                if *arm != recomputed {
                    return Err(format!(
                        "SEM35_R1_CANONICAL_ARM_RECOMPUTATION_MISMATCH:{:?}",
                        arm.program.mode
                    ));
                }
                transported.push(arm.clone().into_temporal()?);
            }
            require_all_arm_modes(&transported)?;
            let development = generate_tasks(
                TemporalSet::Development,
                DEVELOPMENT_SEED,
                DEVELOPMENT_TASK_COUNT,
            );
            let research = run_autonomous_research(&development);
            let primary = evaluate_primary(&research, &transported)?;
            let secondary = evaluate_secondary_independent(&research, &transported)?;
            let primary_levels = [
                primary.sem35_level_a_pass,
                primary.sem35_level_b_pass,
                primary.sem35_level_c_pass,
                primary.sem35_level_d_pass,
                primary.sem35_level_e_pass,
                primary.sem35_level_f_pass,
                primary.sem35_level_g_pass,
                primary.sem35_level_h_pass,
            ];
            let secondary_levels = [
                secondary.level_a_pass,
                secondary.level_b_pass,
                secondary.level_c_pass,
                secondary.level_d_pass,
                secondary.level_e_pass,
                secondary.level_f_pass,
                secondary.level_g_pass,
                secondary.level_h_pass,
            ];
            let acceptance_diff = u64::from(
                primary_levels != secondary_levels
                    || (primary.sem35_status == "PASS") != (secondary.sem35_r1_status == "PASS"),
            );
            if acceptance_diff != 0 {
                return Err("SEM35_R1_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".to_string());
            }
            Ok(Sem35R1VerificationResponse::EvaluationCompleted {
                primary: Box::new(primary),
                secondary: Box::new(secondary),
                verifier_runner_numeric_transport_equivalence: true,
                deterministic_recomputation_diff: 0,
                primary_secondary_acceptance_diff: 0,
            })
        }
    }
}

pub fn generate_fresh_tasks(seed: u64, count: usize) -> Result<Vec<TemporalTask>, String> {
    let mut tasks = generate_tasks(TemporalSet::FinalHoldout, seed, count);
    for (task_index, task) in tasks.iter_mut().enumerate() {
        task.task_id = task.task_id ^ FRESH_TASK_ID_SALT ^ (task_index as u64).rotate_left(17);
        for (process_index, process) in task.processes.iter_mut().enumerate() {
            let entity_delta = FRESH_ENTITY_OFFSET
                .checked_add(
                    u32::try_from(task_index)
                        .map_err(|_| "SEM35_R1_TASK_INDEX_OVERFLOW")?
                        .checked_mul(256)
                        .ok_or("SEM35_R1_ENTITY_OFFSET_OVERFLOW")?,
                )
                .and_then(|value| value.checked_add(u32::try_from(process_index).ok()?))
                .ok_or("SEM35_R1_ENTITY_OFFSET_OVERFLOW")?;
            for entity in &mut process.entity_ids {
                *entity = entity
                    .checked_add(entity_delta)
                    .ok_or("SEM35_R1_ENTITY_ID_OVERFLOW")?;
            }
            for relation in &mut process.relation_topology {
                *relation = relation
                    .checked_add(FRESH_TOPOLOGY_OFFSET)
                    .ok_or("SEM35_R1_RELATION_TOPOLOGY_OVERFLOW")?;
            }
            process.duration = process
                .duration
                .checked_add(FRESH_DURATION_OFFSET)
                .ok_or("SEM35_R1_DURATION_OVERFLOW")?;
            process.duration_uncertainty.0 = process
                .duration_uncertainty
                .0
                .checked_add(FRESH_DURATION_OFFSET)
                .ok_or("SEM35_R1_DURATION_UNCERTAINTY_OVERFLOW")?;
            process.duration_uncertainty.1 = process
                .duration_uncertainty
                .1
                .checked_add(FRESH_DURATION_OFFSET)
                .ok_or("SEM35_R1_DURATION_UNCERTAINTY_OVERFLOW")?;
            if process.interrupt_at.is_some() {
                process.interrupt_at = Some(process.duration / 2);
            }
        }
    }
    Ok(tasks)
}

fn build_fresh_manifest(seed: u64, count: usize) -> Result<FreshTemporalManifest, String> {
    let tasks = generate_fresh_tasks(seed, count)?;
    let historical = generate_tasks(
        TemporalSet::FinalHoldout,
        FINAL_HOLDOUT_SEED,
        FINAL_TASK_COUNT,
    );
    let old_rows = historical
        .iter()
        .map(canonical_task_bytes)
        .collect::<Result<BTreeSet<_>, _>>()?;
    let new_rows = tasks
        .iter()
        .map(canonical_task_bytes)
        .collect::<Result<BTreeSet<_>, _>>()?;
    let overlap = old_rows.intersection(&new_rows).count() as u64;
    let old_durations = historical
        .iter()
        .flat_map(|task| &task.processes)
        .map(|process| process.duration)
        .collect::<BTreeSet<_>>();
    let new_durations = tasks
        .iter()
        .flat_map(|task| &task.processes)
        .map(|process| process.duration)
        .collect::<BTreeSet<_>>();
    let old_entities = historical
        .iter()
        .flat_map(|task| &task.processes)
        .flat_map(|process| process.entity_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    let new_entities = tasks
        .iter()
        .flat_map(|task| &task.processes)
        .flat_map(|process| process.entity_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    let old_topologies = historical
        .iter()
        .flat_map(|task| &task.processes)
        .map(|process| process.relation_topology.clone())
        .collect::<BTreeSet<_>>();
    let new_topologies = tasks
        .iter()
        .flat_map(|task| &task.processes)
        .map(|process| process.relation_topology.clone())
        .collect::<BTreeSet<_>>();
    let mut family_durations: BTreeMap<ProcessFamily, BTreeSet<u16>> = BTreeMap::new();
    for process in tasks.iter().flat_map(|task| &task.processes) {
        family_durations
            .entry(process.family)
            .or_default()
            .insert(process.duration);
    }
    let task_ids = tasks
        .iter()
        .map(|task| task.task_id)
        .collect::<BTreeSet<_>>();
    let manifest = FreshTemporalManifest {
        contract_version: CONTRACT_VERSION.to_string(),
        set: TemporalSet::FinalHoldout,
        seed,
        task_count: count,
        task_fingerprint: task_fingerprint(&tasks),
        task_id_commitment: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&task_ids).map_err(|error| error.to_string())?)
        ),
        historical_sem35_final_fingerprint: task_fingerprint(&historical),
        historical_sem35_final_holdout_reuse: overlap,
        old_new_holdout_overlap: overlap,
        fresh_temporal_holdout: overlap == 0,
        unseen_durations_present: old_durations.is_disjoint(&new_durations),
        unseen_entity_ids_present: old_entities.is_disjoint(&new_entities),
        unseen_relation_topology_present: old_topologies.is_disjoint(&new_topologies),
        delayed_effects_present: tasks
            .iter()
            .flat_map(|task| &task.processes)
            .any(|process| process.delayed_effect),
        interruptible_processes_present: tasks
            .iter()
            .flat_map(|task| &task.processes)
            .any(|process| process.interrupt_at.is_some()),
        same_process_different_duration_present: family_durations
            .values()
            .any(|durations| durations.len() >= 2),
        novel_process_composition_present: tasks.iter().any(|task| task.processes.len() >= 2),
        long_primitive_horizon_present: tasks.iter().any(|task| task.primitive_horizon() >= 28),
    };
    if !manifest.fresh_temporal_holdout
        || manifest.historical_sem35_final_holdout_reuse != 0
        || !manifest.unseen_durations_present
        || !manifest.unseen_entity_ids_present
        || !manifest.unseen_relation_topology_present
        || !manifest.delayed_effects_present
        || !manifest.interruptible_processes_present
        || !manifest.same_process_different_duration_present
        || !manifest.novel_process_composition_present
        || !manifest.long_primitive_horizon_present
    {
        return Err("SEM35_R1_FRESH_MANIFEST_REQUIREMENTS_FAILED".to_string());
    }
    Ok(manifest)
}

fn canonical_task_bytes(task: &TemporalTask) -> Result<Vec<u8>, String> {
    serde_json::to_vec(task).map_err(|error| format!("SEM35_R1_TASK_SERIALIZATION:{error}"))
}

fn validate_manifest(manifest: &FreshTemporalManifest) -> Result<Vec<TemporalTask>, String> {
    require_contract(&manifest.contract_version)?;
    if manifest.set != TemporalSet::FinalHoldout
        || manifest.seed != FRESH_HOLDOUT_SEED
        || manifest.task_count != FRESH_HOLDOUT_TASK_COUNT
        || !manifest.fresh_temporal_holdout
        || manifest.old_new_holdout_overlap != 0
        || manifest.historical_sem35_final_holdout_reuse != 0
    {
        return Err("SEM35_R1_MANIFEST_CONTRACT_FAILED".to_string());
    }
    let expected = build_fresh_manifest(manifest.seed, manifest.task_count)?;
    if *manifest != expected {
        return Err("SEM35_R1_MANIFEST_RECOMPUTATION_MISMATCH".to_string());
    }
    generate_fresh_tasks(manifest.seed, manifest.task_count)
}

fn require_contract(contract_version: &str) -> Result<(), String> {
    if contract_version == CONTRACT_VERSION {
        Ok(())
    } else {
        Err("SEM35_R1_CONTRACT_VERSION_MISMATCH".to_string())
    }
}

fn require_all_arm_modes(arms: &[crate::sem35::engine::TemporalArmResult]) -> Result<(), String> {
    let expected = [
        TemporalArmMode::Sem34FixedScaleBaseline,
        TemporalArmMode::LearnedVariableDuration,
        TemporalArmMode::FixedLengthSegmentation,
        TemporalArmMode::ProcessMemoryOff,
        TemporalArmMode::CrossScaleConsistencyOff,
        TemporalArmMode::InterruptionOff,
        TemporalArmMode::CompositionOff,
    ];
    if arms.len() == expected.len()
        && expected
            .iter()
            .all(|mode| arms.iter().filter(|arm| arm.program.mode == *mode).count() == 1)
    {
        Ok(())
    } else {
        Err("SEM35_R1_REQUIRED_ARM_MODES_MISMATCH".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem35_r1::numeric::canonical_transport_matrix;

    #[test]
    fn fresh_holdout_is_disjoint_from_historical_sem35() {
        let manifest = build_fresh_manifest(FRESH_HOLDOUT_SEED, FRESH_HOLDOUT_TASK_COUNT).unwrap();
        assert!(manifest.fresh_temporal_holdout);
        assert_eq!(manifest.old_new_holdout_overlap, 0);
        assert_eq!(manifest.historical_sem35_final_holdout_reuse, 0);
    }

    #[test]
    fn numeric_matrix_survives_verifier_boundary() {
        let matrix = canonical_transport_matrix().unwrap();
        let response = handle(Sem35R1VerificationRequest::NumericTransportMatrix {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: Box::new(matrix.clone()),
        });
        match response {
            Sem35R1VerificationResponse::NumericTransportMatrixVerified { payload } => {
                assert_eq!(*payload, matrix);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }
}
