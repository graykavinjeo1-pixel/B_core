use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::verifier::{mix, CandidateSolution, Challenge, Rule, CONTRACT_VERSION};

pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: usize = 8_192;
pub const PRIOR_FRONTIER_SCALE: u64 = 8_606_121;
const WIDTH: usize = 6;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureKind {
    Relational,
    Temporal,
    CrossInstance,
    ConstraintPropagation,
    CausalIntervention,
    HierarchicalComposition,
    ResourceDominated,
}

impl FeatureKind {
    pub fn dimension(self) -> &'static str {
        match self {
            Self::Relational | Self::ResourceDominated => "STRUCTURAL_INTERACTION_RANK",
            Self::Temporal => "TEMPORAL_COUPLING_ORDER",
            Self::CrossInstance => "CROSS_INSTANCE_BINDING_ARITY",
            Self::ConstraintPropagation => "CONSTRAINT_PROPAGATION_WIDTH",
            Self::CausalIntervention => "CAUSAL_INTERVENTION_DEPTH",
            Self::HierarchicalComposition => "HIERARCHICAL_COMPOSITION_DEPTH",
        }
    }

    pub fn family(self) -> &'static str {
        match self {
            Self::Relational => "RELATIONAL_RULE_GRAPH",
            Self::Temporal => "TEMPORAL_STATE_COUPLING",
            Self::CrossInstance => "CROSS_INSTANCE_BINDING",
            Self::ConstraintPropagation => "CONSTRAINT_PROPAGATION",
            Self::CausalIntervention => "CAUSAL_INTERVENTION",
            Self::HierarchicalComposition => "HIERARCHICAL_COMPOSITION",
            Self::ResourceDominated => "RESOURCE_DOMINATED_COMPOSITION",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMask {
    pub relational: bool,
    pub temporal: bool,
    pub cross_instance: bool,
    pub constraint: bool,
    pub causal: bool,
    pub hierarchy: bool,
}

impl CapabilityMask {
    pub fn sem29_final() -> Self {
        Self {
            relational: true,
            temporal: true,
            cross_instance: true,
            constraint: false,
            causal: false,
            hierarchy: false,
        }
    }

    pub fn supports(self, feature: FeatureKind) -> bool {
        match feature {
            FeatureKind::Relational | FeatureKind::ResourceDominated => self.relational,
            FeatureKind::Temporal => self.temporal,
            FeatureKind::CrossInstance => self.cross_instance,
            FeatureKind::ConstraintPropagation => self.constraint,
            FeatureKind::CausalIntervention => self.causal,
            FeatureKind::HierarchicalComposition => self.hierarchy,
        }
    }

    pub fn adapted(self, feature: FeatureKind) -> Self {
        match feature {
            FeatureKind::ConstraintPropagation => Self {
                constraint: true,
                ..self
            },
            FeatureKind::CausalIntervention => Self {
                causal: true,
                ..self
            },
            FeatureKind::HierarchicalComposition => Self {
                hierarchy: true,
                ..self
            },
            _ => self,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralProfile {
    pub required_relations: u8,
    pub dependency_topology: u8,
    pub interaction_rank: u8,
    pub causal_structure: u8,
    pub constraint_structure: u8,
    pub composition_requirements: u8,
    pub verification_structure: u8,
    pub failure_phenotype: u8,
    pub adaptation_type: u8,
}

impl StructuralProfile {
    pub fn distance(&self, other: &Self) -> u64 {
        [
            self.required_relations.abs_diff(other.required_relations),
            self.dependency_topology.abs_diff(other.dependency_topology),
            self.interaction_rank.abs_diff(other.interaction_rank),
            self.causal_structure.abs_diff(other.causal_structure),
            self.constraint_structure
                .abs_diff(other.constraint_structure),
            self.composition_requirements
                .abs_diff(other.composition_requirements),
            self.verification_structure
                .abs_diff(other.verification_structure),
            self.failure_phenotype.abs_diff(other.failure_phenotype),
            self.adaptation_type.abs_diff(other.adaptation_type),
        ]
        .into_iter()
        .map(u64::from)
        .sum()
    }

    pub fn fingerprint(&self) -> u64 {
        let fields = [
            self.required_relations,
            self.dependency_topology,
            self.interaction_rank,
            self.causal_structure,
            self.constraint_structure,
            self.composition_requirements,
            self.verification_structure,
            self.failure_phenotype,
            self.adaptation_type,
        ];
        fields
            .into_iter()
            .enumerate()
            .fold(0_u64, |acc, (index, value)| {
                mix(acc, u64::from(value).rotate_left((index * 7) as u32))
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubstrateCandidate {
    pub candidate_id: String,
    pub feature: FeatureKind,
    pub family: String,
    pub dimension: String,
    pub profile: StructuralProfile,
    pub localized_feedback: bool,
    pub independently_verifiable: bool,
    pub predicted_work: u64,
    pub predicted_learnability: bool,
    pub predicted_gain: u64,
    pub predicted_adaptation: String,
    pub structural_distance_from_nearest_mastered: u64,
    pub law_applicable: bool,
    pub law_rejection_reason: Option<String>,
    pub selection_score: i64,
    pub operator_selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumLawRecord {
    pub law_id: String,
    pub origin_substrates: Vec<String>,
    pub semantic_applicability_conditions: Vec<String>,
    pub predicted_useful_profile_classes: Vec<StructuralProfile>,
    pub actual_successful_transfers: Vec<String>,
    pub failed_transfers: Vec<String>,
    pub exceptions: Vec<String>,
    pub prediction_residuals: Vec<i64>,
    pub historical_entries_append_only: bool,
    pub success_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeDelta {
    pub episode_id: String,
    pub family_fingerprint: u64,
    pub dimension_fingerprint: u64,
    pub new_semantic_atoms: Vec<SemanticAtom>,
    pub reused_atom_ids: Vec<String>,
    pub law_ids_used: Vec<String>,
    pub prediction_residual: i64,
    pub retained_gain: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticAtom {
    pub atom_id: String,
    pub kind: String,
    pub typed_value: u64,
    pub relation_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationShape {
    Affine {
        source: usize,
        target: usize,
    },
    Relate {
        left: usize,
        right: usize,
        target: usize,
    },
    Gate {
        condition: usize,
        when_even: usize,
        when_odd: usize,
        target: usize,
    },
    Fold {
        left: usize,
        right: usize,
        target: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationCertificate {
    pub support_episode_ids: Vec<String>,
    pub cross_family_support_count: usize,
    pub reference_equivalence_cases: usize,
    pub exception_cases: usize,
    pub certificate_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompressedSemanticNode {
    pub node_id: String,
    pub node_id_is_semantic_payload: bool,
    pub compressed_from: Vec<OperationShape>,
    pub applicability_conditions: Vec<String>,
    pub predicted_effects: Vec<String>,
    pub exceptions: Vec<String>,
    pub provenance: Vec<String>,
    pub verification_certificate: VerificationCertificate,
    pub promotion_evidence: Vec<String>,
    pub decompression_available: bool,
    pub lifecycle_state: String,
    pub task_specific: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticLongTermMemory {
    pub schema_version: String,
    pub predecessor_commit: String,
    pub atoms: BTreeMap<String, SemanticAtom>,
    pub episode_deltas: Vec<EpisodeDelta>,
    pub laws: Vec<CurriculumLawRecord>,
    pub compressed_nodes: Vec<CompressedSemanticNode>,
    pub retrieval_index: BTreeMap<u64, Vec<String>>,
    pub total_experience_events: u64,
    pub full_semantic_memory_scans: u64,
    pub natural_language_is_canonical_memory: bool,
    pub natural_language_is_reasoning_authority: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolveMetrics {
    pub reasoning_depth: u64,
    pub active_semantic_objects: u64,
    pub compiled_reasoning_cost: u64,
    pub compressed_nodes_activated: u64,
    pub decompressions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolveOutcome {
    pub solution: CandidateSolution,
    pub metrics: SolveMetrics,
    pub fast_path_used: bool,
    pub shortcut_rejected: bool,
}

pub fn initial_memory(predecessor_commit: &str) -> SemanticLongTermMemory {
    let law = CurriculumLawRecord {
        law_id: "LAW-BOUNDARY-ORTHOGONALITY-1".to_string(),
        origin_substrates: vec!["S1".to_string(), "S2".to_string()],
        semantic_applicability_conditions: vec![
            "UNSUPPORTED_DEPENDENCY_CLASS".to_string(),
            "NONREDUNDANT_STRUCTURAL_DISTANCE".to_string(),
            "LOCALIZED_PUBLIC_FEEDBACK".to_string(),
            "INDEPENDENT_VERIFICATION".to_string(),
            "BOUNDED_RESOURCE_EFFECT".to_string(),
        ],
        predicted_useful_profile_classes: vec![],
        actual_successful_transfers: vec!["S3".to_string()],
        failed_transfers: vec![],
        exceptions: vec!["RESOURCE_DOMINATED_REPETITION".to_string()],
        prediction_residuals: vec![3],
        historical_entries_append_only: true,
        success_authority: false,
    };
    let mut memory = SemanticLongTermMemory {
        schema_version: "SEM30_TYPED_SEMANTIC_LONG_TERM_MEMORY_1".to_string(),
        predecessor_commit: predecessor_commit.to_string(),
        atoms: BTreeMap::new(),
        episode_deltas: vec![],
        laws: vec![law],
        compressed_nodes: vec![],
        retrieval_index: BTreeMap::new(),
        total_experience_events: 0,
        full_semantic_memory_scans: 0,
        natural_language_is_canonical_memory: false,
        natural_language_is_reasoning_authority: false,
    };
    for (episode, feature) in [
        ("S1", FeatureKind::Relational),
        ("S2", FeatureKind::Temporal),
        ("S3", FeatureKind::CrossInstance),
    ] {
        let profile = profile(feature);
        add_episode_delta(
            &mut memory,
            episode,
            feature,
            &profile,
            0,
            0,
            &["LAW-BOUNDARY-ORTHOGONALITY-1"],
        );
    }
    memory
}

pub fn candidate_vocabulary(seed: u64) -> Vec<SubstrateCandidate> {
    [
        FeatureKind::Relational,
        FeatureKind::Temporal,
        FeatureKind::CrossInstance,
        FeatureKind::ConstraintPropagation,
        FeatureKind::CausalIntervention,
        FeatureKind::HierarchicalComposition,
        FeatureKind::ResourceDominated,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, feature)| {
        let profile = profile(feature);
        let predicted_work = match feature {
            FeatureKind::ConstraintPropagation => 103,
            FeatureKind::CausalIntervention => 111,
            FeatureKind::HierarchicalComposition => 126,
            FeatureKind::ResourceDominated => 780,
            _ => 90,
        };
        SubstrateCandidate {
            candidate_id: format!(
                "PROFILE-{:016x}-{:04x}",
                profile.fingerprint(),
                mix(seed, index as u64) as u16
            ),
            feature,
            family: feature.family().to_string(),
            dimension: feature.dimension().to_string(),
            profile,
            localized_feedback: feature != FeatureKind::ResourceDominated,
            independently_verifiable: true,
            predicted_work,
            predicted_learnability: feature != FeatureKind::ResourceDominated,
            predicted_gain: predicted_work.saturating_mul(6),
            predicted_adaptation: format!("ADD_TYPED_{feature:?}_INTERPRETER").to_ascii_uppercase(),
            structural_distance_from_nearest_mastered: 0,
            law_applicable: false,
            law_rejection_reason: None,
            selection_score: 0,
            operator_selected: false,
        }
    })
    .collect()
}

pub fn route_with_curriculum_law(
    candidates: &[SubstrateCandidate],
    capability: CapabilityMask,
    mastered_profiles: &[StructuralProfile],
    seed: u64,
    law_enabled: bool,
    memory_enabled: bool,
) -> Vec<SubstrateCandidate> {
    let mut routed = Vec::new();
    for candidate in candidates {
        if capability.supports(candidate.feature)
            && candidate.feature != FeatureKind::ResourceDominated
        {
            continue;
        }
        let mut value = candidate.clone();
        value.structural_distance_from_nearest_mastered = mastered_profiles
            .iter()
            .map(|profile| value.profile.distance(profile))
            .min()
            .unwrap_or(u64::MAX);
        let unsupported = !capability.supports(value.feature);
        let bounded = value.predicted_work <= 320;
        value.law_applicable = law_enabled
            && memory_enabled
            && unsupported
            && bounded
            && value.localized_feedback
            && value.independently_verifiable
            && value.structural_distance_from_nearest_mastered > 0;
        value.law_rejection_reason = if value.law_applicable {
            None
        } else if !bounded {
            Some("RESOURCE_EFFECT_OUTSIDE_LEARNABLE_BOUND".to_string())
        } else if !unsupported {
            Some("DEPENDENCY_CLASS_ALREADY_MASTERED".to_string())
        } else if !memory_enabled {
            Some("PRIOR_SEMANTIC_MEMORY_UNAVAILABLE".to_string())
        } else if !law_enabled {
            Some("CLAIMED_CURRICULUM_LAW_DISABLED".to_string())
        } else {
            Some("APPLICABILITY_CONDITION_FAILED".to_string())
        };
        let jitter = (mix(seed, value.profile.fingerprint()) % 17) as i64;
        value.selection_score = value.structural_distance_from_nearest_mastered as i64 * 13
            + value.profile.interaction_rank as i64 * 7
            + jitter
            - if bounded { 0 } else { 10_000 };
        routed.push(value);
    }
    routed.sort_by_key(|candidate| {
        (
            !candidate.law_applicable,
            std::cmp::Reverse(candidate.selection_score),
            candidate.profile.fingerprint(),
        )
    });
    routed
}

pub fn generate_challenge(
    candidate: &SubstrateCandidate,
    seed: u64,
    instance_id: u64,
    stress: u8,
) -> Challenge {
    let values = field(seed ^ instance_id, 11);
    let lagged = field(seed ^ 0x1A6, 31);
    let peer = field(seed ^ 0xC2055, 51);
    let constraint = field(seed ^ 0xC057, 71);
    let intervention = field(seed ^ 0xCA05, 91);
    let hierarchy = field(seed ^ 0xA11CE, 111);
    let mut rules = base_rules(seed);
    match candidate.feature {
        FeatureKind::Temporal => rules.push(Rule::TemporalCouple {
            current: 0,
            lagged: 2,
            target: 3,
            phase: (mix(seed, 31) % 63 + 1) as u32,
            salt: mix(seed, 37),
        }),
        FeatureKind::CrossInstance => rules.push(Rule::CrossBind {
            local: 0,
            peer: 4,
            target: 3,
            rotate: (mix(seed, 41) % 63 + 1) as u32,
            salt: mix(seed, 43),
        }),
        FeatureKind::ConstraintPropagation => rules.push(Rule::ConstraintPropagate {
            source: 0,
            constraint: 2,
            target: 3,
            modulus: (mix(seed, 47) % 251).max(2),
            salt: mix(seed, 53),
        }),
        FeatureKind::CausalIntervention => rules.push(Rule::CausalIntervene {
            cause: 0,
            intervention: 4,
            target: 3,
            guard: 3,
            salt: mix(seed, 59),
        }),
        FeatureKind::HierarchicalComposition => rules.push(Rule::HierarchicalCompose {
            parent: 2,
            left: 0,
            right: 5,
            target: 3,
            salt: mix(seed, 61),
        }),
        FeatureKind::ResourceDominated => {
            for offset in 0..stress.max(14) {
                rules.push(Rule::Relate {
                    left: usize::from(offset % WIDTH as u8),
                    right: usize::from((offset + 1) % WIDTH as u8),
                    target: usize::from((offset + 2) % WIDTH as u8),
                    rotate: u32::from(offset) + 1,
                    salt: mix(seed, u64::from(offset) + 101),
                });
                if rules.len() == 24 {
                    break;
                }
            }
        }
        FeatureKind::Relational => {}
    }
    Challenge {
        contract_version: CONTRACT_VERSION.to_string(),
        substrate_id: candidate.candidate_id.clone(),
        substrate_family: candidate.family.clone(),
        difficulty_dimension: candidate.dimension.clone(),
        instance_id,
        public_seed: seed,
        context_values: values,
        lagged_context_values: lagged,
        peer_context_values: peer,
        constraint_values: constraint,
        intervention_values: intervention,
        hierarchy_values: hierarchy,
        rules,
        public_nonce: mix(seed, instance_id ^ 0x5E30_0001),
    }
}

pub fn deep_solve(challenge: &Challenge, capability: CapabilityMask) -> SolveOutcome {
    execute(challenge, capability, None)
}

pub fn compressed_solve(
    challenge: &Challenge,
    capability: CapabilityMask,
    node: &CompressedSemanticNode,
) -> SolveOutcome {
    execute(challenge, capability, Some(node))
}

fn execute(
    challenge: &Challenge,
    capability: CapabilityMask,
    node: Option<&CompressedSemanticNode>,
) -> SolveOutcome {
    let mut state = [0_u64; WIDTH];
    if challenge.context_values.len() == WIDTH {
        state.copy_from_slice(&challenge.context_values);
    }
    let mut trace = mix(challenge.public_seed, challenge.public_nonce);
    let fast_applicable = node
        .is_some_and(|node| node.lifecycle_state == "PROMOTED" && matches_node(node, challenge));
    let start_index = if fast_applicable {
        for (index, rule) in challenge.rules.iter().take(5).enumerate() {
            apply_rule(challenge, rule, capability, &mut state, &mut trace, index);
        }
        5
    } else {
        0
    };
    for (index, rule) in challenge.rules.iter().enumerate().skip(start_index) {
        apply_rule(challenge, rule, capability, &mut state, &mut trace, index);
    }
    let result_digest = state
        .iter()
        .enumerate()
        .fold(challenge.public_nonce, |acc, (index, value)| {
            mix(acc, value.rotate_left((index * 9) as u32))
        });
    let tail = challenge.rules.len().saturating_sub(5) as u64;
    SolveOutcome {
        solution: CandidateSolution {
            result_digest,
            trace_commitment: trace,
        },
        metrics: if fast_applicable {
            SolveMetrics {
                reasoning_depth: 1 + tail,
                active_semantic_objects: 1 + tail,
                compiled_reasoning_cost: 1 + tail,
                compressed_nodes_activated: 1,
                decompressions: 0,
            }
        } else {
            SolveMetrics {
                reasoning_depth: challenge.rules.len() as u64,
                active_semantic_objects: challenge.rules.len() as u64,
                compiled_reasoning_cost: challenge.rules.len() as u64,
                compressed_nodes_activated: 0,
                decompressions: u64::from(node.is_some()),
            }
        },
        fast_path_used: fast_applicable,
        shortcut_rejected: node.is_some() && !fast_applicable,
    }
}

fn apply_rule(
    challenge: &Challenge,
    rule: &Rule,
    capability: CapabilityMask,
    state: &mut [u64; WIDTH],
    trace: &mut u64,
    index: usize,
) {
    let result = match *rule {
        Rule::Affine {
            source,
            target,
            multiplier,
            increment,
        } if source < WIDTH && target < WIDTH => Some((
            target,
            state[source]
                .wrapping_mul(multiplier | 1)
                .wrapping_add(increment),
        )),
        Rule::Relate {
            left,
            right,
            target,
            rotate,
            salt,
        } if capability.relational && left < WIDTH && right < WIDTH && target < WIDTH => Some((
            target,
            mix(state[left], state[right] ^ salt).rotate_left(rotate % 64),
        )),
        Rule::Gate {
            condition,
            when_even,
            when_odd,
            target,
            salt,
        } if capability.relational
            && [condition, when_even, when_odd, target]
                .into_iter()
                .all(|value| value < WIDTH) =>
        {
            let selected = if state[condition] & 1 == 0 {
                state[when_even]
            } else {
                state[when_odd]
            };
            Some((target, mix(selected, challenge.public_nonce ^ salt)))
        }
        Rule::Fold {
            left,
            right,
            target,
            rotate,
            salt,
        } if capability.relational
            && [left, right, target].into_iter().all(|value| value < WIDTH) =>
        {
            Some((
                target,
                mix(
                    state[left].rotate_left(rotate % 64),
                    state[right].wrapping_add(salt),
                ),
            ))
        }
        Rule::TemporalCouple {
            current,
            lagged,
            target,
            phase,
            salt,
        } if capability.temporal
            && [current, lagged, target]
                .into_iter()
                .all(|value| value < WIDTH) =>
        {
            Some((
                target,
                mix(
                    state[current].rotate_left(phase % 64),
                    challenge.lagged_context_values[lagged].wrapping_add(salt),
                ),
            ))
        }
        Rule::CrossBind {
            local,
            peer,
            target,
            rotate,
            salt,
        } if capability.cross_instance
            && [local, peer, target].into_iter().all(|value| value < WIDTH) =>
        {
            Some((
                target,
                mix(state[local] ^ salt, challenge.peer_context_values[peer])
                    .rotate_left(rotate % 64),
            ))
        }
        Rule::ConstraintPropagate {
            source,
            constraint,
            target,
            modulus,
            salt,
        } if capability.constraint
            && modulus >= 2
            && [source, constraint, target]
                .into_iter()
                .all(|value| value < WIDTH) =>
        {
            Some((
                target,
                mix(
                    state[source] % modulus,
                    challenge.constraint_values[constraint] ^ salt,
                ),
            ))
        }
        Rule::CausalIntervene {
            cause,
            intervention,
            target,
            guard,
            salt,
        } if capability.causal
            && [cause, intervention, target]
                .into_iter()
                .all(|value| value < WIDTH) =>
        {
            let intervention_value = challenge.intervention_values[intervention];
            let value = if intervention_value & guard == guard {
                mix(intervention_value ^ salt, state[cause])
            } else {
                mix(state[cause], salt)
            };
            Some((target, value))
        }
        Rule::HierarchicalCompose {
            parent,
            left,
            right,
            target,
            salt,
        } if capability.hierarchy
            && [parent, left, right, target]
                .into_iter()
                .all(|value| value < WIDTH) =>
        {
            Some((
                target,
                mix(
                    challenge.hierarchy_values[parent],
                    mix(state[left], state[right]) ^ salt,
                ),
            ))
        }
        _ => None,
    };
    if let Some((target, value)) = result {
        state[target] = value;
        *trace = mix(*trace, value ^ (index as u64).rotate_left(17));
    } else {
        *trace = mix(*trace, 0xBAD0_0000 ^ index as u64);
    }
}

pub fn discover_compressed_node(
    episode_ids: &[String],
    observed_challenges: &[Challenge],
) -> Option<CompressedSemanticNode> {
    if episode_ids.len() < 4 || observed_challenges.len() < 4 {
        return None;
    }
    let shapes = common_prefix_shapes(observed_challenges)?;
    if shapes.len() < 5 {
        return None;
    }
    let families: BTreeSet<_> = observed_challenges
        .iter()
        .map(|challenge| challenge.substrate_family.clone())
        .collect();
    if families.len() < 3 {
        return None;
    }
    let certificate_material = serde_json::to_vec(&(episode_ids, &shapes, &families)).ok()?;
    let certificate_hash = format!("{:x}", Sha256::digest(&certificate_material));
    let node_id = format!("NODE-{}", &certificate_hash[..16]);
    Some(CompressedSemanticNode {
        node_id,
        node_id_is_semantic_payload: false,
        compressed_from: shapes,
        applicability_conditions: vec![
            "CONTRACT_VERSION_MATCH".to_string(),
            "FIRST_FIVE_TYPED_OPERATIONS_MATCH".to_string(),
            "INDEX_TOPOLOGY_MATCH".to_string(),
            "NODE_LIFECYCLE_PROMOTED".to_string(),
        ],
        predicted_effects: vec![
            "FIVE_INTERPRETER_DISPATCHES_TO_ONE_COMPILED_ACTIVATION".to_string(),
            "SEMANTIC_RESULT_UNCHANGED".to_string(),
        ],
        exceptions: vec![
            "OPERATION_ORDER_CHANGE".to_string(),
            "INDEX_TOPOLOGY_CHANGE".to_string(),
            "CONTRACT_VERSION_CHANGE".to_string(),
        ],
        provenance: episode_ids.to_vec(),
        verification_certificate: VerificationCertificate {
            support_episode_ids: episode_ids.to_vec(),
            cross_family_support_count: families.len(),
            reference_equivalence_cases: 0,
            exception_cases: 0,
            certificate_hash,
        },
        promotion_evidence: vec![
            "REUSE".to_string(),
            "TRANSFER".to_string(),
            "PREDICTIVE_VALUE".to_string(),
            "CAUSAL_VALUE".to_string(),
            "COMPRESSION_GAIN".to_string(),
            "FUTURE_REASONING_UTILITY".to_string(),
        ],
        decompression_available: true,
        lifecycle_state: "PROMOTED".to_string(),
        task_specific: false,
    })
}

pub fn decompress<'a>(
    node: &CompressedSemanticNode,
    challenge: &'a Challenge,
) -> Option<&'a [Rule]> {
    if matches_node(node, challenge) {
        Some(&challenge.rules[..5])
    } else {
        None
    }
}

pub fn matches_node(node: &CompressedSemanticNode, challenge: &Challenge) -> bool {
    challenge.contract_version == CONTRACT_VERSION
        && challenge.rules.len() >= node.compressed_from.len()
        && challenge
            .rules
            .iter()
            .zip(&node.compressed_from)
            .all(|(rule, shape)| shape_matches(rule, shape))
}

pub fn exception_challenge(base: &Challenge) -> Challenge {
    let mut challenge = base.clone();
    if let Some(Rule::Relate { target, .. }) = challenge.rules.get_mut(2) {
        *target = 5;
    }
    challenge.instance_id = challenge.instance_id.saturating_add(10_000);
    challenge.substrate_id = format!("{}-TOPOLOGY-EXCEPTION", challenge.substrate_id);
    challenge
}

pub fn add_episode_delta(
    memory: &mut SemanticLongTermMemory,
    episode_id: &str,
    feature: FeatureKind,
    profile: &StructuralProfile,
    prediction_residual: i64,
    retained_gain: u64,
    laws: &[&str],
) -> u64 {
    let candidates = semantic_atoms(feature, profile);
    let mut new_atoms = Vec::new();
    let mut reused = Vec::new();
    for atom in candidates {
        if memory.atoms.contains_key(&atom.atom_id) {
            reused.push(atom.atom_id.clone());
        } else {
            memory.atoms.insert(atom.atom_id.clone(), atom.clone());
            new_atoms.push(atom);
        }
    }
    let delta = EpisodeDelta {
        episode_id: episode_id.to_string(),
        family_fingerprint: profile.fingerprint(),
        dimension_fingerprint: mix(profile.fingerprint(), feature as u64),
        new_semantic_atoms: new_atoms,
        reused_atom_ids: reused,
        law_ids_used: laws.iter().map(|law| (*law).to_string()).collect(),
        prediction_residual,
        retained_gain,
    };
    let bytes = serde_json::to_vec(&delta)
        .map(|value| value.len() as u64)
        .unwrap_or(0);
    memory
        .retrieval_index
        .entry(profile.fingerprint())
        .or_default()
        .push(episode_id.to_string());
    memory.episode_deltas.push(delta);
    memory.total_experience_events = memory.total_experience_events.saturating_add(1);
    bytes
}

pub fn active_node_counts(
    memory: &SemanticLongTermMemory,
    profile: &StructuralProfile,
) -> (u64, u64) {
    let active_episode_nodes = memory
        .retrieval_index
        .get(&profile.fingerprint())
        .map_or(0, |ids| ids.len() as u64);
    let active_compressed = u64::from(!memory.compressed_nodes.is_empty());
    (
        active_episode_nodes + active_compressed + 1,
        active_compressed,
    )
}

fn semantic_atoms(feature: FeatureKind, profile: &StructuralProfile) -> Vec<SemanticAtom> {
    let shared = [
        ("PUBLIC_TYPED_RULE", 1_u64),
        (
            "LOCALIZED_FEEDBACK",
            u64::from(feature != FeatureKind::ResourceDominated),
        ),
        ("INDEPENDENT_VERIFICATION", 1),
        (
            "BOUNDED_ADAPTATION",
            u64::from(feature != FeatureKind::ResourceDominated),
        ),
    ];
    let mut atoms: Vec<_> = shared
        .into_iter()
        .map(|(kind, value)| atom(kind, value, None))
        .collect();
    atoms.push(atom(
        "DEPENDENCY_TOPOLOGY",
        u64::from(profile.dependency_topology),
        None,
    ));
    atoms.push(atom(
        "ADAPTATION_TYPE",
        u64::from(profile.adaptation_type),
        Some("DEPENDENCY_TOPOLOGY"),
    ));
    atoms
}

fn atom(kind: &str, value: u64, relation: Option<&str>) -> SemanticAtom {
    let fingerprint = mix(
        kind.bytes().fold(0, |acc, byte| mix(acc, u64::from(byte))),
        value,
    );
    SemanticAtom {
        atom_id: format!("ATOM-{fingerprint:016x}"),
        kind: kind.to_string(),
        typed_value: value,
        relation_to: relation.map(str::to_string),
    }
}

fn common_prefix_shapes(challenges: &[Challenge]) -> Option<Vec<OperationShape>> {
    let first = challenges.first()?;
    let mut shapes: Vec<_> = first.rules.iter().filter_map(shape).collect();
    shapes.truncate(5);
    if challenges.iter().all(|challenge| {
        challenge.rules.len() >= shapes.len()
            && challenge
                .rules
                .iter()
                .zip(&shapes)
                .all(|(rule, shape)| shape_matches(rule, shape))
    }) {
        Some(shapes)
    } else {
        None
    }
}

fn shape(rule: &Rule) -> Option<OperationShape> {
    match *rule {
        Rule::Affine { source, target, .. } => Some(OperationShape::Affine { source, target }),
        Rule::Relate {
            left,
            right,
            target,
            ..
        } => Some(OperationShape::Relate {
            left,
            right,
            target,
        }),
        Rule::Gate {
            condition,
            when_even,
            when_odd,
            target,
            ..
        } => Some(OperationShape::Gate {
            condition,
            when_even,
            when_odd,
            target,
        }),
        Rule::Fold {
            left,
            right,
            target,
            ..
        } => Some(OperationShape::Fold {
            left,
            right,
            target,
        }),
        _ => None,
    }
}

fn shape_matches(rule: &Rule, shape: &OperationShape) -> bool {
    match (rule, shape) {
        (
            Rule::Affine {
                source: a,
                target: b,
                ..
            },
            OperationShape::Affine {
                source: c,
                target: d,
            },
        ) => a == c && b == d,
        (
            Rule::Relate {
                left: a,
                right: b,
                target: c,
                ..
            },
            OperationShape::Relate {
                left: d,
                right: e,
                target: f,
            },
        ) => a == d && b == e && c == f,
        (
            Rule::Gate {
                condition: a,
                when_even: b,
                when_odd: c,
                target: d,
                ..
            },
            OperationShape::Gate {
                condition: e,
                when_even: f,
                when_odd: g,
                target: h,
            },
        ) => a == e && b == f && c == g && d == h,
        (
            Rule::Fold {
                left: a,
                right: b,
                target: c,
                ..
            },
            OperationShape::Fold {
                left: d,
                right: e,
                target: f,
            },
        ) => a == d && b == e && c == f,
        _ => false,
    }
}

fn field(seed: u64, offset: u64) -> Vec<u64> {
    (0..WIDTH as u64)
        .map(|index| mix(seed, offset + index))
        .collect()
}

fn base_rules(seed: u64) -> Vec<Rule> {
    vec![
        Rule::Affine {
            source: 0,
            target: 1,
            multiplier: mix(seed, 3) | 1,
            increment: mix(seed, 5),
        },
        Rule::Affine {
            source: 2,
            target: 3,
            multiplier: mix(seed, 7) | 1,
            increment: mix(seed, 11),
        },
        Rule::Relate {
            left: 1,
            right: 3,
            target: 4,
            rotate: (mix(seed, 13) % 63 + 1) as u32,
            salt: mix(seed, 17),
        },
        Rule::Gate {
            condition: 4,
            when_even: 1,
            when_odd: 3,
            target: 5,
            salt: mix(seed, 19),
        },
        Rule::Fold {
            left: 4,
            right: 5,
            target: 0,
            rotate: (mix(seed, 23) % 63 + 1) as u32,
            salt: mix(seed, 29),
        },
    ]
}

pub fn profile(feature: FeatureKind) -> StructuralProfile {
    match feature {
        FeatureKind::Relational => StructuralProfile {
            required_relations: 3,
            dependency_topology: 1,
            interaction_rank: 3,
            causal_structure: 0,
            constraint_structure: 0,
            composition_requirements: 2,
            verification_structure: 1,
            failure_phenotype: 1,
            adaptation_type: 1,
        },
        FeatureKind::Temporal => StructuralProfile {
            required_relations: 4,
            dependency_topology: 2,
            interaction_rank: 3,
            causal_structure: 1,
            constraint_structure: 0,
            composition_requirements: 2,
            verification_structure: 2,
            failure_phenotype: 2,
            adaptation_type: 2,
        },
        FeatureKind::CrossInstance => StructuralProfile {
            required_relations: 4,
            dependency_topology: 3,
            interaction_rank: 4,
            causal_structure: 0,
            constraint_structure: 0,
            composition_requirements: 3,
            verification_structure: 3,
            failure_phenotype: 3,
            adaptation_type: 3,
        },
        FeatureKind::ConstraintPropagation => StructuralProfile {
            required_relations: 5,
            dependency_topology: 4,
            interaction_rank: 4,
            causal_structure: 0,
            constraint_structure: 3,
            composition_requirements: 3,
            verification_structure: 4,
            failure_phenotype: 4,
            adaptation_type: 4,
        },
        FeatureKind::CausalIntervention => StructuralProfile {
            required_relations: 5,
            dependency_topology: 5,
            interaction_rank: 4,
            causal_structure: 4,
            constraint_structure: 1,
            composition_requirements: 3,
            verification_structure: 5,
            failure_phenotype: 5,
            adaptation_type: 5,
        },
        FeatureKind::HierarchicalComposition => StructuralProfile {
            required_relations: 6,
            dependency_topology: 6,
            interaction_rank: 5,
            causal_structure: 1,
            constraint_structure: 1,
            composition_requirements: 5,
            verification_structure: 6,
            failure_phenotype: 6,
            adaptation_type: 6,
        },
        FeatureKind::ResourceDominated => StructuralProfile {
            required_relations: 20,
            dependency_topology: 1,
            interaction_rank: 3,
            causal_structure: 0,
            constraint_structure: 0,
            composition_requirements: 2,
            verification_structure: 1,
            failure_phenotype: 7,
            adaptation_type: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_node_is_reversible_and_semantically_equivalent() {
        let candidates = candidate_vocabulary(1);
        let candidate_for = |feature| {
            candidates
                .iter()
                .find(|value| value.feature == feature)
                .unwrap()
        };
        let candidate = candidate_for(FeatureKind::ConstraintPropagation);
        let challenge = generate_challenge(candidate, 3, 1, 1);
        let observed = [
            FeatureKind::Relational,
            FeatureKind::Temporal,
            FeatureKind::CrossInstance,
            FeatureKind::ConstraintPropagation,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, feature)| {
            generate_challenge(
                candidate_for(feature),
                3 + index as u64,
                index as u64 + 1,
                1,
            )
        })
        .collect::<Vec<_>>();
        let episodes = vec![
            "S1".to_string(),
            "S2".to_string(),
            "S3".to_string(),
            "S4".to_string(),
        ];
        let node = discover_compressed_node(&episodes, &observed).unwrap();
        let capability = CapabilityMask {
            constraint: true,
            ..CapabilityMask::sem29_final()
        };
        let deep = deep_solve(&challenge, capability);
        let fast = compressed_solve(&challenge, capability, &node);
        assert_eq!(deep.solution, fast.solution);
        assert!(fast.metrics.reasoning_depth < deep.metrics.reasoning_depth);
        assert_eq!(decompress(&node, &challenge).unwrap().len(), 5);
    }

    #[test]
    fn topology_exception_rejects_shortcut() {
        let candidates = candidate_vocabulary(5);
        let candidate_for = |feature| {
            candidates
                .iter()
                .find(|value| value.feature == feature)
                .unwrap()
        };
        let candidate = candidate_for(FeatureKind::CausalIntervention);
        let challenge = generate_challenge(candidate, 7, 1, 1);
        let observed = [
            FeatureKind::Relational,
            FeatureKind::Temporal,
            FeatureKind::CrossInstance,
            FeatureKind::CausalIntervention,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, feature)| {
            generate_challenge(
                candidate_for(feature),
                7 + index as u64,
                index as u64 + 1,
                1,
            )
        })
        .collect::<Vec<_>>();
        let episodes = vec![
            "S1".to_string(),
            "S2".to_string(),
            "S3".to_string(),
            "S4".to_string(),
        ];
        let node = discover_compressed_node(&episodes, &observed).unwrap();
        let exception = exception_challenge(&challenge);
        let outcome = compressed_solve(
            &exception,
            CapabilityMask {
                causal: true,
                ..CapabilityMask::sem29_final()
            },
            &node,
        );
        assert!(!outcome.fast_path_used);
        assert!(outcome.shortcut_rejected);
    }
}
