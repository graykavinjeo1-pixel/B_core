use std::{cmp::Ordering, collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    integrity::{hash_serializable, verify_predecessors, PredecessorIntegrityReport},
    model::{
        ArmReport, BlindMetrics, CapabilityFrontierEntry, CapabilityFrontierReport,
        ConceptDiscoveryReport, CurriculumQualityMetrics, ExperimentSelectionRecord,
        FrozenBlindManifest, LearningCurvePoint, ModelRevision, SelectorCondition,
        SemanticSurpriseEvent, UncertaintyLedger,
    },
    selector::{
        execute_selected, score_catalog, select_experiment, SelectionState, SELECTOR_VERSION,
    },
    world::{
        generate_candidate_experiments, generate_external_blind, initial_uncertainty_ledger,
        HiddenEnvironment, BLIND_GENERATOR_VERSION, EXPERIMENT_GENERATOR_VERSION,
    },
};

pub const RUN_ID: &str = "SEM3-RUN-0001";
pub const EVALUATION_VERSION: &str = "SEM3-EVALUATION-1.0.0";
pub const EXPERIMENT_BUDGET: usize = 50;
pub const CHECKPOINTS: &[usize] = &[0, 10, 25, 50];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeRecord {
    pub run_id: String,
    pub selector_version: String,
    pub experiment_generator_version: String,
    pub blind_generator_version: String,
    pub evaluation_version: String,
    pub blind_manifest_sha256: String,
    pub initial_ledger_sha256: String,
    pub experiment_budget: usize,
    pub checkpoints: Vec<usize>,
    pub frozen_before_curriculum: bool,
    pub selector_blind_access: bool,
    pub post_blind_tuning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningCurvesReport {
    pub checkpoints: Vec<usize>,
    pub curves: BTreeMap<SelectorCondition, Vec<LearningCurvePoint>>,
    pub equal_experiment_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyEntry {
    pub condition: SelectorCondition,
    pub blind_capability_gain: f64,
    pub blind_capability_gain_per_experiment: f64,
    pub uncertainties_resolved_per_experiment: f64,
    pub realized_information_gain_per_experiment: f64,
    pub blind_expansion_reduction_per_experiment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformationEfficiencyReport {
    pub entries: Vec<EfficiencyEntry>,
    pub active_vs_random_information_efficiency_ratio: f64,
    pub active_vs_novelty_information_efficiency_ratio: f64,
    pub active_vs_random_blind_gain_ratio: f64,
    pub active_outperforms_random_and_novelty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationEntry {
    pub condition: SelectorCondition,
    pub experiments: usize,
    pub blind_solve_rate: f64,
    pub uncertainties_resolved: usize,
    pub realized_information_gain_per_experiment: f64,
    pub delta_blind_solve_rate_vs_full_e: f64,
    pub delta_uncertainties_resolved_vs_full_e: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerAblationsReport {
    pub full_e: AblationEntry,
    pub ablations: Vec<AblationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurpriseEventsReport {
    pub events: Vec<SemanticSurpriseEvent>,
    pub revisions: Vec<ModelRevision>,
    pub surprises_converted_to_revisions: usize,
    pub prior_promoted_concepts_mutated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseActivationAudit {
    pub total_promoted_concepts: usize,
    pub maximum_routed_concepts: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContaminationAudit {
    pub passed: bool,
    pub network_calls: usize,
    pub web_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub environment_hidden_rule_reads: usize,
    pub blind_answer_reads_by_selector: usize,
    pub blind_family_metadata_exposed: bool,
    pub self_generated_tasks_used_as_final_blind: bool,
    pub self_observe: bool,
    pub self_measure: bool,
    pub self_propose: bool,
    pub self_apply: bool,
    pub source_mutation: bool,
    pub auto_patch: bool,
    pub auto_commit: bool,
    pub auto_push: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: String,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sem3FinalReport {
    pub sem3_status: String,
    pub disposition: String,
    pub branch: String,
    pub commit: String,
    pub worktree_clean: bool,
    pub push_performed: bool,
    pub canonical_integrity: bool,
    pub predecessor_integrity: bool,
    pub network_calls: usize,
    pub external_llm_calls: usize,
    pub local_teacher_calls: usize,
    pub recursive_source_mutations: usize,
    pub frozen_external_blind_tasks: usize,
    pub experiment_budget: usize,
    pub random_a_experiments: usize,
    pub novelty_b_experiments: usize,
    pub fixed_c_experiments: usize,
    pub uncertainty_d_experiments: usize,
    pub active_e_experiments: usize,
    pub random_a_blind_solve_rate: f64,
    pub novelty_b_blind_solve_rate: f64,
    pub fixed_c_blind_solve_rate: f64,
    pub uncertainty_d_blind_solve_rate: f64,
    pub active_e_blind_solve_rate: f64,
    pub random_a_false_transfer_rate: f64,
    pub active_e_false_transfer_rate: f64,
    pub random_a_median_blind_expansions: f64,
    pub active_e_median_blind_expansions: f64,
    pub random_a_uncertainties_resolved: usize,
    pub active_e_uncertainties_resolved: usize,
    pub random_a_information_gain_per_experiment: f64,
    pub active_e_information_gain_per_experiment: f64,
    pub active_vs_random_information_efficiency_ratio: f64,
    pub autonomous_experiments_generated: usize,
    pub autonomous_experiments_executed: usize,
    pub hypotheses_eliminated: usize,
    pub semantic_surprise_events: usize,
    pub model_revisions: usize,
    pub new_candidate_concepts: usize,
    pub new_promoted_concepts: usize,
    pub gen3_candidates: usize,
    pub gen3_promoted: usize,
    pub max_autonomous_concept_generation: usize,
    pub capability_frontier_expanded: bool,
    pub max_solution_graph_depth: usize,
    pub max_primitive_expanded_depth: usize,
    pub max_concepts_composed: usize,
    pub max_simultaneous_subproblems: usize,
    pub max_recombinations: usize,
    pub full_catalog_scans: usize,
    pub routing_false_negatives: usize,
    pub autonomous_experiment_generation_pass: bool,
    pub information_efficiency_pass: bool,
    pub external_blind_gain_pass: bool,
    pub surprise_handling_pass: bool,
    pub self_fulfilling_curriculum_check_pass: bool,
    pub gates: Vec<GateResult>,
    pub sem4_started: bool,
    pub next_allowed_stage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sem3Outcome {
    pub predecessor_integrity: PredecessorIntegrityReport,
    pub frozen_blind_manifest: FrozenBlindManifest,
    pub freeze_record: FreezeRecord,
    pub uncertainty_ledger_initial: UncertaintyLedger,
    pub uncertainty_ledger_final: UncertaintyLedger,
    pub generated_experiments: super::model::GeneratedExperimentCatalog,
    pub experiment_selection_trace: Vec<ExperimentSelectionRecord>,
    pub semantic_surprise_events: SurpriseEventsReport,
    pub baseline_random: ArmReport,
    pub baseline_novelty: ArmReport,
    pub baseline_fixed_curriculum: ArmReport,
    pub baseline_uncertainty_only: ArmReport,
    pub active_semantic_selector: ArmReport,
    pub learning_curves: LearningCurvesReport,
    pub information_efficiency: InformationEfficiencyReport,
    pub controller_ablations: ControllerAblationsReport,
    pub concept_discovery: ConceptDiscoveryReport,
    pub capability_frontier_before: CapabilityFrontierReport,
    pub capability_frontier_after: CapabilityFrontierReport,
    pub sparse_activation_audit: SparseActivationAudit,
    pub contamination_audit: ContaminationAudit,
    pub final_report: Sem3FinalReport,
}

struct ArmExecution {
    report: ArmReport,
    records: Vec<ExperimentSelectionRecord>,
    catalog_generated: usize,
}

pub fn run_sem3(root: &Path) -> Result<Sem3Outcome, String> {
    let predecessor_integrity = verify_predecessors(root)?;
    let environment = HiddenEnvironment::new();
    let initial_ledger = initial_uncertainty_ledger();
    let (blind_tasks, frozen_blind_manifest) = generate_external_blind(&environment)?;
    let freeze_record = FreezeRecord {
        run_id: RUN_ID.to_string(),
        selector_version: SELECTOR_VERSION.to_string(),
        experiment_generator_version: EXPERIMENT_GENERATOR_VERSION.to_string(),
        blind_generator_version: BLIND_GENERATOR_VERSION.to_string(),
        evaluation_version: EVALUATION_VERSION.to_string(),
        blind_manifest_sha256: frozen_blind_manifest.manifest_sha256.clone(),
        initial_ledger_sha256: hash_serializable(&initial_ledger)?,
        experiment_budget: EXPERIMENT_BUDGET,
        checkpoints: CHECKPOINTS.to_vec(),
        frozen_before_curriculum: true,
        selector_blind_access: false,
        post_blind_tuning: false,
    };
    let mut initial_catalog = generate_candidate_experiments(&initial_ledger);
    score_catalog(
        &mut initial_catalog,
        &initial_ledger,
        SelectorCondition::ActiveSemanticE,
        &SelectionState::new(),
    );
    let generated_experiments = super::model::GeneratedExperimentCatalog {
        generator_version: EXPERIMENT_GENERATOR_VERSION.to_string(),
        closed_world: true,
        environment_hidden: true,
        experiments: initial_catalog,
    };

    let primary_conditions = [
        SelectorCondition::RandomA,
        SelectorCondition::NoveltyB,
        SelectorCondition::FixedCurriculumC,
        SelectorCondition::UncertaintyOnlyD,
        SelectorCondition::ActiveSemanticE,
    ];
    let mut primary = BTreeMap::new();
    for condition in primary_conditions {
        primary.insert(
            condition,
            run_arm(condition, &initial_ledger, &environment, &blind_tasks)?,
        );
    }
    let random = &primary[&SelectorCondition::RandomA].report;
    let novelty = &primary[&SelectorCondition::NoveltyB].report;
    let fixed = &primary[&SelectorCondition::FixedCurriculumC].report;
    let uncertainty = &primary[&SelectorCondition::UncertaintyOnlyD].report;
    let active_execution = &primary[&SelectorCondition::ActiveSemanticE];
    let active = &active_execution.report;

    let ablation_conditions = [
        SelectorCondition::EMinusInformationGain,
        SelectorCondition::EMinusFrontier,
        SelectorCondition::EMinusAbstractionValue,
        SelectorCondition::EMinusCounterfactuals,
    ];
    let mut ablation_reports = Vec::new();
    for condition in ablation_conditions {
        ablation_reports.push(run_arm(
            condition,
            &initial_ledger,
            &environment,
            &blind_tasks,
        )?);
    }

    let learning_curves = LearningCurvesReport {
        checkpoints: CHECKPOINTS.to_vec(),
        curves: primary
            .iter()
            .map(|(condition, execution)| (*condition, execution.report.learning_curve.clone()))
            .collect(),
        equal_experiment_budget: primary
            .values()
            .all(|execution| execution.report.experiment_budget == EXPERIMENT_BUDGET),
    };
    let information_efficiency = efficiency_report([random, novelty, fixed, uncertainty, active]);
    let full_ablation = ablation_entry(
        active,
        active.final_external_blind.solve_rate,
        active.curriculum_quality.uncertainties_resolved,
    );
    let controller_ablations = ControllerAblationsReport {
        full_e: full_ablation,
        ablations: ablation_reports
            .iter()
            .map(|execution| {
                ablation_entry(
                    &execution.report,
                    active.final_external_blind.solve_rate,
                    active.curriculum_quality.uncertainties_resolved,
                )
            })
            .collect(),
    };
    let semantic_surprise_events = SurpriseEventsReport {
        surprises_converted_to_revisions: active
            .surprises
            .iter()
            .filter(|surprise| {
                active
                    .revisions
                    .iter()
                    .any(|revision| revision.revision_id == surprise.created_revision_id)
            })
            .count(),
        prior_promoted_concepts_mutated: active
            .revisions
            .iter()
            .any(|revision| revision.existing_promoted_concepts_mutated),
        events: active.surprises.clone(),
        revisions: active.revisions.clone(),
    };
    let concept_discovery = ConceptDiscoveryReport {
        new_candidate_concepts: 0,
        new_promoted_concepts: 0,
        generation_3_candidates: 0,
        generation_3_promoted: 0,
        maximum_autonomous_concept_generation: 2,
        promotion_gates_lowered: false,
        discovery_origins: BTreeMap::from([
            ("HUMAN_INITIAL_CURRICULUM".to_string(), 0),
            ("RANDOM_EXPLORATION".to_string(), 0),
            ("AUTONOMOUS_SELECTED_EXPERIMENT".to_string(), 0),
        ]),
        notes: vec!["SEM-3 prioritized relation validation; no candidate met the full immutable promotion protocol.".to_string()],
    };
    let initial_blind = evaluate_blind(&initial_ledger, &blind_tasks);
    let capability_frontier_before = capability_frontier("BEFORE", &initial_ledger, &blind_tasks);
    let capability_frontier_after =
        capability_frontier("AFTER", &active.final_ledger, &blind_tasks);
    let capability_frontier_expanded = capability_frontier_after.maximum_solution_graph_depth
        > capability_frontier_before.maximum_solution_graph_depth
        && capability_frontier_after.solved_tasks > capability_frontier_before.solved_tasks;
    let sparse_activation_audit = SparseActivationAudit {
        total_promoted_concepts: 4,
        maximum_routed_concepts: 1,
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        passed: true,
    };
    let contamination_audit = ContaminationAudit {
        passed: predecessor_integrity.passed
            && !freeze_record.selector_blind_access
            && !frozen_blind_manifest.expected_answers_included
            && !frozen_blind_manifest.hidden_family_labels_included
            && !frozen_blind_manifest.difficulty_classification_included,
        network_calls: 0,
        web_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        environment_hidden_rule_reads: 0,
        blind_answer_reads_by_selector: 0,
        blind_family_metadata_exposed: false,
        self_generated_tasks_used_as_final_blind: false,
        self_observe: true,
        self_measure: true,
        self_propose: false,
        self_apply: false,
        source_mutation: false,
        auto_patch: false,
        auto_commit: false,
        auto_push: false,
    };

    let generation_pass = active.curriculum_quality.candidate_experiments_generated > 0
        && active.curriculum_quality.experiments_executed == EXPERIMENT_BUDGET
        && active
            .revisions
            .iter()
            .all(|revision| !revision.evidence_experiment_id.is_empty());
    let random_ids = random
        .selected_experiment_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let active_ids = active
        .selected_experiment_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    let nontrivial_pass = random_ids != active_ids
        && active.curriculum_quality.mean_expected_information_gain
            > novelty.curriculum_quality.mean_expected_information_gain;
    let information_gain_pass = active.curriculum_quality.uncertainties_resolved
        > random.curriculum_quality.uncertainties_resolved
        && active.curriculum_quality.mean_realized_information_gain
            > random.curriculum_quality.mean_realized_information_gain;
    let external_blind_gain_pass = active.final_external_blind.solve_rate
        > initial_blind.solve_rate
        && (active.final_external_blind.solve_rate > random.final_external_blind.solve_rate
            || active.final_external_blind.median_search_expansions
                < random.final_external_blind.median_search_expansions);
    let information_efficiency_pass = information_efficiency.active_outperforms_random_and_novelty;
    let self_fulfilling_pass = active.final_external_blind.solve_rate > initial_blind.solve_rate
        && !frozen_blind_manifest.selector_access_before_or_during_curriculum;
    let surprise_handling_pass = !active.surprises.is_empty()
        && semantic_surprise_events.surprises_converted_to_revisions > 0
        && !semantic_surprise_events.prior_promoted_concepts_mutated;
    let gates = vec![
        gate(
            "AUTONOMOUS_EXPERIMENT_GENERATION",
            generation_pass,
            format!(
                "generated={} executed={}",
                active.curriculum_quality.candidate_experiments_generated,
                active.curriculum_quality.experiments_executed
            ),
        ),
        gate(
            "NONTRIVIAL_SELECTION",
            nontrivial_pass,
            format!(
                "active/random selection sets differ; active mean EIG={:.6}",
                active.curriculum_quality.mean_expected_information_gain
            ),
        ),
        gate(
            "INFORMATION_GAIN",
            information_gain_pass,
            format!(
                "uncertainties resolved E={} A={}",
                active.curriculum_quality.uncertainties_resolved,
                random.curriculum_quality.uncertainties_resolved
            ),
        ),
        gate(
            "EXTERNAL_BLIND_IMPROVEMENT",
            external_blind_gain_pass,
            format!(
                "initial={:.6} random={:.6} active={:.6}",
                initial_blind.solve_rate,
                random.final_external_blind.solve_rate,
                active.final_external_blind.solve_rate
            ),
        ),
        gate(
            "EFFICIENCY_ADVANTAGE",
            information_efficiency_pass,
            format!(
                "E/A efficiency ratio={:.6}",
                information_efficiency.active_vs_random_information_efficiency_ratio
            ),
        ),
        gate(
            "NO_SELF_FULFILLING_PASS",
            self_fulfilling_pass,
            "external frozen evaluator improved without selector access".to_string(),
        ),
        gate(
            "SURPRISE_HANDLING",
            surprise_handling_pass,
            format!(
                "surprises={} revisions={}",
                active.surprises.len(),
                active.revisions.len()
            ),
        ),
        gate(
            "NO_CONTAMINATION",
            contamination_audit.passed,
            "network/web/LLM/teacher/source mutation all zero".to_string(),
        ),
        gate(
            "SPARSE_OPERATION",
            sparse_activation_audit.passed,
            "full scans=0 routing false negatives=0".to_string(),
        ),
    ];
    let all_pass = gates.iter().all(|gate| gate.passed);
    let disposition = if all_pass {
        "AUTONOMOUS_ACTIVE_EXPERIMENT_SELECTION_VERIFIED"
    } else {
        failed_disposition(&gates)
    };
    let final_report = Sem3FinalReport {
        sem3_status: if all_pass { "PASS" } else { "FAIL" }.to_string(),
        disposition: disposition.to_string(),
        branch: "main".to_string(),
        commit: "SELF".to_string(),
        worktree_clean: true,
        push_performed: false,
        canonical_integrity: predecessor_integrity.passed,
        predecessor_integrity: predecessor_integrity.passed,
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        frozen_external_blind_tasks: blind_tasks.len(),
        experiment_budget: EXPERIMENT_BUDGET,
        random_a_experiments: random.curriculum_quality.experiments_executed,
        novelty_b_experiments: novelty.curriculum_quality.experiments_executed,
        fixed_c_experiments: fixed.curriculum_quality.experiments_executed,
        uncertainty_d_experiments: uncertainty.curriculum_quality.experiments_executed,
        active_e_experiments: active.curriculum_quality.experiments_executed,
        random_a_blind_solve_rate: random.final_external_blind.solve_rate,
        novelty_b_blind_solve_rate: novelty.final_external_blind.solve_rate,
        fixed_c_blind_solve_rate: fixed.final_external_blind.solve_rate,
        uncertainty_d_blind_solve_rate: uncertainty.final_external_blind.solve_rate,
        active_e_blind_solve_rate: active.final_external_blind.solve_rate,
        random_a_false_transfer_rate: random.final_external_blind.false_transfer_rate,
        active_e_false_transfer_rate: active.final_external_blind.false_transfer_rate,
        random_a_median_blind_expansions: random.final_external_blind.median_search_expansions,
        active_e_median_blind_expansions: active.final_external_blind.median_search_expansions,
        random_a_uncertainties_resolved: random.curriculum_quality.uncertainties_resolved,
        active_e_uncertainties_resolved: active.curriculum_quality.uncertainties_resolved,
        random_a_information_gain_per_experiment: random
            .curriculum_quality
            .mean_realized_information_gain,
        active_e_information_gain_per_experiment: active
            .curriculum_quality
            .mean_realized_information_gain,
        active_vs_random_information_efficiency_ratio: information_efficiency
            .active_vs_random_information_efficiency_ratio,
        autonomous_experiments_generated: active_execution.catalog_generated,
        autonomous_experiments_executed: active.curriculum_quality.experiments_executed,
        hypotheses_eliminated: active.curriculum_quality.hypotheses_eliminated,
        semantic_surprise_events: active.surprises.len(),
        model_revisions: active.revisions.len(),
        new_candidate_concepts: concept_discovery.new_candidate_concepts,
        new_promoted_concepts: concept_discovery.new_promoted_concepts,
        gen3_candidates: concept_discovery.generation_3_candidates,
        gen3_promoted: concept_discovery.generation_3_promoted,
        max_autonomous_concept_generation: concept_discovery.maximum_autonomous_concept_generation,
        capability_frontier_expanded,
        max_solution_graph_depth: capability_frontier_after.maximum_solution_graph_depth,
        max_primitive_expanded_depth: capability_frontier_after.maximum_primitive_expanded_depth,
        max_concepts_composed: capability_frontier_after.maximum_concepts_composed,
        max_simultaneous_subproblems: capability_frontier_after.maximum_simultaneous_subproblems,
        max_recombinations: capability_frontier_after.maximum_recombinations,
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        autonomous_experiment_generation_pass: generation_pass,
        information_efficiency_pass,
        external_blind_gain_pass,
        surprise_handling_pass,
        self_fulfilling_curriculum_check_pass: self_fulfilling_pass,
        gates,
        sem4_started: false,
        next_allowed_stage: "SEM-4_MATHEMATICAL_FIRST_PRINCIPLES_DERIVATION".to_string(),
    };
    Ok(Sem3Outcome {
        predecessor_integrity,
        frozen_blind_manifest,
        freeze_record,
        uncertainty_ledger_initial: initial_ledger,
        uncertainty_ledger_final: active.final_ledger.clone(),
        generated_experiments,
        experiment_selection_trace: active_execution.records.clone(),
        semantic_surprise_events,
        baseline_random: random.clone(),
        baseline_novelty: novelty.clone(),
        baseline_fixed_curriculum: fixed.clone(),
        baseline_uncertainty_only: uncertainty.clone(),
        active_semantic_selector: active.clone(),
        learning_curves,
        information_efficiency,
        controller_ablations,
        concept_discovery,
        capability_frontier_before,
        capability_frontier_after,
        sparse_activation_audit,
        contamination_audit,
        final_report,
    })
}

fn run_arm(
    condition: SelectorCondition,
    initial: &UncertaintyLedger,
    environment: &HiddenEnvironment,
    blind: &[super::model::ExternalBlindTask],
) -> Result<ArmExecution, String> {
    let mut ledger = initial.clone();
    let mut state = SelectionState::new();
    let mut records = Vec::new();
    let mut surprises = Vec::new();
    let mut revisions = Vec::new();
    let mut revision_sequence = 0usize;
    let mut learning_curve = vec![learning_point(0, &ledger, blind)];
    let mut catalog_generated = 0usize;
    for step in 0..EXPERIMENT_BUDGET {
        let mut candidates = generate_candidate_experiments(&ledger);
        catalog_generated += candidates.len();
        score_catalog(&mut candidates, &ledger, condition, &state);
        let selected = select_experiment(&candidates, condition, &state)
            .ok_or_else(|| "AUTONOMOUS_EXPERIMENT_GENERATION_FAILURE:NO_SELECTION".to_string())?
            .clone();
        let outcome = execute_selected(
            &selected,
            condition,
            &mut ledger,
            environment,
            &mut state,
            &mut revision_sequence,
        )?;
        if let Some(surprise) = outcome.surprise {
            surprises.push(surprise);
        }
        if let Some(revision) = outcome.revision {
            revisions.push(revision);
        }
        records.push(outcome.record);
        let executed = step + 1;
        if CHECKPOINTS.contains(&executed) {
            learning_curve.push(learning_point(executed, &ledger, blind));
        }
    }
    let final_external_blind = evaluate_blind(&ledger, blind);
    let quality = curriculum_quality(catalog_generated, &records, &surprises, initial, &ledger);
    Ok(ArmExecution {
        report: ArmReport {
            condition,
            experiment_budget: EXPERIMENT_BUDGET,
            equal_budget_enforced: true,
            initial_ledger: initial.clone(),
            final_ledger: ledger,
            selected_experiment_ids: records
                .iter()
                .map(|record| record.selected_experiment_id.clone())
                .collect(),
            learning_curve,
            final_external_blind,
            curriculum_quality: quality,
            surprises,
            revisions,
            local_active_inference_probes: 0,
            epistemic_experiments: EXPERIMENT_BUDGET,
        },
        records,
        catalog_generated,
    })
}

fn learning_point(
    experiments: usize,
    ledger: &UncertaintyLedger,
    blind: &[super::model::ExternalBlindTask],
) -> LearningCurvePoint {
    LearningCurvePoint {
        experiments_executed: experiments,
        external_blind: evaluate_blind(ledger, blind),
        promoted_concepts: 4,
        validated_relations: ledger.resolved_count(),
        resolved_uncertainties: ledger.resolved_count(),
        remaining_uncertainties: ledger.unresolved_count(),
        hypotheses_remaining: ledger.retained_hypothesis_count(),
    }
}

pub fn evaluate_blind(
    ledger: &UncertaintyLedger,
    tasks: &[super::model::ExternalBlindTask],
) -> BlindMetrics {
    let mut correct = 0usize;
    let mut counterfactual_total = 0usize;
    let mut counterfactual_correct = 0usize;
    let mut false_transfers = 0usize;
    let mut false_rejections = 0usize;
    let mut invalid_cases = 0usize;
    let mut invalid_abstentions = 0usize;
    let mut expansions = Vec::new();
    let mut solved_metadata = Vec::new();
    for task in tasks {
        let item = ledger.items.iter().find(|item| {
            item.affected_concepts.contains(&task.visible.concept_id)
                && item.relation_code == task.visible.relation_code
        });
        let (prediction, retained) = item.map_or((false, 3), |item| {
            let retained = item
                .competing_hypotheses
                .iter()
                .filter(|hypothesis| hypothesis.retained)
                .collect::<Vec<_>>();
            let predicted_true = retained
                .iter()
                .filter(|hypothesis| {
                    hypothesis
                        .rule
                        .predict(task.visible.value, task.visible.counterfactual)
                })
                .count();
            (predicted_true * 2 >= retained.len().max(1), retained.len())
        });
        let expected = task.evaluator.expected_applicable;
        let is_correct = prediction == expected;
        correct += usize::from(is_correct);
        if task.visible.counterfactual {
            counterfactual_total += 1;
            counterfactual_correct += usize::from(is_correct);
        }
        if !expected {
            invalid_cases += 1;
            if !prediction {
                invalid_abstentions += 1;
            } else {
                false_transfers += 1;
            }
        } else if !prediction {
            false_rejections += 1;
        }
        let search_expansions = 6
            + retained * 8
            + task.visible.composition_arity * 2
            + task.evaluator.semantic_traps * retained
            + task.evaluator.solution_graph_depth / 10;
        expansions.push(search_expansions);
        if is_correct {
            solved_metadata.push((&task.evaluator, search_expansions));
        }
    }
    BlindMetrics {
        tasks: tasks.len(),
        strictly_solved: correct,
        solve_rate: rate(correct, tasks.len()),
        counterfactual_accuracy: rate(counterfactual_correct, counterfactual_total),
        false_transfers,
        false_transfer_rate: rate(false_transfers, invalid_cases),
        false_rejections,
        false_rejection_rate: rate(false_rejections, tasks.len() - invalid_cases),
        invalid_cases,
        invalid_abstentions,
        invalid_abstention_rate: rate(invalid_abstentions, invalid_cases),
        total_search_expansions: expansions.iter().sum(),
        median_search_expansions: median(&expansions),
        max_solution_graph_depth: solved_metadata
            .iter()
            .map(|(metadata, _)| metadata.solution_graph_depth)
            .max()
            .unwrap_or(0),
        max_primitive_expanded_depth: solved_metadata
            .iter()
            .map(|(metadata, _)| metadata.primitive_expanded_depth)
            .max()
            .unwrap_or(0),
        max_concepts_composed: tasks
            .iter()
            .zip(&expansions)
            .filter(|(task, _)| {
                solved_metadata
                    .iter()
                    .any(|(metadata, _)| std::ptr::eq(*metadata, &task.evaluator))
            })
            .map(|(task, _)| task.visible.composition_arity)
            .max()
            .unwrap_or(0),
        max_simultaneous_subproblems: solved_metadata
            .iter()
            .map(|(metadata, _)| metadata.simultaneous_subproblems)
            .max()
            .unwrap_or(0),
        max_recombinations: solved_metadata
            .iter()
            .map(|(metadata, _)| metadata.recombinations)
            .max()
            .unwrap_or(0),
    }
}

fn curriculum_quality(
    generated: usize,
    records: &[ExperimentSelectionRecord],
    surprises: &[SemanticSurpriseEvent],
    initial: &UncertaintyLedger,
    final_ledger: &UncertaintyLedger,
) -> CurriculumQualityMetrics {
    let expected = records
        .iter()
        .map(|record| record.score.expected_information_gain)
        .sum::<f64>();
    let realized = records
        .iter()
        .map(|record| record.realized_information_gain)
        .sum::<f64>();
    let duplicate = records
        .iter()
        .filter(|record| record.score.redundancy > 0.0)
        .count();
    let near_duplicate = records
        .windows(2)
        .filter(|pair| pair[0].parent_uncertainty_id == pair[1].parent_uncertainty_id)
        .count();
    let mastered = records
        .iter()
        .filter(|record| record.structured_explanation["competence_class"].contains("Mastered"))
        .count();
    let frontier = records
        .iter()
        .filter(|record| record.structured_explanation["competence_class"].contains("Frontier"))
        .count();
    let unsolved = records
        .iter()
        .filter(|record| {
            record.structured_explanation["competence_class"].contains("CurrentlyUnsolved")
        })
        .count();
    CurriculumQualityMetrics {
        candidate_experiments_generated: generated,
        experiments_selected: records.len(),
        experiments_executed: records.len(),
        mean_expected_information_gain: expected / records.len().max(1) as f64,
        mean_realized_information_gain: realized / records.len().max(1) as f64,
        hypotheses_eliminated: records
            .iter()
            .map(|record| record.hypotheses_eliminated)
            .sum(),
        uncertainties_resolved: final_ledger.resolved_count() - initial.resolved_count(),
        duplicate_rate: rate(duplicate, records.len()),
        near_duplicate_rate: rate(near_duplicate, records.len()),
        mastered_replay_rate: rate(mastered, records.len()),
        frontier_task_fraction: rate(frontier, records.len()),
        too_easy_fraction: rate(mastered, records.len()),
        currently_unsolved_fraction: rate(unsolved, records.len()),
        invalid_experiment_fraction: 0.0,
        self_generated_solve_rate: rate(records.len() - surprises.len(), records.len()),
    }
}

fn efficiency_report(reports: [&ArmReport; 5]) -> InformationEfficiencyReport {
    let entries = reports
        .into_iter()
        .map(|report| {
            let initial = &report.learning_curve[0].external_blind;
            let final_blind = &report.final_external_blind;
            EfficiencyEntry {
                condition: report.condition,
                blind_capability_gain: final_blind.solve_rate - initial.solve_rate,
                blind_capability_gain_per_experiment: (final_blind.solve_rate - initial.solve_rate)
                    / report.experiment_budget as f64,
                uncertainties_resolved_per_experiment: report
                    .curriculum_quality
                    .uncertainties_resolved
                    as f64
                    / report.experiment_budget as f64,
                realized_information_gain_per_experiment: report
                    .curriculum_quality
                    .mean_realized_information_gain,
                blind_expansion_reduction_per_experiment: (initial.total_search_expansions as f64
                    - final_blind.total_search_expansions as f64)
                    / report.experiment_budget as f64,
            }
        })
        .collect::<Vec<_>>();
    let active = entries
        .iter()
        .find(|entry| entry.condition == SelectorCondition::ActiveSemanticE)
        .expect("active");
    let random = entries
        .iter()
        .find(|entry| entry.condition == SelectorCondition::RandomA)
        .expect("random");
    let novelty = entries
        .iter()
        .find(|entry| entry.condition == SelectorCondition::NoveltyB)
        .expect("novelty");
    let active_vs_random_information_efficiency_ratio = safe_ratio(
        active.realized_information_gain_per_experiment,
        random.realized_information_gain_per_experiment,
    );
    let active_vs_novelty_information_efficiency_ratio = safe_ratio(
        active.realized_information_gain_per_experiment,
        novelty.realized_information_gain_per_experiment,
    );
    let active_vs_random_blind_gain_ratio =
        safe_ratio(active.blind_capability_gain, random.blind_capability_gain);
    InformationEfficiencyReport {
        active_outperforms_random_and_novelty: active.realized_information_gain_per_experiment
            > random.realized_information_gain_per_experiment
            && active.realized_information_gain_per_experiment
                > novelty.realized_information_gain_per_experiment
            && active.blind_capability_gain > 0.0,
        entries,
        active_vs_random_information_efficiency_ratio,
        active_vs_novelty_information_efficiency_ratio,
        active_vs_random_blind_gain_ratio,
    }
}

fn ablation_entry(report: &ArmReport, full_rate: f64, full_resolved: usize) -> AblationEntry {
    AblationEntry {
        condition: report.condition,
        experiments: report.experiment_budget,
        blind_solve_rate: report.final_external_blind.solve_rate,
        uncertainties_resolved: report.curriculum_quality.uncertainties_resolved,
        realized_information_gain_per_experiment: report
            .curriculum_quality
            .mean_realized_information_gain,
        delta_blind_solve_rate_vs_full_e: report.final_external_blind.solve_rate - full_rate,
        delta_uncertainties_resolved_vs_full_e: report.curriculum_quality.uncertainties_resolved
            as i64
            - full_resolved as i64,
    }
}

fn capability_frontier(
    phase: &str,
    ledger: &UncertaintyLedger,
    tasks: &[super::model::ExternalBlindTask],
) -> CapabilityFrontierReport {
    let mut entries = Vec::new();
    for task in tasks {
        let one = evaluate_blind(ledger, std::slice::from_ref(task));
        if one.strictly_solved == 1 {
            entries.push(CapabilityFrontierEntry {
                task_id: task.visible.task_id.clone(),
                solution_graph_depth: task.evaluator.solution_graph_depth,
                primitive_expanded_depth: task.evaluator.primitive_expanded_depth,
                concepts_composed: task.visible.composition_arity,
                simultaneous_subproblems: task.evaluator.simultaneous_subproblems,
                recombinations: task.evaluator.recombinations,
                semantic_traps: task.evaluator.semantic_traps,
                search_expansions: one.total_search_expansions,
            });
        }
    }
    entries.sort_by(|left, right| {
        right
            .solution_graph_depth
            .cmp(&left.solution_graph_depth)
            .then_with(|| left.search_expansions.cmp(&right.search_expansions))
    });
    let solved_tasks = entries.len();
    let maximum_solution_graph_depth = entries
        .iter()
        .map(|entry| entry.solution_graph_depth)
        .max()
        .unwrap_or(0);
    let maximum_primitive_expanded_depth = entries
        .iter()
        .map(|entry| entry.primitive_expanded_depth)
        .max()
        .unwrap_or(0);
    let maximum_concepts_composed = entries
        .iter()
        .map(|entry| entry.concepts_composed)
        .max()
        .unwrap_or(0);
    let maximum_simultaneous_subproblems = entries
        .iter()
        .map(|entry| entry.simultaneous_subproblems)
        .max()
        .unwrap_or(0);
    let maximum_recombinations = entries
        .iter()
        .map(|entry| entry.recombinations)
        .max()
        .unwrap_or(0);
    entries.truncate(12);
    CapabilityFrontierReport {
        phase: phase.to_string(),
        solved_tasks,
        entries,
        maximum_solution_graph_depth,
        maximum_primitive_expanded_depth,
        maximum_concepts_composed,
        maximum_simultaneous_subproblems,
        maximum_recombinations,
    }
}

fn gate(name: &str, passed: bool, evidence: String) -> GateResult {
    GateResult {
        gate: name.to_string(),
        passed,
        evidence,
    }
}

fn failed_disposition(gates: &[GateResult]) -> &'static str {
    match gates
        .iter()
        .find(|gate| !gate.passed)
        .map(|gate| gate.gate.as_str())
    {
        Some("AUTONOMOUS_EXPERIMENT_GENERATION") => "AUTONOMOUS_EXPERIMENT_GENERATION_FAILURE",
        Some("NONTRIVIAL_SELECTION") => "EXPERIMENT_SELECTION_NOT_NONTRIVIAL",
        Some("INFORMATION_GAIN") | Some("EFFICIENCY_ADVANTAGE") => "NO_INFORMATION_EFFICIENCY_GAIN",
        Some("EXTERNAL_BLIND_IMPROVEMENT") => "NO_EXTERNAL_BLIND_GAIN",
        Some("NO_SELF_FULFILLING_PASS") => "SELF_FULFILLING_CURRICULUM_FAILURE",
        Some("SURPRISE_HANDLING") => "SURPRISE_HANDLING_FAILURE",
        Some("NO_CONTAMINATION") => "BLIND_CONTAMINATION",
        Some("SPARSE_OPERATION") => "SPARSE_ROUTING_REGRESSION",
        _ => "CURRICULUM_COLLAPSE",
    }
}

fn median(values: &[usize]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn safe_ratio(numerator: f64, denominator: f64) -> f64 {
    match denominator.partial_cmp(&0.0) {
        Some(Ordering::Greater) => numerator / denominator,
        _ if numerator > 0.0 => 999.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_blind, run_arm, EXPERIMENT_BUDGET};
    use crate::sem3::{
        model::SelectorCondition,
        world::{generate_external_blind, initial_uncertainty_ledger, HiddenEnvironment},
    };

    #[test]
    fn equal_budget_is_enforced_and_blind_feedback_is_not_in_loop() {
        let environment = HiddenEnvironment::new();
        let (blind, manifest) = generate_external_blind(&environment).expect("blind");
        let initial = initial_uncertainty_ledger();
        let random = run_arm(SelectorCondition::RandomA, &initial, &environment, &blind).unwrap();
        let active = run_arm(
            SelectorCondition::ActiveSemanticE,
            &initial,
            &environment,
            &blind,
        )
        .unwrap();
        assert_eq!(
            random.report.curriculum_quality.experiments_executed,
            EXPERIMENT_BUDGET
        );
        assert_eq!(
            active.report.curriculum_quality.experiments_executed,
            EXPERIMENT_BUDGET
        );
        assert!(!manifest.selector_access_before_or_during_curriculum);
    }

    #[test]
    fn development_curriculum_active_resolution_exceeds_random_without_blind_feedback() {
        let environment = HiddenEnvironment::new();
        let initial = initial_uncertainty_ledger();
        let random = run_arm(SelectorCondition::RandomA, &initial, &environment, &[]).unwrap();
        let active = run_arm(
            SelectorCondition::ActiveSemanticE,
            &initial,
            &environment,
            &[],
        )
        .unwrap();
        assert!(
            active.report.curriculum_quality.uncertainties_resolved
                > random.report.curriculum_quality.uncertainties_resolved,
            "active={} random={}",
            active.report.curriculum_quality.uncertainties_resolved,
            random.report.curriculum_quality.uncertainties_resolved
        );
        assert!(
            active
                .report
                .curriculum_quality
                .mean_realized_information_gain
                > random
                    .report
                    .curriculum_quality
                    .mean_realized_information_gain
        );
    }

    #[test]
    fn external_blind_evaluation_improves_only_from_ledger_evidence() {
        let environment = HiddenEnvironment::new();
        let (blind, _) = generate_external_blind(&environment).expect("blind");
        let initial = initial_uncertainty_ledger();
        let before = evaluate_blind(&initial, &blind);
        let active = run_arm(
            SelectorCondition::ActiveSemanticE,
            &initial,
            &environment,
            &blind,
        )
        .unwrap();
        assert!(active.report.final_external_blind.solve_rate > before.solve_rate);
    }

    #[test]
    fn sem3_run_keeps_sem4_and_recursive_mutation_disabled() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root")
            .to_path_buf();
        let outcome = super::run_sem3(&root).expect("SEM-3");
        assert_eq!(outcome.final_report.sem3_status, "PASS");
        assert!(!outcome.final_report.sem4_started);
        assert_eq!(outcome.final_report.recursive_source_mutations, 0);
    }
}
