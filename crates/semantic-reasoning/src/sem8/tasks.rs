use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    catalog::extract_source_mechanisms,
    model::{
        AssumptionStatus, Domain, MechanismIR, MechanismTransform, RelationKind, RoleKind,
        SourceSplit, TargetAssumptionEvidence, TargetBehavior, TargetManifest,
        TargetRelationDefinition, TargetRoleDefinition, TransferEvaluatorTask,
        TransferTaskCategory, VisibleTransferTask,
    },
};

pub const TARGET_GENERATOR_VERSION: &str = "SEM8-TARGET-GENERATOR-1.0.0";
pub const BLIND_TRANSFER_TASKS: usize = 120;

#[derive(Debug, Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    fn tag(&mut self) -> String {
        format!("{:08x}", self.next() as u32)
    }
}

pub fn generate_transfer_tasks(seed: u64) -> Vec<TransferEvaluatorTask> {
    let catalog = extract_source_mechanisms();
    let mut rng = Rng::new(seed);
    (0..BLIND_TRANSFER_TASKS)
        .map(|index| build_task(index, &catalog, &mut rng))
        .collect()
}

fn build_task(index: usize, catalog: &[MechanismIR], rng: &mut Rng) -> TransferEvaluatorTask {
    let (category, within) = match index {
        0..=19 => (TransferTaskCategory::MathToProgramState, index),
        20..=39 => (TransferTaskCategory::ProgramToMathState, index - 20),
        40..=59 => (TransferTaskCategory::CrossDataDomain, index - 40),
        60..=79 => (TransferTaskCategory::OpaqueStateMachine, index - 60),
        80..=99 => (TransferTaskCategory::StructuralMimicAdversarial, index - 80),
        _ => (
            TransferTaskCategory::SemanticEquivalentDifferentStructure,
            index - 100,
        ),
    };
    let target_domain = target_domain(category, within);
    let transforms = transforms_for(category, within);
    let sources = transforms
        .iter()
        .map(|transform| {
            catalog
                .iter()
                .find(|mechanism| {
                    mechanism.transform == *transform && mechanism.source_domain != target_domain
                })
                .or_else(|| {
                    catalog
                        .iter()
                        .find(|mechanism| mechanism.transform == *transform)
                })
                .expect("source transform")
        })
        .collect::<Vec<_>>();
    let primary = sources[0];
    let behavior = behavior_for(&transforms, within);
    let parameter = 2 + (within % 5) as i64;
    let semantic_equivalence_different_structure =
        category == TransferTaskCategory::SemanticEquivalentDifferentStructure;
    let invalid_analogy = category == TransferTaskCategory::StructuralMimicAdversarial;
    let mut roles = primary
        .roles
        .iter()
        .enumerate()
        .map(|(role_index, role)| TargetRoleDefinition {
            opaque_role_id: format!("Z{index:03}_{role_index}"),
            kind: role.kind,
            type_class: role.type_class.clone(),
        })
        .collect::<Vec<_>>();
    for source in sources.iter().skip(1) {
        for role in &source.roles {
            if !roles.iter().any(|existing| existing.kind == role.kind) {
                let role_index = roles.len();
                roles.push(TargetRoleDefinition {
                    opaque_role_id: format!("Z{index:03}_{role_index}"),
                    kind: role.kind,
                    type_class: role.type_class.clone(),
                });
            }
        }
    }
    let mut relations = Vec::new();
    for source in &sources {
        let mut source_bindings = std::collections::BTreeMap::new();
        let mut used_target_roles = std::collections::BTreeSet::new();
        for source_role in &source.roles {
            if let Some(target_role) = roles.iter().find(|target_role| {
                target_role.kind == source_role.kind
                    && target_role.type_class == source_role.type_class
                    && !used_target_roles.contains(&target_role.opaque_role_id)
            }) {
                source_bindings.insert(
                    source_role.role_id.clone(),
                    target_role.opaque_role_id.clone(),
                );
                used_target_roles.insert(target_role.opaque_role_id.clone());
            }
        }
        for relation in source.dependency_edges.iter().chain(&source.causal_edges) {
            let Some(target_from) = source_bindings.get(&relation.from_role_id) else {
                continue;
            };
            let Some(target_to) = source_bindings.get(&relation.to_role_id) else {
                continue;
            };
            let candidate = TargetRelationDefinition {
                from_opaque_role_id: target_from.clone(),
                kind: relation.kind,
                to_opaque_role_id: target_to.clone(),
                essential: relation.essential,
            };
            if !relations.contains(&candidate) {
                relations.push(candidate);
            }
        }
    }
    if semantic_equivalence_different_structure {
        roles.push(TargetRoleDefinition {
            opaque_role_id: format!("Z{index:03}_OBS"),
            kind: RoleKind::Observation,
            type_class: "observable_value<T>".to_string(),
        });
        roles.push(TargetRoleDefinition {
            opaque_role_id: format!("Z{index:03}_RES"),
            kind: RoleKind::Resource,
            type_class: "value<T>".to_string(),
        });
        relations.push(TargetRelationDefinition {
            from_opaque_role_id: format!("Z{index:03}_OBS"),
            kind: RelationKind::Precedes,
            to_opaque_role_id: format!("Z{index:03}_RES"),
            essential: false,
        });
    }
    let mut assumption_evidence = Vec::new();
    for assumption in sources.iter().flat_map(|source| &source.assumptions) {
        if !assumption_evidence
            .iter()
            .any(|evidence: &TargetAssumptionEvidence| evidence.kind == assumption.kind)
        {
            assumption_evidence.push(TargetAssumptionEvidence {
                kind: assumption.kind,
                status: AssumptionStatus::Satisfied,
                evidence: format!(
                    "FORMAL_TARGET_PROBE:{}:{:?}:SATISFIED",
                    index, assumption.kind
                ),
            });
        }
    }
    if invalid_analogy {
        if let Some(first) = assumption_evidence.first_mut() {
            first.status = AssumptionStatus::Violated;
            first.evidence = format!("FORMAL_COUNTEREXAMPLE:{}:{:?}:VIOLATED", index, first.kind);
        }
    }
    let base_nodes = primary.roles.len();
    let base_edges = primary.dependency_edges.len() + primary.causal_edges.len();
    let (graph_nodes, graph_edges) = if semantic_equivalence_different_structure {
        (base_nodes + 2, base_edges + 3)
    } else {
        (base_nodes, base_edges)
    };
    let executable_definition = format!(
        "domain={target_domain:?}|roles={roles:?}|relations={relations:?}|behavior={behavior:?}|parameter={parameter}|evidence={assumption_evidence:?}"
    );
    let visible = VisibleTransferTask {
        task_id: format!("SEM8-T-{index:03}-{}", rng.tag()),
        target_domain,
        opaque_entities: vec![
            format!("S{index:03}"),
            format!("R{}", 700 + index),
            format!("op_Q{}", rng.tag()),
            format!("gate_M{}", rng.tag()),
        ],
        roles,
        relations,
        behavior,
        parameter,
        assumption_evidence,
        graph_nodes,
        graph_edges,
        primitive_set_sha256: hash_bytes(
            format!("PRIMITIVES:{target_domain:?}:{behavior:?}:{index}").as_bytes(),
        ),
        executable_definition_sha256: hash_bytes(executable_definition.as_bytes()),
        zero_target_examples: matches!(
            category,
            TransferTaskCategory::OpaqueStateMachine
                | TransferTaskCategory::SemanticEquivalentDifferentStructure
        ),
        target_solution_included: false,
        source_mechanism_id_included: false,
        intended_analogy_included: false,
        correct_role_mapping_included: false,
        transfer_family_included: false,
        frozen: true,
    };
    let target_only_expansions_required = if within % 4 == 0 {
        48 + visible.roles.len()
    } else {
        92 + visible.roles.len() * 7 + within
    };
    TransferEvaluatorTask {
        visible,
        category,
        compatible_transforms: transforms,
        expected_source_count: sources.len(),
        invalid_analogy,
        semantic_equivalence_different_structure,
        hidden_inputs: hidden_inputs(behavior, within),
        target_only_expansions_required,
        transfer_expansions_required: 24 + base_nodes * 2 + sources.len() * 3,
        source_split_required: if primary.mechanism_id.as_str() >= "M0005" {
            SourceSplit::Blind
        } else {
            SourceSplit::Development
        },
    }
}

