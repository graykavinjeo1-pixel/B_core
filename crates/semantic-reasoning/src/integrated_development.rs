//! Integration boundary for recursive improvement, existing SEM-5 program
//! composition, and the independently verified RSI installation gate.
//!
//! SEM-5 remains the authoritative program synthesizer.  This module does not
//! duplicate its primitive catalog or synthesis logic; it turns a synthesized
//! typed ProgramIR into the same proposal-only trust path used by self-repair.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::self_repair_contract::{
    sha256, validate_installation_authority, DefectContractIR, Frozen, InstallationGateError,
    ObservationIR, PatchCandidateIR, RepairSpecIR, VerificationReceipt,
};
use crate::sem27::engine::{
    run_post_scaffold_epoch, PostScaffoldEpochRequest, PostScaffoldEpochResult,
};
use crate::sem5::{
    emitter::emit_neutral_text,
    ir::type_check,
    learner::synthesize,
    model::{ProgramIR, ProgramTask, ProgrammingPromotion, SynthesisCondition},
    tasks::programming_primitive_catalog,
};

pub const CAMPAIGN_ID: &str = "B_CORE-INTEGRATED-DEVELOPMENT-01";
pub const AUTHORITATIVE_PREDECESSOR: &str = "8092ea4aba69fd23c9f4e9d56132d488a58e0382";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOpportunityIR {
    pub observed_gap: String,
    pub desired_behavior: String,
    pub trigger: String,
    pub evidence: Vec<String>,
    pub preserved_behavior: Vec<String>,
    pub resource_constraints: Vec<String>,
    pub verification_requirements: Vec<String>,
    pub provenance: Vec<String>,
    pub operator_selected_implementation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionWork {
    pub opportunity: CapabilityOpportunityIR,
    pub task: ProgramTask,
    pub promotions: Vec<ProgrammingPromotion>,
    pub predecessor_tree_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositeProgramCandidateIR {
    pub opportunity: Frozen<CapabilityOpportunityIR>,
    pub observation: Frozen<ObservationIR>,
    pub defect_contract: Frozen<DefectContractIR>,
    pub repair_spec: Frozen<RepairSpecIR>,
    pub program_ir: ProgramIR,
    pub program_ir_sha256: String,
    pub primitive_catalog_sha256: String,
    pub used_primitive_ids: Vec<String>,
    pub promoted_concept_ids: Vec<String>,
    pub primitive_expanded_nodes: usize,
    pub operational_nodes: usize,
    pub recombinations: usize,
    pub type_effect_audit_pass: bool,
    pub neutral_program_sha256: String,
    pub patch_candidate: PatchCandidateIR,
    pub full_source_scan_events: usize,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegratedDevelopmentEpochRequest {
    pub recursive_epoch: PostScaffoldEpochRequest,
    pub composition_work: Option<CompositionWork>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegratedDevelopmentEpochResult {
    pub recursive_epoch: PostScaffoldEpochResult,
    pub composite_candidate: Option<CompositeProgramCandidateIR>,
    pub composition_attempted: bool,
    pub core_self_approval_events: usize,
    pub unverified_install_events: usize,
    pub full_source_scan_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegratedInstallationGateError {
    DefectContractReceiptMismatch,
    RsiGate(InstallationGateError),
}

pub fn run_integrated_development_epoch(
    request: IntegratedDevelopmentEpochRequest,
) -> Result<IntegratedDevelopmentEpochResult, String> {
    let recursive_epoch = run_post_scaffold_epoch(request.recursive_epoch)?;
    let composition_attempted = request.composition_work.is_some();
    let composite_candidate = request
        .composition_work
        .map(compose_existing_sem5_capability)
        .transpose()?;
    Ok(IntegratedDevelopmentEpochResult {
        recursive_epoch,
        composite_candidate,
        composition_attempted,
        core_self_approval_events: 0,
        unverified_install_events: 0,
        full_source_scan_events: 0,
    })
}

/// Reuses the already established SEM-5 typed synthesizer.  The resulting
/// artifact is a proposal only and cannot authorize its own installation.
pub fn compose_existing_sem5_capability(
    work: CompositionWork,
) -> Result<CompositeProgramCandidateIR, String> {
    if work.opportunity.operator_selected_implementation {
        return Err("OPERATOR_SELECTED_IMPLEMENTATION_FORBIDDEN".to_string());
    }
    if work.predecessor_tree_hash.is_empty() {
        return Err("PREDECESSOR_TREE_HASH_MISSING".to_string());
    }

    let opportunity = Frozen::new(work.opportunity.clone())
        .map_err(|error| format!("OPPORTUNITY_FREEZE:{error}"))?;
    let observation = Frozen::new(observation_from_opportunity(&work.opportunity))
        .map_err(|error| format!("OBSERVATION_FREEZE:{error}"))?;
    let defect_contract = Frozen::new(defect_contract_from_opportunity(&work.opportunity))
        .map_err(|error| format!("DEFECT_CONTRACT_FREEZE:{error}"))?;
    let repair_spec = Frozen::new(repair_spec_from_opportunity(&work.opportunity))
        .map_err(|error| format!("REPAIR_SPEC_FREEZE:{error}"))?;

    let program_ir = synthesize(
        &work.task,
        SynthesisCondition::FirstPrinciplesD,
        &work.promotions,
    )?;
    type_check(&program_ir, &work.task.definitions)?;
    if program_ir.recombinations == 0 {
        return Err("COMPOSITE_PROGRAM_REQUIRED".to_string());
    }
    if program_ir.operational_nodes >= program_ir.primitive_expanded_nodes {
        return Err("COMPOSITION_DID_NOT_COMPRESS_PRIMITIVES".to_string());
    }

    let program_bytes =
        serde_json::to_vec(&program_ir).map_err(|error| format!("PROGRAM_IR_SERIALIZE:{error}"))?;
    let program_ir_sha256 = sha256(&program_bytes);
    let catalog = programming_primitive_catalog();
    let catalog_bytes = serde_json::to_vec(&catalog)
        .map_err(|error| format!("PRIMITIVE_CATALOG_SERIALIZE:{error}"))?;
    let primitive_catalog_sha256 = sha256(&catalog_bytes);
    let used_kinds = used_node_kinds(&program_ir)?;
    let used_primitive_ids = catalog
        .iter()
        .filter(|primitive| used_kinds.contains(&primitive.node_kind))
        .map(|primitive| primitive.primitive_id.clone())
        .collect::<Vec<_>>();
    if used_primitive_ids.len() < 2 {
        return Err("INSUFFICIENT_PRIMITIVE_COMPOSITION".to_string());
    }

    let promoted_concept_ids = program_ir
        .concept_ids
        .iter()
        .filter(|concept| concept.starts_with('C'))
        .cloned()
        .collect::<Vec<_>>();
    let neutral_program = emit_neutral_text(&program_ir);
    let neutral_program_sha256 = sha256(neutral_program.as_bytes());
    let artifact_path = format!("generated/{}.program_ir.json", program_ir.program_id);
    let patch_candidate = PatchCandidateIR {
        predecessor_tree_hash: work.predecessor_tree_hash,
        changed_files: vec![artifact_path],
        changed_symbols: vec![program_ir.program_id.clone()],
        unified_diff_sha256: program_ir_sha256.clone(),
        repair_spec_sha256: repair_spec.sha256.clone(),
        consequence_predictions: vec![
            work.opportunity.desired_behavior.clone(),
            "typed effect constraints remain enforced by ProgramIR".to_string(),
            "installation remains blocked until independent verification".to_string(),
        ],
        proposer_confidence_millis: 800,
        core_self_approved: false,
    };

    Ok(CompositeProgramCandidateIR {
        opportunity,
        observation,
        defect_contract,
        repair_spec,
        primitive_expanded_nodes: program_ir.primitive_expanded_nodes,
        operational_nodes: program_ir.operational_nodes,
        recombinations: program_ir.recombinations,
        program_ir,
        program_ir_sha256,
        primitive_catalog_sha256,
        used_primitive_ids,
        promoted_concept_ids,
        type_effect_audit_pass: true,
        neutral_program_sha256,
        patch_candidate,
        full_source_scan_events: 0,
        installed: false,
    })
}

pub fn validate_composite_installation_authority(
    candidate: &CompositeProgramCandidateIR,
    receipt: &VerificationReceipt,
) -> Result<(), IntegratedInstallationGateError> {
    if receipt.defect_contract_sha256 != candidate.defect_contract.sha256 {
        return Err(IntegratedInstallationGateError::DefectContractReceiptMismatch);
    }
    validate_installation_authority(&candidate.patch_candidate, receipt)
        .map_err(IntegratedInstallationGateError::RsiGate)
}

fn observation_from_opportunity(opportunity: &CapabilityOpportunityIR) -> ObservationIR {
    ObservationIR {
        observed_event: opportunity.observed_gap.clone(),
        trigger: opportunity.trigger.clone(),
        expected_observable: opportunity.desired_behavior.clone(),
        actual_observable: opportunity.observed_gap.clone(),
        evidence: opportunity.evidence.clone(),
        provenance: opportunity.provenance.clone(),
        contains_hidden_diagnosis: false,
    }
}

fn defect_contract_from_opportunity(opportunity: &CapabilityOpportunityIR) -> DefectContractIR {
    DefectContractIR {
        affected_behavior: opportunity.desired_behavior.clone(),
        violated_invariant: "requested capability is not yet represented by executable typed code"
            .to_string(),
        scope: "bounded capability gap represented as a typed ProgramTask".to_string(),
        trigger_conditions: vec![opportunity.trigger.clone()],
        expected_vs_observed: format!(
            "expected={} | observed={}",
            opportunity.desired_behavior, opportunity.observed_gap
        ),
        causal_evidence: opportunity.evidence.clone(),
        uncertainty: "composition remains a proposal until independent evaluation".to_string(),
        suspected_mechanism_classes: vec![
            "missing primitive composition".to_string(),
            "missing reusable semantic recombination".to_string(),
        ],
        affected_interfaces: vec!["SEM5_PROGRAM_IR".to_string()],
        preserved_behavior: opportunity.preserved_behavior.clone(),
        provenance: opportunity.provenance.clone(),
        prescribes_concrete_edit: false,
    }
}

fn repair_spec_from_opportunity(opportunity: &CapabilityOpportunityIR) -> RepairSpecIR {
    RepairSpecIR {
        required_postcondition: opportunity.desired_behavior.clone(),
        restored_invariants: vec![
            "program is type/effect valid".to_string(),
            "program is composed from the frozen SEM-5 primitive basis".to_string(),
        ],
        allowed_semantic_changes: vec!["add one isolated composed ProgramIR artifact".to_string()],
        forbidden_semantic_changes: opportunity.preserved_behavior.clone(),
        compatibility_requirements: vec![
            "SEM-5 primitive and promoted-concept semantics remain authoritative".to_string(),
            "RSI verifier and installer remain external to the proposer".to_string(),
        ],
        resource_constraints: opportunity.resource_constraints.clone(),
        expected_consequences: vec![opportunity.desired_behavior.clone()],
        rollback_conditions: vec![
            "remove the isolated ProgramIR artifact and restore predecessor hash".to_string(),
        ],
        verification_requirements: opportunity.verification_requirements.clone(),
        applicability: vec![opportunity.trigger.clone()],
        uncertainty: "independent verifier decides semantic acceptance".to_string(),
        encodes_exact_patch: false,
    }
}

fn used_node_kinds(program_ir: &ProgramIR) -> Result<BTreeSet<String>, String> {
    let encoded =
        serde_json::to_value(program_ir).map_err(|error| format!("PROGRAM_IR_VALUE:{error}"))?;
    let mut kinds = BTreeSet::new();
    collect_node_kinds(&encoded, &mut kinds);
    Ok(kinds)
}

fn collect_node_kinds(value: &JsonValue, kinds: &mut BTreeSet<String>) {
    match value {
        JsonValue::Object(object) => {
            if let Some(kind) = object.get("node_kind").and_then(JsonValue::as_str) {
                kinds.insert(kind.to_string());
            }
            for child in object.values() {
                collect_node_kinds(child, kinds);
            }
        }
        JsonValue::Array(values) => {
            for child in values {
                collect_node_kinds(child, kinds);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::self_repair_contract::{VerificationDecision, VerificationReceipt};
    use crate::sem26::engine::DirectorState;
    use crate::sem27::engine::PostScaffoldState;
    use crate::sem5::{
        ir::execute,
        learner::{discover_candidates, initial_promotions},
        tasks::{evaluate_contract, generate_property_cases, generate_task_sets},
    };

    fn composition_work() -> CompositionWork {
        let sets = generate_task_sets(0x1A7E_600D);
        let candidates = discover_candidates(&sets.discovery);
        let promotions = initial_promotions(&candidates, &sets.calibration);
        assert!(promotions
            .iter()
            .any(|promotion| promotion.concept.concept_id == "C000010" && promotion.promoted));
        CompositionWork {
            opportunity: CapabilityOpportunityIR {
                observed_gap: "no executable composite exists for the requested relation"
                    .to_string(),
                desired_behavior: "compose and execute the requested multi-stage relation"
                    .to_string(),
                trigger: "recursive director selects a capability-construction bottleneck"
                    .to_string(),
                evidence: vec!["typed composition task remains unsatisfied".to_string()],
                preserved_behavior: vec![
                    "do not change verifier, installer, or acceptance policy".to_string()
                ],
                resource_constraints: vec!["activate only the task-local ProgramIR".to_string()],
                verification_requirements: vec![
                    "type/effect audit".to_string(),
                    "fresh semantic cases".to_string(),
                    "regression suite".to_string(),
                ],
                provenance: vec!["SEM5_EXISTING_SYNTHESIZER".to_string()],
                operator_selected_implementation: false,
            },
            task: sets.adversarial[0].visible.clone(),
            promotions,
            predecessor_tree_hash: AUTHORITATIVE_PREDECESSOR.to_string(),
        }
    }

    fn recursive_request() -> PostScaffoldEpochRequest {
        PostScaffoldEpochRequest {
            arm_code: 3,
            epoch: 1,
            seed: 0x2701,
            state: PostScaffoldState::from_sem26(DirectorState::frozen_sem25()),
            resource_ceiling_bytes: 2_000_000,
            historical_roadmap_target_code: None,
            disable_long_term_research_memory: false,
            concrete_future_instance_visible: false,
        }
    }

    #[test]
    fn existing_sem5_primitives_build_executable_composite_candidate() {
        let work = composition_work();
        let task = work.task.clone();
        let candidate = compose_existing_sem5_capability(work).expect("compose existing SEM-5");
        assert!(candidate.type_effect_audit_pass);
        assert!(candidate.recombinations > 0);
        assert!(candidate.used_primitive_ids.len() >= 2);
        assert!(candidate.primitive_expanded_nodes > candidate.operational_nodes);
        assert!(candidate
            .promoted_concept_ids
            .contains(&"C000010".to_string()));
        assert!(!candidate.patch_candidate.core_self_approved);
        assert!(!candidate.installed);
        assert_eq!(candidate.full_source_scan_events, 0);

        for inputs in generate_property_cases(&task, 0xC05E).into_iter().take(3) {
            let expected = evaluate_contract(&task, &inputs).expect("contract");
            let actual = execute(
                &candidate.program_ir,
                &inputs,
                &task.definitions,
                BTreeMap::new(),
            )
            .expect("execute composite");
            assert_eq!(actual.value, expected);
        }
    }

    #[test]
    fn recursive_epoch_and_composition_share_one_bounded_result() {
        let result = run_integrated_development_epoch(IntegratedDevelopmentEpochRequest {
            recursive_epoch: recursive_request(),
            composition_work: Some(composition_work()),
        })
        .expect("integrated epoch");
        assert_eq!(result.recursive_epoch.epoch, 1);
        assert!(result.composition_attempted);
        assert!(result.composite_candidate.is_some());
        assert_eq!(result.core_self_approval_events, 0);
        assert_eq!(result.unverified_install_events, 0);
        assert_eq!(result.full_source_scan_events, 0);
    }

    #[test]
    fn composite_candidate_still_requires_independent_rsi_receipt() {
        let candidate = compose_existing_sem5_capability(composition_work()).expect("candidate");
        let receipt = VerificationReceipt {
            patch_sha256: candidate.patch_candidate.unified_diff_sha256.clone(),
            repair_spec_sha256: candidate.repair_spec.sha256.clone(),
            defect_contract_sha256: candidate.defect_contract.sha256.clone(),
            semantic_checks_sha256: "semantic-checks".to_string(),
            regression_checks_sha256: "regression-checks".to_string(),
            resource_checks_sha256: "resource-checks".to_string(),
            invariant_checks_sha256: "invariant-checks".to_string(),
            decision: VerificationDecision::Accept,
            verifier_identity: "independent-verifier".to_string(),
            verifier_is_proposer: false,
            gold_patch_text_equality_is_authority: false,
            receipt_sha256: "receipt".to_string(),
            authority_seal: "external-authority-seal".to_string(),
        };
        assert_eq!(
            validate_composite_installation_authority(&candidate, &receipt),
            Ok(())
        );

        let mut self_receipt = receipt;
        self_receipt.verifier_is_proposer = true;
        assert_eq!(
            validate_composite_installation_authority(&candidate, &self_receipt),
            Err(IntegratedInstallationGateError::RsiGate(
                InstallationGateError::ProposerVerifierCollision
            ))
        );
    }
}
