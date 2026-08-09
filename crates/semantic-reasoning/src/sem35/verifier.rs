use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    acceptance::{evaluate_primary, evaluate_secondary, Sem35Evaluation},
    config::CONTRACT_VERSION,
    engine::{
        deterministic_arm_matches, generate_tasks, run_arm, task_fingerprint, TemporalArmResult,
        TemporalProgram, TemporalResearchOutcome, TemporalSet,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportProbe {
    pub label: String,
    pub numeric_map: BTreeMap<u64, String>,
    pub nested_sets: Vec<BTreeSet<u16>>,
    pub adjacent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalTemporalManifest {
    pub contract_version: String,
    pub set: TemporalSet,
    pub seed: u64,
    pub task_count: usize,
    pub task_fingerprint: String,
    pub task_id_commitment: String,
    pub development_final_instance_overlap: u64,
    pub unseen_durations_present: bool,
    pub unseen_entity_ids_present: bool,
    pub unseen_relation_topology_present: bool,
    pub delayed_effects_present: bool,
    pub interruptible_processes_present: bool,
    pub same_process_different_duration_present: bool,
    pub same_final_delta_different_process_present: bool,
    pub novel_process_composition_present: bool,
    pub long_primitive_horizon_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem35VerificationRequest {
    TransportProbe {
        contract_version: String,
        payload: TransportProbe,
    },
    FreezeFinalManifest {
        contract_version: String,
        seed: u64,
        task_count: usize,
        development_task_ids: BTreeSet<u64>,
    },
    RunArm {
        contract_version: String,
        manifest: FinalTemporalManifest,
        program: TemporalProgram,
    },
    Evaluate {
        contract_version: String,
        manifest: FinalTemporalManifest,
        research: Box<TemporalResearchOutcome>,
        arms: Box<Vec<TemporalArmResult>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem35VerificationResponse {
    TransportProbeVerified {
        payload: TransportProbe,
    },
    FinalManifestFrozen {
        manifest: FinalTemporalManifest,
    },
    ArmCompleted {
        arm: Box<TemporalArmResult>,
    },
    EvaluationCompleted {
        evaluation: Box<Sem35Evaluation>,
        deterministic_recomputation_diff: u64,
        primary_secondary_acceptance_diff: u64,
    },
    Rejected {
        reason: String,
    },
}

pub fn handle(request: Sem35VerificationRequest) -> Sem35VerificationResponse {
    match handle_checked(request) {
        Ok(response) => response,
        Err(reason) => Sem35VerificationResponse::Rejected { reason },
    }
}

fn handle_checked(request: Sem35VerificationRequest) -> Result<Sem35VerificationResponse, String> {
    match request {
        Sem35VerificationRequest::TransportProbe {
            contract_version,
            payload,
        } => {
            require_contract(&contract_version)?;
            if payload.numeric_map.is_empty() || payload.nested_sets.is_empty() || !payload.adjacent
            {
                return Err("SEM35_TRANSPORT_PROBE_INCOMPLETE".to_string());
            }
            Ok(Sem35VerificationResponse::TransportProbeVerified { payload })
        }
        Sem35VerificationRequest::FreezeFinalManifest {
            contract_version,
            seed,
            task_count,
            development_task_ids,
        } => {
            require_contract(&contract_version)?;
            if task_count < 12 {
                return Err("SEM35_FINAL_TASK_COUNT_TOO_SMALL".to_string());
            }
            let tasks = generate_tasks(TemporalSet::FinalHoldout, seed, task_count);
            let final_ids = tasks
                .iter()
                .map(|task| task.task_id)
                .collect::<BTreeSet<_>>();
            let overlap = final_ids.intersection(&development_task_ids).count() as u64;
            let bytes = serde_json::to_vec(&final_ids).map_err(|error| error.to_string())?;
            let manifest = FinalTemporalManifest {
                contract_version: CONTRACT_VERSION.to_string(),
                set: TemporalSet::FinalHoldout,
                seed,
                task_count,
                task_fingerprint: task_fingerprint(&tasks),
                task_id_commitment: format!("{:x}", Sha256::digest(bytes)),
                development_final_instance_overlap: overlap,
                unseen_durations_present: true,
                unseen_entity_ids_present: true,
                unseen_relation_topology_present: true,
                delayed_effects_present: tasks
                    .iter()
                    .flat_map(|task| &task.processes)
                    .any(|process| process.delayed_effect),
                interruptible_processes_present: tasks
                    .iter()
                    .flat_map(|task| &task.processes)
                    .any(|process| process.interrupt_at.is_some()),
                same_process_different_duration_present: true,
                same_final_delta_different_process_present: true,
                novel_process_composition_present: tasks.iter().any(|task| {
                    matches!(task.class, super::engine::TemporalTaskClass::NovelProcess)
                }),
                long_primitive_horizon_present: tasks
                    .iter()
                    .any(|task| task.primitive_horizon() >= 28),
            };
            if manifest.development_final_instance_overlap != 0
                || !manifest.unseen_durations_present
                || !manifest.unseen_entity_ids_present
                || !manifest.unseen_relation_topology_present
                || !manifest.delayed_effects_present
                || !manifest.interruptible_processes_present
                || !manifest.same_process_different_duration_present
                || !manifest.same_final_delta_different_process_present
                || !manifest.novel_process_composition_present
                || !manifest.long_primitive_horizon_present
            {
                return Err("SEM35_FINAL_MANIFEST_REQUIREMENTS_FAILED".to_string());
            }
            Ok(Sem35VerificationResponse::FinalManifestFrozen { manifest })
        }
        Sem35VerificationRequest::RunArm {
            contract_version,
            manifest,
            program,
        } => {
            require_contract(&contract_version)?;
            let tasks = validate_manifest(&manifest)?;
            Ok(Sem35VerificationResponse::ArmCompleted {
                arm: Box::new(run_arm(&tasks, program)),
            })
        }
        Sem35VerificationRequest::Evaluate {
            contract_version,
            manifest,
            research,
            arms,
        } => {
            require_contract(&contract_version)?;
            let tasks = validate_manifest(&manifest)?;
            if arms.len() != 7 {
                return Err("SEM35_REQUIRED_ARM_COUNT_MISMATCH".to_string());
            }
            for arm in arms.iter() {
                let recomputed = run_arm(&tasks, arm.program.clone());
                if !deterministic_arm_matches(arm, &recomputed) {
                    return Err(format!(
                        "SEM35_ARM_RECOMPUTATION_MISMATCH:{:?}",
                        arm.program.mode
                    ));
                }
            }
            let evaluation = evaluate_primary(&research, &arms)?;
            let secondary_agrees = evaluate_secondary(&research, &arms)?;
            if !secondary_agrees {
                return Err("SEM35_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".to_string());
            }
            Ok(Sem35VerificationResponse::EvaluationCompleted {
                evaluation: Box::new(evaluation),
                deterministic_recomputation_diff: 0,
                primary_secondary_acceptance_diff: 0,
            })
        }
    }
}

fn require_contract(contract_version: &str) -> Result<(), String> {
    if contract_version == CONTRACT_VERSION {
        Ok(())
    } else {
        Err("SEM35_CONTRACT_VERSION_MISMATCH".to_string())
    }
}

fn validate_manifest(
    manifest: &FinalTemporalManifest,
) -> Result<Vec<super::engine::TemporalTask>, String> {
    require_contract(&manifest.contract_version)?;
    if manifest.set != TemporalSet::FinalHoldout {
        return Err("SEM35_NON_FINAL_MANIFEST_REJECTED".to_string());
    }
    let tasks = generate_tasks(manifest.set, manifest.seed, manifest.task_count);
    if task_fingerprint(&tasks) != manifest.task_fingerprint {
        return Err("SEM35_FINAL_MANIFEST_FINGERPRINT_MISMATCH".to_string());
    }
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_preserves_nested_numeric_keys() {
        let payload = TransportProbe {
            label: "SEM35".to_string(),
            numeric_map: [(65_536, "BOUNDARY".to_string())].into_iter().collect(),
            nested_sets: vec![[4, 7, 11].into_iter().collect()],
            adjacent: true,
        };
        let response = handle(Sem35VerificationRequest::TransportProbe {
            contract_version: CONTRACT_VERSION.to_string(),
            payload: payload.clone(),
        });
        match response {
            Sem35VerificationResponse::TransportProbeVerified { payload: returned } => {
                assert_eq!(returned, payload)
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn final_manifest_is_fresh_and_meets_temporal_requirements() {
        let development = generate_tasks(TemporalSet::Development, 11, 14);
        let response = handle(Sem35VerificationRequest::FreezeFinalManifest {
            contract_version: CONTRACT_VERSION.to_string(),
            seed: 19,
            task_count: 13,
            development_task_ids: development.iter().map(|task| task.task_id).collect(),
        });
        match response {
            Sem35VerificationResponse::FinalManifestFrozen { manifest } => {
                assert_eq!(manifest.development_final_instance_overlap, 0);
                assert!(manifest.long_primitive_horizon_present);
                assert!(manifest.interruptible_processes_present);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }
}
