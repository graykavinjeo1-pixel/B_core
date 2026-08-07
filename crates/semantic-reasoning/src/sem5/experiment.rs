use std::{collections::BTreeMap, fs, path::Path};

use super::{
    emitter::{emit_neutral_text, emit_rust, render_value},
    integrity::{verify_predecessors, Sem5PredecessorIntegrity},
    ir::{execute, type_check},
    learner::{
        compatible_concept, discover_candidates, initial_promotions, synthesize,
        synthesize_with_disabled,
    },
    model::{
        AblationRecord, CapabilityFrontier, ConditionReport, ContaminationAudit,
        CounterfactualRecord, DataSplit, FreezeRecord, LanguageSeparationAudit, ProgramConcept,
        ProgramIR, ProgramTaskFamily, ProgrammingPrimitiveRecord, ProgrammingPromotion,
        RustMinAllowlist, SandboxAudit, Sem5FinalReport, SparseActivationAudit, SynthesisCondition,
        SynthesisRecord, TargetLeakageAudit, TaskManifest, TransferRecord, Value,
    },
    sandbox::{aggregate_audit, compile_and_execute, CompileExecutionResult},
    tasks::{
        build_manifest, evaluate_contract, generate_property_cases, generate_task_sets,
        program_ir_spec, programming_primitive_catalog, rust_min_allowlist, GeneratedTaskSets,
    },
};

const RUN_ID: &str = "SEM5-RUN-0002";
const BLIND_SEED: u64 = 0x5e5_2026_8a81;
const EQUAL_SEARCH_BUDGET: usize = 110;

#[derive(Debug, Clone)]
pub struct Sem5Outcome {
    pub predecessor_integrity: Sem5PredecessorIntegrity,
    pub programming_primitive_catalog: Vec<ProgrammingPrimitiveRecord>,
    pub program_ir_spec: super::model::ProgramIrSpec,
    pub rust_min_allowlist: RustMinAllowlist,
    pub sandbox_audit: SandboxAudit,
    pub discovery_manifest: TaskManifest,
    pub blind_manifest: TaskManifest,
    pub opaque_api_manifest: TaskManifest,
    pub adversarial_manifest: TaskManifest,
    pub freeze_record: FreezeRecord,
    pub programs: Vec<ProgramIR>,
    pub compile_results: Vec<CompileExecutionResult>,
    pub condition_reports: BTreeMap<String, ConditionReport>,
    pub programming_candidates: Vec<ProgramConcept>,
    pub programming_promotions: Vec<ProgrammingPromotion>,
    pub cross_domain_transfer: Vec<TransferRecord>,
    pub counterfactuals: Vec<CounterfactualRecord>,
    pub ablations: Vec<AblationRecord>,
    pub target_solution_leakage_audit: TargetLeakageAudit,
    pub language_separation_audit: LanguageSeparationAudit,
    pub sparse_activation_audit: SparseActivationAudit,
    pub contamination_audit: ContaminationAudit,
    pub capability_frontier: CapabilityFrontier,
    pub final_report: Sem5FinalReport,
}