fn target_domain(category: TransferTaskCategory, within: usize) -> Domain {
    match category {
        TransferTaskCategory::MathToProgramState => {
            if within.is_multiple_of(2) {
                Domain::Programming
            } else {
                Domain::StatefulMachine
            }
        }
        TransferTaskCategory::ProgramToMathState => {
            if within.is_multiple_of(2) {
                Domain::Mathematics
            } else {
                Domain::StatefulMachine
            }
        }
        TransferTaskCategory::CrossDataDomain => Domain::DataTransform,
        TransferTaskCategory::OpaqueStateMachine => Domain::StatefulMachine,
        TransferTaskCategory::StructuralMimicAdversarial => match within % 4 {
            0 => Domain::Programming,
            1 => Domain::Mathematics,
            2 => Domain::StatefulMachine,
            _ => Domain::DataTransform,
        },
        TransferTaskCategory::SemanticEquivalentDifferentStructure => match within % 4 {
            0 => Domain::StatefulMachine,
            1 => Domain::DataTransform,
            2 => Domain::Mathematics,
            _ => Domain::Programming,
        },
    }
}

fn transforms_for(category: TransferTaskCategory, within: usize) -> Vec<MechanismTransform> {
    match category {
        TransferTaskCategory::MathToProgramState => {
            if within.is_multiple_of(2) {
                vec![MechanismTransform::StateEvolution]
            } else {
                vec![MechanismTransform::ReversibleStateTransform]
            }
        }
        TransferTaskCategory::ProgramToMathState => match within % 4 {
            0 => vec![MechanismTransform::ElementwiseTransform],
            1 => vec![MechanismTransform::GuardedTraversal],
            2 => vec![MechanismTransform::StatefulReduction],
            _ => vec![MechanismTransform::StageComposition],
        },
        TransferTaskCategory::CrossDataDomain => match within % 4 {
            0 => vec![MechanismTransform::QuotientPartition],
            1 => vec![MechanismTransform::ScopedRelation],
            2 => vec![MechanismTransform::StateEvolution],
            _ => vec![MechanismTransform::StageComposition],
        },
        TransferTaskCategory::OpaqueStateMachine => match within % 5 {
            0 => vec![MechanismTransform::StateEvolution],
            1 => vec![MechanismTransform::ReversibleStateTransform],
            2 => vec![MechanismTransform::QuotientPartition],
            3 => vec![MechanismTransform::ScopedRelation],
            _ => vec![
                MechanismTransform::ElementwiseTransform,
                MechanismTransform::StatefulReduction,
            ],
        },
        TransferTaskCategory::StructuralMimicAdversarial => match within % 4 {
            0 => vec![MechanismTransform::StateEvolution],
            1 => vec![MechanismTransform::ElementwiseTransform],
            2 => vec![MechanismTransform::ReversibleStateTransform],
            _ => vec![MechanismTransform::StageComposition],
        },
        TransferTaskCategory::SemanticEquivalentDifferentStructure => match within % 4 {
            0 => vec![MechanismTransform::StageComposition],
            1 => vec![MechanismTransform::QuotientPartition],
            2 => vec![MechanismTransform::ScopedRelation],
            _ => vec![MechanismTransform::ReversibleStateTransform],
        },
    }
}

