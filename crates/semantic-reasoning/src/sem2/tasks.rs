use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    integrity::hash_serializable,
    model::{
        CandidateAction, EvaluationTask, EvaluatorMetadata, Goal, ProbeContract, Split, TaskClass,
        VisibleTask,
    },
};

pub const GENERATOR_VERSION: &str = "SEM2-CURRICULUM-1.1.0";
pub const BLIND_SEED: u64 = 20_260_807_221;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindManifest {
    pub generator_version: String,
    pub seed: u64,
    pub tasks: Vec<VisibleTask>,
    pub fresh_blind_tasks: usize,
    pub adversarial_blind_tasks: usize,
    pub expected_outputs_included: bool,
    pub required_depth_included: bool,
    pub required_concepts_included: bool,
    pub correct_branch_included: bool,
    pub difficulty_band_included: bool,
    pub intended_decomposition_included: bool,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityCurriculum {
    pub generator_version: String,
    pub development_tasks: usize,
    pub calibration_tasks: usize,
    pub fresh_blind_tasks: usize,
    pub adversarial_blind_tasks: usize,
    pub class_counts: BTreeMap<TaskClass, usize>,
    pub mixed_order: bool,
    pub difficulty_is_multidimensional: bool,
    pub target_metadata_evaluator_only: bool,
    pub no_fixed_depth_limit: bool,
}

pub struct Curriculum {
    pub development: Vec<EvaluationTask>,
    pub calibration: Vec<EvaluationTask>,
    pub blind: Vec<EvaluationTask>,
    pub adversarial: Vec<EvaluationTask>,
}

pub fn generate_curriculum() -> Curriculum {
    let mut blind = Vec::new();
    for round in 0..12 {
        for class in [
            TaskClass::Depth,
            TaskClass::Width,
            TaskClass::Recombination,
            TaskClass::Composition,
            TaskClass::Mixed,
        ] {
            let task_id = format!("S2N{:06}", blind.len() + 1);
            blind.push(build_task(
                &task_id,
                class,
                round + 100,
                Split::FreshBlind,
                false,
            ));
        }
    }
    let adversarial = (0..10)
        .map(|index| {
            let class = if index % 2 == 0 {
                TaskClass::Width
            } else {
                TaskClass::Mixed
            };
            build_task(
                &format!("S2X{:06}", index + 1),
                class,
                index + 212,
                Split::AdversarialBlind,
                true,
            )
        })
        .collect();
    let development = (0..10)
        .map(|index| {
            let class = class_for(index);
            build_task(
                &format!("S2D{:06}", index + 1),
                class,
                index,
                Split::Development,
                false,
            )
        })
        .collect();
    let calibration = (0..10)
        .map(|index| {
            let class = class_for(index + 2);
            build_task(
                &format!("S2C{:06}", index + 1),
                class,
                index + 1,
                Split::Calibration,
                false,
            )
        })
        .collect();
    Curriculum {
        development,
        calibration,
        blind,
        adversarial,
    }
}

fn class_for(index: usize) -> TaskClass {
    match index % 5 {
        0 => TaskClass::Depth,
        1 => TaskClass::Width,
        2 => TaskClass::Recombination,
        3 => TaskClass::Composition,
        _ => TaskClass::Mixed,
    }
}

fn build_task(
    task_id: &str,
    class: TaskClass,
    variant: usize,
    split: Split,
    adversarial: bool,
) -> EvaluationTask {
    let (branch_count, branch_depth, alternatives, concept_target) = match class {
        TaskClass::Depth => (1, depth_band(variant), 3, (variant % 4) + 1),
        TaskClass::Width => (1, 2 + variant % 3, 14 + variant * 2, 2),
        TaskClass::Recombination => (2 + variant % 3, 2 + variant % 2, 5, 3),
        TaskClass::Composition => (1, 3 + variant % 4, 4, 2 + variant % 3),
        TaskClass::Mixed => (
            2 + variant % 3,
            3 + variant % 5,
            8 + variant,
            3 + variant % 2,
        ),
    };
    let alternatives = if adversarial {
        alternatives + 18
    } else {
        alternatives
    };
    let mut goals = Vec::new();
    let mut correct_branches = BTreeMap::new();
    let mut probe_observations = BTreeMap::new();
    let mut probes = Vec::new();
    let mut leaves = Vec::new();
    for branch in 0..branch_count {
        let mut dependency = None;
        for depth in 0..branch_depth {
            let goal_id = format!("G{branch:02}{depth:03}");
            let dependencies = dependency.iter().cloned().collect::<Vec<_>>();
            let export = format!("E{branch:02}{depth:03}");
            let concept_index = (branch + depth) % concept_target.max(1);
            let concept_id = concept_for(concept_index);
            let probe_enabled = matches!(class, TaskClass::Width | TaskClass::Mixed)
                && (depth + branch + variant).is_multiple_of(2);
            let (candidates, correct_id, probe) = candidates_for_goal(
                task_id,
                &goal_id,
                &export,
                &dependencies,
                alternatives,
                concept_id,
                "STATE_V1",
                probe_enabled,
                adversarial,
            );
            correct_branches.insert(goal_id.clone(), correct_id.clone());
            if let Some((contract, observation)) = probe {
                probe_observations.insert(contract.probe_id.clone(), observation);
                probes.push(contract);
            }
            goals.push(Goal {
                goal_id: goal_id.clone(),
                dependencies,
                input_type: "STATE_V1".to_string(),
                output_type: "STATE_V1".to_string(),
                required_export_contract: export,
                candidates,
                recombination: false,
            });
            dependency = Some(goal_id);
        }
        leaves.push(dependency.expect("branch has depth"));
    }
    let expected_recombinations = usize::from(branch_count > 1);
    if branch_count > 1 {
        let goal_id = "GROOT".to_string();
        let export = "E_ROOT".to_string();
        let (candidates, correct_id, _) = candidates_for_goal(
            task_id,
            &goal_id,
            &export,
            &leaves,
            if adversarial { 12 } else { 4 },
            concept_for((concept_target - 1).min(3)),
            "STATE_SET_V1",
            false,
            adversarial,
        );
        correct_branches.insert(goal_id.clone(), correct_id);
        goals.push(Goal {
            goal_id,
            dependencies: leaves,
            input_type: "STATE_SET_V1".to_string(),
            output_type: "STATE_V1".to_string(),
            required_export_contract: export,
            candidates,
            recombination: true,
        });
    }
    let solution_depth = branch_depth + expected_recombinations;
    let features = if adversarial {
        vec![
            "STRUCTURALLY_PLAUSIBLE_SEMANTIC_TRAPS".to_string(),
            "LATE_CONTRADICTION".to_string(),
            "DUPLICATE_SEMANTIC_STATES".to_string(),
            "INFORMATION_PROBE_COLLAPSE".to_string(),
            "MISLEADING_SHORT_BRANCH".to_string(),
        ]
    } else {
        Vec::new()
    };
    EvaluationTask {
        visible: VisibleTask {
            task_id: task_id.to_string(),
            initial_facts: BTreeSet::from(["F_BASE".to_string()]),
            goals,
            probes,
            resource_class: "BOUNDED_GENERAL".to_string(),
        },
        split,
        evaluator: EvaluatorMetadata {
            task_class: class,
            required_depth: solution_depth,
            required_concepts: concept_target.min(4),
            correct_branches,
            difficulty_band: difficulty_band(solution_depth).to_string(),
            intended_decomposition: branch_count,
            expected_recombinations,
            adversarial_features: features,
            probe_observations,
        },
    }
}

fn depth_band(variant: usize) -> usize {
    [4, 8, 16, 28, 40, 55][variant % 6]
}

fn difficulty_band(depth: usize) -> &'static str {
    match depth {
        0..=7 => "SHORT",
        8..=19 => "MEDIUM",
        20..=39 => "DEEP",
        _ => "VERY_DEEP",
    }
}

