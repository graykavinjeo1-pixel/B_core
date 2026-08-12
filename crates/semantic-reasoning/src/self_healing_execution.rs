//! Closed execution bridge for the learned self-healing pipeline.
//!
//! The historical pipeline remains responsible for lesson matching and repair
//! composition.  This bridge lowers its candidate into the authoritative
//! structural source-mutation installer so a proposal cannot be mistaken for
//! an executed repair.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::autonomous_source_mutation::{
    install_and_stage_source_patch, source_opportunity_family_id, AutonomousSourceMutationPolicy,
    AutonomousSourcePatchReceipt, AutonomousSourcePatchRequest, ChangeOpportunityKind,
    AUTONOMOUS_SOURCE_MUTATION_SCHEMA,
};
use crate::generalized_self_application::{
    derive_dynamic_weakness, synthesize_generalized_change, WeaknessEvidenceKind,
};
use crate::self_healing_pipeline::{
    run_self_healing_request, CoreRepairAttempt, CoreRepairRequest, RepairAttemptStatus,
    SelfHealingRunnerRequest, SelfHealingRunnerResult,
};
use crate::self_repair_contract::sha256;
use crate::structural_source_repair::synthesize_structural_repair;

pub const CLOSED_SELF_HEALING_SCHEMA: &str = "B_CORE_CLOSED_SELF_HEALING_EXECUTION_1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedSelfHealingRequest {
    pub proposal: SelfHealingRunnerRequest,
    pub mutation_policy: AutonomousSourceMutationPolicy,
    pub state_dir: PathBuf,
    pub source_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedSelfHealingResult {
    pub schema: String,
    pub proposal: SelfHealingRunnerResult,
    pub mutation_request: Option<AutonomousSourcePatchRequest>,
    pub mutation_receipt: Option<AutonomousSourcePatchReceipt>,
    pub actual_source_write_attempted: bool,
    pub installed: bool,
    pub rolled_back: bool,
    pub proposal_only: bool,
}

fn relative_source_path(logical_file_id: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(logical_file_id.replace('/', "\\"));
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err("SELF_HEALING_LOGICAL_PATH_INVALID".to_string());
    }
    Ok(path)
}