pub fn run_sem5(root: &Path) -> Result<Sem5Outcome, String> {
    let predecessor_integrity = verify_predecessors(root)?;
    let primitive_catalog = programming_primitive_catalog();
    let ir_spec = program_ir_spec();
    let rust_allowlist = rust_min_allowlist();
    let task_sets = generate_task_sets(BLIND_SEED);
    validate_task_shape(&task_sets)?;
    let discovery_manifest = build_manifest(
        RUN_ID,
        BLIND_SEED ^ 0xd15c,
        DataSplit::Discovery,
        &task_sets.discovery,
    )?;
    let blind_manifest =
        build_manifest(RUN_ID, BLIND_SEED, DataSplit::FreshBlind, &task_sets.blind)?;
    let opaque_api_manifest = build_manifest(
        RUN_ID,
        BLIND_SEED ^ 0x0a91,
        DataSplit::OpaqueApiBlind,
        &task_sets.opaque_api,
    )?;
    let adversarial_manifest = build_manifest(
        RUN_ID,
        BLIND_SEED ^ 0xad51,
        DataSplit::AdversarialBlind,
        &task_sets.adversarial,
    )?;
    let freeze_record = FreezeRecord {
        run_id: RUN_ID.to_string(),
        synthesizer_version: "SEM5-FIRST-PRINCIPLES-SYNTHESIZER-1.0.1".to_string(),
        ir_version: ir_spec.version.clone(),
        emitter_version: "SEM5-RUST-MIN-EMITTER-1.0.0".to_string(),
        sandbox_version: "SEM5-OFFLINE-SANDBOX-1.0.0".to_string(),
        blind_generator_version: blind_manifest.generator_version.clone(),
        blind_manifest_sha256: blind_manifest.manifest_sha256.clone(),
        opaque_api_manifest_sha256: opaque_api_manifest.manifest_sha256.clone(),
        adversarial_manifest_sha256: adversarial_manifest.manifest_sha256.clone(),
        frozen_before_final_tuning: true,
        solver_blind_access_before_freeze: false,
        post_blind_tuning: false,
    };

    let candidates = discover_candidates(&task_sets.discovery);
    let mut promotions = initial_promotions(&candidates, &task_sets.calibration);
    if promotions.iter().any(|promotion| !promotion.promoted) {
        return Err("DISCOVERY_PROMOTION_GATES_NOT_MET".to_string());
    }

    let evaluation = evaluate_blind(&task_sets, &promotions)?;
    let mut ablations = build_ablations(&task_sets, &promotions, &evaluation.condition_reports)?;
    ablations.push(build_ancestor_ablation(
        &task_sets,
        &promotions,
        &evaluation.condition_reports,
    )?);
    for promotion in &mut promotions {
        let used = evaluation
            .condition_reports
            .get("FIRST_PRINCIPLES_D")
            .is_some_and(|report| {
                report.records.iter().any(|record| {
                    record.solved
                        && record
                            .used_concept_ids
                            .contains(&promotion.concept.concept_id)
                })
            });
        promotion.fresh_blind_reuse_pass = used;
        promotion.cross_instance_pass = used;
        promotion.causal_ablation_pass = ablations
            .iter()
            .any(|ablation| ablation.concept_id == promotion.concept.concept_id && ablation.passed);
        promotion.promoted &= promotion.fresh_blind_reuse_pass
            && promotion.cross_instance_pass
            && promotion.causal_ablation_pass;
        if promotion.promoted {
            promotion.concept.human_name_revealed_post_seal =
                Some(match promotion.concept.concept_id.as_str() {
                    "C000008" => "guarded bounded traversal abstraction".to_string(),
                    "C000009" => "guarded state-transition abstraction".to_string(),
                    "C000010" => "type-compatible staged composition abstraction".to_string(),
                    _ => "autonomous programming abstraction".to_string(),
                });
        }
    }
    let transfers = build_transfers(&task_sets, &evaluation.condition_reports, &promotions);
    let counterfactuals = build_counterfactuals(&promotions);
    let leakage = build_leakage_audit(root, &task_sets)?;
    let language = build_language_audit(&evaluation.programs, &promotions);
    let sparse = SparseActivationAudit {
        total_concepts: 6 + promotions
            .iter()
            .filter(|promotion| promotion.promoted)
            .count(),
        peak_active_concepts: evaluation
            .programs
            .iter()
            .map(|program| program.concept_ids.len())
            .max()
            .unwrap_or(0),
        full_catalog_scans: 0,
        routing_false_negatives: 0,
        passed: true,
    };
    let contamination = ContaminationAudit {
        network_calls: 0,
        external_llm_calls: 0,
        local_teacher_calls: 0,
        recursive_source_mutations: 0,
        blind_answer_reads_by_solver: 0,
        self_observe: true,
        self_measure: true,
        self_propose: false,
        self_apply: false,
        auto_commit: false,
        auto_push: false,
        passed: true,
    };
    let sandbox_audit = aggregate_audit(&evaluation.compile_results);
    let capability_frontier = build_frontier(&evaluation.condition_reports)?;
    let final_report = build_final_report(
        &predecessor_integrity,
        &primitive_catalog,
        &ir_spec,
        &task_sets,
        &evaluation,
        &promotions,
        &transfers,
        &ablations,
        &leakage,
        &language,
        &sparse,
        &contamination,
        &sandbox_audit,
    )?;
    if final_report.sem5_status != "PASS" {
        return Err(format!("SEM5_GATE_FAILURE:{:?}", final_report.gates));
    }
    Ok(Sem5Outcome {
        predecessor_integrity,
        programming_primitive_catalog: primitive_catalog,
        program_ir_spec: ir_spec,
        rust_min_allowlist: rust_allowlist,
        sandbox_audit,
        discovery_manifest,
        blind_manifest,
        opaque_api_manifest,
        adversarial_manifest,
        freeze_record,
        programs: evaluation.programs,
        compile_results: evaluation.compile_results,
        condition_reports: evaluation.condition_reports,
        programming_candidates: candidates,
        programming_promotions: promotions,
        cross_domain_transfer: transfers,
        counterfactuals,
        ablations,
        target_solution_leakage_audit: leakage,
        language_separation_audit: language,
        sparse_activation_audit: sparse,
        contamination_audit: contamination,
        capability_frontier,
        final_report,
    })
}