fn behavior_for(transforms: &[MechanismTransform], within: usize) -> TargetBehavior {
    if transforms.len() == 2 {
        return TargetBehavior::MapThenSum;
    }
    match transforms[0] {
        MechanismTransform::StateEvolution => TargetBehavior::StateDelta,
        MechanismTransform::ElementwiseTransform => {
            if within.is_multiple_of(2) {
                TargetBehavior::AddEach
            } else {
                TargetBehavior::MultiplyEach
            }
        }
        MechanismTransform::GuardedTraversal => TargetBehavior::FilterGreater,
        MechanismTransform::StatefulReduction => TargetBehavior::Sum,
        MechanismTransform::StageComposition => TargetBehavior::ComposeDeltas,
        MechanismTransform::QuotientPartition => TargetBehavior::QuotientClass,
        MechanismTransform::ScopedRelation => TargetBehavior::ScopedIdentity,
        MechanismTransform::ReversibleStateTransform => TargetBehavior::ReverseDelta,
    }
}

fn hidden_inputs(behavior: TargetBehavior, within: usize) -> Vec<Vec<i64>> {
    if behavior == TargetBehavior::QuotientClass {
        return vec![vec![100], vec![204], vec![404], vec![599]];
    }
    vec![
        vec![1, 2, 3, 4],
        vec![-2, 0, 2, 6],
        vec![within as i64 + 3],
        vec![5, 5, 1, 9],
    ]
}

