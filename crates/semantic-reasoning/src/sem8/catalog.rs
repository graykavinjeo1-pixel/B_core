use serde::Serialize;
use sha2::{Digest, Sha256};

use super::model::{
    AssumptionKind, Domain, MechanismAssumption, MechanismIR, MechanismRelation, MechanismRole,
    MechanismTransform, RelationKind, RoleKind, SourceManifest, SourceManifestEntry, SourceSplit,
};

pub fn extract_source_mechanisms() -> Vec<MechanismIR> {
    vec![
        mechanism(
            "M0001",
            &["C000006", "C000007"],
            Domain::Mathematics,
            3,
            MechanismTransform::StateEvolution,
            &[
                RoleKind::State,
                RoleKind::Input,
                RoleKind::Transform,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            &[
                AssumptionKind::Deterministic,
                AssumptionKind::Terminates,
                AssumptionKind::InvariantGlobal,
            ],
            "reports/sem4/derivation_records.json",
        ),
        mechanism(
            "M0002",
            &["C000008"],
            Domain::Programming,
            3,
            MechanismTransform::ElementwiseTransform,
            &[
                RoleKind::Input,
                RoleKind::Transform,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            &[
                AssumptionKind::Deterministic,
                AssumptionKind::OrderPreserving,
                AssumptionKind::Pure,
            ],
            "reports/sem5/program_ir_records.json",
        ),
        mechanism(
            "M0003",
            &["C000008"],
            Domain::DataTransform,
            3,
            MechanismTransform::GuardedTraversal,
            &[
                RoleKind::Input,
                RoleKind::Condition,
                RoleKind::Transform,
                RoleKind::Boundary,
                RoleKind::Output,
            ],
            &[
                AssumptionKind::Deterministic,
                AssumptionKind::Terminates,
                AssumptionKind::InvariantGlobal,
            ],
            "reports/sem5/definition_only_results.json",
        ),
        mechanism(
            "M0004",
            &["C000009"],
            Domain::Programming,
            3,
            MechanismTransform::StatefulReduction,
            &[
                RoleKind::State,
                RoleKind::Input,
                RoleKind::Accumulator,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            &[AssumptionKind::Deterministic, AssumptionKind::Associative],
            "reports/sem5/condition_d_results.json",
        ),
        mechanism(
            "M0005",
            &["C000010"],
            Domain::Programming,
            4,
            MechanismTransform::StageComposition,
            &[
                RoleKind::Input,
                RoleKind::Stage,
                RoleKind::Transform,
                RoleKind::Stage,
                RoleKind::Output,
            ],
            &[AssumptionKind::Deterministic, AssumptionKind::Pure],
            "reports/sem5/transfer_results.json",
        ),
        mechanism(
            "M0006",
            &["C000012"],
            Domain::ExternalDefinition,
            5,
            MechanismTransform::QuotientPartition,
            &[
                RoleKind::Input,
                RoleKind::Boundary,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            &[AssumptionKind::Deterministic, AssumptionKind::Terminates],
            "reports/sem6/semantic_compilation_results.json",
        ),
        mechanism(
            "M0007",
            &["C000011"],
            Domain::ExternalDefinition,
            5,
            MechanismTransform::ScopedRelation,
            &[
                RoleKind::Input,
                RoleKind::Condition,
                RoleKind::Transform,
                RoleKind::Output,
            ],
            &[AssumptionKind::Deterministic, AssumptionKind::Pure],
            "reports/sem6/external_concept_promotions.json",
        ),
        mechanism(
            "M0008",
            &["C000006", "C000010"],
            Domain::Mathematics,
            5,
            MechanismTransform::ReversibleStateTransform,
            &[
                RoleKind::State,
                RoleKind::Input,
                RoleKind::Transform,
                RoleKind::Invariant,
                RoleKind::Output,
            ],
            &[
                AssumptionKind::Deterministic,
                AssumptionKind::Reversible,
                AssumptionKind::Lossless,
            ],
            "reports/sem4/definition_only_results.json",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn mechanism(
    mechanism_id: &str,
    concepts: &[&str],
    domain: Domain,
    generation: usize,
    transform: MechanismTransform,
    role_kinds: &[RoleKind],
    assumption_kinds: &[AssumptionKind],
    provenance_path: &str,
) -> MechanismIR {
    let roles = role_kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| MechanismRole {
            role_id: format!("{mechanism_id}-R{index}"),
            kind: *kind,
            type_class: type_class(*kind).to_string(),
            required: true,
        })
        .collect::<Vec<_>>();
    let mut dependency_edges = Vec::new();
    for pair in roles.windows(2) {
        dependency_edges.push(MechanismRelation {
            from_role_id: pair[0].role_id.clone(),
            kind: RelationKind::Requires,
            to_role_id: pair[1].role_id.clone(),
            essential: true,
        });
    }
    let causal_edges = roles
        .first()
        .zip(roles.last())
        .map(|(input, output)| {
            vec![MechanismRelation {
                from_role_id: input.role_id.clone(),
                kind: RelationKind::Produces,
                to_role_id: output.role_id.clone(),
                essential: true,
            }]
        })
        .unwrap_or_default();
    let assumptions = assumption_kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| MechanismAssumption {
            assumption_id: format!("{mechanism_id}-A{index}"),
            kind: *kind,
            required: true,
            evidence_origin: provenance_path.to_string(),
        })
        .collect::<Vec<_>>();
    let mut mechanism = MechanismIR {
        mechanism_id: mechanism_id.to_string(),
        source_concept_ids: concepts.iter().map(|value| (*value).to_string()).collect(),
        source_domain: domain,
        generation,
        roles,
        states: vec![
            "typed_state_before".to_string(),
            "typed_state_after".to_string(),
        ],
        inputs: vec!["typed_input".to_string()],
        outputs: vec!["typed_output".to_string()],
        preconditions: vec!["target types satisfy the source role contract".to_string()],
        invariants: vec!["declared invariant is preserved across the transformation".to_string()],
        transform,
        transformations: vec![format!("{transform:?}")],
        dependency_edges,
        causal_edges,
        branch_conditions: vec!["declared target condition when present".to_string()],
        termination_conditions: vec!["finite target input or explicit terminal state".to_string()],
        preserved_properties: vec!["type".to_string(), "declared invariant".to_string()],
        consumed_properties: vec!["input state".to_string()],
        produced_properties: vec!["verified target state".to_string()],
        failure_conditions: vec![
            "required assumption violated or target verifier rejects".to_string()
        ],
        assumptions,
        executable: true,
        provenance: vec![
            provenance_path.to_string(),
            format!("SEALED_CONCEPTS:{}", concepts.join(",")),
        ],
        semantic_sha256: String::new(),
    };
    mechanism.semantic_sha256 = hash_serializable(&mechanism);
    mechanism
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

pub fn build_source_manifest(
    run_id: &str,
    split: SourceSplit,
    mechanisms: &[MechanismIR],
) -> SourceManifest {
    let entries = mechanisms
        .iter()
        .enumerate()
        .filter(|(index, _)| match split {
            SourceSplit::Development => *index < 4,
            SourceSplit::Blind => *index >= 4,
        })
        .map(|(_, mechanism)| SourceManifestEntry {
            mechanism_id: mechanism.mechanism_id.clone(),
            semantic_sha256: mechanism.semantic_sha256.clone(),
            source_domain: mechanism.source_domain,
            split,
            target_pair_metadata_included: false,
            human_analogy_label_included: false,
        })
        .collect::<Vec<_>>();
    #[derive(Serialize)]
    struct Commitment<'a> {
        run_id: &'a str,
        split: SourceSplit,
        entries: &'a [SourceManifestEntry],
        frozen_before_evaluation: bool,
    }
    let commitment = Commitment {
        run_id,
        split,
        entries: &entries,
        frozen_before_evaluation: true,
    };
    let manifest_sha256 = hash_serializable(&commitment);
    SourceManifest {
        run_id: run_id.to_string(),
        split,
        entries,
        frozen_before_evaluation: true,
        manifest_sha256,
    }
}

pub fn hash_serializable(value: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serialize"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_is_domain_light_executable_and_provenanced() {
        let catalog = extract_source_mechanisms();
        assert_eq!(catalog.len(), 8);
        assert!(catalog.iter().all(|mechanism| {
            mechanism.executable
                && !mechanism.roles.is_empty()
                && !mechanism.dependency_edges.is_empty()
                && !mechanism.provenance.is_empty()
                && mechanism.semantic_sha256.len() == 64
        }));
        assert!(catalog
            .iter()
            .flat_map(|mechanism| &mechanism.roles)
            .all(|role| !role.role_id.contains("loop") && !role.role_id.contains("induction")));
    }

    #[test]
    fn source_holdouts_are_disjoint_and_hide_pairs() {
        let catalog = extract_source_mechanisms();
        let dev = build_source_manifest("test", SourceSplit::Development, &catalog);
        let blind = build_source_manifest("test", SourceSplit::Blind, &catalog);
        assert_eq!((dev.entries.len(), blind.entries.len()), (4, 4));
        assert!(dev.entries.iter().all(|left| blind
            .entries
            .iter()
            .all(|right| left.mechanism_id != right.mechanism_id)));
        assert!(dev.entries.iter().chain(&blind.entries).all(|entry| !entry
            .target_pair_metadata_included
            && !entry.human_analogy_label_included));
    }
}