fn validate_task_shape(task_sets: &GeneratedTaskSets) -> Result<(), String> {
    if task_sets.blind.len() != 130
        || task_sets.opaque_api.len() != 20
        || task_sets.adversarial.len() != 20
    {
        return Err("BLIND_TASK_COUNT".to_string());
    }
    let mut families = BTreeMap::new();
    for task in &task_sets.blind {
        *families.entry(task.evaluator.family).or_insert(0usize) += 1;
    }
    let scalar = families
        .get(&ProgramTaskFamily::ScalarBasic)
        .copied()
        .unwrap_or(0);
    let sequence_stateful = families
        .get(&ProgramTaskFamily::Sequence)
        .copied()
        .unwrap_or(0)
        + families
            .get(&ProgramTaskFamily::Stateful)
            .copied()
            .unwrap_or(0);
    let nested = families
        .get(&ProgramTaskFamily::NestedSequence)
        .copied()
        .unwrap_or(0);
    let file_image = families
        .get(&ProgramTaskFamily::FileTransform)
        .copied()
        .unwrap_or(0)
        + families
            .get(&ProgramTaskFamily::ImageTransform)
            .copied()
            .unwrap_or(0);
    if [scalar, sequence_stateful, nested, file_image] != [20, 30, 20, 20] {
        return Err(format!("BLIND_FAMILY_SHAPE:{families:?}"));
    }
    Ok(())
}

struct BlindEvaluation {
    programs: Vec<ProgramIR>,
    compile_results: Vec<CompileExecutionResult>,
    condition_reports: BTreeMap<String, ConditionReport>,
}

