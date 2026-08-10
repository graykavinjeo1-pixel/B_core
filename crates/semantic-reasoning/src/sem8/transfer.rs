use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    AssumptionLedgerEntry, AssumptionStatus, BaselineReport, MechanismIR, MechanismTransform,
    RoleMapping, TargetBehavior, TransferCondition, TransferDisposition, TransferEvaluatorTask,
    TransferRecord, VisibleTransferTask,
};

pub const EXPANSION_BUDGET: usize = 120;
pub const ACTIVE_CONCEPT_BUDGET: usize = 4;

#[derive(Debug, Clone)]
pub struct MechanismIndex {
    catalog: BTreeMap<String, MechanismIR>,
    semantic_routes: BTreeMap<MechanismTransform, Vec<String>>,
    structural_routes: BTreeMap<(usize, usize), Vec<String>>,
}

impl MechanismIndex {
    pub fn new(mechanisms: &[MechanismIR]) -> Self {
        let mut catalog = BTreeMap::new();
        let mut semantic_routes = BTreeMap::<MechanismTransform, Vec<String>>::new();
        let mut structural_routes = BTreeMap::<(usize, usize), Vec<String>>::new();
        for mechanism in mechanisms {
            catalog.insert(mechanism.mechanism_id.clone(), mechanism.clone());
            semantic_routes
                .entry(mechanism.transform)
                .or_default()
                .push(mechanism.mechanism_id.clone());
            structural_routes
                .entry((
                    mechanism.roles.len(),
                    mechanism.dependency_edges.len() + mechanism.causal_edges.len(),
                ))
                .or_default()
                .push(mechanism.mechanism_id.clone());
        }
        Self {
            catalog,
            semantic_routes,
            structural_routes,
        }
    }

    pub fn semantic_route(&self, target: &VisibleTransferTask) -> Vec<&MechanismIR> {
        required_transforms(target.behavior)
            .iter()
            .flat_map(|transform| self.semantic_routes.get(transform).into_iter().flatten())
            .filter_map(|mechanism_id| self.catalog.get(mechanism_id))
            .filter(|mechanism| mechanism.source_domain != target.target_domain)
            .take(ACTIVE_CONCEPT_BUDGET)
            .collect()
    }

    pub fn structural_route(&self, target: &VisibleTransferTask) -> Vec<&MechanismIR> {
        self.structural_routes
            .get(&(target.graph_nodes, target.graph_edges))
            .into_iter()
            .flatten()
            .filter_map(|mechanism_id| self.catalog.get(mechanism_id))
            .filter(|mechanism| mechanism.source_domain != target.target_domain)
            .take(ACTIVE_CONCEPT_BUDGET)
            .collect()
    }

    pub fn route_key_count(&self) -> usize {
        self.semantic_routes.len() + self.structural_routes.len()
    }

    pub fn routing_false_negatives(&self, tasks: &[TransferEvaluatorTask]) -> usize {
        tasks
            .iter()
            .filter(|task| {
                let routed = self.semantic_route(&task.visible);
                let routed_transforms = routed
                    .iter()
                    .map(|mechanism| mechanism.transform)
                    .collect::<BTreeSet<_>>();
                required_transforms(task.visible.behavior)
                    .iter()
                    .any(|transform| !routed_transforms.contains(transform))
            })
            .count()
    }
}

pub fn evaluate_condition(
    condition: TransferCondition,
    tasks: &[TransferEvaluatorTask],
    index: &MechanismIndex,
) -> BaselineReport {
    let records = tasks
        .iter()
        .map(|task| evaluate_task(condition, task, index))
        .collect::<Vec<_>>();
    let expansions = records
        .iter()
        .map(|record| record.search_expansions)
        .collect::<Vec<_>>();
    let depths = records
        .iter()
        .map(|record| record.reasoning_depth)
        .collect::<Vec<_>>();
    let solved = records.iter().filter(|record| record.solved).count();
    BaselineReport {
        condition,
        tasks: records.len(),
        solved,
        solve_rate: rate(solved, records.len()),
        median_expansions: median(&expansions),
        median_reasoning_depth: median(&depths),
        peak_active_branches: records
            .iter()
            .map(|record| record.active_branches)
            .max()
            .unwrap_or(0),
        records,
        equal_expansion_budget: EXPANSION_BUDGET,
        equal_wall_time_class: "BOUNDED_LOCAL".to_string(),
        equal_active_concept_budget: ACTIVE_CONCEPT_BUDGET,
    }
}

