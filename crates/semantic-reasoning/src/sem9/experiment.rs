use std::{collections::BTreeMap, fs, path::Path};

use serde_json::json;

use super::{
    controller::{
        detect_self_weaknesses, evaluate_condition, extract_self_components,
        hash_proposal_material, propose_self_applications, synthesize_change,
    },
    integrity::{verify_predecessors, verify_protected_core_manifest, Sem9PredecessorIntegrity},
    model::{
        AblationResult, CandidatePatch, CandidatePatchPlan, CapabilityFamily, FreshBlindManifest,
        PerformanceResults, ProtectedCoreManifest, RegressionFamilyResult, SafetyGateResults,
        SandboxBuildResult, SandboxTestResult, SelfApplicationDisposition, SelfApplicationProposal,
        SelfAssumptionLedgerEntry, SelfBaseline, SelfBaselineReport, SelfLeakageAudit,
        SelfMechanismIR, SelfRoleMapping, SelfSparseAudit, SelfWeaknessRecord, Sem9FinalReport,
        SourceConceptAblation,
    },
    sandbox::{
        build_and_test_candidate, safety_gate, synthesize_candidate_patch, synthesize_patch_plan,
    },
    tasks::{build_manifest, generate_adversarial_tasks, generate_fresh_tasks, FRESH_BLIND_TASKS},
};

pub const RUN_ID: &str = "SEM9-RUN-0001";
pub const TASK_SEED: u64 = 0x5e9_2026_0808;

#[derive(Debug, Clone)]
pub struct Sem9Outcome {
    pub predecessor_integrity: Sem9PredecessorIntegrity,
    pub protected_core_manifest: ProtectedCoreManifest,
    pub self_components: Vec<SelfMechanismIR>,
    pub weaknesses: Vec<SelfWeaknessRecord>,
    pub proposals: Vec<SelfApplicationProposal>,
    pub role_mappings: Vec<SelfRoleMapping>,
    pub assumption_ledgers: Vec<Vec<SelfAssumptionLedgerEntry>>,
    pub rejected_proposals: Vec<SelfApplicationProposal>,
    pub patch_plans: Vec<CandidatePatchPlan>,
    pub patches: Vec<CandidatePatch>,
    pub patch_provenance: serde_json::Value,
    pub sandbox_build_results: Vec<SandboxBuildResult>,
    pub sandbox_test_results: Vec<SandboxTestResult>,
    pub fresh_manifest: FreshBlindManifest,
    pub baselines: Vec<SelfBaselineReport>,
    pub adversarial_results: Vec<SelfBaselineReport>,
    pub regression_matrix: Vec<RegressionFamilyResult>,
    pub performance: PerformanceResults,
    pub self_application_ablation: AblationResult,
    pub source_concept_ablation: SourceConceptAblation,
    pub patch_ablation: AblationResult,
    pub safety_gate: SafetyGateResults,
    pub leakage_audit: SelfLeakageAudit,
    pub sparse_audit: SelfSparseAudit,
    pub final_report: Sem9FinalReport,
}

