use serde::{Deserialize, Serialize};

use super::verifier::{CandidateSolution, Challenge, Rule, CONTRACT_VERSION};

pub const CURRENT_SUBSTRATE_EFFECTIVE_DIFFICULTY: u64 = 900_000;
pub const MAX_AUTONOMOUS_RESEARCH_EPOCHS: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityBoundary {
    pub regime_id: u16,
    pub transition_count: usize,
    pub requested_difficulty: u64,
    pub effective_verified_difficulty: u64,
    pub prior_frontier_scale: u64,
    pub predecessor_dimensions: [u16; 5],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaturationEvidence {
    pub current_requested_difficulty: u64,
    pub current_effective_difficulty: u64,
    pub counterfactual_requested_difficulties: Vec<u64>,
    pub counterfactual_effective_difficulties: Vec<u64>,
    pub causally_distinct_attempts: usize,
    pub requested_increase_without_effective_increase: bool,
    pub current_substrate_saturated: bool,
    pub classification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuralFeatures {
    pub context_count: u8,
    pub relation_count: u8,
    pub branch_count: u8,
    pub latent_state_count: u8,
    pub repeated_known_work: u16,
    pub localized_feedback_channels: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubstrateCandidate {
    pub candidate_id: String,
    pub features: StructuralFeatures,
    pub mechanically_verifiable: bool,
    pub fresh_instance_applicable: bool,
    pub predicted_novelty_score: i64,
    pub predicted_learnability_score: i64,
    pub predicted_resource_cost: u64,
    pub selection_score: i64,
    pub quality_classification: String,
    pub rejection_reason: Option<String>,
    pub selected_autonomously: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DifficultyDimensionProposal {
    pub name: String,
    pub semantic_definition: String,
    pub predecessor_value: u8,
    pub proposed_value: u8,
    pub mechanical_effect: String,
    pub fresh_instance_applicable: bool,
    pub operator_selected: bool,
}

pub fn difficulty_complexity(dimensions: [u16; 5]) -> u64 {
    let [causal, composition, transfer, constraints, planning] = dimensions.map(u64::from);
    causal
        .saturating_mul(causal)
        .saturating_add(composition.saturating_mul(composition).saturating_mul(2))
        .saturating_add(transfer.saturating_mul(constraints))
        .saturating_add(planning.saturating_mul(planning))
        .saturating_add(causal.saturating_mul(composition).saturating_mul(transfer))
}

pub fn diagnose_saturation(boundary: &CapabilityBoundary) -> SaturationEvidence {
    let mut requested = Vec::new();
    let mut effective = Vec::new();
    for index in 0..boundary.predecessor_dimensions.len() {
        let mut counterfactual = boundary.predecessor_dimensions;
        counterfactual[index] = counterfactual[index].saturating_add(1);
        let work = difficulty_complexity(counterfactual)
            .saturating_mul(384)
            .max(2_000);
        requested.push(work);
        effective.push(work.min(CURRENT_SUBSTRATE_EFFECTIVE_DIFFICULTY));
    }
    let requested_increase_without_effective_increase =
        requested
            .iter()
            .zip(&effective)
            .all(|(requested_work, effective_work)| {
                *requested_work > boundary.requested_difficulty
                    && *effective_work == boundary.effective_verified_difficulty
            });
    SaturationEvidence {
        current_requested_difficulty: boundary.requested_difficulty,
        current_effective_difficulty: boundary.effective_verified_difficulty,
        counterfactual_requested_difficulties: requested,
        counterfactual_effective_difficulties: effective,
        causally_distinct_attempts: boundary.predecessor_dimensions.len(),
        requested_increase_without_effective_increase,
        current_substrate_saturated: requested_increase_without_effective_increase,
        classification: if requested_increase_without_effective_increase {
            "CURRENT_DIFFICULTY_SUBSTRATE_EXHAUSTED".to_string()
        } else {
            "INSUFFICIENT_EVIDENCE".to_string()
        },
    }
}

pub fn generate_substrate_hypotheses(
    boundary: &CapabilityBoundary,
    seed: u64,
) -> Vec<SubstrateCandidate> {
    let jitter = (mix(seed, boundary.prior_frontier_scale) % 3) as u8;
    let feature_sets = [
        StructuralFeatures {
            context_count: 1,
            relation_count: 1,
            branch_count: 0,
            latent_state_count: 0,
            repeated_known_work: 8 + u16::from(jitter),
            localized_feedback_channels: 1,
        },
        StructuralFeatures {
            context_count: 2,
            relation_count: 1,
            branch_count: 0,
            latent_state_count: 4 + jitter,
            repeated_known_work: 0,
            localized_feedback_channels: 0,
        },
        StructuralFeatures {
            context_count: 3 + (jitter & 1),
            relation_count: 3,
            branch_count: 2,
            latent_state_count: 1,
            repeated_known_work: 0,
            localized_feedback_channels: 3,
        },
        StructuralFeatures {
            context_count: 16,
            relation_count: 12,
            branch_count: 8,
            latent_state_count: 8,
            repeated_known_work: 0,
            localized_feedback_channels: 1,
        },
    ];
    feature_sets
        .into_iter()
        .enumerate()
        .map(|(index, features)| score_candidate(index, features, seed))
        .collect()
}

pub fn select_substrate_candidate(candidates: &[SubstrateCandidate]) -> Option<SubstrateCandidate> {
    let selected_index = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.mechanically_verifiable
                && candidate.fresh_instance_applicable
                && candidate.quality_classification == "LEARNABLE_FRONTIER"
        })
        .max_by_key(|(_, candidate)| candidate.selection_score)
        .map(|(index, _)| index)?;
    let mut selected = candidates[selected_index].clone();
    selected.selected_autonomously = true;
    Some(selected)
}

pub fn derive_new_dimension(candidate: &SubstrateCandidate) -> DifficultyDimensionProposal {
    let interaction_rank = candidate
        .features
        .relation_count
        .max(candidate.features.branch_count.saturating_add(1));
    DifficultyDimensionProposal {
        name: "STRUCTURAL_INTERACTION_RANK".to_string(),
        semantic_definition: "maximum number of mutually dependent public semantic relations that must be resolved within one verified solution trace".to_string(),
        predecessor_value: 1,
        proposed_value: interaction_rank,
        mechanical_effect: "changes the rule graph, dependency depth, semantic work units, and verified result under an otherwise fixed instance seed".to_string(),
        fresh_instance_applicable: true,
        operator_selected: false,
    }
}

pub fn generate_challenge(
    candidate: &SubstrateCandidate,
    seed: u64,
    instance_id: u64,
    interaction_rank: u8,
) -> Challenge {
    let rank = interaction_rank.clamp(1, 4);
    let mut values = Vec::with_capacity(4);
    for index in 0..4_u64 {
        values.push(mix(seed ^ instance_id, index.wrapping_mul(97)).max(1));
    }
    let mut rules = vec![
        Rule::Affine {
            source: 0,
            target: 1,
            multiplier: (mix(seed, 11) | 1).max(3),
            increment: mix(seed, 13),
        },
        Rule::Affine {
            source: 2,
            target: 3,
            multiplier: (mix(seed, 17) | 1).max(3),
            increment: mix(seed, 19),
        },
    ];
    if rank >= 2 {
        rules.push(Rule::Relate {
            left: 1,
            right: 3,
            target: 0,
            rotate: (mix(seed, 23) % 63 + 1) as u32,
            salt: mix(seed, 29),
        });
    }
    if rank >= 3 {
        rules.push(Rule::Gate {
            condition: 0,
            when_even: 1,
            when_odd: 3,
            target: 2,
            salt: mix(seed, 31),
        });
        rules.push(Rule::Fold {
            left: 0,
            right: 2,
            target: 3,
            rotate: (mix(seed, 37) % 63 + 1) as u32,
            salt: mix(seed, 41),
        });
    }
    if rank >= 4 {
        rules.push(Rule::Relate {
            left: 3,
            right: 1,
            target: 0,
            rotate: (mix(seed, 43) % 63 + 1) as u32,
            salt: mix(seed, 47),
        });
    }
    Challenge {
        contract_version: CONTRACT_VERSION.to_string(),
        substrate_id: format!(
            "AUTO-SUBSTRATE-{:016x}",
            feature_fingerprint(&candidate.features)
        ),
        instance_id,
        public_seed: seed,
        interaction_rank: rank,
        context_values: values,
        rules,
        public_nonce: mix(seed, instance_id ^ 0x5E28_0001),
    }
}

pub fn baseline_solve(challenge: &Challenge) -> CandidateSolution {
    solve(challenge, false)
}

pub fn adapted_solve(challenge: &Challenge) -> CandidateSolution {
    solve(challenge, true)
}

fn score_candidate(index: usize, features: StructuralFeatures, seed: u64) -> SubstrateCandidate {
    let mechanically_verifiable = features.latent_state_count <= 1 || features.context_count >= 16;
    let fresh_instance_applicable = features.context_count <= 8;
    let novelty = i64::from(features.context_count.saturating_sub(1)) * 17
        + i64::from(features.relation_count.saturating_sub(1)) * 23
        + i64::from(features.branch_count) * 29
        + i64::from(features.latent_state_count) * 11;
    let learnability = if mechanically_verifiable && features.localized_feedback_channels > 0 {
        45 + i64::from(features.localized_feedback_channels) * 17
    } else {
        -70
    };
    let resource_cost = u64::from(features.context_count)
        .saturating_mul(u64::from(features.relation_count.max(1)))
        .saturating_mul(u64::from(features.branch_count.saturating_add(1)))
        .saturating_mul(64);
    let redundancy_penalty = i64::from(features.repeated_known_work) * 19;
    let resource_penalty = if resource_cost > 8_192 { 240 } else { 0 };
    let selection_score = novelty + learnability - redundancy_penalty - resource_penalty;
    let (quality, rejection) = if features.repeated_known_work > 0 {
        ("TOO_EASY", Some("SEMANTICALLY_REDUNDANT"))
    } else if !mechanically_verifiable || features.localized_feedback_channels == 0 {
        (
            "UNINFORMATIVE_FAILURE",
            Some("NOT_INDEPENDENTLY_VERIFIABLE"),
        )
    } else if !fresh_instance_applicable || resource_cost > 8_192 {
        ("TOO_HARD", Some("RESOURCE_DOMINATED"))
    } else {
        ("LEARNABLE_FRONTIER", None)
    };
    SubstrateCandidate {
        candidate_id: format!(
            "HYP-{:02}-{:08x}",
            index + 1,
            mix(seed, index as u64) as u32
        ),
        features,
        mechanically_verifiable,
        fresh_instance_applicable,
        predicted_novelty_score: novelty,
        predicted_learnability_score: learnability,
        predicted_resource_cost: resource_cost,
        selection_score,
        quality_classification: quality.to_string(),
        rejection_reason: rejection.map(str::to_string),
        selected_autonomously: false,
    }
}

fn feature_fingerprint(features: &StructuralFeatures) -> u64 {
    mix(
        u64::from(features.context_count)
            | (u64::from(features.relation_count) << 8)
            | (u64::from(features.branch_count) << 16)
            | (u64::from(features.latent_state_count) << 24),
        u64::from(features.localized_feedback_channels)
            | (u64::from(features.repeated_known_work) << 16),
    )
}

fn solve(challenge: &Challenge, relational_representation_enabled: bool) -> CandidateSolution {
    let mut state = [0_u64; 4];
    if challenge.context_values.len() == 4 {
        state.copy_from_slice(&challenge.context_values);
    }
    let mut trace = solver_mix(challenge.public_seed, challenge.public_nonce);
    for (index, rule) in challenge.rules.iter().enumerate() {
        let target_and_value = match *rule {
            Rule::Affine {
                source,
                target,
                multiplier,
                increment,
            } if source < 4 && target < 4 => Some((
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
            } if relational_representation_enabled && left < 4 && right < 4 && target < 4 => {
                Some((
                    target,
                    solver_mix(state[left], state[right] ^ salt).rotate_left(rotate % 64),
                ))
            }
            Rule::Gate {
                condition,
                when_even,
                when_odd,
                target,
                salt,
            } if relational_representation_enabled
                && condition < 4
                && when_even < 4
                && when_odd < 4
                && target < 4 =>
            {
                let selected = if state[condition] & 1 == 0 {
                    state[when_even]
                } else {
                    state[when_odd]
                };
                Some((target, solver_mix(selected, challenge.public_nonce ^ salt)))
            }
            Rule::Fold {
                left,
                right,
                target,
                rotate,
                salt,
            } if relational_representation_enabled && left < 4 && right < 4 && target < 4 => {
                Some((
                    target,
                    solver_mix(
                        state[left].rotate_left(rotate % 64),
                        state[right].wrapping_add(salt),
                    ),
                ))
            }
            _ => None,
        };
        if let Some((target, value)) = target_and_value {
            state[target] = value;
            trace = solver_mix(trace, value ^ (index as u64).rotate_left(17));
        } else {
            trace = solver_mix(trace, 0xBAD0_0000 ^ index as u64);
        }
    }
    let digest =
        state
            .iter()
            .enumerate()
            .fold(challenge.public_nonce, |accumulator, (index, value)| {
                solver_mix(accumulator, value.rotate_left((index * 13) as u32))
            });
    CandidateSolution {
        result_digest: digest,
        trace_commitment: trace,
    }
}

fn mix(left: u64, right: u64) -> u64 {
    solver_mix(left, right)
}

fn solver_mix(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem28::verifier::{semantic_metrics, verify, VerificationRequest};

    fn boundary() -> CapabilityBoundary {
        let dimensions = [4, 13, 27, 13, 16];
        CapabilityBoundary {
            regime_id: 59,
            transition_count: 58,
            requested_difficulty: difficulty_complexity(dimensions) * 384,
            effective_verified_difficulty: CURRENT_SUBSTRATE_EFFECTIVE_DIFFICULTY,
            prior_frontier_scale: 8_604_841,
            predecessor_dimensions: dimensions,
        }
    }

    #[test]
    fn saturation_requires_effective_work_stagnation() {
        let evidence = diagnose_saturation(&boundary());
        assert!(evidence.current_substrate_saturated);
        assert!(evidence.requested_increase_without_effective_increase);
        assert_eq!(evidence.causally_distinct_attempts, 5);
    }

    #[test]
    fn curriculum_selection_is_deterministic_across_rehydration() {
        let candidates = generate_substrate_hypotheses(&boundary(), 0x5E28_0001);
        let restored: Vec<SubstrateCandidate> =
            serde_json::from_str(&serde_json::to_string(&candidates).unwrap()).unwrap();
        assert_eq!(
            select_substrate_candidate(&candidates),
            select_substrate_candidate(&restored)
        );
    }

    #[test]
    fn interaction_rank_changes_mechanical_work() {
        let candidate =
            select_substrate_candidate(&generate_substrate_hypotheses(&boundary(), 0x5E28_0001))
                .unwrap();
        let low = generate_challenge(&candidate, 11, 1, 1);
        let high = generate_challenge(&candidate, 11, 1, 3);
        assert_ne!(
            semantic_metrics(&low).unwrap(),
            semantic_metrics(&high).unwrap()
        );
    }

    #[test]
    fn adaptation_solves_new_contract_without_regressing_rank_one() {
        let candidate =
            select_substrate_candidate(&generate_substrate_hypotheses(&boundary(), 0x5E28_0001))
                .unwrap();
        let hard = generate_challenge(&candidate, 19, 3, 3);
        let initial = verify(&VerificationRequest {
            challenge: hard.clone(),
            solution: baseline_solve(&hard),
        });
        let adapted = verify(&VerificationRequest {
            challenge: hard.clone(),
            solution: adapted_solve(&hard),
        });
        let anchor = generate_challenge(&candidate, 23, 4, 1);
        let retained = verify(&VerificationRequest {
            challenge: anchor.clone(),
            solution: adapted_solve(&anchor),
        });
        assert!(!initial.accepted);
        assert!(adapted.accepted);
        assert!(retained.accepted);
    }

    #[test]
    fn serialized_challenge_has_no_gold_fields() {
        let candidate =
            select_substrate_candidate(&generate_substrate_hypotheses(&boundary(), 0x5E28_0001))
                .unwrap();
        let challenge = generate_challenge(&candidate, 29, 5, 3);
        let serialized = serde_json::to_string(&challenge)
            .unwrap()
            .to_ascii_lowercase();
        for forbidden in ["expected", "answer", "witness", "gold"] {
            assert!(!serialized.contains(forbidden));
        }
    }
}