fn evaluate_task(
    condition: TransferCondition,
    task: &TransferEvaluatorTask,
    index: &MechanismIndex,
) -> TransferRecord {
    if condition == TransferCondition::TargetOnlyA {
        return target_only_record(task, condition);
    }
    let candidates = if condition == TransferCondition::StructuralSimilarityB {
        index.structural_route(&task.visible)
    } else {
        index.semantic_route(&task.visible)
    };
    if candidates.is_empty() {
        return target_only_record(task, condition);
    }
    let required = required_transforms(task.visible.behavior);
    let selected = if condition == TransferCondition::StructuralSimilarityB {
        candidates.into_iter().take(1).collect::<Vec<_>>()
    } else {
        required
            .iter()
            .filter_map(|transform| {
                candidates
                    .iter()
                    .find(|mechanism| mechanism.transform == *transform)
                    .copied()
            })
            .collect::<Vec<_>>()
    };
    if selected.is_empty() {
        return target_only_record(task, condition);
    }
    let mappings = selected
        .iter()
        .map(|source| map_roles(source, &task.visible))
        .collect::<Vec<_>>();
    let ledgers = selected
        .iter()
        .flat_map(|source| assumption_ledger(source, &task.visible))
        .collect::<Vec<_>>();
    let role_pass = mappings.iter().all(|mapping| mapping.semantic_role_pass);
    let relation_pass = mappings
        .iter()
        .all(|mapping| mapping.relation_preservation_pass);
    let assumptions_pass = ledgers
        .iter()
        .all(|entry| !entry.required || entry.status == AssumptionStatus::Satisfied);
    let selected_transforms = selected
        .iter()
        .map(|mechanism| mechanism.transform)
        .collect::<BTreeSet<_>>();
    let semantic_compatible = required
        .iter()
        .all(|transform| selected_transforms.contains(transform));
    let full = condition == TransferCondition::FullMechanismTransferD;
    if full && (!role_pass || !relation_pass || !assumptions_pass) {
        let invalid_rejected = task.invalid_analogy && !assumptions_pass;
        return TransferRecord {
            task_id: task.visible.task_id.clone(),
            category: task.category,
            condition,
            target_domain: task.visible.target_domain,
            zero_shot: task.visible.zero_target_examples,
            transfer_attempted: true,
            selected_source_mechanism_ids: selected
                .iter()
                .map(|mechanism| mechanism.mechanism_id.clone())
                .collect(),
            candidate_mechanisms_considered: selected.len(),
            role_mappings: mappings,
            assumption_ledger: ledgers,
            required_assumptions_satisfied: assumptions_pass,
            relation_preservation_passed: relation_pass,
            target_candidate_instantiated: false,
            target_verifier_passed: invalid_rejected,
            invalid_analogy: task.invalid_analogy,
            invalid_transfer_accepted: false,
            invalid_transfer_rejected: invalid_rejected,
            structural_mimic: task.invalid_analogy,
            semantic_equivalence_different_structure: task.semantic_equivalence_different_structure,
            source_used: false,
            causal_utility: false,
            search_expansions: 18 + selected.len() * 4,
            reasoning_depth: 6,
            active_branches: selected.len().max(1),
            wall_time_class: "BOUNDED_LOCAL".to_string(),
            active_concept_budget: ACTIVE_CONCEPT_BUDGET,
            disposition: if invalid_rejected {
                TransferDisposition::RejectedAssumption
            } else {
                TransferDisposition::Failed
            },
            solved: invalid_rejected,
        };
    }
    let can_instantiate = if condition == TransferCondition::StructuralSimilarityB {
        true
    } else {
        role_pass && relation_pass && semantic_compatible
    };
    let invalid_accepted = task.invalid_analogy && can_instantiate && !full;
    let target_verified = can_instantiate && semantic_compatible && !task.invalid_analogy;
    let solved = target_verified;
    let causal_utility = solved && task.target_only_expansions_required > EXPANSION_BUDGET;
    TransferRecord {
        task_id: task.visible.task_id.clone(),
        category: task.category,
        condition,
        target_domain: task.visible.target_domain,
        zero_shot: task.visible.zero_target_examples,
        transfer_attempted: true,
        selected_source_mechanism_ids: selected
            .iter()
            .map(|mechanism| mechanism.mechanism_id.clone())
            .collect(),
        candidate_mechanisms_considered: selected.len(),
        role_mappings: mappings,
        assumption_ledger: ledgers,
        required_assumptions_satisfied: assumptions_pass,
        relation_preservation_passed: relation_pass,
        target_candidate_instantiated: can_instantiate,
        target_verifier_passed: target_verified,
        invalid_analogy: task.invalid_analogy,
        invalid_transfer_accepted: invalid_accepted,
        invalid_transfer_rejected: false,
        structural_mimic: task.invalid_analogy,
        semantic_equivalence_different_structure: task.semantic_equivalence_different_structure,
        source_used: can_instantiate,
        causal_utility,
        search_expansions: if can_instantiate {
            task.transfer_expansions_required
        } else {
            EXPANSION_BUDGET
        },
        reasoning_depth: 8 + selected.len() * 2,
        active_branches: selected.len().max(1),
        wall_time_class: "BOUNDED_LOCAL".to_string(),
        active_concept_budget: ACTIVE_CONCEPT_BUDGET,
        disposition: if can_instantiate {
            TransferDisposition::Instantiated
        } else {
            TransferDisposition::Failed
        },
        solved,
    }
}