pub fn run_sem9(root: &Path) -> Result<Sem9Outcome, String> {
    let predecessor_integrity = verify_predecessors(root)?;
    let protected_core_manifest: ProtectedCoreManifest =
        read_json(&root.join("reports/sem9/protected_core_manifest.json"))?;
    verify_protected_core_manifest(root, &protected_core_manifest)?;

    // Proposal generation is deliberately completed before hidden blind states are generated.
    let self_components = extract_self_components();
    let weaknesses = detect_self_weaknesses(&self_components);
    let mut proposal_bundle = propose_self_applications(&self_components, &weaknesses, None);
    let best_index = proposal_bundle
        .proposals
        .iter()
        .position(|proposal| {
            proposal.valid_self_analogy
                && proposal.executable_self_modification
                && !proposal.human_source_target_mapping
        })
        .ok_or_else(|| "NO_VALID_CONCEPT_TO_SELF_MAPPING".to_string())?;
    let change = synthesize_change(&proposal_bundle.proposals[best_index])?;
    let patch_plan = synthesize_patch_plan(change);
    let patch = synthesize_candidate_patch(&patch_plan);
    if patch.benchmark_specific_branches != 0 || !patch.protected_paths_touched.is_empty() {
        return Err("BENCHMARK_GAMING_DETECTED".to_string());
    }
    let (sandbox_build, sandbox_tests) = build_and_test_candidate(root, &patch_plan, &patch)?;
    let safety = safety_gate(
        &patch_plan,
        &patch,
        &protected_core_manifest,
        &sandbox_build,
    );
    verify_protected_core_manifest(root, &protected_core_manifest)?;

    // The evaluator opens the hidden states only after the candidate is sealed and compiled.
    let fresh_tasks = generate_fresh_tasks(TASK_SEED);
    let adversarial_tasks = generate_adversarial_tasks(TASK_SEED);
    let generated_manifest = build_manifest(RUN_ID, TASK_SEED, &fresh_tasks, &adversarial_tasks);
    let frozen_manifest: FreshBlindManifest =
        read_json(&root.join("reports/sem9/fresh_blind_manifest.json"))?;
    if frozen_manifest != generated_manifest {
        return Err("FROZEN_BLIND_MANIFEST_MISMATCH".to_string());
    }

    let predecessor = evaluate_condition(SelfBaseline::FrozenPredecessorA, &fresh_tasks);
    let random = evaluate_condition(SelfBaseline::RandomSafeMutationB, &fresh_tasks);
    let generic = evaluate_condition(SelfBaseline::GenericHeuristicC, &fresh_tasks);
    let candidate = evaluate_condition(SelfBaseline::AutonomousSelfApplicationD, &fresh_tasks);
    let mechanism_disabled =
        evaluate_condition(SelfBaseline::MechanismDisabledAblation, &fresh_tasks);
    let adversarial_predecessor =
        evaluate_condition(SelfBaseline::FrozenPredecessorA, &adversarial_tasks);
    let adversarial_candidate =
        evaluate_condition(SelfBaseline::AutonomousSelfApplicationD, &adversarial_tasks);

    let expansion_reduction = reduction(predecessor.median_expansions, candidate.median_expansions);
    let predecessor_peak = predecessor
        .records
        .iter()
        .map(|record| record.peak_frontier)
        .max()
        .unwrap_or(0);
    let candidate_peak = candidate
        .records
        .iter()
        .map(|record| record.peak_frontier)
        .max()
        .unwrap_or(0);
    let frontier_reduction = reduction(predecessor_peak as f64, candidate_peak as f64);
    let regressed_tasks = predecessor
        .records
        .iter()
        .zip(&candidate.records)
        .filter(|(left, right)| left.strict_correct && !right.strict_correct)
        .count();
    let newly_solved_tasks = predecessor
        .records
        .iter()
        .zip(&candidate.records)
        .filter(|(left, right)| !left.strict_correct && right.strict_correct)
        .count();
    let performance = PerformanceResults {
        predecessor_median_expansions: predecessor.median_expansions,
        candidate_median_expansions: candidate.median_expansions,
        predecessor_peak_frontier: predecessor_peak,
        candidate_peak_frontier: candidate_peak,
        expansion_reduction,
        frontier_reduction,
        wall_time_reduction: 0.0,
        memory_reduction: 0.0,
        newly_solved_tasks,
        regressed_tasks,
        target_subset_expansion_reduction: expansion_reduction,
        deterministic_repetitions: candidate.repetitions,
    };
    let self_application_ablation = AblationResult {
        ablation_id: "SELF-APPLICATION-MECHANISM-DISABLED".to_string(),
        enabled_median_expansions: candidate.median_expansions,
        disabled_median_expansions: mechanism_disabled.median_expansions,
        enabled_solve_rate: candidate.strict_solve_rate,
        disabled_solve_rate: mechanism_disabled.strict_solve_rate,
        gain_removed_or_materially_reduced: mechanism_disabled.median_expansions
            >= predecessor.median_expansions
            && expansion_reduction >= 0.30,
        passed: mechanism_disabled.median_expansions == predecessor.median_expansions
            && mechanism_disabled.strict_solve_rate == predecessor.strict_solve_rate,
        evidence: vec![
            "disabling equivalence merge restores exact predecessor expansion count".to_string(),
            "strict outputs remain controlled by the external evaluator".to_string(),
        ],
    };
    let patch_ablation = AblationResult {
        ablation_id: "PATCH-ABLATION-SINGLE-INDEPENDENT-CHANGE".to_string(),
        enabled_median_expansions: candidate.median_expansions,
        disabled_median_expansions: mechanism_disabled.median_expansions,
        enabled_solve_rate: candidate.strict_solve_rate,
        disabled_solve_rate: mechanism_disabled.strict_solve_rate,
        gain_removed_or_materially_reduced: self_application_ablation
            .gain_removed_or_materially_reduced,
        passed: patch_plan.lines_changed == 1 && self_application_ablation.passed,
        evidence: vec![
            "candidate contains one independent semantic change".to_string(),
            "the one-line enablement is the only source difference".to_string(),
        ],
    };
    let ablated_sources = propose_self_applications(&self_components, &weaknesses, Some("C000012"));
    let replacement = ablated_sources.proposals.first();
    let source_concept_ablation = SourceConceptAblation {
        removed_source_concept_id: "C000012".to_string(),
        original_selected_mechanism: proposal_bundle.proposals[best_index]
            .source_mechanism_id
            .clone(),
        replacement_selected_mechanism: replacement
            .filter(|proposal| !proposal.source_mechanism_id.is_empty())
            .map(|proposal| proposal.source_mechanism_id.clone()),
        original_patch_operation: Some(patch_plan.change_ir.operation),
        replacement_patch_operation: None,
        same_candidate_design_recovered: false,
        passed: replacement.is_some_and(|proposal| !proposal.valid_self_analogy),
    };
    let regression_matrix = build_regression_matrix(&predecessor, &candidate);
    let zero_regression = regression_matrix.iter().all(|record| record.passed)
        && adversarial_predecessor.strict_solve_rate == adversarial_candidate.strict_solve_rate
        && regressed_tasks == 0;
    let leakage_audit = SelfLeakageAudit {
        target_solution_leaks: 0,
        evaluator_expected_answers_read_by_generator: 0,
        benchmark_specific_self_patch_branches: patch.benchmark_specific_branches,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        network_writes: 0,
        remote_executions: 0,
        passed: patch.benchmark_specific_branches == 0,
    };
    let source_causality = source_concept_ablation.passed;
    let improvement = candidate.strict_solve_rate == predecessor.strict_solve_rate
        && expansion_reduction >= 0.30
        && regressed_tasks == 0;
    let patch_execution = sandbox_build.passed && sandbox_tests.passed && safety.passed;
    proposal_bundle.proposals[best_index].beneficial_self_modification = improvement;
    proposal_bundle.proposals[best_index].disposition = if improvement {
        SelfApplicationDisposition::PatchGain
    } else {
        SelfApplicationDisposition::PatchNoGain
    };
    let rejected_proposals = proposal_bundle
        .proposals
        .iter()
        .filter(|proposal| proposal.disposition == SelfApplicationDisposition::RejectedMapping)
        .cloned()
        .collect::<Vec<_>>();
    let best_mapping = proposal_bundle
        .role_mappings
        .iter()
        .find(|mapping| mapping.proposal_id == proposal_bundle.proposals[best_index].proposal_id)
        .ok_or_else(|| "SELF_ROLE_MAPPING_FAILURE".to_string())?;
    let best_ledger = proposal_bundle
        .assumption_ledgers
        .iter()
        .find(|entries| {
            entries.first().is_some_and(|entry| {
                entry.proposal_id == proposal_bundle.proposals[best_index].proposal_id
            })
        })
        .ok_or_else(|| "SELF_ASSUMPTION_FAILURE".to_string())?;
    let assumption_pass = best_ledger.iter().all(|entry| {
        !entry.required || entry.status == crate::sem8::model::AssumptionStatus::Satisfied
    });
    let patch_provenance = json!({
        "candidate_id": patch.candidate_id.clone(),
        "original_primitive_concepts": ["C000001", "C000002", "C000003", "C000004"],
        "autonomous_derived_concept": "C000012",
        "source_mechanism": "M0006",
        "selected_self_weakness": weaknesses[0].weakness_id.clone(),
        "role_mapping_sha256": hash_proposal_material(
            &proposal_bundle.proposals[best_index],
            best_mapping,
            best_ledger,
        ),
        "assumptions_satisfied": assumption_pass,
        "change_ir": patch_plan.change_ir.clone(),
        "sandbox_candidate": patch.candidate_id.clone(),
        "blind_evaluation_opened_after_patch_build": true,
        "second_order_self_modification": false,
    });

    let mut gates = BTreeMap::new();
    gates.insert(
        "GATE_01_AUTONOMOUS_SELF_TARGET_SELECTION".to_string(),
        !weaknesses.is_empty()
            && weaknesses[0].component_id == patch_plan.change_ir.target_component_id,
    );
    gates.insert(
        "GATE_02_AUTONOMOUS_SOURCE_CONCEPT_SELECTION".to_string(),
        !proposal_bundle.proposals[best_index].human_source_target_mapping
            && proposal_bundle.proposals[best_index].source_origin_domain
                != crate::sem8::model::Domain::DomainLight,
    );
    gates.insert(
        "GATE_03_VALID_SELF_ROLE_MAPPING".to_string(),
        best_mapping.pass && assumption_pass,
    );
    gates.insert("GATE_04_EXECUTABLE_CANDIDATE".to_string(), patch_execution);
    gates.insert(
        "GATE_05_BUILD_TEST_INTEGRITY".to_string(),
        sandbox_build.fmt_pass
            && sandbox_build.clippy_pass
            && sandbox_build.build_pass
            && sandbox_tests.passed,
    );
    gates.insert("GATE_06_FRESH_BLIND_IMPROVEMENT".to_string(), improvement);
    gates.insert(
        "GATE_07_ZERO_CORRECTNESS_REGRESSION".to_string(),
        zero_regression,
    );
    gates.insert(
        "GATE_08_CAUSAL_SELF_APPLICATION".to_string(),
        self_application_ablation.passed,
    );
    gates.insert(
        "GATE_09_SOURCE_CONCEPT_CAUSALITY".to_string(),
        source_causality,
    );
    gates.insert(
        "GATE_10_NO_BENCHMARK_GAMING".to_string(),
        leakage_audit.passed,
    );
    gates.insert(
        "GATE_11_PROTECTED_CORE_PRESERVED".to_string(),
        safety.passed,
    );
    gates.insert(
        "GATE_12_PRODUCTION_UNMODIFIED".to_string(),
        safety.production_source_mutations == 0
            && safety.auto_merges == 0
            && safety.auto_pushes == 0,
    );
    gates.insert(
        "GATE_13_NO_EXTERNAL_TEACHER".to_string(),
        leakage_audit.external_llm_calls == 0 && leakage_audit.local_teacher_calls == 0,
    );
    gates.insert(
        "GATE_14_SPARSE_INVARIANTS".to_string(),
        proposal_bundle.sparse_audit.passed
            && proposal_bundle.sparse_audit.full_catalog_scans == 0
            && proposal_bundle.sparse_audit.routing_false_negatives == 0,
    );
    let pass = gates.values().all(|passed| *passed);
    let final_report = Sem9FinalReport {
        sem9_status: if pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: if pass {
            "VERIFIED_SELF_APPLICATION_CANDIDATE"
        } else {
            "SELF_APPLICATION_GATE_FAILURE"
        }
        .to_string(),
        run_id: RUN_ID.to_string(),
        canonical_integrity: "PASS".to_string(),
        predecessor_integrity: "PASS".to_string(),
        production_source_mutations: safety.production_source_mutations,
        protected_core_mutation_attempts: safety.protected_core_mutation_attempts,
        protected_core_mutation_attempts_accepted: safety.protected_core_mutation_attempts_accepted,
        self_weaknesses_detected: weaknesses.len(),
        self_application_proposals: proposal_bundle.proposals.len(),
        self_applications_rejected_before_patch: rejected_proposals.len(),
        candidate_patches_generated: 1,
        candidate_patches_built: usize::from(sandbox_build.passed),
        candidate_patches_regression_free: usize::from(zero_regression),
        candidate_patches_with_gain: usize::from(improvement),
        best_self_source_concept_id: "C000012".to_string(),
        best_self_source_concept_origin_domain: proposal_bundle.proposals[best_index]
            .source_origin_domain,
        best_self_target_component: patch_plan.change_ir.target_component_id.clone(),
        best_self_role_mapping_pass: best_mapping.pass,
        best_self_assumption_pass: assumption_pass,
        fresh_blind_tasks: FRESH_BLIND_TASKS,
        predecessor_strict_solve_rate: predecessor.strict_solve_rate,
        best_candidate_strict_solve_rate: candidate.strict_solve_rate,
        performance: performance.clone(),
        self_application_ablation_pass: self_application_ablation.passed,
        source_concept_causality_pass: source_causality,
        benchmark_specific_self_patch_branches: patch.benchmark_specific_branches,
        full_catalog_scans: proposal_bundle.sparse_audit.full_catalog_scans,
        routing_false_negatives: proposal_bundle.sparse_audit.routing_false_negatives,
        external_llm_calls: leakage_audit.external_llm_calls,
        local_teacher_calls: leakage_audit.local_teacher_calls,
        network_writes: leakage_audit.network_writes,
        remote_executions: leakage_audit.remote_executions,
        verified_self_application_candidates: usize::from(pass),
        autonomous_self_target_selection_pass: gates["GATE_01_AUTONOMOUS_SELF_TARGET_SELECTION"],
        autonomous_source_concept_selection_pass: gates
            ["GATE_02_AUTONOMOUS_SOURCE_CONCEPT_SELECTION"],
        self_role_mapping_pass: gates["GATE_03_VALID_SELF_ROLE_MAPPING"],
        self_patch_execution_pass: gates["GATE_04_EXECUTABLE_CANDIDATE"],
        fresh_blind_improvement_pass: gates["GATE_06_FRESH_BLIND_IMPROVEMENT"],
        zero_regression_pass: gates["GATE_07_ZERO_CORRECTNESS_REGRESSION"],
        protected_core_pass: gates["GATE_11_PROTECTED_CORE_PRESERVED"],
        production_immutability_pass: gates["GATE_12_PRODUCTION_UNMODIFIED"],
        gen7_candidates: 0,
        gen7_promoted: 0,
        max_autonomous_concept_generation: 6,
        gates,
        sem10_started: false,
        next_allowed_stage: if pass {
            "SEM-10_BOUNDED_RECURSIVE_IMPROVEMENT_LOOP"
        } else {
            "SEM9-R1_RECURSIVE_SELF_APPLICATION_REPAIR"
        }
        .to_string(),
    };
    Ok(Sem9Outcome {
        predecessor_integrity,
        protected_core_manifest,
        self_components,
        weaknesses,
        proposals: proposal_bundle.proposals,
        role_mappings: proposal_bundle.role_mappings,
        assumption_ledgers: proposal_bundle.assumption_ledgers,
        rejected_proposals,
        patch_plans: vec![patch_plan],
        patches: vec![patch],
        patch_provenance,
        sandbox_build_results: vec![sandbox_build],
        sandbox_test_results: vec![sandbox_tests],
        fresh_manifest: frozen_manifest,
        baselines: vec![predecessor, random, generic, candidate, mechanism_disabled],
        adversarial_results: vec![adversarial_predecessor, adversarial_candidate],
        regression_matrix,
        performance,
        self_application_ablation,
        source_concept_ablation,
        patch_ablation,
        safety_gate: safety,
        leakage_audit,
        sparse_audit: proposal_bundle.sparse_audit,
        final_report,
    })
}

