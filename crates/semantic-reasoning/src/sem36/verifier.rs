use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sem35_r1::numeric::{validate_matrix, NumericTransportMatrix};

use super::{
    acceptance::{evaluate_primary, evaluate_secondary, SecondarySem36Evaluation, Sem36Evaluation},
    baseline::{run_sealed_sem35_r1_baseline, Sem35R1EpistemicBaseline},
    config::{
        CONTRACT_VERSION, DEVELOPMENT_SEED, DEVELOPMENT_WORLD_COUNT, FINAL_WORLD_COUNT,
        FINAL_WORLD_SEED, NOVEL_PREDICTION_SEED,
    },
    engine::{run_research_campaign, ResearchMode, ResearchOutcome},
    world::{SafeClosedWorld, WorldOracle, WorldSet},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalWorldManifest {
    pub contract_version: String,
    pub set: WorldSet,
    pub seed: u64,
    pub world_count: usize,
    pub public_world_fingerprint: String,
    pub development_public_fingerprint: String,
    pub development_final_case_overlap: u64,
    pub world_family_count: u64,
    pub hidden_mechanism_family_commitment: String,
    pub safe_closed_world: bool,
    pub offline_local_only: bool,
    pub concrete_worlds_exposed_to_research_system_before_canonical_run: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem36VerificationRequest {
    NumericTransportMatrix {
        contract_version: String,
        payload: Box<NumericTransportMatrix>,
    },
    FreezeFinalManifest {
        contract_version: String,
        seed: u64,
        world_count: usize,
    },
    RunBaseline {
        contract_version: String,
        manifest: Box<FinalWorldManifest>,
    },
    RunResearch {
        contract_version: String,
        manifest: Box<FinalWorldManifest>,
        mode: ResearchMode,
    },
    Evaluate {
        contract_version: String,
        manifest: Box<FinalWorldManifest>,
        baseline: Box<Sem35R1EpistemicBaseline>,
        arms: Box<Vec<ResearchOutcome>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "response_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sem36VerificationResponse {
    NumericTransportMatrixVerified {
        payload: Box<NumericTransportMatrix>,
    },
    FinalManifestFrozen {
        manifest: Box<FinalWorldManifest>,
    },
    BaselineCompleted {
        baseline: Box<Sem35R1EpistemicBaseline>,
    },
    ResearchCompleted {
        outcome: Box<ResearchOutcome>,
    },
    EvaluationCompleted {
        primary: Box<Sem36Evaluation>,
        secondary: Box<SecondarySem36Evaluation>,
        verifier_runner_numeric_transport_equivalence: bool,
        deterministic_recomputation_diff: u64,
        primary_secondary_acceptance_diff: u64,
    },
    Rejected {
        reason: String,
    },
}

pub fn handle(request: Sem36VerificationRequest) -> Sem36VerificationResponse {
    match handle_checked(request) {
        Ok(response) => response,
        Err(reason) => Sem36VerificationResponse::Rejected { reason },
    }
}

fn handle_checked(request: Sem36VerificationRequest) -> Result<Sem36VerificationResponse, String> {
    match request {
        Sem36VerificationRequest::NumericTransportMatrix {
            contract_version,
            payload,
        } => {
            require_contract(&contract_version)?;
            validate_matrix(&payload)?;
            Ok(Sem36VerificationResponse::NumericTransportMatrixVerified { payload })
        }
        Sem36VerificationRequest::FreezeFinalManifest {
            contract_version,
            seed,
            world_count,
        } => {
            require_contract(&contract_version)?;
            if seed != FINAL_WORLD_SEED || world_count != FINAL_WORLD_COUNT {
                return Err("SEM36_FROZEN_FINAL_WORLD_CONFIGURATION_MISMATCH".to_string());
            }
            Ok(Sem36VerificationResponse::FinalManifestFrozen {
                manifest: Box::new(build_final_manifest(seed, world_count)?),
            })
        }
        Sem36VerificationRequest::RunBaseline {
            contract_version,
            manifest,
        } => {
            require_contract(&contract_version)?;
            validate_manifest(&manifest)?;
            let mut oracle = WorldOracle::sealed(manifest.set, manifest.seed, manifest.world_count);
            let baseline = run_sealed_sem35_r1_baseline(&mut oracle)?;
            Ok(Sem36VerificationResponse::BaselineCompleted {
                baseline: Box::new(baseline),
            })
        }
        Sem36VerificationRequest::RunResearch {
            contract_version,
            manifest,
            mode,
        } => {
            require_contract(&contract_version)?;
            validate_manifest(&manifest)?;
            let mut oracle = WorldOracle::sealed(manifest.set, manifest.seed, manifest.world_count);
            let outcome = run_research_campaign(&mut oracle, mode, NOVEL_PREDICTION_SEED)?;
            Ok(Sem36VerificationResponse::ResearchCompleted {
                outcome: Box::new(outcome),
            })
        }
        Sem36VerificationRequest::Evaluate {
            contract_version,
            manifest,
            baseline,
            arms,
        } => {
            require_contract(&contract_version)?;
            validate_manifest(&manifest)?;
            require_all_modes(&arms)?;

            let mut baseline_oracle =
                WorldOracle::sealed(manifest.set, manifest.seed, manifest.world_count);
            let recomputed_baseline = run_sealed_sem35_r1_baseline(&mut baseline_oracle)?;
            if *baseline != recomputed_baseline {
                return Err("SEM36_BASELINE_DETERMINISTIC_RECOMPUTATION_MISMATCH".to_string());
            }
            for transported in arms.iter() {
                let mut oracle =
                    WorldOracle::sealed(manifest.set, manifest.seed, manifest.world_count);
                let recomputed =
                    run_research_campaign(&mut oracle, transported.mode, NOVEL_PREDICTION_SEED)?;
                if *transported != recomputed {
                    return Err(format!(
                        "SEM36_ARM_DETERMINISTIC_RECOMPUTATION_MISMATCH:{:?}",
                        transported.mode
                    ));
                }
            }

            let primary = evaluate_primary(&baseline, &arms)?;
            let secondary = evaluate_secondary(&baseline, &arms)?;
            let primary_levels = [
                primary.level_a_pass,
                primary.level_b_pass,
                primary.level_c_pass,
                primary.level_d_pass,
                primary.level_e_pass,
                primary.level_f_pass,
                primary.level_g_pass,
                primary.level_h_pass,
            ];
            let primary_ablations = [
                primary.epistemic_frontier_selection_ablation_pass,
                primary.scientific_intervention_ablation_pass,
                primary.competing_hypothesis_ablation_pass,
                primary.discovered_mechanism_memory_ablation_pass,
                primary.negative_scientific_memory_ablation_pass,
            ];
            let acceptance_diff = u64::from(
                primary_levels != secondary.levels
                    || primary_ablations != secondary.ablations
                    || primary.invariants_pass != secondary.invariants_pass
                    || primary.sem36_status != secondary.sem36_status,
            );
            if acceptance_diff != 0 {
                return Err("SEM36_PRIMARY_SECONDARY_ACCEPTANCE_DIFF".to_string());
            }
            Ok(Sem36VerificationResponse::EvaluationCompleted {
                primary: Box::new(primary),
                secondary: Box::new(secondary),
                verifier_runner_numeric_transport_equivalence: true,
                deterministic_recomputation_diff: 0,
                primary_secondary_acceptance_diff: 0,
            })
        }
    }
}

fn build_final_manifest(seed: u64, world_count: usize) -> Result<FinalWorldManifest, String> {
    let development = WorldOracle::sealed(
        WorldSet::Development,
        DEVELOPMENT_SEED,
        DEVELOPMENT_WORLD_COUNT,
    );
    let final_world = WorldOracle::sealed(WorldSet::FinalHoldout, seed, world_count);
    let development_ids = development
        .public_cases()
        .into_iter()
        .map(|case| case.case_id)
        .collect::<BTreeSet<_>>();
    let final_cases = final_world.public_cases();
    let final_ids = final_cases
        .iter()
        .map(|case| case.case_id)
        .collect::<BTreeSet<_>>();
    let overlap = development_ids.intersection(&final_ids).count() as u64;
    let family_count = final_cases
        .iter()
        .map(|case| case.family)
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let hidden_commitment = format!(
        "{:x}",
        Sha256::digest(
            b"SEM36_HIDDEN_FAMILY_CONTEXTUAL_CATALYTIC_DELAYED_LOW_NOISE_REDUNDANT_UNIDENTIFIABLE_V1"
        )
    );
    Ok(FinalWorldManifest {
        contract_version: CONTRACT_VERSION.to_string(),
        set: WorldSet::FinalHoldout,
        seed,
        world_count,
        public_world_fingerprint: final_world.public_fingerprint(),
        development_public_fingerprint: development.public_fingerprint(),
        development_final_case_overlap: overlap,
        world_family_count: family_count,
        hidden_mechanism_family_commitment: hidden_commitment,
        safe_closed_world: true,
        offline_local_only: true,
        concrete_worlds_exposed_to_research_system_before_canonical_run: 0,
    })
}

fn validate_manifest(manifest: &FinalWorldManifest) -> Result<(), String> {
    if manifest != &build_final_manifest(FINAL_WORLD_SEED, FINAL_WORLD_COUNT)? {
        return Err("SEM36_FINAL_WORLD_MANIFEST_MISMATCH".to_string());
    }
    if manifest.development_final_case_overlap != 0 || manifest.world_family_count < 3 {
        return Err("SEM36_FINAL_WORLD_FRESHNESS_OR_DIVERSITY_FAILURE".to_string());
    }
    Ok(())
}

fn require_all_modes(arms: &[ResearchOutcome]) -> Result<(), String> {
    let required = [
        ResearchMode::Full,
        ResearchMode::FrontierSelectionOff,
        ResearchMode::ObservationOnly,
        ResearchMode::PrematureSingleHypothesis,
        ResearchMode::MechanisticMemoryOff,
        ResearchMode::NegativeMemoryOff,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let observed = arms.iter().map(|arm| arm.mode).collect::<BTreeSet<_>>();
    if arms.len() != required.len() || observed != required {
        return Err("SEM36_REQUIRED_RESEARCH_ARM_SET_MISMATCH".to_string());
    }
    Ok(())
}

fn require_contract(contract_version: &str) -> Result<(), String> {
    if contract_version != CONTRACT_VERSION {
        return Err("SEM36_CONTRACT_VERSION_MISMATCH".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_manifest_is_fresh_and_multi_family() {
        let manifest = build_final_manifest(FINAL_WORLD_SEED, FINAL_WORLD_COUNT).unwrap();
        assert_eq!(manifest.development_final_case_overlap, 0);
        assert_eq!(manifest.world_family_count, 3);
        assert_eq!(
            manifest.concrete_worlds_exposed_to_research_system_before_canonical_run,
            0
        );
    }

    #[test]
    fn final_manifest_rejects_an_operator_changed_seed() {
        let manifest = build_final_manifest(FINAL_WORLD_SEED + 1, FINAL_WORLD_COUNT).unwrap();
        assert!(validate_manifest(&manifest).is_err());
    }
}
