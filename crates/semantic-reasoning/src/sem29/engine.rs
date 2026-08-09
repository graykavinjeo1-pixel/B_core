use serde::{Deserialize, Serialize};

use super::verifier::{mix, CandidateSolution, Challenge, Rule, CONTRACT_VERSION};

pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: usize = 4_096;
pub const PRIOR_FRONTIER_SCALE: u64 = 8_605_137;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureKind {
    RelationalEcho,
    TemporalCoupling,
    CrossInstanceBinding,
    ResourceDominatedComposition,
}

impl FeatureKind {
    pub fn family(self) -> &'static str {
        match self {
            Self::RelationalEcho => "RELATIONAL_RULE_GRAPH",
            Self::TemporalCoupling => "TEMPORAL_STATE_COUPLING",
            Self::CrossInstanceBinding => "CROSS_INSTANCE_BINDING",
            Self::ResourceDominatedComposition => "RESOURCE_DOMINATED_COMPOSITION",
        }
    }

    pub fn dimension(self) -> &'static str {
        match self {
            Self::RelationalEcho | Self::ResourceDominatedComposition => {
                "STRUCTURAL_INTERACTION_RANK"
            }
            Self::TemporalCoupling => "TEMPORAL_COUPLING_ORDER",
            Self::CrossInstanceBinding => "CROSS_INSTANCE_BINDING_ARITY",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundaryPattern {
    pub substrate_id: String,
    pub mastered_dimensions: Vec<String>,
    pub failure_signature: String,
    pub successful_adaptation: String,
    pub verifier_work_units: u64,
    pub retained_gain: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumEpisode {
    pub substrate_id: String,
    pub family: String,
    pub difficulty_dimension: String,
    pub hypothesis_count: u64,
    pub failed_candidates: u64,
    pub calibration_probes: u64,
    pub genesis_cost: u64,
    pub time_to_learnable_frontier: u64,
    pub time_to_retained_gain: u64,
    pub prediction_error: u64,
    pub retained_capability_gain: u64,
    pub future_substrates_enabled: u64,
    pub future_dimensions_enabled: u64,
    pub future_laws_enabled: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumMotif {
    pub motif_id: String,
    pub support: usize,
    pub statement: String,
    pub success_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumLaw {
    pub law_id: String,
    pub support: usize,
    pub antecedent: String,
    pub routing_effect: String,
    pub requires_fresh_independent_verification: bool,
    pub success_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumSchema {
    pub schema_id: String,
    pub support: usize,
    pub stages: Vec<String>,
    pub success_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PredictorState {
    pub observations: usize,
    pub verifier_work_bias: i64,
    pub novelty_weight: i64,
    pub learnability_weight: i64,
    pub calibration_residuals: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurriculumResearchMemory {
    pub schema_version: String,
    pub predecessor_commit: String,
    pub boundary_patterns: Vec<BoundaryPattern>,
    pub episodes: Vec<CurriculumEpisode>,
    pub failed_candidate_patterns: Vec<String>,
    pub successful_candidate_patterns: Vec<String>,
    pub motifs: Vec<CurriculumMotif>,
    pub laws: Vec<CurriculumLaw>,
    pub schemas: Vec<CurriculumSchema>,
    pub predictor: PredictorState,
    pub generator_is_success_authority: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubstrateCandidate {
    pub candidate_id: String,
    pub feature: FeatureKind,
    pub substrate_family: String,
    pub difficulty_dimension: String,
    pub semantic_definition: String,
    pub mechanical_effect: String,
    pub predicted_boundary_stress: String,
    pub predicted_insufficiency: String,
    pub predicted_learnability: String,
    pub predicted_verifier_work_units: u64,
    pub predicted_structural_novelty: u64,
    pub predicted_adaptation: String,
    pub predicted_resource_effect: u64,
    pub predicted_frontier_effect: u64,
    pub routed_by_memory: bool,
    pub routed_by_law: bool,
    pub operator_selected: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMask {
    pub relational: bool,
    pub temporal: bool,
    pub cross_instance: bool,
}

impl CapabilityMask {
    pub fn supports(self, feature: FeatureKind) -> bool {
        match feature {
            FeatureKind::RelationalEcho | FeatureKind::ResourceDominatedComposition => {
                self.relational
            }
            FeatureKind::TemporalCoupling => self.temporal,
            FeatureKind::CrossInstanceBinding => self.cross_instance,
        }
    }

    pub fn adapted(self, feature: FeatureKind) -> Self {
        match feature {
            FeatureKind::TemporalCoupling => Self {
                temporal: true,
                ..self
            },
            FeatureKind::CrossInstanceBinding => Self {
                cross_instance: true,
                ..self
            },
            _ => self,
        }
    }
}

pub fn initial_memory(predecessor_commit: &str) -> CurriculumResearchMemory {
    CurriculumResearchMemory {
        schema_version: "SEM29_CURRICULUM_RESEARCH_MEMORY_1".to_string(),
        predecessor_commit: predecessor_commit.to_string(),
        boundary_patterns: vec![BoundaryPattern {
            substrate_id: "S1-SEM28-STRUCTURAL-INTERACTION".to_string(),
            mastered_dimensions: vec!["STRUCTURAL_INTERACTION_RANK".to_string()],
            failure_signature: "AFFINE_ONLY_SOLVER_REJECTED_COMPOSITE_RELATIONAL_GRAPH".to_string(),
            successful_adaptation: "PUBLIC_RELATIONAL_RULE_GRAPH_INTERPRETER".to_string(),
            verifier_work_units: 296,
            retained_gain: 296,
        }],
        episodes: vec![CurriculumEpisode {
            substrate_id: "S1-SEM28-STRUCTURAL-INTERACTION".to_string(),
            family: "RELATIONAL_RULE_GRAPH".to_string(),
            difficulty_dimension: "STRUCTURAL_INTERACTION_RANK".to_string(),
            hypothesis_count: 4,
            failed_candidates: 3,
            calibration_probes: 4,
            genesis_cost: 1_328,
            time_to_learnable_frontier: 7,
            time_to_retained_gain: 12,
            prediction_error: 43,
            retained_capability_gain: 296,
            future_substrates_enabled: 2,
            future_dimensions_enabled: 2,
            future_laws_enabled: 1,
        }],
        failed_candidate_patterns: vec![
            "REPEATED_KNOWN_WORK".to_string(),
            "NO_LOCALIZED_FEEDBACK".to_string(),
            "RESOURCE_DOMINATED".to_string(),
        ],
        successful_candidate_patterns: vec![
            "NONREDUNDANT_STRUCTURE".to_string(),
            "LOCALIZED_FEEDBACK".to_string(),
            "INDEPENDENT_VERIFIER".to_string(),
        ],
        motifs: vec![],
        laws: vec![],
        schemas: vec![],
        predictor: PredictorState {
            observations: 1,
            verifier_work_bias: 18,
            novelty_weight: 7,
            learnability_weight: 5,
            calibration_residuals: vec![43],
        },
        generator_is_success_authority: false,
    }
}

pub fn candidate_vocabulary(seed: u64) -> Vec<SubstrateCandidate> {
    [
        FeatureKind::RelationalEcho,
        FeatureKind::ResourceDominatedComposition,
        FeatureKind::TemporalCoupling,
        FeatureKind::CrossInstanceBinding,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, feature)| candidate(feature, mix(seed, index as u64 + 1)))
    .collect()
}

pub fn route_candidates(
    vocabulary: &[SubstrateCandidate],
    capability: CapabilityMask,
    memory_enabled: bool,
    law_enabled: bool,
    predictor_enabled: bool,
) -> Vec<SubstrateCandidate> {
    let mut by_feature = |feature: FeatureKind| {
        vocabulary
            .iter()
            .find(|candidate| candidate.feature == feature)
            .cloned()
    };
    let ordering = if !memory_enabled {
        vec![
            FeatureKind::RelationalEcho,
            FeatureKind::ResourceDominatedComposition,
            FeatureKind::TemporalCoupling,
            FeatureKind::CrossInstanceBinding,
        ]
    } else if !predictor_enabled {
        vec![
            FeatureKind::ResourceDominatedComposition,
            FeatureKind::TemporalCoupling,
            FeatureKind::RelationalEcho,
            FeatureKind::CrossInstanceBinding,
        ]
    } else if capability.temporal && law_enabled {
        vec![FeatureKind::CrossInstanceBinding]
    } else {
        vec![
            FeatureKind::TemporalCoupling,
            FeatureKind::CrossInstanceBinding,
        ]
    };
    ordering.into_iter().filter_map(&mut by_feature).collect()
}

pub fn promote_curriculum_abstractions(memory: &mut CurriculumResearchMemory) {
    if memory.episodes.len() < 2 || !memory.motifs.is_empty() {
        return;
    }
    memory.motifs.push(CurriculumMotif {
        motif_id: "MOTIF-NOVEL-LOCAL-VERIFIABLE".to_string(),
        support: 2,
        statement: "nonredundant structural stress plus localized verifier feedback identifies a bounded learnable frontier".to_string(),
        success_authority: false,
    });
    memory.laws.push(CurriculumLaw {
        law_id: "LAW-BOUNDARY-ORTHOGONALITY-1".to_string(),
        support: 2,
        antecedent: "a mastered boundary exposes an unsupported orthogonal public dependency with bounded independent verification".to_string(),
        routing_effect: "suppress replay and resource-dominated candidates; prioritize the unsupported dependency family".to_string(),
        requires_fresh_independent_verification: true,
        success_authority: false,
    });
    memory.schemas.push(CurriculumSchema {
        schema_id: "SCHEMA-PREDICT-PROBE-ADAPT-RETAIN".to_string(),
        support: 2,
        stages: vec![
            "predict boundary stress".to_string(),
            "probe with frozen independent verifier".to_string(),
            "adapt only from localized violation".to_string(),
            "confirm on fresh holdout and anchors".to_string(),
        ],
        success_authority: false,
    });
}

pub fn generate_challenge(
    candidate: &SubstrateCandidate,
    seed: u64,
    instance_id: u64,
    stress: u8,
) -> Challenge {
    let values = (0..6_u64)
        .map(|index| mix(seed ^ instance_id, 31 + index))
        .collect();
    let lagged = (0..6_u64)
        .map(|index| mix(seed ^ 0x1A6, 61 + index))
        .collect();
    let peer = (0..6_u64)
        .map(|index| mix(seed ^ 0xC2055, 91 + index))
        .collect();
    let mut rules = vec![
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
    ];
    match candidate.feature {
        FeatureKind::TemporalCoupling => rules.push(Rule::TemporalCouple {
            current: 0,
            lagged: 2,
            target: 3,
            phase: (mix(seed, 31) % 63 + 1) as u32,
            salt: mix(seed, 37),
        }),
        FeatureKind::CrossInstanceBinding => rules.push(Rule::CrossBind {
            local: 0,
            peer: 4,
            target: 3,
            rotate: (mix(seed, 41) % 63 + 1) as u32,
            salt: mix(seed, 43),
        }),
        FeatureKind::ResourceDominatedComposition => {
            for offset in 0..stress.max(12) {
                rules.push(Rule::Relate {
                    left: usize::from(offset % 6),
                    right: usize::from((offset + 1) % 6),
                    target: usize::from((offset + 2) % 6),
                    rotate: u32::from(offset) + 1,
                    salt: mix(seed, u64::from(offset) + 101),
                });
                if rules.len() == 20 {
                    break;
                }
            }
        }
        FeatureKind::RelationalEcho => {}
    }
    Challenge {
        contract_version: CONTRACT_VERSION.to_string(),
        substrate_id: candidate.candidate_id.clone(),
        substrate_family: candidate.substrate_family.clone(),
        difficulty_dimension: candidate.difficulty_dimension.clone(),
        instance_id,
        public_seed: seed,
        context_values: values,
        lagged_context_values: lagged,
        peer_context_values: peer,
        rules,
        public_nonce: mix(seed, instance_id ^ 0x5E29_0001),
    }
}

pub fn solve(challenge: &Challenge, capability: CapabilityMask) -> CandidateSolution {
    let mut state = [0_u64; 6];
    if challenge.context_values.len() == 6 {
        state.copy_from_slice(&challenge.context_values);
    }
    let mut trace = mix(challenge.public_seed, challenge.public_nonce);
    for (index, rule) in challenge.rules.iter().enumerate() {
        let result = match *rule {
            Rule::Affine {
                source,
                target,
                multiplier,
                increment,
            } if source < 6 && target < 6 => Some((
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
            } if capability.relational && left < 6 && right < 6 && target < 6 => Some((
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
                && condition < 6
                && when_even < 6
                && when_odd < 6
                && target < 6 =>
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
            } if capability.relational && left < 6 && right < 6 && target < 6 => Some((
                target,
                mix(
                    state[left].rotate_left(rotate % 64),
                    state[right].wrapping_add(salt),
                ),
            )),
            Rule::TemporalCouple {
                current,
                lagged,
                target,
                phase,
                salt,
            } if capability.temporal && current < 6 && lagged < 6 && target < 6 => Some((
                target,
                mix(
                    state[current].rotate_left(phase % 64),
                    challenge.lagged_context_values[lagged].wrapping_add(salt),
                ),
            )),
            Rule::CrossBind {
                local,
                peer,
                target,
                rotate,
                salt,
            } if capability.cross_instance && local < 6 && peer < 6 && target < 6 => Some((
                target,
                mix(state[local] ^ salt, challenge.peer_context_values[peer])
                    .rotate_left(rotate % 64),
            )),
            _ => None,
        };
        if let Some((target, value)) = result {
            state[target] = value;
            trace = mix(trace, value ^ (index as u64).rotate_left(17));
        } else {
            trace = mix(trace, 0xBAD0_0000 ^ index as u64);
        }
    }
    let result_digest = state
        .iter()
        .enumerate()
        .fold(challenge.public_nonce, |acc, (index, value)| {
            mix(acc, value.rotate_left((index * 9) as u32))
        });
    CandidateSolution {
        result_digest,
        trace_commitment: trace,
    }
}

fn candidate(feature: FeatureKind, token: u64) -> SubstrateCandidate {
    let (
        definition,
        effect,
        stress,
        insufficiency,
        learnability,
        work,
        novelty,
        adaptation,
        resource,
        frontier,
    ) = match feature {
        FeatureKind::RelationalEcho => (
            "replay of mastered within-instance structural interaction",
            "does not add a dependency class",
            "mastered relational boundary",
            "none: predecessor already represents it",
            "too easy",
            72,
            0,
            "none",
            72,
            0,
        ),
        FeatureKind::TemporalCoupling => (
            "dependency on a public lagged semantic state within one verified trace",
            "adds lag-sensitive value, depth, work and structural signature",
            "lack of temporal state coupling",
            "relational interpreter cannot consume lagged context",
            "bounded because feedback localizes to one public temporal rule",
            90,
            37,
            "add public temporal-coupling representation",
            119,
            340,
        ),
        FeatureKind::CrossInstanceBinding => (
            "dependency on a public peer-instance semantic state",
            "adds cross-instance value, depth, work and structural signature",
            "lack of cross-instance binding",
            "temporal/relational interpreter cannot bind a peer context",
            "bounded because feedback localizes to one public binding rule",
            88,
            49,
            "add public cross-instance binding representation",
            128,
            390,
        ),
        FeatureKind::ResourceDominatedComposition => (
            "repeat mastered relational operations without a new dependency class",
            "increases raw work while preserving known semantics",
            "compute envelope rather than capability boundary",
            "not representationally insufficient",
            "too hard/resource dominated and semantically redundant",
            620,
            1,
            "none",
            620,
            0,
        ),
    };
    SubstrateCandidate {
        candidate_id: format!("CAND-{:?}-{:08x}", feature, token as u32).to_ascii_uppercase(),
        feature,
        substrate_family: feature.family().to_string(),
        difficulty_dimension: feature.dimension().to_string(),
        semantic_definition: definition.to_string(),
        mechanical_effect: effect.to_string(),
        predicted_boundary_stress: stress.to_string(),
        predicted_insufficiency: insufficiency.to_string(),
        predicted_learnability: learnability.to_string(),
        predicted_verifier_work_units: work,
        predicted_structural_novelty: novelty,
        predicted_adaptation: adaptation.to_string(),
        predicted_resource_effect: resource,
        predicted_frontier_effect: frontier,
        routed_by_memory: false,
        routed_by_law: false,
        operator_selected: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem29::verifier::{semantic_metrics, verify, VerificationRequest};

    fn relational() -> CapabilityMask {
        CapabilityMask {
            relational: true,
            temporal: false,
            cross_instance: false,
        }
    }

    #[test]
    fn memory_routes_away_from_replay_and_resource_scaling() {
        let vocabulary = candidate_vocabulary(29);
        let routed = route_candidates(&vocabulary, relational(), true, false, true);
        assert_eq!(routed.len(), 2);
        assert_eq!(routed[0].feature, FeatureKind::TemporalCoupling);
    }

    #[test]
    fn temporal_feature_is_mechanical_and_requires_adaptation() {
        let candidate = candidate_vocabulary(31)
            .into_iter()
            .find(|c| c.feature == FeatureKind::TemporalCoupling)
            .unwrap();
        let challenge = generate_challenge(&candidate, 37, 1, 1);
        let before = verify(&VerificationRequest {
            challenge: challenge.clone(),
            solution: solve(&challenge, relational()),
        });
        let after_mask = relational().adapted(FeatureKind::TemporalCoupling);
        let after = verify(&VerificationRequest {
            challenge: challenge.clone(),
            solution: solve(&challenge, after_mask),
        });
        assert!(!before.accepted);
        assert!(after.accepted);
        assert!(semantic_metrics(&challenge).unwrap().0 > 0);
    }

    #[test]
    fn serialized_challenge_has_no_gold_fields() {
        let candidate = candidate_vocabulary(41)
            .into_iter()
            .find(|c| c.feature == FeatureKind::CrossInstanceBinding)
            .unwrap();
        let challenge = generate_challenge(&candidate, 43, 2, 1);
        let text = serde_json::to_string(&challenge)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in ["expected", "answer", "witness", "gold"] {
            assert!(!text.contains(forbidden));
        }
    }
}