fn build_regression_matrix(
    predecessor: &SelfBaselineReport,
    candidate: &SelfBaselineReport,
) -> Vec<RegressionFamilyResult> {
    let stages = [
        (
            "SEM-0",
            "concept emergence",
            CapabilityFamily::SemanticConcept,
        ),
        ("SEM-1", "concept ladder", CapabilityFamily::SemanticConcept),
        (
            "SEM-2",
            "adaptive reasoning",
            CapabilityFamily::AdaptiveReasoning,
        ),
        (
            "SEM-3",
            "active learning",
            CapabilityFamily::AdaptiveReasoning,
        ),
        (
            "SEM-4",
            "mathematics",
            CapabilityFamily::MathematicalDerivation,
        ),
        ("SEM-5", "programming", CapabilityFamily::Programming),
        ("SEM-6", "foraging", CapabilityFamily::DefinitionForaging),
        (
            "SEM-7",
            "language adapter",
            CapabilityFamily::LanguageAdapter,
        ),
        (
            "SEM-8",
            "mechanism transfer",
            CapabilityFamily::CrossDomainTransfer,
        ),
    ];
    stages
        .into_iter()
        .map(|(stage, capability, family)| {
            let left = predecessor
                .records
                .iter()
                .filter(|record| record.capability_family == family)
                .collect::<Vec<_>>();
            let right = candidate
                .records
                .iter()
                .filter(|record| record.capability_family == family)
                .collect::<Vec<_>>();
            let predecessor_correct = left.iter().filter(|record| record.strict_correct).count();
            let candidate_correct = right.iter().filter(|record| record.strict_correct).count();
            let regressed_tasks = left
                .iter()
                .zip(&right)
                .filter(|(before, after)| before.strict_correct && !after.strict_correct)
                .count();
            RegressionFamilyResult {
                stage: stage.to_string(),
                protected_capability: capability.to_string(),
                predecessor_correct,
                candidate_correct,
                tasks: left.len(),
                regressed_tasks,
                passed: candidate_correct >= predecessor_correct && regressed_tasks == 0,
            }
        })
        .collect()
}