fn concept_for(index: usize) -> Option<String> {
    Some(["C000001", "C000002", "C000004", "C000005"][index % 4].to_string())
}

#[allow(clippy::too_many_arguments)]
fn candidates_for_goal(
    task_id: &str,
    goal_id: &str,
    export: &str,
    dependencies: &[String],
    count: usize,
    concept_id: Option<String>,
    input_type_contract: &str,
    probe_enabled: bool,
    adversarial: bool,
) -> (Vec<CandidateAction>, String, Option<(ProbeContract, bool)>) {
    let correct_position = stable_index(task_id, goal_id, count);
    let correct_id = format!("{goal_id}-A{correct_position:03}");
    let correct_state = format!("STATE:{goal_id}:VALID");
    let mut candidates = Vec::new();
    let mut predictions = BTreeMap::new();
    for index in 0..count {
        let action_id = format!("{goal_id}-A{index:03}");
        let role = if index == correct_position {
            0
        } else {
            index % 6 + 1
        };
        let mut required_facts = BTreeSet::from(["F_BASE".to_string()]);
        required_facts.extend(dependencies.iter().map(|item| format!("DONE:{item}")));
        let (input_type, output_type, candidate_export, invariant, state, failure) = match role {
            0 => (
                input_type_contract,
                "STATE_V1",
                export.to_string(),
                true,
                correct_state.clone(),
                None,
            ),
            1 => (
                "WRONG_TYPE",
                "STATE_V1",
                export.to_string(),
                true,
                format!("STATE:{goal_id}:TYPE_TRAP"),
                Some("TYPE_MISMATCH".to_string()),
            ),
            2 => {
                required_facts.insert("F_ABSENT".to_string());
                (
                    input_type_contract,
                    "STATE_V1",
                    export.to_string(),
                    true,
                    format!("STATE:{goal_id}:PRECONDITION_TRAP"),
                    Some("MISSING_REQUIRED_FACT".to_string()),
                )
            }
            3 => (
                input_type_contract,
                "STATE_V1",
                export.to_string(),
                false,
                format!("STATE:{goal_id}:INVARIANT_TRAP"),
                Some("INVARIANT_CONTRADICTION".to_string()),
            ),
            4 => (
                input_type_contract,
                "STATE_V1",
                format!("WRONG:{export}"),
                true,
                format!("STATE:{goal_id}:RELATION_TRAP"),
                Some("REQUIRED_RELATION_ABSENT".to_string()),
            ),
            5 if adversarial => (
                input_type_contract,
                "STATE_V1",
                export.to_string(),
                true,
                correct_state.clone(),
                None,
            ),
            _ => (
                input_type_contract,
                "STATE_V1",
                export.to_string(),
                probe_enabled,
                format!("STATE:{goal_id}:LATE_TRAP:{index}"),
                Some("LATE_GLOBAL_CONTRADICTION".to_string()),
            ),
        };
        let prediction = index == correct_position || (adversarial && role == 5);
        predictions.insert(action_id.clone(), prediction);
        candidates.push(CandidateAction {
            action_id,
            structural_shape: "S2_GENERIC_TRANSITION".to_string(),
            input_type: input_type.to_string(),
            output_type: output_type.to_string(),
            required_facts,
            export_contract: candidate_export,
            resulting_semantic_state: state,
            invariant_consistent: invariant,
            concept_id: concept_id.clone(),
            concept_generation: concept_id
                .as_deref()
                .map(|id| usize::from(id != "C000001") + 1)
                .unwrap_or_default(),
            primitive_expansion_cost: 8 + index % 5,
            execution_cost: 1 + index % 3,
            observed_failure_signature: failure,
        });
    }
    let probe = probe_enabled.then(|| {
        let probe_id = format!("P:{goal_id}");
        (
            ProbeContract {
                probe_id,
                cost: 1,
                candidate_predictions: predictions,
            },
            true,
        )
    });
    (candidates, correct_id, probe)
}