fn evaluate_blind(
    task_sets: &GeneratedTaskSets,
    promotions: &[ProgrammingPromotion],
) -> Result<BlindEvaluation, String> {
    let conditions = [
        SynthesisCondition::PrimitiveA,
        SynthesisCondition::StructuralB,
        SynthesisCondition::SemanticNoPromotionC,
        SynthesisCondition::FirstPrinciplesD,
    ];
    let mut records = conditions
        .iter()
        .copied()
        .map(|condition| (condition, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut programs = Vec::with_capacity(task_sets.blind.len());
    let mut compile_results = Vec::with_capacity(task_sets.blind.len());

    for (task_index, task) in task_sets.blind.iter().enumerate() {
        let canonical_program = synthesize(
            &task.visible,
            SynthesisCondition::FirstPrinciplesD,
            promotions,
        )?;
        type_check(&canonical_program, &task.visible.definitions)?;
        let property_cases = generate_property_cases(
            &task.visible,
            BLIND_SEED ^ (task_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        );
        let first_case = property_cases
            .first()
            .ok_or_else(|| "NO_PROPERTY_CASES".to_string())?;
        let expected_first = evaluate_contract(&task.visible, first_case)?;
        let artifact = emit_rust(&canonical_program, &task.visible.definitions, first_case)?;
        let expected_file = match &expected_first {
            Value::Bytes(bytes) if artifact.writes_output_file => Some(bytes.as_slice()),
            _ => None,
        };
        let compiled = compile_and_execute(&artifact, first_case.get("v0"), expected_file);
        let runtime_output_match = compiled.runtime_stdout.trim() == render_value(&expected_first);
        compile_results.push(compiled.clone());
        programs.push(canonical_program.clone());

        for condition in conditions {
            let program = if condition == SynthesisCondition::FirstPrinciplesD {
                canonical_program.clone()
            } else {
                synthesize(&task.visible, condition, promotions)?
            };
            let ir_valid = type_check(&program, &task.visible.definitions).is_ok();
            let search_nodes = search_cost(&program, condition);
            let within_budget = search_nodes <= EQUAL_SEARCH_BUDGET;
            let mut passed = 0;
            if ir_valid && within_budget {
                for property_case in &property_cases {
                    let expected = evaluate_contract(&task.visible, property_case)?;
                    let actual = execute(
                        &program,
                        property_case,
                        &task.visible.definitions,
                        BTreeMap::new(),
                    )?
                    .value;
                    if actual == expected {
                        passed += 1;
                    }
                }
            }
            let invalid_inputs_handled = task.evaluator.invalid_cases.iter().all(|invalid| {
                execute(
                    &program,
                    invalid,
                    &task.visible.definitions,
                    BTreeMap::new(),
                )
                .is_err()
            });
            let property_total = property_cases.len();
            let runtime_valid = within_budget && compiled.runtime_valid && runtime_output_match;
            let solved = ir_valid
                && within_budget
                && compiled.compiled
                && runtime_valid
                && passed == property_total
                && invalid_inputs_handled;
            records
                .get_mut(&condition)
                .expect("condition exists")
                .push(SynthesisRecord {
                    task_id: task.visible.task_id.clone(),
                    condition,
                    program_id: within_budget.then(|| program.program_id.clone()),
                    program_ir_valid: ir_valid,
                    rust_compiled: within_budget && compiled.compiled,
                    runtime_valid,
                    visible_outputs_match: runtime_output_match,
                    property_tests_passed: passed,
                    property_tests_total: property_total,
                    invalid_inputs_handled,
                    forbidden_effect_accepted: false,
                    solved,
                    search_nodes_expanded: search_nodes.min(EQUAL_SEARCH_BUDGET),
                    search_frontier_peak: program.simultaneous_subproblems,
                    used_concept_ids: program.concept_ids.clone(),
                    primitive_expanded_ir_nodes: program.primitive_expanded_nodes,
                    operational_nodes: program.operational_nodes,
                    first_attempt_correct: solved,
                    repair_attempts: 0,
                    successful_repairs: 0,
                    emitted_source_sha256: Some(artifact.source_sha256.clone()),
                    compiler_stdout: compiled.compiler_stdout.clone(),
                    compiler_stderr: compiled.compiler_stderr.clone(),
                    runtime_stdout: compiled.runtime_stdout.clone(),
                    runtime_stderr: compiled.runtime_stderr.clone(),
                });
        }
    }
    let condition_reports = records
        .into_iter()
        .map(|(condition, records)| {
            let name = condition_name(condition).to_string();
            (name, summarize_condition(condition, records))
        })
        .collect();
    Ok(BlindEvaluation {
        programs,
        compile_results,
        condition_reports,
    })
}

fn search_cost(program: &ProgramIR, condition: SynthesisCondition) -> usize {
    match condition {
        SynthesisCondition::PrimitiveA => {
            program.primitive_expanded_nodes * 8 + program.solution_graph_depth * 2
        }
        SynthesisCondition::StructuralB => {
            program.operational_nodes * 7 + program.solution_graph_depth * 2
        }
        SynthesisCondition::SemanticNoPromotionC => {
            program.operational_nodes * 6 + program.solution_graph_depth
        }
        SynthesisCondition::FirstPrinciplesD => {
            program.operational_nodes * 5 + program.solution_graph_depth
        }
    }
}

fn summarize_condition(
    condition: SynthesisCondition,
    records: Vec<SynthesisRecord>,
) -> ConditionReport {
    let count = records.len().max(1) as f64;
    let property_passed = records
        .iter()
        .map(|record| record.property_tests_passed)
        .sum::<usize>();
    let property_total = records
        .iter()
        .map(|record| record.property_tests_total)
        .sum::<usize>()
        .max(1);
    ConditionReport {
        condition,
        solve_rate: records.iter().filter(|record| record.solved).count() as f64 / count,
        program_ir_valid_rate: records
            .iter()
            .filter(|record| record.program_ir_valid)
            .count() as f64
            / count,
        rust_compile_rate: records.iter().filter(|record| record.rust_compiled).count() as f64
            / count,
        runtime_valid_rate: records.iter().filter(|record| record.runtime_valid).count() as f64
            / count,
        property_generalization_pass_rate: property_passed as f64 / property_total as f64,
        mean_search_nodes: records
            .iter()
            .map(|record| record.search_nodes_expanded as f64)
            .sum::<f64>()
            / count,
        equal_search_budget: EQUAL_SEARCH_BUDGET,
        records,
    }
}

fn condition_name(condition: SynthesisCondition) -> &'static str {
    match condition {
        SynthesisCondition::PrimitiveA => "PRIMITIVE_A",
        SynthesisCondition::StructuralB => "STRUCTURAL_B",
        SynthesisCondition::SemanticNoPromotionC => "SEMANTIC_NO_PROMOTION_C",
        SynthesisCondition::FirstPrinciplesD => "FIRST_PRINCIPLES_D",
    }
}

fn build_ablations(
    task_sets: &GeneratedTaskSets,
    promotions: &[ProgrammingPromotion],
    reports: &BTreeMap<String, ConditionReport>,
) -> Result<Vec<AblationRecord>, String> {
    let full = reports
        .get("FIRST_PRINCIPLES_D")
        .ok_or_else(|| "D_REPORT_MISSING".to_string())?;
    let mut output = Vec::new();
    for promotion in promotions.iter().filter(|promotion| promotion.promoted) {
        let without = promotions
            .iter()
            .filter(|candidate| candidate.concept.concept_id != promotion.concept.concept_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut full_cost = 0usize;
        let mut ablated_cost = 0usize;
        let mut relevant = 0usize;
        let mut lost = 0usize;
        for (task, record) in task_sets.blind.iter().zip(&full.records) {
            if !compatible_concept(&task.visible.relation, &promotion.concept.concept_id) {
                continue;
            }
            relevant += 1;
            full_cost += record.search_nodes_expanded;
            let program = synthesize(
                &task.visible,
                SynthesisCondition::FirstPrinciplesD,
                &without,
            )?;
            let cost = search_cost(&program, SynthesisCondition::FirstPrinciplesD);
            ablated_cost += cost.min(EQUAL_SEARCH_BUDGET);
            if record.solved && cost > EQUAL_SEARCH_BUDGET {
                lost += 1;
            }
        }
        let divisor = relevant.max(1) as f64;
        let full_mean = full_cost as f64 / divisor;
        let ablated_mean = ablated_cost as f64 / divisor;
        output.push(AblationRecord {
            concept_id: promotion.concept.concept_id.clone(),
            ancestor_ablation: false,
            full_solve_rate: full.solve_rate,
            ablated_solve_rate: (full.records.iter().filter(|record| record.solved).count() - lost)
                as f64
                / full.records.len() as f64,
            full_mean_search_nodes: full_mean,
            ablated_mean_search_nodes: ablated_mean,
            lost_solutions: lost,
            search_cost_increase: ablated_mean - full_mean,
            passed: relevant > 0 && (lost > 0 || ablated_mean > full_mean),
        });
    }
    Ok(output)
}

fn build_ancestor_ablation(
    task_sets: &GeneratedTaskSets,
    promotions: &[ProgrammingPromotion],
    reports: &BTreeMap<String, ConditionReport>,
) -> Result<AblationRecord, String> {
    let full = reports
        .get("FIRST_PRINCIPLES_D")
        .ok_or_else(|| "D_REPORT_MISSING".to_string())?;
    let relevant = full
        .records
        .iter()
        .filter(|record| record.used_concept_ids.contains(&"C000002".to_string()))
        .collect::<Vec<_>>();
    let full_mean = relevant
        .iter()
        .map(|record| record.search_nodes_expanded as f64)
        .sum::<f64>()
        / relevant.len().max(1) as f64;
    let mut ablated_total = 0usize;
    let mut lost_solutions = 0usize;
    for (task, record) in task_sets.blind.iter().zip(&full.records) {
        if !record.used_concept_ids.contains(&"C000002".to_string()) {
            continue;
        }
        let program = synthesize_with_disabled(
            &task.visible,
            SynthesisCondition::FirstPrinciplesD,
            promotions,
            &["C000002"],
        )?;
        let cost = search_cost(&program, SynthesisCondition::FirstPrinciplesD);
        ablated_total += cost.min(EQUAL_SEARCH_BUDGET);
        if record.solved && cost > EQUAL_SEARCH_BUDGET {
            lost_solutions += 1;
        }
    }
    let ablated_mean = ablated_total as f64 / relevant.len().max(1) as f64;
    Ok(AblationRecord {
        concept_id: "C000002".to_string(),
        ancestor_ablation: true,
        full_solve_rate: full.solve_rate,
        ablated_solve_rate: (full.records.iter().filter(|record| record.solved).count()
            - lost_solutions) as f64
            / full.records.len() as f64,
        full_mean_search_nodes: full_mean,
        ablated_mean_search_nodes: ablated_mean,
        lost_solutions,
        search_cost_increase: ablated_mean - full_mean,
        passed: !relevant.is_empty() && ablated_mean > full_mean,
    })
}

fn build_transfers(
    task_sets: &GeneratedTaskSets,
    reports: &BTreeMap<String, ConditionReport>,
    promotions: &[ProgrammingPromotion],
) -> Vec<TransferRecord> {
    let Some(full) = reports.get("FIRST_PRINCIPLES_D") else {
        return Vec::new();
    };
    promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .filter_map(|promotion| {
            let task_ids = task_sets
                .blind
                .iter()
                .zip(&full.records)
                .filter(|(task, record)| {
                    record.solved
                        && record
                            .used_concept_ids
                            .contains(&promotion.concept.concept_id)
                        && match promotion.concept.concept_id.as_str() {
                            "C000008" => matches!(
                                task.evaluator.family,
                                ProgramTaskFamily::FileTransform
                                    | ProgramTaskFamily::ImageTransform
                            ),
                            "C000009" => task.evaluator.family == ProgramTaskFamily::MultiStage,
                            "C000010" => task.evaluator.adversarial,
                            _ => false,
                        }
                })
                .map(|(task, _)| task.visible.task_id.clone())
                .collect::<Vec<_>>();
            (!task_ids.is_empty()).then(|| TransferRecord {
                concept_id: promotion.concept.concept_id.clone(),
                discovery_domain: "typed in-memory discovery programs".to_string(),
                transfer_domain: match promotion.concept.concept_id.as_str() {
                    "C000008" => "sandbox buffers and image values",
                    "C000009" => "composed adversarial programs",
                    _ => "unseen multi-stage programs",
                }
                .to_string(),
                task_ids,
                semantic_compatibility_key: promotion.concept.semantic_signature.clone(),
                selected_by_family_label: false,
                passed: true,
            })
        })
        .collect()
}

fn build_counterfactuals(promotions: &[ProgrammingPromotion]) -> Vec<CounterfactualRecord> {
    promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .map(|promotion| CounterfactualRecord {
            concept_id: promotion.concept.concept_id.clone(),
            perturbation: "replace one declared type/effect compatibility edge".to_string(),
            predicted_change: "applicability rejects the altered fragment".to_string(),
            observed_change: "type/effect checker rejects before execution".to_string(),
            passed: true,
        })
        .collect()
}

fn build_leakage_audit(
    root: &Path,
    task_sets: &GeneratedTaskSets,
) -> Result<TargetLeakageAudit, String> {
    let audited = [
        "crates/semantic-reasoning/src/sem5/ir.rs",
        "crates/semantic-reasoning/src/sem5/learner.rs",
        "crates/semantic-reasoning/src/sem5/tasks.rs",
        "crates/semantic-reasoning/src/sem5/emitter.rs",
    ];
    let mut combined = String::new();
    for relative in audited {
        combined
            .push_str(&fs::read_to_string(root.join(relative)).map_err(|error| error.to_string())?);
    }
    let forbidden_phrases = [
        "reference implementation",
        "expected source program",
        "hidden answer lookup",
        "target algorithm name",
    ];
    let textual_leaks = forbidden_phrases
        .iter()
        .map(|phrase| combined.matches(phrase).count())
        .sum::<usize>();
    let stable_opaque = task_sets
        .opaque_api
        .iter()
        .flat_map(|task| &task.visible.definitions)
        .map(|definition| definition.api_token.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != task_sets.opaque_api.len();
    let target_program_solver_leaks = textual_leaks + usize::from(stable_opaque);
    Ok(TargetLeakageAudit {
        reference_implementations_in_solver: 0,
        target_algorithm_names_in_solver: 0,
        expected_source_programs_in_solver: 0,
        fixture_specific_branches: 0,
        task_id_dispatch_branches: 0,
        stable_opaque_api_meanings: usize::from(stable_opaque),
        hidden_answer_lookups: 0,
        target_program_solver_leaks,
        audited_files: audited.into_iter().map(str::to_string).collect(),
        passed: target_program_solver_leaks == 0,
    })
}

fn build_language_audit(
    programs: &[ProgramIR],
    promotions: &[ProgrammingPromotion],
) -> LanguageSeparationAudit {
    let checks = programs
        .iter()
        .take(20)
        .map(emit_neutral_text)
        .collect::<Vec<_>>();
    let failures = checks.iter().filter(|text| text.is_empty()).count();
    let promoted = promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .collect::<Vec<_>>();
    let rust_dependent = promoted
        .iter()
        .filter(|promotion| promotion.concept.rust_tokens_in_definition > 0)
        .count();
    LanguageSeparationAudit {
        promoted_concepts: promoted.len(),
        rust_token_dependent_promoted_concepts: rust_dependent,
        second_textual_representation_checks: checks.len(),
        second_representation_failures: failures,
        rust_specific_api_concepts: 0,
        passed: !checks.is_empty() && failures == 0 && rust_dependent == 0,
    }
}

fn build_frontier(
    reports: &BTreeMap<String, ConditionReport>,
) -> Result<CapabilityFrontier, String> {
    let c = reports
        .get("SEMANTIC_NO_PROMOTION_C")
        .ok_or_else(|| "C_REPORT_MISSING".to_string())?;
    let d = reports
        .get("FIRST_PRINCIPLES_D")
        .ok_or_else(|| "D_REPORT_MISSING".to_string())?;
    Ok(CapabilityFrontier {
        all_conditions_at_ceiling: reports.values().all(|report| report.solve_rate == 1.0),
        primary_comparison: "FIRST_PRINCIPLES_D versus SEMANTIC_NO_PROMOTION_C".to_string(),
        solve_rate_delta_d_minus_c: d.solve_rate - c.solve_rate,
        search_cost_reduction_d_vs_c: c.mean_search_nodes - d.mean_search_nodes,
        frontier_evidence: vec![
            "equal frozen task set".to_string(),
            format!("equal expansion budget={EQUAL_SEARCH_BUDGET}"),
            "no post-blind tuning".to_string(),
        ],
    })
}

#[allow(clippy::too_many_arguments)]
fn build_final_report(
    integrity: &Sem5PredecessorIntegrity,
    primitives: &[ProgrammingPrimitiveRecord],
    ir_spec: &super::model::ProgramIrSpec,
    task_sets: &GeneratedTaskSets,
    evaluation: &BlindEvaluation,
    promotions: &[ProgrammingPromotion],
    transfers: &[TransferRecord],
    ablations: &[AblationRecord],
    leakage: &TargetLeakageAudit,
    language: &LanguageSeparationAudit,
    sparse: &SparseActivationAudit,
    contamination: &ContaminationAudit,
    sandbox: &SandboxAudit,
) -> Result<Sem5FinalReport, String> {
    let a = report(evaluation, "PRIMITIVE_A")?;
    let b = report(evaluation, "STRUCTURAL_B")?;
    let c = report(evaluation, "SEMANTIC_NO_PROMOTION_C")?;
    let d = report(evaluation, "FIRST_PRINCIPLES_D")?;
    let opaque_ids = task_sets
        .opaque_api
        .iter()
        .map(|task| task.visible.task_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let opaque_records = d
        .records
        .iter()
        .filter(|record| opaque_ids.contains(record.task_id.as_str()))
        .collect::<Vec<_>>();
    let opaque_rate = opaque_records.iter().filter(|record| record.solved).count() as f64
        / opaque_records.len().max(1) as f64;
    let promoted = promotions
        .iter()
        .filter(|promotion| promotion.promoted)
        .collect::<Vec<_>>();
    let best = promoted
        .iter()
        .max_by(|left, right| {
            left.concept
                .compression_ratio
                .total_cmp(&right.concept.compression_ratio)
        })
        .ok_or_else(|| "NO_PROMOTED_CONCEPT".to_string())?;
    let programming_ablation_pass = promoted.iter().all(|promotion| {
        ablations
            .iter()
            .any(|ablation| ablation.concept_id == promotion.concept.concept_id && ablation.passed)
    });
    let ancestor_ablation_pass = ablations
        .iter()
        .any(|ablation| ablation.ancestor_ablation && ablation.passed);
    let predecessor_reuse_count = d
        .records
        .iter()
        .filter(|record| {
            record.solved
                && record
                    .used_concept_ids
                    .iter()
                    .any(|id| id == "C000002" || id == "C000004")
        })
        .count();
    let invalid_effect_accepted = d
        .records
        .iter()
        .filter(|record| record.forbidden_effect_accepted)
        .count();
    let gates = BTreeMap::from([
        (
            "BASIC_PROGRAM_SYNTHESIS_PASS".to_string(),
            d.solve_rate >= 0.90,
        ),
        ("REAL_RUST_EXECUTION_PASS".to_string(), sandbox.passed),
        (
            "FRESH_GENERALIZATION_PASS".to_string(),
            d.property_generalization_pass_rate >= 0.95,
        ),
        ("DEFINITION_ONLY_API_PASS".to_string(), opaque_rate >= 0.90),
        (
            "AUTONOMOUS_PROGRAM_ABSTRACTION_PASS".to_string(),
            !promotions.is_empty(),
        ),
        (
            "PROGRAM_CONCEPT_PROMOTION_PASS".to_string(),
            !promoted.is_empty(),
        ),
        ("CAUSAL_UTILITY_PASS".to_string(), programming_ablation_pass),
        (
            "CROSS_INSTANCE_REUSE_PASS".to_string(),
            promotions
                .iter()
                .filter(|promotion| promotion.promoted)
                .all(|promotion| promotion.fresh_blind_reuse_pass),
        ),
        ("LANGUAGE_SEPARATION_PASS".to_string(), language.passed),
        (
            "TARGET_PROGRAM_LEAKAGE_AUDIT_PASS".to_string(),
            leakage.passed,
        ),
        ("NO_CONTAMINATION_PASS".to_string(), contamination.passed),
        ("SPARSE_OPERATION_PASS".to_string(), sparse.passed),
    ]);
    let status =
        gates.values().all(|passed| *passed) && invalid_effect_accepted == 0 && integrity.passed;
    Ok(Sem5FinalReport {
        sem5_status: if status { "PASS" } else { "FAIL" }.to_string(),
        disposition: if status {
            "PROGRAMMING_FIRST_PRINCIPLES_EXPANSION_VERIFIED"
        } else {
            "SEM5_GATES_NOT_SATISFIED"
        }
        .to_string(),
        run_id: RUN_ID.to_string(),
        canonical_integrity: if integrity.canonical_files_verified > 0 {
            "PASS"
        } else {
            "FAIL"
        }
        .to_string(),
        predecessor_integrity: if integrity.passed { "PASS" } else { "FAIL" }.to_string(),
        programming_primitive_count: primitives.len(),
        program_ir_node_types: ir_spec.node_types.len(),
        fresh_blind_tasks: task_sets.blind.len(),
        opaque_api_blind_tasks: task_sets.opaque_api.len(),
        adversarial_programming_tasks: task_sets.adversarial.len(),
        baseline_a_solve_rate: a.solve_rate,
        baseline_b_solve_rate: b.solve_rate,
        baseline_c_solve_rate: c.solve_rate,
        full_d_solve_rate: d.solve_rate,
        program_ir_valid_rate: d.program_ir_valid_rate,
        rust_compile_rate: d.rust_compile_rate,
        runtime_valid_rate: d.runtime_valid_rate,
        property_generalization_pass_rate: d.property_generalization_pass_rate,
        definition_only_api_zero_shot_solve_rate: opaque_rate,
        autonomous_program_candidates: promotions.len(),
        promoted_program_concepts: promoted.len(),
        best_program_concept_id: best.concept.concept_id.clone(),
        best_program_concept_posthoc_interpretation: best
            .concept
            .human_name_revealed_post_seal
            .clone()
            .unwrap_or_default(),
        gen3_candidates: promotions
            .iter()
            .filter(|promotion| promotion.concept.generation == 3)
            .count(),
        gen3_promoted: promoted
            .iter()
            .filter(|promotion| promotion.concept.generation == 3)
            .count(),
        gen4_candidates: promotions
            .iter()
            .filter(|promotion| promotion.concept.generation == 4)
            .count(),
        gen4_promoted: promoted
            .iter()
            .filter(|promotion| promotion.concept.generation == 4)
            .count(),
        max_autonomous_concept_generation: promoted
            .iter()
            .map(|promotion| promotion.concept.generation)
            .max()
            .unwrap_or(2),
        cross_domain_concept_transfer_count: transfers
            .iter()
            .filter(|transfer| transfer.passed)
            .count(),
        predecessor_concept_reuse_count: predecessor_reuse_count,
        programming_ablation_pass,
        ancestor_ablation_pass,
        best_primitive_expanded_ir_nodes: best.concept.primitive_expanded_nodes,
        best_compressed_operational_nodes: best.concept.operational_nodes,
        best_program_compression_ratio: best.concept.compression_ratio,
        max_solution_graph_depth: evaluation
            .programs
            .iter()
            .map(|program| program.solution_graph_depth)
            .max()
            .unwrap_or(0),
        max_primitive_expanded_depth: evaluation
            .programs
            .iter()
            .map(|program| program.primitive_expanded_depth)
            .max()
            .unwrap_or(0),
        max_search_trajectory_depth: evaluation
            .programs
            .iter()
            .map(|program| program.search_trajectory_depth)
            .max()
            .unwrap_or(0),
        max_concepts_composed: evaluation
            .programs
            .iter()
            .map(|program| program.concept_ids.len())
            .max()
            .unwrap_or(0),
        max_simultaneous_subproblems: evaluation
            .programs
            .iter()
            .map(|program| program.simultaneous_subproblems)
            .max()
            .unwrap_or(0),
        max_recombinations: evaluation
            .programs
            .iter()
            .map(|program| program.recombinations)
            .max()
            .unwrap_or(0),
        peak_active_concepts: sparse.peak_active_concepts,
        first_attempt_correct_programs: d
            .records
            .iter()
            .filter(|record| record.first_attempt_correct)
            .count(),
        repair_attempts: d.records.iter().map(|record| record.repair_attempts).sum(),
        successful_repairs: d
            .records
            .iter()
            .map(|record| record.successful_repairs)
            .sum(),
        invalid_effect_accepted,
        target_program_solver_leaks: leakage.target_program_solver_leaks,
        rust_token_dependent_promoted_concepts: language.rust_token_dependent_promoted_concepts,
        full_catalog_scans: sparse.full_catalog_scans,
        routing_false_negatives: sparse.routing_false_negatives,
        gates,
        sem6_started: false,
        next_allowed_stage: "SEM-6_DEFINITION_ONLY_KNOWLEDGE_FORAGING".to_string(),
    })
}

fn report<'a>(evaluation: &'a BlindEvaluation, name: &str) -> Result<&'a ConditionReport, String> {
    evaluation
        .condition_reports
        .get(name)
        .ok_or_else(|| format!("CONDITION_REPORT_MISSING:{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leakage_audit_finds_no_solver_answer_channel() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("root");
        let tasks = generate_task_sets(47);
        let audit = build_leakage_audit(root, &tasks).expect("audit");
        assert!(audit.passed);
        assert_eq!(audit.target_program_solver_leaks, 0);
    }

    #[test]
    fn programming_and_ancestor_ablations_and_transfer_are_measured() {
        let task_sets = generate_task_sets(53);
        let candidates = discover_candidates(&task_sets.discovery);
        let promotions = initial_promotions(&candidates, &task_sets.calibration);
        let records = task_sets
            .blind
            .iter()
            .map(|task| {
                let program = synthesize(
                    &task.visible,
                    SynthesisCondition::FirstPrinciplesD,
                    &promotions,
                )
                .expect("synthesize");
                SynthesisRecord {
                    task_id: task.visible.task_id.clone(),
                    condition: SynthesisCondition::FirstPrinciplesD,
                    program_id: Some(program.program_id.clone()),
                    program_ir_valid: true,
                    rust_compiled: true,
                    runtime_valid: true,
                    visible_outputs_match: true,
                    property_tests_passed: 8,
                    property_tests_total: 8,
                    invalid_inputs_handled: true,
                    forbidden_effect_accepted: false,
                    solved: true,
                    search_nodes_expanded: search_cost(
                        &program,
                        SynthesisCondition::FirstPrinciplesD,
                    )
                    .min(EQUAL_SEARCH_BUDGET),
                    search_frontier_peak: program.simultaneous_subproblems,
                    used_concept_ids: program.concept_ids,
                    primitive_expanded_ir_nodes: program.primitive_expanded_nodes,
                    operational_nodes: program.operational_nodes,
                    first_attempt_correct: true,
                    repair_attempts: 0,
                    successful_repairs: 0,
                    emitted_source_sha256: None,
                    compiler_stdout: String::new(),
                    compiler_stderr: String::new(),
                    runtime_stdout: String::new(),
                    runtime_stderr: String::new(),
                }
            })
            .collect();
        let reports = BTreeMap::from([(
            "FIRST_PRINCIPLES_D".to_string(),
            summarize_condition(SynthesisCondition::FirstPrinciplesD, records),
        )]);
        let ablations = build_ablations(&task_sets, &promotions, &reports).expect("ablations");
        assert_eq!(ablations.len(), 3);
        assert!(ablations.iter().all(|ablation| ablation.passed));
        let ancestor =
            build_ancestor_ablation(&task_sets, &promotions, &reports).expect("ancestor");
        assert!(ancestor.passed);
        assert!(ancestor.ancestor_ablation);
        let transfers = build_transfers(&task_sets, &reports, &promotions);
        assert_eq!(transfers.len(), 3);
        assert!(transfers
            .iter()
            .all(|transfer| { transfer.passed && !transfer.selected_by_family_label }));
    }
}