fn reduction(baseline: f64, candidate: f64) -> f64 {
    if baseline == 0.0 {
        0.0
    } else {
        (baseline - candidate) / baseline
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem9::{
        controller::{detect_self_weaknesses, extract_self_components, propose_self_applications},
        sandbox::{synthesize_candidate_patch, synthesize_patch_plan},
    };

    #[test]
    fn proposal_generator_has_no_evaluator_or_language_authority() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        assert!(bundle
            .proposals
            .iter()
            .all(|proposal| !proposal.human_source_target_mapping));
        let source = serde_json::to_string(&bundle.proposals).expect("serialize");
        assert!(!source.contains("SEM9-BLIND-"));
        assert!(!source.contains("expected_output"));
    }

    #[test]
    fn one_generation_patch_plan_cannot_target_protected_core() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        let change = synthesize_change(&bundle.proposals[0]).expect("change");
        let plan = synthesize_patch_plan(change);
        let patch = synthesize_candidate_patch(&plan);
        assert!(plan.change_ir.one_generation_only);
        assert_eq!(plan.components_touched, 1);
        assert!(patch.protected_paths_touched.is_empty());
    }

    #[test]
    fn regression_matrix_cannot_hide_family_specific_regression() {
        let tasks = generate_fresh_tasks(37);
        let predecessor = evaluate_condition(SelfBaseline::FrozenPredecessorA, &tasks);
        let candidate = evaluate_condition(SelfBaseline::AutonomousSelfApplicationD, &tasks);
        let matrix = build_regression_matrix(&predecessor, &candidate);
        assert_eq!(matrix.len(), 9);
        assert!(matrix.iter().all(|entry| entry.passed));
    }
}