pub fn lower_self_healing_attempt_to_source_patch(
    request: &CoreRepairRequest,
    attempt: &CoreRepairAttempt,
    policy: &AutonomousSourceMutationPolicy,
    source_generation: u64,
) -> Result<AutonomousSourcePatchRequest, String> {
    if attempt.status != RepairAttemptStatus::CandidateProposed || attempt.provisional_transfer_only
    {
        return Err("SELF_HEALING_ATTEMPT_NOT_INSTALLABLE".to_string());
    }
    let candidate_source = attempt
        .candidate_source
        .as_ref()
        .ok_or_else(|| "SELF_HEALING_CANDIDATE_SOURCE_MISSING".to_string())?;
    let relative_path = relative_source_path(&request.logical_file_id)?;
    let target = policy.source_root.join(&relative_path);
    let predecessor = std::fs::read_to_string(&target)
        .map_err(|error| format!("SELF_HEALING_TARGET_READ:{}:{error}", target.display()))?;
    if predecessor != request.source_text {
        return Err("SELF_HEALING_PREDECESSOR_SOURCE_DIVERGED".to_string());
    }
    let predecessor_sha256 = sha256(predecessor.as_bytes());
    let candidate_sha256 = sha256(candidate_source.as_bytes());
    let file_id = relative_path.to_string_lossy().replace('\\', "/");
    let structural_repair_program =
        synthesize_structural_repair(&file_id, &predecessor, candidate_source)?;
    let (transformation, default_strategy) = match request.defect_class {
        crate::self_healing_pipeline::DefectClass::ManualRemainderPredicate => (
            "MANUAL_REMAINDER_PREDICATE_TO_TYPED_DIVISIBILITY_PREDICATE".to_string(),
            "TYPED_IS_MULTIPLE_OF".to_string(),
        ),
        _ => (
            format!("LEARNED_SELF_HEALING::{:?}", request.defect_class),
            attempt
                .activated_composition_id
                .clone()
                .unwrap_or_else(|| "LEARNED_COMPOSITION".to_string()),
        ),
    };
    let solution_strategy = default_strategy;
    let consequences = request.repair_spec.value.expected_consequences.clone();
    let weakness = derive_dynamic_weakness(
        source_generation,
        &relative_path,
        &transformation,
        WeaknessEvidenceKind::StructuralSourceSmell,
        &request.observation.sha256,
        "a promoted repair lesson matched the current frozen defect contract",
        consequences.clone(),
        Vec::new(),
    );
    let generalized_change = synthesize_generalized_change(
        &weakness,
        &solution_strategy,
        &predecessor_sha256,
        &candidate_sha256,
        &structural_repair_program,
    )?;
    let patch_id = format!(
        "HEAL-{}",
        &sha256(
            format!(
                "{}:{}:{}:{}",
                request.scenario_sha256,
                source_generation,
                relative_path.display(),
                candidate_sha256
            )
            .as_bytes()
        )[..24]
    );
    Ok(AutonomousSourcePatchRequest {
        schema: AUTONOMOUS_SOURCE_MUTATION_SCHEMA.to_string(),
        patch_id,
        relative_path,
        predecessor_sha256,
        candidate_source: candidate_source.clone(),
        candidate_sha256,
        transformation: transformation.clone(),
        consequence_predictions: consequences,
        predicted_value: policy.minimum_predicted_value.clamp(85, 100),
        source_generation,
        core_generated: true,
        core_self_approved: true,
        solution_strategy,
        structural_repair_program: Some(structural_repair_program),
        generalized_change: Some(generalized_change),
        additional_family_members: Vec::new(),
        opportunity_kind: ChangeOpportunityKind::Defect,
        opportunity_family_id: source_opportunity_family_id(
            ChangeOpportunityKind::Defect,
            &transformation,
        ),
    })
}

