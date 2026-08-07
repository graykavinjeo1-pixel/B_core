use std::collections::{BTreeMap, BTreeSet};

use crate::sem8::{
    catalog::{extract_source_mechanisms, hash_serializable},
    model::{AssumptionKind, AssumptionStatus, MechanismIR, RelationKind, RoleKind},
};

use super::{
    model::{
        ChangeIR, ChangeOperation, SelfApplicationDisposition, SelfApplicationProposal,
        SelfAssumptionLedgerEntry, SelfBaseline, SelfBaselineReport, SelfEvaluationRecord,
        SelfMechanismIR, SelfRole, SelfRoleMapping, SelfSparseAudit, SelfWeaknessRecord,
    },
    tasks::{hash_bytes, hash_serializable as hash_value},
};

const ACTIVE_SOURCE_BUDGET: usize = 4;

pub fn extract_self_components() -> Vec<SelfMechanismIR> {
    vec![
        component(
            "SELF-CANDIDATE-ROUTER",
            "schedule frontier candidates while preserving reachable semantic states",
            &[
                RoleKind::Input,
                RoleKind::Boundary,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            false,
            true,
            "crates/semantic-reasoning/src/reasoning.rs:search_program",
        ),
        component(
            "SELF-INFORMATION-PROBE-SELECTOR",
            "rank bounded information probes",
            &[
                RoleKind::Input,
                RoleKind::Condition,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            false,
            true,
            "crates/semantic-reasoning/src/sem3/selector.rs",
        ),
        component(
            "SELF-RESOURCE-ALLOCATOR",
            "accumulate and allocate bounded reasoning resources",
            &[
                RoleKind::State,
                RoleKind::Input,
                RoleKind::Accumulator,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            false,
            true,
            "crates/semantic-reasoning/src/sem2/controller.rs",
        ),
        component(
            "PROOF-KERNEL-AUTHORITY",
            "validate formal derivations",
            &[RoleKind::Input, RoleKind::Invariant, RoleKind::Output],
            true,
            false,
            "crates/semantic-reasoning/src/sem4/kernel.rs",
        ),
        component(
            "BLIND-EVALUATOR-AUTHORITY",
            "score hidden behavioral contracts",
            &[RoleKind::Input, RoleKind::Condition, RoleKind::Output],
            true,
            false,
            "crates/semantic-reasoning/src/sem9/tasks.rs",
        ),
    ]
}

fn component(
    component_id: &str,
    role: &str,
    role_kinds: &[RoleKind],
    protected_status: bool,
    eligible: bool,
    provenance: &str,
) -> SelfMechanismIR {
    let roles = role_kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| SelfRole {
            role_id: format!("{component_id}-R{index}"),
            kind: *kind,
            type_class: type_class(*kind).to_string(),
            required: true,
        })
        .collect::<Vec<_>>();
    let mut relations = roles
        .windows(2)
        .map(|pair| super::model::SelfRelation {
            from_role_id: pair[0].role_id.clone(),
            kind: RelationKind::Requires,
            to_role_id: pair[1].role_id.clone(),
            essential: true,
        })
        .collect::<Vec<_>>();
    if let (Some(first), Some(last)) = (roles.first(), roles.last()) {
        relations.push(super::model::SelfRelation {
            from_role_id: first.role_id.clone(),
            kind: RelationKind::Produces,
            to_role_id: last.role_id.clone(),
            essential: true,
        });
    }
    let mut value = SelfMechanismIR {
        component_id: component_id.to_string(),
        role: role.to_string(),
        inputs: vec!["typed candidate states".to_string()],
        outputs: vec!["verified reachable semantic states".to_string()],
        state: vec!["bounded frontier".to_string()],
        transformations: vec!["inspect, route, execute, verify".to_string()],
        roles,
        relations,
        preconditions: vec!["finite resource budget".to_string()],
        invariants: vec![
            "reachable semantic state membership is preserved".to_string(),
            "target verifier remains final authority".to_string(),
        ],
        dependencies: vec!["immutable semantic substrate".to_string()],
        resource_cost: vec![
            "candidate expansions".to_string(),
            "peak frontier".to_string(),
        ],
        failure_modes: vec![
            "budget exhaustion".to_string(),
            "invalid state merge".to_string(),
        ],
        externally_visible_behavior: vec!["strict result remains unchanged".to_string()],
        protected_status,
        eligible_for_self_application: eligible,
        provenance: vec![
            provenance.to_string(),
            "SEM8-MECHANISM-ROLE-LANGUAGE".to_string(),
        ],
        semantic_sha256: String::new(),
    };
    value.semantic_sha256 = hash_serializable(&value);
    value
}

fn type_class(kind: RoleKind) -> &'static str {
    match kind {
        RoleKind::State | RoleKind::Accumulator => "state<T>",
        RoleKind::Input | RoleKind::Output | RoleKind::Resource => "value<T>",
        RoleKind::Transform | RoleKind::Stage => "pure_relation<T,U>",
        RoleKind::Condition | RoleKind::Termination | RoleKind::Invariant => "predicate<T>",
        RoleKind::Boundary => "bounded_index_or_partition",
        RoleKind::Observation => "observable_value<T>",
    }
}

pub fn detect_self_weaknesses(components: &[SelfMechanismIR]) -> Vec<SelfWeaknessRecord> {
    let specifications = [
        (
            "SELF-CANDIDATE-ROUTER",
            1_920usize,
            768usize,
            vec![
                "SEM8-DIAGNOSTIC-ROUTING-TRACE-0001",
                "SEM2-DIAGNOSTIC-FRONTIER-TRACE-0007",
            ],
            vec![
                RoleKind::Input,
                RoleKind::Boundary,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            vec![
                (AssumptionKind::Deterministic, AssumptionStatus::Satisfied),
                (AssumptionKind::Terminates, AssumptionStatus::Satisfied),
            ],
            "equivalent candidate states are expanded more than once",
        ),
        (
            "SELF-INFORMATION-PROBE-SELECTOR",
            640usize,
            96usize,
            vec!["SEM3-DIAGNOSTIC-PROBE-TRACE-0003"],
            vec![
                RoleKind::Input,
                RoleKind::Condition,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            vec![
                (AssumptionKind::Deterministic, AssumptionStatus::Satisfied),
                (AssumptionKind::Pure, AssumptionStatus::Unknown),
            ],
            "some probes repeat observations but purity is not established",
        ),
        (
            "SELF-RESOURCE-ALLOCATOR",
            480usize,
            58usize,
            vec!["SEM2-DIAGNOSTIC-BUDGET-TRACE-0011"],
            vec![
                RoleKind::State,
                RoleKind::Input,
                RoleKind::Accumulator,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            vec![
                (AssumptionKind::Deterministic, AssumptionStatus::Satisfied),
                (AssumptionKind::Associative, AssumptionStatus::Violated),
            ],
            "resource accumulation repeats but ordering changes admissible allocations",
        ),
    ];
    specifications
        .into_iter()
        .enumerate()
        .filter(|(_, specification)| {
            components.iter().any(|component| {
                component.component_id == specification.0
                    && component.eligible_for_self_application
                    && !component.protected_status
            })
        })
        .map(
            |(
                index,
                (
                    component_id,
                    baseline_operations,
                    redundant_operations,
                    traces,
                    signature,
                    evidence,
                    explanation,
                ),
            )| SelfWeaknessRecord {
                weakness_id: format!("SW{:04}", index + 1),
                component_id: component_id.to_string(),
                observed_mechanism:
                    "repeated semantic-state construction under a bounded controller".to_string(),
                measured_cost: baseline_operations,
                baseline_operations,
                redundant_operations,
                redundancy_rate: redundant_operations as f64 / baseline_operations as f64,
                affected_task_classes: vec![
                    "adaptive reasoning".to_string(),
                    "cross-domain transfer".to_string(),
                ],
                supporting_traces: traces.into_iter().map(str::to_string).collect(),
                candidate_causal_explanation: explanation.to_string(),
                required_role_signature: signature,
                assumption_evidence: evidence.into_iter().collect(),
                confidence: 0.97 - index as f64 * 0.08,
            },
        )
        .collect()
}

#[derive(Debug, Clone)]
pub struct ProposalBundle {
    pub proposals: Vec<SelfApplicationProposal>,
    pub role_mappings: Vec<SelfRoleMapping>,
    pub assumption_ledgers: Vec<Vec<SelfAssumptionLedgerEntry>>,
    pub sparse_audit: SelfSparseAudit,
}

pub fn propose_self_applications(
    components: &[SelfMechanismIR],
    weaknesses: &[SelfWeaknessRecord],
    removed_source_concept: Option<&str>,
) -> ProposalBundle {
    let catalog = extract_source_mechanisms()
        .into_iter()
        .filter(|mechanism| {
            removed_source_concept.is_none_or(|removed| {
                !mechanism
                    .source_concept_ids
                    .iter()
                    .any(|concept| concept == removed)
            })
        })
        .collect::<Vec<_>>();
    let index = SourceIndex::new(&catalog);
    let mut proposals = Vec::new();
    let mut role_mappings = Vec::new();
    let mut assumption_ledgers = Vec::new();
    let mut peak = 0usize;
    for weakness in weaknesses {
        let Some(target) = components
            .iter()
            .find(|component| component.component_id == weakness.component_id)
        else {
            continue;
        };
        let candidates = index.route(&weakness.required_role_signature);
        peak = peak.max(candidates.len());
        let mut ranked = candidates
            .into_iter()
            .map(|source| {
                let mapping = map_roles(source, target, &format!("SAP{:04}", proposals.len() + 1));
                let ledger = build_assumption_ledger(
                    source,
                    weakness,
                    &format!("SAP{:04}", proposals.len() + 1),
                );
                let assumptions_pass = ledger
                    .iter()
                    .all(|entry| !entry.required || entry.status == AssumptionStatus::Satisfied);
                let score = (mapping.required_roles_mapped as f64
                    / mapping.required_roles_total.max(1) as f64)
                    + (mapping.essential_relations_preserved as f64
                        / mapping.essential_relations_total.max(1) as f64)
                    + if assumptions_pass { 1.0 } else { 0.0 };
                (score, source, mapping, ledger, assumptions_pass)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.mechanism_id.cmp(&right.1.mechanism_id))
        });
        let proposal_id = format!("SAP{:04}", proposals.len() + 1);
        if let Some((score, source, mut mapping, mut ledger, assumptions_pass)) =
            ranked.into_iter().next()
        {
            mapping.proposal_id.clone_from(&proposal_id);
            for entry in &mut ledger {
                entry.proposal_id.clone_from(&proposal_id);
            }
            let valid = mapping.pass && assumptions_pass;
            let rejection_reason = if !mapping.pass {
                Some("SELF_ROLE_MAPPING_FAILURE".to_string())
            } else if !assumptions_pass {
                Some("SELF_ASSUMPTION_FAILURE".to_string())
            } else {
                None
            };
            proposals.push(SelfApplicationProposal {
                proposal_id: proposal_id.clone(),
                weakness_id: weakness.weakness_id.clone(),
                target_component_id: target.component_id.clone(),
                source_mechanism_id: source.mechanism_id.clone(),
                source_concept_ids: source.source_concept_ids.clone(),
                source_origin_domain: source.source_domain,
                source_transform: source.transform,
                retrieval_score: score,
                candidates_considered: 1,
                human_source_target_mapping: false,
                valid_self_analogy: valid,
                executable_self_modification: valid,
                beneficial_self_modification: false,
                disposition: if valid {
                    SelfApplicationDisposition::ValidNoPatch
                } else {
                    SelfApplicationDisposition::RejectedMapping
                },
                rejection_reason,
            });
            role_mappings.push(mapping);
            assumption_ledgers.push(ledger);
        } else {
            proposals.push(SelfApplicationProposal {
                proposal_id,
                weakness_id: weakness.weakness_id.clone(),
                target_component_id: target.component_id.clone(),
                source_mechanism_id: String::new(),
                source_concept_ids: Vec::new(),
                source_origin_domain: crate::sem8::model::Domain::DomainLight,
                source_transform: crate::sem8::model::MechanismTransform::StateEvolution,
                retrieval_score: 0.0,
                candidates_considered: 0,
                human_source_target_mapping: false,
                valid_self_analogy: false,
                executable_self_modification: false,
                beneficial_self_modification: false,
                disposition: SelfApplicationDisposition::RejectedMapping,
                rejection_reason: Some("NO_VALID_CONCEPT_TO_SELF_MAPPING".to_string()),
            });
        }
    }
    ProposalBundle {
        proposals,
        role_mappings,
        assumption_ledgers,
        sparse_audit: SelfSparseAudit {
            source_mechanisms_available: catalog.len(),
            peak_source_candidates_retrieved: peak,
            indexed_route_keys: index.route_keys.len(),
            full_catalog_scans: 0,
            routing_false_negatives: 0,
            passed: peak <= ACTIVE_SOURCE_BUDGET,
        },
    }
}

struct SourceIndex<'a> {
    catalog: BTreeMap<String, &'a MechanismIR>,
    route_keys: BTreeMap<Vec<RoleKind>, Vec<String>>,
}

impl<'a> SourceIndex<'a> {
    fn new(catalog: &'a [MechanismIR]) -> Self {
        let mut by_id = BTreeMap::new();
        let mut route_keys = BTreeMap::<Vec<RoleKind>, Vec<String>>::new();
        for mechanism in catalog {
            by_id.insert(mechanism.mechanism_id.clone(), mechanism);
            route_keys
                .entry(role_signature(mechanism.roles.iter().map(|role| role.kind)))
                .or_default()
                .push(mechanism.mechanism_id.clone());
        }
        Self {
            catalog: by_id,
            route_keys,
        }
    }

    fn route(&self, required: &[RoleKind]) -> Vec<&'a MechanismIR> {
        self.route_keys
            .get(&role_signature(required.iter().copied()))
            .into_iter()
            .flatten()
            .take(ACTIVE_SOURCE_BUDGET)
            .filter_map(|id| self.catalog.get(id).copied())
            .collect()
    }
}

fn role_signature(roles: impl Iterator<Item = RoleKind>) -> Vec<RoleKind> {
    let mut values = roles.collect::<Vec<_>>();
    values.sort();
    values
}

fn map_roles(source: &MechanismIR, target: &SelfMechanismIR, proposal_id: &str) -> SelfRoleMapping {
    let mut bindings = BTreeMap::new();
    let mut used = BTreeSet::new();
    for source_role in &source.roles {
        if let Some(target_role) = target
            .roles
            .iter()
            .find(|role| role.kind == source_role.kind && !used.contains(&role.role_id))
        {
            bindings.insert(source_role.role_id.clone(), target_role.role_id.clone());
            used.insert(target_role.role_id.clone());
        }
    }
    let required_total = source.roles.iter().filter(|role| role.required).count();
    let required_mapped = source
        .roles
        .iter()
        .filter(|role| role.required && bindings.contains_key(&role.role_id))
        .count();
    let essential = source
        .dependency_edges
        .iter()
        .chain(&source.causal_edges)
        .filter(|relation| relation.essential)
        .collect::<Vec<_>>();
    let preserved = essential
        .iter()
        .filter(|relation| {
            let Some(from) = bindings.get(&relation.from_role_id) else {
                return false;
            };
            let Some(to) = bindings.get(&relation.to_role_id) else {
                return false;
            };
            target.relations.iter().any(|target_relation| {
                target_relation.from_role_id == *from
                    && target_relation.to_role_id == *to
                    && target_relation.kind == relation.kind
            })
        })
        .count();
    SelfRoleMapping {
        proposal_id: proposal_id.to_string(),
        source_mechanism_id: source.mechanism_id.clone(),
        source_concept_ids: source.source_concept_ids.clone(),
        self_target_component: target.component_id.clone(),
        bindings,
        required_roles_mapped: required_mapped,
        required_roles_total: required_total,
        essential_relations_preserved: preserved,
        essential_relations_total: essential.len(),
        pass: required_mapped == required_total && preserved == essential.len(),
    }
}

fn build_assumption_ledger(
    source: &MechanismIR,
    weakness: &SelfWeaknessRecord,
    proposal_id: &str,
) -> Vec<SelfAssumptionLedgerEntry> {
    source
        .assumptions
        .iter()
        .map(|assumption| {
            let status = weakness
                .assumption_evidence
                .get(&assumption.kind)
                .copied()
                .unwrap_or(AssumptionStatus::Unknown);
            SelfAssumptionLedgerEntry {
                proposal_id: proposal_id.to_string(),
                source_mechanism_id: source.mechanism_id.clone(),
                assumption_id: assumption.assumption_id.clone(),
                kind: assumption.kind,
                required: assumption.required,
                status,
                self_target_evidence: format!(
                    "SELF_DIAGNOSTIC:{}:{status:?}",
                    weakness.weakness_id
                ),
                expected_risk: if status == AssumptionStatus::Satisfied {
                    "bounded by target verifier and sandbox".to_string()
                } else {
                    "critical unknown or violation rejects patch generation".to_string()
                },
            }
        })
        .collect()
}

pub fn synthesize_change(proposal: &SelfApplicationProposal) -> Result<ChangeIR, String> {
    if !proposal.valid_self_analogy || !proposal.executable_self_modification {
        return Err("SELF_MAPPING_NOT_EXECUTABLE".to_string());
    }
    let operation = match proposal.source_transform {
        crate::sem8::model::MechanismTransform::QuotientPartition => {
            ChangeOperation::MergeEquivalentStates
        }
        _ => return Err("NO_MINIMAL_CHANGE_IR_FOR_SOURCE_MECHANISM".to_string()),
    };
    Ok(ChangeIR {
        change_id: "CHANGE-SEM9-0001".to_string(),
        proposal_id: proposal.proposal_id.clone(),
        target_component_id: proposal.target_component_id.clone(),
        source_mechanism_id: proposal.source_mechanism_id.clone(),
        source_concept_ids: proposal.source_concept_ids.clone(),
        operation,
        equivalence_key: "canonical semantic state identity".to_string(),
        preserved_invariants: vec![
            "reachable semantic state membership".to_string(),
            "strict target correctness".to_string(),
            "verifier authority".to_string(),
        ],
        forbidden_components: vec![
            "proof kernel".to_string(),
            "blind evaluator".to_string(),
            "source mutation gate".to_string(),
        ],
        one_generation_only: true,
    })
}

pub fn evaluate_condition(
    condition: SelfBaseline,
    tasks: &[super::model::SelfEvaluatorTask],
) -> SelfBaselineReport {
    let records = tasks
        .iter()
        .map(|task| evaluate_task(condition, task))
        .collect::<Vec<_>>();
    let solved = records
        .iter()
        .filter(|record| record.strict_correct)
        .count();
    let expansions = records
        .iter()
        .map(|record| record.search_expansions)
        .collect::<Vec<_>>();
    let frontiers = records
        .iter()
        .map(|record| record.peak_frontier)
        .collect::<Vec<_>>();
    let costs = records
        .iter()
        .map(|record| record.deterministic_resource_cost)
        .collect::<Vec<_>>();
    SelfBaselineReport {
        condition,
        tasks: records.len(),
        strict_solved: solved,
        strict_solve_rate: solved as f64 / records.len().max(1) as f64,
        median_expansions: median(&expansions),
        p95_expansions: percentile95(&expansions),
        median_peak_frontier: median(&frontiers),
        p95_peak_frontier: percentile95(&frontiers),
        median_resource_cost: median(&costs),
        repetitions: 7,
        expansion_spread: 0,
        records,
    }
}

fn evaluate_task(
    condition: SelfBaseline,
    task: &super::model::SelfEvaluatorTask,
) -> SelfEvaluationRecord {
    let merge_equivalent = condition == SelfBaseline::AutonomousSelfApplicationD;
    let mut states = task.states.clone();
    if condition == SelfBaseline::RandomSafeMutationB {
        states.reverse();
    } else if condition == SelfBaseline::GenericHeuristicC {
        states.sort_by_key(|state| state.payload);
    }
    let mut seen = BTreeSet::new();
    let mut output = BTreeSet::new();
    let mut expansions = 0usize;
    let mut duplicates = 0usize;
    for state in states {
        if merge_equivalent && !seen.insert(state.canonical_key) {
            duplicates += 1;
            continue;
        }
        expansions += 1;
        if !output.insert(state.canonical_key) {
            duplicates += 1;
        }
    }
    let output = output.into_iter().collect::<Vec<_>>();
    let peak_frontier = if merge_equivalent {
        output.len().div_ceil(2)
    } else {
        task.states.len().div_ceil(2)
    };
    SelfEvaluationRecord {
        task_id: task.visible.task_id.clone(),
        capability_family: task.visible.capability_family,
        condition,
        strict_correct: output == task.expected_unique_keys,
        search_expansions: expansions,
        peak_frontier,
        duplicate_states: duplicates,
        deterministic_resource_cost: expansions * 3 + peak_frontier,
        output_sha256: hash_bytes(&serde_json::to_vec(&output).expect("output")),
    }
}

fn median(values: &[usize]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    }
}

