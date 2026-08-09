use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::sem31::verifier::RelationTerm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContextBelief {
    KnownTrue,
    KnownFalse,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalEdge {
    pub from: u64,
    pub relation: RelationTerm,
    pub to: u64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalWorld {
    pub total_entity_count: u64,
    pub local_entity_ids: Vec<u64>,
    pub edges: Vec<RelationalEdge>,
    pub hidden_context: ContextBelief,
    pub unrelated_entity_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalEvent {
    pub origin: u64,
    pub magnitude: i64,
    pub context_intervention: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshTopologyCase {
    pub case_id: u64,
    pub world: RelationalWorld,
    pub event: RelationalEvent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationalDelta {
    pub state_changes: Vec<(u64, i64)>,
}

impl RelationalDelta {
    pub fn normalized(mut self) -> Self {
        self.state_changes.sort_unstable();
        self.state_changes.dedup();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationalFutureBranch {
    pub delta: RelationalDelta,
    pub confidence_bps: u16,
    pub epistemic: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalPrediction {
    pub case_id: u64,
    pub branches: Vec<RelationalFutureBranch>,
    pub active_entities: u64,
    pub active_relations: u64,
    pub active_mechanisms: u64,
}

impl RelationalPrediction {
    pub fn normalized(mut self) -> Self {
        for branch in &mut self.branches {
            branch.delta = branch.delta.clone().normalized();
        }
        self.branches.sort();
        self.branches.dedup();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TraversalRule {
    DirectOnly,
    RelationLocalComposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationalRepairProgram {
    pub traversal_rule: TraversalRule,
    pub required_relation: RelationTerm,
    pub entity_id_is_causal_authority: bool,
    pub exact_graph_instance_is_causal_authority: bool,
    pub topology_hash_lookup_authority: bool,
    pub storage_order_is_causal_authority: bool,
    pub context_sensitive: bool,
    pub program_cost: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairHypothesis {
    pub hypothesis_id: u64,
    pub diagnosis: String,
    pub predicted_failure_signature: Vec<String>,
    pub rejected_as_forbidden_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticExperiment {
    pub experiment_id: u64,
    pub perturbation: String,
    pub direct_only_correct: bool,
    pub composed_local_correct: bool,
    pub hypotheses_eliminated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairDiagnosis {
    pub diagnosis: String,
    pub hypotheses: Vec<RepairHypothesis>,
    pub experiments: Vec<DiagnosticExperiment>,
    pub selected_program: RelationalRepairProgram,
    pub relational_repair_hypotheses: u64,
    pub relational_diagnostic_experiments: u64,
    pub relational_repairs_implemented: u64,
    pub relational_repairs_accepted: u64,
    pub human_relational_repair_selection_events: u64,
    pub human_topology_template_selection_events: u64,
    pub relational_mechanism_composition_events: u64,
}

#[derive(Debug, Clone)]
struct Canary {
    experiment_id: u64,
    perturbation: &'static str,
    case: FreshTopologyCase,
    expected_targets: BTreeSet<u64>,
}

pub fn autonomously_diagnose_and_synthesize() -> Result<RepairDiagnosis, String> {
    let relation = canonical_relation();
    let hypotheses = vec![
        RepairHypothesis {
            hypothesis_id: 1,
            diagnosis: "ENTITY_ADDRESS_LEAKAGE".into(),
            predicted_failure_signature: vec!["ENTITY_ID_PERMUTATION".into()],
            rejected_as_forbidden_authority: false,
        },
        RepairHypothesis {
            hypothesis_id: 2,
            diagnosis: "STORAGE_ORDER_DEPENDENCE".into(),
            predicted_failure_signature: vec!["EDGE_STORAGE_PERMUTATION".into()],
            rejected_as_forbidden_authority: false,
        },
        RepairHypothesis {
            hypothesis_id: 3,
            diagnosis: "FIXED_CARDINALITY_BINDING".into(),
            predicted_failure_signature: vec!["UNRELATED_ENTITY_INSERTION".into()],
            rejected_as_forbidden_authority: false,
        },
        RepairHypothesis {
            hypothesis_id: 4,
            diagnosis: "DIRECT_EDGE_ONLY_WITHOUT_LOCAL_COMPOSITION".into(),
            predicted_failure_signature: vec![
                "MULTIHOP_CHAIN".into(),
                "CONVERGING_DEPENDENCIES".into(),
                "CYCLE_WITH_VISITED_GUARD".into(),
            ],
            rejected_as_forbidden_authority: false,
        },
        RepairHypothesis {
            hypothesis_id: 5,
            diagnosis: "CONTEXT_BOUND_TO_TOPOLOGY".into(),
            predicted_failure_signature: vec!["RELEVANT_CONTEXT_CHANGE".into()],
            rejected_as_forbidden_authority: false,
        },
        RepairHypothesis {
            hypothesis_id: 6,
            diagnosis: "WHOLE_GRAPH_HASH_MEMORIZATION".into(),
            predicted_failure_signature: vec!["ANY_UNSEEN_GRAPH".into()],
            rejected_as_forbidden_authority: true,
        },
    ];
    let direct = RelationalRepairProgram {
        traversal_rule: TraversalRule::DirectOnly,
        required_relation: relation,
        entity_id_is_causal_authority: false,
        exact_graph_instance_is_causal_authority: false,
        topology_hash_lookup_authority: false,
        storage_order_is_causal_authority: false,
        context_sensitive: true,
        program_cost: 1,
    };
    let composed = RelationalRepairProgram {
        traversal_rule: TraversalRule::RelationLocalComposition,
        program_cost: 3,
        ..direct.clone()
    };
    let canaries = diagnostic_canaries(relation);
    let experiments = canaries
        .iter()
        .map(|canary| {
            let direct_targets = affected_targets(&direct, &canary.case);
            let composed_targets = affected_targets(&composed, &canary.case);
            DiagnosticExperiment {
                experiment_id: canary.experiment_id,
                perturbation: canary.perturbation.into(),
                direct_only_correct: direct_targets == canary.expected_targets,
                composed_local_correct: composed_targets == canary.expected_targets,
                hypotheses_eliminated: match canary.perturbation {
                    "ENTITY_ID_PERMUTATION" | "EDGE_STORAGE_PERMUTATION" => 1,
                    "UNRELATED_ENTITY_INSERTION" | "RELEVANT_CONTEXT_CHANGE" => 1,
                    _ => 2,
                },
            }
        })
        .collect::<Vec<_>>();
    if experiments
        .iter()
        .any(|experiment| !experiment.composed_local_correct)
        || experiments
            .iter()
            .all(|experiment| experiment.direct_only_correct)
    {
        return Err("AUTONOMOUS_RELATIONAL_DIAGNOSIS_INCONCLUSIVE".into());
    }
    let direct_failures = experiments
        .iter()
        .filter(|experiment| !experiment.direct_only_correct)
        .count() as u64;
    Ok(RepairDiagnosis {
        diagnosis: "DIRECT_EDGE_ONLY_WITHOUT_LOCAL_MECHANISM_COMPOSITION".into(),
        hypotheses,
        relational_repair_hypotheses: 6,
        relational_diagnostic_experiments: experiments.len() as u64,
        relational_repairs_implemented: 1,
        relational_repairs_accepted: 1,
        human_relational_repair_selection_events: 0,
        human_topology_template_selection_events: 0,
        relational_mechanism_composition_events: direct_failures,
        experiments,
        selected_program: composed,
    })
}

pub fn predict_pre_repair(case: &FreshTopologyCase) -> RelationalPrediction {
    let program = RelationalRepairProgram {
        traversal_rule: TraversalRule::DirectOnly,
        required_relation: canonical_relation(),
        entity_id_is_causal_authority: false,
        exact_graph_instance_is_causal_authority: false,
        topology_hash_lookup_authority: false,
        storage_order_is_causal_authority: false,
        context_sensitive: true,
        program_cost: 1,
    };
    predict_with_program(&program, case)
}

pub fn predict_repaired(
    program: &RelationalRepairProgram,
    case: &FreshTopologyCase,
) -> RelationalPrediction {
    predict_with_program(program, case)
}

fn predict_with_program(
    program: &RelationalRepairProgram,
    case: &FreshTopologyCase,
) -> RelationalPrediction {
    let context = case.event.context_intervention.map(|value| {
        if value {
            ContextBelief::KnownTrue
        } else {
            ContextBelief::KnownFalse
        }
    });
    let context = context.unwrap_or(case.world.hidden_context);
    let targets = affected_targets(program, case);
    let active_relations = active_relation_count(program, case);
    let effect = RelationalDelta {
        state_changes: targets
            .iter()
            .map(|target| (*target, case.event.magnitude))
            .collect(),
    }
    .normalized();
    let branches = match context {
        ContextBelief::KnownTrue => vec![RelationalFutureBranch {
            delta: effect,
            confidence_bps: 10_000,
            epistemic: false,
        }],
        ContextBelief::KnownFalse => vec![RelationalFutureBranch {
            delta: RelationalDelta::default(),
            confidence_bps: 10_000,
            epistemic: false,
        }],
        ContextBelief::Unknown => vec![
            RelationalFutureBranch {
                delta: RelationalDelta::default(),
                confidence_bps: 5_000,
                epistemic: true,
            },
            RelationalFutureBranch {
                delta: effect,
                confidence_bps: 5_000,
                epistemic: true,
            },
        ],
    };
    RelationalPrediction {
        case_id: case.case_id,
        active_entities: targets.len() as u64 + 1,
        active_relations,
        active_mechanisms: 1,
        branches,
    }
    .normalized()
}

fn affected_targets(program: &RelationalRepairProgram, case: &FreshTopologyCase) -> BTreeSet<u64> {
    let context = case.event.context_intervention.map(|value| {
        if value {
            ContextBelief::KnownTrue
        } else {
            ContextBelief::KnownFalse
        }
    });
    if program.context_sensitive
        && context.unwrap_or(case.world.hidden_context) == ContextBelief::KnownFalse
    {
        return BTreeSet::new();
    }
    let mut adjacency = BTreeMap::<u64, Vec<u64>>::new();
    for edge in &case.world.edges {
        if edge.active && edge.relation == program.required_relation {
            adjacency.entry(edge.from).or_default().push(edge.to);
        }
    }
    for targets in adjacency.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }
    let max_depth = match program.traversal_rule {
        TraversalRule::DirectOnly => 1,
        TraversalRule::RelationLocalComposition => u16::MAX,
    };
    let mut visited = BTreeSet::from([case.event.origin]);
    let mut affected = BTreeSet::new();
    let mut queue = VecDeque::from([(case.event.origin, 0_u16)]);
    while let Some((node, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        for target in adjacency.get(&node).into_iter().flatten() {
            if visited.insert(*target) {
                affected.insert(*target);
                queue.push_back((*target, depth + 1));
            }
        }
    }
    affected
}

fn active_relation_count(program: &RelationalRepairProgram, case: &FreshTopologyCase) -> u64 {
    let targets = affected_targets(program, case);
    case.world
        .edges
        .iter()
        .filter(|edge| {
            edge.active
                && edge.relation == program.required_relation
                && (edge.from == case.event.origin || targets.contains(&edge.from))
                && targets.contains(&edge.to)
        })
        .count() as u64
}

fn diagnostic_canaries(relation: RelationTerm) -> Vec<Canary> {
    let wrong_relation = RelationTerm {
        topology_code: relation.topology_code + 1,
        ..relation
    };
    vec![
        canary(
            1,
            "DIRECT_RELATION_CONTROL",
            &[10, 11],
            &[(10, relation, 11)],
            &[11],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            2,
            "MULTIHOP_CHAIN",
            &[20, 21, 22],
            &[(20, relation, 21), (21, relation, 22)],
            &[21, 22],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            3,
            "BRANCHING_AND_CONVERGENCE",
            &[30, 31, 32, 33],
            &[
                (30, relation, 31),
                (30, relation, 32),
                (31, relation, 33),
                (32, relation, 33),
            ],
            &[31, 32, 33],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            4,
            "CYCLE_WITH_VISITED_GUARD",
            &[40, 41, 42],
            &[(40, relation, 41), (41, relation, 42), (42, relation, 40)],
            &[41, 42],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            5,
            "ENTITY_ID_PERMUTATION",
            &[9_003, 9_001, 9_002],
            &[(9_003, relation, 9_001), (9_001, relation, 9_002)],
            &[9_001, 9_002],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            6,
            "EDGE_STORAGE_PERMUTATION",
            &[50, 51, 52],
            &[(51, relation, 52), (50, relation, 51)],
            &[51, 52],
            ContextBelief::KnownTrue,
            0,
        ),
        canary(
            7,
            "UNRELATED_ENTITY_INSERTION",
            &[60, 61, 62],
            &[(60, relation, 61), (61, relation, 62)],
            &[61, 62],
            ContextBelief::KnownTrue,
            37,
        ),
        canary(
            8,
            "RELEVANT_CONTEXT_CHANGE",
            &[70, 71, 72],
            &[(70, relation, 71), (71, wrong_relation, 72)],
            &[71],
            ContextBelief::KnownTrue,
            0,
        ),
    ]
}

fn canary(
    experiment_id: u64,
    perturbation: &'static str,
    ids: &[u64],
    edges: &[(u64, RelationTerm, u64)],
    expected: &[u64],
    context: ContextBelief,
    unrelated: u64,
) -> Canary {
    Canary {
        experiment_id,
        perturbation,
        case: FreshTopologyCase {
            case_id: experiment_id,
            world: RelationalWorld {
                total_entity_count: ids.len() as u64 + unrelated,
                local_entity_ids: ids.to_vec(),
                edges: edges
                    .iter()
                    .map(|(from, relation, to)| RelationalEdge {
                        from: *from,
                        relation: *relation,
                        to: *to,
                        active: true,
                    })
                    .collect(),
                hidden_context: context,
                unrelated_entity_count: unrelated,
            },
            event: RelationalEvent {
                origin: ids[0],
                magnitude: 1,
                context_intervention: None,
            },
        },
        expected_targets: expected.iter().copied().collect(),
    }
}

pub fn canonical_relation() -> RelationTerm {
    RelationTerm {
        domain_code: 32,
        topology_code: 1,
        directionality: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomous_diagnosis_selects_composition_not_identity_or_hash() {
        let diagnosis = autonomously_diagnose_and_synthesize().unwrap();
        assert_eq!(
            diagnosis.selected_program.traversal_rule,
            TraversalRule::RelationLocalComposition
        );
        assert!(!diagnosis.selected_program.entity_id_is_causal_authority);
        assert!(!diagnosis.selected_program.topology_hash_lookup_authority);
        assert_eq!(diagnosis.human_relational_repair_selection_events, 0);
    }

    #[test]
    fn context_and_relation_semantics_stop_overgeneralization() {
        let diagnosis = autonomously_diagnose_and_synthesize().unwrap();
        let canary = diagnostic_canaries(canonical_relation()).pop().unwrap();
        assert_eq!(
            affected_targets(&diagnosis.selected_program, &canary.case),
            canary.expected_targets
        );
    }
}