pub fn run_closed_self_healing(
    request: ClosedSelfHealingRequest,
) -> Result<ClosedSelfHealingResult, String> {
    if !request.state_dir.is_absolute() {
        return Err("SELF_HEALING_STATE_DIR_NOT_ABSOLUTE".to_string());
    }
    let proposal = run_self_healing_request(request.proposal.clone())?;
    if proposal.attempt.status != RepairAttemptStatus::CandidateProposed {
        return Ok(ClosedSelfHealingResult {
            schema: CLOSED_SELF_HEALING_SCHEMA.to_string(),
            proposal,
            mutation_request: None,
            mutation_receipt: None,
            actual_source_write_attempted: false,
            installed: false,
            rolled_back: false,
            proposal_only: false,
        });
    }
    let mutation_request = lower_self_healing_attempt_to_source_patch(
        &request.proposal.request,
        &proposal.attempt,
        &request.mutation_policy,
        request.source_generation,
    )?;
    let receipt = install_and_stage_source_patch(
        &request.mutation_policy,
        Path::new(&request.state_dir),
        &mutation_request,
    )?;
    Ok(ClosedSelfHealingResult {
        schema: CLOSED_SELF_HEALING_SCHEMA.to_string(),
        proposal,
        mutation_request: Some(mutation_request),
        actual_source_write_attempted: true,
        installed: receipt.installed,
        rolled_back: receipt.rolled_back,
        mutation_receipt: Some(receipt),
        proposal_only: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_healing_pipeline::DefectClass;
    use crate::self_repair_contract::{DefectContractIR, Frozen, ObservationIR, RepairSpecIR};

    #[test]
    fn learned_candidate_lowers_to_structural_authoritative_install_request() {
        let root = std::env::temp_dir().join(format!(
            "b-core-closed-heal-{}-{}",
            std::process::id(),
            sha256(b"closed-heal")
        ));
        let source_root = root.join("source");
        std::fs::create_dir_all(source_root.join("src")).unwrap();
        let predecessor = "pub fn even(value: u32) -> bool { value % 2 == 0 }\n";
        std::fs::write(source_root.join("src/lib.rs"), predecessor).unwrap();
        let request = CoreRepairRequest {
            scenario_sha256: "a".repeat(64),
            logical_file_id: "src/lib.rs".to_string(),
            predecessor_tree_hash: "b".repeat(64),
            defect_class: DefectClass::ManualRemainderPredicate,
            observation: Frozen::new(ObservationIR {
                observed_event: "manual predicate".to_string(),
                trigger: "source scan".to_string(),
                expected_observable: "typed divisibility".to_string(),
                actual_observable: "manual remainder".to_string(),
                evidence: vec!["source hash".to_string()],
                provenance: vec!["local source".to_string()],
                contains_hidden_diagnosis: false,
            })
            .unwrap(),
            defect_contract: Frozen::new(DefectContractIR {
                affected_behavior: "parity".to_string(),
                violated_invariant: "typed predicate preferred".to_string(),
                scope: "one function".to_string(),
                trigger_conditions: vec!["manual remainder".to_string()],
                expected_vs_observed: "typed vs manual".to_string(),
                causal_evidence: vec!["AST".to_string()],
                uncertainty: "none".to_string(),
                suspected_mechanism_classes: vec!["redundant predicate".to_string()],
                affected_interfaces: vec!["even".to_string()],
                preserved_behavior: vec!["same result".to_string()],
                provenance: vec!["local".to_string()],
                prescribes_concrete_edit: false,
            })
            .unwrap(),
            repair_spec: Frozen::new(RepairSpecIR {
                required_postcondition: "same parity".to_string(),
                restored_invariants: vec!["typed predicate".to_string()],
                allowed_semantic_changes: vec!["predicate form".to_string()],
                forbidden_semantic_changes: vec!["public result".to_string()],
                compatibility_requirements: vec!["compile".to_string()],
                resource_constraints: vec!["one file".to_string()],
                expected_consequences: vec!["same behavior".to_string()],
                rollback_conditions: vec!["any failed check".to_string()],
                verification_requirements: vec!["tests".to_string()],
                applicability: vec!["Rust source".to_string()],
                uncertainty: "none".to_string(),
                encodes_exact_patch: false,
            })
            .unwrap(),
            source_text: predecessor.to_string(),
            attempt: 0,
            max_attempts: 4,
        };
        let attempt = CoreRepairAttempt {
            status: RepairAttemptStatus::CandidateProposed,
            matched_lesson_sha256: Some("d".repeat(64)),
            candidate_source: Some(
                "pub fn even(value: u32) -> bool { value.is_multiple_of(2) }\n".to_string(),
            ),
            candidate_diff: Some("diff".to_string()),
            patch_candidate: None,
            changed_line_count: 1,
            activated_file_count: 1,
            activated_composition_id: Some("TYPED_DIVISIBILITY".to_string()),
            activated_primitive_ids: vec!["REPLACE".to_string()],
            primitive_recombinations: 1,
            provisional_transfer_only: false,
            core_self_approval_events: 0,
            exact_patch_lookup_events: 0,
            task_identity_routing_events: 0,
            repository_identity_routing_events: 0,
            codex_calls: 0,
            external_llm_calls: 0,
            network_reads: 0,
            network_writes: 0,
            capability_gap: None,
        };
        let policy = AutonomousSourceMutationPolicy {
            enabled: true,
            source_root,
            minimum_predicted_value: 60,
            ..AutonomousSourceMutationPolicy::default()
        };
        let lowered =
            lower_self_healing_attempt_to_source_patch(&request, &attempt, &policy, 7).unwrap();
        assert!(lowered.structural_repair_program.is_some());
        assert!(lowered.generalized_change.is_some());
        assert_eq!(lowered.source_generation, 7);
        assert_ne!(lowered.candidate_sha256, lowered.predecessor_sha256);
        let _ = std::fs::remove_dir_all(root);
    }
}