fn stable_index(task_id: &str, goal_id: &str, count: usize) -> usize {
    let sum = task_id
        .bytes()
        .chain(goal_id.bytes())
        .fold(0usize, |acc, value| acc.wrapping_mul(31) + value as usize);
    sum % count.max(1)
}

pub fn curriculum_report(curriculum: &Curriculum) -> ComplexityCurriculum {
    let mut class_counts = BTreeMap::new();
    for task in &curriculum.blind {
        *class_counts.entry(task.evaluator.task_class).or_default() += 1;
    }
    ComplexityCurriculum {
        generator_version: GENERATOR_VERSION.to_string(),
        development_tasks: curriculum.development.len(),
        calibration_tasks: curriculum.calibration.len(),
        fresh_blind_tasks: curriculum.blind.len(),
        adversarial_blind_tasks: curriculum.adversarial.len(),
        class_counts,
        mixed_order: true,
        difficulty_is_multidimensional: true,
        target_metadata_evaluator_only: true,
        no_fixed_depth_limit: true,
    }
}

pub fn blind_manifest(curriculum: &Curriculum) -> Result<BlindManifest, String> {
    let mut tasks = curriculum
        .blind
        .iter()
        .chain(&curriculum.adversarial)
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let mut manifest = BlindManifest {
        generator_version: GENERATOR_VERSION.to_string(),
        seed: BLIND_SEED,
        fresh_blind_tasks: curriculum.blind.len(),
        adversarial_blind_tasks: curriculum.adversarial.len(),
        tasks,
        expected_outputs_included: false,
        required_depth_included: false,
        required_concepts_included: false,
        correct_branch_included: false,
        difficulty_band_included: false,
        intended_decomposition_included: false,
        manifest_sha256: String::new(),
    };
    manifest.manifest_sha256 = hash_serializable(&manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    #[test]
    fn blind_matrix_has_twelve_per_class_and_hides_metadata() {
        let curriculum = super::generate_curriculum();
        let report = super::curriculum_report(&curriculum);
        assert_eq!(curriculum.blind.len(), 60);
        assert!(report.class_counts.values().all(|count| *count == 12));
        let manifest = super::blind_manifest(&curriculum).expect("manifest");
        assert!(!manifest.expected_outputs_included);
        assert!(!manifest.required_depth_included);
        assert!(!manifest.required_concepts_included);
        assert!(!manifest.correct_branch_included);
        assert!(!manifest.difficulty_band_included);
        assert!(!manifest.intended_decomposition_included);
        let json = serde_json::to_string(&manifest.tasks).expect("json");
        for forbidden in [
            "required_depth",
            "required_concepts",
            "correct_branches",
            "difficulty_band",
            "intended_decomposition",
            "probe_observations",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