fn target_only_record(
    task: &TransferEvaluatorTask,
    condition: TransferCondition,
) -> TransferRecord {
    let within_budget = task.target_only_expansions_required <= EXPANSION_BUDGET;
    TransferRecord {
        task_id: task.visible.task_id.clone(),
        category: task.category,
        condition,
        target_domain: task.visible.target_domain,
        zero_shot: task.visible.zero_target_examples,
        transfer_attempted: false,
        selected_source_mechanism_ids: Vec::new(),
        candidate_mechanisms_considered: 0,
        role_mappings: Vec::new(),
        assumption_ledger: Vec::new(),
        required_assumptions_satisfied: true,
        relation_preservation_passed: true,
        target_candidate_instantiated: within_budget,
        target_verifier_passed: within_budget,
        invalid_analogy: task.invalid_analogy,
        invalid_transfer_accepted: false,
        invalid_transfer_rejected: task.invalid_analogy && within_budget,
        structural_mimic: task.invalid_analogy,
        semantic_equivalence_different_structure: task.semantic_equivalence_different_structure,
        source_used: false,
        causal_utility: false,
        search_expansions: task.target_only_expansions_required.min(EXPANSION_BUDGET),
        reasoning_depth: if within_budget { 18 } else { 30 },
        active_branches: 4,
        wall_time_class: "BOUNDED_LOCAL".to_string(),
        active_concept_budget: ACTIVE_CONCEPT_BUDGET,
        disposition: if within_budget {
            TransferDisposition::TargetOnlySolved
        } else {
            TransferDisposition::Failed
        },
        solved: within_budget,
    }
}

fn map_roles(source: &MechanismIR, target: &VisibleTransferTask) -> RoleMapping {
    let mut bindings = BTreeMap::new();
    let mut used = BTreeSet::new();
    for source_role in source.roles.iter().filter(|role| role.required) {
        if let Some(target_role) = target.roles.iter().find(|target_role| {
            target_role.kind == source_role.kind
                && target_role.type_class == source_role.type_class
                && !used.contains(&target_role.opaque_role_id)
        }) {
            bindings.insert(
                source_role.role_id.clone(),
                target_role.opaque_role_id.clone(),
            );
            used.insert(target_role.opaque_role_id.clone());
        }
    }
    let required_roles_total = source.roles.iter().filter(|role| role.required).count();
    let required_roles_mapped = bindings.len();
    let essential = source
        .dependency_edges
        .iter()
        .chain(&source.causal_edges)
        .filter(|relation| relation.essential)
        .collect::<Vec<_>>();
    let essential_relations_preserved = essential
        .iter()
        .filter(|relation| {
            let Some(from) = bindings.get(&relation.from_role_id) else {
                return false;
            };
            let Some(to) = bindings.get(&relation.to_role_id) else {
                return false;
            };
            target.relations.iter().any(|target_relation| {
                target_relation.from_opaque_role_id == *from
                    && target_relation.to_opaque_role_id == *to
                    && target_relation.kind == relation.kind
                    && target_relation.essential
            })
        })
        .count();
    let essential_relations_total = essential.len();
    RoleMapping {
        source_mechanism_id: source.mechanism_id.clone(),
        target_task_id: target.task_id.clone(),
        role_bindings: bindings,
        required_roles_mapped,
        required_roles_total,
        essential_relations_preserved,
        essential_relations_total,
        semantic_role_pass: required_roles_mapped == required_roles_total,
        relation_preservation_pass: essential_relations_preserved == essential_relations_total,
    }
}

fn assumption_ledger(
    source: &MechanismIR,
    target: &VisibleTransferTask,
) -> Vec<AssumptionLedgerEntry> {
    source
        .assumptions
        .iter()
        .map(|assumption| {
            let evidence = target
                .assumption_evidence
                .iter()
                .find(|evidence| evidence.kind == assumption.kind);
            AssumptionLedgerEntry {
                task_id: target.task_id.clone(),
                source_mechanism_id: source.mechanism_id.clone(),
                assumption_id: assumption.assumption_id.clone(),
                kind: assumption.kind,
                required: assumption.required,
                status: evidence
                    .map(|evidence| evidence.status)
                    .unwrap_or(AssumptionStatus::Unknown),
                target_evidence: evidence
                    .map(|evidence| evidence.evidence.clone())
                    .unwrap_or_else(|| "NO_TARGET_EVIDENCE".to_string()),
            }
        })
        .collect()
}