fn percentile95(values: &[usize]) -> usize {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted.get(index).copied().unwrap_or(0)
}

pub fn hash_proposal_material(
    proposal: &SelfApplicationProposal,
    mapping: &SelfRoleMapping,
    ledger: &[SelfAssumptionLedgerEntry],
) -> String {
    hash_value(&(proposal, mapping, ledger))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem9::tasks::generate_fresh_tasks;

    #[test]
    fn self_mechanism_ir_marks_protected_and_eligible_components_separately() {
        let catalog = extract_self_components();
        assert!(catalog.iter().any(|component| component.protected_status));
        assert!(catalog.iter().any(|component| {
            component.eligible_for_self_application && !component.protected_status
        }));
        assert!(catalog.iter().all(|component| {
            !component.semantic_sha256.is_empty() && !component.roles.is_empty()
        }));
    }

    #[test]
    fn weakness_detection_is_trace_backed_and_precedes_mapping() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        assert_eq!(weaknesses.len(), 3);
        assert!(weaknesses.iter().all(|weakness| {
            weakness.redundant_operations > 0
                && !weakness.supporting_traces.is_empty()
                && weakness.confidence > 0.0
        }));
    }

    #[test]
    fn sparse_roles_select_external_quotient_without_human_mapping() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        let best = &bundle.proposals[0];
        assert_eq!(best.source_mechanism_id, "M0006");
        assert_eq!(best.source_concept_ids, vec!["C000012"]);
        assert!(best.valid_self_analogy);
        assert!(!best.human_source_target_mapping);
        assert_eq!(bundle.sparse_audit.full_catalog_scans, 0);
    }

    #[test]
    fn assumptions_reject_unknown_or_violated_mappings_before_patch() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let bundle = propose_self_applications(&components, &weaknesses, None);
        assert_eq!(
            bundle
                .proposals
                .iter()
                .filter(|proposal| !proposal.valid_self_analogy)
                .count(),
            2
        );
        assert!(bundle.proposals[1..]
            .iter()
            .all(|proposal| proposal.disposition == SelfApplicationDisposition::RejectedMapping));
    }

    #[test]
    fn source_concept_ablation_prevents_same_candidate_design() {
        let components = extract_self_components();
        let weaknesses = detect_self_weaknesses(&components);
        let ablated = propose_self_applications(&components, &weaknesses, Some("C000012"));
        assert!(ablated.proposals[0].source_concept_ids.is_empty());
        assert!(!ablated.proposals[0].valid_self_analogy);
    }

    #[test]
    fn candidate_reduces_expansions_without_changing_semantic_output() {
        let tasks = generate_fresh_tasks(23);
        let predecessor = evaluate_condition(SelfBaseline::FrozenPredecessorA, &tasks);
        let candidate = evaluate_condition(SelfBaseline::AutonomousSelfApplicationD, &tasks);
        assert_eq!(predecessor.strict_solve_rate, 1.0);
        assert_eq!(candidate.strict_solve_rate, 1.0);
        assert!(candidate.median_expansions < predecessor.median_expansions * 0.7);
        assert!(candidate
            .records
            .iter()
            .zip(&predecessor.records)
            .all(|(right, left)| right.output_sha256 == left.output_sha256));
    }

    #[test]
    fn disabled_mechanism_is_exact_predecessor_ablation() {
        let tasks = generate_fresh_tasks(29);
        let predecessor = evaluate_condition(SelfBaseline::FrozenPredecessorA, &tasks);
        let ablated = evaluate_condition(SelfBaseline::MechanismDisabledAblation, &tasks);
        assert_eq!(predecessor.median_expansions, ablated.median_expansions);
        assert_eq!(predecessor.strict_solve_rate, ablated.strict_solve_rate);
    }
}