pub fn build_target_manifest(
    run_id: &str,
    seed: u64,
    tasks: &[TransferEvaluatorTask],
) -> TargetManifest {
    let visible = tasks
        .iter()
        .map(|task| task.visible.clone())
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        generator_version: &'a str,
        seed_commitment_sha256: String,
        tasks: &'a [VisibleTransferTask],
        target_answers_included: bool,
        source_target_pairs_included: bool,
        evaluator_categories_included: bool,
        hidden_cases_included: bool,
        frozen_before_evaluation: bool,
    }
    let seed_commitment_sha256 = hash_bytes(&seed.to_le_bytes());
    let commitment = Commitment {
        run_id,
        generator_version: TARGET_GENERATOR_VERSION,
        seed_commitment_sha256: seed_commitment_sha256.clone(),
        tasks: &visible,
        target_answers_included: false,
        source_target_pairs_included: false,
        evaluator_categories_included: false,
        hidden_cases_included: false,
        frozen_before_evaluation: true,
    };
    let manifest_sha256 = hash_bytes(&serde_json::to_vec(&commitment).expect("commitment"));
    drop(commitment);
    TargetManifest {
        run_id: run_id.to_string(),
        generator_version: TARGET_GENERATOR_VERSION.to_string(),
        seed_commitment_sha256,
        tasks: visible,
        target_answers_included: false,
        source_target_pairs_included: false,
        evaluator_categories_included: false,
        hidden_cases_included: false,
        frozen_before_evaluation: true,
        manifest_sha256,
    }
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn blind_targets_are_balanced_opaque_and_metadata_free() {
        let tasks = generate_transfer_tasks(81);
        assert_eq!(tasks.len(), BLIND_TRANSFER_TASKS);
        let counts = tasks.iter().fold(BTreeMap::new(), |mut counts, task| {
            *counts.entry(task.category).or_insert(0usize) += 1;
            counts
        });
        assert_eq!(counts.len(), 6);
        assert!(counts.values().all(|count| *count == 20));
        assert_eq!(
            tasks
                .iter()
                .filter(|task| task.visible.zero_target_examples)
                .count(),
            40
        );
        assert_eq!(tasks.iter().filter(|task| task.invalid_analogy).count(), 20);
        assert!(tasks.iter().all(|task| {
            !task.visible.target_solution_included
                && !task.visible.source_mechanism_id_included
                && !task.visible.intended_analogy_included
                && !task.visible.correct_role_mapping_included
                && !task.visible.transfer_family_included
                && task.visible.frozen
        }));
    }

    #[test]
    fn opaque_state_machine_uses_only_formal_roles_and_novel_entities() {
        let tasks = generate_transfer_tasks(82);
        let opaque = tasks
            .iter()
            .filter(|task| task.category == TransferTaskCategory::OpaqueStateMachine);
        assert!(opaque.clone().all(|task| task.visible.zero_target_examples));
        assert!(opaque
            .flat_map(|task| &task.visible.opaque_entities)
            .all(|name| {
                name.starts_with('S')
                    || name.starts_with('R')
                    || name.starts_with("op_Q")
                    || name.starts_with("gate_M")
            }));
    }
}