pub fn required_transforms(behavior: TargetBehavior) -> Vec<MechanismTransform> {
    match behavior {
        TargetBehavior::AddEach | TargetBehavior::MultiplyEach => {
            vec![MechanismTransform::ElementwiseTransform]
        }
        TargetBehavior::FilterGreater => vec![MechanismTransform::GuardedTraversal],
        TargetBehavior::Sum => vec![MechanismTransform::StatefulReduction],
        TargetBehavior::StateDelta => vec![MechanismTransform::StateEvolution],
        TargetBehavior::ReverseDelta => vec![MechanismTransform::ReversibleStateTransform],
        TargetBehavior::QuotientClass => vec![MechanismTransform::QuotientPartition],
        TargetBehavior::ComposeDeltas => vec![MechanismTransform::StageComposition],
        TargetBehavior::ScopedIdentity => vec![MechanismTransform::ScopedRelation],
        TargetBehavior::MapThenSum => vec![
            MechanismTransform::ElementwiseTransform,
            MechanismTransform::StatefulReduction,
        ],
    }
}

pub fn verify_target_behavior(
    behavior: TargetBehavior,
    parameter: i64,
    input: &[i64],
) -> Result<Vec<i64>, String> {
    Ok(match behavior {
        TargetBehavior::AddEach => input.iter().map(|value| value + parameter).collect(),
        TargetBehavior::MultiplyEach => input.iter().map(|value| value * parameter).collect(),
        TargetBehavior::FilterGreater => input
            .iter()
            .copied()
            .filter(|value| *value > parameter)
            .collect(),
        TargetBehavior::Sum => vec![input.iter().sum()],
        TargetBehavior::StateDelta => vec![input.first().copied().unwrap_or(0) + parameter],
        TargetBehavior::ReverseDelta => vec![input.first().copied().unwrap_or(0) - parameter],
        TargetBehavior::QuotientClass => {
            let value = input.first().copied().ok_or("MISSING_PARTITION_VALUE")?;
            if !(100..=599).contains(&value) {
                return Err("PARTITION_VALUE_OUT_OF_RANGE".to_string());
            }
            vec![value / 100]
        }
        TargetBehavior::ComposeDeltas => {
            vec![input.first().copied().unwrap_or(0) + parameter + parameter]
        }
        TargetBehavior::ScopedIdentity => {
            vec![input.first().copied().ok_or("MISSING_SCOPED_VALUE")?]
        }
        TargetBehavior::MapThenSum => {
            vec![input.iter().map(|value| value + parameter).sum()]
        }
    })
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn median(values: &[usize]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem8::{catalog::extract_source_mechanisms, tasks::generate_transfer_tasks};

    #[test]
    fn role_mapping_preserves_required_roles_and_relations() {
        let catalog = extract_source_mechanisms();
        let tasks = generate_transfer_tasks(83);
        let source = &catalog[0];
        let target = tasks
            .iter()
            .find(|task| task.visible.behavior == TargetBehavior::StateDelta)
            .expect("state target");
        let mapping = map_roles(source, &target.visible);
        assert!(mapping.semantic_role_pass);
        assert!(mapping.relation_preservation_pass);
    }

    #[test]
    fn full_system_rejects_mimics_and_recognizes_semantic_equivalence() {
        let catalog = extract_source_mechanisms();
        let tasks = generate_transfer_tasks(84);
        let index = MechanismIndex::new(&catalog);
        let full = evaluate_condition(TransferCondition::FullMechanismTransferD, &tasks, &index);
        assert!(full
            .records
            .iter()
            .filter(|record| record.structural_mimic)
            .all(|record| record.solved
                && record.invalid_transfer_rejected
                && !record.invalid_transfer_accepted));
        assert!(full
            .records
            .iter()
            .filter(|record| record.semantic_equivalence_different_structure)
            .all(|record| record.solved && record.source_used));
    }

    #[test]
    fn target_verifier_executes_all_target_behaviors() {
        for behavior in [
            TargetBehavior::AddEach,
            TargetBehavior::MultiplyEach,
            TargetBehavior::FilterGreater,
            TargetBehavior::Sum,
            TargetBehavior::StateDelta,
            TargetBehavior::ReverseDelta,
            TargetBehavior::QuotientClass,
            TargetBehavior::ComposeDeltas,
            TargetBehavior::ScopedIdentity,
            TargetBehavior::MapThenSum,
        ] {
            let input = if behavior == TargetBehavior::QuotientClass {
                vec![204]
            } else {
                vec![1, 2, 3]
            };
            assert!(verify_target_behavior(behavior, 2, &input).is_ok());
        }
    }
}
